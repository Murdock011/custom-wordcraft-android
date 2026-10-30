//! Phone layout tests: touch sheets, selection handles, long press and Android Back.

use super::*;

/// Phone long press selects the whole word under the finger, whichever half of a letter (or of
/// the last letter) it lands on: it used to drop the last letter ("peopl").
#[test]
fn phone_long_press_selects_whole_words() {
    let ctx = egui::Context::default();
    let mut a = app();
    let words = ["alpha", "people", "otherwise", "borrows"];
    a.run("text.insert", json!({"text": words.join(" ")})).unwrap();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 800.0));
    let mut t = 0.0;
    let mut frame = |a: &mut WordApp, events: Vec<egui::Event>| {
        t += 1.0 / 60.0;
        let input = egui::RawInput { events, time: Some(t), screen_rect: Some(screen), ..Default::default() };
        ctx.run_ui(input, |ui| {
            a.logic(ui.ctx());
            a.ui(ui);
        })
        .drop_without_applying_deltas();
    };
    let btn = |pos, pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
    for _ in 0..30 {
        frame(&mut a, Vec::new());
    }
    assert!(a.phone, "a 400 pt wide screen is a phone");
    let Some(page) = a.canvas.page_rects.first().copied() else { panic!("the canvas laid out") };
    let mut bad = Vec::new();
    let mut seen = 0;
    for yi in 0..12 {
        let y = page.top() + 10.0 * yi as f32 + 20.0;
        for xi in 0..40 {
            let at = egui::pos2(page.left() + 4.0 * xi as f32 + 4.0, y);
            a.session.sel = wordcraft_engine::Selection::caret(a.session.sel.focus.clone());
            frame(&mut a, vec![egui::Event::PointerMoved(at)]);
            frame(&mut a, vec![btn(at, true)]);
            for _ in 0..40 {
                frame(&mut a, Vec::new());
            }
            frame(&mut a, vec![btn(at, false)]);
            for _ in 0..3 {
                frame(&mut a, Vec::new());
            }
            let text = a.session.selected_text();
            if text.trim().is_empty() {
                continue;
            }
            seen += 1;
            if !words.contains(&text.trim()) {
                bad.push((at, text));
            }
        }
    }
    assert!(seen > 0, "some presses landed on text");
    assert!(bad.is_empty(), "partial words selected: {bad:?}");
}

/// A phone long press selects the whole word and opens the selection menu (Cut, Copy...).
#[test]
fn phone_long_press_opens_the_selection_menu() {
    let ctx = egui::Context::default();
    let mut a = app();
    a.run("text.insert", json!({"text": "alpha people otherwise borrows"})).unwrap();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 800.0));
    let mut t = 0.0;
    let mut frame = |a: &mut WordApp, events: Vec<egui::Event>| {
        t += 1.0 / 60.0;
        let input = egui::RawInput { events, time: Some(t), screen_rect: Some(screen), ..Default::default() };
        ctx.run_ui(input, |ui| {
            a.logic(ui.ctx());
            a.ui(ui);
        })
        .drop_without_applying_deltas();
    };
    let btn = |pos, pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
    for _ in 0..30 {
        frame(&mut a, Vec::new());
    }
    let Some(page) = a.canvas.page_rects.first().copied() else { panic!("the canvas laid out") };
    // Over the word "people" on the first line (about x 60..100 of the page).
    let at = egui::pos2(page.left() + 70.0, page.top() + 20.0);
    frame(&mut a, vec![egui::Event::PointerMoved(at)]);
    frame(&mut a, vec![btn(at, true)]);
    for _ in 0..40 {
        frame(&mut a, Vec::new());
    }
    frame(&mut a, vec![btn(at, false)]);
    for _ in 0..3 {
        frame(&mut a, Vec::new());
    }
    assert_eq!(a.session.selected_text().trim(), "people", "the long press selects the whole word");
    assert!(a.canvas.context_menu_open, "the long press opens the selection menu");
}

/// Phone frames for the tests below: a screen of `w` by 800 points, `frame` runs one frame.
fn phone_frames(w: f32) -> (egui::Context, impl FnMut(&mut WordApp, Vec<egui::Event>)) {
    let ctx = egui::Context::default();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, 800.0));
    let mut t = 0.0;
    let run = ctx.clone();
    let frame = move |a: &mut WordApp, events: Vec<egui::Event>| {
        t += 1.0 / 60.0;
        let input = egui::RawInput { events, time: Some(t), screen_rect: Some(screen), ..Default::default() };
        run.run_ui(input, |ui| {
            a.logic(ui.ctx());
            a.ui(ui);
        })
        .drop_without_applying_deltas();
    };
    (ctx, frame)
}

/// Update 22: on a phone a dialog is a full-width bottom sheet with 48 pt OK and Cancel buttons
/// pinned at its bottom, and Escape closes it.
#[test]
fn phone_dialog_is_a_sheet_with_48_pt_buttons() {
    let mut a = app();
    a.dialog = dialogs::Dialog::open("font", &mut a);
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(400.0, 800.0)).build_ui_state(
        |ui, app: &mut WordApp| {
            let ctx = ui.ctx().clone();
            app.logic(&ctx);
            app.ui(ui);
        },
        a,
    );
    h.run_steps(4);
    assert!(h.state().phone, "a 400 pt wide screen is a phone");
    let ok = h.get_by_label("OK").rect();
    let cancel = h.get_by_label("Cancel").rect();
    assert!(ok.height() >= 48.0 && cancel.height() >= 48.0, "buttons are 48 pt tall: {ok:?} {cancel:?}");
    assert!(cancel.right() <= 200.0 && ok.left() >= 200.0, "Cancel left, OK right, 50/50: {cancel:?} {ok:?}");
    assert!(ok.right() <= 400.0 && ok.bottom() <= 800.0, "the buttons are on screen: {ok:?}");
    assert!(ok.bottom() > 700.0, "pinned at the bottom of the sheet: {ok:?}");
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    assert!(h.state().dialog.is_none(), "Escape closes the sheet");
}

/// A phone press at `at`, held for `hold` frames (a long press holds about 0.4 s), then released.
fn phone_press(a: &mut WordApp, frame: &mut impl FnMut(&mut WordApp, Vec<egui::Event>), at: egui::Pos2, hold: usize) {
    let btn = |pos, pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
    frame(a, vec![egui::Event::PointerMoved(at)]);
    frame(a, vec![btn(at, true)]);
    for _ in 0..hold {
        frame(a, Vec::new());
    }
    frame(a, vec![btn(at, false)]);
    for _ in 0..3 {
        frame(a, Vec::new());
    }
}

/// Item 1: with a phone tab sheet open, a long press on the page behind it does nothing to the
/// text (no selection, no scroll). The press closes the sheet, as a tap outside it does.
#[test]
fn phone_tab_sheet_takes_the_page_touches() {
    let (ctx, mut frame) = phone_frames(400.0);
    let mut a = app();
    a.run("text.insert", json!({"text": "alpha people otherwise borrows"})).unwrap();
    for _ in 0..30 {
        frame(&mut a, Vec::new());
    }
    let page = a.canvas.page_rects.first().copied().expect("the canvas laid out");
    phone::open_menu(&mut a, &ctx, 0);
    frame(&mut a, Vec::new());
    assert!(phone::sheet_open(&ctx), "the tab sheet is open");
    let at = egui::pos2(page.left() + 70.0, page.top() + 20.0);
    assert!(at.y < 340.0, "the press is on the page, above the sheet: {at:?}");
    let before = a.session.sel.clone();
    let scroll = a.canvas.scroll_offset;
    phone_press(&mut a, &mut frame, at, 40);
    assert_eq!(a.session.sel, before, "the long press did not select anything");
    assert_eq!(a.canvas.scroll_offset, scroll, "the press did not scroll the page");
    assert!(!phone::sheet_open(&ctx), "a press outside the sheet closes it");
}

/// Item 1: a press on the sheet itself keeps the sheet open and leaves the page alone.
#[test]
fn phone_tab_sheet_keeps_touches_on_itself() {
    let (ctx, mut frame) = phone_frames(400.0);
    let mut a = app();
    a.run("text.insert", json!({"text": "alpha people otherwise borrows"})).unwrap();
    for _ in 0..30 {
        frame(&mut a, Vec::new());
    }
    phone::open_menu(&mut a, &ctx, 0);
    frame(&mut a, Vec::new());
    let before = a.session.sel.clone();
    // The sheet's top edge (its drag handle) is at 800 - 0.55 * 800 = 360.
    phone_press(&mut a, &mut frame, egui::pos2(200.0, 362.0), 40);
    assert!(phone::sheet_open(&ctx), "a press on the sheet keeps it open");
    assert_eq!(a.session.sel, before, "the page got no press");
}

/// Item 2: a tap outside the selection menu closes the menu only: the selection and its range
/// stay. With the menu closed, the next tap on the text places the caret.
#[test]
fn phone_tap_away_closes_the_menu_and_keeps_the_selection() {
    let (_ctx, mut frame) = phone_frames(400.0);
    let mut a = app();
    a.run("text.insert", json!({"text": "alpha people otherwise borrows"})).unwrap();
    for _ in 0..30 {
        frame(&mut a, Vec::new());
    }
    let page = a.canvas.page_rects.first().copied().expect("the canvas laid out");
    phone_press(&mut a, &mut frame, egui::pos2(page.left() + 70.0, page.top() + 20.0), 40);
    assert_eq!(a.session.selected_text().trim(), "people", "the long press selected the word");
    assert!(a.canvas.context_menu_open, "and opened the menu");
    let range = a.session.sel.clone();
    // A tap on "alpha", away from the selection and its menu.
    phone_press(&mut a, &mut frame, egui::pos2(page.left() + 10.0, page.top() + 20.0), 1);
    assert!(!a.canvas.context_menu_open, "the tap closed the menu");
    assert_eq!(a.session.sel, range, "the selection range is kept");
    assert_eq!(a.session.selected_text().trim(), "people");
    // The menu is closed now: a tap on the text places the caret. A second tap at once would
    // count as a double tap (egui's click count), so wait out the double-click time first.
    for _ in 0..60 {
        frame(&mut a, Vec::new());
    }
    phone_press(&mut a, &mut frame, egui::pos2(page.left() + 10.0, page.top() + 20.0), 1);
    assert!(a.session.sel.is_collapsed(), "a second tap places a caret: {:?} page {:?}", a.session.sel, page);
}

/// Update 23: the phone's formatting bar is hidden while a dialog sheet is open, and the sheet's
/// OK button is the topmost thing at its centre.
#[test]
fn phone_formatting_bar_stays_under_a_dialog_sheet() {
    let mut a = app();
    a.dialog = dialogs::Dialog::open("font", &mut a);
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(400.0, 800.0)).build_ui_state(
        |ui, app: &mut WordApp| {
            let ctx = ui.ctx().clone();
            app.logic(&ctx);
            app.ui(ui);
        },
        a,
    );
    h.run_steps(4);
    let ok = h.get_by_label("OK").rect();
    let top = h.ctx.layer_id_at(ok.center()).expect("something is under the OK button");
    assert_eq!(top.id, egui::Id::new(("dialog", "Font")), "the dialog sheet is on top at OK, not the formatting bar");
    assert!(!h.state().formatting_bar_shown(), "the formatting bar is hidden while the sheet is open");
    h.state_mut().dialog = None;
    h.run_steps(3);
    assert!(h.state().formatting_bar_shown(), "and comes back when the dialog closes");
}

/// Update 23: text fields in a phone dialog sheet are 48 pt tall (the Find dialog's boxes).
#[test]
fn phone_dialog_text_fields_are_48_pt_tall() {
    let mut a = app();
    a.dialog = dialogs::Dialog::open("find", &mut a);
    assert!(a.dialog.is_some(), "Find opens");
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(400.0, 800.0)).build_ui_state(
        |ui, app: &mut WordApp| {
            let ctx = ui.ctx().clone();
            app.logic(&ctx);
            app.ui(ui);
        },
        a,
    );
    h.run_steps(4);
    let fields: Vec<_> = h.query_all(egui_kittest::kittest::By::new().role(egui::accesskit::Role::TextInput)).map(|n| n.rect()).collect();
    assert!(!fields.is_empty(), "the Find dialog has text fields");
    for f in &fields {
        assert!(f.height() >= 48.0, "a text field is 48 pt tall on the phone: {f:?}");
    }
}

/// Update 24: the number boxes of a phone dialog (Insert Table's columns and rows) are 48 pt
/// tall, and the dialog's OK and Cancel are pinned in the footer.
#[test]
fn phone_dialog_number_boxes_are_48_pt_tall() {
    let mut a = app();
    a.dialog = dialogs::Dialog::open("insertTable", &mut a);
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(400.0, 800.0)).build_ui_state(
        |ui, app: &mut WordApp| {
            let ctx = ui.ctx().clone();
            app.logic(&ctx);
            app.ui(ui);
        },
        a,
    );
    h.run_steps(4);
    let boxes: Vec<_> = h.query_all(egui_kittest::kittest::By::new().role(egui::accesskit::Role::SpinButton)).map(|n| n.rect()).collect();
    let fields: Vec<_> = h.query_all(egui_kittest::kittest::By::new().role(egui::accesskit::Role::TextInput)).map(|n| n.rect()).collect();
    let all: Vec<_> = boxes.into_iter().chain(fields).collect();
    assert!(all.len() >= 2, "the Insert Table dialog has its number boxes: {all:?}");
    for f in &all {
        assert!(f.height() >= 48.0, "a number box is 48 pt tall on the phone: {f:?}");
    }
    assert!(h.get_by_label("OK").rect().height() >= 48.0);
}

/// Update 22: with the window short (the keyboard up), the pinned buttons stay on screen.
#[test]
fn phone_dialog_buttons_stay_on_a_short_screen() {
    let mut a = app();
    a.dialog = dialogs::Dialog::open("font", &mut a);
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(400.0, 420.0)).build_ui_state(
        |ui, app: &mut WordApp| {
            let ctx = ui.ctx().clone();
            app.logic(&ctx);
            app.ui(ui);
        },
        a,
    );
    h.run_steps(4);
    let ok = h.get_by_label("OK").rect();
    assert!(ok.height() >= 48.0 && ok.bottom() <= 420.0, "OK stays reachable: {ok:?}");
}

/// Update 22: on a desktop window a dialog keeps its floating window and desktop-sized buttons.
#[test]
fn desktop_dialog_keeps_its_window() {
    let mut a = app();
    a.dialog = dialogs::Dialog::open("font", &mut a);
    let mut h = harness(a);
    h.run_steps(4);
    assert!(!h.state().phone);
    let window = h.ctx.memory(|m| m.area_rect(egui::Id::new(("dialog", "Font")))).expect("the dialog is a window");
    assert!(window.width() < 1440.0 * 0.6, "a desktop dialog is not full width: {window:?}");
    assert!(h.get_by_label("OK").rect().height() < 40.0, "desktop buttons keep their size");
}

/// Native Android keeps the platform double-click delay, so a double-tap selects the word.
/// (The web phone sets it to 0 because a WebView tap can arrive as two presses.)
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn phone_double_tap_selects_a_word_on_native() {
    let mut a = app();
    a.run("text.insert", json!({"text": "alpha people otherwise borrows"})).unwrap();
    let (ctx, mut frame) = phone_frames(400.0);
    for _ in 0..30 {
        frame(&mut a, Vec::new());
    }
    assert!(a.phone, "a 400 pt wide screen is a phone");
    let delay = ctx.options(|o| o.input_options.max_double_click_delay);
    assert_eq!(delay, egui::InputOptions::default().max_double_click_delay, "the native phone keeps egui's delay");
    let Some(page) = a.canvas.page_rects.first().copied() else { panic!("the canvas laid out") };
    let at = egui::pos2(page.left() + 70.0, page.top() + 20.0);
    let btn = |pos, pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
    frame(&mut a, vec![egui::Event::PointerMoved(at)]);
    for _ in 0..2 {
        frame(&mut a, vec![btn(at, true)]);
        frame(&mut a, vec![btn(at, false)]);
        for _ in 0..3 {
            frame(&mut a, Vec::new());
        }
    }
    for _ in 0..10 {
        frame(&mut a, Vec::new());
    }
    assert_eq!(a.session.selected_text().trim(), "people", "two quick taps select the word");
}

/// The selection handles exist only for a non-empty selection on the phone.
#[test]
fn phone_selection_handles_only_for_a_selection() {
    for (w, phone) in [(400.0_f32, true), (1200.0, false)] {
        let mut a = app();
        a.run("text.insert", json!({"text": "alpha people otherwise borrows"})).unwrap();
        let (_, mut frame) = phone_frames(w);
        for _ in 0..30 {
            frame(&mut a, Vec::new());
        }
        assert_eq!(a.phone, phone);
        assert!(a.canvas.sel_handles.is_none(), "a caret has no handles");
        a.run("select.text", json!({"text": "people"})).unwrap();
        for _ in 0..3 {
            frame(&mut a, Vec::new());
        }
        assert_eq!(a.canvas.sel_handles.is_some(), phone, "handles on the phone only, for a selection");
    }
}

/// Escape with the phone selection menu open (Android Back sends Escape) closes the menu and
/// keeps the selection, as Word does. Escape with nothing open collapses it.
#[test]
fn phone_escape_with_the_menu_open_keeps_the_selection() {
    let mut a = app();
    a.run("text.insert", json!({"text": "alpha people otherwise borrows"})).unwrap();
    let (_, mut frame) = phone_frames(400.0);
    for _ in 0..30 {
        frame(&mut a, Vec::new());
    }
    a.run("select.text", json!({"text": "people"})).unwrap();
    a.canvas.context_menu_open = true;
    for _ in 0..3 {
        frame(&mut a, Vec::new());
    }
    assert!(a.canvas.context_menu_open, "the menu is open");
    let esc = egui::Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() };
    frame(&mut a, vec![esc]);
    for _ in 0..3 {
        frame(&mut a, Vec::new());
    }
    assert!(!a.canvas.context_menu_open, "Escape closes the menu");
    assert_eq!(a.session.selected_text(), "people", "the selection range is unchanged");
}

/// With nothing open, Back has a selection to close: it is Escape, so the selection collapses
/// and the app stays open (Back with nothing to close sends the app to the background).
#[test]
fn phone_back_with_only_a_selection_collapses_it() {
    let mut a = app();
    a.run("text.insert", json!({"text": "alpha people otherwise borrows"})).unwrap();
    let (ctx, mut frame) = phone_frames(400.0);
    for _ in 0..30 {
        frame(&mut a, Vec::new());
    }
    a.run("select.text", json!({"text": "people"})).unwrap();
    for _ in 0..3 {
        frame(&mut a, Vec::new());
    }
    assert!(a.can_go_back(&ctx), "a selection is something Back can close");
    let esc = egui::Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() };
    frame(&mut a, vec![esc]);
    for _ in 0..3 {
        frame(&mut a, Vec::new());
    }
    assert!(a.session.sel.is_collapsed(), "Back collapses the selection to a caret");
    assert!(!a.can_go_back(&ctx), "nothing left to close");
}

/// A status message (a failed open, say) is drawn on the phone, where there is no status bar.
#[test]
fn phone_status_message_is_drawn() {
    fn drawn(a: &mut WordApp, ctx: &egui::Context, msg: &str) -> bool {
        let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 800.0))), ..Default::default() };
        let mut out = ctx.run_ui(input, |ui| {
            a.logic(ui.ctx());
            a.ui(ui);
        });
        out.textures_delta.clear();
        fn has(shape: &egui::Shape, msg: &str) -> bool {
            match shape {
                egui::Shape::Text(t) => t.galley.job.text.contains(msg),
                egui::Shape::Vec(v) => v.iter().any(|s| has(s, msg)),
                _ => false,
            }
        }
        out.shapes.iter().any(|c| has(&c.shape, msg))
    }
    let mut a = app();
    a.run("text.insert", json!({"text": "Body"})).unwrap();
    let ctx = egui::Context::default();
    for _ in 0..30 {
        drawn(&mut a, &ctx, "");
    }
    a.status("Couldn't open quokka.docx: it was moved or deleted.");
    // An Area is laid out before it is painted: the first frame it appears in is a sizing pass.
    drawn(&mut a, &ctx, "");
    assert!(drawn(&mut a, &ctx, "Couldn't open quokka.docx"), "the phone shows the status message");
    // After its time the strip goes away.
    a.status_msg = Some(("Old notice".into(), crate::now_ms() - 60_000.0));
    assert!(!drawn(&mut a, &ctx, "Old notice"), "an old message is not kept on screen");
}

/// Dragging the end handle grows the selection; the start stays where it was.
#[test]
fn phone_end_handle_drag_extends_the_selection() {
    let mut a = app();
    a.run("text.insert", json!({"text": "alpha people otherwise borrows"})).unwrap();
    let (_, mut frame) = phone_frames(400.0);
    for _ in 0..30 {
        frame(&mut a, Vec::new());
    }
    a.run("select.text", json!({"text": "people"})).unwrap();
    for _ in 0..3 {
        frame(&mut a, Vec::new());
    }
    let start = a.session.sel.ordered().0;
    let Some([_, end]) = a.canvas.sel_handles else { panic!("the selection has handles") };
    let btn = |pos, pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
    let grab = end + egui::vec2(0.0, -2.0);
    frame(&mut a, vec![egui::Event::PointerMoved(grab)]);
    frame(&mut a, vec![btn(grab, true)]);
    let before = a.session.selected_text().len();
    let mut at = grab;
    for _ in 0..12 {
        at += egui::vec2(8.0, 0.0);
        frame(&mut a, vec![egui::Event::PointerMoved(at)]);
    }
    frame(&mut a, vec![btn(at, false)]);
    for _ in 0..3 {
        frame(&mut a, Vec::new());
    }
    assert!(a.session.selected_text().len() > before, "the selection grew: {:?}", a.session.selected_text());
    assert!(a.session.selected_text().starts_with("people"), "the start stayed put");
    assert_eq!(a.session.sel.ordered().0, start, "the anchor is the start");
}

/// Dragging a handle on the phone in Web Layout never scrolls the page sideways: the text fits
/// the width, so the horizontal offset stays where it was (the page moved about 18 px in Update 13).
#[test]
fn phone_handle_drag_does_not_scroll_the_page_sideways() {
    for mode in [wordcraft_layout::ViewMode::Web, wordcraft_layout::ViewMode::Print] {
        let mut a = app();
        a.run("text.insert", json!({"text": "alpha people otherwise borrows ".repeat(5)})).unwrap();
        let (_, mut frame) = phone_frames(400.0);
        for _ in 0..30 {
            frame(&mut a, Vec::new());
        }
        a.session.view.mode = mode;
        for _ in 0..30 {
            frame(&mut a, Vec::new());
        }
        a.run("select.text", json!({"text": "otherwise"})).unwrap();
        for _ in 0..3 {
            frame(&mut a, Vec::new());
        }
        let x0 = a.canvas.scroll_offset.x;
        let Some([start_handle, _]) = a.canvas.sel_handles else { panic!("the selection has handles") };
        let btn = |pos, pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
        frame(&mut a, vec![egui::Event::PointerMoved(start_handle)]);
        frame(&mut a, vec![btn(start_handle, true)]);
        let mut at = start_handle;
        let mut worst = 0.0_f32;
        for _ in 0..60 {
            if a.session.selected_text().starts_with("people") {
                break;
            }
            at += egui::vec2(-2.0, 0.0);
            frame(&mut a, vec![egui::Event::PointerMoved(at)]);
            worst = worst.max((a.canvas.scroll_offset.x - x0).abs());
        }
        frame(&mut a, vec![btn(at, false)]);
        for _ in 0..3 {
            frame(&mut a, Vec::new());
        }
        assert!(a.session.selected_text().starts_with("people"), "{mode:?}: the drag moved the start");
        assert!(worst < 0.5, "{mode:?}: the page scrolled sideways by {worst} during the drag");
    }
}

/// Dragging the start handle left grows the selection backwards; the end stays where it was.
#[test]
fn phone_start_handle_drag_extends_the_selection_backwards() {
    let mut a = app();
    a.run("text.insert", json!({"text": "alpha people otherwise borrows ".repeat(5)})).unwrap();
    let (_, mut frame) = phone_frames(400.0);
    for _ in 0..30 {
        frame(&mut a, Vec::new());
    }
    a.run("select.text", json!({"text": "otherwise"})).unwrap();
    for _ in 0..3 {
        frame(&mut a, Vec::new());
    }
    // The selection's line has a line below it, so the handle's centre sits over the next line.
    let end = a.session.sel.ordered().1;
    let Some([start_handle, _]) = a.canvas.sel_handles else { panic!("the selection has handles") };
    let btn = |pos, pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
    // Press on the handle's centre, as a finger does: it sits below the line it marks.
    let grab = start_handle;
    frame(&mut a, vec![egui::Event::PointerMoved(grab)]);
    frame(&mut a, vec![btn(grab, true)]);
    let mut at = grab;
    for _ in 0..60 {
        if a.session.selected_text().starts_with("people") {
            break;
        }
        at += egui::vec2(-2.0, 0.0);
        frame(&mut a, vec![egui::Event::PointerMoved(at)]);
    }
    frame(&mut a, vec![btn(at, false)]);
    for _ in 0..3 {
        frame(&mut a, Vec::new());
    }
    let text = a.session.selected_text();
    assert!(text.starts_with("people"), "the start moved back a word: {text:?}");
    assert!(text.ends_with("otherwise"), "the end stayed put: {text:?}");
    assert_eq!(a.session.sel.ordered().1, end, "the end position is unchanged");
}

/// A press a few points off the handle's centre, held still, does not move the selection.
#[test]
fn phone_handle_press_off_centre_does_not_move_the_selection() {
    let mut a = app();
    a.run("text.insert", json!({"text": "alpha people otherwise borrows"})).unwrap();
    let (_, mut frame) = phone_frames(400.0);
    for _ in 0..30 {
        frame(&mut a, Vec::new());
    }
    a.run("select.text", json!({"text": "people"})).unwrap();
    for _ in 0..3 {
        frame(&mut a, Vec::new());
    }
    let before = a.session.sel.clone();
    let Some([start, end]) = a.canvas.sel_handles else { panic!("the selection has handles") };
    let btn = |pos, pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
    for handle in [start, end] {
        let off = handle + egui::vec2(4.0, 4.0);
        frame(&mut a, vec![egui::Event::PointerMoved(off)]);
        frame(&mut a, vec![btn(off, true)]);
        for _ in 0..5 {
            frame(&mut a, Vec::new());
        }
        frame(&mut a, vec![btn(off, false)]);
        for _ in 0..3 {
            frame(&mut a, Vec::new());
        }
        assert_eq!(a.session.sel.ordered(), before.ordered(), "a press off the handle's centre moves nothing");
    }
}

/// Home with nothing open (the Start screen, or after the document is closed) has no document to
/// go back to: Back, and the back arrow, make a blank document as Blank document does, not an
/// empty window.
#[test]
fn phone_back_from_home_with_nothing_open_makes_a_blank_document() {
    let mut a = app();
    a.start_screen();
    let (_ctx, mut frame) = phone_frames(400.0);
    for _ in 0..30 {
        frame(&mut a, Vec::new());
    }
    assert!(a.ui.backstage && !a.has_document(), "Home with nothing open");
    let esc = egui::Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() };
    frame(&mut a, vec![esc]);
    for _ in 0..3 {
        frame(&mut a, Vec::new());
    }
    assert!(!a.ui.backstage, "Back leaves Home");
    assert!(a.has_document(), "Back makes a blank document, not an empty window");
    assert_eq!(a.session.doc.plain_text(wordcraft_doc::StoryRef::Body), "", "the blank document is empty");

    // The back arrow at the top right of the phone's app bar does the same.
    let mut b = app();
    b.start_screen();
    let (_ctx, mut frame) = phone_frames(400.0);
    for _ in 0..30 {
        frame(&mut b, Vec::new());
    }
    phone_press(&mut b, &mut frame, egui::pos2(370.0, 28.0), 3);
    assert!(!b.ui.backstage, "the back arrow leaves Home");
    assert!(b.has_document(), "the back arrow makes a blank document");
}

/// Back from the Recover page with nothing open makes a blank document too, not an empty window.
#[test]
fn phone_back_from_recover_with_nothing_open_makes_a_blank_document() {
    let mut a = app();
    a.start_screen();
    a.ui.backstage_page = "recover".into();
    let (_ctx, mut frame) = phone_frames(400.0);
    for _ in 0..30 {
        frame(&mut a, Vec::new());
    }
    assert!(a.ui.backstage && !a.has_document(), "Recover with nothing open");
    let esc = egui::Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() };
    frame(&mut a, vec![esc]);
    for _ in 0..3 {
        frame(&mut a, Vec::new());
    }
    assert!(!a.ui.backstage, "Back leaves the Backstage");
    assert!(a.has_document(), "Back makes a blank document, not an empty window");
    assert_eq!(a.session.doc.plain_text(wordcraft_doc::StoryRef::Body), "", "the blank document is empty");
}

/// With a document open, Back from Home returns to that document unchanged.
#[test]
fn phone_back_from_home_with_a_document_keeps_it() {
    let mut a = WordApp::new(Session::new(wordcraft_doc::Document::from_text("Work")), Services::default());
    a.run("ui.backstage", json!({"value": true, "page": "home"})).unwrap();
    let (_ctx, mut frame) = phone_frames(400.0);
    for _ in 0..30 {
        frame(&mut a, Vec::new());
    }
    let esc = egui::Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() };
    frame(&mut a, vec![esc]);
    for _ in 0..3 {
        frame(&mut a, Vec::new());
    }
    assert!(!a.ui.backstage, "Back returns to the editor");
    assert_eq!(a.session.doc.plain_text(wordcraft_doc::StoryRef::Body), "Work", "the open document is kept");
}
