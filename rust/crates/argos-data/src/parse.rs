//! Petits outils de lecture JSON partagés par les exchanges (les exchanges envoient des nombres en texte ou en nombre).

use argos_core::{ArgosError, Bar, Result};
use serde_json::Value;

pub fn json(body: &str) -> Result<Value> {
    serde_json::from_str(body).map_err(|e| ArgosError::Feed(format!("réponse illisible : {e}")))
}

/// Nombre, qu'il arrive en texte (« "80123.5" ») ou en nombre.
pub fn num(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => s.parse().unwrap_or(f64::NAN),
        _ => f64::NAN,
    }
}

/// Première clé présente parmi plusieurs noms possibles.
pub fn field<'a>(o: &'a Value, keys: &[&str]) -> &'a Value {
    keys.iter().map(|k| &o[*k]).find(|v| !v.is_null()).unwrap_or(&Value::Null)
}

/// Temps en millisecondes ou en secondes → secondes.
pub fn secs(v: &Value) -> i64 {
    let t = num(v);
    if !t.is_finite() {
        return 0;
    }
    if t > 1e11 {
        (t / 1000.0) as i64
    } else {
        t as i64
    }
}

/// Bougie en tableau `[temps, ouverture, haut, bas, clôture, volume, ...]`.
pub fn bar_from_array(a: &Value) -> Bar {
    bar_from_array_vol(a, 5)
}

/// A candle written as an array whose volume in coins is at `vol` (OKX and BloFin perps count contracts at 5 and
/// coins at 6).
pub fn bar_from_array_vol(a: &Value, vol: usize) -> Bar {
    Bar {
        time: secs(&a[0]),
        open: num(&a[1]),
        high: num(&a[2]),
        low: num(&a[3]),
        close: num(&a[4]),
        volume: num(&a[vol]),
    }
}

/// Bougie en objet, avec les noms de champs les plus courants.
pub fn bar_from_object(o: &Value) -> Bar {
    Bar {
        time: secs(field(o, &["time", "openTime", "t", "start", "timestamp", "ts"])),
        open: num(field(o, &["open", "o"])),
        high: num(field(o, &["high", "h"])),
        low: num(field(o, &["low", "l"])),
        close: num(field(o, &["close", "c"])),
        volume: num(field(o, &["baseVol", "volume", "vol", "v"])),
    }
}

/// Bougie valable (temps et prix lus) ; sinon `None` — pour les messages du direct.
pub fn valid(b: Bar) -> Option<Bar> {
    (b.time > 0 && [b.open, b.high, b.low, b.close].iter().all(|v| v.is_finite())).then_some(b)
}

/// Une liste de bougies, en tableaux ou en objets.
pub fn bars(list: &Value) -> Vec<Bar> {
    list.as_array()
        .map(|a| a.iter().map(|k| if k.is_array() { bar_from_array(k) } else { bar_from_object(k) }).collect())
        .unwrap_or_default()
}

/// Vérifie le code de réponse d'un exchange (`"0"` ou `0` = OK).
/// Texte d'un champ, qu'il arrive en texte ou en nombre (vide sinon).
pub fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

/// Décimales : un nombre de décimales (« 2 ») ou un pas écrit en texte (« 0.10 » → 1).
pub fn decimals(v: &Value) -> u32 {
    match v {
        Value::Number(n) => n.as_u64().unwrap_or(0) as u32,
        Value::String(s) if s.contains('.') => argos_core::instrument::decimals_of(s),
        Value::String(s) => s.parse().unwrap_or(0),
        _ => 0,
    }
}

pub fn ok_code(v: &Value, key: &str, exchange: &str) -> Result<()> {
    let c = &v[key];
    let ok = c.is_null() || c == "0" || c == 0 || c == 200;
    if ok {
        Ok(())
    } else {
        let msg = field(v, &["msg", "retMsg", "message", "error"]);
        Err(ArgosError::Feed(format!("{exchange} : {} ({c})", msg.as_str().unwrap_or("erreur"))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nombres_texte_ou_nombre() {
        assert_eq!(num(&json("\"1.5\"").unwrap()), 1.5);
        assert_eq!(num(&json("2").unwrap()), 2.0);
        assert!(num(&Value::Null).is_nan());
        assert_eq!(secs(&json("1700000000000").unwrap()), 1_700_000_000);
        assert_eq!(secs(&json("1700000000").unwrap()), 1_700_000_000);
    }

    #[test]
    fn bougies_tableau_et_objet() {
        let a = bars(
            &json(r#"[["1700000000000","1","3","0.5","2","10"],{"time":1700000060000,"open":"2","high":3,"low":1,"close":"2.5","baseVol":"7"}]"#).unwrap(),
        );
        assert_eq!(
            a[0],
            Bar {
                time: 1_700_000_000,
                open: 1.0,
                high: 3.0,
                low: 0.5,
                close: 2.0,
                volume: 10.0
            }
        );
        assert_eq!(a[1].close, 2.5);
        assert_eq!(a[1].volume, 7.0);
    }
}
