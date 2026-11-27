//! 📰 What a news headline is about (pure functions, no network): the cryptos it names ([`coins_of`]) and its tab
//! ([`category_of`]). French and English words, whole words only, case ignored.

/// The cryptos the panels know by name: (ticker, names in French and English).
pub const COINS: &[(&str, &[&str])] = &[
    ("BTC", &["bitcoin", "bitcoins"]),
    ("ETH", &["ether", "ethereum", "ethers"]),
    ("SOL", &["solana"]),
    ("XRP", &["ripple"]),
    ("DOGE", &["dogecoin"]),
    ("ADA", &["cardano"]),
    ("HYPE", &["hyperliquid"]),
    ("LINK", &["chainlink"]),
    ("AVAX", &["avalanche"]),
    ("ZEC", &["zcash"]),
    ("LTC", &["litecoin"]),
    ("DOT", &["polkadot"]),
    ("TON", &["toncoin"]),
    ("TRX", &["tron"]),
    ("BNB", &["binance coin"]),
    ("SUI", &[]),
];

/// A news tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    /// The default one.
    Market,
    Etf,
    Regulation,
    Technology,
}

impl Category {
    /// The tabs, in the panel's order (after « Toutes »).
    pub const ALL: [Category; 4] = [Category::Market, Category::Etf, Category::Regulation, Category::Technology];

    /// Its tab name.
    pub fn label(self) -> &'static str {
        match self {
            Category::Market => "Marché",
            Category::Etf => "ETF crypto",
            Category::Regulation => "Régulation",
            Category::Technology => "Technologie",
        }
    }
}

/// The words of `s` (letters and digits; « l'ether » gives « l » and « ether »).
fn words(s: &str) -> Vec<&str> {
    s.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect()
}

/// Does `title` hold the phrase `name` (one or more whole words, case ignored)?
fn has_phrase(lower: &[String], name: &str) -> bool {
    let n: Vec<String> = words(name).iter().map(|w| w.to_lowercase()).collect();
    !n.is_empty() && lower.windows(n.len()).any(|w| w == n.as_slice())
}

/// 🪙 The cryptos named in `title`, in the order of [`COINS`], each once: by name (« bitcoin », « l'ether ») or by
/// its ticker in capitals (« BTC », not « btc »). None: the news is about the whole market.
pub fn coins_of(title: &str) -> Vec<&'static str> {
    let ws = words(title);
    let lower: Vec<String> = ws.iter().map(|w| w.to_lowercase()).collect();
    COINS
        .iter()
        .filter(|(t, names)| ws.contains(t) || names.iter().any(|n| has_phrase(&lower, n)))
        .map(|(t, _)| *t)
        .collect()
}

/// Words that put a news in a tab: whole words, or a stem when it ends with `…`.
const ETF: &[&str] = &["etf", "etfs"];
const REGULATION: &[&str] = &[
    "sec",
    "cftc",
    "mica",
    "amf",
    "régulat…",
    "regulat…",
    "réglement…",
    "loi",
    "lois",
    "law",
    "laws",
    "tribunal",
    "court",
    "interdi…",
    "ban",
    "bans",
    "banned",
    "banning",
    "procès",
    "lawsuit",
];
const TECHNOLOGY: &[&str] = &[
    "mise à jour",
    "upgrade",
    "upgrades",
    "fork",
    "hard fork",
    "réseau",
    "network",
    "protocole",
    "protocol",
    "layer",
    "quantique",
    "quantum",
    "blockchain",
];

fn has_any(lower: &[String], keys: &[&str]) -> bool {
    keys.iter().any(|k| match k.strip_suffix('…') {
        Some(stem) => lower.iter().any(|w| w.starts_with(stem)),
        None => has_phrase(lower, k),
    })
}

/// 🗂 The tab of `title`: ETF crypto, then Régulation, then Technologie, else Marché.
pub fn category_of(title: &str) -> Category {
    let lower: Vec<String> = words(title).iter().map(|w| w.to_lowercase()).collect();
    if has_any(&lower, ETF) {
        Category::Etf
    } else if has_any(&lower, REGULATION) {
        Category::Regulation
    } else if has_any(&lower, TECHNOLOGY) {
        Category::Technology
    } else {
        Category::Market
    }
}

/// 🪙 The crypto of a chart's base (« BTC » for BTCUSDT.P's base « BTC », « kPEPE » → « PEPE »): one of [`COINS`],
/// else `None` (not a crypto the news know: no ⚡).
pub fn coin_of_base(base: &str) -> Option<&'static str> {
    let b = crate::tickers::norm(base);
    COINS.iter().map(|(t, _)| *t).find(|t| *t == b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cryptos_citees() {
        assert_eq!(coins_of("Bitcoin : le BTC repasse au-dessus de 61 000 $"), ["BTC"]);
        assert_eq!(coins_of("Les ETF bitcoin et ether reçoivent 420 M$"), ["BTC", "ETH"]);
        assert_eq!(coins_of("L'Ethereum prépare sa mise à jour"), ["ETH"]);
        assert_eq!(coins_of("Solana and XRP rally as SOL tops $200"), ["SOL", "XRP"]);
        assert_eq!(coins_of("Ripple wins in court"), ["XRP"]);
        assert_eq!(coins_of("Hyperliquid's HYPE hits a record"), ["HYPE"]);
        assert!(coins_of("Le marché crypto reprend 60 milliards $").is_empty(), "no coin: the market");
        assert!(coins_of("btc in lower case is not a ticker").is_empty());
        assert!(coins_of("Bitcoiners love it").is_empty(), "whole words only");
        assert!(coins_of("Tronçonneuse").is_empty(), "« tron » inside a word is not Tron");
        assert_eq!(coins_of("Tron (TRX) et Zcash"), ["ZEC", "TRX"], "in the table's order");
    }

    #[test]
    fn onglets() {
        assert_eq!(category_of("Les ETF bitcoin reçoivent 420 M$"), Category::Etf);
        assert_eq!(category_of("BlackRock's spot ETFs see outflows"), Category::Etf);
        assert_eq!(category_of("La SEC repousse sa décision"), Category::Regulation);
        assert_eq!(category_of("Crypto regulation: the CFTC weighs in"), Category::Regulation);
        assert_eq!(category_of("L'AMF met en garde"), Category::Regulation);
        assert_eq!(category_of("Le Népal interdit le bitcoin"), Category::Regulation);
        assert_eq!(category_of("Ordinateurs quantiques : la mise à jour de sécurité avance"), Category::Technology);
        assert_eq!(category_of("Ethereum network upgrade goes live"), Category::Technology);
        assert_eq!(category_of("Bitcoin repasse au-dessus de 61 000 $"), Category::Market);
        assert_eq!(category_of("Section 5 of the report"), Category::Market, "« sec » only as a whole word");
        assert_eq!(Category::ALL.map(Category::label), ["Marché", "ETF crypto", "Régulation", "Technologie"]);
    }

    #[test]
    fn crypto_de_la_chart() {
        assert_eq!(coin_of_base("BTC"), Some("BTC"));
        assert_eq!(coin_of_base("ETH"), Some("ETH"));
        assert_eq!(coin_of_base("XAUUSD"), None);
    }
}
