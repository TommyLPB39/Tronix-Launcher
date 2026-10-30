// Jeux Steam non installes : section repliable, page avec Installer, import qui relit le compte.
import { pathToFileURL } from "node:url";
import { STRINGS } from "./strings.mjs";

class El {
  constructor(tag) {
    Object.assign(this, { tag, children: [], attrs: {}, listeners: {}, className: "", textContent: "", hidden: false, title: "", value: "", src: "", alt: "", placeholder: "", type: "" });
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

globalThis.localStorage = { getItem: () => null, setItem() {} };

const program = (id, name, extra = {}) => ({
  id, name, original_name: name, target: "C:\a.exe", source: "steam", added: 1_700_000_000_000, launches: [], play_ms: 0, play_est_ms: 0,
  cover: null, cover_kind: null, cover_ver: 0, custom_name: null, custom_description: null, uninstalled: false, ...extra,
});
let state = {
  onboarded: true, platforms: [], close_to_tray: true, shortcut: "Ctrl+Alt+Space", shortcut_ok: true,
  accent: "#3DF5C4", accent_id: "tronix", accents: [], download_covers: false, steamgriddb_key_set: false,
  online_info: false, rawg_key_set: false, show_uninstalled: true, steam_key_set: true,
  programs: [program("i1", "Installé Un"), program("i2", "Installé Deux"), program("u1", "Alpha", { uninstalled: true }), program("u2", "Bravo", { uninstalled: true }), program("u3", "Charlie", { uninstalled: true })],
};
const detailsFor = (id, name) => ({
  id, name, original_name: name, custom_description: null, source: "steam", state: "off", loading: false, error: null, shots: [],
  can_correct: false, online: false, rawg_key_set: false, details: null,
  local: { added: 1_700_000_000_000, launches: 0, last_launch: null, play_ms: 0, play_est_ms: 0, target: "steam://rungameid/1" },
});
const calls = [];
let syncError = null;
let importedCount = 0;
globalThis.window = {
  __TAURI__: {
    core: {
      convertFileSrc: (p) => `asset://${p}`,
      invoke: async (cmd, args) => {
        if (cmd === "get_strings") return STRINGS;
        calls.push([cmd, args]);
        if (cmd === "open_details" || cmd === "get_details") return detailsFor(args.id, state.programs.find((p) => p.id === args.id).name);
        if (cmd === "import_all") return { programs: state.programs, searched: 1, added: importedCount ? [["steam", importedCount]] : [] };
        if (cmd === "sync_owned_games") {
          if (syncError) throw syncError;
          return { programs: state.programs, uninstalled: 3 };
        }
        if (cmd === "install_program" || cmd === "set_steam_account") return null;
        if (cmd === "launch_program") return state.programs;
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
const toastEl = byId["toast"];
const flush = () => new Promise((r) => setTimeout(r, 0));
const cards = (root) => root.findAll((c) => c.tag === "button" && (c.className === "card" || c.className === "card dim"));

// Section repliée d'office, avec le nombre de jeux
const toggle = () => view.find((c) => c.className === "section-toggle");
check("section : titre avec compteur", toggle()?.textContent, "Non installés (3)");
check("section : repliée d'office", [toggle().attrs["aria-expanded"], cards(view).filter((c) => c.className === "card dim").length], ["false", 0]);
check("accueil : le héros est un jeu installé", view.find((c) => c.tag === "h1")?.textContent, "Installé Un");
check("bibliothèque : jeux installés seulement", cards(view).map((c) => c.find((m) => m.className === "meta").textContent.slice(0, 13)), ["Installé Deux"]);

// Dépliée : trois pochettes grisées, nommées, avec la mention
toggle().listeners.click();
check("section dépliée", toggle().attrs["aria-expanded"], "true");
const dim = cards(view).filter((c) => c.className === "card dim");
check("pochettes grisées", dim.length, 3);
check("mention non installé", dim[0].find((m) => m.tag === "small").textContent, "Steam · non installé");

// Un clic sur un jeu grisé ouvre sa page, sans le lancer
await dim[0].listeners.click();
await flush();
check("clic : page ouverte, pas de lancement", [calls.some((c) => c[0] === "open_details" && c[1].id === "u1"), calls.some((c) => c[0] === "launch_program")], [true, false]);
check("page : Installer remplace Lancer", [view.find((c) => c.textContent === "⬇ Installer") !== null, view.find((c) => c.textContent === "▶ Lancer")], [true, null]);
check("page : étiquette Non installé", view.findAll((c) => c.className === "tag muted-tag").map((c) => c.textContent), ["Non installé"]);
await view.find((c) => c.textContent === "⬇ Installer").listeners.click();
await flush();
check("Installer : demande envoyée", calls.find((c) => c[0] === "install_program")[1], { id: "u1" });
check("Installer : message", toastEl.textContent, "Steam s'ouvre pour installer le jeu.");

// Un jeu installé garde « Lancer »
await view.find((c) => c.textContent === "← Retour").listeners.click();
await cards(view).find((c) => c.find((m) => m.className === "meta")?.textContent.startsWith("Installé Deux")).listeners.click();
await flush();
check("jeu installé : lancement normal", calls.filter((c) => c[0] === "launch_program").map((c) => c[1].id), ["i2"]);

// L'import relit ensuite le compte Steam
calls.length = 0;
await byId["import"].listeners.click();
await flush();
check("import : compte relu après l'import", calls.map((c) => c[0]).filter((n) => n === "import_all" || n === "sync_owned_games"), ["import_all", "sync_owned_games"]);
check("import : résultat annoncé", toastEl.textContent, "3 jeux Steam non installés.");
syncError = "Ton profil Steam est privé : rends les « détails des jeux » publics.";
await byId["import"].listeners.click();
await flush();
check("import : erreur affichée telle quelle", [toastEl.textContent, toastEl.className], [syncError, "toast show error"]);

// Jeux importés ET relecture en échec : l'erreur n'est pas écrasée par le message d'import
importedCount = 2;
await byId["import"].listeners.click();
await flush();
check("import réussi + compte refusé : l'erreur reste", toastEl.textContent, syncError);
syncError = null;
await byId["import"].listeners.click();
await flush();
check("import réussi + compte relu : message d'import", toastEl.textContent, "2 jeux importés (Steam 2)");
importedCount = 0;

// Fonction désactivée ou sans clé : aucun appel réseau
state = { ...state, show_uninstalled: false, programs: state.programs.filter((p) => !p.uninstalled) };
await import(pathToFileURL(process.argv[2]).href + "?off");
calls.length = 0;
await byId["import"].listeners.click();
await flush();
check("désactivé : pas de section", byId["view"].find((c) => c.className === "section-toggle"), null);
check("désactivé : pas de relecture du compte", calls.some((c) => c[0] === "sync_owned_games"), false);

// Uniquement des jeux non installés
state = { ...state, show_uninstalled: true, steam_key_set: true, programs: [program("u1", "Alpha", { uninstalled: true })] };
await import(pathToFileURL(process.argv[2]).href + "?only");
check("que des non installés : message vide + section", [byId["view"].find((c) => c.tag === "h1")?.textContent, byId["view"].find((c) => c.className === "section-toggle")?.textContent], ["Ta bibliothèque est vide", "Non installés (1)"]);

process.exit(ok ? 0 : 1);

