# 🦀 Raccourcis d'Argos (Rust). Depuis le dossier rust/ :
#   make fmt     formater nos crates (jamais la copie du kit dans vendor/)
#   make check   tout vérifier, comme la CI : formatage, clippy (desktop + navigateur), tests
#   make run     lancer l'app desktop
#   make watch   lancer l'app desktop et la relancer toute seule à chaque fichier sauvegardé (cargo install cargo-watch)
#   make vert    « tout vert » de la règle des modules : check + audit + deny (cargo install cargo-audit cargo-deny)
#   make audit   failles connues des dépendances (cargo install cargo-audit) et secrets dans l'historique (gitleaks, si installé)
#   make hooks   rendre make vert obligatoire avant chaque commit et make check avant chaque push (une fois par clone) ;
#                interface-2 ne bouge plus que vers un contenu passé par make porte (hook reference-transaction)
#   make porte   🛡 porte de qualité, obligatoire avant toute fusion dans interface-2 : make vert, le registre des bugs,
#                le travail compté des gestes (1 %), l'app web dans un vrai navigateur (netteté, images/s) ;
#                tout vert = un tampon pour ce contenu exact
#   make registre  le registre des bugs (tools/porte/registre.txt) : chaque test qui garde un bug fermé existe encore
#   make porte-travail  le travail compté des gestes (valgrind) : plus de 1 % au-dessus de son plancher = rouge
#   make clean-cache  nettoyer caches et disque sans casser la compilation (voir tools/argos_nettoyage.sh)
#   make web     construire l'app navigateur (/app) et la servir sur http://localhost:8000/app/
#                (argos-serveur : le serveur d'argos-outils, plus le journal chiffré servi au cell)
#   make tauri   📱 coque Tauri 2 (argos-tauri/) : la vérifier (fmt, clippy, tests, deny, audit) puis la lancer
#   make tauri-check  la vérifier seulement (sans fenêtre)
#   make modules recompiler les modules WASM (app/modules/*.wasm)
#   make essai-bougies  essai réel : ~50 bougies 1 h de chaque connecteur (app native puis coque), sans clé
#   make outils  🧰 construire argos-outils (tools/argos-outils : serveur local, tables générées ; Rust pur, aucun Python)
#   make outils-check  le vérifier seulement (fmt, clippy, tests)
#   make assemble  site v1 : src/ → index.html + app.js (argos-outils assemble) ; assemble-check vérifie, assemble-test écrit aussi _test.html
#   make local   site v1 en local sur http://localhost:8787 (argos-outils local) ; make local-wifi pour le cell
#
# 🐢 Ordi tranquille (règle de Jee) : chaque cargo tourne avec nice -n 19 (le nombre de jobs vient de
# ~/.cargo/config.toml, réglé par Jee : jobs = 10 ; CARGO_BUILD_JOBS=4 pendant que Jee teste), et une seule grosse
# compilation à la fois **sur toute la machine** : verrou flock commun à tous les projets Argos,
# ~/Documents/Argos/.verrou-compilation (créé vide en 0600 s'il manque) ; la deuxième compilation attend la première.
# Avant de prendre le verrou, on attend aussi qu'aucun autre cargo / rustc ne tourne (un projet sans ce verrou).
.PHONY: porte porte-navigateur porte-travail registre fmt check vert audit run run-build watch web web-build hooks clean-cache tauri tauri-build tauri-check tauri-web modules essai-bougies serveur outils outils-check assemble assemble-check assemble-test local local-wifi

CARGO := nice -n 19 cargo
NICE := nice -n 19
LOCK ?= $(HOME)/Documents/Argos/.verrou-compilation
HEAVY := porte porte-navigateur porte-travail registre check vert audit run-build web-build tauri-build tauri-check tauri-web modules essai-bougies serveur outils outils-check

# nos crates seulement : « cargo fmt --all » formaterait aussi les dépendances par chemin, dont la copie figée du kit
# module Argos (vendor/), qui ne doit jamais changer
CRATES := $(patsubst crates/%,-p %,$(wildcard crates/*))

# 📱 coque Tauri 2 : son propre espace de travail (WebKitGTK sous Linux), compilée dans target/tauri ; pas dans make vert
# RUSTSEC-2024-0429 : glib 0.18 (GTK 3 de Tauri sous Linux), itérateur VariantStrIter qu'on n'utilise pas
TAURI := cd argos-tauri && CARGO_TARGET_DIR=../target/tauri

# 🧰 argos-outils : son propre espace de travail (aucune crate d'Argos), compilé dans target/outils
OUTILS_M := --manifest-path tools/argos-outils/Cargo.toml
OUTILS_T := CARGO_TARGET_DIR=target/outils
OUTILS := target/outils/release/argos-outils

fmt:
	$(CARGO) fmt $(CRATES)
	@for m in modules/*/; do $(CARGO) fmt --manifest-path $$m/Cargo.toml; done
	$(CARGO) fmt $(OUTILS_M)
	$(CARGO) fmt --manifest-path argos-tauri/Cargo.toml

watch:
	@command -v cargo-watch >/dev/null || { echo "👉 Il manque cargo-watch : tape une fois  cargo install cargo-watch  puis relance  make watch"; exit 1; }
	$(CARGO) watch -c -w crates -x 'run -p argos-desktop'

# l'app tourne HORS du verrou : seul le build le prend (sinon, tant que l'app est ouverte, toute autre compilation attend)
run: run-build
	./target/release/argos-desktop

tauri: tauri-check tauri-build
	cd argos-tauri && ../target/tauri/debug/argos-tauri

web: web-build serveur
	@echo "👉 Ouvre http://localhost:8000/app/  (Ctrl + C pour arrêter)"
	./target/release/argos-serveur 8000 ../dist

# (les hooks du clone principal, pour chaque worktree : une branche qui n'a pas encore le hook ne s'en passe pas)
hooks:
	git config core.hooksPath "$$(cd "$$(git rev-parse --git-common-dir)/.." && pwd)/rust/tools/hooks"
	@echo "✅ make vert tournera avant chaque git commit (si rust/ change), make check avant chaque git push,"
	@echo "   et interface-2 n'ira plus que vers un contenu tamponné par make porte"

clean-cache:
	bash tools/argos_nettoyage.sh

# chaque grosse commande se relance elle-même sous le verrou (ARGOS_LOCKED : pas de double verrou dans make vert)
ifndef ARGOS_LOCKED
$(HEAVY):
	@[ -e "$(LOCK)" ] || (umask 077 && touch "$(LOCK)")
	@flock -n "$(LOCK)" true || echo "⏳ Une autre compilation tourne : j'attends qu'elle finisse…"
	@while pgrep -x cargo >/dev/null || pgrep -x rustc >/dev/null; do echo "⏳ Une autre compilation tourne (cargo ailleurs) : j'attends…"; sleep 10; done
	@ARGOS_LOCKED=1 flock "$(LOCK)" $(MAKE) --no-print-directory $@
else
check:
	$(CARGO) fmt $(CRATES) --check
	@for m in modules/*/; do $(CARGO) fmt --manifest-path $$m/Cargo.toml --check || exit 1; done
	$(CARGO) fmt --manifest-path argos-tauri/Cargo.toml --check
	$(CARGO) clippy --workspace --all-targets -- -D warnings
	rustup target add wasm32-unknown-unknown
	$(CARGO) clippy -p argos-desktop --target wasm32-unknown-unknown -- -D warnings
	$(CARGO) test --workspace -- --skip tests::vitesse::the_cell_stays_as_fast_as_its_reference
	@# 🏎 the speed guard alone, after the others: a test running beside it would slow it (its time is compared with its
	@# reference, measured alone); --workspace --bins: the very binaries just built, nothing compiled again. It must
	@# have run, once: moved out of the programs --bins builds, or renamed, it would answer « 0 passed » and stay green
	$(CARGO) test --workspace --bins -- --exact tests::vitesse::the_cell_stays_as_fast_as_its_reference --test-threads=1 > target/vitesse.log 2>&1; r=$$?; cat target/vitesse.log; [ $$r -eq 0 ] || exit $$r; \
	[ "$$(grep -c '^test result: ok\. 1 passed;' target/vitesse.log)" = 1 ] || { echo "⛔ make check : le test de vitesse (tests::vitesse::the_cell_stays_as_fast_as_its_reference) n'a pas tourné une fois (« 1 passed » attendu) : déplacé hors des programmes, ou renommé ?"; exit 1; }
	@# 🧩 nos modules (kit module Argos) : chacun son espace de travail, compilé dans target/modules
	@for m in modules/*/; do CARGO_TARGET_DIR=target/modules $(CARGO) clippy --manifest-path $$m/Cargo.toml --all-targets -- -D warnings && CARGO_TARGET_DIR=target/modules $(CARGO) test --manifest-path $$m/Cargo.toml || exit 1; done
	@$(MAKE) --no-print-directory outils-check

# 🛡 porte de qualité (Jee, 24 nov. 2026 : « la version n'entre pas tant qu'elle ne respecte pas tous les garde-fous »),
# obligatoire avant toute fusion dans interface-2 : make vert (fmt, clippy, tous les tests dont le filet d'écrans, la
# bande noire, la netteté et la vitesse ; audit, licences, secrets, aucun .py), gitleaks, le registre des bugs, le travail
# compté des gestes (valgrind : plus de 1 % au-dessus du plancher = rouge), puis l'app web dans un vrai navigateur.
# Tout vert : un tampon pour ce contenu exact (tools/porte/tampon.sh), sans lequel le hook
# reference-transaction refuse de bouger interface-2. Rouge : la branche est corrigée et repasse la porte.
porte:
	@bash tools/porte/tampon.sh debut
	@# 🔑 the secrets of the whole history are checked, never skipped: without gitleaks the gate stays red
	@command -v gitleaks >/dev/null || { echo "⛔ porte : gitleaks absent, les secrets de l'historique ne seraient pas vérifiés (https://github.com/gitleaks/gitleaks)"; exit 1; }
	@$(MAKE) --no-print-directory vert
	@$(NICE) bash tools/porte/registre.sh
	@$(NICE) bash tools/porte/travail.sh
	@$(MAKE) --no-print-directory porte-navigateur
	@bash tools/porte/tampon.sh fin

# 🧮 the work of the gestures, counted in instructions (tools/porte/travail.sh): more than 1 % above its floor is red
porte-travail:
	$(NICE) bash tools/porte/travail.sh

registre:
	$(NICE) bash tools/porte/registre.sh

# 🛡 l'app web construite depuis cet arbre, servie sur 127.0.0.1:8012 avec des réglages jetables (un dossier neuf
# dans target/, jeté avec lui ; jamais ceux de Jee), mesurée dans Chromium en iPhone 13 au doigt
# (tools/porte/navigateur.mjs), puis le serveur arrêté
porte-navigateur: web-build serveur
	@T=$$(mktemp -d -p "$(CURDIR)/target" porte-navigateur.XXXXXX); \
	HOME="$$T" XDG_CONFIG_HOME="$$T" XDG_DATA_HOME="$$T" XDG_CACHE_HOME="$$T" ARGOS_REGLAGES="$$T" ./target/release/argos-serveur 8012 ../dist > "$$T/serveur.log" 2>&1 & s=$$!; \
	for i in $$(seq 1 40); do curl -s -o /dev/null http://127.0.0.1:8012/app/ && break; sleep 0.5; done; \
	node tools/porte/navigateur.mjs http://127.0.0.1:8012; r=$$?; \
	kill $$s 2>/dev/null; wait $$s 2>/dev/null; exit $$r

# 🧰 argos-outils (Rust pur) : le construire, le vérifier
outils:
	$(OUTILS_T) $(CARGO) build --release $(OUTILS_M)

outils-check:
	$(CARGO) fmt $(OUTILS_M) --check
	$(OUTILS_T) $(CARGO) clippy $(OUTILS_M) --all-targets -- -D warnings
	$(OUTILS_T) $(CARGO) test $(OUTILS_M)

# 🧩 site v1 (src/ → index.html + app.js) et 🛰️ son serveur local : argos-outils (Rust), hors du verrou une fois construit
assemble: outils
	$(OUTILS) assemble
assemble-check: outils
	$(OUTILS) assemble --check
assemble-test: outils
	$(OUTILS) assemble --test
local: outils
	$(OUTILS) local
local-wifi: outils
	$(OUTILS) local --wifi

vert: check audit
	@# 🐍 règle de Jee : Rust pur partout, outils compris : aucun fichier .py dans tout le dépôt (suivi par git) ni dans rust/
	@py=$$( (git -C .. ls-files -co --exclude-standard '*.py' '*.pyw'; find . -name '*.py' -not -path './target/*') | sort -u); if [ -n "$$py" ]; then echo "⛔ Fichier Python dans le dépôt (Rust pur, outils compris) :"; echo "$$py"; exit 1; fi
	@command -v cargo-deny >/dev/null || { echo "👉 Il manque cargo-deny : tape une fois  cargo install cargo-deny --locked  puis relance  make vert"; exit 1; }
	$(CARGO) deny check
	@echo "🟢 tout vert : fmt, clippy, tests, audit, deny"

audit:
	@command -v cargo-audit >/dev/null || { echo "👉 Il manque cargo-audit : tape une fois  cargo install cargo-audit --locked  puis relance  make audit"; exit 1; }
	$(CARGO) audit --deny unsound --deny yanked
	@if command -v gitleaks >/dev/null; then cd .. && gitleaks git --redact --no-banner --exit-code 1 .; else echo "ℹ️  gitleaks absent : secrets non vérifiés (https://github.com/gitleaks/gitleaks)"; fi

run-build:
	$(CARGO) build -p argos-desktop --release

# 🛰️ le serveur local de Trading (make web) : argos-outils + journal chiffré
serveur:
	$(CARGO) build -p argos-serveur --release

web-build:
	$(NICE) bash tools/argos_web_build.sh

modules:
	$(NICE) bash tools/argos_module_exemple.sh

# 🌉 pages de la coque (argos-tauri/web, refait à chaque fois, pas dans git) : la page T0 (ui/) et la chart complète
# (T1 : l'app WASM de dist/app, construite par make web-build si elle manque)
tauri-web:
	@[ -f ../dist/app/pkg/argos_bg.wasm ] || $(MAKE) web-build
	rm -rf argos-tauri/web && mkdir -p argos-tauri/web && cp argos-tauri/ui/* argos-tauri/web/ && cp -r ../dist/app argos-tauri/web/app

tauri-check: tauri-web
	@pkg-config --exists webkit2gtk-4.1 || { echo "👉 Il manque WebKitGTK : sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev"; exit 1; }
	$(TAURI) $(CARGO) fmt --check
	$(TAURI) $(CARGO) clippy --all-targets -- -D warnings
	$(TAURI) $(CARGO) test
	$(TAURI) $(CARGO) deny check
	$(TAURI) $(CARGO) audit --deny unsound --deny yanked --ignore RUSTSEC-2024-0429
	@echo "🟢 coque Tauri : fmt, clippy, tests, deny, audit"

# l'app lancée : toujours la chart WASM du moment
tauri-build: web-build tauri-web
	$(TAURI) $(CARGO) build

essai-bougies: tauri-web
	$(CARGO) run -q -p argos-data --features native --example essai_bougies -- "app native"
	$(TAURI) $(CARGO) run -q --example essai_bougies -- "coque Tauri"
endif
