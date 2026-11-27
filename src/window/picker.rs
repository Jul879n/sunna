//! «Ir a…», native: the selector of tabs and spaces (the `picker` mod's work). Spaces with something that needs
//! you come first, quiet ones fold into «anteriores»; type to filter, ↑ ↓ and Enter to go, Ctrl+D closes the
//! tab under the cursor, Esc and the ✕ leave. The last rows are new work: a tab here, a space on this computer
//! or on each connected machine. A mod that opens `{mod: "picker"}` or runs `mod.open {name: "picker"}` gets this
//! one (unless the person keeps a mod folder of that name).
//!
//! What the list holds (order, filter, the rows the cursor can stop on) is a plain function over the data the
//! window publishes to mods, tested at the end; the view draws it.

use super::native::{icon, tone, z};
use super::*;
use crate::i18n;
use gpui_base::input::{Input, InputEvent, InputState};

/// Overlay id: `mod.open` toggles by it.
pub(super) const ID: &str = "mod-picker";
const WIDTH: f32 = 640.0;
const LIST_H: f32 = 320.0;
const SEARCH_ICON: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/search.svg");
const PLUS_ICON: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/plus.svg");

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Row {
    /// A space's name, over its tabs.
    Head { space: Value },
    /// «n quiet spaces ▾»: folded away until asked (or until typing).
    Earlier { count: usize },
    Ctx { space: Value, id: String, name: String, status: String, here: bool, who: String, team: usize },
    NewTab,
    NewSpace { machine: Option<String> },
}

impl Row {
    /// Can the cursor stop here? (A space's name is only a title.)
    pub fn pickable(&self) -> bool {
        !matches!(self, Row::Head { .. })
    }
    fn height(&self) -> f32 {
        match self {
            Row::Head { .. } => 30.0,
            Row::Earlier { .. } => 36.0,
            Row::NewTab => 38.0,
            _ => 32.0,
        }
    }
}

fn text<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or_default()
}

/// How many live terminals hang under `roots` (the helpers an agent started, and theirs).
fn team_size(live: &[&Value], roots: &[String]) -> usize {
    let mut seen: std::collections::HashSet<String> = roots.iter().cloned().collect();
    let mut queue: Vec<String> = roots.to_vec();
    let mut n = 0;
    while let Some(at) = queue.pop() {
        for t in live.iter().filter(|t| text(t, "parent") == at) {
            let address = text(t, "address").to_owned();
            if seen.insert(address.clone()) {
                n += 1;
                queue.push(address);
            }
        }
    }
    n
}

/// The list for what the window shows now: busy spaces (blocked first), then the quiet ones folded (or open when
/// `show_earlier` or typing), then the new-work rows. `machines` are the connected ones.
pub(super) fn rows(spaces: &[Value], agents: &[Value], terminals: &[Value], machines: &[String], filter: &str, show_earlier: bool) -> Vec<Row> {
    let f = filter.to_lowercase();
    let live: Vec<&Value> = terminals.iter().filter(|t| text(t, "state") == "live").collect();
    let groups: Vec<(&Value, Vec<Row>)> = spaces
        .iter()
        .map(|w| {
            let tabs = w.get("tabs").and_then(Value::as_array).cloned().unwrap_or_default();
            let ctxs = tabs
                .iter()
                .filter_map(|tab| {
                    let id = text(tab, "id");
                    let mine: Vec<&Value> = agents.iter().filter(|a| text(a, "tab") == id).collect();
                    let names: Vec<String> = mine.iter().map(|a| a.pointer("/program/name").and_then(Value::as_str).unwrap_or_default().to_owned()).collect();
                    if !f.is_empty() && ![text(w, "name"), text(tab, "name"), &names.join(" ")].join(" ").to_lowercase().contains(&f) {
                        return None;
                    }
                    let who = mine.iter().map(|a| a.pointer("/program/name").and_then(Value::as_str).map_or_else(|| i18n::t("picker.agent"), str::to_owned)).collect::<Vec<_>>().join(" · ");
                    let roots: Vec<String> = mine.iter().map(|a| text(a, "address").to_owned()).collect();
                    Some(Row::Ctx {
                        space: w.clone(),
                        id: id.to_owned(),
                        name: text(tab, "name").to_owned(),
                        status: text(tab, "status").to_owned(),
                        here: w.get("active").and_then(Value::as_bool) == Some(true) && tab.get("active").and_then(Value::as_bool) == Some(true),
                        who,
                        team: team_size(&live, &roots),
                    })
                })
                .collect();
            (w, ctxs)
        })
        .collect();
    let status = |w: &Value| !text(w, "status").is_empty();
    let mut busy: Vec<&(&Value, Vec<Row>)> = groups.iter().filter(|(w, c)| status(w) && !c.is_empty()).collect();
    let quiet: Vec<&(&Value, Vec<Row>)> = groups.iter().filter(|(w, c)| !status(w) && !c.is_empty()).collect();
    busy.sort_by_key(|(w, _)| text(w, "status") != "blocked");
    let mut out = Vec::new();
    let add = |out: &mut Vec<Row>, g: &(&Value, Vec<Row>)| {
        out.push(Row::Head { space: g.0.clone() });
        out.extend(g.1.iter().cloned());
    };
    busy.iter().for_each(|g| add(&mut out, g));
    if !quiet.is_empty() {
        out.push(Row::Earlier { count: quiet.len() });
        if show_earlier || !f.is_empty() {
            quiet.iter().for_each(|g| add(&mut out, g));
        }
    }
    out.push(Row::NewTab);
    out.push(Row::NewSpace { machine: None });
    out.extend(machines.iter().map(|m| Row::NewSpace { machine: Some(m.clone()) }));
    out
}

/// Where the cursor starts: the tab you are in, else the first row.
pub(super) fn start(pickable: &[&Row]) -> usize {
    pickable.iter().position(|r| matches!(r, Row::Ctx { here: true, .. })).unwrap_or(0)
}

/// Where the cursor goes when the filter changes: the first tab that matches.
pub(super) fn first_match(pickable: &[&Row], filter: &str) -> usize {
    if filter.is_empty() { 0 } else { pickable.iter().position(|r| matches!(r, Row::Ctx { .. })).unwrap_or(0) }
}

/// The first row to show so that row `sel` is inside `budget` px: the list scrolls only as far as it takes (a
/// title that opens the selected row's group goes with it).
pub(super) fn scroll_to(rows: &[Row], first: usize, sel: usize, budget: f32) -> usize {
    let mut first = first.min(rows.len().saturating_sub(1));
    let from = if sel > 0 && !rows[sel - 1].pickable() { sel - 1 } else { sel };
    if from < first {
        first = from;
    }
    while first < sel && rows[first..=sel].iter().map(Row::height).sum::<f32>() > budget {
        first += 1;
    }
    first
}

/// `by` steps from `at` among `n` rows, wrapping.
pub(super) fn wrap(at: usize, by: i32, n: usize) -> usize {
    if n == 0 { 0 } else { (at as i64 + by as i64).rem_euclid(n as i64) as usize }
}

pub(super) struct PickerView {
    app: WeakEntity<AppWindow>,
    input: Entity<InputState>,
    focus: dialog::Focus,
    filter: String,
    /// None: not placed yet (the first render puts it on where you are).
    at: Option<usize>,
    first: usize,
    reveal: bool,
    show_earlier: bool,
    machines: Vec<String>,
}

impl PickerView {
    fn new(app: WeakEntity<AppWindow>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(i18n::t("picker.go_to_a_tab_space")));
        input.update(cx, |s, _| s.set_editor_style(dialog::input_style()));
        cx.subscribe_in(&input, window, |this, input, ev: &InputEvent, window, cx| match ev {
            InputEvent::Change => {
                this.filter = input.read(cx).value().to_string();
                let rows = this.rows();
                let pickable: Vec<&Row> = rows.iter().filter(|r| r.pickable()).collect();
                this.at = Some(first_match(&pickable, &this.filter));
                this.first = 0;
                this.reveal = true;
                cx.notify();
            }
            InputEvent::PressEnter { .. } => this.go(window, cx),
            InputEvent::Focus | InputEvent::Blur => cx.notify(),
            _ => {}
        })
        .detach();
        input.update(cx, |s, cx| s.focus(window, cx));
        // The machines that are connected, off the UI thread.
        cx.spawn(async move |this, cx| {
            let listed = cx.background_executor().spawn(async { mods::run_blocking("hc", &["machine".into(), "ls".into(), "--json".into()], None, Duration::from_secs(20)) }).await;
            let machines: Vec<String> = listed
                .ok()
                .filter(|(code, _, _)| *code == 0)
                .and_then(|(_, out, _)| serde_json::from_str::<Value>(&out).ok())
                .and_then(|v| v.get("machines").and_then(Value::as_array).cloned())
                .unwrap_or_default()
                .iter()
                .filter(|m| text(m, "state") == "connected" && m.get("temporary").and_then(Value::as_bool) != Some(true))
                .map(|m| text(m, "name").to_owned())
                .collect();
            let _ = this.update(cx, |this, cx| {
                this.machines = machines;
                cx.notify();
            });
        })
        .detach();
        Self { app, input, focus: dialog::Focus::new(cx), filter: String::new(), at: None, first: 0, reveal: true, show_earlier: false, machines: Vec::new() }
    }

    fn rows(&self) -> Vec<Row> {
        mods::snapshot_with(|s| {
            let list = |v: &Value| v.as_array().cloned().unwrap_or_default();
            rows(&list(&s.spaces), &list(&s.agents), &list(&s.terminals), &self.machines, &self.filter, self.show_earlier)
        })
    }

    fn close(&self, window: &mut Window, cx: &mut Context<Self>) {
        let _ = self.app.update(cx, |app, cx| app.close_overlay(ID, window, cx));
    }

    /// Runs the row under the cursor.
    fn go(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let rows = self.rows();
        let pickable: Vec<&Row> = rows.iter().filter(|r| r.pickable()).collect();
        let Some(row) = pickable.get(self.at.unwrap_or(0)).cloned().cloned() else { return };
        self.choose(row, window, cx);
    }

    fn choose(&mut self, row: Row, window: &mut Window, cx: &mut Context<Self>) {
        let (name, args) = match row {
            Row::Earlier { .. } => {
                self.show_earlier = !self.show_earlier;
                self.input.update(cx, |s, cx| s.focus(window, cx));
                self.reveal = true;
                cx.notify();
                return;
            }
            Row::Ctx { id, .. } => ("tab.select", json!({ "id": id })),
            Row::NewTab => ("tab.new", json!({})),
            Row::NewSpace { machine } => ("space.new", json!({ "machine": machine.unwrap_or_default() })),
            Row::Head { .. } => return,
        };
        let _ = self.app.update(cx, |app, cx| {
            app.close_overlay(ID, window, cx);
            app.run(name, args, window, cx);
        });
    }

    fn glyph(status: &str) -> Div {
        let color = match status {
            "working" => c("working"),
            "blocked" => c("blocked"),
            "done" => c("done"),
            _ => tone("muted"),
        };
        div().w(z(12.0)).flex_none().text_color(color).child(match status {
            "idle" => "○",
            "" => "·",
            _ => "●",
        })
    }
}

impl Render for PickerView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rows = self.rows();
        let pickable: Vec<&Row> = rows.iter().filter(|r| r.pickable()).collect();
        let at = self.at.unwrap_or_else(|| start(&pickable)).min(pickable.len().saturating_sub(1));
        self.at = Some(at);
        let sel = pickable.get(at).copied();
        let sel_index = rows.iter().position(|r| Some(r) == sel).unwrap_or(0);
        self.first = self.first.min(rows.len().saturating_sub(1));
        if self.reveal {
            self.first = scroll_to(&rows, self.first, sel_index, LIST_H);
            self.reveal = false;
        }
        let mut used = 0.0;
        let shown: Vec<(usize, &Row)> = rows.iter().enumerate().skip(self.first).take_while(|(_, r)| {
            used += r.height();
            used <= LIST_H + 0.5
        }).collect();
        let muted = tone("muted");
        let list = div().flex().flex_col().children(shown.into_iter().map(|(i, r)| {
            let on = Some(r) == sel;
            let row = r.clone();
            let base = |id: String| {
                div()
                    .id(SharedString::from(id))
                    .h(z(r.height()))
                    .flex_none()
                    .flex()
                    .flex_row()
                    .items_center()
                    .cursor_pointer()
                    .when(on, |d| d.bg(c("selected")))
                    .hover(|s| s.bg(c("hover")))
                    .on_click(cx.listener({
                        let row = row.clone();
                        move |this, _, w, cx| this.choose(row.clone(), w, cx)
                    }))
            };
            match r {
                Row::Head { space } => {
                    let name = text(space, "name");
                    div()
                        .h(z(r.height()))
                        .flex_none()
                        .px(z(10.0))
                        .pt(z(8.0))
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(z(8.0))
                        .text_size(z(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(muted)
                        .child(
                            div().size(z(18.0)).rounded(z(5.0)).bg(super::spaces::space_color(space)).flex().items_center().justify_center().text_size(z(10.5)).font_weight(FontWeight::BOLD)
                                .text_color(gpui::white())
                                .child(name.chars().next().map(|ch| ch.to_uppercase().to_string()).unwrap_or_else(|| "?".into())),
                        )
                        .child(name.to_owned())
                        .into_any_element()
                }
                Row::Earlier { count } => {
                    let arrow = if self.show_earlier { "▴" } else { "▾" };
                    base(format!("pk-earlier-{i}"))
                        .mx(z(4.0))
                        .px(z(10.0))
                        .rounded(z(7.0))
                        .text_size(z(12.0))
                        .text_color(if on { c("foreground") } else { muted })
                        .child(i18n::tn("picker.earlier_n_quiet_space_arrow", *count, &[("n", &count.to_string()), ("arrow", arrow)]))
                        .into_any_element()
                }
                Row::NewTab | Row::NewSpace { .. } => {
                    let label = match r {
                        Row::NewTab => i18n::t("picker.new_tab"),
                        Row::NewSpace { machine: Some(m) } => i18n::tf("picker.new_space_on_machine", &[("machine", m)]),
                        _ => i18n::t("picker.new_space"),
                    };
                    base(format!("pk-new-{i}"))
                        .mx(z(4.0))
                        .when(matches!(r, Row::NewTab), |d| d.mt(z(6.0)))
                        .px(z(10.0))
                        .gap(z(8.0))
                        .rounded(z(7.0))
                        .text_size(z(13.0))
                        .text_color(if on { c("foreground") } else { muted })
                        .child(icon(PLUS_ICON, 12.0, muted))
                        .child(label)
                        .into_any_element()
                }
                Row::Ctx { id, name, status, here, who, team, .. } => base(format!("pk-{id}"))
                    .mx(z(4.0))
                    .px(z(10.0))
                    .gap(z(10.0))
                    .rounded(z(7.0))
                    .text_size(z(13.0))
                    .child(Self::glyph(status))
                    .child(div().flex_1().min_w_0().truncate().text_color(c("foreground")).when(*here, |d| d.font_weight(FontWeight::SEMIBOLD)).child(name.clone()))
                    .when(status == "blocked", |d| d.child(div().text_size(z(11.0)).text_color(tone("blocked")).child(i18n::t("picker.needs_you"))))
                    .when(status == "done", |d| d.child(div().text_size(z(11.0)).text_color(tone("done")).child(i18n::t("picker.finished"))))
                    .child(div().text_size(z(11.0)).text_color(muted).child(format!("{who}{}", if *team > 0 { format!("  +{team}") } else { String::new() })))
                    .into_any_element(),
            }
        }));
        let this = cx.entity();
        let close = move |w: &mut Window, cx: &mut App| this.update(cx, |v, cx| v.close(w, cx));
        let focused = self.input.read(cx).focus_handle(cx).is_focused(window);
        let search = dialog::field_box(focused, false)
            .flex_none()
            .gap(z(10.0))
            .child(svg().data(SEARCH_ICON).size(z(16.0)).text_color(tone("dim")))
            .child(div().flex_1().h_full().child(Input::new(&self.input)));
        let body = div()
            .flex()
            .flex_col()
            .gap(z(10.0))
            .child(search)
            .child(
                div()
                    .id("picker-list")
                    .h(z(LIST_H.min(rows.iter().map(Row::height).sum())))
                    .overflow_hidden()
                    .on_scroll_wheel(cx.listener(|this, e: &ScrollWheelEvent, _, cx| {
                        let steps = (-f32::from(e.delta.pixel_delta(px(32.0)).y) / 32.0).round() as i64;
                        let last = this.rows().len().saturating_sub(1) as i64;
                        this.first = (this.first as i64 + steps).clamp(0, last) as usize;
                        cx.notify();
                    }))
                    .child(list),
            )
            .child(div().text_size(z(12.0)).text_color(muted).child(i18n::t("rs.picker.hints")));
        let card = dialog::Dialog::new(i18n::t("picker.go_to"), &self.focus, close).width(WIDTH).no_cancel().fields(1).body(body).render("picker");
        div().on_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| {
            let rows = this.rows();
            let pickable: Vec<&Row> = rows.iter().filter(|r| r.pickable()).collect();
            let at = this.at.unwrap_or(0);
            match e.keystroke.key.as_str() {
                "down" => this.at = Some(wrap(at, 1, pickable.len())),
                "up" => this.at = Some(wrap(at, -1, pickable.len())),
                "d" if e.keystroke.modifiers.control => {
                    if let Some(Row::Ctx { id, .. }) = pickable.get(at).cloned().cloned() {
                        let _ = this.app.update(cx, |app, cx| {
                            app.close_overlay(ID, window, cx);
                            app.run("tab.close", json!({ "id": id }), window, cx);
                        });
                    }
                    cx.stop_propagation();
                    return;
                }
                _ => return,
            }
            this.reveal = true;
            cx.stop_propagation();
            cx.notify();
        })).child(card)
    }
}

impl AppWindow {
    /// `mod.open picker` and `openOverlay({mod: "picker"})`: opens the selector, or closes it when it is open.
    pub(super) fn toggle_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.overlays.iter().any(|o| o.id == ID) {
            return self.close_overlay(ID, window, cx);
        }
        let app = cx.weak_entity();
        let view = cx.new(|cx| PickerView::new(app, window, cx));
        let focus = cx.focus_handle();
        self.push_view_overlay(ID.into(), OverlayKind::Popup, size(WIDTH + 48.0, 0.85), Some(40.0), view.into(), focus, window, cx);
        if let Some(o) = self.overlays.iter_mut().find(|o| o.id == ID) {
            o.bare = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // `super::*` brings gpui's own `test` macro: the plain one is the std attribute.
    use core::prelude::v1::test;

    fn space(id: &str, name: &str, status: &str, active: bool, tabs: Vec<Value>) -> Value {
        json!({ "id": id, "name": name, "status": status, "active": active, "tabs": tabs })
    }
    fn tab(id: &str, name: &str, status: &str, active: bool) -> Value {
        json!({ "id": id, "name": name, "status": status, "active": active })
    }
    fn kinds(rows: &[Row]) -> Vec<String> {
        rows.iter().map(|r| match r {
            Row::Head { space } => format!("head:{}", text(space, "name")),
            Row::Earlier { count } => format!("earlier:{count}"),
            Row::Ctx { name, .. } => format!("ctx:{name}"),
            Row::NewTab => "new".into(),
            Row::NewSpace { machine } => format!("space:{}", machine.clone().unwrap_or_default()),
        }).collect()
    }

    #[test]
    fn blocked_spaces_come_first_and_quiet_ones_fold() {
        let spaces = vec![
            space("a", "quiet", "", false, vec![tab("t1", "shell", "idle", false)]),
            space("b", "busy", "working", false, vec![tab("t2", "build", "working", false)]),
            space("c", "stuck", "blocked", true, vec![tab("t3", "asks", "blocked", true)]),
        ];
        let rows = rows(&spaces, &[], &[], &["srv".into()], "", false);
        assert_eq!(kinds(&rows), ["head:stuck", "ctx:asks", "head:busy", "ctx:build", "earlier:1", "new", "space:", "space:srv"]);
        // Open «anteriores»: the quiet space follows it.
        let open = rows_with(&spaces, true);
        assert!(kinds(&open).contains(&"head:quiet".to_owned()));
    }

    fn rows_with(spaces: &[Value], earlier: bool) -> Vec<Row> {
        rows(spaces, &[], &[], &[], "", earlier)
    }

    #[test]
    fn typing_filters_by_space_tab_or_program_and_opens_the_quiet_ones() {
        let spaces = vec![space("a", "api", "", false, vec![tab("t1", "shell", "idle", false), tab("t2", "deploy", "idle", false)])];
        let agents = vec![json!({ "tab": "t1", "address": "x/1", "program": { "name": "claude" } })];
        let r = rows(&spaces, &agents, &[], &[], "clau", false);
        assert_eq!(kinds(&r)[..3], ["earlier:1", "head:api", "ctx:shell"]);
        assert!(!kinds(&r).contains(&"ctx:deploy".to_owned()));
        // The cursor goes to the first tab that matched.
        let pick: Vec<&Row> = r.iter().filter(|x| x.pickable()).collect();
        assert!(matches!(pick[first_match(&pick, "clau")], Row::Ctx { .. }));
    }

    #[test]
    fn a_team_counts_every_live_terminal_under_an_agent() {
        let terminals = [json!({ "address": "m/1", "state": "live", "parent": "" }),
            json!({ "address": "m/2", "state": "live", "parent": "m/1" }),
            json!({ "address": "m/3", "state": "live", "parent": "m/2" }),
            json!({ "address": "m/4", "state": "dead", "parent": "m/1" })];
        let live: Vec<&Value> = terminals.iter().filter(|t| text(t, "state") == "live").collect();
        assert_eq!(team_size(&live, &["m/1".to_owned()]), 2);
    }

    #[test]
    fn the_list_scrolls_the_selected_row_into_the_visible_320_px() {
        let tabs: Vec<Value> = (1..=12).map(|i| tab(&format!("t{i}"), &format!("Task {i}"), "done", i == 1)).collect();
        let spaces = vec![space("s1", "Local", "working", true, tabs)];
        let r = rows(&spaces, &[], &[], &[], "", false);
        let last = r.len() - 1;
        let first = scroll_to(&r, 0, last, LIST_H);
        let shown: f32 = r[first..=last].iter().map(Row::height).sum();
        assert!(first > 0 && shown <= LIST_H, "the last row is inside the list ({shown} px)");
        // Going back up to the first tab brings its space's title with it.
        assert_eq!(scroll_to(&r, first, 1, LIST_H), 0);
        // A short list never scrolls.
        assert_eq!(scroll_to(&r[..4], 0, 3, LIST_H), 0);
    }

    #[test]
    fn the_cursor_starts_where_you_are_and_wraps_at_the_ends() {
        let spaces = vec![space("a", "api", "working", true, vec![tab("t1", "one", "working", false), tab("t2", "two", "working", true)])];
        let r = rows(&spaces, &[], &[], &[], "", false);
        let pick: Vec<&Row> = r.iter().filter(|x| x.pickable()).collect();
        assert_eq!(start(&pick), 1);
        assert_eq!(wrap(0, -1, 5), 4);
        assert_eq!(wrap(4, 1, 5), 0);
        assert_eq!(wrap(0, 1, 0), 0);
        assert!(!Row::Head { space: json!({}) }.pickable());
        // Every row the cursor can stop on is at least 32 px tall.
        assert!(r.iter().filter(|x| x.pickable()).all(|x| x.height() >= 32.0));
    }
}
