//! 🤖 Lot 7 du Simulateur, « Argos AI commente tes trades » (maquette v1, « ok maquette lot 7 » de Jee le 21 nov.) :
//! - à l'ouverture d'une position démo, ce que disaient les indicateurs de la chart est gardé avec elle
//!   (`Position::lecture`, rangé par `argos_ai::read_for_trade`) ;
//! - à la sortie qui finit un trade (pas une sortie partielle), la carte « Argos AI · ton trade » s'ouvre
//!   (`argos_render::avis`) avec le commentaire des règles d'`argos_ai::review` ; le trade suivant la remplace, ✕ la
//!   ferme ;
//! - un toucher sur une ligne de l'Historique du Simulateur la rouvre.
//!
//! Jamais un ordre : la carte ne fait que lire ce que le trade a gardé.

use crate::*;
use argos_core::portfolio::ClosedTrade;
use argos_render::avis::{Avis, AvisClick, Partie, Ton};
use argos_render::text::Text;

/// 🤖 Le trade dont la carte est ouverte, par sa dernière sortie (symbole, heures d'ouverture et de sortie) : retrouvé
/// dans l'historique du compte démo à chaque dessin (un autre compte démo, une remise à zéro : plus de carte).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AvisDe {
    base: String,
    opened_at: i64,
    closed_at: i64,
}

impl AvisDe {
    pub(crate) fn de(c: &ClosedTrade) -> Self {
        Self {
            base: c.base.clone(),
            opened_at: c.opened_at,
            closed_at: c.closed_at,
        }
    }
}

/// 🤖 A part of Argos AI's comment (its tone, its chip, its sentence), as the cards show it.
pub(crate) fn partie(p: argos_ai::Part) -> Partie {
    Partie {
        ton: match p.tone {
            argos_ai::Tone::Good => Ton::Bon,
            argos_ai::Tone::Mixed => Ton::Moyen,
            argos_ai::Tone::Bad => Ton::Mauvais,
            argos_ai::Tone::Neutral => Ton::Neutre,
        },
        pastille: p.chip,
        texte: p.text,
    }
}

/// 🤖 Le commentaire d'Argos AI, tel que la carte le montre.
fn carte(v: argos_ai::TradeReview) -> Avis {
    Avis {
        long: v.long,
        base: v.base,
        r: v.r.map(argos_ai::r_text),
        net: v.net,
        sortie: v.exit,
        entree: partie(v.entry),
        plan: partie(v.plan),
        lecon: v.lesson,
        academie: v.academy,
        rappel: v.reminder.to_string(),
    }
}

impl App {
    /// 🤖 Garde avec la position démo `id` ce que disaient les indicateurs de la chart quand elle s'ouvre : lus sur les
    /// bougies clôturées (`avant` : seulement celles d'avant cette heure, quand un ordre en attente part sur une bougie
    /// du replay).
    pub(crate) fn noter_lecture(&mut self, id: u64, avant: Option<i64>) {
        let Some((base, long)) = self
            .demo
            .positions
            .iter()
            .find(|p| p.id == id)
            .map(|p| (p.base.clone(), p.side == argos_core::Side::Long))
        else {
            return;
        };
        let l = self.lecture_a_l_entree(&base, long, avant);
        if let Some(p) = self.demo.positions.iter_mut().find(|p| p.id == id) {
            p.lecture = Some(l);
        }
    }

    /// 🤖 La carte du dernier trade fini parmi ces sorties (pas une sortie partielle, pas une séance de replay
    /// oubliée) : elle remplace celle d'avant. Rien de fini : la carte ouverte reste.
    pub(crate) fn avis_apres(&mut self, closed: &[ClosedTrade]) {
        if let Some(c) = closed.iter().rev().find(|c| !c.partiel && c.reason != "séance oubliée") {
            self.ui.avis = Some(AvisDe::de(c));
        }
    }

    /// 🤖 L'index, dans l'historique du compte démo, de la dernière sortie du trade dont la carte est ouverte.
    pub(crate) fn avis_index(&self) -> Option<usize> {
        let a = self.ui.avis.as_ref()?;
        self.demo
            .history
            .iter()
            .rposition(|h| !h.partiel && h.base == a.base && h.opened_at == a.opened_at && h.closed_at == a.closed_at)
    }

    /// 🤖 La carte et sa place dans la fenêtre, quand elle se montre : un trade choisi et pas de page ouverte
    /// par-dessus la chart ; 📱 Argos AI ouvert la cache (sa feuille prend le bas de l'écran). 🖥 Le volet du
    /// Simulateur ouvert : la carte reste à sa gauche. ⏪ En replay : jamais sur la barre du replay.
    pub(crate) fn avis_place(&self, text: &Text) -> Option<(Avis, Rect)> {
        if self.page_ouverte() || (self.top.phone && self.ui.ai_open) {
            return None;
        }
        let k = self.avis_index()?;
        let h = &self.demo.history;
        let a = carte(argos_ai::review(&argos_ai::trade_parts(&h[..=k], &h[k]))?);
        let vp = &self.chart.viewport;
        let mut droite = self.rw() + vp.plot_w();
        if self.ui.sim.is_some() && !self.top.phone {
            droite = droite.min(self.sim_rect().x);
        }
        let chart = Rect::new(self.rw(), self.top_h(), (droite - self.rw()).max(0.0), vp.plot_h());
        // ⏪ en replay, sa barre (▶, ⏭) reste visible et à portée : la carte qui la couvrirait se pose au-dessus
        let barre = self.replay.is_some().then(|| {
            let (zone_w, ph) = self.replay_zone();
            let b = argos_render::replay::rect(zone_w, ph);
            Rect::new(b.x + self.rw(), b.y + self.top_h(), b.w, b.h)
        });
        let r = a.place(text, chart, self.win_w(), self.top.phone, barre);
        Some((a, r))
    }

    /// 🤖 Dessine la carte (et rend sa place).
    pub(crate) fn avis_draw(&self, text: &Text) -> Option<(tiny_skia::Pixmap, Rect)> {
        let (a, r) = self.avis_place(text)?;
        let survol = self.survol(Some(self.pointer.win_mouse)).map(|(x, y)| (x - r.x, y - r.y));
        Some((a.draw(text, r.w, survol)?, r))
    }

    /// 🤖 La carte est-elle sous ce point (fenêtre), sans rien au-dessus d'elle ? Les menus, fenêtres, volets et boîtes
    /// sont dessinés par-dessus la carte : ils prennent le clic, la molette et le curseur avant elle.
    pub(crate) fn avis_sous(&self, x: f64, y: f64) -> bool {
        self.ui.avis.is_some()
            && !self.ui.context.is_open()
            && self.ui.goto.is_none()
            && !self.couche_haute(x, y)
            && !(self.ui.sim.is_some() && self.sim_rect().contains(x, y))
            && self.avis_place(&Text::default()).is_some_and(|(_, r)| r.contains(x, y))
    }

    /// 🤖 Sur la carte (rien au-dessus) : `Some(true)` sur ✕ et sur la leçon (la main), `Some(false)` ailleurs (la
    /// flèche).
    pub(crate) fn avis_main(&self, x: f64, y: f64) -> Option<bool> {
        if !self.avis_sous(x, y) {
            return None;
        }
        let text = Text::default();
        let (a, r) = self.avis_place(&text)?;
        Some(a.click(&text, r.w, x - r.x, y - r.y) != AvisClick::Rien)
    }

    /// 🤖 Clic sur la carte : ✕ la ferme ; 🎓 ouvre la leçon de l'Académie (et ferme la carte) ; ailleurs, rien.
    pub(crate) fn avis_click(&mut self, x: f64, y: f64) {
        let text = Text::default();
        let Some((a, r)) = self.avis_place(&text) else { return };
        match a.click(&text, r.w, x - r.x, y - r.y) {
            AvisClick::Rien => {}
            AvisClick::Fermer => self.ui.avis = None,
            AvisClick::Lecon(id) => {
                self.ui.avis = None;
                return self.palette_cible(argos_render::recherche::Cible::Lecon(id.to_string()));
            }
        }
        self.redraw();
    }

    /// 🤖 Un toucher sur une ligne de l'Historique du Simulateur : la carte de ce trade (le même toucher sur la ligne du
    /// trade dont la carte est ouverte la referme). 📱 Le volet couvre l'écran : il se ferme, la carte se montre sur la
    /// chart, au-dessus de la barre du bas.
    pub(crate) fn avis_historique(&mut self, k: usize) {
        if self.avis_index() == Some(k) {
            self.ui.avis = None;
            return;
        }
        let Some(c) = self.demo.history.get(k).filter(|h| !h.partiel) else { return };
        self.ui.avis = Some(AvisDe::de(c));
        if self.top.phone {
            self.ui.sim = None;
        }
    }
}
