//! The agents whose transcripts are read, and what tells them apart.
//!
//! Each one writes `.jsonl` under a folder of its own, in a format of its own.
//! A file is attributed to a source by the root it was found under rather than
//! by its contents: the folder is what the discovery walked, and guessing from
//! the first line would misread a transcript whose head has been rewritten.

use super::reader::FileParse;
use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Claude,
    Codex,
}

pub const ALL: [Source; 2] = [Source::Claude, Source::Codex];

impl Source {
    /// The value stored on a session row, and the one the window filters by.
    pub fn id(self) -> &'static str {
        match self {
            Source::Claude => "claude",
            Source::Codex => "codex",
        }
    }

    pub fn roots(self) -> Vec<PathBuf> {
        match self {
            Source::Claude => crate::paths::claude_scan_roots(),
            Source::Codex => crate::paths::codex_scan_roots(),
        }
    }

    pub fn parse(self, path: &Path, skip_lines: u64) -> Result<FileParse> {
        match self {
            Source::Claude => super::reader::parse_file(path, skip_lines),
            Source::Codex => super::codex::parse_file(path, skip_lines),
        }
    }

    pub fn from_id(id: &str) -> Source {
        ALL.into_iter()
            .find(|source| source.id() == id)
            .unwrap_or(Source::Claude)
    }

    /// The source a loose path belongs to, for the live view, which follows a
    /// file before any session row exists for it.
    ///
    /// Read from the folder the agent owns rather than from the roots, which
    /// are filtered to the ones present: a file must still be attributable
    /// while its directory is being created.
    pub fn of_path(path: &Path) -> Source {
        let owns = |name: &str| {
            path.components()
                .any(|part| part.as_os_str().eq_ignore_ascii_case(name))
        };
        if owns(".codex") {
            return Source::Codex;
        }
        Source::Claude
    }
}
