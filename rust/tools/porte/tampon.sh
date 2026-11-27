#!/usr/bin/env bash
# 🛡 The stamp of the quality gate, for the exact content of HEAD (its tree): `make porte` calls it with `debut`
# before its first check and with `fin` after its last one. `fin` writes the stamp only when HEAD is still the
# commit of `debut` and nothing changed meanwhile: a stamp always names what was really checked. Untracked files count
# too (a reference such as navigateur.txt or travail.txt left uncommitted would make the gate pass on a content the
# stamped tree does not hold); only what .gitignore ignores (target/, the safety net's .nouveau.png…) is allowed.
# The stamps live in the clone's git folder (argos-porte/<tree>), never in the repository.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
dir="$(git rev-parse --git-common-dir)/argos-porte"
mkdir -p "$dir"
debut="$dir/.debut-$(git rev-parse --show-toplevel | sha256sum | cut -c1-16)"
propre() { [ -z "$(git status --porcelain --untracked-files=normal)" ]; }
case "${1:-}" in
  debut)
    propre || { echo "⛔ make porte : des changements ne sont pas commités (fichiers non suivis compris). La porte ne tamponne qu'un contenu commité : git commit d'abord."; git status --short --untracked-files=normal; exit 1; }
    git rev-parse HEAD > "$debut"
    echo "🛡 porte : début sur $(git rev-parse --short HEAD) ($(git rev-parse --abbrev-ref HEAD))"
    ;;
  fin)
    commit=$(git rev-parse HEAD)
    [ -s "$debut" ] && [ "$(cat "$debut")" = "$commit" ] || { echo "⛔ make porte : HEAD a changé pendant la porte : rien n'est tamponné, relance make porte."; exit 1; }
    propre || { echo "⛔ make porte : des fichiers ont changé ou sont apparus pendant la porte : rien n'est tamponné, relance make porte."; git status --short --untracked-files=normal; exit 1; }
    tree=$(git rev-parse "HEAD^{tree}")
    printf 'commit %s\nbranche %s\ndate %s\n' "$commit" "$(git rev-parse --abbrev-ref HEAD)" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$dir/$tree"
    : > "$debut"
    echo "🛡 porte : tout est vert. Tampon écrit pour $(git rev-parse --short HEAD) (arbre $tree)."
    ;;
  *)
    echo "usage : tampon.sh debut|fin"
    exit 2
    ;;
esac
