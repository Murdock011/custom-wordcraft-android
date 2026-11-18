//! The Tiles style of the phone's toolbar sheets (the shared phone shell, plan parts 1.5 and 1.6), the
//! default: a fixed tab bar at the bottom (Home, Insert, Layout, Review, View, More), and a sheet of
//! tiles above it for the open tab, grouped in sections. A tile is an icon above a label of up to two
//! lines; the label and whether the tile is enabled come from the engine's command where there is
//! one. A tile runs what the List style's row with the same id runs (`pick_row`), so both styles do
//! the same thing. While the keyboard is up the floating formatting bar (the keyboard bar) shows
//! instead of the tab bar.

use egui::{Align2, Context, CornerRadius, Margin, Panel, Pos2, Rect, Sense, Stroke, Ui, vec2};
use serde_json::{Value, json};

use super::{
    DRAW_TAB, ICON_BTN, Md, PICTURE_TAB, TABLE_TAB, TABS, align_tiles, draw_controls, format_page, md, menu_tab, pick_row, picture_controls,
    row_icon, set_menu, shading_swatches, shown_tabs, sub,
};
use crate::WordApp;
use crate::icons;
use crate::theme::{medium, regular};

/// The narrowest a tile may be, and its height (an icon over two lines of label).
pub(super) const TILE_MIN_W: f32 = 76.0;
pub(super) const TILE_H: f32 = 76.0;
/// The tab bar, the narrowest a tab slot may be, and a sheet's header.
pub(super) const TAB_BAR_H: f32 = 64.0;
pub(super) const TAB_MIN_W: f32 = 52.0;
pub(super) const SHEET_HEADER_H: f32 = 56.0;
/// The margin at a sheet's sides.
pub(super) const GUTTER: f32 = 16.0;
/// The share of the window a sheet takes at most, and in a window under 500 pt tall.
const SHEET_MAX: f32 = 0.50;
const SHEET_MAX_SHORT: f32 = 0.60;
const _: () = assert!(TILE_MIN_W >= 48.0 && TILE_H >= 48.0 && TAB_BAR_H >= 48.0 && TAB_MIN_W >= 48.0 && SHEET_HEADER_H >= 48.0);

/// The tab bar's tabs (indexes into `TABS`), then More.
const BAR_TABS: [usize; 5] = [0, 1, 2, 3, 4];
/// The menu state of the More sheet: past the tabs.
pub(super) const MORE: usize = 100;

/// One command in a sheet: the id `pick_row` runs, the label when the engine's would be wrong or
/// there is no engine command (a dialog, a page), and whether the sheet stays open after a tap.
#[derive(Clone, Copy)]
pub(super) struct Item {
    pub id: &'static str,
    pub label: Option<&'static str>,
    pub keep: bool,
}

const fn fmt(id: &'static str) -> Item {
    Item { id, label: None, keep: true }
}
const fn run(id: &'static str) -> Item {
    Item { id, label: None, keep: false }
}
const fn named(id: &'static str, label: &'static str, keep: bool) -> Item {
    Item { id, label: Some(label), keep }
}

/// A section of a sheet: a heading and its tiles.
pub(super) struct Section {
    pub heading: &'static str,
    pub items: &'static [Item],
}

const HOME: &[Section] = &[
    Section {
        heading: "Font",
        items: &[
            fmt("format.bold"),
            fmt("format.italic"),
            fmt("format.underline"),
            fmt("format.strikethrough"),
            fmt("format.subscript"),
            fmt("format.superscript"),
            named("format.highlight", "Highlight…", true),
            named("ui.colors", "Font Color…", true),
            fmt("format.clear"),
            named("ui.font", "Font…", false),
        ],
    },
    Section {
        heading: "Paragraph",
        items: &[
            fmt("para.alignLeft"),
            fmt("para.alignCenter"),
            fmt("para.alignRight"),
            fmt("para.justify"),
            fmt("para.bullets"),
            fmt("para.numbering"),
            fmt("para.outdent"),
            fmt("para.indent"),
            named("ui.paragraph", "Paragraph…", false),
        ],
    },
    Section {
        heading: "Clipboard and Editing",
        items: &[
            run("edit.paste"),
            run("edit.cut"),
            run("edit.copy"),
            run("select.all"),
            named("dialog:find", "Find…", false),
            named("dialog:replace", "Replace…", false),
            run("edit.repeat"),
        ],
    },
];

const INSERT: &[Section] = &[
    Section { heading: "Pages and Tables", items: &[named("insert.table", "Table…", false), run("insert.pageBreak"), run("insert.blankPage")] },
    Section {
        heading: "Objects",
        items: &[run("insert.picture"), run("insert.shape"), run("insert.textBox"), named("dialog:link", "Link…", false), run("insert.equation")],
    },
    Section {
        heading: "Document Parts",
        items: &[run("review.newComment"), run("insert.header"), run("insert.pageNumber"), run("references.footnote"), run("references.endnote")],
    },
];

const LAYOUT: &[Section] = &[Section {
    heading: "Page Setup",
    items: &[fmt("layout.margins"), fmt("layout.orientation"), fmt("layout.size"), fmt("layout.columns"), fmt("layout.break")],
}];

const REVIEW: &[Section] = &[
    Section { heading: "Proofing", items: &[run("review.spelling"), run("review.wordCount"), run("review.readAloud")] },
    Section {
        heading: "Tracking",
        items: &[
            fmt("review.trackChanges"),
            fmt("review.previousChange"),
            fmt("review.nextChange"),
            fmt("review.accept"),
            fmt("review.reject"),
            fmt("review.acceptAll"),
            fmt("review.rejectAll"),
        ],
    },
    Section {
        heading: "Comments",
        items: &[run("review.newComment"), named("review.comments", "Show Comments", true), fmt("review.previousComment"), fmt("review.nextComment")],
    },
];

const VIEW: &[Section] = &[
    Section {
        heading: "Views",
        items: &[named("view.webLayout", "Edit View", false), named("phone.printView", "Print View", false), run("view.readMode"), run("view.draft")],
    },
    Section {
        heading: "Zoom",
        items: &[fmt("view.zoomIn"), fmt("view.zoomOut"), fmt("view.onePage"), fmt("view.multiplePages"), fmt("view.pageWidth")],
    },
    Section { heading: "Show", items: &[fmt("view.marks"), fmt("view.darkMode")] },
];

/// The sections of tab `tab`. Table, Picture and Draw keep their rows as one section, with their own
/// controls after it.
pub(super) fn sections(tab: usize) -> &'static [Section] {
    match tab {
        0 => HOME,
        1 => INSERT,
        2 => LAYOUT,
        3 => REVIEW,
        4 => VIEW,
        _ => &[],
    }
}

/// The items of a tab with no sections of its own (Table, Picture, Draw): its rows, in order.
fn row_items(tab: usize) -> Vec<Item> {
    super::rows(tab).iter().map(|(label, id)| Item { id, label: Some(label), keep: true }).collect()
}

/// The label a tile shows, before translation: its own, else the engine command's, else the id.
pub(super) fn label(app: &WordApp, item: &Item) -> String {
    item.label.map(str::to_string).or_else(|| app.session.registry.get(item.id).map(|s| s.label.to_string())).unwrap_or_else(|| item.id.to_string())
}

/// Whether the tile can run now: the engine command's `enabled`, or always for a page or a dialog.
pub(super) fn enabled(app: &WordApp, item: &Item) -> bool {
    app.session.registry.get(item.id).is_none() || crate::widgets::enabled(app, item.id)
}

/// The formatting at the selection, read once a frame for the tiles' on state.
pub(super) struct State {
    format: Value,
    align: Option<wordcraft_doc::Align>,
}

pub(super) fn state(app: &mut WordApp) -> State {
    let format = app.session.run("format.state", &json!({})).unwrap_or_default();
    let align = app.session.doc.para_at(&app.session.sel.focus).map(|_| {
        let rp = wordcraft_engine::cmd::para::selection_para(&app.session);
        rp.align.visual(rp.bidi)
    });
    State { format, align }
}

/// Whether a toggle tile is on at the selection.
pub(super) fn is_on(app: &WordApp, st: &State, id: &str) -> bool {
    use wordcraft_doc::Align as A;
    let flag = |k: &str| st.format.get(k).and_then(Value::as_bool).unwrap_or(false);
    match id {
        "format.bold" => flag("bold"),
        "format.italic" => flag("italic"),
        "format.underline" => flag("underline"),
        "format.strikethrough" => flag("strike"),
        "format.subscript" => flag("subscript"),
        "format.superscript" => flag("superscript"),
        "para.alignLeft" => st.align == Some(A::Left),
        "para.alignCenter" => st.align == Some(A::Center),
        "para.alignRight" => st.align == Some(A::Right),
        "para.justify" => st.align == Some(A::Justify),
        "view.marks" => app.session.view.marks,
        "view.darkMode" => app.session.view.dark_mode,
        _ => false,
    }
}

/// Runs a tile: what the List style's row with the same id runs. A dialog id opens that dialog.
/// The sheet stays open after a formatting tile, and closes after one that inserts or opens a dialog.
pub(super) fn tap(app: &mut WordApp, ctx: &Context, item: &Item) {
    if let Some(name) = item.id.strip_prefix("dialog:") {
        let _ = app.run("ui.dialog", json!({"name": name}));
    } else {
        pick_row(app, ctx, item.id);
    }
    // A page (a colour grid, margins) opened in the sheet keeps it open; so does a format.
    if !item.keep && sub(ctx).is_none() {
        set_menu(ctx, None);
    }
}

/// The height a sheet takes in a window `h` tall.
fn sheet_height(h: f32) -> f32 {
    let share = if h < 500.0 { SHEET_MAX_SHORT } else { SHEET_MAX };
    (h * share).max(SHEET_HEADER_H + 2.0 * TILE_H)
}

fn tab_bar_rect_id() -> egui::Id {
    egui::Id::unique("phone_tab_bar_rect")
}

/// Whether a press at `pos` is on the tab bar (it never counts as outside the sheet). Only a tab bar
/// drawn on the last frame or this one counts: the List style, a dialog sheet or the keyboard hide it.
pub(super) fn on_tab_bar(ctx: &Context, pos: Option<Pos2>) -> bool {
    let now = ctx.cumulative_frame_nr();
    let rect = ctx.data(|d| d.get_temp::<(Rect, u64)>(tab_bar_rect_id())).filter(|(_, at)| at + 1 >= now).map(|(r, _)| r);
    matches!((pos, rect), (Some(p), Some(r)) if r.contains(p))
}

/// A tile's icon: the List row's icon, or the ribbon's for the commands the List style has no row for.
pub(super) fn tile_icon(id: &str) -> &'static str {
    match id {
        "format.bold" => "bold",
        "format.italic" => "italic",
        "format.underline" => "underline",
        "format.strikethrough" => "strike",
        "format.subscript" => "subscript",
        "format.superscript" => "superscript",
        "para.alignLeft" => "alignLeft",
        "para.alignCenter" => "alignCenter",
        "para.alignRight" => "alignRight",
        "para.justify" => "justify",
        "para.bullets" => "bullets",
        "para.numbering" => "numbering",
        "para.outdent" => "outdent",
        "para.indent" => "indentLeft",
        "edit.cut" => "cut",
        "edit.copy" => "copy",
        "select.all" => "select",
        "dialog:find" => "find",
        "dialog:replace" => "replace",
        "dialog:link" => "link",
        "insert.pageBreak" => "pageBreak",
        "review.newComment" => "newComment",
        "review.acceptAll" => "accept",
        "review.rejectAll" => "reject",
        "view.marks" => "pilcrow",
        "view.darkMode" => "darkMode",
        "view.zoomIn" => "zoomIn",
        "view.zoomOut" => "zoomOut",
        "view.onePage" => "onePage",
        "view.multiplePages" => "multiplePages",
        "view.pageWidth" => "pageWidth",
        "view.readMode" => "readMode",
        "view.draft" => "draft",
        "view.webLayout" => "webLayout",
        "phone.printView" => "printLayout",
        _ => row_icon(id),
    }
}

/// The tab icon of a tab-bar slot.
fn tab_icon(tab: usize) -> &'static str {
    match tab {
        0 => "fontdialog",
        1 => "plus",
        2 => "margins",
        3 => "trackChanges",
        4 => "zoom",
        _ => "more",
    }
}

/// The bottom of the screen in the Tiles style: the tab bar, and the open sheet above it.
pub(super) fn bottom(app: &mut WordApp, ui: &mut Ui, open: Option<usize>) {
    let ctx = ui.ctx().clone();
    if open != menu_tab(&ctx) {
        set_menu(&ctx, open);
    }
    tab_bar(app, ui);
    match open {
        Some(MORE) => more_sheet(app, ui),
        Some(super::tools::TOOLS) => tools_sheet(app, ui),
        Some(tab) => tab_sheet(app, ui, tab),
        None => {}
    }
}

/// The tab bar: Home, Insert, Layout, Review, View, More. The open tab is drawn on; tapping it
/// again closes its sheet; another tab switches it. A dot on More while Table or Picture applies.
fn tab_bar(app: &mut WordApp, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let m = md(&ctx);
    let open = menu_tab(&ctx);
    let shown = shown_tabs(app);
    let contextual = shown.get(TABLE_TAB).copied().unwrap_or(false) || shown.get(PICTURE_TAB).copied().unwrap_or(false);
    let on = match open {
        Some(t) if BAR_TABS.contains(&t) => Some(t),
        Some(_) => Some(MORE),
        None => None,
    };
    let panel = Panel::bottom("phone_tab_bar").exact_size(TAB_BAR_H).frame(egui::Frame::NONE.fill(m.surface)).show(ui, |ui| {
        let bar = ui.max_rect();
        ui.painter().hline(bar.x_range(), bar.top(), Stroke::new(1.0, m.outline_variant));
        let slots: Vec<usize> = BAR_TABS.iter().copied().chain([MORE]).collect();
        let w = bar.width() / slots.len() as f32;
        for (i, &slot) in slots.iter().enumerate() {
            let cell = Rect::from_min_size(Pos2::new(bar.left() + i as f32 * w, bar.top()), vec2(w, bar.height()));
            let label = if slot == MORE { "More" } else { TABS[slot] };
            let is_on = on == Some(slot);
            let resp = ui.interact(cell, ui.scope_id().with(("phone_tab", slot)), Sense::click());
            let pill = Rect::from_center_size(Pos2::new(cell.center().x, bar.top() + 22.0), vec2(56.0_f32.min(w - 4.0), 32.0));
            if is_on {
                ui.painter().rect_filled(pill, 16.0, m.primary_container);
            } else if resp.is_pointer_button_down_on() {
                ui.painter().rect_filled(pill, 16.0, m.outline_variant);
            }
            let ink = if is_on { m.on_primary_container } else { m.on_surface_variant };
            icons::paint(ui.painter(), Rect::from_center_size(pill.center(), vec2(24.0, 24.0)), tab_icon(slot), ink, m.primary);
            if slot == MORE && contextual {
                ui.painter().circle_filled(Pos2::new(pill.center().x + 13.0, pill.top() + 6.0), 4.0, m.primary);
            }
            let font = if is_on { medium(12.0) } else { regular(12.0) };
            ui.painter().text(
                Pos2::new(cell.center().x, bar.top() + 50.0),
                Align2::CENTER_CENTER,
                crate::tl!(label),
                font,
                if is_on { m.primary } else { m.on_surface_variant },
            );
            resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, is_on, crate::tl!(label)));
            if resp.clicked() {
                super::set_sub(&ctx, None);
                if is_on {
                    set_menu(&ctx, None);
                } else {
                    super::open_menu(app, &ctx, slot);
                }
            }
        }
    });
    let now = ctx.cumulative_frame_nr();
    ctx.data_mut(|d| d.insert_temp(tab_bar_rect_id(), (panel.response.rect, now)));
}

/// A sheet's frame above the tab bar: the header (title at the left, Close at the right) and a
/// scrolling body. A page (`sub`) draws its own header, as in the List style.
fn frame(app: &mut WordApp, ui: &mut Ui, title: &str, body: impl FnOnce(&mut WordApp, &mut Ui, Md)) {
    let ctx = ui.ctx().clone();
    let m = md(&ctx);
    // The sheet takes the input: while it is open the page must not hold the keyboard.
    app.canvas.want_focus = false;
    app.canvas.focused = false;
    let height = sheet_height(ctx.content_rect().height() - TAB_BAR_H);
    let panel = Panel::bottom("phone_tiles_sheet")
        .exact_size(height)
        .frame(egui::Frame::NONE.fill(m.surface_low).inner_margin(Margin::symmetric(GUTTER as i8, 0)))
        .show(ui, |ui| {
            let full = ui.max_rect();
            ui.painter().hline(full.x_range().expand(GUTTER), full.top(), Stroke::new(1.0, m.outline_variant));
            if let Some(s) = sub(&ctx) {
                return format_page(app, ui, m, &ctx, s);
            }
            ui.allocate_ui_with_layout(vec2(ui.available_width(), SHEET_HEADER_H), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.set_min_height(SHEET_HEADER_H);
                let w = (ui.available_width() - ICON_BTN + 12.0).max(0.0);
                let (r, _) = ui.allocate_exact_size(vec2(w, ICON_BTN), Sense::hover());
                ui.painter().text(Pos2::new(r.left(), r.center().y), Align2::LEFT_CENTER, title, regular(20.0), m.on_surface);
                if super::icon_button(ui, m, "close", "Close").clicked() {
                    set_menu(&ctx, None);
                }
            });
            egui::ScrollArea::vertical().id_salt(("phone_tiles_body", title.to_string())).show(ui, |ui| body(app, ui, m));
        });
    ctx.data_mut(|d| d.insert_temp(super::sheet_rect_id(), panel.response.rect));
}

/// A tab's sheet: Home's font row, then each section's heading and tiles. Table, Picture and Draw
/// show their rows as tiles, then their own controls (cell alignment and shading, the picture's
/// size and alt text, the pen's colour and thickness).
fn tab_sheet(app: &mut WordApp, ui: &mut Ui, tab: usize) {
    let title = crate::tl!(TABS.get(tab).copied().unwrap_or("")).to_string();
    frame(app, ui, &title, |app, ui, m| {
        let ctx = ui.ctx().clone();
        let st = state(app);
        if tab == 0 {
            super::font_name_row(app, ui, m, &ctx);
        }
        let own: Vec<Item>;
        let sections: Vec<(&str, &[Item])> = if sections(tab).is_empty() {
            own = row_items(tab);
            vec![("", own.as_slice())]
        } else {
            sections(tab).iter().map(|s| (s.heading, s.items)).collect()
        };
        for (heading, items) in sections {
            if !heading.is_empty() {
                section_heading(ui, m, crate::tl!(heading));
            }
            tiles(app, ui, m, &ctx, &st, items);
        }
        if tab == TABLE_TAB {
            section_heading(ui, m, crate::tl!("Cell Alignment"));
            align_tiles(app, ui, m);
            section_heading(ui, m, crate::tl!("Shading"));
            shading_swatches(app, ui, m);
        }
        if tab == PICTURE_TAB {
            picture_controls(app, ui, m);
        }
        if tab == DRAW_TAB {
            draw_controls(app, ui, m);
        }
        ui.add_space(GUTTER);
    });
}

/// The More sheet: Search, the contextual tabs that apply now (Table, Picture) under Selection, then
/// Draw.
fn more_sheet(app: &mut WordApp, ui: &mut Ui) {
    let title = crate::tl!("More").to_string();
    frame(app, ui, &title, |app, ui, m| {
        let ctx = ui.ctx().clone();
        if super::list_row(ui, m, crate::tl!("Search"), "find", false).clicked() {
            super::tools::open_search(&ctx);
        }
        let shown = shown_tabs(app);
        if shown.get(TABLE_TAB).copied().unwrap_or(false) || shown.get(PICTURE_TAB).copied().unwrap_or(false) {
            section_heading(ui, m, crate::tl!("Selection"));
            for (tab, icon) in [(TABLE_TAB, "table"), (PICTURE_TAB, "picture")] {
                if shown.get(tab).copied().unwrap_or(false) && super::list_row(ui, m, TABS[tab], icon, true).clicked() {
                    set_menu(&ctx, Some(tab));
                }
            }
        }
        section_heading(ui, m, crate::tl!("More tabs"));
        if super::list_row(ui, m, crate::tl!(TABS[DRAW_TAB]), "pen", true).clicked() {
            set_menu(&ctx, Some(DRAW_TAB));
        }
        if super::list_row(ui, m, crate::tl!("All tools"), "more", true).clicked() {
            super::tools::open(&ctx, super::tools::Tool::Tabs);
        }
    });
}

/// The tallest All tools may be: its lists are long.
const DIALOG_MAX: f32 = 0.85;

/// All tools in a sheet: a header with Back, the screen's title and Close, and the screen's list.
pub(super) fn tools_sheet(app: &mut WordApp, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let m = md(&ctx);
    app.canvas.want_focus = false;
    app.canvas.focused = false;
    let height = (ctx.content_rect().height() * DIALOG_MAX).max(SHEET_HEADER_H + TILE_H);
    let title = crate::tl!(&super::tools::title(&ctx)).to_string();
    let panel = Panel::bottom("phone_tools_sheet")
        .exact_size(height)
        .frame(egui::Frame::NONE.fill(m.surface_low).inner_margin(Margin::symmetric(GUTTER as i8, 0)))
        .show(ui, |ui| {
            let full = ui.max_rect();
            ui.painter().hline(full.x_range().expand(GUTTER), full.top(), Stroke::new(1.0, m.outline_variant));
            ui.allocate_ui_with_layout(vec2(ui.available_width(), SHEET_HEADER_H), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.set_min_height(SHEET_HEADER_H);
                if super::icon_button(ui, m, "chevronLeft", "Back").clicked() {
                    super::tools::back(&ctx);
                }
                let w = (ui.available_width() - ICON_BTN).max(0.0);
                let (r, _) = ui.allocate_exact_size(vec2(w, ICON_BTN), Sense::hover());
                ui.painter().text(Pos2::new(r.left() + 4.0, r.center().y), Align2::LEFT_CENTER, &title, medium(20.0), m.on_surface);
                if super::icon_button(ui, m, "close", "Close").clicked() {
                    set_menu(&ctx, None);
                }
            });
            egui::ScrollArea::vertical().id_salt(("phone_tools_body", title.clone())).show(ui, |ui| super::tools::body(app, ui, m));
        });
    ctx.data_mut(|d| d.insert_temp(super::sheet_rect_id(), panel.response.rect));
}

/// A section heading, small and in the primary colour.
fn section_heading(ui: &mut Ui, m: Md, text: &str) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::hover());
    ui.painter().text(Pos2::new(r.left(), r.bottom() - 8.0), Align2::LEFT_BOTTOM, text, medium(12.0), m.primary);
}

/// A grid of tiles: the sheet's width divided into whole tiles of at least [`TILE_MIN_W`].
fn tiles(app: &mut WordApp, ui: &mut Ui, m: Md, ctx: &Context, st: &State, items: &[Item]) {
    let width = ui.available_width();
    let cols = ((width / TILE_MIN_W).floor() as usize).max(1);
    let w = width / cols as f32;
    for row in items.chunks(cols) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for item in row {
                let text = crate::tl!(&label(app, item)).to_string();
                let on = is_on(app, st, item.id);
                let enabled = enabled(app, item);
                if tile(ui, m, w, tile_icon(item.id), &text, on, enabled).clicked() {
                    tap(app, ctx, item);
                }
            }
        });
    }
}

/// One tile, `w` wide: the icon, then the label on up to two lines, broken only at spaces (a word too
/// wide for a line is drawn smaller, down to 9 pt). An "on" tile is tinted; a disabled one greyed.
fn tile(ui: &mut Ui, m: Md, w: f32, icon: &str, text: &str, on: bool, enabled: bool) -> egui::Response {
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (r, resp) = ui.allocate_exact_size(vec2(w, TILE_H), sense);
    let body = r.shrink(2.0);
    if on {
        ui.painter().rect_filled(body, CornerRadius::same(12), m.primary_container);
    }
    if enabled && resp.is_pointer_button_down_on() {
        ui.painter().rect_filled(body, CornerRadius::same(12), m.outline_variant);
    }
    let ink = if !enabled {
        m.on_surface_variant.gamma_multiply(0.4)
    } else if on {
        m.on_primary_container
    } else {
        m.on_surface
    };
    icons::paint(ui.painter(), Rect::from_center_size(Pos2::new(r.center().x, r.top() + 22.0), vec2(24.0, 24.0)), icon, ink, m.primary);
    let max_w = (w - 4.0).max(1.0);
    let widest = text.split(' ').map(|word| ui.painter().layout_no_wrap(word.to_owned(), regular(12.0), ink).size().x).fold(0.0, f32::max);
    let size = if widest <= max_w { 12.0 } else { (12.0 * max_w / widest).floor().clamp(9.0, 11.0) };
    let mut job = egui::text::LayoutJob::simple(text.to_owned(), regular(size), ink, max_w);
    job.wrap.max_rows = 2;
    job.wrap.break_anywhere = false;
    job.wrap.overflow_character = Some('…');
    job.halign = egui::Align::Center;
    let galley = ui.painter().layout_job(job);
    ui.painter().galley(Pos2::new(r.center().x, r.top() + 40.0), galley, ink);
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, enabled, on, text));
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tile_names_a_command_a_dialog_or_a_page() {
        let s = wordcraft_engine::Session::new(wordcraft_doc::Document::new());
        // Ids `pick_row` and `run` map themselves (pages, dialogs, presets) need no engine command.
        let own = ["ui.colors", "ui.font", "ui.paragraph", "phone.printView", "review.comments"];
        for tab in 0..5 {
            for section in sections(tab) {
                assert!(!section.items.is_empty(), "{} has tiles", section.heading);
                for item in section.items {
                    let known = s.registry.get(item.id).is_some()
                        || item.id.starts_with("dialog:")
                        || own.contains(&item.id)
                        || item.id.starts_with("layout.");
                    assert!(known, "{} names no command", item.id);
                }
            }
        }
    }

    #[test]
    fn every_tile_has_a_real_icon() {
        for tab in 0..5 {
            for section in sections(tab) {
                for item in section.items {
                    assert_ne!(tile_icon(item.id), "more", "{} falls back to the placeholder icon", item.id);
                }
            }
        }
    }

    #[test]
    fn labels_in_a_section_are_distinct() {
        let s = wordcraft_engine::Session::new(wordcraft_doc::Document::new());
        for tab in 0..5 {
            for section in sections(tab) {
                let mut seen = std::collections::HashSet::new();
                for item in section.items {
                    let l = item.label.map(str::to_string).or_else(|| s.registry.get(item.id).map(|c| c.label.to_string())).unwrap_or_default();
                    assert!(seen.insert(l.clone()), "{} repeats {l}", section.heading);
                }
            }
        }
    }
}
