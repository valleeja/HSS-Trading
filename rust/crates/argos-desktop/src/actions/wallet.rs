//! 👛 Wallet en lecture seule : on colle son adresse publique Hyperliquid (jamais la clé privée), l'app montre la
//! valeur du compte, les positions, les soldes spot, les ordres ouverts et les derniers trades (API publique, sans
//! clé). Relu aux 30 s tant que la carte du dock ou le menu est ouvert. Rien n'est signé ni envoyé.

use crate::*;
use argos_render::listmenu::{MenuGroup, MenuItem};

/// Réglage gardé (`wallet.json`) : seulement l'adresse publique.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct WalletPrefs {
    pub(crate) address: String,
}

const REFRESH: i64 = 1;
const CHANGE: i64 = 2;
const FORGET: i64 = 3;

impl App {
    /// Menu Argos ▸ Wallet : la boîte de l'adresse s'il n'y en a pas, sinon l'état du compte.
    pub(crate) fn open_wallet(&mut self, anchor: Rect) {
        if self.feeds.wallet_addr.is_empty() {
            self.ui.name_box = Some(NameBox::new(NameFor::Wallet, "", anchor, self.win_w()));
            self.banner = Some("👛 Colle ton adresse publique Hyperliquid (0x…) : lecture seule, jamais ta clé privée".into());
            return self.redraw();
        }
        self.wallet_refresh(true);
        self.open_list(ListKind::Wallet, anchor);
    }

    /// Adresse tapée dans la boîte.
    pub(crate) fn wallet_typed(&mut self, value: &str) {
        let a = value.trim().to_string();
        if !argos_data::wallet::is_address(&a) {
            self.banner = Some("👛 Adresse invalide : 0x suivi de 40 caractères (ton adresse publique, pas ta clé privée)".into());
            return self.redraw();
        }
        self.feeds.wallet_addr = a.clone();
        self.feeds.wallet = None;
        Self::write_prefs("wallet.json", &WalletPrefs { address: a });
        self.wallet_refresh(true);
        self.banner = Some("👛 Adresse enregistrée : lecture seule (aucune clé, aucun ordre)".into());
        self.redraw();
    }

    /// Relit l'état de l'adresse (`force` : tout de suite, sinon aux 30 s).
    pub(crate) fn wallet_refresh(&mut self, force: bool) {
        let addr = self.feeds.wallet_addr.clone();
        if addr.is_empty() || self.feeds.wallet_busy || (!force && self.feeds.wallet_at.is_some_and(|t| t.elapsed().as_secs() < 30)) {
            return;
        }
        let Some(proxy) = self.proxy.clone() else { return };
        self.feeds.wallet_busy = true;
        self.feeds.wallet_at = Some(Instant::now());
        argos_data::rt::spawn(move || async move {
            let res = argos_data::wallet::snapshot(&addr).await;
            let _ = proxy.send_event(UserEvent::Wallet(addr, res.map_err(String::from)));
        });
    }

    pub(crate) fn wallet_loaded(&mut self, addr: String, res: Result<argos_data::wallet::Snapshot, String>) {
        self.feeds.wallet_busy = false;
        if addr != self.feeds.wallet_addr {
            return;
        }
        match res {
            Ok(s) => (self.feeds.wallet, self.feeds.wallet_note) = (Some(s), String::new()),
            Err(e) => self.feeds.wallet_note = format!("⚠️ {e}"),
        }
        // menu ouvert : refait avec les nouveaux chiffres
        if let Some((ListKind::Wallet, m)) = self.ui.list.take() {
            let (ww, wh) = (self.win_w(), self.area.1 as f64);
            self.ui.list = Some((ListKind::Wallet, ListMenu::new(self.wallet_menu(), vec![], m.anchor, ww, wh)));
        } else if let Some(l) = self.ui.list.take() {
            self.ui.list = Some(l);
        }
        self.redraw();
    }

    /// Menu 👛 : les lignes de l'état, puis ⟳ / changer / oublier.
    pub(crate) fn wallet_menu(&self) -> Vec<MenuGroup> {
        let th = Theme::current();
        let rows = argos_render::dock::wallet_rows(&self.feeds.wallet_addr, self.feeds.wallet.as_ref(), self.tz_offset_s());
        let mut info: Vec<MenuItem> = rows
            .into_iter()
            .map(|(l, v, c)| MenuItem {
                id: -1,
                label: l,
                hint: v,
                enabled: true,
                color: c.or(Some(th.fg)),
                ..Default::default()
            })
            .collect();
        if self.feeds.wallet.is_none() {
            let wait = if self.feeds.wallet_note.is_empty() {
                "Lecture en cours…".to_string()
            } else {
                self.feeds.wallet_note.clone()
            };
            info.push(MenuItem {
                id: -1,
                label: wait,
                enabled: false,
                ..Default::default()
            });
        }
        let act = |id: i64, label: &str| MenuItem {
            id,
            label: label.into(),
            enabled: true,
            ..Default::default()
        };
        vec![
            MenuGroup {
                title: "👛 WALLET · LECTURE SEULE".into(),
                items: info,
                strip: false,
            },
            MenuGroup {
                title: String::new(),
                items: vec![
                    act(REFRESH, "⟳ Relire"),
                    act(CHANGE, "✏️ Changer l'adresse"),
                    act(FORGET, "🗑 Oublier l'adresse"),
                ],
                strip: false,
            },
        ]
    }

    /// Ligne choisie dans le menu 👛 : `true` = le menu reste ouvert.
    pub(crate) fn wallet_pick(&mut self, id: i64, anchor: Rect) -> bool {
        match id {
            REFRESH => {
                self.wallet_refresh(true);
                true
            }
            CHANGE => {
                let a = self.feeds.wallet_addr.clone();
                self.ui.name_box = Some(NameBox::new(NameFor::Wallet, &a, anchor, self.win_w()));
                false
            }
            FORGET => {
                (self.feeds.wallet_addr, self.feeds.wallet) = (String::new(), None);
                Self::write_prefs("wallet.json", &WalletPrefs::default());
                self.banner = Some("👛 Adresse oubliée".into());
                false
            }
            _ => true,
        }
    }
}
