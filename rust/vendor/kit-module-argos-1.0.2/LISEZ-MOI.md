# Kit module Argos (version 1.0.2)

Pour **Argos HSS Project** et **Argos HSS Trading** (et les futurs projets gouvernés par Argos).
Ce kit permet de créer un module en Rust, de le compiler en WASM et de produire une proposition
**non signée** à renvoyer à Argos. Il ne contient aucune clé, ni privée ni de test.

## Contenu
| Fichier | Rôle |
| --- | --- |
| `argos-module-sdk/` | Le kit Rust : l'entrée du module (v1 `entry!` ou v2 `entry_data!`), les données d'entrée et de sortie v2, et les 5 appels à l'hôte (fichiers, réseau, IA locale, modèle signé). Seul endroit où il y a de l'`unsafe`. Les vecteurs de conformité v2 sont dans `argos-module-sdk/tests/vectors/abi-v2.json`. |
| `argos-sandbox/` | La bulle (wasmtime) qui fait tourner un module approuvé, avec ses services fichiers, réseau et IA locale. Pour l'app du projet. |
| `argos-module-policy/` | Les droits, les réglages et l'approbation d'un module, utilisés par la bulle. |
| `argos-modules` | L'outil de vérification et de compilation (binaire Linux x86_64). |
| `rust-toolchain.toml` | La version de Rust figée (1.99.1), avec la cible `wasm32-unknown-unknown`. |
| `allowed-dependencies.txt` | Les seules crates permises dans un module, à la version exacte. |
| `deny.toml` | La politique cargo-deny des modules (licences, avis de sécurité, sources). |
| `exemple-module/` | Un module minimal (interface v1) qui passe les 5 contrôles Cargo. |
| `exemple-moyenne/` | La moyenne mobile (interface v2) : bougies en entrée, une courbe et une carte en sortie, deux réglages. |
| `INTERFACE-V2.md` | La spécification exacte de l'interface v2 (en anglais), assez précise pour un second hôte (le Web Worker du navigateur). |
| `VERIFICATION-PROJETS.md` | Ce qui doit changer dans chaque projet pour utiliser ce kit. |
| `VERSION`, `SHA384SUMS` | La version du kit et l'empreinte de chaque fichier. |

## Avant la première fois
1. Vérifier les empreintes : `sha384sum -c SHA384SUMS` (tout doit afficher `OK`).
2. Installer `cargo-audit` et `cargo-deny` : `cargo install --locked cargo-audit cargo-deny`.
3. Rustup installe tout seul Rust 1.99.1 et la cible WASM grâce à `rust-toolchain.toml`.

## Créer un module, étape par étape
1. Copier `exemple-module/` (v1) ou `exemple-moyenne/` (v2) à côté de `argos-module-sdk/` (le chemin `../argos-module-sdk` doit rester juste).
2. Dans `Cargo.toml`, changer le nom du paquet.
3. Dans `argos-module.json`, mettre le chemin du module (`hss/...` ou `trading/...`), la version (un entier
   qui monte toujours), les droits, l'Argos ID de l'auteur, et s'il le faut les domaines, le modèle d'IA
   et le budget de temps (2 s par défaut, 120 s au maximum).
4. Écrire la logique dans `src/lib.rs`. La première ligne reste `#![forbid(unsafe_code)]`.
   L'entrée se déclare avec `argos_module_sdk::entry!(ma_fonction);`.
5. Seulement des dépendances de `allowed-dependencies.txt`, à la version exacte. Pas de `build.rs`,
   pas de dépendance git, pas d'autre dépendance par chemin que le kit. Pour l'argent : `rust_decimal`,
   jamais `f64`.
6. `cargo generate-lockfile`, puis, depuis le dossier du kit :
   `./argos-modules build mon-module sortie . --offline`
   L'outil vérifie tout, lance fmt, clippy, test, audit et deny, compile deux fois en WASM (même
   empreinte exigée) et écrit `sortie/module.wasm` et `sortie/proposal.json`.

## Données d'entrée et réglages (interface v2)
Nouveau dans la version 1.0.0. Un module v2 ne demande aucun droit pour recevoir ses données :
c'est l'app qui lui donne les bougies et les réglages, et il rend des courbes, des étiquettes et des
cartes. Tout le détail est dans `INTERFACE-V2.md`.

- **Déclarer** dans `argos-module.json` : `"abi": 2` et la liste `"settings"`. Il y a 4 types :
  `number` (entier, avec `min`, `max`, `default` et `scale`, le nombre de décimales de 0 à 6),
  `bool`, `color` (`"#rrggbb"`) et `choice` (une liste de choix). Au plus 16 réglages.
  Exemple : `{ "id": "length", "name": "Longueur", "type": "number", "default": 20, "min": 1, "max": 500 }`.
- **Écrire** `fn run(input: Input) -> Output`, puis `argos_module_sdk::entry_data!(run);`.
  `input.bars` contient les bougies (t, o, h, l, c, v en entiers) ; `input.settings` contient les
  valeurs dans l'ordre du manifeste : un nombre à son échelle, 0 ou 1, `0xRRGGBB`, ou l'index du choix.
- **Les prix sont des entiers**, avec une échelle fixe (`price_scale`). Par exemple, 2345,67 $ à
  l'échelle 2 s'écrit 234567. On n'utilise jamais `f64` pour l'argent. Une valeur absente s'écrit
  `EMPTY` (`i64::MIN`), pas NaN.
- **Limites** : 50 000 bougies et 4 Mio en entrée ; 16 courbes (une valeur par bougie),
  1000 étiquettes et 32 cartes en sortie, 4 Mio au plus.
- **Les réglages ne sont pas des droits.** Ils font partie de la politique signée et la fiche de
  droits les affiche sous « Réglages (ce ne sont pas des droits) ». L'app vérifie chaque valeur avant
  de lancer le module : une valeur hors bornes est refusée.
- **Dans l'app** : `Sandbox::run_data(&approval, &wasm, &input, limits, services)` rend une
  `Output` déjà vérifiée. Prévoir assez de carburant pour les grands graphiques : 50 000 bougies avec
  une longueur de 500 ont été testées avec 200 millions.
- Un module ne peut pas mélanger les deux interfaces : l'outil et la bulle le refusent.

## Migrer depuis la v1 (pour Argos HSS Trading)
1. Remplacer le SDK maison de Trading par `argos-module-sdk` de ce kit, en dépendance par chemin
   (`path = "../argos-module-sdk"`). C'est la seule dépendance par chemin permise.
2. Remplacer l'ancienne entrée par `entry_data!(run)`, avec `fn run(Input) -> Output`.
3. Passer les prix de `f64` à des entiers à l'échelle : l'app convertit une seule fois à l'entrée
   (`(prix × 10^échelle)` arrondi) et une seule fois pour l'affichage. Le module peut calculer en
   flottant à l'intérieur, mais il rend des entiers.
4. Remplacer NaN par `EMPTY`.
5. Déclarer `"abi": 2` et les réglages dans `argos-module.json` (valeurs entières, avec `scale`).
6. Remplacer postcard par le format du kit. Le Web Worker du navigateur l'implémente d'après
   `INTERFACE-V2.md` et doit passer tous les vecteurs de `abi-v2.json`, octet pour octet.
7. Les modules v1 (`entry!`, `run`) continuent de marcher sans changement.
8. L'encodage signé de la politique a changé (ajout de `abi` et `settings`). Une release signée avec
   le kit 0.3.0 doit être signée de nouveau ; aucune vraie clé n'a encore servi.

## Utiliser un modèle d'IA signé (`model_request`)
Un module peut faire tourner UN modèle d'IA (ONNX) déjà approuvé comme module à part
(`argos-modele-<nom>`), avec le droit `Model` et, dans `argos-module.json`, `"model"` : l'empreinte
SHA-384 de ce module-modèle. Le moteur (tract) tourne dans l'app, hors de la bulle ; le module n'envoie
que les données d'entrée : rang (u32), dimensions (u32), valeurs `f32` (petit-boutiste), 64 Ko au plus,
et reçoit les valeurs de sortie. L'app vérifie les poids avant chaque chargement (format par octets,
empreinte, aucun droit). Aucun modèle n'est dans ce kit : pour convertir PaddleOCR ou un autre modèle en
ONNX, demander les outils d'export à l'équipe Argos.

## Les 3 niveaux
- **A, dans son propre compte** : l'utilisateur accorde lui-même les droits après avoir lu la fiche.
- **B, partage par code QR** : même chose, avec la mention « non vérifié » pour celui qui reçoit.
- **C, marketplace** : examen de sécurité, audit signé, vote des mainteneurs et inscription au journal.

## Ce qu'il faut renvoyer à Argos
Pour le niveau C : `proposal.json` (non signé) et `module.wasm`, avec le code source du module.
L'outil ne signe jamais. La signature est toujours faite par un humain, avec sa propre clé.

## Utiliser la bulle dans l'app du projet
La bulle reçoit une `Approval` : l'empreinte SHA-384 du WASM approuvé, ses droits et ses réglages.
Elle ne vérifie aucune signature elle-même : c'est Argos (niveau C) ou l'utilisateur dans son
propre compte (niveaux A et B) qui approuve. Pour lancer un module :
`Sandbox::run_budgeted(&approval, &wasm, "run", Limits::DEFAULT, services)` (interface v1) ou
`Sandbox::run_data(&approval, &wasm, &input, limits, services)` (interface v2), avec les services
créés par `Services::build`. Les tests de `argos-sandbox/tests/` montrent chaque cas.

## Licences, en mots simples
**Gratuit pour les particuliers, payant pour les entreprises.**
- `argos-module-sdk` : **libre**, MIT ou Apache-2.0 au choix. On peut l'utiliser partout pour écrire
  un module, en gardant le fichier de licence.
- `argos-sandbox` et `argos-module-policy` : **PolyForm Noncommercial 1.0.0** (`LICENSE.md`, texte
  officiel). Le code est visible et **gratuit pour les particuliers et tout usage non commercial**.
  Une **entreprise** ou toute autre entité commerciale doit louer une **licence commerciale**, fournie
  avec des **clés de maintenance Argos** qui contrôlent sa branche et ses modules (`COMMERCIAL.md`,
  contact argoshss@proton.me). Il est interdit d'en retirer les vérifications (empreintes, signatures,
  clés).
- Ce n'est **pas** de l'« open source » au sens de l'OSI : c'est une licence « code source visible ».
  Ne jamais écrire « open source » sans le préciser.
- Les projets Argos HSS (Argos HSS Project, Argos HSS Trading et les suivants) et leurs **modules
  officiels** suivent la même licence PolyForm Noncommercial 1.0.0 (décision de Jee du 2026-11-06) :
  **jamais de GPL ni d'AGPL**. Un module garde `publish = false` et ses dépendances restent sous
  licences permissives (`deny.toml`).
- L'outil `argos-modules` est fourni pour s'en servir, pas pour être redistribué.
- Tout le reste d'Argos (clés, journal, votes, détection, app) est privé et n'est pas dans ce kit.
- Aucun module n'est vendu : les entreprises louent des clés de maintenance.

Un avocat doit vérifier avant toute publication sur GitHub.

## Ce qui est interdit
WASI, `unsafe` dans le module, une clé dans le code, un réseau non déclaré, une marque d'autrui
(on lit des formats ouverts : DWG/DXF, IFC).
