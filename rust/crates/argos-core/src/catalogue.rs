//! 🗂️ Catalogue synchronisé entre les appareils : sources de données, symboles et watchlists.
//! Chaque élément porte une **estampille** (moment + appareil) et se fusionne élément par élément : la plus récente
//! gagne, l'appareil départage deux estampilles égales. Un élément effacé reste comme marque « effacé », pour que
//! l'effacement se synchronise lui aussi.
//! Le moment vient d'une **horloge qui ne recule jamais** : nouvelle estampille = max(heure de l'appareil, dernière
//! estampille vue + 1), y compris celles reçues des autres appareils ; un appareil à l'heure en retard ne fait donc
//! jamais perdre une modification plus récente.
//! Rien ici ne lit l'horloge du système ni le réseau : l'hôte donne l'heure. Aucune clé ni secret dans ces types,
//! et une source ne passe jamais d'ordre (elle ne sert qu'à lire des données de marché).

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;

/// Identifiant d'un appareil (tiré une fois au hasard, gardé dans les réglages de l'appareil).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Appareil(pub String);

/// Moment d'une modification : millisecondes de l'horloge qui ne recule jamais, puis l'appareil qui l'a faite.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Estampille {
    pub ms: i64,
    pub appareil: Appareil,
}

impl Ord for Estampille {
    fn cmp(&self, o: &Self) -> Ordering {
        (self.ms, &self.appareil).cmp(&(o.ms, &o.appareil))
    }
}

impl PartialOrd for Estampille {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

/// 🕰️ L'horloge d'un appareil : ne recule jamais, et avance au-delà de tout ce qu'elle a vu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Horloge {
    pub appareil: Appareil,
    /// Dernier moment donné ou vu (ms).
    pub derniere: i64,
}

impl Horloge {
    pub fn new(appareil: Appareil) -> Self {
        Self { appareil, derniere: 0 }
    }

    /// Estampille d'une modification faite maintenant (`maintenant_ms` : l'heure de l'appareil).
    pub fn estampille(&mut self, maintenant_ms: i64) -> Estampille {
        self.derniere = maintenant_ms.max(self.derniere.saturating_add(1));
        Estampille {
            ms: self.derniere,
            appareil: self.appareil.clone(),
        }
    }

    /// Une estampille reçue d'ailleurs : les suivantes de cet appareil seront plus récentes.
    pub fn voir(&mut self, e: &Estampille) {
        self.derniere = self.derniere.max(e.ms);
    }
}

/// Un élément avec son estampille ; `valeur` vide = effacé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Versionne<T> {
    pub valeur: Option<T>,
    pub estampille: Estampille,
}

impl<T: Clone> Versionne<T> {
    /// La version qui gagne entre deux versions du même élément : la plus récente (même résultat dans les deux sens).
    pub fn fusion(&self, autre: &Self) -> Self {
        if autre.estampille > self.estampille {
            autre.clone()
        } else {
            self.clone()
        }
    }
}

/// 🔄 Un paquet de synchro entre appareils : les versions d'éléments plus récentes qu'un moment, effacements compris.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paquet {
    pub sources: Vec<(String, Versionne<Source>)>,
    pub symboles: Vec<(String, Versionne<Symbole>)>,
    pub watchlists: Vec<(String, Versionne<Watchlist>)>,
}

impl Paquet {
    pub fn est_vide(&self) -> bool {
        self.sources.is_empty() && self.symboles.is_empty() && self.watchlists.is_empty()
    }
}

// — sources de données —

/// Sorte de marché d'une source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeSource {
    Crypto,
    Forex,
    TradFi,
    Macro,
    Economie,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Origine {
    /// Fournie avec l'app (fiche vérifiée).
    Integree,
    /// Ajoutée par l'utilisateur.
    Ajoutee,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EtatSource {
    Active,
    Desactivee,
    EnPanne,
}

/// Une source de données de marché (lecture seule : jamais d'ordre, jamais de clé ici ; une clé reste au trousseau).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    /// Nom court en majuscules, comme dans les identifiants de symboles (« BINANCE »).
    pub id: String,
    pub nom: String,
    pub sorte: TypeSource,
    pub origine: Origine,
    /// Adresses HTTPS et WSS (vérifiées par la liste blanche du réseau avant tout appel).
    pub adresses: Vec<String>,
    pub etat: EtatSource,
}

// — symboles —

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TypeMarche {
    Spot,
    Perp,
}

/// Identifiant unique d'un symbole, comme TradingView : `SOURCE:PAIRE` (spot) ou `SOURCE:PAIRE.P` (perp).
/// La paire peut contenir « : » (marchés HIP-3 d'Hyperliquid : `HYPERLIQUID:xyz:NVDA.P`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymboleId {
    pub source: String,
    pub paire: String,
    pub marche: TypeMarche,
}

impl SymboleId {
    /// Lit `SOURCE:PAIRE` ou `SOURCE:PAIRE.P` ; `None` si une partie manque.
    pub fn lire(s: &str) -> Option<Self> {
        let (source, reste) = s.trim().split_once(':')?;
        let (paire, marche) = match reste.strip_suffix(".P") {
            Some(p) => (p, TypeMarche::Perp),
            None => (reste, TypeMarche::Spot),
        };
        let ok = |x: &str| !x.is_empty() && !x.chars().any(char::is_whitespace);
        (ok(source) && ok(paire)).then(|| Self {
            source: source.to_uppercase(),
            paire: paire.to_string(),
            marche,
        })
    }
}

impl fmt::Display for SymboleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let suffixe = if self.marche == TypeMarche::Perp { ".P" } else { "" };
        write!(f, "{}:{}{suffixe}", self.source, self.paire)
    }
}

/// Un symbole du catalogue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Symbole {
    pub id: SymboleId,
    /// Nom lisible (« Bitcoin / USDC perpétuel »).
    pub nom: String,
    /// Clé du logo (symbole de l'actif) ; vide : pastille de couleur.
    pub logo: String,
}

// — watchlists —

/// Drapeau de couleur d'un symbole dans une liste (comme TradingView).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Drapeau {
    Rouge,
    Orange,
    Jaune,
    Vert,
    Bleu,
    Violet,
}

/// Un symbole dans une liste.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entree {
    pub symbole: SymboleId,
    pub drapeau: Option<Drapeau>,
    pub note: String,
    /// Identifiant d'avant la reprise des anciennes listes (« hl:BTC », « oanda:XAU_USD ») : rien ne se perd.
    #[serde(default)]
    pub ancien: Option<String>,
    /// Symbole que la reprise n'a pas reconnu : gardé tel quel, à vérifier (montré à l'écran).
    #[serde(default)]
    pub a_verifier: bool,
}

/// Une section (la première, sans nom, est le haut de la liste).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Section {
    pub nom: String,
    pub repliee: bool,
    pub entrees: Vec<Entree>,
}

/// Une watchlist : privée par défaut ; publiée seulement si l'utilisateur le demande.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Watchlist {
    /// Identifiant stable (ne change pas quand on la renomme).
    pub id: String,
    pub nom: String,
    pub sections: Vec<Section>,
    pub favori: bool,
    pub publiee: bool,
    /// Dernière ouverture (ms), pour « Récemment utilisé ».
    pub ouverte_ms: Option<i64>,
}

impl Watchlist {
    /// Nouvelle liste vide, privée, avec sa section du haut.
    pub fn nouvelle(id: String, nom: String) -> Self {
        Self {
            id,
            nom,
            sections: vec![Section {
                nom: String::new(),
                repliee: false,
                entrees: vec![],
            }],
            favori: false,
            publiee: false,
            ouverte_ms: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(nom: &str) -> Horloge {
        Horloge::new(Appareil(nom.into()))
    }

    #[test]
    fn identifiants_comme_tradingview() {
        for s in [
            "BINANCE:BTCUSDT.P",
            "BINANCE:BTCUSDT",
            "HYPERLIQUID:BTCUSDC.P",
            "BITSTAMP:BTCUSD",
            "HYPERLIQUID:xyz:NVDA.P",
        ] {
            let id = SymboleId::lire(s).unwrap();
            assert_eq!(id.to_string(), s);
        }
        let id = SymboleId::lire("hyperliquid:xyz:NVDA.P").unwrap();
        assert_eq!(
            (id.source.as_str(), id.paire.as_str(), id.marche),
            ("HYPERLIQUID", "xyz:NVDA", TypeMarche::Perp)
        );
        for faux in ["", "BTCUSDT", ":BTC", "BINANCE:", "BINANCE:.P", "BIN ANCE:BTC"] {
            assert_eq!(SymboleId::lire(faux), None, "{faux:?}");
        }
    }

    #[test]
    fn horloge_ne_recule_jamais() {
        let mut a = h("a");
        let e1 = a.estampille(1_000);
        let e2 = a.estampille(900); // l'heure de l'appareil recule
        assert!(e2 > e1 && e2.ms == 1_001);
        let e3 = a.estampille(1_000); // même milliseconde
        assert!(e3 > e2);
    }

    /// Deux appareils, l'un à l'heure en retard de 10 minutes : sa modification faite après celle de l'autre gagne
    /// quand même, parce qu'il a vu l'estampille de l'autre.
    #[test]
    fn appareil_en_retard_ne_perd_rien() {
        let (mut pc, mut cell) = (h("pc"), h("cell"));
        let liste = |nom: &str, e: Estampille| Versionne {
            valeur: Some(Watchlist::nouvelle("l1".into(), nom.into())),
            estampille: e,
        };
        let v_pc = liste("Or", pc.estampille(1_000_000));
        cell.voir(&v_pc.estampille); // le cell reçoit la version du PC
        let v_cell = liste("Or et argent", cell.estampille(1_000_000 - 600_000)); // heure du cell : 10 min de retard
        assert!(v_cell.estampille > v_pc.estampille);
        // même résultat dans les deux sens, et appliquer deux fois ne change rien
        assert_eq!(v_pc.fusion(&v_cell), v_cell);
        assert_eq!(v_cell.fusion(&v_pc), v_cell);
        assert_eq!(v_cell.fusion(&v_cell), v_cell);
        assert_eq!(v_pc.fusion(&v_cell).valeur.unwrap().nom, "Or et argent");
    }

    #[test]
    fn effacement_et_egalite() {
        let (mut a, b) = (h("a"), Appareil("b".into()));
        let mise = Versionne {
            valeur: Some(1),
            estampille: a.estampille(10),
        };
        let effacee = Versionne::<i32> {
            valeur: None,
            estampille: a.estampille(10),
        };
        assert_eq!(mise.fusion(&effacee).valeur, None, "l'effacement plus récent gagne");
        assert_eq!(effacee.fusion(&mise).valeur, None);
        // même moment : l'appareil départage, toujours de la même façon
        let x = Versionne {
            valeur: Some(1),
            estampille: Estampille {
                ms: 5,
                appareil: Appareil("a".into()),
            },
        };
        let y = Versionne {
            valeur: Some(2),
            estampille: Estampille { ms: 5, appareil: b },
        };
        assert_eq!(x.fusion(&y).valeur, Some(2));
        assert_eq!(y.fusion(&x).valeur, Some(2));
    }

    #[test]
    fn privee_par_defaut_et_json() {
        let mut l = Watchlist::nouvelle("l1".into(), "CCQc🚀".into());
        assert!(!l.publiee && !l.favori && l.sections.len() == 1);
        l.sections[0].entrees.push(Entree {
            symbole: SymboleId::lire("BINANCE:ZECUSDT.P").unwrap(),
            drapeau: Some(Drapeau::Vert),
            note: "breakout".into(),
            ancien: None,
            a_verifier: false,
        });
        let s = Source {
            id: "BINANCE".into(),
            nom: "Binance".into(),
            sorte: TypeSource::Crypto,
            origine: Origine::Integree,
            adresses: vec!["https://fapi.binance.com/fapi/v1".into()],
            etat: EtatSource::Active,
        };
        let json = serde_json::to_string(&(&l, &s)).unwrap();
        let (l2, s2): (Watchlist, Source) = serde_json::from_str(&json).unwrap();
        assert_eq!((l2, s2), (l, s));
    }
}
