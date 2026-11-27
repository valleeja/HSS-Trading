//! 📐 En-tête « titre + bouton(s) à droite » : le titre se coupe avec « … » avant le premier bouton (écart 8 px), à
//! toute largeur ; jamais l'un sur l'autre. Le même calcul sert au dessin et au test commun des en-têtes.

use crate::shapes::Rect;
use crate::text::Text;

/// Écart entre la fin du titre et le premier bouton.
pub const ECART: f64 = 8.0;

/// Un titre posé à gauche de ses boutons.
#[derive(Debug, Clone, PartialEq)]
pub struct Entete {
    /// Le titre à écrire (coupé avec « … » s'il ne tient pas).
    pub titre: String,
    /// La place du titre écrit (de sa ligne de base − taille à sa ligne de base + un quart de taille).
    pub titre_r: Rect,
}

/// Titre `s` (taille `size`) écrit à partir de `x`, ligne de base `base`, avant les boutons `boutons`.
pub fn placer(text: &Text, s: &str, size: f32, (x, base): (f64, f64), boutons: &[Rect]) -> Entete {
    let fin = boutons.iter().map(|b| b.x).fold(f64::INFINITY, f64::min);
    let max = fin - ECART - x;
    let titre = if !fin.is_finite() {
        s.to_string()
    } else if (text.width("…", size) as f64) > max {
        String::new()
    } else {
        text.fit(s, size, max)
    };
    let w = text.width(&titre, size) as f64;
    let sz = size as f64;
    Entete {
        titre,
        titre_r: Rect::new(x, base - sz, w, sz * 1.25),
    }
}

/// Deux places se croisent-elles ?
pub fn croise(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titre_coupe_avant_le_bouton() {
        let t = Text::default();
        let b = Rect::new(120.0, 5.0, 70.0, 20.0);
        let e = placer(&t, "Gardien PropFirm · journée verrouillée", 13.0, (25.0, 20.0), &[b]);
        assert!(e.titre.ends_with('…'));
        assert!(!croise(e.titre_r, b) && e.titre_r.x + e.titre_r.w <= b.x - ECART + 0.5);
        let court = placer(&t, "API", 13.0, (25.0, 20.0), &[b]);
        assert_eq!(court.titre, "API");
    }
}

/// 📐 Test commun des en-têtes « titre + bouton à droite » : de 220 à 480 de large (tous les 10 px), le titre écrit ne
/// croise jamais son bouton (carte du gardien et en-tête de la feuille Trading, volets Trading / Journal / Indicateurs,
/// cartes de la liste des sources, en-tête de chaque module de la colonne).
#[cfg(test)]
mod tous_les_entetes {
    use super::*;
    use crate::panels::{Kind, Panels};

    #[test]
    fn aucun_titre_sous_son_bouton() {
        let t = Text::default();
        let noms = ["OANDA (forex, or, indices)", "CoinMarketCap (BTC.D USDT.D, capitalisations)", "Hyperliquid"];
        for pw in (220..=480).step_by(10).map(f64::from) {
            let mut cas: Vec<(String, Entete, Rect)> = vec![];
            for etat in ["dans les règles", "moitié d'une limite", "journée verrouillée", "limite atteinte", "éteint"] {
                for (e, b) in crate::panels::cell::cell_entetes(pw, etat) {
                    cas.push((format!("feuille Trading ({etat})"), e, b));
                }
            }
            for k in [Kind::Trading, Kind::Journal, Kind::Indicators] {
                let (e, b) = Panels::entete(&t, k, pw);
                cas.push((format!("volet {k:?}"), e, b));
            }
            for (i, n) in noms.iter().enumerate() {
                let (e, ic) = crate::exchanges::titre_carte(&t, n, i, pw);
                for b in ic {
                    cas.push((format!("source {n}"), e.clone(), b));
                }
            }
            for m in crate::dock::Module::ALL {
                let r = Rect::new(0.0, 0.0, pw, 120.0);
                cas.push((
                    format!("module {m:?}"),
                    crate::dock::titre_module(&t, m.face().1, r),
                    crate::dock::croix_rect(r),
                ));
            }
            for (nom, e, b) in cas {
                assert!(!croise(e.titre_r, b), "{pw} px · {nom} : « {} » {:?} croise {b:?}", e.titre, e.titre_r);
                assert!(e.titre_r.x + e.titre_r.w <= b.x - ECART + 0.5, "{pw} px · {nom} : écart");
            }
        }
    }
}
