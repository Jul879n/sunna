//! What the sidebar's list lets a person arrange: project groups (`sidebar.groups`), the manual order of
//! projects (`sidebar.project_keys`), the parent of a worktree space (`space.parents`) and where a dragged
//! row lands. Everything that decides is a plain function (tested at the end); `group_command` runs the
//! commands (`group.*`, `project.move`, `space.parent`, `space.worktree`), so `sunna do` can do all that a
//! drag or a menu does.

use super::view::{self, What};
use super::*;
use serde::{Deserialize, Serialize};

/// The commands this file runs (the docs and the tests name each one).
pub(crate) const COMMANDS: [&str; 9] = ["group.new", "group.rename", "group.color", "group.delete", "group.move", "group.reorder", "group.refresh", "project.move", "space.parent"];

/// Opening the new worktree dialog from a space (its menu): the child it makes is that space's.
pub(crate) const WORKTREE_COMMAND: &str = "space.worktree";

pub(crate) fn handles(name: &str) -> bool {
    COMMANDS.contains(&name) || name == WORKTREE_COMMAND
}

// Project groups ---------------------------------------------------------------------

/// A group the person made: a folder of projects in the sidebar. `projects` are project keys, in the order shown.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct PGroup {
    pub id: String,
    pub name: String,
    pub color: String,
    pub projects: Vec<String>,
}

/// The saved groups. Anything unreadable is left out; a project listed twice stays in the first group.
pub(super) fn read_groups(v: Option<&Value>) -> Vec<PGroup> {
    let Some(Value::Array(list)) = v else { return Vec::new() };
    let (mut ids, mut keys) = (BTreeSet::new(), BTreeSet::new());
    list.iter()
        .filter_map(|g| serde_json::from_value::<PGroup>(g.clone()).ok())
        .filter(|g| !g.id.is_empty() && ids.insert(g.id.clone()))
        .map(|mut g| {
            g.name = g.name.trim().to_owned();
            g.projects.retain(|k| keys.insert(k.clone()));
            if !PALETTE.contains(&g.color.as_str()) {
                g.color = PALETTE[0].to_owned();
            }
            g
        })
        .collect()
}

pub(super) fn groups_value(groups: &[PGroup]) -> Value {
    serde_json::to_value(groups).unwrap_or_else(|_| json!([]))
}

fn find<'a>(groups: &'a mut [PGroup], id: &str) -> Result<&'a mut PGroup, String> {
    groups.iter_mut().find(|g| g.id == id).ok_or_else(|| format!("no group {id:?}"))
}

fn fresh_id(groups: &[PGroup]) -> String {
    let next = groups.iter().filter_map(|g| g.id.strip_prefix('g').and_then(|n| n.parse::<u32>().ok())).max().unwrap_or(0) + 1;
    format!("g{next}")
}

fn checked_name(groups: &[PGroup], name: &str, except: Option<&str>) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("a group needs a name".into());
    }
    if groups.iter().any(|g| Some(g.id.as_str()) != except && g.name.to_lowercase() == name.to_lowercase()) {
        return Err(format!("there is already a group named {name:?}"));
    }
    Ok(name.to_owned())
}

/// The hue the fewest groups wear.
fn least_used_color(groups: &[PGroup]) -> &'static str {
    let uses = |c: &str| groups.iter().filter(|g| g.color == c).count();
    PALETTE.iter().fold(PALETTE[0], |best, c| if uses(c) < uses(best) { c } else { best })
}

/// A new group (last) holding `project`, which leaves the group it was in. Its id.
pub(super) fn new_group(groups: &mut Vec<PGroup>, name: &str, project: &str, color: Option<&str>) -> Result<String, String> {
    let name = checked_name(groups, name, None)?;
    let color = match color {
        Some(c) if PALETTE.contains(&c) => c,
        Some(c) => return Err(format!("color must be one of {}, not {c:?}", PALETTE.join(", "))),
        None => least_used_color(groups),
    };
    for g in groups.iter_mut() {
        g.projects.retain(|k| k != project);
    }
    let id = fresh_id(groups);
    groups.push(PGroup { id: id.clone(), name, color: color.to_owned(), projects: vec![project.to_owned()] });
    Ok(id)
}

pub(super) fn rename_group(groups: &mut [PGroup], id: &str, name: &str) -> Result<(), String> {
    let name = checked_name(groups, name, Some(id))?;
    find(groups, id)?.name = name;
    Ok(())
}

pub(super) fn recolor_group(groups: &mut [PGroup], id: &str, color: &str) -> Result<(), String> {
    if !PALETTE.contains(&color) {
        return Err(format!("color must be one of {}", PALETTE.join(", ")));
    }
    find(groups, id)?.color = color.to_owned();
    Ok(())
}

/// The group is gone; its projects go back to the list.
pub(super) fn delete_group(groups: &mut Vec<PGroup>, id: &str) -> Result<PGroup, String> {
    let at = groups.iter().position(|g| g.id == id).ok_or_else(|| format!("no group {id:?}"))?;
    Ok(groups.remove(at))
}

/// Moves `from` to final position `to` (0-based, clamped).
pub(super) fn vec_move<T>(list: &mut Vec<T>, from: usize, to: usize) {
    if from >= list.len() {
        return;
    }
    let item = list.remove(from);
    list.insert(to.min(list.len()), item);
}

/// Puts a group at position `index` (1 = first).
pub(super) fn reorder_group(groups: &mut Vec<PGroup>, id: &str, index: usize) -> Result<(), String> {
    let from = groups.iter().position(|g| g.id == id).ok_or_else(|| format!("no group {id:?}"))?;
    vec_move(groups, from, index.saturating_sub(1));
    Ok(())
}

/// Puts `key` in group `to` at position `index` (1 = first, none: last), or out of every group (`to` none).
pub(super) fn place_project(groups: &mut [PGroup], key: &str, to: Option<&str>, index: Option<usize>) -> Result<(), String> {
    if let Some(id) = to {
        find(groups, id)?;
    }
    for g in groups.iter_mut() {
        g.projects.retain(|k| k != key);
    }
    if let Some(id) = to {
        let g = find(groups, id)?;
        let at = index.map_or(g.projects.len(), |i| i.saturating_sub(1).min(g.projects.len()));
        g.projects.insert(at, key.to_owned());
    }
    Ok(())
}

/// The group a project is in.
pub(super) fn group_of<'a>(groups: &'a [PGroup], key: &str) -> Option<&'a PGroup> {
    groups.iter().find(|g| g.projects.iter().any(|k| k == key))
}

/// The manual order of projects: the saved keys that still exist (in the saved order), then the new ones as first seen.
pub(super) fn manual_order(saved: &[String], present: &[String]) -> Vec<String> {
    let mut out: Vec<String> = saved.iter().filter(|k| present.contains(k)).cloned().collect();
    out.dedup();
    let seen: BTreeSet<String> = out.iter().cloned().collect();
    out.extend(present.iter().filter(|k| !seen.contains(*k)).cloned());
    out
}

/// What the list shows: each group (in order) with its projects, then the projects that are in none.
pub(super) fn layout(pgs: &[PGroup], mut projects: Vec<Group>) -> (Vec<(PGroup, Vec<Group>)>, Vec<Group>) {
    let mut out = Vec::new();
    for pg in pgs {
        let mut mine = Vec::new();
        for k in &pg.projects {
            if let Some(i) = projects.iter().position(|g| &g.key == k) {
                mine.push(projects.remove(i));
            }
        }
        out.push((pg.clone(), mine));
    }
    (out, projects)
}

// Where a dragged row lands -------------------------------------------------------------

/// What is being dragged.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Dragged {
    Space(String),
    Project(String),
    Group(String),
}

impl Dragged {
    /// The list row it is (`What::key`).
    pub(super) fn what(&self) -> What {
        match self {
            Dragged::Space(id) => What::Space(id.clone()),
            Dragged::Project(key) => What::Project(key.clone()),
            Dragged::Group(id) => What::Group(id.clone()),
        }
    }
}

/// What dropping does.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Drop {
    /// Next to another space of the same project.
    Space { over: String, after: bool },
    /// Onto a state's heading: the space takes that state (its `Status::id`).
    Status(String),
    /// Next to another project.
    Project { over: String, after: bool },
    /// Onto a group's heading: the project goes in, last.
    IntoGroup(String),
    /// Next to another group.
    Group { over: String, after: bool },
}

/// What the rules need to know about the list.
pub(super) struct Rel<'a> {
    /// Are these two spaces in one project?
    pub same_project: &'a dyn Fn(&str, &str) -> bool,
    pub by_status: bool,
    pub by_project: bool,
}

/// Where dropping `drag` on `over` (its upper or lower half) goes: nowhere (`None`) when the list does not take it.
pub(super) fn resolve(drag: &Dragged, over: &What, after: bool, rel: &Rel) -> Option<Drop> {
    match (drag, over) {
        (Dragged::Space(a), What::Space(b)) if a != b && (rel.same_project)(a, b) => Some(Drop::Space { over: b.clone(), after }),
        (Dragged::Space(_), What::Section(key)) if rel.by_status => key.strip_prefix("status:").map(|s| Drop::Status(s.to_owned())),
        (Dragged::Project(a), What::Project(b)) if a != b && rel.by_project => Some(Drop::Project { over: b.clone(), after }),
        (Dragged::Project(_), What::Group(g)) if rel.by_project => Some(Drop::IntoGroup(g.clone())),
        (Dragged::Group(a), What::Group(b)) if a != b => Some(Drop::Group { over: b.clone(), after }),
        _ => None,
    }
}

/// The final position (1 = first) of `from` in `order` once it sits next to `over`.
pub(super) fn space_index(order: &[String], from: &str, over: &str, after: bool) -> Option<usize> {
    let rest: Vec<&String> = order.iter().filter(|id| *id != from).collect();
    if !order.iter().any(|id| id == from) {
        return None;
    }
    let at = rest.iter().position(|id| *id == over)?;
    Some(at + usize::from(after) + 1)
}

/// Where a project dropped next to `over` goes: its group (none: the loose list) and its final position (1 = first).
/// `loose` is the manual order of every project.
pub(super) fn project_slot(groups: &[PGroup], loose: &[String], from: &str, over: &str, after: bool) -> Option<(Option<String>, usize)> {
    let (gid, list): (Option<String>, Vec<String>) = match group_of(groups, over) {
        Some(g) => (Some(g.id.clone()), g.projects.clone()),
        None => (None, loose.to_vec()),
    };
    let rest: Vec<&String> = list.iter().filter(|k| *k != from).collect();
    let at = rest.iter().position(|k| *k == over)?;
    Some((gid, at + usize::from(after) + 1))
}

/// Of `order`, the ids that hang from the same parent as `id` (the roots when it has none here): the places Alt+Up
/// and Alt+Down can take a space among. A parent never goes past its own child (the list keeps children under it).
pub(super) fn siblings(order: &[String], parents: &BTreeMap<String, String>, id: &str) -> Vec<String> {
    let here: BTreeSet<&str> = order.iter().map(String::as_str).collect();
    let parent = |x: &str| parents.get(x).filter(|p| here.contains(p.as_str()) && p.as_str() != x).cloned();
    let mine = parent(id);
    order.iter().filter(|x| parent(x) == mine).cloned().collect()
}

/// The neighbor of `key` in `list`, one step up or down.
pub(super) fn neighbor(list: &[String], key: &str, down: bool) -> Option<String> {
    let at = list.iter().position(|k| k == key)?;
    if down { list.get(at + 1) } else { at.checked_sub(1).and_then(|i| list.get(i)) }.cloned()
}

/// The row under height `y`, among `(key, top, bottom)`, and whether `y` is in its lower half.
pub(super) fn row_at(rows: &[(String, f32, f32)], y: f32) -> Option<(usize, bool)> {
    rows.iter().position(|(_, top, bottom)| y >= *top && y < *bottom).map(|i| (i, y >= (rows[i].1 + rows[i].2) / 2.0))
}

/// How far the list scrolls on one tick while a drag is at height `y` of a viewport `top..bottom`: positive moves
/// the content down (to show what is above), 0 away from the edges. It grows the closer the pointer is to the edge.
pub(super) fn edge_scroll(top: f32, bottom: f32, y: f32) -> f32 {
    const ZONE: f32 = 36.0;
    const MAX: f32 = 14.0;
    if bottom - top < ZONE * 2.0 {
        return 0.0;
    }
    if y < top + ZONE {
        (MAX * ((top + ZONE - y) / ZONE).min(1.0)).max(2.0)
    } else if y > bottom - ZONE {
        -(MAX * ((y - (bottom - ZONE)) / ZONE).min(1.0)).max(2.0)
    } else {
        0.0
    }
}

/// A press that moved this far (px) is a drag, not a click.
pub(super) fn is_drag(dx: f32, dy: f32) -> bool {
    dx * dx + dy * dy > 25.0
}

// Parents ---------------------------------------------------------------------------------

/// The saved parents: `{child space id: parent space id}`; anything else is no parent.
pub(super) fn read_parents(v: Option<&Value>) -> BTreeMap<String, String> {
    v.and_then(Value::as_object).map(|m| m.iter().filter_map(|(c, p)| Some((c.clone(), p.as_str()?.to_owned()))).filter(|(c, p)| c != p).collect()).unwrap_or_default()
}

/// `child`'s parent set (or cleared, `None`); a space can't descend from itself.
pub(super) fn set_parent(parents: &mut BTreeMap<String, String>, child: &str, parent: Option<&str>) -> Result<(), String> {
    let Some(parent) = parent else {
        parents.remove(child);
        return Ok(());
    };
    if parent == child {
        return Err("a space can't be its own parent".into());
    }
    let mut up = Some(parent);
    for _ in 0..=parents.len() {
        match up {
            Some(p) if p == child => return Err("that would make the space its own ancestor".into()),
            Some(p) => up = parents.get(p).map(String::as_str),
            None => break,
        }
    }
    parents.insert(child.to_owned(), parent.to_owned());
    Ok(())
}

/// The parents of spaces that still exist, both ends.
pub(super) fn prune_parents(parents: &BTreeMap<String, String>, existing: &BTreeSet<&str>) -> BTreeMap<String, String> {
    parents.iter().filter(|(c, p)| existing.contains(c.as_str()) && existing.contains(p.as_str())).map(|(c, p)| (c.clone(), p.clone())).collect()
}

/// The children of `id`, in `ids`' order.
pub(super) fn children_of(parents: &BTreeMap<String, String>, ids: &[String], id: &str) -> Vec<String> {
    ids.iter().filter(|c| parents.get(*c).is_some_and(|p| p == id)).cloned().collect()
}

/// How deep the list nests children (deeper ones sit at this level).
const MAX_DEPTH: usize = 4;

/// A row of a nested list: how deep it is, whether it is its parent's last child, and for each level above whether
/// that ancestor has more children below (a vertical guide continues).
#[derive(Clone, Debug)]
pub(super) struct Nested {
    pub row: Row,
    pub depth: usize,
    pub last: bool,
    pub trail: Vec<bool>,
}

/// The rows with each child right under its parent (when both are in `rows`), in `rows`' order otherwise.
pub(super) fn nest(rows: &[Row], parents: &BTreeMap<String, String>) -> Vec<Nested> {
    let here: BTreeSet<&str> = rows.iter().map(Row::id).collect();
    let has_parent_here = |r: &Row| parents.get(r.id()).is_some_and(|p| here.contains(p.as_str()) && p.as_str() != r.id());
    let mut out = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    fn walk(rows: &[Row], parents: &BTreeMap<String, String>, row: &Row, depth: usize, last: bool, trail: Vec<bool>, seen: &mut BTreeSet<String>, out: &mut Vec<Nested>) {
        if !seen.insert(row.id().to_owned()) {
            return;
        }
        out.push(Nested { row: row.clone(), depth, last, trail: trail.clone() });
        let kids: Vec<&Row> = rows.iter().filter(|k| parents.get(k.id()).is_some_and(|p| p == row.id()) && !seen.contains(k.id())).collect();
        let n = kids.len();
        // A level's vertical guide runs while that ancestor has more children below; the roots have no column.
        let mut next = trail;
        if depth >= 1 {
            next.push(!last);
        }
        next.truncate(MAX_DEPTH - 1);
        for (i, kid) in kids.into_iter().enumerate() {
            walk(rows, parents, kid, (depth + 1).min(MAX_DEPTH), i + 1 == n, next.clone(), seen, out);
        }
    }
    for r in rows.iter().filter(|r| !has_parent_here(r)) {
        walk(rows, parents, r, 0, true, Vec::new(), &mut seen, &mut out);
    }
    // A cycle left some rows out of every walk: they show flat rather than vanish.
    for r in rows {
        if !seen.contains(r.id()) {
            out.push(Nested { row: r.clone(), depth: 0, last: true, trail: Vec::new() });
        }
    }
    out
}

/// The guide glyphs at the left of a nested row: one cell per level above it (a vertical line while that ancestor
/// has more children below), then the tee or elbow that joins it to its parent. Empty for a root.
pub(super) fn guides(n: &Nested) -> Vec<&'static str> {
    if n.depth == 0 {
        return Vec::new();
    }
    let mut cells: Vec<&'static str> = n.trail.iter().map(|more| if *more { "│" } else { " " }).collect();
    cells.push(if n.last { "└" } else { "├" });
    cells
}

/// How a guide cell is drawn (as lines, not font glyphs, so every row's stroke is the same width and they join):
/// whether a vertical stroke runs the whole cell or stops at the row's middle, and whether an elbow goes right.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Stroke {
    pub full: bool,
    pub half: bool,
    pub elbow: bool,
}

pub(super) fn stroke(glyph: &str) -> Stroke {
    match glyph {
        "│" => Stroke { full: true, half: false, elbow: false },
        "├" => Stroke { full: true, half: false, elbow: true },
        "└" => Stroke { full: false, half: true, elbow: true },
        _ => Stroke { full: false, half: false, elbow: false },
    }
}

// The commands -------------------------------------------------------------------------------

fn key_arg(a: &Value) -> Result<&str, String> {
    a.get("key").and_then(Value::as_str).filter(|k| !k.is_empty()).ok_or_else(|| "key is the project's key".to_owned())
}

fn id_arg<'a>(a: &'a Value, what: &str) -> Result<&'a str, String> {
    a.get("id").and_then(Value::as_str).filter(|k| !k.is_empty()).ok_or_else(|| format!("id is the {what}'s id"))
}

fn saved_groups() -> Vec<PGroup> {
    read_groups(mods::shared("sidebar.groups").as_ref())
}

fn save_groups(groups: &[PGroup]) {
    mods::set_shared("sidebar.groups".into(), groups_value(groups));
}

/// The keys of every project the list knows, in the manual order.
fn all_project_keys(d: &Data) -> Vec<String> {
    let rows = with(|s| ordered_all(&d.spaces, &d.panes, &s.git));
    let present: Vec<String> = group_projects(&rows).into_iter().map(|g| g.key).collect();
    manual_order(&shared_list("sidebar.project_keys"), &present)
}

/// A group's name now (the rename prompt starts with it).
pub(crate) fn group_name(id: &str) -> Option<String> {
    saved_groups().into_iter().find(|g| g.id == id).map(|g| g.name)
}

/// The name a new group starts with: its project's.
pub(crate) fn default_group_name(key: &str) -> String {
    let d = data();
    let rows = with(|s| ordered_all(&d.spaces, &d.panes, &s.git));
    group_projects(&rows).into_iter().find(|g| g.key == key).map(|g| g.label).unwrap_or_default()
}

impl AppWindow {
    fn known_project(&self, key: &str, d: &Data) -> Result<(), String> {
        let rows = with(|s| ordered_all(&d.spaces, &d.panes, &s.git));
        group_projects(&rows).iter().any(|g| g.key == key).then_some(()).ok_or_else(|| "no such project".to_owned())
    }

    /// `group.*`, `project.move`, `space.parent` and `space.worktree`.
    pub(super) fn group_command(&mut self, name: &str, a: &Value, person: bool, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        let d = data();
        let index = |a: &Value| a.get("index").map(|v| v.as_u64().filter(|i| *i >= 1).map(|i| i as usize).ok_or("index is a position, 1 = first")).transpose();
        let mut groups = saved_groups();
        match name {
            "group.new" => {
                let key = key_arg(a)?;
                self.known_project(key, &d)?;
                let typed = a.get("name").and_then(Value::as_str).unwrap_or_default().trim();
                let name = if typed.is_empty() { default_group_name(key) } else { typed.to_owned() };
                let made = new_group(&mut groups, &name, key, a.get("color").and_then(Value::as_str))?;
                save_groups(&groups);
                self.answer(Ok(json!({ "group": made })));
            }
            "group.rename" => {
                let id = id_arg(a, "group")?;
                rename_group(&mut groups, id, a.get("name").and_then(Value::as_str).ok_or("name is the group's new name")?)?;
                save_groups(&groups);
            }
            "group.color" => {
                recolor_group(&mut groups, id_arg(a, "group")?, a.get("color").and_then(Value::as_str).ok_or("color is one of the palette's")?)?;
                save_groups(&groups);
            }
            "group.delete" => {
                let id = id_arg(a, "group")?;
                let g = groups.iter().find(|g| g.id == id).cloned().ok_or_else(|| format!("no group {id:?}"))?;
                let ask = (i18n::tf("groups.delete_title", &[("name", &g.name)]), i18n::tf("groups.delete_detail", &[("n", &g.projects.len().to_string())]), Some(i18n::t("groups.delete")));
                if !self.consent(a, name, ask, person, window, cx)? {
                    return Ok(());
                }
                delete_group(&mut groups, id)?;
                save_groups(&groups);
                let folded: Vec<String> = shared_list("space.folded").into_iter().filter(|k| *k != format!("group:{id}")).collect();
                mods::set_shared("space.folded".into(), json!(folded));
            }
            "group.move" => {
                let key = key_arg(a)?;
                self.known_project(key, &d)?;
                let to = a.get("group").and_then(Value::as_str).filter(|g| !g.is_empty());
                place_project(&mut groups, key, to, index(a)?)?;
                save_groups(&groups);
            }
            "group.reorder" => {
                let at = index(a)?.ok_or("index is a position, 1 = first")?;
                reorder_group(&mut groups, id_arg(a, "group")?, at)?;
                save_groups(&groups);
            }
            "group.refresh" => {
                let id = id_arg(a, "group")?;
                let g = groups.iter().find(|g| g.id == id).ok_or_else(|| format!("no group {id:?}"))?;
                let rows = with(|s| ordered_all(&d.spaces, &d.panes, &s.git));
                let mine: Vec<&Row> = rows.iter().filter(|r| g.projects.contains(&project_key(r))).collect();
                let places: BTreeSet<String> = mine.iter().map(|r| where_key(&r.w, &d.panes)).collect();
                with(|s| {
                    for r in &mine {
                        s.icons.remove(&project_key(r));
                    }
                });
                // The group's count turns into a spinner for a moment: the person sees that it read again.
                let until = Instant::now() + Duration::from_millis(600);
                with(|s| s.refresh_shown = Some(until));
                cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(Duration::from_millis(650)).await;
                    let _ = this.update(cx, |_, cx| cx.notify());
                })
                .detach();
                self.refresh_git(places, cx);
            }
            "project.move" => {
                let key = key_arg(a)?;
                self.known_project(key, &d)?;
                let to = a.get("group").and_then(Value::as_str).filter(|g| !g.is_empty());
                let at = index(a)?;
                place_project(&mut groups, key, to, at)?;
                save_groups(&groups);
                let mut loose = all_project_keys(&d);
                loose.retain(|k| k != key);
                let pos = match (to, at) {
                    (None, Some(i)) => i.saturating_sub(1).min(loose.len()),
                    _ => loose.len(),
                };
                if to.is_none() {
                    loose.insert(pos, key.to_owned());
                } else {
                    loose.push(key.to_owned());
                }
                mods::set_shared("sidebar.project_keys".into(), json!(loose));
                let old = load_view();
                let mut new = old.clone();
                new.project_order = ProjectOrder::Manual;
                save_view(&old, &new);
            }
            "space.parent" => {
                let id = id_arg(a, "space")?;
                if !d.spaces.iter().any(|w| text(w, "id") == id) {
                    return Err("no such space".into());
                }
                let parent = a.get("parent").and_then(Value::as_str).filter(|p| !p.is_empty());
                if let Some(p) = parent
                    && !d.spaces.iter().any(|w| text(w, "id") == p)
                {
                    return Err("no such parent space".into());
                }
                let mut parents = read_parents(mods::shared("space.parents").as_ref());
                set_parent(&mut parents, id, parent)?;
                mods::set_shared("space.parents".into(), json!(parents));
            }
            "space.worktree" => {
                let id = id_arg(a, "space")?;
                let w = d.spaces.iter().find(|w| text(w, "id") == id).ok_or("no such space")?;
                let ctx = with(|s| s.git.get(&where_key(w, &d.panes)).cloned().flatten());
                if ctx.is_none() {
                    return Err("only a git space can have worktrees".into());
                }
                self.open_worktree(json!({ "space": id, "folder": place(w, &d.panes), "machine": opt_text(w, "machine"), "action": "new", "parent": true }), window, cx);
            }
            _ => return Err(format!("unknown command {name:?}")),
        }
        let _ = view::Act::Reveal;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // `super::*` brings gpui's own `test` macro: the plain one is the std attribute.
    use core::prelude::v1::test;

    fn g(id: &str, name: &str, projects: &[&str]) -> PGroup {
        PGroup { id: id.into(), name: name.into(), color: "ansi6".into(), projects: projects.iter().map(|s| (*s).to_owned()).collect() }
    }
    fn s(list: &[&str]) -> Vec<String> {
        list.iter().map(|x| (*x).to_owned()).collect()
    }
    fn row(id: &str, i: usize) -> Row {
        Row { w: json!({ "id": id, "name": id, "tabs": [] }), i, g: None }
    }
    fn ids(n: &[Nested]) -> Vec<(&str, usize)> {
        n.iter().map(|x| (x.row.id(), x.depth)).collect()
    }

    #[test]
    fn saved_groups_read_back_and_bad_ones_are_left_out() {
        let saved = json!([{ "id": "g1", "name": " Work ", "color": "ansi3", "projects": ["a", "b"] }, { "id": "g2", "name": "Home", "projects": ["b", "c"] }, { "id": "g1", "name": "dup" }, { "name": "no id" }, 7, { "id": "g3", "projects": 4 }]);
        let read = read_groups(Some(&saved));
        assert_eq!(read.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(), ["g1", "g2"]);
        assert_eq!(read[0].name, "Work");
        // A project belongs to one group; an unknown color is the first of the palette.
        assert_eq!(read[1].projects, ["c"]);
        assert_eq!(read[1].color, "ansi6");
        assert_eq!(read_groups(Some(&groups_value(&read))), read);
        for bad in [json!(null), json!("x"), json!({})] {
            assert!(read_groups(Some(&bad)).is_empty());
        }
        assert!(read_groups(None).is_empty());
    }

    #[test]
    fn a_group_is_made_from_a_project_renamed_recolored_and_deleted() {
        let mut groups = vec![g("g1", "Work", &["a", "b"])];
        let id = new_group(&mut groups, "  Home ", "b", None).unwrap();
        assert_eq!(id, "g2");
        // The project left its old group; the new hue is the least used one.
        assert_eq!((groups[0].projects.clone(), groups[1].projects.clone()), (s(&["a"]), s(&["b"])));
        assert_eq!(groups[1].color, "ansi3");
        assert!(new_group(&mut groups, "work", "a", None).unwrap_err().contains("already"));
        assert!(new_group(&mut groups, "  ", "a", None).is_err());
        assert!(new_group(&mut groups, "X", "a", Some("red")).is_err());
        rename_group(&mut groups, "g2", "Casa").unwrap();
        assert!(rename_group(&mut groups, "g2", "Work").is_err());
        // Its own name again is fine.
        rename_group(&mut groups, "g2", "casa").unwrap();
        assert!(rename_group(&mut groups, "nope", "Y").is_err());
        recolor_group(&mut groups, "g2", "ansi1").unwrap();
        assert_eq!(groups[1].color, "ansi1");
        assert!(recolor_group(&mut groups, "g2", "pink").is_err());
        let gone = delete_group(&mut groups, "g1").unwrap();
        assert_eq!(gone.projects, ["a"]);
        assert_eq!(groups.len(), 1);
        assert!(delete_group(&mut groups, "g1").is_err());
        // Ids do not repeat while a later one lives.
        assert_eq!(new_group(&mut groups, "Next", "z", None).unwrap(), "g3");
    }

    #[test]
    fn projects_move_between_groups_and_out_of_them() {
        let mut groups = vec![g("g1", "A", &["a", "b"]), g("g2", "B", &["c"])];
        place_project(&mut groups, "b", Some("g2"), Some(1)).unwrap();
        assert_eq!((groups[0].projects.clone(), groups[1].projects.clone()), (s(&["a"]), s(&["b", "c"])));
        place_project(&mut groups, "a", Some("g2"), None).unwrap();
        assert_eq!(groups[1].projects, ["b", "c", "a"]);
        place_project(&mut groups, "c", None, None).unwrap();
        assert!(group_of(&groups, "c").is_none());
        assert_eq!(group_of(&groups, "a").map(|x| x.id.as_str()), Some("g2"));
        assert!(place_project(&mut groups, "a", Some("nope"), None).is_err());
        // The failed move left it where it was.
        assert_eq!(group_of(&groups, "a").map(|x| x.id.as_str()), Some("g2"));
        // Position past the end is the end.
        place_project(&mut groups, "c", Some("g1"), Some(9)).unwrap();
        assert_eq!(groups[0].projects, ["c"]);
    }

    #[test]
    fn groups_reorder_by_final_position() {
        let mut groups = vec![g("g1", "A", &[]), g("g2", "B", &[]), g("g3", "C", &[])];
        reorder_group(&mut groups, "g3", 1).unwrap();
        assert_eq!(groups.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(), ["g3", "g1", "g2"]);
        reorder_group(&mut groups, "g3", 99).unwrap();
        assert_eq!(groups.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(), ["g1", "g2", "g3"]);
        assert!(reorder_group(&mut groups, "zz", 1).is_err());
    }

    #[test]
    fn the_manual_order_keeps_saved_keys_then_adds_new_ones() {
        assert_eq!(manual_order(&s(&["c", "gone", "a"]), &s(&["a", "b", "c"])), ["c", "a", "b"]);
        assert_eq!(manual_order(&[], &s(&["a", "b"])), ["a", "b"]);
        assert!(manual_order(&s(&["a"]), &[]).is_empty());
    }

    fn proj(key: &str) -> Group {
        Group { key: key.into(), label: key.into(), rows: vec![row(&format!("s-{key}"), 0)] }
    }

    #[test]
    fn the_list_shows_each_group_with_its_projects_and_then_the_rest() {
        let pgs = vec![g("g1", "A", &["c", "a", "missing"]), g("g2", "Empty", &[])];
        let (grouped, loose) = layout(&pgs, vec![proj("a"), proj("b"), proj("c")]);
        assert_eq!(grouped[0].1.iter().map(|x| x.key.as_str()).collect::<Vec<_>>(), ["c", "a"]);
        assert!(grouped[1].1.is_empty());
        assert_eq!(loose.iter().map(|x| x.key.as_str()).collect::<Vec<_>>(), ["b"]);
        // No groups: the list is as it was.
        let (none, all) = layout(&[], vec![proj("a"), proj("b")]);
        assert!(none.is_empty());
        assert_eq!(all.len(), 2);
    }

    fn rel<'a>(same: &'a dyn Fn(&str, &str) -> bool, status: bool, project: bool) -> Rel<'a> {
        Rel { same_project: same, by_status: status, by_project: project }
    }

    #[test]
    fn a_drop_goes_only_where_the_list_takes_it() {
        let same = |a: &str, b: &str| a.chars().next() == b.chars().next();
        let all = rel(&same, true, true);
        let sp = Dragged::Space("a1".into());
        assert_eq!(resolve(&sp, &What::Space("a2".into()), true, &all), Some(Drop::Space { over: "a2".into(), after: true }));
        // Another project's space, or itself: no.
        assert_eq!(resolve(&sp, &What::Space("b1".into()), true, &all), None);
        assert_eq!(resolve(&sp, &What::Space("a1".into()), false, &all), None);
        assert_eq!(resolve(&sp, &What::Section("status:done".into()), false, &all), Some(Drop::Status("done".into())));
        assert_eq!(resolve(&sp, &What::Section("pinned".into()), false, &all), None);
        assert_eq!(resolve(&sp, &What::Section("status:done".into()), false, &rel(&same, false, true)), None);
        let pr = Dragged::Project("p".into());
        assert_eq!(resolve(&pr, &What::Project("q".into()), false, &all), Some(Drop::Project { over: "q".into(), after: false }));
        assert_eq!(resolve(&pr, &What::Group("g1".into()), false, &all), Some(Drop::IntoGroup("g1".into())));
        assert_eq!(resolve(&pr, &What::Project("q".into()), false, &rel(&same, true, false)), None);
        assert_eq!(resolve(&pr, &What::Space("a1".into()), false, &all), None);
        let gr = Dragged::Group("g1".into());
        assert_eq!(resolve(&gr, &What::Group("g2".into()), true, &all), Some(Drop::Group { over: "g2".into(), after: true }));
        assert_eq!(resolve(&gr, &What::Group("g1".into()), true, &all), None);
        assert_eq!(resolve(&gr, &What::Project("q".into()), true, &all), None);
        assert_eq!(gr.what(), What::Group("g1".into()));
    }

    #[test]
    fn a_space_lands_next_to_the_one_it_was_dropped_on() {
        let order = s(&["a", "b", "c", "d"]);
        // Down: after c it is third; before c, also third (it left its place first).
        assert_eq!(space_index(&order, "a", "c", true), Some(3));
        assert_eq!(space_index(&order, "a", "c", false), Some(2));
        assert_eq!(space_index(&order, "d", "a", false), Some(1));
        assert_eq!(space_index(&order, "d", "b", true), Some(3));
        assert_eq!(space_index(&order, "x", "b", true), None);
        assert_eq!(space_index(&order, "a", "x", true), None);
    }

    #[test]
    fn a_project_lands_in_the_group_or_the_loose_list_of_the_one_it_was_dropped_on() {
        let groups = vec![g("g1", "A", &["a", "b"]), g("g2", "B", &["c"])];
        let loose = s(&["d", "e", "a", "b", "c"]);
        assert_eq!(project_slot(&groups, &loose, "c", "a", false), Some((Some("g1".into()), 1)));
        assert_eq!(project_slot(&groups, &loose, "c", "a", true), Some((Some("g1".into()), 2)));
        // Inside its own group, after the one that follows it.
        assert_eq!(project_slot(&groups, &loose, "a", "b", true), Some((Some("g1".into()), 2)));
        // Onto a loose project: out of any group, into the loose list.
        assert_eq!(project_slot(&groups, &loose, "a", "e", false), Some((None, 2)));
        assert_eq!(project_slot(&groups, &loose, "d", "e", true), Some((None, 2)));
        assert_eq!(project_slot(&groups, &loose, "a", "zz", true), None);
    }

    #[test]
    fn keys_step_to_the_neighbor_and_stop_at_the_ends() {
        let list = s(&["a", "b", "c"]);
        assert_eq!(neighbor(&list, "b", true).as_deref(), Some("c"));
        assert_eq!(neighbor(&list, "b", false).as_deref(), Some("a"));
        assert_eq!(neighbor(&list, "a", false), None);
        assert_eq!(neighbor(&list, "c", true), None);
        assert_eq!(neighbor(&list, "x", true), None);
    }

    #[test]
    fn the_pointer_picks_a_row_and_its_half() {
        let rows = vec![("a".to_owned(), 0.0, 32.0), ("b".to_owned(), 32.0, 74.0)];
        assert_eq!(row_at(&rows, 10.0), Some((0, false)));
        assert_eq!(row_at(&rows, 20.0), Some((0, true)));
        assert_eq!(row_at(&rows, 40.0), Some((1, false)));
        assert_eq!(row_at(&rows, 74.0), None);
        assert_eq!(row_at(&rows, -1.0), None);
        assert_eq!(row_at(&[], 5.0), None);
    }

    #[test]
    fn the_list_scrolls_near_its_edges_only_and_faster_closer_to_them() {
        assert_eq!(edge_scroll(100.0, 500.0, 300.0), 0.0);
        assert!(edge_scroll(100.0, 500.0, 110.0) > edge_scroll(100.0, 500.0, 130.0));
        assert!(edge_scroll(100.0, 500.0, 110.0) > 0.0, "near the top shows what is above");
        assert!(edge_scroll(100.0, 500.0, 490.0) < 0.0, "near the bottom shows what is below");
        assert_eq!(edge_scroll(100.0, 500.0, 40.0), 14.0);
        // A list too short to have a middle never scrolls.
        assert_eq!(edge_scroll(0.0, 60.0, 5.0), 0.0);
        assert!(!is_drag(2.0, 2.0) && is_drag(6.0, 0.0));
    }

    #[test]
    fn parents_read_back_and_never_make_a_loop() {
        assert_eq!(read_parents(Some(&json!({ "b": "a", "c": 4, "d": "d" }))), BTreeMap::from([("b".to_owned(), "a".to_owned())]));
        assert!(read_parents(Some(&json!(["x"]))).is_empty());
        let mut p = BTreeMap::new();
        set_parent(&mut p, "b", Some("a")).unwrap();
        set_parent(&mut p, "c", Some("b")).unwrap();
        assert!(set_parent(&mut p, "a", Some("c")).is_err(), "a would descend from itself");
        assert!(set_parent(&mut p, "a", Some("a")).is_err());
        set_parent(&mut p, "c", Some("a")).unwrap();
        assert_eq!(p["c"], "a");
        set_parent(&mut p, "c", None).unwrap();
        assert!(!p.contains_key("c"));
        // Closed spaces take their links with them.
        let live: BTreeSet<&str> = ["a", "b"].into_iter().collect();
        assert_eq!(prune_parents(&BTreeMap::from([("b".to_owned(), "a".to_owned()), ("c".to_owned(), "a".to_owned()), ("a".to_owned(), "x".to_owned())]), &live).len(), 1);
        assert_eq!(children_of(&BTreeMap::from([("b".to_owned(), "a".to_owned()), ("c".to_owned(), "a".to_owned())]), &s(&["c", "a", "b"]), "a"), ["c", "b"]);
    }

    #[test]
    fn alt_arrows_move_a_space_among_its_siblings_never_past_its_own_child() {
        let ids = |l: &[&str]| l.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let parents: BTreeMap<String, String> = [("kid".to_owned(), "a".to_owned()), ("kid2".to_owned(), "a".to_owned())].into();
        let order = ids(&["a", "kid", "b", "kid2"]);
        assert_eq!(siblings(&order, &parents, "a"), ids(&["a", "b"]));
        assert_eq!(siblings(&order, &parents, "b"), ids(&["a", "b"]));
        assert_eq!(siblings(&order, &parents, "kid"), ids(&["kid", "kid2"]));
        // A parent that is not in the list (another project) makes its children roots.
        assert_eq!(siblings(&ids(&["kid", "b"]), &parents, "kid"), ids(&["kid", "b"]));
    }

    #[test]
    fn guide_glyphs_become_strokes_that_join_up() {
        assert_eq!(stroke("│"), Stroke { full: true, half: false, elbow: false });
        assert_eq!(stroke("├"), Stroke { full: true, half: false, elbow: true });
        assert_eq!(stroke("└"), Stroke { full: false, half: true, elbow: true });
        assert_eq!(stroke(" "), Stroke { full: false, half: false, elbow: false });
    }

    #[test]
    fn children_sit_under_their_parent_with_guides() {
        let rows: Vec<Row> = ["main", "feat", "x", "feat-a", "feat-b", "feat-a-1"].iter().enumerate().map(|(i, id)| row(id, i)).collect();
        let parents = BTreeMap::from([("feat-a".to_owned(), "feat".to_owned()), ("feat-b".to_owned(), "feat".to_owned()), ("feat-a-1".to_owned(), "feat-a".to_owned())]);
        let nested = nest(&rows, &parents);
        assert_eq!(ids(&nested), [("main", 0), ("feat", 0), ("feat-a", 1), ("feat-a-1", 2), ("feat-b", 1), ("x", 0)]);
        let glyphs: Vec<String> = nested.iter().map(|n| guides(n).concat()).collect();
        assert_eq!(glyphs, ["", "", "├", "│└", "└", ""]);
        // A parent that is not in the list leaves its child at the top.
        let orphan = nest(&rows[..1], &BTreeMap::from([("main".to_owned(), "zz".to_owned())]));
        assert_eq!(ids(&orphan), [("main", 0)]);
        // No parents: the same rows in the same order.
        assert_eq!(ids(&nest(&rows, &BTreeMap::new())).iter().map(|x| x.0).collect::<Vec<_>>(), ["main", "feat", "x", "feat-a", "feat-b", "feat-a-1"]);
    }

    #[test]
    fn a_saved_loop_shows_every_space_anyway() {
        let rows = vec![row("a", 0), row("b", 1)];
        let parents = BTreeMap::from([("a".to_owned(), "b".to_owned()), ("b".to_owned(), "a".to_owned())]);
        let nested = nest(&rows, &parents);
        assert_eq!(nested.len(), 2);
    }

    #[test]
    fn every_text_of_groups_and_lineage_exists_in_both_locales() {
        let mut keys = vec![
            "groups.new_from_project", "groups.move_to", "groups.remove_from", "groups.delete", "groups.refresh", "groups.color", "groups.actions", "groups.new_title", "groups.rename_title", "groups.delete_title", "groups.delete_detail",
            "groups.empty", "groups.untitled", "groups.refreshing", "lineage.title", "lineage.open_parent", "lineage.remove_parent", "lineage.open_child", "lineage.new_here", "worktree.from_here", "worktree.from_here_of", "groups.count_tip", "spaces.drag_to_move", "spaces.machine_count",
        ];
        let colors: Vec<String> = PALETTE.iter().map(|c| format!("groups.color_{c}")).collect();
        keys.extend(colors.iter().map(String::as_str));
        for lang in ["en", "es"] {
            let file = format!("{}/defaults/locales/{lang}.json", env!("CARGO_MANIFEST_DIR"));
            let table: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
            for k in &keys {
                assert!(table[*k].as_str().is_some_and(|s| !s.is_empty()), "{lang}:{k}");
            }
        }
    }
}
