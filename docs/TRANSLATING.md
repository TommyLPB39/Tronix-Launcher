# Traduire Tronix Launcher

**Français** · [English](#translating-tronix-launcher)

Les textes de l'application sont dans **un fichier par langue** : [`src/locales/`](../src/locales). Le français (`fr.json`) est la référence, l'anglais (`en.json`) la langue de repli : un texte absent d'une langue s'affiche en anglais.

## Format

Fichiers JSON **à plat** (une clé = un texte), au format *i18next JSON* que [Weblate](https://weblate.org) sait lire :

```json
{
  "library.title": "Ta bibliothèque",
  "coll.games_one": "{{n}} jeu",
  "coll.games_other": "{{n}} jeux",
  "err.title_too_long": "Le titre est trop long ({{max}} caractères maximum)."
}
```

- **Variables** : `{{nom}}`. Elles doivent être **identiques** dans toutes les langues (le test le vérifie). Ne les traduis pas.
- **Pluriels** : deux clés, `…_one` et `…_other`. L'application choisit selon les règles de la langue (`Intl.PluralRules`). Si ta langue a d'autres formes (`_few`, `_many`…), ajoute-les : elles seront utilisées quand elles existent.
- **`meta.locale`** : l'étiquette de langue utilisée pour les dates et nombres (`fr-FR`, `en-US`, `de-DE`…).
- Les noms de marques (Steam, Epic, GOG…) ne se traduisent pas.
- Pas de HTML dans les textes.
- Les clés `err.*`, `tray.*` et `rating.*` sont utilisées par le cœur Rust, les autres par l'interface ; le fichier est le même pour les deux.

## Ajouter une langue

1. Copie `src/locales/en.json` vers `src/locales/<code>.json` (par ex. `de.json`) et traduis les valeurs.
2. Règle `meta.locale`, `lang.<code>` (nom de la langue dans la langue courante) et `lang.<code>_name` (nom de la langue **écrit dans cette langue**, ex. « Deutsch »). Ajoute aussi `lang.<code>` et `lang.<code>_name` aux autres fichiers.
3. Déclare la langue dans le code :
   - `src-tauri/src/i18n.rs` : ajoute le code à `LANGS`, un `include_str!` et un cas dans `table()` ;
   - `src/main.js` : ajoute le code à la liste du sélecteur (`languageRow`) ;
   - `src-tauri/src/settings.rs` : accepte le code dans `language()`.
4. Lance `npm run test:ui` (vérifie les clés et les variables) et `cargo test --manifest-path src-tauri/Cargo.toml`.

Les langues proposées au choix sont aujourd'hui : Automatique (langue de Windows), français, anglais.

## Weblate (plus tard)

Les fichiers sont déjà au bon format. Pour brancher un projet Weblate, créer un composant de type **« i18next JSON file »** avec le masque de fichiers `src/locales/*.json`, la langue source `fr` (ou `en`), et laisser Weblate proposer ses modifications par pull request. Rien ne change dans le code.

---

# Translating Tronix Launcher

All app texts live in **one file per language**: [`src/locales/`](../src/locales). French (`fr.json`) is the reference, English (`en.json`) is the fallback: a text missing from a language is shown in English.

## Format

**Flat** JSON files (one key = one text) in the *i18next JSON* format that [Weblate](https://weblate.org) can read:

```json
{
  "library.title": "Your library",
  "coll.games_one": "{{n}} game",
  "coll.games_other": "{{n}} games",
  "err.title_too_long": "The title is too long ({{max}} characters maximum)."
}
```

- **Variables**: `{{name}}`. They must be **identical** in every language (a test checks it). Do not translate them.
- **Plurals**: two keys, `…_one` and `…_other`. The app picks according to the language's rules (`Intl.PluralRules`). If your language has more forms (`_few`, `_many`…), add them: they are used when present.
- **`meta.locale`**: the locale tag used for dates and numbers (`fr-FR`, `en-US`, `de-DE`…).
- Brand names (Steam, Epic, GOG…) are not translated.
- No HTML in texts.
- `err.*`, `tray.*` and `rating.*` keys are used by the Rust core, the others by the interface; the file is the same for both.

## Add a language

1. Copy `src/locales/en.json` to `src/locales/<code>.json` (e.g. `de.json`) and translate the values.
2. Set `meta.locale`, `lang.<code>` (the language's name in the current language) and `lang.<code>_name` (the language's name **written in that language**, e.g. "Deutsch"). Also add `lang.<code>` and `lang.<code>_name` to the other files.
3. Declare the language in the code:
   - `src-tauri/src/i18n.rs`: add the code to `LANGS`, an `include_str!` and a case in `table()`;
   - `src/main.js`: add the code to the picker list (`languageRow`);
   - `src-tauri/src/settings.rs`: accept the code in `language()`.
4. Run `npm run test:ui` (checks keys and variables) and `cargo test --manifest-path src-tauri/Cargo.toml`.

Languages offered today: Automatic (Windows language), French, English.

## Weblate (later)

The files are already in the right format. To connect a Weblate project, create a component of type **"i18next JSON file"** with the file mask `src/locales/*.json`, source language `fr` (or `en`), and let Weblate propose its changes as pull requests. Nothing changes in the code.
