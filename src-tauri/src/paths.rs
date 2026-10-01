use std::path::PathBuf;

/// Everything the app writes lives under one directory.
///
/// The transcript folders it reads are treated as foreign, read-only territory:
/// nothing is written there, and nothing is written next to the executable.
pub fn data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("ClaudeAdmin")
}

pub fn database_path() -> PathBuf {
    data_dir().join("usage.db")
}

/// Where Claude Code keeps its transcripts, when the user has not said otherwise.
///
/// The macOS path belongs to the Xcode integration and is kept so the same
/// default list works if the app is ever built for it.
pub fn claude_scan_roots() -> Vec<PathBuf> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };

    let candidates = [
        home.join(".claude").join("projects"),
        home.join("Library")
            .join("Developer")
            .join("Xcode")
            .join("CodingAssistant")
            .join("ClaudeAgentConfig")
            .join("projects"),
    ];

    candidates.into_iter().filter(|p| p.is_dir()).collect()
}

/// Where the Codex CLI keeps its rollouts, one directory per day.
///
/// The same folder serves the VS Code extension and the terminal; a rollout
/// records which of the two started it.
pub fn codex_scan_roots() -> Vec<PathBuf> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };

    [home.join(".codex").join("sessions")]
        .into_iter()
        .filter(|p| p.is_dir())
        .collect()
}

/// Every folder that is read, whichever agent wrote it.
pub fn default_scan_roots() -> Vec<PathBuf> {
    crate::ingest::source::ALL
        .iter()
        .flat_map(|source| source.roots())
        .collect()
}

/// Claude Code's own configuration. Read freely; every write must go through
/// a backup first — see `projects::modify_config`.
pub fn claude_config_path() -> anyhow::Result<PathBuf> {
    dirs::home_dir()
        .map(|home| home.join(".claude.json"))
        .ok_or_else(|| anyhow::anyhow!("no home directory"))
}

pub fn ensure_data_dir() -> std::io::Result<()> {
    std::fs::create_dir_all(data_dir())
}
