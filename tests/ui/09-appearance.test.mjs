// Apparence : thème, accent libre, tailles, vue liste, héros masqué, fond flou, réglages et réinitialisation.
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
const prog = (id, name, extra = {}) => ({
  id, name, original_name: name, target: "C:\a.exe", source: "manual", added: 1_700_000_000_000, launches: [], play_ms: 0, play_est_ms: 0,
  cover: null, cover_kind: null, cover_ver: 0, custom_name: null, custom_description: null, uninstalled: false, ...extra,
});
const LOOK = { theme: "dark", card_size: "medium", radius: "round", view: "grid", show_hero: true, hero_background: false, custom_accent: "" };
let state = {
  onboarded: true, platforms: [], close_to_tray: true, shortcut: "Ctrl+Alt+Space", shortcut_ok: true,
  accent: "#3DF5C4", accent_id: "tronix", accents: [{ id: "tronix", label: "Tronix", hex: "#3DF5C4" }, { id: "ciel", label: "Ciel", hex: "#5AA9FF" }],
  download_covers: false, steamgriddb_key_set: false, online_info: false, rawg_key_set: false, show_uninstalled: false, steam_key_set: false,
  look: { ...LOOK },
  programs: [prog("a", "Alpha", { cover: "C:\c\a.png", cover_kind: "art", cover_ver: 3, launches: [1_760_000_000_000], play_ms: 5_400_000 }), prog("b", "Bravo"), prog("c", "Charlie")],
};
const calls = [];
let osLight = false;
let failSave = false;
globalThis.window = {
  matchMedia: () => ({ get matches() { return osLight; }, addEventListener() {} }),
  __TAURI__: {
    core: {
      convertFileSrc: (p) => `asset://${p}`,
      invoke: async (cmd, args) => {
        if (cmd === "get_strings") return STRINGS;
        calls.push([cmd, args]);
        if (cmd === "set_look") { if (failSave) throw "Disque plein."; return args.look; }
        if (cmd === "set_accent") return args.id === "ciel" ? "#5AA9FF" : "#3DF5C4";
        if (cmd === "reset_look") { state = { ...state, accent: "#3DF5C4", look: { ...LOOK } }; return { ...LOOK }; }
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
const view = byId["view"], overlay = byId["overlay"], root = document.documentElement;
const flush = () => new Promise((r) => setTimeout(r, 0));
const click = async (parent, label) => parent.find((c) => c.tag === "button" && c.textContent === label).listeners.click();
const row = (label) => overlay.find((c) => c.className === "setting-row" && c.children[0]?.textContent === label);
const sw = (label) => overlay.find((c) => c.className === "switch" && c.children[0]?.textContent === label);
const seg = (rowLabel, text) => row(rowLabel).children[1].children.find((b) => b.textContent === text);
const lastSet = () => calls.filter((c) => c[0] === "set_look").at(-1)?.[1].look;
const cardsIn = () => view.findAll((c) => c.tag === "button" && c.className === "card").length;

// Par défaut : sombre, héros affiché, grille, accent de base
check("défaut : attributs", [root.attrs["data-theme"], root.attrs["data-size"], root.attrs["data-radius"]], ["dark", "medium", "round"]);
check("défaut : accent et couleur du texte dessus", [root.style.props["--accent"], root.style.props["--accent-ink"]], ["#3DF5C4", "#04120d"]);
check("défaut : héros + grille de 2", [view.find((c) => c.className === "hero") !== null, cardsIn(), view.find((c) => c.className === "grid list")], [true, 2, null]);
const backdrop = document.body.children.at(-1);
check("défaut : fond désactivé", backdrop.className, "backdrop");

// Ouvrir les paramètres
byId["settings"].listeners.click();
check("paramètres : thème actuel marqué", seg("Thème", "Sombre").attrs["aria-pressed"], "true");

// Thème clair
await seg("Thème", "Clair").listeners.click();
await flush();
check("thème clair : enregistré et appliqué", [lastSet().theme, root.attrs["data-theme"]], ["light", "light"]);
check("thème clair : bouton marqué", [seg("Thème", "Clair").attrs["aria-pressed"], seg("Thème", "Sombre").attrs["aria-pressed"]], ["true", "false"]);

// Thème automatique : suit Windows
osLight = false;
await seg("Thème", "Auto").listeners.click();
await flush();
check("auto + Windows sombre", root.attrs["data-theme"], "dark");
osLight = true;
await seg("Thème", "Auto").listeners.click();
await flush();
check("auto + Windows clair", root.attrs["data-theme"], "light");

// Tailles, coins
await seg("Taille des pochettes", "Grande").listeners.click();
await seg("Coins", "Carrés").listeners.click();
await flush();
check("taille et coins", [root.attrs["data-size"], root.attrs["data-radius"], lastSet().card_size, lastSet().radius], ["large", "square", "large", "square"]);

// Masquer la suggestion
await sw("Afficher la grande suggestion").listeners.click();
await flush();
check("héros masqué : enregistré", lastSet().show_hero, false);
check("héros masqué : tous les jeux dans la grille", [view.find((c) => c.className === "hero"), cardsIn()], [null, 3]);

// Vue liste (héros masqué : les trois jeux sont des lignes) : temps de jeu et dernier lancement
await seg("Affichage", "Liste").listeners.click();
await flush();
check("vue liste : grille en liste", view.find((c) => c.className === "grid list") !== null, true);
check("vue liste : infos de la ligne", view.findAll((c) => c.className === "row-info").map((c) => c.textContent.replace(/ le \d.*/, " le …")), ["1 h 30 · lancé le …", "jamais lancé", "jamais lancé"]);
await seg("Affichage", "Grille").listeners.click();
await flush();
check("retour en grille : pas d'infos de ligne", [view.find((c) => c.className === "grid list"), view.findAll((c) => c.className === "row-info").length], [null, 0]);

await sw("Afficher la grande suggestion").listeners.click();
await flush();
check("héros réaffiché", [view.find((c) => c.className === "hero") !== null, cardsIn()], [true, 2]);

// Fond : pochette du jeu suggéré
await sw("Fond : pochette de la suggestion, floutée").listeners.click();
await flush();
check("fond activé : pochette du héros", [backdrop.className, backdrop.style.backgroundImage], ["backdrop on", 'url("asset://C:\c\a.png?v=3")']);

// Couleur libre : aperçu en direct, enregistrée à la validation, lisible dessus
const picker = row("Couleur libre").children[1];
const before = calls.filter((c) => c[0] === "set_look").length;
picker.value = "#ffd400";
picker.listeners.input();
check("couleur libre : aperçu en direct, texte sombre", [root.style.props["--accent"], root.style.props["--accent-ink"]], ["#ffd400", "#04120d"]);
check("couleur libre : rien d'enregistré pendant l'aperçu", calls.filter((c) => c[0] === "set_look").length, before);
picker.value = "#3a1d8f";
picker.listeners.input();
await picker.listeners.change();
check("couleur libre : enregistrée, texte blanc sur fond foncé", [lastSet().custom_accent, root.style.props["--accent-ink"]], ["#3a1d8f", "#ffffff"]);

// Choisir une couleur proposée remplace la couleur libre
const preset = overlay.find((c) => c.className === "swatch-color" && c.title === "Ciel");
await preset.listeners.click();
await flush();
check("couleur proposée : remplace la couleur libre", [root.style.props["--accent"], picker.value], ["#5AA9FF", "#5AA9FF"]);

// Rétablir l'apparence par défaut
await click(overlay, "Rétablir l'apparence par défaut");
await flush();
check("rétablir : commande puis état relu", calls.slice(-3).map((c) => c[0]).filter((n) => n === "reset_look" || n === "get_state").slice(0, 2), ["reset_look", "get_state"]);
check("rétablir : apparence d'origine", [root.attrs["data-theme"], root.attrs["data-size"], root.attrs["data-radius"], root.style.props["--accent"]], ["dark", "medium", "round", "#3DF5C4"]);

// Une erreur d'enregistrement est affichée
failSave = true;
seg("Thème", "Clair").listeners.click();
await flush();
check("erreur d'enregistrement affichée", byId["toast"].textContent, "Disque plein.");

// Fond : une simple icône n'est pas affichée en grand
state = { ...state, look: { ...LOOK, hero_background: true }, programs: [prog("a", "Alpha", { cover: "C:\c\a.png", cover_kind: "icon" }), prog("b", "Bravo")] };
globalThis.window.__TAURI__.core.invoke = async () => state;
await import(pathToFileURL(process.argv[2]).href + "?icone");
check("fond : une simple icône n'est pas affichée", document.body.children.at(-1).className, "backdrop");

process.exit(ok ? 0 : 1);
