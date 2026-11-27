//! 📅 The « Réel » column of the calendar (USD only), from the official sources Jee accepted (22 Nov 2026):
//! - BLS public API v1, no key (`api.bls.gov`): one POST for every series, 25 requests a day at most for that API, so
//!   [`Reel`] never makes more than [`BLS_PER_DAY`];
//! - the Fed (`www.federalreserve.gov`): its monetary press feed, then the FOMC statement of the day.
//!
//! Each release is read from its time to 60 min after (at +1, +3, +10, +30 and +60 min, stopping as soon as the
//! number is there), and written like the release's « Prévu » (same decimals, same unit) so the columns compare.
//! Plus the price reaction under a past release ([`reaction`]), from candles the chart already loaded.

use crate::eco::Event;
use serde_json::Value;

/// BLS public API v1 (no key).
pub const BLS_URL: &str = "https://api.bls.gov/publicAPI/v1/timeseries/data/";
/// The Fed's monetary policy press releases.
pub const FED_FEED: &str = "https://www.federalreserve.gov/feeds/press_monetary.xml";
/// The Fed's site (the statement's page must be on it).
pub const FED_SITE: &str = "https://www.federalreserve.gov/";
/// BLS v1 allows 25 requests a day: we keep under 20.
pub const BLS_PER_DAY: u32 = 20;
/// Seconds after a release when it is read again (then never).
pub const CHECKS: [i64; 5] = [60, 180, 600, 1800, 3600];

/// How a BLS number is computed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Calc {
    /// Change over 1 month, in %.
    Month,
    /// Change over 12 months, in %.
    Year,
    /// Change of the level, in thousands (« 231K »).
    Diff,
    /// The value itself (%).
    Level,
}

/// Forex Factory title → BLS series → computation.
pub const BLS_TABLE: [(&str, &str, Calc); 9] = [
    ("CPI m/m", "CUSR0000SA0", Calc::Month),
    ("Core CPI m/m", "CUSR0000SA0L1E", Calc::Month),
    ("CPI y/y", "CUUR0000SA0", Calc::Year),
    ("Core CPI y/y", "CUUR0000SA0L1E", Calc::Year),
    ("Non-Farm Employment Change", "CES0000000001", Calc::Diff),
    ("Unemployment Rate", "LNS14000000", Calc::Level),
    ("Average Hourly Earnings m/m", "CES0500000003", Calc::Month),
    ("PPI m/m", "WPSFD4", Calc::Month),
    ("Core PPI m/m", "WPSFD49104", Calc::Month),
];
/// The Fed's rate decision (Forex Factory title).
pub const FED_RATE: &str = "Federal Funds Rate";

/// Can this release get its « Réel » here?
pub fn known(e: &Event) -> bool {
    e.country == "USD" && (e.title == FED_RATE || BLS_TABLE.iter().any(|(t, _, _)| *t == e.title))
}

/// The body of the one BLS request for every series.
pub fn bls_body() -> String {
    let ids: Vec<Value> = BLS_TABLE.iter().map(|(_, s, _)| Value::from(*s)).collect();
    serde_json::json!({ "seriesid": ids }).to_string()
}

/// One BLS series: (year, month 1-12, value), the latest first; « - » (no data) is left out.
pub type Series = Vec<(i32, u32, f64)>;

/// Reads the BLS answer: series id → points. `None`: not the expected form (status, Results.series).
pub fn parse_bls(raw: &str) -> Option<std::collections::HashMap<String, Series>> {
    let v: Value = serde_json::from_str(raw).ok()?;
    if v["status"].as_str()? != "REQUEST_SUCCEEDED" {
        return None;
    }
    let mut out = std::collections::HashMap::new();
    for s in v["Results"]["series"].as_array()? {
        let id = s["seriesID"].as_str()?.to_string();
        let pts: Series = s["data"]
            .as_array()?
            .iter()
            .filter_map(|d| {
                let y: i32 = d["year"].as_str()?.parse().ok()?;
                let m: u32 = d["period"].as_str()?.strip_prefix('M')?.parse().ok()?;
                let x: f64 = d["value"].as_str()?.parse().ok()?;
                (1..=12).contains(&m).then_some((y, m, x))
            })
            .collect();
        out.insert(id, pts);
    }
    Some(out)
}

/// The New York date at `ts` (US daylight time: second Sunday of March 2:00 to first Sunday of November 2:00).
pub fn ny_date(ts: i64) -> chrono::NaiveDate {
    use chrono::{Datelike, NaiveDate};
    let year = chrono::DateTime::from_timestamp(ts, 0).map_or(1970, |d| d.year());
    let sunday = |m: u32, n: u32| {
        let first = NaiveDate::from_ymd_opt(year, m, 1).unwrap_or_default();
        let to_sun = (7 - first.weekday().num_days_from_sunday()) % 7;
        first + chrono::Days::new((to_sun + 7 * (n - 1)) as u64)
    };
    let utc = |d: NaiveDate, h: u32| d.and_hms_opt(h, 0, 0).map_or(0, |t| t.and_utc().timestamp());
    // 2:00 local = 7:00 UTC in March (still standard time), 6:00 UTC in November (still daylight time)
    let dst = ts >= utc(sunday(3, 2), 7) && ts < utc(sunday(11, 1), 6);
    let off = if dst { 4 * 3600 } else { 5 * 3600 };
    chrono::DateTime::from_timestamp(ts - off, 0).map(|d| d.date_naive()).unwrap_or_default()
}

/// (year, month) of the month before the release at `ts` (New York): the month the release is about.
fn month_before(ts: i64) -> (i32, u32) {
    use chrono::Datelike;
    let d = ny_date(ts);
    if d.month() == 1 {
        (d.year() - 1, 12)
    } else {
        (d.year(), d.month() - 1)
    }
}

/// The number of a series for the release at `ts`, before rounding: `None` while the BLS hasn't the month yet.
pub fn bls_value(series: &Series, calc: Calc, ts: i64) -> Option<f64> {
    let (y, m) = month_before(ts);
    let at = |y: i32, m: u32| series.iter().find(|p| p.0 == y && p.1 == m).map(|p| p.2);
    let now = at(y, m)?;
    let prev = if m == 1 { at(y - 1, 12) } else { at(y, m - 1) };
    match calc {
        Calc::Level => Some(now),
        Calc::Month => prev.filter(|p| *p != 0.0).map(|p| (now / p - 1.0) * 100.0),
        Calc::Year => at(y - 1, m).filter(|p| *p != 0.0).map(|p| (now / p - 1.0) * 100.0),
        Calc::Diff => prev.map(|p| now - p),
    }
}

/// The decimals and the unit of a « Prévu » (« 0.3% » → 1, « % » ; « 231K » → 0, « K » ; « 4.00% » → 2, « % »).
pub fn shape_of(forecast: &str) -> Option<(usize, String)> {
    let f = forecast.trim();
    let n = f
        .trim_start_matches(['-', '+'])
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect::<String>();
    if n.is_empty() {
        return None;
    }
    let unit = f.trim_start_matches(['-', '+'])[n.len()..].trim().to_string();
    Some((n.split_once('.').map_or(0, |(_, d)| d.len()), unit))
}

/// `v` written like the forecast (`shape`), else like Forex Factory writes that kind (« 0.3% », « 231K »).
pub fn written(v: f64, calc: Calc, forecast: &str) -> String {
    let (dec, unit) = shape_of(forecast).unwrap_or(match calc {
        Calc::Diff => (0, "K".into()),
        _ => (1, "%".into()),
    });
    let s = format!("{v:.dec$}");
    let s = if s.trim_start_matches('-').chars().all(|c| c == '0' || c == '.') {
        s.trim_start_matches('-').to_string()
    } else {
        s
    };
    format!("{s}{unit}")
}

/// The « Réel » of a BLS release from the series read, or `None`.
pub fn bls_actual(e: &Event, all: &std::collections::HashMap<String, Series>) -> Option<String> {
    let (_, id, calc) = BLS_TABLE.iter().find(|(t, _, _)| *t == e.title)?;
    let v = bls_value(all.get(*id)?, *calc, e.ts)?;
    Some(written(v, *calc, &e.forecast))
}

/// The link of the FOMC statement released on the New York day of `ts` in the Fed's feed (on the Fed's site only).
pub fn fed_statement_link(feed: &str, ts: i64) -> Option<String> {
    let day = ny_date;
    crate::eco::parse_rss(feed, "Fed")
        .into_iter()
        .find(|n| n.title.contains("FOMC statement") && n.ts > 0 && day(n.ts) == day(ts))
        .map(|n| n.link)
        .filter(|l| l.starts_with(FED_SITE))
}

/// « 3-3/4 » → 3.75, « 4 » → 4.0, « 4-1/2 » → 4.5.
fn fraction(s: &str) -> Option<f64> {
    let s = s.trim();
    match s.split_once('-') {
        Some((a, f)) => {
            let (n, d) = f.split_once('/')?;
            Some(a.parse::<f64>().ok()? + n.parse::<f64>().ok()? / d.parse::<f64>().ok()?)
        }
        None => match s.split_once('/') {
            Some((n, d)) => Some(n.parse::<f64>().ok()? / d.parse::<f64>().ok()?),
            None => s.parse().ok(),
        },
    }
}

/// The upper bound of the target range in the statement's page (« … federal funds rate at 3-3/4 to 4 percent »).
pub fn fed_upper(page: &str) -> Option<f64> {
    let k = page.find("target range for the federal funds rate")?;
    let rest = &page[k..(k + 300).min(page.len())];
    let to = rest.find(" to ")?;
    let after = &rest[to + 4..];
    let pct = after.find(" percent")?;
    fraction(&after[..pct])
}

/// The Fed's rate written like the forecast (« 4.00% »).
pub fn fed_actual(e: &Event, page: &str) -> Option<String> {
    let v = fed_upper(page)?;
    let forecast = if e.forecast.is_empty() { "0.00%" } else { e.forecast.as_str() };
    Some(written(v, Calc::Level, forecast))
}

/// Is a reading of `e` due at `now`, its last reading being at `last` (0: never)? From its time to +60 min, at each
/// of [`CHECKS`] not yet done; never once its « Réel » is there.
pub fn due(e: &Event, now: i64, last: i64) -> bool {
    known(e) && e.actual.is_empty() && CHECKS.iter().any(|c| now >= e.ts + c && last < e.ts + c && now <= e.ts + 3600 + 600)
}

/// 🔢 The readings and their numbers, kept between calendar reloads: (title, time) → « Réel », last reading per
/// release, and the BLS requests of the day.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Reel {
    pub values: std::collections::HashMap<(String, i64), String>,
    pub last: std::collections::HashMap<(String, i64), i64>,
    /// (UTC day, requests made to the BLS that day).
    pub bls_day: (i64, u32),
}

impl Reel {
    /// Puts the numbers already read into `events` (an empty « Réel » only).
    pub fn fill(&self, events: &mut [Event]) {
        for e in events.iter_mut().filter(|e| e.actual.is_empty()) {
            if let Some(v) = self.values.get(&(e.title.clone(), e.ts)) {
                e.actual = v.clone();
            }
        }
    }

    /// The releases of `events` to read at `now`.
    pub fn due<'a>(&self, events: &'a [Event], now: i64) -> Vec<&'a Event> {
        events
            .iter()
            .filter(|e| !self.values.contains_key(&(e.title.clone(), e.ts)))
            .filter(|e| due(e, now, self.last.get(&(e.title.clone(), e.ts)).copied().unwrap_or(0)))
            .collect()
    }

    /// May one more BLS request be made at `now` (and count it)?
    pub fn take_bls(&mut self, now: i64) -> bool {
        let day = now.div_euclid(86_400);
        if self.bls_day.0 != day {
            self.bls_day = (day, 0);
        }
        if self.bls_day.1 >= BLS_PER_DAY {
            return false;
        }
        self.bls_day.1 += 1;
        true
    }

    /// Reads what is due in `events` (one BLS request for all of them, the Fed's feed and page when its decision is
    /// due), keeps it and fills `events`.
    #[cfg(any(feature = "native", feature = "web"))]
    pub async fn read<H: crate::http::Http>(&mut self, http: &H, events: &mut [Event], now: i64) {
        let due: Vec<Event> = self.due(events, now).into_iter().cloned().collect();
        if due.is_empty() {
            self.fill(events);
            return;
        }
        for e in &due {
            self.last.insert((e.title.clone(), e.ts), now);
        }
        if due.iter().any(|e| e.title != FED_RATE) && self.take_bls(now) {
            if let Some(all) = http.post_json(BLS_URL, &bls_body()).await.ok().and_then(|r| parse_bls(&r)) {
                for e in due.iter().filter(|e| e.title != FED_RATE) {
                    if let Some(v) = bls_actual(e, &all) {
                        self.values.insert((e.title.clone(), e.ts), v);
                    }
                }
            }
        }
        if let Some(e) = due.iter().find(|e| e.title == FED_RATE) {
            if let Some(link) = http.get(FED_FEED).await.ok().and_then(|f| fed_statement_link(&f, e.ts)) {
                if let Some(v) = http.get(&link).await.ok().and_then(|p| fed_actual(e, &p)) {
                    self.values.insert((e.title.clone(), e.ts), v);
                }
            }
        }
        self.fill(events);
    }
}

/// A number of the calendar (« 0.3% », « 231K », « -0.4% ») as a value, `None` if empty.
pub fn number(s: &str) -> Option<f64> {
    let n: String = s.trim().chars().take_while(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+')).collect();
    n.parse().ok()
}

/// Is a higher number good for the economy and the markets? `None`: not in the table.
pub fn higher_is_good(title: &str) -> Option<bool> {
    let t = title.to_lowercase();
    if t.contains("non-farm") {
        Some(true)
    } else if ["cpi", "ppi", "unemployment", "hourly earnings", "federal funds"].iter().any(|k| t.contains(k)) {
        Some(false)
    } else {
        None
    }
}

/// 🟢 / 🔴 The « Réel » against the « Prévu »: `Some(true)` better, `Some(false)` worse, `None` equal, unknown or
/// missing (white).
pub fn better(e: &Event) -> Option<bool> {
    let (a, f) = (number(&e.actual)?, number(&e.forecast)?);
    let up = higher_is_good(&e.title)?;
    if (a - f).abs() < 1e-9 {
        None
    } else {
        Some((a > f) == up)
    }
}

/// 📈 The price reaction to a release at `ts` in 1-minute candles: % from the close of the minute before the release
/// to 5 min, 15 min and 1 h after (each `None` while that candle isn't there).
pub fn reaction(bars: &[argos_core::Bar], ts: i64) -> Option<[Option<f64>; 3]> {
    let minute = ts - ts.rem_euclid(60);
    let base = bars
        .iter()
        .rev()
        .find(|b| b.time < minute)
        .filter(|b| b.time >= minute - 120 && b.close > 0.0)?
        .close;
    let at = |s: i64| bars.iter().find(|b| b.time == minute + s - 60).map(|b| (b.close / base - 1.0) * 100.0);
    Some([at(300), at(900), at(3600)])
}

/// « BTC −1,2 % en 5 min » (French comma, true minus; « + » when up).
pub fn reaction_text(coin: &str, pct: f64, label: &str) -> String {
    let abs = format!("{:.1}", pct.abs()).replace('.', ",");
    let sign = if abs.chars().all(|c| c == '0' || c == ',') {
        ""
    } else if pct < 0.0 {
        "−"
    } else {
        "+"
    };
    format!("{coin} {sign}{abs} % en {label}")
}

/// A calendar number the French way (« 0.3% » → « 0,3 % », « -0.4% » → « −0,4 % », « 231K » → « 231 k »).
pub fn french(s: &str) -> String {
    let t = s.trim();
    if number(t).is_none() {
        return t.to_string();
    }
    let n: String = t.chars().take_while(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+')).collect();
    let unit = t[n.len()..].trim();
    let n = n.replace('.', ",").replace('-', "−");
    match unit {
        "" => n,
        "K" => format!("{n} k"),
        "M" | "B" | "T" => format!("{n} {unit}"),
        u => format!("{n} {u}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(title: &str, ts: i64, forecast: &str) -> Event {
        Event {
            title: title.into(),
            country: "USD".into(),
            impact: "High".into(),
            ts,
            forecast: forecast.into(),
            ..Default::default()
        }
    }

    // the form read on the PC on 22 Nov 2026 (cut down)
    const BLS: &str = r#"{"status":"REQUEST_SUCCEEDED","responseTime":71,"message":[],"Results":{"series":[
        {"seriesID":"CUSR0000SA0","data":[{"year":"2026","period":"M10","value":"337.476"},{"year":"2026","period":"M09","value":"336.298"}]},
        {"seriesID":"CUUR0000SA0","data":[{"year":"2026","period":"M10","value":"337.511"},{"year":"2025","period":"M10","value":"-"},{"year":"2025","period":"M11","value":"325.0"}]},
        {"seriesID":"CES0000000001","data":[{"year":"2026","period":"M10","value":"159293"},{"year":"2026","period":"M09","value":"159062"}]},
        {"seriesID":"LNS14000000","data":[{"year":"2026","period":"M10","value":"4.1"},{"year":"2026","period":"M09","value":"4.2"}]},
        {"seriesID":"CES0500000003","data":[{"year":"2026","period":"M12","value":"38.20"},{"year":"2026","period":"M11","value":"38.00"}]}
    ]}}"#;

    /// 13 Nov 2026, 8:30 New York: a release about October.
    const NOV13: i64 = 1_794_576_600;

    #[test]
    fn bls_lu_et_arrondi_comme_le_prevu() {
        let all = parse_bls(BLS).unwrap();
        assert_eq!(all["CUUR0000SA0"].len(), 2, "« - » left out");
        assert_eq!(bls_actual(&ev("CPI m/m", NOV13, "0.3%"), &all).as_deref(), Some("0.4%"));
        assert_eq!(bls_actual(&ev("CPI m/m", NOV13, "0.30%"), &all).as_deref(), Some("0.35%"));
        assert_eq!(bls_actual(&ev("Non-Farm Employment Change", NOV13, "222K"), &all).as_deref(), Some("231K"));
        assert_eq!(bls_actual(&ev("Unemployment Rate", NOV13, "4.2%"), &all).as_deref(), Some("4.1%"));
        assert_eq!(bls_actual(&ev("CPI y/y", NOV13, "3.0%"), &all), None, "the year before is « - »");
        assert_eq!(bls_actual(&ev("Average Hourly Earnings m/m", NOV13, "0.3%"), &all), None, "October not there");
        assert_eq!(bls_actual(&ev("Core PPI m/m", NOV13, "0.2%"), &all), None, "series not read");
        assert!(parse_bls(r#"{"status":"REQUEST_NOT_PROCESSED","Results":{}}"#).is_none());
        assert!(bls_body().contains("\"CUSR0000SA0\"") && bls_body().starts_with("{\"seriesid\":["));
        // January: the month before is December of the year before
        assert_eq!(month_before(1_799_000_000), (2026, 12));
        // New York dates around the daylight time changes (2026: 8 March, 1 November)
        let d = |s: &str| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
        assert_eq!(ny_date(1_793_210_400), d("2026-10-28"), "18:00 UTC = 14:00 daylight time");
        assert_eq!(ny_date(1_793_242_800), d("2026-10-28"), "03:00 UTC on the 29th = 23:00 on the 28th");
        assert_eq!(
            ny_date(1_795_618_800 + 9 * 3600),
            d("2026-11-25"),
            "00:00 UTC on the 26th = 19:00 standard time"
        );
    }

    #[test]
    fn formes() {
        assert_eq!(shape_of("0.3%"), Some((1, "%".into())));
        assert_eq!(shape_of("231K"), Some((0, "K".into())));
        assert_eq!(shape_of("-0.4%"), Some((1, "%".into())));
        assert_eq!(shape_of(""), None);
        assert_eq!(written(-0.04, Calc::Month, "0.3%"), "0.0%", "never « -0.0% »");
        assert_eq!(written(231.0, Calc::Diff, ""), "231K");
        assert_eq!(french("0.3%"), "0,3 %");
        assert_eq!(french("-0.4%"), "−0,4 %");
        assert_eq!(french("231K"), "231 k");
        assert_eq!(french(""), "");
    }

    #[test]
    fn fed() {
        let feed = r#"<rss><channel>
            <item><title>Minutes of the Federal Open Market Committee</title><link><![CDATA[https://www.federalreserve.gov/newsevents/pressreleases/monetary20261118a.htm]]></link><pubDate><![CDATA[Wed, 18 Nov 2026 19:00:00 GMT]]></pubDate></item>
            <item><title>Federal Reserve issues FOMC statement</title><link><![CDATA[https://www.federalreserve.gov/newsevents/pressreleases/monetary20261028a.htm]]></link><pubDate><![CDATA[Wed, 28 Oct 2026 18:00:00 GMT]]></pubDate></item>
        </channel></rss>"#;
        // 28 Oct 2026, 14:00 New York
        let ts = 1_793_210_400;
        assert_eq!(
            fed_statement_link(feed, ts).as_deref(),
            Some("https://www.federalreserve.gov/newsevents/pressreleases/monetary20261028a.htm")
        );
        assert_eq!(fed_statement_link(feed, ts + 86_400), None, "not the day of the decision");
        let page = "<p>the Committee decided to lower the target range for the federal funds rate at 3-3/4 to 4 percent, in support</p>";
        assert_eq!(fed_upper(page), Some(4.0));
        assert_eq!(fed_upper("target range for the federal funds rate at 3-1/2 to 3-3/4 percent"), Some(3.75));
        assert_eq!(fed_actual(&ev(FED_RATE, ts, "4.00%"), page).as_deref(), Some("4.00%"));
        assert_eq!(fed_upper("nothing here"), None);
    }

    #[test]
    fn releves_et_compteur() {
        let e = ev("CPI m/m", 10_000, "0.3%");
        assert!(!due(&e, 9_999, 0), "not before its time");
        assert!(due(&e, 10_060, 0), "+1 min");
        assert!(!due(&e, 10_100, 10_060), "already read at +1 min");
        assert!(due(&e, 10_180, 10_060), "+3 min");
        assert!(due(&e, 13_600, 11_800), "+60 min");
        assert!(!due(&e, 13_700, 13_600), "then never");
        assert!(
            !due(
                &Event {
                    actual: "0.4%".into(),
                    ..e.clone()
                },
                10_060,
                0
            ),
            "number there: stop"
        );
        assert!(!due(&ev("GDP q/q", 10_000, ""), 10_060, 0), "not in the table");
        assert!(
            !due(
                &Event {
                    country: "EUR".into(),
                    ..e.clone()
                },
                10_060,
                0
            ),
            "USD only"
        );
        let mut r = Reel::default();
        let day = 20_000 * 86_400;
        for _ in 0..BLS_PER_DAY {
            assert!(r.take_bls(day));
        }
        assert!(!r.take_bls(day + 10), "never more than 20 a day");
        assert!(r.take_bls(day + 86_400), "a new day");
        r.values.insert(("CPI m/m".into(), 10_000), "0.4%".into());
        let mut list = vec![e.clone()];
        assert!(r.due(&list, 10_060).is_empty(), "already read");
        r.fill(&mut list);
        assert_eq!(list[0].actual, "0.4%");
    }

    #[test]
    fn couleur_du_reel() {
        let mut e = ev("CPI m/m", 0, "0.3%");
        e.actual = "0.4%".into();
        assert_eq!(better(&e), Some(false), "inflation higher: worse");
        e.actual = "0.2%".into();
        assert_eq!(better(&e), Some(true));
        e.actual = "0.3%".into();
        assert_eq!(better(&e), None, "equal: white");
        let nfp = Event {
            actual: "231K".into(),
            ..ev("Non-Farm Employment Change", 0, "222K")
        };
        assert_eq!(better(&nfp), Some(true), "more jobs: better");
        let fed = Event {
            actual: "4.00%".into(),
            ..ev(FED_RATE, 0, "3.75%")
        };
        assert_eq!(better(&fed), Some(false));
        assert_eq!(better(&ev("GDP q/q", 0, "1.0%")), None);
    }

    #[test]
    fn reaction_du_prix() {
        let bar = |t: i64, c: f64| argos_core::Bar {
            time: t,
            open: c,
            high: c,
            low: c,
            close: c,
            volume: 1.0,
        };
        let m = 1_000_020; // a whole minute
        let ts = m + 30; // released 30 s into it
        let bars: Vec<_> = (-30..=60)
            .map(|k| bar(m + k * 60, if k < 0 { 100.0 } else { 98.8 + k as f64 * 0.01 }))
            .collect();
        let r = reaction(&bars, ts).unwrap();
        assert!((r[0].unwrap() - (98.84 / 100.0 - 1.0) * 100.0).abs() < 1e-9, "close of the 5th minute");
        assert!(r[2].is_some());
        assert_eq!(reaction(&bars[..40], ts).unwrap()[2], None, "1 h not there yet");
        assert!(reaction(&[], ts).is_none());
        assert_eq!(reaction_text("BTC", -1.234, "5 min"), "BTC −1,2 % en 5 min");
        assert_eq!(reaction_text("BTC", 0.31, "5 min"), "BTC +0,3 % en 5 min");
        assert_eq!(reaction_text("BTC", -0.01, "1 h"), "BTC 0,0 % en 1 h");
    }
}
