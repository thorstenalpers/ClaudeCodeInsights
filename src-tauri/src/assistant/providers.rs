//! The hosted models the assistant can ask, and the one shape they share.
//!
//! Three of the five speak OpenAI's dialect, so they differ only in host, model
//! and the header the key rides in. Gemini and Anthropic each want their own
//! body, which is why the request is built per family rather than per provider.

use crate::error::{Error, Result};
use serde::Serialize;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(120);

/// How a provider wants to be spoken to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dialect {
    /// `POST /chat/completions` with a bearer token — OpenAI and its imitators.
    OpenAi,
    Gemini,
    Anthropic,
}

#[derive(Debug, Clone, Copy)]
pub struct Provider {
    pub id: &'static str,
    dialect: Dialect,
    endpoint: &'static str,
    pub model: &'static str,
    /// Where a free key can be had, for providers that give one away.
    pub free_key_url: Option<&'static str>,
}

/// The providers offered, in the order the picker shows them.
pub const PROVIDERS: &[Provider] = &[
    Provider {
        id: "gemini",
        dialect: Dialect::Gemini,
        endpoint: "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent",
        model: "gemini-2.0-flash",
        free_key_url: Some("https://aistudio.google.com/api-keys"),
    },
    Provider {
        id: "groq",
        dialect: Dialect::OpenAi,
        endpoint: "https://api.groq.com/openai/v1/chat/completions",
        model: "llama-3.3-70b-versatile",
        free_key_url: Some("https://console.groq.com/keys"),
    },
    Provider {
        id: "openai",
        dialect: Dialect::OpenAi,
        endpoint: "https://api.openai.com/v1/chat/completions",
        model: "gpt-4o-mini",
        free_key_url: None,
    },
    Provider {
        id: "anthropic",
        dialect: Dialect::Anthropic,
        endpoint: "https://api.anthropic.com/v1/messages",
        model: "claude-sonnet-4-5",
        free_key_url: None,
    },
    Provider {
        id: "mistral",
        dialect: Dialect::OpenAi,
        endpoint: "https://api.mistral.ai/v1/chat/completions",
        model: "mistral-small-latest",
        free_key_url: None,
    },
];

pub fn find(id: &str) -> Option<&'static Provider> {
    PROVIDERS.iter().find(|provider| provider.id == id)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub id: &'static str,
    pub model: &'static str,
    pub free_key_url: Option<&'static str>,
}

pub fn catalogue() -> Vec<ProviderInfo> {
    PROVIDERS
        .iter()
        .map(|provider| ProviderInfo {
            id: provider.id,
            model: provider.model,
            free_key_url: provider.free_key_url,
        })
        .collect()
}

/// The request body, in whichever dialect the provider expects.
fn body(provider: &Provider, prompt: &str) -> serde_json::Value {
    match provider.dialect {
        Dialect::OpenAi => serde_json::json!({
            "model": provider.model,
            "messages": [{ "role": "user", "content": prompt }],
        }),
        Dialect::Gemini => serde_json::json!({
            "contents": [{ "parts": [{ "text": prompt }] }],
        }),
        Dialect::Anthropic => serde_json::json!({
            "model": provider.model,
            "max_tokens": 1024,
            "messages": [{ "role": "user", "content": prompt }],
        }),
    }
}

/// Digs the answer out of whichever envelope came back.
fn extract(provider: &Provider, value: &serde_json::Value) -> Option<String> {
    let text = match provider.dialect {
        Dialect::OpenAi => value["choices"][0]["message"]["content"].as_str()?,
        Dialect::Gemini => value["candidates"][0]["content"]["parts"][0]["text"].as_str()?,
        Dialect::Anthropic => value["content"][0]["text"].as_str()?,
    };
    Some(text.trim().to_owned())
}

/// Asks one hosted provider and returns its answer.
///
/// The call is made here rather than in the WebView on purpose: the key never
/// reaches the frontend, and the window's content policy stays closed.
pub fn ask(provider: &Provider, key: &str, prompt: &str) -> Result<String> {
    if key.trim().is_empty() {
        return Err(Error::BadRequest(format!(
            "no API key set for {}",
            provider.id
        )));
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(TIMEOUT)
        .build()
        .map_err(|error| Error::BadRequest(error.to_string()))?;

    let mut request = client.post(provider.endpoint).json(&body(provider, prompt));

    request = match provider.dialect {
        Dialect::OpenAi => request.bearer_auth(key),
        // Gemini takes the key in a header rather than the query string, which
        // keeps it out of proxy logs and crash reports.
        Dialect::Gemini => request.header("x-goog-api-key", key),
        Dialect::Anthropic => request
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01"),
    };

    let response = request
        .send()
        .map_err(|error| Error::BadRequest(strip_key(&error.to_string(), key)))?;

    let status = response.status();
    let value: serde_json::Value = response
        .json()
        .map_err(|error| Error::BadRequest(strip_key(&error.to_string(), key)))?;

    if !status.is_success() {
        let message = value["error"]["message"]
            .as_str()
            .unwrap_or("the provider refused the request");
        return Err(Error::BadRequest(strip_key(message, key)));
    }

    extract(provider, &value)
        .ok_or_else(|| Error::BadRequest("the provider returned no answer".to_owned()))
}

/// Opens a provider's free-key page in the system browser.
///
/// It takes a provider id rather than a URL: the address then comes from the
/// table above and never from the window, so nothing the frontend can say ends
/// up as an argument to the shell.
pub fn open_free_key_url(id: &str) -> Result<()> {
    let provider = find(id).ok_or_else(|| Error::BadRequest(format!("unknown provider '{id}'")))?;
    let url = provider
        .free_key_url
        .ok_or_else(|| Error::BadRequest(format!("{id} has no free key page")))?;

    #[cfg(windows)]
    std::process::Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", url])
        .spawn()?;

    #[cfg(not(windows))]
    std::process::Command::new("xdg-open").arg(url).spawn()?;

    Ok(())
}

/// Keeps a key out of anything the window will show or log.
fn strip_key(message: &str, key: &str) -> String {
    if key.is_empty() {
        return message.to_owned();
    }
    message.replace(key, "***")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_provider_is_reachable_by_id() {
        for provider in PROVIDERS {
            assert!(find(provider.id).is_some());
        }
        assert!(find("nope").is_none());
    }

    #[test]
    fn each_dialect_builds_the_body_it_expects() {
        let openai = find("openai").unwrap();
        assert_eq!(body(openai, "hi")["messages"][0]["content"], "hi");

        let gemini = find("gemini").unwrap();
        assert_eq!(body(gemini, "hi")["contents"][0]["parts"][0]["text"], "hi");

        let anthropic = find("anthropic").unwrap();
        assert_eq!(body(anthropic, "hi")["messages"][0]["content"], "hi");
        assert!(body(anthropic, "hi")["max_tokens"].is_number());
    }

    #[test]
    fn each_dialect_finds_its_answer() {
        let openai = find("openai").unwrap();
        let value = serde_json::json!({ "choices": [{ "message": { "content": " yes " } }] });
        assert_eq!(extract(openai, &value).as_deref(), Some("yes"));

        let gemini = find("gemini").unwrap();
        let value =
            serde_json::json!({ "candidates": [{ "content": { "parts": [{ "text": "yes" }] } }] });
        assert_eq!(extract(gemini, &value).as_deref(), Some("yes"));

        let anthropic = find("anthropic").unwrap();
        let value = serde_json::json!({ "content": [{ "text": "yes" }] });
        assert_eq!(extract(anthropic, &value).as_deref(), Some("yes"));
    }

    #[test]
    fn an_answer_that_is_not_there_is_not_invented() {
        let openai = find("openai").unwrap();
        assert!(extract(openai, &serde_json::json!({ "choices": [] })).is_none());
    }

    #[test]
    fn a_missing_key_never_reaches_the_network() {
        let provider = find("openai").unwrap();
        assert!(ask(provider, "   ", "question").is_err());
    }

    #[test]
    fn only_a_known_provider_with_a_free_page_can_be_opened() {
        assert!(open_free_key_url("nope").is_err());
        // OpenAI charges, so it has no page to open and must not silently pass.
        assert!(open_free_key_url("openai").is_err());
    }

    #[test]
    fn a_key_is_scrubbed_from_anything_reported() {
        let message = strip_key("bad token sk-secret-123 rejected", "sk-secret-123");
        assert_eq!(message, "bad token *** rejected");
        assert!(!message.contains("sk-secret"));
    }
}
