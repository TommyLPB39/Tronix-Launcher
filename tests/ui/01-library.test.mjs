// Fait tourner src/main.js avec un faux DOM et un faux cœur Tauri, puis rejoue :
// ouvrir les Paramètres, activer Epic, puis GOG.
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
  find(pred) {
    for (const c of this.children) {
      if (typeof c !== "object") continue;
      if (pred(c)) return c;
      const d = c.find(pred);
      if (d) return d;
    }
    return null;
  }
  querySelector(sel) { return this.find((c) => c.tag === sel); }
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

const ids = ["steam", "epic", "gog", "ubisoft"];
let disabled = ["epic", "gog", "ubisoft"];
const sent = [];
const view = () => ({
  onboarded: true,
  platforms: ids.map((id) => ({ id, label: id, enabled: !disabled.includes(id), detected: false })),
  programs: [],
  close_to_tray: true,
  shortcut: "Ctrl+Alt+Space",
  shortcut_ok: true,
  accent: "#3DF5C4", accent_id: "tronix", accents: [], download_covers: false, steamgriddb_key_set: false,
});
globalThis.window = {
  __TAURI__: {
    core: {
      invoke: async (cmd, args) => {
        if (cmd === "get_strings") return STRINGS;
        if (cmd === "set_platforms") {
          sent.push(args.enabled);
          disabled = ids.filter((id) => !args.enabled.includes(id));
        }
        return view();
      },
    },
    event: { listen: async () => () => {} },
    dialog: { open: async () => null },
  },
};

await import(pathToFileURL(process.argv[2]).href);
await byId["settings"].listeners.click();
const rows = byId["overlay"].children[0].find((c) => c.className === "switches").children;
await rows[1].listeners.click(); // Epic
await rows[2].listeners.click(); // GOG
let ok = true;
const check = (label, got, want) => {
  const pass = JSON.stringify(got) === JSON.stringify(want);
  ok &&= pass;
  console.log(`${pass ? "OK " : "KO "} ${label} -> ${JSON.stringify(got)}`);
};
check("listes de launchers envoyées", sent, [["steam", "epic"], ["steam", "epic", "gog"]]);
check("launchers désactivés au final", disabled, ["ubisoft"]);
process.exit(ok ? 0 : 1);
