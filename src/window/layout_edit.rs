//! Layout edit mode: drag a block of the shown layout next to another one.
use super::*;

pub(super) struct EditDrag {
    id: String,
    down: Point<Pixels>,
    moving: bool,
    /// The block the pointer is over, the axis of its parent and whether the drop lands before it.
    target: Option<(String, Axis, bool)>,
}

/// Is `at` in the first half of `bounds` along `axis`?
fn before_half(bounds: Bounds<Pixels>, axis: Axis, at: Point<Pixels>) -> bool {
    match axis {
        Axis::Row => at.x < bounds.origin.x + bounds.size.width / 2.,
        Axis::Column => at.y < bounds.origin.y + bounds.size.height / 2.,
    }
}

impl AppWindow {
    /// The deepest shown block under `at` that sits in a row or column,
    /// leaving out `skip` and everything inside it.
    fn edit_at(&self, at: Point<Pixels>, skip: Option<&str>) -> Option<(String, Axis, Bounds<Pixels>)> {
        let bounds = self.bounds.borrow();
        let skipped = skip.and_then(|id| self.chrome.find(id));
        let mut found = None;
        for parent in self.chrome.walk() {
            if !matches!(parent.kind, Kind::Row { .. } | Kind::Column { .. }) {
                continue;
            }
            let Some(axis) = parent.axis() else { continue };
            for child in parent.children().iter().filter(|c| !c.hidden) {
                if skipped.is_some_and(|s| s.find(&child.id).is_some()) {
                    continue;
                }
                if let Some(b) = bounds.get(&child.id).filter(|b| b.contains(&at)) {
                    found = Some((child.id.clone(), axis, *b));
                }
            }
        }
        found
    }

    pub(super) fn toggle_layout_edit(&mut self, cx: &mut Context<Self>) {
        self.layout_edit = !self.layout_edit;
        self.pane_drag = None;
        self.edit_drag = None;
        self.edit_hover = None;
        cx.notify();
    }

    fn edit_press(&mut self, at: Point<Pixels>, cx: &mut Context<Self>) {
        if let Some((id, ..)) = self.edit_at(at, None) {
            self.edit_drag = Some(EditDrag { id, down: at, moving: false, target: None });
            cx.notify();
        }
    }

    fn edit_move(&mut self, at: Point<Pixels>, pressed: bool, cx: &mut Context<Self>) {
        let Some(d) = self.edit_drag.as_ref() else {
            let hover = self.edit_at(at, None).map(|(id, ..)| id);
            if hover != self.edit_hover {
                self.edit_hover = hover;
                cx.notify();
            }
            return;
        };
        if !pressed {
            self.edit_drag = None;
            cx.notify();
            return;
        }
        if !d.moving && (at.x - d.down.x).abs().max((at.y - d.down.y).abs()) < settings::z(4.) {
            return;
        }
        let target = self.edit_at(at, Some(&d.id)).map(|(id, axis, b)| (id, axis, before_half(b, axis, at)));
        let d = self.edit_drag.as_mut().unwrap();
        d.moving = true;
        d.target = target;
        cx.notify();
    }

    fn edit_release(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(d) = self.edit_drag.take() else { return false };
        if let (true, Some((target, _, before))) = (d.moving, d.target) {
            let side = if before { "before" } else { "after" };
            self.run("layout.move", json!({ "id": d.id, side: target }), window, cx);
        }
        cx.notify();
        true
    }

    /// The edit mode's layer: outlines, the drop preview, a banner to leave.
    pub(super) fn layout_edit_layer(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.layout_edit {
            return None;
        }
        let outline = |b: Bounds<Pixels>, fill: f32| {
            div().absolute().left(b.origin.x).top(b.origin.y).w(b.size.width).h(b.size.height)
                .bg(c("accent").opacity(fill)).border_2().border_color(c("accent"))
        };
        let bounds = self.bounds.borrow();
        let dragged = self.edit_drag.as_ref().filter(|d| d.moving).map(|d| d.id.as_str());
        let marked = dragged.or(self.edit_hover.as_deref()).and_then(|id| bounds.get(id)).map(|b| outline(*b, 0.08));
        let preview = self.edit_drag.as_ref().and_then(|d| d.target.as_ref()).and_then(|(id, axis, before)| {
            let mut b = *bounds.get(id)?;
            match (*axis, *before) {
                (Axis::Row, true) => b.size.width /= 2.,
                (Axis::Row, false) => { b.size.width /= 2.; b.origin.x += b.size.width; }
                (Axis::Column, true) => b.size.height /= 2.,
                (Axis::Column, false) => { b.size.height /= 2.; b.origin.y += b.size.height; }
            }
            Some(outline(b, 0.25))
        });
        let banner = div().id("layout-edit-banner").absolute().top(settings::z(10.)).left(relative(0.5)).ml(-settings::z(190.))
            .w(settings::z(380.)).h(settings::z(44.)).px(settings::z(12.)).rounded(settings::z(8.))
            .bg(c("raised")).border_1().border_color(c("border")).occlude()
            .flex().items_center().gap(settings::z(10.))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(div().flex_1().text_size(settings::z(12.5)).text_color(c("foreground")).child(crate::i18n::t("layout_edit.hint")))
            .child(div().id("layout-edit-done").h(settings::z(32.)).px(settings::z(10.)).rounded(settings::z(6.)).flex().items_center()
                .bg(c("accent")).text_size(settings::z(12.5)).text_color(c("background")).cursor_pointer()
                .on_click(cx.listener(|this, _, _, cx| this.toggle_layout_edit(cx)))
                .child(crate::i18n::t("layout_edit.done")));
        let this = cx.entity().downgrade();
        Some(div().id("layout-edit-layer").absolute().inset_0().occlude()
            .cursor(if self.edit_drag.as_ref().is_some_and(|d| d.moving) { CursorStyle::ClosedHand } else { CursorStyle::OpenHand })
            .on_mouse_down(MouseButton::Left, cx.listener(|this, ev: &MouseDownEvent, _, cx| this.edit_press(ev.position, cx)))
            .children(marked).children(preview).child(banner)
            .child(canvas(
                |_, _, _| {},
                move |_, _, window, _| {
                    let t = this.clone();
                    window.on_mouse_event(move |ev: &MouseMoveEvent, phase, _, cx| {
                        if phase == DispatchPhase::Capture {
                            let _ = t.update(cx, |this, cx| this.edit_move(ev.position, ev.pressed_button == Some(MouseButton::Left), cx));
                        }
                    });
                    let t = this.clone();
                    window.on_mouse_event(move |ev: &MouseUpEvent, phase, window, cx| {
                        if phase == DispatchPhase::Capture && ev.button == MouseButton::Left
                            && t.update(cx, |this, cx| this.edit_release(window, cx)).unwrap_or(false)
                        { cx.stop_propagation(); }
                    });
                },
            ).absolute().size_full())
            .into_any_element())
    }
}
