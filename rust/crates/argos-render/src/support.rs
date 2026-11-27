//! 🛟 Support: the « Support · Aide et retours » panel (PC, in the floating right panel). Two tabs:
//! « Nouveau billet » (type 🐞 / 💡 / ❓, subject, description, chart screenshot or attached image, ✎ circle, send)
//! and « Mes billets N » (each ticket, its state as clickable pills, Argos replies and yours, a field
//! to reply). The ticket format lives in `argos_core::retours`; the app sends and reloads them.

use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Graisse, Text};
use argos_core::retours::{Billet, Etat, Genre};
use tiny_skia::{Paint, PathBuilder, Pixmap, PixmapPaint, Stroke, Transform};

/// Inner margin of the panel.
const X: f64 = 20.0;
/// Top of the scrolling content (below the title and the tabs).
const HAUT: f64 = 112.0;
/// Height of the Description field and of the reply field.
const H_DESC: f64 = 120.0;
const H_REP: f64 = 70.0;
/// The screenshot sent: at most ~700 KB (a server request body is limited to 1 MiB, base64 included).
pub const ENVOI_MAX: usize = 700_000;
/// Never scaled down below this width (the chart text stays readable).
pub const LARGEUR_MIN: u32 = 1280;
/// 💡 ✕ on the « Qu'est-ce qu'on construit ensuite ? » card hides it this long (seconds), then it comes back.
pub const IDEE_CACHEE_S: i64 = 30 * 86_400;
/// 💡 The subject of an idea: the start of its text, at most this many characters (cut at a word).
pub const IDEE_OBJET: usize = 60;
/// Touch target (and the idea card's field height): 44 at the PC, the finger's minimum by the finger (44 real px).
fn cible() -> f64 {
    if crate::actions::tactile() {
        crate::formfactor::TOUCH_MIN_PT
    } else {
        44.0
    }
}

/// 💡 From the idea card's top to its field: the ✕ target, then a gap (52 at the PC).
fn idee_haut() -> f64 {
    cible() + 8.0
}

/// Open tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Onglet {
    #[default]
    Nouveau,
    Mes,
}

/// Text field that receives the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Champ {
    Objet,
    Description,
    /// Reply to the open ticket.
    Reponse,
    /// 💡 « Ton idée » (the card at the top of « Nouveau billet »).
    Idee,
}

/// The ticket image: its bytes (PNG or JPEG), its name, its preview (PNG only) and the circled ellipses
/// (image coordinates).
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    pub octets: Vec<u8>,
    pub ext: &'static str,
    pub nom: String,
    pub apercu: Option<Pixmap>,
    pub cercles: Vec<Rect>,
}

impl Image {
    /// An image from its bytes (PNG / JPEG signature); `None` for anything else.
    pub fn new(octets: Vec<u8>, nom: &str) -> Option<Image> {
        let ext = argos_core::retours::genre_image(&octets)?;
        let apercu = (ext == "png").then(|| Pixmap::decode_png(&octets).ok()).flatten();
        Some(Image {
            octets,
            ext,
            nom: nom.into(),
            apercu,
            cercles: vec![],
        })
    }

    /// The bytes to send and their extension: the ellipses drawn into the image, then scaled down (never below
    /// `LARGEUR_MIN`) to fit under `ENVOI_MAX`; `Err` if it is still too big.
    pub fn a_envoyer(&self) -> Result<(Vec<u8>, &'static str), String> {
        let Some(pm) = &self.apercu else {
            // JPEG (no preview): as is
            return if self.octets.len() <= ENVOI_MAX {
                Ok((self.octets.clone(), self.ext))
            } else {
                Err(format!("image trop grosse (au plus {} Ko)", ENVOI_MAX / 1000))
            };
        };
        let mut pm = pm.clone();
        entourer(&mut pm, &self.cercles, 1.0);
        let mut png = if self.cercles.is_empty() {
            self.octets.clone()
        } else {
            pm.encode_png().map_err(|e| e.to_string())?
        };
        let mut w = pm.width();
        while png.len() > ENVOI_MAX && w > LARGEUR_MIN {
            w = ((w as f64 * 0.8) as u32).max(LARGEUR_MIN);
            png = reduire(&pm, w).ok_or("réduction impossible")?.encode_png().map_err(|e| e.to_string())?;
        }
        if png.len() > ENVOI_MAX {
            return Err(format!(
                "capture trop grosse même réduite ({} Ko, au plus {} Ko)",
                png.len() / 1000,
                ENVOI_MAX / 1000
            ));
        }
        Ok((png, "png"))
    }
}

/// The image scaled down to width `w`.
fn reduire(pm: &Pixmap, w: u32) -> Option<Pixmap> {
    let k = w as f32 / pm.width() as f32;
    let mut out = Pixmap::new(w, ((pm.height() as f32 * k).round() as u32).max(1))?;
    let pp = PixmapPaint {
        quality: tiny_skia::FilterQuality::Bicubic,
        ..Default::default()
    };
    out.draw_pixmap(0, 0, pm.as_ref(), &pp, Transform::from_scale(k, k), None);
    Some(out)
}

/// Draws the (red) ellipses into `pm`; `k`: scale from the image to `pm`.
fn entourer(pm: &mut Pixmap, cercles: &[Rect], k: f64) {
    let mut p = Paint::default();
    p.set_color_rgba8(0xea, 0x39, 0x43, 0xff);
    p.anti_alias = true;
    let s = Stroke {
        width: (4.0 * k.max(0.4)) as f32,
        ..Default::default()
    };
    for c in cercles {
        let r = tiny_skia::Rect::from_xywh((c.x * k) as f32, (c.y * k) as f32, (c.w * k).max(2.0) as f32, (c.h * k).max(2.0) as f32);
        if let Some(path) = r.and_then(PathBuilder::from_oval) {
            pm.stroke_path(&path, &p, &s, Transform::identity(), None);
        }
    }
}

/// What a click asks of the app.
#[derive(Debug, Clone, PartialEq)]
pub enum SupClick {
    /// Nothing for the app (an internal choice may have changed: redraw).
    None,
    Fermer,
    Agrandir,
    /// 📷 Capture the chart.
    Capturer,
    /// 📎 Attach a file (PNG / JPEG).
    Joindre,
    /// Send the ticket from the form.
    Envoyer,
    /// The « Mes billets » tab opens: reload the tickets.
    Relire,
    /// The ticket (id) is opened: its replies are seen.
    Lu(String),
    /// Change the state of the ticket (id).
    Etat(String, Etat),
    /// Send the typed reply to the ticket (id).
    Repondre(String),
    /// 💡 Send the idea of the card as a 💡 Idée ticket ([`Support::idee_billet`]).
    EnvoyerIdee,
    /// 💡 ✕ of the card: hidden until [`Support::idee_cachee`] (the app keeps the date).
    CacherIdee,
}

/// An item laid out in the panel (panel coordinates).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Zone {
    Fermer,
    Agrandir,
    Onglet(Onglet),
    Genre(Genre),
    Champ(Champ),
    Capturer,
    Joindre,
    Apercu,
    Retirer,
    Entourer,
    Envoyer,
    /// The header of ticket k (expand / collapse).
    Billet(usize),
    Etat(usize, Etat),
    Repondre(usize),
    /// 💡 The idea card: ✕ and ↑.
    IdeeFermer,
    IdeeEnvoyer,
}

/// State of the Support panel.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Support {
    /// Position of the panel (window).
    pub cadre: Rect,
    pub defile: f64,
    pub onglet: Onglet,
    pub genre: Genre,
    pub objet: String,
    pub description: String,
    pub focus: Option<Champ>,
    pub image: Option<Image>,
    /// ✎ Circle: dragging over the preview draws an ellipse.
    pub entourer: bool,
    /// Ellipse being drawn (start, current point; panel coordinates).
    trace: Option<((f64, f64), (f64, f64))>,
    /// My tickets (newest first).
    pub billets: Vec<Billet>,
    /// The expanded ticket (id).
    pub ouvert: Option<String>,
    pub reponse: String,
    /// Message below the form (✅ sent, ⚠️ error).
    pub message: Option<String>,
    /// A send is in progress (the button waits).
    pub envoi: bool,
    /// 💡 The idea card: its text, hidden until (seconds; `None`: shown), the time now (seconds, given by the app),
    /// its message after a send, and the send in progress is the card's.
    pub idee: String,
    pub idee_cachee: Option<i64>,
    pub maintenant: i64,
    pub idee_message: Option<String>,
    pub idee_envoi: bool,
}

/// The displayed text of a field, with the `|` cursor at the end when it has the keyboard.
fn avec_curseur(s: &str, on: bool) -> String {
    if on {
        format!("{s}|")
    } else {
        s.to_string()
    }
}

impl Support {
    /// 🖥 In the floating panel, at its position `v` (window).
    pub fn dans_volet(v: Rect) -> Self {
        Self {
            cadre: v,
            ..Default::default()
        }
    }

    pub fn rect(&self) -> Rect {
        self.cadre
    }

    /// Argos replies not seen yet (red badge on the Support button).
    pub fn non_lues(&self) -> usize {
        self.billets.iter().map(Billet::non_lues).sum()
    }

    /// 💡 The « Qu'est-ce qu'on construit ensuite ? » card is shown (not hidden by ✕ in the last 30 days).
    pub fn idee_visible(&self) -> bool {
        self.idee_cachee.is_none_or(|t| self.maintenant >= t)
    }

    /// 💡 The ticket of the typed idea: (subject = the start of the text, at most 60 characters cut at a word,
    /// description = all the text); `None` when empty.
    pub fn idee_billet(&self) -> Option<(String, String)> {
        let tout = self.idee.trim();
        if tout.is_empty() {
            return None;
        }
        let ligne = tout.split_whitespace().collect::<Vec<_>>().join(" ");
        let objet = if ligne.chars().count() <= IDEE_OBJET {
            ligne
        } else {
            let debut: String = ligne.chars().take(IDEE_OBJET + 1).collect();
            match debut.rfind(' ') {
                Some(k) if k > 0 => debut[..k].to_string(),
                _ => ligne.chars().take(IDEE_OBJET).collect(),
            }
        };
        Some((objet, tout.to_string()))
    }

    /// 💡 The idea went out as ticket `numero`: the field empties and the card thanks.
    pub fn idee_envoyee(&mut self, numero: u32) {
        (self.idee, self.focus, self.idee_envoi) = (String::new(), None, false);
        self.idee_message = Some(format!("💡 Idée envoyée (billet n° {numero}). Merci !"));
    }

    /// 💡 Lines of the idea field (one, or two when the text is long).
    fn idee_lignes(&self, text: &Text, fw: f64) -> Vec<String> {
        let s = avec_curseur(&self.idee, self.focus == Some(Champ::Idee));
        let mut l = crate::assistant::wrap(text, &s, 14.0, fw);
        if l.len() > 2 {
            // (the end of the text stays visible: its last two lines)
            l = l.split_off(l.len() - 2);
        }
        l
    }

    /// Laid-out items (panel coordinates), and the height of all the content.
    fn places(&self, text: &Text) -> (Vec<(Zone, Rect)>, f64) {
        let w = self.cadre.w;
        let iw = w - 2.0 * X;
        let mut out = vec![
            (Zone::Fermer, Rect::new(w - 44.0, 14.0, 30.0, 30.0)),
            (Zone::Agrandir, Rect::new(w - 78.0, 14.0, 30.0, 30.0)),
        ];
        let ow = (iw - 8.0) / 2.0;
        out.push((Zone::Onglet(Onglet::Nouveau), Rect::new(X, 68.0, ow, 30.0)));
        out.push((Zone::Onglet(Onglet::Mes), Rect::new(X + ow + 8.0, 68.0, ow, 30.0)));
        let mut y = HAUT - self.defile;
        match self.onglet {
            Onglet::Nouveau => {
                // 💡 the idea card: logo, title, ✕; the field « Ton idée » and ↑ (44-point targets)
                if self.idee_visible() {
                    let fw = iw - 24.0 - cible() - 20.0;
                    let n = if self.idee.is_empty() { 1 } else { self.idee_lignes(text, fw).len() };
                    let fh = if n > 1 { cible() + 20.0 } else { cible() };
                    let champ = Rect::new(X + 12.0, y + idee_haut(), iw - 24.0, fh);
                    out.push((Zone::Champ(Champ::Idee), champ));
                    out.push((Zone::IdeeFermer, Rect::new(X + iw - cible() - 4.0, y + 4.0, cible(), cible())));
                    out.push((
                        Zone::IdeeEnvoyer,
                        Rect::new(champ.x + champ.w - cible(), champ.y + (fh - cible()) / 2.0, cible(), cible()),
                    ));
                    y += idee_haut() + fh + if self.idee_message.is_some() { 30.0 } else { 12.0 } + 16.0;
                }
                y += 20.0; // « Type »
                let cw = (iw - 16.0) / 3.0;
                for (k, g) in Genre::TOUS.iter().enumerate() {
                    out.push((Zone::Genre(*g), Rect::new(X + k as f64 * (cw + 8.0), y, cw, 62.0)));
                }
                y += 62.0 + 16.0 + 20.0; // « Objet »
                out.push((Zone::Champ(Champ::Objet), Rect::new(X, y, iw, 38.0)));
                y += 38.0 + 14.0 + 20.0; // « Description détaillée »
                out.push((Zone::Champ(Champ::Description), Rect::new(X, y, iw, H_DESC)));
                y += H_DESC + 14.0;
                let bw = (iw - 8.0) / 2.0;
                out.push((Zone::Capturer, Rect::new(X, y, bw, 36.0)));
                out.push((Zone::Joindre, Rect::new(X + bw + 8.0, y, bw, 36.0)));
                y += 36.0 + 24.0; // (« ou Ctrl+V pour coller une image »)
                if let Some(im) = &self.image {
                    let h = im
                        .apercu
                        .as_ref()
                        .map_or(56.0, |p| (iw * p.height() as f64 / p.width().max(1) as f64).min(220.0));
                    let a = Rect::new(X, y, iw, h);
                    out.push((Zone::Apercu, a));
                    out.push((Zone::Retirer, Rect::new(a.x + a.w - 34.0, a.y + 6.0, 28.0, 28.0)));
                    if im.apercu.is_some() {
                        out.push((Zone::Entourer, Rect::new(a.x + 6.0, a.y + a.h - 34.0, 104.0, 28.0)));
                    }
                    y += h + 14.0;
                }
                if self.message.is_some() {
                    y += 24.0;
                }
                out.push((Zone::Envoyer, Rect::new(X, y, iw, 44.0)));
                y += 44.0 + 20.0;
            }
            Onglet::Mes => {
                for (k, b) in self.billets.iter().enumerate() {
                    let haut = y;
                    out.push((Zone::Billet(k), Rect::new(X, y, iw, 52.0)));
                    y += 52.0;
                    let pw = (iw - 24.0 - 12.0) / 3.0;
                    for (i, e) in Etat::TOUS.iter().enumerate() {
                        out.push((Zone::Etat(k, *e), Rect::new(X + 12.0 + i as f64 * (pw + 6.0), y, pw, 24.0)));
                    }
                    y += 24.0 + 10.0;
                    if self.ouvert.as_deref() == Some(b.id.as_str()) {
                        y += crate::assistant::wrap(text, &b.description, 13.0, iw - 24.0).len() as f64 * 18.0 + 8.0;
                        for r in &b.reponses {
                            y += crate::assistant::wrap(text, &r.texte, 13.0, iw * 0.78 - 20.0).len() as f64 * 18.0 + 30.0 + 8.0;
                        }
                        out.push((Zone::Champ(Champ::Reponse), Rect::new(X + 12.0, y, iw - 24.0, H_REP)));
                        y += H_REP + 8.0;
                        out.push((Zone::Repondre(k), Rect::new(X + iw - 12.0 - 110.0, y, 110.0, 32.0)));
                        y += 32.0 + 10.0;
                    }
                    // (the ticket card: from its top to here)
                    let _ = haut;
                    y += 10.0;
                }
                if self.billets.is_empty() {
                    y += 40.0;
                }
            }
        }
        let fin = y + self.defile + crate::aibtn::FIN_DE_LISTE;
        (out, fin)
    }

    /// The card of ticket k (from its header to its last line), panel coordinates.
    fn carte(&self, zones: &[(Zone, Rect)], k: usize) -> Option<Rect> {
        let haut = zones.iter().find(|(z, _)| *z == Zone::Billet(k))?.1;
        let bas = zones
            .iter()
            .filter(|(z, _)| matches!(z, Zone::Etat(i, _) | Zone::Repondre(i) if *i == k) || (*z == Zone::Champ(Champ::Reponse) && self.ouvert_k() == Some(k)))
            .map(|(_, r)| r.y + r.h)
            .fold(haut.y + haut.h, f64::max);
        Some(Rect::new(haut.x, haut.y, haut.w, bas - haut.y + 10.0))
    }

    fn ouvert_k(&self) -> Option<usize> {
        let id = self.ouvert.as_deref()?;
        self.billets.iter().position(|b| b.id == id)
    }

    pub fn wheel(&mut self, dy: f64) {
        let (_, fin) = self.places(&Text::default());
        let max = (fin - self.cadre.h).max(0.0);
        self.defile = (self.defile + dy).clamp(0.0, max);
    }

    /// The zone under (x, y) (app window); scrolling zones only below the tabs.
    fn zone(&self, x: f64, y: f64) -> Option<(Zone, Rect)> {
        let r = self.cadre;
        if !r.contains(x, y) {
            return None;
        }
        let (lx, ly) = (x - r.x, y - r.y);
        let (zones, _) = self.places(&Text::default());
        // (the last one laid out is on top: the preview's ✕ and ✎ before the preview)
        zones.into_iter().rev().find(|(z, q)| {
            let fixe = matches!(z, Zone::Fermer | Zone::Agrandir | Zone::Onglet(_));
            q.contains(lx, ly) && (fixe || ly >= HAUT - 4.0)
        })
    }

    /// Press (app window): starts an ellipse on the preview in ✎ Circle mode; `true` if so.
    pub fn appui(&mut self, x: f64, y: f64) -> bool {
        if !self.entourer {
            return false;
        }
        if let Some((Zone::Apercu, _)) = self.zone(x, y) {
            let p = (x - self.cadre.x, y - self.cadre.y);
            self.trace = Some((p, p));
            return true;
        }
        false
    }

    /// An ellipse is being drawn.
    pub fn trace_en_cours(&self) -> bool {
        self.trace.is_some()
    }

    pub fn glisse(&mut self, x: f64, y: f64) {
        if let Some((a, _)) = self.trace {
            self.trace = Some((a, (x - self.cadre.x, y - self.cadre.y)));
        }
    }

    /// End of the drawing: the ellipse goes into the image (image coordinates) if it has a size.
    pub fn lache(&mut self) {
        let Some(((x0, y0), (x1, y1))) = self.trace.take() else { return };
        let (zones, _) = self.places(&Text::default());
        let Some((_, a)) = zones.into_iter().find(|(z, _)| *z == Zone::Apercu) else {
            return;
        };
        let Some(im) = self.image.as_mut() else { return };
        let Some(pm) = &im.apercu else { return };
        let k = pm.width() as f64 / a.w;
        let (l, t) = (x0.min(x1).max(a.x), y0.min(y1).max(a.y));
        let (r, b) = (x0.max(x1).min(a.x + a.w), y0.max(y1).min(a.y + a.h));
        if r - l >= 6.0 && b - t >= 6.0 {
            im.cercles.push(Rect::new((l - a.x) * k, (t - a.y) * k, (r - l) * k, (b - t) * k));
        }
    }

    /// Click (app window): internal choices change here; what the app must do is returned.
    pub fn click(&mut self, x: f64, y: f64) -> SupClick {
        let Some((z, _)) = self.zone(x, y) else {
            // (in the panel, outside a field: the keyboard leaves it)
            self.focus = None;
            return SupClick::None;
        };
        if !matches!(z, Zone::Champ(_)) {
            self.focus = None;
        }
        match z {
            Zone::Fermer => SupClick::Fermer,
            Zone::Agrandir => SupClick::Agrandir,
            Zone::Onglet(o) => {
                let avant = self.onglet;
                (self.onglet, self.defile) = (o, 0.0);
                if o == Onglet::Mes && avant != o {
                    SupClick::Relire
                } else {
                    SupClick::None
                }
            }
            Zone::Genre(g) => {
                self.genre = g;
                SupClick::None
            }
            Zone::Champ(c) => {
                self.focus = Some(c);
                SupClick::None
            }
            Zone::Capturer => SupClick::Capturer,
            Zone::Joindre => SupClick::Joindre,
            Zone::Apercu => SupClick::None,
            Zone::Retirer => {
                (self.image, self.entourer) = (None, false);
                SupClick::None
            }
            Zone::Entourer => {
                self.entourer = !self.entourer;
                SupClick::None
            }
            Zone::Envoyer => {
                if self.envoi {
                    SupClick::None
                } else if self.objet.trim().is_empty() {
                    self.message = Some("⚠️ Écris un objet (une ligne qui résume).".into());
                    self.focus = Some(Champ::Objet);
                    SupClick::None
                } else {
                    SupClick::Envoyer
                }
            }
            Zone::Billet(k) => {
                let id = self.billets[k].id.clone();
                if self.ouvert.as_deref() == Some(id.as_str()) {
                    self.ouvert = None;
                    SupClick::None
                } else {
                    (self.ouvert, self.reponse) = (Some(id.clone()), String::new());
                    if self.billets[k].non_lues() > 0 {
                        SupClick::Lu(id)
                    } else {
                        SupClick::None
                    }
                }
            }
            Zone::Etat(k, e) => SupClick::Etat(self.billets[k].id.clone(), e),
            Zone::Repondre(k) => {
                if self.reponse.trim().is_empty() {
                    self.focus = Some(Champ::Reponse);
                    SupClick::None
                } else {
                    SupClick::Repondre(self.billets[k].id.clone())
                }
            }
            Zone::IdeeFermer => {
                (self.idee_cachee, self.focus) = (Some(self.maintenant + IDEE_CACHEE_S), None);
                SupClick::CacherIdee
            }
            Zone::IdeeEnvoyer => self.idee_envoyer(),
        }
    }

    /// 💡 ↑ or Enter in the idea field: the app sends it (empty: nothing, the field keeps the keyboard).
    fn idee_envoyer(&mut self) -> SupClick {
        if self.envoi || self.idee_billet().is_none() {
            self.focus = Some(Champ::Idee);
            return SupClick::None;
        }
        self.idee_message = None;
        SupClick::EnvoyerIdee
    }

    /// The field that has the keyboard.
    fn champ_mut(&mut self) -> Option<&mut String> {
        Some(match self.focus? {
            Champ::Objet => &mut self.objet,
            Champ::Description => &mut self.description,
            Champ::Reponse => &mut self.reponse,
            Champ::Idee => &mut self.idee,
        })
    }

    /// Text typed or pasted into the field that has the keyboard; the subject stays on one line and within its length.
    pub fn taper(&mut self, s: &str) {
        let objet = self.focus == Some(Champ::Objet);
        // (the subject and the idea: one line; the subject within its length)
        let une_ligne = matches!(self.focus, Some(Champ::Objet | Champ::Idee));
        let Some(c) = self.champ_mut() else { return };
        for ch in s.chars() {
            if (une_ligne && ch == '\n') || (objet && c.chars().count() >= argos_core::retours::OBJET_MAX) {
                continue;
            }
            if c.chars().count() < argos_core::retours::TEXTE_MAX && (!ch.is_control() || ch == '\n') {
                c.push(ch);
            }
        }
    }

    pub fn effacer(&mut self) {
        if let Some(c) = self.champ_mut() {
            c.pop();
        }
    }

    /// Enter: in the subject, moves to the description; in the description, a new line; in the reply,
    /// sends it (`Some`).
    pub fn entree(&mut self) -> SupClick {
        match self.focus {
            Some(Champ::Objet) => {
                self.focus = Some(Champ::Description);
                SupClick::None
            }
            Some(Champ::Description) => {
                self.taper("\n");
                SupClick::None
            }
            Some(Champ::Idee) => self.idee_envoyer(),
            Some(Champ::Reponse) => match self.ouvert.clone() {
                Some(id) if !self.reponse.trim().is_empty() => SupClick::Repondre(id),
                _ => SupClick::None,
            },
            None => SupClick::None,
        }
    }

    /// The laid-out text fields (app window): phone keyboard in the page.
    pub fn champs(&self) -> Vec<Rect> {
        let r = self.cadre;
        self.places(&Text::default())
            .0
            .into_iter()
            .filter(|(z, q)| matches!(z, Zone::Champ(_)) && q.y >= HAUT - 4.0)
            .map(|(_, q)| Rect::new(r.x + q.x, r.y + q.y, q.w, q.h))
            .collect()
    }

    /// The form is reset after a successful send.
    pub fn envoye(&mut self, numero: u32) {
        let genre = self.genre;
        *self = Self {
            cadre: self.cadre,
            genre,
            billets: std::mem::take(&mut self.billets),
            message: Some(format!("✅ Billet #{numero} envoyé. Tu le retrouves dans « Mes billets ».")),
            ..Default::default()
        };
    }

    /// Image of the panel and its position; `mouse`: app window.
    pub fn draw(&self, text: &Text, mouse: Option<(f64, f64)>) -> Option<(Pixmap, Rect)> {
        use crate::jetons as j;
        let r = self.cadre;
        let mut pm = crate::px::new(r.w.ceil() as u32, r.h.ceil() as u32)?;
        let carte = Rect::new(0.5, 0.5, r.w - 1.0, r.h - 1.0);
        round_fill(&mut pm, carte, j::R_VOLET, j::sur_fond(j::VERRE), 1.0);
        round_stroke(&mut pm, carte, j::R_VOLET, j::sur_fond(j::LISERE), 1.0);
        let survol = |q: &Rect| mouse.is_some_and(|(x, y)| q.contains(x - r.x, y - r.y));
        let gras = Text::graisse(Graisse::Bold);
        gras.draw(&mut pm, "🛟 Support", ((X + 4.0) as f32, 36.0), 17.0, j::TEXTE, Align::Left);
        text.draw(&mut pm, "Aide et retours", ((X + 4.0) as f32, 56.0), 12.0, j::DIM, Align::Left);
        let (zones, _) = self.places(text);
        let iw = r.w - 2.0 * X;
        // scrolling content: drawn into its own image, clipped below the tabs
        let mut c = crate::px::new(r.w.ceil() as u32, r.h.ceil() as u32)?;
        let label = |c: &mut Pixmap, s: &str, q: &Rect| text.draw(c, s, (q.x as f32, (q.y - 7.0) as f32), 12.0, j::DIM, Align::Left);
        // the ticket cards first (under their buttons)
        if self.onglet == Onglet::Mes {
            for k in 0..self.billets.len() {
                if let Some(q) = self.carte(&zones, k) {
                    round_fill(&mut c, q, j::R_LIGNE, j::CARTE.0, j::CARTE.1 + 0.02);
                }
            }
            if self.billets.is_empty() {
                text.draw(
                    &mut c,
                    "Aucun billet pour l'instant.",
                    (X as f32, (HAUT + 24.0 - self.defile) as f32),
                    13.0,
                    j::DIM,
                    Align::Left,
                );
            }
        }
        // 💡 the idea card (under its field, ✕ and ↑): glass, Argos AI logo, title
        if let Some((_, q)) = zones.iter().find(|(z, _)| *z == Zone::Champ(Champ::Idee)) {
            let carte = Rect::new(
                X,
                q.y - idee_haut(),
                iw,
                q.h + idee_haut() + if self.idee_message.is_some() { 30.0 } else { 12.0 },
            );
            round_fill(&mut c, carte, j::R_LIGNE, j::CARTE.0, j::CARTE.1 + 0.02);
            round_stroke(&mut c, carte, j::R_LIGNE, j::sur_fond(j::LISERE), 1.0);
            crate::aibtn::logo(&mut c, carte.x + 14.0, carte.y + 15.0, 22.0);
            let t = gras.fit("Qu'est-ce qu'on construit ensuite ?", 14.0, iw - 44.0 - 60.0);
            gras.draw(&mut c, &t, ((carte.x + 44.0) as f32, (carte.y + 31.0) as f32), 14.0, j::TEXTE, Align::Left);
        }
        for (z, q) in &zones {
            let h = survol(q);
            match *z {
                Zone::Fermer => {
                    if h {
                        round_fill(&mut pm, *q, q.h / 2.0, j::VERRE2.0, j::VERRE2.1);
                    }
                    text.draw(&mut pm, "✕", (q.cx() as f32, (q.cy() + 5.0) as f32), 14.0, j::DIM, Align::Center);
                }
                Zone::Agrandir => {
                    if h {
                        round_fill(&mut pm, *q, q.h / 2.0, j::VERRE2.0, j::VERRE2.1);
                    }
                    crate::shapes::lines(
                        &mut pm,
                        q.cx(),
                        q.cy(),
                        &[
                            &[(-5.0, 5.0), (5.0, -5.0)],
                            &[(1.0, -5.0), (5.0, -5.0), (5.0, -1.0)],
                            &[(-5.0, 1.0), (-5.0, 5.0), (-1.0, 5.0)],
                        ],
                        j::DIM,
                        1.3,
                    );
                }
                Zone::Onglet(o) => {
                    crate::shapes::pilule(&mut pm, *q, self.onglet == o, h);
                    let s = match o {
                        Onglet::Nouveau => "Nouveau billet".to_string(),
                        Onglet::Mes => format!("Mes billets {}", self.billets.len()),
                    };
                    text.draw(&mut pm, &s, (q.cx() as f32, (q.cy() + 4.5) as f32), 13.0, j::TEXTE, Align::Center);
                    if o == Onglet::Mes && self.non_lues() > 0 {
                        round_fill(&mut pm, Rect::new(q.x + q.w - 16.0, q.y + 5.0, 8.0, 8.0), 4.0, j::BAS, 1.0);
                    }
                }
                Zone::Genre(g) => {
                    let on = self.genre == g;
                    round_fill(
                        &mut c,
                        *q,
                        j::R_LIGNE,
                        if on { j::ACCENT } else { j::BLANC },
                        if on {
                            0.18
                        } else if h {
                            0.08
                        } else {
                            0.05
                        },
                    );
                    if on {
                        round_stroke(&mut c, *q, j::R_LIGNE, j::ACCENT, 1.5);
                    }
                    if g == Genre::Probleme {
                        label(&mut c, "Type", q);
                    }
                    let (emoji, nom) = g.nom().split_once(' ').unwrap_or(("", g.nom()));
                    text.draw(&mut c, emoji, (q.cx() as f32, (q.y + 26.0) as f32), 18.0, j::TEXTE, Align::Center);
                    let nom = text.fit(nom, 12.0, q.w - 8.0);
                    text.draw(&mut c, &nom, (q.cx() as f32, (q.y + 49.0) as f32), 12.0, j::TEXTE, Align::Center);
                }
                Zone::Champ(ch) => {
                    let on = self.focus == Some(ch);
                    round_fill(&mut c, *q, j::R_BOUTON, j::CHAMP.0, j::CHAMP.1);
                    round_stroke(&mut c, *q, j::R_BOUTON, if on { j::ACCENT } else { j::sur_fond(j::LISERE) }, 1.0);
                    let (val, vide, titre) = match ch {
                        Champ::Objet => (&self.objet, "Ex. : le bouton AI cache l'axe des dates", "Objet"),
                        Champ::Description => (
                            &self.description,
                            "Ce qui se passe, ce que tu attendais, comment le refaire…",
                            "Description détaillée",
                        ),
                        Champ::Reponse => (&self.reponse, "Ta réponse…", ""),
                        Champ::Idee => {
                            if let Some(m) = &self.idee_message {
                                let m = text.fit(m, 12.5, iw - 24.0);
                                text.draw(&mut c, &m, ((q.x + 2.0) as f32, (q.y + q.h + 20.0) as f32), 12.5, j::DIM, Align::Left);
                            }
                            (&self.idee, "Ton idée", "")
                        }
                    };
                    if !titre.is_empty() {
                        label(&mut c, titre, q);
                    }
                    if val.is_empty() && !on {
                        let v = text.fit(vide, 13.0, q.w - 20.0);
                        text.draw(&mut c, &v, ((q.x + 10.0) as f32, (q.y + 23.0) as f32), 13.0, j::DIM, Align::Left);
                    } else if ch == Champ::Idee {
                        // (one line, two when the text is long; room left for ↑)
                        for (i, l) in self.idee_lignes(text, q.w - 24.0 - cible()).iter().enumerate() {
                            text.draw(
                                &mut c,
                                l,
                                ((q.x + 12.0) as f32, (q.y + 27.0 + (cible() - 44.0) / 2.0 + i as f64 * 20.0) as f32),
                                14.0,
                                j::TEXTE,
                                Align::Left,
                            );
                        }
                    } else if ch == Champ::Objet {
                        // (one line: the end stays visible)
                        let mut s = avec_curseur(val, on);
                        while text.width(&s, 13.0) as f64 > q.w - 20.0 && !s.is_empty() {
                            s.remove(0);
                        }
                        text.draw(&mut c, &s, ((q.x + 10.0) as f32, (q.y + 24.0) as f32), 13.0, j::TEXTE, Align::Left);
                    } else {
                        let lignes = crate::assistant::wrap(text, &avec_curseur(val, on), 13.0, q.w - 20.0);
                        let tient = ((q.h - 12.0) / 18.0).floor().max(1.0) as usize;
                        let debut = lignes.len().saturating_sub(tient);
                        for (i, l) in lignes[debut..].iter().enumerate() {
                            text.draw(
                                &mut c,
                                l,
                                ((q.x + 10.0) as f32, (q.y + 20.0 + i as f64 * 18.0) as f32),
                                13.0,
                                j::TEXTE,
                                Align::Left,
                            );
                        }
                    }
                }
                Zone::Capturer | Zone::Joindre => {
                    round_fill(&mut c, *q, j::R_BOUTON, j::VERRE2.0, j::VERRE2.1 + if h { 0.05 } else { 0.0 });
                    let s = if *z == Zone::Capturer {
                        "📷 Capturer la chart"
                    } else {
                        "📎 Joindre un fichier"
                    };
                    let s = text.fit(s, 13.0, q.w - 10.0);
                    text.draw(&mut c, &s, (q.cx() as f32, (q.cy() + 4.5) as f32), 13.0, j::TEXTE, Align::Center);
                    if *z == Zone::Capturer {
                        // (📱 phone view: « PNG ou JPEG », no Ctrl+V)
                        let t = &crate::actions::sans_touches("PNG ou JPEG · ou Ctrl+V pour coller une image");
                        text.draw(&mut c, t, (q.x as f32, (q.y + q.h + 16.0) as f32), 11.0, j::DIM, Align::Left);
                    }
                }
                Zone::Apercu => {
                    let Some(im) = &self.image else { continue };
                    round_fill(&mut c, *q, j::R_BOUTON, j::FOND, 0.4);
                    if let Some(p) = &im.apercu {
                        let k = q.w / p.width().max(1) as f64;
                        let mut a = crate::px::new(q.w.ceil() as u32, q.h.ceil() as u32)?;
                        let pp = PixmapPaint {
                            quality: tiny_skia::FilterQuality::Bicubic,
                            ..Default::default()
                        };
                        a.draw_pixmap(0, 0, p.as_ref(), &pp, Transform::from_scale(k as f32, k as f32), None);
                        let mut cercles = im.cercles.clone();
                        if let Some(((x0, y0), (x1, y1))) = self.trace {
                            cercles.push(Rect::new(
                                (x0.min(x1) - q.x) / k,
                                (y0.min(y1) - q.y) / k,
                                (x1 - x0).abs() / k,
                                (y1 - y0).abs() / k,
                            ));
                        }
                        entourer(&mut a, &cercles, k);
                        c.draw_pixmap(q.x as i32, q.y as i32, a.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
                    } else {
                        let s = format!("📎 {} · JPEG (pas d'aperçu)", text.fit(&im.nom, 13.0, q.w - 160.0));
                        text.draw(&mut c, &s, ((q.x + 12.0) as f32, (q.cy() + 4.5) as f32), 13.0, j::TEXTE, Align::Left);
                    }
                }
                Zone::Retirer => {
                    round_fill(&mut c, *q, q.h / 2.0, j::FOND, if h { 0.8 } else { 0.6 });
                    text.draw(&mut c, "✕", (q.cx() as f32, (q.cy() + 5.0) as f32), 13.0, j::BLANC, Align::Center);
                }
                Zone::Entourer => {
                    round_fill(
                        &mut c,
                        *q,
                        q.h / 2.0,
                        if self.entourer { j::BAS } else { j::FOND },
                        if self.entourer { 0.9 } else { 0.6 },
                    );
                    text.draw(&mut c, "✎ Entourer", (q.cx() as f32, (q.cy() + 4.5) as f32), 12.5, j::BLANC, Align::Center);
                }
                Zone::IdeeFermer => {
                    if h {
                        round_fill(&mut c, *q, q.h / 2.0, j::VERRE2.0, j::VERRE2.1);
                    }
                    text.draw(&mut c, "✕", (q.cx() as f32, (q.cy() + 5.0) as f32), 14.0, j::DIM, Align::Center);
                }
                Zone::IdeeEnvoyer => {
                    // ↑ : grey while empty, blue once there is an idea
                    let pret = self.idee_billet().is_some();
                    let b = Rect::new(q.cx() - 17.0, q.cy() - 17.0, 34.0, 34.0);
                    round_fill(&mut c, b, 10.0, if pret { j::ACCENT } else { j::BLANC }, if pret { 1.0 } else { 0.12 });
                    let col = if pret { j::BLANC } else { j::DIM };
                    crate::shapes::lines(
                        &mut c,
                        b.cx(),
                        b.cy(),
                        &[&[(0.0, 7.0), (0.0, -7.0)], &[(-6.0, -1.0), (0.0, -7.0), (6.0, -1.0)]],
                        col,
                        2.0,
                    );
                }
                Zone::Envoyer => {
                    if let Some(m) = &self.message {
                        let col = if m.starts_with('⚠') { j::BAS } else { j::DIM };
                        let m = text.fit(m, 12.5, iw);
                        text.draw(&mut c, &m, (q.x as f32, (q.y - 10.0) as f32), 12.5, col, Align::Left);
                    }
                    round_fill(
                        &mut c,
                        *q,
                        j::R_BOUTON,
                        j::ACCENT,
                        if self.envoi {
                            0.45
                        } else if h {
                            1.0
                        } else {
                            0.88
                        },
                    );
                    let s = if self.envoi { "Envoi…" } else { "Envoyer mon billet" };
                    gras.draw(&mut c, s, (q.cx() as f32, (q.cy() + 5.0) as f32), 14.0, j::BLANC, Align::Center);
                }
                Zone::Billet(k) => {
                    let b = &self.billets[k];
                    if h {
                        round_fill(&mut c, *q, j::R_LIGNE, j::BLANC, 0.04);
                    }
                    let (emoji, _) = b.genre.nom().split_once(' ').unwrap_or(("", ""));
                    let titre = text.fit(&format!("#{} {emoji} {}", b.numero, b.objet), 13.5, q.w - 40.0);
                    gras.draw(&mut c, &titre, ((q.x + 12.0) as f32, (q.y + 22.0) as f32), 13.5, j::TEXTE, Align::Left);
                    let n = b.reponses.len();
                    let sous = format!(
                        "{} · {n} réponse{}",
                        b.date().get(..16).unwrap_or(b.date()).replace('T', " "),
                        if n > 1 { "s" } else { "" }
                    );
                    text.draw(&mut c, &sous, ((q.x + 12.0) as f32, (q.y + 41.0) as f32), 11.5, j::DIM, Align::Left);
                    let ouvert = self.ouvert.as_deref() == Some(b.id.as_str());
                    text.draw(
                        &mut c,
                        if ouvert { "⌃" } else { "⌄" },
                        ((q.x + q.w - 16.0) as f32, (q.y + 26.0) as f32),
                        14.0,
                        j::DIM,
                        Align::Center,
                    );
                    if b.non_lues() > 0 {
                        round_fill(&mut c, Rect::new(q.x + q.w - 34.0, q.y + 14.0, 9.0, 9.0), 4.5, j::BAS, 1.0);
                    }
                    if ouvert {
                        let mut y = q.y + q.h + 24.0 + 10.0;
                        for l in crate::assistant::wrap(text, &b.description, 13.0, iw - 24.0) {
                            text.draw(&mut c, &l, ((q.x + 12.0) as f32, (y + 12.0) as f32), 13.0, j::TEXTE, Align::Left);
                            y += 18.0;
                        }
                        y += 8.0;
                        let bw = iw * 0.78;
                        for rep in &b.reponses {
                            let argos = rep.auteur == "argos";
                            let lignes = crate::assistant::wrap(text, &rep.texte, 13.0, bw - 20.0);
                            let bh = lignes.len() as f64 * 18.0 + 30.0;
                            let bx = if argos { q.x + 12.0 } else { q.x + q.w - 12.0 - bw };
                            let bulle = Rect::new(bx, y, bw, bh);
                            round_fill(&mut c, bulle, 12.0, if argos { j::ACCENT } else { j::BLANC }, if argos { 0.16 } else { 0.07 });
                            let qui = if argos { "Argos" } else { "Toi" };
                            let quand = rep.date.get(..16).unwrap_or(&rep.date).replace('T', " ");
                            gras.draw(
                                &mut c,
                                qui,
                                ((bx + 10.0) as f32, (y + 17.0) as f32),
                                11.5,
                                if argos { j::CYAN } else { j::TEXTE },
                                Align::Left,
                            );
                            text.draw(&mut c, &quand, ((bx + bw - 10.0) as f32, (y + 17.0) as f32), 10.5, j::DIM, Align::Right);
                            for (i, l) in lignes.iter().enumerate() {
                                text.draw(
                                    &mut c,
                                    l,
                                    ((bx + 10.0) as f32, (y + 36.0 + i as f64 * 18.0) as f32),
                                    13.0,
                                    j::TEXTE,
                                    Align::Left,
                                );
                            }
                            y += bh + 8.0;
                        }
                    }
                }
                Zone::Etat(k, e) => {
                    let on = self.billets[k].etat == e;
                    let col = match e {
                        Etat::Ouvert => j::TIER2,
                        Etat::EnCours => j::ACCENT,
                        Etat::Resolu => crate::palette::up(),
                    };
                    round_fill(
                        &mut c,
                        *q,
                        q.h / 2.0,
                        col,
                        if on {
                            0.3
                        } else if h {
                            0.1
                        } else {
                            0.05
                        },
                    );
                    if on {
                        round_stroke(&mut c, *q, q.h / 2.0, col, 1.0);
                    }
                    text.draw(
                        &mut c,
                        e.nom(),
                        (q.cx() as f32, (q.cy() + 4.0) as f32),
                        11.5,
                        if on { j::TEXTE } else { j::DIM },
                        Align::Center,
                    );
                }
                Zone::Repondre(_) => {
                    round_fill(&mut c, *q, j::R_BOUTON, j::ACCENT, if h { 1.0 } else { 0.85 });
                    text.draw(&mut c, "Répondre", (q.cx() as f32, (q.cy() + 4.5) as f32), 13.0, j::BLANC, Align::Center);
                }
            }
        }
        // the content, clipped below the tabs and at the panel edge
        let haut = (HAUT - 6.0) as u32;
        if let Some(bas) = c.clone_rect(tiny_skia::IntRect::from_xywh(0, haut as i32, c.width(), c.height().saturating_sub(haut + 4))?) {
            pm.draw_pixmap(0, haut as i32, bas.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
        }
        Some((pm, r))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(w: u32, h: u32) -> Vec<u8> {
        let mut pm = Pixmap::new(w, h).unwrap();
        pm.fill(tiny_skia::Color::from_rgba8(20, 30, 40, 255));
        pm.encode_png().unwrap()
    }

    /// The center (window) of a zone.
    fn centre(s: &Support, z: Zone) -> (f64, f64) {
        let q = s
            .places(&Text::default())
            .0
            .into_iter()
            .find(|(x, _)| *x == z)
            .unwrap_or_else(|| panic!("{z:?}"))
            .1;
        (s.cadre.x + q.cx(), s.cadre.y + q.cy())
    }

    /// 💡 The « Qu'est-ce qu'on construit ensuite ? » card (Jee, 21 Nov): there at the top of « Nouveau billet »;
    /// Enter and ↑ send a 💡 Idée ticket (subject = start of the text cut at a word, description = all); empty: nothing;
    /// ✕ hides it 30 days then it comes back; 44-point targets.
    #[test]
    fn carte_idee() {
        let mut s = Support::dans_volet(Rect::new(1000.0, 66.0, 400.0, 800.0));
        s.maintenant = 1_800_000_000;
        assert!(s.idee_visible());
        let (zones, _) = s.places(&Text::default());
        let y_de = |z: Zone| zones.iter().find(|(x, _)| *x == z).map(|(_, q)| *q);
        let champ = y_de(Zone::Champ(Champ::Idee)).expect("la carte est là");
        assert!(champ.y < y_de(Zone::Genre(Genre::Probleme)).unwrap().y, "avant « Type »");
        for z in [Zone::Champ(Champ::Idee), Zone::IdeeEnvoyer, Zone::IdeeFermer] {
            let q = y_de(z).unwrap();
            assert!(q.w >= 44.0 && q.h >= 44.0, "{z:?} {q:?}");
        }
        assert_eq!(y_de(Zone::IdeeFermer).unwrap().h, 44.0, "PC: unchanged");
        // 👆 by the finger: 55-point targets (44 real px at 0.8), ✕ not over the field
        crate::actions::set_tactile(true);
        let (zd, _) = s.places(&Text::default());
        crate::actions::set_tactile(false);
        let d = |z: Zone| zd.iter().find(|(x, _)| *x == z).map(|(_, q)| *q).unwrap();
        for z in [Zone::Champ(Champ::Idee), Zone::IdeeEnvoyer, Zone::IdeeFermer] {
            let q = d(z);
            assert!(q.w >= crate::formfactor::TOUCH_MIN_PT && q.h >= crate::formfactor::TOUCH_MIN_PT, "{z:?} {q:?}");
        }
        assert!(
            d(Zone::IdeeFermer).y + d(Zone::IdeeFermer).h <= d(Zone::Champ(Champ::Idee)).y,
            "✕ above the field"
        );
        // empty: ↑ and Enter send nothing
        let (x, y) = centre(&s, Zone::IdeeEnvoyer);
        assert_eq!(s.click(x, y), SupClick::None);
        assert_eq!(s.focus, Some(Champ::Idee));
        assert_eq!(s.entree(), SupClick::None);
        assert_eq!(s.idee_billet(), None);
        // Enter sends
        s.taper("Un panneau de marchés au survol du symbole, avec les récents et les favoris\n");
        assert_eq!(s.entree(), SupClick::EnvoyerIdee);
        let (objet, desc) = s.idee_billet().unwrap();
        assert_eq!(objet, "Un panneau de marchés au survol du symbole, avec les récents");
        assert!(objet.chars().count() <= IDEE_OBJET);
        assert_eq!(desc, "Un panneau de marchés au survol du symbole, avec les récents et les favoris");
        // (two lines for a long text: the field grows)
        let (z2, _) = s.places(&Text::default());
        assert!(z2.iter().find(|(z, _)| *z == Zone::Champ(Champ::Idee)).unwrap().1.h > 44.0);
        // ↑ sends too; once sent: empty field and the thanks
        let (x, y) = centre(&s, Zone::IdeeEnvoyer);
        assert_eq!(s.click(x, y), SupClick::EnvoyerIdee);
        s.idee_envoyee(12);
        assert!(s.idee.is_empty());
        assert_eq!(s.idee_message.as_deref(), Some("💡 Idée envoyée (billet n° 12). Merci !"));
        assert!(s.draw(&Text::default(), None).is_some());
        // ✕: hidden 30 days, then back
        let (x, y) = centre(&s, Zone::IdeeFermer);
        assert_eq!(s.click(x, y), SupClick::CacherIdee);
        assert_eq!(s.idee_cachee, Some(1_800_000_000 + IDEE_CACHEE_S));
        assert!(!s.idee_visible());
        assert!(!s.places(&Text::default()).0.iter().any(|(z, _)| *z == Zone::Champ(Champ::Idee)));
        s.maintenant += 29 * 86_400;
        assert!(!s.idee_visible(), "29 jours plus tard : encore cachée");
        s.maintenant += 86_400;
        assert!(s.idee_visible(), "30 jours plus tard : elle revient");
        // a short text: the subject is the whole text
        s.idee = "Mode sombre  plus doux".into();
        assert_eq!(s.idee_billet().unwrap().0, "Mode sombre plus doux");
    }

    /// 🛟 The form: type, subject (one line, placeholder), description, screenshot, ✎ circle, send.
    #[test]
    fn formulaire_du_billet() {
        let mut s = Support::dans_volet(Rect::new(1000.0, 66.0, 400.0, 800.0));
        s.idee_cachee = Some(i64::MAX); // (the idea card has its own test)
        let (x, y) = centre(&s, Zone::Genre(Genre::Question));
        assert_eq!(s.click(x, y), SupClick::None);
        assert_eq!(s.genre, Genre::Question);
        // sending without a subject: rejected, the keyboard goes to the subject
        let (x, y) = centre(&s, Zone::Envoyer);
        assert_eq!(s.click(x, y), SupClick::None);
        assert_eq!(s.focus, Some(Champ::Objet));
        s.taper("Le bouton AI\ncache l'axe");
        assert_eq!(s.objet, "Le bouton AIcache l'axe", "l'objet reste sur une ligne");
        assert_eq!(s.entree(), SupClick::None);
        assert_eq!(s.focus, Some(Champ::Description));
        s.taper("ligne 1");
        s.entree();
        s.taper("ligne 2");
        s.effacer();
        assert_eq!(s.description, "ligne 1\nligne ");
        let (x, y) = centre(&s, Zone::Capturer);
        assert_eq!(s.click(x, y), SupClick::Capturer);
        let (x, y) = centre(&s, Zone::Joindre);
        assert_eq!(s.click(x, y), SupClick::Joindre);
        // a screenshot: preview, ✎ circle (drag = an ellipse), ✕ removes
        s.image = Image::new(png(1600, 900), "chart.png");
        let (x, y) = centre(&s, Zone::Entourer);
        s.click(x, y);
        assert!(s.entourer);
        let (ax, ay) = centre(&s, Zone::Apercu);
        assert!(s.appui(ax - 40.0, ay - 30.0));
        s.glisse(ax + 40.0, ay + 10.0);
        s.lache();
        let c = s.image.as_ref().unwrap().cercles[0];
        assert!((c.w - 80.0 * 1600.0 / 360.0).abs() < 1.0, "en coordonnées de l'image : {c:?}");
        let (o, ext) = s.image.as_ref().unwrap().a_envoyer().unwrap();
        assert_eq!(ext, "png");
        let relu = Pixmap::decode_png(&o).unwrap();
        assert_eq!(relu.width(), 1600);
        let (px, py) = ((c.x + c.w / 2.0) as u32, c.y as u32 + 1);
        assert!(relu.pixel(px, py).unwrap().red() > 150, "l'ellipse est dessinée dans l'image envoyée");
        let (x, y) = centre(&s, Zone::Envoyer);
        assert_eq!(s.click(x, y), SupClick::Envoyer);
        let (x, y) = centre(&s, Zone::Retirer);
        s.click(x, y);
        assert!(s.image.is_none());
        assert!(Image::new(b"GIF89a".to_vec(), "x.gif").is_none(), "seulement PNG ou JPEG");
        let (pm, _) = s.draw(&Text::default(), None).unwrap();
        assert_eq!(pm.pixel(1, 1).unwrap().alpha(), 0, "coins arrondis");
        s.envoye(4);
        assert!(s.objet.is_empty() && s.description.is_empty() && s.message.as_deref().unwrap().contains("#4"));
        assert_eq!(s.genre, Genre::Question);
    }

    /// Screenshot too big: scaled down (never below 1280 wide) until it fits; otherwise rejected.
    #[test]
    fn capture_reduite() {
        // noise does not compress: 2400 × 1400 makes several MB
        let mut pm = Pixmap::new(2400, 1400).unwrap();
        let mut a = argos_core::alea::Alea::new(1);
        for p in pm.pixels_mut() {
            let v = a.suivant();
            *p = tiny_skia::ColorU8::from_rgba(v as u8, (v >> 8) as u8, (v >> 16) as u8, 255).premultiply();
        }
        let im = Image::new(pm.encode_png().unwrap(), "bruit.png").unwrap();
        assert!(im.a_envoyer().is_err(), "trop grosse même à 1280");
        let im = Image::new(png(3000, 1600), "uni.png").unwrap();
        let (o, _) = im.a_envoyer().unwrap();
        assert!(o.len() <= ENVOI_MAX);
    }

    /// Mes billets: open (seen), state pills, reply.
    #[test]
    fn mes_billets() {
        use argos_core::retours::Nouveau;
        let mut s = Support::dans_volet(Rect::new(1000.0, 66.0, 400.0, 800.0));
        let mut b = Billet::nouveau(
            Nouveau {
                objet: "a".into(),
                ..Default::default()
            },
            "id1".into(),
            1,
            "2026-11-17T08:00:00Z".into(),
        )
        .unwrap();
        b.repondre("argos", "Je regarde.", "2026-11-17T09:00:00Z").unwrap();
        s.billets = vec![b];
        assert_eq!(s.non_lues(), 1);
        let (x, y) = centre(&s, Zone::Onglet(Onglet::Mes));
        assert_eq!(s.click(x, y), SupClick::Relire);
        let (x, y) = centre(&s, Zone::Billet(0));
        assert_eq!(s.click(x, y), SupClick::Lu("id1".into()));
        let (x, y) = centre(&s, Zone::Etat(0, Etat::Resolu));
        assert_eq!(s.click(x, y), SupClick::Etat("id1".into(), Etat::Resolu));
        let (x, y) = centre(&s, Zone::Repondre(0));
        assert_eq!(s.click(x, y), SupClick::None, "réponse vide : le clavier va au champ");
        assert_eq!(s.focus, Some(Champ::Reponse));
        s.taper("Merci");
        assert_eq!(s.entree(), SupClick::Repondre("id1".into()));
        assert!(s.draw(&Text::default(), Some((x, y))).is_some());
        assert_eq!(s.champs().len(), 1);
    }
}
