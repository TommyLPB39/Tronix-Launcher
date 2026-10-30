# Design System — Tronix Launcher

## Product Context
- **Quoi** : Launcher qui démarre vite et met en avant ce que le joueur veut lancer maintenant.
- **Pour qui** : Utilisateurs de jeux et de logiciels sur PC, tous profils.
- **Espace** : Launchers de jeux et d'applications (Steam, Epic, GOG, Playnite).
- **Type** : Application desktop.
- **Memorable thing** : « Il sait ce que je veux lancer, instantanément. » (choisie par l'assistant, déléguée par l'utilisateur)

## Aesthetic Direction
- **Direction** : Cinematic Minimal (Brutally Minimal + touches Playful) — fond sombre, une seule scène en grand, tout le reste s'efface.
- **Décoration** : intentionnel — lueurs de couleur derrière la suggestion, pochettes en dégradé, rien d'autre.
- **Mood** : cinéma de salon : sombre, net, un point focal chaud et un accent vif.
- **Références** : aucune recherche web effectuée.

## Typography
- **Display/Hero** : Bricolage Grotesque 800 — caractère ludique et affirmé pour le logo et les titres. Remplace Cabinet Grotesk (non disponible sans téléchargement externe).
- **Body** : Geist 400/500/600 — lisible, neutre, rapide à charger.
- **Data/Tables** : Geist avec `tabular-nums`.
- **Code** : Geist Mono.
- **Loading** : polices embarquées dans l'application (pas de réseau, démarrage plus rapide), via les paquets Fontsource.
- **Scale** : 12 / 14 / 16 / 20 / 24 / 32 / 56 / 64 px.

## Color
- **Approche** : restrained — un accent, la couleur est rare.
- **Primary** : `#3DF5C4` (Tronix) par défaut, au choix du joueur parmi Braise `#FF7A45`, Ciel `#5AA9FF`, Citron `#FFC145` et Rose `#FF5470` — action principale, focus, tags, chiffres des statistiques.
- **Secondary** : `#FF7A45` (Braise) — lueur chaude derrière la suggestion uniquement.
- **Neutrals** : `#F2F4F8` → `#8A90A2` → `#262A38` → `#1A1D28` → `#12141C` → `#0A0B10`.
- **Semantic** : success `#3DF5C4`, warning `#FFC145`, error `#FF5470`, info `#5AA9FF`.
- **Dark mode** : thème sombre unique pour la v1, y compris la barre de titre de la fenêtre.

## Spacing
- **Base** : 4px.
- **Densité** : spacieux.
- **Scale** : 2xs(2) xs(4) sm(8) md(16) lg(24) xl(32) 2xl(48) 3xl(64).

## Layout
- **Approche** : hybrid — un héros pleine largeur, puis une grille disciplinée.
- **Grid** : cartes de 150px minimum, remplissage automatique.
- **Max content width** : fenêtre pleine, contenu fluide.
- **Border radius** : sm 8px, md 14px, lg 22px, full 9999px.

## Motion
- **Approche** : intentionnel — survol des cartes et du bouton Lancer, rien de bloquant au démarrage.
- **Entrée** : la suggestion et les cartes montent en fondu une seule fois au démarrage (désactivé si le système demande moins d'animations).
- **Easing** : enter(ease-out) exit(ease-in) move(ease-in-out).
- **Duration** : micro 100ms, court 150–250ms, moyen 250–400ms.

## Decisions Log
| Date | Décision | Rationale |
|------|----------|-----------|
| 2026-10-26 | Création initiale | /design — direction déléguée par l'utilisateur, objectifs : beau, épuré, rapide |
| 2026-10-26 | Police des titres : Bricolage Grotesque au lieu de Cabinet Grotesk | Disponible sans téléchargement externe, même caractère ludique |
| 2026-10-26 | Barre de titre sombre, icône « t. », animation d'entrée, écran de premier lancement et panneau Paramètres | Finitions demandées par l'utilisateur |
| 2026-10-26 | Couleur d'accentuation au choix, pochettes en image, écran Statistiques | Fonctionnalités de personnalisation et de statistiques décidées avec l'utilisateur |
