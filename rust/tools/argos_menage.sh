#!/usr/bin/env bash
# 🧹 Ménage du dossier Argos sur ton ordi (à lancer toi-même) :
#   bash ~/Documents/Argos/rust/tools/argos_menage.sh
# - doublons et vieilles copies → ~/Documents/Argos/_to_delete/ (rien n'est effacé : tu vides ce dossier toi-même)
# - caches effacés (ils se refont tout seuls) : rust/target (compilation) et ~/.cache/argos (bougies)
# Jamais touchés : Key/, data/ (sauvegardes), ~/.config/argos (tes réglages), le reste de Documents.
set -euo pipefail
A="$HOME/Documents/Argos"
T="$A/_to_delete"
cd "$A"

# même contenu ? (les deux fichiers existent et sont identiques)
same() { [ -f "$1" ] && [ -f "$2" ] && cmp -s "$1" "$2"; }

echo "🧹 Ménage Argos dans $A"
moves=()
same "Argos-raccourcis.pdf" "pdf/Argos-raccourcis.pdf" && moves+=("Argos-raccourcis.pdf")
[ -f "rust/barres.png" ] && moves+=("rust/barres.png")
# vieilles copies remplacées : src/ incomplet (le vrai est sur GitHub), v2/ (JavaScript), desktop/ (ancienne app Tauri)
for d in src v2 desktop; do [ -e "$d" ] && moves+=("$d"); done
pdf_chain="Argos Chain — Plan d'exécution.pdf"

echo
echo "📦 Vers _to_delete/ :"; for m in "${moves[@]}"; do echo "   - $m"; done
[ -f "$pdf_chain" ] && echo "📁 Rangé dans pdf/ : $pdf_chain"
echo "🗑️  Caches effacés : rust/target ($(du -sh rust/target 2>/dev/null | cut -f1 || echo 0)), ~/.cache/argos ($(du -sh "$HOME/.cache/argos" 2>/dev/null | cut -f1 || echo 0))"
echo
read -r -p "On y va ? (o/N) " ok
[ "$ok" = "o" ] || [ "$ok" = "O" ] || { echo "Annulé, rien n'a bougé."; exit 0; }

mkdir -p "$T"
for m in "${moves[@]}"; do mkdir -p "$T/$(dirname "$m")"; mv "$m" "$T/$m"; done
[ -f "$pdf_chain" ] && mv "$pdf_chain" "pdf/"
rm -rf rust/target "$HOME/.cache/argos"
echo "✅ Fini. Regarde _to_delete/ puis vide-le quand tu veux. La prochaine compilation Rust prendra quelques minutes."
