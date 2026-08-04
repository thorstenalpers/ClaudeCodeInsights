pub mod activity;
pub mod categories;
pub mod cost;
pub mod rhythm;
pub mod series;
pub mod sessions;
pub mod transcript;
pub mod usage;

/// The project path, spelling-blind, as SQL.
///
/// `~/.claude.json` registers the same directory twice — once with `\` and once
/// with `/` — and a transcript carries whichever the shell used. Compared as
/// written, a project's own page asks for rows that exist under the other
/// spelling and finds none.
pub const SAME_PATH: &str = "rtrim(replace(lower(COALESCE(s.project_path, '')), '\\', '/'), '/')";

/// The same rule in Rust, for the value bound against it.
pub fn same_path(path: &str) -> String {
    path.to_lowercase()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_owned()
}
