//! Finding voices on Hugging Face, and fetching the ones sherpa can speak.
//!
//! Only two endpoints are used, both public and both read-only: the model
//! search and a repository's file tree. No token is sent, so nothing private is
//! ever visible here.
//!
//! Most Kokoro repositories on the hub carry a bare `.onnx` and a `.npz` of
//! voice embeddings — the layout the Python runtime wants. sherpa needs
//! `tokens.txt` and an `espeak-ng-data` folder besides, and its voice table as
//! a flat `.bin`. A repository without those cannot be spoken here, so it is
//! listed with the reason rather than offered as an install that would fail
//! after a few hundred megabytes.

use crate::error::{Error, Result};
use crate::tts::{Kind, Origin, Pack, Progress};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const SEARCH: &str = "https://huggingface.co/api/models";
const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// One search hit, already judged on whether it can be installed.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HubModel {
    pub repo: String,
    pub likes: u32,
    pub downloads: u64,
    /// The language tags the author gave it, which is all the hub knows.
    pub languages: Vec<String>,
    pub kind: Option<Kind>,
    pub megabytes: u32,
    /// Empty when it can be installed; otherwise why it cannot.
    pub blocked: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchHit {
    id: String,
    #[serde(default)]
    likes: u32,
    #[serde(default)]
    downloads: u64,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct TreeEntry {
    #[serde(rename = "type")]
    kind: String,
    path: String,
    #[serde(default)]
    size: u64,
}

fn client() -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .timeout(TIMEOUT)
        // The hub answers anonymous callers, but asks to be told who is calling.
        .user_agent("claude-insights")
        .build()
        .map_err(|error| Error::BadRequest(error.to_string()))
}

fn get_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T> {
    let response = client()?
        .get(url)
        .send()
        .map_err(|error| Error::BadRequest(error.to_string()))?;

    if !response.status().is_success() {
        return Err(Error::BadRequest(format!(
            "the hub answered {}",
            response.status()
        )));
    }

    response
        .json::<T>()
        .map_err(|error| Error::BadRequest(error.to_string()))
}

/// The files of one repository, flattened, with their sizes.
fn tree(repo: &str) -> Result<Vec<TreeEntry>> {
    let url = format!("https://huggingface.co/api/models/{repo}/tree/main?recursive=true");
    let entries: Vec<TreeEntry> = get_json(&url)?;
    Ok(entries
        .into_iter()
        .filter(|entry| entry.kind == "file")
        .collect())
}

/// Two-letter language tags the author attached, e.g. `de`, `zh`.
fn languages(tags: &[String]) -> Vec<String> {
    tags.iter()
        .filter(|tag| tag.len() == 2 && tag.chars().all(|c| c.is_ascii_lowercase()))
        .cloned()
        .collect()
}

/// What sherpa needs, judged from the file list alone.
fn judge(files: &[TreeEntry]) -> (Option<Kind>, Option<String>) {
    let has = |name: &str| files.iter().any(|entry| entry.path == name);
    let under = |dir: &str| {
        files
            .iter()
            .any(|entry| entry.path.starts_with(&format!("{dir}/")))
    };
    let any_onnx = files.iter().any(|entry| entry.path.ends_with(".onnx"));

    if !any_onnx {
        return (None, Some("no .onnx model".to_owned()));
    }

    let mut missing = Vec::new();
    if !has("tokens.txt") {
        missing.push("tokens.txt");
    }
    if !under("espeak-ng-data") {
        missing.push("espeak-ng-data");
    }

    // The voice table is what tells the two apart: a Piper voice has none.
    let kind = if has("voices.bin") {
        Kind::Kokoro
    } else {
        Kind::Vits
    };

    if kind == Kind::Kokoro && !has("model.onnx") {
        missing.push("model.onnx");
    }

    if missing.is_empty() {
        (Some(kind), None)
    } else {
        (Some(kind), Some(format!("missing {}", missing.join(", "))))
    }
}

/// Searches the hub for speakable voices.
///
/// Deliberately without the hub's `text-to-speech` and `onnx` tag filters: the
/// mirrors that carry the sherpa layout mostly leave those tags off, so
/// filtering on them hid exactly the repositories that can be installed. The
/// file list decides instead, which is the thing that actually settles it.
pub fn search(query: &str) -> Result<Vec<HubModel>> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }

    let url = format!("{SEARCH}?search={}&limit=12", urlencoding(query));
    log::info!("hub: searching for '{query}'");
    let hits: Vec<SearchHit> = get_json(&url)?;

    let mut models = Vec::with_capacity(hits.len());
    for hit in hits {
        // One tree call per hit, which is what tells a usable repository from
        // one that only looks the part. Twelve of them is a second at most.
        let files = tree(&hit.id).unwrap_or_default();
        let (kind, blocked) = judge(&files);
        let bytes: u64 = files.iter().map(|entry| entry.size).sum();
        models.push(HubModel {
            repo: hit.id,
            likes: hit.likes,
            downloads: hit.downloads,
            languages: languages(&hit.tags),
            kind,
            megabytes: (bytes / (1024 * 1024)) as u32,
            blocked,
        });
    }

    // What can be installed goes first: a page of repositories that cannot be
    // spoken is a page nobody reads to the end of.
    models.sort_by_key(|model| (model.blocked.is_some(), std::cmp::Reverse(model.likes)));

    log::info!("hub: {} results for '{query}'", models.len());
    Ok(models)
}

/// Percent-encodes a search term. Only the few characters a model name can
/// carry need it, so a table beats a dependency.
fn urlencoding(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "+".to_owned(),
            other => other
                .to_string()
                .bytes()
                .map(|byte| format!("%{byte:02X}"))
                .collect(),
        })
        .collect()
}

/// The pack a repository would become, ready to be handed to the installer.
pub fn as_pack(repo: &str) -> Result<Pack> {
    let files = tree(repo)?;
    let (kind, blocked) = judge(&files);
    let (Some(kind), None) = (kind, blocked) else {
        return Err(Error::BadRequest(format!(
            "'{repo}' is not a voice this app can speak"
        )));
    };

    let hit: SearchHit = get_json(&format!("https://huggingface.co/api/models/{repo}"))?;
    let bytes: u64 = files.iter().map(|entry| entry.size).sum();
    Ok(Pack {
        id: format!("hub:{repo}"),
        language: languages(&hit.tags).join(","),
        label: repo.to_owned(),
        megabytes: (bytes / (1024 * 1024)) as u32,
        kind,
        origin: Origin::Hub {
            repo: repo.to_owned(),
        },
        dir: folder_for(repo),
    })
}

/// A repository name as one folder: the slash would otherwise make two.
pub fn folder_for(repo: &str) -> String {
    repo.replace('/', "--")
}

/// Downloads every file of a repository into the pack's folder.
///
/// File by file rather than as one archive, because the hub has no archive to
/// offer: the sizes from the tree are added up first so the window can still
/// show one bar for the whole thing.
pub fn install(pack: &Pack, repo: &str, report: &dyn Fn(Progress)) -> Result<()> {
    let files = tree(repo)?;
    let total: u64 = files.iter().map(|entry| entry.size).sum();
    let dir = crate::paths::data_dir().join("voices").join(&pack.dir);
    fs::create_dir_all(&dir)?;

    log::info!(
        "voice pack '{}': fetching {} files from {repo}",
        pack.id,
        files.len()
    );

    let mut done = 0_u64;
    for entry in &files {
        let url = format!("https://huggingface.co/{repo}/resolve/main/{}", entry.path);
        let bytes = crate::tts::download(&url, &pack.id, done, Some(total), report)?;
        write_under(&dir, &entry.path, &bytes)?;
        done += bytes.len() as u64;
    }

    let manifest =
        serde_json::to_string_pretty(pack).map_err(|error| Error::BadRequest(error.to_string()))?;
    fs::write(dir.join(crate::tts::MANIFEST), manifest)?;
    Ok(())
}

/// The sherpa mirror the token table and the phonemiser's data come from.
///
/// A community Kokoro export carries neither, and both are the same for every
/// Kokoro voice: the token table is the model family's own, and espeak-ng's
/// data covers every language at once.
const KOKORO_MIRROR: &str = "csukuangfj/kokoro-en-v0_19";

/// Assembles a Kokoro export into something sherpa can speak.
///
/// Four things have to be true and only one of them is in the repository: the
/// model, a flat table of voices, the token table, and the metadata sherpa
/// reads out of the model. The export has the first as a file and the second as
/// a NumPy archive; the third comes from the mirror, and the fourth is written
/// here.
pub fn assemble_kokoro(
    pack: &Pack,
    repo: &str,
    model: &str,
    voices: &str,
    report: &dyn Fn(Progress),
) -> Result<()> {
    let dir = crate::paths::data_dir().join("voices").join(&pack.dir);
    fs::create_dir_all(&dir)?;

    let files = tree(repo)?;
    let size_of = |name: &str| {
        files
            .iter()
            .find(|entry| entry.path == name)
            .map(|entry| entry.size)
            .unwrap_or(0)
    };
    let espeak = tree(KOKORO_MIRROR)?
        .into_iter()
        .filter(|entry| entry.path.starts_with("espeak-ng-data/"))
        .collect::<Vec<_>>();
    let total = size_of(model) + size_of(voices) + espeak.iter().map(|e| e.size).sum::<u64>();

    log::info!("voice pack '{}': assembling from {repo}", pack.id);

    let raw = crate::tts::download(&file_url(repo, model), &pack.id, 0, Some(total), report)?;
    let mut done = raw.len() as u64;

    let table = crate::tts::download(&file_url(repo, voices), &pack.id, done, Some(total), report)?;
    done += table.len() as u64;
    let (flat, speakers, style_dim) = flatten_voices(&table)?;
    fs::write(dir.join("voices.bin"), flat)?;

    // The token table is tiny and never in the export; the mirror's is the one
    // the whole Kokoro family was trained against.
    let tokens = crate::tts::download(
        &file_url(KOKORO_MIRROR, "tokens.txt"),
        &pack.id,
        done,
        Some(total),
        report,
    )?;
    fs::write(dir.join("tokens.txt"), tokens)?;

    for entry in &espeak {
        let bytes = crate::tts::download(
            &file_url(KOKORO_MIRROR, &entry.path),
            &pack.id,
            done,
            Some(total),
            report,
        )?;
        write_under(&dir, &entry.path, &bytes)?;
        done += bytes.len() as u64;
    }

    let described = with_metadata(raw, pack, speakers, style_dim);
    fs::write(dir.join("model.onnx"), described)?;

    let manifest =
        serde_json::to_string_pretty(pack).map_err(|error| Error::BadRequest(error.to_string()))?;
    fs::write(dir.join(crate::tts::MANIFEST), manifest)?;
    Ok(())
}

fn file_url(repo: &str, path: &str) -> String {
    format!("https://huggingface.co/{repo}/resolve/main/{path}")
}

/// Reads the one array out of a `.npz` and returns it as sherpa's flat table.
///
/// The archive is a zip and NumPy writes its arrays uncompressed, so the entry
/// is taken from the local header rather than by pulling in a zip library for
/// one file. Returned with the speaker count and the shape, both of which the
/// model's metadata has to state and only this file knows.
fn flatten_voices(npz: &[u8]) -> Result<(Vec<u8>, u32, String)> {
    let odd = |what: &str| Error::BadRequest(format!("the voice table is {what}"));

    if npz.get(..4) != Some(b"PK\x03\x04") {
        return Err(odd("not a NumPy archive"));
    }
    let word = |at: usize| -> Result<usize> {
        let bytes = npz.get(at..at + 2).ok_or_else(|| odd("truncated"))?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]) as usize)
    };
    if word(8)? != 0 {
        return Err(odd("compressed, which NumPy does not do by default"));
    }
    let stored = npz
        .get(18..22)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
        .ok_or_else(|| odd("truncated"))?;
    let name_len = word(26)?;
    let extra_len = word(28)?;
    let at = 30 + name_len + extra_len;
    // To the entry's own length, not to the end of the file: the zip's central
    // directory sits behind it and would ride along as seventy-eight stray bytes.
    let npy = npz.get(at..at + stored).ok_or_else(|| odd("truncated"))?;

    if npy.get(..6) != Some(b"\x93NUMPY") {
        return Err(odd("not a NumPy array"));
    }
    let header_len = u16::from_le_bytes([npy[8], npy[9]]) as usize;
    let header = String::from_utf8_lossy(&npy[10..10 + header_len]).to_string();
    let body = npy
        .get(10 + header_len..)
        .ok_or_else(|| odd("truncated"))?
        .to_vec();

    if !header.contains("'<f4'") {
        return Err(odd("not a table of 32-bit floats"));
    }

    // A Kokoro style table is one row per token length by one row per speaker.
    const PER_SPEAKER: usize = 510 * 256 * 4;
    let speakers = (body.len() / PER_SPEAKER).max(1) as u32;
    let rows = body.len() / (256 * 4 * speakers as usize);
    Ok((body, speakers, format!("{rows},1,256")))
}

/// Appends the metadata sherpa's Kokoro loader insists on.
///
/// Protobuf takes repeated fields wherever they appear, so the entries are
/// written onto the end of the model rather than the file being rebuilt. The
/// exports leave these out because the Python runtime never reads them.
fn with_metadata(mut model: Vec<u8>, pack: &Pack, speakers: u32, style_dim: String) -> Vec<u8> {
    let entry = |key: &str, value: &str| {
        let mut payload = Vec::new();
        payload.push(0x0a);
        payload.push(key.len() as u8);
        payload.extend_from_slice(key.as_bytes());
        payload.push(0x12);
        payload.push(value.len() as u8);
        payload.extend_from_slice(value.as_bytes());
        let mut field = vec![0x72, payload.len() as u8];
        field.extend_from_slice(&payload);
        field
    };

    // `voice` is the espeak voice the phonemiser is asked for, so it is the
    // language tag rather than the pack's name.
    let voice = pack.language.split(',').next().unwrap_or("en").to_owned();
    for (key, value) in [
        ("model_type", "kokoro".to_owned()),
        ("language", pack.label.clone()),
        ("has_espeak", "1".to_owned()),
        ("sample_rate", "24000".to_owned()),
        ("version", "1".to_owned()),
        ("voice", voice),
        ("style_dim", style_dim),
        ("n_speakers", speakers.to_string()),
    ] {
        model.extend_from_slice(&entry(key, &value));
    }
    model
}

/// Writes one file of the repository under the pack's folder.
///
/// The path comes from the hub, so it is taken apart and rebuilt from its plain
/// components: a `..` in it would otherwise write outside the folder.
fn write_under(dir: &Path, path: &str, bytes: &[u8]) -> Result<()> {
    let mut target = dir.to_path_buf();
    for part in path.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            return Err(Error::BadRequest(format!(
                "the hub sent an odd path: {path}"
            )));
        }
        target.push(part);
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(target, bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str) -> TreeEntry {
        TreeEntry {
            kind: "file".to_owned(),
            path: path.to_owned(),
            size: 1,
        }
    }

    #[test]
    fn a_sherpa_kokoro_layout_is_speakable() {
        let files = vec![
            file("model.onnx"),
            file("voices.bin"),
            file("tokens.txt"),
            file("espeak-ng-data/phontab"),
        ];
        assert_eq!(judge(&files), (Some(Kind::Kokoro), None));
    }

    #[test]
    fn the_python_kokoro_layout_says_what_it_lacks() {
        // The repository the user asked about: a model and .npz voices, nothing
        // sherpa can phonemise with.
        let files = vec![file("kokoro-martin.onnx"), file("voices-martin.npz")];
        let (kind, blocked) = judge(&files);
        assert_eq!(kind, Some(Kind::Vits));
        assert_eq!(
            blocked.as_deref(),
            Some("missing tokens.txt, espeak-ng-data")
        );
    }

    #[test]
    fn a_repository_without_a_model_is_refused() {
        assert_eq!(
            judge(&[file("README.md")]),
            (None, Some("no .onnx model".to_owned()))
        );
    }

    #[test]
    fn a_numpy_archive_becomes_a_flat_table() {
        // One stored entry holding a (2, 1, 4) float32 array: the same shape a
        // Kokoro style table has, only small enough to write out here.
        let rows = 2usize;
        let body: Vec<u8> = (0..rows * 4)
            .flat_map(|i| (i as f32).to_le_bytes())
            .collect();
        let header =
            format!("{{'descr': '<f4', 'fortran_order': False, 'shape': ({rows}, 1, 4), }}");
        let mut npy = b"\x93NUMPY\x01\x00".to_vec();
        npy.extend_from_slice(&(header.len() as u16).to_le_bytes());
        npy.extend_from_slice(header.as_bytes());
        npy.extend_from_slice(&body);

        let mut npz = b"PK\x03\x04".to_vec();
        npz.resize(18, 0);
        npz.extend_from_slice(&(npy.len() as u32).to_le_bytes()); // stored length
        npz.resize(26, 0);
        npz.extend_from_slice(&4u16.to_le_bytes()); // name length
        npz.extend_from_slice(&0u16.to_le_bytes()); // extra length
        npz.extend_from_slice(b"a.np");
        npz.extend_from_slice(&npy);
        // What a real archive keeps behind the entry, and what must not be read.
        npz.extend_from_slice(b"PK\x01\x02trailing rubbish");

        let (flat, speakers, _) = flatten_voices(&npz).unwrap();
        assert_eq!(
            flat, body,
            "the payload is the table, and nothing behind it"
        );
        assert_eq!(speakers, 1);
    }

    #[test]
    fn a_compressed_archive_is_refused_by_name() {
        let mut npz = b"PK\x03\x04".to_vec();
        npz.resize(40, 0);
        npz[8] = 8; // deflate
        let error = flatten_voices(&npz).unwrap_err().to_string();
        assert!(error.contains("compressed"), "{error}");
    }

    #[test]
    fn the_metadata_carries_the_pack_s_own_language() {
        let pack = Pack {
            id: "hub:x/y".to_owned(),
            language: "de".to_owned(),
            label: "Kokoro German".to_owned(),
            megabytes: 1,
            kind: Kind::Kokoro,
            origin: Origin::Hub {
                repo: "x/y".to_owned(),
            },
            dir: "x--y".to_owned(),
        };
        let out = with_metadata(vec![1, 2, 3], &pack, 1, "510,1,256".to_owned());
        let tail = String::from_utf8_lossy(&out[3..]).to_string();
        assert!(tail.contains("voice"), "{tail}");
        assert!(tail.contains("de"), "{tail}");
        assert!(tail.contains("510,1,256"), "{tail}");
        assert_eq!(&out[..3], &[1, 2, 3], "the model itself is left alone");
    }

    #[test]
    fn a_repository_name_becomes_one_folder() {
        assert_eq!(folder_for("Godelaune/Kokoro-82M"), "Godelaune--Kokoro-82M");
    }

    #[test]
    fn a_climbing_path_is_refused() {
        let dir = std::env::temp_dir().join("claude-admin-hub-test");
        assert!(write_under(&dir, "../escaped.txt", b"no").is_err());
    }

    #[test]
    #[ignore = "reaches the network"]
    fn the_hub_still_answers_in_the_shape_this_expects() {
        let found = search("vits-piper-de_DE-thorsten").expect("the hub must answer");
        assert!(
            found.iter().any(|model| model.blocked.is_none()),
            "a sherpa mirror must come back installable: {found:?}"
        );
    }

    #[test]
    fn a_search_term_survives_the_url() {
        assert_eq!(urlencoding("kokoro german"), "kokoro+german");
        assert_eq!(urlencoding("a/b"), "a%2Fb");
    }
}
