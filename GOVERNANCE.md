# 🏛️ Gouvernance d'Argos

Ce fichier dit **qui décide quoi** dans Argos et **comment une amélioration arrive chez les utilisateurs**.
Il vit dans le code source : le changer demande une pull request, comme n'importe quel code.

## 1. Principe : comme Bitcoin, personne ne force une mise à jour

Dans Bitcoin, aucun développeur ne peut imposer une nouvelle version : chaque personne choisit le logiciel qu'elle fait
tourner, et un changement ne passe que s'il est largement accepté. Argos suit la même idée, adaptée à une app :

| Bitcoin | Argos |
|---|---|
| BIP (proposition d'amélioration) | **AIP** (*Argos Improvement Proposal*), voir [`gouvernance/AIP-0000-modele.md`](gouvernance/AIP-0000-modele.md) |
| Revue publique du code, « rough consensus » | Revue publique de la pull request, consensus des mainteneurs (section 4) |
| Versions signées par plusieurs développeurs | Versions signées par **plusieurs clés de mainteneurs** (seuil, section 6) |
| Chaque nœud choisit sa version | Chaque utilisateur choisit son **canal** (stable / test) et peut refuser ou reporter une mise à jour |
| Le code est libre, on peut forker | Depuis le 2026-11-06 : PolyForm Noncommercial 1.0.0 (usage non commercial permis, usage commercial avec l'accord de Jee) ; les versions déjà publiées sous GPL-3.0 le restent |

**Ce qui est différent d'une blockchain** : Argos n'a pas de registre partagé que tout le monde doit garder identique.
Deux utilisateurs sur deux versions différentes n'abîment rien. Le « consensus » sert donc à décider **ce qui entre dans
la version officielle**, pas à garder un réseau d'accord. Pas besoin de minage ni de jeton pour ça.

## 2. Rôles

| Rôle | Qui | Ce qu'il peut faire |
|---|---|---|
| **Utilisateur** | Tout le monde | Choisir son canal, refuser une mise à jour, ouvrir une issue, commenter une AIP |
| **Contributeur** | Toute personne qui ouvre une pull request | Proposer du code, une AIP, une traduction, un guide |
| **Relecteur** | Contributeur reconnu (au moins 3 PR fusionnées) | Approuver une pull request sur les parties qu'il connaît |
| **Mainteneur** | Liste dans [`gouvernance/MAINTENEURS.md`](gouvernance/MAINTENEURS.md) | Fusionner dans `main`, signer une version avec sa clé |
| **Fondateur** | Jee (`valleeja`) | Mainteneur ; tranche quand il n'y a pas de consensus (section 4), tant qu'il y a moins de 3 mainteneurs |

Aujourd'hui Argos a **un seul mainteneur**. Le pouvoir du fondateur de trancher disparaît dès qu'il y a 3 mainteneurs :
les décisions passent alors au vote des mainteneurs (section 4).

## 3. Comment un changement avance

1. **Petit changement** (bug, texte, traduction, petite amélioration) : une pull request suffit. Voir
   [`CONTRIBUTING.md`](CONTRIBUTING.md).
2. **Gros changement** (nouveau module, changement de format de fichier, sécurité, trading, comptes, mises à jour,
   règles de cette gouvernance) : d'abord une **AIP**.
   - On copie le modèle dans `gouvernance/aip/AIP-XXXX-titre.md` et on ouvre une pull request avec seulement ce texte.
   - Discussion publique sur la pull request pendant **au moins 7 jours**.
   - Statuts : `Brouillon` → `Discussion` → `Acceptée` ou `Refusée` → `Livrée` (quand le code est dans une version).
3. Le code de l'AIP arrive ensuite en une ou plusieurs pull requests qui citent son numéro.

## 4. Comment on décide

- **Consensus d'abord** : une pull request est fusionnée quand les vérifications locales passent (voir [`CONTRIBUTING.md`](CONTRIBUTING.md)), qu'au moins **un mainteneur** l'approuve
  (deux pour les zones sensibles, section 5), et qu'aucune **objection argumentée** d'un mainteneur ou d'un relecteur
  ne reste sans réponse.
- **Pas de consensus après 14 jours** : vote des mainteneurs, majorité des deux tiers. Tant qu'il y a moins de 3
  mainteneurs, le fondateur tranche et écrit pourquoi dans la pull request.
- **Changer cette gouvernance** : AIP obligatoire, 14 jours de discussion, accord de tous les mainteneurs sauf un au plus.
- **Urgence sécurité** : un mainteneur peut fusionner et publier tout de suite un correctif ; il l'explique publiquement
  dès que la faille est corrigée (voir [`SECURITY.md`](SECURITY.md)).

Les votes des utilisateurs (par exemple avec les comptes Nostr d'Argos) peuvent **éclairer** une décision, mais ne la
remplacent pas : un vote en ligne se truque facilement avec de faux comptes. Bitcoin ne vote pas non plus.

## 5. Zones sensibles (deux approbations)

Ces dossiers demandent l'accord de **deux** mainteneurs dès qu'il y en a deux, et sont listés dans
[`.github/CODEOWNERS`](.github/CODEOWNERS) :

- comptes, clés, chiffrement : `api/_nip98.mjs`, `api/me.mjs`, `rust/crates/argos-desktop/src/security/` ;
- trading et prop firms : `rust/crates/argos-propfirm/` ;
- publication des versions : `.github/workflows/` ;
- cette gouvernance : `GOVERNANCE.md`, `gouvernance/`.

## 6. Versions et mises à jour signées

- **Numéros** : `MAJEUR.MINEUR.CORRECTIF` (SemVer). MAJEUR casse quelque chose (format de fichier, profil), MINEUR
  ajoute, CORRECTIF répare.
- **Canaux** :
  - `test` : chaque version candidate (`-rc.N`), pour ceux qui veulent essayer avant tout le monde ;
  - `stable` : une version `test` sans bug bloquant pendant **7 jours** passe en stable.
- **Signature à seuil** : une version n'est officielle que si elle est signée par **au moins K des N clés** listées dans
  [`gouvernance/MAINTENEURS.md`](gouvernance/MAINTENEURS.md) (aujourd'hui 1 sur 1 ; objectif 2 sur 3). Une seule clé
  volée ne suffit donc pas à pousser une fausse mise à jour.
- **L'app vérifie avant d'installer** : l'app desktop télécharge la version et les signatures, compte les signatures
  valides avec les clés qu'elle connaît, et refuse si le seuil n'est pas atteint. Détails techniques :
  [`gouvernance/mises-a-jour-signees.md`](gouvernance/mises-a-jour-signees.md).
- **L'utilisateur garde la main** : il voit les changements, peut reporter, rester sur sa version, ou changer de canal.
- **Changer la liste des clés** passe par une AIP, et la nouvelle liste doit elle-même être signée par le seuil de
  l'ancienne.

Le site web (Vercel) est servi par l'hébergeur : le navigateur fait confiance au site, pas aux signatures. Pour le web, la
garantie vient de la revue publique du code et du fait que seul `main` est déployé.

## 7. Transparence

- Tout se passe en public : issues, pull requests, AIP, notes de version.
- Chaque version publie ses empreintes (`SHA256SUMS.txt`) et ses signatures.
- Ce fichier fait foi. En cas de doute, on ouvre une issue.
