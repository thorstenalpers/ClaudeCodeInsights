use super::reader::{self, FileParse};
use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanStats {
    pub files_total: usize,
    pub files_read: usize,
    pub files_skipped: usize,
    pub turns_inserted: usize,
    pub sessions_seen: usize,
    pub malformed_lines: u64,
    /// Files that could not be ingested. Reported rather than only logged: a
    /// scan that quietly wrote nothing must not look like a successful one.
    pub files_failed: usize,
    pub first_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub files_done: usize,
    pub files_total: usize,
    pub current_file: Option<String>,
}

/// Why a file has to be read, or why it does not.
enum FileWork {
    /// Never seen; read every line.
    Fresh,
    /// Grew since last time; read only the tail.
    Appended { skip_lines: u64, id: i64 },
    /// Shrank or was rewritten; drop what came from it and read it again.
    Rewritten { id: i64 },
    Unchanged,
}

pub fn discover_transcripts(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = roots
        .iter()
        .filter(|root| root.is_dir())
        .flat_map(|root| {
            WalkDir::new(root)
                .follow_links(false)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|entry| entry.file_type().is_file())
                .filter(|entry| {
                    entry
                        .path()
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("jsonl"))
                })
                .map(|entry| entry.into_path())
        })
        .collect();

    found.sort();
    found.dedup();
    found
}

pub fn scan<F>(conn: &mut Connection, roots: &[PathBuf], mut on_progress: F) -> Result<ScanStats>
where
    F: FnMut(ScanProgress),
{
    let files = discover_transcripts(roots);
    let mut stats = ScanStats {
        files_total: files.len(),
        ..ScanStats::default()
    };

    for (index, path) in files.iter().enumerate() {
        on_progress(ScanProgress {
            files_done: index,
            files_total: files.len(),
            current_file: path.file_name().map(|n| n.to_string_lossy().into_owned()),
        });

        if let Err(error) = ingest_one(conn, path, &mut stats) {
            // One unreadable transcript must not abandon the rest of the scan,
            // but it must not vanish either.
            log::warn!("skipping {}: {error:#}", path.display());
            stats.files_failed += 1;
            stats
                .first_error
                .get_or_insert_with(|| format!("{}: {error:#}", path.display()));
        }
    }

    on_progress(ScanProgress {
        files_done: files.len(),
        files_total: files.len(),
        current_file: None,
    });

    recompute_derived(conn)?;

    // Only sessions that actually hold turns are counted. A transcript can leave
    // behind a session row carrying nothing but a title, and reporting those
    // would make this figure disagree with every list that shows real sessions.
    let sessions: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sessions WHERE turn_count > 0",
        [],
        |r| r.get(0),
    )?;
    stats.sessions_seen = sessions as usize;
    Ok(stats)
}

fn ingest_one(conn: &mut Connection, path: &Path, stats: &mut ScanStats) -> Result<()> {
    let metadata = std::fs::metadata(path)?;
    let size = metadata.len() as i64;
    let mtime_ms = metadata
        .modified()?
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let path_text = path.to_string_lossy().into_owned();
    let work = classify(conn, &path_text, size, mtime_ms)?;

    let (skip_lines, existing_id) = match work {
        FileWork::Unchanged => {
            stats.files_skipped += 1;
            return Ok(());
        }
        FileWork::Fresh => (0, None),
        FileWork::Appended { skip_lines, id } => (skip_lines, Some(id)),
        FileWork::Rewritten { id } => {
            // Its rows can no longer be trusted to correspond to anything in the
            // file, so they go before it is read again.
            conn.execute("DELETE FROM turns WHERE scan_file_id = ?1", [id])?;
            (0, Some(id))
        }
    };

    let parsed = reader::parse_file(path, skip_lines)?;
    stats.files_read += 1;
    stats.malformed_lines += parsed.malformed_lines;

    let tx = conn.transaction()?;
    let file_id = upsert_scan_file(
        &tx,
        existing_id,
        &path_text,
        mtime_ms,
        size,
        parsed.line_count as i64,
    )?;
    write_parse(&tx, file_id, &path_text, &parsed, stats)?;
    tx.commit()?;

    Ok(())
}

fn classify(conn: &Connection, path: &str, size: i64, mtime_ms: i64) -> Result<FileWork> {
    let row: Option<(i64, i64, i64, i64)> = conn
        .query_row(
            "SELECT id, mtime_ms, size_bytes, line_count FROM scan_files WHERE path = ?1",
            [path],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;

    let Some((id, stored_mtime, stored_size, stored_lines)) = row else {
        return Ok(FileWork::Fresh);
    };

    if stored_mtime == mtime_ms && stored_size == size {
        return Ok(FileWork::Unchanged);
    }

    if size < stored_size {
        return Ok(FileWork::Rewritten { id });
    }

    Ok(FileWork::Appended {
        skip_lines: stored_lines.max(0) as u64,
        id,
    })
}

fn upsert_scan_file(
    tx: &rusqlite::Transaction<'_>,
    existing_id: Option<i64>,
    path: &str,
    mtime_ms: i64,
    size: i64,
    line_count: i64,
) -> Result<i64> {
    let now = chrono::Utc::now().to_rfc3339();

    if let Some(id) = existing_id {
        tx.execute(
            "UPDATE scan_files SET mtime_ms = ?2, size_bytes = ?3, line_count = ?4, scanned_at = ?5
             WHERE id = ?1",
            params![id, mtime_ms, size, line_count, now],
        )?;
        return Ok(id);
    }

    tx.execute(
        "INSERT INTO scan_files (path, mtime_ms, size_bytes, line_count, scanned_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![path, mtime_ms, size, line_count, now],
    )?;
    Ok(tx.last_insert_rowid())
}

fn write_parse(
    tx: &rusqlite::Transaction<'_>,
    file_id: i64,
    transcript_path: &str,
    parsed: &FileParse,
    stats: &mut ScanStats,
) -> Result<()> {
    for meta in parsed.sessions.values() {
        let project_name = meta.cwd.as_deref().map(project_name_from_cwd);

        tx.execute(
            "INSERT INTO sessions
                (session_id, project_path, project_name, git_branch, topic, topic_source,
                 has_subagents, transcript_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(session_id) DO UPDATE SET
                project_path  = COALESCE(excluded.project_path, sessions.project_path),
                project_name  = COALESCE(excluded.project_name, sessions.project_name),
                git_branch    = COALESCE(excluded.git_branch, sessions.git_branch),
                -- A written title outranks an inferred one, and neither is lost
                -- because a later file happened to carry no title at all.
                topic         = CASE
                                  WHEN excluded.topic_source = 'custom-title' THEN excluded.topic
                                  WHEN sessions.topic_source = 'custom-title' THEN sessions.topic
                                  ELSE COALESCE(excluded.topic, sessions.topic)
                                END,
                topic_source  = CASE
                                  WHEN excluded.topic_source = 'custom-title' THEN excluded.topic_source
                                  WHEN sessions.topic_source = 'custom-title' THEN sessions.topic_source
                                  ELSE COALESCE(excluded.topic_source, sessions.topic_source)
                                END,
                has_subagents = MAX(sessions.has_subagents, excluded.has_subagents),
                transcript_path = COALESCE(excluded.transcript_path, sessions.transcript_path)",
            params![
                meta.session_id,
                meta.cwd,
                project_name,
                meta.git_branch,
                meta.topic,
                meta.topic_source,
                i64::from(meta.has_subagents),
                transcript_path,
            ],
        )?;
    }

    for turn in &parsed.turns {
        // ON CONFLICT rather than OR IGNORE: a rescan that sees a later
        // streaming record for a message must be able to correct the tally.
        let changed = tx.execute(
            "INSERT INTO turns
                (session_id, message_id, uuid, ts_utc, model, input_tokens, output_tokens,
                 cache_read_tokens, cache_write_tokens, cwd, git_branch, is_subagent,
                 agent_id, tool_call_count, scan_file_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
             -- The WHERE clause is not optional: ux_turns_message_id is a partial
             -- index, and SQLite only matches a conflict target to it when the
             -- predicate is repeated here.
             ON CONFLICT(message_id) WHERE message_id IS NOT NULL AND message_id <> ''
             DO UPDATE SET
                input_tokens       = excluded.input_tokens,
                output_tokens      = excluded.output_tokens,
                cache_read_tokens  = excluded.cache_read_tokens,
                cache_write_tokens = excluded.cache_write_tokens,
                model              = COALESCE(excluded.model, turns.model),
                tool_call_count    = excluded.tool_call_count,
                scan_file_id       = excluded.scan_file_id",
            params![
                turn.session_id,
                turn.message_id,
                turn.uuid,
                turn.timestamp,
                turn.model,
                turn.input_tokens,
                turn.output_tokens,
                turn.cache_read_tokens,
                turn.cache_write_tokens,
                turn.cwd,
                turn.git_branch,
                i64::from(turn.is_subagent),
                turn.agent_id,
                turn.tools.len() as i64,
                file_id,
            ],
        )?;
        stats.turns_inserted += changed;

        let turn_id: i64 = match &turn.message_id {
            Some(id) => tx.query_row(
                "SELECT id FROM turns WHERE message_id = ?1",
                [id],
                |r| r.get(0),
            )?,
            None => tx.last_insert_rowid(),
        };

        tx.execute("DELETE FROM turn_tools WHERE turn_id = ?1", [turn_id])?;
        for (seq, tool) in turn.tools.iter().enumerate() {
            tx.execute(
                "INSERT INTO turn_tools (turn_id, seq, tool_name) VALUES (?1, ?2, ?3)",
                params![turn_id, seq as i64, tool],
            )?;
        }
    }

    for agent in &parsed.agents {
        tx.execute(
            "INSERT INTO agents
                (agent_id, agent_type, completed_ts, status, total_tokens,
                 total_duration_ms, tool_use_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(agent_id) DO UPDATE SET
                agent_type        = COALESCE(excluded.agent_type, agents.agent_type),
                status            = COALESCE(excluded.status, agents.status),
                total_tokens      = COALESCE(excluded.total_tokens, agents.total_tokens),
                total_duration_ms = COALESCE(excluded.total_duration_ms, agents.total_duration_ms),
                tool_use_count    = COALESCE(excluded.tool_use_count, agents.tool_use_count)",
            params![
                agent.agent_id,
                agent.agent_type,
                chrono::Utc::now().to_rfc3339(),
                agent.status,
                agent.total_tokens,
                agent.total_duration_ms,
                agent.tool_use_count,
            ],
        )?;
    }

    Ok(())
}

/// Recomputes everything that is derived, from the turn rows.
///
/// Session figures are never accumulated as rows arrive. An insert that hits the
/// message_id conflict corrects a value rather than adding one, so a running
/// total would drift; recomputing is the only version that stays right after a
/// partial rescan.
fn recompute_derived(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        UPDATE sessions SET
            first_ts   = (SELECT MIN(ts_utc) FROM turns t WHERE t.session_id = sessions.session_id),
            last_ts    = (SELECT MAX(ts_utc) FROM turns t WHERE t.session_id = sessions.session_id),
            turn_count = (SELECT COUNT(*)    FROM turns t WHERE t.session_id = sessions.session_id);

        -- The representative model is the highest-ranked one the session used,
        -- so a haiku subagent turn cannot relabel an opus session.
        UPDATE sessions SET model = (
            SELECT t.model
            FROM turns t
            WHERE t.session_id = sessions.session_id AND t.model IS NOT NULL
            GROUP BY t.model
            ORDER BY MAX(
                CASE
                    WHEN t.model LIKE '%fable%' OR t.model LIKE '%mythos%' THEN 5
                    WHEN t.model LIKE '%opus%'   THEN 3
                    WHEN t.model LIKE '%sonnet%' THEN 2
                    WHEN t.model LIKE '%haiku%'  THEN 1
                    ELSE 0
                END
            ) DESC,
            SUM(t.input_tokens + t.output_tokens) DESC
            LIMIT 1
        );

        -- Raw tool names per session, deliberately independent of any
        -- tool-to-category mapping: the derived activity is then a join against
        -- whatever mapping is current, and editing it never needs a rescan.
        DELETE FROM session_tool_counts;
        INSERT INTO session_tool_counts (session_id, tool_name, calls)
        SELECT t.session_id, tt.tool_name, COUNT(*)
        FROM turn_tools tt
        JOIN turns t ON t.id = tt.turn_id
        GROUP BY t.session_id, tt.tool_name;
        "#,
    )
    .context("recomputing derived session data")?;

    Ok(())
}

/// The last two path components, which is enough to tell projects apart without
/// exposing the whole directory layout.
fn project_name_from_cwd(cwd: &str) -> String {
    let normalised = cwd.replace('\\', "/");
    let parts: Vec<&str> = normalised
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();

    match parts.len() {
        0 => "unknown".to_owned(),
        1 => parts[0].to_owned(),
        n => format!("{}/{}", parts[n - 2], parts[n - 1]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::schema;
    use std::io::Write;

    struct Fixture {
        dir: PathBuf,
        conn: Connection,
    }

    impl Fixture {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir()
                .join("cua-scan")
                .join(format!("{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();

            let conn = Connection::open_in_memory().unwrap();
            schema::migrate(&conn).unwrap();
            Self { dir, conn }
        }

        fn write(&self, file: &str, lines: &[String]) -> PathBuf {
            let path = self.dir.join(file);
            let mut handle = std::fs::File::create(&path).unwrap();
            for line in lines {
                writeln!(handle, "{line}").unwrap();
            }
            // Timestamps have limited resolution; without this an append inside
            // the same tick can look unchanged.
            std::thread::sleep(std::time::Duration::from_millis(15));
            path
        }

        fn scan(&mut self) -> ScanStats {
            let roots = vec![self.dir.clone()];
            let stats = super::scan(&mut self.conn, &roots, |_| {}).unwrap();
            assert_eq!(
                stats.files_failed, 0,
                "a file failed to ingest: {:?}",
                stats.first_error
            );
            stats
        }

        fn count(&self, sql: &str) -> i64 {
            self.conn.query_row(sql, [], |r| r.get(0)).unwrap()
        }
    }

    fn turn_line(msg: &str, uuid: &str, output: i64, ts: &str, model: &str) -> String {
        format!(
            r#"{{"type":"assistant","sessionId":"s1","uuid":"{uuid}","timestamp":"{ts}","cwd":"/home/me/work/proj","message":{{"id":"{msg}","model":"{model}","usage":{{"input_tokens":100,"output_tokens":{output}}}}}}}"#
        )
    }

    #[test]
    fn a_second_scan_adds_nothing() {
        let mut fixture = Fixture::new("idempotent");
        fixture.write(
            "a.jsonl",
            &[
                turn_line("m1", "u1", 10, "2026-01-01T10:00:00Z", "claude-opus-4-8"),
                turn_line("m2", "u2", 20, "2026-01-01T10:01:00Z", "claude-opus-4-8"),
            ],
        );

        let first = fixture.scan();
        assert_eq!(first.turns_inserted, 2);
        assert_eq!(fixture.count("SELECT COUNT(*) FROM turns"), 2);

        let second = fixture.scan();
        assert_eq!(second.files_read, 0, "nothing changed, nothing re-read");
        assert_eq!(second.files_skipped, 1);
        assert_eq!(fixture.count("SELECT COUNT(*) FROM turns"), 2);
    }

    #[test]
    fn appending_reads_only_the_tail() {
        let mut fixture = Fixture::new("append");
        let mut lines = vec![turn_line("m1", "u1", 10, "2026-01-01T10:00:00Z", "claude-opus-4-8")];
        fixture.write("a.jsonl", &lines);
        fixture.scan();

        lines.push(turn_line("m2", "u2", 20, "2026-01-01T10:01:00Z", "claude-opus-4-8"));
        fixture.write("a.jsonl", &lines);

        let second = fixture.scan();
        assert_eq!(second.files_read, 1);
        assert_eq!(second.turns_inserted, 1, "only the appended turn");
        assert_eq!(fixture.count("SELECT COUNT(*) FROM turns"), 2);
    }

    #[test]
    fn a_rewritten_file_gives_the_same_result_as_a_full_scan() {
        let mut fixture = Fixture::new("rewrite");
        fixture.write(
            "a.jsonl",
            &[
                turn_line("m1", "u1", 10, "2026-01-01T10:00:00Z", "claude-opus-4-8"),
                turn_line("m2", "u2", 20, "2026-01-01T10:01:00Z", "claude-opus-4-8"),
                turn_line("m3", "u3", 30, "2026-01-01T10:02:00Z", "claude-opus-4-8"),
            ],
        );
        fixture.scan();

        // Shorter than before: the file was truncated or rewritten, so the rows
        // that came from it can no longer be trusted.
        fixture.write(
            "a.jsonl",
            &[turn_line("m9", "u9", 99, "2026-01-01T11:00:00Z", "claude-opus-4-8")],
        );
        fixture.scan();

        assert_eq!(fixture.count("SELECT COUNT(*) FROM turns"), 1);
        assert_eq!(
            fixture.count("SELECT output_tokens FROM turns"),
            99,
            "the stale turns are gone, not merged"
        );
    }

    #[test]
    fn session_totals_match_the_turns() {
        let mut fixture = Fixture::new("aggregates");
        fixture.write(
            "a.jsonl",
            &[
                turn_line("m1", "u1", 10, "2026-01-01T10:00:00Z", "claude-opus-4-8"),
                turn_line("m2", "u2", 20, "2026-01-01T12:00:00Z", "claude-opus-4-8"),
            ],
        );
        fixture.scan();

        let turn_count = fixture.count("SELECT turn_count FROM sessions WHERE session_id = 's1'");
        assert_eq!(turn_count, fixture.count("SELECT COUNT(*) FROM turns"));

        let first: String = fixture
            .conn
            .query_row("SELECT first_ts FROM sessions", [], |r| r.get(0))
            .unwrap();
        let last: String = fixture
            .conn
            .query_row("SELECT last_ts FROM sessions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(first, "2026-01-01T10:00:00Z");
        assert_eq!(last, "2026-01-01T12:00:00Z");
    }

    #[test]
    fn a_haiku_subagent_does_not_relabel_an_opus_session() {
        let mut fixture = Fixture::new("model-rank");
        fixture.write(
            "a.jsonl",
            &[
                turn_line("m1", "u1", 10, "2026-01-01T10:00:00Z", "claude-opus-4-8"),
                // More turns and more tokens, but a lower-ranked model.
                turn_line("m2", "u2", 900, "2026-01-01T10:01:00Z", "claude-haiku-4-5"),
                turn_line("m3", "u3", 900, "2026-01-01T10:02:00Z", "claude-haiku-4-5"),
            ],
        );
        fixture.scan();

        let model: String = fixture
            .conn
            .query_row("SELECT model FROM sessions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(model, "claude-opus-4-8");
    }

    #[test]
    fn tool_counts_are_materialised_per_session() {
        let mut fixture = Fixture::new("tool-counts");
        let with_tools = r#"{"type":"assistant","sessionId":"s1","uuid":"u1","timestamp":"2026-01-01T10:00:00Z","message":{"id":"m1","model":"claude-opus-4-8","usage":{"output_tokens":5},"content":[{"type":"tool_use","name":"Read"},{"type":"tool_use","name":"Edit"},{"type":"tool_use","name":"Read"}]}}"#;
        fixture.write("a.jsonl", &[with_tools.to_owned()]);
        fixture.scan();

        let reads: i64 = fixture
            .conn
            .query_row(
                "SELECT calls FROM session_tool_counts WHERE session_id = 's1' AND tool_name = 'Read'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(reads, 2);
        assert_eq!(fixture.count("SELECT COUNT(*) FROM session_tool_counts"), 2);
    }

    /// Runs against the machine's real transcripts. Ignored by default because
    /// it depends on data no other machine has; run it with
    /// `cargo test -- --ignored --nocapture` when changing the parser.
    #[test]
    #[ignore = "reads the developer's own ~/.claude/projects"]
    fn scans_the_real_transcript_folder() {
        let roots = crate::paths::default_scan_roots();
        if roots.is_empty() {
            eprintln!("no transcript folder on this machine; nothing to check");
            return;
        }

        let mut conn = Connection::open_in_memory().unwrap();
        schema::migrate(&conn).unwrap();

        let started = std::time::Instant::now();
        let stats = super::scan(&mut conn, &roots, |_| {}).unwrap();
        let elapsed = started.elapsed();

        let turns: i64 = conn
            .query_row("SELECT COUNT(*) FROM turns", [], |r| r.get(0))
            .unwrap();
        let tokens: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(input_tokens + output_tokens + cache_read_tokens + cache_write_tokens), 0) FROM turns",
                [],
                |r| r.get(0),
            )
            .unwrap();

        eprintln!("--- real scan ---");
        eprintln!("  elapsed        {elapsed:?}");
        eprintln!("  files          {} read, {} skipped, {} failed", stats.files_read, stats.files_skipped, stats.files_failed);
        eprintln!("  malformed      {}", stats.malformed_lines);
        eprintln!("  sessions       {}", stats.sessions_seen);
        eprintln!("  turns          {turns}");
        eprintln!("  tokens         {tokens}");
        if let Some(error) = &stats.first_error {
            eprintln!("  first error    {error}");
        }

        assert_eq!(stats.files_failed, 0, "no transcript should fail to ingest");

        // Session turn counts must agree with the rows they were derived from.
        let mismatched: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sessions s
                 WHERE s.turn_count <> (SELECT COUNT(*) FROM turns t WHERE t.session_id = s.session_id)",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(mismatched, 0, "session totals drifted from the turn rows");
    }

    #[test]
    fn project_name_keeps_the_last_two_components() {
        assert_eq!(project_name_from_cwd("/home/me/work/proj"), "work/proj");
        assert_eq!(project_name_from_cwd(r"C:\Sources\MyApp"), "Sources/MyApp");
        assert_eq!(project_name_from_cwd("solo"), "solo");
        assert_eq!(project_name_from_cwd(""), "unknown");
    }
}
