// Settings > Mods: what is installed, its card, and turning mods off or deleting them.

import { div, svg, image } from "gpui-kit";
import { v_flex, h_flex, InputState, Input } from "gpui-base";
import { run, appHome, exec, notify, pickFolder, window, tr } from "herdr";
import * as fs from "fs/promises";
import { moveDir, removeDir, listDirs, scanCapabilities, readJson } from "./fsx.js";
import { parseSource, fetchSource, scanSource, withExisting, installItems, latestCommit, catalog, compatible, ageDays } from "./store.js";

// The mods that ship in defaults/mods (a test keeps this list equal to the folder).
export const SHIPPED_MODS = [
  "add-project", "agents", "awake-menu", "bots", "changes", "confirm", "files", "folders", "help", "machine-add", "menu", "new-menu", "pane", "peek",
  "picker", "prompt", "search", "spaces", "tabs", "terminals-bar", "terminals-pane", "toast", "win98", "workspace-chrome", "workspace-status", "worktree",
];

/** A locale key shows translated; anything else is literal text. */
export const text = (k) => (typeof k === "string" && tr(k) !== k ? tr(k) : k);

export function openFolder(dir) { // also opens a URL
  const p = window()?.platform;
  return exec(p === "macos" ? "open" : p === "windows" ? "explorer" : "xdg-open", [dir]).catch(() => null);
}

export class ModsSection {
  constructor(host) {
    this.host = host;
    this.mods = null; // [{ name, disabled, dir, rel, meta, install, caps, shipped }]
    this.confirm = null; // the name whose delete awaits a second click
    this.busy = false;
    this.detail = null; // the gallery entry whose page is open
    this.tab = "installed"; // "installed" | "explore"
    this.installError = null; // why the last install could not start, shown where the person is looking
    this.review = null; // { src, dir, staging, commit, items, replace }: what an install would write
    this.latest = {}; // mod name -> newest commit of its source, once asked
    this.kind = "herdr-mod";
    this.found = {}; // catalog kind -> list | Error
    this.previews = {}; // "owner/repo" -> "loading" | "ok" | "failed"
    this.source = "";
    this.query = ""; this.sort = "stars"; // the gallery's search and order
    this.queryInput = InputState.new({ placeholder: tr("settings.mods_search") });
    this.queryInput.on("change", (_e, cx) => { this.query = this.queryInput.value().trim().toLowerCase(); cx.notify(); });
    this.sourceInput = InputState.new({ placeholder: tr("settings.mods_source_placeholder") });
    this.sourceInput.on("change", (_e, cx) => { this.source = this.sourceInput.value().trim(); cx.notify(); });
    this.sourceInput.on("submit", (_e, cx) => this.startInstall(this.source, cx));
  }

  // Installing ---------------------------------------------------------------

  /** Reads a source (clone or folder) and shows what it holds, before anything is written. */
  startInstall(input, cx, replace = null) {
    if (this.busy) return;
    this.busy = true;
    this.installError = null;
    cx.notify();
    cx.spawn(async (cx) => {
      try {
        const src = parseSource(input);
        const got = await fetchSource(src);
        try {
          const items = await withExisting(await scanSource(got.dir, src.name), replace);
          for (const i of items) {
            if (i.type === "mod") i.caps = await scanCapabilities(i.path);
            i.checked = !i.exists;
          }
          this.review = { src, dir: got.dir, staging: got.staging, commit: got.commit, items, replace };
        } catch (e) {
          if (got.staging) await removeDir(got.staging).catch(() => null);
          throw e;
        }
      } catch (e) {
        this.installError = [tr(e.message), e.body].filter(Boolean).join(": ");
        notify({ level: "error", title: tr(e.message), body: e.body ?? undefined });
      }
      this.busy = false;
      cx.notify();
    });
  }

  async dropStaging() {
    if (this.review?.staging) await removeDir(this.review.staging).catch(() => null);
  }

  cancelReview(cx) {
    cx.spawn(async (cx) => { await this.dropStaging(); this.review = null; cx.notify(); });
  }

  confirmInstall(cx) {
    const r = this.review;
    if (!r || this.busy) return;
    this.busy = true;
    cx.spawn(async (cx) => {
      try {
        const chosen = r.items.filter((i) => i.checked && !i.exists);
        const source = r.src.kind === "github" ? r.src.label : r.src.kind === "git" ? r.src.url : r.src.path;
        const count = await installItems(chosen, { source, ref: r.src.ref ?? null, commit: r.commit }, r.replace);
        await this.dropStaging();
        this.review = null; this.detail = null; this.tab = "installed";
        notify({ title: tr("settings.mods_installed_n", { count }) });
        this.sourceInput.set_value("");
        this.source = "";
        await this.reload();
      } catch (e) {
        notify({ level: "error", title: tr(e.message), body: e.body ?? undefined });
      }
      this.busy = false;
      cx.notify();
    });
  }

  checkUpdate(m, cx) {
    cx.spawn(async (cx) => {
      this.latest[m.name] = (await latestCommit(m.install).catch(() => null)) ?? "";
      cx.notify();
    });
  }

  installBox(t, cx) {
    const h = this.host;
    return v_flex().w_full().gap(8)
      .child(h_flex().gap(8).items_center()
        .child(Input.new(this.sourceInput).flex_1().min_w(0).h(32).px(10).rounded(7).border_1().border_color(t.border).bg(t.background).text_size(13).text_color(t.foreground))
        .child(h.button(t, "st-mod-install", tr("settings.mods_install"), (_e, cx) => this.startInstall(this.source, cx)))
        .child(h.button(t, "st-mod-pick", tr("settings.import_pick_folder"), (_e, cx) => cx.spawn(async (cx) => {
          const dir = await pickFolder();
          if (dir) { this.sourceInput.set_value(dir); this.source = dir; this.startInstall(dir, cx); }
        }))));
  }

  reviewPanel(t, cx) {
    const h = this.host;
    const r = this.review;
    const label = { mod: "settings.mods_kind_mod", theme: "settings.mods_kind_theme", layout: "settings.mods_kind_layout" };
    const capKeys = [["programs", "settings.mods_cap_programs"], ["network", "settings.mods_cap_network"], ["files", "settings.mods_cap_files"]];
    return v_flex().w_full().gap(10).p(14).rounded(10).border_1().border_color(t.border).bg(t.surface)
      .child(div().text_size(14).font_semibold().text_color(t.foreground).child(r.src.label))
      .when(Boolean(r.commit), (el) => el.child(div().text_size(12).text_color(t.dim).child(`${tr("settings.mods_commit")} ${r.commit.slice(0, 7)}`)))
      .children(r.items.map((i, n) =>
        h_flex().id(`st-rev-${n}`).gap(10).items_center().py(4)
          .child(div().id(`st-rev-c-${n}`).w(18).h(18).flex_none().rounded(5).border_1().border_color(t.border).flex().items_center().justify_center()
            .bg(i.checked && !i.exists ? t.accent : t.background).cursor_pointer()
            .on_click((_e, cx) => { if (!i.exists) { i.checked = !i.checked; cx.notify(); } })
            .when(i.checked && !i.exists, (el) => el.child(svg("frame/settings/check.svg").size(12).text_color(t.accent_foreground))))
          .child(div().text_size(13).text_color(i.exists ? t.dim : t.foreground).child(i.name))
          .child(this.badge(t, tr(label[i.type])))
          .when(i.exists, (el) => el.child(div().text_size(12).text_color(t.blocked).child(tr("settings.mods_exists"))))
          .children((i.caps ? capKeys.filter(([k]) => i.caps[k]) : []).map(([, k]) => this.badge(t, tr(k))))))
      .child(div().text_size(12.5).line_height("18px").text_color(t.muted).child(tr("settings.mods_trust_warning")))
      .child(h_flex().gap(8)
        .child(h.button(t, "st-rev-ok", tr("settings.mods_confirm_install"), (_e, cx) => this.confirmInstall(cx)))
        .child(h.button(t, "st-rev-cancel", tr("settings.cancel"), (_e, cx) => this.cancelReview(cx))));
  }

  async reload() {
    const home = appHome();
    const list = [];
    for (const [root, disabled, relRoot] of [[`${home}/mods`, false, "mods"], [`${home}/mod-store/disabled`, true, "mod-store/disabled"]]) {
      for (const name of await listDirs(root)) {
        const dir = `${root}/${name}`;
        list.push({
          name, disabled, dir, rel: `${relRoot}/${name}`,
          meta: (await readJson(`${dir}/mod.json`)) ?? {},
          install: await readJson(`${dir}/.herdr-install.json`),
          caps: await scanCapabilities(dir),
          shipped: !disabled && SHIPPED_MODS.includes(name),
        });
      }
    }
    this.mods = list;
  }

  /** Runs `job`, then reloads the list; failures become a toast. */
  act(cx, job) {
    if (this.busy) return;
    this.busy = true;
    cx.spawn(async (cx) => {
      try {
        await job();
      } catch (e) {
        notify({ level: "error", title: tr(e.message, e.vars ?? {}) });
      }
      this.confirm = null;
      await this.reload();
      this.busy = false;
      cx.notify();
    });
  }

  disable(m, cx) { this.act(cx, () => moveDir(m.dir, `${appHome()}/mod-store/disabled/${m.name}`)); }
  enable(m, cx) { this.act(cx, () => moveDir(m.dir, `${appHome()}/mods/${m.name}`)); }
  remove(m, cx) {
    if (this.confirm !== m.name) { this.confirm = m.name; cx.notify(); return; }
    this.act(cx, () => removeDir(m.dir));
  }

  picture(t, m) {
    const box = (c) => div().w(56).h(56).flex_none().rounded(10).overflow_hidden().flex().items_center().justify_center().bg(t.hover).child(c);
    const dim = (s) => `${m.rel}/${s}`;
    if (m.meta.preview) return box(image(dim(m.meta.preview)).w(56).h(56));
    if (m.meta.panel?.icon) return box(svg(m.meta.panel.icon).size(26).text_color(t.muted));
    return box(div().text_size(22).font_semibold().text_color(t.muted).child((m.meta.name ?? m.name)[0].toUpperCase()));
  }

  badge(t, label, tint) {
    return div().px(6).py(1).rounded(5).bg(tint ?? t.hover).text_size(11).text_color(tint ? t.accent_foreground : t.dim).child(label);
  }

  card(t, m, i) {
    const h = this.host;
    const shortCommit = m.install?.commit ? m.install.commit.slice(0, 7) : null;
    const kind = m.shipped ? tr("settings.mods_shipped") : m.install ? tr("settings.mods_installed") : tr("settings.mods_personal");
    const caps = [["programs", "settings.mods_cap_programs"], ["network", "settings.mods_cap_network"], ["files", "settings.mods_cap_files"]].filter(([k]) => m.caps[k]);
    const actions = [];
    if (m.shipped) actions.push(h.button(t, `st-mod-dup-${i}`, tr("settings.mods_duplicate"), () => run("mod.clone", { name: m.name })));
    else if (m.disabled) {
      actions.push(h.button(t, `st-mod-en-${i}`, tr("settings.mods_enable"), (_e, cx) => this.enable(m, cx)));
      actions.push(h.button(t, `st-mod-del-${i}`, this.confirm === m.name ? tr("settings.confirm_delete") : tr("settings.mods_delete"), (_e, cx) => this.remove(m, cx)));
    } else {
      actions.push(h.button(t, `st-mod-dis-${i}`, tr("settings.mods_disable"), (_e, cx) => this.disable(m, cx)));
      actions.push(h.button(t, `st-mod-del-${i}`, this.confirm === m.name ? tr("settings.confirm_delete") : tr("settings.mods_delete"), (_e, cx) => this.remove(m, cx)));
      actions.push(h.button(t, `st-mod-open-${i}`, tr("settings.mods_open_folder"), () => openFolder(m.dir)));
      if (m.install?.commit) {
        const latest = this.latest[m.name];
        if (latest && latest !== m.install.commit) {
          actions.push(h.button(t, `st-mod-upd-${i}`, tr("settings.mods_update"), (_e, cx) => this.startInstall(m.install.ref ? `${m.install.source}@${m.install.ref}` : m.install.source, cx, m.name)));
        } else if (latest) actions.push(div().py(5).text_size(12.5).text_color(t.dim).child(tr("settings.mods_up_to_date")));
        else actions.push(h.button(t, `st-mod-chk-${i}`, tr("settings.mods_check_update"), (_e, cx) => this.checkUpdate(m, cx)));
      }
    }
    return h_flex().id(`st-mod-${m.name}`).w_full().gap(14).p(12).rounded(10).border_1().border_color(t.border).bg(t.surface).items_start()
      .child(this.picture(t, m))
      .child(
        v_flex().flex_1().min_w(0).gap(5)
          .child(h_flex().gap(8).items_center().flex_wrap()
            .child(div().text_size(14).font_semibold().text_color(t.foreground).child(m.meta.name ?? m.name))
            .when(Boolean(m.meta.version), (el) => el.child(div().text_size(12).text_color(t.dim).child(m.meta.version)))
            .when(Boolean(m.meta.author), (el) => el.child(div().text_size(12).text_color(t.dim).child(m.meta.author)))
            .child(this.badge(t, kind)))
          .when(Boolean(m.meta.description), (el) => el.child(div().text_size(12.5).line_height("18px").text_color(t.muted).child(text(m.meta.description))))
          .when(Boolean(m.install), (el) => el.child(div().text_size(11.5).text_color(t.dim).truncate().child(`${m.install.source}${shortCommit ? ` @ ${shortCommit}` : ""} · ${(m.install.installed_at ?? "").slice(0, 10)}`)))
          .when(caps.length > 0, (el) => el.child(h_flex().gap(5).flex_wrap().children(caps.map(([, k]) => this.badge(t, tr(k))))))
          .child(h_flex().gap(8).pt(4).flex_wrap().children(actions)),
      );
  }

  /** Downloads the first of `urls` that exists into mod-store/cache, once; `previews[key]` says how it went. */
  cached(key, file, urls, cx) {
    if (this.previews[key]) return;
    this.previews[key] = "loading";
    cx.spawn(async (cx) => {
      const path = `${appHome()}/mod-store/cache/${file}`;
      try {
        await fs.mkdir(`${appHome()}/mod-store/cache`, { recursive: true });
        this.previews[key] = "failed";
        for (const url of [].concat(urls)) {
          const res = await exec("curl", ["-fsSL", "-o", path, url], { timeout: 20000 });
          if (res.code === 0 && (await fs.exists(path))) { this.previews[key] = "ok"; break; }
        }
      } catch {
        this.previews[key] = "failed";
      }
      cx.notify();
    });
  }

  /** Starts fetching an entry's preview and its author's avatar (once). */
  loadImages(r, cx) {
    const [owner, repo] = r.full_name.split("/");
    // Shared folders hold their preview.png where the repo put them: try the usual places.
    const base = `https://raw.githubusercontent.com/${r.full_name}/HEAD`, bare = repo.replace(/^herdr-/, "").replace(/-mod$/, "");
    this.cached(r.full_name, `${owner}__${repo}.png`, [`${base}/preview.png`, `${base}/herdr/${repo}/preview.png`, `${base}/herdr/${bare}/preview.png`, `${base}/herdr/${bare}-mod/preview.png`, `${base}/herdr/preview.png`], cx);
    this.cached(`avatar:${owner}`, `avatar-${owner}.png`, `https://github.com/${owner}.png?size=64`, cx);
  }

  /** One gallery entry's page: big preview, who made it, and what to do with it. */
  detailView(t, cx) {
    const h = this.host;
    const r = this.detail;
    const [owner, repo] = r.full_name.split("/");
    this.loadImages(r, cx);
    this.compat ??= {};
    if (this.compat[r.full_name] === undefined) {
      this.compat[r.full_name] = null;
      cx.spawn(async (cx) => { this.compat[r.full_name] = await compatible(r.full_name); cx.notify(); });
    }
    const incompatible = this.compat[r.full_name] === false;
    const days = ageDays(r.pushed_at);
    const kindLabel = { "herdr-layout": "settings.mods_kind_layouts", "herdr-mod": "settings.mods_kind_mods", "herdr-theme": "settings.mods_kind_themes" }[this.kind];
    const facts = [
      [tr("settings.mods_author"), owner],
      [tr("settings.mods_stars"), `★ ${r.stargazers_count}`],
      ...(days === null ? [] : [[tr("settings.mods_updated"), tr("settings.mods_days_ago", { days })]]),
      [tr("settings.mods_source"), r.full_name],
    ];
    return v_flex().w_full().gap(14)
      .child(h_flex().child(h.button(t, "st-detail-back", tr("settings.mods_back"), (_e, cx) => { this.detail = null; this.installError = null; cx.notify(); })))
      .child(v_flex().w_full().rounded(12).border_1().border_color(t.border).bg(t.surface).overflow_hidden()
        .child(div().w_full().h(200).bg(t.raised).flex().items_center().justify_center().overflow_hidden()
          .when(this.previews[r.full_name] !== "ok", (el) => el.child(svg("frame/settings/puzzle.svg").size(40).text_color(t.dim)))
          .when(this.previews[r.full_name] === "ok", (el) => el.child(image(`mod-store/cache/${owner}__${repo}.png`).h(200))))
        .child(v_flex().w_full().gap(10).p(16)
          .child(h_flex().w_full().gap(10).items_center()
            .child(div().size(36).flex_none().rounded(10).bg(t.raised).overflow_hidden()
              .when(this.previews[`avatar:${owner}`] === "ok", (el) => el.child(image(`mod-store/cache/avatar-${owner}.png`).size(36))))
            .child(v_flex().flex_1().min_w(0)
              .child(div().truncate().text_size(18).font_semibold().text_color(t.foreground).child(repo))
              .child(div().text_size(12).text_color(t.dim).child(`${owner} · ${tr(kindLabel)}`)))
            .child(h.button(t, "st-detail-github", tr("settings.mods_view_github"), () => openFolder(r.html_url)))
            .child(h.button(t, "st-detail-install", this.busy ? tr("settings.mods_installing") : tr("settings.mods_install"), (_e, cx) => { if (!incompatible) this.startInstall(r.full_name, cx); })))
          .child(div().text_size(13).line_height("19px").text_color(t.muted).child(r.description ?? tr("settings.mods_no_description")))
          .child(v_flex().w_full().gap(4).pt(4).children(facts.map(([k, v]) =>
            h_flex().w_full().gap(10).child(div().w(90).flex_none().text_size(12).text_color(t.dim).child(k)).child(div().flex_1().min_w(0).truncate().text_size(12).text_color(t.foreground).child(v)))))
          .child(div().text_size(11.5).line_height("16px").text_color(t.dim).child(tr("settings.mods_trust_warning")))
          .when(incompatible, (el) => el.child(div().text_size(12.5).line_height("18px").text_color(t.blocked).child(tr("settings.mods_incompatible"))))
          .when(this.busy && this.review === null, (el) => el.child(div().text_size(12.5).text_color(t.muted).child(tr("settings.mods_installing"))))
          .when(Boolean(this.installError) && !this.busy, (el) => el.child(div().text_size(12.5).line_height("18px").text_color(t.blocked).child(this.installError)))
          .when(this.review !== null, (el) => el.child(this.reviewPanel(t, cx)))
          ));
  }

  // The gallery: layouts, mods or themes people shared, as cards (the same as herdr.dev/gallery).
  explore(t, cx) {
    if (this.detail) return this.detailView(t, cx);
    const h = this.host;
    const kind = this.kind;
    if (this.found[kind] === undefined) {
      this.found[kind] = null;
      cx.spawn(async (cx) => {
        try { this.found[kind] = await catalog(kind); } catch (e) { this.found[kind] = e; }
        cx.notify();
      });
    }
    const list = this.found[kind];
    const q = this.query;
    const rows = (Array.isArray(list) ? list : [])
      .filter((r) => !q || r.full_name.toLowerCase().includes(q) || (r.description ?? "").toLowerCase().includes(q))
      .sort((a, b) => this.sort === "recent" ? Date.parse(b.pushed_at) - Date.parse(a.pushed_at) : b.stargazers_count - a.stargazers_count);
    const WIDTH = 176;
    return v_flex().w_full().gap(12)
      .child(h_flex().w_full().gap(10).items_center()
        .child(h.segmented(t, "mods-kind", [["herdr-layout", "settings.mods_kind_layouts"], ["herdr-mod", "settings.mods_kind_mods"], ["herdr-theme", "settings.mods_kind_themes"]], kind, (v, cx) => { this.kind = v; cx.notify(); }))
        .child(div().flex_1())
        .child(div().id("st-cat-q").w(200).h(30).px(10).flex().items_center().rounded(8).border_1().border_color(t.border).on_mouse_down("left", (_e, cx) => cx.stop_propagation())
          .child(Input.new(this.queryInput).h_full().text_size(12).text_color(t.foreground)))
        .child(h.segmented(t, "mods-sort", [["stars", "settings.mods_sort_popular"], ["recent", "settings.mods_sort_recent"]], this.sort, (v, cx) => { this.sort = v; cx.notify(); })))
      .when(list === null, (el) => el.child(div().text_size(12.5).text_color(t.dim).child("…")))
      .when(list instanceof Error, (el) => el.child(div().text_size(12.5).text_color(t.blocked).child(tr(list.message))))
      .when(Array.isArray(list) && rows.length === 0, (el) => el.child(div().text_size(12.5).text_color(t.dim).child(tr("settings.mods_no_results"))))
      .child(h_flex().w_full().flex_wrap().items_start().gap(12).children(rows.map((r, i) => {
        const [owner, repo] = r.full_name.split("/");
        if (i < 12) this.loadImages(r, cx);
        const days = ageDays(r.pushed_at);
        return v_flex().id(`st-cat-${i}`).w(WIDTH).h(216).flex_none().rounded(10).border_1().border_color(t.border).bg(t.surface).overflow_hidden()
          .cursor_pointer().hover((s) => s.bg(t.raised))
          .on_click((_e, cx) => { this.detail = r; cx.notify(); })
          .child(div().w_full().h(108).flex_none().bg(t.raised).overflow_hidden().flex().items_center().justify_center()
            .when(this.previews[r.full_name] !== "ok", (el) => el.child(svg("frame/settings/puzzle.svg").size(28).text_color(t.dim)))
            .when(this.previews[r.full_name] === "ok", (el) => el.child(image(`mod-store/cache/${owner}__${repo}.png`).w(WIDTH).h(108))))
          .child(h_flex().w_full().gap(8).p(8).items_center().flex_none()
            .child(div().size(28).flex_none().rounded(8).bg(t.raised).overflow_hidden()
              .when(this.previews[`avatar:${owner}`] === "ok", (el) => el.child(image(`mod-store/cache/avatar-${owner}.png`).size(28))))
            .child(v_flex().flex_1().min_w(0)
              .child(div().truncate().text_size(13).font_semibold().text_color(t.foreground).child(repo))
              .child(div().truncate().text_size(11.5).text_color(t.dim).child(days === null ? owner : `${owner} · ${tr("settings.mods_days_ago", { days })}`)))
            .child(div().flex_none().text_size(11).text_color(t.muted).child(`★ ${r.stargazers_count}`)))
          .child(div().px(10).h(34).flex_none().overflow_hidden().text_size(11).line_height("15px").text_color(t.muted).child(r.description ?? ""));
      })));
  }

  render(t, cx) {
    if (this.mods === null && !this.loading) {
      this.loading = true;
      cx.spawn(async (cx) => {
        try { await this.reload(); } catch (e) { this.mods = []; notify({ level: "error", title: String(e?.message ?? e) }); }
        this.loading = false;
        cx.notify();
      });
    }
    const list = this.mods ?? [];
    const on = list.filter((m) => !m.disabled);
    const off = list.filter((m) => m.disabled);
    const h = this.host;
    return v_flex().w_full().gap(12)
      .child(h_flex().child(h.segmented(t, "mods-tab", [["installed", "settings.mods_tab_installed"], ["explore", "settings.mods_tab_explore"]], this.tab, (v, cx) => { this.tab = v; cx.notify(); })))
      .child(this.installBox(t, cx))
      .when(this.review !== null && !this.detail, (el) => el.child(this.reviewPanel(t, cx)))
      .when(this.busy && this.review === null && !this.detail, (el) => el.child(div().text_size(12.5).text_color(t.dim).child(tr("settings.mods_installing"))))
      .when(Boolean(this.installError) && !this.busy && !this.detail, (el) => el.child(div().text_size(12.5).line_height("18px").text_color(t.blocked).child(this.installError)))
      .when(this.tab === "explore", (el) => el.child(this.explore(t, cx)))
      .when(this.tab === "installed", (el) =>
        el.child(v_flex().w_full().gap(10).children(on.map((m) => this.card(t, m, list.indexOf(m)))))
          .when(off.length > 0, (e2) =>
            e2.child(div().pt(12).text_size(12.5).font_medium().text_color(t.muted).child(tr("settings.mods_disabled")))
              .child(v_flex().w_full().gap(10).children(off.map((m) => this.card(t, m, list.indexOf(m)))))));
  }
}
