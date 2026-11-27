# Argos HSS Trading 📈

Une app de trading crypto écrite en Rust : des charts en direct, des indicateurs, un langage de scripts (Argos Script) qui lit aussi le Pine Script, un simulateur et un compte démo.
**Démo seulement : Argos ne passe jamais d'ordre réel.** Argos est gratuit.

> **Version de test.** L'app est encore en construction : des choses peuvent manquer ou bouger. Tes retours sont les bienvenus (onglet Issues).

---

## 🙋 Pourquoi ce dépôt : on cherche des collaborateurs

Argos n'est pas fini : il est à environ 70 %. Je le publie pour trouver des collaborateurs qui veulent m'aider à le continuer.

La raison : j'ai construit Argos avec Claude Code, l'assistant de code d'Anthropic. Depuis sa dernière mise à jour, Claude Code ne respecte plus les garde-fous que j'ai mis en place et il sabote le projet. D'après ce que je lis sur X, des milliers d'autres utilisateurs vivent la même chose. Je ne peux plus continuer le projet seul avec lui.

Tout est prêt pour reprendre : le guide [Continuer Argos en Rust](docs/CONTINUER-EN-RUST.md) explique comment compiler, tester et respecter les garde-fous, et [CONTRIBUTING.md](CONTRIBUTING.md) explique comment proposer un changement. Pour Android, voir [docs/ANDROID.md](docs/ANDROID.md). La [comparaison avec TradingView et OpenMarket](docs/COMPARAISON-TRADINGVIEW-OPENMARKET.md) montre où en est Argos.

— Jee

---

## ✨ Ce que fait l'app aujourd'hui

- **Charts en direct** de 16 bourses crypto (Binance, Hyperliquid, OKX, Bybit, Kraken, Bitget, Bitfinex…), paires contre BTC, perpétuels avec le funding, forex avec une clé OANDA. 17 types de bougies, split écran, plein écran, Market Profile.
- **Outils de dessin** (lignes, Fibonacci, zones, pinceau…), avec l'aimant et les modèles.
- **Watchlist** : favoris par bourse, recherche de symbole, Tape, market cap.
- **Indicateurs** : les classiques (RSI, MACD, Stochastique, SAR, EMA, Bollinger, Ichimoku…), vérifiés contre TradingView, et les indicateurs Argos (Cipher Pro, AI Trend, TVD Pro, TBO, SMC, Price Action, Argos Murs, Argos Vague, Vector Zone Pro), footprint, carte thermique, carnet / delta / DOM.
- **Argos Script** : un éditeur avec sa documentation, qui importe le Pine Script tel quel.
- **Simulateur** (replay, backtest comparé à TradingView) et **Trading démo** (ticket complet, positions, Gardien PropFirm).
- **Alertes**, **actualités** (11 sources gratuites), **calendrier économique**, **portefeuille**, **journal chiffré**, **rapport du mois**.
- **Argos AI** : un assistant qui lit les indicateurs posés sur la chart.
- **Support** : billets Bug, Idée, Question, avec capture.
- **Une seule app** : la fenêtre sur le PC et la même app dans le navigateur, sur le téléphone et la tablette (WebAssembly).

---

## 💻 Installation

Chaque version de test aura ses fichiers prêts à installer dans **Releases** (à droite de cette page).

| Plateforme | Fichier | Comment il est construit | État |
|---|---|---|---|
| 🐧 Linux | `.tar.gz` | sur le PC de Jee | ✅ l'app marche ; le paquet arrive avec la 1re version |
| 🪟 Windows | `.zip` | GitHub Actions | ⏳ 1re construction à venir |
| 🍎 Mac (Apple Silicon) | `.dmg` | GitHub Actions | ⏳ 1re construction à venir |
| 🤖 Android (arm64) | `.apk` | GitHub Actions (coque Tauri 2) | 🧪 expérimental, 1re construction à venir |
| 🌐 Navigateur, téléphone, tablette | page web `/app2/` | sur le PC de Jee | ✅ marche |

### 🐧 Linux
1. Télécharge le fichier `.tar.gz` de la dernière version dans Releases.
2. Décompresse-le (clic droit, « Extraire ici »).
3. Double-clique sur `argos-desktop` (ou lance `./argos-desktop` dans un terminal).

### 🪟 Windows
1. Télécharge le fichier `.zip` de la dernière version dans Releases et décompresse-le.
2. Double-clique sur `Argos.exe`. Windows peut afficher « Windows a protégé votre ordinateur » (l'app n'est pas encore signée par Microsoft) : clique sur « Informations complémentaires », puis « Exécuter quand même ».

### 🍎 Mac
1. Télécharge le fichier `.dmg` de la dernière version dans Releases.
2. Ouvre-le et glisse Argos dans Applications.
3. Au premier lancement, macOS peut refuser une app non signée par Apple : clic droit sur Argos, « Ouvrir », puis « Ouvrir » encore.

### 🤖 Android
> 🧪 **Expérimental.** L'APK n'a encore jamais été essayé sur un vrai téléphone : il peut ne pas s'installer ou avoir des défauts. C'est une version de débogage, pour les téléphones arm64. En attendant, l'app marche déjà dans le navigateur du téléphone (voir plus bas). **Tu veux aider à le corriger ?** Tout est expliqué dans [`docs/ANDROID.md`](docs/ANDROID.md) : comment l'APK est fait, comment le construire et le déboguer, et les points qui risquent de casser.

1. Sur le téléphone, télécharge le fichier `.apk` de la dernière version dans Releases.
2. Ouvre-le. Android demande d'autoriser l'installation depuis cette source : accepte pour ton navigateur.
3. Pour les mises à jour automatiques, l'app [Obtainium](https://github.com/ImranR98/Obtainium) peut suivre les Releases de ce dépôt.

### 🌐 Navigateur, téléphone et tablette
L'app tourne aussi dans le navigateur, servie par le PC :
```bash
cd rust && make web-build && cd ..   # construit les pages (dist/)
./lancer-serveur.sh                  # à la racine du dépôt : port 8000, en arrière-plan
```
Puis `http://localhost:8000/app2/` sur le PC. Pour le téléphone et la tablette, une adresse https privée avec [Tailscale](https://tailscale.com).

### 🦀 Compiler soi-même (toutes les plateformes)
Il faut [Rust](https://rustup.rs) ; la bonne version (1.99.1) s'installe toute seule grâce à `rust/rust-toolchain.toml`.
```bash
cd rust
make run      # compile et ouvre la fenêtre (argos-desktop)
make check    # format, clippy et tous les tests
make porte    # la porte de qualité complète (environ 15 min)
```

### Où l'app garde tes affaires
- Réglages, dessins et listes : `~/.config/argos-hss-trading/charts`
- Journal chiffré : `~/.local/share/argos-hss-trading`
- Clés API des bourses : le trousseau du système (service `argos-hss-trading`). Elles ne quittent jamais ton appareil.

---

## 📊 Avancement par module

Estimation module par module (carte d'avancement v7 du 24 nov. 2026 ; plusieurs fusions sont venues après, surtout des corrections et de la fiabilité).

**Toute l'app : 66 %**
- ① Chart, indicateurs, simulateur, démo : **80 %**
- ② Académie, traduction, Support : **39 %**
- ③ En ligne (comptes, messages, partage) : **25 %**
- ④ Séparer l'app en modules : **20 %**

| Gros module | % | Ce qui est dedans (%) |
|---|---|---|
| ⚡ Flux d'ordres | 98 % | Trades (Tape) 98 · Footprint 95 · Liquidations 100 · Baleines 100 |
| 💼 Trading démo | 99 % | Ticket 98 · Positions et ordres 100 · Gardien 100 |
| ⭐ Watchlist | 96 % | Watchlist 98 · Recherche de symbole 98 · Bourses 93 |
| ⏯ Simulateur | 96 % | Replay 95 · Backtest 98 |
| 🏆 PropFirm | 95 % | PropFirm 95 |
| 🔔 Alertes | 93 % | Alertes 90 · Surveillance 100 · Notifications 90 |
| 📅 Actualités et calendrier | 93 % | Calendrier 90 · Actualités 90 · Sessions 100 |
| 🌍 Marchés | 88 % | Funding 95 · Marché 100 · CoinMarketCap 95 · Macro 100 · Sentiment 100 · DEX 100 · Hyperliquid 95 et 90 · Indicateurs clés 100 · Options 0 |
| 📈 Chart | 80 % | Chart 92 · Bougies 100 · Outils 90 · Pinceau 100 · Split 90 · Zen 100 · Market Profile 90 · Objets 50 · Comparer des symboles 10 |
| 🏠 Interface | 80 % | Accueil 90 · Clavier 90 · Palette 90 · Thèmes 80 · Espaces de travail 40 · Appli 90 |
| 📜 Argos Script et indicateurs | 74 % | Moteur et éditeur 90 · Classiques 50 · Argos 76 · Bibliothèque 80 |
| 📊 Portefeuille et journal | 73 % | Portefeuille 93 · Journal 90 · Wallet 80 · Import / Export 40 · Rapports PDF 60 |
| ⭕ Argos AI | 62 % | Argos Lens 90 · Argos AI 85 · Connecteur AI 10 |
| 📖 Carnet | 60 % | Carnet 40 · Carte thermique 50 · Murs 90 |
| 🛟 Support | 60 % | Support 60 |
| 📡 Scanner | 50 % | Scanner 50 |
| 👤 Compte | 42 % | Connexion 50 · Clés API 90 · Synchro PC et téléphone 80 · Profils 0 · Sécurité 30 · P2P 0 |
| 🛒 Marketplace | 40 % | Marketplace 40 |
| 🌐 Langues | 35 % | Packs de langue 35 |
| 🎓 Académie | 30 % | Académie 60 · Manuel 0 |
| 🤖 Bots et stratégies | 10 % | Bots et stratégies 20 · Stratégie en direct 0 |
| 👥 Communauté | 8 % | Communauté 40 · Partage 10 · le reste 0 |
| 💰 Gagner | 0 % | plus tard |
| 🔒 La coquille (le cœur) | 20 % | filet de sécurité et kit des modules faits ; reste la porte fixe, le registre, une crate par module, les mises à jour signées |

**Depuis cette carte** (24 au 26 nov.) : version tactile du téléphone et de la tablette, tableaux plus rapides, fluidité, porte de qualité et registre des bugs, panneau Carnet / Delta / DOM, Cipher et Ichimoku comme TradingView, 7 scripts Pine qui roulent tels quels, Argos Script plus robuste, MACD / Stochastique / SAR comme TradingView. Plus de **2 100 tests** passent.

---

## 🧭 Ce qui reste à faire

### 1. En cours : finir à 100 % (le périmètre final)
Décidé par Jee le 25 nov. 2026 : on finit à 100 % la chart, les outils, la watchlist, Argos Script, les indicateurs, l'import du Pine Script et le Support.

- **Lot en cours** : `interface-100-1` (défauts d'affichage et de saisie sur PC, tablette et téléphone, texte d'Argos AI en style verre, chart libre vers la gauche). Il n'est pas encore dans ce dépôt. Le Croisement EMA, les EMA multiples et TVD Pro sont maintenant comme TradingView.
- **📈 Chart** : quelques chevauchements (pilule du titre, barre flottante), étiquette de date du réticule, grille « Unité de temps » sur téléphone, repère DÉMO sur tablette debout, trou d'une heure sur Bitfinex, le PC qui redessine tout à chaque mouvement de souris, panneau Objets, comparer des symboles.
- **✏️ Outils** : essais au doigt sur l'iPad ; Coller, curseur, flèches et Entrée dans les champs de texte.
- **⭐ Watchlist** : variation 24 h des favoris Binance, Entrée qui vide la recherche, format du volume ; Bitunix, Bitstamp et MEXC bloqués dans la version web (relais à décider).
- **📜 Argos Script et Pine** : boutons de l'éditeur sur téléphone, vieux fichiers Pine renommés, décalage après la dernière bougie, liste « Mes scripts » et console.
- **📊 Indicateurs** : recherche sur iPad, les 7 entrées « bientôt » du menu, « Mes modèles », environ 60 indicateurs classiques à écrire en Argos Script, Argos Pression Bubble et Argos Liquidité.
- **🛟 Support** : carte « Qu'est-ce qu'on construit ensuite ? », rapport de plantage automatique, FAQ et articles d'aide.

### 2. 🤖 Bots et stratégies (module K) : pour plus tard
Maquettes approuvées le 23 nov. ; rien n'est codé. Toujours en démo, jamais d'argent réel.
- **Créer une stratégie** : tu la décris, le rond Argos AI l'écrit en Argos Script, l'essaie et la corrige. Rien ne va sur la chart sans ton ▶.
- **Partir le bot** : dans la stratégie, un bouton la fait tourner en direct sur le compte démo, avec son rapport, une pause du jour à −3 % et un bouton d'arrêt.
- **Page « Bots de trading »**, « + Nouveau bot », et le bot « Retour 66-88 » tiré de la recherche du 23 nov.
- **API locale des indicateurs** (idée du 26 nov.) : un autre programme (bot, tableur, script) demande à Argos la valeur d'un indicateur réglé, par exemple RSI 14 sur BTC en 1 jour, et la reçoit. TradingView ne le permet pas.
- Ordres automatiques en démo, règles sans code : à étudier après la séparation.

### 3. 🎓 Académie : en pause (reprise prévue après la séparation en modules)
La maquette v15 est approuvée : **3 parties, 136 leçons**, 18 schémas, démos PC et téléphone, une page Pratique.
- **HSS Trading** : comment se servir de l'app (h01 à h30).
- **Formation de base** : de « coin, token, blockchain » jusqu'à la routine et au journal, plus « Les coulisses de la finance ».
- **FTA** : les comptes PropFirm, le risque, les figures, le SMC, 16 stratégies, les news et les sessions.

Reste : la coder en Rust comme un module (avec la vraie chart en démo), les vidéos de l'app, le manuel d'Argos, un parcours « Créer ses indicateurs » et « Créer ses bots ».

### 4. Tout le reste, rangé par module
- **Qualité** : garde-fous 1 à 4 (indicateurs comparés à TradingView en permanence, relecture automatique, budgets, tests de propriétés et de chaos, endurance, fuzz, accessibilité, dépendances verrouillées, versions signées, retour arrière en une ligne).
- **Séparer l'app en modules** (environ 35 fusions) : chaque module dans sa propre crate, mis à jour seul et signé par la clé de Jee. Puis un moteur par carte graphique (120 images/s).
- **Marchés** : Options (Deribit), carte thermique épurée, Argos Murs lot 4, Scanner Order flow.
- **Interface** : rouvrir l'app exactement où on l'a laissée, ajout rapide aux favoris depuis la recherche, espaces de travail, menu des modules à épingles.
- **Simulateur et démo** : petits défauts notés, bandeau DÉMO.
- **Argos AI** : le connecteur (Claude, ChatGPT…).
- **Langues** : environ 850 phrases à traduire, polices chinoises, japonaises et arabes.
- **Marketplace** : la refonte.
- **En ligne (P2P, sans serveur)** : comptes, messages, partage de trades, direct, classement, billets de support en ligne, mises à jour dans l'app.
- **Après** : les 10 widgets et les 74 idées de « ce qu'ils ont de plus », marchés de prédiction, app de bureau multi-fenêtres, dépôt public.

---

## 🔐 Sécurité
- Aucune clé privée, phrase de récupération ni clé de bourse dans ce dépôt. L'historique est vérifié avec gitleaks.
- Les clés API restent dans le trousseau de ton appareil.
- Chaque adresse appelée par l'app est dans une liste blanche (`rust/crates/argos-data/src/net.rs`), en HTTPS ou WSS seulement.
- Aucun ordre réel : le trading est en démo.

Signaler une faille : voir [`SECURITY.md`](SECURITY.md).

## 📜 Licence
Argos HSS Trading est sous **PolyForm Noncommercial 1.0.0** ([`LICENSE`](LICENSE)), © 2026 Jee : tu peux l'utiliser, le copier et le modifier **sans but commercial** ; tout usage commercial demande l'accord de Jee. Quelques fichiers de tiers gardent leur propre licence : voir [`LICENCES-TIERS.md`](LICENCES-TIERS.md).
