//! Settings > Appearance > Right sidebar: which panels the sidebar offers, in
//! what order, and adding the panels mods bring (a mod whose mod.json has a `panel`).

use super::*;
use controls::{button, icon_button, switch};
use std::collections::BTreeMap;

pub const DEFAULT_PANELS: [&str; 3] = ["files", "changes", "search"];
const UP: &[u8] = include_bytes!("../../../assets/icons/settings/chevron-up.svg");
const DOWN: &[u8] = include_bytes!("../../../assets/icons/settings/chevron-down.svg");

/// One change to the sidebar's panels.
#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    Add(String),
    Remove(String),
    Move(String, i32),
}

#[derive(Default)]
pub struct State {
    /// mod folder -> the title its mod.json `panel` gives (a locale key or text).
    pub info: Option<BTreeMap<String, String>>,
}

/// The node called `id` somewhere in `node`.
pub fn find<'a>(node: &'a Value, id: &str) -> Option<&'a Value> {
    if node["id"] == id {
        return Some(node);
    }
    node["children"].as_array()?.iter().find_map(|c| find(c, id))
}

fn find_mut<'a>(node: &'a mut Value, id: &str) -> Option<&'a mut Value> {
    if node["id"] == id {
        return Some(node);
    }
    node["children"].as_array_mut()?.iter_mut().find_map(|c| find_mut(c, id))
}

/// The panel ids the sidebar offers now (`workspace.panels`, else the three shipped).
pub fn enabled() -> Vec<String> {
    match mods::shared("workspace.panels") {
        Some(Value::Array(l)) => l.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect(),
        _ => DEFAULT_PANELS.iter().map(|s| (*s).to_owned()).collect(),
    }
}

/// `(id, mod)` of each child of the panel body, in order.
pub fn rows(layout: &Value) -> Vec<(String, String)> {
    find(layout, "panel-body")
        .and_then(|b| b["children"].as_array())
        .map(|kids| {
            kids.iter()
                .map(|c| {
                    let id = c["id"].as_str().unwrap_or_default().to_owned();
                    (id.clone(), c["content"]["mod"].as_str().map_or(id, str::to_owned))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The tree after `op`, and the enabled list that goes with it. None when there is no panel body.
pub fn apply(layout: &Value, on: Vec<String>, op: &Op) -> Option<(Value, Vec<String>)> {
    let mut tree = layout.clone();
    let body = find_mut(&mut tree, "panel-body")?;
    let mut on = on;
    let kids = body["children"].as_array_mut()?;
    match op {
        Op::Add(m) => {
            let id = format!("panel-{m}");
            if !kids.iter().any(|k| k["id"] == id) {
                kids.push(json!({ "id": id, "type": "block", "content": { "mod": m } }));
            }
            on.push(id);
        }
        Op::Remove(id) => {
            kids.retain(|k| k["id"] != *id);
            on.retain(|x| x != id);
            if body["active"] == *id {
                body["active"] = body["children"].as_array().and_then(|k| k.first()).map_or(Value::Null, |k| k["id"].clone());
            }
        }
        Op::Move(id, dir) => {
            let i = kids.iter().position(|k| k["id"] == *id)?;
            let j = i as i32 + dir;
            if j >= 0 && (j as usize) < kids.len() {
                kids.swap(i, j as usize);
            }
        }
    }
    Some((tree, on))
}

fn text(k: &str) -> String {
    let t = i18n::t(k);
    if t != k { t } else { k.to_owned() }
}

fn load_info() -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let Ok(dir) = std::fs::read_dir(mods::app_dir().join("mods")) else { return out };
    for e in dir.flatten() {
        let Some(name) = e.file_name().to_str().map(str::to_owned) else { continue };
        let panel = std::fs::read_to_string(e.path().join("mod.json")).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()).map(|v| v["panel"].clone());
        if let Some(p) = panel.filter(Value::is_object) {
            out.insert(name.clone(), p["title"].as_str().map_or(name, str::to_owned));
        }
    }
    out
}

impl SettingsView {
    /// A structural change: on a Sunna layout, first on its personal copy (the way layout.move does).
    fn edit_sidebar(&mut self, op: Op, window: &mut Window, cx: &mut Context<Self>) {
        let name = store::get_str("layout");
        if shipped_layouts().contains(&name.as_str()) {
            let copy = format!("my-{name}");
            if layout_names().contains(&copy) {
                self.run("layout.switch", json!({ "name": copy }), window, cx);
            } else {
                self.run("layout.clone", json!({ "name": name, "as": copy }), window, cx);
            }
        }
        self.apply_sidebar(&op, window, cx);
    }

    fn apply_sidebar(&mut self, op: &Op, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((tree, on)) = apply(&mods::layout_snapshot(), enabled(), op) {
            mods::set_shared("workspace.panels".into(), json!(on));
            self.run("layout.set", json!({ "layout": tree }), window, cx);
        }
    }

    fn toggle_panel(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let on = enabled();
        let next: Vec<String> = if on.iter().any(|x| x == id) { on.iter().filter(|x| *x != id).cloned().collect() } else { on.iter().cloned().chain([id.to_owned()]).collect() };
        mods::set_shared("workspace.panels".into(), json!(next));
        let active = find(&mods::layout_snapshot(), "panel-body").map(|b| b["active"].clone());
        if on.iter().any(|x| x == id) && (active == Some(json!(id)) || next.is_empty()) {
            self.run("layout.hide", json!({ "id": "panel" }), window, cx);
        }
    }

    pub(super) fn sidebar(&mut self, cx: &mut Context<Self>) -> Div {
        let z = settings::z;
        let info = self.sidebar.info.get_or_insert_with(load_info).clone();
        let layout = mods::layout_snapshot();
        let mut out = div().w_full().child(controls::heading(i18n::t("settings.sidebar")));
        if find(&layout, "panel-body").is_none() {
            return out.child(controls::note(i18n::t("settings.sidebar_unsupported")));
        }
        let rows = rows(&layout);
        let on = enabled();
        let held: Vec<&String> = rows.iter().map(|(_, m)| m).collect();
        let me = cx.entity().downgrade();
        let line = |id: String| div().id(SharedString::from(id)).w_full().min_h(z(44.0)).mb(z(8.0)).px(z(16.0)).flex().flex_row().items_center().gap(z(8.0)).rounded(z(10.0)).border_1().border_color(c("border")).bg(c("surface"));
        let n = rows.len();
        for (i, (id, m)) in rows.iter().enumerate() {
            let title = info.get(m).map_or_else(|| id.clone(), |t| text(t));
            let enabled_now = on.contains(id);
            let mv = |dir: i32| {
                let id = id.clone();
                Self::act_on(&me, move |this, w, cx| this.edit_sidebar(Op::Move(id.clone(), dir), w, cx))
            };
            let mut l = line(format!("st-sb-{id}")).child(div().flex_1().min_w_0().truncate().text_size(z(14.0)).text_color(if enabled_now { c("foreground") } else { c("muted") }).child(title));
            l = l.child(if i > 0 { icon_button(format!("st-sb-up-{id}"), UP, mv(-1)) } else { icon_button_blank() });
            l = l.child(if i + 1 < n { icon_button(format!("st-sb-down-{id}"), DOWN, mv(1)) } else { icon_button_blank() });
            if id.starts_with("panel-") {
                let rid = id.clone();
                l = l.child(button(format!("st-sb-rm-{id}"), i18n::t("settings.sidebar_remove"), Self::act_on(&me, move |this, w, cx| this.edit_sidebar(Op::Remove(rid.clone()), w, cx))));
            }
            let tid = id.clone();
            l = l.child(switch(format!("st-sb-t-{id}"), enabled_now, Self::act_on(&me, move |this, w, cx| this.toggle_panel(&tid, w, cx))));
            out = out.child(l);
        }
        for (m, t) in info.iter().filter(|(m, _)| !held.contains(m)) {
            let mid = m.clone();
            out = out.child(
                line(format!("st-sb-c-{m}"))
                    .child(div().flex_1().min_w_0().truncate().text_size(z(14.0)).text_color(tone("muted")).child(text(t)))
                    .child(button(format!("st-sb-add-{m}"), i18n::t("settings.sidebar_add"), Self::act_on(&me, move |this, w, cx| this.edit_sidebar(Op::Add(mid.clone()), w, cx)))),
            );
        }
        out
    }
}

fn icon_button_blank() -> Stateful<Div> {
    div().id("st-sb-blank").size(settings::z(32.0)).flex_none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::prelude::v1::test; // gpui::* (through super) has a `test` too

    fn tree() -> Value {
        serde_json::from_str(
            r#"{ "id": "root", "type": "row", "children": [
                { "id": "main", "type": "block" },
                { "id": "panel", "type": "column", "children": [
                    { "id": "panel-body", "type": "stack", "active": "files", "children": [
                        { "id": "files", "type": "block", "content": { "mod": "files" } },
                        { "id": "changes", "type": "block", "content": { "mod": "changes" } },
                        { "id": "panel-notes", "type": "block", "content": { "mod": "notes" } }
                    ] }
                ] }
            ] }"#,
        )
        .unwrap()
    }

    fn ids(t: &Value) -> Vec<String> {
        rows(t).into_iter().map(|r| r.0).collect()
    }

    #[test]
    fn rows_name_each_panel_and_its_mod() {
        let want: Vec<(String, String)> = [("files", "files"), ("changes", "changes"), ("panel-notes", "notes")].iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
        assert_eq!(rows(&tree()), want);
        let bare: Value = serde_json::from_str(r#"{"id":"x"}"#).unwrap();
        assert!(rows(&bare).is_empty());
    }

    #[test]
    fn moving_swaps_with_the_neighbor_and_stops_at_the_ends() {
        let (t, _) = apply(&tree(), vec![], &Op::Move("changes".into(), -1)).unwrap();
        assert_eq!(ids(&t), ["changes", "files", "panel-notes"]);
        let (t, _) = apply(&tree(), vec![], &Op::Move("files".into(), -1)).unwrap();
        assert_eq!(ids(&t), ["files", "changes", "panel-notes"]);
        let (t, _) = apply(&tree(), vec![], &Op::Move("panel-notes".into(), 1)).unwrap();
        assert_eq!(ids(&t), ["files", "changes", "panel-notes"]);
    }

    #[test]
    fn adding_a_panel_once_enables_it_and_removing_moves_the_active_one() {
        let (t, on) = apply(&tree(), vec!["files".into()], &Op::Add("todo".into())).unwrap();
        assert_eq!(ids(&t).last().unwrap(), "panel-todo");
        assert_eq!(on, ["files", "panel-todo"]);
        assert_eq!(find(&t, "panel-todo").unwrap()["content"]["mod"], "todo");
        let (t, _) = apply(&t, on, &Op::Add("todo".into())).unwrap();
        assert_eq!(ids(&t).iter().filter(|i| *i == "panel-todo").count(), 1);
        let (t, on) = apply(&tree(), vec!["files".into(), "panel-notes".into()], &Op::Remove("files".into())).unwrap();
        assert_eq!(ids(&t), ["changes", "panel-notes"]);
        assert_eq!(on, ["panel-notes"]);
        assert_eq!(find(&t, "panel-body").unwrap()["active"], "changes");
    }

    #[test]
    fn without_a_panel_body_nothing_changes() {
        assert!(apply(&json!({ "id": "root", "children": [] }), vec![], &Op::Add("x".into())).is_none());
    }
}
