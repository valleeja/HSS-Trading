//! 📅 The « Réel » of the calendar for the page (PC and phone): the `/api/calendar` relay fills `actual` itself with
//! the same functions as the desktop app (`argos_data::reel`: BLS public API v1 and the Fed), so nothing changes in
//! the page. 📰 And the news feeds of the relay are the app's (`argos_data::eco::FEEDS`).

use argos_data::eco::Event;
use argos_data::reel::Reel;
use argos_outils::serveur::Serveur;
use serde_json::Value;
use std::sync::{Arc, Mutex};

/// Fills the empty `actual` of the Forex Factory week (`raw`, its JSON) at `now`: what was read before, and what is
/// due now (one BLS request for all of them, the Fed's statement), read with `lire`. Anything unreadable: `raw` as is.
pub fn remplir(raw: &[u8], reel: &mut Reel, now: i64, lire: &dyn Fn(&mut Reel, &mut [Event], i64)) -> Vec<u8> {
    let Ok(mut v) = serde_json::from_slice::<Value>(raw) else {
        return raw.to_vec();
    };
    let Some(list) = v.as_array_mut() else { return raw.to_vec() };
    let mut events: Vec<Event> = list.iter().map(|e| argos_data::eco::event_of(e).unwrap_or_default()).collect();
    lire(reel, &mut events, now);
    for (o, e) in list.iter_mut().zip(&events) {
        if !e.actual.is_empty() && o["actual"].as_str().is_none_or(str::is_empty) {
            o["actual"] = Value::from(e.actual.clone());
        }
    }
    serde_json::to_vec(&v).unwrap_or_else(|_| raw.to_vec())
}

/// The real reading: BLS and the Fed, through the app's network client (white list).
fn lire_vraiment(reel: &mut Reel, events: &mut [Event], now: i64) {
    if reel.due(events, now).is_empty() {
        reel.fill(events);
        return;
    }
    let c = argos_data::rt::Client::default();
    let _ = argos_data::rt::bloquer(reel.read(&c, events, now));
}

/// Plugs the « Réel » into the server's `/api/calendar` relay (the readings are kept while the server runs).
pub fn brancher(s: &mut Serveur) {
    let reel = Arc::new(Mutex::new(Reel::default()));
    s.calendrier = Some(Box::new(move |raw: &[u8]| {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() as i64);
        match reel.lock() {
            Ok(mut r) => remplir(raw, &mut r, now, &lire_vraiment),
            Err(_) => raw.to_vec(),
        }
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 📅 The relay fills an empty « Réel » (and only an empty one) with what was read; anything else is kept.
    #[test]
    fn le_relais_remplit_le_reel() {
        let ff = br#"[{"title":"CPI m/m","country":"USD","date":"2026-11-13T08:30:00-05:00","impact":"High","forecast":"0.3%","previous":"0.2%","actual":""},
                      {"title":"GDP q/q","country":"USD","date":"2026-11-13T08:30:00-05:00","impact":"High","forecast":"1.0%","previous":"","actual":"1.2%"}]"#;
        let mut r = Reel::default();
        let lu = std::cell::Cell::new(0);
        let lire = |r: &mut Reel, ev: &mut [Event], now: i64| {
            lu.set(r.due(ev, now).len());
            r.values.insert(("CPI m/m".into(), ev[0].ts), "0.4%".into());
            r.fill(ev);
        };
        let out = remplir(ff, &mut r, 1_794_576_600 + 60, &lire);
        let v: Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(v[0]["actual"], "0.4%");
        assert_eq!(v[1]["actual"], "1.2%", "kept");
        assert_eq!(v[0]["forecast"], "0.3%");
        assert_eq!(lu.get(), 1, "the CPI was due at +1 min");
        assert_eq!(remplir(b"pas du json", &mut r, 0, &lire), b"pas du json");
    }

    /// 📰 The relay reads the same feeds as the app (one list: the app's).
    #[test]
    fn memes_flux_que_l_app() {
        let app: Vec<(&str, &str, &str)> = argos_data::eco::FEEDS.to_vec();
        let relais: Vec<(&str, &str, &str)> = argos_outils::serveur::NEWS_FEEDS.to_vec();
        assert_eq!(app, relais);
    }
}
