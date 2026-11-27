//! 💥 Argos Carte des liquidations (indicateur) : ESTIMATION d'Argos (modèle `argos_core::liqmap` sur l'intérêt ouvert
//! public de Binance, OKX et Hyperliquid). Bandes horizontales derrière les bougies, de leur naissance jusqu'à leur
//! liquidation (ou maintenant), plus vives pour les gros montants ; à droite, le cumul par prix des bandes encore là,
//! avec les liquidations cumulées des longs (sous le prix) et des shorts (au-dessus). Toucher ou survoler une bande :
//! son prix et son montant estimé.

use crate::shapes::{lines, Rect};
use crate::text::{Align, Rgb, Text};
use argos_core::liqmap::Band;
use argos_script::inputs::{check, choice, num, Inputs, Param};
use tiny_skia::Pixmap;

pub const GROUPS: [&str; 3] = ["💥 Carte des liquidations", "⚖️ Leviers (poids)", "🎨 Affichage"];
pub const PARAMS: [Param; 12] = [
    choice("venue", 0, "Bourse", 3, &["Binance", "OKX", "Hyperliquid", "Les trois"]),
    choice(
        "period",
        0,
        "Période",
        1,
        &["12 h", "24 h", "48 h", "3 jours", "1 semaine", "2 semaines", "1 mois"],
    ),
    choice("split", 0, "Longs / shorts", 0, &["50 / 50", "Selon la bougie"]),
    choice("bucket", 0, "Regroupement des prix", 0, &["Auto (1 ‰)", "0,5 ‰", "2 ‰", "5 ‰"]),
    num("w10", 1, "10x", 30.0, 0.0, 100.0, 5.0),
    num("w25", 1, "25x", 30.0, 0.0, 100.0, 5.0),
    num("w50", 1, "50x", 25.0, 0.0, 100.0, 5.0),
    num("w100", 1, "100x", 15.0, 0.0, 100.0, 5.0),
    num("thr", 2, "Seuil de liquidité", 0.5, 0.0, 0.99, 0.05),
    check("profile", 2, "Cumul par prix à droite", true),
    num("op", 2, "Opacité (%)", 80.0, 10.0, 100.0, 5.0),
    choice("pal", 2, "Palette", 0, &["Viridis", "Feu", "Glace"]),
];

/// Heures de chaque choix de période.
pub const HOURS: [i64; 7] = [12, 24, 48, 72, 168, 336, 720];

/// Ce que la chart dessine : les bandes estimées, le cumul par prix, et les messages (bourses en panne, historique en
/// construction).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LiqView {
    pub bands: Vec<Band>,
    pub profile: Vec<(f64, f64, f64)>,
    pub bucket: f64,
    pub notes: Vec<String>,
}

impl LiqView {
    /// Montant de la plus grosse bande (échelle des couleurs).
    pub fn max(&self) -> f64 {
        self.bands.iter().map(|b| b.usd).fold(0.0, f64::max)
    }

    /// Bande la plus grosse au prix `p` (à une demi-case près), parmi celles vivantes au temps `t`.
    pub fn at(&self, p: f64, t: i64) -> Option<&Band> {
        self.bands
            .iter()
            .filter(|b| (b.price - p).abs() <= self.bucket / 2.0 && b.born <= t && b.end.is_none_or(|e| e >= t))
            .max_by(|a, b| a.usd.total_cmp(&b.usd))
    }
}

/// Couleur d'une intensité 0..1 dans la palette choisie (0 viridis : violet → turquoise → jaune ; 1 feu ; 2 glace).
pub fn color(t: f64, pal: usize) -> Rgb {
    let stops: [(f64, f64, f64); 4] = match pal {
        1 => [(40.0, 6.0, 30.0), (150.0, 30.0, 50.0), (240.0, 120.0, 30.0), (255.0, 235.0, 120.0)],
        2 => [(15.0, 25.0, 70.0), (30.0, 100.0, 200.0), (90.0, 200.0, 240.0), (235.0, 250.0, 255.0)],
        _ => [(68.0, 1.0, 84.0), (49.0, 104.0, 142.0), (53.0, 183.0, 121.0), (253.0, 231.0, 37.0)],
    };
    let t = t.clamp(0.0, 1.0) * 3.0;
    let i = (t.floor() as usize).min(2);
    let f = t - i as f64;
    let m = |a: f64, b: f64| (a + (b - a) * f).round() as u8;
    Rgb(m(stops[i].0, stops[i + 1].0), m(stops[i].1, stops[i + 1].1), m(stops[i].2, stops[i + 1].2))
}

/// Montant court : « 12,3 M$ », « 950 k$ ».
pub fn usd(v: f64) -> String {
    if v >= 1e9 {
        format!("{:.2} G$", v / 1e9)
    } else if v >= 1e6 {
        format!("{:.1} M$", v / 1e6)
    } else {
        format!("{:.0} k$", v / 1e3)
    }
    .replace('.', ",")
}

/// Repère de la chart pour le dessin : x d'une heure (`None` hors des bougies), y d'un prix, largeur et hauteur du tracé.
pub struct Frame<'a> {
    pub x_of_time: &'a dyn Fn(i64) -> Option<f64>,
    pub y_of: &'a dyn Fn(f64) -> f64,
    pub pw: f64,
    pub ph: f64,
    /// Heure de la dernière bougie (les bandes encore là vont jusqu'au bord droit).
    pub now_x: f64,
}

/// Dessine la carte (avant les bougies).
pub fn draw(pm: &mut Pixmap, text: &Text, f: &Frame, v: &LiqView, vals: &[f64]) {
    let p = Inputs { params: &PARAMS, vals };
    let top = v.max();
    if top <= 0.0 {
        for (k, n) in v.notes.iter().enumerate() {
            text.draw(pm, n, (8.0, (22.0 + 16.0 * k as f64) as f32), 11.5, crate::palette::GREY, Align::Left);
        }
        return;
    }
    let thr = p.get("thr").clamp(0.0, 0.99);
    let (pal, op) = (p.pick("pal"), (p.get("op") / 100.0).clamp(0.05, 1.0) as f32);
    let h = ((f.y_of)(0.0) - (f.y_of)(v.bucket)).abs().clamp(1.0, 6.0);
    // plus le seuil est haut, plus les bandes moyennes deviennent vives ; les toutes petites ne sont pas dessinées
    let scale = top * (1.0 - thr).max(0.01);
    for b in &v.bands {
        if b.usd < top * 0.05 {
            continue;
        }
        let y = (f.y_of)(b.price);
        if !(0.0..f.ph).contains(&y) {
            continue;
        }
        let x0 = (f.x_of_time)(b.born).unwrap_or(0.0).max(0.0);
        let x1 = b.end.and_then(|e| (f.x_of_time)(e)).unwrap_or(f.now_x).min(f.pw);
        if x1 <= x0 {
            continue;
        }
        let t = (b.usd / scale).min(1.0);
        crate::fill(pm, x0, y - h / 2.0, x1 - x0, h, color(t, pal), op * (0.35 + 0.65 * t as f32));
    }
    // cumul par prix (bandes encore là) : barres + liquidations cumulées longs (rouge) et shorts (vert)
    if p.on("profile") && !v.profile.is_empty() {
        let w = 90.0f64.min(f.pw * 0.2);
        let x = f.pw - w;
        crate::fill(pm, x, 0.0, w, f.ph, crate::palette::night(), 0.35);
        let pmax = v.profile.iter().map(|r| r.1 + r.2).fold(0.0, f64::max).max(1.0);
        for &(price, l, s) in &v.profile {
            let y = (f.y_of)(price);
            if (0.0..f.ph).contains(&y) {
                let t = ((l + s) / scale).min(1.0);
                crate::fill(pm, x, y - h / 2.0, w * (l + s) / pmax, h.max(1.0), color(t, pal), 0.9);
            }
        }
        // cumul : longs du prix vers le bas, shorts du prix vers le haut
        let total_l: f64 = v.profile.iter().map(|r| r.1).sum::<f64>().max(1.0);
        let total_s: f64 = v.profile.iter().map(|r| r.2).sum::<f64>().max(1.0);
        let mut acc = 0.0;
        let mut long_pts = vec![];
        for &(price, l, _) in v.profile.iter().rev() {
            acc += l;
            long_pts.push(((x + w * acc / total_l) as f32, (f.y_of)(price) as f32));
        }
        acc = 0.0;
        let mut short_pts = vec![];
        for &(price, _, s) in &v.profile {
            acc += s;
            short_pts.push(((x + w * acc / total_s) as f32, (f.y_of)(price) as f32));
        }
        lines(pm, 0.0, 0.0, &[&long_pts], crate::palette::down(), 1.2);
        lines(pm, 0.0, 0.0, &[&short_pts], crate::palette::up(), 1.2);
    }
    let mut y = 18.0;
    for n in std::iter::once(&"Estimation Argos".to_string()).chain(&v.notes) {
        text.draw(pm, n, (8.0, y as f32), 11.0, crate::palette::GREY, Align::Left);
        y += 15.0;
    }
}

/// Bulle d'une bande sous la souris / le doigt : prix, côté et montant estimé.
pub fn tooltip(pm: &mut Pixmap, text: &Text, b: &Band, mx: f64, my: f64, pw: f64) {
    let s = format!(
        "{} · {} liquidés · ≈ {}",
        crate::viewport::format_price(b.price, 0),
        if b.long { "longs" } else { "shorts" },
        usd(b.usd)
    );
    let w = text.width(&s, 12.0) as f64 + 16.0;
    let x = (mx + 14.0).min(pw - w - 4.0);
    let r = Rect::new(x, my - 30.0, w, 22.0);
    crate::shapes::round_fill(pm, r, 5.0, crate::palette::popup(), 0.95);
    text.draw(pm, &s, ((r.x + 8.0) as f32, (r.cy() + 4.0) as f32), 12.0, crate::palette::WHITE, Align::Left);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bande_sous_le_doigt_et_montants() {
        let b = |price: f64, born: i64, end: Option<i64>, usd: f64| Band {
            price,
            long: true,
            born,
            end,
            usd,
        };
        let v = LiqView {
            bands: vec![b(78_800.0, 0, None, 1e6), b(78_800.0, 0, Some(100), 5e6), b(77_200.0, 50, None, 2e6)],
            profile: vec![],
            bucket: 100.0,
            notes: vec![],
        };
        assert_eq!(v.max(), 5e6);
        assert_eq!(v.at(78_830.0, 50).map(|x| x.usd), Some(5e6), "la plus grosse vivante à ce moment");
        assert_eq!(v.at(78_830.0, 200).map(|x| x.usd), Some(1e6), "l'autre est liquidée");
        assert!(v.at(77_200.0, 10).is_none(), "pas encore née");
        assert_eq!(usd(12_345_678.0), "12,3 M$");
        assert_eq!(usd(950_000.0), "950 k$");
        assert_ne!(color(0.0, 0), color(1.0, 0));
    }
}
