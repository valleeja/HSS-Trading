#!/usr/bin/env bash
# 🧩 Construit le module d'exemple « Moyenne » du kit module Argos, puis nos modules (rust/modules/*) (copie figée dans vendor/, jamais modifiée) :
# interface v2, aucun droit, réglages longueur et source. Résultat dans app/modules/ (moyenne.wasm + moyenne.json) :
# l'app l'embarque (📦 module d'exemple) et /app/ le sert.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
kit=vendor/kit-module-argos-1.0.2
rustup target add wasm32-unknown-unknown >/dev/null
# dossier de compilation à part (dans target/ de Trading, jamais dans le kit) ; --locked : le kit n'est pas modifié
CARGO_TARGET_DIR=target/modules cargo build --locked --manifest-path "$kit/exemple-moyenne/Cargo.toml" \
  --target wasm32-unknown-unknown --release
out=../app/modules
mkdir -p "$out"
cp target/modules/wasm32-unknown-unknown/release/argos_exemple_moyenne.wasm "$out/moyenne.wasm"
cp "$kit/exemple-moyenne/argos-module.json" "$out/moyenne.json"
# 📈 nos modules (rust/modules/*) : même construction, même dossier de sortie
for m in modules/*/; do
  name=$(basename "$m")
  CARGO_TARGET_DIR=target/modules cargo build --locked --manifest-path "$m/Cargo.toml" --target wasm32-unknown-unknown --release
  cp "target/modules/wasm32-unknown-unknown/release/argos_module_${name//-/_}.wasm" "$out/$name.wasm"
  cp "$m/argos-module.json" "$out/$name.json"
done
ls -la "$out"
