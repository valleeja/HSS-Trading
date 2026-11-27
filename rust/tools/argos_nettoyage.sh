#!/usr/bin/env bash
# 🧽 Nettoyage automatique des caches et du disque, sans question : prévu pour tourner toutes les 30 min.
# Contrairement à argos_menage.sh (interactif, déplace des fichiers), celui-ci n'efface QUE des caches qui se refont seuls,
# et garde la compilation incrémentale récente (pas de recompilation complète à chaque passage).
#
#   bash rust/tools/argos_nettoyage.sh          (ou : cd rust && make clean-cache)
#
# Toutes les 30 min, automatiquement :
#   Linux / macOS : crontab -e  puis ajouter la ligne (adapter le chemin) :
#     */30 * * * * bash "$HOME/Documents/Argos/rust/tools/argos_nettoyage.sh" >> "$HOME/.cache/argos-nettoyage.log" 2>&1
#   Windows (Git Bash) : Planificateur de tâches ▸ toutes les 30 min ▸ programme bash.exe, argument = chemin de ce script.
#
# Réglages (variables d'environnement) :
#   ARGOS_TARGET_MAX_GO   taille max de rust/target avant un cargo clean complet (défaut 20)
#   ARGOS_CACHE_JOURS     âge max des bougies et logos en cache (défaut 14)
# Jamais touchés : tes réglages (~/.config/argos), Key/, data/, le code.
set -uo pipefail
ici=$(cd "$(dirname "$0")/.." && pwd) # dossier rust/
max_go=${ARGOS_TARGET_MAX_GO:-20}
jours=${ARGOS_CACHE_JOURS:-14}
cache="${XDG_CACHE_HOME:-$HOME/.cache}/argos"
taille() { du -sh "$1" 2>/dev/null | cut -f1 || echo 0; }

echo "🧽 $(date '+%Y-%m-%d %H:%M') nettoyage Argos"

# 1. compilation : les dossiers « incremental » de plus de 2 jours (ce sont eux qui gonflent), puis cargo clean si trop gros
if [ -d "$ici/target" ]; then
  find "$ici/target" -maxdepth 3 -type d -name incremental -print0 2>/dev/null |
    while IFS= read -r -d '' d; do find "$d" -mindepth 1 -maxdepth 1 -mtime +2 -exec rm -rf {} +; done
  ko=$(du -sk "$ici/target" 2>/dev/null | cut -f1 || echo 0)
  if [ "${ko:-0}" -gt $((max_go * 1024 * 1024)) ]; then
    echo "   rust/target dépasse ${max_go} Go : cargo clean (la prochaine compilation sera plus longue)"
    (cd "$ici" && cargo clean -q)
  fi
  echo "   rust/target : $(taille "$ici/target")"
fi

# 2. bougies et logos en cache, plus vieux que $jours jours (ils se re-téléchargent)
if [ -d "$cache" ]; then
  find "$cache" -type f -mtime +"$jours" -delete 2>/dev/null
  echo "   cache Argos : $(taille "$cache")"
fi

# 3. archives de crates téléchargées non utilisées depuis 30 jours (cargo les re-télécharge si besoin)
reg="${CARGO_HOME:-$HOME/.cargo}/registry/cache"
if [ -d "$reg" ]; then
  find "$reg" -type f -name '*.crate' -atime +30 -delete 2>/dev/null
  echo "   crates en cache : $(taille "$reg")"
fi

# 4. état du disque
df -h "$ici" 2>/dev/null | awk 'NR==2 {print "   disque : " $4 " libres sur " $2 " (" $5 " utilisés)"}'
