import { test, expect } from "bun:test";
import { tr } from "./tr-stub";
import { readFile } from "node:fs/promises";

const source = (await readFile(new URL("../defaults/mods/workspace-chrome/main.js", import.meta.url), "utf8"))
  .replace(/^import .*;\n/gm, "").replace("export default class WorkspaceChrome", "class WorkspaceChrome");

function harness(root: unknown, platform = "linux") {
  const calls: unknown[] = [];
  const WorkspaceChrome = new Function("View", "layout", "run", "window", "shared", "setShared", "props", "tr", `${source}; return WorkspaceChrome;`)(
    class {}, () => root, (...args: unknown[]) => calls.push(args), () => ({ platform }), () => undefined, () => {}, () => ({}), tr,
  );
  return { chrome: new WorkspaceChrome(), calls };
}

const tree = (sidebar: number, brand: number, hidden = false) => ({
  id: "root",
  children: [
    { id: "topbar", children: [{ id: "brand", size: { px: brand } }] },
    { id: "body", children: [{ id: "sidebar", size: { px: sidebar }, hidden }] },
  ],
});

test("the brand follows a dragged sidebar", () => {
  const { chrome, calls } = harness(tree(320, 268));
  chrome.syncBrand();
  expect(calls).toEqual([["layout.resize", { id: "brand", px: 320 }]]);
});

test("equal widths emit nothing", () => {
  const { chrome, calls } = harness(tree(268, 268));
  chrome.syncBrand();
  expect(calls).toEqual([]);
});

test("a folded sidebar leaves room for the window buttons by platform", () => {
  const mac = harness(tree(44, 268), "macos");
  mac.chrome.syncBrand();
  expect(mac.calls).toEqual([["layout.resize", { id: "brand", px: 178 }]]);
  const linux = harness(tree(44, 268));
  linux.chrome.syncBrand();
  expect(linux.calls).toEqual([["layout.resize", { id: "brand", px: 108 }]]);
});

test("a hidden sidebar narrows the brand", () => {
  const { chrome, calls } = harness(tree(268, 268, true));
  chrome.syncBrand();
  expect(calls).toEqual([["layout.resize", { id: "brand", px: 108 }]]);
});

test("layouts without a brand are left alone", () => {
  const { chrome, calls } = harness({ id: "root", children: [{ id: "sidebar", size: { px: 320 } }] });
  chrome.syncBrand();
  expect(calls).toEqual([]);
});

test("the divider fades from the border's own alpha, so theme colors with alpha stay valid", () => {
  const fadeColor = new Function("View", `${source}; return fadeColor;`)(class {});
  expect(fadeColor("#ffffff24", 1)).toBe("#ffffff33"); // a faint border is lifted to a visible floor
  expect(fadeColor("#ffffff60", 1)).toBe("#ffffff60");
  expect(fadeColor("#ffffff24", 0)).toBe("#ffffff00");
  expect(fadeColor("#808080", 1)).toBe("#808080ff");
  expect(fadeColor("#808080", 0.5)).toBe("#80808080");
  for (const c of ["#ffffff24", "#808080"]) for (const k of [0, 0.25, 0.5, 1]) expect(fadeColor(c, k)).toMatch(/^#[0-9a-f]{8}$/);
});

function toggler(sidebarPx: number, stored: Record<string, unknown>) {
  const calls: unknown[] = [];
  const root = { id: "root", children: [{ id: "sidebar", size: { px: sidebarPx } }] };
  const toggleSidebar = new Function("View", "layout", "run", "window", "shared", "setShared", `${source}; return toggleSidebar;`)(
    class {}, () => root, (...args: unknown[]) => calls.push(args), () => ({ platform: "linux" }),
    (key: string) => stored[key], (key: string, value: unknown) => { stored[key] = value; },
  );
  return { toggleSidebar, calls, stored };
}

test("the sidebar button folds to the rail and remembers the width", () => {
  const { toggleSidebar, calls, stored } = toggler(320, {});
  toggleSidebar();
  expect(calls).toEqual([["layout.resize", { id: "sidebar", px: 44 }]]);
  expect(stored["sidebar.width"]).toBe(320);
  expect(stored["sidebar.collapsed"]).toBe(true);
});

test("the sidebar button unfolds to the remembered width, or the default", () => {
  const remembered = toggler(44, { "sidebar.width": 320 });
  remembered.toggleSidebar();
  expect(remembered.calls).toEqual([["layout.resize", { id: "sidebar", px: 320 }]]);
  expect(remembered.stored["sidebar.collapsed"]).toBe(false);
  const fresh = toggler(44, {});
  fresh.toggleSidebar();
  expect(fresh.calls).toEqual([["layout.resize", { id: "sidebar", px: 268 }]]);
});
