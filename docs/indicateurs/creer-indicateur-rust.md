# Créer un indicateur en Rust

22 nov. 2026 · v2

Deux façons d'écrire un indicateur en Rust : un module WASM que tout le monde peut charger, ou un indicateur natif compilé dans l'app. Le kit des modules n'est pas encore complet ; cette doc dit ce qui marche aujourd'hui et ce qui manque.

## Deux chemins

Un module WASM se charge dans Argos sans recompiler l'app, mais il ne dessine que des courbes, des étiquettes et des cartes ; un indicateur natif dessine tout, mais il demande de recompiler l'app.

|  | Module WASM | Indicateur natif |
| --- | --- | --- |
| Pour qui | Toute personne | Contributeur de l'app |
| Ce qu'on livre | Un fichier `.wasm` + un manifeste `.json` | Du code dans le dépôt |
| Recompiler l'app | Non | Oui |
| Ce qu'il dessine | 16 courbes, 1 000 étiquettes, 32 cartes | Tout : lignes, zones, formes, tableaux, pastilles, signaux |
| Réglages | 16 au plus, 4 types | Tous les types de la fenêtre ⚙ |
| Réseau, IA | Refusés dans Trading | Selon l'app |
| Isolé du reste | Oui : bulle wasmtime, temps et mémoire limités | Non |
| Bouton { } | Non | Pas encore : à ajouter à la main |

Plus simple que les deux : un indicateur en Argos Script (voir la doc « Argos Script — la référence »). C'est déjà le cas des moyennes SMA, WMA, HMA, VWMA, SMMA et du ruban de moyennes : un script Pine rangé dans l'app, qui donne réglages, dessin et bouton { } sans une ligne de code de rendu.

## Où en est le kit aujourd'hui

Un module se compile et se charge déjà dans Argos, sur PC et sur le web ; il manque l'outil officiel du kit, la signature des modules et le dessin des formes, tableaux et alertes.

Ce qui marche (relevé du 22 nov. 2026, kit 1.0.2 rangé dans le dépôt sous `rust/vendor/kit-module-argos-1.0.2`) :

- Compiler un module en `wasm32-unknown-unknown` avec cargo.
- L'installer depuis la page 🧩 Modules, le démarrer, l'arrêter, régler ses réglages, le retirer.
- Le faire tourner dans une bulle isolée : temps, mémoire et fichiers limités.
- Quatre modules d'exemple sont déjà dans l'app : moyenne, EMA, Académie (sessions de New York), pinceau de zone.

Ce qui manque :

| Manque | Conséquence aujourd'hui |
| --- | --- |
| L'outil `argos-modules` du kit (vérifie, compile deux fois, écrit la proposition) | On compile avec cargo directement |
| La signature des modules | L'accord se fait sur l'empreinte SHA-384 du fichier, acceptée à l'installation |
| Formes, tableaux, alertes dans la sortie d'un module | Un module ne sort que des courbes, des étiquettes et des cartes |
| Un sélecteur de fichier sur PC | On dépose `nom.wasm` + `nom.json` dans Téléchargements |
| Un modèle de projet hors du dépôt | On copie l'exemple du kit |
| La coque Tauri à jour | Elle est encore en wasmtime 50.0.1, l'app en 50.0.2 |

La séparation de l'app en modules ne change ni le manifeste ni l'API des modules. Elle changera plus tard la façon d'ajouter un indicateur natif (voir la partie Indicateur natif).

## Un module pas à pas

Un module, c'est un petit projet Rust qui exporte une seule fonction `run`, compilé en WASM, plus un manifeste JSON qui le décrit.

1. Installer Rust 1.99.1 et la cible WASM : `rustup target add wasm32-unknown-unknown`.
2. Copier l'exemple `exemple-moyenne/` du kit, à côté du dossier `argos-module-sdk/` : le chemin `../argos-module-sdk` du `Cargo.toml` doit rester juste.
3. Dans `Cargo.toml` : `crate-type = ["cdylib", "rlib"]`, `publish = false`, une section `[workspace]` vide. Dépendances : seulement celles de `allowed-dependencies.txt`, version exacte ; ni `build.rs`, ni dépendance git.
4. Dans `src/lib.rs` : `#![forbid(unsafe_code)]`, une fonction `fn run(input: Input) -> Output`, puis `argos_module_sdk::entry_data!(run);`.
5. Écrire le manifeste `argos-module.json` (tableau ci-dessous).
6. Compiler : `cargo build --release --target wasm32-unknown-unknown`. Le fichier sort dans `target/wasm32-unknown-unknown/release/ma_moyenne.wasm`.
7. Le nom du fichier prend des _ à la place des - : renomme-le comme le manifeste. Installer sur PC : déposer `<nom>.wasm` et `<nom>.json` dans Téléchargements, puis page 🧩 Modules → « 📂 Installer un module ». Sur le web : le même bouton ouvre un sélecteur de fichier.
8. Dans la page 🧩 Modules : démarrer le module, ouvrir ses réglages, lui donner ou non son dossier privé.

Le manifeste `argos-module.json` (tout champ inconnu est refusé) :

| Champ | Valeur | Règle dans Trading |
| --- | --- | --- |
| `path` | `"trading/ma-moyenne"` | Commence par `trading/` ; 64 caractères au plus, lettres, chiffres, `-`, `_`, `/` |
| `version` | `1` | Entier qui monte à chaque nouvelle version |
| `author` | Ton Argos ID | Obligatoire |
| `abi` | `2` | Obligatoire : 2 |
| `settings` | Liste de réglages | 16 au plus (partie Réglages) |
| `capabilities` | `[]`, `"ReadFiles"`, `"WriteFiles"` | `Network`, `Ai`, `Model` refusés |
| `budget_secs` | `2` par défaut | De 1 à 120 secondes |
| `subfolders` | Facultatif | Sous-dossiers permis du dossier privé |

`domains`, `ai_model` et `model` existent dans le kit pour d'autres projets, mais Trading les refuse.

## Ce qu'un module reçoit et renvoie

Le module reçoit les bougies et ses réglages en nombres entiers, et renvoie des courbes, des étiquettes et des cartes, aussi en entiers avec leur nombre de décimales.

Pourquoi des entiers : le calcul donne le même résultat sur PC, cell et web. Un prix de 60 123,45 avec `price_scale = 2` arrive comme `6012345`. Une valeur absente vaut `i64::MIN`.

Entrée, `Input` :

| Champ | Type | Sens |
| --- | --- | --- |
| `timeframe` | `u32` | Unité de temps en secondes (300 = 5 min) |
| `price_scale` | `u8` | Décimales des prix (8 au plus) |
| `volume_scale` | `u8` | Décimales des volumes (6 au plus) |
| `symbol` | `String` | Le symbole de la chart |
| `bars` | `Vec<Bar>` | 50 000 bougies au plus, les plus anciennes coupées ; `Bar { t, o, h, l, c, v }` en `i64`, `t` toujours croissant |
| `settings` | `Vec<i64>` | Les réglages, dans l'ordre du manifeste |

Sortie, `Output` :

| Champ | Type | Limite |
| --- | --- | --- |
| `lines` | `Vec<Line>` : `name`, `scale`, `values` | 16 courbes, une valeur par bougie |
| `labels` | `Vec<Label>` : `bar`, `text` | 1 000 étiquettes, 256 octets de texte |
| `cards` | `Vec<Card>` : `name`, `value`, `scale` | 32 cartes (une valeur affichée à part) |

Limites de la bulle :

| Limite | Valeur |
| --- | --- |
| Temps | `budget_secs` du manifeste, 2 s par défaut |
| Calcul | 200 000 000 unités de carburant par appel |
| Mémoire | 16 Mio |
| Entrée | 4 Mio |
| Dossier privé | 1 Mio, seulement si l'utilisateur le donne |
| Web | Le module tourne dans un Web Worker, arrêté après le budget + 300 ms |

Codes d'erreur rendus par le module : -1 entrée invalide, -2 sortie trop grande, -3 mémoire.

## Les réglages d'un module

Les réglages se déclarent dans le manifeste ; Argos dessine la fenêtre ⚙ tout seul et passe les valeurs au module dans `input.settings`, dans le même ordre.

```json
"settings": [
  { "id": "length", "name": "Longueur", "type": "number", "default": 20, "min": 1, "max": 500 },
  { "id": "mult",   "name": "Écart",    "type": "number", "default": 200, "min": 10, "max": 1000, "scale": 2 },
  { "id": "fill",   "name": "Remplir",  "type": "bool",   "default": true },
  { "id": "col",    "name": "Couleur",  "type": "color",  "default": "#2f8cff" },
  { "id": "src",    "name": "Source",   "type": "choice", "default": "close", "choices": ["close", "hl2", "hlc3"] }
]
```

| Type | Champs | Ce que reçoit le module |
| --- | --- | --- |
| `number` | `default`, `min`, `max` entiers ; `scale` de 0 à 6 | L'entier : avec `scale = 2`, 200 veut dire 2,00 |
| `bool` | `default` | 0 ou 1 |
| `color` | `default` en `"#rrggbb"` | `0xRRGGBB` |
| `choice` | `default`, `choices` (1 à 32, tous différents) | Le numéro du choix, 0 pour le premier |

Règles : 16 réglages au plus ; `id` de 1 à 32 caractères (lettres, chiffres, `-`, `_`), unique ; `name` et chaque choix de 1 à 64 octets ; `min ≤ default ≤ max`. Argos vérifie les valeurs avant chaque appel : le module reçoit toujours des valeurs dans les bornes.

## Exemple complet : une moyenne mobile

Une moyenne simple réglable, une courbe sur le prix et une carte avec la dernière valeur. Version courte de `exemple-moyenne` du kit, qui reste la référence (il arrondit au plus proche et ajoute un réglage Source) ; compilée sans changement le 22 nov. 2026 (ma_moyenne.wasm, 32 549 octets).

`argos-module.json` :

```json
{
  "path": "trading/ma-moyenne",
  "version": 1,
  "author": "<ton Argos ID>",
  "abi": 2,
  "settings": [
    { "id": "length", "name": "Longueur", "type": "number", "default": 20, "min": 1, "max": 500 }
  ]
}
```

`src/lib.rs` :

```rust
#![forbid(unsafe_code)]
use argos_module_sdk::{Card, Input, Line, Output, EMPTY}; // EMPTY = valeur absente

pub fn run(input: Input) -> Output {
    // Réglage 0 = Longueur (number, scale 0)
    let len = input.settings.first().copied().unwrap_or(20).clamp(1, 500) as usize;
    let closes: Vec<i64> = input.bars.iter().map(|b| b.c).collect();
    let mut values = vec![EMPTY; closes.len()];
    let mut sum: i128 = 0;
    for (i, c) in closes.iter().enumerate() {
        sum += i128::from(*c);
        if i >= len {
            sum -= i128::from(closes[i - len]);
        }
        if i + 1 >= len {
            values[i] = (sum / len as i128) as i64;
        }
    }
    let last = values.last().copied().unwrap_or(EMPTY);
    Output {
        lines: vec![Line { name: "Moyenne".into(), scale: input.price_scale, values }],
        labels: vec![],
        cards: vec![Card { name: "Dernière moyenne".into(), value: last, scale: input.price_scale }],
    }
}

argos_module_sdk::entry_data!(run);
```

`Cargo.toml` (extrait) :

```toml
[package]
name = "ma-moyenne"
version = "0.1.0"
edition = "2021"
publish = false

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
argos-module-sdk = { path = "../argos-module-sdk" }

[workspace]
```

Points à retenir :

- La courbe garde l'échelle des prix (`price_scale`) : Argos la place sur le prix sans conversion.
- La somme est en `i128` pour ne jamais déborder sur 500 bougies de gros prix.
- Les premières bougies, avant `len`, restent `EMPTY` : rien n'est dessiné là.

## Indicateur natif dans l'app

Un indicateur natif n'implémente aucun trait : c'est un calcul pur dans `argos-script`, une entrée à la fin de l'énumération `Ind`, une fiche au catalogue et un dessin dans `argos-render`.

Étapes pour ajouter `Foo` (exemples à lire : le Tableau multi-UT, `argos-script/src/indicators/mtf_table.rs`, 137 lignes, et le croisement d'EMA, `ema_cross_n` dans `classic.rs`) :

1. Calcul : `argos-script/src/indicators/foo.rs` avec `GROUPS`, `PARAMS`, une fonction `compute(bars, …)` et ses tests ; ajouter `pub mod foo;`.
2. Rendu : `Foo` à la fin de `Ind` et de `Ind::ALL` (`argos-render/src/indicators/mod.rs`). Pour des lignes simples, une branche dans `plot()` (`classic_plots.rs`) suffit : légende, signaux et tableau résumé viennent tout seuls. Sinon un champ dans `IndOut`, plus `compute` et `draw`.
3. Catalogue : une fiche à la fin de `FICHES` et une catégorie dans `CATEGORIES_FX` (`argos-script/src/catalogue.rs`) ; mots de recherche dans `MOTS_CLES` si besoin. Les menus PC et cell se remplissent tout seuls.
4. Bouton { } : ajouter `Ind::Foo => include_str!("…")` dans `argos-render/src/sources.rs`. Sans cette ligne, pas de bouton : c'est le cas aujourd'hui des 40 indicateurs Rust.
5. Facultatif : nom anglais dans `DICT` (`i18n.rs`), indicateur par défaut, lecture par Argos AI.
6. Mettre à jour les comptes écrits en dur dans les tests, puis `cd rust && make check`.

Réglages : chaque réglage est une paire (clé, valeur `f64`). Constructeurs : `int`, `num` (min, max, pas), `check`, `choice`, `color`, `color_t`, `price`, `time`, `session`, `string`, `text_area`, `symbol`, `source`, avec les options `.tip`, `.inline`, `.active_if`, `.display`, `.confirm`. La fenêtre ⚙ est générée, sur PC et sur cell.

| Code Source | Valeur |
| --- | --- |
| 0 | Clôture |
| 1 | Ouverture |
| 2 | Haut |
| 3 | Bas |
| 4 | (H+B)/2 |
| 5 | (H+B+C)/3 |
| 6 | (O+H+B+C)/4 |
| 7 | Volume |
| 8 | (H+B+C+C)/4 |
| 1000 et plus | Sortie d'un autre indicateur |

Dessin (`argos-render`) : lignes `poly_trait`, zones `zone` et `cloud`, points `dot`, texte `Text::draw`, tableaux `table()`, pastilles `shapes::pilule`. Pour ne rien chevaucher : `poser_tableau`, `reserver` / `hors_reserve`, `eviter()`. Couleurs : `palette::up()` / `down()`, `on_chart(c)` pour le thème clair et le mode daltonien.

Trois règles fixes :

- Toujours ajouter à la fin : la position sert d'identifiant (réglages, sources).
- Ne jamais renommer une variante ni une clé de réglage : les réglages des gens sont gardés par nom.
- Rust pur, sans fichier ni thread dans le calcul : le même code tourne sur le web.

À venir avec la séparation en modules : un indicateur natif se déclarera par un registre (plus d'énumération à la main) et ses fichiers iront dans `rust/chart-et-analyse/h-argos-script/`. Cette partie sera mise à jour à ce moment.

## Questions fréquentes

Les réponses viennent du code de l'app au 22 nov. 2026.

| Question | Réponse |
| --- | --- |
| Module ou natif ? | Module : pour tout le monde, sans recompiler l'app, sortie limitée. Natif : pour les contributeurs, tout le dessin, l'app se recompile. |
| Faut-il un trait ? | Non. Module : une fonction `run` + `entry_data!(run)`. Natif : des fonctions + l'énumération `Ind`. |
| Quelle cible ? | `wasm32-unknown-unknown`, en `--release`. |
| Pourquoi des entiers dans un module ? | Pour le même résultat partout. Chaque courbe dit son nombre de décimales (`scale`). |
| Combien de bougies reçoit un module ? | 50 000 au plus ; les plus anciennes sont coupées. |
| Un module peut-il aller sur Internet ou appeler une IA ? | Non dans Trading. Seul le dossier privé (1 Mio) peut être donné. |
| Faut-il signer un module ? | Pas encore : l'accord se fait sur l'empreinte SHA-384 à l'installation. |
| Quand est-ce recalculé ? | À chaque nouvelle bougie ou clôture qui change, au plus toutes les 500 ms sur PC et 1,5 s sur cell. Le dessin, lui, se refait à chaque image. |
| Un natif tourne-t-il sur le web ? | Oui : le calcul et le dessin sont en Rust pur, vérifiés pour wasm32. |
| Signaux achat / vente gratuits ? | Oui pour un natif à lignes : `signals(crosses(a, b), "Achat", "Vente")` dans `plot()`. |
| Tableau résumé gratuit ? | Oui : remplir `Plot.tableau` ; position et taille réglables. |
| Peut-on renommer une variante ou une clé ? | Non : les réglages des gens sont gardés par nom. On remplace par `CONVERSIONS`. |
| Plus simple que Rust ? | Oui : un script Argos Script rangé comme fiche du catalogue donne réglages, dessin et bouton { } sans code de rendu. |

Pour une IA : donne-lui cette doc et le fichier `2026-11-22-argos-script-llms-v1.txt` (dans `documents/` du PC), et pour Rust le fichier `2026-11-22-creer-indicateur-rust-llms-v2.txt` du même dossier.
