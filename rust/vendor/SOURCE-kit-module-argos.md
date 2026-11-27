# Copie figée du kit module Argos

| | |
|---|---|
| Dossier | `rust/vendor/kit-module-argos-1.0.2/` |
| Version | kit-module-argos **1.0.2** (2026-11-06), fichier `VERSION` du kit |
| Source | version principale `7b7deeb` (interface v2 ; bulle et politique sous PolyForm Noncommercial 1.0.0 ; SDK MIT ou Apache-2.0) |
| Copié depuis | `~/Documents/Argos/Argos/Bibliotheque/kit-module-argos/` (lecture seule) |
| Vérification | `sha384sum -c --ignore-missing SHA384SUMS` dans le dossier : 56 fichiers OK (le binaire `argos-modules` n'est pas copié) |

Changements depuis 1.0.0 (copie précédente, remplacée le 2026-11-06) :
- `argos-sandbox` et `argos-module-policy` : licence **PolyForm Noncommercial 1.0.0** (`LICENSE.md`, `COMMERCIAL.md`)
  au lieu de la licence propriétaire ; `argos-module-sdk` reste **MIT OR Apache-2.0** ;
- le test d'indépendance de la bulle liste les seules crates permises (au lieu des crates privées interdites) ;
- textes (LISEZ-MOI, INTERFACE-V2, VERIFICATION-PROJETS, commentaires) ; **aucun changement de code** dans le SDK, la
  politique ou la bulle : l'interface v2, le format et les vecteurs de conformité sont les mêmes.

Correctif local du 2026-11-19 (OK écrit de Jee : « ok kit wasmtime 50.0.2 ») :
- une seule ligne changée : `argos-sandbox/Cargo.toml`, `wasmtime = "=50.0.1"` → `"=50.0.2"` (avis RUSTSEC-2026-0566 à
  0573, version corrective, même interface) ; la ligne de ce fichier dans `SHA384SUMS` est mise à jour ;
- à faire : le kit officiel de la Bibliothèque (`kit-module-argos`) est à passer à wasmtime 50.0.2 par son propre fil ;
  ensuite, revenir à une copie fidèle de ce kit.

Règles :
- **Jamais modifiée** (sauf le correctif local ci-dessus). Elle est hors de l'espace de travail (`exclude = ["vendor"]`) et `make fmt` / `make check` ne
  formatent que nos crates. Une mise à jour du kit = une nouvelle copie vérifiée, dans un commit à part.
- Le build ne dépend jamais du dossier de la Bibliothèque : seulement de cette copie.
- Licences : voir `LICENCES-TIERS.md`.
