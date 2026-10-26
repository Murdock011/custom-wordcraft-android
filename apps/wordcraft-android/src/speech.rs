//! Read Aloud on Android: each sentence is spoken by `TextToSpeech` in `MainActivity.kt`.
//!
//! Rust never calls Kotlin (as in `clip.rs`). A sentence queues a request that Kotlin takes with
//! [`take_request`] on its poll and speaks; Kotlin reports how each one ended through [`event`].
//! The engine's player asks through `SpeechHost` (see `AndroidSpeech` below). The queue logic has
//! no Android or engine dependency, so it runs in host tests.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

/// Requests Kotlin has not taken yet. The player waits for each sentence, so this stays short.
const MAX_QUEUED: usize = 64;

enum Request {
    Speak { id: u64, text: String, lang: Option<String>, rate: f32 },
    Stop,
}

/// How the sentence with the latest id has gone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Speaking,
    Done,
    Failed(String),
}

static QUEUE: Mutex<VecDeque<Request>> = Mutex::new(VecDeque::new());
/// The id of the latest sentence, and its status. Events for other ids are old and ignored.
static LATEST: Mutex<(u64, Status)> = Mutex::new((0, Status::Speaking));
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn push(r: Request) {
    if let Ok(mut q) = QUEUE.lock()
        && q.len() < MAX_QUEUED
    {
        q.push_back(r);
    }
}

/// Queue `text` to be spoken in `lang` at `rate`; returns the id its events carry.
pub fn speak(text: &str, lang: Option<&str>, rate: f32) -> u64 {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    if let Ok(mut l) = LATEST.lock() {
        *l = (id, Status::Speaking);
    }
    push(Request::Speak { id, text: text.to_string(), lang: lang.map(str::to_string), rate });
    id
}

/// Queue a stop: Kotlin stops the speech it is making.
pub fn halt() {
    push(Request::Stop);
}

/// The status of sentence `id`: `Speaking` until its event comes (or for an id that isn't the latest).
pub fn status(id: u64) -> Status {
    match LATEST.lock() {
        Ok(l) if l.0 == id => l.1.clone(),
        _ => Status::Speaking,
    }
}

/// Kotlin: the next request for the speech, or an empty string when there is none. `x` is a stop;
/// `s<id>\t<rate>\t<lang>\t<text>` speaks (`lang` empty for the default voice). Tabs and line
/// breaks in the text become spaces, so the fields can't run into each other.
pub fn take_request() -> String {
    let Some(r) = QUEUE.lock().ok().and_then(|mut q| q.pop_front()) else { return String::new() };
    match r {
        Request::Stop => "x".into(),
        Request::Speak { id, text, lang, rate } => {
            let text: String = text.chars().map(|c| if matches!(c, '\t' | '\n' | '\r') { ' ' } else { c }).collect();
            format!("s{id}\t{rate:.2}\t{}\t{text}", lang.unwrap_or_default())
        }
    }
}

/// Kotlin: sentence `id` ended. `kind` 1 is done; 2 failed with `msg` (a sentence for the player).
pub fn event(id: u64, kind: i32, msg: String) {
    let status = match kind {
        1 => Status::Done,
        2 => Status::Failed(if msg.trim().is_empty() { "Text-to-speech failed.".into() } else { msg }),
        _ => return,
    };
    if let Ok(mut l) = LATEST.lock()
        && l.0 == id
    {
        l.1 = status;
    }
}

/// The host's speech for the engine's player, backed by the queue above.
#[cfg(target_os = "android")]
pub struct AndroidSpeech;

#[cfg(target_os = "android")]
impl wordcraft_engine::speech::SpeechHost for AndroidSpeech {
    fn speak(&self, text: &str, lang: Option<&str>, rate: f32) -> Result<u64, String> {
        Ok(speak(text, lang, rate))
    }

    fn poll(&self, id: u64) -> wordcraft_engine::speech::HostSpeech {
        use wordcraft_engine::speech::HostSpeech;
        match status(id) {
            Status::Speaking => HostSpeech::Speaking,
            Status::Done => HostSpeech::Done,
            Status::Failed(e) => HostSpeech::Failed(e),
        }
    }

    fn stop(&self) {
        halt();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sentence_is_queued_then_reported_done_or_failed() {
        let id = speak("One.\tTwo", Some("ru"), 1.5);
        assert_eq!(status(id), Status::Speaking);
        assert_eq!(take_request(), format!("s{id}\t1.50\tru\tOne. Two"));
        assert_eq!(take_request(), "", "taken once");
        event(id, 1, String::new());
        assert_eq!(status(id), Status::Done);
        let next = speak("Three.", None, 1.0);
        event(id, 2, "late".into());
        assert_eq!(status(next), Status::Speaking, "an old sentence's event is ignored");
        event(next, 2, String::new());
        assert_eq!(status(next), Status::Failed("Text-to-speech failed.".into()));
        assert_eq!(take_request(), format!("s{next}\t1.00\t\tThree."));
        halt();
        assert_eq!(take_request(), "x", "requests come out in the order they went in");
    }
}
