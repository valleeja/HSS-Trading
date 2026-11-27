use super::*;
use crate::text::ecrits;

/// 1 Nov 2026, 00:00 UTC (a Sunday).
const NOV_1: i64 = 1_793_491_200;
const LESSON: &str = "0-02-savoir-dans-quoi-tu-t-embarques";

/// The mock-up's trades (donnees.js): day, time, symbol, long, exit, R (1 000 $ of risk), costs in cents, plan,
/// market date in replay.
#[allow(clippy::type_complexity)]
const TRADES: [(u32, &str, &str, bool, &str, f64, i64, PlanMark, Option<&str>); 12] = [
    (2, "10:42", "BTC", true, "objectif", 2.0, 5820, PlanMark::Followed, None),
    (3, "14:05", "ETH", false, "stop", -1.0, 4130, PlanMark::Followed, None),
    (5, "09:58", "SOL", true, "objectif", 2.0, 3560, PlanMark::Followed, None),
    (6, "11:20", "BTC", true, "stop", -1.62, 9620, PlanMark::NotFollowed, None),
    (9, "15:31", "BTC", false, "fermé à la main", 0.8, 5210, PlanMark::Partly, None),
    (10, "10:12", "ETH", true, "objectif", 2.0, 6190, PlanMark::Followed, None),
    (12, "13:47", "BTC", true, "break-even", 0.0, 4810, PlanMark::Followed, None),
    (13, "20:15", "SOL", false, "stop", -1.0, 3000, PlanMark::Followed, Some("14 mars 2025")),
    (16, "10:05", "BTC", true, "stop", -1.4, 7010, PlanMark::NotFollowed, None),
    (18, "09:44", "SOL", true, "objectif", 2.5, 4480, PlanMark::Followed, None),
    (19, "16:02", "ETH", false, "fermé à la main", -0.45, 3320, PlanMark::Partly, None),
    (20, "21:30", "BTC", true, "fin du replay", 0.35, 2240, PlanMark::NotJudged, Some("2 avr. 2025")),
];

fn net(r: f64, costs: i64) -> Money {
    Money::from_cents((r * 100_000.0).round() as i64 - costs)
}

/// The mock-up's month: November 2026 seen on the 21st, 12 trades, 2 of them in replay.
fn mock_up() -> MonthView {
    let trades: Vec<TradeRow> = TRADES
        .iter()
        .enumerate()
        .map(|(k, (day, time, base, long, exit, r, costs, plan, market))| TradeRow {
            long: *long,
            base: base.to_string(),
            exit: exit.to_string(),
            day: day_label(NOV_1 + i64::from(day - 1) * 86_400),
            time: time.to_string(),
            market: market.map(str::to_string),
            r: Some(*r),
            net: net(*r, *costs),
            costs: Money::from_cents(*costs),
            plan: *plan,
            last: k,
        })
        .collect();
    let start = Money::from_cents(10_120_460);
    let mut balance = start;
    let days = (1..=21)
        .map(|d| {
            for t in TRADES.iter().filter(|t| t.0 == d) {
                balance += net(t.5, t.6);
            }
            balance
        })
        .collect::<Vec<_>>();
    let total = trades.iter().fold(Money::ZERO, |s, t| s + t.net);
    let r: f64 = TRADES.iter().map(|t| t.5).sum();
    let symbol = |base: &str| {
        let ts: Vec<_> = TRADES.iter().filter(|t| t.2 == base).collect();
        SymbolRow {
            base: base.into(),
            trades: ts.len(),
            r: ts.iter().map(|t| t.5).sum(),
            net: ts.iter().fold(Money::ZERO, |s, t| s + net(t.5, t.6)),
        }
    };
    MonthView {
        account: "Démo Argos 100K".into(),
        zone: "heure de New York".into(),
        year: 2026,
        month: 11,
        prev: Some((2026, 10)),
        next: None,
        start,
        net: total,
        pct: total.to_f64() * 100.0 / start.to_f64(),
        r,
        r_avg: Some(r / 12.0),
        winners: 6,
        judged: 11,
        followed: 7,
        partly: 2,
        not_followed: 2,
        days,
        month_days: 30,
        high: Money::from_cents(10_503_930),
        drawdown: Money::from_cents(254_820),
        drawdown_pct: 254_820.0 * 100.0 / 10_503_930.0,
        drawdown_days: Some((10, 16)),
        losing_streak: 3,
        streak_days: Some((12, 16)),
        fees: Money::from_cents(41_230),
        slippage: Money::from_cents(15_160),
        funding: Money::from_cents(3_000),
        trades,
        best: Some(9),
        worst: Some(3),
        symbols: vec![symbol("BTC"), symbol("ETH"), symbol("SOL")],
        ai: Some(MonthAi {
            worked: Partie {
                ton: Ton::Bon,
                pastille: "+6,50 R".into(),
                texte: "Tes 7 plans suivis ont fait +6,50 R à eux seuls.".into(),
            },
            to_review: Partie {
                ton: Ton::Mauvais,
                pastille: "2 fois".into(),
                texte: "Stop éloigné à la main : −3,02 R au lieu de −2 R.".into(),
            },
            lesson: "Garde le stop où ton plan l'a mis : c'est lui qui limite ta perte.".into(),
            academy: Some(LESSON),
            reminder: "Analyse affichée, pas un ordre.".into(),
        }),
    }
}

/// A month without any trade (the 1st of the month).
fn empty() -> MonthView {
    let start = Money::from_cents(10_000_000);
    MonthView {
        net: Money::ZERO,
        pct: 0.0,
        r: 0.0,
        r_avg: None,
        winners: 0,
        judged: 0,
        followed: 0,
        partly: 0,
        not_followed: 0,
        days: vec![start],
        start,
        high: start,
        drawdown: Money::ZERO,
        drawdown_pct: 0.0,
        drawdown_days: None,
        losing_streak: 0,
        streak_days: None,
        fees: Money::ZERO,
        slippage: Money::ZERO,
        funding: Money::ZERO,
        trades: vec![],
        best: None,
        worst: None,
        symbols: vec![],
        ai: None,
        ..mock_up()
    }
}

/// 🖥 The mock-up's panel: 960 wide (the window of 1 440, the Simulator panel on the right).
fn pc(h: f64) -> Frame {
    Frame {
        w: 960.0,
        h,
        sheet: false,
        room: 0.0,
    }
}

/// 📱 The mock-up's phone: 390 × 844, the whole screen, the Argos AI round over its bottom.
fn phone() -> Frame {
    Frame {
        w: 390.0,
        h: 844.0,
        sheet: true,
        room: 76.0,
    }
}

/// What the page writes, in order.
fn written(v: &MonthView, page: &MonthPage, f: Frame) -> Vec<String> {
    let _ = ecrits();
    page.draw(v, &Text::default(), f, None).unwrap();
    ecrits()
}

#[test]
fn the_page_says_what_the_mock_up_says() {
    let v = mock_up();
    let e = written(&v, &MonthPage::default(), pc(800.0));
    let expected = [
        "Rapport du mois",
        "‹",
        "›",
        "novembre 2026",
        "Imprimer en PDF",
        "✕",
        "Démo Argos 100K · 12 trades fermés · heure de New York · dont 2 en replay ⏪",
        "RÉSULTAT NET",
        "+3 586,10 $",
        "+3,54 % du compte au 1",
        "er",
        "\u{a0}nov.",
        "EN R",
        "+4,18 R",
        "moyenne +0,35 R par trade",
        "RÉUSSITE",
        "50 %",
        "6 gagnants sur 12",
        "PLANS SUIVIS",
        "7 sur 11",
        "2 en partie · 2 pas suivis",
        "Courbe du compte",
        "plus haut 105 039,30 $",
        "102 k",
        "105 k",
        "1 nov.",
        "30 nov.",
        "Plus forte baisse du mois : −2 548,20 $ (−2,43 %)",
        "Argos AI · ton mois",
        "CE QUI A MARCHÉ",
        "+6,50 R",
        "Tes 7 plans suivis ont fait +6,50 R à eux seuls.",
        "À REVOIR",
        "2 fois",
        "Stop éloigné à la main : −3,02 R au lieu de −2 R.",
        "UNE LEÇON",
        "Leçon : Savoir dans quoi tu t'embarques",
        "Analyse affichée, pas un ordre.",
        "MEILLEUR TRADE",
        "▲ SOL · objectif",
        "mer. 18 nov.",
        "+2,50 R",
        "+2 455,20 $",
        "PIRE TRADE",
        "▲ BTC · stop",
        "ven. 6 nov.",
        "−1,62 R",
        "−1 716,20 $",
        "Tous les détails",
        "par symbole, coûts payés, série de pertes, liste des trades",
        "⌄",
    ];
    for t in expected {
        assert!(e.iter().any(|x| x == t), "« {t} » is missing: {e:?}");
    }
    // folded: nothing of the details, never a text cut by « … »
    assert!(!e.iter().any(|x| x == "PAR SYMBOLE" || x == "LES TRADES DU MOIS"), "{e:?}");
    assert!(e.iter().all(|x| !x.ends_with('…')), "{e:?}");
    // 101 k (under the balances) and 106 k (above them) are not written: the mock-up's four round values
    assert!(!e.iter().any(|x| x == "101 k" || x == "106 k"), "{e:?}");
}

/// The mock-up's boxes, as its browser lays them out: 🖥 722.0625 high, 📱 1 212.09375 (and the room of the round).
#[test]
fn heights_of_the_mock_up() {
    let (v, t) = (mock_up(), Text::default());
    let page = MonthPage::default();
    assert_eq!(page.height(&v, &t, pc(800.0)), 722.0625);
    let cell = Frame { room: 0.0, ..phone() };
    assert_eq!(page.height(&v, &t, cell), 1212.09375);
    assert!((page.height(&v, &t, phone()) - page.height(&v, &t, cell) - 76.0).abs() < 1e-9);
    // without the Argos AI's analysis nor any trade: shorter, without « Meilleur trade »
    let e = empty();
    assert!(page.height(&e, &t, pc(800.0)) < 722.0625 - 80.0);
    let w = written(&e, &page, pc(800.0));
    assert!(!w.iter().any(|x| x == "MEILLEUR TRADE"), "{w:?}");
    for s in [
        "—",
        "aucun trade fermé",
        "aucun plan jugé",
        "aucun trade avec un stop",
        "Pas de baisse ce mois-ci.",
    ] {
        assert!(w.iter().any(|x| x == s), "« {s} » is missing: {w:?}");
    }
    assert!(w.iter().any(|x| x.starts_with("Pas encore de trade fini ce mois-ci")), "{w:?}");
}

/// Where the mock-up puts each button (its browser's boxes, panel coordinates).
#[test]
fn clicks_where_the_mock_up_shows_the_buttons() {
    let (v, t) = (mock_up(), Text::default());
    let f = pc(800.0);
    let mut page = MonthPage::default();
    // › leads nowhere (the month is the current one); ‹ shows October, from the top (a window too low: it scrolls)
    assert_eq!(page.click(&v, &t, f, 434.6, 42.0), MonthClick::Nothing);
    page.scroll = 30.0;
    assert_eq!(page.click(&v, &t, pc(600.0), 273.5, 42.0 - 30.0), MonthClick::Changed);
    assert_eq!((page.month, page.scroll), (Some((2026, 10)), 0.0));
    let mut page = MonthPage::default();
    assert_eq!(page.click(&v, &t, f, 924.6, 42.0), MonthClick::Close);
    assert_eq!(page.click(&v, &t, f, 822.0, 42.0), MonthClick::Print);
    assert_eq!(page.click(&v, &t, f, 120.0, 618.0), MonthClick::Card(9));
    assert_eq!(page.click(&v, &t, f, 700.0, 618.0), MonthClick::Card(3));
    assert_eq!(page.click(&v, &t, f, 700.0, 486.0), MonthClick::Lesson(LESSON));
    assert!(page.over_button(&v, &t, f, 700.0, 486.0));
    // the page holds the clicks off its buttons (nothing goes to the chart under it)
    assert_eq!(page.click(&v, &t, f, 300.0, 400.0), MonthClick::Nothing);
    assert!(!page.over_button(&v, &t, f, 300.0, 400.0));
    // « Tous les détails » unfolds and folds back
    let folded = page.height(&v, &t, f);
    assert_eq!(page.click(&v, &t, f, 400.0, 679.0), MonthClick::Changed);
    assert!(page.details && page.height(&v, &t, f) > folded + 400.0);
    assert_eq!(page.click(&v, &t, f, 400.0, 679.0), MonthClick::Changed);
    assert!(!page.details);
}

/// 📱 The page scrolls under the finger, never past its end; the clicks follow the scroll.
#[test]
fn the_phone_page_scrolls() {
    let (v, t) = (mock_up(), Text::default());
    let f = phone();
    let mut page = MonthPage::default();
    let h = page.height(&v, &t, f);
    assert!(h > f.h);
    page.wheel(-50.0, &v, &t, f);
    assert_eq!(page.scroll, 0.0);
    page.wheel(10_000.0, &v, &t, f);
    assert!((page.scroll - (h - f.h)).abs() < 1e-9);
    // « Tous les détails » (1 140.09 to 1 184.09 in the mock-up) at the bottom of the screen
    let y = 1162.09 - page.scroll;
    assert!(y < f.h - 76.0, "above the round: {y}");
    assert_eq!(page.click(&v, &t, f, 120.0, y), MonthClick::Changed);
    assert!(page.details);
    // unfolded, the page goes on: the end of the list can come up
    page.wheel(10_000.0, &v, &t, f);
    assert!((page.scroll - (page.height(&v, &t, f) - f.h)).abs() < 1e-9);
    let e = written(&v, &page, f);
    for s in ["PAR SYMBOLE", "COÛTS ET RISQUE", "LES TRADES DU MOIS"] {
        assert!(e.iter().any(|x| x == s) || page.scroll > 0.0, "« {s} »: {e:?}");
    }
    // the header and the round's room: the phone shows the short month
    let top = written(&v, &MonthPage::default(), f);
    assert!(top.iter().any(|x| x == "nov. 2026"), "{top:?}");
    assert!(
        !top.iter().any(|x| x == "par symbole, coûts payés, série de pertes, liste des trades"),
        "{top:?}"
    );
}

/// 📱 Suivi 5 (Jee, 21 Nov): the Argos AI round lay over the content while it scrolled. The content now stops above
/// the round's room: nothing of it is drawn there, and a touch there takes nothing. (Without the room, « Tous les
/// détails » is drawn there and opens.)
#[test]
fn nothing_of_the_content_under_the_round() {
    let (v, t) = (mock_up(), Text::default());
    for (room, under) in [(76.0, false), (0.0, true)] {
        let f = Frame { room, ..phone() };
        let mut page = MonthPage::default();
        // « Tous les détails » (1 140.09 to 1 184.09 in the mock-up) in the middle of the round's 76 points
        let y = f.h - 38.0;
        page.scroll = 1162.09 - y;
        let pm = page.draw(&v, &t, f, None).unwrap();
        let bg = pm.pixel(4, 768).unwrap();
        let empty = (768..844).all(|py| (1..384).all(|px| pm.pixel(px, py) == Some(bg)));
        assert_eq!(empty, !under, "room {room}: the content drawn under the round: {under}");
        let click = page.click(&v, &t, f, 120.0, y);
        assert_eq!(click == MonthClick::Changed, under, "room {room}: {click:?}");
    }
}

#[test]
fn the_details_say_the_symbols_the_costs_and_every_trade() {
    let v = mock_up();
    let page = MonthPage {
        details: true,
        ..MonthPage::default()
    };
    let e = written(&v, &page, pc(2400.0));
    let expected = [
        "⌃",
        "PAR SYMBOLE",
        "BTC",
        "−217,10 $",
        "SOL",
        "+3,50 R",
        "+3 389,60 $",
        "COÛTS ET RISQUE",
        "593,90 $",
        "412,30 $",
        "151,60 $",
        "30,00 $",
        "−2 548,20 $ (−2,43 %)",
        "du 10 au 16 nov.",
        "3 trades",
        "du 12 au 16 nov.",
        "LES TRADES DU MOIS",
        "✓",
        "~",
        "✗",
        "—",
        "▲ BTC · fin du replay",
    ];
    for t in expected {
        assert!(e.iter().any(|x| x == t), "« {t} » is missing: {e:?}");
    }
    // a replay trade: the day it was made, then its market's date
    assert!(
        e.iter()
            .any(|x| x.replace('\u{a0}', " ").starts_with("⏪ ven. 13 nov. · 20:15 · marché 14 mars 2025")),
        "{e:?}"
    );
    assert!(e.iter().filter(|x| x.as_str() == "✓").count() == 7, "{e:?}");
    // 🖥 a narrower page (the 1 440 window's): its long lines go on, an amount or a date is never cut
    let e = written(&v, &page, Frame { w: 900.0, ..pc(2400.0) });
    let line = e
        .iter()
        .position(|x| x.replace('\u{a0}', " ").starts_with("⏪ ven. 13 nov."))
        .expect("the replay trade");
    assert!(e[line].ends_with(" ·") && e[line + 1] == "coûts\u{a0}30,00\u{a0}$", "{:?}", &e[line..line + 2]);
    assert!(!e.iter().any(|x| x == "$" || x.ends_with("coûts") || x.ends_with("marché")), "{e:?}");
}

/// The mock-up's axis: 400 $ above and under the balances, a line every 1 000 $ (102 k to 105 k).
#[test]
fn the_curve_scale_of_the_mock_up() {
    let g = curve_geo(&mock_up());
    assert!((g.lo - 100_804.6).abs() < 1e-6 && (g.hi - 105_439.3).abs() < 1e-6, "{} {}", g.lo, g.hi);
    assert_eq!(g.step, 1000.0);
    assert_eq!(g.marks, vec![102_000.0, 103_000.0, 104_000.0, 105_000.0]);
    assert_eq!(axis_text(102_000.0, 1000.0), "102 k");
    assert_eq!(axis_text(100_250.0, 50.0), "100,25 k");
    assert_eq!(axis_text(950.0, 50.0), "950");
    // a flat month: round values on the whole axis, inside it
    let g = curve_geo(&empty());
    assert!(
        g.marks.len() >= 2 && g.marks.iter().all(|m| *m > g.lo && *m < g.hi),
        "{:?} {} {}",
        g.marks,
        g.lo,
        g.hi
    );
    assert_eq!(
        (round_step(1158.7), round_step(160.0), round_step(4.0), round_step(0.0)),
        (1000.0, 200.0, 5.0, 1.0)
    );
}

#[test]
fn the_dates_and_figures_as_written() {
    assert_eq!(
        (month_label(2026, 11, false), month_label(2026, 11, true)),
        ("novembre 2026".into(), "nov. 2026".into())
    );
    assert_eq!(month_label(2027, 2, true), "févr. 2027");
    assert_eq!(day_label(NOV_1 + 17 * 86_400), "mer. 18 nov.");
    assert_eq!(day_label(NOV_1 - 1), "sam. 31 oct.");
    assert_eq!(date_label(1_741_910_400), "14 mars 2025");
    assert_eq!((r_text(4.18), r_text(-1.62), r_text(0.001)), ("+4,18 R".into(), "−1,62 R".into(), "0 R".into()));
    assert_eq!(
        (pct_text(3.5434), pct_text(-2.4259), pct_text(0.0)),
        ("+3,54 %".into(), "−2,43 %".into(), "0,00 %".into())
    );
}

#[test]
fn the_glyphs_of_the_page_are_in_the_fonts() {
    let t = Text::default();
    for c in ['✕', '⌄', '⌃', '‹', '›', '▲', '▼', '✓', '✗'] {
        assert!(t.has_char(c), "{c}");
    }
}

/// 📄 The printed page says what the mock-up's printed page says.
#[test]
fn the_printed_page_of_the_mock_up() {
    let v = mock_up();
    let h = printable(&v, "sam. 21 nov. 2026 à 18:20");
    for s in [
        "<div class=\"marque\">ARGOS HSS TRADING</div><div class=\"titre\">Rapport du mois</div><div class=\"mois\">Novembre 2026</div>",
        "<b>Compte démo · Démo Argos 100K</b><br>Du 1<sup>er</sup> au 21 nov. 2026 · heure de New York<br>Imprimé le sam. 21 nov. 2026 à 18:20",
        "<div class=\"big up\">+3 586,10 $</div><div class=\"pt\">+3,54 % du compte au 1<sup>er</sup> nov.</div>",
        "<div class=\"big up\">+4,18 R</div>",
        "<div class=\"big \">7 sur 11</div><div class=\"pt\">2 en partie · 2 pas suivis</div>",
        "solde le 1er : 101 204,60 $ · aujourd'hui : 104 790,70 $ · plus haut : 105 039,30 $",
        "<b>Argos AI · ton mois.</b> Ce qui a marché : tes 7 plans suivis ont fait +6,50 R à eux seuls. À revoir : stop éloigné à la \
         main 2 fois, −3,02 R au lieu de −2 R. Une leçon : garde le stop où ton plan l'a mis, c'est lui qui limite ta perte \
         (Académie : « Savoir dans quoi tu t'embarques »).",
        "<div class=\"rap\">Analyse affichée, pas un ordre.</div>",
        "⏪ : trade de replay, compté le jour où tu l'as fait (date du marché à côté)",
        "<td>⏪ ven. 13 nov. <span class=\"rep\">(marché 14 mars 2025)</span></td><td>20:15</td><td>SOL</td><td>▼ vente</td><td>stop</td>",
        "<td class=\"n down\">−1,62 R</td><td class=\"n\">96,20 $</td><td class=\"n down\">−1 716,20 $</td><td class=\"c ko\">✗</td>",
        "<td class=\"c nn\">—</td>",
        "<span>Coûts payés :</span> 593,90 $ (frais 412,30 $ · écart et glissement 151,60 $ · funding 30,00 $)",
        "<span>Plus forte baisse :</span> −2 548,20 $ (−2,43 %), du 10 au 16 nov.",
        "<span>Plus longue série de pertes :</span> 3 trades (12 au 16 nov.)",
        "<span>Meilleur trade :</span> ▲ SOL, mer. 18 nov., +2,50 R (+2 455,20 $)",
        "<span>Pire trade :</span> ▲ BTC, ven. 6 nov., −1,62 R (−1 716,20 $)",
        "Simulation sur compte démo : pas un conseil financier. Aucun ordre réel n'a été passé.",
        "Argos HSS Trading · page 1 / 1",
    ] {
        assert!(h.contains(s), "« {s} » is missing:\n{h}");
    }
    assert_eq!(h.matches("<tr><td>").count(), 12 + 3, "12 trades and 3 symbols");
    // the curve: light colours, the mock-up's round values
    assert!(h.contains("stroke=\"#13955f\"") && h.contains(">102 k</text>") && h.contains(">30 nov.</text>"));
    // the Argos AI logo is in the page itself (nothing loaded from elsewhere)
    assert!(h.contains("src=\"data:image/png;base64,iVBORw0KGgo"));
    assert!(!h.contains("http://") && !h.contains("https://") || h.matches("http").count() == h.matches("http://www.w3.org/2000/svg").count());
}

#[test]
fn the_printed_page_on_more_pages_and_with_odd_names() {
    let mut v = mock_up();
    v.account = "Démo <b>& \"moi\"".into();
    let mut more = v.trades.clone();
    for _ in 0..2 {
        more.extend(v.trades.clone());
    }
    v.trades = more;
    // a best trade without a risk (no stop): no R, nothing in its place
    let best = v.best.unwrap();
    v.trades[best].r = None;
    let h = printable(&v, "aujourd'hui");
    assert!(h.contains("Compte démo · Démo &lt;b&gt;&amp; &quot;moi&quot;"), "{h}");
    assert!(h.contains("<span>Meilleur trade :</span> ▲ SOL, mer. 18 nov. (+2 455,20 $)"), "{h}");
    assert!(!h.contains("page 1 / 1"), "36 trades: more than one page");
    assert!(h.contains("<thead>"), "the table's head comes again on each page");
    let empty = printable(&empty(), "aujourd'hui");
    assert!(empty.contains("Aucun trade fermé ce mois-ci.") && empty.contains("Pas encore de trade fini ce mois-ci."));
    // 🖥 a whole file, which opens the print window by itself
    let d = printable_document(&mock_up(), "aujourd'hui");
    assert!(d.starts_with("<!doctype html>") && d.contains("<div id=\"argos-imprime\">") && d.contains("print()"));
    assert!(d.contains("<title>Argos · Rapport du mois · novembre 2026</title>"));
}

#[test]
fn a_part_said_inside_the_printed_paragraph() {
    let p = |pastille: &str, texte: &str| Partie {
        ton: Ton::Mauvais,
        pastille: pastille.into(),
        texte: texte.into(),
    };
    assert_eq!(
        inside_part(&p("2 fois", "Stop éloigné à la main : −3,02 R au lieu de −2 R.")),
        "stop éloigné à la main 2 fois, −3,02 R au lieu de −2 R"
    );
    // the sentence says it already, or the chip is not a count: unchanged
    assert_eq!(
        inside_part(&p("3 fois", "Entré sans stop 3 fois : la perte n'avait pas de limite.")),
        "entré sans stop 3 fois, la perte n'avait pas de limite"
    );
    assert_eq!(
        inside_part(&p("+6,50 R", "Tes 7 plans suivis ont fait +6,50 R à eux seuls.")),
        "tes 7 plans suivis ont fait +6,50 R à eux seuls"
    );
}
