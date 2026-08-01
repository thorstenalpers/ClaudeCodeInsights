use anyhow::Result;
use rusqlite::Connection;

/// Bumped whenever a migration is added. Forward only — this is a cache that can
/// be rebuilt from the transcripts at any time, so there is no downgrade path.
const TARGET_VERSION: i64 = 1;

pub fn migrate(conn: &Connection) -> Result<()> {
    let current: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

    if current < 1 {
        conn.execute_batch(V1)?;
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
