//! 🎡 Roue de la pilule « symbole · unité » du cell (vidéo de Jee, TradingView mobile) : le doigt se pose sur une
//! moitié de la pilule et glisse verticalement ; dès quelques points, une roue de verre s'ouvre au-dessus de la pilule
//! et la liste (symboles de la liste ouverte, ou unités de temps) défile sous le doigt, 1 point de doigt = 1 point de
//! liste ; la chart change en direct pour la ligne du milieu ; relâcher garde cette ligne et referme la roue. Un simple
//! toucher (sans glisser) garde son action d'avant (la feuille Symbole, les unités de temps).

use crate::*;
use argos_render::ecrans::CellHit;

/// Hauteur d'une ligne de la roue (points) et seuil du glisser.
pub(crate) const LIGNE: f64 = 24.0;
const SEUIL: f64 = 8.0;

/// Ce que fait la roue.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RoueSorte {
    Symbole,
    Unite,
}

/// 🎡 Un doigt posé sur la pilule (la roue s'ouvre quand il glisse).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Roue {
    pub(crate) sorte: RoueSorte,
    /// La moitié de la pilule touchée (fenêtre) et ce qu'aurait fait un simple toucher.
    pub(crate) zone: Rect,
    pub(crate) toucher: CellHit,
    /// Hauteur du doigt au départ et maintenant.
    pub(crate) y0: f64,
    pub(crate) y: f64,
    /// Les lignes (texte) et leur valeur (symbole ou secondes), la ligne du départ.
    pub(crate) lignes: Vec<(String, String)>,
    pub(crate) depart: usize,
    /// La roue est ouverte (le doigt a glissé) ; la ligne déjà appliquée à la chart.
    pub(crate) ouverte: bool,
    pub(crate) appliquee: usize,
}

impl Roue {
    /// La ligne sous le doigt (au milieu de la roue) : le doigt qui monte fait venir les lignes d'après.
    pub(crate) fn ligne(&self) -> usize {
        let pas = ((self.y0 - self.y) / LIGNE).round() as i64;
        (self.depart as i64 + pas).clamp(0, self.lignes.len().saturating_sub(1) as i64) as usize
    }

    /// Décalage fin (en lignes, −0,5 … 0,5) pour que la liste suive le doigt au point près.
    pub(crate) fn fraction(&self) -> f64 {
        let f = (self.y0 - self.y) / LIGNE;
        f - f.round()
    }

    /// Place de la roue (fenêtre) : juste au-dessus de la moitié de pilule touchée.
    pub(crate) fn rect(&self) -> Rect {
        let b = argos_render::ecrans::boite_de("roue").unwrap_or([12.0, 552.0, 140.0, 168.0]);
        Rect::new(self.zone.x, self.zone.y - 8.0 - b[3], b[2].max(self.zone.w), b[3])
    }
}

impl App {
    /// 🎡 Les lignes de la roue : symboles de la liste ouverte, ou unités de temps (avec la ligne actuelle).
    fn roue_lignes(&self, sorte: &RoueSorte) -> (Vec<(String, String)>, usize) {
        match sorte {
            RoueSorte::Symbole => {
                let lignes: Vec<(String, String)> = self
                    .watch
                    .order()
                    .into_iter()
                    .map(|c| {
                        let l = self
                            .watch
                            .sections
                            .iter()
                            .flat_map(|s| &s.rows)
                            .find(|r| r.coin == c)
                            .map_or_else(|| c.clone(), |r| r.label.clone());
                        (l, c)
                    })
                    .collect();
                let k = lignes.iter().position(|(_, c)| *c == self.watch.selected).unwrap_or(0);
                (lignes, k)
            }
            RoueSorte::Unite => {
                let mut s: Vec<i64> = SECTIONS
                    .iter()
                    .flat_map(|(_, l)| l.iter().copied())
                    .chain(self.tools.custom.iter().copied())
                    .collect();
                s.retain(|&x| self.interval_ok(Interval(x)));
                s.sort_unstable();
                s.dedup();
                let cur = self.chart.interval.secs();
                let k = s.iter().position(|&x| x == cur).unwrap_or(0);
                (s.into_iter().map(|x| (Interval(x).label(), x.to_string())).collect(), k)
            }
        }
    }

    /// 🎡 Le doigt se pose sur la pilule : la roue attend un glisser (sinon, au relâcher, le toucher d'avant).
    pub(crate) fn roue_poser(&mut self, hit: CellHit, r: Rect) -> bool {
        let sorte = match hit {
            CellHit::Symbole | CellHit::SymbolePrec | CellHit::SymboleSuiv => RoueSorte::Symbole,
            CellHit::Unite | CellHit::UnitePrec | CellHit::UniteSuiv => RoueSorte::Unite,
            _ => return false,
        };
        // la moitié entière de la pilule (molette comprise)
        let z = match sorte {
            RoueSorte::Symbole => argos_render::ecrans::cell().place("pg.0", self.win_w(), self.cell_h()),
            RoueSorte::Unite => argos_render::ecrans::cell().place("pg.3", self.win_w(), self.cell_h()),
        }
        .unwrap_or(r);
        let (lignes, depart) = self.roue_lignes(&sorte);
        let y = self.pointer.win_mouse.1;
        self.ui.roue = Some(Roue {
            sorte,
            zone: Rect::new(z.x, z.y, z.w, z.h),
            toucher: hit,
            y0: y,
            y,
            lignes,
            depart,
            ouverte: false,
            appliquee: depart,
        });
        true
    }

    /// 🎡 Le doigt bouge : la roue s'ouvre au-delà du seuil, la liste suit, la chart change pour la ligne du milieu.
    pub(crate) fn roue_bouger(&mut self, y: f64) {
        let Some(r) = self.ui.roue.as_mut() else { return };
        r.y = y;
        r.ouverte |= (r.y - r.y0).abs() > SEUIL;
        if !r.ouverte || r.lignes.is_empty() {
            return self.redraw();
        }
        let k = r.ligne();
        if k != r.appliquee {
            r.appliquee = k;
            let (sorte, v) = (r.sorte.clone(), r.lignes[k].1.clone());
            self.roue_appliquer(&sorte, &v);
        }
        self.redraw();
    }

    fn roue_appliquer(&mut self, sorte: &RoueSorte, v: &str) {
        match sorte {
            RoueSorte::Symbole => self.open_coin(v.to_string()),
            RoueSorte::Unite => {
                if let Ok(s) = v.parse::<i64>() {
                    self.set_interval(Interval(s));
                }
            }
        }
    }

    /// 🎡 Le doigt se lève : la roue ouverte garde sa ligne et se referme ; sans glisser, le toucher d'avant.
    pub(crate) fn roue_lever(&mut self) {
        let Some(r) = self.ui.roue.take() else { return };
        if !r.ouverte {
            return self.cell_click(r.toucher, r.zone);
        }
        self.redraw();
    }
}
