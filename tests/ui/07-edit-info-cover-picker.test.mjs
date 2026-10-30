// Modification du titre et de la description, et sélecteur de pochettes SteamGridDB.
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

const program = (id, name, extra = {}) => ({
  id, name, original_name: name, target: "C:\\a.exe", source: "manual", added: 1_700_000_000_000, launches: [], play_ms: 0, play_est_ms: 0,
  cover: null, cover_kind: null, cover_ver: 0, custom_name: null, custom_description: null, ...extra,
});
let state = {
  onboarded: true, platforms: [], close_to_tray: true, shortcut: "Ctrl+Alt+Space", shortcut_ok: true,
  accent: "#3DF5C4", accent_id: "tronix", accents: [], download_covers: false, steamgriddb_key_set: true,
  online_info: true, rawg_key_set: false, programs: [program("p1", "blender-launcher")],
};
const detailsView = (extra) => ({
  id: "p1", name: "blender-launcher", original_name: "blender-launcher", custom_description: null, source: "manual", state: "ready",
  loading: false, error: null, shots: [], can_correct: true, online: true, rawg_key_set: false,
  details: { provider: "wikipedia", match_id: "fr:Blender", match_name: "Blender", lang: "fr", summary: "logiciel libre", description: "Texte de la source.", genres: [], developers: [], publishers: [], platforms: [], news: [], page_url: "https://fr.wikipedia.org/wiki/Blender" },
  local: { added: 1_700_000_000_000, launches: 0, last_launch: null, play_ms: 0, play_est_ms: 0, target: "C:\\a.exe" },
  ...extra,
});
let currentDetails = detailsView();
const calls = [];
let sgdbFailure = null;
globalThis.window = {
  __TAURI__: {
    core: {
      convertFileSrc: (p) => `asset://${p}`,
      invoke: async (cmd, args) => {
        if (cmd === "get_strings") return STRINGS;
        calls.push([cmd, args]);
        if (cmd === "open_details" || cmd === "get_details") return currentDetails;
        if (cmd === "set_program_info") {
          state.programs = [program("p1", args.name || "blender-launcher", { original_name: "blender-launcher", custom_name: args.name || null, custom_description: args.description || null })];
          currentDetails = detailsView({ name: args.name || "blender-launcher", custom_description: args.description || null });
          return state.programs;
        }
        if (cmd === "sgdb_search") {
          if (sgdbFailure) throw sgdbFailure;
          return args.query === "inconnu" ? [] : [{ id: 7, name: "Blender", year: 2002 }, { id: 8, name: "Blender 2", year: null }];
        }
        if (cmd === "sgdb_grids") return args.gameId === 7 ? [{ id: 1, url: "https://cdn2.steamgriddb.com/grid/a.png", thumb: "https://cdn2.steamgriddb.com/thumb/a.jpg", style: "alternate", author: "Moi" }] : [];
        if (cmd === "sgdb_pick") return state.programs;
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
const overlay = byId["overlay"];
const click = async (root, label) => root.find((c) => c.tag === "button" && c.textContent === label).listeners.click();
const flush = () => new Promise((r) => setTimeout(r, 0));

// Ouvrir la page
await click(view.find((c) => c.className === "hero"), "Détails");
check("page : titre affiché", view.find((c) => c.tag === "h1")?.textContent, "blender-launcher");
check("page : bouton Modifier présent, pas de Rétablir", [view.find((c) => c.textContent === "Modifier") !== null, view.find((c) => c.textContent === "Rétablir l'original")], [true, null]);

// Mode modification
await click(view, "Modifier");
const nameInput = view.find((c) => c.className === "d-title-input");
const textarea = view.find((c) => c.className === "d-textarea");
check("modification : titre prérempli", nameInput.value, "blender-launcher");
check("modification : description de la source préremplie", textarea.value, "Texte de la source.");
check("modification : boutons", view.findAll((c) => c.tag === "button" && ["Enregistrer", "Annuler"].includes(c.textContent)).length, 2);

// Annuler
await click(view, "Annuler");
check("annuler : retour à l'affichage", [view.find((c) => c.className === "d-title-input"), view.find((c) => c.tag === "h1")?.textContent], [null, "blender-launcher"]);

// Changer le titre seulement : la description identique à la source n'est pas enregistrée comme modification
await click(view, "Modifier");
view.find((c) => c.className === "d-title-input").value = "Blender";
await click(view, "Enregistrer");
await flush();
check("enregistrement du titre seul", calls.filter((c) => c[0] === "set_program_info").at(-1)[1], { id: "p1", name: "Blender", description: "", tz: new Date().getTimezoneOffset() });
check("titre mis à jour sur la page", view.find((c) => c.tag === "h1")?.textContent, "Blender");

// Changer la description
await click(view, "Modifier");
view.find((c) => c.className === "d-textarea").value = "Ma description à moi.";
await click(view, "Enregistrer");
await flush();
check("enregistrement de la description", calls.filter((c) => c[0] === "set_program_info").at(-1)[1].description, "Ma description à moi.");
check("description du joueur affichée et signalée", [view.find((c) => c.className === "d-text")?.text, view.find((c) => c.className === "estimated")?.textContent], ["Ma description à moi.", "modifiée"]);

// Rétablir l'original
await click(view, "Modifier");
check("rétablir : proposé quand il y a une modification", view.find((c) => c.textContent === "Rétablir l'original") !== null, true);
await click(view, "Rétablir l'original");
await flush();
check("rétablir : texte vides envoyés", calls.filter((c) => c[0] === "set_program_info").at(-1)[1], { id: "p1", name: "", description: "", tz: new Date().getTimezoneOffset() });
check("rétablir : titre d'origine", view.find((c) => c.tag === "h1")?.textContent, "blender-launcher");

// Programme sans page : on peut écrire une description
currentDetails = detailsView({ state: "none", details: null });
await click(view, "← Retour");
await click(view.find((c) => c.className === "hero"), "Détails");
check("sans page : bannière", view.find((c) => c.className === "d-banner")?.text.startsWith("Aucune page trouvée"), true);
check("sans page : pas de description", view.find((c) => c.className === "d-text"), null);
await click(view, "Modifier");
check("sans page : champ de description vide", view.find((c) => c.className === "d-textarea").value, "");
view.find((c) => c.className === "d-textarea").value = "Écrite à la main.";
await click(view, "Enregistrer");
await flush();
check("sans page : description enregistrée telle quelle", calls.filter((c) => c[0] === "set_program_info").at(-1)[1].description, "Écrite à la main.");
check("sans page : description affichée", view.find((c) => c.className === "d-text")?.text, "Écrite à la main.");
await click(view, "← Retour");

// Sélecteur SteamGridDB depuis le panneau Image
await click(view.find((c) => c.className === "hero"), "Image");
await click(overlay, "Chercher sur SteamGridDB…");
await flush();
const panel = overlay.children[0];
check("sélecteur : recherche lancée avec le titre affiché", calls.find((c) => c[0] === "sgdb_search")[1], { query: "blender-launcher" });
check("sélecteur : jeux proposés", panel.findAll((c) => c.className === "sgdb-game").map((c) => c.textContent), ["Blender (2002)", "Blender 2"]);
check("sélecteur : pochettes du premier jeu", [calls.find((c) => c[0] === "sgdb_grids")[1], panel.findAll((c) => c.className === "sgdb-cover").length], [{ gameId: 7 }, 1]);
check("sélecteur : miniature distante", panel.find((c) => c.className === "sgdb-cover").src, "https://cdn2.steamgriddb.com/thumb/a.jpg");
await panel.find((c) => c.className === "sgdb-cover").listeners.click();
await flush();
check("choix de la pochette", calls.find((c) => c[0] === "sgdb_pick")[1], { id: "p1", url: "https://cdn2.steamgriddb.com/grid/a.png", tz: new Date().getTimezoneOffset() });
check("sélecteur fermé après le choix", overlay.hidden, true);

// Autre jeu sans pochette, recherche libre sans résultat, erreur réseau
await click(view.find((c) => c.className === "hero"), "Image");
await click(overlay, "Chercher sur SteamGridDB…");
await flush();
const panel2 = overlay.children[0];
await panel2.findAll((c) => c.className === "sgdb-game")[1].listeners.click();
await flush();
check("jeu sans pochette verticale : message", panel2.find((c) => c.className === "sgdb-status")?.textContent, "Aucune pochette verticale pour ce jeu.");
panel2.find((c) => c.className === "text-input").value = "inconnu";
panel2.find((c) => c.tag === "form").listeners.submit({ preventDefault() {} });
await flush();
check("recherche sans résultat : message", panel2.find((c) => c.className === "sgdb-status")?.textContent, "Aucun jeu trouvé. Essaie un autre nom.");
sgdbFailure = "La clé SteamGridDB a été refusée.";
panel2.find((c) => c.tag === "form").listeners.submit({ preventDefault() {} });
await flush();
check("clé refusée : message", panel2.find((c) => c.className === "sgdb-status")?.textContent, "La clé SteamGridDB a été refusée.");
overlay.hidden = true;

// Sans clé SteamGridDB
state.steamgriddb_key_set = false;
await import(pathToFileURL(process.argv[2]).href + "?sans-cle");
await click(byId["view"].find((c) => c.className === "hero"), "Image");
await click(byId["overlay"], "Chercher sur SteamGridDB…");
check("sans clé : message et accès aux paramètres", [byId["overlay"].children[0].text.includes("clé SteamGridDB"), byId["overlay"].children[0].find((c) => c.textContent === "Ouvrir les paramètres") !== null], [true, true]);

process.exit(ok ? 0 : 1);
