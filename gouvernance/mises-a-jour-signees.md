# Mises à jour signées à seuil : plan technique

Esquisse de la façon dont l'app desktop Rust (`rust/crates/argos-desktop`) vérifiera une mise à jour avant de
l'installer. Rien de ceci n'est encore codé : ce sera une AIP puis des pull requests séparées.

## Ce qu'on veut garantir

1. Une mise à jour n'est installée que si **K mainteneurs sur N** l'ont signée (règle dans
   [`GOVERNANCE.md`](../GOVERNANCE.md), section 6).
2. Une seule clé volée, ou un compte GitHub piraté, ne suffit pas.
3. Personne ne peut faire revenir l'app à une vieille version vulnérable (pas de retour en arrière forcé).
4. L'utilisateur choisit son canal (`stable` / `test`) et peut refuser.

## Le manifeste de version

Chaque version publie un petit fichier `manifeste.json` à côté des installeurs :

```json
{
  "version": "0.3.0",
  "canal": "stable",
  "publiee": "2026-11-15T12:00:00Z",
  "aip": [3, 5],
  "notes": "https://github.com/valleeja/Argos-Trading/releases/tag/desktop-v0.3.0",
  "fichiers": [
    { "plateforme": "windows-x64", "nom": "Argos_0.3.0_x64-setup.exe", "sha256": "…" },
    { "plateforme": "macos-aarch64", "nom": "Argos_0.3.0_aarch64.dmg", "sha256": "…" },
    { "plateforme": "linux-x64", "nom": "Argos_0.3.0_amd64.AppImage", "sha256": "…" }
  ]
}
```

Et un fichier `manifeste.sig` : une signature **Ed25519** par mainteneur, sur les octets exacts de `manifeste.json`.

```json
[
  { "cle": "base64 de la clé publique", "sig": "base64 de la signature" }
]
```

Les installeurs ne sont pas signés un par un : leur empreinte SHA-256 est dans le manifeste signé, ça suffit.

## Où vivent les clés

Les clés publiques des mainteneurs et le seuil sont **compilés dans l'app** (un fichier
`argos-desktop/src/update/cles.rs` généré depuis [`MAINTENEURS.md`](MAINTENEURS.md)). Une nouvelle liste de clés
arrive seulement dans une mise à jour signée par le seuil de l'**ancienne** liste.

## Vérification dans l'app (esquisse Rust)

Bibliothèque prévue : `ed25519-dalek` (pur Rust, sans `unsafe` dans notre code, marche aussi en WASM). `sha2` est déjà
une dépendance.

```rust
use ed25519_dalek::{Signature, VerifyingKey};

pub struct Politique {
    pub cles: &'static [[u8; 32]],
    pub seuil: usize,
}

#[derive(Debug)]
pub enum ErreurMaj {
    Format,
    SeuilNonAtteint { valides: usize, seuil: usize },
    VersionPlusVieille,
    EmpreinteFausse,
}

/// Compte les signatures valides de clés connues (chaque clé ne compte qu'une fois).
pub fn verifier_manifeste(octets: &[u8], sigs: &[(/*cle*/ [u8; 32], /*sig*/ [u8; 64])], p: &Politique) -> Result<(), ErreurMaj> {
    let mut vues: Vec<[u8; 32]> = Vec::new();
    for (cle, sig) in sigs {
        if !p.cles.contains(cle) || vues.contains(cle) {
            continue;
        }
        let Ok(vk) = VerifyingKey::from_bytes(cle) else { continue };
        if vk.verify_strict(octets, &Signature::from_bytes(sig)).is_ok() {
            vues.push(*cle);
        }
    }
    if vues.len() >= p.seuil { Ok(()) } else { Err(ErreurMaj::SeuilNonAtteint { valides: vues.len(), seuil: p.seuil }) }
}
```

Ensuite, dans l'ordre :

1. lire `manifeste.json` et vérifier que `version` est **plus grande** que la version installée (sinon
   `VersionPlusVieille`) et que `canal` correspond au choix de l'utilisateur ;
2. télécharger l'installeur de sa plateforme et comparer son SHA-256 (sinon `EmpreinteFausse`) ;
3. montrer les notes de version et demander « Installer maintenant / Plus tard / Ignorer cette version ».

Tests prévus : seuil atteint, seuil manqué, même clé comptée deux fois, clé inconnue, signature d'un autre texte,
version plus vieille, empreinte fausse.

## Côté publication

- La CI (`.github/workflows/desktop.yml`, lancée à la main) construit les installeurs et écrit `manifeste.json` (elle produit déjà
  `SHA256SUMS.txt`).
- Chaque mainteneur télécharge le manifeste, le relit, et le signe **sur son appareil** avec un petit outil
  (`argos-sign`, à écrire) ; il dépose sa signature dans la Release.
- La Release passe de « pré-version » à « publiée » seulement quand le seuil est atteint.
- La clé privée n'est jamais dans les secrets GitHub.

## Et le site web (`/app`) ?

Le navigateur charge ce que Vercel sert : il ne vérifie pas nos signatures. La protection du web vient de la revue
publique et du fait que seul `main` est déployé. Plus tard on pourra publier l'empreinte du WASM dans le manifeste signé
pour que n'importe qui puisse comparer.
