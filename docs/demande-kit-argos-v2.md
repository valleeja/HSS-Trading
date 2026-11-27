# Demande pour le kit Argos : kit module v2 (données d'entrée + réglages)

> De : Argos HSS Trading, le 2026-11-06. Pour : la session du kit Argos, qui fait la v2 du kit.
> Argos HSS Trading ne modifie pas le kit : il le lit (copie figée) et suit sa version.
> Contexte : `docs/plan-migration-kit-argos.md`.

## Pourquoi

Avec le kit 0.2.0, l'entrée d'un module est `run() -> i32` : il ne reçoit aucune donnée et ne rend qu'un entier.
Un module d'Argos HSS Trading est surtout un **indicateur de chart** : il doit recevoir les bougies et rendre des
courbes. Aujourd'hui c'est impossible sans détour (passer par des fichiers, ce qui donne des droits fichiers à
chaque indicateur : à éviter).

## Ce qu'on demande

### 1. Une entrée avec données (en plus de `run`)

- Export `run_data(ptr: i32, len: i32) -> i64` : l'hôte écrit l'entrée dans la mémoire du module, le module rend
  sa sortie (pointeur sur 32 bits hauts, longueur sur 32 bits bas ; une valeur négative = code d'erreur
  `HOST_*`).
- Exports `alloc(len: i32) -> i32` et `free(ptr: i32, len: i32)` pour que l'hôte place l'entrée et reprenne la
  sortie. Toujours la seule surface `unsafe`, dans le SDK (les modules restent `#![forbid(unsafe_code)]`).
- Macro `entry_data!(ma_fonction)` avec `fn ma_fonction(input: Input) -> Output`.
- Format : **postcard** (binaire compact, déterministe), version d'interface dans l'entrée.
- Taille bornée par l'hôte (par exemple 16 Mo en entrée, 16 Mo en sortie), budget de temps inchangé.

### 2. Les types d'un indicateur (à mettre dans le SDK, libre)

- `Input` : version, symbole, unité de temps (s), bougies (`t`, `o`, `h`, `l`, `c`, `v`), réglages (liste de
  nombres, dans l'ordre du manifeste).
- `Output` : courbes (nom + une valeur par bougie, `NaN` = vide), étiquettes (bougie + texte), lignes « nom ·
  valeur » pour une carte du tableau de bord. Champs ajoutés plus tard = en fin de structure, avec une
  valeur par défaut.
- Aucun accès réseau, fichier ni IA n'est nécessaire pour un indicateur : il garde **aucun droit**.

### 3. Des réglages déclarés dans le manifeste

Dans `argos-module.json`, une liste `settings` que l'app transforme en fenêtre de réglages :

```json
"settings": [
  { "id": "length", "name": "Longueur", "type": "number", "default": 20, "min": 1, "max": 500 },
  { "id": "color", "name": "Couleur de la ligne", "type": "color", "default": "#2962ff" },
  { "id": "mode", "name": "Mode", "type": "choice", "choices": ["a", "b"], "default": "a" },
  { "id": "show", "name": "Afficher", "type": "bool", "default": true }
]
```

- Types : `number` (avec `min` / `max`), `bool`, `color` (« #rrggbb »), `choice`.
- Identifiants stables (lettres, chiffres, `-`, `_`), uniques, 16 réglages au plus ; défaut obligatoire et du
  bon type, sinon le manifeste est refusé.
- La valeur choisie par l'utilisateur va au module dans `Input.settings` (nombre ; case 0 / 1 ; couleur
  0xRRGGBB ; choix = son index). Elle est gardée par l'app d'une version du module à l'autre.
- Les réglages ne sont **pas** des droits : ils ne changent rien à la fiche de droits ni à l'approbation.

### 4. Pour le navigateur

`argos-sandbox` ne compile pas en WASM (`ureq`, `rustix`). Pour que l'app web de Trading tourne aussi, il faut que
la v2 soit décrite assez précisément (noms, signatures, codes `HOST_*`, refus par défaut, limites) pour qu'un
moteur du navigateur (Web Worker) l'applique à l'identique, avec une suite de tests de conformité commune.

## Ce que Trading fera ensuite

Une fois la v2 publiée : copie figée du kit dans le dépôt (empreintes vérifiées), notre `argos-module-sdk` retiré,
la page 🧩 Modules et le module d'exemple « Moyenne » passés au kit, le Worker du navigateur aligné sur la v2.

## Exemple : le module « Moyenne » de Trading, écrit pour la v2

```rust
#![forbid(unsafe_code)]
use argos_module_sdk::{entry_data, Input, Line, Output};

fn run(i: Input) -> Output {
    let n = i.settings.first().copied().unwrap_or(20.0).max(1.0) as usize; // réglage « length »
    let close: Vec<f64> = i.bars.iter().map(|b| b.c).collect();
    let values = (0..close.len())
        .map(|k| if k + 1 < n { f64::NAN } else { close[k + 1 - n..=k].iter().sum::<f64>() / n as f64 })
        .collect();
    Output { lines: vec![Line { name: format!("Moyenne {n}"), values }], ..Default::default() }
}
entry_data!(run);
```

```json
{
  "path": "trading/moyenne",
  "version": 1,
  "capabilities": [],
  "author": "ARG-XXXX-0000",
  "budget_secs": 2,
  "settings": [
    { "id": "length", "name": "Longueur", "type": "number", "default": 20, "min": 1, "max": 500 },
    { "id": "color", "name": "Couleur de la ligne", "type": "color", "default": "#2962ff" }
  ]
}
```

- Entrée d'un appel : 5000 bougies ≈ 240 Ko en postcard ; sortie : une courbe de 5000 valeurs ≈ 40 Ko.
- L'app applique elle-même la couleur à la courbe (la couleur ne change pas le calcul du module).
- Aucun droit : la fiche de droits dit « n'a accès ni à Internet, ni à tes fichiers, ni à l'IA ».
