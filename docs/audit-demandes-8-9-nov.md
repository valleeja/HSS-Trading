# Audit des demandes de Jee (8 et 9 nov. 2026) contre la branche cell-1

Fait le 9 nov. 2026, à partir du code (git log, recherche dans le code, tests). Aucun statut n'est donné de mémoire.

Légende : ✅ fait (commit) · ⏳ en partie, ou fait sans vérification sur l'appareil · ❌ pas fait · ❓ non vérifiable sans plus de détails (capture ou geste à refaire).

| N° | Demande | Statut | Où (commit, fichier) |
|---|---|---|---|
| 1 | Le panneau des modules ne fonctionne pas | ❓ | Non reproduit dans les tests ; il faut une capture du geste exact |
| 2 | L'app web se synchronise avec la dernière session | ✅ | e1d3f37, 02b60d7, cf1cf49 (synchro en direct 1/5 à 5/5), c544abd |
| 3 | Unité de temps à côté du symbole (pilule BTC-USDC · 4h) | ✅ | 6161c51 (PC) ; d199801 (cell : pilule symbole · unité) |
| 4 | Barre des favoris dans les autres cases en multi-chart | ❌ | La barre flottante n'existe que sur la chart principale (toolbar.rs) |
| 5 | Molette des symboles et des unités de temps (maquette cell) | ✅ | maquette-3 : symbole et unité d'avant et d'après en petit, toucher en haut ou en bas pour passer (test cell_molette_des_unites) |
| 6 | Menu ARGOS : seulement l'essentiel + infos du compte | ✅ | maquette-3 : carte du compte (connexion, démo, solde, P&L, 🛡, wallet) + 9 lignes ; ⏳ Bientôt regroupe le reste (test menu_argos_epure) |
| 7 | Barres plus transparentes quand on utilise la chart, visibles au survol | ✅ | PC (maquette-3) : rangée du haut et barre de gauche estompées à 20 % sur la chart, pleines près de la souris (réglage ☰ Appli ▸ Estomper les barres) ; barre flottante cachée loin de la souris ; cell : barres cachées et montrées au toucher |
| 8 | Barre des favoris déplaçable et repliable au minimum | ✅ | toolbar.rs (poignée ⋮⋮, `retracted`) |
| 9 | Indicateurs actifs dans le menu de la petite flèche, pas sous la pastille | ✅ | indleg.rs (« ⌄ N » : la liste s'ouvre sous la flèche) |
| 10 | Bouton Argos AI en bas à droite, pastille de verre, logo | ✅ | aibtn.rs ; sur cell aussi : d199801 |
| 11 | Barre de droite tout à droite, toujours visible, configurable ; accès à l'échelle de prix | ✅ | watchlist.rs (barre d'icônes, ordre, masquer) ; barre de la maquette : commit en cours |
| 12 | UTC dans la barre du bas, menu par la flèche à côté de UTC-5, sessions qui se chevauchent | ✅ | ListKind::Timezone (horloge en bas), sessions.rs ; ae59e71 (New York par défaut) |
| 13 | Réglages : sous-catégories repliées | ❓ | À revoir écran par écran avec la maquette (chantier maquette-3) |
| 14 | Outils de dessin : une image pour chacun, comme TradingView | ✅ | toolbar.rs (fenêtre des outils en cases avec icônes), rail.rs |
| 15 | Menu Journal condensé | ❓ | journal.rs : à comparer à la maquette (pc-25) |
| 16 | Le menu ARGOS s'ouvre sous le bouton ARGOS | ✅ | topmenu.rs `ArgosMenu::new(anchor…)` |
| 17 | Enlever le bouton AI du menu du haut | ✅ | topbar.rs : plus de bouton AI dans la rangée (efde0b9) |
| 18 | Actions rapides en bas de la chart (Zen, Replay) qui apparaissent et disparaissent | ✅ | navbar.rs (bande du bas, Zen, ⏪) |
| 19 | Flèche des indicateurs en dessous ; « 5 indicateurs » en petit sur la chart | ✅ | indleg.rs (bouton « ⌄ 5 » sous la légende) |
| 20 | Enlever « + ajouter aux favoris » ; flèche repliable qui change de sens | ❓ | À vérifier sur la capture d'origine |
| 21 | Tout a grossi : revenir à la taille normale | ✅ | formfactor.rs : agrandissement seulement au doigt hors téléphone |
| 22 | Raccourcis clavier (zoom) dans la page web | ✅ | cf680a2 (les raccourcis du navigateur passent) |
| 23 | Logo du bouton Trading plus parlant | ✅ | Commit en cours (icône courbe montante au lieu de la manette) |
| 24 | Réordonner la barre de droite par glisser | ✅ | watchlist.rs `move_tool` (glisser = réordonner) |
| 25 | Toutes les boîtes de menu déplaçables et redimensionnables | ❌ | Seules les fenêtres de réglages d'indicateur et de dessin se déplacent |
| 26 | Enlever « Toutes mes mises en page (Maj + T) » | ✅ | cell-1 : ligne retirée de topmenu.rs (test plus_de_toutes_mes_mises_en_page) |
| 27 | « Module WASM / installer kit » devient Argos Script | ❓ | Plus de texte « installer kit » dans le code ; à vérifier sur l'écran visé |
| 28 | Académie et Marketplace dans la barre de droite | ✅ | Commit en cours (barre de la maquette) |
| 29 | « Source d'exchange » pas à cette place | ❓ | À vérifier sur la capture d'origine |
| 30 | Menu des types de bougies comme avant, avec les images | ✅ | toolbar.rs PopKind::Style (cases avec icônes) ; 📊 en haut (efde0b9) |
| 31 | Boutons fonctionnels (mêmes actions) | ✅ | Tests cell_chaque_action_a_un_acces, barre_droite_chaque_bouton_ouvre |
| 32 | Remettre « ma barre » comme elle était | ❓ | Laquelle ? À préciser avec Jee |
| 33 | Bulle d'info : flèche à droite, petite bulle en bas | ❓ | À vérifier sur la capture d'origine |
| 34 | Barre de droite qui se cache comme celle de gauche ; Argos Script dedans | ✅ | watchlist `folded` (languettes ‹ ›) ; Argos Script : commit en cours |
| 35 | Barres de défilement : pilule transparente aux couleurs du thème | ✅ | aacf685 (une seule barre, dans les jetons) ; maquette corrigée |
| 36 | Modules sous les indicateurs, défilement, déplaçables | ⏳ | dock.rs (colonne de modules en bas) ; déplacement entre panneaux : à vérifier |
| 37 | « Zone de liquidité » à sa place ; volume des bougies au survol | ❓ | Légende (Vol, acheteurs, vendeurs) au survol : oui ; la zone : à préciser |
| 38 | Défaire / refaire aussi en bas, avec les raccourcis | ✅ | cell-1 : ↶ ↷ dans la pilule du bas (navbar.rs) ; raccourcis Ctrl+Z et Ctrl+Y (8d61d81) |
| 39 | Page d'Accueil au départ | ✅ | f033dce ; réglage « Accueil au lancement » (☰ Appli, d199801) |
| 40 | Barre « Demande à Argos AI » en bas du volet | ✅ | aa67834, test 3efe076 ; maquette corrigée (volet en flex) |
| 41 | New York par défaut | ✅ | ae59e71 |
| 42 | La recherche Ctrl+K fonctionne | ✅ | 095d5a2 |
| 43 | Les bulles des baleines suivent les boutons qui les activent | ✅ | 0ef43b4 |
| 44 | Le vieux thème s'affichait | ✅ | 55a9f00 |
| 45 | Police Inter comme la maquette | ✅ | f30557f |
| 46 | Ctrl+Maj+R rafraîchit | ✅ | cf680a2, 9386d21 |
| 47 | P&L en vert ou rouge, bulle DÉMO aussi | ✅ | 54338e8 |
| 48 | « Confirmé » devient « Trader » (app et maquette) | ✅ | 9188df1 ; maquette corrigée le 9 nov. |
| 49 | Positions sur la chart comme TradingView, bulle rouge si inversé, achat qui part, démo automatique | ✅ | 0126938, 0648dcc (fusion demo-2 : 27c0810) |
| 50 | Bulle DÉMO dans la barre du bas sur tablette et téléphone | ✅ | 5245c9a |
| 51 | Relais des nouvelles et du calendrier éco | ✅ | 9a2c105 |
| 52 | Rafraîchir ne ramène plus à l'Accueil | ✅ | 7208f9b |
| 53 | Lignes bleues enlevées, ligne horizontale plus pâle | ✅ | 2d26e68 |
| 54 | Trading démo : le P&L et le SL bougent | ✅ | Fusion demo-2 (27c0810) |
| 55 | Cell : rien ne s'entremêle en haut | ✅ | 7008540 (test sans chevauchement de 320 à 1600 px) |
| 56 | Cell : démarre sur la chart | ✅ | 7008540 |
| 57 | Cell et tablette : copie de la vue téléphone de la maquette | ⏳ | d199801, aacf685 et molette (maquette-3) ; à vérifier sur l'iPhone |
| 58 | Pilule − + ‹ › ↺ tout en bas, PC et cell | ✅ | d199801 (test avec 0, 1 et 5 panneaux) |
| 59 | Cell : chargement rapide | ⏳ | d3cd8b5, 405bd1b ; pas de mesure sur l'iPhone |
| 60 | Cell : les boutons de réglages des indicateurs répondent | ⏳ | d3cd8b5 (tolérance au doigt) ; non reproduit, à vérifier sur l'iPhone |
| 61 | Cell : barres du haut et du bas cachées, un toucher les montre ou les cache | ✅ | d199801 |
| 62 | Cell : ⌂ Accueil et réglage « Accueil au lancement » | ✅ | d199801 |
| 63 | PC : copie exacte de la maquette, écran par écran | ⏳ | Rangées du haut, Argos AI, Accueil, barres : verre-2 ; le reste : chantier maquette-3 (voir plus bas) |
| 64 | Barre de droite : ordre par défaut = capture de Jee | ✅ | Commit en cours |
| 65 | Thème Clair + verre propre (PC et cell) ; carte « Verre clair » de la maquette qui marche | ✅ | Maquette : la carte et Réglages ▸ Thème basculent en clair ; app : Argos AI lisible en clair (5702377, test argos_ai_lisible_en_clair), barre du bas du cell vérifiée en capture |
| 66 | Maquette : barre flottante qui ne se cache pas | ✅ | Maquette : disparaît loin de la souris ; app : barre_outils_proche (cell-1, test) |
| 67 | Maquette : voir la vue cell et tablette | ✅ | Maquette : boutons 💻 PC · 📱 Cell · 📟 Tablette (cadre 390 × 844, 820 × 1180) |

## Ce qui reste (❌ et ⏳)

- **Fait dans cell-1 :** 26, 38, 64, 66 et 67.
- **Il reste dans cell-1 :** 65 (vérifier le verre en thème clair dans l'app).
- **Branche à part (gros travail) :**
  - 5 (molette de symboles) ;
  - 6 (menu ARGOS épuré) ;
  - 7 (barres estompées sur PC) ;
  - 25 (toutes les fenêtres déplaçables et redimensionnables) ;
  - 4 (barre des favoris dans chaque case du split) ;
  - 63 (maquette écran par écran).
- **À vérifier sur l'iPhone :** 57, 59 et 60.
- **À préciser avec Jee (une capture suffit) :** 1, 13, 15, 20, 27, 29, 32, 33 et 37.
