// Langues : mêmes clés et mêmes variables dans chaque fichier, toutes les clés utilisées par le code existent.
import { readdirSync, readFileSync } from "node:fs";

const root = new URL("../../", import.meta.url);
const read = (path) => readFileSync(new URL(path, root), "utf8");
const locales = Object.fromEntries(
  readdirSync(new URL("src/locales/", root))
    .filter((name) => name.endsWith(".json"))
    .map((name) => [name.replace(".json", ""), JSON.parse(read(`src/locales/${name}`))]),
);

let ok = true;
const check = (label, got, want = []) => {
  const pass = JSON.stringify(got) === JSON.stringify(want);
  ok &&= pass;
  console.log(`${pass ? "OK " : "KO "} ${label}${pass ? "" : ` -> ${JSON.stringify(got).slice(0, 400)}`}`);
};

const langs = Object.keys(locales).sort();
check("langues présentes", langs.includes("fr") && langs.includes("en"), true);

const placeholders = (text) => [...text.matchAll(/\{\{(\w+)\}\}/g)].map((m) => m[1]).sort();
const reference = locales.fr;
for (const lang of langs) {
  const data = locales[lang];
  check(`${lang} : mêmes clés que fr (manquantes)`, Object.keys(reference).filter((key) => !(key in data)));
  check(`${lang} : mêmes clés que fr (en trop)`, Object.keys(data).filter((key) => !(key in reference)));
  check(`${lang} : mêmes variables que fr`, Object.keys(data).filter((key) => key in reference && JSON.stringify(placeholders(data[key])) !== JSON.stringify(placeholders(reference[key]))));
  check(`${lang} : aucun texte vide`, Object.keys(data).filter((key) => typeof data[key] !== "string" || data[key].trim() === ""));
  check(`${lang} : pluriels complets`, Object.keys(data).filter((key) => key.endsWith("_one") && !(`${key.slice(0, -4)}_other` in data)));
  check(`${lang} : pas de balise HTML dans les textes`, Object.keys(data).filter((key) => /<[a-z/]/i.test(data[key])));
}

// Toutes les clés écrites dans le code existent (une clé de pluriel existe si `_other` existe).
const known = (key) => key in reference || `${key}_other` in reference;
const used = (source) => [...source.matchAll(/"([a-z]+(?:\.[a-z0-9_]+)+)"/g)].map((m) => m[1]).filter((key) => !/\.(js|json|html|css|exe|com)$/.test(key));
for (const file of ["src/main.js", "src/quick.js"]) {
  check(`${file} : clés inconnues`, [...new Set(used(read(file)))].filter((key) => !known(key)));
}
// Familles de clés construites par le code (`t(\`sort.${value}\`)`…).
const family = (prefix, names) => names.filter((name) => !known(`${prefix}.${name}`)).map((name) => `${prefix}.${name}`);
check("familles : tris", family("sort", ["rank", "name", "recent", "playtime", "added"]));
check("familles : jours", family("weekday", ["sunday", "monday", "tuesday", "wednesday", "thursday", "friday", "saturday"]));
check("familles : couleurs", family("accent", ["tronix", "braise", "ciel", "citron", "rose"]));
check("familles : langues", [...family("lang", langs), ...langs.map((l) => `lang.${l}_name`).filter((key) => !(key in reference))]);

// Messages du cœur Rust.
const rustFiles = readdirSync(new URL("src-tauri/src/", root)).filter((name) => name.endsWith(".rs"));
const rustKeys = new Set();
for (const name of rustFiles) {
  for (const m of read(`src-tauri/src/${name}`).matchAll(/"((?:err|tray|rating)\.[a-z0-9_]+)"/g)) rustKeys.add(m[1]);
}
check("clés utilisées par le cœur Rust", [...rustKeys].filter((key) => !(key in reference)));
check("le cœur Rust utilise bien des clés", rustKeys.size > 40, true);

process.exit(ok ? 0 : 1);
