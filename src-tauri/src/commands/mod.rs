//! The Tauri command surface, one module per domain.
//!
//! Commands stay thin: open a connection, call the domain, let `?` turn a
//! failure into `Error`. Anything longer belongs in the domain module.

pub mod overview;
pub mod projects;
pub mod scan;
pub mod sessions;

use crate::paths;
use crate::storage;
use rusqlite::Connection;

fn db() -> crate::error::Result<Connection> {
    Ok(storage::open(&paths::database_path())?)
}
