# Argos HSS Trading — architecture en couches (Rust) 🧱

## En bref (mis à jour le 2026-11-16)

**Une seule app, deux cibles** : `argos-desktop` est compilée pour l'ordi (fenêtre winit) et pour le navigateur / cell
(WASM, page `app/`). Le code est le même ; ce qui change (fichiers, téléchargements, partage, Worker des modules) est
rangé dans `argos-desktop/src/platform.rs`.

| Crate | Rôle en une ligne |
|---|---|
| `argos-core` | Types communs (bougies, unités de temps, comptes, portfolio, backtest) et **le type d'erreur** `ArgosError` |
| `argos-data` | Sources de prix (exchanges, OANDA, DEX, Yahoo…), direct, liste blanche réseau (`net.rs`), symboles calculés (`synth`) |
| `argos-script` | Calcul des indicateurs (`ta`, `indicators/*`) et langage Argos Script (compatible Pine v6) |
| `argos-render` | Dessin en pixels : chart (`lib.rs` + `theme`, `style`, `series`, `timefmt`, `scale`), `indicators/` par famille, interface |
| `argos-store` | Base locale redb du desktop (Rust pur) (catalogue des symboles, watchlists) |
| `argos-propfirm` | Règles PropFirm neutres (profils, gardien, risque) |
| `argos-scanner` | Scanner de structure (calcul pur) |
| `argos-academy` | Académie : leçons Markdown embarquées, parcours, progression, quiz, recherche, horaires (logique pure, sans dessin) |
| `argos-lens` | Argos Lens : lecture du marché (régime, scores, setup) en paquets `argos-packet/v1` chaînés ; calcul pur |
| `argos-module-host` (+ `vendor/kit-module-argos-1.0.2`) | Modules WASM du kit Argos (interface v2) : manifeste `argos-module.json`, droits refusés par défaut, aucun réseau, bulle du kit sur le desktop, Web Worker dans le navigateur |
| `argos-desktop` | L'app : état (`state/`), actions (`actions/`), clavier / souris / toucher (`input/`), plateforme |
| `argos-serveur` | Serveur local (`make web`, port 8000) : celui d'argos-outils, plus le journal chiffré servi au cell |

Hors de l'espace de travail (chacun son `Cargo.toml`) : `tools/argos-outils` (outils en ligne de commande, serveur
générique, tables générées ; aucune crate d'Argos, licence MIT ou Apache-2.0), `argos-tauri` (coque Tauri 2),
`modules/*` (modules WASM d'exemple) et `vendor/` (copie figée du kit, jamais modifiée).

**Qui utilise qui** (une flèche = « utilise ») : `data`, `script`, `propfirm` → `core` ; `store` → `core`, `data` ;
`lens` → `core`, `script` ; `render` → `core`, `data` (types seulement, sans réseau), `script`, `lens`, `propfirm`,
`academy` ; `desktop` → tout ce qui précède + `module-host`, `scanner`, `store` ; `serveur` → `core`, `data`, `store`,
`argos-outils`.

**Le chemin d'une bougie** : `argos-data` la télécharge (superviseur : réessais et secours) → l'app la garde
(`chart.raw`, cache disque) → `argos-script` calcule les indicateurs quand les bougies changent → `argos-render` dessine
chaque image → la fenêtre (ou le canvas du navigateur) affiche l'image.

**Règles du code**
- Une couche n'utilise que celles du dessous ; le compilateur le garantit.
- Erreurs : `argos_core::Result` / `ArgosError` dans `core` et `data` ; un message pour l'utilisateur passe par
  `ArgosError::Other` et reste mot pour mot (conversion automatique vers / depuis `String`).
- Toute nouvelle adresse réseau va dans `argos-data/src/net.rs` (HTTPS / WSS seulement).
- Aucune clé dans le code, les modules ou le navigateur ; le jeton OANDA et les clés d'exchanges restent chez l'utilisateur.
- `make vert` avant chaque commit : formatage, clippy (desktop + navigateur), tests, audit, licences.
- Tests : à côté du code (`mod tests`) et dans `tests/` (valeurs de référence des indicateurs, comparaisons au site,
  conformité du bac à sable des modules).

---

## Détail des couches

Chaque couche est une crate Rust. **Une couche utilise seulement les couches en dessous.** Modifier une section ne touche jamais les autres : le compilateur le garantit. Aucune fonction réemballée.

```
Layer 4 · Prop firm        argos-propfirm   (neutre, profils standard par défaut, modifiables)
Layer 3 · Marketplace      argos-market     (prévu, pas encore de crate : Nostr lié au compte, ⭐ /5, code source ouvert obligatoire)
Layer 2B · Argos Script    argos-script     (indicateurs par défaut + propositions)
   └─ sous-couche Pine     argos-script::pine (transcription Pine → Argos Script)
Layer 2A · API chart       argos-data       (Binance, OKX, Bybit, Bitstamp, Hyperliquid, WEEX, Bitunix, BloFin)
   └─ base locale          argos-store      (redb : catalogue des symboles + watchlists)
Layer 1 · Core             argos-core       (compte, structure, sécurité, sandbox, backtest, portfolio)
```

## Layer 1 · Core (`argos-core`)
| Module | Rôle |
|---|---|
| `types` | Bar, Symbol, Timeframe, Side, erreurs communes, `aggregate` / `bucket_start` (regroupe en unité de temps plus haute, mois inclus) |
| `instrument` | 📇 `Instrument` (un symbole du catalogue), id `exchange:symbole`, `MarketKind` perp / spot |
| `store` | 🗄️ Contrat `SymbolStore` (catalogue, recherche, watchlists) — redb au desktop, autre stockage possible au web |
| `account` | Clé de 24 mots, clé Nostr dérivée, verrouillage de session |
| `security` | Sandbox : wallet en lecture seule, confirmation à l'ouverture, NIP-98, aucune clé privée stockée en clair |
| `feed` | Contrat `trait Feed` (lecture d'API de bougies) que chaque exchange de la couche 2A respecte |
| `backtest` | Rejoue une stratégie sur des bougies → trades, P&L, drawdown |
| `portfolio` | Comptes démo, positions, historique, équité |

## Layer 2A · API chart (`argos-data`) ✅
Un fichier par exchange (`exchanges/*.rs`), chacun avec 4 fonctions pures : `candles_request`, `parse_candles`, `markets_request`, `parse_markets`. `ExchangeFeed` les branche sur un client HTTP injecté et respecte le `trait Feed` du Core.
- Client HTTP : `NativeHttp` (option `native`, desktop/serveur), `FetchHttp` (option `web`, navigateur / cell : `fetch`), faux client (tests).
- 🛡️ Superviseur (`supervisor`) : réessaie l'exchange choisi (0,5 s puis 1,5 s), vérifie les bougies (vides, figées, prix impossibles ; trous = avertissement), bascule sur un exchange de secours (Binance → Bybit → OKX → BloFin → Bitunix → WEEX → Hyperliquid → Bitstamp) et affiche « ⚠️ Source temporaire ». Revient à l'exchange choisi dès qu'il refonctionne.
- Pagination : `candles()` va chercher jusqu'à `DEFAULT_CANDLES` = 5000 bougies, page par page vers le passé (limite de chaque exchange : Hyperliquid 5000, Binance 1500, BloFin 1440, Bybit / Bitstamp 1000, OKX 300 puis 100 par page d'historique, WEEX 300, Bitunix 200), avec une petite pause entre les pages. Si un exchange ignore la pagination, Argos s'arrête au lieu de boucler.
- WEEX et Bitunix bloquent les navigateurs : relais Argos sur le web, direct sur desktop.
- Test réel chez toi : `cargo test -p argos-data --features native --test live -- --ignored --nocapture`
- `live` : **direct** WebSocket sur les 8 exchanges (desktop : tokio ; navigateur : WebSocket de la page) — la dernière bougie est remplacée ou une nouvelle commence, l'historique ne change jamais ; reconnexion automatique (2 s → 30 s).
- 📇 Catalogue (`ExchangeFeed::instruments`) : tous les symboles de chaque exchange, infos fixes sans prix (modèle `argos_core::Instrument`, id `exchange:symbole`). Hyperliquid + dex HIP-3 (growth mode), Bybit page par page ; perps linéaires seulement (Bitstamp : spot).
- `watchlist` : 🚩 marchés Hyperliquid les plus populaires (volume 24 h) — sections Crypto (+ PAXG), HIP-3 (`dex:SYMBOLE`) et HIP-3 Growth mode, rafraîchis en arrière-plan.
- `prices` : 💲 prix (dernier + 24 h) des autres exchanges, une requête par exchange pour tous ses marchés, seulement ceux demandés par la fenêtre (liste affichée + symbole ouvert), aux 5 s.
- `live` : direct des bougies — Hyperliquid, Binance, Bybit, OKX, BloFin (abonnement et lecture dans le fichier de chaque exchange) ; trades (secondes) : Hyperliquid.
- `whales` : 🐋 grosses transactions en direct (trades Hyperliquid avec portefeuille du preneur, aggTrades Binance Futures), 50 k$ et plus ; panneau Baleines (`argos-render::whales`) : seuil, pièce du graphique, pression achat / vente 15 min, alerte à 1 M$.
- `logos` : 🪙 logos CoinMarketCap (table symbole → id), gardés dans `~/.cache/argos/logos/`.
- `cache` : 💾 bougies gardées entre deux lancements (`~/.cache/argos/candles/`) ; au démarrage on télécharge seulement ce qui manque, les fraîches remplacent la dernière du cache.
- `Exchange::ticker` / `chart_title` : ticker complet comme l'exchange l'affiche (« BTCUSDT Perpétuel · 30m · Binance »).
- `rt` : tâches et pauses (desktop : fil + tokio ; navigateur : `spawn_local`), même code partout.
- `claude` : ✨ assistant (API Messages avec ta clé, recherche web).

## Layer 2B · Argos Script (`argos-script`) 🟡 en cours
| Module | État |
|---|---|
| `ta` (sma, ema, rma, wma, rsi, stoch, stdev, atr, adx, pivots, highest, lowest — logique Pine) | ✅ |
| `indicators::volume_candles` (TVD Pro : LOW vide / MID bleu-rose / HIGH vert-rouge / GOLD or-mauve foncé, acheteurs/vendeurs ; LOW et couleurs baissières propres à Argos ; largeur hybride : formule TradingView `1 + log2(ratio)` comparée aux 20 bougies d'avant) | ✅ paliers identiques au site (test sur 400 bougies) |
| `indicators::cipher` (WaveTrend, RSI, Stoch RSI, RSI+MFI, divergences, points, Golden / Purple dot, confluence /4) | ✅ identique au site (test sur 600 bougies) |
| `indicators::ai_trend` (AI Trend Index, signal, divergences 🤑, div MTF, zones extrêmes, boîte HTF, filtre EMA) | ✅ identique au site (test sur 3000 bougies 30m) |
| `indicators::tbo` (trendlines, signalDelta, 11 lissages, filtres ATR / ADX / HTF / volume, Vector 150+ / 200+, divergences, statut) | ✅ identique au site (4 réglages × 3000 bougies) |
| `indicators::smc` (OB · BB · FVG · iFVG, BOS / CHoCH, EQH / EQL + sweeps, UT supérieure, résumé) | ✅ identique au site (3 réglages × 3000 bougies) |
| `indicators::smc_pro` (« Argos SMC » : structure, OB / BB / FVG / iFVG, EQH / EQL + sweeps, sessions & killzones (vrais fuseaux, `chrono-tz`), Premium / Discount, biais multi-UT, tendance de fond, Fibo / OTE, confirmation, plan + score /10) | ✅ identique au site (5 réglages × 6 longueurs = 30 états) |
| `indicators::pivot_vwap` (Argos Pivot VWAP, conception Argos : VWAP ancrés sur les pivots, une échelle ou multi-échelle) | ✅ écrit de zéro (2026-11-06) |
| Langage Argos Script (`lang` : compatible Pine v6, types, méthodes, tableaux, boucles, dessins) | ✅ |
| 🌐 Langues (`lang::dialect`) : Pine v4 / v5 (dont LuxAlgo), ancien Argos Script v1, wrun reconnus et traduits en Pine v6 ; erreurs à la ligne d'origine, messages pour ce qui n'a pas d'équivalent | ✅ (7 modèles v1 + tests v4, v5, wrun) |

## Affichage · `argos-render` (bougies maison, sans TradingView) 🟡 en cours
Dessine la chart dans une image en pixels (`tiny-skia`) : grille, bougies classiques ou bougies de volume Argos, échelle de prix (« 80 660,0 »), échelle de temps (heures / jours), ligne du dernier prix. Le même rendu servira à la fenêtre desktop, au canvas web (WASM) et au cell.
- Polices incluses (libres, usage commercial permis) : Inter (OFL 1.1), DejaVu Sans, Liberation Sans (OFL 1.1), avec repli automatique d'une police à l'autre.
- 14 types (comme TradingView) : bougies de volume Argos (défaut), bougies, creuses, Heikin Ashi (option vrai dernier prix), barres OHLC, barres HLC, ligne, ligne avec marqueurs, escalier, aire, aire HLC, ligne de base, colonnes, haut-bas. Survol : O / H / L / C + volume + acheteurs / vendeurs (estimés) sur tous.
- Table d'actions unique (module `actions`) : nom, raccourci et effet de chaque action, lus par le clavier, la barre de navigation et tous les menus (le raccourci affiché est toujours le vrai).
- Moteur de menus (module `context_menu`) : menus en données, sous-menus qui s'ouvrent à côté au survol (et défilent à la molette s'ils sont longs). Sources : colonne de prix (clic droit ou ⚙), échelle de temps (réinitialiser, fuseau horaire, arrêts de session), graphique (réinitialiser, copier le prix, effacer les dessins), ➕. Fonctions pas encore prêtes : en gris avec la raison.
- Watchlist (module `watchlist`) : barre d'icônes à droite (tous les outils, toujours là), panneau Symbole · Dernier · Chg · Chg% (tri, sections repliables, drapeaux, chiffres qui changent en couleur). Clic = ouvrir le symbole, ↑ / ↓ = précédent / suivant (`WatchPrev` / `WatchNext`).
- Panneaux de droite (module `panels`) : **comme Argos v1** — 📈 Indicateurs, scripts et modèles (Personnel, Intégrés, Communauté), 💼 Trading démo (compte démo 100K, gardien PropFirm, calcul de lot $ → lots / lots → $, ticket démo : aucun ordre réel), 📓 Journal (Trades / Stats / PropFirm, périodes). Une table de zones par panneau, lue par le dessin et le clic. Bouton **AI** au logo A d'Argos (`assets/argos-a-128.png`) dans chaque en-tête.
- Sessions (module `sessions`) : Sydney, Tokyo, Londres, New York écrites dans l'heure de leur ville (changements d'heure justes) ; pastilles des sessions ouvertes + ⚡ Chevauchement en bas à droite, près de l'horloge ; clic = menu Fuseau horaire (fuseaux d'Argos v1 + heures des sessions dans ce fuseau).
- Barre du haut (module `topbar`) : **comme Argos v1**, 3 rangées au-dessus de tout — logo ARGOS (`assets/argos-logo.png`) ▾, mise en page et ses onglets, 🪶 Léger / ✨ Complet, Connect, 🌐, ⚙ ; bandeau des tickers de la watchlist (★ drapeaux seulement, $ / %, ›) ; fiche du symbole (Prix, Oracle, Variation 24 h, Volume 24 h, Intérêt ouvert, Funding avec compte à rebours, source). La fenêtre décale tout le reste de `topbar::HEIGHT`.
- Section Exchanges (module `exchanges`, outil 🔌 de la barre de droite) : chaque exchange avec son interrupteur (catalogue téléchargé et rafraîchi chaque jour, filtre dans le panneau ＋), son état, ⟳, et sa clé. Boîte de clé : champs masqués (Hyperliquid : adresse publique seulement), collage Ctrl + V, valeurs envoyées au trousseau du système (`argos-desktop/src/keys.rs`, crate `keyring`) puis oubliées ; jamais dans un fichier ni affichées en clair.
- Menus de la watchlist (module `wlmenu`, comme Argos v1) : nom de la liste ⌄ = menu des listes (Hyperliquid auto + tes listes avec leur nombre, « Voir : Tout ● ● ● » une rangée de pastilles, Nouvelle liste, Ajouter une section, Renommer, Supprimer en 2 clics) ; ＋ = panneau d'ajout dans la watchlist (recherche, filtres Tout / Hyperliquid / Binance / WEEX / groupes du catalogue, « déjà là » / +) ; ✕ au survol pour retirer un symbole ou une section. Listes et sections dans la base redb ; catalogues Hyperliquid, Binance, WEEX téléchargés en arrière-plan, rafraîchis chaque jour.
- Intervalles (`argos_core::Interval`) : 1 seconde → 12 mois. Ceux que l'exchange ne donne pas sont regroupés à partir du plus grand qui tombe juste dedans (50m = 10 × 5m, 3 mois = jours) ; secondes = trades en direct (`live::fold_trade`).
- Barre flottante (module `toolbar`) : poignée ⋮⋮ (au survol), intervalles ⭐ favoris, 🕐 menu des intervalles, type de chart, fx / ⏰ / outils de dessin (bientôt), ⇅. Menu en liste réutilisable (module `listmenu`) : sections repliables, ⭐, lignes grises.
- Barre de gauche (module `rail`) : **la même qu'Argos v1** — curseur, 6 groupes (Lignes/canaux/fourchettes, Fibonacci et Gann, Figures, Prévisions et mesures, Formes, Annotations) avec sous-menus (sections, Alt + touche, « ? », ⭐), règle, aimant faible / fort, mode continu, verrouiller, masquer, gomme, RR, défaire, supprimer. Catalogue des 93 outils (module `tools`) et icônes d'Argos v1 (module `icons`, SVG dessinés avec resvg).
- Formes partagées (module `shapes`) : rectangle cliquable, rectangle arrondi plein / contour, traits d'icônes.
- Barre de navigation (module `navbar`) : − + ‹ › ↺ (carrés arrondis, icônes au trait comme TradingView) en bas de la chart avec bulle nom + raccourci. Zoom de 0,5 à 250 px par bougie.
- Barre du bas (module `bottombar`) : périodes 1D 5D 1M 3M 6M YTD 1Y Tout (`ShowFrom`), 📅 « Aller à » (module `goto`, Alt + G) : calendrier comme TradingView, onglets Date (`CenterOn`) et Plage personnalisée (`ShowRange`), horloge. Heures dans le futur sur l'échelle de temps.
- Étiquettes / lignes : dernier prix, nom du symbole, clôture du jour précédent, haut et bas visibles, compte à rebours ; arrêts de session ; double-clic sur la chart = mode libre au démarrage ⇄ mode normal (prix auto) ⇄ mode libre.
- Réglages (module `settings`) : `ChartSettings` = source unique de tous les réglages (rendu, menus, Réglages, raccourcis), JSON avec valeurs par défaut pour les vieux fichiers. Desktop (`saved.rs`) : type, zoom, réglages et dessins gardés par symbole dans `~/.config/argos/charts/`, écrits après chaque action (écriture sûre).
- Objets dessinés (module `drawings`) : **le modèle d'Argos v1** — un objet = id de l'outil (table `tools`) + points (temps + prix) + style (couleur, épaisseur, tirets, verrouillé, caché, texte, RR). `Frame` / `Geo` font temps ↔ x (futur compris) et prix ↔ y ; `snap` = aimant faible / fort sur O / H / L / C. Toucher, poignées, déplacer, dessiner écrits une fois ; dessins propres : lignes (horizontale, rayon horizontal, verticale, transversale, tendance, rayon, étendue), rectangle, Fibonacci, position longue / courte (RR de la barre), règle / plages, texte, flèches ; les 69 autres (phase C) : leur forme dans le module `toolgeom` — lignes d'info / angle / régression, canaux, fourchettes, Fibonacci (extension, temps, éventail, cercles, spirale, arcs, coin), Gann, figures (XABCD, Cypher, ETE, ABCD, triangle, trois poussées), vagues d'Elliott, cycles, prévisions, VWAP ancré et profils de volume, formes, annotations. Une forme = traits + zones + étiquettes, lue par le dessin et par le toucher. Placement : N clics avec aperçu, double-clic / Entrée pour les outils à points libres, mode continu ; sélection, poignées, Suppr, Échap. Anciennes sauvegardes `HLine` relues.
- Réglages d'un objet (module `objdlg`) : double-clic sur un dessin → onglets Style (couleur, épaisseur, plein / tirets / points, verrouiller), Texte (texte, taille), Coordonnées (prix de chaque point), Visibilité (unités de temps) ; changements en direct, Annuler remet la copie d'avant. Positions : onglets Paramètres en Entrée (compte, lot, risque % / $, entrée, profit, stop), Style (lignes, couleurs stop / objectif / texte, taille, étiquettes de prix, Stats à cocher, compact, toujours afficher) et Visibilité. Bas : ⋯ (Sauvegarder sous… → `templates.json` par outil, tes modèles, paramètres par défaut), Annuler, D'accord.
- Positions longue / courte (module `position`) : comme TradingView — zones cible / stop, étiquettes Cible / Ouverture P&L + Qté + RR / Stop (13 stats au choix), 4 poignées (entrée, cible, stop, largeur) ; calcul : risque = compte × % (ou $), qté = risque ÷ |entrée − stop| ÷ lot.
- Barre des favoris flottante (module `toolbar`) : copie de celle d'Argos v1 — ⋮⋮ (toucher = fermer / ouvrir, glisser = déplacer), intervalles favoris (barre horizontale), 🕐 grille des unités de temps, type de graphique, fx, ⏰, outils ⭐ (glisser = l'ordre), 7 catégories (fenêtre en petits carrés avec ☆), ⇅ verticale ; clic maintenu = déplacer ; clic droit = menu de l'outil / de la barre d'outils (aussi sur la barre de gauche) ; masquer / afficher.
- 📱 Cell (modules `phonebar`, `topbar` en mode cell) : comme Argos v1 — barre du haut d'une rangée, en bas symbole · UT · ✏️ · ⋯ puis Panneaux ⇅, Market, CMC, Argos AI ; fenêtres (`Sheet`) sur toute la largeur ; ✨ assistant (`assistant`) ; clavier du cell par un champ caché de la page.
- 🎨 Thèmes : Défaut (sombre / clair), Argos (bleu v1), Or, Matrix (`Theme::named`, `palette::light_ui` pour le clair).
- Barre du haut (modules `topbar`, `topmenu`) : Argos ▾ (navigation, compte, gagner, outils), Mises en page ▾ (sauvegarder, sauvegarde auto, renommer, modèles), Connect (Compte Argos : démo), 🌐 format des nombres (4 formats, `viewport::set_number_format`) et langue, ⚙ Réglages (tuiles).
- Clic droit sur un dessin (module `objmenu`, une table) : alerte (bientôt), premier plan / arrière-plan, visibilité des intervalles, cloner, copier (Ctrl + C, Ctrl + V colle), inverser (positions), ordre limite / synchro / arborescence (bientôt), verrouiller, cacher, retirer, configurations.
- Bougies de volume : chaque bougie prend une place selon son volume (`viewport::Slots`) — les grosses poussent leurs voisines au lieu de les chevaucher ; corps avec la même règle d'écart que TradingView.
- Zoom autour de la souris, défilement, réticule (prix, heure, O / H / L / C / V), précision des prix selon le symbole, bandeau d'avertissement du superviseur.
- Fenêtre desktop (`argos-desktop`, winit + softbuffer) : `cargo run -p argos-desktop --release -- hl BTC 30m` (molette = zoom, glisser = défiler, ← → + − Fin, T / Maj+T = changer de type). Hors ligne : `-- bougies.json 30m`.
- Navigateur et cell : même app compilée en WASM (`argos-desktop`, page `app/`), taille et toucher en pixels de la page (écrans ×2, ×3).

## Layer 3 · Marketplace (`argos-market`, prévu)
Compte Nostr créé en même temps que le compte Argos, publication et distribution des indicateurs, notes ⭐ sur 5, **code source ouvert obligatoire**.

## Layer 4 · Prop firm (`argos-propfirm`) — neutre
✅ `rules` construit : `propfirm-defaults.json` (version 2026-10-25), types de perte max (fixe, trailing fin de journée, trailing intraday) et constance (Best Day, Consistency sur le profit total avec date de bascule) + contrôle `check_consistency`.
| Sous-catégorie | Contenu |
|---|---|
| `rules` | 3 profils par défaut (2 étapes, 1 étape, Instantané), modifiables, dupliquables, sauvegardés ; mise à jour auto des défauts (fichier `propfirm-defaults.json` versionné, approuvé par Jee) sans écraser les profils personnalisés |
| `guard` | Gardien : bloque un ordre qui ferait dépasser la perte max par jour / totale (fixe, trailing fin de journée, trailing intraday) |
| `risk` | Calcul de risque : taille de position selon le SL, risque $ / %, R/R, marge, levier, pertes restantes avant la limite |
| `calendar` | Calendrier économique : nouvelles à fort impact (CPI, NFP, FOMC…), fenêtres à éviter autour des annonces |
| `sessions` | Heures de marché, killzones (Asie, Londres, New York), jours fériés |
| `stats` | Progression du challenge : objectif, jours tradés, règle de constance, meilleure journée |
| `journal` | Journal des trades du challenge, lié au portfolio du Core |

## Ordre de construction (backtest à chaque étape)
1. Core → 2. API chart → 3. Argos Script + indicateurs → 4. Pine → 5. Marketplace → 6. Prop firm

## Base locale (`argos-store`) 🗄️
- Un seul fichier redb, propre à Argos HSS Trading : `~/.local/share/argos-hss-trading/argos.redb`. Tables `instruments`, `exchanges`, `meta` (version, watchlists). L'ancien `~/.local/share/argos/argos.db` (SQLite) n'est plus lu ni touché : les listes reviennent par `watchlists.json`, les catalogues se retéléchargent.
- Respecte `argos_core::store::SymbolStore`. Version du schéma dans `user_version` : mise à jour automatique, sans perdre les listes.
- Un symbole retiré par l'exchange reste (inactif) : les listes qui le contiennent ne cassent pas.
