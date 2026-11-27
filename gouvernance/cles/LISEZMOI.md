# Clés publiques de signature des versions

Une clé **Ed25519 publique** par mainteneur, en base64 (32 octets), un fichier par personne : `<github>.ed25519.pub`.
Elles serviront à vérifier les versions signées de l'app (voir `gouvernance/mises-a-jour-signees.md`).

| Mainteneur | Fichier | Ajoutée le |
|---|---|---|
| Jee (`valleeja`) | [`valleeja.ed25519.pub`](valleeja.ed25519.pub) | 2026-10-28 |

Seules les clés **publiques** vont ici. Une clé privée n'est jamais mise dans le dépôt ni dans les secrets GitHub.

Vérifier une signature (`manifeste.sig` brut, 64 octets) avec OpenSSL :

```
(printf '\x30\x2a\x30\x05\x06\x03\x2b\x65\x70\x03\x21\x00'; base64 -d valleeja.ed25519.pub) > cle.der
openssl pkeyutl -verify -pubin -keyform DER -inkey cle.der -rawin -in manifeste.json -sigfile manifeste.sig
```
