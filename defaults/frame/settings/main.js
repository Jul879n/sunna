// App-owned settings mod. Writes changes immediately; no Save step.

import { View, div, svg, image } from "gpui-kit";
import { v_flex, h_flex, Scrollbar, InputState, Input, Popup } from "gpui-base";
import { hc, run, layout, keys, knownAgents, pickFolder, checking, theme, themes, fonts, updates, exec, telemetry, settings, layouts, shared, appHome, readTextFile, writeTextFile, playSound, props, closeOverlay, notify, window, tr } from "herdr";

import { themeCard, ImportPanel, ThemeEditor, loadThemeFile } from "./themes.js";
import { ModsSection } from "./mods.js";
import { SidebarEditor } from "./sidebar.js";

const I = (name) => `frame/settings/${name}.svg`;

const SECTIONS = [
  { id: "look", title: "settings.look", icon: "palette" },
  { id: "layout", title: "settings.layout", icon: "layout" },
  { id: "mods", title: "settings.mods", icon: "puzzle" },
  { id: "general", title: "settings.general", icon: "settings" },
  { id: "agents", title: "settings.agents", icon: "terminal" },
  { id: "keyboard", title: "settings.keyboard", icon: "keyboard" },
  { id: "machines", title: "settings.machines", icon: "server" },
  { id: "alerts", title: "settings.alerts", icon: "bell" },
  { id: "updates", title: "settings.updates", icon: "refresh" },
  { id: "privacy", title: "settings.privacy", icon: "shield" },
];

// What Herdr's own layout blocks are called to a person; other layouts show their title or id.
const LAYOUT_NAMES = {
  sidebar: "settings.layout_sidebar", brand: "settings.layout_brand", "sidebar-lists": "settings.layout_lists", spaces: "settings.layout_spaces",
  agents: "settings.layout_agents", "sidebar-footer": "settings.layout_footer", work: "settings.layout_work", tabs: "settings.layout_tabs",
  "work-body": "settings.layout_body", main: "settings.layout_terminals", panel: "settings.layout_panel", status: "settings.layout_status",
};

const SHIPPED_LAYOUTS = ["main", "terminals", "win98"];

const LOOKS = { main: "Sunna", terminals: "settings.terminals", win98: "Herdr 98" };
const APPEARANCES = [
  [null, "settings.system"],
  ["light", "settings.light"],
  ["dark", "settings.dark"],
];

// What the app uses when settings.json names no font (settings.rs).
const DEFAULT_FONT = "JetBrains Mono";
const DEFAULT_UI_FONT = "Geist";

export default class Settings extends View {
  init(_props, cx) {
    const asked = props()?.section;
    this.section = SECTIONS.some((s) => s.id === asked) ? asked : "look";
    this.picking = null; // "font" | "ui_font" while its list is open
    this.modsSection = new ModsSection(this);
    this.sidebar = new SidebarEditor(this, SHIPPED_LAYOUTS);
    this.importing = false; // the theme import panel is open
    this.importPanel = null;
    this.editing = null; // a ThemeEditor while a theme of your own is edited
    this.filter = "";
    this.fontInput = InputState.new({ placeholder: tr("settings.type_to_search_fonts") });
    this.fontInput.on("change", (_e, cx) => { this.filter = this.fontInput.value(); cx.notify(); });
    this.renameInput = InputState.new();
    this.renameInput.on("focus", (_e, cx) => { this.renameFocused = true; cx.notify(); });
    this.renameInput.on("blur", (_e, cx) => { this.renameFocused = false; cx.notify(); });
    this.renameInput.on("change", (_e, cx) => { if (this.machineEdit?.action === "rename") this.machineEdit.name = this.renameInput.value(); cx.notify(); });
    this.renameInput.on("submit", (_e, cx) => {
      const edit = this.machineEdit;
      if (edit?.action === "rename") this.machineAction(edit.machine, "rename", cx, edit.name);
    });
    this.person = {};
    this.raw = null;
    this.previews = {}; // layout name -> its "preview" picture
    this.previewsOf = "";
    this.showData = false;
    this.machineList = null;
    this.machineLoading = false;
    this.machineError = null;
    this.machineBusy = {};
    this.machineResults = {};
    this.machineEdit = null;
    this.machineNotice = null;
    this.known = null; // knownAgents(): the agents this computer can start
    this.knownFailed = false;
    this.capturing = null; // { section, action, old }: the shortcut being changed
    this.runInputs = {};
    this.handle = cx.focus_handle();
    this.handle.focus();
    cx.spawn(async (cx) => {
      await this.load();
      if (this.section === "machines") await this.loadMachines();
      cx.notify();
    });
    // After tr("settings.check_now"): follow the look (the answer comes from the app), then
    // "Up to date" for a moment.
    cx.timer.every(300, (cx) => {
      if (!this.asked) return;
      if (updates().checking) this.asked = Date.now();
      else if (Date.now() - this.asked >= 3000) this.asked = null;
      cx.notify();
    });
    cx.timer.every(3000, (cx) => {
      if (this.section !== "machines") return;
      cx.spawn(async (cx) => {
        await this.loadMachines();
        cx.notify();
      });
    });
  }

  file() {
    return `${appHome()}/settings.json`;
  }

  // Each layout file may name a picture of itself (`"preview"` on its root).
  loadPreviews(names, cx) {
    this.previewsOf = names.join("\n");
    cx.spawn(async (cx) => {
      const found = {};
      for (const name of names) {
        try {
          const preview = JSON.parse((await readTextFile(`${appHome()}/layouts/${name}.json`)) ?? "{}").preview;
          if (typeof preview === "string" && preview) found[name] = preview;
        } catch {}
      }
      this.previews = found;
      cx.notify();
    });
  }

  // `broken`: settings.json doesn't parse (mid-edit), or isn't an object, so
  // nothing may be written over it.
  async load() {
    this.raw = await readTextFile(this.file());
    try {
      const person = this.raw ? JSON.parse(this.raw) : {};
      this.broken = !person || typeof person !== "object" || Array.isArray(person);
      this.person = this.broken ? {} : person;
    } catch {
      this.person = {};
      this.broken = true;
    }
  }

  // Changes settings.json the way an agent would: read, change, write back
  // only if nobody changed it meanwhile (else read again and redo). Says so
  // when it couldn't.
  change(edit, cx) {
    cx.spawn(async (cx) => {
      const failed = await this.save(edit);
      if (failed) notify({ ...failed, level: "error" });
      cx.notify();
    });
  }

  // The toast saying why `edit` wasn't written, or null once it is.
  async save(edit) {
    let error;
    for (let attempt = 0; attempt < 3; attempt++) {
      let next;
      try {
        await this.load();
        if (this.broken) return { title: tr("settings.settings_json_has_errors_fix_it") };
        next = JSON.parse(JSON.stringify(this.person));
        edit(next);
      } catch (e) {
        return { title: tr("settings.cannot_read_settings_json"), body: String(e?.message ?? e) };
      }
      try {
        await writeTextFile(this.file(), JSON.stringify(next, null, 2) + "\n", this.raw);
        this.person = next;
        return null;
      } catch (e) {
        error = e;
      }
    }
    return { title: tr("settings.not_saved_to_settings_json"), body: String(error?.message ?? error) };
  }

  // The same for another file of the app's home (programs.json, keys.json):
  // read, `edit` the parsed object, write back. The toast saying why not, or null.
  async saveFile(name, edit) {
    const path = `${appHome()}/${name}`;
    for (let attempt = 0; attempt < 3; attempt++) {
      let raw, next;
      try {
        raw = await readTextFile(path);
        const parsed = JSON.parse(raw ?? "{}");
        if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return { title: tr("settings.file_has_errors", { name }) };
        next = parsed;
        edit(next);
      } catch (e) {
        return { title: tr("settings.file_has_errors", { name }), body: String(e?.message ?? e) };
      }
      try {
        await writeTextFile(path, JSON.stringify(next, null, 2) + "\n", raw);
        return null;
      } catch (e) {
        if (attempt === 2) return { title: tr("settings.file_not_saved", { name }), body: String(e?.message ?? e) };
      }
    }
    return null;
  }

  changeFile(name, edit, cx) {
    cx.spawn(async (cx) => {
      const failed = await this.saveFile(name, edit);
      if (failed) notify({ ...failed, level: "error" });
      if (name === "programs.json") this.known = null;
      cx.notify();
    });
  }

  close() {
    closeOverlay(props().overlay);
  }

  // Pieces ---------------------------------------------------------------

  heading(t, text) {
    return div().pt(6).pb(8).text_size(12).font_semibold().text_color(t.dim).child(text.toUpperCase());
  }

  // One setting: what it is on the left, the control on the right.
  row(t, title, about, control) {
    return h_flex()
      .w_full()
      .gap(16)
      .py(12)
      .items_center()
      .border_b(1)
      .border_color(t.border)
      .child(
        v_flex()
          .flex_1()
          .min_w(0)
          .gap(3)
          .child(div().text_size(14).text_color(t.foreground).child(title))
          .when(Boolean(about), (el) => el.child(div().text_size(12.5).line_height("18px").text_color(t.dim).child(about))),
      )
      .child(div().flex_none().child(control));
  }

  button(t, id, label, onClick) {
    return div()
      .id(id)
      .px(12)
      .py(5)
      .rounded(7)
      .border_1()
      .border_color(t.border)
      .bg(t.raised)
      .cursor_pointer()
      .text_size(13)
      .text_color(t.foreground)
      .hover((s) => s.bg(t.hover))
      .on_click(onClick)
      .child(label);
  }

  // − value +
  stepper(t, id, value, onMinus, onPlus) {
    const b = (k, label, f) =>
      div()
        .id(`${id}-${k}`)
        .w(28)
        .h(28)
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .text_size(15)
        .text_color(t.muted)
        .hover((s) => s.bg(t.hover).text_color(t.foreground))
        .on_click(f)
        .child(label);
    return h_flex()
      .items_center()
      .rounded(7)
      .border_1()
      .border_color(t.border)
      .bg(t.raised)
      .overflow_hidden()
      .child(b("minus", "\u2212", onMinus))
      .child(div().w(54).text_size(13).text_color(t.foreground).flex().justify_center().child(value))
      .child(b("plus", "+", onPlus));
  }

  toggle(t, id, on, onClick) {
    return div()
      .id(id)
      .w(36)
      .h(20)
      .rounded(10)
      .p(2)
      .cursor_pointer()
      .bg(on ? t.accent : t.hover)
      .flex()
      .when(on, (el) => el.justify_end())
      .on_click(onClick)
      .child(div().w(16).h(16).rounded(8).bg(on ? t.accent_foreground : t.muted));
  }

  // Look -----------------------------------------------------------------

  // A small drawing of each layout, in the theme's colors (as the welcome draws them).
  preview(t, name) {
    const row = () => div().flex().flex_row();
    const col = () => div().flex().flex_col();
    const term = (bg = t.background, edge = t.border) => div().flex_1().rounded(2).bg(bg).border_1().border_color(edge);
    const line = (w, c = t.dim) => div().h(3).w(w).flex_none().rounded(2).bg(c);
    const lines = (n, c) => col().gap(4).p(5).children([...Array(n)].map((_, i) => line(i % 2 ? 22 : 32, c)));
    const frame = (el) => el.size_full().rounded(6).overflow_hidden().border_1().border_color(t.border).bg(t.surface);
    if (name === "main")
      return frame(row())
        .child(col().w(42).border_r(1).border_color(t.border).child(lines(4)))
        .child(
          col()
            .flex_1()
            .child(row().h(11).gap(2).p(2).border_b(1).border_color(t.border).child(div().w(20).rounded(2).bg(t.accent)).child(div().w(16).rounded(2).bg(t.raised)).child(div().w(16).rounded(2).bg(t.raised)))
            .child(row().flex_1().p(4).gap(3).child(term()).child(term())),
        );
    if (name === "terminals") {
      const card = () => col().flex_1().rounded(4).border_1().border_color("#2c2f34").bg("#17191c").child(row().h(7).px(3).items_center().border_b(1).border_color("#25282c").child(line(14, "#5d6168")));
      return frame(col())
        .bg("#131619")
        .p(4)
        .gap(4)
        .child(row().h(10).gap(3).p(1).rounded(4).bg("#1d1f22").child(div().w(30).rounded(3).bg("#34373c").border_1().border_color("#4a4d53")).child(div().w(30)))
        .child(row().flex_1().gap(4).child(card()).child(card()));
    }
    // A layout of the person's own: a plain frame.
    return frame(col()).child(row().h(11).border_b(1).border_color(t.border)).child(row().flex_1().p(4).child(term()));
  }

  layoutPicker(t, cx) {
    const current = shared("layout") ?? "main";
    // Shipped looks in the welcome's order, then the person's own; new files show up by themselves.
    const shipped = Object.keys(LOOKS);
    const rank = (n) => (shipped.includes(n) ? shipped.indexOf(n) : shipped.length);
    const names = [...layouts()].sort((a, b) => rank(a) - rank(b) || a.localeCompare(b));
    if (names.join("\n") !== this.previewsOf) this.loadPreviews(names, cx);
    return h_flex()
      .w_full()
      .flex_wrap()
      .gap(8)
      .children(
        names.map((name) => {
          const on = name === current;
          return v_flex()
            .id(`st-l-${name}`)
            .w(124)
            .gap(6)
            .p(6)
            .rounded(10)
            .border_2()
            .border_color(on ? t.accent : "#00000000")
            .bg(on ? t.raised : "#00000000")
            .cursor_pointer()
            .hover((s) => s.bg(t.raised))
            .on_click((_e, cx) => {
              run("layout.switch", { name });
              cx.notify();
            })
            .child(
              div()
                .h(62)
                .w_full()
                .overflow_hidden()
                .child(
                  this.previews[name]
                    ? image(this.previews[name]).w_full().h(62).rounded(6).border_1().border_color(t.border)
                    : this.preview(t, name),
                ),
            )
            .child(
              h_flex()
                .gap(5)
                .items_center()
                .child(div().text_size(12.5).text_color(on ? t.foreground : t.muted).child(LOOKS[name] === "settings.terminals" ? tr(LOOKS[name]) : (LOOKS[name] ?? name)))
                .when(on, (el) => el.child(svg(I("check")).size(12).text_color(t.accent))),
            );
        }),
      );
  }

  // A row of choices, one picked: [[value, label key or text], ...].
  segmented(t, name, choices, now, pick) {
    return h_flex()
      .p(2)
      .gap(2)
      .rounded(8)
      .border_1()
      .border_color(t.border)
      .bg(t.background)
      .children(
        choices.map(([value, label]) =>
          div()
            .id(`st-${name}-${label}`)
            .px(12)
            .py(4)
            .rounded(6)
            .text_size(13)
            .cursor_pointer()
            .when(now === value, (el) => el.bg(t.raised).text_color(t.foreground).border_1().border_color(t.border))
            .when(now !== value, (el) => el.text_color(t.muted).hover((s) => s.text_color(t.foreground)))
            .on_click((_e, cx) => pick(value, cx))
            .child(tr(label)),
        ),
      );
  }

  appearancePicker(t, cx) {
    return this.segmented(t, "ap", APPEARANCES, this.person.appearance ?? null, (value, cx) =>
      this.change((p) => {
        if (value === null) delete p.appearance;
        else p.appearance = value;
      }, cx),
    );
  }

  mods(t, cx) {
    return v_flex().w_full().child(this.heading(t, tr("settings.mods"))).child(this.modsSection.render(t, cx));
  }

  themePicker(t, light) {
    const s = settings();
    const current = light ? s.theme_light : s.theme_dark;
    return h_flex()
      .w_full()
      .flex_wrap()
      .gap(10)
      .children(
        // A theme named like a layout (themes/win98.json) is that layout's own.
        themes()
          .filter((x) => Boolean(x.light) === light && (x.name === shared("layout") || !layouts().includes(x.name)))
          .map((entry) =>
            themeCard(t, entry, {
              selected: entry.name === current,
              onClick: (_e, cx) => {
                run("app.theme", { name: entry.name });
                cx.notify();
              },
              onEdit: (_e, cx) =>
                cx.spawn(async (cx) => {
                  const file = await loadThemeFile(entry.name);
                  if (file) this.editing = new ThemeEditor(this, { file });
                  cx.notify();
                }),
            }),
          ),
      );
  }

  // The theme tools under the "Theme" heading: import, a new theme of your own, and their panels.
  themeTools(t, cx) {
    const s = settings();
    return v_flex()
      .w_full()
      .gap(10)
      .child(
        h_flex()
          .gap(8)
          .pb(6)
          .child(this.button(t, "st-theme-import", tr("settings.import_theme"), (_e, cx) => {
            this.importing = !this.importing;
            if (this.importing) this.importPanel = new ImportPanel(this);
            cx.notify();
          }))
          .child(this.button(t, "st-theme-new", tr("settings.theme_new"), (_e, cx) => {
            const all = themes();
            this.editing = new ThemeEditor(this, { entry: all.find((x) => x.name === s.theme) ?? all[0] });
            cx.notify();
          })),
      )
      .when(this.importing && this.importPanel !== null, (el) => el.child(this.importPanel.render(t, cx)))
      .when(this.editing !== null, (el) => el.child(this.editing.render(t, cx)));
  }

  // The font in use, written in itself; a click opens the list.
  fontButton(t, which, family, def) {
    const open = this.picking === which;
    const trigger = div()
      .id(`st-f-${which}`)
      .w(220)
      .px(12)
      .py(6)
      .rounded(7)
      .border_1()
      .border_color(open ? t.accent : t.border)
      .bg(t.raised)
      .cursor_pointer()
      .hover((s) => s.bg(t.hover))
      .on_click((_e, cx) => {
        this.picking = open ? null : which;
        this.filter = "";
        this.fontInput.set_value("");
        if (this.picking) this.fontInput.focus();
        else this.handle.focus();
        cx.notify();
      })
      .child(div().font_family(family).text_size(13.5).text_color(t.foreground).child(family));
    return Popup.new(`st-fp-${which}`, trigger)
      .when(open, (el) => el.content(this.fontList(t, which, def)));
  }

  fontList(t, which, def) {
    const q = this.filter.toLowerCase();
    const matches = fonts().filter((f) => f.toLowerCase().includes(q)).slice(0, 200);
    const choose = (name, cx) => {
      this.change((p) => {
        if (name === null) delete p[which];
        else p[which] = name;
      }, cx);
      this.picking = null;
      this.handle.focus();
      cx.notify();
    };
    const item = (id, label, family, onClick) =>
      div()
        .id(id)
        .px(10)
        .py(5)
        .rounded(6)
        .cursor_pointer()
        .hover((s) => s.bg(t.hover))
        .on_click(onClick)
        .child(div().font_family(family).text_size(13.5).text_color(t.foreground).child(label));
    return v_flex()
      .id(`st-font-list-${which}`)
      .w(360)
      .on_mouse_down_out((_e, cx) => {
        this.picking = null;
        this.handle.focus();
        cx.notify();
      })
      .rounded(8)
      .border_1()
      .border_color(t.border)
      .bg(t.background)
      // Let the child's native scroll run before stopping the page's wheel event.
      .on_scroll_wheel((_e, cx) => cx.stop_propagation())
      .child(
        div()
          .px(12)
          .py(8)
          .border_b(1)
          .border_color(t.border)
          .text_size(13)
          .child(Input.new(this.fontInput).h(28).text_color(t.foreground)),
      )
      .child(
        div()
          .id(`st-fl-${which}`)
          .h(200)
          .p(4)
          .overflow_y_scrollbar()
          .when(!q, (el) => el.child(item(`st-fd-${which}`, tr("settings.default_def", { def }), def, (_e, cx) => choose(null, cx))))
          .children(matches.map((f, i) => item(`st-fi-${which}-${i}`, f, f, (_e, cx) => choose(f, cx)))),
      );
  }

  look(t, cx) {
    const s = settings();
    const p = this.person;
    // Numbers come back from the host as 32-bit floats: 1.1 reads as 1.100000023841858.
    const same = (a, b) => (typeof a === "number" && typeof b === "number" ? Math.abs(a - b) < 1e-4 : a === b);
    const own = (key, composed, def) => !same(p[key] ?? def, composed);
    const size = (key, value, d, cx) => this.change((q) => (q[key] = Math.max(8, Math.min(32, (q[key] ?? value) + d))), cx);
    const step = (key, value, d, lo, hi, cx) => this.change((q) => (q[key] = Math.round(Math.max(lo, Math.min(hi, (q[key] ?? value) + d)) * 100) / 100), cx);
    const fontRow = (which, sizeKey, title, family, def, sizeNow) =>
      v_flex()
        .w_full()
        .child(
          this.row(
            t,
            title,
            own(which, family, def) ? tr("settings.this_layout_sets_its_own") : null,
            h_flex()
              .gap(8)
              .items_center()
              .child(this.fontButton(t, which, family, def))
              // The panels' text follows zoom: they size their rows to it.
              .when(Boolean(sizeKey), (el) => el.child(this.stepper(t, `st-${sizeKey}`, `${Math.round(sizeNow)}`, (_e, cx) => size(sizeKey, sizeNow, -1, cx), (_e, cx) => size(sizeKey, sizeNow, 1, cx)))),
          ),
        );
    return v_flex()
      .w_full()
      .child(this.heading(t, tr("settings.layout")))
      .child(this.layoutPicker(t, cx))
      .child(div().h(18))
      .child(this.heading(t, tr("settings.theme")))
      .child(this.themeTools(t, cx))
      .child(this.row(t, tr("settings.appearance"), tr("settings.light_or_dark_or_as"), this.appearancePicker(t, cx)))
      .child(div().pt(12).pb(8).text_size(12.5).text_color(t.dim).child(tr("settings.for_the_layout_you_re_in")))
      .child(div().pb(6).text_size(12.5).font_medium().text_color(t.muted).child(tr("settings.dark")))
      .child(this.themePicker(t, false))
      .child(div().pt(10).pb(6).text_size(12.5).font_medium().text_color(t.muted).child(tr("settings.light")))
      .child(this.themePicker(t, true))
      .child(div().h(18))
      .child(this.heading(t, tr("settings.size_and_fonts")))
      .child(
        this.row(
          t,
          "Zoom",
          tr("settings.everything_terminals_panels_text"),
          h_flex()
            .gap(8)
            .items_center()
            .child(this.stepper(t, "st-zoom", `${Math.round(s.zoom * 100)}%`, () => run("app.zoom_out"), () => run("app.zoom_in")))
            .child(this.button(t, "st-zoom-reset", tr("settings.reset"), () => run("app.zoom_reset"))),
        ),
      )
      .child(fontRow("font", "font_size", tr("settings.terminal_font"), s.font_family, DEFAULT_FONT, s.font_size))
      .child(
        this.row(
          t,
          tr("settings.terminal_font_weight"),
          own("font_weight", s.font_weight, 500) ? tr("settings.this_layout_sets_its_own") : tr("settings.bold_text_stays_at_700"),
          this.stepper(t, "st-font_weight", `${s.font_weight}`, (_e, cx) => step("font_weight", s.font_weight, -100, 100, 900, cx), (_e, cx) => step("font_weight", s.font_weight, 100, 100, 900, cx)),
        ),
      )
      .children([
        ["terminal_line_height", tr("settings.terminal_line_height"), 0.8, 2, 0.1],
        ["terminal_cell_width", tr("settings.terminal_cell_width"), 0.8, 2, 0.05],
        ["terminal_scroll_speed", tr("settings.terminal_scroll_speed"), 0.1, 5, 0.1],
      ].map(([key, title, min, max, step]) => {
        const by = (d) => (_e, cx) => this.change((q) => (q[key] = Math.max(min, Math.min(max, Math.round(Math.round(((q[key] ?? s[key]) + d) / step) * step * 100) / 100))), cx);
        return this.row(
          t, title,
          own(key, s[key], 1) ? tr("settings.this_layout_sets_its_own") : null,
          this.stepper(t, `st-${key}`, `${Math.round(s[key] * 100)}%`, by(-step), by(step)),
        );
      }))
      .child(fontRow("ui_font", null, tr("settings.interface_font"), s.ui_font_family, DEFAULT_UI_FONT));
  }

  // Layout -------------------------------------------------------------------

  // Every block of the shown layout, grouped by the row or column it sits in:
  // move one earlier or later in its group, hide or show it.
  design(t, cx) {
    const root = layout();
    const named = (n) => (LAYOUT_NAMES[n.id] ? tr(LAYOUT_NAMES[n.id]) : n.title ?? n.id);
    const icon = (id, name, onClick) => div().id(id).cursor_pointer().p(5).rounded(5).hover((s) => s.bg(t.hover)).on_click(onClick).child(svg(name).size(14).text_color(t.muted));
    const group = (node, top) => {
      if (!node.children || node.type === "stack") return [];
      const kids = node.children;
      const row = node.type === "row";
      const own = kids.length > 1 || top || kids.some((k) => k.hidden)
        ? [
            div().pt(top ? 0 : 14).pb(4).text_size(12).font_semibold().text_color(t.dim).child(top ? tr("settings.layout_window") : named(node)),
            ...kids.map((k, i) => h_flex().id(`st-ly-${k.id}`).w_full().gap(8).py(4).items_center()
              .child(div().flex_1().min_w(0).truncate().text_size(13).text_color(k.hidden ? t.dim : t.foreground).child(named(k)))
              .when(i > 0, (el) => el.child(icon(`st-ly-b-${k.id}`, row ? "frame/settings/chevron-left.svg" : "frame/settings/chevron-up.svg", () => run("layout.move", { id: k.id, before: kids[i - 1].id }))))
              .when(i === 0, (el) => el.child(div().w(24)))
              .when(i < kids.length - 1, (el) => el.child(icon(`st-ly-a-${k.id}`, row ? "frame/settings/chevron-right.svg" : "frame/settings/chevron-down.svg", () => run("layout.move", { id: k.id, after: kids[i + 1].id }))))
              .when(i === kids.length - 1, (el) => el.child(div().w(24)))
              .child(k.id === "main" && !k.hidden ? div().w(36) : this.toggle(t, `st-ly-t-${k.id}`, !k.hidden, () => run("layout.toggle", { id: k.id })))),
          ]
        : [];
      return [...own, ...kids.filter((k) => !k.hidden).flatMap((k) => group(k, false))];
    };
    const base = String(shared("layout") ?? "").replace(/^my-/, "");
    const mine = String(shared("layout") ?? "").startsWith("my-") && SHIPPED_LAYOUTS.includes(base);
    return v_flex()
      .w_full()
      .child(this.heading(t, tr("settings.layout")))
      .child(div().pb(12).text_size(12.5).line_height("18px").text_color(t.dim).child(tr("settings.layout_note")))
      .child(div().pb(14).child(this.button(t, "st-ly-drag", tr("settings.layout_drag"), () => { run("overlay.close"); run("layout.edit"); })))
      .children(group(root, true))
      .child(div().h(18))
      .child((this.sidebar ??= new SidebarEditor(this, SHIPPED_LAYOUTS)).render(t, cx))
      .when(mine, (el) => el.child(div().pt(18).child(this.button(t, "st-ly-back", tr("settings.layout_back"), () => run("layout.switch", { name: base })))));
  }

  // General ----------------------------------------------------------------

  general(t) {
    const p = this.person;
    const roots = p.folders?.roots ?? ["~"];
    const setRoots = (next, cx) => this.change((q) => { q.folders = { ...(q.folders ?? {}), roots: next.length ? next : ["~"] }; }, cx);
    return v_flex()
      .w_full()
      .child(this.heading(t, tr("settings.general")))
      .child(this.row(t, tr("settings.language"), tr("settings.language_about"),
        this.segmented(t, "lang", [[null, "settings.language_system"], ["es", "settings.language_es"], ["en", "settings.language_en"]], p.language ?? null, (value, cx) =>
          this.change((q) => { if (value === null) delete q.language; else q.language = value; }, cx))))
      .when(window()?.platform !== "windows", (el) => el.child(this.row(t, tr("settings.keep_awake"), tr("settings.keep_awake_about"),
        this.segmented(t, "awake", [["off", "awake.menu.off"], ["auto", "awake.menu.auto"], ["on", "awake.menu.on"]], settings().keep_awake ?? "off", (value) => run("app.keep_awake", { mode: value })))))
      .child(this.row(t, tr("settings.resume_agents"), tr("settings.resume_agents_about"),
        this.toggle(t, "st-resume", p.resume_agents !== false, (_e, cx) =>
          this.change((q) => { if (q.resume_agents === false) delete q.resume_agents; else q.resume_agents = false; }, cx))))
      .child(this.row(t, tr("settings.folder_roots"), tr("settings.folder_roots_about"),
        v_flex().gap(6).items_end()
          .children(roots.map((r, i) => h_flex().id(`st-root-${i}`).gap(8).items_center()
            .child(div().text_size(13).text_color(t.muted).child(r))
            .child(div().id(`st-root-x-${i}`).cursor_pointer().on_click((_e, cx) => setRoots(roots.filter((_, j) => j !== i), cx))
              .child(svg("mods/workspace-chrome/x.svg").size(13).text_color(t.dim)))))
          .child(this.button(t, "st-root-add", tr("settings.add_folder"), (_e, cx) => {
            cx.spawn(async (cx) => {
              const folder = await pickFolder({ prompt: tr("settings.add_folder_prompt") });
              if (folder && !roots.includes(folder)) setRoots([...roots, folder], cx);
              cx.notify();
            });
          }))));
  }

  // Agents -------------------------------------------------------------------

  loadAgents(cx) {
    if (this.known || checking() || this.knownLoading) return;
    this.knownLoading = true;
    cx.spawn(async (cx) => {
      try {
        const known = (await knownAgents()).filter((a) => a.installed);
        // Inputs are made here, never while drawing.
        for (const a of known) {
          const input = InputState.new({ value: a.base.join(" ") });
          input.on("submit", (_e, cx) => {
            const words = input.value().trim().split(/\s+/).filter(Boolean);
            this.changeFile("programs.json", (o) => {
              o.programs ??= {};
              o.programs[a.key] ??= {};
              if (words.length) o.programs[a.key].run = words;
              else delete o.programs[a.key].run;
            }, cx);
          });
          this.runInputs[a.key] = input;
        }
        this.known = known;
        this.knownFailed = false;
      } catch {
        this.knownFailed = true;
      }
      this.knownLoading = false;
      cx.notify();
    });
  }

  agents(t, cx) {
    this.loadAgents(cx);
    const yolo = this.person.yolo ?? {};
    const all = yolo.enabled ?? true;
    const list = this.known ?? [];
    return v_flex()
      .w_full()
      .child(this.heading(t, tr("settings.agents")))
      .child(this.row(t, tr("settings.full_access"), tr("settings.full_access_about"),
        this.toggle(t, "st-yolo", all, (_e, cx) => this.change((q) => { q.yolo = { ...(q.yolo ?? {}), enabled: !all }; }, cx))))
      .when(this.knownFailed, (el) => el.child(div().py(12).text_size(13).text_color(t.dim).child(tr("settings.agents_failed"))))
      .when(!this.known && !this.knownFailed, (el) => el.child(div().py(12).text_size(13).text_color(t.dim).child(tr("settings.agents_checking"))))
      .children(list.map((a) => {
        const input = this.runInputs[a.key];
        const on = yolo.agents?.[a.key] ?? all;
        return h_flex().id(`st-agent-${a.key}`).w_full().gap(12).py(10).items_center()
          .child(div().w(130).flex_none().text_size(14).text_color(t.foreground).child(a.name))
          .child(Input.new(input).flex_1().min_w(0).h(30).px(8).rounded(7).border_1().border_color(t.border).bg(t.background).text_size(12.5).text_color(t.foreground))
          .child(div().w(190).flex_none().text_size(11.5).text_color(t.dim).truncate().child(a.yolo.join(" ")))
          .child(a.yolo.length
            ? this.toggle(t, `st-yolo-${a.key}`, on, (_e, cx) => this.change((q) => {
                q.yolo = { ...(q.yolo ?? {}) };
                q.yolo.agents = { ...(q.yolo.agents ?? {}), [a.key]: !on };
              }, cx))
            : div().w(36));
      }));
  }

  keyboard(t) {
    const mac = window()?.platform === "macos";
    const choices = [
      ["auto", tr("settings.automatic_alt_on_us_keyboards")],
      ["on", "Alt"],
      ["off", tr("settings.types_characters")],
      ["left", tr("settings.left_option_is_alt")],
      ["right", tr("settings.right_option_is_alt")],
    ];
    const option = v_flex()
      .w_full()
      .child(this.heading(t, tr("settings.option_key")))
      .child(div().pb(12).text_size(13).text_color(t.dim).child(tr("settings.use_option_for_terminal_shortcuts")))
      .children(choices.map(([value, label]) => {
        const on = (this.person.option_as_alt ?? "auto") === value;
        return h_flex()
          .id(`st-option-${value}`)
          .gap(10)
          .px(12)
          .py(10)
          .rounded(7)
          .items_center()
          .cursor_pointer()
          .hover((s) => s.bg(t.hover))
          .on_click((_e, cx) => this.change((p) => { p.option_as_alt = value; }, cx))
          .child(div().text_color(on ? t.accent : t.muted).child(on ? "◉" : "○"))
          .child(div().text_size(14).text_color(t.foreground).child(label));
      }));
    return v_flex().w_full().when(mac, (el) => el.child(option)).child(this.shortcuts(t));
  }

  // Every shortcut of keys.json, grouped by section: change or remove one.
  shortcuts(t) {
    const bySection = {};
    for (const k of keys()) (bySection[k.section] ??= []).push(k);
    const order = Object.keys(bySection).sort((a, b) => {
      const rank = (n) => (n === "global" ? 0 : n === "terminal" ? 1 : 2);
      return rank(a) - rank(b) || a.localeCompare(b);
    });
    const icon = (id, name, onClick) => div().id(id).cursor_pointer().p(4).rounded(5).hover((s) => s.bg(t.hover)).on_click(onClick).child(svg(name).size(14).text_color(t.dim));
    return v_flex()
      .w_full()
      .child(this.heading(t, tr("settings.shortcuts")))
      .child(div().pb(12).text_size(12.5).line_height("18px").text_color(t.dim).child(tr("settings.keys_note")))
      .children(order.flatMap((section) => [
        div().pt(10).pb(4).text_size(12).font_semibold().text_color(t.dim).child(section),
        ...bySection[section].map((k, i) => {
          const editable = typeof k.action === "string";
          const asking = this.capturing && this.capturing.section === section && this.capturing.old === k.keys;
          const name = editable ? k.action : k.action?.command ?? "";
          return h_flex().id(`st-key-${section}-${i}`).w_full().gap(10).py(6).items_center()
            .child(div().flex_1().min_w(0).truncate().text_size(13).text_color(t.foreground).child(name))
            .child(div().text_size(12.5).text_color(asking ? t.accent : t.muted).child(asking ? tr("settings.keys_press") : k.keys))
            .when(editable, (el) => el
              .child(this.button(t, `st-key-change-${section}-${i}`, tr("settings.keys_change"), (_e, cx) => {
                this.capturing = asking ? null : { section, action: k.action, old: k.keys };
                this.handle.focus();
                cx.notify();
              }))
              .child(icon(`st-key-del-${section}-${i}`, "mods/workspace-chrome/x.svg", (_e, cx) =>
                this.changeFile("keys.json", (o) => { delete o[section]?.[k.keys]; }, cx))));
        }),
      ]));
  }

  // The chord pressed while a shortcut is being changed. True when it was used.
  capture(e, cx) {
    const c = this.capturing;
    if (e.key === "escape") {
      this.capturing = null;
      return true;
    }
    if (["shift", "control", "alt", "cmd", "platform", "function"].includes(e.key)) return true;
    const chord = e.keystroke;
    const taken = keys().find((k) => k.section === c.section && k.keys === chord && k.keys !== c.old);
    if (taken) {
      notify({ level: "error", title: tr("settings.keys_taken", { action: typeof taken.action === "string" ? taken.action : taken.action?.command ?? "" }) });
      return true;
    }
    this.capturing = null;
    this.changeFile("keys.json", (o) => {
      o[c.section] ??= {};
      delete o[c.section][c.old];
      o[c.section][chord] = c.action;
    }, cx);
    return true;
  }

  // Machines -------------------------------------------------------------

  async loadMachines() {
    if (this.machineLoading) return;
    this.machineLoading = true;
    try {
      const r = await hc(["machine", "ls", "--json"]);
      if (r.code !== 0) throw new Error(r.stderr || r.stdout || tr("settings.couldn_t_read_machines"));
      const list = JSON.parse(r.stdout).machines;
      if (!Array.isArray(list)) throw new Error(tr("settings.couldn_t_read_machines"));
      this.machineList = list.filter((m) => !m.temporary);
      this.machineError = null;
    } catch (e) {
      this.machineError = String(e);
    } finally {
      this.machineLoading = false;
    }
  }

  machineAction(machine, action, cx, name) {
    if (this.machineBusy[machine]) return;
    if (action === "rename" && (!name?.trim() || name.trim() === machine)) return;
    this.machineBusy[machine] = action;
    delete this.machineResults[machine];
    this.machineEdit = null;
    this.handle.focus();
    cx.notify();
    cx.spawn(async (cx) => {
      try {
        const args = ["machine", action, machine];
        if (action === "rename") args.push(name.trim());
        const r = await hc(args);
        let payload = null;
        try { payload = JSON.parse(r.stdout || r.stderr); } catch {}
        if (r.code !== 0) throw new Error(payload?.error?.message || (r.stderr || r.stdout || tr("settings.it_didn_t_work")).trim());
        const message = action === "update"
          ? (payload?.version ? tr("settings.updated_machine_to_hc_version", { machine, version: payload.version }) : tr("settings.updated_machine_its_terminals_kept", { machine }))
          : action === "rename" ? tr("settings.renamed_machine_to_name", { machine, name: name.trim() }) : tr("settings.removed_machine_its_terminals_keep", { machine });
        if (action === "update") this.machineResults[machine] = { message, ok: true };
        else this.machineNotice = message;
      } catch (e) {
        this.machineResults[machine] = { message: String(e), ok: false };
      } finally {
        delete this.machineBusy[machine];
      }
      await this.loadMachines();
      cx.notify();
    });
  }

  olderCore(remote, local) {
    const parse = (v) => /^(\d+)\.(\d+)\.(\d+)(?:-([^+]+))?(?:\+.*)?$/.exec(v ?? "");
    const a = parse(remote), b = parse(local);
    if (!a || !b) return false;
    for (let i = 1; i <= 3; i++) {
      if (+a[i] !== +b[i]) return +a[i] < +b[i];
    }
    if (!a[4] || !b[4]) return Boolean(a[4] && !b[4]);
    const x = a[4].split("."), y = b[4].split(".");
    for (let i = 0; i < Math.max(x.length, y.length); i++) {
      if (x[i] === y[i]) continue;
      if (x[i] === undefined || y[i] === undefined) return x[i] === undefined;
      const xn = /^\d+$/.test(x[i]), yn = /^\d+$/.test(y[i]);
      return xn && yn ? +x[i] < +y[i] : xn !== yn ? xn : x[i] < y[i];
    }
    return false;
  }

  machineRow(t, m, localVersion) {
    const busy = this.machineBusy[m.name];
    const edit = this.machineEdit?.machine === m.name ? this.machineEdit : null;
    const STATES = { connected: "settings.connected_2", online: "settings.online", connecting: "settings.connecting", reconnecting: "settings.reconnecting", offline: "settings.offline", unknown: "settings.unknown", "needs_login": "settings.needs_login" };
    const state = STATES[m.state ?? "unknown"] ? tr(STATES[m.state ?? "unknown"]) : String(m.state ?? "unknown").replace(/[_-]/g, " ");
    const color = ["connected", "online"].includes(m.state) ? t.ansi2 : ["connecting", "reconnecting"].includes(m.state) ? t.working : t.blocked;
    const result = this.machineResults[m.name];
    const controls = h_flex().gap(6).items_center();
    if (busy) controls.child(svg("mods/workspace-chrome/spinner.svg").size(12).text_color(t.working).spin(900)).child(div().text_size(12).text_color(t.muted).child(busy === "update" ? tr("settings.updating") : busy === "rename" ? tr("settings.renaming") : tr("settings.removing")));
    else {
      if (this.olderCore(m.version, localVersion)) controls.child(this.button(t, `st-m-update-${m.name}`, tr("settings.update"), (_e, cx) => this.machineAction(m.name, "update", cx)));
      for (const [action, label] of [["rename", tr("settings.rename")], ["rm", tr("settings.remove")]]) {
        controls.child(this.button(t, `st-m-${action}-${m.name}`, label, (_e, cx) => {
          this.machineEdit = { machine: m.name, action, name: m.name };
          if (action === "rename") {
            this.renameInput.set_value(m.name);
            this.renameInput.set_selected_range({ start: 0, end: encodeURIComponent(m.name).replace(/%[0-9A-F]{2}/g, "_").length });
            this.renameFocused = true;
            this.renameInput.focus();
          }
          cx.notify();
        }));
      }
    }
    const row = v_flex().w_full().py(12).gap(8).border_b(1).border_color(t.border)
      .child(h_flex().gap(10).items_center()
        .child(div().w(7).h(7).flex_none().rounded(4).bg(color))
        .child(v_flex().flex_1().min_w(0).gap(3)
          .child(div().text_size(14).text_color(t.foreground).child(m.name))
          .child(div().text_size(12.5).text_color(t.dim).child(`${m.target ?? ""} · ${state} · ${m.version ? `hc ${m.version}` : tr("settings.hc_version_unknown")}`)))
        .when(m.keeps_running_after_logout === false, el => el.child(div().id(`st-m-logout-${m.name}`)
          .text_color(t.blocked).cursor_pointer().tooltip(tr("settings.agents_stop_when_you_disconnect"))
          .on_click(() => run("mod.open", { name: "machine-add" })).child("⚠")))
        .child(controls.flex_none()));
    if (m.state === "needs_login") {
      const command = `hc machine login ${m.name}`;
      row.child(v_flex().gap(5)
        .child(div().text_size(12.5).text_color(t.blocked).child(tr("settings.needs_a_login_run")))
        .child(h_flex().gap(8).items_center()
          .child(div().flex_1().min_w(0).text_size(12.5).text_color(t.foreground).child(command))
          .child(this.button(t, `st-m-login-copy-${m.name}`, this.copiedLogin === command ? tr("settings.copied") : tr("settings.copy"), (_e, cx) => {
            cx.write_to_clipboard(command);
            this.copiedLogin = command;
            cx.notify();
          }))));
    }
    if (busy === "update") row.child(div().text_size(12.5).text_color(t.dim).child(tr("settings.copying_this_computer_s_herdr_core_its")));
    if (edit && !busy) {
      const cancel = this.button(t, `st-m-cancel-${m.name}`, tr("settings.cancel"), (_e, cx) => { this.machineEdit = null; this.handle.focus(); cx.notify(); });
      if (edit.action === "rm") row.child(v_flex().gap(8)
        .child(div().text_size(12.5).text_color(t.muted).child(tr("settings.remove_name_its_terminals_keep", { name: m.name })))
        .child(h_flex().gap(6).child(cancel).child(this.button(t, `st-m-confirm-${m.name}`, tr("settings.remove_machine"), (_e, cx) => this.machineAction(m.name, "rm", cx)))));
      else row.child(v_flex().gap(6)
        .child(div().text_size(12).text_color(t.dim).child(tr("settings.new_name_addresses_using_the")))
        .child(h_flex().gap(6).items_center()
          .child(Input.new(this.renameInput).flex_1().min_w(0).h(32).px(10).rounded(7).border_1().border_color(this.renameFocused ? t.accent : t.input).bg(t.background).text_size(13).text_color(t.foreground))
          .child(cancel).child(this.button(t, `st-m-save-${m.name}`, tr("settings.save"), (_e, cx) => this.machineAction(m.name, "rename", cx, edit.name)))));
    }
    if (result) row.child(div().text_size(12.5).text_color(result.ok ? t.ansi2 : t.blocked).child(result.message));
    return row;
  }

  machines(t) {
    const version = updates().core.version;
    return v_flex().w_full()
      .child(this.heading(t, tr("settings.machines")))
      .child(div().pb(8).text_size(12.5).line_height("18px").text_color(t.dim).child(tr("settings.your_computers_reached_over_ssh")))
      .child(this.row(t, tr("settings.this_computer"), `${tr("settings.connected")} · ${version ? `hc ${version}` : tr("settings.hc_version_unknown")}`, div().w(7).h(7).rounded(4).bg(t.ansi2)))
      .children((this.machineList ?? []).map((m) => this.machineRow(t, m, version)))
      .when(this.machineList === null && !this.machineError, (el) => el.child(h_flex().py(14).gap(8).items_center().child(svg("mods/workspace-chrome/spinner.svg").size(12).text_color(t.working).spin(900)).child(div().text_size(12.5).text_color(t.dim).child(tr("settings.reading_machines")))))
      .when(this.machineList?.length === 0, (el) => el.child(div().py(14).text_size(12.5).text_color(t.dim).child(tr("settings.no_other_machines_yet"))))
      .when(Boolean(this.machineError), (el) => el.child(div().py(10).text_size(12.5).text_color(t.blocked).child(this.machineError)))
      .when(Boolean(this.machineNotice), (el) => el.child(div().py(10).text_size(12.5).text_color(t.ansi2).child(this.machineNotice)))
      .child(h_flex().pt(16).child(this.button(t, "st-m-add", tr("settings.add_a_machine"), () => run("mod.open", { name: "machine-add" }))));
  }

  // Alerts ---------------------------------------------------------------

  alerts(t) {
    const sounds = this.person.sounds ?? {};
    const on = sounds.enabled !== false;
    const file = (f) => (f ? f.split(/[\\/]/).pop() : tr("settings.herdr_s_sound"));
    return v_flex()
      .w_full()
      .child(this.heading(t, tr("settings.sounds")))
      .child(
        this.row(
          t,
          tr("settings.play_sounds"),
          tr("settings.when_an_agent_needs_you"),
          this.toggle(t, "st-sounds", on, (_e, cx) => this.change((p) => (p.sounds = { ...(p.sounds ?? {}), enabled: !on }), cx)),
        ),
      )
      .child(this.row(t, tr("settings.an_agent_needs_you"), file(sounds.blocked), this.button(t, "st-play-blocked", tr("settings.play"), () => playSound("blocked"))))
      .child(this.row(t, tr("settings.an_agent_finished"), file(sounds.done), this.button(t, "st-play-done", tr("settings.play"), () => playSound("done"))))
      .child(div().pt(12).text_size(12.5).line_height("18px").text_color(t.dim).child(tr("settings.want_your_own_sounds_ask")));
  }

  // Updates --------------------------------------------------------------

  updates(t) {
    const u = updates();
    const at = (secs) => {
      const d = new Date(secs * 1000);
      return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
    };
    const checked = u.checked ? ` ${tr("settings.checked_at_time", { time: at(u.checked) })}` : "";
    const result = u.checking ? tr("settings.checking") : u.checked
      ? `${tr("settings.up_to_date")}.${checked}`
      : tr("settings.no_successful_check_check_your");
    // Pressed: "Checking\u2026" while it looks, then "Up to date" for a moment
    // (a find changes the row instead). One per row: an element shows once.
    const asked = this.asked && Date.now() - this.asked < 3000;
    const check = (id) =>
      !u.checks
        ? null
        : u.checking && this.asked
          ? div().px(12).py(5).text_size(13).text_color(t.dim).child(tr("settings.checking"))
          : asked && u.checked && !u.checking
            ? div().px(12).py(5).text_size(13).text_color(t.ansi2).child(`\u2713 ${tr("settings.up_to_date")}`)
            : this.button(t, id, tr("settings.check_now"), (_e, cx) => {
                this.asked = Date.now();
                run("app.check_updates");
                cx.notify();
              });
    const primary = (id, label, f) =>
      div().id(id).px(12).py(5).rounded(7).cursor_pointer().bg(t.accent).text_color(t.accent_foreground).text_size(13).font_semibold().hover((s) => s.opacity(0.9)).on_click(f).child(label);
    const app = !u.app.updates
      ? [tr("settings.a_build_from_source_it"), null]
      : u.app.ready
        ? [tr("settings.version_is_ready_restart_to", { version: u.app.ready }), primary("st-app-update", tr("settings.restart_to_update"), () => run("app.update"))]
        : [result, check("st-check-app")];
    const c = u.core;
    const core = c.managed
      ? [`${tr("settings.managed_by_manager_update_it", { manager: c.managed })}${c.ready ? ` ${tr("settings.version_is_available", { version: c.ready })}` : ""}`, u.checks ? check("st-check-core") : null]
      : c.updating
        ? [tr("settings.updating_your_terminals_keep_running"), null]
        : c.error
          ? [tr("settings.the_update_failed_error", { error: c.error }), primary("st-core-retry", tr("settings.try_again"), () => run("app.update_core"))]
          : c.ready
            ? [tr("settings.version_is_ready_your_terminals", { version: c.ready }), primary("st-core-update", tr("settings.update_herdr_core"), () => run("app.update_core"))]
            : u.checks
              ? [result, check("st-check-core")]
              : [tr("settings.not_checked_a_build_from"), null];
    const f = u.fork;
    const forkText = !f ? "" : f.error
      ? tr("settings.fork_error", { error: f.error })
      : f.newer
        ? tr("settings.fork_new", { version: f.latest })
        : f.latest
          ? tr("settings.fork_current", { version: f.latest })
          : tr("settings.fork_none");
    const forkOpen = f?.newer && f.url
      ? this.button(t, "st-fork-open", tr("settings.fork_open"), () => exec(window()?.platform === "macos" ? "open" : "xdg-open", [f.url]).catch(() => null))
      : null;
    return v_flex()
      .w_full()
      .child(this.heading(t, "Sunna"))
      .child(this.row(t, `Sunna ${u.app.version}`, app[0], h_flex().gap(8).flex_none()
        .child(this.button(t, "st-whats-new", tr("settings.what_s_new"), () => { closeOverlay(props()?.overlay); run("app.whats_new"); }))
        .when(Boolean(app[1]), (el) => el.child(app[1]))))
      .when(Boolean(f), (el) => el.child(this.row(t, tr("settings.fork_release"), forkText, forkOpen ?? div())))
      .child(div().h(18))
      .child(this.heading(t, "herdr-core"))
      .child(this.row(t, `herdr-core ${c.version ?? ""}`, core[0], core[1] ?? div()))
      .child(div().pt(12).text_size(12.5).line_height("18px").text_color(t.dim).child(tr("settings.herdr_looks_for_both_every")));
  }


  // Privacy --------------------------------------------------------------

  // A report as it goes out, one line per line of JSON.
  report(t, id, title, about, report) {
    const lines = JSON.stringify(report, null, 2).split("\n");
    return v_flex()
      .w_full()
      .pt(14)
      .gap(4)
      .child(div().text_size(13.5).text_color(t.foreground).child(title))
      .when(Boolean(about), (el) => el.child(div().text_size(12.5).text_color(t.dim).child(about)))
      .child(
        v_flex()
          .id(id)
          .mt(4)
          .w_full()
          .p(10)
          .rounded(8)
          .border_1()
          .border_color(t.border)
          .bg(t.background)
          .children(lines.map((l) => div().font_family(DEFAULT_FONT).text_size(12).line_height("17px").text_color(t.muted).whitespace_nowrap().child(l || " "))),
      );
  }

  privacy(t) {
    const s = telemetry();
    const on = s ? s.setting : this.person.telemetry !== false;
    const at = (secs) => new Date(secs * 1000).toLocaleString("es-CL");
    const why = s?.off_because ? tr("settings.off_for_now_reason_is", { reason: s.off_because }) : null;
    const body = v_flex()
      .w_full()
      .child(this.heading(t, tr("settings.usage_data")))
      .child(
        this.row(
          t,
          tr("settings.share_anonymous_usage_data"),
          why ?? tr("settings.once_a_day_which_agents"),
          this.toggle(t, "st-telemetry", on, (_e, cx) =>
            this.change((p) => {
              if (on) p.telemetry = false;
              else delete p.telemetry;
            }, cx),
          ),
        ),
      )
      .child(
        this.row(
          t,
          tr("settings.what_we_send"),
          s && !s.sends ? tr("settings.this_build_never_sends_it_s") : tr("settings.exactly_what_leaves_your_computer"),
          this.button(t, "st-telemetry-show", this.showData ? tr("settings.hide") : tr("settings.show"), (_e, cx) => {
            this.showData = !this.showData;
            cx.notify();
          }),
        ),
      );
    if (!this.showData) return body;
    if (!s?.enabled) return body.child(div().pt(14).text_size(13).text_color(t.dim).child(tr("settings.nothing_is_collected_or_sent")));
    return body
      .children(s.queued.map((r, i) => this.report(t, `st-tq-${i}`, tr("settings.waiting_to_send_day", { day: r.day }), tr("settings.sent_as_is_soon"), r)))
      .when(Boolean(s.draft), (el) => el.child(this.report(t, "st-td", tr("settings.today_so_far"), tr("settings.a_draft_it_changes_until"), s.draft)))
      .when(Boolean(s.last_sent), (el) => el.child(this.report(t, "st-tl", tr("settings.last_sent_day", { day: s.last_sent.report.day }), tr("settings.sent_time", { time: at(s.last_sent.at) }), s.last_sent.report)));
  }

  // ----------------------------------------------------------------------

  render(cx) {
    const t = theme();
    const u = updates();
    const waiting = Boolean(u.app.ready || (u.core.ready && !u.core.managed) || u.fork?.newer);
    const nav = v_flex()
      .flex_none()
      .w(190)
      .p(10)
      .gap(2)
      .border_r(1)
      .border_color(t.border)
      .child(div().px(10).pt(6).pb(12).text_size(16).font_semibold().text_color(t.foreground).child(tr("settings.settings")))
      .children(
        SECTIONS.map((s) => {
          const on = s.id === this.section;
          return h_flex()
            .id(`st-s-${s.id}`)
            .gap(10)
            .px(10)
            .py(7)
            .items_center()
            .rounded(7)
            .cursor_pointer()
            .bg(on ? t.raised : "#00000000")
            .hover((st) => st.bg(t.raised))
            .on_click((_e, cx) => {
              this.section = s.id;
              this.picking = null;
              this.machineEdit = null;
              this.handle.focus();
              if (s.id === "mods") this.modsSection.mods = null; // read the folders again on entering
              if (s.id === "machines") cx.spawn(async (cx) => { await this.loadMachines(); cx.notify(); });
              cx.notify();
            })
            .child(svg(I(s.icon)).size(15).text_color(on ? t.accent : t.dim))
            .child(div().flex_1().text_size(13.5).text_color(on ? t.foreground : t.muted).child(tr(s.title)))
            .when(s.id === "updates" && waiting, (el) => el.child(div().w(7).h(7).rounded(4).bg(t.accent)));
        }),
      );
    return h_flex()
      .id("settings")
      .track_focus(this.handle)
      .when(Boolean(this.picking), (el) => el.on_action("overlay.close", (_e, cx) => {
        this.picking = null;
        this.handle.focus();
        cx.stop_propagation();
        cx.notify();
      }))
      .w_full()
      .h(620)
      .min_h(0)
      .items_start()
      .on_key_down((e, cx) => {
        const k = e.key;
        if (this.capturing) {
          this.capture(e, cx);
          cx.stop_propagation();
          cx.notify();
          return;
        }
        if (this.machineEdit) {
          if (k === "escape") this.machineEdit = null;
          else return;
          this.handle.focus();
          cx.stop_propagation();
          cx.notify();
          return;
        }
        if (k === "escape") {
          if (this.picking) this.picking = null;
          else return this.close();
          this.handle.focus();
        } else return;
        cx.stop_propagation();
        cx.notify();
      })
      .child(nav.h_full().child(div().flex_1()).child(this.button(t, "st-feedback", tr("settings.send_feedback"), () => { closeOverlay(props()?.overlay); run("app.feedback"); })))
      .child(
        // A bar that always shows: the pages are taller than the dialog and
        // people should see that they scroll.
        div()
          .relative()
          .flex_1()
          .min_w(0)
          .h_full()
          .child(
            div()
              .id("st-body")
              .size_full()
              .overflow_y_scroll()
              // Padding inside, not on the scroll area: there it counts as
              // content and a page that fits still shows a bar.
              .child(
                div()
                  .px(28)
                  .py(22)
                  .child(
                    this.section === "look"
                      ? this.look(t, cx)
                      : this.section === "layout"
                        ? this.design(t, cx)
                      : this.section === "mods"
                        ? this.mods(t, cx)
                      : this.section === "general"
                        ? this.general(t)
                      : this.section === "agents"
                        ? this.agents(t, cx)
                      : this.section === "keyboard"
                        ? this.keyboard(t)
                      : this.section === "machines"
                        ? this.machines(t)
                      : this.section === "alerts"
                        ? this.alerts(t)
                        : this.section === "privacy"
                          ? this.privacy(t)
                          : this.updates(t),
                  ),
              ),
          )
          .child(Scrollbar.vertical("st-body").mode("always").absolute().inset_0()),
      );
  }
}
