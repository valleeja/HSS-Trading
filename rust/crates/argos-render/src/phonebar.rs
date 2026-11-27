//! 📱 Cell : les boutons du bas (icônes de la maquette) et la pastille DÉMO. Le dessin et le toucher de l'écran
//! principal du cell viennent des zones mesurées dans la maquette (`ecrans`) ; ce module garde les noms des boutons
//! (`Icone`, `PhoneHit`, utilisés par les actions) et la pastille DÉMO.

use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Text};
use tiny_skia::Pixmap;

/// Icône de la rangée du bas (maquette v6 tel-08 : dans cet ordre, puis ⚙ après un trait).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icone {
    Trading,
    Demo,
    PropFirm,
    Baleines,
    Modules,
    Script,
    Academie,
    Reglages,
    /// 📟 Tablette (maquette, à la suite de 🎓 quand il y a la place) : 👥 Communauté, 🌐 Macro, 🛒 Marketplace.
    Communaute,
    Macro,
    Marketplace,
}

impl Icone {
    pub const TOUTES: [Icone; 8] = [
        Icone::Trading,
        Icone::Demo,
        Icone::PropFirm,
        Icone::Baleines,
        Icone::Modules,
        Icone::Script,
        Icone::Academie,
        Icone::Reglages,
    ];
}

/// Bouton du bas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhoneHit {
    /// Nom du symbole : la watchlist.
    Symbol,
    /// Unité de temps.
    Interval,
    /// ƒx Indicateurs.
    Fx,
    /// 🟠 Pastille DÉMO (P&L du jour) : le Simulateur.
    Demo,
    /// Une icône de la rangée du bas.
    Outil(Icone),
    /// 🎡 Molette : le symbole ou l'unité de temps d'avant (en haut de la pilule) ou d'après (en bas).
    SymbolePrec,
    SymboleSuiv,
    IntervallePrec,
    IntervalleSuiv,
}

/// 🟠 Dessine la pastille DÉMO dans `r` : point orange, « DÉMO » en or, le montant à sa couleur (vert gain, rouge
/// perte). Une seule fonction : la barre du bas et la mini-pastille des barres cachées.
pub fn pastille_demo(pm: &mut Pixmap, text: &Text, r: Rect, t: &str, c: crate::text::Rgb) {
    let orange = crate::palette::ORANGE;
    let cy = r.cy();
    round_fill(pm, r, r.h / 2.0, crate::text::Rgb(0x2a, 0x1f, 0x10), 0.95);
    round_stroke(pm, r, r.h / 2.0, orange, 1.0);
    round_fill(pm, Rect::new(r.x + 8.0, cy - 3.0, 6.0, 6.0), 3.0, orange, 1.0);
    let (mot, montant) = t.split_once(' ').unwrap_or(("", t));
    let x0 = (r.x + 18.0) as f32;
    text.draw(pm, mot, (x0, (cy + 4.0) as f32), 12.0, crate::palette::GOLD, Align::Left);
    let mx = x0 + text.width(mot, 12.0) + if mot.is_empty() { 0.0 } else { 4.0 };
    text.draw(pm, montant, (mx, (cy + 4.0) as f32), 12.0, c, Align::Left);
}

/// 🟠 Largeur de la pastille DÉMO pour ce texte.
pub fn pastille_w(text: &Text, t: &str) -> f64 {
    text.width(t, 12.0) as f64 + 24.0
}

#[cfg(test)]
mod tests_clair {
    use super::*;

    /// 📸 Thème clair (le verre passé en clair) : Argos AI ; seulement si
    /// `ARGOS_CAPTURES`.
    #[test]
    fn captures_clair() {
        let Ok(dir) = std::env::var("ARGOS_CAPTURES") else { return };
        let t = Text::default();
        let avant = crate::theme_index();
        crate::set_theme(1);
        let ai = crate::assistant::Assistant {
            key: crate::assistant::KeyState::Ready,
            lieu: "Tu es sur la chart : BTC-USDC.P · 4h".into(),
            iv: "4h".into(),
            msgs: vec![(false, "Structure 4h : BOS haussier.".into()), (true, "Explique-moi la zone".into())],
            ..Default::default()
        };
        let mut a = ai.draw(&t, 420, 700, None).unwrap();
        crate::palette::light_ui(&mut a);
        a.save_png(format!("{dir}/clair-argos-ai.png")).unwrap();
        crate::set_theme(avant);
    }

    /// 🌞 Thème clair (Jee, 2026-11-09) : la puce 📍 d'Argos AI reste lisible après le passage en clair (fond clair,
    /// texte foncé) — avant, un bleu nuit gardé tel quel rendait son texte illisible. Sans toucher au thème global.
    #[test]
    fn argos_ai_lisible_en_clair() {
        let t = Text::default();
        let mut a = crate::px::new(420, 60).unwrap();
        crate::fill(&mut a, 0.0, 0.0, 420.0, 60.0, crate::jetons::FOND, 1.0);
        crate::fill(&mut a, 0.0, 0.0, 420.0, 60.0, crate::jetons::VERRE.0, crate::jetons::VERRE.1);
        let ((c, al), encre) = crate::assistant::teinte_accent_pour(true);
        let r = Rect::new(10.0, 10.0, 400.0, 34.0);
        round_fill(&mut a, r, 12.0, c, al);
        t.draw(&mut a, "📍 Tu es sur la chart : BTC-USDC.P · 4h", (22.0, 32.0), 13.5, encre, Align::Left);
        crate::palette::light_ui(&mut a);
        let lum = |x: u32, y: u32| {
            let p = a.pixel(x, y).unwrap().demultiply();
            (p.red() as u32 + p.green() as u32 + p.blue() as u32) / 3
        };
        let fond = lum(395, 18);
        let plus_sombre = (30..300).flat_map(|x| (18..38).map(move |y| (x, y))).map(|(x, y)| lum(x, y)).min().unwrap();
        assert!(fond > 150, "fond clair : {fond}");
        assert!(plus_sombre < 90, "texte foncé : {plus_sombre}");
    }
}
