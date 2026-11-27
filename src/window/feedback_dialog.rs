//! Send feedback, native: the form of `app.feedback {message?, from?}` in the dialog every window uses. The
//! person writes a message (up to 4000 characters), may add up to 3 screenshots (paste, pick or drop) and an
//! email, and sees the exact JSON that will be sent. Nothing leaves until Send; a build without an endpoint
//! says so and cannot send. The data is `crate::feedback` as before.

use super::dialog::{self, Act, Dialog};
use super::native::{icon, outline, tone, z};
use super::*;
use crate::feedback;
use crate::i18n;
use gpui_base::input::{InputEvent, InputState, Textarea, TextareaState};

/// Overlay id: opening it again replaces it.
pub(super) const ID: &str = "feedback";
const WIDTH: f32 = 560.0;
const CLOSE_ICON: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/x.svg");
const THUMB_H: f32 = 72.0;

/// Can the form be sent now? A message, a build that can send, and nothing already going out.
pub(super) fn can_send(message: &str, available: bool, sending: bool) -> bool {
    available && !sending && !message.trim().is_empty()
}

pub(super) struct FeedbackView {
    app: WeakEntity<AppWindow>,
    message: Entity<TextareaState>,
    email: Entity<InputState>,
    focus: dialog::Focus,
    from: String,
    layout: String,
    shots: Vec<String>,
    error: Option<String>,
    sending: bool,
    sent: bool,
}

impl FeedbackView {
    fn new(app: WeakEntity<AppWindow>, message: String, from: String, layout: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
        feedback::clear_shots();
        let area = cx.new(|cx| TextareaState::new(window, cx).default_value(message).placeholder(i18n::t("feedback.what_happened_what_did_you")).rows(5));
        area.update(cx, |s, _| s.set_editor_style(dialog::input_style()));
        cx.subscribe_in(&area, window, |this, area, ev: &InputEvent, _, cx| match ev {
            InputEvent::Change => {
                this.error = None;
                dialog::mark_typed(area.entity_id(), !area.read(cx).value().is_empty());
                cx.notify();
            }
            InputEvent::PasteWithoutText => this.paste_image(cx),
            InputEvent::Focus | InputEvent::Blur => cx.notify(),
            _ => {}
        })
        .detach();
        let email = dialog::input("", &i18n::t("feedback.email_optional"), window, cx, |this: &mut Self, ev, _, cx| {
            if matches!(ev, InputEvent::Change) {
                this.error = None;
                cx.notify();
            }
        });
        area.update(cx, |s, cx| s.focus(window, cx));
        Self { app, message: area, email, focus: dialog::Focus::new(cx), from, layout, shots: Vec::new(), error: None, sending: false, sent: false }
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let _ = self.app.update(cx, |app, cx| app.close_overlay(ID, window, cx));
    }

    /// Takes `{ path }`s up to the limit; the first `{ error }` is shown.
    fn add(&mut self, results: Vec<Value>) {
        self.error = None;
        for r in results {
            if let Some(e) = r.get("error").and_then(Value::as_str) {
                self.error = Some(e.to_owned());
            } else if self.shots.len() >= feedback::MAX_IMAGES {
                self.error = Some(i18n::t("feedback.up_to_3_screenshots"));
            } else if let Some(p) = r.get("path").and_then(Value::as_str) {
                self.shots.push(p.to_owned());
            }
        }
    }

    /// A paste with no text: a screenshot, if the clipboard holds one.
    fn paste_image(&mut self, cx: &mut Context<Self>) {
        if self.shots.len() >= feedback::MAX_IMAGES {
            self.error = Some(i18n::t("feedback.up_to_3_screenshots"));
        } else if let Some(bytes) = cx.read_from_clipboard().and_then(|c| c.entries().iter().find_map(|e| match e {
            ClipboardEntry::Image(i) => Some(i.bytes.clone()),
            _ => None,
        })) {
            self.add(vec![match feedback::attach(&bytes) { Ok(path) => json!({ "path": path }), Err(e) => json!({ "error": e }) }]);
        }
        cx.notify();
    }

    fn dropped(&mut self, paths: Vec<std::path::PathBuf>, cx: &mut Context<Self>) {
        if self.sending || self.sent {
            return;
        }
        let room = feedback::MAX_IMAGES.saturating_sub(self.shots.len());
        if room == 0 {
            self.error = Some(i18n::t("feedback.up_to_3_screenshots"));
        } else {
            self.add(feedback::attach_files(&paths[..paths.len().min(room)]));
        }
        cx.notify();
    }

    fn pick(&mut self, cx: &mut Context<Self>) {
        let picked = cx.prompt_for_paths(PathPromptOptions { files: true, directories: false, multiple: true, prompt: Some(crate::i18n::t("rs.window.add_files").into()) });
        cx.spawn(async move |this, cx| {
            let paths = picked.await.ok().and_then(Result::ok).flatten().unwrap_or_default();
            let out = feedback::attach_files(&paths);
            let _ = this.update(cx, |this, cx| {
                this.add(out);
                cx.notify();
            });
        })
        .detach();
    }

    fn body(&self, cx: &App) -> Value {
        feedback::draft(&self.message.read(cx).value(), &self.email.read(cx).value(), self.shots.len(), &self.from, &self.layout)
    }

    fn send(&mut self, cx: &mut Context<Self>) {
        let message = self.message.read(cx).value().to_string();
        if !can_send(&message, feedback::available(), self.sending) || self.sent {
            return;
        }
        self.sending = true;
        self.error = None;
        let (body, shots) = (self.body(cx), self.shots.clone());
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = feedback::send(body, shots).await;
            let _ = this.update(cx, |this, cx| {
                this.sending = false;
                match result {
                    Ok(()) => this.sent = true,
                    Err(why) => this.error = Some(why),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn thumbs(&self, cx: &mut Context<Self>) -> Div {
        let mut row = div().flex().flex_row().flex_wrap().gap(z(8.0)).items_center();
        for (i, path) in self.shots.iter().enumerate() {
            row = row.child(
                div()
                    .id(SharedString::from(format!("shot-{i}")))
                    .relative()
                    .h(z(THUMB_H))
                    .rounded(z(6.0))
                    .overflow_hidden()
                    .border_1()
                    .border_color(c("border"))
                    .child(img(mods::app_dir().join(path)).h(z(THUMB_H - 2.0)))
                    .child(
                        div()
                            .id(SharedString::from(format!("shot-x-{i}")))
                            .absolute()
                            .top(z(2.0))
                            .right(z(2.0))
                            .size(z(32.0))
                            .rounded(z(8.0))
                            .bg(c("raised"))
                            .border_1()
                            .border_color(c("border"))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .focusable()
                            .tab_stop(true)
                            .focus_visible(|s| s.border_2().border_color(graphic("accent")))
                            .tooltip(super::native::tip(i18n::t("menu.close")))
                            .on_key_down(cx.listener(move |this, e: &KeyDownEvent, _, cx| {
                                if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                                    this.shots.remove(i.min(this.shots.len().saturating_sub(1)));
                                    cx.stop_propagation();
                                    cx.notify();
                                }
                            }))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if i < this.shots.len() {
                                    this.shots.remove(i);
                                }
                                this.error = None;
                                cx.notify();
                            }))
                            .child(icon(CLOSE_ICON, 12.0, c("foreground"))),
                    ),
            );
        }
        if self.shots.len() < feedback::MAX_IMAGES {
            let this = cx.entity();
            row = row.child(dialog::side_button("feedback-add", i18n::t("feedback.add_screenshot"), move |_, cx| this.update(cx, |v, cx| v.pick(cx))));
        }
        row
    }
}

impl Render for FeedbackView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity();
        let close = dialog::run(&this, |v, w, cx| v.close(w, cx));
        let available = feedback::available();
        let message = self.message.read(cx).value().to_string();
        let body = if self.sent {
            div().py(z(12.0)).text_size(z(15.0)).child(i18n::t("feedback.sent_thanks_for_helping_us"))
        } else {
            let json = serde_json::to_string_pretty(&self.body(cx)).unwrap_or_default();
            let area_focused = self.message.read(cx).focus_handle(cx).is_focused(window);
            div()
                .flex()
                .flex_col()
                .gap(z(10.0))
                .when(!available, |d| d.child(dialog::message(i18n::t("feedback.this_build_was_made_from"), true)))
                .when(self.from == "agent", |d| d.child(dialog::message(i18n::t("feedback.drafted_by_an_agent_read"), false)))
                .child(dialog::label(i18n::t("feedback.your_message_up_to_4000")))
                .child(
                    dialog::field_box(area_focused, false)
                        .h(z(124.0))
                        .items_start()
                        .py(z(6.0))
                        .child(div().flex_1().h_full().child(Textarea::new(&self.message))),
                )
                .child(self.thumbs(cx))
                .child(dialog::field(i18n::t("feedback.email_only_if_you_want"), &self.email, false, window, cx))
                .child(dialog::label(i18n::t("feedback.what_we_send_exactly_this")))
                .child(
                    div()
                        .id("feedback-json")
                        .max_h(z(120.0))
                        .overflow_y_scroll()
                        .p(z(10.0))
                        .rounded(z(8.0))
                        .border_1()
                        .border_color(outline())
                        .bg(c("background"))
                        .text_size(z(11.0))
                        .line_height(z(15.0))
                        .text_color(tone("muted"))
                        .children(json.lines().map(|l| div().child(l.replace(' ', "\u{a0}")))),
                )
                .when_some(self.error.clone(), |d, e| d.child(dialog::message(e, true)))
        };
        let mut card = Dialog::new(i18n::t("feedback.send_feedback"), &self.focus, close).width(WIDTH).fields(2).body(body);
        card = if self.sent {
            card.no_cancel().action(Act::primary(i18n::t("feedback.done"), dialog::run(&this, |v, w, cx| v.close(w, cx))))
        } else {
            let send = dialog::run(&this, |v, _, cx| v.send(cx));
            let label = i18n::t(if self.sending { "feedback.sending" } else { "feedback.send" });
            card.cancel_label(i18n::t("feedback.cancel")).action(Act::primary(label, send).enabled(can_send(&message, available, self.sending)))
        };
        let elem = card.render("feedback");
        div().on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| this.dropped(paths.paths().to_vec(), cx))).child(elem)
    }
}

impl AppWindow {
    /// `app.feedback`: the form, prefilled with `message`; `from` is `person` or `agent`.
    pub(super) fn open_feedback(&mut self, message: String, from: String, window: &mut Window, cx: &mut Context<Self>) {
        self.close_overlay(ID, window, cx);
        let (app, layout) = (cx.weak_entity(), self.layout_name.clone());
        let view = cx.new(|cx| FeedbackView::new(app, message, from, layout, window, cx));
        self.push_dialog_overlay(ID.into(), WIDTH + 48.0, view.into(), window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // `super::*` brings gpui's own `test` macro: the plain one is the std attribute.
    use core::prelude::v1::test;

    #[test]
    fn it_sends_only_with_a_message_a_build_that_can_and_nothing_going_out() {
        assert!(can_send("hello", true, false));
        assert!(!can_send("   ", true, false));
        assert!(!can_send("hello", false, false));
        assert!(!can_send("hello", true, true));
    }
}
