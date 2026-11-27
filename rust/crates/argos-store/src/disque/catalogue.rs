//! 🗂️ Le catalogue synchronisé (`argos_core::catalogue`) dans la base de l'appareil : une table par sorte
//! d'élément (sources, symboles, watchlists), une ligne par élément (`id` → JSON de sa version). L'identifiant
//! de l'appareil et son horloge sont dans `meta` : propres à l'appareil, jamais synchronisés comme des données.
//! Aucune clé ni secret ici (les clés restent au trousseau).

use super::{err, Store, META};
use argos_core::catalogue::{Appareil, Estampille, Horloge, Paquet, SymboleId, Versionne, Watchlist};
use argos_core::{ArgosError, Result};
use redb::{ReadableDatabase, ReadableTable, TableDefinition};
use serde::de::DeserializeOwned;
use serde::Serialize;

/// Sorte d'élément du catalogue (une table chacune).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sorte {
    Sources,
    Symboles,
    Watchlists,
}

impl Sorte {
    fn table(self) -> TableDefinition<'static, &'static str, &'static str> {
        TableDefinition::new(match self {
            Sorte::Sources => "cat_sources",
            Sorte::Symboles => "cat_symboles",
            Sorte::Watchlists => "cat_watchlists",
        })
    }
}

fn json_err(e: serde_json::Error) -> ArgosError {
    ArgosError::Invalid(format!("catalogue : {e}"))
}

impl Store {
    /// L'identifiant de cet appareil, tiré une fois au hasard et gardé dans la base de l'appareil.
    pub fn appareil(&mut self) -> Result<Appareil> {
        if let Some(a) = self.meta_lire("appareil")? {
            return Ok(Appareil(a));
        }
        let mut octets = [0u8; 8];
        getrandom::fill(&mut octets).map_err(|e| ArgosError::Invalid(format!("hasard : {e}")))?;
        let a: String = octets.iter().map(|b| format!("{b:02x}")).collect();
        self.meta_ecrire("appareil", &a)?;
        Ok(Appareil(a))
    }

    fn horloge(&mut self) -> Result<Horloge> {
        let mut h = Horloge::new(self.appareil()?);
        h.derniere = self.meta_lire("horloge")?.and_then(|s| s.parse().ok()).unwrap_or(0);
        Ok(h)
    }

    /// Estampille d'une modification faite sur cet appareil (`maintenant_ms` : son heure) ; l'horloge est gardée.
    pub fn estampille(&mut self, maintenant_ms: i64) -> Result<Estampille> {
        let mut h = self.horloge()?;
        let e = h.estampille(maintenant_ms);
        self.meta_ecrire("horloge", &h.derniere.to_string())?;
        Ok(e)
    }

    /// La version gardée d'un élément (effacé compris).
    pub fn catalogue_lire<T: DeserializeOwned>(&self, sorte: Sorte, id: &str) -> Result<Option<Versionne<T>>> {
        let rx = self.db.begin_read().map_err(err)?;
        let t = match rx.open_table(sorte.table()) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(e) => return Err(err(e)),
        };
        let v = t.get(id).map_err(err)?.map(|g| g.value().to_string());
        v.map(|s| serde_json::from_str(&s).map_err(json_err)).transpose()
    }

    /// Tous les éléments d'une sorte, par identifiant (effacés compris : la synchro en a besoin).
    pub fn catalogue_lister<T: DeserializeOwned>(&self, sorte: Sorte) -> Result<Vec<(String, Versionne<T>)>> {
        let rx = self.db.begin_read().map_err(err)?;
        let t = match rx.open_table(sorte.table()) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(vec![]),
            Err(e) => return Err(err(e)),
        };
        let mut out = vec![];
        for row in t.iter().map_err(err)? {
            let (k, v) = row.map_err(err)?;
            out.push((k.value().to_string(), serde_json::from_str(v.value()).map_err(json_err)?));
        }
        Ok(out)
    }

    /// Fusionne une version (d'ici ou reçue d'un autre appareil) avec celle gardée : la plus récente reste. L'horloge
    /// de l'appareil voit l'estampille. `true` si la version gardée a changé.
    pub fn catalogue_fusionner<T: Serialize + DeserializeOwned + Clone + PartialEq>(&mut self, sorte: Sorte, id: &str, v: &Versionne<T>) -> Result<bool> {
        let mut h = self.horloge()?;
        h.voir(&v.estampille);
        self.meta_ecrire("horloge", &h.derniere.to_string())?;
        let gardee: Option<Versionne<T>> = self.catalogue_lire(sorte, id)?;
        let nouvelle = gardee.as_ref().map_or_else(|| v.clone(), |g| g.fusion(v));
        if gardee.as_ref() == Some(&nouvelle) {
            return Ok(false);
        }
        let json = serde_json::to_string(&nouvelle).map_err(json_err)?;
        let tx = self.db.begin_write().map_err(err)?;
        tx.open_table(sorte.table()).map_err(err)?.insert(id, json.as_str()).map_err(err)?;
        tx.commit().map_err(err)?;
        Ok(true)
    }

    /// 🔄 Paquet de synchro pour un autre appareil : tout ce qui est plus récent que `depuis_ms` (son dernier passage).
    pub fn paquet_depuis(&self, depuis_ms: i64) -> Result<Paquet> {
        fn recents<T: DeserializeOwned>(s: &Store, sorte: Sorte, depuis_ms: i64) -> Result<Vec<(String, Versionne<T>)>> {
            Ok(s.catalogue_lister::<T>(sorte)?
                .into_iter()
                .filter(|(_, v)| v.estampille.ms > depuis_ms)
                .collect())
        }
        Ok(Paquet {
            sources: recents(self, Sorte::Sources, depuis_ms)?,
            symboles: recents(self, Sorte::Symboles, depuis_ms)?,
            watchlists: recents(self, Sorte::Watchlists, depuis_ms)?,
        })
    }

    /// Applique un paquet reçu (élément par élément, la plus récente version reste) ; nombre d'éléments changés.
    pub fn appliquer_paquet(&mut self, p: &Paquet) -> Result<usize> {
        let mut n = 0;
        for (id, v) in &p.sources {
            n += usize::from(self.catalogue_fusionner(Sorte::Sources, id, v)?);
        }
        for (id, v) in &p.symboles {
            n += usize::from(self.catalogue_fusionner(Sorte::Symboles, id, v)?);
        }
        for (id, v) in &p.watchlists {
            n += usize::from(self.catalogue_fusionner(Sorte::Watchlists, id, v)?);
        }
        Ok(n)
    }

    /// 🔄 Reprend les watchlists actuelles (listes, sections, symboles, dans leur ordre) dans le nouveau format, une
    /// seule fois. Sans perte et réversible : l'ancien bloc (`meta["watchlists"]`) n'est jamais touché, chaque
    /// symbole garde son identifiant d'avant (`ancien`), et un symbole que `convertir` ne reconnaît pas (identifiant
    /// « exchange:symbole » → identifiant TradingView) est gardé tel quel, « à vérifier ». Renvoie le nombre de listes
    /// reprises (0 si déjà fait).
    pub fn reprendre_watchlists(&mut self, maintenant_ms: i64, convertir: impl Fn(&str) -> Option<SymboleId>) -> Result<usize> {
        if self.meta_lire("reprise_watchlists")?.is_some() {
            return Ok(0);
        }
        let ancien = crate::listes::lire(&crate::listes::vers_json(&self.mem)).unwrap_or_default();
        let appareil = self.appareil()?;
        let listes = vers_watchlists(&ancien, &appareil, &convertir);
        for l in &listes {
            let v = Versionne {
                valeur: Some(l.clone()),
                estampille: self.estampille(maintenant_ms)?,
            };
            self.catalogue_fusionner(Sorte::Watchlists, &l.id, &v)?;
        }
        self.meta_ecrire("reprise_watchlists", "1")?;
        Ok(listes.len())
    }

    fn meta_lire(&self, cle: &str) -> Result<Option<String>> {
        let rx = self.db.begin_read().map_err(err)?;
        let t = rx.open_table(META).map_err(err)?;
        let v = t.get(cle).map_err(err)?.map(|g| g.value().to_string());
        Ok(v)
    }

    fn meta_ecrire(&mut self, cle: &str, valeur: &str) -> Result<()> {
        let tx = self.db.begin_write().map_err(err)?;
        tx.open_table(META).map_err(err)?.insert(cle, valeur).map_err(err)?;
        tx.commit().map_err(err)
    }
}

/// Anciennes listes (nom, sections, (section, symbole)) → watchlists : la section sans nom en haut, puis les
/// sections dans leur ordre, chaque symbole à sa place (avec son identifiant d'avant) ; privées, sans drapeau ni note.
fn vers_watchlists(ancien: &crate::listes::Listes, appareil: &Appareil, convertir: &impl Fn(&str) -> Option<SymboleId>) -> Vec<Watchlist> {
    use argos_core::catalogue::{Entree, Section};
    ancien
        .0
        .iter()
        .enumerate()
        .map(|(k, (nom, sections, items))| {
            let mut l = Watchlist::nouvelle(format!("{}-{k}", appareil.0), nom.clone());
            l.sections.extend(sections.iter().map(|s| Section {
                nom: s.clone(),
                repliee: false,
                entrees: vec![],
            }));
            for (sec, id) in items {
                let Some(s) = l.sections.iter_mut().find(|x| &x.nom == sec) else { continue };
                // inconnu : gardé tel quel (« source:symbole »), à vérifier
                let (symbole, a_verifier) = match convertir(id) {
                    Some(x) => (x, false),
                    None => {
                        let (source, paire) = id.split_once(':').unwrap_or(("?", id.as_str()));
                        let brut = SymboleId {
                            source: source.to_uppercase(),
                            paire: paire.to_string(),
                            marche: argos_core::catalogue::TypeMarche::Spot,
                        };
                        (brut, true)
                    }
                };
                s.entrees.push(Entree {
                    symbole,
                    drapeau: None,
                    note: String::new(),
                    ancien: Some(id.clone()),
                    a_verifier,
                });
            }
            l
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use argos_core::catalogue::{Estampille, Source};
    use argos_core::store::SymbolStore;

    #[test]
    fn appareil_et_horloge_gardes() {
        let dir = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/tmp")).join(format!("argos-catalogue-{}", std::process::id()));
        let path = dir.join("argos.redb");
        let (a, e1) = {
            let mut s = Store::open(&path).unwrap();
            let a = s.appareil().unwrap();
            assert_eq!(a.0.len(), 16);
            (a, s.estampille(5_000).unwrap())
        };
        let mut s = Store::open(&path).unwrap(); // réouverture : même appareil, l'horloge ne recule pas
        assert_eq!(s.appareil().unwrap(), a);
        assert!(s.estampille(1_000).unwrap() > e1, "l'heure en retard ne recule pas l'horloge");
        drop(s);
        std::fs::remove_dir_all(dir).ok();
    }

    /// Deux appareils, l'un à l'heure en retard : la modification la plus récente reste sur les deux, quel que soit
    /// l'ordre d'arrivée ; un effacement plus récent se synchronise aussi.
    #[test]
    fn deux_appareils_fusion_et_effacement() {
        let (mut pc, mut cell) = (Store::in_memory().unwrap(), Store::in_memory().unwrap());
        let mut l = Watchlist::nouvelle("l1".into(), "Or".into());
        let v1 = Versionne {
            valeur: Some(l.clone()),
            estampille: pc.estampille(1_000_000).unwrap(),
        };
        assert!(pc.catalogue_fusionner(Sorte::Watchlists, "l1", &v1).unwrap());
        assert!(cell.catalogue_fusionner(Sorte::Watchlists, "l1", &v1).unwrap());
        assert!(!cell.catalogue_fusionner(Sorte::Watchlists, "l1", &v1).unwrap(), "déjà là : rien ne change");
        // le cell (heure en retard de 10 min) renomme : sa version gagne quand même
        l.nom = "Or et argent".into();
        let v2 = Versionne {
            valeur: Some(l.clone()),
            estampille: cell.estampille(400_000).unwrap(),
        };
        assert!(v2.estampille > v1.estampille);
        cell.catalogue_fusionner(Sorte::Watchlists, "l1", &v2).unwrap();
        pc.catalogue_fusionner(Sorte::Watchlists, "l1", &v2).unwrap();
        pc.catalogue_fusionner(Sorte::Watchlists, "l1", &v1).unwrap(); // l'ancienne arrive en retard : ignorée
        let nom = |s: &Store| s.catalogue_lire::<Watchlist>(Sorte::Watchlists, "l1").unwrap().unwrap().valeur.unwrap().nom;
        assert_eq!((nom(&pc), nom(&cell)), ("Or et argent".to_string(), "Or et argent".to_string()));
        // le PC efface : l'effacement passe au cell
        let eff = Versionne::<Watchlist> {
            valeur: None,
            estampille: pc.estampille(1_000_001).unwrap(),
        };
        pc.catalogue_fusionner(Sorte::Watchlists, "l1", &eff).unwrap();
        cell.catalogue_fusionner(Sorte::Watchlists, "l1", &eff).unwrap();
        assert!(cell.catalogue_lire::<Watchlist>(Sorte::Watchlists, "l1").unwrap().unwrap().valeur.is_none());
        assert_eq!(
            cell.catalogue_lister::<Watchlist>(Sorte::Watchlists).unwrap().len(),
            1,
            "la marque « effacé » reste"
        );
        // les sortes sont rangées à part
        assert!(cell.catalogue_lister::<Source>(Sorte::Sources).unwrap().is_empty());
        let e: Estampille = cell.estampille(0).unwrap();
        assert!(e.ms > eff.estampille.ms, "l'horloge a vu l'effacement du PC");
    }

    /// Une liste qui mélange toutes les sources (Hyperliquid, Binance perp et spot, Bitstamp, OANDA, CRYPTOCAP) et un
    /// identifiant inconnu → reprise complète (l'inconnu gardé « à vérifier »), aller-retour identique ; l'ancien bloc
    /// n'a pas bougé.
    #[test]
    fn reprise_complete_et_aller_retour_identique() {
        let mut s = Store::in_memory().unwrap();
        let a = s.create_list("CCQc🚀").unwrap();
        for id in ["hl:BTC", "hl:xyz:NVDA", "binance:ZECUSDT", "binance-spot:BTCUSDT", "bitstamp:btcusd"] {
            s.add_item(a, "", id).unwrap();
        }
        for id in ["oanda:XAU_USD", "oanda:EUR_USD", "cmc:USDT.D", "inconnu:ABC"] {
            s.add_item(a, "Macro", id).unwrap();
        }
        s.add_section(a, "Vide").unwrap();
        let b = s.create_list("Spot Trading").unwrap();
        s.add_item(b, "", "bitstamp:ethusd").unwrap();
        let avant = crate::listes::vers_json(&s);

        assert_eq!(s.reprendre_watchlists(10, argos_data::tv::depuis_ancien_id).unwrap(), 2);
        assert_eq!(s.reprendre_watchlists(20, argos_data::tv::depuis_ancien_id).unwrap(), 0, "une seule fois");
        assert_eq!(crate::listes::vers_json(&s), avant, "l'ancien bloc n'a pas bougé");

        let mut listes: Vec<Watchlist> = s
            .catalogue_lister::<Watchlist>(Sorte::Watchlists)
            .unwrap()
            .into_iter()
            .filter_map(|(_, v)| v.valeur)
            .collect();
        listes.sort_by_key(|l| l.id.rsplit('-').next().and_then(|k| k.parse::<usize>().ok()));
        // retour vers l'ancien format : exactement les mêmes listes, sections, symboles et ordre
        let revenu: Vec<(String, Vec<String>, Vec<(String, String)>)> = listes
            .iter()
            .map(|l| {
                let secs = l.sections.iter().skip(1).map(|x| x.nom.clone()).collect();
                let items = l
                    .sections
                    .iter()
                    .flat_map(|x| x.entrees.iter().map(|e| (x.nom.clone(), e.ancien.clone().unwrap_or_default())))
                    .collect();
                (l.nom.clone(), secs, items)
            })
            .collect();
        assert_eq!(revenu, crate::listes::lire(&avant).unwrap().0);
        let toutes: Vec<_> = listes.iter().flat_map(|l| l.sections.iter().flat_map(|x| x.entrees.iter())).collect();
        let noms: Vec<String> = toutes.iter().map(|e| e.symbole.to_string()).collect();
        for n in [
            "HYPERLIQUID:BTCUSDC.P",
            "BINANCE:ZECUSDT.P",
            "BINANCE:BTCUSDT",
            "BITSTAMP:BTCUSD",
            "OANDA:XAUUSD",
            "OANDA:EURUSD",
            "CRYPTOCAP:USDT.D",
        ] {
            assert!(noms.contains(&n.to_string()), "{n} : {noms:?}");
        }
        let a_verifier: Vec<_> = toutes.iter().filter(|e| e.a_verifier).map(|e| e.ancien.clone().unwrap_or_default()).collect();
        assert_eq!(a_verifier, vec!["inconnu:ABC".to_string()], "seul l'inconnu est à vérifier");
        assert!(listes.iter().all(|l| !l.publiee), "privées par défaut");
    }

    /// 🔍 Les vraies listes de Jee, sur une COPIE de son `watchlists.json` (jamais l'original) : `ARGOS_COPIE_WATCHLISTS`
    /// donne la copie ; la reprise se fait dans une base en mémoire. Affiche seulement des nombres.
    #[test]
    fn reprise_sur_une_copie_des_vraies_listes() {
        let Ok(copie) = std::env::var("ARGOS_COPIE_WATCHLISTS") else { return };
        let json = std::fs::read_to_string(copie).unwrap();
        let mut s = Store::in_memory().unwrap();
        crate::listes::appliquer(&mut s, &crate::listes::lire(&json).expect("format des listes"), true);
        let avant = crate::listes::vers_json(&s);
        let ancien = crate::listes::lire(&avant).unwrap_or_default();
        let n = s.reprendre_watchlists(0, argos_data::tv::depuis_ancien_id).unwrap();
        let listes: Vec<Watchlist> = s
            .catalogue_lister::<Watchlist>(Sorte::Watchlists)
            .unwrap()
            .into_iter()
            .filter_map(|(_, v)| v.valeur)
            .collect();
        let entrees: Vec<_> = listes.iter().flat_map(|l| l.sections.iter().flat_map(|x| x.entrees.iter())).collect();
        let symboles: usize = ancien.0.iter().map(|l| l.2.len()).sum();
        eprintln!(
            "RESULTAT listes {}/{} · symboles {}/{} · à vérifier {}",
            n,
            ancien.0.len(),
            entrees.len(),
            symboles,
            entrees.iter().filter(|e| e.a_verifier).count()
        );
        assert_eq!((n, entrees.len()), (ancien.0.len(), symboles), "toutes les listes, tous les symboles");
        assert_eq!(crate::listes::vers_json(&s), avant, "l'ancien bloc n'a pas bougé");
    }

    /// 🔄 Synchro par paquets entre deux appareils (l'un à l'heure en retard) : chacun envoie ce qui a changé depuis
    /// le dernier échange ; ils arrivent au même état, et renvoyer le même paquet ne change plus rien.
    #[test]
    fn synchro_par_paquets_entre_deux_appareils() {
        use argos_core::catalogue::{Symbole, SymboleId};
        let (mut pc, mut cell) = (Store::in_memory().unwrap(), Store::in_memory().unwrap());
        let ecrire = |s: &mut Store, l: Watchlist, t: i64| {
            let v = Versionne {
                valeur: Some(l.clone()),
                estampille: s.estampille(t).unwrap(),
            };
            s.catalogue_fusionner(Sorte::Watchlists, &l.id, &v).unwrap();
        };
        ecrire(&mut pc, Watchlist::nouvelle("a".into(), "Crypto".into()), 1_000_000);
        ecrire(&mut cell, Watchlist::nouvelle("b".into(), "Or".into()), 400_000); // heure du cell en retard
        let sym = Symbole {
            id: SymboleId::lire("BINANCE:BTCUSDT.P").unwrap(),
            nom: "Bitcoin".into(),
            logo: "BTC".into(),
        };
        let v = Versionne {
            valeur: Some(sym),
            estampille: pc.estampille(1_000_001).unwrap(),
        };
        pc.catalogue_fusionner(Sorte::Symboles, "BINANCE:BTCUSDT.P", &v).unwrap();

        // premier échange : tout
        let (p_pc, p_cell) = (pc.paquet_depuis(0).unwrap(), cell.paquet_depuis(0).unwrap());
        assert_eq!(cell.appliquer_paquet(&p_pc).unwrap(), 2);
        assert_eq!(pc.appliquer_paquet(&p_cell).unwrap(), 1);
        assert_eq!(pc.paquet_depuis(0).unwrap(), cell.paquet_depuis(0).unwrap(), "même état");
        assert_eq!(cell.appliquer_paquet(&p_pc).unwrap(), 0, "le même paquet ne change plus rien");

        // le cell (en retard) renomme « Crypto » après l'avoir reçu : son changement est plus récent et passe au PC
        // moment du dernier échange : la plus récente estampille connue
        let tout = pc.paquet_depuis(0).unwrap();
        let depuis = tout
            .watchlists
            .iter()
            .map(|(_, v)| v.estampille.ms)
            .chain(tout.symboles.iter().map(|(_, v)| v.estampille.ms))
            .max()
            .unwrap();
        let mut a = Watchlist::nouvelle("a".into(), "Crypto 🚀".into());
        a.favori = true;
        ecrire(&mut cell, a, 400_100);
        let p = cell.paquet_depuis(depuis).unwrap();
        assert_eq!(p.watchlists.len(), 1, "seulement ce qui a changé depuis le dernier échange");
        assert!(p.symboles.is_empty() && p.sources.is_empty());
        assert_eq!(pc.appliquer_paquet(&p).unwrap(), 1);
        let nom = pc.catalogue_lire::<Watchlist>(Sorte::Watchlists, "a").unwrap().unwrap().valeur.unwrap().nom;
        assert_eq!(nom, "Crypto 🚀");
        assert!(pc.paquet_depuis(i64::MAX).unwrap().est_vide());
    }
}
