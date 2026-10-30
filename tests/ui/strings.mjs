// Textes français pour les faux cœurs des scénarios : les mêmes fichiers que l'application.
import { readFileSync } from "node:fs";

const locale = (name) => JSON.parse(readFileSync(new URL(`../../src/locales/${name}.json`, import.meta.url), "utf8"));

export const STRINGS = { lang: "fr", strings: locale("fr") };
export const STRINGS_EN = { lang: "en", strings: { ...locale("fr"), ...locale("en") } };
