//! 🧭 Barre du haut, **comme Argos v1** — trois rangées au-dessus de tout le reste :
//! 1. **comme la maquette verre** : ⌂ · logo ARGOS ▾ · 📊 types de graphique · fx indicateurs · 🔔 alertes (pastille du
//!    nombre) · ▦ disposition · mise en page (« Argos ⌄ · ✓ Sauvegardé ») · 📤 Partager ; à droite 🔎 « Où est… ?
//!    Ctrl+K », la pilule du 🛡 Gardien PropFirm (« 0,8 % / 5 % ») et ⚙ réglages (☰ Appli, 🌐 Langue et le compte
//!    sont dans les menus ARGOS ▾ et ☰ Appli) ;
//! 2. bandeau des tickers : ★ (seulement les symboles à drapeau), $ (prix) / % (variation), les tickers de la watchlist
//!    (clic = ouvrir), › pour avancer ;
//! 3. fiche du symbole : pastille, nom ⌄, puis Prix, Variation 24 h, Volume 24 h, et pour Hyperliquid Oracle,
//!    Intérêt ouvert, Funding (avec le compte à rebours jusqu'au prochain paiement), et la source.
//!
//! 📱 Sur cell (`phone`), une seule rangée comme v1 : ARGOS ▾ · mise en page · Chart ⇅ · wallet · 🌐 · ⚙.
//!
//! Une seule table de zones (`layout`) sert au dessin et au clic.

use crate::palette as pal;
use crate::palette::IconCat;

/// 🎨 Couleur d'une icône dessinée à la main : celle de sa catégorie, plus claire au survol (comme `palette::icon_tint`).
fn icon_c(cat: IconCat, hot: bool) -> Rgb {
    let c = crate::palette::icon_color(cat);
    if hot {
        crate::palette::icon_hot(c)
    } else {
        c
    }
}
use crate::palette::{GREY, WHITE};
use crate::shapes::{lines, round_fill, round_stroke, Rect};
use crate::text::{Align, Graisse, Rgb, Text};
use crate::watchlist::coin_badge;
use crate::{fill, Theme};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;
use tiny_skia::Pixmap;

/// Hauteur totale de la barre (les 3 rangées et leurs marges) : la chart commence en dessous.
pub const HEIGHT: u32 = (ROW3.0 + ROW3.1 + 6.0) as u32;
/// 📱 Hauteur sur cell (une rangée).
pub const PHONE_HEIGHT: u32 = 48;
/// 🖥 Hauteur de la barre, les deux rangées du bas repliées (▾) : la 1re rangée seule.
pub const SANS_BANDEAUX_H: u32 = (ROW1.0 + ROW1.1 + 6.0) as u32;
const ROW1: (f64, f64) = (8.0, crate::formfactor::RANGEE1_PC);
/// 📱 La rangée du cell : une pilule de verre.
const PHONE_ROW: (f64, f64) = (4.0, 40.0);
const ROW2: (f64, f64) = (ROW1.0 + ROW1.1 + 6.0, crate::formfactor::RANGEE2_PC);
/// 🛡 Hauteur de la pastille du Gardien (maquette PC : 88 × 23 pour « 0,8 % / 5 % »).
const PILULE_H: f64 = ROW1.1 - 14.0;
/// Rangée de la fiche du symbole (maquette PC : 38 de haut, la chart commence 6 dessous).
const ROW3: (f64, f64) = (ROW2.0 + ROW2.1 + 6.0, crate::formfactor::RANGEE3_PC);
const M: f64 = 8.0;
/// Texte du nom du symbole, de l'unité et des valeurs de la fiche (maquette PC : 12,5).
const SYM_PX: f32 = 12.5;

/// 🫧 Fond opaque sous le verre d'une pastille (celui des anciennes rangées en verre).
const PANEL: Rgb = Rgb(0x0f, 0x11, 0x17);

/// 🫧 One element's own glass pill (Jee, 19 Nov: « garder les pastilles pilule verre »): the old row frame's fill
/// (opaque panel tone, then the glass) and its light outline, round ends — around ONE element, never a whole row,
/// so its text stays readable when the rows lie over the chart.
fn pastille_verre(pm: &mut Pixmap, r: Rect) {
    let k = crate::jetons::pilule(r);
    round_fill(pm, r, k, pal::tone(PANEL), 1.0);
    round_fill(pm, r, k, crate::jetons::VERRE.0, crate::jetons::VERRE.1);
    round_stroke(pm, r, k, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
}

/// Logo ARGOS (mot, PNG 234 × 72), décodé une seule fois.
fn wordmark() -> Option<&'static Pixmap> {
    static LOGO: OnceLock<Option<Pixmap>> = OnceLock::new();
    LOGO.get_or_init(|| Pixmap::decode_png(include_bytes!("../assets/argos-logo.png")).ok())
        .as_ref()
}

/// Logo ARGOS aux couleurs du thème `k` (bleu d'origine pour 🔷 Argos ; blanc, or ou vert pour les autres, en
/// gardant ses dégradés), préparé une seule fois par thème.
fn wordmark_for(k: usize) -> Option<&'static Pixmap> {
    static TINTED: OnceLock<Vec<Option<Pixmap>>> = OnceLock::new();
    let all = TINTED.get_or_init(|| {
        (0..Theme::NAMES.len())
            .map(|t| {
                let mut p = wordmark()?.clone();
                if let Some(c) = Theme::named(t).logo {
                    for px in p.pixels_mut() {
                        let a = px.alpha() as f32;
                        if a == 0.0 {
                            continue;
                        }
                        // éclat du pixel d'origine (dégradé) appliqué à la couleur du thème
                        let l = ((px.red().max(px.green()).max(px.blue()) as f32 / a) * 1.15).min(1.0);
                        let ch = |v: u8| (v as f32 * l * a / 255.0).round() as u8;
                        *px = tiny_skia::PremultipliedColorU8::from_rgba(ch(c.0), ch(c.1), ch(c.2), px.alpha())?;
                    }
                }
                Some(p)
            })
            .collect()
    });
    all.get(k).and_then(|p| p.as_ref())
}

/// Ce qui est gardé entre deux lancements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    /// Bandeau : variation en % (sinon le prix en $).
    pub pct: bool,
    /// ★ : seulement les symboles à drapeau.
    pub flagged: bool,
    /// 🌐 Format des nombres (index dans `viewport::NUMBER_FORMATS`).
    pub number_format: usize,
    /// 🌐 Langue (index dans `i18n::CATALOG` ; faites dans le code : `i18n::DONE`, les autres : packs téléchargés).
    pub language: usize,
    /// Mises en page ▸ Sauvegarde automatique.
    pub auto_save: bool,
    /// Nom de la mise en page (onglet).
    pub layout_name: String,
    /// 🎨 Thème (index dans `Theme::NAMES`).
    pub theme: usize,
    /// 🫧 Style verre par-dessus le thème (menus et fenêtres flottantes translucides et floutées).
    pub glass: bool,
    /// ⌂ Ouvrir l'Accueil au lancement.
    pub accueil: bool,
    /// 📱 Cell : barre du haut / du bas cachée par sa poignée (comme les barres latérales du PC) ; gardé.
    #[serde(default)]
    pub cell_haut_cache: bool,
    #[serde(default)]
    pub cell_bas_cache: bool,
    /// 🫥 PC : estomper la rangée du haut et la barre de gauche quand on travaille sur la chart (maquette).
    #[serde(default = "oui")]
    pub estomper: bool,
    /// ⌂ Profil de trader (index dans `accueil::PROFILS`).
    pub profil: usize,
    /// ⌂ Les 3 écrans du premier lancement ont été vus (📐 interface-2 : relu depuis « interface-2.json », que les
    /// anciennes versions ne réécrivent pas).
    pub bienvenue_vue: bool,
    /// 🫧 Réglages déjà passés au style verre (verre-1). Des réglages gardés d'avant (le verre y était éteint par
    /// défaut, `glass: false`) rallument le verre une seule fois ; ensuite le choix de l'utilisateur est respecté.
    /// Absent d'un fichier : `false` (pas le défaut de la structure).
    #[serde(default)]
    pub verre_v1: bool,
    /// ⌂ Réglages déjà passés à « l'Accueil seulement s'il était ouvert » (verre-2) : des réglages d'avant avaient
    /// « Accueil au lancement » allumé par défaut (il revenait à chaque rechargement) ; il s'éteint une seule fois.
    #[serde(default)]
    pub accueil_v2: bool,
    /// 🔔 « Lignes d'alerte » (right-click on an alert line): the full-width dashed lines; unchecked, each alert keeps
    /// a short form glued to the price scale.
    #[serde(default = "oui")]
    pub lignes_alerte: bool,
    /// 🔔 « Modifier la couleur des alertes… »: one colour « #rrggbb » for all alert lines; empty: as before.
    #[serde(default)]
    pub couleur_alertes: String,
    /// 🔊 « Son des alertes » (Réglages): the default sound of alerts, a name of `argos_core::alerts::SONS`.
    #[serde(default = "son_defaut")]
    pub son_alertes: String,
    /// 🔊 The alerts' volume, 0 – 100.
    #[serde(default = "volume_defaut")]
    pub volume_alertes: u8,
}

/// Valeur par défaut `true` d'un réglage ajouté plus tard (absent d'un ancien fichier).
fn oui() -> bool {
    true
}

/// 🔊 Default alert sound for settings saved before sounds existed.
fn son_defaut() -> String {
    argos_core::alerts::SON_DEFAUT.into()
}

/// 🔊 Default alert volume for settings saved before sounds existed.
fn volume_defaut() -> u8 {
    argos_core::alerts::VOLUME_DEFAUT
}

impl Prefs {
    /// ⌂ Éteint une fois « Accueil au lancement » des réglages d'avant verre-2 ; `true` si ça change.
    pub fn passer_accueil_v2(&mut self) -> bool {
        if self.accueil_v2 {
            return false;
        }
        (self.accueil, self.accueil_v2) = (false, true);
        true
    }

    /// ⌂ Does the Accueil open at launch? The very first launch, the « Accueil au lancement » setting on, or
    /// `open_in_this_tab`: it was open in this same tab before a reload (F5). That last state belongs to the tab only
    /// (never in the shared settings: an Accueil left open on the phone or in another tab used to come back here).
    pub fn accueil_au_lancement(&self, open_in_this_tab: bool) -> bool {
        !self.bienvenue_vue || open_in_this_tab || self.accueil
    }

    /// 🫧 Passe des réglages d'avant verre-1 au style verre (une seule fois) ; `true` si ça change (à réécrire).
    pub fn passer_au_verre(&mut self) -> bool {
        if self.verre_v1 {
            return false;
        }
        (self.glass, self.verre_v1) = (true, true);
        true
    }
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            pct: false,
            flagged: false,
            number_format: 0,
            language: 0,
            auto_save: true,
            layout_name: "Argos".into(),
            theme: Theme::CLASSIC,
            glass: true,
            accueil: false,
            cell_haut_cache: false,
            cell_bas_cache: false,
            estomper: true,
            profil: 0,
            bienvenue_vue: false,
            verre_v1: true,
            accueil_v2: true,
            lignes_alerte: true,
            couleur_alertes: String::new(),
            son_alertes: son_defaut(),
            volume_alertes: volume_defaut(),
        }
    }
}

/// Un ticker du bandeau.
#[derive(Debug, Clone, PartialEq)]
pub struct Tick {
    /// Ligne de la watchlist (pour l'ouvrir).
    pub coin: String,
    pub label: String,
    /// Prix écrit (vide : pas de prix).
    pub price: String,
    /// Variation 24 h en %.
    pub pct: Option<f64>,
}

/// Un chiffre de la fiche du symbole.
#[derive(Debug, Clone, PartialEq)]
pub struct Stat {
    pub label: String,
    pub value: String,
    /// Couleur du chiffre (vert / rouge) ; `None` = blanc.
    pub color: Option<Rgb>,
}

/// Résultat d'un clic.
#[derive(Debug, Clone, PartialEq)]
pub enum TopClick {
    None,
    /// ▾ Les deux rangées (tickers, fiche du symbole) se replient ; « Afficher prix et symbole » les rouvre.
    ReplierBandeaux,
    /// 🔎 Ouvrir la recherche Ctrl + K.
    Recherche,
    /// 📊 Types de graphique, sous ce bouton.
    Style(Rect),
    /// fx Indicateurs, sous ce bouton.
    Indicateurs(Rect),
    /// 🔔 Alertes, sous ce bouton.
    Alertes(Rect),
    /// 📤 Partager une image de la chart.
    Partager,
    /// 🛡 Gardien PropFirm : ouvrir sa carte (Trading).
    Gardien,
    /// 🛟 Support: open the tickets panel.
    Support,
    /// Réglage de la barre changé (redessiner, sauvegarder).
    Changed,
    /// Ouvrir ce symbole (ticker du bandeau).
    Open(String),
    /// Nom du symbole : choisir un autre symbole (la watchlist).
    Symbol,
    /// ⌂ Ouvrir l'Accueil.
    Home,
    /// ⏱ Bouton de l'unité de temps (à droite du symbole) : ouvrir le menu des intervalles sous ce bouton.
    Intervals(Rect),
    /// ⏱ Raccourci favori : passer à cette unité de temps (secondes).
    Interval(i64),
    /// Ouvrir ce menu / cette fenêtre sous son bouton (coordonnées de la fenêtre de l'app).
    Menu(TopMenu, Rect),
}

/// Menus et fenêtres de la barre du haut (comme Argos v1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopMenu {
    /// Argos ▾.
    Argos,
    /// Mises en page ▾ (et l'onglet).
    Layouts,
    /// 🌐 Langue et format des nombres.
    Lang,
    /// ⚙ Réglages.
    Settings,
    /// 🪟 Splits (plusieurs charts).
    Split,
    /// ☰ Appli : mises à jour, installer, copier l'URL, zoom, plein écran, vider le cache, infos (comme le menu ☰ des
    /// applis installées).
    App,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Hit {
    /// ⌂ Accueil (en tête de la barre).
    Home,
    /// ▾ Replier les deux rangées (tickers, fiche du symbole), maquette PC.
    Replier,
    /// 🔎 Recherche « Où est… ? » (Ctrl + K).
    Recherche,
    Logo,
    Layout,
    /// 📊 fx 🔔 📤 et la pilule du Gardien (maquette verre).
    Style,
    Indic,
    Alertes,
    Partager,
    Gardien,
    /// 🛟 Support, left of « Où est… ? ».
    Support,
    Settings,
    Star,
    Usd,
    Pct,
    Tick(usize),
    Next,
    Symbol,
    /// ⏱ Unité de temps courante (ouvre le menu) et raccourcis favoris.
    Iv,
    Fav(usize),
    Split,
    App,
}

/// La barre du haut.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TopBar {
    pub prefs: Prefs,
    /// Tickers du bandeau (la watchlist affichée) et le 1er affiché.
    pub tape: Vec<Tick>,
    pub tape_from: usize,
    /// Symbole ouvert : sa ligne de watchlist (logo), son nom, ses chiffres, sa source.
    pub coin: String,
    pub symbol: String,
    pub stats: Vec<Stat>,
    pub source: String,
    /// Nom de la mise en page (onglet).
    pub layout: String,
    /// 🪟 Disposition des charts (clé de `splits::cells`).
    pub split: String,
    /// 📱 Écran de cell : une seule rangée.
    pub phone: bool,
    /// ✨ Compte Argos de cet appareil (`@nom#tag`) : remplace « Connect » sur le bouton ; vide = pas de compte.
    pub account: String,
    /// 💾 Une sauvegarde est en cours (« Sauvegarde… ») ; sinon « ✓ Sauvegardé », sous le nom de la mise en page.
    pub saving: bool,
    /// ⏱ Unité de temps courante (secondes, nom court « 4h ») et favoris ★ (secondes, nom court), à droite du
    /// symbole, comme TradingView.
    pub iv: (i64, String),
    pub iv_favs: Vec<(i64, String)>,
    /// 🔔 Nombre d'alertes du symbole ouvert (pastille rouge ; 0 = pas de pastille).
    pub alertes: usize,
    /// 🖥 Les deux rangées du bas (tickers, fiche du symbole) repliées par ▾ (gardé dans interface-2.json).
    pub sans_bandeaux: bool,
    /// 🖥 Rangées repliées montrées par-dessus la chart (souris sur leur bande) : dessinées, la place reste à 58.
    pub survol_bandeaux: bool,
    /// 🖥🧘 Laid over the chart (Zen, brought back at the top edge): only its pills, no background around them.
    pub flottante: bool,
    /// 🖥 Bords gauche et droit des rangées 2 et 3 ouvertes au survol (entre le rail et le volet ou la colonne) ;
    /// `None` : toute la largeur.
    pub rangs: Option<(f64, f64)>,
    /// 🛡 Pilule du Gardien PropFirm : (« 0,8 % / 5 % », couleur de son niveau) ; `None` = Gardien éteint.
    pub gardien: Option<(String, Rgb)>,
    /// 🛟 Argos replies not read yet in Support (red dot on the button; 0 = nothing).
    pub support: usize,
    /// 🛟 Right edge of what is already placed in the free space of the 1st row (the DÉMO pill), given by the app:
    /// the Support button never goes to its left.
    pub support_gauche: f64,
}

/// 🖥 Largeur de la fenêtre à partir de laquelle la 1re rangée montre ses étiquettes (maquette : `@media
/// (max-width:1500px){#haut .lbl{display:none}}`).
pub const ETIQUETTES_DES: f64 = 1500.0;
/// Couleur des étiquettes de la 1re rangée (le texte de la maquette).
const TEXT_LBL: Rgb = crate::jetons::TEXTE;
const ETIQ_PX: f32 = 13.0;

/// 🖥 L'étiquette d'un bouton de la 1re rangée (maquette `.lbl`).
/// 🛟 The Support button text.
const SUPPORT: &str = "🛟 Support";

fn etiquette(h: Hit) -> Option<&'static str> {
    Some(match h {
        Hit::Style => "Bougies",
        Hit::Indic => "Indicateurs",
        Hit::Alertes => "Alertes",
        Hit::Split => "Split",
        _ => return None,
    })
}

/// Le texte sous le nom de la mise en page.
pub fn save_label(saving: bool) -> &'static str {
    if saving {
        "Sauvegarde…"
    } else {
        "✓ Sauvegardé"
    }
}

impl TopBar {
    /// Tickers montrés (★ : seulement ceux de `flagged`).
    fn shown<'a>(&'a self, flagged: &'a dyn Fn(&str) -> bool) -> impl Iterator<Item = (usize, &'a Tick)> + 'a {
        self.tape
            .iter()
            .enumerate()
            .filter(move |(_, t)| !self.prefs.flagged || flagged(&t.coin))
            .skip(self.tape_from)
    }

    /// 🖥 Bords gauche et droit des rangées 2 et 3 (largeur `w`).
    fn bords(&self, w: f64) -> (f64, f64) {
        match self.rangs.filter(|_| self.sans_bandeaux && self.survol_bandeaux) {
            Some((g, d)) => (g.max(M), d.min(w - M).max(g + 200.0)),
            None => (M, w - M),
        }
    }

    /// Hauteur de la barre (cell : une rangée).
    pub fn height(&self) -> u32 {
        if self.phone {
            PHONE_HEIGHT
        } else if self.sans_bandeaux {
            SANS_BANDEAUX_H
        } else {
            HEIGHT
        }
    }

    /// Zones (coordonnées de la barre, largeur `w`).
    /// 📡 Markets of the ticker band shown at width `w` (none when the band is folded away, or on a phone: one row).
    pub fn ticks_visibles(&self, w: f64, text: &Text, flagged: &dyn Fn(&str) -> bool) -> Vec<String> {
        if self.phone || (self.sans_bandeaux && !self.survol_bandeaux) {
            return vec![];
        }
        self.layout(w, text, flagged)
            .into_iter()
            .filter_map(|(_, h)| match h {
                Hit::Tick(i) => self.tape.get(i).map(|t| t.coin.clone()),
                _ => None,
            })
            .collect()
    }

    fn layout(&self, w: f64, text: &Text, flagged: &dyn Fn(&str) -> bool) -> Vec<(Rect, Hit)> {
        if self.phone {
            // une pilule de verre (maquette tel-25) : ⌂ · ARGOS ▾ · mise en page (coupée avec …) · à droite 🔔 🔎 ⚙ ☰ ;
            // les largeurs sont mesurées, rien ne se chevauche (🌐 Langue : ☰ Appli ; Wallet et compte : ARGOS ▾)
            let (y, h) = (PHONE_ROW.0 + 2.0, PHONE_ROW.1 - 4.0);
            let mut out = vec![(Rect::new(M + 2.0, y, 30.0, h), Hit::Home), (Rect::new(M + 34.0, y, 72.0, h), Hit::Logo)];
            // à droite, comme la maquette cell : 🔔 · ▦ · 📤 · 🔎 · 🗺 (le plan : ☰ Appli, tout Argos)
            let mut r = w - M - 4.0;
            for (lw, hit) in [
                (32.0, Hit::App),
                (32.0, Hit::Recherche),
                (32.0, Hit::Partager),
                (40.0, Hit::Split),
                (32.0, Hit::Alertes),
            ] {
                r -= lw;
                out.push((Rect::new(r, y, lw, h), hit));
            }
            let x = M + 110.0;
            let name_w = text.width(&self.layout, 12.0) as f64 + 28.0;
            let lw = name_w.min(r - 6.0 - x);
            if lw >= 40.0 {
                out.push((Rect::new(x, y, lw, h), Hit::Layout));
            }
            return out;
        }
        let (y1, h1) = ROW1;
        let mut out = vec![
            (Rect::new(M + 6.0, y1 + 7.0, 32.0, h1 - 14.0), Hit::Home),
            (Rect::new(M + 44.0, y1 + 6.0, 112.0, h1 - 12.0), Hit::Logo),
        ];
        // 📊 fx 🔔 ▦ (maquette verre), puis la mise en page et 📤
        let mut x = M + 172.0;
        // (maquette `#haut .lbl` : les étiquettes « Bougies », « Indicateurs »… au-dessus de 1500 de large, icônes
        // seules en dessous)
        for (wd, hit) in [(34.0, Hit::Style), (34.0, Hit::Indic), (34.0, Hit::Alertes), (46.0, Hit::Split)] {
            let wd = match etiquette(hit).filter(|_| w > ETIQUETTES_DES) {
                Some(l) => wd + text.width(l, ETIQ_PX) as f64 + 8.0,
                None => wd,
            };
            out.push((Rect::new(x, y1 + 7.0, wd, h1 - 14.0), hit));
            x += wd + 4.0;
        }
        x += 14.0;
        let lw = text.width(save_label(false), 11.0).max(text.width(&self.layout, 13.5) + 18.0).min(220.0) as f64 + 16.0;
        let gauche = vec![
            (Rect::new(x, y1 + 4.0, lw, h1 - 8.0), Hit::Layout),
            (Rect::new(x + lw + 8.0, y1 + 7.0, 34.0, h1 - 14.0), Hit::Partager),
        ];
        x += lw + 8.0 + 34.0;
        // à droite (de droite à gauche) : ⚙, la pilule du Gardien, 🔎
        let mut r = w - M - 8.0;
        r -= 32.0;
        out.push((Rect::new(r, y1 + 7.0, 32.0, h1 - 14.0), Hit::Settings));
        r -= 8.0;
        // 🛡 pastille de la maquette (badgepf) : 23 de haut, texte de 11,5, bouclier à gauche (88 de large pour « 0,8 % / 5 % »)
        let gw = (Text::graisse(Graisse::Medium).width(self.gardien.as_ref().map_or("Gardien", |g| g.0.as_str()), 11.5) as f64 + 27.0).round();
        if r - gw > x + 8.0 {
            r -= gw;
            out.push((Rect::new(r, y1 + ((h1 - PILULE_H) / 2.0).round(), gw, PILULE_H), Hit::Gardien));
            r -= 10.0;
        }
        // 🔎 « Où est… ? Ctrl+K » : s'il reste au moins 170 px après 📤
        let libre = r - (x + 16.0);
        if libre >= 170.0 {
            let sw = libre.min(300.0);
            out.push((Rect::new(r - sw, y1 + 7.0, sw, h1 - 14.0), Hit::Recherche));
            // 🛟 Support just to its left, in the remaining room (after 📤 and the DÉMO pill): without moving anything;
            // « 🛟 Support », otherwise the 🛟 circle alone, otherwise nothing (🔎 finds it)
            let fin = r - sw - 8.0;
            let debut = (x + 16.0).max(self.support_gauche + 8.0);
            let plein = (text.width(SUPPORT, 13.0) as f64 + 24.0).round();
            let h = h1 - 14.0;
            if let Some(sw) = [plein, h].into_iter().find(|w| fin - debut >= *w) {
                out.push((Rect::new(fin - sw, y1 + 7.0, sw, h), Hit::Support));
            }
        }
        // la mise en page et 📤 seulement s'ils tiennent avant la droite (tablette en portrait, petite fenêtre)
        out.extend(gauche.into_iter().filter(|(q, _)| q.x + q.w <= r));
        // bandeau
        let (y2, h2) = ROW2;
        let (g, d) = self.bords(w);
        let mut x = g + 8.0;
        for (wd, hit) in [(22.0, Hit::Star), (24.0, Hit::Usd), (24.0, Hit::Pct)] {
            out.push((Rect::new(x, y2 + 5.0, wd, h2 - 10.0), hit));
            x += wd + 4.0;
        }
        x += 8.0;
        let next = Rect::new(d - 30.0, y2 + 4.0, 24.0, h2 - 8.0);
        for (i, t) in self.shown(flagged) {
            let tw = self.tick_w(t, text);
            if x + tw > next.x - 4.0 {
                break;
            }
            out.push((Rect::new(x, y2 + 4.0, tw, h2 - 8.0), Hit::Tick(i)));
            x += tw + 6.0;
        }
        out.push((next, Hit::Next));
        // fiche du symbole
        let (y3, h3) = ROW3;
        let sw = Text::graisse(Graisse::Bold).width(&self.symbol, SYM_PX) as f64 + 39.0 + 24.0;
        out.push((Rect::new(g + 6.0, y3 + 4.0, sw, h3 - 8.0), Hit::Symbol));
        // ⏱ l'unité de temps (« 4h ⌄ » : le menu des intervalles), puis les favoris en raccourcis tant qu'il y a la place
        if !self.iv.1.is_empty() {
            let mut x = g + 6.0 + sw + 8.0;
            let bw = text.width(&self.iv.1, SYM_PX) as f64 + 26.0;
            out.push((Rect::new(x, y3 + 4.0, bw, h3 - 8.0), Hit::Iv));
            x += bw + 6.0;
            let room = if self.phone { 0.0 } else { (g + (d - g) * 0.55).max(x) };
            for (i, (_, l)) in self.iv_favs.iter().enumerate() {
                let cw = text.width(l, 11.5) as f64 + 14.0;
                if x + cw > room {
                    break;
                }
                out.push((Rect::new(x, y3 + 7.0, cw, h3 - 14.0), Hit::Fav(i)));
                x += cw + 2.0;
            }
        }
        // 🖥 ▾ au bout de la fiche : replier les deux rangées ; repliées, plus de zones dessous
        if !self.phone {
            if self.sans_bandeaux && !self.survol_bandeaux {
                out.retain(|(r, _)| r.y < ROW2.0);
            } else {
                out.push((Rect::new(d - 30.0, y3 + (h3 - 26.0) / 2.0, 26.0, 26.0), Hit::Replier));
            }
        }
        out
    }

    fn tick_text(&self, t: &Tick) -> String {
        match (self.prefs.pct, t.pct) {
            (true, Some(p)) => format!("{p:+.2} %"),
            (false, _) if !t.price.is_empty() => t.price.clone(),
            _ => "–".into(),
        }
    }

    fn tick_w(&self, t: &Tick, text: &Text) -> f64 {
        // maquette PC : nom gras de 12, valeur de 12
        Text::graisse(Graisse::Bold).width(&t.label, 12.0) as f64 + text.width(&self.tick_text(t), 12.0) as f64 + 18.0
    }

    /// Y a-t-il un bouton de la barre en `(x, y)` (largeur `w`) ?
    /// 🖥 L'espace vide de la 1re rangée (largeur `w`) entre le groupe de gauche (… mise en page, 📤) et celui de droite
    /// (🔎, 🛡, ⚙) : bords gauche et droit, et le haut et la hauteur de la rangée. La pilule DÉMO s'y pose.
    pub fn espace_libre(&self, w: f64, text: &Text) -> Option<(f64, f64, f64, f64)> {
        if self.phone {
            return None;
        }
        let (y1, h1) = ROW1;
        // (🛟 Support is placed after the DÉMO pill, in what remains: it does not count here)
        let z: Vec<Rect> = self
            .layout(w, text, &|_: &str| false)
            .into_iter()
            .filter(|(r, h)| r.y < y1 + h1 && *h != Hit::Support)
            .map(|(r, _)| r)
            .collect();
        let x0 = z.iter().filter(|r| r.cx() < w / 2.0).map(|r| r.x + r.w).fold(M, f64::max);
        let x1 = z.iter().filter(|r| r.cx() >= w / 2.0).map(|r| r.x).fold(w - M, f64::min);
        (x1 - x0 > 40.0).then_some((x0 + 8.0, x1 - 8.0, y1, h1))
    }

    /// 🔎 The symbol pill (window, bar `w` wide): the market panel opens under it.
    pub fn symbole_rect(&self, w: f64, text: &Text) -> Option<Rect> {
        self.layout(w, text, &|_| false).into_iter().find(|(_, h)| *h == Hit::Symbol).map(|(r, _)| r)
    }

    pub fn zone_en(&self, x: f64, y: f64, w: f64, text: &Text) -> bool {
        self.layout(w, text, &|_: &str| false).iter().any(|(r, _)| r.contains(x, y))
    }

    /// Clic (coordonnées de la barre, largeur `w`) ; `flagged` : le symbole a-t-il un drapeau ?
    pub fn click(&mut self, x: f64, y: f64, w: f64, text: &Text, flagged: &dyn Fn(&str) -> bool) -> TopClick {
        // (🔎 1st row: click zones at least 28 high, centred on the drawn button; the row has a single line of buttons)
        let zone = |r: &Rect| {
            let h = if r.y < ROW2.0 { r.h.max(28.0) } else { r.h };
            Rect::new(r.x, r.cy() - h / 2.0, r.w, h)
        };
        let Some((r, hit)) = self.layout(w, text, flagged).into_iter().find(|(r, _)| zone(r).contains(x, y)) else {
            return TopClick::None;
        };
        match hit {
            Hit::Home => TopClick::Home,
            Hit::Logo => TopClick::Menu(TopMenu::Argos, r),
            Hit::Layout => TopClick::Menu(TopMenu::Layouts, r),
            Hit::Style => TopClick::Style(r),
            Hit::Indic => TopClick::Indicateurs(r),
            Hit::Alertes => TopClick::Alertes(r),
            Hit::Partager => TopClick::Partager,
            Hit::Gardien => TopClick::Gardien,
            Hit::Support => TopClick::Support,
            Hit::Settings => TopClick::Menu(TopMenu::Settings, r),
            Hit::App => TopClick::Menu(TopMenu::App, r),
            Hit::Split => TopClick::Menu(TopMenu::Split, r),
            Hit::Symbol => TopClick::Symbol,
            Hit::Recherche => TopClick::Recherche,
            Hit::Iv => TopClick::Intervals(r),
            Hit::Fav(i) => self.iv_favs.get(i).map_or(TopClick::None, |f| TopClick::Interval(f.0)),
            Hit::Tick(i) => TopClick::Open(self.tape[i].coin.clone()),
            Hit::Star => {
                self.prefs.flagged = !self.prefs.flagged;
                self.tape_from = 0;
                TopClick::Changed
            }
            Hit::Usd | Hit::Pct => {
                self.prefs.pct = hit == Hit::Pct;
                TopClick::Changed
            }
            Hit::Replier => TopClick::ReplierBandeaux,
            Hit::Next => {
                // avance d'une page ; au bout, revient au début
                let n = self.tape.iter().filter(|t| !self.prefs.flagged || flagged(&t.coin)).count();
                let page = self.layout(w, text, flagged).iter().filter(|(_, h)| matches!(h, Hit::Tick(_))).count().max(1);
                self.tape_from = if self.tape_from + page >= n { 0 } else { self.tape_from + page };
                TopClick::Changed
            }
        }
    }

    /// Dessine la barre (largeur `w`, hauteur `HEIGHT`) ; `mouse` en coordonnées de la barre.
    pub fn draw(&self, text: &Text, w: u32, mouse: Option<(f64, f64)>, logos: &HashMap<String, Pixmap>, flagged: &dyn Fn(&str) -> bool, th: &Theme) -> Pixmap {
        let wf = w as f64;
        // (🖥 rangées repliées montrées au survol : l'image a toute sa hauteur, posée par-dessus la chart)
        let hi = if !self.phone && self.sans_bandeaux && self.survol_bandeaux {
            HEIGHT
        } else {
            self.height()
        };
        let mut pm = crate::px::new(w.max(1), hi).expect("taille de la barre du haut");
        // (🫧 the bar's background only behind its place: the rows opened on hover over the chart are their pills
        // alone — no rectangle around them, Jee 17 Nov; 🧘 laid over the chart in Zen: no background at all)
        if !self.flottante {
            fill(&mut pm, 0.0, 0.0, wf, self.height() as f64, th.chrome, 1.0);
        }
        // 🚫 No frame around the rows (Jee's rule, 16 Nov, asked five times; it overrides the maquette's `.verre`
        // rows): no background, no border, no separator around or between them, on PC, tablet and phone; only the
        // pills themselves, laid directly on the bar's place. Tests: `rangees_sans_cadre_partout`.
        let (_, d) = self.bords(wf);
        // 🫧 Row 1 (⌂ ARGOS ▾, Bougies, Indicateurs, Alertes, Split…) on its glass pill again, as it was before
        // 51ef2bef took it away (Jee, 20 Nov: « retirée sans que je le demande ») : the same style (opaque panel tone,
        // the glass, the light outline, round ends), row 1 only; rows 2 and 3 keep one pill per element (a4e52b74)
        if !self.phone && !self.flottante {
            let (y, h) = ROW1;
            pastille_verre(&mut pm, Rect::new(M, y, wf - 2.0 * M, h));
        }
        let hover = |r: &Rect| mouse.is_some_and(|(x, y)| r.contains(x, y));
        // 🖥 boutons à étiquette (plus larges que 34) : l'icône à gauche, l'étiquette après
        let ic_x = |r: &Rect| if r.w > 40.0 { r.x + 17.0 } else { r.cx() };
        let lbl = |pm: &mut Pixmap, r: &Rect, h: Hit| {
            if let Some(l) = etiquette(h).filter(|_| r.w > 40.0) {
                text.draw(pm, l, ((r.x + 30.0) as f32, (r.cy() + 4.5) as f32), ETIQ_PX, TEXT_LBL, Align::Left);
            }
        };
        let zones = self.layout(wf, text, flagged);
        // 🫧 each element of rows 2 and 3 (and of row 1 in Zen, laid over the chart) on its own glass pill; the
        // quick timeframe buttons share one pill (their group); nothing between the pills (Jee, 19 Nov)
        let favs = zones.iter().filter(|(_, h)| matches!(h, Hit::Fav(_))).map(|(r, _)| *r).reduce(|a, b| {
            let (x0, x1) = (a.x.min(b.x), (a.x + a.w).max(b.x + b.w));
            Rect::new(x0, a.y, x1 - x0, a.h)
        });
        for (r, hit) in &zones {
            let sur_la_chart = r.y >= ROW2.0 || (self.flottante && !self.phone);
            if !sur_la_chart || matches!(hit, Hit::Fav(_)) {
                continue;
            }
            pastille_verre(&mut pm, *r);
        }
        if let Some(f) = favs {
            let (y3, h3) = ROW3;
            pastille_verre(&mut pm, Rect::new(f.x - 3.0, y3 + 4.0, f.w + 6.0, h3 - 8.0));
        }
        for (r, hit) in &zones {
            let r = *r;
            let mid = (r.cy() + 4.5) as f32;
            if hover(&r) && !matches!(hit, Hit::Gardien | Hit::Recherche | Hit::Support | Hit::Usd | Hit::Pct) {
                round_fill(&mut pm, r, r.h / 2.0, crate::jetons::VERRE2.0, crate::jetons::VERRE2.1);
            }
            match *hit {
                // 📱 la loupe : la recherche Ctrl + K au doigt
                Hit::Recherche if self.phone => {
                    crate::phosphor::draw(&mut pm, "magnifying-glass", r.cx(), r.cy(), 18, icon_c(IconCat::Info, hover(&r)));
                }
                Hit::Recherche => {
                    let j = crate::jetons::CHAMP;
                    round_fill(&mut pm, r, r.h / 2.0, j.0, if hover(&r) { 0.1 } else { j.1 });
                    round_stroke(&mut pm, r, r.h / 2.0, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
                    crate::phosphor::draw(&mut pm, "magnifying-glass", r.x + 18.0, r.cy(), 15, crate::jetons::DIM);
                    text.draw(&mut pm, "Où est… ?", ((r.x + 34.0) as f32, mid), 13.0, crate::jetons::DIM, Align::Left);
                    if crate::actions::montrer_raccourcis() {
                        text.draw(&mut pm, "Ctrl+K", ((r.x + r.w - 14.0) as f32, mid), 12.0, crate::jetons::DIM, Align::Right);
                    }
                }
                Hit::Support => {
                    let j = crate::jetons::CHAMP;
                    round_fill(&mut pm, r, r.h / 2.0, j.0, if hover(&r) { 0.1 } else { j.1 });
                    round_stroke(&mut pm, r, r.h / 2.0, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
                    // (🛟 circle alone when room is short)
                    let s = if r.w > r.h + 1.0 { SUPPORT } else { "🛟" };
                    text.draw(&mut pm, s, (r.cx() as f32, mid), 13.0, crate::jetons::TEXTE, Align::Center);
                    if self.support > 0 {
                        round_fill(&mut pm, Rect::new(r.x + r.w - 11.0, r.y + 2.0, 9.0, 9.0), 4.5, crate::jetons::BAS, 1.0);
                    }
                }
                Hit::Home => crate::accueil::maison(&mut pm, r.cx(), r.cy(), icon_c(IconCat::Settings, hover(&r))),
                Hit::Logo => {
                    if let Some(l) = wordmark_for(crate::theme_index()) {
                        let k = ((r.h - 4.0) / l.height() as f64).min((r.w - 14.0) / l.width() as f64) as f32;
                        let at_y = r.cy() - k as f64 * l.height() as f64 / 2.0;
                        let at = tiny_skia::Transform::from_row(k, 0.0, 0.0, k, (r.x + 2.0) as f32, at_y as f32);
                        let paint = tiny_skia::PixmapPaint {
                            quality: tiny_skia::FilterQuality::Bicubic,
                            ..Default::default()
                        };
                        pm.draw_pixmap(0, 0, l.as_ref(), &paint, crate::px::t().pre_concat(at), None);
                    } else {
                        text.draw(&mut pm, "ARGOS", ((r.x + 4.0) as f32, mid), 18.0, crate::palette::cyan(), Align::Left);
                    }
                    chevron(&mut pm, r.x + r.w - 8.0, r.cy(), GREY);
                }
                Hit::Layout if !self.phone => {
                    // « Argos ⌄ » et « ✓ Sauvegardé » dessous (maquette verre)
                    let gras = Text::graisse(Graisse::Medium);
                    let nom = gras.fit(&self.layout, 13.5, r.w - 30.0);
                    let nw = gras.width(&nom, 13.5) as f64;
                    gras.draw(
                        &mut pm,
                        &nom,
                        ((r.x + 8.0) as f32, (r.cy() - 1.0) as f32),
                        13.5,
                        crate::jetons::TEXTE,
                        Align::Left,
                    );
                    chevron(&mut pm, r.x + 16.0 + nw, r.cy() - 5.0, GREY);
                    text.draw(
                        &mut pm,
                        &text.fit(save_label(self.saving), 11.0, r.w - 16.0),
                        ((r.x + 8.0) as f32, (r.cy() + 12.0) as f32),
                        11.0,
                        if self.saving { pal::accent() } else { crate::jetons::DIM },
                        Align::Left,
                    );
                }
                // 📱 maquette cell : « Argos ✓ » en petit et gris (la mise en page, sauvegardée)
                Hit::Layout if self.phone => {
                    let t = format!("{} {}", self.layout, if self.saving { "…" } else { "✓" });
                    let t = text.fit(&t, 12.0, r.w - 8.0);
                    text.draw(&mut pm, &t, ((r.x + 4.0) as f32, mid), 12.0, crate::jetons::DIM, Align::Left);
                }
                Hit::Layout => {
                    // écran + ⌄
                    let screen: [(f32, f32); 5] = [(-8.0, -6.0), (8.0, -6.0), (8.0, 4.0), (-8.0, 4.0), (-8.0, -6.0)];
                    lines(
                        &mut pm,
                        r.x + 14.0,
                        r.cy(),
                        &[&screen, &[(-3.0, 8.0), (3.0, 8.0)]],
                        icon_c(IconCat::Settings, hover(&r)),
                        1.4,
                    );
                    chevron(&mut pm, r.x + 32.0, r.cy(), GREY);
                    let (big, small, room) = if self.phone { (12.5, 10.0, r.w - 46.0) } else { (13.5, 11.0, 220.0) };
                    text.draw(
                        &mut pm,
                        &text.fit(&self.layout, big, room),
                        ((r.x + 44.0) as f32, (r.cy() - 1.0) as f32),
                        big,
                        WHITE,
                        Align::Left,
                    ); // nom de la mise en page
                    text.draw(
                        &mut pm,
                        &text.fit(save_label(self.saving), small, room),
                        ((r.x + 44.0) as f32, (r.cy() + 11.0) as f32),
                        small,
                        if self.saving { pal::accent() } else { GREY },
                        Align::Left,
                    );
                }
                Hit::Style => {
                    crate::phosphor::draw(&mut pm, "chart-bar", ic_x(&r), r.cy(), 18, icon_c(IconCat::Analyse, hover(&r)));
                    lbl(&mut pm, &r, *hit);
                }
                Hit::Indic => {
                    let c = icon_c(IconCat::Analyse, hover(&r));
                    crate::text::Text::graisse(Graisse::Medium).draw(&mut pm, "ƒx", (ic_x(&r) as f32, mid), 16.0, c, Align::Center);
                    lbl(&mut pm, &r, *hit);
                }
                Hit::Alertes => {
                    crate::phosphor::draw(&mut pm, "bell", ic_x(&r), r.cy(), 18, icon_c(IconCat::Info, hover(&r)));
                    lbl(&mut pm, &r, *hit);
                    if self.alertes > 0 {
                        // pastille rouge du nombre d'alertes (maquette : 🔔 3)
                        let n = if self.alertes > 9 { "9+".to_string() } else { self.alertes.to_string() };
                        let (bx, by) = (r.x + r.w - 9.0, r.y + 7.0);
                        let bw = (text.width(&n, 9.5) as f64 + 7.0).max(14.0);
                        round_fill(&mut pm, Rect::new(bx - bw / 2.0, by - 7.0, bw, 14.0), 7.0, crate::jetons::BAS, 1.0);
                        text.draw(&mut pm, &n, (bx as f32, (by + 3.5) as f32), 9.5, crate::jetons::BLANC, Align::Center);
                    }
                }
                Hit::Partager => {
                    crate::phosphor::draw(&mut pm, "export", r.cx(), r.cy(), 18, icon_c(IconCat::Trading, hover(&r)));
                }
                Hit::Gardien => {
                    // 🛡 pilule du Gardien PropFirm : « perte du jour / limite du jour » en % du compte, à sa couleur
                    let (t, c) = self.gardien.clone().unwrap_or_else(|| ("Gardien".into(), crate::jetons::DIM));
                    round_fill(&mut pm, r, r.h / 2.0, c, if hover(&r) { 0.3 } else { 0.2 });
                    round_stroke(&mut pm, r, r.h / 2.0, crate::jetons::sur_fond((c, 0.45)), 1.0);
                    crate::phosphor::draw(&mut pm, "shield-check", r.x + 12.0, r.cy(), 12, c);
                    Text::graisse(Graisse::Medium).draw(&mut pm, &t, ((r.x + 21.0) as f32, (r.cy() + 4.2) as f32), 11.5, c, Align::Left);
                }
                Hit::Split => {
                    // la disposition en petit (▭, ◫, ⊞…) + ⌄
                    if hover(&r) {
                        round_fill(&mut pm, r, r.h / 2.0, pal::tile(), 1.0);
                    }
                    let icon = Rect::new(if self.phone { r.cx() - 10.0 } else { r.x + 6.0 }, r.cy() - 7.0, 20.0, 14.0);
                    for c in crate::splits::cells(&self.split) {
                        let cell = Rect::new(icon.x + c[0] * icon.w, icon.y + c[1] * icon.h, c[2] * icon.w, c[3] * icon.h);
                        round_stroke(&mut pm, cell, 1.5, icon_c(IconCat::Settings, hover(&r)), 1.2);
                    }
                    if !self.phone {
                        // (avec l'étiquette « Split » : elle suit l'icône, le ⌄ au bout)
                        if r.w > 50.0 {
                            text.draw(&mut pm, "Split", ((r.x + 31.0) as f32, mid), ETIQ_PX, TEXT_LBL, Align::Left);
                        }
                        chevron(&mut pm, r.x + r.w - 10.0, r.cy(), GREY);
                    }
                }
                // 📱 🗺 le plan de l'app (maquette cell)
                Hit::App if self.phone => {
                    crate::phosphor::draw(&mut pm, "compass", r.cx(), r.cy(), 18, icon_c(IconCat::Settings, hover(&r)));
                }
                Hit::App => {
                    // ☰ : trois traits, comme le menu des applis installées
                    let c = icon_c(IconCat::Settings, hover(&r));
                    for dy in [-6.0f32, 0.0, 6.0] {
                        lines(&mut pm, r.cx(), r.cy(), &[&[(-8.0, dy), (8.0, dy)]], c, 1.6);
                    }
                }
                Hit::Settings => {
                    let teeth: Vec<(f32, f32)> = (0..=48)
                        .map(|k| {
                            let a = k as f32 / 48.0 * std::f32::consts::TAU;
                            let rr = if (k / 3) % 2 == 0 { 8.0 } else { 6.2 };
                            (a.cos() * rr, a.sin() * rr)
                        })
                        .collect();
                    lines(
                        &mut pm,
                        r.cx(),
                        r.cy(),
                        &[&teeth, &crate::shapes::arc(2.6, 0.0, 360.0, 16)],
                        icon_c(IconCat::Settings, hover(&r)),
                        1.3,
                    );
                }
                Hit::Star => {
                    crate::listmenu::star(&mut pm, r.cx(), r.cy(), self.prefs.flagged);
                }
                Hit::Usd | Hit::Pct => {
                    let on = self.prefs.pct == (*hit == Hit::Pct);
                    if on {
                        round_fill(&mut pm, r, r.h / 2.0, pal::on(), 1.0);
                        round_stroke(&mut pm, r, r.h / 2.0, crate::palette::accent(), 1.0);
                    }
                    text.draw(
                        &mut pm,
                        if *hit == Hit::Usd { "$" } else { "%" },
                        (r.cx() as f32, mid),
                        13.0,
                        if on { crate::palette::cyan() } else { GREY },
                        Align::Center,
                    );
                }
                Hit::Tick(i) => {
                    let t = &self.tape[i];
                    Text::graisse(Graisse::Bold).draw(
                        &mut pm,
                        &t.label,
                        ((r.x + 6.0) as f32, mid),
                        12.0,
                        if i == self.tape_from { crate::palette::cyan() } else { WHITE },
                        Align::Left,
                    );
                    let v = self.tick_text(t);
                    let c = match t.pct {
                        Some(p) if self.prefs.pct || !t.price.is_empty() => {
                            if p >= 0.0 {
                                th.bull
                            } else {
                                th.bear
                            }
                        }
                        _ => GREY,
                    };
                    text.draw(&mut pm, &v, ((r.x + r.w - 6.0) as f32, mid), 12.0, c, Align::Right);
                }
                Hit::Next => lines(&mut pm, r.cx(), r.cy(), &[&[(-2.0, -4.5), (2.5, 0.0), (-2.0, 4.5)]], GREY, 1.4),
                Hit::Replier => lines(
                    &mut pm,
                    r.cx(),
                    r.cy(),
                    if self.sans_bandeaux {
                        &[&[(-4.5, 2.0), (0.0, -2.5), (4.5, 2.0)]]
                    } else {
                        &[&[(-4.5, -2.0), (0.0, 2.5), (4.5, -2.0)]]
                    },
                    if hover(&r) { WHITE } else { GREY },
                    1.5,
                ),
                Hit::Symbol => {
                    // (a Bitfinex perpetual « bitfinex:XRPF0:USTF0 »: XRP's badge, its logo when the PC has it)
                    // (no row chosen yet, a chart reopened at start-up: the coin of its name, « BITFINEX:XRPUSDT.P » → XRP)
                    let bitfinex = argos_data::exchanges::bitfinex::pastille(&self.coin).or_else(|| {
                        let nom = self.symbol.rsplit(':').next().unwrap_or("");
                        self.coin.is_empty().then(|| argos_data::tv::pair_parts(nom).map(|(b, _)| b)).flatten()
                    });
                    coin_badge(
                        &mut pm,
                        text,
                        bitfinex.as_deref().unwrap_or_else(|| self.coin.rsplit(':').next().unwrap_or(&self.coin)),
                        logos.get(&self.coin).or_else(|| bitfinex.as_ref().and_then(|b| logos.get(b))),
                        r.x + 16.0,
                        r.cy(),
                        9.0,
                    );
                    Text::graisse(Graisse::Bold).draw(&mut pm, &self.symbol, ((r.x + 39.0) as f32, (r.cy() + 4.6) as f32), SYM_PX, WHITE, Align::Left);
                    chevron(&mut pm, r.x + r.w - 12.0, r.cy() + 1.0, GREY);
                }
                Hit::Iv => {
                    round_fill(&mut pm, r, r.h / 2.0, pal::field(), 1.0);
                    round_stroke(&mut pm, r, r.h / 2.0, pal::edge(), 1.0);
                    text.draw(&mut pm, &self.iv.1, ((r.x + 9.0) as f32, (r.cy() + 4.5) as f32), SYM_PX, WHITE, Align::Left);
                    chevron(&mut pm, r.x + r.w - 11.0, r.cy() + 1.0, GREY);
                }
                Hit::Fav(i) => {
                    if let Some((secs, l)) = self.iv_favs.get(i) {
                        let on = *secs == self.iv.0;
                        if on {
                            round_fill(&mut pm, r, r.h / 2.0, pal::accent(), 0.25);
                        }
                        let c = if on { pal::accent() } else { GREY };
                        text.draw(&mut pm, l, (r.cx() as f32, (r.cy() + 4.0) as f32), 11.5, c, Align::Center);
                    }
                }
            }
        }
        // chiffres de la fiche, après le nom
        if self.phone || (self.sans_bandeaux && !self.survol_bandeaux) {
            return pm;
        }
        let sym = zones
            .iter()
            .filter(|(_, h)| matches!(h, Hit::Symbol | Hit::Iv | Hit::Fav(_)))
            .map(|(r, _)| r.x + r.w)
            .fold(M, f64::max);
        let (y3, _) = ROW3;
        let mut x = sym + 24.0;
        for st in self.stats.iter().chain(std::iter::once(&Stat {
            label: "Source".into(),
            value: self.source.clone(),
            color: None,
        })) {
            let gras = Text::graisse(Graisse::Bold);
            let cw = (text.width(&st.label, 10.0) as f64).max(gras.width(&st.value, SYM_PX) as f64);
            if x + cw > d - 40.0 {
                break;
            }
            // (no separator between the columns: each column, label and value, on its own glass pill)
            pastille_verre(&mut pm, Rect::new(x - 8.0, y3 + 1.0, cw + 16.0, ROW3.1 - 2.0));
            text.draw(&mut pm, &st.label, (x as f32, (y3 + 14.0) as f32), 10.0, GREY, Align::Left);
            gras.draw(
                &mut pm,
                &st.value,
                (x as f32, (y3 + 28.5) as f32),
                SYM_PX,
                st.color.unwrap_or(WHITE),
                Align::Left,
            );
            x += cw + 21.0;
        }
        pm
    }
}

fn chevron(pm: &mut Pixmap, cx: f64, cy: f64, c: Rgb) {
    lines(pm, cx, cy, &[&[(-3.5, -1.5), (0.0, 2.0), (3.5, -1.5)]], c, 1.3);
}

/// Funding : « 0,0013 % · 23:41 » (taux horaire en % et temps jusqu'au prochain paiement, à chaque heure pile).
pub fn funding_text(rate: f64, now_s: i64) -> String {
    let left = 3600 - now_s.rem_euclid(3600);
    format!("{:.4} % · {:02}:{:02}", rate * 100.0, left / 60, left % 60)
}

/// Gros nombres : 1,2 G / 345,6 M / 12,3 k.
pub fn big(v: f64) -> String {
    let (d, s) = match v.abs() {
        a if a >= 1e9 => (v / 1e9, " G"),
        a if a >= 1e6 => (v / 1e6, " M"),
        a if a >= 1e3 => (v / 1e3, " k"),
        _ => (v, ""),
    };
    format!("{}{s}", crate::viewport::format_price(d, if s.is_empty() { 2 } else { 1 }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar() -> TopBar {
        TopBar {
            tape: (0..40)
                .map(|i| Tick {
                    coin: format!("C{i}"),
                    label: format!("C{i}"),
                    price: "1.5".into(),
                    pct: Some(i as f64 - 20.0),
                })
                .collect(),
            coin: "BTC".into(),
            symbol: "BTC-USDC".into(),
            stats: vec![Stat {
                label: "Prix".into(),
                value: "80 261".into(),
                color: None,
            }],
            source: "Hyperliquid".into(),
            layout: "Argos".into(),
            ..TopBar::default()
        }
    }

    /// 🚫 Jee (16 Nov, « enlever toutes les lignes carrées », asked five times): no frame around the rows of the top
    /// bar — no background, no border, no separator — on PC, tablet and phone, in the bar, in Zen and when rows 2 and
    /// 3 open over the chart. Where a row's frame had its edge (its two ends and its top and bottom lines), only
    /// what is behind the row. 🫧 (19 Nov) But each element is its own glass pill: glass just inside its edge, what is
    /// behind between two pills.
    #[test]
    fn rangee_1_sur_sa_pilule_verre() {
        // 🫧 Jee (20 Nov): row 1 (⌂ ARGOS ▾, Bougies, Indicateurs…) on its glass pill, as before 51ef2bef: glass just
        // inside its edges, round ends (the corners show the bar behind); not on the phone, not laid over the chart
        let t = Text::default();
        for (rangees_ouvertes, w) in [(false, 1200u32), (true, 1200), (false, 2000)] {
            let mut b = bar();
            if rangees_ouvertes {
                (b.sans_bandeaux, b.survol_bandeaux) = (true, true);
            }
            let pm = b.draw(&t, w, None, &HashMap::new(), &|_| false, &crate::Theme::default());
            let px = |x: f64, y: f64| {
                let p = pm.pixel(x as u32, y as u32).unwrap();
                (p.red(), p.green(), p.blue(), p.alpha())
            };
            let (y, h) = ROW1;
            let derriere = px(1.0, y + h / 2.0);
            for (x, yy) in [(M + h, y + 1.5), (M + h, y + h - 1.5), (w as f64 - M - h, y + 1.5), (M + 1.5, y + h / 2.0)] {
                assert_ne!(px(x, yy), derriere, "{w}: row 1's glass at ({x}, {yy})");
            }
            assert_eq!(px(M + 0.6, y + 0.6), derriere, "{w}: round end (the corner shows what is behind)");
        }
        for (flottante, phone) in [(true, false), (false, true)] {
            let mut b = bar();
            (b.flottante, b.phone) = (flottante, phone);
            let w = if phone { 402 } else { 1200 };
            let pm = b.draw(&t, w, None, &HashMap::new(), &|_| false, &crate::Theme::default());
            let row = if phone { PHONE_ROW } else { ROW1 };
            let px = |x: f64, y: f64| {
                let p = pm.pixel(x as u32, y as u32).unwrap();
                (p.red(), p.green(), p.blue(), p.alpha())
            };
            assert_eq!(px(M + 0.5, row.0 + row.1 / 2.0), px(1.0, row.0 + row.1 / 2.0), "Zen / phone: no row pill");
        }
    }

    #[test]
    fn rangees_sans_cadre_partout() {
        let t = Text::default();
        let w = 1200u32;
        for (quoi, flottante, survol, phone) in [
            ("PC", false, false, false),
            ("Zen", true, false, false),
            ("rows over the chart", false, true, false),
            ("phone", false, false, true),
        ] {
            let mut b = bar();
            (b.flottante, b.phone) = (flottante, phone);
            if survol {
                (b.sans_bandeaux, b.survol_bandeaux) = (true, true);
            }
            let w = if phone { 402 } else { w };
            let pm = b.draw(&t, w, None, &HashMap::new(), &|_| false, &crate::Theme::default());
            let px = |x: f64, y: f64| {
                let p = pm.pixel(x as u32, y as u32).unwrap();
                (p.red(), p.green(), p.blue(), p.alpha())
            };
            // (🫧 20 Nov: on the PC, row 1 is on its glass pill again: checked in `rangee_1_sur_sa_pilule_verre`)
            let rows: &[(f64, f64)] = if phone {
                &[PHONE_ROW]
            } else if !flottante {
                &[ROW2, ROW3]
            } else {
                &[ROW1, ROW2, ROW3]
            };
            for &(y, h) in rows {
                let derriere = px(1.0, y + h / 2.0);
                for (x, yy) in [
                    (M + 0.5, y + h / 2.0),
                    (w as f64 - M - 0.5, y + h / 2.0),
                    (M + h, y + 0.5),
                    (M + h, y + h - 0.5),
                    (w as f64 - M - h, y + 0.5),
                    (w as f64 - M - h, y + h - 0.5),
                ] {
                    assert_eq!(px(x, yy), derriere, "{quoi}: a frame around the row at {y} ({x}, {yy})");
                }
            }
            // the pills are still there
            assert!(pm.pixels().iter().any(|p| p.alpha() == 255 && p.red() > 150), "{quoi}: the pills are drawn");
            if phone {
                continue;
            }
            // 🫧 Jee (19 Nov): each element on its own glass pill — just inside a pill's edge, its glass (not what is
            // behind); between two pills, what is behind (no background, no frame between them)
            let zones = b.layout(w as f64, &t, &|_| false);
            // (neighbours on the screen, left to right in the row — 🛟 Support is laid out last, between others)
            let rangee = |r: &Rect| if flottante { r.y < ROW2.0 } else { r.y >= ROW2.0 && r.y < ROW3.0 };
            let mut voisines: Vec<Rect> = zones.iter().map(|z| z.0).filter(rangee).collect();
            voisines.sort_by(|p, q| p.x.total_cmp(&q.x));
            let paires: Vec<(Rect, Rect)> = voisines
                .windows(2)
                .filter(|z| (z[0].y - z[1].y).abs() < 2.0 && z[1].x - (z[0].x + z[0].w) >= 3.0)
                .map(|z| (z[0], z[1]))
                .collect();
            assert!(paires.len() >= 3, "{quoi}: neighbouring pills to look at");
            for (a, c) in paires {
                let derriere = px(1.0, a.cy());
                let entre = ((a.x + a.w + c.x) / 2.0, a.cy());
                assert_eq!(px(entre.0, entre.1), derriere, "{quoi}: something between two pills at {entre:?}");
                for r in [a, c] {
                    assert_ne!(px(r.cx(), r.y + 1.5), derriere, "{quoi}: no glass inside the pill {r:?}");
                }
            }
            // row 3's stat columns: each its own pill, what is behind between them
            if !flottante {
                let y = ROW3.0 + 3.0;
                let sym = zones
                    .iter()
                    .filter(|(_, h)| matches!(h, Hit::Iv | Hit::Fav(_) | Hit::Symbol))
                    .map(|(r, _)| r.x + r.w)
                    .fold(0.0, f64::max);
                let derriere = px(1.0, ROW3.0 + ROW3.1 / 2.0);
                let mut runs = 0;
                let mut dans = false;
                for x in (sym + 8.0) as u32..(w - 40) {
                    let p = px(x as f64, y) != derriere;
                    if p && !dans {
                        runs += 1;
                    }
                    dans = p;
                }
                assert!(runs >= 2, "{quoi}: the stat columns' own pills ({runs})");
            }
        }
    }

    #[test]
    fn rechargement_revient_ou_on_etait() {
        // réglages d'avant verre-2 : « Accueil au lancement » allumé par défaut → éteint une fois
        let mut p: Prefs = serde_json::from_str(r#"{"accueil": true, "bienvenue_vue": true}"#).unwrap();
        assert!(p.passer_accueil_v2() && !p.accueil);
        assert!(!p.passer_accueil_v2(), "une seule fois");
        // Accueil fermé en partant : on revient sur la chart (pas l'Accueil)
        assert!(!p.accueil_au_lancement(false));
        // laissé ouvert dans cet onglet : il revient
        assert!(p.accueil_au_lancement(true));
        // tout premier lancement : ses 3 écrans
        assert!(Prefs::default().accueil_au_lancement(false));
        // le réglage rallumé à la main : toujours l'Accueil
        let q = Prefs {
            accueil: true,
            bienvenue_vue: true,
            ..Default::default()
        };
        assert!(q.accueil_au_lancement(false));
    }

    /// ⌂ « The Accueil is open » is no longer a shared setting: an old file's value is ignored when read, and the
    /// field is never written (another tab or device can no longer open the Accueil here at F5).
    #[test]
    fn accueil_ouvert_n_est_plus_partage() {
        let p: Prefs = serde_json::from_str(r#"{"accueil_ouvert": true, "bienvenue_vue": true, "accueil_v2": true}"#).unwrap();
        assert!(!p.accueil_au_lancement(false), "an Accueil left open elsewhere stays there");
        let ecrit = serde_json::to_string(&p).unwrap();
        assert!(!ecrit.contains("accueil_ouvert"), "{ecrit}");
    }

    #[test]
    fn reglages_d_avant_verre_1_passent_au_verre_une_fois() {
        // fichier d'avant verre-1 : verre éteint, pas de repère
        let mut p: Prefs = serde_json::from_str(r#"{"glass": false, "theme": 0}"#).unwrap();
        assert!(!p.verre_v1);
        assert!(p.passer_au_verre() && p.glass, "rallumé une fois");
        p.glass = false; // l'utilisateur l'éteint ensuite
        let mut q: Prefs = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert!(!q.passer_au_verre() && !q.glass, "son choix est gardé");
        assert!(Prefs::default().verre_v1 && Prefs::default().glass, "nouvelle installation : verre allumé");
    }

    /// 🔊 Settings saved before alert sounds: « Bip » at 60; a chosen sound and volume are kept.
    #[test]
    fn reglages_d_avant_les_sons() {
        let mut p: Prefs = serde_json::from_str(r#"{"glass": true, "theme": 0}"#).unwrap();
        assert_eq!((p.son_alertes.as_str(), p.volume_alertes), ("Bip", 60));
        assert_eq!((Prefs::default().son_alertes.as_str(), Prefs::default().volume_alertes), ("Bip", 60));
        (p.son_alertes, p.volume_alertes) = ("Cloche".into(), 30);
        let q: Prefs = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!((q.son_alertes.as_str(), q.volume_alertes), ("Cloche", 30));
    }

    #[test]
    fn clics() {
        let t = Text::default();
        let no = |_: &str| false;
        let mut b = bar();
        let w = 1600.0;
        let z = b.layout(w, &t, &no);
        let at = |h: Hit| z.iter().find(|(_, x)| *x == h).unwrap().0;
        let r = at(Hit::Pct);
        assert_eq!(b.click(r.cx(), r.cy(), w, &t, &no), TopClick::Changed);
        assert!(b.prefs.pct);
        // les largeurs des tickers changent avec $ / % : on relit les zones
        let r = b.layout(w, &t, &no).into_iter().find(|(_, x)| *x == Hit::Tick(3)).unwrap().0;
        assert_eq!(b.click(r.cx(), r.cy(), w, &t, &no), TopClick::Open("C3".into()));
        let r = at(Hit::Symbol);
        assert_eq!(b.click(r.cx(), r.cy(), w, &t, &no), TopClick::Symbol);
        // 🔎 « Où est… ? Ctrl+K » de la maquette : ouvre la recherche ; absent s'il n'y a pas la place
        let r = at(Hit::Recherche);
        assert_eq!(b.click(r.cx(), r.cy(), w, &t, &no), TopClick::Recherche);
        assert!(
            !b.layout(700.0, &t, &no).iter().any(|(_, x)| *x == Hit::Recherche),
            "fenêtre étroite : pas de place"
        );
        // ⏱ à droite du symbole : l'unité de temps (menu) et les favoris en raccourcis, l'actif surligné
        b.iv = (14_400, "4h".into());
        b.iv_favs = vec![(60, "1m".into()), (900, "15m".into()), (14_400, "4h".into())];
        let z = b.layout(w, &t, &no);
        let iv = z.iter().find(|(_, h)| *h == Hit::Iv).unwrap().0;
        assert!(iv.x > at(Hit::Symbol).x + at(Hit::Symbol).w, "juste à droite du symbole");
        assert_eq!(b.click(iv.cx(), iv.cy(), w, &t, &no), TopClick::Intervals(iv));
        let f = z.iter().find(|(_, h)| *h == Hit::Fav(1)).unwrap().0;
        assert_eq!(b.click(f.cx(), f.cy(), w, &t, &no), TopClick::Interval(900));
        // 📱 cell : pas de raccourcis (sa barre du bas a déjà son bouton d'unité de temps)
        b.phone = true;
        assert!(!b.layout(390.0, &t, &no).iter().any(|(_, h)| matches!(h, Hit::Fav(_))));
        b.phone = false;
        // 📊 fx 🔔 ▦ 📤 et la pilule du Gardien (maquette verre) ; Léger, Connect, 🌐 et ☰ ne sont plus sur la rangée
        for (hit, attendu) in [
            (Hit::Style, "style"),
            (Hit::Indic, "fx"),
            (Hit::Alertes, "alertes"),
            (Hit::Partager, "partager"),
            (Hit::Gardien, "gardien"),
        ] {
            let r = at(hit);
            let c = b.click(r.cx(), r.cy(), w, &t, &no);
            let ok = match attendu {
                "style" => c == TopClick::Style(r),
                "fx" => c == TopClick::Indicateurs(r),
                "alertes" => c == TopClick::Alertes(r),
                "partager" => c == TopClick::Partager,
                _ => c == TopClick::Gardien,
            };
            assert!(ok, "{attendu} : {c:?}");
        }
        // 🛟 Support: nothing else moves (same bar without it), never left of what is placed before it
        // (the DÉMO pill), « 🛟 Support », otherwise the circle alone, otherwise nothing
        for largeur in (700..2700).step_by(7).map(f64::from) {
            for gauche in [0.0, largeur * 0.45, largeur * 0.55] {
                let mut avec = b.clone();
                avec.support_gauche = gauche;
                let sans = TopBar {
                    support_gauche: f64::INFINITY,
                    ..avec.clone()
                };
                let za = avec.layout(largeur, &t, &no);
                let zs = sans.layout(largeur, &t, &no);
                let sup: Vec<Rect> = za.iter().filter(|(_, h)| *h == Hit::Support).map(|(r, _)| *r).collect();
                assert!(!zs.iter().any(|(_, h)| *h == Hit::Support));
                assert_eq!(
                    za.iter().filter(|(_, h)| *h != Hit::Support).cloned().collect::<Vec<_>>(),
                    zs,
                    "{largeur} : rien ne bouge"
                );
                if let Some(s) = sup.first() {
                    assert!(
                        s.x >= gauche + 8.0 - 0.01 && (s.w > s.h || (s.w - s.h).abs() < 0.01),
                        "{largeur} {gauche} : {s:?}"
                    );
                    let rech = zs.iter().find(|(_, h)| *h == Hit::Recherche).unwrap().0;
                    assert!(
                        zs.iter().all(|(q, _)| q.x >= s.x + s.w || q.x + q.w <= s.x || q.y >= s.y + s.h),
                        "{largeur} : rien sous 🛟"
                    );
                    assert!((s.x + s.w - (rech.x - 8.0)).abs() < 0.01);
                }
            }
        }
        // 🛟 Support: just left of « Où est… ? », same height
        let (sup, rech) = (at(Hit::Support), at(Hit::Recherche));
        assert!(
            sup.x + sup.w <= rech.x - 7.5 && sup.x + sup.w >= rech.x - 8.5 && sup.y == rech.y,
            "{sup:?} {rech:?}"
        );
        assert_eq!(b.click(sup.cx(), sup.cy(), w, &t, &no), TopClick::Support);
        // 🔎 1st row: click zones at least 28 high (drawn buttons are thinner in the compact interface)
        let g = at(Hit::Gardien);
        assert!(g.h < 28.0);
        assert_eq!(b.click(g.cx(), g.cy() - 13.5, w, &t, &no), TopClick::Gardien, "28-high click zone");
        assert_eq!(b.click(g.cx(), g.cy() - 14.5, w, &t, &no), TopClick::None);
        let z = b.layout(w, &t, &no);
        assert!(!z.iter().any(|(_, h)| *h == Hit::App), "☰ : dans ARGOS ▾ sur PC");
        let (g, s) = (at(Hit::Gardien), at(Hit::Settings));
        assert!(g.x + g.w <= s.x && s.x + s.w <= w - M, "pilule du Gardien juste avant ⚙, à droite");
        assert!(at(Hit::Recherche).x + at(Hit::Recherche).w <= g.x, "🔎 avant la pilule");
        // 🛡 taille de la pastille de la maquette PC (88 × 23 pour « 0,8 % / 5 % »)
        let mut b2 = b.clone();
        b2.gardien = Some(("0,8 % / 5 %".into(), crate::jetons::TEXTE));
        let g2 = b2.layout(w, &t, &no).into_iter().find(|(_, h)| *h == Hit::Gardien).unwrap().0;
        assert_eq!(g2.h, PILULE_H, "the height of its row (compact interface)");
        assert!((g2.w - 88.0).abs() <= 6.0, "largeur {}", g2.w);
        // les zones ne se chevauchent pas, même dans une fenêtre étroite
        for ww in [700.0, 1024.0, w] {
            let z: Vec<Rect> = b.layout(ww, &t, &no).into_iter().filter(|(r, _)| r.y < ROW2.0).map(|(r, _)| r).collect();
            for (i, a) in z.iter().enumerate() {
                for c in &z[i + 1..] {
                    assert!(a.x + a.w <= c.x + 0.5 || c.x + c.w <= a.x + 0.5, "{ww} : {a:?} {c:?}");
                }
            }
        }
        // › : page suivante, puis retour au début
        let page = b.layout(w, &t, &no).iter().filter(|(_, h)| matches!(h, Hit::Tick(_))).count();
        let r = at(Hit::Next);
        b.click(r.cx(), r.cy(), w, &t, &no);
        assert_eq!(b.tape_from, page);
        for _ in 0..10 {
            b.click(r.cx(), r.cy(), w, &t, &no);
        }
        assert!(b.tape_from < 40);
        // ★ : seulement les drapeaux
        let flagged = |c: &str| c == "C7";
        let r = at(Hit::Star);
        b.click(r.cx(), r.cy(), w, &t, &flagged);
        let z = b.layout(w, &t, &flagged);
        assert_eq!(z.iter().filter(|(_, h)| matches!(h, Hit::Tick(_))).count(), 1);
        let pm = b.draw(&t, 1600, Some((10.0, 10.0)), &HashMap::new(), &flagged, &Theme::default());
        assert_eq!(pm.height(), HEIGHT);
        assert!(wordmark().is_some(), "logo ARGOS décodé");
        assert!((0..Theme::NAMES.len()).all(|k| wordmark_for(k).is_some()), "logo de chaque thème");
        assert_eq!(
            wordmark_for(2).map(|p| p.data().to_vec()),
            wordmark().map(|p| p.data().to_vec()),
            "🔷 Argos : logo d'origine"
        );
    }

    /// Bug cell (Jee, 2026-11-09 : « tout s'entremêle dans le menu du haut ») : à chaque largeur, aucune zone de la
    /// barre ne chevauche une autre, et toutes restent dans la largeur. Cell : ⌂ ARGOS ▾ · mise en page · ⚙ ☰.
    #[test]
    fn rien_ne_se_chevauche() {
        let t = Text::default();
        let no = |_: &str| false;
        let mut b = bar();
        (b.layout, b.gardien, b.alertes) = ("jee crypto avec un très long nom".into(), Some(("0,8 % / 5 %".into(), WHITE)), 3);
        for (phone, ws) in [(true, &[320.0, 375.0, 390.0, 430.0][..]), (false, &[768.0, 1024.0, 1440.0, 1600.0][..])] {
            b.phone = phone;
            for &w in ws {
                let z = b.layout(w, &t, &no);
                for (i, (a, ha)) in z.iter().enumerate() {
                    assert!(a.x >= 0.0 && a.x + a.w <= w + 0.5, "{w} : {ha:?} hors de la barre");
                    for (c, hc) in &z[i + 1..] {
                        let separes = a.x + a.w <= c.x + 0.5 || c.x + c.w <= a.x + 0.5 || a.y + a.h <= c.y + 0.5 || c.y + c.h <= a.y + 0.5;
                        assert!(separes, "{w} : {ha:?} {a:?} chevauche {hc:?} {c:?}");
                    }
                }
                if phone {
                    let hits: Vec<Hit> = z.iter().map(|(_, h)| *h).collect();
                    assert!(
                        [Hit::Home, Hit::Logo, Hit::Alertes, Hit::Split, Hit::Partager, Hit::Recherche, Hit::App]
                            .iter()
                            .all(|h| hits.contains(h)),
                        "{w} : {hits:?}"
                    );
                    assert!(w < 360.0 || hits.contains(&Hit::Layout), "{w} : la mise en page tient");
                }
            }
        }
    }

    #[test]
    fn textes() {
        // 13 h 20 min 30 s : 39 min 30 s avant le prochain funding
        assert_eq!(funding_text(0.0000125, 13 * 3600 + 20 * 60 + 30), "0.0013 % · 39:30");
        assert_eq!(big(2.5e9), "2,5 G".replace(',', &crate::viewport::format_price(0.5, 1)[1..2]));
        assert!(big(1234.0).ends_with(" k"));
    }

    /// 📸 Capture de la barre du haut pour la comparaison avec la maquette (1440 px) : seulement si `ARGOS_CAPTURES`.
    #[test]
    fn captures_pour_jee() {
        let Ok(dir) = std::env::var("ARGOS_CAPTURES") else { return };
        let mut b = bar();
        (b.alertes, b.gardien) = (3, Some(("0,8 % / 5 %".into(), crate::palette::up())));
        let pm = b.draw(&Text::default(), 1440, None, &HashMap::new(), &|_: &str| false, &Theme::default());
        pm.save_png(format!("{dir}/app-barre-du-haut.png")).unwrap();
        // 📱 cell (390 px) : une pilule de verre
        b.phone = true;
        b.layout = "jee crypto".into();
        let pm = b.draw(&Text::default(), 390, None, &HashMap::new(), &|_: &str| false, &Theme::default());
        pm.save_png(format!("{dir}/app-cell-barre-du-haut.png")).unwrap();
    }
}
