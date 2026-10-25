//! File pick, save and open through the Android Storage Access Framework.
//!
//! Rust and Kotlin exchange only strings and small files, so no JNI call goes from Rust to Kotlin.
//! Rust queues a [`Request`]; `MainActivity.kt` polls [`take_request`], shows the system picker or
//! copies the bytes between a cache file and a document, then reports through [`save_done`],
//! [`opened`] or [`notice`]. No Android or egui dependency, so it runs in host tests.
//!
//! A document the user picked has a *location*: its `content://` URI, a slash, then the provider's
//! display name, as in `content://…/document/…/quokka.docx`. The name is the last path segment, so
//! the window title, Recent's name column and the engine's extension rules work unchanged. The URI
//! is everything before the last slash. Recent keeps locations, so an entry reopens after a restart
//! while the persisted grant lasts.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

pub enum Request {
    /// Copy the cache file at `path` to a document the user picks, named `name`.
    Save { ticket: u64, name: String, path: String },
    /// Copy the cache file at `path` over the document at `uri` (plain Save of a picked document).
    Write { ticket: u64, name: String, uri: String, path: String },
    /// Copy the document at `uri`, named `name`, to a cache file and open it (Recent).
    Read { uri: String, name: String },
    /// Let the user pick a document for `purpose` (`document`, `picture`, ...).
    Open { purpose: String },
    /// Show the system print dialog for the PDF in the cache file at `path`.
    Print { path: String },
}

#[derive(Debug, PartialEq, Eq)]
pub enum Done {
    /// Saved. A document (not an export) carries the location it now lives at.
    Saved(Option<String>),
    Cancelled,
    Failed(String),
}

/// A save in flight: its ticket, the name it saves under, the cache file it copies and whether it
/// writes over a document (so a failure can fall back to the picker).
struct Flight {
    ticket: u64,
    name: String,
    path: String,
    write: bool,
}

/// Where Rust puts files for Kotlin to copy out (set from `android_main`).
pub static DIR: OnceLock<PathBuf> = OnceLock::new();

static REQUESTS: Mutex<VecDeque<Request>> = Mutex::new(VecDeque::new());
static SAVED: Mutex<Vec<(u64, Done)>> = Mutex::new(Vec::new());
/// Picked documents: (location, cache file).
static OPENED: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());
/// Messages for the status line, from Kotlin or from a save that fell back to the picker.
static NOTICES: Mutex<Vec<String>> = Mutex::new(Vec::new());
static FLIGHTS: Mutex<Vec<Flight>> = Mutex::new(Vec::new());
/// The location of the open document, for a plain Save. The app sets it each frame.
static CURRENT: Mutex<Option<String>> = Mutex::new(None);

const SCHEME: &str = "content://";

pub fn request(r: Request) {
    match &r {
        Request::Save { ticket, name, path } => flight(*ticket, name, path, false),
        Request::Write { ticket, name, path, .. } => flight(*ticket, name, path, true),
        _ => {}
    }
    if let Ok(mut q) = REQUESTS.lock() {
        q.push_back(r);
    }
}

fn flight(ticket: u64, name: &str, path: &str, write: bool) {
    if let Ok(mut f) = FLIGHTS.lock() {
        f.push(Flight { ticket, name: name.into(), path: path.into(), write });
    }
}

/// The location of the document at `uri` whose provider name is `name`. The name keeps no slash
/// or control character, so the last slash always starts the name.
pub fn location(uri: &str, name: &str) -> String {
    let name: String = name.chars().map(|c| if c == '/' || c.is_control() { '_' } else { c }).collect();
    let name = if name.trim().is_empty() || name == "." || name == ".." { "document".to_string() } else { name };
    format!("{uri}/{name}")
}

/// The (URI, name) of `loc` when it is a document location, else `None` (a file path).
pub fn parse_location(loc: &str) -> Option<(&str, &str)> {
    let (uri, name) = loc.rsplit_once('/')?;
    (uri.len() > SCHEME.len() && uri.starts_with(SCHEME) && !name.is_empty()).then_some((uri, name))
}

/// Recent (or any host location) asks for the document at `loc`: queue a read and return `true`,
/// or `false` when `loc` is not a document location and the file system should open it.
pub fn read_location(loc: &str) -> bool {
    let Some((uri, name)) = parse_location(loc) else { return false };
    request(Request::Read { uri: uri.into(), name: name.into() });
    true
}

const RECENT_FILE: &str = "recent.txt";

/// The Recent list as one location per line. Only document locations are kept.
pub fn recent_text(list: &[String]) -> String {
    list.iter().filter(|l| parse_location(l).is_some()).map(String::as_str).collect::<Vec<_>>().join("\n")
}

pub fn parse_recent(text: &str) -> Vec<String> {
    text.lines().filter(|l| parse_location(l).is_some()).map(String::from).collect()
}

/// The Recent list kept in the data directory; empty when there is none yet.
pub fn load_recent() -> Vec<String> {
    DIR.get().and_then(|d| std::fs::read_to_string(d.join(RECENT_FILE)).ok()).map(|t| parse_recent(&t)).unwrap_or_default()
}

/// Keeps the Recent list for the next launch. A failed write only loses that list.
pub fn save_recent(list: &[String]) {
    if let Some(d) = DIR.get() {
        let _ = std::fs::write(d.join(RECENT_FILE), recent_text(list));
    }
}

/// Where the recovery copies of unsaved work live: the app's data directory.
pub fn recovery_dir() -> Option<PathBuf> {
    DIR.get().map(|d| d.join("recovery"))
}

/// Document types WordCraft opens from another app (Open with, Share): by extension, and by MIME type
/// for a provider that sends no usable name. Mirrors the intent filters in AndroidManifest.xml.
const DOCUMENT_EXTENSIONS: &[&str] = &["docx", "doc", "rtf", "odt", "txt", "text", "md", "markdown"];
const DOCUMENT_MIMES: &[&str] = &[
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    "application/msword",
    "application/rtf",
    "text/rtf",
    "application/vnd.oasis.opendocument.text",
    "text/plain",
    "text/markdown",
];

/// Whether a file named `name` with MIME type `mime` (empty when unknown) is a document WordCraft
/// opens. The extension decides first; `application/octet-stream` counts for nothing else.
pub fn accepts_document(name: &str, mime: &str) -> bool {
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    DOCUMENT_EXTENSIONS.contains(&ext.as_str()) || DOCUMENT_MIMES.contains(&mime.trim().to_ascii_lowercase().as_str())
}

/// Exports (PDF, PNG) are copies, not the document: they always ask where to save.
pub fn is_export(name: &str) -> bool {
    name.ends_with(".pdf") || name.ends_with(".png")
}

/// The location of the open document, or `None` for a new one. Called by the app each frame.
pub fn set_current(loc: Option<String>) {
    if let Ok(mut c) = CURRENT.lock() {
        *c = loc;
    }
}

/// The URI a plain Save writes to: the open document's, when it was picked.
pub fn save_target() -> Option<String> {
    let loc = CURRENT.lock().ok().and_then(|c| c.clone())?;
    parse_location(&loc).map(|(uri, _)| uri.to_string())
}

/// The next request as one line for Kotlin: `save\tticket\tname\tpath`,
/// `write\tticket\turi\tpath`, `read\turi\tname`, `open\tpurpose` or `print\tpath`; empty when
/// there is none.
pub fn take_request() -> String {
    let r = REQUESTS.lock().ok().and_then(|mut q| q.pop_front());
    match r {
        Some(Request::Save { ticket, name, path }) => format!("save\t{ticket}\t{}\t{path}", name.replace(['\t', '\n'], " ")),
        Some(Request::Write { ticket, uri, path, .. }) => format!("write\t{ticket}\t{uri}\t{path}"),
        Some(Request::Read { uri, name }) => format!("read\t{uri}\t{}", name.replace(['\t', '\n'], " ")),
        Some(Request::Open { purpose }) => format!("open\t{purpose}"),
        Some(Request::Print { path }) => format!("print\t{path}"),
        None => String::new(),
    }
}

/// Kotlin: save `ticket` ended. `status` 1 saved (`msg` is the document URI and `display` its
/// provider name), 0 cancelled, anything else failed with `msg`.
pub fn save_done(ticket: u64, status: i32, msg: String, display: String) {
    let flight = FLIGHTS.lock().ok().and_then(|mut f| f.iter().position(|x| x.ticket == ticket).map(|i| f.swap_remove(i)));
    // A plain Save that could not write the document asks where to save instead, and says why.
    if let Some(f) = flight.as_ref().filter(|f| f.write && !matches!(status, 0 | 1)) {
        notice(format!("Couldn't save {}: {msg}. Choose where to save.", f.name));
        request(Request::Save { ticket, name: f.name.clone(), path: f.path.clone() });
        return;
    }
    let d = match status {
        1 => Done::Saved(flight.as_ref().filter(|f| !is_export(&f.name)).filter(|_| !msg.is_empty()).map(|_| location(&msg, &display))),
        0 => Done::Cancelled,
        _ => Done::Failed(if msg.is_empty() { "unknown error".into() } else { msg }),
    };
    if let Ok(mut v) = SAVED.lock() {
        v.push((ticket, d));
    }
    crate::ime::wake();
}

/// Kotlin: the user picked the document at `uri` (or Recent asked for it), named `name`; its bytes
/// are in the cache file at `path`.
pub fn opened(name: String, path: String, uri: String) {
    if let Ok(mut v) = OPENED.lock() {
        v.push((location(&uri, &name), path));
    }
    crate::ime::wake();
}

/// Kotlin or Rust: say `msg` on the status line.
pub fn notice(msg: String) {
    if let Ok(mut n) = NOTICES.lock() {
        n.push(msg);
    }
    crate::ime::wake();
}

pub fn drain_notices() -> Vec<String> {
    NOTICES.lock().map(|mut n| n.drain(..).collect()).unwrap_or_default()
}

pub fn drain_saved() -> Vec<(u64, Done)> {
    SAVED.lock().map(|mut v| v.drain(..).collect()).unwrap_or_default()
}

/// Picked documents as (location, bytes); the cache files are removed.
pub fn drain_opened() -> Vec<(String, Vec<u8>)> {
    let picked: Vec<_> = OPENED.lock().map(|mut v| v.drain(..).collect()).unwrap_or_default();
    picked
        .into_iter()
        .map(|(loc, path)| {
            let bytes = std::fs::read(&path).unwrap_or_default();
            let _ = std::fs::remove_file(&path);
            (loc, bytes)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locations_keep_the_name_last_and_round_trip() {
        let uri = "content://auth/doc/primary%3ADownload%2Fquokka.docx";
        let loc = location(uri, "quokka (1).docx");
        assert_eq!(loc, format!("{uri}/quokka (1).docx"));
        assert_eq!(parse_location(&loc), Some((uri, "quokka (1).docx")));
        assert_eq!(location("content://a/b", "x/y\tz.docx"), "content://a/b/x_y_z.docx");
        assert_eq!(location("content://a/b", ".."), "content://a/b/document");
        assert_eq!(parse_location("/data/user/0/app/files/quokka.docx"), None, "a file path is not a location");
        assert_eq!(parse_location("content://a"), None, "no name");
        assert_eq!(parse_location("content://a/"), None, "an empty name");
    }

    #[test]
    fn requests_round_trip_to_one_line_each() {
        request(Request::Save { ticket: 7, name: "a\tb.docx".into(), path: "/c/s7.bin".into() });
        request(Request::Open { purpose: "document".into() });
        request(Request::Write { ticket: 8, name: "u.docx".into(), uri: "content://d/u".into(), path: "/c/s8.bin".into() });
        assert!(read_location("content://d/u/quokka.docx"));
        assert!(!read_location("/data/quokka.docx"), "a file path is left to the file system");
        request(Request::Print { path: "/c/p.pdf".into() });
        assert_eq!(take_request(), "save\t7\ta b.docx\t/c/s7.bin");
        assert_eq!(take_request(), "open\tdocument");
        assert_eq!(take_request(), "write\t8\tcontent://d/u\t/c/s8.bin");
        assert_eq!(take_request(), "read\tcontent://d/u\tquokka.docx");
        assert_eq!(take_request(), "print\t/c/p.pdf");
        assert_eq!(take_request(), "");

        // A write that fails asks the picker over the same ticket, and says why.
        request(Request::Write { ticket: 30, name: "u.docx".into(), uri: "content://d/u".into(), path: "/c/s30.bin".into() });
        assert_eq!(take_request(), "write\t30\tcontent://d/u\t/c/s30.bin");
        save_done(30, -1, "Permission denied".into(), String::new());
        assert_eq!(take_request(), "save\t30\tu.docx\t/c/s30.bin");
        assert_eq!(drain_notices(), vec!["Couldn't save u.docx: Permission denied. Choose where to save.".to_string()]);
        assert!(drain_saved().iter().all(|(t, _)| *t != 30), "the save is still waiting for the picker");
    }

    #[test]
    fn save_status_maps_to_outcome() {
        save_done(1, 1, String::new(), String::new());
        save_done(2, 0, String::new(), String::new());
        save_done(3, -1, String::new(), String::new());
        let mine: Vec<_> = drain_saved().into_iter().filter(|(t, _)| *t <= 3).collect();
        assert_eq!(mine, vec![(1, Done::Saved(None)), (2, Done::Cancelled), (3, Done::Failed("unknown error".into()))]);
    }

    #[test]
    fn a_saved_or_opened_document_gets_its_location() {
        let mut f = FLIGHTS.lock().unwrap();
        f.push(Flight { ticket: 20, name: "u.docx".into(), path: String::new(), write: false });
        f.push(Flight { ticket: 21, name: "a.pdf".into(), path: String::new(), write: false });
        drop(f);
        // Save As picked "quokka (1).docx": the document lives there now.
        save_done(20, 1, "content://d/u".into(), "quokka (1).docx".into());
        // An export keeps no location.
        save_done(21, 1, "content://d/p".into(), "a.pdf".into());
        let mine: Vec<_> = drain_saved().into_iter().filter(|(t, _)| *t == 20 || *t == 21).collect();
        assert_eq!(mine, vec![(20, Done::Saved(Some("content://d/u/quokka (1).docx".into()))), (21, Done::Saved(None))]);
        opened("quokka.docx".into(), "/nonexistent".into(), "content://d/q".into());
        let picked: Vec<_> = drain_opened().into_iter().filter(|(l, _)| l.starts_with("content://d/q")).collect();
        assert_eq!(picked, vec![("content://d/q/quokka.docx".to_string(), Vec::new())]);
    }

    #[test]
    fn recent_round_trips_document_locations_only() {
        let list = vec!["content://d/u/quokka.docx".to_string(), "/data/old.docx".to_string(), "content://d/v/a b.docx".to_string()];
        assert_eq!(parse_recent(&recent_text(&list)), vec!["content://d/u/quokka.docx".to_string(), "content://d/v/a b.docx".to_string()]);
        assert!(parse_recent("\n\ncontent://x\n").is_empty());
    }

    #[test]
    fn open_with_accepts_word_rtf_odt_and_text_by_name_or_type() {
        assert!(accepts_document("quokka.docx", "application/octet-stream"), "a provider's octet-stream .docx");
        assert!(accepts_document("QUOKKA.DOCX", ""));
        assert!(accepts_document("old.doc", "application/msword"));
        assert!(accepts_document("notes.rtf", ""));
        assert!(accepts_document("letter.odt", ""));
        assert!(accepts_document("readme.md", "text/markdown"));
        assert!(accepts_document("notes", "text/plain"), "no extension, but a text type");
        assert!(!accepts_document("photo.jpg", "image/jpeg"));
        assert!(!accepts_document("report.pdf", "application/pdf"));
        assert!(!accepts_document("data.bin", "application/octet-stream"), "octet-stream alone is not a document");
        assert!(!accepts_document("archive.docx.zip", "application/zip"));
        assert!(!accepts_document("", ""));
    }

    #[test]
    fn plain_save_writes_back_to_the_open_documents_uri() {
        set_current(Some("content://d/u/quokka.docx".into()));
        assert_eq!(save_target().as_deref(), Some("content://d/u"));
        set_current(Some("/data/x.docx".into()));
        assert_eq!(save_target(), None);
        set_current(None);
        assert_eq!(save_target(), None, "a new document asks where to save");
    }
}
