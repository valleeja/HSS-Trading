# Vérification des projets (2026-11-06, en lecture seule)

Faite par Argos pour que le kit module convienne à chaque projet. Aucun fichier des projets n'a
été modifié ; aucun fichier de clé ou de secret n'a été ouvert.

## Argos HSS Project (`.developpement/en-cours/argos-hss`)
**Compatible**
- Rust 1.99.1 figé dans `rust-toolchain.toml`, comme Argos.
- Les sommes d'argent utilisent `rust_decimal` 1.43.0 (la même version que la liste autorisée) ;
  pas de `f64` trouvé dans les devis, les heures ni la CCQ.

**À noter / à changer**
- Édition 2024 dans le projet ; le kit et l'exemple sont en édition 2021. Un module peut rester en
  2021 sans souci ; s'il passe en 2024, le kit fonctionne aussi.
- Il faut ajouter la cible `wasm32-unknown-unknown` au `rust-toolchain.toml` pour compiler des
  modules (elle est dans celui du kit).
- Aucune crate n'a `#![forbid(unsafe_code)]` (0 sur 14). C'est exigé dans les **modules** seulement,
  pas dans l'app ; à envisager quand même.
- `ia-locale` utilise **candle** 0.11 (Rust pur, safetensors). Ses dépendances ne sont pas dans la
  liste autorisée des modules : à faire passer par une décision Argos si l'IA doit tourner dans un module.
- Des fichiers `.wasm` déjà construits (`argos-sst.wasm`, `argos-securite.wasm`) sont livrés dans
  l'interface. Ce ne sont pas des modules Argos : ils doivent être reconstruits avec l'outil du kit
  (source, empreinte, proposition) avant de devenir des modules signés.
- Licence : passe de GPL-3.0 à PolyForm Noncommercial 1.0.0 (décision de Jee du 2026-11-06, voir plus bas).

## Argos HSS Trading (`argos-hss-trading/rust`)
**Compatible**
- wasmtime 50.0.1 sans WASI dans `argos-module-host`, comme la bulle d'Argos.
- Modules prévus en `wasm32-unknown-unknown`, refus par défaut, aucune clé donnée aux modules :
  mêmes principes qu'Argos.

**Conflit à régler (décision de Jee)**
- Le projet a **son propre crate `argos-module-sdk`**, avec une autre interface : import `argos_host`
  (`http_get`, `file_read`, `file_write`, `ai_ask`), exports `argos_abi`, `argos_alloc`, `argos_free`,
  `argos_run(ptr, len) -> i64`, entrées et sorties en postcard, manifeste `module.toml`.
  Le kit d'Argos utilise l'import `argos` (`read_file`, `write_file`, `network_request`,
  `ai_request`, avec un tampon de sortie), l'export `run() -> i32` et le manifeste `argos-module.json`.
  Les deux ne sont pas compatibles et portent le même nom de crate. Recommandation : l'interface
  d'Argos fait référence (gouvernance, signatures, politique) ; Trading renomme son crate ou passe
  au kit Argos. Les bonnes idées de Trading (allocation, sortie riche) ont nourri l'interface v2 du
  kit 1.0.0 (2026-11-06, demande de Trading a19e359) : suivre la section « Migrer depuis la v1 » du
  LISEZ-MOI (entiers à échelle fixe au lieu de `f64`, format du kit au lieu de postcard).

**À changer**
- Pas de `rust-toolchain.toml` : ajouter celui du kit (Rust 1.99.1 et la cible WASM).
- Aucune crate n'a `#![forbid(unsafe_code)]`.
- Les règles des prop firms (`argos-propfirm`) utilisent des `f64` pour des montants (taille du
  compte, perte journalière) : passer à `rust_decimal` pour l'argent (les indicateurs techniques et
  les pourcentages peuvent rester en `f64`).
- `argos_local.py` est un petit serveur de développement local : c'est correct tant qu'il reste un
  outil à part et n'entre jamais dans les applis.
- Licence : passe de GPL-3.0-only à PolyForm Noncommercial 1.0.0 (décision de Jee du 2026-11-06, voir plus bas).

## Licence (décidée par Jee le 2026-11-06, version finale du jour)
- `argos-module-sdk` : libre (MIT OR Apache-2.0), compatible avec tout projet.
- `argos-sandbox` et `argos-module-policy` : **PolyForm Noncommercial 1.0.0** (SPDX
  `PolyForm-Noncommercial-1.0.0`) : code visible, gratuit pour les particuliers et les usages non
  commerciaux ; les entreprises louent une licence commerciale avec des clés de maintenance Argos.
- **Conflit réglé** : les projets Argos HSS (Argos HSS Project, Argos HSS Trading et les suivants) et
  leurs modules officiels passent eux aussi en **PolyForm Noncommercial 1.0.0** : **jamais de GPL ni
  d'AGPL**. Ce n'est pas de l'« open source » au sens OSI (licence « code source visible »).
- À faire dans chaque projet (par sa propre session) : remplacer les fichiers et champs de licence
  GPL/AGPL (`LICENSE`, `license = "PolyForm-Noncommercial-1.0.0"` dans chaque `Cargo.toml`, en-têtes,
  texte officiel copié tel quel de `argos-sandbox/LICENSE.md` avec sa propre ligne « Required Notice »),
  ajouter un `COMMERCIAL.md`, et retirer l'AGPL des modules en cours (par exemple
  `hss/portfolio-tracking`). Les dépendances tierces restent sous licences permissives (`deny.toml`).
- Le code déjà publié sous GPL reste disponible sous GPL pour qui l'a déjà reçu ; le changement vaut pour
  la suite (possible parce que Jee est l'auteur ; à confirmer pour toute contribution extérieure).
- Un avocat doit vérifier avant toute publication sur GitHub.
