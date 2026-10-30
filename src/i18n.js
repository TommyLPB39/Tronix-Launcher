// Textes de l'interface. Le cœur de l'application envoie le dictionnaire de la langue en cours (`get_strings`),
// lu dans src/locales/<langue>.json : clés à plat, pluriels `_one` / `_other`, variables `{{nom}}` (format i18next).
// Une clé absente s'affiche telle quelle : un texte manquant se voit tout de suite, sans casser l'écran.

let strings = {};
let lang = "fr";

export function setStrings(data) {
  strings = data?.strings ?? {};
  lang = data?.lang ?? "fr";
}

const fill = (text, params) => (params ? text.replace(/\{\{(\w+)\}\}/g, (whole, name) => (name in params ? String(params[name]) : whole)) : text);

// Texte d'une clé, avec ses variables : t("coll.of_game", { name }).
export const t = (key, params) => fill(strings[key] ?? key, params);

// Texte au singulier ou au pluriel selon `n` (règles de la langue) : tn("coll.games", 2) cherche coll.games_other.
export function tn(key, n, params = {}) {
  let form = "other";
  try {
    form = new Intl.PluralRules(locale()).select(n);
  } catch {}
  return fill(strings[`${key}_${form}`] ?? strings[`${key}_other`] ?? key, { n, ...params });
}

// Étiquette de langue pour les dates, nombres et tris ("fr-FR", "en-US").
export const locale = () => strings["meta.locale"] ?? "fr-FR";
export const currentLang = () => lang;
