# Plan interface-2 : nouvelle couche écran, même moteur

Décision de Jee (9 nov. 2026, 03h06) : la maquette (`croquis/maquette-verre/index.html`) est la source de vérité
(structure, tailles, rayons, couleurs, icônes, libellés, ordre). Branche `interface-2` ; `travail-local` garde la
version de secours (/app/). Jee teste interface-2 sur **/app2/** (même serveur, port 8000).

1. **Gardé tel quel (le moteur)** : argos-core, argos-data, Argos Script, dessin des bougies et indicateurs
   (argos-render : chart, indicateurs, dessins), trading démo, Gardien, persistance (réglages, synchro), tests.
2. **Refait (la couche écran)** : un module `ecran` (argos-render) qui dessine chaque écran de la maquette à partir
   d'une table de zones (une seule table pour le dessin ET le toucher), et un aiguillage unique des touchers dans
   argos-desktop. Chaque zone mène à une action réelle du moteur (menu, volet, feuille, page).
3. **Ordre (cell d'abord)** : ① écran cell principal (chart, rangée du haut en pilule, barre de dessin, DÉMO,
   pilule symbole/unité + molette, ƒx, Argos AI, dock 7 icônes + ⚙, poignées) ; ② feuilles du bas (watchlist,
   Trading, indicateurs ƒx + ◎ réglages, ⚙ réglages, alertes 🔔, ▦ types, 🔎, 🗺, ARGOS ⌄) ; ③ pages (Accueil,
   Académie, Marketplace, PropFirm, Modules, Script) ; ④ tablette ; ⑤ PC écran par écran (pc-01 → pc-36).
4. **L'ancien code part** : quand un écran neuf remplace un ancien (barre, volet, menu, bascule, dock), l'ancien
   code et ses tests sont supprimés dans le même commit ; rien n'est laissé à côté.
5. **Chaque écran** : construit → capture 390×844 (et 1440×900 pour le PC) à côté de la maquette → test tactile sur
   la vraie page web (PointerEvent « touch » + `window.argosEtat`) et test Rust « chaque toucher ouvre quelque chose
   de non vide » → écart zéro ou « donnée inexistante » noté → `make vert` → commit → build dans dist/app2/.
6. **Fin** : interface-2 se fusionne dans travail-local seulement quand tout marche (parité des fonctions + maquette),
   avec l'OK de Jee. Compilation toujours sous le verrou (jobs 10).

## État réel (9 nov. 2026, 06h55 — /app2/ 202611090651, commit 3015c03)

Fait et vérifié (capture côte à côte dans `croquis/references/captures/`, test Rust, essai au doigt en `?essai=1`) :
- ① Écran cell principal (`page1-cell.png`) : zones mesurées (`ecrans/maquette-cell.json`), Argos AI au coin partout
  (choix B), plus de ⚙ du coin des échelles, légende des indicateurs (noms, barre 👁 ⚙ 🗑 ⋯, menu ⋯ Cacher /
  Afficher, Retirer, Arborescence).
- Watchlist (`watchlist-cell.png`) et roue symbole / unité (`roue-cell.png`, modèle : la vidéo de Jee).
- ④ Tablette page 1 (`page1-tablette.png`) : même écran que le cell, agrandi ; test zone par zone contre
  `ecrans/maquette-tablette.json` ; 👥 🌐 🛒 dans le dock.
- ② Feuille Trading du cell (`trading-cell.png`) : `panels/cell.rs`, mesures dans
  `panels/maquette-trading-cell.txt` ; 70 % de l'écran, en verre sur la chart entière.
- Corrigés en route : bug « feuille vide » (réglé), premier toucher perdu après une pause (heure du doigt notée par
  la page), surlignage qui restait après un toucher, menu ⋯ plus large que l'écran, défilement des feuilles du bas.

En attente : lot de remarques de Jee sur page 1 + Trading avant les feuilles suivantes (ƒx, 🔔, ▦, 🔎, 🗺, ARGOS).
OK de Jee demandé : télécharger `@twemoji/svg` 15.0.0 pour l'emoji 🎫 du bouton Ticket (🧾 en attendant).

## Les « bientôt » (la maquette les montre, le moteur ne les a pas encore)

- Feuille Trading : ⬓ Ancrer le volet, ⤢ Agrandir, ☀ Routine / « Routine du jour », 📋 Mon plan, avertissement
  R:R sous le minimum du plan (pas de plan de trading dans le moteur).
- Menu ⋯ d'un indicateur : Ajouter un indicateur / une stratégie à…, Ordre de visualisation, Visibilité des
  intervalles, Épingler à l'échelle, Copier.
- 🌐 Macro (Agenda) : « Calendrier économique : bientôt » quand la source manque (serveur local sans `/api`).
- Profils de la Communauté (salons, défis, devoirs, coaching, Mentor) : voir plus bas.

Écarts acceptés : la pilule ⋮⋮ des favoris est ~30 pt plus à gauche sur tablette (échelle de prix plus large) ; la
maquette n'a pas d'écran pour la roue ; « ⋯ Toutes les options » montre encore l'ancien ticket complet.

Profils (question de Jee, 9 nov. 03h10) : aujourd'hui le profil (Débutant, Bootcamp, Trader, Mentor) est gardé et
ordonne seulement les cartes de l'Accueil. Ce que chaque profil apporte dans la maquette (salons, défis, devoirs,
coaching, lives Bootcamp, « Mes étudiants » et correction pour Mentor) passe par la Communauté, pas encore codée
(« bientôt ») : étape à part, après les écrans, car il lui faut un serveur (salons, membres).

Superposer le code à la maquette (Jee, 9 nov. 03h25) : les zones ne sont plus recopiées à la main. Un script
versionné (`tools/maquette-mesure.js`) est lancé dans le navigateur sur /maquette/ (390 × 844 et 1440 × 900, vue
par vue) ; il relève pour chaque élément de la maquette sa boîte (getBoundingClientRect), son rayon, ses couleurs,
son libellé et son icône, et écrit `rust/crates/argos-render/src/ecrans/maquette-{cell,pc}.json` (versionnés).
Le Rust lit ces fichiers pour dessiner ET pour le hit-test ; un test échoue si une zone de la maquette n'a pas
d'équivalent dans l'app (ou l'inverse). Le CSS de la maquette a besoin d'un navigateur : d'où la mesure plutôt qu'un
analyseur Python.
