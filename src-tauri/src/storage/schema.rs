use anyhow::Result;
use rusqlite::Connection;

/// Bumped whenever a migration is added. Forward only — this is a cache that can
/// be rebuilt from the transcripts at any time, so there is no downgrade path.
pub(crate) const TARGET_VERSION: i64 = 5;

pub fn migrate(conn: &Connection) -> Result<()> {
    let current: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

    if current < 1 {
        conn.execute_batch(V1)?;
    }
    if current < 2 {
        conn.execute_batch(V2)?;
    }
    if current < 3 {
        conn.execute_batch(V3)?;
    }
    if current < 4 {
        conn.execute_batch(V4)?;
    }
    if current < 5 {
        conn.execute_batch(V5)?;
    }

    conn.pragma_update(None, "user_version", TARGET_VERSION)?;
    Ok(())
}

const V1: &str = r#"
-- Bookkeeping for the incremental scan. A file is re-read only when its
-- mtime, size or line count no longer match.
CREATE TABLE IF NOT EXISTS scan_files (
    id          INTEGER PRIMARY KEY,
    path        TEXT NOT NULL UNIQUE,
    mtime_ms    INTEGER NOT NULL,
    size_bytes  INTEGER NOT NULL,
    line_count  INTEGER NOT NULL,
    scanned_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
    session_id    TEXT PRIMARY KEY,
    project_path  TEXT,
    project_name  TEXT,
    git_branch    TEXT,
    first_ts      TEXT,
    last_ts       TEXT,
    model         TEXT,
    turn_count    INTEGER NOT NULL DEFAULT 0,
    topic         TEXT,
    topic_source  TEXT,
    has_subagents INTEGER NOT NULL DEFAULT 0,
    transcript_path TEXT
);

CREATE TABLE IF NOT EXISTS turns (
    id                  INTEGER PRIMARY KEY,
    session_id          TEXT NOT NULL,
    message_id          TEXT,
    uuid                TEXT,
    ts_utc              TEXT NOT NULL,
    model               TEXT,
    input_tokens        INTEGER NOT NULL DEFAULT 0,
    output_tokens       INTEGER NOT NULL DEFAULT 0,
    cache_read_tokens   INTEGER NOT NULL DEFAULT 0,
    cache_write_tokens  INTEGER NOT NULL DEFAULT 0,
    cwd                 TEXT,
    git_branch          TEXT,
    is_subagent         INTEGER NOT NULL DEFAULT 0,
    agent_id            TEXT,
    tool_call_count     INTEGER NOT NULL DEFAULT 0,
    scan_file_id        INTEGER NOT NULL
);

-- The cross-scan deduplication backbone. Claude Code writes several streaming
-- records per API response and only the last carries the final usage tally;
-- within a file the last one wins, and across scans this index makes a repeat
-- insert a no-op. Turns without a message_id (the reasoning bucket) bypass it.
CREATE UNIQUE INDEX IF NOT EXISTS ux_turns_message_id
    ON turns(message_id) WHERE message_id IS NOT NULL AND message_id <> '';

CREATE INDEX IF NOT EXISTS ix_turns_session ON turns(session_id);
CREATE INDEX IF NOT EXISTS ix_turns_ts      ON turns(ts_utc);
CREATE INDEX IF NOT EXISTS ix_turns_file    ON turns(scan_file_id);

-- One row per tool call, ordered. Keyed on turn_id rather than message_id so a
-- turn without a message_id still has first-class rows instead of being a
-- query-time residual.
CREATE TABLE IF NOT EXISTS turn_tools (
    turn_id   INTEGER NOT NULL,
    seq       INTEGER NOT NULL,
    tool_name TEXT NOT NULL,
    PRIMARY KEY (turn_id, seq)
);

CREATE INDEX IF NOT EXISTS ix_turn_tools_name ON turn_tools(tool_name);

CREATE TABLE IF NOT EXISTS agents (
    agent_id          TEXT PRIMARY KEY,
    agent_type        TEXT,
    parent_session_id TEXT,
    completed_ts      TEXT,
    status            TEXT,
    total_tokens      INTEGER,
    total_duration_ms INTEGER,
    tool_use_count    INTEGER
);

CREATE INDEX IF NOT EXISTS ix_agents_type ON agents(agent_type);

-- Materialised per scan and deliberately mapping-independent: it counts raw
-- tool names, so a session's derived activity is a cheap join against whatever
-- tool-to-category mapping is current. Editing the mapping never needs a rescan.
CREATE TABLE IF NOT EXISTS session_tool_counts (
    session_id TEXT NOT NULL,
    tool_name  TEXT NOT NULL,
    calls      INTEGER NOT NULL,
    PRIMARY KEY (session_id, tool_name)
);

-- User-assigned labels. In the database rather than in browser storage, because
-- they are data about sessions and belong with the sessions.
CREATE TABLE IF NOT EXISTS tags (
    name       TEXT PRIMARY KEY,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS session_tags (
    session_id TEXT NOT NULL,
    tag        TEXT NOT NULL,
    PRIMARY KEY (session_id, tag)
);

CREATE INDEX IF NOT EXISTS ix_session_tags_tag ON session_tags(tag);
"#;

const V2: &str = r#"
-- The activity derived from a session's tool mix, materialised so the sessions
-- list can filter, sort and group on it in SQL.
--
-- It depends on the tool-to-category mapping, which the user can edit, so it is
-- rebuilt whenever that mapping changes. That is cheap because
-- session_tool_counts holds raw tool names and encodes no mapping itself — the
-- rebuild never touches a transcript.
CREATE TABLE IF NOT EXISTS session_activity (
    session_id   TEXT PRIMARY KEY,
    activity     TEXT NOT NULL,
    profile_json TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS ix_session_activity ON session_activity(activity);
"#;

const V3: &str = r#"
-- What a project is meant to get done, in the order it should happen.
--
-- The app's own list, not Claude Code's: these are the features and stories a
-- session is started for, and the only table here the user writes by hand.
CREATE TABLE IF NOT EXISTS tasks (
    id           INTEGER PRIMARY KEY,
    project_path TEXT NOT NULL,
    title        TEXT NOT NULL,
    notes        TEXT NOT NULL DEFAULT '',
    -- open, running, waiting, deferred, done
    state        TEXT NOT NULL DEFAULT 'open',
    position     INTEGER NOT NULL DEFAULT 0,
    created_ts   TEXT NOT NULL,
    -- The session that worked on it, once one has.
    session_id   TEXT
);

CREATE INDEX IF NOT EXISTS ix_tasks_project ON tasks(project_path, position);
"#;

const V4: &str = r#"
-- What a tool call was pointed at, and how to find out whether it worked.
--
-- tool_use_id is what pairs a call with its outcome; older transcripts wrote
-- none, so both of these stay nullable and every query treats them as optional.
ALTER TABLE turn_tools ADD COLUMN tool_use_id TEXT;
ALTER TABLE turn_tools ADD COLUMN file_path   TEXT;

CREATE INDEX IF NOT EXISTS ix_turn_tools_use_id ON turn_tools(tool_use_id);
CREATE INDEX IF NOT EXISTS ix_turn_tools_file   ON turn_tools(file_path);

-- The outcome of a tool call, in its own table rather than as a column on the
-- call. A result lands on a later record than the call that produced it, so an
-- incremental scan regularly reads the two halves in different passes; joined
-- at query time they find each other whenever both have arrived.
CREATE TABLE IF NOT EXISTS tool_results (
    tool_use_id  TEXT PRIMARY KEY,
    is_error     INTEGER NOT NULL DEFAULT 0,
    scan_file_id INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS ix_tool_results_file ON tool_results(scan_file_id);

-- Things the user did that are not turns: a compaction, or a slash command.
-- Rows, not counters, for the same reason the turn figures are recomputed —
-- a tally written during a partial scan drifts.
CREATE TABLE IF NOT EXISTS session_events (
    id           INTEGER PRIMARY KEY,
    session_id   TEXT NOT NULL,
    uuid         TEXT,
    ts_utc       TEXT NOT NULL,
    -- compact-auto, compact-manual, slash
    kind         TEXT NOT NULL,
    detail       TEXT,
    pre_tokens   INTEGER,
    scan_file_id INTEGER NOT NULL
);

-- Same trick as the turns: a repeat scan of the same line is a no-op.
CREATE UNIQUE INDEX IF NOT EXISTS ux_session_events_uuid
    ON session_events(uuid) WHERE uuid IS NOT NULL AND uuid <> '';

CREATE INDEX IF NOT EXISTS ix_session_events_session ON session_events(session_id, kind);
CREATE INDEX IF NOT EXISTS ix_session_events_file    ON session_events(scan_file_id);

ALTER TABLE sessions ADD COLUMN compact_auto   INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sessions ADD COLUMN compact_manual INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sessions ADD COLUMN files_touched  INTEGER NOT NULL DEFAULT 0;

-- None of the above can be backfilled from what is already stored, and this
-- database is a cache of the transcripts. Forgetting what was scanned makes the
-- next scan read every file once and fill it all in.
DELETE FROM scan_files;
"#;

const V5: &str = r#"
-- Which agent wrote the transcript a session was read from.
--
-- Every session already in the database came from Claude Code, so the default
-- is the whole backfill. It lives on the session rather than on the file
-- because that is what every list, filter and facet reads.
ALTER TABLE sessions ADD COLUMN source TEXT NOT NULL DEFAULT 'claude';

CREATE INDEX IF NOT EXISTS ix_sessions_source ON sessions(source);
"#;
