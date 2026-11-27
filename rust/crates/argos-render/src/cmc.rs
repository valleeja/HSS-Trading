//! 📊 Panneau Marché crypto (CMC d'Argos v1) : le jeton ouvert (prix, variations, cap., volume, FDV, ATH), peur &
//! avidité (jauge), marché crypto (TOTAL, TOTAL2, TOTAL3, dominances, volume) et saison des altcoins.
//! Les données viennent de `argos_data::market` (CoinGecko, alternative.me), remplies par la fenêtre.

use crate::shapes::{arc, lines, round_fill, Rect};
use crate::text::{Align, Rgb, Text};
use crate::{fill, Theme};
use argos_data::market::MarketInfo;
use tiny_skia::Pixmap;

const PAD: f64 = 16.0;
use crate::palette as pal;
use crate::palette::{GOLD, GREY, WHITE};

/// Hauteur dont le panneau a besoin (il défile en dessous).
pub const HEIGHT: f64 = 740.0;

/// « 2,68 T $ », « 49,65 B $ », « 812,3 M $ ».
pub fn big(v: f64) -> String {
    let p = |x: f64, s: &str| format!("{} {s}", crate::viewport::format_price(x, 2));
    match v.abs() {
        a if a >= 1e12 => p(v / 1e12, "T $"),
        a if a >= 1e9 => p(v / 1e9, "B $"),
        a if a >= 1e6 => p(v / 1e6, "M $"),
        _ => format!("{} $", crate::viewport::format_price(v, 0)),
    }
}

/// Prix lisible (plus de décimales pour les petits prix).
fn px(v: f64) -> String {
    let d = if v >= 1000.0 {
        2
    } else if v >= 1.0 {
        4
    } else {
        6
    };
    format!("{} $", crate::viewport::format_price(v, d))
}

fn pct(v: Option<f64>) -> (String, Rgb) {
    match v {
        Some(x) => (
            format!("{}{:.2} %", if x >= 0.0 { "+" } else { "" }, x),
            if x >= 0.0 { crate::palette::up() } else { crate::palette::down() },
        ),
        None => ("—".into(), GREY),
    }
}

/// Coupe `s` en lignes de largeur `max` au plus (aux espaces).
fn wrap(text: &Text, s: &str, size: f32, max: f64) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for w in s.split_whitespace() {
        match out.last_mut() {
            Some(l) if text.width(&format!("{l} {w}"), size) as f64 <= max => *l = format!("{l} {w}"),
            _ => out.push(w.to_string()),
        }
    }
    out
}

/// Texte gris sur plusieurs lignes ; avance `y`.
fn para(pm: &mut Pixmap, text: &Text, y: &mut f64, s: &str, size: f32, c: Rgb, pw: f64) {
    for l in wrap(text, s, size, pw - 2.0 * PAD) {
        text.draw(pm, &l, (PAD as f32, (*y + 8.0) as f32), size, c, Align::Left);
        *y += size as f64 + 6.0;
    }
    *y += 6.0;
}

/// Right end of the title « Marché crypto » (and a gap).
/// 📱 Taille d'un texte d'état (attente, erreur) : 13 au moins au doigt, pour le lire sur cell.
fn lisible(size: f32) -> f32 {
    if crate::actions::tactile() {
        size.max(13.0)
    } else {
        size
    }
}

fn titre_fin() -> f64 {
    PAD + Text::default().width("Marché crypto", 15.0) as f64 + 10.0
}

/// 🔑 Bouton « Clé API » de l'en-tête (ajouter une API payante, optionnel ; sans clé : sources gratuites). `droite` :
/// place gardée à droite de l'en-tête (le ✕ ⤢ ⬓ du volet ; 0 dans la colonne de modules). Sur la ligne du titre s'il y
/// a la place, sinon sous le titre : jamais par-dessus (Jee, 20 nov. : colonne étroite, on ne voyait que « M »).
pub fn key_rect(pw: f64, droite: f64) -> Rect {
    let x = pw - PAD - 92.0 - droite;
    if x >= titre_fin() {
        Rect::new(x, 10.0, 92.0, 28.0)
    } else {
        Rect::new(PAD, 38.0, 92.0, 28.0)
    }
}

/// Hauteur ajoutée sous le titre quand le bouton « Clé API » est sur sa propre ligne.
fn decale(pw: f64, droite: f64) -> f64 {
    if key_rect(pw, droite).y > 10.0 {
        34.0
    } else {
        0.0
    }
}

/// 🔑 Ligne discrète sous l'en-tête, sans clé CoinMarketCap : un toucher ouvre la boîte de clé.
pub fn cle_rect(pw: f64, droite: f64) -> Rect {
    Rect::new(PAD, 48.0 + decale(pw, droite), pw - 2.0 * PAD, 20.0)
}

/// Texte de la ligne de la clé.
pub const CLE_TXT: &str = "🔑 Ajoute ta clé CoinMarketCap pour plus";

/// 📈 Ce que la clé CoinMarketCap ajoute (vide sans clé ou sans relevé) : le marché selon CoinMarketCap (capitalisation
/// totale et sa variation 24 h, BTC.D, ETH.D, USDC.D, volume 24 h), sa peur & avidité et sa saison des altcoins.
pub fn blocs_cmc(m: &MarketInfo) -> (Vec<(String, String)>, Option<String>, Option<String>) {
    let Some(p) = m.cmc.as_ref().filter(|_| m.cmc_cle) else {
        return (vec![], None, None);
    };
    let d = |v: Option<f64>| v.map_or("—".into(), |x| format!("{x:.2} %"));
    let (chg, _) = pct(p.chg_24h);
    let marche = vec![
        ("Cap. totale".to_string(), format!("{} {chg}", big(p.total))),
        ("Dominance BTC".into(), d(Some(p.btc_d))),
        ("Dominance ETH".into(), d(Some(p.eth_d))),
        ("Dominance USDC".into(), d(p.usdc_d)),
        ("Volume 24 h".into(), p.volume.map_or("—".into(), big)),
    ];
    let fng = p.fng.as_ref().map(|(v, l)| format!("CoinMarketCap : {v} · {l}"));
    let alt = p.alt_season.map(|a| format!("CoinMarketCap : {a} / 100"));
    (marche, fng, alt)
}

/// Dessine le panneau (largeur `pw`) dans `pm` ; `droite` : place gardée à droite de l'en-tête (voir [`key_rect`]).
pub fn draw(m: &MarketInfo, pm: &mut Pixmap, text: &Text, pw: f64, droite: f64, th: &Theme) {
    let h = crate::px::lh(pm);
    fill(pm, 0.0, 0.0, pw, h, th.background, 1.0);
    // (rien ne dépasse la largeur : chaque texte est coupé avec « … » dans sa place)
    let large = (pw - 2.0 * PAD).max(20.0);
    text.draw(pm, &text.fit("Marché crypto", 15.0, large), (PAD as f32, 28.0), 15.0, WHITE, Align::Left);
    let k = key_rect(pw, droite);
    round_fill(pm, k, 8.0, pal::hover(), 1.0);
    text.draw(pm, "🔑 Clé API", (k.cx() as f32, (k.cy() + 5.0) as f32), 12.5, WHITE, Align::Center);
    let dy = decale(pw, droite);
    fill(pm, 0.0, 44.0 + dy, pw, 1.0, th.grid, 1.0);
    let mut y = 62.0 + dy;
    // 🔑 sans clé : une ligne discrète (la clé ne fait qu'ajouter, rien n'est retiré)
    if !m.cmc_cle {
        let r = cle_rect(pw, droite);
        let l = text.fit(CLE_TXT, 11.5, r.w);
        text.draw(pm, &l, (r.x as f32, (r.y + 14.0) as f32), 11.5, crate::jetons::ACCENT, Align::Left);
        y += 16.0;
    }
    let (cmc_marche, cmc_fng, cmc_alt) = blocs_cmc(m);
    let sec = |pm: &mut Pixmap, y: &mut f64, s: &str| {
        text.draw(pm, &text.fit(s, 11.0, large), (PAD as f32, (*y + 12.0) as f32), 11.0, GREY, Align::Left);
        *y += 22.0;
    };
    // grille de chiffres : 2 colonnes (le texte se coupe dans sa carte)
    let kv = |pm: &mut Pixmap, y: &mut f64, items: &[(String, String, Rgb)]| {
        let cw = (pw - 2.0 * PAD - 8.0) / 2.0;
        for (i, (l, v, c)) in items.iter().enumerate() {
            let r = Rect::new(PAD + (i % 2) as f64 * (cw + 8.0), *y + (i / 2) as f64 * 50.0, cw, 44.0);
            round_fill(pm, r, 8.0, pal::panel(), 1.0);
            let place = (r.w - 16.0).max(8.0);
            text.draw(
                pm,
                &text.fit(l, 10.5, place),
                ((r.x + 10.0) as f32, (r.y + 16.0) as f32),
                10.5,
                GREY,
                Align::Left,
            );
            // la valeur : plus petite si elle ne tient pas, puis coupée
            let t = if text.width(v, 13.5) as f64 <= place { 13.5 } else { 11.5 };
            text.draw(pm, &text.fit(v, t, place), ((r.x + 10.0) as f32, (r.y + 35.0) as f32), t, *c, Align::Left);
        }
        *y += items.len().div_ceil(2) as f64 * 50.0 + 8.0;
    };
    let nothing = m.coin.is_none() && m.fng.is_none() && m.global.is_none();
    if nothing {
        let s = m.error.clone().unwrap_or_else(|| "Chargement… (CoinGecko et alternative.me)".into());
        para(pm, text, &mut y, &s, 12.5, GREY, pw);
        return;
    }
    // le jeton ouvert
    if let Some(c) = &m.coin {
        let rang = c.rank.map(|r| format!("#{r}")).unwrap_or_default();
        let rw = if rang.is_empty() { 0.0 } else { text.width(&rang, 12.0) as f64 + 8.0 };
        let nom = text.fit(&c.name, 16.0, (large - rw - 40.0).max(20.0));
        text.draw(pm, &nom, (PAD as f32, (y + 14.0) as f32), 16.0, WHITE, Align::Left);
        let nx = PAD + text.width(&nom, 16.0) as f64 + 8.0;
        let sym = text.fit(&c.symbol, 12.0, (pw - PAD - rw - nx).max(0.0));
        text.draw(pm, &sym, (nx as f32, (y + 14.0) as f32), 12.0, GREY, Align::Left);
        if !rang.is_empty() {
            text.draw(pm, &rang, ((pw - PAD) as f32, (y + 14.0) as f32), 12.0, GREY, Align::Right);
        }
        y += 30.0;
        let (p24, col) = pct(c.chg[1]);
        // le prix (plus petit si la colonne est étroite), la variation 24 h à côté ou dessous
        let prix = px(c.price);
        let tp = if text.width(&prix, 22.0) as f64 <= large { 22.0 } else { 17.0 };
        text.draw(pm, &text.fit(&prix, tp, large), (PAD as f32, (y + 16.0) as f32), tp, WHITE, Align::Left);
        let vx = PAD + text.width(&prix, tp) as f64 + 10.0;
        if vx + text.width(&p24, 13.0) as f64 <= pw - PAD {
            text.draw(pm, &p24, (vx as f32, (y + 16.0) as f32), 13.0, col, Align::Left);
        } else {
            y += 22.0;
            text.draw(pm, &p24, (PAD as f32, (y + 16.0) as f32), 13.0, col, Align::Left);
        }
        y += 30.0;
        // 1 h, 7 j, 30 j, 1 an : ce qui ne tient pas passe à la ligne
        let mut x = PAD;
        for (k, name) in [(0, "1 h"), (2, "7 j"), (3, "30 j"), (4, "1 an")] {
            let (s, col) = pct(c.chg[k]);
            let w = text.width(name, 11.0) as f64 + 5.0 + text.width(&s, 11.0) as f64;
            if x > PAD && x + w > pw - PAD {
                (x, y) = (PAD, y + 18.0);
            }
            text.draw(pm, name, (x as f32, (y + 12.0) as f32), 11.0, GREY, Align::Left);
            let lx = x + text.width(name, 11.0) as f64 + 5.0;
            text.draw(pm, &s, (lx as f32, (y + 12.0) as f32), 11.0, col, Align::Left);
            x = lx + text.width(&s, 11.0) as f64 + 14.0;
        }
        y += 24.0;
        let range = match (c.low_24h, c.high_24h) {
            (Some(l), Some(hh)) => format!("{} – {}", px(l), px(hh)),
            _ => "—".into(),
        };
        let ath = match (c.ath, c.ath_chg) {
            (Some(a), Some(ch)) => format!("{} ({:.1} %)", px(a), ch),
            (Some(a), None) => px(a),
            _ => "—".into(),
        };
        let vc = if c.market_cap > 0.0 {
            format!("{:.2} %", c.volume / c.market_cap * 100.0)
        } else {
            "—".into()
        };
        kv(
            pm,
            &mut y,
            &[
                ("Cap. boursière".into(), big(c.market_cap), WHITE),
                ("Volume 24 h".into(), big(c.volume), WHITE),
                ("Vol / cap.".into(), vc, WHITE),
                ("FDV".into(), c.fdv.map_or("—".into(), big), WHITE),
                ("Bas / haut 24 h".into(), range, WHITE),
                ("ATH".into(), ath, WHITE),
            ],
        );
    } else {
        para(
            pm,
            text,
            &mut y,
            "Données du jeton : crypto seulement (pas pour l'or, le forex ou les indices)",
            11.5,
            GREY,
            pw,
        );
    }
    // peur & avidité
    sec(pm, &mut y, "PEUR & AVIDITÉ");
    match &m.fng {
        Some(f) => {
            let (cx, cy) = (PAD + 46.0, y + 44.0);
            // demi-cercle du rouge (peur) au vert (avidité), aiguille sur la valeur
            for k in 0..30 {
                let a0 = 270.0 + k as f32 * 6.0; // `arc` : 0° en haut, sens horaire
                let t = k as f32 / 29.0;
                let c = Rgb(
                    (0xf2 as f32 * (1.0 - t) + 0x08 as f32 * t) as u8,
                    (0x36 as f32 * (1.0 - t) + 0x99 as f32 * t) as u8,
                    (0x45 as f32 * (1.0 - t) + 0x81 as f32 * t) as u8,
                );
                lines(pm, cx, cy, &[&arc(38.0, a0, a0 + 6.5, 3)], c, 6.0);
            }
            let a = (180.0 + f.value as f32 / 100.0 * 180.0).to_radians();
            lines(pm, cx, cy, &[&[(0.0, 0.0), (32.0 * a.cos(), 32.0 * a.sin())]], WHITE, 2.0);
            text.draw(pm, &f.value.to_string(), (cx as f32, (cy + 18.0) as f32), 15.0, WHITE, Align::Center);
            let tx = PAD + 104.0;
            let place = (pw - PAD - tx).max(10.0);
            text.draw(pm, &text.fit(&f.label, 15.0, place), (tx as f32, (y + 30.0) as f32), 15.0, WHITE, Align::Left);
            let hier = format!("Hier : {}", f.yesterday.map_or("—".into(), |v| v.to_string()));
            let semaine = format!("Il y a 7 j : {}", f.week_ago.map_or("—".into(), |v| v.to_string()));
            let hist = format!("{hier} · {semaine}");
            if text.width(&hist, 11.5) as f64 <= place {
                text.draw(pm, &hist, (tx as f32, (y + 52.0) as f32), 11.5, GREY, Align::Left);
            } else {
                // colonne étroite : sur deux lignes
                text.draw(pm, &text.fit(&hier, 11.5, place), (tx as f32, (y + 50.0) as f32), 11.5, GREY, Align::Left);
                text.draw(pm, &text.fit(&semaine, 11.5, place), (tx as f32, (y + 66.0) as f32), 11.5, GREY, Align::Left);
            }
            y += 90.0;
            // 📈 avec la clé : celle de CoinMarketCap à côté, chaque source nommée
            if let Some(s) = &cmc_fng {
                text.draw(
                    pm,
                    &text.fit("alternative.me (ci-dessus)", 10.5, large),
                    (PAD as f32, (y + 2.0) as f32),
                    10.5,
                    GREY,
                    Align::Left,
                );
                text.draw(pm, &text.fit(s, 12.5, large), (PAD as f32, (y + 20.0) as f32), 12.5, WHITE, Align::Left);
                y += 30.0;
            }
        }
        None => {
            text.draw(pm, "Indisponible", (PAD as f32, (y + 8.0) as f32), 12.0, GREY, Align::Left);
            y += 22.0;
        }
    }
    // marché crypto (avec la clé : la source de chaque bloc est nommée)
    sec(pm, &mut y, if m.cmc_cle { "MARCHÉ CRYPTO · CoinGecko" } else { "MARCHÉ CRYPTO" });
    if let Some(g) = &m.global {
        let d = |v: Option<f64>| v.map_or("—".into(), |x| format!("{x:.2} %"));
        let (chg, col) = pct(g.total_chg_24h);
        kv(
            pm,
            &mut y,
            &[
                (
                    "Cap. totale (TOTAL)".into(),
                    format!("{} {chg}", big(g.total)),
                    if g.total_chg_24h.is_some() { col } else { WHITE },
                ),
                ("TOTAL2 (sans BTC)".into(), g.total2().map_or("—".into(), big), WHITE),
                ("TOTAL3 (sans BTC/ETH)".into(), g.total3().map_or("—".into(), big), WHITE),
                ("Dominance BTC".into(), d(g.btc_d), WHITE),
                ("Dominance ETH".into(), d(g.eth_d), WHITE),
                ("Dominance USDT".into(), d(g.usdt_d), WHITE),
                ("Volume 24 h total".into(), big(g.volume), WHITE),
            ],
        );
    }
    if !cmc_marche.is_empty() {
        sec(pm, &mut y, "MARCHÉ CRYPTO · CoinMarketCap");
        let items: Vec<(String, String, Rgb)> = cmc_marche.into_iter().map(|(l, v)| (l, v, WHITE)).collect();
        kv(pm, &mut y, &items);
    } else if m.cmc_cle {
        para(pm, text, &mut y, "CoinMarketCap : relevé en attente (un par jour)", 11.0, GREY, pw);
    }
    // saison des altcoins
    sec(pm, &mut y, "SAISON DES ALTCOINS (TOP 50 VS BTC SUR 30 J)");
    match m.alt_season {
        Some(a) => {
            let r = Rect::new(PAD, y + 4.0, pw - 2.0 * PAD, 8.0);
            for k in 0..60 {
                let t = k as f64 / 59.0;
                let c = if t < 0.5 {
                    let u = t * 2.0;
                    Rgb(
                        (0xf7 as f64 * (1.0 - u) + 0x6b as f64 * u) as u8,
                        (0x93 as f64 * (1.0 - u) + 0x6b as f64 * u) as u8,
                        (0x1a as f64 * (1.0 - u) + 0x73 as f64 * u) as u8,
                    )
                } else {
                    let u = (t - 0.5) * 2.0;
                    Rgb(
                        (0x6b as f64 * (1.0 - u) + 0x29 as f64 * u) as u8,
                        (0x6b as f64 * (1.0 - u) + 0x62 as f64 * u) as u8,
                        (0x73 as f64 * (1.0 - u) + 0xff as f64 * u) as u8,
                    )
                };
                fill(pm, r.x + t * (r.w - r.w / 60.0), r.y, r.w / 60.0 + 1.0, r.h, c, 1.0);
            }
            let mx = r.x + r.w * a as f64 / 100.0;
            round_fill(pm, Rect::new(mx - 2.0, r.y - 3.0, 4.0, 14.0), 2.0, WHITE, 1.0);
            let what = if a >= 75 {
                "Saison des altcoins"
            } else if a <= 25 {
                "Saison du Bitcoin"
            } else {
                "Neutre"
            };
            text.draw(pm, "Bitcoin", (PAD as f32, (y + 30.0) as f32), 10.5, GREY, Align::Left);
            text.draw(pm, "Altcoins", ((pw - PAD) as f32, (y + 30.0) as f32), 10.5, GREY, Align::Right);
            // la valeur au milieu, entre « Bitcoin » et « Altcoins » ; colonne étroite : sur sa ligne, dessous
            let valeur = format!("{a} / 100 · {what}");
            let bords = text.width("Bitcoin", 10.5).max(text.width("Altcoins", 10.5)) as f64 + 8.0;
            if text.width(&valeur, 11.5) as f64 <= pw - 2.0 * PAD - 2.0 * bords {
                text.draw(pm, &valeur, ((pw / 2.0) as f32, (y + 30.0) as f32), 11.5, WHITE, Align::Center);
            } else {
                y += 18.0;
                text.draw(pm, &text.fit(&valeur, 11.5, large), (PAD as f32, (y + 30.0) as f32), 11.5, WHITE, Align::Left);
            }
            y += 44.0;
            if let Some(s) = &cmc_alt {
                text.draw(pm, &text.fit(s, 12.0, large), (PAD as f32, (y + 4.0) as f32), 12.0, WHITE, Align::Left);
                y += 22.0;
            }
        }
        None => {
            text.draw(pm, "Calcul en cours…", (PAD as f32, (y + 8.0) as f32), lisible(12.0), GREY, Align::Left);
            y += 22.0;
        }
    }
    if let Some(e) = &m.error {
        para(pm, text, &mut y, &format!("⚠ {e}"), lisible(11.0), GOLD, pw);
    }
    let source = if m.cmc_cle {
        "Source : CoinGecko et alternative.me · mise à jour aux 60 s ; CoinMarketCap avec ta clé (un relevé par jour)"
    } else {
        "Source : CoinGecko et alternative.me · mise à jour aux 60 s"
    };
    para(pm, text, &mut y, source, 10.5, GREY, pw);
}

#[cfg(test)]
mod tests {
    use super::*;
    use argos_data::market::{Coin, Fng, Global};

    /// 📱 Au doigt, les textes d'état (« Calcul en cours… », ⚠ erreur) font 13 au moins ; le PC garde ses tailles.
    #[test]
    fn au_doigt_textes_d_etat_lisibles() {
        crate::actions::set_tactile(true);
        let doigt = (lisible(12.0), lisible(11.0), lisible(15.0));
        crate::actions::set_tactile(false);
        assert_eq!(doigt, (13.0, 13.0, 15.0));
        assert_eq!((lisible(12.0), lisible(11.0)), (12.0, 11.0));
    }

    #[test]
    fn panneau() {
        assert_eq!(big(2.6778e12), format!("{} T $", crate::viewport::format_price(2.68, 2)));
        let m = MarketInfo {
            coin: Some(Coin {
                name: "Bitcoin".into(),
                symbol: "BTC".into(),
                rank: Some(1),
                price: 80096.0,
                chg: [Some(0.6), Some(-0.85), None, Some(-4.7), Some(-28.1)],
                market_cap: 1.61e12,
                volume: 1.8e10,
                ..Default::default()
            }),
            fng: Some(Fng {
                value: 50,
                label: "Neutre".into(),
                yesterday: Some(54),
                week_ago: Some(31),
            }),
            global: Some(Global {
                total: 2.68e12,
                total_chg_24h: Some(-4.5),
                btc_d: Some(60.0),
                eth_d: Some(10.6),
                usdt_d: Some(6.8),
                usdc_d: None,
                volume: 4.96e10,
            }),
            alt_season: Some(25),
            error: None,
            ..Default::default()
        };
        let mut pm = crate::px::new(340, 900).unwrap();
        draw(&m, &mut pm, &Text::default(), 340.0, 0.0, &Theme::default());
        if let Ok(p) = std::env::var("ARGOS_CMC_PNG") {
            pm.save_png(p).unwrap();
        }
        draw(&MarketInfo::default(), &mut pm, &Text::default(), 340.0, 0.0, &Theme::default());
    }

    /// 🔑 La clé CoinMarketCap ne fait qu'ajouter : sans clé, le panneau est le même au pixel près (même si un relevé
    /// traîne) ; avec la clé, les 4 blocs (marché selon CMC avec USDC.D, peur & avidité de CMC, saison des altcoins de CMC)
    /// s'ajoutent, et les valeurs gratuites restent.
    #[test]
    fn la_cle_cmc_ajoute_sans_rien_retirer() {
        use argos_data::cmcap::CapPoint;
        use argos_data::market::{Fng, Global};
        let base = MarketInfo {
            fng: Some(Fng {
                value: 40,
                label: "Peur".into(),
                ..Default::default()
            }),
            global: Some(Global {
                total: 3.1e12,
                btc_d: Some(58.0),
                eth_d: Some(11.0),
                ..Default::default()
            }),
            alt_season: Some(30),
            ..Default::default()
        };
        let p = CapPoint {
            t: 1,
            total: 3.2e12,
            btc_d: 59.0,
            eth_d: 10.5,
            usdc_d: Some(2.1),
            volume: Some(9e10),
            chg_24h: Some(1.5),
            fng: Some((45, "Neutre".into())),
            alt_season: Some(33),
            ..Default::default()
        };
        let image = |m: &MarketInfo| {
            let mut pm = crate::px::new(360, 1100).unwrap();
            draw(m, &mut pm, &Text::default(), 360.0, 0.0, &Theme::default());
            pm
        };
        let traine = MarketInfo {
            cmc: Some(p.clone()),
            ..base.clone()
        };
        assert_eq!(blocs_cmc(&traine), (vec![], None, None), "sans clé : rien de CoinMarketCap");
        assert!(image(&traine) == image(&base), "sans clé : le même panneau");
        let cle = MarketInfo {
            cmc_cle: true,
            ..traine.clone()
        };
        let (marche, fng, alt) = blocs_cmc(&cle);
        let noms: Vec<&str> = marche.iter().map(|(l, _)| l.as_str()).collect();
        assert_eq!(noms, ["Cap. totale", "Dominance BTC", "Dominance ETH", "Dominance USDC", "Volume 24 h"]);
        assert_eq!(marche[3].1, "2.10 %");
        assert_eq!(fng.as_deref(), Some("CoinMarketCap : 45 · Neutre"));
        assert_eq!(alt.as_deref(), Some("CoinMarketCap : 33 / 100"));
        assert!(image(&cle) != image(&base), "avec la clé : des blocs en plus");
        assert_eq!(cle.global, base.global, "les valeurs gratuites restent");
        // 📸 ARGOS_CMC_CLE_PNG=dossier : le panneau sans clé et avec la clé, pour le regarder
        if let Ok(dir) = std::env::var("ARGOS_CMC_CLE_PNG") {
            image(&base).save_png(format!("{dir}/marche-sans-cle.png")).unwrap();
            image(&cle).save_png(format!("{dir}/marche-avec-cle.png")).unwrap();
        }
    }

    /// 🖥 Jee (20 Nov, module « Marché crypto » of the module column): in a narrow column « 🔑 Clé API » was drawn on
    /// the title (only « M » showed) and the cards ran past the right edge. At every width (220 to 420), with or without
    /// the panel's ✕ ⤢ ⬓ place, the button never covers the title and nothing is drawn past the width.
    #[test]
    fn colonne_etroite_rien_ne_deborde() {
        use argos_data::cmcap::CapPoint;
        let m = MarketInfo {
            coin: Some(Coin {
                name: "Bitcoin".into(),
                symbol: "BTC".into(),
                rank: Some(1),
                price: 61_356.25,
                chg: [Some(0.61), Some(-0.85), Some(-12.4), Some(-4.7), Some(-28.1)],
                market_cap: 1.61e12,
                volume: 1.8e10,
                fdv: Some(1.7e12),
                low_24h: Some(60_123.45),
                high_24h: Some(62_834.12),
                ath: Some(126_080.0),
                ath_chg: Some(-51.3),
                ..Default::default()
            }),
            fng: Some(Fng {
                value: 50,
                label: "Peur extrême".into(),
                yesterday: Some(54),
                week_ago: Some(31),
            }),
            global: Some(Global {
                total: 2.68e12,
                total_chg_24h: Some(-4.5),
                btc_d: Some(60.0),
                eth_d: Some(10.6),
                usdt_d: Some(6.8),
                usdc_d: None,
                volume: 4.96e10,
            }),
            alt_season: Some(25),
            ..Default::default()
        };
        let avec_cle = MarketInfo {
            cmc_cle: true,
            cmc: Some(CapPoint {
                t: 1,
                total: 3.2e12,
                btc_d: 59.0,
                eth_d: 10.5,
                usdc_d: Some(2.1),
                volume: Some(9e10),
                chg_24h: Some(1.5),
                fng: Some((45, "Neutre".into())),
                alt_season: Some(33),
                ..Default::default()
            }),
            ..m.clone()
        };
        let t = Text::default();
        let titre = Rect::new(PAD, 10.0, t.width("Marché crypto", 15.0) as f64, 26.0);
        for pw in (220..=420).step_by(20).map(f64::from) {
            for droite in [0.0, crate::watchlist::FERME_PLACE] {
                let k = key_rect(pw, droite);
                let croise = k.x < titre.x + titre.w && titre.x < k.x + k.w && k.y < titre.y + titre.h && titre.y < k.y + k.h;
                assert!(!croise, "{pw} / {droite}: button {k:?} on the title");
                assert!(k.x >= PAD && k.x + k.w <= pw - PAD + 0.01, "{pw} / {droite}: button in the width");
                let l = cle_rect(pw, droite);
                assert!(l.y >= k.y + k.h, "{pw} / {droite}: the key line under the button");
                // nothing drawn past the width: the image is 60 points wider, that strip stays empty
                for info in [&m, &avec_cle] {
                    let mut pm = crate::px::new(pw as u32 + 60, 1200).unwrap();
                    draw(info, &mut pm, &t, pw, droite, &Theme::default());
                    let (iw, w) = (pm.width() as usize, pw.ceil() as usize + 1);
                    let deborde = pm.pixels().chunks(iw).any(|ligne| ligne[w..].iter().any(|p| p.alpha() > 0));
                    assert!(!deborde, "{pw} / {droite} / key {}: something drawn past the width", info.cmc_cle);
                }
            }
        }
        // a wide panel keeps the button on the title's line
        assert_eq!(key_rect(420.0, 0.0).y, 10.0);
        assert_eq!(key_rect(240.0, 0.0).y, 10.0, "even a 240 column has room when nothing is kept at the right");
    }
}
