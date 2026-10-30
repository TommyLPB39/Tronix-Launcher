# Contribuer à Tronix Launcher

**Français** · [English](CONTRIBUTING.en.md)

Merci de t'y intéresser ! Le projet est sous [licence MIT](LICENSE) : en proposant une modification, tu acceptes qu'elle soit publiée sous cette licence.

## Avant de commencer

- **Un bug ?** Ouvre un ticket « Bug » (le formulaire te guide). Si ça touche à la **sécurité** (faille, fuite de clé…), ne publie pas les détails dans un ticket : utilise l'onglet *Security* → *Report a vulnerability* du dépôt GitHub s'il est activé, sinon contacte le mainteneur en privé.
- **Une idée ?** Ouvre un ticket « Idée » avant de coder : le projet garde une ligne simple (rapide, épurée, utile) et une fonctionnalité qui ne s'y inscrit pas sera refusée. Les fonctions déjà cadrées sont dans [docs/PRD.md](docs/PRD.md).
- **Un correctif évident** (faute, petit bug) : une pull request directe suffit.

## Mettre en place

Voir [README.md](README.md#compiler) pour les outils. Ensuite :

```powershell
npm install
cargo test --manifest-path src-tauri/Cargo.toml   # tests du cœur (Rust)
npm run test:ui                                    # tests de l'interface (faux DOM)
cargo run --manifest-path src-tauri/Cargo.toml --profile fast
```

## Règles du projet

Elles protègent la vitesse et la sécurité. Les plus importantes :

1. **Aucun appel réseau au démarrage.** Tout appel réseau vient d'un réglage que l'utilisateur a activé, et ne tient jamais un verrou (`Mutex`) pendant qu'il attend.
2. **Interface sans framework**, DOM construit avec `el()` / `button()`. **Jamais d'`innerHTML`** (ni `eval`, ni script en ligne) avec du contenu venu de l'extérieur : la politique de sécurité du contenu (`tauri.conf.json`) les bloque de toute façon.
3. **Tout identifiant venu de l'interface est contrôlé** avant de servir dans un chemin de fichier (`store::is_safe_id`).
4. **Les clés d'API** sont chiffrées sur le disque (`secret.rs`) et n'apparaissent **jamais** dans un message d'erreur, un log ou une adresse affichée (`covers::net_error`). Les appels qui portent une clé dans l'adresse ne suivent aucune redirection.
5. **Ordre des verrous : réglages, puis bibliothèque** (`settings` avant `store`), partout.
6. Les cibles de lancement qui ne sont pas des `.exe` passent par `explorer.exe` (jamais `cmd /C start`), et seuls les types de lien connus sont acceptés.
7. **Les pages et fichiers de l'utilisateur ne sont jamais écrasés** sans copie : un fichier illisible est mis de côté en `.corrupt`.
8. **Dis ce qui est vérifié et ce qui ne l'est pas** dans ta pull request. Ce projet distingue « testé sur de vrais jeux » de « testé sur des fichiers d'exemple ».

## Proposer une modification

1. Fais un *fork*, puis une branche (`fix/...` ou `feat/...`).
2. Garde la modification **petite et ciblée** ; une pull request = un sujet.
3. Ajoute ou adapte les **tests** (`cargo test`, et `tests/ui/` si tu changes `src/main.js`). Les tests doivent passer.
4. Le code et ses commentaires sont en **français** ; tu peux écrire les tiens en **anglais**, c'est bienvenu. **Aucun texte visible ne s'écrit en dur** : il passe par `t("clé")` (interface) ou `i18n::t("clé")` (Rust), avec la clé ajoutée à `fr.json` **et** `en.json` (voir [docs/TRANSLATING.md](docs/TRANSLATING.md)).
5. Remplis le modèle de pull request, avec une capture d'écran si l'interface change.

## Style

- Rust : `cargo fmt` n'est pas imposé aujourd'hui ; imite le style du fichier que tu modifies. Pas de `unwrap()` sur une entrée de l'utilisateur ; pour un verrou, utilise `lock_safe()`.
- JavaScript : modules ES, pas de dépendance ajoutée sans en discuter (le but est un démarrage rapide et un petit installeur).
- Un commentaire explique **pourquoi**, pas ce que fait la ligne.
