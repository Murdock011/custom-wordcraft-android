//! All tools (the shared phone shell, plan part 1.7): every command of the engine, reachable on a
//! phone. Ported from DeckCraft's and GridCraft's phone tools.
//!
//! The tile sheets are the everyday front page. This is the rest: one screen per ribbon tab, built
//! from the engine's own command list (the session's registry), so a command the engine gains
//! shows up here without a hand-kept table. Tabs and groups come from each command's `location`
//! (`Home › Font`). A row runs its command the way Search does (a command that needs values opens
//! its dialog); a row that needs one value from a short list opens a picker; a row that needs typed
//! values and cannot run without them opens a small form. Search finds any command by name or id.
//!
//! Back closes one level: form or picker, then the tab, then the list of tabs, then back to More.

use egui::{Align2, Context, CornerRadius, Pos2, Rect, Sense, Ui, vec2};
use serde_json::{Value, json};
use wordcraft_engine::CommandSpec;

use super::tiles::{GUTTER, MORE};
use super::{Md, set_menu};
use crate::WordApp;
use crate::icons;
use crate::theme::{medium, regular};

/// The menu state of All tools: past the tabs and More.
pub(super) const TOOLS: usize = 101;
/// A list row, and a row with a second line under its label.
const ROW_H: f32 = 52.0;
/// The primary button of a form.
const TOUCH: f32 = 48.0;

/// The ribbon and contextual tabs, in the order they are listed. A command whose location names
/// none of them is folded into one by [`tab_of`], or listed under "Other".
pub(super) const TABS: [&str; 21] = [
    "Home",
    "Insert",
    "Draw",
    "Design",
    "Layout",
    "References",
    "Mailings",
    "Review",
    "View",
    "Developer",
    "Header & Footer",
    "Table Design",
    "Table Layout",
    "Picture Format",
    "Shape Format",
    "SmartArt Design",
    "Chart Design",
    "Equation",
    "Outlining",
    "File",
    "Other",
];

/// Tabs listed only while the selection calls for them (the ribbon's contextual tabs).
const CONTEXTUAL: [&str; 8] =
    ["Table Design", "Table Layout", "Picture Format", "Shape Format", "SmartArt Design", "Chart Design", "Equation", "Outlining"];

/// Locations with no row: the caret and selection primitives, the title and status bars, and the
/// agents' commands (an AI agent or script needs a computer).
const HIDDEN: [&str; 4] = ["Agents", "Navigation", "Title Bar", "Status Bar"];

/// Commands that only answer a question, and the language-pack installs Search leaves out too.
const QUERIES: [&str; 3] = ["format.state", "proof.installPack", "proof.removePack"];

/// One screen of the tools. The stack kept in egui's memory is the way back.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Tool {
    /// The list of tabs.
    Tabs,
    /// Search every command (the text typed so far).
    Search(String),
    /// One tab's commands.
    Tab(&'static str),
    /// A choice for one command.
    Pick(Pick),
    /// Values to type for one command.
    Form(Form),
}

/// A choice: each option is the parameters to run the command with, and its label.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Pick {
    pub id: &'static str,
    pub label: &'static str,
    pub options: Vec<(Value, String)>,
}

/// Values to type: (name, what the params doc says of it, the text so far).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Form {
    pub id: &'static str,
    pub label: &'static str,
    pub fields: Vec<(String, String, String)>,
}

fn stack_id() -> egui::Id {
    egui::Id::unique("phone_tools_stack")
}

fn search_focus_id() -> egui::Id {
    egui::Id::unique("phone_tools_search_focused")
}

/// The open screens, the top last. Empty while the tools are closed.
pub(super) fn stack(ctx: &Context) -> Vec<Tool> {
    ctx.data(|d| d.get_temp::<Vec<Tool>>(stack_id())).unwrap_or_default()
}

fn set_stack(ctx: &Context, s: Vec<Tool>) {
    ctx.data_mut(|d| d.insert_temp(stack_id(), s));
}

/// Opens the tools at `first` (the list of tabs, Search, or one tab).
pub(crate) fn open(ctx: &Context, first: Tool) {
    let mut s = vec![Tool::Tabs];
    if first != Tool::Tabs {
        s.push(first);
    }
    ctx.data_mut(|d| d.insert_temp(search_focus_id(), false));
    set_stack(ctx, s);
    super::set_sub(ctx, None);
    set_menu(ctx, Some(TOOLS));
}

/// Opens the tools on Search.
pub(crate) fn open_search(ctx: &Context) {
    open(ctx, Tool::Search(String::new()));
}

/// Back one level; from the list of tabs, back to the More sheet.
pub(super) fn back(ctx: &Context) {
    let mut s = stack(ctx);
    s.pop();
    if s.is_empty() {
        set_menu(ctx, Some(MORE));
    }
    set_stack(ctx, s);
}

/// Closes the tools and the sheet.
fn close_all(ctx: &Context) {
    set_stack(ctx, Vec::new());
    set_menu(ctx, None);
}

/// The tab a command is listed under, or `None` for one with no row. Groups outside the ribbon are
/// folded into the tab they belong with.
pub(super) fn tab_of(spec: &CommandSpec) -> Option<&'static str> {
    let first = spec.location.split('›').next().unwrap_or("").trim();
    if HIDDEN.contains(&first) || QUERIES.contains(&spec.id) {
        return None;
    }
    let tab = match first {
        "Quick Access Toolbar" | "Editing" => "Home",
        "Reading" => "View",
        "Zotero" => "References",
        other => other,
    };
    Some(TABS.iter().copied().find(|t| *t == tab && *t != "Other").unwrap_or("Other"))
}

/// The group a command is listed under within its tab (the second part of its location).
fn group_of(spec: &CommandSpec) -> Option<&'static str> {
    spec.location.split('›').nth(1).map(str::trim).filter(|g| !g.is_empty())
}

/// The commands listed under `tab`, grouped (in the order each group first appears), each group in
/// the engine's order.
pub(super) fn tab_specs(app: &WordApp, tab: &str) -> Vec<CommandSpec> {
    let mine: Vec<CommandSpec> = app.session.registry.all().iter().filter(|s| tab_of(s) == Some(tab)).cloned().collect();
    let mut groups: Vec<Option<&str>> = Vec::new();
    for s in &mine {
        if !groups.contains(&group_of(s)) {
            groups.push(group_of(s));
        }
    }
    groups.into_iter().flat_map(|g| mine.iter().filter(move |s| group_of(s) == g).cloned()).collect()
}

/// The contextual tabs that apply now.
fn contextual_now(app: &WordApp) -> Vec<&'static str> {
    crate::ribbon::contextual_tabs(&app.session)
}

/// The tabs to list now: every tab with commands, a contextual one only while it applies.
fn available_tabs(app: &WordApp) -> Vec<&'static str> {
    let now = contextual_now(app);
    TABS.iter().copied().filter(|t| (!CONTEXTUAL.contains(t) || now.contains(t)) && !tab_specs(app, t).is_empty()).collect()
}

// ---------------------------------------------------------------- reading a params doc

/// One parameter in a params doc: its name, whether it may be left out, and what the doc says of it.
#[derive(Debug, PartialEq)]
pub(super) struct Param {
    pub name: String,
    pub optional: bool,
    pub about: String,
}

/// Splits `s` at `sep` where it is not inside quotes or brackets.
fn split_top(s: &str, sep: char) -> Vec<String> {
    let mut out = vec![String::new()];
    let (mut depth, mut quote) = (0i32, false);
    for c in s.chars() {
        match c {
            '"' => quote = !quote,
            '{' | '[' | '(' if !quote => depth += 1,
            '}' | ']' | ')' if !quote => depth -= 1,
            _ => {}
        }
        if c == sep && depth == 0 && !quote {
            out.push(String::new());
        } else if let Some(last) = out.last_mut() {
            last.push(c);
        }
    }
    out
}

/// The name of a `name?: about` part, if it starts with one.
fn param_name(part: &str) -> Option<(&str, bool, &str)> {
    let (head, about) = part.split_once(':').map_or((part, ""), |(h, a)| (h, a));
    let head = head.trim();
    let optional = head.ends_with('?');
    let name = head.trim_end_matches('?');
    (!name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')).then_some((name, optional, about.trim()))
}

/// The parameters of a params doc such as `{range?, preset: bottom|top, color?}`. Where a part offers
/// alternative names (`height: 20 | points: 15`) the first is taken. Text after `→` is the answer.
pub(super) fn params(doc: &str) -> Vec<Param> {
    let doc = doc.split('→').next().unwrap_or("").trim();
    let inner = doc.strip_prefix('{').and_then(|d| d.strip_suffix('}')).unwrap_or("");
    let mut out = Vec::new();
    for part in split_top(inner, ',') {
        // `a: x | b: y` names alternatives; `a: x|y` or `a: "x" | "y"` lists values for one name.
        let first = split_top(&part, '|').into_iter().fold(String::new(), |acc, seg| {
            if !acc.is_empty() && param_name(&seg).is_some_and(|(_, _, about)| !about.is_empty()) && seg.contains(':') {
                return acc;
            }
            if acc.is_empty() { seg } else { format!("{acc}|{seg}") }
        });
        if let Some((name, optional, about)) = param_name(&first) {
            out.push(Param { name: name.to_string(), optional, about: about.to_string() });
        }
    }
    out
}

/// The parameters a command cannot run without.
pub(super) fn required(doc: &str) -> Vec<Param> {
    params(doc).into_iter().filter(|p| !p.optional).collect()
}

/// Options spelled out in a doc: `a|b|c` when every one is a plain word and the list is not cut short.
pub(super) fn spelled_out(about: &str) -> Option<Vec<String>> {
    if about.contains('…') || about.contains("...") {
        return None;
    }
    let first = about.trim_matches('"').split([' ', ',', '}', '(']).next().unwrap_or("");
    let parts: Vec<&str> = first.split('|').map(|p| p.trim_matches('"')).collect();
    let plain = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    (parts.len() >= 2 && parts.iter().all(|p| plain(p))).then(|| parts.into_iter().map(str::to_string).collect())
}

/// Words made readable: `pageWidth` becomes "Page width".
pub(super) fn readable(s: &str) -> String {
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if c == '-' || c == '_' {
            out.push(' ');
        } else if c.is_uppercase() && i > 0 {
            out.push(' ');
            out.extend(c.to_lowercase());
        } else if i == 0 {
            out.extend(c.to_uppercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// A typed value as JSON: a number, true/false, a list or object when it reads as one, else text.
pub(super) fn parse_value(text: &str) -> Value {
    let t = text.trim();
    serde_json::from_str::<Value>(t).unwrap_or_else(|_| Value::String(t.to_string()))
}

/// The options the phone offers for `param` of command `id`: the lists the sheets use, else values
/// spelled out in the doc.
pub(super) fn choices(id: &str, param: &str, about: &str) -> Option<Vec<(Value, String)>> {
    let list: Vec<(Value, String)> = match (id, param) {
        ("format.size", "size") => wordcraft_engine::cmd::format::SIZES.iter().map(|s| (json!({"size": s}), format!("{s}"))).collect(),
        _ => spelled_out(about)?.into_iter().map(|v| (json!({ param: v.clone() }), readable(&v))).collect(),
    };
    (!list.is_empty()).then_some(list)
}

/// What tapping a command does.
enum Plan {
    /// Run it as Search does (its dialog opens when it needs values).
    Run,
    /// Choose its one value from a list.
    Pick(Pick),
    /// Type its values, if running it as Search does fails.
    Form(Form),
}

fn plan(spec: &CommandSpec) -> Plan {
    let req = required(spec.params);
    if req.is_empty() {
        return Plan::Run;
    }
    if let [one] = req.as_slice()
        && let Some(options) = choices(spec.id, &one.name, &one.about)
    {
        return Plan::Pick(Pick { id: spec.id, label: spec.label, options });
    }
    Plan::Form(Form { id: spec.id, label: spec.label, fields: req.into_iter().map(|p| (p.name, p.about, String::new())).collect() })
}

/// Runs a command picked or typed in the tools, then closes them. An error goes to the status line,
/// and the tools stay open so the values can be changed.
fn run_with(app: &mut WordApp, ctx: &Context, id: &str, params: Value) {
    match app.run(id, params) {
        Ok(_) => close_all(ctx),
        Err(e) => app.status(e),
    }
}

/// What a tap on a command's row does. One that needs no values runs as Search runs it; one that
/// needs one value from a list opens a picker; one that needs typed values runs as Search does
/// (which opens its dialog, where it has one), and only when that fails opens a form.
fn tap(app: &mut WordApp, ctx: &Context, spec: &CommandSpec) {
    let next = match plan(spec) {
        Plan::Run => {
            close_all(ctx);
            if let Err(e) = app.run(spec.id, json!({})) {
                app.status(e);
            }
            return;
        }
        Plan::Pick(p) => Tool::Pick(p),
        Plan::Form(f) => {
            if app.run(spec.id, json!({})).is_ok() {
                close_all(ctx);
                return;
            }
            Tool::Form(f)
        }
    };
    push(ctx, next);
}

fn push(ctx: &Context, t: Tool) {
    let mut s = stack(ctx);
    s.push(t);
    set_stack(ctx, s);
}

/// The title of the top screen, before translation.
pub(super) fn title(ctx: &Context) -> String {
    match stack(ctx).last() {
        Some(Tool::Search(_)) => "Search".into(),
        Some(Tool::Tab(t)) => (*t).to_string(),
        Some(Tool::Pick(p)) => p.label.to_string(),
        Some(Tool::Form(f)) => f.label.to_string(),
        Some(Tool::Tabs) | None => "All tools".into(),
    }
}

// ---------------------------------------------------------------- drawing

/// A list row: the label, an optional second line, and a chevron. A disabled row is greyed and inert.
fn row(ui: &mut Ui, m: Md, label: &str, sub: Option<&str>, chevron: bool, enabled: bool) -> bool {
    let h = if sub.is_some() { ROW_H + 12.0 } else { ROW_H };
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), h), sense);
    if enabled && resp.is_pointer_button_down_on() {
        ui.painter().rect_filled(r, CornerRadius::same(8), m.outline_variant);
    }
    let ink = if enabled { m.on_surface } else { m.on_surface_variant.gamma_multiply(0.6) };
    let right = if chevron { 40.0 } else { 8.0 };
    let width = (r.width() - 4.0 - right).max(1.0);
    let y = if sub.is_some() { r.center().y - 10.0 } else { r.center().y };
    paint_cut(ui, Pos2::new(r.left() + 4.0, y), width, label, regular(16.0), ink);
    if let Some(s) = sub {
        paint_cut(ui, Pos2::new(r.left() + 4.0, r.center().y + 13.0), width, s, regular(12.0), m.on_surface_variant);
    }
    if chevron {
        icons::paint(
            ui.painter(),
            Rect::from_center_size(Pos2::new(r.right() - 20.0, r.center().y), vec2(20.0, 20.0)),
            "chevronRight",
            ink,
            m.primary,
        );
    }
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, label));
    resp.clicked()
}

/// One line of text from `at` (left, centred on its y), cut with an ellipsis to `width`.
fn paint_cut(ui: &Ui, at: Pos2, width: f32, text: &str, font: egui::FontId, color: egui::Color32) {
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_owned(), font, color);
    job.wrap.max_width = width;
    job.wrap.max_rows = 1;
    job.wrap.overflow_character = Some('…');
    let galley = ui.painter().layout_job(job);
    let y = at.y - galley.size().y / 2.0;
    ui.painter().galley(Pos2::new(at.x, y), galley, color);
}

/// A section heading, small and in the primary colour.
fn heading(ui: &mut Ui, m: Md, text: &str) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::hover());
    ui.painter().text(Pos2::new(r.left() + 4.0, r.bottom() - 8.0), Align2::LEFT_BOTTOM, text, medium(12.0), m.primary);
}

/// A command's row: its label, why it cannot run now (when it cannot), and a chevron when it asks
/// for a value. True when tapped.
fn command_row(app: &WordApp, ui: &mut Ui, m: Md, spec: &CommandSpec) -> bool {
    let reason = if crate::widgets::enabled(app, spec.id) { None } else { Some(app.session.why_disabled(spec.id).unwrap_or("Not available now")) };
    let asks = !required(spec.params).is_empty();
    row(ui, m, crate::tl!(spec.label), reason.map(|r| crate::tl!(r)), asks, reason.is_none())
}

/// The tools' body: the top screen of the stack.
pub(super) fn body(app: &mut WordApp, ui: &mut Ui, m: Md) {
    let ctx = ui.ctx().clone();
    let mut s = stack(&ctx);
    let Some(top) = s.last_mut() else { return };
    let mut next: Option<Tool> = None;
    let mut tapped: Option<CommandSpec> = None;
    let mut ran: Option<(&'static str, Value)> = None;
    match top {
        Tool::Tabs => {
            if row(ui, m, crate::tl!("Search all commands"), None, true, true) {
                next = Some(Tool::Search(String::new()));
            }
            heading(ui, m, crate::tl!("Ribbon tabs"));
            let contextual = contextual_now(app);
            let tabs = available_tabs(app);
            for tab in tabs.iter().filter(|t| !contextual.contains(t)) {
                let label = format!("{}  ({})", crate::tl!(tab), tab_specs(app, tab).len());
                if row(ui, m, &label, None, true, true) {
                    next = Some(Tool::Tab(tab));
                }
            }
            if tabs.iter().any(|t| contextual.contains(t)) {
                heading(ui, m, crate::tl!("Selection"));
                for tab in tabs.iter().filter(|t| contextual.contains(t)) {
                    let label = format!("{}  ({})", crate::tl!(tab), tab_specs(app, tab).len());
                    if row(ui, m, &label, None, true, true) {
                        next = Some(Tool::Tab(tab));
                    }
                }
            }
        }
        Tool::Search(q) => {
            let hint = crate::tl!("Search commands");
            let r = ui.add(super::touch_edit(ui, egui::TextEdit::singleline(q).hint_text(hint).desired_width(f32::INFINITY)));
            r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, hint));
            // The field takes the focus once, as the Search screen opens.
            if !ctx.data(|d| d.get_temp::<bool>(search_focus_id()).unwrap_or(false)) {
                r.request_focus();
                ctx.data_mut(|d| d.insert_temp(search_focus_id(), true));
            }
            let needle = q.trim().to_lowercase();
            if needle.is_empty() {
                heading(ui, m, crate::tl!("Type a name, such as table, margins or comment"));
            } else {
                let found: Vec<CommandSpec> = app
                    .session
                    .registry
                    .all()
                    .iter()
                    .filter(|s| {
                        tab_of(s).is_some()
                            && (crate::tl!(s.label).to_lowercase().contains(&needle)
                                || s.label.to_lowercase().contains(&needle)
                                || s.id.to_lowercase().contains(&needle))
                    })
                    .take(80)
                    .cloned()
                    .collect();
                for spec in found {
                    if command_row(app, ui, m, &spec) {
                        tapped = Some(spec);
                    }
                }
            }
        }
        Tool::Tab(tab) => {
            let mut group: Option<&str> = None;
            for spec in tab_specs(app, tab) {
                let g = group_of(&spec);
                if g != group {
                    group = g;
                    if let Some(name) = g {
                        heading(ui, m, crate::tl!(name));
                    }
                }
                if command_row(app, ui, m, &spec) {
                    tapped = Some(spec);
                }
            }
        }
        Tool::Pick(p) => {
            for (params, label) in &p.options {
                if row(ui, m, crate::tl!(label), None, false, true) {
                    ran = Some((p.id, params.clone()));
                }
            }
        }
        Tool::Form(f) => {
            for (name, about, value) in &mut f.fields {
                ui.add_space(8.0);
                let l = ui.label(egui::RichText::new(readable(name)).font(regular(13.0)).color(m.on_surface_variant));
                ui.add(super::touch_edit(ui, egui::TextEdit::singleline(value).hint_text(about.as_str()).desired_width(f32::INFINITY)))
                    .labelled_by(l.id);
            }
            ui.add_space(16.0);
            let label = crate::tl!(f.label).trim_end_matches('…').to_string();
            let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), TOUCH), Sense::click());
            let fill = if resp.is_pointer_button_down_on() { m.on_primary_container } else { m.primary };
            ui.painter().rect_filled(r, TOUCH / 2.0, fill);
            ui.painter().text(r.center(), Align2::CENTER_CENTER, &label, medium(15.0), m.on_primary);
            resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label.clone()));
            if resp.clicked() {
                let mut params = json!({});
                for (name, _, value) in &f.fields {
                    if !value.trim().is_empty() {
                        params[name.as_str()] = parse_value(value);
                    }
                }
                ran = Some((f.id, params));
            }
        }
    }
    ui.add_space(GUTTER);
    // Keep what was typed, then apply the tap: the tap may change the stack.
    set_stack(&ctx, s.clone());
    if let Some(n) = next {
        if matches!(n, Tool::Search(_)) {
            ctx.data_mut(|d| d.insert_temp(search_focus_id(), false));
        }
        push(&ctx, n);
    } else if let Some(spec) = tapped {
        tap(app, &ctx, &spec);
    } else if let Some((id, params)) = ran {
        run_with(app, &ctx, id, params);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> WordApp {
        WordApp::new(wordcraft_engine::Session::new(wordcraft_doc::Document::new()), crate::Services::default())
    }

    #[test]
    fn every_action_command_is_in_exactly_one_tab() {
        let a = app();
        for spec in a.session.registry.all() {
            let n = TABS.iter().filter(|t| tab_specs(&a, t).iter().any(|s| s.id == spec.id)).count();
            let expected = usize::from(tab_of(spec).is_some());
            assert_eq!(n, expected, "{} ({}) is listed in {n} tabs", spec.id, spec.location);
        }
    }

    #[test]
    fn the_ribbon_tabs_have_commands_and_other_stays_small() {
        let a = app();
        for tab in ["Home", "Insert", "Layout", "References", "Review", "View", "File"] {
            assert!(!tab_specs(&a, tab).is_empty(), "{tab} has commands");
        }
        assert!(
            tab_specs(&a, "Other").len() < 10,
            "most commands fold into a ribbon tab: {:?}",
            tab_specs(&a, "Other").iter().map(|s| s.location).collect::<Vec<_>>()
        );
    }

    #[test]
    fn params_docs_read_names_and_optional_parts() {
        let p = params("{range?, preset: bottom|top|left, style?: thin|thick…, color?}");
        assert_eq!(
            p.iter().map(|p| (p.name.as_str(), p.optional)).collect::<Vec<_>>(),
            vec![("range", true), ("preset", false), ("style", true), ("color", true)]
        );
        assert_eq!(p[1].about, "bottom|top|left");
        let p = params("{height: 20 (pixels) | points: 15}");
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].name, "height");
        assert!(params("{}").is_empty());
        assert_eq!(required("{range?, size: 11} → {ok}").iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), vec!["size"]);
    }

    #[test]
    fn spelled_out_lists_need_every_word_and_no_ellipsis() {
        assert_eq!(spelled_out("portrait|landscape"), Some(vec!["portrait".into(), "landscape".into()]));
        assert_eq!(spelled_out("\"portrait\"|\"landscape\""), Some(vec!["portrait".into(), "landscape".into()]));
        assert_eq!(spelled_out("thin|thick…"), None);
        assert_eq!(spelled_out("11"), None);
    }

    #[test]
    fn typed_values_become_numbers_lists_or_words() {
        assert_eq!(parse_value("14"), json!(14));
        assert_eq!(parse_value(" true "), json!(true));
        assert_eq!(parse_value("[1,2]"), json!([1, 2]));
        assert_eq!(parse_value("Heading 1"), json!("Heading 1"));
        assert_eq!(readable("pageWidth"), "Page width");
    }
}
