// Theme importers: Ghostty, Warp and VS Code theme files to Herdr's `themes/<name>.json`.
// Pure functions (no `herdr` imports). Errors carry a translation key in `message`
// and its variables in `vars`.

const fail = (key, vars = {}) => Object.assign(new Error(key), { vars });

/** `#rgb` / `#rrggbb` / `#rrggbbaa` (with or without `#`) as lowercase `#rrggbb[aa]`, or null. */
function hex(v) {
  if (typeof v !== "string") return null;
  let h = v.trim().replace(/^#/, "").toLowerCase();
  if (/^[0-9a-f]{3}$/.test(h)) h = [...h].map((c) => c + c).join("");
  else if (/^[0-9a-f]{4}$/.test(h)) h = [...h].map((c) => c + c).join("");
  return /^([0-9a-f]{6}|[0-9a-f]{8})$/.test(h) ? "#" + h : null;
}

/** Terminal keys take exactly 6 digits: drop an alpha channel. */
const term = (v) => { const h = hex(v); return h ? h.slice(0, 7) : null; };

const ANSI_NAMES = ["black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"];

function finish({ background, foreground, selection, cursor, ansi, colors }) {
  for (let i = 0; i < 16; i++) if (!ansi[i]) throw fail("settings.import_missing_ansi", { index: i });
  const bg = term(background);
  if (!bg) throw fail("settings.import_no_background");
  const theme = {
    background: bg,
    foreground: term(foreground) ?? ansi[7],
    selection: term(selection) ?? ansi[8],
  };
  const c = term(cursor);
  if (c) theme.cursor = c;
  theme.ansi = ansi;
  const keep = {};
  for (const [k, v] of Object.entries(colors ?? {})) {
    const h = hex(v);
    if (h) keep[k] = h;
  }
  if (Object.keys(keep).length) theme.colors = keep;
  return theme;
}

export function fromGhostty(text) {
  const ansi = Array(16).fill(null);
  const o = {};
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim();
    if (!line || line.startsWith("#")) continue;
    const eq = line.indexOf("=");
    if (eq < 0) continue;
    const key = line.slice(0, eq).trim();
    const value = line.slice(eq + 1).trim();
    if (key === "palette") {
      const m = value.match(/^(\d+)\s*=\s*(\S+)/);
      if (m && +m[1] < 16) ansi[+m[1]] = term(m[2]);
    } else o[key] = value;
  }
  return finish({
    background: o.background, foreground: o.foreground, selection: o["selection-background"],
    cursor: o["cursor-color"], ansi, colors: {},
  });
}

/** A minimal YAML reader: nested `key: value` by indentation. */
function yaml(text) {
  const root = {};
  const stack = [{ indent: -1, node: root }];
  for (const raw of text.split(/\r?\n/)) {
    if (!raw.trim() || raw.trim().startsWith("#")) continue;
    const indent = raw.match(/^ */)[0].length;
    const m = raw.trim().match(/^([\w.-]+)\s*:\s*(.*)$/);
    if (!m) continue;
    while (stack.length > 1 && stack.at(-1).indent >= indent) stack.pop();
    const parent = stack.at(-1).node;
    const value = m[2].replace(/\s+#.*$/, "").trim().replace(/^(['"])(.*)\1$/, "$2");
    if (value === "") {
      const node = {};
      parent[m[1]] = node;
      stack.push({ indent, node });
    } else parent[m[1]] = value;
  }
  return root;
}

export function fromWarp(text) {
  const y = yaml(text);
  const tc = y.terminal_colors ?? {};
  const ansi = Array(16).fill(null);
  ANSI_NAMES.forEach((n, i) => {
    ansi[i] = term(tc.normal?.[n]);
    ansi[i + 8] = term(tc.bright?.[n]);
  });
  const accent = typeof y.accent === "string" ? y.accent : y.accent?.color;
  return finish({
    background: typeof y.background === "string" ? y.background : y.background?.color,
    foreground: y.foreground, selection: null, cursor: y.cursor, ansi, colors: { accent },
  });
}

/** JSON with comments and trailing commas (VS Code's flavor) to plain JSON. */
function relaxJson(text) {
  let out = "";
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (c === '"') {
      let j = i + 1;
      while (j < text.length && text[j] !== '"') j += text[j] === "\\" ? 2 : 1;
      out += text.slice(i, j + 1);
      i = j;
    } else if (c === "/" && text[i + 1] === "/") {
      while (i < text.length && text[i] !== "\n") i++;
      out += "\n";
    } else if (c === "/" && text[i + 1] === "*") {
      const end = text.indexOf("*/", i + 2);
      i = end < 0 ? text.length : end + 1;
    } else out += c;
  }
  return out.replace(/^﻿/, "").replace(/,(\s*[}\]])/g, "$1");
}

export function fromVSCode(text) {
  const doc = JSON.parse(relaxJson(text));
  const c = doc.colors ?? {};
  const cap = (s) => s[0].toUpperCase() + s.slice(1);
  const ansi = [
    ...ANSI_NAMES.map((n) => term(c[`terminal.ansi${cap(n)}`])),
    ...ANSI_NAMES.map((n) => term(c[`terminal.ansiBright${cap(n)}`])),
  ];
  return finish({
    background: c["terminal.background"] ?? c["editor.background"],
    foreground: c["terminal.foreground"] ?? c["editor.foreground"],
    selection: c["terminal.selectionBackground"] ?? c["editor.selectionBackground"],
    cursor: c["terminalCursor.foreground"],
    ansi,
    colors: {
      accent: c.focusBorder ?? c["button.background"],
      chrome: c["sideBar.background"],
      border: c["panel.border"] ?? c["editorGroup.border"],
    },
  });
}

/** The importer for a file name: `.json` VS Code, `.yaml`/`.yml` Warp, anything else Ghostty. */
export function detect(fileName) {
  const n = fileName.toLowerCase();
  if (n.endsWith(".json")) return "vscode";
  if (n.endsWith(".yaml") || n.endsWith(".yml")) return "warp";
  return "ghostty";
}

export const PARSERS = { ghostty: fromGhostty, warp: fromWarp, vscode: fromVSCode };

export function slug(name) {
  return name.toLowerCase().trim().replace(/\s+/g, "-").replace(/[^a-z0-9-]/g, "").replace(/-+/g, "-").replace(/^-|-$/g, "");
}

export function toThemeFile(theme) {
  return JSON.stringify(theme, null, 2) + "\n";
}
