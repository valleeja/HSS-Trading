# Continuer Argos en Rust : garde-fous et mode d'emploi

26 nov. 2026 · Pour toute personne qui reprend ou aide le projet.

## En bref

Tout le code est en Rust, dans `rust/`. La règle d'or : rien n'entre dans `main` sans la porte de qualité verte (`make porte`), et un bug corrigé ne doit jamais revenir (chaque bug a sa ligne au registre et son test).

- L'app : une seule app Rust, en fenêtre sur le PC (`argos-desktop`) et dans le navigateur, le téléphone et la tablette (WebAssembly, page `/app2/`).
- Taille : environ 260 000 lignes, plus de 2 100 tests, registre des bugs B-001 à B-102.
- Démo seulement : aucun ordre réel. Argos est gratuit. Licence PolyForm Noncommercial 1.0.0, jamais de GPL.
- Ce qu'il reste à faire est dans le [lisez-moi](../README.md), partie « Ce qui reste à faire ».

## Installer et compiler

Rust est figé à la version 1.99.1 par `rust/rust-toolchain.toml` : `rustup` l'installe tout seul, avec rustfmt, clippy et la cible `wasm32-unknown-unknown`. Toutes les commandes se lancent depuis `rust/`.

Outils à installer une fois : `cargo install cargo-audit cargo-deny --locked`, puis gitleaks et valgrind (par le gestionnaire de paquets), et Playwright pour la porte du navigateur. Une fois par clone : `make hooks`.

| Commande | Ce qu'elle fait |
| --- | --- |
| `make run` | compile en release et ouvre la fenêtre `argos-desktop` |
| `make web-build` | construit l'app navigateur (`tools/argos_web_build.sh`) dans `dist/` ; puis `./lancer-serveur.sh` à la racine la sert sur le port 8000 |
| `make web` | construit l'app navigateur puis lance `argos-serveur` au premier plan (`/app/`) |
| `make fmt` | met en forme nos crates (jamais `vendor/`) |
| `make check` | formatage, clippy (PC et navigateur), tous les tests, le test de vitesse seul, les modules WASM, argos-outils |
| `make vert` | `make check` + `cargo audit` + `cargo deny` + aucun fichier Python |
| `make porte` | la porte de qualité complète, obligatoire avant toute fusion (environ 15 min) |
| `make registre` | vérifie que chaque test du registre des bugs existe encore |
| `make clean-cache` | vide les caches qui se refont seuls ; à lancer toutes les 30 min d'une longue session, ou si le disque est plein |
| `make tauri` | vérifie et lance la coque Tauri 2 (`argos-tauri/`, à part du reste) |

Les crates (`rust/crates/`) : argos-core (types communs, comptes, backtest, portefeuille), argos-data (bourses, direct, liste blanche `net.rs`), argos-script (Argos Script et Pine), argos-render (dessin de la chart et des indicateurs), argos-store (base locale redb), argos-propfirm, argos-desktop (l'app, PC et WASM), argos-scanner, argos-module-host (modules WASM), argos-academy, argos-lens, argos-serveur, argos-ai. Le plan complet est dans `rust/ARCHITECTURE.md`.

Une seule compilation à la fois : chaque grosse commande du Makefile prend toute seule un verrou et tourne sous `nice -n 19`. Sur un ordinateur avec peu de mémoire (16 Go ou moins), deux compilations en même temps peuvent le figer : limiter la mémoire, par exemple `systemd-run --user --scope -p MemoryMax=12G -- make check`, et baisser le nombre de tâches (`-j 6`) au besoin.

## Les tests cargo et la porte de qualité

`make porte` est la seule façon de faire avancer la branche principale. Elle tourne dans cet ordre et s'arrête au premier rouge :

1. **Début du tampon** (`tools/porte/tampon.sh debut`) : tout doit être commité, fichiers non suivis compris.
2. **gitleaks obligatoire** : sans lui, la porte reste rouge.
3. **`make vert`** = `make check` + `cargo audit --deny unsound --deny yanked` + gitleaks sur tout l'historique + aucun fichier `.py` + `cargo deny check` (licences : aucune GPL, AGPL ni LGPL).
4. **Le registre des bugs** (`make registre`) : chaque test qui garde un bug fermé existe encore.
5. **Le travail compté** (`tools/porte/travail.sh`, valgrind) : 4 gestes (glisser et pincer la chart du téléphone, liste des marchés, glisser la chart du PC). Plus de 1 % au-dessus de la référence `travail.txt` = rouge. La référence est un cliquet : elle ne descend qu'avec `ARGOS_VITESSE=maj`, et ne monte qu'avec `ARGOS_VITESSE=plus-lent` et l'accord du mainteneur.
6. **Le navigateur** (`make porte-navigateur`) : l'app web de cet arbre, servie sur le port 8012 avec des réglages jetables, mesurée dans Chromium en iPhone 13 (netteté, images par seconde, à +25 % de `navigateur.txt` au plus).
7. **Fin du tampon** : un fichier `argos-porte/<arbre>` dans le dossier git. Un hook `reference-transaction` peut protéger la branche principale : il refuse tout mouvement vers un contenu sans tampon.

Ce que `make check` contient :

- `cargo fmt --check` sur nos crates, les modules et la coque Tauri ;
- `cargo clippy --workspace --all-targets -- -D warnings`, puis clippy d'`argos-desktop` en `wasm32-unknown-unknown` ;
- `cargo test --workspace` (plus de 2 100 tests au 26 nov.), dont le **filet de sécurité** (9 tests `safety_net::*` : 6 169 écrans de référence, frontières entre modules, fichiers gardés), la **bande noire**, la **netteté** (`render::tests::sharp_at_rest_even_after_slow_frames`), les **tests de fiabilité des données** sur de vraies réponses enregistrées des bourses, les **indicateurs comparés à TradingView**, et `toutes_les_sources_du_code_sont_listees` (liste blanche réseau) ;
- le **test de vitesse** `tests::vitesse::the_cell_stays_as_fast_as_its_reference`, lancé seul à la fin (il doit afficher « 1 passed ») ;
- clippy et tests de chaque module WASM, puis `argos-outils`.

Le **registre des bugs** est `rust/tools/porte/registre.txt` : une ligne par bug, 7 champs séparés par `|` (id, date, le bug vu, la cause, la correction, la branche et le commit, les tests qui le gardent). Un bug corrigé sans ligne ni test n'est pas fini.

Quand le **filet** est rouge après un changement d'écran voulu : regarder les images `safety-net-<scène>.nouveau.png`, puis `ARGOS_REFERENCES=maj cargo test -p argos-desktop --locked safety_net`, relire le diff (seulement ses propres écrans) et commiter. Un fichier `.rs` nouveau va dans `safety_net/map.txt` ; une fenêtre nouvelle va aussi dans `App::par_dessus` (render.rs). Jamais `maj` à l'aveugle, jamais retirer une scène pour passer au vert.

## Les garde-fous

La règle du projet : « je ne veux jamais revenir sur un bug corrigé ». Aucune version n'entre dans la branche principale sans tous les garde-fous ; un lot rouge recommence jusqu'au vert.

**Les règles de travail**

- Défauts et failles trouvés : corrigés avant de montrer quoi que ce soit (porte repassée, ligne au registre, test).
- Améliorations trouvées : proposées avec une recommandation, jamais faites sans l'accord de Jee.
- Toute branche est relue par quelqu'un d'autre que son auteur avant la fusion. Jee fusionne.
- Jamais `--no-verify`. Jamais de contournement d'un refus d'un hook ou de la porte.
- Une seule compilation à la fois. Après 3 essais ratés : s'arrêter et écrire ce qui bloque, avec la sortie.
- Toute nouvelle adresse réseau va dans `rust/crates/argos-data/src/net.rs` (HTTPS ou WSS seulement).
- Chaque nouvelle dépendance (crate ou outil) est demandée à Jee, une par une.
- Tout code d'ordre reste en démo ; le portefeuille reste en lecture seule.
- Rust pur partout, outils compris : aucun fichier Python.
- Les workflows GitHub Actions restent manuels (`workflow_dispatch`) : jamais de déclencheur `push` ou `pull_request`, et les actions sont figées par hash de commit.

**La base, à chaque fusion (toute en place)**

- [x] Formatage, clippy, tous les tests
- [x] Filet de sécurité (écrans de référence)
- [x] Sécurité : audit des dépendances, licences, secrets (gitleaks)
- [x] Affichage net au repos, aucune bande noire
- [x] Vitesse : travail compté à 1 % près, mesure dans un vrai navigateur
- [x] Registre des bugs avec une fiche et un test par bug
- [x] Branche principale protégée par le hook

**Les 15 garde-fous pro**

| N° | Garde-fou | État au 26 nov. |
| --- | --- | --- |
| 1 | Budgets chiffrés : démarrage, mémoire, poids, batterie | à faire (étape 1) |
| 2 | Endurance de plusieurs heures, sans fuite ni ralentissement | à faire (étape 3) |
| 3 | Indicateurs identiques à TradingView sur un historique figé | en grande partie : 25 indicateurs vérifiés ; à rendre permanent (étape 1) |
| 4 | Fuzz : données folles, aucun plantage | à faire (étape 3) |
| 5 | Relecture par un second relecteur indépendant | faite à la main ; à automatiser (étape 1) |
| 6 | Accessibilité | à faire (étape 3) |
| 7 | Retour arrière en une ligne | à la main ; script pas encore installé |
| 8 | Tests de mutation | à faire (étape 4) |
| 9 | Tests de propriétés (solde jamais négatif, aucun centime perdu) | à faire (étape 2) |
| 10 | Chaos : pannes provoquées exprès | à faire (étape 2) |
| 11 | Versions reproductibles et signées | à faire (étape 4) |
| 12 | Dépendances verrouillées, SBOM | à faire (étape 3) |
| 13 | Plan des menaces | à faire (étape 3) |
| 14 | Déploiement par étapes | à faire (étape 2) |
| 15 | Fiche après chaque bug | fait (registre) |

L'ordre approuvé : étape 0 (faite : la porte, le registre, la branche protégée), puis les étapes 1 à 4, chacune avec l'accord de Jee.

## Faire un changement, de bout en bout

1. **Partir de `main`** dans une branche à part (un fork, ou `git worktree add ../wt-<nom> -b <nom> main`). Un nom court qui dit le lot (ex. `tv-classiques-2`).
2. **Coder petit** : un lot = un sujet. Chaque bug corrigé reçoit son numéro B-xxx, sa ligne au registre et un test qui échoue sans la correction.
3. **Vérifier en cours de route** : seulement les tests touchés (`cargo test -p <crate> <nom>`).
4. **Commiter** : le hook `pre-commit` lance `make vert` si `rust/` change ; rouge = commit refusé. Signer avec `git commit -s` (voir [CLA.md](../CLA.md)).
5. **Passer la porte** : `cd rust && make porte` (environ 15 min, tout commité avant).
6. **Ouvrir la pull request** avec ses preuves : résultat de la porte, nombre de tests, lignes ajoutées au registre.
7. **Relecture** : « pas conforme » = corriger, repasser la porte, pousser sur la même branche.
8. **Fusion** par Jee.

Pour lancer l'app : `make run` pour la fenêtre du PC ; `cd rust && make web-build`, puis `./lancer-serveur.sh` à la racine du dépôt, et `http://localhost:8000/app2/` dans le navigateur. Pour le téléphone et la tablette, il faut une adresse https privée (par exemple avec Tailscale).

## Les pièges connus

Ces pièges ont tous coûté du temps au moins une fois.

- **Mémoire** : deux compilations en même temps peuvent figer un ordinateur de 16 Go. Le verrou absent ne prouve pas qu'une autre compilation est finie (pause entre deux étapes).
- **rust-analyzer** lance tout seul un `cargo check` dès qu'un `.rs` est modifié : pendant une compilation lourde, n'éditer aucun `.rs`.
- **Dossier de compilation partagé entre copies de travail** : après une compilation depuis une autre copie, faire `touch` sur le `lib.rs` touché avant de compiler, sinon l'ancien code est lié sans rien dire.
- **`make porte` dure environ 15 min** : certains outils coupent une commande au premier plan après 10 min. La lancer en tâche de fond suivie, avec un journal horodaté.
- **Le filet et l'horloge** : une scène ne doit dépendre ni du jour, ni de l'heure, ni de la vitesse de l'ordinateur. Vérifier une scène douteuse sous `faketime -m -f '+1d x4'`.
- **Une fenêtre nouvelle** doit aller dans `App::par_dessus` (render.rs), sinon la porte du navigateur ne la voit pas (B-041).
- **Le test de vitesse** compare à l'ordinateur de référence du projet : un autre ordinateur ou une autre version de Rust demande une nouvelle référence locale (`ARGOS_VITESSE=plus-lent`), à ne pas commiter sans accord.
- **Coque Tauri** (`rust/argos-tauri`) : son propre `Cargo.lock`, hors de `make vert`.
- **Secrets** : aucune clé privée ni clé de bourse dans le dépôt. Les clés API vivent dans le trousseau du système (service `argos-hss-trading`). Les faux positifs connus de gitleaks sont listés dans `.gitleaks.toml`.

## Où sont les documents

Dans le dépôt : [`README.md`](../README.md) (l'app, l'avancement par module, ce qui reste à faire), `rust/ARCHITECTURE.md` (les crates et le chemin d'une bougie), `rust/tools/porte/registre.txt` (le registre des bugs), [`CONTRIBUTING.md`](../CONTRIBUTING.md), [`GOVERNANCE.md`](../GOVERNANCE.md), [`LICENCES-TIERS.md`](../LICENCES-TIERS.md), [`docs/PROVENANCE.md`](PROVENANCE.md) (d'où vient chaque indicateur), la [comparaison avec TradingView et OpenMarket](COMPARAISON-TRADINGVIEW-OPENMARKET.md), et [`docs/ANDROID.md`](ANDROID.md) (l'APK Android, à finir).

Ce qui reste à faire, dans l'ordre : finir le lot en cours (`interface-100-1`), finir à 100 % la chart, les outils, la watchlist, Argos Script, les indicateurs et le Support, puis les garde-fous 1 à 4, la séparation en modules, les Bots, l'Académie en Rust et le reste de la liste.
