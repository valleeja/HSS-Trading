//! Géométrie de la chart : quelles bougies sont visibles, où elles tombent en pixels, et les graduations (prix et temps).

use argos_core::Bar;

/// Zoom : espace minimum / maximum entre deux bougies, en pixels (comme TradingView : très dézoomé à bougies énormes).
/// Très dézoomé, plusieurs bougies tombent dans un même pixel : elles sont regroupées par colonne au dessin.
pub const MIN_SPACING: f64 = 0.002;
pub const MAX_SPACING: f64 = 250.0;
/// ≫ F1j: share of the candles' width the view may go past the live position into the future.
pub const FUTUR_MAX: f64 = 0.25;

/// Fenêtre de la chart en pixels, et le zoom / défilement horizontal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
    /// Largeur d'une bougie (espace compris), en pixels : c'est le zoom.
    pub bar_spacing: f64,
    /// Espace vide à droite de la dernière bougie, en bougies (positif) ; le défilement vers le passé est négatif.
    pub right_offset: f64,
    /// Largeur de l'échelle de prix (à droite).
    pub price_axis: u32,
    /// Hauteur de l'échelle de temps (en bas).
    pub time_axis: u32,
    /// Hauteur de la barre du bas, sous l'échelle de temps (périodes, « Aller à », horloge) ; 0 = pas de barre.
    pub footer: u32,
    /// Free band kept at the right of the plot, in pixels, on top of `right_offset` (Argos Murs: room for its pills
    /// and depth bars, so they never sit on the last candles); 0: none.
    pub marge: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            width: 1400,
            height: 800,
            bar_spacing: 7.0,
            right_offset: 5.0,
            price_axis: 78,
            time_axis: 28,
            footer: 0,
            marge: 0.0,
        }
    }
}

impl Viewport {
    /// Largeur de la zone des bougies.
    pub fn plot_w(&self) -> f64 {
        self.width.saturating_sub(self.price_axis) as f64
    }
    /// Hauteur de la zone des bougies.
    pub fn plot_h(&self) -> f64 {
        self.height.saturating_sub(self.time_axis + self.footer) as f64
    }
    /// Width where the candles go: the plot without the free band at its right (`marge`).
    pub fn bars_w(&self) -> f64 {
        (self.plot_w() - self.marge.max(0.0)).max(self.plot_w().min(40.0))
    }
    /// Centre horizontal de la bougie `i`.
    pub fn x_of(&self, i: usize, s: &Slots) -> f64 {
        self.bars_w() - (s.total() - s.center(i) + self.right_offset) * self.bar_spacing
    }
    /// Bougie sous l'abscisse `x`, `None` hors des bougies.
    pub fn index_at(&self, x: f64, s: &Slots) -> Option<usize> {
        if x < 0.0 || x > self.plot_w() {
            return None;
        }
        s.index_at(s.total() + self.right_offset - (self.bars_w() - x) / self.bar_spacing)
    }

    /// Affiche les bougies `i` à `j` sur toute la largeur (périodes 1D, 5D… de la barre du bas, plage de « Aller à »).
    /// Jusqu'à la dernière bougie : l'espace vide habituel à droite ; sinon une bougie de marge.
    pub fn show_range(&mut self, i: usize, j: usize, s: &Slots) {
        let Some(last) = s.len().checked_sub(1) else { return };
        let j = j.min(last);
        let i = i.min(j);
        let pad = if j + 1 == s.len() { Viewport::default().right_offset } else { 1.0 };
        let (start, end) = (s.center(i) - s.size(i) / 2.0, s.center(j) + s.size(j) / 2.0);
        self.bar_spacing = (self.bars_w() / (end - start + pad)).clamp(MIN_SPACING, MAX_SPACING);
        self.right_offset = pad - (s.total() - end);
    }
    /// Met la bougie `i` au milieu de la chart (« Aller à »).
    pub fn center_on(&mut self, i: usize, s: &Slots) {
        if i < s.len() {
            self.right_offset = self.bars_w() / (2.0 * self.bar_spacing) - s.total() + s.center(i);
        }
    }

    /// Zoom (molette / pincement) en gardant la bougie sous la souris au même endroit. `factor` > 1 = rapprocher.
    pub fn zoom_at(&mut self, factor: f64, x: f64, s: &Slots) {
        let sp = self.bar_spacing;
        // dézoomer : pas plus loin que tout l'historique chargé ; zoomer : jusqu'à MAX_SPACING (jamais l'inverse)
        let new_sp = if factor < 1.0 {
            (sp * factor).max(self.min_spacing(s)).min(sp)
        } else {
            (sp * factor).min(MAX_SPACING).max(sp)
        };
        // position « en espaces depuis la droite » du point sous la souris, conservée
        let anchor = (self.bars_w() - x) / sp - self.right_offset;
        self.bar_spacing = new_sp;
        self.right_offset = (self.bars_w() - x) / new_sp - anchor;
        self.clamp_offset(s);
    }

    /// Dézoom le plus loin permis : toutes les bougies chargées sur la largeur (le dézoom n'est limité que par
    /// l'historique ; plus d'historique se charge quand on arrive au début).
    pub fn min_spacing(&self, s: &Slots) -> f64 {
        (self.plot_w() / (s.total() + 10.0)).clamp(MIN_SPACING, 2.0)
    }

    /// Le début de l'historique chargé est-il à l'écran (ou presque) ? Il faut alors charger plus ancien.
    pub fn near_start(&self, s: &Slots) -> bool {
        self.visible(s).is_some_and(|(first, _)| first <= 20) || self.at_zoom_limit(s)
    }

    /// 🔭 Dézoomé au maximum de ce qui est chargé (le dézoom bute) : il faut aussi charger plus ancien, même si de
    /// l'espace vide à droite cache encore le début.
    pub fn at_zoom_limit(&self, s: &Slots) -> bool {
        !s.is_empty() && self.bar_spacing <= self.min_spacing(s) * 1.05 && self.min_spacing(s) > MIN_SPACING
    }

    /// ≫ F1j: the most the view may go into the future (`right_offset`, in candles): the live 5 candles plus a
    /// quarter of the candles' width.
    pub fn futur_max(&self) -> f64 {
        Viewport::default().right_offset + FUTUR_MAX * self.bars_w() / self.bar_spacing.max(f64::EPSILON)
    }

    /// ≫ F1j: the view is off live by more than 2 candles (future or past): the back-to-live button shows.
    pub fn hors_direct(&self) -> bool {
        (self.right_offset - Viewport::default().right_offset).abs() > 2.0
    }

    /// Défilement horizontal (glisser) : `dx` > 0 = tirer vers la droite, donc remonter dans le passé.
    pub fn scroll_px(&mut self, dx: f64, s: &Slots) {
        self.right_offset -= dx / self.bar_spacing;
        self.clamp_offset(s);
    }

    /// Garde toujours au moins quelques bougies à l'écran ; vers le futur (F1j, 22 nov. : l'iPad de Jee montrait 46 %
    /// de vide) pas plus loin que [`FUTUR_MAX`] : l'espace vide après la dernière bougie ne dépasse jamais 5 bougies +
    /// 25 % de la largeur des bougies (glisser et pincer pareil ; la marge d'Argos Murs s'ajoute en points).
    fn clamp_offset(&mut self, s: &Slots) {
        let max = (self.bars_w() / self.bar_spacing - 5.0).min(self.futur_max());
        let min = -s.total() + 5.0;
        self.right_offset = self.right_offset.clamp(min.min(max), max.max(min));
    }

    /// Bougies visibles (au moins en partie) : indices `first..=last`, `None` s'il n'y en a aucune.
    pub fn visible(&self, s: &Slots) -> Option<(usize, usize)> {
        let n = s.len();
        if n == 0 || self.bar_spacing <= 0.0 {
            return None;
        }
        // portion visible, en espaces depuis le début (une bougie de marge de chaque côté)
        let hi = s.total() + self.right_offset + 1.0;
        let lo = hi - self.bars_w() / self.bar_spacing - 2.0;
        let first = s.ends.partition_point(|&e| e <= lo);
        let count = if hi <= 0.0 { 0 } else { 1 + s.ends[..n - 1].partition_point(|&e| e < hi) };
        (first < count).then(|| (first, count - 1))
    }
}

/// Place de chaque bougie sur l'axe du temps, en espaces normaux. Bougies classiques : 1 chacune.
/// Bougies de volume : selon le volume (les grosses poussent leurs voisines au lieu de les chevaucher).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Slots {
    /// Fin de chaque bougie depuis le début de l'historique (somme des places).
    ends: Vec<f64>,
}

impl Slots {
    /// `n` bougies d'une place chacune.
    pub fn uniform(n: usize) -> Self {
        Self {
            ends: (1..=n).map(|k| k as f64).collect(),
        }
    }
    /// Une place par bougie (une place invalide ou ≤ 0 compte pour 1).
    pub fn weighted(places: impl IntoIterator<Item = f64>) -> Self {
        let mut t = 0.0;
        let ends = places
            .into_iter()
            .map(|w| {
                t += if w.is_finite() && w > 0.0 { w } else { 1.0 };
                t
            })
            .collect();
        Self { ends }
    }
    pub fn len(&self) -> usize {
        self.ends.len()
    }
    pub fn is_empty(&self) -> bool {
        self.ends.is_empty()
    }
    /// Longueur totale de l'historique, en espaces.
    pub fn total(&self) -> f64 {
        self.ends.last().copied().unwrap_or(0.0)
    }
    /// Fin de la bougie `i` ; au-delà de la table (bougies arrivées depuis le dernier calcul), une place par bougie
    /// après la dernière : jamais de plantage.
    fn end(&self, i: usize) -> f64 {
        match self.ends.get(i) {
            Some(e) => *e,
            None => self.total() + (i + 1 - self.ends.len()) as f64,
        }
    }
    fn start(&self, i: usize) -> f64 {
        if i == 0 {
            0.0
        } else {
            self.end(i - 1)
        }
    }
    /// Centre de la bougie `i`, en espaces depuis le début.
    pub fn center(&self, i: usize) -> f64 {
        (self.start(i) + self.end(i)) / 2.0
    }
    /// Place de la bougie `i`, en espaces.
    pub fn size(&self, i: usize) -> f64 {
        self.end(i) - self.start(i)
    }
    /// Bougie qui contient la position `u` (en espaces depuis le début).
    fn index_at(&self, u: f64) -> Option<usize> {
        let i = self.ends.partition_point(|&e| e <= u);
        (u >= 0.0 && i < self.len()).then_some(i)
    }
}

/// Échelle de prix verticale (ajustée aux bougies visibles, avec une marge en haut et en bas).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PriceScale {
    /// Bornes dans l'espace de l'échelle (le prix, ou son ln en logarithmique).
    pub min: f64,
    pub max: f64,
    pub top: f64,
    pub height: f64,
    pub log: bool,
    pub invert: bool,
}

impl PriceScale {
    /// Ajuste l'échelle au plus haut / plus bas des bougies `first..=last` (marge de 8 %).
    pub fn fit(bars: &[Bar], first: usize, last: usize, plot_h: f64) -> Self {
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for b in &bars[first..=last] {
            lo = lo.min(b.low);
            hi = hi.max(b.high);
        }
        if !(lo.is_finite() && hi.is_finite()) {
            (lo, hi) = (0.0, 1.0);
        }
        if hi - lo <= f64::EPSILON * hi.abs().max(1.0) {
            let pad = hi.abs().max(1.0) * 0.01;
            (lo, hi) = (lo - pad, hi + pad);
        }
        let m = (hi - lo) * 0.08;
        Self {
            min: lo - m,
            max: hi + m,
            top: 0.0,
            height: plot_h,
            log: false,
            invert: false,
        }
    }
    /// 🎫 📏 Zoom auto (maquette du 22 nov.) : l'échelle affichée agrandie juste assez pour que les `prix` (Limite, TP,
    /// SL du ticket) soient dans la partie vue de l'écran, `vu` = (haut, bas) en fractions de la hauteur depuis le
    /// haut ((0, 1) : toute la hauteur ; 📱 sous la légende et au-dessus de la feuille). Rien ne bouge quand ils y sont
    /// déjà (à 3 % du bord près) ; seul le côté dépassé s'agrandit, avec 8 % de marge. En logarithmique, en ln ;
    /// inversée, le haut de l'écran est le bas de l'échelle.
    pub fn garder_en_vue(self, prix: &[f64], vu: (f64, f64)) -> Self {
        let span = self.max - self.min;
        let v: Vec<f64> = prix.iter().filter(|p| **p > 0.0).map(|&p| self.t(p)).filter(|v| v.is_finite()).collect();
        if !(span > 0.0 && span.is_finite()) || v.is_empty() || !(vu.0.is_finite() && vu.1.is_finite()) {
            return self;
        }
        let a = vu.0.clamp(0.0, 0.8);
        let b = vu.1.clamp(a + 0.05, 1.0);
        let pad = 0.03 * span;
        // (la partie vue de l'échelle)
        let (vlo, vhi) = if self.invert {
            (self.min + a * span, self.min + b * span)
        } else {
            (self.max - b * span, self.max - a * span)
        };
        let (pmin, pmax) = v.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(x, y), &p| (x.min(p), y.max(p)));
        let (bas, haut) = (pmin < vlo + pad, pmax > vhi - pad);
        if !bas && !haut {
            return self;
        }
        let m = 0.08 * (vhi.max(pmax) - vlo.min(pmin));
        let lo = if bas { pmin - m } else { vlo };
        let hi = if haut { pmax + m } else { vhi };
        let full = (hi - lo) / (b - a);
        if self.invert {
            let min = lo - a * full;
            Self { min, max: min + full, ..self }
        } else {
            let max = hi + a * full;
            Self { min: max - full, max, ..self }
        }
    }
    /// Passe en logarithmique (les bornes deviennent des ln ; prix ≤ 0 ramenés à un minimum).
    pub fn with_log(self, log: bool) -> Self {
        if !log || self.log {
            return self;
        }
        let lo = self.min.max(self.max * 1e-6).max(1e-12);
        Self {
            min: lo.ln(),
            max: self.max.max(lo * 1.01).ln(),
            log: true,
            ..self
        }
    }
    fn t(&self, p: f64) -> f64 {
        if self.log {
            p.max(1e-12).ln()
        } else {
            p
        }
    }
    fn inv(&self, v: f64) -> f64 {
        if self.log {
            v.exp()
        } else {
            v
        }
    }
    /// Prix en bas et en haut de l'échelle.
    pub fn price_range(&self) -> (f64, f64) {
        (self.inv(self.min), self.inv(self.max))
    }
    pub fn y_of(&self, price: f64) -> f64 {
        let f = (self.max - self.t(price)) / (self.max - self.min);
        self.top + if self.invert { 1.0 - f } else { f } * self.height
    }
    pub fn price_of(&self, y: f64) -> f64 {
        let f = (y - self.top) / self.height;
        let f = if self.invert { 1.0 - f } else { f };
        self.inv(self.max - f * (self.max - self.min))
    }
    /// Graduations « rondes » (1, 2, 2,5 ou 5 × 10ⁿ) environ tous les `px` pixels.
    pub fn ticks(&self, px: f64) -> (f64, Vec<f64>) {
        let (lo, hi) = self.price_range();
        let step = nice_step((hi - lo) * px / self.height);
        let mut v = (lo / step).ceil() * step;
        let mut out = vec![];
        while v <= hi {
            out.push(v);
            v += step;
        }
        (step, out)
    }
}

/// Pas « rond » le plus proche au-dessus de `raw`.
pub fn nice_step(raw: f64) -> f64 {
    if raw <= 0.0 || !raw.is_finite() {
        return 1.0;
    }
    let mag = 10f64.powf(raw.log10().floor());
    let n = raw / mag;
    let k = if n <= 1.0 {
        1.0
    } else if n <= 2.0 {
        2.0
    } else if n <= 2.5 {
        2.5
    } else if n <= 5.0 {
        5.0
    } else {
        10.0
    };
    k * mag
}

/// Nombre de décimales à afficher pour un pas de prix.
pub fn decimals_for(step: f64) -> usize {
    let mut d = 0;
    while d < 10 && ((step * 10f64.powi(d as i32)).round() - step * 10f64.powi(d as i32)).abs() > 1e-9 {
        d += 1;
    }
    d
}

/// Précision réelle des prix d'un symbole : le plus de décimales vues dans les dernières bougies (8 max au-dessus de
/// 1 ; sous 1, tous les chiffres reçus, 12 décimales au plus).
pub fn data_decimals(bars: &[Bar]) -> usize {
    bars.iter()
        .rev()
        .take(200)
        .flat_map(|b| [b.open, b.high, b.low, b.close])
        .filter(|v| v.is_finite())
        .map(price_digits)
        .max()
        .unwrap_or(2)
        .max(small_price_decimals(bars.last().map_or(0.0, |b| b.close)))
        .min(12)
}

/// Décimales d'un prix reçu. Sous 1, celles que la bourse a écrites (« 0.0123456 » → 7, « 0.000016312 » → 9) tant
/// qu'il a au plus 8 chiffres utiles ; au-delà, c'est un prix calculé (paire calculée), et comme au-dessus de 1 : le
/// moins de décimales qui l'écrivent (8 max).
fn price_digits(v: f64) -> usize {
    if v != 0.0 && v.abs() < 1.0 {
        let s = format!("{}", v.abs());
        let frac = s.split_once('.').map_or("", |(_, f)| f);
        if frac.trim_start_matches('0').len() <= 8 {
            return frac.len();
        }
    }
    (0..=8)
        .find(|&d| ((v * 10f64.powi(d)).round() - v * 10f64.powi(d)).abs() < 1e-6 * 10f64.powi(d).max(1.0))
        .unwrap_or(8) as usize
}

/// Très petits prix (paire calculée « XRPBTC » ≈ 0,0000163) : assez de décimales pour 4 chiffres utiles (max 12).
fn small_price_decimals(p: f64) -> usize {
    let p = p.abs();
    if !(p > 0.0 && p < 1.0 && p.is_finite()) {
        return 0;
    }
    ((-p.log10()).floor() as usize + 4).min(12)
}

// 🌐 Numbers as on the screen (prices, amounts, the formats of 🌐 and the chosen one): `argos_core::nombres`, shared
// with Argos AI and the summary tables.
pub use argos_core::nombres::{format_money, format_number, format_price, number_format, set_number_format, NUMBER_FORMATS};

/// Volume court comme TradingView : « 14,85 M », « 12,3 K », « 845,20 ».
pub fn format_volume(v: f64) -> String {
    let a = v.abs();
    if a >= 1e9 {
        format!("{} B", format_price(v / 1e9, 2))
    } else if a >= 1e6 {
        format!("{} M", format_price(v / 1e6, 2))
    } else if a >= 1e3 {
        format!("{} K", format_price(v / 1e3, 2))
    } else {
        format_price(v, 2)
    }
}

/// Intervalle entre deux étiquettes de temps : le plus petit qui laisse au moins `min_px` pixels entre elles.
pub fn time_interval(tf_seconds: i64, bar_spacing: f64, min_px: f64) -> i64 {
    // au-delà d'un mois : des mois du calendrier (1, 3, 6), puis des années (1, 2, 5), voir `MONTH`
    const CANDIDATES: [i64; 20] = [
        60,
        300,
        900,
        1800,
        3600,
        7200,
        10_800,
        14_400,
        21_600,
        43_200,
        86_400,
        172_800,
        604_800,
        1_209_600,
        MONTH,
        3 * MONTH,
        6 * MONTH,
        12 * MONTH,
        24 * MONTH,
        60 * MONTH,
    ];
    let px_per_s = bar_spacing / tf_seconds.max(1) as f64;
    CANDIDATES
        .into_iter()
        .find(|&c| c >= tf_seconds && c as f64 * px_per_s >= min_px)
        .unwrap_or(120 * MONTH)
}

/// Un « mois » pour l'espacement des étiquettes de temps (30 jours) ; à partir de là, les étiquettes suivent les mois
/// du calendrier.
pub const MONTH: i64 = 2_592_000;

#[cfg(test)]
mod tests {
    use super::*;

    /// 🎫 📏 Zoom auto (mock-up of 22 Nov): lines already in view leave the scale as it is; a TP above widens only the
    /// top (8 % margin); on a phone, the lines go into the part above the sheet; upside down and in log too.
    #[test]
    fn zoom_auto_garde_les_lignes_du_ticket() {
        let s0 = PriceScale {
            min: 90.0,
            max: 110.0,
            top: 0.0,
            height: 500.0,
            log: false,
            invert: false,
        };
        assert_eq!(s0.garder_en_vue(&[95.0, 105.0], (0.0, 1.0)), s0, "already in view: nothing moves");
        assert_eq!(s0.garder_en_vue(&[], (0.0, 0.3)), s0);
        let s = s0.garder_en_vue(&[100.0, 130.0, 95.0], (0.0, 1.0));
        assert_eq!(s.min, 90.0, "the bottom stays");
        assert!((s.max - (130.0 + 0.08 * 40.0)).abs() < 1e-9, "{s:?}");
        let y = s.y_of(130.0);
        assert!(y > 10.0 && y < 490.0, "the TP under the top edge ({y})");
        // 📱 the sheet hides the bottom 70 %: the TP and the SL go into the top 30 %
        let s = s0.garder_en_vue(&[130.0, 80.0], (0.0, 0.3));
        for p in [130.0, 80.0] {
            let y = s.y_of(p);
            assert!(y > 0.0 && y < 0.3 * 500.0, "{p} above the sheet: {y}");
        }
        // 📱 and under the legend (the top 20 %)
        let s = s0.garder_en_vue(&[130.0, 80.0], (0.2, 0.5));
        for p in [130.0, 80.0] {
            let y = s.y_of(p);
            assert!(y > 0.2 * 500.0 && y < 0.5 * 500.0, "{p} between the legend and the sheet: {y}");
        }
        // upside down: the top of the screen is the bottom of the scale
        let inv = PriceScale { invert: true, ..s0 };
        let s = inv.garder_en_vue(&[130.0, 80.0], (0.2, 0.5));
        for p in [130.0, 80.0] {
            let y = s.y_of(p);
            assert!(y > 0.2 * 500.0 && y < 0.5 * 500.0, "{p} upside down: {y}");
        }
        // logarithmic: bounds in ln
        let log = PriceScale {
            min: 90f64.ln(),
            max: 110f64.ln(),
            log: true,
            ..s0
        };
        let s = log.garder_en_vue(&[130.0], (0.0, 1.0));
        let y = s.y_of(130.0);
        assert!(y > 10.0 && y < 490.0 && s.min == log.min, "log: {y}");
    }

    /// Argos Murs keeps a free band at the right: the last candle moves left by exactly that many pixels, and a click
    /// in the band is on no candle; without the band nothing moves.
    #[test]
    fn marge_a_droite_decale_les_bougies() {
        let s = Slots::uniform(100);
        let v = Viewport {
            width: 1078,
            bar_spacing: 10.0,
            right_offset: 0.0,
            ..Viewport::default()
        };
        let m = Viewport { marge: 270.0, ..v };
        assert_eq!(v.x_of(99, &s) - m.x_of(99, &s), 270.0);
        assert_eq!(m.index_at(m.x_of(50, &s), &s), Some(50));
        assert_eq!(m.index_at(m.plot_w() - 10.0, &s), None);
        // a phone-size plot keeps room for candles whatever the band
        let small = Viewport { width: 180, marge: 400.0, ..v };
        assert!(small.bars_w() >= 40.0);
    }

    #[test]
    fn position_et_visibles() {
        let v = Viewport {
            width: 178,
            height: 100,
            bar_spacing: 10.0,
            right_offset: 0.0,
            price_axis: 78,
            time_axis: 0,
            footer: 0,
            marge: 0.0,
        };
        // zone de 100 px, 10 px par bougie : la dernière est centrée à 95, la 10e en partant de la fin à 5
        let s = Slots::uniform(100);
        assert_eq!(v.x_of(99, &s), 95.0);
        assert_eq!(v.x_of(90, &s), 5.0);
        let (first, last) = v.visible(&s).unwrap();
        assert_eq!(last, 99);
        assert!((89..=90).contains(&first));
        assert_eq!(v.visible(&Slots::uniform(0)), None);
    }

    #[test]
    fn zoom_garde_la_bougie_sous_la_souris() {
        let mut v = Viewport {
            width: 1078,
            height: 600,
            bar_spacing: 8.0,
            // (in the past: the future limit of F1j never moves the view while zooming here)
            right_offset: -200.0,
            price_axis: 78,
            time_axis: 28,
            footer: 0,
            marge: 0.0,
        };
        let n = &Slots::uniform(1000);
        let x = 400.0;
        let before = v.index_at(x, n).unwrap();
        v.zoom_at(1.5, x, n);
        assert_eq!(v.bar_spacing, 12.0);
        assert_eq!(v.index_at(x, n), Some(before));
        v.zoom_at(0.25, x, n);
        assert_eq!(v.index_at(x, n), Some(before));
        // bornes du zoom
        v.zoom_at(1000.0, x, n);
        assert_eq!(v.bar_spacing, MAX_SPACING);
        // dézoom borné par l'historique chargé : toutes les bougies tiennent sur la largeur
        v.zoom_at(1e-9, x, n);
        assert_eq!(v.bar_spacing, 1000.0 / 1010.0);
        assert_eq!(v.bar_spacing, v.min_spacing(n));
        // 50 000 bougies : on dézoome jusqu'à plusieurs bougies par pixel
        let big = &Slots::uniform(50_000);
        v.zoom_at(1e-9, x, big);
        assert!(v.bar_spacing < 0.05 && v.bar_spacing >= MIN_SPACING);
        v.show_range(0, 49_999, big);
        assert!(v.near_start(big), "tout l'historique à l'écran : on est au début");
        v.show_range(40_000, 49_999, big);
        assert!(!v.near_start(big));
    }

    #[test]
    fn defilement_borne() {
        let mut v = Viewport {
            width: 1078,
            height: 600,
            bar_spacing: 10.0,
            right_offset: 5.0,
            price_axis: 78,
            time_axis: 28,
            footer: 0,
            marge: 0.0,
        };
        let s = &Slots::uniform(1000);
        v.scroll_px(100.0, s); // 10 bougies vers le passé
        assert_eq!(v.right_offset, -5.0);
        v.scroll_px(1e9, s); // on ne peut pas sortir de l'historique
        assert_eq!(v.right_offset, -995.0);
        v.scroll_px(-1e9, s); // ni partir trop loin dans le futur (F1j : 5 bougies + 25 % de 1 000 px / 10 px = 30)
        assert_eq!(v.right_offset, 30.0);
        // le pincement non plus : zoomer / dézoomer au bord droit ne passe jamais la limite
        for f in [0.5, 2.0, 0.8, 1.6] {
            v.zoom_at(f, v.bars_w(), s);
            assert!(v.right_offset <= v.futur_max() + 1e-9, "{f}: {}", v.right_offset);
        }
        assert!(v.hors_direct());
        v.right_offset = 6.0;
        assert!(!v.hors_direct());
        assert!(v.visible(s).is_some());
    }

    #[test]
    fn places_selon_le_volume() {
        let v = Viewport {
            width: 178,
            height: 100,
            bar_spacing: 10.0,
            right_offset: 0.0,
            price_axis: 78,
            time_axis: 0,
            footer: 0,
            marge: 0.0,
        };
        // 3 bougies : 1, 3 et 1 espaces → la grosse prend 30 px et pousse ses voisines
        let s = Slots::weighted([1.0, 3.0, 1.0]);
        assert_eq!(s.total(), 5.0);
        assert_eq!((v.x_of(2, &s), v.x_of(1, &s), v.x_of(0, &s)), (95.0, 75.0, 55.0));
        assert_eq!(s.size(1), 3.0);
        assert_eq!(v.index_at(62.0, &s), Some(1)); // bord gauche de la grosse
        assert_eq!(v.index_at(58.0, &s), Some(0));
        assert_eq!(v.index_at(10.0, &s), None);
        assert_eq!(v.visible(&s), Some((0, 2)));
        assert_eq!(Slots::weighted([f64::NAN, -2.0]).total(), 2.0);
        // bougie au-delà de la table (arrivée depuis le dernier calcul) : une place après la dernière, pas de plantage
        let s = Slots::weighted([1.0, 3.0]);
        assert_eq!((s.center(2), s.size(2), s.center(3)), (4.5, 1.0, 5.5));
    }

    #[test]
    fn echelle_de_prix() {
        let b = |l: f64, h: f64| Bar {
            time: 0,
            open: l,
            high: h,
            low: l,
            close: h,
            volume: 1.0,
        };
        let s = PriceScale::fit(&[b(100.0, 110.0), b(95.0, 105.0)], 0, 1, 500.0);
        assert!(s.min < 95.0 && s.max > 110.0);
        assert!((s.price_of(s.y_of(101.5)) - 101.5).abs() < 1e-9);
        // inversée : le haut en bas, aller-retour exact
        let inv = PriceScale { invert: true, ..s };
        assert!((inv.y_of(s.max) - s.height).abs() < 1e-9);
        assert!((inv.price_of(inv.y_of(101.5)) - 101.5).abs() < 1e-9);
        // logarithmique : 100 → 110 prend la même hauteur que 1000 → 1100
        let lg = PriceScale::fit(&[b(90.0, 1200.0)], 0, 0, 500.0).with_log(true);
        let h1 = lg.y_of(100.0) - lg.y_of(110.0);
        let h2 = lg.y_of(1000.0) - lg.y_of(1100.0);
        assert!((h1 - h2).abs() < 1e-9 && (lg.price_of(lg.y_of(321.0)) - 321.0).abs() < 1e-6);
        let (step, ticks) = s.ticks(60.0);
        assert!(ticks.iter().all(|t| (t / step).fract().abs() < 1e-9 || (t / step).fract() > 1.0 - 1e-9));
    }

    #[test]
    fn pas_ronds_et_format() {
        assert_eq!(nice_step(0.7), 1.0);
        assert_eq!(nice_step(1.3), 2.0);
        assert_eq!(nice_step(2.2), 2.5);
        assert_eq!(nice_step(380.0), 500.0);
        assert_eq!(decimals_for(500.0), 0);
        assert_eq!(decimals_for(0.25), 2);
        assert_eq!(format_price(80660.0, 1), "80\u{202f}660,0");
        assert_eq!(format_price(1.2982, 4), "1,2982");
        assert_eq!(format_volume(14_851_077.9), "14,85 M");
        assert_eq!(format_volume(12_345.0), "12,35 K");
        assert_eq!(format_volume(845.2), "845,20");
        assert_eq!(time_interval(1800, 7.0, 80.0), 21_600);
        let b = |c: f64| Bar {
            time: 0,
            open: c,
            high: c,
            low: c,
            close: c,
            volume: 0.0,
        };
        assert_eq!(data_decimals(&[b(1.2982), b(1.3)]), 4);
        assert_eq!(data_decimals(&[b(80660.0), b(80661.0)]), 0);
        assert_eq!(data_decimals(&[b(76.862)]), 3); // 12 bougies × 7 px = 84 px
        assert_eq!(data_decimals(&[b(0.0000163)]), 8, "paire calculée : 0,00001630");
        assert_eq!(data_decimals(&[b(0.000000000123)]), 12, "12 décimales au plus");
        // tous les chiffres reçus sous 1 : XRPBTC chez Bitfinex (5 chiffres utiles), un prix à 7 décimales
        assert_eq!(data_decimals(&[b(0.000016312), b(0.000016409)]), 9, "XRPBTC Bitfinex : 0,000016312");
        assert_eq!(data_decimals(&[b(0.0123456)]), 7, "0,0123456");
        assert_eq!(format_price(0.000016312, data_decimals(&[b(0.000016312)])), "0,000016312");
        // une paire calculée (division) garde l'ancienne règle : pas de chiffres inventés par le calcul
        assert_eq!(data_decimals(&[b(0.5 / 30_652.7)]), 8);
    }
}
