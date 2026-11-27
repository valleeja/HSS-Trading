# Plan : collecte progressive des bougies (Argos HSS Trading)

> Approuvé par Jee le 6 nov. 2026, avec 5 ajouts (validation des bougies et taille maximale des réponses, fichiers
> protégés et vérifiés, tests jamais dans le vrai cache, format compact à l'étape 5, direct branché tout de suite).
> Une branche par étape, `make vert` + `make essai-bougies`, puis le « ok fusion » de Jee. Rien n'est poussé.

## Point de départ

- Premier écran : le cache de la dernière session s'affiche tout de suite, puis on télécharge seulement ce qui manque.
- Cache : un fichier par exchange, symbole et unité de temps, dans le dossier **propre à Argos HSS Trading**
  (`~/.cache/argos-hss-trading/candles`, 0700 ; avant le 7 nov. : `~/.cache/argos/candles`, plus jamais touché), écriture sûre
  (fichier temporaire puis renommage).
- Passé : quand on dézoome ou qu'on recule, une page de 3000 bougies à la fois, au plus une toutes les 0,7 s, jamais
  pendant un geste ; 200 000 bougies au plus en mémoire.
- Chaque exchange a sa taille de page et sa pause ; le superviseur prend une source de secours en cas de panne.
- Les unités absentes chez l'exchange sont regroupées à partir d'une plus petite.

## Règles

1. **Premier écran rapide** : le cache, puis seulement les bougies manquantes (au plus N récentes).
2. **Direct tout de suite** : la connexion en direct se branche juste après le premier écran, **avant** de remonter
   dans le passé.
3. **Passé en arrière-plan** : seulement si on en a besoin (dézoom, défilement, indicateur), une page à la fois, avec la
   fiche de chaque exchange (taille de page, pause, limite) ; erreur 429 ou délai : attente qui double (2 s → 60 s),
   puis abandon sans bloquer la chart ; une seule file, basse priorité.
4. **Bougies vérifiées** avant d'aller au cache : haut ≥ bas, ouverture et clôture entre les deux, volume ≥ 0, heures
   croissantes. Une réponse d'exchange a une **taille maximale** : une réponse folle est refusée, sans planter. Une
   bougie refusée est **notée dans le journal** (sans rien de sensible) et devient un **trou à réparer** ; **plus de
   5 %** de bougies refusées dans une réponse : elle est rejetée en entier et on passe à l'exchange de secours.
5. **Cache** : seulement des bougies **fermées** (la bougie en cours reste en mémoire) ; fichiers en **0600** avec une
   **empreinte** : un fichier abîmé ou modifié est jeté puis rechargé, sans planter ; **jamais** de clé ni de jeton
   dans le cache ou les journaux ; plafond par fichier et total (retrait des moins utilisés) ; version du format.
6. **Trous** : vérifiés après le premier écran et après le remplissage du passé, redemandés seulement pour la plage
   manquante (avec Retry-After et jitter) ; un **marché fermé** (week-end, 25 décembre, 1er janvier pour l'or, les
   indices et les actions) n'est pas réparé ; un trou que l'exchange n'a pas est noté une fois et devient « connu ».
7. **Reprise après coupure** : seulement ce qui manque depuis la dernière bougie fermée ; hors ligne, la chart reste
   sur le cache avec la mention « hors ligne ».
8. **Unités construites** à partir de la plus petite disponible ; seule l'unité source va au cache.
9. **Écrans partagés** : même cache et même file, une seule requête pour deux fenêtres.
10. **Paires calculées** : chaque jambe a son cache ; seulement les heures présentes dans toutes les jambes ; la paire
    n'est pas mise en cache ; un trou d'une jambe reste un trou.
11. **Coque Tauri** : toute la collecte dans le cœur Rust ; la page reçoit seulement les bougies prêtes, par l'IPC.
12. **Modules** : une fenêtre de bougies récentes sous la limite de 4 Mio (avec l'échauffement demandé), jamais tout
    l'historique.
13. **Tests** : sans réseau, et **jamais dans le vrai `~/.cache`** (dossier temporaire) ; `make essai-bougies` vérifie
    chaque source et le cache.

## Étapes (une branche chacune)

| # | Étape | Effort | État |
|---|---|---|---|
| 1 | Le cache ne garde que les bougies fermées ; tests dans un dossier temporaire | petit | ✅ fusionnée |
| 1b | Bougies vérifiées + taille maximale des réponses ; fichiers 0600 + empreinte, abîmé = jeté et rechargé ; aucune clé dans le cache ni les journaux | petit à moyen | ✅ fusionnée |
| 2 | Fiche par exchange (page, pause) + attente qui double sur 429 (2 s → 60 s) ; **bougie refusée notée dans le journal** (exchange, symbole, unité, heure, raison ; jamais d'adresse ni de clé) et gardée comme **trou à réparer** (étape 3) ; **plus de 5 % de refusées = réponse rejetée**, bascule sur le secours | petit | ✅ fusionnée |
| 2b | Premier écran de 500 bougies, direct affiché tout de suite (même sur le cache pendant le chargement), puis le passé rempli en arrière-plan jusqu'à 5000 ; **Retry-After** de l'exchange respecté (secondes ou date HTTP, 60 s au plus) et petit **délai au hasard** (jitter) ajouté aux attentes croissantes (commit à part) | petit | ✅ fusionnée |
| 3 | Détection et réparation des trous (dont les bougies refusées de l'étape 2) ; **marché fermé** (week-end, 25 déc., 1er janv.) jamais réparé pour l'or, les indices et les actions (HIP-3 `xyz:`), la crypto n'en a pas ; trou que l'exchange n'a pas = noté une fois, jamais redemandé ; au plus 10 trous par passage | moyen | ✅ fusionnée |
| 4 | File commune (`argos-data/file.rs`) : premier écran en haute priorité (jamais en file), direct jamais en file ; passé, pages anciennes, trous et cases du split en basse priorité, **une à la fois par exchange** et seulement après le premier écran ; deux cases sur le même marché = **une seule demande** | moyen | ✅ fusionnée |
| 5 | Format **2 binaire** (48 octets par bougie, empreinte SHA-256, 0600), version lue ; format 1 (texte) relu une fois puis remplacé, autre version jetée ; au plus 200 000 bougies par fichier et 200 Mio en tout (retrait des moins utilisés) ; temps de chargement mesuré avant et après (`make essai-bougies`). Binaire plutôt qu'une base : la couche de données sert aussi au navigateur (WASM), où une base sur fichier ne tourne pas | moyen | ✅ fusionnée |
| 6 | Reprise après coupure : direct qui se reconnecte seul (2 s → 30 s + hasard), refait aussi après 60 s sans aucun message (sortie de veille), rattrapage des bougies manquées puis réparation des trous ; étiquette « 📴 Hors ligne » sur la chart (qui reste utilisable avec le cache) ; test de robustesse : des centaines de fichiers abîmés ou au hasard refusés proprement | moyen | ✅ fusionnée |
| 7 | Fenêtre des modules : les bougies les plus récentes qui tiennent dans les limites du kit (50 000 bougies et 4 Mio une fois encodées, taille réelle mesurée : 50 000 bougies = 2,29 Mio) ; au-delà, les plus anciennes sont laissées de côté et la légende du module dit « · 50000 dernières bougies » ; kit inchangé | petit | ✅ fusionnée |
| 8 | Coque : la même collecte dans le cœur Rust (`argos-data/collecte.rs`, partagée) : cache du dossier propre, premier écran en haute priorité, passé et trous en basse priorité dans un fil à part, hors ligne = le cache ; fenêtre de l'étape 7 pour le module ; la page n'a toujours ni réseau ni clé | moyen | ✅ fusionnée |
| 9 | `make essai-bougies` : pour **chaque** exchange, 2e passage depuis le cache, remontée de 3 pages (pauses de la fiche respectées), trou créé exprès réparé ; il a trouvé que WEEX ignorait `endTime` (passé et trous impossibles) : corrigé avec `historyKlines` | petit | ✅ branche `collecte-9-essai` (attend « ok fusion ») |

## Revue R&D (7 nov. 2026) — branche `collecte-revue-rd`

| Point | État |
|---|---|
| 1. Le secours ne pollue pas le cache | corrigé : le passé d'un exchange d'appoint ne va jamais dans le cache de l'exchange de la chart (`debut_propre`) |
| 2. Horloge et dernières bougies | corrigé : « fermée » = fin + 2 min ; les 3 dernières bougies du cache sont redemandées |
| 3. Ajouter sans tout réécrire | corrigé : morceaux de 10 000 bougies, seul le morceau qui change est réécrit (160 Ko au lieu de 9,6 Mo) |
| 4. Contrôle à la lecture | déjà fait (étapes 1b, 5, 6) |
| 5. Paires calculées approximatives | corrigé : « calculé, approximatif » partout, jamais en cache |
| 6. Réponse vide ou erreur | déjà fait (étapes 1b, 2, 6) ; 2 tests de plus |

## Pour plus tard (notes de Jee, 7 nov. 2026)

- ✅ **Jours fériés par marché** (branche `collecte-feries`) : actions et indices américains = jours fériés de la
  bourse de New York (MLK, Presidents Day, Vendredi saint, Memorial Day, Juneteenth, 4 juillet, Labor Day,
  Thanksgiving, Noël, 1er janvier ; samedi → vendredi, dimanche → lundi) ; métaux, énergie et forex = Vendredi saint en
  plus ; Japon : week-end, Noël et 1er janvier seulement (ses fêtes pas encore). Pâques et Thanksgiving calculés,
  testés de 2026 à 2028 (et le jour de la semaine de 1990 à 2099).
- ✅ **Trou « impossible » réessayé une fois par semaine** (branche `collecte-trou-semaine`) : le trou est noté avec
  l'heure de l'essai, laissé de côté 7 jours puis redemandé ; les trous connus sont gardés à côté du cache de la série
  (`….trous`, 0600, 1000 au plus) et survivent à un redémarrage (app de bureau et coque).
- **Relais du site pour WEEX** (version en ligne seulement) : il ne demande que les bougies récentes ; le passé de
  WEEX dans le navigateur demanderait `historyKlines` dans `api/exchange.mjs`.
- **Limites des API publiques** vues par l'essai (pas des pannes) : Kraken ne rend que ses 720 dernières bougies,
  Bitget ne remonte qu'environ un mois en 1 h ; au-delà, l'app prend le passé chez Binance (même perp).
- **Qualité des données Bitunix** : quelques bougies anciennes incohérentes (haut sous la clôture…) ; elles sont
  refusées et notées (bien sous 5 %).
