//! 🦎 DEX on-chain (sources gratuites et sans clé) : recherche de paires dans le panneau ＋ (DexScreener), chart de
//! la pool (bougies GeckoTerminal, relues à la minute) et carte du dock (liquidité, volume, acheteurs / vendeurs,
//! derniers trades). Aucune clé : rien ne passe par le trousseau.

use crate::*;
use argos_data::dex;

/// Préfixe d'une ligne « chercher sur les DEX » du panneau ＋ (Entrée ou clic : la recherche part).
pub(crate) const ASK: &str = "dexq:";

/// Titre de la chart d'une paire DEX.
pub(crate) fn dex_title(name: &str, network: &str, label: &str) -> String {
    format!("{name} · {label} · DEX {network}")
}

/// Lignes du panneau ＋ pour des paires trouvées.
pub(crate) fn found_of(pairs: &[dex::Pair], have: &std::collections::HashSet<String>) -> Vec<argos_render::wlmenu::Found> {
    let money = |v: Option<f64>| v.map_or("—".to_string(), argos_render::whales::money);
    pairs
        .iter()
        .take(40)
        .map(|p| {
            let name = format!("{}/{}", p.base, p.quote);
            let id = dex::id(&p.network(), &p.pool, &name);
            argos_render::wlmenu::Found {
                have: have.contains(&id),
                detail: format!("{} · {} · liq. {} · vol. 24 h {}", p.chain, p.dex, money(p.liq_usd), money(p.vol_24h)),
                group: "DEX (DexScreener)".into(),
                name,
                id,
            }
        })
        .collect()
}

impl App {
    /// La ligne « 🔎 chercher sur les DEX » quand le texte tapé le demande (`dex:pepe`, adresse de contrat).
    pub(crate) fn dex_hint(&self, query: &str) -> Option<Vec<argos_render::wlmenu::Found>> {
        let q = dex::query_of(query)?;
        let mut rows = vec![argos_render::wlmenu::Found {
            id: format!("{ASK}{q}"),
            name: format!("🔎 Chercher « {q} » sur les DEX"),
            detail: "Entrée ou clic · DexScreener, gratuit".into(),
            group: "DEX (DexScreener)".into(),
            have: false,
        }];
        // résultats déjà reçus pour ce texte
        if let Some((done, pairs)) = &self.feeds.dex_found {
            if *done == q {
                let have = self.list_ids();
                rows.extend(found_of(pairs, &have));
            }
        }
        Some(rows)
    }

    /// Lance la recherche DexScreener (résultat : `UserEvent::DexFound`).
    pub(crate) fn dex_search(&mut self, q: String) {
        let Some(proxy) = self.proxy.clone() else { return };
        self.banner = Some(format!("🦎 Recherche de « {q} » sur les DEX…"));
        argos_data::rt::spawn(move || async move {
            let res = dex::search(&argos_data::rt::Client::default(), &q).await.map_err(|e| e.to_string());
            let _ = proxy.send_event(UserEvent::DexFound(q, res));
        });
        self.redraw();
    }

    /// Résultat de la recherche : gardé, et montré si le panneau ＋ cherche encore ce texte.
    pub(crate) fn dex_found(&mut self, q: String, res: Result<Vec<dex::Pair>, String>) {
        match res {
            Ok(pairs) => {
                self.banner = (pairs.is_empty()).then(|| format!("🦎 Aucune paire DEX pour « {q} »"));
                self.feeds.dex_found = Some((q, pairs));
                self.search_add();
            }
            Err(e) => self.banner = Some(format!("🦎 DexScreener : {e}")),
        }
        self.redraw();
    }

    /// Symboles de la watchlist affichée.
    pub(crate) fn list_ids(&self) -> std::collections::HashSet<String> {
        match (self.watch.prefs.list, self.store.as_ref()) {
            (Some(id), Some(st)) => st.items(id).unwrap_or_default().into_iter().map(|(_, i)| i).collect(),
            _ => Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lignes_du_panneau_et_titre() {
        let p = dex::Pair {
            chain: "ethereum".into(),
            dex: "uniswap".into(),
            pool: "0xa43f".into(),
            base: "PEPE".into(),
            quote: "WETH".into(),
            liq_usd: Some(33_000_000.0),
            vol_24h: Some(4_200_000.0),
            ..Default::default()
        };
        let f = found_of(&[p], &Default::default());
        assert_eq!((f[0].id.as_str(), f[0].name.as_str()), ("dex:eth:0xa43f:PEPE/WETH", "PEPE/WETH"));
        assert!(f[0].detail.starts_with("ethereum · uniswap · liq."), "{}", f[0].detail);
        assert_eq!(dex_title("PEPE/WETH", "eth", "1h"), "PEPE/WETH · 1h · DEX eth");
    }
}
