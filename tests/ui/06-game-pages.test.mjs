// Pages des jeux : états, contenu, correction, réglages et premier lancement.
import { pathToFileURL } from "node:url";
import { STRINGS } from "./strings.mjs";

class El {
  constructor(tag) {
    Object.assign(this, { tag, children: [], attrs: {}, listeners: {}, className: "", textContent: "", hidden: false, title: "", value: "", src: "", alt: "" });
    this.style = { setProperty() {}, cssText: "" };
    this.classList = { add() {}, remove() {} };
  }
  append(...n) { this.children.push(...n.map((x) => (typeof x === "string" ? Object.assign(new El("#text"), { textContent: x }) : x))); }
  replaceChildren(...n) { this.children = []; this.append(...n); }
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

const program = (id, name, source, extra = {}) => ({
  id, name, target: "x", source, added: 1_700_000_000_000, launches: [], play_ms: 0, play_est_ms: 0,
  cover: null, cover_kind: null, cover_ver: 0, ...extra,
});
let state = {
  onboarded: true, platforms: [], close_to_tray: true, shortcut: "Ctrl+Alt+Space", shortcut_ok: true,
  accent: "#3DF5C4", accent_id: "tronix", accents: [], download_covers: false, steamgriddb_key_set: false,
  online_info: true, rawg_key_set: false,
  programs: [program("s1", "Jeu Steam", "steam"), program("m1", "Logiciel", "manual", { target: "C:\\a.exe" })],
};
const baseView = (p, extra) => ({
  id: p.id, name: p.name, source: p.source, state: "ready", loading: false, error: null, details: null, shots: [],
  can_correct: p.source !== "steam", online: true, rawg_key_set: false,
  local: { added: p.added, launches: 3, last_launch: 1_780_000_000_000, play_ms: 3_600_000, play_est_ms: 1_800_000, target: p.target },
  ...extra,
});
const steamPage = {
  provider: "steam", match_id: "1", match_name: "Jeu Steam", lang: "en", summary: "Résumé court.",
  description: "Paragraphe un.\n\nParagraphe deux. " + "x".repeat(900), genres: ["Action", "Aventure"], developers: ["Studio"], publishers: ["Éditeur"],
  release_date: "26 févr. 2025", platforms: ["Windows"], website: "https://example.com", page_url: "https://store.steampowered.com/app/1",
  metacritic: 88, rating: { label: "Recommandations Steam", percent: null, count: 346424 },
  news: [{ title: "Mise à jour 1.2", url: "https://news/1", date: 1_780_000_000, summary: "Correctifs.", label: "Annonces" }],
};
let views = {};
const calls = [];
globalThis.window = {
  __TAURI__: {
    core: {
      convertFileSrc: (p) => `asset://${p}`,
      invoke: async (cmd, args) => {
        if (cmd === "get_strings") return STRINGS;
        calls.push([cmd, args]);
        if (cmd === "open_details" || cmd === "get_details") return views[args.id];
        if (cmd === "details_candidates") return [
          { provider: "wikipedia", id: "fr:Logiciel", name: "Logiciel", hint: "Programme" },
          { provider: "rawg", id: "9", name: "Logiciel (jeu)", hint: "2015" },
        ];
        return state;
      },
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
const textOf = (cls) => view.find((c) => c.className === cls)?.text;
const openPage = async (name) => {
  const cards = view.findAll((c) => c.className === "card-wrap");
  const hero = view.find((c) => c.className === "hero");
  const heroName = hero.find((c) => c.tag === "h1").textContent;
  const button = heroName === name
    ? hero.find((c) => c.textContent === "Détails")
    : cards.find((c) => c.find((x) => x.className === "meta")?.text.startsWith(name)).find((c) => c.textContent === "Détails");
  await button.listeners.click();
};

// Classement : « Logiciel » (ajouté plus tôt que Jeu Steam ? même date) — on lit simplement le héros.
const programsOrder = view.find((c) => c.className === "hero").find((c) => c.tag === "h1").textContent;
const other = programsOrder === "Jeu Steam" ? "Logiciel" : "Jeu Steam";

// 1. Page prête (Steam)
views.s1 = baseView(state.programs[0], { details: steamPage, shots: [{ path: "C:\\d\\s1\\shot1.jpg", ver: 5 }], loading: false });
await openPage("Jeu Steam");
check("page prête : titre", textOf("d-title-text")?.startsWith("Jeu Steam"), true);
check("étiquette de la source", view.find((c) => c.className === "d-tags").children[0].textContent, "Steam");
check("texte en anglais signalé", view.find((c) => c.className === "d-tags").text.includes("texte en anglais"), true);
check("résumé", textOf("d-lead"), "Résumé court.");
check("description tronquée avec « Lire la suite »", view.find((c) => c.textContent === "Lire la suite") !== null, true);
check("capture affichée via le cache", view.find((c) => c.className === "d-shot")?.src, "asset://C:\\d\\s1\\shot1.jpg?v=5");
check("actualité", view.find((c) => c.className === "d-news-title")?.textContent, "Mise à jour 1.2");
check("note formatée", textOf("d-rating-value")?.replace(/\s/g, " "), "346 424 recommandations");
check("Metacritic", textOf("d-score"), "88");
check("temps de jeu estimé étiqueté", view.findAll((c) => c.className === "estimated").length, 1);
check("jeu Steam : pas de correction", view.find((c) => c.textContent === "Ce n'est pas le bon ?"), null);
await view.find((c) => c.textContent === "Lire la suite").listeners.click();
check("« Lire la suite » déplie", view.find((c) => c.textContent === "Réduire") !== null, true);
await view.find((c) => c.textContent === "Voir sur Steam").listeners.click();
check("lien de la source", calls.at(-1), ["open_url", { url: "https://store.steampowered.com/app/1" }]);
await view.find((c) => c.className === "d-news-title").listeners.click();
check("lien d'actualité", calls.at(-1), ["open_url", { url: "https://news/1" }]);

// 2. Retour
await view.find((c) => c.textContent === "← Retour").listeners.click();
check("retour à la bibliothèque", view.find((c) => c.className === "details"), null);

// 3. Logiciel : correction
views.m1 = baseView(state.programs[1], {
  details: { provider: "wikipedia", match_id: "fr:Autre", match_name: "Autre", lang: "fr", summary: "desc", description: "Texte.", genres: [], developers: [], publishers: [], platforms: [], news: [], page_url: "https://fr.wikipedia.org/wiki/Autre" },
});
await openPage("Logiciel");
check("logiciel : emplacement affiché", view.findAll((c) => c.tag === "dt").some((c) => c.textContent === "Emplacement"), true);
await view.find((c) => c.textContent === "Ce n'est pas le bon ?").listeners.click();
await new Promise((r) => setTimeout(r, 0));
const panel = byId["overlay"].children[0];
check("candidats proposés", panel.findAll((c) => c.className === "candidate").map((c) => c.children[0].textContent), ["Logiciel", "Logiciel (jeu)"]);
await panel.findAll((c) => c.className === "candidate")[1].listeners.click();
check("choix mémorisé", calls.find((c) => c[0] === "choose_details"), ["choose_details", { id: "m1", provider: "rawg", matchId: "9" }]);
await view.find((c) => c.textContent === "Ce n'est pas le bon ?").listeners.click();
await new Promise((r) => setTimeout(r, 0));
await byId["overlay"].children[0].find((c) => c.textContent === "Aucune de ces pages").listeners.click();
check("« aucune page » mémorisé", calls.filter((c) => c[0] === "choose_details").at(-1)[1], { id: "m1", provider: "none", matchId: "" });
await view.find((c) => c.textContent === "← Retour").listeners.click();

// 4. États sans contenu
const stateCases = [
  ["off", "Informations en ligne désactivées", "Activer les informations en ligne"],
  ["needs_key", "Une clé RAWG est nécessaire", "Ouvrir les paramètres"],
  ["none", "Aucune page trouvée", "Chercher une autre page"],
];
for (const [name, title, action] of stateCases) {
  views.m1 = baseView(state.programs[1], { state: name, online: name !== "off" });
  await openPage("Logiciel");
  check(`état « ${name} » : message`, textOf("d-banner")?.startsWith(title), true);
  check(`état « ${name} » : action`, view.find((c) => c.tag === "button" && c.textContent === action) !== null, true);
  await view.find((c) => c.textContent === "← Retour").listeners.click();
}
views.m1 = baseView(state.programs[1], { state: "pending", loading: true, error: "Réponse inattendue du service." });
await openPage("Logiciel");
check("hors connexion : message et nouvel essai", [textOf("d-loading"), view.find((c) => c.textContent === "Réessayer") !== null], ["Réseau indisponible : Réponse inattendue du service.", true]);
await view.find((c) => c.textContent === "← Retour").listeners.click();

// 5. Activation depuis la page
views.m1 = baseView(state.programs[1], { state: "off", online: false });
await openPage("Logiciel");
calls.length = 0;
await view.find((c) => c.textContent === "Activer les informations en ligne").listeners.click();
check("activation depuis la page", calls[0], ["set_online_info", { enabled: true, key: null }]);
await view.find((c) => c.textContent === "← Retour").listeners.click();

// 6. Paramètres : interrupteur et clé RAWG
await byId["settings"].listeners.click();
const settings = byId["overlay"].children[0];
const onlineRow = settings.findAll((c) => c.attrs.role === "switch").find((c) => c.children[0]?.textContent === "Informations en ligne");
calls.length = 0;
await onlineRow.listeners.click();
check("interrupteur des informations en ligne", [calls[0], onlineRow.attrs["aria-checked"]], [["set_online_info", { enabled: false, key: null }], "false"]);
const keyInputs = settings.findAll((c) => c.className === "text-input");
const rawg = keyInputs.find((c) => c.placeholder.includes("RAWG"));
rawg.value = " ma-cle-rawg ";
await rawg.listeners.change();
check("clé RAWG envoyée", calls.at(-1), ["set_online_info", { enabled: false, key: " ma-cle-rawg " }]);
check("clé RAWG : champ vidé", rawg.value, "");

// 7. Premier lancement
state = { ...state, onboarded: false, platforms: [{ id: "steam", label: "Steam", enabled: true, detected: true }] };
calls.length = 0;
await import(pathToFileURL(process.argv[2]).href + "?second");
const tiles = byId["view"].findAll((c) => c.className === "tile tile-wide");
const online = tiles.find((t) => t.text.startsWith("Informations en ligne"));
check("tuile des informations en ligne, désactivée d'office", online?.attrs["aria-pressed"], "false");
online.listeners.click();
await byId["view"].find((c) => c.tag === "button" && c.textContent === "Continuer").listeners.click();
const sets = calls.filter((c) => c[0].startsWith("set_"));
check("choix envoyés", sets.map((c) => [c[0], c[1].enabled]).slice(0, 3), [["set_download_covers", false], ["set_online_info", true], ["set_platforms", ["steam"]]]);

process.exit(ok ? 0 : 1);
