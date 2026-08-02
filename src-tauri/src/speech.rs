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

    pub fn recognize(locale: &str) -> Result<String> {
        let recognizer = match language(locale) {
            Some(language) => SpeechRecognizer::Create(&language),
            None => SpeechRecognizer::new(),
        }
        .map_err(failed)?;

        // WinRT hands back futures; the command already runs on a worker, so
        // blocking on them here is what keeps this function readable.
        pollster::block_on(recognizer.CompileConstraintsAsync().map_err(failed)?).map_err(failed)?;

        let result = pollster::block_on(recognizer.RecognizeAsync().map_err(failed)?).map_err(failed)?;

        let status = result.Status().map_err(failed)?;

        // Silence is an outcome, not a failure: the caller shows the box empty
        // rather than an error nobody caused.
        if status != SpeechRecognitionResultStatus::Success {
            return Ok(String::new());
        }

        Ok(result.Text().map_err(failed)?.to_string())
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;

    pub fn available() -> bool {
        false
    }

    pub fn recognize(_locale: &str) -> Result<String> {
        Err(Error::BadRequest(
            "speech recognition needs Windows".to_owned(),
        ))
    }
}

pub use imp::{available, recognize};

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
    #[ignore = "opens the microphone and waits for the developer to speak"]
    fn hears_something() {
        let heard = recognize("en-US").unwrap();
        println!("heard: {heard:?}");
    }
}
