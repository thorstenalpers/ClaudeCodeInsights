//! Dictation through Windows' own speech recogniser.
//!
//! Not the Web Speech API: WebView2 does not implement recognition, and the
//! Chrome implementation that does sends audio to a Google service. `Windows.
//! Media.SpeechRecognition` runs on the machine against the installed language
//! pack, which keeps a dictated question as private as a typed one and costs
//! the binary nothing but bindings.
//!
//! Synthesis is deliberately absent here — the WebView already speaks through
//! the same Windows voices, and a second path would be a second thing to keep
//! working.

use crate::error::{Error, Result};

/// Windows refuses to recognise until the speech privacy policy is accepted.
///
/// It is the one failure the user can actually do something about, and the
/// system message for it says nothing about where to go — so it is caught by
/// code and answered with directions.
pub const PRIVACY_NOT_ACCEPTED: i32 = 0x8004_5509_u32 as i32;

/// Where Windows keeps both the recogniser's language pack and the voices.
#[cfg(windows)]
const SETTINGS_URI: &str = "ms-settings:speech";

/// Where the default input device is chosen, which is the only way to change
/// which microphone the recogniser hears.
#[cfg(windows)]
const SOUND_URI: &str = "ms-settings:sound";

/// One capture device Windows knows about.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Microphone {
    pub name: String,
    /// True for the one Windows hands to anything that does not ask for a
    /// device — which is every recogniser, this one included.
    pub is_default: bool,
}

/// The capture devices, with the default marked.
///
/// Listed rather than chosen from: `Windows.Media.SpeechRecognition` takes no
/// device, it listens to whatever is default. Naming them is still worth it —
/// a microphone that does nothing is usually the wrong one being default, and
/// this is how a reader finds that out.
pub fn microphones() -> Vec<Microphone> {
    use rodio::cpal::traits::{DeviceTrait, HostTrait};

    let host = rodio::cpal::default_host();
    let default = host
        .default_input_device()
        .and_then(|device| device.name().ok());

    host.input_devices()
        .map(|devices| {
            devices
                .filter_map(|device| device.name().ok())
                .map(|name| Microphone {
                    is_default: Some(&name) == default.as_ref(),
                    name,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The language part of a tag: `de-DE`, `de_DE` and `de` all give `de`.
pub fn primary(tag: &str) -> String {
    tag.to_lowercase()
        .replace('_', "-")
        .split('-')
        .next()
        .unwrap_or_default()
        .to_owned()
}

/// Whether one of the installed recogniser languages covers `wanted`.
///
/// Matched on the primary subtag: a machine with `de-DE` dictates German for an
/// app set to plain `de`, and refusing that would send the user looking for a
/// pack they already have.
fn supports(installed: &[String], wanted: &str) -> bool {
    let wanted = primary(wanted);
    installed.iter().any(|tag| primary(tag) == wanted)
}

/// Opens the page where the default input device is set.
pub fn open_sound_settings() -> Result<()> {
    #[cfg(windows)]
    std::process::Command::new("explorer.exe")
        .arg(SOUND_URI)
        .spawn()?;

    #[cfg(not(windows))]
    return Err(Error::BadRequest("only Windows has this page".to_owned()));

    #[cfg(windows)]
    Ok(())
}

#[cfg(windows)]
mod imp {
    use super::*;
    use windows::Globalization::Language;
    use windows::Media::SpeechRecognition::{SpeechRecognitionResultStatus, SpeechRecognizer};
    use windows::core::HSTRING;

    /// A BCP-47 tag Windows will accept, or none to take the system default.
    fn language(locale: &str) -> Option<Language> {
        Language::CreateLanguage(&HSTRING::from(locale)).ok()
    }

    /// Turns a WinRT failure into something the window can act on.
    fn failed(error: windows::core::Error) -> Error {
        if error.code().0 == PRIVACY_NOT_ACCEPTED {
            return Error::BadRequest("speech-privacy-not-accepted".to_owned());
        }
        Error::BadRequest(error.message())
    }

    /// Whether a recogniser can be constructed at all.
    ///
    /// Deliberately not a promise that dictation will work: the privacy policy
    /// is only checked when recognition actually starts, so the setting stays
    /// offered and the refusal is explained at the point it happens.
    pub fn available() -> bool {
        SpeechRecognizer::new().is_ok()
    }

    /// The languages this machine can actually dictate in.
    ///
    /// Installed separately from the display language and from the voices, so
    /// an English Windows with the app set to German has none for it — which is
    /// the whole of "The requested language is not supported".
    pub fn languages() -> Vec<String> {
        SpeechRecognizer::SupportedTopicLanguages()
            .into_iter()
            .flatten()
            .filter_map(|language| language.LanguageTag().ok())
            .map(|tag| tag.to_string())
            .collect()
    }

    pub fn recognize(locale: &str) -> Result<String> {
        let installed = languages();
        if !installed.is_empty() && !super::supports(&installed, locale) {
            return Err(Error::BadRequest(format!(
                "speech-language-missing:{locale}"
            )));
        }

        let recognizer = match language(locale) {
            Some(language) => SpeechRecognizer::Create(&language),
            None => SpeechRecognizer::new(),
        }
        .map_err(failed)?;

        // WinRT hands back futures; the command already runs on a worker, so
        // blocking on them here is what keeps this function readable.
        pollster::block_on(recognizer.CompileConstraintsAsync().map_err(failed)?)
            .map_err(failed)?;

        let result =
            pollster::block_on(recognizer.RecognizeAsync().map_err(failed)?).map_err(failed)?;

        let status = result.Status().map_err(failed)?;

        // Silence is an outcome, not a failure: the caller shows the box empty
        // rather than an error nobody caused.
        if status != SpeechRecognitionResultStatus::Success {
            return Ok(String::new());
        }

        Ok(result.Text().map_err(failed)?.to_string())
    }

    /// Opens the page where Windows installs voices and language packs.
    ///
    /// The address is a constant for the same reason the free-key page is one:
    /// nothing the window says becomes an argument to the shell.
    pub fn open_settings() -> Result<()> {
        std::process::Command::new("explorer.exe")
            .arg(SETTINGS_URI)
            .spawn()?;
        Ok(())
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;

    pub fn open_settings() -> Result<()> {
        Err(Error::BadRequest("speech settings need Windows".to_owned()))
    }

    pub fn available() -> bool {
        false
    }

    pub fn recognize(_locale: &str) -> Result<String> {
        Err(Error::BadRequest(
            "speech recognition needs Windows".to_owned(),
        ))
    }

    pub fn languages() -> Vec<String> {
        Vec::new()
    }
}

pub use imp::{available, languages, open_settings, recognize};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn availability_answers_without_a_microphone() {
        // Whichever way it goes, asking must not panic — the window calls this
        // on every start to decide whether to offer the setting at all.
        let _ = available();
    }

    #[test]
    fn the_capture_devices_can_be_listed() {
        // A machine without a microphone answers with an empty list; what must
        // not happen is a panic, because the settings page asks on every visit.
        let found = microphones();
        assert!(
            found.iter().filter(|entry| entry.is_default).count() <= 1,
            "Windows hands out one default, not several"
        );
        for entry in &found {
            println!("microphone {:40} default={}", entry.name, entry.is_default);
        }
    }

    #[test]
    fn a_language_is_matched_on_its_primary_subtag() {
        let installed = ["en-US".to_owned(), "en-GB".to_owned()];
        assert!(supports(&installed, "en"));
        assert!(
            supports(&installed, "en-AU"),
            "any English pack dictates en"
        );
        assert!(!supports(&installed, "de-DE"));
        assert!(!supports(&installed, "de"));
        assert!(supports(&["de-DE".to_owned()], "de_DE"), "underscores too");
    }

    #[test]
    fn the_recogniser_languages_can_be_listed() {
        // On a machine with speech installed this is the answer to "why does
        // German fail here"; on one without, it must still not panic.
        println!("recognition languages: {:?}", languages());
    }

    #[test]
    #[ignore = "opens the microphone and waits for the developer to speak"]
    fn hears_something() {
        let heard = recognize("en-US").unwrap();
        println!("heard: {heard:?}");
    }
}
