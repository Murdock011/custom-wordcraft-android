//! Keyboard input from the Android `InputConnection` (`ImeView` in MainActivity.kt).
//!
//! Kotlin calls the JNI exports in `android.rs` on the UI thread, and they push a `Msg` here.
//! `Shell::logic` pops the messages one at a time and turns them into egui events. The queue is
//! the only shared state, so this module has no Android or egui dependency and runs in host tests.
//!
//! Offsets: the input method counts UTF-16 code units for composing regions and the caret; the
//! document counts bytes. The helpers below convert between them, and refuse offsets that are
//! past the end or split a surrogate pair (they come from the keyboard, so they are hostile).

use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

#[derive(Debug, PartialEq, Eq)]
pub enum Msg {
    /// `setComposingText`: the text of the composition so far (empty ends it).
    Compose(String),
    /// `commitText`: final text at the caret (replaces the composition).
    Commit(String),
    /// `finishComposingText`: the composition becomes final text.
    Finish,
    /// `deleteSurroundingText`: characters before and after the caret (code points).
    Delete { before: usize, after: usize },
    /// `setComposingRegion`: UTF-16 offsets into the context text (`start..end`).
    Region { start: usize, end: usize },
    /// A key press with an Android `KeyEvent` code (Enter, DEL, forward DEL, Escape).
    Key(i32),
}

static QUEUE: Mutex<VecDeque<Msg>> = Mutex::new(VecDeque::new());

/// The context the input method reads (`context_text`), encoded for Kotlin, written each frame.
static CONTEXT: Mutex<String> = Mutex::new(String::new());

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

/// The oldest queued message, if any.
pub fn pop() -> Option<Msg> {
    QUEUE.lock().ok()?.pop_front()
}

/// Stores the encoded context for Kotlin to read (`context_text`).
pub fn publish_context(encoded: String) {
    if let Ok(mut c) = CONTEXT.lock() {
        *c = encoded;
    }
}

/// The last published context, for Kotlin (empty until the first frame).
pub fn context_text() -> String {
    CONTEXT.lock().map(|c| c.clone()).unwrap_or_default()
}

/// UTF-16 length of `s`.
pub fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// The byte offset in `text` of the UTF-16 offset `units`, or `None` when it is past the end or
/// falls inside a surrogate pair.
pub fn byte_at_utf16(text: &str, units: usize) -> Option<usize> {
    let mut u = 0usize;
    for (b, c) in text.char_indices() {
        if u == units {
            return Some(b);
        }
        u = u.checked_add(c.len_utf16())?;
        if u > units {
            return None;
        }
    }
    (u == units).then_some(text.len())
}

/// The UTF-16 offset of byte `byte` in `text` (0 when `byte` is not a boundary).
pub fn utf16_at_byte(text: &str, byte: usize) -> usize {
    text.get(..byte).map_or(0, utf16_len)
}

/// Text being composed (the input method's composing span), shown at the caret and not yet in the
/// document. `caret` is a byte offset in `text`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Composition {
    pub text: String,
    pub caret: usize,
}

impl Composition {
    /// A composition that ends with its caret at the end of `text`.
    pub fn new(text: String) -> Self {
        let caret = text.len();
        Self { text, caret }
    }

    /// The caret inside the composition, in chars (what the overlay reports as its cursor).
    pub fn caret_chars(&self) -> usize {
        self.text.get(..self.caret).map_or(0, |s| s.chars().count())
    }
}

/// Erases `before` chars before and `after` chars after the caret, the composition's chars first.
/// Returns what is left to erase from the document around the composition's place.
pub fn erase_around_caret(comp: &mut Composition, before: usize, after: usize) -> (usize, usize) {
    let head = comp.text.get(..comp.caret).unwrap_or("");
    let take_b = before.min(head.chars().count());
    let start = if take_b == 0 { comp.caret } else { head.char_indices().rev().nth(take_b - 1).map_or(0, |(i, _)| i) };
    let tail = comp.text.get(comp.caret..).unwrap_or("");
    let take_a = after.min(tail.chars().count());
    let end = comp.caret.saturating_add(tail.char_indices().nth(take_a).map_or(tail.len(), |(i, _)| i));
    let (Some(kept_head), Some(kept_tail)) = (comp.text.get(..start), comp.text.get(end..)) else {
        return (before, after);
    };
    comp.text = format!("{kept_head}{kept_tail}");
    comp.caret = start;
    (before.saturating_sub(take_b), after.saturating_sub(take_a))
}

/// A composing region taken from the text before the caret: the document loses `before_chars`
/// before and `after_chars` after the caret, and the removed `text` becomes the composition with its
/// caret at byte `caret`.
#[derive(Debug, PartialEq, Eq)]
pub struct Region {
    pub before_chars: usize,
    pub after_chars: usize,
    pub text: String,
    pub caret: usize,
}

/// The composing region `start..end` (UTF-16, in `text`) as the input method re-composes a word
/// it puts under the caret. `None` when the offsets are invalid or the caret is outside it.
pub fn region_in_text(text: &str, caret: usize, start: usize, end: usize) -> Option<Region> {
    let (s, e) = (start.min(end), start.max(end));
    let sb = byte_at_utf16(text, s)?;
    let eb = byte_at_utf16(text, e)?;
    if !text.is_char_boundary(caret) || caret < sb || caret > eb {
        return None;
    }
    let before = text.get(sb..caret)?;
    let after = text.get(caret..eb)?;
    Some(Region { before_chars: before.chars().count(), after_chars: after.chars().count(), text: text.get(sb..eb)?.to_string(), caret: caret - sb })
}

/// What the input method reads: the paragraph with the composition inside it, and where the caret
/// and the composition are, in UTF-16 units. `composing` is `None` without a composition.
#[derive(Debug, PartialEq, Eq)]
pub struct Context {
    pub text: String,
    pub caret: usize,
    pub composing: Option<(usize, usize)>,
}

/// The context around the caret at byte `caret` of `para`, with `comp` spliced in at the caret.
/// `None` when `caret` is not a character boundary of `para`.
pub fn context(para: &str, caret: usize, comp: &Composition) -> Option<Context> {
    let head = para.get(..caret)?;
    let tail = para.get(caret..)?;
    let at = utf16_len(head);
    let comp_len = utf16_len(&comp.text);
    let text = format!("{head}{}{tail}", comp.text);
    let caret16 = at.checked_add(utf16_at_byte(&comp.text, comp.caret))?;
    let composing = (!comp.text.is_empty()).then(|| (at, at.saturating_add(comp_len)));
    Some(Context { text, caret: caret16, composing })
}

/// One context line for Kotlin: `ack`, caret, composing start and end (-1 for none), then the text.
/// Fields are split by U+0001, and the text comes last so it may contain anything else.
pub fn encode(ack: u64, c: &Context) -> String {
    let (s, e) = c.composing.map_or((-1, -1), |(s, e)| (s as i64, e as i64));
    format!("{ack}\u{1}{}\u{1}{s}\u{1}{e}\u{1}{}", c.caret, c.text)
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
    fn messages_pop_in_order_once() {
        push(Msg::Compose("he".into()));
        push(Msg::Commit("hello".into()));
        assert_eq!(pop(), Some(Msg::Compose("he".into())));
        assert_eq!(pop(), Some(Msg::Commit("hello".into())));
        assert_eq!(pop(), None);
    }

    #[test]
    fn utf16_offsets_map_over_accents_and_emoji() {
        // "a" 1 unit, "é" 1 unit (2 bytes), "😀" 2 units (surrogate pair, 4 bytes), "b" 1 unit.
        let t = "aé😀b";
        assert_eq!(utf16_len(t), 5);
        assert_eq!(byte_at_utf16(t, 0), Some(0));
        assert_eq!(byte_at_utf16(t, 1), Some(1));
        assert_eq!(byte_at_utf16(t, 2), Some(3), "after é");
        assert_eq!(byte_at_utf16(t, 3), None, "inside the emoji's surrogate pair");
        assert_eq!(byte_at_utf16(t, 4), Some(7), "after the emoji");
        assert_eq!(byte_at_utf16(t, 6), None, "past the end");
        assert_eq!(utf16_at_byte(t, 7), 4);
        assert_eq!(utf16_at_byte(t, 2), 0, "not a boundary: no offset");
    }

    #[test]
    fn erasing_takes_the_composition_first_then_the_document() {
        let mut c = Composition::new("hé".into());
        assert_eq!(erase_around_caret(&mut c, 1, 0), (0, 0));
        assert_eq!(c, Composition { text: "h".into(), caret: 1 }, "the accented letter goes whole");
        let mut c = Composition::new("hé".into());
        assert_eq!(erase_around_caret(&mut c, 3, 0), (1, 0), "two from the composition, one more from the document");
        assert!(c.text.is_empty());
    }

    #[test]
    fn erasing_after_the_caret_inside_the_composition() {
        let mut c = Composition { text: "wor".into(), caret: 1 };
        assert_eq!(erase_around_caret(&mut c, 0, 5), (0, 3), "two chars come from the composition, three from the document");
        assert_eq!(c, Composition { text: "w".into(), caret: 1 });
    }

    #[test]
    fn a_region_becomes_the_composition_with_the_caret_inside_it() {
        // "I like hello" with the caret after "hel" of "hello" (UTF-16 7..12 is "hello").
        let r = region_in_text("I like hello", 10, 7, 12).expect("valid");
        assert_eq!(r, Region { before_chars: 3, after_chars: 2, text: "hello".into(), caret: 3 });
    }

    #[test]
    fn a_region_with_non_ascii_text_counts_chars_and_units() {
        // "café 😀": the region "é " is UTF-16 3..5 (bytes 3..6), the caret after its space (byte 6).
        let r = region_in_text("café 😀", 6, 3, 5).expect("valid");
        assert_eq!(r.text, "é ");
        assert_eq!((r.before_chars, r.after_chars), (2, 0), "é and the space are before the caret");
        assert_eq!(r.caret, 3, "the caret is after the 3 bytes of é and the space's start");
        assert!(region_in_text("café 😀", 6, 0, 6).is_none(), "the region's end, 6 units, splits the emoji");
    }

    #[test]
    fn a_region_the_caret_is_outside_or_that_is_hostile_is_refused() {
        assert_eq!(region_in_text("abcdef", 6, 0, 3), None, "caret after the region");
        assert_eq!(region_in_text("abc", 1, 0, 9), None, "past the end");
        assert_eq!(region_in_text("a😀", 1, 0, 2), None, "end splits the emoji");
        assert!(region_in_text("abc", 1, 3, 0).is_some(), "reversed bounds are sorted");
    }

    #[test]
    fn the_context_splices_the_composition_at_the_caret() {
        let para = "café";
        let comp = Composition::new("wo".into());
        // Caret after "café" (5 bytes, 4 units): the composition sits there.
        let c = context(para, 5, &comp).expect("boundary");
        assert_eq!(c, Context { text: "caféwo".into(), caret: 6, composing: Some((4, 6)) });
        assert_eq!(context(para, 4, &comp), None, "inside é is not a boundary");
        let none = context("ab", 1, &Composition::default()).expect("boundary");
        assert_eq!(none, Context { text: "ab".into(), caret: 1, composing: None });
    }

    #[test]
    fn the_encoded_context_keeps_the_text_last() {
        let c = Context { text: "a\u{1}b".into(), caret: 2, composing: None };
        assert_eq!(encode(7, &c), "7\u{1}2\u{1}-1\u{1}-1\u{1}a\u{1}b");
    }

    /// A paragraph as the input method and the document see it, driven the way `Shell` drives it.
    #[derive(Default)]
    struct Doc {
        para: String,
        caret: usize,
        comp: Composition,
    }

    impl Doc {
        /// The text as the input method sees it: the paragraph with the composition in it.
        fn seen(&self) -> String {
            context(&self.para, self.caret, &self.comp).map(|c| c.text).unwrap_or_default()
        }
        fn compose(&mut self, t: &str) {
            self.comp = Composition::new(t.into());
        }
        fn delete(&mut self, before: usize, after: usize) {
            // The composition's chars go first; what is left erases the paragraph before the caret.
            let (b, _) = erase_around_caret(&mut self.comp, before, after);
            if b > 0 {
                let head = &self.para[..self.caret];
                let cut = head.char_indices().rev().nth(b - 1).map_or(0, |(i, _)| i);
                self.para.replace_range(cut..self.caret, "");
                self.caret = cut;
            }
        }
        fn commit(&mut self, t: &str) {
            self.comp = Composition::default();
            self.para.insert_str(self.caret, t);
            self.caret += t.len();
        }
        fn region(&mut self, s: usize, e: usize) {
            if !self.comp.text.is_empty() {
                return;
            }
            if let Some(r) = region_in_text(&self.para, self.caret, s, e) {
                let end = self.caret + self.para[self.caret..].chars().take(r.after_chars).map(char::len_utf8).sum::<usize>();
                let begin = self.caret - self.para[..self.caret].chars().rev().take(r.before_chars).map(char::len_utf8).sum::<usize>();
                self.para.replace_range(begin..end, "");
                self.caret = begin;
                self.comp = Composition { text: r.text, caret: r.caret };
            }
        }
        fn finish(&mut self) {
            let t = std::mem::take(&mut self.comp.text);
            self.comp = Composition::default();
            self.commit(&t);
        }
    }

    #[test]
    fn a_suggestion_replaces_the_composing_word_once() {
        let mut d = Doc { para: "say ".into(), caret: 4, ..Default::default() };
        d.compose("helo");
        // Gboard taps the "hello" suggestion: it deletes the composing word, then commits it.
        d.delete(4, 0);
        d.commit("hello ");
        assert_eq!(d.para, "say hello ", "once, with no composition left");
        assert!(d.comp.text.is_empty());
    }

    #[test]
    fn autocorrect_on_space_replaces_the_word_once() {
        let mut d = Doc { para: String::new(), caret: 0, ..Default::default() };
        d.compose("teh");
        d.delete(3, 0);
        d.commit("the ");
        assert_eq!(d.para, "the ");
    }

    #[test]
    fn backspacing_into_a_word_composes_it_and_typing_edits_it_in_place() {
        let mut d = Doc { para: "hello ".into(), caret: 5, ..Default::default() };
        // The caret after "hello": Gboard recomposes the word before it.
        d.region(0, 5);
        assert_eq!(d.para, " ", "the word left the document");
        assert_eq!(d.comp.text, "hello");
        assert_eq!(d.seen(), "hello ");
        // Typing replaces the composition with the edited word; no second copy.
        d.compose("hellox");
        d.finish();
        assert_eq!(d.para, "hellox ");
    }

    #[test]
    fn deleting_across_the_composition_keeps_the_text_before_it() {
        let mut d = Doc { para: "ab".into(), caret: 2, ..Default::default() };
        d.compose("c");
        d.delete(2, 0);
        assert_eq!(d.para, "a", "one char from the composition, one from the document");
        assert_eq!(d.caret, 1);
    }
}
