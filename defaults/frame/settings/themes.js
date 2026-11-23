// Settings > Look: theme cards with a preview, importing Ghostty / Warp / VS Code themes,
// and the editor for a theme of your own.

import { div, svg } from "gpui-kit";
import { v_flex, h_flex, InputState, Input } from "gpui-base";
import { run, themes, settings, appHome, home, readTextFile, writeTextFile, exec, pickFolder, notify, window, tr } from "herdr";
import * as fs from "fs/promises";
import { PARSERS, detect, slug, toThemeFile } from "./theme-import.js";

const I = (name) => `frame/settings/${name}.svg`;
const windows = () => window()?.platform === "windows";
const HEX6 = /^#[0-9a-f]{6}$/i;
const HEX = /^#([0-9a-f]{6}|[0-9a-f]{8})$/i;

// A theme drawn small, with its own colors and style. `entry`: a themes() item.
export function themeCard(t, entry, { selected = false, onClick, size = { w: 132, h: 84 }, id, onEdit } = {}) {
  const c = entry.colors;
  const st = entry.style ?? {};
  const label = entry.name.replace(/-/g, " ");
  const dot = (color) => div().w(6).h(6).rounded(3).bg(color).flex_none();
  const bar = (color, opacity, w) => div().h(5).w(Math.round((size.w - 44) * w)).rounded(2).bg(color).opacity(opacity);
  const thumb = h_flex()
    .relative()
    .w(size.w)
    .h(size.h)
    .flex_none()
    .rounded(st.pane_radius ?? 6)
    .overflow_hidden()
    .bg(c.background)
    .child(div().w(22).h_full().flex_none().bg(c.chrome))
    .child(
      v_flex()
        .flex_1()
        .h_full()
        .child(h_flex().w_full().h(12).flex_none().px(5).gap(3).items_center().bg(c.chrome).child(dot(c.ansi1)).child(dot(c.ansi2)))
        .child(
          v_flex().flex_1().p(7).gap(5)
            .child(bar(c.terminal_foreground, 0.7, 0.8))
            .child(bar(c.ansi2, 0.45, 0.55))
            .child(bar(c.ansi4, 0.6, 0.7))
            .child(bar(c.ansi5, 0.3, 0.45)),
        ),
    )
    .child(div().absolute().right(6).bottom(6).px(6).py(2).rounded(st.radius ?? 6).bg(c.accent).text_color(c.accent_foreground).text_size(10).child("Aa"));
  const personal = !entry.shipped;
  return v_flex()
    .id(id ?? `st-t-${entry.name}`)
    .gap(6)
    .p(6)
    .rounded(10)
    .when(selected, (el) => el.border_2().border_color(t.accent))
    .when(!selected, (el) => el.border_1().border_color(t.border))
    .cursor_pointer()
    .hover((s) => s.bg(t.raised))
    .on_click(onClick)
    .child(thumb)
    .child(
      h_flex().w(size.w).min_w(0).gap(5).items_center()
        .child(div().flex_1().min_w(0).truncate().tooltip(label).text_size(12.5).text_color(selected ? t.foreground : t.muted).child(label))
        .when(personal, (el) => el.child(div().flex_none().px(5).py(1).rounded(5).bg(t.hover).text_size(10.5).text_color(t.dim).child(tr("settings.theme_personal"))))
        .when(personal && Boolean(onEdit), (el) =>
          el.child(
            div().id(`st-te-${entry.name}`).flex_none().p(3).rounded(5).cursor_pointer().tooltip(tr("settings.theme_edit"))
              .hover((s) => s.bg(t.hover))
              .on_click((e, cx) => { cx.stop_propagation(); onEdit(e, cx); })
              .child(svg(I("pencil")).size(13).text_color(t.muted)),
          ))
        .when(selected, (el) => el.child(svg(I("check")).size(12).flex_none().text_color(t.accent))),
    );
}

// Files ---------------------------------------------------------------------

/** Names inside `dir` (files, or folders with `dirs`). Falls back to `find` where fs/promises can't read it. */
export async function listFiles(dir, { dirs = false } = {}) {
  try {
    const entries = await fs.readdir(dir, { withFileTypes: true });
    return entries.filter((e) => e.isDirectory() === dirs).map((e) => e.name).sort();
  } catch {}
  if (windows()) return [];
  try {
    const r = await exec("find", [dir, "-mindepth", "1", "-maxdepth", "1", "-type", dirs ? "d" : "f"]);
    if (r?.code !== 0) return [];
    return r.stdout.split("\n").filter(Boolean).map((p) => p.slice(p.lastIndexOf("/") + 1)).sort();
  } catch {
    return [];
  }
}

export async function readFileText(path) {
  try {
    const text = await readTextFile(path);
    if (typeof text === "string") return text;
  } catch {}
  if (windows()) return null;
  try {
    const r = await exec("cat", [path]);
    return r?.code === 0 ? r.stdout : null;
  } catch {
    return null;
  }
}

async function exists(path) {
  try { return await fs.exists(path); } catch { return false; }
}

/** A theme name that doesn't exist yet: `base`, `base-2`, `base-3`... */
export function freeName(base, taken) {
  let n = base || "theme";
  for (let i = 2; taken.includes(n); i++) n = `${base}-${i}`;
  return n;
}

// Importing -----------------------------------------------------------------

export class ImportPanel {
  constructor(host) {
    this.host = host;
    this.sources = null; // [{ id, label, files: [{ name, path }] }]
    this.extra = []; // folders picked by hand
    this.filter = "";
    this.input = InputState.new({ placeholder: tr("settings.import_filter") });
    this.input.on("change", (_e, cx) => { this.filter = this.input.value().toLowerCase(); cx.notify(); });
  }

  async scan() {
    const h = home();
    const sources = [];
    const add = (id, label, files) => files.length && sources.push({ id, label, files });
    const plain = async (dir, keep = () => true) => (await listFiles(dir)).filter(keep).map((name) => ({ name, path: `${dir}/${name}` }));
    const platform = window()?.platform;
    const ghostty = [`${h}/.config/ghostty/themes`];
    if (platform === "macos") ghostty.push("/Applications/Ghostty.app/Contents/Resources/ghostty/themes");
    if (platform === "linux") ghostty.push("/usr/share/ghostty/themes");
    for (const dir of ghostty) add(`ghostty:${dir}`, `Ghostty · ${dir}`, await plain(dir));
    const yaml = (n) => /\.ya?ml$/i.test(n);
    const warp = `${h}/.warp/themes`;
    const warpFiles = await plain(warp, yaml);
    for (const sub of await listFiles(warp, { dirs: true })) warpFiles.push(...(await plain(`${warp}/${sub}`, yaml)));
    add("warp", `Warp · ${warp}`, warpFiles);
    for (const [label, root] of [["VS Code", `${h}/.vscode/extensions`], ["Cursor", `${h}/.cursor/extensions`]]) {
      const files = [];
      for (const ext of await listFiles(root, { dirs: true })) files.push(...(await plain(`${root}/${ext}/themes`, (n) => /\.json$/i.test(n))));
      add(label, `${label} · ${root}`, files);
    }
    for (const dir of this.extra) add(`folder:${dir}`, dir, await plain(dir));
    this.sources = sources;
  }

  async pick(cx) {
    const dir = await pickFolder();
    if (!dir) return;
    if (!this.extra.includes(dir)) this.extra.push(dir);
    await this.scan();
    cx.notify();
  }

  /** Reads one file and installs it as `themes/<name>.json`, then applies it. */
  async importFile(file) {
    try {
      const text = await readFileText(file.path);
      if (text === null) throw new Error("settings.import_unreadable");
      const theme = PARSERS[detect(file.name)](text);
      const base = slug(file.name.replace(/\.[^.]*$/, ""));
      const taken = themes().map((x) => x.name);
      let name = freeName(base, taken);
      while (await exists(`${appHome()}/themes/${name}.json`)) name = freeName(name.replace(/-\d+$/, ""), [...taken, name]);
      await writeTextFile(`${appHome()}/themes/${name}.json`, toThemeFile(theme), null);
      run("app.theme", { name });
      notify({ title: tr("settings.import_done", { name }) });
    } catch (e) {
      notify({ level: "error", title: tr("settings.import_failed"), body: tr(e.message, e.vars ?? {}) });
    }
  }

  render(t, cx) {
    const h = this.host;
    if (this.sources === null && !this.scanning) {
      this.scanning = true;
      cx.spawn(async (cx) => { await this.scan(); cx.notify(); });
    }
    const q = this.filter;
    const shown = (this.sources ?? []).map((s) => ({ ...s, files: s.files.filter((f) => !q || f.name.toLowerCase().includes(q)) })).filter((s) => s.files.length);
    return v_flex().w_full().gap(10).p(12).rounded(10).border_1().border_color(t.border).bg(t.surface)
      .child(
        h_flex().gap(8).items_center()
          .child(Input.new(this.input).flex_1().min_w(0).h(30).px(8).rounded(7).border_1().border_color(t.border).bg(t.background).text_size(12.5).text_color(t.foreground))
          .child(h.button(t, "st-imp-pick", tr("settings.import_pick_folder"), (_e, cx) => cx.spawn(async (cx) => this.pick(cx))))
          .child(h.button(t, "st-imp-close", tr("settings.cancel"), (_e, cx) => { h.importing = false; cx.notify(); })),
      )
      .when(this.sources === null, (el) => el.child(div().text_size(12.5).text_color(t.dim).child("…")))
      .when(this.sources !== null && shown.length === 0, (el) => el.child(div().text_size(12.5).text_color(t.dim).child(tr("settings.import_nothing_found"))))
      .children(
        shown.map((s, i) =>
          v_flex().gap(4)
            .child(div().text_size(12).font_medium().text_color(t.muted).child(s.label))
            .child(
              h_flex().flex_wrap().gap(6).children(
                s.files.slice(0, 60).map((f, j) =>
                  div().id(`st-imp-${i}-${j}`).px(8).py(4).rounded(6).border_1().border_color(t.border).bg(t.raised).cursor_pointer()
                    .text_size(12.5).text_color(t.foreground).hover((x) => x.bg(t.hover))
                    .on_click((_e, cx) => cx.spawn(async (cx) => { await this.importFile(f); h.importing = false; cx.notify(); }))
                    .child(f.name.replace(/\.[^.]*$/, "")),
                ),
              ),
            ),
        ),
      );
  }
}

// Editor --------------------------------------------------------------------

const UI_COLORS = ["accent", "chrome", "canvas", "surface", "border", "foreground", "muted"];
const ANSI_LABELS = ["black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"];
// [key, min, max, default when first set]
const NUMBERS = [
  ["radius", 0, 24, 6, "settings.style_radius"],
  ["pane_radius", 0, 24, 6, "settings.style_pane_radius"],
  ["pane_gap", 0, 32, 0, "settings.style_pane_gap"],
  ["border_width", 0, 4, 1, "settings.style_border_width"],
  ["font_size", 6, 72, 14, "settings.style_font_size"],
];

export class ThemeEditor {
  /** `fromEntry`: a themes() item (a new theme) or null; `file`: { name, text } to edit one. */
  constructor(host, { entry = null, file = null } = {}) {
    this.host = host;
    this.confirmDelete = false;
    this.inputs = {};
    if (file) {
      this.original = file.text;
      this.originalName = file.name;
      this.name = file.name;
      const d = JSON.parse(file.text);
      this.draft = { background: d.background, foreground: d.foreground, selection: d.selection, cursor: d.cursor ?? "", ansi: [...d.ansi], colors: { ...(d.colors ?? {}) }, style: { ...(d.style ?? {}) } };
    } else {
      this.original = null;
      this.originalName = null;
      const c = entry.colors;
      this.name = freeName(`${entry.name}-mio`, themes().map((x) => x.name));
      this.draft = {
        background: c.background, foreground: c.terminal_foreground, selection: c.selection, cursor: c.cursor ?? "",
        ansi: [...Array(16)].map((_, i) => c[`ansi${i}`]), colors: {}, style: { ...(entry.style ?? {}) },
      };
    }
    this.nameInput = InputState.new({ value: this.name });
    this.nameInput.on("change", (_e, cx) => { this.name = this.nameInput.value().trim(); cx.notify(); });
  }

  input(key, value, apply, placeholder = "#rrggbb") {
    if (!this.inputs[key]) {
      const s = InputState.new({ value: value ?? "", placeholder });
      s.on("change", (_e, cx) => { apply(s.value().trim()); cx.notify(); });
      this.inputs[key] = s;
    }
    return this.inputs[key];
  }

  /** Every problem that keeps the draft from being saved. */
  invalid() {
    const d = this.draft;
    const bad = [];
    if (!/^[a-z0-9-]+$/.test(this.name)) bad.push("name");
    for (const k of ["background", "foreground", "selection"]) if (!HEX6.test(d[k] ?? "")) bad.push(k);
    if (d.cursor && !HEX6.test(d.cursor)) bad.push("cursor");
    d.ansi.forEach((v, i) => { if (!HEX6.test(v ?? "")) bad.push(`ansi${i}`); });
    for (const [k, v] of Object.entries(d.colors)) if (v && !HEX.test(v)) bad.push(`colors.${k}`);
    return bad;
  }

  /** The file's JSON: empty values dropped. */
  result() {
    const d = this.draft;
    const out = { background: d.background, foreground: d.foreground, selection: d.selection };
    if (d.cursor) out.cursor = d.cursor;
    out.ansi = d.ansi;
    const colors = Object.fromEntries(Object.entries(d.colors).filter(([, v]) => v));
    if (Object.keys(colors).length) out.colors = colors;
    const style = Object.fromEntries(Object.entries(d.style).filter(([, v]) => v !== undefined && v !== ""));
    if (Object.keys(style).length) out.style = style;
    return out;
  }

  /** The draft as a themes() entry, for the preview. */
  entry(t) {
    const d = this.draft;
    const ok = (v) => (HEX.test(v ?? "") ? v : null);
    const colors = { ...t };
    const set = (k, v) => { if (ok(v)) colors[k] = v; };
    set("background", d.background);
    set("terminal_foreground", d.foreground);
    set("selection", d.selection);
    d.ansi.forEach((v, i) => set(`ansi${i}`, v));
    for (const [k, v] of Object.entries(d.colors)) set(k, v);
    return { name: this.name || "theme", colors, style: d.style, shipped: false };
  }

  async save(cx) {
    if (this.invalid().length) return;
    const taken = themes().map((x) => x.name);
    if (this.name !== this.originalName && taken.includes(this.name)) {
      notify({ level: "error", title: tr("settings.theme_name_taken") });
      return;
    }
    try {
      const expected = this.name === this.originalName ? this.original : null;
      await writeTextFile(`${appHome()}/themes/${this.name}.json`, toThemeFile(this.result()), expected);
      if (this.originalName && this.name !== this.originalName) await fs.unlink(`${appHome()}/themes/${this.originalName}.json`);
      run("app.theme", { name: this.name });
      this.host.editing = null;
    } catch (e) {
      notify({ level: "error", title: tr("settings.theme_save_failed"), body: String(e?.message ?? e) });
    }
    cx.notify();
  }

  async remove(cx) {
    if (!this.originalName) return;
    try {
      await fs.unlink(`${appHome()}/themes/${this.originalName}.json`);
      const s = settings();
      if (s.theme === this.originalName) {
        const light = themes().find((x) => x.name === this.originalName)?.light;
        run("app.theme", { name: light ? "paper" : "graphite" });
      }
      this.host.editing = null;
    } catch (e) {
      notify({ level: "error", title: tr("settings.theme_save_failed"), body: String(e?.message ?? e) });
    }
    cx.notify();
  }

  swatchRow(t, key, label, value, apply, bad) {
    const input = this.input(key, value, apply);
    return h_flex().gap(8).items_center()
      .child(div().w(78).flex_none().text_size(12).text_color(t.muted).child(label))
      .child(Input.new(input).w(112).h(28).px(8).rounded(6).border_1().border_color(bad ? t.blocked : t.border).bg(t.background).text_size(12.5).text_color(t.foreground))
      .child(div().w(16).h(16).flex_none().rounded(4).border_1().border_color(t.border).bg(HEX.test(value ?? "") ? value : "#00000000"));
  }

  render(t, cx) {
    const h = this.host;
    const d = this.draft;
    const bad = new Set(this.invalid());
    const heading = (text) => div().pt(10).pb(4).text_size(12).font_semibold().text_color(t.dim).child(text.toUpperCase());
    const term = (k, label) => this.swatchRow(t, k, label, d[k], (v) => (d[k] = v), bad.has(k));
    const ansi = (row) =>
      v_flex().gap(6).children(
        ANSI_LABELS.map((n, j) => {
          const i = row * 8 + j;
          return this.swatchRow(t, `ansi${i}`, n, d.ansi[i], (v) => (d.ansi[i] = v), bad.has(`ansi${i}`));
        }),
      );
    const ui = (k) => this.swatchRow(t, `colors.${k}`, k, d.colors[k] ?? "", (v) => (v ? (d.colors[k] = v) : delete d.colors[k]), bad.has(`colors.${k}`));
    const number = ([k, min, max, def, label]) => {
      const v = d.style[k];
      const set = (n) => { if (n < min) delete d.style[k]; else d.style[k] = Math.min(max, n); };
      return h.row(t, tr(label), null, h.stepper(t, `st-te-n-${k}`, v === undefined ? "—" : `${v}`,
        (_e, cx) => { set(v === undefined ? def : v - 1); cx.notify(); },
        (_e, cx) => { set(v === undefined ? def : v + 1); cx.notify(); }));
    };
    const text = (k, label) => {
      const input = this.input(`style.${k}`, d.style[k] ?? "", (v) => (v ? (d.style[k] = v) : delete d.style[k]), "");
      return h.row(t, tr(label), null, Input.new(input).w(200).h(28).px(8).rounded(6).border_1().border_color(t.border).bg(t.background).text_size(12.5).text_color(t.foreground));
    };
    const personal = this.originalName !== null;
    return v_flex().w_full().gap(6).p(14).rounded(10).border_1().border_color(t.border).bg(t.surface)
      .child(
        h_flex().gap(16).items_start()
          .child(
            v_flex().flex_1().min_w(0).gap(6)
              .child(h_flex().gap(8).items_center()
                .child(div().w(78).flex_none().text_size(12).text_color(t.muted).child(tr("settings.theme_name")))
                .child(Input.new(this.nameInput).w(200).h(28).px(8).rounded(6).border_1().border_color(bad.has("name") ? t.blocked : t.border).bg(t.background).text_size(12.5).text_color(t.foreground)))
              .child(heading(tr("settings.theme_terminal_colors")))
              .child(h_flex().gap(24).items_start().child(v_flex().gap(6).child(term("background", "background")).child(term("foreground", "foreground")).child(term("selection", "selection")).child(term("cursor", "cursor"))))
              .child(h_flex().gap(24).items_start().child(ansi(0)).child(ansi(1))),
          )
          .child(
            v_flex().gap(8).flex_none()
              .child(themeCard(t, this.entry(t), { selected: false, onClick: () => {}, size: { w: 260, h: 160 }, id: "st-te-preview" })),
          ),
      )
      .child(heading(tr("settings.theme_ui_colors")))
      .child(h_flex().flex_wrap().gap(24).children(UI_COLORS.map((k) => ui(k))))
      .child(heading(tr("settings.theme_style")))
      .children(NUMBERS.map(number))
      .child(h.row(t, tr("settings.style_shadow"), null, h.segmented(t, "te-sh", [[undefined, "—"], ["none", "none"], ["sm", "sm"], ["md", "md"], ["lg", "lg"]], d.style.shadow,
        (v, cx) => { if (v === undefined) delete d.style.shadow; else d.style.shadow = v; cx.notify(); })))
      .child(text("font", "settings.style_font"))
      .child(text("ui_font", "settings.style_ui_font"))
      .child(
        h_flex().gap(8).pt(10)
          .child(h.button(t, "st-te-save", tr("settings.theme_save"), (_e, cx) => { if (!bad.size) cx.spawn(async (cx) => this.save(cx)); }).opacity(bad.size ? 0.5 : 1))
          .child(h.button(t, "st-te-cancel", tr("settings.cancel"), (_e, cx) => { h.editing = null; cx.notify(); }))
          .when(personal, (el) =>
            el.child(h.button(t, "st-te-delete", this.confirmDelete ? tr("settings.confirm_delete") : tr("settings.theme_delete"), (_e, cx) => {
              if (!this.confirmDelete) { this.confirmDelete = true; cx.notify(); } else cx.spawn(async (cx) => this.remove(cx));
            }))),
      );
  }
}

/** Reads `themes/<name>.json` for editing, or null. */
export async function loadThemeFile(name) {
  const text = await readTextFile(`${appHome()}/themes/${name}.json`);
  try {
    if (text) JSON.parse(text);
  } catch {
    return null;
  }
  return text ? { name, text } : null;
}
