//! 📚 Côté rendu du catalogue unique (`argos_script::catalogue`) : la fiche d'un `Ind`, l'`Ind` d'un identifiant, et
//! l'inscription des réglages tenus hors d'argos-script (carte du carnet et des liquidations ici, Lens dans
//! argos-lens).

use crate::indicators::Ind;
use argos_script::catalogue::{self, Fiche};
use argos_script::inputs::Param;

/// La fiche d'un indicateur (`FICHES` est dans l'ordre de `Ind::ALL`).
pub fn fiche(i: Ind) -> &'static Fiche {
    &catalogue::FICHES[i as usize]
}

/// L'indicateur d'un identifiant du catalogue.
pub fn ind(id: &str) -> Option<Ind> {
    Ind::ALL.iter().copied().find(|i| fiche(*i).id == id)
}

/// What the pickers' search reads for an indicator: name, category and search words (« carnet », « order book »…).
pub fn texte_recherche(i: Ind) -> String {
    catalogue::texte_recherche(fiche(i).id, i.name())
}

/// The order of the PC indicator list: classics first (catalogue order), then each other section grouped by ƒx
/// category, in the categories' order (Argos Murs first of « Order flow / carnet »); the ones in no category last.
pub fn ordre_liste() -> Vec<(usize, Ind)> {
    let all: Vec<(usize, Ind)> = Ind::ALL.iter().copied().enumerate().collect();
    let mut classiques: Vec<(usize, Ind)> = all.iter().copied().filter(|(_, i)| i.section() == "CLASSIQUES").collect();
    let mut autres: Vec<(usize, Ind)> = all.iter().copied().filter(|(_, i)| i.section() != "CLASSIQUES").collect();
    // (stable: within a section and outside the categories, the catalogue order stays)
    autres.sort_by_key(|(k, i)| (i.section(), catalogue::rang_fx(fiche(*i).id).unwrap_or((usize::MAX, *k))));
    classiques.append(&mut autres);
    classiques
}

/// Les réglages d'un indicateur (groupes, table), complément inscrit au premier appel.
pub fn reglages(i: Ind) -> (&'static [&'static str], &'static [Param]) {
    static INSCRIT: std::sync::Once = std::sync::Once::new();
    INSCRIT.call_once(|| catalogue::inscrire_complement(complement));
    catalogue::reglages(fiche(i))
}

/// Les réglages tenus hors d'argos-script.
fn complement(id: &str) -> Option<(&'static [&'static str], &'static [Param])> {
    Some(match id {
        "BookHeat" => (&crate::bookheat::GROUPS, &crate::bookheat::PARAMS),
        "LiqMap" => (&crate::liqview::GROUPS, &crate::liqview::PARAMS),
        "ArgosMurs" => (&crate::murs::GROUPS, &crate::murs::PARAMS),
        "Lens" => (&argos_lens::reglages::GROUPS, &argos_lens::reglages::PARAMS),
        _ => return None,
    })
}

/// Version of `indicator_settings.json`.
pub const FORMAT_REGLAGES: u32 = 1;

/// 📚 Indicator settings saved by name (`indicator_settings.json`, step 2 of the Argos Script engine): built-in
/// indicators by their catalogue id, then each `Param::key` (see `argos_script::catalogue::par_nom`). `scripts`: user
/// scripts (step 3), by their stable script id, then each input's title. Unknown ids and
/// names read from the file are kept when it is written again.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ReglagesParNom {
    pub format: u32,
    pub indicateurs: std::collections::BTreeMap<String, std::collections::BTreeMap<String, f64>>,
    pub scripts: std::collections::BTreeMap<String, std::collections::BTreeMap<String, ValeurScript>>,
    /// 📋 (tableaux-1, appended) Each table's place and size chosen by the user, by table key (`tableaux::cle`, or
    /// `script:<id>:<n>`).
    pub tableaux: std::collections::BTreeMap<String, crate::tableaux::TableLayout>,
}

/// ⚙ A chosen value of a user script's input (step 3), as written in the file: a number, a yes / no, a text, or a
/// colour (`{"couleur": 0xRRGGBB}`, so it is not mistaken for a number).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum ValeurScript {
    Bool(bool),
    Num(f64),
    Text(String),
    Couleur { couleur: u32 },
}

impl From<&argos_script::lang::InVal> for ValeurScript {
    fn from(v: &argos_script::lang::InVal) -> Self {
        use argos_script::lang::InVal;
        match v {
            InVal::Num(x) => ValeurScript::Num(*x),
            InVal::Bool(b) => ValeurScript::Bool(*b),
            InVal::Text(t) => ValeurScript::Text(t.clone()),
            InVal::Color(c) => ValeurScript::Couleur { couleur: *c },
        }
    }
}

impl From<&ValeurScript> for argos_script::lang::InVal {
    fn from(v: &ValeurScript) -> Self {
        use argos_script::lang::InVal;
        match v {
            ValeurScript::Num(x) => InVal::Num(*x),
            ValeurScript::Bool(b) => InVal::Bool(*b),
            ValeurScript::Text(t) => InVal::Text(t.clone()),
            ValeurScript::Couleur { couleur } => InVal::Color(*couleur),
        }
    }
}

impl ReglagesParNom {
    /// The settings by position the engine uses (indicators whose id is unknown are left out).
    pub fn par_position(&self) -> crate::indicators::Settings {
        self.indicateurs
            .iter()
            .filter_map(|(id, noms)| {
                let i = ind(id)?;
                Some((i, catalogue::par_position(reglages(i).1, noms)))
            })
            .collect()
    }

    /// 📜 Renames the saved settings of the natives replaced by their script (`catalogue::CONVERSIONS`) to the
    /// script's keys. `true` if something changed (once: a converted file has no old key left).
    pub fn convertir(&mut self) -> bool {
        let mut change = false;
        for c in &catalogue::CONVERSIONS {
            if let Some(n) = self.indicateurs.get(c.id).and_then(|noms| c.depuis_noms(noms)) {
                self.indicateurs.insert(c.id.to_string(), n);
                change = true;
            }
        }
        change
    }

    /// This file with the settings `set` written in (by name), everything else it had kept.
    pub fn avec(mut self, set: &crate::indicators::Settings) -> ReglagesParNom {
        self.format = FORMAT_REGLAGES;
        for (i, vals) in set {
            let noms = self.indicateurs.entry(fiche(*i).id.to_string()).or_default();
            // (names this version does not know, from a newer file, stay)
            noms.retain(|n, _| !reglages(*i).1.iter().any(|p| n == p.key || n.starts_with(&format!("{}@", p.key))) && !n.starts_with('#'));
            noms.extend(catalogue::par_nom(reglages(*i).1, vals));
        }
        self
    }
}

/// 📜 Settings read from the old by-position file (`indicator_inputs.json`): the natives replaced by their script
/// (`catalogue::CONVERSIONS`) get the script's list, from their old positions. `true` if something changed.
pub fn convertir_positions(set: &mut crate::indicators::Settings) -> bool {
    let mut change = false;
    for c in &catalogue::CONVERSIONS {
        let Some(i) = ind(c.id) else { continue };
        if let Some(v) = set.get_mut(&i) {
            *v = catalogue::par_position(reglages(i).1, &c.depuis_positions(v));
            change = true;
        }
    }
    change
}

/// 📜 The by-position list the old /app/ reads for `i` (`indicator_inputs.json`): the native's layout for a native
/// replaced by its script, else the list as it is.
pub fn positions_anciennes(i: Ind, vals: &[f64]) -> Vec<f64> {
    match catalogue::conversion(fiche(i).id) {
        Some(c) => c.vers_positions(reglages(i).1, vals),
        None => vals.to_vec(),
    }
}

/// Les indicateurs officiels du Marketplace : (indicateur, catégorie, « or / scalping », description).
pub fn officiels() -> Vec<(Ind, &'static str, bool, &'static str)> {
    catalogue::MARCHE.iter().filter_map(|(id, c, g, d)| Some((ind(id)?, *c, *g, *d))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 📚 Step 2: every built-in indicator's settings go by name and come back the same (all 45, with changed values);
    /// unknown indicators and names in the file stay when it is written again; the JSON format is versioned.
    #[test]
    fn reglages_par_nom_sans_perte() {
        let mut set = crate::indicators::Settings::new();
        for (k, i) in Ind::ALL.iter().enumerate() {
            let v: Vec<f64> = reglages(*i).1.iter().enumerate().map(|(j, p)| p.def + (k + j) as f64 % 3.0).collect();
            set.insert(*i, v);
        }
        let mut ancien = ReglagesParNom::default();
        ancien.indicateurs.insert("Futur".into(), [("x".to_string(), 1.0)].into());
        ancien
            .indicateurs
            .entry(fiche(Ind::Rsi).id.to_string())
            .or_default()
            .insert("nouveau_reglage".into(), 3.0);
        let r = ancien.avec(&set);
        assert_eq!(r.format, FORMAT_REGLAGES);
        assert_eq!(r.par_position(), set, "same values back");
        assert!(r.indicateurs.contains_key("Futur"), "unknown indicator kept");
        assert_eq!(r.indicateurs[fiche(Ind::Rsi).id]["nouveau_reglage"], 3.0, "unknown name kept");
        let j = serde_json::to_string(&r).unwrap();
        assert_eq!(serde_json::from_str::<ReglagesParNom>(&j).unwrap(), r);
        assert!(j.contains("\"format\":1"));
    }

    /// 📚 Step 1 of the Argos Script engine: the catalogue gives exactly what was there before it (measured on
    /// interface-2 792c5930 without the catalogue): same names, sections, groups and settings of the 45 indicators
    /// (816 settings, same fingerprint); same Marketplace (19 entries, 10 categories, same fingerprint); each entry is
    /// the `Ind` of the same rank; each indicator has its settings.
    /// First measured at a1190a8 (802 settings); interface-2 then added 14 settings: Argos Price Action +8 (pwr,
    /// buySell, palette, pwrMode, n4, n4Min, pwrMin, vecMult) and Argos Heatmap du carnet +6 (walls, wallsN, wallsMult,
    /// wallsDepth, wallsSell, wallsBuy).
    #[test]
    fn catalogue_identique_a_avant() {
        let fnv = |h: &mut u64, s: &str| {
            for b in s.bytes() {
                *h ^= b as u64;
                *h = h.wrapping_mul(0x100000001b3);
            }
        };
        let (mut h, mut total) = (0xcbf29ce484222325u64, 0);
        for (k, i) in Ind::ALL.iter().enumerate() {
            assert_eq!(*i as usize, k, "Ind::ALL dans l'ordre de l'enum");
            assert_eq!(fiche(*i).id, format!("{i:?}"), "fiche {k}");
            let (g, p) = i.params();
            assert!(!p.is_empty(), "{i:?} : réglages inscrits");
            // (the fingerprint is the one of the 45 built before the Argos Script built-ins, which come after them)
            if k < argos_script::catalogue::AVANT_SCRIPTS {
                total += p.len();
                fnv(&mut h, &format!("{}|{}|{:?}|{:?}", i.name(), i.section(), g, p));
            }
        }
        // (bollinger-script-1, 19 Nov: Bollinger is TradingView's script, its 9 native settings replaced by its 5
        // inputs and 10 style settings (3 plots + the fill): 816 − 9 + 15)
        // (argos-murs-2, 21 Nov: the heatmap's « Carnets lus » group, 7 settings shared with the order-book panel)
        // (footprint-multi-1, 22 Nov: the Footprint's « 🏦 Bourses » group, 14 settings: auto, the 10 exchanges, unite,
        // aligne, parts)
        assert_eq!(total, 843, "843 settings (816 on interface-2 792c5930)");
        // (bubbles-donnees-1, 19 Nov: the help of Argos Bubbles' « Afficher le POC » explains the yellow dot, was
        // 5aff4247381cbd8d; bollinger-script-1, 19 Nov: Bollinger's settings are its TradingView script's, see above)
        // (argos-murs-2, 21 Nov: the heatmap's « Carnets lus » group, was 14ddb466bf0dd277; footprint-multi-1, 22 Nov:
        // the Footprint's « 🏦 Bourses » group, was 61613a2721a49f7f; tv-classiques-1, 26 Nov: the Stochastic's %K
        // smoothing starts at 1, as TradingView's, Jee's choice, was f74671b00cb060fc)
        assert_eq!(format!("{h:016x}"), "d64c70dd2bf7644a", "noms, sections, groupes et réglages identiques");
        let mut m = 0xcbf29ce484222325u64;
        fnv(&mut m, &format!("{:?}|{:?}", officiels(), crate::marketplace::CATS));
        // (bubbles-donnees-1, 19 Nov: Argos Bubbles' description no longer says « estimé à partir des bougies »)
        assert_eq!(format!("{m:016x}"), "a5857dab8fc23e14", "Marketplace identique");
    }
}

#[cfg(test)]
mod tests_liste {
    use super::*;

    /// Jee, 21 Nov (« les placer à la bonne place »): Argos Murs is in ARGOS, first of « Order flow / carnet », on the
    /// PC list as on the ƒx page; searching « murs », « carnet », « order book » finds it; the heatmap says its book.
    #[test]
    fn argos_murs_premier_de_sa_categorie_et_trouve() {
        assert_eq!(Ind::ArgosMurs.section(), "ARGOS");
        assert_eq!(catalogue::rang_fx("ArgosMurs"), Some((6, 0)));
        let l: Vec<Ind> = ordre_liste().into_iter().map(|(_, i)| i).collect();
        assert_eq!(l.len(), Ind::ALL.len());
        let at = |i: Ind| l.iter().position(|x| *x == i).unwrap();
        assert_eq!(at(Ind::ArgosMurs) + 1, at(Ind::Footprint), "right before the rest of its category");
        assert!(l[..at(Ind::ArgosMurs)].iter().all(|i| i.section() == "CLASSIQUES" || i.section() == "ARGOS"));
        assert!(
            l.iter().skip_while(|i| i.section() == "CLASSIQUES").all(|i| i.section() != "CLASSIQUES"),
            "one CLASSIQUES block"
        );
        for q in ["murs", "carnet", "order book", "Mur", "walls"] {
            assert!(crate::i18n::plain(&texte_recherche(Ind::ArgosMurs)).contains(&crate::i18n::plain(q)), "{q}");
        }
        let out = crate::indicators::IndOut::default();
        assert!(crate::indicators::legend_title(Ind::BookHeat, &out).starts_with("Argos Heatmap du carnet · carnet Hyperliquid USDC ("));
    }
}
