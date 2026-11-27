//! 📅 Agenda et 📰 nouvelles, **les mêmes sources qu'Argos v1** :
//! - calendrier économique de la semaine : flux JSON public de Forex Factory (faireconomy) ;
//! - nouvelles crypto / marchés : flux RSS publics (CoinDesk, Cointelegraph, Decrypt, Bitcoin Magazine, CryptoSlate,
//!   FXStreet, MarketWatch) et, en français (Jee, 22 nov. 2026), Cointribune, BeInCrypto FR, Journal du Coin et Invezz
//!   FR (Cointelegraph FR répondait « 410 Gone » le 22 nov. : laissé de côté).
//!
//! Desktop : direct à la source. Navigateur : par les relais du site (`/api/calendar`, `/api/news`), parce que ces
//! sources refusent les pages web (CORS). Lecture seule.

use serde_json::Value;

/// Calendrier de la semaine (Forex Factory).
pub const CALENDAR_URL: &str = "https://nfs.faireconomy.media/ff_calendar_thisweek.json";
/// Flux RSS (source, adresse, langue « fr » / « en ») ; le relais du serveur local lit la même liste (test dans
/// argos-serveur).
pub const FEEDS: [(&str, &str, &str); 11] = [
    ("CoinDesk", "https://www.coindesk.com/arc/outboundfeeds/rss/", "en"),
    ("Cointelegraph", "https://cointelegraph.com/rss", "en"),
    ("Decrypt", "https://decrypt.co/feed", "en"),
    ("Bitcoin Magazine", "https://bitcoinmagazine.com/.rss/full/", "en"),
    ("CryptoSlate", "https://cryptoslate.com/feed/", "en"),
    ("FXStreet", "https://www.fxstreet.com/rss/news", "en"),
    ("MarketWatch", "https://feeds.content.dowjones.io/public/rss/mw_topstories", "en"),
    ("Cointribune", "https://www.cointribune.com/feed/", "fr"),
    ("BeInCrypto FR", "https://fr.beincrypto.com/feed/", "fr"),
    ("Journal du Coin", "https://journalducoin.com/feed/", "fr"),
    ("Invezz FR", "https://invezz.com/fr/feed/", "fr"),
];

/// Langue d'une source (« fr » / « en » ; inconnue : « en »).
pub fn lang_of(source: &str) -> &'static str {
    FEEDS.iter().find(|f| f.0 == source).map_or("en", |f| f.2)
}

/// Un événement du calendrier.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Event {
    pub title: String,
    /// Devise (USD, EUR, CAD…).
    pub country: String,
    /// « High », « Medium », « Low », « Holiday ».
    pub impact: String,
    /// Heure (secondes Unix).
    pub ts: i64,
    pub forecast: String,
    pub previous: String,
    pub actual: String,
}

/// Une nouvelle.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct News {
    pub title: String,
    pub link: String,
    /// Heure (secondes Unix).
    pub ts: i64,
    pub source: String,
    /// Langue de sa source (« fr » / « en »).
    pub lang: String,
}

fn s(v: &Value, k: &str) -> String {
    v[k].as_str().unwrap_or("").to_string()
}

/// Lit le calendrier : le flux de Forex Factory (tableau, `date` RFC 3339) ou le relais du site (`{items:[{ts}]}`,
/// `ts` en millisecondes). Trié par heure.
pub fn parse_calendar(raw: &str) -> Vec<Event> {
    let Ok(v) = serde_json::from_str::<Value>(raw) else { return vec![] };
    let list = v.as_array().cloned().or_else(|| v["items"].as_array().cloned()).unwrap_or_default();
    let mut out: Vec<Event> = list.iter().filter_map(event_of).filter(|e| !e.title.is_empty()).collect();
    out.sort_by_key(|e| e.ts);
    out
}

/// Un événement du calendrier (objet de Forex Factory, `date` RFC 3339, ou du relais, `ts` en millisecondes).
pub fn event_of(e: &Value) -> Option<Event> {
    let ts = match e["ts"].as_i64() {
        Some(ms) => ms / 1000,
        None => chrono::DateTime::parse_from_rfc3339(e["date"].as_str()?).ok()?.timestamp(),
    };
    Some(Event {
        title: s(e, "title"),
        country: s(e, "country"),
        impact: s(e, "impact"),
        ts,
        forecast: s(e, "forecast"),
        previous: s(e, "previous"),
        actual: s(e, "actual"),
    })
}

/// Texte d'une balise XML (CDATA, balises HTML et entités enlevées).
fn tag(item: &str, name: &str) -> String {
    let Some(a) = item.find(&format!("<{name}")) else { return String::new() };
    let Some(open_end) = item[a..].find('>').map(|k| a + k + 1) else {
        return String::new();
    };
    let Some(close) = item[open_end..].find(&format!("</{name}>")).map(|k| open_end + k) else {
        return String::new();
    };
    clean(&item[open_end..close])
}

fn clean(x: &str) -> String {
    let mut t = x.trim().to_string();
    if let Some(inner) = t.strip_prefix("<![CDATA[").and_then(|r| r.strip_suffix("]]>")) {
        t = inner.to_string();
    }
    // balises HTML
    let mut out = String::new();
    let mut inside = false;
    for c in t.chars() {
        match c {
            '<' => inside = true,
            '>' => inside = false,
            c if !inside => out.push(c),
            _ => {}
        }
    }
    out.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#039;", "'")
        .replace("&apos;", "'")
        .trim()
        .to_string()
}

/// Lit un flux RSS (ou Atom) : les 30 premières nouvelles avec un lien http(s).
pub fn parse_rss(xml: &str, source: &str) -> Vec<News> {
    let mut out = vec![];
    let mut rest = xml;
    while out.len() < 30 {
        let (open, close) = match (rest.find("<item"), rest.find("<entry")) {
            (Some(i), _) => (i, "</item>"),
            (None, Some(e)) => (e, "</entry>"),
            _ => break,
        };
        let Some(end) = rest[open..].find(close).map(|k| open + k + close.len()) else {
            break;
        };
        let item = &rest[open..end];
        rest = &rest[end..];
        let mut link = tag(item, "link");
        if link.is_empty() {
            // Atom : <link href="…"/>
            if let Some(h) = item.find("href=\"") {
                link = item[h + 6..].split('"').next().unwrap_or("").to_string();
            }
        }
        let date = [tag(item, "pubDate"), tag(item, "dc:date"), tag(item, "updated"), tag(item, "published")]
            .into_iter()
            .find(|d| !d.is_empty())
            .unwrap_or_default();
        let ts = chrono::DateTime::parse_from_rfc2822(&date)
            .or_else(|_| chrono::DateTime::parse_from_rfc3339(&date))
            .map(|d| d.timestamp())
            .unwrap_or(0);
        let title = tag(item, "title");
        if !title.is_empty() && (link.starts_with("http://") || link.starts_with("https://")) {
            out.push(News {
                title,
                link,
                ts,
                source: source.to_string(),
                lang: lang_of(source).to_string(),
            });
        }
    }
    out
}

/// Lit le relais du site (`{items:[{title, link, ts (ms), source}]}`).
pub fn parse_news_api(raw: &str) -> Vec<News> {
    let Ok(v) = serde_json::from_str::<Value>(raw) else { return vec![] };
    v["items"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|n| News {
            title: s(n, "title"),
            link: s(n, "link"),
            ts: n["ts"].as_i64().unwrap_or(0) / 1000,
            source: s(n, "source"),
            lang: n["lang"]
                .as_str()
                .map_or_else(|| lang_of(n["source"].as_str().unwrap_or("")).to_string(), String::from),
        })
        .filter(|n| !n.title.is_empty() && n.link.starts_with("http"))
        .collect()
}

/// Calendrier de la semaine.
#[cfg(any(feature = "native", feature = "web"))]
pub async fn calendar() -> argos_core::Result<Vec<Event>> {
    use crate::http::Http;
    let url = if crate::direct() { CALENDAR_URL } else { "/api/calendar" };
    let raw = crate::rt::Client::default().get(url).await.map_err(|e| e.to_string())?;
    let list = parse_calendar(&raw);
    if list.is_empty() {
        return Err("calendrier vide ou illisible".into());
    }
    Ok(list)
}

/// Un message d'erreur montrable : jamais de HTML brut (une page « 404 » renvoyée par un serveur), court (100
/// caractères au plus).
pub fn erreur_lisible(e: &str) -> String {
    let t = e.split('<').next().unwrap_or("").trim().trim_end_matches(':').trim();
    let t = if t.is_empty() { "réponse inattendue du serveur" } else { t };
    if t.chars().count() > 100 {
        format!("{}…", t.chars().take(99).collect::<String>())
    } else {
        t.to_string()
    }
}

/// Nouvelles de toutes les sources, les plus récentes d'abord (200 au plus).
#[cfg(any(feature = "native", feature = "web"))]
pub async fn news() -> argos_core::Result<Vec<News>> {
    use crate::http::Http;
    let c = crate::rt::Client::default();
    let mut all = vec![];
    if crate::direct() {
        for (src, url, _) in FEEDS {
            if let Ok(xml) = c.get(url).await {
                all.extend(parse_rss(&xml, src));
            }
        }
    } else {
        all = parse_news_api(&c.get("/api/news").await.map_err(|e| e.to_string())?);
    }
    if all.is_empty() {
        return Err("aucune nouvelle (pas d'Internet ?)".into());
    }
    all.sort_by_key(|n| -n.ts);
    all.truncate(200);
    Ok(all)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Jamais de HTML brut dans un message (le relais /api/news absent renvoyait sa page 404).
    #[test]
    fn erreur_sans_html() {
        let e = "source de données : HTTP 404 : <!DOCTYPE HTML><html><head><title>Error response</title>";
        assert_eq!(erreur_lisible(e), "source de données : HTTP 404");
        assert_eq!(erreur_lisible("<html>"), "réponse inattendue du serveur");
        assert_eq!(erreur_lisible(&"x".repeat(300)).chars().count(), 100);
        assert_eq!(erreur_lisible("pas d'Internet"), "pas d'Internet");
    }

    #[test]
    fn calendrier() {
        let ff = r#"[{"title":"CPI m/m","country":"USD","date":"2026-10-27T08:30:00-04:00","impact":"High","forecast":"0.3%","previous":"0.2%"},
                     {"title":"BOC Rate Statement","country":"CAD","date":"2026-10-26T09:45:00-04:00","impact":"High","forecast":"","previous":""}]"#;
        let e = parse_calendar(ff);
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].country, "CAD", "trié par heure");
        assert_eq!(e[1].ts, chrono::DateTime::parse_from_rfc3339("2026-10-27T12:30:00Z").unwrap().timestamp());
        assert_eq!(e[1].forecast, "0.3%");
        let api = r#"{"items":[{"title":"NFP","country":"USD","impact":"High","ts":1793450000000,"forecast":"","previous":"","actual":"150K"}]}"#;
        assert_eq!(parse_calendar(api)[0].ts, 1793450000);
        assert!(parse_calendar("pas du json").is_empty());
    }

    #[test]
    fn nouvelles() {
        let rss = r#"<rss><channel><item><title><![CDATA[Bitcoin &amp; l'or <b>montent</b>]]></title><link>https://x.com/a</link><pubDate>Mon, 26 Oct 2026 14:00:00 GMT</pubDate></item>
            <item><title>Sans lien</title><link>javascript:alert(1)</link></item></channel></rss>"#;
        let n = parse_rss(rss, "CoinDesk");
        assert_eq!(n.len(), 1, "un lien qui n'est pas http(s) est laissé de côté");
        assert_eq!(n[0].title, "Bitcoin & l'or montent");
        assert_eq!((n[0].source.as_str(), n[0].lang.as_str()), ("CoinDesk", "en"));
        assert_eq!(parse_rss(rss, "Cointribune")[0].lang, "fr", "a French source");
        assert!(n[0].ts > 0);
        let atom = r#"<feed><entry><title>Atom</title><link href="https://y.com/b"/><updated>2026-10-26T10:00:00Z</updated></entry></feed>"#;
        assert_eq!(parse_rss(atom, "Y")[0].link, "https://y.com/b");
        let api = r#"{"items":[{"title":"T","link":"https://z.com","ts":1793450000000,"source":"Decrypt"}]}"#;
        assert_eq!(parse_news_api(api)[0].ts, 1793450000);
        let api = r#"{"items":[{"title":"T","link":"https://z.com","ts":1,"source":"Journal du Coin","lang":"fr"}]}"#;
        assert_eq!(parse_news_api(api)[0].lang, "fr");
        assert_eq!(lang_of("Invezz FR"), "fr");
        assert_eq!(FEEDS.iter().filter(|f| f.2 == "fr").count(), 4);
    }
}
