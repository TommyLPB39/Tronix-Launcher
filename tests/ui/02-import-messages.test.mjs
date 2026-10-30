// Fait tourner src/main.js avec un faux DOM : messages d'import et erreur de la boîte de dialogue.
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
  querySelector() { return null; }
}

const byId = {};
globalThis.document = {
  documentElement: new El("html"),
  getElementById: (id) => (byId[id] ??= new El(id)),
  createElement: (tag) => new El(tag),
  addEventListener() {},
  body: new El("body"),
};
globalThis.requestAnimationFrame = (cb) => cb();

const state = { onboarded: true, platforms: [], programs: [], close_to_tray: true, shortcut: "Ctrl+Alt+Space", shortcut_ok: true };
let importReport = { programs: [], searched: 0, added: [] };
let dialogError = null;
globalThis.window = {
  __TAURI__: {
    core: {
      invoke: async (cmd) => (cmd === "get_strings" ? STRINGS : cmd === "import_all" ? importReport : state),
    },
    event: { listen: async () => () => {} },
    dialog: {
      open: async () => {
        if (dialogError) throw new Error(dialogError);
        return null;
      },
    },
  },
};

await import(pathToFileURL(process.argv[2]).href);
const toast = () => byId["toast"].textContent;
let ok = true;
const check = (label, got, want) => {
  const pass = got === want;
  ok &&= pass;
  console.log(`${pass ? "OK " : "KO "} ${label} -> ${JSON.stringify(got)}`);
};

importReport = { programs: [], searched: 0, added: [] };
await byId["import"].listeners.click();
check("import, aucun launcher activé", toast(), "Aucun launcher activé : active-en un dans les paramètres.");

importReport = { programs: [], searched: 2, added: [] };
await byId["import"].listeners.click();
check("import, rien de nouveau", toast(), "Aucun nouveau jeu trouvé.");

dialogError = "boîte de dialogue indisponible";
await byId["add"].listeners.click();
check("boîte de dialogue en erreur", toast(), "Error: boîte de dialogue indisponible");

process.exit(ok ? 0 : 1);
