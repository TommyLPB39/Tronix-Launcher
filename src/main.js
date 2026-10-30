import { currentLang, locale, setStrings, t, tn } from "./i18n.js";

const { invoke } = window.__TAURI__.core;
const { open } = window.__TAURI__.dialog;

const view = document.getElementById("view");
const overlay = document.getElementById("overlay");
const toastEl = document.getElementById("toast");
const headerActions = document.getElementById("header-actions");

// Décalage horaire envoyé au cœur de l'application pour le classement par heure.
const tz = new Date().getTimezoneOffset();

let programs = [];
let platforms = [];
let onboarded = false;
let closeToTray = true;
let shortcut = "Ctrl+Alt+Space";
let capturing = false;
let screen = "library"; // "library" ou "stats"
let accents = [];
let accentId = "tronix";
let downloadCovers = false;
let keySet = false;
let onlineInfo = false;
let rawgKeySet = false;
let showUninstalled = false;
let steamKeySet = false;
let language = "auto"; // réglage : "auto", "fr" ou "en"
let toastTimer;

// Organisation de la bibliothèque : recherche et filtre (le temps de la session), tri (retenu).
let query = "";
let sourceFilter = "";
let sortMode = "rank";
let collections = []; // { id, name }, créées par le joueur
let collectionFilter = "";
let resultsEl = null;
const sortOptions = () => ["rank", "name", "recent", "playtime", "added"].map((value) => [value, t(`sort.${value}`)]);
const normalize = (text) => text.normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase();
const lastLaunch = (p) => (p.launches.length > 0 ? Math.max(...p.launches) : 0);

// Le classement par usage est déjà l'ordre reçu ; les autres tris se font ici.
function sorted(list) {
  const by = {
    name: (a, b) => normalize(a.name).localeCompare(normalize(b.name), locale()),
    recent: (a, b) => lastLaunch(b) - lastLaunch(a),
    playtime: (a, b) => b.play_ms + b.play_est_ms - (a.play_ms + a.play_est_ms),
    added: (a, b) => b.added - a.added,
  }[sortMode];
  return by ? [...list].sort(by) : list;
}

// Apparence choisie par le joueur (voir `Look` côté Rust) et couleur d'accent de base.
const DEFAULT_LOOK = { theme: "dark", card_size: "medium", radius: "round", view: "grid", show_hero: true, hero_background: false, custom_accent: "" };
let look = { ...DEFAULT_LOOK };
let accentHex = "#3df5c4";

// Couleur du texte posé sur l'accent : sombre sur un accent clair, blanc sur un accent foncé.
function inkFor(hex) {
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255);
  return 0.2126 * r + 0.7152 * g + 0.0722 * b > 0.5 ? "#04120d" : "#ffffff";
}

const lightQuery = window.matchMedia?.("(prefers-color-scheme: light)");

function applyLook() {
  const root = document.documentElement;
  const light = look.theme === "light" || (look.theme === "auto" && lightQuery?.matches);
  const accent = look.custom_accent || accentHex;
  root.setAttribute("data-theme", light ? "light" : "dark");
  root.setAttribute("data-size", look.card_size);
  root.setAttribute("data-radius", look.radius);
  root.style.setProperty("--accent", accent);
  root.style.setProperty("--accent-ink", inkFor(accent));
  updateBackdrop();
}
lightQuery?.addEventListener?.("change", () => look.theme === "auto" && applyLook());

// Fond de l'accueil : pochette du jeu suggéré, très floue (réglage désactivé d'office).
const backdrop = el("div", "backdrop");
document.body.append(backdrop);
function updateBackdrop() {
  const top = programs.find((p) => !p.uninstalled);
  const art = look.hero_background && top?.cover && top.cover_kind !== "icon" ? top : null;
  backdrop.className = art ? "backdrop on" : "backdrop";
  backdrop.style.backgroundImage = art ? `url("${window.__TAURI__.core.convertFileSrc(art.cover)}?v=${art.cover_ver}")` : "none";
}

// La section « Non installés » reste repliée tant que le joueur ne l'a pas ouverte ; son choix est retenu.
let uninstalledOpen = false;
try {
  uninstalledOpen = localStorage.getItem("uninstalledOpen") === "1";
} catch {}

const COLORS = ["#3df5c4", "#ff7a45", "#5aa9ff", "#ffc145", "#ff5470"];

function hash(text) {
  let h = 0;
  for (const ch of text) h = (h * 31 + ch.charCodeAt(0)) >>> 0;
  return h;
}

function coverStyle(name) {
  const color = COLORS[hash(name) % COLORS.length];
  return `background: linear-gradient(160deg, ${color}, #0e1017 85%)`;
}

// Les noms des launchers sont des marques : seul « Ajouté » (programme ajouté à la main) se traduit.
const BRANDS = { steam: "Steam", epic: "Epic", gog: "GOG", ubisoft: "Ubisoft", xbox: "Xbox", ea: "EA", battlenet: "Battle.net" };
const sourceName = (source) => (source === "manual" ? t("source.manual") : (BRANDS[source] ?? source));

function toast(message, isError = false) {
  toastEl.textContent = message;
  toastEl.className = `toast show${isError ? " error" : ""}`;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (toastEl.className = "toast"), Math.max(3500, message.length * 70));
}

function el(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

function button(className, text, onClick) {
  const node = el("button", className, text);
  node.addEventListener("click", onClick);
  return node;
}

function applyState(state) {
  onboarded = state.onboarded;
  platforms = state.platforms;
  programs = state.programs;
  closeToTray = state.close_to_tray;
  shortcut = state.shortcut;
  accents = state.accents;
  accentId = state.accent_id;
  downloadCovers = state.download_covers;
  keySet = state.steamgriddb_key_set;
  onlineInfo = state.online_info;
  rawgKeySet = state.rawg_key_set;
  showUninstalled = state.show_uninstalled;
  steamKeySet = state.steam_key_set;
  accentHex = state.accent ?? accentHex;
  sortMode = state.sort ?? "rank";
  collections = state.collections ?? [];
  language = state.language ?? "auto";
  look = { ...DEFAULT_LOOK, ...state.look };
  applyLook();
}

// ------------------------------------------------------------ Bibliothèque

// Image du programme (pochette, image du joueur ou icône), ou rien s'il n'y en a pas.
function coverImage(program, className) {
  if (!program.cover) return null;
  const img = el("img", className);
  img.alt = "";
  img.draggable = false;
  img.src = `${window.__TAURI__.core.convertFileSrc(program.cover)}?v=${program.cover_ver}`;
  // Image illisible : on retombe sur la lettre et le dégradé.
  img.addEventListener("error", () => img.remove());
  return img;
}

function card(program, index) {
  const wrap = el("div", "card-wrap");
  wrap.style.setProperty("--i", Math.min(index, 12));
  const node = el("button", program.uninstalled ? "card dim" : "card");
  const cover = el("div", "cover", program.name.charAt(0).toUpperCase());
  cover.style.cssText = coverStyle(program.name);
  const image = coverImage(program, program.cover_kind === "icon" ? "cover-icon" : "cover-img");
  if (image) cover.append(image);
  const meta = el("div", "meta", program.name);
  meta.append(el("small", "", program.uninstalled ? t("card.not_installed", { source: sourceName(program.source) }) : sourceName(program.source)));
  node.append(cover, meta);
  if (look.view === "list" && !program.uninstalled) {
    const last = program.launches.length > 0 ? Math.max(...program.launches) : null;
    const info = [program.play_ms + program.play_est_ms > 0 ? formatDuration(program.play_ms + program.play_est_ms) : null, last ? t("card.last_launch", { date: new Date(last).toLocaleDateString(locale(), { day: "numeric", month: "short" }) }) : t("card.never_launched")];
    node.append(el("span", "row-info", info.filter(Boolean).join(" · ")));
  }
  node.addEventListener("click", () => launch(program));
  const tools = el("div", "card-tools");
  const infoButton = button("card-edit", t("common.details"), () => openDetailsPage(program));
  infoButton.title = t("card.view_page", { name: program.name });
  infoButton.setAttribute("aria-label", infoButton.title);
  const edit = button("card-edit", t("common.image"), () => openCoverPanel(program));
  edit.title = t("card.change_image", { name: program.name });
  edit.setAttribute("aria-label", edit.title);
  tools.append(infoButton, edit);
  if (!program.uninstalled) {
    const box = button("card-edit", t("common.collections"), () => openProgramCollections(program.id));
    box.title = t("card.sort_into", { name: program.name });
    box.setAttribute("aria-label", box.title);
    tools.append(box);
  }
  wrap.append(node, tools);
  if (!program.uninstalled) {
    const star = button(program.favorite ? "card-star on" : "card-star", program.favorite ? "★" : "☆", () => toggleFavorite(program));
    star.title = t(program.favorite ? "card.unfavorite" : "card.favorite", { name: program.name });
    star.setAttribute("aria-label", star.title);
    star.setAttribute("aria-pressed", String(Boolean(program.favorite)));
    wrap.append(star);
  }
  return wrap;
}

function hero(program) {
  const color = COLORS[hash(program.name) % COLORS.length];
  const section = el("section", "hero");
  section.style.setProperty("--glow", color);
  const art = coverImage(program, program.cover_kind === "icon" ? "hero-icon" : "hero-art");
  if (art) section.append(art);
  const count = program.launches.length;
  const stats = count === 0 ? t("hero.never") : tn("hero.count", count);
  const inner = el("div", "hero-in");
  const actions = el("div", "actions");
  actions.append(
    button("btn primary", t("common.launch"), () => launch(program)),
    button("btn ghost", t("common.details"), () => openDetailsPage(program)),
    button("btn ghost", t("common.image"), () => openCoverPanel(program)),
  );
  inner.append(
    el("span", "tag", t("hero.tag")),
    el("h1", "", program.name),
    el("p", "", `${stats} · ${sourceName(program.source)}`),
    actions,
  );
  section.append(inner);
  return section;
}

// Sélecteur de pochettes SteamGridDB : recherche d'un jeu, grille de pochettes verticales, un clic pour choisir.
function openSteamGridPicker(program) {
  overlay.replaceChildren();
  const panel = el("div", "panel wide");
  panel.setAttribute("role", "dialog");
  panel.setAttribute("aria-label", t("sgdb.title"));
  panel.append(el("h2", "", t("sgdb.title")));
  overlay.append(panel);
  overlay.hidden = false;

  if (!keySet) {
    panel.append(
      el("p", "", t("sgdb.need_key")),
      button("btn primary", t("sgdb.open_settings"), openSettings),
      button("btn ghost", t("common.close"), closeSettings),
    );
    return;
  }

  const form = el("form", "sgdb-search");
  const input = el("input", "text-input");
  input.type = "text";
  input.value = program.name;
  input.setAttribute("aria-label", t("sgdb.name_label"));
  const submit = el("button", "btn primary", t("sgdb.search"));
  submit.type = "submit";
  form.append(input, submit);
  const games = el("div", "sgdb-games");
  const status = el("p", "sgdb-status", "");
  const grid = el("div", "sgdb-grid");
  panel.append(
    el("p", "", t("sgdb.intro")),
    form,
    games,
    status,
    grid,
    button("btn ghost", t("common.close"), closeSettings),
  );

  const pick = async (cover) => {
    status.textContent = t("sgdb.downloading");
    try {
      programs = await invoke("sgdb_pick", { id: program.id, url: cover.url, tz });
      closeSettings();
      renderLibrary();
      toast(t("sgdb.saved"));
    } catch (error) {
      status.textContent = String(error);
    }
  };

  const showGrids = async (game, chip) => {
    for (const other of games.children) other.setAttribute("aria-pressed", "false");
    chip.setAttribute("aria-pressed", "true");
    grid.replaceChildren();
    status.textContent = t("sgdb.loading");
    try {
      const list = await invoke("sgdb_grids", { gameId: game.id });
      if (list.length === 0) {
        status.textContent = t("sgdb.none_vertical");
        return;
      }
      status.textContent = tn("sgdb.pick", list.length);
      for (const cover of list) {
        const img = el("img", "sgdb-cover");
        img.alt = t("common.photo_alt");
        img.loading = "lazy";
        img.src = cover.thumb;
        img.title = [cover.style, cover.author].filter(Boolean).join(" · ");
        img.addEventListener("click", () => pick(cover));
        grid.append(img);
      }
    } catch (error) {
      status.textContent = String(error);
    }
  };

  const search = async () => {
    const query = input.value.trim();
    games.replaceChildren();
    grid.replaceChildren();
    if (!query) {
      status.textContent = t("sgdb.type_name");
      return;
    }
    status.textContent = t("sgdb.searching");
    try {
      const list = await invoke("sgdb_search", { query });
      if (list.length === 0) {
        status.textContent = t("sgdb.no_game");
        return;
      }
      const chips = list.map((game) => {
        const chip = button("sgdb-game", game.year ? `${game.name} (${game.year})` : game.name, () => showGrids(game, chip));
        chip.setAttribute("aria-pressed", "false");
        return chip;
      });
      games.append(...chips);
      showGrids(list[0], chips[0]);
    } catch (error) {
      status.textContent = String(error);
    }
  };
  form.addEventListener("submit", (event) => {
    event.preventDefault();
    search();
  });
  search();
}

// Panneau pour changer l'image d'un programme.
function openCoverPanel(program) {
  overlay.replaceChildren();
  const panel = el("div", "panel");
  panel.setAttribute("role", "dialog");
  panel.setAttribute("aria-label", t("cover.aria", { name: program.name }));
  panel.append(
    el("h2", "", program.name),
    el("p", "", t("cover.intro")),
  );
  const choose = button("btn primary", t("cover.choose"), async () => {
    try {
      const path = await open({
        multiple: false,
        filters: [{ name: t("cover.images_filter"), extensions: ["jpg", "jpeg", "png", "webp"] }],
      });
      if (!path) return;
      programs = await invoke("set_custom_cover", { id: program.id, path, tz });
      closeSettings();
      renderLibrary();
    } catch (error) {
      toast(String(error), true);
    }
  });
  panel.append(choose, button("btn ghost", t("cover.search_sgdb"), () => openSteamGridPicker(program)));
  if (program.cover_kind === "custom") {
    panel.append(
      button("btn ghost", t("cover.remove"), async () => {
        try {
          programs = await invoke("clear_custom_cover", { id: program.id, tz });
          closeSettings();
          renderLibrary();
        } catch (error) {
          toast(String(error), true);
        }
      }),
    );
  }
  panel.append(button("btn ghost", t("common.close"), closeSettings));
  overlay.append(panel);
  overlay.hidden = false;
  choose.focus();
}

function renderLibrary() {
  updateBackdrop();
  // Tant que l'écran des statistiques est ouvert, un changement de bibliothèque le rafraîchit.
  if (screen === "stats") {
    renderStats();
    return;
  }
  // Sur une page de jeu (par exemple après « Lancer »), on la rafraîchit au lieu de revenir à la bibliothèque.
  if (screen === "details") {
    if (!editing) renderDetails(false);
    return;
  }
  view.replaceChildren();
  const installed = programs.filter((p) => !p.uninstalled);
  const uninstalled = programs.filter((p) => p.uninstalled);
  if (installed.length === 0) {
    const buttons = el("div", "actions");
    buttons.append(
      button("btn primary big", t("library.add"), addProgram),
      button("btn ghost big", t("library.import"), importGames),
    );
    const empty = el("div", "empty");
    empty.append(
      el("h1", "", t("library.empty_title")),
      el("p", "", t("library.empty_text")),
      buttons,
    );
    view.append(empty);
    if (uninstalled.length > 0) view.append(uninstalledSection(uninstalled));
    return;
  }
  // Un filtre qui ne correspond plus à aucun launcher (désactivé depuis) est abandonné.
  if (sourceFilter && !programs.some((p) => p.source === sourceFilter)) sourceFilter = "";
  if (collectionFilter && !collections.some((c) => c.id === collectionFilter)) collectionFilter = "";
  resultsEl = el("div", "results");
  const showBar = programs.length >= 4 || query || sourceFilter || collectionFilter || collections.length > 0;
  view.append(...(showBar ? [libraryToolbar()] : []), resultsEl);
  renderResults();
}

// Barre de recherche, filtre par launcher et tri. Les saisies ne refont que les résultats : le champ garde le focus.
function libraryToolbar() {
  const bar = el("div", "toolbar");
  const search = el("input", "text-input toolbar-search");
  search.type = "search";
  search.placeholder = t("library.search_placeholder");
  search.setAttribute("aria-label", t("library.search_label"));
  search.autocomplete = "off";
  search.spellcheck = false;
  search.value = query;
  search.addEventListener("input", () => {
    query = search.value;
    renderResults();
  });

  const choose = (label, options, current, onChange) => {
    const select = el("select", "toolbar-select");
    select.setAttribute("aria-label", label);
    for (const [value, text] of options) {
      const option = el("option", "", text);
      option.value = value;
      select.append(option);
    }
    select.value = current;
    select.addEventListener("change", () => onChange(select.value));
    return select;
  };
  const sources = [...new Set(programs.map((p) => p.source))].sort((a, b) => sourceName(a).localeCompare(sourceName(b), locale()));
  bar.append(
    search,
    choose(t("library.filter_source"), [["", t("library.all_sources")], ...sources.map((s) => [s, sourceName(s)])], sourceFilter, (value) => {
      sourceFilter = value;
      renderResults();
    }),
    choose(t("library.sort_label"), sortOptions(), sortMode, (value) => {
      sortMode = value;
      renderResults();
      invoke("set_sort", { sort: value }).catch((error) => toast(String(error), true));
    }),
  );
  if (collections.length > 0) {
    bar.append(
      choose(t("library.filter_collection"), [["", t("library.all_collections")], ...collections.map((c) => [c.id, c.name])], collectionFilter, (value) => {
        collectionFilter = value;
        renderResults();
      }),
    );
  }
  bar.append(button("btn ghost toolbar-button", t("common.collections"), openCollectionsManager));
  return bar;
}

// Suggestion, favoris, bibliothèque et jeux non installés, selon la recherche, le filtre et le tri.
function renderResults() {
  const q = normalize(query.trim());
  const filtering = q !== "" || sourceFilter !== "" || collectionFilter !== "";
  const matches = (p) =>
    (!sourceFilter || p.source === sourceFilter) &&
    (!collectionFilter || p.collections?.includes(collectionFilter)) &&
    (!q || normalize(p.name).includes(q));
  const installed = programs.filter((p) => !p.uninstalled && matches(p));
  const uninstalled = programs.filter((p) => p.uninstalled && matches(p));
  const parts = [];

  // La suggestion reste le jeu le plus probable ; elle disparaît pendant une recherche ou un filtre.
  const top = programs.find((p) => !p.uninstalled);
  const showHero = look.show_hero && !filtering && top;
  if (showHero) parts.push(hero(top));
  const others = showHero ? installed.filter((p) => p.id !== top.id) : installed;
  const grid = (list) => {
    const node = el("div", look.view === "list" ? "grid list" : "grid");
    node.append(...list.map(card));
    return node;
  };
  const favorites = sorted(others.filter((p) => p.favorite));
  const regular = sorted(others.filter((p) => !p.favorite));
  if (favorites.length > 0) parts.push(el("div", "section-title", t("library.favorites")), grid(favorites));
  if (regular.length > 0) parts.push(el("div", "section-title", t("library.title")), grid(regular));
  if (filtering && installed.length === 0 && uninstalled.length === 0) parts.push(el("div", "no-match", t("library.no_match")));
  if (uninstalled.length > 0) parts.push(uninstalledSection(uninstalled, filtering));
  resultsEl.replaceChildren(...parts);
}

// Applique un nouvel état reçu du cœur de l'application et rafraîchit l'écran.
function useState(state) {
  applyState(state);
  renderLibrary();
}

// Fenêtre « Collections » : créer, renommer, supprimer.
function openCollectionsManager() {
  overlay.replaceChildren();
  const panel = el("div", "panel");
  panel.setAttribute("role", "dialog");
  panel.setAttribute("aria-label", t("coll.title"));
  overlay.append(panel);
  overlay.hidden = false;
  let armed = null; // collection dont la suppression attend sa confirmation
  const run = async (action) => {
    try {
      useState(await action());
    } catch (error) {
      toast(String(error), true);
    }
    draw();
  };
  const draw = () => {
    panel.replaceChildren(el("h2", "", t("coll.title")), el("p", "", t("coll.intro")));
    if (collections.length === 0) panel.append(el("p", "no-match", t("coll.none")));
    for (const collection of collections) {
      const count = programs.filter((p) => p.collections?.includes(collection.id)).length;
      const row = el("div", "setting-row coll-row");
      const name = el("input", "text-input");
      name.type = "text";
      name.maxLength = 40;
      name.value = collection.name;
      name.setAttribute("aria-label", t("coll.name_aria", { name: collection.name }));
      name.addEventListener("change", () => run(() => invoke("rename_collection", { id: collection.id, name: name.value, tz })));
      const remove = button(armed === collection.id ? "btn ghost danger" : "btn ghost", armed === collection.id ? t("common.confirm_delete") : t("common.delete"), () => {
        if (armed !== collection.id) {
          armed = collection.id;
          draw();
          return;
        }
        armed = null;
        if (collectionFilter === collection.id) collectionFilter = "";
        run(() => invoke("delete_collection", { id: collection.id, tz }));
      });
      row.append(name, el("small", "coll-count", tn("coll.games", count)), remove);
      panel.append(row);
    }
    panel.append(collectionForm(t("coll.new"), t("common.create"), (name) => run(() => invoke("create_collection", { name, tz }))));
    panel.append(button("btn ghost", t("common.close"), closeSettings));
  };
  draw();
}

// Champ de nom + bouton, validés par Entrée ou par le bouton.
function collectionForm(placeholder, label, onSubmit) {
  const form = el("form", "coll-form");
  const input = el("input", "text-input");
  input.type = "text";
  input.maxLength = 40;
  input.placeholder = placeholder;
  input.autocomplete = "off";
  input.setAttribute("aria-label", placeholder);
  const submit = el("button", "btn primary", label);
  submit.type = "submit";
  form.append(input, submit);
  form.addEventListener("submit", (event) => {
    event.preventDefault();
    const name = input.value;
    input.value = "";
    onSubmit(name);
  });
  return form;
}

// Fenêtre « Collections d'un jeu » : une case par collection, et création d'une nouvelle.
function openProgramCollections(programId) {
  overlay.replaceChildren();
  const panel = el("div", "panel");
  panel.setAttribute("role", "dialog");
  overlay.append(panel);
  overlay.hidden = false;
  const draw = () => {
    const program = programs.find((p) => p.id === programId);
    if (!program) return closeSettings();
    panel.setAttribute("aria-label", t("coll.of_game", { name: program.name }));
    panel.replaceChildren(el("h2", "", t("coll.of_game", { name: program.name })));
    if (collections.length === 0) panel.append(el("p", "", t("coll.create_first")));
    const list = el("div", "switches");
    for (const collection of collections) {
      const member = Boolean(program.collections?.includes(collection.id));
      const row = el("button", "switch");
      row.setAttribute("role", "switch");
      row.setAttribute("aria-checked", String(member));
      row.append(el("span", "", collection.name), el("i", "knob"));
      row.addEventListener("click", async () => {
        try {
          useState(await invoke("set_program_collection", { id: programId, collection: collection.id, member: !member, tz }));
        } catch (error) {
          toast(String(error), true);
        }
        draw();
      });
      list.append(row);
    }
    panel.append(list);
    panel.append(
      collectionForm(t("coll.new"), t("coll.create_add"), async (name) => {
        try {
          const state = await invoke("create_collection", { name, tz });
          const created = state.collections.at(-1);
          useState(await invoke("set_program_collection", { id: programId, collection: created.id, member: true, tz }));
        } catch (error) {
          toast(String(error), true);
        }
        draw();
      }),
      button("btn ghost", t("coll.manage"), openCollectionsManager),
      button("btn ghost", t("common.close"), closeSettings),
    );
  };
  draw();
}

async function toggleFavorite(program) {
  try {
    programs = await invoke("set_favorite", { id: program.id, favorite: !program.favorite, tz });
    renderLibrary();
  } catch (error) {
    toast(String(error), true);
  }
}

// Jeux Steam possédés mais pas installés : repliés d'office, pochettes grisées.
function uninstalledSection(list, filtering = false) {
  const section = el("section", "uninstalled");
  // Pendant une recherche, les résultats non installés sont montrés d'office.
  const open = uninstalledOpen || filtering;
  const toggle = el("button", "section-toggle", tn("library.uninstalled", list.length));
  toggle.setAttribute("aria-expanded", String(open));
  toggle.addEventListener("click", () => {
    uninstalledOpen = !uninstalledOpen;
    try {
      localStorage.setItem("uninstalledOpen", uninstalledOpen ? "1" : "0");
    } catch {}
    renderLibrary();
  });
  section.append(toggle);
  if (open) {
    const grid = el("div", look.view === "list" ? "grid list" : "grid");
    grid.append(...list.map(card));
    section.append(grid);
  }
  return section;
}

// ------------------------------------------------------------ Statistiques

function formatDuration(ms) {
  const minutes = Math.round(ms / 60000);
  if (minutes < 1) return ms > 0 ? t("time.under_min") : t("time.zero");
  if (minutes < 60) return t("time.min", { m: minutes });
  return t("time.hm", { h: Math.floor(minutes / 60), mm: String(minutes % 60).padStart(2, "0") });
}

function statTile(value, label, sub) {
  const tile = el("div", "stat-tile");
  tile.append(el("div", "stat-value", value), el("div", "stat-label", label));
  if (sub) tile.append(el("div", "stat-sub", sub));
  return tile;
}

function playTime(program) {
  const total = program.play_ms + program.play_est_ms;
  if (total === 0) return null;
  const node = el("span", "play-time", formatDuration(total));
  if (program.play_est_ms > 0) node.append(el("em", "estimated", t("play.estimated")));
  return node;
}

async function renderStats() {
  screen = "stats";
  const stats = await invoke("get_stats", { tz });
  if (screen !== "stats") return; // fermé pendant le chargement
  view.replaceChildren();

  const head = el("div", "stats-head");
  head.append(button("btn ghost", t("common.back"), closeStats), el("h1", "", t("stats.title")));
  const wrap = el("div", "stats");
  wrap.append(head);

  if (stats.total_launches === 0 && stats.total_play_ms + stats.total_play_est_ms === 0) {
    const empty = el("div", "stats-empty");
    empty.append(
      el("h2", "", t("stats.empty_title")),
      el("p", "", t("stats.empty_text")),
    );
    wrap.append(empty);
    view.append(wrap);
    return;
  }

  const totalPlay = stats.total_play_ms + stats.total_play_est_ms;
  const tiles = el("div", "stat-tiles");
  tiles.append(
    statTile(String(stats.total_launches), tn("stats.launches", stats.total_launches), tn("stats.played", stats.programs_played)),
    statTile(
      String(stats.current_streak),
      tn("stats.streak", stats.current_streak),
      tn("stats.best", stats.best_streak),
    ),
    statTile(
      totalPlay > 0 ? formatDuration(totalPlay) : "—",
      t("stats.play_time"),
      stats.total_play_est_ms > 0 ? t("stats.of_which_estimated", { time: formatDuration(stats.total_play_est_ms) }) : null,
    ),
  );
  wrap.append(tiles);

  // Bilan des 7 derniers jours
  const week = el("section", "week");
  week.append(el("h3", "", t("stats.week")));
  const lines = el("ul", "week-lines");
  const line = (label, value) => {
    const item = el("li");
    item.append(el("span", "", label), el("strong", "", value));
    lines.append(item);
  };
  line(t("stats.week_launches"), String(stats.week.launches));
  line(t("stats.week_top"), stats.week.top ? tn("stats.week_top_value", stats.week.top.count, { name: stats.week.top.name }) : "—");
  line(t("stats.week_busiest"), stats.week.busiest_day ? tn("stats.week_busiest_value", stats.week.busiest_day.count, { day: t(`weekday.${stats.week.busiest_day.label}`) }) : "—");
  line(t("stats.week_new"), stats.week.new_played.length > 0 ? stats.week.new_played.join(", ") : t("stats.none"));
  week.append(lines);
  wrap.append(week);

  // Classement des programmes
  const board = el("section", "board");
  board.append(el("h3", "", t("stats.board")));
  const most = Math.max(...stats.top.map((p) => p.launches), 1);
  stats.top.forEach((program, index) => {
    const row = el("div", "top-row");
    const meter = el("div", "meter");
    meter.style.setProperty("--w", `${Math.round((program.launches / most) * 100)}%`);
    const name = el("div", "top-name", program.name);
    name.append(el("small", "", sourceName(program.source)));
    row.append(el("span", "top-rank", String(index + 1)), name, meter, el("span", "top-count", tn("stats.launches", program.launches)));
    const time = playTime(program);
    row.append(time ?? el("span", "play-time muted", "—"));
    board.append(row);
  });
  wrap.append(board);
  view.append(wrap);
}

function closeStats() {
  screen = "library";
  renderLibrary();
}

function toggleStats() {
  if (screen === "stats") closeStats();
  else renderStats();
}
// ------------------------------------------------------- Pages des jeux et logiciels

let detailsId = null;
let descriptionOpen = false;

const providerName = (provider) => (provider === "wikipedia" ? t("provider.wikipedia") : ({ steam: "Steam", rawg: "RAWG" })[provider]);
const DESCRIPTION_PREVIEW = 700;

function formatDate(ms) {
  return new Date(ms).toLocaleDateString(locale(), { day: "numeric", month: "long", year: "numeric" });
}

function assetUrl(path, version) {
  return `${window.__TAURI__.core.convertFileSrc(path)}?v=${version}`;
}

function infoRow(label, value, extra) {
  const row = el("div", "d-row");
  row.append(el("dt", "", label));
  const dd = el("dd", "", value);
  if (extra) dd.append(extra);
  row.append(dd);
  return row;
}

function banner(title, text, actionLabel, onAction) {
  const box = el("div", "d-banner");
  box.append(el("strong", "", title), el("p", "", text));
  if (actionLabel) box.append(button("btn primary", actionLabel, onAction));
  return box;
}

function paragraphs(text, className) {
  const wrap = el("div", className);
  for (const block of text.split(/\n{2,}/)) {
    const p = el("p");
    block.split("\n").forEach((line, index) => {
      if (index > 0) p.append(document.createElement("br"));
      p.append(line);
    });
    wrap.append(p);
  }
  return wrap;
}

function openShot(shot) {
  overlay.replaceChildren();
  const img = el("img", "lightbox");
  img.alt = "";
  img.src = assetUrl(shot.path, shot.ver);
  img.addEventListener("click", closeSettings);
  overlay.append(img);
  overlay.hidden = false;
}

function ratingBlock(rating) {
  const box = el("div", "d-rating");
  const count = rating.count.toLocaleString(locale());
  const text =
    rating.percent != null
      ? t("rating.reviews", { percent: rating.percent, count })
      : tn("rating.recs", rating.count, { count });
  box.append(el("div", "d-rating-label", rating.label), el("div", "d-rating-value", text));
  if (rating.percent != null) {
    const meter = el("div", "meter");
    meter.style.setProperty("--w", `${rating.percent}%`);
    box.append(meter);
  }
  return box;
}

function newsBlock(items) {
  const section = el("section", "d-section");
  section.append(el("h3", "", t("details.news")));
  for (const item of items) {
    const entry = el("article", "d-news");
    const link = button("d-news-title", item.title, () => invoke("open_url", { url: item.url }).catch((e) => toast(String(e), true)));
    const meta = [item.date ? formatDate(item.date * 1000) : null, item.label].filter(Boolean).join(" · ");
    entry.append(link, el("div", "d-news-meta", meta));
    if (item.summary) entry.append(el("p", "", item.summary));
    section.append(entry);
  }
  return section;
}

function usageBlock(view) {
  const box = el("dl", "d-facts");
  const local = view.local;
  box.append(infoRow(t("details.local_launches"), String(local.launches)));
  box.append(infoRow(t("details.local_last"), local.last_launch ? formatDate(local.last_launch) : t("details.local_never")));
  const total = local.play_ms + local.play_est_ms;
  box.append(
    infoRow(t("details.local_play"), total > 0 ? formatDuration(total) : "—", local.play_est_ms > 0 ? el("em", "estimated", t("play.estimated")) : null),
  );
  box.append(infoRow(t("details.local_added"), formatDate(local.added)));
  if (view.source === "manual") box.append(infoRow(t("details.local_location"), local.target));
  return box;
}

function factsBlock(page) {
  const box = el("dl", "d-facts");
  const add = (label, value) => {
    if (value && value.length) box.append(infoRow(label, Array.isArray(value) ? value.join(", ") : value));
  };
  add(t("details.fact_developer"), page.developers);
  add(t("details.fact_publisher"), page.publishers);
  add(t("details.fact_release"), page.release_date);
  add(t("details.fact_platforms"), page.platforms);
  add(t("details.fact_genres"), page.genres);
  return box;
}

let editing = false;
let descriptionInput = null;

// Enregistre le titre et la description du joueur (texte vide = valeur d'origine).
async function saveProgramInfo(id, name, description) {
  try {
    programs = await invoke("set_program_info", { id, name, description, tz });
    editing = false;
    renderDetails(false);
  } catch (error) {
    toast(String(error), true);
  }
}

// Description du programme : la sienne si le joueur en a écrit une, sinon celle de la source.
function descriptionSection(info, page) {
  const online = (info.state === "ready" ? page?.description : "") ?? "";
  const custom = info.custom_description;
  const text = custom ?? online;
  if (editing) {
    const section = el("section", "d-section");
    section.append(el("h3", "", t("details.description")));
    descriptionInput = el("textarea", "d-textarea");
    descriptionInput.value = text;
    descriptionInput.maxLength = 5000;
    descriptionInput.placeholder = t("details.description_placeholder");
    descriptionInput.setAttribute("aria-label", t("details.description"));
    section.append(
      descriptionInput,
      el("p", "d-help", t(online ? "details.description_help_online" : "details.description_help_none")),
    );
    return section;
  }
  if (!text) return null;
  const long = text.length > DESCRIPTION_PREVIEW;
  const shown = long && !descriptionOpen ? `${text.slice(0, DESCRIPTION_PREVIEW).trimEnd()}…` : text;
  const section = el("section", "d-section");
  const heading = el("h3", "", t("details.description"));
  if (custom) heading.append(el("em", "estimated", t("details.edited")));
  section.append(heading, paragraphs(shown, "d-text"));
  if (long) {
    section.append(
      button("btn ghost", descriptionOpen ? t("details.collapse") : t("details.read_more"), () => {
        descriptionOpen = !descriptionOpen;
        renderDetails(false);
      }),
    );
  }
  return section;
}

async function renderDetails(first = true) {
  const info = await invoke(first ? "open_details" : "get_details", { id: detailsId });
  if (screen !== "details" || info.id !== detailsId) return;
  const program = programs.find((p) => p.id === info.id);
  const page = info.details;
  const root = el("div", "details");

  // En-tête : retour, titre, actions
  const head = el("section", "d-head");
  head.append(button("btn ghost", t("common.back"), closeDetails));
  const title = el("div", "d-title");
  const art = program ? coverImage(program, program.cover_kind === "icon" ? "d-icon" : "d-poster") : null;
  if (art) title.append(art);
  const text = el("div", "d-title-text");
  let nameInput = null;
  if (editing) {
    nameInput = el("input", "d-title-input");
    nameInput.type = "text";
    nameInput.maxLength = 80;
    nameInput.value = info.name;
    nameInput.placeholder = info.original_name;
    nameInput.setAttribute("aria-label", t("details.title_label"));
    text.append(nameInput);
  } else {
    text.append(el("h1", "", info.name));
  }
  const tags = el("div", "d-tags");
  const source = sourceName(info.source);
  tags.append(el("span", "tag", source));
  if (program?.uninstalled) tags.append(el("span", "tag muted-tag", t("details.not_installed")));
  if (page) {
    for (const genre of page.genres.slice(0, 4)) tags.append(el("span", "tag muted-tag", genre));
    if (page.lang !== currentLang() && page.description) tags.append(el("span", "tag muted-tag", t("details.text_in", { lang: t(`lang.${page.lang}`) })));
  }
  text.append(tags);
  const actions = el("div", "actions");
  const onlineText = (info.state === "ready" ? page?.description : "") ?? "";
  if (editing) {
    actions.append(
      button("btn primary", t("common.save"), () => {
        // Un texte identique à celui de la source n'est pas une modification : on garde la source.
        const typed = descriptionInput.value;
        saveProgramInfo(info.id, nameInput.value, typed.trim() === onlineText.trim() ? "" : typed);
      }),
      button("btn ghost", t("common.cancel"), () => {
        editing = false;
        renderDetails(false);
      }),
    );
    if (info.name !== info.original_name || info.custom_description) {
      actions.append(button("btn ghost", t("details.restore"), () => saveProgramInfo(info.id, "", "")));
    }
  } else {
    if (program?.uninstalled) actions.append(button("btn primary", t("common.install"), () => installProgram(program)));
    else if (program) actions.append(button("btn primary", t("common.launch"), () => launch(program)));
    actions.append(
      button("btn ghost", t("common.edit"), () => {
        editing = true;
        renderDetails(false);
      }),
    );
    if (program) actions.append(button("btn ghost", t("common.image"), () => openCoverPanel(program)));
    if (program && !program.uninstalled) {
      actions.append(
        button("btn ghost", program.favorite ? t("details.favorite_on") : t("details.favorite_off"), () => toggleFavorite(program)),
        button("btn ghost", t("common.collections"), () => openProgramCollections(program.id)),
      );
    }
    if (page?.page_url) {
      actions.append(button("btn ghost", t("details.view_on", { provider: providerName(page.provider) ?? t("details.provider_fallback") }), () => invoke("open_url", { url: page.page_url }).catch((e) => toast(String(e), true))));
    }
  }
  text.append(actions);
  title.append(text);
  head.append(title);
  root.append(head);

  const columns = el("div", "d-columns");
  const main = el("div", "d-main");
  const side = el("aside", "d-side");

  // États sans contenu en ligne
  if (info.state === "off") {
    main.append(
      banner(
        t("details.off_title"),
        t("details.off_text"),
        t("details.off_action"),
        async () => {
          try {
            await invoke("set_online_info", { enabled: true, key: null });
            onlineInfo = true;
            renderDetails(true);
          } catch (error) {
            toast(String(error), true);
          }
        },
      ),
    );
  } else if (info.state === "needs_key") {
    main.append(
      banner(
        t("details.key_title"),
        t("details.key_text"),
        t("sgdb.open_settings"),
        openSettings,
      ),
    );
  } else if (info.state === "none") {
    main.append(banner(t("details.none_title"), t("details.none_text"), info.can_correct ? t("details.none_action") : null, () => openCandidates(info)));
  } else if (info.state === "pending") {
    main.append(el("div", "d-loading", info.error ? t("details.network", { error: info.error }) : t("details.loading")));
    if (info.error) main.append(button("btn ghost", t("common.retry"), () => renderDetails(true)));
  }

  // Résumé trouvé en ligne, puis description (celle du joueur si elle existe)
  if (page && info.state === "ready" && page.summary) main.append(el("p", "d-lead", page.summary));
  const description = descriptionSection(info, page);
  if (description) main.append(description);

  // Contenu en ligne
  if (page && info.state === "ready") {
    if (info.shots.length > 0) {
      const section = el("section", "d-section");
      section.append(el("h3", "", t("details.shots")));
      const strip = el("div", "d-shots");
      for (const shot of info.shots) {
        const img = el("img", "d-shot");
        img.alt = t("details.shot_alt");
        img.src = assetUrl(shot.path, shot.ver);
        img.addEventListener("click", () => openShot(shot));
        strip.append(img);
      }
      section.append(strip);
      main.append(section);
    } else if (info.loading && page.provider !== "wikipedia") {
      main.append(el("div", "d-loading", t("details.shots_loading")));
    }
    if (page.news.length > 0) main.append(newsBlock(page.news));

    // Fiche technique et notes
    const facts = factsBlock(page);
    if (facts.children.length > 0 || page.metacritic != null || page.rating) {
      const card = el("section", "d-card");
      card.append(el("h3", "", t("details.specs")), facts);
      if (page.metacritic != null) {
        const score = el("div", "d-metacritic");
        score.append(el("span", "d-score", String(page.metacritic)), el("span", "", t("details.metacritic")));
        card.append(score);
      }
      if (page.rating) card.append(ratingBlock(page.rating));
      if (page.website) {
        card.append(button("btn ghost", t("details.website"), () => invoke("open_url", { url: page.website }).catch((e) => toast(String(e), true))));
      }
      side.append(card);
    }
  }

  // Usage local : toujours affiché
  const usage = el("section", "d-card");
  usage.append(el("h3", "", t("details.usage")), usageBlock(info));
  side.append(usage);

  columns.append(main, side);
  root.append(columns);

  // Pied : source et correction
  if (page && info.state === "ready") {
    const foot = el("div", "d-foot");
    foot.append(el("span", "", t("details.found", { name: page.match_name, provider: providerName(page.provider) ?? page.provider })));
    if (info.can_correct) foot.append(button("btn ghost", t("details.wrong"), () => openCandidates(info)));
    root.append(foot);
  }
  if (info.loading && info.state === "ready") root.append(el("div", "d-updating", t("details.updating")));

  view.replaceChildren(root);
}

async function openCandidates(detailsView) {
  overlay.replaceChildren();
  const panel = el("div", "panel");
  panel.setAttribute("role", "dialog");
  panel.append(el("h2", "", t("pick.title")), el("p", "", t("pick.searching")));
  panel.append(button("btn ghost", t("common.close"), closeSettings));
  overlay.append(panel);
  overlay.hidden = false;
  try {
    const list = await invoke("details_candidates", { id: detailsView.id });
    panel.replaceChildren(el("h2", "", t("pick.title")));
    if (list.length === 0) {
      panel.append(el("p", "", t(detailsView.rawg_key_set ? "pick.none_other" : "pick.none_nokey")));
    }
    const choose = async (provider, matchId) => {
      try {
        await invoke("choose_details", { id: detailsView.id, provider, matchId });
        closeSettings();
        renderDetails(false);
      } catch (error) {
        toast(String(error), true);
      }
    };
    const choices = el("div", "candidates");
    for (const candidate of list) {
      const item = el("button", "candidate");
      item.append(el("strong", "", candidate.name), el("small", "", [providerName(candidate.provider), candidate.hint].filter(Boolean).join(" · ")));
      item.addEventListener("click", () => choose(candidate.provider, candidate.id));
      choices.append(item);
    }
    panel.append(choices);
    panel.append(button("btn ghost", t("pick.none_of_these"), () => choose("none", "")));
    panel.append(button("btn ghost", t("pick.auto"), () => choose("auto", "")));
    panel.append(button("btn ghost", t("common.close"), closeSettings));
  } catch (error) {
    panel.replaceChildren(el("h2", "", t("pick.failed")), el("p", "", String(error)), button("btn ghost", t("common.close"), closeSettings));
  }
}

function openDetailsPage(program) {
  detailsId = program.id;
  descriptionOpen = false;
  editing = false;
  screen = "details";
  renderDetails(true).catch((error) => toast(String(error), true));
}

function closeDetails() {
  editing = false;
  screen = "library";
  detailsId = null;
  renderLibrary();
}

// ----------------------------------------------------- Premier lancement

function renderOnboarding() {
  document.body.classList.add("onboarding");
  view.replaceChildren();
  const chosen = new Set(platforms.filter((p) => p.enabled).map((p) => p.id));

  const tiles = el("div", "tiles");
  let wantsCovers = false;
  let wantsOnline = false;
  const next = button("btn primary big", "", () => finishOnboarding([...chosen], wantsCovers, wantsOnline));
  const refresh = () => {
    next.textContent = chosen.size > 0 ? t("onboard.continue") : t("onboard.continue_none");
  };
  for (const platform of platforms) {
    const tile = el("button", "tile");
    tile.setAttribute("aria-pressed", String(chosen.has(platform.id)));
    tile.append(el("span", "tile-name", platform.label));
    if (platform.detected) tile.append(el("span", "tag", t("onboard.detected")));
    tile.addEventListener("click", () => {
      chosen.has(platform.id) ? chosen.delete(platform.id) : chosen.add(platform.id);
      tile.setAttribute("aria-pressed", String(chosen.has(platform.id)));
      refresh();
    });
    tiles.append(tile);
  }
  refresh();

  // Le téléchargement envoie des noms de jeux sur Internet : il est proposé, jamais activé d'office.
  const covers = el("button", "tile tile-wide");
  covers.setAttribute("aria-pressed", "false");
  const coversText = el("span", "tile-name", t("onboard.covers_title"));
  coversText.append(el("small", "", t("onboard.covers_text")));
  covers.append(coversText);
  covers.addEventListener("click", () => {
    wantsCovers = !wantsCovers;
    covers.setAttribute("aria-pressed", String(wantsCovers));
  });

  const online = el("button", "tile tile-wide");
  online.setAttribute("aria-pressed", "false");
  const onlineText = el("span", "tile-name", t("onboard.online_title"));
  onlineText.append(el("small", "", t("onboard.online_text")));
  online.append(onlineText);
  online.addEventListener("click", () => {
    wantsOnline = !wantsOnline;
    online.setAttribute("aria-pressed", String(wantsOnline));
  });

  const wrap = el("div", "onboard");
  wrap.append(
    el("span", "tag", t("onboard.welcome")),
    el("h1", "", t("onboard.title")),
    el("p", "", t("onboard.text")),
    tiles,
    online,
    covers,
    next,
  );
  view.append(wrap);
}

async function finishOnboarding(enabled, wantsCovers, wantsOnline) {
  try {
    await invoke("set_download_covers", { enabled: wantsCovers, key: null });
    await invoke("set_online_info", { enabled: wantsOnline, key: null });
    applyState(await invoke("set_platforms", { enabled, tz }));
    document.body.classList.remove("onboarding");
    renderLibrary();
    if (enabled.length > 0) await importGames();
  } catch (error) {
    toast(String(error), true);
  }
}

// -------------------------------------------------------------- Paramètres

function openSettings() {
  overlay.replaceChildren();
  const panel = el("div", "panel");
  panel.setAttribute("role", "dialog");
  panel.setAttribute("aria-label", t("settings.title"));
  panel.append(el("h2", "", t("settings.title")), languageRow(), el("p", "", t("settings.launchers_text")));

  const list = el("div", "switches");
  for (const platform of platforms) {
    const row = el("button", "switch");
    row.setAttribute("role", "switch");
    row.setAttribute("aria-checked", String(platform.enabled));
    row.append(el("span", "", platform.label), el("i", "knob"));
    row.addEventListener("click", async () => {
      // `platforms` est remplacé après chaque enregistrement : on relit l'état courant.
      const current = platforms.find((p) => p.id === platform.id);
      current.enabled = !current.enabled;
      row.setAttribute("aria-checked", String(current.enabled));
      try {
        const enabled = platforms.filter((p) => p.enabled).map((p) => p.id);
        applyState(await invoke("set_platforms", { enabled, tz }));
        renderLibrary();
      } catch (error) {
        toast(String(error), true);
      }
    });
    list.append(row);
  }
  const closeRow = el("button", "switch");
  closeRow.setAttribute("role", "switch");
  closeRow.setAttribute("aria-checked", String(closeToTray));
  closeRow.append(el("span", "", t("settings.close_tray")), el("i", "knob"));
  closeRow.addEventListener("click", async () => {
    const next = !closeToTray;
    try {
      await invoke("set_close_to_tray", { enabled: next });
      closeToTray = next;
      closeRow.setAttribute("aria-checked", String(next));
    } catch (error) {
      toast(String(error), true);
    }
  });

  const shortcutRow = el("div", "setting-row");
  const shortcutButton = button("btn ghost", shortcutLabel(shortcut), () => captureShortcut(shortcutButton));
  shortcutRow.append(el("span", "", t("settings.shortcut")), shortcutButton);

  const colorInput = el("input", "color-input");
  colorInput.type = "color";
  colorInput.value = look.custom_accent || accentHex;
  colorInput.setAttribute("aria-label", t("settings.accent_free"));
  // Aperçu en direct pendant le choix ; l'enregistrement se fait à la validation.
  colorInput.addEventListener("input", () => {
    look.custom_accent = colorInput.value;
    applyLook();
  });
  colorInput.addEventListener("change", async () => {
    for (const other of swatches.children) other.setAttribute("aria-pressed", "false");
    await saveLook({ custom_accent: colorInput.value });
  });

  const swatches = el("div", "swatches");
  for (const accent of accents) {
    const swatch = el("button", "swatch-color");
    swatch.style.setProperty("--swatch", accent.hex);
    const accentLabel = t(`accent.${accent.id}`);
    swatch.title = accentLabel;
    swatch.setAttribute("aria-label", accentLabel);
    swatch.setAttribute("aria-pressed", String(accent.id === accentId && !look.custom_accent));
    swatch.addEventListener("click", async () => {
      try {
        const hex = await invoke("set_accent", { id: accent.id });
        accentId = accent.id;
        accentHex = hex;
        look.custom_accent = "";
        colorInput.value = hex;
        applyLook();
        for (const other of swatches.children) other.setAttribute("aria-pressed", "false");
        swatch.setAttribute("aria-pressed", "true");
      } catch (error) {
        toast(String(error), true);
      }
    });
    swatches.append(swatch);
  }

  const downloadRow = el("button", "switch");
  downloadRow.setAttribute("role", "switch");
  downloadRow.setAttribute("aria-checked", String(downloadCovers));
  downloadRow.append(el("span", "", t("settings.covers_download")), el("i", "knob"));
  downloadRow.addEventListener("click", async () => {
    const next = !downloadCovers;
    try {
      await invoke("set_download_covers", { enabled: next, key: null });
      downloadCovers = next;
      downloadRow.setAttribute("aria-checked", String(next));
    } catch (error) {
      toast(String(error), true);
    }
  });

  const steamRow = el("button", "switch");
  steamRow.setAttribute("role", "switch");
  steamRow.setAttribute("aria-checked", String(showUninstalled));
  steamRow.append(el("span", "", t("settings.steam_show")), el("i", "knob"));
  steamRow.addEventListener("click", async () => {
    const next = !showUninstalled;
    try {
      await invoke("set_steam_account", { enabled: next, key: null });
      showUninstalled = next;
      steamRow.setAttribute("aria-checked", String(next));
      if (next && steamKeySet) await syncOwnedGames();
      else if (next) toast(t("settings.steam_add_key"));
      else {
        applyState(await invoke("get_state", { tz }));
        renderLibrary();
      }
    } catch (error) {
      toast(String(error), true);
    }
  });
  const steamKeyInput = el("input", "text-input");
  steamKeyInput.type = "text";
  steamKeyInput.autocomplete = "off";
  steamKeyInput.spellcheck = false;
  const steamKeyHint = () => (t(steamKeySet ? "settings.steam_key_set" : "settings.steam_key"));
  steamKeyInput.placeholder = steamKeyHint();
  steamKeyInput.addEventListener("change", async () => {
    try {
      await invoke("set_steam_account", { enabled: showUninstalled, key: steamKeyInput.value });
      steamKeySet = steamKeyInput.value.trim() !== "";
      steamKeyInput.value = "";
      steamKeyInput.placeholder = steamKeyHint();
      toast(steamKeySet ? t("settings.key_saved") : t("settings.key_removed"));
      if (steamKeySet && showUninstalled) await syncOwnedGames();
    } catch (error) {
      toast(String(error), true);
    }
  });

  const keyInput = el("input", "text-input");
  keyInput.type = "text";
  keyInput.autocomplete = "off";
  keyInput.spellcheck = false;
  const sgdbHint = () => t(keySet ? "settings.sgdb_key_set" : "settings.sgdb_key");
  keyInput.placeholder = sgdbHint();
  keyInput.addEventListener("change", async () => {
    try {
      await invoke("set_download_covers", { enabled: downloadCovers, key: keyInput.value });
      keySet = keyInput.value.trim() !== "";
      keyInput.value = "";
      keyInput.placeholder = sgdbHint();
      toast(keySet ? t("settings.key_saved") : t("settings.key_removed"));
    } catch (error) {
      toast(String(error), true);
    }
  });
  const fetchButton = button("btn ghost", t("settings.fetch_now"), async () => {
    try {
      await invoke("fetch_covers");
      toast(t("settings.fetch_started"));
    } catch (error) {
      toast(String(error), true);
    }
  });

  const onlineRow = el("button", "switch");
  onlineRow.setAttribute("role", "switch");
  onlineRow.setAttribute("aria-checked", String(onlineInfo));
  onlineRow.append(el("span", "", t("settings.online")), el("i", "knob"));
  onlineRow.addEventListener("click", async () => {
    const next = !onlineInfo;
    try {
      await invoke("set_online_info", { enabled: next, key: null });
      onlineInfo = next;
      onlineRow.setAttribute("aria-checked", String(next));
    } catch (error) {
      toast(String(error), true);
    }
  });

  const rawgInput = el("input", "text-input");
  rawgInput.type = "text";
  rawgInput.autocomplete = "off";
  rawgInput.spellcheck = false;
  const rawgHint = () => (t(rawgKeySet ? "settings.rawg_key_set" : "settings.rawg_key"));
  rawgInput.placeholder = rawgHint();
  rawgInput.addEventListener("change", async () => {
    try {
      await invoke("set_online_info", { enabled: onlineInfo, key: rawgInput.value });
      rawgKeySet = rawgInput.value.trim() !== "";
      rawgInput.value = "";
      rawgInput.placeholder = rawgHint();
      toast(rawgKeySet ? t("settings.key_saved") : t("settings.key_removed"));
    } catch (error) {
      toast(String(error), true);
    }
  });

  panel.append(
    list,
    el("h3", "", t("settings.steam_title")),
    steamRow,
    el("p", "", t("settings.steam_text")),
    steamKeyInput,
    el("h3", "", t("settings.pages_title")),
    onlineRow,
    el("p", "", t("settings.online_text")),
    rawgInput,
    el("h3", "", t("settings.look_title")),
    segmented(t("settings.theme"), "theme", [["dark", t("settings.theme_dark")], ["light", t("settings.theme_light")], ["auto", t("settings.theme_auto")]]),
    el("h3", "", t("settings.accent_title")),
    swatches,
    (() => {
      const row = el("div", "setting-row");
      row.append(el("span", "", t("settings.accent_free_row")), colorInput);
      return row;
    })(),
    el("h3", "", t("settings.library_title")),
    segmented(t("settings.view"), "view", [["grid", t("settings.view_grid")], ["list", t("settings.view_list")]]),
    segmented(t("settings.size"), "card_size", [["small", t("settings.size_small")], ["medium", t("settings.size_medium")], ["large", t("settings.size_large")]]),
    segmented(t("settings.radius"), "radius", [["square", t("settings.radius_square")], ["round", t("settings.radius_round")], ["soft", t("settings.radius_soft")]]),
    lookSwitch(t("settings.show_hero"), "show_hero"),
    lookSwitch(t("settings.hero_bg"), "hero_background"),
    button("btn ghost", t("settings.reset_look"), async () => {
      try {
        await invoke("reset_look");
        applyState(await invoke("get_state", { tz }));
        renderLibrary();
        openSettings();
      } catch (error) {
        toast(String(error), true);
      }
    }),
    el("h3", "", t("settings.covers_title")),
    downloadRow,
    el("p", "", t("settings.covers_text")),
    keyInput,
    fetchButton,
    el("h3", "", t("settings.window_title")),
    closeRow,
    shortcutRow,
    button("btn ghost", t("common.close"), closeSettings),
  );
  overlay.append(panel);
  overlay.hidden = false;
  panel.querySelector("button").focus();
}

// Textes de la barre du haut (écrits dans index.html pour l'affichage avant le premier rendu).
function applyStaticText() {
  document.documentElement.setAttribute("lang", currentLang());
  document.getElementById("import").textContent = t("app.import");
  document.getElementById("add").textContent = t("app.add");
  for (const [id, key] of [["stats", "app.stats"], ["settings", "app.settings"]]) {
    const node = document.getElementById(id);
    node.setAttribute("aria-label", t(key));
    node.title = t(key);
  }
}

// Choix de la langue : appliqué tout de suite, sans redémarrer ; l'icône près de l'horloge suit aussi.
async function changeLanguage(value) {
  try {
    const state = await invoke("set_language", { language: value, tz });
    setStrings(await invoke("get_strings"));
    applyStaticText();
    applyState(state);
    renderLibrary();
    openSettings();
  } catch (error) {
    toast(String(error), true);
  }
}

function languageRow() {
  const row = el("div", "setting-row");
  const group = el("div", "seg");
  for (const value of ["auto", "fr", "en"]) {
    const choice = button("seg-btn", value === "auto" ? t("lang.auto") : t(`lang.${value}_name`), () => changeLanguage(value));
    choice.setAttribute("aria-pressed", String(language === value));
    group.append(choice);
  }
  row.append(el("span", "", t("settings.language")), group);
  return row;
}

// Enregistre un changement d'apparence : appliqué tout de suite, retenu pour le prochain lancement.
async function saveLook(patch) {
  look = { ...look, ...patch };
  applyLook();
  renderLibrary();
  try {
    look = { ...DEFAULT_LOOK, ...(await invoke("set_look", { look })) };
    applyLook();
  } catch (error) {
    toast(String(error), true);
  }
}

// Choix parmi quelques valeurs, en boutons côte à côte.
function segmented(label, key, options) {
  const row = el("div", "setting-row");
  const group = el("div", "seg");
  for (const [value, text] of options) {
    const choice = button("seg-btn", text, () => {
      for (const other of group.children) other.setAttribute("aria-pressed", String(other === choice));
      saveLook({ [key]: value });
    });
    choice.setAttribute("aria-pressed", String(look[key] === value));
    group.append(choice);
  }
  row.append(el("span", "", label), group);
  return row;
}

function lookSwitch(label, key) {
  const row = el("button", "switch");
  row.setAttribute("role", "switch");
  row.setAttribute("aria-checked", String(look[key]));
  row.append(el("span", "", label), el("i", "knob"));
  row.addEventListener("click", () => {
    row.setAttribute("aria-checked", String(!look[key]));
    saveLook({ [key]: !look[key] });
  });
  return row;
}

const shortcutLabel = (accelerator) => accelerator.replaceAll("Space", t("settings.key_space"));

// Nom de touche attendu par le raccourci global, à partir de `event.code`.
function keyName(code) {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit\d$/.test(code)) return code.slice(5);
  if (/^F([1-9]|1\d|2[0-4])$/.test(code)) return code;
  const named = { Space: "Space", Enter: "Enter", Tab: "Tab", Backspace: "Backspace", ArrowUp: "Up", ArrowDown: "Down", ArrowLeft: "Left", ArrowRight: "Right" };
  return named[code] ?? null;
}

// Une combinaison valide = au moins un modificateur + une touche.
function acceleratorFrom(event) {
  const key = keyName(event.code);
  const modifiers = [];
  if (event.ctrlKey) modifiers.push("Ctrl");
  if (event.altKey) modifiers.push("Alt");
  if (event.shiftKey) modifiers.push("Shift");
  if (event.metaKey) modifiers.push("Super");
  return key && modifiers.length > 0 ? [...modifiers, key].join("+") : null;
}

function captureShortcut(target) {
  if (capturing) return;
  capturing = true;
  target.textContent = t("settings.shortcut_press");
  const finish = () => {
    capturing = false;
    document.removeEventListener("keydown", onKey, true);
    target.textContent = shortcutLabel(shortcut);
  };
  const onKey = async (event) => {
    event.preventDefault();
    event.stopPropagation();
    if (event.key === "Escape") return finish();
    const accelerator = acceleratorFrom(event);
    if (!accelerator) return; // une touche seule ne suffit pas : on attend la combinaison
    try {
      shortcut = await invoke("set_shortcut", { shortcut: accelerator });
      toast(t("settings.shortcut_done", { shortcut: shortcutLabel(shortcut) }));
    } catch (error) {
      toast(String(error), true);
    }
    finish();
  };
  document.addEventListener("keydown", onKey, true);
}

function closeSettings() {
  overlay.hidden = true;
  overlay.replaceChildren();
}

overlay.addEventListener("click", (event) => {
  if (event.target === overlay) closeSettings();
});
document.addEventListener("keydown", (event) => {
  if (event.key === "Escape" && !overlay.hidden && !capturing) closeSettings();
});

// ---------------------------------------------------------------- Actions

async function addProgram() {
  try {
    const path = await open({
      multiple: false,
      filters: [{ name: t("add.filter"), extensions: ["exe", "lnk", "bat"] }],
    });
    if (!path) return;
    programs = await invoke("add_program", { path, tz });
    renderLibrary();
  } catch (error) {
    toast(String(error), true);
  }
}

async function importGames() {
  try {
    const report = await invoke("import_all", { tz });
    programs = report.programs;
    renderLibrary();
    // Les jeux possédés mais non installés sont relus à chaque import, jamais au démarrage.
    if (showUninstalled && steamKeySet) {
      // En cas d'échec, l'erreur reste affichée : elle compte plus que le nombre de jeux importés.
      const synced = await syncOwnedGames(report.added.length === 0);
      if (!synced || report.added.length === 0) return;
    }
    if (report.added.length === 0) {
      toast(
        report.searched === 0
          ? t("import.none_active")
          : t("import.none_new"),
      );
      return;
    }
    const total = report.added.reduce((sum, [, n]) => sum + n, 0);
    const detail = report.added.map(([source, n]) => `${sourceName(source)} ${n}`).join(", ");
    toast(tn("import.done", total, { detail }));
  } catch (error) {
    toast(String(error), true);
  }
}

// Relit les jeux possédés sur le compte Steam. `announce` : annoncer le résultat. Renvoie vrai si cela a réussi.
async function syncOwnedGames(announce = true) {
  try {
    const report = await invoke("sync_owned_games", { tz });
    programs = report.programs;
    renderLibrary();
    if (announce) toast(report.uninstalled === 0 ? t("steam.none_uninstalled") : tn("steam.uninstalled", report.uninstalled));
    return true;
  } catch (error) {
    toast(String(error), true);
    return false;
  }
}

async function installProgram(program) {
  try {
    await invoke("install_program", { id: program.id });
    toast(t("steam.opening"));
  } catch (error) {
    toast(String(error), true);
  }
}

async function launch(program) {
  // Un jeu non installé ne se lance pas : on ouvre sa page, où « Installer » l'attend.
  if (program.uninstalled) return openDetailsPage(program);
  try {
    programs = await invoke("launch_program", { id: program.id, tz });
    renderLibrary();
  } catch (error) {
    toast(String(error), true);
  }
}

document.getElementById("add").addEventListener("click", addProgram);
document.getElementById("import").addEventListener("click", importGames);
document.getElementById("settings").addEventListener("click", openSettings);
document.getElementById("stats").addEventListener("click", toggleStats);

// ------------------------------------------------------------- Démarrage

const [strings, initial] = await Promise.all([invoke("get_strings"), invoke("get_state", { tz })]);
setStrings(strings);
applyStaticText();
applyState(initial);
if (onboarded) {
  renderLibrary();
  if (!initial.shortcut_ok) {
    toast(t("startup.shortcut_taken", { shortcut: shortcutLabel(shortcut) }), true);
  }
  // Animation d'entrée, jouée une seule fois.
  view.classList.add("enter");
  setTimeout(() => view.classList.remove("enter"), 900);
} else {
  renderOnboarding();
}
requestAnimationFrame(() => requestAnimationFrame(() => invoke("app_ready")));

// Une page de jeu se met à jour pendant son chargement (texte, captures, actualités).
window.__TAURI__.event.listen("details-updated", (event) => {
  // Pendant une modification, on ne remplace pas la page : ce que le joueur tape serait perdu.
  if (screen === "details" && event.payload === detailsId && !editing) renderDetails(false);
});

// Un lancement fait depuis l'icône ou la mini-fenêtre change le classement : on recharge.
window.__TAURI__.event.listen("library-changed", async (event) => {
  if (event.payload === "main") return;
  applyState(await invoke("get_state", { tz }));
  if (onboarded) renderLibrary();
});
