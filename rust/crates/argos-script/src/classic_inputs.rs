//! ⚙️ Réglages des indicateurs classiques (demande de Jee, 9 nov. 2026 : chacun des 43 indicateurs a son menu de
//! réglages qui marche). Les défauts sont les réglages usuels des manuels, ceux qu'Argos utilisait en dur : rien ne
//! change tant qu'on ne touche à rien. Les réglages ajoutés plus tard vont **à la fin** de leur liste.

use crate::inputs::{check, choice, color, num, Param};

/// Types de ligne d'une sortie (onglet Style) : l'index est gardé dans les réglages.
pub const TRAITS: [&str; 3] = ["Plein", "Tirets", "Points"];

/// Épaisseur d'une sortie (1 à 4), onglet Style ; pas montrée dans la légende.
const fn epaisseur(key: &'static str, label: &'static str, def: f64) -> Param {
    Param {
        display: false,
        ..num(key, 1, label, def, 1.0, 4.0, 0.5)
    }
}

/// Type de ligne d'une sortie (plein, tirets, points), onglet Style ; pas montré dans la légende.
const fn type_ligne(key: &'static str, label: &'static str) -> Param {
    Param {
        display: false,
        ..choice(key, 1, label, 0, &TRAITS)
    }
}

pub const LEN_GROUPS: [&str; 2] = ["📏 Réglages", "🎨 Style"];

pub const RSI_PARAMS: [Param; 6] = [
    num("len", 0, "Longueur", 14.0, 2.0, 200.0, 1.0),
    num("haut", 0, "Niveau haut", 70.0, 50.0, 100.0, 1.0),
    num("bas", 0, "Niveau bas", 30.0, 0.0, 50.0, 1.0),
    color("col", 1, "Couleur", 0x7e57c2),
    epaisseur("ep1", "Épaisseur · RSI", 2.0),
    type_ligne("ty1", "Type de ligne · RSI"),
];

pub const MACD_PARAMS: [Param; 9] = [
    num("fast", 0, "EMA rapide", 12.0, 1.0, 200.0, 1.0),
    num("slow", 0, "EMA lente", 26.0, 2.0, 400.0, 1.0),
    num("signal", 0, "Signal", 9.0, 1.0, 100.0, 1.0),
    color("col", 1, "Couleur MACD", 0x2962ff),
    color("colSig", 1, "Couleur signal", 0xffffff),
    epaisseur("ep1", "Épaisseur · MACD", 2.0),
    type_ligne("ty1", "Type de ligne · MACD"),
    epaisseur("ep2", "Épaisseur · Signal", 2.0),
    type_ligne("ty2", "Type de ligne · Signal"),
];

pub const EMA_CROSS_PARAMS: [Param; 8] = [
    num("fast", 0, "EMA rapide", 9.0, 1.0, 400.0, 1.0),
    num("slow", 0, "EMA lente", 21.0, 2.0, 800.0, 1.0),
    color("col", 1, "Couleur rapide", 0xffd700),
    color("colSlow", 1, "Couleur lente", 0x2962ff),
    epaisseur("ep1", "Épaisseur · EMA rapide", 2.0),
    type_ligne("ty1", "Type de ligne · EMA rapide"),
    epaisseur("ep2", "Épaisseur · EMA lente", 2.0),
    type_ligne("ty2", "Type de ligne · EMA lente"),
];

pub const STOCH_PARAMS: [Param; 8] = [
    num("len", 0, "Longueur %K", 14.0, 1.0, 200.0, 1.0),
    // 14, 1, 3 par défaut, comme le Stochastique de TradingView (choix de Jee, 26 nov. 2026 ; avant : 14, 3, 3)
    num("smooth", 0, "Lissage %K", 1.0, 1.0, 50.0, 1.0),
    num("d", 0, "Lissage %D", 3.0, 1.0, 50.0, 1.0),
    color("col", 1, "Couleur %K", 0x2962ff),
    epaisseur("ep1", "Épaisseur · %K", 2.0),
    type_ligne("ty1", "Type de ligne · %K"),
    epaisseur("ep2", "Épaisseur · %D", 1.0),
    type_ligne("ty2", "Type de ligne · %D"),
];

pub const ATR_PARAMS: [Param; 4] = [
    num("len", 0, "Longueur", 14.0, 1.0, 200.0, 1.0),
    color("col", 1, "Couleur", 0xf06292),
    epaisseur("ep1", "Épaisseur · ATR", 2.0),
    type_ligne("ty1", "Type de ligne · ATR"),
];

pub const VWAP_PARAMS: [Param; 4] = [
    check("hl3", 0, "Prix typique (haut + bas + clôture) / 3 ; décoché : la clôture", true),
    color("col", 1, "Couleur", 0x00bcd4),
    epaisseur("ep1", "Épaisseur · VWAP", 2.0),
    type_ligne("ty1", "Type de ligne · VWAP"),
];

pub const REL_VOLUME_PARAMS: [Param; 3] = [
    num("len", 0, "Moyenne du volume", 20.0, 2.0, 400.0, 1.0),
    num("pic", 0, "Pic de volume (× la moyenne)", 3.0, 1.0, 20.0, 0.1),
    color("col", 1, "Couleur", 0x2196f3),
];

pub const PIVOT_VWAP_PARAMS: [Param; 1] = [num("span", 0, "Bougies de chaque côté d'un pivot", 6.0, 1.0, 50.0, 1.0)];

pub const PIVOT_VWAP_MULTI_PARAMS: [Param; 5] = [
    num("s1", 0, "Échelle 1", 4.0, 1.0, 50.0, 1.0),
    num("s2", 0, "Échelle 2", 9.0, 1.0, 50.0, 1.0),
    num("s3", 0, "Échelle 3", 16.0, 1.0, 50.0, 1.0),
    num("votes", 0, "Échelles qui doivent voir le pivot", 2.0, 1.0, 3.0, 1.0),
    check("cut", 0, "Arrêter le tronçon quand une clôture le casse", true),
];

pub const STOCH_RSI_PARAMS: [Param; 9] = [
    num("rsi", 0, "Longueur RSI", 14.0, 2.0, 200.0, 1.0),
    num("len", 0, "Longueur stochastique", 14.0, 2.0, 200.0, 1.0),
    num("k", 0, "Lissage %K", 3.0, 1.0, 50.0, 1.0),
    num("d", 0, "Lissage %D", 3.0, 1.0, 50.0, 1.0),
    color("col", 1, "Couleur %K", 0x2962ff),
    epaisseur("ep1", "Épaisseur · %K", 2.0),
    type_ligne("ty1", "Type de ligne · %K"),
    epaisseur("ep2", "Épaisseur · %D", 1.0),
    type_ligne("ty2", "Type de ligne · %D"),
];

pub const ICHIMOKU_PARAMS: [Param; 15] = [
    num("tenkan", 0, "Tenkan-sen", 9.0, 1.0, 200.0, 1.0),
    num("kijun", 0, "Kijun-sen", 26.0, 1.0, 400.0, 1.0),
    num("senkou", 0, "Senkou span B", 52.0, 1.0, 800.0, 1.0),
    num("shift", 0, "Décalage", 26.0, 0.0, 400.0, 1.0),
    color("col", 1, "Couleur tenkan", 0x2962ff),
    epaisseur("ep1", "Épaisseur · Tenkan", 1.0),
    type_ligne("ty1", "Type de ligne · Tenkan"),
    epaisseur("ep2", "Épaisseur · Kijun", 1.0),
    type_ligne("ty2", "Type de ligne · Kijun"),
    epaisseur("ep3", "Épaisseur · Senkou A", 1.0),
    type_ligne("ty3", "Type de ligne · Senkou A"),
    epaisseur("ep4", "Épaisseur · Senkou B", 1.0),
    type_ligne("ty4", "Type de ligne · Senkou B"),
    epaisseur("ep5", "Épaisseur · Chikou", 1.0),
    type_ligne("ty5", "Type de ligne · Chikou"),
];

pub const KELTNER_PARAMS: [Param; 10] = [
    num("len", 0, "EMA", 20.0, 2.0, 400.0, 1.0),
    num("atr", 0, "ATR", 10.0, 1.0, 200.0, 1.0),
    num("mult", 0, "Multiplicateur", 2.0, 0.1, 10.0, 0.1),
    color("col", 1, "Couleur des bandes", 0x2962ff),
    epaisseur("ep1", "Épaisseur · Bande haute", 1.0),
    type_ligne("ty1", "Type de ligne · Bande haute"),
    epaisseur("ep2", "Épaisseur · Milieu", 1.0),
    type_ligne("ty2", "Type de ligne · Milieu"),
    epaisseur("ep3", "Épaisseur · Bande basse", 1.0),
    type_ligne("ty3", "Type de ligne · Bande basse"),
];

pub const DONCHIAN_PARAMS: [Param; 8] = [
    num("len", 0, "Longueur", 20.0, 2.0, 400.0, 1.0),
    color("col", 1, "Couleur des bandes", 0x2962ff),
    epaisseur("ep1", "Épaisseur · Bande haute", 1.0),
    type_ligne("ty1", "Type de ligne · Bande haute"),
    epaisseur("ep2", "Épaisseur · Milieu", 1.0),
    type_ligne("ty2", "Type de ligne · Milieu"),
    epaisseur("ep3", "Épaisseur · Bande basse", 1.0),
    type_ligne("ty3", "Type de ligne · Bande basse"),
];

pub const PSAR_PARAMS: [Param; 6] = [
    num("start", 0, "Départ", 0.02, 0.001, 1.0, 0.001),
    num("step", 0, "Pas", 0.02, 0.001, 1.0, 0.001),
    num("max", 0, "Maximum", 0.2, 0.01, 1.0, 0.01),
    color("col", 1, "Couleur", 0xffffff),
    epaisseur("ep1", "Épaisseur · SAR", 1.0),
    type_ligne("ty1", "Type de ligne · SAR"),
];

pub const DMI_PARAMS: [Param; 10] = [
    num("len", 0, "Longueur DI", 14.0, 1.0, 200.0, 1.0),
    num("adx", 0, "Lissage ADX", 14.0, 1.0, 200.0, 1.0),
    num("seuil", 0, "Seuil de tendance", 25.0, 0.0, 100.0, 1.0),
    color("col", 1, "Couleur ADX", 0xff9800),
    epaisseur("ep1", "Épaisseur · +DI", 1.5),
    type_ligne("ty1", "Type de ligne · +DI"),
    epaisseur("ep2", "Épaisseur · −DI", 1.5),
    type_ligne("ty2", "Type de ligne · −DI"),
    epaisseur("ep3", "Épaisseur · ADX", 2.0),
    type_ligne("ty3", "Type de ligne · ADX"),
];

pub const CCI_PARAMS: [Param; 4] = [
    num("len", 0, "Longueur", 20.0, 2.0, 400.0, 1.0),
    color("col", 1, "Couleur", 0x7e57c2),
    epaisseur("ep1", "Épaisseur · CCI", 2.0),
    type_ligne("ty1", "Type de ligne · CCI"),
];

pub const WILLIAMS_PARAMS: [Param; 4] = [
    num("len", 0, "Longueur", 14.0, 2.0, 400.0, 1.0),
    color("col", 1, "Couleur", 0x7e57c2),
    epaisseur("ep1", "Épaisseur · %R", 2.0),
    type_ligne("ty1", "Type de ligne · %R"),
];

pub const OBV_PARAMS: [Param; 6] = [
    num("sig", 0, "Moyenne de l'OBV (0 = aucune)", 0.0, 0.0, 400.0, 1.0),
    color("col", 1, "Couleur", 0x2196f3),
    epaisseur("ep1", "Épaisseur · OBV", 2.0),
    type_ligne("ty1", "Type de ligne · OBV"),
    epaisseur("ep2", "Épaisseur · Moyenne", 1.0),
    type_ligne("ty2", "Type de ligne · Moyenne"),
];

pub const SUPERTREND_PARAMS: [Param; 6] = [
    num("atr", 0, "ATR", 10.0, 1.0, 200.0, 1.0),
    num("mult", 0, "Multiplicateur", 3.0, 0.1, 20.0, 0.1),
    epaisseur("ep1", "Épaisseur · Haussier", 2.0),
    type_ligne("ty1", "Type de ligne · Haussier"),
    epaisseur("ep2", "Épaisseur · Baissier", 2.0),
    type_ligne("ty2", "Type de ligne · Baissier"),
];

pub const AROON_PARAMS: [Param; 5] = [
    num("len", 0, "Longueur", 25.0, 2.0, 400.0, 1.0),
    epaisseur("ep1", "Épaisseur · Aroon haut", 2.0),
    type_ligne("ty1", "Type de ligne · Aroon haut"),
    epaisseur("ep2", "Épaisseur · Aroon bas", 2.0),
    type_ligne("ty2", "Type de ligne · Aroon bas"),
];

pub const MFI_PARAMS: [Param; 4] = [
    num("len", 0, "Longueur", 14.0, 2.0, 200.0, 1.0),
    color("col", 1, "Couleur", 0x26a69a),
    epaisseur("ep1", "Épaisseur · MFI", 2.0),
    type_ligne("ty1", "Type de ligne · MFI"),
];
