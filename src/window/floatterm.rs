//! The floating terminal (Orca's «Toggle Floating Terminal», ⌘⌥A): one shell in a window over the
//! work area, shown and hidden with `terminal.float`. Hiding only closes the window: the shell keeps
//! running, and the same session is back the next time (even after restarting the app: its address
//! is kept in `shared("float.terminal")`). It starts in the focused terminal's folder; if its shell
//! ends, the next show starts a new one. The window's top bar drags it and has a visible hide button.

use super::native::{icon, keyed, r, tone, z};
use super::*;
use crate::i18n;

pub(super) const ID: &str = "float-terminal";
const BLOCK: &str = "term";
const CLOSE: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/x.svg");
const KEY: &str = "float.terminal";

/// The session to show: the one kept, if the core still lists it, else none (start a new one).
pub(super) fn reusable(kept: Option<String>, alive: impl Fn(&str) -> bool) -> Option<String> {
    kept.filter(|a| !a.is_empty() && alive(a))
}

/// The window's layout: the bar and the terminal.
pub(super) fn layout(address: &str) -> Value {
    json!({
        "type": "column",
        "children": [
            { "id": "bar", "type": "block", "size": { "px": 36 }, "content": { "native": "float-bar" } },
            { "id": BLOCK, "type": "block", "content": { "terminal": address } }
        ]
    })
}

impl AppWindow {
    pub(super) fn native_float_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let muted = tone("muted");
        let ring = c("accent");
        let keys = self.key_hint("terminal.float");
        let app = cx.entity();
        div()
            .size_full()
            .pl(z(12.0))
            .pr(z(2.0))
            .gap(z(8.0))
            .flex()
            .items_center()
            .bg(c("surface"))
            .border_b_1()
            .border_color(c("border"))
            .text_size(z(12.0))
            .child(div().font_weight(FontWeight::SEMIBOLD).text_color(c("foreground")).child(i18n::t("float.title")))
            .child(div().flex_1().truncate().text_color(muted).child(keys.unwrap_or_default()))
            .child(
                keyed(
                    div()
                        .id("float-hide")
                        .flex_none()
                        .h(z(32.0))
                        .px(z(10.0))
                        .gap(z(6.0))
                        .flex()
                        .items_center()
                        .rounded(r(6.0))
                        .border_1()
                        .border_color(Hsla { a: 0.0, ..ring })
                        .text_color(muted)
                        .cursor_pointer()
                        .hover(|s| s.bg(c("hover")).text_color(c("foreground")))
                        .tooltip(super::native::tip(i18n::t("float.hide"))),
                    ring,
                    move |window, cx| app.update(cx, |this, cx| this.close_overlay(ID, window, cx)),
                )
                .child(icon(CLOSE, 14.0, muted)),
            )
            .into_any_element()
    }

    /// The first key bound to `command`, as the status bar writes it («⌘⌥A»).
    fn key_hint(&self, command: &str) -> Option<String> {
        let (_, keys, _) = crate::keys::listing().into_iter().find(|(_, _, action)| action == command)?;
        Some(super::native::key_parts(&keys).concat())
    }

    /// `terminal.float`: hides the window, or shows it (starting the shell the first time).
    pub(super) fn toggle_float_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        if self.overlays.iter().any(|o| o.id == ID) {
            self.close_overlay(ID, window, cx);
            return Ok(());
        }
        let kept = self.float_term.clone().or_else(|| mods::shared(KEY).and_then(|v| v.as_str().map(str::to_owned)));
        let address = match reusable(kept, |a| self.live.contains_key(a)) {
            Some(a) => a,
            None => {
                let here = self.model.focused_pane().and_then(|p| self.model.address_of(&p)).and_then(|a| self.cwds.get(&a).cloned());
                let folder = here.or_else(|| self.model.active_space().map(|s| s.folder.clone())).unwrap_or_else(home);
                // No `herdr.space` label: the app adopts a terminal that has one into a tab.
                let labels = json!({ "herdr.float": "1", "herdr.seen": "0" });
                self.require_document(None)?;
                let address = self.core.run(Vec::new(), Some(folder.clone()), labels, None, None, crate::terminal::initial_display(window)).map_err(|e| format!("{}: {e:#}", i18n::t("float.no_terminal")))?;
                self.born.borrow_mut().insert(address.clone(), Instant::now());
                self.cwds.insert(address.clone(), folder);
                mods::set_shared(KEY.into(), json!(address));
                address
            }
        };
        self.float_term = Some(address.clone());
        self.open_overlay(ID.into(), json!({ "kind": "window", "key": ID, "width": 0.6, "height": 0.55, "layout": layout(&address) }), window, cx);
        if let Some(t) = self.terms.get(&format!("ov{ID}/{BLOCK}")) {
            t.focus.focus(window, cx);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // `super::*` brings in gpui's own `test` attribute, which would shadow the standard one.
    use std::prelude::v1::test;

    #[test]
    fn a_kept_session_is_shown_again_only_while_the_core_lists_it() {
        assert_eq!(reusable(Some("t7".into()), |a| a == "t7"), Some("t7".to_owned()));
        assert_eq!(reusable(Some("t7".into()), |_| false), None);
        assert_eq!(reusable(Some(String::new()), |_| true), None);
        assert_eq!(reusable(None, |_| true), None);
    }

    #[test]
    fn the_window_is_a_bar_over_one_terminal_block() {
        let l = layout("t9");
        assert_eq!(l["children"][0]["content"]["native"], "float-bar");
        assert_eq!(l["children"][1]["content"]["terminal"], "t9");
        assert_eq!(l["children"][1]["id"], BLOCK);
    }
}
