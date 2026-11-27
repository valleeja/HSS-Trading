//! Test RÉEL sur Internet (ignoré par défaut). À lancer chez toi :
//!   cargo test -p argos-data --features native --test live -- --ignored --nocapture
//! Il demande 5000 bougies BTC 30m (pagination) et la liste des marchés à chaque exchange, puis affiche ce qu'il reçoit.

#![cfg(feature = "native")]

use argos_core::feed::Feed;
use argos_core::Timeframe;
use argos_data::http::NativeHttp;
use argos_data::{Exchange, ExchangeFeed, DEFAULT_CANDLES};

#[tokio::test]
#[ignore]
async fn huit_exchanges_en_direct() {
    let mut failed = vec![];
    for ex in Exchange::ALL {
        let feed = ExchangeFeed::new(ex, NativeHttp::default());
        match feed.candles("BTC", Timeframe::M30, DEFAULT_CANDLES).await {
            Ok(b) if !b.is_empty() => {
                let last = b.last().unwrap();
                println!(
                    "✅ {:<12} {} bougies · de {} à {} · clôture {}",
                    ex.name(),
                    b.len(),
                    b[0].time,
                    last.time,
                    last.close
                );
            }
            Ok(_) => {
                println!("⚠️ {:<12} aucune bougie", ex.name());
                failed.push(ex.name());
            }
            Err(e) => {
                println!("❌ {:<12} {e}", ex.name());
                failed.push(ex.name());
            }
        }
        match feed.markets().await {
            Ok(m) => println!("   {} marchés", m.len()),
            Err(e) => println!("   marchés ❌ {e}"),
        }
    }
    assert!(failed.is_empty(), "exchanges en échec : {failed:?}");
}

/// 🧮 XAUGBP / XAUEUR / XAGEUR (Hyperliquid HIP-3) et le DXY (Yahoo, puis calculé avec Kraken), en vrai.
#[tokio::test]
#[ignore]
async fn symboles_calcules_en_direct() {
    use argos_data::synth;
    let y = synth::dxy_yahoo(3600).await.expect("DXY Yahoo");
    let c = synth::dxy_calc(3600).await.expect("DXY calculé");
    let (yl, cl) = (y.last().unwrap().close, c.last().unwrap().close);
    println!("DXY Yahoo {yl:.3} ({} bougies) · calculé {cl:.3} ({} bougies)", y.len(), c.len());
    assert!((yl - cl).abs() / yl < 0.01, "calculé à moins de 1 % du vrai");
    for s in ["XAUGBP", "XAUEUR", "XAGEUR"] {
        let (a, b) = synth::legs(s).unwrap();
        let feed = ExchangeFeed::new(Exchange::Hyperliquid, NativeHttp::default());
        let x = feed.candles(a, Timeframe::H1, 200).await.expect(a);
        let z = feed.candles(b, Timeframe::H1, 200).await.expect(b);
        let r = argos_data::tv::ratio_bars(&x, &z);
        println!("{s} : {} bougies · dernière {:.3}", r.len(), r.last().map_or(0.0, |b| b.close));
        assert!(!r.is_empty());
    }
}
