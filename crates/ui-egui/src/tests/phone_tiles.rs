//! The phone's Tiles style (the default) and the List style option: the tab bar, the tile sheets and
//! the More sheet, and the setting that picks the style.

use egui_kittest::kittest::{NodeT, Queryable};

use super::*;

type H = egui_kittest::Harness<'static, WordApp>;

fn harness(list: bool) -> H {
    let mut a = app();
    a.ui.phone_list_style = list;
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(412.0, 915.0)).build_ui_state(
        |ui, app: &mut WordApp| {
            let ctx = ui.ctx().clone();
            app.logic(&ctx);
            app.ui(ui);
        },
        a,
    );
    h.run_steps(4);
    h
}

fn tap(h: &mut H, label: &str) {
    h.get_by_role_and_label(egui::accesskit::Role::Button, label).click();
    h.run_steps(3);
}

/// Scrolls the open sheet down by a wheel over its middle.
fn scroll_sheet(h: &mut H) {
    let at = egui::pos2(206.0, 700.0);
    h.input_mut().events.push(egui::Event::PointerMoved(at));
    h.run_steps(2);
    h.input_mut().events.push(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, -400.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
        modifiers: egui::Modifiers::NONE,
    });
    h.run_steps(30);
}

fn has_button(h: &H, label: &str) -> bool {
    h.query_all_by_role_and_label(egui::accesskit::Role::Button, label).next().is_some()
}

/// The default is the Tiles style: a tab bar of Home, Insert, Layout, Review, View and More, at
/// least 48 pt tall, and no floating bar.
#[test]
fn phone_tiles_is_the_default_with_a_tab_bar() {
    assert!(!app().ui.phone_list_style, "Tiles is the default");
    let h = harness(false);
    for tab in ["Home", "Insert", "Layout", "Review", "View", "More"] {
        let node = h.get_by_role_and_label(egui::accesskit::Role::Button, tab);
        assert!(node.rect().height() >= 48.0, "{tab} is a 48 pt target");
    }
    assert!(!phone::sheet_open(&h.ctx), "no sheet is open at the start");
}

/// A tab opens its tile sheet; a formatting tile runs its command and the sheet stays open; the open
/// tab tapped again closes the sheet.
#[test]
fn phone_tab_opens_tiles_that_run_commands() {
    let mut h = harness(false);
    h.state_mut().run("text.insert", json!({"text": "alpha"})).unwrap();
    h.state_mut().run("select.all", json!({})).unwrap();
    h.run_steps(2);
    tap(&mut h, "Home");
    assert!(phone::sheet_open(&h.ctx), "Home opens its sheet");
    tap(&mut h, "Bold");
    let st = h.state_mut().session.run("format.state", &json!({})).unwrap();
    assert_eq!(st.get("bold"), Some(&json!(true)), "the Bold tile made the text bold");
    assert!(phone::sheet_open(&h.ctx), "a formatting tile keeps the sheet open");
    tap(&mut h, "Home");
    assert!(!phone::sheet_open(&h.ctx), "the open tab tapped again closes the sheet");
}

/// The tab bar switches sheets, and the Layout tiles open their choice pages in the sheet.
#[test]
fn phone_tab_bar_switches_sheets() {
    let mut h = harness(false);
    tap(&mut h, "Insert");
    assert!(has_button(&h, "Page Break") || has_button(&h, "Blank Page"), "Insert shows its tiles");
    tap(&mut h, "Layout");
    assert!(has_button(&h, "Margins"), "Layout shows its tiles");
    tap(&mut h, "More");
    assert!(has_button(&h, "Search"), "More has Search");
    assert!(has_button(&h, "Draw"), "More has Draw");
}

/// The List style keeps the floating bar with More, and no tab bar.
#[test]
fn phone_list_style_keeps_the_floating_bar() {
    let mut h = harness(true);
    assert!(!has_button(&h, "Insert"), "no tab bar in the List style");
    tap(&mut h, "More");
    assert!(phone::sheet_open(&h.ctx), "More opens the list sheet");
}

/// `ui.phoneSheetStyle` picks the style, and rejects an unknown one.
#[test]
fn phone_sheet_style_command_switches_styles() {
    let mut a = app();
    assert_eq!(a.run("ui.phoneSheetStyle", json!({"style": "list"})).unwrap(), json!({"style": "list"}));
    assert!(a.ui.phone_list_style);
    assert_eq!(a.run("ui.phoneSheetStyle", json!({})).unwrap(), json!({"style": "list"}), "no style reads it");
    a.run("ui.phoneSheetStyle", json!({"style": "tiles"})).unwrap();
    assert!(!a.ui.phone_list_style);
    assert!(a.run("ui.phoneSheetStyle", json!({"style": "grid"})).is_err());
}

/// The top bar: File, the name with ` •` once there are unsaved changes, then Undo, Redo and Save.
/// Undo is disabled with nothing to undo. Search is not in the top bar; More has it.
#[test]
fn phone_top_bar_has_save_and_a_changed_mark() {
    let mut h = harness(false);
    let undo = h.get_by_role_and_label(egui::accesskit::Role::Button, "Undo");
    assert!(undo.accesskit_node().is_disabled(), "nothing to undo yet");
    assert!(has_button(&h, "Save") && has_button(&h, "Redo") && has_button(&h, "File"));
    assert!(!has_button(&h, "Search"), "Search is in More, not the top bar");
    h.state_mut().run("text.insert", json!({"text": "alpha"})).unwrap();
    h.run_steps(3);
    let stem = h.state().title_stem();
    assert!(h.query_all_by_label(&format!("{stem} •")).next().is_some(), "the name shows the changed mark");
    assert!(!h.get_by_role_and_label(egui::accesskit::Role::Button, "Undo").accesskit_node().is_disabled(), "Undo is enabled after an edit");
}

/// Android starts Light when no theme is saved (no file, or a file without one); a saved theme is
/// kept. The desktop default stays System.
#[test]
fn android_settings_start_light() {
    use crate::theme::Appearance;
    assert_eq!(UiState::default().theme, Appearance::System, "the desktop default is unchanged");
    assert_eq!(UiState::android(None).theme, Appearance::Light);
    assert_eq!(UiState::android(Some(br#"{"recentCount": 5}"#)).theme, Appearance::Light);
    assert_eq!(UiState::android(Some(br#"{"theme": "dark"}"#)).theme, Appearance::Dark);
    assert_eq!(UiState::android(Some(br#"{"theme": "system"}"#)).theme, Appearance::System);
    assert_eq!(UiState::android(Some(b"not json")).theme, Appearance::Light);
}

/// Dark is app wide on a phone: the dark theme turns the page dark, and Light turns it back.
#[test]
fn phone_dark_theme_darkens_the_page() {
    let mut h = harness(false);
    assert!(!h.state().session.view.dark_mode, "light theme, light page");
    h.state_mut().run("ui.theme", json!({"value": "dark"})).unwrap();
    h.run_steps(4);
    assert!(h.state().session.view.dark_mode, "the dark theme darkens the page");
    h.state_mut().run("ui.theme", json!({"value": "light"})).unwrap();
    h.run_steps(4);
    assert!(!h.state().session.view.dark_mode, "Light turns the page back");
}

/// More has Search and All tools. All tools lists the ribbon tabs; a tab lists its commands by group;
/// a command that needs no values runs and closes the tools. Back steps out one level at a time.
#[test]
fn phone_all_tools_lists_tabs_and_runs_commands() {
    let mut h = harness(false);
    h.state_mut().run("text.insert", json!({"text": "alpha"})).unwrap();
    h.state_mut().run("select.all", json!({})).unwrap();
    h.run_steps(2);
    tap(&mut h, "More");
    tap(&mut h, "All tools");
    assert!(h.query_all_by_label_contains("Home  (").next().is_some(), "the Home tab is listed with its count");
    h.query_all_by_label_contains("Home  (").next().expect("the Home row").click();
    h.run_steps(3);
    assert!(phone::sheet_open(&h.ctx));
    h.get_by_role_and_label(egui::accesskit::Role::Button, "Back").click();
    h.run_steps(3);
    assert!(h.query_all_by_label_contains("Home  (").next().is_some(), "Back returns to the list of tabs");
    h.get_by_role_and_label(egui::accesskit::Role::Button, "Back").click();
    h.run_steps(3);
    assert!(has_button(&h, "All tools"), "Back from the tabs returns to More");
}

/// Search Commands on a phone opens All tools' Search, and finds a command by name.
#[test]
fn phone_search_commands_opens_all_tools_search() {
    let mut h = harness(false);
    h.state_mut().run("text.insert", json!({"text": "alpha"})).unwrap();
    h.state_mut().run("select.all", json!({})).unwrap();
    h.run_steps(3);
    assert_eq!(h.state_mut().run("ui.dialog", json!({"name": "commands"})).unwrap(), json!({"dialog": "allTools"}));
    h.run_steps(3);
    assert!(h.state().dialog.is_none(), "no Search Commands dialog on a phone");
    let field = h.get_by_role_and_label(egui::accesskit::Role::TextInput, "Search commands");
    field.type_text("italic");
    h.run_steps(4);
    tap(&mut h, "Italic");
    let st = h.state_mut().session.run("format.state", &json!({})).unwrap();
    assert_eq!(st.get("italic"), Some(&json!(true)), "the found command ran");
    assert!(!phone::sheet_open(&h.ctx), "running a command closes the tools");
}

/// While the keyboard is up (the window shrinks), the Tiles style hides the tab bar and shows the
/// keyboard bar: formats, lists, New Comment, Link, Undo, and Hide Keyboard at the right end.
#[test]
fn phone_keyboard_bar_replaces_the_tab_bar_while_typing() {
    let mut h = harness(false);
    h.set_size(egui::vec2(412.0, 520.0));
    h.run_steps(4);
    assert!(!has_button(&h, "Insert"), "the tab bar hides while the keyboard is up");
    for name in ["Bold", "Italic", "Underline", "Bullets", "Numbering", "Undo", "Hide Keyboard"] {
        assert!(has_button(&h, name), "{name} is on the keyboard bar");
    }
    let bar = h.get_by_role_and_label(egui::accesskit::Role::Button, "Hide Keyboard").rect();
    assert!(bar.bottom() > 520.0 - 60.0, "the bar sits on the keyboard: {bar:?}");
    h.state_mut().canvas.focused = true;
    tap(&mut h, "Hide Keyboard");
    assert!(!h.state().canvas.focused, "Hide Keyboard hands the focus back");
}

/// Home's Styles tile opens the style list (Normal and the headings first) and applies a style;
/// Line Spacing opens its choices.
#[test]
fn phone_styles_and_line_spacing_pages() {
    let mut h = harness(false);
    // The sample document has the heading styles; a blank one has only Normal.
    h.state_mut().session.set_document(wordcraft_engine::sample::sample_document());
    h.run_steps(2);
    tap(&mut h, "Home");
    scroll_sheet(&mut h);
    tap(&mut h, "Styles…");
    tap(&mut h, "Heading 1");
    let st = h.state_mut().session.run("format.state", &json!({})).unwrap();
    assert_eq!(st.get("styleName").and_then(|s| s.as_str()), Some("Heading 1"), "the style was applied: {st}");
    h.get_by_role_and_label(egui::accesskit::Role::Button, "Back").click();
    h.run_steps(3);

    tap(&mut h, "Line Spacing…");
    assert!(has_button(&h, "Double Spacing"), "the line spacing choices show");
}
