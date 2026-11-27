//! Argos HSS Trading — fenêtre desktop.
//! `argos-desktop [exchange] [symbole] [unité]`  ex. `argos-desktop hl BTC 30m` (exchanges : hl, okx, binance, bybit, bitstamp, weex, bitunix, blofin)
//! Hors ligne : `argos-desktop fichier.json [unité]` (liste de bougies `{time, open, high, low, close, volume}`).
//! Toutes les actions (clavier, barre de navigation en bas, menus clic droit / ⚙ / ➕) passent par la table unique
//! `argos_render::actions` : nom, raccourci et effet écrits une seule fois.
//! Souris : molette = zoom · glisser = déplacer la chart (dans le temps ; aussi en prix en mode libre) ·
//! double-clic sur la chart = mode libre (au démarrage : toute la chart bouge) ⇄ mode normal (prix auto) · glisser l'échelle de prix = écraser / étirer ·
//! double-clic sur l'échelle de prix = auto · clic droit = menu de ce qui est sous la souris (graphique, échelle de prix,
//! échelle de temps) · objets dessinés : main ✋ au survol, cliquer-glisser pour les déplacer.

#![forbid(unsafe_code)]

mod actions;
mod app;
mod events;
mod fond;
mod input;
mod persist;
mod platform;
mod render;
mod run;
mod security;
mod state;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_lens;
mod types;

use app::App;
use types::*;

use events::UserEvent;
/// 💥 Hyperliquid pour la carte des liquidations : bougies (temps s, o, h, l, c) et intérêt ouvert du moment ($).
pub(crate) type LiqHl = (Vec<(i64, f64, f64, f64, f64)>, Option<f64>);
use persist::{saved, v1import};
use render::{ui_size, window_icon};
use run::{load_live, run, start_live, start_load, Loaded, Source};
#[cfg(target_arch = "wasm32")]
use security::passkey;
use security::{account, keys, vault};

use argos_core::interval::SECTIONS;
use argos_core::store::{Search, SymbolStore, WatchList};
use argos_core::Instrument;
use argos_core::{aggregate, Bar, Interval, Timeframe};
use argos_data::cache;
use argos_data::live::{self, LiveEvent};
use argos_data::supervisor::Supervisor;
use argos_data::watchlist as hl_watch;
use argos_data::{Exchange, DEFAULT_CANDLES};
use argos_render::actions::{self as chart_actions, ChartAction};
use argos_render::bottombar::{self, Hit};
use argos_render::context_menu::{self, Anchor, Click, ContextMenu, Ctx, Source as MenuSource};
use argos_render::drawings::{Drawing, Frame, Point, Style};
use argos_render::exchanges::{ExAction, ExRow, KeyBox, KeyClick};
use argos_render::goto::{self, GoTo, Outcome, Target};
use argos_render::listmenu::{ListMenu, MenuClick, MenuGroup, MenuItem};
use argos_render::objdlg::{DlgClick, ObjDialog, Tab as DlgTab};
use argos_render::objmenu::{self, ObjAction};
use argos_render::phonebar::PhoneHit;
use argos_render::rail::{self, RAIL_W};
use argos_render::settings::{self, ChartSettings};
use argos_render::shapes::Rect;
use argos_render::sheet::{Act, Sheet, SheetClick, SheetKind};
use argos_render::toolbar::{self, Btn};
use argos_render::topbar::{self, Stat, Tick, TopBar, TopClick, TopMenu};
use argos_render::topmenu::{self, TopAct};
use argos_render::viewport::{data_decimals, format_price, PriceScale, Viewport};
use argos_render::watchlist::{self as wl, Watchlist};
use argos_render::wlmenu::{AddClick, AddPanel, Chip, Found, NameBox, NameFor};
use argos_render::wordsbox::{WordsBox, WordsClick};
/// La base des symboles et watchlists : redb sur desktop, en mémoire dans le navigateur (listes gardées par la page).
#[cfg(not(target_arch = "wasm32"))]
type Store = argos_store::Store;
#[cfg(target_arch = "wasm32")]
type Store = argos_store::MemStore;
use argos_render::{current_scale, frame, render, slots, CandleStyle, Chart, Theme};
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use web_time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{CursorIcon, Window, WindowId};

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    // intervalle : celui de la commande (ex. « 5m », « 50m », « 4h », « 1D », « 15s »), sinon le dernier choisi
    let (src, iv) = match args.get(1).filter(|a| a.ends_with(".json")) {
        Some(path) => (Source::File(path.clone()), args.get(2).and_then(|s| Interval::parse(s))),
        // 🔄 sans rien demandé : le dernier marché ouvert (sur cet appareil ou un autre)
        None if args.len() < 2 => match actions::synchro::derniere() {
            Some((exchange, base, iv)) => (Source::Live { exchange, base }, iv),
            None => (
                Source::Live {
                    exchange: Exchange::Hyperliquid,
                    base: "BTC".into(),
                },
                None,
            ),
        },
        None => {
            let exchange = args.get(1).and_then(|s| Exchange::parse(s)).unwrap_or(Exchange::Hyperliquid);
            // HIP-3 (« xyz:NVDA ») : le nom du dex reste tel quel
            let base = args.get(2).cloned().unwrap_or_else(|| "BTC".into());
            let base = if base.contains(':') { base } else { base.to_uppercase() };
            (Source::Live { exchange, base }, args.get(3).and_then(|s| Interval::parse(s)))
        }
    };
    run(src, iv);
}

/// Navigateur : le marché vient de l'adresse (`/app?ex=hl&s=BTC&tf=15m`), sinon BTC sur Hyperliquid.
#[cfg(target_arch = "wasm32")]
fn main() {
    console_error_panic_hook::set_once();
    let params = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .and_then(|q| web_sys::UrlSearchParams::new_with_str(&q).ok());
    let get = |k: &str| params.as_ref().and_then(|p| p.get(k));
    // 🔄 sans marché dans l'adresse : le dernier ouvert (sur cet appareil ou un autre, session partagée)
    if get("ex").is_none() && get("s").is_none() && get("dex").is_none() {
        if let Some((exchange, base, iv)) = actions::synchro::derniere() {
            return run(Source::Live { exchange, base }, get("tf").and_then(|s| Interval::parse(&s)).or(iv));
        }
    }
    let exchange = get("ex").and_then(|s| Exchange::parse(&s)).unwrap_or(Exchange::Hyperliquid);
    let base = get("s").unwrap_or_else(|| "BTC".into());
    let base = if base.contains(':') { base } else { base.to_uppercase() };
    let iv = get("tf").and_then(|s| Interval::parse(&s));
    // 🦎 paire DEX : `?dex=eth:0x…:PEPE/WETH`
    if let Some((network, pool, name)) = get("dex").and_then(|d| argos_data::dex::parse_id(&format!("dex:{d}"))) {
        return run(Source::Dex { network, pool, name }, iv);
    }
    run(Source::Live { exchange, base }, iv);
}
