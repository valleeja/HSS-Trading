//! 🦀 Argos Scanner : stratégie double structure (Daily + 4H) sur les perps Hyperliquid.
//!
//! Utilisation :
//!   argos-scanner                         scan unique de la watchlist par défaut
//!   argos-scanner --coins BTC,ZEC,NEAR    scan de coins choisis
//!   argos-scanner --loop 15               rescanne toutes les 15 minutes
//!   argos-scanner --pivot 3               bougies de chaque côté d'un pivot (3 par défaut)
//!   argos-scanner --offline ./data        lit COIN_1d.json et COIN_4h.json au lieu de l'API

#![forbid(unsafe_code)]

mod hl;

use argos_scanner::scan::{self, Report, Verdict};
use argos_scanner::DEFAULT_COINS;
use std::{env, fs, thread, time::Duration};

struct Args {
    coins: Vec<String>,
    every_min: Option<u64>,
    pivot: usize,
    offline: Option<String>,
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        coins: DEFAULT_COINS.iter().map(|s| s.to_string()).collect(),
        every_min: None,
        pivot: 3,
        offline: None,
    };
    let mut it = env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut val = |n: &str| it.next().ok_or(format!("{n} attend une valeur"));
        match arg.as_str() {
            "--coins" => a.coins = val("--coins")?.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
            "--loop" => a.every_min = Some(val("--loop")?.parse().map_err(|_| "--loop attend un nombre de minutes")?),
            "--pivot" => a.pivot = val("--pivot")?.parse().map_err(|_| "--pivot attend un entier")?,
            "--offline" => a.offline = Some(val("--offline")?),
            "-h" | "--help" => {
                println!(
                    "{}",
                    include_str!("main.rs")
                        .lines()
                        .take(9)
                        .map(|l| l.trim_start_matches("//!"))
                        .collect::<Vec<_>>()
                        .join("\n")
                );
                std::process::exit(0);
            }
            other => return Err(format!("argument inconnu : {other}")),
        }
    }
    Ok(a)
}

fn load(args: &Args, client: Option<&hl::Client>, coin: &str) -> Result<(Vec<hl::Candle>, Vec<hl::Candle>), String> {
    match (&args.offline, client) {
        (Some(dir), _) => {
            let read = |tf: &str| {
                let p = format!("{dir}/{coin}_{tf}.json");
                fs::read_to_string(&p).map_err(|e| format!("{p} : {e}")).and_then(|s| hl::parse(&s))
            };
            Ok((read("1d")?, read("4h")?))
        }
        (None, Some(c)) => {
            let d = c.candles(coin, "1d", 150)?;
            thread::sleep(Duration::from_millis(150)); // on reste poli avec l'API
            let h4 = c.candles(coin, "4h", 40)?;
            Ok((d, h4))
        }
        _ => Err("aucune source de données".into()),
    }
}

fn fmt(p: f64) -> String {
    if p >= 1000.0 {
        format!("{p:.1}")
    } else if p >= 10.0 {
        format!("{p:.2}")
    } else if p >= 1.0 {
        format!("{p:.4}")
    } else {
        format!("{p:.6}")
    }
}

fn print_reports(reports: &[Report]) {
    println!(
        "\n{:<6} {:>12}  {:<20} {:<20} {:>6} {:>6} {:>7}  VERDICT",
        "COIN", "PRIX", "DAILY", "4H", "RSI4H", "STO4H", "EMA200"
    );
    println!("{}", "─".repeat(104));
    for r in reports {
        println!(
            "{:<6} {:>12}  {} {:<17} {} {:<17} {:>6} {:>6} {:>7}  {}",
            r.coin,
            fmt(r.price),
            r.daily.bias.emoji(),
            r.daily.label(),
            r.h4.bias.emoji(),
            r.h4.label(),
            r.rsi_4h.map_or("-".into(), |v| format!("{v:.0}")),
            r.stoch_4h.map_or("-".into(), |v| format!("{v:.0}")),
            r.ema200_4h.map_or("-", |e| if r.price > e { "dessus" } else { "dessous" }),
            r.verdict.label()
        );
    }

    let aligned: Vec<&Report> = reports.iter().filter(|r| matches!(r.verdict, Verdict::Long | Verdict::Short)).collect();
    if !aligned.is_empty() {
        println!("\n🎯 SETUPS DOUBLE STRUCTURE");
        for r in aligned {
            if let Some(p) = &r.plan {
                println!(
                    "  {} {:<6} entrée {:>12}  SL {:>12}{}  TP {:>12}  RR {}",
                    r.verdict.label(),
                    r.coin,
                    fmt(p.entry),
                    fmt(p.stop),
                    if p.atr_stop { " (2 ATR)" } else { "" },
                    p.target.map_or("-".into(), fmt),
                    p.rr.map_or("-".into(), |v| format!("{v:.1}R"))
                );
            }
        }
    }

    let watch: Vec<&Report> = reports.iter().filter(|r| r.verdict == Verdict::Watch).collect();
    if !watch.is_empty() {
        println!("\n👀 À SURVEILLER");
        for r in watch {
            println!("  {:<6} {}", r.coin, r.note);
        }
    }
    println!("\n⚠️  Bémol : détection automatique par pivots. Valide toujours sur ta chart. Pas un conseil financier.");
}

fn run_once(args: &Args, client: Option<&hl::Client>) {
    let mut reports = Vec::new();
    for coin in &args.coins {
        match load(args, client, coin) {
            Ok((d, h4)) => match scan::analyse(coin, &d, &h4, args.pivot) {
                Some(r) => reports.push(r),
                None => eprintln!("⚠️  {coin} : pas assez de données"),
            },
            Err(e) => eprintln!("⚠️  {e}"),
        }
    }
    // Les setups alignés en premier.
    reports.sort_by_key(|r| match r.verdict {
        Verdict::Long | Verdict::Short => 0,
        Verdict::Watch => 1,
        Verdict::None => 2,
    });
    print_reports(&reports);
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("❌ {e}\nEssaie --help");
            std::process::exit(2);
        }
    };
    let client = if args.offline.is_none() {
        match hl::Client::new() {
            Ok(c) => Some(c),
            Err(e) => {
                eprintln!("❌ {e}");
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    println!("🦀 Argos Scanner · double structure Daily + 4H · {} coins", args.coins.len());
    loop {
        run_once(&args, client.as_ref());
        match args.every_min {
            Some(m) => {
                println!("\n⏳ Prochain scan dans {m} min (Ctrl+C pour arrêter)");
                thread::sleep(Duration::from_secs(m * 60));
            }
            None => break,
        }
    }
}
