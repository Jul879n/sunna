import { test, expect } from "bun:test";
import { readFile } from "node:fs/promises";

const source = await readFile(new URL("../defaults/mods/changes/main.js", import.meta.url), "utf8");
const rowsSource = source.slice(source.indexOf("const untab ="), source.indexOf("\nexport default class"));
const diffRows = new Function(`${rowsSource}; return diffRows;`)();

test("Changes keeps tab indentation, to the next 4-column stop", () => {
  const rows = diffRows(["@@ -1,2 +1,2 @@", "-\tif x {", "+\t\tif x {", " a\tb", "+    spaces"]);
  expect(rows.map((r: { text: string }) => r.text)).toEqual(["@@ -1,2 +1,2 @@", "    if x {", "        if x {", "a   b", "    spaces"]);
});

// The git bar: branch, ahead/behind, push, history. A stand-in `exec` answers by git subcommand.
class El {
  nodes: any[] = [];
  calls: Record<string, any[]> = {};
  constructor() {
    const proxy: any = new Proxy(this, {
      get: (t, k: string) => k in t ? Reflect.get(t, k) : (...a: any[]) => {
        if (k === "child") t.nodes.push(a[0]);
        else if (k === "children") t.nodes.push(...a[0]);
        else if (k === "when") { if (a[0]) a[1](proxy); }
        else t.calls[k] = a;
        return proxy;
      },
    });
    return proxy;
  }
}
const texts = (e: any, out: any[] = []): any[] => {
  if (!e || typeof e !== "object") { out.push(e); return out; }
  e.nodes.forEach((n: any) => texts(n, out));
  return out;
};
const byId = (e: any, id: string): any => {
  if (!e || typeof e !== "object") return null;
  if (e.calls?.id?.[0] === id) return e;
  for (const n of e.nodes) { const r = byId(n, id); if (r) return r; }
  return null;
};
const pending: Promise<unknown>[] = [];
const cx = { notify() {}, spawn: (f: any) => pending.push(f(cx)), timer: { every() {} }, focus_handle: () => ({}) };
const settle = async () => { while (pending.length) await pending.shift(); };
const execLog: string[][] = [];
let answer: (args: string[]) => { code: number; stdout: string; stderr?: string };
const toasts: any[] = [];
const modValues: Record<string, any> = {
  tr: (k: string, v: any = {}) => k.replace(/\{(\w+)\}/g, (_: string, n: string) => v[n]) + (Object.keys(v).length ? "" : ""),
  View: class {}, div: () => new El(), v_flex: () => new El(), h_flex: () => new El(), Scrollbar: { vertical: () => new El() }, v_virtual_list: () => new El(),
  InputState: { new: () => ({ on() {}, value: () => "", set_value() {} }) }, Input: { new: () => new El() },
  spaces: () => [{ id: "s", active: true, folder: "/repo" }], panes: () => [], focused: () => null,
  exec: async (_c: string, args: string[]) => { execLog.push(args); const r = answer(args.filter((a) => !a.startsWith("-c") && a !== "core.fsmonitor=false" && a !== "diff.autoRefreshIndex=false" && a !== "--no-optional-locks")); return { stderr: "", ...r }; },
  readTextFile: async () => null, watchFolder: async () => { throw new Error("no watcher in tests"); }, notify: (n: any) => toasts.push(n),
  theme: () => new Proxy({}, { get: () => "#000000" }), settings: () => ({ font_family: "mono" }), style: () => ({ radius: 6 }),
};
const ChangesView = new Function(...Object.keys(modValues), source.replace(/^import .*;\n/gm, "").replace("export default class Changes", "return class Changes"))(...Object.values(modValues));

async function opened(counts: string | null, branch = "main") {
  execLog.length = 0; toasts.length = 0;
  answer = (a) => {
    if (a[0] === "rev-parse" && a[1] === "--show-toplevel") return { code: 0, stdout: "/repo\n" };
    if (a[0] === "rev-parse") return { code: 0, stdout: "abc\n" };
    if (a[0] === "branch" && a[1] === "--show-current") return { code: 0, stdout: branch + "\n" };
    if (a[0] === "rev-list") return counts === null ? { code: 128, stdout: "", stderr: "no upstream" } : { code: 0, stdout: counts + "\n" };
    if (a[0] === "log") return { code: 0, stdout: "a1b2c3d\x1ffirst\x1fAna\x1f2 hours ago\nd4e5f6a\x1fsecond\x1fLuis\x1f1 day ago\n" };
    return { code: 0, stdout: "" };
  };
  const v = new ChangesView();
  v.init({}, cx);
  await settle();
  return v;
}

test("the bar shows how far the branch is from its upstream", async () => {
  const v = await opened("2\t1");
  expect(texts(v.gitBar(modValues.theme())).flat()).toContain("↑1 ↓2");
});

test("push without an upstream sets one on origin", async () => {
  const v = await opened(null, "feature");
  execLog.length = 0;
  byId(v.gitBar(modValues.theme()), "git-push").calls.on_click[0]({}, cx);
  await settle();
  expect(execLog.find((a) => a[0] === "push")).toEqual(["push", "-u", "origin", "feature"]);
});

test("the history tab lists the last commits", async () => {
  const v = await opened("0\t0");
  v.showView("history", cx);
  await settle();
  const list = v.historyList(modValues.theme());
  expect(v.commits).toHaveLength(2);
  expect(byId(list, "git-c-0")).not.toBeNull();
  expect(byId(list, "git-c-1")).not.toBeNull();
  expect(byId(list, "git-c-2")).toBeNull();
});

test("a git failure that is not 'not a repository' shows git's own message", async () => {
  execLog.length = 0;
  answer = (a) => a[0] === "rev-parse" ? { code: 128, stdout: "", stderr: "fatal: cannot change to '/repo': Operation not permitted\n" } : { code: 0, stdout: "" };
  const v = new ChangesView();
  v.init({}, cx);
  await settle();
  expect(v.error).toBe("fatal: cannot change to '/repo': Operation not permitted");
  answer = (a) => a[0] === "rev-parse" ? { code: 128, stdout: "", stderr: "fatal: not a git repository (or any of the parent directories): .git\n" } : { code: 0, stdout: "" };
  const w = new ChangesView();
  w.init({}, cx);
  await settle();
  expect(w.error).toBe("changes.not_a_git_repository");
});
