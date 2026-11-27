//! 🔎 Market panel of the symbol pill (Jee, 21 Nov, mock-up v2): « Chercher un marché » with the open market as a
//! token, 4 tabs (Tout · Crypto · Actions · Forex, an empty one is hidden), RÉCENTS then FAVORIS, RÉSULTATS while
//! typing or filtering, and « ⚙ Tous les filtres › » (TYPE, BOURSE, QUOTE, CLASSE chips that become blue tokens).
//! PC: opened by the mouse resting 0.3 s on the symbol pill, closed 0.3 s after it left the pill and the panel
//! (unless the field has the keyboard). By the finger: a bottom sheet (phone) or a panel under the pill (tablet).
//! Pure state, places, click, keyboard and drawing; the app gives the rows and opens the chosen market.

use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Graisse, Text};
use tiny_skia::Pixmap;

/// The mouse rests this long on the pill before the panel opens, and is away this long before it closes (seconds).
pub const DELAI: f64 = 0.3;
/// Width of the panel (PC, tablet).
pub const LARGE: f64 = 400.0;
const PAD: f64 = 12.0;
/// Heights: search field, tabs and « Tous les filtres » (PC: 44; by the finger: the touch minimum, drawn at 44 in it),
/// section title, row (PC / finger), chip (PC).
const H_BARRE: f64 = 44.0;
const H_TITRE: f64 = 26.0;
const H_LIGNE: f64 = 48.0;
const H_LIGNE_DOIGT: f64 = 56.0;
const H_PUCE: f64 = 32.0;
/// ₿ Bitcoin's orange (the small ₿ of a market against BTC).
const BTC_ORANGE: crate::text::Rgb = crate::text::Rgb(247, 147, 26);

/// A market row (given by the app).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Ligne {
    /// What opens it (the app's id).
    pub id: String,
    pub base: String,
    pub quote: String,
    pub bourse: String,
    pub perp: bool,
    /// Group of the argos-store base (« crypto », « actions », « forex », « memecoins »…).
    pub groupe: String,
    /// 24 h change (%), when known.
    pub variation: Option<f64>,
    /// 🧮 A pair computed by Argos (« XRP ÷ BTC »), not an exchange's market: in its own CALCULÉE section.
    pub calculee: bool,
}

/// The tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Onglet {
    #[default]
    Tout,
    Crypto,
    Actions,
    Forex,
}

impl Onglet {
    pub const TOUS: [Onglet; 4] = [Onglet::Tout, Onglet::Crypto, Onglet::Actions, Onglet::Forex];

    pub fn nom(self) -> &'static str {
        match self {
            Onglet::Tout => "Tout",
            Onglet::Crypto => "Crypto",
            Onglet::Actions => "Actions",
            Onglet::Forex => "Forex",
        }
    }

    /// Does a row of group `g` belong here?
    pub fn garde(self, g: &str) -> bool {
        let g = g.to_lowercase();
        match self {
            Onglet::Tout => true,
            Onglet::Crypto => !(Onglet::Actions.garde(&g) || Onglet::Forex.garde(&g)),
            // (the HIP-3 markets of Hyperliquid: stocks)
            Onglet::Actions => g.contains("action") || g.contains("stock") || g.contains("hip-3"),
            Onglet::Forex => g.contains("forex") || g.contains("oanda") || g == "fx",
        }
    }
}

/// Kind of a filter chip / token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filtre {
    Type,
    Bourse,
    Quote,
    Classe,
}

impl Filtre {
    pub const TOUS: [Filtre; 4] = [Filtre::Type, Filtre::Bourse, Filtre::Quote, Filtre::Classe];

    fn titre(self) -> &'static str {
        match self {
            Filtre::Type => "TYPE",
            Filtre::Bourse => "BOURSE",
            Filtre::Quote => "QUOTE",
            Filtre::Classe => "CLASSE",
        }
    }
}

/// A blue token of the field: (kind, value in lower case).
pub type Jeton = (Filtre, String);

/// What a click or a key asks of the app.
#[derive(Debug, Clone, PartialEq)]
pub enum MarcheClick {
    /// Nothing for the app (an internal choice may have changed: redraw).
    None,
    /// Outside the panel: it closes.
    Fermer,
    /// The search changed: the app gives new results ([`Panneau::resultats`]).
    Chercher,
    /// Open this market (row id) on the chart; the panel closes.
    Ouvrir(String),
    /// 👆 « Ma watchlist › » (by the finger): close the panel and open the whole watchlist.
    Watchlist,
}

#[derive(Debug, Clone, PartialEq)]
enum Zone {
    Champ,
    /// ✕ of a token (index).
    Jeton(usize),
    Onglet(Onglet),
    Ligne(usize),
    /// 👆 « Ma watchlist › », at the top by the finger.
    Watchlist,
    Filtres,
    Puce(Filtre, String),
}

/// State of the market panel.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Panneau {
    /// Place (window); `feuille`: the phone's bottom sheet; `doigt`: touch targets (rows of 56).
    pub rect: Rect,
    pub feuille: bool,
    pub doigt: bool,
    /// The open market (« BTC »), shown as the first token (its ✕ empties it: search everything).
    pub ouvert: Option<String>,
    pub texte: String,
    pub jetons: Vec<Jeton>,
    pub onglet: Onglet,
    /// « ⚙ Tous les filtres › » unfolded.
    pub filtres: bool,
    /// The field has the keyboard (the panel then stays open without the mouse).
    pub focus: bool,
    /// 📌 Opened by a click on the pill (PC, Jee 22 Nov): it stays open when the mouse goes away (a click outside or
    /// Escape closes it).
    pub epingle: bool,
    /// Row chosen with ↑ ↓ (index in [`Panneau::lignes`]).
    pub sel: Option<usize>,
    pub defile: f64,
    pub recents: Vec<Ligne>,
    pub favoris: Vec<Ligne>,
    /// Shown only when there is no favourite (a new profile without a watchlist): the biggest 24 h volumes of the
    /// chart's exchange the app already has.
    pub populaires: Vec<Ligne>,
    /// Results of the search (from the app's market engine), filtered here by the words and tokens.
    pub resultats: Vec<Ligne>,
    /// Chips offered under « Tous les filtres » (BOURSE: the exchanges that work here; CLASSE: the base's groups).
    pub bourses: Vec<String>,
    pub classes: Vec<String>,
    /// 📱 Room kept at the right of « Tous les filtres » at the bottom of the phone's sheet: the Argos AI round sits
    /// there (filled by the app; 0: nothing there).
    pub place_ai: f64,
}

/// TYPE and QUOTE chips (fixed).
const TYPES: [&str; 2] = ["perp", "spot"];
/// (₿ « btc »: every market against Bitcoin, Jee 23 Nov)
const QUOTES: [&str; 4] = ["usdc", "usdt", "usd", "btc"];

impl Panneau {
    /// PC / tablet: under the pill `pastille` (left edge aligned), within the window `w` × `h`; above it when the
    /// pill is low on the screen (the tablet's bottom pill).
    pub fn sous(pastille: Rect, w: f64, h: f64, doigt: bool) -> Self {
        let x = pastille.x.clamp(6.0, (w - LARGE - 6.0).max(6.0));
        let y = pastille.y + pastille.h + 6.0;
        let (y, haut) = if h - y - 10.0 >= 300.0 {
            (y, (h - y - 10.0).min(620.0))
        } else {
            let haut = (pastille.y - 16.0).clamp(160.0, 620.0);
            ((pastille.y - 6.0 - haut).max(6.0), haut)
        };
        Self {
            rect: Rect::new(x, y, LARGE.min(w - 12.0), haut),
            doigt,
            ..Default::default()
        }
    }

    /// 📱 Phone: the bottom sheet (whole width, 80 % of the height).
    pub fn feuille(w: f64, h: f64) -> Self {
        let haut = h * 0.8;
        Self {
            rect: Rect::new(0.0, h - haut, w, haut),
            feuille: true,
            doigt: true,
            ..Default::default()
        }
    }

    /// The words typed and the tokens, as filters: a typed word that is a chip's value (« binance », « perp ») filters
    /// like that chip; the other words must all be found in the row, in any order.
    fn criteres(&self) -> (Vec<Jeton>, Vec<String>) {
        let mut jetons = self.jetons.clone();
        let mut mots = vec![];
        let seul = self.texte.split_whitespace().count() == 1;
        for m in self.texte.split_whitespace().map(str::to_lowercase) {
            match self.filtre_de(&m) {
                // ₿ « btc » typed alone stays a search for Bitcoin's markets (as before); « xrp btc »: against BTC
                Some(Filtre::Quote) if seul && m == "btc" => mots.push(m),
                Some(f) => jetons.push((f, m)),
                None => mots.push(m),
            }
        }
        (jetons, mots)
    }

    /// The kind of chip a word names, if any.
    fn filtre_de(&self, m: &str) -> Option<Filtre> {
        let eq = |v: &String| v.eq_ignore_ascii_case(m);
        if TYPES.contains(&m) {
            Some(Filtre::Type)
        } else if QUOTES.contains(&m) {
            Some(Filtre::Quote)
        } else if self.bourses.iter().any(eq) {
            Some(Filtre::Bourse)
        } else if self.classes.iter().any(eq) {
            Some(Filtre::Classe)
        } else {
            None
        }
    }

    /// Does `l` pass the tokens, the words and the tab?
    fn garde(&self, l: &Ligne, jetons: &[Jeton], mots: &[String]) -> bool {
        let ok = |(f, v): &Jeton| match f {
            Filtre::Type => (v == "perp") == l.perp,
            Filtre::Bourse => l.bourse.eq_ignore_ascii_case(v),
            Filtre::Quote => l.quote.eq_ignore_ascii_case(v),
            Filtre::Classe => l.groupe.eq_ignore_ascii_case(v),
        };
        // (also the pair written without separators: « xrpbtc », « xrp/btc », « xrp-btc » find XRP against BTC)
        let tout = format!("{} {} {} {} {}{}", l.base, l.quote, l.bourse, l.groupe, l.base, l.quote).to_lowercase();
        let net = |m: &str| m.chars().filter(|c| !matches!(c, '/' | '-' | '_' | '.')).collect::<String>();
        self.onglet.garde(&l.groupe) && jetons.iter().all(ok) && mots.iter().all(|m| tout.contains(m.as_str()) || tout.contains(&net(m)))
    }

    /// The panel is searching (words or tokens): RÉSULTATS instead of RÉCENTS and FAVORIS.
    pub fn cherche(&self) -> bool {
        !self.texte.trim().is_empty() || !self.jetons.is_empty()
    }

    /// The free words of the search, for the app's market engine (tokens and chip words left out).
    pub fn requete(&self) -> String {
        self.criteres().1.join(" ")
    }

    /// The sections shown: (title, rows).
    pub fn sections(&self) -> Vec<(&'static str, Vec<&Ligne>)> {
        let (jetons, mots) = self.criteres();
        let f = |v: &'_ [Ligne]| -> Vec<usize> { (0..v.len()).filter(|&k| self.garde(&v[k], &jetons, &mots)).collect() };
        if self.cherche() {
            // ₿ the « btc » chip: « CONTRE BTC »; 🧮 the computed pairs in their own section, last
            let titre = if self.jetons.iter().any(|(g, v)| *g == Filtre::Quote && v == "btc") {
                "CONTRE BTC"
            } else {
                "RÉSULTATS"
            };
            let (calc, vrais): (Vec<&Ligne>, Vec<&Ligne>) = f(&self.resultats).into_iter().map(|k| &self.resultats[k]).partition(|l| l.calculee);
            let mut out = vec![(titre, vrais)];
            if !calc.is_empty() {
                out.retain(|(_, l)| !l.is_empty());
                out.push(("CALCULÉE", calc));
            }
            return out;
        }
        let mut out = vec![];
        let suite = if self.favoris.is_empty() { &self.populaires } else { &self.favoris };
        let titre = if self.favoris.is_empty() { "POPULAIRES" } else { "FAVORIS" };
        for (t, v) in [("RÉCENTS", &self.recents), (titre, suite)] {
            let l: Vec<&Ligne> = f(v).into_iter().map(|k| &v[k]).collect();
            if !l.is_empty() {
                out.push((t, l));
            }
        }
        out
    }

    /// Every row shown, in order (for ↑ ↓).
    pub fn lignes(&self) -> Vec<&Ligne> {
        self.sections().into_iter().flat_map(|(_, l)| l).collect()
    }

    /// The tabs shown: a tab with nothing behind it is hidden (never « bientôt »).
    pub fn onglets(&self) -> Vec<Onglet> {
        let tout: Vec<&Ligne> = self
            .recents
            .iter()
            .chain(&self.favoris)
            .chain(&self.populaires)
            .chain(&self.resultats)
            .collect();
        let groupes = self.classes.iter().map(String::as_str).chain(tout.iter().map(|l| l.groupe.as_str()));
        let g: Vec<&str> = groupes.collect();
        Onglet::TOUS
            .into_iter()
            .filter(|o| *o == Onglet::Tout || g.iter().any(|x| o.garde(x)))
            .collect()
    }

    fn chips(&self, f: Filtre) -> Vec<String> {
        match f {
            Filtre::Type => TYPES.iter().map(|s| s.to_string()).collect(),
            Filtre::Quote => QUOTES.iter().map(|s| s.to_string()).collect(),
            Filtre::Bourse => self.bourses.iter().map(|s| s.to_lowercase()).collect(),
            Filtre::Classe => self.classes.iter().map(|s| s.to_lowercase()).collect(),
        }
    }

    /// Height of the field, the tabs and « Tous les filtres » (their touch zone).
    fn h_barre(&self) -> f64 {
        if self.doigt {
            crate::formfactor::TOUCH_MIN_PT
        } else {
            H_BARRE
        }
    }

    /// Height of a chip (its touch zone).
    fn h_puce(&self) -> f64 {
        if self.doigt {
            crate::formfactor::TOUCH_MIN_PT
        } else {
            H_PUCE
        }
    }

    fn h_ligne(&self) -> f64 {
        if self.doigt {
            H_LIGNE_DOIGT
        } else {
            H_LIGNE
        }
    }

    /// Top of the content (window).
    fn haut(&self) -> f64 {
        self.rect.y + if self.feuille { 20.0 } else { PAD }
    }

    /// Height of the « Ma watchlist › » row: by the finger, a touch target; PC, only in the panel opened by a click
    /// (its way to the big Symboles panel, which the click on the pill used to open); 0: no row.
    fn h_watchlist(&self) -> f64 {
        if self.doigt {
            crate::formfactor::TOUCH_MIN_PT
        } else if self.epingle {
            36.0
        } else {
            0.0
        }
    }

    /// Top of the search field (window): under « Ma watchlist › » when it is there.
    fn haut_champ(&self) -> f64 {
        let h = self.h_watchlist();
        self.haut() + if h > 0.0 { h + 4.0 } else { 0.0 }
    }

    /// Laid-out zones (window) and the height of the content.
    fn zones(&self, text: &Text) -> (Vec<(Zone, Rect)>, f64) {
        let r = self.rect;
        let (x, w) = (r.x + PAD, r.w - 2.0 * PAD);
        let mut out = vec![];
        // 👆 by the finger: « Ma watchlist › » first, above the field
        if self.h_watchlist() > 0.0 {
            out.push((Zone::Watchlist, Rect::new(x, self.haut(), w, self.h_watchlist())));
        }
        let champ = Rect::new(x, self.haut_champ(), w, self.h_barre());
        out.push((Zone::Champ, champ));
        // tokens inside the field: the open market, then the blue ones (their ✕ zone: the whole token)
        let mut tx = champ.x + 36.0;
        for (k, t) in self.jetons_affiches().iter().enumerate() {
            let tw = text.width(t, 13.0) as f64 + 40.0;
            out.push((Zone::Jeton(k), Rect::new(tx, champ.cy() - (H_BARRE - 8.0) / 2.0, tw, H_BARRE - 8.0)));
            tx += tw + 6.0;
        }
        let mut y = champ.y + self.h_barre() + 10.0;
        let ong = self.onglets();
        let ow = w / ong.len().max(1) as f64;
        for (k, o) in ong.iter().enumerate() {
            out.push((Zone::Onglet(*o), Rect::new(x + k as f64 * ow, y, ow, self.h_barre())));
        }
        y += self.h_barre() + 8.0;
        let fixe_bas = r.y + r.h - PAD - self.h_barre();
        let mut cy = y - self.defile;
        let mut k = 0;
        for (_, l) in self.sections() {
            cy += H_TITRE;
            for _ in l {
                out.push((Zone::Ligne(k), Rect::new(x, cy, w, self.h_ligne())));
                cy += self.h_ligne();
                k += 1;
            }
        }
        if self.filtres {
            cy += 8.0;
            for f in Filtre::TOUS {
                let v = self.chips(f);
                if v.is_empty() {
                    continue;
                }
                cy += H_TITRE;
                let mut px = x;
                for c in v {
                    let cw = text.width(&c, 13.0) as f64 + 24.0;
                    let cw = if self.doigt { cw.max(self.h_puce()) } else { cw };
                    if px + cw > x + w {
                        (px, cy) = (x, cy + if self.doigt { self.h_puce() + 1.0 } else { H_PUCE + 12.0 });
                    }
                    out.push((Zone::Puce(f, c), Rect::new(px, cy, cw, self.h_puce())));
                    px += cw + 8.0;
                }
                cy += self.h_puce() + 4.0;
            }
        }
        let contenu = cy + self.defile - y;
        out.push((Zone::Filtres, Rect::new(x, fixe_bas, (w - self.place_ai).max(120.0), self.h_barre())));
        (out, contenu)
    }

    /// The tokens written in the field: the open market, then the filters.
    fn jetons_affiches(&self) -> Vec<String> {
        self.ouvert.iter().cloned().chain(self.jetons.iter().map(|(_, v)| v.clone())).collect()
    }

    /// Height of the scrolling list (between the tabs and « Tous les filtres »).
    fn liste(&self) -> (f64, f64) {
        let r = self.rect;
        let haut = self.haut_champ() + self.h_barre() + 10.0 + self.h_barre() + 8.0;
        (haut, r.y + r.h - PAD - self.h_barre() - 6.0)
    }

    fn zone(&self, x: f64, y: f64) -> Option<Zone> {
        let (haut, bas) = self.liste();
        let (zones, _) = self.zones(&Text::default());
        // (the tokens are inside the field: they win)
        zones.into_iter().rev().find_map(|(z, q)| {
            let defile = matches!(z, Zone::Ligne(_) | Zone::Puce(..));
            (q.contains(x, y) && (!defile || (y >= haut && y <= bas))).then_some(z)
        })
    }

    /// The row of a market (window), for tests and the app.
    pub fn ligne_rect(&self, k: usize) -> Option<Rect> {
        self.zones(&Text::default()).0.into_iter().find(|(z, _)| *z == Zone::Ligne(k)).map(|(_, q)| q)
    }

    /// The touch / click targets laid out (window): field, tabs, rows, « Tous les filtres », chips.
    pub fn cibles(&self) -> Vec<Rect> {
        self.zones(&Text::default())
            .0
            .into_iter()
            .filter(|(z, _)| !matches!(z, Zone::Jeton(_)))
            .map(|(_, q)| q)
            .collect()
    }

    /// The chip `v` of kind `f` (window), once « Tous les filtres » is unfolded.
    pub fn puce_rect(&self, f: Filtre, v: &str) -> Option<Rect> {
        self.zones(&Text::default())
            .0
            .into_iter()
            .find(|(z, _)| matches!(z, Zone::Puce(g, c) if *g == f && c == v))
            .map(|(_, q)| q)
    }

    /// The tab `o` (window), if shown.
    pub fn onglet_rect(&self, o: Onglet) -> Option<Rect> {
        self.zones(&Text::default()).0.into_iter().find(|(z, _)| *z == Zone::Onglet(o)).map(|(_, q)| q)
    }

    /// 👆 « Ma watchlist › » (window): by the finger, and on PC in the panel opened by a click.
    pub fn watchlist_rect(&self) -> Option<Rect> {
        self.zones(&Text::default()).0.into_iter().find(|(z, _)| *z == Zone::Watchlist).map(|(_, q)| q)
    }

    /// 🔎 The search field « Chercher un marché » (window): a touch there opens the phone's keyboard.
    pub fn champ_rect(&self) -> Rect {
        self.zones(&Text::default())
            .0
            .into_iter()
            .find(|(z, _)| *z == Zone::Champ)
            .map_or(self.rect, |(_, q)| q)
    }

    /// « ⚙ Tous les filtres › » (window).
    pub fn filtres_rect(&self) -> Rect {
        self.zones(&Text::default())
            .0
            .into_iter()
            .find(|(z, _)| *z == Zone::Filtres)
            .map_or(self.rect, |(_, q)| q)
    }

    pub fn wheel(&mut self, dy: f64) {
        let (haut, bas) = self.liste();
        let (_, contenu) = self.zones(&Text::default());
        self.defile = (self.defile + dy).clamp(0.0, (contenu - (bas - haut)).max(0.0));
    }

    /// Click / touch (window).
    pub fn click(&mut self, x: f64, y: f64) -> MarcheClick {
        if !self.rect.contains(x, y) {
            return MarcheClick::Fermer;
        }
        match self.zone(x, y) {
            Some(Zone::Champ) => {
                self.focus = true;
                MarcheClick::None
            }
            Some(Zone::Jeton(k)) => {
                // the open market first, then the filters
                match (k, self.ouvert.is_some()) {
                    (0, true) => self.ouvert = None,
                    (k, o) => {
                        let i = k - usize::from(o);
                        if i < self.jetons.len() {
                            self.jetons.remove(i);
                        }
                    }
                }
                self.sel = None;
                MarcheClick::Chercher
            }
            Some(Zone::Onglet(o)) => {
                (self.onglet, self.sel, self.defile) = (o, None, 0.0);
                MarcheClick::None
            }
            Some(Zone::Ligne(k)) => self.lignes().get(k).map_or(MarcheClick::None, |l| MarcheClick::Ouvrir(l.id.clone())),
            Some(Zone::Watchlist) => MarcheClick::Watchlist,
            Some(Zone::Filtres) => {
                self.filtres = !self.filtres;
                MarcheClick::None
            }
            Some(Zone::Puce(f, v)) => {
                if !self.jetons.contains(&(f, v.clone())) {
                    self.jetons.push((f, v));
                }
                // (new results: from their top, not where the chips were scrolled to)
                (self.sel, self.defile) = (None, 0.0);
                MarcheClick::Chercher
            }
            None => MarcheClick::None,
        }
    }

    /// Text typed into the field (it takes the keyboard).
    pub fn taper(&mut self, s: &str) -> MarcheClick {
        self.focus = true;
        self.texte.extend(s.chars().filter(|c| !c.is_control()));
        self.sel = None;
        MarcheClick::Chercher
    }

    /// Backspace: a letter; on an empty field, the last token (the open market last).
    pub fn effacer(&mut self) -> MarcheClick {
        if self.texte.pop().is_none() && self.jetons.pop().is_none() {
            self.ouvert = None;
        }
        self.sel = None;
        MarcheClick::Chercher
    }

    /// ↓ (`+1`) / ↑ (`-1`): the chosen row.
    pub fn bouger(&mut self, d: i32) {
        let n = self.lignes().len();
        if n == 0 {
            self.sel = None;
            return;
        }
        let k = match self.sel {
            None if d > 0 => 0,
            None => n - 1,
            Some(k) => (k as i64 + d as i64).rem_euclid(n as i64) as usize,
        };
        self.sel = Some(k);
    }

    /// Enter: the chosen row (or the first) opens.
    pub fn entree(&self) -> MarcheClick {
        let l = self.lignes();
        l.get(self.sel.unwrap_or(0)).map_or(MarcheClick::None, |l| MarcheClick::Ouvrir(l.id.clone()))
    }

    /// Image of the panel and its place (glass: the app blurs the chart under it).
    pub fn draw(&self, text: &Text, mouse: Option<(f64, f64)>, logo: &dyn Fn(&str) -> Option<Pixmap>) -> Option<(Pixmap, Rect)> {
        use crate::jetons as j;
        let r = self.rect;
        let mut pm = crate::px::new(r.w.ceil() as u32, r.h.ceil() as u32)?;
        let loc = |q: Rect| Rect::new(q.x - r.x, q.y - r.y, q.w, q.h);
        let rayon = if self.feuille { 26.0 } else { j::R_MENU };
        let carte = Rect::new(0.5, 0.5, r.w - 1.0, r.h - 1.0 + if self.feuille { 30.0 } else { 0.0 });
        // (a little transparency, like the mock-up: 55 %)
        round_fill(&mut pm, carte, rayon, j::VERRE.0, 0.55);
        round_stroke(&mut pm, carte, rayon, j::sur_fond(j::LISERE), 1.0);
        let survol = |q: &Rect| mouse.is_some_and(|(x, y)| q.contains(x, y));
        let gras = Text::graisse(Graisse::Bold);
        let (haut, bas) = self.liste();
        let mut liste = crate::px::new(r.w.ceil() as u32, r.h.ceil() as u32)?;
        let (zones, _) = self.zones(text);
        let lignes = self.lignes();
        // section titles
        let mut cy = haut - self.defile;
        for (t, l) in self.sections() {
            text.draw(&mut liste, t, ((PAD + 4.0) as f32, (cy - r.y + 18.0) as f32), 10.5, j::DIM, Align::Left);
            cy += H_TITRE + l.len() as f64 * self.h_ligne();
        }
        if lignes.is_empty() {
            let m = if self.cherche() {
                "Aucun marché ne correspond."
            } else {
                "Tape un nom de marché."
            };
            text.draw(&mut liste, m, ((PAD + 4.0) as f32, (haut - r.y + 24.0) as f32), 13.0, j::DIM, Align::Left);
        }
        let tokens = self.jetons_affiches();
        for (z, q) in &zones {
            let lq = loc(*q);
            // 👆 by the finger the touch zone is bigger than what is drawn (44 points, in its middle)
            let lq = if matches!(z, Zone::Champ | Zone::Onglet(_) | Zone::Filtres | Zone::Puce(..)) && lq.h > H_BARRE {
                Rect::new(lq.x, lq.cy() - H_BARRE / 2.0, lq.w, H_BARRE)
            } else {
                lq
            };
            match z {
                Zone::Watchlist => {
                    // the watchlist's icon of the app (☆, Phosphor « star »), its name, ›
                    round_fill(&mut pm, lq, j::R_LIGNE, j::BLANC, if survol(q) { 0.09 } else { 0.05 });
                    crate::phosphor::draw(&mut pm, "star", lq.x + 22.0, lq.cy(), 18, j::TEXTE);
                    gras.draw(
                        &mut pm,
                        "Ma watchlist",
                        ((lq.x + 44.0) as f32, (lq.cy() + 5.0) as f32),
                        15.0,
                        j::TEXTE,
                        Align::Left,
                    );
                    text.draw(&mut pm, "›", ((lq.x + lq.w - 14.0) as f32, (lq.cy() + 6.0) as f32), 18.0, j::DIM, Align::Right);
                }
                Zone::Champ => {
                    round_fill(&mut pm, lq, lq.h / 2.0, j::CHAMP.0, j::CHAMP.1);
                    round_stroke(&mut pm, lq, lq.h / 2.0, if self.focus { j::ACCENT } else { j::sur_fond(j::LISERE) }, 1.0);
                    text.draw(&mut pm, "🔎", ((lq.x + 18.0) as f32, (lq.cy() + 5.0) as f32), 14.0, j::DIM, Align::Center);
                    let tx = zones
                        .iter()
                        .filter(|(z, _)| matches!(z, Zone::Jeton(_)))
                        .map(|(_, t)| t.x + t.w + 6.0 - r.x)
                        .fold(lq.x + 36.0, f64::max);
                    let (s, col) = if self.texte.is_empty() && !self.focus {
                        ("Chercher un marché".to_string(), j::DIM)
                    } else {
                        (format!("{}{}", self.texte, if self.focus { "|" } else { "" }), j::TEXTE)
                    };
                    let s = text.fit(&s, 14.0, (lq.x + lq.w - tx - 10.0).max(20.0));
                    text.draw(&mut pm, &s, (tx as f32, (lq.cy() + 5.0) as f32), 14.0, col, Align::Left);
                }
                Zone::Jeton(k) => {
                    let marche = *k == 0 && self.ouvert.is_some();
                    let (c, a) = if marche { j::VERRE2 } else { j::PUCE_ON };
                    round_fill(&mut pm, lq, lq.h / 2.0, c, a + 0.05);
                    let t = tokens.get(*k).map_or("", String::as_str);
                    text.draw(&mut pm, t, ((lq.x + 12.0) as f32, (lq.cy() + 4.5) as f32), 13.0, j::TEXTE, Align::Left);
                    text.draw(&mut pm, "✕", ((lq.x + lq.w - 14.0) as f32, (lq.cy() + 4.5) as f32), 11.0, j::DIM, Align::Center);
                }
                Zone::Onglet(o) => {
                    crate::shapes::pilule(&mut pm, lq, self.onglet == *o, survol(q));
                    text.draw(&mut pm, o.nom(), (lq.cx() as f32, (lq.cy() + 5.0) as f32), 14.0, j::TEXTE, Align::Center);
                }
                Zone::Ligne(k) => {
                    let Some(l) = lignes.get(*k) else { continue };
                    if self.sel == Some(*k) || survol(q) {
                        round_fill(&mut liste, lq, j::R_LIGNE, j::BLANC, 0.07);
                    }
                    let lg = logo(&l.base);
                    crate::watchlist::coin_badge(&mut liste, text, &l.base, lg.as_ref(), lq.x + 22.0, lq.cy(), 15.0);
                    // ₿ a market against Bitcoin: a small orange ₿ at the bottom right of the coin
                    if l.quote.eq_ignore_ascii_case("BTC") {
                        let (bx, by) = (lq.x + 31.0, lq.cy() + 9.0);
                        crate::shapes::round_fill(&mut liste, Rect::new(bx - 7.0, by - 7.0, 14.0, 14.0), 7.0, BTC_ORANGE, 1.0);
                        gras.draw(&mut liste, "₿", (bx as f32, (by + 3.5) as f32), 9.5, j::TEXTE, Align::Center);
                    }
                    let by = (lq.cy() - 2.0) as f32;
                    gras.draw(&mut liste, &l.base, ((lq.x + 46.0) as f32, by), 15.0, j::TEXTE, Align::Left);
                    let bw = gras.width(&l.base, 15.0);
                    text.draw(&mut liste, &l.quote, ((lq.x + 46.0) as f32 + bw, by), 15.0, j::DIM, Align::Left);
                    // 🧮 the computed pair: a small « calculée » tag, and what it is made of
                    if l.calculee {
                        let tx = lq.x + 46.0 + (bw + text.width(&l.quote, 15.0)) as f64 + 8.0;
                        let tw = text.width("calculée", 11.5) as f64 + 16.0;
                        let tag = Rect::new(tx, lq.cy() - 15.0, tw, 18.0);
                        round_stroke(&mut liste, tag, 9.0, j::sur_fond(j::LISERE), 1.0);
                        text.draw(&mut liste, "calculée", (tag.cx() as f32, (tag.cy() + 4.0) as f32), 11.5, j::DIM, Align::Center);
                    }
                    let d = if l.calculee {
                        format!("{} ÷ {} (prix des perps)", l.base, l.quote)
                    } else {
                        format!("{} · {}", l.bourse, if l.perp { "Perp" } else { "Spot" })
                    };
                    text.draw(&mut liste, &d, ((lq.x + 46.0) as f32, by + 18.0), 12.0, j::DIM, Align::Left);
                    if let Some(v) = l.variation {
                        let col = if v >= 0.0 { crate::palette::up() } else { crate::palette::down() };
                        let s = format!("{v:+.2} %").replace('.', ",");
                        text.draw(&mut liste, &s, ((lq.x + lq.w - 8.0) as f32, (lq.cy() + 5.0) as f32), 13.5, col, Align::Right);
                    }
                }
                Zone::Filtres => {
                    round_fill(&mut pm, lq, j::R_LIGNE, j::BLANC, if survol(q) { 0.09 } else { 0.05 });
                    text.draw(
                        &mut pm,
                        "⚙ Tous les filtres",
                        ((lq.x + 14.0) as f32, (lq.cy() + 5.0) as f32),
                        14.0,
                        j::TEXTE,
                        Align::Left,
                    );
                    let fl = if self.filtres { "⌄" } else { "›" };
                    text.draw(&mut pm, fl, ((lq.x + lq.w - 14.0) as f32, (lq.cy() + 5.0) as f32), 14.0, j::DIM, Align::Right);
                }
                Zone::Puce(f, v) => {
                    let on = self.jetons.contains(&(*f, v.clone()));
                    crate::shapes::pilule(&mut liste, lq, on, survol(q));
                    text.draw(&mut liste, v, (lq.cx() as f32, (lq.cy() + 4.5) as f32), 13.0, j::TEXTE, Align::Center);
                }
            }
        }
        // the chip groups' titles (above the first chip of each kind)
        for f in Filtre::TOUS {
            if let Some((_, q)) = zones.iter().find(|(z, _)| matches!(z, Zone::Puce(g, _) if *g == f)) {
                text.draw(&mut liste, f.titre(), ((PAD + 4.0) as f32, (q.y - r.y - 8.0) as f32), 10.5, j::DIM, Align::Left);
            }
        }
        // the list, clipped between the tabs and « Tous les filtres »
        if let Some(part) = crate::px::int_rect(0.0, haut - r.y, r.w, (bas - haut).max(1.0)).and_then(|q| liste.clone_rect(q)) {
            crate::px::put(&mut pm, 0.0, haut - r.y, &part);
        }
        Some((pm, r))
    }
}

/// 🖱 PC: the hover timer of the symbol pill. `sur_pastille` / `sur_panneau`: where the mouse is; `ouvert`: the
/// panel is open; `focus`: its field has the keyboard; `t`: now (seconds). Returns `Some(true)` to open, `Some(false)`
/// to close, `None`: nothing yet.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Survol {
    /// Since when the mouse is on the pill (closed panel) / away from pill and panel (open panel).
    depuis: Option<f64>,
    /// Closed by a click on the pill: no reopening by hover until the mouse has left the pill.
    retenu: bool,
}

impl Survol {
    pub fn pas(&mut self, t: f64, sur_pastille: bool, sur_panneau: bool, ouvert: bool, focus: bool) -> Option<bool> {
        if !ouvert && self.retenu {
            self.retenu = sur_pastille;
            self.depuis = None;
            return None;
        }
        let attend = if ouvert { !(sur_pastille || sur_panneau) && !focus } else { sur_pastille };
        if !attend {
            self.depuis = None;
            return None;
        }
        let d = *self.depuis.get_or_insert(t);
        if t - d >= DELAI {
            self.depuis = None;
            return Some(!ouvert);
        }
        None
    }

    /// The panel was just closed by a click on the pill (the mouse still on it): don't reopen it by hover.
    pub fn retenir(&mut self) {
        (self.retenu, self.depuis) = (true, None);
    }

    /// Waiting (the app wakes up again soon).
    pub fn en_cours(&self) -> bool {
        self.depuis.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn l(base: &str, quote: &str, bourse: &str, perp: bool, groupe: &str) -> Ligne {
        Ligne {
            id: format!("{bourse}:{base}{quote}{}", if perp { ".P" } else { "" }),
            base: base.into(),
            quote: quote.into(),
            bourse: bourse.into(),
            perp,
            groupe: groupe.into(),
            variation: Some(1.0),
            calculee: false,
        }
    }

    /// ₿ « xrpbtc », « xrp/btc », « xrp btc »: the real XRP pairs against BTC, then CALCULÉE; the « btc » chip: only
    /// markets against BTC, « CONTRE BTC »; « btc » typed alone still searches Bitcoin's markets.
    #[test]
    fn paires_contre_btc_et_puce_btc() {
        let mut p = panneau();
        let mut calc = l("XRP", "BTC", "Binance", true, "Crypto");
        (calc.id, calc.calculee) = ("ratio:binance:XRP:BTC".into(), true);
        p.resultats = vec![
            l("XRP", "BTC", "Bitfinex", false, "Crypto"),
            l("XRP", "BTC", "Binance", false, "Crypto"),
            l("XRP", "USDT", "Binance", false, "Crypto"),
            calc,
        ];
        let noms = |p: &Panneau| {
            p.sections()
                .into_iter()
                .map(|(t, l)| (t, l.iter().map(|x| format!("{}{} {}", x.base, x.quote, x.bourse)).collect::<Vec<_>>()))
                .collect::<Vec<_>>()
        };
        for q in ["xrpbtc", "xrp/btc", "xrp-btc", "xrp btc", "XRPBTC"] {
            p.texte = q.into();
            assert_eq!(
                noms(&p),
                [
                    ("RÉSULTATS", vec!["XRPBTC Bitfinex".to_string(), "XRPBTC Binance".into()]),
                    ("CALCULÉE", vec!["XRPBTC Binance".into()])
                ],
                "{q}"
            );
        }
        // the « btc » chip (« ⚙ Tous les filtres »): only the markets against BTC
        p.texte.clear();
        p.jetons = vec![(Filtre::Quote, "btc".into())];
        let s = noms(&p);
        assert_eq!(s[0].0, "CONTRE BTC");
        assert!(s.iter().flat_map(|x| &x.1).all(|n| n.contains("XRPBTC")));
        assert!(p.chips(Filtre::Quote).contains(&"btc".to_string()));
        // « btc » typed alone: a word, not the chip (Bitcoin's markets, as before)
        p.jetons.clear();
        p.texte = "btc".into();
        assert_eq!(p.requete(), "btc");
    }

    fn panneau() -> Panneau {
        let mut p = Panneau::sous(Rect::new(20.0, 120.0, 300.0, 34.0), 1440.0, 900.0, false);
        p.bourses = vec!["Binance".into(), "Hyperliquid".into()];
        p.classes = vec!["crypto".into(), "memecoins".into()];
        p.recents = vec![l("ZEC", "USDC", "Hyperliquid", true, "crypto")];
        p.favoris = vec![l("BTC", "USDC", "Hyperliquid", true, "crypto"), l("ETH", "USDC", "Hyperliquid", true, "crypto")];
        p.resultats = vec![
            l("ETH", "USDT", "Binance", true, "crypto"),
            l("ETH", "USDT", "Binance", false, "crypto"),
            l("ETH", "USDC", "Hyperliquid", true, "crypto"),
            l("PEPE", "USDT", "Binance", false, "memecoins"),
        ];
        p
    }

    #[test]
    fn recents_puis_favoris_et_onglet_vide_cache() {
        let p = panneau();
        let s = p.sections();
        assert_eq!(s.iter().map(|(t, _)| *t).collect::<Vec<_>>(), ["RÉCENTS", "FAVORIS"]);
        assert_eq!(p.lignes().len(), 3);
        // no stock, no forex: those tabs are hidden
        assert_eq!(p.onglets(), [Onglet::Tout, Onglet::Crypto]);
        assert!(p.onglet_rect(Onglet::Actions).is_none());
        let mut q = p.clone();
        q.resultats.push(l("NVDA", "USDC", "Hyperliquid", true, "actions"));
        assert_eq!(q.onglets(), [Onglet::Tout, Onglet::Crypto, Onglet::Actions]);
        let mut q = p.clone();
        q.taper("eth");
        assert!(p.draw(&Text::default(), None, &|_| None).is_some());
        assert!(q.draw(&Text::default(), None, &|_| None).is_some());
    }

    /// Never empty (Jee, 21 Nov): no favourite → POPULAIRES (when the app has some), else the hint text.
    #[test]
    fn populaires_sans_favoris() {
        let mut p = panneau();
        p.populaires = vec![l("SOL", "USDC", "Hyperliquid", true, "crypto")];
        let titres = |p: &Panneau| p.sections().iter().map(|(t, _)| *t).collect::<Vec<_>>();
        assert_eq!(titres(&p), ["RÉCENTS", "FAVORIS"], "favourites first: no POPULAIRES");
        p.favoris.clear();
        assert_eq!(titres(&p), ["RÉCENTS", "POPULAIRES"]);
        assert_eq!(p.lignes()[1].base, "SOL");
        (p.recents, p.populaires) = (vec![], vec![]);
        assert!(p.sections().is_empty(), "nothing: « Tape un nom de marché. »");
    }

    #[test]
    fn mots_dans_n_importe_quel_ordre() {
        let mut a = panneau();
        a.taper("eth binance perp");
        let mut b = panneau();
        b.taper("perp eth binance");
        let ids = |p: &Panneau| p.lignes().iter().map(|l| l.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(&a), ["Binance:ETHUSDT.P"]);
        assert_eq!(ids(&a), ids(&b));
        assert_eq!(a.requete(), "eth", "only the free words go to the market engine");
        assert_eq!(a.sections()[0].0, "RÉSULTATS");
    }

    #[test]
    fn jetons_par_les_puces_et_le_retour_arriere() {
        let mut p = panneau();
        p.ouvert = Some("BTC".into());
        assert_eq!(p.click(p.filtres_rect().cx(), p.filtres_rect().cy()), MarcheClick::None);
        assert!(p.filtres);
        let c = p.puce_rect(Filtre::Type, "spot").expect("puce spot");
        assert_eq!(p.click(c.cx(), c.cy()), MarcheClick::Chercher);
        assert_eq!(p.jetons, [(Filtre::Type, "spot".to_string())]);
        let c = p.puce_rect(Filtre::Bourse, "binance").expect("puce binance");
        p.click(c.cx(), c.cy());
        assert_eq!(p.jetons.len(), 2);
        assert!(p.lignes().iter().all(|l| !l.perp && l.bourse == "Binance"));
        // ✕ of the 2nd filter token (after the open market): removed
        let (zones, _) = p.zones(&Text::default());
        let t = zones.iter().find(|(z, _)| *z == Zone::Jeton(2)).unwrap().1;
        p.click(t.cx(), t.cy());
        assert_eq!(p.jetons, [(Filtre::Type, "spot".to_string())]);
        // Backspace on an empty field: the last token, then the open market
        p.effacer();
        assert!(p.jetons.is_empty() && p.ouvert.is_some());
        p.effacer();
        assert!(p.ouvert.is_none());
        // typing « binance » filters like the chip
        p.taper("binance");
        assert!(p.lignes().iter().all(|l| l.bourse == "Binance"));
    }

    #[test]
    fn clic_sur_une_ligne_et_clavier() {
        let mut p = panneau();
        let r = p.ligne_rect(1).unwrap();
        assert_eq!(p.click(r.cx(), r.cy()), MarcheClick::Ouvrir("Hyperliquid:BTCUSDC.P".into()));
        assert_eq!(p.click(5.0, 5.0), MarcheClick::Fermer, "dehors : fermé");
        p.bouger(1);
        p.bouger(1);
        assert_eq!(p.entree(), MarcheClick::Ouvrir("Hyperliquid:BTCUSDC.P".into()));
        p.bouger(-1);
        p.bouger(-1);
        assert_eq!(p.sel, Some(2), "↑ wraps");
        let c = p.rect;
        p.click(c.x + 40.0, c.y + 30.0);
        assert!(p.focus, "le champ prend le clavier");
    }

    #[test]
    fn au_doigt_cibles_de_55_et_feuille() {
        for mut p in [
            Panneau::feuille(402.0, 874.0),
            Panneau::sous(Rect::new(20.0, 60.0, 200.0, 44.0), 820.0, 1180.0, true),
        ] {
            let q = panneau();
            (p.recents, p.favoris, p.bourses, p.classes, p.filtres) = (q.recents, q.favoris, q.bourses, q.classes, true);
            let t = crate::formfactor::TOUCH_MIN_PT;
            let fin = p.filtres_rect();
            // (what scrolls below « Tous les filtres » is hidden under it: not a target there)
            let vues: Vec<Rect> = p.cibles().into_iter().filter(|c| *c == fin || c.y + c.h <= fin.y).collect();
            assert!(vues.len() >= 8, "{vues:?}");
            for (k, a) in vues.iter().enumerate() {
                assert!(a.h >= t && a.w >= t, "{a:?}");
                for b in &vues[k + 1..] {
                    assert!(!(a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h), "{a:?} {b:?}");
                }
            }
            assert!(p.ligne_rect(0).unwrap().h >= 56.0);
            // « Ma watchlist › » at the top, above the field; touching it asks for the watchlist
            let wr = p.watchlist_rect().expect("Ma watchlist by the finger");
            assert!(wr.h >= t && wr.y + wr.h <= p.cibles()[1].y, "{wr:?}");
            assert_eq!(p.clone().click(wr.cx(), wr.cy()), MarcheClick::Watchlist);
            assert!(p.draw(&Text::default(), None, &|_| None).is_some());
        }
        let f = Panneau::feuille(402.0, 874.0);
        assert!(f.feuille && f.rect.y + f.rect.h == 874.0 && f.rect.w == 402.0);
        // PC: unchanged (field, tabs and « Tous les filtres » of 44, chips of 32)
        let mut pc = panneau();
        pc.filtres = true;
        assert!(pc.watchlist_rect().is_none(), "PC: no « Ma watchlist » row");
        // PC, opened by a click (pinned): the row, under it the field
        pc.epingle = true;
        let wr = pc.watchlist_rect().expect("pinned: « Ma watchlist › »");
        assert!(wr.y + wr.h < pc.puce_rect(Filtre::Type, "perp").unwrap().y);
        assert_eq!(pc.clone().click(wr.cx(), wr.cy()), MarcheClick::Watchlist);
        pc.epingle = false;
        assert_eq!(pc.filtres_rect().h, 44.0);
        assert_eq!(pc.onglet_rect(Onglet::Tout).unwrap().h, 44.0);
        assert_eq!(pc.puce_rect(Filtre::Type, "perp").unwrap().h, 32.0);
    }

    #[test]
    fn survol_0_3_s_ouvre_sortie_0_3_s_ferme_champ_actif_reste() {
        let mut s = Survol::default();
        assert_eq!(s.pas(0.0, true, false, false, false), None);
        assert_eq!(s.pas(0.2, true, false, false, false), None);
        assert_eq!(s.pas(0.31, true, false, false, false), Some(true), "0,3 s sur la pastille : ouvert");
        // on the panel: stays
        assert_eq!(s.pas(1.0, false, true, true, false), None);
        assert_eq!(s.pas(2.0, false, true, true, false), None);
        // away from both: closes 0.3 s later
        assert_eq!(s.pas(3.0, false, false, true, false), None);
        assert_eq!(s.pas(3.31, false, false, true, false), Some(false));
        // the field has the keyboard: stays open
        assert_eq!(s.pas(4.0, false, false, true, true), None);
        assert_eq!(s.pas(9.0, false, false, true, true), None);
        // a short pass over the pill does not open
        let mut s = Survol::default();
        s.pas(0.0, true, false, false, false);
        assert_eq!(s.pas(0.1, false, false, false, false), None);
        assert_eq!(s.pas(0.5, true, false, false, false), None);
    }
}
