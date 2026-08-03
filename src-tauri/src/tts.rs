//! Reading answers out through a neural voice that lives on this machine.
//!
//! Windows' own voices are what the window uses by default; they are installed
//! per language and many machines have only English. A Kokoro model fills that
//! gap: one file set per language pack, unpacked under this app's own data
//! folder, run through sherpa-onnx here rather than in the WebView.
//!
//! Fetching a pack is the second thing in this app that opens a network
//! connection, after the hosted assistant. It happens only when the user asks
//! for a pack by name, to an address from the table below — never to one the
//! window supplies.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Runtime};

/// Which family a pack belongs to, which decides both the files it must bring
/// and the branch of sherpa's config it is loaded through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    /// One model, a table of speaker embeddings, one language family.
    Kokoro,
    /// A Piper-style single-speaker voice.
    Vits,
}

/// Where a pack's bytes come from.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "camelCase")]
pub enum Origin {
    /// One `.tar.bz2` from the sherpa-onnx release page.
    Archive { url: String },
    /// A Hugging Face repository, downloaded file by file.
    Hub { repo: String },
    /// A Kokoro export that has to be assembled before sherpa can speak it.
    ///
    /// The community exports carry the model and a NumPy `.npz` of voices and
    /// nothing else. The token table and the phonemiser's data come from the
    /// sherpa mirror, the voices are rewritten as a flat table, and the
    /// metadata sherpa reads is appended to the model — which is the whole
    /// difference between a repository that cannot be spoken and one that can.
    KokoroExport {
        repo: String,
        model: String,
        voices: String,
    },
}

/// A voice pack that can be fetched, and where it comes from.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pack {
    pub id: String,
    /// BCP-47 tag the pack speaks, for matching against the window's language.
    pub language: String,
    pub label: String,
    /// Roughly, for the confirmation before a download starts.
    pub megabytes: u32,
    pub kind: Kind,
    #[serde(flatten)]
    pub origin: Origin,
    /// The folder the pack lives in, which is not always the id.
    pub dir: String,
}

/// What a pack installed from the hub leaves behind, so a later start knows
/// what it is looking at without asking the network again.
pub(crate) const MANIFEST: &str = "pack.json";

/// How many speakers a Vits pack turned out to have, written the first time it
/// is loaded. The number is the model's own answer and costs a load to get, so
/// it is kept rather than asked for on every listing.
const SPEAKERS: &str = "speakers.txt";

fn archive(id: &str, language: &str, label: &str, megabytes: u32, kind: Kind) -> Pack {
    Pack {
        id: id.to_owned(),
        language: language.to_owned(),
        label: label.to_owned(),
        megabytes,
        kind,
        origin: Origin::Archive {
            url: format!(
                "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/{id}.tar.bz2"
            ),
        },
        dir: id.to_owned(),
    }
}

/// The packs this app offers by name, before anything from the hub.
///
/// The multi-lang Kokoro is v1.1-zh: English and Chinese, not every language
/// the window speaks. It is named for what it says, because a pack labelled
/// "multilingual" that answers in Chinese is worse than no pack at all.
pub fn catalogue() -> Vec<Pack> {
    vec![
        Pack {
            id: "hub:Godelaune/Kokoro-82M-ONNX-German-Martin".to_owned(),
            language: "de".to_owned(),
            label: "Kokoro German (Martin)".to_owned(),
            megabytes: 330,
            kind: Kind::Kokoro,
            origin: Origin::KokoroExport {
                repo: "Godelaune/Kokoro-82M-ONNX-German-Martin".to_owned(),
                model: "kokoro-martin.onnx".to_owned(),
                voices: "voices-martin.npz".to_owned(),
            },
            dir: "Godelaune--Kokoro-82M-ONNX-German-Martin".to_owned(),
        },
        archive("kokoro-en-v0_19", "en", "Kokoro English", 330, Kind::Kokoro),
        archive(
            "kokoro-multi-lang-v1_1",
            "en,zh",
            "Kokoro English + Chinese",
            380,
            Kind::Kokoro,
        ),
    ]
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackInfo {
    pub id: String,
    pub language: String,
    pub label: String,
    pub megabytes: u32,
    pub installed: bool,
    /// How many speakers the installed pack offers; zero until it is there.
    pub voices: u32,
}

/// Every pack this machine knows: the named ones, plus whatever the user has
/// pulled from the hub and left a manifest for.
fn known() -> Vec<Pack> {
    let mut packs = catalogue();
    let Ok(root) = voices_dir() else {
        return packs;
    };
    let Ok(entries) = fs::read_dir(root) else {
        return packs;
    };

    for entry in entries.flatten() {
        let manifest = entry.path().join(MANIFEST);
        if !manifest.is_file() {
            continue;
        }
        let Ok(text) = fs::read_to_string(&manifest) else {
            continue;
        };
        match serde_json::from_str::<Pack>(&text) {
            Ok(pack) if !packs.iter().any(|known| known.id == pack.id) => packs.push(pack),
            Ok(_) => (),
            Err(error) => log::warn!("voice pack: {} is unreadable: {error}", manifest.display()),
        }
    }

    packs
}

fn find(id: &str) -> Option<Pack> {
    known().into_iter().find(|pack| pack.id == id)
}

/// Where packs are unpacked: this app's own folder, never `~/.claude`.
fn voices_dir() -> Result<PathBuf> {
    let dir = crate::paths::data_dir().join("voices");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn pack_dir(pack: &Pack) -> Result<PathBuf> {
    Ok(voices_dir()?.join(&pack.dir))
}

/// The single `.onnx` a Vits pack brings, whose name carries the voice.
fn onnx_in(dir: &Path) -> Option<PathBuf> {
    fs::read_dir(dir).ok()?.flatten().find_map(|entry| {
        let path = entry.path();
        (path.extension().is_some_and(|ext| ext == "onnx")).then_some(path)
    })
}

/// A pack counts as installed once the files sherpa needs are all there.
fn is_installed(kind: Kind, dir: &Path) -> bool {
    let shared = dir.join("tokens.txt").is_file() && dir.join("espeak-ng-data").is_dir();
    shared
        && match kind {
            Kind::Kokoro => dir.join("model.onnx").is_file() && dir.join("voices.bin").is_file(),
            Kind::Vits => onnx_in(dir).is_some(),
        }
}

/// The number of speakers a pack offers.
///
/// Kokoro says it in the size of its voice table: a flat array of float32
/// embeddings, 510 * 256 per speaker. A Vits pack keeps no such table — most
/// are one voice, but an emotional Piper model is several, and only the loaded
/// model knows. Until it has been loaded once, one is the honest guess.
fn speaker_count(kind: Kind, dir: &Path) -> u32 {
    const PER_SPEAKER: u64 = 510 * 256 * 4;
    match kind {
        Kind::Vits => fs::read_to_string(dir.join(SPEAKERS))
            .ok()
            .and_then(|text| text.trim().parse().ok())
            .unwrap_or(1),
        Kind::Kokoro => fs::metadata(dir.join("voices.bin"))
            .map(|meta| (meta.len() / PER_SPEAKER) as u32)
            .unwrap_or(0),
    }
}

/// Where packs live, so the window can name the folder it is talking about.
pub fn folder() -> Result<String> {
    Ok(voices_dir()?.to_string_lossy().into_owned())
}

pub fn list() -> Result<Vec<PackInfo>> {
    let root = voices_dir()?;
    Ok(known()
        .iter()
        .map(|pack| {
            let dir = root.join(&pack.dir);
            let installed = is_installed(pack.kind, &dir);
            PackInfo {
                id: pack.id.clone(),
                language: pack.language.clone(),
                label: pack.label.clone(),
                megabytes: pack.megabytes,
                installed,
                voices: if installed {
                    speaker_count(pack.kind, &dir)
                } else {
                    0
                },
            }
        })
        .collect())
}

/// The packs whose download the user has given up on.
///
/// A list rather than one flag: several packs can be on their way at once, and
/// a single flag would have stopped all of them together.
static CANCELLED: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// How much has arrived, for the window to show while it waits.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub id: String,
    pub received: u64,
    /// What the server promised, or none when it did not say.
    pub total: Option<u64>,
}

/// Asks one running download to stop at its next chunk.
pub fn cancel(id: &str) {
    log::info!("voice pack '{id}': cancel requested");
    if let Ok(mut cancelled) = CANCELLED.lock() {
        cancelled.push(id.to_owned());
    }
}

fn is_cancelled(id: &str) -> bool {
    CANCELLED
        .lock()
        .map(|cancelled| cancelled.iter().any(|entry| entry == id))
        .unwrap_or(false)
}

/// Forgets a pack's cancellation, so a second attempt is not stopped by the first.
fn clear_cancel(id: &str) {
    if let Ok(mut cancelled) = CANCELLED.lock() {
        cancelled.retain(|entry| entry != id);
    }
}

/// Fetches a pack and unpacks it, reporting how far along it is.
///
/// The archive is read in chunks rather than in one call, which is what makes
/// both the progress and the cancel button possible: each chunk is a moment to
/// say how far it has come and to notice that the user has given up. It is
/// still collected in memory rather than into a temp file — a half-written
/// folder that looks installed would be worse than a few hundred megabytes.
pub fn install<R: Runtime>(app: &AppHandle<R>, id: &str) -> Result<PackInfo> {
    fetch(id, &emitter(app))
}

/// Installs a voice the user found on the hub rather than one this app names.
pub fn install_from_hub<R: Runtime>(app: &AppHandle<R>, repo: &str) -> Result<PackInfo> {
    fetch_pack(&crate::hub::as_pack(repo)?, &emitter(app))
}

fn emitter<R: Runtime>(app: &AppHandle<R>) -> impl Fn(Progress) + use<'_, R> {
    |progress| {
        let _ = app.emit("voice:progress", progress);
    }
}

/// The download itself, reporting through a callback rather than an app handle.
///
/// Split out so the probe below can run it without a window behind it.
fn fetch(id: &str, report: &dyn Fn(Progress)) -> Result<PackInfo> {
    let pack = find(id).ok_or_else(|| Error::BadRequest(format!("unknown voice pack '{id}'")))?;
    fetch_pack(&pack, report)
}

fn fetch_pack(pack: &Pack, report: &dyn Fn(Progress)) -> Result<PackInfo> {
    let id = &pack.id;
    let dir = pack_dir(pack)?;
    if is_installed(pack.kind, &dir) {
        log::info!("voice pack '{id}': already installed at {}", dir.display());
        return Ok(installed_info(pack, &dir));
    }

    clear_cancel(id);

    match &pack.origin {
        Origin::Archive { url } => {
            log::info!("voice pack '{id}': fetching {url}");
            let bytes = download(url, id, 0, None, report)?;
            log::info!(
                "voice pack '{id}': {} bytes fetched, unpacking",
                bytes.len()
            );
            let decoder = bzip2::read::BzDecoder::new(Cursor::new(bytes));
            tar::Archive::new(decoder).unpack(voices_dir()?)?;
        }
        Origin::Hub { repo } => crate::hub::install(pack, repo, report)?,
        Origin::KokoroExport {
            repo,
            model,
            voices,
        } => crate::hub::assemble_kokoro(pack, repo, model, voices, report)?,
    }

    if !is_installed(pack.kind, &dir) {
        log::warn!("voice pack '{id}': what arrived held no usable voice");
        return Err(Error::BadRequest(
            "the download did not contain a usable voice".to_owned(),
        ));
    }

    log::info!("voice pack '{id}': installed at {}", dir.display());
    Ok(installed_info(pack, &dir))
}

/// Reads one URL into memory, saying how far it has come and stopping when the
/// user has given up.
///
/// In memory rather than into the folder as it arrives: a half-written pack
/// that looks installed would be worse than holding a few hundred megabytes.
/// `base` and `total` are what a caller downloading several files in a row
/// passes so the window sees one bar rather than a row of them.
pub(crate) fn download(
    url: &str,
    id: &str,
    base: u64,
    total: Option<u64>,
    report: &dyn Fn(Progress),
) -> Result<Vec<u8>> {
    let mut response = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60 * 30))
        .build()
        .map_err(|error| Error::BadRequest(error.to_string()))?
        .get(url)
        .send()
        .map_err(|error| Error::BadRequest(error.to_string()))?;

    if !response.status().is_success() {
        log::warn!(
            "voice pack '{id}': the download answered {}",
            response.status()
        );
        return Err(Error::BadRequest(format!(
            "the download answered {}",
            response.status()
        )));
    }

    let total = total.or_else(|| response.content_length());
    let mut bytes: Vec<u8> = Vec::with_capacity(response.content_length().unwrap_or(0) as usize);
    let mut chunk = [0_u8; 64 * 1024];
    let mut announced = 0_u64;

    loop {
        if is_cancelled(id) {
            log::info!("voice pack '{id}': cancelled after {} bytes", bytes.len());
            return Err(Error::BadRequest("cancelled".to_owned()));
        }
        let read = response.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..read]);

        // Every megabyte rather than every chunk: a progress bar that repaints
        // a thousand times a second is only a way to spend the window's time.
        let received = bytes.len() as u64;
        if received - announced >= 1024 * 1024 {
            announced = received;
            report(Progress {
                id: id.to_owned(),
                received: base + received,
                total,
            });
        }
    }

    Ok(bytes)
}

fn installed_info(pack: &Pack, dir: &Path) -> PackInfo {
    PackInfo {
        id: pack.id.clone(),
        language: pack.language.clone(),
        label: pack.label.clone(),
        megabytes: pack.megabytes,
        installed: true,
        voices: speaker_count(pack.kind, dir),
    }
}

pub fn remove(id: &str) -> Result<()> {
    let pack = find(id).ok_or_else(|| Error::BadRequest(format!("unknown voice pack '{id}'")))?;
    let dir = pack_dir(&pack)?;
    if dir.is_dir() {
        fs::remove_dir_all(dir)?;
    }
    Ok(())
}

/// The engine, kept alive between sentences.
///
/// Loading the model costs a second or two; a reader who presses play twice
/// should pay that once. The mutex is what makes one engine safe to share.
static ENGINE: Mutex<Option<(String, sherpa_onnx::OfflineTts)>> = Mutex::new(None);

fn engine_for(pack: &Pack) -> Result<()> {
    let mut slot = ENGINE
        .lock()
        .map_err(|_| Error::BadRequest("the speech engine is wedged".to_owned()))?;

    if slot.as_ref().is_some_and(|(id, _)| *id == pack.id) {
        return Ok(());
    }

    let dir = pack_dir(pack)?;
    if !is_installed(pack.kind, &dir) {
        return Err(Error::BadRequest(format!("'{}' is not installed", pack.id)));
    }

    let tokens = Some(dir.join("tokens.txt").to_string_lossy().into_owned());
    let data_dir = Some(dir.join("espeak-ng-data").to_string_lossy().into_owned());

    let mut model = sherpa_onnx::OfflineTtsModelConfig {
        num_threads: 2,
        ..Default::default()
    };

    match pack.kind {
        Kind::Kokoro => {
            model.kokoro = sherpa_onnx::OfflineTtsKokoroModelConfig {
                model: Some(dir.join("model.onnx").to_string_lossy().into_owned()),
                voices: Some(dir.join("voices.bin").to_string_lossy().into_owned()),
                tokens,
                data_dir,
                ..Default::default()
            };
        }
        Kind::Vits => {
            let onnx = onnx_in(&dir)
                .ok_or_else(|| Error::BadRequest(format!("'{}' has no model", pack.id)))?;
            model.vits = sherpa_onnx::OfflineTtsVitsModelConfig {
                model: Some(onnx.to_string_lossy().into_owned()),
                tokens,
                data_dir,
                ..Default::default()
            };
        }
    }

    let config = sherpa_onnx::OfflineTtsConfig {
        model,
        ..Default::default()
    };

    let tts = sherpa_onnx::OfflineTts::create(&config)
        .ok_or_else(|| Error::BadRequest("the voice would not load".to_owned()))?;

    if pack.kind == Kind::Vits {
        let _ = fs::write(dir.join(SPEAKERS), tts.num_speakers().max(1).to_string());
    }

    *slot = Some((pack.id.to_owned(), tts));
    Ok(())
}

/// Speaks one piece of text and returns when it has been said.
pub fn speak(id: &str, speaker: i32, text: &str) -> Result<()> {
    if text.trim().is_empty() {
        return Ok(());
    }

    let pack = find(id).ok_or_else(|| Error::BadRequest(format!("unknown voice pack '{id}'")))?;
    engine_for(&pack)?;

    let mut slot = ENGINE
        .lock()
        .map_err(|_| Error::BadRequest("the speech engine is wedged".to_owned()))?;
    let Some((_, tts)) = slot.as_mut() else {
        return Err(Error::BadRequest("the speech engine vanished".to_owned()));
    };

    let audio = tts
        .generate_with_config(
            text,
            &sherpa_onnx::GenerationConfig {
                speed: 1.0,
                sid: speaker,
                ..Default::default()
            },
            None::<fn(&[f32], f32) -> bool>,
        )
        .ok_or_else(|| Error::BadRequest("the voice could not speak".to_owned()))?;

    play(audio.samples(), audio.sample_rate() as u32)
}

/// Set while the reader should stop; cleared when the next sentence starts.
static SILENCED: AtomicBool = AtomicBool::new(false);

/// Cuts the sentence being read short.
pub fn silence() {
    SILENCED.store(true, Ordering::Relaxed);
}

/// Plays samples through whatever the system calls its speakers.
///
/// The wait is the audio's own length rather than only `sleep_until_end`: that
/// call came back while the sound was still going and took the stream — and the
/// rest of the sentence — down with it. Waiting in short steps is also what
/// lets the stop button end a sentence half way through.
fn play(samples: &[f32], sample_rate: u32) -> Result<()> {
    SILENCED.store(false, Ordering::Relaxed);

    let stream = rodio::OutputStreamBuilder::open_default_stream()
        .map_err(|error| Error::BadRequest(format!("no audio output: {error}")))?;
    let sink = rodio::Sink::connect_new(stream.mixer());

    sink.append(rodio::buffer::SamplesBuffer::new(
        1,
        sample_rate,
        samples.to_vec(),
    ));

    let length =
        std::time::Duration::from_secs_f64(samples.len() as f64 / sample_rate.max(1) as f64);
    // What the device has yet to emit once the mixer has read everything. It
    // cannot be asked for, and closing the stream at that moment took the last
    // seconds of the sentence with it, so it is waited out.
    const TAIL: std::time::Duration = std::time::Duration::from_millis(2500);
    let start = std::time::Instant::now();

    loop {
        if SILENCED.load(Ordering::Relaxed) {
            sink.stop();
            return Ok(());
        }
        // Two clocks, because neither alone is the end: the sink's own position
        // runs ahead of the speaker, and the wall clock knows nothing about a
        // device that started late.
        if sink.empty() && sink.get_pos() >= length && start.elapsed() >= length + TAIL {
            break;
        }
        // Past twice the sentence plus the wait, something is wedged and
        // holding this thread helps nobody.
        if start.elapsed() > length * 2 + TAIL {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pack_is_reachable_by_id() {
        for pack in catalogue() {
            assert!(find(&pack.id).is_some());
        }
        assert!(find("nope").is_none());
    }

    #[test]
    fn a_vits_pack_is_one_speaker_and_a_kokoro_pack_is_counted() {
        let dir = Path::new("C:/nowhere/at/all");
        assert_eq!(speaker_count(Kind::Vits, dir), 1);
        assert_eq!(speaker_count(Kind::Kokoro, dir), 0);
    }

    #[test]
    fn a_folder_without_the_files_is_not_installed() {
        assert!(!is_installed(Kind::Kokoro, Path::new("C:/nowhere/at/all")));
        assert!(!is_installed(Kind::Vits, Path::new("C:/nowhere/at/all")));
    }
}

#[cfg(test)]
mod probe {
    use super::*;

    /// Speaks a sentence with whichever pack is named, and says what came out.
    ///
    /// The point is the samples, not the sound: a model whose vocabulary does
    /// not match its tokens still returns a buffer, but a silent or absurdly
    /// short one. Length and peak are what can be checked without ears.
    #[test]
    #[ignore = "needs an installed pack"]
    fn a_pack_says_something() {
        let id = std::env::var("PACK").expect("set PACK to a pack id");
        let text = std::env::var("SAY")
            .unwrap_or_else(|_| "Guten Tag, so werden die Antworten klingen.".to_owned());

        let pack = find(&id).expect("the pack must be known");
        engine_for(&pack).expect("the voice must load");
        let mut slot = ENGINE.lock().unwrap();
        let (_, tts) = slot.as_mut().expect("the engine must be there");

        let audio = tts
            .generate_with_config(
                &text,
                &sherpa_onnx::GenerationConfig {
                    speed: 1.0,
                    sid: 0,
                    ..Default::default()
                },
                None::<fn(&[f32], f32) -> bool>,
            )
            .expect("the voice must speak");

        let samples = audio.samples();
        let seconds = samples.len() as f32 / audio.sample_rate() as f32;
        let peak = samples.iter().fold(0.0_f32, |a, s| a.max(s.abs()));
        println!(
            "pack={id} rate={} samples={} seconds={seconds:.2} peak={peak:.3}",
            audio.sample_rate(),
            samples.len()
        );

        if let Ok(path) = std::env::var("WAV") {
            assert!(audio.save(&path), "the sample must be writable");
            println!("wrote {path}");
        }

        assert!(
            seconds > 1.0,
            "a sentence that short cannot be the sentence"
        );
        assert!(peak > 0.05, "the buffer is silence");
    }

    #[test]
    #[ignore = "downloads a few hundred megabytes"]
    fn a_pack_installs() {
        let id = std::env::var("PACK").unwrap_or_else(|_| "kokoro-en-v0_19".to_owned());
        let info = fetch(&id, &|progress| {
            if let Some(total) = progress.total {
                eprintln!("  {} / {} bytes", progress.received, total);
            }
        })
        .unwrap();
        assert!(info.installed);
        assert!(info.voices > 0, "a pack with no speaker cannot be chosen");
    }

    #[test]
    fn the_voices_folder_can_be_created() {
        let dir = voices_dir().expect("the voices folder must be creatable");
        assert!(dir.is_dir(), "{} is not a folder", dir.display());
    }
}

#[cfg(test)]
mod tls {
    /// The download needs a TLS backend; without one every https call fails at
    /// runtime while the build stays green, which is how it slipped through.
    #[test]
    #[ignore = "reaches the network"]
    fn https_reaches_the_release_host() {
        let response = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap()
            .head("https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/kokoro-en-v0_19.tar.bz2")
            .send()
            .expect("https must work");
        assert!(response.status().is_success());
    }
}
