//! 📦 Import des données de la v1.

use crate::*;

impl App {
    /// 📥 Reprend les données d'Argos v1 (navigateur : le stockage du même site ; desktop : `argos-v1.json` des
    /// Téléchargements) sans rien écraser : ajoute les listes, dessins, scripts et trades qui manquent ; le compte démo
    /// seulement s'il est encore vide.
    pub(crate) fn import_v1(&mut self) {
        #[cfg(target_arch = "wasm32")]
        let data = Some(v1import::bundle(platform::web::v1_pairs())).filter(|d| !d.is_empty());
        #[cfg(not(target_arch = "wasm32"))]
        let data = platform::read_download(v1import::FILE).and_then(|raw| v1import::from_export(&raw));
        let Some(data) = data else {
            self.banner = Some(format!(
                "📥 Rien à reprendre : dans Argos v1, Mises en page ▾ ▸ 📦 Tout exporter pour Argos Rust, puis mets {} dans tes Téléchargements",
                v1import::FILE
            ));
            return self.redraw();
        };
        let plan = v1import::plan(&data);
        let mut done: Vec<String> = vec![];
        // watchlists (celles qui ont déjà ce nom sont gardées telles quelles)
        if let Some(st) = self.store.as_mut() {
            let have: Vec<String> = st.lists().unwrap_or_default().into_iter().map(|l| l.name).collect();
            let mut n = 0;
            for (name, items) in plan.lists.iter().filter(|(n, _)| !have.contains(n)) {
                let Ok(id) = st.create_list(name) else { continue };
                let mut secs: Vec<&str> = vec![];
                for (sec, coin) in items {
                    if !sec.is_empty() && !secs.contains(&sec.as_str()) {
                        let _ = st.add_section(id, sec);
                        secs.push(sec);
                    }
                    let _ = st.add_item(id, sec, coin);
                }
                n += 1;
            }
            if n > 0 {
                done.push(format!("{n} liste(s)"));
            }
        }
        self.load_lists();
        // dessins, par symbole (ceux déjà là ne sont pas doublés)
        let mut nd = 0;
        for (file, list) in &plan.drawings {
            let mut s = saved::read(file).unwrap_or_default();
            for d in list {
                if !s.drawings.contains(d) {
                    s.drawings.push(d.clone());
                    nd += 1;
                    if self.chart.save_path.as_deref() == Some(file.as_str()) && !self.chart.drawings.contains(d) {
                        self.chart.drawings.push(d.clone());
                    }
                }
            }
            let _ = saved::write(file, &s);
        }
        if nd > 0 {
            done.push(format!("{nd} dessin(s)"));
        }
        // scripts
        let before = self.scripts.len();
        for (name, src, on) in &plan.scripts {
            if !self.scripts.iter().any(|s| &s.name == name && &s.src == src) {
                self.scripts.push(SavedScript {
                    name: name.clone(),
                    src: src.clone(),
                    on: *on,
                    ..Default::default()
                });
            }
        }
        if self.scripts.len() > before {
            Self::write_prefs("scripts.json", &self.scripts);
            self.ind.key = Default::default();
            done.push(format!("{} script(s)", self.scripts.len() - before));
        }
        // journal (par identifiant)
        let j = &mut self.watch.panels.journal;
        let before = j.trades.len();
        for t in plan.journal {
            if !j.trades.iter().any(|x| x.id == t.id) {
                j.trades.push(t);
            }
        }
        if j.trades.len() > before {
            done.push(format!("{} trade(s) au journal", j.trades.len() - before));
            self.journal_ecrire();
        }
        // compte démo : seulement si celui d'Argos Rust est encore vide
        let mut skipped = plan.skipped;
        if let Some(d) = plan.demo {
            if self.demo.history.is_empty() && self.demo.positions.is_empty() {
                done.push(format!("compte démo « {} »", d.name));
                self.demo = d;
                Self::write_prefs("demo_account.json", &self.demo);
            } else {
                skipped.push("compte démo : celui d'Argos Rust a déjà des trades, il est gardé".into());
            }
        }
        self.banner = Some(match (done.is_empty(), skipped.is_empty()) {
            (true, _) => "📥 Argos v1 : tout était déjà repris".to_string(),
            (false, true) => format!("📥 Repris d'Argos v1 : {}", done.join(", ")),
            (false, false) => format!("📥 Repris d'Argos v1 : {} · laissé de côté : {}", done.join(", "), skipped.len()),
        });
        for s in &skipped {
            eprintln!("📥 laissé de côté : {s}");
        }
        self.redraw();
    }
}
