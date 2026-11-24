// The frame's command palette: everything you can do, by name, with its key.
// Places (spaces, tabs) stay in the picker. Fuzzy: your letters in
// order, word starts and runs rank higher. Empty: recent first, then every
// group. Choices are entries of their own ("Layout: Zen"); commands that need
// a name ask for it. Enter runs, ↑/↓ move, Esc closes.
// The app's own mod: rewritten from the binary on every start.

import { View, div, svg } from "gpui-kit";
import { v_flex, h_flex, InputState, Input } from "gpui-base";
import { commands, layouts, keys, run, theme, props, closeOverlay, openOverlay, shared, window as win, tr } from "herdr";

// The list keeps one height while you move: rows and group titles have fixed
// heights, and the window shows as many as fit in LIST_H.
// 10 rows: with the search line it fits the popup (at most 85% of the
// window) down to ~580 px windows at zoom 1.3.
const SHOWN = 10;
const ROW_H = 32;
const HEAD_H = 26;
const LIST_H = SHOWN * ROW_H;
const GROUPS = ["palette.recent", "palette.tab", "palette.pane", "palette.space", "palette.layout", "palette.terminal", "palette.agent", "palette.window", "palette.app"];

// The palette's names for commands: [command, args, group, title, keywords].
const NAMED = [
  ["tab.new", {}, "palette.tab", "palette.new_tab", "tab agent new task pestaña agente nueva tarea"],
  ["mod.open", { name: "picker" }, "palette.tab", "palette.go_to_space_or_tab", "switch jump find cambiar saltar buscar"],
  ["file.goto", {}, "palette.tab", "palette.go_to_file", "file open find archivo abrir buscar"],
  ["mod.open", { name: "folders" }, "palette.tab", "folders.open", "folder directory open carpeta directorio abrir"],
  ["tab.rename", {}, "palette.tab", "palette.rename_tab", "tab renombrar"],
  ["tab.close", {}, "palette.tab", "palette.close_tab", "tab kill cerrar"],
  ["tab.next", {}, "palette.tab", "palette.next_tab", "tab siguiente"],
  ["tab.prev", {}, "palette.tab", "palette.previous_tab", "tab anterior"],
  ["pane.split", { direction: "right" }, "palette.pane", "palette.split_right", "vertical dividir"],
  ["pane.split", { direction: "down" }, "palette.pane", "palette.split_down", "horizontal dividir"],
  ["pane.close", {}, "palette.pane", "palette.close_pane", "kill cerrar"],
  ["pane.zoom", {}, "palette.pane", "palette.zoom_pane", "maximize fullscreen maximizar pantalla completa"],
  ...["left", "right", "up", "down"].map((d) => ["pane.go", { direction: d }, "palette.pane", () => tr("palette.focus_pane_direction", { direction: tr("palette." + d) }), "move go mover ir enfocar"]),
  ...["left", "right", "up", "down"].map((d) => ["pane.swap", { direction: d }, "palette.pane", () => tr("palette.swap_pane_direction", { direction: tr("palette." + d) }), "move mover intercambiar"]),
  ["space.new", {}, "palette.space", "palette.new_space", "workspace nuevo"],
  ["space.rename", {}, "palette.space", "palette.rename_space", "workspace renombrar"],
  ["space.close", {}, "palette.space", "palette.close_space", "workspace kill cerrar"],
  ["space.next", {}, "palette.space", "palette.next_space", "workspace siguiente"],
  ["space.prev", {}, "palette.space", "palette.previous_space", "workspace anterior"],
  ["layout.switch", {}, "palette.layout", "palette.next_layout", "look switch apariencia cambiar siguiente"],
  ["layout.reset", {}, "palette.layout", "palette.reset_layout", "new version default restore herdr's restablecer predeterminado"],
  ["layout.clone", {}, "palette.layout", "palette.duplicate_layout", "clone copy customize duplicar copiar personalizar"],
  ["mod.clone", {}, "palette.layout", "palette.duplicate_mod", "clone copy customize duplicar copiar personalizar"],
  ["terminal.open", {}, "palette.terminal", "palette.new_terminal", "shell nueva"],
  ["terminal.copy", {}, "palette.terminal", "palette.copy", "clipboard copiar portapapeles"],
  ["terminal.paste", {}, "palette.terminal", "palette.paste", "clipboard pegar portapapeles"],
  ["agent.next", {}, "palette.agent", "palette.next_agent", "siguiente"],
  ["agent.prev", {}, "palette.agent", "palette.previous_agent", "anterior"],
  ["window.minimize", {}, "palette.window", "palette.minimize_window", "minimizar"],
  ["window.maximize", {}, "palette.window", "palette.maximize_window", "restore maximizar restaurar"],
  ["window.close", {}, "palette.window", "palette.close_window", "quit cerrar salir"],
  ["app.settings", {}, "palette.app", "palette.settings", "preferences options config look font theme sound ajustes preferencias opciones apariencia fuente tema sonido"],
  ["app.help", {}, "palette.app", "palette.keys", "keybindings shortcuts help atajos ayuda teclado"],
  ["app.welcome", {}, "palette.app", "palette.welcome", "intro start onboarding inicio introducción"],
  ["app.whats_new", {}, "palette.app", "palette.what_s_new", "release changes news update cambios actualización"],
  ["app.reset", {}, "palette.app", "palette.reset_to_the_default_screen", "escape hatch broken restore reset restablecer roto"],
  ["app.zoom_in", {}, "palette.app", "palette.zoom_in", "bigger font size aumentar fuente tamaño"],
  ["app.zoom_out", {}, "palette.app", "palette.zoom_out", "smaller font size reducir fuente tamaño"],
  ["app.zoom_reset", {}, "palette.app", "palette.reset_zoom", "font size restablecer fuente tamaño"],
  ["app.quit", {}, "palette.app", "palette.quit_herdr", "exit salir cerrar"],
  ...["graphite", "ink", "moss", "atom-one-dark"].map((n) => ["app.theme", { name: n }, "palette.app", () => tr("palette.theme_name", { name: n.replace(/-/g, " ") }), "colors look colores apariencia"]),
];

// Commands that act on something you point at (ids, terminals): not for a list.
const HIDDEN = new Set([
  "space.move", "tab.move", "space.select", "tab.select", "pane.focus", "terminal.keep",
  "terminal.park", "terminal.kill", "terminal.show", "agent.focus", "layout.set", "layout.toggle",
  "layout.show", "layout.hide", "layout.resize", "layout.select", "mod.open", "overlay.close",
  "tab.settle", "tab.unsettle",
  "app.palette", "app.hide", "app.hide_others", "app.show_all", "app.theme", "app.settings",
]);

function titleOf(name) {
  const [noun, verb] = name.split(".");
  return { group: noun.charAt(0).toUpperCase() + noun.slice(1), title: `${verb.replace(/_/g, " ")} ${noun}`.replace(/^./, (c) => c.toUpperCase()) };
}

// Every entry for the state we're in now.
function entries() {
  const known = new Set(commands().map((c) => c.name));
  const mac = win()?.platform === "macos";
  const list = NAMED.filter(([n]) => known.has(n))
    .filter(([n]) => !(mac && n.startsWith("window.")))
    .map(([name, args, group, title, words]) => ({ name, args, group: tr(group), title: typeof title === "function" ? title() : tr(title), words }));
  for (const l of layouts()) {
    const t = { main: "Sunna", terminals: "Terminals", win98: "Herdr 98" }[l] ?? l.charAt(0).toUpperCase() + l.slice(1);
    list.push({ name: "layout.switch", args: { name: l }, group: tr("palette.layout"), title: tr("palette.layout_name", { name: t }), words: `look layout diseño ${l}` });
  }
  // Anything new in the app shows up too, until it gets a better name here.
  const named = new Set(NAMED.map(([n]) => n));
  for (const c of commands()) {
    if (named.has(c.name) || HIDDEN.has(c.name) || c.args.some((a) => a.required && a.name !== "name")) continue;
    list.push({ name: c.name, args: {}, ...titleOf(c.name), words: c.about });
  }
  return list.map((e) => ({ ...e, id: `${e.name}:${JSON.stringify(e.args)}` }));
}

// The key bound to an entry: `name` or `name:value` in keys.json.
function keyOf(e, bound) {
  const v = Object.values(e.args)[0];
  const want = v === undefined ? e.name : `${e.name}:${v}`;
  const k = bound.find((b) => b.section === "global" && b.action === want);
  return k ? k.keys : null;
}

// "ctrl-shift-p" -> ["ctrl", "shift", "p"]; "ctrl--" -> ["ctrl", "-"].
function keyParts(k) {
  return k.endsWith("--") ? [...k.slice(0, -2).split("-"), "-"] : k.split("-");
}

// Fuzzy score of `q` in `text`, with the matched positions; null: no match.
function fuzzy(q, text) {
  const t = text.toLowerCase();
  let score = 0;
  let at = 0;
  let run = 0;
  const hits = [];
  for (const ch of q) {
    const i = t.indexOf(ch, at);
    if (i < 0) return null;
    const start = i === 0 || /[\s:.\-_/]/.test(t[i - 1]);
    run = i === at && hits.length ? run + 1 : 0;
    score += 1 + (start ? 6 : 0) + run * 3 - Math.min(i - at, 6) * 0.3;
    hits.push(i);
    at = i + 1;
  }
  return { score: score - text.length * 0.02, hits };
}

export default class Palette extends View {
  init(_props, cx) {
    this.query = "";
    this.input = InputState.new({ placeholder: tr("palette.type_a_command") });
    this.input.on("change", (_e, cx) => { this.query = this.input.value(); this.at = 0; cx.notify(); });
    this.input.on("submit", () => this.go(this.rows()[this.at]?.e));
    this.input.focus();
    cx.timer.after(600, () => this.input.focus());
    this.at = 0;
    // How far the list is scrolled, px.
    this.scroll = 0;
    this.recent = JSON.parse(localStorage.getItem("recent") ?? "[]");
    this.handle = cx.focus_handle();
  }

  close() {
    closeOverlay(props().overlay);
  }

  go(e) {
    if (!e) return;
    this.recent = [e.id, ...this.recent.filter((r) => r !== e.id)].slice(0, 20);
    localStorage.setItem("recent", JSON.stringify(this.recent));
    this.close();
    if (e.name === "mod.clone") {
      openOverlay({ mod: "prompt", kind: "dialog", width: 380, props: {
        title: tr("palette.duplicate_mod"), hint: tr("palette.mod_to_copy_creates_name_copy"),
        command: { name: "mod.clone", args: {} },
      } });
    } else {
      run(e.name, e.name === "layout.clone" ? { name: shared("layout") ?? "main" } : e.args);
    }
  }

  // The rows to show: grouped when empty, ranked while typing.
  rows() {
    const all = entries();
    const q = this.query.trim().toLowerCase().replace(/\s+/g, "");
    if (!q) {
      const recent = this.recent.map((id) => all.find((e) => e.id === id)).filter(Boolean).slice(0, 5);
      const groups = GROUPS.map((g) => tr(g));
      const rest = groups.slice(1).flatMap((g) => all.filter((e) => e.group === g && !recent.includes(e)));
      const others = all.filter((e) => !groups.includes(e.group) && !recent.includes(e));
      return [...recent.map((e) => ({ e, head: tr("palette.recent") })), ...[...rest, ...others].map((e) => ({ e, head: e.group }))];
    }
    const rank = (e) => this.recent.indexOf(e.id);
    return all
      .map((e) => {
        const title = fuzzy(q, e.title);
        const more = fuzzy(q, `${e.group} ${e.title} ${e.name} ${e.words}`);
        const best = title ?? (more && { score: more.score - 4, hits: [] });
        return best && { e, score: best.score, hits: best.hits };
      })
      .filter(Boolean)
      .sort((a, b) => b.score - a.score || (rank(a.e) < 0 ? 99 : rank(a.e)) - (rank(b.e) < 0 ? 99 : rank(b.e)));
  }

  title(t, text, hits, on) {
    if (!hits?.length) return div().text_color(on ? t.foreground : t.muted).child(text);
    const parts = [];
    let cur = "";
    let lit = false;
    [...text].forEach((ch, i) => {
      const h = hits.includes(i);
      if (h !== lit && cur) {
        parts.push([cur, lit]);
        cur = "";
      }
      lit = h;
      cur += ch;
    });
    if (cur) parts.push([cur, lit]);
    return h_flex().children(parts.map(([s, h]) => div().text_color(h ? t.accent : on ? t.foreground : t.muted).when(h, (d) => d.font_semibold()).child(s.replace(/ /g, "\u00a0"))));
  }

  render() {
    const t = theme();
    const rows = this.rows();
    const bound = keys();
    this.at = Math.min(this.at, Math.max(0, rows.length - 1));
    // One continuous strip: each row (and the group title before it) at a
    // fixed y. Scrolling moves the strip in pixels, only as far as it takes to
    // keep the selected row (and its group's title, if it opens one) in view.
    const tops = [];
    let y = 0;
    rows.forEach((r, i) => {
      const head = !this.query && (i === 0 || r.head !== rows[i - 1].head);
      tops.push({ head, title: y, row: y + (head ? HEAD_H : 0) });
      y += (head ? HEAD_H : 0) + ROW_H;
    });
    const total = y;
    const fixed = total > LIST_H;
    this.maxScroll = Math.max(0, total - LIST_H);
    const sel = tops[this.at];
    if (sel) {
      const from = sel.head ? sel.title : sel.row;
      if (from < this.scroll) this.scroll = from;
      if (sel.row + ROW_H > this.scroll + LIST_H) this.scroll = sel.row + ROW_H - LIST_H;
    }
    this.scroll = Math.max(0, Math.min(this.scroll, this.maxScroll));
    const key = (k) =>
      h_flex().gap(3).children(keyParts(k).map((p) => div().px(5).rounded(4).border_1().border_color(t.border).bg(t.raised).text_size(11).text_color(t.dim).child(p)));
    // In a scrolling list each item is placed at its edges from the model
    // above, not stacked: GPUI rounds every height to whole pixels, and a
    // stack of rounded heights drifts away from the model (19 px at zoom 1.3
    // by row 40, cutting the selection off). Edges from the model can't add up.
    const place = (el, start, end) =>
      fixed
        ? el.absolute().left(0).right(0).top(start - this.scroll).bottom(LIST_H - (end - this.scroll))
        : el.flex_none().h(end - start);
    const visible = (start, end) => !fixed || (end > this.scroll && start < this.scroll + LIST_H);
    const list = [];
    rows.forEach((r, i) => {
      const { head, title, row } = tops[i];
      if (head && visible(title, row)) list.push(place(h_flex(), title, row).px(14).pb(4).items_end().text_size(11).font_semibold().text_color(t.dim).child(r.head));
      if (!visible(row, row + ROW_H)) return;
      const on = i === this.at;
      const k = keyOf(r.e, bound);
      list.push(
        place(h_flex(), row, row + ROW_H)
          .id(`pl-${i}`)
          .mx(6)
          .px(10)
          .gap(10)
          .items_center()
          .rounded(7)
          .cursor_pointer()
          .text_size(12.8)
          .when(on, (el) => el.bg(t.selected))
          .hover((s) => s.bg(t.hover))
          .on_click(() => this.go(r.e))
          .child(div().flex_1().min_w(0).child(this.title(t, r.e.title, r.hits, on)))
          .when(Boolean(this.query), (el) => el.child(div().text_size(11.5).text_color(t.dim).child(r.e.group)))
          .when(Boolean(k), (el) => el.child(key(k))),
      );
    });
    return v_flex()
      .id("palette")
      .track_focus(this.handle)
      .w_full()
      .min_h(0)
      .p(18)
      .gap(10)
      .bg(t.popover)
      .on_key_down((e, cx) => {
        const k = e.key;
        if (k === "escape") return this.close();
        // Stops at the ends: wrapping jumps out of sight.
        if (k === "down") this.at = Math.min(this.at + 1, rows.length - 1);
        else if (k === "up") this.at = Math.max(this.at - 1, 0);
        else if (k === "pagedown") this.at = Math.min(this.at + SHOWN, rows.length - 1);
        else if (k === "pageup") this.at = Math.max(this.at - SHOWN, 0);
        else return;
        cx.stop_propagation();
        cx.notify();
      })
      .child(
        h_flex()
          .h(36)
          .flex_none()
          .px(10)
          .gap(10)
          .items_center()
          .rounded(9)
          .border_1()
          .border_color(t.accent)
          .bg(t.background)
          .child(svg(win()?.platform === "macos" ? "frame/palette/command.svg" : "mods/workspace-chrome/search.svg").size(16).text_color(t.dim))
          .child(Input.new(this.input).flex_1().h_full().text_size(12.8).text_color(t.foreground))
          .child(div().text_size(11).text_color(t.dim).child("Esc")),
      )
      .child(
        rows.length
          ? div()
              .id("palette-rows")
              .relative()
              .mb(6)
              .when(fixed, (el) => el.h(LIST_H).overflow_hidden())
              .on_scroll_wheel((e, cx) => {
                // The event bridge converts both wheel and touchpad deltas to pixels.
                this.scroll = Math.max(0, Math.min(this.scroll - e.delta.y, this.maxScroll));
                // The selection stays on screen, so the next frame doesn't scroll back to it.
                // A row that opens a group is shown with its title, so the title counts.
                while (this.at < rows.length - 1 && (tops[this.at].head ? tops[this.at].title : tops[this.at].row) < this.scroll) this.at++;
                while (this.at > 0 && tops[this.at].row + ROW_H > this.scroll + LIST_H) this.at--;
                cx.notify();
              })
              .children(fixed ? list : [v_flex().children(list)])
          : div().px(16).py(14).text_size(13).text_color(t.dim).child(tr("palette.nothing_matches_query_places_are", { query: this.query })),
      );
  }
}
