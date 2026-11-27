//! The Activity panel (`{"native": "activity"}`, once the `activity` mod): every agent with filters
//! (state, program, text) and counts, and the log of when agents finished or needed you
//! (`shared("activity.events")`, written by `agents::track`; `activity.seen` is the time it was last
//! looked at). Filtering, counting and ages are plain functions, tested at the end.

use super::agents::{program_glyph, since_text, status_mark, unread};
use super::native::{fade, icon, r, tip, tone, z};
use super::spaces::{hf, opt_text, text, vf};
use super::*;
use crate::i18n;
use gpui_base::input::{Input, InputEvent, InputState};

const CLOSE: &[u8] = include_bytes!("../../assets/icons/pane/close.svg");
const ROW_H: f32 = 44.0;
const BUTTON_H: f32 = 32.0;
pub(super) const STATUSES: [&str; 5] = ["all", "blocked", "working", "done", "idle"];

// Pure rules ----------------------------------------------------------------------------------

fn program_name(a: &Value) -> &str {
    opt_text(&a["program"], "name").or_else(|| opt_text(a, "app")).unwrap_or_default()
}

/// The agents that pass the filter (`"all"` or `""` for no limit on state or program; the text is matched
/// in the space, tab, title and program names, ignoring case).
pub(super) fn filter_agents<'a>(list: &'a [Value], status: &str, app: &str, query: &str) -> Vec<&'a Value> {
    let q = query.trim().to_lowercase();
    list.iter()
        .filter(|a| {
            (status.is_empty() || status == "all" || text(a, "status") == status)
                && (app.is_empty() || app == "all" || program_name(a) == app)
                && (q.is_empty() || format!("{} {} {} {}", text(a, "space_name"), text(a, "tab_name"), text(a, "title"), opt_text(&a["program"], "name").unwrap_or_default()).to_lowercase().contains(&q))
        })
        .collect()
}

/// How many agents sit in each state, for the filter chips: `[all, blocked, working, done, idle]`.
pub(super) fn counts(list: &[Value]) -> [usize; 5] {
    let mut n = [list.len(), 0, 0, 0, 0];
    for a in list {
        if let Some(i) = STATUSES.iter().position(|s| *s != "all" && *s == text(a, "status")) {
            n[i] += 1;
        }
    }
    n
}

/// The distinct agent programs present, sorted by name.
pub(super) fn apps(list: &[Value]) -> Vec<String> {
    let mut names: Vec<String> = list.iter().map(|a| program_name(a).to_owned()).filter(|n| !n.is_empty()).collect();
    names.sort();
    names.dedup();
    names
}

/// The age of a log entry, "now" under a minute.
fn age(at: u64, now: u64) -> String {
    since_text(at, now).unwrap_or_else(|| i18n::t("activity.now"))
}

// The view -------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Agents,
    Feed,
}

pub(super) struct ActivityView {
    app: WeakEntity<AppWindow>,
    tab: Tab,
    status: &'static str,
    program: String,
    query: String,
    input: Entity<InputState>,
    typing: bool,
    scroll: ScrollHandle,
}

impl ActivityView {
    pub(super) fn new(app: WeakEntity<AppWindow>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(i18n::t("activity.search")));
        input.update(cx, |s, _| s.set_editor_style(super::settings_ui::controls::input_style()));
        cx.subscribe_in(&input, window, |this, input, ev: &InputEvent, _window, cx| match ev {
            InputEvent::Change => {
                this.query = input.read(cx).value().to_string();
                cx.notify();
            }
            InputEvent::Focus => {
                this.typing = true;
                cx.notify();
            }
            InputEvent::Blur => {
                this.typing = false;
                cx.notify();
            }
            _ => {}
        })
        .detach();
        cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(Duration::from_secs(30)).await;
            if this.update(cx, |_, cx| cx.notify()).is_err() {
                break;
            }
        })
        .detach();
        Self { app, tab: Tab::Agents, status: "all", program: "all".into(), query: String::new(), input, typing: false, scroll: ScrollHandle::new() }
    }

    fn agents() -> Vec<Value> {
        mods::snapshot_with(|s| s.agents.as_array().cloned().unwrap_or_default())
    }

    fn go(&self, address: &str, window: &mut Window, cx: &mut Context<Self>) {
        let args = json!({ "terminal": address });
        let _ = self.app.update(cx, |app, cx| app.run("agent.focus", args, window, cx));
    }

    fn close(&self, window: &mut Window, cx: &mut Context<Self>) {
        let _ = self.app.update(cx, |app, cx| app.run("layout.hide", json!({ "id": "panel" }), window, cx));
    }

    /// A 32 px button that works with the mouse and, focused, with Enter or Space.
    fn chip(&self, id: impl Into<ElementId>, label: String, on: bool, count: Option<usize>, cx: &mut Context<Self>, act: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + Clone + 'static) -> Stateful<Div> {
        let key_act = act.clone();
        hf().id(id)
            .h(z(BUTTON_H))
            .px(z(10.0))
            .gap(z(5.0))
            .flex_none()
            .rounded(r(6.0))
            .border_1()
            .border_color(c(if on { "accent" } else { "border" }))
            .bg(c(if on { "selected" } else { "background" }))
            .text_size(z(12.0))
            .text_color(if on { c("foreground") } else { tone("muted") })
            .cursor_pointer()
            .hover(|s| s.bg(c("hover")))
            .focusable()
            .tab_stop(true)
            .focus_visible(|s| s.border_color(c("foreground")))
            .on_key_down(cx.listener(move |this, e: &KeyDownEvent, window, cx| {
                if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                    key_act(this, window, cx);
                    cx.stop_propagation();
                    cx.notify();
                }
            }))
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                act(this, window, cx);
                cx.notify();
            }))
            .child(label)
            .when_some(count, |d, n| d.child(div().text_color(tone("dim")).child(n.to_string())))
    }

    /// A row of the lists: an icon, two lines and a mark.
    fn entry(&self, id: SharedString, selected: bool, lead: AnyElement, top: String, bottom: String, tail: AnyElement, cx: &mut Context<Self>, act: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + Clone + 'static) -> AnyElement {
        let key_act = act.clone();
        hf().id(id)
            .w_full()
            .flex_none()
            .h(z(ROW_H))
            .px(z(10.0))
            .gap(z(10.0))
            .rounded(r(8.0))
            .cursor_pointer()
            .border_2()
            .border_color(Hsla { a: 0.0, ..c("accent") })
            .when(selected, |d| d.bg(c("selected")))
            .hover(|s| s.bg(c("hover")))
            .focusable()
            .tab_stop(true)
            .focus_visible(|s| s.border_color(graphic("accent")))
            .on_key_down(cx.listener(move |this, e: &KeyDownEvent, window, cx| {
                if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                    key_act(this, window, cx);
                    cx.stop_propagation();
                }
            }))
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| act(this, window, cx)))
            .child(lead)
            .child(vf().flex_1().min_w(px(0.0)).child(div().id("activity-top").truncate().text_size(z(13.0)).text_color(c("foreground")).tooltip(tip(top.to_string())).child(top)).child(div().truncate().text_size(z(11.0)).text_color(tone("dim")).child(bottom)))
            .child(tail)
            .into_any_element()
    }

    fn agents_view(&self, list: &[Value], cx: &mut Context<Self>) -> AnyElement {
        let n = counts(list);
        let shown = filter_agents(list, self.status, &self.program, &self.query);
        let names = apps(list);
        let filtered = self.status != "all" || self.program != "all" || !self.query.trim().is_empty();
        let chips = STATUSES.iter().enumerate().map(|(i, s)| {
            let status: &'static str = s;
            self.chip(format!("act-st-{s}"), i18n::t(&format!("activity.status_{s}")), self.status == status, Some(n[i]), cx, move |this, _, _| this.status = status).into_any_element()
        }).collect::<Vec<_>>();
        let programs = std::iter::once("all".to_owned()).chain(names.clone()).map(|p| {
            let (label, pick) = (if p == "all" { i18n::t("activity.status_all") } else { p.clone() }, p.clone());
            self.chip(format!("act-app-{p}"), label, self.program == p, None, cx, move |this, _, _| this.program = pick.clone()).into_any_element()
        }).collect::<Vec<_>>();
        let rows: Vec<AnyElement> = shown
            .iter()
            .map(|a| {
                let address = text(a, "address").to_owned();
                let go = address.clone();
                self.entry(
                    SharedString::from(format!("act-a-{address}")),
                    a["focused"] == true,
                    program_glyph(&a["program"], 16.0),
                    text(a, "space_name").to_owned(),
                    [text(a, "tab_name"), text(a, "title")].into_iter().find(|s| !s.is_empty()).unwrap_or_default().to_owned(),
                    status_mark(text(a, "status"), true),
                    cx,
                    move |this, window, cx| this.go(&go, window, cx),
                )
            })
            .collect();
        vf().flex_1()
            .min_h(px(0.0))
            .child(
                vf().flex_none()
                    .p(z(10.0))
                    .gap(z(8.0))
                    .child(
                        hf().id("act-search")
                            .h(z(BUTTON_H))
                            .px(z(10.0))
                            .rounded(r(8.0))
                            .border_1()
                            .border_color(c(if self.typing { "accent" } else { "border" }))
                            .cursor_text()
                            .text_size(z(12.0))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .child(div().flex_1().child(Input::new(&self.input))),
                    )
                    .child(hf().gap(z(6.0)).flex_wrap().children(chips))
                    .when(names.len() > 1, |d| d.child(hf().gap(z(6.0)).flex_wrap().children(programs))),
            )
            .child(fade(8.0).my(z(0.0)))
            .child(
                vf().id("act-list")
                    .flex_1()
                    .min_h(px(0.0))
                    .p(z(8.0))
                    .gap(z(4.0))
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .when(list.is_empty(), |d| d.child(div().p(z(8.0)).text_size(z(12.0)).text_color(tone("muted")).child(i18n::t("activity.none"))))
                    .when(!list.is_empty() && shown.is_empty(), |d| {
                        d.child(
                            vf().p(z(8.0))
                                .gap(z(8.0))
                                .items_start()
                                .child(div().text_size(z(12.0)).text_color(tone("muted")).child(i18n::t("activity.no_match")))
                                .when(filtered, |d| {
                                    d.child(self.chip("act-clear", i18n::t("activity.clear_filters"), false, None, cx, |this, window, cx| {
                                        this.status = "all";
                                        this.program = "all".into();
                                        this.query.clear();
                                        this.input.update(cx, |s, cx| s.set_value("", window, cx));
                                    }))
                                }),
                        )
                    })
                    .children(rows),
            )
            .into_any_element()
    }

    fn feed_view(&self, events: &[Value], now: u64, seen: u64, cx: &mut Context<Self>) -> AnyElement {
        let rows: Vec<AnyElement> = events
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let blocked = text(e, "kind") == "blocked";
                let at = e.get("at").and_then(Value::as_u64).unwrap_or(0);
                let address = text(e, "address").to_owned();
                self.entry(
                    SharedString::from(format!("act-e-{i}")),
                    false,
                    div().size(z(8.0)).flex_none().rounded_full().bg(c(if blocked { "blocked" } else { "done" })).into_any_element(),
                    text(e, "name").to_owned(),
                    i18n::t(if blocked { "activity.event_blocked" } else { "activity.event_done" }),
                    div().flex_none().text_size(z(11.0)).text_color(tone(if at > seen { "accent" } else { "dim" })).child(age(at, now)).into_any_element(),
                    cx,
                    move |this, window, cx| this.go(&address, window, cx),
                )
            })
            .collect();
        vf().flex_1()
            .min_h(px(0.0))
            .child(
                hf().flex_none()
                    .h(z(44.0))
                    .px(z(12.0))
                    .gap(z(8.0))
                    .child(div().flex_1().text_size(z(11.0)).text_color(tone("muted")).child(i18n::tn("activity.feed_count", events.len(), &[("count", &events.len().to_string())])))
                    .child(self.chip("act-read", i18n::t("activity.mark_read"), false, None, cx, |_, _, _| mods::set_shared("activity.seen".into(), json!(model::now_ms()))))
                    .child(self.chip("act-wipe", i18n::t("activity.clear"), false, None, cx, |_, _, _| {
                        mods::set_shared("activity.events".into(), json!([]));
                        mods::set_shared("activity.seen".into(), json!(model::now_ms()));
                    })),
            )
            .child(fade(8.0).my(z(0.0)))
            .child(
                vf().id("act-feed")
                    .flex_1()
                    .min_h(px(0.0))
                    .p(z(8.0))
                    .gap(z(4.0))
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .when(events.is_empty(), |d| d.child(div().p(z(8.0)).text_size(z(12.0)).text_color(tone("muted")).child(i18n::t("activity.feed_empty"))))
                    .children(rows),
            )
            .into_any_element()
    }
}

impl Render for ActivityView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let now = model::now_ms();
        let list = Self::agents();
        let events: Vec<Value> = mods::shared("activity.events").and_then(|v| v.as_array().cloned()).unwrap_or_default();
        let seen = mods::shared("activity.seen").and_then(|v| v.as_u64()).unwrap_or(0);
        let fresh = unread(&events, seen);
        // The log being looked at counts as read.
        if self.tab == Tab::Feed && fresh > 0 {
            mods::set_shared("activity.seen".into(), json!(now));
        }
        let body = match self.tab {
            Tab::Agents => self.agents_view(&list, cx),
            Tab::Feed => self.feed_view(&events, now, seen, cx),
        };
        let agents_count = list.len();
        vf().id("activity")
            .size_full()
            .min_h(px(0.0))
            .rounded(r(10.0))
            .overflow_hidden()
            .bg(c("surface"))
            .border_1()
            .border_color(c("border"))
            .text_color(c("foreground"))
            .text_size(z(13.0))
            .child(
                hf().h(z(46.0))
                    .flex_none()
                    .pl(z(14.0))
                    .pr(z(5.0))
                    .gap(z(8.0))
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(i18n::t("tabs.activity")))
                    .child(div().flex_1())
                    .child(self.chip("act-tab-agents", i18n::t("activity.tab_agents"), self.tab == Tab::Agents, Some(agents_count), cx, |this, _, _| this.tab = Tab::Agents))
                    .child(self.chip("act-tab-feed", i18n::t("activity.tab_feed"), self.tab == Tab::Feed, (fresh > 0).then_some(fresh), cx, |this, _, _| this.tab = Tab::Feed))
                    .child(
                        hf().id("panel-close")
                            .size(z(BUTTON_H))
                            .flex_none()
                            .justify_center()
                            .rounded(r(6.0))
                            .border_1()
                            .border_color(Hsla { a: 0.0, ..c("accent") })
                            .cursor_pointer()
                            .hover(|s| s.bg(c("hover")))
                            .focusable()
                            .tab_stop(true)
                            .focus_visible(|s| s.border_color(graphic("accent")))
                            .tooltip(tip(i18n::t("menu.close")))
                            .on_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| {
                                if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                                    this.close(window, cx);
                                    cx.stop_propagation();
                                }
                            }))
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.close(window, cx)))
                            .child(icon(CLOSE, 13.0, tone("muted"))),
                    ),
            )
            .child(fade(8.0).my(z(0.0)))
            .child(body)
    }
}

#[cfg(test)]
mod tests {
    use super::{apps, counts, filter_agents};
    use crate::window::spaces::text;
    use serde_json::{Value, json};

    fn agent(address: &str, space: &str, status: &str, program: &str) -> Value {
        json!({ "address": address, "space_name": space, "tab_name": "main", "title": "", "status": status, "program": { "name": program } })
    }

    fn sample() -> Vec<Value> {
        vec![agent("a", "api", "blocked", "claude"), agent("b", "api", "working", "codex"), agent("c", "web", "idle", "claude")]
    }

    #[test]
    fn agents_filter_by_state_program_and_text() {
        let list = sample();
        let addresses = |v: Vec<&Value>| v.into_iter().map(|a| text(a, "address").to_owned()).collect::<Vec<_>>();
        assert_eq!(addresses(filter_agents(&list, "blocked", "all", "")), ["a"]);
        assert_eq!(addresses(filter_agents(&list, "all", "codex", "")), ["b"]);
        assert_eq!(addresses(filter_agents(&list, "all", "all", "  WEB ")), ["c"]);
        assert_eq!(filter_agents(&list, "all", "all", "").len(), 3);
        assert_eq!(filter_agents(&list, "", "", "").len(), 3);
        assert!(filter_agents(&list, "done", "all", "").is_empty());
    }

    #[test]
    fn the_chips_count_each_state_and_list_the_programs_present() {
        let list = sample();
        assert_eq!(counts(&list), [3, 1, 1, 0, 1]);
        assert_eq!(apps(&list), ["claude", "codex"]);
        assert_eq!(counts(&[]), [0; 5]);
        assert!(apps(&[json!({ "address": "x" })]).is_empty());
    }

    #[test]
    fn the_program_falls_back_to_the_apps_word() {
        let list = vec![json!({ "address": "x", "app": "pi", "status": "idle" })];
        assert_eq!(apps(&list), ["pi"]);
        assert_eq!(filter_agents(&list, "all", "pi", "").len(), 1);
    }
}
