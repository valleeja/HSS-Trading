//! 💥 Carte des liquidations : téléchargement (Binance, OKX, Hyperliquid) toutes les 5 min tant que l'indicateur est
//! allumé, relevés de l'intérêt ouvert de Hyperliquid (pas d'historique public : il se construit ici), et calcul de
//! la carte estimée (seulement quand les données ou les réglages changent).

use crate::*;
use argos_core::liqmap::{self, OiBar, Params};
use argos_data::liqdata::{self, Venue};
use argos_render::indicators::Ind;
use argos_render::liqview::{self, LiqView};

/// Relevés gardés par pièce (un mois en 5 min).
const HL_KEEP: usize = 9_000;

impl App {
    /// 💥 L'indicateur est-il allumé ?
    pub(crate) fn liq_on(&self) -> bool {
        self.ind.on.contains(&Ind::LiqMap)
    }

    fn liq_vals(&self) -> Vec<f64> {
        argos_render::indicators::values(&self.ind.set, Ind::LiqMap)
    }

    fn liq_inputs<'a>(vals: &'a [f64]) -> argos_script::inputs::Inputs<'a> {
        argos_script::inputs::Inputs {
            params: &liqview::PARAMS,
            vals,
        }
    }

    /// Demande les données si l'indicateur est allumé et qu'elles manquent ou ont plus de 5 min.
    pub(crate) fn liq_refresh(&mut self) {
        if !self.liq_on() {
            if self.feeds.liq_view.is_some() {
                (self.feeds.liq_view, self.feeds.liq_data, self.feeds.liq_key) = (None, vec![], String::new());
            }
            return;
        }
        let Some(coin) = self.book_coin() else {
            self.feeds.liq_view = Some(LiqView {
                notes: vec!["💥 Carte des liquidations : pas de données pour ce symbole (crypto seulement)".into()],
                ..Default::default()
            });
            return;
        };
        self.liq_compute();
        let vals = self.liq_vals();
        let p = Self::liq_inputs(&vals);
        let hours = liqview::HOURS[p.pick("period").min(liqview::HOURS.len() - 1)];
        let venue = p.pick("venue");
        let key = format!("{coin}|{hours}|{venue}");
        if key == self.feeds.liq_key && self.feeds.liq_at.is_some_and(|t| t.elapsed().as_secs() < 300) {
            return;
        }
        let Some(proxy) = self.proxy.clone() else { return };
        (self.feeds.liq_key, self.feeds.liq_at) = (key.clone(), Some(Instant::now()));
        let venues: Vec<Venue> = match venue {
            0 => vec![Venue::Binance],
            1 => vec![Venue::Okx],
            2 => vec![],
            _ => vec![Venue::Binance, Venue::Okx],
        };
        let hl = venue >= 2;
        let now = now_s();
        argos_data::rt::spawn(move || async move {
            let http = argos_data::rt::Client::default();
            let mut out = vec![];
            for v in venues {
                let r = match v {
                    Venue::Binance => liqdata::binance(&http, &format!("{coin}USDT"), hours, now).await,
                    Venue::Okx => liqdata::okx(&http, &format!("{coin}-USDT-SWAP"), hours, now).await,
                    Venue::Hyperliquid => continue,
                };
                out.push((v, r.map_err(String::from)));
            }
            let hl = if hl {
                liqdata::hyperliquid(&http, &coin, hours, now).await.ok()
            } else {
                None
            };
            let _ = proxy.send_event(UserEvent::LiqMap(key, out, hl));
        });
    }

    /// Données reçues : gardées, l'intérêt ouvert de Hyperliquid noté, la carte recalculée.
    pub(crate) fn liq_loaded(&mut self, key: String, venues: Vec<(Venue, Result<Vec<OiBar>, String>)>, hl: Option<LiqHl>) {
        if key != self.feeds.liq_key {
            return; // réponse d'un autre symbole ou d'une autre période
        }
        if let (Some((_, Some(oi))), Some(coin)) = (hl.as_ref(), key.split('|').next()) {
            if self.feeds.liq_hl_oi.is_empty() {
                self.feeds.liq_hl_oi = Self::read_prefs("liq_hl.json");
            }
            let v = self.feeds.liq_hl_oi.entry(coin.to_string()).or_default();
            v.push((now_s(), *oi));
            if v.len() > HL_KEEP {
                v.drain(..v.len() - HL_KEEP);
            }
            Self::write_prefs("liq_hl.json", &self.feeds.liq_hl_oi);
        }
        (self.feeds.liq_data, self.feeds.liq_hl, self.feeds.liq_sig) = (venues, hl, vec![]);
        self.liq_compute();
        self.redraw();
    }

    /// Recalcule la carte si les données ou les réglages ont changé.
    fn liq_compute(&mut self) {
        let vals = self.liq_vals();
        let price = self.chart.bars.last().map_or(0.0, |b| b.close);
        let mut sig = vals.clone();
        sig.push(self.feeds.liq_data.len() as f64);
        if sig == self.feeds.liq_sig || price <= 0.0 || (self.feeds.liq_data.is_empty() && self.feeds.liq_hl.is_none()) {
            return;
        }
        self.feeds.liq_sig = sig;
        let p = Self::liq_inputs(&vals);
        let frac = [0.001, 0.0005, 0.002, 0.005][p.pick("bucket").min(3)];
        let params = Params {
            levers: vec![(10.0, p.get("w10")), (25.0, p.get("w25")), (50.0, p.get("w50")), (100.0, p.get("w100"))],
            mm: 0.005,
            bucket: argos_render::bookheat::nice(price * frac),
            long_share: (p.pick("split") == 0).then_some(0.5),
        };
        let (mut bands, mut notes) = (vec![], vec![]);
        for (v, r) in &self.feeds.liq_data {
            match r {
                Ok(bars) if bars.len() > 1 => bands.extend(liqmap::compute(bars, &params)),
                Ok(_) => notes.push(format!("{} : pas assez de données", v.name())),
                Err(e) => notes.push(format!("{} indisponible ({e})", v.name())),
            }
        }
        if let Some((candles, _)) = &self.feeds.liq_hl {
            let coin = self.feeds.liq_key.split('|').next().unwrap_or("");
            let snaps = self.feeds.liq_hl_oi.get(coin).cloned().unwrap_or_default();
            let step = candles.windows(2).next().map_or(300, |w| w[1].0 - w[0].0);
            let bars = liqdata::join(candles, &snaps, step);
            let since = snaps
                .first()
                .and_then(|s| chrono::DateTime::from_timestamp(s.0 + self.tz_offset_s(), 0))
                .map(|d| d.format("%d/%m %H:%M").to_string());
            notes.push(format!(
                "Hyperliquid : historique en construction{}",
                since.map(|s| format!(" (depuis le {s})")).unwrap_or_default()
            ));
            bands.extend(liqmap::compute(&bars, &params));
        }
        let profile = liqmap::profile(&bands);
        self.feeds.liq_view = Some(LiqView {
            bands,
            profile,
            bucket: params.bucket,
            notes,
        });
    }
}
