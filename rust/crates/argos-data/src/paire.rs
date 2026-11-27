//! ₿ Markets quoted in Bitcoin (Jee, 23 Nov 2026: « XRP/BTC », real pairs of each exchange). A chart opens a market
//! by its exchange and its « base »; the usual markets keep their base (« XRP »: XRP against USDT, USD or USDC, as
//! the exchange's module always did). A pair against BTC is written « XRP/BTC »: the asset, a slash, the quote. Every
//! exchange module reads it with [`parts`] and asks for the BTC pair; nothing else changes for the other markets.

/// The quote currencies that get their own chart base (only Bitcoin for now).
pub const QUOTES: [&str; 1] = ["BTC"];

/// The asset and the quote of a chart base: « XRP/BTC » → (« XRP », Some(« BTC »)); « XRP » → (« XRP », None).
pub fn parts(base: &str) -> (&str, Option<&str>) {
    match base.split_once('/') {
        Some((a, q)) if !a.is_empty() && QUOTES.iter().any(|x| x.eq_ignore_ascii_case(q)) => (a, Some(q)),
        _ => (base, None),
    }
}

/// The asset alone (« XRP/BTC » → « XRP »; « XRP » stays).
pub fn asset(base: &str) -> &str {
    parts(base).0
}

/// The quote written in the base, or `default` (« XRP/BTC » → « BTC »; « XRP » → `default`).
pub fn quote_or<'a>(base: &'a str, default: &'a str) -> &'a str {
    parts(base).1.unwrap_or(default)
}

/// Is this chart base a pair against Bitcoin?
pub fn contre_btc(base: &str) -> bool {
    parts(base).1.is_some_and(|q| q.eq_ignore_ascii_case("BTC"))
}

/// The chart base of a catalogue market: its asset, with « /BTC » when it is quoted in Bitcoin (never BTC/BTC).
pub fn base(asset: &str, quote: &str) -> String {
    if QUOTES.iter().any(|q| q.eq_ignore_ascii_case(quote)) && !asset.eq_ignore_ascii_case(quote) {
        format!("{}/{}", asset.to_uppercase(), quote.to_uppercase())
    } else {
        asset.to_string()
    }
}

/// An exchange symbol with the asset and the quote (`default` when the base has none) joined by `sep`
/// (« XRP/BTC », « - » → « XRP-BTC »; « XRP », « USDT », « » → « XRPUSDT »).
pub fn symbole(base: &str, sep: &str, default: &str) -> String {
    let (a, q) = parts(base);
    format!("{}{sep}{}", a.to_uppercase(), q.unwrap_or(default).to_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_des_paires_contre_btc() {
        assert_eq!(parts("XRP/BTC"), ("XRP", Some("BTC")));
        assert_eq!(parts("XRP"), ("XRP", None));
        assert_eq!(parts("xyz:NVDA"), ("xyz:NVDA", None), "HIP-3 unchanged");
        assert_eq!(parts("XRP/EUR"), ("XRP/EUR", None), "only BTC gets its own base");
        assert!(contre_btc("xrp/btc") && !contre_btc("BTC"));
        assert_eq!(base("XRP", "BTC"), "XRP/BTC");
        assert_eq!(base("XRP", "USDT"), "XRP");
        assert_eq!(base("BTC", "BTC"), "BTC");
        assert_eq!(symbole("XRP/BTC", "-", "USDT"), "XRP-BTC");
        assert_eq!(symbole("xrp", "", "USDT"), "XRPUSDT");
        assert_eq!(quote_or("XRP", "USD"), "USD");
        assert_eq!(asset("XRP/BTC"), "XRP");
    }
}
