#!/usr/bin/env bash
# 🌐 Construction Vercel : le site (fichiers statiques) + l'app Rust complète (argos-desktop) compilée en WASM dans /app.
# Vercel installe Rust et wasm-bindgen (même version que Cargo.lock), compile l'app, et sert le dossier dist/
# (le code source rust/, desktop/, src/ et tools/ n'est pas publié).
set -euo pipefail
cd "$(dirname "$0")/../.."
export PATH="$HOME/.cargo/bin:$PATH"
if ! command -v cargo >/dev/null; then
  curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable
fi
rustup target add wasm32-unknown-unknown
WBV=$(grep -A1 '^name = "wasm-bindgen"$' rust/Cargo.lock | sed -n 's/^version = "\(.*\)"/\1/p')
WB="/tmp/wasm-bindgen-$WBV-x86_64-unknown-linux-musl/wasm-bindgen"
# sur ton ordi (Mac…) : le wasm-bindgen installé (cargo install wasm-bindgen-cli --version $WBV), s'il a la bonne version
if command -v wasm-bindgen >/dev/null && wasm-bindgen --version | grep -q " $WBV\b"; then WB=$(command -v wasm-bindgen); fi
[ -x "$WB" ] || curl -sSL "https://github.com/wasm-bindgen/wasm-bindgen/releases/download/$WBV/wasm-bindgen-$WBV-x86_64-unknown-linux-musl.tar.gz" | tar xz -C /tmp
# 📦 version du build (date UTC) et commit : dans l'app (Infos, mises à jour) et dans dist/app/version.json
export ARGOS_BUILD="${ARGOS_BUILD:-$(date -u +%Y%m%d%H%M)}" ARGOS_COMMIT="${ARGOS_COMMIT:-$(git describe --always --dirty=-modifié 2>/dev/null || echo '?')}"
(cd rust && cargo build -p argos-desktop --release --target wasm32-unknown-unknown)
rm -rf dist && mkdir dist
for f in *; do
  case "$f" in rust|desktop|src|tools|dist|_test.html|node_modules) ;; *) cp -r "$f" dist/ ;; esac
done
# 🖼️ captures côte à côte maquette | app : toujours servies sur /captures/ (sinon effacées à chaque build)
mkdir -p dist/captures && cp croquis/references/captures/*.png dist/captures/ 2>/dev/null || true
"$WB" --target web --no-typescript --out-name argos --out-dir dist/app/pkg rust/target/wasm32-unknown-unknown/release/argos-desktop.wasm
# 📦 version de /app/ : argos-outils (Rust pur, construit dans rust/target/outils)
(cd rust && CARGO_TARGET_DIR=target/outils cargo build --release --manifest-path tools/argos-outils/Cargo.toml)
rust/target/outils/release/argos-outils version-app dist
ls -la dist/app dist/app/pkg
