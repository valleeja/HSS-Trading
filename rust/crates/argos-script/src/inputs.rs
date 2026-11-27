//! ⚙️ Inputs (réglages) des indicateurs, comme Argos v1 : une table par indicateur (groupe, nom, défaut, min, max, pas).
//! Les valeurs sont gardées dans l'ordre de la table (`f64` ; une case à cocher vaut 0 ou 1). La fenêtre de réglages
//! lit la table pour se dessiner, l'indicateur lit les valeurs pour calculer.

/// Sorte d'input, comme Pine Script v6 (`input.*`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    /// Nombre : (min, max, pas) — `input.int` / `input.float`.
    Num(f64, f64, f64),
    /// Case à cocher — `input.bool`.
    Bool,
    /// Liste : la valeur est l'index de l'option — `options`, `input.string` à options, `input.enum`, `input.timeframe`.
    Choice(&'static [&'static str]),
    /// Couleur : 0xRRGGBB, plus la transparence (0 à 100, comme Pine) dans les bits 24 à 30 — `input.color`.
    Color,
    /// Prix, qu'on peut aussi glisser sur la chart (ligne horizontale) — `input.price`.
    Price,
    /// Heure (millisecondes Unix), qu'on peut glisser sur la chart (ligne verticale) — `input.time`.
    Time,
    /// Session « HHMM-HHMM » : début × 10 000 + fin, en minutes depuis minuit — `input.session`.
    Session,
    /// Texte sur une ligne, ou plusieurs (`multi`) — `input.string` / `input.text_area`. Le texte est gardé à part
    /// (`Texts`), la valeur numérique ne sert pas.
    Text { multi: bool },
    /// Symbole (« BTC », « hl:ETH »…) — `input.symbol`. Gardé à part comme un texte.
    Symbol,
    /// Source : un prix (`SOURCES`) ou la sortie d'un autre indicateur de la chart (`source_ext`) — `input.source`.
    Source,
}

/// Unités de temps des listes de v1 (TFSEC : 5m … 1W), en secondes, dans le même ordre.
pub const TF_SECS: [i64; 11] = [300, 900, 1_800, 3_600, 7_200, 14_400, 18_000, 43_200, 86_400, 172_800, 604_800];

/// Sources de v1 (« Clôture », « (H + L + C)/3 »…), dans le même ordre.
pub const SOURCES: [&str; 9] = [
    "Clôture",
    "Ouverture",
    "Haut",
    "Bas",
    "(H + L)/2",
    "(H + L + C)/3",
    "(O + H + L + C)/4",
    // (added at the end, Jee 18 Nov: the saved indices 0 to 6 keep their source)
    "Volume",
    "(H + L + C + C)/4",
];

/// The Source menu in TradingView's order (Ouverture, Haut, Bas, Clôture, Volume, (H + L)/2, (H + L + C)/3,
/// (O + H + L + C)/4, (H + L + C + C)/4): indices in `SOURCES`.
pub const SOURCES_ORDRE_TV: [usize; 9] = [1, 2, 3, 0, 7, 4, 5, 6, 8];

/// Un input : clé, groupe (index dans les groupes de l'indicateur), nom affiché, valeur par défaut, sorte, et les
/// options de Pine v6 : aide (`tooltip`), même ligne (`inline`), actif seulement si une case est cochée (`active`),
/// valeur montrée dans la légende (`display`), demandée à l'ajout sur la chart (`confirm`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Param {
    pub key: &'static str,
    pub group: usize,
    pub label: &'static str,
    pub def: f64,
    pub kind: Kind,
    /// Aide (« ? » au bout du champ ; `\n` pour une nouvelle ligne). Vide : pas d'aide.
    pub tip: &'static str,
    /// Les inputs du même groupe avec le même `inline` (non vide) sont sur une seule ligne.
    pub inline: &'static str,
    /// Clé d'une case à cocher : décochée, cet input est grisé et ne se change pas. Vide : toujours actif.
    pub active: &'static str,
    /// Valeur montrée à côté du nom dans la légende (par défaut : oui, sauf case à cocher et couleur, comme Pine).
    pub display: bool,
    /// Demandé dans « Confirmer les inputs » quand on ajoute l'indicateur ; un prix ou une heure se choisit alors en
    /// cliquant sur la chart.
    pub confirm: bool,
    /// Texte par défaut (`Text`, `Symbol`).
    pub text: &'static str,
}

const fn base(key: &'static str, group: usize, label: &'static str, def: f64, kind: Kind, display: bool) -> Param {
    Param {
        key,
        group,
        label,
        def,
        kind,
        tip: "",
        inline: "",
        active: "",
        display,
        confirm: false,
        text: "",
    }
}

/// Nombre entier (pas 1).
pub const fn int(key: &'static str, group: usize, label: &'static str, def: f64, min: f64, max: f64) -> Param {
    base(key, group, label, def, Kind::Num(min, max, 1.0), true)
}

/// Nombre décimal.
pub const fn num(key: &'static str, group: usize, label: &'static str, def: f64, min: f64, max: f64, step: f64) -> Param {
    base(key, group, label, def, Kind::Num(min, max, step), true)
}

/// Case à cocher.
pub const fn check(key: &'static str, group: usize, label: &'static str, def: bool) -> Param {
    base(key, group, label, def as u8 as f64, Kind::Bool, false)
}

/// Liste (`def` : index de l'option par défaut).
pub const fn choice(key: &'static str, group: usize, label: &'static str, def: usize, opts: &'static [&'static str]) -> Param {
    base(key, group, label, def as f64, Kind::Choice(opts), true)
}

/// Couleur (« #rrggbb » écrit en 0xRRGGBB), opaque.
pub const fn color(key: &'static str, group: usize, label: &'static str, rgb: u32) -> Param {
    base(key, group, label, rgb as f64, Kind::Color, false)
}

/// Couleur avec transparence (0 = opaque, 100 = invisible, comme Pine).
pub const fn color_t(key: &'static str, group: usize, label: &'static str, rgb: u32, transp: u32) -> Param {
    base(key, group, label, (rgb | (transp << 24)) as f64, Kind::Color, false)
}

/// Prix (glissable sur la chart).
pub const fn price(key: &'static str, group: usize, label: &'static str, def: f64) -> Param {
    base(key, group, label, def, Kind::Price, true)
}

/// Heure (millisecondes Unix, glissable sur la chart).
pub const fn time(key: &'static str, group: usize, label: &'static str, def_ms: f64) -> Param {
    base(key, group, label, def_ms, Kind::Time, true)
}

/// Session « HHMM-HHMM » (ex. « 0930-1600 »).
pub const fn session(key: &'static str, group: usize, label: &'static str, def: &'static str) -> Param {
    base(key, group, label, session_val(def), Kind::Session, true)
}

/// Texte sur une ligne (`input.string` sans options).
pub const fn string(key: &'static str, group: usize, label: &'static str, def: &'static str) -> Param {
    Param {
        text: def,
        ..base(key, group, label, 0.0, Kind::Text { multi: false }, true)
    }
}

/// Texte sur plusieurs lignes (`input.text_area`, pas dans la légende).
pub const fn text_area(key: &'static str, group: usize, label: &'static str, def: &'static str) -> Param {
    Param {
        text: def,
        ..base(key, group, label, 0.0, Kind::Text { multi: true }, false)
    }
}

/// Symbole (`input.symbol`).
pub const fn symbol(key: &'static str, group: usize, label: &'static str, def: &'static str) -> Param {
    Param {
        text: def,
        ..base(key, group, label, 0.0, Kind::Symbol, true)
    }
}

/// Source (`def` : index dans `SOURCES`).
pub const fn source(key: &'static str, group: usize, label: &'static str, def: usize) -> Param {
    base(key, group, label, def as f64, Kind::Source, true)
}

impl Param {
    /// Aide (`tooltip`).
    pub const fn tip(self, tip: &'static str) -> Self {
        Param { tip, ..self }
    }
    /// Sur la même ligne que les autres inputs du même `inline`.
    pub const fn inline(self, inline: &'static str) -> Self {
        Param { inline, ..self }
    }
    /// Actif seulement si la case `key` est cochée.
    pub const fn active_if(self, key: &'static str) -> Self {
        Param { active: key, ..self }
    }
    /// Montré (ou pas) dans la légende.
    pub const fn display(self, display: bool) -> Self {
        Param { display, ..self }
    }
    /// Demandé à l'ajout sur la chart.
    pub const fn confirm(self) -> Self {
        Param { confirm: true, ..self }
    }
}

/// « 0930-1600 » → 570 × 10 000 + 960 (minutes depuis minuit). Mal écrit : 0 (toute la journée).
pub const fn session_val(s: &str) -> f64 {
    let b = s.as_bytes();
    if b.len() != 9 || b[4] != b'-' {
        return 0.0;
    }
    let mut d = [0u32; 8];
    let mut k = 0;
    while k < 9 {
        if k != 4 {
            if !b[k].is_ascii_digit() {
                return 0.0;
            }
            d[if k < 4 { k } else { k - 1 }] = (b[k] - b'0') as u32;
        }
        k += 1;
    }
    let (h0, m0, h1, m1) = (d[0] * 10 + d[1], d[2] * 10 + d[3], d[4] * 10 + d[5], d[6] * 10 + d[7]);
    if h0 > 24 || h1 > 24 || m0 > 59 || m1 > 59 {
        return 0.0;
    }
    ((h0 * 60 + m0) * 10_000 + h1 * 60 + m1) as f64
}

/// Valeur de session → « 0930-1600 ».
pub fn session_txt(v: f64) -> String {
    let v = v.max(0.0) as u32;
    let (a, b) = (v / 10_000, v % 10_000);
    format!("{:02}{:02}-{:02}{:02}", a / 60, a % 60, b / 60, b % 60)
}

/// Source qui vient de la sortie `out` de l'indicateur `ind` (index dans la liste d'indicateurs de la chart).
pub fn source_ext(ind: usize, out: usize) -> f64 {
    ((ind + 1) * 1000 + out) as f64
}

/// Ce que vaut une source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Src {
    /// Index dans `SOURCES`.
    Price(usize),
    /// (indicateur, sortie).
    Ext(usize, usize),
}

impl Src {
    pub fn of(v: f64) -> Src {
        let v = v.max(0.0).round() as usize;
        if v >= 1000 {
            Src::Ext(v / 1000 - 1, v % 1000)
        } else {
            Src::Price(v.min(SOURCES.len() - 1))
        }
    }
}

/// Série d'un prix de `SOURCES` sur ces bougies.
pub fn price_series(bars: &[argos_core::Bar], k: usize) -> Vec<f64> {
    bars.iter()
        .map(|b| match k {
            1 => b.open,
            2 => b.high,
            3 => b.low,
            4 => (b.high + b.low) / 2.0,
            5 => (b.high + b.low + b.close) / 3.0,
            6 => (b.open + b.high + b.low + b.close) / 4.0,
            7 => b.volume,
            8 => (b.high + b.low + 2.0 * b.close) / 4.0,
            _ => b.close,
        })
        .collect()
}

/// Valeurs par défaut d'une table.
pub fn defaults(params: &[Param]) -> Vec<f64> {
    params.iter().map(|p| p.def).collect()
}

/// Valeur bornée et arrondie au pas (ce que la fenêtre garde quand on tape un nombre).
pub fn clamp(p: &Param, v: f64) -> f64 {
    match p.kind {
        Kind::Bool => (v != 0.0) as u8 as f64,
        Kind::Choice(o) => (v.round() as i64).rem_euclid(o.len().max(1) as i64) as f64,
        Kind::Color => {
            let c = v.round().max(0.0) as u32;
            ((c & 0xff_ffff) | (((c >> 24) & 0x7f).min(100) << 24)) as f64
        }
        Kind::Num(min, max, step) => {
            let v = if step > 0.0 { ((v / step).round() * step * 1e9).round() / 1e9 } else { v };
            v.clamp(min, max)
        }
        Kind::Session => {
            let v = v.max(0.0) as u32;
            let (a, b) = (v / 10_000, v % 10_000);
            if a <= 1440 && b <= 1440 {
                v as f64
            } else {
                p.def
            }
        }
        Kind::Source => {
            let v = v.max(0.0).round();
            if v >= 1000.0 || (v as usize) < SOURCES.len() {
                v
            } else {
                p.def
            }
        }
        Kind::Price | Kind::Time => {
            if v.is_finite() {
                v
            } else {
                p.def
            }
        }
        Kind::Text { .. } | Kind::Symbol => 0.0,
    }
}

/// Lecture des valeurs par clé (valeur par défaut si la liste est trop courte : réglages d'une ancienne version).
pub struct Inputs<'a> {
    pub params: &'a [Param],
    pub vals: &'a [f64],
}

impl Inputs<'_> {
    /// Valeur d'un input ; une clé inconnue (erreur de programmation) ne fait jamais planter Argos : 0, avec un
    /// message (les tests, eux, échouent : `debug_assert`).
    pub fn get(&self, key: &str) -> f64 {
        match self.params.iter().position(|p| p.key == key) {
            Some(k) => self.vals.get(k).copied().unwrap_or(self.params[k].def),
            None => {
                eprintln!("⚠️ input inconnu : {key}");
                #[cfg(test)]
                panic!("input inconnu : {key}");
                #[allow(unreachable_code)]
                0.0
            }
        }
    }
    pub fn len(&self, key: &str) -> usize {
        self.get(key).round().max(1.0) as usize
    }
    pub fn on(&self, key: &str) -> bool {
        self.get(key) != 0.0
    }
    /// Index de l'option choisie.
    pub fn pick(&self, key: &str) -> usize {
        self.get(key).round().max(0.0) as usize
    }
    /// Couleur (r, g, b).
    pub fn rgb(&self, key: &str) -> (u8, u8, u8) {
        let c = self.get(key) as u32;
        ((c >> 16) as u8, (c >> 8) as u8, c as u8)
    }
    /// Opacité de la couleur (1 = opaque ; Pine : transparence 0 à 100).
    pub fn opacity(&self, key: &str) -> f32 {
        1.0 - (((self.get(key) as u32) >> 24) & 0x7f).min(100) as f32 / 100.0
    }
    /// Session : (début, fin) en minutes depuis minuit.
    pub fn session(&self, key: &str) -> (u32, u32) {
        let v = self.get(key).max(0.0) as u32;
        (v / 10_000, v % 10_000)
    }
    /// La minute `m` (depuis minuit) est-elle dans la session ? (une session qui passe minuit est permise ; 0 = toujours)
    pub fn in_session(&self, key: &str, m: u32) -> bool {
        match self.session(key) {
            (0, 0) => true,
            (a, b) if a <= b => m >= a && m < b,
            (a, b) => m >= a || m < b,
        }
    }
    /// Source choisie.
    pub fn source(&self, key: &str) -> Src {
        Src::of(self.get(key))
    }
    /// L'input est-il actif (sa case `active` cochée, ou pas de case) ?
    pub fn active(&self, i: usize) -> bool {
        let a = self.params[i].active;
        a.is_empty() || self.on(a)
    }
    /// Valeurs montrées dans la légende (`display`), comme TradingView : « 14, 9, Clôture ». `texts` : les textes gardés.
    pub fn legend(&self, texts: &Texts) -> String {
        self.params
            .iter()
            .enumerate()
            .filter(|(_, p)| p.display)
            .map(|(i, p)| {
                let v = self.vals.get(i).copied().unwrap_or(p.def);
                match p.kind {
                    Kind::Num(..) | Kind::Price => {
                        let s = format!("{v:.4}");
                        s.trim_end_matches('0').trim_end_matches('.').to_string()
                    }
                    Kind::Choice(o) => o.get(v.max(0.0) as usize).copied().unwrap_or("").to_string(),
                    Kind::Source => match Src::of(v) {
                        Src::Price(k) => SOURCES[k].to_string(),
                        Src::Ext(..) => "↪".to_string(),
                    },
                    Kind::Session => session_txt(v),
                    Kind::Time => chrono_date(v),
                    Kind::Text { .. } | Kind::Symbol => texts.text(p.key).to_string(),
                    Kind::Bool | Kind::Color => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// « 2026-10-26 14:30 » (UTC) pour la légende d'une heure.
fn chrono_date(ms: f64) -> String {
    let s = (ms / 1000.0) as i64;
    let (days, rem) = (s.div_euclid(86_400), s.rem_euclid(86_400));
    // jours depuis 1970 → date (algorithme civil de H. Hinnant)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + (m <= 2) as i64;
    format!("{y}-{m:02}-{d:02} {:02}:{:02}", rem / 3600, rem % 3600 / 60)
}

/// Textes des inputs `Text` et `Symbol` (gardés à part des nombres) : la valeur gardée, sinon le texte par défaut.
pub struct Texts<'a> {
    pub params: &'a [Param],
    pub vals: &'a std::collections::HashMap<String, String>,
}

impl Texts<'_> {
    pub fn text(&self, key: &str) -> &str {
        self.vals
            .get(key)
            .map(String::as_str)
            .unwrap_or_else(|| self.params.iter().find(|p| p.key == key).map_or("", |p| p.text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inputs() {
        const T: [Param; 3] = [
            int("len", 0, "Longueur", 10.0, 2.0, 100.0),
            num("mult", 0, "Mult", 2.0, 1.0, 10.0, 0.1),
            check("show", 1, "Afficher", true),
        ];
        let v = defaults(&T);
        assert_eq!(v, vec![10.0, 2.0, 1.0]);
        let i = Inputs { params: &T, vals: &v[..2] };
        assert_eq!((i.len("len"), i.get("mult"), i.on("show")), (10, 2.0, true), "valeur manquante : défaut");
        assert_eq!((clamp(&T[0], 500.0), clamp(&T[1], 2.34), clamp(&T[2], 3.0)), (100.0, 2.3, 1.0));
    }

    #[test]
    fn inputs_comme_pine() {
        const T: [Param; 10] = [
            check("sig", 0, "Afficher le signal", true),
            int("slen", 0, "Longueur du signal", 9.0, 1.0, 50.0)
                .active_if("sig")
                .inline("s")
                .tip("Lissage\nde la ligne"),
            color_t("scol", 0, "Couleur", 0xffffff, 40).inline("s"),
            source("src", 0, "Source", 0),
            session("sess", 1, "Session", "0930-1600"),
            price("lvl", 1, "Niveau", 80_000.0).confirm(),
            time("t0", 1, "Départ", 1_793_039_400_000.0).inline("pt"),
            string("note", 2, "Note", "OK"),
            text_area("syms", 2, "Symboles", "BTC,ETH"),
            symbol("cmp", 2, "Comparer avec", "ETH"),
        ];
        let mut v = defaults(&T);
        let texts = std::collections::HashMap::new();
        let tx = Texts { params: &T, vals: &texts };
        let i = Inputs { params: &T, vals: &v };
        // active, inline, tip, display par défaut (case et couleur cachées, comme Pine)
        assert!(i.active(1));
        assert_eq!(
            (T[1].inline, T[1].tip, T[0].display, T[2].display, T[1].display),
            ("s", "Lissage\nde la ligne", false, false, true)
        );
        assert!(T[5].confirm);
        // couleur à 40 % de transparence
        assert_eq!(i.rgb("scol"), (255, 255, 255));
        assert!((i.opacity("scol") - 0.6).abs() < 1e-6);
        // session
        assert_eq!(i.session("sess"), (570, 960));
        assert_eq!(session_txt(i.get("sess")), "0930-1600");
        assert!(i.in_session("sess", 600) && !i.in_session("sess", 1000));
        assert_eq!(session_val("2200-0200"), (1320 * 10_000 + 120) as f64);
        assert_eq!(session_val("9h30-16h"), 0.0, "mal écrit : toute la journée");
        // source : prix, puis la sortie d'un autre indicateur
        assert_eq!(i.source("src"), Src::Price(0));
        let bars = [argos_core::Bar {
            time: 0,
            open: 1.0,
            high: 4.0,
            low: 0.0,
            close: 2.0,
            volume: 1.0,
        }];
        assert_eq!(price_series(&bars, 5), vec![2.0]);
        v[3] = source_ext(2, 1);
        let i = Inputs { params: &T, vals: &v };
        assert_eq!(i.source("src"), Src::Ext(2, 1));
        // textes gardés à part
        assert_eq!((tx.text("note"), tx.text("cmp")), ("OK", "ETH"));
        // légende comme TradingView
        assert_eq!(i.legend(&tx), "9, ↪, 0930-1600, 80000, 2026-10-26 18:30, OK, ETH");
        // décochée : la longueur est grisée
        v[0] = 0.0;
        assert!(!Inputs { params: &T, vals: &v }.active(1));
        // bornes
        assert_eq!(clamp(&T[2], (0x123456u32 | (120 << 24)) as f64), (0x123456u32 | (100 << 24)) as f64);
        assert_eq!(clamp(&T[3], 9.0), 0.0, "source inconnue : défaut");
        assert_eq!(clamp(&T[4], 99_999_999.0), T[4].def);
    }
}
