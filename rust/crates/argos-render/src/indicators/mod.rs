//! 📈 Indicateurs d'Argos sur la chart (calculés par `argos_script`, testés contre le site et TradingView).
//! Une table (`Ind`) : nom et section (comme Argos v1, même ordre) et dessin. Le calcul est fait par la fenêtre
//! seulement quand les bougies changent (`compute`), le dessin à chaque image (`draw`).
//! - Classiques : RSI 14, MACD, Bandes de Bollinger, Croisement EMA 9/21, Stochastique, ATR 14, VWAP, Volume relatif
//!   (un `Plot` générique : lignes, histogramme, niveaux, comme le registre de v1).
//! - Sur le prix : EMA multiples, Argos OB · BB · FVG · iFVG, Argos SMC, Argos Bollinger Bands,
//!   Argos Volume Profile, Argos Pivot VWAP (et Multi), Argos Footprint, Argos Bubbles, Argos Tape,
//!   Argos Price Action.
//! - Oscillateurs (bande en bas de la chart) : Argos Cipher Pro, Argos AI Trend V Pro, Argos Heatmap Volume,
//!   Argos Bollinger Oscillator, et les classiques en sous-fenêtre.
//!
//! Fichiers : `mod.rs` (table `Ind`, calcul `compute`, bandes, `draw`, axes), `classic_plots` (classiques en `Plot`),
//! `osc` (oscillateurs en bande), `flow_draw` (volume et flux), `zones` (SMC, Vector Zone, figures, price action),
//! `tables` (tableaux et étiquettes).

use crate::palette as pal;
use crate::palette::{GOLD, GREY};
use crate::shapes::lines;
use crate::text::{Align, Rgb, Text};
use crate::viewport::{PriceScale, Slots, Viewport};
use argos_core::Bar;
use argos_script::indicators::ai_trend_volume::Tier;
use argos_script::indicators::{
    ai_trend, anchored_vwap, cipher, classic, flow, order_blocks, patterns, pivot_vwap, smc, smc_pro, tbo, textbook, tvd, vector_zone,
};
use argos_script::v1_inputs as v1;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tiny_skia::Pixmap;

// calcul des classiques, puis dessins par famille (même API : `spark` reste `indicators::spark`)
mod classic_plots;
mod flow_draw;
pub mod lens;
mod osc;
mod tables;
mod zones;
pub use classic_plots::spark;
use classic_plots::*;
use flow_draw::*;
use osc::*;
use tables::*;
use zones::*;

/// Les indicateurs, dans l'ordre d'Argos v1 (classiques, puis natifs ; Pivot VWAP en plus à la fin).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Ind {
    Rsi,
    Macd,
    Bollinger,
    EmaCross,
    Stochastic,
    Atr,
    Vwap,
    RelVolume,
    Tvd,
    EmaMulti,
    Cipher,
    AiTrend,
    Tbo,
    Smc,
    Heatmap,
    Profile,
    Bands,
    BandsOsc,
    SmcPro,
    SwingVwap,
    SmartSwingVwap,
    Footprint,
    Bubbles,
    Tape,
    PriceAction,
    AnchoredVwap,
    Patterns,
    BookHeat,
    LiqMap,
    VectorZone,
    MtfTable,
    // 📚 classiques des manuels (ajoutés à la fin : les réglages « source » gardent leur index)
    StochRsi,
    Ichimoku,
    Keltner,
    Donchian,
    Psar,
    Dmi,
    Cci,
    WilliamsR,
    Obv,
    Supertrend,
    Aroon,
    Mfi,
    // 🧮 delta cumulé (vrais trades quand il y en a, sinon estimé)
    Cvd,
    // 🔎 Argos Lens : régime, scores long / short, setup (affichage seul)
    Lens,
    // 📜 built-in indicators written in Argos Script (their source runs as it is; see `argos_script::integres`)
    SpringTension,
    // 📜 TradingView's moving averages, their exact code (lot « moyennes »)
    Sma,
    Wma,
    Hma,
    Vwma,
    Smma,
    MaRibbon,
    /// 📜 Argos Bollinger + RSI (Argos Script, Jee 19 Nov).
    BollingerRsi,
    /// 🧱 Argos Murs: the biggest resting order blocks summed over several exchanges (live order books).
    ArgosMurs,
    /// 📜 Argos Vague (Argos Script, Jee 20 Nov).
    ArgosVague,
    /// 📜 LuxAlgo's Smart Money Concepts (its Pine v6 run as it is, Jee 25 Nov).
    LuxAlgoSmc,
}

impl Ind {
    pub const ALL: [Ind; 56] = [
        Ind::Rsi,
        Ind::Macd,
        Ind::Bollinger,
        Ind::EmaCross,
        Ind::Stochastic,
        Ind::Atr,
        Ind::Vwap,
        Ind::RelVolume,
        Ind::Tvd,
        Ind::EmaMulti,
        Ind::Cipher,
        Ind::AiTrend,
        Ind::Tbo,
        Ind::Smc,
        Ind::Heatmap,
        Ind::Profile,
        Ind::Bands,
        Ind::BandsOsc,
        Ind::SmcPro,
        Ind::SwingVwap,
        Ind::SmartSwingVwap,
        Ind::Footprint,
        Ind::Bubbles,
        Ind::Tape,
        Ind::PriceAction,
        Ind::AnchoredVwap,
        Ind::Patterns,
        Ind::BookHeat,
        Ind::LiqMap,
        Ind::VectorZone,
        Ind::MtfTable,
        Ind::StochRsi,
        Ind::Ichimoku,
        Ind::Keltner,
        Ind::Donchian,
        Ind::Psar,
        Ind::Dmi,
        Ind::Cci,
        Ind::WilliamsR,
        Ind::Obv,
        Ind::Supertrend,
        Ind::Aroon,
        Ind::Mfi,
        Ind::Cvd,
        Ind::Lens,
        Ind::SpringTension,
        Ind::Sma,
        Ind::Wma,
        Ind::Hma,
        Ind::Vwma,
        Ind::Smma,
        Ind::MaRibbon,
        Ind::BollingerRsi,
        Ind::ArgosMurs,
        Ind::ArgosVague,
        Ind::LuxAlgoSmc,
    ];

    /// Nom (catalogue unique, `argos_script::catalogue`).
    pub fn name(self) -> &'static str {
        crate::catalogue::fiche(self).nom
    }

    /// Section du panneau Indicateurs (catalogue unique).
    pub fn section(self) -> &'static str {
        crate::catalogue::fiche(self).section
    }

    /// Réglages : (groupes, table), lus dans le catalogue unique.
    pub fn params(self) -> (&'static [&'static str], &'static [argos_script::inputs::Param]) {
        crate::catalogue::reglages(self)
    }

    /// Hauteur de sa bande d'oscillateur : celle choisie en glissant son bord (`set_heights`), sinon celle par défaut.
    pub fn band_h(self) -> f64 {
        if PLEIN_ECRAN.with(|p| p.get()) == Some(self) {
            return PLEIN_H.with(|h| h.get());
        }
        if self.replie() {
            return BAND_REPLIE;
        }
        HEIGHTS.with(|m| m.borrow().get(&self).copied()).unwrap_or(self.default_h()) * SQUEEZE.with(|s| s.get())
    }

    /// 🖥 Panneau replié par ▾ (maquette) : 22 de haut, sa légende seule.
    pub fn replie(self) -> bool {
        REPLIES.with(|r| r.borrow().contains(&self))
    }

    /// Hauteur par défaut de sa bande (le Cipher a plus de lignes : bande plus haute, comme v1).
    pub fn default_h(self) -> f64 {
        match self {
            Ind::Cipher => 2.0 * OSC_H,
            Ind::AiTrend => 1.5 * OSC_H,
            // 📜 Argos Vague (mockup v7): close to half of the plot; a low screen squeezes it (`MAX_BANDS_SHARE`)
            Ind::ArgosVague => VAGUE_H,
            _ => OSC_H,
        }
    }

    /// Oscillateur (dessiné dans une bande en bas) ?
    pub fn oscillator(self) -> bool {
        matches!(
            self,
            Ind::Rsi
                | Ind::Macd
                | Ind::Stochastic
                | Ind::Atr
                | Ind::RelVolume
                | Ind::Cipher
                | Ind::AiTrend
                | Ind::Tbo
                | Ind::Heatmap
                | Ind::BandsOsc
                | Ind::StochRsi
                | Ind::Dmi
                | Ind::Cci
                | Ind::WilliamsR
                | Ind::Obv
                | Ind::Aroon
                | Ind::Mfi
                | Ind::Cvd
                | Ind::SpringTension
                | Ind::ArgosVague
        )
    }
}

/// Indicateur simple, décrit comme le registre de v1 : lignes, histogramme, niveaux, signaux.
#[derive(Debug, Clone, PartialEq)]
pub struct Plot {
    pub ind: Ind,
    pub name: &'static str,
    /// Dans une bande en bas (sinon sur le prix).
    pub sub: bool,
    /// (valeurs, couleur, épaisseur).
    pub lines: Vec<(Vec<f64>, Rgb, f32)>,
    /// Histogramme : (valeur, couleur, opacité) par bougie ; vide s'il n'y en a pas.
    pub hist: Vec<(f64, Rgb, f32)>,
    /// Fonds horizontaux dans la bande : (de, à, couleur, opacité).
    pub zones: Vec<(f64, f64, Rgb, f32)>,
    /// Couleur imposée aux bougies (comme `barColors` de v1) ; vide : aucune.
    pub bars: Vec<Option<Rgb>>,
    /// Niveaux horizontaux (réglables : RSI 70 / 30, seuil du DMI…).
    pub levels: Vec<f64>,
    /// Échelle fixe (0–100) ; `None` : selon les bougies visibles.
    pub range: Option<(f64, f64)>,
    /// Signaux : (bougie, sous la bougie ?, couleur, texte) ; sur le prix, sous le bas / au-dessus du haut.
    pub marks: Vec<(usize, bool, Rgb, &'static str)>,
    /// Nuage entre deux lignes : (ligne a, ligne b, couleur, opacité).
    pub fill: Option<(usize, usize, Rgb, f32)>,
    /// The cloud's colour where line a is below line b (`None`: one colour), as TradingView's Ichimoku: green when
    /// senkou A is above senkou B, red otherwise (B-045).
    pub fill_down: Option<Rgb>,
    /// Values after the last bar, one list per line (empty: none): TradingView draws Ichimoku's cloud 25 bars ahead
    /// (B-045). The k-th value is the bar `bars.len() + k`.
    pub ahead: Vec<Vec<f64>>,
    /// Type de ligne de chaque ligne (onglet Style) : 0 plein, 1 tirets, 2 points ; vide : pleines.
    pub dash: Vec<usize>,
    /// 📋 Summary table rows (« Tableau résumé » ticked, as v1's `dash`); empty: no table.
    pub tableau: Vec<(&'static str, String)>,
}

const fn hex(c: u32) -> Rgb {
    Rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

/// Couleurs du registre de v1.
const ORANGE: Rgb = hex(0xff9800);
const EMA_COLORS: [Rgb; 6] = [hex(0xf0b90b), hex(0x2962ff), hex(0x26a69a), hex(0xb26cf0), hex(0xff9800), hex(0xe91e63)];

/// Valeurs des inputs de chaque indicateur (dans l'ordre de sa table) ; absent : valeurs par défaut.
pub type Settings = HashMap<Ind, Vec<f64>>;

/// Textes des inputs texte et symbole de chaque indicateur (par clé) ; absent : texte par défaut.
pub type Texts = HashMap<Ind, HashMap<String, String>>;

/// Nom dans la légende avec les valeurs de ses inputs (`display`), comme TradingView : « Argos TBO Pro Div (14, 9…) ».
/// Au plus 4 valeurs, puis « … ». 🧮 Argos Bubbles adds where its real trades come from ([`bubbles_statut`]).
pub fn legend_title(ind: Ind, out: &IndOut) -> String {
    let base = legend_base(ind, out);
    match (ind, out.bubbles.as_ref()) {
        (Ind::Bubbles, Some(b)) => format!("{base} · {}", bubbles_statut(b)),
        // 🔥 the heatmap reads Hyperliquid's book only (also on a Binance or Bybit chart): said in its legend
        // (right after the name: a phone that shortens the legend cuts the settings, never the source)
        (Ind::BookHeat, _) => format!(
            "{} · {}{}",
            ind.name(),
            out.carnets.as_deref().unwrap_or("carnet Hyperliquid USDC"),
            base.strip_prefix(ind.name()).unwrap_or("")
        ),
        // 🌊 mockup v7: the name, « <style> · <engine> », the last wave value (drawn in a glass pill, see indleg)
        (Ind::ArgosVague, _) if out.vague.is_some() => {
            let v = vague_valeur(out).map_or(String::new(), |(x, _)| format!("  {}", crate::viewport::format_number(x, 1)));
            format!("{}  {}{v}", ind.name(), out.vague.as_deref().unwrap_or(""))
        }
        (Ind::ArgosMurs, _) => match &out.murs {
            Some(m) => format!("{} {}", ind.name(), m.long),
            None => ind.name().to_string(),
        },
        // 🧮 no real trades for this market (exchange not connected, flow unavailable, contract size unknown…): said
        // as long as it lasts, gone as soon as a real complete bar arrives; right after the name, so a phone that
        // shortens a long legend cuts the settings, never the warning
        (Ind::Footprint, _) if vrais_trades_fp(out) && out.fp_multi.is_some() => {
            format!("{} · {}", ind.name(), out.fp_multi.as_ref().map_or(String::new(), |m| m.nombre()))
        }
        (Ind::Footprint, _) | (Ind::Cvd, _) if !(if ind == Ind::Footprint { vrais_trades_fp(out) } else { vrais_trades(out) }) => {
            let reglages = base.strip_prefix(ind.name()).map(str::trim).filter(|r| !r.is_empty());
            match reglages {
                Some(r) => format!("{} · {ESTIME} · {r}", ind.name()),
                None => format!("{base} · {ESTIME}"),
            }
        }
        _ => base,
    }
}

/// 🧮 Footprint and CVD without any real trade: estimated from the bars, said in their legend.
pub const ESTIME: &str = "estimé (pas de vrais trades)";

/// 🧮 Has the market of the chart any real trades (a complete bar of real orderflow)?
pub fn vrais_trades(out: &IndOut) -> bool {
    out.reel.iter().any(|r| r.complete)
}

/// 🏦 Has the footprint any real trades: those of the chart's market, or (multi-exchange) any bar of real trades.
pub fn vrais_trades_fp(out: &IndOut) -> bool {
    match (&out.fp_multi, &out.footprint) {
        (Some(_), Some(f)) => f.bars.iter().any(|b| b.origine == flow::Origine::Reel),
        _ => vrais_trades(out),
    }
}

/// 🧮 Argos Bubbles without any real trade (for this market or this unit): said in its legend.
pub const BULLES_ABSENTES: &str = "Bulles : vrais trades absents pour ce marché ou cette unité";

/// 🧮 Where Argos Bubbles' real trades come from, for its legend: « vrais trades depuis 2026-11-18 14:30 ET » (the
/// first bar with real trades, New York time as the Tape), or [`BULLES_ABSENTES`].
pub fn bubbles_statut(b: &flow::Bubbles) -> String {
    match b.depuis {
        Some(t) => format!("vrais trades depuis {} ET", crate::journal::date_txt(t * 1000)),
        None => BULLES_ABSENTES.to_string(),
    }
}

/// The legend name with its input values, without any status.
fn legend_base(ind: Ind, out: &IndOut) -> String {
    let vals = values(&out.inputs, ind);
    let empty = HashMap::new();
    let tx = argos_script::inputs::Texts {
        params: ind.params().1,
        vals: out.texts.get(&ind).unwrap_or(&empty),
    };
    let all = argos_script::inputs::Inputs {
        params: ind.params().1,
        vals: &vals,
    }
    .legend(&tx);
    let parts: Vec<&str> = all.split(", ").filter(|s| !s.is_empty()).collect();
    // 📜 a built-in written in Argos Script: like TradingView, all its values, separated by spaces (« Spring Tension
    // 14 14 5 35 65 RSI sur hlc3 20 5 20 10 4 »)
    if argos_script::catalogue::source(crate::catalogue::fiche(ind)).is_some() {
        return [ind.name()].into_iter().chain(parts).collect::<Vec<_>>().join(" ");
    }
    match parts.len() {
        0 => ind.name().to_string(),
        n if n > 4 => format!("{} ({}…)", ind.name(), parts[..4].join(", ")),
        _ => format!("{} ({})", ind.name(), parts.join(", ")),
    }
}

/// Sorties qu'un autre indicateur peut prendre comme source (`input.source`) : (« Indicateur : sortie », valeur de
/// l'input).
pub fn source_choices(on: &[Ind]) -> Vec<(String, f64)> {
    source_outputs(on)
        .into_iter()
        .map(|(i, k, n)| {
            (
                format!("{} : {n}", i.name()),
                argos_script::inputs::source_ext(Ind::ALL.iter().position(|x| *x == i).unwrap_or(0), k),
            )
        })
        .collect()
}

/// Sorties de ces indicateurs : (indicateur, sortie, nom).
pub fn source_outputs(on: &[Ind]) -> Vec<(Ind, usize, &'static str)> {
    let names = |i: Ind| -> &'static [&'static str] {
        match i {
            Ind::Rsi => &["RSI"],
            Ind::Macd => &["MACD", "Signal"],
            Ind::Stochastic => &["%K", "%D"],
            Ind::Atr => &["ATR"],
            Ind::Vwap => &["VWAP"],
            Ind::AnchoredVwap => &["VWAP ancré"],
            Ind::Tvd => &["Fast", "Slow", "AI"],
            Ind::Cipher => &["WT1", "WT2"],
            Ind::AiTrend => &["AI", "Signal"],
            Ind::Tbo => &["Delta", "Signal"],
            Ind::StochRsi => &["%K", "%D"],
            Ind::Ichimoku => &["Tenkan", "Kijun", "Senkou A", "Senkou B", "Chikou"],
            Ind::Keltner | Ind::Donchian => &["Haut", "Milieu", "Bas"],
            Ind::Psar => &["SAR"],
            Ind::Dmi => &["+DI", "−DI", "ADX"],
            Ind::Cci => &["CCI"],
            Ind::WilliamsR => &["%R"],
            Ind::Obv => &["OBV"],
            Ind::Supertrend => &["Haussier", "Baissier"],
            Ind::Aroon => &["Haut", "Bas"],
            Ind::Mfi => &["MFI"],
            Ind::Cvd => &["CVD"],
            _ => &[],
        }
    };
    Ind::ALL
        .iter()
        .filter(|i| on.contains(i))
        .flat_map(|&i| names(i).iter().enumerate().map(move |(k, n)| (i, k, *n)))
        .collect()
}

/// Série d'une sortie (pour une source qui vient d'un autre indicateur).
pub fn output_series(out: &IndOut, ind: Ind, k: usize) -> Option<Vec<f64>> {
    let line = |n: usize| out.plots.iter().find(|p| p.ind == ind).and_then(|p| p.lines.get(n).map(|l| l.0.clone()));
    match ind {
        Ind::Tvd => out.tvd.as_ref().map(|t| [&t.fast, &t.slow, &t.ai][k.min(2)].clone()),
        Ind::Cipher => out.cipher.as_ref().map(|c| if k == 0 { c.wt1.clone() } else { c.wt2.clone() }),
        Ind::AiTrend => out.ai.as_ref().map(|a| if k == 0 { a.ai.clone() } else { a.signal.clone() }),
        Ind::Tbo => out.tbo.as_ref().map(|t| if k == 0 { t.core.delta.clone() } else { t.signal.clone() }),
        Ind::AnchoredVwap => out.avwap.as_ref().map(|a| a.vwap.clone()),
        _ => line(k),
    }
}

/// Valeurs de `ind` (défauts si rien n'est gardé).
/// ⚙️ Réglages des Pivot VWAP (une échelle, ou plusieurs) depuis leurs inputs.
fn pivot_config(set: &Settings, ind: Ind) -> pivot_vwap::Config {
    let vals = values(set, ind);
    let inp = argos_script::inputs::Inputs {
        params: ind.params().1,
        vals: &vals,
    };
    let mut c = pivot_vwap::Config::default();
    if ind == Ind::SwingVwap {
        c.span = inp.len("span");
    } else {
        c.spans = vec![inp.len("s1"), inp.len("s2"), inp.len("s3")];
        c.min_votes = inp.len("votes").min(3);
        c.cut_on_break = inp.on("cut");
    }
    c
}

pub fn values(set: &Settings, ind: Ind) -> Vec<f64> {
    set.get(&ind).cloned().unwrap_or_else(|| argos_script::inputs::defaults(ind.params().1))
}

/// Résultats calculés (un par indicateur actif).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct IndOut {
    pub cipher: Option<cipher::Cipher>,
    pub ai: Option<ai_trend::AiTrend>,
    pub tbo: Option<tbo::Tbo>,
    pub smc: Option<smc::Smc>,
    pub smc_pro: Option<smc_pro::SmcPro>,
    pub swing: Option<pivot_vwap::PivotVwap>,
    pub smart: Option<pivot_vwap::PivotVwap>,
    pub tvd: Option<tvd::Tvd>,
    /// Indicateurs simples, dans l'ordre de `Ind::ALL`.
    pub plots: Vec<Plot>,
    pub profile: Option<classic::Profile>,
    pub footprint: Option<flow::Footprint>,
    /// 🧮 Orderflow réel à l'unité de la chart (footprint en type de chart).
    pub reel: Vec<argos_core::orderflow::FlowBar>,
    /// 🏦 Footprint multi-bourses : ses bourses, la part de chacune, le détail d'une rangée (`None` : le footprint
    /// d'une seule bourse). Posé par la fenêtre après `compute`, avec `footprint` refait sur les trades additionnés.
    pub fp_multi: Option<std::sync::Arc<crate::fpmulti::FpMulti>>,
    pub bubbles: Option<flow::Bubbles>,
    /// Ruban en direct : son état passe d'un calcul au suivant (`compute` le reçoit).
    pub tape: Option<flow::Tape>,
    pub pa: Option<order_blocks::OrderBlocks>,
    /// 🔺 Patterns auto : figures trouvées (les plus récentes d'abord).
    pub pat: Option<Vec<patterns::Pattern>>,
    /// 🧭 Argos Vector Zone Pro.
    pub vz: Option<vector_zone::VectorZone>,
    /// 📊 Tableau multi-UT.
    pub mtf: Option<Vec<argos_script::indicators::mtf_table::Row>>,
    /// 🔎 Argos Lens.
    pub lens: Option<lens::LensVue>,
    pub avwap: Option<anchored_vwap::AnchoredVwap>,
    /// 🧱 Argos Murs: what its legend says after the name (exchanges and count, books, or the replay's note).
    pub murs: Option<crate::murs::Legende>,
    /// 🔥 📖 The books the heatmap reads (« Binance USDT 1/1 »); `None`: Hyperliquid's live book.
    pub carnets: Option<String>,
    /// Indicateurs actifs (ordre du panneau).
    pub on: Vec<Ind>,
    /// Indicateurs masqués (👁 de la légende) : calculés mais pas dessinés ; un oscillateur garde sa bande vide.
    pub hidden: Vec<Ind>,
    /// ☆ Favoris (les mêmes que le Marketplace).
    pub favs: Vec<Ind>,
    /// Bougie montante (clôture ≥ ouverture) ? Couleur des bougies Vector du TBO et de l'AI Trend.
    pub bar_up: Vec<bool>,
    /// Inputs utilisés pour ce calcul (le dessin en lit aussi : couleurs, éléments affichés).
    pub inputs: Settings,
    /// Textes des inputs texte et symbole (remplis par la fenêtre).
    pub texts: Texts,
    /// 📜 Scripts Argos (éditeur), calculés par la fenêtre.
    pub scripts: Vec<crate::scriptview::ScriptView>,
    /// 📜 Built-in indicators written in Argos Script (Spring Tension…): their script's output, drawn in their band.
    pub integres: Vec<(Ind, crate::scriptview::ScriptView)>,
    /// 🌊 Argos Vague's legend after its name: the style really used (« Auto » resolved by the chart's unit) and the
    /// engine, « Trader du jour · Classique » (mockup v7).
    pub vague: Option<String>,
}

impl IndOut {
    /// Combien d'oscillateurs à dessiner (une bande chacun).
    pub fn oscillators(&self) -> usize {
        self.bands(0.0).len()
    }

    pub fn shown(&self, i: Ind) -> bool {
        !self.hidden.contains(&i)
    }

    /// Bandes d'oscillateurs de bas en haut : (indicateur, haut de la bande), sous une zone de prix de hauteur `plot_h`.
    pub fn bands(&self, plot_h: f64) -> Vec<(Ind, f64)> {
        let subs = self.plots.iter().filter(|p| p.sub).map(|p| p.ind);
        let order: Vec<Ind> = subs
            .chain(self.ai.as_ref().map(|_| Ind::AiTrend))
            .chain(self.tbo.as_ref().map(|_| Ind::Tbo))
            .chain(self.cipher.as_ref().map(|_| Ind::Cipher))
            .chain(self.integres.iter().filter(|(_, v)| !v.out.overlay).map(|(i, _)| *i))
            .collect();
        // 🖥 ordre choisi (↑ ↓) d'abord ; les autres gardent leur place (tri stable)
        let mut order = order;
        ORDRE.with(|o| {
            let o = o.borrow();
            order.sort_by_key(|i| o.iter().position(|x| x == i).unwrap_or(usize::MAX));
        });
        // ⛶ a panel full screen: it alone
        if let Some(p) = plein_ecran().filter(|p| order.contains(p)) {
            order = vec![p];
        }
        let mut bottom = plot_h;
        order
            .into_iter()
            .map(|i| {
                bottom -= i.band_h();
                (i, bottom)
            })
            .collect()
    }

    /// Bandes des scripts sans `overlay`, au-dessus de celles des indicateurs : (index du script, haut de la bande).
    pub fn script_bands(&self, plot_h: f64) -> Vec<(usize, f64)> {
        let bands = self.bands(0.0);
        // (⛶ the same condition as bands(): a panel full screen only if it is on the chart)
        let plein = bands.iter().any(|(i, _)| Some(*i) == plein_ecran());
        let mut bottom = plot_h - bands.iter().map(|(i, _)| i.band_h()).sum::<f64>();
        self.scripts
            .iter()
            .enumerate()
            .filter(|(_, s)| !s.out.overlay && !plein)
            .map(|(k, s)| {
                bottom -= s.band_h();
                (k, bottom)
            })
            .collect()
    }

    /// 📜 Script pane whose top edge is under `y` (±5 px, 👆 ±14 by the finger, like `band_edge`): its index in
    /// `scripts`.
    pub fn script_edge(&self, plot_h: f64, y: f64) -> Option<usize> {
        let reach = edge_reach();
        self.script_bands(plot_h).into_iter().find(|(_, top)| (y - top).abs() <= reach).map(|(k, _)| k)
    }

    /// 📜 Script pane under `y` (off its top edge, like `band_at`): its index in `scripts`.
    pub fn script_at(&self, plot_h: f64, y: f64) -> Option<usize> {
        self.script_bands(plot_h)
            .into_iter()
            .find(|(k, top)| y > top + 5.0 && y < top + self.scripts[*k].band_h())
            .map(|(k, _)| k)
    }

    /// Bande dont le bord du haut est sous `y` (±5 px ; 👆 ±14 au doigt, [`edge_reach`]) : on le glisse pour agrandir
    /// la bande.
    pub fn band_edge(&self, plot_h: f64, y: f64) -> Option<Ind> {
        let reach = edge_reach();
        self.bands(plot_h).into_iter().find(|(_, top)| (y - top).abs() <= reach).map(|(i, _)| i)
    }

    /// Bande sous `y` (hors de son bord du haut).
    pub fn band_at(&self, plot_h: f64, y: f64) -> Option<Ind> {
        self.bands(plot_h)
            .into_iter()
            .find(|(i, top)| y > top + 5.0 && y < top + i.band_h())
            .map(|(i, _)| i)
    }

    /// Hauteur prise en bas par les bandes d'oscillateurs et des scripts (la zone de prix finit au-dessus).
    pub fn osc_h(&self) -> f64 {
        self.bands(0.0).iter().map(|(i, _)| i.band_h()).sum::<f64>() + self.script_bands(0.0).iter().map(|(k, _)| self.scripts[*k].band_h()).sum::<f64>()
    }

    /// Couleur imposée à la bougie `i` (bougies vector de TVD Pro), comme `barColors` de v1.
    pub fn bar_color(&self, i: usize) -> Option<Rgb> {
        let from_plots = || self.plots.iter().filter(|p| self.shown(p.ind)).find_map(|p| p.bars.get(i).copied().flatten());
        let tvd = self
            .tvd
            .as_ref()
            .filter(|_| self.shown(Ind::Tvd))
            .and_then(|t| t.vector.get(i).copied().flatten())
            .map(|v| match v {
                tvd::Vector::HighUp => hex(0x00e676),
                tvd::Vector::HighDown => hex(0xff1744),
                tvd::Vector::MidUp => hex(0x2962ff),
                tvd::Vector::MidDown => hex(0xd500f9),
            });
        // bougies Vector 150+ / 200+ du TBO (« Colorer les bougies Vector »)
        let tbo = || {
            let t = self.tbo.as_ref().filter(|_| self.shown(Ind::Tbo))?;
            let vals = values(&self.inputs, Ind::Tbo);
            let p = argos_script::inputs::Inputs {
                params: &tbo::PARAMS,
                vals: &vals,
            };
            let lvl = *t.vector.get(i)?;
            if !p.on("paint") || lvl == 0 {
                return None;
            }
            let up = self.bar_up.get(i).copied().unwrap_or(true);
            let k = match (lvl, up) {
                (2, true) => "cV2u",
                (2, false) => "cV2d",
                (_, true) => "cV1u",
                _ => "cV1d",
            };
            Some(crate::palette::on_chart(p.rgb(k)))
        };
        let script = || {
            self.scripts
                .iter()
                .filter(|s| !s.hidden)
                .find_map(|s| s.out.barcolor.get(i).copied().flatten())
                .map(|c| {
                    let (r, g, b, _) = argos_script::lang::value::rgba(c);
                    Rgb(r, g, b)
                })
        };
        // Volume Vector de l'AI Trend (le pattern de retournement passe devant si « Pattern color overrides Vector color »)
        let ai = || {
            let a = self.ai.as_ref().filter(|_| self.shown(Ind::AiTrend))?;
            let vals = values(&self.inputs, Ind::AiTrend);
            let p = argos_script::inputs::Inputs {
                params: &ai_trend::PARAMS,
                vals: &vals,
            };
            if !p.on("vv_paint") {
                return None;
            }
            let v = &a.volume;
            let pat = |list: &[(usize, &str)]| list.binary_search_by_key(&i, |x| x.0).is_ok();
            let k = if p.on("pat_over") && pat(&v.bull_patterns) {
                "pat_cbu"
            } else if p.on("pat_over") && pat(&v.bear_patterns) {
                "pat_cbe"
            } else {
                let up = self.bar_up.get(i).copied()?;
                match (*v.tier.get(i)?, up) {
                    (Tier::Gold, true) => "vv_gbu",
                    (Tier::High, true) => "vv_hbu",
                    (Tier::Mid, true) => "vv_mbu",
                    (Tier::Low, true) => "vv_lbu",
                    (Tier::Gold, false) => "vv_gbe",
                    (Tier::High, false) => "vv_hbe",
                    (Tier::Mid, false) => "vv_mbe",
                    (Tier::Low, false) => "vv_lbe",
                }
            };
            // couleur 100 % transparente (LOW par défaut) : la bougie garde sa couleur normale
            (p.opacity(k) > 0.0).then(|| col(&p, k))
        };
        // bougies vector PVSRA de l'Argos Vector Zone Pro
        let vz = || {
            let z = self.vz.as_ref().filter(|_| self.shown(Ind::VectorZone))?;
            let vals = values(&self.inputs, Ind::VectorZone);
            let p = argos_script::inputs::Inputs {
                params: &vector_zone::PARAMS,
                vals: &vals,
            };
            if !p.on("showVec") {
                return None;
            }
            let up = *z.bull.get(i)?;
            let k = match ((*z.vec.get(i)?)?, up) {
                (vector_zone::VecKind::Climax, true) => "cBullC",
                (vector_zone::VecKind::Climax, false) => "cBearC",
                (vector_zone::VecKind::Vector, true) => "cBullA",
                (vector_zone::VecKind::Vector, false) => "cBearA",
            };
            Some(crate::palette::on_chart(p.rgb(k)))
        };
        script().or(tvd).or_else(tbo).or_else(ai).or_else(vz).or_else(from_plots)
    }
}

/// Calcule les indicateurs actifs (`on`) sur ces bougies ; `tape` : le ruban du calcul d'avant (il continue), `now` :
/// heure de ce calcul (secondes Unix, pour les lignes du ruban), `lower` : bougies des slots MTF de l'AI Trend plus petits que la chart.
/// `reel` : 🧮 l'orderflow réel du marché à l'unité de la chart (trié par heure ; vide : tout est estimé), pour le
/// footprint, le CVD, les bulles (vrais trades seulement) et le footprint en type de chart.
pub fn compute(
    bars: &[Bar],
    tf_seconds: i64,
    on: &[Ind],
    set: &Settings,
    tape: Option<flow::Tape>,
    now: i64,
    lower: &ai_trend::LowerBars,
    reel: &[argos_core::orderflow::FlowBar],
) -> IndOut {
    let has = |i: Ind| on.contains(&i) && bars.len() > 30;
    let mut out = IndOut {
        cipher: has(Ind::Cipher).then(|| cipher::compute(bars, &cipher::Settings::from_inputs(&values(set, Ind::Cipher)))),
        ai: has(Ind::AiTrend).then(|| ai_trend::compute(bars, tf_seconds, &ai_trend::Settings::from_inputs(&values(set, Ind::AiTrend)), lower)),
        tbo: has(Ind::Tbo).then(|| tbo::compute(bars, tf_seconds, &tbo::Settings::from_inputs(&values(set, Ind::Tbo)))),
        smc: has(Ind::Smc).then(|| smc::compute(bars, tf_seconds, &smc::Settings::from_inputs(&values(set, Ind::Smc)))),
        smc_pro: has(Ind::SmcPro).then(|| smc_pro::compute(bars, tf_seconds, &smc_pro::Settings::from_inputs(&values(set, Ind::SmcPro)))),
        swing: has(Ind::SwingVwap).then(|| pivot_vwap::compute(bars, &pivot_config(set, Ind::SwingVwap))),
        tvd: has(Ind::Tvd).then(|| tvd::compute(bars, &values(set, Ind::Tvd))),
        smart: has(Ind::SmartSwingVwap).then(|| pivot_vwap::compute_multi(bars, &pivot_config(set, Ind::SmartSwingVwap))),
        plots: Ind::ALL.iter().filter(|i| has(**i)).filter_map(|&i| plot(i, bars, set, reel)).collect(),
        profile: if has(Ind::Profile) {
            classic::volume_profile(bars, &values(set, Ind::Profile))
        } else {
            None
        },
        footprint: has(Ind::Footprint).then(|| flow::footprint(bars, &values(set, Ind::Footprint), reel)),
        reel: reel.to_vec(),
        fp_multi: None,
        bubbles: has(Ind::Bubbles).then(|| flow::bubbles(bars, &values(set, Ind::Bubbles), reel, &|v| crate::viewport::format_price(v, 6))),
        tape: has(Ind::Tape).then(|| {
            let mut t = tape.unwrap_or_default();
            t.update(bars, tf_seconds, &values(set, Ind::Tape), now);
            t
        }),
        pa: has(Ind::PriceAction).then(|| order_blocks::compute(bars, &values(set, Ind::PriceAction))),
        pat: has(Ind::Patterns).then(|| patterns::compute(bars, &values(set, Ind::Patterns))),
        vz: has(Ind::VectorZone).then(|| vector_zone::compute(bars, tf_seconds, now, &values(set, Ind::VectorZone))),
        mtf: has(Ind::MtfTable).then(|| argos_script::indicators::mtf_table::compute(bars, tf_seconds, now, &values(set, Ind::MtfTable))),
        lens: has(Ind::Lens).then(|| lens::compute(bars, tf_seconds, now, &values(set, Ind::Lens))),
        avwap: None,
        murs: None,
        carnets: None,
        on: on.to_vec(),
        hidden: vec![],
        favs: vec![],
        bar_up: if has(Ind::Tbo) || has(Ind::AiTrend) {
            bars.iter().map(|b| b.close >= b.open).collect()
        } else {
            vec![]
        },
        inputs: set.clone(),
        texts: Texts::new(),
        scripts: vec![],
        integres: Ind::ALL
            .iter()
            .filter(|i| has(**i))
            .filter_map(|&i| integre(i, bars, tf_seconds, set, now, reel))
            .collect(),
        vague: has(Ind::ArgosVague).then(|| vague_legende(set, tf_seconds)).flatten(),
    };
    // en dernier : sa source peut être la sortie d'un autre indicateur (input.source)
    if has(Ind::AnchoredVwap) {
        let a = anchored_vwap::compute(bars, &values(set, Ind::AnchoredVwap), &|i, k| {
            Ind::ALL.get(i).and_then(|&ind| output_series(&out, ind, k))
        });
        out.avwap = Some(a);
    }
    out
}

/// 📜 A built-in indicator written in Argos Script: its exact source run on the bars with its ⚙ values (by title), as a
/// user script would be. A script error keeps an empty drawing and says why (shown in the band). 🎨 Its ⚙ Style
/// (after the inputs) is not given to the script: it goes with the drawing.
/// 🧮 The script also gets the real delta of each bar (`argos.real_delta`, from `reel`) and whether the last bar is
/// still forming at `now` (`barstate.isconfirmed` false on it).
fn integre(
    i: Ind,
    bars: &[Bar],
    tf_seconds: i64,
    set: &Settings,
    now: i64,
    reel: &[argos_core::orderflow::FlowBar],
) -> Option<(Ind, crate::scriptview::ScriptView)> {
    let src = argos_script::catalogue::source(crate::catalogue::fiche(i))?;
    let specs = argos_script::catalogue::entrees(src);
    let vals = values(set, i);
    let choisies = argos_script::lang::reglages::choisies(&specs, &vals, &Default::default());
    let opts = run_options(bars, tf_seconds, now, reel);
    let (out, error) = match argos_script::lang::run_with(src, bars, tf_seconds, &choisies, &opts) {
        Ok(out) => (out, None),
        Err(e) => (argos_script::lang::Output::default(), Some(format!("Ligne {} : {}", e.line, e.msg))),
    };
    Some((
        i,
        crate::scriptview::ScriptView {
            name: i.name().to_string(),
            out,
            hidden: false,
            error,
            style: argos_script::style_integres::style(src, &vals),
            id: Default::default(),
        },
    ))
}

/// 🌊 « <style> · <engine> » of Argos Vague, as its script resolves them: « Auto » = Scalpeur up to 15 min, Trader du
/// jour up to 4 h, Swing above.
pub fn vague_legende(set: &Settings, tf_seconds: i64) -> Option<String> {
    let src = argos_script::catalogue::source(crate::catalogue::fiche(Ind::ArgosVague))?;
    let specs = argos_script::catalogue::entrees(src);
    let c = argos_script::lang::reglages::choisies(&specs, &values(set, Ind::ArgosVague), &Default::default());
    let texte = |k: &str| match c.get(k) {
        Some(argos_script::lang::InVal::Text(t)) => Some(t.clone()),
        _ => None,
    };
    let style = texte("Style").unwrap_or_else(|| "Auto".into());
    let tf_min = tf_seconds / 60;
    let style = if style == "Auto" {
        if tf_min <= 15 {
            "Scalpeur".into()
        } else if tf_min <= 240 {
            "Trader du jour".into()
        } else {
            "Swing".into()
        }
    } else {
        style
    };
    Some(format!("{style} · {}", texte("Moteur").unwrap_or_else(|| "Classique".into())))
}

/// 🌊 Argos Vague's last wave value and its colour (the wave line's), for its legend.
pub fn vague_valeur(out: &IndOut) -> Option<(f64, Rgb)> {
    let (_, v) = out.integres.iter().find(|(i, _)| *i == Ind::ArgosVague)?;
    let p = v.out.plots.iter().find(|p| p.title == "Vague")?;
    let k = p.values.iter().rposition(|x| x.is_finite())?;
    let c = p.colors.get(k).copied().flatten().unwrap_or(0xB2B5BE);
    Some((p.values[k], hex(c & 0xFF_FF_FF)))
}

/// 🧮 What a script learns beyond OHLCV: the real delta of each bar (`reel`, empty: none) and whether the last bar is
/// still forming at `now` (seconds; 0: unknown, taken as closed).
pub fn run_options(bars: &[Bar], tf_seconds: i64, now: i64, reel: &[argos_core::orderflow::FlowBar]) -> argos_script::lang::RunOptions {
    argos_script::lang::RunOptions {
        real_delta: if reel.is_empty() {
            vec![]
        } else {
            argos_script::indicators::flow::real_deltas(bars, reel)
        },
        last_open: now > 0 && tf_seconds > 0 && bars.last().is_some_and(|b| b.time + tf_seconds > now),
        ..Default::default()
    }
}

/// Bleu des lignes d'indicateurs (couleur de l'indicateur, pas du thème).
const BLUE: Rgb = Rgb(0x29, 0x62, 0xff);
const CYAN: Rgb = Rgb(0x3d, 0xc8, 0xf0);
const PURPLE: Rgb = Rgb(0xb3, 0x6b, 0xff);

/// Hauteur d'une bande d'oscillateur.
pub const OSC_H: f64 = 120.0;
/// 📜 Default height of the Argos Vague panel (mockup v7: the wave takes close to half of the plot).
pub const VAGUE_H: f64 = 2.75 * OSC_H;
/// Hauteur d'une bande : de `BAND_MIN` à `BAND_MAX` px.
pub const BAND_MIN: f64 = 50.0;
pub const BAND_MAX: f64 = 600.0;

thread_local! {
    /// 📜 Chosen heights of the user scripts' panes, by script id (chart settings), given by `set_script_heights`.
    static SCRIPT_HEIGHTS: std::cell::RefCell<std::collections::HashMap<String, f64>> = Default::default();
    /// Hauteurs choisies des bandes (réglages de la chart), données par `set_heights` avant chaque dessin.
    static HEIGHTS: std::cell::RefCell<std::collections::HashMap<Ind, f64>> = Default::default();
    /// 🖥 Panneaux repliés (▾ de leur légende), donnés par `set_replies` avant chaque dessin.
    static REPLIES: std::cell::RefCell<Vec<Ind>> = const { std::cell::RefCell::new(Vec::new()) };
    /// 🖥 Ordre choisi des panneaux (↑ ↓ de leur légende), du bas vers le haut ; les autres gardent leur place après.
    static ORDRE: std::cell::RefCell<Vec<Ind>> = const { std::cell::RefCell::new(Vec::new()) };
    /// 🖥 Panneau sous la souris : ses boutons se montrent (maquette : au survol).
    static SURVOL: std::cell::Cell<Option<Ind>> = const { std::cell::Cell::new(None) };
    /// 🖥 Ce que la carte Argos Lens évite (barre de favoris), en coordonnées de la chart.
    static EVITER: std::cell::RefCell<Vec<crate::shapes::Rect>> = const { std::cell::RefCell::new(Vec::new()) };
    /// 📐 Réduction des bandes d'oscillateurs sur un écran bas (1 = taille normale), réglée à chaque dessin.
    static SQUEEZE: std::cell::Cell<f64> = const { std::cell::Cell::new(1.0) };
    /// Tableaux déjà posés pendant ce dessin (un tableau qui en chevaucherait un autre n'est pas dessiné).
    static TABLES: std::cell::RefCell<Vec<crate::shapes::Rect>> = const { std::cell::RefCell::new(Vec::new()) };
    /// 🧱 Argos Murs draws its wall pills on the right of the price pane in this drawing (`set_murs_a_droite`).
    static MURS_DROITE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// 🔍 Places kept free of tables during this drawing (the navigation pill): a table there moves up above it.
    static RESERVE: std::cell::RefCell<Vec<crate::shapes::Rect>> = const { std::cell::RefCell::new(Vec::new()) };
    /// 🔎 Place de la carte Argos Lens au dernier dessin (coordonnées de la chart ; `None` : pas de carte).
    static CARTE_LENS: std::cell::Cell<Option<crate::shapes::Rect>> = const { std::cell::Cell::new(None) };
    /// Vue de chaque bande (décalage en fraction de son échelle, zoom), donnée par `set_views`.
    static VIEWS: std::cell::RefCell<std::collections::HashMap<Ind, (f64, f64)>> = Default::default();
    /// 📜 The view of each script's pane (offset, zoom) by its key ([`crate::scriptview::ScriptView::view_key`]), given
    /// by `set_script_views`.
    static SCRIPT_VIEWS: std::cell::RefCell<std::collections::HashMap<String, (f64, f64)>> = Default::default();
    /// ⛶ Panel shown full screen (`set_plein_ecran`), kept from one drawing to the next (not cleared by `fit_bands`).
    static PLEIN_ECRAN: std::cell::Cell<Option<Ind>> = const { std::cell::Cell::new(None) };
    /// ⛶ Its height: the whole plot height given to the last `fit_bands`.
    static PLEIN_H: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) };
}

/// ⛶ Shows the panel `i` full screen — it alone, the whole plot height, its own scale — or (`None`) every panel as
/// usual. A panel not on the chart (the Lens preview's outputs, a removed indicator) changes nothing.
pub fn set_plein_ecran(i: Option<Ind>) {
    PLEIN_ECRAN.with(|p| p.set(i));
}

/// ⛶ The panel shown full screen, if any.
pub fn plein_ecran() -> Option<Ind> {
    PLEIN_ECRAN.with(|p| p.get())
}

/// 🖥 Hauteur d'un panneau replié (maquette : `.vol.replie`, 22).
pub const BAND_REPLIE: f64 = 22.0;

/// 🖥 Les panneaux repliés (interface-2) : `band_h` et le dessin les lisent.
pub fn set_replies(v: &[Ind]) {
    REPLIES.with(|r| *r.borrow_mut() = v.to_vec());
}

/// 🖥 L'ordre choisi des panneaux (interface-2), du bas vers le haut : `bands` le suit.
pub fn set_ordre(v: &[Ind]) {
    ORDRE.with(|r| *r.borrow_mut() = v.to_vec());
}

/// 🖥 Le panneau sous la souris (ses boutons se montrent dans sa légende).
/// 🖥 Zones que la carte Argos Lens évite (la barre de favoris) : elle se pose dessous (en haut) ou dessus (en bas).
pub fn set_eviter(r: Vec<crate::shapes::Rect>) {
    EVITER.with(|e| *e.borrow_mut() = r);
}

pub(crate) fn eviter() -> Vec<crate::shapes::Rect> {
    EVITER.with(|e| e.borrow().clone())
}

pub fn set_survol(i: Option<Ind>) {
    SURVOL.with(|s| s.set(i));
}

/// 🖥 Le panneau sous la souris.
pub fn survol() -> Option<Ind> {
    SURVOL.with(|s| s.get())
}

/// 🖥 Les panneaux repliés.
pub fn replies() -> Vec<Ind> {
    REPLIES.with(|r| r.borrow().clone())
}

/// Hauteurs choisies des bandes (les réglages de la chart) : `band_h` les lit.
pub fn set_heights(h: &std::collections::HashMap<Ind, f64>) {
    HEIGHTS.with(|m| m.borrow_mut().clone_from(h));
}

/// 📜 Chosen heights of the user scripts' panes (chart settings, by script id): `ScriptView::band_h` reads them.
pub fn set_script_heights(h: &std::collections::HashMap<String, f64>) {
    SCRIPT_HEIGHTS.with(|m| m.borrow_mut().clone_from(h));
}

/// 📜 The chosen height of the script `id`'s pane, if any.
pub fn hauteur_script(id: &str) -> Option<f64> {
    SCRIPT_HEIGHTS.with(|m| m.borrow().get(id).copied()).filter(|_| !id.is_empty())
}

/// 📐 The squeeze of the bands on a low screen (1 = normal height), as set by the last `fit_bands`.
pub fn squeeze() -> f64 {
    SQUEEZE.with(|s| s.get())
}

/// Part la plus haute de la zone de tracé que les bandes d'oscillateurs peuvent prendre : la chart des bougies garde
/// toujours au moins 45 % de la hauteur (tablette en paysage, petite fenêtre).
pub const MAX_BANDS_SHARE: f64 = 0.55;

/// 📋 Puts a table drawn outside the indicators (a user script's `table.new`, step 4) in the same list as the
/// indicator tables: it is not drawn when it would cover one already there (`false`), like them.
#[cfg(test)]
pub(crate) fn poser_tableau(r: crate::shapes::Rect) -> bool {
    let croise = |a: &crate::shapes::Rect| a.x < r.x + r.w && r.x < a.x + a.w && a.y < r.y + r.h && r.y < a.y + a.h;
    TABLES.with(|t| {
        if t.borrow().iter().any(croise) {
            return false;
        }
        t.borrow_mut().push(r);
        true
    })
}

/// 📋 (tableaux-1) The one path every table takes: indicators' tables, the multi-UT table, the Vector Zone watchlist,
/// user scripts' and imported Pine tables. `taille(s)` is the table's size drawn at scale `s` (its layout's width and
/// row stretches already in); `defaut(w, h)` its automatic place (its corner, the walls' mirror rule). Not moved: the
/// automatic place, off the reserved places; (tour 2, F5) when that place is taken (a table already there, Argos
/// Murs' pills, the news row…), the nearest free place at its size. Moved: its own place in `area`, snapped to the
/// nearest place free of the reserved ones, of Argos Murs' pills, of the news row and of the other tables. Either way
/// a smaller scale (down to 0.6) only when no place fits, and not drawn only when nothing fits even then. Returns the
/// rect and the scale to draw at; the rect is listed in [`tableaux`].
pub(crate) fn placer(
    id: &crate::tableaux::Id,
    area: crate::shapes::Rect,
    taille: &dyn Fn(f64) -> (f64, f64),
    defaut: &dyn Fn(f64, f64) -> crate::shapes::Rect,
) -> Option<(crate::shapes::Rect, f64)> {
    use crate::tableaux::{croise, libre, SCALE_MIN};
    let lay = crate::tableaux::layout(&id.key);
    let reserve: Vec<crate::shapes::Rect> = RESERVE.with(|t| t.borrow().clone());
    let poses: Vec<crate::shapes::Rect> = TABLES.with(|t| t.borrow().clone());
    let obst: Vec<_> = reserve.iter().chain(&poses).copied().chain(crate::tableaux::obstacles()).collect();
    let finir = |r: crate::shapes::Rect, s: f64| {
        TABLES.with(|t| t.borrow_mut().push(r));
        crate::tableaux::poser(id, r, area, s);
        Some((r, s))
    };
    // (the scales tried: its own, then smaller by tenths down to 0.6)
    let echelles: Vec<f64> = std::iter::once(lay.scale)
        .chain((1..=14).map(|k| lay.scale * (1.0 - 0.1 * k as f64)).filter(|s| *s >= SCALE_MIN - 1e-9))
        .chain((lay.scale > SCALE_MIN).then_some(SCALE_MIN))
        .collect();
    if !lay.moved {
        for &s in &echelles {
            let (w, h) = taille(s);
            let d = defaut(w, h);
            let r = hors_reserve(d, area.y).unwrap_or(d);
            let pris = |r: &crate::shapes::Rect| r.y < area.y - 0.01 || r.y + r.h > area.y + area.h + 0.01 || obst.iter().any(|a| croise(a, r));
            if !pris(&r) {
                return finir(r, s);
            }
            // (its place taken — T6: the legend's list unfolded on the cell; F5: another table, the walls' pills —:
            // the nearest free place at this size)
            if let Some(r) = libre(r, area, &obst) {
                return finir(r, s);
            }
        }
        return None;
    }
    for &s in &echelles {
        let (w, h) = taille(s);
        if let Some(r) = libre(lay.place(area, w, h), area, &obst) {
            return finir(r, s);
        }
    }
    // (nowhere free even smaller: its own place, never over another table)
    let (w, h) = taille(lay.scale);
    let r = lay.place(area, w, h);
    if poses.iter().any(|a| croise(a, &r)) {
        return None;
    }
    finir(r, lay.scale)
}

/// 📋 (tour 2, F6) The reserved places of this drawing (the legend's lines, the floating bar, Argos Murs' bottom
/// pills, the hint pill…): the table's menu opens off them.
pub(crate) fn reserves() -> Vec<crate::shapes::Rect> {
    RESERVE.with(|t| t.borrow().clone())
}

/// 🧪 A new drawing for the tests that place tables by hand: no table, no reserved place, no obstacle.
#[cfg(test)]
pub(crate) fn nouveau_dessin_test() {
    TABLES.with(|t| t.borrow_mut().clear());
    RESERVE.with(|t| t.borrow_mut().clear());
    crate::tableaux::nouveau_dessin();
}

/// 📋 Places des tableaux des indicateurs (Divergence, AI Trend, Golden Dot…) au dernier dessin (coordonnées de la
/// chart) : la souris y a la flèche, comme sur les légendes.
pub fn tableaux() -> Vec<crate::shapes::Rect> {
    TABLES.with(|t| t.borrow().clone())
}

/// 🔍 Keeps `r` free of indicator tables during this drawing (the navigation pill): only a table that really covers
/// it moves up, just enough (it is not one of the `tableaux`).
pub(crate) fn reserver(r: crate::shapes::Rect) {
    RESERVE.with(|t| t.borrow_mut().push(r));
}

/// 🔍 `r` moved off the reserved places it covers: just above one lower than it (the navigation pill, Argos Murs'
/// bottom pills), just below one higher than it (the legend's lines, its open list); `None`: no room left above
/// `min_y`.
pub(crate) fn hors_reserve(mut r: crate::shapes::Rect, min_y: f64) -> Option<crate::shapes::Rect> {
    let croise = |a: &crate::shapes::Rect, r: &crate::shapes::Rect| a.x < r.x + r.w && r.x < a.x + a.w && a.y < r.y + r.h && r.y < a.y + a.h;
    RESERVE.with(|t| {
        let t = t.borrow();
        for _ in 0..4 {
            let Some(a) = t.iter().find(|a| croise(a, &r)) else { break };
            r.y = if a.cy() < r.cy() { a.y + a.h + 4.0 } else { a.y - r.h - 4.0 };
        }
    });
    (r.y >= min_y).then_some(r)
}

/// 🧱 Argos Murs shows its wall pills on the right of the price pane in this drawing (or not): see
/// [`table_corner_with_walls`].
pub(crate) fn set_murs_a_droite(on: bool) {
    MURS_DROITE.with(|m| m.set(on));
}

/// 📋🧱 The corner a price-pane table really takes (lot pilules-1, coordinator's rule A of 22 Nov): while Argos Murs
/// shows its pills on the right, a table set at a right corner (top, middle, bottom right: `pos` 0, 3, 5) goes to the
/// mirror left corner (1, 4, 6) and the walls keep the right side; otherwise its own corner. Indicator tables
/// ([`tables::table`]), user scripts' tables (`scriptview`) and the multi-UT table all ask here: another rule is a
/// change of this function only.
pub(crate) fn table_corner_with_walls(pos: usize) -> usize {
    match (MURS_DROITE.with(|m| m.get()), pos) {
        (true, 0) => 1,
        (true, 3) => 4,
        (true, 5) => 6,
        _ => pos,
    }
}

/// 🔎 Place de la carte Argos Lens au dernier dessin (coordonnées de la chart), pour vérifier qu'elle tient entière.
pub fn carte_lens() -> Option<crate::shapes::Rect> {
    CARTE_LENS.with(|c| c.get())
}

/// 📐 Ajuste les bandes à la hauteur de la zone de tracé `plot_h` : si leur hauteur normale dépasse
/// [`MAX_BANDS_SHARE`], toutes rétrécissent dans la même proportion (s'ajuste tout seul à la taille de l'écran).
pub fn fit_bands(out: &IndOut, plot_h: f64) {
    TABLES.with(|t| t.borrow_mut().clear()); // nouveau dessin : aucun tableau posé
    crate::tableaux::nouveau_dessin();
    RESERVE.with(|t| t.borrow_mut().clear());
    CARTE_LENS.with(|c| c.set(None));
    SQUEEZE.with(|s| s.set(1.0));
    PLEIN_H.with(|h| h.set(plot_h));
    let need = out.osc_h();
    let room = plot_h * MAX_BANDS_SHARE;
    SQUEEZE.with(|s| s.set(if need > room && need > 0.0 { (room / need).max(0.2) } else { 1.0 }));
}

/// ↕️ How far from a pane's top edge it is still grabbed: 5 points with the mouse, 👆 14 by the finger (Jee, 24 Nov:
/// the line between the chart and a pane could not be caught on the iPhone).
pub fn edge_reach() -> f64 {
    if crate::actions::tactile() {
        14.0
    } else {
        5.0
    }
}

/// Vues des bandes (glisser dans une bande = la monter / descendre, glisser sur sa colonne de valeurs = zoom), et 📜
/// celles des volets des scripts, par clé (`ScriptView::view_key`).
pub fn set_views(v: &std::collections::HashMap<Ind, (f64, f64)>, scripts: &std::collections::HashMap<String, (f64, f64)>) {
    VIEWS.with(|m| m.borrow_mut().clone_from(v));
    SCRIPT_VIEWS.with(|m| m.borrow_mut().clone_from(scripts));
}

/// The view (offset, zoom) of the band `ind`; none: (0, 1).
pub fn band_view_of(ind: Ind) -> (f64, f64) {
    VIEWS.with(|m| m.borrow().get(&ind).copied()).unwrap_or((0.0, 1.0))
}

/// 📜 The view (offset, zoom) of a script's pane, by its key; none: (0, 1).
pub fn script_view_of(key: &str) -> (f64, f64) {
    SCRIPT_VIEWS.with(|m| m.borrow().get(key).copied()).unwrap_or((0.0, 1.0))
}

/// The scale `lo..hi` of a pane with the view `(offset, zoom)` applied.
pub fn apply_view((off, z): (f64, f64), lo: f64, hi: f64) -> (f64, f64) {
    let (mid, half) = ((lo + hi) / 2.0 + off * (hi - lo), (hi - lo) / 2.0 * z.clamp(0.05, 20.0));
    (mid - half, mid + half)
}

/// Échelle `lo..hi` d'une bande, avec sa vue (décalage, zoom) appliquée.
fn view(ind: Ind, lo: f64, hi: f64) -> (f64, f64) {
    apply_view(band_view_of(ind), lo, hi)
}
/// Taille du nom en haut d'une bande (la légende place ses boutons juste après).
pub const BAND_FONT: f32 = 12.5;

/// At most so many bars drawn after the last one (Ichimoku's 25, a script's `offset`), whatever the zoom.
const AHEAD_MAX: usize = 500;

/// Où dessiner : bougies visibles et passage index / prix → pixels.
pub struct Geo<'a> {
    pub vp: &'a Viewport,
    pub scale: &'a PriceScale,
    pub slots: &'a Slots,
    pub first: usize,
    pub last: usize,
    pub bars: &'a [Bar],
    /// Bas de la zone de prix (au-dessus des bandes d'oscillateurs).
    pub price_h: f64,
    /// Décalage du fuseau de la chart (secondes), pour les heures écrites sur la chart.
    pub tz_offset_s: i64,
}

impl Geo<'_> {
    pub(crate) fn x(&self, i: usize) -> f64 {
        self.vp.x_of(i, self.slots)
    }
    pub(crate) fn y(&self, p: f64) -> f64 {
        self.scale.y_of(p)
    }
    /// Place de la bougie `i` (x gauche, x droit) : jusqu'à mi-chemin des voisines. Avec les bougies de volume, les
    /// largeurs changent d'une bougie à l'autre : un fond (bgcolor) par bougie reste ainsi collé, sans trou.
    pub(crate) fn slot(&self, i: usize) -> (f64, f64) {
        let x = self.x(i);
        let n = self.bars.len();
        let left = (i > 0).then(|| (x - self.x(i - 1)) / 2.0);
        let right = (i + 1 < n).then(|| (self.x(i + 1) - x) / 2.0);
        let (l, r) = match (left, right) {
            (Some(l), Some(r)) => (l, r),
            (Some(l), None) => (l, l),
            (None, Some(r)) => (r, r),
            (None, None) => (3.0, 3.0),
        };
        (x - l, x + r)
    }
    /// x d'une position de bougie fractionnaire (aussi avant la première ou après la dernière bougie chargée).
    pub(crate) fn xf(&self, i: f64) -> f64 {
        let n = self.bars.len();
        if n < 2 {
            return self.x(0) + i * 6.0;
        }
        let last = (n - 1) as f64;
        let step = |a: usize| self.x(a + 1) - self.x(a);
        if i <= 0.0 {
            self.x(0) + i * step(0)
        } else if i >= last {
            self.x(n - 1) + (i - last) * step(n - 2)
        } else {
            let k = i.floor() as usize;
            self.x(k) + (i - k as f64) * step(k)
        }
    }
    /// How many bars after the last one have their place on the screen, the last one shown (0 when the last bar
    /// is not): TradingView draws Ichimoku's cloud and a script's plots shifted ahead there (B-045, B-046).
    pub(crate) fn apres(&self) -> usize {
        let n = self.bars.len();
        if n < 2 || self.last + 1 < n {
            return 0;
        }
        (0..AHEAD_MAX).take_while(|&k| self.xf((n + k) as f64) <= self.right()).count()
    }
    /// Zone de prix (pour placer un tableau).
    fn price_area(&self) -> crate::shapes::Rect {
        crate::shapes::Rect::new(0.0, 0.0, self.right(), self.price_h)
    }
    /// Bord droit de la zone de prix.
    pub(crate) fn right(&self) -> f64 {
        self.vp.plot_w()
    }
}

fn seg(pm: &mut Pixmap, a: (f64, f64), b: (f64, f64), c: Rgb, w: f32) {
    lines(pm, 0.0, 0.0, &[&[(a.0 as f32, a.1 as f32), (b.0 as f32, b.1 as f32)]], c, w);
}

/// ⚡ Courbe colorée bougie par bougie : les segments voisins de même couleur sont tracés d'un seul trait (un trait
/// par couleur et par morceau au lieu d'un par bougie : bien plus rapide pendant un glissé, même image).
/// `pts[k]` = point de la bougie, `col(k)` = couleur du segment qui arrive à ce point (`None` : pas de segment).
pub(super) fn seg_couleurs(pm: &mut Pixmap, pts: &[(f64, f64)], col: &dyn Fn(usize) -> Option<Rgb>, w: f32) {
    let mut run: Vec<(f32, f32)> = vec![];
    let mut cur: Option<Rgb> = None;
    let flush = |pm: &mut Pixmap, run: &mut Vec<(f32, f32)>, c: Option<Rgb>| {
        if let (Some(c), true) = (c, run.len() > 1) {
            lines(pm, 0.0, 0.0, &[run], c, w);
        }
        run.clear();
    };
    for k in 1..pts.len() {
        let (a, b) = (pts[k - 1], pts[k]);
        let c = col(k).filter(|_| a.1.is_finite() && b.1.is_finite());
        if c != cur || c.is_none() {
            flush(pm, &mut run, cur);
            cur = c;
            if c.is_some() {
                run.push((a.0 as f32, a.1 as f32));
            }
        }
        if c.is_some() {
            run.push((b.0 as f32, b.1 as f32));
        }
    }
    flush(pm, &mut run, cur);
}

fn poly(pm: &mut Pixmap, pts: &[(f64, f64)], c: Rgb, w: f32) {
    poly_trait(pm, pts, c, w, 0);
}

/// Ligne d'une sortie avec son type (onglet Style) : 0 plein, 1 tirets, 2 points.
pub(super) fn poly_trait(pm: &mut Pixmap, pts: &[(f64, f64)], c: Rgb, w: f32, dash: usize) {
    let p = crate::dense_line(pts.iter().copied().filter(|(_, y)| y.is_finite()));
    if p.len() < 2 {
        return;
    }
    let (on, off) = match dash {
        1 => (7.0, 5.0),
        2 => (1.5 + w as f64 * 0.5, 3.5),
        _ => return lines(pm, 0.0, 0.0, &[&p], c, w),
    };
    let pts: Vec<(f64, f64)> = p.iter().map(|&(x, y)| (x as f64, y as f64)).collect();
    tirets(pm, &pts, on, off, c, w, 1.0);
}

/// ⚡ Trait en tirets tracé d'un seul coup : tous les tirets dans un seul tracé (avant : un tracé par tiret, des
/// centaines par image pour une ligne de niveau sur toute la largeur). Même image.
pub(crate) fn tirets(pm: &mut Pixmap, pts: &[(f64, f64)], on: f64, off: f64, c: Rgb, w: f32, alpha: f32) {
    let parts: Vec<Vec<(f32, f32)>> = crate::toolgeom::dashes(pts, on, off)
        .into_iter()
        .map(|d| d.iter().map(|z| (z.0 as f32, z.1 as f32)).collect::<Vec<_>>())
        .filter(|q| q.len() > 1)
        .collect();
    let refs: Vec<&[(f32, f32)]> = parts.iter().map(Vec::as_slice).collect();
    crate::shapes::lines_alpha(pm, 0.0, 0.0, &refs, c, w, alpha);
}

/// Ligne de divergence : `style` 0 pointillé, 1 tirets, 2 plein (input « Style des divergences cachées »).
fn div_seg(pm: &mut Pixmap, a: (f64, f64), b: (f64, f64), c: Rgb, w: f32, style: usize) {
    let (on, off) = match style {
        0 => (2.0, 3.0),
        1 => (7.0, 5.0),
        _ => return seg(pm, a, b, c, w),
    };
    tirets(pm, &[a, b], on, off, c, w, 1.0);
}

/// Nuage entre deux lignes sur le prix, continu comme le `fill` du Pine : une tranche par paire de bougies voisines, de
/// centre à centre (pas de trous avec les bougies de volume, dont les largeurs changent), à la couleur de la bougie de
/// droite. `at(i)` = (ligne a, ligne b, couleur et opacité ; `None` = pas de nuage).
fn cloud(pm: &mut Pixmap, g: &Geo, at: &dyn Fn(usize) -> (f64, f64, Option<(Rgb, f32)>)) {
    cloud_to(pm, g, g.last.min(g.bars.len().saturating_sub(1)), at);
}

/// [`cloud`] up to the bar `fin`, which may be after the last bar (`at` gives the values there: Ichimoku's cloud
/// ahead, B-045).
fn cloud_to(pm: &mut Pixmap, g: &Geo, fin: usize, at: &dyn Fn(usize) -> (f64, f64, Option<(Rgb, f32)>)) {
    // ⚡ un seul polygone par morceau de même couleur (ligne a à l'aller, ligne b au retour) au lieu d'un par paire de
    // bougies : même image (remplissage sans anticrénelage), bien moins de tracés pendant un glissé
    let n = g.bars.len();
    let mut run: Vec<(f32, f32, f32)> = vec![];
    let mut cur: Option<(Rgb, f32)> = None;
    let flush = |pm: &mut Pixmap, run: &mut Vec<(f32, f32, f32)>, c: Option<(Rgb, f32)>| {
        if let (Some((c, alpha)), true) = (c, run.len() > 1) {
            let mut pb = tiny_skia::PathBuilder::new();
            pb.move_to(run[0].0, run[0].1);
            for p in &run[1..] {
                pb.line_to(p.0, p.1);
            }
            for p in run.iter().rev() {
                pb.line_to(p.0, p.2);
            }
            pb.close();
            if let Some(path) = pb.finish() {
                let mut paint = crate::paint(c, alpha);
                paint.anti_alias = false;
                pm.fill_path(&path, &paint, tiny_skia::FillRule::Winding, crate::px::t(), None);
            }
        }
        run.clear();
    };
    let x = |i: usize| if i < n { g.x(i) } else { g.xf(i as f64) };
    let pt = |i: usize, a: f64, b: f64| (x(i) as f32, g.y(a) as f32, g.y(b) as f32);
    for i in g.first.max(1)..=fin {
        let ((a0, b0, _), (a1, b1, c)) = (at(i - 1), at(i));
        let c = c.filter(|_| [a0, b0, a1, b1].iter().all(|v| v.is_finite()));
        if c != cur || c.is_none() {
            flush(pm, &mut run, cur);
            cur = c;
            if c.is_some() {
                run.push(pt(i - 1, a0, b0));
            }
        }
        if c.is_some() {
            run.push(pt(i, a1, b1));
        }
    }
    flush(pm, &mut run, cur);
}

fn dot(pm: &mut Pixmap, x: f64, y: f64, r: f64, c: Rgb) {
    crate::shapes::round_fill(pm, crate::shapes::Rect::new(x - r, y - r, 2.0 * r, 2.0 * r), r, c, 1.0);
}

/// Boîte de prix de la bougie `from` à `to` (ou jusqu'au bord droit).
fn zone(pm: &mut Pixmap, g: &Geo, from: usize, to: Option<usize>, top: f64, bot: f64, c: Rgb, alpha: f32) {
    if to.is_some_and(|t| t < g.first) || from > g.last {
        return;
    }
    let x0 = g.x(from).max(0.0);
    let x1 = to.map_or(g.right(), |t| g.x(t)).min(g.right());
    let (y0, y1) = (g.y(top).min(g.y(bot)), g.y(top).max(g.y(bot)));
    if x1 > x0 {
        crate::fill(pm, x0, y0, x1 - x0, (y1 - y0).max(1.0), c, alpha);
        crate::fill(pm, x0, y0, x1 - x0, 1.0, c, alpha * 3.0);
        crate::fill(pm, x0, y1, x1 - x0, 1.0, c, alpha * 3.0);
    }
}

/// Dessine les indicateurs sur le prix, puis les bandes des oscillateurs en bas de la zone de prix.
pub fn draw(pm: &mut Pixmap, text: &Text, g: &Geo, out: &IndOut) -> Vec<BandAxis> {
    let mut axes = vec![];
    // 🌊 Argos Vague's pills: laid out again by its band below, drawn by `draw_axes`
    crate::scriptview::vague_pills_reset();
    // 🔎 Argos Lens d'abord : les autres tableaux évitent sa carte
    if let Some(l) = out.lens.as_ref().filter(|_| out.shown(Ind::Lens)) {
        lens::draw_reperes(pm, text, g, l);
        let biais = argos_lens::reglages::nom_biais(&values(&out.inputs, Ind::Lens));
        let carte = argos_lens::reglages::carte(&values(&out.inputs, Ind::Lens));
        if let Some(r) = lens::draw_carte(pm, text, l, g, biais, crate::indleg::bas_liste(out) + 8.0, carte) {
            TABLES.with(|t| t.borrow_mut().push(r));
            CARTE_LENS.with(|c| c.set(Some(r)));
        }
    }
    if let Some(s) = out.smc.as_ref().filter(|_| out.shown(Ind::Smc)) {
        draw_smc(pm, text, g, s, &values(&out.inputs, Ind::Smc));
    }
    if let Some(s) = out.smc_pro.as_ref().filter(|_| out.shown(Ind::SmcPro)) {
        draw_smc_pro(pm, text, g, s, &values(&out.inputs, Ind::SmcPro));
    }
    // 📍 Argos Pivot VWAP : une échelle, ou multi-échelle (Multi)
    for (r, ind, colors) in [
        (&out.swing, Ind::SwingVwap, (crate::palette::up(), crate::palette::down())),
        (&out.smart, Ind::SmartSwingVwap, (CYAN, PURPLE)),
    ] {
        let Some(r) = r.as_ref().filter(|_| out.shown(ind)) else { continue };
        draw_pivot_vwap(pm, text, g, r, colors);
    }
    if let Some(t) = out.tvd.as_ref().filter(|_| out.shown(Ind::Tvd)) {
        draw_tvd(pm, text, g, t, &values(&out.inputs, Ind::Tvd));
    }
    if let Some(a) = out.ai.as_ref().filter(|_| out.shown(Ind::AiTrend)) {
        draw_ai_chart(pm, text, g, &a.volume, &values(&out.inputs, Ind::AiTrend));
    }
    // TBO sur le prix (options de v1) : trendlines et divergences ; le reste est dans sa bande
    if let Some(t) = out.tbo.as_ref().filter(|_| out.shown(Ind::Tbo)) {
        let vals = values(&out.inputs, Ind::Tbo);
        let p = argos_script::inputs::Inputs {
            params: &tbo::PARAMS,
            vals: &vals,
        };
        if p.on("tl_chart") {
            for tl in t
                .core
                .trendlines
                .iter()
                .filter(|tl| tl.i1 >= g.first && tl.i0 <= g.last && g.bars.len().saturating_sub(tl.i1) < 400)
            {
                let c = if tl.upper { hex(0xef5350) } else { hex(0x26a69a) };
                seg(pm, (g.x(tl.i0), g.y(tl.p0)), (g.x(tl.i1), g.y(tl.p1)), c, 1.2);
            }
        }
        if p.on("div_on") && p.on("div_chart") {
            for d in t.divs.iter().filter(|d| d.pivot >= g.first && d.prev <= g.last) {
                let (a, b) = (&g.bars[d.prev], &g.bars[d.pivot]);
                let (p0, p1) = if d.bull { (a.low, b.low) } else { (a.high, b.high) };
                let Rgb(r, gg, bb) = crate::palette::on_chart(p.rgb(if d.bull { "cDivU" } else { "cDivD" }));
                let style = if d.hidden { p.pick("hid_style") } else { 2 };
                div_seg(pm, (g.x(d.prev), g.y(p0)), (g.x(d.pivot), g.y(p1)), Rgb(r, gg, bb), 1.2, style);
            }
        }
    }
    for pl in out.plots.iter().filter(|p| !p.sub && out.shown(p.ind)) {
        let n = g.bars.len();
        // the bars after the last one that have their place on the screen, for the values ahead (B-045)
        let devant = match pl.ahead.iter().map(Vec::len).max() {
            Some(m) if m > 0 => g.apres().min(m),
            _ => 0,
        };
        let at = |k: usize, i: usize| match i.checked_sub(n) {
            None => pl.lines[k].0.get(i).copied().unwrap_or(f64::NAN),
            Some(j) => pl.ahead.get(k).and_then(|f| f.get(j)).copied().unwrap_or(f64::NAN),
        };
        if let Some((a, b, c, alpha)) = pl.fill {
            let teinte = |i: usize| match pl.fill_down {
                Some(d) if at(a, i) < at(b, i) => d,
                _ => c,
            };
            let fin = g.last.min(n.saturating_sub(1)) + devant;
            cloud_to(pm, g, fin, &|i| (at(a, i), at(b, i), Some((teinte(i), alpha))));
        }
        for (k, (v, c, w)) in pl.lines.iter().enumerate() {
            let devant = if pl.ahead.get(k).is_some_and(|f| !f.is_empty()) { devant } else { 0 };
            poly_trait(
                pm,
                &(g.first..=g.last)
                    .map(|i| (g.x(i), g.y(v.get(i).copied().unwrap_or(f64::NAN))))
                    .chain((n..n + devant).map(|i| (g.xf(i as f64), g.y(at(k, i)))))
                    .collect::<Vec<_>>(),
                crate::palette::on_chart(*c),
                *w,
                pl.dash.get(k).copied().unwrap_or(0),
            );
        }
        for &(i, below, c, label) in pl.marks.iter().filter(|m| g.vp.bar_spacing >= 2.0 && (g.first..=g.last).contains(&m.0)) {
            let b = &g.bars[i];
            let y = if below { g.y(b.low) + 8.0 } else { g.y(b.high) - 8.0 };
            dot(pm, g.x(i), y, 3.0, c);
            text.draw(
                pm,
                label,
                (g.x(i) as f32, (if below { y + 14.0 } else { y - 6.0 }) as f32),
                10.0,
                c,
                Align::Center,
            );
        }
    }
    if let Some(c) = out.cipher.as_ref().filter(|_| out.shown(Ind::Cipher)) {
        draw_confluence(pm, text, g, c, &values(&out.inputs, Ind::Cipher));
    }
    if let Some(p) = out.profile.as_ref().filter(|_| out.shown(Ind::Profile)) {
        let vals = values(&out.inputs, Ind::Profile);
        draw_profile(pm, text, g, p, &vals);
        let inp = argos_script::inputs::Inputs {
            params: &v1::PROFILE_PARAMS,
            vals: &vals,
        };
        // (the shared rows are always computed: « Tableau résumé » is checked here, as for the other tables)
        let rows = if inp.on("showTable") { profile_tableau(p, g.bars) } else { vec![] };
        if !rows.is_empty() {
            table(pm, text, g.price_area(), Ind::Profile.name(), &rows, &inp, "", Ind::Profile);
        }
    }
    if let Some(a) = out.avwap.as_ref().filter(|_| out.shown(Ind::AnchoredVwap)) {
        draw_avwap(pm, g, a, &values(&out.inputs, Ind::AnchoredVwap));
    }
    if let Some(p) = out.pa.as_ref().filter(|_| out.shown(Ind::PriceAction)) {
        draw_pa(pm, text, g, p, &values(&out.inputs, Ind::PriceAction));
    }
    if let Some(list) = out.pat.as_ref().filter(|_| out.shown(Ind::Patterns)) {
        draw_patterns(pm, text, g, list, &values(&out.inputs, Ind::Patterns));
    }
    if let Some(rows) = out.mtf.as_ref().filter(|_| out.shown(Ind::MtfTable)) {
        draw_mtf_table(pm, text, g, rows);
    }
    if let Some(z) = out.vz.as_ref().filter(|_| out.shown(Ind::VectorZone)) {
        draw_vector_zone(pm, text, g, z, &values(&out.inputs, Ind::VectorZone), out.texts.get(&Ind::VectorZone));
    }
    if let Some(f) = out.footprint.as_ref().filter(|_| out.shown(Ind::Footprint)) {
        let vals = values(&out.inputs, Ind::Footprint);
        draw_footprint(pm, text, g, f, &vals);
        if let Some(m) = &out.fp_multi {
            draw_fp_parts(pm, g, f, m, &vals);
        }
    }
    if let Some(b) = out.bubbles.as_ref().filter(|_| out.shown(Ind::Bubbles)) {
        draw_bubbles(pm, text, g, b, &values(&out.inputs, Ind::Bubbles));
    }
    if let Some(t) = out.tape.as_ref().filter(|_| out.shown(Ind::Tape)) {
        draw_tape(pm, text, g, t, &values(&out.inputs, Ind::Tape));
    }
    // 📜 scripts : sur le prix, ou dans leur bande au-dessus de celles des oscillateurs
    for v in out.scripts.iter().filter(|s| s.out.overlay) {
        let _ = crate::scriptview::draw(pm, text, g, v, None);
    }
    // 📜 built-in indicators written in Argos Script, on the price (the others are in their band below)
    for (_, v) in out.integres.iter().filter(|(i, v)| v.out.overlay && out.shown(*i)) {
        let _ = crate::scriptview::draw(pm, text, g, v, None);
    }
    // 📌 `force_overlay = true` marks of the scripts in a band: on the price
    for v in out.scripts.iter() {
        crate::scriptview::draw_forced(pm, text, g, v);
    }
    for (_, v) in out.integres.iter().filter(|(i, _)| out.shown(*i)) {
        crate::scriptview::draw_forced(pm, text, g, v);
    }
    for (k, top) in out.script_bands(g.vp.plot_h()) {
        // (a script's pane has its numbers on the price column, as a native pane)
        let v = &out.scripts[k];
        let h = v.band_h();
        axes.extend(
            dans_sa_bande(pm, g, top, h, |lp| {
                crate::scriptview::draw_view(lp, text, g, v, Some((top, h)), script_view_of(&v.view_key()))
            })
            .flatten(),
        );
    }
    // 🧪 the strategies' fills: on the price, even for a strategy in its own band (like TradingView)
    for v in &out.scripts {
        crate::scriptview::draw_ordres(pm, text, g, v);
    }
    // oscillateurs : une bande chacun, collées en bas de la zone de prix (masqué : bande vide avec son en-tête)
    for (ind, top) in out.bands(g.vp.plot_h()) {
        band(pm, g, top, ind);
        // (replié : son en-tête seul)
        if !out.shown(ind) || ind.replie() {
            continue;
        }
        // 📜 a built-in indicator written in Argos Script: its script's drawing, in its band (its own scale)
        if let Some((_, v)) = out.integres.iter().find(|(i, _)| *i == ind) {
            // (its view — dragging in it, on its value column — applied as on a native band: Argos Vague, 24 Nov;
            // cut to its band like the others: dragged, it never spills over the price, Jee 24 Nov « les indicateurs
            // doivent rester dans leur boîte »)
            let h = ind.band_h();
            axes.extend(
                dans_sa_bande(pm, g, top, h, |lp| {
                    crate::scriptview::draw_view(lp, text, g, v, Some((top, h)), band_view_of(ind))
                })
                .flatten(),
            );
            continue;
        }
        // dessinée sur une couche à part puis coupée à sa bande : décalée ou zoomée, elle ne déborde pas chez les autres
        axes.extend(
            dans_sa_bande(pm, g, top, ind.band_h(), |lp| match ind {
                Ind::AiTrend => draw_ai(lp, text, g, top, out.ai.as_ref(), &values(&out.inputs, Ind::AiTrend)),
                Ind::Cipher => draw_cipher(lp, text, g, top, out.cipher.as_ref(), &values(&out.inputs, Ind::Cipher)),
                Ind::Tbo => draw_tbo(lp, text, g, top, out.tbo.as_ref(), &values(&out.inputs, Ind::Tbo)),
                _ => out.plots.iter().find(|p| p.ind == ind).map(|pl| draw_sub(lp, g, top, pl)),
            })
            .flatten(),
        );
    }
    // 📋 summary tables of the simple indicators (Heatmap, Bands, Bollinger Oscillator): on the chart, or in the
    // indicator's band (« Tableau : afficher dans »), drawn once the bands are done so that nothing covers them
    for pl in out.plots.iter().filter(|p| !p.tableau.is_empty() && out.shown(p.ind)) {
        let vals = values(&out.inputs, pl.ind);
        let inp = argos_script::inputs::Inputs {
            params: pl.ind.params().1,
            vals: &vals,
        };
        let in_band = inp.params.iter().any(|p| p.key == "tblIn") && inp.pick("tblIn") == 1;
        let band = out.bands(g.vp.plot_h()).into_iter().find(|(i, _)| *i == pl.ind);
        let area = match band {
            Some((i, top)) if in_band => {
                if i.replie() {
                    continue;
                }
                crate::shapes::Rect::new(0.0, top, g.right(), i.band_h())
            }
            _ => g.price_area(),
        };
        table(pm, text, area, pl.name, &pl.tableau, &inp, "", pl.ind);
    }
    axes
}

/// Draws a band (`top`, `h`) with `f` on a layer of its own, then brings back only the band (x from 0 to the chart's
/// right edge: the price column stays as it is): shifted or zoomed, it never spills over the other bands or the price.
/// ⚡ Fast while dragging: the layer (the size of the screen) is kept from one image to the next; it gets a plain copy
/// of the band's rows (background and header already drawn), the indicator is drawn on it, and the band's area comes
/// back to the screen by copying rows. Before: a new screen per band and per image, then a paste mixing the colours
/// (1 to 2.5 ms per band). `None`: the band is off the screen.
fn dans_sa_bande<R>(pm: &mut Pixmap, g: &Geo, top: f64, h: f64, f: impl FnOnce(&mut Pixmap) -> R) -> Option<R> {
    let clip = crate::px::int_rect(0.0, top.max(0.0), g.right().max(1.0), h.max(1.0))?;
    let reprise = COUCHE
        .with(|c| c.borrow_mut().take())
        .filter(|l| (l.width(), l.height()) == (pm.width(), pm.height()));
    let mut layer = reprise.or_else(|| Pixmap::new(pm.width(), pm.height()))?;
    let stride = layer.width() as usize * 4;
    let (y0, y1) = (
        clip.y().max(0) as usize,
        (clip.y().max(0) as usize + clip.height() as usize).min(layer.height() as usize),
    );
    if y0 >= y1 {
        COUCHE.with(|c| *c.borrow_mut() = Some(layer));
        return None;
    }
    layer.data_mut()[y0 * stride..y1 * stride].copy_from_slice(&pm.data()[y0 * stride..y1 * stride]);
    let r = f(&mut layer);
    // retour de la zone de la bande (x de 0 au bord droit de la chart : l'échelle de prix à droite reste intacte)
    let (x0, x1) = (
        clip.x().max(0) as usize * 4,
        ((clip.x().max(0) as usize + clip.width() as usize) * 4).min(stride),
    );
    if x0 < x1 {
        let (src, dst) = (layer.data(), pm.data_mut());
        for row in y0..y1 {
            let r = row * stride;
            dst[r + x0..r + x1].copy_from_slice(&src[r + x0..r + x1]);
        }
    }
    COUCHE.with(|c| *c.borrow_mut() = Some(layer));
    Some(r)
}

thread_local! {
    /// ⚡ Couche de dessin des bandes d'oscillateurs, gardée entre les images (voir `draw`).
    static COUCHE: std::cell::RefCell<Option<Pixmap>> = const { std::cell::RefCell::new(None) };
}

/// Échelle d'une bande d'oscillateur : ses valeurs sur la colonne de prix (comme les panneaux de v1).
#[derive(Debug, Clone, PartialEq)]
pub struct BandAxis {
    pub top: f64,
    pub h: f64,
    /// Marges du haut (en-tête) et du bas de la bande.
    pub pad: (f64, f64),
    pub lo: f64,
    pub hi: f64,
    /// Dernières valeurs (valeur, couleur) : une étiquette de couleur chacune.
    pub last: Vec<(f64, Rgb)>,
}

impl BandAxis {
    fn y(&self, v: f64) -> f64 {
        self.top + self.pad.0 + (self.hi - v) / (self.hi - self.lo).max(1e-12) * (self.h - self.pad.0 - self.pad.1)
    }

    /// Valeur à la hauteur `y` (réticule dans la bande).
    pub fn value_at(&self, y: f64) -> f64 {
        self.hi - (y - self.top - self.pad.0) / (self.h - self.pad.0 - self.pad.1).max(1.0) * (self.hi - self.lo)
    }

    /// `y` est-il dans cette bande ?
    pub fn contains(&self, y: f64) -> bool {
        (self.top..self.top + self.h).contains(&y)
    }
}

/// Valeurs des bandes sur la colonne de prix : quelques repères gris et la dernière valeur de chaque ligne en couleur.
pub fn draw_axes(pm: &mut Pixmap, text: &Text, g: &Geo, axes: &[BandAxis], bg: Rgb) {
    let (x0, w) = (g.right(), g.vp.price_axis as f64);
    for a in axes.iter().filter(|a| a.hi > a.lo) {
        crate::fill(pm, x0, a.top, w, a.h, bg, 1.0);
        crate::fill(pm, x0, a.top, w, 1.0, pal::edge_soft(), 1.0);
        let step = crate::viewport::nice_step((a.hi - a.lo) / (a.h / 28.0).max(1.0));
        let dec = crate::viewport::decimals_for(step);
        let mut v = (a.lo / step).ceil() * step;
        while v <= a.hi + 1e-9 {
            let y = a.y(v);
            if y > a.top + 12.0 && y < a.top + a.h - 6.0 {
                text.draw(
                    pm,
                    &crate::viewport::format_number(v, dec),
                    ((x0 + 8.0) as f32, (y + 4.0) as f32),
                    11.0,
                    if crate::palette::luminance(bg) > 0.5 {
                        crate::palette::readable_on(GREY, bg, 4.5)
                    } else {
                        GREY
                    },
                    Align::Left,
                );
            }
            v += step;
        }
        for &(v, c) in &a.last {
            let y = a.y(v.clamp(a.lo, a.hi)).clamp(a.top + 10.0, a.top + a.h - 10.0).round();
            crate::fill(pm, x0, y - 9.0, w, 18.0, c, 1.0);
            // texte foncé ou blanc, le plus contrasté sur la couleur de la ligne
            let ink = crate::palette::ink_on(c);
            text.draw(
                pm,
                &crate::viewport::format_number(v, if v.abs() >= 100.0 { 1 } else { 2 }),
                ((x0 + 8.0) as f32, (y + 4.0) as f32),
                11.0,
                ink,
                Align::Left,
            );
        }
    }
    // 🌊 Argos Vague's pills right of its gauge, once the value columns are drawn: over its value column when the
    // chart has no room for them
    crate::scriptview::draw_vague_pills(pm, text);
}

/// Argos Footprint comme v1 : par rangée, ventes à gauche / achats à droite (chiffres quand il y a la place, sinon
/// barres), déséquilibres diagonaux en couleur pleine, POC encadré, delta sous la bougie.
pub(crate) fn draw_footprint(pm: &mut Pixmap, text: &Text, g: &Geo, f: &flow::Footprint, vals: &[f64]) {
    let p = argos_script::inputs::Inputs {
        params: &flow::FOOTPRINT_PARAMS,
        vals,
    };
    let (cb, cs, cp) = (col(&p, "cb"), col(&p, "cs"), col(&p, "cp"));
    let sp = if g.last > g.first {
        (g.x(g.last) - g.x(g.first)) / (g.last - g.first) as f64
    } else {
        8.0
    };
    let cw = (sp * 0.9).max(2.0);
    if cw < 7.0 {
        return;
    }
    let row_h = (g.y(0.0) - g.y(f.step)).abs();
    let disp = p.pick("disp");
    let num = disp == 1 || (disp == 0 && cw >= 56.0 && row_h >= 11.0);
    let imb = p.get("imb") / 100.0;
    let fs = (row_h as f32 - 2.0).clamp(8.0, 11.0);
    let light = Rgb(0xe6, 0xe6, 0xe8);
    for b in f.bars.iter().filter(|b| (g.first..=g.last).contains(&b.i)) {
        let xc = g.x(b.i);
        let mx = if b.mx > 0.0 { b.mx } else { 1.0 };
        for (k, r) in b.rows.iter().enumerate() {
            let (y0, y1) = (g.y((r.0 + 1) as f64 * f.step), g.y(r.0 as f64 * f.step));
            let (y, h) = (y0.min(y1), ((y1 - y0).abs() - if row_h > 4.0 { 1.0 } else { 0.0 }).max(1.0));
            if y + h < 0.0 || y > g.price_h {
                continue;
            }
            // rangée du dessous (achat ≥ imb × ses ventes) et du dessus (vente ≥ imb × ses achats) : la règle d'argos-core
            let (imb_b, imb_s) = argos_core::orderflow::desequilibre_diagonal(&b.rows, k, imb);
            if num {
                let a = 0.1 + 0.35 * r.1.max(r.2) / mx;
                crate::fill(pm, xc - cw / 2.0, y, cw, h, if r.1 >= r.2 { cb } else { cs }, a as f32);
                let ty = (y + h / 2.0 + fs as f64 * 0.36) as f32;
                text.draw(pm, &kf(r.2), ((xc - 3.0) as f32, ty), fs, if imb_s { cs } else { light }, Align::Right);
                text.draw(pm, &kf(r.1), ((xc + 3.0) as f32, ty), fs, if imb_b { cb } else { light }, Align::Left);
                crate::fill(pm, xc, y, 1.0, h, crate::palette::WHITE, 0.18);
            } else {
                let half = cw / 2.0;
                let (ws, wb) = (half * r.2 / mx, half * r.1 / mx);
                crate::fill(pm, xc - ws, y, ws, h, cs, if imb_s { 0.9 } else { 0.5 });
                crate::fill(pm, xc, y, wb, h, cb, if imb_b { 0.9 } else { 0.5 });
            }
            if p.on("poc") && r.0 == b.poc && cw >= 14.0 {
                crate::shapes::round_stroke(
                    pm,
                    crate::shapes::Rect::new(xc - cw / 2.0 + 0.5, y + 0.5, cw - 1.0, (h - 1.0).max(1.0)),
                    0.0,
                    cp,
                    1.2,
                );
            }
        }
        // 🧮 un petit point au-dessus de la bougie : cyan = vrais trades, orange = vrais trades incomplets (estimée) ;
        // rien = estimée à partir de la bougie
        let point = match b.origine {
            flow::Origine::Reel => Some(crate::palette::cyan()),
            flow::Origine::Incomplet => Some(crate::palette::ORANGE),
            flow::Origine::Estime => None,
        };
        if let Some(c) = point {
            let y = g.y(g.bars[b.i].high) - 8.0;
            crate::fill(pm, xc - 2.0, y - 2.0, 4.0, 4.0, c, 0.9);
        }
        if p.on("dl") && cw >= 28.0 {
            let yl = g.y(g.bars[b.i].low) + 14.0;
            text.draw(
                pm,
                &format!("{}{}", if b.d >= 0.0 { "+" } else { "" }, kf(b.d)),
                (xc as f32, yl as f32),
                fs,
                if b.d >= 0.0 { cb } else { cs },
                Align::Center,
            );
        }
    }
}

/// 🏦 Footprint multi-bourses : sous chaque bougie (sous son delta), la petite barre de la part de chaque bourse
/// (maquette v1 : 4 de haut, 26 sous le bas de la dernière rangée sur PC).
fn draw_fp_parts(pm: &mut Pixmap, g: &Geo, f: &flow::Footprint, m: &crate::fpmulti::FpMulti, vals: &[f64]) {
    let p = argos_script::inputs::Inputs {
        params: &flow::FOOTPRINT_PARAMS,
        vals,
    };
    if !p.on("parts") {
        return;
    }
    let sp = if g.last > g.first {
        (g.x(g.last) - g.x(g.first)) / (g.last - g.first) as f64
    } else {
        8.0
    };
    let cw = (sp * 0.9).max(2.0);
    if cw < 14.0 {
        return;
    }
    for b in f.bars.iter().filter(|b| (g.first..=g.last).contains(&b.i)) {
        let Some(parts) = m.parts_de(g.bars[b.i].time) else { continue };
        let bas = b
            .rows
            .first()
            .map_or(g.y(g.bars[b.i].low), |r| g.y(r.0 as f64 * f.step))
            .max(g.y(g.bars[b.i].low));
        let y = bas + if p.on("dl") && cw >= 28.0 { 22.0 } else { 6.0 };
        if y > g.price_h - 4.0 {
            continue;
        }
        crate::fpmulti::bande(pm, m, parts, g.x(b.i), y, cw * 0.91);
    }
}

/// 📍 VWAP ancrés sur les pivots et marques HH / HL / LH / LL ; `colors` : (haussier, baissier).
fn draw_pivot_vwap(pm: &mut Pixmap, text: &Text, g: &Geo, r: &pivot_vwap::PivotVwap, colors: (Rgb, Rgb)) {
    for leg in &r.legs {
        let pts: Vec<(f64, f64)> = leg
            .points
            .iter()
            .filter(|(i, _)| *i + 1 >= g.first && *i <= g.last + 1)
            .map(|(i, p)| (g.x(*i), g.y(*p)))
            .collect();
        poly(pm, &pts, if leg.rising { colors.0 } else { colors.1 }, 1.6);
    }
    for p in r
        .pivots
        .iter()
        .filter(|p| (g.first..=g.last).contains(&p.at) && p.mark != pivot_vwap::Mark::First)
    {
        let y = g.y(p.price) + if p.is_low { 16.0 } else { -8.0 };
        let c = if p.is_low { colors.0 } else { colors.1 };
        text.draw(pm, p.mark.short(), (g.x(p.at) as f32, y as f32), 10.5, c, Align::Center);
    }
}

#[cfg(test)]
mod tests {

    /// 📏 Jee (19 Nov): scrolled to the first bars of the history, a panel's scale must stay readable. Every
    /// indicator, on a BTC-like history that starts flat (no move at all, like a new listing), then moves: no value of
    /// its panel series is absurd — farther than 1 000 × the series' normal spread (5th–95th percentiles) — the kind
    /// of value Cipher gave while it warmed up (−25 000 000).
    #[test]
    fn aucun_indicateur_ne_donne_de_valeur_absurde_au_demarrage() {
        let bars: Vec<Bar> = (0..600i64)
            .map(|i| {
                let c = if i < 40 {
                    63_000.0
                } else {
                    63_000.0 + ((i - 40) as f64 / 9.0).sin() * 900.0 + (i - 40) as f64 * 2.0
                };
                let r = if i < 40 { 0.0 } else { 120.0 };
                Bar {
                    time: i * 1800,
                    open: c,
                    high: c + r,
                    low: c - r,
                    close: c,
                    volume: 10.0 + (i % 7) as f64,
                }
            })
            .collect();
        let mut absurdes = vec![];
        for ind in Ind::ALL {
            let out = compute(&bars, 1800, &[ind], &Settings::new(), None, 0, &Default::default(), &[]);
            let mut series: Vec<(String, Vec<f64>)> = vec![];
            for pl in out.plots.iter().filter(|p| p.sub) {
                for (k, l) in pl.lines.iter().enumerate() {
                    series.push((format!("{} ligne {k}", pl.name), l.0.clone()));
                }
                if !pl.hist.is_empty() {
                    series.push((format!("{} histogramme", pl.name), pl.hist.iter().map(|h| h.0).collect()));
                }
            }
            if let Some(c) = &out.cipher {
                for (n, v) in [("wt1", &c.wt1), ("wt2", &c.wt2), ("mfi", &c.mfi), ("rsi", &c.rsi), ("stoch", &c.stoch_k)] {
                    series.push((format!("Cipher {n}"), v.clone()));
                }
            }
            if let Some(t) = &out.tbo {
                series.push(("TBO delta".into(), t.core.delta.clone()));
            }
            for (nom, v) in series {
                let mut f: Vec<f64> = v.iter().copied().filter(|x| x.is_finite()).collect();
                if f.len() < 20 {
                    continue;
                }
                f.sort_by(f64::total_cmp);
                let (p5, p95) = (f[f.len() * 5 / 100], f[f.len() * 95 / 100]);
                let ecart = p95 - p5;
                if let Some(x) = f.iter().find(|&&x| ecart > 0.0 && (x < p5 - 1000.0 * ecart || x > p95 + 1000.0 * ecart)) {
                    absurdes.push(format!("{ind:?} · {nom} : {x} (normal {p5} … {p95})"));
                }
            }
        }
        assert!(absurdes.is_empty(), "absurd values:\n{}", absurdes.join("\n"));
    }

    /// 📜 Spring Tension (built-in written in Argos Script): computed by its script on real bars, with no error, drawn
    /// in its own band at the bottom (the « Ruban » plot, the dots); the old by-position list of 45 is unchanged.
    #[test]
    fn spring_tension_calcule_par_son_script() {
        use super::*;
        let bars: Vec<Bar> = (0..300)
            .map(|i| {
                let c = 100.0 + (i as f64 * 0.1).sin() * 5.0;
                Bar {
                    time: i * 1800,
                    open: c - 0.3,
                    high: c + 1.0,
                    low: c - 1.0,
                    close: c,
                    volume: 10.0 + (i % 7) as f64,
                }
            })
            .collect();
        let out = compute(&bars, 1800, &[Ind::SpringTension], &Settings::new(), None, 0, &Default::default(), &[]);
        let (i, v) = out.integres.first().expect("computed");
        assert_eq!(*i, Ind::SpringTension);
        assert!(v.error.is_none(), "{:?}", v.error);
        assert!(!v.out.overlay, "in its band");
        assert!(v.out.plots.iter().any(|p| p.title == "Ruban"), "the ribbon");
        assert_eq!(out.bands(500.0).first().map(|b| b.0), Some(Ind::SpringTension), "its band");
        assert_eq!(Ind::SpringTension as usize, argos_script::catalogue::AVANT_SCRIPTS);
        for m in [Ind::Sma, Ind::Wma, Ind::Hma, Ind::Vwma, Ind::Smma, Ind::MaRibbon] {
            let o = compute(&bars, 1800, &[m], &Settings::new(), None, 0, &Default::default(), &[]);
            let (_, v) = o.integres.first().unwrap_or_else(|| panic!("{m:?}: computed"));
            assert!(v.error.is_none(), "{m:?}: {:?}", v.error);
            assert!(v.out.overlay, "{m:?}: on the price");
        }
        // the legend as TradingView writes it (Jee's capture, 18 Nov)
        assert_eq!(legend_title(Ind::SpringTension, &out), "Spring Tension 14 14 5 35 65 RSI sur hlc3 20 5 20 10 4");
    }

    #[test]
    fn oscillateurs_retrecis_sur_ecran_bas() {
        use super::*;
        let bars: Vec<Bar> = (0..300)
            .map(|i| {
                let c = 100.0 + (i as f64 * 0.1).sin() * 5.0;
                Bar {
                    time: i * 3600,
                    open: c,
                    high: c + 1.0,
                    low: c - 1.0,
                    close: c,
                    volume: 10.0,
                }
            })
            .collect();
        let out = compute(
            &bars,
            3600,
            &[Ind::Cipher, Ind::AiTrend, Ind::Rsi],
            &Default::default(),
            None,
            0,
            &Default::default(),
            &[],
        );
        fit_bands(&out, 2000.0);
        let full = out.osc_h();
        assert!(full <= 2000.0 * MAX_BANDS_SHARE, "grand écran : taille normale ({full})");
        fit_bands(&out, 500.0);
        let small = out.osc_h();
        assert!(small <= 500.0 * MAX_BANDS_SHARE + 0.5, "écran bas : au plus 55 % ({small})");
        assert!(small < full);
        fit_bands(&out, 2000.0);
    }

    /// ⛶ A panel full screen: it alone, the whole plot height (the candles none); `None`: as before; a panel not on
    /// the chart (the Lens preview's outputs, without Cipher) changes nothing.
    #[test]
    fn panneau_en_plein_ecran() {
        use super::*;
        let bars: Vec<Bar> = (0..300)
            .map(|i| {
                let c = 100.0 + (i as f64 * 0.1).sin() * 5.0;
                Bar {
                    time: i * 3600,
                    open: c,
                    high: c + 1.0,
                    low: c - 1.0,
                    close: c,
                    volume: 10.0,
                }
            })
            .collect();
        let calcul = |on: &[Ind]| compute(&bars, 3600, on, &Default::default(), None, 0, &Default::default(), &[]);
        let out = calcul(&[Ind::Cipher, Ind::AiTrend, Ind::Rsi]);
        fit_bands(&out, 800.0);
        let avant = (out.bands(800.0), out.osc_h());
        set_plein_ecran(Some(Ind::Cipher));
        fit_bands(&out, 800.0);
        assert_eq!(out.bands(800.0), vec![(Ind::Cipher, 0.0)], "Cipher alone, from the top");
        assert_eq!(out.osc_h(), 800.0, "the whole plot height: no room left for the candles");
        fit_bands(&out, 600.0);
        assert_eq!(Ind::Cipher.band_h(), 600.0, "follows the plot height, kept from one drawing to the next");
        // the Lens preview (its outputs have no Cipher): unchanged
        let lens = calcul(&[Ind::Lens]);
        fit_bands(&lens, 800.0);
        assert!(lens.bands(800.0).is_empty() && lens.osc_h() == 0.0, "Lens preview: no panel, as before");
        // full screen set on an indicator not on the chart: every panel as before
        set_plein_ecran(Some(Ind::Tbo));
        fit_bands(&out, 800.0);
        assert_eq!((out.bands(800.0), out.osc_h()), avant, "an absent indicator: as before");
        set_plein_ecran(None);
        fit_bands(&out, 800.0);
        assert_eq!((out.bands(800.0), out.osc_h()), avant, "None: every panel as before");
    }
    use super::*;

    fn bars() -> Vec<Bar> {
        // série en vagues : swings, cassures et zones pour chaque indicateur
        (0..600)
            .map(|i| {
                let t = i as f64;
                let c = 100.0 + 8.0 * (t / 17.0).sin() + 4.0 * (t / 5.0).sin() + t * 0.02;
                Bar {
                    time: 1_700_000_000 + i * 1800,
                    open: c - 0.4,
                    high: c + 1.2,
                    low: c - 1.3,
                    close: c + 0.3,
                    volume: 1000.0 + 300.0 * (t / 3.0).cos().abs() + if i % 97 == 0 { 6000.0 } else { 0.0 },
                }
            })
            .collect()
    }

    #[test]
    fn tous_les_indicateurs() {
        let b = bars();
        let out = compute(&b, 1800, &Ind::ALL, &Settings::new(), None, 0, &Default::default(), &[]);
        assert!(out.cipher.is_some() && out.ai.is_some() && out.tbo.is_some() && out.smc.is_some());
        assert!(out.smc_pro.is_some() && out.swing.is_some() && out.smart.is_some() && out.tvd.is_some());
        assert!((0..b.len()).any(|i| out.bar_color(i).is_some()), "bougies vector");
        assert_eq!(out.oscillators(), Ind::ALL.iter().filter(|i| i.oscillator()).count());
        // (bollinger-script-1, 19 Nov: Bollinger is drawn by TradingView's script, no longer a `Plot`)
        assert_eq!(out.plots.len(), 24, "11 simples + 12 des manuels + le CVD");
        assert!(
            out.integres.iter().any(|(i, v)| *i == Ind::Bollinger && v.error.is_none()),
            "Bollinger by its script"
        );
        assert!(out.profile.is_some());
        assert!(
            compute(&b, 1800, &[], &Settings::new(), None, 0, &Default::default(), &[]).smc.is_none(),
            "rien d'actif : rien de calculé"
        );
        assert_eq!(
            Ind::ALL.iter().filter(|i| i.section() == "CLASSIQUES").count(),
            28,
            "8 + 12 des manuels + le CVD + les 6 moyennes de TradingView + Argos Bollinger + RSI"
        );
        assert!(Ind::ALL.iter().enumerate().all(|(k, i)| *i as usize == k), "ALL dans l'ordre de l'enum");
    }

    /// 🌊 Argos Vague's legend (mockup v7): the style « Auto » resolved by the unit and the engine, then the last wave
    /// value.
    #[test]
    fn legende_argos_vague() {
        let b = bars();
        let set = Settings::new();
        assert_eq!(vague_legende(&set, 300).as_deref(), Some("Scalpeur · Classique"));
        assert_eq!(vague_legende(&set, 1800).as_deref(), Some("Trader du jour · Classique"));
        assert_eq!(vague_legende(&set, 14_400).as_deref(), Some("Trader du jour · Classique"));
        assert_eq!(vague_legende(&set, 86_400).as_deref(), Some("Swing · Classique"));
        let out = compute(&b, 1800, &[Ind::ArgosVague], &set, None, 0, &Default::default(), &[]);
        let t = legend_title(Ind::ArgosVague, &out);
        let (v, _) = vague_valeur(&out).expect("a wave");
        assert!(t.starts_with("Argos Vague  Trader du jour · Classique  "), "{t}");
        assert!(t.ends_with(&crate::viewport::format_number(v, 1)), "{t}");
    }

    /// 📜 Argos Vague (mockup v7): a tall panel by default (close to half of a PC plot), squeezed on a low screen,
    /// and a height the user chose by dragging its edge is kept.
    #[test]
    fn argos_vague_bande_haute_par_defaut() {
        let b = bars();
        let out = compute(&b, 1800, &[Ind::ArgosVague], &Settings::new(), None, 0, &Default::default(), &[]);
        set_heights(&Default::default());
        fit_bands(&out, 740.0);
        let part = Ind::ArgosVague.band_h() / 740.0;
        assert!((0.4..=0.5).contains(&part), "PC: close to half ({part})");
        assert!(Ind::ArgosVague.band_h() > 2.0 * Ind::Rsi.default_h(), "taller than an ordinary panel");
        fit_bands(&out, 500.0);
        assert!(out.osc_h() <= 500.0 * MAX_BANDS_SHARE + 1e-9, "low screen: the candles keep their part");
        fit_bands(&out, 740.0);
        let mut h = std::collections::HashMap::new();
        h.insert(Ind::ArgosVague, 180.0);
        set_heights(&h);
        assert_eq!(Ind::ArgosVague.band_h(), 180.0, "the height chosen by the user is kept");
        set_heights(&Default::default());
    }

    /// A chart drawn by [`dessine_avec_place_devant`]: its image and the numbers to find a bar and a price on it.
    struct Dessin {
        pm: Pixmap,
        /// x of the bar 0, and from one bar to the next.
        x0: f64,
        pas: f64,
        /// The price scale: lowest and highest price, top and height in pixels.
        bas: f64,
        haut: f64,
        top: f64,
        h: f64,
        /// Right edge of the prices.
        right: f64,
    }

    impl Dessin {
        /// x of a bar (fractional too, also after the last bar).
        fn x(&self, i: f64) -> f64 {
            self.x0 + i * self.pas
        }
        fn y(&self, p: f64) -> f64 {
            self.top + (self.haut - p) / (self.haut - self.bas) * self.h
        }
    }

    /// Draws `out` on a black 1200 × 900 chart showing the last bar with `devant` empty bar places after it.
    fn dessine_avec_place_devant(b: &[Bar], out: &IndOut, devant: f64) -> Dessin {
        let vp = Viewport {
            width: 1200,
            height: 900,
            right_offset: devant,
            ..Viewport::default()
        };
        let slots = Slots::uniform(b.len());
        let (first, last) = vp.visible(&slots).unwrap();
        assert_eq!(last + 1, b.len(), "the last bar is shown");
        let scale = PriceScale::fit(b, first, last, vp.plot_h());
        let g = Geo {
            vp: &vp,
            scale: &scale,
            slots: &slots,
            first,
            last,
            bars: b,
            price_h: vp.plot_h() - out.osc_h(),
            tz_offset_s: 0,
        };
        let mut pm = crate::px::new(1200, 900).unwrap();
        pm.fill(tiny_skia::Color::BLACK);
        draw(&mut pm, &Text::default(), &g, out);
        Dessin {
            x0: g.xf(0.0),
            pas: g.xf(1.0) - g.xf(0.0),
            bas: scale.min,
            haut: scale.max,
            top: scale.top,
            h: scale.height,
            right: g.right(),
            pm,
        }
    }

    /// Colour of the pixel at (x, y), its three channels.
    fn teinte(pm: &Pixmap, x: f64, y: f64) -> (u8, u8, u8) {
        assert!(x >= 0.0 && y >= 0.0, "on the image: {x}, {y}");
        let p = pm.pixel(x.floor() as u32, y.floor() as u32).unwrap();
        (p.red(), p.green(), p.blue())
    }

    /// 📖 B-045: Ichimoku's cloud as on TradingView: drawn 25 bars ahead of the last bar (senkou A and B moved forward
    /// by `displacement - 1`), green where senkou A is above senkou B and red where it is below. It stopped at the last
    /// bar, in one colour.
    #[test]
    fn le_nuage_d_ichimoku_va_25_bougies_devant_en_vert_ou_rouge() {
        let b = bars();
        let n = b.len();
        let out = compute(&b, 1800, &[Ind::Ichimoku], &Settings::new(), None, 0, &Default::default(), &[]);
        let pl = out.plots.iter().find(|p| p.ind == Ind::Ichimoku).unwrap();
        // the values ahead: TradingView's senkou, computed 25 bars before the bar they are drawn at
        let mid = |i: usize, len: usize| {
            let w = &b[i + 1 - len..=i];
            (w.iter().map(|x| x.high).fold(f64::MIN, f64::max) + w.iter().map(|x| x.low).fold(f64::MAX, f64::min)) / 2.0
        };
        assert_eq!((pl.ahead[2].len(), pl.ahead[3].len()), (25, 25), "25 bars ahead, as TradingView's 26 − 1");
        for k in 0..25 {
            let i = n - 25 + k;
            assert!((pl.ahead[2][k] - (mid(i, 9) + mid(i, 26)) / 2.0).abs() < 1e-9, "senkou A, bar {}", n + k);
            assert!((pl.ahead[3][k] - mid(i, 52)).abs() < 1e-9, "senkou B, bar {}", n + k);
        }
        assert!(
            pl.ahead[0].is_empty() && pl.ahead[1].is_empty() && pl.ahead[4].is_empty(),
            "only the two senkou"
        );
        // drawn: room for 40 bars after the last one, the cloud goes 25 bars and stops there
        let d = dessine_avec_place_devant(&b, &out, 40.0);
        let (pm, right, x, y) = (&d.pm, d.right, |i: f64| d.x(i), |p: f64| d.y(p));
        assert!(x(n as f64 + 40.0) < right + 20.0, "the room is on the screen");
        let a_b = |k: usize| (pl.ahead[2][k], pl.ahead[3][k]);
        let mut vus = (0, 0);
        for k in 1..24 {
            let (a, bb) = a_b(k);
            // (away from a crossing: the segment before and after the bar have its colour)
            let meme = [k - 1, k + 1].iter().all(|&j| (a_b(j).0 > a_b(j).1) == (a > bb));
            if (y(a) - y(bb)).abs() < 8.0 || !meme {
                continue;
            }
            let (r, g, _) = teinte(pm, x((n + k) as f64), (y(a) + y(bb)) / 2.0);
            if a > bb {
                assert!(g > r + 3, "bar {}: green where A is above B, not {:?}", n + k, (r, g));
                vus.0 += 1;
            } else {
                assert!(r > g + 3, "bar {}: red where A is below B, not {:?}", n + k, (r, g));
                vus.1 += 1;
            }
        }
        assert!(vus.0 + vus.1 >= 5, "the cloud is drawn ahead of the last bar: {vus:?}");
        // and the past too: red where A is below B (one colour before)
        let sous = |i: usize| pl.lines[2].0[i] < pl.lines[3].0[i];
        let rouges = (n - 200..n - 1)
            .filter(|&i| x(i as f64) > 2.0 && sous(i - 1) && sous(i) && sous(i + 1))
            .filter(|&i| (y(pl.lines[2].0[i]) - y(pl.lines[3].0[i])).abs() > 8.0)
            .inspect(|&i| {
                let (r, g, _) = teinte(pm, x(i as f64), (y(pl.lines[2].0[i]) + y(pl.lines[3].0[i])) / 2.0);
                assert!(r > g + 3, "bar {i}: red where A is below B, not {:?}", (r, g));
            })
            .count();
        assert!(rouges > 0, "the history has a red cloud");
        // nothing past the 25 bars, nor past the prices' right edge
        for xx in [x(n as f64 + 27.0), x(n as f64 + 35.0), right + 2.0] {
            for yy in (0..860).step_by(4) {
                assert_eq!(teinte(pm, xx, yy as f64), (0, 0, 0), "nothing at x {xx}, y {yy}");
            }
        }
    }

    /// 📖 B-046: a script's plots shifted ahead (`plot(…, offset = 10)`, as TradingView's own Ichimoku) are drawn after
    /// the last bar, and the fill between them is shifted with them: each bar's colour is the one of the bar it was
    /// computed on. They stopped at the last bar, and the fill took its colour 10 bars off.
    #[test]
    fn un_script_decale_vers_l_avant_dessine_apres_la_derniere_bougie() {
        let b = bars();
        let n = b.len();
        let dessine = |plots: &str, fill: &str| {
            let src = format!(
                "//@version=6\nindicator(\"Avant\", overlay = true)\na = ta.sma(close, 5)\nb = ta.sma(close, 30)\n\
                 p1 = plot(a, offset = 10, color = {plots})\np2 = plot(b, offset = 10, color = {plots})\n{fill}"
            );
            let o = argos_script::lang::run(&src, &b, 1800, &Default::default()).unwrap();
            let (a, bb) = (o.plots[0].values.clone(), o.plots[1].values.clone());
            let mut out = compute(&b, 1800, &[], &Settings::new(), None, 0, &Default::default(), &[]);
            out.scripts.push(crate::scriptview::ScriptView {
                name: "Avant".into(),
                out: o,
                ..Default::default()
            });
            (dessine_avec_place_devant(&b, &out, 30.0), a, bb)
        };
        // the two lines alone (white): after the last bar, 10 bars ahead, then nothing
        let (d, a, bb) = dessine("color.white", "");
        let (pm, right, x, y) = (&d.pm, d.right, |i: f64| d.x(i), |p: f64| d.y(p));
        assert!(x(n as f64 + 12.0) < right, "the room is on the screen");
        for k in 1..10 {
            let j = n - 10 + k;
            for (nom, v) in [("a", &a), ("b", &bb)] {
                let blanc = (-2..=2).any(|d| {
                    let (r, g, bl) = teinte(pm, x((n + k) as f64), y(v[j]) + d as f64);
                    r > 100 && g > 100 && bl > 100
                });
                assert!(blanc, "plot {nom} at bar {}", n + k);
            }
        }
        for yy in (0..860).step_by(4) {
            assert_eq!(teinte(pm, x(n as f64 + 14.0), yy as f64), (0, 0, 0), "nothing at y {yy}");
        }
        // the fill alone (lines with no colour): each bar's colour is the one of the bar its values were computed on,
        // 10 bars before where it is drawn, also after the last bar
        let (d, a, bb) = dessine("na", "fill(p1, p2, color = a > b ? color.lime : color.red)");
        let (pm, x, y) = (&d.pm, |i: f64| d.x(i), |p: f64| d.y(p));
        let dessus = |j: usize| a[j] > bb[j];
        let mut vus = (0, 0);
        for i in (n - 150..n + 9).filter(|&i| x(i as f64) > 2.0) {
            let j = i - 10;
            let (ya, yb) = (y(a[j]), y(bb[j]));
            // (away from a crossing: the segment before and after the bar have its colour)
            if (ya - yb).abs() < 10.0 || dessus(j - 1) != dessus(j) || dessus(j + 1) != dessus(j) {
                continue;
            }
            let (r, g, _) = teinte(pm, x(i as f64), (ya + yb) / 2.0);
            if dessus(j) {
                assert!(g > r + 100, "bar {i}: lime (computed at {j}), not {:?}", (r, g));
            } else {
                assert!(r > g + 100, "bar {i}: red (computed at {j}), not {:?}", (r, g));
            }
            if i >= n {
                vus.1 += 1;
            } else {
                vus.0 += 1;
            }
        }
        assert!(vus.0 > 30 && vus.1 > 2, "bars looked at, before and after the last one: {vus:?}");
    }

    /// 🌊 Argos Vague's pane (retouch 3, V2) has its scale on the price column like the RSI's: a built-in written in
    /// Argos Script gives back its band's axis (the script's own range), and so does a script in its own pane.
    #[test]
    fn argos_vague_et_script_en_panneau_ont_une_echelle() {
        let b = bars();
        let mut out = compute(&b, 1800, &[Ind::ArgosVague, Ind::Rsi], &Settings::new(), None, 0, &Default::default(), &[]);
        let src = "//@version=6\nindicator(\"S\", overlay = false)\nplot(close)";
        let o = argos_script::lang::run(src, &b, 1800, &Default::default()).unwrap();
        out.scripts.push(crate::scriptview::ScriptView {
            name: "s".into(),
            out: o,
            ..Default::default()
        });
        let vp = Viewport {
            width: 1200,
            height: 900,
            ..Viewport::default()
        };
        let slots = Slots::uniform(b.len());
        let (first, last) = vp.visible(&slots).unwrap_or((0, b.len() - 1));
        let scale = PriceScale::fit(&b, first, last, vp.plot_h());
        let g = Geo {
            vp: &vp,
            scale: &scale,
            slots: &slots,
            first,
            last,
            bars: &b,
            price_h: vp.plot_h() - out.osc_h(),
            tz_offset_s: 0,
        };
        let mut pm = crate::px::new(1200, 900).unwrap();
        let axes = draw(&mut pm, &Text::default(), &g, &out);
        let top = |i: Ind| out.bands(vp.plot_h()).into_iter().find(|(j, _)| *j == i).unwrap().1;
        let v = axes.iter().find(|a| a.top == top(Ind::ArgosVague)).expect("Argos Vague's scale");
        assert!(v.hi > v.lo && v.lo < 0.0 && v.hi > 0.0, "{} .. {}", v.lo, v.hi);
        assert_eq!(v.h, Ind::ArgosVague.band_h());
        assert!(axes.iter().any(|a| a.top == top(Ind::Rsi)), "the RSI's, as before");
        let (_, st) = out.script_bands(vp.plot_h())[0];
        assert!(axes.iter().any(|a| a.top == st), "a script's pane");
        assert_eq!(axes.len(), 3);
        draw_axes(&mut pm, &Text::default(), &g, &axes, Rgb(0, 0, 0));
    }

    /// 📜 R1 (editeur-pine-1): a user script's pane is `OSC_H` (one plot), 2 × `OSC_H` with drawings in its own pane
    /// or 6 plots or more; a height dragged (by script id) is used; with a native band the two shrink together on a
    /// low screen and the prices keep their 45 %.
    #[test]
    fn hauteur_d_un_panneau_de_script() {
        let b = bars();
        let run = |src: &str| argos_script::lang::run(src, &b, 1800, &Default::default()).unwrap();
        let vue = |id: &str, src: &str| crate::scriptview::ScriptView {
            name: id.into(),
            id: id.into(),
            out: run(src),
            ..Default::default()
        };
        set_script_heights(&Default::default());
        let mut out = compute(&b, 1800, &[Ind::Rsi], &Settings::new(), None, 0, &Default::default(), &[]);
        fit_bands(&out, 2000.0);
        let simple = vue("a", "//@version=6\nindicator(\"S\")\nplot(close)");
        assert_eq!(simple.band_h(), OSC_H, "one plot");
        let six = vue("b", &format!("//@version=6\nindicator(\"S\")\n{}", "plot(close)\n".repeat(6)));
        assert_eq!(six.band_h(), 2.0 * OSC_H, "6 plots");
        let dessin = vue(
            "c",
            "//@version=6\nindicator(\"S\")\nplot(close)\nif barstate.islast\n    label.new(bar_index, 1, \"x\")\n",
        );
        assert_eq!(dessin.band_h(), 2.0 * OSC_H, "a drawing in its pane");
        let force = vue(
            "d",
            "//@version=6\nindicator(\"S\")\nplot(close)\nif barstate.islast\n    label.new(bar_index, close, \"x\", force_overlay = true)\n",
        );
        assert_eq!(force.band_h(), OSC_H, "a drawing on the price does not count");
        out.scripts.push(dessin);
        let ph = 900.0;
        let (_, top) = out.script_bands(ph)[0];
        assert_eq!(out.script_edge(ph, top + 2.0), Some(0));
        assert_eq!(top, ph - Ind::Rsi.band_h() - 2.0 * OSC_H);
        set_script_heights(&[("c".to_string(), 330.0)].into_iter().collect());
        assert_eq!(out.scripts[0].band_h(), 330.0, "the dragged height, by id");
        assert_eq!(out.osc_h(), Ind::Rsi.band_h() + 330.0);
        fit_bands(&out, 500.0);
        assert!(out.osc_h() <= 500.0 * MAX_BANDS_SHARE + 1e-9, "squeezed together: the prices keep 45 %");
        assert!(out.scripts[0].band_h() < 330.0);
        fit_bands(&out, 2000.0);
        set_script_heights(&Default::default());
    }

    /// ↕️ Bandes : hauteur choisie (gardée dans les réglages), bord attrapable, valeurs sur la colonne de prix.
    #[test]
    fn bandes_agrandies_et_valeurs() {
        let b = bars();
        let out = compute(&b, 1800, &[Ind::Rsi, Ind::AiTrend], &Settings::new(), None, 0, &Default::default(), &[]);
        let ph = 600.0;
        let tops = out.bands(ph);
        assert_eq!(tops.len(), 2);
        let (ind, top) = tops[1];
        assert_eq!(out.band_edge(ph, top + 3.0), Some(ind));
        assert_eq!(out.band_at(ph, top + 30.0), Some(ind));
        let mut h = std::collections::HashMap::new();
        h.insert(ind, 300.0);
        set_heights(&h);
        assert_eq!(ind.band_h(), 300.0);
        assert!(out.bands(ph)[1].1 < top, "bande plus haute : son bord monte");
        let cs = crate::settings::ChartSettings {
            bands: h.clone(),
            ..Default::default()
        };
        let json = serde_json::to_string(&cs).unwrap();
        assert_eq!(
            serde_json::from_str::<crate::settings::ChartSettings>(&json).unwrap().bands,
            h,
            "gardée dans les réglages"
        );
        set_heights(&Default::default());
        assert_eq!(ind.band_h(), ind.default_h());
        // valeurs : une échelle par bande, avec les dernières valeurs
        let vp = Viewport {
            width: 900,
            height: 700,
            ..Viewport::default()
        };
        let slots = Slots::uniform(b.len());
        let (first, last) = vp.visible(&slots).unwrap_or((0, b.len() - 1));
        let scale = PriceScale::fit(&b, first, last, vp.plot_h());
        let g = Geo {
            vp: &vp,
            scale: &scale,
            slots: &slots,
            first,
            last,
            bars: &b,
            price_h: vp.plot_h() - out.osc_h(),
            tz_offset_s: 0,
        };
        let mut pm = crate::px::new(900, 700).unwrap();
        let axes = draw(&mut pm, &Text::default(), &g, &out);
        assert_eq!(axes.len(), 2);
        assert!(axes.iter().all(|a| a.hi > a.lo && !a.last.is_empty()));
        assert_eq!((axes.iter().find(|a| a.hi == 100.0).map(|a| a.lo)), Some(0.0), "AI Trend : 0 à 100");
        draw_axes(&mut pm, &Text::default(), &g, &axes, Rgb(0, 0, 0));
        // vue de la bande (glisser dedans / sur sa colonne) : son échelle bouge, le réticule lit la bonne valeur
        let ai_top = out.bands(g.vp.plot_h()).into_iter().find(|(i, _)| *i == Ind::AiTrend).unwrap().1;
        let ai = axes.iter().find(|a| a.top == ai_top).unwrap().clone();
        assert!((ai.value_at(ai.y(50.0)) - 50.0).abs() < 1e-9);
        let mut v = std::collections::HashMap::new();
        v.insert(Ind::AiTrend, (0.25, 2.0));
        set_views(&v, &Default::default());
        let moved = draw(&mut pm, &Text::default(), &g, &out);
        let a2 = moved.iter().find(|a| a.top == ai.top).unwrap();
        assert_eq!((a2.lo, a2.hi), (-25.0, 175.0), "décalée d'un quart, zoom × 2");
        set_views(&Default::default(), &Default::default());
    }

    /// Le fond entre les bandes #1 de l'Anchored VWAP est uniforme (un rectangle par bougie faisait des stries).
    #[test]
    fn avwap_remplissage_sans_stries() {
        let b = bars();
        let a = anchored_vwap::AnchoredVwap {
            from: 0,
            vwap: vec![100.0; b.len()],
            dev: vec![5.0; b.len()],
        };
        let vp = Viewport {
            width: 800,
            height: 600,
            bar_spacing: 2.3,
            ..Viewport::default()
        };
        // bougies de volume : des places inégales
        let slots = Slots::weighted((0..b.len()).map(|i| [0.6, 1.0, 2.4, 1.3, 0.8][i % 5]));
        let (first, last) = vp.visible(&slots).unwrap_or((0, b.len() - 1));
        let scale = PriceScale::fit(&b, first, last, vp.plot_h());
        let g = Geo {
            vp: &vp,
            scale: &scale,
            slots: &slots,
            first,
            last,
            bars: &b,
            price_h: vp.plot_h(),
            tz_offset_s: 0,
        };
        let mut pm = crate::px::new(800, 600).unwrap();
        draw_avwap(&mut pm, &g, &a, &values(&Settings::new(), Ind::AnchoredVwap));
        let y = ((g.y(100.0) + g.y(105.0)) / 2.0) as u32;
        let (x0, x1) = (g.x(first).ceil() as u32 + 2, g.x(last).floor() as u32 - 2);
        let row: Vec<u8> = (x0..x1).map(|x| pm.pixel(x, y).unwrap().alpha()).collect();
        assert!(row[0] > 0, "le fond est dessiné");
        assert!(row.iter().all(|&v| v == row[0]), "fond uniforme : {:?}", &row[..row.len().min(40)]);
    }

    /// Les 4 emplacements multi-UT d'AI Trend se dessinent (le 4e plantait : « input inconnu : m5_cb »).
    #[test]
    fn ai_trend_quatre_emplacements_multi_ut() {
        let b = bars();
        let mut out = compute(&b, 1800, &[Ind::AiTrend], &Settings::new(), None, 0, &Default::default(), &[]);
        let ai = out.ai.as_mut().unwrap();
        ai.mtf = (1..=4)
            .map(|slot| ai_trend::MtfDot {
                slot,
                bull: slot % 2 == 0,
                index: b.len() - 10 + slot,
            })
            .collect();
        let vp = Viewport {
            width: 800,
            height: 600,
            ..Viewport::default()
        };
        let slots = Slots::uniform(b.len());
        let (first, last) = vp.visible(&slots).unwrap_or((0, b.len() - 1));
        let scale = PriceScale::fit(&b, first, last, vp.plot_h());
        let g = Geo {
            vp: &vp,
            scale: &scale,
            slots: &slots,
            first,
            last,
            bars: &b,
            price_h: vp.plot_h() - out.osc_h(),
            tz_offset_s: 0,
        };
        let mut pm = crate::px::new(800, 600).unwrap();
        draw(&mut pm, &Text::default(), &g, &out);
        for n in 1..=4 {
            for k in ["cb", "cr", "sz"] {
                assert!(ai_trend::PARAMS.iter().any(|p| p.key == format!("m{n}_{k}")), "m{n}_{k}");
            }
        }
    }

    /// Settings where `ind`'s « Tableau résumé » is ticked (`on`) or not.
    fn tableau_coche(ind: Ind, on: bool) -> Settings {
        let mut v = values(&Settings::new(), ind);
        let k = ind.params().1.iter().position(|p| p.key == "showTable").expect("showTable");
        v[k] = if on { 1.0 } else { 0.0 };
        let mut s = Settings::new();
        s.insert(ind, v);
        s
    }

    /// The table rows' names of a simple indicator (`Plot::tableau`).
    fn noms_tableau(out: &IndOut, ind: Ind) -> Vec<&'static str> {
        out.plots
            .iter()
            .find(|p| p.ind == ind)
            .map(|p| p.tableau.iter().map(|r| r.0).collect())
            .unwrap_or_default()
    }

    /// A 800 × 600 chart of `b` (all of it visible).
    fn geo_de<'a>(b: &'a [Bar], vp: &'a Viewport, slots: &'a Slots, scale: &'a PriceScale) -> Geo<'a> {
        let (first, last) = vp.visible(slots).unwrap_or((0, b.len() - 1));
        Geo {
            vp,
            scale,
            slots,
            first,
            last,
            bars: b,
            price_h: vp.plot_h(),
            tz_offset_s: 0,
        }
    }

    /// 🧮 Argos Bubbles reads only the real trades: bubbles on the bar that has them, none elsewhere, and the legend
    /// says since when; without real trades, no bubble and the legend says they are missing.
    #[test]
    fn bulles_vrais_trades_et_legende() {
        let b = bars();
        let n = b.len();
        let vals = values(&Settings::new(), Ind::Bubbles);
        let per = argos_script::inputs::Inputs {
            params: &flow::BUBBLES_PARAMS,
            vals: &vals,
        }
        .get("per");
        let step = flow::auto_step(&b, per);
        let k0 = (b[n - 1].close / step).floor() as i64;
        let reel = vec![argos_core::orderflow::FlowBar {
            time: b[n - 1].time,
            pas: step,
            rangees: vec![(k0 - 1, 5.0, 5.0), (k0, 5.0, 5.0), (k0 + 1, 5.0, 5.0), (k0 + 2, 100.0, 1.0)],
            complete: true,
        }];
        let avec = compute(&b, 1800, &[Ind::Bubbles], &Settings::new(), None, 0, &Default::default(), &reel);
        let bu = avec.bubbles.as_ref().unwrap();
        assert!(
            !bu.items.is_empty() && bu.items.iter().all(|x| x.i == n - 1),
            "bubbles only on the bar with trades"
        );
        assert_eq!(bu.pocs.iter().map(|p| p.0).collect::<Vec<_>>(), vec![n - 1]);
        let leg = legend_title(Ind::Bubbles, &avec);
        assert!(leg.contains(" · vrais trades depuis ") && leg.ends_with(" ET"), "{leg}");
        let sans = compute(&b, 1800, &[Ind::Bubbles], &Settings::new(), None, 0, &Default::default(), &[]);
        let bu = sans.bubbles.as_ref().unwrap();
        assert!(bu.items.is_empty() && bu.pocs.is_empty(), "no real trades: nothing estimated");
        assert!(legend_title(Ind::Bubbles, &sans).ends_with(BULLES_ABSENTES));
        assert_eq!(BULLES_ABSENTES, "Bulles : vrais trades absents pour ce marché ou cette unité");
    }

    /// The Bubbles POC is a clearly visible yellow dot (radius 3, it was 1.8) with a thin dark outline.
    #[test]
    fn point_poc_bien_visible() {
        let b = bars();
        let vp = Viewport {
            width: 800,
            height: 600,
            ..Viewport::default()
        };
        let slots = Slots::uniform(b.len());
        let (first, last) = vp.visible(&slots).unwrap_or((0, b.len() - 1));
        let scale = PriceScale::fit(&b, first, last, vp.plot_h());
        let g = geo_de(&b, &vp, &slots, &scale);
        let i = (g.first + g.last) / 2;
        let bu = flow::Bubbles {
            items: vec![],
            pocs: vec![(i, b[i].close)],
            dash: vec![],
            depuis: None,
        };
        let mut vals = values(&Settings::new(), Ind::Bubbles);
        let k = flow::BUBBLES_PARAMS.iter().position(|p| p.key == "showTable").unwrap();
        vals[k] = 0.0;
        let mut pm = crate::px::new(800, 600).unwrap();
        draw_bubbles(&mut pm, &Text::default(), &g, &bu, &vals);
        const { assert!(POC_R >= 3.0) };
        let (cx, cy) = (g.x(i).round() as i64, g.y(b[i].close).round() as i64);
        let (mut peints, mut jaunes, mut sombres) = (0, 0, 0);
        for y in cy - 6..=cy + 6 {
            for x in cx - 6..=cx + 6 {
                let Some(px) = pm.pixel(x as u32, y as u32) else { continue };
                if px.alpha() == 0 {
                    continue;
                }
                peints += 1;
                if px.alpha() > 200 && px.red() > 200 && px.green() > 150 && px.blue() < 100 {
                    jaunes += 1;
                } else if px.red() < 120 && px.green() < 120 {
                    sombres += 1;
                }
            }
        }
        assert!(peints >= 30, "radius 3 and its outline (the 1.8 dot touched about 20 pixels): {peints}");
        assert!(jaunes > 0, "still yellow inside");
        assert!(sombres > 0, "its dark outline");
    }

    /// 📐 A table that does not fit even at the smallest size is replaced by one line « Tableau trop grand pour cette
    /// place » in its corner (no more silent skip).
    #[test]
    fn tableau_trop_grand_une_ligne() {
        TABLES.with(|t| t.borrow_mut().clear());
        let vals = values(&Settings::new(), Ind::Bubbles);
        let inp = argos_script::inputs::Inputs {
            params: &flow::BUBBLES_PARAMS,
            vals: &vals,
        };
        let rows: Vec<(&str, String)> = (0..5).map(|k| ("Ligne", k.to_string())).collect();
        let mut pm = crate::px::new(800, 300).unwrap();
        let area = crate::shapes::Rect::new(0.0, 0.0, 800.0, 20.0);
        table(&mut pm, &Text::default(), area, "Argos Bubbles", &rows, &inp, "", Ind::Bubbles);
        let posees = tableaux();
        assert_eq!(posees.len(), 1, "the line takes the table's place");
        assert!((posees[0].h - (9.0 * 1.8 + 8.0)).abs() < 1e-6, "one line, smallest size: {:?}", posees[0]);
        assert!(posees[0].x > 400.0, "in the table's corner (top right by default)");
        assert!(pm.data().iter().any(|&v| v != 0), "drawn");
        assert_eq!(TROP_GRAND, "Tableau trop grand pour cette place");
        // room enough: the real table, taller than one line
        TABLES.with(|t| t.borrow_mut().clear());
        table(
            &mut pm,
            &Text::default(),
            crate::shapes::Rect::new(0.0, 0.0, 800.0, 300.0),
            "Argos Bubbles",
            &rows,
            &inp,
            "",
            Ind::Bubbles,
        );
        assert!(tableaux()[0].h > 9.0 * 1.8 + 8.0);
    }

    /// 📋 Argos Heatmap Volume: its summary table (v1's rows) when « Tableau résumé » is ticked, none otherwise.
    #[test]
    fn tableau_heatmap() {
        let b = bars();
        let on = compute(&b, 1800, &[Ind::Heatmap], &tableau_coche(Ind::Heatmap, true), None, 0, &Default::default(), &[]);
        assert_eq!(noms_tableau(&on, Ind::Heatmap), vec!["Volume", "Niveau", "Écart", "Moyenne"]);
        let row = &on.plots[0].tableau;
        assert!(["Très fort", "Fort", "Moyen", "Normal", "Faible"].contains(&row[1].1.as_str()));
        assert!(row[2].1.ends_with(" σ"));
        let off = compute(
            &b,
            1800,
            &[Ind::Heatmap],
            &tableau_coche(Ind::Heatmap, false),
            None,
            0,
            &Default::default(),
            &[],
        );
        assert!(noms_tableau(&off, Ind::Heatmap).is_empty());
    }

    /// 📋 Argos Volume Profile: POC, VAH, VAL and price against the POC when ticked, nothing otherwise.
    #[test]
    fn tableau_profile() {
        let b = bars();
        let set = tableau_coche(Ind::Profile, true);
        let out = compute(&b, 1800, &[Ind::Profile], &set, None, 0, &Default::default(), &[]);
        let p = out.profile.as_ref().unwrap();
        let rows = profile_tableau(p, &b);
        assert_eq!(rows.iter().map(|r| r.0).collect::<Vec<_>>(), vec!["POC", "VAH", "VAL", "Prix vs POC"]);
        assert!(rows[3].1 == "Au-dessus ⬆️" || rows[3].1 == "En dessous ⬇️");
        // (unticked: the rows are still computed — shared with Argos AI — and the chart does not draw the table)
        let off = tableau_coche(Ind::Profile, false);
        let out_off = compute(&b, 1800, &[Ind::Profile], &off, None, 0, &Default::default(), &[]);
        assert_eq!(profile_tableau(out_off.profile.as_ref().unwrap(), &b), rows);
    }

    /// 📋 Argos Bollinger Bands: %B, width, squeeze and position when ticked (and the table is drawn), nothing otherwise.
    #[test]
    fn tableau_bands() {
        let b = bars();
        let on = compute(&b, 1800, &[Ind::Bands], &tableau_coche(Ind::Bands, true), None, 0, &Default::default(), &[]);
        assert_eq!(noms_tableau(&on, Ind::Bands), vec!["%B", "Largeur", "Squeeze", "Position"]);
        let off = compute(&b, 1800, &[Ind::Bands], &tableau_coche(Ind::Bands, false), None, 0, &Default::default(), &[]);
        assert!(noms_tableau(&off, Ind::Bands).is_empty());
        // drawn on the chart
        let vp = Viewport {
            width: 800,
            height: 600,
            ..Viewport::default()
        };
        let slots = Slots::uniform(b.len());
        let (first, last) = vp.visible(&slots).unwrap_or((0, b.len() - 1));
        let scale = PriceScale::fit(&b, first, last, vp.plot_h());
        let g = geo_de(&b, &vp, &slots, &scale);
        let mut pm = crate::px::new(800, 600).unwrap();
        TABLES.with(|t| t.borrow_mut().clear());
        draw(&mut pm, &Text::default(), &g, &on);
        assert_eq!(tableaux().len(), 1, "its table is on the chart");
        TABLES.with(|t| t.borrow_mut().clear());
        draw(&mut pm, &Text::default(), &g, &off);
        assert!(tableaux().is_empty());
    }

    /// 📋 Argos Bollinger Oscillator: bullish and bearish strength when ticked (off by default), nothing otherwise.
    #[test]
    fn tableau_bands_osc() {
        let b = bars();
        let on = compute(
            &b,
            1800,
            &[Ind::BandsOsc],
            &tableau_coche(Ind::BandsOsc, true),
            None,
            0,
            &Default::default(),
            &[],
        );
        assert_eq!(noms_tableau(&on, Ind::BandsOsc), vec!["Haussier", "Baissier"]);
        assert!(on.plots[0].tableau.iter().all(|r| r.1.ends_with(" %")));
        let off = compute(&b, 1800, &[Ind::BandsOsc], &Settings::new(), None, 0, &Default::default(), &[]);
        assert!(noms_tableau(&off, Ind::BandsOsc).is_empty(), "unticked by default");
    }

    /// 🧮 CVD et footprint avec l'orderflow réel : la dernière bougie a ses vrais trades, le reste est estimé.
    #[test]
    fn cvd_et_footprint_avec_l_orderflow_reel() {
        let b = bars();
        let last = b.last().unwrap();
        let reel = vec![argos_core::orderflow::FlowBar {
            time: last.time,
            pas: 0.01,
            rangees: vec![((last.close / 0.01) as i64, 900.0, 100.0)],
            complete: true,
        }];
        let on = [Ind::Cvd, Ind::Footprint];
        let sans = compute(&b, 1800, &on, &Settings::new(), None, 0, &Default::default(), &[]);
        let avec = compute(&b, 1800, &on, &Settings::new(), None, 0, &Default::default(), &reel);
        let cvd = |o: &IndOut| o.plots.iter().find(|p| p.ind == Ind::Cvd).map(|p| p.lines[0].0.clone()).unwrap();
        let (c0, c1) = (cvd(&sans), cvd(&avec));
        let n = b.len();
        assert_eq!(c0[..n - 1], c1[..n - 1], "avant : la même estimation");
        assert!((c1[n - 1] - c1[n - 2] - 800.0).abs() < 1e-6, "la dernière bougie : son vrai delta +800");
        let fp = avec.footprint.as_ref().unwrap();
        assert_eq!(fp.bars.last().map(|x| x.origine), Some(flow::Origine::Reel));
        assert_eq!(fp.bars[fp.bars.len() - 2].origine, flow::Origine::Estime);
        assert_eq!(avec.reel, reel, "gardé pour le footprint en type de chart");
        // 🧮 the legend says « estimé » without real trades (or with incomplete ones only), not with a real bar
        for i in on {
            let debut = format!("{} · {ESTIME}", i.name());
            assert!(legend_title(i, &sans).starts_with(&debut), "{}", legend_title(i, &sans));
            assert!(!legend_title(i, &avec).contains(ESTIME), "{}", legend_title(i, &avec));
        }
        let incomplet = [argos_core::orderflow::FlowBar {
            complete: false,
            ..reel[0].clone()
        }];
        let partiel = compute(&b, 1800, &on, &Settings::new(), None, 0, &Default::default(), &incomplet);
        assert!(legend_title(Ind::Footprint, &partiel).contains(ESTIME), "incomplete bars only: still estimated");
        // on a phone the legend is shortened from the end: the name and the warning fit in 390 pt
        let titre = format!("{} · {ESTIME}", Ind::Footprint.name());
        let large = crate::text::Text::default().width(&titre, 13.0);
        assert!(large < 390.0 - 16.0, "fits on a phone: {titre} ({large} px)");
        assert_eq!(Ind::Cvd.name(), "CVD");
        assert!(Ind::Cvd.oscillator() && Ind::ALL.contains(&Ind::Cvd));
    }
}
