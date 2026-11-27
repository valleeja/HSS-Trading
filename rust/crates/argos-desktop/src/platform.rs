//! 🖥️ / 🌐 Ce qui change entre le desktop et le navigateur (WASM), écrit à un seul endroit :
//! - **réglages** : un fichier par nom dans `~/.config/argos/charts/` (écriture sûre), ou le stockage de la page
//!   (`localStorage`, clé `argos:<nom>`), partagé avec l'app de bureau par le serveur local (`app/synchro.js`,
//!   `/api/reglages` de `argos-outils serveur`) ;
//! - **fichiers pour toi** (export du journal, capture) : dossier Téléchargements, ou téléchargement du navigateur ;
//! - **presse-papiers** : celui du système, ou celui du navigateur ;
//! - **ouvrir un lien** : le navigateur du système, ou un nouvel onglet.

/// Lit le réglage `name` (« toolbar.json », « lang/de.json », « hyperliquid-btc.json »).
pub fn read(name: &str) -> Option<String> {
    #[cfg(not(target_arch = "wasm32"))]
    return std::fs::read_to_string(crate::persist::saved::dir()?.join(name)).ok();
    #[cfg(target_arch = "wasm32")]
    return web::storage()?.get_item(&web::cle(name)).ok().flatten();
}

/// 🔄 Ce que cette app vient d'écrire (empreinte par réglage) : le fil qui surveille le dossier ne le prend pas pour
/// un changement venu d'ailleurs.
#[cfg(not(target_arch = "wasm32"))]
pub mod ecrits {
    use std::collections::HashMap;
    use std::hash::{Hash, Hasher};
    use std::sync::{Mutex, OnceLock};

    fn table() -> &'static Mutex<HashMap<String, u64>> {
        static T: OnceLock<Mutex<HashMap<String, u64>>> = OnceLock::new();
        T.get_or_init(Default::default)
    }

    fn empreinte(text: &str) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        text.hash(&mut h);
        h.finish()
    }

    /// Heure (ms) de la dernière écriture.
    pub static DERNIERE: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

    pub fn noter(name: &str, text: &str) {
        if let Ok(mut t) = table().lock() {
            t.insert(name.to_string(), empreinte(text));
        }
    }

    /// Le fichier contient-il exactement ce que cette app y a écrit en dernier ?
    pub fn est_le_mien(name: &str, text: &str) -> bool {
        table().lock().is_ok_and(|t| t.get(name) == Some(&empreinte(text)))
    }
}

/// Heure actuelle (ms), sur le bureau comme dans le navigateur.
pub fn maintenant_ms() -> i64 {
    web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

/// 💾 Une sauvegarde est-elle en cours ? Bureau : une écriture il y a moins de 0,7 s ; page web avec le serveur
/// local : un réglage pas encore reçu par le serveur (`argosSynchro.enCours()`), sinon comme le bureau.
pub fn sauvegarde_en_cours() -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    return maintenant_ms() - ecrits::DERNIERE.load(std::sync::atomic::Ordering::Relaxed) < 700;
    #[cfg(target_arch = "wasm32")]
    return web::sauvegarde_en_cours();
}

/// 🌐 Efface le réglage `name` du stockage de la page (l'ancienne copie du journal, une fois rendue au PC).
#[cfg(target_arch = "wasm32")]
pub fn effacer(name: &str) {
    if let Some(s) = web::storage() {
        let _ = s.remove_item(&web::cle(name));
    }
}

/// 🗂 State of this tab only (never shared, never synced): on the web the page's `sessionStorage`, which survives a
/// reload (F5) of the same tab but not other tabs or devices; on the desktop app (no F5) in memory only, gone when the
/// app quits, so a new launch follows the usual rules.
pub mod onglet {
    #[cfg(not(target_arch = "wasm32"))]
    thread_local! {
        static T: std::cell::RefCell<std::collections::HashMap<String, String>> = Default::default();
    }

    /// Reads the tab state `name`.
    pub fn lire(name: &str) -> Option<String> {
        #[cfg(not(target_arch = "wasm32"))]
        return T.with(|t| t.borrow().get(name).cloned());
        #[cfg(target_arch = "wasm32")]
        return web_sys::window()?
            .session_storage()
            .ok()
            .flatten()?
            .get_item(&super::web::cle(name))
            .ok()
            .flatten();
    }

    /// Writes the tab state `name` (a full session storage only loses it).
    pub fn ecrire(name: &str, text: &str) {
        #[cfg(not(target_arch = "wasm32"))]
        T.with(|t| t.borrow_mut().insert(name.to_string(), text.to_string()));
        #[cfg(target_arch = "wasm32")]
        if let Some(s) = web_sys::window().and_then(|w| w.session_storage().ok().flatten()) {
            let _ = s.set_item(&super::web::cle(name), text);
        }
    }
}

/// Écrit le réglage `name` (desktop : fichier temporaire puis renommage, un fichier à moitié écrit ne remplace
/// jamais le bon).
pub fn write(name: &str, text: &str) -> Result<(), String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = crate::persist::saved::dir().ok_or("pas de dossier de config")?.join(name);
        let write = || -> std::io::Result<()> {
            if let Some(d) = path.parent() {
                std::fs::create_dir_all(d)?;
            }
            // un nom de fichier temporaire par écriture : deux écritures en même temps ne se marchent pas dessus
            static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let tmp = path.with_extension(format!("{}.tmp", n));
            std::fs::write(&tmp, text)?;
            ecrits::noter(name, text);
            ecrits::DERNIERE.store(maintenant_ms(), std::sync::atomic::Ordering::Relaxed);
            std::fs::rename(tmp, &path)
        };
        write().map_err(|e| e.to_string())
    }
    #[cfg(target_arch = "wasm32")]
    {
        web::storage()
            .ok_or("stockage du navigateur indisponible")?
            .set_item(&web::cle(name), text)
            .map_err(|_| "stockage du navigateur plein".to_string())?;
        // 🔄 session partagée : le réglage part aussi à l'app de bureau (serveur local, page synchro.js) — 🧪 pas en
        // mode essai (?essai=1 : réglages à part, rien ne touche la vraie session)
        if !web::essai() {
            web::DERNIERE.with(|d| d.set(maintenant_ms()));
            web::call_page("argosSynchroEcrire", &[name, text]);
        }
        Ok(())
    }
}

/// Donne un fichier à l'utilisateur (`name`, contenu) ; rend où il est allé.
pub fn save_download(name: &str, bytes: &[u8]) -> Result<String, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let dir = downloads();
        std::fs::write(dir.join(name), bytes).map_err(|e| format!("{} : {e}", dir.join(name).display()))?;
        Ok(dir.display().to_string())
    }
    #[cfg(target_arch = "wasm32")]
    web::download(name, bytes).map(|_| "tes téléchargements".to_string())
}

/// 📤 Partage une image PNG et un texte : sur le téléphone, la feuille de partage du système (Web Share API, par
/// `argosShare` de la page /app/ ; sans elle, l'image est téléchargée) ; sur l'ordi, l'image va dans Téléchargements.
pub fn share_image(name: &str, png: &[u8], text: &str) -> Result<String, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = text;
        save_download(name, png)
    }
    #[cfg(target_arch = "wasm32")]
    {
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(png);
        web::call_page("argosShare", &[name, &b64, text]);
        Ok("la feuille de partage".to_string())
    }
}

/// 📄 Prints a page of HTML (lot 8, « Imprimer en PDF »): 🌐 `html` is the page to print, given to the /app/ page
/// (`argosImprimer`: the browser's print window, which offers « Enregistrer en PDF »); 🖥 `html` is a whole document:
/// it goes to Downloads (`name`) and opens in the browser, which prints it. Gives where it went.
pub fn print_page(name: &str, html: &str) -> Result<String, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let dir = save_download(name, html.as_bytes())?;
        // (🧪 the tests never open a browser)
        #[cfg(not(test))]
        open_url(&std::path::Path::new(&dir).join(name).display().to_string());
        Ok(dir)
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = name;
        web::call_page("argosImprimer", &[html]);
        Ok("la fenêtre d'impression".to_string())
    }
}

/// Relit un fichier que l'utilisateur a mis dans ses téléchargements (`None` dans le navigateur : pas d'accès).
pub fn read_download(name: &str) -> Option<String> {
    #[cfg(not(target_arch = "wasm32"))]
    return std::fs::read_to_string(downloads().join(name)).ok();
    #[cfg(target_arch = "wasm32")]
    {
        let _ = name;
        None
    }
}

/// 🧩 Noms des fichiers des Téléchargements qui finissent par `ext` (desktop ; le navigateur choisit les fichiers).
#[cfg(not(target_arch = "wasm32"))]
pub fn list_downloads(ext: &str) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(downloads())
        .map(|d| d.filter_map(|e| e.ok()?.file_name().into_string().ok()).filter(|n| n.ends_with(ext)).collect())
        .unwrap_or_default();
    v.sort();
    v
}

/// ⚖️ The latest CSV file in Downloads that Argos did not write (TradingView's « List of trades »): (name, bytes).
#[cfg(not(target_arch = "wasm32"))]
pub fn dernier_csv_telecharge() -> Option<(String, Vec<u8>)> {
    let d = downloads();
    let (_, p) = std::fs::read_dir(&d)
        .ok()?
        .flatten()
        .filter(|e| {
            let n = e.file_name().to_string_lossy().to_lowercase();
            n.ends_with(".csv") && !n.starts_with("argos-")
        })
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .max_by_key(|(t, _)| *t)?;
    Some((p.file_name()?.to_string_lossy().into_owned(), std::fs::read(&p).ok()?))
}

/// 🛟 📎 The latest PNG / JPEG image in Downloads (the most recent): (name, bytes).
#[cfg(not(target_arch = "wasm32"))]
pub fn derniere_image_telechargee() -> Option<(String, Vec<u8>)> {
    let d = downloads();
    let (_, p) = std::fs::read_dir(&d)
        .ok()?
        .flatten()
        .filter(|e| {
            let n = e.file_name().to_string_lossy().to_lowercase();
            n.ends_with(".png") || n.ends_with(".jpg") || n.ends_with(".jpeg")
        })
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .max_by_key(|(t, _)| *t)?;
    Some((p.file_name()?.to_string_lossy().into_owned(), std::fs::read(&p).ok()?))
}

/// 🧩 Relit un fichier binaire des Téléchargements (desktop).
#[cfg(not(target_arch = "wasm32"))]
pub fn read_download_bytes(name: &str) -> Option<Vec<u8>> {
    std::fs::read(downloads().join(name)).ok()
}

/// 🔔 Alert fired: its sound `son` (a name of `argos_core::alerts::SONS`, or « aucun »: silent) at `volume` (0 – 100)
/// and the system notification (browser: only if allowed, by `argosAlert` of the page /app/, which synthesises the
/// sound). On the PC: the app's banner is enough, and the sound is the terminal's beep.
pub fn alert_notify(title: &str, body: &str, son: &str, volume: u8) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (title, body);
        jouer_son(son, volume);
    }
    #[cfg(target_arch = "wasm32")]
    web::call_page("argosAlert", &[title, body, son, &volume.to_string()]);
}

/// 🔊 ▶ Plays the alert sound `son` at `volume` now (to choose it), without a notification.
pub fn alert_son(son: &str, volume: u8) {
    #[cfg(not(target_arch = "wasm32"))]
    jouer_son(son, volume);
    #[cfg(target_arch = "wasm32")]
    web::call_page("argosSon", &[son, &volume.to_string()]);
}

/// 🔊 PC: the terminal's beep (if the app was started there; no audio dependency), none for « Son coupé » or at
/// volume 0.
#[cfg(not(target_arch = "wasm32"))]
fn jouer_son(son: &str, volume: u8) {
    if son == argos_core::alerts::SON_COUPE || volume == 0 {
        return;
    }
    #[cfg(test)]
    SONS_JOUES.with(|v| v.borrow_mut().push(son.to_string()));
    #[cfg(not(test))]
    print!("\x07");
}

#[cfg(all(test, not(target_arch = "wasm32")))]
thread_local! {
    /// 🧪 The sounds played in this test thread.
    static SONS_JOUES: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// 🧪 The sounds played since the last call (this test thread).
#[cfg(all(test, not(target_arch = "wasm32")))]
pub fn sons_joues() -> Vec<String> {
    SONS_JOUES.with(|v| std::mem::take(&mut *v.borrow_mut()))
}

/// 🔔 Première alerte créée : le navigateur demande la permission des notifications (une fois).
pub fn ask_notifications() {
    #[cfg(target_arch = "wasm32")]
    web::call_page("argosAlertAsk", &[]);
}

/// Ouvre un lien dans le navigateur.
pub fn open_url(url: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
    #[cfg(target_arch = "wasm32")]
    if let Some(w) = web_sys::window() {
        let _ = w.open_with_url_and_target(url, "_blank");
    }
}

/// Dossier où vont les fichiers pour toi (tests : un dossier de test dans rust/target/tmp).
#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn dossier_telechargements() -> std::path::PathBuf {
    downloads()
}

/// Dossier Téléchargements (sinon le dossier personnel).
#[cfg(not(target_arch = "wasm32"))]
fn downloads() -> std::path::PathBuf {
    // 🧪 les tests n'écrivent jamais dans les vrais Téléchargements (le bouton Partager y laissait une image à chaque
    // `make vert`)
    #[cfg(test)]
    {
        let d = argos_core::dossiers::de_test("telechargements");
        let _ = std::fs::create_dir_all(&d);
        return d;
    }
    #[allow(unreachable_code)]
    let home = std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    ["Téléchargements", "Downloads"]
        .iter()
        .map(|d| home.join(d))
        .find(|p| p.is_dir())
        .unwrap_or(home)
}

/// Presse-papiers : celui du système (desktop) ou du navigateur. Dans le navigateur, une page ne peut pas lire le
/// presse-papiers d'elle-même : coller arrive par l'événement « coller » de la page (`web::listen_paste`).
pub struct Clipboard {
    #[cfg(not(target_arch = "wasm32"))]
    inner: arboard::Clipboard,
}

impl Clipboard {
    pub fn new() -> Option<Self> {
        #[cfg(not(target_arch = "wasm32"))]
        return arboard::Clipboard::new().ok().map(|inner| Self { inner });
        #[cfg(target_arch = "wasm32")]
        return Some(Self {});
    }

    pub fn get_text(&mut self) -> Result<String, String> {
        #[cfg(not(target_arch = "wasm32"))]
        return self.inner.get_text().map_err(|e| e.to_string());
        #[cfg(target_arch = "wasm32")]
        return Err("dans le navigateur, coller arrive par l'événement de la page".to_string());
    }

    pub fn set_text(&mut self, text: String) -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        return self.inner.set_text(text).map_err(|e| e.to_string());
        #[cfg(target_arch = "wasm32")]
        {
            let w = web_sys::window().ok_or("pas de fenêtre")?;
            let _ = w.navigator().clipboard().write_text(&text);
            Ok(())
        }
    }
}

/// 🌐 Web version: is this key left to the browser instead of the app? winit cancels (`preventDefault`) every key
/// it gets, so a key the app does not use must never reach it, or the browser loses it. One rule:
/// - F1…F12 (the app only uses F5, which is the browser's reload anyway), Alt + ← / → / Début (history);
/// - Ctrl / ⌘ + a letter or digit, except the app's own: K (search), Z / Y (undo, redo), C (copy a drawing),
///   S (save the script), A and X (kept so the page text is never selected or cut); Ctrl + Maj + I / J / C (dev
///   tools) always go to the browser;
/// - Ctrl + V when a text field is open (`saisie`): a page cannot read the clipboard itself, pasted text only
///   comes through the browser's "paste" event (`web::listen_paste`); on the chart, Ctrl + V stays the app's
///   « paste a drawing »;
/// - Ctrl + Tab, Page ↑ / ↓, + = − 0 (the page zoom keys, taken first by the page for the app's own zoom).
///
/// `key`: `KeyboardEvent.key`; `ctrl`: Ctrl or ⌘. Lone modifiers, arrows, Entrée, ⌫… always go to the app.
#[cfg(any(target_arch = "wasm32", test))]
pub fn touche_du_navigateur(key: &str, ctrl: bool, shift: bool, alt: bool, saisie: bool) -> bool {
    let k = key.to_ascii_lowercase();
    let mut c = k.chars();
    let un = c.next().filter(|_| c.next().is_none());
    if k.len() >= 2 && k.starts_with('f') && k[1..].parse::<u8>().is_ok_and(|n| (1..=12).contains(&n)) {
        return true;
    }
    if alt && !ctrl {
        return matches!(k.as_str(), "arrowleft" | "arrowright" | "home");
    }
    if !ctrl || alt {
        return false;
    }
    if shift && matches!(k.as_str(), "i" | "j" | "c") {
        return true;
    }
    match un {
        Some('v') => saisie,
        Some('k' | 'z' | 'y' | 'c' | 's' | 'a' | 'x') => false,
        Some(ch) if ch.is_ascii_alphanumeric() => true,
        Some('+' | '=' | '-') => true,
        _ => matches!(k.as_str(), "tab" | "pageup" | "pagedown"),
    }
}

#[cfg(target_arch = "wasm32")]
pub mod web {
    use wasm_bindgen::{JsCast, JsValue};

    /// 🧪 Mode essai (`?essai=1` dans l'adresse) : réglages sous une autre clé, jamais synchronisés — pour essayer au
    /// doigt sans toucher la vraie session.
    pub fn essai() -> bool {
        thread_local! {
            static E: bool = web_sys::window()
                .and_then(|w| w.location().search().ok())
                .is_some_and(|q| q.contains("essai=1"));
        }
        E.with(|e| *e)
    }

    /// Clé du stockage de la page pour le réglage `name`.
    pub fn cle(name: &str) -> String {
        if essai() {
            format!("argos_essai:{name}") // (pas « argos- » : ce sont les données d'Argos v1)
        } else {
            format!("argos:{name}")
        }
    }

    /// Appelle la fonction `name` de la page (définie dans /app/index.html), si elle existe, avec des textes.
    /// 📤 Pose `octets` (RGBA, `w` × `h`) sur le canvas à la hauteur `y` (pixels du canvas) : un seul putImageData.
    pub fn poser_image(canvas: &web_sys::HtmlCanvasElement, octets: &[u8], w: u32, h: u32, y: f64) {
        let Ok(Some(ctx)) = canvas.get_context("2d") else { return };
        let Ok(ctx) = ctx.dyn_into::<web_sys::CanvasRenderingContext2d>() else {
            return;
        };
        if let Ok(img) = web_sys::ImageData::new_with_u8_clamped_array_and_sh(wasm_bindgen::Clamped(octets), w, h) {
            let _ = ctx.put_image_data(&img, 0.0, y);
        }
    }

    pub fn call_page(name: &str, args: &[&str]) {
        let Some(w) = web_sys::window() else { return };
        let Ok(f) = js_sys::Reflect::get(&w, &JsValue::from_str(name)) else { return };
        let Some(f) = f.dyn_ref::<js_sys::Function>() else { return };
        let a = js_sys::Array::new();
        for s in args {
            a.push(&JsValue::from_str(s));
        }
        let _ = f.apply(&w, &a);
    }

    /// Prend (et vide) la liste globale `name` de la page : tableaux de chaînes posés par la page /app/.
    fn take_rows(name: &str) -> Vec<Vec<String>> {
        let Some(w) = web_sys::window() else { return vec![] };
        let key = JsValue::from_str(name);
        let Ok(v) = js_sys::Reflect::get(&w, &key) else { return vec![] };
        let Some(arr) = v.dyn_ref::<js_sys::Array>() else { return vec![] };
        if arr.length() == 0 {
            return vec![];
        }
        let rows = arr
            .iter()
            .filter_map(|r| {
                r.dyn_ref::<js_sys::Array>()
                    .map(|r| r.iter().map(|x| x.as_string().unwrap_or_default()).collect())
            })
            .collect();
        let _ = js_sys::Reflect::set(&w, &key, &js_sys::Array::new());
        rows
    }

    /// 🧩 Fichiers choisis dans la fenêtre de choix de la page (`argosPick`) : (nom, contenu en base64).
    /// 🛟 Images pasted in the page (Ctrl + V): (name, base64), set aside by the page (`argosColle`).
    pub fn take_colle() -> Vec<(String, String)> {
        take_rows("argosColle")
            .into_iter()
            .filter_map(|r| Some((r.first()?.clone(), r.get(1)?.clone())))
            .collect()
    }

    pub fn take_picked() -> Vec<(String, String)> {
        take_rows("argosPicked")
            .into_iter()
            .filter_map(|r| Some((r.first()?.clone(), r.get(1)?.clone())))
            .collect()
    }

    thread_local! {
        /// Heure (ms) de la dernière écriture dans le stockage de la page.
        pub static DERNIERE: std::cell::Cell<i64> = const { std::cell::Cell::new(0) };
    }

    /// 💾 Un réglage pas encore reçu par le serveur local, ou une écriture il y a moins de 0,7 s.
    pub fn sauvegarde_en_cours() -> bool {
        let recent = super::maintenant_ms() - DERNIERE.with(|d| d.get()) < 700;
        let attente = web_sys::window()
            .and_then(|w| js_sys::Reflect::get(&w, &JsValue::from_str("argosSynchro")).ok())
            .and_then(|s| js_sys::Reflect::get(&s, &JsValue::from_str("enCours")).ok())
            .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
            .and_then(|f| f.call0(&JsValue::NULL).ok())
            .is_some_and(|v| v.is_truthy());
        recent || attente
    }

    /// 🔄 `window.argosSynchroReveil(noms)` : la page (synchro.js) dit quels réglages ont changé ailleurs (un nom par
    /// ligne) ; `on` les fait relire.
    pub fn reveil_synchro(on: impl Fn(Vec<String>) + 'static) {
        let Some(w) = web_sys::window() else { return };
        let cb = wasm_bindgen::closure::Closure::<dyn Fn(String)>::new(move |s: String| {
            // vide : seulement redessiner (la sauvegarde vient de finir)
            on(s.lines().filter(|l| !l.is_empty()).map(str::to_string).take(200).collect());
        });
        let _ = js_sys::Reflect::set(&w, &JsValue::from_str("argosSynchroReveil"), cb.as_ref());
        cb.forget();
    }

    /// 🩺 Diagnostic en lecture seule : `window.argosEtat` (le dernier clic / toucher et la zone qui l'a reçu), pour
    /// vérifier la vraie page au doigt.
    pub fn etat(s: &str) {
        let Some(w) = web_sys::window() else { return };
        let _ = js_sys::Reflect::set(&w, &JsValue::from_str("argosEtat"), &JsValue::from_str(s));
    }

    /// 🩺 Read-only diagnostic: `window.argosOrderflow` (counters and times of the real orderflow of each followed
    /// market, `App::orderflow_diag`), to check the page with the capture tool. No account data, no key.
    pub fn orderflow(s: &str) {
        let Some(w) = web_sys::window() else { return };
        let _ = js_sys::Reflect::set(&w, &JsValue::from_str("argosOrderflow"), &JsValue::from_str(s));
    }

    /// 🧩 Ajoute un résultat de module à la liste de la page (`argosModOut`), comme le Worker : utilisé par la coque
    /// Tauri, où le module tourne dans le cœur Rust.
    pub fn push_mod_result(req: &str, ok: bool, v: &str) {
        let Some(w) = web_sys::window() else { return };
        let key = JsValue::from_str("argosModOut");
        let arr = match js_sys::Reflect::get(&w, &key).ok().and_then(|a| a.dyn_into::<js_sys::Array>().ok()) {
            Some(a) => a,
            None => {
                let a = js_sys::Array::new();
                let _ = js_sys::Reflect::set(&w, &key, &a);
                a
            }
        };
        arr.push(&js_sys::Array::of3(&req.into(), &(if ok { "1" } else { "0" }).into(), &v.into()));
    }

    /// 🧩 Résultats des modules lancés dans un Worker (`argosModRun`) : (requête, ok, sortie en base64 ou erreur).
    pub fn take_mod_results() -> Vec<(String, bool, String)> {
        take_rows("argosModOut")
            .into_iter()
            .filter_map(|r| Some((r.first()?.clone(), r.get(1)? == "1", r.get(2)?.clone())))
            .collect()
    }

    pub fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok().flatten()
    }

    /// Téléchargement par le navigateur (lien temporaire vers le contenu).
    pub fn download(name: &str, bytes: &[u8]) -> Result<(), String> {
        let e = |_: JsValue| "téléchargement impossible".to_string();
        let parts = js_sys::Array::of1(&js_sys::Uint8Array::from(bytes));
        let blob = web_sys::Blob::new_with_u8_array_sequence(&parts).map_err(e)?;
        let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(e)?;
        let doc = web_sys::window().and_then(|w| w.document()).ok_or("pas de document")?;
        let a: web_sys::HtmlAnchorElement = doc.create_element("a").map_err(e)?.dyn_into().map_err(|_| "lien".to_string())?;
        a.set_href(&url);
        a.set_download(name);
        a.click();
        let _ = web_sys::Url::revoke_object_url(&url);
        Ok(())
    }

    /// 📥 Paires (clé, valeur) du stockage de la page dont la clé commence par « argos- » (données d'Argos v1, même site).
    pub fn v1_pairs() -> Vec<(String, String)> {
        let Some(s) = storage() else { return vec![] };
        let n = s.length().unwrap_or(0);
        (0..n)
            .filter_map(|i| s.key(i).ok().flatten())
            .filter(|k| k.starts_with("argos-"))
            .filter_map(|k| Some((k.clone(), s.get_item(&k).ok().flatten()?)))
            .collect()
    }

    thread_local! {
        /// Dernière position du doigt / de la souris, en pixels de la page (comme la taille du canvas).
        static POINTER: std::cell::Cell<Option<(f64, f64)>> = const { std::cell::Cell::new(None) };
    }

    /// Note la position de chaque doigt / souris **avant** winit (capture), en pixels de la page : les
    /// coordonnées de winit sont fausses sur les écrans ×2, ×3.
    pub fn listen_pointer() {
        let Some(w) = web_sys::window() else { return };
        let f = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::PointerEvent)>::new(move |e: web_sys::PointerEvent| {
            POINTER.with(|p| p.set(Some((e.client_x() as f64, e.client_y() as f64))));
        });
        for ev in ["pointermove", "pointerdown", "pointerup"] {
            let _ = w.add_event_listener_with_callback_and_bool(ev, f.as_ref().unchecked_ref(), true);
        }
        f.forget();
    }

    thread_local! {
        /// 👆 Heure (horloge des gestes) du dernier doigt vu par la page, notée tout de suite : les événements
        /// « souris » que le navigateur tire pour ce même doigt arrivent à l'app AVANT le message du doigt (file
        /// d'attente) ; sans cette heure, le premier toucher après une pause était pris aussi pour un clic de souris.
        static DOIGT_VU: std::cell::Cell<f64> = const { std::cell::Cell::new(f64::NEG_INFINITY) };
    }

    /// 👆 Heure du dernier doigt vu par la page (`f64::NEG_INFINITY` : aucun).
    pub fn doigt_vu() -> f64 {
        DOIGT_VU.with(|c| c.get())
    }

    /// 👆 Chaque doigt posé sur le canvas, suivi jusqu'à ce qu'il se lève, en pixels de l'interface :
    /// `on(doigt, phase, x, y)` avec 0 posé, 1 bouge, 2 levé, 3 annulé. Lu directement sur la page (pas par winit, qui
    /// ne voit pas les mouvements du doigt sur Safari iOS) ; un doigt posé ailleurs (bandeau, fenêtre de la page) est
    /// ignoré.
    /// 👆 The fingers on the canvas, read by the page: `on(finger, phase, x, y, te)`, `te` being when the finger did it
    /// (its `timeStamp`, on `touch::now_ms`'s clock; B-048), read now when the browser's time does not match.
    pub fn listen_touch(on: impl Fn(u64, u8, f64, f64, f64) + 'static) {
        let Some(w) = web_sys::window() else { return };
        let down = std::rc::Rc::new(std::cell::RefCell::new(Vec::<i32>::new()));
        let f = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::PointerEvent)>::new(move |e: web_sys::PointerEvent| {
            if e.pointer_type() != "touch" {
                return;
            }
            let lu = crate::input::touch::now_ms();
            DOIGT_VU.with(|c| c.set(lu));
            let id = e.pointer_id();
            let phase = match e.type_().as_str() {
                "pointerdown" => 0,
                "pointermove" => 1,
                "pointerup" => 2,
                _ => 3,
            };
            let mut d = down.borrow_mut();
            if phase == 0 {
                let on_canvas = e.target().and_then(|t| t.dyn_into::<web_sys::HtmlCanvasElement>().ok()).is_some();
                if !on_canvas {
                    return;
                }
                d.retain(|&k| k != id);
                d.push(id);
            } else if !d.contains(&id) {
                return;
            } else if phase >= 2 {
                d.retain(|&k| k != id);
            }
            drop(d);
            let (x, y) = to_ui(e.client_x() as f64, e.client_y() as f64);
            // 👆 (B-048) how long ago the finger did it: the page's clock (`performance.now`) minus the event's own
            // time; a value that does not fit (an old browser's date) counts as now
            let age = page_now().map(|n| n - e.time_stamp()).filter(|a| (0.0..=5_000.0).contains(a)).unwrap_or(0.0);
            on(id as u32 as u64, phase, x, y, lu - age);
        });
        for ev in ["pointerdown", "pointermove", "pointerup", "pointercancel"] {
            let _ = w.add_event_listener_with_callback_and_bool(ev, f.as_ref().unchecked_ref(), true);
        }
        f.forget();
    }

    /// 👆 The page's clock (`performance.now()`, milliseconds), the one of the events' `timeStamp`.
    fn page_now() -> Option<f64> {
        let w = web_sys::window()?;
        let perf = js_sys::Reflect::get(&w, &JsValue::from_str("performance")).ok()?;
        let now = js_sys::Reflect::get(&perf, &JsValue::from_str("now")).ok()?;
        now.dyn_ref::<js_sys::Function>()?.call0(&perf).ok()?.as_f64()
    }

    thread_local! {
        /// 📐 Vue du canvas : agrandissement de l'interface (au doigt) et coin haut gauche du canvas dans la page
        /// (zones sûres : encoche, barre du bas d'iOS).
        static VIEW: std::cell::Cell<(f64, f64, f64)> = const { std::cell::Cell::new((1.0, 0.0, 0.0)) };
    }

    /// 📐 Agrandissement `zoom` et coin du canvas (`ox`, `oy`) dans la page, revus à chaque image.
    pub fn set_view(zoom: f64, ox: f64, oy: f64) {
        VIEW.with(|v| v.set((zoom.max(0.5), ox, oy)));
    }

    /// 📐 (agrandissement, x, y du canvas dans la page).
    pub fn view() -> (f64, f64, f64) {
        VIEW.with(|v| v.get())
    }

    /// Point de la page → point de l'interface (sans le décalage du canvas, divisé par l'agrandissement).
    pub fn to_ui(x: f64, y: f64) -> (f64, f64) {
        let (z, ox, oy) = view();
        ((x - ox) / z, (y - oy) / z)
    }

    /// Dernière position du doigt / de la souris (pixels de l'interface).
    pub fn pointer() -> Option<(f64, f64)> {
        POINTER.with(|p| p.get()).map(|(x, y)| to_ui(x, y))
    }

    /// 📐 Ce que la page sait de l'écran (`argosScreen` de /app/) : taille, échelle, pointeur, segments, posture.
    pub fn screen_info() -> Option<String> {
        let w = web_sys::window()?;
        let f = js_sys::Reflect::get(&w, &JsValue::from_str("argosScreen")).ok()?;
        f.dyn_ref::<js_sys::Function>()?.call0(&w).ok()?.as_string()
    }

    thread_local! {
        /// 📱 Zones où on tape (pixels de la page) : y toucher ouvre le clavier du cell.
        static ZONES: std::cell::RefCell<Vec<(f64, f64, f64, f64)>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    /// 📱 Zones où on tape (x, y, largeur, hauteur), remises à jour à chaque image.
    pub fn set_text_zones(z: Vec<(f64, f64, f64, f64)>) {
        ZONES.with(|v| *v.borrow_mut() = z);
    }

    /// 📱 Clavier du cell : un champ caché de la page prend le focus quand le doigt touche une zone où on tape (le
    /// clavier ne s'ouvre que pour un vrai champ, et seulement pendant le toucher) ; ce qu'on tape va à `on_text`,
    /// Effacer / Entrée à `on_key`. Souris et clavier d'ordi : rien ne change (le canvas garde le clavier).
    pub fn listen_keyboard(on_text: impl Fn(String) + 'static, on_key: impl Fn(&'static str) + 'static) {
        let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return };
        let Ok(el) = doc.create_element("input") else { return };
        let _ = el.set_attribute("id", "argos-kb");
        let _ = el.set_attribute("autocomplete", "off");
        let _ = el.set_attribute("autocapitalize", "sentences");
        // 16 px : sinon le cell zoome la page ; presque invisible, en bas de l'écran
        let _ = el.set_attribute(
            "style",
            "position:fixed;left:0;bottom:0;width:1px;height:1px;opacity:0.01;font-size:16px;border:0;padding:0;",
        );
        let Ok(input) = el.dyn_into::<web_sys::HtmlInputElement>() else { return };
        if let Some(body) = doc.body() {
            let _ = body.append_child(&input);
        }
        let inp = input.clone();
        let on_input = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
            let v = inp.value();
            if !v.is_empty() {
                inp.set_value("");
                on_text(v);
            }
        });
        let _ = input.add_event_listener_with_callback("input", on_input.as_ref().unchecked_ref());
        on_input.forget();
        let on_down = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
            let k = match e.key().as_str() {
                "Backspace" => "Backspace",
                "Enter" => "Enter",
                "Escape" => "Escape",
                "ArrowUp" => "ArrowUp",
                "ArrowDown" => "ArrowDown",
                _ => return,
            };
            e.prevent_default();
            on_key(k);
        });
        let _ = input.add_event_listener_with_callback("keydown", on_down.as_ref().unchecked_ref());
        on_down.forget();
        let inp = input.clone();
        let on_up = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::PointerEvent)>::new(move |e: web_sys::PointerEvent| {
            if e.pointer_type() != "touch" {
                return;
            }
            let (x, y) = to_ui(e.client_x() as f64, e.client_y() as f64);
            if ZONES.with(|z| z.borrow().iter().any(|&(zx, zy, zw, zh)| x >= zx && x <= zx + zw && y >= zy && y <= zy + zh)) {
                let _ = inp.focus();
            } else {
                let _ = inp.blur();
            }
        });
        if let Some(w) = web_sys::window() {
            let _ = w.add_event_listener_with_callback_and_bool("pointerup", on_up.as_ref().unchecked_ref(), true);
        }
        on_up.forget();
        // 📱 Safari (iPhone) n'ouvre le clavier que pendant un vrai geste : on refait le focus sur « touchend » aussi
        // (position du doigt lue dans changedTouches[0])
        let inp = input.clone();
        let on_end = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::Event)>::new(move |e: web_sys::Event| {
            let t = js_sys::Reflect::get(&e, &JsValue::from_str("changedTouches"))
                .ok()
                .and_then(|l| js_sys::Reflect::get(&l, &JsValue::from_f64(0.0)).ok())
                .filter(|t| !t.is_undefined());
            let Some(t) = t else { return };
            let n = |k: &str| js_sys::Reflect::get(&t, &JsValue::from_str(k)).ok().and_then(|v| v.as_f64()).unwrap_or(-1.0);
            let (x, y) = to_ui(n("clientX"), n("clientY"));
            if ZONES.with(|z| z.borrow().iter().any(|&(zx, zy, zw, zh)| x >= zx && x <= zx + zw && y >= zy && y <= zy + zh)) {
                let _ = inp.focus();
            }
        });
        if let Some(w) = web_sys::window() {
            let _ = w.add_event_listener_with_callback_and_bool("touchend", on_end.as_ref().unchecked_ref(), true);
        }
        on_end.forget();
    }

    /// 🌐 Les raccourcis du navigateur passent toujours : attrapés sur la page avant la chart (phase de capture), ils
    /// ne vont pas à winit, qui bloquerait l'action du navigateur (`preventDefault` sur toutes les touches).
    pub fn listen_browser_keys() {
        let Some(w) = web_sys::window() else { return };
        let f = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
            let saisie = ZONES.with(|z| !z.borrow().is_empty());
            if super::touche_du_navigateur(&e.key(), e.ctrl_key() || e.meta_key(), e.shift_key(), e.alt_key(), saisie) {
                e.stop_immediate_propagation();
            }
        });
        let _ = w.add_event_listener_with_callback_and_bool("keydown", f.as_ref().unchecked_ref(), true);
        f.forget();
    }

    /// Chaque « coller » de la page (Ctrl + V, menu du cell) donne son texte à `on_paste`.
    pub fn listen_paste(on_paste: impl Fn(String) + 'static) {
        let Some(w) = web_sys::window() else { return };
        let f = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::ClipboardEvent)>::new(move |e: web_sys::ClipboardEvent| {
            if let Some(t) = e.clipboard_data().and_then(|d| d.get_data("text").ok()).filter(|t| !t.is_empty()) {
                e.prevent_default();
                on_paste(t);
            }
        });
        let _ = w.add_event_listener_with_callback("paste", f.as_ref().unchecked_ref());
        f.forget();
    }
}
