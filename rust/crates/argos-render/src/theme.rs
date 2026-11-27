//! 🎨 Thèmes de la chart (couleurs) et le thème choisi pour toute l'app.

use crate::*;

/// Couleurs de la chart (thème sombre Argos par défaut).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    pub background: Rgb,
    pub grid: Rgb,
    pub text: Rgb,
    pub axis_line: Rgb,
    pub bull: Rgb,
    pub bear: Rgb,
    /// Texte fort (titre de la chart).
    pub fg: Rgb,
    /// Fond des barres du haut et du bas (le thème clair garde des barres sombres pour l'instant).
    pub chrome: Rgb,
    /// Accent de l'interface (choix, onglets) et couleur vive (Connect, éléments allumés) : `palette::accent` / `cyan`.
    pub accent: Rgb,
    pub cyan: Rgb,
    /// Couleur du logo ARGOS (`None` : son bleu d'origine).
    pub logo: Option<Rgb>,
}

/// 🎨 Thème choisi (index dans `Theme::NAMES`).
pub(crate) static THEME: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(Theme::CLASSIC);

/// Choisit le thème de toute l'app (index dans `Theme::NAMES`).
pub fn set_theme(k: usize) {
    THEME.store(k.min(Theme::NAMES.len() - 1), std::sync::atomic::Ordering::Relaxed);
}

/// Index du thème choisi (dans `Theme::NAMES`).
pub fn theme_index() -> usize {
    THEME.load(std::sync::atomic::Ordering::Relaxed)
}

impl Theme {
    /// Thèmes : Défaut (couleurs du desktop, sombre ou clair), puis ceux d'Argos v1 : Argos (bleu), Or, Matrix, Classique.
    pub const NAMES: [&'static str; 10] = [
        "Défaut · sombre",
        "Clair",
        "🔷 Argos",
        "🥇 Or",
        "🟢 Matrix",
        "⚫ Classique",
        "Daltonien (rouge-vert)",
        "Daltonien (blanc, orange, bleu)",
        "🫧 Verre sombre",
        "🫧 Verre clair",
    ];
    /// 🫧 Thèmes verre : le style verre est allumé d'office (comme la case « Style verre », partout).
    pub const GLASS: [usize; 2] = [8, 9];

    /// Le thème `k` est-il un thème verre ?
    pub fn is_glass(k: usize) -> bool {
        Self::GLASS.contains(&k)
    }
    /// ⚫ Classique (noir de v1) : le thème par défaut.
    pub const CLASSIC: usize = 5;

    /// Le thème `k` (couleurs de la chart d'Argos v1 pour Argos, Or et Matrix).
    pub fn named(k: usize) -> Self {
        let d = Self::default();
        let dark = |bg: &str, grid: &str, text: &str, axis: &str, fg: &str| Self {
            background: Rgb::hex(bg),
            grid: Rgb::hex(grid),
            text: Rgb::hex(text),
            axis_line: Rgb::hex(axis),
            fg: Rgb::hex(fg),
            chrome: Rgb::hex(bg),
            ..d
        };
        match k {
            1 => Self {
                background: Rgb::hex("#ffffff"),
                grid: Rgb::hex("#f0f3fa"),
                text: Rgb::hex("#131722"),
                axis_line: Rgb::hex("#d1d4dc"),
                fg: Rgb::hex("#131722"),
                ..d
            },
            2 => Self {
                accent: palette::ACCENT,
                cyan: Rgb(0x3d, 0xc8, 0xf0),
                logo: None,
                ..dark("#090e18", "#0e1626", "#8394b3", "#17223a", "#e6f2ff")
            },
            3 => Self {
                logo: Some(Rgb::hex("#f0b90b")),
                ..dark("#0f0d09", "#15120c", "#a8977a", "#2a2316", "#f6eedb")
            },
            4 => Self {
                accent: Rgb::hex("#2e9e48"),
                cyan: Rgb::hex("#43c35e"),
                logo: Some(Rgb::hex("#43c35e")),
                ..dark("#020a04", "#04170a", "#43c35e", "#0c2a14", "#c4ffcc")
            },
            5 => dark("#000000", "#111111", "#b2b5be", "#1f1f1f", "#e6e8ee"),
            // 👁 daltonien rouge-vert : fond sombre, on garde le vert et le rouge mais ceux d'Okabe et Ito (vert bleuté
            // clair et vermillon foncé), qui restent distincts en deutéranopie et protanopie (teinte ET clarté)
            6 => Self {
                bull: Rgb::hex("#3fd6a6"),
                bear: Rgb::hex("#d55e00"),
                ..d
            },
            // daltonien blanc, orange, bleu : fond blanc, hausse bleu foncé, baisse orange foncé (lisibles sur le blanc)
            7 => Self {
                background: Rgb::hex("#ffffff"),
                grid: Rgb::hex("#eef1f6"),
                text: Rgb::hex("#131722"),
                axis_line: Rgb::hex("#d1d4dc"),
                fg: Rgb::hex("#131722"),
                bull: Rgb::hex("#1f5fbf"),
                bear: Rgb::hex("#d9590b"),
                ..d
            },
            // 🫧 verre sombre : bleu nuit profond (le flou de la chart ressort derrière le verre fumé)
            8 => Self {
                accent: palette::ACCENT,
                cyan: Rgb(0x5a, 0xd8, 0xff),
                ..dark("#0a0f1e", "#141b2e", "#a9b4cc", "#1d2640", "#eef2fb")
            },
            // 🫧 verre clair : blanc bleuté (verre laiteux par-dessus)
            9 => Self {
                background: Rgb::hex("#f7f9fd"),
                grid: Rgb::hex("#e9eef7"),
                text: Rgb::hex("#131722"),
                axis_line: Rgb::hex("#d1d8e6"),
                fg: Rgb::hex("#131722"),
                ..d
            },
            _ => d,
        }
    }

    /// Le thème choisi (`set_theme`).
    pub fn current() -> Self {
        Self::named(THEME.load(std::sync::atomic::Ordering::Relaxed))
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            background: Rgb(0x0d, 0x0f, 0x14),
            grid: Rgb(0x1c, 0x20, 0x29),
            text: Rgb(0xb2, 0xb5, 0xbe),
            axis_line: Rgb(0x2a, 0x2e, 0x39),
            bull: Rgb::hex("#089981"),
            bear: Rgb::hex("#f23645"),
            fg: Rgb(0xe6, 0xe8, 0xee),
            chrome: Rgb(0x0d, 0x0f, 0x14),
            // noir et gris : pas de bleu (le thème clair retourne ces gris en foncé)
            accent: Rgb(0x8a, 0x8e, 0x9a),
            cyan: Rgb(0xe6, 0xe8, 0xee),
            logo: Some(Rgb(0xf2, 0xf3, 0xf5)),
        }
    }
}
