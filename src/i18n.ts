import { EN } from "./en.ts";

/** O português é a fonte: `tr("Parar {0}", x)` devolve o texto em português ou a tradução do EN, com {0}, {1}… trocados pelos argumentos. */
export type Lang = "pt" | "en";
export const LANGS: Lang[] = ["pt", "en"];

// Fora do webview (os testes no node) fica em português; no app, a escolha gravada ou o idioma do sistema.
function initial(): Lang {
  if (typeof window === "undefined") return "pt";
  try {
    const saved = JSON.parse(localStorage.getItem("lang") ?? "null");
    if (LANGS.includes(saved)) return saved;
  } catch {}
  return navigator.language.toLowerCase().startsWith("pt") ? "pt" : "en";
}

export let lang: Lang = initial();
export const setLang = (l: Lang) => (lang = l);
export const locale = () => (lang === "en" ? "en-US" : "pt-BR");

export function tr(key: string, ...args: unknown[]): string {
  const s = lang === "en" ? (EN[key] ?? key) : key;
  return args.length ? s.replace(/\{(\d+)\}/g, (m, i) => (+i < args.length ? String(args[+i]) : m)) : s;
}

/** Traduz o HTML estático (index.html): os textos e os atributos que se leem, comparados sem os espaços das pontas. */
export function translateDom(root: ParentNode) {
  if (lang === "pt") return;
  const swap = (s: string) => {
    const k = s.replace(/\s+/g, " ").trim();
    return k && EN[k] ? s.replace(/\S[\s\S]*\S|\S/, EN[k]) : s;
  };
  const walk = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  for (let n = walk.nextNode(); n; n = walk.nextNode()) n.nodeValue = swap(n.nodeValue!);
  for (const e of root.querySelectorAll("[title], [placeholder], [aria-label]"))
    for (const a of ["title", "placeholder", "aria-label"]) if (e.hasAttribute(a)) e.setAttribute(a, swap(e.getAttribute(a)!));
}
