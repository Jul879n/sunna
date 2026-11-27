//! The bar on top of a peeked terminal (`terminal.peek`, a sub-agent clicked in the agents list):
//! who it is and how it is doing, «↗ go to» its pane (or, without one, «↗ keep» it as a tab) and
//! a visible ✕ that closes the peek. Native since the `peek` mod went away; a layout that says
//! `{"mod": "peek"}` gets the same bar unless the person keeps a mod folder of that name.

use super::native::{icon, keyed, r, tone, z};
use super::*;
use crate::i18n;

const CLOSE: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/x.svg");
const SPINNER: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/spinner.svg");
pub(super) const OVERLAY: &str = "terminal-peek";

/// The word the status has in the bar.
pub(super) fn status_key(status: &str) -> Option<&'static str> {
    match status {
        "working" => Some("peek.working"),
        "blocked" => Some("peek.needs_you"),
        "done" => Some("peek.finished"),
        "idle" => Some("peek.idle"),
        _ => None,
    }
}

/// The glyph before the name for a status that is not a spinner.
pub(super) fn status_glyph(status: Option<&str>) -> &'static str {
    match status {
        Some("idle") => "○",
        Some(_) => "●",
        None => "·",
    }
}

/// «↗ go to» when a pane already shows the terminal, else «↗ keep».
pub(super) fn keep_command(has_pane: bool) -> &'static str {
    if has_pane { "pane.focus" } else { "terminal.keep" }
}

impl AppWindow {
    pub(super) fn native_peek(&self, props: &Value, cx: &mut Context<Self>) -> AnyElement {
        let terminal = props.get("terminal").and_then(Value::as_str).unwrap_or_default().to_owned();
        let rec = self.live.get(&terminal);
        let text = |k: &str| rec.and_then(|r| r.get(k)).and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_owned);
        let program = rec.map(|r| r["program"].clone()).unwrap_or(Value::Null);
        let name = text("name").or_else(|| text("template")).or_else(|| program.get("name").and_then(Value::as_str).map(str::to_owned)).unwrap_or_else(|| terminal.clone());
        let status = self.model.status(&terminal).map(str::to_owned).or_else(|| text("status"));
        let parent = text("parent").map(|p| self.live.get(&p).and_then(|r| r.get("name")).and_then(Value::as_str).filter(|s| !s.is_empty()).map_or(p.clone(), str::to_owned));
        let has_pane = self.model.pane_of(&terminal).is_some();
        let muted = tone("muted");
        let ring = c("accent");
        let program_icon = program.get("icon").and_then(Value::as_str).map(|p| mods::app_dir().join(p));
        let tint = program.get("tint").and_then(Value::as_bool).unwrap_or(true);
        let status_color = status.as_deref().map_or(muted, |s| if matches!(s, "working" | "blocked" | "done") { tone(s) } else { muted });
        let app = cx.entity();
        let button = |id: &'static str, child: AnyElement, label: String, on: Rc<dyn Fn(&mut AppWindow, &mut Window, &mut Context<AppWindow>)>| {
            let app = app.clone();
            keyed(
                div()
                    .id(id)
                    .flex_none()
                    .h(z(32.0))
                    .min_w(z(32.0))
                    .px(z(10.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(r(6.0))
                    .border_1()
                    .border_color(Hsla { a: 0.0, ..ring })
                    .text_color(muted)
                    .text_size(z(12.0))
                    .cursor_pointer()
                    .hover(|s| s.bg(c("hover")).text_color(c("foreground")))
                    .tooltip(super::native::tip(label)),
                ring,
                move |window, cx| {
                    let on = on.clone();
                    app.update(cx, |this, cx| on(this, window, cx))
                },
            )
            .child(child)
        };
        let target = terminal.clone();
        let go = button(
            "peek-keep",
            div().child(i18n::t(if has_pane { "peek.go_to" } else { "peek.keep" })).into_any_element(),
            i18n::t(if has_pane { "peek.go_to" } else { "peek.keep" }),
            Rc::new(move |this, window, cx| {
                this.run(keep_command(has_pane), json!({ "terminal": target }), window, cx);
                this.close_overlay(OVERLAY, window, cx);
            }),
        );
        let close = button("peek-close", icon(CLOSE, 14.0, muted).into_any_element(), i18n::t("menu.close"), Rc::new(|this, window, cx| this.close_overlay(OVERLAY, window, cx)));
        let lead: AnyElement = match status.as_deref() {
            Some("working") => icon(SPINNER, 12.0, c("working")).with_animation("peek-spin", Animation::new(Duration::from_millis(900)).repeat(), |s, delta| s.with_transformation(Transformation::rotate(percentage(delta)))).into_any_element(),
            other => div().w(z(12.0)).flex_none().text_color(status_color).child(status_glyph(other)).into_any_element(),
        };
        div()
            .size_full()
            .px(z(12.0))
            .gap(z(8.0))
            .flex()
            .items_center()
            .bg(c("surface"))
            .border_b_1()
            .border_color(c("border"))
            .text_size(z(12.0))
            .child(lead)
            .when_some(program_icon, |el, path| {
                if tint {
                    el.child(svg().external_path(path.to_string_lossy().into_owned()).size(z(14.0)).flex_none().text_color(c("foreground")))
                } else {
                    el.child(img(path).size(z(14.0)).flex_none())
                }
            })
            .child(div().truncate().font_weight(FontWeight::SEMIBOLD).text_color(c("foreground")).child(name))
            .when_some(status.as_deref().and_then(status_key), |el, key| el.child(div().flex_none().text_color(status_color).child(i18n::t(key))))
            .when_some(parent, |el, p| el.child(div().truncate().text_color(muted).child(i18n::tf("peek.from_name", &[("name", p.as_str())]))))
            .child(div().flex_1())
            .child(go)
            .child(close)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // `super::*` brings in gpui's own `test` attribute, which would shadow the standard one.
    use std::prelude::v1::test;

    #[test]
    fn each_status_has_its_word_and_an_unknown_one_none() {
        assert_eq!(status_key("working"), Some("peek.working"));
        assert_eq!(status_key("blocked"), Some("peek.needs_you"));
        assert_eq!(status_key("done"), Some("peek.finished"));
        assert_eq!(status_key("idle"), Some("peek.idle"));
        assert_eq!(status_key("none"), None);
    }

    #[test]
    fn the_glyph_before_the_name_follows_the_status() {
        assert_eq!(status_glyph(Some("idle")), "○");
        assert_eq!(status_glyph(Some("done")), "●");
        assert_eq!(status_glyph(None), "·");
    }

    #[test]
    fn a_terminal_with_a_pane_is_focused_else_kept_as_a_tab() {
        assert_eq!(keep_command(true), "pane.focus");
        assert_eq!(keep_command(false), "terminal.keep");
    }
}
