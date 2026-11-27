//! ⌂ Accueil (page plein écran en verre, à l'identique de la maquette v6, jetons de `crate::jetons`) : Bonjour + profil
//! de trader et recherche en pilule (Ctrl + K), les mises en page en grandes tuiles, une bande dégradée, puis une
//! mosaïque de 8 grandes cartes (2, 3, 3) avec un aperçu en verre branché sur les vraies données de l'app
//! (Académie, gardien PropFirm, Agenda, compte démo, watchlist, marchés macro, journal). Chaque carte a sa lettre en
//! coin (C A F E P N H J M G) : la touche ouvre la section. Au tout premier lancement, 3 écrans d'accueil (profil, thème,
//! sources), une seule fois.
//!
//! Les calculs (prochaines news, semaine du journal, lignes des cartes) sont des fonctions pures, testées ici ; la
//! fenêtre remplit [`Donnees`] et ouvre la section demandée.

use crate::jetons as j;
use crate::palette as pal;
use crate::palette::{GREY, WHITE};
use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Graisse, Rgb, Text};
use argos_core::journal::JTrade;
use argos_core::money::Money;
use argos_data::eco::Event;
use chrono::TimeZone;
use tiny_skia::Pixmap;

/// Profils de trader (réglage simple, gardé dans `topbar.json`).
pub const PROFILS: [&str; 4] = ["Débutant", "Bootcamp", "Trader", "Mentor"];

/// Les 8 sections de l'Accueil, dans l'ordre des cartes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Communaute,
    Academie,
    PropFirm,
    Calendrier,
    Portefeuille,
    Marches,
    Macro,
    Journal,
    Actualites,
    Thermique,
}

impl Section {
    /// Dans l'ordre de la mosaïque de la maquette : 2 grandes, 3, 3, puis 2 grandes.
    pub const ALL: [Section; 10] = [
        Section::Communaute,
        Section::Academie,
        Section::PropFirm,
        Section::Calendrier,
        Section::Portefeuille,
        Section::Actualites,
        Section::Thermique,
        Section::Journal,
        Section::Marches,
        Section::Macro,
    ];

    /// Lettre de raccourci (en coin de la carte).
    pub fn lettre(self) -> char {
        match self {
            Section::Communaute => 'C',
            Section::Academie => 'A',
            Section::PropFirm => 'F',
            Section::Calendrier => 'E',
            Section::Portefeuille => 'P',
            Section::Marches => 'M',
            Section::Macro => 'G',
            Section::Journal => 'J',
            Section::Actualites => 'N',
            Section::Thermique => 'H',
        }
    }

    /// La section d'une touche (majuscule ou minuscule).
    pub fn depuis_lettre(c: char) -> Option<Section> {
        let c = c.to_ascii_uppercase();
        Self::ALL.into_iter().find(|s| s.lettre() == c)
    }

    pub fn nom(self) -> &'static str {
        match self {
            Section::Communaute => "Communauté",
            Section::Academie => "Académie",
            Section::PropFirm => "PropFirm",
            Section::Calendrier => "Calendrier",
            Section::Portefeuille => "Portefeuille",
            Section::Marches => "Marchés",
            Section::Macro => "Macro",
            Section::Journal => "Journal",
            Section::Actualites => "Actualités",
            Section::Thermique => "Carte thermique",
        }
    }

    pub fn sous_titre(self) -> &'static str {
        match self {
            Section::Communaute => "Les wins, les lives et ton Bootcamp",
            Section::Academie => "Reprends là où tu t'es arrêté",
            Section::PropFirm => "Ton challenge sous contrôle",
            Section::Calendrier => "Les news qui bougent l'or et le dollar",
            Section::Portefeuille => "Ton compte démo en un coup d'œil",
            Section::Marches => "Ce qui bouge maintenant",
            Section::Macro => "Les grands chiffres du monde",
            Section::Journal => "Ta semaine en chiffres",
            Section::Actualites => "Ce qui se passe, en ce moment",
            Section::Thermique => "Toute ta watchlist, en couleurs",
        }
    }

    /// Émoji du titre de la carte (comme la maquette).
    pub fn emoji(self) -> &'static str {
        match self {
            Section::Communaute => "👥",
            Section::Academie => "🎓",
            Section::PropFirm => "🛡️",
            Section::Calendrier => "📅",
            Section::Portefeuille => "💼",
            Section::Marches => "📈",
            Section::Macro => "🌍",
            Section::Journal => "📓",
            Section::Actualites => "📰",
            Section::Thermique => "📊",
        }
    }

    /// 👤 L'ordre des cartes selon le profil de trader (les 2 premières et les 2 dernières sont en grand) :
    /// Débutant : apprendre et s'entraîner d'abord ; Bootcamp : la communauté et l'Académie ; Trader : la maquette ;
    /// Mentor : la communauté, l'Académie, puis le Journal.
    pub fn ordre(profil: usize) -> [Section; 10] {
        use Section::*;
        match profil {
            0 => [
                Academie,
                PropFirm,
                Calendrier,
                Portefeuille,
                Journal,
                Actualites,
                Thermique,
                Communaute,
                Marches,
                Macro,
            ],
            1 => [
                Communaute,
                Academie,
                PropFirm,
                Calendrier,
                Journal,
                Portefeuille,
                Actualites,
                Thermique,
                Marches,
                Macro,
            ],
            3 => [
                Communaute,
                Academie,
                Journal,
                PropFirm,
                Calendrier,
                Portefeuille,
                Actualites,
                Thermique,
                Marches,
                Macro,
            ],
            _ => Section::ALL,
        }
    }

    /// Icône Phosphor.
    pub fn icone(self) -> &'static str {
        match self {
            Section::Communaute => "users-three",
            Section::Academie => "graduation-cap",
            Section::PropFirm => "shield-check",
            Section::Calendrier => "calendar",
            Section::Portefeuille => "briefcase",
            Section::Marches => "chart-line-up",
            Section::Macro => "globe",
            Section::Journal => "notebook",
            Section::Actualites => "newspaper",
            Section::Thermique => "squares-four",
        }
    }
}

/// 🛡️ Gardien PropFirm (montants en cents).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Garde {
    pub perte_jour: Money,
    pub limite_jour: Money,
    pub pnl: Money,
    pub objectif: Money,
    pub jours: usize,
    pub jours_min: u32,
}

/// 💼 Compte démo (montants en cents).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Demo {
    pub equite: Money,
    pub jour: Money,
    /// (symbole et sens, P&L latent).
    pub positions: Vec<(String, Money)>,
}

/// 📓 La semaine du journal.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Semaine {
    pub trades: usize,
    pub reussite: Option<f64>,
    pub r_moyen: Option<f64>,
}

/// Données réelles des aperçus (remplies par la fenêtre à chaque dessin).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Donnees {
    /// 🎓 (guides terminés, total, titre du prochain guide du parcours).
    pub academie: (usize, usize, Option<String>),
    pub garde: Option<Garde>,
    /// 📅 (titre, devise, heure Unix en secondes, fort impact ?).
    pub news: Vec<(String, String, i64, bool)>,
    pub demo: Option<Demo>,
    /// 🌍 (symbole, prix écrit, variation %).
    pub marches: Vec<(String, String, Option<f64>)>,
    /// 🌐 (nom, prix, variation %).
    pub macro_: Vec<(String, f64, f64)>,
    pub semaine: Semaine,
    /// 📓 Courbe d'équité du compte démo (la plus ancienne d'abord).
    pub courbe: Vec<f64>,
    /// 📰 Dernières nouvelles : (titre, source, heure Unix en secondes).
    pub actus: Vec<(String, String, i64)>,
    /// 🟩 Watchlist pour la carte thermique : (symbole, variation % sur 24 h).
    pub chaleur: Vec<(String, Option<f64>)>,
    /// Maintenant (secondes), pour « il y a 2 h ».
    pub maintenant: i64,
}

/// « il y a 5 min », « il y a 2 h », « il y a 3 j ».
pub fn il_y_a(ts: i64, now: i64) -> String {
    let m = ((now - ts).max(0)) / 60;
    if m < 60 {
        format!("il y a {} min", m.max(1))
    } else if m < 1440 {
        format!("il y a {} h", m / 60)
    } else {
        format!("il y a {} j", m / 1440)
    }
}

/// Une ligne d'aperçu : texte à gauche, valeur à droite, couleur de la valeur.
#[derive(Debug, Clone, PartialEq)]
pub struct Ligne {
    pub g: String,
    pub d: String,
    pub c: Rgb,
    /// Étiquette colorée d'une ligne à deux étages (annonce : « Tier 1 », et sa date en `d`).
    pub tag: Option<(&'static str, Rgb)>,
}

/// Une carte : sa section, ses lignes, une jauge (0 à 1) et un bouton.
#[derive(Debug, Clone, PartialEq)]
pub struct Carte {
    pub section: Section,
    pub lignes: Vec<Ligne>,
    pub jauge: Option<f64>,
    pub bouton: Option<&'static str>,
    /// 📓 Courbe d'équité (carte Journal).
    pub courbe: Vec<f64>,
    /// 🟩 Tuiles de la carte thermique (symbole, variation %).
    pub chaleur: Vec<(String, Option<f64>)>,
}

/// Nom court d'une source de nouvelles (étiquette).
pub fn source_courte(s: &str) -> &'static str {
    match s {
        "CoinDesk" => "CoinDesk",
        "Cointelegraph" => "Cointelegraph",
        "Decrypt" => "Decrypt",
        "Bitcoin Magazine" => "BTC Mag",
        "CryptoSlate" => "CryptoSlate",
        "FXStreet" => "FXStreet",
        "MarketWatch" => "MarketWatch",
        _ => "News",
    }
}

/// 📅 Les `n` prochaines annonces (impact fort ou moyen) à partir de `now` (secondes), les plus proches d'abord.
pub fn prochaines_news(agenda: &[Event], now: i64, n: usize) -> Vec<&Event> {
    let mut v: Vec<&Event> = agenda.iter().filter(|e| e.ts >= now && (e.impact == "High" || e.impact == "Medium")).collect();
    v.sort_by_key(|e| e.ts);
    v.truncate(n);
    v
}

/// Heure de New York (ET) d'une annonce : « mer. 11 nov. · 08:30 ET ».
pub fn heure_et(ts: i64) -> String {
    const J: [&str; 7] = ["lun.", "mar.", "mer.", "jeu.", "ven.", "sam.", "dim."];
    const M: [&str; 12] = [
        "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc.",
    ];
    use chrono::{Datelike, Timelike};
    let Some(d) = chrono_tz::America::New_York.timestamp_opt(ts, 0).single() else {
        return String::new();
    };
    format!(
        "{} {} {} · {:02}:{:02} ET",
        J[d.weekday().num_days_from_monday() as usize],
        d.day(),
        M[d.month0() as usize],
        d.hour(),
        d.minute()
    )
}

/// 📓 La semaine (7 derniers jours avant `now_ms`) : nombre de trades, réussite et R moyen des trades fermés.
pub fn semaine(trades: &[JTrade], now_ms: i64) -> Semaine {
    let sub: Vec<JTrade> = trades
        .iter()
        .filter(|t| t.ts() >= now_ms - 7 * 86_400_000 && t.ts() <= now_ms)
        .cloned()
        .collect();
    let s = argos_core::journal::stats(&sub);
    Semaine {
        trades: sub.len(),
        reussite: s.win,
        r_moyen: s.avg_r,
    }
}

fn argent(m: Money) -> String {
    crate::journal::money(Some(m.to_f64()))
}

fn signe(m: Money) -> String {
    if m > Money::ZERO {
        format!("+{}", argent(m))
    } else {
        argent(m)
    }
}

fn ton(v: f64) -> Rgb {
    if v > 0.0 {
        pal::up()
    } else if v < 0.0 {
        pal::down()
    } else {
        WHITE
    }
}

fn pct(p: Option<f64>) -> (String, Rgb) {
    match p {
        Some(p) => (format!("{p:+.2} %").replace('.', ","), ton(p)),
        None => ("—".into(), GREY),
    }
}

/// Les 8 cartes de l'Accueil, avec leurs lignes d'aperçu tirées des vraies données.
pub fn cartes(d: &Donnees) -> Vec<Carte> {
    let l = |g: &str, v: String, c: Rgb| Ligne {
        g: g.into(),
        d: v,
        c,
        tag: None,
    };
    Section::ALL
        .into_iter()
        .map(|s| {
            let (mut lignes, mut jauge, mut bouton) = (vec![], None, None);
            match s {
                Section::Communaute => {
                    lignes.push(l("Wins, lives, Bootcamp", "bientôt".into(), GREY));
                    lignes.push(l("Le fil de la communauté arrive dans une prochaine version", String::new(), GREY));
                }
                Section::Academie => {
                    let (fait, total, prochain) = &d.academie;
                    lignes.push(l("Progression", format!("{fait} / {total} guides"), WHITE));
                    if let Some(p) = prochain {
                        lignes.push(l(p, String::new(), WHITE));
                        bouton = Some("▶ Reprendre la leçon");
                    } else if *total > 0 {
                        lignes.push(l("Parcours terminé", "✓".into(), pal::up()));
                    }
                    jauge = (*total > 0).then(|| *fait as f64 / *total as f64);
                }
                Section::PropFirm => match &d.garde {
                    Some(g) => {
                        lignes.push(l("Perte du jour", format!("{} / {}", argent(g.perte_jour), argent(g.limite_jour)), WHITE));
                        if g.objectif > Money::ZERO {
                            lignes.push(l("Objectif", format!("{} / {}", signe(g.pnl), argent(g.objectif)), ton(g.pnl.to_f64())));
                        }
                        lignes.push(l("Jours", format!("{} / {}", g.jours, g.jours_min), WHITE));
                        jauge = (g.limite_jour > Money::ZERO).then(|| (g.perte_jour.to_f64() / g.limite_jour.to_f64()).clamp(0.0, 1.0));
                        bouton = Some("🧪 Simulateur");
                    }
                    None => lignes.push(l("Règles du challenge à régler dans le Journal", String::new(), GREY)),
                },
                Section::Calendrier => {
                    if d.news.is_empty() {
                        lignes.push(l("Agenda pas encore chargé", String::new(), GREY));
                    }
                    for (t, ccy, ts, fort) in d.news.iter().take(3) {
                        let tag = if *fort { ("Tier 1", j::BAS) } else { ("Tier 2", j::TIER2) };
                        lignes.push(Ligne {
                            tag: Some(tag),
                            ..l(&format!("{ccy} · {t}"), heure_et(*ts), j::DIM)
                        });
                    }
                }
                Section::Portefeuille => match &d.demo {
                    Some(p) => {
                        lignes.push(l("Équité démo", argent(p.equite), WHITE));
                        lignes.push(l("P&L du jour", signe(p.jour), ton(p.jour.to_f64())));
                        for (n, v) in p.positions.iter().take(2) {
                            lignes.push(l(n, signe(*v), ton(v.to_f64())));
                        }
                        if p.positions.is_empty() {
                            lignes.push(l("Aucune position ouverte", String::new(), GREY));
                        }
                    }
                    None => lignes.push(l("Pas encore de compte démo", String::new(), GREY)),
                },
                Section::Marches => {
                    if d.marches.is_empty() {
                        lignes.push(l("Watchlist pas encore chargée", String::new(), GREY));
                    }
                    for (n, prix, p) in d.marches.iter().take(4) {
                        let (t, c) = pct(*p);
                        lignes.push(l(&format!("{n}  {prix}"), t, c));
                    }
                }
                Section::Macro => {
                    let pick: Vec<&(String, f64, f64)> = ["VIX", "DXY", "10 ans"]
                        .iter()
                        .filter_map(|k| d.macro_.iter().find(|t| t.0.contains(k)))
                        .collect();
                    if pick.is_empty() {
                        for n in ["VIX", "Dollar (DXY)", "Taux US 10 ans"] {
                            lignes.push(l(n, "—".into(), GREY));
                        }
                    }
                    for (n, prix, p) in pick {
                        let (t, c) = pct(Some(*p));
                        lignes.push(l(&format!("{n}  {}", crate::viewport::format_price(*prix, 2)), t, c));
                    }
                }
                Section::Actualites => {
                    if d.actus.is_empty() {
                        lignes.push(l("Nouvelles pas encore chargées", String::new(), GREY));
                    }
                    for (t, src, ts) in d.actus.iter().take(3) {
                        lignes.push(Ligne {
                            tag: Some((crate::accueil::source_courte(src), j::ACCENT)),
                            ..l(t, il_y_a(*ts, d.maintenant), j::DIM)
                        });
                    }
                }
                Section::Thermique => {
                    if d.chaleur.is_empty() {
                        lignes.push(l("Watchlist pas encore chargée", String::new(), GREY));
                    }
                }
                Section::Journal => {
                    let w = &d.semaine;
                    lignes.push(l("Trades (7 j)", w.trades.to_string(), WHITE));
                    lignes.push(l("Réussite", w.reussite.map_or("—".into(), |r| format!("{r:.0} %")), WHITE));
                    let r = w.r_moyen;
                    lignes.push(l(
                        "R moyen",
                        r.map_or("—".into(), |r| format!("{r:+.2} R").replace('.', ",")),
                        ton(r.unwrap_or(0.0)),
                    ));
                }
            }
            Carte {
                section: s,
                lignes,
                jauge,
                bouton,
                courbe: if s == Section::Journal { d.courbe.clone() } else { vec![] },
                chaleur: if s == Section::Thermique {
                    d.chaleur.iter().take(8).cloned().collect()
                } else {
                    vec![]
                },
            }
        })
        .collect()
}

/// Ce qu'un clic demande à la fenêtre.
#[derive(Debug, Clone, PartialEq)]
pub enum AccClick {
    None,
    Close,
    /// Le profil a changé (à garder).
    Profil,
    Section(Section),
    /// 🎓 ▶ Reprendre le prochain guide.
    Reprendre,
    /// 🧪 Ouvrir le Simulateur (bouton de la carte PropFirm).
    Simulateur,
    /// 🔎 Ouvrir la recherche Ctrl + K.
    Recherche,
    NouvelleLayout,
    Layout(usize),
    /// ★ d'une tuile de mise en page : favori ou plus.
    Favori(usize),
    /// ⋯ d'une tuile : son menu (renommer, dupliquer, favori, supprimer), sous ce bouton (coordonnées de la fenêtre).
    MenuLayout(usize, Rect),
    /// 🔑 Premier lancement : ouvrir les clés API (optionnel).
    CleApi,
    /// Premier lancement terminé (drapeau à garder).
    BienvenueFin,
    /// 🫧 Premier lancement : thème Verre clair (`true`) ou Verre sombre.
    Theme(bool),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Zone {
    Profil,
    Recherche,
    Nouvelle,
    Layout(usize),
    /// ★ et ⋯ d'une tuile de mise en page.
    Etoile(usize),
    Plus(usize),
    /// « ⌄ Toutes les mises en page (n) ».
    Tout,
    Bande,
    Carte(Section),
    /// Le bouton de l'aperçu (Académie : Reprendre la leçon ; PropFirm : Simulateur).
    Bouton(Section),
}

/// Hauteur de l'en-tête.
pub const HEAD: f64 = 56.0;
/// Haut d'une tuile de mise en page et de l'en-tête d'une carte (titre et sous-titre) avant l'aperçu.
const TUILE_H: f64 = 104.0;
const TETE_CARTE: f64 = 96.0;
/// Hauteur d'une ligne d'aperçu (simple, à deux étages), du bouton, de la jauge.
const L1: f64 = 32.0;
/// Rangée de cases chiffrées (Journal : réussite, R moyen, trades).
const KPI: f64 = 60.0;
const L2: f64 = 54.0;
const BTN: f64 = 40.0;
const JAUGE: f64 = 18.0;

/// La page.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AccueilPage {
    pub scroll: f64,
    /// 🗂 (nom, symbole · unité, ouverte ?) des mises en page sauvegardées.
    pub layouts: Vec<(String, String, bool)>,
    /// ★ Noms des mises en page favorites.
    pub favoris: Vec<String>,
    /// Index dans [`PROFILS`].
    pub profil: usize,
    /// Premier lancement : écran affiché (0 profil, 1 thème, 2 sources) ; `None` = fini.
    pub bienvenue: Option<u8>,
    /// 🫧 Premier lancement : le thème choisi est Verre clair.
    pub verre_clair: bool,
    /// Heure locale (salutation : Bonjour / Bonsoir).
    pub heure: u32,
    /// « Toutes les mises en page » ouvert (sinon les 3 premières).
    pub tout: bool,
}

/// Hauteur de l'aperçu d'une carte (lignes, jauge, bouton, marges).
fn hauteur_lignes(c: &Carte) -> f64 {
    if c.section == Section::Journal {
        return KPI;
    }
    c.lignes.iter().map(|l| if l.tag.is_some() { L2 } else { L1 }).sum()
}

fn hauteur_apercu(c: &Carte) -> f64 {
    let lignes = hauteur_lignes(c);
    let extra = match c.section {
        Section::Communaute => 150.0,
        Section::Journal => 110.0,
        Section::Thermique => 130.0,
        _ => 0.0,
    };
    12.0 + lignes + extra + if c.jauge.is_some() { JAUGE } else { 0.0 } + if c.bouton.is_some() { BTN + 6.0 } else { 0.0 } + 22.0
}

impl AccueilPage {
    /// Marge et largeur du contenu (centré, 1180 px au plus, comme la maquette).
    fn cadre(w: f64) -> (f64, f64) {
        let pad = if w < 600.0 { 14.0 } else { 28.0 };
        let inner = (w - 2.0 * pad).clamp(200.0, j::LARGEUR_MAX);
        ((w - inner) / 2.0, inner)
    }

    /// ★ (`k` = 1) et ⋯ (`k` = 0) d'une tuile de mise en page, sur la ligne du nom, à droite.
    fn bouton_tuile(tuile: Rect, k: usize) -> Rect {
        Rect::new(tuile.x + tuile.w - 40.0 - k as f64 * 30.0, tuile.y + 46.0, 28.0, 28.0)
    }

    /// Zones (coordonnées sous l'en-tête, avant défilement) et hauteur totale. `cartes` : leurs aperçus donnent la
    /// hauteur des rangées.
    fn zones(&self, text: &Text, w: f64, cartes: &[Carte]) -> (Vec<(Rect, Zone)>, f64) {
        let (x0, inner) = Self::cadre(w);
        let etroit = w < 900.0;
        let mut z = vec![];
        let mut y = 22.0;
        // Bonjour + profil, recherche en pilule (à droite ; sur téléphone, en dessous)
        let salut_w = Text::graisse(Graisse::Bold).width(self.salut(), j::T_SALUT) as f64;
        let pw = text.width(PROFILS[self.profil.min(3)], j::T_TEXTE) as f64 + 46.0;
        z.push((Rect::new(x0 + salut_w + 18.0, y + 4.0, pw, 34.0), Zone::Profil));
        let qw = if etroit { inner } else { (inner * 0.5).min(780.0) };
        let qy = if etroit { y + 54.0 } else { y };
        z.push((Rect::new(x0 + inner - qw, qy, qw, 44.0), Zone::Recherche));
        y = qy + 44.0 + 26.0;
        // 🗂 mises en page : 4 tuiles par rangée (2 sur téléphone), « ＋ Nouvelle » d'abord
        let cols = if etroit { 2 } else { 4 };
        let tw = (inner - j::ESPACE_TUILES * (cols as f64 - 1.0)) / cols as f64;
        let n = if self.tout { self.layouts.len() } else { self.layouts.len().min(cols - 1) };
        for k in 0..=n {
            let (c, r) = (k % cols, k / cols);
            let rect = Rect::new(
                x0 + c as f64 * (tw + j::ESPACE_TUILES),
                y + r as f64 * (TUILE_H + j::ESPACE_TUILES),
                tw,
                TUILE_H,
            );
            if k == 0 {
                z.push((rect, Zone::Nouvelle));
            } else {
                // ★ et ⋯ à droite du nom (maquette), par-dessus la tuile
                z.push((rect, Zone::Layout(k - 1)));
                z.push((Self::bouton_tuile(rect, 1), Zone::Etoile(k - 1)));
                z.push((Self::bouton_tuile(rect, 0), Zone::Plus(k - 1)));
            }
        }
        y += (n / cols + 1) as f64 * (TUILE_H + j::ESPACE_TUILES);
        if self.layouts.len() > cols - 1 {
            let t = format!("⌄ Toutes les mises en page ({})", self.layouts.len());
            z.push((Rect::new(x0, y + 2.0, text.width(&t, j::T_TEXTE) as f64 + 8.0, 26.0), Zone::Tout));
            y += 30.0;
        }
        // bande dégradée, sur toute la largeur
        y += 14.0;
        z.push((Rect::new(0.0, y, w, j::BANDE), Zone::Bande));
        y += j::BANDE + 8.0;
        // 🧩 mosaïque : 12 colonnes (6 + 6, puis 4 + 4 + 4) ; écran moyen : 2 par rangée ; téléphone : 1
        let grille = if etroit {
            12
        } else if w < 1200.0 {
            6
        } else {
            0
        };
        let col = (inner - j::ESPACE * 11.0) / 12.0;
        let (mut cx, mut rangee): (usize, Vec<(Section, usize)>) = (0, vec![]);
        let place = |z: &mut Vec<(Rect, Zone)>, y: &mut f64, rangee: &mut Vec<(Section, usize)>| {
            let h = rangee
                .iter()
                .filter_map(|(s, _)| cartes.iter().find(|c| c.section == *s))
                .map(|c| TETE_CARTE + hauteur_apercu(c))
                .fold(j::HAUT_CARTE_MIN, f64::max);
            let mut x = x0;
            for (s, span) in rangee.drain(..) {
                let cw = col * span as f64 + j::ESPACE * (span as f64 - 1.0);
                let r = Rect::new(x, *y, cw, h);
                z.push((r, Zone::Carte(s)));
                if let Some(c) = cartes.iter().find(|c| c.section == s) {
                    if let Some(b) = Self::bouton(r, c) {
                        z.push((b, Zone::Bouton(s)));
                    }
                }
                x += cw + j::ESPACE;
            }
            *y += h + j::ESPACE;
        };
        for (k, s) in Section::ordre(self.profil).into_iter().enumerate() {
            let span = if grille > 0 {
                grille
            } else if !(2..8).contains(&k) {
                6
            } else {
                4
            };
            if cx + span > 12 {
                place(&mut z, &mut y, &mut rangee);
                cx = 0;
            }
            rangee.push((s, span));
            cx += span;
        }
        place(&mut z, &mut y, &mut rangee);
        (z, y + 40.0)
    }

    /// L'aperçu en verre d'une carte `r` (collé au bas de la carte, comme la maquette).
    fn apercu(r: Rect) -> Rect {
        let m = if r.w > 500.0 { 24.0 } else { 6.0 };
        let top = r.y + TETE_CARTE;
        Rect::new(r.x + m, top, r.w - 2.0 * m, r.y + r.h - top)
    }

    /// Le bouton de l'aperçu (sous ses lignes), s'il y en a un.
    fn bouton(r: Rect, c: &Carte) -> Option<Rect> {
        c.bouton?;
        let a = Self::apercu(r);
        let y = a.y + 12.0 + hauteur_lignes(c) + if c.jauge.is_some() { JAUGE } else { 0.0 } + 4.0;
        Some(Rect::new(a.x + 16.0, y, a.w - 32.0, BTN - 6.0))
    }

    fn salut(&self) -> &'static str {
        if (5..18).contains(&self.heure) {
            "Bonjour"
        } else {
            "Bonsoir"
        }
    }

    pub fn wheel(&mut self, dy: f64, text: &Text, w: f64, h: f64, cartes: &[Carte]) {
        let (_, full) = self.zones(text, w, cartes);
        self.scroll = (self.scroll + dy).clamp(0.0, (full - (h - HEAD)).max(0.0));
    }

    pub fn at_top(&self) -> bool {
        self.scroll <= 0.0
    }

    /// Échap : ferme la page (le premier lancement, lui, se termine par ses boutons ou Échap = « plus tard »).
    pub fn escape(&mut self) -> AccClick {
        if self.bienvenue.take().is_some() {
            return AccClick::BienvenueFin;
        }
        AccClick::Close
    }

    /// 👋 Premier lancement, comme la maquette (`bienvenue()`) : la carte de verre, ses lignes et ses zones, pour une
    /// page de `w` × `h` (cell : une colonne de cartes ; plus de 900 points : deux).
    fn bv_plan(&self, w: f64, h: f64) -> BvPlan {
        let t = Text::default();
        let gras = Text::graisse(Graisse::Bold);
        let e = self.bienvenue.unwrap_or(0);
        let dw = (w * 0.92).min(640.0);
        let (pad, x0) = (22.0, (w - dw) / 2.0);
        let inner = dw - 2.0 * pad;
        let mut p = BvPlan::default();
        let mut y = pad;
        let (titre, sous) = BV_TEXTES[e.min(2) as usize];
        p.textes.push((x0 + pad, y + 22.0, titre.into(), 22.0, true));
        y += 36.0;
        for l in couper(&t, sous, 13.0, inner) {
            p.textes.push((x0 + pad, y + 13.0, l, 13.0, false));
            y += 18.0;
        }
        y += 10.0;
        let cols = if w > 900.0 { 2 } else { 1 };
        let cw = (inner - 10.0 * (cols as f64 - 1.0)) / cols as f64;
        match e {
            0 => {
                let mut haut = 0.0f64;
                for (k, (ic, nom, desc)) in BV_PROFILS.iter().enumerate() {
                    let d = couper(&t, desc, 12.0, cw - 24.0);
                    let ch = 12.0 + 26.0 + 22.0 + d.len() as f64 * 16.0 + 10.0;
                    let (c, l) = (k % cols, k / cols);
                    if c == 0 && k > 0 {
                        y += haut + 10.0;
                        haut = 0.0;
                    }
                    let _ = l;
                    haut = haut.max(ch);
                    let r = Rect::new(x0 + pad + c as f64 * (cw + 10.0), y, cw, ch);
                    p.cartes.push((r, k as u8, ic.to_string(), nom.to_string(), d));
                }
                y += haut + 10.0;
            }
            1 => {
                for (k, nom) in ["Verre sombre", "Verre clair"].iter().enumerate() {
                    let (c, l) = (k % cols, k / cols);
                    let r = Rect::new(x0 + pad + c as f64 * (cw + 10.0), y + l as f64 * 130.0, cw, 120.0);
                    p.cartes.push((r, 20 + k as u8, String::new(), nom.to_string(), vec![]));
                }
                y += if cols == 2 { 130.0 } else { 260.0 };
                p.textes
                    .push((x0 + pad, y + 12.0, "Tu pourras changer dans Réglages ▸ Thème.".into(), 12.0, false));
                y += 24.0;
            }
            _ => {
                let blocs: [(&str, &str); 2] = [
                    (
                        "✓ Sources gratuites par défaut",
                        "Hyperliquid (crypto), données publiques pour la Macro ; rien à configurer.",
                    ),
                    ("🔑 Clé API (optionnel)", "Jamais dans la page : rangée dans le trousseau du système."),
                ];
                for (k, (b, d)) in blocs.iter().enumerate() {
                    let dl = couper(&t, d, 12.0, inner - 24.0);
                    let bh = 12.0 + 20.0 + dl.len() as f64 * 16.0 + if k == 1 { 42.0 } else { 0.0 } + 10.0;
                    let r = Rect::new(x0 + pad, y, inner, bh);
                    p.blocs.push((r, b.to_string(), dl));
                    if k == 1 {
                        let lw = gras.width(BV_CLE, 12.5) as f64 + 28.0;
                        p.zones.push((Rect::new(r.x + 12.0, r.y + bh - 42.0, lw, 30.0), 12));
                    }
                    y += bh + 10.0;
                }
            }
        }
        // pied : les 3 points à gauche ; Passer (1re page) ou ‹ Retour ; Suivant › ou Arriver sur l'accueil ›
        y += 6.0;
        p.points = (x0 + pad, y + 17.0);
        let suite = if e < 2 { "Suivant ›" } else { "Arriver sur l'accueil ›" };
        let sw = gras.width(suite, 12.5) as f64 + 28.0;
        let sr = Rect::new(x0 + dw - pad - sw, y, sw, 34.0);
        p.zones.push((sr, 11));
        let (av, id) = if e == 0 { ("Passer", 13) } else { ("‹ Retour", 10) };
        let aw = t.width(av, 12.5) as f64 + 28.0;
        p.zones.push((Rect::new(sr.x - 8.0 - aw, y, aw, 34.0), id));
        y += 34.0 + pad;
        // la carte au milieu de la page (en haut si elle est plus haute que la page)
        let dy = ((h - y) / 2.0).max(12.0);
        p.r = Rect::new(x0, dy, dw, y);
        for z in &mut p.zones {
            z.0.y += dy;
        }
        for c in &mut p.cartes {
            c.0.y += dy;
        }
        for b in &mut p.blocs {
            b.0.y += dy;
        }
        for l in &mut p.textes {
            l.1 += dy;
        }
        p.points.1 += dy;
        // les cartes se touchent aussi
        let mut z: Vec<(Rect, u8)> = p.cartes.iter().map(|c| (c.0, c.1)).collect();
        z.append(&mut p.zones);
        p.zones = z;
        p
    }

    /// Zones du premier lancement : (rect, id) ; 0..4 profils, 20 / 21 thème sombre / clair, 10 Retour, 11 Suivant /
    /// Arriver, 12 Clé API, 13 Passer.
    fn bv_zones(&self, w: f64, h: f64) -> Vec<(Rect, u8)> {
        self.bv_plan(w, h).zones
    }

    /// Clic (coordonnées de la page `w` × `h`).
    pub fn click(&mut self, text: &Text, w: f64, h: f64, x: f64, y: f64, cartes: &[Carte]) -> AccClick {
        if let Some(e) = self.bienvenue {
            let Some(id) = self.bv_zones(w, h).into_iter().find(|(r, _)| r.contains(x, y)).map(|(_, id)| id) else {
                return AccClick::None;
            };
            return match id {
                0..=3 => {
                    self.profil = id as usize;
                    AccClick::Profil
                }
                10 => {
                    self.bienvenue = Some(e.saturating_sub(1));
                    AccClick::None
                }
                11 if e < 2 => {
                    self.bienvenue = Some(e + 1);
                    AccClick::None
                }
                11 => {
                    self.bienvenue = None;
                    AccClick::BienvenueFin
                }
                12 => AccClick::CleApi,
                13 => {
                    self.bienvenue = None;
                    AccClick::BienvenueFin
                }
                20 | 21 => {
                    self.verre_clair = id == 21;
                    AccClick::Theme(id == 21)
                }
                _ => AccClick::None,
            };
        }
        if Rect::new(w - 52.0, 10.0, 36.0, 36.0).contains(x, y) {
            return AccClick::Close;
        }
        if y < HEAD {
            return AccClick::None;
        }
        let (zones, _) = self.zones(text, w, cartes);
        let cy = y - HEAD + self.scroll;
        match zones.iter().rev().find(|(r, z)| r.contains(x, cy) && *z != Zone::Bande).map(|(_, z)| *z) {
            Some(Zone::Profil) => {
                self.profil = (self.profil + 1) % PROFILS.len();
                AccClick::Profil
            }
            Some(Zone::Recherche) => AccClick::Recherche,
            Some(Zone::Nouvelle) => AccClick::NouvelleLayout,
            Some(Zone::Layout(k)) => AccClick::Layout(k),
            Some(Zone::Etoile(k)) => AccClick::Favori(k),
            Some(Zone::Plus(k)) => {
                let r = zones.iter().find(|(_, z)| *z == Zone::Plus(k)).map_or(Rect::new(x, y, 1.0, 1.0), |(r, _)| *r);
                AccClick::MenuLayout(k, Rect::new(r.x, r.y + HEAD - self.scroll, r.w, r.h))
            }
            Some(Zone::Tout) => {
                self.tout = !self.tout;
                AccClick::None
            }
            Some(Zone::Bouton(Section::PropFirm)) => AccClick::Simulateur,
            Some(Zone::Bouton(Section::Academie)) => AccClick::Reprendre,
            Some(Zone::Bouton(s)) | Some(Zone::Carte(s)) => AccClick::Section(s),
            Some(Zone::Bande) | None => AccClick::None,
        }
    }

    /// Dessine la page (`w` × `h`) avec ses cartes.
    pub fn draw(&self, text: &Text, w: u32, h: u32, mouse: (f64, f64), cartes: &[Carte]) -> Option<Pixmap> {
        let (wf, hf) = (w as f64, h as f64);
        let mut pm = crate::px::new(w.max(1), h.max(1))?;
        crate::fill(&mut pm, 0.0, 0.0, wf, hf, j::FOND, 1.0);
        let mut body = crate::px::new(w.max(1), (hf - HEAD).max(1.0) as u32)?;
        let (zones, _) = self.zones(text, wf, cartes);
        let dy = -self.scroll;
        let at = |r: &Rect| Rect::new(r.x, r.y + dy, r.w, r.h);
        let hover = |r: &Rect| self.bienvenue.is_none() && r.contains(mouse.0, mouse.1 - HEAD + self.scroll);
        let (x0, _) = Self::cadre(wf);
        let lisere = j::sur_fond(j::LISERE);
        // la bande d'abord : les cartes passent par-dessus
        for (r, z) in &zones {
            if *z == Zone::Bande {
                bande(&mut body, at(r));
            }
        }
        // Bonjour
        let (gras, moyen) = (Text::graisse(Graisse::Bold), Text::graisse(Graisse::Medium));
        gras.draw(
            &mut body,
            self.salut(),
            (x0 as f32, (22.0 + 32.0 + dy) as f32),
            j::T_SALUT,
            j::TEXTE,
            Align::Left,
        );
        for (r, z) in &zones {
            let (r0, r) = (*r, at(r));
            if r.y + r.h < -4.0 || r.y > hf {
                continue;
            }
            let survol = hover(&r0);
            match z {
                Zone::Profil => {
                    round_fill(&mut body, r, r.h / 2.0, j::VERRE2.0, if survol { 0.14 } else { j::VERRE2.1 });
                    crate::phosphor::draw(&mut body, "user-circle", r.x + 18.0, r.cy(), 16, j::TEXTE);
                    let t = PROFILS[self.profil.min(3)];
                    moyen.draw(&mut body, t, ((r.x + 32.0) as f32, (r.cy() + 5.0) as f32), j::T_TEXTE, j::TEXTE, Align::Left);
                }
                Zone::Recherche => {
                    round_fill(&mut body, r, r.h / 2.0, j::CHAMP.0, if survol { 0.09 } else { j::CHAMP.1 });
                    round_stroke(&mut body, r, r.h / 2.0, lisere, 1.0);
                    crate::phosphor::draw(&mut body, "magnifying-glass", r.x + 24.0, r.cy(), 17, j::DIM);
                    let t = text.fit("Rechercher : symbole, leçon, indicateur, réglage…", 14.0, r.w - 150.0);
                    text.draw(&mut body, &t, ((r.x + 44.0) as f32, (r.cy() + 5.0) as f32), 14.0, j::DIM, Align::Left);
                    let mut kx = r.x + r.w - 14.0;
                    for k in ["K", "Ctrl"] {
                        let kw = text.width(k, 12.0) as f64 + 14.0;
                        kx -= kw;
                        touche(&mut body, text, Rect::new(kx, r.cy() - 12.0, kw, 24.0), k);
                        kx -= 6.0;
                    }
                }
                Zone::Nouvelle => {
                    round_fill(&mut body, r, j::R_TUILE, j::VERRE2.0, if survol { 0.06 } else { 0.0 });
                    pointilles(&mut body, r, j::R_TUILE, lisere);
                    crate::shapes::lines(
                        &mut body,
                        r.cx(),
                        r.y + 38.0,
                        &[&[(-11.0, 0.0), (11.0, 0.0)], &[(0.0, -11.0), (0.0, 11.0)]],
                        j::TEXTE,
                        1.4,
                    );
                    gras.draw(
                        &mut body,
                        "Nouvelle mise en page",
                        (r.cx() as f32, (r.y + 76.0) as f32),
                        j::T_TUILE,
                        j::TEXTE,
                        Align::Center,
                    );
                }
                Zone::Layout(k) => {
                    let Some((n, sub, ouverte)) = self.layouts.get(*k) else { continue };
                    round_fill(&mut body, r, j::R_TUILE, j::VERRE.0, j::VERRE.1);
                    round_fill(&mut body, r, j::R_TUILE, j::BLANC, if survol { 0.05 } else { 0.0 });
                    round_stroke(&mut body, r, j::R_TUILE, if *ouverte { j::ACCENT } else { lisere }, 1.0);
                    if *ouverte {
                        text.draw(
                            &mut body,
                            "● ouverte",
                            ((r.x + 18.0) as f32, (r.y + 28.0) as f32),
                            j::T_PETIT,
                            j::ACCENT,
                            Align::Left,
                        );
                    }
                    gras.draw(
                        &mut body,
                        &gras.fit(n, j::T_TUILE, r.w - 100.0),
                        ((r.x + 18.0) as f32, (r.y + 66.0) as f32),
                        j::T_TUILE,
                        j::TEXTE,
                        Align::Left,
                    );
                    text.draw(
                        &mut body,
                        &text.fit(sub, 12.0, r.w - 36.0),
                        ((r.x + 18.0) as f32, (r.y + 88.0) as f32),
                        12.0,
                        j::DIM,
                        Align::Left,
                    );
                }
                Zone::Etoile(k) => {
                    // ★ orange pour un favori (maquette), contour sinon
                    let nom = self.layouts.get(*k).map(|l| l.0.as_str()).unwrap_or("");
                    let fav = self.favoris.iter().any(|f| f == nom);
                    if survol {
                        round_fill(&mut body, r, r.h / 2.0, j::VERRE2.0, j::VERRE2.1);
                    }
                    let pts: Vec<(f32, f32)> = (0..=10)
                        .map(|i| {
                            let ray = if i % 2 == 0 { 7.0f32 } else { 3.2 };
                            let a = (i as f32 * 36.0).to_radians();
                            (ray * a.sin(), -ray * a.cos())
                        })
                        .collect();
                    if fav {
                        crate::shapes::poly_fill(&mut body, r.cx(), r.cy(), &pts, j::TIER2, 1.0);
                    } else {
                        crate::shapes::lines(&mut body, r.cx(), r.cy(), &[&pts], j::DIM, 1.2);
                    }
                }
                Zone::Plus(_) => {
                    if survol {
                        round_fill(&mut body, r, r.h / 2.0, j::VERRE2.0, j::VERRE2.1);
                    }
                    for dx in [-6.0, 0.0, 6.0] {
                        round_fill(&mut body, Rect::new(r.cx() + dx - 1.5, r.cy() - 1.5, 3.0, 3.0), 1.5, j::DIM, 1.0);
                    }
                }
                Zone::Tout => {
                    let t = if self.tout {
                        format!("⌃ Moins de mises en page ({})", self.layouts.len())
                    } else {
                        format!("⌄ Toutes les mises en page ({})", self.layouts.len())
                    };
                    gras.draw(&mut body, &t, ((r.x + 2.0) as f32, (r.cy() + 5.0) as f32), j::T_TEXTE, j::ACCENT, Align::Left);
                }
                Zone::Carte(s) => {
                    let Some(c) = cartes.iter().find(|c| c.section == *s) else { continue };
                    self.draw_carte(&mut body, text, r, c, survol);
                }
                Zone::Bouton(_) | Zone::Bande => {}
            }
        }
        // les boutons des aperçus, par-dessus leur carte
        for (r, z) in &zones {
            if let Zone::Bouton(s) = z {
                let Some(b) = cartes.iter().find(|c| c.section == *s).and_then(|c| c.bouton) else {
                    continue;
                };
                let rr = at(r);
                round_fill(&mut body, rr, j::pilule(rr), j::PUCE_ON.0, if hover(r) { 0.5 } else { j::PUCE_ON.1 });
                moyen.draw(&mut body, b, (rr.cx() as f32, (rr.cy() + 5.0) as f32), 14.0, j::TEXTE, Align::Center);
            }
        }
        // (📱 phone view: no « Touches : … » line, there is no keyboard)
        if let Some((r, _)) = zones
            .iter()
            .rev()
            .find(|(_, z)| matches!(z, Zone::Carte(_)))
            .filter(|_| crate::actions::montrer_raccourcis())
        {
            let y = r.y + r.h + 26.0 + dy;
            let t = text.fit(
                "Touches : la lettre en coin de chaque carte ouvre la section · Ctrl + K cherche partout · Ctrl + / liste les raccourcis",
                j::T_PETIT,
                wf - 2.0 * x0,
            );
            text.draw(&mut body, &t, (x0 as f32, y as f32), j::T_PETIT, j::DIM, Align::Left);
        }
        crate::px::put(&mut pm, 0.0, HEAD, &body);
        // en-tête : ⌂ Accueil, ✕
        maison(&mut pm, 30.0, HEAD / 2.0, j::TEXTE);
        gras.draw(
            &mut pm,
            "Accueil",
            (50.0, (HEAD / 2.0 + 7.0) as f32),
            j::T_TITRE_PAGE + 2.0,
            j::TEXTE,
            Align::Left,
        );
        let x = Rect::new(wf - 52.0, 10.0, 36.0, 36.0);
        round_fill(
            &mut pm,
            x,
            18.0,
            j::VERRE2.0,
            if x.contains(mouse.0, mouse.1) && self.bienvenue.is_none() {
                0.16
            } else {
                j::VERRE2.1
            },
        );
        crate::shapes::lines(
            &mut pm,
            x.cx(),
            x.cy(),
            &[&[(-6.0, -6.0), (6.0, 6.0)], &[(6.0, -6.0), (-6.0, 6.0)]],
            j::TEXTE,
            1.6,
        );
        if self.bienvenue.is_some() {
            self.draw_bienvenue(&mut pm, text, wf, hf, mouse);
        }
        Some(pm)
    }

    /// Une grande carte : titre (émoji + nom), sous-titre, lettre en coin, et son aperçu en verre collé en bas.
    fn draw_carte(&self, pm: &mut Pixmap, text: &Text, r: Rect, c: &Carte, survol: bool) {
        let lisere = j::sur_fond(j::LISERE);
        let fond = if survol { j::CARTE_SURVOL } else { j::CARTE };
        round_fill(pm, r, j::R_CARTE, fond.0, fond.1);
        round_stroke(pm, r, j::R_CARTE, if survol { j::sur_fond((j::BLANC, 0.35)) } else { lisere }, 1.0);
        let s = c.section;
        let p = j::PAD_CARTE;
        if let Some(e) = crate::emoji::image(s.emoji(), 26) {
            crate::px::put(pm, r.x + p, r.y + p + 4.0, &e);
        }
        let (gras, moyen) = (Text::graisse(Graisse::Bold), Text::graisse(Graisse::Medium));
        gras.draw(
            pm,
            s.nom(),
            ((r.x + p + 34.0) as f32, (r.y + p + 26.0) as f32),
            j::T_TITRE_CARTE,
            j::TEXTE,
            Align::Left,
        );
        text.draw(
            pm,
            &text.fit(s.sous_titre(), j::T_TEXTE, r.w - 2.0 * p),
            ((r.x + p) as f32, (r.y + p + 52.0) as f32),
            j::T_TEXTE,
            j::DIM,
            Align::Left,
        );
        let k = Rect::new(r.x + r.w - 40.0, r.y + 14.0, 26.0, 24.0);
        touche(pm, text, k, &s.lettre().to_string());
        // aperçu en verre
        let a = Self::apercu(r);
        round_fill(pm, a, j::R_APERCU, j::VERRE.0, j::VERRE.1);
        round_stroke(pm, a, j::R_APERCU, lisere, 1.0);
        crate::fill(pm, a.x + j::R_APERCU, a.y, a.w - 2.0 * j::R_APERCU, 1.0, j::REFLET.0, j::REFLET.1);
        let (lx, lw) = (a.x + 16.0, a.w - 32.0);
        let mut y = a.y + 12.0;
        if s == Section::Communaute {
            communaute(pm, text, Rect::new(lx, y, lw.min(520.0), 140.0));
            y += 150.0;
        }
        if s == Section::Journal {
            // 3 cases côte à côte : petit libellé, gros chiffre (`.ap-kpi`)
            let n = c.lignes.len().max(1) as f64;
            let cw = (lw - 8.0 * (n - 1.0)) / n;
            for (k, l) in c.lignes.iter().enumerate() {
                let b = Rect::new(lx + k as f64 * (cw + 8.0), y, cw, KPI - 8.0);
                round_fill(pm, b, j::R_LIGNE, j::LIGNE.0, j::LIGNE.1);
                let lab = text.fit(&l.g, j::T_PETIT, b.w - 16.0);
                text.draw(pm, &lab, ((b.x + 10.0) as f32, (b.y + 18.0) as f32), j::T_PETIT, j::DIM, Align::Left);
                gras.draw(pm, &l.d, ((b.x + 10.0) as f32, (b.y + 40.0) as f32), 15.0, l.c, Align::Left);
            }
            y += KPI;
        }
        for l in c.lignes.iter().filter(|_| s != Section::Journal) {
            let hl = if l.tag.is_some() { L2 } else { L1 } - 6.0;
            let b = Rect::new(lx, y, lw, hl);
            round_fill(pm, b, j::R_LIGNE, j::LIGNE.0, j::LIGNE.1);
            let dw = if l.d.is_empty() || l.tag.is_some() {
                0.0
            } else {
                text.width(&l.d, j::T_APERCU) as f64 + 16.0
            };
            let gc = if l.d.is_empty() && l.tag.is_none() { l.c } else { j::TEXTE };
            let gy = (b.y + if l.tag.is_some() { 18.0 } else { b.h / 2.0 + 4.5 }) as f32;
            text.draw(
                pm,
                &text.fit(&l.g, j::T_APERCU, b.w - 24.0 - dw),
                ((b.x + 12.0) as f32, gy),
                j::T_APERCU,
                gc,
                Align::Left,
            );
            match l.tag {
                Some((t, tc)) => {
                    let tw = text.width(t, j::T_PETIT) as f64 + 14.0;
                    let tr = Rect::new(b.x + 10.0, b.y + 26.0, tw, 19.0);
                    round_fill(pm, tr, 7.0, tc, 0.25);
                    text.draw(pm, t, (tr.cx() as f32, (tr.cy() + 4.0) as f32), j::T_PETIT, tc, Align::Center);
                    text.draw(pm, &l.d, ((tr.x + tw + 8.0) as f32, (tr.cy() + 4.0) as f32), 12.0, j::DIM, Align::Left);
                }
                None if !l.d.is_empty() => {
                    moyen.draw(pm, &l.d, ((b.x + b.w - 12.0) as f32, gy), j::T_APERCU, l.c, Align::Right);
                }
                None => {}
            }
            y += hl + 6.0;
        }
        if let Some(g) = c.jauge {
            let b = Rect::new(lx, y + 4.0, lw, 6.0);
            round_fill(pm, b, 3.0, j::BLANC, 0.1);
            round_fill(pm, Rect::new(b.x, b.y, (b.w * g).max(6.0), b.h), 3.0, pal::up(), 1.0);
        }
        if s == Section::Journal {
            courbe(pm, Rect::new(lx, y + 6.0, lw, 96.0), &c.courbe);
        }
        if s == Section::Thermique && !c.chaleur.is_empty() {
            thermique(pm, text, Rect::new(lx, y, lw, 120.0), &c.chaleur);
        }
    }

    fn draw_bienvenue(&self, pm: &mut Pixmap, text: &Text, w: f64, h: f64, mouse: (f64, f64)) {
        let e = self.bienvenue.unwrap_or(0);
        let gras = Text::graisse(Graisse::Bold);
        // fond : nuit (rgba 5, 7, 12, 0,82) et une lueur bleue en haut à gauche (maquette #bienvenue)
        crate::fill(pm, 0.0, 0.0, w, h, Rgb(5, 7, 12), 0.93); // (la maquette floute en plus : on fonce davantage)
        if let Some(fort) = force_lueur(crate::palette::is_light(crate::theme::theme_index())) {
            lueur(pm, w * 0.3, h * 0.2, w.max(h) * 0.6, j::ACCENT, fort);
        }
        let p = self.bv_plan(w, h);
        j::verre(pm, p.r, 24.0);
        for (x, y, l, size, fort) in &p.textes {
            let (f, c) = if *fort { (&gras, j::TEXTE) } else { (text, j::DIM) };
            f.draw(pm, l, (*x as f32, *y as f32), *size, c, Align::Left);
        }
        let on = |r: &Rect| r.contains(mouse.0, mouse.1);
        for (r, id, ic, nom, desc) in &p.cartes {
            let sel = match id {
                0..=3 => self.profil == *id as usize,
                _ => self.verre_clair == (*id == 21),
            };
            round_fill(pm, *r, 16.0, j::VERRE2.0, if on(r) { 0.11 } else { j::VERRE2.1 });
            if sel {
                round_stroke(pm, Rect::new(r.x - 2.0, r.y - 2.0, r.w + 4.0, r.h + 4.0), 18.0, j::ACCENT, 2.0 * 0.4 + 1.0);
                round_stroke(pm, *r, 16.0, j::ACCENT, 1.0);
            } else {
                round_stroke(pm, *r, 16.0, j::sur_fond(j::LISERE), 1.0);
            }
            if *id >= 20 {
                // aperçu du thème (.bv-th) : fond, deux traits, un bloc
                let clair = *id == 21;
                let (f, tx) = if clair {
                    (Rgb(0xe9, 0xed, 0xf5), Rgb(0x14, 0x18, 0x24))
                } else {
                    (Rgb(0x0b, 0x0e, 0x16), Rgb(0xe8, 0xea, 0xf0))
                };
                let a = Rect::new(r.x + 12.0, r.y + 12.0, r.w - 24.0, 70.0);
                round_fill(pm, a, 12.0, f, 1.0);
                round_stroke(pm, a, 12.0, j::sur_fond(j::LISERE), 1.0);
                round_fill(pm, Rect::new(a.x + 10.0, a.y + 10.0, (a.w - 20.0) * 0.85, 6.0), 3.0, tx, 0.6);
                round_fill(pm, Rect::new(a.x + 10.0, a.y + 22.0, (a.w - 20.0) * 0.6, 6.0), 3.0, tx, 0.6);
                round_fill(pm, Rect::new(a.x + 10.0, a.y + 34.0, a.w - 20.0, a.h - 44.0), 8.0, Rgb(128, 128, 128), 0.25);
                gras.draw(pm, nom, ((r.x + 12.0) as f32, (r.y + 104.0) as f32), 14.0, j::TEXTE, Align::Left);
                continue;
            }
            let couleur = [crate::palette::GREEN, j::TIER2, j::ACCENT, j::VIOLET][(*id as usize).min(3)];
            crate::phosphor::draw(pm, ic, r.x + 23.0, r.y + 25.0, 22, couleur);
            gras.draw(pm, nom, ((r.x + 12.0) as f32, (r.y + 56.0) as f32), 14.0, j::TEXTE, Align::Left);
            for (k, l) in desc.iter().enumerate() {
                text.draw(pm, l, ((r.x + 12.0) as f32, (r.y + 74.0 + k as f64 * 16.0) as f32), 12.0, j::DIM, Align::Left);
            }
        }
        for (r, b, d) in &p.blocs {
            round_fill(pm, *r, 14.0, j::VERRE2.0, j::VERRE2.1);
            round_stroke(pm, *r, 14.0, j::sur_fond(j::LISERE), 1.0);
            gras.draw(pm, b, ((r.x + 12.0) as f32, (r.y + 26.0) as f32), 13.0, j::TEXTE, Align::Left);
            for (k, l) in d.iter().enumerate() {
                text.draw(pm, l, ((r.x + 12.0) as f32, (r.y + 46.0 + k as f64 * 16.0) as f32), 12.0, j::DIM, Align::Left);
            }
        }
        // les 3 points : celui de la page en pilule bleue
        let (mut px, py) = p.points;
        for k in 0..3u8 {
            let lw = if k == e { 20.0 } else { 8.0 };
            let (c, a) = if k == e { (j::ACCENT, 1.0) } else { (j::BLANC, 0.2) };
            round_fill(pm, Rect::new(px, py - 4.0, lw, 8.0), 4.0, c, a);
            px += lw + 5.0;
        }
        for (z, id) in p.zones.iter().filter(|(_, id)| matches!(id, 10..=13)) {
            let (t, fond) = match id {
                10 => ("‹ Retour", false),
                11 => (if e < 2 { "Suivant ›" } else { "Arriver sur l'accueil ›" }, true),
                12 => (BV_CLE, false),
                _ => ("Passer", false),
            };
            if *id == 13 {
                // lien : sans fond
                text.draw(
                    pm,
                    t,
                    (z.cx() as f32, (z.cy() + 4.5) as f32),
                    12.5,
                    if on(z) { j::TEXTE } else { j::DIM },
                    Align::Center,
                );
                continue;
            }
            puce(pm, *z, fond, on(z));
            let f = if fond { &gras } else { text };
            f.draw(
                pm,
                t,
                (z.cx() as f32, (z.cy() + 4.5) as f32),
                12.5,
                if fond { j::BLANC } else { j::TEXTE },
                Align::Center,
            );
        }
    }
}

/// 👋 Premier lancement : la carte et ce qu'elle contient (coordonnées de la page).
#[derive(Default)]
struct BvPlan {
    r: Rect,
    /// (x, ligne de base, texte, taille, gras)
    textes: Vec<(f64, f64, String, f32, bool)>,
    /// (carte, id, icône, nom, description coupée)
    cartes: Vec<(Rect, u8, String, String, Vec<String>)>,
    /// (bloc, titre, lignes)
    blocs: Vec<(Rect, String, Vec<String>)>,
    zones: Vec<(Rect, u8)>,
    points: (f64, f64),
}

/// 👋 Les 3 pages de la maquette : titre et sous-titre.
const BV_TEXTES: [(&str, &str); 3] = [
    (
        "Bienvenue dans HSS Trading",
        "1 / 3 · Quel trader es-tu ? Ça règle la Communauté (salons, défis, devoirs). Tu pourras changer plus tard.",
    ),
    ("Ton thème", "2 / 3 · Le verre, en sombre ou en clair."),
    ("Tes données de marché", "3 / 3 · Par défaut : sources gratuites, sans compte ni clé."),
];

/// 👤 Les profils de la maquette (communaute-donnees.js) : icône (Phosphor, à la place de 🌱 🏕 📈 🧭 que la police n'a
/// pas toutes), nom, description — dans l'ordre de `PROFILS`.
const BV_PROFILS: [(&str, &str, &str); 4] = [
    ("tree-evergreen", "Débutant", "Les bases : wins, annonces, news éco, questions, parcours 0."),
    ("fire", "Bootcamp", "Devoirs de la semaine, coaching, lives bootcamp."),
    ("chart-line-up", "Trader", "Tous les salons de marché + coaching."),
    ("compass", "Mentor", "Tout, plus « Mes étudiants » et la correction des devoirs."),
];

const BV_CLE: &str = "+ Ajouter une clé API";

/// Coupe `s` en lignes d'au plus `max` points (par mots).
fn couper(t: &Text, s: &str, size: f32, max: f64) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    let mut cur = String::new();
    for mot in s.split(' ') {
        let essai = if cur.is_empty() { mot.to_string() } else { format!("{cur} {mot}") };
        if !cur.is_empty() && t.width(&essai, size) as f64 > max {
            out.push(std::mem::take(&mut cur));
            cur = mot.to_string();
        } else {
            cur = essai;
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// 💊 Une puce de la maquette (`.puce`, `.puce.on`) : pilule de verre, ou bleue pleine.
fn puce(pm: &mut Pixmap, r: Rect, on: bool, survol: bool) {
    if on {
        round_fill(pm, r, r.h / 2.0, j::ACCENT, if survol { 1.0 } else { 0.9 });
    } else {
        round_fill(pm, r, r.h / 2.0, j::VERRE2.0, if survol { 0.12 } else { j::VERRE2.1 });
        round_stroke(pm, r, r.h / 2.0, j::sur_fond(j::LISERE), 1.0);
    }
}

/// ✨ Lueur ronde (dégradé radial de `c` à transparent).
/// 🌞 The strength of the welcome screens' blue glow; `None` in a light theme (« Verre clair »…). The page is put on the
/// screen in glass (`glass`): its see-through night becomes the light glass, but where the glow was strong enough its
/// pixels turned whole (opaque) and were kept as they are: a night-blue disc in the white page, with a stair-step edge
/// where the glow faded. Without the glow, the whole night is glass, light like the rest of the page.
fn force_lueur(clair: bool) -> Option<f32> {
    (!clair).then_some(0.25)
}

fn lueur(pm: &mut Pixmap, cx: f64, cy: f64, rayon: f64, c: Rgb, a: f32) {
    use tiny_skia::{Color, GradientStop, Paint, Point, RadialGradient, SpreadMode};
    let k = crate::px::dev(1.0) as f32;
    let Some(sh) = RadialGradient::new(
        Point::from_xy(cx as f32 * k, cy as f32 * k),
        Point::from_xy(cx as f32 * k, cy as f32 * k),
        rayon as f32 * k,
        vec![
            GradientStop::new(0.0, Color::from_rgba8(c.0, c.1, c.2, (a * 255.0) as u8)),
            GradientStop::new(1.0, Color::from_rgba8(c.0, c.1, c.2, 0)),
        ],
        SpreadMode::Pad,
        tiny_skia::Transform::identity(),
    ) else {
        return;
    };
    let paint = Paint {
        shader: sh,
        anti_alias: true,
        ..Default::default()
    };
    if let Some(r) = tiny_skia::Rect::from_xywh(0.0, 0.0, pm.width() as f32, pm.height() as f32) {
        pm.fill_rect(r, &paint, tiny_skia::Transform::identity(), None);
    }
}

/// 🌈 Bande dégradée de l'Accueil (`.acc-bande`) : cyan → bleu → violet de gauche à droite, qui s'efface vers le bas.
fn bande(pm: &mut Pixmap, r: Rect) {
    use tiny_skia::{Color, GradientStop, LinearGradient, Paint, Point, SpreadMode};
    let pas = 3.0;
    let n = (r.h / pas).ceil() as usize;
    for k in 0..n {
        let f = 1.0 - k as f32 / n as f32; // fondu vertical (masque de la maquette)
        let c = |c: Rgb, a: f32| Color::from_rgba8(c.0, c.1, c.2, (a * f * 255.0).round() as u8);
        let stops = vec![
            GradientStop::new(0.0, c(j::CYAN, 0.0)),
            GradientStop::new(0.18, c(j::CYAN, 0.35)),
            GradientStop::new(0.5, c(j::ACCENT, 0.35)),
            GradientStop::new(0.82, c(j::VIOLET, 0.4)),
            GradientStop::new(1.0, c(j::VIOLET, 0.0)),
        ];
        let Some(shader) = LinearGradient::new(
            Point::from_xy(r.x as f32, 0.0),
            Point::from_xy((r.x + r.w) as f32, 0.0),
            stops,
            SpreadMode::Pad,
            tiny_skia::Transform::identity(),
        ) else {
            return;
        };
        let paint = Paint {
            shader,
            anti_alias: false,
            ..Paint::default()
        };
        if let Some(b) = tiny_skia::Rect::from_xywh(r.x as f32, (r.y + k as f64 * pas) as f32, r.w as f32, pas as f32) {
            pm.fill_rect(b, &paint, crate::px::t(), None);
        }
    }
}

/// ⌨️ Une touche (`kbd`) : petit cadre arrondi avec son texte.
fn touche(pm: &mut Pixmap, text: &Text, r: Rect, t: &str) {
    // 📱 phone view: no keyboard, so no key cap (« Ctrl » « K » of the search, the letter in each card's corner)
    if !crate::actions::montrer_raccourcis() {
        return;
    }
    round_fill(pm, r, 6.0, j::BLANC, 0.06);
    round_stroke(pm, r, 6.0, j::sur_fond((j::BLANC, 0.3)), 1.0);
    text.draw(pm, t, (r.cx() as f32, (r.cy() + 4.5) as f32), 12.0, j::TEXTE, Align::Center);
}

/// Contour en pointillés d'un rectangle arrondi (tuile « Nouvelle mise en page »).
fn pointilles(pm: &mut Pixmap, r: Rect, rayon: f64, c: Rgb) {
    use tiny_skia::{Paint, PathBuilder, Stroke, StrokeDash};
    let k = rayon as f32;
    let (x, y, w, h) = (r.x as f32, r.y as f32, r.w as f32, r.h as f32);
    let mut pb = PathBuilder::new();
    pb.move_to(x + k, y);
    pb.line_to(x + w - k, y);
    pb.quad_to(x + w, y, x + w, y + k);
    pb.line_to(x + w, y + h - k);
    pb.quad_to(x + w, y + h, x + w - k, y + h);
    pb.line_to(x + k, y + h);
    pb.quad_to(x, y + h, x, y + h - k);
    pb.line_to(x, y + k);
    pb.quad_to(x, y, x + k, y);
    pb.close();
    let Some(path) = pb.finish() else { return };
    let stroke = Stroke {
        width: 1.2,
        dash: StrokeDash::new(vec![5.0, 4.0], 0.0),
        ..Stroke::default()
    };
    let mut paint = Paint::default();
    paint.set_color_rgba8(c.0, c.1, c.2, 255);
    paint.anti_alias = true;
    pm.stroke_path(&path, &paint, &stroke, crate::px::t(), None);
}

/// 👥 Aperçu de la Communauté : la vignette d'un live (▶) en attendant le vrai fil (« bientôt »).
fn communaute(pm: &mut Pixmap, text: &Text, r: Rect) {
    round_fill(pm, r, 12.0, j::FOND, 1.0);
    // une petite courbe décorative cyan (comme la vignette de la maquette), sans donnée inventée
    let n = 40;
    let pts: Vec<(f32, f32)> = (0..=n)
        .map(|i| {
            let t = i as f64 / n as f64;
            let v = (t * 19.0).sin() * 0.3 + (t * 7.0).cos() * 0.25;
            ((r.x + t * r.w) as f32, (r.y + r.h * (0.5 - v * 0.6)) as f32)
        })
        .collect();
    crate::shapes::lines_alpha(pm, 0.0, 0.0, &[&pts], j::CYAN, 2.0, 0.55);
    let c = (r.cx(), r.y + r.h * 0.44);
    round_fill(pm, Rect::new(c.0 - 24.0, c.1 - 24.0, 48.0, 48.0), 24.0, j::BLANC, 1.0);
    crate::shapes::poly_fill(pm, c.0, c.1, &[(-6.0, -10.0), (11.0, 0.0), (-6.0, 10.0)], j::FOND, 1.0);
    let t = text.fit("Lives, wins et Bootcamp : bientôt dans Argos", j::T_PETIT, r.w - 16.0);
    text.draw(pm, &t, ((r.x + 8.0) as f32, (r.y + r.h - 10.0) as f32), j::T_PETIT, j::TEXTE, Align::Left);
}

/// 🟩 Carte thermique : une tuile par symbole, verte qui monte ou rouge qui baisse, plus vive quand ça bouge fort
/// (comme la maquette : 2 rangées, tailles variées). Sans variation connue : gris.
fn thermique(pm: &mut Pixmap, text: &Text, r: Rect, v: &[(String, Option<f64>)]) {
    const SPANS: [usize; 8] = [3, 2, 2, 1, 2, 1, 1, 2];
    let cols = 7.0;
    let (cw, ch) = ((r.w - 6.0 * 3.0) / cols, (r.h - 6.0 * 2.0) / 3.0);
    let (mut col, mut row) = (0usize, 0usize);
    for (k, (sym, p)) in v.iter().enumerate() {
        let span = SPANS[k % SPANS.len()];
        if col + span > 7 {
            (col, row) = (0, row + 1);
        }
        if row > 2 {
            break;
        }
        let t = Rect::new(
            r.x + col as f64 * (cw + 3.0),
            r.y + row as f64 * (ch + 6.0),
            span as f64 * cw + (span as f64 - 1.0) * 3.0,
            ch,
        );
        let (c, a) = match p {
            Some(x) if *x >= 0.0 => (pal::up(), 0.25 + (x / 3.0).min(0.6) as f32),
            Some(x) => (pal::down(), 0.25 + (-x / 3.0).min(0.6) as f32),
            None => (j::DIM, 0.2),
        };
        round_fill(pm, t, 6.0, c, a);
        let nom = text.fit(sym.split('-').next().unwrap_or(sym), 12.0, t.w - 6.0);
        text.draw(pm, &nom, (t.cx() as f32, (t.cy() - 2.0) as f32), 12.0, j::TEXTE, Align::Center);
        let pc = p.map_or("—".to_string(), |x| format!("{x:+.2} %").replace('.', ","));
        text.draw(pm, &pc, (t.cx() as f32, (t.cy() + 13.0) as f32), 11.0, j::TEXTE, Align::Center);
        col += span;
    }
}

/// 📓 Courbe d'équité (carte Journal) : la ligne et le départ en pointillé gris ; rien sans données.
fn courbe(pm: &mut Pixmap, r: Rect, v: &[f64]) {
    if v.len() < 2 {
        return;
    }
    let (lo, hi) = v.iter().fold((f64::MAX, f64::MIN), |(a, b), x| (a.min(*x), b.max(*x)));
    let span = (hi - lo).max(1e-9);
    let pts: Vec<(f32, f32)> = v
        .iter()
        .enumerate()
        .map(|(i, x)| ((r.x + i as f64 * r.w / (v.len() - 1) as f64) as f32, (r.y + (hi - x) / span * r.h) as f32))
        .collect();
    let c = pal::pnl(v[v.len() - 1] - v[0]);
    crate::shapes::lines(pm, 0.0, 0.0, &[&pts], c, 2.0);
}

/// ⌂ Petite maison (bouton Accueil) centrée en (cx, cy).
pub fn maison(pm: &mut Pixmap, cx: f64, cy: f64, c: Rgb) {
    crate::shapes::lines(
        pm,
        cx,
        cy,
        &[
            &[(-8.0, -1.0), (0.0, -8.0), (8.0, -1.0)],
            &[(-6.0, -2.5), (-6.0, 7.0), (6.0, 7.0), (6.0, -2.5)],
            &[(-2.0, 7.0), (-2.0, 2.0), (2.0, 2.0), (2.0, 7.0)],
        ],
        c,
        1.6,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 🌞 B (« Verre clair »): the glass puts the page's night (its most frequent colour) as glass and keeps any pixel
    /// more than 6 away from it as it is. Under the glow, the night's pixels are more than that away (a night-blue disc
    /// kept in the white page, a stair-step edge); in a light theme there is no glow: every pixel of the night is the
    /// night, all of it glass.
    #[test]
    fn lueur_de_bienvenue_eclaircie_en_theme_clair() {
        let ecart_max = |a: Option<f32>| {
            let mut pm = Pixmap::new(200, 200).unwrap();
            // (the night of the welcome screen: rgb 5, 7, 12 at 0.93)
            pm.fill(tiny_skia::Color::from_rgba8(5, 7, 12, 237));
            let nuit = pm.pixels()[0].demultiply();
            if let Some(a) = a {
                lueur(&mut pm, 60.0, 40.0, 120.0, j::ACCENT, a);
            }
            pm.pixels()
                .iter()
                .map(|p| {
                    let c = p.demultiply();
                    let d = |x: u8, y: u8| (x as i32 - y as i32).abs();
                    d(c.red(), nuit.red()) + d(c.green(), nuit.green()) + d(c.blue(), nuit.blue())
                })
                .max()
                .unwrap_or(0)
        };
        assert_eq!(force_lueur(false), Some(0.25), "the dark themes keep their glow");
        assert!(ecart_max(force_lueur(false)) > 6, "the cause: under the glow, pixels that are not the night");
        assert_eq!(force_lueur(true), None);
        assert_eq!(ecart_max(force_lueur(true)), 0, "a light theme: the night whole, all of it glass");
    }

    fn ev(t: &str, impact: &str, ts: i64) -> Event {
        Event {
            title: t.into(),
            country: "USD".into(),
            impact: impact.into(),
            ts,
            ..Default::default()
        }
    }

    #[test]
    fn lettres_des_sections() {
        let lettres: String = Section::ALL.iter().map(|s| s.lettre()).collect();
        assert_eq!(lettres, "CAFEPNHJMG");
        for s in Section::ALL {
            assert_eq!(Section::depuis_lettre(s.lettre()), Some(s));
            assert_eq!(Section::depuis_lettre(s.lettre().to_ascii_lowercase()), Some(s), "minuscule aussi");
        }
        assert_eq!(Section::depuis_lettre('x'), None);
        assert_eq!(Section::depuis_lettre('z'), None);
    }

    #[test]
    fn trois_prochaines_news_en_heure_et() {
        let now = 1_762_867_800; // mar. 11 nov. 2025, 13:30 UTC
        let agenda = vec![
            ev("Passé", "High", now - 60),
            ev("Faible", "Low", now + 60),
            ev("PPI", "Medium", now + 7200),
            ev("CPI", "High", now + 3600),
            ev("Claims", "High", now + 86_400),
            ev("Retail", "High", now + 90_000),
        ];
        let v: Vec<&str> = prochaines_news(&agenda, now, 3).iter().map(|e| e.title.as_str()).collect();
        assert_eq!(v, ["CPI", "PPI", "Claims"]);
        // 13:30 UTC = 08:30 à New York (heure normale de l'Est)
        assert_eq!(heure_et(now), "mar. 11 nov. · 08:30 ET");
    }

    #[test]
    fn semaine_du_journal() {
        let now = 1_000 * 86_400_000;
        let t = |jours: i64, entry: f64, exit: Option<f64>| JTrade {
            entry: Some(entry),
            sl: Some(entry - 10.0),
            exit,
            side: "long".into(),
            risk: Some(100.0),
            entry_ts: now - jours * 86_400_000,
            exit_ts: exit.map(|_| now - jours * 86_400_000),
            ..Default::default()
        };
        let trades = vec![
            t(1, 100.0, Some(120.0)), // +2 R
            t(2, 100.0, Some(90.0)),  // −1 R
            t(3, 100.0, None),        // ouvert
            t(9, 100.0, Some(130.0)), // trop vieux
        ];
        let s = semaine(&trades, now);
        assert_eq!(s.trades, 3);
        assert_eq!(s.reussite.map(|r| r.round()), Some(50.0));
        assert_eq!(s.r_moyen, Some(0.5));
        let vide = semaine(&[], now);
        assert_eq!((vide.trades, vide.reussite), (0, None));
    }

    #[test]
    fn cartes_branchees_sur_les_donnees() {
        let d = Donnees {
            academie: (3, 20, Some("Structure de marché".into())),
            garde: Some(Garde {
                perte_jour: Money::from_cents(120_000),
                limite_jour: Money::from_cents(500_000),
                pnl: Money::from_cents(420_000),
                objectif: Money::from_cents(1_000_000),
                jours: 3,
                jours_min: 4,
            }),
            news: vec![("CPI m/m".into(), "USD".into(), 1_762_867_800, true)],
            demo: Some(Demo {
                equite: Money::from_cents(10_041_200),
                jour: Money::from_cents(-1_250),
                positions: vec![("BTC long".into(), Money::from_cents(4_000))],
            }),
            marches: vec![("BTC".into(), "97 000".into(), Some(1.5))],
            macro_: vec![("VIX".into(), 15.2, -2.0), ("Dollar (DXY)".into(), 99.1, 0.1)],
            semaine: Semaine {
                trades: 7,
                reussite: Some(57.0),
                r_moyen: Some(0.6),
            },
            courbe: vec![100_000.0, 100_400.0, 100_412.0],
            actus: vec![("Le dollar recule avant le CPI".into(), "FXStreet".into(), 1_000)],
            chaleur: vec![("BTC-USDC".into(), Some(-0.18)), ("ETH-USDC".into(), Some(0.41))],
            maintenant: 1_000 + 7_200,
        };
        let c = cartes(&d);
        assert_eq!(c.iter().find(|c| c.section == Section::Journal).unwrap().courbe.len(), 3);
        assert_eq!(c.len(), 10);
        assert!(c.iter().zip(Section::ALL).all(|(c, s)| c.section == s), "ordre de la mosaïque");
        let actu = c.iter().find(|c| c.section == Section::Actualites).unwrap();
        assert_eq!((actu.lignes[0].d.as_str(), actu.lignes[0].tag.map(|t| t.0)), ("il y a 2 h", Some("FXStreet")));
        assert_eq!(c.iter().find(|c| c.section == Section::Thermique).unwrap().chaleur.len(), 2);
        let get = |s: Section| c.iter().find(|c| c.section == s).unwrap();
        assert_eq!(get(Section::Academie).bouton, Some("▶ Reprendre la leçon"));
        assert_eq!(get(Section::PropFirm).bouton, Some("🧪 Simulateur"));
        assert_eq!(get(Section::Academie).jauge, Some(0.15));
        assert_eq!(get(Section::PropFirm).lignes[0].d, "1 200 $ / 5 000 $");
        assert_eq!(get(Section::PropFirm).jauge, Some(0.24));
        assert_eq!(get(Section::Portefeuille).lignes[0].d, "100 412 $");
        assert_eq!(get(Section::Portefeuille).lignes[1].d, "−12 $");
        assert_eq!(get(Section::Calendrier).lignes[0].d, "mar. 11 nov. · 08:30 ET");
        assert_eq!(get(Section::Calendrier).lignes[0].tag.map(|t| t.0), Some("Tier 1"));
        assert_eq!(get(Section::Journal).lignes[1].d, "57 %");
        assert_eq!(get(Section::Journal).lignes[2].d, "+0,60 R");
        assert_eq!(get(Section::Macro).lignes.len(), 2, "VIX et DXY trouvés, pas le 10 ans");
        assert!(get(Section::Communaute).lignes[0].d.contains("bientôt"), "placeholder propre");
        // sans données : rien d'inventé
        let vide = cartes(&Donnees::default());
        assert!(vide.iter().find(|c| c.section == Section::Macro).unwrap().lignes.iter().all(|l| l.d == "—"));
        assert!(vide.iter().find(|c| c.section == Section::Portefeuille).unwrap().lignes[0]
            .g
            .contains("Pas encore"));
    }

    #[test]
    fn premier_lancement_en_trois_ecrans() {
        let t = Text::default();
        let (w, h) = (1280.0, 800.0);
        let mut p = AccueilPage {
            bienvenue: Some(0),
            ..Default::default()
        };
        let zone = |p: &AccueilPage, id: u8| p.bv_zones(w, h).into_iter().find(|(_, i)| *i == id).unwrap().0;
        let r = zone(&p, 2);
        assert_eq!(p.click(&t, w, h, r.cx(), r.cy(), &[]), AccClick::Profil);
        assert_eq!(PROFILS[p.profil], "Trader");
        for e in 1..=2 {
            let r = zone(&p, 11);
            assert_eq!(p.click(&t, w, h, r.cx(), r.cy(), &[]), AccClick::None);
            assert_eq!(p.bienvenue, Some(e));
        }
        let r = zone(&p, 12);
        assert_eq!(p.click(&t, w, h, r.cx(), r.cy(), &[]), AccClick::CleApi);
        assert!(p.draw(&t, 1280, 800, (0.0, 0.0), &cartes(&Donnees::default())).is_some());
        let r = zone(&p, 11);
        assert_eq!(p.click(&t, w, h, r.cx(), r.cy(), &[]), AccClick::BienvenueFin);
        assert_eq!(p.bienvenue, None);
        // Échap pendant le premier lancement : terminé aussi (une seule fois), puis Échap ferme la page
        let mut p = AccueilPage {
            bienvenue: Some(1),
            ..Default::default()
        };
        assert_eq!(p.escape(), AccClick::BienvenueFin);
        assert_eq!(p.escape(), AccClick::Close);
    }

    #[test]
    fn clics_de_la_page() {
        let t = Text::default();
        let (w, h) = (1280.0, 900.0);
        let c = cartes(&Donnees {
            academie: (3, 49, Some("Double top".into())),
            garde: Some(Garde::default()),
            ..Default::default()
        });
        let mut p = AccueilPage {
            layouts: vec![
                ("Argos".into(), "BTC · 4h".into(), true),
                ("Or".into(), "XAU · 15m".into(), false),
                ("FX".into(), "EUR · 1h".into(), false),
                ("Nuit".into(), "ETH · 1D".into(), false),
            ],
            heure: 9,
            profil: 2,
            ..Default::default()
        };
        let (zones, _) = p.zones(&t, w, &c);
        let at = |z: Zone| zones.iter().find(|(_, x)| *x == z).unwrap_or_else(|| panic!("{z:?}")).0;
        let clic = |p: &mut AccueilPage, r: Rect| p.click(&t, w, h, r.cx(), r.cy() + HEAD, &c);
        assert_eq!(clic(&mut p, at(Zone::Recherche)), AccClick::Recherche);
        assert_eq!(clic(&mut p, at(Zone::Nouvelle)), AccClick::NouvelleLayout);
        assert_eq!(clic(&mut p, at(Zone::Layout(1))), AccClick::Layout(1));
        // ★ et ⋯ de la tuile (maquette) : par-dessus la tuile, le menu s'ouvre sous ⋯ (coordonnées de la fenêtre)
        assert_eq!(clic(&mut p, at(Zone::Etoile(1))), AccClick::Favori(1));
        let plus = at(Zone::Plus(1));
        assert_eq!(clic(&mut p, plus), AccClick::MenuLayout(1, Rect::new(plus.x, plus.y + HEAD, plus.w, plus.h)));
        let tuile = at(Zone::Layout(1));
        assert!(
            plus.x + plus.w <= tuile.x + tuile.w && at(Zone::Etoile(1)).x + 28.0 <= plus.x,
            "★ puis ⋯ dans la tuile"
        );
        p.favoris = vec!["Or".into()];
        assert!(p.draw(&t, w as u32, h as u32, (0.0, 0.0), &c).is_some());
        assert_eq!(clic(&mut p, at(Zone::Carte(Section::Macro))), AccClick::Section(Section::Macro));
        assert_eq!(clic(&mut p, at(Zone::Bouton(Section::Academie))), AccClick::Reprendre);
        assert_eq!(clic(&mut p, at(Zone::Bouton(Section::PropFirm))), AccClick::Simulateur);
        assert!(!zones.iter().any(|(_, z)| *z == Zone::Bouton(Section::Journal)), "pas de bouton sur le Journal");
        assert_eq!(clic(&mut p, at(Zone::Profil)), AccClick::Profil);
        assert_eq!(p.profil, 3, "Trader → Mentor");
        assert_eq!(p.click(&t, w, h, w - 34.0, 28.0, &c), AccClick::Close);
        // 3 tuiles + « Nouvelle » ; « Toutes les mises en page » montre la 4e
        assert!(!zones.iter().any(|(_, z)| *z == Zone::Layout(3)));
        assert_eq!(clic(&mut p, at(Zone::Tout)), AccClick::None);
        assert!(p.tout && p.zones(&t, w, &c).0.iter().any(|(_, z)| *z == Zone::Layout(3)));
        // mosaïque de la maquette : Communauté + Académie sur une rangée, puis 3 cartes par rangée
        let r = |z: &[(Rect, Zone)], s: Section| z.iter().find(|(_, x)| *x == Zone::Carte(s)).unwrap().0;
        assert_eq!(r(&zones, Section::Communaute).y, r(&zones, Section::Academie).y);
        assert_eq!(r(&zones, Section::PropFirm).y, r(&zones, Section::Portefeuille).y);
        assert!(r(&zones, Section::Communaute).w > r(&zones, Section::PropFirm).w * 1.4);
        assert!(r(&zones, Section::Marches).y > r(&zones, Section::PropFirm).y);
        // cartes dans la page, sans chevauchement, et chaque bouton dans sa carte (PC, écran moyen, téléphone)
        for w in [1600.0, 1280.0, 1000.0, 390.0] {
            let (z, _) = p.zones(&t, w, &c);
            let cs: Vec<Rect> = z.iter().filter(|(_, x)| matches!(x, Zone::Carte(_))).map(|(r, _)| *r).collect();
            assert_eq!(cs.len(), 10);
            assert!(cs.iter().all(|r| r.x >= 0.0 && r.x + r.w <= w + 0.1 && r.h >= j::HAUT_CARTE_MIN), "largeur {w}");
            for (i, a) in cs.iter().enumerate() {
                for b in &cs[i + 1..] {
                    assert!(a.x + a.w <= b.x + 0.1 || b.x + b.w <= a.x + 0.1 || a.y + a.h <= b.y + 0.1 || b.y + b.h <= a.y + 0.1);
                }
            }
            for (b, zb) in z.iter().filter(|(_, x)| matches!(x, Zone::Bouton(_))) {
                let Zone::Bouton(s) = zb else { continue };
                let carte = r(&z, *s);
                assert!(b.y >= carte.y && b.y + b.h <= carte.y + carte.h, "bouton {s:?} dans sa carte ({w})");
            }
            assert!(p.draw(&t, w as u32, 800, (0.0, 0.0), &c).is_some());
        }
        assert_eq!(p.salut(), "Bonjour");
        p.heure = 20;
        assert_eq!(p.salut(), "Bonsoir");
    }

    /// 📸 Captures de l'Accueil pour la comparaison avec la maquette (1440 × 900, comme ses captures) : seulement si
    /// `ARGOS_CAPTURES` donne un dossier. Données d'exemple, comme celles de la maquette.
    #[test]
    fn captures_pour_jee() {
        let Ok(dir) = std::env::var("ARGOS_CAPTURES") else { return };
        let t = Text::default();
        let m = Money::from_cents;
        let d = Donnees {
            academie: (12, 49, Some("Double top (M)".into())),
            garde: Some(Garde {
                perte_jour: m(80_000),
                limite_jour: m(500_000),
                pnl: m(420_000),
                objectif: m(1_000_000),
                jours: 3,
                jours_min: 4,
            }),
            news: vec![
                ("CPI (inflation)".into(), "USD".into(), 1_794_403_800, true),
                ("PPI (prix à la production)".into(), "USD".into(), 1_794_490_200, false),
                ("Confiance des ménages (UMich)".into(), "USD".into(), 1_794_582_000, false),
            ],
            demo: Some(Demo {
                equite: m(10_041_200),
                jour: m(70_000),
                positions: vec![("BTC long".into(), m(40_000)), ("ETH short".into(), m(-10_000))],
            }),
            marches: vec![
                ("BTC-USDC".into(), "74 495".into(), Some(-1.62)),
                ("ETH-USDC".into(), "3 512".into(), Some(0.41)),
                ("SOL-USDC".into(), "187,4".into(), Some(1.02)),
                ("XAU/USD".into(), "2 650,5".into(), Some(0.35)),
            ],
            macro_: vec![
                ("VIX".into(), 15.2, -2.0),
                ("Dollar (DXY)".into(), 102.279, 0.7),
                ("Taux US 10 ans".into(), 4.31, 0.2),
            ],
            semaine: Semaine {
                trades: 7,
                reussite: Some(57.0),
                r_moyen: Some(0.6),
            },
            courbe: (0..40).map(|i| 100_000.0 + i as f64 * 12.0 + ((i * 7) % 5) as f64 * 30.0).collect(),
            actus: vec![
                (
                    "Emplois US : création d'emplois au-dessus des attentes".into(),
                    "MarketWatch".into(),
                    1_794_160_000,
                ),
                ("L'or recule avant le CPI de mercredi".into(), "FXStreet".into(), 1_794_150_000),
            ],
            chaleur: ["BTC", "ETH", "SOL", "HYPE", "ZEC", "XMR", "XRP", "NEAR"]
                .iter()
                .zip([-0.18, 0.41, 1.02, -0.63, 2.11, -0.22, 0.05, -1.4])
                .map(|(n, v)| (n.to_string(), Some(v)))
                .collect(),
            maintenant: 1_794_170_000,
        };
        let c = cartes(&d);
        let mut p = AccueilPage {
            layouts: vec![
                ("Argos".into(), "BTC-USDC.P · 4h · aujourd'hui".into(), true),
                ("Or du matin".into(), "XAU/USD · 15m · hier".into(), false),
                ("Double Structure FX".into(), "EUR/USD · 1h · 5 nov.".into(), false),
                ("Nuit crypto".into(), "ETH · 1D".into(), false),
                ("Indices".into(), "NAS100 · 1h".into(), false),
            ],
            favoris: vec!["Argos".into(), "Or du matin".into()],
            heure: 9,
            profil: 2,
            ..Default::default()
        };
        p.draw(&t, 1440, 900, (0.0, 0.0), &c)
            .unwrap()
            .save_png(format!("{dir}/app-accueil-haut.png"))
            .unwrap();
        p.scroll = 600.0;
        p.draw(&t, 1440, 900, (0.0, 0.0), &c)
            .unwrap()
            .save_png(format!("{dir}/app-accueil-mosaique.png"))
            .unwrap();
        p.scroll = 1300.0;
        p.wheel(0.0, &t, 1440.0, 900.0, &c);
        p.draw(&t, 1440, 900, (0.0, 0.0), &c)
            .unwrap()
            .save_png(format!("{dir}/app-accueil-bas.png"))
            .unwrap();
        // tablette (1024 × 768) et cell (390 × 844)
        p.scroll = 0.0;
        for (nom, w, h) in [("tablette", 1024, 768), ("cell", 390, 844)] {
            p.draw(&t, w, h, (0.0, 0.0), &c)
                .unwrap()
                .save_png(format!("{dir}/app-accueil-{nom}.png"))
                .unwrap();
        }
        // 👋 premier lancement : les 3 pages, cell et PC
        for e in 0..3u8 {
            p.bienvenue = Some(e);
            for (nom, w, h) in [("cell", 390, 844), ("tablette", 1024, 768), ("tablette-portrait", 768, 1024), ("pc", 1440, 900)] {
                p.draw(&t, w, h, (0.0, 0.0), &c)
                    .unwrap()
                    .save_png(format!("{dir}/bienvenue-{e}-{nom}.png"))
                    .unwrap();
            }
        }
        p.bienvenue = None;
    }

    /// 👤 Le profil ordonne les cartes : chacune une seule fois ; Débutant commence par l'Académie, Mentor par la
    /// Communauté, Trader suit la maquette.
    #[test]
    fn profil_ordonne_les_cartes() {
        for (pr, nom) in PROFILS.iter().enumerate() {
            let mut v = Section::ordre(pr).to_vec();
            v.sort_by_key(|s| Section::ALL.iter().position(|x| x == s));
            assert_eq!(v, Section::ALL.to_vec(), "{nom}");
        }
        assert_eq!(Section::ordre(0)[0], Section::Academie);
        assert_eq!(Section::ordre(3)[0], Section::Communaute);
        assert_eq!(Section::ordre(2), Section::ALL);
        let t = Text::default();
        let c = cartes(&Donnees::default());
        let p = AccueilPage {
            profil: 0,
            ..Default::default()
        };
        let (z, _) = p.zones(&t, 1440.0, &c);
        let premier = z.iter().find_map(|(_, x)| if let Zone::Carte(s) = x { Some(*s) } else { None });
        assert_eq!(premier, Some(Section::Academie));
    }
}
