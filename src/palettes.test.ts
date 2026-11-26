import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { PALETTES, TERM_COLORS } from "./palettes.ts";

// Os contrastes do DESIGN.md, em toda paleta, de noite e de dia, lidos do próprio styles.scss.
const css = readFileSync(new URL("styles.scss", import.meta.url), "utf8");
const block = (selector: string) => {
  const at = css.indexOf(`${selector} {`);
  assert.ok(at >= 0, `sem o bloco ${selector}`);
  const body = css.slice(at, css.indexOf("\n}", at));
  return Object.fromEntries([...body.matchAll(/--([\w-]+):\s*(#[0-9a-f]{6});/gi)].map((m) => [m[1], m[2]]));
};
const base = { dark: block(":root,\n[data-palette]"), light: { ...block(":root,\n[data-palette]"), ...block(':root[data-theme="light"],\n[data-palette][data-theme="light"]') } };
const tokens = (p: string, mode: "dark" | "light") =>
  p === "hotswap" ? base[mode] : { ...base[mode], ...block(`[data-palette="${p}"]` + (mode === "dark" ? ':not([data-theme="light"])' : '[data-theme="light"]')) };

const lum = (hex: string) => {
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255).map((v) => (v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
};
const ratio = (a: string, b: string) => {
  const [x, y] = [lum(a), lum(b)].sort((m, n) => n - m);
  return (x + 0.05) / (y + 0.05);
};

for (const p of PALETTES)
  for (const mode of ["dark", "light"] as const)
    test(`${p} (${mode === "dark" ? "noite" : "dia"}): contraste AA`, () => {
      const t = tokens(p, mode);
      const at = (fg: string, bgs: string[], min: number) => {
        for (const bg of bgs) assert.ok(ratio(t[fg], t[bg]) >= min, `${fg} sobre ${bg}: ${ratio(t[fg], t[bg]).toFixed(2)} < ${min}`);
      };
      const surfaces = ["board", "rail", "flap", "flap-hi"];
      at("ink", surfaces, 4.5);
      at("ink-2", surfaces, 4.5);
      at("ink-3", surfaces, 3);
      for (const s of ["run", "start", "fail"]) at(s, ["board", "rail"], 4.5);
      if (mode === "dark") at("board", ["body-move-top", "body-move-bot"], 4.5);
      else {
        at("flap-ink", ["flap-top", "flap-low"], 4.5);
        at("ink", ["body-run-top", "body-run-bot", "body-move-top", "body-move-bot", "body-fail-top", "body-fail-bot"], 4.5);
        at("ink-stop", ["body-stop-top", "body-stop-bot"], 4.5);
        at("start", ["ends-bg"], 4.5);
      }
      // ANSI: conteúdo, mas lido o dia todo; o black é o próprio fundo por convenção
      for (const [name, color] of Object.entries(TERM_COLORS[p][mode]))
        if (name !== "black") assert.ok(ratio(color, t.board) >= 4.5, `ANSI ${name} ${color} sobre o fundo: ${ratio(color, t.board).toFixed(2)}`);
    });
