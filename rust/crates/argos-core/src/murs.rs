//! 🧱 Argos Murs: the biggest resting order blocks of the order books, summed over several exchanges per price bucket,
//! in dollars (never traded volume). Perp and spot books are kept apart (never summed).
//!
//! Pure functions, no network: `aggregate` sums the books, `Tracker` follows each wall over time (held, TESTÉ, SWEEP,
//! RETIRÉ, SPOOF n×, eaten) and writes compact `Event`s, and `Replay` rebuilds the same walls from those events at a
//! replay clock without ever reading past it.
//!
//! Sizes are market data shown on the chart (not balances): `f64`, like the books they come from.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// One exchange's book: `(price, size in coins)` per level.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VenueBook {
    pub venue: String,
    pub bids: Vec<(f64, f64)>,
    pub asks: Vec<(f64, f64)>,
}

/// A far wall (« mur lointain »): one grouped level of a deep book, beyond the near range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lointain {
    pub price: f64,
    pub usd: f64,
    pub coins: f64,
    pub sell: bool,
    pub since_ms: i64,
}

/// A far wall already shown counts this much bigger when the far walls are ranked again (no flicker between two
/// groups of about the same size).
pub const LOIN_HYSTERESE: f64 = 0.10;

/// The far walls of `book` (a deep book grouped by the exchange, e.g. Hyperliquid `nSigFigs` 2): per side the
/// `per_side` biggest groups lying entirely beyond `near_pct` % of `mid` (a group overlapping the near range is not
/// one: the near walls show its money already) and within `far_pct` %. A group's size is the smallest gap between two
/// neighbouring prices of its side: a bid covers `[price, price + size)`, an ask `(price − size, price]`. A group
/// already in `avant` (same side, same price) counts `LOIN_HYSTERESE` bigger and keeps its `since_ms`; a new one gets
/// `now_ms`. Sorted by price, highest first.
pub fn lointains(book: &VenueBook, mid: f64, near_pct: f64, far_pct: f64, per_side: usize, avant: &[Lointain], now_ms: i64) -> Vec<Lointain> {
    if !(mid > 0.0) || far_pct <= near_pct || per_side == 0 {
        return vec![];
    }
    let (near_lo, near_hi) = (mid * (1.0 - near_pct / 100.0), mid * (1.0 + near_pct / 100.0));
    let (far_lo, far_hi) = (mid * (1.0 - far_pct / 100.0), mid * (1.0 + far_pct / 100.0));
    let mut out = vec![];
    for (levels, sell) in [(&book.bids, false), (&book.asks, true)] {
        let mut px: Vec<f64> = levels.iter().map(|l| l.0).filter(|p| p.is_finite() && *p > 0.0).collect();
        px.sort_by(f64::total_cmp);
        let size = px.windows(2).map(|w| w[1] - w[0]).filter(|d| *d > 0.0).fold(f64::INFINITY, f64::min);
        if !size.is_finite() {
            continue;
        }
        let eps = size * 1e-9;
        let mut cand: Vec<(f64, Lointain)> = levels
            .iter()
            .filter(|&&(p, q)| q > 0.0 && p > 0.0)
            .filter(|&&(p, _)| {
                if sell {
                    p - size >= near_hi - eps && p <= far_hi + eps
                } else {
                    p + size <= near_lo + eps && p >= far_lo - eps
                }
            })
            .map(|&(p, q)| {
                let deja = avant.iter().find(|a| a.sell == sell && (a.price - p).abs() <= eps.max(1e-9));
                let usd = p * q;
                let rang = usd * if deja.is_some() { 1.0 + LOIN_HYSTERESE } else { 1.0 };
                (
                    rang,
                    Lointain {
                        price: p,
                        usd,
                        coins: q,
                        sell,
                        since_ms: deja.map_or(now_ms, |a| a.since_ms),
                    },
                )
            })
            .collect();
        cand.sort_by(|a, b| b.0.total_cmp(&a.0));
        out.extend(cand.into_iter().take(per_side).map(|(_, l)| l));
    }
    out.sort_by(|a, b| b.price.total_cmp(&a.price));
    out
}

/// 📖 Lot loin-1 B: one exchange's book kept from its REST photos and its diff depth stream (Binance perp: the photo
/// stops at about ±0.2 %, the stream also tells the levels beyond it). Per price level (integer ticks of
/// `FUSION_TICK`): the last known absolute quantity and the update id it comes from (last writer wins by update id;
/// quantity 0 is kept as a tombstone until a photo is newer).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CarnetFusionne {
    /// Per tick: (price as the exchange wrote it, quantity, update id).
    bids: BTreeMap<i64, (f64, f64, u64)>,
    asks: BTreeMap<i64, (f64, f64, u64)>,
    /// Final update id of the last stream event (continuity: the next event's `pu`).
    dernier_u: Option<u64>,
    /// The last photo's span (lowest bid, highest ask) and update id.
    pub span: Option<(f64, f64)>,
    pub photo_id: u64,
}

/// Price grid of the merged book: 1e-8 (every exchange price is a multiple of it).
pub const FUSION_TICK: f64 = 1e-8;

fn cle(px: f64) -> i64 {
    (px / FUSION_TICK).round() as i64
}

impl CarnetFusionne {
    /// A stream event (`pu`: final id of the previous event, `u`: its own). A level changes only when `u` is newer
    /// than what is held there. A break of continuity (`pu` ≠ the previous `u`) is a gap first. Gives `true` on a gap.
    pub fn evenement(&mut self, pu: u64, u: u64, bids: &[(f64, f64)], asks: &[(f64, f64)]) -> bool {
        let trou = self.dernier_u.is_some_and(|d| d != pu);
        if trou {
            self.trou();
        }
        self.dernier_u = Some(u);
        for (side, levels) in [(&mut self.bids, bids), (&mut self.asks, asks)] {
            for &(px, q) in levels {
                let e = side.entry(cle(px)).or_insert((px, 0.0, 0));
                if u > e.2 {
                    *e = (px, q.max(0.0), u);
                }
            }
        }
        trou
    }

    /// A gap (lost continuity, a reconnection): every level outside the last photo's span is forgotten (it may have
    /// changed unseen); the next photo repairs the inside. No photo yet: everything is forgotten.
    pub fn trou(&mut self) {
        let span = self.span.map(|(a, b)| (cle(a), cle(b)));
        for side in [&mut self.bids, &mut self.asks] {
            side.retain(|k, _| span.is_some_and(|(a, b)| (a..=b).contains(k)));
        }
        self.dernier_u = None;
    }

    /// A REST photo with update id `l`, complete between its lowest bid and its highest ask: each of its levels is set
    /// where `l` is newer; a held level inside that span which the photo does not list is set to 0 where `l` is newer;
    /// levels outside are left alone. Then tombstones not newer than `l`, and levels beyond ±`garde_pct` % of `mid`,
    /// are dropped.
    pub fn photo(&mut self, l: u64, b: &VenueBook, mid: f64, garde_pct: f64) {
        let lo = b.bids.iter().chain(&b.asks).map(|x| x.0).fold(f64::INFINITY, f64::min);
        let hi = b.bids.iter().chain(&b.asks).map(|x| x.0).fold(f64::NEG_INFINITY, f64::max);
        if !(lo.is_finite() && hi.is_finite()) {
            return;
        }
        let (klo, khi) = (cle(lo), cle(hi));
        for (side, levels) in [(&mut self.bids, &b.bids), (&mut self.asks, &b.asks)] {
            let listes: std::collections::BTreeSet<i64> = levels.iter().map(|x| cle(x.0)).collect();
            for (k, e) in side.range_mut(klo..=khi) {
                if !listes.contains(k) && l > e.2 {
                    (e.1, e.2) = (0.0, l);
                }
            }
            for &(px, q) in levels.iter() {
                let e = side.entry(cle(px)).or_insert((px, 0.0, 0));
                if l > e.2 {
                    *e = (px, q.max(0.0), l);
                }
            }
        }
        self.span = Some((lo, hi));
        self.photo_id = l;
        let (glo, ghi) = (cle(mid * (1.0 - garde_pct / 100.0)), cle(mid * (1.0 + garde_pct / 100.0)));
        for side in [&mut self.bids, &mut self.asks] {
            side.retain(|k, e| !(e.1 <= 0.0 && e.2 <= l) && (!(mid > 0.0) || (glo..=ghi).contains(k)));
        }
    }

    /// The book as `VenueBook` (named `venue`): bids below `mid`, asks above, quantity above 0.
    pub fn livre(&self, venue: &str, mid: f64) -> VenueBook {
        let cote = |side: &BTreeMap<i64, (f64, f64, u64)>, bas: bool| -> Vec<(f64, f64)> {
            side.values()
                .map(|e| (e.0, e.1))
                .filter(|(px, q)| *q > 0.0 && if bas { *px < mid } else { *px > mid })
                .collect()
        };
        let mut bids = cote(&self.bids, true);
        bids.reverse();
        VenueBook {
            venue: venue.into(),
            bids,
            asks: cote(&self.asks, false),
        }
    }

    /// Levels with money held outside the last photo's span (known from the stream only).
    pub fn hors_photo(&self) -> usize {
        let Some((lo, hi)) = self.span.map(|(a, b)| (cle(a), cle(b))) else { return 0 };
        self.bids.iter().chain(&self.asks).filter(|(k, e)| e.1 > 0.0 && !(lo..=hi).contains(*k)).count()
    }

    /// Correctness check against a photo just merged: its levels compared, and how many differ in the merged book
    /// although no newer stream update holds them (expected 0).
    pub fn ecarts(&self, l: u64, b: &VenueBook) -> (usize, usize) {
        let mut n = 0;
        let mut faux = 0;
        for (side, levels) in [(&self.bids, &b.bids), (&self.asks, &b.asks)] {
            for &(px, q) in levels {
                n += 1;
                match side.get(&cle(px)) {
                    Some(e) if e.2 > l => {}
                    Some(e) if (e.1 - q).abs() <= 1e-12 => {}
                    _ => faux += 1,
                }
            }
        }
        (n, faux)
    }
}

/// A price bucket of the summed book (one side).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Cell {
    /// Summed size in dollars.
    pub usd: f64,
    /// Summed size in coins.
    pub coins: f64,
    /// Exchanges that have size here (bit i = book i).
    pub venues: u32,
    /// Exchanges that carry a wall here (their own size is at least `VENUE_MULT` × their own median bucket).
    pub strong: u32,
    /// Summed size in dollars of the base books only (`BASE_VENUES`): it decides which blocks become walls.
    pub base: f64,
    /// Size in dollars of the tag exchange alone (`TAG_VENUE`, Binance): its biggest bucket is always a wall.
    pub tag: f64,
    /// Size in dollars of each book (index = the book's place in `aggregate`'s list, the first `PAR_BOOKS`): the
    /// split by exchange of the tooltip on a depth bar or a wall.
    pub par: [f64; PAR_BOOKS],
}

/// Books whose own size is kept per bucket (`Cell::par`): more than Argos Murs ever reads on one side.
pub const PAR_BOOKS: usize = 8;

/// Base perp books (Jee, 21 Nov: « le mur soit binance okx hyperliquid »): the walls are chosen on their sum; the
/// other books (Bybit, Bitunix) only add their size to a wall that exists.
pub const BASE_VENUES: [&str; 3] = ["Binance", "OKX", "Hyperliquid"];

/// A wall already shown ranks this many times its size when the walls of a side are chosen (Jee, 22 Nov: the walls
/// must not swap on small changes).
pub const GARDE_RANG: f64 = 1.25;

/// The tagged Binance wall stays on its bucket while Binance holds there at least this share of its biggest bucket.
pub const TAG_GARDE: f64 = 0.8;

/// The exchange that always has one wall per side shown, tagged on its pill (« j'en veux au moins un de binance »).
pub const TAG_VENUE: &str = "Binance";

/// An exchange carries a wall at a bucket where it holds at least this many times its own median bucket.
pub const VENUE_MULT: f64 = 1.5;

impl Cell {
    /// Exchanges that carry a wall here.
    pub fn n_venues(&self) -> u32 {
        self.strong.count_ones()
    }
}

/// The summed books around the price: bucket index (price / step) → cell, per side.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Depth {
    pub step: f64,
    pub mid: f64,
    pub asks: BTreeMap<i64, Cell>,
    pub bids: BTreeMap<i64, Cell>,
    /// Per book: (lowest bid, highest ask) it reaches, to know which exchanges can see a bucket at all.
    pub reach: Vec<(f64, f64)>,
    /// Books with data in this update.
    pub venues: u32,
    /// A base book (`BASE_VENUES`) has data: walls are chosen on the base sum; otherwise on all books (fallback).
    pub base_ok: bool,
    /// The tag exchange (`TAG_VENUE`) has data.
    pub tag_ok: bool,
    /// Name of each book, in `aggregate`'s order (`Cell::par`).
    pub noms: Vec<String>,
}

/// A "round" step at or above `x` (1, 2, 2.5, 5 × 10ⁿ).
pub fn nice(x: f64) -> f64 {
    if !(x > 0.0) {
        return 1.0;
    }
    let e = 10f64.powf(x.log10().floor());
    [1.0, 2.0, 2.5, 5.0, 10.0]
        .iter()
        .map(|m| m * e)
        .find(|v| *v >= x - e * 1e-9)
        .unwrap_or(10.0 * e)
}

/// Bucket step for a price: about one thousandth of it, rounded (BTC 60 000 → 100 $, ETH 3 000 → 5 $).
pub fn step_for(price: f64) -> f64 {
    nice(price * 0.001)
}

/// Sums `books` per bucket of `step` within `range_pct` % of `mid`. Asks go above, bids below.
pub fn aggregate(books: &[VenueBook], mid: f64, step: f64, range_pct: f64) -> Depth {
    let mut d = Depth {
        step,
        mid,
        ..Default::default()
    };
    if !(mid > 0.0 && step > 0.0) {
        return d;
    }
    let lim = mid * range_pct / 100.0;
    d.noms = books.iter().take(32).map(|b| b.venue.clone()).collect();
    for (i, b) in books.iter().enumerate().take(32) {
        let bit = 1u32 << i;
        let (is_base, is_tag) = (BASE_VENUES.contains(&b.venue.as_str()), b.venue == TAG_VENUE);
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        // this exchange's own buckets first: it "carries" a wall where it holds at least VENUE_MULT × its own median
        let mut own: [BTreeMap<i64, (f64, f64)>; 2] = [BTreeMap::new(), BTreeMap::new()];
        for (s, levels) in [(0, &b.bids), (1, &b.asks)] {
            for &(px, sz) in levels.iter() {
                lo = lo.min(px);
                hi = hi.max(px);
                let side_ok = if s == 0 { px <= mid } else { px >= mid };
                if !(px > 0.0 && sz > 0.0) || !side_ok || (px - mid).abs() > lim {
                    continue;
                }
                let e = own[s].entry((px / step).round() as i64).or_insert((0.0, 0.0));
                e.0 += px * sz;
                e.1 += sz;
            }
        }
        for (s, cells) in own.iter().enumerate() {
            let mut sizes: Vec<f64> = cells.values().map(|c| c.0).collect();
            sizes.sort_by(f64::total_cmp);
            let median = sizes.get(sizes.len() / 2).copied().unwrap_or(0.0);
            let side = if s == 0 { &mut d.bids } else { &mut d.asks };
            for (&k, &(usd, coins)) in cells {
                let c = side.entry(k).or_default();
                c.usd += usd;
                c.coins += coins;
                c.venues |= bit;
                if is_base {
                    c.base += usd;
                }
                if is_tag {
                    c.tag += usd;
                }
                if let Some(x) = c.par.get_mut(i) {
                    *x += usd;
                }
                if usd >= median * VENUE_MULT {
                    c.strong |= bit;
                }
            }
        }
        if lo.is_finite() {
            d.venues |= bit;
            d.base_ok |= is_base;
            d.tag_ok |= is_tag;
            d.reach.push((lo, hi));
        } else {
            d.reach.push((f64::NAN, f64::NAN));
        }
    }
    d
}

/// The fine depth profile behind the depth bars: `books` summed on a grid of `step` (finer than the walls') within
/// `range_pct` % of `mid`. A book that only gives grouped levels, spaced wider than `step` (Hyperliquid groups its
/// book by significant figures), has each level's size spread evenly over its own group (the exchange only says the
/// money is somewhere in it), never beyond: no price gets money from a level that does not cover it.
pub fn profile(books: &[VenueBook], mid: f64, step: f64, range_pct: f64) -> Depth {
    if !(step > 0.0) {
        return aggregate(books, mid, step, range_pct);
    }
    let fine: Vec<VenueBook> = books
        .iter()
        .map(|b| VenueBook {
            venue: b.venue.clone(),
            bids: spread(&b.bids, step),
            asks: spread(&b.asks, step),
        })
        .collect();
    aggregate(&fine, mid, step, range_pct)
}

/// One side of a book on the fine grid: unchanged when its levels are at most 1.5 `step` apart (median spacing of
/// the first levels); otherwise each level split evenly over its group (the median spacing), centred on it.
fn spread(levels: &[(f64, f64)], step: f64) -> Vec<(f64, f64)> {
    let mut gaps: Vec<f64> = levels.windows(2).take(64).map(|w| (w[1].0 - w[0].0).abs()).filter(|g| *g > 0.0).collect();
    gaps.sort_by(f64::total_cmp);
    let Some(&group) = gaps.get(gaps.len().saturating_sub(1) / 2) else {
        return levels.to_vec();
    };
    if group <= step * 1.5 {
        return levels.to_vec();
    }
    let n = ((group / step).round() as usize).clamp(1, 400);
    levels
        .iter()
        .flat_map(|&(px, sz)| (0..n).map(move |i| (px + (i as f64 - (n as f64 - 1.0) / 2.0) * step, sz / n as f64)))
        .collect()
}

impl Depth {
    /// Price of bucket `k`.
    pub fn price(&self, k: i64) -> f64 {
        k as f64 * self.step
    }

    /// Books that reach the price `px` (their snapshot goes that deep).
    pub fn reaching(&self, px: f64) -> u32 {
        self.reach
            .iter()
            .filter(|(lo, hi)| *lo <= px + self.step / 2.0 && px - self.step / 2.0 <= *hi)
            .count() as u32
    }

    /// The books whose snapshot covers the price `px` (bit i = book i): its coverage class (nested bands around the
    /// price, Binance's ±0.2 % inside OKX's ±1.5 % inside Hyperliquid's ±3 %).
    pub fn reach_mask(&self, px: f64) -> u32 {
        self.reach
            .iter()
            .enumerate()
            .take(32)
            .filter(|(_, (lo, hi))| *lo <= px && px <= *hi)
            .fold(0, |m, (i, _)| m | 1 << i)
    }

    /// The size that decides the walls: the base books' sum when one of them is read, all books otherwise.
    pub fn rank(&self, c: &Cell) -> f64 {
        if self.base_ok {
            c.base
        } else {
            c.usd
        }
    }

    /// The tag exchange's biggest bucket of a side (not the bucket holding the price): its wall, whatever its size.
    pub fn tag_wall(&self, sell: bool) -> Option<(i64, Cell)> {
        if !self.tag_ok {
            return None;
        }
        let half = self.step / 2.0;
        self.side(sell)
            .iter()
            .filter(|(k, c)| c.tag > 0.0 && (self.price(**k) - self.mid).abs() >= half)
            .max_by(|a, b| a.1.tag.total_cmp(&b.1.tag))
            .map(|(k, c)| (*k, *c))
    }

    fn side(&self, sell: bool) -> &BTreeMap<i64, Cell> {
        if sell {
            &self.asks
        } else {
            &self.bids
        }
    }

    /// Summed size of one side, in coins.
    pub fn total_coins(&self, sell: bool) -> f64 {
        self.side(sell).values().map(|c| c.coins).sum()
    }

    /// Cells of one side that pass the wall test: at least `mult` × the side's median size and seen on at least
    /// `min_venues` books (or on every book that reaches that price, when fewer reach it). Biggest first.
    pub fn qualifying(&self, sell: bool, mult: f64, min_venues: u32) -> Vec<(i64, Cell)> {
        self.qualifying_by(sell, mult, min_venues, false)
    }

    /// Like [`Depth::qualifying`]; `base`: ranked on the base books' sum ([`Depth::rank`]), and a block the base books
    /// do not hold (only Bybit or Bitunix) is no wall.
    pub fn qualifying_by(&self, sell: bool, mult: f64, min_venues: u32, base: bool) -> Vec<(i64, Cell)> {
        let base = base && self.base_ok;
        let side = self.side(sell);
        if side.is_empty() {
            return vec![];
        }
        let mut sizes: Vec<f64> = side.values().map(|c| c.usd).collect();
        sizes.sort_by(f64::total_cmp);
        let median = sizes[sizes.len() / 2];
        let mut out: Vec<(i64, Cell)> = side
            .iter()
            .filter(|(k, c)| {
                let need = min_venues.min(self.reaching(self.price(**k)));
                c.usd >= median * mult && (min_venues == 0 || c.n_venues() >= need.max(1)) && (!base || c.base > 0.0)
            })
            .map(|(k, c)| (*k, *c))
            .collect();
        if base {
            out.sort_by(|a, b| b.1.base.total_cmp(&a.1.base));
        } else {
            out.sort_by(|a, b| b.1.usd.total_cmp(&a.1.usd));
        }
        out
    }
}

/// Settings of the wall tracker.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cfg {
    /// Walls shown per side.
    pub per_side: usize,
    /// Minimum size, × the side's median bucket.
    pub mult: f64,
    /// Minimum number of exchanges.
    pub min_venues: u32,
    /// Held this long before it is shown (ms).
    pub hold_ms: i64,
    /// Below this share of its peak, the wall is gone.
    pub gone_ratio: f64,
    /// "Close" to the price, in % (a pull that close counts toward SPOOF).
    pub near_pct: f64,
    /// Grey words (RETIRÉ, TESTÉ, SPOOF, MANGÉ) fade after this long (ms).
    pub ghost_ms: i64,
    /// Books read within this % of the price.
    pub range_pct: f64,
    /// `true`: a wall must be seen on `min_venues` exchanges and held `hold_ms` before it is shown. `false` (default,
    /// Jee 21 Nov): the biggest blocks are shown from the first photo, exchanges and hold time are only quality marks.
    pub gate: bool,
    /// Perp: the tag exchange's biggest bucket per side is always shown and tagged (setting « Toujours montrer un mur
    /// de Binance », on by default).
    pub tag: bool,
}

impl Default for Cfg {
    fn default() -> Self {
        Self {
            per_side: 3,
            mult: 1.5,
            min_venues: 3,
            hold_ms: 3 * 60_000,
            gone_ratio: 0.3,
            near_pct: 0.6,
            ghost_ms: 4 * 3_600_000,
            range_pct: 3.0,
            gate: false,
            tag: true,
        }
    }
}

/// How a wall ended (grey word left behind).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fin {
    /// Pulled before the price touched it.
    Retire,
    /// Touched at least once, held, then went away.
    Teste,
    /// Pulled n times (n ≥ 2) at about the same price as the price came close, never traded.
    Spoof(u32),
    /// The price went through and the size is gone.
    Mange,
}

impl Fin {
    /// The grey word on the chart.
    pub fn mot(self) -> String {
        match self {
            Fin::Retire => "RETIRÉ".into(),
            Fin::Teste => "TESTÉ".into(),
            Fin::Spoof(n) => format!("SPOOF {n}×"),
            Fin::Mange => "MANGÉ".into(),
        }
    }
}

/// A wall followed by the tracker.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wall {
    pub key: i64,
    pub sell: bool,
    pub price: f64,
    pub usd: f64,
    pub coins: f64,
    pub peak: f64,
    pub venues: u32,
    pub first_ms: i64,
    /// Times the price touched it and left (TESTÉ n×).
    pub tests: u32,
    touching: bool,
    /// Passes the wall test in the last update.
    qual: bool,
    /// Distance to the price when first seen (%).
    dist0: f64,
    /// Shown as a pill in the last update (for the events).
    shown: bool,
    /// Size written in the last Show / Size event (a new Size event only after a 5 % change).
    recorded: f64,
    /// When it started being shown (its line starts there); 0: never shown.
    shown_ms: i64,
    /// Exchanges written in the last Show / Size event (a change writes a new Size event).
    rec_venues: u32,
    /// The size that ranks it among the walls (the base books' sum, see [`Depth::rank`]).
    rank: f64,
    /// The tag exchange's biggest bucket of its side in the last photo (always shown, tagged).
    pub tag: bool,
    /// Its money per book now (`Cell::par`), and at its last Show / Size event (the pill's split, adding up to
    /// `recorded`).
    pub par: [f64; PAR_BOOKS],
    pub rec_par: [f64; PAR_BOOKS],
}

/// A grey word left where a wall ended.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ghost {
    pub key: i64,
    pub sell: bool,
    pub price: f64,
    pub fin: Fin,
    pub at_ms: i64,
    /// Where its dashed line starts: when the wall was first shown (or when it ended, never shown).
    pub since_ms: i64,
}

/// What happened to a wall (recorded for the replay).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Kind {
    /// Starts being shown as a pill (size in $).
    Show,
    /// Size changed by 5 % or more while shown.
    Size,
    /// No longer among the shown walls, still there.
    Hide,
    /// Touched and left; the wall holds.
    Tested,
    /// The wick went through, the candle closed back on the wall's side.
    Sweep,
    /// The wall ended (grey word).
    End(Fin),
    /// Recording heartbeat (once a minute while the books are read): the replay knows which periods exist.
    Alive,
    /// Really filled by trades (REMPLI): eaten, and takers traded at least `PART_REMPLI` of its size at its price;
    /// `usd`: its size before it was hit.
    Filled,
}

/// A compact wall event: time, bucket, side, size, perp or spot.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub t_ms: i64,
    pub key: i64,
    pub step: f64,
    pub sell: bool,
    pub spot: bool,
    pub usd: f64,
    pub kind: Kind,
    /// Exchanges that carried it (quality mark; 0 in recordings made before it existed).
    #[serde(default)]
    pub venues: u32,
}

impl Event {
    pub fn price(&self) -> f64 {
        self.key as f64 * self.step
    }
}

/// Two shown walls of a side are at least this share of the price apart (0.3 %; at least one bucket).
pub const SPACING: f64 = 0.003;

/// A shown wall's size is written again once it moved this much (5 %).
pub const SIZE_STEP: f64 = 0.05;

/// Follows the walls of one kind of book (perp or spot) over time.
#[derive(Debug, Clone, PartialEq)]
pub struct Tracker {
    pub cfg: Cfg,
    pub spot: bool,
    pub step: f64,
    walls: BTreeMap<(bool, i64), Wall>,
    /// Pulls near the price per bucket, for the whole session (SPOOF n×).
    pulls: HashMap<(bool, i64), u32>,
    ghosts: Vec<Ghost>,
    events: Vec<Event>,
    /// Sweeps seen: (bar time, price, sell).
    pub sweeps: Vec<(i64, f64, bool)>,
}

impl Tracker {
    pub fn new(cfg: Cfg, spot: bool) -> Self {
        Self {
            cfg,
            spot,
            step: 0.0,
            walls: BTreeMap::new(),
            pulls: HashMap::new(),
            ghosts: vec![],
            events: vec![],
            sweeps: vec![],
        }
    }

    fn event(&mut self, t_ms: i64, w: &Wall, kind: Kind) {
        self.events.push(Event {
            t_ms,
            key: w.key,
            step: self.step,
            sell: w.sell,
            spot: self.spot,
            usd: w.usd,
            kind,
            venues: w.venues,
        });
    }

    fn ghost(&mut self, w: &Wall, fin: Fin, now: i64) {
        self.ghosts.retain(|g| !(g.key == w.key && g.sell == w.sell));
        self.ghosts.push(Ghost {
            key: w.key,
            sell: w.sell,
            price: w.price,
            fin,
            at_ms: now,
            since_ms: if w.shown_ms > 0 { w.shown_ms } else { now },
        });
    }

    /// One new photo of the summed books at `now`. `lo` / `hi`: lowest and highest price traded since the previous
    /// update (the price path), `price`: the last price.
    pub fn update(&mut self, now: i64, d: &Depth, price: f64, lo: f64, hi: f64) {
        if d.step != self.step {
            // another grid (symbol or price scale changed): the shown walls leave (recorded, so a replay drops them
            // too), then start again; the session's SPOOF counts go too
            let events = std::mem::take(&mut self.events);
            let (old, old_step) = (self.shown(), self.step);
            *self = Self::new(self.cfg, self.spot);
            self.events = events;
            for w in &old {
                self.events.push(Event {
                    t_ms: now,
                    key: w.key,
                    step: old_step,
                    sell: w.sell,
                    spot: self.spot,
                    usd: w.recorded,
                    kind: Kind::Hide,
                    venues: w.venues,
                });
            }
            self.step = d.step;
        }
        let half = self.step / 2.0;
        let cfg = self.cfg;
        let mut qual: HashMap<(bool, i64), Cell> = HashMap::new();
        for sell in [true, false] {
            // default (no gate): simply the biggest blocks of the side, whatever the median
            let (need, mult) = if cfg.gate { (cfg.min_venues, cfg.mult) } else { (0, 0.0) };
            for (k, c) in d.qualifying_by(sell, mult, need, !self.spot).into_iter().take(cfg.per_side * 4) {
                qual.insert((sell, k), c);
            }
        }
        // the tag exchange's biggest bucket of each side: a wall whatever its size, hold or exchanges (perp only)
        let tags: Vec<(bool, i64)> = if self.spot || !cfg.tag {
            vec![]
        } else {
            [true, false]
                .into_iter()
                .filter_map(|sell| {
                    let (k, c) = d.tag_wall(sell)?;
                    // the tagged wall stays on its bucket while Binance still holds there at least TAG_GARDE of its
                    // biggest bucket of the side (no swap at every photo for a few %)
                    let garde = self.walls.iter().find(|(id, w)| id.0 == sell && w.tag).map(|(id, _)| *id).and_then(|id| {
                        d.side(sell)
                            .get(&id.1)
                            .filter(|x| x.tag >= c.tag * TAG_GARDE && (d.price(id.1) - d.mid).abs() >= half)
                            .map(|x| (id, *x))
                    });
                    Some(garde.unwrap_or(((sell, k), c)))
                })
                .map(|(id, c)| {
                    qual.insert(id, c);
                    id
                })
                .collect()
        };
        let spot = self.spot;
        let rank = |c: &Cell| if spot { c.usd } else { d.rank(c) };
        let keys: Vec<(bool, i64)> = self.walls.keys().copied().collect();
        for id in keys {
            let Some(mut w) = self.walls.get(&id).copied() else { continue };
            let cell = d.side(w.sell).get(&w.key).copied().unwrap_or_default();
            let (top, bot) = (w.price + half, w.price - half);
            let touched = hi >= bot && lo <= top;
            let crossed = if w.sell { price > top } else { price < bot };
            let gone = cell.usd < w.peak * cfg.gone_ratio;
            let far = (w.price / price - 1.0).abs() * 100.0 > cfg.range_pct * 1.5;
            if gone {
                let fin = if crossed || touched || w.touching {
                    // traded through: never a spoof
                    self.pulls.remove(&id);
                    Fin::Mange
                } else {
                    let dist = (w.price / price - 1.0).abs() * 100.0;
                    // near the price, and not further than when it appeared: a wall that flickers in place (pulled,
                    // put back, pulled) counts too, a wall left behind by the price does not
                    if dist <= cfg.near_pct && dist <= w.dist0 + 1e-9 {
                        let n = self.pulls.entry(id).or_insert(0);
                        *n += 1;
                        if *n >= 2 {
                            Fin::Spoof(*n)
                        } else if w.tests > 0 {
                            Fin::Teste
                        } else {
                            Fin::Retire
                        }
                    } else if w.tests > 0 {
                        Fin::Teste
                    } else {
                        Fin::Retire
                    }
                };
                self.walls.remove(&id);
                // only walls that were shown leave a grey word (a candidate that vanished never was on the chart)
                if w.shown || matches!(fin, Fin::Spoof(_)) {
                    self.ghost(&w, fin, now);
                    // the size it had before it went (the last one shown, else its peak), not the remains
                    let avant = Wall {
                        usd: if w.recorded > 0.0 { w.recorded } else { w.peak },
                        ..w
                    };
                    self.event(now, &avant, Kind::End(fin));
                }
                continue;
            }
            if far {
                self.walls.remove(&id);
                if w.shown {
                    self.event(now, &w, Kind::Hide);
                }
                continue;
            }
            if touched {
                w.touching = true;
            } else if w.touching && (w.price - price).abs() > self.step {
                w.touching = false;
                w.tests += 1;
                self.event(now, &w, Kind::Tested);
            }
            w.usd = cell.usd;
            w.par = cell.par;
            w.coins = cell.coins;
            w.venues = cell.n_venues();
            w.peak = w.peak.max(cell.usd);
            w.qual = qual.contains_key(&id);
            w.rank = rank(&cell);
            w.tag = tags.contains(&id);
            self.walls.insert(id, w);
        }
        for (id, c) in qual {
            // a new wall never starts in the bucket holding the price: that is the book at the touch, not a wall
            if self.walls.contains_key(&id) || (id.1 as f64 * self.step - price).abs() < half {
                continue;
            }
            let (r, tag) = (rank(&c), tags.contains(&id));
            let p = id.1 as f64 * self.step;
            self.walls.insert(
                id,
                Wall {
                    key: id.1,
                    sell: id.0,
                    price: p,
                    usd: c.usd,
                    coins: c.coins,
                    peak: c.usd,
                    venues: c.n_venues(),
                    first_ms: now,
                    tests: 0,
                    touching: false,
                    qual: true,
                    dist0: (p / price - 1.0).abs() * 100.0,
                    shown: false,
                    recorded: 0.0,
                    shown_ms: 0,
                    rec_venues: 0,
                    par: c.par,
                    rec_par: [0.0; PAR_BOOKS],
                    rank: r,
                    tag,
                },
            );
        }
        self.ghosts.retain(|g| now - g.at_ms < cfg.ghost_ms);
        // the walls shown now, and their events (Show, Size, Hide)
        let shown: Vec<(bool, i64)> = self.pick(now).iter().map(|w| (w.sell, w.key)).collect();
        let ids: Vec<(bool, i64)> = self.walls.keys().copied().collect();
        for id in ids {
            let Some(mut w) = self.walls.get(&id).copied() else { continue };
            let now_shown = shown.contains(&id);
            if now_shown && !w.shown {
                (w.recorded, w.shown_ms, w.rec_venues, w.rec_par) = (w.usd, now, w.venues, w.par);
                self.event(now, &w, Kind::Show);
            } else if !now_shown && w.shown {
                self.event(now, &w, Kind::Hide);
            } else if now_shown && w.recorded > 0.0 && ((w.usd / w.recorded - 1.0).abs() >= SIZE_STEP || w.venues != w.rec_venues) {
                (w.recorded, w.rec_venues, w.rec_par) = (w.usd, w.venues, w.par);
                self.event(now, &w, Kind::Size);
            }
            w.shown = now_shown;
            self.walls.insert(id, w);
        }
    }

    /// A candle closed: a wick through a shown wall with the close back on its side is a SWEEP (the wall holds).
    pub fn bar_closed(&mut self, now: i64, bar_t: i64, high: f64, low: f64, close: f64) {
        let half = self.step / 2.0;
        let shown: Vec<Wall> = self.pick(now);
        for w in shown {
            let swept = if w.sell {
                high > w.price + half && close < w.price - half
            } else {
                low < w.price - half && close > w.price + half
            };
            if swept {
                self.sweeps.push((bar_t, w.price, w.sell));
                self.event(now, &w, Kind::Sweep);
            }
        }
        if self.sweeps.len() > 200 {
            self.sweeps.drain(..self.sweeps.len() - 200);
        }
    }

    fn pick(&self, now: i64) -> Vec<Wall> {
        let mut out = vec![];
        for sell in [true, false] {
            let mut l: Vec<Wall> = self
                .walls
                .values()
                .filter(|w| w.sell == sell && w.qual && (w.tag || !self.cfg.gate || now - w.first_ms >= self.cfg.hold_ms))
                .copied()
                .collect();
            // a wall already shown ranks × GARDE_RANG: it is replaced only by a clearly bigger one (no swap on small
            // changes between two photos)
            let r = |w: &Wall| if w.shown { w.rank * GARDE_RANG } else { w.rank };
            l.sort_by(|a, b| r(b).total_cmp(&r(a)));
            let tag = l.iter().find(|w| w.tag).copied();
            // distinct blocks: a smaller one right next to a bigger one (the same block spread over two buckets) is
            // not a wall of its own, so the walls spread over the range instead of piling up
            let mut kept: Vec<Wall> = vec![];
            for w in l {
                if kept.len() < self.cfg.per_side && kept.iter().all(|k| (k.price - w.price).abs() >= self.step.max(w.price * SPACING)) {
                    kept.push(w);
                }
            }
            // the tag exchange's wall is always there: tagged when it is among them, one more pill otherwise
            if let Some(t) = tag.filter(|t| !kept.iter().any(|k| k.key == t.key)) {
                kept.push(t);
            }
            out.extend(kept);
        }
        out
    }

    /// The walls shown now (pills): held long enough, passing the test, the biggest per side.
    pub fn shown(&self) -> Vec<Wall> {
        self.walls.values().filter(|w| w.shown).copied().collect()
    }

    /// Grey words still on the chart.
    pub fn ghosts(&self) -> &[Ghost] {
        &self.ghosts
    }

    /// Events written since the last call (to record).
    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }
}

/// A wall pill on the chart.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pill {
    pub price: f64,
    pub usd: f64,
    pub sell: bool,
    pub spot: bool,
    /// When it started being shown (its line starts there).
    pub since_ms: i64,
    /// Exchanges that carry it (quality mark).
    pub venues: u32,
    /// The tag exchange's wall (BINANCE on the pill). Live only: a replay does not know it (events unchanged).
    pub tag: bool,
    /// The pill's money per book (`Depth::noms` of its tracker's books), adding up to `usd`; zeros in a replay.
    pub par: [f64; PAR_BOOKS],
}

/// A grey word on the chart (RETIRÉ, TESTÉ, SPOOF n×, MANGÉ).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Word {
    pub price: f64,
    pub sell: bool,
    pub spot: bool,
    pub fin: Fin,
    pub at_ms: i64,
    /// Where its dashed line starts.
    pub since_ms: i64,
}

/// A wall really filled by trades: the REMPLI mark (its size before it was hit).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rempli {
    pub price: f64,
    pub sell: bool,
    pub spot: bool,
    pub usd: f64,
    pub at_ms: i64,
}

/// What the indicator draws: pills, grey words and REMPLI marks (live or replayed).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Vue {
    pub pills: Vec<Pill>,
    pub words: Vec<Word>,
    pub remplis: Vec<Rempli>,
}

/// Share of the chart exchange's own part of a wall that its takers must trade at the wall's price for REMPLI (about
/// half; less is a pull).
pub const PART_REMPLI: f64 = 0.5;

/// The chart exchange's usual share of the summed book when it cannot be measured (15 %).
pub const PART_BOURSE_DEFAUT: f64 = 0.15;

/// What the chart's own exchange holds of a wall: the trades read are that exchange's only, so they are compared to
/// its own part of the summed wall, never to the whole sum.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PartBourse {
    /// Its own size ($) in the wall's bucket, in the last photos before the hit (0: none there).
    pub own_usd: f64,
    /// Its usual share of the summed book (0..1); `None`: unknown, `PART_BOURSE_DEFAUT` is used.
    pub share: Option<f64>,
}

impl PartBourse {
    /// Dollars the chart exchange's takers must trade at the wall for REMPLI: half of its own part of the wall, or,
    /// when it holds nothing there, half of the summed wall scaled by its usual share of the book.
    pub fn seuil(&self, wall_usd: f64) -> f64 {
        if self.own_usd > 0.0 {
            PART_REMPLI * self.own_usd
        } else {
            let share = self.share.filter(|s| *s > 0.0 && *s <= 1.0).unwrap_or(PART_BOURSE_DEFAUT);
            PART_REMPLI * wall_usd * share
        }
    }
}

/// One exchange's own size ($) in the bucket of `step` around `price`, on the asks (`sell`) or the bids.
pub fn own_usd(book: &VenueBook, price: f64, step: f64, sell: bool) -> f64 {
    let k = (price / step).round() as i64;
    let levels = if sell { &book.asks } else { &book.bids };
    levels
        .iter()
        .filter(|(px, sz)| *px > 0.0 && *sz > 0.0 && (px / step).round() as i64 == k)
        .map(|(px, sz)| px * sz)
        .sum()
}

/// Share (0..1) of book `own` in the summed books within `range_pct` % of `mid` (both sides); `None` when unknown.
pub fn usual_share(books: &[VenueBook], own: usize, mid: f64, range_pct: f64) -> Option<f64> {
    let lim = mid * range_pct / 100.0;
    let usd = |b: &VenueBook| -> f64 {
        b.bids
            .iter()
            .chain(&b.asks)
            .filter(|(px, sz)| *px > 0.0 && *sz > 0.0 && (px - mid).abs() <= lim)
            .map(|(px, sz)| px * sz)
            .sum()
    };
    let total: f64 = books.iter().map(usd).sum();
    let mine = usd(books.get(own)?);
    (total > 0.0 && mine > 0.0).then(|| mine / total)
}

/// Taker volume (coins) of the 1-minute orderflow bars that overlap `from_ms..=to_ms`, at prices within `lo..=hi`:
/// market buys (`buys`) or market sells.
pub fn volume_preneur(minutes: &[crate::orderflow::FlowBar], from_ms: i64, to_ms: i64, lo: f64, hi: f64, buys: bool) -> f64 {
    minutes
        .iter()
        .filter(|b| b.time * 1000 + 60_000 > from_ms && b.time * 1000 <= to_ms)
        .flat_map(|b| b.rangees.iter().map(move |r| (r, b.pas)))
        .filter(|((k, _, _), pas)| (*k as f64 + 1.0) * pas > lo && *k as f64 * pas <= hi)
        .map(|((_, a, v), _)| if buys { *a } else { *v })
        .sum()
}

/// 🧱 Was this ended wall really filled? Only an eaten wall (the price went to or through it) whose side was hit by
/// the chart exchange's takers, at its price, between `since_ms` and its end (`minutes`: the real trades of the
/// chart's market), for at least `part.seuil` (half of that exchange's own part of the wall). A wall gone without
/// such trades was pulled: `None`, nothing drawn.
pub fn rempli(e: &Event, minutes: &[crate::orderflow::FlowBar], since_ms: i64, part: PartBourse) -> Option<Rempli> {
    if e.kind != Kind::End(Fin::Mange) || !(e.usd > 0.0) {
        return None;
    }
    let (p, half) = (e.price(), e.step / 2.0);
    // a sell wall is hit by market buys, a buy wall by market sells
    let coins = volume_preneur(minutes, since_ms, e.t_ms, p - half, p + half, e.sell);
    (coins > 0.0 && coins * p >= part.seuil(e.usd)).then_some(Rempli {
        price: p,
        sell: e.sell,
        spot: e.spot,
        usd: e.usd,
        at_ms: e.t_ms,
    })
}

impl Vue {
    /// Pills and words in a fixed order (to compare two views).
    pub fn sorted(mut self) -> Self {
        self.pills
            .sort_by(|a, b| (a.spot, a.sell).cmp(&(b.spot, b.sell)).then(a.price.total_cmp(&b.price)));
        self.words
            .sort_by(|a, b| (a.spot, a.sell).cmp(&(b.spot, b.sell)).then(a.price.total_cmp(&b.price)));
        self
    }

    /// Tests: the view as a replay rebuilds it (sorted, the pills' split by exchange unknown: it is not recorded).
    #[cfg(test)]
    pub fn comme_relue(mut self) -> Self {
        for p in self.pills.iter_mut() {
            p.par = [0.0; PAR_BOOKS];
        }
        self.sorted()
    }

    /// Adds another view (perp + spot).
    pub fn with(mut self, other: Vue) -> Self {
        self.pills.extend(other.pills);
        self.words.extend(other.words);
        self.remplis.extend(other.remplis);
        self
    }
}

impl Tracker {
    /// The view now: the shown walls at their recorded size (the size of their last event, so a replay of the
    /// events draws exactly the same pills) and the grey words.
    pub fn vue(&self) -> Vue {
        Vue {
            pills: self
                .shown()
                .iter()
                .map(|w| Pill {
                    price: w.price,
                    usd: w.recorded,
                    sell: w.sell,
                    spot: self.spot,
                    since_ms: w.shown_ms,
                    venues: w.rec_venues,
                    tag: w.tag,
                    par: w.rec_par,
                })
                .collect(),
            words: self
                .ghosts
                .iter()
                .map(|g| Word {
                    price: g.price,
                    sell: g.sell,
                    spot: self.spot,
                    fin: g.fin,
                    at_ms: g.at_ms,
                    since_ms: g.since_ms,
                })
                .collect(),
            remplis: vec![],
        }
    }
}

/// 🎬 Rebuilds the view from recorded `events` at the replay clock `until_ms`: only events at or before it are read
/// (never the future). Grey words older than `ghost_ms` are gone, as live.
pub fn replay(events: &[Event], until_ms: i64, ghost_ms: i64) -> Vue {
    let mut pills: BTreeMap<(bool, bool, i64), Pill> = BTreeMap::new();
    let mut words: BTreeMap<(bool, bool, i64), Word> = BTreeMap::new();
    let mut since: BTreeMap<(bool, bool, i64), i64> = BTreeMap::new();
    let mut remplis = vec![];
    for e in events.iter().filter(|e| e.t_ms <= until_ms) {
        let id = (e.spot, e.sell, e.key);
        match e.kind {
            Kind::Show | Kind::Size => {
                if e.kind == Kind::Show {
                    since.insert(id, e.t_ms);
                }
                pills.insert(
                    id,
                    Pill {
                        price: e.price(),
                        usd: e.usd,
                        sell: e.sell,
                        spot: e.spot,
                        since_ms: since.get(&id).copied().unwrap_or(e.t_ms),
                        venues: e.venues,
                        tag: false,
                        par: [0.0; PAR_BOOKS],
                    },
                );
            }
            Kind::Hide => {
                pills.remove(&id);
            }
            Kind::End(fin) => {
                pills.remove(&id);
                words.insert(
                    id,
                    Word {
                        price: e.price(),
                        sell: e.sell,
                        spot: e.spot,
                        fin,
                        at_ms: e.t_ms,
                        since_ms: since.remove(&id).unwrap_or(e.t_ms),
                    },
                );
            }
            Kind::Filled => {
                if until_ms - e.t_ms < ghost_ms {
                    remplis.push(Rempli {
                        price: e.price(),
                        sell: e.sell,
                        spot: e.spot,
                        usd: e.usd,
                        at_ms: e.t_ms,
                    });
                }
            }
            Kind::Tested | Kind::Sweep | Kind::Alive => {}
        }
    }
    Vue {
        pills: pills.into_values().collect(),
        words: words.into_values().filter(|w| until_ms - w.at_ms < ghost_ms).collect(),
        remplis,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The depth bars' profile: a dense book stays as it is; a grouped one (100 $ groups) is spread over its own group
    /// only, never into a price no level covers.
    #[test]
    fn profil_fin_sans_invention() {
        let dense = VenueBook {
            venue: "A".into(),
            bids: vec![(59_990.0, 1.0), (59_989.0, 2.0), (59_970.0, 1.0)],
            asks: vec![(60_010.0, 1.0), (60_011.0, 1.0)],
        };
        let d = profile(std::slice::from_ref(&dense), 60_000.0, 5.0, 3.0);
        assert_eq!(d.asks.len(), 1, "levels 1 $ apart: summed in their 5 $ bucket, nothing spread");
        assert_eq!(d.bids.len(), 2, "59 970 alone in its bucket, nothing between");
        assert!((d.total_coins(false) - 4.0).abs() < 1e-9);
        let groupe = VenueBook {
            venue: "HL".into(),
            bids: vec![(59_900.0, 2.0), (59_800.0, 2.0)],
            asks: vec![(60_100.0, 2.0), (60_200.0, 2.0)],
        };
        let d = profile(std::slice::from_ref(&groupe), 60_000.0, 5.0, 3.0);
        // each 100 $ group spread over 20 buckets of 5 $, the same coins in all
        assert_eq!(d.asks.len(), 40);
        assert!((d.total_coins(true) - 4.0).abs() < 1e-9);
        let (lo, hi) = (d.price(*d.asks.keys().next().unwrap()), d.price(*d.asks.keys().last().unwrap()));
        assert!(lo >= 60_050.0 && hi <= 60_250.0, "{lo} {hi}: never beyond the groups");
        // no level near 60 400: nothing there
        assert!(d.asks.keys().all(|k| d.price(*k) < 60_300.0));
    }

    const STEP: f64 = 100.0;

    /// The strict settings (exchanges and hold time are conditions): the wall tests below were written for them.
    fn gated() -> Cfg {
        Cfg { gate: true, ..Cfg::default() }
    }

    /// Three exchanges with 1 coin every 100 $ from 58 200 to 61 800 (the background), plus `walls`:
    /// (price, coins per exchange, number of exchanges that carry it).
    fn books(mid: f64, walls: &[(f64, f64, usize)]) -> Vec<VenueBook> {
        (0..3)
            .map(|i| {
                let mut b = VenueBook {
                    venue: format!("ex{i}"),
                    ..Default::default()
                };
                for k in 582..=618 {
                    let px = k as f64 * 100.0;
                    let mut sz = 1.0;
                    for &(wp, wc, n) in walls {
                        if wp == px && i < n {
                            sz += wc;
                        }
                    }
                    if px > mid {
                        b.asks.push((px, sz));
                    } else if px < mid {
                        b.bids.push((px, sz));
                    }
                }
                b
            })
            .collect()
    }

    fn depth(mid: f64, walls: &[(f64, f64, usize)]) -> Depth {
        aggregate(&books(mid, walls), mid, STEP, 3.0)
    }

    /// Runs `steps` updates 10 s apart at price `px` with `walls`, from `t`; gives the next time.
    fn run(tr: &mut Tracker, t: &mut i64, steps: usize, px: f64, walls: &[(f64, f64, usize)]) {
        for _ in 0..steps {
            *t += 10_000;
            tr.update(*t, &depth(px + 10.0, walls), px, px, px);
        }
    }

    fn ends(ev: &[Event]) -> Vec<Fin> {
        ev.iter()
            .filter_map(|e| match e.kind {
                Kind::End(f) => Some(f),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn somme_par_tranche_et_bourses() {
        assert_eq!(step_for(60_000.0), 100.0);
        assert_eq!(step_for(3_000.0), 5.0);
        assert_eq!(step_for(150.0), 0.2);
        let d = depth(60_050.0, &[(60_500.0, 20.0, 3)]);
        let c = d.asks[&605];
        assert_eq!(c.n_venues(), 3);
        assert!((c.coins - 63.0).abs() < 1e-9 && (c.usd - 63.0 * 60_500.0).abs() < 1e-3);
        assert!(!d.bids.contains_key(&605) && d.bids.contains_key(&600));
        // two prices in one bucket are summed; outside the range: dropped
        let b = VenueBook {
            venue: "a".into(),
            bids: vec![(59_960.0, 1.0), (60_040.0, 2.0), (50_000.0, 9.0)],
            asks: vec![],
        };
        let d = aggregate(&[b], 60_100.0, STEP, 3.0);
        assert_eq!((d.bids.len(), d.bids[&600].coins), (1, 3.0));
        assert_eq!(d.reaching(60_000.0), 1);
        assert_eq!(d.reaching(70_000.0), 0);
        assert!((d.total_coins(false) - 3.0).abs() < 1e-9);
    }

    #[test]
    fn un_mur_vu_sur_trois_bourses() {
        let d = depth(60_050.0, &[(60_500.0, 20.0, 3), (60_800.0, 60.0, 1)]);
        let q = d.qualifying(true, 4.0, 3);
        assert_eq!(q.iter().map(|x| x.0).collect::<Vec<_>>(), vec![605], "one exchange only: not a wall");
        // only one exchange reaches that deep: it is enough there
        let mut deep = books(60_050.0, &[]);
        deep[0].asks.push((61_850.0, 80.0));
        let d = aggregate(&deep, 60_050.0, STEP, 3.0);
        assert_eq!(d.qualifying(true, 4.0, 3).first().map(|x| x.0), Some(619));
    }

    #[test]
    fn montre_seulement_apres_trois_minutes() {
        let mut tr = Tracker::new(gated(), false);
        let mut t = 0;
        let w = [(60_500.0, 20.0, 3), (59_500.0, 20.0, 3)];
        run(&mut tr, &mut t, 17, 60_000.0, &w);
        assert!(tr.shown().is_empty(), "170 s: not yet");
        run(&mut tr, &mut t, 2, 60_000.0, &w);
        let s = tr.shown();
        assert_eq!(s.len(), 2);
        assert!(s.iter().any(|w| w.sell && w.price == 60_500.0) && s.iter().any(|w| !w.sell && w.price == 59_500.0));
        assert_eq!(tr.take_events().iter().filter(|e| e.kind == Kind::Show).count(), 2);
        // 3 per side at most, the biggest
        let many = [(60_500.0, 20.0, 3), (60_700.0, 30.0, 3), (60_900.0, 40.0, 3), (61_100.0, 50.0, 3)];
        let mut tr = Tracker::new(gated(), false);
        run(&mut tr, &mut t, 20, 60_000.0, &many);
        let mut p: Vec<f64> = tr.shown().iter().map(|w| w.price).collect();
        p.sort_by(f64::total_cmp);
        assert_eq!(p, vec![60_700.0, 60_900.0, 61_100.0]);
    }

    #[test]
    fn teste_puis_retire() {
        let mut tr = Tracker::new(gated(), false);
        let mut t = 0;
        let w = [(60_500.0, 20.0, 3)];
        run(&mut tr, &mut t, 20, 60_000.0, &w);
        // the price touches the wall's bucket and comes back down: TESTÉ, the wall holds
        run(&mut tr, &mut t, 1, 60_460.0, &w);
        run(&mut tr, &mut t, 1, 60_200.0, &w);
        assert_eq!(tr.shown()[0].tests, 1);
        assert!(tr.take_events().iter().any(|e| e.kind == Kind::Tested));
        // later it goes away with the price far: grey TESTÉ
        run(&mut tr, &mut t, 1, 59_600.0, &[]);
        assert_eq!(ends(&tr.take_events()), vec![Fin::Teste]);
        assert_eq!(tr.vue().words[0].fin.mot(), "TESTÉ");
        assert!(tr.shown().is_empty());
    }

    #[test]
    fn retire_loin_du_prix() {
        let mut tr = Tracker::new(gated(), false);
        let mut t = 0;
        run(&mut tr, &mut t, 20, 60_000.0, &[(61_500.0, 20.0, 3)]);
        assert_eq!(tr.shown().len(), 1);
        run(&mut tr, &mut t, 1, 60_000.0, &[]);
        assert_eq!(ends(&tr.take_events()), vec![Fin::Retire]);
        // a candidate never shown leaves nothing
        run(&mut tr, &mut t, 3, 60_000.0, &[(59_000.0, 20.0, 3)]);
        run(&mut tr, &mut t, 1, 60_000.0, &[]);
        assert!(ends(&tr.take_events()).is_empty());
    }

    #[test]
    fn spoof_deux_fois_pres_du_prix() {
        let mut tr = Tracker::new(gated(), false);
        let mut t = 0;
        let w = [(60_500.0, 20.0, 3)];
        for _ in 0..2 {
            run(&mut tr, &mut t, 20, 60_000.0, &w);
            // the price comes close (0.3 %), the wall is pulled before it is touched
            run(&mut tr, &mut t, 1, 60_300.0, &w);
            run(&mut tr, &mut t, 1, 60_300.0, &[]);
        }
        assert_eq!(ends(&tr.take_events()), vec![Fin::Retire, Fin::Spoof(2)], "one pull = RETIRÉ, two = SPOOF 2×");
        assert_eq!(tr.vue().words.len(), 1, "one grey word per price");
        assert_eq!(tr.vue().words[0].fin.mot(), "SPOOF 2×");
    }

    /// A recorded book sequence where a wall near the price flickers (there, gone, back, gone, back, gone) while the
    /// price does not move: SPOOF 2× then 3×, live and in the replay of the recorded events (the default settings).
    #[test]
    fn mur_qui_clignote_spoof_en_direct_et_en_relecture() {
        let mut tr = Tracker::new(Cfg::default(), false);
        let mut t = 0;
        let mut all: Vec<Event> = vec![];
        let w = [(60_200.0, 20.0, 3)];
        for _ in 0..3 {
            run(&mut tr, &mut t, 3, 60_000.0, &w);
            run(&mut tr, &mut t, 1, 60_000.0, &[]);
            all.extend(tr.take_events());
        }
        assert_eq!(ends(&all), vec![Fin::Retire, Fin::Spoof(2), Fin::Spoof(3)]);
        let live = tr.vue();
        assert_eq!(live.words.len(), 1);
        assert_eq!(live.words[0].fin.mot(), "SPOOF 3×");
        let rec = replay(&all, t, Cfg::default().ghost_ms);
        assert_eq!(rec.sorted(), live.comme_relue(), "the replay of the recording marks it SPOOF too");
    }

    /// 1-minute orderflow bars: (minute start s, price, market buys, market sells), rows of 10 $.
    fn flux(l: &[(i64, f64, f64, f64)]) -> Vec<crate::orderflow::FlowBar> {
        l.iter()
            .map(|&(t, p, a, v)| crate::orderflow::FlowBar {
                time: t,
                pas: 10.0,
                rangees: vec![((p / 10.0).floor() as i64, a, v)],
                complete: true,
            })
            .collect()
    }

    /// REMPLI: a sell wall eaten while the chart exchange's takers bought half of that exchange's own part of it or
    /// more at its price; pulled (no such trades): nothing.
    #[test]
    fn rempli_seulement_avec_des_trades() {
        let mut tr = Tracker::new(Cfg::default(), false);
        let mut t = 0;
        let w = [(60_500.0, 20.0, 3)];
        run(&mut tr, &mut t, 5, 60_000.0, &w);
        // the price goes through, the wall is gone: eaten
        t += 10_000;
        tr.update(t, &depth(60_610.0, &[]), 60_600.0, 60_000.0, 60_600.0);
        let ev = tr.take_events();
        let end = *ev.iter().find(|e| e.kind == Kind::End(Fin::Mange)).unwrap();
        // its size before the hit, not what was left
        let taille = end.usd;
        assert!(taille > 50.0 * 60_000.0, "{taille}");
        // the chart's exchange held a fifth of the summed wall
        let own = taille / 5.0;
        let part = PartBourse {
            own_usd: own,
            share: Some(0.3),
        };
        let coins = own / 60_500.0;
        let t_s = end.t_ms / 1000;
        // its takers bought 60 % of its own part at the wall: REMPLI (12 % of the summed wall: the old rule said no)
        let plein = flux(&[(t_s - 60, 60_495.0, coins * 0.6, 1.0)]);
        let r = rempli(&end, &plein, end.t_ms - 300_000, part).expect("filled by trades");
        assert_eq!((r.price, r.sell, r.usd), (60_500.0, true, taille));
        // only 20 % bought there, or the sells (wrong side), or trades elsewhere, or too long ago: a pull, nothing
        for l in [
            vec![(t_s - 60, 60_495.0, coins * 0.2, 0.0)],
            vec![(t_s - 60, 60_495.0, 0.0, coins)],
            vec![(t_s - 60, 61_000.0, coins, 0.0)],
            vec![(t_s - 900, 60_495.0, coins, 0.0)],
        ] {
            assert_eq!(rempli(&end, &flux(&l), end.t_ms - 300_000, part), None, "{l:?}");
        }
        // gone without any trade: nothing, whatever the share
        assert_eq!(rempli(&end, &[], end.t_ms - 300_000, part), None);
        // a wall RETIRÉ (never touched) is never REMPLI, whatever traded
        let mut tr = Tracker::new(Cfg::default(), false);
        let mut t = 0;
        run(&mut tr, &mut t, 5, 60_000.0, &[(61_500.0, 20.0, 3)]);
        run(&mut tr, &mut t, 1, 60_000.0, &[]);
        let pulled = tr.take_events().into_iter().find(|e| matches!(e.kind, Kind::End(_))).unwrap();
        assert_eq!(pulled.kind, Kind::End(Fin::Retire));
        assert_eq!(rempli(&pulled, &flux(&[(t / 1000 - 60, 61_495.0, 1e6, 0.0)]), 0, part), None);
        // recorded as a Filled event: the replay gives the mark back
        let ev = Event { kind: Kind::Filled, ..end };
        assert_eq!(replay(&[ev], end.t_ms, Cfg::default().ghost_ms).remplis, vec![r]);
    }

    /// The chart exchange holds nothing in the wall: the summed wall counts, scaled by its usual share (15 % unknown).
    #[test]
    fn rempli_sans_part_de_la_bourse() {
        let wall = 54e6;
        let sans = PartBourse { own_usd: 0.0, share: None };
        assert_eq!(sans.seuil(wall), 0.5 * 0.15 * wall);
        assert_eq!(
            PartBourse {
                own_usd: 0.0,
                share: Some(0.4)
            }
            .seuil(wall),
            0.5 * 0.4 * wall
        );
        assert_eq!(
            PartBourse {
                own_usd: 2e6,
                share: Some(0.4)
            }
            .seuil(wall),
            1e6
        );
        let end = Event {
            t_ms: 600_000,
            key: 600,
            step: STEP,
            sell: true,
            spot: false,
            usd: wall,
            kind: Kind::End(Fin::Mange),
            venues: 0,
        };
        // 9 % of the summed wall bought at it: above 7.5 %, REMPLI; 6 %: not
        let c = |x: f64| flux(&[(480, 59_995.0, x * wall / 60_000.0, 0.0)]);
        assert!(rempli(&end, &c(0.09), 300_000, sans).is_some());
        assert!(rempli(&end, &c(0.06), 300_000, sans).is_none());
    }

    /// The chart exchange's own part of a bucket, and its usual share of the summed books.
    #[test]
    fn part_de_la_bourse_dans_le_carnet() {
        let a = VenueBook {
            venue: "a".into(),
            bids: vec![(59_990.0, 1.0)],
            asks: vec![(60_010.0, 1.0), (60_480.0, 10.0), (60_530.0, 5.0), (61_000.0, 2.0)],
        };
        let b = VenueBook {
            venue: "b".into(),
            bids: vec![(59_990.0, 3.0)],
            asks: vec![(60_010.0, 3.0), (60_500.0, 30.0)],
        };
        // bucket 60 500 (100 $): 60 480 and 60 530 (rounded to 605), not 61 000; asks only
        assert_eq!(own_usd(&a, 60_500.0, STEP, true), 60_480.0 * 10.0 + 60_530.0 * 5.0);
        assert_eq!(own_usd(&a, 60_500.0, STEP, false), 0.0);
        // within 1 % (600 $): everything but 61 000
        let s = usual_share(&[a.clone(), b.clone()], 0, 60_000.0, 1.0).unwrap();
        let ua = 59_990.0 + 60_010.0 + 60_480.0 * 10.0 + 60_530.0 * 5.0;
        let ub = 59_990.0 * 3.0 + 60_010.0 * 3.0 + 60_500.0 * 30.0;
        assert!((s - ua / (ua + ub)).abs() < 1e-12, "{s}");
        assert_eq!(usual_share(&[a, b], 2, 60_000.0, 1.0), None);
    }

    #[test]
    fn mange_quand_le_prix_traverse() {
        let mut tr = Tracker::new(gated(), false);
        let mut t = 0;
        let w = [(60_500.0, 20.0, 3)];
        run(&mut tr, &mut t, 20, 60_000.0, &w);
        run(&mut tr, &mut t, 1, 60_300.0, &w);
        run(&mut tr, &mut t, 1, 60_300.0, &[]);
        // back, and this time the price goes through: eaten, and the bucket's pulls are forgotten (traded: no spoof)
        run(&mut tr, &mut t, 20, 60_000.0, &w);
        t += 10_000;
        tr.update(t, &depth(60_610.0, &[]), 60_600.0, 60_000.0, 60_600.0);
        assert_eq!(ends(&tr.take_events()), vec![Fin::Retire, Fin::Mange]);
        assert!(tr.pulls.is_empty());
    }

    #[test]
    fn sweep_meche_au_dela_cloture_en_deca() {
        let mut tr = Tracker::new(gated(), false);
        let mut t = 0;
        let w = [(60_500.0, 20.0, 3), (59_500.0, 20.0, 3)];
        run(&mut tr, &mut t, 20, 60_000.0, &w);
        tr.take_events();
        tr.bar_closed(t, 1_000, 60_620.0, 59_900.0, 60_100.0);
        tr.bar_closed(t, 2_000, 60_300.0, 59_380.0, 59_700.0);
        tr.bar_closed(t, 3_000, 60_700.0, 59_900.0, 60_650.0);
        let ev = tr.take_events();
        assert_eq!(
            ev.iter().filter(|e| e.kind == Kind::Sweep).count(),
            2,
            "the third one closed above: not a sweep"
        );
        assert_eq!(tr.sweeps, vec![(1_000, 60_500.0, true), (2_000, 59_500.0, false)]);
    }

    #[test]
    fn la_relecture_donne_les_memes_murs_et_jamais_le_futur() {
        let mut tr = Tracker::new(gated(), true);
        let mut t = 0;
        let mut all: Vec<Event> = vec![];
        let mut seen = vec![];
        let script: [(usize, f64, Vec<(f64, f64, usize)>); 7] = [
            (20, 60_000.0, vec![(60_500.0, 20.0, 3), (59_500.0, 25.0, 3)]),
            (5, 60_000.0, vec![(60_500.0, 30.0, 3), (59_500.0, 25.0, 3), (61_000.0, 22.0, 3)]),
            (20, 60_100.0, vec![(60_500.0, 30.0, 3), (59_500.0, 25.0, 3), (61_000.0, 22.0, 3)]),
            (1, 60_460.0, vec![(60_500.0, 30.0, 3), (59_500.0, 25.0, 3)]),
            (2, 60_000.0, vec![(60_500.0, 30.0, 3), (59_500.0, 25.0, 3)]),
            (1, 60_000.0, vec![(59_500.0, 25.0, 3)]),
            (3, 59_000.0, vec![]),
        ];
        for (n, px, w) in script {
            for _ in 0..n {
                run(&mut tr, &mut t, 1, px, &w);
                all.extend(tr.take_events());
                let live = tr.vue().comme_relue();
                assert_eq!(live, replay(&all, t, Cfg::default().ghost_ms).sorted(), "t = {t}");
                seen.push((t, live));
            }
        }
        assert!(all.iter().all(|e| e.spot));
        // read at an earlier clock with every event at hand: the same as live back then (nothing from later)
        for (t0, live) in seen.iter().step_by(7) {
            assert_eq!(&replay(&all, *t0, Cfg::default().ghost_ms).sorted(), live, "t = {t0}");
        }
        // grey words fade like live
        assert!(replay(&all, t + Cfg::default().ghost_ms, Cfg::default().ghost_ms).words.is_empty());
        // another price grid: the walls leave in the replay too
        let mut tr = Tracker::new(gated(), false);
        let mut t = 0;
        run(&mut tr, &mut t, 20, 60_000.0, &[(60_500.0, 20.0, 3)]);
        let mut ev = tr.take_events();
        t += 10_000;
        tr.update(t, &aggregate(&books(60_050.0, &[]), 60_050.0, 50.0, 3.0), 60_000.0, 60_000.0, 60_000.0);
        ev.extend(tr.take_events());
        assert!(replay(&ev, t, Cfg::default().ghost_ms).pills.is_empty());
    }
}

#[cfg(test)]
mod tests_contact {
    use super::*;

    /// The bucket holding the price is the book at the touch: never a new wall (it would always be the biggest). The
    /// next bucket is a real block (Jee, 21 Nov: refusing it left the chart empty).
    #[test]
    fn pas_de_mur_au_contact() {
        let d = |big: i64| {
            let mut b = VenueBook::default();
            for k in 582..=618 {
                let px = k as f64 * 100.0;
                let sz = if k == big { 50.0 } else { 1.0 };
                if px > 60_010.0 {
                    b.asks.push((px, sz));
                } else {
                    b.bids.push((px, sz));
                }
            }
            aggregate(&[b], 60_010.0, 100.0, 3.0)
        };
        let mut tr = Tracker::new(Cfg::default(), false);
        tr.update(10_000, &d(600), 60_000.0, 60_000.0, 60_000.0);
        assert!(tr.shown().iter().all(|w| w.price != 60_000.0), "{:?}", tr.shown());
        let mut tr = Tracker::new(Cfg::default(), false);
        tr.update(10_000, &d(601), 60_000.0, 60_000.0, 60_000.0);
        assert!(tr.shown().iter().any(|w| w.price == 60_100.0 && w.usd > 1e6));
    }

    /// Jee, 21 Nov: the biggest blocks are on the chart from the first photo, even on one exchange or just born; the
    /// exchanges that carry them ride along as a quality mark (and into the recording, so the replay has it too).
    #[test]
    fn les_plus_gros_blocs_des_la_premiere_photo() {
        let mut b = VenueBook::default();
        for k in 582..=618 {
            let px = k as f64 * 100.0;
            let sz = match k {
                605 => 30.0,
                607 => 20.0,
                610 => 40.0,
                612 => 10.0,
                595 => 25.0,
                590 => 15.0,
                _ => 1.0,
            };
            if px > 60_010.0 {
                b.asks.push((px, sz));
            } else {
                b.bids.push((px, sz));
            }
        }
        let d = aggregate(&[b], 60_010.0, 100.0, 3.0);
        let mut tr = Tracker::new(Cfg::default(), false);
        tr.update(10_000, &d, 60_000.0, 60_000.0, 60_000.0);
        let mut sells: Vec<f64> = tr.shown().iter().filter(|w| w.sell).map(|w| w.price).collect();
        sells.sort_by(f64::total_cmp);
        assert_eq!(sells, vec![60_500.0, 60_700.0, 61_000.0], "the 3 biggest sell blocks, at once");
        assert_eq!(tr.shown().iter().filter(|w| !w.sell).count(), 3, "always 3 per side");
        let ev = tr.take_events();
        // the real blocks carry their exchange; a 1-coin filler (third buy) carries none
        assert!(ev.iter().filter(|e| e.kind == Kind::Show && e.usd > 5e5).all(|e| e.venues == 1));
        let v = tr.vue();
        assert!(v.pills.iter().all(|p| p.since_ms == 10_000));
        assert_eq!(v.pills.iter().filter(|p| p.venues == 1).count(), 5);
        assert_eq!(replay(&ev, 10_000, Cfg::default().ghost_ms).sorted(), v.comme_relue());
        // blocks of about the same size (none 1.5 × the median): still the 3 biggest (spread, see below)
        let mut b = VenueBook::default();
        for k in 601..=606 {
            b.asks.push((k as f64 * 100.0, 10.0 + k as f64 - 600.0));
        }
        let flat = aggregate(&[b], 60_010.0, 100.0, 3.0);
        let mut tr2 = Tracker::new(Cfg::default(), false);
        tr2.update(10_000, &flat, 60_000.0, 60_000.0, 60_000.0);
        let mut p: Vec<f64> = tr2.shown().iter().map(|w| w.price).collect();
        p.sort_by(f64::total_cmp);
        assert_eq!(p, vec![60_200.0, 60_400.0, 60_600.0]);
        // neighbours are one block: the 3 walls are spread (not 60 400 / 60 500 / 60 600)
        let mut b = VenueBook::default();
        for k in 601..=615 {
            b.asks.push((k as f64 * 100.0, 10.0 + k as f64 - 600.0));
        }
        let mut tr3 = Tracker::new(Cfg::default(), false);
        tr3.update(10_000, &aggregate(&[b], 60_010.0, 100.0, 3.0), 60_000.0, 60_000.0, 60_000.0);
        let mut p: Vec<f64> = tr3.shown().iter().map(|w| w.price).collect();
        p.sort_by(f64::total_cmp);
        assert_eq!(p, vec![61_100.0, 61_300.0, 61_500.0]);
        // the strict settings still wait
        let mut tr = Tracker::new(
            Cfg {
                gate: true,
                min_venues: 1,
                ..Cfg::default()
            },
            false,
        );
        tr.update(10_000, &d, 60_000.0, 60_000.0, 60_000.0);
        assert!(tr.shown().is_empty());
        // an old recording (no exchange count) still reads
        let old: Event = serde_json::from_str(r#"{"t_ms":1,"key":605,"step":100.0,"sell":true,"spot":false,"usd":1.0,"kind":"Show"}"#).unwrap();
        assert_eq!(old.venues, 0);
    }
}

/// Jee, 21 Nov 20:23 UTC: « tous les murs + 1 de Binance ». The walls are chosen on the base books (Binance, OKX,
/// Hyperliquid), the pill says what every book read holds there, and Binance's biggest bucket per side is always shown.
#[cfg(test)]
mod tests_base {
    use super::*;

    const STEP: f64 = 100.0;
    const MID: f64 = 60_050.0;

    /// One book: 1 coin every 100 $ from 59 000 to 61 000, plus `walls` (price, coins).
    fn book(venue: &str, walls: &[(f64, f64)]) -> VenueBook {
        let mut b = VenueBook {
            venue: venue.into(),
            ..Default::default()
        };
        for k in 590..=610 {
            let px = k as f64 * STEP;
            let sz = 1.0 + walls.iter().filter(|w| w.0 == px).map(|w| w.1).sum::<f64>();
            if px > MID {
                b.asks.push((px, sz));
            } else {
                b.bids.push((px, sz));
            }
        }
        b
    }

    fn shown(books: &[VenueBook], cfg: Cfg) -> Vec<Wall> {
        let mut tr = Tracker::new(cfg, false);
        let d = aggregate(books, MID, STEP, 3.0);
        tr.update(0, &d, MID, MID, MID);
        tr.shown()
    }

    fn one_side(cfg: Cfg) -> Cfg {
        Cfg { per_side: 1, ..cfg }
    }

    #[test]
    fn choix_sur_la_somme_de_base_montant_de_tous() {
        // Bybit alone holds a huge block at 60 800; the base books hold a smaller one at 60 500
        let books = vec![
            book("Binance", &[]),
            book("OKX", &[(60_500.0, 10.0)]),
            book("Hyperliquid", &[(60_500.0, 10.0)]),
            book("Bybit", &[(60_800.0, 500.0), (60_500.0, 4.0)]),
        ];
        let cfg = one_side(Cfg { tag: false, ..Cfg::default() });
        let w = shown(&books, cfg);
        let sell: Vec<&Wall> = w.iter().filter(|w| w.sell).collect();
        assert_eq!(sell.len(), 1);
        assert_eq!(sell[0].price, 60_500.0, "a Bybit-only block creates no wall");
        // the pill's amount: every book read (Binance 1 + OKX 11 + HL 11 + Bybit 5 coins)
        assert!((sell[0].usd - 28.0 * 60_500.0).abs() < 1e-6, "{}", sell[0].usd);
    }

    /// Lot 3e S3: a wall already shown is replaced only by a clearly bigger one: a challenger 10 % bigger does not
    /// take its place, 30 % bigger does.
    #[test]
    fn mur_montre_pas_remplace_pour_peu() {
        let cfg = one_side(Cfg { tag: false, ..Cfg::default() });
        let base = |a: f64, b: f64| {
            vec![
                book("OKX", &[(60_500.0, a), (60_800.0, b)]),
                book("Hyperliquid", &[(60_500.0, a), (60_800.0, b)]),
            ]
        };
        for (defi, change) in [(1.10, false), (1.30, true)] {
            let mut tr = Tracker::new(cfg, false);
            tr.update(0, &aggregate(&base(50.0, 20.0), MID, STEP, 3.0), MID, MID, MID);
            assert_eq!(tr.shown().iter().find(|w| w.sell).map(|w| w.price), Some(60_500.0));
            // 60 800 grows to `defi` × 60 500 (the sizes count the 1-coin floor of each book)
            let b = 51.0 * defi - 1.0;
            tr.update(10_000, &aggregate(&base(50.0, b), MID, STEP, 3.0), MID, MID, MID);
            let p = tr.shown().iter().find(|w| w.sell).map(|w| w.price);
            assert_eq!(p, Some(if change { 60_800.0 } else { 60_500.0 }), "challenger × {defi}");
        }
    }

    /// Lot 3e S3: the tagged Binance wall stays on its bucket while Binance holds there at least 80 % of its biggest
    /// bucket of the side; below, it moves to the biggest.
    #[test]
    fn tag_binance_reste_sur_son_paquet() {
        let cfg = one_side(Cfg::default());
        let livres = |a: f64, b: f64| vec![book("Binance", &[(60_300.0, a), (60_600.0, b)]), book("OKX", &[(60_900.0, 80.0)])];
        for (b, garde) in [(11.0, true), (14.0, false)] {
            let mut tr = Tracker::new(cfg, false);
            tr.update(0, &aggregate(&livres(10.0, 5.0), MID, STEP, 3.0), MID, MID, MID);
            let tag = |tr: &Tracker| tr.shown().iter().find(|w| w.sell && w.tag).map(|w| w.price);
            assert_eq!(tag(&tr), Some(60_300.0));
            // 60 600 becomes Binance's biggest: 60 300 holds 11 / (b + 1) of it
            tr.update(10_000, &aggregate(&livres(10.0, b), MID, STEP, 3.0), MID, MID, MID);
            assert_eq!(tag(&tr), Some(if garde { 60_300.0 } else { 60_600.0 }), "60 600 at {b}");
        }
    }

    /// Lot 3e R2: each bucket keeps each book's own money (the tooltip's split by exchange), in the books' order.
    #[test]
    fn detail_par_bourse() {
        let books = vec![book("Binance", &[(60_500.0, 2.0)]), book("OKX", &[(60_500.0, 4.0)])];
        let d = aggregate(&books, MID, STEP, 3.0);
        assert_eq!(d.noms, ["Binance", "OKX"]);
        let c = d.asks[&605];
        assert!((c.par[0] - 3.0 * 60_500.0).abs() < 1e-6 && (c.par[1] - 5.0 * 60_500.0).abs() < 1e-6);
        assert!((c.par.iter().sum::<f64>() - c.usd).abs() < 1e-6);
    }

    /// Lot 3e 2b: a pill's money per exchange adds up exactly to the pill's amount (the split recorded with it).
    #[test]
    fn pilule_detail_egal_montant() {
        let books = vec![
            book("Binance", &[(60_500.0, 2.0)]),
            book("OKX", &[(60_500.0, 40.0)]),
            book("Hyperliquid", &[(60_500.0, 30.0)]),
        ];
        let mut tr = Tracker::new(one_side(Cfg::default()), false);
        tr.update(0, &aggregate(&books, MID, STEP, 3.0), MID, MID, MID);
        let v = tr.vue();
        let p = v.pills.iter().find(|p| p.sell && p.price == 60_500.0).unwrap();
        assert!((p.par.iter().sum::<f64>() - p.usd).abs() < 1e-6, "{:?} {}", p.par, p.usd);
        assert!(p.par[1] > p.par[2] && p.par[2] > p.par[0]);
    }

    #[test]
    fn repli_sans_carnet_de_base() {
        let books = vec![book("Bybit", &[(60_800.0, 50.0)]), book("Bitunix", &[(60_300.0, 20.0)])];
        let w = shown(&books, one_side(Cfg::default()));
        assert_eq!(w.iter().find(|w| w.sell).map(|w| w.price), Some(60_800.0), "the other books decide");
        assert!(w.iter().all(|w| !w.tag), "no Binance: no tag");
    }

    #[test]
    fn mur_binance_toujours_montre() {
        // Binance's biggest sell bucket is 60 200 (small next to the base walls at 60 700 and 60 900)
        let books = vec![
            book("Binance", &[(60_200.0, 3.0)]),
            book("OKX", &[(60_700.0, 40.0), (60_900.0, 30.0)]),
            book("Hyperliquid", &[(60_700.0, 40.0), (60_900.0, 30.0)]),
        ];
        let w = shown(&books, Cfg { per_side: 2, ..Cfg::default() });
        let sell: Vec<&Wall> = w.iter().filter(|w| w.sell).collect();
        assert_eq!(sell.len(), 3, "the 2 biggest + Binance's: {sell:?}");
        let t: Vec<&&Wall> = sell.iter().filter(|w| w.tag).collect();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].price, 60_200.0);
        // and on the buy side too, whatever its size
        assert_eq!(w.iter().filter(|w| !w.sell && w.tag).count(), 1);
        // already among the walls: tagged, no extra pill
        let books = vec![
            book("Binance", &[(60_700.0, 5.0)]),
            book("OKX", &[(60_700.0, 40.0), (60_900.0, 30.0)]),
            book("Hyperliquid", &[(60_700.0, 40.0), (60_900.0, 30.0)]),
        ];
        let w = shown(&books, Cfg { per_side: 2, ..Cfg::default() });
        let sell: Vec<&Wall> = w.iter().filter(|w| w.sell).collect();
        assert_eq!(sell.len(), 2);
        assert!(sell.iter().any(|w| w.tag && w.price == 60_700.0));
        // setting off: today's behaviour
        let w = shown(
            &books,
            Cfg {
                per_side: 2,
                tag: false,
                ..Cfg::default()
            },
        );
        assert!(w.iter().all(|w| !w.tag));
        assert_eq!(w.iter().filter(|w| !w.sell).count(), 2, "the 2 biggest, no extra pill");
        // Binance unread: no tag, nothing else changes
        let w = shown(&books[1..], Cfg { per_side: 2, ..Cfg::default() });
        assert!(w.iter().all(|w| !w.tag));
        assert_eq!(w.iter().filter(|w| w.sell).count(), 2);
    }

    #[test]
    fn mur_binance_suit_les_photos() {
        let mut tr = Tracker::new(Cfg { per_side: 1, ..Cfg::default() }, false);
        let base = |b: f64| {
            vec![
                book("Binance", &[(b, 3.0)]),
                book("OKX", &[(60_900.0, 40.0)]),
                book("Hyperliquid", &[(60_900.0, 40.0)]),
            ]
        };
        tr.update(0, &aggregate(&base(60_200.0), MID, STEP, 3.0), MID, MID, MID);
        assert!(tr.shown().iter().any(|w| w.tag && w.price == 60_200.0));
        tr.update(10_000, &aggregate(&base(60_400.0), MID, STEP, 3.0), MID, MID, MID);
        let tagged: Vec<f64> = tr.shown().iter().filter(|w| w.tag && w.sell).map(|w| w.price).collect();
        assert_eq!(tagged, vec![60_400.0], "the tag moves with Binance's biggest bucket");
        // spot trackers never tag
        let mut sp = Tracker::new(Cfg::default(), true);
        sp.update(0, &aggregate(&base(60_200.0), MID, STEP, 3.0), MID, MID, MID);
        assert!(sp.shown().iter().all(|w| !w.tag));
    }

    #[test]
    fn classe_de_couverture() {
        let a = VenueBook {
            venue: "Binance".into(),
            bids: vec![(59_900.0, 1.0)],
            asks: vec![(60_100.0, 1.0)],
        };
        let b = VenueBook {
            venue: "OKX".into(),
            bids: vec![(59_000.0, 1.0)],
            asks: vec![(61_000.0, 1.0)],
        };
        let d = aggregate(&[a, b], 60_000.0, 100.0, 3.0);
        assert_eq!(d.reach_mask(60_050.0), 0b11);
        assert_eq!(d.reach_mask(60_500.0), 0b10);
        assert_eq!(d.reach_mask(62_000.0), 0);
    }
}

#[cfg(test)]
mod tests_loin {
    use super::*;

    /// A Hyperliquid-like book grouped by 1 000 $ around 60 300.
    fn livre() -> VenueBook {
        VenueBook {
            venue: "Hyperliquid".into(),
            bids: vec![
                (60_000.0, 50.0),
                (59_000.0, 900.0),
                (58_000.0, 400.0),
                (57_000.0, 100.0),
                (55_000.0, 260.0),
                (53_000.0, 460.0),
                (50_000.0, 2_000.0),
            ],
            asks: vec![
                (61_000.0, 40.0),
                (62_000.0, 800.0),
                (63_000.0, 500.0),
                (65_000.0, 315.0),
                (66_000.0, 10.0),
                (80_000.0, 5_000.0),
            ],
        }
    }

    /// Lot loin-1: nothing inside the near range, a group overlapping it refused, nothing beyond `far_pct`; the two
    /// biggest per side, highest price first; usd = price × coins.
    #[test]
    fn filtre_rang_et_ordre() {
        let l = lointains(&livre(), 60_300.0, 3.0, 15.0, 2, &[], 7);
        let px: Vec<f64> = l.iter().map(|x| x.price).collect();
        // near range 58 491 .. 62 109: bids 59 000 and 58 000 (58 000 .. 59 000) overlap it, asks 62 000 and 63 000
        // (62 000 .. 63 000) too; 80 000 and 50 000 are beyond 15 % (51 255 .. 69 345); 57 000 is the smaller buy
        assert_eq!(px, vec![66_000.0, 65_000.0, 55_000.0, 53_000.0]);
        assert!(l.iter().all(|x| x.since_ms == 7));
        let v = l.iter().find(|x| x.price == 65_000.0).unwrap();
        assert!(v.sell && (v.usd - 65_000.0 * 315.0).abs() < 1e-6 && v.coins == 315.0);
        assert!(!l.iter().find(|x| x.price == 53_000.0).unwrap().sell);
        // a 1 % range (59 697 .. 60 903): 58 000 .. 59 000 lies beyond it, 59 000 .. 60 000 overlaps it
        let near = lointains(&livre(), 60_300.0, 1.0, 15.0, 6, &[], 0);
        assert!(near.iter().any(|x| x.price == 58_000.0 && !x.sell));
        assert!(!near.iter().any(|x| x.price == 59_000.0 || x.price == 60_000.0));
    }

    /// Hysteresis: a shown group beats a 5 % bigger challenger, not a 15 % bigger one; it keeps its `since_ms`.
    #[test]
    fn hysteresis_et_debut_garde() {
        let b = |a: f64, c: f64| VenueBook {
            venue: "Hyperliquid".into(),
            bids: vec![(60_000.0, 1.0), (55_000.0, a), (54_000.0, c)],
            asks: vec![],
        };
        let mid = 60_300.0;
        let avant = lointains(&b(100.0, 10.0), mid, 3.0, 15.0, 1, &[], 1);
        assert_eq!((avant[0].price, avant[0].since_ms), (55_000.0, 1));
        // 54 000 × q against 55 000 × 100: 5 % more money
        let q5 = 55_000.0 * 100.0 * 1.05 / 54_000.0;
        let l = lointains(&b(100.0, q5), mid, 3.0, 15.0, 1, &avant, 2);
        assert_eq!((l[0].price, l[0].since_ms), (55_000.0, 1), "kept");
        let q15 = 55_000.0 * 100.0 * 1.15 / 54_000.0;
        let l = lointains(&b(100.0, q15), mid, 3.0, 15.0, 1, &avant, 3);
        assert_eq!((l[0].price, l[0].since_ms), (54_000.0, 3), "replaced, a new start");
    }

    #[test]
    fn cas_vides() {
        assert!(lointains(&livre(), 60_300.0, 3.0, 3.0, 2, &[], 0).is_empty(), "far <= near");
        assert!(lointains(&livre(), 60_300.0, 3.0, 2.0, 2, &[], 0).is_empty());
        assert!(lointains(&livre(), 60_300.0, 3.0, 15.0, 0, &[], 0).is_empty(), "none per side");
        assert!(lointains(&livre(), 0.0, 3.0, 15.0, 2, &[], 0).is_empty());
        let seul = VenueBook {
            venue: "x".into(),
            bids: vec![(50_000.0, 1.0)],
            asks: vec![],
        };
        assert!(lointains(&seul, 60_000.0, 3.0, 30.0, 2, &[], 0).is_empty(), "one level: no group size");
    }
}

#[cfg(test)]
mod tests_fusion {
    use super::*;

    fn photo(bids: &[(f64, f64)], asks: &[(f64, f64)]) -> VenueBook {
        VenueBook {
            venue: "Binance".into(),
            bids: bids.to_vec(),
            asks: asks.to_vec(),
        }
    }

    fn qte(c: &CarnetFusionne, px: f64, mid: f64) -> f64 {
        let b = c.livre("Binance", mid);
        b.bids.iter().chain(&b.asks).find(|x| (x.0 - px).abs() < 1e-9).map_or(0.0, |x| x.1)
    }

    /// Lot loin-1 B: a newer event overrides a photo; an older one is ignored; quantity 0 removes; an older photo
    /// cannot bring a removed level back.
    #[test]
    fn evenements_et_photos_par_id() {
        let mid = 100.0;
        let mut c = CarnetFusionne::default();
        c.photo(10, &photo(&[(99.9, 5.0), (99.8, 1.0)], &[(100.1, 2.0)]), mid, 4.0);
        assert_eq!(qte(&c, 99.9, mid), 5.0);
        c.evenement(11, 12, &[(99.9, 7.0)], &[]);
        assert_eq!(qte(&c, 99.9, mid), 7.0, "newer event");
        // an event older than the photo (u = 9) changes nothing
        let mut d = CarnetFusionne::default();
        d.photo(10, &photo(&[(99.9, 5.0)], &[(100.1, 2.0)]), mid, 4.0);
        d.evenement(8, 9, &[(99.9, 1.0)], &[]);
        assert_eq!(qte(&d, 99.9, mid), 5.0, "older event ignored");
        // quantity 0 removes; an older photo listing it does not bring it back
        c.evenement(12, 13, &[(99.8, 0.0)], &[]);
        assert_eq!(qte(&c, 99.8, mid), 0.0);
        c.photo(11, &photo(&[(99.9, 5.0), (99.8, 1.0)], &[(100.1, 2.0)]), mid, 4.0);
        assert_eq!(qte(&c, 99.8, mid), 0.0, "older photo");
        assert_eq!(qte(&c, 99.9, mid), 7.0, "the newer event stays");
        // a level removed that we never held: normal, nothing happens
        c.evenement(13, 14, &[(90.0, 0.0)], &[]);
        assert_eq!(c.livre("Binance", mid).bids.len(), 1);
    }

    /// A photo removes a held level it does not list inside its span and leaves the levels outside; a gap forgets the
    /// outside only; pruning beyond the kept range; tombstones dropped.
    #[test]
    fn portee_trou_et_elagage() {
        let mid = 100.0;
        let mut c = CarnetFusionne::default();
        c.photo(10, &photo(&[(99.9, 5.0), (99.8, 1.0)], &[(100.1, 2.0), (100.2, 3.0)]), mid, 4.0);
        // the stream tells a far level (beyond the photo) and a level inside
        c.evenement(11, 12, &[(98.0, 40.0), (99.85, 2.0)], &[(102.0, 9.0)]);
        assert_eq!(c.hors_photo(), 2);
        // next photo, newer: 99.85 inside its span but not listed → gone; 98 and 102 outside → kept
        c.photo(20, &photo(&[(99.9, 5.0), (99.8, 1.0)], &[(100.1, 2.0), (100.2, 3.0)]), mid, 4.0);
        assert_eq!(qte(&c, 99.85, mid), 0.0);
        assert_eq!((qte(&c, 98.0, mid), qte(&c, 102.0, mid)), (40.0, 9.0));
        // the tombstone of 99.85 (id 20 ≤ 20) is dropped
        assert_eq!(c.bids.len() + c.asks.len(), 6);
        assert_eq!(c.ecarts(20, &photo(&[(99.9, 5.0), (99.8, 1.0)], &[(100.1, 2.0), (100.2, 3.0)])), (4, 0));
        // a gap (pu ≠ the previous u) forgets the outside only
        assert!(!c.evenement(12, 21, &[], &[]), "continuity: 12 then 21 with pu 12");
        assert!(c.evenement(99, 30, &[], &[]), "pu 99 ≠ 21: a gap");
        assert_eq!((qte(&c, 98.0, mid), qte(&c, 102.0, mid)), (0.0, 0.0));
        assert_eq!(qte(&c, 99.9, mid), 5.0, "the inside stays");
        // pruning: beyond ±4 % of the mid a level leaves at the next photo
        c.evenement(30, 31, &[(95.0, 3.0), (97.0, 3.0)], &[]);
        c.photo(40, &photo(&[(99.9, 5.0)], &[(100.1, 2.0)]), mid, 4.0);
        assert_eq!((qte(&c, 95.0, mid), qte(&c, 97.0, mid)), (0.0, 3.0));
        // no photo yet: a gap forgets everything
        let mut d = CarnetFusionne::default();
        d.evenement(1, 2, &[(99.0, 1.0)], &[]);
        d.trou();
        assert!(d.livre("Binance", mid).bids.is_empty());
    }

    /// The book handed to Argos Murs: bids below the mid best first, asks above, nothing at 0.
    #[test]
    fn livre_cote_par_cote() {
        let mut c = CarnetFusionne::default();
        c.photo(10, &photo(&[(99.9, 5.0), (99.0, 1.0)], &[(100.1, 2.0)]), 100.0, 4.0);
        c.evenement(11, 12, &[(99.5, 0.0)], &[(103.0, 4.0)]);
        let b = c.livre("Binance", 100.0);
        assert_eq!(b.bids, vec![(99.9, 5.0), (99.0, 1.0)]);
        assert_eq!(b.asks, vec![(100.1, 2.0), (103.0, 4.0)]);
        assert_eq!(b.venue, "Binance");
    }
}
