# 🛡️ Sécurité d'Argos

## Signaler une faille

**Ne publie pas une faille dans une issue publique.** Utilise le signalement privé de GitHub :
onglet *Security* ▸ *Report a vulnerability*. Décris le problème, comment le reproduire et son impact ; on répond le
plus vite possible et on te crédite (si tu veux) quand c'est corrigé.

## Ce qu'Argos ne garde jamais

| Donnée | Où elle reste |
|---|---|
| Clés API des exchanges | trousseau du système (desktop) ; jamais dans un fichier, jamais dans le dépôt |
| Clé API Claude | chiffrée (AES-256-GCM, clé tirée de ton code par PBKDF2-SHA256, 200 000 tours) sur ton appareil ; ouverte en mémoire le temps de la session |
| 24 mots, clés privées, phrase secrète | jamais demandés, jamais gardés ; Hyperliquid : adresse publique seulement |
| Ordres réels | aucun : le trading d'Argos est démo seulement |

## Ce qui est vérifié automatiquement

En local avant chaque pull request (`make check` et `make audit` dans `rust/`), et sur GitHub avec le workflow
[`Sécurité`](.github/workflows/securite.yml), lancé à la main (onglet *Actions*) pour économiser les minutes payantes :

- **dépendances** : failles connues de toutes les bibliothèques Rust (base RustSec, `cargo audit`) ;
- **secrets** : aucun jeton ni clé dans le code et tout l'historique (`gitleaks`) ;
- **code** : `clippy` et tous les tests.

Et chaque semaine, **Dependabot** propose les mises à jour de sécurité des bibliothèques et des actions GitHub.

## Choix de conception

- **Rust sans `unsafe`** dans le code d'Argos : pas de dépassement de mémoire ni d'accès après libération.
- **Argos Script tourne dans un bac à sable** : un script ne peut ni aller sur Internet, ni lire tes fichiers ou tes
  clés, ni passer d'ordres.
- **Assistant Claude** : les pages web et les données de marché sont traitées comme des données, jamais comme des
  instructions ; aucun ordre réel possible.
- **Site** : en-têtes de sécurité (CSP stricte, pas d'iframe), relais en lecture seule.

## Limites connues

- Les vérifications automatiques trouvent les failles **connues** ; elles ne remplacent pas un audit humain.
- Les programmes Solana d'Argos Chain auront un **audit professionnel** avant le réseau principal.
