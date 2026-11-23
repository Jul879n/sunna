import { expect, test } from "bun:test";
import { readFile } from "node:fs/promises";
import { loadModule } from "./mod-module";

const templates = await loadModule(new URL("../defaults/mods/bots/templates.js", import.meta.url), {});
const source = await readFile(new URL("../defaults/mods/bots/main.js", import.meta.url), "utf8");

const files: Record<string, string> = {};
const runs: any[] = [];
const toasts: any[] = [];
let ghOut = "";
let ghCode = 0;
let working = false;
const agent = { key: "claude", installed: true, base: ["claude"], yolo: ["claude", "--yolo"] };
const values: Record<string, any> = {
  View: class {}, div: () => ({}), svg: () => ({}), v_flex: () => ({}), h_flex: () => ({}), Scrollbar: {}, InputState: { new: () => ({ on() {} }) }, Input: {},
  spaces: () => [], agents: () => working ? [{ tab_name: "bot b #1", status: "working" }] : [],
  run: (...a: any[]) => runs.push(a), exec: async () => ({ code: ghCode, stdout: ghOut, stderr: "" }),
  knownAgents: async () => [agent], readTextFile: async (p: string) => files[p] ?? null,
  writeTextFile: async (p: string, t: string) => { files[p] = t; }, pickFolder: async () => null,
  notify: (n: any) => toasts.push(n), setShared() {}, appHome: () => "/h", home: () => "/home/u", checking: () => false,
  theme: () => ({}), tr: (k: string, v: any = {}) => k + JSON.stringify(v), style: () => ({ radius: 6 }),
  readdir: async () => [], mkdir: async () => {},
};
const body = source.replace(/^import .*;\n/gm, "").replace("export default class Bots", "return class Bots");
const Bots = new Function(...Object.keys(values), ...Object.keys(templates), body)(...Object.values(values), ...Object.values(templates));

const make = (config: any = {}, state: any = { runs: [], seen: [], last_poll: {} }) => {
  const v = new Bots();
  v.launching = new Set();
  return { v, bot: { name: "b", soul: "Haz algo.\n", config: { agent: "claude", folder: "/r", enabled: true, yolo: false, max_runs_per_day: 10, triggers: [{ type: "manual" }], ...config }, state } };
};
const reset = () => { runs.length = 0; toasts.length = 0; Object.keys(files).forEach((k) => delete files[k]); working = false; ghCode = 0; };

test("an interval trigger that is due starts a run without taking focus", async () => {
  reset();
  const { v, bot } = make({ triggers: [{ type: "interval", minutes: 60 }] }, { runs: [{ id: 1, at: new Date(Date.now() - 61 * 60000).toISOString(), trigger: "interval", tab: "bot b #1" }], seen: [], last_poll: {} });
  const due = templates.dueTriggers(bot.config, bot.state, Date.now());
  expect(due).toHaveLength(1);
  await v.launch(bot, "interval", "");
  expect(runs).toHaveLength(1);
  expect(runs[0][0]).toBe("space.new");
  expect(runs[0][1]).toMatchObject({ name: "bot: b", tab: "bot b #2", folder: "/r", argv: ["claude"], focus: false });
  expect(JSON.parse(files["/h/bots/b/runs.json"]).runs.at(-1).tab).toBe("bot b #2");
});

test("github: the first poll only learns what exists, the next launches the new item", async () => {
  reset();
  const { v, bot } = make({ triggers: [{ type: "github", query: "q", every_minutes: 10 }] });
  ghOut = "https://x/1\tone\n";
  await v.poll(bot, bot.config.triggers[0], Date.now());
  expect(runs).toHaveLength(0);
  ghOut = "https://x/1\tone\nhttps://x/2\ttwo\n";
  await v.poll(bot, bot.config.triggers[0], Date.now() + 1000);
  expect(runs).toHaveLength(1);
  expect(runs[0][1].task).toContain("https://x/2");
});

test("yolo is ignored for a github trigger", async () => {
  reset();
  const { v, bot } = make({ yolo: true });
  await v.launch(bot, "github", "item");
  expect(runs.at(-1)![1].argv).toEqual(["claude"]);
  await v.launch({ ...bot, state: { runs: [], seen: [], last_poll: {} } }, "manual", "");
  expect(runs.at(-1)![1].argv).toEqual(["claude", "--yolo"]);
});

test("max_runs_per_day stops further runs, and so does a working agent", async () => {
  reset();
  const { v, bot } = make({ max_runs_per_day: 1 });
  expect(await v.launch(bot, "manual", "")).toBe(true);
  expect(await v.launch(bot, "manual", "")).toBe(false);
  reset();
  const other = make({ max_runs_per_day: 5 });
  working = false;
  expect(await other.v.launch(other.bot, "manual", "")).toBe(true);
  working = true;
  expect(await other.v.launch(other.bot, "manual", "")).toBe(false);
  expect(runs).toHaveLength(1);
});

test("a daily trigger fires once after its hour", () => {
  const bot = { triggers: [{ type: "daily", at: "09:00" }] };
  const day = new Date(); day.setHours(10, 0, 0, 0);
  expect(templates.dueTriggers(bot, { runs: [] }, day.getTime())).toHaveLength(1);
  expect(templates.dueTriggers(bot, { runs: [{ at: new Date(day.getTime() - 1000).toISOString(), trigger: "daily" }] }, day.getTime())).toHaveLength(0);
  const early = new Date(); early.setHours(8, 0, 0, 0);
  expect(templates.dueTriggers(bot, { runs: [] }, early.getTime())).toHaveLength(0);
});

test("a failed first poll does not count as having learned what exists", async () => {
  reset();
  const { v, bot } = make({ triggers: [{ type: "github", query: "q", every_minutes: 10 }] });
  ghCode = 1;
  await v.poll(bot, bot.config.triggers[0], Date.now());
  expect(bot.state.learned).toBeFalsy();
  ghCode = 0;
  ghOut = "https://x/1\tone\n";
  await v.poll(bot, bot.config.triggers[0], Date.now() + 1000);
  expect(runs).toHaveLength(0); // the first good look only learns
  expect(bot.state.learned).toBe(true);
});

test("an item refused by the daily cap is not marked seen, so it is tried again", async () => {
  reset();
  const { v, bot } = make({ max_runs_per_day: 1, triggers: [{ type: "github", query: "q", every_minutes: 10 }] }, { runs: [], seen: [], last_poll: {}, learned: true });
  ghOut = "https://x/1\tone\nhttps://x/2\ttwo\n";
  await v.poll(bot, bot.config.triggers[0], Date.now());
  expect(runs).toHaveLength(1);
  expect(bot.state.seen).toEqual(["https://x/1"]);
});

test("a folder with ~ is expanded before it is used", () => {
  const { v, bot } = make({ folder: "~/dev/p" });
  expect(v.cwd(bot)).toBe("/home/u/dev/p");
  bot.config.folder = "~";
  expect(v.cwd(bot)).toBe("/home/u");
});

test("templates: category and search filter them, and ones with a trigger start switched off", () => {
  const label = (id: string) => id.replace(/-/g, " ");
  const all = templates.filterTemplates("", "all", label);
  expect(all.length).toBe(Object.keys(templates.TEMPLATES).length);
  expect(templates.filterTemplates("", "releases", label).every(([, t]: any) => t.category === "releases")).toBe(true);
  expect(templates.filterTemplates("CI fail", "all", label).map(([id]: any) => id)).toEqual(["ci-failures"]);
  expect(templates.filterTemplates("zzz", "all", label)).toEqual([]);
  expect(templates.startsOff(templates.TEMPLATES["pr-review"])).toBe(true);
  expect(templates.startsOff(templates.TEMPLATES.blank)).toBe(false);
});

test("every template has a known category", () => {
  for (const t of Object.values<any>(templates.TEMPLATES)) expect(templates.CATEGORIES).toContain(t.category);
});
