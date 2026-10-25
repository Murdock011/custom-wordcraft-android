//! The phone layout, in Material 3: a narrow window (a phone held upright) gets a top app bar, a
//! floating toolbar and a bottom sheet in place of the ribbon, title bar, rulers, panes and status
//! bar. The page keeps its canvas and opens in Edit view. Every control runs a command id, so the
//! phone chrome adds no behaviour of its own.

use egui::{Align2, Color32, Context, CornerRadius, Layout, Margin, Order, Panel, Pos2, Rect, Sense, Ui, vec2};
use serde_json::{Value, json};

use crate::WordApp;
use crate::icons;
use crate::theme::{medium, regular};

/// Windows narrower than this (points) get the phone layout. A phone held upright is about 390 pt.
pub const MAX_WIDTH: f32 = 600.0;
/// Height of the top app bar (Material's small app bar).
const APP_BAR_H: f32 = 64.0;
/// A Material icon button: 48 pt, the platform's touch target.
const ICON_BTN: f32 = 48.0;
/// The height of a list row (`list_row`): the Android minimum touch target, so it cannot go lower.
const LIST_ROW_H: f32 = 48.0;
/// Space between the floating toolbar and the bottom of the window.
const FLOAT_GAP: f32 = 16.0;
/// Corner radius of the floating toolbar and the bottom sheet.

/// Material 3 colour roles for the phone layout, light or dark as the app's theme is.
#[derive(Clone, Copy)]
pub struct Md {
    pub primary: Color32,
    pub on_primary: Color32,
    pub primary_container: Color32,
    pub on_primary_container: Color32,
    /// The screen's surface: the top app bar and the page's background.
    pub surface: Color32,
    /// Bottom sheets and the navigation drawer.
    pub surface_low: Color32,
    /// The floating toolbar.
    pub surface_high: Color32,
    pub on_surface: Color32,
    pub on_surface_variant: Color32,
    pub outline: Color32,
    pub outline_variant: Color32,
}

fn hex(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// The Material 3 colour roles for the current theme. Brand blue as the primary colour.
pub fn md(ctx: &Context) -> Md {
    if ctx.global_style().visuals.dark_mode {
        Md {
            primary: hex(0xB7C4FF),
            on_primary: hex(0x0E2A8C),
            primary_container: hex(0x2D44B3),
            on_primary_container: hex(0xDCE1FF),
            surface: hex(0x121317),
            surface_low: hex(0x1B1C21),
            surface_high: hex(0x292A30),
            on_surface: hex(0xE4E2E8),
            on_surface_variant: hex(0xC8C5D0),
            outline: hex(0x918F99),
            outline_variant: hex(0x46464F),
        }
    } else {
        Md {
            primary: hex(0x3B5BDB),
            on_primary: hex(0xFFFFFF),
            primary_container: hex(0xDCE1FF),
            on_primary_container: hex(0x001A5C),
            surface: hex(0xFDFBFF),
            surface_low: hex(0xF6F3F9),
            surface_high: hex(0xECE9F0),
            on_surface: hex(0x1B1B20),
            on_surface_variant: hex(0x46464F),
            outline: hex(0x77767F),
            outline_variant: hex(0xC8C5D0),
        }
    }
}

/// A colour with the given opacity: the state layer drawn over a control on hover or press.
fn state(c: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)
}

/// Whether the window is a phone: its shorter side is under the breakpoint, so a phone turned
/// sideways keeps the phone layout (a desktop or tablet window is wide and tall).
pub fn active(ctx: &Context) -> bool {
    let size = ctx.content_rect().size();
    is_narrow(size.x.min(size.y))
}

fn is_narrow(short_side: f32) -> bool {
    short_side < MAX_WIDTH
}

/// The state layer of a control: a tint over its shape on hover, a stronger one while pressed.
fn paint_state(ui: &Ui, rect: Rect, radius: u8, color: Color32, resp: &egui::Response) {
    let alpha = if resp.is_pointer_button_down_on() {
        30
    } else if resp.hovered() {
        20
    } else {
        return;
    };
    ui.painter().rect_filled(rect, CornerRadius::same(radius), state(color, alpha));
}

/// A Material icon button: 48 pt, a drawn icon, its name as the tooltip.
fn icon_button(ui: &mut Ui, m: Md, icon: &str, tip: &str) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(ICON_BTN, ICON_BTN), Sense::click());
    paint_state(ui, r, (ICON_BTN / 2.0) as u8, m.on_surface, &resp);
    icons::paint(ui.painter(), Rect::from_center_size(r.center(), vec2(24.0, 24.0)), icon, m.on_surface_variant, m.primary);
    resp.on_hover_text(crate::tl!(tip))
}

/// A Material text label button of at least `min_w` by 40 pt, in the primary colour.
fn text_button(ui: &mut Ui, m: Md, label: &str, color: Color32, min_w: f32) -> egui::Response {
    let font = medium(14.0);
    let text_w = ui.ctx().fonts_mut(|f| f.layout_no_wrap(label.to_string(), font.clone(), color).size().x);
    let (r, resp) = ui.allocate_exact_size(vec2((text_w + 24.0).max(min_w), ICON_BTN), Sense::click());
    paint_state(ui, r, 20, color, &resp);
    ui.painter().text(r.center(), Align2::CENTER_CENTER, label, font, color);
    let _ = m;
    resp
}

/// The top app bar: File (the Backstage), the document's name, and undo, redo and search.
pub fn top_bar(app: &mut WordApp, ui: &mut Ui) {
    let m = md(ui.ctx());
    let title = if app.has_document() { app.title_stem() } else { "WordCraft".to_string() };
    Panel::top("phone_top_bar").exact_size(APP_BAR_H).frame(egui::Frame::NONE.fill(m.surface).inner_margin(Margin::symmetric(4, 0))).show(ui, |ui| {
        ui.horizontal_centered(|ui| {
            if icon_button(ui, m, "file", "File").clicked() {
                let _ = app.run("ui.backstage", json!({"value": true}));
            }
            // The name gets what the icons on the right leave, and is cut short.
            let name_w = (ui.available_width() - 3.0 * ICON_BTN - 8.0).max(0.0);
            ui.add_sized(vec2(name_w, ICON_BTN), egui::Label::new(egui::RichText::new(title).font(medium(20.0)).color(m.on_surface)).truncate());
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                if icon_button(ui, m, "find", "Search").clicked() {
                    let _ = app.run("ui.dialog", json!({"name": "commands"}));
                }
                if icon_button(ui, m, "redo", "Redo").clicked() {
                    run(app, "edit.redo");
                }
                if icon_button(ui, m, "undo", "Undo").clicked() {
                    run(app, "edit.undo");
                }
            });
        });
    });
}

/// How long a status message stays on the phone. The desktop status bar keeps it for the same time.
const STATUS_MS: f64 = 6000.0;

/// The status message as a strip under the app bar. A phone has no status bar, so a notice (a
/// file that can't be opened, a save that fell back to the picker) would otherwise go unseen.
pub fn status_line(app: &WordApp, ui: &mut Ui) {
    let Some((msg, at)) = app.status_msg.as_ref() else { return };
    let age = crate::now_ms() - at;
    if !(0.0..STATUS_MS).contains(&age) {
        return;
    }
    let ctx = ui.ctx().clone();
    ctx.request_repaint_after(std::time::Duration::from_millis((STATUS_MS - age).max(0.0) as u64 + 50));
    let m = md(&ctx);
    let screen = ctx.content_rect();
    let width = (screen.width() - 2.0 * FLOAT_GAP).max(0.0);
    let pos = Pos2::new(screen.left() + FLOAT_GAP, screen.top() + APP_BAR_H + 8.0);
    egui::Area::new(egui::Id::new("phone_status")).fixed_pos(pos).order(Order::Foreground).interactable(false).show(&ctx, |ui| {
        egui::Frame::NONE.fill(m.surface_high).corner_radius(CornerRadius::same(8)).shadow(elevation()).inner_margin(Margin::symmetric(12, 8)).show(ui, |ui| {
            ui.set_width((width - 24.0).max(0.0));
            ui.add(egui::Label::new(egui::RichText::new(msg.as_str()).font(regular(14.0)).color(m.on_surface)).wrap());
        });
    });
}

/// The tabs of the menu. Draw has no commands yet, so it is left out. Table and Picture are last
/// and only listed while the caret is in a table, or a picture is selected (Word's contextual tabs).
const TABS: [&str; 8] = ["Home", "Insert", "Layout", "Review", "View", "Table", "Picture", "Draw"];
const TABLE_TAB: usize = 5;
const PICTURE_TAB: usize = 6;
const DRAW_TAB: usize = 7;

/// Which tabs the menu lists now: every tab but the contextual ones, then Table and Picture only
/// while their object is selected.
fn shown_tabs(app: &WordApp) -> [bool; 8] {
    [true, true, true, true, true, crate::ribbon::in_table_public(app), crate::ribbon::has_picture_selected(&app.session), true]
}

/// The rows of a tab: (label, command id). `ui.colors` is the colour dialog, run by [`run`].
fn rows(tab: usize) -> &'static [(&'static str, &'static str)] {
    match TABS.get(tab).copied() {
        Some("Home") => &[
            ("Paste", "edit.paste"),
            ("Highlight", "format.highlight"),
            ("Font Color", "ui.colors"),
            ("Clear Formatting", "format.clear"),
            ("Save", "file.save"),
            ("Repeat", "edit.repeat"),
        ],
        Some("Insert") => &[
            ("Blank Page", "insert.blankPage"),
            ("Table", "insert.table"),
            ("Picture", "insert.picture"),
            ("Shape", "insert.shape"),
            ("Text Box", "insert.textBox"),
            ("Comment", "review.newComment"),
            ("Header & Footer", "insert.header"),
            ("Page Number", "insert.pageNumber"),
            ("Equation", "insert.equation"),
            ("Footnote", "references.footnote"),
            ("Endnote", "references.endnote"),
        ],
        Some("Table") => &[
            ("Insert Row Above", "table.insertRowAbove"),
            ("Insert Row Below", "table.insertRowBelow"),
            ("Insert Column Left", "table.insertColumnLeft"),
            ("Insert Column Right", "table.insertColumnRight"),
            ("Delete Rows", "table.deleteRow"),
            ("Delete Columns", "table.deleteColumn"),
            ("Delete Table", "table.deleteTable"),
            ("Merge Cells", "table.merge"),
            ("Split Cells", "table.split"),
            ("AutoFit Contents", "table.autofitContents"),
            ("AutoFit Window", "table.autofitWindow"),
            ("Distribute Rows", "table.distributeRows"),
            ("Distribute Columns", "table.distributeColumns"),
            ("All Borders", "table.allBorders"),
            ("No Borders", "table.noBorders"),
            ("Outside Borders", "table.outsideBorders"),
            ("Inside Borders", "table.insideBorders"),
            ("Grid Table Style", "table.styleGrid"),
            ("Plain Table Style", "table.stylePlain"),
            ("Table Properties", "table.properties"),
        ],
        Some("Picture") => &[
            ("Size", "picture.size"),
            ("Crop", "ui.crop"),
            ("Rotate Right", "arrange.rotate.right"),
            ("Rotate Left", "arrange.rotate.left"),
            ("Flip Horizontal", "arrange.flipH"),
            ("Flip Vertical", "arrange.flipV"),
            ("Reset Picture", "picture.reset"),
            ("Delete Picture", "text.delete"),
            ("Style: Simple Frame", "picture.style.simpleFrame"),
            ("Style: Shadow", "picture.style.shadow"),
            ("Style: Rounded", "picture.style.rounded"),
            ("Effect: Blur", "picture.effects.blur"),
            ("Effect: Invert", "picture.effects.invert"),
            ("Effect: None", "picture.effects.none"),
            ("Brighter", "picture.corrections.brighter"),
            ("Darker", "picture.corrections.darker"),
            ("Higher Contrast", "picture.corrections.contrast"),
        ],
        Some("Layout") => &[
            ("Margins", "layout.margins"),
            ("Orientation", "layout.orientation"),
            ("Size", "layout.size"),
            ("Columns", "layout.columns"),
            ("Breaks", "layout.break"),
        ],
        Some("Review") => &[
            ("Spelling & Grammar", "review.spelling"),
            ("Word Count", "review.wordCount"),
            ("Track Changes", "review.trackChanges"),
            ("Next Change", "review.nextChange"),
            ("Previous Change", "review.previousChange"),
            ("Accept All Changes", "review.acceptAll"),
            ("Reject All Changes", "review.rejectAll"),
            ("Accept Change", "review.accept"),
            ("Reject Change", "review.reject"),
            ("New Comment", "review.newComment"),
            ("Show Comments", "review.comments"),
            ("Next Comment", "review.nextComment"),
            ("Previous Comment", "review.previousComment"),
        ],
        Some("Draw") => &[
            ("Pen", "draw.pen"),
            ("Highlighter", "draw.highlighter"),
            ("Eraser", "draw.eraser"),
            ("Lasso Select", "draw.lasso"),
            ("Done", "draw.select"),
        ],
        Some("View") => &[
            ("Edit view", "view.webLayout"),
            ("Print view", "phone.printView"),
            ("Zoom in", "view.zoomIn"),
            ("One Page", "view.onePage"),
            ("Multiple Pages", "view.multiplePages"),
            ("Page Width", "view.pageWidth"),
            ("Draft", "view.draft"),
            ("Formatting Marks", "view.marks"),
            ("Dark Mode", "view.darkMode"),
            ("Read mode", "view.readMode"),
        ],
        _ => &[],
    }
}

/// Runs a command from the phone chrome. The page keeps the keyboard: a toolbar press hands the
/// focus back to the page, so the keyboard stays up while the user types.
fn run(app: &mut WordApp, id: &str) {
    app.canvas.want_focus = true;
    let _ = match id {
        "ui.colors" => app.run("ui.dialog", json!({"name": "colors"})),
        // Table: Insert Table asks for the rows and columns, as Word's sheet does.
        "insert.table" => app.run("ui.dialog", json!({"name": "insertTable"})),
        "table.autofitContents" => app.run("table.autofit", json!({"mode": "contents"})),
        "table.autofitWindow" => app.run("table.autofit", json!({"mode": "window"})),
        "table.allBorders" => app.run("table.borders", json!({"kind": "all"})),
        "table.noBorders" => app.run("table.borders", json!({"kind": "none"})),
        "table.outsideBorders" => app.run("table.borders", json!({"kind": "outside"})),
        "table.insideBorders" => app.run("table.borders", json!({"kind": "inside"})),
        "table.styleGrid" => app.run("table.style", json!({"style": "TableGrid"})),
        "table.stylePlain" => app.run("table.style", json!({"style": "TableNormal"})),
        "arrange.rotate.right" => app.run("arrange.rotate", json!({"direction": "right"})),
        "arrange.rotate.left" => app.run("arrange.rotate", json!({"direction": "left"})),
        "arrange.flipH" => app.run("arrange.rotate", json!({"direction": "flipHorizontal"})),
        "arrange.flipV" => app.run("arrange.rotate", json!({"direction": "flipVertical"})),
        // Picture presets: a frame style, an effect, or a brightness or contrast step.
        "picture.style.simpleFrame" => app.run("picture.style", json!({"style": "simpleFrame"})),
        "picture.style.shadow" => app.run("picture.style", json!({"style": "shadow"})),
        "picture.style.rounded" => app.run("picture.style", json!({"style": "rounded"})),
        "picture.effects.blur" => app.run("picture.effects", json!({"effect": "blur"})),
        "picture.effects.invert" => app.run("picture.effects", json!({"effect": "invert"})),
        "picture.effects.none" => app.run("picture.effects", json!({"effect": "none"})),
        "picture.corrections.brighter" => app.run("picture.corrections", json!({"brightness": 20})),
        "picture.corrections.darker" => app.run("picture.corrections", json!({"brightness": -20})),
        "picture.corrections.contrast" => app.run("picture.corrections", json!({"contrast": 20})),
        // Print view: the pages, fitted to the screen's width so each page fits without sideways scrolling.
        "phone.printView" => {
            let _ = app.run("view.printLayout", json!({}));
            app.run("view.zoom", json!({"value": "pageWidth"}))
        }
        // Word Count is a dialog; the command itself only returns the numbers.
        "review.wordCount" => app.run("ui.dialog", json!({"name": "wordCount"})),
        _ => app.run(id, Value::Object(Default::default())),
    };
}

/// The menu's open tab, or `None` while it is closed. Kept in egui's memory, so it lasts the
/// session without a field on `WordApp`.
fn menu_tab(ctx: &Context) -> Option<usize> {
    ctx.data(|d| d.get_temp::<Option<usize>>(menu_id())).flatten()
}

fn set_menu(ctx: &Context, tab: Option<usize>) {
    ctx.data_mut(|d| d.insert_temp(menu_id(), tab));
}

/// Whether the bottom sheet is open. Its panel sits on the background layer, so a tap on it must not
/// count as a tap on the page's chrome (that would hand the keyboard to the page).
pub fn sheet_open(ctx: &Context) -> bool {
    menu_tab(ctx).is_some()
}

fn menu_id() -> egui::Id {
    egui::Id::new("phone_menu_tab")
}

/// Opens the menu on a tab, and takes the keyboard away from the page: the menu and the keyboard
/// never show together.
fn open_menu(app: &mut WordApp, ctx: &Context, tab: usize) {
    app.canvas.want_focus = false;
    app.canvas.focused = false;
    set_menu(ctx, Some(tab));
    // The keyboard takes a few frames to go; keep drawing until it has.
    ctx.request_repaint();
}

/// Whether the on-screen keyboard has just come up (it was down on the last frame).
fn keyboard_rising(ctx: &Context) -> bool {
    let up = keyboard_up_now(ctx);
    let id = egui::Id::new("phone_keyboard_was_up");
    let was_up = ctx.data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
    ctx.data_mut(|d| d.insert_temp(id, up));
    up && !was_up
}

/// Whether the on-screen keyboard is up: the window has shrunk well below the tallest it has been.
fn keyboard_up_now(ctx: &Context) -> bool {
    let h = ctx.content_rect().height();
    let id = egui::Id::new("phone_tallest_height");
    let tallest = ctx.data_mut(|d| {
        let m = d.get_temp::<f32>(id).unwrap_or(0.0).max(h);
        d.insert_temp(id, m);
        m
    });
    h < tallest * 0.75
}

/// The shadow the floating toolbar and the sheet cast: Material's elevation level 2.
fn elevation() -> egui::Shadow {
    egui::Shadow { offset: [0, 4], blur: 16, spread: 0, color: Color32::from_black_alpha(60) }
}

/// Height of the floating bar: the 48 pt buttons and its padding.
const PILL_H: f32 = ICON_BTN + 8.0;
/// Corner radius of the floating bar: a pill.
const PILL_R: u8 = (PILL_H / 2.0) as u8;

/// Space the floating bar takes from the bottom of the view: the page keeps the caret above it.
pub fn bar_clearance(ctx: &Context) -> f32 {
    if pill_hidden(ctx) { 0.0 } else { PILL_H + FLOAT_GAP }
}

/// A phone turned sideways with the keyboard up has a band too short for the floating bar: the bar
/// steps aside (the caret stays visible) until the keyboard closes.
pub fn pill_hidden(ctx: &Context) -> bool {
    keyboard_up_now(ctx) && ctx.content_rect().height() < 300.0
}

/// Where the floating bar goes: docked at the bottom of the visible area, which is the top of the
/// keyboard while it is up. It never sits under the app bar.
fn bar_top(ctx: &Context) -> f32 {
    let screen = ctx.content_rect();
    (screen.bottom() - FLOAT_GAP - PILL_H).max(screen.top() + APP_BAR_H)
}

/// The floating toolbar: formats, lists, insert and undo in groups, scrolled sideways when they
/// do not fit (centred when they do), with ⋮ at the right end. It sits at the bottom of the view, and
/// the bottom sheet takes its place when ⋮ opens it. The pill steps aside while a context menu
/// is open.
pub fn bottom_bar(app: &mut WordApp, ui: &mut Ui) {
    let m = md(ui.ctx());
    let ctx = ui.ctx().clone();
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        back_step(&ctx);
    }
    if pill_hidden(&ctx) {
        // Keep the keyboard tracking current while the bar is out of the way.
        if keyboard_rising(&ctx) {
            set_menu(&ctx, None);
        }
        return;
    }
    if tap_into_comment(app, &ctx) {
        set_menu(&ctx, Some(3));
        set_sub(&ctx, Some(Sub::Comments));
    }
    // The keyboard coming up closes the menu: the two never show together. The comment field is the
    // exception: its keyboard is what the sheet is for.
    let typing = matches!(sub(&ctx), Some(Sub::CommentInput));
    let open = if keyboard_rising(&ctx) && !typing {
        set_menu(&ctx, None);
        None
    } else {
        // A contextual tab goes away with its object (the table, the picture): fall back to Home.
        let shown = shown_tabs(app);
        menu_tab(&ctx).map(|t| if shown.get(t).copied().unwrap_or(false) { t } else { 0 })
    };
    // With the menu open it takes the bottom of the screen, and the toolbar hides.
    if let Some(tab) = open {
        return menu(app, ui, tab);
    }
    if app.canvas.context_menu_open {
        return;
    }
    let width = (ctx.content_rect().width() - 2.0 * FLOAT_GAP).max(0.0);
    let pos = Pos2::new(ctx.content_rect().left() + FLOAT_GAP, bar_top(&ctx));
    egui::Area::new(egui::Id::new("phone_quick_bar")).fixed_pos(pos).order(Order::Foreground).show(&ctx, |ui| {
        egui::Frame::NONE
            .fill(m.surface_high)
            .corner_radius(CornerRadius::same(PILL_R))
            .shadow(elevation())
            .inner_margin(Margin::symmetric(8, 4))
            .show(ui, |ui| {
                ui.set_width((width - 16.0).max(0.0));
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let middle = (width - 16.0 - ICON_BTN - 8.0).max(ICON_BTN);
                    // Centred when the groups fit: the last frame's width says how far to indent.
                    let used_id = egui::Id::new("phone_bar_used");
                    let used = ctx.data(|d| d.get_temp::<f32>(used_id)).unwrap_or(0.0);
                    egui::ScrollArea::horizontal()
                        .id_salt("quick_bar_items")
                        .max_width(middle)
                        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 0.0;
                                ui.add_space(((middle - used) / 2.0).max(0.0));
                                let start = ui.cursor().left();
                                for (icon, label, id) in [
                                    ("bold", "Bold", "format.bold"),
                                    ("italic", "Italic", "format.italic"),
                                    ("underline", "Underline", "format.underline"),
                                ] {
                                    if icon_button(ui, m, icon, label).clicked() {
                                        run(app, id);
                                    }
                                }
                                if icon_button(ui, m, "highlight", "Highlight").clicked() {
                                    run(app, "format.highlight");
                                }
                                if icon_button(ui, m, "fontcolor", "Font Color").clicked() {
                                    run(app, "ui.colors");
                                }
                                divider(ui, m);
                                for (icon, label, id) in [
                                    ("bullets", "Bullets", "para.bullets"),
                                    ("numbering", "Numbering", "para.numbering"),
                                    ("outdent", "Decrease Indent", "para.outdent"),
                                    ("indentLeft", "Increase Indent", "para.indent"),
                                ] {
                                    if icon_button(ui, m, icon, label).clicked() {
                                        run(app, id);
                                    }
                                }
                                divider(ui, m);
                                if icon_button(ui, m, "comment", "New Comment").clicked() {
                                    set_menu(&ctx, Some(1));
                                    comment_field(&ctx, CommentTarget::New);
                                }
                                if icon_button(ui, m, "link", "Link").clicked() {
                                    let _ = app.run("ui.dialog", json!({"name": "link"}));
                                }
                                if icon_button(ui, m, "picture", "Picture").clicked() {
                                    run(app, "insert.picture");
                                }
                                if icon_button(ui, m, "table", "Table").clicked() {
                                    run(app, "insert.table");
                                }
                                divider(ui, m);
                                if icon_button(ui, m, "undo", "Undo").clicked() {
                                    run(app, "edit.undo");
                                }
                                let used = ui.cursor().left() - start;
                                ctx.data_mut(|d| d.insert_temp(used_id, used));
                            });
                        });
                    divider(ui, m);
                    // Fixed at the right: ⋮ opens the menu.
                    if icon_button(ui, m, "more", "More").clicked() {
                        open_menu(app, &ctx, 0);
                    }
                });
            });
    });
}

/// Whether Android Back has a phone sheet page, sheet or menu to close.
pub fn can_go_back(ctx: &Context) -> bool {
    sub(ctx).is_some() || menu_tab(ctx).is_some()
}

/// Escape or Back: closes the innermost open part of the sheet (its page first, then the sheet).
pub fn back_step(ctx: &Context) {
    if sub(ctx).is_some() {
        sub_back(ctx);
    } else {
        set_menu(ctx, None);
    }
}

/// Opens the phone selection menu at its top list (not in More).
pub fn reset_context_more(ctx: &Context) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("phone_ctx_more"), false));
}

/// The selection menu on a phone (`canvas::context_menu`): for a misspelt word a strip of
/// suggestions with Ignore All and Add to Dictionary, then Cut, Copy, Paste, Select All and New
/// Comment, and More for the rarer paste and format items, with a back arrow to return.
pub fn context_menu(app: &mut WordApp, ui: &mut Ui) {
    let m = md(ui.ctx());
    let ctx = ui.ctx().clone();
    let more_id = egui::Id::new("phone_ctx_more");
    let in_more = ctx.data(|d| d.get_temp::<bool>(more_id)).unwrap_or(false);
    ui.set_width(260.0);
    // No gap between rows: the 48 pt rows alone set the pitch.
    ui.spacing_mut().item_spacing.y = 0.0;
    if in_more {
        if list_row(ui, m, &crate::tl!("Back").to_string(), "chevronLeft", false).clicked() {
            ctx.data_mut(|d| d.insert_temp(more_id, false));
        }
        for (label, icon, id, params) in [
            ("Keep Text Only", "paste", "edit.pasteText", json!({})),
            ("Merge Formatting", "paste", "edit.pasteMerge", json!({})),
            ("Paste Special…", "paste", "edit.pasteSpecial", json!({})),
            ("Font…", "fontdialog", "ui.dialog", json!({"name": "font"})),
            ("Paragraph…", "lineSpacing", "ui.dialog", json!({"name": "paragraph"})),
            ("Link…", "link", "ui.dialog", json!({"name": "link"})),
        ] {
            if list_row(ui, m, &crate::tl!(label).to_string(), icon, false).clicked() {
                let _ = app.run(id, params);
                ui.close();
            }
        }
        return;
    }
    if let Some(issue) = app.canvas.context_issue.clone().filter(|i| i.get("kind").and_then(|k| k.as_str()) == Some("spelling")) {
        let sugg: Vec<String> = issue
            .get("suggestions")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
            .unwrap_or_default();
        egui::ScrollArea::horizontal().id_salt("ctx_suggestions").auto_shrink([false, true]).show(ui, |ui| {
            ui.horizontal(|ui| {
                for sgt in sugg.iter().take(3) {
                    if ui.button(egui::RichText::new(sgt).strong()).clicked() {
                        let _ = app.run("review.applySuggestion", json!({"text": sgt}));
                        ui.close();
                    }
                }
                if ui.button(crate::tl!("Ignore All").to_string()).clicked() {
                    let _ = app.run("review.ignoreAll", json!({}));
                    ui.close();
                }
                if ui.button(crate::tl!("Add").to_string()).clicked() {
                    let _ = app.run("review.addToDictionary", json!({}));
                    ui.close();
                }
            });
        });
        ui.separator();
    }
    let selected = !app.session.sel.is_collapsed();
    for (label, icon, id) in
        [("Cut", "cut", "edit.cut"), ("Copy", "copy", "edit.copy"), ("Paste", "paste", "edit.paste"), ("Select All", "select", "select.all")]
    {
        let on = crate::widgets::enabled(app, id) && (selected || !matches!(id, "edit.cut" | "edit.copy"));
        ui.add_enabled_ui(on, |ui| {
            if list_row(ui, m, &crate::tl!(label).to_string(), icon, false).clicked() {
                let _ = app.run(id, json!({}));
                ui.close();
            }
        });
    }
    if list_row(ui, m, &crate::tl!("New Comment").to_string(), "comment", false).clicked() {
        set_menu(&ctx, Some(3));
        comment_field(&ctx, CommentTarget::New);
        ui.close();
    }
    if list_row(ui, m, &crate::tl!("More").to_string(), "more", true).clicked() {
        ctx.data_mut(|d| d.insert_temp(more_id, true));
    }
}

/// A thin vertical rule between groups of toolbar controls.
fn divider(ui: &mut Ui, m: Md) {
    let (r, _) = ui.allocate_exact_size(vec2(1.0, 24.0), Sense::hover());
    ui.painter().vline(r.center().x, r.y_range(), egui::Stroke::new(1.0, m.outline_variant));
}

/// The bottom sheet: a drag handle, the tabs (the selected one underlined in the primary colour),
/// Close, and the tab's rows as list items. It replaces the toolbar while it is open.
fn menu(app: &mut WordApp, ui: &mut Ui, tab: usize) {
    let m = md(ui.ctx());
    let ctx = ui.ctx().clone();
    // The sheet takes the input: while it is open the page must not hold focus, or a tap on a row
    // reaches the page's text input, raises the keyboard, and keyboard_rising closes the sheet.
    app.canvas.want_focus = false;
    app.canvas.focused = false;
    let height = (ctx.content_rect().height() * 0.55).clamp(300.0, 520.0);
    // Flat top edge: the rounded corners showed dark wedges on the device, so the sheet is square.
    let top_only = CornerRadius::ZERO;
    Panel::bottom("phone_menu")
        .exact_size(height)
        .frame(egui::Frame::NONE.fill(m.surface_low).corner_radius(top_only).shadow(elevation()).inner_margin(Margin::symmetric(16, 0)))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                let (handle, _) = ui.allocate_exact_size(vec2(32.0, 4.0), Sense::hover());
                ui.painter().rect_filled(handle, CornerRadius::same(2), m.outline);
            });
            if let Some(s) = sub(&ctx) {
                return format_page(app, ui, m, &ctx, s);
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                tab_dropdown(ui, m, &ctx, tab, shown_tabs(app));
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    if text_button(ui, m, &crate::tl!("Close").to_string(), m.primary, 48.0).clicked() {
                        set_menu(&ctx, None);
                    }
                    if icon_button(ui, m, "redo", "Redo").clicked() {
                        run(app, "edit.redo");
                    }
                    if icon_button(ui, m, "undo", "Undo").clicked() {
                        run(app, "edit.undo");
                    }
                });
            });
            // The character row is Home's alone; the other tabs show their own rows.
            if tab == 0 {
                font_row(app, ui, m, &ctx);
            }
            ui.add_space(8.0);
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (label, id) in rows(tab) {
                    if list_row(ui, m, label, row_icon(id), row_chevron(id)).clicked() {
                        // Highlight and Font Color open their swatch grids, not the last colour.
                        match *id {
                            "format.highlight" => set_sub(&ctx, Some(Sub::Highlight)),
                            "ui.colors" => set_sub(&ctx, Some(Sub::Colour)),
                            "layout.margins" => set_sub(&ctx, Some(Sub::Margins)),
                            "layout.orientation" => set_sub(&ctx, Some(Sub::Orientation)),
                            "layout.size" => set_sub(&ctx, Some(Sub::PaperSize)),
                            "layout.columns" => set_sub(&ctx, Some(Sub::Columns)),
                            "layout.break" => set_sub(&ctx, Some(Sub::Breaks)),
                            "review.comments" => set_sub(&ctx, Some(Sub::Comments)),
                            "review.newComment" => comment_field(&ctx, CommentTarget::New),
                            _ => run(app, id),
                        }
                        // A menu row keeps the keyboard down: the menu is open.
                        app.canvas.want_focus = false;
                    }
                }
                if tab == TABLE_TAB {
                    ui.add_space(8.0);
                    tile_section(ui, m, &crate::tl!("Cell Alignment").to_string());
                    align_tiles(app, ui, m);
                    ui.add_space(8.0);
                    tile_section(ui, m, &crate::tl!("Shading").to_string());
                    shading_swatches(app, ui, m);
                }
                if tab == PICTURE_TAB {
                    picture_controls(app, ui, m);
                }
                if tab == DRAW_TAB {
                    draw_controls(app, ui, m);
                }
            });
        });
}

/// Home's font row: the font name and size stepper, then the character formats.
fn font_row(app: &mut WordApp, ui: &mut Ui, m: Md, ctx: &Context) {
    // The font row, always here (no selection needed): the font name and the size stepper
    // in a fixed row, then the character formats, which scroll sideways.
    let st: Value = app.session.run("format.state", &json!({})).unwrap_or_default();
    let font = st.get("font").and_then(Value::as_str).unwrap_or("Font").to_string();
    let cur = st.get("size").and_then(Value::as_f64).unwrap_or(12.0) as f32;
    let size = if cur.fract() == 0.0 { format!("{cur:.0}") } else { format!("{cur}") };
    ui.horizontal(|ui| {
        if text_button(ui, m, &font, m.on_surface, 170.0).clicked() {
            set_sub(ctx, Some(Sub::Font));
        }
        if text_button(ui, m, "−", m.primary, 44.0).clicked() {
            if let Some(x) = wordcraft_engine::cmd::format::SIZES.iter().rev().copied().find(|x| *x < cur) {
                let _ = app.run("format.size", json!({"size": x}));
            }
        }
        if text_button(ui, m, &size, m.on_surface_variant, 56.0).clicked() {
            set_sub(ctx, Some(Sub::Size));
        }
        if text_button(ui, m, "+", m.primary, 44.0).clicked() {
            if let Some(x) = wordcraft_engine::cmd::format::SIZES.iter().copied().find(|x| *x > cur) {
                let _ = app.run("format.size", json!({"size": x}));
            }
        }
    });
    egui::ScrollArea::horizontal().id_salt("phone_font_row").scroll_source(egui::scroll_area::ScrollSource::ALL).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for (icon, label, id) in [
                ("bold", "Bold", "format.bold"),
                ("italic", "Italic", "format.italic"),
                ("underline", "Underline", "format.underline"),
                ("strike", "Strikethrough", "format.strikethrough"),
                ("subscript", "Subscript", "format.subscript"),
                ("superscript", "Superscript", "format.superscript"),
                ("clear", "Clear Formatting", "format.clear"),
            ] {
                if icon_button(ui, m, icon, label).clicked() {
                    run(app, id);
                }
            }
            divider(ui, m);
            if icon_button(ui, m, "highlight", "Highlight").clicked() {
                set_sub(ctx, Some(Sub::Highlight));
            }
            if icon_button(ui, m, "fontcolor", "Font Color").clicked() {
                set_sub(ctx, Some(Sub::Colour));
            }
        });
    });
}

/// The format pages a sheet can show in place of its tabs (font list, size, highlight, colour).
#[derive(Clone, Copy, PartialEq)]
enum Sub {
    Font,
    Size,
    Highlight,
    Colour,
    Margins,
    Orientation,
    PaperSize,
    Columns,
    Breaks,
    Comments,
    CommentInput,
}

/// What the comment field writes: a new comment on the selection, or a reply in a thread.
#[derive(Clone, Copy, PartialEq)]
enum CommentTarget {
    New,
    Reply(u32),
}

fn target_id() -> egui::Id {
    egui::Id::new("phone_comment_target")
}

fn comment_target(ctx: &Context) -> CommentTarget {
    ctx.data(|d| d.get_temp(target_id())).unwrap_or(CommentTarget::New)
}

/// Opens the comment field empty, with the keyboard asked for. The caller opens the sheet.
fn comment_field(ctx: &Context, target: CommentTarget) {
    ctx.data_mut(|d| {
        d.insert_temp(target_id(), target);
        d.insert_temp(egui::Id::new("phone_comment_text"), String::new());
        d.insert_temp(egui::Id::new("phone_comment_focus"), true);
    });
    set_sub(ctx, Some(Sub::CommentInput));
}

/// Back from a sub-page: a reply returns to the comment list, anything else to the tabs.
fn sub_back(ctx: &Context) {
    let next = match sub(ctx) {
        Some(Sub::CommentInput) if matches!(comment_target(ctx), CommentTarget::Reply(_)) => Some(Sub::Comments),
        _ => None,
    };
    set_sub(ctx, next);
}

/// A tap that lands in a comment's range opens the Comments sheet, as a tap on a commented word does
/// in Word. It fires on the frame after the tap, when the caret has moved. Taps on the sheet, the
/// top bar and the floating bar do not count.
fn tap_into_comment(app: &WordApp, ctx: &Context) -> bool {
    let id = egui::Id::new("phone_tap_pending");
    let pending = ctx.data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
    // Read the layout before `ctx.input`: a context lock taken inside another one panics on the web build.
    let (page_top, page_bottom) = (ctx.content_rect().top() + APP_BAR_H, bar_top(ctx));
    let released = ctx.input(|i| {
        let on_page = i.pointer.interact_pos().is_some_and(|p| p.y > page_top && p.y < page_bottom);
        i.pointer.primary_released() && !i.pointer.is_decidedly_dragging() && on_page
    });
    let armed = released && menu_tab(ctx).is_none();
    ctx.data_mut(|d| d.insert_temp(id, armed));
    if armed {
        ctx.request_repaint();
    }
    pending && menu_tab(ctx).is_none() && wordcraft_engine::cmd::review::comment_range_at_caret(&app.session).is_some()
}

fn sub_id() -> egui::Id {
    egui::Id::new("phone_format_sub")
}

fn sub(ctx: &Context) -> Option<Sub> {
    ctx.data(|d| d.get_temp::<Option<Sub>>(sub_id())).flatten()
}

fn set_sub(ctx: &Context, s: Option<Sub>) {
    ctx.data_mut(|d| d.insert_temp(sub_id(), s));
}

/// Standard colours for the font colour grid (our own palette).
const FONT_COLOURS: [&str; 16] = [
    "000000", "FFFFFF", "C00000", "FF0000", "FFC000", "FFFF00", "92D050", "00B050", "00B0F0", "0070C0", "002060", "7030A0", "1F4E79", "2E75B6",
    "A9D18E", "F4B183",
];

fn hex_colour(hex: &str) -> Color32 {
    let v = u32::from_str_radix(hex, 16).unwrap_or(0);
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// One swatch of a colour grid: a filled rounded square, outlined when it is the palette's blank.
fn swatch(ui: &mut Ui, m: Md, fill: Option<Color32>) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(56.0, 44.0), Sense::click());
    paint_state(ui, r, 8, m.on_surface, &resp);
    let body = r.shrink(6.0);
    match fill {
        Some(c) => {
            ui.painter().rect_filled(body, CornerRadius::same(6), c);
        }
        None => {
            ui.painter().rect_stroke(body, CornerRadius::same(6), egui::Stroke::new(1.0, m.outline), egui::StrokeKind::Inside);
            ui.painter().text(body.center(), Align2::CENTER_CENTER, "None", egui::FontId::proportional(11.0), m.on_surface_variant);
        }
    }
    resp
}

/// A format page: back arrow and title, then the list or grid. Each choice applies at the caret
/// or to the selection and keeps the sheet open.
fn format_page(app: &mut WordApp, ui: &mut Ui, m: Md, ctx: &Context, s: Sub) {
    let title = match s {
        Sub::Font => "Font",
        Sub::Size => "Size",
        Sub::Highlight => "Highlight",
        Sub::Colour => "Font Color",
        Sub::Margins => "Margins",
        Sub::Orientation => "Orientation",
        Sub::PaperSize => "Size",
        Sub::Columns => "Columns",
        Sub::Breaks => "Breaks",
        Sub::Comments => "Comments",
        Sub::CommentInput => match comment_target(ctx) {
            CommentTarget::New => "New Comment",
            CommentTarget::Reply(_) => "Reply",
        },
    };
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(crate::tl!(title).to_string()).strong());
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            if text_button(ui, m, &crate::tl!("Back").to_string(), m.primary, 72.0).clicked() {
                sub_back(ctx);
            }
        });
    });
    ui.add_space(4.0);
    match s {
        Sub::Font => {
            let fams = app.previews.families();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for f in fams {
                    if list_row(ui, m, &f, "fontdialog", false).clicked() {
                        let name = app.previews.font_stored(&f);
                        let _ = app.run("format.font", json!({"name": name}));
                        app.canvas.want_focus = false;
                    }
                }
            });
        }
        Sub::Size => {
            let st: Value = app.session.run("format.state", &json!({})).unwrap_or_default();
            let cur = st.get("size").and_then(Value::as_f64).unwrap_or(12.0) as f32;
            let sizes = wordcraft_engine::cmd::format::SIZES;
            ui.horizontal(|ui| {
                if text_button(ui, m, "−", m.primary, 56.0).clicked() {
                    if let Some(x) = sizes.iter().rev().copied().find(|x| *x < cur) {
                        let _ = app.run("format.size", json!({"size": x}));
                    }
                }
                ui.label(egui::RichText::new(format!("{cur:.0}")).strong());
                if text_button(ui, m, "+", m.primary, 56.0).clicked() {
                    if let Some(x) = sizes.iter().copied().find(|x| *x > cur) {
                        let _ = app.run("format.size", json!({"size": x}));
                    }
                }
            });
            egui::ScrollArea::vertical().show(ui, |ui| {
                for x in sizes {
                    if list_row(ui, m, &format!("{x}"), "size", false).clicked() {
                        let _ = app.run("format.size", json!({"size": x}));
                    }
                }
            });
        }
        Sub::Highlight => {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for h in wordcraft_doc::props::Highlight::ALL {
                        let fill = h.rgb().map(|c| Color32::from_rgb(c.0, c.1, c.2));
                        if swatch(ui, m, fill).clicked() {
                            let _ = app.run("format.highlight", json!({"color": format!("{h:?}")}));
                            app.canvas.want_focus = false;
                        }
                    }
                });
            });
        }
        // Layout pages: each row runs its command and keeps the sheet open, as the format pages do.
        Sub::Margins | Sub::Orientation | Sub::PaperSize | Sub::Columns | Sub::Breaks => {
            let choices: &[(&str, &str, &str, Value)] = match s {
                Sub::Margins => &[
                    ("Normal", "margins", "layout.margins", json!({"preset": "normal"})),
                    ("Narrow", "margins", "layout.margins", json!({"preset": "narrow"})),
                    ("Moderate", "margins", "layout.margins", json!({"preset": "moderate"})),
                    ("Wide", "margins", "layout.margins", json!({"preset": "wide"})),
                ],
                Sub::Orientation => &[
                    ("Portrait", "printLayout", "layout.orientation", json!({"value": "portrait"})),
                    ("Landscape", "printLayout", "layout.orientation", json!({"value": "landscape"})),
                ],
                Sub::PaperSize => &[
                    ("Letter", "size", "layout.size", json!({"name": "Letter"})),
                    ("Legal", "size", "layout.size", json!({"name": "Legal"})),
                    ("A3", "size", "layout.size", json!({"name": "A3"})),
                    ("A4", "size", "layout.size", json!({"name": "A4"})),
                ],
                Sub::Columns => &[
                    ("One", "printLayout", "layout.columns", json!({"count": 1})),
                    ("Two", "printLayout", "layout.columns", json!({"count": 2})),
                    ("Three", "printLayout", "layout.columns", json!({"count": 3})),
                ],
                _ => &[
                    ("Page", "more", "layout.break", json!({"kind": "page"})),
                    ("Column", "more", "layout.break", json!({"kind": "column"})),
                    ("Text Wrapping", "more", "layout.break", json!({"kind": "textWrapping"})),
                    ("Next Page", "more", "layout.break", json!({"kind": "nextPage"})),
                    ("Continuous", "more", "layout.break", json!({"kind": "continuous"})),
                    ("Even Page", "more", "layout.break", json!({"kind": "evenPage"})),
                    ("Odd Page", "more", "layout.break", json!({"kind": "oddPage"})),
                ],
            };
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (label, icon, id, params) in choices {
                    if list_row(ui, m, label, icon, false).clicked() {
                        let _ = app.run(id, params.clone());
                        app.canvas.want_focus = false;
                    }
                }
            });
        }
        Sub::Comments => comments_page(app, ui, m, ctx),
        Sub::CommentInput => comment_input_page(app, ui, m, ctx),
        Sub::Colour => {
            if list_row(ui, m, &crate::tl!("Automatic").to_string(), "fontcolor", false).clicked() {
                let _ = app.run("format.color", json!({"color": "auto"}));
                app.canvas.want_focus = false;
            }
            ui.horizontal_wrapped(|ui| {
                for hex in FONT_COLOURS {
                    if swatch(ui, m, Some(hex_colour(hex))).clicked() {
                        let _ = app.run("format.color", json!({"color": hex}));
                        app.canvas.want_focus = false;
                    }
                }
            });
        }
    }
}

/// The top comment of the thread a comment belongs to (its parent chain, bounded by the list).
fn thread_root(list: &[Value], id: u32) -> u32 {
    let mut cur = id;
    for _ in 0..list.len() {
        let parent =
            list.iter().find(|c| c.get("id").and_then(Value::as_u64) == Some(cur as u64)).and_then(|c| c.get("parent")).and_then(Value::as_u64);
        match parent {
            Some(p) => cur = p as u32,
            None => break,
        }
    }
    cur
}

/// The Comments sheet: every comment in reading order with its anchored text, and Reply, Resolve
/// and Delete for each thread. Replies are indented under their comment.
fn comments_page(app: &mut WordApp, ui: &mut Ui, m: Md, ctx: &Context) {
    let list: Vec<Value> = app.session.run("review.comments", &json!({})).ok().and_then(|v| v.as_array().cloned()).unwrap_or_default();
    if list.is_empty() {
        ui.add_space(16.0);
        ui.label(egui::RichText::new(crate::tl!("No comments in this document.").to_string()).color(m.on_surface_variant));
        return;
    }
    egui::ScrollArea::vertical().show(ui, |ui| {
        for c in &list {
            let id = c.get("id").and_then(Value::as_u64).unwrap_or(0) as u32;
            let reply = c.get("parent").is_some_and(|p| !p.is_null());
            let root = thread_root(&list, id);
            let resolved = c.get("resolved").and_then(Value::as_bool).unwrap_or(false);
            let author = c.get("author").and_then(Value::as_str).unwrap_or("");
            let text = c.get("text").and_then(Value::as_str).unwrap_or("").trim_end().to_string();
            let anchor = c.get("anchorText").and_then(Value::as_str).unwrap_or("").to_string();
            let body = |ui: &mut Ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(author).strong());
                    if resolved {
                        ui.label(egui::RichText::new(crate::tl!("Resolved").to_string()).small().color(m.primary));
                    }
                });
                if !reply && !anchor.is_empty() {
                    ui.label(egui::RichText::new(format!("\u{201C}{anchor}\u{201D}")).italics().color(m.on_surface_variant));
                }
                ui.label(egui::RichText::new(text).color(m.on_surface));
                ui.horizontal(|ui| {
                    if text_button(ui, m, &crate::tl!("Reply").to_string(), m.primary, 64.0).clicked() {
                        comment_field(ctx, CommentTarget::Reply(root));
                    }
                    let toggle = if resolved { crate::tl!("Reopen") } else { crate::tl!("Resolve") };
                    if text_button(ui, m, &toggle.to_string(), m.primary, 80.0).clicked() {
                        let _ = app.run("review.resolveComment", json!({"id": root}));
                    }
                    if text_button(ui, m, &crate::tl!("Delete").to_string(), m.on_surface_variant, 72.0).clicked() {
                        let _ = app.run("review.deleteComment", json!({"id": id}));
                    }
                });
            };
            if reply {
                ui.indent(("phone_comment_reply", id), body);
            } else {
                body(ui);
            }
            ui.separator();
        }
    });
}

/// The comment field: a multiline box with the keyboard, and Cancel and Add. A new comment goes on
/// the selection and closes the sheet; a reply goes back to the comment list.
fn comment_input_page(app: &mut WordApp, ui: &mut Ui, m: Md, ctx: &Context) {
    let buf_id = egui::Id::new("phone_comment_text");
    let mut text: String = ctx.data(|d| d.get_temp(buf_id)).unwrap_or_default();
    let focus = ctx.data_mut(|d| d.remove_temp::<bool>(egui::Id::new("phone_comment_focus"))).unwrap_or(false);
    let field = ui
        .add(egui::TextEdit::multiline(&mut text).desired_rows(4).desired_width(f32::INFINITY).hint_text(crate::tl!("Write a comment").to_string()));
    if focus {
        field.request_focus();
    }
    ctx.data_mut(|d| d.insert_temp(buf_id, text.clone()));
    ui.add_space(8.0);
    let target = comment_target(ctx);
    let ready = !text.trim().is_empty();
    ui.horizontal(|ui| {
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            let add = ui.add_enabled(ready, egui::Button::new(crate::tl!("Add").to_string()));
            if add.clicked() {
                let done = match target {
                    CommentTarget::New => app.run("review.newComment", json!({"text": text.trim_end()})),
                    CommentTarget::Reply(root) => app.run("review.reply", json!({"id": root, "text": text.trim_end()})),
                };
                if done.is_ok() {
                    ctx.data_mut(|d| d.insert_temp(buf_id, String::new()));
                    finish_comment(ctx, target);
                }
            }
            if text_button(ui, m, &crate::tl!("Cancel").to_string(), m.primary, 80.0).clicked() {
                finish_comment(ctx, target);
            }
        });
    });
}

/// Leaves the comment field: a reply goes back to the list, a new comment closes the sheet.
fn finish_comment(ctx: &Context, target: CommentTarget) {
    match target {
        CommentTarget::Reply(_) => set_sub(ctx, Some(Sub::Comments)),
        CommentTarget::New => {
            set_sub(ctx, None);
            set_menu(ctx, None);
        }
    }
}

/// The Draw tab's colours and thicknesses. They set the pen in use: a highlighter stays a highlighter,
/// any other tool becomes the pen (the engine's `draw.pen` picks the tool).
fn draw_controls(app: &mut WordApp, ui: &mut Ui, m: Md) {
    let tool = if app.session.view.draw.mode == wordcraft_engine::cmd::draw::DrawMode::Pen(wordcraft_doc::freeform::InkTool::Highlighter) {
        "draw.highlighter"
    } else {
        "draw.pen"
    };
    ui.add_space(8.0);
    tile_section(ui, m, &crate::tl!("Colour").to_string());
    ui.horizontal_wrapped(|ui| {
        for hex in ["000000", "C00000", "0070C0", "00B050", "FFC000"] {
            if swatch(ui, m, Some(hex_colour(hex))).clicked() {
                let _ = app.run(tool, json!({"color": hex}));
            }
        }
    });
    ui.add_space(8.0);
    tile_section(ui, m, &crate::tl!("Thickness").to_string());
    ui.horizontal(|ui| {
        for (label, width) in [("Thin", 1.5), ("Medium", 3.0), ("Thick", 6.0)] {
            if text_button(ui, m, &crate::tl!(label).to_string(), m.primary, 96.0).clicked() {
                let _ = app.run(tool, json!({"width": width}));
            }
        }
    });
}

/// The Picture tab's controls under its rows: wrapping, alignment and alt text. Each one runs a
/// command, so the sheet adds no picture behaviour of its own.
fn picture_controls(app: &mut WordApp, ui: &mut Ui, m: Md) {
    ui.add_space(8.0);
    tile_section(ui, m, &crate::tl!("Wrap Text").to_string());
    ui.horizontal_wrapped(|ui| {
        ui.add_space(16.0);
        for (label, value) in [
            ("Inline", "inline"),
            ("Square", "square"),
            ("Tight", "tight"),
            ("Through", "through"),
            ("Top and Bottom", "topAndBottom"),
            ("Behind Text", "behindText"),
            ("In Front of Text", "inFrontOfText"),
        ] {
            if ui.add(egui::Button::new(crate::tl!(label).to_string())).clicked() {
                let _ = app.run("arrange.wrap", json!({"wrap": value}));
            }
        }
    });
    ui.add_space(8.0);
    tile_section(ui, m, &crate::tl!("Align").to_string());
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        for (label, value) in [("Left", "left"), ("Center", "center"), ("Right", "right")] {
            if ui.add(egui::Button::new(crate::tl!(label).to_string())).clicked() {
                let _ = app.run("arrange.align", json!({"value": value}));
            }
        }
    });
    ui.add_space(8.0);
    tile_section(ui, m, &crate::tl!("Alt Text").to_string());
    alt_text_field(app, ui);
}

/// The selected picture's description, edited in place. The buffer is kept with the description it
/// started from, so selecting another picture shows that picture's text.
fn alt_text_field(app: &mut WordApp, ui: &mut Ui) {
    let current = match wordcraft_engine::cmd::objects::selected(&app.session) {
        Some((_, wordcraft_doc::para::InlineObject::Image { alt, .. })) => alt,
        _ => String::new(),
    };
    let id = egui::Id::new("phone_alt_text_buffer");
    let (seen, mut buf): (String, String) = ui.data(|d| d.get_temp(id)).unwrap_or_else(|| (current.clone(), current.clone()));
    if seen != current {
        buf = current.clone();
    }
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        ui.vertical(|ui| {
            ui.add(
                egui::TextEdit::multiline(&mut buf)
                    .desired_rows(3)
                    .desired_width(ui.available_width() - 16.0)
                    .hint_text(crate::tl!("Describe this picture").to_string()),
            );
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new(crate::tl!("Save").to_string())).clicked() {
                    let _ = app.run("object.altText", json!({"text": buf.clone()}));
                }
                if ui.add(egui::Button::new(crate::tl!("Mark as decorative").to_string())).clicked() {
                    let _ = app.run("object.altText", json!({"decorative": true}));
                }
            });
        });
    });
    ui.data_mut(|d| d.insert_temp(id, (current, buf)));
}

/// A small heading above a row of tiles or swatches.
fn tile_section(ui: &mut Ui, m: Md, label: &str) {
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        ui.label(egui::RichText::new(label).font(medium(14.0)).color(m.on_surface_variant));
    });
}

/// Cell alignment: six tiles in two rows of three, each a small cell with a dot where the text
/// sits. ponytail: the engine takes all nine positions (`table.cellAlign`); the middle row is left
/// out to keep the grid at Word's six tiles.
fn align_tiles(app: &mut WordApp, ui: &mut Ui, m: Md) {
    const TILES: [(&str, &str); 6] = [
        ("Top Left", "topLeft"),
        ("Top Center", "topCenter"),
        ("Top Right", "topRight"),
        ("Bottom Left", "bottomLeft"),
        ("Bottom Center", "bottomCenter"),
        ("Bottom Right", "bottomRight"),
    ];
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add_space(12.0);
        for (label, value) in TILES {
            let (r, resp) = ui.allocate_exact_size(vec2(ICON_BTN, ICON_BTN), Sense::click());
            paint_state(ui, r, 8, m.on_surface, &resp);
            let frame = Rect::from_center_size(r.center(), vec2(28.0, 22.0));
            ui.painter().rect_stroke(frame, CornerRadius::same(2), egui::Stroke::new(1.5, m.on_surface_variant), egui::StrokeKind::Inside);
            let x = if value.ends_with("Left") {
                frame.left() + 6.0
            } else if value.ends_with("Right") {
                frame.right() - 6.0
            } else {
                frame.center().x
            };
            let y = if value.starts_with("top") { frame.top() + 6.0 } else { frame.bottom() - 6.0 };
            ui.painter().circle_filled(Pos2::new(x, y), 3.0, m.primary);
            let clicked = resp.clicked();
            resp.on_hover_text(crate::tl!(label));
            if clicked {
                let _ = app.run("table.cellAlign", json!({"value": value}));
                app.canvas.want_focus = false;
            }
        }
    });
}

/// Shading: colour swatches for the caret's cells, and a swatch that removes the shading.
/// The colours are our own picks, not Word's palette.
fn shading_swatches(app: &mut WordApp, ui: &mut Ui, m: Md) {
    // (tooltip, RRGGBB or None for no shading)
    const SWATCHES: [(&str, Option<&str>); 6] = [
        ("Soft Yellow", Some("FFF2CC")),
        ("Soft Blue", Some("DDEBF7")),
        ("Soft Green", Some("E2EFDA")),
        ("Soft Grey", Some("EDEDED")),
        ("Soft Orange", Some("FCE4D6")),
        ("No Shading", None),
    ];
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add_space(12.0);
        for (label, colour) in SWATCHES {
            let (r, resp) = ui.allocate_exact_size(vec2(ICON_BTN, ICON_BTN), Sense::click());
            paint_state(ui, r, 8, m.on_surface, &resp);
            let c = Rect::from_center_size(r.center(), vec2(30.0, 30.0));
            match colour.and_then(|h| u32::from_str_radix(h, 16).ok()) {
                Some(v) => {
                    ui.painter().rect_filled(c, CornerRadius::same(4), hex(v));
                    ui.painter().rect_stroke(c, CornerRadius::same(4), egui::Stroke::new(1.0, m.outline_variant), egui::StrokeKind::Inside);
                }
                None => {
                    ui.painter().rect_stroke(c, CornerRadius::same(4), egui::Stroke::new(1.5, m.on_surface_variant), egui::StrokeKind::Inside);
                    ui.painter().line_segment([c.left_bottom(), c.right_top()], egui::Stroke::new(1.5, m.on_surface_variant));
                }
            }
            let clicked = resp.clicked();
            resp.on_hover_text(crate::tl!(label));
            if clicked {
                let value = colour.map_or(Value::Null, |h| json!(h));
                let _ = app.run("table.shading", json!({"color": value}));
                app.canvas.want_focus = false;
            }
        }
    });
}

/// The tab picker: a rounded field showing the current tab, and a menu of the tabs under it.
/// Picking one closes the menu and shows that tab's rows.
fn tab_dropdown(ui: &mut Ui, m: Md, ctx: &Context, tab: usize, shown: [bool; 8]) {
    let label = crate::tl!(TABS[tab]).to_string();
    let font = medium(16.0);
    let text_w = ui.ctx().fonts_mut(|f| f.layout_no_wrap(label.clone(), font.clone(), m.on_surface).size().x);
    let (r, resp) = ui.allocate_exact_size(vec2(text_w + 56.0, 40.0), Sense::click());
    ui.painter().rect_filled(r, CornerRadius::same(20), m.surface_high);
    paint_state(ui, r, 20, m.on_surface, &resp);
    ui.painter().text(Pos2::new(r.left() + 16.0, r.center().y), Align2::LEFT_CENTER, label, font, m.on_surface);
    // The field's arrow: a small triangle pointing down.
    let c = Pos2::new(r.right() - 20.0, r.center().y);
    let arrow = vec![c + vec2(-6.0, -3.0), c + vec2(6.0, -3.0), c + vec2(0.0, 3.0)];
    ui.painter().add(egui::Shape::convex_polygon(arrow, m.on_surface_variant, egui::Stroke::NONE));
    egui::Popup::menu(&resp).show(|ui| {
        ui.set_min_width(200.0);
        for (i, name) in TABS.iter().enumerate() {
            if !shown[i] {
                continue;
            }
            let selected = i == tab;
            let (row, row_resp) = ui.allocate_exact_size(vec2(ui.available_width(), 48.0), Sense::click());
            let ink = if selected { m.on_primary_container } else { m.on_surface };
            if selected {
                ui.painter().rect_filled(row, CornerRadius::same(24), m.primary_container);
            } else {
                paint_state(ui, row, 24, m.on_surface, &row_resp);
            }
            let font = if selected { medium(16.0) } else { regular(16.0) };
            ui.painter().text(Pos2::new(row.left() + 16.0, row.center().y), Align2::LEFT_CENTER, crate::tl!(name), font, ink);
            if row_resp.clicked() {
                set_menu(ctx, Some(i));
                ui.close();
            }
        }
    });
}

/// The icon of a menu row, by its command.
fn row_icon(id: &str) -> &'static str {
    match id {
        "edit.paste" => "paste",
        "format.highlight" => "highlight",
        "ui.colors" => "fontcolor",
        "format.clear" => "clear",
        "file.save" => "save",
        "edit.repeat" => "redo",
        "insert.table" => "table",
        id if id.starts_with("table.") => "table",
        "insert.picture" => "picture",
        id if id.starts_with("picture.") || id.starts_with("arrange.") || id == "ui.crop" || id == "text.delete" => "picture",
        "insert.shape" => "shapes",
        "layout.margins" => "margins",
        "review.spelling" => "find",
        "review.comments" => "comment",
        "draw.pen" => "pen",
        "draw.highlighter" => "highlight",
        "draw.eraser" => "clear",
        "draw.lasso" => "select",
        "draw.select" => "more",
        "view.webLayout" => "pen",
        "phone.printView" => "printLayout",
        "view.zoomIn" | "view.zoomOut" | "view.onePage" | "view.multiplePages" | "view.pageWidth" => "find",
        "view.draft" => "pen",
        "view.marks" => "more",
        "view.darkMode" => "more",
        "review.wordCount" => "more",
        "review.trackChanges" => "pen",
        id if id.starts_with("review.") && id.contains("Change") => "pen",
        id if id.starts_with("review.") && id.contains("Comment") => "comment",
        _ => "more",
    }
}

/// Whether a menu row opens a sub-menu or a dialog: those rows show a chevron, as Word's do.
fn row_chevron(id: &str) -> bool {
    matches!(
        id,
        "format.highlight"
            | "ui.colors"
            | "layout.margins"
            | "layout.orientation"
            | "layout.size"
            | "layout.columns"
            | "layout.break"
            | "review.comments"
    )
}

/// A Material list item: 48 pt (the Android minimum touch target; was 56 pt), an icon, the label
/// at 16 pt, a chevron for rows that open more, and a state layer on hover and press.
fn list_row(ui: &mut Ui, m: Md, label: &str, icon: &str, chevron: bool) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), LIST_ROW_H), Sense::click());
    paint_state(ui, r, 0, m.on_surface, &resp);
    let mid = r.center().y;
    icons::paint(ui.painter(), Rect::from_center_size(Pos2::new(r.left() + 36.0, mid), vec2(24.0, 24.0)), icon, m.on_surface_variant, m.primary);
    ui.painter().text(Pos2::new(r.left() + 72.0, mid), Align2::LEFT_CENTER, crate::tl!(label), regular(16.0), m.on_surface);
    if chevron {
        let c = Pos2::new(r.right() - 28.0, mid);
        let stroke = egui::Stroke::new(2.0, m.on_surface_variant);
        ui.painter().line_segment([c + vec2(-3.0, -6.0), c + vec2(3.0, 0.0)], stroke);
        ui.painter().line_segment([c + vec2(3.0, 0.0), c + vec2(-3.0, 6.0)], stroke);
    }
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phone_layout_starts_below_the_breakpoint() {
        assert!(is_narrow(390.0));
        assert!(is_narrow(MAX_WIDTH - 1.0));
        assert!(!is_narrow(MAX_WIDTH));
        assert!(!is_narrow(1440.0));
        // A phone in landscape is decided by its shorter side (the 390 pt height), not its width.
        assert!(is_narrow(390.0));
    }

    #[test]
    fn table_tab_rows_are_table_commands() {
        assert_eq!(TABS[TABLE_TAB], "Table");
        for (label, id) in rows(TABLE_TAB) {
            assert!(id.starts_with("table."), "{label} runs {id}");
        }
    }

    #[test]
    fn picture_tab_rows_are_picture_commands() {
        assert_eq!(TABS[PICTURE_TAB], "Picture");
        for (label, id) in rows(PICTURE_TAB) {
            let ok = id.starts_with("picture.") || id.starts_with("arrange.") || matches!(*id, "ui.crop" | "text.delete");
            assert!(ok, "{label} runs {id}");
        }
    }

    #[test]
    fn every_tab_has_rows() {
        for tab in 0..TABS.len() {
            assert!(!rows(tab).is_empty(), "tab {} has no rows", TABS[tab]);
        }
    }
}
