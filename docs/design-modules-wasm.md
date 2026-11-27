# 🧩 Design : les modules d'Argos HSS Trading en Rust → WASM, dans un bac à sable

> Document de design seulement : aucun code n'est écrit tant que Jee ne l'a pas validé.
> Il applique la « règle des modules » décidée par Jee le 2026-11-05 (Bibliothèque d'Argos, en lecture seule).
> Rédigé le 2026-11-05, branche locale `travail-local`.

## 1. Le but en une phrase

Un module, c'est du code Rust compilé en WASM. Il tourne dans un bac à sable qui, par défaut, ne lui donne **rien** :
pas de fichiers, pas de réseau, pas d'IA et jamais de clés. Il ne reçoit que ce que son manifeste demande et que
l'utilisateur a accepté. Le même module tourne sur desktop (wasmtime) et dans l'app navigateur `/app/`
(le WebAssembly du navigateur).

## 2. Ce qui est un module, et ce qui n'en est pas un

| Élément | Aujourd'hui | Avec ce design |
|---|---|---|
| Indicateurs officiels (Vector Zone, Patterns, SMC…) | Rust compilé **dans** l'app | Restent dans l'app : ce sont des parties de l'app, pas des modules. Ils pourront devenir des modules officiels plus tard (phase 4), sans obligation. |
| Cartes du dock (🎯 Indicateurs clés…) | Rust dans l'app | Pareil : ce sont des parties de l'app. |
| Scripts d'utilisateurs (éditeur, Pine collé) | Interprétés par `argos-script::lang` dans l'app | Phase 3 : l'interpréteur lui-même devient un **module officiel**, et le script de l'utilisateur n'est plus qu'une donnée qu'on lui passe. |
| Modules de tiers (Marketplace) | — (aucun encore) | Uniquement des modules WASM avec manifeste. |

Chaque module appartient à **un seul projet** : ici `argos-trading`. Les modules peuvent former un **arbre** : le champ
`parent` du manifeste (par exemple « Scanner » → « Scanner crypto » → « Scanner Hyperliquid »). Un enfant n'a jamais
plus de droits que son parent.

## 3. Le module vu de l'intérieur (côté auteur)

- C'est une crate Rust de type `cdylib`, compilée pour `wasm32-unknown-unknown`. **Pas de WASI** : il n'y a donc aucun
  accès caché au système.
- Elle dépend d'une petite crate **`argos-module-sdk`** (nouvelle, sous la licence du projet). Elle donne :
  - les types d'entrée (bougies, unité de temps, réglages) et de sortie (tracés, zones, étiquettes, lignes du dock) ;
  - les appels à l'hôte qui existent (réseau, fichiers, IA). Chacun échoue proprement si le droit n'est pas accordé.
- Ce que le module **exporte** (interface binaire, version 1) :
  - `argos_abi() -> u32` : version de l'interface (1) ;
  - `argos_alloc(len) -> ptr` et `argos_free(ptr, len)` : mémoire d'échange ;
  - `argos_run(ptr, len) -> u64` : reçoit l'entrée sérialisée et rend la sortie sérialisée (pointeur et longueur
    regroupés dans le u64).
- Sérialisation : **postcard**, binaire compact et déterministe. Pour déboguer, le SDK sait aussi écrire en JSON.
- Ce que le module peut **importer** : seulement les fonctions `argos_host::*` décrites au point 5. L'hôte ne fournit
  ces fonctions que si le droit correspondant est accordé. Sinon, l'import est remplacé par une fonction qui renvoie
  « refusé ». Un module qui importe autre chose est refusé au chargement.

## 4. Le manifeste (`module.toml`) : tout est refusé par défaut

```toml
id = "argos-trading/scanner-hl"   # projet/nom : un seul projet par module
version = "0.3.1"
parent = "argos-trading/scanner"  # arbre de modules (facultatif)
abi = 1
auteur = { argos_id = "…", cle_publique = "ed25519:…" }   # jamais de clé privée
sha256 = "…"                       # empreinte du .wasm

[droits]                           # vide = aucun droit
reseau  = ["api.hyperliquid.xyz"]  # seulement des adresses déjà dans net.rs (HTTPS / WSS)
fichiers = []                      # dossier privé du module seulement, jamais ailleurs
ia = { modele = "nom-exact@version-figée" }   # facultatif ; version figée, pas de clé

[limites]
memoire_mo = 64
temps_ms = 200                     # par appel de argos_run
```

Règles vérifiées au chargement, et le module est refusé si une seule échoue :

- `reseau` ⊂ liste blanche de `argos-data/src/net.rs` ⊂ droits du parent.
- `fichiers` : le seul chemin possible est le dossier privé du module, `modules/<id>/`. Il n'y a ni chemin absolu ni `..`.
- `ia.modele` doit être un nom exact avec sa version figée. « dernier » ou « latest » est refusé.
- Ces champs n'existent pas et sont refusés : `cles`, `secrets`, ou tout autre champ inconnu.
- L'utilisateur voit les droits en français dans une fenêtre et les accepte un par un. Ce qu'il refuse reste refusé.

## 5. L'hôte : ce que le bac à sable donne, et comment

Une crate **`argos-module-host`** contient la logique commune : lecture du manifeste, vérifications, droits, limites.
Elle a deux moteurs, choisis à la compilation.

### 5.1 Desktop : wasmtime

- `wasmtime` sans WASI. Licence Apache-2.0 WITH LLVM-exception, déjà acceptée dans `deny.toml`. Il n'est compilé que
  pour le desktop, jamais dans le build navigateur.
- Limites :
  - `StoreLimits` pour la mémoire (`memoire_mo`) ;
  - **fuel** (nombre d'instructions) et **epoch interruption** pour le temps (`temps_ms`).
  Un module trop lent ou trop gourmand est arrêté, pas l'app.
- Imports fournis selon les droits :
  - `argos_host::http_get(url)` : l'hôte vérifie l'adresse (manifeste + net.rs) et fait lui-même la requête avec le
    client de l'app ;
  - `argos_host::file_read / file_write(nom)` : dossier privé du module seulement ;
  - `argos_host::ai_ask(texte)` : l'hôte appelle le modèle figé du manifeste.
- **Clés** : le module ne voit jamais une clé, même indirectement. Il ne peut pas demander « ajoute ma clé
  CoinMarketCap ». Si un module a besoin de données qui demandent une clé, c'est l'app qui les récupère (fils de
  données existants) et qui les met dans l'**entrée** du module, déjà lues. Le module ne peut pas choisir l'adresse
  d'une requête faite avec une clé.

### 5.2 Navigateur (`/app/`) : WebAssembly du navigateur

wasmtime ne tourne pas dans un navigateur. L'app `/app/` est elle-même du WASM. Elle charge donc le module avec
`WebAssembly.instantiate` (par `js_sys::WebAssembly`), en lui donnant **le même objet d'imports** que sur desktop,
construit par `argos-module-host`. Ce sont les mêmes droits et les mêmes refus.

- Mémoire : le module doit déclarer une mémoire **maximale**. Le navigateur la fait respecter. Un module sans maximum
  ou au-dessus de `memoire_mo` est refusé.
- Temps : le navigateur n'a pas de fuel. Le module tourne donc dans un **Web Worker**. Si `argos_run` dépasse
  `temps_ms`, le Worker est arrêté (`terminate`) et le module est marqué « trop lent ». La page ne gèle jamais.
- Réseau : `http_get` passe par l'app, qui vérifie net.rs, comme sur desktop. Le Worker n'a pas le droit de faire
  `fetch` lui-même : il ne reçoit que les imports de l'hôte.
- Fichiers : le dossier privé devient un espace de stockage de la page préfixé `argos:module:<id>:`.

### 5.3 Pareil partout

Les deux moteurs passent la même suite de tests de conformité. Elle charge des modules de test qui essaient
d'outrepasser leurs droits (réseau non listé, `..` dans un nom de fichier, boucle infinie, mémoire énorme, import
inconnu). Ils doivent tous être refusés ou arrêtés, de la même façon sur desktop et dans le navigateur.

## 6. Versions, audits et vote (lien avec Argos)

- Chaque version d'un module doit être **toute verte dans Cargo** avant le vote : `make vert` dans le dépôt du module
  (fmt, clippy, test, audit, deny), comme `make vert` d'Argos HSS Trading (commit a0215d4).
- L'empreinte `sha256` du `.wasm` est dans le manifeste. L'app refuse un `.wasm` dont l'empreinte ne correspond pas.
- Signature et audits : ils sont liés à la **clé publique** et à l'**Argos ID** de l'auteur. Ils ne sont vérifiés
  qu'avec la petite crate de vérification publiée par Argos, jamais avec le noyau privé (règle 6). Le vote des
  mainteneurs, la Marketplace et les commissions passent par Argos (règle 3). Argos HSS Trading ne fait que **vérifier**.
- Les IA attachées à un module ont une version figée, n'ont pas de clé et ne signent jamais. L'app n'a aucune fonction
  « signer » à leur offrir.

## 7. Transition des scripts actuels (sans rien casser)

| Phase | Ce qui change | Les scripts actuels |
|---|---|---|
| 0 (aujourd'hui) | Rien | L'interpréteur `argos-script::lang` tourne dans l'app. Il n'a déjà ni réseau, ni fichiers, ni clés : c'est une fonction pure (bougies → tracés). |
| 1 | `argos-module-sdk` + `argos-module-host` (desktop, wasmtime) + tests de conformité + un module d'exemple (un indicateur simple, par exemple RSI) | Inchangés |
| 2 | Moteur navigateur (Worker + WebAssembly) avec les mêmes tests | Inchangés |
| 3 | L'interpréteur Argos Script est compilé en **module officiel** (droits : aucun). L'éditeur envoie le texte du script comme donnée d'entrée. | Ils tournent dans le bac à sable, avec le même texte et les mêmes résultats. Une option « ancien moteur » reste un temps pour comparer. |
| 4 | La Marketplace n'accepte que des modules WASM avec manifeste vérifié. Des indicateurs officiels peuvent devenir des modules. | L'ancien moteur hors bac à sable est retiré. |

Chaque phase = une étape avec un commit local, `make vert` et un rapport, comme aujourd'hui.

## 8. Ce qu'il faudra ajouter (et ce que ça coûte)

- Nouvelles crates : `argos-module-sdk`, `argos-module-host`, avec des tests.
- Nouvelles dépendances :
  - `wasmtime` (desktop seulement, ajoute quelques Mo au binaire desktop) ;
  - `postcard` (petite).
  Les deux sont gratuites et open source, et leurs licences sont déjà acceptées.
- Aucun nouveau service, rien de payant. Aucune nouvelle adresse réseau : les modules n'ont droit qu'à celles de net.rs.

## 9. Questions ouvertes pour Jee

1. Le dossier privé d'un module : le garder entre deux versions du module, ou l'effacer à chaque mise à jour ?
2. Fenêtre des droits : faut-il redemander quand une nouvelle version **ajoute** un droit ? (Proposition : oui,
   toujours.)
3. IA dans les modules : quels modèles autoriser, et qui paie l'appel ? (Proposition : désactivé tant que l'étape 2 de
   l'assistant n'a pas de budget.)
4. Phase 3 : comparer l'ancien et le nouveau moteur pendant combien de versions avant de retirer l'ancien ?

## 10. Décisions (Jee, 2026-11-05)

Réponses de Jee aux questions du point 9 (« ok » aux 4 propositions) :

1. **Dossier privé** : il est **gardé** quand le module passe à une nouvelle version.
2. **Nouveaux droits** : une nouvelle version qui **ajoute** un droit redemande toujours l'accord de l'utilisateur.
   Sans accord, ce droit reste refusé.
3. **IA dans les modules** : **désactivée** tant qu'il n'y a pas de budget. L'import `argos_host::ai_ask` renvoie
   toujours « refusé ».
4. **Transition des scripts** : l'ancien moteur reste en place **pendant 2 versions**, en parallèle du nouveau (phase 3).
   Il est retiré ensuite (phase 4).

Le code des modules se fera **après** la file d'attente actuelle, sauf si Jee dit autrement.
