# Architecture

> **English summary.** Tronix Launcher is a Rust + Tauri 2 app with a framework-free HTML/CSS/JS interface (`src/`). The Rust core (`src-tauri/src/`)
> owns all state and exposes it through Tauri commands; the interface only displays it. Data lives in `%APPDATA%\com.tronix.launcher\`
> (`programs.json`, `settings.json`, `covers\`, `details\`). Security is layered: strict CSP, minimal window permissions, DPAPI-encrypted API keys,
> validated identifiers/URLs, https-only network. This document is in French; the sections "Security", "Rules" and "Not verified" are the most useful to review first.

## 1. Vue d'ensemble

Tronix Launcher est un launcher Windows (jeux et logiciels) : **performance d'abord**, puis simplicité et différenciation (mettre en avant ce que le joueur veut lancer *maintenant*).

- **Rust + Tauri 2** pour le cœur et la fenêtre, **HTML/CSS/JS sans framework** pour l'interface (pas d'étape de build côté interface : `frontendDist` pointe sur `src/`).
- Le cœur Rust **possède l'état** ; l'interface ne fait qu'appeler des **commandes Tauri** (`invoke`) et afficher ce qu'elles renvoient.
- Windows d'abord. macOS et Linux sont possibles avec Tauri, mais le code actuel dépend de Windows (DPAPI, registre, `explorer.exe`).

## 2. Où est quoi

| Chemin | Rôle |
|---|---|
| `src/` | Interface : `index.html`, `main.js` (fenêtre principale), `quick.html` + `quick.js` (mini-fenêtre de lancement rapide), `i18n.js`, `locales/` (textes par langue), `styles.css`, `fonts/` |
| `src-tauri/src/lib.rs` | Commandes Tauri, état (`AppState`), icône de la zone de notification, raccourci global, mini-fenêtre |
| `src-tauri/src/store.rs` | Bibliothèque (`programs.json`), lancement des programmes, favoris, collections, jeux Steam non installés |
| `src-tauri/src/settings.rs` | Réglages (`settings.json`) : launchers actifs, apparence, tri, collections, clés (chiffrées) |
| `src-tauri/src/rank.rs` | Classement par probabilité (récence + fréquence + heure habituelle) |
| `src-tauri/src/imports.rs`, `launchers.rs` | Détection des jeux : Steam, Epic, GOG, Ubisoft / Xbox, EA, Battle.net |
| `src-tauri/src/steam_account.rs` | SteamID (lu dans `loginusers.vdf`) et liste des jeux possédés (API web Steam, clé d'API) |
| `src-tauri/src/covers.rs` | Pochettes : locales, personnelles, téléchargées (Steam CDN, SteamGridDB), garde-fous réseau |
| `src-tauri/src/details.rs` | Pages des jeux : Steam, RAWG, Wikipédia ; cache sur disque |
| `src-tauri/src/secret.rs` | Chiffrement des clés d'API avec Windows DPAPI |
| `src-tauri/src/i18n.rs` | Langues : lecture des fichiers de `src/locales`, langue de Windows, messages du cœur |
| `src-tauri/src/playtime.rs`, `stats.rs`, `icons.rs` | Temps de jeu, statistiques, icône d'un `.exe` |
| `src-tauri/tauri.conf.json`, `capabilities/` | Fenêtre, CSP, protocole d'images, permissions, installeur NSIS |
| `tests/ui/` | Tests de l'interface (faux DOM, sans navigateur) |
| `assets/logo.png` | Logo d'origine (PNG transparent). `src/logo.png` en est la version recadrée utilisée par l'interface : seul son canal alpha sert (masque CSS), la couleur vient du thème (`--logo`, dérivée de l'accent, plus foncée en thème clair). Les icônes de `src-tauri/icons/` et `app-icon.png` en sont tirées (logo menthe sur carré sombre arrondi) |
| `docs/` | `PRD.md` (86 user stories), `DESIGN.md` (système de design), ce fichier |

**Données de l'utilisateur** : `%APPDATA%\com.tronix.launcher\` → `programs.json`, `settings.json` (clés chiffrées, préfixe `dpapi:`), `covers\`, `details\`.
Un fichier illisible est mis de côté en `.corrupt`, jamais écrasé (`programs.json` comme `settings.json`).

## 3. Règles de conception (à respecter)

1. **Aucun appel réseau au démarrage.** Les appels réseau ne partent que de réglages activés par l'utilisateur (pochettes, informations en ligne, compte Steam à l'import), et **jamais en tenant un verrou**.
2. **Ordre des verrous partout : réglages, puis bibliothèque** (`settings` avant `store`). Les verrous se prennent avec `lock_safe()` (survit à un panic d'un autre fil).
3. **Identifiants stables** : `make_id(source, clé)` = hachage FNV-1a de `source|clé`. Un jeu Steam non installé a le **même** identifiant que lorsqu'il est installé.
4. **Lancement** : un `.exe` est lancé directement (durée mesurable) ; tout le reste (liens `steam://`, `shell:AppsFolder\…`, `.bat`, `.lnk`) passe par `explorer.exe` (chemin complet), **jamais `cmd /C start`** (il coupe la cible au premier `&`). Seuls les types de lien produits par les imports sont acceptés.
5. **Interface** : DOM construit avec `el()` / `button()`, **jamais d'`innerHTML`** avec du contenu externe. Les textes HTML venus d'Internet sont convertis en texte brut côté Rust (`html_to_text`).
6. Un nouveau launcher ou une nouvelle option est **actif par défaut**, sauf ce qui envoie des données sur Internet (désactivé d'office).
7. Les images (pochettes, captures) sont servies par le **protocole d'images** de Tauri, limité à `$APPDATA/covers/**` et `$APPDATA/details/**`.

## 3 bis. Langues

- Les textes sont dans `src/locales/<langue>.json` (clés à plat, pluriels `_one` / `_other`, variables `{{nom}}`, format i18next lisible par Weblate). **Une seule source** pour l'interface et le cœur Rust (`i18n.rs` les embarque avec `include_str!`).
- Le cœur choisit la langue : réglage `language` ("auto", "fr", "en") dans `settings.json` ; « auto » suit la langue d'affichage de Windows (`GetUserDefaultUILanguage`), l'anglais pour toute langue non gérée. Commandes `get_strings` (dictionnaire complété par l'anglais) et `set_language` (applique tout de suite, rafraîchit le menu de l'icône).
- L'interface (`src/i18n.js`) expose `t(clé, variables)` et `tn(clé, n, variables)` (pluriel selon `Intl.PluralRules`). Une clé absente s'affiche telle quelle. La fenêtre rapide recharge les textes à chaque ouverture.
- Les messages d'erreur du cœur sont traduits **à leur création** (`i18n::t`) avec la langue en cours ; les constantes de `steam_account.rs` sont des clés, traduites à l'affichage. Les jours de la semaine des statistiques sont envoyés comme clés anglaises (`sunday`…) et traduits par l'interface.
- Les pages en ligne suivent la langue : `l=french|english` pour Steam, Wikipédia dans la langue puis l'autre. Chaque page gardée mémorise sa langue (`Details.ui_lang`) ; `Details::is_fresh` la rafraîchit si la langue a changé (une page sans langue enregistrée compte comme française).
- Tests : `tests/ui/00-locales.test.mjs` (clés, variables, pluriels, clés utilisées par le code JS et Rust), `12-language.test.mjs` (anglais, pluriels, dates, changement de langue) ; côté Rust, `i18n.rs` vérifie la cohérence des deux fichiers.
- Ajouter une langue ou brancher Weblate : voir [TRANSLATING.md](TRANSLATING.md).

## 4. Sécurité

- **CSP stricte** (`tauri.conf.json`) : scripts et styles de l'application seulement, images locales + `*.steamgriddb.com`, aucune connexion sortante depuis la page, ni objet, ni cadre, ni formulaire.
- **Permissions des fenêtres au minimum** (`capabilities/default.json`) : écouter les événements + `dialog:allow-open`. Tout le reste passe par les commandes de l'application. Les commandes ne sont **pas** restreintes par fenêtre (les deux fenêtres peuvent toutes les appeler).
- **Clés d'API chiffrées** (DPAPI, compte Windows courant) ; une clé chiffrée par un autre compte ou PC est lue comme vide. Les erreurs réseau n'affichent jamais l'adresse (`covers::net_error`).
- **Réseau** : https seulement ; les appels qui portent une clé dans l'adresse (RAWG, API Steam) ne suivent aucune redirection ; les téléchargements d'images valident chaque adresse (`covers::remote_url_ok` : https, nom de domaine, ni IP, ni nom local) et chaque redirection ; corps de réponse limités.
- **Chemins** : tout identifiant venu de l'interface est contrôlé (`store::is_safe_id`) avant d'entrer dans un nom de fichier ; « Ajouter » n'accepte que `.exe`, `.lnk`, `.bat`, `.cmd` ; une image personnelle doit être un vrai fichier ≤ 8 Mo reconnu par ses premiers octets.
- **Pas de mot de passe Steam** : l'identifiant du compte est lu dans les fichiers de Steam, la liste des jeux vient de l'API web avec une clé gratuite.
- **Non fait** : audit des dépendances (`cargo audit`), signature de code de l'installeur, test d'intrusion.

## 5. Construire, tester, mesurer

Voir [README.md](../README.md#compiler). Points utiles :

- Le profil **`fast`** (dans `Cargo.toml`) compile plus vite que `release` pour essayer l'application.
- Sous Windows, un chemin de projet trop long fait échouer l'édition de liens (`LNK1104`) : utiliser `CARGO_TARGET_DIR` près de la racine du disque.
- `cargo test` : tests unitaires du cœur (formats d'import sur des fichiers d'exemple, DPAPI, classement, sécurité des identifiants et des adresses…). Un test réseau est ignoré par défaut (`-- --ignored`).
- `npm run test:ui` : fait tourner `src/main.js` avec un faux DOM et un faux cœur Tauri, et rejoue les scénarios d'usage.
- **Performance mesurée** (installeur, démarrage jusqu'à l'accueil, à chaud) : médiane ~0,5 s (objectif < 2 s). Le démarrage à froid n'est pas mesuré.
- Pour essayer l'interface dans le moteur web réel avec des captures, on peut lancer l'exécutable avec `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9333` et piloter la page par le protocole CDP.

## 6. Ce qui n'est PAS vérifié

À ne pas présenter comme acquis :

- **Epic, GOG, Ubisoft, Xbox, EA, Battle.net** : lecteurs testés sur des fichiers d'exemple (formats de `MicrosoftGame.Config`, `installerdata.xml`, codes `battlenet://`, lien `link2ea://` issus de sources communautaires). Seuls l'identifiant d'une application du Store et le lancement `shell:AppsFolder` ont été confirmés sur de vraies applications.
- **RAWG**, **SteamGridDB** (y compris le sélecteur de pochettes) et la **liste des jeux possédés** de l'API Steam : jamais appelés avec une vraie clé ; testés sur des réponses d'exemple. Vérifié en réel : le refus d'une fausse clé Steam, et la lecture du SteamID.
- **Avis Steam (pourcentage)** : l'adresse `appreviews` a redirigé vers la page d'accueil sur la machine de test ; repli sur le nombre de recommandations et la note Metacritic.
- **Icône de la zone de notification**, **boîtes de dialogue de choix de fichier** : jamais pilotées pour de vrai.
- **Temps de jeu estimé** sur de vrais launchers : testé avec un faux jeu seulement.
- **Raccourci global** par défaut (Ctrl+Alt+Espace) : peut être pris par un autre programme ; l'application prévient l'utilisateur.
- **macOS / Linux** : rien essayé.

## 6 bis. Idées et points ouverts

- Signature de code de l'installeur (sinon avertissement SmartScreen) — demande un certificat payant.
- Mise à jour automatique, journal de versions, tests d'intégration en CI.
- Page d'homonymie Wikipédia (ex. « Paint ») : donne « aucune page », l'utilisateur corrige à la main.
- Traduction de l'interface (le texte est en français, sans système de traduction).
- Restreindre les commandes Tauri par fenêtre.

## 7. Pièges courants

- Les fichiers de `src/` sont **embarqués à la compilation** : un changement d'interface demande de recompiler.
- Un `programs.json` / `settings.json` préparé à la main doit être écrit **sans BOM** si possible (le programme le tolère, mais d'autres outils non).
- Le disque peut manquer de place : les dossiers de compilation (`target`) pèsent plusieurs Go et se régénèrent.
