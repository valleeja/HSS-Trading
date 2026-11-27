//! 🔐 Compte et clés : 24 mots, Face ID, clés API, clé Claude.

use crate::*;

impl App {
    /// ✨ Création du compte : « Générer » (nom + code → 24 mots) puis « Créer » (les 3 mots retapés → clés chiffrées).
    /// Une erreur garde la boîte ouverte avec ce qui a été tapé.
    pub(crate) fn account_save(&mut self, mut b: KeyBox) {
        let val = |b: &KeyBox, id: &str| b.fields.iter().find(|f| f.id == id).map(|f| f.value.trim().to_string()).unwrap_or_default();
        let (w, h) = (self.win_w(), self.area.1 as f64 + self.top_h());
        if b.exchange == "account-passkey" {
            return self.passkey_enable(b);
        }
        if b.exchange == "account-login" {
            match account::restore(&val(&b, "words"), &val(&b, "handle"), &val(&b, "code"), &val(&b, "confirm"), now_s()) {
                Ok(a) => {
                    b.wipe();
                    self.banner = Some(format!("✅ Connecté : {} (clés gardées chiffrées avec ton code sur cet appareil)", a.id_full()));
                    self.top.account = a.id_full();
                    self.session.account = Some(a);
                }
                Err(e) => {
                    self.banner = Some(format!("🔑 {e}"));
                    self.session.key_box = Some(b);
                }
            }
            return;
        }
        if b.exchange == "account-new" {
            match account::begin(&val(&b, "handle"), &val(&b, "code"), &val(&b, "confirm")) {
                Ok(d) => {
                    b.wipe();
                    self.session.words_box = Some(WordsBox::new(d.words.clone(), w, h));
                    self.session.acc_draft = Some(d);
                }
                Err(e) => {
                    self.banner = Some(format!("✨ {e}"));
                    self.session.key_box = Some(b);
                }
            }
            return;
        }
        // account-verify
        let Some(d) = self.session.acc_draft.as_ref() else { return };
        let typed: Vec<String> = ["w0", "w1", "w2"].iter().map(|id| val(&b, id)).collect();
        if !account::verify(d, &typed) {
            self.banner = Some("❌ Un des mots ne correspond pas : vérifie ce que tu as noté".into());
            self.session.key_box = Some(b);
            return;
        }
        b.wipe();
        self.banner = Some("✨ Création du compte…".into());
        let res = account::create(d, now_s());
        self.session.acc_draft = None; // les mots et le code sont effacés de la mémoire
        match res {
            Ok(a) => {
                self.banner = Some(format!(
                    "✅ Compte {} créé : ses clés sont gardées chiffrées avec ton code sur cet appareil",
                    a.id_full()
                ));
                self.top.account = a.id_full();
                self.session.account = Some(a);
            }
            Err(e) => self.banner = Some(format!("⚠️ Compte pas créé : {e}")),
        }
    }

    /// 🔐 Face ID / empreinte : navigateur et cell. Avec un compte sans passkey = l'activer (le code ouvre ses clés) ;
    /// sans compte = se connecter avec Face ID (paquet chiffré de cet appareil). Desktop : bientôt (code QR).
    pub(crate) fn passkey_start(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.banner = Some("🔐 Face ID / empreinte : dans le navigateur et sur cell. Desktop : bientôt, avec un code QR à scanner avec ton cell".into());
        }
        #[cfg(target_arch = "wasm32")]
        match (&self.session.account, passkey::load()) {
            (Some(a), Some(_)) => self.banner = Some(format!("🔐 Face ID est déjà activé pour {}", a.id_full())),
            (Some(_), None) => {
                let (w, h) = (self.win_w(), self.area.1 as f64 + self.top_h());
                self.session.key_box = Some(KeyBox::new("account-passkey", "Compte Argos", w, h));
            }
            (None, packet) => {
                let Some(proxy) = self.proxy.clone() else { return };
                let Some(p) = packet else {
                    self.banner = Some("🔐 Pas de Face ID sur cet appareil : crée ton compte ou connecte-toi avec tes 24 mots, puis active Face ID".into());
                    return self.redraw();
                };
                self.banner = Some("🔐 Face ID…".into());
                argos_data::rt::spawn(move || async move {
                    let res = async {
                        use base64::Engine;
                        let cred = base64::engine::general_purpose::STANDARD
                            .decode(&p.cred)
                            .map_err(|_| "paquet abîmé".to_string())?;
                        let (cred, prf) = passkey::get(&cred).await?;
                        passkey::open(&p, &cred, &prf)
                    }
                    .await;
                    let _ = proxy.send_event(UserEvent::Passkey(res, true));
                });
            }
        }
        self.redraw();
    }

    /// 🔐 Activer Face ID : le code ouvre les clés du compte, puis la passkey les chiffre sur cet appareil.
    pub(crate) fn passkey_enable(&mut self, mut b: KeyBox) {
        let code = b.fields.first().map(|f| f.value.trim().to_string()).unwrap_or_default();
        let opened = vault::load(vault::ACCOUNT_FILE)
            .ok_or_else(|| "les clés du compte ne sont pas sur cet appareil".to_string())
            .and_then(|s| vault::open(&s, &code));
        b.wipe();
        let (Some(a), Some(proxy)) = (self.session.account.clone(), self.proxy.clone()) else {
            return;
        };
        let secret = match opened {
            Ok(s) => s,
            Err(e) => {
                self.banner = Some(format!("🔐 {e}"));
                return;
            }
        };
        let payload = serde_json::json!({ "account": a, "secret": secret }).to_string();
        self.banner = Some("🔐 Face ID / empreinte…".into());
        argos_data::rt::spawn(move || async move {
            #[cfg(target_arch = "wasm32")]
            let res = async {
                let user = argos_core::account::nostr_user_id(&a.pub_hex);
                let (cred, prf) = passkey::create(&user, &a.id_full()).await?;
                passkey::store(&passkey::seal(&cred, &prf, &payload)?)?;
                Ok(format!("🔐 Face ID activé pour {} sur cet appareil", a.id_full()))
            }
            .await;
            #[cfg(not(target_arch = "wasm32"))]
            let res: Result<String, String> = {
                let _ = (&a, &payload);
                Err("Face ID : navigateur et cell seulement".into())
            };
            let _ = proxy.send_event(UserEvent::Passkey(res, false));
        });
    }

    /// Clic dans la boîte des 24 mots.
    pub(crate) fn words_click(&mut self, c: WordsClick) {
        match c {
            WordsClick::None => {}
            WordsClick::Copy => {
                let text = self.session.words_box.as_ref().map(|b| b.words.join(" ")).unwrap_or_default();
                let ok = self.clipboard.as_mut().is_some_and(|c| c.set_text(text).is_ok());
                self.banner = Some(
                    if ok {
                        "📋 Copié : colle-les dans un endroit sûr, puis efface le presse-papiers"
                    } else {
                        "Copie impossible : note-les à la main"
                    }
                    .into(),
                );
            }
            WordsClick::Continue => {
                if let Some(mut b) = self.session.words_box.take() {
                    b.wipe();
                }
                if let Some(d) = &self.session.acc_draft {
                    let (w, h) = (self.win_w(), self.area.1 as f64 + self.top_h());
                    self.session.key_box = Some(KeyBox::verify(d.ask, w, h));
                }
            }
            WordsClick::Cancel => {
                if let Some(mut b) = self.session.words_box.take() {
                    b.wipe();
                }
                self.session.acc_draft = None;
                self.banner = Some("✨ Création du compte annulée : les mots sont effacés".into());
            }
        }
    }

    /// Enregistrer dans la boîte de clé : chaque champ part au trousseau, puis la boîte oublie tout.
    pub(crate) fn key_save(&mut self) {
        let Some(mut b) = self.session.key_box.take() else { return };
        if b.exchange.starts_with("claude") {
            return self.claude_key_save(b);
        }
        if b.exchange.starts_with("account-") {
            return self.account_save(b);
        }
        #[cfg(not(target_arch = "wasm32"))]
        if b.exchange.starts_with("journal-") {
            return self.journal_secours_save(b);
        }
        if b.exchange == "custom" {
            // ➕ API personnalisée : le nom choisi fait la carte, la clé va au trousseau sous ce nom
            let val = |id: &str| b.fields.iter().find(|f| f.id == id).map(|f| f.value.trim().to_string()).unwrap_or_default();
            let (name, key) = (val("name"), val("key"));
            b.wipe();
            let id = custom_id(&name);
            if id == "custom-" {
                self.banner = Some("➕ Donne un nom à cette API".into());
                return;
            }
            #[cfg(not(target_arch = "wasm32"))]
            if let Err(e) = keys::save(&id, "key", &key) {
                self.banner = Some(format!("⚠️ {name} : {e} (le trousseau du système est-il ouvert ?)"));
                return;
            }
            #[cfg(target_arch = "wasm32")]
            let _ = key;
            if !self.ex_prefs.custom.iter().any(|n| custom_id(n) == id) {
                self.ex_prefs.custom.push(name.clone());
                Self::write_prefs("exchanges.json", &self.ex_prefs);
            }
            self.banner = Some(format!("🔑 {name} : enregistrée dans le trousseau de ton ordi"));
            return;
        }
        if b.exchange == "cmc" {
            // 📈 CoinMarketCap : la clé au trousseau, le lien dans les réglages (tout ce qui a été tapé est effacé)
            let val = |id: &str| b.fields.iter().find(|f| f.id == id).map(|f| f.value.trim().to_string()).unwrap_or_default();
            let (key, url, every) = (val("key"), val("url"), val("every"));
            b.wipe();
            return self.cmc_key_save(key, url, every);
        }
        if b.exchange == "oanda" {
            // 🏦 jeton OANDA : trousseau sur desktop ; navigateur = en mémoire pour la session (rien d'écrit)
            let val = |id: &str| b.fields.iter().find(|f| f.id == id).map(|f| f.value.trim().to_string()).unwrap_or_default();
            let (token, env) = (val("token"), val("env").to_lowercase());
            b.wipe();
            let live = env.starts_with('r') || env.starts_with('l');
            if !argos_data::oanda::token_ok(&token) {
                self.banner = Some("🏦 Jeton OANDA invalide : copie-le au complet depuis hub.oanda.com ▸ Personal token".into());
                return;
            }
            #[cfg(not(target_arch = "wasm32"))]
            if let Err(e) = keys::save("oanda", "token", &token).and_then(|_| keys::save("oanda", "env", if live { "live" } else { "practice" })) {
                self.banner = Some(format!("⚠️ OANDA : {e} (le trousseau du système est-il ouvert ?)"));
                return;
            }
            self.session.oanda = Some((token, live));
            self.refresh_catalogs(); // 🏦 son catalogue complet
            return self.oanda_test();
        }
        let Some(ex) = Exchange::parse(&b.exchange) else { return };
        let res: Result<(), String> = b.fields.iter().try_for_each(|f| keys::save(ex.id(), f.id, f.value.trim()));
        b.wipe();
        match res {
            Ok(()) => {
                self.session.key_set.insert(ex);
                self.banner = Some(format!("🔑 {} : enregistré dans le trousseau de ton ordi", ex.name()));
            }
            Err(e) => self.banner = Some(format!("⚠️ {} : {e} (le trousseau du système est-il ouvert ?)", ex.name())),
        }
    }

    /// ✨ Coffre de la clé Claude : « claude » = nouvelle clé + code (chiffrée puis gardée), « claude-unlock » = le
    /// code ouvre le coffre pour la session. Ce qui a été tapé est effacé dans tous les cas.
    pub(crate) fn claude_key_save(&mut self, mut b: KeyBox) {
        use argos_render::assistant::KeyState;
        let val = |id: &str| b.fields.iter().find(|f| f.id == id).map(|f| f.value.trim().to_string()).unwrap_or_default();
        let (key, code, confirm) = (val("key"), val("code"), val("confirm"));
        let unlock = b.exchange == "claude-unlock";
        b.wipe();
        let res = if unlock {
            vault::load(vault::FILE)
                .ok_or_else(|| "aucune clé gardée sur cet appareil".to_string())
                .and_then(|s| vault::open(&s, &code))
        } else if !key.starts_with("sk-ant-") {
            Err("une clé Claude commence par « sk-ant- »".into())
        } else if code.chars().count() < 4 {
            Err("ton code doit avoir au moins 4 caractères".into())
        } else if code != confirm {
            Err("les deux codes ne sont pas pareils".into())
        } else {
            vault::seal(&key, &code).and_then(|s| vault::store(vault::FILE, &s)).map(|_| key)
        };
        match res {
            Ok(k) => {
                self.session.claude_key = Some(k);
                self.ai.key = KeyState::Ready;
                self.banner = Some(if unlock {
                    "🔓 Clé Claude ouverte pour la session".into()
                } else {
                    "🔐 Clé Claude chiffrée et gardée sur cet appareil".into()
                });
            }
            Err(e) => self.banner = Some(format!("⚠️ Claude : {e}")),
        }
    }
}
