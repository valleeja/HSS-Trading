//! 🕯️ Types de bougies (classiques, barres, Heikin Ashi, Renko, volume Argos…), leurs créneaux et la série affichée.

use crate::*;

/// Style des bougies.
#[derive(Debug, Clone, PartialEq)]
pub enum CandleStyle {
    /// Bougies japonaises classiques (vert / rouge).
    Classic,
    /// Barres OHLC (comme TradingView) : trait du haut au bas, petit trait à gauche = ouverture, à droite = clôture.
    Bars,
    /// Bougies creuses (règle TradingView) : vide si clôture > ouverture, pleine sinon ;
    /// vert si clôture ≥ clôture précédente, rouge sinon.
    Hollow,
    /// Heikin Ashi (calcul TradingView) : bougies lissées, vert si clôture HA ≥ ouverture HA. Échelle, réticule et dernier prix en valeurs HA.
    HeikinAshi {
        /// Ligne du dernier prix au vrai prix du marché (pas la clôture HA lissée).
        real_last_price: bool,
    },
    /// Ligne (comme TradingView) : les clôtures reliées, bleu #2962FF, 2 px.
    Line,
    /// Aire : la ligne + un dégradé bleu dessous.
    Area,
    /// Ligne de base : niveau au milieu de l'échelle ; au-dessus vert, en dessous rouge, avec dégradés.
    Baseline,
    /// Ligne avec marqueurs : la ligne + un point sur chaque clôture (quand il y a la place).
    LineMarkers,
    /// Ligne en escalier : la clôture reste plate jusqu'à la bougie suivante.
    StepLine,
    /// Aire HLC : lignes du haut (vert), du bas (rouge) et de la clôture (gris), zones colorées entre elles.
    HlcArea,
    /// Colonnes : une colonne du bas jusqu'à la clôture, verte / rouge à 50 % selon la clôture d'avant.
    Columns,
    /// Haut-Bas : une barre bleue du haut au bas, avec les prix haut / bas écrits quand il y a la place.
    HiLo,
    /// Barres HLC : comme les barres OHLC sans le trait d'ouverture ; couleur selon la clôture d'avant.
    HlcBars,
    /// Renko (méthode traditionnelle de TradingView) : briques de taille fixe, sans le temps.
    Renko(RenkoBox),
    /// Footprint (comme LuxAlgo) : bougies fines et, à côté, les achats × ventes de chaque rangée de prix.
    Footprint,
    /// TPO (comme LuxAlgo) : un profil de lettres par session à la place des bougies.
    Tpo,
    /// Bougies de volume Argos : couleur selon le palier (LOW / MID / HIGH / GOLD), place et largeur selon le volume.
    Volume(Box<volume_candles::Settings>),
}

/// Taille des briques Renko.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenkoBox {
    /// ATR de N bougies à la dernière bougie (défaut de TradingView : 14), arrondi à la précision du symbole.
    Atr(usize),
    /// Taille fixe en prix.
    Fixed(f64),
}

impl Default for RenkoBox {
    fn default() -> Self {
        RenkoBox::Atr(14)
    }
}

impl CandleStyle {
    /// Tous les types, dans l'ordre du menu (bougies de volume Argos en premier : c'est le défaut).
    pub fn all() -> Vec<CandleStyle> {
        vec![
            CandleStyle::Volume(Box::default()),
            CandleStyle::Classic,
            CandleStyle::Hollow,
            CandleStyle::HeikinAshi { real_last_price: true },
            CandleStyle::Bars,
            CandleStyle::HlcBars,
            CandleStyle::Line,
            CandleStyle::LineMarkers,
            CandleStyle::StepLine,
            CandleStyle::Area,
            CandleStyle::HlcArea,
            CandleStyle::Baseline,
            CandleStyle::Columns,
            CandleStyle::HiLo,
            CandleStyle::Renko(RenkoBox::default()),
            CandleStyle::Footprint,
            CandleStyle::Tpo,
        ]
    }

    /// Icône du type (module `icons`), pour le bouton de la barre flottante.
    pub fn icon(&self) -> &'static str {
        match self {
            CandleStyle::Classic | CandleStyle::Volume(_) => "candles",
            CandleStyle::Bars | CandleStyle::HlcBars => "bars",
            CandleStyle::Hollow => "hollow",
            CandleStyle::HeikinAshi { .. } => "ha",
            CandleStyle::Line => "line",
            CandleStyle::Area => "area",
            CandleStyle::Baseline => "baseline",
            CandleStyle::LineMarkers => "linemk",
            CandleStyle::StepLine => "step",
            CandleStyle::HlcArea => "hlcarea",
            CandleStyle::Columns => "columns",
            CandleStyle::HiLo => "hilo",
            CandleStyle::Renko(_) => "renko",
            CandleStyle::Footprint => "footprint",
            CandleStyle::Tpo => "tpo",
        }
    }

    /// Nom affiché du type (comme dans le menu de TradingView).
    pub fn name(&self) -> &'static str {
        match self {
            CandleStyle::Classic => "Bougies",
            CandleStyle::Bars => "Barres",
            CandleStyle::Hollow => "Bougies creuses",
            CandleStyle::HeikinAshi { .. } => "Heikin Ashi",
            CandleStyle::Line => "Ligne",
            CandleStyle::Area => "Aire",
            CandleStyle::Baseline => "Ligne de base",
            CandleStyle::LineMarkers => "Ligne avec marqueurs",
            CandleStyle::StepLine => "Ligne en escalier",
            CandleStyle::HlcArea => "Aire HLC",
            CandleStyle::Columns => "Colonnes",
            CandleStyle::HiLo => "Haut-Bas",
            CandleStyle::HlcBars => "Barres HLC",
            CandleStyle::Renko(_) => "Renko",
            CandleStyle::Footprint => "Footprint",
            CandleStyle::Tpo => "TPO",
            CandleStyle::Volume(_) => "Bougies de volume Argos",
        }
    }
}

/// Place de chaque bougie sur l'axe du temps : 1 par bougie en classique, selon le volume en bougies de volume.
/// La fenêtre desktop s'en sert aussi pour le zoom et le défilement.
pub fn slots(bars: &[Bar], style: &CandleStyle) -> Slots {
    match style {
        CandleStyle::Volume(s) => Slots::weighted(volume_candles::compute(bars, s).into_iter().map(|v| v.width)),
        CandleStyle::Renko(_) => Slots::uniform(series(bars, style).len()),
        _ => Slots::uniform(bars.len()),
    }
}

/// Taille d'une brique Renko pour ces bougies.
pub fn renko_size(bars: &[Bar], b: RenkoBox) -> f64 {
    match b {
        RenkoBox::Fixed(x) => x,
        RenkoBox::Atr(len) => {
            let a = argos_script::ta::atr(bars, len.max(1)).last().copied().unwrap_or(f64::NAN);
            let k = 10f64.powi(data_decimals(bars) as i32);
            (a * k).round() / k
        }
    }
}

/// Série affichée : les bougies telles quelles, ou transformées (Heikin Ashi, briques Renko).
pub fn series<'a>(bars: &'a [Bar], style: &CandleStyle) -> std::borrow::Cow<'a, [Bar]> {
    use std::borrow::Cow;
    match style {
        CandleStyle::HeikinAshi { .. } => Cow::Owned(argos_core::heikin_ashi(bars)),
        CandleStyle::Renko(b) => Cow::Owned(argos_core::renko(bars, renko_size(bars, *b))),
        _ => Cow::Borrowed(bars),
    }
}

/// Largeur du corps (px) dans une place de `t` px : même règle d'écart que TradingView, largeur impaire pour centrer la mèche.
pub(crate) fn candle_width(t: f64) -> f64 {
    if (2.5..=4.0).contains(&t) {
        return 3.0;
    }
    let k = 1.0 - 0.2 * (t.max(4.0) - 4.0).atan() / std::f64::consts::FRAC_PI_2;
    let w = (t * k).floor().min(t.floor()).max(1.0);
    if w >= 2.0 && w % 2.0 == 0.0 {
        w - 1.0
    } else {
        w
    }
}

/// Types qui s'ajustent sur les clôtures seulement (comme TradingView).
pub(crate) fn close_scaled(style: &CandleStyle) -> bool {
    matches!(
        style,
        CandleStyle::Line | CandleStyle::Area | CandleStyle::Baseline | CandleStyle::LineMarkers | CandleStyle::StepLine | CandleStyle::Columns
    )
}
