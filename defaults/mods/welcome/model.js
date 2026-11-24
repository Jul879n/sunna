// What the welcome screen lists: the shortcuts read from keys.json, never fixed.
export const SHORTCUTS = [
  ["space.new", "welcome.empty_key_new"],
  ["space.prev", "welcome.empty_key_prev"],
  ["space.next", "welcome.empty_key_next"],
];

const SYMBOLS = { cmd: "⌘", shift: "⇧", alt: "⌥", ctrl: "⌃", up: "↑", down: "↓", left: "←", right: "→", enter: "↵", escape: "Esc" };

// "cmd-shift-down" -> ["⌘", "⇧", "↓"]; a plain letter is shown capitalized.
export function keyParts(keys) {
  return String(keys).split("-").filter(Boolean).map((k) => SYMBOLS[k] ?? (k.length === 1 ? k.toUpperCase() : k));
}

// The first key bound to each action in `list` (the `keys()` of the mod API); unbound ones are left out.
export function shortcuts(list, label) {
  return SHORTCUTS.flatMap(([action, text]) => {
    const found = (list ?? []).find((k) => k.action === action);
    return found ? [{ action, label: label(text), parts: keyParts(found.keys) }] : [];
  });
}
