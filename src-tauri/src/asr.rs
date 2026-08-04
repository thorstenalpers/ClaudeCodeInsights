//! Dictation that runs in this app rather than in Windows.
//!
//! Windows' own recogniser takes no capture device — it hears whatever is set
//! as the system default — and only speaks the languages Windows has packs for,
//! which is why German dictation fails on an English machine. This path records
//! from a microphone the user picked and reads a Whisper model that lives in
//! this app's data folder, so both of those stop being Windows' decision.
//!
//! Nothing leaves the machine here either: the model is downloaded once, from
//! the same release page the voices come from, and every sample is decoded
//! locally.

use crate::error::{Error, Result};
use crate::tts::Progress;
use serde::Serialize;
use sherpa_onnx::{
    LinearResampler, OfflineModelConfig, OfflineRecognizer, OfflineRecognizerConfig,
    OfflineWhisperModelConfig, SileroVadModelConfig, VadModelConfig, VoiceActivityDetector,
};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Runtime};

/// What the recogniser works at; everything recorded is resampled to it.
const RATE: i32 = 16_000;

/// How long to wait for someone to start speaking before giving up.
const PATIENCE: Duration = Duration::from_secs(10);

/// The longest a single dictation may run, speech included.
const LIMIT: Duration = Duration::from_secs(60);

/// A model that can be fetched, as the window lists it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub label: String,
    /// Roughly, for the confirmation before a download starts.
    pub megabytes: u32,
    pub installed: bool,
}

struct Model {
    id: &'static str,
    label: &'static str,
    megabytes: u32,
    /// The prefix the files in the archive carry, e.g. `tiny` in `tiny-encoder.onnx`.
    stem: &'static str,
}

/// The models this app offers, both multilingual.
///
/// Whisper rather than a Zipformer: the point of this path is a language
/// Windows has no pack for, and the English-only models would trade one gap for
/// another. Tiny is the one that keeps dictation quick on a laptop; base is
/// noticeably better on names and long sentences.
const MODELS: [Model; 2] = [
    Model {
        id: "whisper-tiny",
        label: "Whisper Tiny (multilingual)",
        megabytes: 111,
        stem: "tiny",
    },
    Model {
        id: "whisper-base",
        label: "Whisper Base (multilingual)",
        megabytes: 198,
        stem: "base",
    },
];

const RELEASE: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models";

/// The endpointer. Small enough to come along with every model.
const VAD_FILE: &str = "silero_vad.onnx";

fn find(id: &str) -> Option<&'static Model> {
    MODELS.iter().find(|model| model.id == id)
}

fn models_dir() -> Result<PathBuf> {
    let dir = crate::paths::data_dir().join("speech");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn model_dir(model: &Model) -> Result<PathBuf> {
    Ok(models_dir()?.join(format!("sherpa-onnx-{}", model.id)))
}

/// The quantised file where there is one: a third of the size and no worse at
/// dictation, which is the only thing this path does.
fn pick(dir: &Path, stem: &str, part: &str) -> Option<PathBuf> {
    let quantised = dir.join(format!("{stem}-{part}.int8.onnx"));
    let plain = dir.join(format!("{stem}-{part}.onnx"));
    quantised
        .is_file()
        .then_some(quantised)
        .or_else(|| plain.is_file().then_some(plain))
}

fn is_installed(model: &Model) -> bool {
    let Ok(dir) = model_dir(model) else {
        return false;
    };
    pick(&dir, model.stem, "encoder").is_some()
        && pick(&dir, model.stem, "decoder").is_some()
        && dir.join(format!("{}-tokens.txt", model.stem)).is_file()
}

pub fn list() -> Vec<ModelInfo> {
    MODELS
        .iter()
        .map(|model| ModelInfo {
            id: model.id.to_owned(),
            label: model.label.to_owned(),
            megabytes: model.megabytes,
            installed: is_installed(model),
        })
        .collect()
}

pub fn folder() -> Result<String> {
    Ok(models_dir()?.to_string_lossy().into_owned())
}

/// Fetches a model and the endpointer beside it.
pub fn install<R: Runtime>(app: &AppHandle<R>, id: &str) -> Result<ModelInfo> {
    let model =
        find(id).ok_or_else(|| Error::BadRequest(format!("unknown speech model '{id}'")))?;
    let report = |progress: Progress| {
        let _ = app.emit("voice:progress", progress);
    };

    if !is_installed(model) {
        let url = format!("{RELEASE}/sherpa-onnx-{}.tar.bz2", model.id);
        log::info!("speech model '{id}': fetching {url}");
        let bytes = crate::tts::download(&url, id, 0, None, &report)?;
        let decoder = bzip2::read::BzDecoder::new(Cursor::new(bytes));
        tar::Archive::new(decoder).unpack(models_dir()?)?;
    }

    let vad = models_dir()?.join(VAD_FILE);
    if !vad.is_file() {
        log::info!("speech model '{id}': fetching the endpointer");
        let bytes = crate::tts::download(&format!("{RELEASE}/{VAD_FILE}"), id, 0, None, &report)?;
        fs::write(&vad, bytes)?;
    }

    if !is_installed(model) {
        return Err(Error::BadRequest(
            "the download did not contain a usable model".to_owned(),
        ));
    }

    log::info!("speech model '{id}': installed");
    Ok(ModelInfo {
        id: model.id.to_owned(),
        label: model.label.to_owned(),
        megabytes: model.megabytes,
        installed: true,
    })
}

pub fn remove(id: &str) -> Result<()> {
    let model =
        find(id).ok_or_else(|| Error::BadRequest(format!("unknown speech model '{id}'")))?;
    let dir = model_dir(model)?;
    if dir.is_dir() {
        fs::remove_dir_all(&dir)?;
    }
    Ok(())
}

fn stopping() -> &'static AtomicBool {
    static STOP: AtomicBool = AtomicBool::new(false);
    &STOP
}

/// Ends the current dictation early. The samples heard so far are still read.
pub fn stop() {
    stopping().store(true, Ordering::SeqCst);
}

/// Records from `device` until the speaker stops, and reads back what was said.
///
/// `device` is a name from the microphone list, or none for whatever Windows
/// hands out by default — the only choice the Windows recogniser ever offered.
pub fn listen(device: Option<&str>, model_id: &str, locale: &str) -> Result<String> {
    let model =
        find(model_id).ok_or_else(|| Error::BadRequest(format!("unknown model '{model_id}'")))?;
    if !is_installed(model) {
        return Err(Error::BadRequest("speech-model-missing".to_owned()));
    }
    let dir = model_dir(model)?;

    let vad_path = models_dir()?.join(VAD_FILE);
    if !vad_path.is_file() {
        return Err(Error::BadRequest("speech-model-missing".to_owned()));
    }

    let detector = VoiceActivityDetector::create(
        &VadModelConfig {
            silero_vad: SileroVadModelConfig {
                model: Some(vad_path.to_string_lossy().into_owned()),
                threshold: 0.5,
                min_silence_duration: 0.6,
                min_speech_duration: 0.25,
                window_size: 512,
                max_speech_duration: 20.0,
            },
            sample_rate: RATE,
            num_threads: 1,
            ..Default::default()
        },
        30.0,
    )
    .ok_or_else(|| Error::BadRequest("the endpointer could not be loaded".to_owned()))?;

    let heard = record(device, &detector)?;
    if heard.is_empty() {
        return Ok(String::new());
    }

    let recognizer = OfflineRecognizer::create(&OfflineRecognizerConfig {
        model_config: OfflineModelConfig {
            whisper: OfflineWhisperModelConfig {
                encoder: pick(&dir, model.stem, "encoder").map(path_string),
                decoder: pick(&dir, model.stem, "decoder").map(path_string),
                // Whisper guesses the language when it is not told, and guesses
                // wrong on a short phrase often enough to be worth telling.
                language: Some(primary(locale)),
                task: Some("transcribe".to_owned()),
                ..Default::default()
            },
            tokens: Some(path_string(dir.join(format!("{}-tokens.txt", model.stem)))),
            num_threads: 2,
            ..Default::default()
        },
        ..Default::default()
    })
    .ok_or_else(|| Error::BadRequest("the speech model could not be loaded".to_owned()))?;

    let stream = recognizer.create_stream();
    stream.accept_waveform(RATE, &heard);
    recognizer.decode(&stream);

    Ok(stream
        .get_result()
        .map(|result| result.text.trim().to_owned())
        .unwrap_or_default())
}

fn path_string(path: PathBuf) -> String {
    path.to_string_lossy().into_owned()
}

/// `de-DE` is `de` to Whisper, which takes the language alone.
fn primary(locale: &str) -> String {
    locale
        .to_lowercase()
        .replace('_', "-")
        .split('-')
        .next()
        .unwrap_or("en")
        .to_owned()
}

/// Opens the microphone and returns the first thing said, at 16 kHz mono.
fn record(device: Option<&str>, detector: &VoiceActivityDetector) -> Result<Vec<f32>> {
    use rodio::cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    let host = rodio::cpal::default_host();
    let input = match device {
        Some(name) => host
            .input_devices()
            .ok()
            .and_then(|mut devices| {
                devices.find(|candidate| candidate.name().is_ok_and(|found| found == name))
            })
            // A microphone that was unplugged since the list was drawn must not
            // end dictation: the default one is still better than an error.
            .or_else(|| host.default_input_device()),
        None => host.default_input_device(),
    }
    .ok_or_else(|| Error::BadRequest("this machine has no capture device".to_owned()))?;

    let config = input
        .default_input_config()
        .map_err(|error| Error::BadRequest(error.to_string()))?;
    let rate = config.sample_rate().0;
    let channels = config.channels() as usize;

    let taken: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let writer = Arc::clone(&taken);
    let stream = input
        .build_input_stream(
            &config.config(),
            move |samples: &[f32], _| {
                // Whatever the device offers is mixed down: the model wants one
                // channel and a stereo headset would otherwise be read twice.
                let Ok(mut buffer) = writer.lock() else {
                    return;
                };
                buffer.extend(
                    samples
                        .chunks(channels)
                        .map(|frame| frame.iter().sum::<f32>() / channels as f32),
                );
            },
            |error| log::warn!("dictation: {error}"),
            None,
        )
        .map_err(|error| Error::BadRequest(error.to_string()))?;

    stopping().store(false, Ordering::SeqCst);
    stream
        .play()
        .map_err(|error| Error::BadRequest(error.to_string()))?;

    let resampler = (rate as i32 != RATE)
        .then(|| LinearResampler::create(rate as i32, RATE))
        .flatten();

    let started = Instant::now();
    let mut spoke = false;
    let mut pending: Vec<f32> = Vec::new();
    let mut segment = None;

    while segment.is_none() {
        std::thread::sleep(Duration::from_millis(50));

        let fresh = match taken.lock() {
            Ok(mut buffer) => std::mem::take(&mut *buffer),
            Err(_) => break,
        };
        let fresh = match &resampler {
            Some(resampler) => resampler.resample(&fresh, false),
            None => fresh,
        };
        pending.extend(fresh);

        // Silero reads one window at a time and keeps nothing of a partial one.
        while pending.len() >= 512 {
            let window: Vec<f32> = pending.drain(..512).collect();
            detector.accept_waveform(&window);
        }
        spoke |= detector.detected();

        if !detector.is_empty() {
            segment = detector.front().map(|found| found.samples().to_vec());
            detector.pop();
            break;
        }

        let waited = started.elapsed();
        if stopping().load(Ordering::SeqCst) || waited > LIMIT || (!spoke && waited > PATIENCE) {
            break;
        }
    }

    let _ = stream.pause();
    drop(stream);

    // A dictation cut short by the stop button still has to say what it heard,
    // so the flushed tail counts as much as a segment the endpointer closed.
    if segment.is_none() {
        detector.flush();
        segment = detector.front().map(|found| found.samples().to_vec());
    }

    Ok(segment.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_model_is_reachable_by_id() {
        for model in list() {
            assert!(find(&model.id).is_some(), "{} has no entry", model.id);
        }
        assert!(find("whisper-large").is_none());
    }

    #[test]
    fn a_language_tag_is_cut_down_to_what_whisper_takes() {
        assert_eq!(primary("de-DE"), "de");
        assert_eq!(primary("en_US"), "en");
        assert_eq!(primary("fr"), "fr");
    }

    #[test]
    fn the_quantised_file_wins_where_both_are_there() {
        let dir = std::env::temp_dir().join("claude-admin-asr-pick");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("tiny-encoder.onnx"), b"x").unwrap();
        assert!(
            pick(&dir, "tiny", "encoder")
                .unwrap()
                .ends_with("tiny-encoder.onnx")
        );

        fs::write(dir.join("tiny-encoder.int8.onnx"), b"x").unwrap();
        assert!(
            pick(&dir, "tiny", "encoder")
                .unwrap()
                .ends_with("tiny-encoder.int8.onnx")
        );
        assert!(pick(&dir, "tiny", "decoder").is_none());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    #[ignore = "downloads a model of a hundred megabytes or more"]
    fn a_model_installs() {
        let id = std::env::var("MODEL").unwrap_or_else(|_| "whisper-tiny".to_owned());
        let model = find(&id).unwrap();
        println!("installed: {}", is_installed(model));
    }
}
