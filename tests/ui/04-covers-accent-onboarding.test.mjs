// Pochettes, couleur d'accentuation et premier lancement avec téléchargement.
import { pathToFileURL } from "node:url";
import { STRINGS } from "./strings.mjs";

class El {
  constructor(tag) {
    Object.assign(this, { tag, children: [], attrs: {}, listeners: {}, className: "", textContent: "", hidden: false, title: "", value: "" });
    const props = {};
    this.style = { setProperty: (k, v) => (props[k] = v), props, cssText: "" };
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
}

const byId = {};
const docEl = new El("html");
globalThis.document = {
  getElementById: (id) => (byId[id] ??= new El(id)),
  createElement: (tag) => new El(tag),
  addEventListener() {},
  removeEventListener() {},
  documentElement: docEl,
  body: new El("body"),
};
globalThis.requestAnimationFrame = (cb) => cb();

const calls = [];
const program = (id, name, extra = {}) => ({
  id, name, target: "x", source: "manual", added: 1, launches: [], cover: null, cover_kind: null, cover_ver: 0, ...extra,
});
let state = {
  onboarded: true,
  platforms: [],
  programs: [
    program("a", "Avec pochette", { cover: "C:\\covers\\a.jpg", cover_kind: "art", cover_ver: 7 }),
    program("b", "Avec icône", { cover: "C:\\covers\\b.icon.png", cover_kind: "icon", cover_ver: 3 }),
    program("c", "Image du joueur", { cover: "C:\\covers\\c.custom.png", cover_kind: "custom", cover_ver: 9 }),
    program("d", "Sans image"),
  ],
  close_to_tray: true,
  shortcut: "Ctrl+Alt+Space",
  shortcut_ok: true,
  accent: "#FF7A45",
  accent_id: "braise",
  accents: [
    { id: "tronix", label: "Tronix", hex: "#3DF5C4" },
    { id: "braise", label: "Braise", hex: "#FF7A45" },
  ],
  download_covers: false,
  steamgriddb_key_set: false,
};
let dialogPath = "C:\\images\\ma-pochette.png";
globalThis.window = {
  __TAURI__: {
    core: {
      convertFileSrc: (p) => `http://asset.localhost/${encodeURIComponent(p)}`,
      invoke: async (cmd, args) => {
        if (cmd === "get_strings") return STRINGS;
        calls.push([cmd, args]);
        if (cmd === "set_accent") return "#3DF5C4";
        if (cmd === "set_custom_cover" || cmd === "clear_custom_cover") return state.programs;
        if (cmd === "set_platforms") return { ...state, onboarded: true };
        if (cmd === "import_all") return { programs: state.programs, searched: 1, added: [] };
        return state;
      },
    },
    dialog: { open: async () => dialogPath },
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
const hero = view.find((c) => c.className === "hero");
const cards = view.findAll((c) => c.className === "card-wrap");

check("couleur appliquée au démarrage", docEl.style.props["--accent"], "#FF7A45");
// Le premier du classement est « Avec pochette » : son image est dans la suggestion.
const art = hero.find((c) => c.className === "hero-art");
check("suggestion : pochette affichée", art?.src, "http://asset.localhost/C%3A%5Ccovers%5Ca.jpg?v=7");
check("cartes : une par programme restant", cards.length, 3);
check("carte avec icône", cards[0].find((c) => c.tag === "img")?.className, "cover-icon");
check("carte avec image du joueur", cards[1].find((c) => c.tag === "img")?.className, "cover-img");
check("carte sans image", cards[2].find((c) => c.tag === "img"), null);

// Choisir une image : bouton « Image » de la suggestion, puis « Choisir une image… »
hero.find((c) => c.tag === "button" && c.textContent === "Image").listeners.click();
const panel = byId["overlay"].children[0];
check("panneau : pas de « Retirer » sans image du joueur", panel.find((c) => c.textContent === "Retirer mon image"), null);
await panel.find((c) => c.textContent === "Choisir une image…").listeners.click();
check("image choisie envoyée", calls.at(-1), ["set_custom_cover", { id: "a", path: "C:\\images\\ma-pochette.png", tz: new Date().getTimezoneOffset() }]);

// Retirer l'image du joueur (programme « c »)
cards[1].findAll((c) => c.className === "card-edit" && c.textContent === "Image").at(-1).listeners.click();
const panel2 = byId["overlay"].children[0];
await panel2.find((c) => c.textContent === "Retirer mon image").listeners.click();
check("retrait envoyé", calls.at(-1)[0], "clear_custom_cover");

// Paramètres : couleur, téléchargement, clé
await byId["settings"].listeners.click();
const settings = byId["overlay"].children[0];
const swatches = settings.find((c) => c.className === "swatches").children;
await swatches[0].listeners.click();
check("couleur choisie", calls.at(-1), ["set_accent", { id: "tronix" }]);
check("couleur appliquée", docEl.style.props["--accent"], "#3DF5C4");
const rows = settings.findAll((c) => c.attrs.role === "switch");
const dl = rows.find((c) => c.children[0]?.textContent === "Télécharger les pochettes manquantes");
await dl.listeners.click();
check("téléchargement activé", calls.at(-1), ["set_download_covers", { enabled: true, key: null }]);
const key = settings.findAll((c) => c.className === "text-input").find((c) => c.placeholder.includes("SteamGridDB"));
key.value = " ma-cle ";
await key.listeners.change();
check("clé envoyée", calls.at(-1), ["set_download_covers", { enabled: true, key: " ma-cle " }]);
await settings.find((c) => c.textContent === "Télécharger maintenant").listeners.click();
check("téléchargement relancé", calls.at(-1)[0], "fetch_covers");

// Premier lancement : le téléchargement est proposé, désactivé d'office
state = { ...state, onboarded: false, platforms: [{ id: "steam", label: "Steam", enabled: true, detected: true }] };
calls.length = 0;
await import(pathToFileURL(process.argv[2]).href + "?second");
const tile = byId["view"].findAll((c) => c.className === "tile tile-wide").find((t) => t.children[0]?.textContent.startsWith("Télécharger"));
check("premier lancement : tuile du téléchargement", tile?.attrs["aria-pressed"], "false");
tile.listeners.click();
check("tuile cochée", tile.attrs["aria-pressed"], "true");
const next = byId["view"].find((c) => c.tag === "button" && c.textContent === "Continuer");
await next.listeners.click();
const sets = calls.filter((c) => c[0].startsWith("set_"));
check("choix envoyé avant les launchers", sets.filter((c) => c[0] !== "set_online_info").slice(0, 2).map((c) => c[0]), ["set_download_covers", "set_platforms"]);
check("téléchargement demandé", sets[0][1], { enabled: true, key: null });

process.exit(ok ? 0 : 1);
