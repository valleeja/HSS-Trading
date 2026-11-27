# Plan : toute la logique de Trading en modules (kit module Argos), l'affichage à la fin

> Demande de Jee (2026-11-06) : « toute la logique de Trading passe en modules Rust → WASM signés avec
> argos-module-sdk (kit Argos 1.0.0), l'affichage dans une coque Tauri 2. Pur calcul d'abord, affichage à la fin.
> Un module à la fois. »
>
> Mise à jour du 6 novembre au soir : Jee choisit la coque Tauri 2 (section D).
>
> Point de départ réel : l'app est **déjà en Rust** (calcul dans `argos-script`, dessin dans `argos-render`, fenêtre
> native et version navigateur WASM dans `argos-desktop`). Il n'y a plus de JavaScript de calcul : « l'ancien JS »
> veut dire ici **l'indicateur intégré en Rust**. Le kit 1.0.0 est déjà copié dans `rust/vendor/` (commit e043ed1) ;
> l'ancien dossier Tauri `desktop/` a été retiré le 4 novembre (d3e421a).

## A. Le calcul pur d'abord : un module par indicateur

- Chaque indicateur devient une crate `rust/modules/<nom>/` (son propre espace de travail, comme les modules du
  kit), avec `argos-module.json` (`trading/<nom>`, `abi: 2`, **aucun droit**, ses réglages), seulement le SDK du kit,
  `#![forbid(unsafe_code)]`.
- L'app lui donne les données (adaptations du kit pour Trading) : **aucun réseau**, entrée de **4 Mio au plus**
  (50 000 bougies), **prix en entiers** à l'échelle choisie par l'app (arrondis une fois), `EMPTY` = vide. Le module
  peut calculer en nombres à virgule dedans, il rend des entiers (échelle des prix + 4 décimales pour une courbe de
  prix ; 6 décimales pour un oscillateur).
- Ordre : les indicateurs un par un (voir E), puis les **figures** (patterns : sortie = étiquettes et lignes), puis
  l'**évaluation des alertes** (un module qui reçoit les bougies et les conditions en réglages et rend « vrai /
  faux » par bougie dans une courbe).
- Ce qui ne peut pas devenir un module tel quel (ruban, bulles, carnet : données en direct, pas des bougies) attend
  une interface v3 du kit (à demander à Argos le moment venu).

## B. L'indicateur intégré reste tant que le module n'est pas égal

1. Le module est écrit et ajouté au catalogue local (🧩 Modules ▸ CATALOGUE LOCAL), à côté de l'intégré.
2. Test d'égalité dans `make check` : le module, dans la **bulle du kit** (desktop), rend la même courbe que
   l'intégré sur des bougies fixes, à l'arrondi de sa sortie près, plus les **valeurs de référence** déjà vérifiées
   à part (`argos-script/tests/valeurs_de_reference.rs`). Dans le navigateur, le même `.wasm` passe par le Web Worker
   avec la même entrée (format du kit, vérifié octet pour octet).
3. Seulement quand ces tests sont verts : un interrupteur « calcul par module » pour cet indicateur, puis, après un
   temps d'essai par Jee, l'intégré est retiré (un commit à part).

## C. Signature

- Le kit **ne signe jamais** : son outil `argos-modules build` (dans la Bibliothèque d'Argos, pas copié dans
  Trading) vérifie le module, le compile deux fois (même empreinte) et écrit `module.wasm` + `proposal.json` **non
  signé**. La signature est faite par un humain, avec **sa** clé, côté Argos (niveau marketplace : audit signé,
  vote, journal).
- Commande pour Jee (lui seul la lance), depuis le dossier du kit dans la Bibliothèque :
  `./argos-modules build "<chemin>/argos-hss-trading/rust/modules/ema" sortie-ema . --offline`
  puis il remet `sortie-ema/proposal.json`, `sortie-ema/module.wasm` et le code source à Argos pour la signature.
- Trading **ne lit jamais** la clé privée (`~/.argos-signature/prive.pem`) et n'a aucune fonction « signer ».
  Aujourd'hui l'accord est l'empreinte SHA-384 du `.wasm` (niveau A : l'utilisateur dans son compte). Quand Argos
  Argos publiera sa petite crate de vérification, Trading vérifiera les signatures avec la **clé publique** seulement.

## D. L'affichage : coque Tauri 2 (décision de Jee, 6 novembre au soir)

Jee veut une **vraie app en magasin** (Android, iPhone, PC) avec notifications, caméra pour les QR codes, stockage
sécurisé des clés et mode hors ligne : la coque Tauri 2 est retenue. **L'app native actuelle reste** tant que la
coque ne fait pas tout.

Règles qui ne changent pas :
- les modules restent **signés et vérifiés comme avant** (manifeste du kit, SHA-384 accepté, droits refusés par
  défaut) et tournent dans la **bulle du kit, dans le cœur Rust de la coque**, jamais dans le JavaScript de la page ;
- les **clés** ne vont jamais dans la page web ni dans le JS : elles restent dans le cœur Rust et le coffre du système ;
- les **capacités Tauri 2** sont au minimum (refus par défaut) : chaque plugin et chaque commande ajoutés un par un,
  seulement pour la fenêtre qui en a besoin.

| Étape | Contenu | État |
|---|---|---|
| **T0** | Coque minimale (`rust/argos-tauri/`, son propre espace de travail) : une page web qui trace le **module EMA**. Le cœur Rust vérifie le manifeste, le `.wasm` et l'empreinte acceptée (`approbations.json`), lance le module sans aucun droit, avec les bougies de la couche de données d'Argos (BTC 1 h, source de secours) ou un échantillon fixe hors ligne. Une seule commande permise (`module_ema`), aucun plugin, CSP stricte. `make tauri` la vérifie et la lance. | ✅ fait (Linux) |
| **T1** | Toute la chart : la version WASM existante (`/app/`) chargée dans la coque. Les modules passent par une commande du cœur Rust (bulle du kit) au lieu du Web Worker ; le réseau passe par le cœur (liste blanche `net.rs`). | fait (branche `tauri-t1`, attend « ok fusion ») : réseau (`reseau`), direct (`direct_ouvrir` / `direct_fermer`, un canal) et modules (`module_lancer`, bulle du kit, aucun droit) par le cœur ; pages copiées dans `argos-tauri/web` par `make tauri-web` ; clés et relais `/api/` du site pas encore (T2) |
| **T2** | Plugins officiels, chacun avec sa capacité minimale : notifications (`notification`), QR codes (`barcode-scanner`, mobile), clés dans le coffre du système (Keychain iOS / Keystore Android / trousseau du PC, ou `stronghold`), cache hors ligne (bougies et réglages dans le dossier de l'app). | à faire |
| **T3** | Build **Android** : Android SDK + NDK + JDK sur le PC, puis `cargo tauri android init` et `cargo tauri android build`. | à faire |
| **T4** | Build **iPhone** : demande **macOS + Xcode**, donc impossible sur le PC Linux de Jee (un Mac, ou un Mac loué / CI macOS). | bloqué sans Mac |
| **T5** | Magasins : icônes, signature des paquets (clé de signature Android gardée par Jee, certificat Apple), fiches. **Payant** : compte Google Play (25 $ une fois) et Apple Developer (99 $ / an) — décision de Jee. | à faire |
| **T6** | Retrait de l'app native **seulement** quand la coque fait tout (même chart, mêmes outils, mêmes tests). | plus tard |

Risques notés :
- **iOS interdit la compilation à la volée** (JIT) : wasmtime y tourne avec son interpréteur (Pulley), plus lent ;
  à mesurer à T4 (sinon un autre moteur WASM sans JIT dans la bulle).
- **Linux** : la vue web est WebKitGTK (plus lente que Chrome) et Tauri tire GTK 3 (`glib` 0.18 a un avis
  RUSTSEC-2024-0429 sur un itérateur qu'on n'utilise pas, ignoré seulement pour la coque).
- **Licences** : Tauri amène 5 petites crates en MPL-2.0 (lecture du CSS) : permises dans `argos-tauri/deny.toml`,
  acceptées par Jee le 6 nov. (licence d'Argos HSS Trading : PolyForm Noncommercial 1.0.0).
- Deux affichages (natif et coque) à garder égaux jusqu'à T6.

## E. Ordre des indicateurs (effort estimé)

| # | Module | Effort | Pourquoi dans cet ordre |
|---|---|---|---|
| 1 | **EMA** ✅ (fait : `rust/modules/ema`, égal à l'intégrée) | petit | courbe sur le prix, valeur de référence |
| 2 | SMA, Bandes de Bollinger, VWAP du jour | petit | courbes sur le prix, références existantes |
| 3 | RSI, MACD, Stochastique, ATR, volume relatif | petit | oscillateurs (il faudra afficher un module en bande : ajouter « bande » à la fiche du module) |
| 4 | Les 12 des manuels (RSI stochastique, Ichimoku, Keltner…) | moyen | déjà écrits de façon pure, 23 valeurs de référence |
| 5 | Pivot VWAP, order blocks, force des cassures, chaleur et profil de volume | moyen | écrits de zéro le 6 novembre |
| 6 | Figures (patterns) | moyen | sortie en étiquettes + lignes |
| 7 | Évaluation des alertes | moyen | conditions en réglages, sortie « vrai / faux » |
| 8 | TVD Pro, TBO, AI Trend, Cipher, SMC, Vector Zone | grand | gros indicateurs de Jee, beaucoup de sorties (zones, boîtes, tableaux) : demander une sortie plus riche au kit si besoin |

Chaque ligne = un commit local, `make vert` vert, sans push.
