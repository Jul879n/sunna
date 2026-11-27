//! Moving rows of the sidebar's list: by pointer (press a space, a project or a group header and drag; the
//! indicator shows where it lands, the list scrolls at its edges) and by keyboard (Alt+Up and Alt+Down move the
//! row the cursor is on). Where a drop lands is decided by `groups::resolve` and friends; what a drop does is a
//! command (`space.move`, `space.status`, `project.move`, `group.move`, `group.reorder`), so a script can do it too.

use super::groups::{self, Drop, Dragged, Rel};
use super::native::drop_line;
use super::view::{self, What};
use super::*;

/// How a row shows that something is about to land on or next to it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Marker {
    Before,
    After,
    Into,
}

/// What the list knows when a press turns into a drag.
#[derive(Clone, Debug)]
pub(super) struct Facts {
    /// Each space's project key.
    pub project_of: HashMap<String, String>,
    pub by_status: bool,
    pub by_project: bool,
}

pub(super) fn dragging() -> bool {
    with(|s| s.drag.as_ref().is_some_and(|d| d.active))
}

/// Esc during a drag: nothing lands, the row stays where it was, and the button's release afterwards does nothing
/// (no click on the row either). Returns whether a drag was on.
pub(super) fn cancel() -> bool {
    let was = with(|s| s.drag.take().is_some_and(|d| d.active));
    CANCELLED.with(|c| c.set(was));
    was
}

thread_local! {
    /// A drag was cancelled with Esc and the button is still down: the click it ends in is not a click.
    static CANCELLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// A header's click ends a drag (landed or cancelled): it must not fold or open anything.
pub(super) fn swallows_click() -> bool {
    dragging() || CANCELLED.with(std::cell::Cell::get)
}

/// How the row with this key shows the drop, if it is the target.
pub(super) fn marker(key: &str) -> Option<Marker> {
    with(|s| {
        let d = s.drag.as_ref().filter(|d| d.active)?;
        let (drop, over) = d.over.as_ref()?;
        (over == key).then_some(match drop {
            Drop::Space { after, .. } | Drop::Project { after, .. } | Drop::Group { after, .. } => {
                if *after { Marker::After } else { Marker::Before }
            }
            Drop::Status(_) | Drop::IntoGroup(_) => Marker::Into,
        })
    })
}

/// The row being dragged (drawn dim).
pub(super) fn is_source(key: &str) -> bool {
    with(|s| s.drag.as_ref().is_some_and(|d| d.active && d.what.what().key() == key))
}

/// A row that takes a drop: dim while it is the one dragged, tinted with a full accent border while the drop goes
/// into it, a line with dots on its edge while the drop goes next to it, and the grabbing hand while a drag is on.
pub(super) fn drop_style(el: Stateful<Div>, key: &str) -> Stateful<Div> {
    let m = marker(key);
    el.relative()
        .when(is_source(key), |e| e.opacity(0.5))
        .when(m == Some(Marker::Into), |e| e.bg(alpha(c("accent"), 0.15)).border_color(c("accent")))
        .when(m == Some(Marker::Before), |e| e.child(drop_line(true)))
        .when(m == Some(Marker::After), |e| e.child(drop_line(false)))
        .when(dragging(), |e| e.cursor(CursorStyle::ClosedHand))
}

/// Remembers that this row was pressed: it is a drag once the pointer moves.
pub(super) fn press(what: Dragged, at: Point<Pixels>, d: &Data, view: &view::View) {
    CANCELLED.with(|c| c.set(false));
    let rows = with(|s| ordered_all(&d.spaces, &d.panes, &s.git));
    let facts = Facts { project_of: rows.iter().map(|r| (r.id().to_owned(), project_key(r))).collect(), by_status: view.group_by == GroupBy::Status, by_project: view.group_by == GroupBy::Project };
    with(|s| s.drag = Some(Drag { what, start: at, active: false, pos: at, over: None, facts }));
}

/// What is under the pointer and how the drag resolves there.
fn retarget(s: &State, drag: &Drag) -> Option<(Drop, String)> {
    let mut rows: Vec<(String, f32, f32, What)> = s
        .stops
        .iter()
        .filter_map(|w| s.bounds.get(&w.key()).map(|b| (w.key(), f32::from(b.top()), f32::from(b.bottom()), w.clone())))
        .collect();
    rows.sort_by(|a, b| a.1.total_cmp(&b.1));
    let spans: Vec<(String, f32, f32)> = rows.iter().map(|(k, t, b, _)| (k.clone(), *t, *b)).collect();
    let (at, after) = groups::row_at(&spans, f32::from(drag.pos.y))?;
    let facts = &drag.facts;
    let same = |a: &str, b: &str| facts.project_of.get(a).is_some_and(|p| facts.project_of.get(b) == Some(p));
    let rel = Rel { same_project: &same, by_status: facts.by_status, by_project: facts.by_project };
    groups::resolve(&drag.what, &rows[at].3, after, &rel).map(|d| (d, rows[at].0.clone()))
}

impl AppWindow {
    /// The pointer moved over the list: starts the drag when it has moved enough, and finds where it would land.
    pub(super) fn drag_move(&mut self, e: &MouseMoveEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(mut drag) = with(|s| s.drag.clone()) else { return };
        if e.pressed_button != Some(MouseButton::Left) {
            with(|s| s.drag = None);
            cx.notify();
            return;
        }
        drag.pos = e.position;
        let started = !drag.active && groups::is_drag(f32::from(e.position.x - drag.start.x), f32::from(e.position.y - drag.start.y));
        if !drag.active && !started {
            with(|s| s.drag = Some(drag));
            return;
        }
        drag.active = true;
        with(|s| {
            drag.over = retarget(s, &drag);
            s.drag = Some(drag);
        });
        if started {
            // The list takes the keys for the drag's length: Esc cancels it.
            if let Some(focus) = with(|s| s.list_focus.clone()) {
                focus.focus(window, cx);
            }
            self.drag_scroll_timer(cx);
        }
        cx.notify();
    }

    /// While a drag is on, the list scrolls when the pointer rests near its top or bottom edge, and the target is looked for again.
    fn drag_scroll_timer(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_millis(16)).await;
                let alive = this.update(cx, |_, cx| {
                    let moved = with(|s| {
                        let drag = s.drag.clone().filter(|d| d.active)?;
                        let view = s.scroll.bounds();
                        let by = groups::edge_scroll(f32::from(view.top()), f32::from(view.bottom()), f32::from(drag.pos.y));
                        if by != 0.0 {
                            let mut offset = s.scroll.offset();
                            offset.y += px(by);
                            s.scroll.set_offset(offset);
                        }
                        let over = retarget(s, &drag);
                        if let Some(d) = s.drag.as_mut() {
                            d.over = over;
                        }
                        Some(by != 0.0)
                    });
                    if moved == Some(true) {
                        cx.notify();
                    }
                    moved.is_some()
                });
                if !alive.unwrap_or(false) {
                    break;
                }
            }
        })
        .detach();
    }

    /// The button went up over the list: what was dragged lands where the indicator was. Returns whether a drag was on
    /// (a plain click is not one: the row's own click handles it).
    pub(super) fn drag_release(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(drag) = with(|s| s.drag.take()) else { return false };
        cx.notify();
        if !drag.active {
            return false;
        }
        if let Some((drop, _)) = drag.over {
            self.apply_drop(&drag.what, &drop, window, cx);
        }
        true
    }

    /// Makes the sort Manual (a drop that orders spaces by hand means it).
    fn manual_sort(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if load_view().sort_by != SortBy::Manual {
            self.run("sidebar.view", json!({ "sort_by": "manual" }), window, cx);
        }
    }

    fn apply_drop(&mut self, from: &Dragged, drop: &Drop, window: &mut Window, cx: &mut Context<Self>) {
        let d = data();
        match (from, drop) {
            (Dragged::Space(id), Drop::Space { over, after }) => {
                let order: Vec<String> = d.spaces.iter().map(|w| text(w, "id").to_owned()).collect();
                if let Some(index) = groups::space_index(&order, id, over, *after) {
                    self.manual_sort(window, cx);
                    self.run("space.move", json!({ "id": id, "index": index }), window, cx);
                }
            }
            (Dragged::Space(id), Drop::Status(status)) => self.run("space.status", json!({ "id": id, "status": status }), window, cx),
            (Dragged::Project(key), Drop::Project { over, after }) => self.move_project(key, over, *after, window, cx),
            (Dragged::Project(key), Drop::IntoGroup(group)) => self.run("group.move", json!({ "key": key, "group": group }), window, cx),
            (Dragged::Group(id), Drop::Group { over, after }) => {
                let order: Vec<String> = groups::read_groups(mods::shared("sidebar.groups").as_ref()).into_iter().map(|g| g.id).collect();
                if let Some(index) = groups::space_index(&order, id, over, *after) {
                    self.run("group.reorder", json!({ "id": id, "index": index }), window, cx);
                }
            }
            _ => {}
        }
    }

    /// Puts project `key` next to `over` (in its group, or in the loose list).
    fn move_project(&mut self, key: &str, over: &str, after: bool, window: &mut Window, cx: &mut Context<Self>) {
        let d = data();
        let rows = with(|s| ordered_all(&d.spaces, &d.panes, &s.git));
        let present: Vec<String> = group_projects(&rows).into_iter().map(|g| g.key).collect();
        let loose = groups::manual_order(&shared_list("sidebar.project_keys"), &present);
        let pgs = groups::read_groups(mods::shared("sidebar.groups").as_ref());
        if let Some((group, index)) = groups::project_slot(&pgs, &loose, key, over, after) {
            let mut args = json!({ "key": key, "index": index });
            if let Some(g) = group {
                args["group"] = json!(g);
            }
            self.run("project.move", args, window, cx);
        }
    }

    /// Alt+Up and Alt+Down: the row the cursor is on goes one place up or down among its neighbors.
    pub(super) fn list_move(&mut self, what: &What, down: bool, window: &mut Window, cx: &mut Context<Self>) {
        let (d, view, lists) = (data(), load_view(), Lists::load());
        match what {
            What::Space(id) => {
                let rows = with(|s| ordered_all(&d.spaces, &d.panes, &s.git));
                let Some(me) = rows.iter().find(|r| r.id() == id) else { return };
                let key = project_key(me);
                let mine: Vec<String> = rows.iter().filter(|r| project_key(r) == key).map(|r| r.id().to_owned()).collect();
                let mine = groups::siblings(&mine, &groups::read_parents(mods::shared("space.parents").as_ref()), id);
                let Some(over) = groups::neighbor(&mine, id, down) else { return };
                self.apply_drop(&Dragged::Space(id.clone()), &Drop::Space { over, after: down }, window, cx);
            }
            What::Project(key) => {
                let rows = view_rows(&d, &view, &lists);
                let (_, rest) = view::split_pinned(rows, &lists.pinned);
                let arranged = arrange(&d, rest, &view, &lists);
                let list: Vec<String> = match arranged.groups.iter().find(|(g, _)| g.projects.contains(key)) {
                    Some((_, ps)) => ps.iter().map(|p| p.key.clone()).collect(),
                    None => arranged.loose.iter().map(|p| p.key.clone()).collect(),
                };
                let Some(over) = groups::neighbor(&list, key, down) else { return };
                self.move_project(key, &over, down, window, cx);
            }
            What::Group(id) => {
                let pgs = groups::read_groups(mods::shared("sidebar.groups").as_ref());
                let order: Vec<String> = pgs.into_iter().map(|g| g.id).collect();
                let Some(over) = groups::neighbor(&order, id, down) else { return };
                self.apply_drop(&Dragged::Group(id.clone()), &Drop::Group { over, after: down }, window, cx);
            }
            _ => return,
        }
        list_reveal(&what.key());
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // `super::*` brings gpui's own `test` macro: the plain one is the std attribute.
    use core::prelude::v1::test;

    #[test]
    fn a_row_shows_the_drop_only_while_a_drag_is_on_and_it_is_the_target() {
        with(|s| s.drag = None);
        assert!(!dragging() && marker("space:a").is_none() && !is_source("space:a"));
        let facts = Facts { project_of: HashMap::new(), by_status: false, by_project: true };
        let at = Point { x: px(0.0), y: px(0.0) };
        let mut drag = Drag { what: Dragged::Space("a".into()), start: at, active: false, pos: at, over: Some((Drop::Space { over: "b".into(), after: true }, "space:b".into())), facts };
        with(|s| s.drag = Some(drag.clone()));
        // Pressed, not yet moved: nothing shows.
        assert!(marker("space:b").is_none() && !is_source("space:a"));
        drag.active = true;
        with(|s| s.drag = Some(drag.clone()));
        assert_eq!(marker("space:b"), Some(Marker::After));
        assert_eq!(marker("space:c"), None);
        assert!(is_source("space:a") && dragging());
        drag.over = Some((Drop::IntoGroup("g1".into()), "group:g1".into()));
        with(|s| s.drag = Some(drag));
        assert_eq!(marker("group:g1"), Some(Marker::Into));
        // Esc: the drag is gone, nothing shows and nothing lands.
        assert!(cancel());
        assert!(swallows_click() && !dragging(), "the release after Esc is not a click");
        CANCELLED.with(|c| c.set(false));
        assert!(!swallows_click());
        assert!(!dragging() && marker("group:g1").is_none() && !cancel());
        // A press that never moved is not a drag to cancel (but it is forgotten all the same).
        with(|s| s.drag = Some(Drag { what: Dragged::Space("a".into()), start: at, active: false, pos: at, over: None, facts: Facts { project_of: HashMap::new(), by_status: false, by_project: true } }));
        assert!(!cancel() && with(|s| s.drag.is_none()));
        with(|s| s.drag = None);
    }
}
