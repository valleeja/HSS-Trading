#!/usr/bin/env bash
# 🛡 The bug register (rust/tools/porte/registre.txt, Jee 24 Nov 2026: « si une nouvelle branche crée un bug de
# cette liste, elle devra le corriger »): every line is well formed, and every guard test it names still exists in
# the workspace and is not ignored. The tests themselves run in `make check`: a bug that comes back turns its test
# red there. Run from rust/ after `make check`: the test programs are the ones make check built (same workspace,
# same features), each one asked for its list; nothing is compiled again.
set -euo pipefail
cd "$(dirname "$0")/../.."
reg=tools/porte/registre.txt
[ -s "$reg" ] || { echo "⛔ registre des bugs absent : $reg"; exit 1; }
# (its lists in target/porte-registre, written again at each run)
tmp=target/porte-registre
mkdir -p "$tmp"
: > "$tmp/tests"
rouge=0
dire() { echo "⛔ registre : $*"; rouge=1; }
net() { local s="$1"; s="${s#"${s%%[![:space:]]*}"}"; s="${s%"${s##*[![:space:]]}"}"; printf '%s' "$s"; }
declare -A vus
lignes=0
while IFS= read -r l || [ -n "$l" ]; do
  case "$l" in ''|'#'*) continue ;; esac
  lignes=$((lignes + 1))
  IFS='|' read -r -a f <<< "$l"
  [ "${#f[@]}" -eq 7 ] || { dire "7 champs attendus, ${#f[@]} lus : ${l:0:80}"; continue; }
  id=$(net "${f[0]}")
  date=$(net "${f[1]}")
  [[ "$id" =~ ^B-[0-9]{3}$ ]] || dire "numéro « $id » (B-001, B-002…)"
  [ -z "${vus[$id]:-}" ] || dire "numéro $id deux fois"
  vus[$id]=1
  [[ "$date" =~ ^20[0-9]{2}-[01][0-9]-[0-3][0-9]$ ]] || dire "$id : date « $date » (AAAA-MM-JJ)"
  for k in 2 3 4 5; do [ -n "$(net "${f[$k]}")" ] || dire "$id : champ $((k + 1)) vide"; done
  tests=$(net "${f[6]}")
  [ -n "$tests" ] || dire "$id : aucun test qui le garde fermé"
  IFS=',' read -r -a ts <<< "$tests"
  for t in "${ts[@]}"; do
    t=$(net "$t")
    crate=${t%% *}
    chemin=${t#* }
    [ "$crate" != "$chemin" ] && [ -n "$chemin" ] && [[ "$chemin" != *" "* ]] || { dire "$id : « $t » : écrire « crate chemin::du::test »"; continue; }
    echo "$id $crate $chemin" >> "$tmp/tests"
  done
done < "$reg"
[ "$lignes" -gt 0 ] || dire "aucun bug inscrit"
[ -s "$tmp/tests" ] || { echo "⛔ registre rouge"; exit 1; }
# (a guard test left out by a `--skip` of the Makefile's `cargo test` lines would never run and stay green here: it is
# refused, the filter read as cargo reads it, a part of the test's path; the speed guard alone is skipped there on
# purpose, run apart after the others and checked to have run once: allowed only while that line is there)
vitesse=tests::vitesse::the_cell_stays_as_fast_as_its_reference
grep -oE -e '--skip([= ]+)[^ ;|&)\\]+' Makefile | sed -E 's/^--skip[= ]+//' | sort -u > "$tmp/sautes" || true
while read -r s; do
  if [ "$s" = "$vitesse" ]; then
    grep -qF -e "--exact $vitesse --test-threads=1" Makefile && grep -qF -e 'test result: ok\. 1 passed;' Makefile ||
      dire "le test de vitesse est sauté par le Makefile sans y être lancé à part et vérifié une fois"
    continue
  fi
  while read -r id crate chemin; do
    case "$chemin" in *"$s"*) dire "$id : le test $crate $chemin est sauté par « --skip $s » dans le Makefile" ;; esac
  done < "$tmp/tests"
done < "$tmp/sautes"
# the test programs of the workspace, as make check builds them (a crate alone could build with fewer features),
# each with the crate it belongs to (the folder of its Cargo.toml); a tab between the two, the paths keep their spaces
# (Jee's clone lives in « Argos HSS Trading »)
cargo test --workspace --locked --no-run --message-format=json 2> "$tmp/build.log" > "$tmp/build.json" || { tail -20 "$tmp/build.log"; dire "les programmes de test ne se construisent pas"; }
sed -n 's/.*"manifest_path":"\([^"]*\)\/Cargo\.toml".*"profile":{[^}]*"test":true[^}]*}.*"executable":"\([^"]*\)".*/\1\t\2/p' "$tmp/build.json" > "$tmp/programmes"
[ -s "$tmp/programmes" ] || dire "aucun programme de test trouvé"
for crate in $(cut -d' ' -f2 "$tmp/tests" | sort -u); do
  : > "$tmp/$crate.liste"
  : > "$tmp/$crate.ignores"
done
while IFS=$'\t' read -r dossier programme; do
  crate=$(basename "$dossier")
  [ -e "$tmp/$crate.liste" ] || continue
  "$programme" --list 2> /dev/null | sed -n 's/: test$//p' >> "$tmp/$crate.liste" || dire "$crate : liste des tests illisible ($programme)"
  "$programme" --list --ignored 2> /dev/null | sed -n 's/: test$//p' >> "$tmp/$crate.ignores" || true
done < "$tmp/programmes"
n=0
while read -r id crate chemin; do
  n=$((n + 1))
  grep -qxF "$chemin" "$tmp/$crate.liste" || { dire "$id : le test $crate $chemin n'existe plus (renommé ou retiré ?)"; continue; }
  if grep -qxF "$chemin" "$tmp/$crate.ignores"; then dire "$id : le test $crate $chemin est ignoré (#[ignore])"; fi
done < "$tmp/tests"
if [ "$rouge" = 1 ]; then
  echo "⛔ registre des bugs rouge : un test qui garde un bug fermé ne se retire, ne se renomme et ne s'ignore jamais."
  exit 1
fi
echo "🛡 registre des bugs : $lignes bugs, $n tests qui les gardent fermés, tous présents."
