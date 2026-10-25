//! Keyboard input from the Android `InputConnection` (`ImeView` in MainActivity.kt).
//!
//! Kotlin calls the JNI exports in `android.rs` on the UI thread, and they push a `Msg` here.
//! `Shell::logic` drains the queue each frame and turns the messages into egui events. The queue is
//! the only shared state, so this module has no Android or egui dependency and runs in host tests.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

#[derive(Debug, PartialEq, Eq)]
pub enum Msg {
    /// `setComposingText`: the text of the composition so far (empty ends it).
    Compose(String),
    /// `commitText`: final text at the caret.
    Commit(String),
    /// `finishComposingText`: the composition becomes final text.
    Finish,
    /// `deleteSurroundingText`: characters before and after the caret.
    Delete { before: usize, after: usize },
    /// A key press with an Android `KeyEvent` code (Enter, DEL, forward DEL, Escape).
    Key(i32),
}

static QUEUE: Mutex<VecDeque<Msg>> = Mutex::new(VecDeque::new());

/// Whether egui wants the soft keyboard (a canvas or text field is focused). Written each frame.
pub static WANTS_KEYBOARD: AtomicBool = AtomicBool::new(false);
/// Whether Android Back has a sheet, menu or Backstage to close. Written each frame.
pub static CAN_GO_BACK: AtomicBool = AtomicBool::new(false);
/// Whether the system is in night mode (set by Kotlin; winit reports no theme on Android).
pub static NIGHT: AtomicBool = AtomicBool::new(false);

/// Set on the first frame so `push` can wake the event loop, which sleeps until an event arrives.
pub static WAKE: std::sync::OnceLock<Box<dyn Fn() + Send + Sync>> = std::sync::OnceLock::new();

/// Wakes the event loop (a no-op before the first frame).
pub fn wake() {
    if let Some(f) = WAKE.get() {
        f();
    }
}

/// Left and top padding of the surface in pixels (set by Kotlin); touch positions arrive in window
/// pixels, so the host subtracts it.
pub static INSET_LEFT: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);
pub static INSET_TOP: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

/// Set by Kotlin when the activity is backgrounded (onPause, onStop): the next frame copies unsaved
/// work to the recovery store, before Android can kill the process.
pub static FLUSH: AtomicBool = AtomicBool::new(false);

pub fn request_flush() {
    FLUSH.store(true, std::sync::atomic::Ordering::Relaxed);
    wake();
}

/// Whether a flush was requested since the last call.
pub fn take_flush() -> bool {
    FLUSH.swap(false, std::sync::atomic::Ordering::Relaxed)
}

pub fn push(m: Msg) {
    if let Ok(mut q) = QUEUE.lock() {
        q.push_back(m);
    }
    wake();
}

pub fn drain() -> Vec<Msg> {
    QUEUE.lock().map(|mut q| q.drain(..).collect()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flush_request_is_taken_once() {
        assert!(!take_flush());
        request_flush();
        request_flush();
        assert!(take_flush(), "the request is seen");
        assert!(!take_flush(), "and seen only once");
    }

    #[test]
    fn messages_drain_in_order_once() {
        push(Msg::Compose("he".into()));
        push(Msg::Commit("hello".into()));
        assert_eq!(drain(), vec![Msg::Compose("he".into()), Msg::Commit("hello".into())]);
        assert!(drain().is_empty());
    }
}
