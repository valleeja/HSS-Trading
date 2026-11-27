//! 👥 Communauté (v1 locale, sans serveur ni compte) : un seul endroit pour le Top gagnants / perdants, l'Académie et
//! le Marketplace (ce qui existait déjà), plus un fil 💡 Idées gardé sur l'appareil (`ideas.json`) : une idée de trade
//! (marché, unité, sens, entrée / SL / TP, texte) publiée depuis la chart ouverte, rouverte d'un clic, exportée et
//! importée en fichier pour la partager à la main. 🌐 Nostr : emplacement prévu, rien n'est envoyé.
//! 📸 Une idée peut garder une image de la chart (lignes entrée / SL / TP et légende dessinées, 1280 px de large au
//! plus) : rangée à part (`idea-images/<id>.b64`, pas dans `ideas.json`), mise dans le fichier d'export, partagée par la
//! feuille de partage du téléphone ou téléchargée.

use crate::*;
use argos_render::listmenu::{MenuGroup, MenuItem};

/// Une idée de trade.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct Idea {
    pub(crate) id: String,
    /// Moment de la publication (secondes).
    pub(crate) t: i64,
    /// Ligne de watchlist du marché (« hl:BTC », « binance-spot:BTCUSDT »…) et son nom affiché.
    pub(crate) coin: String,
    pub(crate) label: String,
    /// Unité de la chart (secondes) au moment de la publication.
    pub(crate) interval: i64,
    pub(crate) long: bool,
    pub(crate) entry: Option<f64>,
    pub(crate) sl: Option<f64>,
    pub(crate) tp: Option<f64>,
    pub(crate) text: String,
    /// 📸 Une image de la chart est rangée pour cette idée.
    pub(crate) snap: bool,
    /// 📸 L'image (PNG en base64) : seulement dans le fichier d'export, jamais dans `ideas.json`.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) image: String,
}

impl Idea {
    /// R:R (objectif / risque) si entrée, stop et objectif sont là.
    pub(crate) fn rr(&self) -> Option<f64> {
        let (e, s, t) = (self.entry?, self.sl?, self.tp?);
        let risk = (e - s).abs();
        (risk > 0.0).then(|| (t - e).abs() / risk)
    }

    pub(crate) fn line(&self) -> String {
        let n = |v: Option<f64>| v.map_or("—".to_string(), |x| argos_render::viewport::format_number(x, 2));
        let rr = self.rr().map_or(String::new(), |r| format!(" · R:R {r:.1}"));
        format!(
            "{} {} {} · E {} · SL {} · TP {}{rr}",
            if self.long { "▲" } else { "▼" },
            self.label,
            Interval(self.interval).short(),
            n(self.entry),
            n(self.sl),
            n(self.tp)
        )
    }
}

/// Lit une idée tapée : « long 76000 sl 75000 tp 80000 cassure du range » (sens, puis prix après sl / tp, le reste en
/// texte). Les nombres acceptent l'espace et la virgule (« 76 000 », « 0,52 »).
pub(crate) fn parse_idea(input: &str, default_entry: Option<f64>) -> Option<(bool, Option<f64>, Option<f64>, Option<f64>, String)> {
    let mut long = None;
    let (mut entry, mut sl, mut tp) = (None, None, None);
    let mut text = vec![];
    let words: Vec<&str> = input.split_whitespace().collect();
    let num = |s: &str| {
        s.replace(',', ".")
            .replace(['$', ' '], "")
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite() && *v > 0.0)
    };
    let mut i = 0;
    while i < words.len() {
        let w = words[i].to_lowercase();
        match w.as_str() {
            "long" | "achat" | "buy" if long.is_none() => long = Some(true),
            "short" | "vente" | "sell" if long.is_none() => long = Some(false),
            "sl" | "stop" | "tp" | "objectif" | "e" | "entrée" | "entree" => {
                if let Some(v) = words.get(i + 1).and_then(|s| num(s)) {
                    match w.as_str() {
                        "sl" | "stop" => sl = Some(v),
                        "tp" | "objectif" => tp = Some(v),
                        _ => entry = Some(v),
                    }
                    i += 1;
                } else {
                    text.push(words[i]);
                }
            }
            _ => match num(&w) {
                Some(v) if entry.is_none() && long.is_some() && text.is_empty() => entry = Some(v),
                _ => text.push(words[i]),
            },
        }
        i += 1;
    }
    Some((long?, entry.or(default_entry), sl, tp, text.join(" ").chars().take(300).collect()))
}

const PUBLISH: i64 = 1;
const EXPORT: i64 = 2;
const IMPORT: i64 = 3;
const DELETE: i64 = 4;
const MOVERS: i64 = 10;
const ACADEMY: i64 = 11;
const MARKET: i64 = 12;
const MODULES: i64 = 14;
const OPEN: i64 = 5;
const SHARE: i64 = 6;
const SAVE_IMG: i64 = 7;
const BACK: i64 = 8;
const PUBLISH_SNAP: i64 = 9;
const SNAP_NOW: i64 = 13;
const IDEA0: i64 = 100;
/// 📸 Largeur maximale de l'image d'une idée (pixels).
const IMG_W: u32 = 1280;

/// Nom sûr (lettres, chiffres, tiret) tiré de l'identifiant d'une idée.
fn safe_id(id: &str) -> String {
    id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').take(40).collect()
}

fn img_key(id: &str) -> String {
    format!("idea-images/{}.b64", safe_id(id))
}

fn b64() -> base64::engine::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}

/// Texte qui accompagne l'image partagée.
fn share_text(i: &Idea) -> String {
    let t = if i.text.is_empty() { String::new() } else { format!(" — {}", i.text) };
    format!("💡 {}{t} · Argos HSS Trading", i.line())
}
const FILE: &str = "argos-idees.json";

impl App {
    pub(crate) fn save_ideas(&self) {
        Self::write_prefs("ideas.json", &self.ui.ideas);
    }

    /// 📸 L'image rangée d'une idée (PNG).
    pub(crate) fn idea_image(i: &Idea) -> Option<Vec<u8>> {
        use base64::Engine;
        let raw = platform::read(&img_key(&i.id)).filter(|s| !s.is_empty())?;
        b64().decode(raw.trim()).ok().filter(|b| b.starts_with(b"\x89PNG"))
    }

    /// 📸 Range l'image d'une idée ; `false` si le stockage est plein.
    fn store_image(id: &str, png: &[u8]) -> bool {
        use base64::Engine;
        platform::write(&img_key(id), &b64().encode(png)).is_ok()
    }

    /// 📸 La chart affichée, avec les lignes de l'idée et sa légende, en PNG (1280 px de large au plus).
    pub(crate) fn idea_snapshot(&self, i: &Idea) -> Option<Vec<u8>> {
        use argos_render::palette;
        use argos_render::snapshot::{idea_png, Mark};
        let f = self.frame_now()?;
        let g = f.geo();
        let n = |v: f64| argos_render::viewport::format_number(v, 2);
        let marks: Vec<Mark> = [
            (i.entry, palette::accent(), "Entrée"),
            (i.sl, palette::down(), "SL"),
            (i.tp, palette::up(), "TP"),
        ]
        .into_iter()
        .filter_map(|(v, color, name)| {
            v.map(|v| Mark {
                y: g.y_of(v),
                color,
                label: format!("{name} {}", n(v)),
            })
        })
        .collect();
        let rr = i.rr().map_or(String::new(), |r| format!(" · R:R {r:.1}"));
        let caption = format!(
            "{} {} {} · {}{rr} · Argos HSS Trading",
            if i.long { "▲" } else { "▼" },
            i.label,
            Interval(i.interval).short(),
            if i.long { "LONG" } else { "SHORT" }
        );
        let chart = Chart {
            crosshair: None,
            banner: None,
            pad: None,
            trades: vec![],
            ..self.chart_with(true)
        };
        let (pw, ph) = (chart.viewport.plot_w(), chart.viewport.plot_h());
        idea_png(argos_render::render(&chart), pw, ph, &marks, &caption, &i.text, IMG_W)
    }

    /// 📤 Partager (barre du haut) : une image de la chart affichée (symbole, unité, « Argos HSS Trading »), vers la
    /// feuille de partage (navigateur, cell) ou les téléchargements (PC) ; le message du bandeau.
    pub(crate) fn partager_chart(&self) -> String {
        if self.frame_now().is_none() {
            return "📤 Rien à partager ici (pas de chart)".into();
        }
        let chart = Chart {
            crosshair: None,
            banner: None,
            pad: None,
            ..self.chart_with(true)
        };
        let (pw, ph) = (chart.viewport.plot_w(), chart.viewport.plot_h());
        let titre = format!("{} {} · Argos HSS Trading", self.symbol_label(), self.chart.interval.short());
        let Some(png) = argos_render::snapshot::idea_png(argos_render::render(&chart), pw, ph, &[], &titre, "", IMG_W) else {
            return "📤 Image impossible ici".into();
        };
        let nom = format!("argos-chart-{}.png", now_s());
        match platform::share_image(&nom, &png, &titre) {
            Ok(vers) => format!("📤 Image de la chart envoyée vers {vers}"),
            Err(e) => format!("Partage impossible ({e})"),
        }
    }

    /// 📸 Capture la chart pour l'idée n° `k` et la range ; le message du bandeau.
    fn snap_idea(&mut self, k: usize) -> String {
        let Some(i) = self.ui.ideas.get(k).cloned() else { return String::new() };
        let Some(png) = self.idea_snapshot(&i) else {
            return "📸 Capture impossible ici (pas de chart)".into();
        };
        if !Self::store_image(&i.id, &png) {
            return format!(
                "📸 Image trop lourde pour le stockage de cet appareil ({} Ko) : idée gardée sans image",
                png.len() / 1024
            );
        }
        self.ui.ideas[k].snap = true;
        self.save_ideas();
        format!("📸 Image gardée avec l'idée ({} Ko)", png.len() / 1024)
    }

    /// Menu 👥 Communauté.
    pub(crate) fn community_menu(&self) -> Vec<MenuGroup> {
        let item = |id: i64, label: String| MenuItem {
            id,
            label,
            enabled: true,
            ..Default::default()
        };
        let mut ideas: Vec<MenuItem> = self
            .ui
            .ideas
            .iter()
            .enumerate()
            .rev()
            .take(40)
            .map(|(k, i)| MenuItem {
                id: IDEA0 + k as i64,
                label: format!(
                    "{}{}{}",
                    if self.ui.ideas_delete { "🗑 " } else { "" },
                    if i.snap { "📸 " } else { "" },
                    i.line()
                ),
                hint: i.text.chars().take(60).collect(),
                enabled: true,
                color: Some(if i.long { argos_render::palette::up() } else { argos_render::palette::down() }),
                ..Default::default()
            })
            .collect();
        if ideas.is_empty() {
            ideas.push(MenuItem {
                id: -1,
                label: "Aucune idée pour l'instant : publie la première depuis la chart".into(),
                enabled: false,
                ..Default::default()
            });
        }
        let mut groups = vec![];
        if let Some(i) = self.ui.idea_open.as_ref().and_then(|id| self.ui.ideas.iter().find(|i| &i.id == id)) {
            let mut items = vec![
                MenuItem {
                    id: -1,
                    label: i.line(),
                    hint: i.text.chars().take(80).collect(),
                    enabled: false,
                    color: Some(if i.long { argos_render::palette::up() } else { argos_render::palette::down() }),
                    ..Default::default()
                },
                item(OPEN, "👁 Ouvrir la chart de l'idée".into()),
            ];
            if i.snap {
                items.push(item(SHARE, "📤 Partager (image + texte)".into()));
                items.push(item(SAVE_IMG, "💾 Télécharger l'image".into()));
            } else {
                items.push(item(SNAP_NOW, "📸 Ajouter une image de la chart affichée".into()));
            }
            items.push(item(BACK, "↩ Retour aux idées".into()));
            groups.push(MenuGroup {
                title: "🔎 IDÉE".into(),
                items,
                strip: false,
            });
        }
        // 📱 au doigt : l'Académie, le Marketplace et les Modules sont déjà des tuiles du menu ⋯ Argos, et rien de « bientôt »
        let doigt = argos_render::actions::tactile();
        let mut communaute = vec![item(MOVERS, "🚀 Top gagnants / perdants 24 h".into())];
        if !doigt {
            communaute.extend([
                item(ACADEMY, "🎓 Académie Argos".into()),
                item(MARKET, "🛒 Marketplace d'indicateurs".into()),
                item(MODULES, "🧩 Modules : catalogue local (installer, mettre à jour)".into()),
            ]);
        }
        let mut fichier = vec![
            item(PUBLISH, format!("✏️ Publier une idée sur {}", self.symbol_label())),
            item(PUBLISH_SNAP, format!("📸 Publier avec une image de la chart ({})", self.symbol_label())),
            item(EXPORT, "💾 Exporter mes idées (fichier)".into()),
            item(
                IMPORT,
                if doigt {
                    "📂 Importer des idées".into()
                } else {
                    format!("📂 Importer des idées ({FILE} dans Téléchargements)")
                },
            ),
            item(
                DELETE,
                if self.ui.ideas_delete {
                    "✅ Fini d'effacer".into()
                } else {
                    "🗑 Effacer des idées…".into()
                },
            ),
        ];
        if !doigt {
            fichier.push(MenuItem {
                id: -1,
                label: "🌐 Bientôt : partage par Nostr (rien n'est envoyé pour l'instant)".into(),
                enabled: false,
                ..Default::default()
            });
        }
        groups.extend([
            MenuGroup {
                title: "👥 COMMUNAUTÉ".into(),
                items: communaute,
                strip: false,
            },
            MenuGroup {
                title: format!("💡 IDÉES ({}) · gardées sur cet appareil", self.ui.ideas.len()),
                items: ideas,
                strip: false,
            },
            MenuGroup {
                title: String::new(),
                items: fichier,
                strip: false,
            },
        ]);
        groups
    }

    /// Ligne choisie : `true` = le menu reste ouvert (refait).
    pub(crate) fn community_pick(&mut self, id: i64, anchor: Rect) -> bool {
        match id {
            MOVERS => {
                self.ui.list = None;
                self.open_list(ListKind::Movers, anchor);
                false
            }
            ACADEMY => {
                self.ui.list = None;
                self.open_academy();
                false
            }
            MODULES => {
                self.ui.list = None;
                self.open_list(ListKind::Modules, anchor);
                false
            }
            MARKET => {
                self.ui.list = None;
                self.open_market();
                false
            }
            PUBLISH | PUBLISH_SNAP => {
                self.ui.idea_snap = id == PUBLISH_SNAP;
                let price = self.chart.bars.last().map(|b| format!("{}", (b.close * 100.0).round() / 100.0));
                let value = format!("long {} sl  tp  ", price.unwrap_or_default());
                self.ui.name_box = Some(NameBox::new(NameFor::Idea, &value, anchor, self.win_w()));
                false
            }
            EXPORT => {
                // 📸 les images vont dans le fichier (base64), pour que l'autre appareil les retrouve
                let full: Vec<Idea> = self
                    .ui
                    .ideas
                    .iter()
                    .map(|i| {
                        use base64::Engine;
                        let mut i = i.clone();
                        if i.snap {
                            i.image = Self::idea_image(&i).map(|b| b64().encode(b)).unwrap_or_default();
                        }
                        i
                    })
                    .collect();
                let body = serde_json::to_string_pretty(&full).unwrap_or_default();
                self.banner = Some(match platform::save_download(FILE, body.as_bytes()) {
                    Ok(dir) => format!(
                        "💾 {} idée(s) enregistrée(s) dans {dir}/{FILE} : envoie ce fichier à qui tu veux",
                        self.ui.ideas.len()
                    ),
                    Err(e) => format!("Écriture impossible ({e})"),
                });
                true
            }
            IMPORT => {
                let got = platform::read_download(FILE).and_then(|s| serde_json::from_str::<Vec<Idea>>(&s).ok());
                self.banner = Some(match got {
                    Some(list) => {
                        let n = self.merge_ideas(list);
                        self.save_ideas();
                        format!("📂 {n} idée(s) importée(s) (sans doublon)")
                    }
                    None => format!("Fichier absent ou invalide : mets {FILE} dans tes Téléchargements"),
                });
                true
            }
            DELETE => {
                self.ui.ideas_delete = !self.ui.ideas_delete;
                self.ui.idea_open = None;
                true
            }
            OPEN | SHARE | SAVE_IMG | SNAP_NOW | BACK => {
                let k = self.ui.idea_open.as_ref().and_then(|id| self.ui.ideas.iter().position(|i| &i.id == id));
                let Some(k) = k.filter(|_| id != BACK) else {
                    self.ui.idea_open = None;
                    return true;
                };
                let i = self.ui.ideas[k].clone();
                let name = format!("argos-idee-{}.png", safe_id(&i.id));
                match id {
                    OPEN => {
                        // 👁 rouvrir la chart de l'idée, à son unité ; le détail en bandeau
                        self.ui.list = None;
                        self.open_coin(i.coin.clone());
                        self.set_interval(Interval(i.interval));
                        self.banner = Some(share_text(&i));
                        return false;
                    }
                    SNAP_NOW => self.banner = Some(self.snap_idea(k)),
                    _ => {
                        self.banner = Some(match Self::idea_image(&i) {
                            None => "📸 Image introuvable sur cet appareil".into(),
                            Some(png) if id == SHARE => match platform::share_image(&name, &png, &share_text(&i)) {
                                Ok(to) => format!("📤 Image envoyée vers {to}"),
                                Err(e) => format!("Partage impossible ({e})"),
                            },
                            Some(png) => match platform::save_download(&name, &png) {
                                Ok(dir) => format!("💾 Image enregistrée : {dir}/{name}"),
                                Err(e) => format!("Écriture impossible ({e})"),
                            },
                        });
                    }
                }
                true
            }
            k if k >= IDEA0 => {
                let k = (k - IDEA0) as usize;
                if self.ui.ideas_delete {
                    if k < self.ui.ideas.len() {
                        let i = self.ui.ideas.remove(k);
                        if i.snap {
                            let _ = platform::write(&img_key(&i.id), ""); // l'image part avec l'idée
                        }
                        self.save_ideas();
                    }
                    return true;
                }
                // 🔎 le détail de l'idée en haut du menu (ouvrir, partager, télécharger l'image)
                self.ui.idea_open = self.ui.ideas.get(k).map(|i| i.id.clone());
                true
            }
            _ => true,
        }
    }

    /// Idée tapée dans la boîte : gardée si le sens est lisible.
    pub(crate) fn idea_typed(&mut self, value: &str) {
        let last = self.chart.bars.last().map(|b| b.close);
        let Some((long, entry, sl, tp, text)) = parse_idea(value, last) else {
            self.banner = Some("💡 Écris le sens d'abord : « long 76000 sl 75000 tp 80000 ton texte »".into());
            return;
        };
        let t = now_s();
        let idea = Idea {
            id: format!("{t}-{}", self.ui.ideas.len()),
            t,
            coin: if self.watch.selected.is_empty() {
                self.chart.source.key()
            } else {
                self.watch.selected.clone()
            },
            label: self.symbol_label(),
            interval: self.chart.interval.secs(),
            long,
            entry,
            sl,
            tp,
            text,
            ..Default::default()
        };
        self.banner = Some(format!("💡 Idée gardée : {}", idea.line()));
        self.ui.ideas.push(idea);
        self.save_ideas();
        if std::mem::take(&mut self.ui.idea_snap) {
            let msg = self.snap_idea(self.ui.ideas.len() - 1);
            self.banner = Some(format!("💡 Idée gardée · {msg}"));
        }
    }

    /// Ajoute des idées importées, sans doublon (même identifiant) ; le nombre d'ajoutées.
    pub(crate) fn merge_ideas(&mut self, list: Vec<Idea>) -> usize {
        let mut n = 0;
        for i in list.into_iter().filter(|i| !i.id.is_empty() && !i.coin.is_empty()) {
            if !self.ui.ideas.iter().any(|x| x.id == i.id) {
                let mut i = i;
                // 📸 l'image du fichier est rangée à part ; sans image lisible, l'idée arrive sans
                let png = {
                    use base64::Engine;
                    b64().decode(i.image.trim()).ok().filter(|b| b.starts_with(b"\x89PNG"))
                };
                i.snap = png.is_some_and(|b| Self::store_image(&i.id, &b));
                i.image.clear();
                self.ui.ideas.push(i);
                n += 1;
            }
        }
        self.ui.ideas.sort_by_key(|i| i.t);
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idee_tapee_lue() {
        let (long, e, sl, tp, text) = parse_idea("long 76 000 sl 75000 tp 80000 cassure du range", None).unwrap();
        assert!(long);
        assert_eq!((e, sl, tp), (Some(76.0), Some(75000.0), Some(80000.0)), "« 76 000 » : l'espace coupe le nombre");
        assert_eq!(text, "000 cassure du range");
        let (long, e, sl, tp, text) = parse_idea("short 2450,5 sl 2500 tp 2300 rejet de l'OB", None).unwrap();
        assert!(!long);
        assert_eq!((e, sl, tp, text.as_str()), (Some(2450.5), Some(2500.0), Some(2300.0), "rejet de l'OB"));
        let (_, e, ..) = parse_idea("long sl 1 tp 3", Some(2.0)).unwrap();
        assert_eq!(e, Some(2.0), "sans entrée : le dernier prix");
        assert!(parse_idea("juste un texte", None).is_none(), "sans sens : refusée");
        // mots français et anglais, prix avec virgule, ordre libre des sl / tp
        let (long, e, sl, tp, text) = parse_idea("vente 0,52 tp 0,40 stop 0,58 range", None).unwrap();
        assert!(!long);
        assert_eq!((e, sl, tp, text.as_str()), (Some(0.52), Some(0.58), Some(0.40), "range"));
        let (long, e, ..) = parse_idea("achat entrée 3990 sl 3970", Some(1.0)).unwrap();
        assert!(long && e == Some(3990.0), "« entrée » prime sur le dernier prix");
        let (_, _, sl, ..) = parse_idea("long 100 sl abc", None).unwrap();
        assert_eq!(sl, None, "sl sans nombre : gardé dans le texte");
        let long_text = format!("long 1 {}", "x".repeat(400));
        assert_eq!(parse_idea(&long_text, None).unwrap().4.chars().count(), 300, "texte coupé à 300");
        let i = Idea {
            long: true,
            entry: Some(100.0),
            sl: Some(90.0),
            tp: Some(130.0),
            label: "BTC".into(),
            interval: 1800,
            ..Default::default()
        };
        assert_eq!(i.rr(), Some(3.0));
        assert!(i.line().contains("R:R 3.0") && i.line().starts_with('▲'));
    }
}
