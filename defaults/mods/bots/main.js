import { View, div, svg } from "gpui-kit";
import { v_flex, h_flex, InputState, Input, Scrollbar } from "gpui-base";
import { spaces, agents, run, exec, knownAgents, readTextFile, writeTextFile, pickFolder, notify, shared, setShared, appHome, home, checking, theme, tr, style } from "herdr";
import { readdir, mkdir } from "fs/promises";
import { TEMPLATES, CATEGORIES, NAME, purpose, runsToday, dueTriggers, withRun, startsOff, filterTemplates } from "./templates.js";

// Radius scale: the theme's `style.radius` (6 = the original sizes).
const R = (n) => Math.round(n * style().radius / 6);

// One color per bot, by its name.
const MARK = ["#22a99a", "#e3814f", "#5b8def", "#8b7cf6", "#3fae6a", "#d9649a"];
const dir = (name) => `${appHome()}/bots/${name}`;
const DEFAULT_BOT = { agent: "claude", folder: "~", enabled: true, yolo: false, max_runs_per_day: 10, triggers: [{ type: "manual" }] };

async function readJson(path, fallback) {
  try { return JSON.parse((await readTextFile(path)) ?? "null") ?? fallback; } catch { return fallback; }
}
// writeTextFile replaces only what it last saw: read first.
async function save(path, text) { await writeTextFile(path, text, await readTextFile(path)); }

export default class Bots extends View {
  init(_props, cx) {
    this.seenNew = shared("bots.new") ?? null; // the sidebar asks for a new bot by setting this
    this.seenTemplates = shared("bots.templates") ?? null; // ...or for the template gallery
    this.gallery = null; // { category } while the gallery is shown
    this.searchInput = InputState.new({ placeholder: tr("bots.templates_search") });
    this.searchInput.on("change", () => { if (this.gallery) this.gallery.query = this.searchInput.value(); });
    this.bots = []; this.form = null; this.history = null; this.busy = false; this.launching = new Set();
    this.nameInput = InputState.new({ placeholder: "my-bot" });
    this.nameInput.on("change", () => { if (this.form) this.form.name = this.nameInput.value().trim(); });
    this.load(cx);
    // The checks of a mod run init for real: no timers or process starts there.
    if (!checking()) cx.timer.every(60000, (cx) => this.tick(cx));
  }

  async load(cx) {
    let names = [];
    try { names = (await readdir(`${appHome()}/bots`, { withFileTypes: true })).filter((e) => e.isDirectory()).map((e) => e.name); } catch {}
    this.bots = await Promise.all(names.sort().map(async (name) => ({
      name,
      soul: (await readTextFile(`${dir(name)}/SOUL.md`)) ?? "",
      config: { ...DEFAULT_BOT, ...(await readJson(`${dir(name)}/bot.json`, {})) },
      state: await readJson(`${dir(name)}/runs.json`, { runs: [], seen: [], last_poll: {} }),
    })));
    cx.notify();
  }

  // Scheduler ---------------------------------------------------------------

  tick(cx) {
    if (this.busy) return;
    this.busy = true;
    cx.spawn(async (cx) => {
      try {
        const now = Date.now();
        for (const bot of this.bots) {
          if (!bot.config.enabled) continue;
          for (const trg of dueTriggers(bot.config, bot.state, now)) {
            // One failing bot (no gh, a bad folder) must not stop the others.
            try {
              if (trg.type === "github") await this.poll(bot, trg, now);
              else await this.launch(bot, trg.type, "", now);
            } catch (e) { console.error(`bot ${bot.name}: ${e?.message ?? e}`); }
          }
        }
      } finally { this.busy = false; cx.notify(); }
    });
  }

  // `~` is not expanded by the process start: the folder must be a real path.
  cwd(bot) { const f = bot.config.folder; return f === "~" ? home() : f.startsWith("~/") ? home() + f.slice(1) : f; }

  // The files may have been edited (SOUL.md, bot.json) since they were read.
  async fresh(bot) {
    bot.soul = (await readTextFile(`${dir(bot.name)}/SOUL.md`)) ?? bot.soul;
    bot.config = { ...DEFAULT_BOT, ...(await readJson(`${dir(bot.name)}/bot.json`, bot.config)) };
  }

  async poll(bot, trg, now) {
    const learned = Boolean(bot.state.learned);
    let r;
    try {
      r = await exec("gh", ["api", "-X", "GET", "search/issues", "-f", `q=${trg.query}`, "-f", "per_page=20", "--jq", ".items[] | [.html_url, .title] | @tsv"], { cwd: this.cwd(bot) });
    } catch { r = { code: 1, stdout: "" }; }
    // Even a failed look waits its interval, but only a good one counts as having learned what exists.
    bot.state.last_poll = { ...bot.state.last_poll, github: new Date(now).toISOString() };
    if (r.code !== 0) { await this.persist(bot); return; }
    const items = r.stdout.split("\n").filter(Boolean).map((l) => { const [url, ...title] = l.split("\t"); return { url, title: title.join("\t") }; });
    const seen = new Set(bot.state.seen ?? []);
    const fresh = items.filter((i) => !seen.has(i.url));
    // The first good look only learns what exists: no mass launch.
    if (!learned) fresh.forEach((i) => seen.add(i.url));
    else for (const item of fresh) {
      // Only what was launched counts as seen: a refused one (cap, busy) is tried again next time.
      if (await this.launch(bot, "github", `${item.title}\n${item.url}`, now)) seen.add(item.url);
    }
    bot.state.learned = true;
    bot.state.seen = [...seen].slice(-500);
    await this.persist(bot);
  }

  async launch(bot, trigger, item, now = Date.now()) {
    if (this.launching.has(bot.name)) return false;
    this.launching.add(bot.name);
    try { return await this.start(bot, trigger, item, now); } finally { this.launching.delete(bot.name); }
  }

  async start(bot, trigger, item, now) {
    await this.fresh(bot);
    const { config } = bot;
    if (runsToday(bot.state.runs ?? [], now) >= config.max_runs_per_day) return false;
    // One at a time: skip while the previous run's agent is still working.
    const prev = (bot.state.runs ?? []).at(-1);
    if (prev && agents().some((a) => a.tab_name === prev.tab && a.status === "working")) return false;
    let agent;
    try { agent = (await knownAgents()).find((a) => a.key === config.agent && a.installed); } catch {}
    if (!agent) { notify({ level: "error", title: tr("bots.agent_missing", { agent: config.agent }) }); return false; }
    // A github item is text from outside: never with the permission prompts off.
    const argv = config.yolo && trigger !== "github" ? agent.yolo : agent.base;
    const task = `${bot.soul}\n\n---\n${tr("bots.trigger_context", { trigger, item })}`;
    const n = (bot.state.runs?.length ? bot.state.runs.at(-1).id : 0) + 1;
    const tab = `bot ${bot.name} #${n}`, space = spaces().find((s) => s.name === `bot: ${bot.name}`);
    if (space) run("tab.new", { space: space.id, name: tab, folder: this.cwd(bot), argv, task, focus: false });
    else run("space.new", { name: `bot: ${bot.name}`, tab, folder: this.cwd(bot), argv, task, focus: false });
    bot.state = withRun(bot.state, { id: n, at: new Date(now).toISOString(), trigger, tab });
    await this.persist(bot);
    return true;
  }

  async persist(bot) { await save(`${dir(bot.name)}/runs.json`, JSON.stringify(bot.state, null, 2) + "\n"); }

  // Actions -----------------------------------------------------------------

  async setEnabled(bot, on, cx) {
    bot.config = { ...bot.config, enabled: on };
    await save(`${dir(bot.name)}/bot.json`, JSON.stringify(bot.config, null, 2) + "\n");
    cx.notify();
  }

  openSoul(bot) {
    setShared("files.reveal", { path: `${dir(bot.name)}/SOUL.md`, line: 1, at: Date.now() });
    run("layout.select", { stack: "panel-body", child: "files" });
  }

  newForm(cx, template = "blank") {
    this.gallery = null;
    this.form = { name: "", template, agent: null, folder: "~", yolo: false, installed: [] };
    this.nameInput.set_value("");
    cx.spawn(async (cx) => {
      try { this.form.installed = (await knownAgents()).filter((a) => a.installed); } catch {}
      this.form.agent = this.form.installed[0]?.key ?? "claude";
      cx.notify();
    });
    cx.notify();
  }

  async create(cx) {
    const f = this.form;
    if (!f || !NAME.test(f.name) || this.bots.some((b) => b.name === f.name)) return;
    const tpl = TEMPLATES[f.template];
    await mkdir(dir(f.name), { recursive: true });
    const config = { ...DEFAULT_BOT, agent: f.agent, folder: f.folder, yolo: f.yolo, triggers: tpl.triggers, enabled: !startsOff(tpl) };
    await writeTextFile(`${dir(f.name)}/SOUL.md`, tpl.soul, null);
    await writeTextFile(`${dir(f.name)}/bot.json`, JSON.stringify(config, null, 2) + "\n", null);
    this.form = null;
    await this.load(cx);
  }

  // Render ------------------------------------------------------------------

  nextRun(bot) {
    const trg = bot.config.triggers.find((x) => x.type === "interval" || x.type === "daily" || x.type === "github");
    if (!trg) return tr("bots.trigger_manual");
    return tr(`bots.trigger_${trg.type}`);
  }

  chip(t, id, label, on, pick) {
    return div().id(id).h(24).px(8).flex().items_center().rounded(R(6)).border_1().border_color(on ? t.accent : t.border).bg(on ? t.selected : t.background)
      .text_size(11).text_color(on ? t.foreground : t.muted).cursor_pointer().hover((s) => s.bg(t.hover)).on_click((_e, cx) => { pick(cx); cx.notify(); }).child(label);
  }

  // working: its last run's agent is busy; idle: waiting for a trigger; off: switched off.
  liveState(bot) {
    if (!bot.config.enabled) return "off";
    const prev = bot.state.runs?.at(-1);
    return prev && agents().some((a) => a.tab_name === prev.tab && a.status === "working") ? "working" : "idle";
  }

  card(t, bot) {
    const last = bot.state.runs?.at(-1);
    const btn = (id, label, action) => this.chip(t, id, label, false, action);
    const live = this.liveState(bot);
    const tone = live === "working" ? t.working : live === "idle" ? t.done ?? t.accent : t.dim;
    const hue = MARK[[...bot.name].reduce((n, c) => n + c.charCodeAt(0), 0) % MARK.length];
    const meta = (label) => div().h(20).px(7).flex().items_center().rounded(R(6)).bg(t.raised).text_size(11).text_color(t.muted).child(label);
    return v_flex().id(`bot-${bot.name}`).w_full().flex_none().p(12).gap(10).rounded(R(10)).border_1().border_color(t.border).bg(t.surface)
      .child(h_flex().gap(10).items_start()
        .child(div().size(34).flex_none().rounded(R(9)).bg(hue).flex().items_center().justify_center().text_size(15).font_bold().text_color("#ffffff").child([...bot.name][0]?.toUpperCase() ?? "?"))
        .child(v_flex().flex_1().min_w(0).gap(2)
          .child(div().truncate().font_semibold().text_size(14).child(bot.name))
          .child(div().text_size(12).line_height("16px").max_h(32).overflow_hidden().text_color(t.muted).child(purpose(bot.soul))))
        .child(this.chip(t, `bot-on-${bot.name}`, tr(`bots.status_${live}`), live !== "off", (cx) => this.setEnabled(bot, !bot.config.enabled, cx))
          .child(div().size(6).ml(6).rounded_full().bg(tone))))
      .child(h_flex().gap(6).flex_wrap()
        .child(meta(bot.config.agent)).child(meta(this.nextRun(bot)))
        .child(meta(`${tr("bots.last_run")}: ${last ? new Date(last.at).toLocaleString() : tr("bots.never")}`)))
      .child(h_flex().gap(6)
        .child(btn(`bot-run-${bot.name}`, tr("bots.run_now"), (cx) => cx.spawn(async (cx) => {
          if (!(await this.launch(bot, "manual", ""))) notify({ title: tr("bots.not_started") });
          cx.notify();
        })))
        .child(btn(`bot-soul-${bot.name}`, tr("bots.edit_soul"), () => this.openSoul(bot)))
        .child(btn(`bot-hist-${bot.name}`, tr("bots.detail"), () => { this.history = this.history === bot.name ? null : bot.name; })))
      .when(this.history === bot.name, (el) => el.child(this.detail(t, bot)))
      .when(this.history === bot.name, (el) => el.children((bot.state.runs ?? []).slice().reverse().slice(0, 20).map((r) =>
        div().id(`bot-r-${bot.name}-${r.id}`).h(22).px(6).flex().items_center().rounded(R(6)).text_size(11).text_color(t.muted).cursor_pointer().hover((s) => s.bg(t.hover))
          .on_click(() => { const tab = spaces().flatMap((s) => s.tabs).find((x) => x.name === r.tab); if (tab) run("tab.select", { id: tab.id }); })
          .child(`#${r.id} · ${r.trigger} · ${new Date(r.at).toLocaleString()}`))));
  }

  // Where it runs and what it can do: the bot acts as the person, so this says so before they switch it on.
  detail(t, bot) {
    const line = (text, tone = t.muted) => div().text_size(11).line_height("16px").text_color(tone).child(text);
    return v_flex().gap(3).p(8).rounded(R(8)).bg(t.raised)
      .child(line(tr("bots.runs_on", { folder: bot.config.folder, agent: bot.config.agent })))
      .child(line(tr("bots.perm_scope")))
      .child(line(bot.config.yolo ? tr("bots.perm_yolo") : tr("bots.perm_default"), bot.config.yolo ? t.blocked : t.muted));
  }

  label(id) { return `${tr(`bots.template_${id.replace(/-/g, "_")}`)} ${tr(`bots.first_${id.replace(/-/g, "_")}`)}`; }

  openGallery(cx) {
    this.form = null; this.gallery = { category: "all", query: "" };
    this.searchInput.set_value("");
    cx.notify();
  }

  // The templates: search, a category filter, and for each what it does first.
  galleryView(t) {
    const g = this.gallery, key = (id) => id.replace(/-/g, "_");
    const found = filterTemplates(g.query, g.category, (id) => this.label(id));
    const card = ([id, tpl]) => v_flex().id(`bot-tpl-card-${id}`).w_full().flex_none().p(10).gap(6).rounded(R(10)).border_1().border_color(t.border).bg(t.surface)
      .child(h_flex().gap(8).items_center()
        .child(div().flex_1().min_w(0).truncate().font_semibold().text_size(13).child(tr(`bots.template_${key(id)}`)))
        .child(div().h(18).px(6).flex().items_center().rounded(R(6)).bg(t.raised).text_size(10.5).text_color(t.muted).child(tr(`bots.cat_${tpl.category}`))))
      .child(div().text_size(11.5).line_height("16px").text_color(t.muted).child(`${tr("bots.template_first")}: ${tr(`bots.first_${key(id)}`)}`))
      .child(h_flex().child(this.chip(t, `bot-tpl-use-${id}`, tr("bots.template_use"), false, (cx) => this.newForm(cx, id))));
    return v_flex().flex_1().min_h(0).child(
      v_flex().flex_none().p(10).gap(8).border_b(1).border_color(t.border)
        .child(div().id("bot-search").h(30).px(10).flex().items_center().rounded(R(8)).border_1().border_color(t.border)
          .on_mouse_down("left", (_e, cx) => cx.stop_propagation()).child(Input.new(this.searchInput).h_full().text_size(12).text_color(t.foreground)))
        .child(h_flex().gap(6).flex_wrap().children(["all", ...CATEGORIES].map((c) => this.chip(t, `bot-cat-${c}`, tr(`bots.cat_${c}`), g.category === c, () => { g.category = c; }))))
        .child(div().text_size(11).text_color(t.dim).child(tr("bots.templates_note"))))
      .child(div().flex_1().min_h(0).relative()
        .child(v_flex().id("bots-templates").absolute().inset_0().overflow_y_scroll().p(8).gap(8)
          .children(found.map(card))
          .when(found.length === 0, (el) => el.child(v_flex().gap(8).p(8)
            .child(div().text_size(12).text_color(t.muted).child(tr("bots.templates_none", { query: g.query })))
            .child(h_flex().child(this.chip(t, "bot-tpl-blank", tr("bots.templates_blank"), false, (cx) => this.newForm(cx, "blank")))))))
        .child(Scrollbar.vertical("bots-templates").absolute().inset_0()));
  }

  formView(t) {
    const f = this.form;
    const taken = this.bots.some((b) => b.name === f.name);
    const valid = NAME.test(f.name) && !taken;
    return v_flex().flex_none().p(10).gap(8).border_b(1).border_color(t.border)
      .child(div().id("bot-name").h(30).px(10).flex().items_center().rounded(R(8)).border_1().border_color(f.name && !valid ? t.blocked : t.border)
        .on_mouse_down("left", (_e, cx) => cx.stop_propagation()).child(Input.new(this.nameInput).h_full().text_size(12).text_color(t.foreground)))
      .when(taken, (el) => el.child(div().text_size(11).text_color(t.blocked).child(tr("bots.name_taken"))))
      .child(h_flex().gap(6).children(Object.keys(TEMPLATES).map((k) => this.chip(t, `bot-tpl-${k}`, tr(`bots.template_${k.replace("-", "_")}`), f.template === k, () => { f.template = k; }))))
      .child(h_flex().gap(6).children(f.installed.map((a) => this.chip(t, `bot-agent-${a.key}`, a.name, f.agent === a.key, () => { f.agent = a.key; }))))
      .child(h_flex().gap(6).items_center()
        .child(this.chip(t, "bot-folder", f.folder, false, (cx) => cx.spawn(async (cx) => { const p = await pickFolder(); if (p) f.folder = p; cx.notify(); })))
        .child(this.chip(t, "bot-yolo", "yolo", f.yolo, () => { f.yolo = !f.yolo; })))
      .when(f.yolo, (el) => el.child(div().text_size(11).text_color(t.blocked).child(tr("bots.yolo_warning"))))
      .child(h_flex().gap(6)
        .child(this.chip(t, "bot-create", tr("bots.create"), valid, (cx) => this.create(cx)))
        .child(this.chip(t, "bot-cancel", tr("settings.cancel"), false, () => { this.form = null; })));
  }

  render(cx) {
    const asked = shared("bots.new") ?? null;
    if (asked !== this.seenNew) { this.seenNew = asked; if (!this.form) this.newForm(cx); }
    const askedTemplates = shared("bots.templates") ?? null;
    if (askedTemplates !== this.seenTemplates) { this.seenTemplates = askedTemplates; this.openGallery(cx); }
    const t = theme();
    return v_flex().id("bots").size_full().min_h(0).rounded(R(10)).overflow_hidden().bg(t.background).border_1().border_color(t.border).text_color(t.foreground).text_size(13)
      .child(h_flex().h(42).flex_none().px(14).gap(8).items_center().border_b(1).border_color(t.border)
        .child(div().font_semibold().child(tr("tabs.bots"))).child(div().flex_1())
        .child(this.chip(t, "bot-templates", tr("bots.templates"), Boolean(this.gallery), (cx) => this.gallery ? (this.gallery = null) : this.openGallery(cx)))
        .child(this.chip(t, "bot-new", tr("bots.new"), false, (cx) => this.newForm(cx)))
        .child(div().id("panel-close").size(24).flex_none().flex().items_center().justify_center().rounded(R(6)).text_color(t.muted).cursor_pointer().hover((s) => s.bg(t.hover)).tooltip(tr("menu.close")).on_click(() => run("layout.hide", { id: "panel" })).child(svg("mods/pane/close.svg").size(13))))
      .when(Boolean(this.form), (el) => el.child(this.formView(t)))
      .when(Boolean(this.gallery), (el) => el.child(this.galleryView(t)))
      .when(!this.gallery, (el) => el.child(div().flex_1().min_h(0).relative()
        .child(v_flex().id("bots-list").absolute().inset_0().overflow_y_scroll().p(8).gap(8).children(this.bots.map((b) => this.card(t, b))))
        .child(Scrollbar.vertical("bots-list").absolute().inset_0())));
  }
}
