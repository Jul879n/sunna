import { test, expect } from "bun:test";
import { fromGhostty, fromWarp, fromVSCode, detect, slug, toThemeFile } from "../defaults/frame/settings/theme-import.js";

const DRACULA = `palette = 0=#21222c
palette = 1=#ff5555
palette = 2=#50fa7b
palette = 3=#f1fa8c
palette = 4=#bd93f9
palette = 5=#ff79c6
palette = 6=#8be9fd
palette = 7=#f8f8f2
palette = 8=#6272a4
palette = 9=#ff6e6e
palette = 10=#69ff94
palette = 11=#ffffa5
palette = 12=#d6acff
palette = 13=#ff92df
palette = 14=#a4ffff
palette = 15=#ffffff
background = #282a36
foreground = #f8f8f2
cursor-color = #f8f8f2
cursor-text = #282a36
selection-background = #44475a
selection-foreground = #ffffff
`;

test("ghostty: the Dracula file", () => {
  const t = fromGhostty(DRACULA);
  expect(t.background).toBe("#282a36");
  expect(t.ansi[4]).toBe("#bd93f9");
  expect(t.selection).toBe("#44475a");
  expect(t.cursor).toBe("#f8f8f2");
  expect(t.ansi).toHaveLength(16);
  expect(JSON.parse(toThemeFile(t))).toEqual(t);
});

test("warp: normal and bright make 16 ansi, accent goes to colors", () => {
  const names = ["black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"];
  const block = (n: string, base: number) => names.map((c, i) => `    ${c}: '#${(base + i).toString(16).padStart(2, "0").repeat(3)}'`).join("\n");
  const t = fromWarp(`# a comment\naccent: "#ff8800"\nbackground: '#101010'\nforeground: '#eeeeee'\nterminal_colors:\n  normal:\n${block("n", 16)}\n  bright:\n${block("b", 32)}\n`);
  expect(t.ansi).toHaveLength(16);
  expect(t.ansi[0]).toBe("#101010");
  expect(t.ansi[8]).toBe("#202020");
  expect(t.colors.accent).toBe("#ff8800");
  expect(t.selection).toBe(t.ansi[8]);
});

const vscode = (omitAnsi = false) => {
  const names = ["Black", "Red", "Green", "Yellow", "Blue", "Magenta", "Cyan", "White"];
  const ansi = omitAnsi ? "" : names.map((n, i) => `"terminal.ansi${n}": "#0${i}0000", "terminal.ansiBright${n}": "#1${i}0000",`).join("\n");
  return `{
  // a theme
  "name": "x", /* block */
  "colors": {
    "editor.background": "#1e1e1e", "editor.foreground": "#ccc",
    "focusBorder": "#007acc88", "sideBar.background": "#252526",
    ${ansi}
  },
}`;
};

test("vscode: comments and trailing commas", () => {
  const t = fromVSCode(vscode());
  expect(t.background).toBe("#1e1e1e");
  expect(t.foreground).toBe("#cccccc");
  expect(t.ansi).toHaveLength(16);
  expect(t.ansi[9]).toBe("#110000");
  expect(t.colors.accent).toBe("#007acc88");
  expect(t.colors.chrome).toBe("#252526");
});

test("vscode without terminal.ansi* asks for the missing color", () => {
  expect(() => fromVSCode(vscode(true))).toThrow("settings.import_missing_ansi");
});

test("helpers", () => {
  expect(detect("a.JSON")).toBe("vscode");
  expect(detect("a.yml")).toBe("warp");
  expect(detect("Dracula")).toBe("ghostty");
  expect(slug("Tokyo Night Day!")).toBe("tokyo-night-day");
  expect(() => fromGhostty("palette = 0=#000000\n")).toThrow("settings.import_missing_ansi");
});
