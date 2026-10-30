// Fait tourner chaque scénario de tests/ui/*.test.mjs contre src/main.js.
// Runs every scenario in tests/ui/*.test.mjs against src/main.js (fake DOM + fake Tauri core, no browser).
//
//   npm run test:ui
//
// main.js utilise `await` au niveau du module : on en fait une copie .mjs dans le dossier temporaire,
// et chaque scénario l'importe. Sortie non nulle si un scénario échoue.
import { spawnSync } from "node:child_process";
import { copyFileSync, mkdtempSync, readdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const main = join(here, "..", "..", "src", "main.js");
const work = mkdtempSync(join(tmpdir(), "tronix-ui-"));
const copy = join(work, "main.mjs");
copyFileSync(main, copy);
copyFileSync(join(here, "..", "..", "src", "i18n.js"), join(work, "i18n.js")); // importé par main.js

let failed = 0;
const suites = readdirSync(here).filter((name) => name.endsWith(".test.mjs")).sort();
for (const name of suites) {
  const run = spawnSync(process.execPath, [join(here, name), copy], { encoding: "utf8", cwd: work });
  const lines = (run.stdout ?? "").split("\n").filter((line) => line.startsWith("OK ") || line.startsWith("KO "));
  const bad = lines.filter((line) => line.startsWith("KO "));
  const ok = run.status === 0 && bad.length === 0;
  console.log(`${ok ? "PASS" : "FAIL"}  ${name}  (${lines.length - bad.length}/${lines.length} vérifications)`);
  if (!ok) {
    failed += 1;
    for (const line of bad) console.log(`      ${line.slice(0, 200)}`);
    if (run.stderr) console.log(run.stderr.split("\n").slice(0, 6).join("\n"));
  }
}
rmSync(work, { recursive: true, force: true });
console.log(failed === 0 ? `\n${suites.length} scénarios réussis.` : `\n${failed} scénario(s) en échec.`);
process.exit(failed === 0 ? 0 : 1);
