use super::*;

/// 📋 Défaut signalé par Jee (« BT… », « ET… ») : à la largeur minimum du volet, la colonne Chg se masque (comme
/// TradingView) et chaque nom de la liste se lit en entier devant son prix ; plus large, Chg revient.
#[test]
fn noms_entiers_chg_masquee_si_la_place_manque() {
    let t = Text::default();
    let rows = [
        ("BTC", "BTC-USDC", "65316.0"),
        ("ETH", "ETH-USDC", "1814.6"),
        ("HYPE", "HYPE-USDC", "67.309"),
        ("SOL", "SOL-USDC", "71.651"),
        ("XAU", "XAU/USD", "2482.27"),
        ("PAXG", "PAXG-USDC", "2475.1"),
        ("XAUUSDT", "XAUUSDT", "2481.9"),
        ("ZEC", "ZEC-USDC", "898.5"),
        ("XRP", "XRP-USDC", "0.9595"),
    ];
    let mut w = Watchlist::default();
    w.prefs.open = true;
    w.sections = vec![Section {
        name: "Mes favoris".into(),
        rows: rows.iter().map(|(c, l, m)| Row::new(c, l, m, 1.0)).collect(),
    }];
    w.prefs.width = MIN_W;
    assert!(w.sans_chg(), "étroit : Chg masquée");
    let c = w.cols();
    for r in &w.sections[0].rows {
        let prix = t.width(&format_price(r.price(), r.decimals()), 13.5) as f64;
        assert!(t.width(&r.label, 13.5) as f64 <= c[0] - prix - 10.0 - 48.0, "« {} » entier", r.label);
    }
    w.prefs.width = MAX_W;
    assert!(!w.sans_chg(), "large : Chg revient");
}
