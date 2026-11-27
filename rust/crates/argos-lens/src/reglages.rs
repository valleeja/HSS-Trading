//! ⚙️ Réglages de Lens dans l'app (fenêtre de réglages de l'indicateur, comme les autres) : les 5 poids du score et
//! l'unité de temps du biais. Les valeurs sont dans l'ordre de `PARAMS`.

use crate::{Config, Weights};
use argos_core::Timeframe;
use argos_script::inputs::{choice, int, Inputs, Param};

pub const GROUPS: [&str; 3] = ["⚖️ Poids du score (total 100)", "🧭 Biais", "🖼 Carte"];

/// Unités de temps proposées pour le biais (`None` = pas de biais).
const BIAIS: [Option<Timeframe>; 4] = [None, Some(Timeframe::H1), Some(Timeframe::H4), Some(Timeframe::D1)];

pub const PARAMS: [Param; 8] = [
    int("trend", 0, "Tendance", 20.0, 0.0, 100.0),
    int("momentum", 0, "Momentum", 20.0, 0.0, 100.0),
    int("strength", 0, "Force", 20.0, 0.0, 100.0),
    int("volume", 0, "Volume", 20.0, 0.0, 100.0),
    int("structure", 0, "Structure", 20.0, 0.0, 100.0).tip("La somme des 5 poids doit faire 100."),
    choice("htf", 1, "Unité du biais EMA", 2, &["Aucune", "1h", "4h", "1D"]),
    choice("place", 2, "Emplacement", 0, &EMPLACEMENTS)
        .tip("Coin de la zone des prix où se pose la carte (Auto : comme avant ; en haut, sous la légende ; jamais sur l'échelle\nde prix, l'axe du temps ni le bouton Argos AI)."),
    choice("taille", 2, "Taille", 0, &TAILLES)
        .tip("Normal : la carte telle quelle. Small et Tiny : la même carte en plus petit (texte, jauges et pastilles\nréduits ensemble)."),
];

/// Coins proposés pour la carte (« Emplacement »), dans l'ordre des valeurs.
pub const EMPLACEMENTS: [&str; 5] = ["Auto", "Haut gauche", "Haut droite", "Bas gauche", "Bas droite"];
/// Tailles proposées pour la carte (« Taille »), et leur échelle.
pub const TAILLES: [&str; 3] = ["Normal", "Small", "Tiny"];
const ECHELLES: [f64; 3] = [1.0, 0.85, 0.72];

/// 🖼 La carte : emplacement choisi (0 auto — comme avant : haut gauche sur grand écran, en bas sur un écran étroit —,
/// 1 haut gauche, 2 haut droite, 3 bas gauche, 4 bas droite) et échelle (1 ; 0,85 ; 0,72).
pub fn carte(vals: &[f64]) -> (usize, f64) {
    let p = Inputs { params: &PARAMS, vals };
    (p.pick("place").min(4), ECHELLES[p.pick("taille").min(2)])
}

/// Nom de l'unité du biais choisie (« 4h » ; vide : pas de biais).
pub fn nom_biais(vals: &[f64]) -> &'static str {
    let p = Inputs { params: &PARAMS, vals };
    ["", "1h", "4h", "1D"][p.pick("htf").min(3)]
}

/// Les réglages de Lens lus dans les valeurs de la fenêtre (une valeur absente prend son défaut).
pub fn config(vals: &[f64]) -> Config {
    let p = Inputs { params: &PARAMS, vals };
    let w = |k: &str| p.get(k).round().clamp(0.0, 100.0) as u8;
    Config {
        weights: Weights {
            trend: w("trend"),
            momentum: w("momentum"),
            strength: w("strength"),
            volume: w("volume"),
            structure: w("structure"),
        },
        htf: BIAIS[p.pick("htf").min(BIAIS.len() - 1)],
        bar_close: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defauts_et_lecture() {
        assert_eq!(config(&argos_script::inputs::defaults(&PARAMS)), Config::default());
        let c = config(&[40.0, 10.0, 10.0, 20.0, 20.0, 0.0]);
        assert_eq!((c.weights.trend, c.weights.momentum, c.htf), (40, 10, None));
        assert_eq!(config(&[250.0, -3.0]).weights.trend, 100, "borné à 0 … 100");
        assert_eq!(nom_biais(&argos_script::inputs::defaults(&PARAMS)), "4h");
        assert_eq!(nom_biais(&[20.0, 20.0, 20.0, 20.0, 20.0, 0.0]), "");
    }
}
