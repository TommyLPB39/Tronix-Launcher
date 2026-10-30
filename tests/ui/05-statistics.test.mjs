// Écran des statistiques : valeurs affichées, temps estimé, état vide, retour.
import { pathToFileURL } from "node:url";
import { STRINGS } from "./strings.mjs";

class El {
  constructor(tag) {
    Object.assign(this, { tag, children: [], attrs: {}, listeners: {}, className: "", textContent: "", hidden: false, title: "", value: "" });
    this.style = { setProperty() {}, cssText: "" };
    this.classList = { add() {}, remove() {} };
  }
  append(...n) { this.children.push(...n); }
  replaceChildren(...n) { this.children = [...n]; }
  setAttribute(k, v) { this.attrs[k] = v; }
  addEventListener(t, f) { this.listeners[t] = f; }
  remove() {}
  focus() {}
  querySelector(tag) { return this.find((c) => c.tag === tag); }
  find(pred) {
    for (const c of this.children) {
      if (pred(c)) return c;
      const d = c.find?.(pred);
      if (d) return d;
    }
    return null;
  }
  findAll(pred, out = []) {
    for (const c of this.children) {
      if (pred(c)) out.push(c);
      c.findAll?.(pred, out);
    }
    return out;
  }
  // Tout le texte contenu, enfants compris.
  get text() { return this.textContent + this.children.map((c) => c.text ?? "").join(""); }
}

const byId = {};
globalThis.document = {
  getElementById: (id) => (byId[id] ??= new El(id)),
  createElement: (tag) => new El(tag),
  addEventListener() {},
  removeEventListener() {},
  documentElement: new El("html"),
  body: new El("body"),
};
globalThis.requestAnimationFrame = (cb) => cb();

const state = {
  onboarded: true, platforms: [], programs: [], close_to_tray: true, shortcut: "Ctrl+Alt+Space", shortcut_ok: true,
  accent: "#3DF5C4", accent_id: "tronix", accents: [], download_covers: false, steamgriddb_key_set: false,
};
const row = (id, name, source, launches, play_ms, play_est_ms) => ({ id, name, source, launches, play_ms, play_est_ms });
let stats = {
  total_launches: 12, programs_played: 3, current_streak: 6, best_streak: 6,
  total_play_ms: 135 * 60000, total_play_est_ms: 795 * 60000,
  week: { launches: 11, top: { name: "R.E.P.O.", count: 6 }, busiest_day: { label: "sunday", count: 4 }, new_played: ["mspaint", "Scam"] },
  top: [row("a", "R.E.P.O.", "steam", 7, 0, 700 * 60000), row("b", "mspaint", "manual", 3, 135 * 60000, 0), row("c", "Rien", "manual", 1, 0, 0)],
};
const calls = [];
globalThis.window = {
  __TAURI__: {
    core: {
      convertFileSrc: (p) => p,
      invoke: async (cmd) => { if (cmd === "get_strings") return STRINGS; calls.push(cmd); return cmd === "get_stats" ? stats : state; },
    },
    dialog: { open: async () => null },
    event: { listen: async () => () => {} },
  },
};

await import(pathToFileURL(process.argv[2]).href);
let ok = true;
const check = (label, got, want) => {
  const pass = JSON.stringify(got) === JSON.stringify(want);
  ok &&= pass;
  console.log(`${pass ? "OK " : "KO "} ${label} -> ${JSON.stringify(got)}`);
};
const view = byId["view"];

await byId["stats"].listeners.click();
const tiles = view.findAll((c) => c.className === "stat-tile").map((t) => t.text);
check("tuile des lancements", tiles[0], "1212 lancements3 programmes joués");
check("tuile de la série", tiles[1], "6jours d'affiléeMeilleure série : 6 jours");
check("tuile du temps de jeu", tiles[2], "15 h 30de temps de jeudont 13 h 15 estimé");
const week = view.find((c) => c.className === "week-lines").children.map((li) => li.children.map((c) => c.textContent));
check("bilan de la semaine", week, [
  ["Lancements", "11"],
  ["Programme le plus lancé", "R.E.P.O. · 6 fois"],
  ["Jour le plus actif", "dimanche · 4 lancements"],
  ["Nouveautés jouées", "mspaint, Scam"],
]);
const rows = view.findAll((c) => c.className === "top-row");
check("lignes du classement", rows.length, 3);
check("temps estimé étiqueté", rows[0].find((c) => c.className === "estimated")?.textContent, "estimé");
check("temps mesuré sans étiquette", rows[1].find((c) => c.className === "estimated"), null);
check("sans temps de jeu : tiret", rows[2].find((c) => c.className === "play-time muted")?.textContent, "—");
check("durée formatée", rows[1].find((c) => c.className === "play-time")?.text, "2 h 15");

// Retour à la bibliothèque
await view.find((c) => c.tag === "button" && c.textContent === "← Retour").listeners.click();
check("retour : l'écran des statistiques disparaît", view.find((c) => c.className === "stats"), null);

// Rien joué : message d'accueil
stats = { ...stats, total_launches: 0, programs_played: 0, current_streak: 0, best_streak: 0, total_play_ms: 0, total_play_est_ms: 0, top: [] };
await byId["stats"].listeners.click();
check("état vide", view.find((c) => c.className === "stats-empty")?.text, "Rien à afficher pour l'instantLance un programme depuis Tronix et tes statistiques apparaîtront ici.");

// Le bouton rouvre/ferme l'écran
await byId["stats"].listeners.click();
check("le bouton referme l'écran", view.find((c) => c.className === "stats"), null);
process.exit(ok ? 0 : 1);
