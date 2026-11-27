//! ⏪ Replay des bougies.

use crate::*;

impl App {
    /// ⏪ Démarre le replay : départ au milieu de ce qui est affiché, puis on choisit la bougie (✂️).
    pub(crate) fn replay_start(&mut self) {
        if self.chart.bars.len() < 3 {
            self.banner = Some("⏳ Attends que les bougies soient chargées".into());
            return self.redraw();
        }
        let n = self.chart.bars.len();
        let mid = self
            .chart
            .viewport
            .visible(&self.chart.slots)
            .map_or(n.saturating_sub(100), |(a, b)| (a + b) / 2)
            .clamp(1, n - 1);
        self.sim_replay_end();
        self.replay = Some(argos_render::replay::Replay::new(self.chart.bars[mid].time));
        (self.ui.sheet, self.ui.argos, self.ui.list) = (None, None, None);
        self.banner = Some("⏪ Replay : clique la bougie de départ, puis ▶".into());
        self.rebuild();
        self.redraw();
    }

    /// ⏪ Where the replay bar goes (chart coordinates): the width it is centred in and the price zone's height. The
    /// main chart's price zone; a split whose main pane is too narrow for the bar (📱 3 columns): the whole chart area,
    /// the bar over the panes (Suivi 7: it was cut off in the left column), above the other panes' navigation pills.
    pub(crate) fn replay_zone(&self) -> (f64, f64) {
        let vp = &self.chart.viewport;
        let (pw, ph) = (vp.plot_w(), vp.plot_h() - self.ind.out.osc_h());
        let panes = argos_render::splits::cells(&self.split.layout).len() > 1;
        if panes && pw < argos_render::replay::ETROITE + 8.0 {
            let pilules = argos_render::navbar::rect(vp, vp.plot_h(), false).y;
            (self.area.0 as f64, ph.min(pilules - 6.0 + argos_render::replay::AU_DESSUS))
        } else {
            (pw, ph)
        }
    }

    /// ⏪ Un bouton de la barre de replay.
    pub(crate) fn replay_act(&mut self, c: argos_render::replay::ReplayClick) {
        use argos_render::replay::ReplayClick as R;
        let Some(rp) = self.replay.as_mut() else { return };
        match c {
            R::Exit => {
                self.sim_replay_end();
                self.replay = None;
                self.banner = Some("📡 Retour au direct".into());
            }
            R::Pick => (rp.pick, rp.play) = (true, false),
            R::Prev | R::Next => {
                rp.play = false;
                return self.replay_step(if c == R::Next { 1 } else { -1 });
            }
            R::Play => {
                (rp.play, rp.pick) = (!rp.play, false);
                self.replay_at = Instant::now();
            }
            R::Speed => rp.next_speed(),
        }
        self.rebuild();
        self.redraw();
    }

    /// ⏪ Avance (ou recule) de `d` bougies ; au bout, la lecture s'arrête.
    pub(crate) fn replay_step(&mut self, d: isize) {
        let Some(rp) = self.replay.as_mut() else { return };
        rp.pick = false;
        if let Some(t) = rp.step(&self.replay_times, d) {
            rp.t = t;
        }
        if rp.play && self.replay_times.last().is_some_and(|&l| rp.t >= l) {
            rp.play = false;
            self.banner = Some("🏁 Fin du replay : ✕ pour revenir au direct".into());
        }
        self.replay_at = Instant::now();
        self.rebuild();
        self.redraw();
    }

    /// ⏪ The replay's hour: the end of the last bar shown (`None`: no replay, or its first bar is being chosen). The
    /// other panes of the layout show what is known at that hour (`pane_bars`).
    pub(crate) fn replay_heure(&self) -> Option<i64> {
        let rp = self.replay.as_ref().filter(|r| !r.pick)?;
        Some(argos_core::bucket_end(rp.t, self.chart.interval.secs()))
    }

    /// ⏪ Before the chart goes to `iv` during a replay, the hour to keep. A bigger unit whose bar is in progress first
    /// plays the finer bars up to its end (open orders play them, nothing is skipped and nothing seen ahead); a
    /// smaller unit keeps the hour as it is. `None`: no replay running.
    pub(crate) fn replay_avant_ut(&mut self, iv: Interval) -> Option<i64> {
        let ancien = self.chart.interval.secs();
        let heure = self.replay_heure()?;
        let fin = argos_core::fin_en_cours(heure, iv.secs());
        let t = self.replay.as_ref()?.t;
        let jusqua = self
            .replay_times
            .iter()
            .copied()
            .take_while(|&x| argos_core::bucket_end(x, ancien) <= fin)
            .last()
            .filter(|&x| x > t);
        let Some(t2) = jusqua else { return Some(heure) };
        if let Some(rp) = self.replay.as_mut() {
            rp.t = t2;
        }
        self.rebuild(); // 🧪 the Simulator plays these bars
        Some(argos_core::bucket_end(t2, ancien))
    }

    /// ⏪ After the chart went to its new unit during a replay: the replay stands on the last bar finished at `heure`
    /// (the bars before it count as played: the Simulator does not play them twice), and the past loads back to it.
    pub(crate) fn replay_apres_ut(&mut self, heure: i64, avance: bool) {
        let iv = self.chart.interval;
        let t = argos_core::derniere_finie(heure, iv.secs());
        let Some(rp) = self.replay.as_mut() else { return };
        rp.t = t;
        self.ui.sim_last_t = self.ui.sim_last_t.max(t);
        let quand = crate::actions::simulateur::date_courte(heure, self.tz_offset_s());
        let message = if avance {
            format!("⏩ Replay en {} : il avance à la fin de la bougie en cours ({quand})", iv.short())
        } else {
            format!("⏪ Replay en {}, à la même heure ({quand})", iv.short())
        };
        self.banner = Some(message.clone());
        self.ui.replay_ut = Some((t, message));
        self.rebuild();
        self.replay_ut_poursuivre();
    }

    /// ⏪ After a change of unit during a replay: pages of the past until the bars reach the replay's bar (or the
    /// start of the history, or the memory limit). Called when bars and pages of the past arrive.
    pub(crate) fn replay_ut_poursuivre(&mut self) {
        let Some(t) = self.ui.replay_ut.as_ref().map(|u| u.0) else { return };
        if self.replay.is_none() {
            self.ui.replay_ut = None;
            return;
        }
        if self.chart.loading.is_some() || self.chart.raw.is_empty() {
            return; // the bars are on their way
        }
        self.older_key_sync();
        if self.chart.older_busy {
            return; // a page of the past is on its way
        }
        use crate::actions::load::{MAX_RAW, OLDER_PAGE};
        let atteint = self.chart.raw.first().is_some_and(|b| b.time <= t) || self.chart.history_start || self.chart.raw.len() >= MAX_RAW;
        if !atteint && self.request_older(OLDER_PAGE) {
            self.banner = Some(format!(
                "⏳ Chargement des bougies {} jusqu'au {}",
                self.chart.interval.short(),
                crate::actions::simulateur::date_courte(t, self.tz_offset_s())
            ));
            return;
        }
        let message = self.ui.replay_ut.take().map(|u| u.1);
        // the message of the change, said again once the bars are there (their arrival writes its own banner)
        self.banner = match self.chart.raw.first().map(|b| b.time).filter(|&d| d > t) {
            Some(debut) => Some(format!(
                "📜 L'historique {} commence le {} : prends une unité plus grande pour cette date",
                self.chart.interval.short(),
                crate::actions::simulateur::date_courte(debut, self.tz_offset_s())
            )),
            None => message,
        };
        self.rebuild();
        // 📏 the price scale fitted to the bars of the new unit at the replay's hour (not the old unit's bounds)
        self.start_free();
    }
}
