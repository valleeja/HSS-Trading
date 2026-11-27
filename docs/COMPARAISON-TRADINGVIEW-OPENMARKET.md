# Argos HSS Trading, TradingView et OpenMarket

Version 3 · 26 nov. 2026

Ce document compare Argos HSS Trading à deux apps de charts connues, **TradingView** et **OpenMarket**, sur 64 fonctions rangées en 6 parties. La colonne Argos est à jour au 26 nov. 2026. Les colonnes TradingView et OpenMarket viennent de la lecture de leurs sites publics le 23 nov. 2026 ; leurs offres et leurs prix peuvent avoir changé depuis.

## En bref

| | Argos HSS Trading | TradingView | OpenMarket |
|---|---|---|---|
| Note sur les 64 fonctions | **76 / 100** | **69 / 100** | **74 / 100** |
| Avis global (finition, taille, communauté) | environ 65 / 100 | environ 90 / 100 | environ 80 / 100 |
| Prix | gratuit, sans limite, sans pub | gratuit limité avec pub, puis 4 offres | gratuit limité, puis 35 $ ou 120 $ par mois |
| Où vont tes données | elles restent sur ton appareil | sur leurs serveurs | sur leurs serveurs |
| Ordres réels | non, démo seulement (choix du projet) | oui, plus de 100 courtiers | oui, Binance, Bybit, Hyperliquid |

**Comment la note est comptée.** Pour Argos : fini = 1, en cours = ½, prévu ou pas prévu = 0. Pour TradingView et OpenMarket : oui = 1, payant = ¾, partiel = ½, non ou pas indiqué = 0. La note compte seulement des fonctions présentes ou absentes. Elle ne mesure ni la finition, ni la taille de la communauté, ni le nombre de marchés : d'où l'avis global à part, qui est une estimation et pas un calcul (il date du 23 nov. et n'a pas été refait).

**Ce qui a changé depuis la version 2 (23 nov.).** Argos passe de 74 à 76 :
- le panneau **Carnet / Delta / DOM** est fini, sur le même carnet que la carte thermique ;
- les **tableaux des indicateurs** se déplacent et se redimensionnent, à la souris et au doigt ;
- **Argos Script** lit maintenant les types, les méthodes et les matrices du Pine, et 7 scripts de la communauté roulent tels quels ;
- **RSI, MACD, Stochastique, SAR, Ichimoku et Cipher** donnent les mêmes valeurs que TradingView dès la première bougie ;
- la **version tactile** du téléphone et de la tablette, la fluidité et une **porte de qualité** (registre des bugs, plus de 2 100 tests) ;
- les versions **Windows, Mac et Android** sont en préparation sur GitHub.

## Ce qu'Argos fait que les deux autres ne font pas

- **Gratuit, sans limite et sans compte** : tout tourne sur ton appareil, rien n'est envoyé à un serveur d'Argos.
- **Le flux d'ordres crypto en gratuit** : footprint de 10 bourses additionnées, carte des liquidations, bulles de baleines, Argos Murs (les gros ordres du carnet sur la chart).
- **Gardien PropFirm** : perte maximale, risque par trade et verrou pendant les news, sur le compte démo.
- **Comparer un backtest avec TradingView**, trade par trade, côte à côte.
- **Rapport du mois en PDF**, commenté par le rond Argos AI.
- **Les indicateurs Argos** : TVD Pro, Cipher Pro, AI Trend, SMC, Vector Zone, Argos Lens.

## Ce qui manque encore à Argos

- Les **indicateurs classiques** : 56 sont intégrés, il en manque environ 60 face aux plus de 400 de TradingView.
- L'**écran partagé** : seule la première case est une vraie chart, et les dessins ne se synchronisent pas encore.
- **Comparer deux symboles** sur un graphique, le **scanner** complet, les **Options** (Deribit).
- Une **communauté** et une **bibliothèque de scripts** partagés.
- Les **traductions** (63 langues au menu, environ 850 phrases à traduire).
- Choix du projet, pas des oublis : pas d'ordres réels, pas d'alertes quand l'appareil est éteint, pas d'actions ni de futures CME payants.

## Légende

- Argos : ✅ Fini · 🟡 En cours · ⏳ Prévu · ⛔ Pas prévu (choix du projet)
- TradingView et OpenMarket : ✅ Oui · 💲 Payant · 🟡 Partiel · ❌ Non · ❔ Pas indiqué sur leur site

## Les 64 fonctions

### 1. Chart et dessin

La base : le graphique, les bougies, les outils.

| Fonction | Argos HSS Trading | TradingView | OpenMarket |
|---|---|---|---|
| Graphique maison (sans TradingView dedans) | ✅ Fini : Écrit en Rust, même code PC, web, téléphone | ✅ Oui : Leur propre graphique | ✅ Oui : Leur propre graphique, web et appli |
| Types de bougies | ✅ Fini : 17 types, dont Renko et bougies de volume Argos | ✅ Oui : 21 types (17 en gratuit) | ❔ Pas indiqué : Le site ne décrit que les bougies |
| Outils de dessin | ✅ Fini : 109 : Fibonacci, Gann, Elliott, FVG, OB… | ✅ Oui : Plus de 110 | ✅ Oui : 99 outils en 9 groupes |
| Unités de temps libres (2m, 5h, 3 mois…) | ✅ Fini : De 1 seconde à 12 mois ; secondes en direct sur 4 bourses | 💲 Payant : Sur mesure, secondes et ticks en payant | 💲 Payant : Secondes (1 s à 30 s) en payant |
| Écran partagé (splits) | 🟡 En cours : 1 à 16 cases ; seule la 1re est une vraie chart | 💲 Payant : 2 à 16 graphiques en payant | ✅ Oui : Jusqu'à 16, même en gratuit |
| Synchro des splits (réticule, dessins) | 🟡 En cours : Symbole, unité, réticule et temps faits ; dessins à finir | 💲 Payant : Complète, avec les offres payantes | ✅ Oui : Réticule, symbole, unité, temps, dessins |
| Plein écran / mode Zen | ✅ Fini : Double tap au doigt, triple clic PC | ✅ Oui | ❔ Pas indiqué |
| Market Profile / TPO | ✅ Fini : Petits restes | 💲 Payant : Offres payantes | ✅ Oui : TPO et profil de volume, additionnés |
| Paires contre BTC (XRP/BTC…) | ✅ Fini : Avec Bitfinex | ✅ Oui | ❔ Pas indiqué |
| Comparer deux symboles sur un graphique | ⏳ Prévu : Pas commencé | ✅ Oui | ❔ Pas indiqué |

### 2. Indicateurs et Argos Script

Le moteur de scripts compatible Pine, et nos indicateurs.

| Fonction | Argos HSS Trading | TradingView | OpenMarket |
|---|---|---|---|
| Indicateurs intégrés | 🟡 En cours : 56 intégrés ; RSI, MACD, Stochastique, SAR, Ichimoku et Cipher vérifiés contre TradingView ; environ 60 classiques à ajouter | ✅ Oui : Plus de 400 avec les stratégies | ✅ Oui : 80 à 197 selon la page |
| Indicateurs Argos exclusifs | ✅ Fini : TVD Pro, Cipher Pro, AI Trend, SMC, Vector Zone, Lens… | ❌ Non : Seulement via la communauté | ❌ Non : Équivalents SMC par la communauté |
| Argos Murs (murs du carnet sur la chart) | ✅ Fini : Pilules verre, millions en direct | ❌ Non | 🟡 Partiel : Carte thermique du carnet |
| Langage de script | ✅ Fini : Argos Script, compatible Pine v6 | ✅ Oui : Pine v6 (l'original) | ✅ Oui : wrun (style TypeScript, WebAssembly) |
| Importer un script Pine | 🟡 En cours : Pine v6 tel quel, v2 à v5 traduits ; types, méthodes et matrices faits ; 7 scripts de la communauté roulent tels quels ; manque l'import de bibliothèques | ✅ Oui : Natif | ✅ Oui : Conversion Pine dans l'éditeur |
| Éditeur de code avec aide 📖 Doc | ✅ Fini : Dans l'app, PC et téléphone | ✅ Oui : Autocomplétion, versions, profileur | ✅ Oui : Doc, 85 recettes, IA pour écrire |
| Tableaux déplaçables sur la chart | ✅ Fini : Déplaçables et redimensionnables, à la souris et au doigt | 🟡 Partiel : Position fixe dans un coin | ❔ Pas indiqué : Cartes et HUD ; déplacement pas dit |
| Bibliothèque de scripts de la communauté | ⏳ Prévu : Avec la Marketplace | ✅ Oui : 100 000 à 150 000 scripts | ✅ Oui : Scripts partagés, copiables ou vendus |

### 3. Flux d'ordres et marchés

Là où Argos va plus loin que TradingView, surtout en crypto.

| Fonction | Argos HSS Trading | TradingView | OpenMarket |
|---|---|---|---|
| Footprint | ✅ Fini : 10 bourses additionnées (Binance, Bybit, OKX, Coinbase…) | 💲 Payant : Offres payantes, une source | ✅ Oui : Jusqu'à 14 bourses additionnées |
| Trades en direct (Tape) | ✅ Fini | 🟡 Partiel : Liste simple | ✅ Oui : Trades et liquidations de plusieurs bourses |
| Carte des liquidations | ✅ Fini : Gratuit, en direct | ❌ Non | ✅ Oui : Cartes et rafales de liquidations |
| Bulles de baleines | ✅ Fini : De vrais trades | ❌ Non | ✅ Oui : Bulles de volume, gros trades |
| Carte thermique du carnet | 🟡 En cours : Carnet partagé fait ; carte épurée à faire | ❌ Non | 💲 Payant : Réduite en gratuit, complète en payant |
| Panneau Carnet / Delta / DOM | ✅ Fini : Fait le 25 nov., sur le même carnet que la carte thermique | 🟡 Partiel : DOM avec un courtier ou en compte démo | ✅ Oui : Carnet, tape et DOM |
| Funding, Open Interest | ✅ Fini | 🟡 Partiel : Selon la source | ✅ Oui : Additionnés sur plusieurs bourses |
| Hyperliquid : Explorer et Stats | ✅ Fini : Positions sur la chart, stats des marchés | ❌ Non | 💲 Payant : Suivre une adresse en payant ; liquidations et TP/SL |
| Market cap (CoinMarketCap), DEX, sentiment, macro | ✅ Fini | ✅ Oui : Capitalisation, DEX, macro, réseaux sociaux | 🟡 Partiel : Séries éco, mèmecoins Solana, ETF |
| Options (Deribit) | ⏳ Prévu : Maquette validée, pas commencé | 🟡 Partiel : Chaîne et stratégies, pas pour la crypto | ✅ Oui : 7 bourses, volatilité, skew, GEX |
| Bourses crypto en direct | ✅ Fini : 18 bourses (16 avec charts) et les DEX, gratuites | ✅ Oui : Des dizaines | ✅ Oui : 29 bourses, plus de 4 000 paires |
| Forex et or | ✅ Fini : Avec ta clé OANDA | ✅ Oui | ✅ Oui : Plus de 1 200 paires, or, argent |
| Actions du monde entier | ⛔ Pas prévu : Choix du projet : la crypto ; quelques actions en perpétuels HIP-3 | ✅ Oui : Souvent en différé sans abonnement | 🟡 Partiel : États-Unis seulement ; direct à 19 $/mois |
| Futures CME | ⛔ Pas prévu : Choix du projet : seulement des données gratuites | 💲 Payant : Abonnement aux données | 💲 Payant : Différé gratuit ; direct dès 7 $/mois |
| Marchés de prédiction (Polymarket) | ⏳ Prévu : Demandé, pas commencé | ❌ Non | ✅ Oui : Cotes en direct |

### 4. Watchlist, alertes, actualités

Suivre le marché sans rien rater.

| Fonction | Argos HSS Trading | TradingView | OpenMarket |
|---|---|---|---|
| Watchlist et favoris par bourse | ✅ Fini : Panneau des marchés épinglé | ✅ Oui : 1 liste de 30 en gratuit | ✅ Oui : Listes, sections, drapeaux, notes |
| Recherche de symbole | ✅ Fini | ✅ Oui | ✅ Oui : Avec Ctrl + K |
| Alertes de prix et multi-conditions | ✅ Fini : Panneau Actives / Historique, son, notifications | 💲 Payant : 3 en gratuit ; multi-conditions en payant | 🟡 Partiel : Prix : 200 en gratuit ; multi-conditions pas dites |
| Alertes sur n'importe quel indicateur | ✅ Fini : Sur toute ligne d'indicateur ou de script, et sur les dessins | 💲 Payant : En payant (20 à 1 000) | ✅ Oui : 5 en gratuit ; aussi sur les dessins |
| Alertes même PC éteint | ⛔ Pas prévu : Choix du projet : tout reste sur ton appareil | ✅ Oui : Sur leurs serveurs | ✅ Oui : Sur leurs serveurs ; 60 jours en gratuit |
| Calendrier économique | ✅ Fini : Avec le réel et la réaction du prix | ✅ Oui | 🟡 Partiel : Séries éco, pas de calendrier décrit |
| Actualités avec ⚡ sur la chart | ✅ Fini : Onglets, filtre par symbole | 🟡 Partiel : Fil d'actualités ; événements des entreprises sur la chart | ✅ Oui : Fil en direct sur la chart |
| Sessions de marché | ✅ Fini | ✅ Oui | ✅ Oui : Carte des sessions |
| Scanner | 🟡 En cours : Double Structure, Top 24 h, tri de tous les marchés | ✅ Oui : Screeners très complets, scanner par script | ✅ Oui : Scanner par script, Pulse (mèmecoins) |

### 5. Trading démo, simulateur, backtest

S'entraîner sans risque, et vérifier ses stratégies.

| Fonction | Argos HSS Trading | TradingView | OpenMarket |
|---|---|---|---|
| Ticket d'ordre démo (Marché, Limite, Stop) | ✅ Fini : Zoom auto sur Limite, TP et SL | ✅ Oui : Compte démo | ✅ Oui : Compte démo gratuit, 100 000 $ |
| Positions et ordres | ✅ Fini | ✅ Oui | ✅ Oui : Sur la chart |
| Gardien PropFirm (perte max, risque, verrou news) | ✅ Fini | ❌ Non : Rien d'équivalent | ❔ Pas indiqué : Connexion PropFirm ; pas de gardien décrit |
| Vrais ordres chez une bourse | ⛔ Pas prévu : Choix du projet : démo seulement | ✅ Oui : Plus de 100 courtiers | ✅ Oui : Binance, Bybit, Hyperliquid ; CME par courtier |
| Simulateur / Replay | ✅ Fini : Multi-unités, ordres pendant le replay, coûts réels | 💲 Payant : Jours gratuits ; minutes, secondes, ticks en payant | 🟡 Partiel : Replay sans ordres ; trade par trade en payant |
| Backtest de stratégies | ✅ Fini : Rapport complet | ✅ Oui : Testeur complet ; historique profond en payant | ✅ Oui : Strategy Tester, frais et perps |
| Comparer le backtest avec TradingView | ✅ Fini : Trade par trade, côte à côte | ❌ Non | ❌ Non |
| Volet 🎯 Stratégie | ✅ Fini : Dans la barre de droite | 🟡 Partiel : Panneau du testeur | 🟡 Partiel : Panneau du testeur |
| Rapport du mois en PDF | ✅ Fini : Avec le commentaire du rond Argos AI | ❌ Non | 🟡 Partiel : Statistiques du compte démo |
| Journal et portefeuille | ✅ Fini | 🟡 Partiel : Portefeuilles ; pas de journal intégré | ✅ Oui : Journal (notes, étiquettes, images), portefeuille |
| Bots et stratégies (en démo) | ⏳ Prévu : Maquettes approuvées, recherche faite | 🟡 Partiel : Via webhooks et outils externes | 🟡 Partiel : Autopilot (Pulse, bêta), webhooks |

### 6. Argos AI, appareils et le reste

L'assistant, les écrans, et ce qui viendra en ligne.

| Fonction | Argos HSS Trading | TradingView | OpenMarket |
|---|---|---|---|
| Le rond Argos AI (lecture des indicateurs) | ✅ Fini : Petits restes | 💲 Payant : Assistant IA en option payante | ✅ Oui : Kata AI, écrit des indicateurs (crédits) |
| Argos Lens | ✅ Fini | 🟡 Partiel : Note technique achat / vente | ❔ Pas indiqué |
| Académie (figures, SMC, stratégies) | 🟡 En cours : 9 parcours, 5 faits | 🟡 Partiel : Contenu éducatif à part | ✅ Oui : Academy, environ 119 articles d'aide |
| PC, web, téléphone, tablette | ✅ Fini : Même code partout, version tactile ; Linux et web prêts ; Windows, Mac et Android en test | ✅ Oui : Applis natives partout | 🟡 Partiel : Web, Mac, Linux, Android ; Windows à venir ; iPhone en bêta |
| Synchro PC ↔ téléphone | 🟡 En cours : Presque finie (sans serveur) | ✅ Oui : Avec le compte | ✅ Oui : Avec le compte (5 à 20 appareils) |
| Langues | 🟡 En cours : Menu de 63 langues ; traductions en cours | ✅ Oui | ✅ Oui : 8 langues |
| Marketplace de modules signés | 🟡 En cours : Kit WASM fait, signature à faire | 🟡 Partiel : Scripts payants | ✅ Oui : Publier et vendre des scripts |
| Communauté, partage de trades, chat | ⏳ Prévu : Après les comptes et le P2P | ✅ Oui : Énorme communauté | 🟡 Partiel : Mises en page et scripts partagés, Discord |
| Prix | ✅ Fini : Gratuit, sans limite, sans pub | 💲 Payant : Gratuit limité avec pub, puis 4 offres | 💲 Payant : Gratuit limité ; Plus 35 $, Max 120 $ par mois |
| Vie privée | ✅ Fini : Tout reste sur ton appareil | ❌ Non : Compte et données chez eux | ❌ Non : Compte en ligne, code privé dans leur nuage |
| Brancher son IA (Claude, ChatGPT…) | 🟡 En cours : Connecteur AI commencé | 💲 Payant : 10 à 500 demandes par jour selon l'offre | ✅ Oui : L'agent change la chart |

## Sources

- TradingView : pages publiques de tradingview.com (fonctions, offres et prix), lues le 23 nov. 2026.
- OpenMarket : pages publiques du site d'OpenMarket (fonctions, aide, offres), lues le 23 nov. 2026.
- Argos HSS Trading : le code de ce dépôt (`rust/`), l'avancement par module du [lisez-moi](../README.md) et le registre des bugs (`rust/tools/porte/registre.txt`).

TradingView et OpenMarket sont des marques de leurs propriétaires. Argos n'a aucun lien avec eux et n'utilise pas leur code : le graphique d'Argos est écrit en Rust.
