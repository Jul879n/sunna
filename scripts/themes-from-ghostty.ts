// Writes the shipped themes (defaults/themes) from Ghostty's theme folder:
//   bun scripts/themes-from-ghostty.ts "/Applications/Ghostty.app/Contents/Resources/ghostty/themes"
// Ghostty's themes come from iTerm2-Color-Schemes (MIT, see licenses/iTerm2-Color-Schemes.txt).
import { resolve, join } from "node:path";
import { fromGhostty, toThemeFile } from "../defaults/frame/settings/theme-import.js";

const SOURCES: Record<string, string> = {
  "Catppuccin Mocha": "catppuccin-mocha",
  "Catppuccin Latte": "catppuccin-latte",
  "Dracula": "dracula",
  "Nord": "nord",
  "TokyoNight Night": "tokyo-night",
  "TokyoNight Day": "tokyo-night-day",
  "Gruvbox Dark": "gruvbox-dark",
  "Gruvbox Light": "gruvbox-light",
  "Rose Pine": "rose-pine",
  "Rose Pine Dawn": "rose-pine-dawn",
  "GitHub Dark Default": "github-dark",
  "GitHub Light Default": "github-light",
};

const folder = process.argv[2];
if (!folder) throw new Error("usage: bun scripts/themes-from-ghostty.ts <ghostty themes folder>");
const out = resolve(import.meta.dir, "../defaults/themes");
for (const [source, name] of Object.entries(SOURCES)) {
  const theme = fromGhostty(await Bun.file(join(folder, source)).text());
  await Bun.write(join(out, `${name}.json`), toThemeFile(theme));
  console.log(`${source} -> defaults/themes/${name}.json`);
}
