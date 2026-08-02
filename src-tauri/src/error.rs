//! The one error type every command returns.
//!
//! It serialises as `{ kind, message }` rather than a bare string, so the
//! frontend can branch on the kind without parsing prose.

use serde::{Serialize, Serializer};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Database(#[from] rusqlite::Error),

    #[error("{0}")]
    Io(#[from] std::io::Error),

    /// Anything the domain modules report through `anyhow`, with their context
    /// chain preserved.
    #[error("{0:#}")]
    Internal(#[from] anyhow::Error),

    /// The frontend asked for something that cannot exist. Never a bug in the
    /// backend, so it is worth telling apart from the rest.
    #[error("{0}")]
    BadRequest(String),
}

impl Error {
    fn kind(&self) -> &'static str {
        match self {
            Self::Database(_) => "database",
            Self::Io(_) => "io",
            Self::Internal(_) => "internal",
            Self::BadRequest(_) => "badRequest",
        }
    }
}

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Wire<'a> {
            kind: &'a str,
            message: String,
        }

        Wire {
            kind: self.kind(),
            message: self.to_string(),
        }
        .serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialises_with_kind_and_message() {
        let error = Error::BadRequest("no such project".to_owned());
        let json = serde_json::to_value(&error).unwrap();
        assert_eq!(json["kind"], "badRequest");
        assert_eq!(json["message"], "no such project");
    }

    #[test]
    fn keeps_the_anyhow_context_chain() {
        let cause = anyhow::anyhow!("file is missing").context("reading ~/.claude.json");
        let json = serde_json::to_value(Error::from(cause)).unwrap();
        assert_eq!(json["kind"], "internal");
        assert_eq!(
            json["message"],
            "reading ~/.claude.json: file is missing",
            "the context chain is what makes a report actionable"
        );
    }
}
