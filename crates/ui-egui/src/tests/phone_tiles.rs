//! The phone's Tiles style (the default) and the List style option: the tab bar, the tile sheets and
//! the More sheet, and the setting that picks the style.

use egui_kittest::kittest::Queryable;

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

/// The List style keeps the floating bar with ⋮, and no tab bar.
#[test]
fn phone_list_style_keeps_the_floating_bar() {
    let mut h = harness(true);
    assert!(!has_button(&h, "Insert"), "no tab bar in the List style");
    tap(&mut h, "More");
    assert!(phone::sheet_open(&h.ctx), "⋮ opens the list sheet");
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
