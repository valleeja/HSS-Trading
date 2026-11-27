//! 🎯 The chart image kept while only the mouse moves (vitesse-1). The chart is drawn once without its mouse layer
//! (`argos_render::render_fond`: candles, indicators, drawings…), then each frame a copy of it is finished
//! (`argos_render::finir`: the crosshair layer, then the pill, bottom bar, title, menus over it). `render` is exactly
//! `render_fond` then `finir`, so the image is the same as before. Anything else that changes the image (bars,
//! indicators, view, settings, drawings, the clock, the theme, the language, the glass, an emoji arriving…) draws
//! it again. A long / short position drawn lights up under the mouse: then the chart is drawn whole each frame, as
//! before.

use argos_render::bookheat::BookHeat;
use argos_render::indicators::Ind;
use argos_render::liqview::LiqView;
use argos_render::{Chart, Reticule};
use tiny_skia::Pixmap;

/// The image kept, its mouse layer, and what it was drawn from.
pub(crate) struct Fond {
    key: Key,
    pm: Pixmap,
    layer: Option<Reticule>,
}

/// Everything the image under the mouse layer is drawn from: the chart without the mouse, owned copies of what it
/// borrows, and the settings the drawing reads outside the chart.
struct Key {
    chart: Chart<'static>,
    bars: Vec<argos_core::Bar>,
    ind: Option<IndSig>,
    heat: Option<BookHeat>,
    liq: Option<LiqView>,
    /// 🤖 The explanation's drawings (Argos AI level 1).
    explication: Vec<argos_lens::explain::Command>,
    globals: Globals,
}

/// What the indicator results come from: the moment they were computed (`App::refresh_indicators`, the only place
/// that makes them) and what is changed in them afterwards (hidden ones, favourites, scripts hidden). Their numbers
/// are not compared: they hold NaN (warm-up), never equal to itself.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct IndSig {
    pub(crate) at: web_time::Instant,
    pub(crate) hidden: Vec<Ind>,
    pub(crate) favs: Vec<Ind>,
    pub(crate) scripts: Vec<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Globals {
    scale: f32,
    theme: usize,
    glass: (bool, usize, bool),
    lang: usize,
    numbers: usize,
    emojis: usize,
    /// (🧭 the Explorer's lines of an address, set from outside the chart)
    dehors: usize,
    /// 📋 The tables' layouts (moved / resized): drawn again when they change.
    tableaux: u64,
    /// ✏️ Drawn as a draft (a finger moves the chart): drawn again sharp at rest.
    brouillon: bool,
}

fn globals() -> Globals {
    use argos_render::glass;
    Globals {
        scale: argos_render::px::scale(),
        theme: argos_render::theme_index(),
        glass: (glass::on(), glass::level(), glass::fige()),
        lang: argos_render::i18n::version(),
        numbers: argos_render::viewport::number_format(),
        emojis: argos_render::emoji::arrivees(),
        dehors: argos_render::px::fond_revision(),
        tableaux: argos_render::tableaux::version(),
        brouillon: argos_render::px::brouillon(),
    }
}

/// The chart without the mouse and without what it borrows (compared apart). Every field is named: a new field of
/// `Chart` does not compile here until it is taken into account.
fn sans_souris(c: &Chart) -> Chart<'static> {
    let Chart {
        bars: _,
        tf_seconds,
        viewport,
        theme,
        style,
        tz_offset_s,
        title,
        crosshair: _,
        banner,
        settings,
        context,
        drawings,
        selected,
        now_s,
        live,
        hors_ligne,
        legende,
        navigation,
        goto,
        indicators: _,
        mark,
        whales,
        events,
        alerts,
        alertes_vue,
        ordres,
        demo,
        bid_ask,
        book_heat: _,
        book_note,
        murs,
        liq_map: _,
        pad,
        trades,
        explication: _,
        garder,
        garder_pilules,
        boutons_bas,
    } = c.clone();
    Chart {
        bars: &[],
        tf_seconds,
        viewport,
        theme,
        style,
        tz_offset_s,
        title,
        crosshair: None,
        banner,
        settings,
        context,
        drawings,
        selected,
        now_s,
        live,
        hors_ligne,
        legende,
        navigation,
        goto,
        indicators: None,
        mark,
        whales,
        events,
        alerts,
        alertes_vue,
        ordres,
        demo,
        bid_ask,
        book_heat: None,
        book_note,
        murs,
        liq_map: None,
        pad,
        trades,
        explication: &[],
        garder,
        garder_pilules,
        boutons_bas,
    }
}

impl Key {
    fn of(c: &Chart, ind: &IndSig, globals: Globals) -> Key {
        Key {
            chart: sans_souris(c),
            bars: c.bars.to_vec(),
            ind: c.indicators.map(|_| ind.clone()),
            heat: c.book_heat.cloned(),
            liq: c.liq_map.cloned(),
            explication: c.explication.to_vec(),
            globals,
        }
    }

    fn same(&self, c: &Chart, ind: &IndSig, globals: Globals) -> bool {
        self.globals == globals
            && self.bars[..] == *c.bars
            && self.ind.as_ref() == c.indicators.map(|_| ind)
            && self.heat.as_ref() == c.book_heat
            && self.liq.as_ref() == c.liq_map
            && self.explication[..] == *c.explication
            && self.chart == sans_souris(c)
    }
}

impl Fond {
    /// 🧪 Where the kept image lives (the same place: it was kept, not drawn again).
    #[cfg(test)]
    pub(crate) fn place(&self) -> *const u8 {
        self.pm.data().as_ptr()
    }
}

/// The image under the mouse layer follows the mouse: a long / short position lights up under it.
fn suit_la_souris(c: &Chart) -> bool {
    c.drawings.iter().any(|d| d.tool == "long" || d.tool == "short")
}

/// The chart image for `c` (its indicator results: `ind`), the same as `argos_render::render(c)`: the kept image
/// finished for the mouse when nothing else changed, else drawn again (and kept).
pub(crate) fn image(kept: &mut Option<Fond>, c: &Chart, ind: &IndSig) -> Pixmap {
    if suit_la_souris(c) {
        *kept = None;
        return argos_render::render(c);
    }
    let g = globals();
    if !kept.as_ref().is_some_and(|f| f.key.same(c, ind, g)) {
        let (pm, layer) = argos_render::render_fond(c);
        *kept = Some(Fond {
            key: Key::of(c, ind, g),
            pm,
            layer,
        });
    }
    let Some(f) = kept.as_ref() else {
        return argos_render::render(c);
    };
    let mut pm = f.pm.clone();
    argos_render::finir(&mut pm, c, f.layer.as_ref());
    pm
}
