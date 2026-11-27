//! 🏷️ Tickers écrits à la façon TradingView : `PLATEFORME:PAIRE` (spot), `PLATEFORME:PAIRE.P` (perpétuel),
//! `PAIREBTC` (token contre Bitcoin) et la formule `XRPUSDT/BTCUSDT`. Voir le guide
//! « documents/texte/guide-tickers.txt ».
//!
//! Le lecteur ne touche pas au catalogue : il dit quel exchange, quelle base, quelle devise et quel type de
//! contrat le ticker demande. L'app cherche ensuite le vrai marché dans son catalogue.

use crate::Exchange;
use argos_core::catalogue::{SymboleId, TypeMarche};

/// Préfixes TradingView reconnus mais pas encore offerts par Argos (message clair au lieu d'un symbole cassé).
const NOT_YET: [&str; 15] = [
    "CME",
    "CME_MINI",
    "INDEX",
    "CRYPTO",
    "BNC",
    "HTX",
    "GEMINI",
    "DERIBIT",
    "NASDAQ",
    "NYSE",
    "AMEX",
    "TVC",
    "FX",
    "FRED",
    "ECONOMICS",
];

/// Devises de cotation reconnues à la fin d'une paire (les plus longues d'abord).
const QUOTES: [&str; 7] = ["USDT", "USDC", "BUSD", "PERP", "USD", "EUR", "BTC"];

/// Ce que demande un ticker TradingView.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TvTicker {
    /// `None` : pas de préfixe (« XRPBTC ») ; l'app choisit l'exchange.
    pub exchange: Option<Exchange>,
    /// Actif de base en majuscules (« BTC », « XYZ:NVDA » reste tel quel pour Hyperliquid).
    pub base: String,
    /// Devise de cotation (« USDT », « USD », « BTC »…) ; `None` si le ticker ne la donne pas (« HL:BTC »).
    pub quote: Option<String>,
    /// `.P` : perpétuel.
    pub perp: bool,
}

impl TvTicker {
    /// Paire contre Bitcoin (« XRPBTC ») : l'app prend la vraie paire si elle existe, sinon la calcule.
    pub fn is_btc_pair(&self) -> bool {
        self.quote.as_deref() == Some("BTC") && self.base != "BTC"
    }
}

/// Résultat de la lecture d'un texte tapé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tv {
    /// Pas un ticker TradingView : recherche normale (« btc », « xyz:NVDA »).
    Plain,
    /// Un ticker reconnu.
    Ticker(TvTicker),
    /// Formule `A/B` (« XRPUSDT/BTCUSDT ») : le ratio de deux marchés.
    Ratio(TvTicker, TvTicker),
    /// Ticker reconnu mais pas géré : la raison, à afficher telle quelle.
    NotYet(String),
}

/// Lit un texte tapé dans la recherche (majuscules ou minuscules).
pub fn parse(input: &str) -> Tv {
    let s = input.trim().to_uppercase().replace(' ', "");
    if s.is_empty() {
        return Tv::Plain;
    }
    // formule « A/B » (pas « BTC/USD », qui est une paire Bitstamp) : deux paires complètes
    if let Some((a, b)) = s.split_once('/') {
        if let (Tv::Ticker(x), Tv::Ticker(y)) = (one(a, false), one(b, false)) {
            return Tv::Ratio(x, y);
        }
    }
    one(&s, true)
}

/// Un seul ticker. `strict` : sans préfixe, seuls les tickers clairement TradingView sont pris.
fn one(s: &str, strict: bool) -> Tv {
    if s.is_empty() {
        return Tv::Plain;
    }
    let (prefix, rest) = match s.split_once(':') {
        Some((p, r)) => (Some(p), r),
        None => (None, s),
    };
    if let Some(p) = prefix {
        if p.starts_with("CRYPTOCAP") {
            return Tv::Plain; // les séries CRYPTOCAP: sont cherchées à part (CoinMarketCap)
        }
        if NOT_YET.contains(&p) {
            return Tv::NotYet(format!("{p}: n'est pas encore géré dans Argos (exchanges offerts : {})", offered()));
        }
    }
    let exchange = match prefix {
        Some("HL" | "HYPERLIQUID") => Some(Exchange::Hyperliquid),
        Some("GATEIO" | "GATE") => Some(Exchange::Gate),
        Some(p) => match Exchange::ALL.into_iter().find(|e| e.id().eq_ignore_ascii_case(p)) {
            Some(e) => Some(e),
            // préfixe inconnu (ex. « xyz:NVDA », un marché HIP-3) : recherche normale
            None => return Tv::Plain,
        },
        None => None,
    };
    let (body, perp) = match rest.strip_suffix(".P") {
        Some(b) => (b, true),
        None => (rest, false),
    };
    if body.is_empty() {
        return Tv::Plain;
    }
    // Hyperliquid : « HL:BTC », « HYPERLIQUID:BTCUSDC.P », « HL:XYZ:NVDA »
    if exchange == Some(Exchange::Hyperliquid) {
        let (base, quote) = split_pair(body).map_or((body.to_string(), None), |(b, q)| (b, Some(q)));
        if body.contains(':') {
            return Tv::Ticker(TvTicker {
                exchange,
                base: body.to_string(),
                quote: None,
                perp: true,
            });
        }
        return Tv::Ticker(TvTicker {
            exchange,
            base,
            quote,
            perp: true,
        });
    }
    match split_pair(body) {
        Some((base, quote)) => {
            let mut t = TvTicker {
                exchange,
                base,
                quote: Some(quote),
                perp,
            };
            // 🪙 sans « .P » sur un exchange qui a aussi du spot (Binance, OKX, Bybit, Bitget) : le spot
            // (pas pour une paire contre BTC, un ratio de perps, ni pour « …-SWAP », qui est le perp d'OKX)
            if !perp && !t.is_btc_pair() && !body.to_uppercase().ends_with("SWAP") {
                if let Some(s) = t.exchange.and_then(|e| e.spot_of()) {
                    t.exchange = Some(s);
                }
            }
            if exchange == Some(Exchange::Bitstamp) && perp {
                return Tv::NotYet("Bitstamp n'a que du spot : enlève le « .P »".into());
            }
            // sans préfixe, seuls les tickers clairement TradingView sont pris (« XRPBTC », « BTCUSDT.P ») ;
            // « BTC » ou « BTCUSDT » restent une recherche normale du catalogue
            if strict && exchange.is_none() && !perp && !t.is_btc_pair() {
                return Tv::Plain;
            }
            Tv::Ticker(t)
        }
        None if exchange.is_some() => Tv::NotYet(format!("paire « {body} » illisible : écris par exemple {}", example(exchange))),
        None => Tv::Plain,
    }
}

/// Sépare « BTCUSDT », « BTC-USDT », « BTC/USD », « BTC-USDT-SWAP » en (base, devise).
fn split_pair(body: &str) -> Option<(String, String)> {
    let b = body.trim_end_matches("-SWAP");
    if let Some((x, y)) = b.split_once(['-', '/', '_']) {
        return (!x.is_empty() && QUOTES.contains(&y)).then(|| (x.to_string(), y.to_string()));
    }
    QUOTES.iter().find_map(|q| {
        let base = b.strip_suffix(q)?;
        (!base.is_empty() && base.chars().all(|c| c.is_ascii_alphanumeric())).then(|| (base.to_string(), q.to_string()))
    })
}

/// 🔔 A symbol's base and quote whatever the exchange writes: « SOLUSDT », « BTC-USDT », « BTC_USDT », « BTC/USDT »,
/// « BTCUSDC.P », « BTC-USDT-SWAP » → (« SOL », « USDT »)…; `None` when no known quote ends it.
pub fn pair_parts(sym: &str) -> Option<(String, String)> {
    let s = sym.trim().to_uppercase();
    split_pair(s.trim_end_matches(".P"))
}

fn offered() -> String {
    Exchange::ALL.iter().map(|e| e.name().to_uppercase()).collect::<Vec<_>>().join(", ")
}

fn example(ex: Option<Exchange>) -> String {
    match ex {
        Some(Exchange::Bitstamp) => "BITSTAMP:BTCUSD".into(),
        Some(e) => format!("{}:BTCUSDT.P", e.name().to_uppercase()),
        None => "BINANCE:BTCUSDT.P".into(),
    }
}

/// Nom TradingView d'un marché Argos : « BINANCE:BTCUSDT.P », « BITSTAMP:BTCUSD », « HYPERLIQUID:BTCUSDC.P ».
pub fn name(ex: Exchange, base: &str) -> String {
    let prefix = match ex {
        Exchange::Hyperliquid => "HYPERLIQUID".to_string(),
        Exchange::Gate => "GATEIO".to_string(),
        e => e.parent().name().to_uppercase(),
    };
    if ex == Exchange::Hyperliquid && base.contains(':') {
        return format!("{prefix}:{base}"); // HIP-3 : « HYPERLIQUID:xyz:NVDA »
    }
    // ₿ a real pair against BTC (« XRP/BTC ») : « BINANCE:XRPBTC »
    if crate::paire::contre_btc(base) {
        return format!("{prefix}:{}", crate::paire::symbole(base, "", "BTC"));
    }
    let b = base.to_uppercase();
    match ex {
        Exchange::Bitstamp | Exchange::Coinbase | Exchange::Kraken | Exchange::BitfinexSpot => format!("{prefix}:{b}USD"),
        Exchange::Kucoin | Exchange::BinanceSpot | Exchange::OkxSpot | Exchange::BybitSpot | Exchange::BitgetSpot => {
            format!("{prefix}:{b}USDT")
        }
        Exchange::Hyperliquid => format!("{prefix}:{b}USDC.P"),
        _ => format!("{prefix}:{b}USDT.P"),
    }
}

/// Nom d'une paire calculée : « XRPBTC (calculée, approximative) » (revue R&D, point 5 : jamais une donnée d'exchange).
pub fn ratio_name(base: &str, quote: &str) -> String {
    format!("{}{} (calculée, approximative)", base.to_uppercase(), quote.to_uppercase())
}

/// Exchange par défaut d'une paire calculée sans préfixe (« XRPBTC ») : le plus gros volume (voir le guide).
pub const RATIO_DEFAULT: Exchange = Exchange::Binance;

/// Identifiant d'une paire calculée dans une watchlist : « ratio:binance:XRP:BTC ».
pub fn ratio_id(ex: Exchange, base: &str, quote: &str) -> String {
    format!("ratio:{}:{}:{}", ex.id(), base.to_uppercase(), quote.to_uppercase())
}

/// Relit `ratio_id` : (exchange, base, devise).
pub fn parse_ratio_id(id: &str) -> Option<(Exchange, String, String)> {
    let mut it = id.strip_prefix("ratio:")?.splitn(3, ':');
    let ex = Exchange::parse(it.next()?)?;
    let (base, quote) = (it.next()?, it.next()?);
    (!base.is_empty() && !quote.is_empty()).then(|| (ex, base.to_string(), quote.to_string()))
}

/// 🧮 Paire calculée bougie par bougie : `a ÷ b` (ex. XRP en USDT ÷ BTC en USDT = XRP en BTC).
/// Seules les bougies présentes dans les deux séries (même heure d'ouverture) sont gardées.
/// Ouverture = a.open ÷ b.open, clôture = a.close ÷ b.close ; le haut et le bas viennent de a.high ÷ b.high et
/// a.low ÷ b.low, élargis pour toujours contenir l'ouverture et la clôture (le vrai haut du ratio n'est pas connu
/// à partir de deux bougies séparées). Pas de volume.
pub fn ratio_bars(a: &[argos_core::Bar], b: &[argos_core::Bar]) -> Vec<argos_core::Bar> {
    let by_time: std::collections::HashMap<i64, &argos_core::Bar> = b.iter().map(|x| (x.time, x)).collect();
    let div = |x: f64, y: f64| if y > 0.0 && x.is_finite() && y.is_finite() { Some(x / y) } else { None };
    let mut out: Vec<argos_core::Bar> = a
        .iter()
        .filter_map(|x| {
            let y = by_time.get(&x.time)?;
            let (open, close) = (div(x.open, y.open)?, div(x.close, y.close)?);
            let (hi, lo) = (div(x.high, y.high)?, div(x.low, y.low)?);
            Some(argos_core::Bar {
                time: x.time,
                open,
                high: hi.max(lo).max(open).max(close),
                low: lo.min(hi).min(open).min(close),
                close,
                volume: 0.0,
            })
        })
        .collect();
    out.sort_by_key(|x| x.time);
    out
}

/// 🗂️ Identifiant du catalogue synchronisé d'un marché Argos : le nom TradingView (`name`), avec le type de marché de
/// l'exchange (« BINANCE:BTCUSDT.P », « BITSTAMP:BTCUSD », « HYPERLIQUID:xyz:NVDA.P » : un marché HIP-3 est un perp).
pub fn symbole_id(ex: Exchange, base: &str) -> SymboleId {
    let n = name(ex, base);
    let (source, paire) = n.split_once(':').unwrap_or(("", n.as_str()));
    SymboleId {
        source: source.to_string(),
        paire: paire.trim_end_matches(".P").to_string(),
        marche: if ex.market_kind() == "Spot" { TypeMarche::Spot } else { TypeMarche::Perp },
    }
}

/// 🔄 Ancien identifiant de watchlist → identifiant du catalogue, pour toutes les sources de l'app : les exchanges
/// (« hl:BTC », « binance:BTCUSDT », « binance-spot:BTCUSDT », « bitstamp:btcusd »), OANDA (« oanda:XAU_USD » →
/// OANDA:XAUUSD) et CoinMarketCap (« cmc:… », « cryptocap:… » → CRYPTOCAP:…). `None` : source inconnue ou paire
/// illisible (la reprise garde alors le symbole tel quel, « à vérifier »).
pub fn depuis_ancien_id(id: &str) -> Option<SymboleId> {
    let (ex, sym) = argos_core::instrument::split_id(id)?;
    let autre = |source: &str, paire: String| SymboleId {
        source: source.to_string(),
        paire,
        marche: TypeMarche::Spot,
    };
    match ex {
        "oanda" => return Some(autre("OANDA", crate::oanda::label(id).unwrap_or_else(|| sym.replace('_', "")))),
        "cmc" | "cryptocap" => return Some(autre("CRYPTOCAP", sym.to_uppercase())),
        _ => {}
    }
    let ex = Exchange::parse(ex)?;
    let base = if ex == Exchange::Hyperliquid {
        sym.to_string()
    } else {
        // (₿ « binance-spot:XRPBTC » keeps its quote: « BINANCE:XRPBTC »)
        let (b, q) = split_pair(&sym.to_uppercase())?;
        crate::paire::base(&b, &q)
    };
    Some(symbole_id(ex, &base))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tk(ex: Option<Exchange>, base: &str, quote: Option<&str>, perp: bool) -> Tv {
        Tv::Ticker(TvTicker {
            exchange: ex,
            base: base.into(),
            quote: quote.map(Into::into),
            perp,
        })
    }

    #[test]
    fn perpetuels_et_spot() {
        assert_eq!(parse("BINANCE:BTCUSDT.P"), tk(Some(Exchange::Binance), "BTC", Some("USDT"), true));
        assert_eq!(parse("binance:btcusdt.p"), tk(Some(Exchange::Binance), "BTC", Some("USDT"), true), "minuscules");
        assert_eq!(parse("BYBIT:ETHUSDT.P"), tk(Some(Exchange::Bybit), "ETH", Some("USDT"), true));
        assert_eq!(parse("OKX:BTC-USDT-SWAP"), tk(Some(Exchange::Okx), "BTC", Some("USDT"), false));
        assert_eq!(parse("BLOFIN:BTCUSDT.P"), tk(Some(Exchange::Blofin), "BTC", Some("USDT"), true));
        assert_eq!(parse("BINANCE:BTCUSDT"), tk(Some(Exchange::BinanceSpot), "BTC", Some("USDT"), false), "spot");
        assert_eq!(parse("OKX:BTCUSDT"), tk(Some(Exchange::OkxSpot), "BTC", Some("USDT"), false), "spot OKX");
        assert_eq!(parse("BYBIT:ETHUSDT"), tk(Some(Exchange::BybitSpot), "ETH", Some("USDT"), false));
        assert_eq!(parse("BITGET:SOLUSDT"), tk(Some(Exchange::BitgetSpot), "SOL", Some("USDT"), false));
        assert_eq!(parse("BITGET:SOLUSDT.P"), tk(Some(Exchange::Bitget), "SOL", Some("USDT"), true), "perp Bitget");
        assert_eq!(name(Exchange::BinanceSpot, "BTC"), "BINANCE:BTCUSDT", "nom TradingView du spot, sans .P");
        assert_eq!(parse("BITSTAMP:BTCUSD"), tk(Some(Exchange::Bitstamp), "BTC", Some("USD"), false));
        assert_eq!(parse("WEEX:SOLUSDT.P"), tk(Some(Exchange::Weex), "SOL", Some("USDT"), true));
        assert_eq!(parse("BITUNIX:SOLUSDT.P"), tk(Some(Exchange::Bitunix), "SOL", Some("USDT"), true));
    }

    #[test]
    fn hyperliquid() {
        assert_eq!(parse("HL:BTC"), tk(Some(Exchange::Hyperliquid), "BTC", None, true));
        assert_eq!(parse("HYPERLIQUID:BTCUSDC.P"), tk(Some(Exchange::Hyperliquid), "BTC", Some("USDC"), true));
        assert_eq!(parse("hl:xyz:NVDA"), tk(Some(Exchange::Hyperliquid), "XYZ:NVDA", None, true));
    }

    #[test]
    fn recherche_normale_gardee() {
        for q in ["", "btc", "BTCUSDT", "xyz:NVDA", "CRYPTOCAP:USDT.D", "eth", "BTC/USD"] {
            assert_eq!(parse(q), Tv::Plain, "{q}");
        }
    }

    #[test]
    fn prefixes_pas_encore_geres() {
        for q in ["CME:BTC1!", "INDEX:BTCUSD", "BNC:BLX", "HTX:BTCUSDT"] {
            let Tv::NotYet(m) = parse(q) else {
                panic!("{q} devrait être « pas encore géré »")
            };
            assert!(m.contains("pas encore géré"), "{m}");
        }
        assert!(matches!(parse("BITSTAMP:BTCUSD.P"), Tv::NotYet(_)), "Bitstamp n'a pas de perp");
        assert!(matches!(parse("BINANCE:???"), Tv::NotYet(_)), "paire illisible");
    }

    #[test]
    fn paires_contre_bitcoin() {
        let Tv::Ticker(t) = parse("XRPBTC") else { panic!() };
        assert!(t.is_btc_pair() && t.exchange.is_none() && t.base == "XRP");
        let Tv::Ticker(t) = parse("BINANCE:HYPEBTC") else { panic!() };
        assert!(t.is_btc_pair() && t.exchange == Some(Exchange::Binance) && t.base == "HYPE");
        // Bitfinex is offered now (no longer « pas encore géré »)
        let Tv::Ticker(t) = parse("BITFINEX:XRPBTC") else { panic!() };
        assert!(t.is_btc_pair() && t.exchange == Some(Exchange::Bitfinex) && t.base == "XRP");
        assert_eq!(name(Exchange::BitfinexSpot, "XRP/BTC"), "BITFINEX:XRPBTC");
        assert_eq!(name(Exchange::BitfinexSpot, "XRP"), "BITFINEX:XRPUSD");
        assert_eq!(name(Exchange::Bitfinex, "XRP"), "BITFINEX:XRPUSDT.P");
        assert_eq!(
            parse("XRPUSDT/BTCUSDT"),
            Tv::Ratio(
                TvTicker {
                    exchange: None,
                    base: "XRP".into(),
                    quote: Some("USDT".into()),
                    perp: false
                },
                TvTicker {
                    exchange: None,
                    base: "BTC".into(),
                    quote: Some("USDT".into()),
                    perp: false
                },
            )
        );
        let Tv::Ratio(a, _) = parse("BINANCE:XRPUSDT.P/BINANCE:BTCUSDT.P") else {
            panic!()
        };
        assert_eq!(a.exchange, Some(Exchange::Binance));
    }

    #[test]
    fn noms_tradingview() {
        assert_eq!(name(Exchange::Binance, "BTC"), "BINANCE:BTCUSDT.P");
        assert_eq!(name(Exchange::Okx, "eth"), "OKX:ETHUSDT.P");
        assert_eq!(name(Exchange::Hyperliquid, "BTC"), "HYPERLIQUID:BTCUSDC.P");
        assert_eq!(name(Exchange::Hyperliquid, "xyz:NVDA"), "HYPERLIQUID:xyz:NVDA");
        assert_eq!(name(Exchange::Bitstamp, "BTC"), "BITSTAMP:BTCUSD");
        // ₿ real pairs against BTC
        assert_eq!(name(Exchange::BinanceSpot, "XRP/BTC"), "BINANCE:XRPBTC");
        assert_eq!(name(Exchange::Kraken, "XRP/BTC"), "KRAKEN:XRPBTC");
        assert_eq!(Exchange::OkxSpot.ticker("XRP/BTC"), "XRP-BTC");
        assert_eq!(Exchange::Kraken.ticker("XRP/BTC"), "XRP/BTC");
        assert_eq!(Exchange::BinanceSpot.ticker("XRP/BTC"), "XRPBTC");
        assert_eq!(Exchange::BinanceSpot.ticker("XRP"), "XRPUSDT", "unchanged");
        assert_eq!(ratio_name("xrp", "btc"), "XRPBTC (calculée, approximative)");
    }

    fn bar(time: i64, o: f64, h: f64, l: f64, c: f64) -> argos_core::Bar {
        argos_core::Bar {
            time,
            open: o,
            high: h,
            low: l,
            close: c,
            volume: 9.0,
        }
    }

    #[test]
    fn ratio_bougie_par_bougie() {
        let xrp = [bar(0, 1.0, 1.2, 0.9, 1.1), bar(60, 1.1, 1.1, 1.0, 1.05), bar(120, 2.0, 2.0, 2.0, 2.0)];
        let btc = [bar(0, 50_000.0, 52_000.0, 49_000.0, 51_000.0), bar(60, 51_000.0, 51_500.0, 50_000.0, 50_500.0)];
        let r = ratio_bars(&xrp, &btc);
        assert_eq!(r.len(), 2, "la bougie de 120 n'existe que d'un côté : enlevée");
        assert!((r[0].open - 0.00002).abs() < 1e-12);
        assert!((r[0].close - 1.1 / 51_000.0).abs() < 1e-12);
        assert!((r[0].high - 1.2 / 52_000.0).abs() < 1e-12);
        assert!((r[0].low - 0.9 / 49_000.0).abs() < 1e-12);
        assert_eq!(r[0].volume, 0.0, "pas de volume");
        for x in &r {
            assert!(
                x.high >= x.open.max(x.close) && x.low <= x.open.min(x.close),
                "haut et bas contiennent tout : {x:?}"
            );
        }
        assert!(ratio_bars(&xrp, &[bar(0, 0.0, 0.0, 0.0, 0.0)]).is_empty(), "division par zéro ignorée");
        assert!(ratio_bars(&[], &btc).is_empty());
    }

    #[test]
    fn identifiant_de_paire_calculee() {
        let id = ratio_id(Exchange::Binance, "xrp", "btc");
        assert_eq!(id, "ratio:binance:XRP:BTC");
        assert_eq!(parse_ratio_id(&id), Some((Exchange::Binance, "XRP".into(), "BTC".into())));
        assert_eq!(parse_ratio_id("binance:XRPUSDT"), None);
        assert_eq!(parse_ratio_id("ratio:inconnu:XRP:BTC"), None);
    }

    #[test]
    fn anciens_identifiants_vers_le_catalogue() {
        for (ancien, nouveau) in [
            ("hl:BTC", "HYPERLIQUID:BTCUSDC.P"),
            ("hl:xyz:NVDA", "HYPERLIQUID:xyz:NVDA.P"),
            ("binance:BTCUSDT", "BINANCE:BTCUSDT.P"),
            ("binance-spot:BTCUSDT", "BINANCE:BTCUSDT"),
            ("bitstamp:btcusd", "BITSTAMP:BTCUSD"),
            ("binance-spot:XRPBTC", "BINANCE:XRPBTC"),
        ] {
            assert_eq!(depuis_ancien_id(ancien).map(|s| s.to_string()).as_deref(), Some(nouveau), "{ancien}");
        }
        assert_eq!(depuis_ancien_id("oanda:XAU_USD").map(|s| s.to_string()).as_deref(), Some("OANDA:XAUUSD"));
        assert_eq!(depuis_ancien_id("oanda:EUR_USD").map(|s| s.to_string()).as_deref(), Some("OANDA:EURUSD"));
        assert_eq!(depuis_ancien_id("cmc:USDT.D").map(|s| s.to_string()).as_deref(), Some("CRYPTOCAP:USDT.D"));
        assert_eq!(depuis_ancien_id("inconnu:ABC"), None, "source inconnue : à vérifier");
        assert_eq!(depuis_ancien_id("binance:???"), None);
        assert_eq!(depuis_ancien_id("BTC"), None);
    }
}
