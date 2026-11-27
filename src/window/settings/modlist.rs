//! Settings > Mods: what is installed (cards by group, turn off, delete,
//! duplicate, update), installing from a folder, a git URL or GitHub, with a
//! review of what would be written, and the gallery of mods, themes and
//! layouts people shared. The disk and network work is in `modstore`.

use super::modstore::{self as store_, Caps, Entry, Fail, Installed, Item, ItemKind, Origin, Source, SourceKind};
use super::*;
use controls::{button, primary};
use gpui_base::input::{InputEvent, InputState};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Installed,
    Explore,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Filter {
    All,
    Active,
    Disabled,
    Updates,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sort {
    Stars,
    Recent,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Img {
    Loading,
    Ok,
    Failed,
}

/// What an install would write, shown before anything is written.
pub struct Review {
    src: Source,
    staging: Option<PathBuf>,
    commit: Option<String>,
    items: Vec<Item>,
    replace: Option<String>,
}

pub struct State {
    pub list: Option<Vec<Installed>>,
    loading: bool,
    confirm: Option<String>,
    busy: bool,
    detail: Option<Entry>,
    tab: Tab,
    install_error: Option<String>,
    review: Option<Review>,
    /// mod name -> newest commit of its source once asked ("" = couldn't tell)
    latest: HashMap<String, String>,
    kind: &'static str,
    found: HashMap<&'static str, Result<Vec<Entry>, Fail>>,
    found_loading: HashSet<&'static str>,
    previews: HashMap<String, Img>,
    compat: HashMap<String, Option<bool>>,
    sort: Sort,
    filter: Filter,
    auto_checked: bool,
    queue: Vec<String>,
    query: String,
    iq: String,
    source: String,
    source_input: Entity<InputState>,
    query_input: Entity<InputState>,
    iq_input: Entity<InputState>,
}

pub const KINDS: [(&str, &str); 3] = [("herdr-layout", "settings.mods_kind_layouts"), ("herdr-mod", "settings.mods_kind_mods"), ("herdr-theme", "settings.mods_kind_themes")];

fn kind_key(kind: &str) -> &'static str {
    KINDS.iter().find(|(k, _)| *k == kind).map_or("herdr-mod", |(k, _)| *k)
}

impl State {
    pub fn new(window: &mut Window, cx: &mut Context<SettingsView>) -> Self {
        let mk = |placeholder: String, window: &mut Window, cx: &mut Context<SettingsView>| {
            let input = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
            input.update(cx, |s, _| s.set_editor_style(controls::input_style()));
            input
        };
        let source_input = mk(i18n::t("settings.mods_source_placeholder"), window, cx);
        let query_input = mk(i18n::t("settings.mods_search"), window, cx);
        let iq_input = mk(i18n::t("settings.mods_search_installed"), window, cx);
        cx.subscribe_in(&source_input, window, |this, input, ev: &InputEvent, w, cx| match ev {
            InputEvent::Change => this.mods.source = input.read(cx).value().trim().to_owned(),
            InputEvent::PressEnter { .. } => {
                let source = this.mods.source.clone();
                this.start_install(source, None, w, cx);
            }
            _ => {}
        })
        .detach();
        cx.subscribe_in(&query_input, window, |this, input, ev: &InputEvent, _, cx| {
            if matches!(ev, InputEvent::Change) {
                this.mods.query = input.read(cx).value().trim().to_lowercase();
                cx.notify();
            }
        })
        .detach();
        cx.subscribe_in(&iq_input, window, |this, input, ev: &InputEvent, _, cx| {
            if matches!(ev, InputEvent::Change) {
                this.mods.iq = input.read(cx).value().trim().to_lowercase();
                cx.notify();
            }
        })
        .detach();
        Self {
            list: None,
            loading: false,
            confirm: None,
            busy: false,
            detail: None,
            tab: Tab::Installed,
            install_error: None,
            review: None,
            latest: HashMap::new(),
            kind: "herdr-mod",
            found: HashMap::new(),
            found_loading: HashSet::new(),
            previews: HashMap::new(),
            compat: HashMap::new(),
            sort: Sort::Stars,
            filter: Filter::All,
            auto_checked: false,
            queue: Vec::new(),
            query: String::new(),
            iq: String::new(),
            source: String::new(),
            source_input,
            query_input,
            iq_input,
        }
    }

    /// Whether a newer commit of `m`'s source is known.
    pub fn has_update(&self, m: &Installed) -> bool {
        update_available(m, &self.latest)
    }

    /// The installed mods that pass the filter and the search.
    pub fn visible<'a>(&self, list: &'a [Installed]) -> Vec<&'a Installed> {
        visible_in(list, self.filter, &self.iq, &self.latest)
    }
}

/// Whether a newer commit than the installed one is known (`latest`: "" means it couldn't tell).
pub fn update_available(m: &Installed, latest: &HashMap<String, String>) -> bool {
    match (m.commit(), latest.get(&m.name)) {
        (Some(now), Some(l)) => !l.is_empty() && l != now,
        _ => false,
    }
}

pub fn visible_in<'a>(list: &'a [Installed], filter: Filter, iq: &str, latest: &HashMap<String, String>) -> Vec<&'a Installed> {
    list.iter()
        .filter(|m| match filter {
            Filter::Active => !m.disabled,
            Filter::Disabled => m.disabled,
            Filter::Updates => update_available(m, latest),
            Filter::All => true,
        })
        .filter(|m| {
            iq.is_empty() || {
                let hay = format!("{} {} {}", m.name, m.meta["name"].as_str().unwrap_or_default(), text(m.meta["description"].as_str().unwrap_or_default())).to_lowercase();
                hay.contains(iq)
            }
        })
        .collect()
}

/// A locale key shows translated; anything else is literal text.
pub fn text(k: &str) -> String {
    let t = i18n::t(k);
    if t != k { t } else { k.to_owned() }
}

fn eid(s: String) -> SharedString {
    SharedString::from(s)
}

fn z(v: f32) -> Pixels {
    settings::z(v)
}

fn badge(label: impl Into<SharedString>, tint: bool) -> Div {
    div()
        .px(z(6.0))
        .py(z(1.0))
        .rounded(z(5.0))
        .bg(if tint { c("accent") } else { c("hover") })
        .text_size(z(11.0))
        .text_color(if tint { on_accent() } else { tone("muted") })
        .child(label.into())
}

/// A button that can't be pressed now.
fn inert(label: impl Into<SharedString>) -> Div {
    div().h(z(32.0)).px(z(12.0)).flex().items_center().rounded(z(7.0)).border_1().border_color(c("border")).opacity(0.5).text_size(z(13.0)).text_color(tone("dim")).child(label.into())
}

fn caps_badges(caps: &Caps) -> Vec<Div> {
    [(caps.programs, "settings.mods_cap_programs"), (caps.network, "settings.mods_cap_network"), (caps.files, "settings.mods_cap_files")]
        .into_iter()
        .filter(|(on, _)| *on)
        .map(|(_, k)| badge(i18n::t(k), false))
        .collect()
}

fn open_path(p: &std::path::Path) {
    crate::links::open(&crate::links::Target::File(p.to_owned()));
}

impl SettingsView {
    /// Runs `work` on a background thread, then `then` on this view.
    fn bg<T: Send + 'static>(&self, window: &mut Window, cx: &mut Context<Self>, work: impl FnOnce() -> T + Send + 'static, then: impl FnOnce(&mut Self, T, &mut Window, &mut Context<Self>) + 'static) {
        cx.spawn_in(window, async move |this, cx| {
            let out = cx.background_executor().spawn(async move { work() }).await;
            let _ = this.update_in(cx, |this, window, cx| {
                then(this, out, window, cx);
                cx.notify();
            });
        })
        .detach();
    }

    fn toast_fail(&self, f: &Fail, window: &mut Window, cx: &mut Context<Self>) {
        let mut n = json!({ "level": "error", "title": i18n::t(&f.key) });
        if let Some(b) = &f.body {
            n["body"] = json!(b);
        }
        self.notify(n, window, cx);
    }

    /// Entering the page: read the folders again and ask the sources once more.
    pub(super) fn visit_mods(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.mods.loading = false;
        self.mods.auto_checked = false;
        self.reload_mods(window, cx);
    }

    /// Reads the folders again.
    pub(super) fn reload_mods(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.mods.loading {
            return;
        }
        self.mods.loading = true;
        self.bg(window, cx, || store_::list_installed(&mods::app_dir()), |this, list, _, _| {
            this.mods.loading = false;
            this.mods.list = Some(list);
        });
    }

    /// Reads a source (clone or folder) and shows what it holds, before anything is written.
    pub(super) fn start_install(&mut self, input: String, replace: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        if self.mods.busy {
            return;
        }
        self.mods.busy = true;
        self.mods.install_error = None;
        let home = mods::app_dir();
        self.bg(
            window,
            cx,
            move || -> Result<Review, Fail> {
                let src = store_::parse_source(&input)?;
                let got = store_::fetch_source(&home, &src)?;
                let items = match store_::scan_source(&got.dir, &src.name) {
                    Ok(items) => items,
                    Err(e) => {
                        if let Some(s) = &got.staging {
                            let _ = std::fs::remove_dir_all(s);
                        }
                        return Err(e);
                    }
                };
                let mut items = store_::with_existing(&home, items, replace.as_deref());
                for i in &mut items {
                    if i.kind == ItemKind::Mod {
                        i.caps = store_::scan_capabilities(&i.path);
                    }
                }
                Ok(Review { src, staging: got.staging, commit: got.commit, items, replace })
            },
            |this, result, w, cx| {
                this.mods.busy = false;
                match result {
                    Ok(review) => this.mods.review = Some(review),
                    Err(f) => {
                        this.mods.install_error = Some(f.text());
                        this.toast_fail(&f, w, cx);
                    }
                }
            },
        );
    }

    fn drop_staging(review: &Review) {
        if let Some(s) = &review.staging {
            let _ = std::fs::remove_dir_all(s);
        }
    }

    /// Reviews each mod with a newer commit, one after another: each still asks before it writes anything.
    fn update_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let list = self.mods.list.clone().unwrap_or_default();
        self.mods.queue = list.iter().filter(|m| self.mods.has_update(m)).map(|m| m.name.clone()).collect();
        self.next_queued(window, cx);
    }

    fn next_queued(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        while !self.mods.queue.is_empty() {
            let name = self.mods.queue.remove(0);
            let Some(m) = self.mods.list.as_ref().and_then(|l| l.iter().find(|m| m.name == name)).cloned() else { continue };
            if let Some(input) = m.source_input() {
                self.start_install(input, Some(m.name), window, cx);
                return;
            }
        }
    }

    fn cancel_review(&mut self) {
        self.mods.queue.clear();
        if let Some(r) = self.mods.review.take() {
            std::thread::spawn(move || Self::drop_staging(&r));
        }
    }

    fn confirm_install(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.mods.busy {
            return;
        }
        let Some(r) = self.mods.review.take() else { return };
        let chosen: Vec<Item> = r.items.iter().filter(|i| i.checked && !i.exists).cloned().collect();
        if chosen.is_empty() {
            self.mods.review = Some(r);
            return;
        }
        self.mods.busy = true;
        let source = match r.src.kind {
            SourceKind::Github => r.src.label.clone(),
            SourceKind::Git => r.src.url.clone(),
            SourceKind::Folder => r.src.path.clone(),
        };
        let origin = Origin { source, git_ref: r.src.git_ref.clone(), commit: r.commit.clone() };
        let replace = r.replace.clone();
        let home = mods::app_dir();
        self.bg(
            window,
            cx,
            move || {
                let done = store_::install_items(&home, &chosen, &origin, replace.as_deref());
                Self::drop_staging(&r);
                done
            },
            |this, result, w, cx| {
                this.mods.busy = false;
                match result {
                    Ok(count) => {
                        this.mods.detail = None;
                        this.mods.tab = Tab::Installed;
                        this.notify(json!({ "title": i18n::tf("settings.mods_installed_n", &[("count", &count.to_string())]) }), w, cx);
                        this.mods.source.clear();
                        this.mods.source_input.update(cx, |s, cx| s.set_value("", w, cx));
                        this.mods.loading = false;
                        this.reload_mods(w, cx);
                        this.next_queued(w, cx);
                    }
                    Err(f) => this.toast_fail(&f, w, cx),
                }
            },
        );
    }

    /// Runs a change on the folders, then reads them again; a failure is a toast.
    fn change_mod(&mut self, window: &mut Window, cx: &mut Context<Self>, job: impl FnOnce() -> Result<(), Fail> + Send + 'static) {
        if self.mods.busy {
            return;
        }
        self.mods.busy = true;
        self.bg(window, cx, job, |this, result, w, cx| {
            this.mods.busy = false;
            this.mods.confirm = None;
            if let Err(f) = result {
                this.toast_fail(&f, w, cx);
            }
            this.mods.loading = false;
            this.reload_mods(w, cx);
        });
    }

    fn disable(&mut self, m: &Installed, window: &mut Window, cx: &mut Context<Self>) {
        let (from, to) = (m.dir.clone(), mods::app_dir().join("mod-store").join("disabled").join(&m.name));
        self.change_mod(window, cx, move || store_::move_dir(&from, &to));
    }

    fn enable(&mut self, m: &Installed, window: &mut Window, cx: &mut Context<Self>) {
        let (from, to) = (m.dir.clone(), mods::app_dir().join("mods").join(&m.name));
        self.change_mod(window, cx, move || store_::move_dir(&from, &to));
    }

    fn remove(&mut self, m: &Installed, window: &mut Window, cx: &mut Context<Self>) {
        if self.mods.confirm.as_deref() != Some(m.name.as_str()) {
            self.mods.confirm = Some(m.name.clone());
            return;
        }
        let dir = m.dir.clone();
        self.change_mod(window, cx, move || std::fs::remove_dir_all(dir).map_err(|e| Fail { key: "settings.it_didn_t_work".into(), body: Some(e.to_string()) }));
    }

    fn check_update(&mut self, m: &Installed, window: &mut Window, cx: &mut Context<Self>) {
        let name = m.name.clone();
        let Some(i) = m.install.clone() else { return };
        self.bg(
            window,
            cx,
            move || store_::latest_commit(i["source"].as_str().unwrap_or_default(), i["ref"].as_str().filter(|r| !r.is_empty())).unwrap_or_default(),
            move |this, latest, _, _| {
                this.mods.latest.insert(name, latest);
            },
        );
    }

    /// Once per visit, asks each installed mod's source for its newest commit.
    fn auto_check(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.mods.auto_checked {
            return;
        }
        let Some(list) = self.mods.list.clone() else { return };
        self.mods.auto_checked = true;
        let todo: Vec<Installed> = list.into_iter().filter(|m| !m.disabled && m.commit().is_some() && !self.mods.latest.contains_key(&m.name)).collect();
        cx.spawn_in(window, async move |this, cx| {
            for m in todo {
                let Some(i) = m.install.clone() else { continue };
                let latest = cx
                    .background_executor()
                    .spawn(async move { store_::latest_commit(i["source"].as_str().unwrap_or_default(), i["ref"].as_str().filter(|r| !r.is_empty())).unwrap_or_default() })
                    .await;
                if this.update(cx, |this, cx| {
                    this.mods.latest.insert(m.name, latest);
                    cx.notify();
                }).is_err() {
                    break;
                }
            }
        })
        .detach();
    }

    /// Downloads an entry's preview and its author's avatar (once each).
    fn load_images(&mut self, r: &Entry, window: &mut Window, cx: &mut Context<Self>) {
        let (owner, repo) = r.full_name.split_once('/').unwrap_or((&r.full_name, ""));
        let (owner, repo) = (owner.to_owned(), repo.to_owned());
        let cache = mods::app_dir().join("mod-store").join("cache");
        let base = format!("https://raw.githubusercontent.com/{}/HEAD", r.full_name);
        let bare = repo.strip_prefix("herdr-").unwrap_or(&repo).trim_end_matches("-mod").to_owned();
        // Shared folders hold their preview.png where the repo put them: try the usual places.
        let urls = vec![
            format!("{base}/preview.png"),
            format!("{base}/herdr/{repo}/preview.png"),
            format!("{base}/herdr/{bare}/preview.png"),
            format!("{base}/herdr/{bare}-mod/preview.png"),
            format!("{base}/herdr/preview.png"),
        ];
        let jobs = [
            (r.full_name.clone(), cache.join(format!("{owner}__{repo}.png")), urls),
            (format!("avatar:{owner}"), cache.join(format!("avatar-{owner}.png")), vec![format!("https://github.com/{owner}.png?size=64")]),
        ];
        for (key, path, urls) in jobs {
            if self.mods.previews.contains_key(&key) {
                continue;
            }
            self.mods.previews.insert(key.clone(), Img::Loading);
            self.bg(window, cx, move || store_::download(&path, &urls), move |this, ok, _, _| {
                this.mods.previews.insert(key, if ok { Img::Ok } else { Img::Failed });
            });
        }
    }

    fn preview_ok(&self, key: &str) -> bool {
        self.mods.previews.get(key) == Some(&Img::Ok)
    }

    fn seg<V: PartialEq + Clone + 'static>(&self, cx: &mut Context<Self>, id: &str, choices: Vec<(V, String)>, now: V, set: impl Fn(&mut Self, V) + 'static) -> Div {
        let view = cx.entity().downgrade();
        controls::segmented(id, choices, &now, move |v: V, _w, cx| {
            view.update(cx, |this, cx| {
                set(this, v);
                cx.notify();
            })
            .ok();
        })
    }

    fn search_box(&self, id: &'static str, input: &Entity<InputState>, window: &Window, cx: &App) -> Stateful<Div> {
        let focused = input.read(cx).focus_handle(cx).is_focused(window);
        div().id(id).w(z(200.0)).h(z(32.0)).px(z(10.0)).flex().items_center().rounded(z(8.0)).border_1().border_color(if focused { c("ring") } else { c("border") }).bg(super::super::dialog::field_bg()).text_size(z(12.5)).on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation()).child(div().size_full().child(gpui_base::input::Input::new(input)))
    }

    fn install_box(&mut self, window: &Window, cx: &mut Context<Self>) -> Div {
        let focused = self.mods.source_input.read(cx).focus_handle(cx).is_focused(window);
        div()
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .gap(z(8.0))
            .child(controls::field_fill(&self.mods.source_input, focused))
            .child(button("st-mod-install", i18n::t("settings.mods_install"), self.act(cx, |this, w, cx| {
                let source = this.mods.source.clone();
                this.start_install(source, None, w, cx);
            })))
            .child(button("st-mod-pick", i18n::t("settings.import_pick_folder"), self.act(cx, |this, w, cx| {
                this.pick_folder(i18n::t("settings.import_pick_folder"), w, cx, |this, dir, w, cx| {
                    this.mods.source = dir.clone();
                    this.mods.source_input.update(cx, |s, cx| s.set_value(dir.clone(), w, cx));
                    this.start_install(dir, None, w, cx);
                });
            })))
    }

    fn review_panel(&mut self, cx: &mut Context<Self>) -> Div {
        let Some(r) = &self.mods.review else { return div() };
        let kinds = |k: ItemKind| match k {
            ItemKind::Mod => "settings.mods_kind_mod",
            ItemKind::Theme => "settings.mods_kind_theme",
            ItemKind::Layout => "settings.mods_kind_layout",
        };
        let any = r.items.iter().any(|i| i.checked && !i.exists);
        let mut panel = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(z(10.0))
            .p(z(14.0))
            .rounded(z(10.0))
            .border_1()
            .border_color(c("border"))
            .bg(c("surface"))
            .child(div().text_size(z(14.0)).font_weight(FontWeight::SEMIBOLD).text_color(c("foreground")).child(r.src.label.clone()));
        if let Some(commit) = &r.commit {
            panel = panel.child(div().text_size(z(12.0)).text_color(tone("dim")).child(format!("{} {}", i18n::t("settings.mods_commit"), commit.chars().take(7).collect::<String>())));
        }
        for (n, i) in r.items.iter().enumerate() {
            let on = i.checked && !i.exists;
            let exists = i.exists;
            let row = div()
                .id(eid(format!("st-rev-{n}")))
                .min_h(z(32.0))
                .flex()
                .flex_row()
                .items_center()
                .gap(z(10.0))
                .rounded(z(6.0))
                .when(!exists, |d| d.cursor_pointer().hover(|s| s.bg(c("raised"))))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(i) = this.mods.review.as_mut().and_then(|r| r.items.get_mut(n)).filter(|i| !i.exists) {
                        i.checked = !i.checked;
                        cx.notify();
                    }
                }))
                .child(
                    div()
                        .w(z(18.0))
                        .h(z(18.0))
                        .flex_none()
                        .rounded(z(5.0))
                        .border_1()
                        .border_color(c("border"))
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(if on { c("accent") } else { super::super::dialog::field_bg() })
                        .when(on, |d| d.child(svg().data(include_bytes!("../../../assets/icons/settings/check.svg")).size(z(12.0)).text_color(on_accent()))),
                )
                .child(div().text_size(z(13.0)).text_color(if exists { c("dim") } else { c("foreground") }).child(i.name.clone()))
                .child(badge(i18n::t(kinds(i.kind)), false))
                .when(exists, |d| d.child(div().text_size(z(12.0)).text_color(tone("blocked")).child(i18n::t("settings.mods_exists"))))
                .children(caps_badges(&i.caps));
            panel = panel.child(row);
        }
        panel
            .child(div().text_size(z(12.5)).line_height(z(18.0)).text_color(tone("muted")).child(i18n::t("settings.mods_trust_warning")))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(z(8.0))
                    .child(if any {
                        primary("st-rev-ok", i18n::t("settings.mods_confirm_install"), self.act(cx, |this, w, cx| this.confirm_install(w, cx))).into_any_element()
                    } else {
                        inert(i18n::t("settings.mods_confirm_install")).into_any_element()
                    })
                    .child(button("st-rev-cancel", i18n::t("settings.cancel"), self.act(cx, |this, _, _| this.cancel_review()))),
            )
    }

    fn picture(&self, m: &Installed) -> Div {
        let home = mods::app_dir();
        let inner: AnyElement = if let Some(p) = m.meta["preview"].as_str() {
            img(home.join(&m.rel).join(p)).w(z(56.0)).h(z(56.0)).object_fit(gpui::ObjectFit::Cover).into_any_element()
        } else if let Some(icon) = m.meta["panel"]["icon"].as_str().filter(|i| home.join(i).exists()) {
            svg().external_path(home.join(icon).to_string_lossy().into_owned()).size(z(26.0)).text_color(tone("muted")).into_any_element()
        } else {
            div().text_size(z(22.0)).font_weight(FontWeight::SEMIBOLD).text_color(tone("muted")).child(m.title().chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default()).into_any_element()
        };
        div().w(z(56.0)).h(z(56.0)).flex_none().rounded(z(10.0)).overflow_hidden().flex().items_center().justify_center().bg(c("hover")).child(inner)
    }

    fn card(&mut self, m: &Installed, i: usize, cx: &mut Context<Self>) -> Stateful<Div> {
        let kind = if m.shipped { "settings.mods_shipped" } else if m.install.is_some() { "settings.mods_installed" } else { "settings.mods_personal" };
        let delete_label = if self.mods.confirm.as_deref() == Some(m.name.as_str()) { "settings.confirm_delete" } else { "settings.mods_delete" };
        let mut actions: Vec<AnyElement> = Vec::new();
        let (mm, id) = (m.clone(), |k: &str| eid(format!("st-mod-{k}-{i}")));
        if m.shipped {
            let name = m.name.clone();
            actions.push(button(id("dup"), i18n::t("settings.mods_duplicate"), self.act(cx, move |this, w, cx| this.run("mod.clone", json!({ "name": name }), w, cx))).into_any_element());
        } else if m.disabled {
            let (a, b) = (mm.clone(), mm.clone());
            actions.push(button(id("en"), i18n::t("settings.mods_enable"), self.act(cx, move |this, w, cx| this.enable(&a, w, cx))).into_any_element());
            actions.push(button(id("del"), i18n::t(delete_label), self.act(cx, move |this, w, cx| this.remove(&b, w, cx))).into_any_element());
        } else {
            let (a, b, dir) = (mm.clone(), mm.clone(), m.dir.clone());
            actions.push(button(id("dis"), i18n::t("settings.mods_disable"), self.act(cx, move |this, w, cx| this.disable(&a, w, cx))).into_any_element());
            actions.push(button(id("del"), i18n::t(delete_label), self.act(cx, move |this, w, cx| this.remove(&b, w, cx))).into_any_element());
            actions.push(button(id("open"), i18n::t("settings.mods_open_folder"), controls::act(move |_, _| open_path(&dir))).into_any_element());
            if let Some(commit) = m.commit() {
                match self.mods.latest.get(&m.name) {
                    Some(latest) if !latest.is_empty() && latest != commit => {
                        let (c1, input) = (mm.clone(), m.source_input().unwrap_or_default());
                        let name = c1.name.clone();
                        actions.push(button(id("upd"), i18n::t("settings.mods_update"), self.act(cx, move |this, w, cx| this.start_install(input.clone(), Some(name.clone()), w, cx))).into_any_element());
                    }
                    Some(latest) if !latest.is_empty() => {
                        actions.push(div().h(z(32.0)).flex().items_center().text_size(z(12.5)).text_color(tone("dim")).child(i18n::t("settings.mods_up_to_date")).into_any_element());
                    }
                    _ => {
                        let c1 = mm.clone();
                        actions.push(button(id("chk"), i18n::t("settings.mods_check_update"), self.act(cx, move |this, w, cx| this.check_update(&c1, w, cx))).into_any_element());
                    }
                }
            }
        }
        let short = m.commit().map(|c| c.chars().take(7).collect::<String>());
        let mut head = div().flex().flex_row().items_center().gap(z(8.0)).flex_wrap().child(div().text_size(z(14.0)).font_weight(FontWeight::SEMIBOLD).text_color(c("foreground")).child(m.title()));
        for key in ["version", "author"] {
            if let Some(v) = m.meta[key].as_str().filter(|v| !v.is_empty()) {
                head = head.child(div().text_size(z(12.0)).text_color(tone("dim")).child(v.to_owned()));
            }
        }
        head = head.child(badge(i18n::t(kind), false)).child(badge(i18n::t(if m.disabled { "settings.mods_off" } else { "settings.mods_on" }), false));
        if self.mods.has_update(m) {
            head = head.child(badge(i18n::t("settings.mods_update_available"), true));
        }
        let mut body = div().flex_1().min_w_0().flex().flex_col().gap(z(5.0)).child(head);
        if let Some(d) = m.meta["description"].as_str().filter(|d| !d.is_empty()) {
            body = body.child(div().text_size(z(12.5)).line_height(z(18.0)).text_color(tone("muted")).child(text(d)));
        }
        if let Some(i) = &m.install {
            let line = format!("{}{} · {}", i["source"].as_str().unwrap_or_default(), short.map(|s| format!(" @ {s}")).unwrap_or_default(), i["installed_at"].as_str().unwrap_or_default().chars().take(10).collect::<String>());
            body = body.child(div().text_size(z(11.5)).text_color(tone("dim")).truncate().child(line));
        }
        let caps = caps_badges(&m.caps);
        if !caps.is_empty() {
            body = body.child(div().flex().flex_row().flex_wrap().gap(z(5.0)).children(caps));
        }
        body = body.child(div().flex().flex_row().flex_wrap().gap(z(8.0)).pt(z(4.0)).children(actions));
        div()
            .id(eid(format!("st-mod-{}", m.name)))
            .w_full()
            .flex()
            .flex_row()
            .items_start()
            .gap(z(14.0))
            .p(z(12.0))
            .rounded(z(10.0))
            .border_1()
            .border_color(c("border"))
            .bg(c("surface"))
            .child(self.picture(m))
            .child(body)
    }

    fn detail_view(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let Some(r) = self.mods.detail.clone() else { return div() };
        let (owner, repo) = r.full_name.split_once('/').map(|(o, p)| (o.to_owned(), p.to_owned())).unwrap_or_default();
        self.load_images(&r, window, cx);
        if !self.mods.compat.contains_key(&r.full_name) {
            self.mods.compat.insert(r.full_name.clone(), None);
            let name = r.full_name.clone();
            let key = name.clone();
            self.bg(window, cx, move || store_::compatible(&name), move |this, v, _, _| {
                this.mods.compat.insert(key, v);
            });
        }
        let incompatible = self.mods.compat.get(&r.full_name) == Some(&Some(false));
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
        let days = store_::age_days(&r.pushed_at, now);
        let kind_label = i18n::t(KINDS.iter().find(|(k, _)| *k == self.mods.kind).map_or("settings.mods_kind_mods", |(_, l)| l));
        let mut facts = vec![(i18n::t("settings.mods_author"), owner.clone()), (i18n::t("settings.mods_stars"), format!("\u{2605} {}", r.stargazers_count))];
        if let Some(d) = days {
            facts.push((i18n::t("settings.mods_updated"), i18n::tf("settings.mods_days_ago", &[("days", &d.to_string())])));
        }
        facts.push((i18n::t("settings.mods_source"), r.full_name.clone()));
        let cache = mods::app_dir().join("mod-store").join("cache");
        let hero: AnyElement = if self.preview_ok(&r.full_name) {
            img(cache.join(format!("{owner}__{repo}.png"))).h(z(200.0)).object_fit(gpui::ObjectFit::Contain).into_any_element()
        } else {
            svg().data(include_bytes!("../../../assets/icons/settings/puzzle.svg")).size(z(40.0)).text_color(tone("dim")).into_any_element()
        };
        let avatar = div().w(z(36.0)).h(z(36.0)).flex_none().rounded(z(10.0)).bg(c("raised")).overflow_hidden().when(self.preview_ok(&format!("avatar:{owner}")), |d| d.child(img(cache.join(format!("avatar-{owner}.png"))).size(z(36.0))));
        let (url, name) = (r.html_url.clone(), r.full_name.clone());
        let install = if self.mods.busy { inert(i18n::t("settings.mods_installing")).into_any_element() } else if incompatible {
            inert(i18n::t("settings.mods_install")).into_any_element()
        } else {
            primary("st-detail-install", i18n::t("settings.mods_install"), self.act(cx, move |this, w, cx| this.start_install(name.clone(), None, w, cx))).into_any_element()
        };
        let mut info = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(z(10.0))
            .p(z(16.0))
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(z(10.0))
                    .child(avatar)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(div().truncate().text_size(z(18.0)).font_weight(FontWeight::SEMIBOLD).text_color(c("foreground")).child(repo))
                            .child(div().text_size(z(12.0)).text_color(tone("dim")).child(format!("{owner} · {kind_label}"))),
                    )
                    .child(button("st-detail-github", i18n::t("settings.mods_view_github"), controls::act(move |_, _| crate::links::open(&crate::links::Target::Url(url.clone())))))
                    .child(install),
            )
            .child(div().text_size(z(13.0)).line_height(z(19.0)).text_color(tone("muted")).child(if r.description.is_empty() { i18n::t("settings.mods_no_description") } else { r.description.clone() }))
            .child(div().w_full().flex().flex_col().gap(z(4.0)).pt(z(4.0)).children(facts.into_iter().map(|(k, v)| {
                div().w_full().flex().flex_row().gap(z(10.0)).child(div().w(z(90.0)).flex_none().text_size(z(12.0)).text_color(tone("dim")).child(k)).child(div().flex_1().min_w_0().truncate().text_size(z(12.0)).text_color(c("foreground")).child(v))
            })))
            .child(div().text_size(z(12.0)).line_height(z(17.0)).text_color(tone("dim")).child(i18n::t("settings.mods_trust_warning")));
        if incompatible {
            info = info.child(div().text_size(z(12.5)).line_height(z(18.0)).text_color(tone("blocked")).child(i18n::t("settings.mods_incompatible")));
        }
        if self.mods.busy && self.mods.review.is_none() {
            info = info.child(div().text_size(z(12.5)).text_color(tone("muted")).child(i18n::t("settings.mods_installing")));
        }
        if let (Some(e), false) = (&self.mods.install_error, self.mods.busy) {
            info = info.child(div().text_size(z(12.5)).line_height(z(18.0)).text_color(tone("blocked")).child(e.clone()));
        }
        if self.mods.review.is_some() {
            info = info.child(self.review_panel(cx));
        }
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(z(14.0))
            .child(div().flex().child(button("st-detail-back", i18n::t("settings.mods_back"), self.act(cx, |this, _, _| {
                this.mods.detail = None;
                this.mods.install_error = None;
            }))))
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .rounded(z(12.0))
                    .border_1()
                    .border_color(c("border"))
                    .bg(c("surface"))
                    .overflow_hidden()
                    .child(div().w_full().h(z(200.0)).bg(c("raised")).flex().items_center().justify_center().overflow_hidden().child(hero))
                    .child(info),
            )
    }

    /// The gallery: layouts, mods or themes people shared, as cards.
    fn explore(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        if self.mods.detail.is_some() {
            return self.detail_view(window, cx);
        }
        let kind = self.mods.kind;
        if !self.mods.found.contains_key(kind) && self.mods.found_loading.insert(kind) {
            self.bg(window, cx, move || store_::catalog(kind), move |this, r, _, _| {
                this.mods.found_loading.remove(kind);
                this.mods.found.insert(kind, r);
            });
        }
        let q = self.mods.query.clone();
        let (mut rows, error): (Vec<Entry>, Option<Fail>) = match self.mods.found.get(kind) {
            Some(Ok(l)) => (l.iter().filter(|r| q.is_empty() || r.full_name.to_lowercase().contains(&q) || r.description.to_lowercase().contains(&q)).cloned().collect(), None),
            Some(Err(e)) => (Vec::new(), Some(e.clone())),
            None => (Vec::new(), None),
        };
        match self.mods.sort {
            Sort::Stars => rows.sort_by_key(|r| std::cmp::Reverse(r.stargazers_count)),
            Sort::Recent => rows.sort_by(|a, b| b.pushed_at.cmp(&a.pushed_at)),
        }
        let loaded = matches!(self.mods.found.get(kind), Some(Ok(_)));
        let have: HashSet<String> = self.mods.list.iter().flatten().filter_map(|m| m.install.as_ref()?["source"].as_str().map(str::to_lowercase)).collect();
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
        let cache = mods::app_dir().join("mod-store").join("cache");
        let kinds: Vec<(&'static str, String)> = KINDS.iter().map(|(k, l)| (*k, i18n::t(l))).collect();
        let sorts = vec![(Sort::Stars, i18n::t("settings.mods_sort_popular")), (Sort::Recent, i18n::t("settings.mods_sort_recent"))];
        let mut page = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(z(12.0))
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(z(10.0))
                    .child(self.seg(cx, "mods-kind", kinds, kind, |this, v| this.mods.kind = kind_key(v)))
                    .child(div().flex_1())
                    .child(self.search_box("st-cat-q", &self.mods.query_input.clone(), window, cx))
                    .child(self.seg(cx, "mods-sort", sorts, self.mods.sort, |this, v| this.mods.sort = v)),
            );
        if !loaded && error.is_none() {
            page = page.child(div().text_size(z(12.5)).text_color(tone("dim")).child("\u{2026}"));
        }
        if let Some(e) = &error {
            page = page.child(div().text_size(z(12.5)).text_color(tone("blocked")).child(i18n::t(&e.key)));
        }
        if loaded && rows.is_empty() {
            page = page.child(div().text_size(z(12.5)).text_color(tone("dim")).child(i18n::t("settings.mods_no_results")));
        }
        let mut grid = div().w_full().flex().flex_row().flex_wrap().items_start().gap(z(12.0));
        for (i, r) in rows.into_iter().enumerate() {
            if i < 12 {
                self.load_images(&r, window, cx);
            }
            let (owner, repo) = r.full_name.split_once('/').map(|(o, p)| (o.to_owned(), p.to_owned())).unwrap_or_default();
            let days = store_::age_days(&r.pushed_at, now);
            let sub = match days {
                Some(d) => format!("{owner} · {}", i18n::tf("settings.mods_days_ago", &[("days", &d.to_string())])),
                None => owner.clone(),
            };
            let thumb: AnyElement = if self.preview_ok(&r.full_name) {
                img(cache.join(format!("{owner}__{repo}.png"))).w_full().h(z(108.0)).object_fit(gpui::ObjectFit::Cover).into_any_element()
            } else {
                svg().data(include_bytes!("../../../assets/icons/settings/puzzle.svg")).size(z(28.0)).text_color(tone("dim")).into_any_element()
            };
            let avatar = div().w(z(28.0)).h(z(28.0)).flex_none().rounded(z(8.0)).bg(c("raised")).overflow_hidden().when(self.preview_ok(&format!("avatar:{owner}")), |d| d.child(img(cache.join(format!("avatar-{owner}.png"))).size(z(28.0))));
            let entry = r.clone();
            grid = grid.child(
                div()
                    .id(eid(format!("st-cat-{i}")))
                    .w(z(196.0))
                    .h(z(222.0))
                    .flex_none()
                    .flex()
                    .flex_col()
                    .rounded(z(10.0))
                    .border_1()
                    .border_color(c("border"))
                    .bg(c("surface"))
                    .overflow_hidden()
                    .cursor_pointer()
                    .hover(|s| s.bg(c("raised")))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.mods.detail = Some(entry.clone());
                        cx.notify();
                    }))
                    .child(div().w_full().h(z(108.0)).flex_none().bg(c("raised")).overflow_hidden().flex().items_center().justify_center().child(thumb))
                    .child(
                        div()
                            .w_full()
                            .flex_none()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(z(8.0))
                            .p(z(8.0))
                            .child(avatar)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .flex()
                                            .flex_row()
                                            .items_center()
                                            .gap(z(6.0))
                                            .child(div().id("mod-repo").truncate().text_size(z(13.0)).font_weight(FontWeight::SEMIBOLD).text_color(c("foreground")).tooltip(crate::window::native::tip(repo.to_string())).child(repo.clone()))
                                            .when(have.contains(&r.full_name.to_lowercase()), |d| d.child(badge(i18n::t("settings.mods_installed"), false))),
                                    )
                                    .child(div().id("mod-sub").truncate().text_size(z(11.5)).text_color(tone("dim")).tooltip(crate::window::native::tip(sub.to_string())).child(sub)),
                            )
                            .child(div().flex_none().text_size(z(11.0)).text_color(tone("muted")).child(format!("\u{2605} {}", r.stargazers_count))),
                    )
                    .child(div().px(z(10.0)).h(z(36.0)).flex_none().overflow_hidden().text_size(z(11.0)).line_height(z(15.0)).text_color(tone("muted")).child(r.description.clone())),
            );
        }
        page.child(grid)
    }

    pub(super) fn mods_page(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        if self.mods.list.is_none() {
            self.reload_mods(window, cx);
        }
        let list = self.mods.list.clone().unwrap_or_default();
        let shown: Vec<Installed> = self.mods.visible(&list).into_iter().cloned().collect();
        let updates = list.iter().filter(|m| self.mods.has_update(m)).count();
        if self.mods.tab == Tab::Installed {
            self.auto_check(window, cx);
        }
        let installed_tab = self.mods.tab == Tab::Installed;
        let detail = self.mods.detail.is_some();
        let tabs = vec![(Tab::Installed, i18n::t("settings.mods_tab_installed")), (Tab::Explore, i18n::t("settings.mods_tab_explore"))];
        let mut page = div().w_full().flex().flex_col().gap(z(12.0)).child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(z(10.0))
                .child(self.seg(cx, "mods-tab", tabs, self.mods.tab, |this, v| this.mods.tab = v))
                .when(installed_tab && !list.is_empty(), |d| d.child(div().text_size(z(12.0)).text_color(tone("dim")).child(i18n::tf("settings.mods_count", &[("count", &list.len().to_string())])))),
        );
        page = page.child(self.install_box(window, cx));
        if self.mods.review.is_some() && !detail {
            page = page.child(self.review_panel(cx));
        }
        if self.mods.busy && self.mods.review.is_none() && !detail {
            page = page.child(div().text_size(z(12.5)).text_color(tone("dim")).child(i18n::t("settings.mods_installing")));
        }
        if let (Some(e), false, false) = (&self.mods.install_error, self.mods.busy, detail) {
            page = page.child(div().text_size(z(12.5)).line_height(z(18.0)).text_color(tone("blocked")).child(e.clone()));
        }
        if !installed_tab {
            return page.child(self.explore(window, cx));
        }
        if !list.is_empty() {
            let mut filters = vec![(Filter::All, "settings.mods_filter_all"), (Filter::Active, "settings.mods_filter_active"), (Filter::Disabled, "settings.mods_filter_disabled")];
            if updates > 0 || self.mods.filter == Filter::Updates {
                filters.push((Filter::Updates, "settings.mods_filter_updates"));
            }
            let filters = filters.into_iter().map(|(f, l)| (f, i18n::t(l))).collect();
            page = page.child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(z(10.0))
                    .child(self.seg(cx, "mods-filter", filters, self.mods.filter, |this, v| this.mods.filter = v))
                    .child(div().flex_1())
                    .when(updates > 1, |d| d.child(button("st-mod-upd-all", i18n::tf("settings.mods_update_all", &[("count", &updates.to_string())]), self.act(cx, |this, w, cx| this.update_all(w, cx)))))
                    .child(self.search_box("st-inst-q", &self.mods.iq_input.clone(), window, cx)),
            );
        }
        if self.mods.list.is_some() && list.is_empty() {
            page = page.child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(z(8.0))
                    .items_start()
                    .child(div().text_size(z(12.5)).text_color(tone("dim")).child(i18n::t("settings.mods_empty_installed")))
                    .child(button("st-mod-explore", i18n::t("settings.mods_tab_explore"), self.act(cx, |this, _, _| this.mods.tab = Tab::Explore))),
            );
        } else if !list.is_empty() && shown.is_empty() {
            page = page.child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(z(8.0))
                    .items_start()
                    .child(div().text_size(z(12.5)).text_color(tone("dim")).child(i18n::t("settings.mods_no_results")))
                    .child(button("st-mod-clear", i18n::t("settings.mods_clear_search"), self.act(cx, |this, w, cx| {
                        this.mods.filter = Filter::All;
                        this.mods.iq.clear();
                        this.mods.iq_input.update(cx, |s, cx| s.set_value("", w, cx));
                    }))),
            );
        }
        let index = |m: &Installed| list.iter().position(|x| x.name == m.name && x.disabled == m.disabled).unwrap_or(0);
        let (bundled, personal): (Vec<Installed>, Vec<Installed>) = shown.into_iter().partition(|m| m.shipped);
        if !personal.is_empty() {
            let mut col = div().w_full().flex().flex_col().gap(z(10.0));
            for m in &personal {
                col = col.child(self.card(m, index(m), cx));
            }
            page = page.child(col);
        }
        if !bundled.is_empty() {
            let mut col = div().w_full().flex().flex_col().gap(z(10.0));
            for m in &bundled {
                col = col.child(self.card(m, index(m), cx));
            }
            page = page.child(div().pt(z(12.0)).text_size(z(12.5)).font_weight(FontWeight::MEDIUM).text_color(tone("muted")).child(i18n::t("settings.mods_bundled"))).child(col);
        }
        page
    }
}

#[cfg(test)]
mod tests {
    use super::{Caps, Filter, Installed, KINDS, kind_key, text, update_available, visible_in};
    use crate::i18n;
    use serde_json::{Value, json};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn installed(name: &str, disabled: bool, meta: Value, install: Option<Value>) -> Installed {
        Installed { name: name.into(), disabled, dir: PathBuf::from("/m").join(name), rel: format!("mods/{name}"), meta, install, caps: Caps::default(), shipped: false }
    }

    #[test]
    fn the_installed_list_filters_by_state_and_by_search() {
        let list = vec![
            installed("hola", false, json!({ "name": "Hola", "description": "Prueba" }), None),
            installed("zeta", false, json!({ "name": "Zeta", "description": "otra cosa" }), None),
            installed("off", true, json!({ "name": "Off" }), None),
            installed("upd", false, json!({}), Some(json!({ "source": "a/upd", "commit": "aaa1111" }))),
        ];
        let mut latest = HashMap::new();
        let names = |f: Filter, iq: &str, latest: &HashMap<String, String>| visible_in(&list, f, iq, latest).iter().map(|m| m.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(Filter::All, "", &latest), ["hola", "zeta", "off", "upd"]);
        assert_eq!(names(Filter::Disabled, "", &latest), ["off"]);
        assert_eq!(names(Filter::Active, "", &latest), ["hola", "zeta", "upd"]);
        assert_eq!(names(Filter::All, "otra", &latest), ["zeta"]);
        assert_eq!(names(Filter::All, "hola", &latest), ["hola"]);
        assert!(names(Filter::All, "zzz", &latest).is_empty());
        // A newer commit than installed shows in the Updates filter.
        latest.insert("upd".into(), "bbb2222".into());
        assert_eq!(names(Filter::Updates, "", &latest), ["upd"]);
        latest.insert("upd".into(), "aaa1111".into());
        assert!(names(Filter::Updates, "", &latest).is_empty());
        latest.insert("upd".into(), String::new());
        assert!(!update_available(&list[3], &latest));
    }

    #[test]
    fn a_locale_key_translates_and_anything_else_is_literal() {
        assert_eq!(text("settings.cancel"), i18n::t("settings.cancel"));
        assert_eq!(text("Just words"), "Just words");
    }

    #[test]
    fn kinds_are_the_three_github_topics() {
        assert_eq!(KINDS.map(|(k, _)| k), ["herdr-layout", "herdr-mod", "herdr-theme"]);
        assert_eq!(kind_key("herdr-theme"), "herdr-theme");
        assert_eq!(kind_key("x"), "herdr-mod");
    }
}
