//! 💼 Portfolio, comme Argos v1 (façon Hyperliquid) : page plein écran avec les actions guidées, volume et frais sur
//! 14 jours, résumé de la période (24 h · 7 jours · 30 jours · Tout), courbe de la valeur du compte ou du P&L, puis
//! les onglets Soldes · Positions · Ordres ouverts · Historique des trades. Compte démo ; le wallet en lecture seule
//! arrive avec Connect. Argos ne signe rien : une action guidée se fait chez Hyperliquid, avec le wallet de Jee.

use crate::palette as pal;
use crate::palette::{GREY, WHITE};
use crate::shapes::{lines, round_fill, round_stroke, Rect};
use crate::text::{Align, Rgb, Text};
use argos_core::portfolio::Portfolio;
use argos_core::Side;
use tiny_skia::Pixmap;

/// Turquoise du Portfolio de v1 (couleur de Hyperliquid) ; aussi celui de la page du rapport de backtest.
pub(crate) const TEAL: Rgb = Rgb(0x50, 0xd2, 0xc1);

/// Périodes du résumé : (nom, durée en secondes ; 0 = tout).
pub const RANGES: [(&str, i64); 4] = [("24 h", 86_400), ("7 jours", 7 * 86_400), ("30 jours", 30 * 86_400), ("Tout", 0)];
const TABS: [&str; 4] = ["Soldes", "Positions", "Ordres ouverts", "Historique des trades"];
/// Montants offerts pour déposer / retirer sur le compte démo.
const AMOUNTS: [f64; 4] = [1_000.0, 10_000.0, 50_000.0, 100_000.0];
/// Lien du Portfolio de Hyperliquid (où l'action se signe).
pub const HL_APP: &str = "https://app.hyperliquid.xyz/portfolio";

/// Actions guidées de v1 : (bouton, titre, description, étapes, mise en garde).
pub const GUIDES: [(&str, &str, &str, [&str; 4], &str); 7] = [
    ("🔗 Lier le staking", "🔗 Lier les comptes de trading et de staking", "Applique les rabais de frais du HYPE staké à ton compte de trading.",
        ["Sur Hyperliquid, va dans Portfolio ▸ Link Staking.", "Entre d'abord l'adresse du compte de staking (celui qui a le HYPE staké).", "Connecte-toi avec le compte de staking, puis entre l'adresse du compte de trading.", "Confirme avec ton wallet."],
        "Ce lien est permanent : le compte de staking aura le contrôle des fonds du compte de trading. Ne lie jamais un compte que tu ne contrôles pas toi-même."),
    ("🔄 Échanger stablecoins", "🔄 Échanger des stablecoins", "Convertir USDC ↔ USDT sur le carnet d'ordres spot de Hyperliquid.",
        ["Portfolio ▸ Swap Stablecoins.", "Choisis ce que tu vends (ex. USDC) et ce que tu achètes (ex. USDT).", "Entre le montant (MAX = tout ton solde spot).", "Vérifie « Tu reçois environ » et les frais, puis confirme."],
        "L'exécution n'est pas garantie : si le glissement dépasse 1 %, l'échange est rempli seulement en partie."),
    ("⇄ EVM ↔ Core", "⇄ Transfert EVM ↔ Core", "Déplacer des actifs entre ton solde HyperEVM et ton solde spot HyperCore.",
        ["Portfolio ▸ EVM ⇄ Core.", "Choisis le sens : Spot → EVM ou EVM → Spot.", "Choisis l'actif (ex. USDC) et le montant.", "Confirme avec ton wallet."],
        "Recevoir des USDC sur l'EVM coûte un petit frais en HYPE (~0,00002 HYPE) : garde un peu de HYPE."),
    ("🧾 Type de compte", "🧾 Type de compte", "Comment ta marge est partagée entre spot et perps.",
        ["Compte unifié (recommandé) : chaque actif de garantie a son solde ; les perps utilisent seulement l'actif de règlement.", "Marge de portefeuille (avancé) : HYPE, BTC… servent directement de garantie pour les perps ; la garantie gagne ou paie de l'intérêt.", "Manuel : pour les bots ; chaque DEX a son solde séparé.", "Choisis sur Hyperliquid ▸ Portfolio ▸ Account Type, puis confirme."],
        "La marge de portefeuille est en bêta (compte de moins de 25 M$) et augmente le risque de liquidation si la garantie baisse."),
    ("📤 Envoyer", "📤 Envoyer des tokens", "Envoyer à un autre compte sur la blockchain Hyperliquid (HyperCore), instantané et sans frais de gaz.",
        ["Portfolio ▸ Send.", "Colle l'adresse de destination (0x…) et vérifie les 4 premiers et 4 derniers caractères.", "Choisis le compte source (trading ou spot) et le token.", "Entre le montant et confirme."],
        "Un envoi est définitif. N'envoie jamais à l'adresse d'un exchange qui ne supporte pas HyperCore : les fonds seraient perdus."),
    ("⬆️ Retirer", "⬆️ Retirer vers Arbitrum", "Sortir des USDC de Hyperliquid vers ton adresse sur Arbitrum.",
        ["Portfolio ▸ Withdraw.", "Actif : USDC · Réseau : Arbitrum.", "Entre le montant (frais : 0,2 USDC + un petit frais en HYPE).", "Confirme : ça arrive en environ 5 minutes."],
        "Les fonds arrivent sur la même adresse que ton wallet, sur Arbitrum. Vérifie que ton wallet supporte Arbitrum avant de retirer."),
    ("⬇️ Déposer", "⬇️ Déposer", "Envoyer des USDC (ou d'autres actifs) de ton wallet vers Hyperliquid.",
        ["Portfolio ▸ Deposit.", "Choisis l'actif et le réseau (USDC sur Arbitrum est le plus simple).", "Entre le montant et approuve dans ton wallet.", "Le solde apparaît en environ 1 minute."],
        "Dépose seulement l'actif et le réseau indiqués : un mauvais réseau peut faire perdre les fonds."),
];

/// Ce qu'un clic demande à la fenêtre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PfClick {
    None,
    /// Redessiner (onglet, période, guide…).
    Changed,
    Close,
    /// Ouvrir ce lien dans le navigateur (action guidée chez Hyperliquid).
    Open(&'static str),
    /// Compte démo : + dépôt / − retrait (montant en $).
    Fund(f64),
    /// Pas encore là : afficher ce message.
    Soon(&'static str),
    /// + Compte démo : le menu des comptes démo.
    Accounts,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Hit {
    Close,
    Guide(usize),
    Deposit,
    Withdraw,
    NewAccount,
    Space,
    /// 📱 Au doigt : « Compte réel › » ouvre / referme les 7 guides du compte réel.
    Reel,
    Range(usize),
    Metric(usize),
    Tab(usize),
    History,
    /// Guide ouvert : bouton Hyperliquid, fermer.
    GuideGo,
    GuideClose,
    /// Montant du dépôt / retrait.
    Amount(usize),
}

/// État de la page.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PfPage {
    /// Période (index dans `RANGES`) ; 2 = 30 jours par défaut, comme v1.
    pub range: usize,
    /// 0 = valeur du compte, 1 = P&L.
    pub metric: usize,
    pub tab: usize,
    guide: Option<usize>,
    /// Choix du montant ouvert : `true` = dépôt, `false` = retrait.
    fund: Option<bool>,
    /// 📱 Défilement (points) : écran étroit, les blocs sont empilés et la page défile.
    pub scroll: f64,
    /// 📱 Au doigt : les guides du compte réel sont montrés (sous « Compte réel › »).
    reel: bool,
}

/// En dessous de cette largeur utile, les trois blocs et le tableau sont empilés (cell, petite fenêtre).
const NARROW: f64 = 900.0;

/// Chiffres de la page, calculés du compte démo.
#[derive(Debug, Clone, PartialEq)]
struct Data {
    series: Vec<(i64, f64)>,
    vol14: f64,
    pnl: f64,
    vol: f64,
    dd: f64,
    equity: f64,
    upl: f64,
    margin: f64,
}

/// Levier du compte démo (marge utilisée = valeur / levier).
const LEV: f64 = 10.0;

fn usd(v: f64) -> String {
    crate::viewport::format_money(v, 2)
}

impl PfPage {
    pub fn new() -> Self {
        Self {
            range: 2,
            ..Default::default()
        }
    }

    /// Statistiques comme v1 (`demoSeries`, `accStats`) ; `price` : prix actuel d'un symbole (sinon le prix d'entrée).
    fn data(&self, a: &Portfolio, now: i64, price: &dyn Fn(&str) -> Option<f64>) -> Data {
        let mut h = a.history.clone();
        h.sort_by_key(|t| t.closed_at);
        // 💸 les trades fermés comptent net (coûts déduits), comme le solde
        let tot = h.iter().map(|t| t.net()).sum::<argos_core::Money>();
        let s0 = (a.balance - tot).to_f64();
        let upl: f64 = a.positions.iter().map(|p| p.pnl(price(&p.base).unwrap_or(p.entry))).sum();
        let equity = a.balance.to_f64() + upl;
        let margin: f64 = a.positions.iter().map(|p| (p.entry * p.qty).abs() / LEV).sum();
        // valeur du compte au fil des trades fermés, puis maintenant (avec le latent)
        let mut pts = vec![(h.first().map_or(now - 86_400, |t| t.opened_at), s0, 0.0)];
        let (mut v, mut pl) = (s0, 0.0);
        for t in &h {
            v += t.net().to_f64();
            pl += t.net().to_f64();
            pts.push((t.closed_at, v, pl));
        }
        pts.push((now, equity, pl + upl));
        let cut = match RANGES[self.range.min(3)].1 {
            0 => i64::MIN,
            d => now - d,
        };
        let inside: Vec<_> = pts.iter().filter(|p| p.0 >= cut).copied().collect();
        let base = inside.first().map_or(0.0, |p| p.2);
        let series = (if inside.is_empty() { pts[pts.len() - 1..].to_vec() } else { inside })
            .iter()
            .map(|p| (p.0, if self.metric == 1 { p.2 - base } else { p.1 }))
            .collect();
        let (mut pk, mut dd) = (f64::NEG_INFINITY, 0.0f64);
        for p in &pts {
            pk = pk.max(p.1);
            if pk > 0.0 {
                dd = dd.max((pk - p.1) / pk);
            }
        }
        let turn = |t: &&argos_core::portfolio::ClosedTrade| (t.entry * t.qty).abs() + (t.exit * t.qty).abs();
        let in_range: Vec<_> = h.iter().filter(|t| t.closed_at >= cut).collect();
        Data {
            series,
            vol14: h.iter().filter(|t| t.closed_at > now - 14 * 86_400).map(|t| turn(&t)).sum(),
            pnl: in_range.iter().map(|t| t.net()).sum::<argos_core::Money>().to_f64() + if self.range == 3 { upl } else { 0.0 },
            vol: in_range.iter().map(turn).sum(),
            dd,
            equity,
            upl,
            margin,
        }
    }

    /// Mise en page (pour le dessin et le clic) dans une fenêtre `w` × `h`.
    fn layout(&self, text: &Text, w: f64, h: f64) -> (Vec<(Rect, Hit)>, Geo) {
        let inner = (w - 40.0).min(1400.0);
        let x0 = (w - inner) / 2.0;
        let mut z = vec![(Rect::new(x0 + inner - 36.0, 16.0, 32.0, 32.0), Hit::Close)];
        // boutons : les 7 guides, puis les actions du compte démo, sur une ou plusieurs lignes
        // (📱 au doigt : d'abord le compte démo, les guides du compte réel rangés sous « Compte réel › », pas de Connect)
        let labels: Vec<(String, Hit)> = self.boutons().into_iter().map(|h| (bouton_nom(h, self.reel).to_string(), h)).collect();
        let (mut bx, mut by) = (x0, 64.0);
        for (l, hit) in &labels {
            let bw = text.width(l, 13.5) as f64 + 28.0;
            if bx + bw > x0 + inner {
                bx = x0;
                by += 42.0;
            }
            z.push((Rect::new(bx, by, bw, 34.0), *hit));
            bx += bw + 8.0;
        }
        let top = by + 52.0;
        // 📱 étroit : les blocs l'un sous l'autre, toute la largeur (la page défile)
        let narrow = inner < NARROW;
        let (col1, col2, col3) = if narrow {
            (
                Rect::new(x0, top, inner, 300.0),
                Rect::new(x0, top + 312.0, inner, 300.0),
                Rect::new(x0, top + 624.0, inner, 300.0),
            )
        } else {
            (
                Rect::new(x0, top, 240.0, 300.0),
                Rect::new(x0 + 252.0, top, 320.0, 300.0),
                Rect::new(x0 + 584.0, top, inner - 584.0, 300.0),
            )
        };
        let top = if narrow { top + 624.0 } else { top };
        for (k, (name, _)) in RANGES.iter().enumerate() {
            let bw = text.width(name, 12.5) as f64 + 18.0;
            let x = col2.x + 14.0 + RANGES[..k].iter().map(|(n, _)| text.width(n, 12.5) as f64 + 24.0).sum::<f64>();
            z.push((Rect::new(x, col2.y + 12.0, bw, 26.0), Hit::Range(k)));
        }
        for (k, name) in ["Valeur du compte", "P&L"].iter().enumerate() {
            let x = col3.x + 16.0 + if k == 0 { 0.0 } else { text.width("Valeur du compte", 13.5) as f64 + 18.0 };
            z.push((Rect::new(x, col3.y + 8.0, text.width(name, 13.5) as f64, 28.0), Hit::Metric(k)));
        }
        z.push((Rect::new(col1.x + 16.0, col1.y + 90.0, 140.0, 20.0), Hit::History));
        let table = if narrow {
            Rect::new(x0, top + 312.0, inner, 460.0)
        } else {
            Rect::new(x0, top + 312.0, inner, (h - top - 312.0 - 20.0).max(160.0))
        };
        let mut tx = table.x + 16.0;
        for (k, name) in TABS.iter().enumerate() {
            let tw = text.width(name, 14.0) as f64;
            z.push((Rect::new(tx, table.y + 6.0, tw, 34.0), Hit::Tab(k)));
            tx += tw + 22.0;
        }
        // fenêtres par-dessus : choix du montant, guide (en tête de la table : prioritaires au clic)
        let mut over = vec![];
        if let Some(dep) = self.fund {
            let b = labels
                .iter()
                .position(|(_, h)| *h == if dep { Hit::Deposit } else { Hit::Withdraw })
                .map(|k| z[k + 1].0)
                .unwrap_or(Rect::new(x0, 64.0, 100.0, 34.0));
            // under the last row of buttons (📱 under the button they went over the second row), inside the page
            // (« Retirer » is on the right: they went out of it), on two rows if one is not wide enough
            let largeurs: Vec<f64> = AMOUNTS.iter().map(|a| text.width(&usd(*a), 13.0) as f64 + 20.0).collect();
            let rangee = |ws: &[f64]| ws.iter().sum::<f64>() + 6.0 * (ws.len().saturating_sub(1)) as f64;
            let rangees: Vec<&[f64]> = if rangee(&largeurs) <= inner {
                vec![&largeurs[..]]
            } else {
                vec![&largeurs[..2], &largeurs[2..]]
            };
            let mut k = 0;
            for (n, ws) in rangees.iter().enumerate() {
                let mut x = b.x.min(x0 + inner - rangee(ws)).max(x0);
                for aw in ws.iter() {
                    over.push((Rect::new(x, by + 40.0 + n as f64 * 36.0, *aw, 30.0), Hit::Amount(k)));
                    x += aw + 6.0;
                    k += 1;
                }
            }
        }
        over.extend(z);
        (over, Geo { col1, col2, col3, table })
    }

    /// Les boutons du haut, dans l'ordre.
    fn boutons(&self) -> Vec<Hit> {
        let guides = (0..GUIDES.len()).map(Hit::Guide);
        let demo = [Hit::Deposit, Hit::Withdraw, Hit::NewAccount];
        if crate::actions::tactile() {
            let reel: Vec<Hit> = if self.reel { guides.collect() } else { vec![] };
            demo.into_iter().chain([Hit::Reel]).chain(reel).collect()
        } else {
            guides.chain(demo).chain([Hit::Space]).collect()
        }
    }

    /// Hauteur de tout le contenu (plus que la fenêtre quand les blocs sont empilés ; 📱 au doigt, de la place en bas
    /// pour que le rond Argos AI ne cache pas la fin).
    fn content_h(&self, text: &Text, w: f64, h: f64) -> f64 {
        let (_, g) = self.layout(text, w, h);
        (g.table.y + g.table.h + if crate::actions::tactile() { 90.0 } else { 20.0 }).max(h)
    }

    /// 📱 Molette / doigt : la page défile (`dy` > 0 : vers le bas).
    pub fn wheel(&mut self, dy: f64, text: &Text, w: f64, h: f64) {
        let max = (self.content_h(text, w, h) - h).max(0.0);
        self.scroll = (self.scroll + dy).clamp(0.0, max);
    }

    /// Boutons du guide ouvert (coordonnées de la fenêtre, pas du contenu).
    fn guide_zones(w: f64, h: f64) -> [(Rect, Hit); 2] {
        let g = guide_rect(w, h);
        [
            (Rect::new(g.x + g.w - 44.0, g.y + 14.0, 30.0, 30.0), Hit::GuideClose),
            (Rect::new(g.x + 24.0, g.y + g.h - 64.0, 300.0, 40.0), Hit::GuideGo),
        ]
    }

    pub fn click(&mut self, text: &Text, w: f64, h: f64, x: f64, y: f64) -> PfClick {
        let hit = if self.guide.is_some() {
            Self::guide_zones(w, h).into_iter().find(|(r, _)| r.contains(x, y)).map(|(_, h)| h)
        } else {
            let ch = self.content_h(text, w, h);
            let (z, _) = self.layout(text, w, ch);
            z.into_iter().find(|(r, _)| r.contains(x, y + self.scroll)).map(|(_, h)| h)
        };
        // un guide ouvert : seul lui répond (à côté : il se ferme)
        if let Some(k) = self.guide {
            return match hit {
                Some(Hit::GuideGo) => PfClick::Open(HL_APP),
                Some(Hit::GuideClose) => {
                    self.guide = None;
                    PfClick::Changed
                }
                _ if !guide_rect(w, h).contains(x, y) => {
                    self.guide = None;
                    PfClick::Changed
                }
                _ => {
                    let _ = k;
                    PfClick::None
                }
            };
        }
        let fund = self.fund.take();
        match hit {
            Some(Hit::Amount(k)) => PfClick::Fund(if fund == Some(false) { -AMOUNTS[k] } else { AMOUNTS[k] }),
            Some(Hit::Close) => PfClick::Close,
            Some(Hit::Guide(k)) => {
                self.guide = Some(k);
                PfClick::Changed
            }
            Some(Hit::Deposit) => {
                self.fund = (fund != Some(true)).then_some(true);
                PfClick::Changed
            }
            Some(Hit::Withdraw) => {
                self.fund = (fund != Some(false)).then_some(false);
                PfClick::Changed
            }
            Some(Hit::NewAccount) => PfClick::Accounts,
            Some(Hit::Reel) => {
                self.reel = !self.reel;
                PfClick::Changed
            }
            Some(Hit::Space) => PfClick::Soon("Connect (wallet en lecture seule) : bientôt, avec toi"),
            Some(Hit::Range(k)) => {
                self.range = k;
                PfClick::Changed
            }
            Some(Hit::Metric(k)) => {
                self.metric = k;
                PfClick::Changed
            }
            Some(Hit::Tab(k)) => {
                self.tab = k;
                PfClick::Changed
            }
            Some(Hit::History) => {
                self.tab = 3;
                PfClick::Changed
            }
            _ if fund.is_some() => PfClick::Changed,
            _ => PfClick::None,
        }
    }

    /// Échap : ferme le guide ou le choix du montant, sinon la page.
    pub fn escape(&mut self) -> bool {
        if self.guide.take().is_some() || self.fund.take().is_some() {
            return false;
        }
        true
    }

    /// Dessine la page (toute la fenêtre `w` × `h`).
    pub fn draw(&self, text: &Text, w: u32, h: u32, a: &Portfolio, now: i64, price: &dyn Fn(&str) -> Option<f64>, mouse: (f64, f64)) -> Option<Pixmap> {
        // 📱 tout le contenu, puis la partie visible selon le défilement
        let ch = self.content_h(text, w as f64, h as f64).ceil() as u32;
        let full = self.draw_content(text, w, ch, a, now, price, (mouse.0, mouse.1 + self.scroll))?;
        let mut pm = if ch == h {
            full
        } else {
            let mut pm = crate::px::new(w, h)?;
            let row = full.width() as usize;
            let top = ((self.scroll * crate::px::scale() as f64).round() as usize).min((full.height() as usize).saturating_sub(pm.height() as usize));
            let n = pm.height() as usize * row;
            pm.pixels_mut().copy_from_slice(&full.pixels()[top * row..top * row + n]);
            pm
        };
        if let Some(k) = self.guide {
            self.draw_guide(&mut pm, text, k, w as f64, h as f64, mouse);
        }
        Some(pm)
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_content(&self, text: &Text, w: u32, h: u32, a: &Portfolio, now: i64, price: &dyn Fn(&str) -> Option<f64>, mouse: (f64, f64)) -> Option<Pixmap> {
        let mut pm = crate::px::new(w, h)?;
        let (wf, hf) = (w as f64, h as f64);
        crate::fill(&mut pm, 0.0, 0.0, wf, hf, pal::backdrop(), 1.0);
        let d = self.data(a, now, price);
        let (z, geo) = self.layout(text, wf, hf);
        let hover = |r: &Rect| r.contains(mouse.0, mouse.1) && self.guide.is_none();
        let x0 = geo.col1.x;
        text.draw(&mut pm, "Portfolio", (x0 as f32, 44.0), 26.0, WHITE, Align::Left);
        let pill = format!("💼 Démo · {}", a.name);
        let pw = text.width(&pill, 12.5) as f64 + 18.0;
        let px = x0 + text.width("Portfolio", 26.0) as f64 + 14.0;
        round_fill(&mut pm, Rect::new(px, 24.0, pw, 24.0), 6.0, TEAL, 0.14);
        text.draw(&mut pm, &pill, ((px + 9.0) as f32, 41.0), 12.5, TEAL, Align::Left);
        // cartes
        let card = |pm: &mut Pixmap, r: Rect| {
            round_fill(pm, r, 10.0, pal::panel(), 1.0);
            round_stroke(pm, r, 10.0, pal::edge(), 1.0);
        };
        let c1a = Rect::new(geo.col1.x, geo.col1.y, geo.col1.w, 140.0);
        let c1b = Rect::new(geo.col1.x, geo.col1.y + 152.0, geo.col1.w, 148.0);
        for r in [c1a, c1b, geo.col2, geo.col3, geo.table] {
            card(&mut pm, r);
        }
        let small = |pm: &mut Pixmap, s: &str, x: f64, y: f64| text.draw(pm, s, (x as f32, y as f32), 13.0, GREY, Align::Left);
        small(&mut pm, "Volume sur 14 jours", c1a.x + 16.0, c1a.y + 28.0);
        text.draw(&mut pm, &usd(d.vol14), ((c1a.x + 16.0) as f32, (c1a.y + 66.0) as f32), 24.0, WHITE, Align::Left);
        small(&mut pm, "Frais (14 jours)", c1b.x + 16.0, c1b.y + 28.0);
        text.draw(&mut pm, &usd(0.0), ((c1b.x + 16.0) as f32, (c1b.y + 66.0) as f32), 24.0, WHITE, Align::Left);
        small(&mut pm, "aucun frais en démo", c1b.x + 16.0, c1b.y + 92.0);
        // résumé de la période
        let rows = [
            ("P&L", usd(d.pnl), Some(d.pnl >= 0.0)),
            ("Volume", usd(d.vol), None),
            ("Drawdown max", format!("{:.2} %", d.dd * 100.0), None),
            ("Valeur totale", usd(d.equity), None),
            ("Perps", usd(d.equity), None),
            ("Spot", usd(0.0), None),
        ];
        for (k, (name, val, up)) in rows.iter().enumerate() {
            let y = geo.col2.y + 70.0 + k as f64 * 34.0;
            text.draw(&mut pm, name, ((geo.col2.x + 16.0) as f32, y as f32), 13.5, GREY, Align::Left);
            let c = match up {
                Some(true) => TEAL,
                Some(false) => crate::palette::down(),
                None => WHITE,
            };
            text.draw(&mut pm, val, ((geo.col2.x + geo.col2.w - 16.0) as f32, y as f32), 13.5, c, Align::Right);
        }
        // courbe (valeur du compte ou P&L)
        let ch = Rect::new(geo.col3.x + 16.0, geo.col3.y + 52.0, geo.col3.w - 80.0, geo.col3.h - 72.0);
        crate::fill(&mut pm, geo.col3.x + 16.0, geo.col3.y + 40.0, geo.col3.w - 32.0, 1.0, pal::edge(), 1.0);
        let pts = &d.series;
        if let (Some(f), Some(l)) = (pts.first(), pts.last()) {
            let (t0, t1) = (f.0, l.0.max(f.0 + 1));
            let (lo, hi) = pts.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| (a.min(p.1), b.max(p.1)));
            let (lo, hi) = if hi > lo {
                (lo - (hi - lo) * 0.1, hi + (hi - lo) * 0.1)
            } else {
                (lo - 1.0, hi + 1.0)
            };
            let xy = |p: &(i64, f64)| {
                (
                    (ch.x + (p.0 - t0) as f64 / (t1 - t0) as f64 * ch.w) as f32,
                    (ch.y + (hi - p.1) / (hi - lo) * ch.h) as f32,
                )
            };
            let mut line: Vec<(f32, f32)> = pts.iter().map(xy).collect();
            if line.len() == 1 {
                line.insert(0, ((ch.x) as f32, line[0].1));
            }
            let up = l.1 >= f.1;
            let c = if up { TEAL } else { crate::palette::down() };
            let mut area = line.clone();
            area.push((line[line.len() - 1].0, (ch.y + ch.h) as f32));
            area.push((line[0].0, (ch.y + ch.h) as f32));
            crate::shapes::poly_fill(&mut pm, 0.0, 0.0, &area, c, 0.18);
            lines(&mut pm, 0.0, 0.0, &[&line], c, 2.0);
            for (k, v) in [hi, (hi + lo) / 2.0, lo].iter().enumerate() {
                text.draw(
                    &mut pm,
                    &crate::viewport::format_number(*v, 0),
                    ((ch.x + ch.w + 50.0) as f32, (ch.y + k as f64 * ch.h / 2.0 + 4.0) as f32),
                    11.0,
                    GREY,
                    Align::Right,
                );
            }
        }
        // onglets du bas et leur tableau
        crate::fill(&mut pm, geo.table.x + 16.0, geo.table.y + 41.0, geo.table.w - 32.0, 1.0, pal::edge(), 1.0);
        let body = self.table_rows(a, &d, price);
        let cols = body.0.len().max(1);
        let cw = (geo.table.w - 32.0) / cols as f64;
        // (📱 narrow: each column keeps to its width, cut with « … », never over the next one)
        for (k, head) in body.0.iter().enumerate() {
            text.draw(
                &mut pm,
                &text.fit(head, 12.5, cw - 6.0),
                ((geo.table.x + 16.0 + k as f64 * cw) as f32, (geo.table.y + 70.0) as f32),
                12.5,
                GREY,
                Align::Left,
            );
        }
        if body.1.is_empty() {
            text.draw(
                &mut pm,
                body.2,
                ((geo.table.x + 16.0) as f32, (geo.table.y + 104.0) as f32),
                13.5,
                GREY,
                Align::Left,
            );
        }
        for (r, row) in body.1.iter().take(((geo.table.h - 90.0) / 28.0).max(0.0) as usize).enumerate() {
            for (k, (cell, c)) in row.iter().enumerate() {
                text.draw(
                    &mut pm,
                    &text.fit(cell, 13.0, cw - 6.0),
                    ((geo.table.x + 16.0 + k as f64 * cw) as f32, (geo.table.y + 100.0 + r as f64 * 28.0) as f32),
                    13.0,
                    *c,
                    Align::Left,
                );
            }
        }
        // boutons, périodes, onglets
        for (r, hit) in z.iter().rev() {
            let (r, hit) = (*r, *hit);
            match hit {
                Hit::Close => lines(&mut pm, r.cx(), r.cy(), &[&[(-7.0, -7.0), (7.0, 7.0)], &[(7.0, -7.0), (-7.0, 7.0)]], GREY, 1.8),
                Hit::Guide(_) | Hit::Withdraw | Hit::NewAccount | Hit::Space | Hit::Deposit | Hit::Reel => {
                    let pri = hit == Hit::Deposit;
                    round_fill(
                        &mut pm,
                        r,
                        8.0,
                        if pri {
                            TEAL
                        } else if hover(&r) {
                            pal::hover()
                        } else {
                            pal::backdrop()
                        },
                        1.0,
                    );
                    if !pri {
                        round_stroke(&mut pm, r, 8.0, pal::edge(), 1.0);
                    }
                    text.draw(
                        &mut pm,
                        bouton_nom(hit, self.reel),
                        (r.cx() as f32, (r.cy() + 5.0) as f32),
                        13.5,
                        if pri { Rgb(0x04, 0x20, 0x1c) } else { WHITE },
                        Align::Center,
                    );
                }
                Hit::Range(k) => {
                    let on = k == self.range.min(3);
                    round_fill(
                        &mut pm,
                        r,
                        6.0,
                        if on {
                            TEAL
                        } else if hover(&r) {
                            pal::hover()
                        } else {
                            pal::field()
                        },
                        if on { 0.2 } else { 1.0 },
                    );
                    round_stroke(&mut pm, r, 6.0, if on { TEAL } else { pal::edge() }, 1.0);
                    text.draw(
                        &mut pm,
                        RANGES[k].0,
                        (r.cx() as f32, (r.cy() + 4.5) as f32),
                        12.5,
                        if on { TEAL } else { WHITE },
                        Align::Center,
                    );
                }
                Hit::Metric(k) | Hit::Tab(k) => {
                    let (on, label, size) = match hit {
                        Hit::Metric(_) => (k == self.metric, ["Valeur du compte", "P&L"][k], 13.5),
                        _ => (k == self.tab, TABS[k], 14.0),
                    };
                    text.draw(
                        &mut pm,
                        label,
                        (r.x as f32, (r.cy() + 5.0) as f32),
                        size,
                        if on { WHITE } else { GREY },
                        Align::Left,
                    );
                    if on {
                        crate::fill(&mut pm, r.x, r.y + r.h - 1.0, r.w, 2.0, TEAL, 1.0);
                    }
                }
                Hit::History => text.draw(&mut pm, "Voir l'historique", (r.x as f32, (r.y + 14.0) as f32), 12.5, TEAL, Align::Left),
                Hit::Amount(k) => {
                    round_fill(&mut pm, r, 6.0, if hover(&r) { pal::hover() } else { pal::panel() }, 1.0);
                    round_stroke(&mut pm, r, 6.0, TEAL, 1.0);
                    let sign = if self.fund == Some(false) { "− " } else { "+ " };
                    text.draw(
                        &mut pm,
                        &format!("{sign}{}", usd(AMOUNTS[k])),
                        (r.cx() as f32, (r.cy() + 4.5) as f32),
                        13.0,
                        WHITE,
                        Align::Center,
                    );
                }
                Hit::GuideGo | Hit::GuideClose => {}
            }
        }
        Some(pm)
    }

    /// Tableau de l'onglet ouvert : (en-têtes, lignes de (texte, couleur), message si vide).
    fn table_rows(&self, a: &Portfolio, d: &Data, price: &dyn Fn(&str) -> Option<f64>) -> (Vec<&'static str>, Vec<Vec<(String, Rgb)>>, &'static str) {
        let pnl_c = |v: f64| if v >= 0.0 { TEAL } else { crate::palette::down() };
        match self.tab {
            0 => (
                vec!["Compte", "Solde", "Équité", "P&L latent", "Marge utilisée", "Dispo"],
                vec![vec![
                    (format!("{}  démo {}x", a.name, LEV), WHITE),
                    (usd(a.balance.to_f64()), WHITE),
                    (usd(d.equity), WHITE),
                    (usd(d.upl), pnl_c(d.upl)),
                    (usd(d.margin), WHITE),
                    (usd(d.equity - d.margin), WHITE),
                ]],
                "",
            ),
            1 => (
                vec!["Marché", "Sens", "Taille", "Entrée", "Prix", "P&L"],
                a.positions
                    .iter()
                    .map(|p| {
                        let px = price(&p.base).unwrap_or(p.entry);
                        let long = p.side == Side::Long;
                        vec![
                            (p.base.clone(), WHITE),
                            (if long { "Long" } else { "Short" }.into(), if long { TEAL } else { crate::palette::down() }),
                            (format!("{}", p.qty), WHITE),
                            (crate::viewport::format_price(p.entry, 4), WHITE),
                            (crate::viewport::format_price(px, 4), WHITE),
                            (usd(p.pnl(px)), pnl_c(p.pnl(px))),
                        ]
                    })
                    .collect(),
                "Aucune position ouverte.",
            ),
            // the demo account's orders waiting (Limite, Stop), as the ticket placed them
            2 => (
                vec!["Marché", "Sens", "Taille", "Prix", "Stop", "Objectif"],
                a.pending
                    .iter()
                    .map(|p| {
                        let o = &p.order;
                        let long = o.side == Side::Long;
                        let px = |v: Option<f64>| v.map_or("—".to_string(), |v| crate::viewport::format_price(v, 4));
                        vec![
                            (o.base.clone(), WHITE),
                            (if long { "Long" } else { "Short" }.into(), if long { TEAL } else { crate::palette::down() }),
                            (format!("{}", o.qty), WHITE),
                            (crate::viewport::format_price(o.price, 4), WHITE),
                            (px(o.stop), WHITE),
                            (px(o.target), WHITE),
                        ]
                    })
                    .collect(),
                "Aucun ordre ouvert.",
            ),
            _ => {
                let mut h = a.history.clone();
                h.sort_by_key(|t| std::cmp::Reverse(t.closed_at));
                (
                    vec!["Date", "Marché", "Sens", "Entrée", "Sortie", "P&L"],
                    h.iter()
                        .take(200)
                        .map(|t| {
                            let date = crate::crosshair_label(t.closed_at, 60);
                            let long = t.side == Side::Long;
                            vec![
                                (date, WHITE),
                                (t.base.clone(), WHITE),
                                (if long { "Long" } else { "Short" }.into(), if long { TEAL } else { crate::palette::down() }),
                                (crate::viewport::format_price(t.entry, 4), WHITE),
                                (crate::viewport::format_price(t.exit, 4), WHITE),
                                (usd(t.net().to_f64()), pnl_c(t.net().to_f64())),
                            ]
                        })
                        .collect(),
                    "Aucun trade.",
                )
            }
        }
    }

    fn draw_guide(&self, pm: &mut Pixmap, text: &Text, k: usize, w: f64, h: f64, mouse: (f64, f64)) {
        crate::fill(pm, 0.0, 0.0, w, h, Rgb(0, 0, 0), 0.5);
        let g = guide_rect(w, h);
        round_fill(pm, g, 12.0, pal::panel(), 1.0);
        round_stroke(pm, g, 12.0, pal::edge(), 1.0);
        let (_, title, desc, steps, warn) = GUIDES[k];
        let wrap = |s: &str, size: f32, max: f64| -> Vec<String> {
            let mut out = vec![String::new()];
            for word in s.split(' ') {
                let last = out.last_mut().unwrap();
                let next = if last.is_empty() { word.to_string() } else { format!("{last} {word}") };
                if text.width(&next, size) as f64 > max && !last.is_empty() {
                    out.push(word.to_string());
                } else {
                    *last = next;
                }
            }
            out
        };
        let x = g.x + 24.0;
        let room = g.w - 48.0;
        text.draw(pm, title, (x as f32, (g.y + 38.0) as f32), 18.0, WHITE, Align::Left);
        lines(
            pm,
            g.x + g.w - 29.0,
            g.y + 29.0,
            &[&[(-6.0, -6.0), (6.0, 6.0)], &[(6.0, -6.0), (-6.0, 6.0)]],
            GREY,
            1.6,
        );
        let mut y = g.y + 72.0;
        for l in wrap(desc, 13.5, room) {
            text.draw(pm, &l, (x as f32, y as f32), 13.5, GREY, Align::Left);
            y += 20.0;
        }
        y += 8.0;
        for (n, s) in steps.iter().enumerate() {
            for (j, l) in wrap(s, 13.5, room - 24.0).iter().enumerate() {
                if j == 0 {
                    text.draw(pm, &format!("{}.", n + 1), (x as f32, y as f32), 13.5, TEAL, Align::Left);
                }
                text.draw(pm, l, ((x + 24.0) as f32, y as f32), 13.5, WHITE, Align::Left);
                y += 20.0;
            }
            y += 4.0;
        }
        let lw = wrap(&format!("⚠️ {warn}"), 13.0, room - 24.0);
        let wr = Rect::new(x, y + 4.0, room, lw.len() as f64 * 19.0 + 20.0);
        round_fill(pm, wr, 8.0, crate::palette::down(), 0.1);
        round_stroke(pm, wr, 8.0, crate::palette::down(), 0.5);
        for (j, l) in lw.iter().enumerate() {
            text.draw(
                pm,
                l,
                ((x + 12.0) as f32, (wr.y + 24.0 + j as f64 * 19.0) as f32),
                13.0,
                Rgb(0xff, 0xb3, 0xb9),
                Align::Left,
            );
        }
        let go = Rect::new(g.x + 24.0, g.y + g.h - 64.0, 300.0, 40.0);
        round_fill(pm, go, 8.0, TEAL, if go.contains(mouse.0, mouse.1) { 0.85 } else { 1.0 });
        text.draw(
            pm,
            "Faire l'action sur Hyperliquid ↗",
            (go.cx() as f32, (go.cy() + 5.0) as f32),
            14.0,
            Rgb(0x04, 0x20, 0x1c),
            Align::Center,
        );
        text.draw(
            pm,
            "Argos ne signe rien et ne garde aucune clé : l'action se confirme chez Hyperliquid, dans ton wallet.",
            (x as f32, (g.y + g.h - 12.0) as f32),
            11.5,
            GREY,
            Align::Left,
        );
    }
}

/// Zones de la page.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Geo {
    col1: Rect,
    col2: Rect,
    col3: Rect,
    table: Rect,
}

/// Le nom écrit sur un bouton du haut (`reel` : les guides du compte réel sont ouverts).
fn bouton_nom(hit: Hit, reel: bool) -> &'static str {
    match hit {
        Hit::Guide(k) => GUIDES[k].0,
        Hit::Deposit => "Déposer (démo)",
        Hit::Withdraw => "Retirer (démo)",
        Hit::NewAccount => "+ Compte démo",
        Hit::Reel if reel => "Compte réel ⌄",
        Hit::Reel => "Compte réel ›",
        _ => "Connect",
    }
}

fn guide_rect(w: f64, h: f64) -> Rect {
    // (📱 narrower than the guide: its lines wrap more, its note at the bottom was cut; taller there)
    let haut: f64 = if w - 24.0 < 620.0 { 720.0 } else { 520.0 };
    let (gw, gh) = (620.0f64.min(w - 24.0), haut.min(h - 24.0));
    Rect::new((w - gw) / 2.0, (h - gh) / 2.0, gw, gh)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_blocs_empiles_et_page_qui_defile() {
        let t = Text::default();
        let mut p = PfPage::new();
        p.wheel(5000.0, &t, 375.0, 700.0);
        assert!(p.scroll > 500.0, "cell : la page défile ({})", p.scroll);
        let (_, g) = p.layout(&t, 375.0, 700.0);
        assert!(g.col2.y > g.col1.y && g.col1.x + g.col1.w <= 375.0, "blocs l'un sous l'autre, dans la largeur");
        let mut big = PfPage::new();
        big.wheel(5000.0, &t, 1400.0, 900.0);
        assert_eq!(big.scroll, 0.0, "grand écran : tout tient, pas de défilement");
    }
    use argos_core::portfolio::{ClosedTrade, Portfolio};

    /// 💼 « Ordres ouverts » lists the demo account's orders waiting (it always said « Aucun ordre ouvert. »).
    #[test]
    fn ordres_ouverts_liste_les_ordres_en_attente() {
        use argos_core::portfolio::{Order, OrderKind, Pending};
        let mut a = Portfolio::default();
        let mut p = PfPage::new();
        p.tab = 2;
        let d = Data {
            series: vec![],
            vol14: 0.0,
            pnl: 0.0,
            vol: 0.0,
            dd: 0.0,
            equity: 0.0,
            upl: 0.0,
            margin: 0.0,
        };
        let (_, rows, vide) = p.table_rows(&a, &d, &|_| None);
        assert!(rows.is_empty() && vide == "Aucun ordre ouvert.");
        a.pending.push(Pending {
            id: 1,
            kind: OrderKind::Limit,
            order: Order {
                base: "BTC".into(),
                side: Side::Long,
                qty: 0.5,
                price: 59_000.0,
                stop: Some(58_000.0),
                target: None,
                couts: None,
                perp: None,
            },
            placed_at: 0,
            until: None,
            oco: None,
            gestion: Default::default(),
            reduce_only: false,
        });
        let (_, rows, _) = p.table_rows(&a, &d, &|_| None);
        let cells: Vec<&str> = rows[0].iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(cells[..3], ["BTC", "Long", "0.5"]);
        assert!(cells[3].contains("59") && cells[4].contains("58") && cells[5] == "—", "{cells:?}");
    }

    #[test]
    fn page_portfolio() {
        let t = Text::default();
        let mut a = Portfolio::default();
        a.balance += argos_core::Money::from_f64(500.0);
        a.history.push(ClosedTrade {
            base: "BTC".into(),
            side: Side::Long,
            qty: 1.0,
            entry: 100.0,
            exit: 600.0,
            pnl: argos_core::Money::from_f64(500.0),
            fees: argos_core::Money::ZERO,
            opened_at: 1_000,
            closed_at: 2_000,
            reason: "TP".into(),
            risk: argos_core::Money::from_f64(250.0),
            partiel: false,
            glissement: argos_core::Money::ZERO,
            funding: argos_core::Money::ZERO,
            levier: None,
            plan: None,
            real_closed_at: None,
        });
        let mut p = PfPage::new();
        let d = p.data(&a, 3_000, &|_| None);
        assert_eq!((d.equity, d.pnl, d.series.len()), (100_500.0, 500.0, 3));
        assert!(d.vol14 > 0.0);
        // onglet Historique, guide, dépôt
        let (z, _) = p.layout(&t, 1600.0, 900.0);
        let at = |h: Hit| z.iter().find(|(_, x)| *x == h).map(|(r, _)| (r.cx(), r.cy())).unwrap();
        let (x, y) = at(Hit::Tab(3));
        assert_eq!(p.click(&t, 1600.0, 900.0, x, y), PfClick::Changed);
        assert_eq!(p.tab, 3);
        let (x, y) = at(Hit::Guide(6));
        p.click(&t, 1600.0, 900.0, x, y);
        let g = guide_rect(1600.0, 900.0);
        assert_eq!(p.click(&t, 1600.0, 900.0, g.x + 100.0, g.y + g.h - 44.0), PfClick::Open(HL_APP));
        assert!(!p.escape(), "Échap ferme d'abord le guide");
        let (x, y) = at(Hit::Deposit);
        p.click(&t, 1600.0, 900.0, x, y);
        let (z, _) = p.layout(&t, 1600.0, 900.0);
        let amt = z.iter().find(|(_, h)| *h == Hit::Amount(1)).unwrap().0;
        assert_eq!(p.click(&t, 1600.0, 900.0, amt.cx(), amt.cy()), PfClick::Fund(10_000.0));
        assert!(p.draw(&t, 1600, 900, &a, 3_000, &|_| None, (0.0, 0.0)).is_some());
        assert!(p.escape());
    }

    /// 📱 Au doigt : en haut, seulement le compte démo et « Compte réel › » (qui montre les 7 guides) ; pas de
    /// Connect (bientôt) ; de la place en bas pour le rond Argos AI. Le PC garde ses 11 boutons.
    #[test]
    fn au_doigt_compte_demo_d_abord() {
        let t = Text::default();
        let (w, h) = (402.0, 874.0);
        let boutons = |p: &PfPage| -> Vec<Hit> {
            let (z, _) = p.layout(&t, w, h);
            z.into_iter()
                .map(|(_, x)| x)
                .filter(|x| matches!(x, Hit::Guide(_) | Hit::Deposit | Hit::Withdraw | Hit::NewAccount | Hit::Space | Hit::Reel))
                .collect()
        };
        let mut p = PfPage::new();
        crate::actions::set_tactile(false);
        let pc = boutons(&p);
        crate::actions::set_tactile(true);
        let doigt = boutons(&p);
        assert_eq!(doigt, [Hit::Deposit, Hit::Withdraw, Hit::NewAccount, Hit::Reel]);
        let (_, g) = p.layout(&t, w, h);
        assert!(p.content_h(&t, w, h) >= g.table.y + g.table.h + 90.0, "marge pour le rond Argos AI");
        let (z, _) = p.layout(&t, w, h);
        let r = z.iter().find(|(_, x)| *x == Hit::Reel).unwrap().0;
        assert_eq!(p.click(&t, w, h, r.cx(), r.cy()), PfClick::Changed);
        let ouvert = boutons(&p);
        assert_eq!(
            ouvert.iter().filter(|x| matches!(x, Hit::Guide(_))).count(),
            GUIDES.len(),
            "les 7 guides sous « Compte réel »"
        );
        assert!(!ouvert.contains(&Hit::Space));
        assert!(p.draw(&t, w as u32, h as u32, &Portfolio::default(), 3_000, &|_| None, (0.0, 0.0)).is_some());
        crate::actions::set_tactile(false);
        assert_eq!(pc.len(), GUIDES.len() + 4, "PC : tout reste");
        assert!(pc.contains(&Hit::Space) && !pc.contains(&Hit::Reel));
    }
}
