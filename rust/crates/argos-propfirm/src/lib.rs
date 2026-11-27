//! # Argos Prop firm — Layer 4 (neutre)
//! `rules` : les profils de challenge (2 étapes, 1 étape, Instantané par défaut), modifiables et dupliquables.
//! Les défauts viennent de `propfirm-defaults.json` (versionné, approuvé par Jee) ; une nouvelle version des défauts
//! remplace seulement les profils par défaut, jamais ceux que tu as modifiés.

#![forbid(unsafe_code)]

pub mod examen;
pub mod rules;
