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

    pub fn available() -> bool {
        SpeechRecognizer::new().is_ok()
    }

    pub fn recognize(locale: &str) -> Result<String> {
        let recognizer = match language(locale) {
            Some(language) => SpeechRecognizer::Create(&language),
            None => SpeechRecognizer::new(),
        }
        .map_err(|error| Error::BadRequest(error.message()))?;

        // WinRT hands back futures; the command already runs on a worker, so
        // blocking on them here is what keeps this function readable.
        pollster::block_on(
            recognizer
                .CompileConstraintsAsync()
                .map_err(|error| Error::BadRequest(error.message()))?,
        )
        .map_err(|error| Error::BadRequest(error.message()))?;

        let result = pollster::block_on(
            recognizer
                .RecognizeAsync()
                .map_err(|error| Error::BadRequest(error.message()))?,
        )
        .map_err(|error| Error::BadRequest(error.message()))?;

        let status = result
            .Status()
            .map_err(|error| Error::BadRequest(error.message()))?;

        // Silence is an outcome, not a failure: the caller shows the box empty
        // rather than an error nobody caused.
        if status != SpeechRecognitionResultStatus::Success {
            return Ok(String::new());
        }

        Ok(result
            .Text()
            .map_err(|error| Error::BadRequest(error.message()))?
            .to_string())
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
