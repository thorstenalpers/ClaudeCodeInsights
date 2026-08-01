use std::path::PathBuf;

/// Everything the app writes lives under one directory.
///
/// The transcript folders it reads are treated as foreign, read-only territory:
/// nothing is written there, and nothing is written next to the executable.
pub fn data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("ClaudeUsageAnalyzer")
}

pub fn database_path() -> PathBuf {
    data_dir().join("usage.db")
}

/// Where Claude Code keeps its transcripts, when the user has not said otherwise.
///
/// The macOS path belongs to the Xcode integration and is kept so the same
/// default list works if the app is ever built for it.
pub fn default_scan_roots() -> Vec<PathBuf> {
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

pub fn ensure_data_dir() -> std::io::Result<()> {
    std::fs::create_dir_all(data_dir())
}
