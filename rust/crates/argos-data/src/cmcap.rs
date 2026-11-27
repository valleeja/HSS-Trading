//! 📈 CoinMarketCap (clé de l'utilisateur, plan gratuit) : dominances BTC.D, ETH.D, USDT.D, USDC.D et capitalisations
//! TOTAL, TOTAL2, TOTAL3, cherchables comme `CRYPTOCAP:USDT.D`.
//!
//! L'historique de ces séries est payant chez CoinMarketCap : Argos garde lui-même ses relevés dans le stockage local
//! et dessine le graphique avec ce qu'il a accumulé (il commence vide). Sans clé, ces relevés viennent gratuitement de
//! CoinGecko (marché global, au plus un point toutes les 5 min quand le panneau Marché ou une série est ouvert).
//!
//! Avec une clé (🔑 Clés API), **un relevé par jour** par défaut (réglable), plus un au démarrage si le dernier est
//! plus vieux, et le bouton ⟳. Un relevé = 4 crédits (plan gratuit) :
//! - `/v1/global-metrics/quotes/latest` : capitalisation totale, parts de BTC et d'ETH, volume 24 h ;
//! - `/v2/cryptocurrency/quotes/latest?symbol=USDT,USDC` : capitalisation de USDT et USDC (USDT.D, USDC.D) ;
//! - `/v3/fear-and-greed/latest` : peur & avidité de CoinMarketCap ;
//! - `/v1/cryptocurrency/listings/latest?limit=80` : saison des altcoins.
//!
//! Soit ~120 crédits par mois. Le compteur du mois (`Credits`) ne laisse jamais passer plus de 10 000.

use argos_core::Bar;
use serde_json::Value;

/// Adresse de l'API quand l'utilisateur n'en donne pas.
pub const DEFAULT_URL: &str = "https://pro-api.coinmarketcap.com";
/// 🛰️ Relais du serveur du PC pour la page (PC et cell) : `/api/cmc/<chemin de l'API>` ; le serveur y met la clé de
/// son trousseau et ses propres paramètres (liste fermée : [`relais_permis`]).
pub const RELAIS: &str = "/api/cmc";

/// Clé CoinMarketCap plausible : 8 à 200 caractères, sans espace (elle se copie au complet depuis
/// pro.coinmarketcap.com ▸ API Key).
pub fn cle_valide(cle: &str) -> bool {
    (8..=200).contains(&cle.len()) && !cle.contains(char::is_whitespace)
}

/// Message lisible d'une erreur du serveur du PC (« … HTTP 502 : {"error": "…"} ») : seulement le texte de `error`,
/// sinon le message tel quel.
pub fn erreur_du_relais(message: &str) -> String {
    message
        .find('{')
        .and_then(|i| serde_json::from_str::<Value>(&message[i..]).ok())
        .and_then(|v| v["error"].as_str().map(String::from))
        .unwrap_or_else(|| message.to_string())
}

/// Les appels permis par le relais : (chemin du relais, sans requête ; adresse complète chez CoinMarketCap). La
/// page ne choisit que le chemin : les paramètres sont toujours ceux d'Argos.
pub fn relais_permis() -> Vec<(String, String)> {
    let adresses: [fn(&str) -> String; 5] = [global_url, stables_url, fng_url, top_url, crate::logos::carte_url];
    adresses
        .iter()
        .map(|f| {
            let complete = f(DEFAULT_URL);
            let chemin = complete[DEFAULT_URL.len()..].split('?').next().unwrap_or("").to_string();
            (format!("{RELAIS}{chemin}"), complete)
        })
        .collect()
}
/// Pas de l'historique (5 min) : unité de temps la plus fine des séries CRYPTOCAP:.
pub const EVERY_S: i64 = 300;
/// Relevé CoinMarketCap par défaut : une fois par jour.
pub const DEFAULT_EVERY_S: i64 = 86_400;
/// Points gardés au plus (~70 jours à 5 min).
pub const MAX_POINTS: usize = 20_000;

/// Les séries offertes : (nom après `CRYPTOCAP:`, description).
pub const SERIES: [(&str, &str); 7] = [
    ("BTC.D", "Dominance BTC (%)"),
    ("ETH.D", "Dominance ETH (%)"),
    ("USDT.D", "Dominance USDT (%)"),
    ("USDC.D", "Dominance USDC (%)"),
    ("TOTAL", "Capitalisation crypto totale ($)"),
    ("TOTAL2", "Capitalisation sans BTC ($)"),
    ("TOTAL3", "Capitalisation sans BTC ni ETH ($)"),
];

/// Un relevé du marché.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct CapPoint {
    /// Heure du relevé (secondes).
    pub t: i64,
    /// Capitalisation totale ($).
    pub total: f64,
    /// Parts en % de la capitalisation totale.
    pub btc_d: f64,
    pub eth_d: f64,
    /// `None` : capitalisation des stablecoins pas encore reçue.
    #[serde(default)]
    pub usdt_d: Option<f64>,
    #[serde(default)]
    pub usdc_d: Option<f64>,
    /// Volume 24 h de tout le marché ($) et variation de la capitalisation totale sur 24 h (%) : pour le panneau
    /// Marché crypto (sans crédit en plus).
    #[serde(default)]
    pub volume: Option<f64>,
    #[serde(default)]
    pub chg_24h: Option<f64>,
    /// Peur & avidité et saison des altcoins de CoinMarketCap (relevé avec clé seulement).
    #[serde(default)]
    pub fng: Option<(u32, String)>,
    #[serde(default)]
    pub alt_season: Option<u32>,
    /// 🌐 Market caps of the top 80 (symbol in capitals → $), read from the same « listings » answer as the altcoin
    /// season (no credit more): they fill the Cap column where CoinGecko has nothing. Not kept in the history.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub caps: std::collections::HashMap<String, f64>,
}

impl CapPoint {
    /// Valeur d'une série (« USDT.D », « TOTAL3 »…).
    pub fn value(&self, series: &str) -> Option<f64> {
        let v = match series {
            "BTC.D" => self.btc_d,
            "ETH.D" => self.eth_d,
            "USDT.D" => self.usdt_d?,
            "USDC.D" => self.usdc_d?,
            "TOTAL" => self.total,
            // exact : la part de BTC (et d'ETH) est une part de la capitalisation totale
            "TOTAL2" => self.total * (1.0 - self.btc_d / 100.0),
            "TOTAL3" => self.total * (1.0 - (self.btc_d + self.eth_d) / 100.0),
            _ => return None,
        };
        v.is_finite().then_some(v)
    }
}

/// Crédits du plan gratuit par mois : jamais dépassés.
pub const CREDITS_CAP: u32 = 10_000;

/// 🧮 Compteur des crédits CoinMarketCap du mois (local, remis à zéro le 1er du mois, UTC). Partagé par le relevé
/// CRYPTOCAP: et le panneau Marché crypto ; gardé par l'app dans `cmc_credits.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct Credits {
    /// Mois du compte (année × 100 + mois, ex. 202611).
    pub month: i64,
    pub used: u32,
}

/// Le compteur partagé entre les fils.
pub type CreditBox = std::sync::Arc<std::sync::Mutex<Credits>>;

/// (adresse de l'API, clé, compteur) de CoinMarketCap, réglé par l'app quand une clé est mise (`None` : pas de clé).
/// Partagé par les chandelles DEX et les logos.
pub type Reglage = Option<(String, String, CreditBox)>;
static REGLAGE: std::sync::Mutex<Reglage> = std::sync::Mutex::new(None);

/// Règle (ou retire) la clé CoinMarketCap.
pub fn regler(r: Reglage) {
    if let Ok(mut m) = REGLAGE.lock() {
        *m = r;
    }
}

/// La clé CoinMarketCap réglée, s'il y en a une.
pub fn reglage() -> Reglage {
    REGLAGE.lock().ok().and_then(|r| r.clone())
}

/// Mois (année × 100 + mois, UTC) de l'heure `t` (secondes).
pub fn month_of(t: i64) -> i64 {
    // jours depuis 1970 → date civile (algorithme de Howard Hinnant)
    let z = t.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    y * 100 + m
}

impl Credits {
    /// Nouveau mois : remis à zéro.
    pub fn roll(&mut self, t: i64) {
        let m = month_of(t);
        if m != self.month {
            *self = Credits { month: m, used: 0 };
        }
    }
    /// Peut-on dépenser `n` crédits sans dépasser le plafond ?
    pub fn can_spend(&self, n: u32) -> bool {
        self.used + n <= CREDITS_CAP
    }
    pub fn spend(&mut self, n: u32) {
        self.used = (self.used + n).min(CREDITS_CAP);
    }
    /// Texte du panneau 🔌 API.
    pub fn label(&self) -> String {
        format!("crédits CMC ce mois : {} / {}", self.used, CREDITS_CAP)
    }
}

/// Réserve `n` crédits dans le compteur (mois à jour) ; `false` : plafond atteint, rien ne part.
pub fn reserve(credits: &CreditBox, n: u32, t: i64) -> bool {
    let Ok(mut c) = credits.lock() else { return false };
    c.roll(t);
    if !c.can_spend(n) {
        return false;
    }
    c.spend(n);
    true
}

/// Capitalisation de USDT et USDC ($).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Stables {
    pub usdt: Option<f64>,
    pub usdc: Option<f64>,
}

/// Nom de série d'un texte tapé : « CRYPTOCAP:USDT.D », « usdt.d », « TOTAL3 » → « USDT.D », « TOTAL3 ».
pub fn series_of(input: &str) -> Option<&'static str> {
    let s = input.trim().to_uppercase();
    let s = s.strip_prefix("CRYPTOCAP:").unwrap_or(&s);
    SERIES.iter().map(|(n, _)| *n).find(|n| *n == s)
}

/// Séries dont le nom commence par ce texte (« CRYPTOCAP: » seul = toutes, « US » = USDT.D, USDC.D).
pub fn series_matching(input: &str) -> Vec<&'static str> {
    let s = input.trim().to_uppercase();
    let (s, prefixed) = match s.strip_prefix("CRYPTOCAP:") {
        Some(r) => (r.to_string(), true),
        None => (s, false),
    };
    if s.is_empty() && !prefixed {
        return vec![];
    }
    // sans « CRYPTOCAP: », seulement ce qui ne peut pas être une crypto : « USDT.D », « BTC.D », « TOTAL3 »
    let clear = prefixed || s.contains('.') || s.starts_with("TOTAL");
    SERIES.iter().map(|(n, _)| *n).filter(|n| clear && n.starts_with(&s)).collect()
}

/// Identifiant d'une série dans une watchlist : « cap:USDT.D ».
pub fn id(series: &str) -> String {
    format!("cap:{series}")
}

/// Relit `id` : la série.
pub fn parse_id(id: &str) -> Option<&'static str> {
    series_of(id.strip_prefix("cap:")?)
}

/// Adresse de l'API : vide = celle de CoinMarketCap ; sinon HTTPS seulement (la liste blanche réseau vérifie aussi
/// l'hôte à chaque appel).
pub fn base_url(custom: &str) -> argos_core::Result<String> {
    let u = custom.trim().trim_end_matches('/');
    if u.is_empty() {
        return Ok(DEFAULT_URL.into());
    }
    let secure = u.strip_prefix("https").is_some_and(|r| r.starts_with("://"));
    if !secure || u.contains('@') || u.contains(char::is_whitespace) {
        return Err("lien CoinMarketCap refusé : HTTPS seulement (lien sécurisé)".into());
    }
    Ok(u.to_string())
}

pub fn global_url(base: &str) -> String {
    format!("{base}/v1/global-metrics/quotes/latest?convert=USD")
}

pub fn fng_url(base: &str) -> String {
    format!("{base}/v3/fear-and-greed/latest")
}

pub fn top_url(base: &str) -> String {
    format!("{base}/v1/cryptocurrency/listings/latest?limit=80&sort=market_cap&convert=USD")
}

/// Peur & avidité de CoinMarketCap (`/v3/fear-and-greed/latest`) : valeur et mot en français.
pub fn parse_fng(body: &str) -> Option<(u32, String)> {
    let v: Value = serde_json::from_str(body).ok()?;
    let d = &v["data"];
    let val = f(&d["value"])? as u32;
    let label = match d["value_classification"].as_str().unwrap_or("") {
        "Extreme fear" | "Extreme Fear" => "Peur extrême",
        "Fear" => "Peur",
        "Neutral" => "Neutre",
        "Greed" => "Avidité",
        "Extreme greed" | "Extreme Greed" => "Avidité extrême",
        other => other,
    };
    Some((val, label.to_string()))
}

/// 🌐 Market caps from the top 80 of CoinMarketCap (`/v1/cryptocurrency/listings/latest`): symbol (capitals) → $; a
/// symbol carried by several tokens: the biggest cap wins (as with CoinGecko).
pub fn parse_caps(body: &str) -> std::collections::HashMap<String, f64> {
    let mut caps = std::collections::HashMap::new();
    let Ok(v) = serde_json::from_str::<Value>(body) else { return caps };
    for c in v["data"].as_array().into_iter().flatten() {
        let (Some(s), Some(m)) = (c["symbol"].as_str(), f(&c["quote"]["USD"]["market_cap"])) else {
            continue;
        };
        if m > 0.0 {
            let e = caps.entry(s.to_uppercase()).or_insert(0.0);
            *e = f64::max(*e, m);
        }
    }
    caps
}

/// The CoinGecko caps `cg` (they stay) plus CoinMarketCap's `cmc` for the symbols CoinGecko doesn't give (Jee's rule:
/// CMC adds, never removes nor replaces).
pub fn caps_avec_cmc(cg: &std::collections::HashMap<String, f64>, cmc: &std::collections::HashMap<String, f64>) -> std::collections::HashMap<String, f64> {
    let mut out = cg.clone();
    for (k, v) in cmc {
        out.entry(k.clone()).or_insert(*v);
    }
    out
}

/// Saison des altcoins depuis le top 80 de CoinMarketCap : part du top 50 (hors BTC et stablecoins) qui fait mieux
/// que BTC sur 30 jours, sur 100 (même règle que le panneau Marché crypto avec CoinGecko).
pub fn parse_alt_season(body: &str) -> Option<u32> {
    let v: Value = serde_json::from_str(body).ok()?;
    let list: Vec<Value> = v["data"]
        .as_array()?
        .iter()
        .map(|c| {
            let id = if c["symbol"] == "BTC" {
                "bitcoin".to_string()
            } else {
                format!("cmc{}", f(&c["id"]).unwrap_or(0.0))
            };
            serde_json::json!({
                "id": id,
                "symbol": c["symbol"].as_str().unwrap_or("").to_lowercase(),
                "name": c["name"],
                "price_change_percentage_30d_in_currency": c["quote"]["USD"]["percent_change_30d"],
            })
        })
        .collect();
    crate::market::parse_alt_season(&Value::Array(list).to_string())
}

pub fn stables_url(base: &str) -> String {
    format!("{base}/v2/cryptocurrency/quotes/latest?symbol=USDT,USDC&convert=USD")
}

fn f(v: &Value) -> Option<f64> {
    v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

/// Message d'erreur de CoinMarketCap (`status.error_message`), s'il y en a un.
pub fn error_of(body: &str) -> Option<String> {
    let v: Value = serde_json::from_str(body).ok()?;
    let st = &v["status"];
    let code = st["error_code"].as_i64().unwrap_or(0);
    (code != 0).then(|| st["error_message"].as_str().unwrap_or("erreur inconnue").to_string())
}

/// Lit `/v1/global-metrics/quotes/latest` ; les dominances USDT et USDC viennent de `stables` (capitalisation de
/// USDT et USDC ÷ capitalisation totale).
pub fn parse_global(body: &str, stables: Stables, t: i64) -> argos_core::Result<CapPoint> {
    if let Some(e) = error_of(body) {
        return Err(e.into());
    }
    let v: Value = serde_json::from_str(body).map_err(|_| "réponse CoinMarketCap illisible".to_string())?;
    let d = &v["data"];
    let total = f(&d["quote"]["USD"]["total_market_cap"]).ok_or("capitalisation totale absente de la réponse")?;
    let btc_d = f(&d["btc_dominance"]).ok_or("dominance BTC absente de la réponse")?;
    let eth_d = f(&d["eth_dominance"]).ok_or("dominance ETH absente de la réponse")?;
    if total <= 0.0 {
        return Err("capitalisation totale nulle".into());
    }
    let share = |cap: Option<f64>| cap.map(|c| c / total * 100.0);
    Ok(CapPoint {
        t,
        total,
        btc_d,
        eth_d,
        usdt_d: share(stables.usdt),
        usdc_d: share(stables.usdc),
        volume: f(&d["quote"]["USD"]["total_volume_24h"]),
        chg_24h: f(&d["quote"]["USD"]["total_market_cap_yesterday_percentage_change"]),
        fng: None,
        alt_season: None,
        caps: Default::default(),
    })
}

/// Lit `/v2/cryptocurrency/quotes/latest?symbol=USDT,USDC` (le plus gros jeton de chaque symbole).
pub fn parse_stables(body: &str) -> argos_core::Result<Stables> {
    if let Some(e) = error_of(body) {
        return Err(e.into());
    }
    let v: Value = serde_json::from_str(body).map_err(|_| "réponse CoinMarketCap illisible".to_string())?;
    let cap = |sym: &str| {
        let d = &v["data"][sym];
        // v2 : une liste par symbole (plusieurs jetons peuvent avoir le même) ; v1 : un objet
        let list: Vec<&Value> = d.as_array().map_or(vec![d], |a| a.iter().collect());
        list.iter()
            .filter_map(|x| f(&x["quote"]["USD"]["market_cap"]))
            .fold(None, |m: Option<f64>, c| Some(m.map_or(c, |m| m.max(c))))
    };
    Ok(Stables {
        usdt: cap("USDT"),
        usdc: cap("USDC"),
    })
}

/// Ajoute un relevé à l'historique : remplace celui de moins d'1 min avant (pas de doublon), garde l'ordre et
/// au plus `MAX_POINTS`.
pub fn push(history: &mut Vec<CapPoint>, mut p: CapPoint) {
    // (the market caps are for the Cap column, not the history)
    p.caps.clear();
    if let Some(last) = history.last_mut() {
        if p.t < last.t {
            return;
        }
        if p.t - last.t < 60 {
            *last = p;
            return;
        }
    }
    history.push(p);
    if history.len() > MAX_POINTS {
        let extra = history.len() - MAX_POINTS;
        history.drain(..extra);
    }
}

/// Bougies de 5 min d'une série, à partir de l'historique : ouverture = valeur précédente, clôture = valeur du
/// relevé (pas de volume). Les relevés d'une même tranche de 5 min font une seule bougie.
pub fn bars(history: &[CapPoint], series: &str) -> Vec<Bar> {
    let mut out: Vec<Bar> = vec![];
    let mut prev: Option<f64> = None;
    for p in history {
        let Some(v) = p.value(series) else { continue };
        let time = p.t - p.t.rem_euclid(EVERY_S);
        match out.last_mut().filter(|b| b.time == time) {
            Some(b) => {
                b.high = b.high.max(v);
                b.low = b.low.min(v);
                b.close = v;
            }
            None => {
                let open = prev.unwrap_or(v);
                out.push(Bar {
                    time,
                    open,
                    high: open.max(v),
                    low: open.min(v),
                    close: v,
                    volume: 0.0,
                });
            }
        }
        prev = Some(v);
    }
    out
}

/// GET avec la clé (en-tête `X-CMC_PRO_API_KEY`, jamais dans l'adresse ni dans les journaux).
#[cfg(feature = "native")]
pub async fn get(url: &str, key: &str) -> Result<String, String> {
    crate::net::verifier(url).map_err(|e| e.to_string())?;
    let c = reqwest::Client::builder()
        .user_agent("Argos/2.0")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    let r = c
        .get(url)
        .header("X-CMC_PRO_API_KEY", key)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|_| "CoinMarketCap ne répond pas".to_string())?;
    let status = r.status();
    let body = r.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(error_of(&body).unwrap_or_else(|| format!("HTTP {status}")));
    }
    Ok(body)
}

/// Navigateur : GET par le relais du serveur du PC (`url` commence par [`RELAIS`]) ; la clé reste au trousseau du
/// PC, la page n'en a pas (`_key` vide).
#[cfg(all(feature = "web", not(feature = "native")))]
pub(crate) async fn get(url: &str, _key: &str) -> Result<String, String> {
    use crate::http::Http;
    let body = crate::rt::Client::default().get(url).await.map_err(|e| erreur_du_relais(&e.to_string()))?;
    match error_of(&body) {
        Some(e) => Err(e),
        None => Ok(body),
    }
}

/// Un relevé complet (4 appels au plus : `n_calls` du compteur déjà réservés ; 2 = sans peur & avidité ni saison).
#[cfg(any(feature = "native", feature = "web"))]
pub async fn fetch(base: &str, key: &str, n_calls: u32, t: i64) -> argos_core::Result<CapPoint> {
    let stables = get(&stables_url(base), key)
        .await
        .and_then(|b| parse_stables(&b).map_err(String::from))
        .unwrap_or_default();
    let mut p = parse_global(&get(&global_url(base), key).await?, stables, t)?;
    if n_calls >= 4 {
        p.fng = get(&fng_url(base), key).await.ok().and_then(|b| parse_fng(&b));
        let top = get(&top_url(base), key).await.ok();
        p.alt_season = top.as_deref().and_then(parse_alt_season);
        p.caps = top.as_deref().map(parse_caps).unwrap_or_default();
    }
    Ok(p)
}

/// Teste la clé : la capitalisation totale du moment, ou l'erreur de CoinMarketCap.
#[cfg(any(feature = "native", feature = "web"))]
pub async fn test(base: &str, key: &str) -> argos_core::Result<f64> {
    parse_global(&get(&global_url(base), key).await?, Stables::default(), 0).map(|p| p.total)
}

/// Réglage du fil de fond : (adresse de l'API, clé) ; `None` = pas de clé, le fil attend.
pub type Wanted = std::sync::Arc<std::sync::Mutex<Option<(String, String)>>>;

/// Réglages du relevé : intervalle (secondes, 24 h par défaut) et demande manuelle (bouton ⟳).
#[derive(Debug)]
pub struct Ctl {
    pub every_s: std::sync::atomic::AtomicI64,
    pub force: std::sync::atomic::AtomicBool,
}

impl Default for Ctl {
    fn default() -> Self {
        Self {
            every_s: std::sync::atomic::AtomicI64::new(DEFAULT_EVERY_S),
            force: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

/// Un relevé est-il dû ? (`last` : heure du dernier relevé gardé, `now`, intervalle, demande manuelle, clé changée)
pub fn due(last: Option<i64>, now: i64, every_s: i64, force: bool, key_changed: bool) -> bool {
    force || key_changed || last.is_none_or(|t| now - t >= every_s.max(3600))
}

/// Lance le relevé de fond tant qu'une clé est réglée : au démarrage si le dernier relevé (`last`) est plus vieux que
/// l'intervalle, puis à chaque intervalle (24 h par défaut), et quand ⟳ est demandé. Chaque appel prend 1 crédit au
/// compteur `credits` ; plafond atteint : rien ne part jusqu'au 1er du mois. `on_point` reçoit chaque relevé (ou
/// l'erreur) ; le fil s'arrête quand il renvoie `false`.
#[cfg(any(feature = "native", feature = "web"))]
pub fn spawn(
    wanted: Wanted,
    credits: CreditBox,
    ctl: std::sync::Arc<Ctl>,
    mut last: Option<i64>,
    now_s: fn() -> i64,
    on_point: impl Fn(Result<CapPoint, String>) -> bool + Send + 'static,
) {
    use std::sync::atomic::Ordering;
    crate::rt::spawn(move || async move {
        let mut last_cfg: Option<(String, String)> = None;
        loop {
            let cfg = wanted.lock().ok().and_then(|w| w.clone());
            let now = now_s();
            if let Some((base, key)) = cfg.clone() {
                // nouvelle clé (pas au démarrage) : un relevé tout de suite
                let changed = last_cfg.is_some() && cfg != last_cfg;
                last_cfg = cfg;
                let force = ctl.force.swap(false, Ordering::SeqCst);
                if due(last, now, ctl.every_s.load(Ordering::SeqCst), force, changed) {
                    last = Some(now);
                    // 🧮 4 crédits si possible, sinon les 2 indispensables, sinon rien (plafond du mois)
                    let n = if reserve(&credits, 4, now) {
                        4
                    } else if reserve(&credits, 2, now) {
                        2
                    } else {
                        0
                    };
                    let res = if n == 0 {
                        Err(format!("plafond de {CREDITS_CAP} crédits du mois atteint : prochain relevé le 1er"))
                    } else {
                        fetch(&base, &key, n, now).await.map_err(String::from)
                    };
                    if !on_point(res) {
                        return;
                    }
                }
            } else {
                last_cfg = None;
            }
            crate::rt::sleep(std::time::Duration::from_secs(2)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const GLOBAL: &str = r#"{"status":{"error_code":0,"error_message":null,"credit_count":1},
        "data":{"btc_dominance":60.0,"eth_dominance":10.0,"active_cryptocurrencies":9000,
        "quote":{"USD":{"total_market_cap":3000000000000,"total_volume_24h":100000000000,"altcoin_market_cap":1200000000000}}}}"#;
    const STABLES: &str = r#"{"status":{"error_code":0},"data":{
        "USDT":[{"symbol":"USDT","quote":{"USD":{"market_cap":150000000000}}},{"symbol":"USDT","quote":{"USD":{"market_cap":1000}}}],
        "USDC":[{"symbol":"USDC","quote":{"USD":{"market_cap":60000000000}}}]}}"#;

    #[test]
    fn releve_et_calculs() {
        let st = parse_stables(STABLES).unwrap();
        assert_eq!(st.usdt, Some(150e9), "le plus gros jeton du symbole");
        let p = parse_global(GLOBAL, st, 1000).unwrap();
        assert_eq!(p.value("BTC.D"), Some(60.0));
        assert_eq!(p.value("ETH.D"), Some(10.0));
        assert_eq!(p.value("TOTAL"), Some(3e12));
        assert!((p.value("TOTAL2").unwrap() - 1.2e12).abs() < 1.0, "sans BTC");
        assert!((p.value("TOTAL3").unwrap() - 0.9e12).abs() < 1.0, "sans BTC ni ETH");
        assert!((p.value("USDT.D").unwrap() - 5.0).abs() < 1e-9);
        assert!((p.value("USDC.D").unwrap() - 2.0).abs() < 1e-9);
        let sans = parse_global(GLOBAL, Stables::default(), 0).unwrap();
        assert_eq!(sans.value("USDT.D"), None, "pas encore de cap. USDT : pas de point inventé");
    }

    /// 🛰️ Relais du serveur du PC : 5 chemins fermés (les paramètres restent ceux d'Argos), clé plausible, erreur lisible.
    #[test]
    fn relais_du_serveur() {
        let r = relais_permis();
        assert_eq!(r.len(), 5);
        assert!(r.contains(&("/api/cmc/v1/global-metrics/quotes/latest".into(), global_url(DEFAULT_URL))));
        assert!(r.contains(&("/api/cmc/v1/cryptocurrency/map".into(), crate::logos::carte_url(DEFAULT_URL))));
        assert!(r.iter().all(|(c, a)| !c.contains('?') && a.starts_with(DEFAULT_URL) && crate::net::autorise(a)));
        assert!(cle_valide("cle-de-test-1234") && !cle_valide("court") && !cle_valide("avec espace 123"));
        assert_eq!(
            erreur_du_relais(r#"source de données : HTTP 502 : {"error": "This API Key is invalid."}"#),
            "This API Key is invalid."
        );
        assert_eq!(erreur_du_relais("PC injoignable"), "PC injoignable");
    }

    /// 🌐 Market caps with the CMC key (Jee, 22 Nov): read from the top 80 answer (biggest token of a symbol), added
    /// only where CoinGecko has none; never kept in the history.
    #[test]
    fn market_caps_de_coinmarketcap() {
        let top = r#"{"status":{"error_code":0},"data":[
            {"id":1,"symbol":"BTC","quote":{"USD":{"market_cap":1.9e12,"percent_change_30d":1.0}}},
            {"id":2,"symbol":"NEWC","quote":{"USD":{"market_cap":5.0e8}}},
            {"id":3,"symbol":"newc","quote":{"USD":{"market_cap":1.0e6}}},
            {"id":4,"symbol":"NOCAP","quote":{"USD":{"market_cap":null}}}]}"#;
        let cmc = parse_caps(top);
        assert_eq!(cmc.get("NEWC"), Some(&5.0e8), "the biggest token of the symbol");
        assert!(!cmc.contains_key("NOCAP"));
        assert!(parse_caps("pas du json").is_empty());
        let cg: std::collections::HashMap<String, f64> = [("BTC".to_string(), 2.0e12), ("ETH".to_string(), 4.0e11)].into();
        let tout = caps_avec_cmc(&cg, &cmc);
        assert_eq!((tout["BTC"], tout["ETH"], tout["NEWC"]), (2.0e12, 4.0e11, 5.0e8), "CoinGecko stays, CMC adds");
        let mut h = vec![];
        push(
            &mut h,
            CapPoint {
                t: 100,
                caps: cmc,
                ..Default::default()
            },
        );
        assert!(h[0].caps.is_empty(), "not in the history");
    }

    #[test]
    fn erreurs_de_coinmarketcap() {
        let bad = r#"{"status":{"error_code":1001,"error_message":"This API Key is invalid."}}"#;
        assert_eq!(
            parse_global(bad, Stables::default(), 0).unwrap_err().to_string(),
            "This API Key is invalid.",
            "message tel quel"
        );
        assert!(parse_global("pas du json", Stables::default(), 0).is_err());
        assert!(parse_global(r#"{"data":{}}"#, Stables::default(), 0).is_err());
    }

    #[test]
    fn lien_https_seulement() {
        assert_eq!(base_url("").unwrap(), DEFAULT_URL);
        assert_eq!(base_url("https://pro-api.coinmarketcap.com/").unwrap(), DEFAULT_URL);
        assert!(base_url("http://pro-api.coinmarketcap.com").is_err());
        assert!(base_url("ftp://x").is_err());
        assert!(base_url("https://a@pirate.io").is_err());
        assert!(crate::net::autorise(&global_url(DEFAULT_URL)), "dans la liste blanche");
    }

    #[test]
    fn noms_des_series() {
        assert_eq!(series_of("CRYPTOCAP:USDT.D"), Some("USDT.D"));
        assert_eq!(series_of("total3"), Some("TOTAL3"));
        assert_eq!(series_of("CRYPTOCAP:OTHERS"), None);
        assert_eq!(series_matching("CRYPTOCAP:").len(), 7);
        assert_eq!(series_matching("CRYPTOCAP:US"), vec!["USDT.D", "USDC.D"]);
        assert_eq!(series_matching("TOTAL"), vec!["TOTAL", "TOTAL2", "TOTAL3"]);
        assert!(series_matching("btc").is_empty(), "« btc » reste une recherche normale");
        assert!(series_matching("USDT").is_empty(), "« USDT » aussi");
        assert_eq!(series_matching("usdt.d"), vec!["USDT.D"]);
        assert_eq!(parse_id(&id("BTC.D")), Some("BTC.D"));
        assert_eq!(parse_id("binance:BTCUSDT"), None);
    }

    #[test]
    fn peur_avidite_et_saison_coinmarketcap() {
        let f = parse_fng(
            r#"{"data":{"value":38,"update_time":"2026-11-05T00:00:00Z","value_classification":"Fear"},"status":{"error_code":"0","credit_count":1}}"#,
        );
        assert_eq!(f, Some((38, "Peur".to_string())));
        let mut top = vec![r#"{"id":1,"symbol":"BTC","name":"Bitcoin","quote":{"USD":{"percent_change_30d":-5}}}"#.to_string()];
        for i in 0..40 {
            let c = if i < 30 { -10 } else { 5 };
            top.push(format!(
                r#"{{"id":{},"symbol":"A{i}","name":"Alt {i}","quote":{{"USD":{{"percent_change_30d":{c}}}}}}}"#,
                i + 10
            ));
        }
        assert_eq!(parse_alt_season(&format!(r#"{{"data":[{}]}}"#, top.join(","))), Some(25));
    }

    #[test]
    fn credits_du_mois() {
        // 2026-11-05 et 2026-12-01 (UTC)
        let (nov, dec) = (1_793_836_800, 1_796_083_200);
        assert_eq!((month_of(nov), month_of(dec), month_of(0)), (202_611, 202_612, 197_001));
        let b: CreditBox = Default::default();
        assert!(reserve(&b, 2, nov));
        assert_eq!(b.lock().unwrap().used, 2);
        // un relevé par jour par défaut, au démarrage seulement si le dernier est plus vieux, ⟳ toujours
        assert!(due(None, nov, DEFAULT_EVERY_S, false, false), "jamais relevé : tout de suite");
        assert!(!due(Some(nov - 3600), nov, DEFAULT_EVERY_S, false, false), "relevé il y a 1 h : on attend");
        assert!(due(Some(nov - 90_000), nov, DEFAULT_EVERY_S, false, false), "plus de 24 h : relevé");
        assert!(due(Some(nov - 60), nov, DEFAULT_EVERY_S, true, false), "⟳");
        assert!(!due(Some(nov - 600), nov, 60, false, false), "jamais plus d'un relevé par heure");
        b.lock().unwrap().used = 9_999;
        assert!(!reserve(&b, 2, nov), "jamais au-delà de 10 000");
        assert!(reserve(&b, 1, nov));
        assert!(!reserve(&b, 1, nov));
        assert_eq!(b.lock().unwrap().label(), "crédits CMC ce mois : 10000 / 10000");
        assert!(reserve(&b, 1, dec), "remis à zéro le 1er");
        assert_eq!(b.lock().unwrap().used, 1);
    }

    #[test]
    fn historique_et_bougies() {
        let pt = |t: i64, b: f64| CapPoint {
            t,
            total: 1.0,
            btc_d: b,
            eth_d: 0.0,
            usdt_d: None,
            usdc_d: None,
            volume: None,
            chg_24h: None,
            fng: None,
            alt_season: None,
            caps: Default::default(),
        };
        let mut h = vec![];
        push(&mut h, pt(0, 50.0));
        push(&mut h, pt(30, 51.0));
        assert_eq!(h.len(), 1, "moins d'1 min : remplacé");
        push(&mut h, pt(300, 52.0));
        push(&mut h, pt(200, 1.0));
        assert_eq!(h.len(), 2, "un relevé plus vieux est ignoré");
        let b = bars(&h, "BTC.D");
        assert_eq!(b.len(), 2);
        assert_eq!((b[1].time, b[1].open, b[1].close), (300, 51.0, 52.0), "ouverture = valeur précédente");
        assert_eq!(b[1].volume, 0.0);
        assert!(bars(&h, "USDT.D").is_empty(), "série pas encore relevée : vide");
        let mut h2 = vec![];
        for i in 0..MAX_POINTS as i64 + 10 {
            push(&mut h2, pt(i * 300, 1.0));
        }
        assert_eq!(h2.len(), MAX_POINTS);
        assert_eq!(h2[0].t, 10 * 300, "les plus vieux partent");
    }
}
