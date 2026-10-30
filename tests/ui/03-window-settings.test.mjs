// Réglages de la fenêtre dans les Paramètres : croix de fermeture et capture du raccourci.
import { pathToFileURL } from "node:url";
import { STRINGS } from "./strings.mjs";

class El {
  constructor(tag) {
    Object.assign(this, { tag, children: [], attrs: {}, listeners: {}, className: "", textContent: "", hidden: false });
    this.style = { setProperty() {}, cssText: "" };
    this.classList = { add() {}, remove() {} };
  }
  append(...n) { this.children.push(...n); }
  replaceChildren(...n) { this.children = [...n]; }
  setAttribute(k, v) { this.attrs[k] = v; }
  addEventListener(t, f) { this.listeners[t] = f; }
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
}

const byId = {};
const docListeners = [];
globalThis.document = {
  documentElement: new El("html"),
  getElementById: (id) => (byId[id] ??= new El(id)),
  createElement: (tag) => new El(tag),
  addEventListener: (type, fn, capture) => docListeners.push({ type, fn, capture }),
  removeEventListener: (type, fn) => {
    const i = docListeners.findIndex((l) => l.fn === fn);
    if (i >= 0) docListeners.splice(i, 1);
  },
  body: new El("body"),
};
globalThis.requestAnimationFrame = (cb) => cb();

const calls = [];
let shortcutResult = (accelerator) => accelerator;
const state = {
  onboarded: true,
  platforms: [{ id: "steam", label: "Steam", enabled: true, detected: false }],
  programs: [],
  close_to_tray: true,
  shortcut: "Ctrl+Alt+Space",
  shortcut_ok: false,
  accent: "#3DF5C4", accent_id: "tronix", accents: [], download_covers: false, steamgriddb_key_set: false,
};
globalThis.window = {
  __TAURI__: {
    core: {
      invoke: async (cmd, args) => {
        if (cmd === "get_strings") return STRINGS;
        calls.push([cmd, args]);
        if (cmd === "set_shortcut") return shortcutResult(args.shortcut);
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
const toast = () => byId["toast"].textContent;

check("avertissement si le raccourci est refusé au démarrage", toast(), "Le raccourci Ctrl+Alt+Espace est déjà utilisé : choisis-en un autre dans les paramètres.");

await byId["settings"].listeners.click();
const panel = byId["overlay"].children[0];
const closeRow = panel.find((c) => c.attrs.role === "switch" && c.children[0]?.textContent?.startsWith("La croix"));
await closeRow.listeners.click();
check("croix : appel envoyé", calls.at(-1), ["set_close_to_tray", { enabled: false }]);
check("croix : interrupteur éteint", closeRow.attrs["aria-checked"], "false");

const row = panel.find((c) => c.className === "setting-row" && c.children[0]?.textContent?.startsWith("Raccourci"));
const button = row.children[1];
check("bouton du raccourci", button.textContent, "Ctrl+Alt+Espace");
button.listeners.click();
check("mode capture", button.textContent, "Appuie sur ta combinaison…");
const press = async (init) => {
  const event = { preventDefault() {}, stopPropagation() {}, ...init };
  for (const l of [...docListeners]) if (l.type === "keydown" && l.capture) await l.fn(event);
};
await press({ code: "ControlLeft", key: "Control", ctrlKey: true });
check("touche modificateur seule ignorée", button.textContent, "Appuie sur ta combinaison…");
await press({ code: "KeyJ", key: "j", ctrlKey: true, altKey: false });
check("combinaison envoyée", calls.at(-1), ["set_shortcut", { shortcut: "Ctrl+J" }]);
check("bouton mis à jour", button.textContent, "Ctrl+J");

button.listeners.click();
shortcutResult = () => { throw "Ce raccourci est invalide ou déjà utilisé par un autre programme."; };
await press({ code: "Space", key: " ", ctrlKey: true, shiftKey: true });
check("raccourci refusé : message", toast(), "Ce raccourci est invalide ou déjà utilisé par un autre programme.");
check("raccourci refusé : ancien affiché", button.textContent, "Ctrl+J");

button.listeners.click();
await press({ code: "Escape", key: "Escape" });
check("Échap annule la capture", button.textContent, "Ctrl+J");
check("plus d'écouteur de capture", docListeners.filter((l) => l.capture).length, 0);

process.exit(ok ? 0 : 1);
