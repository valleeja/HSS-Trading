# Dépendances ajoutées depuis le début de travail-local

> ✅ **Approuvée par Jee le 6 nov. 2026** (« ok tout »), après la revue du cœur : toutes acceptées, avec 4 corrections
> (kit 1.0.2, options réseau de Tauri coupées, `cargo deny` plus strict, crates à surveiller ci-dessous). Toute
> dépendance qui n'est pas dans cette liste demande encore l'accord de Jee **avant** d'être ajoutée.
>
> Depuis la liste : options par défaut de Tauri coupées (`dynamic-acl`, `dbus`) → **dbus 0.9.12 et libdbus-sys 0.2.7
> retirées** de la coque.

## 👀 À surveiller (crates plus maintenues ou à risque)

Gardées pour l'instant, chacune notée dans `deny.toml` (exception avec sa raison) ; à revoir à chaque mise à jour.

| Crate | Où | Avis | Pourquoi on la garde | Remplaçant possible |
|---|---|---|---|---|
| ttf-parser | app (fontdue, winit) | RUSTSEC-2026-0192, plus maintenue | lecteur de polices du texte d'argos-render et de la barre de titre Wayland ; aucune faille | `skrifa` / `read-fonts` (projet fontations), via une version de fontdue ou un moteur de texte qui l'utilise |
| zbus_names | app (keyring 3 → zbus 4) | RUSTSEC-2026-0371, plus maintenue | trousseau du système sous Linux pour les clés API du desktop ; aucune faille | keyring 4 (zbus 5) quand il sera stable |
| zvariant | app (keyring 3 → zbus 4) | RUSTSEC-2026-0370, plus maintenue | idem | keyring 4 (zbus 5) |
| zvariant_derive | app (keyring 3 → zbus 4) | RUSTSEC-2026-0372, plus maintenue | idem | keyring 4 (zbus 5) |
| proc-macro-error | coque (glib-macros 0.18, GTK 3 de Tauri sous Linux) | RUSTSEC-2024-0370, plus maintenue | macro de compilation seulement ; Tauri 2 utilise GTK 3 sous Linux | `proc-macro-error2` (suite maintenue), qui arrivera quand Tauri passera à GTK 4 / glib ≥ 0.20 |

Aussi noté, sans être « plus maintenu » :
- **glib 0.18** (coque, Linux) : RUSTSEC-2024-0429 (« unsound » dans un itérateur `VariantStrIter` qu'on n'utilise
  pas) ; ignoré seulement dans l'audit de la coque (`make tauri-check`). Partira avec GTK 4.
- **reqwest 0.13** (coque) : demandé par Tauri **seulement pour Android et iPhone** (dépendance fixe de Tauri sur ces
  cibles, aucune option ne la retire) ; pas compilé sur PC. La page web n'y a pas accès : tout le réseau d'Argos
  passe par la couche de données (`argos-data`, reqwest 0.12) dans le cœur Rust.

> Généré le 2026-11-06 à partir des fichiers `Cargo.lock` : base = commit `2963911` (dernier `main` commun, 28 oct.), 
> comparé à l'app (`rust/Cargo.lock`) et à la coque Tauri (`rust/argos-tauri/Cargo.lock`). Une ligne = une crate dans une 
> version qui n'était pas là au départ. « directe » = déclarée par nos crates ou par le kit module Argos ; « transitive » = 
> amenée par une autre.

**Règles (Jee, 6 nov.)** : MPL-2.0 acceptée ; GPL, AGPL ou toute licence qui oblige à ouvrir l'app : interdite. 
`cargo deny` (app et coque) le fait respecter : seules les licences de la liste `allow` passent.

**Total : 289 crates (7 directes, 282 transitives).** 
Licences interdites trouvées : **aucune**.

| Licence | Crates |
|---|---|
| MIT OR Apache-2.0 | 117 |
| MIT | 70 |
| Apache-2.0 OR MIT | 29 |
| Apache-2.0 WITH LLVM-exception | 28 |
| Zlib OR Apache-2.0 OR MIT | 9 |
| MIT/Apache-2.0 | 8 |
| Unlicense OR MIT | 7 |
| MPL-2.0 | 5 |
| Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | 4 |
| BSD-3-Clause | 2 |
| Apache-2.0 | 1 |
| BSD-2-Clause OR MIT OR Apache-2.0 | 1 |
| BSD-3-Clause/MIT | 1 |
| Apache-2.0/MIT | 1 |
| BSD-3-Clause AND MIT | 1 |
| CC0-1.0 OR MIT-0 OR Apache-2.0 | 1 |
| Apache-2.0 / MIT | 1 |
| MIT OR Apache-2.0 OR Zlib | 1 |
| ISC | 1 |
| Zlib | 1 |

## Kit Argos : la bulle argos-sandbox — wasmtime 50.0.1 (exécuter un module WASM avec carburant, mémoire et temps limités), ureq et rustix (services du kit : fichiers privés, réseau refusé chez nous)

65 crates : 4 directe(s), 61 transitive(s).

| Crate | Version | Licence | Lien | Déjà là (autre version) |
|---|---|---|---|---|
| addr2line | 0.26.1 | Apache-2.0 OR MIT | transitive |  |
| anyhow | 1.0.104 | MIT OR Apache-2.0 | transitive |  |
| arbitrary | 1.5.0 | MIT OR Apache-2.0 | transitive |  |
| cc | 1.11.1 | MIT OR Apache-2.0 | transitive | oui |
| cobs | 0.3.0 | MIT OR Apache-2.0 | transitive |  |
| cpp_demangle | 0.5.1 | MIT OR Apache-2.0 | transitive |  |
| cranelift-assembler-x64 | 0.137.1 | Apache-2.0 WITH LLVM-exception | transitive |  |
| cranelift-assembler-x64-meta | 0.137.1 | Apache-2.0 WITH LLVM-exception | transitive |  |
| cranelift-bforest | 0.137.1 | Apache-2.0 WITH LLVM-exception | transitive |  |
| cranelift-bitset | 0.137.1 | Apache-2.0 WITH LLVM-exception | transitive |  |
| cranelift-codegen | 0.137.1 | Apache-2.0 WITH LLVM-exception | transitive |  |
| cranelift-codegen-meta | 0.137.1 | Apache-2.0 WITH LLVM-exception | transitive |  |
| cranelift-codegen-shared | 0.137.1 | Apache-2.0 WITH LLVM-exception | transitive |  |
| cranelift-control | 0.137.1 | Apache-2.0 WITH LLVM-exception | transitive |  |
| cranelift-entity | 0.137.1 | Apache-2.0 WITH LLVM-exception | transitive |  |
| cranelift-frontend | 0.137.1 | Apache-2.0 WITH LLVM-exception | transitive |  |
| cranelift-isle | 0.137.1 | Apache-2.0 WITH LLVM-exception | transitive |  |
| cranelift-native | 0.137.1 | Apache-2.0 WITH LLVM-exception | transitive |  |
| cranelift-srcgen | 0.137.1 | Apache-2.0 WITH LLVM-exception | transitive |  |
| crc32fast | 1.5.4 | MIT OR Apache-2.0 | transitive | oui |
| either | 1.19.0 | MIT OR Apache-2.0 | transitive |  |
| embedded-io | 0.4.0 | MIT OR Apache-2.0 | transitive |  |
| embedded-io | 0.6.1 | MIT OR Apache-2.0 | transitive |  |
| fnv | 1.0.7 | Apache-2.0 / MIT | transitive |  |
| foldhash | 0.2.0 | Zlib | transitive | oui |
| futures | 0.3.34 | MIT OR Apache-2.0 | transitive |  |
| gimli | 0.33.0 | MIT OR Apache-2.0 | transitive |  |
| hashbrown | 0.16.1 | MIT OR Apache-2.0 | transitive | oui |
| heck | 0.5.0 | MIT OR Apache-2.0 | transitive |  |
| itertools | 0.14.0 | MIT OR Apache-2.0 | transitive |  |
| js-sys | 0.3.107 | MIT OR Apache-2.0 | transitive | oui |
| leb128fmt | 0.1.0 | MIT OR Apache-2.0 | transitive |  |
| log | 0.4.35 | MIT OR Apache-2.0 | transitive | oui |
| mach2 | 0.6.0 | BSD-2-Clause OR MIT OR Apache-2.0 | transitive |  |
| memfd | 0.6.6 | MIT OR Apache-2.0 | transitive |  |
| object | 0.40.0 | Apache-2.0 OR MIT | transitive |  |
| postcard | 1.1.3 | MIT OR Apache-2.0 | transitive |  |
| pulley-interpreter | 50.0.1 | Apache-2.0 WITH LLVM-exception | transitive |  |
| pulley-macros | 50.0.1 | Apache-2.0 WITH LLVM-exception | transitive |  |
| regalloc2 | 0.15.2 | Apache-2.0 WITH LLVM-exception | transitive |  |
| rustc-demangle | 0.1.28 | MIT/Apache-2.0 | transitive |  |
| rustix | 1.1.6 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | directe | oui |
| smallvec | 1.16.5 | MIT OR Apache-2.0 | transitive | oui |
| target-lexicon | 0.13.5 | Apache-2.0 WITH LLVM-exception | transitive |  |
| termcolor | 1.4.1 | Unlicense OR MIT | transitive |  |
| ureq | 3.5.0 | MIT OR Apache-2.0 | directe |  |
| ureq-proto | 0.6.5 | MIT OR Apache-2.0 | transitive |  |
| utf8-zero | 0.8.1 | MIT OR Apache-2.0 | transitive |  |
| wasm-bindgen | 0.2.130 | MIT OR Apache-2.0 | transitive | oui |
| wasm-bindgen-macro | 0.2.130 | MIT OR Apache-2.0 | transitive | oui |
| wasm-bindgen-macro-support | 0.2.130 | MIT OR Apache-2.0 | transitive | oui |
| wasm-bindgen-shared | 0.2.130 | MIT OR Apache-2.0 | transitive | oui |
| wasm-encoder | 0.261.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | transitive |  |
| wasmparser | 0.261.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | directe |  |
| wasmprinter | 0.261.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | transitive |  |
| wasmtime | 50.0.1 | Apache-2.0 WITH LLVM-exception | directe |  |
| wasmtime-environ | 50.0.1 | Apache-2.0 WITH LLVM-exception | transitive |  |
| wasmtime-internal-component-util | 50.0.1 | Apache-2.0 WITH LLVM-exception | transitive |  |
| wasmtime-internal-core | 50.0.1 | Apache-2.0 WITH LLVM-exception | transitive |  |
| wasmtime-internal-cranelift | 50.0.1 | Apache-2.0 WITH LLVM-exception | transitive |  |
| wasmtime-internal-fiber | 50.0.1 | Apache-2.0 WITH LLVM-exception | transitive |  |
| wasmtime-internal-jit-debug | 50.0.1 | Apache-2.0 WITH LLVM-exception | transitive |  |
| wasmtime-internal-jit-icache-coherence | 50.0.1 | Apache-2.0 WITH LLVM-exception | transitive |  |
| wasmtime-internal-unwinder | 50.0.1 | Apache-2.0 WITH LLVM-exception | transitive |  |
| wasmtime-internal-versioned-export-macros | 50.0.1 | Apache-2.0 WITH LLVM-exception | transitive |  |

## argos-scanner : reqwest bloquant + serde (outil en ligne de commande du scanner, option « cli »)

4 crates : 0 directe(s), 4 transitive(s).

| Crate | Version | Licence | Lien | Déjà là (autre version) |
|---|---|---|---|---|
| utf8_iter | 1.1.0 | Apache-2.0 OR MIT | transitive | oui |
| utf_types | 1.0.0 | Apache-2.0 OR MIT | transitive |  |
| wasm-bindgen-futures | 0.4.80 | MIT OR Apache-2.0 | transitive | oui |
| web-sys | 0.3.107 | MIT OR Apache-2.0 | transitive | oui |

## Coque Tauri 2 (rust/argos-tauri, pas dans l'app native) : tauri 2, tauri-build, wry / tao (fenêtre et vue web), WebKitGTK et GTK 3 sous Linux, muda (menus), CSP (cssparser, selectors)

219 crates : 3 directe(s), 216 transitive(s).

| Crate | Version | Licence | Lien | Déjà là (autre version) |
|---|---|---|---|---|
| aho-corasick | 1.1.5 | Unlicense OR MIT | transitive |  |
| alloc-no-stdlib | 3.0.0 | BSD-3-Clause | transitive |  |
| alloc-stdlib | 0.3.0 | BSD-3-Clause | transitive |  |
| atk | 0.18.2 | MIT | transitive |  |
| atk-sys | 0.18.2 | MIT | transitive |  |
| base64 | 0.21.7 | MIT OR Apache-2.0 | transitive | oui |
| bit-set | 0.8.0 | Apache-2.0 OR MIT | transitive |  |
| bit-vec | 0.8.0 | Apache-2.0 OR MIT | transitive |  |
| block2 | 0.6.2 | MIT | transitive | oui |
| brotli | 9.1.1 | BSD-3-Clause AND MIT | transitive |  |
| brotli-decompressor | 6.3.0 | BSD-3-Clause/MIT | transitive |  |
| bs58 | 0.5.1 | MIT/Apache-2.0 | transitive |  |
| cairo-rs | 0.18.5 | MIT | transitive |  |
| cairo-sys-rs | 0.18.2 | MIT | transitive |  |
| camino | 1.2.6 | MIT OR Apache-2.0 | transitive |  |
| cargo-platform | 0.1.9 | MIT OR Apache-2.0 | transitive |  |
| cargo_metadata | 0.19.2 | MIT | transitive |  |
| cargo_toml | 1.0.1 | Apache-2.0 OR MIT | transitive |  |
| cesu8 | 1.1.0 | Apache-2.0/MIT | transitive |  |
| cfb | 0.14.0 | MIT | transitive |  |
| cfg-expr | 0.15.8 | MIT OR Apache-2.0 | transitive |  |
| cookie | 0.18.2 | MIT OR Apache-2.0 | transitive |  |
| core-graphics | 0.25.0 | MIT OR Apache-2.0 | transitive | oui |
| core-graphics-types | 0.2.0 | MIT OR Apache-2.0 | transitive | oui |
| crossbeam-channel | 0.5.17 | MIT OR Apache-2.0 | transitive |  |
| crypto-bigint | 0.7.10 | Apache-2.0 OR MIT | transitive | oui |
| cssparser | 0.37.0 | MPL-2.0 | transitive |  |
| cssparser-macros | 0.7.1 | MPL-2.0 | transitive |  |
| ctor | 1.0.13 | Apache-2.0 OR MIT | transitive | oui |
| ctutils | 0.4.8 | Apache-2.0 OR MIT | transitive | oui |
| darling | 0.24.1 | MIT | transitive |  |
| darling_core | 0.24.1 | MIT | transitive |  |
| darling_macro | 0.24.1 | MIT | transitive |  |
| defmt | 1.2.0 | MIT OR Apache-2.0 | transitive |  |
| defmt-macros | 1.2.0 | MIT OR Apache-2.0 | transitive |  |
| defmt-parser | 1.1.0 | MIT OR Apache-2.0 | transitive |  |
| der | 0.8.3 | Apache-2.0 OR MIT | transitive | oui |
| deranged | 0.5.9 | MIT OR Apache-2.0 | transitive |  |
| derive_more | 2.1.1 | MIT | transitive |  |
| derive_more-impl | 2.1.1 | MIT | transitive |  |
| dirs | 7.0.0 | MIT OR Apache-2.0 | transitive |  |
| dirs-sys | 0.5.0 | MIT OR Apache-2.0 | transitive |  |
| dlopen2 | 0.8.2 | MIT | transitive |  |
| dlopen2_derive | 0.4.3 | MIT | transitive |  |
| dom_query | 0.28.0 | MIT | transitive |  |
| dtoa | 1.0.11 | MIT OR Apache-2.0 | transitive |  |
| dtoa-short | 0.3.5 | MPL-2.0 | transitive |  |
| dunce | 1.0.5 | CC0-1.0 OR MIT-0 OR Apache-2.0 | transitive |  |
| dyn-clone | 1.0.20 | MIT OR Apache-2.0 | transitive |  |
| elliptic-curve | 0.14.3 | Apache-2.0 OR MIT | transitive | oui |
| embed-resource | 3.0.12 | MIT | transitive |  |
| embed_plist | 1.2.2 | MIT OR Apache-2.0 | transitive |  |
| erased-serde | 0.4.11 | MIT OR Apache-2.0 | transitive |  |
| field-offset | 0.3.6 | MIT OR Apache-2.0 | transitive |  |
| flate2 | 1.1.11 | MIT OR Apache-2.0 | directe | oui |
| futures-executor | 0.3.34 | MIT OR Apache-2.0 | transitive |  |
| gdk | 0.18.2 | MIT | transitive |  |
| gdk-pixbuf | 0.18.5 | MIT | transitive |  |
| gdk-pixbuf-sys | 0.18.0 | MIT | transitive |  |
| gdk-sys | 0.18.2 | MIT | transitive |  |
| gdkwayland-sys | 0.18.2 | MIT | transitive |  |
| gdkx11 | 0.18.2 | MIT | transitive |  |
| gdkx11-sys | 0.18.2 | MIT | transitive |  |
| gio | 0.18.4 | MIT | transitive |  |
| gio-sys | 0.18.1 | MIT | transitive |  |
| glib | 0.18.5 | MIT | transitive |  |
| glib-macros | 0.18.5 | MIT | transitive |  |
| glib-sys | 0.18.1 | MIT | transitive |  |
| glob | 0.3.4 | MIT OR Apache-2.0 | transitive |  |
| gobject-sys | 0.18.0 | MIT | transitive |  |
| gtk | 0.18.2 | MIT | transitive |  |
| gtk-sys | 0.18.2 | MIT | transitive |  |
| gtk3-macros | 0.18.2 | MIT | transitive |  |
| hashbrown | 0.12.3 | MIT OR Apache-2.0 | transitive | oui |
| heck | 0.4.1 | MIT OR Apache-2.0 | transitive |  |
| html5ever | 0.39.0 | MIT OR Apache-2.0 | transitive |  |
| ico | 0.6.0 | MIT | transitive |  |
| ident_case | 1.0.1 | MIT/Apache-2.0 | transitive |  |
| indexmap | 1.9.3 | Apache-2.0 OR MIT | transitive | oui |
| infer | 0.22.0 | MIT | transitive |  |
| javascriptcore-rs | 1.1.2 | MIT | transitive |  |
| javascriptcore-rs-sys | 1.1.1 | MIT | transitive |  |
| jiff | 0.2.39 | Unlicense OR MIT | transitive |  |
| jiff-core | 0.1.2 | Unlicense OR MIT | transitive |  |
| jiff-static | 0.2.39 | Unlicense OR MIT | transitive |  |
| jiff-tzdb | 0.1.9 | Unlicense OR MIT | transitive |  |
| jiff-tzdb-platform | 0.1.3 | Unlicense OR MIT | transitive |  |
| jni | 0.21.1 | MIT/Apache-2.0 | transitive | oui |
| json-patch | 4.2.0 | MIT/Apache-2.0 | transitive |  |
| jsonptr | 0.7.1 | MIT OR Apache-2.0 | transitive |  |
| keyboard-types | 0.8.3 | MIT OR Apache-2.0 | transitive |  |
| libappindicator | 0.9.0 | Apache-2.0 OR MIT | transitive |  |
| libappindicator-sys | 0.9.0 | Apache-2.0 OR MIT | transitive |  |
| libloading | 0.7.4 | ISC | transitive | oui |
| markup5ever | 0.39.0 | MIT OR Apache-2.0 | transitive |  |
| mime | 0.3.18 | MIT OR Apache-2.0 | transitive |  |
| muda | 0.20.0 | Apache-2.0 OR MIT | transitive |  |
| new_debug_unreachable | 1.0.6 | MIT | transitive |  |
| num-conv | 0.2.2 | MIT OR Apache-2.0 | transitive |  |
| objc2-cloud-kit | 0.3.2 | Zlib OR Apache-2.0 OR MIT | transitive | oui |
| objc2-core-data | 0.3.2 | Zlib OR Apache-2.0 OR MIT | transitive | oui |
| objc2-core-image | 0.3.2 | Zlib OR Apache-2.0 OR MIT | transitive | oui |
| objc2-core-location | 0.3.2 | Zlib OR Apache-2.0 OR MIT | transitive | oui |
| objc2-core-text | 0.3.2 | Zlib OR Apache-2.0 OR MIT | transitive |  |
| objc2-exception-helper | 0.1.1 | Zlib OR Apache-2.0 OR MIT | transitive |  |
| objc2-ui-kit | 0.3.2 | Zlib OR Apache-2.0 OR MIT | transitive | oui |
| objc2-user-notifications | 0.3.2 | Zlib OR Apache-2.0 OR MIT | transitive | oui |
| objc2-web-kit | 0.3.2 | Zlib OR Apache-2.0 OR MIT | transitive |  |
| option-ext | 0.2.0 | MPL-2.0 | transitive |  |
| pango | 0.18.3 | MIT | transitive |  |
| pango-sys | 0.18.0 | MIT | transitive |  |
| phf | 0.13.1 | MIT | transitive | oui |
| phf_codegen | 0.13.1 | MIT | transitive |  |
| phf_generator | 0.13.1 | MIT | transitive |  |
| phf_macros | 0.13.1 | MIT | transitive |  |
| phf_shared | 0.13.1 | MIT | transitive | oui |
| plist | 1.10.3 | MIT | transitive |  |
| png | 0.18.1 | MIT OR Apache-2.0 | transitive | oui |
| portable-atomic | 1.15.0 | Apache-2.0 OR MIT | transitive |  |
| portable-atomic-util | 0.2.8 | Apache-2.0 OR MIT | transitive |  |
| powerfmt | 0.2.1 | MIT OR Apache-2.0 | transitive |  |
| precomputed-hash | 0.1.1 | MIT | transitive |  |
| proc-macro-crate | 1.3.1 | MIT OR Apache-2.0 | transitive | oui |
| proc-macro-crate | 2.0.2 | MIT OR Apache-2.0 | transitive | oui |
| proc-macro-error | 1.0.4 | MIT OR Apache-2.0 | transitive |  |
| proc-macro-error-attr | 1.0.4 | MIT OR Apache-2.0 | transitive |  |
| quick-xml | 0.42.0 | MIT | transitive | oui |
| redox_users | 0.5.3 | MIT | transitive |  |
| ref-cast | 1.0.27 | MIT OR Apache-2.0 | transitive |  |
| ref-cast-impl | 1.0.27 | MIT OR Apache-2.0 | transitive |  |
| regex | 1.13.1 | MIT OR Apache-2.0 | transitive |  |
| regex-automata | 0.4.18 | MIT OR Apache-2.0 | transitive |  |
| regex-syntax | 0.8.11 | MIT OR Apache-2.0 | transitive |  |
| reqwest | 0.13.5 | MIT OR Apache-2.0 | transitive | oui |
| schemars | 0.8.22 | MIT | transitive |  |
| schemars | 0.9.0 | MIT | transitive |  |
| schemars | 1.2.3 | MIT | transitive |  |
| schemars_derive | 0.8.22 | MIT | transitive |  |
| selectors | 0.38.0 | MPL-2.0 | transitive |  |
| serde-untagged | 0.1.9 | MIT OR Apache-2.0 | transitive |  |
| serde_derive_internals | 0.29.1 | MIT OR Apache-2.0 | transitive |  |
| serde_spanned | 0.6.9 | MIT OR Apache-2.0 | transitive |  |
| serde_spanned | 1.1.3 | MIT OR Apache-2.0 | transitive |  |
| serde_with | 3.24.0 | MIT OR Apache-2.0 | transitive |  |
| serde_with_macros | 3.24.0 | MIT OR Apache-2.0 | transitive |  |
| serialize-to-javascript | 0.1.2 | MIT OR Apache-2.0 | transitive |  |
| serialize-to-javascript-impl | 0.1.2 | MIT OR Apache-2.0 | transitive |  |
| servo_arc | 0.4.3 | MIT OR Apache-2.0 | transitive |  |
| soup3 | 0.5.0 | MIT | transitive |  |
| soup3-sys | 0.5.0 | MIT | transitive |  |
| static_vcruntime | 3.0.0 | MIT OR Apache-2.0 OR Zlib | transitive |  |
| string_cache | 0.9.0 | MIT OR Apache-2.0 | transitive |  |
| string_cache_codegen | 0.6.1 | MIT OR Apache-2.0 | transitive |  |
| strsim | 0.11.1 | MIT | transitive |  |
| swift-rs | 1.0.9 | MIT OR Apache-2.0 | transitive |  |
| syn | 1.0.109 | MIT OR Apache-2.0 | transitive | oui |
| system-deps | 6.2.2 | MIT OR Apache-2.0 | transitive |  |
| tao | 0.37.2 | Apache-2.0 | transitive |  |
| tao-macros | 0.1.4 | MIT OR Apache-2.0 | transitive |  |
| target-lexicon | 0.12.16 | Apache-2.0 WITH LLVM-exception | transitive |  |
| tauri | 2.12.5 | Apache-2.0 OR MIT | directe |  |
| tauri-build | 2.7.4 | Apache-2.0 OR MIT | directe |  |
| tauri-codegen | 2.7.4 | Apache-2.0 OR MIT | transitive |  |
| tauri-macros | 2.7.4 | Apache-2.0 OR MIT | transitive |  |
| tauri-runtime | 2.12.2 | Apache-2.0 OR MIT | transitive |  |
| tauri-runtime-wry | 2.12.3 | Apache-2.0 OR MIT | transitive |  |
| tauri-utils | 2.10.2 | Apache-2.0 OR MIT | transitive |  |
| tauri-winres | 0.3.7 | MIT | transitive |  |
| tendril | 0.5.1 | MIT OR Apache-2.0 | transitive |  |
| time | 0.3.55 | MIT OR Apache-2.0 | transitive |  |
| time-core | 0.1.9 | MIT OR Apache-2.0 | transitive |  |
| time-macros | 0.2.32 | MIT OR Apache-2.0 | transitive |  |
| tokio-util | 0.7.20 | MIT | transitive |  |
| toml | 0.8.2 | MIT OR Apache-2.0 | transitive |  |
| toml | 1.2.1+spec-1.1.0 | MIT OR Apache-2.0 | transitive |  |
| toml_datetime | 0.6.3 | MIT OR Apache-2.0 | transitive | oui |
| toml_datetime | 1.1.3+spec-1.1.0 | MIT OR Apache-2.0 | transitive | oui |
| toml_edit | 0.19.15 | MIT OR Apache-2.0 | transitive | oui |
| toml_edit | 0.20.2 | MIT OR Apache-2.0 | transitive | oui |
| toml_edit | 0.25.19+spec-1.1.0 | MIT OR Apache-2.0 | transitive | oui |
| toml_writer | 1.2.0+spec-1.1.0 | MIT OR Apache-2.0 | transitive |  |
| tray-icon | 0.25.1 | MIT OR Apache-2.0 | transitive |  |
| typeid | 1.0.3 | MIT OR Apache-2.0 | transitive |  |
| urlpattern | 0.6.0 | MIT | transitive |  |
| uuid | 1.29.0 | Apache-2.0 OR MIT | transitive |  |
| version-compare | 0.2.1 | MIT | transitive |  |
| vswhom | 0.1.0 | MIT | transitive |  |
| vswhom-sys | 0.1.3 | MIT | transitive |  |
| wasm-streams | 0.5.0 | MIT OR Apache-2.0 | transitive |  |
| web_atoms | 0.2.6 | MIT OR Apache-2.0 | transitive |  |
| webkit2gtk | 2.0.2 | MIT | transitive |  |
| webkit2gtk-sys | 2.0.2 | MIT | transitive |  |
| webview2-com | 0.39.1 | MIT | transitive |  |
| webview2-com-macros | 0.8.1 | MIT | transitive |  |
| webview2-com-sys | 0.39.1 | MIT | transitive |  |
| winapi | 0.3.9 | MIT/Apache-2.0 | transitive |  |
| winapi-i686-pc-windows-gnu | 0.4.0 | MIT/Apache-2.0 | transitive |  |
| winapi-x86_64-pc-windows-gnu | 0.4.0 | MIT/Apache-2.0 | transitive |  |
| window-vibrancy | 0.8.1 | Apache-2.0 OR MIT | transitive |  |
| windows | 0.62.2 | MIT OR Apache-2.0 | transitive |  |
| windows-collections | 0.3.2 | MIT OR Apache-2.0 | transitive |  |
| windows-future | 0.3.2 | MIT OR Apache-2.0 | transitive |  |
| windows-numerics | 0.3.1 | MIT OR Apache-2.0 | transitive |  |
| windows-sys | 0.45.0 | MIT OR Apache-2.0 | transitive | oui |
| windows-targets | 0.42.2 | MIT OR Apache-2.0 | transitive | oui |
| windows-threading | 0.2.1 | MIT OR Apache-2.0 | transitive |  |
| windows-version | 0.1.7 | MIT OR Apache-2.0 | transitive |  |
| windows_aarch64_gnullvm | 0.42.2 | MIT OR Apache-2.0 | transitive | oui |
| windows_aarch64_msvc | 0.42.2 | MIT OR Apache-2.0 | transitive | oui |
| windows_i686_gnu | 0.42.2 | MIT OR Apache-2.0 | transitive | oui |
| windows_i686_msvc | 0.42.2 | MIT OR Apache-2.0 | transitive | oui |
| windows_x86_64_gnu | 0.42.2 | MIT OR Apache-2.0 | transitive | oui |
| windows_x86_64_gnullvm | 0.42.2 | MIT OR Apache-2.0 | transitive | oui |
| windows_x86_64_msvc | 0.42.2 | MIT OR Apache-2.0 | transitive | oui |
| winnow | 0.5.40 | MIT | transitive | oui |
| winnow | 1.0.5 | MIT | transitive | oui |
| winreg | 0.56.0 | MIT | transitive |  |
| wry | 0.57.1 | Apache-2.0 OR MIT | transitive |  |
| x11 | 2.21.0 | MIT | transitive |  |

## Base locale du desktop (`argos-store`) : redb à la place de SQLite

✅ redb approuvée par Jee le 14 nov. 2026 (Argos HSS Trading : plus de SQLite). Rust pur, sa seule dépendance (libc)
était déjà là. **Retirées** du `Cargo.lock` de l'app avec SQLite : rusqlite, libsqlite3-sys (code C), hashlink,
hashbrown (ancienne version), fallible-iterator, fallible-streaming-iterator, vcpkg ; l'exception d'audit
RUSTSEC-2026-0459 est retirée de `deny.toml` et du `Makefile`.

| Crate | Version | Licence | Lien | Déjà là (autre version) |
|---|---|---|---|---|
| redb | 4.5.0 | MIT OR Apache-2.0 | directe |  |

## Argos Lens (`argos-lens`) : prix du paquet `argos-packet/v1` en décimal exact

✅ Approuvée par Jee le 14 nov. 2026. Sans les options par défaut ; ses deux dépendances (arrayvec, num-traits) étaient déjà là.

| Crate | Version | Licence | Lien | Déjà là (autre version) |
|---|---|---|---|---|
| rust_decimal | 1.43.0 | MIT | directe |  |

## Mises à jour ou options de dépendances déjà là (même crate, autre version ou fonction en plus)

1 crates : 0 directe(s), 1 transitive(s).

| Crate | Version | Licence | Lien | Déjà là (autre version) |
|---|---|---|---|---|
| async-executor | 1.14.1 | Apache-2.0 OR MIT | transitive | oui |

