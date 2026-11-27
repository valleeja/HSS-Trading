//! 💼 Plusieurs comptes démo (comme v1) : chacun a ses positions, ses ordres en attente, son historique, son journal
//! et ses réglages du gardien PropFirm. Le compte choisi vit dans `App::demo` et le journal ; les autres sont gardés
//! dans demo_accounts.json. Le compte d'avant (demo_account.json + journal.json) devient « Démo 1 », sans rien perdre.

use crate::*;
use argos_core::journal::{JCfg, JTrade};
use argos_core::portfolio::Portfolio;
use argos_render::listmenu::{MenuGroup, MenuItem};
use serde::{Deserialize, Serialize};

/// Un compte démo complet.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct DemoAccount {
    pub(crate) pf: Portfolio,
    pub(crate) trades: Vec<JTrade>,
    pub(crate) cfg: JCfg,
}

/// Tous les comptes démo ; `list[active]` est copié depuis l'app à chaque sauvegarde.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct DemoAccounts {
    pub(crate) active: usize,
    pub(crate) list: Vec<DemoAccount>,
}

/// Soldes de départ proposés pour un nouveau compte.
const STARTS: [f64; 5] = [10_000.0, 25_000.0, 50_000.0, 100_000.0, 200_000.0];
const ID_NEW: i64 = 1;
const ID_RENAME: i64 = 20;
const ID_DELETE: i64 = 30;
const ID_CONFIRM: i64 = 31;
const ID_PICK: i64 = 100;

impl App {
    /// Au démarrage : sans demo_accounts.json, le compte actuel devient « Démo 1 ».
    pub(crate) fn accounts_ready(&mut self) {
        if self.accounts.list.is_empty() {
            if self.demo.name == Portfolio::default().name {
                self.demo.name = "Démo 1".into();
            }
            self.accounts.active = 0;
            self.accounts.list.push(self.current_account());
            self.save_accounts();
        }
        self.accounts.active = self.accounts.active.min(self.accounts.list.len() - 1);
    }

    fn current_account(&self) -> DemoAccount {
        let j = &self.watch.panels.journal;
        DemoAccount {
            pf: self.demo.clone(),
            trades: j.trades.clone(),
            cfg: j.cfg.clone(),
        }
    }

    /// Garde le compte choisi dans la liste, puis tout sur disque.
    pub(crate) fn save_accounts(&mut self) {
        let cur = self.current_account();
        if let Some(a) = self.accounts.list.get_mut(self.accounts.active) {
            *a = cur;
        }
        Self::write_prefs("demo_accounts.json", &self.accounts);
        Self::write_prefs("demo_account.json", &self.demo);
        self.journal_ecrire();
    }

    /// Passe au compte `i` (le compte quitté est gardé tel quel).
    pub(crate) fn switch_account(&mut self, i: usize) {
        let Some(next) = self.accounts.list.get(i).cloned() else { return };
        self.save_accounts();
        self.accounts.active = i;
        self.demo = next.pf;
        let j = &mut self.watch.panels.journal;
        (j.trades, j.cfg) = (next.trades, next.cfg);
        self.save_accounts();
        self.banner = Some(format!(
            "💼 Compte démo : {} (solde {} $)",
            self.demo.name,
            argos_render::viewport::format_price(self.demo.balance.to_f64(), 0)
        ));
    }

    /// Nouveau compte démo avec ce solde de départ (réglages du gardien par défaut), choisi tout de suite.
    pub(crate) fn new_account(&mut self, start: f64) {
        self.save_accounts();
        let n = self.accounts.list.len() + 1;
        let cfg = JCfg {
            account: start,
            ..Default::default()
        };
        self.accounts.list.push(DemoAccount {
            pf: Portfolio::new(&format!("Démo {n}"), start, 0.0),
            trades: vec![],
            cfg,
        });
        self.switch_account(self.accounts.list.len() - 1);
    }

    /// Supprime le compte choisi (jamais le dernier) et passe au premier.
    pub(crate) fn delete_account(&mut self) -> bool {
        if self.accounts.list.len() <= 1 {
            self.banner = Some("💼 Il faut garder au moins un compte démo".into());
            return false;
        }
        let gone = self.accounts.list.remove(self.accounts.active);
        let next = self.accounts.list[0].clone();
        self.accounts.active = 0;
        self.demo = next.pf;
        let j = &mut self.watch.panels.journal;
        (j.trades, j.cfg) = (next.trades, next.cfg);
        self.save_accounts();
        self.banner = Some(format!("💼 Compte « {} » supprimé", gone.pf.name));
        true
    }

    /// Menu des comptes démo (étiquette du compte en haut du panneau Trading démo).
    pub(crate) fn accounts_menu(&self) -> Vec<MenuGroup> {
        let item = |id: i64, label: String, checked: bool| MenuItem {
            id,
            label,
            enabled: true,
            checked,
            ..Default::default()
        };
        let money = |v: f64| argos_render::viewport::format_price(v, 0);
        let mut groups = vec![
            MenuGroup {
                title: "COMPTES DÉMO".into(),
                items: self
                    .accounts
                    .list
                    .iter()
                    .enumerate()
                    .map(|(i, a)| {
                        let (name, bal) = if i == self.accounts.active {
                            (&self.demo.name, self.demo.balance)
                        } else {
                            (&a.pf.name, a.pf.balance)
                        };
                        item(ID_PICK + i as i64, format!("{name} · {} $", money(bal.to_f64())), i == self.accounts.active)
                    })
                    .collect(),
                strip: false,
            },
            MenuGroup {
                title: "NOUVEAU COMPTE (solde de départ)".into(),
                items: STARTS
                    .iter()
                    .enumerate()
                    .map(|(k, &s)| item(ID_NEW + k as i64, format!("{} $", money(s)), false))
                    .collect(),
                strip: true,
            },
        ];
        let mut this = vec![item(ID_RENAME, format!("Renommer « {} »", self.demo.name), false)];
        if self.accounts.list.len() > 1 {
            this.push(if self.ui.delete_armed {
                item(ID_CONFIRM, format!("⚠️ Toucher encore pour supprimer « {} »", self.demo.name), false)
            } else {
                item(ID_DELETE, format!("Supprimer « {} »", self.demo.name), false)
            });
        }
        groups.push(MenuGroup {
            title: "CE COMPTE".into(),
            items: this,
            strip: false,
        });
        groups
    }

    /// Choix dans le menu des comptes ; `true` si le menu reste ouvert (suppression à confirmer).
    pub(crate) fn accounts_pick(&mut self, id: i64, anchor: Rect) -> bool {
        match id {
            i if i >= ID_PICK => {
                self.ui.delete_armed = false;
                self.switch_account((i - ID_PICK) as usize);
                false
            }
            i if (ID_NEW..ID_NEW + STARTS.len() as i64).contains(&i) => {
                self.ui.delete_armed = false;
                self.new_account(STARTS[(i - ID_NEW) as usize]);
                false
            }
            ID_RENAME => {
                let name = self.demo.name.clone();
                self.ui.name_box = Some(NameBox::new(NameFor::Account, &name, anchor, self.win_w()));
                false
            }
            ID_DELETE => {
                self.ui.delete_armed = true;
                true
            }
            ID_CONFIRM => {
                self.ui.delete_armed = false;
                self.delete_account();
                false
            }
            _ => false,
        }
    }
}
