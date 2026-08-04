//! The one place Windows keeps its voices.
//!
//! Dictation used to live here as well — `Windows.Media.SpeechRecognition`, and
//! later a recogniser of this app's own. Both are gone: this app reads and
//! speaks, it does not listen. What is left is the way to the page where the
//! voices are installed, which is a setting about output.

#[cfg(not(windows))]
use crate::error::Error;
use crate::error::Result;

/// Where Windows installs the voices, and nothing else.
#[cfg(windows)]
const SETTINGS_URI: &str = "ms-settings:speech";

/// Opens the page where Windows installs voices.
///
/// The address is a constant for the same reason the free-key page is one:
/// nothing the window says becomes an argument to the shell.
pub fn open_settings() -> Result<()> {
    #[cfg(windows)]
    std::process::Command::new("explorer.exe")
        .arg(SETTINGS_URI)
        .spawn()?;

    #[cfg(not(windows))]
    return Err(Error::BadRequest("speech settings need Windows".to_owned()));

    #[cfg(windows)]
    Ok(())
}
