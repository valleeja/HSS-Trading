#!/usr/bin/env bash
# 🛰️ Lance le serveur local d'Argos HSS Trading (argos-serveur) : l'app web (/app2/, /app/), la session partagée
# entre le PC et le cell (/api/reglages) et le journal chiffré du PC (/api/journal). « / » renvoie vers /app/.
# Tout est dans ce dépôt : rien à faire dans un autre projet.
#
#   ./lancer-serveur.sh            port 8000, en arrière-plan (journal : /tmp/argos-trading-<port>.log)
#   ./lancer-serveur.sh 8001       un autre port
#
# Construit au besoin le serveur (make serveur) ; les pages (dist/) viennent de « cd rust && make web-build ».
# ARGOS_DIST=<dossier> : servir un autre dossier de pages (par exemple un lien basculé d'un coup après un build, pour
# ne jamais servir de pages à moitié construites).
set -u
DEPOT="$(cd "$(dirname "$0")" && pwd)"
PORT="${1:-8000}"
cd "$DEPOT/rust" || exit 1

if ss -ltn 2>/dev/null | grep -q ":$PORT "; then
  echo "Le port $PORT est déjà utilisé : le serveur tourne sans doute déjà."
  exit 0
fi
DIST="${ARGOS_DIST:-$DEPOT/dist}"
if [ ! -d "$DIST/app" ]; then
  echo "❌ Les pages ne sont pas encore construites : lance d'abord « cd rust && make web-build »."
  exit 1
fi
SERVEUR="$DEPOT/rust/target/release/argos-serveur"
if [ ! -x "$SERVEUR" ]; then
  echo "🧰 argos-serveur n'est pas encore construit : je le construis (make serveur)…"
  nice -n 19 make serveur || exit 1
fi
JOURNAL="/tmp/argos-trading-$PORT.log"
setsid nohup "$SERVEUR" "$PORT" "$DIST" /app/ > "$JOURNAL" 2>&1 < /dev/null &
echo "🌐 Serveur lancé sur le port $PORT : http://localhost:$PORT/app2/ (journal : $JOURNAL)"
