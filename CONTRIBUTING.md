# 🤝 Contribuer à Argos

Merci ! Argos cherche des collaborateurs pour continuer le projet (voir le [lisez-moi](README.md)). Toute aide compte :
code, bug signalé, traduction, guide de l'Académie. Les règles de décision sont dans [`GOVERNANCE.md`](GOVERNANCE.md).

**Commence par [`docs/CONTINUER-EN-RUST.md`](docs/CONTINUER-EN-RUST.md)** : comment compiler, les tests cargo, la porte de
qualité (`make porte`), le registre des bugs, les garde-fous et les pièges connus.

## Avant de commencer

- **Une faille de sécurité ?** Pas d'issue publique : suis [`SECURITY.md`](SECURITY.md).
- **Un gros changement ?** (nouveau module, format de fichier, comptes, trading, mises à jour) Écris d'abord une
  **AIP** avec le modèle [`gouvernance/AIP-0000-modele.md`](gouvernance/AIP-0000-modele.md).
- **Petit changement ?** Ouvre directement une pull request.

## Le chemin d'une pull request

1. Fais un fork, puis une branche à partir de `main` (personne ne pousse directement sur `main`).
2. Garde la pull request **petite et sur un seul sujet**. Mieux vaut trois petites PR qu'une énorme.
3. **Vérifie tout en local** : GitHub ne lance plus rien tout seul (les minutes GitHub Actions sont payantes), c'est
   donc toi qui fais la vérification avant d'ouvrir la PR.
   - **Rust** (`rust/`) : `make check` (formatage, clippy strict desktop + navigateur, tous les tests) et `make audit`
     (failles connues des dépendances, secrets) ;
   - **App navigateur** : `make web-build` dans `rust/`, puis `./lancer-serveur.sh` à la racine du dépôt, et ouvre http://localhost:8000/app2/ ;
   - **Avant la fusion** : `make porte` (la porte de qualité complète, environ 15 min) ; un bug corrigé a sa ligne au
     registre `rust/tools/porte/registre.txt` et son test ;
   - `make hooks` (une fois par clone) rend `make check` **obligatoire avant chaque `git push`** qui touche `rust/`.
   - Toute nouvelle adresse réseau appelée par l'app va dans la liste blanche `rust/crates/argos-data/src/net.rs`.
   - Écris dans la PR les commandes lancées et leur résultat (par exemple « make check : 313 tests OK »).
4. Décris dans la pull request : quoi, pourquoi, comment tu l'as testé, et le numéro d'AIP s'il y en a une.
5. Un mainteneur relit. Réponds aux commentaires en poussant sur la même branche.
6. Quand les vérifications locales sont bonnes et que la relecture est approuvée, un mainteneur fusionne.

Les workflows GitHub (`Sécurité`, `Argos Web`, `Version de test`…) existent toujours, mais seulement **à la main** :
onglet *Actions* ▸ le workflow ▸ *Run workflow*. Un mainteneur peut s'en servir avant une version.

## Style

- Textes de l'app et documentation **en français**, simples et clairs.
- Rust : pas de `unsafe`, pas de `unwrap()` dans le code de l'app (seulement dans les tests), erreurs remontées avec `?`.
- Un test pour chaque bug corrigé quand c'est possible.
- Messages de commit : `type(zone) : ce qui change`, par exemple `fix(chart) : la bougie en direct ne saute plus`.

## Licence et CLA (obligatoire)

Le projet est sous **PolyForm Noncommercial 1.0.0** (voir [`LICENSE`](LICENSE)), © 2026 Jee ; Jee relit et fusionne les contributions. **Toute contribution exige d'accepter
l'accord de licence de contributeur [`CLA.md`](CLA.md)** (français et anglais). Tu gardes ton droit d'auteur. Tu
accordes au propriétaire du projet une licence perpétuelle et irrévocable, avec le droit de relicencier, y compris
sous une licence commerciale. Tu confirmes aussi que ta contribution est originale.

Pour l'accepter :

1. **Signe chaque commit** avec `git commit -s` : il ajoute la ligne `Signed-off-by: Ton Nom <ton-adresse>`.
2. **Coche la case** « J'accepte le CLA d'Argos (CLA.md) » dans la description de la pull request (elle est dans le
   modèle de PR).

Une pull request sans ces deux éléments n'est pas fusionnée.
