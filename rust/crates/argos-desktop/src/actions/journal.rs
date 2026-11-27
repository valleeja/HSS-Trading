//! 📓 Journal de trading. 🔐 Il vit chiffré dans la base sensible du PC (`argos_store::journal`) : l'app de bureau
//! l'ouvre le temps d'une lecture ou d'une écriture ; le cell le lit et l'écrit par le serveur local du PC, sans en
//! garder de copie. Écritures entrée par entrée (`Changements`) : jamais le journal entier, jamais d'écrasement par un
//! état plus vieux.

use crate::*;
use argos_core::journal::{Changements, Donnees};

/// 🔐 Où en est le journal partagé.
#[derive(Default)]
pub(crate) struct JournalSync {
    /// Le dernier état connu de la base (ce que cet appareil a lu ou écrit) : les changements se calculent par rapport
    /// à lui.
    pub(crate) connu: Donnees,
    /// Le journal est lu : on peut l'écrire. Sinon (base ou PC injoignable), lecture seule : rien n'est écrit.
    pub(crate) pret: bool,
    /// 🖥️ La base sensible (après `journal_charger` ; `None` : en mémoire seulement), sa date vue et la dernière
    /// vérification (le journal est relu quand un autre appareil l'a changé).
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) base: Option<std::path::PathBuf>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) vu: Option<std::time::SystemTime>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) verifie: Option<Instant>,
}

/// 🌐 L'ancienne copie en clair du journal dans la page (`argos:journal.json`), rendue au PC puis effacée.
#[cfg(any(target_arch = "wasm32", test))]
const ANCIEN_PAGE: &str = "journal.json";

/// 🌐 Ce que l'ancienne copie de la page (`copie`, l'ancien format du journal) a de plus que le PC (`pc`) : ses trades
/// absents du PC (trades notés sur le cell hors ligne). Un trade présent des deux côtés garde la version du PC (la
/// copie de la page est plus vieille). `None` : copie illisible (gardée telle quelle).
#[cfg(any(target_arch = "wasm32", test))]
fn copie_a_rendre(copie: &str, pc: &Donnees) -> Option<Changements> {
    if copie.trim().is_empty() {
        return Some(Changements::default());
    }
    let vieux: Donnees = serde_json::from_str(copie).ok()?;
    Some(Changements {
        mettre: vieux
            .trades
            .into_iter()
            .filter(|t| !t.id.is_empty() && !pc.trades.iter().any(|p| p.id == t.id))
            .collect(),
        ..Default::default()
    })
}

/// 🌐 Le journal du PC par le serveur local (`/api/journal`, mêmes accès que la synchro : cet ordi ou Tailscale) :
/// lu (`None`), ou après l'envoi de changements.
#[cfg(target_arch = "wasm32")]
async fn journal_du_pc(ch: Option<&Changements>) -> Result<Donnees, String> {
    use argos_data::http::Http;
    let c = argos_data::rt::Client::default();
    let texte = match ch {
        None => c.get("/api/journal").await,
        Some(ch) => c.post_json("/api/journal", &serde_json::to_string(ch).map_err(|e| e.to_string())?).await,
    }
    .map_err(|e| e.to_string())?;
    serde_json::from_str(&texte).map_err(|_| "réponse du PC illisible".to_string())
}

/// 🖥️ Date du fichier de la base (signal de changement, sans l'ouvrir).
#[cfg(not(target_arch = "wasm32"))]
fn date_de(p: &std::path::Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

/// 🔐 What became of a journal write, for the banners that say a trade is noted.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Note {
    /// Written in the encrypted journal (or nothing new to write).
    Notee,
    /// 🌐 Sent to the PC (the phone): its answer comes later.
    #[cfg(target_arch = "wasm32")]
    Envoyee,
    /// Not written, and why (journal unavailable, write refused, trade missing from the journal).
    NonNotee(String),
}

impl Note {
    /// The end of a banner about closed trades: noted, sent, or a clear warning (the closing itself is done).
    pub(crate) fn suffixe(&self) -> String {
        match self {
            Note::Notee => ", inscrit au journal".into(),
            #[cfg(target_arch = "wasm32")]
            Note::Envoyee => ", envoyé au journal du PC".into(),
            Note::NonNotee(pourquoi) => format!(" ; ⚠ non noté au journal ({pourquoi})"),
        }
    }
}

impl App {
    /// 🔐 Au lancement : le journal de la base chiffrée (l'ancien `journal.json` en clair y est migré une fois).
    /// Base indisponible (trousseau absent, base illisible…) : lecture seule avec un bandeau, rien n'est écrit en clair.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn journal_charger(&mut self) {
        let Some(p) = crate::persist::journal::chemin() else {
            return self.journal_indisponible("pas de dossier de données");
        };
        match crate::persist::journal::charger(&p) {
            Ok(d) => {
                self.jsync.vu = date_de(&p);
                self.jsync.base = Some(p);
                self.journal_recu(d);
            }
            Err(e) => self.journal_indisponible(&e),
        }
    }

    /// 🔐 Journal illisible ou injoignable : lecture seule, réglages du Gardien par défaut, un bandeau ; aucune écriture
    /// (rien ne pourra écraser le vrai journal avec un état vide).
    pub(crate) fn journal_indisponible(&mut self, e: &str) {
        self.jsync.pret = false;
        self.banner = Some(format!("🔐 Journal indisponible : {e} (lecture seule)"));
    }

    /// 🔐 Le journal lu (ou relu) : il remplace celui de l'écran ; on peut l'écrire.
    pub(crate) fn journal_recu(&mut self, d: Donnees) {
        let j = &mut self.watch.panels.journal;
        (j.cfg, j.trades) = (d.cfg.clone(), d.trades.clone());
        self.jsync.connu = d;
        self.jsync.pret = true;
    }

    /// 🔐 Enregistre ce qui a changé dans le journal (`journal_noter`), quand celui qui écrit n'a rien à en dire.
    pub(crate) fn journal_ecrire(&mut self) {
        self.journal_noter();
    }

    /// 🔐 Enregistre ce qui a changé dans le journal depuis le dernier état connu (entrée par entrée), et dit ce
    /// qu'il en est (`Note`). Lecture seule : rien n'est écrit (un bandeau le dit). Une écriture ratée reste à envoyer
    /// (elle repart avec la suivante).
    pub(crate) fn journal_noter(&mut self) -> Note {
        if !self.jsync.pret {
            self.banner = Some("🔐 Journal en lecture seule : rien n'est enregistré".into());
            return Note::NonNotee("journal indisponible".into());
        }
        let (cfg, trades) = (self.watch.panels.journal.cfg.clone(), self.watch.panels.journal.trades.clone());
        let ch = Changements::entre(&self.jsync.connu, &cfg, &trades);
        if ch.vide() {
            return Note::Notee;
        }
        #[cfg(not(target_arch = "wasm32"))]
        let note = match self.jsync.base.clone() {
            // en mémoire seulement (tests) : le nouvel état devient l'état connu
            None => {
                self.jsync.connu = Donnees {
                    cfg,
                    trades,
                    version: self.jsync.connu.version,
                };
                Note::Notee
            }
            Some(p) => match crate::persist::journal::ecrire(&p, &ch) {
                Ok(d) => {
                    // ce que cet appareil a écrit devient l'état connu ; les changements des autres viennent par
                    // `journal_suivre` (relu en entier)
                    self.jsync.connu = Donnees {
                        cfg,
                        trades,
                        version: d.version,
                    };
                    self.jsync.vu = date_de(&p);
                    Note::Notee
                }
                Err(e) => {
                    self.banner = Some(format!("🔐 Journal non enregistré : {e} (réessayé à la prochaine écriture)"));
                    Note::NonNotee(e)
                }
            },
        };
        // 🌐 le cell : envoyé au PC, sa réponse vient plus tard (`journal_ecrit` dit si c'est raté)
        #[cfg(target_arch = "wasm32")]
        let note = {
            self.journal_envoyer(ch, false);
            Note::Envoyee
        };
        note
    }

    /// 🌐 Le cell : le journal est demandé au PC (serveur local) ; rien n'est gardé dans la page. Jusqu'à la réponse,
    /// lecture seule.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn journal_charger(&mut self) {
        self.jsync.pret = false;
        let Some(proxy) = self.proxy.clone() else { return };
        argos_data::rt::spawn(move || async move {
            let res = journal_du_pc(None).await;
            let _ = proxy.send_event(UserEvent::JournalLu(res));
        });
    }

    /// 🌐 Le journal du PC est arrivé : il remplace celui de l'écran. L'ancienne copie en clair de la page
    /// (`argos:journal.json`) rend au PC les trades qu'il n'a pas, puis elle est effacée (seulement après la réponse du
    /// PC : aucun trade perdu). PC injoignable : lecture seule, rien n'est écrit.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn journal_lu(&mut self, res: Result<Donnees, String>) {
        let d = match res {
            Ok(d) => d,
            Err(_) => return self.journal_indisponible("PC injoignable"),
        };
        self.journal_recu(d);
        if let Some(copie) = platform::read(ANCIEN_PAGE) {
            match copie_a_rendre(&copie, &self.jsync.connu) {
                Some(ch) if !ch.vide() => self.journal_envoyer(ch, true),
                _ => platform::effacer(ANCIEN_PAGE),
            }
        }
        self.redraw();
    }

    /// 🌐 Envoie des changements au PC (le cell n'en garde aucune copie).
    #[cfg(target_arch = "wasm32")]
    fn journal_envoyer(&self, ch: Changements, effacer: bool) {
        let Some(proxy) = self.proxy.clone() else { return };
        argos_data::rt::spawn(move || async move {
            let res = journal_du_pc(Some(&ch)).await;
            let _ = proxy.send_event(UserEvent::JournalEcrit(res, ch, effacer));
        });
    }

    /// 🌐 Réponse du PC : ce qui a été envoyé rejoint l'état connu (sinon, il repart avec la prochaine écriture) ;
    /// l'ancienne copie rendue au PC est effacée de la page.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn journal_ecrit(&mut self, res: Result<Donnees, String>, envoye: Changements, effacer: bool) {
        match res {
            Ok(d) if effacer => {
                platform::effacer(ANCIEN_PAGE);
                self.journal_recu(d);
            }
            Ok(d) => {
                envoye.appliquer(&mut self.jsync.connu);
                self.jsync.connu.version = d.version;
            }
            Err(_) => self.banner = Some("🔐 Journal non enregistré : PC injoignable (réessayé à la prochaine écriture)".into()),
        }
        self.redraw();
    }

    /// 🖥️ Le journal a-t-il changé ailleurs (le cell, par le serveur local) ? Vérifié au plus toutes les 2 s, d'abord par
    /// la date du fichier (sans l'ouvrir), puis par sa version ; s'il a changé, il est relu en entier.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn journal_suivre(&mut self) {
        let Some(p) = self.jsync.base.clone() else { return };
        if self.jsync.verifie.is_some_and(|t| t.elapsed().as_secs() < 2) {
            return;
        }
        self.jsync.verifie = Some(Instant::now());
        let date = date_de(&p);
        if date == self.jsync.vu {
            return;
        }
        if let Ok(d) = crate::persist::journal::charger(&p) {
            self.jsync.vu = date;
            if d.version != self.jsync.connu.version {
                self.journal_recu(d);
                self.redraw();
            }
        }
    }

    /// 📓 Journal : nouvelle fiche, modifier, export CSV / sauvegarde JSON (dans Téléchargements), import de la sauvegarde.
    pub(crate) fn journal_click(&mut self, c: argos_render::journal::JClick) {
        use argos_render::journal::{date_txt, now_ms, sess_of, JClick, JForm};
        let (w, h) = (self.win_w(), self.area.1 as f64 + self.top_h());
        let j = &mut self.watch.panels.journal;
        match c {
            JClick::New => {
                let now = now_ms();
                let t = argos_core::journal::JTrade {
                    id: format!("{now:x}"),
                    sym: if self.top.coin.is_empty() {
                        self.watch.selected.clone()
                    } else {
                        self.top.coin.clone()
                    },
                    side: "long".into(),
                    tf: self.chart.interval.short(),
                    strat: j.cfg.strats.first().cloned().unwrap_or_default(),
                    risk: Some(j.cfg.risk),
                    entry_ts: now,
                    ..Default::default()
                };
                self.ui.j_form = Some(JForm::new(t, true, j.cfg.strats.clone(), w, h));
            }
            JClick::Edit(i) => {
                if let Some(t) = j.trades.get(i) {
                    self.ui.j_form = Some(JForm::new(t.clone(), false, j.cfg.strats.clone(), w, h));
                }
            }
            JClick::Export(csv) => {
                let (name, body) = if csv {
                    let list: Vec<_> = j
                        .trades
                        .iter()
                        .filter(|t| j.strat.as_ref().is_none_or(|s| *s == t.strat) && j.sym.as_ref().is_none_or(|s| *s == t.sym))
                        .cloned()
                        .collect();
                    ("argos-journal.csv", argos_core::journal::csv(&list, &date_txt, &|ms| sess_of(ms).to_string()))
                } else {
                    ("argos-journal.json", serde_json::to_string_pretty(&j.trades).unwrap_or_default())
                };
                self.banner = Some(match platform::save_download(name, body.as_bytes()) {
                    // 🔐 le journal est chiffré dans Argos, mais ce fichier exporté, lui, est en clair
                    Ok(dir) => format!("💾 {name} enregistré dans {dir} · ⚠️ en clair : garde-le pour toi"),
                    Err(e) => format!("Écriture impossible ({e})"),
                });
            }
            JClick::Import => {
                let got = platform::read_download("argos-journal.json").and_then(|s| serde_json::from_str::<Vec<argos_core::journal::JTrade>>(&s).ok());
                self.banner = Some(match got {
                    Some(list) => {
                        let mut n = 0;
                        for t in list.into_iter().filter(|t| !t.id.is_empty() && t.entry.is_some()) {
                            j.trades.retain(|x| x.id != t.id);
                            j.trades.push(t);
                            n += 1;
                        }
                        self.journal_ecrire();
                        format!("📂 {n} trades importés")
                    }
                    None => "Fichier invalide ou absent : mets la sauvegarde JSON du journal Argos (argos-journal.json) dans tes Téléchargements".to_string(),
                });
            }
            // 🆘 the rescue code of the journal key: desktop app only (the button is not in the browser page)
            #[cfg(not(target_arch = "wasm32"))]
            JClick::Secours => {
                let avis = vec![
                    "Note-le sur papier : avec lui, ton journal se rouvre même après une réinstallation ou un trousseau perdu.".to_string(),
                    "• Pour le voir : tape AFFICHER, puis Valider.".into(),
                    "• Pour restaurer la clé : tape ton code (tirets permis), puis Valider. Il est vérifié sur ton journal avant d'aller au trousseau.".into(),
                ];
                self.session.key_box = Some(argos_render::exchanges::KeyBox::new("journal-secours", "", w, h).avec_avis(avis));
            }
            #[cfg(target_arch = "wasm32")]
            JClick::Secours => {}
            JClick::None | JClick::Changed => {}
        }
    }

    /// 🆘 Answer of the rescue code boxes: « AFFICHER » shows the code of the device key (checked against the journal) in
    /// its own box; a code restores the key (checked against the journal, then kept in the keyring) and reopens the
    /// journal; « Fermer » on the code box wipes it. Everything typed or shown is overwritten, never written anywhere.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn journal_secours_save(&mut self, mut b: argos_render::exchanges::KeyBox) {
        use argos_render::exchanges::{effacer, KeyBox};
        let mut tape = b.fields.first().map(|f| f.value.trim().to_string()).unwrap_or_default();
        b.wipe();
        if b.exchange == "journal-code" {
            return;
        }
        let Some(p) = crate::persist::journal::chemin() else {
            self.banner = Some("🆘 pas de dossier de données sur cet appareil".into());
            return effacer(&mut tape);
        };
        let r = if tape == "AFFICHER" {
            argos_store::secours::code_de_l_appareil(&p, argos_store::journal::cle_gardee).map(|mut code| {
                let (w, h) = (self.win_w(), self.area.1 as f64 + self.top_h());
                self.session.key_box = Some(KeyBox::code_secours(&code, w, h));
                effacer(&mut code);
            })
        } else {
            argos_store::secours::restaurer(&p, &tape).map(|()| {
                self.journal_charger();
                if self.jsync.pret {
                    self.banner = Some("✅ Clé du journal restaurée dans le trousseau : ton journal est rouvert".into());
                }
            })
        };
        effacer(&mut tape);
        if let Err(e) = r {
            self.banner = Some(format!("🆘 {e}"));
        }
    }

    /// Réponse de la fiche d'un trade.
    pub(crate) fn j_form_result(&mut self, f: argos_render::journal::JForm, c: argos_render::journal::FormClick) {
        use argos_render::journal::FormClick;
        let j = &mut self.watch.panels.journal;
        match c {
            FormClick::Close => return,
            FormClick::Warn(msg) => self.banner = Some(msg.into()),
            FormClick::Save(t) => {
                let t = *t;
                if !t.strat.is_empty() && !j.cfg.strats.contains(&t.strat) {
                    j.cfg.strats.push(t.strat.clone());
                }
                match j.trades.iter_mut().find(|x| x.id == t.id) {
                    Some(x) => *x = t,
                    None => j.trades.push(t),
                }
                self.journal_ecrire();
                self.banner = Some("📓 Trade enregistré".into());
                return;
            }
            FormClick::Delete(id) => {
                j.trades.retain(|x| x.id != id);
                self.journal_ecrire();
                return;
            }
            FormClick::None | FormClick::Changed => {}
        }
        self.ui.j_form = Some(f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argos_core::journal::JTrade;

    fn trade(id: &str) -> JTrade {
        JTrade {
            id: id.into(),
            sym: "BTC".into(),
            entry: Some(100.0),
            ..Default::default()
        }
    }

    fn app() -> App {
        let iv = Interval(1800);
        App::blank(Source::File("essai.json".into()), iv, iv, vec![], "essai".into())
    }

    /// 🌐 L'ancienne copie du cell : ses trades absents du PC lui sont rendus (aucun perdu) ; un trade présent des deux
    /// côtés garde la version du PC ; une copie illisible n'est pas effacée.
    #[test]
    fn copie_du_cell_rendue_sans_perte() {
        let pc = Donnees {
            trades: vec![trade("a"), trade("b")],
            ..Default::default()
        };
        let copie = r#"{"tab":0,"trades":[{"id":"a","sym":"BTC","entry":1.0},{"id":"hors-ligne","sym":"ETH","entry":2.0}],"cfg":{"tilt_n":9}}"#;
        let ch = copie_a_rendre(copie, &pc).unwrap();
        assert_eq!(ch.mettre.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(), ["hors-ligne"]);
        assert!(
            ch.cfg.is_none() && ch.retirer.is_empty(),
            "jamais les réglages ni des retraits depuis la vieille copie"
        );
        assert!(copie_a_rendre("", &pc).unwrap().vide(), "copie vide : rien à rendre, elle s'efface");
        assert_eq!(copie_a_rendre("{ abîmée", &pc), None, "illisible : gardée");
        assert_eq!(ANCIEN_PAGE, "journal.json");
    }

    /// 🔐 PC (ou trousseau) injoignable : lecture seule ; rien n'est écrit (pas d'écrasement par un journal vide), un
    /// ordre démo est refusé avec un message.
    #[test]
    fn injoignable_sans_ecrasement() {
        let mut a = app();
        a.journal_indisponible("PC injoignable");
        assert_eq!(a.banner.as_deref(), Some("🔐 Journal indisponible : PC injoignable (lecture seule)"));
        a.watch.panels.journal.trades.clear();
        a.journal_ecrire();
        assert_eq!(a.jsync.connu, Donnees::default(), "rien d'envoyé");
        assert_eq!(a.banner.as_deref(), Some("🔐 Journal en lecture seule : rien n'est enregistré"));
        let t = argos_core::simu::Ticket {
            buy: true,
            kind: argos_core::simu::Kind::Market,
            price: 100.0,
            sl: 90.0,
            tp: 120.0,
            risk: argos_core::Money::from_f64(100.0),
        };
        a.demo_ticket(t, 100.0, 0);
        assert!(a.demo.positions.is_empty());
        assert!(a.banner.as_deref().is_some_and(|b| b.starts_with("🔐 Ordre démo refusé")));
    }

    /// 🖥️ Deux appareils sur la même base chiffrée (l'app de bureau et le cell par le serveur) : chacun n'écrit que ses
    /// entrées ; le bureau relit ce que l'autre a écrit (date du fichier, puis version) ; rien de perdu.
    #[test]
    fn bureau_relit_ce_que_le_cell_a_ecrit() {
        let p = crate::persist::journal::chemin().unwrap();
        let _ = std::fs::remove_dir_all(p.parent().unwrap());
        let mut pc = app();
        pc.journal_charger();
        assert!(pc.jsync.pret && pc.jsync.base.is_some());
        pc.watch.panels.journal.trades.push(trade("pc-1"));
        pc.journal_ecrire();
        // l'autre appareil (le serveur pour le cell) ajoute un trade et change un réglage, entrée par entrée
        let cell = Changements {
            cfg: Some(argos_core::journal::JCfg {
                tilt_n: 2,
                ..Default::default()
            }),
            mettre: vec![trade("cell-1")],
            retirer: vec![],
        };
        std::thread::sleep(std::time::Duration::from_millis(20)); // une date de fichier différente
        crate::persist::journal::ecrire(&p, &cell).unwrap();
        pc.jsync.verifie = None; // pas d'attente des 2 s dans le test
        pc.journal_suivre();
        let ids: Vec<&str> = pc.watch.panels.journal.trades.iter().map(|t| t.id.as_str()).collect();
        assert!(ids.contains(&"pc-1") && ids.contains(&"cell-1"), "{ids:?}");
        assert_eq!(pc.watch.panels.journal.cfg.tilt_n, 2);
        // relance : tout est relu de la base chiffrée
        let mut relance = app();
        relance.journal_charger();
        assert_eq!(relance.watch.panels.journal.trades.len(), 2);
        let _ = std::fs::remove_dir_all(p.parent().unwrap());
    }
}
