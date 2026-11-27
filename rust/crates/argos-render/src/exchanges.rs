//! 🔌 Panneau **API** (ouvert par 🔌 à côté du ＋ de la watchlist) et boîte de **clé API**.
//! - Panneau : « ＋ Ajouter une API », puis un groupe par exchange : pastille d'état, interrupteur (activer / couper
//!   sans effacer la clé), état du catalogue, ⟳, et sa clé (toujours en lecture seule) avec ✕ pour l'oublier.
//! - Boîte de clé : les champs de l'exchange (clé, secret, passphrase ; Hyperliquid : l'adresse publique seulement),
//!   masqués par des points. Les valeurs ne vivent qu'ici le temps de taper : la fenêtre les envoie au trousseau du
//!   système et les oublie. Elles ne sont jamais dessinées en clair, ni écrites dans un fichier.

use crate::panels::{ai_button, AI_W};
use crate::shapes::{lines, round_fill, round_stroke, Rect};
use crate::text::{Align, Rgb, Text};
use tiny_skia::Pixmap;

use crate::palette as pal;
use crate::palette::{accent, DIM, GOLD, GREY, WHITE};

/// Haut des groupes (sous le titre, l'avis et « ＋ Ajouter une API »).
const TOP: f64 = 124.0;
/// Hauteur d'un groupe (un exchange et sa clé).
const ROW_H: f64 = 96.0;

/// Un exchange du panneau.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExRow {
    pub id: String,
    pub name: String,
    /// Activé : catalogue téléchargé chaque jour et clé utilisée ; coupé : rien, la clé reste au trousseau.
    pub enabled: bool,
    /// Symboles actifs du catalogue (0 : pas encore téléchargé).
    pub symbols: usize,
    /// Il y a combien de secondes le catalogue a été mis à jour (`None` : jamais).
    pub age_s: Option<i64>,
    /// Une clé (ou une adresse) est dans le trousseau.
    pub has_key: bool,
    /// Téléchargement en cours.
    pub loading: bool,
    /// Ligne d'état écrite par l'app (remplace celle calculée ; vide = calculée). Ex. crédits CoinMarketCap du mois.
    pub note: String,
}

impl ExRow {
    /// Pastille d'état : vert = activé avec clé, bleu = activé (données publiques), gris = coupé.
    fn dot(&self) -> Rgb {
        match (self.enabled, self.has_key) {
            (true, true) => crate::palette::up(),
            (true, false) => accent(),
            _ => DIM,
        }
    }
}

/// Ce qu'un clic demande à la fenêtre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExAction {
    /// ＋ Ajouter une API : la fenêtre propose les exchanges.
    Add,
    Toggle(String),
    Refresh(String),
    Key(String),
    Forget(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Hit {
    Ai,
    Add,
    Toggle(usize),
    Refresh(usize),
    Key(usize),
    Forget(usize),
}

/// Ce qu'un clic dans le panneau renvoie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExClick {
    None,
    Ai,
    Action(ExAction),
}

fn ago(s: i64) -> String {
    if s < 120 {
        "à l'instant".into()
    } else if s < 7200 {
        format!("il y a {} min", s / 60)
    } else if s < 172_800 {
        format!("il y a {} h", s / 3600)
    } else {
        format!("il y a {} j", s / 86_400)
    }
}

/// Carte d'un groupe.
fn card(i: usize, pw: f64) -> Rect {
    Rect::new(10.0, TOP + i as f64 * ROW_H, pw - 20.0, ROW_H - 8.0)
}

/// ↻ et interrupteur d'une carte `c`, à droite de son titre.
fn icones_carte(c: Rect) -> [Rect; 2] {
    [
        Rect::new(c.x + c.w - 92.0, c.y + 10.0, 30.0, 28.0),
        Rect::new(c.x + c.w - 54.0, c.y + 12.0, 42.0, 24.0),
    ]
}

/// 📐 Titre d'une carte (« OANDA (forex, or, indices) »…) : coupé (…) avant ↻ et l'interrupteur, à toute largeur.
pub fn titre_carte(text: &Text, name: &str, i: usize, pw: f64) -> (crate::entete::Entete, [Rect; 2]) {
    let c = card(i, pw);
    let ic = icones_carte(c);
    (crate::entete::placer(text, name, 15.0, (c.x + 30.0, c.y + 30.0), &ic), ic)
}

fn layout(rows: &[ExRow], pw: f64) -> Vec<(Rect, Hit)> {
    let mut out = vec![
        (Rect::new(pw - AI_W - 12.0 - crate::watchlist::FERME_PLACE, 9.0, AI_W, 28.0), Hit::Ai),
        (Rect::new(14.0, 70.0, pw - 28.0, 38.0), Hit::Add),
    ];
    for (i, r) in rows.iter().enumerate() {
        let c = card(i, pw);
        let [refresh, toggle] = icones_carte(c);
        out.push((toggle, Hit::Toggle(i)));
        out.push((refresh, Hit::Refresh(i)));
        // ligne de la clé : le nom de la clé (ou « ＋ Ajouter une clé »), ✕ pour l'oublier
        out.push((Rect::new(c.x + 8.0, c.y + 56.0, c.w - 50.0, 26.0), Hit::Key(i)));
        if r.has_key {
            out.push((Rect::new(c.x + c.w - 36.0, c.y + 57.0, 24.0, 24.0), Hit::Forget(i)));
        }
    }
    out
}

/// Hauteur dont le panneau a besoin (il défile en dessous).
pub fn content_h(rows: &[ExRow]) -> f64 {
    TOP + rows.len() as f64 * ROW_H + 16.0
}

/// 📱 Au doigt : ↻ et l'interrupteur se touchent sur 44 de haut (leur dessin garde sa taille ; ils ne se chevauchent pas).
fn zone_du_doigt(q: Rect, hit: &Hit) -> Rect {
    match hit {
        _ if !crate::actions::tactile() => q,
        Hit::Refresh(_) => Rect::new(q.cx() - 22.0, q.cy() - 22.0, 44.0, 44.0),
        Hit::Toggle(_) => Rect::new(q.x, q.cy() - 22.0, q.w + 8.0, 44.0),
        _ => q,
    }
}

/// Clic dans le panneau (coordonnées du panneau, largeur `pw`).
pub fn click(rows: &[ExRow], x: f64, y: f64, pw: f64) -> ExClick {
    match layout(rows, pw)
        .into_iter()
        .rev()
        .find(|(r, h)| zone_du_doigt(*r, h).contains(x, y))
        .map(|(_, h)| h)
    {
        Some(Hit::Ai) => ExClick::Ai,
        Some(Hit::Add) => ExClick::Action(ExAction::Add),
        Some(Hit::Toggle(i)) => ExClick::Action(ExAction::Toggle(rows[i].id.clone())),
        Some(Hit::Refresh(i)) => ExClick::Action(ExAction::Refresh(rows[i].id.clone())),
        Some(Hit::Key(i)) => ExClick::Action(ExAction::Key(rows[i].id.clone())),
        Some(Hit::Forget(i)) => ExClick::Action(ExAction::Forget(rows[i].id.clone())),
        None => ExClick::None,
    }
}

/// Dessine le panneau (largeur `pw`) ; `mouse` en coordonnées du panneau.
pub fn draw(rows: &[ExRow], pm: &mut Pixmap, text: &Text, pw: f64, mouse: Option<(f64, f64)>, _grid: Rgb) {
    let hover = |r: &Rect| mouse.is_some_and(|(x, y)| r.contains(x, y));
    text.draw(pm, "API", (14.0, 29.0), 16.0, WHITE, Align::Left);
    text.draw(
        pm,
        "Par exchange · clés en lecture seule, dans ton trousseau",
        (14.0, 58.0),
        11.5,
        GREY,
        Align::Left,
    );
    for (i, r) in rows.iter().enumerate() {
        let c = card(i, pw);
        round_fill(pm, c, 10.0, pal::panel(), 1.0);
        round_stroke(pm, c, 10.0, pal::edge_soft(), 1.0);
        round_fill(pm, Rect::new(c.x + 12.0, c.y + 20.0, 9.0, 9.0), 4.5, r.dot(), 1.0);
        text.draw(
            pm,
            &titre_carte(text, &r.name, i, pw).0.titre,
            ((c.x + 30.0) as f32, (c.y + 30.0) as f32),
            15.0,
            if r.enabled { WHITE } else { GREY },
            Align::Left,
        );
        let status = match (r.enabled, r.loading, r.symbols, r.age_s) {
            _ if !r.note.is_empty() => r.note.clone(),
            // 🏦 OANDA : branché avec le jeton, sinon les équivalents Hyperliquid
            (true, ..) if r.id == "oanda" => format!("{} symboles · vrais prix OANDA", r.symbols),
            (..) if r.id.starts_with("custom-") => "API personnalisée · ta clé, pour tes scripts et outils".to_string(),
            (false, ..) if r.id == "oanda" => "sans jeton : équivalents Hyperliquid".to_string(),
            (false, ..) => "coupé · la clé reste dans ton trousseau".to_string(),
            (_, true, _, _) => "téléchargement…".into(),
            (_, _, 0, _) => "catalogue à télécharger (⟳)".into(),
            (_, _, n, Some(a)) => format!("{n} symboles · {}", ago(a)),
            (_, _, n, None) => format!("{n} symboles"),
        };
        text.draw(
            pm,
            &status,
            ((c.x + 30.0) as f32, (c.y + 49.0) as f32),
            11.5,
            if r.loading { GOLD } else { GREY },
            Align::Left,
        );
    }
    for (q, hit) in layout(rows, pw) {
        let h = hover(&q);
        match hit {
            Hit::Ai => ai_button(pm, q, h),
            Hit::Add => {
                round_fill(pm, q, 8.0, if h { pal::hover() } else { pal::field() }, 1.0);
                round_stroke(pm, q, 8.0, accent(), 1.0);
                text.draw(pm, "+  Ajouter une API", (q.cx() as f32, (q.cy() + 5.0) as f32), 14.0, WHITE, Align::Center);
            }
            Hit::Toggle(i) => {
                let on = rows[i].enabled;
                round_fill(pm, q, q.h / 2.0, if on { accent() } else { pal::track() }, 1.0);
                let kx = if on { q.x + q.w - q.h / 2.0 } else { q.x + q.h / 2.0 };
                round_fill(pm, Rect::new(kx - 9.0, q.cy() - 9.0, 18.0, 18.0), 9.0, WHITE, 1.0);
            }
            Hit::Refresh(_) => {
                if h {
                    round_fill(pm, q, 6.0, pal::hover(), 1.0);
                }
                let arc = crate::shapes::arc(7.0, 40.0, 330.0, 24);
                lines(pm, q.cx(), q.cy(), &[&arc, &[(-1.0, -10.0), (-4.0, -7.0), (-1.0, -4.0)]], GREY, 1.4);
            }
            Hit::Key(i) => {
                let set = rows[i].has_key;
                round_fill(pm, q, 6.0, if h { pal::hover() } else { pal::field() }, 1.0);
                let (label, col) = match (set, rows[i].id.as_str()) {
                    (true, "hl") => ("Adresse publique · lecture seule", crate::palette::up()),
                    (true, "oanda") => ("Jeton OANDA · branché", crate::palette::up()),
                    (false, "oanda") => ("+  Ajouter ton jeton OANDA", GREY),
                    (true, id) if id.starts_with("custom-") => ("Clé · lecture seule", crate::palette::up()),
                    (false, id) if id.starts_with("custom-") => ("+  Ajouter la clé", GREY),
                    (true, _) => ("Clé API · lecture seule", crate::palette::up()),
                    (false, "hl") => ("+  Ajouter l'adresse publique", GREY),
                    (false, _) => ("+  Ajouter une clé (lecture seule)", GREY),
                };
                text.draw(pm, label, ((q.x + 10.0) as f32, (q.cy() + 4.5) as f32), 12.5, col, Align::Left);
            }
            Hit::Forget(_) => {
                if h {
                    round_fill(pm, q, 4.0, pal::hover(), 1.0);
                }
                lines(pm, q.cx(), q.cy(), &[&[(-4.5, -4.5), (4.5, 4.5)], &[(4.5, -4.5), (-4.5, 4.5)]], GREY, 1.4);
            }
        }
    }
}

/// Un champ de la boîte de clé.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyField {
    /// Nom dans le trousseau (« key », « secret », « passphrase », « address »).
    pub id: &'static str,
    pub label: String,
    /// Masqué par des points (tout sauf une adresse publique).
    pub masked: bool,
    pub value: String,
}

/// Résultat d'un clic dans la boîte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyClick {
    None,
    Cancel,
    Save,
}

/// Boîte pour entrer la clé d'un exchange.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyBox {
    pub exchange: String,
    pub name: String,
    pub fields: Vec<KeyField>,
    pub focus: usize,
    pub rect: Rect,
    /// ⚠️ Avertissement sous le titre, dans une carte en verre : ses lignes déjà coupées à la largeur de la boîte,
    /// chacune avec son retrait (la suite d'une puce « • » s'aligne sous son texte).
    pub avis: Vec<(f64, String)>,
    /// 🌐 Dans le navigateur : la clé de ce service ne s'y entre pas (règle de Jee : aucune clé dans le navigateur ; elle
    /// se range dans le trousseau de l'app du PC) — la boîte l'explique, sans champ ni bouton grisé, et se ferme.
    pub navigateur: bool,
}

/// 🌐 Les étapes pour entrer la clé dans l'app du PC (boîte ouverte dans le navigateur).
pub const NAVIGATEUR_TXT: [&str; 6] = [
    "Ta clé se met dans l'app installée sur ton PC, pas dans le navigateur, pour qu'elle reste dans ton ordi.",
    "Sur ton PC :",
    "1. Ouvre l'icône « Argos HSS Trading » dans le menu des applications.",
    "2. Ouvre le volet API (🔑 dans la colonne de droite).",
    "3. Sur cette source : « Ajouter une clé », colle-la (Ctrl + V).",
    "4. Clique Enregistrer : elle va dans le trousseau de ton ordi.",
];

/// 🌐 La clé de ce service se range-t-elle dans le trousseau de l'ordi (impossible depuis le navigateur) ? Les boîtes
/// du compte Argos et de Claude ont leur propre coffre chiffré ; CoinMarketCap passe par le serveur du PC, qui range la
/// clé dans son trousseau (`ok clé serveur` de Jee) : sa boîte garde son champ dans le navigateur.
fn par_le_trousseau(exchange: &str) -> bool {
    !exchange.starts_with("claude") && !exchange.starts_with("account-") && exchange != "cmc"
}

/// Overwrites a secret text before freeing it (typed key, 🆘 rescue code).
pub fn effacer(v: &mut String) {
    let n = v.len();
    v.clear();
    v.extend(std::iter::repeat_n('0', n));
    v.clear();
}

/// Taille du texte de l'avertissement, hauteur d'une de ses lignes.
const AVIS_T: f32 = 12.0;
const AVIS_L: f64 = 17.0;

impl KeyBox {
    /// Champs selon l'exchange : Hyperliquid = l'adresse publique (jamais de clé privée) ; OKX et BloFin = clé, secret,
    /// passphrase ; les autres = clé et secret.
    pub fn new(exchange: &str, name: &str, win_w: f64, win_h: f64) -> Self {
        let f = |id, label: &str, masked| KeyField {
            id,
            label: label.into(),
            masked,
            value: String::new(),
        };
        let fields = match exchange {
            "hl" => vec![f("address", "Adresse publique du wallet (0x…)", false)],
            // ✨ clé Claude : chiffrée avec ton code (coffre) ; déverrouiller : le code seulement
            "claude" => vec![
                f("key", "Clé API Claude (sk-ant-…)", true),
                f("code", "Ton code (4 caractères ou plus)", true),
                f("confirm", "Confirme ton code", true),
            ],
            "claude-unlock" => vec![f("code", "Ton code", true)],
            "okx" | "blofin" => vec![f("key", "Clé API", true), f("secret", "Secret", true), f("passphrase", "Passphrase", true)],
            // 🏦 OANDA : le jeton personnel, et le type de compte (démo par défaut)
            "oanda" => vec![
                f("token", "Jeton (compte démo gratuit · hub.oanda.com ▸ Personal token)", true),
                KeyField {
                    value: "démo".into(),
                    ..f("env", "Compte : démo ou réel", false)
                },
            ],
            // 📈 CoinMarketCap : la clé (trousseau) et le lien de l'API (vide = celui de CoinMarketCap)
            "cmc" => vec![
                f("key", "Clé API CoinMarketCap", true),
                f("url", "Lien API CoinMarketCap (vide = pro-api.coinmarketcap.com)", false),
                f("every", "Relevé toutes les … heures (24 = une fois par jour)", false),
            ],
            // ✨ compte Argos : ton nom (ID @nom#tag) et le code qui chiffre ses clés sur cet appareil
            "account-new" => vec![
                f("handle", "Ton nom (3 à 20 lettres, chiffres, . ou _)", false),
                f("code", "Ton code (4 caractères ou plus)", true),
                f("confirm", "Confirme ton code", true),
            ],
            // 🔑 compte existant : ses 24 mots (masqués), son nom, et un code pour cet appareil
            "account-passkey" => vec![f("code", "Ton code (celui du compte sur cet appareil)", true)],
            // ➕ API personnalisée : un nom à toi, puis la clé (ou l'adresse), comme la boîte Hyperliquid
            "custom" => vec![
                f("name", "Nom de l'API (ex. Mon broker)", false),
                f("key", "Clé API ou adresse (lecture seule)", true),
            ],
            // 🆘 rescue code of the journal key (desktop app only): « AFFICHER » to see it, or the code to restore it
            "journal-secours" => vec![f("code", "AFFICHER pour voir ton code, ou ton code pour le restaurer", true)],
            // 🆘 the code shown (in the warning card, never in a field): nothing to type
            "journal-code" => vec![],
            "account-login" => vec![
                f("words", "Tes 24 mots Argos (séparés par des espaces)", true),
                f("handle", "Ton nom (ID)", false),
                f("code", "Un code pour cet appareil (4 caractères ou plus)", true),
                f("confirm", "Confirme ton code", true),
            ],
            _ => vec![f("key", "Clé API", true), f("secret", "Secret", true)],
        };
        let navigateur = cfg!(target_arch = "wasm32") && par_le_trousseau(exchange);
        // (🌐 navigateur : la place des étapes pour l'app du PC)
        let h = if navigateur { 360.0 } else { 170.0 + fields.len() as f64 * 62.0 };
        let w = 460.0f64.min(win_w - 12.0);
        Self {
            exchange: exchange.into(),
            name: name.into(),
            fields,
            focus: 0,
            rect: Rect::new((win_w - w) / 2.0, ((win_h - h) / 2.0).max(6.0), w, h),
            avis: vec![],
            navigateur,
        }
    }

    /// ✅ Vérification du compte : les mots n° `ask` (0 à 23) des 24 mots, à réécrire.
    pub fn verify(ask: [usize; 3], win_w: f64, win_h: f64) -> Self {
        let mut b = Self::new("account-verify", "Compte Argos", win_w, win_h);
        b.fields = ["w0", "w1", "w2"]
            .iter()
            .zip(ask)
            .map(|(id, i)| KeyField {
                id,
                label: format!("Mot n° {}", i + 1),
                masked: false,
                value: String::new(),
            })
            .collect();
        b.rect.h = 170.0 + b.fields.len() as f64 * 62.0;
        b
    }

    /// ⚠️ Ajoute un avertissement sous le titre : ses lignes, coupées à la largeur de la boîte (la suite d'une puce
    /// « • » en retrait, sous son texte) ; la boîte grandit d'autant et reste centrée.
    pub fn avec_avis(mut self, lignes: Vec<String>) -> Self {
        let text = Text::default();
        let w = self.rect.w - 60.0;
        for l in &lignes {
            let (puce, reste) = l.strip_prefix("• ").map_or(("", l.as_str()), |r| ("• ", r));
            let retrait = text.width(puce, AVIS_T) as f64;
            for (k, s) in crate::assistant::wrap(&text, reste, AVIS_T, w - retrait).into_iter().enumerate() {
                self.avis.push(if k == 0 { (0.0, format!("{puce}{s}")) } else { (retrait, s) });
            }
        }
        let plus = self.avis_h();
        self.rect.h += plus;
        self.rect.y = (self.rect.y - plus / 2.0).max(6.0);
        self
    }

    /// Hauteur de la carte de l'avertissement (0 sans avertissement).
    fn avis_h(&self) -> f64 {
        if self.avis.is_empty() {
            0.0
        } else {
            self.avis.len() as f64 * AVIS_L + 24.0
        }
    }

    fn field_rect(&self, i: usize) -> Rect {
        Rect::new(
            self.rect.x + 18.0,
            self.rect.y + 92.0 + self.avis_h() + i as f64 * 62.0,
            self.rect.w - 36.0,
            34.0,
        )
    }

    fn cancel_rect(&self) -> Rect {
        Rect::new(self.rect.x + self.rect.w - 230.0, self.rect.y + self.rect.h - 52.0, 100.0, 36.0)
    }

    fn save_rect(&self) -> Rect {
        Rect::new(self.rect.x + self.rect.w - 120.0, self.rect.y + self.rect.h - 52.0, 102.0, 36.0)
    }

    /// Tous les champs remplis ?
    pub fn complete(&self) -> bool {
        // le lien CoinMarketCap et la fréquence peuvent rester vides (= celui de CoinMarketCap, 24 h)
        self.fields
            .iter()
            .filter(|f| f.id != "url" && f.id != "every")
            .all(|f| !f.value.trim().is_empty())
    }

    pub fn click(&mut self, x: f64, y: f64) -> KeyClick {
        // 🌐 navigateur : un seul bouton, Fermer (à la place d'Enregistrer) ; à côté aussi
        if self.navigateur {
            return if self.save_rect().contains(x, y) || !self.rect.contains(x, y) {
                KeyClick::Cancel
            } else {
                KeyClick::None
            };
        }
        if self.save_rect().contains(x, y) {
            return if self.complete() { KeyClick::Save } else { KeyClick::None };
        }
        if self.cancel_rect().contains(x, y) || !self.rect.contains(x, y) {
            return KeyClick::Cancel;
        }
        if let Some(i) = (0..self.fields.len()).find(|&i| self.field_rect(i).contains(x, y)) {
            self.focus = i;
        }
        KeyClick::None
    }

    /// Texte tapé ou collé dans le champ choisi (sans espaces ni retours de ligne autour).
    pub fn type_text(&mut self, s: &str) {
        if let Some(f) = self.fields.get_mut(self.focus) {
            f.value.extend(s.chars().filter(|c| !c.is_control() && !c.is_whitespace()));
        }
    }

    pub fn backspace(&mut self) {
        if let Some(f) = self.fields.get_mut(self.focus) {
            f.value.pop();
        }
    }

    /// Tab : champ suivant.
    pub fn next(&mut self) {
        self.focus = (self.focus + 1) % self.fields.len().max(1);
    }

    /// Oublie tout ce qui a été tapé (après l'envoi au trousseau, ou Annuler), et le texte de la carte (🆘 le code de
    /// secours affiché).
    pub fn wipe(&mut self) {
        let textes = self.fields.iter_mut().map(|f| &mut f.value).chain(self.avis.iter_mut().map(|a| &mut a.1));
        textes.for_each(effacer);
        self.avis.clear();
    }

    /// 🆘 The box that shows the rescue code of the journal key: the code in the card, in short lines (never cut
    /// inside a group), with what to do with it.
    pub fn code_secours(code: &str, win_w: f64, win_h: f64) -> Self {
        let groupes: Vec<&str> = code.split('-').collect();
        let mut lignes = vec!["Recopie-le sur papier, sans faute, et garde-le à l'abri. Jamais en photo, ni dans un fichier ou un message.".to_string()];
        lignes.extend(groupes.chunks(4).map(|g| format!("• {}", g.join("-"))));
        lignes.push("• Ce code disparaît de l'écran quand tu fermes cette boîte.".into());
        Self::new("journal-code", "", win_w, win_h).avec_avis(lignes)
    }

    pub fn draw(&self, text: &Text, mouse: (f64, f64)) -> Option<(Pixmap, Rect)> {
        let r = self.rect;
        let mut pm = crate::px::new(r.w as u32, r.h as u32)?;
        let loc = |q: Rect| Rect::new(q.x - r.x, q.y - r.y, q.w, q.h);
        // (fond opaque : les courbes de la chart ne se voient pas au travers ; liseré du verre, rayon de .fen)
        round_fill(&mut pm, loc(r), crate::jetons::R_FENETRE, crate::jetons::FOND, 0.97);
        crate::jetons::verre(&mut pm, loc(r), crate::jetons::R_FENETRE);
        let title = match self.exchange.as_str() {
            "hl" => format!("Compte {}", self.name),
            "claude" => "🔐 Clé Claude".to_string(),
            "claude-unlock" => "🔓 Déverrouiller Claude".to_string(),
            "oanda" => "🏦 Jeton OANDA".to_string(),
            "account-new" => "✨ Créer un compte Argos".to_string(),
            "account-verify" => "✅ Vérification des mots".to_string(),
            "account-login" => "🔑 J'ai déjà une clé".to_string(),
            "account-passkey" => "🔐 Activer Face ID / empreinte".to_string(),
            "custom" => "➕ API personnalisée".to_string(),
            "journal-secours" => "🆘 Code de secours du journal".to_string(),
            "journal-code" => "🆘 Ton code de secours".to_string(),
            _ => format!("Clé API — {}", self.name),
        };
        text.draw(&mut pm, &title, (18.0, 32.0), 16.0, WHITE, Align::Left);
        let warn = match self.exchange.as_str() {
            "hl" => "Adresse publique seulement : jamais de clé privée ni de phrase secrète.",
            "claude" => "Chiffrée avec ton code sur cet appareil ; envoyée seulement à Claude.",
            "claude-unlock" => "Ton code ouvre le coffre de ta clé, le temps de la session.",
            "oanda" => "Un compte démo suffit (vrais prix). Le jeton reste dans ton ordi ; navigateur : le temps de la session.",
            "account-new" => "Ni courriel ni serveur : une clé de 24 mots, chiffrée ici avec ton code.",
            "account-verify" => "Écris les mots demandés, tels que tu les as notés.",
            "account-login" => "Jamais la phrase de ton wallet crypto : seulement ta clé Argos (en français).",
            "account-passkey" => "Ton code ouvre les clés du compte ; Face ID les protège ensuite sur cet appareil.",
            "custom" => "Clé en lecture seule, sans retrait. Gardée dans ton ordi sous ce nom.",
            "cmc" => "Clé gratuite « Basic » sur coinmarketcap.com/api : elle reste dans ton ordi.",
            "journal-secours" | "journal-code" => "Il rouvre ton journal chiffré si le trousseau de ton ordi est perdu.",
            _ => "Crée une clé en lecture seule, sans retrait. Elle reste dans ton ordi.",
        };
        text.draw(&mut pm, &text.fit(warn, 12.0, r.w - 36.0), (18.0, 58.0), 12.0, GOLD, Align::Left);
        if !self.avis.is_empty() {
            crate::jetons::verre(&mut pm, Rect::new(18.0, 70.0, r.w - 36.0, self.avis_h() - 8.0), 10.0);
            for (k, (retrait, l)) in self.avis.iter().enumerate() {
                // la phrase d'en-tête en or, la liste (« • … ») en blanc
                let couleur = if self.avis[..=k].iter().any(|x| x.1.starts_with('•')) {
                    WHITE
                } else {
                    GOLD
                };
                text.draw(
                    &mut pm,
                    l,
                    ((30.0 + retrait) as f32, (88.0 + k as f64 * AVIS_L) as f32),
                    AVIS_T,
                    couleur,
                    Align::Left,
                );
            }
        }
        // 🌐 navigateur : les étapes pour l'app du PC, et un seul bouton Fermer
        if self.navigateur {
            let mut y = 92.0;
            for (k, l) in NAVIGATEUR_TXT.iter().enumerate() {
                for ligne in crate::assistant::wrap(text, &crate::actions::sans_touches(l), 13.0, r.w - 36.0) {
                    let c = if k == 0 { WHITE } else { crate::jetons::TEXTE };
                    text.draw(&mut pm, &ligne, (18.0, y as f32), 13.0, c, Align::Left);
                    y += 19.0;
                }
                y += if k == 0 { 8.0 } else { 2.0 };
            }
            let s = loc(self.save_rect());
            let hov = self.save_rect().contains(mouse.0, mouse.1);
            round_fill(&mut pm, s, 6.0, if hov { pal::hover() } else { accent() }, 1.0);
            text.draw(
                &mut pm,
                "Fermer",
                (s.cx() as f32, (s.cy() + 5.0) as f32),
                14.0,
                crate::palette::ink_on(accent()),
                Align::Center,
            );
            return Some((pm, r));
        }
        for (i, f) in self.fields.iter().enumerate() {
            let q = loc(self.field_rect(i));
            text.draw(&mut pm, &f.label, (q.x as f32, (q.y - 7.0) as f32), 12.5, GREY, Align::Left);
            let on = i == self.focus;
            round_fill(&mut pm, q, 6.0, pal::field(), 1.0);
            round_stroke(&mut pm, q, 6.0, if on { accent() } else { pal::edge_soft() }, 1.2);
            // jamais en clair : des points (une adresse publique, elle, se lit)
            let shown: String = if f.masked {
                "•".repeat(f.value.chars().count().min(48))
            } else {
                f.value.clone()
            };
            let (s, col) = if shown.is_empty() {
                (crate::actions::sans_touches("Colle ou tape ici (Ctrl + V)"), DIM)
            } else {
                (shown, WHITE)
            };
            text.draw(&mut pm, &s, ((q.x + 10.0) as f32, (q.cy() + 5.0) as f32), 14.0, col, Align::Left);
            if on {
                let cx = if f.value.is_empty() {
                    q.x + 10.0
                } else {
                    q.x + 11.0 + text.width(&s, 14.0) as f64
                };
                crate::fill(&mut pm, cx, q.cy() - 9.0, 1.5, 18.0, WHITE, 1.0);
            }
        }
        let (c, s) = (self.cancel_rect(), self.save_rect());
        let hov = |q: &Rect| q.contains(mouse.0, mouse.1);
        // nothing to type (🆘 the code shown): one button, Fermer, and no keyboard hint
        let champs = !self.fields.is_empty();
        if champs {
            round_fill(&mut pm, loc(c), 6.0, if hov(&c) { pal::hover() } else { pal::field() }, 1.0);
            round_stroke(&mut pm, loc(c), 6.0, pal::edge_soft(), 1.0);
            text.draw(&mut pm, "Annuler", (loc(c).cx() as f32, (loc(c).cy() + 5.0) as f32), 14.0, WHITE, Align::Center);
        }
        let ok = self.complete();
        round_fill(&mut pm, loc(s), 6.0, if ok { accent() } else { pal::hover() }, 1.0);
        let go = match self.exchange.as_str() {
            "account-new" => "Générer",
            "account-verify" => "Créer",
            "account-login" => "Me connecter",
            "account-passkey" => "Activer",
            "journal-secours" => "Valider",
            "journal-code" => "Fermer",
            _ => "Enregistrer",
        };
        text.draw(
            &mut pm,
            go,
            (loc(s).cx() as f32, (loc(s).cy() + 5.0) as f32),
            14.0,
            if ok { crate::palette::ink_on(accent()) } else { GREY },
            Align::Center,
        );
        if !champs {
            return Some((pm, r));
        }
        text.draw(
            &mut pm,
            // (📱 phone view: keyboard only, left out)
            &crate::actions::sans_touches("Tab = champ suivant · Entrée = enregistrer · Échap = annuler"),
            (18.0, (r.h - 68.0) as f32),
            11.0,
            DIM,
            Align::Left,
        );
        Some((pm, r))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<ExRow> {
        vec![
            ExRow {
                id: "hl".into(),
                name: "Hyperliquid".into(),
                enabled: true,
                symbols: 230,
                age_s: Some(3600 * 3),
                ..Default::default()
            },
            ExRow {
                id: "okx".into(),
                name: "OKX".into(),
                has_key: true,
                ..Default::default()
            },
        ]
    }

    #[test]
    fn panneau() {
        let r = rows();
        let z = layout(&r, 340.0);
        let at = |h: Hit| z.iter().find(|(_, x)| *x == h).unwrap().0;
        let q = at(Hit::Add);
        assert_eq!(click(&r, q.cx(), q.cy(), 340.0), ExClick::Action(ExAction::Add));
        let q = at(Hit::Toggle(1));
        assert_eq!(click(&r, q.cx(), q.cy(), 340.0), ExClick::Action(ExAction::Toggle("okx".into())));
        let q = at(Hit::Key(0));
        assert_eq!(click(&r, q.cx(), q.cy(), 340.0), ExClick::Action(ExAction::Key("hl".into())));
        let q = at(Hit::Forget(1));
        assert_eq!(
            click(&r, q.cx(), q.cy(), 340.0),
            ExClick::Action(ExAction::Forget("okx".into())),
            "✕ par-dessus la ligne de la clé"
        );
        assert!(!z.iter().any(|(_, h)| *h == Hit::Forget(0)), "pas de ✕ sans clé");
        assert_eq!((r[0].dot(), r[1].dot()), (accent(), DIM), "activé sans clé : bleu ; coupé : gris");
        assert_eq!(ago(3 * 3600), "il y a 3 h");
        let mut pm = crate::px::new(340, content_h(&r) as u32).unwrap();
        draw(&r, &mut pm, &Text::default(), 340.0, None, Rgb(30, 30, 30));
        if let Ok(p) = std::env::var("ARGOS_API_PNG") {
            pm.save_png(p).unwrap();
        }
    }

    /// 📱 Au doigt : ↻ et l'interrupteur se touchent sur 44 × 44 au moins (même un peu à côté du dessin), sans se
    /// chevaucher ni toucher la ligne de la clé ; le PC garde les zones du dessin.
    #[test]
    fn au_doigt_boutons_de_44() {
        let r = rows();
        let z = layout(&r, 340.0);
        let at = |h: Hit| z.iter().find(|(_, x)| *x == h).unwrap().0;
        let (rf, tg, key) = (at(Hit::Refresh(1)), at(Hit::Toggle(1)), at(Hit::Key(1)));
        crate::actions::set_tactile(true);
        let (drf, dtg) = (zone_du_doigt(rf, &Hit::Refresh(1)), zone_du_doigt(tg, &Hit::Toggle(1)));
        let bord_haut = click(&r, tg.cx(), tg.cy() - 20.0, 340.0);
        let a_gauche = click(&r, rf.cx() - 20.0, rf.cy(), 340.0);
        crate::actions::set_tactile(false);
        assert!(drf.w >= 44.0 && drf.h >= 44.0 && dtg.w >= 44.0 && dtg.h >= 44.0, "{drf:?} {dtg:?}");
        assert!(drf.x + drf.w <= dtg.x && dtg.y + dtg.h <= key.y, "rien ne se chevauche");
        assert_eq!(bord_haut, ExClick::Action(ExAction::Toggle("okx".into())));
        assert_eq!(a_gauche, ExClick::Action(ExAction::Refresh("okx".into())));
        assert_eq!(click(&r, tg.cx(), tg.cy() - 20.0, 340.0), ExClick::None, "PC : la zone du dessin");
    }

    #[test]
    fn boite_de_cle() {
        let mut b = KeyBox::new("okx", "OKX", 1400.0, 900.0);
        assert_eq!(b.fields.len(), 3, "OKX : clé, secret, passphrase");
        assert!(b.fields.iter().all(|f| f.masked));
        b.type_text(" abc\n");
        b.backspace();
        assert_eq!(b.fields[0].value, "ab", "espaces et retours de ligne ignorés");
        assert!(!b.complete());
        let s = b.save_rect();
        assert_eq!(b.click(s.cx(), s.cy()), KeyClick::None, "incomplet : pas d'enregistrement");
        b.next();
        b.type_text("s3cr3t");
        b.next();
        b.type_text("pass");
        assert_eq!(b.click(s.cx(), s.cy()), KeyClick::Save);
        b.wipe();
        assert!(b.fields.iter().all(|f| f.value.is_empty()), "tout est oublié");
        // Hyperliquid : l'adresse publique seulement, lisible
        let h = KeyBox::new("hl", "Hyperliquid", 1400.0, 900.0);
        assert_eq!((h.fields.len(), h.fields[0].id, h.fields[0].masked), (1, "address", false));
        assert!(b.draw(&Text::default(), (0.0, 0.0)).is_some());
    }

    /// ⚠️ L'avertissement de la boîte (📱 cell 390 × 844 et PC) : coupé à la largeur, dans une carte en verre au-dessus
    /// des champs, la boîte grandit et reste dans l'écran.
    #[test]
    fn avertissement_dans_la_boite() {
        let lignes = vec![
            "⚠️ Les données quittent l'appareil : à chaque question, vers Claude (Anthropic) :".to_string(),
            "• les 60 dernières bougies".to_string(),
            "• tes notes 🧠 Mémoire et ta méthode (sauf en 🧪 Sandbox), et ce qui est retenu".to_string(),
        ];
        let text = Text::default();
        let puce = text.width("• ", AVIS_T) as f64;
        for (w, h) in [(390.0, 844.0), (1400.0, 900.0)] {
            let nu = KeyBox::new("claude", "Claude", w, h);
            let b = KeyBox::new("claude", "Claude", w, h).avec_avis(lignes.clone());
            assert!(
                b.avis.len() >= 3 && b.avis.iter().all(|(r, l)| r + text.width(l, AVIS_T) as f64 <= b.rect.w - 60.0),
                "{:?}",
                b.avis
            );
            let textes: Vec<&str> = b.avis.iter().map(|x| x.1.as_str()).collect();
            assert_eq!(textes.join(" "), lignes.join(" "), "rien de perdu en coupant");
            // une ligne commence par « • » ou par le texte de l'en-tête sans retrait ; la suite d'une puce s'aligne
            // sous son texte
            let mut dans_la_liste = false;
            for (r, l) in &b.avis {
                dans_la_liste |= l.starts_with('•');
                let attendu = if dans_la_liste && !l.starts_with('•') { puce } else { 0.0 };
                assert_eq!(*r, attendu, "retrait de « {l} »");
            }
            assert!(b.rect.h > nu.rect.h && b.rect.y >= 6.0 && b.rect.y + b.rect.h <= h, "{:?}", b.rect);
            assert!(b.field_rect(0).y >= b.rect.y + 70.0 + b.avis_h(), "les champs sous la carte");
            assert!(b.draw(&text, (0.0, 0.0)).is_some());
        }
        assert!(
            KeyBox::new("okx", "OKX", 1400.0, 900.0).avis.is_empty(),
            "les autres boîtes : pas d'avertissement"
        );
    }

    /// 🌐 Dans le navigateur, la boîte d'une clé rangée au trousseau n'a ni champ ni bouton grisé : les étapes pour l'app
    /// du PC et un seul bouton Fermer (lui ou un clic à côté la ferment, un clic dans le texte ne fait rien) ; les boîtes du
    /// compte Argos et de Claude gardent leurs champs.
    #[test]
    fn boite_de_cle_dans_le_navigateur() {
        let mut b = KeyBox::new("binance", "Binance", 1400.0, 900.0);
        b.navigateur = true;
        let t = Text::default();
        assert!(b.draw(&t, (0.0, 0.0)).is_some());
        let r = b.rect;
        assert_eq!(b.click(r.x + 40.0, r.y + 120.0), KeyClick::None, "le texte ne fait rien");
        let s = b.save_rect();
        assert_eq!(b.click(s.cx(), s.cy()), KeyClick::Cancel, "Fermer");
        assert_eq!(b.click(r.x - 20.0, r.y), KeyClick::Cancel, "à côté");
        assert!(par_le_trousseau("binance") && par_le_trousseau("oanda"));
        assert!(!par_le_trousseau("cmc"), "CoinMarketCap : par le serveur du PC, sa boîte garde son champ");
        assert!(!par_le_trousseau("claude") && !par_le_trousseau("account-new"));
        assert!(NAVIGATEUR_TXT[0].contains("app installée sur ton PC"));
    }

    /// 🆘 The rescue code box: its code shown in groups in the card, nothing to type, Fermer closes; closing wipes it.
    #[test]
    fn boite_du_code_de_secours() {
        let code = "0123A-BCDEF-GHJKM-NPQRS-TVWXY-Z0123-45678-9ABCD-EFGHJ-KMNPQ-RSTVW";
        let mut b = KeyBox::code_secours(code, 1400.0, 900.0);
        let tout: String = b.avis.iter().map(|a| a.1.replace("• ", "")).collect::<Vec<_>>().join("-");
        assert!(tout.contains(code), "{tout}");
        assert!(b.fields.is_empty() && b.complete());
        assert!(b.draw(&Text::default(), (0.0, 0.0)).is_some());
        let s = b.save_rect();
        assert_eq!(b.click(s.cx(), s.cy()), KeyClick::Save);
        b.wipe();
        assert!(b.avis.is_empty(), "the code leaves the screen");
        let mut d = KeyBox::new("journal-secours", "", 1400.0, 900.0);
        assert!(d.fields[0].masked && !d.complete());
        d.type_text("AFFICHER");
        assert!(d.complete());
        // 📸 ARGOS_SECOURS_PNG=folder: both boxes (made-up code), to look at them
        if let Ok(dir) = std::env::var("ARGOS_SECOURS_PNG") {
            let d = d.avec_avis(vec!["Note-le sur papier.".into(), "• Pour le voir : tape AFFICHER, puis Valider.".into()]);
            for (b, nom) in [(&d, "demande"), (&KeyBox::code_secours(code, 1400.0, 900.0), "code")] {
                b.draw(&Text::default(), (0.0, 0.0))
                    .unwrap()
                    .0
                    .save_png(format!("{dir}/secours-{nom}.png"))
                    .unwrap();
            }
        }
    }

    /// 🔑 La boîte est opaque : à l'intérieur de son cadre arrondi, rien de ce qui est dessous ne se voit (tableaux des
    /// indicateurs, courbes).
    #[test]
    fn boite_de_cle_opaque() {
        let b = KeyBox::new("cmc", "CoinMarketCap", 1400.0, 900.0);
        let (pm, r) = b.draw(&Text::default(), (0.0, 0.0)).unwrap();
        let k = crate::jetons::R_FENETRE;
        let mut transparents = 0;
        for y in (k as u32..(r.h - k) as u32).step_by(7) {
            for x in (k as u32..(r.w - k) as u32).step_by(7) {
                if pm.pixel(x, y).unwrap().alpha() < 240 {
                    transparents += 1;
                }
            }
        }
        assert_eq!(transparents, 0, "aucun pixel laisse voir dessous");
    }
}
