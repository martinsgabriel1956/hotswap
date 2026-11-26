import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { EN } from "./en.ts";
import { setLang, tr } from "./i18n.ts";

const holes = (s: string) => (s.match(/\{\d+\}/g) ?? []).sort().join();

test("tr troca os {n} e cai no português sem tradução", () => {
  setLang("en");
  assert.equal(tr("Reiniciar {0}", "feat/x"), "Restart feat/x");
  assert.equal(tr("texto sem tradução {0}", 1), "texto sem tradução 1");
  setLang("pt");
  assert.equal(tr("Reiniciar {0}", "feat/x"), "Reiniciar feat/x");
});

test("todo tr(\"…\") do código tem tradução, com os mesmos {n}", () => {
  const dir = new URL(".", import.meta.url);
  for (const f of readdirSync(dir).filter((f) => f.endsWith(".ts") && !f.endsWith(".test.ts") && f !== "en.ts" && f !== "i18n.ts")) {
    for (const [, raw] of readFileSync(new URL(f, dir), "utf8").matchAll(/\btr\(("(?:[^"\\]|\\.)*")/g)) {
      const key: string = JSON.parse(raw);
      assert.ok(key in EN, `${f}: sem tradução para ${raw}`);
      assert.equal(holes(EN[key]), holes(key), `${f}: ${raw}`);
    }
  }
});

test("o texto do index.html tem tradução", () => {
  const keep = new Set(["Hotswap", "API", "Front", "Ctrl", "Shift", "Alt", "Enter", "Esc", "Branch", "Git", "Zoom:", "SIGKILL", "parallelEnv", ".bare", "Setup", "npm run", "npm", "yarn", "pnpm", "Português", "English", "Terminal", "Worktrees"]);
  const html = readFileSync(new URL("../index.html", import.meta.url), "utf8").replace(/<script[\s\S]*?<\/script>|<style[\s\S]*?<\/style>|<!--[\s\S]*?-->/g, "");
  const texts = [...html.matchAll(/>([^<>]+)</g)].map((m) => m[1]);
  const attrs = [...html.matchAll(/\s(?:title|placeholder|aria-label)="([^"]+)"/g)].map((m) => m[1]);
  const missing = [...texts, ...attrs].map((s) => s.replace(/\s+/g, " ").trim()).filter((s) => /[A-Za-zÀ-ú]{2,}/.test(s) && !keep.has(s) && !(s in EN));
  assert.deepEqual(missing, []);
});
