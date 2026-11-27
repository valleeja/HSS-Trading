//! 🐋 Panneau Baleines : les grosses transactions en direct (Hyperliquid et Binance Futures), la plus récente en
//! haut ; seuil (100 k$ à 1 M$), toutes les pièces ou celle du graphique, pression achat / vente des 15 dernières
//! minutes ; toucher une transaction Hyperliquid = ouvrir le portefeuille de la baleine dans l'explorateur.
//! 🔵 Bulles sur la chart (au prix et au moment de chaque transaction, taille selon le montant) : le flux continue
//! alors même panneau fermé.
//! 👁 Suivre une baleine : toucher une transaction Hyperliquid montre le compte et les positions ouvertes de ce
//! portefeuille (adresse publique seulement), rafraîchis par la fenêtre ; ses transactions sont marquées 👁.
//!
//! Une seule table de zones (`zones`) sert au dessin et au clic.

use crate::palette as pal;
use crate::palette::{accent, GOLD, GREY, WHITE};
use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Text};
use crate::{fill, Theme};
use argos_data::whales::{Account, Whale};
use tiny_skia::Pixmap;

/// Seuils (libellé, dollars).
pub const LEVELS: [(&str, f64); 4] = [("100 k$", 100e3), ("250 k$", 250e3), ("500 k$", 500e3), ("1 M$", 1e6)];
/// Transactions gardées en mémoire.
const KEEP: usize = 20_000;
/// 📱 Bulles au plus sur la chart du cell : les plus grosses.
pub const BULLES_CELL: usize = 8;
/// ⏱ Périodes du résumé achats / ventes et de la liste : (bouton, minutes). 15 min par défaut.
pub const PERIODS: [(&str, u32); 7] = [("5m", 5), ("15m", 15), ("1h", 60), ("4h", 240), ("12h", 720), ("24h", 1440), ("7j", 10_080)];
/// Fenêtre de la pression achat / vente (ms).
const WINDOW_MS: i64 = 15 * 60 * 1000;

const PAD: f64 = 14.0;
const HEAD: f64 = 48.0;
const CHIP_H: f64 = 28.0;
const PRESSURE: f64 = 130.0;
const ROW: f64 = 52.0;
const POS_ROW: f64 = 44.0;

/// Données et filtres.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Whales {
    /// La plus récente d'abord.
    pub list: Vec<Whale>,
    /// Index dans `LEVELS`.
    pub level: usize,
    /// Seulement la pièce du graphique.
    pub only: bool,
    /// 🔵 Bulles sur la chart.
    pub bubbles: bool,
    /// Pièce du graphique (BTC, ETH…).
    pub coin: String,
    /// Message (connexion, erreur).
    pub note: String,
    /// 👁 Portefeuille suivi (adresse publique), son compte (`None` : en chargement) et son message d'erreur.
    pub follow: Option<String>,
    pub account: Option<Account>,
    pub follow_note: String,
    /// ⏱ Période choisie, en minutes (0 : 15 min, la valeur par défaut).
    pub period_min: u32,
    /// Heure de la dernière image (ms) : la période se compte depuis là.
    pub now_ms: i64,
    /// Décalage du fuseau (secondes) : l'heure « données depuis HH:MM ».
    pub tz_offset_s: i64,
    /// 📊 Flux de tous les ordres (bougies Binance Futures, taker buy) de `flow_coin` : 7 j en 1 h, 24 h en 5 min.
    pub flow_h: Vec<argos_data::whales::Flow>,
    pub flow_m: Vec<argos_data::whales::Flow>,
    pub flow_coin: String,
    /// 🏛️ Filtre : 0 tout, 1 crypto, 2 tradfi (marchés HIP-3 de Hyperliquid : or, actions, indices…).
    pub kind: u8,
    /// 🏦 Exchanges écoutés (noms de `argos_data::whales::EXCHANGES`) ; vide : les 5 plus gros (par défaut).
    pub exchanges: Vec<String>,
}

/// Résultat d'un clic.
#[derive(Debug, Clone, PartialEq)]
pub enum WhaleClick {
    None,
    Changed,
    /// Ouvrir le portefeuille dans l'explorateur.
    Open(String),
    /// 👁 Suivre ce portefeuille (la fenêtre charge ses positions).
    Follow(String),
}

#[derive(Debug, Clone, PartialEq)]
enum Zone {
    Period(u32),
    /// 🏛️ Tout / Crypto / Tradfi.
    Kind(u8),
    /// 🏦 « Tous » (`None`) ou un exchange (index dans `EXCHANGES`).
    Exchange(Option<usize>),
    Level(usize),
    Only(bool),
    Bubbles,
    Explorer,
    Unfollow,
    Row(usize),
}

/// 🏛️ Boutons du filtre crypto / tradfi.
const KINDS: [&str; 3] = ["Tout", "Crypto", "Tradfi"];

/// 🏛️ Marché tradfi : les dex HIP-3 de Hyperliquid écrivent « dex:NOM » (xyz:GOLD, xyz:NVDA…).
pub fn is_tradfi(coin: &str) -> bool {
    coin.contains(':')
}

/// Nom affiché : « GOLD » pour « xyz:GOLD ».
fn short_coin(coin: &str) -> &str {
    coin.rsplit(':').next().unwrap_or(coin)
}

/// 🏦 Rangées de boutons d'exchanges (6 par rangée, avec « 5 gros »).
fn ex_rows() -> f64 {
    (argos_data::whales::EXCHANGES.len() + 1).div_ceil(6) as f64
}

/// Montant court : « 1,25 M$ », « 250 k$ », « 800 $ ».
pub fn money(v: f64) -> String {
    let (sign, a) = if v < 0.0 { ("−", -v) } else { ("", v) };
    if a >= 1e6 {
        format!("{sign}{:.2} M$", a / 1e6).replace('.', ",")
    } else if a >= 1e3 {
        format!("{sign}{:.0} k$", a / 1e3)
    } else {
        format!("{sign}{a:.0} $")
    }
}

/// Adresse courte : « 0x7a3f…c4d1 ».
fn short(a: &str) -> String {
    if a.len() > 12 {
        format!("{}…{}", &a[..6], &a[a.len() - 4..])
    } else {
        a.to_string()
    }
}

/// Prix avec assez de décimales.
fn price(p: f64) -> String {
    let d = if p >= 1000.0 {
        1
    } else if p >= 1.0 {
        3
    } else {
        5
    };
    format!("{p:.d$}")
}

impl Whales {
    /// Ajoute des transactions (les plus récentes en haut), en gardant `KEEP` au plus et 7 jours au plus.
    pub fn push(&mut self, mut new: Vec<Whale>) {
        new.sort_by_key(|w| -w.time_ms);
        new.append(&mut self.list);
        // 🔁 jamais deux fois la même transaction (reconnexion, instantané renvoyé…) : même exchange, même moment, même
        // marché, même prix, même taille
        let mut seen = std::collections::HashSet::new();
        new.retain(|w| seen.insert((w.src, w.time_ms, w.coin.clone(), w.price.to_bits(), w.size.to_bits())));
        let newest = new.first().map_or(0, |w| w.time_ms);
        self.now_ms = self.now_ms.max(newest);
        let oldest = self.now_ms - 7 * 86_400_000;
        new.retain(|w| w.time_ms >= oldest);
        new.truncate(KEEP);
        self.list = new;
    }

    /// 🏦 Exchanges écoutés (les 5 plus gros si aucun choix).
    pub fn exchanges_on(&self) -> Vec<&'static str> {
        use argos_data::whales::{DEFAULT_ON, EXCHANGES};
        if self.exchanges.is_empty() {
            return EXCHANGES[..DEFAULT_ON].to_vec();
        }
        EXCHANGES.iter().copied().filter(|e| self.exchanges.iter().any(|x| x == e)).collect()
    }

    /// ⏱ Période choisie (ms).
    pub fn window_ms(&self) -> i64 {
        let m = if self.period_min == 0 { WINDOW_MS / 60_000 } else { self.period_min as i64 };
        m * 60_000
    }

    /// Nom court de la période (« 15m », « 4h », « 7j »).
    pub fn period_label(&self) -> &'static str {
        let m = (self.window_ms() / 60_000) as u32;
        PERIODS.iter().find(|(_, x)| *x == m).map_or("15m", |(l, _)| l)
    }

    /// Transactions gardées par les filtres et la période.
    fn shown(&self) -> Vec<&Whale> {
        let min = LEVELS[self.level.min(LEVELS.len() - 1)].1;
        let w = self.window_ms();
        self.list
            .iter()
            .filter(|x| x.usd() >= min && (!self.only || x.coin == self.coin) && (self.now_ms == 0 || self.now_ms - x.time_ms <= w))
            .filter(|x| self.exchanges_on().contains(&x.src))
            .filter(|x| match self.kind {
                1 => !is_tradfi(&x.coin),
                2 => is_tradfi(&x.coin),
                _ => true,
            })
            .collect()
    }

    /// Achats et ventes (dollars) sur la période choisie, avec les filtres.
    pub fn pressure(&self, now_ms: i64) -> (f64, f64) {
        let w = self.window_ms();
        self.shown()
            .iter()
            .filter(|x| now_ms - x.time_ms <= w)
            .fold((0.0, 0.0), |(b, s), x| if x.buy { (b + x.usd(), s) } else { (b, s + x.usd()) })
    }

    /// Durée réellement couverte (ms) : depuis la plus vieille transaction gardée (le flux commence à l'ouverture).
    pub fn covered_ms(&self, now_ms: i64) -> i64 {
        self.list.last().map_or(0, |w| (now_ms - w.time_ms).max(0))
    }

    /// 📊 Achats / ventes de TOUS les ordres (bougies Binance) sur la période choisie, tout de suite.
    pub fn flow_sum(&self, now_ms: i64) -> Option<(f64, f64)> {
        let w = self.window_ms();
        let (bars, len) = if w <= 86_400_000 && !self.flow_m.is_empty() {
            (&self.flow_m, 300_000.0)
        } else {
            (&self.flow_h, 3_600_000.0)
        };
        let since = now_ms - w;
        // une bougie à cheval sur le début de la période compte pour la part qui est dedans
        let part = |f: &argos_data::whales::Flow| (((f.t_ms as f64 + len) - since as f64) / len).clamp(0.0, 1.0);
        let v: Vec<_> = bars.iter().map(|f| (f, part(f))).filter(|(_, k)| *k > 0.0).collect();
        (!v.is_empty()).then(|| v.iter().fold((0.0, 0.0), |(b, s), (f, k)| (b + f.buy_usd * k, s + f.sell_usd * k)))
    }

    /// 📊 Moyenne par heure sur 7 jours (achats, ventes), et le rythme de la période choisie comparé à elle.
    pub fn flow_avg(&self, now_ms: i64) -> Option<((f64, f64), f64)> {
        let n = self.flow_h.len();
        if n < 2 {
            return None;
        }
        let (b, s) = self.flow_h.iter().fold((0.0, 0.0), |(b, s), f| (b + f.buy_usd, s + f.sell_usd));
        let avg = (b / n as f64, s / n as f64);
        let (pb, ps) = self.flow_sum(now_ms)?;
        let hours = (self.window_ms() as f64 / 3_600_000.0).max(1.0 / 12.0);
        let per_h = (pb + ps) / hours;
        let ratio = per_h / (avg.0 + avg.1).max(1.0);
        Some((avg, ratio))
    }

    /// 🐋 Grosses transactions de la période (filtres compris) : nombre et taille moyenne.
    pub fn whale_stats(&self) -> (usize, f64) {
        let v = self.shown();
        let n = v.len();
        (n, if n == 0 { 0.0 } else { v.iter().map(|w| w.usd()).sum::<f64>() / n as f64 })
    }

    /// ⏳ La période de `minutes` dépasse-t-elle ce qui a été reçu ?
    pub fn beyond(&self, minutes: u32, now_ms: i64) -> bool {
        self.covered_ms(now_ms) + 60_000 < minutes as i64 * 60_000
    }

    /// ⏳ « Données reçues depuis 19:12 (41 min) » quand la période choisie dépasse ce qui a été reçu, sinon rien.
    pub fn coverage_note(&self, now_ms: i64) -> Option<String> {
        let first = self.list.last()?.time_ms;
        if !self.beyond((self.window_ms() / 60_000) as u32, now_ms) {
            return None;
        }
        let m = ((now_ms - first) / 60_000).max(0);
        let d = if m >= 1440 {
            format!("{} j {} h", m / 1440, m % 1440 / 60)
        } else if m >= 60 {
            format!("{} h {:02}", m / 60, m % 60)
        } else {
            format!("{m} min")
        };
        let s = (first / 1000 + self.tz_offset_s).rem_euclid(86_400);
        Some(format!("⏳ Données reçues depuis {:02}:{:02} ({d})", s / 3600, s % 3600 / 60))
    }

    /// 🔵 Transactions à mettre en bulles sur la chart : celles de la pièce du graphique, avec les mêmes filtres que
    /// les totaux du panneau (seuil, période, exchanges, Tout / Crypto / Tradfi).
    pub fn on_chart(&self, phone: bool) -> Vec<Whale> {
        if !self.bubbles {
            return vec![];
        }
        let mut v: Vec<Whale> = self.shown().into_iter().filter(|w| w.coin == self.coin).cloned().collect();
        // 📱 sur cell : seulement les plus grosses (un paquet de bulles cachait les bougies et ralentissait chaque image)
        if phone && v.len() > BULLES_CELL {
            v.sort_by(|a, b| b.usd().total_cmp(&a.usd()));
            v.truncate(BULLES_CELL);
        }
        v
    }

    /// Haut du bloc 👁 (sous la pression).
    fn follow_y(&self) -> f64 {
        HEAD + (4.0 + ex_rows()) * (CHIP_H + 8.0) + 8.0 + PRESSURE
    }

    /// Hauteur du bloc 👁 (0 : personne de suivi).
    fn follow_h(&self) -> f64 {
        match &self.follow {
            None => 0.0,
            Some(_) => 30.0 + CHIP_H + 10.0 + self.account.as_ref().map_or(1, |a| a.positions.len().max(1)) as f64 * POS_ROW + 12.0,
        }
    }

    fn top(&self) -> f64 {
        self.follow_y() + self.follow_h()
    }

    /// Hauteur du contenu (le panneau défile).
    pub fn content_h(&self) -> f64 {
        self.top() + self.shown().len().max(1) as f64 * ROW + 40.0
    }

    fn zones(&self, text: &Text, pw: f64) -> Vec<(Rect, Zone)> {
        let mut z = vec![];
        let mut x = PAD;
        for (k, (l, _)) in LEVELS.iter().enumerate() {
            let w = text.width(l, 12.0) as f64 + 20.0;
            z.push((Rect::new(x, HEAD + 4.0, w, CHIP_H), Zone::Level(k)));
            x += w + 6.0;
        }
        let mut x = PAD;
        for only in [false, true] {
            let w = text.width(&self.only_label(only), 12.0) as f64 + 20.0;
            z.push((Rect::new(x, HEAD + CHIP_H + 12.0, w, CHIP_H), Zone::Only(only)));
            x += w + 6.0;
        }
        let w = text.width(BUBBLES, 12.0) as f64 + 20.0;
        z.push((Rect::new(PAD, HEAD + 2.0 * CHIP_H + 20.0, w, CHIP_H), Zone::Bubbles));
        // 🏛️ Tout / Crypto / Tradfi, à droite des bulles
        let mut x = PAD + w + 6.0;
        for (k, l) in KINDS.iter().enumerate() {
            let cw = text.width(l, 12.0) as f64 + 16.0;
            z.push((Rect::new(x, HEAD + 2.0 * CHIP_H + 20.0, cw, CHIP_H), Zone::Kind(k as u8)));
            x += cw + 4.0;
        }
        // ⏱ périodes (rangée qui tient dans la largeur : boutons étroits)
        let n = PERIODS.len() as f64;
        let pw_chip = ((pw - 2.0 * PAD - (n - 1.0) * 4.0) / n).max(30.0);
        for (k, (_, m)) in PERIODS.iter().enumerate() {
            z.push((
                Rect::new(PAD + k as f64 * (pw_chip + 4.0), HEAD + 3.0 * CHIP_H + 28.0, pw_chip, CHIP_H),
                Zone::Period(*m),
            ));
        }
        // 🏦 exchanges, en rangées de 6 : « 5 gros » (le choix par défaut), puis chacun (toucher = allumer / éteindre)
        let ex = argos_data::whales::EXCHANGES;
        let per = 6usize;
        let cw = ((pw - 2.0 * PAD - (per as f64 - 1.0) * 4.0) / per as f64).max(30.0);
        for k in 0..=ex.len() {
            let (row, col) = (k / per, k % per);
            let r = Rect::new(PAD + col as f64 * (cw + 4.0), HEAD + (4 + row) as f64 * (CHIP_H + 8.0) + 4.0, cw, CHIP_H);
            z.push((r, Zone::Exchange(k.checked_sub(1))));
        }
        if self.follow.is_some() {
            let y = self.follow_y() + 30.0;
            let w1 = text.width(EXPLORER, 12.0) as f64 + 20.0;
            z.push((Rect::new(PAD, y, w1, CHIP_H), Zone::Explorer));
            z.push((Rect::new(PAD + w1 + 6.0, y, text.width(UNFOLLOW, 12.0) as f64 + 20.0, CHIP_H), Zone::Unfollow));
        }
        for k in 0..self.shown().len() {
            z.push((Rect::new(0.0, self.top() + k as f64 * ROW, pw, ROW), Zone::Row(k)));
        }
        z
    }

    fn only_label(&self, only: bool) -> String {
        if only {
            format!("{} seulement", if self.coin.is_empty() { "Graphique" } else { &self.coin })
        } else {
            "Toutes les pièces".into()
        }
    }

    /// Clic (coordonnées du contenu, défilement déjà compté).
    pub fn click(&mut self, text: &Text, pw: f64, x: f64, y: f64) -> WhaleClick {
        match self.zones(text, pw).into_iter().find(|(r, _)| r.contains(x, y)).map(|(_, z)| z) {
            Some(Zone::Level(k)) => {
                self.level = k;
                WhaleClick::Changed
            }
            Some(Zone::Period(m)) => {
                self.period_min = m;
                WhaleClick::Changed
            }
            Some(Zone::Kind(k)) => {
                self.kind = k;
                WhaleClick::Changed
            }
            Some(Zone::Exchange(None)) => {
                self.exchanges.clear();
                WhaleClick::Changed
            }
            Some(Zone::Exchange(Some(k))) => {
                use argos_data::whales::{DEFAULT_ON, EXCHANGES};
                let e = EXCHANGES[k].to_string();
                let mut on: Vec<String> = self.exchanges_on().iter().map(|s| s.to_string()).collect();
                if on.contains(&e) {
                    on.retain(|x| *x != e);
                } else {
                    on.push(e);
                }
                // aucun coché, ou exactement les 5 gros : le choix par défaut
                let default: Vec<String> = EXCHANGES[..DEFAULT_ON].iter().map(|s| s.to_string()).collect();
                on.sort_by_key(|x| EXCHANGES.iter().position(|e| e == x));
                self.exchanges = if on.is_empty() || on == default { vec![] } else { on };
                WhaleClick::Changed
            }
            Some(Zone::Only(o)) => {
                self.only = o;
                WhaleClick::Changed
            }
            Some(Zone::Bubbles) => {
                self.bubbles = !self.bubbles;
                WhaleClick::Changed
            }
            Some(Zone::Explorer) => match &self.follow {
                Some(a) => WhaleClick::Open(format!("https://app.hyperliquid.xyz/explorer/address/{a}")),
                None => WhaleClick::None,
            },
            Some(Zone::Unfollow) => {
                (self.follow, self.account) = (None, None);
                self.follow_note.clear();
                WhaleClick::Changed
            }
            Some(Zone::Row(k)) => match self.shown().get(k) {
                Some(w) if argos_data::whales::is_address(&w.wallet) => {
                    let a = w.wallet.clone();
                    (self.follow, self.account) = (Some(a.clone()), None);
                    self.follow_note.clear();
                    WhaleClick::Follow(a)
                }
                _ => WhaleClick::None,
            },
            None => WhaleClick::None,
        }
    }

    /// Dessine le panneau (largeur `pw`) dans `pm` (sa hauteur = le contenu) ; `now_ms` : l'heure (ms).
    pub fn draw(&self, pm: &mut Pixmap, text: &Text, pw: f64, now_ms: i64, mouse: Option<(f64, f64)>, th: &Theme) {
        let h = crate::px::lh(pm);
        fill(pm, 0.0, 0.0, pw, h, th.background, 1.0);
        text.draw(
            pm,
            &text.fit("🐋 Baleines · grosses transactions en direct", 15.0, pw - 2.0 * PAD),
            (PAD as f32, 30.0),
            15.0,
            WHITE,
            Align::Left,
        );
        fill(pm, 0.0, HEAD - 4.0, pw, 1.0, th.grid, 1.0);
        let hover = |r: &Rect| mouse.is_some_and(|(x, y)| r.contains(x, y));
        let zones = self.zones(text, pw);
        if self.follow.is_some() {
            fill(pm, 0.0, self.follow_y(), pw, self.follow_h() - 6.0, pal::field(), 1.0);
        }
        for (r, z) in &zones {
            let (label, on) = match z {
                Zone::Period(m) => (
                    PERIODS.iter().find(|(_, x)| x == m).map_or("", |(l, _)| *l).to_string(),
                    self.window_ms() == *m as i64 * 60_000,
                ),
                Zone::Level(k) => (LEVELS[*k].0.to_string(), *k == self.level),
                Zone::Kind(k) => (KINDS[*k as usize].to_string(), *k == self.kind),
                Zone::Exchange(None) => ("5 gros".to_string(), self.exchanges.is_empty()),
                Zone::Exchange(Some(k)) => {
                    let e = argos_data::whales::EXCHANGES[*k];
                    let short = match e {
                        "Hyperliquid" => "HL",
                        "Coinbase" => "Coinb.",
                        "Crypto.com" => "Crypto",
                        e => e,
                    };
                    (short.to_string(), self.exchanges_on().contains(&e))
                }
                Zone::Only(o) => (self.only_label(*o), *o == self.only),
                Zone::Bubbles => (BUBBLES.to_string(), self.bubbles),
                Zone::Explorer => (EXPLORER.to_string(), false),
                Zone::Unfollow => (UNFOLLOW.to_string(), false),
                Zone::Row(_) => continue,
            };
            round_fill(
                pm,
                *r,
                8.0,
                if on {
                    accent()
                } else if hover(r) {
                    pal::hover()
                } else {
                    pal::field()
                },
                if on { 0.3 } else { 1.0 },
            );
            round_stroke(pm, *r, 8.0, if on { accent() } else { pal::edge() }, 1.0);
            // ⏳ période plus longue que les données reçues : bouton grisé (toujours cliquable)
            let dim = matches!(z, Zone::Period(m) if self.beyond(*m, now_ms));
            text.draw(
                pm,
                &label,
                (r.cx() as f32, (r.cy() + 4.5) as f32),
                12.0,
                if dim { GREY } else { WHITE },
                Align::Center,
            );
        }
        // pression achat / vente
        let y0 = HEAD + (4.0 + ex_rows()) * (CHIP_H + 8.0) + 8.0;
        let (b, s) = self.pressure(now_ms);
        text.draw(
            pm,
            &text.fit(
                &format!("{} · achats {} · ventes {}", self.period_label(), money(b), money(s)),
                12.0,
                pw - 2.0 * PAD,
            ),
            (PAD as f32, (y0 + 14.0) as f32),
            12.0,
            GREY,
            Align::Left,
        );
        let bar = Rect::new(PAD, y0 + 24.0, pw - 2.0 * PAD, 10.0);
        round_fill(pm, bar, 5.0, pal::field(), 1.0);
        if b + s > 0.0 {
            let wb = bar.w * b / (b + s);
            fill(pm, bar.x, bar.y, wb, bar.h, crate::palette::up(), 1.0);
            fill(pm, bar.x + wb, bar.y, bar.w - wb, bar.h, crate::palette::down(), 1.0);
        }
        // ⏳ période plus longue que ce qui a été reçu : sur sa propre ligne, jamais coupée (texte plus court si étroit)
        if let Some(note) = self.coverage_note(now_ms) {
            let room = pw - 2.0 * PAD;
            let short = note.replace("Données reçues depuis", "Depuis");
            let line = if text.width(&note, 12.0) as f64 <= room { note } else { short };
            text.draw(pm, &line, (PAD as f32, (y0 + 54.0) as f32), 12.0, crate::palette::accent(), Align::Left);
        }
        // 📊 tous les ordres (bougies Binance), moyennes 7 j, et baleines de la période
        let room = pw - 2.0 * PAD;
        let coin = if self.flow_coin.is_empty() { "BTC" } else { &self.flow_coin };
        let mut ly = y0 + 74.0;
        let mut line = |pm: &mut Pixmap, s: String, c: crate::Rgb| {
            text.draw(pm, &text.fit(&s, 11.5, room), (PAD as f32, ly as f32), 11.5, c, Align::Left);
            ly += 18.0;
        };
        match self.flow_sum(now_ms) {
            Some((fb, fs)) => line(
                pm,
                format!("📊 {coin} tous les ordres (Binance) · achats {} · ventes {}", money(fb), money(fs)),
                WHITE,
            ),
            None => line(pm, format!("📊 {coin} tous les ordres : chargement…"), GREY),
        }
        if let Some(((ab, as_), r)) = self.flow_avg(now_ms) {
            line(
                pm,
                format!("Moy. 7 j par heure : achats {} · ventes {} · ici {r:.1}×", money(ab), money(as_)),
                GREY,
            );
        }
        let (n, avg) = self.whale_stats();
        let min = LEVELS[self.level.min(LEVELS.len() - 1)].0;
        line(
            pm,
            if n == 0 {
                format!("🐋 Aucune baleine ≥ {min} reçue")
            } else {
                format!("🐋 {n} baleines ≥ {min} · taille moyenne {}", money(avg))
            },
            GREY,
        );
        // 👁 baleine suivie : compte et positions
        if let Some(addr) = &self.follow {
            let y = self.follow_y();
            let head = match &self.account {
                Some(a) => format!("👁 {} · compte {}", short(addr), money(a.value)),
                None => format!("👁 {} · chargement…", short(addr)),
            };
            text.draw(
                pm,
                &text.fit(&head, 13.5, pw - 2.0 * PAD),
                (PAD as f32, (y + 21.0) as f32),
                13.5,
                GOLD,
                Align::Left,
            );
            let mut py = y + 30.0 + CHIP_H + 10.0;
            match &self.account {
                Some(a) if !a.positions.is_empty() => {
                    for p in &a.positions {
                        let (side, c) = if p.size > 0.0 {
                            ("LONG", crate::palette::up())
                        } else {
                            ("SHORT", crate::palette::down())
                        };
                        text.draw(pm, &p.coin, (PAD as f32, (py + 17.0) as f32), 13.0, WHITE, Align::Left);
                        text.draw(
                            pm,
                            &format!("{side} {:.0}x", p.leverage),
                            ((PAD + 56.0) as f32, (py + 17.0) as f32),
                            12.0,
                            c,
                            Align::Left,
                        );
                        text.draw(pm, &money(p.value), ((pw - PAD) as f32, (py + 17.0) as f32), 13.0, WHITE, Align::Right);
                        let liq = if p.liq > 0.0 { format!(" · liq. {}", price(p.liq)) } else { String::new() };
                        let line = format!("entrée {}{liq} · PnL {}{}", price(p.entry), if p.pnl >= 0.0 { "+" } else { "" }, money(p.pnl));
                        text.draw(
                            pm,
                            &text.fit(&line, 11.5, pw - 2.0 * PAD),
                            (PAD as f32, (py + 34.0) as f32),
                            11.5,
                            if p.pnl >= 0.0 { crate::palette::up() } else { crate::palette::down() },
                            Align::Left,
                        );
                        py += POS_ROW;
                    }
                }
                _ => {
                    let msg = if !self.follow_note.is_empty() {
                        self.follow_note.as_str()
                    } else if self.account.is_some() {
                        "Aucune position ouverte."
                    } else {
                        "Lecture des positions…"
                    };
                    text.draw(
                        pm,
                        &text.fit(msg, 12.5, pw - 2.0 * PAD),
                        (PAD as f32, (py + 20.0) as f32),
                        12.5,
                        GREY,
                        Align::Left,
                    );
                }
            }
        }
        let shown = self.shown();
        if shown.is_empty() {
            let msg = if !self.note.is_empty() {
                self.note.clone()
            } else {
                format!("En attente d'une transaction d'au moins {}…", LEVELS[self.level.min(LEVELS.len() - 1)].0)
            };
            text.draw(
                pm,
                &text.fit(&msg, 13.0, pw - 2.0 * PAD),
                (PAD as f32, (self.top() + 24.0) as f32),
                13.0,
                GREY,
                Align::Left,
            );
        }
        for (k, w) in shown.iter().enumerate() {
            let y = self.top() + k as f64 * ROW;
            let r = Rect::new(0.0, y, pw, ROW);
            if hover(&r) && argos_data::whales::is_address(&w.wallet) {
                fill(pm, 0.0, y, pw, ROW, pal::hover(), 1.0);
            }
            let side = if w.buy {
                ("ACHAT", crate::palette::up())
            } else {
                ("VENTE", crate::palette::down())
            };
            round_fill(pm, Rect::new(PAD, y + 9.0, 4.0, ROW - 18.0), 2.0, side.1, 1.0);
            text.draw(pm, short_coin(&w.coin), ((PAD + 12.0) as f32, (y + 21.0) as f32), 13.5, GOLD, Align::Left);
            text.draw(pm, side.0, ((PAD + 64.0) as f32, (y + 21.0) as f32), 12.0, side.1, Align::Left);
            text.draw(pm, &money(w.usd()), ((pw - PAD) as f32, (y + 21.0) as f32), 14.5, WHITE, Align::Right);
            let ago = (now_ms - w.time_ms).max(0) / 1000;
            let when = if ago < 60 {
                format!("il y a {ago} s")
            } else if ago < 3600 {
                format!("il y a {} min", ago / 60)
            } else {
                format!("il y a {} h", ago / 3600)
            };
            let who = match (&self.follow, w.wallet.len() > 12) {
                (Some(f), _) if *f == w.wallet => format!(" · 👁 {}", short(&w.wallet)),
                (_, true) => format!(" · {}", short(&w.wallet)),
                _ => String::new(),
            };
            // 📱 étroit : nom d'exchange abrégé et portefeuille sur la ligne du haut (rien de coupé)
            let room = pw - PAD - 26.0;
            let full = format!("{} @ {} · {} · {when}{who}", fmt_size(w.size), price(w.price), w.src);
            let line = if text.width(&full, 11.5) as f64 <= room {
                full
            } else {
                let src = match w.src {
                    "Hyperliquid" => "HL",
                    "Binance" => "BN",
                    s => s,
                };
                format!("{} @ {} · {src} · {when}", fmt_size(w.size), price(w.price))
            };
            text.draw(
                pm,
                &text.fit(&line, 11.5, room),
                ((PAD + 12.0) as f32, (y + 40.0) as f32),
                11.5,
                GREY,
                Align::Left,
            );
            fill(pm, PAD, y + ROW - 1.0, pw - 2.0 * PAD, 1.0, th.grid, 1.0);
        }
    }
}

/// Libellés des boutons.
const BUBBLES: &str = "🔵 Bulles sur la chart";
const EXPLORER: &str = "Explorateur ↗";
const UNFOLLOW: &str = "✕ Ne plus suivre";

/// 🔵 Dessine les bulles : au prix et au moment de chaque transaction, verte (achat) ou rouge (vente), rayon selon le
/// montant ; le montant écrit à côté à partir de 1 M$.
pub fn bubbles(pm: &mut Pixmap, text: &Text, geo: &crate::drawings::Geo, list: &[Whale]) {
    // les plus grosses d'abord : les petites restent visibles par-dessus
    let mut l: Vec<&Whale> = list.iter().collect();
    l.sort_by(|a, b| b.usd().total_cmp(&a.usd()));
    for w in l {
        let (x, y) = (geo.x_of(w.time_ms / 1000), geo.scale.y_of(w.price));
        let r = radius(w.usd());
        let c = if w.buy { crate::palette::up() } else { crate::palette::down() };
        let rect = Rect::new(x - r, y - r, 2.0 * r, 2.0 * r);
        round_fill(pm, rect, r, c, 0.35);
        round_stroke(pm, rect, r, c, 1.5);
        if w.usd() >= 1e6 {
            text.draw(pm, &money(w.usd()), ((x + r + 4.0) as f32, (y + 4.0) as f32), 11.0, c, Align::Left);
        }
    }
}

/// Rayon d'une bulle (px) : de 5 (100 k$) à 30 (≈ 30 M$ et plus).
fn radius(usd: f64) -> f64 {
    (5.0 * (usd / 1e5).max(0.0).sqrt()).clamp(4.0, 30.0)
}

/// Quantité courte (pièces).
fn fmt_size(q: f64) -> String {
    if q >= 1000.0 {
        format!("{q:.0}")
    } else {
        format!("{q:.3}").trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(coin: &str, usd: f64, buy: bool, t: i64, wallet: &str) -> Whale {
        Whale {
            coin: coin.into(),
            time_ms: t,
            price: 100.0,
            size: usd / 100.0,
            buy,
            src: "Hyperliquid",
            id: None,
            wallet: wallet.into(),
        }
    }

    /// 📱 Sur cell, seulement les 8 plus grosses bulles (Jee, 2026-11-09 : un paquet de bulles cachait les bougies).
    #[test]
    fn bulles_allegees_sur_cell() {
        let mut p = Whales {
            coin: "ETH".into(),
            bubbles: true,
            ..Default::default()
        };
        p.push(
            (0..20)
                .map(|k| w("ETH", 200e3 + k as f64 * 10e3, k % 2 == 0, 9_000_000 + k * 1000, ""))
                .collect(),
        );
        assert_eq!(p.on_chart(false).len(), 20, "PC : toutes");
        let cell = p.on_chart(true);
        assert_eq!(cell.len(), BULLES_CELL);
        assert!(cell.iter().all(|x| x.usd() >= 200e3 + 12.0 * 10e3 - 1.0), "les plus grosses");
    }

    /// 🔵 Les filtres du panneau (seuil, période, exchanges, Tout / Crypto) s'appliquent aussi aux bulles de la chart.
    #[test]
    fn bulles_suivent_les_filtres() {
        let mut p = Whales {
            coin: "ETH".into(),
            bubbles: true,
            ..Default::default()
        };
        let mut binance = w("ETH", 300e3, true, 9_000_000, "");
        binance.src = "Binance";
        p.push(vec![w("ETH", 150e3, true, 9_400_000, ""), w("ETH", 2e6, false, 9_500_000, ""), binance]);
        p.exchanges = vec!["Hyperliquid".into(), "Binance".into()];
        let usd = |p: &Whales| p.on_chart(false).iter().map(|x| x.usd()).collect::<Vec<_>>();
        assert_eq!(usd(&p), [2e6, 150e3, 300e3], "tout au-dessus de 100 k$");
        p.level = LEVELS.iter().position(|l| l.1 >= 250e3).unwrap();
        assert_eq!(usd(&p), [2e6, 300e3], "seuil 250 k$ : la bulle de 150 k$ disparaît");
        p.exchanges = vec!["Hyperliquid".into()];
        assert_eq!(usd(&p), [2e6], "sans Binance");
        p.exchanges = vec!["Hyperliquid".into(), "Binance".into()];
        (p.now_ms, p.period_min, p.level) = (10_000_000, 15, 0);
        assert_eq!(usd(&p), [2e6, 150e3], "période 15 min : la transaction Binance d'il y a 16 min disparaît");
        p.kind = 2;
        assert!(usd(&p).is_empty(), "Tradfi seulement : ETH n'en est pas");
    }

    #[test]
    fn panneau_baleines() {
        let t = Text::default();
        let mut p = Whales {
            coin: "ETH".into(),
            ..Default::default()
        };
        p.push(vec![w("BTC", 120e3, true, 1_000, "0x1111111111111111"), w("ETH", 2e6, false, 3_000, "")]);
        p.push(vec![w("ETH", 60e3, true, 5_000, "")]);
        assert_eq!(p.list[0].time_ms, 5_000, "la plus récente en haut");
        assert_eq!(p.shown().len(), 2, "60 k$ sous le seuil de 100 k$");
        assert_eq!(p.pressure(10_000), (120e3, 2e6));
        assert_eq!(money(2e6), "2,00 M$");
        assert_eq!(money(250e3), "250 k$");
        // « ETH seulement »
        let (r, _) = p.zones(&t, 390.0).into_iter().find(|(_, z)| *z == Zone::Only(true)).unwrap();
        assert_eq!(p.click(&t, 390.0, r.cx(), r.cy()), WhaleClick::Changed);
        assert_eq!(p.shown().len(), 1);
        p.only = false;
        // toucher la transaction Hyperliquid = suivre ce portefeuille
        let addr = "0x1111111111111111111111111111111111111111";
        p.list.iter_mut().find(|w| w.coin == "BTC").unwrap().wallet = addr.into();
        let y = p.top() + ROW * 1.5;
        assert_eq!(p.click(&t, 390.0, 100.0, y), WhaleClick::Follow(addr.into()));
        assert!(p.follow_h() > 0.0, "bloc 👁 ouvert");
        p.account = Some(Account {
            value: 3e6,
            positions: vec![argos_data::whales::Position {
                coin: "BTC".into(),
                size: 5.0,
                entry: 1e5,
                value: 5e5,
                pnl: -2e4,
                leverage: 10.0,
                liq: 9e4,
            }],
        });
        let (r, _) = p.zones(&t, 390.0).into_iter().find(|(_, z)| *z == Zone::Explorer).unwrap();
        assert_eq!(
            p.click(&t, 390.0, r.cx(), r.cy()),
            WhaleClick::Open(format!("https://app.hyperliquid.xyz/explorer/address/{addr}"))
        );
        let mut pm = crate::px::new(390, p.content_h() as u32).unwrap();
        p.draw(&mut pm, &t, 390.0, 10_000, None, &Theme::default());
        let (r, _) = p.zones(&t, 390.0).into_iter().find(|(_, z)| *z == Zone::Unfollow).unwrap();
        assert_eq!(p.click(&t, 390.0, r.cx(), r.cy()), WhaleClick::Changed);
        assert!(p.follow.is_none() && p.follow_h() == 0.0);
        assert_eq!(money(-2e4), "−20 k$");
        for (r, _) in p.zones(&t, 390.0).iter().filter(|(_, z)| !matches!(z, Zone::Row(_))) {
            assert!(r.x + r.w <= 390.0 - PAD + 0.1, "{r:?}");
        }
        // 🔵 bulles : seulement la pièce du graphique, interrupteur dans le panneau
        assert!(p.on_chart(false).is_empty(), "bulles éteintes par défaut");
        let (r, _) = p.zones(&t, 390.0).into_iter().find(|(_, z)| *z == Zone::Bubbles).unwrap();
        assert_eq!(p.click(&t, 390.0, r.cx(), r.cy()), WhaleClick::Changed);
        assert_eq!(p.on_chart(false).iter().map(|w| w.usd()).collect::<Vec<_>>(), [2e6], "ETH au-dessus de 100 k$");
        assert!(radius(1e5) < radius(1e6) && radius(1e9) == 30.0);
        let mut pm = crate::px::new(390, p.content_h() as u32).unwrap();
        p.draw(&mut pm, &t, 390.0, 10_000, None, &Theme::default());
    }
}
