//! 💾 Orderflow réel gardé entre deux lancements : les minutes **fermées** d'`argos_core::orderflow`, dans le même
//! format que le cache des bougies (`cache.rs` : en-tête, empreinte SHA-256, morceaux, droits 0600, fichier abîmé
//! jeté sans plantage). Un morceau par heure (`hl-btc.m<heure>.bin`) dans `~/.cache/argos-hss-trading/orderflow`.
//!
//! Corps : par minute, début (i64), pas (f64), complète (1 octet), 3 octets à zéro, nombre de rangées (u32), puis par
//! rangée : numéro (i64), achats (f64), ventes (f64). Limites : `GARDE_JOURS` jours et `MAX_OCTETS_ORDERFLOW` en tout,
//! les plus vieux fichiers effacés d'abord.

use crate::cache;
use argos_core::orderflow::{Cle, FlowBar, FlowStore};
use std::path::{Path, PathBuf};

/// Durée de garde.
pub const GARDE_JOURS: i64 = 30;
/// Au plus autant d'octets pour tout l'orderflow.
pub const MAX_OCTETS_ORDERFLOW: u64 = 200 * 1024 * 1024;
/// Un morceau de fichier par heure.
pub const MORCEAU_S: i64 = 3600;

const MAGIC: &[u8; 4] = b"ARGF";
const VERSION: u8 = 1;
/// Octets d'une minute (sans ses rangées) et d'une rangée.
const MINUTE: usize = 8 + 8 + 4 + 4;
const RANGEE: usize = 24;

/// 🏦 Flux de trades **non filtré** d'une bourse branchée à l'orderflow réel (le même marché que ses bougies), pour les
/// pièces `coins` ; ses messages se lisent avec `whales::parse_all` (un seul décodeur par bourse, partagé avec les
/// Baleines, qui gardent leur seuil). Hyperliquid n'est pas ici : ses trades passent par la connexion du carnet
/// (`book::sub`). Brancher la bourse suivante = son décodeur dans `whales::parse_all` + une ligne ici.
pub fn source(ex: crate::Exchange, coins: &[String]) -> Option<crate::live::LiveSub> {
    match ex {
        // perps USDT-M, comme les bougies Binance d'Argos ; aggTrade : `m` = l'acheteur est le teneur → vente preneur
        crate::Exchange::Binance => Some(crate::whales::binance(coins)),
        // perps USDT (instId « BTC-USDT-SWAP »), comme les bougies OKX d'Argos ; taille en contrats, comme leur volume
        crate::Exchange::Okx => Some(crate::whales::okx_trades(coins, "-USDT-SWAP")),
        // spot en dollars US (« live_trades_btcusd »), comme les bougies Bitstamp d'Argos ; taille en pièces
        crate::Exchange::Bitstamp => Some(crate::exchanges::bitstamp::trades_sub(coins)),
        // perps USDT (« btcusdt@trade »), comme les bougies WEEX d'Argos ; taille en pièces, sans numéro de trade
        crate::Exchange::Weex => Some(crate::exchanges::weex::trades_sub(coins)),
        // perps USDT (aggTrade, format Binance Futures), comme les bougies Aster d'Argos ; taille en pièces, numérotée (a)
        crate::Exchange::Aster => Some(crate::exchanges::aster::trades_sub(coins)),
        // 🪙 le spot de Binance, OKX, Bybit et Bitget (contre l'USDT), comme leurs bougies spot ; taille en pièces
        crate::Exchange::BinanceSpot | crate::Exchange::OkxSpot | crate::Exchange::BybitSpot | crate::Exchange::BitgetSpot => {
            ex.spot_venue().map(|v| crate::exchanges::spot::trades_sub(v, coins))
        }
        // perps USDT (« BTC_USDT », futures.trades), comme les bougies Gate.io d'Argos (pas le spot des Baleines) ;
        // taille en contrats convertie en pièces, seulement pour les pièces dont la taille du contrat est connue
        crate::Exchange::Gate => Some(crate::exchanges::gate::trades_sub(coins)),
        // le flux des Baleines de la bourse, sur les mêmes perps USDT que ses bougies (Bitunix : fapi ; Bybit : linear ;
        // Bitget : USDT-FUTURES ; BloFin et MEXC : taille en contrats convertie en pièces, seulement pour les pièces
        // dont la taille du contrat est connue) ; Bitget, BloFin et MEXC numérotent leurs trades, les deux autres non
        // (après une reconnexion, l'heure écarte ce qui est déjà compté). Coinbase et Kraken : le spot en dollars US
        // (« BTC-USD », canal matches ; « BTC/USD », canal trade v2), comme leurs bougies, numérotés (trade_id)
        crate::Exchange::Bitunix
        | crate::Exchange::Bybit
        | crate::Exchange::Bitget
        | crate::Exchange::Blofin
        | crate::Exchange::Mexc
        | crate::Exchange::Coinbase
        | crate::Exchange::Kraken => crate::whales::sub_of(ex.name(), coins),
        _ => None,
    }
}

/// La bourse est-elle branchée à l'orderflow réel ici (`crate::direct` : app de bureau, coque Tauri) ?
pub fn branchee(ex: crate::Exchange) -> bool {
    branchee_si(ex, crate::direct())
}

/// [`branchee`] selon que les requêtes partent sans les règles d'un navigateur (`direct`) ou d'une page web.
fn branchee_si(ex: crate::Exchange, direct: bool) -> bool {
    match ex {
        crate::Exchange::Hyperliquid => true,
        // KuCoin : son flux s'ouvre après un jeton public (`whales::kucoin_preparer`) que KuCoin refuse à une page web
        // (CORS) : dans le navigateur, l'estimé, annoncé, plutôt qu'un jeton redemandé sans fin
        crate::Exchange::Kucoin => direct,
        // Bitfinex: its real trades of the chart's exact market, read every few seconds (`bitfinex::trades_request`;
        // in the browser through the PC server's relay): its WebSocket trades carry no symbol to share one decoder
        crate::Exchange::Bitfinex | crate::Exchange::BitfinexSpot => direct || crate::relais::actif(),
        _ => source(ex, &[]).is_some(),
    }
}

/// L'orderflow réel de `coin` sur `ex` est-il possible ? La bourse est branchée et la taille de ses trades se lit en
/// pièces (bourse à contrats : seulement quand la taille du contrat de cette pièce est connue). Sinon : l'estimé.
pub fn reel_pour(ex: crate::Exchange, coin: &str) -> bool {
    // ₿ a pair against BTC (« XRP/BTC »): the trade streams above follow the USDT / USD markets, so the estimate;
    // Bitfinex reads the exact market (« XRP/BTC » included)
    (ex.bitfinex().is_some() || !crate::paire::contre_btc(coin)) && branchee(ex) && crate::whales::taille_lisible(ex.name(), coin)
}

/// 🔁 Bitfinex: how often its trades are read (5 s: 12 requests a minute, under its public limits).
pub const BITFINEX_PERIODE_MS: u64 = 5_000;

/// The Bitfinex trades not seen yet (id above `vu`), oldest first, and the new highest id.
pub fn bitfinex_nouveaux(mut list: Vec<crate::whales::Whale>, vu: Option<u64>) -> (Vec<crate::whales::Whale>, Option<u64>) {
    list.retain(|t| vu.is_none_or(|v| t.id.is_some_and(|i| i > v)));
    list.sort_by_key(|t| (t.time_ms, t.id));
    let haut = list.iter().filter_map(|t| t.id).max().or(vu);
    (list, haut)
}

/// Le contenu binaire de minutes (dans l'ordre donné).
pub fn bytes(minutes: &[FlowBar]) -> Vec<u8> {
    let mut body = vec![];
    for b in minutes {
        body.extend_from_slice(&b.time.to_le_bytes());
        body.extend_from_slice(&b.pas.to_le_bytes());
        body.extend_from_slice(&[u8::from(b.complete), 0, 0, 0]);
        body.extend_from_slice(&(b.rangees.len() as u32).to_le_bytes());
        for &(r, a, v) in &b.rangees {
            body.extend_from_slice(&r.to_le_bytes());
            body.extend_from_slice(&a.to_le_bytes());
            body.extend_from_slice(&v.to_le_bytes());
        }
    }
    cache::emballer(MAGIC, VERSION, minutes.len(), &body)
}

/// Les minutes d'un fichier, si tout est bon (signature, version, empreinte, tailles, nombres positifs, pas > 0,
/// minutes et rangées dans l'ordre) ; `None` sinon. Les tailles lues ne servent jamais à réserver de la mémoire.
pub fn parse(data: &[u8]) -> Option<Vec<FlowBar>> {
    let (n, mut body) = cache::deballer(data, MAGIC, VERSION)?;
    let mut out = vec![];
    let i64_de = |b: &[u8]| i64::from_le_bytes(b.try_into().unwrap_or_default());
    let f64_de = |b: &[u8]| f64::from_le_bytes(b.try_into().unwrap_or_default());
    for _ in 0..n {
        let tete = prendre(&mut body, MINUTE)?;
        let (time, pas) = (i64_de(&tete[..8]), f64_de(&tete[8..16]));
        if !(pas.is_finite() && pas > 0.0) || tete[16] > 1 || tete[17..20] != [0, 0, 0] {
            return None;
        }
        let nr = u32::from_le_bytes(tete[20..24].try_into().ok()?) as usize;
        let mut rangees = vec![];
        for _ in 0..nr {
            let r = prendre(&mut body, RANGEE)?;
            let (a, v) = (f64_de(&r[8..16]), f64_de(&r[16..24]));
            if !(a.is_finite() && v.is_finite() && a >= 0.0 && v >= 0.0) {
                return None;
            }
            rangees.push((i64_de(&r[..8]), a, v));
        }
        if !rangees.is_sorted_by_key(|r| r.0) {
            return None;
        }
        out.push(FlowBar {
            time,
            pas,
            rangees,
            complete: tete[16] == 1,
        });
    }
    (body.is_empty() && out.is_sorted_by_key(|b| b.time)).then_some(out)
}

/// Les `k` premiers octets de `body`, retirés de `body` (`None` s'il en reste moins).
fn prendre<'a>(body: &mut &'a [u8], k: usize) -> Option<&'a [u8]> {
    let (a, b) = body.split_at_checked(k)?;
    *body = b;
    Some(a)
}

/// Dossier de l'orderflow : à côté des bougies, `~/.cache/argos-hss-trading/orderflow` (un dossier temporaire dans
/// les tests).
pub fn dir() -> Option<PathBuf> {
    Some(cache::dir()?.parent()?.join("orderflow"))
}

/// 💾 L'orderflow dans le cache binaire (la seule mise en œuvre de `FlowStore` aujourd'hui).
#[derive(Debug, Clone, PartialEq)]
pub struct FlowCache {
    pub dir: PathBuf,
}

impl FlowCache {
    /// La série d'un marché : `…/hl-btc.bin` (ses morceaux : `hl-btc.m<heure>.bin`).
    fn serie(&self, cle: &Cle) -> PathBuf {
        let net = |s: &str| s.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_lowercase();
        self.dir.join(format!("{}-{}.bin", net(&cle.exchange), net(&cle.symbole)))
    }

    /// Les minutes d'un morceau ; un morceau abîmé est supprimé (rien n'est rendu pour lui).
    fn lire_morceau(p: &Path) -> Vec<FlowBar> {
        let Ok(data) = std::fs::read(p) else { return vec![] };
        parse(&data).unwrap_or_else(|| {
            let _ = std::fs::remove_file(p);
            vec![]
        })
    }
}

impl FlowStore for FlowCache {
    fn lire(&self, cle: &Cle, de_s: i64, a_s: i64) -> Vec<FlowBar> {
        let (k0, k1) = (de_s.div_euclid(MORCEAU_S), a_s.div_euclid(MORCEAU_S));
        cache::morceaux(&self.serie(cle))
            .into_iter()
            .filter(|(k, _)| (k0..=k1).contains(k))
            .flat_map(|(_, p)| Self::lire_morceau(&p))
            .filter(|b| (de_s..a_s).contains(&b.time))
            .collect()
    }

    fn ecrire(&mut self, cle: &Cle, minutes: &[FlowBar]) -> Result<(), String> {
        cache::creer_dossier(&self.dir).map_err(|e| e.to_string())?;
        let serie = self.serie(cle);
        let mut nouvelles = minutes.to_vec();
        nouvelles.sort_by_key(|b| b.time);
        for groupe in nouvelles.chunk_by(|a, b| a.time.div_euclid(MORCEAU_S) == b.time.div_euclid(MORCEAU_S)) {
            let p = cache::chemin_morceau(&serie, groupe[0].time.div_euclid(MORCEAU_S));
            let mut toutes = Self::lire_morceau(&p);
            toutes.retain(|b| groupe.iter().all(|n| n.time != b.time));
            toutes.extend_from_slice(groupe);
            toutes.sort_by_key(|b| b.time);
            cache::ecrire_morceau(&p, &bytes(&toutes)).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn limiter(&mut self, now_s: i64) {
        let avant = (now_s - GARDE_JOURS * 86_400).div_euclid(MORCEAU_S);
        let Ok(rd) = std::fs::read_dir(&self.dir) else { return };
        let series: Vec<PathBuf> = rd
            .flatten()
            .filter_map(|e| {
                let n = e.file_name().into_string().ok()?;
                let (stem, _) = n.split_once(".m")?;
                Some(self.dir.join(format!("{stem}.bin")))
            })
            .collect();
        for s in series {
            for (k, p) in cache::morceaux(&s) {
                if k < avant {
                    let _ = std::fs::remove_file(p);
                }
            }
        }
        // pas de lecture qui rafraîchit la date : les fichiers les plus vieux (dernière écriture) partent d'abord
        cache::evict(&self.dir, MAX_OCTETS_ORDERFLOW, Path::new(""));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cle() -> Cle {
        Cle {
            exchange: "hl".into(),
            symbole: "BTC".into(),
        }
    }

    fn minute(time: i64, complete: bool) -> FlowBar {
        FlowBar {
            time,
            pas: 10.0,
            rangees: vec![(9_000, 1.5, 0.25), (9_001, 0.0, 3.0)],
            complete,
        }
    }

    fn dossier(nom: &str) -> FlowCache {
        // dans le dossier de compilation du projet (rust/target/tmp), jamais dans /tmp
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/tmp")
            .join(format!("argos-of-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        FlowCache { dir }
    }

    #[test]
    fn ecriture_puis_relecture_identique() {
        let mut c = dossier("aller");
        let m: Vec<FlowBar> = (0..130).map(|k| minute(1_800_000_000 + k * 60, k % 7 != 0)).collect();
        c.ecrire(&cle(), &m[..70]).unwrap();
        c.ecrire(&cle(), &m[60..]).unwrap();
        assert_eq!(c.lire(&cle(), 0, i64::MAX), m, "deux écritures qui se chevauchent : rien en double");
        assert_eq!(c.lire(&cle(), m[10].time, m[20].time), m[10..20].to_vec());
        assert!(cache::morceaux(&c.serie(&cle())).len() >= 3, "un morceau par heure");
        assert_eq!(parse(&bytes(&m)), Some(m));
        std::fs::remove_dir_all(&c.dir).ok();
    }

    #[test]
    fn fichier_abime_ignore_sans_panique() {
        let mut c = dossier("abime");
        let m = vec![minute(1_800_000_000, true)];
        c.ecrire(&cle(), &m).unwrap();
        let (_, p) = cache::morceaux(&c.serie(&cle())).remove(0);
        let mut d = std::fs::read(&p).unwrap();
        let fin = d.len() - 1;
        d[fin] ^= 0xFF;
        std::fs::write(&p, &d).unwrap();
        assert!(c.lire(&cle(), 0, i64::MAX).is_empty());
        assert!(!p.exists(), "morceau abîmé supprimé");
        // corps cohérent avec la bonne empreinte mais des valeurs absurdes, coupé, ou trop court : refusé
        let mut nan = minute(0, true);
        nan.rangees[0].1 = f64::NAN;
        assert_eq!(parse(&bytes(&[nan])), None);
        let bon = bytes(&m);
        for k in 0..bon.len() {
            assert_eq!(parse(&bon[..k]), None, "coupé à {k} octets");
        }
        let mut faux_nombre = bon.clone();
        faux_nombre[8] = 0xFF; // annonce un nombre énorme de minutes
        assert_eq!(parse(&faux_nombre), None);
        std::fs::remove_dir_all(&c.dir).ok();
    }

    /// 🏦 Binance : messages aggTrade au format réel de fstream (achat preneur `m: false`, vente preneur `m: true`), lus
    /// par le décodeur des Baleines **sans** leur seuil, avec le numéro `a`.
    #[test]
    fn binance_trades_non_filtres_avec_numero() {
        let achat = r#"{"e":"aggTrade","E":1793908804400,"a":2689914736,"s":"BTCUSDT","p":"76319.20","q":"0.015","f":5866104221,"l":5866104221,"T":1793908804337,"m":false}"#;
        let vente = r#"{"e":"aggTrade","E":1793908804500,"a":2689914737,"s":"BTCUSDT","p":"76319.10","q":"0.250","f":5866104222,"l":5866104223,"T":1793908804451,"m":true}"#;
        let a = &crate::whales::parse_all(achat)[0];
        assert_eq!(
            (a.coin.as_str(), a.buy, a.size, a.time_ms, a.id),
            ("BTC", true, 0.015, 1_793_908_804_337, Some(2_689_914_736))
        );
        assert!(a.usd() < crate::whales::FLOOR, "petit trade : gardé ici, pas pour les Baleines");
        assert!(crate::whales::parse(achat).is_empty());
        let v = &crate::whales::parse_all(vente)[0];
        assert_eq!((v.buy, v.id), (false, Some(2_689_914_737)), "m = l'acheteur est le teneur : vente au marché");
        let s = source(crate::Exchange::Binance, &["BTC".into(), "ETH".into()]).unwrap();
        assert!(crate::net::autorise(&s.url), "adresse déjà dans la liste blanche");
        assert!(s.url.contains("fstream") && s.subscribe.contains("btcusdt@aggTrade") && s.subscribe.contains("ethusdt@aggTrade"));
        assert!(branchee(crate::Exchange::Hyperliquid) && branchee(crate::Exchange::Binance));
        // Bitfinex: its real trades, the exact market (a pair against BTC included)
        let bitfinex = |e: &crate::Exchange| e.bitfinex().is_some();
        assert!(crate::Exchange::ALL.iter().all(|e| branchee(*e)), "toutes les bourses de l'app sont branchées");
        assert!(reel_pour(crate::Exchange::BitfinexSpot, "XRP/BTC") && reel_pour(crate::Exchange::Bitfinex, "XRP"));
        assert!(
            !reel_pour(crate::Exchange::Binance, "XRP/BTC"),
            "the others: the estimate for a pair against BTC"
        );
        // 🌐 KuCoin in a web page: its token is refused to the browser (CORS), so not connected there; the others are
        // (Bitfinex in a page: only through the PC server's relay)
        assert!(!branchee_si(crate::Exchange::Kucoin, false) && branchee_si(crate::Exchange::Kucoin, true));
        assert!(!branchee_si(crate::Exchange::Bitfinex, false) || crate::relais::actif());
        assert!(crate::Exchange::ALL
            .iter()
            .filter(|e| **e != crate::Exchange::Kucoin && !bitfinex(e))
            .all(|e| branchee_si(*e, false)));
        // 🔁 Bitfinex read again every 5 s: only the trades not seen yet, oldest first
        let t = crate::exchanges::bitfinex::parse_trades("[[12,1795413822520,0.5,1],[11,1795413822519,-0.1,1],[10,1795413822000,0.2,1]]", "XRP/BTC").unwrap();
        let (n, haut) = bitfinex_nouveaux(t.clone(), None);
        assert_eq!((n.iter().map(|w| w.id.unwrap()).collect::<Vec<_>>(), haut), (vec![10, 11, 12], Some(12)));
        let (n, haut) = bitfinex_nouveaux(t, Some(11));
        assert_eq!((n.len(), n[0].id, n[0].coin.as_str(), haut), (1, Some(12), "XRP/BTC", Some(12)));
    }

    /// 🏦 OKX : messages du canal public `trades` des perps au format réel (champs relevés sur l'API publique le
    /// 2026-11-15 : instId, tradeId, px, sz en contrats, side = côté du preneur, ts en ms), lus par le décodeur des
    /// Baleines sans leur seuil, avec le numéro `tradeId`.
    #[test]
    fn okx_trades_non_filtres_avec_numero() {
        let msg = r#"{"arg":{"channel":"trades","instId":"BTC-USDT-SWAP"},"data":[{"instId":"BTC-USDT-SWAP","tradeId":"3084754957","px":"65174","sz":"0.01","side":"buy","ts":"1794751520397","count":"1","source":"0"},{"instId":"BTC-USDT-SWAP","tradeId":"3084754958","px":"65174.1","sz":"10.44","side":"sell","ts":"1794751520397","count":"2","source":"0"}]}"#;
        let t = crate::whales::parse_all(msg);
        assert_eq!(t.len(), 2);
        let (a, v) = (&t[0], &t[1]);
        assert_eq!(
            (a.coin.as_str(), a.buy, a.size, a.time_ms, a.id),
            ("BTC", true, 0.01, 1_794_751_520_397, Some(3_084_754_957)),
            "achat preneur"
        );
        assert_eq!((v.buy, v.size, v.id), (false, 10.44, Some(3_084_754_958)), "vente preneur, même milliseconde");
        assert_eq!(
            crate::whales::parse(msg).len(),
            1,
            "le seuil des Baleines ne garde que le gros trade ; ici, tous comptent"
        );
        assert!(crate::whales::parse_all(r#"{"event":"subscribe","arg":{"channel":"trades","instId":"BTC-USDT-SWAP"}}"#).is_empty());
        assert!(crate::whales::parse_all("pong").is_empty());
        let s = source(crate::Exchange::Okx, &["BTC".into(), "ETH".into()]).unwrap();
        assert!(crate::net::autorise(&s.url), "ws.okx.com déjà dans la liste blanche");
        assert!(s.subscribe.contains(r#""instId":"BTC-USDT-SWAP""#) && s.subscribe.contains(r#""instId":"ETH-USDT-SWAP""#));
        assert_eq!(s.subscribe.lines().count(), 1, "une seule connexion, un seul abonnement");
        assert_eq!(s.ping, "ping");
        assert!(branchee(crate::Exchange::Okx));
    }

    /// 🏦 Bitstamp (spot en dollars US) : vrais messages du flux public `live_trades_btcusd` relevés le 2026-11-15, lus
    /// par son décodeur unique (bougies, Baleines, orderflow) : `type` 0 = le preneur achète, 1 = il vend (dans chaque
    /// trade relevé, le côté dont l'ordre est le plus récent) ; `id` pour ne jamais compter un trade deux fois.
    #[test]
    fn bitstamp_trades_reels_sens_et_numero() {
        let vente = r#"{"data":{"id":674397349,"timestamp":"1794768305","amount":0.0013823,"amount_str":"0.00138230","price":64585.66,"price_str":"64585.66","type":1,"microtimestamp":"1794768305957000","buy_order_id":2072302918311936,"sell_order_id":2072302925004800},"channel":"live_trades_btcusd","event":"trade"}"#;
        let achat = r#"{"data":{"id":674400458,"timestamp":"1794768325","amount":0.00774251,"amount_str":"0.00774251","price":64587.51,"price_str":"64587.51","type":0,"microtimestamp":"1794768325218000","buy_order_id":2072303003893760,"sell_order_id":2072302976892929},"channel":"live_trades_btcusd","event":"trade"}"#;
        let v = crate::whales::parse_all(vente);
        let a = crate::whales::parse_all(achat);
        assert_eq!(
            (v[0].coin.as_str(), v[0].buy, v[0].size, v[0].price, v[0].time_ms, v[0].id, v[0].src),
            ("BTC", false, 0.0013823, 64585.66, 1_794_768_305_957, Some(674_397_349), "Bitstamp"),
            "type 1 : vente preneur (l'ordre de vente est le plus récent)"
        );
        assert_eq!((a[0].buy, a[0].id), (true, Some(674_400_458)), "type 0 : achat preneur");
        assert!(
            crate::whales::parse(achat).is_empty(),
            "le seuil des Baleines écarte ce petit trade ; ici, il compte"
        );
        // le même décodeur sert aux bougies
        let t = crate::exchanges::bitstamp::parse_trades(achat);
        assert_eq!((t[0].time_ms, t[0].price, t[0].size), (1_794_768_325_218, 64587.51, 0.00774251));
        for autre in [
            r#"{"event":"bts:subscription_succeeded","channel":"live_trades_btcusd","data":{}}"#,
            r#"{"event":"bts:heartbeat","channel":"","data":{"status":"success"}}"#,
            r#"{"data":{"id":1,"amount":1.0,"price":1.0,"type":2,"microtimestamp":"1000"},"channel":"live_trades_btcusd","event":"trade"}"#,
        ] {
            assert!(crate::whales::parse_all(autre).is_empty(), "{autre}");
        }
        let s = source(crate::Exchange::Bitstamp, &["BTC".into(), "ETH".into()]).unwrap();
        assert!(crate::net::autorise(&s.url), "ws.bitstamp.net déjà dans la liste blanche");
        assert_eq!(
            s.subscribe.lines().collect::<Vec<_>>(),
            [
                r#"{"event":"bts:subscribe","data":{"channel":"live_trades_btcusd"}}"#,
                r#"{"event":"bts:subscribe","data":{"channel":"live_trades_ethusd"}}"#
            ],
            "une seule connexion, un canal par pièce"
        );
        assert!(branchee(crate::Exchange::Bitstamp));
    }

    /// 🏦 Bitunix (perps USDT) : vrais messages du canal public `trade` relevés le 2026-11-15, lus par le décodeur des
    /// Baleines sans leur seuil. `s` = le côté du preneur : relevé avec le meilleur prix du carnet (`depth_book1`), 74
    /// achats sur 84 au prix vendeur et 39 ventes sur 54 au prix acheteur (les autres : carnet un peu en retard, il
    /// arrive toutes les ~300 ms). Pas de numéro de trade ; heure à la seconde ; taille en BTC.
    #[test]
    fn bitunix_trades_reels_sens_sans_numero() {
        let ventes = r#"{"ch":"trade","symbol":"BTCUSDT","ts":1794772303513,"data":[{"t":"2026-11-15T19:51:42Z","p":"64950.4","v":"0.0345","s":"sell"},{"t":"2026-11-15T19:51:42Z","p":"64952.6","v":"0.0003","s":"sell"}]}"#;
        let achat = r#"{"ch":"trade","symbol":"BTCUSDT","ts":1794772305226,"data":[{"t":"2026-11-15T19:51:45Z","p":"64950.4","v":"0.0126","s":"buy"}]}"#;
        let v = crate::whales::parse_all(ventes);
        assert_eq!(v.len(), 2);
        assert_eq!(
            (v[0].coin.as_str(), v[0].buy, v[0].size, v[0].price, v[0].time_ms, v[0].id, v[0].src),
            ("BTC", false, 0.0345, 64950.4, 1_794_772_302_000, None, "Bitunix"),
            "vente preneur, heure à la seconde, sans numéro"
        );
        let a = crate::whales::parse_all(achat);
        assert_eq!((a[0].buy, a[0].size, a[0].time_ms), (true, 0.0126, 1_794_772_305_000), "achat preneur");
        assert!(
            crate::whales::parse(achat).is_empty(),
            "le seuil des Baleines écarte ce petit trade ; ici, il compte"
        );
        for autre in [
            r#"{"op":"connect","ts":1794772303339,"data":{"result":true}}"#,
            r#"{"ch":"depth_book1","symbol":"BTCUSDT","ts":1794772376306,"data":{"b":[["64921.3","3.5186"]],"a":[["64921.4","4.4301"]]}}"#,
        ] {
            assert!(crate::whales::parse_all(autre).is_empty(), "{autre}");
        }
        let s = source(crate::Exchange::Bitunix, &["BTC".into(), "ETH".into()]).unwrap();
        assert!(crate::net::autorise(&s.url) && s.url.contains("fapi"), "perps, déjà dans la liste blanche");
        assert!(s.subscribe.contains(r#"{"symbol":"BTCUSDT","ch":"trade"}"#) && s.subscribe.contains(r#"{"symbol":"ETHUSDT","ch":"trade"}"#));
        assert!(branchee(crate::Exchange::Bitunix));
    }

    /// 🏦 Bybit (perps USDT, linear) : vrais messages de `publicTrade.BTCUSDT` relevés le 2026-11-15, lus par le décodeur
    /// des Baleines sans leur seuil. `S` = le côté du preneur : relevé avec le meilleur prix du carnet (`orderbook.1`),
    /// 72 achats sur 72 au prix vendeur et 253 ventes sur 253 au prix acheteur. Pas de numéro utilisable : `i` est un
    /// texte, et `seq` est le même pour tous les trades d'un message (il les ferait jeter) ; heure en ms ; taille en BTC.
    #[test]
    fn bybit_trades_reels_sens_sans_numero() {
        let achats = r#"{"topic":"publicTrade.BTCUSDT","type":"snapshot","ts":1794773160536,"data":[{"T":1794773160534,"s":"BTCUSDT","S":"Buy","v":"0.001","p":"64818.20","L":"PlusTick","i":"236ed8a5-011d-50f6-9dbb-1b88cb161c96","BT":false,"RPI":false,"seq":847148520745},{"T":1794773160534,"s":"BTCUSDT","S":"Buy","v":"0.001","p":"64818.40","L":"PlusTick","i":"17b3634a-0539-50a1-adee-abcd7f84bc84","BT":false,"RPI":false,"seq":847148520745}]}"#;
        let vente = r#"{"topic":"publicTrade.BTCUSDT","type":"snapshot","ts":1794773159288,"data":[{"T":1794773159286,"s":"BTCUSDT","S":"Sell","v":"0.005","p":"64817.50","L":"MinusTick","i":"7d30c850-23a2-5516-b5af-3eaf4df45b31","BT":false,"RPI":false,"seq":847148512988}]}"#;
        let a = crate::whales::parse_all(achats);
        assert_eq!(
            a.iter().map(|w| (w.coin.as_str(), w.buy, w.price, w.size, w.time_ms, w.id)).collect::<Vec<_>>(),
            [
                ("BTC", true, 64818.2, 0.001, 1_794_773_160_534, None),
                ("BTC", true, 64818.4, 0.001, 1_794_773_160_534, None)
            ],
            "deux achats preneur du même ordre, tous les deux comptés"
        );
        let v = crate::whales::parse_all(vente);
        assert_eq!((v[0].buy, v[0].size, v[0].src), (false, 0.005, "Bybit"), "vente preneur");
        assert!(
            crate::whales::parse(vente).is_empty(),
            "le seuil des Baleines écarte ce petit trade ; ici, il compte"
        );
        assert!(crate::whales::parse_all(r#"{"success":true,"ret_msg":"pong","conn_id":"x","op":"ping"}"#).is_empty());
        let s = source(crate::Exchange::Bybit, &["BTC".into(), "ETH".into()]).unwrap();
        assert!(
            crate::net::autorise(&s.url) && s.url.ends_with("/linear"),
            "perps linear, déjà dans la liste blanche"
        );
        assert!(s.subscribe.contains(r#""publicTrade.BTCUSDT""#) && s.subscribe.contains(r#""publicTrade.ETHUSDT""#));
        assert!(branchee(crate::Exchange::Bybit));
    }

    /// 🏦 Bitget (perps USDT-FUTURES) : vrais messages du canal public `trade` relevés le 2026-11-15, lus par le
    /// décodeur des Baleines sans leur seuil. `side` = le côté du preneur : relevé avec le meilleur prix du carnet
    /// (`books1`), 185 achats sur 185 au prix vendeur et 205 ventes sur 205 au prix acheteur. `tradeId` toujours
    /// croissant d'un message à l'autre, mais chaque message va du plus récent au plus ancien : l'agrégateur les range.
    #[test]
    fn bitget_trades_reels_sens_et_numero() {
        let achats = r#"{"action":"update","arg":{"instType":"USDT-FUTURES","channel":"trade","instId":"BTCUSDT"},"data":[{"ts":"1794773817565","price":"64728.5","size":"0.0001","side":"buy","tradeId":"1505956030111825924"},{"ts":"1794773817565","price":"64728.5","size":"0.0002","side":"buy","tradeId":"1505956030111825922"}],"ts":1794773817566}"#;
        let vente = r#"{"action":"update","arg":{"instType":"USDT-FUTURES","channel":"trade","instId":"BTCUSDT"},"data":[{"ts":"1794773821617","price":"64728.4","size":"0.0035","side":"sell","tradeId":"1505956047107145736"}],"ts":1794773821617}"#;
        let a = crate::whales::parse_all(achats);
        assert_eq!(
            a.iter().map(|w| (w.coin.as_str(), w.buy, w.size, w.time_ms, w.id)).collect::<Vec<_>>(),
            [
                ("BTC", true, 0.0001, 1_794_773_817_565, Some(1_505_956_030_111_825_924)),
                ("BTC", true, 0.0002, 1_794_773_817_565, Some(1_505_956_030_111_825_922))
            ],
            "achats preneur, du plus récent au plus ancien"
        );
        // l'agrégateur compte les deux, même reçus à l'envers
        let mut agg = argos_core::orderflow::Agregateur::new(Some(1.0));
        let flux: Vec<_> = a
            .iter()
            .map(|w| argos_core::orderflow::TradeFlux {
                time_ms: w.time_ms,
                prix: w.price,
                taille: w.size,
                achat: w.buy,
                id: w.id,
            })
            .collect();
        agg.ajouter(&flux);
        let m = agg.fermees(1_794_773_817_565 + 120_000);
        assert!((m[0].delta() - 0.0003).abs() < 1e-12, "{}", m[0].delta());
        let v = crate::whales::parse_all(vente);
        assert_eq!(
            (v[0].buy, v[0].size, v[0].src, v[0].id),
            (false, 0.0035, "Bitget", Some(1_505_956_047_107_145_736))
        );
        assert!(
            crate::whales::parse(vente).is_empty(),
            "le seuil des Baleines écarte ce petit trade ; ici, il compte"
        );
        assert!(crate::whales::parse_all("pong").is_empty());
        let s = source(crate::Exchange::Bitget, &["BTC".into(), "ETH".into()]).unwrap();
        assert!(crate::net::autorise(&s.url), "ws.bitget.com déjà dans la liste blanche");
        assert!(s.subscribe.contains(r#""instType":"USDT-FUTURES","channel":"trade","instId":"BTCUSDT""#));
        assert!(s.subscribe.contains(r#""instId":"ETHUSDT""#));
        assert!(branchee(crate::Exchange::Bitget));
    }

    /// 🏦 BloFin (perps USDT) : vrais messages du canal public `trades` relevés le 2026-11-15, lus par le décodeur des
    /// Baleines sans leur seuil. `side` = le côté du preneur : relevé avec le meilleur prix du carnet (`books5`), 44 achats
    /// sur 45 au prix vendeur et 3 ventes sur 4 au prix acheteur. `tradeId` toujours croissant ; `size` en contrats
    /// (0,001 BTC chacun), convertie en pièces : seulement pour les pièces dont la taille du contrat est connue.
    #[test]
    fn blofin_trades_reels_sens_numero_et_contrats() {
        let achat = r#"{"arg":{"channel":"trades","instId":"BTC-USDT"},"data":[{"instId":"BTC-USDT","tradeId":"51290192464","price":"64827.1","size":"20","side":"buy","ts":"1794774285056"}]}"#;
        let vente = r#"{"arg":{"channel":"trades","instId":"BTC-USDT"},"data":[{"instId":"BTC-USDT","tradeId":"51290198671","price":"64879.9","size":"11","side":"sell","ts":"1794774307077"}]}"#;
        let a = crate::whales::parse_all(achat);
        assert_eq!(
            (a[0].coin.as_str(), a[0].buy, a[0].price, a[0].time_ms, a[0].id, a[0].src),
            ("BTC", true, 64827.1, 1_794_774_285_056, Some(51_290_192_464), "BloFin"),
            "achat preneur"
        );
        assert!((a[0].size - 0.02).abs() < 1e-12, "20 contrats de 0,001 BTC : {}", a[0].size);
        let v = crate::whales::parse_all(vente);
        assert_eq!((v[0].buy, v[0].id), (false, Some(51_290_198_671)), "vente preneur");
        assert!(
            crate::whales::parse(achat).is_empty(),
            "le seuil des Baleines écarte ce petit trade ; ici, il compte"
        );
        assert!(crate::whales::parse_all(r#"{"event":"subscribe","arg":{"channel":"trades","instId":"BTC-USDT"}}"#).is_empty());
        let s = source(crate::Exchange::Blofin, &["BTC".into(), "ETH".into()]).unwrap();
        assert!(crate::net::autorise(&s.url), "openapi.blofin.com déjà dans la liste blanche");
        assert!(s.subscribe.contains(r#"{"channel":"trades","instId":"BTC-USDT"}"#) && s.subscribe.contains(r#""instId":"ETH-USDT""#));
        assert!(branchee(crate::Exchange::Blofin));
        // une pièce dont la taille du contrat est inconnue : pas de réel (l'estimé) ; ailleurs, toujours lisible
        assert!(reel_pour(crate::Exchange::Blofin, "BTC") && !reel_pour(crate::Exchange::Blofin, "ZZINCONNUE"));
        assert!(reel_pour(crate::Exchange::Bybit, "ZZINCONNUE") && reel_pour(crate::Exchange::BinanceSpot, "ZZINCONNUE"));
    }

    /// 🏦 WEEX (perps USDT) : vrais messages du canal public `btcusdt@trade` relevés le 2026-11-15, lus par son
    /// décodeur. `m` = l'acheteur est le teneur : relevé avec le carnet reconstruit (`btcusdt@depth15`), 143 achats
    /// preneur sur 158 au prix vendeur et 138 ventes sur 145 au prix acheteur. `q` en BTC (`v` = `p` × `q`) ; `t` est un
    /// texte (pas de numéro) ; un lot n'est pas trié par heure.
    #[test]
    fn weex_trades_reels_sens_sans_numero() {
        let lot = r#"{"e":"trade","E":1794775058373,"s":"BTCUSDT","d":[{"T":1794775058371,"t":"e23bb46c-ebc0-478f-9bff-5e8351511053","p":"65034.0","q":"0.0031","v":"201.60540","m":true},{"T":1794775057971,"t":"5d680fc4-21c1-4f55-b6a8-a74c1e3180cb","p":"65033.8","q":"0.0024","v":"156.08112","m":false},{"T":1794775058170,"t":"b7245466-e9be-4f2b-bc3f-a7f642e940d6","p":"65034.0","q":"0.0012","v":"78.04080","m":false}]}"#;
        let t = crate::whales::parse_all(lot);
        assert_eq!(
            t.iter()
                .map(|w| (w.coin.as_str(), w.buy, w.price, w.size, w.time_ms, w.id, w.src))
                .collect::<Vec<_>>(),
            [
                ("BTC", false, 65034.0, 0.0031, 1_794_775_058_371, None, "WEEX"),
                ("BTC", true, 65033.8, 0.0024, 1_794_775_057_971, None, "WEEX"),
                ("BTC", true, 65034.0, 0.0012, 1_794_775_058_170, None, "WEEX")
            ],
            "m: true = vente preneur ; m: false = achat preneur"
        );
        assert!(
            crate::whales::parse(lot).is_empty(),
            "le seuil des Baleines écarte ces petits trades ; ici, ils comptent"
        );
        for autre in [
            r#"{"cid":"b0e0510b-54d0-6f70-0b0f-a3b08b2fdc8f","event":"connected","time":"1794774999158"}"#,
            r#"{"result":true,"id":1}"#,
            r#"{"e":"trade","E":1,"s":"BTCUSDT","p":"1","q":"1","T":1}"#,
        ] {
            assert!(crate::whales::parse_all(autre).is_empty(), "{autre}");
        }
        let s = source(crate::Exchange::Weex, &["BTC".into(), "ETH".into()]).unwrap();
        assert!(crate::net::autorise(&s.url), "ws-contract.weex.com déjà dans la liste blanche");
        assert_eq!(s.subscribe, r#"{"method":"SUBSCRIBE","params":["btcusdt@trade","ethusdt@trade"],"id":1}"#);
        assert!(
            branchee(crate::Exchange::Weex) && reel_pour(crate::Exchange::Weex, "DOGE"),
            "en pièces : toujours lisible"
        );
    }

    /// 🏦 Gate.io (perps USDT, `futures.trades`, pas le spot des Baleines) : vrais messages relevés le 2026-11-15. Une
    /// taille positive = achat preneur, négative = vente preneur (avec le carnet `futures.book_ticker` juste avant : 70
    /// achats sur 82 au prix vendeur, 30 ventes sur 30 au prix acheteur) ; `size` en contrats de 0,0001 BTC (liste
    /// publique des contrats, `quanto_multiplier`) ; `id` toujours croissant.
    #[test]
    fn gate_trades_reels_sens_numero_et_contrats() {
        let vente = r#"{"time":1794775352,"time_ms":1794775352529,"channel":"futures.trades","event":"update","result":[{"id":881418367,"size":-4,"create_time":1794775352,"create_time_ms":1794775352529,"price":"65015","contract":"BTC_USDT"}]}"#;
        let achat = r#"{"time":1794775358,"time_ms":1794775358023,"channel":"futures.trades","event":"update","result":[{"id":881418369,"size":49,"create_time":1794775358,"create_time_ms":1794775358022,"price":"65015.1","contract":"BTC_USDT"}]}"#;
        let v = crate::whales::parse_all(vente);
        assert_eq!(
            (v[0].coin.as_str(), v[0].buy, v[0].price, v[0].time_ms, v[0].id, v[0].src),
            ("BTC", false, 65015.0, 1_794_775_352_529, Some(881_418_367), "Gate.io"),
            "taille négative : vente preneur"
        );
        assert!((v[0].size - 0.0004).abs() < 1e-12, "4 contrats de 0,0001 BTC : {}", v[0].size);
        let a = crate::whales::parse_all(achat);
        assert!(a[0].buy && (a[0].size - 0.0049).abs() < 1e-12, "{:?}", a[0]);
        // une pièce dont la taille du contrat est inconnue : ignorée (jamais une quantité fausse)
        assert!(crate::whales::parse_all(&achat.replace("BTC_USDT", "ZZGATE_USDT")).is_empty());
        assert!(crate::whales::parse_all(r#"{"time":1,"channel":"futures.trades","event":"subscribe","result":{"status":"success"}}"#).is_empty());
        // ses 🐋 Baleines écoutent le spot : leur top ne lit pas la liste des contrats (MEXC et BloFin, si)
        assert!(!crate::whales::baleines_en_contrats("Gate.io"));
        assert!(crate::whales::baleines_en_contrats("MEXC") && crate::whales::baleines_en_contrats("BloFin"));
        // la liste publique des contrats (extrait réel) : la liste elle-même, pas sous « data »
        let liste = r#"[{"name":"BTC_USDT","quanto_multiplier":"0.0001","type":"direct"},{"name":"ETH_USDT","quanto_multiplier":"0.01","type":"direct"}]"#;
        assert_eq!(
            crate::whaletop::parse_contracts("Gate.io", liste),
            [("BTC".to_string(), 0.0001), ("ETH".to_string(), 0.01)]
        );
        let s = source(crate::Exchange::Gate, &["BTC".into(), "ETH".into()]).unwrap();
        assert!(crate::net::autorise(&s.url) && s.url.contains("fx-ws"), "perps, déjà dans la liste blanche");
        assert!(s
            .subscribe
            .contains(r#""channel":"futures.trades","event":"subscribe","payload":["BTC_USDT","ETH_USDT"]"#));
        assert!(branchee(crate::Exchange::Gate) && reel_pour(crate::Exchange::Gate, "BTC") && !reel_pour(crate::Exchange::Gate, "ZZGATE"));
    }

    /// 🏦 MEXC (perps USDT, `sub.deal`) : vrais messages relevés le 2026-11-15, lus par le décodeur des Baleines sans leur
    /// seuil. `T` = 1 achat preneur, 2 vente preneur (avec le ticker `bid1` / `ask1` juste avant, qui n'arrive que toutes
    /// les ~2,5 s : 86 achats sur 93 au prix vendeur, 30 ventes sur 44 au prix acheteur) ; `v` en contrats de 0,0001 BTC
    /// (API publique des contrats) ; `i` toujours croissant, mais un lot va du plus récent au plus ancien.
    #[test]
    fn mexc_trades_reels_sens_numero_et_contrats() {
        let lot = r#"{"symbol":"BTC_USDT","data":[{"p":64995.6,"v":113,"T":1,"O":2,"M":2,"t":1794775693103,"i":"17037699914","cts":"1794775693103"},{"p":64995.5,"v":100,"T":2,"O":1,"M":2,"t":1794775693037,"i":"17037699907","cts":"1794775693037"}],"channel":"push.deal","ts":1794775693103}"#;
        let t = crate::whales::parse_all(lot);
        assert_eq!(
            t.iter().map(|w| (w.coin.as_str(), w.buy, w.price, w.time_ms, w.id, w.src)).collect::<Vec<_>>(),
            [
                ("BTC", true, 64995.6, 1_794_775_693_103, Some(17_037_699_914), "MEXC"),
                ("BTC", false, 64995.5, 1_794_775_693_037, Some(17_037_699_907), "MEXC")
            ],
            "T = 1 achat, 2 vente ; du plus récent au plus ancien"
        );
        assert!(
            (t[0].size - 0.0113).abs() < 1e-12 && (t[1].size - 0.01).abs() < 1e-12,
            "{} {}",
            t[0].size,
            t[1].size
        );
        // l'agrégateur compte les deux, même reçus à l'envers
        let mut agg = argos_core::orderflow::Agregateur::new(Some(1.0));
        let flux: Vec<_> = t
            .iter()
            .map(|w| argos_core::orderflow::TradeFlux {
                time_ms: w.time_ms,
                prix: w.price,
                taille: w.size,
                achat: w.buy,
                id: w.id,
            })
            .collect();
        agg.ajouter(&flux);
        assert!((agg.fermees(1_794_775_693_103 + 120_000)[0].delta() - 0.0013).abs() < 1e-12);
        assert!(
            crate::whales::parse(lot).is_empty(),
            "le seuil des Baleines écarte ces petits trades ; ici, ils comptent"
        );
        let s = source(crate::Exchange::Mexc, &["BTC".into(), "ETH".into()]).unwrap();
        assert!(
            crate::net::autorise(&s.url) && s.url.contains("contract.mexc.com"),
            "perps, déjà dans la liste blanche"
        );
        assert!(s.subscribe.contains(r#"{"method":"sub.deal","param":{"symbol":"BTC_USDT"}}"#) && s.subscribe.contains("ETH_USDT"));
        assert!(branchee(crate::Exchange::Mexc) && reel_pour(crate::Exchange::Mexc, "BTC") && !reel_pour(crate::Exchange::Mexc, "ZZMEXC"));
    }

    /// 🏦 KuCoin (spot, comme ses bougies et ses Baleines) : vrais messages de `/market/match:BTC-USDT` relevés le
    /// 2026-11-15, lus par le décodeur des Baleines sans leur seuil. `side` = le côté du preneur (avec le ticker juste
    /// avant : 16 achats sur 16 au prix vendeur, 267 ventes sur 341 au prix acheteur ; les autres, au-dessus du dernier
    /// carnet connu, dans une montée rapide) ; `tradeId` toujours croissant ; heure en ns ; taille en BTC. Son flux
    /// s'ouvre après un jeton public (`whales::kucoin_preparer`) : pas de `source` immédiate, mais branchée.
    #[test]
    fn kucoin_trades_reels_sens_et_numero() {
        let vente = r#"{"topic":"/market/match:BTC-USDT","type":"message","subject":"trade.l3match","data":{"makerOrderId":"511549248764932098","price":"65127.6","sequence":"25299163879325696","side":"sell","size":"0.00030434","symbol":"BTC-USDT","takerOrderId":"511549356621537281","time":"1794776072748000000","tradeId":"25299163879325696","type":"match"}}"#;
        let achat = r#"{"topic":"/market/match:BTC-USDT","type":"message","subject":"trade.l3match","data":{"makerOrderId":"511549363852525570","price":"65124.1","sequence":"25299164405841920","side":"buy","size":"0.01417901","symbol":"BTC-USDT","takerOrderId":"6afa1c0c9702e80007aea952","time":"1794776076974000000","tradeId":"25299164405841920","type":"match"}}"#;
        let v = crate::whales::parse_all(vente);
        assert_eq!(
            (v[0].coin.as_str(), v[0].buy, v[0].price, v[0].size, v[0].time_ms, v[0].id, v[0].src),
            ("BTC", false, 65127.6, 0.00030434, 1_794_776_072_748, Some(25_299_163_879_325_696), "KuCoin"),
            "vente preneur"
        );
        let a = crate::whales::parse_all(achat);
        assert_eq!((a[0].buy, a[0].id), (true, Some(25_299_164_405_841_920)), "achat preneur, numéro plus grand");
        assert!(
            crate::whales::parse(achat).is_empty(),
            "le seuil des Baleines écarte ce petit trade ; ici, il compte"
        );
        let s = crate::whales::kucoin_sub_at("wss://ws-api-spot.kucoin.com/?token=x&connectId=argos", &["BTC".into(), "ETH".into()]);
        assert!(crate::net::autorise(&s.url), "ws-api-spot.kucoin.com déjà dans la liste blanche");
        assert!(s.subscribe.contains("/market/match:BTC-USDT,ETH-USDT"));
        assert!(source(crate::Exchange::Kucoin, &["BTC".into()]).is_none(), "le flux attend son jeton");
        assert!(
            branchee(crate::Exchange::Kucoin) && reel_pour(crate::Exchange::Kucoin, "DOGE"),
            "en pièces : toujours lisible"
        );
    }

    /// 🏦 Coinbase (spot en dollars US, comme ses bougies et ses Baleines) : vrais messages du canal public `matches`
    /// relevés le 2026-11-15. `side` = le côté du **teneur** : le preneur fait l'inverse (avec le ticker juste avant :
    /// « buy » 585 fois sur 610 au prix acheteur, « sell » 811 fois sur 880 au prix vendeur) ; `trade_id` toujours
    /// croissant (1 490 trades), y compris `last_match`, envoyé à l'abonnement ; taille en BTC.
    #[test]
    fn coinbase_trades_reels_sens_et_numero() {
        let teneur_achete = r#"{"type":"match","trade_id":1130234654,"maker_order_id":"d266fecd-2c24-4fb1-8506-74d29f0d8cc2","taker_order_id":"02aeb562-b9a3-4e49-a4e2-41fa7fdcdd8a","side":"buy","size":"0.00062299","price":"65072.26","product_id":"BTC-USD","sequence":139512478702,"time":"2026-11-15T20:59:18.152420Z"}"#;
        let dernier = r#"{"type":"last_match","trade_id":1130234651,"maker_order_id":"b6c24989-f9df-463f-8eaa-3c7fd1367cef","taker_order_id":"7b7f8315-4b30-4d68-805b-5a7cb03e9f48","side":"sell","size":"0.00092143","price":"65072.27","product_id":"BTC-USD","sequence":139512478669,"time":"2026-11-15T20:59:18.025434Z"}"#;
        let v = crate::whales::parse_all(teneur_achete);
        assert_eq!(
            (v[0].coin.as_str(), v[0].buy, v[0].size, v[0].price, v[0].time_ms, v[0].id, v[0].src),
            ("BTC", false, 0.00062299, 65072.26, 1_794_776_358_152, Some(1_130_234_654), "Coinbase"),
            "le teneur achète : le preneur vend"
        );
        let a = crate::whales::parse_all(dernier);
        assert_eq!((a[0].buy, a[0].id), (true, Some(1_130_234_651)), "le teneur vend : le preneur achète");
        assert!(
            crate::whales::parse(teneur_achete).is_empty(),
            "le seuil des Baleines écarte ce petit trade ; ici, il compte"
        );
        assert!(crate::whales::parse_all(r#"{"type":"subscriptions","channels":[{"name":"matches","product_ids":["BTC-USD"]}]}"#).is_empty());
        let s = source(crate::Exchange::Coinbase, &["BTC".into(), "ETH".into()]).unwrap();
        assert!(crate::net::autorise(&s.url), "ws-feed.exchange.coinbase.com déjà dans la liste blanche");
        assert!(s.subscribe.contains(r#""BTC-USD""#) && s.subscribe.contains(r#""ETH-USD""#) && s.subscribe.contains("matches"));
        assert!(branchee(crate::Exchange::Coinbase));
    }

    /// 🏦 Kraken (spot en dollars US, comme ses bougies et ses Baleines) : vrais messages du canal public `trade` (v2)
    /// relevés le 2026-11-15. `side` = le côté du preneur (avec le ticker `bbo` juste avant : 40 achats sur 40 au prix
    /// vendeur, 28 ventes sur 28 au prix acheteur) ; `trade_id` toujours croissant ; `qty` en BTC.
    #[test]
    fn kraken_trades_reels_sens_et_numero() {
        let ventes = r#"{"channel":"trade","type":"update","data":[{"symbol":"BTC/USD","side":"sell","price":65037.9,"qty":0.00057659,"ord_type":"limit","trade_id":116217514,"timestamp":"2026-11-15T21:10:35.778589Z"},{"symbol":"BTC/USD","side":"sell","price":65037.9,"qty":0.00009941,"ord_type":"limit","trade_id":116217515,"timestamp":"2026-11-15T21:10:35.778589Z"}]}"#;
        let achat = r#"{"channel":"trade","type":"update","data":[{"symbol":"BTC/USD","side":"buy","price":65030.1,"qty":0.00130877,"ord_type":"limit","trade_id":116217520,"timestamp":"2026-11-15T21:10:38.321583Z"}]}"#;
        let v = crate::whales::parse_all(ventes);
        assert_eq!(
            v.iter().map(|w| (w.coin.as_str(), w.buy, w.size, w.time_ms, w.id)).collect::<Vec<_>>(),
            [
                ("BTC", false, 0.00057659, 1_794_777_035_778, Some(116_217_514)),
                ("BTC", false, 0.00009941, 1_794_777_035_778, Some(116_217_515))
            ],
            "ventes preneur, même instant, chacune son numéro"
        );
        let a = crate::whales::parse_all(achat);
        assert_eq!(
            (a[0].buy, a[0].price, a[0].time_ms, a[0].id, a[0].src),
            (true, 65030.1, 1_794_777_038_321, Some(116_217_520), "Kraken")
        );
        assert!(
            crate::whales::parse(achat).is_empty(),
            "le seuil des Baleines écarte ce petit trade ; ici, il compte"
        );
        assert!(crate::whales::parse_all(r#"{"channel":"heartbeat"}"#).is_empty());
        let s = source(crate::Exchange::Kraken, &["BTC".into(), "ETH".into()]).unwrap();
        assert!(crate::net::autorise(&s.url), "ws.kraken.com déjà dans la liste blanche");
        assert!(s.subscribe.contains(r#""channel":"trade","symbol":["BTC/USD","ETH/USD"]"#));
        assert!(branchee(crate::Exchange::Kraken));
    }

    /// 🏦 Aster (perps USDT, format Binance Futures) : vrais messages `btcusdt@aggTrade` relevés le 2026-11-15, lus par
    /// le décodeur des aggTrades de Binance. `m` = l'acheteur est le teneur (avec `bookTicker` juste avant : 8 achats
    /// preneur sur 8 au prix vendeur, 8 ventes sur 8 au prix acheteur ; Aster est peu actif) ; `a` toujours croissant.
    #[test]
    fn aster_trades_reels_sens_et_numero() {
        let vente =
            r#"{"e":"aggTrade","E":1794777399943,"a":89415148,"s":"BTCUSDT","p":"64923.8","q":"0.001","f":150044632,"l":150044632,"T":1794777399750,"m":true}"#;
        let achat = r#"{"e":"aggTrade","E":1794777400592,"a":89415149,"s":"BTCUSDT","p":"64923.9","q":"0.001","f":150044633,"l":150044633,"T":1794777400400,"m":false}"#;
        let v = crate::whales::parse_all(vente);
        assert_eq!(
            (v[0].coin.as_str(), v[0].buy, v[0].size, v[0].time_ms, v[0].id),
            ("BTC", false, 0.001, 1_794_777_399_750, Some(89_415_148)),
            "m = true : vente preneur"
        );
        let a = crate::whales::parse_all(achat);
        assert_eq!((a[0].buy, a[0].id), (true, Some(89_415_149)), "m = false : achat preneur");
        let s = source(crate::Exchange::Aster, &["BTC".into(), "ETH".into()]).unwrap();
        assert!(
            crate::net::autorise(&s.url) && s.url.contains("asterdex"),
            "fstream.asterdex.com déjà dans la liste blanche"
        );
        assert!(s.subscribe.contains(r#""btcusdt@aggTrade","ethusdt@aggTrade""#));
        assert_eq!(
            crate::whales::binance(&["BTC".into()]).url,
            "wss://fstream.binance.com/market/ws",
            "Binance inchangé"
        );
        assert!(branchee(crate::Exchange::Aster));
    }

    /// 🪙 Le spot de Binance, OKX, Bybit et Bitget (contre l'USDT) : vrais messages relevés le 2026-11-15, lus par les
    /// décodeurs de leurs perps. Sens du preneur vérifié : OKX 57 achats sur 59 au prix vendeur et 19 ventes sur 19 au
    /// prix acheteur ; Bybit 14 sur 14 et 156 sur 156 ; Bitget 13 sur 13 et 37 sur 37 ; Binance (son `bookTicker`
    /// spot n'a pas d'heure) par les rafales à la même milliseconde : prix qui monte 85 fois sur 85 avec `m: false`,
    /// qui descend 11 fois sur 11 avec `m: true`. Numéros : `a`, `tradeId`, et `i` de Bybit (un nombre sur le spot).
    #[test]
    fn spot_trades_reels_sens_et_numero() {
        let binance = r#"{"e":"aggTrade","E":1794777936143,"s":"BTCUSDT","a":4119310443,"p":"64946.00000000","q":"0.03079000","f":6865723117,"l":6865723117,"T":1794777936143,"m":true,"M":true}"#;
        let okx = r#"{"arg":{"channel":"trades","instId":"BTC-USDT"},"data":[{"instId":"BTC-USDT","tradeId":"1084602310","px":"65010.1","sz":"0.000011","side":"sell","ts":"1794777991338","count":"1","source":"0","seqId":84059999710}]}"#;
        let bybit = r#"{"topic":"publicTrade.BTCUSDT","ts":1794778026069,"type":"snapshot","data":[{"i":"2290000001268873363","T":1794778026067,"p":"65037.5","v":"0.00243","S":"Sell","seq":117649862813,"s":"BTCUSDT","BT":false,"RPI":false}]}"#;
        let bitget = r#"{"action":"update","arg":{"instType":"SPOT","channel":"trade","instId":"BTCUSDT"},"data":[{"ts":"1794778067467","price":"64997.9","size":"0.000001","side":"buy","tradeId":"1505973855492751362"},{"ts":"1794778067467","price":"64997.18","size":"0.000286","side":"buy","tradeId":"1505973855492751360"}],"ts":1794778067468}"#;
        let un = |m: &str| {
            crate::whales::parse_all(m)
                .into_iter()
                .map(|w| (w.coin, w.buy, w.size, w.id))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            un(binance),
            [("BTC".to_string(), false, 0.03079, Some(4_119_310_443))],
            "Binance : m = true, vente preneur"
        );
        assert_eq!(un(okx), [("BTC".to_string(), false, 0.000011, Some(1_084_602_310))], "OKX : vente preneur");
        assert_eq!(
            un(bybit),
            [("BTC".to_string(), false, 0.00243, Some(2_290_000_001_268_873_363))],
            "Bybit : i, un numéro"
        );
        assert_eq!(
            un(bitget),
            [
                ("BTC".to_string(), true, 0.000001, Some(1_505_973_855_492_751_362)),
                ("BTC".to_string(), true, 0.000286, Some(1_505_973_855_492_751_360))
            ],
            "Bitget spot : achats preneur, du plus récent au plus ancien"
        );
        let coins = ["BTC".to_string()];
        for (ex, adresse, morceau) in [
            (crate::Exchange::BinanceSpot, "stream.binance.com", r#""btcusdt@aggTrade""#),
            (crate::Exchange::OkxSpot, "ws.okx.com", r#""instId":"BTC-USDT"}"#),
            (crate::Exchange::BybitSpot, "/v5/public/spot", r#""publicTrade.BTCUSDT""#),
            (crate::Exchange::BitgetSpot, "ws.bitget.com", r#""instType":"SPOT""#),
        ] {
            let s = source(ex, &coins).unwrap();
            assert!(
                crate::net::autorise(&s.url) && s.url.contains(adresse) && s.subscribe.contains(morceau),
                "{ex:?} : {s:?}"
            );
        }
        // les perps restent sur leurs perps
        assert!(source(crate::Exchange::Bybit, &coins).unwrap().url.ends_with("/linear"));
        assert!(source(crate::Exchange::Bitget, &coins).unwrap().subscribe.contains("USDT-FUTURES"));
    }

    #[test]
    fn limites_appliquees() {
        let mut c = dossier("limites");
        let now = 1_800_000_000;
        let vieux = now - (GARDE_JOURS + 1) * 86_400;
        let recent = now - 3_600;
        c.ecrire(&cle(), &[minute(vieux, true), minute(recent, true)]).unwrap();
        c.limiter(now);
        assert_eq!(c.lire(&cle(), 0, i64::MAX), vec![minute(recent, true)], "plus de 30 jours : retiré");
        // taille : au-delà de la limite, les plus vieux fichiers partent d'abord (ici, limite à 0 : tout part)
        assert!(cache::evict(&c.dir, 0, Path::new("")) >= 1);
        assert!(c.lire(&cle(), 0, i64::MAX).is_empty());
        std::fs::remove_dir_all(&c.dir).ok();
    }
}
