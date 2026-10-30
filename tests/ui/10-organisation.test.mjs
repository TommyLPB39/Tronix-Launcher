// Organisation : tri, favoris, recherche et filtre par launcher.
import { pathToFileURL } from "node:url";
import { STRINGS } from "./strings.mjs";

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
  id, name, original_name: name, target: "C:\\a.exe", source, added: 1, launches: [], play_ms: 0, play_est_ms: 0,
  cover: null, cover_kind: null, cover_ver: 0, custom_name: null, custom_description: null, uninstalled: false, favorite: false, ...extra,
});
const LOOK = { theme: "dark", card_size: "medium", radius: "round", view: "grid", show_hero: true, hero_background: false, custom_accent: "" };
const all = () => [
  prog("a", "Zelda", "steam", { launches: [100], play_ms: 3_600_000, added: 5 }),
  prog("b", "Éclair", "manual", { added: 9 }),
  prog("c", "Mario", "steam", { launches: [300, 200], play_ms: 7_200_000, added: 1 }),
  prog("d", "Alpha", "epic", { added: 7, favorite: true }),
  prog("e", "Beta", "manual", { added: 3 }),
  prog("f", "Uninst", "steam", { uninstalled: true }),
];
let state = {
  onboarded: true, platforms: [], close_to_tray: true, shortcut: "Ctrl+Alt+Space", shortcut_ok: true, accent: "#3DF5C4", accent_id: "tronix", accents: [],
  download_covers: false, steamgriddb_key_set: false, online_info: false, rawg_key_set: false, show_uninstalled: true, steam_key_set: false,
  look: { ...LOOK }, sort: "rank", programs: all(),
};
const detailsFor = (p) => ({
  id: p.id, name: p.name, original_name: p.name, custom_description: null, source: p.source, state: "off", loading: false, error: null, shots: [],
  can_correct: false, online: false, rawg_key_set: false, details: null,
  local: { added: 1, launches: 0, last_launch: null, play_ms: 0, play_est_ms: 0, target: "x" },
});
const calls = [];
globalThis.window = {
  __TAURI__: {
    core: {
      convertFileSrc: (p) => `asset://${p}`,
      invoke: async (cmd, args) => {
        if (cmd === "get_strings") return STRINGS;
        calls.push([cmd, args]);
        if (cmd === "set_favorite") {
          state.programs = state.programs.map((p) => (p.id === args.id ? { ...p, favorite: args.favorite } : p));
          return state.programs;
        }
        if (cmd === "open_details" || cmd === "get_details") return detailsFor(state.programs.find((p) => p.id === args.id));
        if (cmd === "set_sort") return null;
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
const flush = () => new Promise((r) => setTimeout(r, 0));
const name = (c) => c.find((m) => m.className === "meta").textContent;
const titles = () => view.findAll((c) => c.className === "section-title").map((c) => c.textContent);
const isCard = (c) => c.tag === "button" && (c.className === "card" || c.className === "card dim");
// Noms des cartes de la section qui suit un titre donné
const under = (title) => {
  const results = view.find((c) => c.className === "results");
  const i = results.children.findIndex((c) => c.className === "section-title" && c.textContent === title);
  return i < 0 ? null : results.children[i + 1].findAll(isCard).map(name);
};
const select = (label) => view.find((c) => c.tag === "select" && c.attrs["aria-label"] === label);
const pick = async (label, value) => { const s = select(label); s.value = value; await s.listeners.change(); await flush(); };
const search = () => view.find((c) => c.className === "text-input toolbar-search");
const type = async (text) => { search().value = text; await search().listeners.input(); await flush(); };
const heroName = () => view.find((c) => c.className === "hero")?.find((c) => c.tag === "h1").textContent ?? null;

// Présentation d'origine : suggestion, favoris, bibliothèque
check("barre présente (6 programmes)", [search() !== null, select("Filtrer par launcher") !== null, select("Trier la bibliothèque") !== null], [true, true, true]);
check("suggestion = jeu le plus probable", heroName(), "Zelda");
check("titres des sections", titles(), ["Favoris", "Ta bibliothèque"]);
check("favoris", under("Favoris"), ["Alpha"]);
check("bibliothèque : ordre de probabilité reçu", under("Ta bibliothèque"), ["Éclair", "Mario", "Beta"]);
check("options du filtre", select("Filtrer par launcher").children.map((o) => o.textContent), ["Tous les launchers", "Ajouté", "Epic", "Steam"]);
check("options du tri", select("Trier la bibliothèque").children.map((o) => o.textContent), ["Probabilité", "Nom A-Z", "Récemment lancé", "Temps de jeu", "Date d'ajout"]);

// Tris
await pick("Trier la bibliothèque", "name");
check("tri par nom (sans tenir compte des accents)", under("Ta bibliothèque"), ["Beta", "Éclair", "Mario"]);
check("tri enregistré", calls.filter((c) => c[0] === "set_sort").at(-1)[1], { sort: "name" });
await pick("Trier la bibliothèque", "recent");
check("tri récemment lancé", under("Ta bibliothèque"), ["Mario", "Éclair", "Beta"]);
await pick("Trier la bibliothèque", "playtime");
check("tri par temps de jeu", under("Ta bibliothèque"), ["Mario", "Éclair", "Beta"]);
await pick("Trier la bibliothèque", "added");
check("tri par date d'ajout", under("Ta bibliothèque"), ["Éclair", "Beta", "Mario"]);
check("la suggestion ne dépend pas du tri", heroName(), "Zelda");
await pick("Trier la bibliothèque", "rank");

// Recherche : le champ n'est pas recréé (il garde le focus), accents ignorés
const field = search();
await type("eclair");
check("recherche : champ conservé", search() === field, true);
check("recherche : sans accents, sans suggestion ni favoris", [under("Ta bibliothèque"), heroName(), titles()], [["Éclair"], null, ["Ta bibliothèque"]]);
await type("zzz");
check("recherche sans résultat", view.find((c) => c.className === "no-match")?.textContent, "Aucun jeu ne correspond.");
await type("uninst");
check("recherche : jeux non installés montrés d'office", [view.find((c) => c.className === "section-toggle").attrs["aria-expanded"], view.findAll((c) => c.className === "card dim").length], ["true", 1]);
await type("");
check("recherche effacée : présentation d'origine", [titles(), heroName()], [["Favoris", "Ta bibliothèque"], "Zelda"]);

// Filtre par launcher
await pick("Filtrer par launcher", "steam");
check("filtre Steam : Zelda et Mario, un non installé, pas de suggestion", [under("Ta bibliothèque"), view.findAll((c) => c.className === "card dim").length, heroName()], [["Zelda", "Mario"], 1, null]);
await type("mar");
check("filtre + recherche combinés", under("Ta bibliothèque"), ["Mario"]);
await pick("Filtrer par launcher", "epic");
check("filtre Epic + recherche « mar » : rien", view.find((c) => c.className === "no-match") !== null, true);
await type("");
check("filtre Epic : favori Alpha seulement", [under("Favoris"), under("Ta bibliothèque")], [["Alpha"], null]);
await pick("Filtrer par launcher", "");

// Favoris : étoile sur la carte et bouton sur la page
const wrapOf = (n) => view.findAll((c) => c.className === "card-wrap").find((w) => name(w) === n);
const star = wrapOf("Éclair").find((c) => c.tag === "button" && c.className.startsWith("card-star"));
check("étoile : libellé", [star.textContent, star.attrs["aria-pressed"]], ["☆", "false"]);
await star.listeners.click();
await flush();
check("étoile : favori envoyé", calls.filter((c) => c[0] === "set_favorite").at(-1)[1], { id: "b", favorite: true, tz: new Date().getTimezoneOffset() });
check("étoile : Éclair rejoint les favoris", [under("Favoris"), under("Ta bibliothèque")], [["Éclair", "Alpha"], ["Mario", "Beta"]]);
check("jeu non installé : pas d'étoile", view.findAll((c) => c.className === "card-wrap").filter((w) => w.find((c) => c.className === "card dim")).every((w) => w.find((c) => c.className.startsWith("card-star")) === null), true);

await wrapOf("Alpha").find((c) => c.textContent === "Détails").listeners.click();
await flush();
const fav = view.find((c) => c.tag === "button" && c.textContent === "★ Favori");
check("page du jeu : bouton favori", fav !== null, true);
await fav.listeners.click();
await flush();
check("page du jeu : retrait du favori", calls.filter((c) => c[0] === "set_favorite").at(-1)[1].favorite, false);
check("page du jeu : bouton mis à jour", view.find((c) => c.tag === "button" && c.textContent === "☆ Favori") !== null, true);

// Suggestion masquée : tous les jeux dans les sections
state = { ...state, look: { ...LOOK, show_hero: false }, programs: all() };
await import(pathToFileURL(process.argv[2]).href + "?sanshero");
check("sans suggestion : tous les jeux installés listés", [byId["view"].find((c) => c.className === "hero"), byId["view"].findAll((c) => c.className === "card").length], [null, 5]);

// Peu de jeux : pas de barre ; tri retenu relu au démarrage
state = { ...state, look: { ...LOOK }, sort: "name", programs: all().slice(0, 3) };
await import(pathToFileURL(process.argv[2]).href + "?peu");
check("moins de 4 programmes : pas de barre", byId["view"].find((c) => c.className === "toolbar"), null);
state = { ...state, sort: "name", programs: all() };
await import(pathToFileURL(process.argv[2]).href + "?tri");
check("tri retenu au démarrage", byId["view"].find((c) => c.tag === "select" && c.attrs["aria-label"] === "Trier la bibliothèque").value, "name");

process.exit(ok ? 0 : 1);
