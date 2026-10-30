## Problème

Les joueurs et les utilisateurs de logiciels perdent du temps avec les launchers existants. Ces derniers démarrent lentement, affichent trop de boutons et de menus, enferment chaque jeu dans une boutique et sont peu soignés visuellement. Surtout, ils présentent la même grille à tout le monde, sans tenir compte de ce que le joueur veut vraiment lancer à cet instant.

## Solution

Tronix Launcher est un launcher qui démarre vite, qui est beau et épuré, et qui met en avant ce que le joueur a envie de lancer maintenant. Dès l'ouverture, il propose en grand le jeu ou le logiciel le plus probable, lançable en un clic. Dessous, il affiche la bibliothèque classée par probabilité. Il apprend automatiquement des habitudes du joueur, sans effort de sa part. La bibliothèque se remplit par ajout manuel et par import depuis les autres launchers.

## Utilisateur cible

Toute personne qui lance régulièrement des jeux et des logiciels sur un PC Windows : joueurs occasionnels ou passionnés, mais aussi gens qui lancent des outils (création, développement, bureautique). Elle possède souvent plusieurs launchers et veut accéder vite à ce qu'elle utilise, sans configuration.

## User Stories

1. En tant qu'utilisateur, je veux ouvrir le launcher et accéder à l'écran d'accueil en moins de 2 secondes, afin de ne pas attendre avant de jouer.
2. En tant qu'utilisateur, je veux voir en grand, dès l'accueil, le jeu ou logiciel que j'ai le plus probablement envie de lancer, afin de le lancer sans chercher.
3. En tant qu'utilisateur, je veux lancer cette suggestion en un clic, afin de gagner du temps.
4. En tant qu'utilisateur, je veux voir en dessous ma bibliothèque classée par probabilité de lancement, afin de retrouver vite ce que je lance le plus souvent.
5. En tant qu'utilisateur, je veux que le launcher apprenne automatiquement de ce que je lance, afin que ses suggestions s'améliorent sans effort de ma part.
6. En tant qu'utilisateur, je veux ajouter manuellement un jeu ou un logiciel, afin de compléter ma bibliothèque.
7. En tant qu'utilisateur, je veux importer ma bibliothèque depuis Steam, Epic Games Store, GOG Galaxy et d'autres launchers (Xbox, EA, Ubisoft, Battle.net), afin de tout retrouver au même endroit.
8. En tant qu'utilisateur, je veux lancer un jeu importé depuis le launcher, sans ouvrir le launcher d'origine à la main.
9. En tant que nouvel utilisateur avec une bibliothèque vide, je veux qu'on me propose d'ajouter ou d'importer mes jeux, afin de ne pas voir un écran vide.
10. En tant qu'utilisateur, je veux être prévenu clairement si un jeu ne se lance pas, afin de comprendre pourquoi.
11. En tant qu'utilisateur, je veux une interface épurée, belle et ludique, afin que l'utiliser soit agréable.
12. En tant que nouvel utilisateur, je veux choisir au premier lancement les launchers que j'utilise, afin que Tronix ne cherche que ceux-là.
13. En tant qu'utilisateur, je veux activer ou désactiver un launcher dans les paramètres, afin de masquer ceux que je n'utilise pas et d'accélérer l'import.

### Lancement rapide
14. En tant qu'utilisateur, je veux une icône de Tronix près de l'horloge, afin d'y accéder sans ouvrir la fenêtre.
15. En tant qu'utilisateur, je veux qu'un clic sur cette icône affiche les 5 programmes les plus probables, afin d'en lancer un en un clic.
16. En tant qu'utilisateur, je veux un raccourci clavier global qui ouvre une mini-fenêtre au-dessus de tout, afin de lancer un programme sans ouvrir le launcher.
17. En tant qu'utilisateur, je veux que le raccourci par défaut soit Ctrl+Alt+Espace et pouvoir le changer dans les paramètres, afin d'éviter les conflits avec mes autres programmes.
18. En tant qu'utilisateur, je veux être prévenu si le raccourci choisi est invalide ou déjà pris, afin d'en choisir un autre.
19. En tant qu'utilisateur, je veux taper quelques lettres dans la mini-fenêtre pour filtrer la liste classée par probabilité, afin de retrouver vite un programme.
20. En tant qu'utilisateur, je veux lancer le programme sélectionné avec Entrée (le premier par défaut) et fermer la mini-fenêtre avec Échap ou en cliquant ailleurs, afin de rester au clavier.
21. En tant qu'utilisateur, je veux choisir dans les paramètres si la croix de fermeture réduit Tronix près de l'horloge ou le quitte, afin de décider s'il reste actif en arrière-plan.
22. En tant qu'utilisateur, je veux quitter vraiment Tronix depuis l'icône près de l'horloge, afin de le fermer complètement.

### Statistiques
23. En tant qu'utilisateur, je veux voir le nombre de lancements par programme et au total, afin de connaître mes habitudes.
24. En tant qu'utilisateur, je veux voir mes séries de jours consécutifs de jeu, afin de me motiver.
25. En tant qu'utilisateur, je veux un bilan de la semaine (programme le plus lancé, jour le plus actif, nouveautés jouées), afin de revoir ma semaine.
26. En tant qu'utilisateur, je veux voir le temps de jeu, mesuré avec certitude pour les programmes lancés directement par Tronix, afin de connaître le temps passé.
27. En tant qu'utilisateur, je veux voir un temps de jeu estimé, clairement étiqueté « estimé », pour les jeux lancés via Steam, Epic ou GOG, afin d'avoir un ordre de grandeur partout.
28. En tant qu'utilisateur, je veux ouvrir un écran dédié aux statistiques depuis la barre du haut, afin que l'accueil reste épuré.

### Personnalisation visuelle
29. En tant qu'utilisateur, je veux que Tronix utilise les images déjà présentes sur mon PC (Steam, icônes des programmes) comme pochettes, afin d'avoir une bibliothèque plus belle sans Internet.
30. En tant qu'utilisateur, je veux choisir une image de mon PC comme pochette de n'importe quel jeu, afin de personnaliser ma bibliothèque.
31. En tant qu'utilisateur, je veux changer la couleur d'accentuation de Tronix parmi quelques choix, afin d'adapter l'apparence à mes goûts.
32. En tant que nouvel utilisateur, je veux qu'on me propose au premier lancement de télécharger les pochettes, afin de choisir en connaissance de cause.
33. En tant qu'utilisateur, je veux que Tronix télécharge les pochettes manquantes depuis les images publiques de Steam puis, si j'ai renseigné une clé, depuis SteamGridDB, afin de compléter ma bibliothèque.
34. En tant qu'utilisateur, je veux activer ou désactiver le téléchargement des pochettes et renseigner ma clé SteamGridDB dans les paramètres, afin de garder le contrôle de ce qui sort de mon PC.

### Pages des jeux et logiciels
35. En tant qu'utilisateur, je veux ouvrir la page d'un jeu ou d'un logiciel avec un bouton « Détails » placé à côté de « Lancer » et au survol des cartes, afin d'en savoir plus sans perdre la vitesse de lancement (le clic sur une carte continue de lancer).
36. En tant qu'utilisateur, je veux que la page s'affiche sur un écran dédié avec un bouton « Retour », et que « Lancer » reste visible, afin de lire à l'aise et de jouer d'un clic.
37. En tant qu'utilisateur, je veux voir la description et des captures d'écran d'un jeu, afin de me rappeler de quoi il s'agit.
38. En tant qu'utilisateur, je veux voir la fiche technique (genre, éditeur, développeur, date de sortie, plateformes), afin de repérer l'essentiel d'un coup d'œil.
39. En tant qu'utilisateur, je veux voir les notes et avis des joueurs, afin de savoir ce qu'on en pense.
40. En tant qu'utilisateur, je veux voir les dernières actualités et mises à jour d'un jeu Steam, afin de suivre ses nouveautés.
41. En tant qu'utilisateur, je veux voir sur la page d'un logiciel son nom, son icône, mes statistiques d'usage et, quand il existe, un court résumé tiré de Wikipédia, afin d'avoir une page utile même pour un outil.
42. En tant qu'utilisateur, je veux que la page d'un jeu Steam utilise l'API publique de Steam, afin d'avoir des informations exactes sans rien configurer.
43. En tant qu'utilisateur, je veux pouvoir renseigner une clé d'API RAWG dans les paramètres, afin d'avoir aussi des pages pour mes jeux Epic, GOG, Ubisoft, Xbox, EA et Battle.net.
44. En tant qu'utilisateur, je veux voir en français les textes disponibles en français, et en anglais sinon avec une mention de la langue, afin de comprendre ce que je lis.
45. En tant que nouvel utilisateur, je veux qu'on me propose au premier lancement d'activer les « Informations en ligne », désactivées d'office, afin de choisir en connaissance de cause ce qui part sur Internet.
46. En tant qu'utilisateur, je veux activer ou désactiver les « Informations en ligne » dans les paramètres, indépendamment du téléchargement des pochettes, afin de garder le contrôle de ce qui sort de mon PC.
47. En tant qu'utilisateur, je veux qu'une page affiche un message clair avec un bouton d'activation quand les informations en ligne sont désactivées, afin de comprendre pourquoi elle est limitée.
48. En tant qu'utilisateur, je veux que Tronix prépare en arrière-plan, après l'import, le texte de chaque page (fiche, description, notes), afin qu'elle s'ouvre instantanément.
49. En tant qu'utilisateur, je veux que les captures et les actualités se chargent à l'ouverture de la page puis restent en cache, afin de ne pas télécharger des centaines de Mo pour des pages que je n'ouvrirai pas.
50. En tant qu'utilisateur, je veux que les pages restent affichables hors connexion grâce au cache local, afin de les consulter sans Internet.
51. En tant qu'utilisateur, je veux que la page indique le nom de la page trouvée et sa source, et propose « Ce n'est pas le bon ? » avec une liste de résultats (ou « aucun »), afin de corriger une erreur de correspondance.
52. En tant qu'utilisateur, je veux que mon choix de correspondance soit mémorisé, afin de ne pas le refaire.
53. En tant qu'utilisateur, je veux ouvrir un lien d'actualité ou de site officiel dans mon navigateur, afin de lire l'article complet.
54. En tant qu'utilisateur, je veux voir un message clair quand aucune page n'est trouvée ou que le réseau est indisponible, afin de ne pas croire à un bug.

### Modification des informations et choix de pochette
55. En tant qu'utilisateur, je veux modifier le titre d'un jeu ou d'un logiciel, afin de corriger un nom mal détecté ou de le personnaliser ; le nouveau titre s'affiche partout (accueil, cartes, mini-fenêtre, icône près de l'horloge, statistiques, page).
56. En tant qu'utilisateur, je veux modifier la description affichée sur la page d'un jeu ou d'un logiciel, y compris en écrire une quand aucune n'a été trouvée, afin d'avoir le texte que je veux.
57. En tant qu'utilisateur, je veux modifier ces informations même quand les informations en ligne sont désactivées, afin de personnaliser ma bibliothèque sans rien envoyer sur Internet.
58. En tant qu'utilisateur, je veux que mes modifications soient conservées et ne soient pas écrasées par les mises à jour des informations en ligne, afin de ne pas les refaire.
59. En tant qu'utilisateur, je veux rétablir le titre et la description d'origine en un clic, afin de revenir en arrière.
60. En tant qu'utilisateur, je veux chercher des pochettes SteamGridDB depuis le panneau « Image » d'un jeu ou d'un logiciel, afin d'en choisir une parmi de vraies images.
61. En tant qu'utilisateur, je veux voir une grille de pochettes verticales pour le titre du programme et pouvoir chercher un autre nom si ce n'est pas le bon jeu, afin de trouver la bonne image.
62. En tant qu'utilisateur, je veux qu'un clic sur une pochette en fasse l'image du programme (comme une image personnelle, que je peux retirer), afin de choisir ma pochette en un geste.
63. En tant qu'utilisateur, je veux un message clair quand la clé SteamGridDB manque, qu'aucun résultat n'est trouvé ou que le réseau est indisponible, afin de savoir quoi faire.

### Jeux Steam non installés
64. En tant qu'utilisateur, je veux saisir une clé d'API Steam dans les paramètres, afin que Tronix connaisse les jeux que je possède sur mon compte.
65. En tant qu'utilisateur, je veux que Tronix lise mon identifiant Steam dans les fichiers de Steam sur mon PC, afin de ne rien avoir à saisir d'autre ; Tronix ne me demande jamais mon mot de passe Steam.
66. En tant qu'utilisateur, je veux que mes clés d'API (Steam, SteamGridDB, RAWG) soient chiffrées sur mon PC, et que celles déjà enregistrées en clair soient converties automatiquement, afin qu'elles ne soient pas lisibles depuis un autre compte.
67. En tant qu'utilisateur, je veux activer ou désactiver « Afficher mes jeux Steam non installés » dans les paramètres, désactivé d'office, afin de décider si mon identifiant Steam et ma clé sont envoyés à Steam.
68. En tant qu'utilisateur, je veux que la liste de mes jeux Steam soit actualisée à chaque import et gardée sur mon PC, jamais au démarrage, afin de la consulter hors connexion sans ralentir le lancement.
69. En tant qu'utilisateur, je veux voir mes jeux non installés dans une section « Non installés (N) » sous la bibliothèque, repliable et pliée par défaut, avec des pochettes grisées, afin de ne pas alourdir l'accueil.
70. En tant qu'utilisateur, je veux qu'un clic sur un jeu grisé ouvre sa page, où « Installer » remplace « Lancer » et ouvre la fenêtre d'installation de Steam, afin d'installer un jeu sans clic accidentel.
71. En tant qu'utilisateur, je veux que les jeux non installés soient ignorés par la suggestion de l'accueil, la mini-fenêtre, l'icône près de l'horloge et les statistiques, afin que seuls les jeux lançables soient proposés.
72. En tant qu'utilisateur, je veux qu'un jeu installé passe tout seul dans la bibliothèque normale à l'import suivant, et qu'un jeu désinstallé passe dans « Non installés », afin que les sections restent justes.
73. En tant qu'utilisateur, je veux un message clair quand la clé est refusée, que mon profil Steam est privé, que Steam est introuvable sur le PC ou que le réseau est indisponible, afin de savoir quoi corriger.

### Personnalisation du launcher (3 lots : apparence, organisation, collections)
Lot 1 — Apparence
74. En tant qu'utilisateur, je veux choisir une couleur d'accent libre avec un sélecteur de couleur, en plus des 5 couleurs proposées, afin d'avoir exactement la teinte que j'aime.
75. En tant qu'utilisateur, je veux choisir le thème clair, sombre ou automatique (qui suit Windows), sombre d'office, afin que le launcher s'adapte à ma lumière ambiante.
76. En tant qu'utilisateur, je veux afficher la pochette du jeu suggéré, très floue et assombrie, en fond de l'accueil (réglage désactivé d'office), afin que l'ambiance change avec ma suggestion.
77. En tant qu'utilisateur, je veux régler la taille des pochettes (petite, moyenne, grande), afin de voir plus de jeux ou de plus gros visuels.
78. En tant qu'utilisateur, je veux passer de la grille à une vue liste (une ligne par jeu avec temps de jeu et dernier lancement), afin d'avoir une bibliothèque plus compacte.
79. En tant qu'utilisateur, je veux masquer la grande suggestion de l'accueil, afin de ne voir que la bibliothèque.
80. En tant qu'utilisateur, je veux choisir l'arrondi des coins (carré, arrondi, très arrondi), afin d'adapter le style.
81. En tant qu'utilisateur, je veux que tous ces choix s'appliquent tout de suite, soient retenus au prochain lancement, et que « Rétablir l'apparence par défaut » les remette à zéro, afin d'essayer sans risque.
Lot 2 — Organisation
82. En tant qu'utilisateur, je veux choisir le tri de la bibliothèque (probabilité, nom A-Z, récemment lancé, temps de jeu, date d'ajout), afin de retrouver mes jeux comme je le souhaite ; la suggestion de l'accueil reste celle du classement par usage.
83. En tant qu'utilisateur, je veux épingler un jeu en favori avec une étoile, afin que mes favoris apparaissent en premier dans une section à part.
84. En tant qu'utilisateur, je veux une barre de recherche et un filtre par launcher dans la bibliothèque, afin de retrouver un jeu rapidement.
Lot 3 — Collections
85. En tant qu'utilisateur, je veux créer, renommer et supprimer mes propres collections (ex. « Coop », « Travail »), afin de ranger mes jeux à ma façon.
86. En tant qu'utilisateur, je veux ajouter ou retirer un jeu d'une collection depuis sa carte ou sa page, et filtrer la bibliothèque par collection, afin de n'afficher qu'un groupe de jeux.

### Langues
87. En tant qu'utilisateur, je veux que Tronix s'affiche dans ma langue (français ou anglais), choisie au premier lancement d'après la langue de Windows (français si Windows est en français, anglais sinon), afin de comprendre l'interface sans réglage.
88. En tant qu'utilisateur, je veux changer de langue dans Paramètres (Automatique, Français, English), avec effet immédiat et sans redémarrer, afin d'essayer ou de corriger le choix automatique.
89. En tant qu'utilisateur, je veux que tous les textes de l'application suivent la langue : fenêtre principale, mini-fenêtre de lancement rapide, menu de l'icône près de l'horloge et messages d'erreur, afin de ne jamais voir deux langues mélangées.
90. En tant qu'utilisateur, je veux que les dates, les nombres, les durées, les jours de la semaine et les pluriels respectent la langue choisie (« 1 game », « 2 games » ; « 1 jeu », « 2 jeux »), afin que l'affichage paraisse naturel.
91. En tant qu'utilisateur, je veux que les descriptions et fiches venues de Steam et de Wikipédia soient demandées dans la langue choisie, et que les pages déjà gardées dans l'autre langue soient rafraîchies à leur prochaine ouverture, afin que les pages suivent l'interface.
92. En tant qu'utilisateur, je veux qu'une étiquette m'indique quand le texte d'une page n'est pas dans la langue de l'interface (« texte en anglais », « text in French »), afin de savoir pourquoi la page n'est pas traduite.
93. En tant que contributeur, je veux que les textes soient dans un fichier par langue (`src/locales/<langue>.json`, clés à plat, pluriels `_one` / `_other`, variables `{{nom}}`, format i18next lisible par Weblate), afin d'ajouter ou de corriger une langue sans toucher au code.
94. En tant que contributeur, je veux un test automatique qui vérifie que chaque langue a les mêmes clés et les mêmes variables que le français, et que toutes les clés utilisées par le code existent, afin qu'un texte oublié soit détecté avant la publication.
95. En tant qu'utilisateur, je veux qu'un texte manquant dans une langue s'affiche en anglais plutôt que de casser l'écran, afin qu'une traduction incomplète reste utilisable.

## Critères de succès

- Le launcher affiche un écran d'accueil utilisable en moins de 2 secondes après le clic sur son icône, sur un PC Windows courant.
- La suggestion principale se lance en un seul clic depuis l'écran d'accueil.
- Après plusieurs lancements d'un même programme, il remonte en tête de la suggestion et du classement.
- Les jeux installés de chaque launcher pris en charge apparaissent dans la bibliothèque après import, et se lancent depuis Tronix.
- Un programme ajouté manuellement apparaît dans la bibliothèque et se lance.

## Hors périmètre

- Le site web de présentation.
- Le shop et le téléchargement de jeux et logiciels depuis le launcher.
- Les versions macOS et Linux (prévues après la première version).
- Toute idée de différenciation autre que la suggestion automatique, tant qu'elle n'a pas été validée.
- Les suggestions fondées sur le temps disponible, l'humeur ou les jeux délaissés.
- Le démarrage automatique de Tronix avec Windows.
- Toute fonctionnalité non listée dans ce document.

## Décisions d'implémentation

- La première version fonctionne sur Windows uniquement.
- L'écran d'accueil montre une suggestion en grand, puis la bibliothèque classée par probabilité.
- Le classement s'appuie sur la récence, la fréquence et le moment de la journée où le joueur lance chaque programme.
- Les données d'usage restent sur l'ordinateur du joueur.
- Les launchers pris en charge à l'import sont Steam, Epic Games Store, GOG Galaxy, Xbox, EA, Ubisoft et Battle.net.
- Avec une bibliothèque vide, l'accueil propose d'ajouter ou d'importer.
- Au premier lancement, un écran demande quels launchers l'utilisateur emploie ; ceux détectés sur le PC sont pré-cochés. Les jeux des launchers cochés sont importés aussitôt.
- Les paramètres permettent d'activer ou désactiver chaque launcher. Un launcher désactivé n'est plus cherché et ses jeux sont masqués (ils réapparaissent à la réactivation).

- Le raccourci par défaut est Ctrl+Alt+Espace ; la mini-fenêtre montre un champ de recherche et la liste classée par probabilité.
- L'icône près de l'horloge montre les 5 programmes les plus probables, puis « Ouvrir Tronix » et « Quitter ».
- La croix de fermeture réduit Tronix près de l'horloge par défaut ; un réglage permet de le quitter à la place.
- Les statistiques ont leur propre écran, ouvert depuis la barre du haut.
- Le temps de jeu est étiqueté « estimé » quand il n'est pas mesuré directement.
- Le téléchargement des pochettes est proposé au premier lancement et réglable ensuite dans les paramètres.
- Les pochettes viennent d'abord de l'image choisie par le joueur, puis des images déjà présentes sur le PC (Steam), puis du téléchargement, puis de l'icône du programme ; à défaut, une lettre sur fond coloré.
- Cinq couleurs d'accentuation sont proposées (Tronix, Braise, Ciel, Citron, Rose) ; la couleur choisie s'applique à toutes les fenêtres.
- Le temps de jeu est mesuré avec précision pour les programmes lancés directement par Tronix. Pour les jeux lancés via un autre launcher, il est estimé en repérant le jeu en cours d'exécution, avec une erreur d'environ 5 secondes ; il est alors étiqueté « estimé ».
- Le bilan de la semaine porte sur les 7 derniers jours ; la série de jours reste en cours tant que le joueur a joué aujourd'hui ou hier.
## Notes complémentaires

- Le shop et le site web sont des évolutions futures.
- Le support de macOS et Linux vient après la première version.
- Le nombre de launchers à importer dès la première version est un risque de planning.
- Les idées de différenciation autres que la suggestion automatique restent à définir.
- Ordre de réalisation décidé : lancement rapide, puis personnalisation visuelle ou statistiques.
- Le temps de jeu estimé et le téléchargement de pochettes sont les parties les plus incertaines.
- Le temps de jeu estimé n'est pas enregistré si Tronix est fermé pendant la partie.
- Les pochettes téléchargées s'appuient sur des services externes (images publiques de Steam, SteamGridDB) dont la disponibilité n'est pas garantie.
- Les imports Xbox, EA et Battle.net reposent sur les fichiers et le registre que ces launchers laissent sur le PC ; ils n'ont pas pu être vérifiés avec de vrais jeux et peuvent manquer certains titres.
- Les pages dépendent de services externes (Steam, RAWG, Wikipédia) dont les formats et la disponibilité peuvent changer ; RAWG et Wikipédia peuvent se tromper de page, d'où la correction possible.
