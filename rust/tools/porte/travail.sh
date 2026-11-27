#!/usr/bin/env bash
# 🧮 Quality gate, counted work (Jee, 25 Nov 2026: « je veux 1 % », « sinon on peut les accumuler vite »).
# A time changes from one run to the next (up to 9 % in a browser); the number of instructions the processor runs to
# draw the same images does not (0.3 % at most, measured). Each gesture of tests/vitesse.rs (`work_*`: the cell's chart
# dragged and pinched, its market list, the PC's chart dragged) runs twice under valgrind (cachegrind counts the
# instructions), with ARGOS_TRAVAIL_IMAGES=N then 2N: the difference is the exact work of N images of the gesture, the
# start of the program taken away.
#  - more than 1 % above its reference (tools/porte/travail.txt): red;
#  - the reference is a ratchet: ARGOS_VITESSE=maj writes it again only where the work went down (Jee's phrase);
#    a slower gesture is accepted only by Jee (ARGOS_VITESSE=plus-lent, with his phrase). Small slowdowns never add up.
#  - the counts depend on the computer and on the Rust compiler: the reference is written on Jee's PC, and again
#    (ARGOS_VITESSE=plus-lent, Jee's phrase) when the compiler changes.
# Run from rust/ after `make check` (the test programs are the ones it built: nothing is compiled again).
set -euo pipefail
cd "$(dirname "$0")/../.."
command -v valgrind > /dev/null || { echo "⛔ porte, travail : valgrind absent : le travail des gestes ne serait pas compté (sudo apt install valgrind)"; exit 1; }
ref=tools/porte/travail.txt
N=4
GESTES="work_cell_drag work_cell_pinch work_cell_market_list work_pc_drag"
# (its files in target/porte-travail, written again at each run)
tmp=target/porte-travail
mkdir -p "$tmp"
cargo test --workspace --locked --no-run --message-format=json 2> "$tmp/build.log" > "$tmp/build.json" || { tail -20 "$tmp/build.log"; echo "⛔ porte, travail : les programmes de test ne se construisent pas"; exit 1; }
prog=$(sed -n 's/.*"manifest_path":"[^"]*\/crates\/argos-desktop\/Cargo\.toml".*"profile":{[^}]*"test":true[^}]*}.*"executable":"\([^"]*\)".*/\1/p' "$tmp/build.json" | head -1)
[ -n "$prog" ] && [ -x "$prog" ] || { echo "⛔ porte, travail : programme de test d'argos-desktop introuvable"; exit 1; }

# instructions run by one gesture test with `images` images (the test must pass)
compter() {
  local geste=$1 images=$2 sortie
  sortie=$(cd crates/argos-desktop && ARGOS_TRAVAIL_IMAGES=$images valgrind --tool=cachegrind --cache-sim=no --cachegrind-out-file=/dev/null "$prog" --exact "tests::vitesse::$geste" --test-threads=1 2>&1) || { echo "$sortie" | tail -20 >&2; return 1; }
  echo "$sortie" | grep -q "test result: ok. 1 passed" || { echo "$sortie" | tail -20 >&2; return 1; }
  echo "$sortie" | sed -n 's/.*I *refs: *\([0-9,]*\).*/\1/p' | tr -d ,
}

declare -A mesure
for g in $GESTES; do
  a=$(compter "$g" "$N") || { echo "⛔ porte, travail : $g ne tourne pas sous valgrind"; exit 1; }
  b=$(compter "$g" $((2 * N))) || { echo "⛔ porte, travail : $g ne tourne pas sous valgrind"; exit 1; }
  mesure[$g]=$((b - a))
  [ "${mesure[$g]}" -gt 0 ] || { echo "⛔ porte, travail : $g : mesure impossible ($a puis $b instructions)"; exit 1; }
done

declare -A plancher
if [ -s "$ref" ]; then
  [ "$(sed -n 's/^images \([0-9]*\)$/\1/p' "$ref")" = "$N" ] || { echo "⛔ porte, travail : la référence ne compte pas $N images : ARGOS_VITESSE=plus-lent, avec la phrase de Jee"; exit 1; }
  while read -r g v; do
    case "$g" in work_*) plancher[$g]=$v ;; esac
  done < "$ref"
fi

ecrire() {
  {
    echo "# Written by tools/porte/travail.sh (ARGOS_VITESSE=maj or plus-lent, Jee's phrase), never by hand."
    echo "# gesture, instructions run to draw its images (valgrind, cachegrind), on Jee's PC"
    echo "images $N"
    for g in $GESTES; do echo "$g ${nouveau[$g]}"; done
  } > "$ref"
  echo "🧮 travail : référence écrite ($ref) : à commiter avec la branche"
}

declare -A nouveau
mode=${ARGOS_VITESSE:-}
rouge=0
for g in $GESTES; do
  m=${mesure[$g]}
  p=${plancher[$g]:-}
  if [ -z "$p" ]; then
    echo "🧮 travail : $g $m instructions pour $N images (pas encore de plancher)"
    nouveau[$g]=$m
    [ -n "$mode" ] || { echo "⛔ porte, travail : $g sans référence : ARGOS_VITESSE=maj sur le PC de Jee, avec sa phrase"; rouge=1; }
    continue
  fi
  ecart=$(awk -v m="$m" -v p="$p" 'BEGIN { printf "%+.2f", (m / p - 1) * 100 }')
  echo "🧮 travail : $g $m instructions pour $N images ($ecart % par rapport au plancher $p)"
  nouveau[$g]=$p
  if awk -v m="$m" -v p="$p" 'BEGIN { exit !(m > p * 1.01) }'; then
    if [ "$mode" = plus-lent ]; then
      nouveau[$g]=$m
      echo "🧮 travail : $g plus lent, accepté par Jee (ARGOS_VITESSE=plus-lent) : le plancher monte"
    else
      echo "⛔ porte, travail : $g fait $ecart % de travail en plus (plus de 1 %) : la branche est refusée"
      rouge=1
    fi
  elif awk -v m="$m" -v p="$p" 'BEGIN { exit !(m < p) }'; then
    if [ -n "$mode" ]; then
      nouveau[$g]=$m
    elif awk -v m="$m" -v p="$p" 'BEGIN { exit !(m < p * 0.99) }'; then
      echo "🧮 travail : $g plus rapide ($ecart %) : le plancher peut descendre (ARGOS_VITESSE=maj, phrase de Jee, puis commit)"
    fi
  fi
done
if [ -n "$mode" ] && [ "$rouge" = 0 ]; then
  ecrire
fi
[ "$rouge" = 0 ] || exit 1
echo "🧮 travail : vert"
