import { test, expect } from "bun:test";
import { tr } from "./tr-stub";
import { readFile } from "node:fs/promises";
import { loadModule } from "./mod-module";

const m = await loadModule(new URL("../defaults/mods/welcome/model.js", import.meta.url), {});
const bound = [
  { section: "global", keys: "cmd-n", action: "space.new" },
  { section: "global", keys: "cmd-shift-up", action: "space.prev" },
  { section: "global", keys: "cmd-shift-down", action: "space.next" },
  { section: "global", keys: "cmd-t", action: "tab.new" },
];

test("shortcuts come from the keys list, in order, with readable key caps", () => {
  const list = m.shortcuts(bound, tr);
  expect(list.map((s: any) => s.action)).toEqual(["space.new", "space.prev", "space.next"]);
  expect(list[0]).toEqual({ action: "space.new", label: "Create workspace", parts: ["⌘", "N"] });
  expect(list[2].parts).toEqual(["⌘", "⇧", "↓"]);
});

test("changed keys are shown as changed; unbound actions are left out", () => {
  const list = m.shortcuts([{ keys: "ctrl-alt-n", action: "space.new" }], tr);
  expect(list).toHaveLength(1);
  expect(list[0].parts).toEqual(["⌃", "⌥", "N"]);
  expect(m.shortcuts([], tr)).toEqual([]);
  expect(m.shortcuts(undefined, tr)).toEqual([]);
});

test("every text the welcome screen shows exists in both locales", async () => {
  const keys = [...(await readFile(new URL("../defaults/mods/welcome/main.js", import.meta.url), "utf8")).matchAll(/tr\("([\w.]+)"\)/g)].map((x) => x[1])
    .concat(m.SHORTCUTS.map((s: string[]) => s[1]), ["welcome.empty_changes", "welcome.empty_files", "mods.welcome.description"]);
  for (const lang of ["en", "es"]) {
    const table = JSON.parse(await readFile(new URL(`../defaults/locales/${lang}.json`, import.meta.url), "utf8"));
    for (const k of keys) expect(table[k], `${lang}:${k}`).toBeTruthy();
  }
});

test("the empty-state wording is the approved Spanish", async () => {
  const es = JSON.parse(await readFile(new URL("../defaults/locales/es.json", import.meta.url), "utf8"));
  expect(es["welcome.empty_subtitle"]).toBe("Selecciona un espacio de trabajo en la barra lateral para comenzar.");
  expect(es["welcome.empty_add_project"]).toBe("Agregar proyecto");
  expect(es["welcome.empty_new_space"]).toBe("Crear espacio de trabajo");
  expect(es["welcome.empty_changes"]).toBe("Seleccione un espacio de trabajo para ver los cambios");
});
