//! 🧪 Strategies (`strategy.entry`, `strategy.exit`, `strategy.close`…): every expected fill worked out by hand, with
//! TradingView's rules (next open, the path inside a bar, turning around, pyramiding, trailing stop).

use super::*;
use argos_core::{Bar, Side};

/// Bars of one minute from (open, high, low, close).
fn barres(v: &[(f64, f64, f64, f64)]) -> Vec<Bar> {
    v.iter()
        .enumerate()
        .map(|(i, &(open, high, low, close))| Bar {
            time: 1_700_000_000 + i as i64 * 60,
            open,
            high,
            low,
            close,
            volume: 1.0,
        })
        .collect()
}

fn joue(src: &str, b: &[Bar]) -> Output {
    run(src, b, 60, &HashMap::new()).unwrap_or_else(|e| panic!("{e}"))
}

fn bilan(o: &Output) -> &strategie::Bilan {
    o.strategie.as_ref().expect("a strategy")
}

/// (entry id, exit id, entry price, exit price, side, quantity) of each closed trade.
fn trades(o: &Output) -> Vec<(String, String, f64, f64, Side, f64)> {
    bilan(o)
        .trades
        .iter()
        .map(|t| (t.entree.clone(), t.sortie.clone(), t.ferme.entry, t.ferme.exit, t.ferme.side, t.ferme.qty))
        .collect()
}

const MONTE: [(f64, f64, f64, f64); 5] = [
    (10.0, 11.0, 9.0, 10.0),
    (10.0, 12.0, 9.0, 11.0),
    (12.0, 13.0, 11.0, 12.0),
    (12.0, 14.0, 11.0, 13.0),
    (15.0, 16.0, 14.0, 15.0),
];

#[test]
fn entree_et_fermeture_a_l_ouverture_suivante() {
    let o = joue(
        r#"//@version=6
strategy("Test", overlay = true)
if bar_index == 1
    strategy.entry("L", strategy.long)
if bar_index == 3
    strategy.close("L")
plot(strategy.position_size, "pos")
plot(strategy.equity, "eq")
"#,
        &barres(&MONTE),
    );
    // given at bar 1's close: filled at bar 2's open (12); closed at bar 4's open (15)
    assert_eq!(trades(&o), vec![("L".into(), "Close entry(s) order L".into(), 12.0, 15.0, Side::Long, 1.0)]);
    let pos = &o.plots.iter().find(|p| p.title == "pos").unwrap().values;
    assert_eq!(pos, &vec![0.0, 0.0, 1.0, 1.0, 0.0], "the script reads the position at each close");
    let b = bilan(&o);
    assert_eq!(b.net.to_f64(), 3.0);
    assert_eq!(b.equite, vec![1_000_000.0, 1_000_000.0, 1_000_000.0, 1_000_001.0, 1_000_003.0]);
    assert_eq!(b.marques.len(), 2);
    assert!(b.marques[0].achat && !b.marques[1].achat);
    assert_eq!((b.marques[0].bar, b.marques[1].bar), (2, 4));
}

#[test]
fn stop_et_objectif_dans_la_meme_bougie_comme_tradingview() {
    // the entry fills at bar 2's open (100); its exit (stop 95, limit 105) waits for it, then works in the same bar
    let src = r#"//@version=6
strategy("Test")
if bar_index == 1
    strategy.entry("L", strategy.long)
    strategy.exit("X", "L", stop = 95, limit = 105)
"#;
    // high nearer to the open (5.5 against 6): open → high → low → close: the limit first
    let haut = barres(&[(100.0, 101.0, 99.0, 100.0), (100.0, 101.0, 99.0, 100.0), (100.0, 105.5, 94.0, 100.0)]);
    assert_eq!(trades(&joue(src, &haut)), vec![("L".into(), "X".into(), 100.0, 105.0, Side::Long, 1.0)]);
    // low nearer: the stop first
    let bas = barres(&[(100.0, 101.0, 99.0, 100.0), (100.0, 101.0, 99.0, 100.0), (100.0, 106.0, 94.5, 100.0)]);
    assert_eq!(trades(&joue(src, &bas)), vec![("L".into(), "X".into(), 100.0, 95.0, Side::Long, 1.0)]);
    // the open already past the stop (a gap): out at the open
    let trou = barres(&[
        (100.0, 101.0, 99.0, 100.0),
        (100.0, 101.0, 99.0, 100.0),
        (100.0, 100.0, 99.0, 99.0),
        (90.0, 91.0, 89.0, 90.0),
    ]);
    assert_eq!(trades(&joue(src, &trou)), vec![("L".into(), "X".into(), 100.0, 90.0, Side::Long, 1.0)]);
}

#[test]
fn retournement_et_pyramiding() {
    let src = r#"//@version=6
strategy("Test"PYR)
if bar_index == 1 or bar_index == 2
    strategy.entry("L", strategy.long)
if bar_index == 3
    strategy.entry("S", strategy.short, qty = 2)
plot(strategy.position_size, "pos")
"#;
    let b = barres(&MONTE);
    // without pyramiding: the 2nd long is refused; the short closes the long and turns around
    let o = joue(&src.replace("PYR", ""), &b);
    assert_eq!(trades(&o), vec![("L".into(), "S".into(), 12.0, 15.0, Side::Long, 1.0)]);
    assert_eq!(o.plots[0].values, vec![0.0, 0.0, 1.0, 1.0, -2.0]);
    assert_eq!(bilan(&o).ouverts.len(), 1);
    // pyramiding = 2: two longs (12, then 12 at bar 3's open), both closed by the short
    let o = joue(&src.replace("PYR", ", pyramiding = 2"), &b);
    assert_eq!(o.plots[0].values, vec![0.0, 0.0, 1.0, 2.0, -2.0]);
    assert_eq!(trades(&o).len(), 2);
}

#[test]
fn profit_et_perte_en_ticks_stop_suiveur() {
    // whole prices: one tick = 1. Entry at 100 (bar 2's open); trailing from +2, 1 tick behind the best price.
    // Bar 2: open 100 → low 99 (nearer) → high 104 (trailing starts, best 104) → close 103: the stop at 103 is hit.
    let o = joue(
        r#"//@version=6
strategy("Test")
if bar_index == 1
    strategy.entry("L", strategy.long)
strategy.exit("T", "L", trail_points = 2, trail_offset = 1, loss = 5)
"#,
        &barres(&[
            (100.0, 101.0, 99.0, 100.0),
            (100.0, 101.0, 99.0, 100.0),
            (100.0, 104.0, 99.0, 103.0),
            (103.0, 104.0, 102.0, 103.0),
        ]),
    );
    assert_eq!(trades(&o), vec![("L".into(), "T".into(), 100.0, 103.0, Side::Long, 1.0)]);
    // loss = 3 ticks: the stop at 97
    let o = joue(
        r#"//@version=6
strategy("Test")
if bar_index == 1
    strategy.entry("L", strategy.long)
strategy.exit("P", "L", profit = 10, loss = 3)
"#,
        &barres(&[(100.0, 101.0, 99.0, 100.0), (100.0, 101.0, 99.0, 100.0), (100.0, 101.0, 96.0, 98.0)]),
    );
    assert_eq!(trades(&o), vec![("L".into(), "P".into(), 100.0, 97.0, Side::Long, 1.0)]);
}

#[test]
fn ordres_a_la_cloture_taille_et_commission() {
    // process_orders_on_close: filled at the close of the bar that gives the order; 100 contracts; 0.1 % commission
    let o = joue(
        r#"//@version=6
strategy("Test", process_orders_on_close = true, default_qty_value = 100, commission_type = strategy.commission.percent, commission_value = 0.1)
if bar_index == 1
    strategy.entry("L", strategy.long)
if bar_index == 3
    strategy.close_all()
"#,
        &barres(&MONTE),
    );
    assert_eq!(trades(&o), vec![("L".into(), "Close position order".into(), 11.0, 13.0, Side::Long, 100.0)]);
    let t = &bilan(&o).trades[0].ferme;
    // 1100 × 0.1 % + 1300 × 0.1 % = 1.10 + 1.30
    assert_eq!((t.pnl.to_f64(), t.fees.to_f64()), (200.0, 2.4));
    assert_eq!(bilan(&o).net.to_f64(), 197.6);
}

#[test]
fn limite_stop_order_et_annulation() {
    let o = joue(
        r#"//@version=6
strategy("Test")
if bar_index == 0
    strategy.entry("Lim", strategy.long, limit = 11.5)
    strategy.entry("Stp", strategy.short, stop = 8)
if bar_index == 2
    strategy.cancel("Stp")
if bar_index == 3
    strategy.order("Vente", strategy.short, qty = 3)
"#,
        &barres(&MONTE),
    );
    // the limit waits: bar 1 never goes down to 11.5 from 10 (it is below: at the open, 10 ≤ 11.5): filled at 10
    // then strategy.order sells 3 at bar 4's open: 1 closes the long, 2 open a short
    let t = trades(&o);
    assert_eq!(t, vec![("Lim".into(), "Vente".into(), 10.0, 15.0, Side::Long, 1.0)]);
    let b = bilan(&o);
    assert_eq!(
        b.ouverts.iter().map(|x| (x.entree.as_str(), x.side, x.qty)).collect::<Vec<_>>(),
        vec![("Vente", Side::Short, 2.0)]
    );
}

#[test]
fn hors_d_une_strategie() {
    let e = run(
        "//@version=6\nindicator(\"I\")\nstrategy.entry(\"L\", strategy.long)\n",
        &barres(&MONTE),
        60,
        &HashMap::new(),
    )
    .unwrap_err();
    assert!(e.msg.contains("seulement dans un script strategy"), "{}", e.msg);
    assert_eq!(e.line, 3);
    let o = joue("//@version=6\nindicator(\"I\")\nplot(close)\n", &barres(&MONTE));
    assert!(o.strategie.is_none());
}

/// ⚙ The properties of `strategy(...)` in the settings, after the script's inputs; a value changed there is played.
#[test]
fn proprietes_dans_les_reglages() {
    let src = r#"//@version=6
strategy("Test", initial_capital = 5000, default_qty_value = 2, pyramiding = 3)
long = input.bool(true, "Long")
if bar_index == 1 and long
    strategy.entry("L", strategy.long)
"#;
    let o = joue(src, &barres(&MONTE));
    let titres: Vec<&str> = o.inputs.iter().map(|s| s.title.as_str()).collect();
    assert_eq!(
        titres,
        [
            "Long",
            "Capital de départ",
            "Taille des ordres",
            "Type de taille",
            "Pyramiding",
            "Commission %",
            "Glissement (ticks)",
            "Ordres à la clôture"
        ]
    );
    assert!(o.inputs[1..].iter().all(|s| s.group == strategie::PROPRIETES));
    assert_eq!(o.inputs[1].def, InVal::Num(5000.0));
    assert_eq!(o.inputs[3].options, ["Contrats", "Montant", "% du compte"]);
    let b = bilan(&o);
    assert_eq!((b.reglages.capital, b.reglages.valeur, b.reglages.pyramiding), (5000.0, 2.0, 3));
    assert_eq!(b.ouverts[0].qty, 2.0);
    // changed in ⚙: 1 000 $ a trade, on the close
    let choix = HashMap::from([
        ("Type de taille".to_string(), InVal::Text("Montant".into())),
        ("Taille des ordres".to_string(), InVal::Num(1100.0)),
        ("Ordres à la clôture".to_string(), InVal::Bool(true)),
    ]);
    let o = run(src, &barres(&MONTE), 60, &choix).unwrap();
    let b = bilan(&o);
    assert_eq!(b.reglages.taille, strategie::Taille::Montant);
    // 1 100 $ at bar 1's close (11): 100 contracts
    assert_eq!((b.ouverts[0].prix, b.ouverts[0].qty), (11.0, 100.0));
}

/// 📖 The guide's example (EMA 9 / 21 with a stop at 2 % and a target at 4 %, 10 % of the account, 0.05 % commission)
/// on real bars: it plays without error, every trade leaves by its stop, its target or a turn around, at a price the
/// bar reached; the closed results add up to the net, fees taken off.
#[test]
fn exemple_du_guide_sur_de_vraies_bougies() {
    let src = r#"//@version=6
strategy("Exemple EMA 9 / 21", overlay = true, initial_capital = 10000, default_qty_type = strategy.percent_of_equity, default_qty_value = 10, commission_type = strategy.commission.percent, commission_value = 0.05)
rapide = ta.ema(close, input.int(9, "EMA rapide"))
lente = ta.ema(close, input.int(21, "EMA lente"))
plot(rapide, "EMA rapide", color.aqua)
plot(lente, "EMA lente", color.orange)
if ta.crossover(rapide, lente)
    strategy.entry("Achat", strategy.long)
    strategy.exit("Sortie achat", "Achat", stop = close * 0.98, limit = close * 1.04)
if ta.crossunder(rapide, lente)
    strategy.entry("Vente", strategy.short)
    strategy.exit("Sortie vente", "Vente", stop = close * 1.02, limit = close * 0.96)
"#;
    let b = super::tests::bars();
    let o = run(src, &b, 3000, &HashMap::new()).unwrap_or_else(|e| panic!("{e}"));
    let bl = bilan(&o);
    assert!(bl.trades.len() >= 3, "{} trades", bl.trades.len());
    for t in &bl.trades {
        assert!(["Sortie achat", "Sortie vente", "Achat", "Vente"].contains(&t.sortie.as_str()), "{}", t.sortie);
        let barre = &b[t.bar_out];
        assert!(t.ferme.exit >= barre.low - 1e-9 && t.ferme.exit <= barre.high + 1e-9, "a price of its bar");
        assert!(t.bar_out >= t.bar_in);
        assert!(t.ferme.fees > argos_core::Money::ZERO);
    }
    let somme: argos_core::Money = bl.trades.iter().map(|t| t.ferme.net()).sum();
    assert_eq!(somme, bl.net);
    assert_eq!(bl.equite.len(), b.len());
}

/// Like TradingView: an open trade is not in the net profit, not even its entry fee (that one is in the open profit);
/// equity = initial capital + net profit + open profit.
#[test]
fn trade_ouvert_hors_du_resultat_net() {
    let o = joue(
        r#"//@version=6
strategy("Test", default_qty_value = 100, commission_type = strategy.commission.percent, commission_value = 0.1)
if bar_index == 1
    strategy.entry("L", strategy.long)
plot(strategy.netprofit, "net")
plot(strategy.openprofit, "open")
plot(strategy.equity, "eq")
"#,
        &barres(&MONTE),
    );
    let serie = |t: &str| o.plots.iter().find(|p| p.title == t).unwrap().values.clone();
    let proche = |a: &[f64], b: &[f64]| a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-6);
    // 100 bought at 12 (bar 2's open), 1.20 of fee: closes 12, 13, 15
    assert!(serie("net").iter().all(|x| *x == 0.0));
    let open = serie("open");
    assert!(proche(&open, &[0.0, 0.0, -1.2, 98.8, 298.8]), "{open:?}");
    let eq = serie("eq");
    assert!(proche(&eq, &[1e6, 1e6, 1e6 - 1.2, 1e6 + 98.8, 1e6 + 298.8]), "{eq:?}");
    let b = bilan(&o);
    assert_eq!(b.net, argos_core::Money::ZERO);
    assert_eq!((b.ouverts.len(), b.ouverts[0].resultat), (1, 300.0));
}
