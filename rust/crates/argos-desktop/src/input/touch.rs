//! 👆 Gestes tactiles (cell, tablette, écran tactile) : un doigt fait la souris, deux doigts pincent pour zoomer, un
//! appui long fait le clic droit, un coup de doigt rapide vers la droite ou vers le bas ferme la page ouverte.
//! `Touches` ne fait que reconnaître les gestes (testable seul) ; la fenêtre les joue avec ses fonctions de souris.

/// Ce que la fenêtre doit faire (coordonnées de la fenêtre).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Gesture {
    /// Bouton gauche appuyé / souris déplacée / bouton relâché.
    Press(f64, f64),
    Move(f64, f64),
    Release(f64, f64),
    /// Appui long : clic droit (le menu de ce qui est sous le doigt).
    RightClick(f64, f64),
    /// 👆 The finger still down after the long press moves: the crosshair follows it.
    Hold(f64, f64),
    /// Pincement autour de `(cx, cy)` : écart horizontal et vertical multipliés par `sx` et `sy` (> 1 : on écarte).
    Pinch {
        cx: f64,
        cy: f64,
        sx: f64,
        sy: f64,
    },
    /// Coup de doigt rapide vers la droite (`down` faux) ou vers le bas, parti de `(x, y)` : fermer la page ouverte.
    Swipe {
        x: f64,
        y: f64,
        down: bool,
    },
}

/// Horloge des gestes (millisecondes depuis le premier appel).
pub(crate) fn now_ms() -> f64 {
    static T0: std::sync::OnceLock<web_time::Instant> = std::sync::OnceLock::new();
    T0.get_or_init(web_time::Instant::now).elapsed().as_secs_f64() * 1000.0
}

/// Appui long, en millisecondes.
pub(crate) const LONG_MS: f64 = 500.0;
/// 👆 (B-047) A long press whose time came while the app was busy waits this much (milliseconds) for one more look:
/// a lift already waiting in the browser is read first.
pub(crate) const RELIRE_MS: f64 = 50.0;
/// Au-delà de ce déplacement (pixels), le doigt glisse : ce n'est plus un tap ni un appui long.
const SLOP: f64 = 10.0;
/// 👆 A real tap on a pane's legend button: the finger lifted less than this far (points) from where it went down.
pub(crate) const TAP_STILL: f64 = 8.0;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Start {
    x: f64,
    y: f64,
    /// When the down was read (the long press counts from it: a lift not read yet can still make a tap, B-047).
    t: f64,
    /// 👆 (B-048) When the finger went down, its own time (a swipe's speed is counted with the lift's own time).
    te: f64,
}

/// État des doigts posés.
#[derive(Debug, Default)]
pub(crate) struct Touches {
    /// (identifiant, x, y) de chaque doigt posé.
    pts: Vec<(u64, f64, f64)>,
    start: Option<Start>,
    /// Le bouton gauche est appuyé (le doigt glisse : la chart défile, ou on dessine).
    pressed: bool,
    /// L'appui long a déjà fait son clic droit.
    long: bool,
    /// Écart (horizontal, vertical) des deux doigts au dernier mouvement.
    span: Option<(f64, f64)>,
    /// Un pincement a eu lieu : rien d'autre jusqu'à ce que tous les doigts soient levés.
    done: bool,
    /// 👆 How far the finger of the last tap went between going down and lifting (points).
    tap_moved: f64,
    /// 🏷️ The press being played is no real tap on a legend button: the legend lets it through to what is under it.
    legende_muette: bool,
}

impl Touches {
    /// 🖐 Deux doigts posés (pincement en cours).
    pub(crate) fn pince(&self) -> bool {
        self.pts.len() >= 2
    }

    /// 👆 At least one finger is down.
    pub(crate) fn doigt_pose(&self) -> bool {
        !self.pts.is_empty()
    }

    /// 👆 How far the finger of the last tap moved between going down and lifting (points): a legend button acts only
    /// under [`TAP_STILL`].
    pub(crate) fn tap_moved(&self) -> f64 {
        self.tap_moved
    }

    /// 🏷️ The press being played goes past the legend's buttons (a finger sliding from one, or lifted too far).
    pub(crate) fn legende_muette(&self) -> bool {
        self.legende_muette
    }

    pub(crate) fn set_legende_muette(&mut self, v: bool) {
        self.legende_muette = v;
    }

    /// Un doigt (ou un appui) en attente d'appui long ? (la fenêtre se réveille pour `tick`)
    pub(crate) fn waiting(&self) -> bool {
        self.pts.len() == 1 && !self.pressed && !self.long && !self.done
    }

    /// Quand l'appui long tombera (millisecondes de `now_ms`), si un doigt attend.
    pub(crate) fn long_at(&self) -> Option<f64> {
        self.start.filter(|_| self.waiting()).map(|s| s.t + LONG_MS)
    }

    fn spread(&self) -> Option<(f64, f64, f64, f64)> {
        let (a, b) = (self.pts.first()?, self.pts.get(1)?);
        Some(((a.1 + b.1) / 2.0, (a.2 + b.2) / 2.0, (a.1 - b.1).abs(), (a.2 - b.2).abs()))
    }

    /// Un doigt se pose (`now` en millisecondes).
    #[cfg(test)]
    pub(crate) fn down(&mut self, id: u64, x: f64, y: f64, now: f64) -> Vec<Gesture> {
        self.down_at(id, x, y, now, now)
    }

    /// 👆 A finger goes down: read at `now`, done by the finger at `te` (its own time, B-048; milliseconds).
    pub(crate) fn down_at(&mut self, id: u64, x: f64, y: f64, now: f64, te: f64) -> Vec<Gesture> {
        self.pts.retain(|p| p.0 != id);
        self.pts.push((id, x, y));
        let mut out = vec![];
        match self.pts.len() {
            1 => {
                (self.start, self.pressed, self.long, self.span, self.done) = (Some(Start { x, y, t: now, te }), false, false, None, false);
            }
            2 => {
                // deuxième doigt : le glisser en cours s'arrête, le pincement commence
                if self.pressed {
                    let (_, px, py) = self.pts[0];
                    out.push(Gesture::Release(px, py));
                    self.pressed = false;
                }
                self.done = true;
                self.span = self.spread().map(|s| (s.2, s.3));
            }
            _ => {}
        }
        out
    }

    /// Un doigt bouge.
    pub(crate) fn moved(&mut self, id: u64, x: f64, y: f64) -> Vec<Gesture> {
        let Some(p) = self.pts.iter_mut().find(|p| p.0 == id) else {
            return vec![];
        };
        (p.1, p.2) = (x, y);
        if self.pts.len() >= 2 {
            let (Some((cx, cy, w, h)), Some((w0, h0))) = (self.spread(), self.span) else {
                return vec![];
            };
            self.span = Some((w, h));
            // écart trop petit dans un sens : pas de zoom dans ce sens (évite les sauts quand les doigts se croisent)
            let k = |a: f64, b: f64| if a > 20.0 && b > 20.0 { a / b } else { 1.0 };
            return vec![Gesture::Pinch {
                cx,
                cy,
                sx: k(w, w0),
                sy: k(h, h0),
            }];
        }
        if self.done {
            return vec![];
        }
        if self.long {
            return vec![Gesture::Hold(x, y)];
        }
        if self.pressed {
            return vec![Gesture::Move(x, y)];
        }
        match self.start {
            Some(s) if (x - s.x).hypot(y - s.y) > SLOP => {
                self.pressed = true;
                vec![Gesture::Move(s.x, s.y), Gesture::Press(s.x, s.y), Gesture::Move(x, y)]
            }
            _ => vec![],
        }
    }

    /// Un doigt se lève (`te` : son heure à lui, B-048 ; millisecondes).
    pub(crate) fn up(&mut self, id: u64, x: f64, y: f64, te: f64) -> Vec<Gesture> {
        let was = self.pts.len();
        self.pts.retain(|p| p.0 != id);
        if was != 1 || !self.pts.is_empty() {
            // fin d'un pincement : on attend que tous les doigts soient levés
            if self.pts.is_empty() {
                *self = Self::default();
            }
            return vec![];
        }
        let (start, pressed, long, done) = (self.start, self.pressed, self.long, self.done);
        *self = Self::default();
        let Some(s) = start else { return vec![] };
        if done || long {
            return vec![];
        }
        let (dx, dy, fast) = (x - s.x, y - s.y, te - s.te < 700.0);
        let right = fast && dx > 80.0 && dy.abs() < 60.0;
        let down = fast && dy > 110.0 && dx.abs() < 60.0;
        self.tap_moved = dx.hypot(dy);
        let mut out = if pressed {
            vec![Gesture::Move(x, y), Gesture::Release(x, y)]
        } else {
            // tap : appui et relâche au même endroit (deux taps rapprochés = double-clic, comme à la souris)
            vec![Gesture::Move(s.x, s.y), Gesture::Press(s.x, s.y), Gesture::Release(s.x, s.y)]
        };
        if right || down {
            out.push(Gesture::Swipe { x: s.x, y: s.y, down });
        }
        out
    }

    /// Le temps passe : un doigt immobile depuis `LONG_MS` fait le clic droit (une fois).
    pub(crate) fn tick(&mut self, now: f64) -> Vec<Gesture> {
        match self.start {
            Some(s) if self.waiting() && now - s.t >= LONG_MS => {
                self.long = true;
                vec![Gesture::Move(s.x, s.y), Gesture::RightClick(s.x, s.y)]
            }
            _ => vec![],
        }
    }

    /// ↕️ The long press fell on something the finger holds and drags (a pane's top edge): from now on the finger is a
    /// pressed button — its moves are `Move`, lifting it is `Release` — and never a swipe.
    pub(crate) fn grab(&mut self) {
        if self.pts.len() == 1 && !self.done {
            (self.pressed, self.long) = (true, false);
            if let Some(s) = self.start.as_mut() {
                (s.t, s.te) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
            }
        }
    }

    /// Le système annule les doigts (appel entrant, geste du système) : on relâche tout.
    pub(crate) fn cancel(&mut self) -> Vec<Gesture> {
        let out = match (self.pressed, self.pts.first()) {
            (true, Some(&(_, x, y))) => vec![Gesture::Release(x, y)],
            _ => vec![],
        };
        *self = Self::default();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::Gesture::*;
    use super::*;

    #[test]
    fn tap_et_glisser_a_un_doigt() {
        let mut t = Touches::default();
        assert!(t.down(1, 100.0, 100.0, 0.0).is_empty(), "rien avant de savoir ce que fait le doigt");
        assert!(t.moved(1, 104.0, 102.0).is_empty(), "petit tremblement : toujours un tap");
        assert_eq!(
            t.up(1, 104.0, 102.0, 120.0),
            vec![Move(100.0, 100.0), Press(100.0, 100.0), Release(100.0, 100.0)]
        );
        // glisser : appui là où le doigt s'est posé, puis la souris suit
        t.down(1, 100.0, 100.0, 0.0);
        assert_eq!(t.moved(1, 130.0, 100.0), vec![Move(100.0, 100.0), Press(100.0, 100.0), Move(130.0, 100.0)]);
        assert_eq!(t.moved(1, 140.0, 105.0), vec![Move(140.0, 105.0)]);
        assert_eq!(
            t.up(1, 140.0, 105.0, 2_000.0),
            vec![Move(140.0, 105.0), Release(140.0, 105.0)],
            "lent : pas un coup de doigt"
        );
    }

    #[test]
    fn appui_long_fait_le_clic_droit() {
        let mut t = Touches::default();
        t.down(1, 50.0, 60.0, 1_000.0);
        assert!(t.waiting());
        assert!(t.tick(1_400.0).is_empty(), "pas encore 500 ms");
        assert_eq!(t.tick(1_500.0), vec![Move(50.0, 60.0), RightClick(50.0, 60.0)]);
        assert!(t.tick(1_600.0).is_empty(), "une seule fois");
        assert_eq!(
            t.moved(1, 90.0, 60.0),
            vec![Hold(90.0, 60.0)],
            "après l'appui long, le doigt ne fait plus défiler : il tient la croix"
        );
        assert!(t.up(1, 90.0, 60.0, 1_700.0).is_empty(), "ni tap au lever");
        // un doigt qui glisse n'est pas un appui long
        t.down(1, 50.0, 60.0, 0.0);
        t.moved(1, 80.0, 60.0);
        assert!(t.tick(900.0).is_empty());
    }

    #[test]
    fn long_press_then_grab_drags_and_releases() {
        let mut t = Touches::default();
        t.down(1, 50.0, 60.0, 1_000.0);
        assert_eq!(t.tick(1_500.0), vec![Move(50.0, 60.0), RightClick(50.0, 60.0)]);
        t.grab();
        assert_eq!(t.moved(1, 50.0, 90.0), vec![Move(50.0, 90.0)], "held: the finger drags");
        assert_eq!(t.moved(1, 50.0, 240.0), vec![Move(50.0, 240.0)]);
        assert_eq!(
            t.up(1, 50.0, 240.0, 1_600.0),
            vec![Move(50.0, 240.0), Release(50.0, 240.0)],
            "lifted: let go, never a swipe down"
        );
        assert!(!t.waiting());
    }

    #[test]
    fn pincer_a_deux_doigts() {
        let mut t = Touches::default();
        t.down(1, 100.0, 200.0, 0.0);
        assert!(t.down(2, 200.0, 200.0, 10.0).is_empty());
        // on écarte : l'écart horizontal passe de 100 à 150 (× 1,5)
        let g = t.moved(2, 250.0, 200.0);
        assert_eq!(g.len(), 1);
        let Pinch { cx, sx, sy, .. } = g[0] else { panic!("{g:?}") };
        assert!(
            (cx - 175.0).abs() < 1e-9 && (sx - 1.5).abs() < 1e-9 && sy == 1.0,
            "vertical trop petit : pas de zoom"
        );
        // un doigt se lève : pas de tap ni de glisser parasite
        assert!(t.up(2, 250.0, 200.0, 300.0).is_empty());
        assert!(t.moved(1, 300.0, 200.0).is_empty());
        assert!(t.up(1, 300.0, 200.0, 350.0).is_empty());
        // pendant un glisser, le deuxième doigt relâche le bouton d'abord
        t.down(1, 100.0, 100.0, 0.0);
        t.moved(1, 140.0, 100.0);
        assert_eq!(t.down(2, 300.0, 100.0, 50.0), vec![Release(140.0, 100.0)]);
        assert!(!t.waiting());
    }

    #[test]
    fn coup_de_doigt_ferme_la_page() {
        let mut t = Touches::default();
        t.down(1, 40.0, 300.0, 0.0);
        t.moved(1, 160.0, 310.0);
        assert_eq!(
            t.up(1, 160.0, 310.0, 250.0).last(),
            Some(&Swipe {
                x: 40.0,
                y: 300.0,
                down: false
            }),
            "vers la droite"
        );
        t.down(1, 200.0, 100.0, 0.0);
        t.moved(1, 205.0, 260.0);
        assert_eq!(
            t.up(1, 205.0, 260.0, 300.0).last(),
            Some(&Swipe {
                x: 200.0,
                y: 100.0,
                down: true
            }),
            "vers le bas"
        );
        t.down(1, 200.0, 300.0, 0.0);
        t.moved(1, 60.0, 300.0);
        assert!(!matches!(t.up(1, 60.0, 300.0, 200.0).last(), Some(Swipe { .. })), "vers la gauche : non");
        assert_eq!(t.cancel(), vec![]);
    }
}
