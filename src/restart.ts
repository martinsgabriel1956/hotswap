import { tr } from "./i18n.ts";
/** O que o Reiniciar (Ctrl+Shift+R; com Alt, os dois lados) diz no cabeçalho em vez de agir, ou null quando ele reinicia:
 *  para o que roda e sobe o start de novo. Lado sem start não reinicia; parado, não há o que reiniciar, e o recado diz
 *  quem sobe. A troca e a parada em andamento são vistas antes, como no Parar e no Subir.
 *  `what`: os lados pedidos ("API", "API e Front"); `hasStart`: algum deles tem start; `running`: algum com start roda. */
export function restartRefusal(o: { what: string; all: boolean; hasStart: boolean; running: boolean }): string | null {
  if (!o.hasStart) return tr("{0}: sem comando de start", o.what);
  if (!o.running) return tr("Nada rodando {0} · Ctrl+Shift+{1}S sobe", o.all ? tr("na worktree") : tr("em {0}", o.what), o.all ? "Alt+" : "");
  return null;
}
