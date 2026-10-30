import { setStrings, t } from "./i18n.js";

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const input = document.getElementById("q");
const list = document.getElementById("list");
const msg = document.getElementById("msg");
const tz = new Date().getTimezoneOffset();
const MAX_ROWS = 8;
const COLORS = ["#3df5c4", "#ff7a45", "#5aa9ff", "#ffc145", "#ff5470"];
const BRANDS = { steam: "Steam", epic: "Epic", gog: "GOG", ubisoft: "Ubisoft", xbox: "Xbox", ea: "EA", battlenet: "Battle.net" };
const sourceName = (source) => (source === "manual" ? t("source.manual") : (BRANDS[source] ?? source));

let programs = [];
let shown = [];
let selected = 0;
let openedAt = 0;

function hash(text) {
  let h = 0;
  for (const ch of text) h = (h * 31 + ch.charCodeAt(0)) >>> 0;
  return h;
}

function el(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

function row(program, index) {
  const node = el("button", "q-item");
  node.setAttribute("role", "option");
  node.setAttribute("aria-selected", String(index === selected));
  const icon = el("span", "q-ico", program.name.charAt(0).toUpperCase());
  icon.style.background = `linear-gradient(160deg, ${COLORS[hash(program.name) % COLORS.length]}, #0e1017 90%)`;
  if (program.cover) {
    const img = el("img", program.cover_kind === "icon" ? "q-img q-img-icon" : "q-img");
    img.alt = "";
    img.src = `${window.__TAURI__.core.convertFileSrc(program.cover)}?v=${program.cover_ver}`;
    img.addEventListener("error", () => img.remove());
    icon.append(img);
  }
  const text = el("span", "q-name", program.name);
  text.append(el("small", "", sourceName(program.source)));
  node.append(icon, text);
  node.addEventListener("click", () => launch(program));
  return node;
}

function render() {
  const query = input.value.trim().toLowerCase();
  // La liste garde l'ordre de probabilité ; la saisie ne fait que filtrer.
  const matches = query ? programs.filter((p) => p.name.toLowerCase().includes(query)) : programs;
  shown = matches.slice(0, MAX_ROWS);
  selected = Math.min(selected, Math.max(shown.length - 1, 0));
  list.replaceChildren(...shown.map(row));
  if (shown.length === 0) {
    list.append(el("div", "q-empty", programs.length === 0 ? t("quick.empty") : t("quick.no_result")));
  }
  list.children[selected]?.scrollIntoView?.({ block: "nearest" });
}

async function reload() {
  // La langue a pu changer dans la fenêtre principale depuis la dernière ouverture.
  setStrings(await invoke("get_strings"));
  input.placeholder = t("quick.placeholder");
  document.documentElement.setAttribute("lang", t("meta.locale").slice(0, 2));
  const state = await invoke("get_state", { tz });
  const root = document.documentElement;
  const light = state.look?.theme === "light" || (state.look?.theme === "auto" && window.matchMedia?.("(prefers-color-scheme: light)").matches);
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(state.accent.slice(i, i + 2), 16) / 255);
  root.setAttribute("data-theme", light ? "light" : "dark");
  root.setAttribute("data-radius", state.look?.radius ?? "round");
  root.style.setProperty("--accent", state.accent);
  root.style.setProperty("--accent-ink", 0.2126 * r + 0.7152 * g + 0.0722 * b > 0.5 ? "#04120d" : "#ffffff");
  programs = state.programs.filter((p) => !p.uninstalled);
  render();
}

async function launch(program) {
  if (!program) return;
  try {
    await invoke("launch_program", { id: program.id, tz });
    await invoke("hide_quick");
  } catch (error) {
    msg.textContent = String(error);
  }
}

function open() {
  input.value = "";
  selected = 0;
  msg.textContent = "";
  openedAt = Date.now();
  input.focus();
  reload().then(() => input.focus());
}

input.addEventListener("input", () => {
  selected = 0;
  render();
});

document.addEventListener("keydown", (event) => {
  if (event.key === "Escape") {
    invoke("hide_quick");
  } else if (event.key === "Enter") {
    launch(shown[selected]);
  } else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
    event.preventDefault();
    if (shown.length === 0) return;
    const step = event.key === "ArrowDown" ? 1 : -1;
    selected = (selected + step + shown.length) % shown.length;
    render();
  }
});

// Un clic ailleurs ferme la mini-fenêtre ; on ignore la perte de focus qui suit l'ouverture.
window.addEventListener("blur", () => {
  if (Date.now() - openedAt > 400) invoke("hide_quick");
});

await listen("quick-open", open);
open();
