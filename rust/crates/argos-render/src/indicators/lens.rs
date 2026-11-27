//! 🔎 Argos Lens sur la chart : la photo de la dernière bougie clôturée (régime, scores, setup) et le dernier BOS.
//! Seulement des bougies clôturées : la bougie en cours ne compte jamais. Affichage seul, aucun ordre.

use crate::jetons;
use crate::palette as pal;
use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Mono, Rgb, Text};
use argos_core::Bar;
use argos_lens::{decimales_prix, nombre_fr, nouveaux_setups, photos, reglages, Bias, Decimal, Direction, LensError, Regime, Setup, Snapshot};
use argos_script::indicators::smc;
use tiny_skia::Pixmap;

/// Bougies analysées au plus (les plus récentes), pour garder un calcul rapide à chaque mise à jour.
const MAX_BARS: usize = 1500;

/// Ce que la carte et les repères affichent.
#[derive(Debug, Clone, PartialEq)]
pub struct LensVue {
    /// Dernière bougie clôturée : son index dans les bougies de la chart et sa photo.
    pub photo: Option<(usize, Snapshot)>,
    /// Setup en cours : bougie (index dans la chart) où il est apparu, et le setup de cette bougie.
    pub setup: Option<(usize, Setup)>,
    /// Dernier BOS (index dans la chart).
    pub bos: Option<smc::StructureBreak>,
    /// Calcul refusé (poids dont la somme ne fait pas 100, bougies absurdes).
    pub erreur: Option<LensError>,
}

/// Calcul sur les bougies clôturées à `now` (secondes) ; `vals` : réglages de la fenêtre (`reglages::PARAMS`).
pub fn compute(bars: &[Bar], tf_seconds: i64, now: i64, vals: &[f64]) -> LensVue {
    let closed = bars.partition_point(|b| b.time.saturating_add(tf_seconds) <= now);
    let start = closed.saturating_sub(MAX_BARS);
    let b = &bars[start..closed];
    let ph = match photos(b, tf_seconds, &reglages::config(vals), None) {
        Ok(ph) => ph,
        Err(e) => {
            return LensVue {
                photo: None,
                setup: None,
                bos: None,
                erreur: Some(e),
            }
        }
    };
    let last = ph.last().cloned();
    // le setup en cours a commencé au dernier setup nouveau, s'il tient encore (même sens) sur la dernière photo
    let side = last.as_ref().and_then(|p| p.setup).map(|s| s.side);
    let setup = nouveaux_setups(&ph)
        .into_iter()
        .enumerate()
        .rev()
        .find_map(|(i, s)| s.map(|s| (start + i, s)))
        .filter(|(_, s)| Some(s.side) == side);
    let bos = smc::core(b, &smc::Settings::default())
        .breaks
        .into_iter()
        .rev()
        .find(|x| x.kind == smc::BreakType::Bos)
        .map(|x| smc::StructureBreak {
            i0: start + x.i0,
            i1: start + x.i1,
            ..x
        });
    LensVue {
        photo: last.map(|p| (closed - 1, p)),
        setup,
        bos,
        erreur: None,
    }
}

// — 🫧 la carte en verre « Argos Lens » (maquette Lens : en haut à gauche de la chart) —

const PAD: f64 = 14.0;
const T: f32 = 13.0;
const T_MONO: f32 = 12.0;
const LIGNE: f64 = 18.0;
/// Seuil d'un setup (trait jaune sur les jauges).
const SEUIL: f64 = 0.70;
/// Largeur d'écran à partir de laquelle la carte passe en bas de la chart, sur toute la largeur (maquette : ≤ 640 px).
const ETROIT: f64 = 640.0;
const PIED: &str = "Analyse affichée, pas un ordre. Taille de position : Gardien PropFirm.";

fn regime(r: Regime) -> (&'static str, Rgb) {
    match r {
        Regime::TrendUp => ("TENDANCE ↑", pal::up()),
        Regime::TrendDown => ("TENDANCE ↓", pal::down()),
        Regime::Range => ("RANGE", jetons::ACCENT),
        Regime::HighVolatility => ("HAUTE VOLAT.", jetons::TIER2),
    }
}

/// Coupe `s` en lignes qui tiennent dans `w` points.
fn couper(text: &Text, s: &str, w: f64, size: f32) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for mot in s.split(' ') {
        match out.last_mut() {
            Some(l) if text.width(&format!("{l} {mot}"), size) as f64 <= w => {
                l.push(' ');
                l.push_str(mot);
            }
            _ => out.push(mot.to_string()),
        }
    }
    out
}

/// Les lignes de texte de la carte, avant le dessin (sert aussi à calculer sa hauteur).
struct Contenu {
    regime: Option<(&'static str, Rgb)>,
    scores: Option<(u8, u8)>,
    /// (titre, couleur, lignes (libellé, prix, couleur)).
    setup: Option<(String, Rgb, Vec<(&'static str, String, Rgb)>)>,
    /// Le setup sur une ligne (carte compacte) : « Setup LONG · Entrée … · Stop … », plus court s'il ne tient pas.
    setup_court: Option<(String, Rgb)>,
    puces: Vec<String>,
    /// Message à la place des scores (calcul refusé, pas assez de bougies).
    message: Option<String>,
    pied: Vec<String>,
}

fn contenu(text: &Text, v: &LensVue, bars: &[Bar], biais: &str, w: f64, tz: i64) -> Contenu {
    let photo = v.photo.as_ref().map(|p| &p.1);
    let message = match (&v.erreur, photo.and_then(|p| p.regime)) {
        (Some(e), _) => Some(e.to_string()),
        (None, None) => Some("Pas encore assez de bougies clôturées.".to_string()),
        _ => None,
    };
    let setup = v.setup.map(|(i, s)| {
        let (nom, c) = if s.side == Direction::Long {
            ("LONG", pal::up())
        } else {
            ("SHORT", pal::down())
        };
        // heure de clôture de la bougie du setup (fuseau de la chart) = début de la bougie suivante, ou début + durée d'avant
        let fin = match (bars.get(i), bars.get(i + 1), i.checked_sub(1).and_then(|k| bars.get(k))) {
            (_, Some(n), _) => Some(n.time),
            (Some(b), None, Some(p)) => Some(2 * b.time - p.time),
            _ => None,
        };
        let heure = fin
            .map(|t| t + tz)
            .map_or(String::new(), |t| format!(" {:02}:{:02}", t.rem_euclid(86_400) / 3600, t.rem_euclid(3600) / 60));
        let dp = decimales_prix(s.entry);
        let lignes = vec![
            ("Entrée", nombre_fr(s.entry, dp), jetons::TEXTE),
            ("Stop", nombre_fr(s.stop, dp), pal::down()),
            ("Cible 1 (1,5R)", nombre_fr(s.tp1, dp), pal::up()),
            ("Cible 2 (3R)", nombre_fr(s.tp2, dp), pal::up()),
        ];
        (format!("Setup {nom} · bougie clôturée{heure}"), c, lignes)
    });
    let setup_court = v.setup.zip(setup.as_ref()).map(|((_, s), (_, c, lignes))| {
        let nom = if s.side == Direction::Long { "LONG" } else { "SHORT" };
        let long = format!("Setup {nom} · Entrée {} · Stop {}", lignes[0].1, lignes[1].1);
        let court = format!("{nom} · E {} · S {}", lignes[0].1, lignes[1].1);
        (if text.width(&long, T_MONO) as f64 <= w - 2.0 * PAD { long } else { court }, *c)
    });
    let puces = photo
        .map(|p| {
            let n = |d: Option<Decimal>, dp: u32| d.map_or("—".to_string(), |d| nombre_fr(d, dp));
            let fleche = match p.htf_bias {
                Some(Bias::Up) => "↑",
                Some(Bias::Down) => "↓",
                Some(Bias::Neutral) => "→",
                None => "—",
            };
            let mut v = vec![
                format!("ADX {}", n(p.adx14, 0)),
                format!("RSI {}", n(p.rsi14, 0)),
                format!("ATR p{}", n(p.atr_pct, 0)),
                format!("Vol ×{}", n(p.rel_volume, 1)),
            ];
            if !biais.is_empty() {
                v.push(format!("Biais {biais} {fleche}"));
            }
            v
        })
        .unwrap_or_default();
    Contenu {
        regime: photo.and_then(|p| p.regime).map(regime),
        scores: photo.filter(|_| message.is_none()).map(|p| (p.score_long, p.score_short)),
        setup: setup.filter(|_| message.is_none()),
        setup_court: setup_court.filter(|_| message.is_none()),
        puces: if message.is_none() { puces } else { vec![] },
        message,
        pied: couper(text, PIED, w - 2.0 * PAD, T_MONO),
    }
}

/// Mini-pastilles rangées sur des lignes de largeur `w` : (texte, x, rang de ligne).
fn ranger_puces(text: &Text, puces: &[String], w: f64) -> Vec<(String, f64, usize, f64)> {
    let (mut x, mut rang, mut out) = (0.0, 0usize, vec![]);
    for p in puces {
        let pw = text.width(p, T_MONO) as f64 + 14.0;
        if x > 0.0 && x + pw > w {
            (x, rang) = (0.0, rang + 1);
        }
        out.push((p.clone(), x, rang, pw));
        x += pw + 5.0;
    }
    out
}

/// Les tailles de la carte, de la plus complète à la plus petite : la plus grande qui tient entière est prise
/// (jamais une ligne coupée, jamais sous les bandes d'oscillateurs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Palier {
    /// Tout : jauges, bloc setup, mini-pastilles, phrase grise.
    Complete,
    /// Jauges, setup sur une ligne, mini-pastilles sur une ligne, sans la phrase grise.
    Compacte,
    /// Comme la compacte, sans les mini-pastilles (titre, régime, jauges, setup sur une ligne).
    Jauges,
    /// Une seule pastille : « Argos Lens · régime · L 20 · S 65 ».
    Pastille,
}

/// Écart gardé au-dessus des bandes d'oscillateurs.
const MARGE: f64 = 8.0;
const PASTILLE_H: f64 = 28.0;

/// Texte de la pastille (palier le plus petit).
fn texte_pastille(c: &Contenu) -> String {
    match (c.regime, c.scores) {
        (Some((r, _)), Some((l, s))) => format!("Argos Lens · {r} · L {l} · S {s}"),
        _ => "Argos Lens · —".to_string(),
    }
}

/// Hauteur de la carte à ce palier.
fn hauteur(text: &Text, c: &Contenu, w: f64, palier: Palier) -> f64 {
    let tete = PAD + LIGNE + 6.0 + if c.scores.is_some() { 2.0 * LIGNE + 4.0 } else { LIGNE };
    match palier {
        Palier::Complete => {
            let rangs = ranger_puces(text, &c.puces, w - 2.0 * PAD).last().map_or(0, |p| p.2 + 1);
            let setup = c.setup.as_ref().map_or(0.0, |(_, _, l)| 10.0 + (l.len() + 1) as f64 * 16.0 + 12.0);
            tete + setup + rangs as f64 * 24.0 + 4.0 + c.pied.len() as f64 * 15.0 + PAD
        }
        Palier::Compacte | Palier::Jauges => {
            let setup = if c.setup.is_some() { LIGNE + 4.0 } else { 0.0 };
            let puces = if c.puces.is_empty() || palier == Palier::Jauges { 0.0 } else { 24.0 };
            tete + setup + puces + PAD - 4.0
        }
        Palier::Pastille => PASTILLE_H,
    }
}

/// Place et taille de la carte : en haut à gauche de la zone de prix à partir de `haut` (sous la liste des
/// indicateurs), ou en bas sur toute la largeur quand l'écran est étroit ; toujours entière dans la zone de prix
/// (`price_h` : bas de la zone, au-dessus des bandes). `None` : même la pastille n'a pas la place.
#[cfg(test)]
pub(super) fn place_carte(text: &Text, v: &LensVue, bars: &[Bar], biais: &str, plot_w: f64, price_h: f64, haut: f64) -> Option<(Rect, Palier)> {
    place_carte_de(text, v, bars, biais, plot_w, price_h, haut, (0, 1.0)).map(|(r, p, _)| (r, p))
}

/// Place de la carte pour un emplacement et une échelle (`carte` : réglages « Emplacement » et « Taille ») : sa place
/// sur la chart (déjà réduite), son palier, et sa largeur avant réduction. Les 4 paliers restent valables partout : la
/// plus grande taille qui tient entière (réduite) est prise. En haut : sous la légende (`haut`) ; en bas à droite : au
/// dessus de la place du bouton Argos AI.
#[allow(clippy::too_many_arguments)]
pub(super) fn place_carte_de(
    text: &Text,
    v: &LensVue,
    bars: &[Bar],
    biais: &str,
    plot_w: f64,
    price_h: f64,
    haut: f64,
    carte: (usize, f64),
) -> Option<(Rect, Palier, f64)> {
    let (coin, k) = carte;
    let etroit = plot_w <= ETROIT;
    let w = if etroit { (plot_w - 24.0) / k } else { 330f64.min((plot_w - 24.0) / k) };
    let c = contenu(text, v, bars, biais, w, 0);
    // Auto : comme avant (haut gauche ; écran étroit : en bas sur toute la largeur)
    let coin = if coin == 0 {
        if etroit {
            3
        } else {
            1
        }
    } else {
        coin
    };
    let en_haut = coin <= 2;
    let a_droite = coin == 2 || coin == 4;
    // (le bas de la légende des indicateurs, `haut`, ne compte qu'à gauche : à droite, sous la ligne du titre et de la
    // pilule DÉMO seulement)
    let haut = if etroit {
        52.0
    } else if a_droite {
        HAUT_DROITE
    } else {
        haut
    };
    let bas = if etroit {
        price_h - 40.0
    } else {
        price_h - MARGE - if coin == 4 { BOUTON_AI } else { 0.0 }
    };
    [Palier::Complete, Palier::Compacte, Palier::Jauges, Palier::Pastille]
        .into_iter()
        .find_map(|palier| {
            let h = hauteur(text, &c, w, palier);
            let wl = match palier {
                Palier::Pastille => (text.width(&texte_pastille(&c), T_MONO) as f64 + 24.0).min(w),
                _ => w,
            };
            let (ww, hh) = (wl * k, h * k);
            let x = if a_droite { plot_w - 12.0 - ww } else { 12.0 };
            let mut y = if en_haut { haut } else { bas - hh };
            // 🖥 jamais sous la barre de favoris : la carte se pose sous elle (en haut) ou au-dessus (en bas)
            for e in super::eviter() {
                if x < e.x + e.w && e.x < x + ww && y < e.y + e.h && e.y < y + hh {
                    y = if en_haut { y.max(e.y + e.h + 8.0) } else { y.min(e.y - 8.0 - hh) };
                }
            }
            (y >= haut - 0.01 && y + hh <= bas + 0.01 && wl > 0.0).then_some((Rect::new(x, y, ww, hh), palier, wl))
        })
}

/// Haut de la carte en haut à droite (la légende et le titre sont à gauche ; la pilule DÉMO est dans la barre du haut).
const HAUT_DROITE: f64 = 18.0;

/// Place gardée en bas à droite pour le bouton rond Argos AI (68 et sa marge).
const BOUTON_AI: f64 = 90.0;

/// Text of the button that opens Argos AI's explanation of the card.
const EXPLIQUE: &str = "Explique";
/// Smallest touch area of the « Explique » button for a finger (points, width and height).
pub const TOUCHER_MIN: f64 = 40.0;

thread_local! {
    /// The card and its « Explique » button at the last drawing (chart coordinates).
    static BOUTON: std::cell::Cell<Option<(Rect, Rect)>> = const { std::cell::Cell::new(None) };
}

/// Width of the regime pill.
fn regime_w(nom: &str) -> f64 {
    Mono::advance(T_MONO) as f64 * nom.chars().count() as f64 + 16.0
}

/// Place of the « Explique » pill in the title line, between « Argos Lens » and the regime pill (no new line, the
/// card keeps its size); `None` without a snapshot to explain or without room.
fn bouton_rect(text: &Text, c: &Contenu, x0: f64, x1: f64, y: f64) -> Option<Rect> {
    c.scores?;
    let x = x0 + text.width("Argos Lens", T + 1.0) as f64 + 8.0;
    let w = text.width(EXPLIQUE, T_MONO) as f64 + 14.0;
    let fin = c.regime.map_or(x1, |(nom, _)| x1 - regime_w(nom)) - 6.0;
    (x + w <= fin).then(|| Rect::new(x, y - 1.0, w, 20.0))
}

/// Title, « Explique » pill and regime pill; returns the top of the next line and the pill.
fn draw_tete(pm: &mut Pixmap, text: &Text, c: &Contenu, x0: f64, x1: f64, y: f64) -> (f64, Option<Rect>) {
    text.draw(pm, "Argos Lens", (x0 as f32, (y + 13.0) as f32), T + 1.0, jetons::TEXTE, Align::Left);
    let bouton = bouton_rect(text, c, x0, x1, y);
    if let Some(b) = bouton {
        // (the 1 pt outline drawn half a point inside: nothing goes out of the pill's place)
        let d = Rect::new(b.x + 0.5, b.y + 0.5, b.w - 1.0, b.h - 1.0);
        round_fill(pm, d, d.h / 2.0, jetons::VERRE2.0, jetons::VERRE2.1);
        round_stroke(pm, d, d.h / 2.0, jetons::sur_fond(jetons::LISERE), 1.0);
        text.draw(pm, EXPLIQUE, ((b.x + 7.0) as f32, (y + 13.0) as f32), T_MONO, jetons::TEXTE, Align::Left);
    }
    if let Some((nom, coul)) = c.regime {
        let pw = regime_w(nom);
        let p = Rect::new(x1 - pw, y - 1.0, pw, 20.0);
        round_fill(pm, p, 10.0, coul, 0.18);
        round_stroke(pm, p, 10.0, coul, 1.0);
        Mono::draw(pm, nom, ((p.x + 8.0) as f32, (y + 13.0) as f32), T_MONO, coul);
    }
    (y + LIGNE + 6.0, bouton)
}

/// 🤖 The « Explique » pill of the Lens card at the last drawing (chart coordinates): a click there opens Argos AI's
/// explanation of the card. `None`: no card on the last drawing, or no pill (smallest card, nothing to explain).
pub fn bouton_explique() -> Option<Rect> {
    BOUTON
        .with(|b| b.get())
        .filter(|(carte, _)| super::carte_lens() == Some(*carte))
        .map(|(_, b)| b)
}

/// The same pill with its touch area grown around its centre to at least [`TOUCHER_MIN`] × [`TOUCHER_MIN`], for a
/// finger (the drawn pill stays small).
pub fn bouton_explique_doigt() -> Option<Rect> {
    bouton_explique().map(|b| {
        let (w, h) = (b.w.max(TOUCHER_MIN), b.h.max(TOUCHER_MIN));
        Rect::new(b.x + (b.w - w) / 2.0, b.y + (b.h - h) / 2.0, w, h)
    })
}

/// Jauges long / short avec le seuil à 70 (ou le message à leur place) ; renvoie le haut de la ligne suivante.
fn draw_jauges(pm: &mut Pixmap, text: &Text, c: &Contenu, x0: f64, x1: f64, mut y: f64) -> f64 {
    if let Some((long, short)) = c.scores {
        for (nom, score, coul) in [("Long", long, pal::up()), ("Short", short, pal::down())] {
            text.draw(pm, nom, (x0 as f32, (y + 12.0) as f32), T, coul, Align::Left);
            let (gx, gw) = (x0 + 52.0, (x1 - 54.0) - (x0 + 52.0));
            let g = Rect::new(gx, y + 5.0, gw.max(0.0), 8.0);
            round_fill(pm, g, 4.0, jetons::BLANC, 0.10);
            round_fill(pm, Rect::new(gx, y + 5.0, (gw * f64::from(score) / 100.0).max(0.0), 8.0), 4.0, coul, 0.95);
            crate::fill(pm, gx + gw * SEUIL - 1.0, y + 2.0, 2.0, 14.0, jetons::TIER2, 1.0);
            let s = format!("{score}/100");
            let sw = Mono::advance(T_MONO) as f64 * s.chars().count() as f64;
            Mono::draw(pm, &s, ((x1 - sw) as f32, (y + 12.0) as f32), T_MONO, jetons::TEXTE);
            y += LIGNE;
        }
        y + 4.0
    } else {
        if let Some(m) = &c.message {
            text.draw(pm, m, (x0 as f32, (y + 12.0) as f32), T_MONO, jetons::TIER2, Align::Left);
        }
        y + LIGNE
    }
}

/// Mini-pastilles ; `une_ligne` : seulement celles de la première ligne. Renvoie le haut de la ligne suivante.
fn draw_puces(pm: &mut Pixmap, text: &Text, c: &Contenu, x0: f64, x1: f64, y: f64, une_ligne: bool) -> f64 {
    let puces: Vec<_> = ranger_puces(text, &c.puces, x1 - x0).into_iter().filter(|p| !une_ligne || p.2 == 0).collect();
    for (p, px, rang, pw) in &puces {
        let pr = Rect::new(x0 + px, y + *rang as f64 * 24.0, *pw, 20.0);
        round_fill(pm, pr, 10.0, jetons::VERRE2.0, jetons::VERRE2.1);
        text.draw(pm, p, ((pr.x + 7.0) as f32, (pr.y + 14.0) as f32), T_MONO, jetons::TEXTE, Align::Left);
    }
    y + puces.last().map_or(0.0, |p| (p.2 + 1) as f64 * 24.0)
}

/// Dessine la carte, à la plus grande taille qui tient entière ; `biais` : unité du biais (« 4h », vide si aucun) ;
/// heures dans le fuseau de la chart ; `haut` : bord du haut de la carte sur grand écran (sous la liste des
/// indicateurs). Renvoie sa place (`None` : pas la place, rien n'est dessiné).
pub(super) fn draw_carte(pm: &mut Pixmap, text: &Text, v: &LensVue, g: &super::Geo, biais: &str, haut: f64, carte: (usize, f64)) -> Option<Rect> {
    BOUTON.with(|b| b.set(None));
    let (place, palier, wl) = place_carte_de(text, v, g.bars, biais, g.right(), g.price_h, haut, carte)?;
    let k = carte.1;
    if k >= 1.0 {
        let bouton = peindre_carte(pm, text, v, g, biais, place, palier);
        BOUTON.with(|b| b.set(bouton.map(|x| (place, x))));
        return Some(place);
    }
    // 🖼 Small / Tiny : la carte entière dessinée à sa taille normale, puis réduite d'un coup (texte, jauges, pastilles)
    let (wl, hl) = (wl, place.h / k);
    let mut sub = crate::px::new(wl.ceil() as u32 + 1, hl.ceil() as u32 + 1)?;
    let bouton = peindre_carte(&mut sub, text, v, g, biais, Rect::new(0.0, 0.0, wl, hl), palier);
    // the pill, reduced with the card
    let bouton = bouton.map(|x| Rect::new(place.x + x.x * k, place.y + x.y * k, x.w * k, x.h * k));
    BOUTON.with(|b| b.set(bouton.map(|x| (place, x))));
    let tr = tiny_skia::Transform::from_scale(k as f32, k as f32).post_translate(crate::px::dev(place.x) as f32, crate::px::dev(place.y) as f32);
    let paint = tiny_skia::PixmapPaint {
        quality: tiny_skia::FilterQuality::Bicubic,
        ..Default::default()
    };
    pm.draw_pixmap(0, 0, sub.as_ref(), &paint, tr, None);
    Some(place)
}

/// Peint la carte dans `r` (taille normale), au `palier` choisi ; renvoie la pilule « Explique » (dans les
/// coordonnées de `pm`) si elle est dessinée.
fn peindre_carte(pm: &mut Pixmap, text: &Text, v: &LensVue, g: &super::Geo, biais: &str, r: Rect, palier: Palier) -> Option<Rect> {
    let c = contenu(text, v, g.bars, biais, r.w, g.tz_offset_s);
    // 🫧 verre de la maquette, plus opaque : sur la chart il n'y a pas de flou derrière (texte lisible, ≥ 4,5:1)
    let rayon = if palier == Palier::Pastille { r.h / 2.0 } else { jetons::R_CARTE };
    round_fill(pm, r, rayon, jetons::VERRE.0, 0.88);
    round_stroke(pm, r, rayon, jetons::sur_fond(jetons::LISERE), 1.0);
    let (x0, x1) = (r.x + PAD, r.x + r.w - PAD);
    match palier {
        Palier::Pastille => {
            let coul = c.regime.map_or(jetons::TEXTE, |x| x.1);
            text.draw(pm, &texte_pastille(&c), ((r.x + 12.0) as f32, (r.y + 18.0) as f32), T_MONO, coul, Align::Left);
            None
        }
        Palier::Compacte | Palier::Jauges => {
            let (mut y, bouton) = draw_tete(pm, text, &c, x0, x1, r.y + PAD);
            y = draw_jauges(pm, text, &c, x0, x1, y);
            if let Some((court, coul)) = &c.setup_court {
                text.draw(pm, court, (x0 as f32, (y + 12.0) as f32), T_MONO, *coul, Align::Left);
                y += LIGNE + 4.0;
            }
            if palier == Palier::Compacte {
                draw_puces(pm, text, &c, x0, x1, y, true);
            }
            bouton
        }
        Palier::Complete => {
            let (mut y, bouton) = draw_tete(pm, text, &c, x0, x1, r.y + PAD);
            y = draw_jauges(pm, text, &c, x0, x1, y);
            if let Some((titre, coul, lignes)) = &c.setup {
                y += 6.0;
                let b = Rect::new(x0, y, x1 - x0, (lignes.len() + 1) as f64 * 16.0 + 12.0);
                round_fill(pm, b, 14.0, *coul, 0.12);
                round_stroke(pm, b, 14.0, *coul, 1.0);
                text.draw(pm, titre, ((x0 + 10.0) as f32, (y + 18.0) as f32), T, *coul, Align::Left);
                for (k, (nom, prix, c)) in lignes.iter().enumerate() {
                    let yb = (y + 18.0 + 16.0 * (k + 1) as f64) as f32;
                    text.draw(pm, nom, ((x0 + 10.0) as f32, yb), T_MONO, *c, Align::Left);
                    let pw = Mono::advance(T_MONO) as f64 * prix.chars().count() as f64;
                    Mono::draw(pm, prix, ((x1 - 10.0 - pw) as f32, yb), T_MONO, *c);
                }
                y += b.h + 6.0;
            }
            y = draw_puces(pm, text, &c, x0, x1, y, false) + 4.0;
            for l in &c.pied {
                text.draw(pm, l, (x0 as f32, (y + 11.0) as f32), T_MONO, jetons::DIM, Align::Left);
                y += 15.0;
            }
            bouton
        }
    }
}

// — repères sur la chart (maquette Lens) : zones et lignes du setup, triangle du signal, dernier BOS —

/// Ligne horizontale en tirets de `x0` à `x1`.
fn tirets(pm: &mut Pixmap, x0: f64, x1: f64, y: f64, c: Rgb, alpha: f32) {
    let mut x = x0;
    while x < x1 {
        crate::fill(pm, x, y, (x1 - x).min(6.0), 1.0, c, alpha);
        x += 10.0;
    }
}

/// Zones vertes (entrée → cible 2) et rouges (entrée → stop) à partir de la bougie du setup, quatre lignes
/// étiquetées, un triangle « LONG 80 » sous la bougie (au-dessus pour un short), et le dernier BOS en pointillé.
pub(super) fn draw_reperes(pm: &mut Pixmap, text: &Text, g: &super::Geo, v: &LensVue) {
    let right = g.right();
    if let Some((i, s)) = v.setup.filter(|(i, _)| *i <= g.last) {
        let [entree, stop, t1, t2] = s.niveaux_f64();
        let long = s.side == Direction::Long;
        let (vert, rouge) = (pal::up(), pal::down());
        let x0 = g.x(i).max(0.0);
        let (ye, ys, y1, y2) = (g.y(entree), g.y(stop), g.y(t1), g.y(t2));
        crate::fill(pm, x0, ye.min(y2), right - x0, (ye - y2).abs(), vert, 0.10);
        crate::fill(pm, x0, ye.min(ys), right - x0, (ye - ys).abs(), rouge, 0.10);
        for (y, nom, c) in [
            (y2, "Cible 2 · 3R", vert),
            (y1, "Cible 1 · 1,5R", vert),
            (ye, "Entrée", jetons::TEXTE),
            (ys, "Stop · 1,5 ATR", rouge),
        ] {
            tirets(pm, x0, right, y, c, 0.9);
            text.draw(pm, nom, ((right - 8.0) as f32, (y - 4.0) as f32), 11.0, c, Align::Right);
        }
        // triangle du signal sous (long) ou au-dessus (short) de la bougie du setup, avec le score
        if let Some(b) = g.bars.get(i) {
            let (c, score) = if long {
                (vert, v.photo.as_ref().map_or(0, |p| p.1.score_long))
            } else {
                (rouge, v.photo.as_ref().map_or(0, |p| p.1.score_short))
            };
            let x = g.x(i);
            let (pointe, base) = if long {
                (g.y(b.low) + 6.0, g.y(b.low) + 14.0)
            } else {
                (g.y(b.high) - 6.0, g.y(b.high) - 14.0)
            };
            let tri = [(x as f32, pointe as f32), ((x - 6.0) as f32, base as f32), ((x + 6.0) as f32, base as f32)];
            crate::shapes::poly_fill(pm, 0.0, 0.0, &tri, c, 1.0);
            let ty = if long { base + 13.0 } else { base - 4.0 };
            let nom = format!("{} {score}", if long { "LONG" } else { "SHORT" });
            text.draw(pm, &nom, (x as f32, ty as f32), 11.0, c, Align::Center);
        }
    }
    if let Some(b) = v.bos.filter(|b| b.i1 >= g.first && b.i0 <= g.last) {
        let c = if b.up { pal::up() } else { pal::down() };
        let (x0, x1, y) = (g.x(b.i0), g.x(b.i1), g.y(b.price));
        tirets(pm, x0, x1, y, c, 0.9);
        let nom = if b.up { "BOS ↑" } else { "BOS ↓" };
        text.draw(pm, nom, (((x0 + x1) / 2.0) as f32, (y - 4.0) as f32), 11.0, c, Align::Center);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bars(n: usize) -> Vec<Bar> {
        (0..n)
            .map(|i| {
                let c = 200.0 + (i as f64 / 40.0).sin() * 30.0 + (i as f64 / 3.0).sin() * 1.5;
                Bar {
                    time: 1_699_920_000 + i as i64 * 3600,
                    open: c - 0.3,
                    high: c + 1.0,
                    low: c - 1.0,
                    close: c,
                    volume: 100.0 + (i % 7) as f64 * 15.0,
                }
            })
            .collect()
    }

    #[test]
    fn derniere_bougie_cloturee_seulement() {
        let b = bars(400);
        let defauts = argos_script::inputs::defaults(&reglages::PARAMS);
        let fin = b[399].time + 3600;
        // la dernière bougie n'est pas finie : la photo est celle d'avant
        let v = compute(&b, 3600, fin - 1, &defauts);
        assert_eq!(v.photo.as_ref().map(|p| p.0), Some(398));
        assert!(v.erreur.is_none());
        let v = compute(&b, 3600, fin, &defauts);
        assert_eq!(v.photo.as_ref().map(|p| p.0), Some(399));
        if let Some((i, s)) = v.setup {
            assert!(i <= 399 && Some(s.side) == v.photo.as_ref().and_then(|p| p.1.setup).map(|x| x.side));
        }
        assert!(v.bos.is_some_and(|x| x.i1 <= 399));
        // poids qui ne font pas 100 : pas de photo, l'erreur est gardée pour la carte
        let mut w = defauts.clone();
        w[0] = 50.0;
        assert_eq!(compute(&b, 3600, fin, &w).erreur, Some(LensError::Weights { sum: 130 }));
        assert_eq!(compute(&[], 3600, fin, &defauts).photo, None);
    }

    /// Chart d'essai avec Lens seul, de `w` × `h`.
    fn image(b: &[Bar], w: u32, h: u32) -> Pixmap {
        image_de(b, w, h, None)
    }

    /// La chart avec Lens et ces réglages de Lens (`None` : ses défauts).
    fn image_de(b: &[Bar], w: u32, h: u32, vals: Option<Vec<f64>>) -> Pixmap {
        use crate::indicators::{compute as calcul, Ind, Settings};
        use crate::{CandleStyle, Chart, ChartSettings, Theme, Viewport};
        let now = b.last().map_or(0, |x| x.time + 3600);
        let mut out = calcul(b, 3600, &[Ind::Lens], &Settings::new(), None, now, &Default::default(), &[]);
        (out.on, out.inputs) = (vec![Ind::Lens], Settings::new());
        if let Some(v) = vals {
            out.inputs.insert(Ind::Lens, v);
        }
        let c = Chart {
            bars: b,
            tf_seconds: 3600,
            viewport: Viewport {
                width: w,
                height: h,
                ..Viewport::default()
            },
            theme: Theme::default(),
            style: CandleStyle::Classic,
            tz_offset_s: 0,
            title: None,
            crosshair: None,
            banner: None,
            context: Default::default(),
            drawings: vec![],
            selected: None,
            now_s: now,
            live: None,
            hors_ligne: false,
            legende: false,
            navigation: true,
            goto: None,
            indicators: Some(&out),
            mark: None,
            whales: vec![],
            events: vec![],
            alerts: vec![],
            alertes_vue: Default::default(),
            ordres: Default::default(),
            demo: None,
            bid_ask: None,
            book_heat: None,
            book_note: None,
            murs: None,
            liq_map: None,
            pad: None,
            trades: vec![],
            explication: &[],
            garder: vec![],
            garder_pilules: vec![],
            boutons_bas: None,
            settings: ChartSettings::default(),
        };
        crate::render(&c)
    }

    /// Hausse nette : un setup LONG en cours sur la dernière bougie.
    fn hausse(n: usize) -> Vec<Bar> {
        (0..n)
            .map(|i| {
                let c = 2000.0 + i as f64 * 1.2 + (i as f64 / 3.0).sin() * 4.0;
                Bar {
                    time: 1_699_920_000 + i as i64 * 3600,
                    open: c - 1.5,
                    high: c + 3.0,
                    low: c - 3.5,
                    close: c,
                    volume: 100.0 + (i % 7) as f64 * 20.0,
                }
            })
            .collect()
    }

    /// 🫧 La carte se dessine en haut à gauche (grand écran) et en bas (cell) ; 📸 captures pour la comparaison avec
    /// la maquette Lens si `ARGOS_CAPTURES` donne un dossier.
    #[test]
    fn carte_dessinee_et_captures() {
        let b = hausse(400);
        let defauts = argos_script::inputs::defaults(&reglages::PARAMS);
        let v = compute(&b, 3600, b[399].time + 3600, &defauts);
        assert!(
            v.setup.is_some(),
            "setup en cours : {:?}",
            v.photo.as_ref().map(|p| (p.1.score_long, p.1.score_short))
        );
        let t = Text::default();
        let (grand, palier) = place_carte(&t, &v, &b, "4h", 900.0, 620.0, 140.0).unwrap();
        assert_eq!((grand.x, grand.y, grand.w, palier), (12.0, 140.0, 330.0, Palier::Complete));
        let (cell, _) = place_carte(&t, &v, &b, "4h", 390.0, 700.0, 140.0).unwrap();
        // l'heure du setup est écrite dans le fuseau de la chart (New York en hiver : UTC − 5 h)
        let titre = |tz: i64| contenu(&t, &v, &b, "4h", 330.0, tz).setup.map(|s| s.0).unwrap_or_default();
        let (utc, ny) = (titre(0), titre(-5 * 3600));
        let hh = |s: &str| {
            s.rsplit(' ')
                .next()
                .and_then(|h| h.split(':').next())
                .and_then(|h| h.parse::<i64>().ok())
                .unwrap_or(-1)
        };
        assert_eq!((hh(&utc) - 5).rem_euclid(24), hh(&ny), "{utc} / {ny}");
        assert!(cell.w == 366.0 && cell.y + cell.h <= 700.0 && cell.y > 52.0, "cell : en bas, toute la largeur");
        for (nom, w, h) in [("pc", 900, 520), ("cell", 390, 700)] {
            let pm = image(&b, w, h);
            if let Ok(dir) = std::env::var("ARGOS_CAPTURES") {
                pm.save_png(format!("{dir}/lens-{nom}.png")).unwrap();
            }
        }
    }

    /// 🏷️ PC : la liste des indicateurs dépliée (Lens et d'autres sur le prix) ne passe jamais sur la carte.
    #[test]
    fn carte_sous_la_legende_depliee() {
        use crate::indicators::{compute as calcul, Ind, Settings};
        let b = hausse(400);
        let now = b[399].time + 3600;
        for on in [vec![Ind::Lens], vec![Ind::Vwap, Ind::Lens, Ind::EmaMulti, Ind::Bollinger]] {
            let mut out = calcul(&b, 3600, &on, &Settings::new(), None, now, &Default::default(), &[]);
            out.inputs = Settings::new();
            let t = Text::default();
            let v = out.lens.clone().expect("Lens calculé");
            let (carte, _) = place_carte(&t, &v, &b, "4h", 1100.0, 640.0, crate::indleg::bas_liste(&out) + 8.0).unwrap();
            for (z, c) in crate::indleg::zones(&t, &out.on, &out, true, 640.0) {
                let touche = z.x < carte.x + carte.w && carte.x < z.x + z.w && z.y < carte.y + carte.h && carte.y < z.y + z.h;
                assert!(!touche, "{c:?} {z:?} sur la carte {carte:?} ({on:?})");
            }
        }
    }

    /// 🖼 Emplacement × taille (réglages de Jee) : Auto = la place d'avant (PC et écran étroit) ; les 4 coins × Normal,
    /// Small, Tiny restent entiers dans la zone des prix (sous la légende, jamais sur l'échelle, au-dessus du bouton
    /// Argos AI en bas à droite), plus petits en Small / Tiny ; la carte réduite est bien dessinée.
    #[test]
    fn emplacement_et_taille_de_la_carte() {
        let b = hausse(400);
        let v = compute(&b, 3600, b[399].time + 3600, &argos_script::inputs::defaults(&reglages::PARAMS));
        let t = Text::default();
        for (pw, ph, haut) in [(1100.0, 640.0, 60.0), (366.0, 500.0, 60.0)] {
            let avant = place_carte(&t, &v, &b, "4h", pw, ph, haut).unwrap();
            let auto = place_carte_de(&t, &v, &b, "4h", pw, ph, haut, (0, 1.0)).unwrap();
            assert_eq!((auto.0, auto.1), avant, "Auto = comme avant ({pw})");
            for coin in 1..=4 {
                let mut aire = f64::MAX;
                for k in [1.0, 0.85, 0.72] {
                    let (r, _, _) = place_carte_de(&t, &v, &b, "4h", pw, ph, haut, (coin, k)).expect("place");
                    let haut_min = if pw <= ETROIT {
                        52.0
                    } else if coin == 2 || coin == 4 {
                        HAUT_DROITE
                    } else {
                        haut
                    };
                    let bas_max = if pw <= ETROIT {
                        ph - 40.0
                    } else {
                        ph - MARGE - if coin == 4 { BOUTON_AI } else { 0.0 }
                    };
                    assert!(r.x >= 0.0 && r.x + r.w <= pw, "{pw} coin {coin} × {k} : {r:?} dans la largeur");
                    assert!(r.y >= haut_min - 0.01 && r.y + r.h <= bas_max + 0.01, "{pw} coin {coin} × {k} : {r:?}");
                    assert!(r.w * r.h <= aire + 0.5, "plus petite en Small / Tiny");
                    aire = r.w * r.h;
                    if pw > ETROIT {
                        assert_eq!(r.x + r.w / 2.0 > pw / 2.0, coin == 2 || coin == 4, "droite / gauche");
                    }
                    assert_eq!(r.y + r.h / 2.0 < ph / 2.0, coin <= 2, "haut / bas");
                }
            }
        }
        // (défaut de Jee) légende de gauche ouverte loin vers le bas : en haut à droite, la carte reste en haut
        let (r, _, _) = place_carte_de(&t, &v, &b, "4h", 1100.0, 640.0, 330.0, (2, 0.85)).unwrap();
        assert!(r.y <= 60.0, "haut droite sous la légende de gauche : {r:?}");
        // 📸 captures (ARGOS_CAPTURES) : la chart PC dans chaque coin × chaque taille
        if let Ok(dir) = std::env::var("ARGOS_CAPTURES") {
            for coin in 1..=4 {
                for (k, nom) in [(0.0, "normal"), (1.0, "small"), (2.0, "tiny")] {
                    let mut vals = argos_script::inputs::defaults(&reglages::PARAMS);
                    (vals[6], vals[7]) = (coin as f64, k);
                    image_de(&b, 900, 520, Some(vals)).save_png(format!("{dir}/lens-coin{coin}-{nom}.png")).unwrap();
                }
            }
        }
    }

    /// La place de la carte est toujours entière dans la zone de prix : sous `haut` et au-dessus des bandes, à toutes
    /// les hauteurs ; plus la place manque, plus la carte est petite ; pas de place du tout : rien.
    fn dans_la_zone(r: &Rect, haut: f64, price_h: f64) -> bool {
        r.y >= haut - 0.01 && r.y + r.h <= price_h - MARGE + 0.01 && r.x >= 0.0
    }

    #[test]
    fn paliers_selon_la_hauteur() {
        let b = hausse(400);
        let v = compute(&b, 3600, b[399].time + 3600, &argos_script::inputs::defaults(&reglages::PARAMS));
        let t = Text::default();
        let haut = 140.0;
        let mut avant = Palier::Complete;
        for price_h in [900.0, 600.0, 420.0, 330.0, 260.0, 200.0, 185.0, 170.0, 160.0] {
            match place_carte(&t, &v, &b, "4h", 1200.0, price_h, haut) {
                Some((r, palier)) => {
                    assert!(dans_la_zone(&r, haut, price_h), "{price_h} : {r:?} {palier:?}");
                    assert!(palier as u8 >= avant as u8, "jamais plus grande avec moins de place");
                    avant = palier;
                }
                None => assert!(price_h - MARGE - haut < PASTILLE_H, "{price_h} : la pastille aurait tenu"),
            }
        }
        assert_eq!(avant, Palier::Pastille, "tous les paliers sont passés");
        assert!(
            place_carte(&t, &v, &b, "4h", 1200.0, 160.0, haut).is_none(),
            "pas la place : rien (jamais coupée)"
        );
    }

    /// 🐞 Le cas de Jee (15 nov. 2026, capture PC) : 7 indicateurs dont des bandes d'oscillateurs, écran 1920 × 1080 ;
    /// la carte complète dépassait sur la première bande. Maintenant : entière, au-dessus des bandes, sous la légende.
    #[test]
    fn cas_de_jee_sept_indicateurs_avec_bandes() {
        use crate::indicators::{compute as calcul, Ind, Settings};
        use crate::Viewport;
        let b = hausse(400);
        let now = b[399].time + 3600;
        let t = Text::default();
        let lourds = [Ind::Cipher, Ind::AiTrend, Ind::Tbo, Ind::Rsi];
        for n in 0..=lourds.len() {
            let mut on = vec![Ind::Tvd, Ind::EmaMulti, Ind::Lens];
            on.extend_from_slice(&lourds[..n]);
            let mut out = calcul(&b, 3600, &on, &Settings::new(), None, now, &Default::default(), &[]);
            out.inputs = Settings::new();
            for (w, h) in [(1920u32, 1080u32), (1920, 900), (1280, 720), (390, 700)] {
                let vp = Viewport {
                    width: w,
                    height: h,
                    ..Viewport::default()
                };
                crate::indicators::fit_bands(&out, vp.plot_h());
                let price_h = vp.plot_h() - out.osc_h();
                let haut = crate::indleg::bas_liste(&out) + 8.0;
                let v = out.lens.clone().expect("Lens calculé");
                if let Some((r, _)) = place_carte(&t, &v, &b, "4h", vp.plot_w(), price_h, haut) {
                    let haut = if vp.plot_w() <= ETROIT { 52.0 } else { haut };
                    assert!(dans_la_zone(&r, haut, price_h), "{n} bandes, {w}×{h} : {r:?} (zone de prix {price_h})");
                }
                if (w, h, n) == (1920, 1080, lourds.len()) {
                    // avant la correction, la carte complète (toujours prise) dépassait sur les bandes
                    let c = contenu(&t, &v, &b, "4h", 330.0, 0);
                    assert!(
                        haut + hauteur(&t, &c, 330.0, Palier::Complete) > price_h - MARGE,
                        "le cas de Jee est bien reproduit"
                    );
                    assert!(place_carte(&t, &v, &b, "4h", vp.plot_w(), price_h, haut).is_some(), "une carte reste visible");
                }
            }
        }
    }

    /// 🤖 The « Explique » pill: in the title line of the card (full, compact, reduced), never on the title or the
    /// regime pill, the card keeps its size; its finger area is at least 40 × 40; none on a chart without Lens.
    #[test]
    fn bouton_explique_dans_la_carte() {
        let b = hausse(400);
        let t = Text::default();
        let v = compute(&b, 3600, b[399].time + 3600, &argos_script::inputs::defaults(&reglages::PARAMS));
        let c = contenu(&t, &v, &b, "4h", 330.0, 0);
        let mut tiny = argos_script::inputs::defaults(&reglages::PARAMS);
        tiny[7] = 2.0;
        for (w, h, vals, k) in [(900, 520, None, 1.0), (390, 700, None, 1.0), (900, 520, Some(tiny), 0.72)] {
            let _ = image_de(&b, w, h, vals);
            let carte = crate::indicators::carte_lens().unwrap();
            let bouton = bouton_explique().unwrap_or_else(|| panic!("{w}×{h} : no pill on {carte:?}"));
            let dedans = |r: &Rect| r.x >= carte.x && r.y >= carte.y && r.x + r.w <= carte.x + carte.w && r.y + r.h <= carte.y + carte.h;
            assert!(dedans(&bouton), "{w}×{h} : {bouton:?} dans {carte:?}");
            // on the title line, after « Argos Lens », before the regime pill
            let titre = carte.x + (PAD + t.width("Argos Lens", T + 1.0) as f64) * k;
            let regime = carte.x + carte.w - (PAD + regime_w(c.regime.unwrap().0)) * k;
            assert!(bouton.x > titre && bouton.x + bouton.w < regime, "{w}×{h} : {titre} < {bouton:?} < {regime}");
            assert!((bouton.y - (carte.y + (PAD - 1.0) * k)).abs() < 0.01, "title line");
            let d = bouton_explique_doigt().unwrap();
            assert!(d.w >= TOUCHER_MIN && d.h >= TOUCHER_MIN && d.x <= bouton.x && d.y <= bouton.y);
        }
        // the card keeps its size: same height as without a pill (nothing added under the title)
        let avant = hauteur(&t, &c, 330.0, Palier::Complete);
        assert_eq!(place_carte(&t, &v, &b, "4h", 900.0, 620.0, 140.0).unwrap().0.h, avant);
        // a chart without Lens: no card, no pill (the last one is never reused)
        let _ = crate::render(&crate::Chart {
            indicators: None,
            ..sans_lens(&b)
        });
        assert_eq!(bouton_explique(), None);
    }

    /// A plain chart of `b` (no indicator).
    fn sans_lens(b: &[Bar]) -> crate::Chart<'_> {
        use crate::{CandleStyle, Chart, ChartSettings, Theme, Viewport};
        Chart {
            bars: b,
            tf_seconds: 3600,
            viewport: Viewport {
                width: 900,
                height: 520,
                ..Viewport::default()
            },
            theme: Theme::default(),
            style: CandleStyle::Classic,
            tz_offset_s: 0,
            title: None,
            crosshair: None,
            banner: None,
            context: Default::default(),
            drawings: vec![],
            selected: None,
            now_s: b.last().map_or(0, |x| x.time + 3600),
            live: None,
            hors_ligne: false,
            legende: false,
            navigation: true,
            goto: None,
            indicators: None,
            mark: None,
            whales: vec![],
            events: vec![],
            alerts: vec![],
            alertes_vue: Default::default(),
            ordres: Default::default(),
            demo: None,
            bid_ask: None,
            book_heat: None,
            book_note: None,
            murs: None,
            liq_map: None,
            pad: None,
            trades: vec![],
            explication: &[],
            garder: vec![],
            garder_pilules: vec![],
            boutons_bas: None,
            settings: ChartSettings::default(),
        }
    }
}
