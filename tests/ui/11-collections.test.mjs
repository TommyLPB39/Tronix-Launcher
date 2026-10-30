// Collections : création, renommage, suppression, rangement et filtre.
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
  id, name, original_name: name, target: "C:\\a.exe", source: "manual", added: 1, launches: [], play_ms: 0, play_est_ms: 0,
  cover: null, cover_kind: null, cover_ver: 0, custom_name: null, custom_description: null, uninstalled: false, favorite: false, collections: [], ...extra,
});
const LOOK = { theme: "dark", card_size: "medium", radius: "round", view: "grid", show_hero: true, hero_background: false, custom_accent: "" };
let state = {
  onboarded: true, platforms: [], close_to_tray: true, shortcut: "Ctrl+Alt+Space", shortcut_ok: true, accent: "#3DF5C4", accent_id: "tronix", accents: [],
  download_covers: false, steamgriddb_key_set: false, online_info: false, rawg_key_set: false, show_uninstalled: false, steam_key_set: false,
  look: { ...LOOK }, sort: "rank", collections: [],
  programs: [prog("z", "Zero"), prog("a", "Alpha"), prog("b", "Beta"), prog("c", "Gamma"), prog("d", "Delta")],
};
const detailsFor = (p) => ({
  id: p.id, name: p.name, original_name: p.name, custom_description: null, source: p.source, state: "off", loading: false, error: null, shots: [],
  can_correct: false, online: false, rawg_key_set: false, details: null,
  local: { added: 1, launches: 0, last_launch: null, play_ms: 0, play_est_ms: 0, target: "x" },
});
const calls = [];
let seq = 0;
// Faux cœur : mêmes règles que le vrai (noms uniques sans tenir compte de la casse, retrait des jeux à la suppression).
const checkName = (name, except) => {
  name = name.trim();
  if (!name) throw "Donne un nom à la collection.";
  if (state.collections.some((c) => c.id !== except && c.name.toLowerCase() === name.toLowerCase())) throw "Une collection porte déjà ce nom.";
  return name;
};
globalThis.window = {
  __TAURI__: {
    core: {
      convertFileSrc: (p) => `asset://${p}`,
      invoke: async (cmd, args) => {
        if (cmd === "get_strings") return STRINGS;
        calls.push([cmd, args]);
        if (cmd === "create_collection") {
          const name = checkName(args.name);
          state = { ...state, collections: [...state.collections, { id: `c${++seq}`, name }] };
          return state;
        }
        if (cmd === "rename_collection") {
          const name = checkName(args.name, args.id);
          state = { ...state, collections: state.collections.map((c) => (c.id === args.id ? { ...c, name } : c)) };
          return state;
        }
        if (cmd === "delete_collection") {
          state = { ...state, collections: state.collections.filter((c) => c.id !== args.id), programs: state.programs.map((p) => ({ ...p, collections: p.collections.filter((c) => c !== args.id) })) };
          return state;
        }
        if (cmd === "set_program_collection") {
          state = { ...state, programs: state.programs.map((p) => (p.id !== args.id ? p : { ...p, collections: args.member ? [...p.collections.filter((c) => c !== args.collection), args.collection] : p.collections.filter((c) => c !== args.collection) })) };
          return state;
        }
        if (cmd === "open_details" || cmd === "get_details") return detailsFor(state.programs.find((p) => p.id === args.id));
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
const view = byId["view"], overlay = byId["overlay"], toastEl = byId["toast"];
const flush = () => new Promise((r) => setTimeout(r, 0));
const name = (c) => c.find((m) => m.className === "meta").textContent;
const isCard = (c) => c.tag === "button" && c.className === "card";
const shown = () => view.findAll(isCard).map(name);
const heroName = () => view.find((c) => c.className === "hero")?.find((c) => c.tag === "h1").textContent ?? null;
const select = (label) => view.find((c) => c.tag === "select" && c.attrs["aria-label"] === label);
const wrapOf = (n) => view.findAll((c) => c.className === "card-wrap").find((w) => name(w) === n);
const submit = async (form, text) => { form.find((c) => c.tag === "input").value = text; await form.listeners.submit({ preventDefault() {} }); await flush(); };
const panel = () => overlay.children[0];
const rows = () => panel().findAll((c) => c.className === "setting-row coll-row");
const names = () => rows().map((r) => r.children[0].value);

// Sans collection : le bouton de gestion existe, pas le filtre
check("barre : bouton Collections, pas de filtre", [view.find((c) => c.className.includes("toolbar-button")) !== null, select("Filtrer par collection")], [true, null]);

// Gestion : création
await view.find((c) => c.className.includes("toolbar-button")).listeners.click();
check("gestion : liste vide", panel().find((c) => c.className === "no-match")?.textContent, "Aucune collection pour l'instant.");
await submit(panel().find((c) => c.tag === "form"), "  Coop ");
check("création : nom nettoyé envoyé", calls.filter((c) => c[0] === "create_collection").at(-1)[1].name, "  Coop ");
check("création : collection listée avec son compte", [names(), rows()[0].find((c) => c.className === "coll-count").textContent], [["Coop"], "0 jeu"]);
await submit(panel().find((c) => c.tag === "form"), "coop");
check("doublon refusé avec un message clair", [toastEl.textContent, names()], ["Une collection porte déjà ce nom.", ["Coop"]]);
await submit(panel().find((c) => c.tag === "form"), "   ");
check("nom vide refusé", toastEl.textContent, "Donne un nom à la collection.");

// Renommer
const nameInput = rows()[0].children[0];
nameInput.value = "Équipe";
await nameInput.listeners.change();
await flush();
check("renommer : commande puis liste à jour", [calls.filter((c) => c[0] === "rename_collection").at(-1)[1], names()], [{ id: "c1", name: "Équipe", tz: new Date().getTimezoneOffset() }, ["Équipe"]]);

// Fermer : le filtre par collection apparaît
await panel().find((c) => c.tag === "button" && c.textContent === "Fermer").listeners.click();
check("filtre par collection proposé", select("Filtrer par collection").children.map((o) => o.textContent), ["Toutes les collections", "Équipe"]);

// Ranger un jeu depuis sa carte
await wrapOf("Alpha").find((c) => c.textContent === "Collections").listeners.click();
check("carte : fenêtre du jeu", [panel().find((c) => c.tag === "h2").textContent, panel().findAll((c) => c.className === "switch").map((r) => [r.children[0].textContent, r.attrs["aria-checked"]])], ["Collections de Alpha", [["Équipe", "false"]]]);
await panel().find((c) => c.className === "switch").listeners.click();
await flush();
check("carte : jeu rangé", [calls.filter((c) => c[0] === "set_program_collection").at(-1)[1], panel().find((c) => c.className === "switch").attrs["aria-checked"]], [{ id: "a", collection: "c1", member: true, tz: new Date().getTimezoneOffset() }, "true"]);

// Créer une collection depuis la fenêtre du jeu : le jeu y est rangé aussitôt
await submit(panel().find((c) => c.tag === "form"), "Solo");
check("création depuis le jeu : rangé dans la nouvelle collection", panel().findAll((c) => c.className === "switch").map((r) => [r.children[0].textContent, r.attrs["aria-checked"]]), [["Équipe", "true"], ["Solo", "true"]]);
await panel().find((c) => c.tag === "button" && c.textContent === "Fermer").listeners.click();

// Filtrer par collection
await wrapOf("Beta").find((c) => c.textContent === "Collections").listeners.click();
await panel().findAll((c) => c.className === "switch")[1].listeners.click();
await flush();
await panel().find((c) => c.tag === "button" && c.textContent === "Fermer").listeners.click();
const filter = select("Filtrer par collection");
filter.value = "c2";
await filter.listeners.change();
await flush();
check("filtre Solo : Alpha et Beta, pas de suggestion", [shown(), heroName()], [["Alpha", "Beta"], null]);
const field = view.find((c) => c.className === "text-input toolbar-search");
field.value = "alp";
await field.listeners.input();
await flush();
check("filtre + recherche", shown(), ["Alpha"]);
field.value = "";
await field.listeners.input();
await flush();
filter.value = "";
await filter.listeners.change();
await flush();
check("filtre retiré : suggestion et tous les jeux", [heroName(), shown()], ["Zero", ["Alpha", "Beta", "Gamma", "Delta"]]);

// Page d'un jeu
await wrapOf("Gamma").find((c) => c.textContent === "Détails").listeners.click();
await flush();
const pageButton = view.find((c) => c.tag === "button" && c.textContent === "Collections");
check("page du jeu : bouton Collections", pageButton !== null, true);
await pageButton.listeners.click();
check("page du jeu : fenêtre ouverte", panel().find((c) => c.tag === "h2").textContent, "Collections de Gamma");
await panel().find((c) => c.tag === "button" && c.textContent === "Fermer").listeners.click();
await view.find((c) => c.textContent === "← Retour").listeners.click();

// Suppression en deux temps, depuis la gestion
filter.value = "c2";
await filter.listeners.change();
await view.find((c) => c.className.includes("toolbar-button")).listeners.click();
check("gestion : comptes", rows().map((r) => r.find((c) => c.className === "coll-count").textContent), ["1 jeu", "2 jeux"]);
const deleteCalls = () => calls.filter((c) => c[0] === "delete_collection").length;
await rows()[1].find((c) => c.tag === "button").listeners.click();
check("suppression : 1er clic demande confirmation, rien supprimé", [rows()[1].find((c) => c.tag === "button").textContent, deleteCalls()], ["Confirmer ?", 0]);
await rows()[0].find((c) => c.tag === "button").listeners.click();
check("suppression : un clic ailleurs change la cible à confirmer", [rows()[0].find((c) => c.tag === "button").textContent, rows()[1].find((c) => c.tag === "button").textContent, deleteCalls()], ["Confirmer ?", "Supprimer", 0]);
await rows()[1].find((c) => c.tag === "button").listeners.click();
await rows()[1].find((c) => c.tag === "button").listeners.click();
await flush();
check("suppression confirmée : collection retirée", [calls.filter((c) => c[0] === "delete_collection").at(-1)[1].id, names()], ["c2", ["Équipe"]]);
check("suppression : les jeux restent, la collection est retirée de leurs données", [state.programs.length, state.programs.some((p) => p.collections.includes("c2"))], [5, false]);
await panel().find((c) => c.tag === "button" && c.textContent === "Fermer").listeners.click();
check("suppression : filtre actif remis à « toutes »", [select("Filtrer par collection").value, shown().length, heroName()], ["", 4, "Zero"]);

process.exit(ok ? 0 : 1);
