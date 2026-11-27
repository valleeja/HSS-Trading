//! 🛟 Support (🖥 PC): the « 🛟 Support » button in the top bar opens the tickets panel (problem, idea, question).
//! Tickets go into the tickets folder (`Argos HSS Trading/retours/`): the desktop app writes them itself
//! (`argos_store::retours`); the page goes through the PC server (`/api/retours…`). Nothing goes out to the
//! Internet. The panel is a modal layer: while it is open, the chart receives nothing.

use crate::*;
use argos_core::retours::{Billet, Contexte, Nouveau};
use argos_render::support::{Image, SupClick, Support};

/// The PC server routes (and the same names for the desktop app, which writes directly).
const LISTE: &str = "/api/retours";
const LU: &str = "/api/retours/lu";
const REPONDRE: &str = "/api/retours/repondre";
const ETAT: &str = "/api/retours/etat";

impl App {
    /// 🛟 Opens (or closes) the Support panel, in place of the right panel; on opening, the tickets are reloaded.
    pub(crate) fn support_ouvrir(&mut self) {
        let ouvert = self.ui.support.is_some() && self.watch.prefs.open && self.watch.prefs.panel == wl::Tool::Support;
        if ouvert {
            return self.support_fermer();
        }
        if self.top.phone {
            self.banner = Some("🛟 Le Support s'ouvre sur l'ordinateur pour l'instant.".into());
            return self.redraw();
        }
        (self.ui.list, self.ui.sheet, self.ui.argos, self.ui.notifs, self.ui.reglages) = (None, None, None, None, None);
        (self.watch.prefs.open, self.watch.prefs.panel) = (true, wl::Tool::Support);
        let v = self
            .volet_rect()
            .unwrap_or(Rect::new(self.side_x() - 408.0, crate::render::VOLET_HAUT, 400.0, 600.0));
        let mut g = Support::dans_volet(v);
        // 💡 the idea card: hidden until the date kept by its ✕ (interface-2.json)
        (g.idee_cachee, g.maintenant) = (Self::interface2().idee_cachee, crate::types::now_s());
        self.ui.support = Some(g);
        self.save_watch();
        self.support_demande(LISTE, None);
        self.redraw();
    }

    fn support_fermer(&mut self) {
        self.ui.support = None;
        self.watch.prefs.open = false;
        self.save_watch();
        self.redraw();
    }

    /// Click in the Support panel (position in the app window).
    pub(crate) fn support_click(&mut self) {
        let (x, y) = self.pointer.win_mouse;
        let Some(c) = self.ui.support.as_mut().map(|g| g.click(x, y)) else { return };
        self.support_faire(c);
    }

    pub(crate) fn support_faire(&mut self, c: SupClick) {
        match c {
            SupClick::None => {}
            SupClick::Fermer => return self.support_fermer(),
            SupClick::Agrandir => {
                self.ui.volet_large = !self.ui.volet_large;
                let l = self.ui.volet_large;
                Self::interface2_changer(|i| i.volet_large = l);
            }
            SupClick::Capturer => self.support_capturer(),
            SupClick::Joindre => self.support_joindre(),
            SupClick::Envoyer => self.support_envoyer(),
            SupClick::EnvoyerIdee => self.support_envoyer_idee(),
            SupClick::CacherIdee => {
                let t = self.ui.support.as_ref().and_then(|g| g.idee_cachee);
                Self::interface2_changer(|i| i.idee_cachee = t);
            }
            SupClick::Relire => self.support_demande(LISTE, None),
            SupClick::Lu(id) => self.support_demande(LU, Some(serde_json::json!({ "id": id }).to_string())),
            SupClick::Etat(id, e) => self.support_demande(ETAT, Some(serde_json::json!({ "id": id, "etat": e }).to_string())),
            SupClick::Repondre(id) => {
                let texte = self.ui.support.as_ref().map(|g| g.reponse.clone()).unwrap_or_default();
                self.support_demande(REPONDRE, Some(serde_json::json!({ "id": id, "texte": texte }).to_string()));
                if let Some(g) = self.ui.support.as_mut() {
                    (g.reponse, g.focus) = (String::new(), None);
                }
            }
        }
        self.redraw();
    }

    /// ⌨️ The Support panel keyboard: text into the selected field, Backspace, Enter, Escape (leaves the field, then
    /// closes the panel), Ctrl + V pastes (text).
    pub(crate) fn key_support(&mut self, event: winit::event::KeyEvent) {
        let ctrl = self.pointer.mods.control_key();
        let nom = match &event.logical_key {
            Key::Named(NamedKey::Escape) => "Escape",
            Key::Named(NamedKey::Backspace) => "Backspace",
            Key::Named(NamedKey::Enter) => "Enter",
            Key::Character(c) if ctrl && c.eq_ignore_ascii_case("v") => {
                // (browser: pasted content arrives through the page; a pasted image too)
                let t = self.clipboard.as_mut().and_then(|c| c.get_text().ok());
                if let (Some(t), Some(g)) = (t, self.ui.support.as_mut()) {
                    g.taper(&t);
                }
                return self.redraw();
            }
            _ => "",
        };
        let texte = event.text.as_ref().filter(|_| !ctrl).map(|t| t.to_string());
        self.key_support_nom(nom, texte)
    }

    /// A named key (`Escape`, `Backspace`, `Enter`) or text, for the Support panel (also the page's hidden field on
    /// the phone).
    pub(crate) fn key_support_nom(&mut self, nom: &str, texte: Option<String>) {
        let Some(g) = self.ui.support.as_mut() else { return };
        match nom {
            "Escape" if g.focus.is_some() => g.focus = None,
            "Escape" => return self.support_fermer(),
            "Backspace" => g.effacer(),
            "Enter" => {
                let c = g.entree();
                return self.support_faire(c);
            }
            _ => {
                if let Some(t) = texte {
                    g.taper(&t);
                }
            }
        }
        self.redraw();
    }

    /// The context recorded automatically: version, screen, symbol, timeframe, window size, date.
    fn support_contexte(&self) -> Contexte {
        let ecran = if self.ui.accueil.is_some() {
            "Accueil"
        } else if self.ui.mk_page.is_some() {
            "Marketplace"
        } else if self.ui.ac_page.is_some() {
            "Académie"
        } else if self.ui.pf_page.is_some() {
            "Portefeuille"
        } else if self.ui.bt_page.is_some() {
            "Rapport du backtest"
        } else {
            "chart"
        };
        Contexte {
            version: crate::actions::appmenu::BUILD.into(),
            ecran: ecran.into(),
            symbole: self.symbol_label(),
            ut: self.chart.interval.short(),
            taille: format!("{}×{}", self.win_w().round(), (self.top_h() + self.area.1 as f64).round()),
            date: argos_core::retours::date_iso(now_s()),
        }
    }

    /// 📷 Screenshot of the chart (like 📤 Partager, at the chart's size) into the ticket.
    fn support_capturer(&mut self) {
        let msg = if self.frame_now().is_none() {
            Some("⚠️ Pas de chart à capturer ici.".to_string())
        } else {
            let chart = Chart {
                crosshair: None,
                banner: None,
                pad: None,
                ..self.chart_with(true)
            };
            let (pw, ph) = (chart.viewport.plot_w(), chart.viewport.plot_h());
            let titre = format!("{} {} · Argos HSS Trading", self.symbol_label(), self.chart.interval.short());
            let png = argos_render::snapshot::idea_png(argos_render::render(&chart), pw, ph, &[], &titre, "", pw.ceil() as u32);
            match png.and_then(|o| Image::new(o, "chart.png")) {
                Some(im) => {
                    if let Some(g) = self.ui.support.as_mut() {
                        (g.image, g.entourer) = (Some(im), false);
                    }
                    None
                }
                None => Some("⚠️ Capture impossible ici.".to_string()),
            }
        };
        if let Some(g) = self.ui.support.as_mut() {
            g.message = msg;
        }
    }

    /// 📎 Attach a PNG / JPEG image: the page opens the file picker; the desktop app takes the latest image from
    /// Downloads.
    fn support_joindre(&mut self) {
        #[cfg(target_arch = "wasm32")]
        {
            self.ui.support_choix = true;
            let _ = platform::web::take_picked();
            platform::web::call_page("argosPick", &[".png,.jpg,.jpeg,image/png,image/jpeg"]);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let msg = match platform::derniere_image_telechargee() {
                Some((nom, o)) => self.support_image(o, &nom),
                None => Some("⚠️ Aucune image PNG ou JPEG dans tes Téléchargements.".into()),
            };
            if let Some(g) = self.ui.support.as_mut() {
                g.message = msg;
            }
        }
    }

    /// An attached or pasted image; the error message if there is one.
    fn support_image(&mut self, o: Vec<u8>, nom: &str) -> Option<String> {
        let Some(im) = Image::new(o, nom) else {
            return Some("⚠️ Seulement une image PNG ou JPEG.".into());
        };
        let g = self.ui.support.as_mut()?;
        (g.image, g.entourer) = (Some(im), false);
        None
    }

    /// Sends the ticket from the form.
    fn support_envoyer(&mut self) {
        let contexte = self.support_contexte();
        let appareil = self.support_appareil();
        let Some(g) = self.ui.support.as_mut() else { return };
        let image = match g.image.as_ref().map(Image::a_envoyer).transpose() {
            Ok(i) => i.map_or(String::new(), |(o, _)| argos_core::retours::base64_ecrire(&o)),
            Err(e) => {
                g.message = Some(format!("⚠️ {e}"));
                return;
            }
        };
        let n = Nouveau {
            genre: g.genre,
            objet: g.objet.clone(),
            description: g.description.clone(),
            contexte,
            appareil,
            image,
        };
        if let Err(e) = n.valider() {
            g.message = Some(format!("⚠️ {e}"));
            return;
        }
        (g.envoi, g.message) = (true, None);
        let corps = serde_json::to_string(&n).unwrap_or_default();
        self.support_demande(LISTE, Some(corps));
    }

    /// 💡 The idea of the card goes out as a 💡 Idée ticket, by the same path as the others (no image).
    fn support_envoyer_idee(&mut self) {
        let contexte = self.support_contexte();
        let appareil = self.support_appareil();
        let Some(g) = self.ui.support.as_mut() else { return };
        let Some((objet, description)) = g.idee_billet() else { return };
        let n = Nouveau {
            genre: argos_core::retours::Genre::Idee,
            objet,
            description,
            contexte,
            appareil,
            image: String::new(),
        };
        if let Err(e) = n.valider() {
            g.idee_message = Some(format!("⚠️ {e}"));
            return;
        }
        (g.envoi, g.idee_envoi, g.idee_message) = (true, true, None);
        let corps = serde_json::to_string(&n).unwrap_or_default();
        self.support_demande(LISTE, Some(corps));
    }

    /// This device's identifier (the sync one, in the local database); the page has none.
    fn support_appareil(&mut self) -> String {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(a) = self.store.as_mut().and_then(|s| s.appareil().ok()) {
            return a.0;
        }
        "page".into()
    }

    /// A request (route, body): the page sends it to the PC server (reply as [`UserEvent::Support`]); the desktop
    /// app handles it right away in the tickets folder.
    fn support_demande(&mut self, route: &'static str, corps: Option<String>) {
        #[cfg(target_arch = "wasm32")]
        {
            let Some(proxy) = self.proxy.clone() else { return };
            argos_data::rt::spawn(move || async move {
                use argos_data::http::Http;
                let c = argos_data::rt::Client::default();
                let envoi = corps.is_some();
                let rep = match corps {
                    Some(b) => c.post_json(route, &b).await,
                    None => c.get(route).await,
                };
                let _ = proxy.send_event(UserEvent::Support(route, envoi, rep.map_err(|e| e.to_string())));
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let envoi = corps.is_some();
            let rep = support_sur_le_disque(&support_dossier(), route, corps.as_deref());
            self.support_recu(route, envoi, rep);
        }
    }

    /// The response to a request: the list, or the changed ticket (or created: the form is cleared).
    pub(crate) fn support_recu(&mut self, route: &str, envoi: bool, rep: Result<String, String>) {
        let cree = route == LISTE && envoi;
        let rep = rep.and_then(|t| {
            if route == LISTE && !envoi {
                serde_json::from_str::<Vec<Billet>>(&t).map(Some).map_err(|e| e.to_string())
            } else if route == LU && t.contains("\"ok\"") {
                Ok(None)
            } else {
                serde_json::from_str::<Billet>(&t).map(|b| Some(vec![b])).map_err(|e| e.to_string())
            }
        });
        let Some(g) = self.ui.support.as_mut() else {
            // (panel closed: only the button badge)
            if let Ok(Some(v)) = &rep {
                if route == LISTE && !envoi {
                    self.ui.support_non_lues = v.iter().map(Billet::non_lues).sum();
                }
            }
            return self.redraw();
        };
        g.envoi = false;
        let idee = std::mem::take(&mut g.idee_envoi);
        match rep {
            Err(e) if idee => g.idee_message = Some(format!("⚠️ Support : {e}")),
            Err(e) => g.message = Some(format!("⚠️ Support : {e}")),
            Ok(None) => g.billets.iter_mut().for_each(Billet::marquer_lu),
            Ok(Some(v)) if route == LISTE && !envoi => g.billets = v,
            Ok(Some(mut v)) => {
                if let Some(b) = v.pop() {
                    match g.billets.iter_mut().find(|x| x.id == b.id) {
                        Some(x) => *x = b.clone(),
                        None => g.billets.insert(0, b.clone()),
                    }
                    if cree && idee {
                        g.idee_envoyee(b.numero);
                    } else if cree {
                        g.envoye(b.numero);
                    }
                }
            }
        }
        self.ui.support_non_lues = g.non_lues();
        self.redraw();
    }

    /// 🛟 On each loop wait: at startup, the tickets are reloaded once (button badge); in the page, the chosen (📎)
    /// or pasted (Ctrl + V) image arrives. `true`: still waiting (fast wake-up).
    pub(crate) fn support_tick(&mut self) -> bool {
        if !self.ui.support_lance && !self.top.phone {
            self.ui.support_lance = true;
            self.support_demande(LISTE, None);
        }
        #[cfg(target_arch = "wasm32")]
        let attente = {
            if self.ui.support.is_some() {
                let mut arrive = platform::web::take_colle();
                if self.ui.support_choix {
                    arrive.extend(platform::web::take_picked());
                }
                if let Some((nom, b64)) = arrive.pop() {
                    self.ui.support_choix = false;
                    let msg = match argos_core::retours::base64_lire(&b64) {
                        Some(o) => self.support_image(o, &nom),
                        None => Some("⚠️ Image illisible.".into()),
                    };
                    if let Some(g) = self.ui.support.as_mut() {
                        g.message = msg;
                    }
                    self.redraw();
                }
            } else {
                self.ui.support_choix = false;
            }
            self.ui.support.is_some()
        };
        #[cfg(not(target_arch = "wasm32"))]
        let attente = false;
        attente
    }
}

/// The desktop app's tickets folder (above the executable, like the server above the served folder).
#[cfg(not(target_arch = "wasm32"))]
fn support_dossier() -> std::path::PathBuf {
    // 🧪 tests never write into the real tickets folder
    if cfg!(test) {
        return argos_core::dossiers::de_test("retours");
    }
    let exe = std::env::current_exe().ok();
    argos_store::retours::dossier(exe.as_deref().and_then(|e| e.parent()))
}

/// The desktop app handles the request itself in folder `d` (same JSON responses as the PC server).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn support_sur_le_disque(d: &std::path::Path, route: &str, corps: Option<&str>) -> Result<String, String> {
    use argos_store::retours as disque;
    let err = |(_, m): (u16, String)| m;
    let json = |b: Billet| serde_json::to_string(&b).map_err(|e| e.to_string());
    let v: serde_json::Value = match corps {
        Some(c) if route != LISTE => serde_json::from_str(c).map_err(|e| e.to_string())?,
        _ => serde_json::Value::Null,
    };
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let id = s("id");
    match (route, corps) {
        (LISTE, None) => serde_json::to_string(&disque::liste(d)).map_err(|e| e.to_string()),
        (LISTE, Some(c)) => {
            let n: Nouveau = serde_json::from_str(c).map_err(|e| e.to_string())?;
            json(disque::creer(d, n).map_err(err)?)
        }
        (LU, _) if id.is_empty() => disque::tous_lus(d).map(|_| r#"{"ok":true}"#.to_string()).map_err(err),
        (LU, _) => json(
            disque::changer(d, &id, |b| {
                b.marquer_lu();
                Ok(())
            })
            .map_err(err)?,
        ),
        (REPONDRE, _) => json(disque::changer(d, &id, |b| b.repondre("admin", &s("texte"), &disque::maintenant())).map_err(err)?),
        (ETAT, _) => {
            let e: argos_core::retours::Etat = serde_json::from_value(v["etat"].clone()).map_err(|_| "état inconnu".to_string())?;
            json(disque::changer(d, &id, |b| b.changer_etat(e, "admin", &disque::maintenant())).map_err(err)?)
        }
        _ => Err(format!("route inconnue : {route}")),
    }
}
