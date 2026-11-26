/** A versão do último release ("v0.10.0") é mais nova que a instalada ("0.9.3")? Número a número, não como texto
 *  ("0.10" > "0.9"). Pré-release ("0.6.0-beta.1") nunca conta: o aviso é só para versões publicadas. */
export function isNewer(latest: string, current: string): boolean {
  const parse = (v: string) => v.trim().replace(/^v/, "");
  const a = parse(latest);
  const b = parse(current);
  if (a.includes("-") || !/^\d+(\.\d+)*$/.test(a) || !/^\d+(\.\d+)*/.test(b)) return false;
  const x = a.split(".").map(Number);
  const y = b.split("-")[0].split(".").map(Number);
  for (let i = 0; i < Math.max(x.length, y.length); i++) {
    const d = (x[i] ?? 0) - (y[i] ?? 0);
    if (d) return d > 0;
  }
  return false;
}
