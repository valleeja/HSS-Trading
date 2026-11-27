//! Bougies de volume — Argos TVD Pro.
//! Chaque bougie est classée selon son volume comparé à la moyenne des N bougies D'AVANT (la bougie elle-même n'est pas dans la moyenne) :
//! GOLD ≥ 300 %, HIGH ≥ 200 %, MID ≥ 150 %, LOW < 50 % (réglables).
//! Largeur (hybride) : formule de TradingView (`1 + log2(ratio)`, minimum à 0,9 × la moyenne), mais comparée aux N bougies d'avant
//! au lieu de tout l'historique chargé. Chaque bougie prend une place à sa largeur : les grosses poussent leurs voisines.
//! Couleurs Argos : GOLD or / mauve foncé, HIGH vert / rouge, MID bleu / rose, LOW vide avec contour vert / rouge.
//! GOLD / HIGH / MID : mêmes seuils et même moyenne que le site et que l'indicateur TVD Pro sur TradingView. LOW est propre à Argos.

use argos_core::Bar;

/// Palier de volume d'une bougie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// Volume faible (sous `low_pct` de la moyenne) : bougie vide, seulement le contour.
    Low,
    Normal,
    Mid,
    High,
    Gold,
}

/// Réglages (« ⚙️ Réglages des bougies de volume »).
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub enabled: bool,
    /// Seuils en % de la moyenne (`low_pct` = 0 : pas de palier LOW).
    pub low_pct: f64,
    pub mid_pct: f64,
    pub high_pct: f64,
    pub gold_pct: f64,
    /// Nombre de bougies d'avant pour la moyenne.
    pub avg_len: usize,
    pub bull_gold: String,
    pub bull_high: String,
    pub bull_mid: String,
    pub bear_gold: String,
    pub bear_high: String,
    pub bear_mid: String,
    /// Couleurs des bougies sans palier.
    pub bull_normal: String,
    pub bear_normal: String,
    /// Couleurs du contour des bougies LOW (corps vide).
    pub bull_low: String,
    pub bear_low: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: true,
            low_pct: 50.0,
            mid_pct: 150.0,
            high_pct: 200.0,
            gold_pct: 300.0,
            avg_len: 20,
            bull_gold: "#ffd600".into(),
            bull_high: "#00c853".into(),
            bull_mid: "#2196f3".into(),
            bear_gold: "#6a1b9a".into(),
            bear_high: "#ff1744".into(),
            bear_mid: "#f06292".into(),
            bull_normal: "#089981".into(),
            bear_normal: "#f23645".into(),
            bull_low: "#089981".into(),
            bear_low: "#f23645".into(),
        }
    }
}

/// Résultat pour une bougie.
#[derive(Debug, Clone, PartialEq)]
pub struct VolumeCandle {
    pub tier: Tier,
    pub bull: bool,
    /// Volume ÷ moyenne des N bougies d'avant (0 si pas encore assez d'historique).
    pub ratio: f64,
    /// Couleur du corps.
    pub color: String,
    /// Opacité : 1 avec un palier MID / HIGH / GOLD, 0,8 sans (20 % de transparence), 0,9 pour le contour LOW.
    pub opacity: f64,
    /// Corps vide (seulement le contour) : les bougies LOW.
    pub hollow: bool,
    /// Place de la bougie sur la chart, en espaces normaux : 1 au volume moyen, 2 à 2 ×, 3 à 4 ×, ~0,85 au minimum.
    pub width: f64,
}

/// Place selon le ratio (formule de TradingView) : `1 + log2(ratio)`, ratio bloqué à 0,9 minimum ; 1 sans historique.
pub fn width_for_ratio(ratio: f64) -> f64 {
    if ratio > 0.0 {
        1.0 + ratio.max(0.9).log2()
    } else {
        1.0
    }
}

pub fn compute(bars: &[Bar], s: &Settings) -> Vec<VolumeCandle> {
    let len = s.avg_len.max(1);
    let mut out = Vec::with_capacity(bars.len());
    let mut sum = 0.0;
    for (i, b) in bars.iter().enumerate() {
        // somme glissante des `len` volumes d'avant
        if i > len {
            sum -= bars[i - len - 1].volume;
        }
        if i > 0 {
            sum += bars[i - 1].volume;
        }
        let have = i.min(len);
        let avg = if have > 0 { sum / have as f64 } else { 0.0 };
        let ratio = if avg > 0.0 { b.volume / avg } else { 0.0 };
        let full = i >= len;
        let tier = if !s.enabled || !full {
            Tier::Normal
        } else if ratio >= s.gold_pct / 100.0 {
            Tier::Gold
        } else if ratio >= s.high_pct / 100.0 {
            Tier::High
        } else if ratio >= s.mid_pct / 100.0 {
            Tier::Mid
        } else if ratio < s.low_pct / 100.0 {
            Tier::Low
        } else {
            Tier::Normal
        };
        let bull = b.is_bull();
        let color = match (tier, bull) {
            (Tier::Gold, true) => &s.bull_gold,
            (Tier::High, true) => &s.bull_high,
            (Tier::Mid, true) => &s.bull_mid,
            (Tier::Normal, true) => &s.bull_normal,
            (Tier::Low, true) => &s.bull_low,
            (Tier::Gold, false) => &s.bear_gold,
            (Tier::High, false) => &s.bear_high,
            (Tier::Mid, false) => &s.bear_mid,
            (Tier::Normal, false) => &s.bear_normal,
            (Tier::Low, false) => &s.bear_low,
        }
        .clone();
        out.push(VolumeCandle {
            tier,
            bull,
            ratio,
            color,
            opacity: match tier {
                Tier::Low => 0.9,
                Tier::Normal => 0.8,
                _ => 1.0,
            },
            width: width_for_ratio(ratio),
            hollow: tier == Tier::Low,
        });
    }
    out
}

/// Qui a dominé la bougie, estimé selon la clôture dans le range : (acheteurs %, vendeurs %).
pub fn buyers_sellers(b: &Bar) -> (u8, u8) {
    let r = b.range();
    let buy = if r > 0.0 { ((b.close - b.low) / r * 100.0).round() } else { 50.0 } as u8;
    (buy, 100 - buy)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(i: i64, vol: f64, bull: bool) -> Bar {
        let (o, c) = if bull { (100.0, 101.0) } else { (101.0, 100.0) };
        Bar {
            time: i * 60,
            open: o,
            high: 102.0,
            low: 99.0,
            close: c,
            volume: vol,
        }
    }

    #[test]
    fn paliers_apres_la_fenetre() {
        let mut bars: Vec<Bar> = (0..20).map(|i| bar(i, 10.0, true)).collect();
        bars.push(bar(20, 15.0, true)); // 150 % → MID
        bars.push(bar(21, 30.0, false)); // moyenne des 20 d'avant = 10,25 → 293 % → HIGH
        bars.push(bar(22, 100.0, true)); // GOLD
        let v = compute(&bars, &Settings::default());
        assert!(
            v[..20].iter().all(|x| x.tier == Tier::Normal),
            "pas de palier tant que la fenêtre n'est pas pleine"
        );
        assert_eq!(v[20].tier, Tier::Mid);
        assert_eq!(v[20].color, "#2196f3");
        assert_eq!(v[21].tier, Tier::High);
        assert_eq!(v[21].color, "#ff1744");
        assert_eq!(v[22].tier, Tier::Gold);
        assert_eq!(v[22].color, "#ffd600");
        assert_eq!(v[0].opacity, 0.8);
        assert_eq!(v[22].opacity, 1.0);
    }

    #[test]
    fn palier_low_vide_avec_contour() {
        let mut bars: Vec<Bar> = (0..20).map(|i| bar(i, 10.0, true)).collect();
        bars.push(bar(20, 4.0, false)); // 40 % de la moyenne → LOW
        bars.push(bar(21, 6.0, true)); // ~ 59 % → normal
        let v = compute(&bars, &Settings::default());
        assert_eq!(v[20].tier, Tier::Low);
        assert_eq!((v[20].hollow, v[20].color.as_str()), (true, "#f23645"));
        assert!(!v[21].hollow);
        assert_eq!(v[20].width, width_for_ratio(0.5), "LOW = place minimale");
        assert_eq!(v[21].tier, Tier::Normal);
        // pas de LOW avant que la fenêtre soit pleine, ni si le seuil est à 0
        assert!(v[..20].iter().all(|x| x.tier == Tier::Normal));
        assert_eq!(
            compute(
                &bars,
                &Settings {
                    low_pct: 0.0,
                    ..Settings::default()
                }
            )[20]
                .tier,
            Tier::Normal
        );
    }

    #[test]
    fn reglages_personnalises_et_desactive() {
        let mut bars: Vec<Bar> = (0..5).map(|i| bar(i, 10.0, true)).collect();
        bars.push(bar(5, 12.0, false));
        let s = Settings {
            avg_len: 5,
            mid_pct: 110.0,
            ..Settings::default()
        };
        assert_eq!(compute(&bars, &s)[5].tier, Tier::Mid);
        assert_eq!(compute(&bars, &Settings { enabled: false, ..s })[5].tier, Tier::Normal);
    }

    #[test]
    fn largeur_et_dominance() {
        assert_eq!(width_for_ratio(1.0), 1.0);
        assert_eq!(width_for_ratio(2.0), 2.0);
        assert_eq!(width_for_ratio(8.0), 4.0);
        assert!((width_for_ratio(0.5) - (1.0 + 0.9f64.log2())).abs() < 1e-12); // minimum ≈ 0,848
        assert_eq!(width_for_ratio(0.0), 1.0); // pas encore d'historique
        let b = Bar {
            time: 0,
            open: 100.0,
            high: 110.0,
            low: 100.0,
            close: 102.5,
            volume: 1.0,
        };
        assert_eq!(buyers_sellers(&b), (25, 75));
    }
}
