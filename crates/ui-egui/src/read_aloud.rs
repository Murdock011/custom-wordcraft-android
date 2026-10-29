//! The Read Aloud player: previous / play-pause / next / stop, a speed slider and "Skip citations &
//! bibliography". Every control runs a `readAloud.*` command; the player itself is
//! [`wordcraft_engine::speech::ReadAloud`].
//!
//! On the desktop it is a small floating window. On the phone it is a bottom sheet (Update 27):
//! the four transport controls are 48 pt touch targets spread across the width, and the sheet has
//! no action row (closing it stops reading, as the window's close box does).
//!
//! While reading, the caret follows the sentence being spoken (so stopping leaves it where
//! reading stopped) and the sentence is highlighted on the page.

use egui::{Color32, Rect, Sense, Ui, Vec2, vec2};
use serde_json::json;
use wordcraft_engine::speech::{MAX_RATE, MIN_RATE, State};

use crate::WordApp;
use crate::theme::Tokens;

/// A phone touch target (Android's 48 pt minimum).
const TOUCH_H: f32 = crate::phone::TOUCH_H;

/// Per-frame bookkeeping: follow the spoken sentence with the caret, report errors, and keep
/// frames coming while reading.
pub fn poll(app: &mut WordApp, ctx: &egui::Context) {
    let ra = &app.session.read_aloud;
    if !ra.open {
        app.read_aloud_at = None;
        return;
    }
    if ra.state() == State::Playing {
        ctx.request_repaint_after(std::time::Duration::from_millis(120));
    }
    if let Some(e) = app.session.read_aloud.error()
        && app.read_aloud_error.as_deref() != Some(e.as_str())
    {
        app.read_aloud_error = Some(e.clone());
        app.status(e);
    }
    // The caret follows the reading only in the document being read.
    let current = if reads_shown_document(app) { app.session.read_aloud.current().map(|u| u.start) } else { None };
    if current != app.read_aloud_at {
        if let Some(start) = &current {
            app.session.sel = wordcraft_engine::Selection::caret(app.session.doc.clamp(start));
            app.session.goal_x = None;
            app.canvas.scroll_to_caret = true;
        }
        app.read_aloud_at = current;
    }
}

/// The page rectangles (in page units) of the sentence being read, to highlight.
pub fn highlight(app: &mut WordApp) -> Option<(wordcraft_engine::doc::Pos, wordcraft_engine::doc::Pos)> {
    if !app.session.read_aloud.open || !reads_shown_document(app) {
        return None;
    }
    let u = app.session.read_aloud.current()?;
    Some((app.session.doc.clamp(&u.start), app.session.doc.clamp(&u.end)))
}

/// Whether the reading is of the document the window shows: its sentence positions are in it.
fn reads_shown_document(app: &WordApp) -> bool {
    app.session.read_aloud.document == Some(app.session.document_id())
}

/// The player: a bottom sheet on the phone, a floating window on the desktop.
pub fn show(app: &mut WordApp, ctx: &egui::Context) {
    if !app.session.read_aloud.open {
        return;
    }
    // With a window per document, the player is in the window of the document being read.
    if app.windows.native()
        && let Some(d) = app.session.read_aloud.document
        && d != app.session.document_id()
        && app.session.is_open(d)
    {
        return;
    }
    let t = Tokens::get(ctx);
    let mut run: Option<(&'static str, serde_json::Value)> = None;
    let close = if app.phone {
        let (close, _) = crate::phone::dialog_sheet(ctx, egui::Id::new("read-aloud-player"), "Read Aloud", |ui| {
            controls(app, ui, &t, true, &mut run);
            false
        });
        close
    } else {
        let right_top = app.canvas.canvas_rect.map_or(ctx.content_rect().right_top() + vec2(-24.0, 160.0), |r| r.right_top() + vec2(-24.0, 16.0));
        let window = egui::Window::new(tl!("Read Aloud"))
            .id(egui::Id::unique("read-aloud-player"))
            .collapsible(false)
            .resizable(false)
            .pivot(egui::Align2::RIGHT_TOP)
            .default_pos(right_top);
        let shown = crate::window_close::window(ctx, window).show(ctx, |ui| {
            ui.set_width(236.0);
            controls(app, ui, &t, false, &mut run);
        });
        shown.is_some_and(|s| crate::window_close::show(ctx, &s.response, &t))
    };
    if close {
        run = Some(("readAloud.stop", json!({})));
    }
    if let Some((id, params)) = run {
        let _ = app.run(id, params);
        app.ui.read_aloud_rate = app.session.read_aloud.rate();
        app.ui.read_aloud_skip_citations = app.session.read_aloud.skip_citations;
        ctx.request_repaint();
    }
}

/// The player's controls. `phone` lays the transport buttons out as 48 pt targets across the width,
/// with the status line under them; the desktop keeps the compact row.
fn controls(app: &WordApp, ui: &mut Ui, t: &Tokens, phone: bool, run: &mut Option<(&'static str, serde_json::Value)>) {
    let state = app.session.read_aloud.state();
    let (index, count) = app.session.read_aloud.progress();
    let what = match state {
        State::Playing => format!("{} {} / {}", tl!("Sentence"), (index + 1).min(count), count),
        State::Paused => tl!("Paused").to_string(),
        State::Stopped if count > 0 && index >= count => tl!("Finished").to_string(),
        State::Stopped => tl!("Stopped").to_string(),
    };
    let (play_icon, play_tip) = if state == State::Playing { ("mediaPause", tl!("Pause")) } else { ("mediaPlay", tl!("Play")) };
    let steps: [(&str, &str, bool, &'static str); 4] = [
        ("mediaPrev", tl!("Previous sentence"), count > 0 && index > 0, "readAloud.previous"),
        (play_icon, play_tip, true, "readAloud.playPause"),
        ("mediaNext", tl!("Next sentence"), index + 1 < count, "readAloud.next"),
        ("mediaStop", tl!("Stop"), true, "readAloud.stop"),
    ];
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = if phone { 8.0 } else { 6.0 };
        // Phone: four equal touch targets across the sheet. Desktop: 30 pt icon buttons.
        let size: Vec2 = if phone { vec2(((ui.available_width() - 24.0) / 4.0).max(TOUCH_H), TOUCH_H) } else { vec2(30.0, 30.0) };
        let icon_px = if phone { 24.0 } else { 20.0 };
        for (icon, tip, enabled, command) in steps {
            if button(ui, t, icon, tip, enabled, size, icon_px) {
                *run = Some((command, json!({})));
            }
        }
        if !phone {
            ui.add_space(4.0);
            ui.label(egui::RichText::new(what.clone()).color(t.text_dim).small());
        }
    });
    if phone {
        ui.label(egui::RichText::new(what).color(t.text_dim));
    }
    ui.add_space(6.0);
    // Speed: applied when the slider is let go, so dragging doesn't restart the sentence
    // on every frame.
    let id = egui::Id::new("read-aloud-rate");
    let mut rate = ui.data(|d| d.get_temp::<f32>(id)).unwrap_or_else(|| app.session.read_aloud.rate());
    ui.horizontal(|ui| {
        let speed = ui.label(tl!("Speed"));
        ui.spacing_mut().slider_width = if phone { (ui.available_width() - 124.0).max(110.0) } else { 110.0 };
        let r = ui
            .add(
                egui::Slider::new(&mut rate, MIN_RATE..=MAX_RATE)
                    .step_by(0.05)
                    .trailing_fill(true)
                    .custom_formatter(|v, _| format!("{v:.2}×"))
                    .custom_parser(|s| s.trim().trim_end_matches(['×', 'x']).trim().parse().ok()),
            )
            .labelled_by(speed.id);
        if r.dragged() {
            ui.data_mut(|d| d.insert_temp(id, rate));
        } else {
            ui.data_mut(|d| d.remove::<f32>(id));
            if r.changed() || r.drag_stopped() {
                *run = Some(("readAloud.speed", json!({"value": rate})));
            }
        }
        let normal = if phone { ui.add(egui::Button::new("1×").min_size(vec2(48.0, TOUCH_H))) } else { ui.small_button("1×") };
        if normal.on_hover_text(tl!("Normal speed")).clicked() {
            *run = Some(("readAloud.speed", json!({"value": 1.0})));
        }
    });
    let mut skip = app.session.read_aloud.skip_citations;
    if ui
        .checkbox(&mut skip, tl!("Skip citations & bibliography"))
        .on_hover_text(tl!("Don't read Zotero, Mendeley or EndNote citations and bibliographies"))
        .changed()
    {
        *run = Some(("readAloud.skipCitations", json!({"value": skip})));
    }
    // The error `poll` saw this frame, not the worker's latest: the player thread can set
    // one between the two, and the window would show a message the status bar hasn't.
    if let Some(e) = app.read_aloud_error.as_ref().filter(|e| app.session.read_aloud.error().as_ref() == Some(*e)) {
        ui.label(egui::RichText::new(e.as_str()).color(t.red).small());
    }
}

/// A transport button with a drawn icon and a tooltip; returns whether it was clicked.
fn button(ui: &mut Ui, t: &Tokens, icon: &str, tip: &str, enabled: bool, size: Vec2, icon_px: f32) -> bool {
    let (rect, resp) = ui.allocate_exact_size(size, if enabled { Sense::click() } else { Sense::hover() });
    let resp = resp.on_hover_text(tip);
    if enabled && resp.hovered() {
        ui.painter().rect_filled(rect, 4.0, if resp.is_pointer_button_down_on() { t.pressed } else { t.hover });
    }
    let c = if enabled { t.icon } else { t.text_disabled };
    let r = Rect::from_center_size(rect.center(), vec2(icon_px, icon_px));
    crate::icons::paint(ui.painter(), r, icon, c, if enabled { t.accent } else { Color32::TRANSPARENT });
    enabled && resp.clicked()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Services;

    /// Update 27: the Read Aloud player is a full-width sheet on the phone (closing it stops
    /// reading) and a floating window on the desktop.
    #[test]
    fn read_aloud_is_a_sheet_on_the_phone_and_a_window_on_desktop() {
        for (w, h, phone) in [(400.0, 800.0, true), (1440.0, 900.0, false)] {
            let mut a = WordApp::new(wordcraft_engine::Session::new(wordcraft_doc::Document::new()), Services::default());
            a.session.read_aloud.open = true;
            let mut harness = egui_kittest::Harness::builder().with_size(egui::vec2(w, h)).build_ui_state(
                |ui, app: &mut WordApp| {
                    let ctx = ui.ctx().clone();
                    app.logic(&ctx);
                    app.ui(ui);
                },
                a,
            );
            harness.run_steps(4);
            assert_eq!(harness.state().phone, phone, "a {w} pt screen");
            let area = harness.ctx.memory(|m| m.area_rect(egui::Id::new("read-aloud-player"))).expect("the player is drawn");
            if phone {
                assert!((area.width() - w).abs() < 1.0, "the sheet is the full width: {area:?}");
                assert!(area.bottom() <= h + 1.0 && area.top() > h / 2.0, "a bottom sheet: {area:?}");
                harness.key_press(egui::Key::Escape);
                harness.run_steps(3);
                assert!(!harness.state().session.read_aloud.open, "closing the phone sheet stops reading");
            } else {
                assert!(area.width() < 400.0, "a floating window, not full width: {area:?}");
            }
        }
    }
}
