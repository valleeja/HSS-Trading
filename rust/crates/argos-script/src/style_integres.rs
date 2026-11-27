//! 🎨 The « Style » tab of the built-in indicators written in Argos Script (owner's decision, 19 Nov 2026): for each
//! `plot`, `fill` and `hline` of the script, its colour, width and line type — the same settings the classic natives
//! have (`classic_inputs`), nothing more. Their defaults are what the script's own calls say, so an indicator with no
//! style set draws exactly as before.
//!
//! These settings are not inputs: the script runs with its `input()` values only, and the style is applied when
//! drawing (`argos_render::scriptview`). They come after the inputs in the indicator's table (see
//! `catalogue::reglages`), in their own group « 🎨 Style », and are saved by name like the other settings.

use crate::classic_inputs::TRAITS;
use crate::inputs::{choice, color, num, Param};
use crate::lang::value::HandleKind;
use crate::lang::{InputSpec, Output, PlotStyle, Tracer};
use std::collections::HashMap;
use std::sync::Mutex;

/// The group of the style settings.
pub const GROUPE: &str = "🎨 Style";

/// The line types of `TRAITS` (Plein, Tirets, Points) as the drawing names them.
const LIGNES: [&str; 3] = ["solid", "dashed", "dotted"];

/// What a style setting changes: a plot, a fill or a hline (its index in the script's output).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cible {
    Plot(usize),
    Fill(usize),
    Hline(usize),
}

/// Which property of its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prop {
    /// The colour (with its transparency, as Pine: one value).
    Couleur,
    Epaisseur,
    /// The line type (index in `TRAITS`).
    Trait,
}

/// The style chosen for one plot / fill / hline: `None` keeps the script's own.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Trace {
    pub couleur: Option<u32>,
    pub epaisseur: Option<f64>,
    /// « solid », « dashed » or « dotted ».
    pub ligne: Option<&'static str>,
}

/// The style chosen for a script's drawing (empty: the script's own everywhere).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Style {
    pub plots: Vec<Trace>,
    pub fills: Vec<Trace>,
    pub hlines: Vec<Trace>,
}

impl Style {
    pub fn plot(&self, k: usize) -> Trace {
        self.plots.get(k).copied().unwrap_or_default()
    }
    pub fn fill(&self, k: usize) -> Trace {
        self.fills.get(k).copied().unwrap_or_default()
    }
    pub fn hline(&self, k: usize) -> Trace {
        self.hlines.get(k).copied().unwrap_or_default()
    }
}

/// The full settings table of a built-in script: its inputs, then its style settings.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Table {
    pub groupes: &'static [&'static str],
    pub params: &'static [Param],
    /// How many inputs come first (the style settings follow).
    pub entrees: usize,
    /// What each style setting changes, in table order (after the inputs).
    pub style: &'static [(Cible, Prop)],
}

/// A string that lives as long as the program (one table per built-in script).
fn fixe(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

/// The bars of the reference run: varied (prices, volume), so a colour that depends on the bars shows it is not
/// constant (Spring Tension's ribbon), and long enough for every average to be drawn.
fn barres_de_reference() -> Vec<argos_core::Bar> {
    (0..300)
        .map(|i| {
            let x = i as f64;
            let c = 100.0 + 10.0 * (x * 0.17).sin() + 3.0 * (x * 0.61).sin() + x * 0.05;
            let o = c - 2.0 * (x * 0.43).cos();
            argos_core::Bar {
                time: i * 60,
                open: o,
                high: c.max(o) + 1.0 + (x * 0.9).sin().abs(),
                low: c.min(o) - 1.0 - (x * 1.3).cos().abs(),
                close: c,
                volume: 50.0 + 40.0 * (x * 0.37).sin().abs() + (i % 7) as f64 * 5.0,
            }
        })
        .collect()
}

/// The single colour of a series of colours (`None`: it changes from bar to bar, or is never set).
fn couleur_fixe(c: &[Option<u32>]) -> Option<u32> {
    let mut v = c.iter().flatten();
    let premiere = *v.next()?;
    v.all(|x| *x == premiere).then_some(premiere)
}

/// The name of a plot or hline, as its style settings show it.
fn nom(out: &Output, t: Tracer) -> String {
    match t {
        (HandleKind::Hline, k) => nom_hline(out, k),
        (_, k) => out.plots.get(k).map_or_else(String::new, |p| p.title.clone()),
    }
}

fn nom_hline(out: &Output, k: usize) -> String {
    match out.hlines.get(k) {
        Some(h) if !h.1.is_empty() => h.1.clone(),
        _ => format!("Ligne horizontale {}", k + 1),
    }
}

/// The value of a plot or hline at bar `i` (a plot's own bar, without its offset: enough to know it is drawn).
fn valeur(out: &Output, t: Tracer, i: usize) -> f64 {
    match t {
        (HandleKind::Hline, k) => out.hlines.get(k).map_or(f64::NAN, |h| h.0),
        (_, k) => out.plots.get(k).and_then(|p| p.values.get(i).copied()).unwrap_or(f64::NAN),
    }
}

/// The style settings of a script's output: (target, property, key, label, default). Only what changes the drawing:
/// a colour that is the same on every bar (a colour that varies is the script's own, often from its colour inputs), a
/// width except for columns (drawn as wide as a bar), a line type for lines only; a fill only if it is drawn in the
/// reference run (else its colour there is not the one it is drawn with).
fn reglages_de(out: &Output) -> Vec<(Cible, Prop, String, String, f64)> {
    let mut v = vec![];
    let mut trace = |cible: Cible, titre: &str, couleur: Option<u32>, epaisseur: Option<f64>, ligne: Option<usize>| {
        if let Some(c) = couleur {
            v.push((cible, Prop::Couleur, format!("{titre} · couleur"), format!("Couleur · {titre}"), c as f64));
        }
        if let Some(e) = epaisseur {
            v.push((cible, Prop::Epaisseur, format!("{titre} · épaisseur"), format!("Épaisseur · {titre}"), e));
        }
        if let Some(l) = ligne {
            v.push((
                cible,
                Prop::Trait,
                format!("{titre} · type de ligne"),
                format!("Type de ligne · {titre}"),
                l as f64,
            ));
        }
    };
    for (k, p) in out.plots.iter().enumerate() {
        let ligne = matches!(p.style, PlotStyle::Line | PlotStyle::LineBr | PlotStyle::StepLine | PlotStyle::Area);
        trace(
            Cible::Plot(k),
            &p.title,
            couleur_fixe(&p.colors),
            (p.style != PlotStyle::Columns).then_some(p.width),
            ligne.then_some(0),
        );
    }
    for (k, (a, b, couleurs)) in out.fills.iter().enumerate() {
        let dessine = couleurs
            .iter()
            .enumerate()
            .any(|(i, c)| c.is_some() && valeur(out, *a, i).is_finite() && valeur(out, *b, i).is_finite());
        if dessine {
            let titre = format!("Remplissage {} – {}", nom(out, *a), nom(out, *b));
            trace(Cible::Fill(k), &titre, couleur_fixe(couleurs), None, None);
        }
    }
    for (k, h) in out.hlines.iter().enumerate() {
        // (a hline is drawn 1 px wide, in its own line style: « dashed », « dotted », else solid)
        let ligne = LIGNES.iter().position(|l| *l == h.3).unwrap_or(0);
        trace(Cible::Hline(k), &nom_hline(out, k), Some(h.2), Some(1.0), Some(ligne));
    }
    v
}

/// The settings table of the built-in script `src` with the inputs `specs` (made once per script and kept).
pub(crate) fn table(src: &'static str, specs: &[InputSpec]) -> Table {
    static TABLES: Mutex<Option<HashMap<usize, Table>>> = Mutex::new(None);
    let mut t = TABLES.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(x) = t.get_or_insert_with(HashMap::new).get(&(src.as_ptr() as usize)) {
        return *x;
    }
    let (groupes_entrees, entrees) = crate::lang::reglages::table(specs);
    let style = crate::lang::run(src, &barres_de_reference(), 60, &HashMap::new()).map_or_else(|_| vec![], |o| reglages_de(&o));
    let x = if style.is_empty() {
        Table {
            groupes: groupes_entrees,
            params: entrees,
            entrees: entrees.len(),
            style: &[],
        }
    } else {
        let g = groupes_entrees.len();
        let mut params = entrees.to_vec();
        let mut cibles = vec![];
        for (cible, prop, key, label, def) in style {
            // (a key already taken in the table gets « key@k », k its position: the rule of `catalogue::par_nom`)
            let key = if params.iter().any(|p| p.key == key) {
                format!("{key}@{}", params.len())
            } else {
                key
            };
            let (key, label) = (fixe(key), fixe(label));
            // (not shown in the legend, like the classic natives')
            let p = match prop {
                Prop::Couleur => color(key, g, label, 0),
                Prop::Epaisseur => num(key, g, label, 0.0, 1.0, def.max(4.0), 0.5).display(false),
                Prop::Trait => choice(key, g, label, 0, &TRAITS).display(false),
            };
            params.push(Param { def, ..p });
            cibles.push((cible, prop));
        }
        let mut groupes = groupes_entrees.to_vec();
        groupes.push(GROUPE);
        Table {
            groupes: Box::leak(groupes.into_boxed_slice()),
            params: Box::leak(params.into_boxed_slice()),
            entrees: entrees.len(),
            style: Box::leak(cibles.into_boxed_slice()),
        }
    };
    t.get_or_insert_with(HashMap::new).insert(src.as_ptr() as usize, x);
    x
}

/// The style chosen in the ⚙ of the built-in script `src` from its values `vals` (inputs, then style, in table
/// order): only the settings that differ from the script's own (none: an empty style, the drawing as before).
pub fn style(src: &'static str, vals: &[f64]) -> Style {
    let t = table(src, &crate::catalogue::entrees(src));
    let mut s = Style::default();
    for (k, (cible, prop)) in t.style.iter().enumerate() {
        let p = &t.params[t.entrees + k];
        let Some(v) = vals.get(t.entrees + k).copied().filter(|v| *v != p.def) else {
            continue;
        };
        let (liste, i) = match *cible {
            Cible::Plot(i) => (&mut s.plots, i),
            Cible::Fill(i) => (&mut s.fills, i),
            Cible::Hline(i) => (&mut s.hlines, i),
        };
        if liste.len() <= i {
            liste.resize(i + 1, Trace::default());
        }
        match prop {
            Prop::Couleur => liste[i].couleur = Some(v as u32),
            Prop::Epaisseur => liste[i].epaisseur = Some(v),
            Prop::Trait => liste[i].ligne = Some(LIGNES[(v as usize).min(LIGNES.len() - 1)]),
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogue::{self, fiche, reglages};
    use crate::inputs::Kind;

    /// The style settings of a fiche: (key, label, default, kind) after its inputs.
    fn style_de(id: &str) -> Vec<(&'static str, &'static str, f64, Kind)> {
        let f = fiche(id).unwrap();
        let src = catalogue::source(f).unwrap();
        let n = catalogue::entrees(src).len();
        reglages(f).1[n..].iter().map(|p| (p.key, p.label, p.def, p.kind)).collect()
    }

    /// 🎨 Keys by the plot's title (« MA · couleur », « MA · épaisseur », « MA · type de ligne »), labels like
    /// the classic natives' (« Couleur · MA »…), defaults from the script's own `plot` (colour, width 1, solid), in the group
    /// « 🎨 Style »; the same key on every call (stable).
    #[test]
    fn cles_et_defauts_du_script() {
        let s = style_de("Wma");
        let cles: Vec<&str> = s.iter().map(|x| x.0).collect();
        assert_eq!(cles, ["WMA · couleur", "WMA · épaisseur", "WMA · type de ligne"]);
        let labels: Vec<&str> = s.iter().map(|x| x.1).collect();
        assert_eq!(labels, ["Couleur · WMA", "Épaisseur · WMA", "Type de ligne · WMA"]);
        assert_eq!(s[0].2 as u32, 0x2962ff, "plot(color = #2962FF)");
        assert!(matches!(s[0].3, Kind::Color));
        assert_eq!(s[1].2, 1.0, "no linewidth: 1");
        assert!(matches!(s[2].3, Kind::Choice(o) if o == TRAITS) && s[2].2 == 0.0, "solid");
        assert_eq!(style_de("Wma"), s, "stable");
        let (groupes, params) = reglages(fiche("Wma").unwrap());
        assert_eq!(groupes.last(), Some(&GROUPE));
        assert!(params
            .iter()
            .filter(|p| p.key.contains(" · "))
            .all(|p| p.group == groupes.len() - 1 && !p.display));
        // the ribbon: the four lines, each its own colour
        let r = style_de("MaRibbon");
        assert_eq!(r.len(), 12);
        let couleurs: Vec<u32> = r.iter().filter(|x| x.0.ends_with(" · couleur")).map(|x| x.2 as u32).collect();
        assert_eq!(couleurs, [0xf6c309, 0xfb9800, 0xfb6500, 0xf60c0c]);
        assert_eq!(r[0].0, "MA #1 · couleur");
    }

    /// 🎨 A colour that varies from bar to bar (Spring Tension's ribbon, from its colour inputs) and columns (as wide
    /// as a bar) get no style setting: no button that does nothing. The SMA's Bollinger fill has no colour in the
    /// default run (« None » type: it is not drawn): no setting for it either; its lines have theirs.
    #[test]
    fn rien_qui_ne_fait_rien() {
        assert!(style_de("SpringTension").is_empty());
        let (groupes, _) = reglages(fiche("SpringTension").unwrap());
        assert!(!groupes.contains(&GROUPE));
        let sma: Vec<&str> = style_de("Sma").iter().map(|x| x.0).collect();
        assert!(sma.contains(&"Upper Bollinger Band · couleur") && sma.contains(&"SMA-based MA · type de ligne"));
        assert!(!sma.iter().any(|k| k.starts_with("Remplissage")));
    }

    /// 🎨 A title met twice: the second key gets « @position » (the rule of `catalogue::par_nom`), its label the plain
    /// title; `par_nom` keeps the keys as they are.
    #[test]
    fn titre_en_double() {
        const SRC: &str = "//@version=6\nindicator(\"Deux\", overlay = true)\nlen = input.int(5, \"Length\")\nplot(ta.sma(close, len), \"MA\", color = color.red)\nplot(ta.ema(close, len), \"MA\", color = color.blue, linewidth = 2)\nh = hline(100, \"Niveau\", color = color.gray, linestyle = hline.style_dotted)\nfill(plot(close, \"C\"), h, color = color.new(color.green, 80))\n";
        let specs = crate::lang::run(SRC, &barres_de_reference(), 60, &HashMap::new()).unwrap().inputs;
        let t = table(SRC, &specs);
        let cles: Vec<&str> = t.params.iter().map(|p| p.key).collect();
        assert_eq!(
            cles,
            [
                "Length",
                "MA · couleur",
                "MA · épaisseur",
                "MA · type de ligne",
                "MA · couleur@4",
                "MA · épaisseur@5",
                "MA · type de ligne@6",
                "C · couleur",
                "C · épaisseur",
                "C · type de ligne",
                "Remplissage C – Niveau · couleur",
                "Niveau · couleur",
                "Niveau · épaisseur",
                "Niveau · type de ligne",
            ]
        );
        assert_eq!(t.params[4].label, "Couleur · MA");
        assert_eq!(t.params[5].def, 2.0, "linewidth = 2");
        let fill = t.params.iter().find(|p| p.key.starts_with("Remplissage")).unwrap();
        assert_eq!(crate::lang::value::rgba(fill.def as u32).3, 80, "the fill's transparency kept in its colour");
        assert_eq!(t.params.last().unwrap().def, 2.0, "hline.style_dotted: « Points »");
        let vals: Vec<f64> = t.params.iter().map(|p| p.def).collect();
        let noms: Vec<String> = catalogue::par_nom(t.params, &vals).into_keys().collect();
        assert!(noms.contains(&"MA · couleur@4".to_string()) && !noms.iter().any(|n| n.contains("@4@")));
    }

    /// 🎨 Only what differs from the script's own becomes a style: nothing set, an empty style; a colour, a width and
    /// a line type set, on their plot.
    #[test]
    fn style_choisi() {
        let f = fiche("MaRibbon").unwrap();
        let src = catalogue::source(f).unwrap();
        let params = reglages(f).1;
        let defauts: Vec<f64> = params.iter().map(|p| p.def).collect();
        assert_eq!(style(src, &defauts), Style::default(), "nothing set: the script's own");
        let mut v = defauts.clone();
        let k = |cle: &str| params.iter().position(|p| p.key == cle).unwrap();
        v[k("MA #2 · couleur")] = 0x00ff00 as f64;
        v[k("MA #2 · épaisseur")] = 3.0;
        v[k("MA #2 · type de ligne")] = 1.0;
        let s = style(src, &v);
        assert_eq!(s.plot(0), Trace::default());
        assert_eq!(
            s.plot(1),
            Trace {
                couleur: Some(0x00ff00),
                epaisseur: Some(3.0),
                ligne: Some("dashed"),
            }
        );
    }
}
