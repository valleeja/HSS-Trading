# Consignes pour Claude (et tout agent) dans ce dépôt

- **Tout se fait en local d'abord** : travail sur une branche locale, aucune fusion sans « ok fusion » écrit par Jee.
- **Dépôt public GitHub `valleeja/HSS-Trading`** (depuis le 26 nov. 2026) : on n'y pousse qu'une version passée par `make porte` et le fil Contrôle, avec la phrase exacte de Jee. Jamais de secret, jamais l'ancien site v1.
- **Tout en local d'abord** : les workflows GitHub Actions sont manuels (minutes payantes). Ne pas ajouter de
  déclencheur `push` ou `pull_request`.
- **Vérification cargo obligatoire avant chaque fusion** qui touche `rust/` : lancer `cd rust && make check` à la main
  avant chaque fusion. `make hooks` installe aussi un hook qui lance `make vert` avant chaque commit : celui-là tourne bien en local.
  Le hook `make check` installé par `make hooks` se déclenche au push, et on ne pousse jamais : il ne tourne donc pas.
  Il sera remplacé plus tard par celui de l'outil commun Argos. Écrire le résultat dans le compte rendu à Jee
  (par exemple « make check : 313 tests OK »). Ne jamais utiliser `--no-verify`. Si la vérification bloque, arrêter
  et écrire à Jee ce qui bloque, avec la sortie.
- **Nettoyage toutes les 30 min** pendant une longue session : lancer `cd rust && make clean-cache`
  (`rust/tools/argos_nettoyage.sh` : n'efface que des caches qui se refont seuls, garde la compilation récente).
  Le lancer aussi tout de suite si le disque se remplit (« no space left on device »).
- **Réseau** : toute nouvelle adresse appelée par l'app doit être ajoutée à `rust/crates/argos-data/src/net.rs`
  (liste blanche, HTTPS/WSS seulement). Le test `toutes_les_sources_du_code_sont_listees` échoue sinon.
- **Actions GitHub** figées par hash de commit (`uses: owner/action@<sha> # vX`) ; Dependabot les met à jour.
- Gouvernance et zones sensibles : `GOVERNANCE.md`, `.github/CODEOWNERS`.

## Standing orders from Jee (all Argos projects, 17 Nov 2026)

These orders apply to every agent working in this project. Keep them in this file.

1. Every delivered document is copied onto the PC, into this project's folder under `~/Documents/Argos/`,
   with the date and the version in its file name. Give Jee the full path.
2. The project touches nothing outside its own folder. The Argos kit (« le kit Argos ») and the
   `Bibliotheque` folder are read-only.
3. No merge without « ok fusion » written by Jee in the current session. No push to GitHub.
4. Only one compilation at a time, holding the lock `~/Documents/Argos/.verrou-compilation` and running
   under `nice -n 19`.
5. After 3 failed attempts, stop and write to Jee what is blocking, with the command output.
6. Clean up after every change: no temporary files and no build leftovers.
7. Reports for Jee are in French. Code and repository documents are in English.
8. Always use the full names of projects, modules and branches.
9. Every question asked to Jee comes with a recommendation.

### Argos HSS Trading: role

- All order code stays in demo mode, the portfolio stays read-only and the webhook stays signed,
  until Jee decides otherwise.
- All calculations use `rust_decimal`.
- The verification gate waits for Jee's choice on RUSTSEC-2026-0459.

### Copie en français (texte de Jee)

ORDRES PERMANENTS DE JEE : communs à tous les projets Argos
1. Chaque document livré est copié sur le PC dans le dossier du projet sous ~/Documents/Argos/, avec la date et la version dans le nom. Le chemin complet est donné à Jee.
2. Le projet ne touche à rien hors de son propre dossier. Le kit Argos et la Bibliotheque sont en lecture seule.
3. Aucune fusion sans « ok fusion » écrit par Jee dans cette session. Aucun push sur GitHub.
4. Une seule compilation à la fois, avec le verrou ~/Documents/Argos/.verrou-compilation et nice -n 19.
5. Après 3 essais ratés, arrêter et écrire à Jee ce qui bloque, avec la sortie.
6. Ménage après chaque changement : aucun fichier temporaire ni reste de compilation.
7. Rapports pour Jee en français. Code et documents du dépôt en anglais.
8. Toujours les noms complets des projets, modules et branches.
9. Chaque question posée à Jee vient avec une recommandation.

ARGOS HSS TRADING : rôle
Tout code d'ordre reste en démo, portefeuille en lecture seule, webhook signé, jusqu'à décision de Jee. Calculs en rust_decimal. La porte de vérification attend le choix sur RUSTSEC-2026-0459.
