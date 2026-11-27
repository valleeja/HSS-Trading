//! 🛡️ Liste blanche réseau : Argos ne parle qu'aux serveurs listés ici, en HTTPS ou WSS.
//! Chaque client (HTTP natif, `fetch` du navigateur, WebSocket, OANDA, Claude, logos) appelle `verifier` avant de partir.
//! Ajouter une source = ajouter son domaine exact ici (revue obligatoire : `GOVERNANCE.md`, zones sensibles).

use argos_core::{ArgosError, Result};

/// Domaines autorisés (nom exact, sans sous-domaine implicite).
pub const HOTES: &[&str] = &[
    // exchanges
    "fapi.binance.com",
    "api.binance.com",
    "fapi.asterdex.com",
    "fstream.asterdex.com",
    "stream.binance.com",
    "fstream.binance.com",
    "api.bybit.com",
    "stream.bybit.com",
    "ws.bitget.com",
    "ws-feed.exchange.coinbase.com",
    "ws.kraken.com",
    "api.gateio.ws",
    "contract.mexc.com",
    "api.huobi.pro",
    "fx-ws.gateio.ws",
    "api.bitget.com",
    "api.crypto.com",
    "api.exchange.coinbase.com",
    "api.kraken.com",
    "stream.crypto.com",
    "api.kucoin.com",
    "ws-api-spot.kucoin.com",
    "openapi.blofin.com",
    "www.okx.com",
    "ws.okx.com",
    "api-contract.weex.com",
    "ws-contract.weex.com",
    "fapi.bitunix.com",
    "www.bitstamp.net",
    "ws.bitstamp.net",
    "api.hyperliquid.xyz",
    // Bitfinex : API publique en lecture seule (HTTPS et WSS, même domaine ; jamais le domaine privé des ordres),
    // approuvée par Jee le 23 nov. 2026
    "api-pub.bitfinex.com",
    // DEX on-chain (gratuits, sans clé) : recherche de paires, chandelles, pools, trades
    "api.dexscreener.com",
    "api.geckoterminal.com",
    // forex, indices
    "api-fxtrade.oanda.com",
    "api-fxpractice.oanda.com",
    "query1.finance.yahoo.com",
    // marché, calendrier, actualités
    "api.coingecko.com",
    "api.alternative.me",
    "nfs.faireconomy.media",
    "feeds.content.dowjones.io",
    "www.coindesk.com",
    "cointelegraph.com",
    "decrypt.co",
    "cryptoslate.com",
    "bitcoinmagazine.com",
    "www.fxstreet.com",
    // nouvelles en français (Jee, 22 nov. 2026)
    "www.cointribune.com",
    "fr.beincrypto.com",
    "journalducoin.com",
    "invezz.com",
    // « Réel » du calendrier : BLS (API publique v1 sans clé) et la Fed
    "api.bls.gov",
    "www.federalreserve.gov",
    "s2.coinmarketcap.com",
    "pro-api.coinmarketcap.com",
    // assistant Claude, site d'Argos (packs de langue)
    "api.anthropic.com",
    "argos-smoky.vercel.app",
];

/// Machine locale (tests, `ARGOS_LANG_URL` vers un serveur local) : autorisée, même en clair.
const LOCAL: &[&str] = &["localhost", "127.0.0.1", "[::1]"];

/// Le domaine d'une adresse, si son schéma est connu : `("https", "api.bybit.com")`.
fn decouper(url: &str) -> Option<(&str, &str)> {
    let (schema, reste) = url.split_once("://")?;
    let autorite = reste.split(['/', '?', '#']).next()?;
    // « https://api.bybit.com@pirate.io » : l'hôte réel serait pirate.io ; on refuse tout identifiant dans l'adresse
    if autorite.contains('@') {
        return None;
    }
    let hote = if autorite.starts_with('[') {
        autorite.split_inclusive(']').next()?
    } else {
        autorite.split(':').next()?
    };
    Some((schema, hote))
}

/// `true` si Argos a le droit d'appeler cette adresse.
pub fn autorise(url: &str) -> bool {
    // navigateur : même origine que la page (`/api/oanda?…`), servie par le site lui-même
    if url.starts_with('/') && !url.starts_with("//") {
        return true;
    }
    let Some((schema, hote)) = decouper(url) else { return false };
    let hote = hote.to_ascii_lowercase();
    if LOCAL.contains(&hote.as_str()) {
        return matches!(schema, "http" | "https" | "ws" | "wss");
    }
    matches!(schema, "https" | "wss") && HOTES.contains(&hote.as_str())
}

/// Erreur claire si l'adresse n'est pas dans la liste blanche.
pub fn verifier(url: &str) -> Result<()> {
    if autorise(url) {
        Ok(())
    } else {
        Err(ArgosError::Feed(format!(
            "adresse refusée (hors liste blanche) : {}",
            url.chars().take(80).collect::<String>()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sources_connues_autorisees() {
        assert!(autorise("https://fapi.binance.com/fapi/v1/klines?symbol=BTCUSDT"));
        assert!(autorise("wss://ws.okx.com:8443/ws/v5/business"));
        assert!(autorise("https://API.BYBIT.COM/v5/market"));
        assert!(autorise("https://api.dexscreener.com/latest/dex/search?q=pepe"));
        assert!(autorise("https://api.geckoterminal.com/api/v2/networks/eth/pools/0x1/ohlcv/hour"));
        assert!(!autorise("https://api.geckoterminal.com.pirate.io/x"));
        assert!(autorise("/api/oanda?instrument=XAU_USD"));
        assert!(autorise("http://localhost:8000/lang/en.json"));
    }

    #[test]
    fn le_reste_est_refuse() {
        assert!(!autorise("http://fapi.binance.com/fapi/v1"), "en clair");
        assert!(!autorise("https://pirate.io/x"));
        assert!(!autorise("https://fapi.binance.com.pirate.io/x"), "faux sous-domaine");
        assert!(!autorise("https://evil.fapi.binance.com/x"), "sous-domaine non listé");
        assert!(!autorise("https://api.bybit.com@pirate.io/x"), "identifiant dans l'adresse");
        assert!(!autorise("//pirate.io/x"), "adresse sans schéma");
        assert!(!autorise("file:///etc/passwd"));
        assert!(!autorise("pas une adresse"));
    }

    /// Garde-fou : toute adresse écrite en dur dans `argos-data` doit être dans la liste blanche.
    #[test]
    fn toutes_les_sources_du_code_sont_listees() {
        // faux domaines des tests (jamais appelés : les tests utilisent un faux client)
        let faux = ["x.com", "y.com", "z.com"];
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut fichiers = vec![src.clone()];
        let mut vus = 0;
        while let Some(p) = fichiers.pop() {
            if p.is_dir() {
                fichiers.extend(std::fs::read_dir(&p).unwrap().map(|e| e.unwrap().path()));
                continue;
            }
            if p.extension().is_none_or(|e| e != "rs") || p.ends_with("net.rs") {
                continue;
            }
            let code = std::fs::read_to_string(&p).unwrap();
            for morceau in code.split('"').skip(1).step_by(2) {
                if !(morceau.starts_with("https://") || morceau.starts_with("wss://")) {
                    continue;
                }
                let Some((_, hote)) = decouper(morceau) else { continue };
                if hote.contains('{') || faux.contains(&hote) {
                    continue;
                }
                vus += 1;
                assert!(autorise(morceau), "{} : {morceau} n'est pas dans net::HOTES", p.display());
            }
        }
        assert!(vus > 20, "le garde-fou doit voir les adresses du code ({vus})");
    }
}
