# Mainteneurs et clés de signature

Les mainteneurs peuvent fusionner dans `main` et signer les versions. Changer cette liste demande une AIP
(voir [`GOVERNANCE.md`](../GOVERNANCE.md), section 6).

**Seuil actuel : 1 signature sur 1.** Objectif : 2 sur 3 dès qu'il y a trois mainteneurs.

| Mainteneur | GitHub | Clé publique de signature (Ed25519) | Depuis |
|---|---|---|---|
| Jee (fondateur) | `valleeja` | *à générer* | 2026-10-28 |

La clé privée ne quitte jamais l'appareil du mainteneur (idéalement une clé matérielle). Elle n'est jamais dans le dépôt,
ni dans les secrets GitHub : la CI construit, les mainteneurs signent.
