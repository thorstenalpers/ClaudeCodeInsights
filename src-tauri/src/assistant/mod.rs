//! Where an answer comes from.
//!
//! Two kinds of source: the Claude Code binary already on this machine, which
//! needs no key and no network from this app, and a hosted model, which needs
//! both. The choice is the user's and it is explicit, because it decides
//! whether a summary of their usage leaves this machine at all.

pub mod cli;
pub mod providers;
pub mod secrets;

use crate::error::{Error, Result};
use cli::Answer;

/// The id the local CLI answers to, alongside the hosted providers.
pub const LOCAL: &str = "claude-code";

/// Routes one question to the chosen source.
///
/// `model` and `effort` only reach the local CLI; a hosted provider is pinned
/// to the model its entry names, and pretending otherwise would put a control
/// on screen that changes nothing.
pub fn ask(
    source: &str,
    cli_path: Option<&str>,
    prompt: &str,
    model: Option<&str>,
    effort: Option<&str>,
) -> Result<Answer> {
    if prompt.trim().is_empty() {
        return Err(Error::BadRequest("the prompt is empty".to_owned()));
    }

    if source == LOCAL {
        return cli::ask(cli_path, prompt, model, effort);
    }

    let provider = providers::find(source)
        .ok_or_else(|| Error::BadRequest(format!("unknown source '{source}'")))?;
    let key = secrets::get(source)?
        .ok_or_else(|| Error::BadRequest(format!("no API key set for {source}")))?;

    Ok(Answer {
        text: providers::ask(provider, &key, prompt)?,
        model: Some(provider.model.to_owned()),
        ..Answer::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_prompt_never_reaches_a_source() {
        assert!(ask(LOCAL, None, "   ", None, None).is_err());
        assert!(ask("openai", None, "", None, None).is_err());
    }

    #[test]
    fn an_unknown_source_is_refused() {
        let error = ask("telepathy", None, "question", None, None).unwrap_err();
        assert!(matches!(error, Error::BadRequest(_)));
    }
}
