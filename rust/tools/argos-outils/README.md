# 🧰 argos-outils

Les petits outils d'Argos, en **Rust pur** (règle de Jee : aucun Python, outils compris). Un seul programme, une
commande par outil. Aucune dépendance aux crates d'Argos HSS : son propre espace de travail, compilé dans
`rust/target/outils`. Seulement des crates déjà approuvées (flate2, serde_json, sha1, ureq, mêmes versions que
`rust/Cargo.lock`).

Aussi une bibliothèque : un autre programme peut réutiliser le serveur (`argos_outils::serveur::main`) et y ajouter
ses propres routes JSON privées (`Route`, mêmes accès que `/api/reglages`) et des réglages jamais synchronisés
(`Serveur::interdits`) ; argos-outils, lui, ne sait rien de ce qu'elles servent.

Licence : MIT OR Apache-2.0.

## Construire

```bash
cd rust && make outils
```

Le programme est alors `rust/target/outils/release/argos-outils`.

## Commandes

| Commande | Ce qu'elle fait | Remplace |
|---|---|---|
| `serveur [port] [dossier] [/app/]` | serveur local de l'app (`/app/` en 3e : « / » y renvoie ; gzip, ETag, HTTP/1.1, relais DXY / news / calendrier, session partagée `/api/reglages`) ; Argos HSS Trading le lance sur le port 8000 par son propre programme (`argos-serveur`, `make web`) | `argos_serve.py` (+ ses tests : `cargo test`) |
| `version-app [dist]` | écrit `dist/app/version.json` et met la version dans `index.html` et `sw.js` | `argos_app_version.py` |
| `langue modele` / `langue verifier` | packs de langue `lang/*.json` d'après `i18n.rs` | `argos_lang.py` |
| `emoji <dossier svg>` | émojis du code → `assets/twemoji/` + `twemoji.rs` | `argos_emoji.py` |
| `emoji-tout <dossier svg> [emoji-test.txt] [--noms UnicodeData.txt]` | tout Twemoji → `emoji/` + `emoji_tout.rs` | `argos_emoji_tout.py` |
| `maquette <vue>` | `ecrans/maquette-<vue>.json` → `maquette_<vue>.rs` | `argos_maquette_rs.py` |
| `v1-inputs` | inputs des indicateurs de v1 (`app.js`, évalué par `node`) → `v1_inputs.rs` | `argos_v1_inputs.py` |
| `assemble [--check / --test]` | site v1 : `src/*` → `index.html` + `app.js?v=…` (SHA-256 sans dépendance) ; `make assemble` | `tools/assemble.py` (sortie identique octet pour octet) |
| `local [--wifi [--nouveau-code]]` | site v1 en local sur `http://localhost:8787` (`$ARGOS_PORT`) : relais `/api/` vers le site en ligne, sauvegarde `/local/backup`, relais Ollama, calendrier, code secret `?t=` du mode wifi ; `make local` | `argos_local.py` (mêmes réponses) |
| `publier-capture <fichier.png> [nom.png] [adresse…]` | copie une capture dans le dossier stable des captures servies (`$ARGOS_CAPTURES_DOSSIER`, sinon `captures-servies` à côté du dépôt), puis vérifie `adresse/captures/nom` par une vraie requête (200 et image) sur chaque adresse (par défaut `http://127.0.0.1:8000`) : une adresse n'est donnée qu'après ce ✅ | `publier-capture.sh` |

Option commune : `--racine <dossier rust/>` (par défaut le dossier courant).

Pour `emoji-tout`, les noms anglais des caractères (recherche) viennent de `--noms` (`UnicodeData.txt` d'unicode.org)
ou, sans l'option, du `unicore/Name.pl` de Perl déjà sur le PC.

## Vérifier

```bash
cd rust && make outils-check
```

(formatage, clippy, tests). `make vert` le fait aussi, et refuse tout fichier `.py` dans `rust/`.
