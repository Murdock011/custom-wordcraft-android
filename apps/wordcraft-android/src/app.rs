//! The desktop/web `WordApp` on the native window, opened on a blank document (File › Home over it
//! when the Start screen option is on, as the desktop app does with nothing to open).
//!
//! Keyboard input comes from the Kotlin `InputConnection` (`ImeView`, MainActivity.kt) through the
//! queue in `ime.rs`. Each frame the queued messages become egui IME and key events, which the
//! canvas already handles for the web build.

use std::sync::atomic::Ordering;

use egui::{Event, ImeEvent, Key};
use wordcraft_engine::Session;
use wordcraft_engine::doc::Pos;
use wordcraft_ui_egui::async_saves::{AsyncSaves, SaveOutcome};
use wordcraft_ui_egui::{Inbox, MAX_RECENT, Services, WordApp};

use crate::files::{self, Done, Request};
use crate::ime::{self, Msg, Op, Span};

/// Android `KeyEvent` codes the keyboard sends.
const KEYCODE_ESCAPE: i32 = 111;
const KEYCODE_DEL: i32 = 67;
const KEYCODE_FORWARD_DEL: i32 = 112;
const KEYCODE_ENTER: i32 = 66;

/// A composition: the span in the document, the paragraph's position it is in, and whether it was
/// just edited (then it is read again from the document before the next message).
struct Held {
    at: Pos,
    span: Span,
    fresh: bool,
}

pub struct Shell {
    app: WordApp,
    pending: Vec<Event>,
    /// The text the input method is composing. It stays in the document; this only marks it.
    held: Option<Held>,
    /// Set while events queued for egui have not been applied yet. No message is read then, so
    /// each one sees the document as the last one left it (`drain_ime`).
    applying: bool,
    /// Messages read from the queue so far. Published with the context: the input method knows
    /// its edits took effect once the context's count reaches what it sent.
    drained: u64,
    saves: AsyncSaves,
    inbox: Inbox,
    /// The Recent list as last kept on disk, so it is written only when it changes.
    saved_recent: Vec<String>,
}

impl Shell {
    pub fn new() -> Self {
        // A blank document on the session's region, as File › New makes it. Nothing to open is the
        // desktop's start-up (`apps/wordcraft/src/main.rs`), so the Start screen goes over it below.
        let mut session = Session::new(wordcraft_engine::doc::Document::new());
        let blank = session.blank_document();
        session.set_document(blank);
        // Read Aloud speaks through Android's TextToSpeech (`speech.rs`, MainActivity.kt), not a program.
        session.read_aloud.use_host(std::sync::Arc::new(crate::speech::AndroidSpeech));
        let saves = AsyncSaves::default();
        let inbox = Inbox::default();
        let services = Services {
            close_quits: true,
            open_async: Some(Box::new(|purpose: &str| files::request(Request::Open { purpose: purpose.into() }))),
            // Recent entries of picked documents are content locations: read them through the picker's grant.
            open_location: Some(Box::new(files::read_location)),
            // Exports ask for a name; the system picker decides where the file goes.
            pick_save: Some(Box::new(|name: &str| Some(name.to_string()))),
            async_saves: Some(saves.clone()),
            download: Some(Box::new({
                let saves = saves.clone();
                // Plain Save rewrites the picked document it was opened from or saved to; Save As,
                // exports and a first save ask with the system picker.
                move |path: &str, bytes: &[u8], save_as: bool| {
                    let name = std::path::Path::new(path).file_name().map_or_else(|| "document".into(), |n| n.to_string_lossy().into_owned());
                    let dir = files::DIR.get().cloned().unwrap_or_else(std::env::temp_dir);
                    let ticket = saves.start();
                    let file = dir.join(format!("save-{ticket}.bin"));
                    match std::fs::write(&file, bytes) {
                        Ok(()) => {
                            let path = file.to_string_lossy().into_owned();
                            match files::save_target().filter(|_| !save_as && !files::is_export(&name)) {
                                Some(uri) => files::request(Request::Write { ticket, name, uri, path }),
                                None => files::request(Request::Save { ticket, name, path }),
                            }
                        }
                        Err(e) => saves.finish(ticket, SaveOutcome::Failed(e.to_string())),
                    }
                    Ok(())
                }
            })),
            // The system print dialog prints the PDF from a cache file (Kotlin deletes it).
            print: Some(Box::new(|bytes: &[u8]| {
                let dir = files::DIR.get().cloned().unwrap_or_else(std::env::temp_dir);
                let file = dir.join("print.pdf");
                std::fs::write(&file, bytes).map_err(|e| e.to_string())?;
                files::request(Request::Print { path: file.to_string_lossy().into_owned() });
                Ok(())
            })),
            inbox: Some(inbox.clone()),
            clipboard_text: Some(Box::new(crate::clip::text)),
            ..Default::default()
        };
        let mut app = WordApp::new(session, services);
        // Local recovery, the desktop store: copies of unsaved work survive the process being killed.
        if let Some(dir) = files::recovery_dir() {
            let wake: std::sync::Arc<dyn Fn() + Send + Sync> = std::sync::Arc::new(ime::wake);
            if let Err(e) = app.session.enable_recovery(dir, wake) {
                log::warn!("local recovery unavailable: {e}");
            }
        }
        // Nothing to open: Home comes up over the blank document when the Start screen option is on.
        app.start_screen();
        let recent = files::load_recent();
        app.ui.recent = recent.clone();
        Self { app, pending: Vec::new(), held: None, applying: false, drained: 0, saves, inbox, saved_recent: recent }
    }

    /// A picked document now lives at `loc`: the title, Recent and plain Save follow it.
    fn adopt_location(&mut self, loc: String) {
        self.app.session.path = Some(loc.clone().into());
        self.app.ui.recent.retain(|r| *r != loc);
        self.app.ui.recent.insert(0, loc);
        self.app.ui.recent.truncate(MAX_RECENT);
    }

    fn drain_files(&mut self) {
        for (ticket, done) in files::drain_saved() {
            let outcome = match done {
                Done::Saved(location) => {
                    if let Some(loc) = location {
                        self.adopt_location(loc);
                    }
                    SaveOutcome::Saved
                }
                Done::Cancelled => SaveOutcome::Cancelled,
                Done::Failed(e) => SaveOutcome::Failed(e),
            };
            self.saves.finish(ticket, outcome);
        }
        let opened = files::drain_opened();
        if !opened.is_empty() {
            self.inbox.lock().unwrap_or_else(|e| e.into_inner()).extend(opened);
        }
        for msg in files::drain_notices() {
            self.app.status(msg);
        }
    }

    fn press(&mut self, key: Key) {
        for pressed in [true, false] {
            self.pending.push(Event::Key { key, physical_key: Some(key), pressed, repeat: false, modifiers: Default::default() });
        }
    }

    /// The caret's position and its paragraph's text, when the caret is a plain caret in text.
    fn focus_para(&self) -> Option<(Pos, String)> {
        let s = &self.app.session;
        if s.math.is_some() || !s.sel.is_collapsed() {
            return None;
        }
        let para = s.doc.para_at(&s.sel.focus)?;
        Some((s.sel.focus.clone(), para.text.clone()))
    }

    /// Keeps the held span only while the caret is where it was left and the text is unchanged.
    /// Otherwise the span is dropped and its text stays in the document.
    fn sync_held(&mut self) {
        let Some(h) = self.held.take() else { return };
        let Some((f, text)) = self.focus_para() else { return };
        if f.story != h.at.story || f.path != h.at.path {
            return;
        }
        if let Some(span) = ime::sync(&h.span, h.fresh, &text, f.off) {
            self.held = Some(Held { at: h.at, span, fresh: false });
        }
    }

    /// Queues the document edits for `op` as egui events.
    fn push_op(&mut self, op: &Op) {
        match op {
            Op::Erase { before, after } => self.pending.push(Event::Ime(ImeEvent::DeleteSurrounding { before_chars: *before, after_chars: *after })),
            Op::Insert(t) => self.pending.push(Event::Ime(ImeEvent::Commit(t.clone()))),
        }
    }

    /// Puts `t` in place of the held span, or at the caret when none is held. `keep` marks the
    /// result as a composition still in progress, which is read again after the edit.
    fn replace_held(&mut self, t: &str, keep: bool) {
        let focus = self.focus_para();
        let held = self.held.take();
        for op in ime::replace_ops(held.as_ref().map(|h| &h.span), t) {
            self.push_op(&op);
        }
        if !keep || t.is_empty() {
            return;
        }
        let (at, start) = match (held, focus) {
            (Some(h), _) => (h.at, h.span.start),
            (None, Some((f, _))) => {
                let start = f.off;
                (f, start)
            }
            (None, None) => return,
        };
        self.held = Some(Held { at, span: Span { start, caret: start, text: String::new() }, fresh: true });
    }

    /// `setComposingRegion`: the words the input method marks are kept in place as the composition.
    /// Nothing is deleted; the next edit replaces the span.
    fn recompose(&mut self, start: usize, end: usize) {
        self.held = None;
        let Some((f, text)) = self.focus_para() else { return };
        let Some(span) = ime::region_span(&text, f.off, start, end) else { return };
        self.held = Some(Held { at: f, span, fresh: false });
    }

    /// Reads the queue one message at a time, and stops after one that queued events, so the
    /// next message sees the document those events made. Reads nothing while events are applied.
    fn drain_ime(&mut self) {
        if self.applying {
            return;
        }
        while let Some(m) = ime::pop() {
            self.drained = self.drained.saturating_add(1);
            self.handle_ime(m);
            if !self.pending.is_empty() {
                break;
            }
        }
    }

    fn handle_ime(&mut self, m: Msg) {
        self.sync_held();
        match m {
            Msg::Compose(t) => self.replace_held(&t, true),
            Msg::Commit(t) => {
                if t == "\n" {
                    self.held = None;
                    self.press(Key::Enter);
                } else {
                    self.replace_held(&t, false);
                }
            }
            Msg::Finish => self.held = None,
            Msg::Delete { before, after } => {
                if before.saturating_add(after) > 0 {
                    self.push_op(&Op::Erase { before, after });
                }
                if let Some(h) = self.held.as_mut() {
                    h.fresh = true;
                }
            }
            Msg::Region { start, end } => self.recompose(start, end),
            Msg::Key(code) => {
                if code == KEYCODE_DEL && self.held.is_some() {
                    return self.handle_ime(Msg::Delete { before: 1, after: 0 });
                }
                // Any other key ends the composition; its text stays in the document.
                self.held = None;
                let key = match code {
                    KEYCODE_ENTER => Key::Enter,
                    KEYCODE_DEL => Key::Backspace,
                    KEYCODE_FORWARD_DEL => Key::Delete,
                    KEYCODE_ESCAPE => Key::Escape,
                    _ => return,
                };
                self.press(key);
            }
        }
    }

    /// The text around the caret for the input method: the paragraph and the composing span.
    fn context_line(&self) -> String {
        let context = self.focus_para().and_then(|(f, text)| ime::context(&text, f.off, self.held.as_ref().map(|h| &h.span)));
        let context = context.unwrap_or(ime::Context { text: String::new(), caret: 0, composing: None });
        ime::encode(self.drained, &context)
    }
}

impl eframe::App for Shell {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_ime();
        self.drain_files();
        if ime::take_flush() {
            self.app.session.flush_recovery();
        }
        files::set_current(self.app.session.path.as_ref().map(|p| p.to_string_lossy().into_owned()));
        if !self.pending.is_empty() {
            ctx.request_repaint();
        }
        self.app.logic(ctx);
        if self.app.ui.recent != self.saved_recent {
            files::save_recent(&self.app.ui.recent);
            self.saved_recent = self.app.ui.recent.clone();
        }
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        // Touch arrives in window pixels; the surface sits inside the system-bar padding.
        let ppp = ctx.pixels_per_point();
        let shift = egui::vec2(ime::INSET_LEFT.load(Ordering::Relaxed) as f32, ime::INSET_TOP.load(Ordering::Relaxed) as f32) / ppp;
        for e in &mut raw.events {
            match e {
                Event::PointerMoved(p) | Event::PointerButton { pos: p, .. } | Event::Touch { pos: p, .. } => *p -= shift,
                _ => {}
            }
        }
        if !self.pending.is_empty() {
            self.applying = true;
        }
        raw.events.append(&mut self.pending);
        raw.system_theme = Some(if ime::NIGHT.load(Ordering::Relaxed) { egui::Theme::Dark } else { egui::Theme::Light });
        self.app.raw_input_hook(raw);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Insets are applied by the Kotlin MainActivity as padding on the content view, so egui's
        // area is already the free area above the status bar and the keyboard.
        let ctx = ui.ctx().clone();
        let _ = ime::WAKE.set(Box::new(move || ctx.request_repaint()));
        let idle = !self.applying;
        let sel_before = self.app.session.sel.clone();
        self.app.ui(ui);
        ime::CANVAS_EDITING.store(self.app.canvas_editing(ui.ctx()), std::sync::atomic::Ordering::Relaxed);
        ime::QUIT.store(self.app.quit_requested, std::sync::atomic::Ordering::Relaxed);
        // The events handed over this frame are applied now, so the context shows them.
        self.applying = false;
        if idle && !self.pending.is_empty() && self.app.session.sel != sel_before {
            // A tap moved the caret in this frame, after these events were queued for the old caret.
            // Deletions and keys would hit other text, so only the typed text is kept (it lands at
            // the caret); the composition is dropped and its text stays in the document.
            self.pending.retain(|e| matches!(e, Event::Ime(ImeEvent::Commit(_))));
            self.held = None;
        }
        if self.pending.is_empty() {
            self.sync_held();
            ime::publish_context(self.context_line());
        }
        // egui sets `output.ime` while a text position is focused: that is when the keyboard shows.
        let wants = ui.ctx().output(|o| o.ime.is_some());
        ime::WANTS_KEYBOARD.store(wants, std::sync::atomic::Ordering::Relaxed);
        ime::CAN_GO_BACK.store(self.app.can_go_back(ui.ctx()), std::sync::atomic::Ordering::Relaxed);
        // egui-winit has no Android clipboard; copies (Copy, Cut, a text field's) go to ClipboardManager.
        let copies: Vec<String> = ui
            .ctx()
            .output(|o| o.commands.iter().filter_map(|c| if let egui::OutputCommand::CopyText(t) = c { Some(t.clone()) } else { None }).collect());
        if let Some(t) = copies.into_iter().last() {
            crate::clip::copied(t);
        }
    }
}
