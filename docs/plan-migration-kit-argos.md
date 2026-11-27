# Plan : passer les modules d'Argos HSS Trading au kit Argos

> ✅ **Fait le 2026-11-06 avec le kit 1.0.0** (interface v2, réponse à `docs/demande-kit-argos-v2.md`) : voir
> `rust/vendor/SOURCE-kit-module-argos.md`. Le texte ci-dessous est le plan d'origine.

> Rédigé le 2026-11-06 après lecture **seule** de `Bibliotheque/kit-module-argos/` (version 0.2.0, branche du kit
> `step14-licence`), de `VERIFICATION-PROJETS.md` et de `LISEZ-MOI-PROJETS.md`. Rien n'a été modifié là-bas.
> À valider par Jee avant tout code.

## 1. Les différences

| Sujet | Trading aujourd'hui (`argos-module-sdk` 0.1, maison) | Kit Argos 0.2.0 |
|---|---|---|
| Nom du crate | `argos-module-sdk` | `argos-module-sdk` : **même nom, autre interface** (conflit) |
| Licence du SDK | celle du dépôt (GPL-3.0) | MIT OR Apache-2.0 (libre) |
| Exécution (bulle) | `argos-module-host` : wasmtime 50, sans WASI | `argos-sandbox` : wasmtime 50.0.1, sans WASI, **propriétaire** |
| Module d'import | `argos_host` | `argos` |
| Appels à l'hôte | `http_get`, `file_read`, `file_write`, `ai_ask` (réponse dans un bloc donné par `argos_alloc`) | `read_file`, `write_file`, `network_request`, `ai_request` (réponse dans un tampon fourni par le module) |
| Entrée du module | `argos_run(ptr, len) -> i64` + `argos_abi`, `argos_alloc`, `argos_free` | `run() -> i32` (macro `entry!`), rien d'autre |
| Données in / out | `Input` (bougies, unité, symbole, réglages) → `Output` (courbes, étiquettes, lignes du dock), en postcard | **aucune** : `run()` ne reçoit rien et rend un entier |
| Manifeste | `module.toml` : `id = "argos-trading/…"`, version texte, `sha256`, auteur (Argos ID + clé publique), `[droits]`, `[limites]`, `[[reglages]]` | `argos-module.json` : `path = "trading/…"`, version entière croissante, `capabilities`, `author`, `budget_secs`, domaines, modèle d'IA |
| Droits | réseau (hôtes de `net.rs`), dossier privé, IA (désactivée) | `ReadFiles`, `WriteFiles`, `Network` (domaines exacts), `Ai` (modèle local figé) ; sous-dossiers permis |
| Empreinte | SHA-256 dans le manifeste | SHA-384 dans l'`Approval` (aucune autre empreinte acceptée) |
| Limites | `memoire_mo`, `temps_ms` par appel | `Limits::DEFAULT` + `budget_secs` (2 s par défaut, 120 s au plus) |
| Réglages utilisateur | `[[reglages]]` (nombre, case, couleur, choix) | pas d'équivalent (`ServicePolicy` = réglages des services, pas de l'indicateur) |
| Navigateur (`/app/`) | Web Worker de la page, mêmes vérifications (`scan`) | rien : `argos-sandbox` dépend de `ureq` et `rustix`, **ne compile pas en WASM** |
| Dépendances d'un module | libres | liste fermée `allowed-dependencies.txt` (versions exactes), pas de macro procédurale |
| Compilation | `tools/argos_module_exemple.sh` | outil `argos-modules build` (fmt, clippy, test, audit, deny, 2 compilations identiques, `proposal.json`) |

## 2. Le vrai trou : un indicateur a besoin de données

L'interface du kit ne passe **ni les bougies ni le résultat** : `run()` n'a pas d'argument et rend un `i32`. Un
indicateur de chart (la moyenne d'exemple, et tout module de Trading) ne peut pas s'écrire avec elle sans détour.
Deux façons :

1. **Proposer une version 2 de l'interface au kit Argos** (recommandé ; `VERIFICATION-PROJETS.md` l'envisage déjà) :
   une entrée `run_data(ptr, len) -> i64` + `alloc` / `free`, entrée et sortie en postcard, comme Trading. Les
   droits, le manifeste et la bulle du kit restent la référence.
2. En attendant, un détour : l'app écrit l'entrée dans un fichier que le module lit (`ReadFiles`) et lit sa
   sortie (`WriteFiles`). Possible, mais lent et il donne des droits fichiers à tous les indicateurs : je le
   déconseille.

## 3. Ce qui change chez nous (une fois la v2 décidée)

- **Crate** : notre `argos-module-sdk` disparaît (ou est renommé `argos-trading-indicator` le temps de la
  transition) ; les modules utilisent le SDK du kit Argos.
- **`argos-module-host`** : sur desktop, il appelle `argos_sandbox::Sandbox::run_budgeted` avec une `Approval`
  (empreinte SHA-384, droits, `ServicePolicy`) au lieu de son moteur wasmtime ; on garde la page, les droits
  redemandés et les réglages. Dans le navigateur, le Worker de la page doit parler la même interface `argos`
  (même refus par défaut, mêmes codes `HOST_*`).
- **Page 🧩 Modules** : lit `argos-module.json` ; droits affichés avec les mots du kit Argos (fichiers lecture /
  écriture, réseau + domaines, IA + modèle) ; version entière ; mention « non vérifié » pour un module reçu
  (niveau B) ; installation d'un `module.wasm` + `proposal.json`.
- **Module d'exemple « Moyenne »** : refait avec le kit (`#![forbid(unsafe_code)]`, `entry!`, seulement des
  dépendances permises), compilé par `argos-modules build`.
- **Réglages** : `[[reglages]]` n'existe pas dans le kit Argos ; à proposer dans la v2 (ou gardés dans un fichier à côté
  du manifeste du kit Argos, propre à Trading).

## 4. Comment dépendre du kit

**Recommandation : une copie figée dans le dépôt** (`rust/vendor/kit-module-argos-0.2.0/`), avec une note de
source (version, date, branche du kit, empreintes `SHA384SUMS` vérifiées) et une dépendance par chemin interne.
Le build ne dépend jamais de l'endroit où se trouve le dossier du kit ; une mise à jour du kit = une nouvelle copie,
vérifiée, dans un commit à part. (Une dépendance vers le dossier du kit casserait si le dossier bouge ; une
dépendance git demanderait un dépôt du kit Argos accessible et n'est pas permise pour un module.)

À noter : `argos-sandbox` et `argos-module-policy` sont **propriétaires** et fournis seulement sous licence ou clé
de maintenance : les copier dans Trading exige d'abord la sortie de la GPL (voir § 6).

## 5. Notre dépôt face à `VERIFICATION-PROJETS.md`

| Point | État |
|---|---|
| `rust-toolchain.toml` Rust 1.99.1 + cible WASM | ✅ fait (commit `f7f9fff`) |
| `#![forbid(unsafe_code)]` | ✅ fait (commit `dcdf552` ; exception étroite : notre SDK) |
| Argent en décimal dans `argos-propfirm` | ⚠️ fait en **centimes entiers** (`Money`, i64 ; commits `ccc6deb`, `da84ee4`) ; le kit Argos conseille `rust_decimal` 1.43.0. Les deux sont exacts ; passer `Money` sur `rust_decimal` est possible sans changer l'API si Jee le veut. |
| Conflit `argos-module-sdk` | ❌ à régler : ce plan |
| `argos_local.py` reste un outil à part | ✅ |
| Licence GPL-3.0-only | ❌ à régler : voir § 6 |

## 6. Licence : passer en propriétaire — ce qui bloque

- **Dépendances** : aucune GPL / AGPL / LGPL imposée (`r-efi` est aussi sous MIT). ✅
- **Contributeurs** : seulement Jee (et l'assistant sous sa direction), plus des robots de mise à jour de
  dépendances. ✅
- **Code d'autres auteurs** : ❌ bloquant tel quel —
  - `argos-script/src/indicators/swing_vwap.rs` et `smart_swing_vwap.rs` : transcriptions sous **CC BY-NC-SA 4.0**
    (pas d'usage commercial, même licence obligatoire) ;
  - `argos-script/src/indicators/price_action.rs` (port d'un script tiers d'order / breaker blocks) et l'oscillateur
    de Bollinger de `classic.rs` (port d'un script tiers) : leur licence d'origine est à vérifier ; ils ne sont pas
    listés dans `LICENCES-TIERS.md` ;
  - `classic.rs` : la heatmap de volume et le profil de volume (ports de scripts tiers), et `cipher.rs` (logique
    réécrite d'un script tiers) : licences à vérifier.
  Il faut les **retirer**, les **réécrire de zéro** (sans le code d'origine), ou obtenir l'**accord écrit** de
  leurs auteurs, avant la nouvelle licence.
- Les versions déjà publiées sous GPL restent sous GPL pour ceux qui les ont reçues.
- Un avocat doit vérifier avant toute publication.
