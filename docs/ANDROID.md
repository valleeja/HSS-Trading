# 🤖 Argos sur Android : état, construction et aide pour corriger

26 nov. 2026 · Pour toute personne qui veut aider à faire marcher l'APK.

## L'état aujourd'hui

- **L'APK Android est expérimental.** Il n'a jamais été construit ni essayé sur un vrai téléphone avant cette version de test. La première construction dans GitHub Actions peut échouer, et l'app peut avoir des défauts au démarrage.
- C'est une **version de débogage** (`--debug`), pour les téléphones **arm64** seulement (presque tous les téléphones récents). Elle est signée avec la clé de débogage d'Android, pas avec une clé de publication.
- Sur un téléphone, l'app marche déjà **dans le navigateur** (page web `/app2/`, la même app en WebAssembly). L'APK sert à l'avoir comme une vraie app, sans serveur sur un PC.

Si tu trouves un défaut, ouvre une **issue** avec : le modèle du téléphone, la version d'Android, ce que tu as fait, ce que tu as vu, et si possible le journal (`adb logcat`, voir plus bas).

## Comment l'APK est fait

L'app Android n'a pas de code à part : c'est la même app que dans le navigateur, mise dans une **coque Tauri 2**.

```
rust/crates/argos-desktop   ──(wasm32)──▶  dist/app/   (l'app WebAssembly, la même que /app2/)
rust/argos-tauri            ──(aarch64-linux-android)──▶  la coque : une WebView Android qui ouvre app/index.html
```

- `rust/argos-tauri/` : la coque, avec **son propre espace de travail et son propre `Cargo.lock`** (hors de `make check` et `make vert` ; vérifiée par `make tauri`).
- `src/lib.rs` : le point d'entrée (`#[cfg_attr(mobile, tauri::mobile_entry_point)] pub fn run()`) et les commandes que la page peut appeler.
- `src/passerelle.rs` : **la page n'a aucun accès au réseau**. Chaque requête de la page passe par le cœur Rust (`argos-data`, en rustls), qui vérifie la liste blanche `rust/crates/argos-data/src/net.rs`. Le direct (WebSocket) et les modules WASM passent aussi par là.
- `src/reglages.rs` : les réglages, partagés avec l'app de bureau.
- `capabilities/default.json` et `build.rs` : la liste fermée des commandes permises. Aucun plugin Tauri.
- `tauri.conf.json` : identifiant `com.argos.hsstrading`, pages dans `./web`, fenêtre sur `app/index.html`, politique de sécurité (CSP) sans réseau direct.
- `tests/securite.rs` : vérifie que la page ne peut pas appeler le réseau elle-même.

## Comment GitHub le construit

Le travail `android` de `.github/workflows/version-test.yml` (lancé à la main : onglet *Actions* ▸ *Version de test* ▸ *Run workflow*, avec un nom de version comme `test-v0.1.0`) :

1. construit l'app WebAssembly : `bash rust/tools/argos_web_build.sh` (résultat dans `dist/app`) ;
2. prépare les pages de la coque : `rust/argos-tauri/ui/*` et `dist/app` copiés dans `rust/argos-tauri/web/` ;
3. installe la cible Rust `aarch64-linux-android` et `tauri-cli` 2.12.4 ; prend le JDK 17 et le NDK déjà installés sur la machine de GitHub (`JAVA_HOME_17_X64`, `ANDROID_NDK_LATEST_HOME`) ;
4. dans `rust/argos-tauri` : `cargo tauri android init --ci`, puis `cargo tauri android build --apk --debug --target aarch64 --ci` ;
5. copie l'APK en `Argos-android-arm64.apk` et l'ajoute à la Release de la version.

Le projet Android (`rust/argos-tauri/gen/android/`) est **généré à chaque construction** par `android init` : il n'est pas dans le dépôt. Pour changer quelque chose dans le projet Android (permissions, icône, nom), il faudra soit le commiter, soit passer par `tauri.conf.json`.

## Le construire sur ton ordinateur

Il faut : Rust (la version 1.99.1 s'installe toute seule), **Android Studio** ou le SDK Android en ligne de commande, le **NDK**, un **JDK 17**.

```bash
# une fois
rustup target add aarch64-linux-android
cargo install tauri-cli --version "=2.12.4" --locked
export JAVA_HOME=/chemin/vers/jdk-17
export ANDROID_HOME=$HOME/Android/Sdk
export NDK_HOME=$ANDROID_HOME/ndk/<version>

# à chaque fois, depuis la racine du dépôt
bash rust/tools/argos_web_build.sh
rm -rf rust/argos-tauri/web && mkdir -p rust/argos-tauri/web
cp rust/argos-tauri/ui/* rust/argos-tauri/web/ && cp -r dist/app rust/argos-tauri/web/app
cd rust/argos-tauri
cargo tauri android init            # la première fois
cargo tauri android dev             # sur un téléphone branché en USB (débogage USB activé) ou un émulateur
cargo tauri android build --apk --debug --target aarch64
```

Voir le journal du téléphone : `adb logcat | grep -i -E "argos|tauri|rust|panic"`. La page web de l'app se débogue depuis Chrome sur l'ordinateur : `chrome://inspect`, avec le téléphone branché.

## Ce qui risque de casser (à vérifier en premier)

Ces points n'ont pas pu être vérifiés sans construction Android. Ce sont les premières pistes si l'APK échoue :

1. **Le dossier des réglages.** `src/reglages.rs` cherche `$XDG_CONFIG_HOME` puis `~/.config/argos-hss-trading/charts`. Sur Android, `HOME` n'est souvent pas défini : il faudra sans doute passer par le dossier de l'app que donne Tauri (`app.path().app_config_dir()`).
2. **Les clés API des bourses.** Sur le bureau, elles vont dans le trousseau du système. Android n'a pas ce trousseau : il faudra le Keystore d'Android, ou garder les fonctions à clé désactivées sur Android au début.
3. **Les options de Tauri.** `Cargo.toml` active `common-controls-v6` (Windows) et `x11` (Linux). Elles devraient être ignorées sur Android ; sinon, les mettre seulement pour ces systèmes.
4. **La compilation de `ring`** (utilisé par rustls) demande le compilateur C du NDK : si elle échoue, vérifier `NDK_HOME` et le `CC` de la cible `aarch64-linux-android`.
5. **Les modules WASM** (`argos-module-host`, wasmtime) : gros à compiler pour Android ; si la construction est trop longue ou échoue, on peut les couper sur Android au début.
6. **L'écran tactile** : l'app est déjà faite pour le doigt (version téléphone de `/app2/`), mais dans la WebView, vérifier le zoom à deux doigts, le clavier et la barre du haut d'Android.
7. **Le réseau** : la permission `INTERNET` doit être dans le manifeste généré (Tauri la met normalement).

## Les règles à garder

- Aucune nouvelle adresse réseau sans l'ajouter à `rust/crates/argos-data/src/net.rs` (HTTPS ou WSS seulement).
- La page reste sans réseau direct : tout passe par la passerelle du cœur.
- Aucun plugin Tauri sans l'accord de Jee ; chaque nouvelle dépendance est demandée une par une.
- Démo seulement : aucun ordre réel.
- Le reste des règles est dans [`CONTINUER-EN-RUST.md`](CONTINUER-EN-RUST.md) et [`CONTRIBUTING.md`](../CONTRIBUTING.md).
