// Langues dans l'interface : anglais d'abord, pluriels, dates, étiquettes, changement de langue sans redémarrer.
import { pathToFileURL } from "node:url";
import { STRINGS, STRINGS_EN } from "./strings.mjs";

class El {
  constructor(tag) {
    Object.assign(this, { tag, children: [], attrs: {}, listeners: {}, className: "", textContent: "", hidden: false, title: "", value: "", src: "", alt: "", placeholder: "", type: "" });
    this.style = { props: {}, setProperty(k, v) { this.props[k] = v; }, cssText: "" };
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

globalThis.localStorage = { getItem: () => null, setItem() {} };
const prog = (id, name, source, extra = {}) => ({
  id, name, original_name: name, target: "C:\\a.exe", source, added: 1_700_000_000_000, launches: [], play_ms: 0, play_est_ms: 0,
  cover: null, cover_kind: null, cover_ver: 0, custom_name: null, custom_description: null, uninstalled: false, favorite: false, collections: [], ...extra,
});
const LAUNCH = 1_760_000_000_000;
const LOOK = { theme: "dark", card_size: "medium", radius: "round", view: "grid", show_hero: true, hero_background: false, custom_accent: "" };
let state = {
  onboarded: true, platforms: [], close_to_tray: true, shortcut: "Ctrl+Alt+Space", shortcut_ok: true, accent: "#3DF5C4", accent_id: "tronix",
  accents: [{ id: "tronix", label: "Tronix", hex: "#3DF5C4" }, { id: "braise", label: "Braise", hex: "#FF7A45" }],
  download_covers: false, steamgriddb_key_set: false, online_info: true, rawg_key_set: false, show_uninstalled: false, steam_key_set: false,
  look: { ...LOOK }, sort: "rank", language: "auto", collections: [{ id: "c1", name: "Coop" }],
  programs: [prog("z", "Zero", "steam", { launches: [LAUNCH] }), prog("a", "Alpha", "manual", { launches: [LAUNCH], play_ms: 5_400_000, collections: ["c1"] }), prog("b", "Beta", "manual"), prog("c", "Gamma", "epic"), prog("d", "Delta", "manual")],
};
let dictionary = STRINGS_EN; // l'anglais d'abord : Windows en anglais, réglage « auto »
const calls = [];
const detailsFor = (p) => ({
  id: p.id, name: p.name, original_name: p.name, custom_description: null, source: p.source, state: "ready", loading: false, error: null, shots: [],
  can_correct: true, online: true, rawg_key_set: false,
  details: { provider: "wikipedia", match_id: "fr:Alpha", match_name: "Alpha", lang: "fr", summary: "", description: "Un texte.", genres: [], developers: [], publishers: [], platforms: [], news: [], page_url: "https://fr.wikipedia.org/wiki/Alpha" },
  local: { added: 1_700_000_000_000, launches: 1, last_launch: LAUNCH, play_ms: 5_400_000, play_est_ms: 0, target: "C:\\a.exe" },
});
globalThis.window = {
  __TAURI__: {
    core: {
      convertFileSrc: (p) => `asset://${p}`,
      invoke: async (cmd, args) => {
        if (cmd === "get_strings") return dictionary;
        calls.push([cmd, args]);
        if (cmd === "set_language") {
          dictionary = args.language === "fr" ? STRINGS : STRINGS_EN;
          state = { ...state, language: args.language };
          return state;
        }
        if (cmd === "open_details" || cmd === "get_details") return detailsFor(state.programs.find((p) => p.id === args.id));
        if (cmd === "get_stats") {
          return {
            total_launches: 9, programs_played: 3, current_streak: 1, best_streak: 4, total_play_ms: 3_600_000, total_play_est_ms: 0,
            week: { launches: 5, top: { name: "Alpha", count: 1 }, busiest_day: { label: "sunday", count: 2 }, new_played: [] },
            top: [{ name: "Alpha", source: "manual", launches: 1, play_ms: 0, play_est_ms: 0 }],
          };
        }
        return state;
      },
    },
    dialog: { open: async () => null },
    event: { listen: async () => () => {} },
  },
};
for (const id of ["import", "add", "stats", "settings"]) document.getElementById(id);

await import(pathToFileURL(process.argv[2]).href);
let ok = true;
const check = (label, got, want) => {
  const pass = JSON.stringify(got) === JSON.stringify(want);
  ok &&= pass;
  console.log(`${pass ? "OK " : "KO "} ${label} -> ${JSON.stringify(got)}`);
};
const view = byId["view"], overlay = byId["overlay"], html = document.documentElement;
const flush = () => new Promise((r) => setTimeout(r, 0));
const texts = (root, cls) => root.findAll((c) => c.className === cls).map((c) => c.textContent);
const select = (label) => view.find((c) => c.tag === "select" && c.attrs["aria-label"] === label);
const shortDate = (locale) => new Date(LAUNCH).toLocaleDateString(locale, { day: "numeric", month: "short" });

// ---- Anglais
check("barre du haut en anglais", [byId["import"].textContent, byId["add"].textContent, byId["stats"].attrs["aria-label"], byId["settings"].title], ["Import", "+ Add", "Statistics", "Settings"]);
check("langue du document", html.attrs["lang"], "en");
check("suggestion", [texts(view, "tag")[0], view.find((c) => c.tag === "h1").textContent, view.find((c) => c.className === "hero").find((c) => c.tag === "p").textContent], ["Your pick right now", "Zero", "Launched 1 time · Steam"]);
check("bouton Lancer", view.find((c) => c.className === "btn primary").textContent, "▶ Launch");
check("titre et recherche", [texts(view, "section-title"), view.find((c) => c.className === "text-input toolbar-search").placeholder], [["Your library"], "Search for a game…"]);
check("filtres et tris", [select("Filter by launcher").children.map((o) => o.textContent), select("Sort the library").children.map((o) => o.textContent), select("Filter by collection").children.map((o) => o.textContent)], [["All launchers", "Added", "Epic", "Steam"], ["Likelihood", "Name A-Z", "Recently launched", "Play time", "Date added"], ["All collections", "Coop"]]);
check("source d'un programme ajouté à la main", view.findAll((c) => c.tag === "small").map((c) => c.textContent).includes("Added"), true);
const alphaWrap = view.findAll((c) => c.className === "card-wrap").find((w) => w.find((m) => m.className === "meta").textContent.startsWith("Alpha"));
check("infobulles d'une carte", alphaWrap.findAll((c) => c.title).map((c) => c.title), ["Open the page of Alpha", "Change the image of Alpha", "Put Alpha in collections", "Add Alpha to favorites"]);

// ---- Pluriels, dates et durées
await view.find((c) => c.className.includes("toolbar-button")).listeners.click();
check("collections : pluriel anglais", [overlay.children[0].find((c) => c.tag === "h2").textContent, overlay.children[0].find((c) => c.className === "coll-count").textContent], ["Collections", "1 game"]);
overlay.hidden = true;
await byId["settings"].listeners.click();
const settingsPanel = overlay.children[0];
const languageRow = settingsPanel.find((c) => c.className === "setting-row" && c.children[0]?.textContent === "Language");
check("réglage de langue", languageRow.children[1].children.map((b) => [b.textContent, b.attrs["aria-pressed"]]), [["Automatic", "true"], ["Français", "false"], ["English", "false"]]);
check("accents traduits", settingsPanel.findAll((c) => c.className === "swatch-color").map((c) => c.title), ["Tronix", "Ember"]);
check("raccourci : touche Espace", settingsPanel.find((c) => c.className === "setting-row" && c.children[0]?.textContent === "Quick launch shortcut").children[1].textContent, "Ctrl+Alt+Space");

// ---- Page d'un jeu : texte dans une autre langue que l'interface
overlay.hidden = true;
await alphaWrap.find((c) => c.textContent === "Details").listeners.click();
await flush();
check("page : étiquette de langue du texte", texts(view, "tag muted-tag"), ["text in French"]);
check("page : usage en anglais", view.findAll((c) => c.tag === "dt").map((c) => c.textContent), ["Launches", "Last launch", "Play time", "Added on", "Location"]);
check("page : date et durée formatées en anglais", view.findAll((c) => c.tag === "dd").slice(0, 3).map((c) => c.textContent), ["1", new Date(LAUNCH).toLocaleDateString("en-US", { day: "numeric", month: "long", year: "numeric" }), "1 h 30"]);
await view.find((c) => c.textContent === "← Back").listeners.click();

// ---- Statistiques
await byId["stats"].listeners.click();
await flush();
check("statistiques en anglais", [view.find((c) => c.tag === "h1").textContent, view.findAll((c) => c.className === "stat-label").map((c) => c.textContent), view.findAll((c) => c.tag === "li").map((c) => c.children[1].textContent).slice(0, 3)], ["Statistics", ["9 launches", "day in a row", "of play time"], ["5", "Alpha · 1 time", "Sunday · 2 launches"]]);
await byId["stats"].listeners.click();
await flush();

// ---- Changer de langue : tout passe en français sans redémarrer
await byId["settings"].listeners.click();
const row2 = overlay.children[0].find((c) => c.className === "setting-row" && c.children[0]?.textContent === "Language");
await row2.children[1].children.find((b) => b.textContent === "Français").listeners.click();
await flush();
check("commande envoyée", calls.filter((c) => c[0] === "set_language").at(-1)[1], { language: "fr", tz: new Date().getTimezoneOffset() });
check("barre du haut en français", [byId["import"].textContent, byId["add"].textContent, byId["stats"].attrs["aria-label"], html.attrs["lang"]], ["Importer", "+ Ajouter", "Statistiques", "fr"]);
check("suggestion en français", [texts(view, "tag")[0], view.find((c) => c.className === "hero").find((c) => c.tag === "p").textContent], ["Ton envie du moment", "Lancé 1 fois · Steam"]);
check("la fenêtre des paramètres se rouvre dans la nouvelle langue", [overlay.hidden, overlay.children[0].find((c) => c.tag === "h2").textContent], [false, "Paramètres"]);
const row3 = overlay.children[0].find((c) => c.className === "setting-row" && c.children[0]?.textContent === "Langue");
check("réglage mémorisé", row3.children[1].children.map((b) => [b.textContent, b.attrs["aria-pressed"]]), [["Automatique", "false"], ["Français", "true"], ["English", "false"]]);
overlay.hidden = true;
await view.find((c) => c.className.includes("toolbar-button")).listeners.click();
check("collections : pluriel français", overlay.children[0].find((c) => c.className === "coll-count").textContent, "1 jeu");
check("tri en français", select("Trier la bibliothèque").children.map((o) => o.textContent), ["Probabilité", "Nom A-Z", "Récemment lancé", "Temps de jeu", "Date d'ajout"]);

// ---- Retour à l'anglais
overlay.hidden = true;
await byId["settings"].listeners.click();
await overlay.children[0].find((c) => c.className === "setting-row" && c.children[0]?.textContent === "Langue").children[1].children.find((b) => b.textContent === "English").listeners.click();
await flush();
check("retour à l'anglais", [byId["import"].textContent, texts(view, "tag")[0]], ["Import", "Your pick right now"]);

process.exit(ok ? 0 : 1);
