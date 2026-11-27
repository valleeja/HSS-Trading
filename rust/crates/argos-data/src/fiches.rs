//! 📇 Fiches des sources vérifiées (Binance, Hyperliquid, Bitstamp) : des **données**, pas du code. Pour chaque
//! marché : comment lister les symboles, lire les prix, lire les bougies (et combien par appel), le flux en direct,
//! et les limites. Les adresses sont exactement celles que les modules d'exchange appellent déjà (un test le vérifie,
//! et que chacune passe la liste blanche du réseau). Lecture seule : une source ne passe jamais d'ordre, et aucune
//! clé n'est dans une fiche.

use argos_core::catalogue::{EtatSource, Origine, Source, TypeMarche, TypeSource};

/// Méthode HTTP d'un appel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Methode {
    Get,
    /// POST avec un corps JSON (Hyperliquid).
    PostJson,
}

/// Un appel de l'API publique ; `{BASE}`, `{UT}`, `{N}`, `{FIN}` sont remplacés par le module de l'exchange.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Appel {
    pub methode: Methode,
    pub adresse: &'static str,
    /// Corps JSON (POST) ; vide pour un GET.
    pub corps: &'static str,
    /// Nombre d'éléments au plus par appel (bougies) ; 0 : tout d'un coup.
    pub max: usize,
}

/// Un marché d'une source (spot ou perp).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FicheMarche {
    pub marche: TypeMarche,
    pub catalogue: Appel,
    pub prix: Appel,
    pub bougies: Appel,
    /// Flux en direct (WSS).
    pub direct: &'static str,
}

/// La fiche d'une source vérifiée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fiche {
    /// Comme dans les identifiants TradingView (« BINANCE »).
    pub id: &'static str,
    pub nom: &'static str,
    pub marches: &'static [FicheMarche],
    /// Réponse 429 / 503 : attente du `Retry-After`, au plus (secondes).
    pub attente_max_s: u64,
    /// Les prix de la watchlist se relisent au plus toutes les … secondes.
    pub prix_toutes_s: u64,
}

const fn get(adresse: &'static str, max: usize) -> Appel {
    Appel {
        methode: Methode::Get,
        adresse,
        corps: "",
        max,
    }
}

const fn post(adresse: &'static str, corps: &'static str, max: usize) -> Appel {
    Appel {
        methode: Methode::PostJson,
        adresse,
        corps,
        max,
    }
}

const HL: &str = "https://api.hyperliquid.xyz/info";

/// Les sources vérifiées, dans l'ordre choisi par Jee : Binance, Hyperliquid, Bitstamp.
pub const FICHES: [Fiche; 3] = [
    Fiche {
        id: "BINANCE",
        nom: "Binance",
        marches: &[
            FicheMarche {
                marche: TypeMarche::Perp,
                catalogue: get("https://fapi.binance.com/fapi/v1/exchangeInfo", 0),
                prix: get("https://fapi.binance.com/fapi/v1/ticker/24hr", 0),
                bougies: get("https://fapi.binance.com/fapi/v1/klines?symbol={BASE}USDT&interval={UT}&limit={N}{FIN}", 1500),
                direct: "wss://fstream.binance.com/market/ws",
            },
            FicheMarche {
                marche: TypeMarche::Spot,
                catalogue: get(
                    "https://api.binance.com/api/v3/exchangeInfo?permissions=SPOT&showPermissionSets=false&symbolStatus=TRADING",
                    0,
                ),
                prix: get("https://api.binance.com/api/v3/ticker/24hr", 0),
                bougies: get("https://api.binance.com/api/v3/klines?symbol={BASE}USDT&interval={UT}&limit={N}{FIN}", 1000),
                direct: "wss://stream.binance.com:9443/ws",
            },
        ],
        attente_max_s: crate::http::MAX_RETRY_AFTER_S,
        prix_toutes_s: crate::prices::EVERY_S,
    },
    Fiche {
        id: "HYPERLIQUID",
        nom: "Hyperliquid",
        marches: &[FicheMarche {
            marche: TypeMarche::Perp,
            catalogue: post(HL, r#"{"type":"meta"}"#, 0),
            prix: post(HL, r#"{"type":"metaAndAssetCtxs"}"#, 0),
            bougies: post(
                HL,
                r#"{"type":"candleSnapshot","req":{"coin":"{BASE}","interval":"{UT}","startTime":…,"endTime":…}}"#,
                5000,
            ),
            direct: "wss://api.hyperliquid.xyz/ws",
        }],
        attente_max_s: crate::http::MAX_RETRY_AFTER_S,
        prix_toutes_s: crate::prices::EVERY_S,
    },
    Fiche {
        id: "BITSTAMP",
        nom: "Bitstamp",
        marches: &[FicheMarche {
            marche: TypeMarche::Spot,
            catalogue: get("https://www.bitstamp.net/api/v2/markets/", 0),
            prix: get("https://www.bitstamp.net/api/v2/ticker/", 0),
            bougies: get("https://www.bitstamp.net/api/v2/ohlc/{BASE}usd/?step={UT}&limit={N}{FIN}", 1000),
            // pas de flux de bougies : elles se construisent à partir des trades en direct
            direct: "wss://ws.bitstamp.net",
        }],
        attente_max_s: crate::http::MAX_RETRY_AFTER_S,
        prix_toutes_s: crate::prices::EVERY_S,
    },
];

impl Fiche {
    /// Toutes les adresses de la fiche (sans doublon), dans l'ordre.
    pub fn adresses(&self) -> Vec<&'static str> {
        let mut v: Vec<&'static str> = vec![];
        for m in self.marches {
            for a in [m.catalogue.adresse, m.prix.adresse, m.bougies.adresse, m.direct] {
                // l'adresse sans ses paramètres (`?…`) ni ses gabarits (`{…}`)
                let base = a.split(['?', '{']).next().unwrap_or(a);
                if !v.contains(&base) {
                    v.push(base);
                }
            }
        }
        v
    }

    /// La source du catalogue synchronisé (intégrée, active).
    pub fn source(&self) -> Source {
        Source {
            id: self.id.to_string(),
            nom: self.nom.to_string(),
            sorte: TypeSource::Crypto,
            origine: Origine::Integree,
            adresses: self.adresses().into_iter().map(str::to_string).collect(),
            etat: EtatSource::Active,
        }
    }
}

/// 🗂️ Toutes les sources que l'app connaît, comme éléments « intégrés » du catalogue : chaque exchange (une source
/// par nom TradingView : Binance perps et spot = BINANCE), OANDA et CoinMarketCap (CRYPTOCAP). Les adresses viennent
/// de la fiche vérifiée quand il y en a une (Binance, Hyperliquid, Bitstamp) ; sinon la liste est vide (fiche à venir).
#[cfg(test)]
pub fn sources_integrees() -> Vec<Source> {
    let mut v: Vec<Source> = vec![];
    for ex in crate::Exchange::ALL {
        let id = crate::tv::symbole_id(ex, "BTC").source;
        if v.iter().any(|s| s.id == id) {
            continue;
        }
        v.push(match FICHES.iter().find(|f| f.id == id) {
            Some(f) => f.source(),
            None => Source {
                id,
                nom: ex.parent().name().to_string(),
                sorte: TypeSource::Crypto,
                origine: Origine::Integree,
                adresses: vec![],
                etat: EtatSource::Active,
            },
        });
    }
    for (id, nom, sorte) in [("OANDA", "OANDA", TypeSource::Forex), ("CRYPTOCAP", "CoinMarketCap", TypeSource::Macro)] {
        v.push(Source {
            id: id.to_string(),
            nom: nom.to_string(),
            sorte,
            origine: Origine::Integree,
            adresses: vec![],
            etat: EtatSource::Active,
        });
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exchanges::{binance, bitstamp, hyperliquid, spot, Request};
    use argos_core::Timeframe;

    fn url(r: &Request) -> &str {
        match r {
            Request::Get(u) => u,
            Request::PostJson { url, .. } => url,
        }
    }

    /// Le début d'un gabarit (avant le premier `{`).
    fn debut(a: &str) -> &str {
        a.split('{').next().unwrap_or(a)
    }

    #[test]
    fn toutes_les_adresses_passent_la_liste_blanche() {
        for f in FICHES {
            assert!(!f.marches.is_empty(), "{}", f.id);
            for a in f.adresses() {
                assert!(crate::net::autorise(a), "{} : {a}", f.id);
                assert!(matches!(a.split_once("://").map(|x| x.0), Some("https" | "wss")), "{a}");
            }
            let s = f.source();
            assert_eq!((s.origine, s.etat), (Origine::Integree, EtatSource::Active));
        }
        assert_eq!(FICHES.map(|f| f.id), ["BINANCE", "HYPERLIQUID", "BITSTAMP"], "l'ordre choisi par Jee");
    }

    /// Les fiches disent exactement ce que les modules d'exchange appellent (aucune adresse inventée).
    #[test]
    fn fiches_conformes_aux_modules() {
        let tf = Timeframe::H1;
        let [bn, hl, bs] = FICHES;
        let (perp, spot_bn) = (&bn.marches[0], &bn.marches[1]);
        assert_eq!(url(&binance::instruments_request()), perp.catalogue.adresse);
        assert_eq!(url(&binance::markets_request()), perp.prix.adresse);
        assert!(url(&binance::candles_request("BTC", tf, 10, None).unwrap()).starts_with(debut(perp.bougies.adresse)));
        assert_eq!(binance::live_sub("BTC", tf).url, perp.direct);
        assert_eq!(url(&spot::instruments_request(spot::Venue::Binance)), spot_bn.catalogue.adresse);
        assert_eq!(url(&spot::markets_request(spot::Venue::Binance)), spot_bn.prix.adresse);
        assert!(url(&spot::candles_request(spot::Venue::Binance, "BTC", tf, 10, None).unwrap()).starts_with(debut(spot_bn.bougies.adresse)));
        assert_eq!(spot::live_sub(spot::Venue::Binance, "BTC", tf).map(|s| s.url), Some(spot_bn.direct.to_string()));

        let m = &hl.marches[0];
        for (r, a) in [
            (hyperliquid::instruments_request(""), m.catalogue),
            (hyperliquid::markets_request(), m.prix),
            (hyperliquid::candles_request("BTC", tf, 10, 3_600_000_000, None).unwrap(), m.bougies),
        ] {
            let Request::PostJson { url, body } = r else {
                panic!("Hyperliquid : POST attendu")
            };
            assert_eq!(url, a.adresse);
            assert!(body.starts_with(debut(a.corps).trim_end_matches('"')), "{body} / {}", a.corps);
        }

        let m = &bs.marches[0];
        assert_eq!(url(&bitstamp::instruments_request()), m.catalogue.adresse);
        assert_eq!(url(&bitstamp::markets_request()), m.prix.adresse);
        assert!(url(&bitstamp::candles_request("BTC", tf, 10, None).unwrap()).starts_with(debut(m.bougies.adresse)));
        assert_eq!(bitstamp::trades_sub(&["BTC".to_string()]).url, m.direct);
        // maximum de bougies par appel : celui des modules
        assert!(url(&binance::candles_request("BTC", tf, 99_999, None).unwrap()).contains(&format!("limit={}", perp.bougies.max)));
        assert!(url(&bitstamp::candles_request("BTC", tf, 99_999, None).unwrap()).contains(&format!("limit={}", m.bougies.max)));
    }

    #[test]
    fn toutes_les_sources_de_l_app_sont_integrees() {
        let v = sources_integrees();
        let ids: Vec<&str> = v.iter().map(|s| s.id.as_str()).collect();
        for id in ["BINANCE", "HYPERLIQUID", "BITSTAMP", "OKX", "BYBIT", "OANDA", "CRYPTOCAP"] {
            assert!(ids.contains(&id), "{id} manque : {ids:?}");
        }
        assert_eq!(ids.iter().filter(|i| **i == "BINANCE").count(), 1, "une seule source BINANCE (perps et spot)");
        assert!(v
            .iter()
            .all(|s| s.origine == Origine::Integree && s.adresses.iter().all(|a| crate::net::autorise(a))));
        assert!(
            !v.iter().find(|s| s.id == "BINANCE").unwrap().adresses.is_empty(),
            "la fiche vérifiée donne ses adresses"
        );
    }
}
