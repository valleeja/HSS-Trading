//! ✨ Argos AI, **comme la maquette verre (pc-36)** : une carte en verre à droite, en-tête (Argos AI, 🧠, 🔑, ✕),
//! 📍 l'endroit ouvert, raccourcis en puces (Analyse 4H, Setup, Mes stats, News, Macro semaine, Quoi éviter
//! aujourd'hui), la conversation, puis la barre fixée en bas : 🧭 Analyse guidée, « Demande à Argos AI… » avec ➤, et
//! « Ce n'est pas un conseil financier. ». Sans clé : « 🔑 Ajouter ma clé » ; clé verrouillée : « 🔓 Déverrouiller ».
//!
//! Une seule table de zones (`zones`) sert au dessin et au clic.

use crate::jetons as j;
use crate::palette::{accent, DIM, GOLD, WHITE};
use crate::shapes::{lines, round_fill, round_stroke, Rect};
use crate::text::{Align, Graisse, Rgb, Text};
use tiny_skia::Pixmap;

/// Raccourcis de la maquette (pc-36) : (bouton, demande envoyée) ; `{iv}` = l'unité de temps ouverte, `{s}` = le
/// symbole et l'unité de temps. La méthode SMC : niveaux tirés du graphique, jamais inventés.
pub const CHIPS: [(&str, &str); 3] = [
    (
        "Lis la structure {iv} (BOS, CHoCH)",
        "Lis la structure de {s} (BOS, CHoCH), de la grande unité de temps vers la petite. Dis d'où vient chaque niveau ; n'en invente aucun.",
    ),
    (
        "Repère les zones : OB, FVG, liquidité",
        "Repère les zones de {s} : Order Blocks et FVG (déjà mitigés ou non), liquidité (BSL, SSL, sweeps). Dis d'où vient chaque niveau ; n'en invente aucun.",
    ),
    (
        "Quelle news aujourd'hui pour ce symbole ?",
        "Quelles news importantes aujourd'hui pour {s} ? Heures en heure de New York, impact possible, et cite tes sources.",
    ),
];

/// État de la clé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeyState {
    /// Aucune clé sur cet appareil.
    #[default]
    None,
    /// Clé gardée (chiffrée) : il faut le code.
    Locked,
    /// Clé ouverte pour la session.
    Ready,
}

/// Résultat d'un clic.
#[derive(Debug, Clone, PartialEq)]
pub enum AsClick {
    None,
    Close,
    /// 🔑 : ajouter / changer la clé.
    Key,
    /// 🔓 Déverrouiller.
    Unlock,
    /// Oublier la clé de cet appareil.
    Forget,
    /// Envoyer cette demande.
    Send(String),
    /// Le champ est choisi (clavier).
    Focus,
    /// 🧠 Menu de la mémoire (notes, méthode, Sandbox).
    Memory,
    /// 🧠 Note proposée : retenue / refusée.
    NoteYes,
    NoteNo,
    /// 🤖 « Effacer l'explication »: its drawings go from the chart.
    EffacerExplication,
    /// 🤖 « ✨ Lire tous les indicateurs » (idea 1): the indicators on the chart read together.
    LireTout,
    /// 🤖 « ✨ Accord des unités de temps » (idea 2): the chart's unit and the higher one side by side.
    Accord,
    /// 🤖 « ✨ Volatilité du moment » (idea 4): how much the market moves now.
    Volatilite,
    /// 🤖 « ✨ Niveaux du jour » (idea 5): yesterday's high and low and the day's VWAP.
    NiveauxJour,
    /// 🤖 « ⚙ Tous les réglages »: the ⚙ of the indicator whose reading is shown.
    TousReglages,
    /// 🤖 « ＋ N autres réglages utiles »: the reading again, with all the settings its rules read.
    AutresReglages,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Zone {
    Effacer,
    TousReglages,
    AutresReglages,
    Close,
    Memory,
    NoteYes,
    NoteNo,
    Key,
    Unlock,
    Forget,
    Chip(usize),
    /// 🤖 « ✨ Lire tous les indicateurs » (« règles » mode).
    LireTout,
    /// 🤖 « ✨ Accord des unités de temps » (« règles » mode).
    Accord,
    /// 🤖 « ✨ Volatilité du moment » (« règles » mode).
    Volatilite,
    /// 🤖 « ✨ Niveaux du jour » (« règles » mode).
    NiveauxJour,
    /// 🧭 Analyse guidée (barre du bas).
    Guide,
    Field,
    Send,
}

/// L'assistant (la conversation reste quand on le ferme).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Assistant {
    /// (vient de moi ?, texte).
    pub msgs: Vec<(bool, String)>,
    pub input: String,
    /// Le champ reçoit le clavier.
    pub typing: bool,
    /// Une réponse est en route.
    pub busy: bool,
    pub key: KeyState,
    /// Défilement de la conversation (0 = en bas, les derniers messages).
    pub scroll: f64,
    /// 🤖 « règles » mode (Argos AI level 1, no network, no Claude key): its tag beside the title.
    pub regles: bool,
    /// 🤖 An explanation is drawn on the chart: the « Effacer l'explication » button.
    pub effacer: bool,
    /// 🤖 The indicator whose rule-based reading shows only the settings it reads (« Ok reglage court », Jee 18 Nov):
    /// the « ⚙ Tous les réglages » button opens its ⚙.
    pub tous_reglages: Option<crate::indicators::Ind>,
    /// 🤖 How many more settings the rules read than the reading shows (0: no « ＋ N autres réglages utiles »).
    pub autres_reglages: usize,
    /// 🧪 Mode Sandbox (rien n'est retenu) ; sinon 🧠 Mémoire.
    pub sandbox: bool,
    /// 🧠 Note proposée, en attente de ton accord (rien n'est retenu sans lui).
    pub pending_note: Option<String>,
    /// 📍 L'endroit ouvert (« Tu es sur la chart : BTC · 4h ») ; vide : pas de puce.
    pub lieu: String,
    /// ⏱ L'unité de temps ouverte (« 4h »), dans le 1er raccourci.
    pub iv: String,
    /// 🤖 The higher unit of the timeframe agreement (« 1h »); `None`: there is none (1 month), the line is greyed.
    pub unite_sup: Option<String>,
    /// The highest `scroll` of the last drawing (the first message's top at the top of the area): the wheel stops
    /// there instead of scrolling into the empty space above (Jee 18 Nov, long readings).
    pub scroll_max: std::cell::Cell<f64>,
}

/// 🤖 The « règles » mode's tag and its button.
const REGLES: &str = "règles";
const EFFACER: &str = "🧹 Effacer l'explication";
pub const TOUS_REGLAGES: &str = "⚙ Tous les réglages";
/// 🤖 The three lines at the top of the « règles » mode (ideas 1, 2 and 4), 40 points high for the finger.
pub const LIRE_TOUT: &str = "✨ Lire tous les indicateurs";
pub const ACCORD: &str = "✨ Accord des unités de temps";
pub const VOLATILITE: &str = "✨ Volatilité du moment";
pub const NIVEAUX_JOUR: &str = "✨ Niveaux du jour";
const LIGNE_H: f64 = 40.0;

/// Hauteur de la carte « Retenir ? ».
const NOTE_H: f64 = 74.0;

/// Marge autour de la carte en verre (le volet flotte, coins arrondis) et intérieur de la carte.
const M: f64 = 8.0;
const PAD: f64 = 14.0;
const HEAD: f64 = 52.0;
const LIEU_H: f64 = 34.0;
const CHIP_H: f64 = 30.0;
const CHAMP_H: f64 = 42.0;
/// Barre fixée en bas (maquette pc-36) : 🧭 Analyse guidée, le champ, « Ce n'est pas un conseil financier. »
const FOOT: f64 = CHIP_H + 8.0 + CHAMP_H + 26.0 + M;
const T: f32 = 13.0;

/// 🧭 Analyse guidée : la méthode SMC du haut vers le bas, sans niveau inventé.
pub const GUIDE: &str = "Fais l'analyse guidée SMC de {s}, de la grande unité de temps vers la petite : structure (BOS, CHoCH), premium / discount, liquidité (BSL, SSL, sweeps), Order Blocks et FVG (déjà mitigés ou non), confluence, puis un plan (entrée, stop, cibles, R:R, invalidation) ou « pas de trade ». N'invente aucun niveau : dis d'où vient chacun. Termine par « Ce n'est pas un conseil financier. »";

/// 📐 Où vont les choses dans le volet.
struct Disposition {
    zones: Vec<(Rect, Zone)>,
    /// Haut de la conversation.
    haut: f64,
    /// Haut de l'aide de la carte de la clé.
    aide_y: f64,
    /// Haut de la barre du bas (sa rangée la plus haute).
    pied_y: f64,
}

impl Assistant {
    /// La carte en verre (coordonnées de l'image `w` × `h`).
    fn carte(w: f64, h: f64) -> Rect {
        Rect::new(M, M, (w - 2.0 * M).max(1.0), (h - 2.0 * M).max(1.0))
    }

    /// Le texte du raccourci `k` (l'unité de temps ouverte à la place de `{iv}`).
    fn puce(&self, k: usize) -> String {
        CHIPS[k].0.replace("{iv}", if self.iv.is_empty() { "" } else { &self.iv }).replace("  ", " ")
    }

    /// Le champ « Demande à Argos AI… » (coordonnées de l'image `w` × `h`) : 📱 y toucher ouvre le clavier.
    pub fn champ(w: f64, h: f64) -> Rect {
        let c = Self::carte(w, h);
        Rect::new(c.x + PAD, c.y + c.h - FOOT + M + CHIP_H + 8.0, c.w - 2.0 * PAD, CHAMP_H)
    }

    /// Zones (coordonnées de l'image `w` × `h`) et le haut de la conversation.
    fn zones(&self, text: &Text, w: f64, h: f64) -> (Vec<(Rect, Zone)>, f64) {
        let d = self.disposition(text, w, h);
        (d.zones, d.haut)
    }

    /// Zones, haut de la conversation, haut de l'aide de la clé et haut de la barre du bas.
    fn disposition(&self, text: &Text, w: f64, h: f64) -> Disposition {
        let c = Self::carte(w, h);
        let (x0, x1) = (c.x + PAD, c.x + c.w - PAD);
        let mut z = vec![
            (Rect::new(x1 - 30.0, c.y + 11.0, 30.0, 30.0), Zone::Close),
            (Rect::new(x1 - 66.0, c.y + 11.0, 30.0, 30.0), Zone::Key),
            (Rect::new(x1 - 102.0, c.y + 11.0, 30.0, 30.0), Zone::Memory),
        ];
        let mut y = c.y + HEAD;
        if !self.lieu.is_empty() {
            y += LIEU_H + 10.0;
        }
        // 🤖 « règles » mode: read all the indicators, and the timeframe agreement (a line each, the whole width)
        if self.regles {
            for zone in [Zone::LireTout, Zone::Accord, Zone::Volatilite, Zone::NiveauxJour] {
                z.push((Rect::new(x0, y, x1 - x0, LIGNE_H), zone));
                y += LIGNE_H + 6.0;
            }
            y += 4.0;
        }
        // raccourcis en puces, une par ligne (maquette pc-36) ; 🤖 en mode « règles » sans clé Claude, ils ne
        // servent pas (ils parlent à Claude) : la place va à la lecture (sur le cell, elle tenait sur une ligne)
        let puces = if self.regles && self.key != KeyState::Ready { 0 } else { CHIPS.len() };
        for k in 0..puces {
            let bw = text.width(&self.puce(k), 12.5) as f64 + 24.0;
            z.push((Rect::new(x0, y, bw.min(x1 - x0), CHIP_H), Zone::Chip(k)));
            y += CHIP_H + 6.0;
        }
        let carte_y = y + 6.0;
        // la barre fixée en bas : 🧭 Analyse guidée (elle parle à Claude : en mode « règles » sans clé, elle laisse
        // sa place aux boutons de la lecture), 🧹 Effacer l'explication, « + N autres réglages utiles », ⚙ Tous les
        // réglages
        let fy = c.y + c.h - FOOT + M;
        let mut pied: Vec<(f64, Zone)> = vec![];
        if !(self.regles && self.key != KeyState::Ready) {
            pied.push((text.width("🧭 Analyse guidée", 12.5) as f64 + 24.0, Zone::Guide));
        }
        if self.effacer {
            pied.push((text.width(EFFACER, 12.5) as f64 + 24.0, Zone::Effacer));
        }
        if self.tous_reglages.is_some() && self.autres_reglages > 0 {
            pied.push((text.width(&self.libelle_autres(), 12.5) as f64 + 24.0, Zone::AutresReglages));
        }
        if self.tous_reglages.is_some() {
            pied.push((text.width(TOUS_REGLAGES, 12.5) as f64 + 24.0, Zone::TousReglages));
        }
        // 📱 boutons de gauche à droite ; ce qui ne tient pas passe sur une rangée de plus, au-dessus (jamais l'un
        // sur l'autre : sur le cell, « Ajouter ma clé » était dessiné sur « Effacer l'explication », Jee 20 nov.)
        let rangees = |items: &[(f64, Zone)]| -> (Vec<(Rect, Zone)>, f64) {
            let mut lignes: Vec<Vec<(f64, Zone)>> = vec![];
            let mut x = x0;
            for &(bw, zone) in items {
                let bw = bw.min(x1 - x0);
                if lignes.is_empty() || (x > x0 && x + bw > x1) {
                    lignes.push(vec![]);
                    x = x0;
                }
                if let Some(l) = lignes.last_mut() {
                    l.push((bw, zone));
                }
                x += bw + 8.0;
            }
            let n = lignes.len().max(1) as f64;
            let haut = fy - (n - 1.0) * (CHIP_H + 6.0);
            let mut out = vec![];
            for (k, l) in lignes.iter().enumerate() {
                let ry = haut + k as f64 * (CHIP_H + 6.0);
                let mut x = x0;
                for &(bw, zone) in l {
                    out.push((Rect::new(x, ry, bw, CHIP_H), zone));
                    x += bw + 8.0;
                }
            }
            (out, haut)
        };
        let (mut bas, mut pied_y) = rangees(&pied);
        // la carte de la clé : l'aide (2 lignes), ses boutons dessous
        let cle: Vec<(f64, Zone)> = match self.key {
            KeyState::None => vec![(180.0, Zone::Key)],
            KeyState::Locked => vec![(160.0, Zone::Unlock), (150.0, Zone::Forget)],
            KeyState::Ready => vec![],
        };
        let mut top = carte_y;
        if !cle.is_empty() {
            let large: f64 = cle.iter().map(|&(bw, _)| bw + 8.0).sum::<f64>() - 8.0;
            if carte_y + 88.0 <= pied_y && large <= x1 - x0 {
                let mut x = x0;
                for &(bw, zone) in &cle {
                    z.push((Rect::new(x, carte_y + 44.0, bw, 36.0), zone));
                    x += bw + 8.0;
                }
                top = carte_y + 96.0;
            } else {
                // 📱 pas la place sous l'aide (volet court du cell) : les boutons de la clé passent dans la barre du
                // bas, devant les autres
                let libelle = |zone: Zone| match zone {
                    Zone::Key => "🔑 Ajouter ma clé",
                    Zone::Unlock => "🔓 Déverrouiller",
                    _ => "Oublier la clé",
                };
                let tous: Vec<(f64, Zone)> = cle
                    .iter()
                    .map(|&(_, zone)| (text.width(libelle(zone), 13.5) as f64 + 28.0, zone))
                    .chain(pied.iter().copied())
                    .collect();
                (bas, pied_y) = rangees(&tous);
                top = carte_y + 40.0;
            }
        }
        // 📱 very short panel: a reading line or a shortcut that would reach the bottom bar is left out (never one
        // button on another)
        z.retain(|(r, zone)| {
            !matches!(zone, Zone::LireTout | Zone::Accord | Zone::Volatilite | Zone::NiveauxJour | Zone::Chip(_)) || r.y + r.h <= pied_y - 6.0
        });
        if self.pending_note.is_some() {
            let cy = pied_y - NOTE_H - 8.0;
            z.push((Rect::new(x1 - 184.0, cy + 36.0, 86.0, 30.0), Zone::NoteYes));
            z.push((Rect::new(x1 - 92.0, cy + 36.0, 92.0, 30.0), Zone::NoteNo));
        }
        z.extend(bas);
        let champ = Self::champ(w, h);
        z.push((champ, Zone::Field));
        z.push((Rect::new(champ.x + champ.w - 38.0, champ.y + 5.0, 32.0, 32.0), Zone::Send));
        Disposition {
            zones: z,
            haut: top,
            aide_y: carte_y,
            pied_y,
        }
    }

    /// 🤖 « ＋ N autres réglages utiles ».
    fn libelle_autres(&self) -> String {
        // (the full-width « ＋ » is not in the font: it was drawn as an empty box; the plain « + »)
        format!(
            "+ {} {}",
            self.autres_reglages,
            if self.autres_reglages > 1 {
                "autres réglages utiles"
            } else {
                "autre réglage utile"
            }
        )
    }

    /// 🤖 The « ＋ N autres réglages utiles » button (in the image `w` × `h`), when it is shown.
    pub fn bouton_autres_reglages(&self, text: &Text, w: f64, h: f64) -> Option<Rect> {
        self.zones(text, w, h).0.into_iter().find(|(_, z)| *z == Zone::AutresReglages).map(|(r, _)| r)
    }

    /// 🤖 The « ⚙ Tous les réglages » button (in the image `w` × `h`), when it is shown.
    pub fn bouton_tous_reglages(&self, text: &Text, w: f64, h: f64) -> Option<Rect> {
        self.zones(text, w, h).0.into_iter().find(|(_, z)| *z == Zone::TousReglages).map(|(r, _)| r)
    }

    /// 🤖 The four lines of the « règles » mode (« Lire tous les indicateurs », « Accord des unités de temps »,
    /// « Volatilité du moment », « Niveaux du jour »), in the image `w` × `h`; empty out of that mode.
    pub fn lignes_regles(&self, text: &Text, w: f64, h: f64) -> Vec<Rect> {
        self.zones(text, w, h)
            .0
            .into_iter()
            .filter(|(_, z)| matches!(z, Zone::LireTout | Zone::Accord | Zone::Volatilite | Zone::NiveauxJour))
            .map(|(r, _)| r)
            .collect()
    }

    /// Clic dans l'image `w` × `h` ; `symbol` remplace `{s}` dans les raccourcis.
    pub fn click(&mut self, text: &Text, w: f64, h: f64, x: f64, y: f64, symbol: &str) -> AsClick {
        let (z, _) = self.zones(text, w, h);
        let hit = z.iter().rev().find(|(r, _)| r.contains(x, y)).map(|(_, z)| *z);
        self.typing = hit == Some(Zone::Field);
        match hit {
            Some(Zone::Close) => AsClick::Close,
            Some(Zone::Memory) => AsClick::Memory,
            Some(Zone::NoteYes) => AsClick::NoteYes,
            Some(Zone::NoteNo) => AsClick::NoteNo,
            Some(Zone::Key) => AsClick::Key,
            Some(Zone::Unlock) => AsClick::Unlock,
            Some(Zone::Forget) => AsClick::Forget,
            Some(Zone::Chip(k)) => AsClick::Send(CHIPS[k].1.replace("{s}", symbol)),
            Some(Zone::Guide) => AsClick::Send(GUIDE.replace("{s}", symbol)),
            Some(Zone::Effacer) => AsClick::EffacerExplication,
            Some(Zone::TousReglages) => AsClick::TousReglages,
            Some(Zone::AutresReglages) => AsClick::AutresReglages,
            Some(Zone::LireTout) => AsClick::LireTout,
            Some(Zone::Accord) if self.unite_sup.is_some() => AsClick::Accord,
            Some(Zone::Accord) => AsClick::None,
            Some(Zone::Volatilite) => AsClick::Volatilite,
            Some(Zone::NiveauxJour) => AsClick::NiveauxJour,
            Some(Zone::Field) => AsClick::Focus,
            Some(Zone::Send) => self.take_input().map_or(AsClick::None, AsClick::Send),
            None => AsClick::None,
        }
    }

    /// Ce qui est dans le champ (vidé), s'il y a quelque chose.
    pub fn take_input(&mut self) -> Option<String> {
        let t = self.input.trim().to_string();
        self.input.clear();
        (!t.is_empty()).then_some(t)
    }

    /// Molette : la conversation défile (`dy` > 0 : vers le bas, les messages récents).
    pub fn wheel(&mut self, dy: f64) {
        let max = self.scroll_max.get();
        self.scroll = (self.scroll - dy).clamp(0.0, if max > 0.0 { max } else { f64::MAX });
    }

    /// Dessine l'assistant dans une image `w` × `h` : une carte en verre (le reste est transparent).
    pub fn draw(&self, text: &Text, w: u32, h: u32, mouse: Option<(f64, f64)>) -> Option<Pixmap> {
        let (wf, hf) = (w as f64, h as f64);
        let mut pm = crate::px::new(w.max(1), h.max(1))?;
        let c = Self::carte(wf, hf);
        let (x0, x1) = (c.x + PAD, c.x + c.w - PAD);
        round_fill(&mut pm, c, j::R_CARTE, j::FOND, 0.94);
        round_fill(&mut pm, c, j::R_CARTE, j::VERRE.0, j::VERRE.1);
        round_stroke(&mut pm, c, j::R_CARTE, j::sur_fond(j::LISERE), 1.0);
        let hover = |r: &Rect| mouse.is_some_and(|(x, y)| r.contains(x, y));
        let puce = |pm: &mut Pixmap, r: Rect, on: bool| {
            round_fill(pm, r, j::pilule(r), j::VERRE2.0, if on { j::VERRE2.1 * 2.0 } else { j::VERRE2.1 });
        };
        crate::aibtn::ai_icon(&mut pm, x0 + 11.0, c.y + 27.0, 22.0);
        Text::graisse(Graisse::Bold).draw(&mut pm, "Argos AI", ((x0 + 28.0) as f32, (c.y + 32.0) as f32), 15.0, j::TEXTE, Align::Left);
        // 🤖 « règles » mode: its tag beside the title (no network, no Claude key)
        if self.regles {
            let tw = Text::graisse(Graisse::Bold).width("Argos AI", 15.0) as f64;
            let lw = text.width(REGLES, j::T_PETIT) as f64 + 16.0;
            let r = Rect::new(x0 + 36.0 + tw, c.y + 17.0, lw, 20.0);
            round_fill(&mut pm, r, j::pilule(r), j::ACCENT, 0.25);
            text.draw(&mut pm, REGLES, (r.cx() as f32, (r.cy() + 4.0) as f32), j::T_PETIT, j::TEXTE, Align::Center);
        } else if self.key == KeyState::Ready {
            let tw = Text::graisse(Graisse::Bold).width("Argos AI", 15.0) as f64;
            text.draw(
                &mut pm,
                "· clé ouverte",
                ((x0 + 36.0 + tw) as f32, (c.y + 32.0) as f32),
                j::T_PETIT,
                GOLD,
                Align::Left,
            );
        }
        // 📍 l'endroit ouvert
        if !self.lieu.is_empty() {
            let r = Rect::new(x0, c.y + HEAD, x1 - x0, LIEU_H);
            let (fond, encre) = teinte_accent();
            round_fill(&mut pm, r, j::pilule(r), fond.0, fond.1);
            let l = text.fit(&format!("📍 {}", self.lieu), 13.5, r.w - 24.0);
            text.draw(&mut pm, &l, ((r.x + 12.0) as f32, (r.cy() + 5.0) as f32), 13.5, encre, Align::Left);
        }
        let d = self.disposition(text, wf, hf);
        let (z, top, fy) = (d.zones, d.haut, d.pied_y);
        // la conversation : bulles qui passent à la ligne, les dernières en bas
        let note_h = if self.pending_note.is_some() { NOTE_H + 8.0 } else { 0.0 };
        let (area_top, area_bot) = (top, fy - 8.0 - note_h);
        let room = (x1 - x0 - 30.0).max(80.0);
        let mut bubbles: Vec<(bool, Vec<String>)> = self.msgs.iter().map(|(me, t)| (*me, wrap(text, t, 13.5, room - 24.0))).collect();
        if self.busy {
            bubbles.push((false, vec!["✨ Argos AI réfléchit…".into()]));
        }
        if self.msgs.is_empty() && !self.busy && self.key == KeyState::Ready {
            let hint = "✨ Demande ce que tu veux : une analyse de la chart affichée, un setup, tes stats du journal, les news… Argos AI voit le symbole et l'unité de temps ouverts.";
            bubbles.push((false, wrap(text, hint, 13.5, room - 24.0)));
        }
        let mut sub = crate::px::new(w.max(1), (area_bot - area_top).max(1.0) as u32)?;
        let sh = crate::px::lh(&sub);
        // (never above the first message's top: the wheel stops there)
        let total: f64 = bubbles.iter().map(|(_, rows)| rows.len() as f64 * 19.0 + 16.0 + 10.0).sum();
        let max = (total + 10.0 - sh).max(0.0);
        self.scroll_max.set(max);
        let mut y = sh + self.scroll.min(max);
        for (me, rows) in bubbles.iter().rev() {
            let bh = rows.len() as f64 * 19.0 + 16.0;
            y -= bh + 10.0;
            if y + bh < 0.0 || y > sh {
                continue;
            }
            // mes messages : pleine largeur en bleu ; ceux d'Argos AI : carte claire (maquette pc-36)
            let r = Rect::new(x0, y, x1 - x0, bh);
            let encre = if *me {
                let (fond, encre) = teinte_accent();
                round_fill(&mut sub, r, 14.0, fond.0, fond.1);
                encre
            } else {
                round_fill(&mut sub, r, 14.0, j::BLANC, 0.06);
                j::TEXTE
            };
            for (k, l) in rows.iter().enumerate() {
                text.draw(&mut sub, l, ((x0 + 12.0) as f32, (y + 22.0 + k as f64 * 19.0) as f32), 13.5, encre, Align::Left);
            }
        }
        crate::px::put(&mut pm, 0.0, area_top, &sub);
        // carte de la clé
        let aide = match self.key {
            KeyState::None => "Ajoute ta clé API Claude : elle reste chiffrée sur cet appareil.",
            KeyState::Locked => "Ta clé est chiffrée : entre ton code pour l'ouvrir.",
            KeyState::Ready => "",
        };
        if !aide.is_empty() {
            // retour à la ligne dans la largeur du volet (2 lignes au plus, au-dessus des boutons)
            for (k, l) in aide_lignes(text, aide, x1 - x0).iter().enumerate() {
                text.draw(&mut pm, l, (x0 as f32, (d.aide_y + 20.0 + 15.5 * k as f64) as f32), 12.5, j::DIM, Align::Left);
            }
        }
        // 🧠 carte « Retenir ? » au-dessus de la barre du bas
        if let Some(n) = &self.pending_note {
            let card = Rect::new(x0, fy - NOTE_H - 8.0, x1 - x0, NOTE_H);
            round_fill(&mut pm, card, 14.0, j::BLANC, 0.06);
            round_stroke(&mut pm, card, 14.0, accent(), 1.0);
            let line = text.fit(&format!("🧠 Retenir : « {n} » ?"), T, card.w - 24.0);
            text.draw(&mut pm, &line, ((card.x + 12.0) as f32, (card.y + 22.0) as f32), T, j::TEXTE, Align::Left);
        }
        for (r, zone) in &z {
            let r = *r;
            match zone {
                Zone::Close => {
                    if hover(&r) {
                        puce(&mut pm, r, true);
                    }
                    lines(
                        &mut pm,
                        r.cx(),
                        r.cy(),
                        &[&[(-6.0, -6.0), (6.0, 6.0)], &[(6.0, -6.0), (-6.0, 6.0)]],
                        j::DIM,
                        1.6,
                    );
                }
                Zone::Memory => {
                    if hover(&r) {
                        puce(&mut pm, r, true);
                    }
                    crate::icons::draw(&mut pm, if self.sandbox { "🧪" } else { "🧠" }, r.cx(), r.cy(), 18, WHITE);
                }
                Zone::NoteYes | Zone::NoteNo => {
                    let yes = *zone == Zone::NoteYes;
                    if yes {
                        round_fill(&mut pm, r, j::pilule(r), j::ACCENT, 1.0);
                    } else {
                        puce(&mut pm, r, hover(&r));
                    }
                    let l = if yes { "✅ Retenir" } else { "Non merci" };
                    text.draw(&mut pm, l, (r.cx() as f32, (r.cy() + 5.0) as f32), T, j::TEXTE, Align::Center);
                }
                Zone::Key if r.y < c.y + HEAD => {
                    if hover(&r) {
                        puce(&mut pm, r, true);
                    }
                    crate::icons::draw(&mut pm, "🔑", r.cx(), r.cy(), 18, WHITE);
                }
                Zone::Key | Zone::Unlock | Zone::Forget => {
                    if *zone == Zone::Forget {
                        puce(&mut pm, r, hover(&r));
                    } else {
                        round_fill(&mut pm, r, j::pilule(r), j::ACCENT, 1.0);
                    }
                    let label = match zone {
                        Zone::Key => "🔑 Ajouter ma clé",
                        Zone::Unlock => "🔓 Déverrouiller",
                        _ => "Oublier la clé",
                    };
                    text.draw(&mut pm, label, (r.cx() as f32, (r.cy() + 5.0) as f32), 13.5, j::TEXTE, Align::Center);
                }
                Zone::Chip(_) | Zone::Guide => {
                    puce(&mut pm, r, hover(&r));
                    let label = match zone {
                        Zone::Chip(k) => self.puce(*k),
                        _ => "🧭 Analyse guidée".to_string(),
                    };
                    let l = text.fit(&label, 12.5, r.w - 16.0);
                    let couleur = if self.key == KeyState::Ready { j::TEXTE } else { DIM };
                    text.draw(&mut pm, &l, (r.cx() as f32, (r.cy() + 4.5) as f32), 12.5, couleur, Align::Center);
                }
                Zone::LireTout | Zone::Accord | Zone::Volatilite | Zone::NiveauxJour => {
                    let actif = *zone != Zone::Accord || self.unite_sup.is_some();
                    puce(&mut pm, r, actif && hover(&r));
                    let label = match (zone, &self.unite_sup) {
                        (Zone::LireTout, _) => LIRE_TOUT.to_string(),
                        (Zone::Volatilite, _) => VOLATILITE.to_string(),
                        (Zone::NiveauxJour, _) => NIVEAUX_JOUR.to_string(),
                        (_, Some(sup)) => format!("{ACCORD} ({} et {sup})", self.iv),
                        (_, None) => format!("{ACCORD} · pas d'unité supérieure"),
                    };
                    let l = text.fit(&label, 13.5, r.w - 24.0);
                    text.draw(
                        &mut pm,
                        &l,
                        ((r.x + 14.0) as f32, (r.cy() + 5.0) as f32),
                        13.5,
                        if actif { j::TEXTE } else { DIM },
                        Align::Left,
                    );
                }
                Zone::Effacer | Zone::TousReglages | Zone::AutresReglages => {
                    puce(&mut pm, r, hover(&r));
                    let label = match zone {
                        Zone::Effacer => EFFACER.to_string(),
                        Zone::TousReglages => TOUS_REGLAGES.to_string(),
                        _ => self.libelle_autres(),
                    };
                    let l = text.fit(&label, 12.5, r.w - 16.0);
                    text.draw(&mut pm, &l, (r.cx() as f32, (r.cy() + 4.5) as f32), 12.5, j::TEXTE, Align::Center);
                }
                Zone::Field => {
                    round_fill(&mut pm, r, j::pilule(r), j::CHAMP.0, j::CHAMP.1);
                    round_stroke(&mut pm, r, j::pilule(r), if self.typing { accent() } else { j::sur_fond(j::LISERE) }, 1.0);
                    let place = r.w - 60.0;
                    let (s, couleur) = if self.input.is_empty() {
                        ("Demande à Argos AI…".to_string(), j::DIM)
                    } else {
                        (self.input.clone(), j::TEXTE)
                    };
                    // ce qui est tapé : la fin reste visible ; le texte d'aide : coupé à la fin
                    let mut shown = if self.input.is_empty() { text.fit(&s, 13.5, place) } else { s.clone() };
                    while text.width(&shown, 13.5) as f64 > place && !shown.is_empty() {
                        shown.remove(0);
                    }
                    text.draw(&mut pm, &shown, ((r.x + 14.0) as f32, (r.cy() + 5.0) as f32), 13.5, couleur, Align::Left);
                    if self.typing {
                        let cx = r.x + 15.0 + if self.input.is_empty() { 0.0 } else { text.width(&shown, 13.5) as f64 };
                        crate::fill(&mut pm, cx, r.cy() - 9.0, 1.5, 18.0, j::TEXTE, 1.0);
                    }
                    text.draw(
                        &mut pm,
                        "Ce n'est pas un conseil financier.",
                        (r.cx() as f32, (r.y + r.h + 17.0) as f32),
                        j::T_PETIT,
                        j::DIM,
                        Align::Center,
                    );
                }
                Zone::Send => {
                    // ➤ rond dans le champ : bleu quand on peut envoyer
                    let ok = !self.busy && self.key == KeyState::Ready && !self.input.trim().is_empty();
                    round_fill(&mut pm, r, r.h / 2.0, if ok { j::ACCENT } else { j::BLANC }, if ok { 1.0 } else { 0.08 });
                    let col = if ok { j::BLANC } else { j::DIM };
                    lines(&mut pm, r.cx() + 1.0, r.cy(), &[&[(-5.0, -6.0), (6.0, 0.0), (-5.0, 6.0)]], col, 1.8);
                }
            }
        }
        Some(pm)
    }
}

/// Fond teinté à l'accent (📍 où l'on est, mes messages) et couleur de son texte. Thème clair : un bleu ardoise peu
/// saturé, que le passage en clair retourne comme les autres puces (un bleu vif y restait tel quel et rendait le texte
/// illisible).
fn teinte_accent() -> ((Rgb, f32), Rgb) {
    teinte_accent_pour(crate::palette::is_light(crate::theme_index()))
}

/// [`teinte_accent`] pour un thème clair ou sombre (sans lire le thème global : testable).
pub(crate) fn teinte_accent_pour(clair: bool) -> ((Rgb, f32), Rgb) {
    if clair {
        ((Rgb(0x2a, 0x33, 0x4a), 1.0), j::TEXTE)
    } else {
        ((j::ACCENT, 0.2), j::TEXTE)
    }
}

/// Coupe un texte en lignes de largeur `w` (les retours de ligne sont gardés).
/// L'aide de la carte de la clé, coupée à la largeur `w` : 2 lignes au plus (la 2e raccourcie si besoin).
fn aide_lignes(text: &Text, s: &str, w: f64) -> Vec<String> {
    let mut v = wrap(text, s, 12.5, w - 4.0);
    if v.len() > 2 {
        let reste = v[1..].join(" ");
        v = vec![v[0].clone(), text.fit(&reste, 12.5, w - 4.0)];
    }
    v
}

pub fn wrap(text: &Text, s: &str, size: f32, w: f64) -> Vec<String> {
    let mut out = vec![];
    for para in s.split('\n') {
        let mut line = String::new();
        for word in para.split(' ') {
            let next = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if text.width(&next, size) as f64 > w && !line.is_empty() {
                out.push(std::mem::take(&mut line));
                line = word.to_string();
            } else {
                line = next;
            }
        }
        out.push(line);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 🤖 « + N autres réglages utiles »: every letter is in the font (the full-width « ＋ » was an empty box).
    #[test]
    fn bouton_autres_reglages_sans_case_vide() {
        let t = Text::default();
        for n in [1, 3] {
            let a = Assistant {
                autres_reglages: n,
                ..Default::default()
            };
            let l = a.libelle_autres();
            assert!(l.chars().all(|c| t.has_char(c)), "« {l} »: a letter missing from the font");
        }
    }

    /// 🔐 L'aide de la carte de la clé revient à la ligne dans la largeur du volet (PC 1440 × 900 : volet de 360 ;
    /// cell : 390 moins les marges) : jamais coupée par le bord, 2 lignes au plus.
    #[test]
    fn aide_de_la_cle_dans_la_largeur() {
        let t = Text::default();
        for w in [240.0, 300.0, 336.0, 358.0, 420.0] {
            for s in [
                "Ajoute ta clé API Claude : elle reste chiffrée sur cet appareil.",
                "Ta clé est chiffrée : entre ton code pour l'ouvrir.",
            ] {
                let v = aide_lignes(&t, s, w);
                assert!(!v.is_empty() && v.len() <= 2, "{w} : {v:?}");
                for l in &v {
                    assert!(t.width(l, 12.5) as f64 <= w, "{w} : « {l} » dépasse");
                }
            }
        }
        assert_eq!(
            aide_lignes(&t, "Ajoute ta clé API Claude : elle reste chiffrée sur cet appareil.", 240.0).len(),
            2
        );
    }

    /// Jee 18 Nov (long readings): the wheel stops at the first message's top, never in the empty space above it;
    /// back down, the last message comes back at once.
    #[test]
    fn la_molette_s_arrete_au_debut_de_la_lecture() {
        let t = Text::default();
        let mut a = Assistant {
            regles: true,
            msgs: vec![(false, "ligne de lecture\n".repeat(120))],
            ..Default::default()
        };
        let _ = a.draw(&t, 400, 800, None);
        let max = a.scroll_max.get();
        assert!(max > 0.0, "a long reading scrolls");
        for _ in 0..50 {
            a.wheel(-500.0);
        }
        assert_eq!(a.scroll, max, "stops at the first message's top");
        a.wheel(1e9);
        assert_eq!(a.scroll, 0.0, "back to the last message");
    }

    /// 🤖 « règles » mode without a Claude key (Jee's phone, 18 Nov): the four lines, no Claude shortcut (they
    /// need a key), so the reading starts higher; with a key, the shortcuts come back.
    #[test]
    fn mode_regles_sans_raccourcis_claude() {
        let t = Text::default();
        let mut a = Assistant {
            regles: true,
            unite_sup: Some("2h".into()),
            ..Default::default()
        };
        let (z, haut) = a.zones(&t, 390.0, 700.0);
        assert_eq!(a.lignes_regles(&t, 390.0, 700.0).len(), 4);
        assert!(!z.iter().any(|(_, z)| matches!(z, Zone::Chip(_))), "no Claude shortcut without a key");
        a.key = KeyState::Ready;
        let (z2, haut2) = a.zones(&t, 390.0, 700.0);
        assert!(z2.iter().any(|(_, z)| matches!(z, Zone::Chip(_))), "with a key: the shortcuts");
        assert!(haut + 90.0 < haut2 + 96.0, "the reading starts higher without them: {haut} / {haut2}");
    }

    /// 📱 Jee 20 Nov (cell, « règles » mode, no key, an explanation drawn): « 🔑 Ajouter ma clé » was drawn on
    /// « 🧹 Effacer l'explication ». Whatever the panel's size and state, no two buttons overlap, each stays in the
    /// card's width, and the bottom bar stays above the field.
    #[test]
    fn boutons_jamais_l_un_sur_l_autre() {
        let t = Text::default();
        for key in [KeyState::None, KeyState::Locked, KeyState::Ready] {
            for (effacer, tous, autres) in [(true, false, 0), (true, true, 3), (false, true, 1), (false, false, 0)] {
                for regles in [true, false] {
                    let a = Assistant {
                        key,
                        regles,
                        effacer,
                        tous_reglages: tous.then_some(crate::indicators::Ind::EmaCross),
                        autres_reglages: autres,
                        lieu: "Tu es sur la chart : BTC · 5h".into(),
                        unite_sup: Some("1M".into()),
                        ..Default::default()
                    };
                    for w in [300.0, 359.0, 375.0, 404.0, 420.0] {
                        for h in [420.0, 520.0, 620.0, 700.0, 900.0] {
                            let (z, _) = a.zones(&t, w, h);
                            let c = Assistant::carte(w, h);
                            let champ = Assistant::champ(w, h);
                            let boutons: Vec<&(Rect, Zone)> = z
                                .iter()
                                .filter(|(r, zone)| {
                                    !matches!(zone, Zone::Field | Zone::Send | Zone::Close | Zone::Memory) && !(*zone == Zone::Key && r.y < c.y + HEAD)
                                })
                                .collect();
                            for (i, (r, zone)) in boutons.iter().enumerate() {
                                assert!(r.x >= c.x + PAD - 0.01 && r.x + r.w <= c.x + c.w - PAD + 0.01, "{w}×{h} {zone:?} {r:?}");
                                if matches!(
                                    zone,
                                    Zone::Key | Zone::Unlock | Zone::Forget | Zone::Effacer | Zone::Guide | Zone::TousReglages | Zone::AutresReglages
                                ) {
                                    assert!(r.y + r.h <= champ.y + 0.01, "{w}×{h} {zone:?} sous le champ");
                                }
                                for (r2, zone2) in &boutons[i + 1..] {
                                    let croise = r.x < r2.x + r2.w && r2.x < r.x + r.w && r.y < r2.y + r2.h && r2.y < r.y + r.h;
                                    assert!(!croise, "{w}×{h} {key:?} : {zone:?} {r:?} sur {zone2:?} {r2:?}");
                                }
                            }
                            assert!(a.draw(&t, w as u32, h as u32, None).is_some());
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn assistant() {
        let t = Text::default();
        let mut a = Assistant {
            key: KeyState::Ready,
            ..Default::default()
        };
        let (z, _) = a.zones(&t, 390.0, 700.0);
        // les raccourcis entrent dans la largeur
        for (r, zone) in &z {
            if let Zone::Chip(_) = zone {
                assert!(r.x >= M + PAD && r.x + r.w <= 390.0 - M - PAD, "{r:?}");
            }
        }
        let (r, _) = z.iter().find(|(_, z)| *z == Zone::Chip(0)).unwrap();
        assert_eq!(
            a.click(&t, 390.0, 700.0, r.cx(), r.cy(), "BTC 4h"),
            AsClick::Send(CHIPS[0].1.replace("{s}", "BTC 4h"))
        );
        let (f, _) = z.iter().find(|(_, z)| *z == Zone::Field).unwrap();
        assert_eq!(a.click(&t, 390.0, 700.0, f.cx(), f.cy(), ""), AsClick::Focus);
        assert!(a.typing);
        a.input = "  bonjour  ".into();
        let (s, _) = z.iter().find(|(_, z)| *z == Zone::Send).unwrap();
        assert_eq!(a.click(&t, 390.0, 700.0, s.cx(), s.cy(), ""), AsClick::Send("bonjour".into()));
        assert!(a.input.is_empty());
        // sans clé : le bouton 🔑 Ajouter ma clé
        let b = Assistant::default();
        assert!(b.zones(&t, 390.0, 700.0).0.iter().filter(|(_, z)| *z == Zone::Key).count() == 2);
        a.msgs = vec![
            (true, "salut".into()),
            (
                false,
                "Une longue réponse de Claude qui doit passer à la ligne plusieurs fois dans une bulle étroite.".into(),
            ),
        ];
        assert!(a.draw(&t, 390, 700, None).is_some());
        assert!(wrap(&t, "a b c d e f g h i j k l m n o p", 13.5, 40.0).len() > 2);
    }

    /// 🫧 verre-2 (pc-36) : la barre est fixée en bas de la carte, 🧭 Analyse guidée envoie la méthode SMC, la puce
    /// 📍 décale les raccourcis, et les coins de l'image restent transparents (le volet flotte).
    #[test]
    fn barre_fixee_en_bas_et_lieu() {
        let t = Text::default();
        let mut a = Assistant {
            key: KeyState::Ready,
            ..Default::default()
        };
        let (h1, h2) = (400.0, 900.0);
        let champ = |a: &Assistant, h: f64| a.zones(&t, 400.0, h).0.into_iter().find(|(_, z)| *z == Zone::Field).unwrap().0;
        let (c1, c2) = (champ(&a, h1), champ(&a, h2));
        assert!((h1 - (c1.y + c1.h) - (h2 - (c2.y + c2.h))).abs() < 0.01, "même distance au bas");
        assert!(c1.y + c1.h + 20.0 < h1, "la ligne « pas un conseil » tient sous le champ");
        for (c, h) in [(c1, h1), (c2, h2)] {
            assert!(
                (h - (c.y + c.h) - (M + 26.0)).abs() < 0.01,
                "{h} : le champ est collé en bas (marge + ligne d'avertissement)"
            );
        }
        let (g, _) = a.zones(&t, 400.0, h1).0.into_iter().find(|(_, z)| *z == Zone::Guide).unwrap();
        match a.click(&t, 400.0, h1, g.cx(), g.cy(), "XAUUSD 4h") {
            AsClick::Send(q) => assert!(q.contains("XAUUSD 4h") && q.contains("Ce n'est pas un conseil financier")),
            autre => panic!("{autre:?}"),
        }
        let puce0 = |a: &Assistant| a.zones(&t, 400.0, h1).0.into_iter().find(|(_, z)| *z == Zone::Chip(0)).unwrap().0;
        let sans = puce0(&a);
        a.lieu = "Tu es sur la chart : BTC · 4h".into();
        assert!(puce0(&a).y > sans.y + LIEU_H);
        let pm = a.draw(&t, 400, 700, None).unwrap();
        assert_eq!(pm.pixel(0, 0).unwrap().alpha(), 0, "coin transparent");
        assert!(pm.pixel(200, 350).unwrap().alpha() > 200, "verre au centre");
    }

    /// 📸 Argos AI pour la comparaison avec la maquette (pc-36) : seulement si `ARGOS_CAPTURES`.
    #[test]
    fn captures_pour_jee() {
        let Ok(dir) = std::env::var("ARGOS_CAPTURES") else { return };
        let t = Text::default();
        let a = Assistant {
            key: KeyState::Ready,
            lieu: "Tu es sur la chart : BTC-USDC.P · 4h · Hyperliquid".into(),
            msgs: vec![
                (false, "Structure 4h : BOS haussier, FVG non comblée sous 75 500.".into()),
                (true, "Explique-moi la zone".into()),
                (
                    false,
                    "La FVG est un trou laissé par l'impulsion ; le prix revient souvent la combler avant de repartir.".into(),
                ),
            ],
            ..Default::default()
        };
        a.draw(&t, 420, 860, None).unwrap().save_png(format!("{dir}/app-argos-ai.png")).unwrap();
    }
}
