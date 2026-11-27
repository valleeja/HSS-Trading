//! 🔄 Les watchlists partagées entre l'app de bureau (redb), la page web et la coque (en mémoire) : un seul format
//! JSON (`watchlists.json`, dans les réglages partagés), écrit à partir du contrat `SymbolStore` — jamais en touchant
//! au schéma de la base. Les listes se reconnaissent **par leur nom** (les numéros diffèrent d'une base à l'autre).
//! - `vers_json` : les listes d'une base, toujours dans le même ordre (liste par liste) ;
//! - `appliquer` : une base reprend les listes du JSON ; `tout` = remplacer (listes, sections et symboles absents
//!   retirés), sinon seulement ajouter ce qui manque (au démarrage de l'app de bureau : rien ne se perd).

use argos_core::store::SymbolStore;
use serde_json::{json, Value};

/// Version du format (les pages d'avant n'en avaient pas).
pub const VERSION: i64 = 2;

/// Les listes lues d'un JSON : (nom, sections, (section, symbole)).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Listes(pub Vec<(String, Vec<String>, Vec<(String, String)>)>);

/// Les listes d'une base, en JSON (même format que `MemStore::lists_json`, plus la version).
pub fn vers_json<S: SymbolStore + ?Sized>(s: &S) -> String {
    let lists = s.lists().unwrap_or_default();
    let mut sections = vec![];
    let mut items = vec![];
    for l in &lists {
        for sec in s.sections(l.id).unwrap_or_default() {
            sections.push(json!([l.id, sec]));
        }
        for (sec, id) in s.items(l.id).unwrap_or_default() {
            items.push(json!([l.id, sec, id]));
        }
    }
    json!({
        "v": VERSION,
        "lists": lists.iter().map(|l| json!([l.id, l.name])).collect::<Vec<_>>(),
        "sections": sections,
        "items": items,
    })
    .to_string()
}

/// Lit le JSON des listes (`None` s'il n'en est pas un : rien n'est appliqué). Au plus 200 listes, 5000 symboles.
pub fn lire(raw: &str) -> Option<Listes> {
    let v: Value = serde_json::from_str(raw).ok()?;
    let rows = |k: &str| v[k].as_array().cloned();
    let lists = rows("lists")?;
    let txt = |x: &Value, k: usize| x[k].as_str().map(str::to_string);
    let mut out = Vec::new();
    for l in lists.iter().take(200) {
        let (Some(id), Some(name)) = (l[0].as_i64(), txt(l, 1)) else { continue };
        if name.trim().is_empty() || out.iter().any(|(n, _, _): &(String, _, _)| *n == name) {
            continue;
        }
        let secs = rows("sections")
            .unwrap_or_default()
            .iter()
            .filter(|x| x[0].as_i64() == Some(id))
            .filter_map(|x| txt(x, 1))
            .collect();
        let items = rows("items")
            .unwrap_or_default()
            .iter()
            .filter(|x| x[0].as_i64() == Some(id))
            .filter_map(|x| Some((txt(x, 1)?, txt(x, 2)?)))
            .take(5000)
            .collect();
        out.push((name, secs, items));
    }
    Some(Listes(out))
}

/// Une base reprend ces listes ; rend `true` si quelque chose a changé. `tout` : remplacer (ce qui n'y est pas est
/// retiré) ; sinon seulement ajouter ce qui manque.
pub fn appliquer<S: SymbolStore + ?Sized>(s: &mut S, l: &Listes, tout: bool) -> bool {
    let mut change = false;
    if tout {
        for ex in s.lists().unwrap_or_default() {
            if !l.0.iter().any(|(n, _, _)| *n == ex.name) && s.delete_list(ex.id).is_ok() {
                change = true;
            }
        }
    }
    for (name, secs, items) in &l.0 {
        let avant = s.lists().unwrap_or_default().len();
        let Ok(id) = s.create_list(name) else { continue };
        change |= s.lists().unwrap_or_default().len() != avant;
        if tout {
            for (sec, it) in s.items(id).unwrap_or_default() {
                if !items.iter().any(|(s2, i2)| *s2 == sec && *i2 == it) && s.remove_item(id, &it).is_ok() {
                    change = true;
                }
            }
            for sec in s.sections(id).unwrap_or_default() {
                if !secs.contains(&sec) && s.remove_section(id, &sec).is_ok() {
                    change = true;
                }
            }
        }
        let cur_secs = s.sections(id).unwrap_or_default();
        for sec in secs.iter().filter(|x| !cur_secs.contains(x)) {
            change |= s.add_section(id, sec).is_ok();
        }
        let cur = s.items(id).unwrap_or_default();
        for (sec, it) in items.iter().filter(|x| !cur.iter().any(|c| c.1 == x.1)) {
            change |= s.add_item(id, sec, it).is_ok();
        }
    }
    change
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemStore;

    fn base(noms: &[(&str, &[(&str, &str)])]) -> MemStore {
        let mut m = MemStore::default();
        for (n, items) in noms {
            let id = m.create_list(n).unwrap();
            for (sec, it) in *items {
                if !sec.is_empty() {
                    m.add_section(id, sec).unwrap();
                }
                m.add_item(id, sec, it).unwrap();
            }
        }
        m
    }

    #[test]
    fn aller_retour_et_ordre_stable() {
        let m = base(&[("Mes favoris", &[("", "hl:BTC"), ("", "hl:ETH")]), ("Or", &[("Métaux", "oanda:XAUUSD")])]);
        let j = vers_json(&m);
        assert!(j.contains("\"v\":2"));
        let back = MemStore::from_lists_json(&j);
        assert_eq!(vers_json(&back), j, "relu puis réécrit : identique (pas de va-et-vient)");
        assert_eq!(lire(&j).unwrap().0.len(), 2);
        for mauvais in ["", "pas json", "{}", "{\"lists\": 3}"] {
            assert_eq!(lire(mauvais), None, "{mauvais}");
        }
    }

    #[test]
    fn remplacer_ou_seulement_ajouter() {
        let distant = lire(&vers_json(&base(&[
            ("Mes favoris", &[("", "hl:BTC"), ("", "hl:SOL")]),
            ("Crypto", &[("L1", "binance:ETH")]),
        ])))
        .unwrap();
        // au démarrage : on ajoute seulement (la liste locale « Actions » reste, BTC reste, SOL arrive)
        let mut local = base(&[("Mes favoris", &[("", "hl:BTC"), ("", "hl:ETH")]), ("Actions", &[("", "hl:xyz:NVDA")])]);
        assert!(appliquer(&mut local, &distant, false));
        let noms: Vec<String> = local.lists().unwrap().into_iter().map(|l| l.name).collect();
        assert_eq!(noms, ["Mes favoris", "Actions", "Crypto"]);
        let fav = local.lists().unwrap()[0].id;
        assert_eq!(local.items(fav).unwrap().len(), 3, "BTC, ETH et SOL");
        // en direct : on remplace (Actions et ETH partent)
        assert!(appliquer(&mut local, &distant, true));
        let noms: Vec<String> = local.lists().unwrap().into_iter().map(|l| l.name).collect();
        assert_eq!(noms, ["Mes favoris", "Crypto"]);
        let ids: Vec<String> = local.items(fav).unwrap().into_iter().map(|i| i.1).collect();
        assert_eq!(ids, ["hl:BTC", "hl:SOL"]);
        let crypto = local.lists().unwrap()[1].id;
        assert_eq!(local.sections(crypto).unwrap(), ["L1"]);
        // une 2e fois : plus rien ne change
        assert!(!appliquer(&mut local, &distant, true));
    }
}
