//! 🗂 Mises en page nommées (étape 5 de synchro-direct), comme TradingView : une mise en page garde le marché et son
//! unité de temps, le split (cases, symboles, unités), les indicateurs et leurs réglages, et les dessins du symbole
//! ouvert. Menu « Mises en page » : la liste (cliquer = ouvrir), ➕ Nouvelle, 📄 Créer une copie, ✏️ Renommer,
//! 💾 Sauvegarder ; avec 🔄 Sauvegarde automatique, la mise en page ouverte suit chaque sauvegarde.
//! Un seul fichier (`layouts.json`), partagé entre les appareils par la synchro ; la dernière ouverte se rouvre
//! partout.

use crate::actions::synchro::{Derniere, SESSION};
use crate::*;
use serde::{Deserialize, Serialize};

pub(crate) const FICHIER: &str = "layouts.json";
/// Au plus autant de mises en page.
pub(crate) const MAX: usize = 50;

/// Menu ⋯ d'une tuile de l'Accueil, et sa confirmation.
const TUILE_OUVRIR: i64 = 1;
const TUILE_RENOMMER: i64 = 2;
const TUILE_DUPLIQUER: i64 = 3;
const TUILE_FAVORI: i64 = 4;
const TUILE_SUPPRIMER: i64 = 5;
const TUILE_OUI: i64 = 6;

/// Une mise en page.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Layout {
    pub(crate) nom: String,
    pub(crate) marche: Derniere,
    /// Split (`split.json`), indicateurs (`indicators.json`) et leurs réglages (`indicator_inputs.json`).
    pub(crate) split: serde_json::Value,
    pub(crate) indicateurs: serde_json::Value,
    pub(crate) reglages: serde_json::Value,
    /// Dessins et réglages du symbole ouvert : (fichier de sauvegarde du symbole, contenu).
    pub(crate) dessins: Option<(String, saved::Saved)>,
    /// ★ Favorite (étoile de sa tuile sur l'Accueil).
    pub(crate) favori: bool,
}

/// Toutes les mises en page, et celle ouverte.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Layouts {
    pub(crate) ouverte: String,
    pub(crate) liste: Vec<Layout>,
}

impl Layouts {
    /// Un nom libre : « base », sinon « base 2 », « base 3 »…
    pub(crate) fn nom_libre(&self, base: &str) -> String {
        let pris = |n: &str| self.liste.iter().any(|l| l.nom == n);
        if !pris(base) {
            return base.to_string();
        }
        (2..).map(|k| format!("{base} {k}")).find(|n| !pris(n)).unwrap_or_default()
    }

    /// Ajoute ou remplace (même nom) ; au plus `MAX`.
    pub(crate) fn garder(&mut self, l: Layout) {
        if let Some(k) = self.liste.iter().position(|x| x.nom == l.nom) {
            self.liste[k] = l;
        } else if self.liste.len() < MAX {
            self.liste.push(l);
        }
    }
}

/// 🖍 In every layout whose drawings are `file`'s, the globally synced drawings become `globaux` (added, changed,
/// removed — a drawing removed or set back to « Pas de sync » loses its copies); nothing else changes. `true`: a
/// layout changed.
pub(crate) fn sync_dans(all: &mut Layouts, file: &str, globaux: &[Drawing]) -> bool {
    let mut change = false;
    for l in all.liste.iter_mut() {
        let Some((f, sv)) = l.dessins.as_mut() else { continue };
        if f != file {
            continue;
        }
        let mut v: Vec<Drawing> = sv.drawings.iter().filter(|d| d.style.sync != 2).cloned().collect();
        v.extend(globaux.iter().cloned());
        if v != sv.drawings {
            sv.drawings = v;
            change = true;
        }
    }
    change
}

impl App {
    /// 🖍 « Synchronisation globale » (bar ⋯): the other saved layouts of this symbol get the same synced drawings.
    pub(crate) fn sync_globaux(&self) {
        let Some(file) = self.chart.save_path.clone() else { return };
        let mut all: Layouts = Self::read_prefs(FICHIER);
        let globaux: Vec<Drawing> = self.chart.drawings.iter().filter(|d| d.style.sync == 2).cloned().collect();
        if sync_dans(&mut all, &file, &globaux) {
            Self::write_prefs(FICHIER, &all);
        }
    }

    /// La mise en page telle qu'elle est à l'écran, sous le nom `nom`.
    pub(crate) fn photo_layout(&self, nom: &str) -> Layout {
        let json = |v: serde_json::Result<serde_json::Value>| v.unwrap_or(serde_json::Value::Null);
        let marche = match &self.chart.source {
            Source::Live { exchange, base } => Derniere {
                ex: exchange.id().into(),
                base: base.clone(),
                interval: self.chart.interval.secs(),
            },
            _ => Self::read_prefs::<Derniere>(SESSION),
        };
        Layout {
            nom: nom.to_string(),
            favori: Self::read_prefs::<Layouts>(FICHIER).liste.iter().any(|l| l.nom == nom && l.favori),
            marche,
            split: json(serde_json::to_value(&self.split)),
            indicateurs: json(serde_json::to_value(&self.ind.on)),
            reglages: json(serde_json::to_value(&self.ind.set)),
            dessins: self.chart.save_path.clone().map(|f| (f, self.to_saved())),
        }
    }

    /// 💾 Garde la mise en page ouverte (seulement si elle a changé).
    pub(crate) fn garder_layout(&self) {
        let mut all: Layouts = Self::read_prefs(FICHIER);
        let nom = self.top.prefs.layout_name.clone();
        let l = self.photo_layout(&nom);
        if all.liste.contains(&l) && all.ouverte == nom {
            return;
        }
        all.ouverte = nom;
        all.garder(l);
        Self::write_prefs(FICHIER, &all);
    }

    /// 🗂 Ouvre une mise en page : split, indicateurs, dessins, puis son marché. `garder_avant` : la mise en page
    /// qu'on quitte garde d'abord son état (pas quand l'ordre vient d'un autre appareil : pas de va-et-vient).
    pub(crate) fn ouvrir_layout(&mut self, l: &Layout, garder_avant: bool) {
        if garder_avant && self.top.prefs.auto_save {
            self.save();
            self.garder_layout();
        }
        if let Ok(s) = serde_json::from_value(l.split.clone()) {
            self.split = s;
            Self::write_prefs("split.json", &self.split);
        }
        if let Ok(on) = serde_json::from_value(l.indicateurs.clone()) {
            self.ind.on = on;
            Self::write_prefs("indicators.json", &self.ind.on);
        }
        if let Ok(set) = serde_json::from_value(l.reglages.clone()) {
            self.ind.set = set;
            self.garder_reglages();
        }
        if let Some((f, s)) = &l.dessins {
            if saved::file_name(f.trim_end_matches(".json")) == *f {
                let _ = saved::write(f, s);
            }
        }
        self.top.prefs.layout_name = l.nom.clone();
        self.save_top();
        let mut all: Layouts = Self::read_prefs(FICHIER);
        all.ouverte = l.nom.clone();
        Self::write_prefs(FICHIER, &all);
        self.panes.clear();
        self.sync_panes();
        match l.marche.marche() {
            Some((ex, base, iv)) => {
                self.save();
                self.write_cache();
                if let Some(iv) = iv {
                    self.chart.interval = iv;
                    self.tools.interval = iv.secs();
                }
                self.open_live(ex, base);
            }
            None => {
                self.restore_session();
                self.rebuild();
            }
        }
        self.banner = Some(format!("🗂 Mise en page « {} » ouverte", l.nom));
        self.redraw();
    }

    /// 🗂 Le menu : (nom, symbole · unité, ouverte ?) de chaque mise en page ; la mise en page ouverte y est toujours.
    pub(crate) fn liste_layouts(&self) -> Vec<(String, String, bool)> {
        let all: Layouts = Self::read_prefs(FICHIER);
        let ouverte = &self.top.prefs.layout_name;
        let mut v: Vec<(String, String, bool)> = all
            .liste
            .iter()
            .map(|l| {
                let iv = Interval(l.marche.interval.max(1)).short();
                (l.nom.clone(), format!("{} · {iv}", l.marche.base), l.nom == *ouverte)
            })
            .collect();
        if !v.iter().any(|x| x.2) {
            v.insert(0, (ouverte.clone(), format!("{} · {}", self.symbol_label(), self.chart.interval.short()), true));
        }
        v
    }

    /// Clic sur une mise en page de la liste.
    pub(crate) fn clic_layout(&mut self, k: usize) {
        let all: Layouts = Self::read_prefs(FICHIER);
        let noms: Vec<String> = self.liste_layouts().into_iter().map(|x| x.0).collect();
        let Some(nom) = noms.get(k) else { return self.redraw() };
        if *nom == self.top.prefs.layout_name {
            return self.redraw();
        }
        if let Some(l) = all.liste.iter().find(|l| l.nom == *nom).cloned() {
            self.ouvrir_layout(&l, true);
        }
    }

    /// 🔄 `layouts.json` a changé sur un autre appareil : s'il y a ouvert une autre mise en page, elle s'ouvre ici.
    pub(crate) fn layouts_recus(&mut self) {
        let all: Layouts = Self::read_prefs(FICHIER);
        if all.ouverte.is_empty() || all.ouverte == self.top.prefs.layout_name {
            return;
        }
        if let Some(l) = all.liste.iter().find(|l| l.nom == all.ouverte).cloned() {
            self.ouvrir_layout(&l, false);
        }
    }

    /// ➕ Nouvelle mise en page ou 📄 copie de celle ouverte. L'écran actuel est d'abord gardé (sauvegarde
    /// automatique) ; la nouvelle s'ouvre avec le même marché et la même unité de temps, un seul chart et les
    /// indicateurs par défaut ([`App::ecran_par_defaut`]) ; la copie garde tout.
    pub(crate) fn nouvelle_layout(&mut self, copie: bool) {
        if self.top.prefs.auto_save {
            self.save();
            self.garder_layout();
        }
        let mut all: Layouts = Self::read_prefs(FICHIER);
        if all.liste.len() >= MAX {
            self.banner = Some(format!("🗂 {MAX} mises en page au plus : renomme ou réutilise une mise en page"));
            return self.redraw();
        }
        let base = if copie {
            format!("{} (copie)", self.top.prefs.layout_name)
        } else {
            "Mise en page".to_string()
        };
        let nom = all.nom_libre(&base);
        // ➕ a new layout starts clean: one chart, the default indicators (📄 a copy keeps everything)
        if !copie {
            self.ecran_par_defaut();
        }
        all.garder(self.photo_layout(&nom));
        all.ouverte = nom.clone();
        Self::write_prefs(FICHIER, &all);
        self.top.prefs.layout_name = nom.clone();
        self.save_top();
        self.banner = Some(format!("🗂 « {nom} » créée (✏️ Renommer pour changer son nom)"));
        self.redraw();
    }

    /// ➕ The screen of a new layout (Jee, 21 Nov): one chart (split 1), the default indicators of a fresh install
    /// (`murs::DEFAUTS`), each with « mes défauts » when there are some, else its original settings. The market, the
    /// time frame and the drawings do not change; `indicators_defaults.json` is not touched.
    pub(crate) fn ecran_par_defaut(&mut self) {
        use crate::actions::murs::DEFAUTS;
        self.split.layout = "1".into();
        Self::write_prefs("split.json", &self.split);
        self.panes.clear();
        self.sync_panes();
        self.ind.on = DEFAUTS.to_vec();
        for i in DEFAUTS {
            let (vals, texts) = Self::mes_defauts(i).unwrap_or_else(|| (argos_render::indicators::values(&Default::default(), i), Default::default()));
            self.ind.set.insert(i, vals);
            self.ind.texts.insert(i, texts);
        }
        Self::write_prefs("indicators.json", &self.ind.on);
        Self::write_prefs("indicator_texts.json", &self.ind.texts);
        self.garder_reglages();
        self.ind.key = Default::default();
    }

    /// ★ Noms des mises en page favorites (tuiles de l'Accueil).
    pub(crate) fn favoris_layouts(&self) -> Vec<String> {
        Self::read_prefs::<Layouts>(FICHIER)
            .liste
            .into_iter()
            .filter(|l| l.favori)
            .map(|l| l.nom)
            .collect()
    }

    /// Le nom de la mise en page n° `k` de la liste (tuiles de l'Accueil, menu Mises en page).
    pub(crate) fn nom_layout(&self, k: usize) -> Option<String> {
        self.liste_layouts().into_iter().nth(k).map(|x| x.0)
    }

    /// Toutes les mises en page, celle ouverte comprise (gardée d'abord si elle n'est pas encore dans le fichier).
    fn layouts_avec_ouverte(&self) -> Layouts {
        let mut all: Layouts = Self::read_prefs(FICHIER);
        let nom = &self.top.prefs.layout_name;
        if !all.liste.iter().any(|l| l.nom == *nom) {
            all.garder(self.photo_layout(nom));
        }
        all
    }

    /// ★ Favori ou plus (tuile de l'Accueil, menu ⋯).
    pub(crate) fn basculer_favori(&mut self, nom: &str) {
        let mut all = self.layouts_avec_ouverte();
        if let Some(l) = all.liste.iter_mut().find(|l| l.nom == nom) {
            l.favori = !l.favori;
            self.banner = Some(if l.favori {
                format!("★ « {nom} » dans tes favoris")
            } else {
                format!("☆ « {nom} » retirée des favoris")
            });
        }
        Self::write_prefs(FICHIER, &all);
        self.rafraichir_tuiles();
    }

    /// 📄 Dupliquer une mise en page (menu ⋯) : sa copie « nom (copie) », sans l'ouvrir.
    pub(crate) fn dupliquer_layout(&mut self, nom: &str) {
        let mut all = self.layouts_avec_ouverte();
        if all.liste.len() >= MAX {
            self.banner = Some(format!("🗂 {MAX} mises en page au plus : supprime ou réutilise une mise en page"));
            return;
        }
        if let Some(l) = all.liste.iter().find(|l| l.nom == nom).cloned() {
            let copie = all.nom_libre(&format!("{nom} (copie)"));
            all.garder(Layout {
                nom: copie.clone(),
                favori: false,
                ..l
            });
            self.banner = Some(format!("📄 « {copie} » créée"));
        }
        Self::write_prefs(FICHIER, &all);
        self.rafraichir_tuiles();
    }

    /// 🗑 Supprimer une mise en page (après la confirmation du menu ⋯). Celle ouverte : une autre s'ouvre d'abord
    /// (la 1re favorite, sinon la 1re de la liste), sans garder celle qu'on supprime ; la seule ne se supprime pas
    /// (« Supprimer… » est grisé dans son menu).
    pub(crate) fn supprimer_layout(&mut self, nom: &str) {
        let mut all = self.layouts_avec_ouverte();
        let mut ouverte = None;
        if nom == self.top.prefs.layout_name {
            let autres: Vec<&Layout> = all.liste.iter().filter(|l| l.nom != nom).collect();
            let Some(cible) = autres.iter().find(|l| l.favori).or(autres.first()).map(|l| (*l).clone()) else {
                self.banner = Some("🗂 C'est la seule mise en page : crée-en une autre avant de la supprimer".into());
                return self.rafraichir_tuiles();
            };
            self.ouvrir_layout(&cible, false);
            all = Self::read_prefs(FICHIER);
            ouverte = Some(cible.nom);
        }
        all.liste.retain(|l| l.nom != nom);
        Self::write_prefs(FICHIER, &all);
        self.banner = Some(match ouverte {
            Some(o) => format!("🗑 « {nom} » supprimée · « {o} » ouverte"),
            None => format!("🗑 « {nom} » supprimée"),
        });
        self.rafraichir_tuiles();
    }

    /// ⌂ Les tuiles de l'Accueil suivent la liste (après ★, dupliquer, supprimer, renommer).
    pub(crate) fn rafraichir_tuiles(&mut self) {
        let (layouts, favoris) = (self.liste_layouts(), self.favoris_layouts());
        if let Some(p) = self.ui.accueil.as_mut() {
            (p.layouts, p.favoris) = (layouts, favoris);
        }
    }

    /// ⋯ d'une tuile de l'Accueil : Ouvrir, Renommer, Dupliquer, ★ Favori, Supprimer… (sous le bouton `anchor`).
    pub(crate) fn menu_tuile(&mut self, k: usize, anchor: Rect) {
        let Some(nom) = self.nom_layout(k) else { return self.redraw() };
        let fav = self.favoris_layouts().contains(&nom);
        let item = |id: i64, icon: &'static str, label: &str| MenuItem {
            id,
            label: label.into(),
            icon: Some(icon),
            enabled: true,
            ..Default::default()
        };
        let groups = vec![MenuGroup {
            title: nom.to_uppercase(),
            items: vec![
                item(TUILE_OUVRIR, "🗂", "Ouvrir"),
                item(TUILE_RENOMMER, "✏️", "Renommer"),
                item(TUILE_DUPLIQUER, "📄", "Dupliquer"),
                item(TUILE_FAVORI, "⭐", if fav { "Retirer des favoris" } else { "Ajouter aux favoris" }),
                // (🗑 the only layout: greyed, there is nothing to open instead)
                MenuItem {
                    enabled: self.liste_layouts().len() > 1,
                    ..item(TUILE_SUPPRIMER, "🗑", "Supprimer…")
                },
            ],
            strip: false,
        }];
        let (ww, wh) = (self.win_w(), self.top_h() + self.area.1 as f64);
        let at = Rect::new(anchor.x + anchor.w - 220.0, anchor.y, 220.0, anchor.h);
        self.ui.list = Some((ListKind::TuileLayout(k), ListMenu::new(groups, vec![], at, ww, wh)));
        self.redraw();
    }

    /// Un choix du menu ⋯ d'une tuile, ou de sa confirmation « Supprimer ? ».
    pub(crate) fn tuile_pick(&mut self, kind: ListKind, id: i64, anchor: Rect) {
        let k = match kind {
            ListKind::TuileLayout(k) | ListKind::SupprimerLayout(k) => k,
            _ => return,
        };
        let Some(nom) = self.nom_layout(k) else { return self.redraw() };
        match (kind, id) {
            (ListKind::TuileLayout(_), TUILE_OUVRIR) => {
                self.ui.accueil = None;
                return self.clic_layout(k);
            }
            (ListKind::TuileLayout(_), TUILE_RENOMMER) => {
                let ww = self.win_w();
                self.ui.layout_cible = Some(nom.clone());
                self.ui.name_box = Some(NameBox::new(NameFor::Layout, &nom, anchor, ww));
            }
            (ListKind::TuileLayout(_), TUILE_DUPLIQUER) => self.dupliquer_layout(&nom),
            (ListKind::TuileLayout(_), TUILE_FAVORI) => self.basculer_favori(&nom),
            (ListKind::TuileLayout(_), TUILE_SUPPRIMER) => {
                // 🗑 confirmation : un 2e menu « Oui, supprimer / Annuler »
                let item = |id: i64, label: String, icon: Option<&'static str>| MenuItem {
                    id,
                    label,
                    icon,
                    enabled: true,
                    ..Default::default()
                };
                let groups = vec![MenuGroup {
                    title: "SUPPRIMER CETTE MISE EN PAGE ?".into(),
                    items: vec![item(TUILE_OUI, format!("Oui, supprimer « {nom} »"), Some("🗑")), item(0, "Annuler".into(), None)],
                    strip: false,
                }];
                let (ww, wh) = (self.win_w(), self.top_h() + self.area.1 as f64);
                self.ui.list = Some((ListKind::SupprimerLayout(k), ListMenu::new(groups, vec![], anchor, ww, wh)));
            }
            (ListKind::SupprimerLayout(_), TUILE_OUI) => self.supprimer_layout(&nom),
            _ => {}
        }
        self.redraw();
    }

    /// ✏️ Renommer la mise en page `ancien` (tuile de l'Accueil) : celle ouverte passe par `renommer_layout`.
    pub(crate) fn renommer_layout_de(&mut self, ancien: &str, nom: String) {
        if ancien == self.top.prefs.layout_name {
            self.renommer_layout(nom);
            return self.rafraichir_tuiles();
        }
        let mut all: Layouts = Self::read_prefs(FICHIER);
        if all.liste.iter().any(|l| l.nom == nom) {
            self.banner = Some("🗂 Ce nom est déjà pris".into());
            return self.redraw();
        }
        if let Some(l) = all.liste.iter_mut().find(|l| l.nom == ancien) {
            l.nom = nom;
        }
        Self::write_prefs(FICHIER, &all);
        self.rafraichir_tuiles();
        self.redraw();
    }

    /// ✏️ Renommer la mise en page ouverte (son entrée dans la liste suit).
    pub(crate) fn renommer_layout(&mut self, nom: String) {
        let mut all: Layouts = Self::read_prefs(FICHIER);
        let ancien = std::mem::replace(&mut self.top.prefs.layout_name, nom.clone());
        if ancien != nom && !all.liste.iter().any(|l| l.nom == nom) {
            if let Some(l) = all.liste.iter_mut().find(|l| l.nom == ancien) {
                l.nom = nom.clone();
            }
        } else if ancien != nom {
            // un nom déjà pris : on garde l'ancien
            self.top.prefs.layout_name = ancien;
            self.banner = Some("🗂 Ce nom est déjà pris".into());
            return self.redraw();
        }
        all.ouverte = nom;
        Self::write_prefs(FICHIER, &all);
        self.save_top();
        self.garder_layout();
        self.redraw();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noms_libres_et_plafond() {
        let mut all = Layouts::default();
        assert_eq!(all.nom_libre("Mise en page"), "Mise en page");
        all.garder(Layout {
            nom: "Mise en page".into(),
            ..Default::default()
        });
        all.garder(Layout {
            nom: "Mise en page 2".into(),
            ..Default::default()
        });
        assert_eq!(all.nom_libre("Mise en page"), "Mise en page 3");
        // même nom : remplacé, pas en double
        all.garder(Layout {
            nom: "Mise en page".into(),
            marche: Derniere {
                ex: "hl".into(),
                base: "ETH".into(),
                interval: 60,
            },
            ..Default::default()
        });
        assert_eq!(all.liste.len(), 2);
        assert_eq!(all.liste[0].marche.base, "ETH");
        for k in 0..100 {
            all.garder(Layout {
                nom: format!("L{k}"),
                ..Default::default()
            });
        }
        assert_eq!(all.liste.len(), MAX);
    }
}
