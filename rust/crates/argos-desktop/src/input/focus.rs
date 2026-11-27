//! 🎯 Qui reçoit le clavier : une seule fenêtre à la fois, choisie dans un ordre de priorité fixe.

use crate::*;

/// La fenêtre (ou la chart) qui reçoit la touche.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Focus {
    /// Saisie d'une clé (API, Claude, OANDA, 24 mots).
    KeyBox,
    /// 🔎 Recherche universelle Ctrl + K.
    Palette,
    /// 🔎 Market panel of the symbol pill (its field, ↑ ↓, Enter, Escape).
    Marches,
    /// ⌂ Accueil (lettres des cartes, Échap).
    Accueil,
    /// ⌨️ Volet des raccourcis (Échap seulement).
    Raccourcis,
    /// ✨ Demande à l'assistant.
    Assistant,
    /// 🛟 Support panel (modal layer: its fields, Escape closes it).
    Support,
    /// 📜 Éditeur Argos Script.
    Editor,
    /// 🛒 Marketplace.
    Market,
    /// 🎓 Académie (Échap : retour, puis fermer).
    Academy,
    /// 💼 Page Portfolio.
    Portfolio,
    /// 📓 Fiche d'un trade du journal.
    Journal,
    /// 📍 Choix d'un prix / d'une heure sur la chart (Échap seulement).
    IndHere,
    /// Réglages d'un indicateur.
    IndDlg,
    /// Réglages d'un dessin.
    ObjDlg,
    /// 🔔 « Créer une alerte sur » a drawing.
    DrawAlert,
    /// Boîte de nom ou panneau ＋.
    NameBox,
    /// 🔎 Recherche du panneau Indicateurs.
    IndSearch,
    /// 😀 Recherche du sélecteur d'émojis.
    EmojiSearch,
    /// 🅘 Recherche de la page Indicateurs.
    FxSearch,
    /// Aller à une date / un symbole.
    GoTo,
    /// ⚡ 📰 📅 The news card or sheet, a release's card (Escape), the Actualités search field (typing).
    Actu,
    /// 🧭 The Explorer Hyperliquid's address field (typing).
    Hl,
    /// 🔔 The alerts panel's field « Chercher une alerte » (typing, Escape leaves it).
    Alertes,
    /// Rien d'ouvert : raccourcis de la chart.
    Chart,
}

/// Ce qui est ouvert (et peut prendre le clavier).
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Open {
    pub key_box: bool,
    pub palette: bool,
    pub marches: bool,
    pub accueil: bool,
    pub raccourcis: bool,
    pub simulateur: bool,
    pub assistant: bool,
    pub support: bool,
    pub editor: bool,
    pub market: bool,
    pub academy: bool,
    pub portfolio: bool,
    pub journal: bool,
    pub ind_here: bool,
    pub ind_dlg: bool,
    pub obj_dlg: bool,
    pub draw_alert: bool,
    pub name_box: bool,
    pub ind_search: bool,
    pub emoji_search: bool,
    pub fx_search: bool,
    pub goto: bool,
    /// ⚡ 📅 A news card or a release's card is open; 📰 the Actualités search takes the keyboard.
    pub actu: bool,
    pub actu_recherche: bool,
    /// 🧭 The Explorer Hyperliquid's address field takes the keyboard.
    pub hl_saisie: bool,
    /// 🔔 The alerts panel's search field takes the keyboard.
    pub alertes_recherche: bool,
}

impl Focus {
    /// La première fenêtre ouverte dans l'ordre de priorité. `escape` : la touche est Échap (le choix d'un prix
    /// sur la chart ne prend que celle-là, les autres vont à la fenêtre de réglages restée derrière).
    pub(crate) fn pick(o: Open, escape: bool) -> Focus {
        if o.key_box {
            Focus::KeyBox
        } else if o.palette {
            Focus::Palette
        } else if o.marches {
            Focus::Marches
        } else if o.accueil && o.name_box {
            Focus::NameBox // ✏️ Renommer une tuile par-dessus l'Accueil
        } else if o.accueil {
            Focus::Accueil
        } else if (o.raccourcis || o.simulateur) && escape {
            Focus::Raccourcis
        } else if o.assistant {
            Focus::Assistant
        } else if o.support {
            Focus::Support
        } else if o.editor && !o.market && !o.portfolio {
            Focus::Editor
        } else if o.academy {
            Focus::Academy
        } else if o.market {
            Focus::Market
        } else if o.portfolio {
            Focus::Portfolio
        } else if o.journal {
            Focus::Journal
        } else if o.ind_here && escape {
            Focus::IndHere
        } else if o.ind_dlg {
            Focus::IndDlg
        } else if o.obj_dlg {
            Focus::ObjDlg
        } else if o.draw_alert {
            Focus::DrawAlert
        } else if o.name_box {
            Focus::NameBox
        } else if o.fx_search {
            Focus::FxSearch
        } else if o.emoji_search {
            Focus::EmojiSearch
        } else if o.ind_search {
            Focus::IndSearch
        } else if o.goto {
            Focus::GoTo
        } else if o.actu_recherche || (o.actu && escape) {
            Focus::Actu
        } else if o.hl_saisie {
            Focus::Hl
        } else if o.alertes_recherche {
            Focus::Alertes
        } else {
            Focus::Chart
        }
    }
}

impl App {
    /// Ce qui est ouvert en ce moment.
    pub(crate) fn open_now(&self) -> Open {
        Open {
            key_box: self.session.key_box.is_some(),
            palette: self.ui.palette.is_some(),
            marches: self.ui.marches.is_some(),
            accueil: self.ui.accueil.is_some(),
            raccourcis: self.ui.raccourcis.is_some(),
            simulateur: self.ui.sim.is_some(),
            assistant: self.ui.ai_open && self.ai.typing,
            support: self.ui.support.is_some(),
            editor: self.ui.editor.is_some(),
            market: self.ui.mk_page.is_some(),
            academy: self.ui.ac_page.is_some(),
            // (📊 the backtest report: a full-screen page like the Portfolio, Échap closes it)
            portfolio: self.ui.pf_page.is_some() || self.ui.bt_page.is_some(),
            journal: self.ui.j_form.is_some(),
            ind_here: self.ui.ind_here.is_some(),
            ind_dlg: self.ui.ind_dlg.is_some(),
            obj_dlg: self.ui.obj_dlg.is_some(),
            draw_alert: self.ui.draw_alert.is_some(),
            name_box: self.ui.name_box.is_some() || self.ui.add.is_some(),
            ind_search: self.watch.panels.search.is_some() && self.watch.prefs.open && self.watch.prefs.panel == wl::Tool::Indicators,
            emoji_search: self.emoji_pick().is_some_and(|p| p.recherche.is_some()),
            fx_search: self.ui.fx_page.as_ref().is_some_and(|p| p.recherche.is_some())
                || self.ui.pfeuille.as_ref().is_some_and(|p| p.saisie)
                || self.ui.symboles.as_ref().is_some_and(|p| p.recherche.is_some()),
            goto: self.ui.goto.is_some(),
            actu: self.ui.eclair.is_some() || self.watch.eco.carte.is_some(),
            actu_recherche: self.watch.eco.recherche.is_some()
                && ((self.watch.prefs.open && self.watch.prefs.panel == wl::Tool::News)
                    || (self.dock.rect.is_some() && self.watch.dock.shown().iter().any(|i| i.module == argos_render::dock::Module::News))),
            hl_saisie: self.watch.explorer.saisie.is_some()
                && ((self.watch.prefs.open && self.watch.prefs.panel == wl::Tool::Explorer)
                    || (self.dock.rect.is_some() && self.watch.dock.shown().iter().any(|i| i.module == argos_render::dock::Module::Explorer))),
            alertes_recherche: self.ui.notifs.as_ref().is_some_and(|n| !n.fil && n.alertes.recherche.is_some()),
        }
    }

    /// 😀 Le sélecteur d'émojis ouvert, s'il y en a un.
    pub(crate) fn emoji_pick(&self) -> Option<&argos_render::emojipick::EmojiPick> {
        self.ui.pop.as_ref().and_then(|p| p.pick.as_ref())
    }

    /// Qui reçoit cette touche.
    pub(crate) fn focus(&self, key: &Key) -> Focus {
        Focus::pick(self.open_now(), matches!(key, Key::Named(NamedKey::Escape)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rien_d_ouvert_va_a_la_chart() {
        assert_eq!(Focus::pick(Open::default(), false), Focus::Chart);
        assert_eq!(Focus::pick(Open::default(), true), Focus::Chart);
    }

    /// 🔎 The market panel of the symbol pill takes the keyboard (over the windows behind it), after the search.
    #[test]
    fn panneau_de_marches_prend_le_clavier() {
        let o = Open {
            marches: true,
            support: true,
            ..Default::default()
        };
        assert_eq!(Focus::pick(o, false), Focus::Marches);
        let o = Open {
            marches: true,
            palette: true,
            ..Default::default()
        };
        assert_eq!(Focus::pick(o, false), Focus::Palette);
    }

    #[test]
    fn la_saisie_de_cle_passe_avant_tout() {
        let all = Open {
            key_box: true,
            palette: true,
            marches: true,
            accueil: true,
            raccourcis: true,
            simulateur: false,
            assistant: true,
            support: true,
            editor: true,
            market: true,
            academy: true,
            portfolio: true,
            journal: true,
            ind_here: true,
            ind_dlg: true,
            obj_dlg: true,
            draw_alert: true,
            name_box: true,
            ind_search: true,
            emoji_search: true,
            fx_search: true,
            goto: true,
            actu: true,
            actu_recherche: true,
            hl_saisie: true,
            alertes_recherche: true,
        };
        assert_eq!(Focus::pick(all, true), Focus::KeyBox);
    }

    #[test]
    fn marketplace_et_portfolio_passent_devant_l_editeur() {
        let o = Open {
            editor: true,
            market: true,
            ..Default::default()
        };
        assert_eq!(Focus::pick(o, false), Focus::Market);
        let o = Open {
            editor: true,
            portfolio: true,
            ..Default::default()
        };
        assert_eq!(Focus::pick(o, false), Focus::Portfolio);
        let o = Open {
            editor: true,
            journal: true,
            ..Default::default()
        };
        assert_eq!(Focus::pick(o, false), Focus::Editor);
    }

    #[test]
    fn recherche_puis_accueil_puis_raccourcis() {
        let o = Open {
            palette: true,
            accueil: true,
            raccourcis: true,
            simulateur: false,
            academy: true,
            ..Default::default()
        };
        assert_eq!(Focus::pick(o, false), Focus::Palette, "Ctrl + K par-dessus l'Accueil");
        let o = Open {
            accueil: true,
            academy: true,
            ..Default::default()
        };
        assert_eq!(Focus::pick(o, false), Focus::Accueil);
        // ✏️ renommer une tuile de l'Accueil : la boîte de nom prend le clavier
        let o = Open {
            accueil: true,
            name_box: true,
            ..Default::default()
        };
        assert_eq!(Focus::pick(o, false), Focus::NameBox);
        let o = Open {
            raccourcis: true,
            simulateur: false,
            ..Default::default()
        };
        assert_eq!(Focus::pick(o, true), Focus::Raccourcis, "Échap ferme le volet");
        assert_eq!(Focus::pick(o, false), Focus::Chart, "les autres touches restent à la chart");
    }

    #[test]
    fn choix_d_un_prix_ne_prend_que_echap() {
        let o = Open {
            ind_here: true,
            ind_dlg: true,
            ..Default::default()
        };
        assert_eq!(Focus::pick(o, true), Focus::IndHere);
        assert_eq!(Focus::pick(o, false), Focus::IndDlg);
        let o = Open {
            ind_here: true,
            ..Default::default()
        };
        assert_eq!(Focus::pick(o, false), Focus::Chart);
    }

    #[test]
    fn ordre_des_fenetres_de_reglages() {
        let o = Open {
            ind_dlg: true,
            obj_dlg: true,
            name_box: true,
            ind_search: true,
            goto: true,
            ..Default::default()
        };
        assert_eq!(Focus::pick(o, false), Focus::IndDlg);
        let o = Open {
            obj_dlg: true,
            name_box: true,
            ..Default::default()
        };
        assert_eq!(Focus::pick(o, false), Focus::ObjDlg);
        let o = Open {
            name_box: true,
            ind_search: true,
            goto: true,
            ..Default::default()
        };
        assert_eq!(Focus::pick(o, false), Focus::NameBox);
        let o = Open {
            ind_search: true,
            goto: true,
            ..Default::default()
        };
        assert_eq!(Focus::pick(o, false), Focus::IndSearch);
        let o = Open {
            goto: true,
            actu: true,
            ..Default::default()
        };
        assert_eq!(Focus::pick(o, false), Focus::GoTo);
        // ⚡ 📅 a news card or a release's card: Escape only; 📰 the Actualités search: every key
        let o = Open {
            actu: true,
            ..Default::default()
        };
        assert_eq!((Focus::pick(o, true), Focus::pick(o, false)), (Focus::Actu, Focus::Chart));
        let o = Open {
            actu_recherche: true,
            ..Default::default()
        };
        assert_eq!(Focus::pick(o, false), Focus::Actu);
    }
}
