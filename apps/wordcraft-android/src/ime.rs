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
/// Whether the document canvas has keyboard focus for editing, with no dialog, sheet or File page
/// open. Written each frame; a resume shows the keyboard only when this is set.
pub static CANVAS_EDITING: AtomicBool = AtomicBool::new(false);
/// File › Close ended the app: Kotlin finishes the activity.
pub static QUIT: AtomicBool = AtomicBool::new(false);
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
/// A span of document text the input method is composing. The text stays in the document; the
/// span only marks it, so nothing is held outside the document. Offsets are bytes in the paragraph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    /// Where the span starts in the paragraph.
    pub start: usize,
    /// The caret, where the input method left it (inside `start..end`).
    pub caret: usize,
    /// The text of the span, as the document holds it.
    pub text: String,
}

impl Span {
    pub fn end(&self) -> usize {
        self.start.saturating_add(self.text.len())
    }

    /// Chars before and after the caret inside the span: what erasing the span takes.
    pub fn erase_counts(&self) -> (usize, usize) {
        let rel = self.caret.saturating_sub(self.start);
        let head = self.text.get(..rel).unwrap_or("");
        let tail = self.text.get(rel..).unwrap_or("");
        (head.chars().count(), tail.chars().count())
    }
}

/// Whether `span` still holds in `para` with the caret at `caret`: the text is still there, and the
/// caret has not moved since the span was last read.
pub fn holds(span: &Span, para: &str, caret: usize) -> bool {
    caret == span.caret
        && span.caret >= span.start
        && span.caret <= span.end()
        && para.is_char_boundary(caret)
        && para.get(span.start..span.end()) == Some(span.text.as_str())
}

/// The span read again after an edit made for it: from `start` to the caret, as the paragraph now
/// reads there (a correction may have changed its length). `None` when the caret is before `start`
/// or nothing is left.
pub fn resynced(start: usize, para: &str, caret: usize) -> Option<Span> {
    if caret < start || !para.is_char_boundary(caret) || !para.is_char_boundary(start) {
        return None;
    }
    let text = para.get(start..caret)?.to_string();
    (!text.is_empty()).then_some(Span { start, caret, text })
}

/// The span the held one is at now. After an edit made for it (`fresh`), it is read again; before
/// that, it is kept only while it holds. `None` ends the composition, and the text stays where it is.
pub fn sync(held: &Span, fresh: bool, para: &str, caret: usize) -> Option<Span> {
    if fresh { resynced(held.start, para, caret) } else { holds(held, para, caret).then(|| held.clone()) }
}

/// The composing region `start..end` (UTF-16, in `para`) the input method set over text. The words
/// stay where they are, marked as the composition, with the caret inside them. `None` when the
/// offsets are invalid, the caret is outside them, or the region is empty.
pub fn region_span(para: &str, caret: usize, start: usize, end: usize) -> Option<Span> {
    let (s, e) = (start.min(end), start.max(end));
    let sb = byte_at_utf16(para, s)?;
    let eb = byte_at_utf16(para, e)?;
    if sb == eb || !para.is_char_boundary(caret) || caret < sb || caret > eb {
        return None;
    }
    Some(Span { start: sb, caret, text: para.get(sb..eb)?.to_string() })
}

/// A document edit the Shell makes, in order: erase chars around the caret, or insert text at it.
#[derive(Debug, PartialEq, Eq)]
pub enum Op {
    Erase { before: usize, after: usize },
    Insert(String),
}

/// The edits that put `t` in place of the span (or at the caret when there is no span): erase the
/// span, then insert the text.
pub fn replace_ops(span: Option<&Span>, t: &str) -> Vec<Op> {
    let mut ops = Vec::new();
    if let Some(s) = span {
        let (before, after) = s.erase_counts();
        if before.saturating_add(after) > 0 {
            ops.push(Op::Erase { before, after });
        }
    }
    if !t.is_empty() {
        ops.push(Op::Insert(t.to_string()));
    }
    ops
}

/// What the input method reads: the paragraph, the caret and the composing span, in UTF-16 units.
/// `composing` is `None` without a span.
#[derive(Debug, PartialEq, Eq)]
pub struct Context {
    pub text: String,
    pub caret: usize,
    pub composing: Option<(usize, usize)>,
}

/// The context around the caret at byte `caret` of `para`. `None` when `caret` is not a character
/// boundary of `para`.
pub fn context(para: &str, caret: usize, span: Option<&Span>) -> Option<Context> {
    if !para.is_char_boundary(caret) {
        return None;
    }
    let composing = span.map(|s| (utf16_at_byte(para, s.start), utf16_at_byte(para, s.end())));
    Some(Context { text: para.to_string(), caret: utf16_at_byte(para, caret), composing })
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
    fn a_region_marks_the_words_and_leaves_them_in_place() {
        // "I like hello" with the caret after "hel" of "hello" (UTF-16 7..12 is "hello").
        let s = region_span("I like hello", 10, 7, 12).expect("valid");
        assert_eq!(s, Span { start: 7, caret: 10, text: "hello".into() });
        assert_eq!(s.erase_counts(), (3, 2));
    }

    #[test]
    fn a_region_with_non_ascii_text_counts_bytes_not_units() {
        // "café 😀": the region "é " is UTF-16 3..5 (bytes 3..6), the caret after its space (byte 6).
        let s = region_span("café 😀", 6, 3, 5).expect("valid");
        assert_eq!(s.text, "é ");
        assert_eq!(s.start, 3);
        assert_eq!(s.erase_counts(), (2, 0), "é and the space are before the caret");
        assert!(region_span("café 😀", 6, 0, 6).is_none(), "the region's end, 6 units, splits the emoji");
    }

    #[test]
    fn a_region_the_caret_is_outside_or_that_is_hostile_is_refused() {
        assert_eq!(region_span("abcdef", 6, 0, 3), None, "caret after the region");
        assert_eq!(region_span("abc", 1, 0, 9), None, "past the end");
        assert_eq!(region_span("a😀", 1, 0, 2), None, "end splits the emoji");
        assert_eq!(region_span("abc", 1, 1, 1), None, "empty region");
        assert!(region_span("abc", 1, 3, 0).is_some(), "reversed bounds are sorted");
    }

    #[test]
    fn a_span_holds_only_while_its_text_and_caret_are_where_they_were() {
        let s = Span { start: 4, caret: 6, text: "hello".into() };
        assert!(holds(&s, "say hello now", 6));
        assert!(!holds(&s, "say hello now", 7), "the caret moved");
        assert!(!holds(&s, "say hallo now", 6), "the text changed");
        assert!(!holds(&s, "say", 6), "the text is gone");
    }

    #[test]
    fn a_fresh_span_is_read_again_from_the_document() {
        // Autocorrect made "teh " into "the " after the span was set at 0.
        let held = Span { start: 0, caret: 3, text: "teh".into() };
        assert_eq!(sync(&held, true, "the ", 4), Some(Span { start: 0, caret: 4, text: "the ".into() }));
        assert_eq!(sync(&held, true, "x", 0), None, "nothing left");
        assert_eq!(sync(&held, false, "the ", 4), None, "not fresh: its text no longer matches");
    }

    #[test]
    fn the_context_marks_the_span_and_keeps_the_text_as_it_is() {
        let s = Span { start: 3, caret: 5, text: "é".into() };
        let c = context("café", 5, Some(&s)).expect("boundary");
        assert_eq!(c, Context { text: "café".into(), caret: 4, composing: Some((3, 4)) });
        assert_eq!(context("café", 4, None), None, "inside é is not a boundary");
        let none = context("ab", 1, None).expect("boundary");
        assert_eq!(none, Context { text: "ab".into(), caret: 1, composing: None });
    }

    #[test]
    fn the_encoded_context_keeps_the_text_last() {
        let c = Context { text: "a\u{1}b".into(), caret: 2, composing: None };
        assert_eq!(encode(7, &c), "7\u{1}2\u{1}-1\u{1}-1\u{1}a\u{1}b");
    }

    /// A paragraph as the document holds it, edited the way the Shell's events edit it.
    struct Doc {
        para: String,
        caret: usize,
    }

    impl Doc {
        fn apply(&mut self, op: &Op) {
            match *op {
                Op::Erase { before, after } => {
                    let start =
                        if before == 0 { self.caret } else { self.para[..self.caret].char_indices().rev().nth(before - 1).map_or(0, |(i, _)| i) };
                    let tail = &self.para[self.caret..];
                    let end = self.caret + if after == 0 { 0 } else { tail.char_indices().nth(after).map_or(tail.len(), |(i, _)| i) };
                    self.para.replace_range(start..end, "");
                    self.caret = start;
                }
                Op::Insert(ref t) => {
                    self.para.insert_str(self.caret, t);
                    self.caret += t.len();
                }
            }
        }
    }

    #[test]
    fn a_region_then_a_caret_move_leaves_the_text_alone() {
        let mut d = Doc { para: "I like hello now".into(), caret: 10 };
        let s = region_span(&d.para, d.caret, 7, 12).expect("valid");
        d.caret = 0; // a tap somewhere else
        assert_eq!(sync(&s, false, &d.para, d.caret), None, "the span ends");
        assert_eq!(d.para, "I like hello now", "no word lost, none moved");
    }

    #[test]
    fn a_region_then_a_save_reads_the_text_that_is_there() {
        let d = Doc { para: "I like hello now".into(), caret: 10 };
        let _ = region_span(&d.para, d.caret, 7, 12).expect("valid");
        assert_eq!(d.para, "I like hello now", "the document never lost the word");
    }

    #[test]
    fn a_region_then_composing_replaces_the_word_once() {
        let mut d = Doc { para: "say hello now".into(), caret: 6 };
        let mut s = region_span(&d.para, d.caret, 4, 9).expect("valid");
        for op in replace_ops(Some(&s), "words") {
            d.apply(&op);
        }
        s = resynced(s.start, &d.para, d.caret).expect("the new word");
        assert_eq!(d.para, "say words now", "once, in place");
        assert_eq!(s.text, "words");
        assert!(holds(&s, &d.para, d.caret));
    }

    #[test]
    fn a_region_then_finish_leaves_the_text_alone() {
        let d = Doc { para: "I like hello now".into(), caret: 10 };
        let s = region_span(&d.para, d.caret, 7, 12).expect("valid");
        // Finish drops the span; the document keeps the word.
        assert!(holds(&s, &d.para, d.caret));
        assert_eq!(d.para, "I like hello now");
    }

    #[test]
    fn a_suggestion_replaces_the_composing_word_once() {
        let mut d = Doc { para: "say helo".into(), caret: 8 };
        let s = Span { start: 4, caret: 8, text: "helo".into() };
        for op in replace_ops(Some(&s), "hello ") {
            d.apply(&op);
        }
        assert_eq!(d.para, "say hello ", "once");
    }

    #[test]
    fn deleting_across_the_span_keeps_the_text_before_it() {
        let mut d = Doc { para: "ab c".into(), caret: 4 };
        let s = Span { start: 2, caret: 4, text: " c".into() };
        // Two chars before the caret: the span's "c" and the space before it.
        d.apply(&Op::Erase { before: 2, after: 0 });
        assert_eq!(d.para, "ab", "both chars go, the text before the span stays");
        assert_eq!(sync(&s, true, &d.para, d.caret), None, "nothing is left of the span");
    }

    /// The composition state `app.rs` keeps (`held`, `fresh`) and the messages it handles
    /// (`handle_ime`), over the same `Doc`. The Shell is Android-only, so these run its steps here.
    struct Typist {
        doc: Doc,
        held: Option<Span>,
        fresh: bool,
    }

    impl Typist {
        fn at(para: &str, caret: usize) -> Self {
            Self { doc: Doc { para: para.into(), caret }, held: None, fresh: false }
        }

        /// A tap: the caret moves, and the held span (if any) is checked on the next message.
        fn tap(&mut self, caret: usize) {
            self.doc.caret = caret;
        }

        fn sync_held(&mut self) {
            let Some(h) = self.held.take() else { return };
            self.held = sync(&h, self.fresh, &self.doc.para, self.doc.caret);
            self.fresh = false;
        }

        /// `replace_held`: puts `t` in place of the span, or at the caret when none is held.
        fn replace_held(&mut self, t: &str, keep: bool) {
            let start = self.held.as_ref().map_or(self.doc.caret, |h| h.start);
            for op in replace_ops(self.held.as_ref(), t) {
                self.doc.apply(&op);
            }
            self.held = (keep && !t.is_empty()).then(|| Span { start, caret: start, text: String::new() });
            self.fresh = true;
        }

        fn handle(&mut self, m: Msg) {
            self.sync_held();
            match m {
                Msg::Compose(t) => self.replace_held(&t, true),
                Msg::Commit(t) => self.replace_held(&t, false),
                Msg::Finish => self.held = None,
                Msg::Delete { before, after } => {
                    if before + after > 0 {
                        self.doc.apply(&Op::Erase { before, after });
                    }
                    if self.held.is_some() {
                        self.fresh = true;
                    }
                }
                _ => {}
            }
        }

        /// Gboard's typing of `word`: each prefix is set as the composition, then the word commits.
        fn type_word(&mut self, word: &str) {
            let chars: Vec<char> = word.chars().collect();
            for n in 1..chars.len() {
                self.handle(Msg::Compose(chars[..n].iter().collect()));
            }
            self.handle(Msg::Commit(word.into()));
        }
    }

    #[test]
    fn commit_then_backspace_at_the_end_of_a_word_erases_one_letter() {
        let mut t = Typist::at("", 0);
        t.type_word("Hello");
        assert_eq!((t.doc.para.as_str(), t.doc.caret), ("Hello", 5));
        // Gboard's Backspace: finish the composition, then delete the character before the caret.
        t.handle(Msg::Finish);
        t.handle(Msg::Delete { before: 1, after: 0 });
        assert_eq!((t.doc.para.as_str(), t.doc.caret), ("Hell", 4));
        assert!(t.held.is_none());
    }

    #[test]
    fn commit_then_backspace_mid_word_leaves_the_word_as_it_was() {
        let mut t = Typist::at("Thursday", 5);
        // Gboard's `q`: composed, then committed; Backspace removes it again.
        t.handle(Msg::Compose("q".into()));
        t.handle(Msg::Commit("q".into()));
        assert_eq!((t.doc.para.as_str(), t.doc.caret), ("Thursqday", 6));
        t.handle(Msg::Finish);
        t.handle(Msg::Delete { before: 1, after: 0 });
        assert_eq!((t.doc.para.as_str(), t.doc.caret), ("Thursday", 5), "no stray letter, caret after Thurs");
    }

    #[test]
    fn commit_then_backspace_after_a_tap_removes_only_the_typed_letter() {
        let mut t = Typist::at("xyz", 3);
        t.type_word("abc");
        assert_eq!((t.doc.para.as_str(), t.doc.caret), ("xyzabc", 6));
        // A tap after the first letter moves the caret; the next composition lands there.
        t.tap(1);
        t.handle(Msg::Compose("x".into()));
        t.handle(Msg::Commit("x".into()));
        assert_eq!((t.doc.para.as_str(), t.doc.caret), ("xxyzabc", 2));
        t.handle(Msg::Finish);
        t.handle(Msg::Delete { before: 1, after: 0 });
        assert_eq!((t.doc.para.as_str(), t.doc.caret), ("xyzabc", 1), "the x at the tap is gone, the rest is unchanged");
    }
}
