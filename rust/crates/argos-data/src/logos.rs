//! 🪙 Logos des cryptos, pris sur CoinMarketCap (`s2.coinmarketcap.com/static/img/coins/64x64/<id>.png`).
//! CoinMarketCap range ses logos par numéro (id). La table ci-dessous fait le lien symbole → id pour tous les perps
//! d'Hyperliquid (sans clé d'API). Avec la clé CoinMarketCap de l'utilisateur (🔑 Clé API), la carte officielle
//! des ids (`/v1/cryptocurrency/map`, gardée 7 jours) couvre en plus n'importe quelle monnaie de base (Binance,
//! Bitstamp…). Un symbole sans id garde sa pastille de couleur. Les logos sont téléchargés une seule fois, puis
//! gardés dans `~/.cache/argos-hss-trading/logos/`.

/// Symboles Hyperliquid → id CoinMarketCap : tous les perps d'Hyperliquid (25 oct. 2026), chacun relié à la crypto
/// CoinMarketCap du même symbole la mieux classée (liste publique de CoinMarketCap, lue dans le navigateur de Jee).
const CMC_IDS: &[(&str, u32)] = &[
    ("BTC", 1),
    ("ETH", 1027),
    ("ATOM", 3794),
    ("DYDX", 28324),
    ("SOL", 5426),
    ("AVAX", 5805),
    ("BNB", 1839),
    ("APE", 18876),
    ("OP", 11840),
    ("LTC", 2),
    ("ARB", 11841),
    ("DOGE", 74),
    ("INJ", 7226),
    ("SUI", 20947),
    ("PEPE", 24478),
    ("CRV", 6538),
    ("LDO", 8000),
    ("LINK", 1975),
    ("STX", 4847),
    ("CFX", 7334),
    ("GMX", 11857),
    ("SNX", 2586),
    ("XRP", 52),
    ("BCH", 1831),
    ("APT", 21794),
    ("AAVE", 7278),
    ("COMP", 5692),
    ("WLD", 13502),
    ("YGG", 10688),
    ("TRX", 1958),
    ("SHIB", 5994),
    ("UNI", 7083),
    ("SEI", 23149),
    ("RUNE", 4157),
    ("ZRO", 26997),
    ("DOT", 6636),
    ("BANANA", 28066),
    ("TRB", 4944),
    ("BIGTIME", 28230),
    ("KAS", 20396),
    ("BLUR", 23121),
    ("TIA", 22861),
    ("BSV", 3602),
    ("ADA", 2010),
    ("MINA", 8646),
    ("POLYX", 20362),
    ("GAS", 1785),
    ("PENDLE", 9481),
    ("FET", 3773),
    ("NEAR", 6535),
    ("MEME", 28301),
    ("ORDI", 25028),
    ("NEO", 1376),
    ("ZEN", 1698),
    ("FIL", 2280),
    ("PYTH", 28177),
    ("SUSHI", 6758),
    ("IMX", 10603),
    ("BONK", 23095),
    ("GMT", 18069),
    ("SUPER", 8290),
    ("JUP", 29210),
    ("LUNC", 4172),
    ("RSR", 3964),
    ("GALA", 7080),
    ("JTO", 28541),
    ("ACE", 28674),
    ("WIF", 28752),
    ("CAKE", 7186),
    ("PEOPLE", 14806),
    ("ENS", 13855),
    ("ETC", 1321),
    ("XAI", 28933),
    ("MANTA", 13631),
    ("UMA", 5617),
    ("ONDO", 21159),
    ("ALT", 29073),
    ("ZETA", 21259),
    ("DYM", 28932),
    ("W", 29587),
    ("STRK", 22691),
    ("TAO", 22974),
    ("AR", 5632),
    ("FLOKI", 10804),
    ("BOME", 29870),
    ("ETHFI", 29814),
    ("ENA", 30171),
    ("MNT", 27075),
    ("TNSR", 30449),
    ("SAGA", 30372),
    ("MERL", 30712),
    ("HBAR", 4642),
    ("POPCAT", 28782),
    ("EIGEN", 30494),
    ("REZ", 30843),
    ("NOT", 28850),
    ("TURBO", 24911),
    ("BRETT", 29743),
    ("IO", 29835),
    ("ZK", 24091),
    ("RENDER", 5690),
    ("POL", 28321),
    ("CELO", 5567),
    ("HMSTR", 32195),
    ("NEIRO", 32521),
    ("GOAT", 33440),
    ("MOODENG", 33093),
    ("GRASS", 32956),
    ("PURR", 34332),
    ("PNUT", 33788),
    ("XLM", 512),
    ("SAND", 6210),
    ("IOTA", 1720),
    ("ALGO", 4030),
    ("HYPE", 32196),
    ("ME", 32197),
    ("MOVE", 32452),
    ("VIRTUAL", 29420),
    ("PENGU", 34466),
    ("USUAL", 33979),
    ("FARTCOIN", 33597),
    ("AIXBT", 34103),
    ("BIO", 34812),
    ("GRIFFAIN", 34792),
    ("SPX", 28081),
    ("S", 32684),
    ("MORPHO", 34104),
    ("TRUMP", 35336),
    ("MELANIA", 35347),
    ("ANIME", 35319),
    ("VVV", 35509),
    ("BERA", 24647),
    ("LAYER", 35429),
    ("KAITO", 35763),
    ("NIL", 35702),
    ("PAXG", 4705),
    ("BABY", 32198),
    ("WCT", 33152),
    ("HYPER", 36281),
    ("ZORA", 35931),
    ("INIT", 33120),
    ("NXPC", 36044),
    ("SOPH", 32087),
    ("RESOLV", 36713),
    ("SYRUP", 33824),
    ("PUMP", 36507),
    ("PROVE", 37593),
    ("XPL", 36645),
    ("WLFI", 33251),
    ("LINEA", 27657),
    ("SKY", 33038),
    ("ASTER", 36341),
    ("AVNT", 38299),
    ("STBL", 38359),
    ("0G", 38337),
    ("HEMI", 38159),
    ("APEX", 19843),
    ("2Z", 38515),
    ("ZEC", 1437),
    ("MON", 30495),
    ("MET", 38353),
    ("MEGA", 38770),
    ("CC", 37263),
    ("ICP", 8916),
    ("AERO", 29270),
    ("STABLE", 38892),
    ("FOGO", 38356),
    ("LIT", 39125),
    ("XMR", 328),
    ("AXS", 6783),
    ("DASH", 131),
    ("SKR", 39377),
    ("AZTEC", 39521),
    ("CHIP", 39870),
    ("GRAM", 11419),
    ("CASHCAT", 40809),
    ("PONS", 40938),
    ("USELESS", 36828),
];

/// Monnaie de base sans son multiplicateur : « kPEPE » (Hyperliquid), « 1000PEPE », « 1000000MOG », « 1MBABYDOGE »
/// (Binance) → « PEPE », « MOG », « BABYDOGE ». HIP-3 (`dex:…`) et séries (`cap:…`) : aucune.
pub fn base_nue(coin: &str) -> Option<&str> {
    if coin.is_empty() || coin.contains(':') {
        return None;
    }
    let majuscule = |r: &str| r.chars().next().is_some_and(|c| c.is_ascii_uppercase());
    let nue = ["1000000", "1000", "1M", "k"]
        .iter()
        .find_map(|p| coin.strip_prefix(p).filter(|r| majuscule(r)));
    Some(nue.unwrap_or(coin))
}

/// Id CoinMarketCap de la table intégrée (sans clé).
pub fn cmc_id(coin: &str) -> Option<u32> {
    let base = base_nue(coin)?;
    CMC_IDS.iter().find(|(s, _)| *s == base).map(|(_, id)| *id)
}

/// Carte officielle symbole → id de CoinMarketCap (`/v1/cryptocurrency/map`, avec la clé de l'utilisateur).
pub type Carte = std::collections::HashMap<String, u32>;

/// Durée de vie de la carte gardée sur disque : 7 jours.
pub const CARTE_VALIDE_S: i64 = 7 * 86_400;

/// Id CoinMarketCap : la table intégrée d'abord, sinon la carte (si elle est là).
pub fn id_de(coin: &str, carte: Option<&Carte>) -> Option<u32> {
    cmc_id(coin).or_else(|| carte?.get(base_nue(coin)?).copied())
}

/// Vaut-il la peine de chercher un logo ? Oui si la table le connaît, ou avec la clé pour toute monnaie au symbole
/// simple (lettres et chiffres).
pub fn demandable(coin: &str, avec_cle: bool) -> bool {
    cmc_id(coin).is_some() || (avec_cle && base_nue(coin).is_some_and(|b| b.chars().all(|c| c.is_ascii_alphanumeric())))
}

/// Adresse de la carte officielle (la clé part dans l'en-tête `X-CMC_PRO_API_KEY`, jamais dans l'adresse).
pub fn carte_url(base: &str) -> String {
    format!("{base}/v1/cryptocurrency/map?listing_status=active&aux=is_active")
}

/// Lit la réponse de `/v1/cryptocurrency/map` : pour chaque symbole, la crypto la mieux classée (`rank` le plus
/// petit). Lecture tolérante : une entrée sans `id` ou sans `symbol` est sautée.
pub fn lire_carte(body: &str) -> Result<Carte, String> {
    if let Some(e) = crate::cmcap::error_of(body) {
        return Err(e);
    }
    let v: serde_json::Value = serde_json::from_str(body).map_err(|e| format!("carte CoinMarketCap illisible : {e}"))?;
    let data = v["data"].as_array().ok_or("carte CoinMarketCap : pas de liste « data »")?;
    let mut meilleurs: std::collections::HashMap<String, (f64, u32)> = Default::default();
    for c in data {
        let id = c["id"].as_u64().and_then(|i| u32::try_from(i).ok());
        let (Some(sym), Some(id)) = (c["symbol"].as_str(), id) else { continue };
        let rang = c["rank"].as_f64().filter(|r| *r > 0.0).unwrap_or(f64::MAX);
        let e = meilleurs.entry(sym.to_uppercase()).or_insert((rang, id));
        if rang < e.0 {
            *e = (rang, id);
        }
    }
    Ok(meilleurs.into_iter().map(|(s, (_, id))| (s, id)).collect())
}

/// Crédits pris par un appel (`status.credit_count`, 1 si absent).
pub fn credits_pris(body: &str) -> u32 {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v["status"]["credit_count"].as_u64())
        .and_then(|n| u32::try_from(n).ok())
        .unwrap_or(1)
}

/// Relit la carte gardée sur disque si elle a moins de 7 jours.
pub fn carte_du_disque(fichier: &std::path::Path, maintenant_s: i64) -> Option<Carte> {
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(fichier).ok()?).ok()?;
    let carte: Carte = serde_json::from_value(v["ids"].clone()).ok()?;
    let t = v["t"].as_i64()?;
    (maintenant_s - t < CARTE_VALIDE_S && !carte.is_empty()).then_some(carte)
}

/// Garde la carte sur disque avec son heure.
pub fn garder_carte(fichier: &std::path::Path, carte: &Carte, maintenant_s: i64) -> std::io::Result<()> {
    if let Some(d) = fichier.parent() {
        std::fs::create_dir_all(d)?;
    }
    std::fs::write(fichier, serde_json::json!({ "t": maintenant_s, "ids": carte }).to_string())
}

/// Adresse du logo 64 × 64.
pub fn url(id: u32) -> String {
    format!("https://s2.coinmarketcap.com/static/img/coins/64x64/{id}.png")
}

/// Dossier des logos : `$XDG_CACHE_HOME/argos-hss-trading/logos`, sinon `~/.cache/argos-hss-trading/logos` (à côté
/// du cache des bougies, dans le dossier propre à Argos HSS Trading).
pub fn dir() -> Option<std::path::PathBuf> {
    crate::cache::dir().and_then(|d| d.parent().map(|p| p.join("logos")))
}

#[cfg(feature = "native")]
pub use native::{spawn, Chercheur};

/// Route du serveur du PC qui donne les logos à la page.
pub const ROUTE_LOGOS: &str = "/api/logos";
/// Bases demandées d'un coup au serveur, au plus.
pub const MAX_PAR_DEMANDE: usize = 64;

/// 🌐 Navigateur : les logos par le serveur du PC ([`ROUTE_LOGOS`], [`MAX_PAR_DEMANDE`] bases par demande). Mêmes
/// arguments que `spawn` du desktop.
#[cfg(all(feature = "web", not(feature = "native")))]
pub fn par_le_serveur(coins: Vec<(String, String)>, on_logo: impl Fn(String, Vec<u8>) -> bool + Send + 'static) {
    crate::rt::spawn(move || async move {
        use crate::http::Http;
        let c = crate::rt::Client::default();
        for lot in coins.chunks(MAX_PAR_DEMANDE) {
            let mut bases: Vec<&str> = lot.iter().map(|(_, b)| b.as_str()).collect();
            bases.sort_unstable();
            bases.dedup();
            let Ok(rep) = c.post_json(ROUTE_LOGOS, &serde_json::json!({ "bases": bases }).to_string()).await else {
                return;
            };
            let Ok(v) = serde_json::from_str::<serde_json::Value>(&rep) else { return };
            for (coin, base) in lot {
                if let Some(png) = v[base.as_str()].as_array() {
                    let png = png.iter().filter_map(|x| x.as_u64().and_then(|n| u8::try_from(n).ok())).collect();
                    if !on_logo(coin.clone(), png) {
                        return;
                    }
                }
            }
        }
    });
}

#[cfg(feature = "native")]
mod native {
    use super::*;

    /// Heure du dernier appel de la carte (secondes) : pas plus d'un par heure, même s'il a échoué.
    static DERNIER_APPEL: std::sync::Mutex<i64> = std::sync::Mutex::new(0);

    fn maintenant_s() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() as i64)
    }

    /// La carte des ids, seulement avec une clé réglée : celle du disque (moins de 7 jours), sinon CoinMarketCap
    /// (1 crédit réservé au compteur, plus ce que CoinMarketCap dit avoir pris), puis gardée sur disque.
    async fn carte_avec_cle() -> Option<Carte> {
        let (base, cle, credits) = crate::cmcap::reglage()?;
        let maintenant = maintenant_s();
        let fichier = dir().map(|d| d.join("carte-cmc.json"));
        if let Some(c) = fichier.as_deref().and_then(|f| carte_du_disque(f, maintenant)) {
            return Some(c);
        }
        {
            let mut d = DERNIER_APPEL.lock().ok()?;
            if maintenant - *d < 3600 {
                return None;
            }
            *d = maintenant;
        }
        if !crate::cmcap::reserve(&credits, 1, maintenant) {
            return None;
        }
        let corps = crate::cmcap::get(&carte_url(&base), &cle).await.ok()?;
        let en_plus = credits_pris(&corps).saturating_sub(1);
        if en_plus > 0 {
            if let Ok(mut c) = credits.lock() {
                c.spend(en_plus);
            }
        }
        let carte = lire_carte(&corps).ok()?;
        if let Some(f) = &fichier {
            let _ = garder_carte(f, &carte, maintenant);
        }
        Some(carte)
    }

    /// Cherche les logos les uns après les autres (disque d'abord, sinon CoinMarketCap, puis gardé sur disque) ; la
    /// carte des ids n'est lue qu'une fois, au premier symbole absent de la table. Partagé par l'app de bureau et le
    /// serveur local (logos de la page).
    pub struct Chercheur {
        client: reqwest::Client,
        dossier: Option<std::path::PathBuf>,
        carte: Option<Option<Carte>>,
    }

    impl Default for Chercheur {
        fn default() -> Self {
            Self {
                client: reqwest::Client::new(),
                dossier: dir(),
                carte: None,
            }
        }
    }

    impl Chercheur {
        /// Le logo (PNG) de ce marché, s'il a un id CoinMarketCap.
        pub async fn png(&mut self, base: &str) -> Option<Vec<u8>> {
            if cmc_id(base).is_none() && self.carte.is_none() {
                self.carte = Some(carte_avec_cle().await);
            }
            let id = id_de(base, self.carte.as_ref().and_then(Option::as_ref))?;
            let fichier = self.dossier.as_ref().map(|d| d.join(format!("{id}.png")));
            if let Some(png) = fichier.as_ref().and_then(|f| std::fs::read(f).ok()) {
                return Some(png);
            }
            if !crate::net::autorise(&url(id)) {
                return None;
            }
            let octets = self.client.get(url(id)).send().await.ok()?.error_for_status().ok()?.bytes().await.ok()?;
            if let Some(f) = &fichier {
                let _ = f.parent().map(std::fs::create_dir_all);
                let _ = std::fs::write(f, &octets);
            }
            Some(octets.to_vec())
        }
    }

    /// Va chercher les logos de ces marchés dans un fil à part. `on_logo(coin, png)` reçoit chaque logo ; arrêt quand
    /// il renvoie `false`. `coins` : (clé rendue avec le logo, marché dont on cherche le logo).
    pub fn spawn(coins: Vec<(String, String)>, on_logo: impl Fn(String, Vec<u8>) -> bool + Send + 'static) {
        crate::rt::spawn(move || async move {
            let mut chercheur = Chercheur::default();
            for (coin, base) in coins {
                if let Some(png) = chercheur.png(&base).await {
                    if !on_logo(coin, png) {
                        return;
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_coinmarketcap() {
        assert_eq!(cmc_id("BTC"), Some(1));
        assert_eq!(cmc_id("ETH"), Some(1027));
        assert_eq!(cmc_id("HYPE"), Some(32196));
        assert_eq!(cmc_id("kPEPE"), Some(24478), "kPEPE = 1000 PEPE");
        assert_eq!(cmc_id("1000PEPE"), Some(24478), "1000PEPE de Binance = PEPE");
        assert_eq!(cmc_id("xyz:NVDA"), None, "HIP-3 : pas de logo crypto");
        assert_eq!(cmc_id("INCONNU"), None);
        assert_eq!(url(1), "https://s2.coinmarketcap.com/static/img/coins/64x64/1.png");
        let mut seen = std::collections::HashSet::new();
        assert!(CMC_IDS.iter().all(|(s, _)| seen.insert(*s)), "un seul id par symbole");
    }

    #[test]
    fn base_sans_multiplicateur() {
        assert_eq!(base_nue("1000000MOG"), Some("MOG"));
        assert_eq!(base_nue("1MBABYDOGE"), Some("BABYDOGE"));
        assert_eq!(base_nue("1000SATS"), Some("SATS"));
        assert_eq!(base_nue("1INCH"), Some("1INCH"), "1INCH n'a pas de multiplicateur");
        assert_eq!(base_nue("kBONK"), Some("BONK"));
        assert_eq!(base_nue("cap:USDT.D"), None);
        assert_eq!(base_nue(""), None);
    }

    /// Réponse d'exemple au format de la documentation officielle (`/v1/cryptocurrency/map`) ; ids et noms fictifs
    /// (sauf BTC = 1).
    const CARTE: &str = r#"{"status":{"timestamp":"2026-11-15T12:00:00.000Z","error_code":0,"error_message":null,
        "elapsed":10,"credit_count":1,"notice":null},"data":[
        {"id":1,"rank":1,"name":"Bitcoin","symbol":"BTC","slug":"bitcoin","is_active":1},
        {"id":50001,"rank":40,"name":"Exemple Or","symbol":"XAUT","slug":"exemple-or","is_active":1},
        {"id":99999,"rank":3000,"name":"Faux Tether Gold","symbol":"XAUT","slug":"faux","is_active":1},
        {"id":77777,"name":"Sans rang","symbol":"ZZZ","slug":"zzz"},
        {"id":"illisible","symbol":"BAD"},
        {"rank":5,"symbol":"SANSID"}]}"#;

    #[test]
    fn carte_officielle_lue() {
        let c = lire_carte(CARTE).unwrap();
        assert_eq!(c.get("XAUT"), Some(&50001), "le mieux classé gagne");
        assert_eq!(c.get("ZZZ"), Some(&77777), "sans rang : gardé quand même");
        assert!(!c.contains_key("BAD") && !c.contains_key("SANSID"), "entrées incomplètes sautées");
        assert_eq!(credits_pris(CARTE), 1);
        assert_eq!(credits_pris("{}"), 1);
        let erreur = r#"{"status":{"error_code":1001,"error_message":"This API Key is invalid.","credit_count":0}}"#;
        assert_eq!(lire_carte(erreur), Err("This API Key is invalid.".to_string()));
        assert!(lire_carte("pas du json").is_err());
    }

    #[test]
    fn table_dabord_puis_carte() {
        let c = lire_carte(CARTE).unwrap();
        assert_eq!(id_de("BTC", None), Some(1));
        assert_eq!(id_de("XAUT", None), None, "sans clé : la table seule");
        assert_eq!(id_de("XAUT", Some(&c)), Some(50001));
        assert_eq!(id_de("xyz:XAUT", Some(&c)), None);
        assert!(demandable("BTC", false) && demandable("1000PEPE", false));
        assert!(!demandable("XAUT", false) && demandable("XAUT", true));
        assert!(!demandable("cap:USDT.D", true) && !demandable("币安人生", true));
    }

    #[test]
    fn carte_gardee_sept_jours() {
        let f = argos_core::dossiers::de_test("logos-carte").join("carte-cmc.json");
        let c = lire_carte(CARTE).unwrap();
        garder_carte(&f, &c, 1_000).unwrap();
        assert_eq!(carte_du_disque(&f, 1_000 + CARTE_VALIDE_S - 1), Some(c));
        assert_eq!(carte_du_disque(&f, 1_000 + CARTE_VALIDE_S), None, "trop vieille : redemandée");
    }

    #[test]
    fn adresse_de_la_carte_autorisee() {
        let u = carte_url(crate::cmcap::DEFAULT_URL);
        assert_eq!(u, "https://pro-api.coinmarketcap.com/v1/cryptocurrency/map?listing_status=active&aux=is_active");
        assert!(crate::net::autorise(&u));
    }
}
