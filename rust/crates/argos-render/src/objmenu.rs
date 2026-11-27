//! 🖱️ Menu du clic droit sur un dessin (comme TradingView) : alerte, ordre de visualisation, visibilité des
//! intervalles, cloner, copier, inverser (positions), 🖌 pinceau de zone (rectangles), verrouiller, cacher, retirer,
//! configurations.
//! Une table (`ROWS`) donne l'ordre, le nom, l'icône et le raccourci ; le desktop fait l'action.

use crate::actions::{shortcut, ChartAction};
use crate::drawings::Drawing;
use crate::listmenu::{MenuGroup, MenuItem};
use crate::position::is_position;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjAction {
    Alert = 1,
    Front,
    Back,
    Intervals,
    Clone,
    Copy,
    Invert,
    LimitOrder,
    Sync,
    Tree,
    Lock,
    Hide,
    Remove,
    Settings,
    Pinceau,
}

use ObjAction::*;

/// (action, nom, icône, section). Une section = un bloc séparé dans le menu.
const ROWS: [(ObjAction, &str, Option<&str>, u8); 15] = [
    (Alert, "Ajouter une alerte", Some("alarm"), 0),
    (Front, "Mettre au premier plan", None, 1),
    (Back, "Mettre en arrière-plan", None, 1),
    (Intervals, "Visibilité des intervalles…", None, 1),
    (Clone, "Cloner", None, 2),
    (Copy, "Copier", None, 2),
    (Invert, "Inverser", None, 2),
    (LimitOrder, "Créer un ordre limite…", None, 2),
    (Pinceau, "🖌 Pinceau de zone", None, 2),
    (Sync, "Synchroniser sur toutes les charts", None, 3),
    (Tree, "Arborescence des objets…", None, 3),
    (Lock, "Verrouiller", Some("lock"), 4),
    (Hide, "Cacher", None, 4),
    (Remove, "Retirer", Some("trash"), 4),
    (Settings, "Configurations…", Some("gear"), 5),
];

impl ObjAction {
    pub fn from_id(id: i64) -> Option<Self> {
        ROWS.iter().map(|r| r.0).find(|a| *a as i64 == id)
    }

    /// Pas encore fait : le message à afficher.
    pub fn soon(self) -> Option<&'static str> {
        match self {
            LimitOrder => Some("Ordre limite depuis une position : bientôt (trading démo d'abord)"),
            Sync => Some("Synchroniser sur toutes les charts : bientôt (multi-charts)"),
            _ => None,
        }
    }
}

/// Le menu pour ce dessin (Inverser et Ordre limite seulement pour les positions).
pub fn groups(d: &Drawing) -> Vec<MenuGroup> {
    let pos = is_position(&d.tool);
    let mut out: Vec<MenuGroup> = vec![];
    let mut last = u8::MAX;
    for (a, name, icon, sec) in ROWS {
        if matches!(a, Invert | LimitOrder) && !pos {
            continue;
        }
        if a == Pinceau && d.tool != "rect" {
            continue;
        }
        // 🔔 an alert follows a line (the drawing bar's alarm, same window)
        if a == Alert && !crate::drawbar::alertable(&d.tool) {
            continue;
        }
        let label = match a {
            Lock if d.locked() => "Déverrouiller",
            _ => name,
        };
        let hint = match a {
            Copy => shortcut(ChartAction::CopyDrawing).unwrap_or_default(),
            Remove => "Suppr".into(),
            Clone => "Ctrl + glisser".into(),
            _ => String::new(),
        };
        let item = MenuItem {
            id: a as i64,
            label: label.into(),
            enabled: !(a == Remove && d.locked()),
            icon,
            hint,
            ..Default::default()
        };
        if sec != last {
            out.push(MenuGroup {
                title: String::new(),
                items: vec![],
                strip: false,
            });
            last = sec;
        }
        if let Some(g) = out.last_mut() {
            g.items.push(item);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drawings::Point;

    #[test]
    fn menu_selon_l_objet() {
        let ids = |d: &Drawing| groups(d).iter().flat_map(|g| g.items.iter().map(|i| i.id)).collect::<Vec<_>>();
        let mut t = Drawing::new("trend", vec![Point { t: 0, p: 1.0 }]);
        assert!(!ids(&t).contains(&(Invert as i64)), "Inverser : positions seulement");
        let p = Drawing::new("short", vec![Point { t: 0, p: 1.0 }]);
        assert!(ids(&p).contains(&(Invert as i64)));
        t.set_locked(true);
        let g = groups(&t);
        let rm = g.iter().flat_map(|g| &g.items).find(|i| i.id == Remove as i64).unwrap();
        assert!(!rm.enabled, "objet verrouillé : on ne peut pas le retirer");
        assert!(g.iter().flat_map(|g| &g.items).any(|i| i.label == "Déverrouiller"));
        assert_eq!(ObjAction::from_id(Settings as i64), Some(Settings));
        assert!(Alert.soon().is_none() && Clone.soon().is_none(), "🔔 the alert window of a drawing");
        assert!(
            ids(&t).contains(&(Alert as i64)) && !ids(&p).contains(&(Alert as i64)),
            "an alert follows a line"
        );
        // 🖌 Pinceau de zone : seulement sur un rectangle
        assert!(!ids(&p).contains(&(Pinceau as i64)));
        let r = Drawing::new("rect", vec![Point { t: 0, p: 1.0 }, Point { t: 60, p: 2.0 }]);
        assert!(ids(&r).contains(&(Pinceau as i64)));
        assert!(Pinceau.soon().is_none());
    }
}
