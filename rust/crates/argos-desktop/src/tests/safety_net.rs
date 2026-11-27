//! 🕸 Safety net of the split into modules (step 0, AIP-0001): a reference for every screen of the app.
//!
//! What it protects: moving code from one place to another must change nothing on the screen. So every screen the
//! app can show is drawn (without a window, as the other tests of `tests.rs` do) and compared with what it drew
//! before the move.
//!
//! - **The scenes** ([`scenes::all`]) come from the code's own lists (panels, column modules, pages, menus, sheets,
//!   indicators, candle styles, themes), on the PC (1440 × 900, the mouse), on the phone (Jee's: 502 × 972 points,
//!   a finger) and on the tablet lying down and upright (1092 × 734 and 819 × 1008, a finger): [`Form::size`]. A
//!   scene starts from the app at rest, opens its screen with the function the app itself calls, and is drawn, the
//!   chart whole each time, until two images in a row are the same.
//! - **The reference** of a scene is text, not an image: a fingerprint of the whole image, of each named zone of the
//!   screen (`App::zones_pc`) and of a grid of tiles, in `tests/references/safety-net-screens.txt`. The comparison is
//!   exact. A wanted change: `ARGOS_REFERENCES=maj`, then read the diff of that file (it names the scenes and the
//!   zones that changed) and commit it.
//! - **With their data** ([`answers`]): every kind of message the app receives from its other threads (candles, a
//!   book, the answer of a source or its refusal, a setting changed on another device, the keyboard and the fingers
//!   of the page) arrives made up in a scene, on a screen that shows what the app does with it.
//! - **In their other states** ([`states`]): what no list of the code names is reached with made-up data or by
//!   presses: each thing a script of the user can draw; each control of the full pages (home, portfolio, Marketplace,
//!   Academy, backtest report), pressed wherever a press does something, down to the end of the page; each control
//!   of the windows over the chart ([`windows`]), and what a press brings pressed in turn.
//! - **The tables of the chart** ([`tables`]): moved, scaled and stretched by the mouse and by the finger, their menu,
//!   and where the app keeps them when the list of the legend unfolds.
//! - **Nothing forgotten** ([`universe`]): a screen of those lists or a kind of message that no scene shows makes
//!   `every_screen_has_a_scene` fail, and a new variant in one of the lists does not compile until it is listed.
//! - **When a source stops** ([`stopped`], the data reliability tests): the screens that show the data of a source
//!   are drawn before and after that source stops; a stop that changes nothing on the screen is written down there.
//! - **On its own**: the scenes are drawn by copies of this test program (one per processor, 4 at most), so the
//!   settings the app keeps for the whole program (theme, language, glass) are never changed by another test while a
//!   scene is drawn. Each scene has its own thread, its own empty settings folder and a frozen clock. A copy cannot
//!   reach outside the app: it has no network (`App::blank` has no proxy), no program to start (so no browser), no
//!   system keyring and no personal folder.
//! - **The same on every computer, at any hour, on any day**: the data are made up ([`furniture`]), the clock is
//!   frozen, what leaves by itself stays (a banner, a bubble), and the one part of a screen that shows the hour it
//!   really is (the form of a new trade of the journal, [`universe::real_clock`]) is left out of its image. The app
//!   has two clocks no test can freeze (`argos_render::journal::now_ms`, and the age of a pool in
//!   `argos_render::dock`): an example read with one of them is dated from the real clock, never from the frozen
//!   one, or its screen changes with the day. The check of a new scene, when in doubt: this test program run with
//!   the computer's clock moved (libfaketime: `faketime -m -f '+1d x4' <the test program> safety_net`, tomorrow
//!   and with time going four times faster, as on a computer four times slower), where nothing may differ. The
//!   whole net was run so a day, nine days, forty days and seven months ahead, in two time zones, as on a computer
//!   four times faster, four times slower and a hundred times slower.
//!
//! On a difference the message names the scene and its zones, and the new image is written next to the reference
//! file (`safety-net-<scene>.nouveau.png`, the changed tiles framed in red). When many screens changed alike (a
//! button every screen shows), the message says it in short first: each kind of change (the same zones and the same
//! tiles of a form), how many screens it has and one of them to look at ([`in_short`]). `ARGOS_CAPTURES=<folder>`
//! writes the image of every scene there (to look at them, or to compare two commits side by side).
//! `ARGOS_SCENES=<text>` draws only the scenes whose name contains that text (or the one screen the text names
//! whole, as a message of this test writes it): a quick look while working on one screen, which is never a pass.

use super::*;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use argos_render::formfactor::{choose, Choice, Screen};

mod alerts;
mod answers;
mod boundaries;
mod furniture;
mod saved;
mod scenes;
mod states;
mod stopped;
mod tables;
mod universe;
mod windows;

/// The window a scene is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Form {
    Pc,
    Phone,
    /// 📐 The tablet lying down: the bars of the PC, by the finger.
    TabletWide,
    /// 📐 The tablet held upright: the bars of the PC, by the finger, and the panel of the right as a drawer.
    TabletTall,
}

impl Form {
    /// The two forms every screen of the app has a scene on ([`universe`]).
    pub(super) const ALL: [Form; 2] = [Form::Pc, Form::Phone];
    /// 📐 The tablet: the screens of the PC it lays out, or answers, its own way ([`scenes`]).
    pub(super) const TABLET: [Form; 2] = [Form::TabletWide, Form::TabletTall];

    /// The screen the app runs on, as its window or its page tells it (`App::update_form`): the size in px of the
    /// page, the density, whether a finger uses it.
    /// - The PC: a window of 1440 × 900, with a mouse.
    /// - The phone: Jee's, an iPhone 17 Pro (402 × 874, ×3) held upright with the app installed. The page gives the
    ///   app what is between the safe areas of that phone (62 px above, 34 px below): 402 × 778.
    /// - The tablet: Jee's, an iPad Pro 13 (1366 × 1024, ×2) in its browser, whose bars take 86 px above the page
    ///   (measured on a capture of his) and the safe area 20 px below: 1366 × 918 lying down, 1024 × 1260 upright.
    fn screen(self) -> Screen {
        let (w, h, scale, coarse) = match self {
            Form::Pc => (1440.0, 900.0, 1.0, false),
            Form::Phone => (402.0, 778.0, 3.0, true),
            Form::TabletWide => (1366.0, 918.0, 2.0, true),
            Form::TabletTall => (1024.0, 1260.0, 2.0, true),
        };
        Screen {
            w,
            h,
            scale,
            coarse,
            ..Default::default()
        }
    }

    /// What the app chooses on that screen: the bars of a phone or those of a PC, the size of what a finger presses,
    /// its zoom.
    fn choice(self) -> Choice {
        choose(&self.screen())
    }

    /// Its size in points, what the app lays itself out on: the px of the page divided by the zoom of the form
    /// (`render::ui_size`). 📱 The phone's zoom is the compact factor 0.8 (`formfactor::COMPACTE_CELL`): its 402 px
    /// are 502 points, so 502 × 972. (No real phone lays the app out on less than 400 points.) 📐 The tablet's zoom is
    /// the finger's 1.25: 1092 × 734 lying down (wider than 1024 px: the PC's form), 819 × 1008 upright.
    fn size(self) -> (u32, u32) {
        let (screen, zoom) = (self.screen(), self.choice().zoom);
        ((screen.w / zoom) as u32, (screen.h / zoom) as u32)
    }

    /// The first part of the names of its scenes.
    pub(super) fn tag(self) -> &'static str {
        match self {
            Form::Pc => "pc",
            Form::Phone => "cell",
            Form::TabletWide => "tablet-wide",
            Form::TabletTall => "tablet-tall",
        }
    }
}

/// What opens a screen on the app at rest.
pub(super) type Open = std::sync::Arc<dyn Fn(&mut App) + Send + Sync>;
/// What can be done next on a drawn screen: a name (« Pick(3) ») and the gesture.
pub(super) type Next = Vec<(String, Open)>;

/// One screen of the app, as the test opens it.
pub(super) struct Scene {
    /// « pc/tool/Trading »: the form, the family, the screen.
    pub(super) name: String,
    pub(super) form: Form,
    pub(super) open: Open,
    /// What must be on screen once drawn (ids of [`universe::shown`]).
    pub(super) shows: Vec<String>,
    /// Read on the drawn screen, what is done next (each row of the open menu clicked, each tile touched…): one more
    /// scene for each, named after this one (« pc/list/Styles/Pick(3) »).
    pub(super) follow: Option<fn(&mut App) -> Next>,
    /// For a scene that follows another one: its gesture, done once that one's screen was drawn whole this many times
    /// (as many as it took that screen to come to rest).
    pub(super) then: Option<(usize, Open)>,
    /// Without the data ([`furniture`]): the app of a first launch before anything arrived (« Chargement… »).
    pub(super) empty: bool,
}

/// Side of a tile of the grid (points): the tiles say where an image changed.
const TILE: u32 = 120;
/// A scene still changing after this many images is not at rest.
const MAX_FRAMES: usize = 8;
/// New images written by each copy of the test program for the scenes that are not at rest or not stable, at most.
const MAX_IMAGES: usize = 6;
/// New images written by each copy of the test program for the screens that changed, at most: one for each kind of
/// change it meets (the same zones and the same tiles of a form), not one for each screen.
const MAX_KINDS: usize = 12;
/// What the parent test gives a copy of the test program: the file of its results (in the folder where the copies say
/// which scenes they took).
const SHARE_OUT: &str = "ARGOS_SAFETY_NET_OUT";
/// A look at a part of the screens: only the scenes whose name contains this text are drawn (`ARGOS_SCENES=list/`).
/// The whole name of a screen that follows a scene (« pc/alerts/active/Onglet1 », as the messages write it) draws
/// that screen alone: its scene is opened to read what follows it, and says nothing itself.
const ONLY: &str = "ARGOS_SCENES";
/// The keyring service of a copy of the test program (`ARGOS_SERVICE_CLES`): never the app's.
const KEYRING: &str = "argos-hss-trading-safety-net";
/// Where the app is known to keep, while only the mouse moves, an image that is not the one it draws whole: what it
/// does, and the state of the app it happens in. The net found it and does not fix it: a scene in that state says so
/// instead of failing. The day no scene shows it any more, the test asks for the line to go.
const KEPT_APART: [(&str, fn(&App) -> bool); 1] = [(
    "a banner on a split screen goes under the legend of the last cell drawn, not under the one of its own cell",
    |a| a.banner.is_some() && argos_render::splits::cells(&a.split.layout).len() > 1,
)];

/// Where the app is known never to come to rest, each image different from the one before: what it does, and the
/// state of the app it happens in. As for [`KEPT_APART`]: a scene in that state says so instead of failing, and the
/// day no scene shows it any more the test asks for the line to go.
/// (n° 19, the column of modules at the left that showed at one image and not at the next while a panel was open and
/// the room was short: fixed, `App::placer` no longer counts the column twice; its line went.)
const MOVING_APART: [(&str, fn(&App) -> bool); 0] = [];

/// Where the app is known to stop (a panic: the page of the web app freezes, the desktop app closes): what it does,
/// the file of the code that stops, and how what it says starts. The net found it and does not fix it: a scene that
/// stops there says so instead of failing, and has no image to compare. As for [`KEPT_APART`]: the day no scene
/// stops there any more, the test asks for the line to go. (None today: the one of the navigation pill of a narrow
/// cell of a split screen was fixed by paires-btc-1.)
const STOPS_APART: [(&str, &str, &str); 0] = [];

/// The fingerprints of a scene: what the reference file keeps.
#[derive(Debug, Clone, Default, PartialEq)]
struct Print {
    size: (u32, u32),
    /// The whole image: what the comparison rests on.
    image: u64,
    /// Name, place (x, y, width, height) and fingerprint of each zone; the last one: what no zone covers. They say
    /// where an image changed, in words.
    zones: Vec<(String, [u32; 4], u32)>,
    /// Fingerprint of each tile, row by row: they frame what changed on the new image.
    tiles: Vec<u16>,
}

/// A scene drawn by a copy of the test program: its fingerprints, what was on screen, what is wrong with it.
#[derive(Debug, Clone, Default)]
struct Entry {
    print: Print,
    shown: Vec<String>,
    problems: Vec<String>,
    /// What the app is known to do there ([`KEPT_APART`], [`MOVING_APART`]).
    known: Vec<String>,
    /// Images drawn and the time they took (ms).
    frames: usize,
    ms: f64,
    /// Where the rest of the scene's time went (ms): the app made at rest, the screen opened, what can be done next
    /// read on it; and the whole scene (drawn a second time when it was).
    spent: [f64; 4],
}

/// FNV-1a (64 bits) over the red, green and blue of the pixels: the same image gives the same number everywhere.
fn hash(pixels: impl Iterator<Item = u32>) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for p in pixels {
        for b in [p >> 16, p >> 8, p] {
            h = (h ^ (b & 0xff) as u64).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    h
}

/// A scene's name is also a file name: letters, digits, « _ », « - », « . » and « / » only.
pub(super) fn plain(name: &str) -> String {
    name.chars().filter(|c| c.is_alphanumeric() || "_-./".contains(*c)).collect()
}

impl Print {
    fn of(image: &[u32], (w, h): (u32, u32), zones: &[(String, Rect)]) -> Print {
        let inside = |x0: u32, y0: u32, x1: u32, y1: u32| (y0..y1).flat_map(move |y| (x0..x1).map(move |x| (y * w + x) as usize));
        let mut covered = vec![false; image.len()];
        let mut names: BTreeMap<&str, usize> = BTreeMap::new();
        let mut out = vec![];
        for (name, r) in zones {
            let (x0, y0) = ((r.x.max(0.0) as u32).min(w), (r.y.max(0.0) as u32).min(h));
            let (x1, y1) = (((r.x + r.w).max(0.0) as u32).clamp(x0, w), ((r.y + r.h).max(0.0) as u32).clamp(y0, h));
            for i in inside(x0, y0, x1, y1) {
                covered[i] = true;
            }
            // (two zones of the same name, as two panels of the same indicator: the second one is numbered)
            let n = names.entry(name.as_str()).or_default();
            *n += 1;
            let name = if *n > 1 { format!("{name} ({n})") } else { name.clone() };
            let print = hash(inside(x0, y0, x1, y1).map(|i| image[i]));
            // (« | » and « = » are the reference file's own signs)
            out.push((
                name.replace('|', "/").replace('=', ":"),
                [x0, y0, x1 - x0, y1 - y0],
                (print ^ (print >> 32)) as u32,
            ));
        }
        let rest = hash(image.iter().zip(&covered).filter(|(_, c)| !**c).map(|(p, _)| *p));
        out.push(("outside the zones".into(), [0, 0, 0, 0], (rest ^ (rest >> 32)) as u32));
        let tiles = (0..h.div_ceil(TILE))
            .flat_map(|r| (0..w.div_ceil(TILE)).map(move |c| (c, r)))
            .map(|(c, r)| {
                let t = hash(inside(c * TILE, r * TILE, ((c + 1) * TILE).min(w), ((r + 1) * TILE).min(h)).map(|i| image[i]));
                (t ^ (t >> 16) ^ (t >> 32) ^ (t >> 48)) as u16
            })
            .collect();
        Print {
            size: (w, h),
            image: hash(image.iter().copied()),
            zones: out,
            tiles,
        }
    }

    /// The names and places of the zones, as the reference file writes them once for all the scenes laid out alike.
    fn layout(&self) -> String {
        let zone = |(name, [x, y, w, h], _): &(String, [u32; 4], u32)| format!("{name}={x},{y},{w},{h}");
        self.zones.iter().map(zone).collect::<Vec<_>>().join(" | ")
    }

    /// What differs from the reference `r`: the zones (in words) and the tiles (their index).
    fn differences(&self, r: &Print) -> (Vec<String>, Vec<usize>) {
        if self.size != r.size {
            return (vec![format!("the window was {} × {}", r.size.0, r.size.1)], vec![]);
        }
        let mut zones = vec![];
        for (name, place, print) in &self.zones {
            match r.zones.iter().find(|(n, _, _)| n == name) {
                None => zones.push(format!("« {name} » is new")),
                Some((_, p, _)) if p != place => zones.push(format!("« {name} » moved from {p:?} to {place:?}")),
                Some((_, _, f)) if f != print => zones.push(format!("« {name} »")),
                Some(_) => {}
            }
        }
        for (name, _, _) in r.zones.iter().filter(|(n, _, _)| !self.zones.iter().any(|(m, _, _)| m == n)) {
            zones.push(format!("« {name} » is gone"));
        }
        let tiles = (0..self.tiles.len()).filter(|&k| r.tiles.get(k) != Some(&self.tiles[k])).collect();
        (zones, tiles)
    }
}

/// The short name of a layout of zones in the reference file (from its text: it never changes with the other lines).
fn layout_id(layout: &str) -> String {
    let h = hash(layout.bytes().map(u32::from));
    format!("{:08x}", (h ^ (h >> 32)) as u32)
}

/// The text of scenes. First the layouts of zones, `% id name=x,y,width,height | …`; then for each scene
/// `@ name width×height image`, `. layout zone zone…` (the fingerprint of each zone of the layout) and `: tiles`; and
/// from a copy of the test program only: `+ what is on screen`, `~ images and time`, `! a problem`.
fn write(entries: &BTreeMap<String, Entry>, reference: bool) -> String {
    let mut t = String::new();
    if reference {
        t.push_str("# Argos HSS Trading: the fingerprints of every screen of the app (safety net of the split into modules).\n");
        t.push_str("# Written by the test `safety_net::every_screen_matches_its_reference` with ARGOS_REFERENCES=maj, never by hand.\n");
    }
    let layouts: BTreeMap<String, String> = entries.values().map(|e| e.print.layout()).map(|l| (layout_id(&l), l)).collect();
    for (id, layout) in &layouts {
        let _ = writeln!(t, "% {id} {layout}");
    }
    for (name, e) in entries {
        let p = &e.print;
        let _ = writeln!(t, "@ {name} {}x{} {:016x}", p.size.0, p.size.1, p.image);
        let zones: String = p.zones.iter().map(|(_, _, print)| format!(" {print:08x}")).collect();
        let _ = writeln!(t, ". {}{zones}", layout_id(&p.layout()));
        let tiles: String = p.tiles.iter().map(|v| format!("{v:04x}")).collect();
        let _ = writeln!(t, ": {tiles}");
        if !reference {
            let _ = writeln!(t, "+ {}", e.shown.join(" "));
            let [made, opened, read, whole] = e.spent;
            let _ = writeln!(t, "~ {} {} {made} {opened} {read} {whole}", e.frames, e.ms);
            for problem in &e.problems {
                let _ = writeln!(t, "! {}", problem.replace('\n', " "));
            }
            for known in &e.known {
                let _ = writeln!(t, "? {known}");
            }
        }
    }
    t
}

/// Reads back what [`write`] wrote.
fn parse(text: &str) -> BTreeMap<String, Entry> {
    let hex = |v: &str| u64::from_str_radix(v, 16).expect("a fingerprint");
    let mut layouts: BTreeMap<&str, Vec<(String, [u32; 4])>> = BTreeMap::new();
    let mut all: BTreeMap<String, Entry> = BTreeMap::new();
    let mut name = String::new();
    for line in text.lines().filter(|l| !l.starts_with('#') && !l.is_empty()) {
        let (kind, rest) = line.split_at(1);
        let rest = rest.trim_start();
        match kind {
            "%" => {
                let (id, zones) = rest.split_once(' ').unwrap_or((rest, ""));
                let zone = |z: &str| {
                    let (zone, place) = z.rsplit_once('=').expect("a zone and its place");
                    let place: Vec<u32> = place.split(',').map(|v| v.parse().expect("a place")).collect();
                    (zone.to_string(), place.try_into().expect("x, y, width, height"))
                };
                // (a scene that stopped has no zone)
                layouts.insert(id, zones.split(" | ").filter(|z| !z.is_empty()).map(zone).collect());
                continue;
            }
            "@" => {
                let mut parts = rest.rsplitn(3, ' ');
                let (image, size, n) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""), parts.next().expect("a scene"));
                let (w, h) = size.split_once('x').expect("width x height");
                name = n.to_string();
                let e = all.entry(name.clone()).or_default();
                (e.print.size, e.print.image) = ((w.parse().expect("a width"), h.parse().expect("a height")), hex(image));
                continue;
            }
            _ => {}
        }
        let e = all.get_mut(&name).expect("a scene before its lines");
        match kind {
            "." => {
                let mut prints = rest.split(' ');
                let layout = &layouts[prints.next().expect("a layout")];
                e.print.zones = layout
                    .iter()
                    .zip(prints)
                    .map(|((zone, place), print)| (zone.clone(), *place, hex(print) as u32))
                    .collect();
            }
            ":" => {
                e.print.tiles = rest
                    .as_bytes()
                    .chunks(4)
                    .map(|c| hex(std::str::from_utf8(c).expect("hexadecimal")) as u16)
                    .collect();
            }
            "+" => e.shown = rest.split(' ').filter(|s| !s.is_empty()).map(str::to_string).collect(),
            "~" => {
                let mut numbers = rest.split(' ');
                e.frames = numbers.next().and_then(|v| v.parse().ok()).expect("images");
                let times: Vec<f64> = numbers.map(|v| v.parse().expect("a time")).collect();
                let [ms, made, opened, read, whole] = times[..] else {
                    panic!("the times of a scene: {line}")
                };
                (e.ms, e.spent) = (ms, [made, opened, read, whole]);
            }
            "!" => e.problems.push(rest.to_string()),
            "?" => e.known.push(rest.to_string()),
            _ => panic!("a line of an unknown kind: {line}"),
        }
    }
    all
}

fn references_folder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("references")
}

fn reference_path() -> PathBuf {
    references_folder().join("safety-net-screens.txt")
}

/// The references are being written again (`ARGOS_REFERENCES=maj`), not compared.
fn updating() -> bool {
    std::env::var("ARGOS_REFERENCES").is_ok_and(|v| v == "maj")
}

/// The text the names of the scenes to draw contain, when only a part of the screens is looked at ([`ONLY`]).
fn only() -> Option<String> {
    std::env::var(ONLY).ok().filter(|text| !text.is_empty())
}

/// The image as a PNG file, the tiles `framed` outlined in red.
fn save_png(path: &Path, image: &[u32], (w, h): (u32, u32), framed: &[usize]) {
    let mut image = image.to_vec();
    let cols = w.div_ceil(TILE) as usize;
    for &k in framed {
        let (x0, y0) = ((k % cols) as u32 * TILE, (k / cols) as u32 * TILE);
        let (x1, y1) = ((x0 + TILE).min(w), (y0 + TILE).min(h));
        for (x, y) in (y0..y1).flat_map(|y| (x0..x1).map(move |x| (x, y))) {
            if x < x0 + 2 || x + 2 >= x1 || y < y0 + 2 || y + 2 >= y1 {
                image[(y * w + x) as usize] = 0xff_20_20;
            }
        }
    }
    let mut pm = tiny_skia::Pixmap::new(w, h).expect("an image of the window's size");
    for (p, v) in pm.pixels_mut().iter_mut().zip(&image) {
        *p = tiny_skia::ColorU8::from_rgba((v >> 16) as u8, (v >> 8) as u8, *v as u8, 255).premultiply();
    }
    pm.save_png(path).expect("the image is written");
}

/// Paints this part of the image in one flat colour: what was drawn there is not compared.
fn blank(image: &mut [u32], (w, h): (u32, u32), part: Rect) {
    let (x0, y0) = ((part.x.max(0.0) as u32).min(w), (part.y.max(0.0) as u32).min(h));
    let (x1, y1) = (
        ((part.x + part.w).ceil().max(0.0) as u32).clamp(x0, w),
        ((part.y + part.h).ceil().max(0.0) as u32).clamp(y0, h),
    );
    for (x, y) in (y0..y1).flat_map(|y| (x0..x1).map(move |x| (x, y))) {
        image[(y * w + x) as usize] = 0xff_00_ff;
    }
}

/// What drawing a scene gave.
struct Drawn {
    image: Vec<u32>,
    zones: Vec<(String, Rect)>,
    shown: BTreeSet<String>,
    /// Images drawn after the opening, the time they took (ms), and the image before the last one drawn whole when
    /// the two were not the same: the screen was still moving.
    frames: usize,
    ms: f64,
    moving: Option<Vec<u32>>,
    /// The image drawn after them from the one the app keeps, when it is not the same again; and what the app is
    /// known to do in that state, if it is one of [`MOVING_APART`] (a screen still moving) or of [`KEPT_APART`].
    kept: Option<Vec<u32>>,
    apart: Option<&'static str>,
    /// Images drawn whole before the screen stopped changing (the one after them only said so).
    settled: usize,
    /// The scenes that follow this one ([`Scene::follow`]).
    next: Next,
    /// The time it took to make the app at rest, to open the screen and to read what follows it (ms).
    spent: [f64; 3],
}

/// 200 candles of 30 min that go up, down and sideways, with wicks and uneven volumes: always the same ones (whole
/// numbers and halves only, so every machine computes the same prices), between 110 and 290 like the tests' candles.
pub(super) fn candles() -> Vec<Bar> {
    let mut seed: u64 = 0x5eed_0a26_0522;
    let mut next = move |below: u64| {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        ((seed >> 33) % below) as f64
    };
    let mut close = 200.0;
    (0..200)
        .map(|i| {
            // (a trend of 25 candles at a time: up, down, flat…)
            let drift = [1.5, -1.0, 0.5, -2.0, 1.0, 0.0, -0.5, 2.0][(i / 25) as usize % 8];
            let open = close;
            close = (open + (next(17) - 8.0) * 0.5 + drift).clamp(110.0, 290.0);
            let (high, low) = (open.max(close) + next(7) * 0.5, open.min(close) - next(7) * 0.5);
            let volume = 5.0 + next(40) + if next(11) == 0.0 { 120.0 } else { 0.0 };
            Bar {
                time: START + i * 1800,
                open,
                high,
                low,
                close,
                volume,
            }
        })
        .collect()
}

/// The app at rest in this window, opened on a file of candles (as `app()` of the other tests): the default settings,
/// the welcome pages already seen. Unless `empty`, it is an app in use: on a market of an exchange, its data arrived
/// ([`furniture`]).
fn at_rest(form: Form, empty: bool) -> App {
    let iv = Interval(1800);
    let mut a = App::blank(Source::File("essai.json".into()), iv, iv, vec![], "essai.json · 30m · fichier".into());
    a.chart.raw = candles();
    if !empty {
        furniture::in_use(&mut a);
    }
    a.rebuild();
    a.start_free();
    a.appliquer_cadre(&crate::actions::Interface2::default());
    a.top.prefs.bienvenue_vue = true;
    a.ui.form = form.choice();
    // (📐 a tablet held upright: its panel is a drawer, folded when the app first reads its screen, as
    // `App::update_form` does once it has a window)
    if a.ui.form.drawer() {
        a.ui.drawer_was_open = a.watch.prefs.open;
        a.watch.prefs.open = false;
    }
    let (w, h) = form.size();
    a.ui.taille_test = Some(winit::dpi::PhysicalSize::new(w, h));
    // (an app past its first images: the phone draws its indicators from the third one on)
    a.ui.frames = 5;
    if !empty {
        furniture::arrive(&mut a);
    }
    a
}

/// An app at rest in its own thread, with nothing left from the scene before: the frozen clock (the tests' candles
/// end there), an empty settings folder, and one image drawn (the screen is laid out, as before a first click).
fn fresh<T: Send>(form: Form, empty: bool, then: impl FnOnce(&mut App) -> T + Send) -> Result<T, String> {
    stops_located();
    let (done, thread) = std::thread::scope(|s| {
        let scene = s.spawn(|| {
            crate::types::NOW_FIGE.with(|n| n.set(Some(START + 200 * 1800 + 600)));
            dossier_a_soi("safety-net");
            // (what a test program keeps for all its tests is emptied too: the cache of candles, the order flow, the
            // tickets of the Support, the downloads; and the CoinMarketCap key noted for its background readings)
            let folders = ["retours", "telechargements"].map(|name| Some(argos_core::dossiers::de_test(name)));
            for kept in folders
                .into_iter()
                .chain([cache_dir(), crate::actions::orderflow::store().map(|s| s.dir)])
                .flatten()
            {
                let _ = std::fs::remove_dir_all(kept);
            }
            argos_data::cmcap::regler(None);
            // (and the exchanges relay a page found: a page without it)
            argos_data::relais::activer(false);
            let mut a = at_rest(form, empty);
            a.draw();
            then(&mut a)
        });
        let thread = scene.thread().id();
        (scene.join(), thread)
    });
    done.map_err(|p| {
        let said = p
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_else(|| "a panic without a message".into());
        // (with the line of the code that stopped: the app's own, or the scene's)
        let mut stops = STOPS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        match stops.iter().position(|(of, _)| *of == thread).map(|k| stops.swap_remove(k).1) {
            Some(place) => format!("{said} ({place})"),
            None => said,
        }
    })
}

/// Where each thread that stopped did so (file and line), until its scene reads it.
static STOPS: std::sync::Mutex<Vec<(std::thread::ThreadId, String)>> = std::sync::Mutex::new(Vec::new());

/// In the trace of a panic, the first line of the project's own code after the hook that noted it:
/// « crates/argos-render/src/navbar.rs:152 ». (The trace names the files of this crate from its folder, « ./src/… »,
/// and those of the others in full.)
fn ours(trace: &str) -> Option<String> {
    let mut function = "";
    trace.lines().map(str::trim).find_map(|line| {
        let Some(at) = line.strip_prefix("at ") else {
            function = line;
            return None;
        };
        let at = at.rsplit_once(':').map_or(at, |(line, _column)| line);
        let file = match (at.find("crates/argos-"), at.strip_prefix("./src/")) {
            (Some(from), _) => at[from..].to_string(),
            (None, Some(inside)) => format!("crates/argos-desktop/src/{inside}"),
            (None, None) => return None,
        };
        (!function.contains("stops_located")).then_some(file)
    })
}

/// From now on a panic notes its place in [`STOPS`], then goes on as before (the message the test program prints):
/// the scene that stopped tells which line of the code did it.
fn stops_located() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let before = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic| {
            if let Some(place) = panic.location() {
                let mut place = format!("{}:{}", place.file(), place.line());
                // (stopped inside the language's own code, `clamp` or an index: the line of the project that led
                // there, read in the trace)
                if !place.starts_with("crates/") {
                    place = ours(&std::backtrace::Backtrace::force_capture().to_string()).unwrap_or(place);
                }
                let mut stops = STOPS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                stops.retain(|(of, _)| *of != std::thread::current().id());
                stops.push((std::thread::current().id(), place));
                // (a thread that no scene reads leaves its place there: the list stays short)
                let extra = stops.len().saturating_sub(64);
                stops.drain(..extra);
            }
            before(panic);
        }));
    });
}

/// A banner of ☰ Appli that tells what was built on this computer, made the same on every computer. « Infos » names
/// the version and the commit of the build (`ARGOS_BUILD`, `ARGOS_COMMIT` when the program was compiled): it says what
/// a build without them says. « Mises à jour » reads `dist/app/version.json` around the program and the folder it is
/// run from (written at each build of the web app): its three answers become one, that is not a text of the app.
fn built_here(banner: &str) -> String {
    const UPDATES: [&str; 3] = ["⬆️ Nouvelle version locale disponible", "✅ Argos est à jour", "✅ Version "];
    if UPDATES.iter().any(|said| banner.starts_with(said)) {
        return "✅ (the version built on this computer)".into();
    }
    // (read as `actions::appmenu` reads them: the tests are compiled with the app)
    let (build, commit) = (option_env!("ARGOS_BUILD").unwrap_or("dev"), option_env!("ARGOS_COMMIT").unwrap_or("?"));
    banner.replace(&format!("version {build} · commit {commit}"), "version dev · commit ?")
}

/// Draws a scene from scratch: on the app at rest the screen is opened, the gesture of a scene that follows another
/// one is done, then the app draws itself (`App::draw`, as for a real window) until nothing moves. With `followed`,
/// what can be done next on the drawn screen is read ([`Scene::follow`]).
fn draw(scene: &Scene, followed: bool) -> Result<Drawn, String> {
    let ms = |since: web_time::Instant| since.elapsed().as_secs_f64() * 1000.0;
    let start = web_time::Instant::now();
    fresh(scene.form, scene.empty, |a| {
        let made = ms(start);
        let opening = web_time::Instant::now();
        (scene.open)(a);
        if let Some((frames, gesture)) = &scene.then {
            for _ in 0..*frames {
                a.ui.fond = None;
                a.draw();
            }
            gesture(a);
        }
        let opened = ms(opening);
        // what leaves by itself stays (a banner after 8 s, the full-screen bubble after 2 s): a slow computer draws
        // the same image as a fast one; and a banner that names the folder of the downloads (another one on each
        // computer and at each run of the tests) says « Téléchargements », one that tells what was built on this
        // computer says the same on every computer ([`built_here`])
        let later = web_time::Instant::now() + std::time::Duration::from_secs(3600);
        let downloads = crate::platform::dossier_telechargements().display().to_string();
        if let Some(banner) = a.banner.as_mut() {
            *banner = built_here(&banner.replace(&downloads, "Téléchargements"));
            a.ui.banner_since = Some((banner.clone(), later));
        }
        if let Some((bubble, _)) = a.ui.bulle_plein {
            a.ui.bulle_plein = Some((bubble, later));
        }
        let t0 = web_time::Instant::now();
        let (mut image, mut before, mut frames, mut rest) = (vec![], vec![], 0, false);
        while frames < MAX_FRAMES && !rest {
            // the chart is drawn whole each time, as the app does at each second of its clock: the image it keeps
            // while only the mouse moves (`fond.rs`) would stop a scene one drawing early (a price label that keeps
            // clear of what the drawing before it laid)
            a.ui.fond = None;
            a.draw();
            frames += 1;
            let new = std::mem::take(&mut a.ui.image_test);
            rest = new == image;
            before = std::mem::replace(&mut image, new);
        }
        let settled = frames.saturating_sub(1).max(1);
        // then once with the image the app keeps, as when only the mouse moves: the same image
        a.draw();
        frames += 1;
        let mut kept = Some(std::mem::take(&mut a.ui.image_test)).filter(|kept| *kept != image);
        let images = ms(t0);
        let mut moving = (!rest).then_some(before);
        let known: &[(&str, fn(&App) -> bool)] = match (&moving, &kept) {
            (Some(_), _) => &MOVING_APART,
            (None, Some(_)) => &KEPT_APART,
            (None, None) => &[],
        };
        let apart = known.iter().find(|(_, state)| state(a)).map(|(what, _)| *what);
        // (what shows the hour it really is is not compared: one flat colour in its place)
        if let Some(clock) = universe::real_clock(a) {
            for shown in [Some(&mut image), kept.as_mut(), moving.as_mut()].into_iter().flatten() {
                blank(shown, scene.form.size(), clock);
            }
        }
        // (the phone's screen is three bands: the top, the chart, the bottom; the tablet has the bars of the PC)
        let zones = if a.top.phone {
            let (w, h) = (scene.form.size().0 as f64, scene.form.size().1 as f64);
            let (top, chart) = (a.top_h(), a.area.1 as f64);
            vec![
                ("top".into(), Rect::new(0.0, 0.0, w, top)),
                ("chart".into(), Rect::new(0.0, top, w, chart)),
                ("bottom".into(), Rect::new(0.0, top + chart, w, h - top - chart)),
            ]
        } else {
            a.zones_pc().into_iter().map(|(n, _, r)| (n, r)).collect()
        };
        // (with what is on screen, the kinds of messages the app received: `universe::ANSWERS`)
        let mut shown = universe::shown(a);
        shown.extend(furniture::arrived());
        let reading = web_time::Instant::now();
        let next = scene.follow.filter(|_| followed).map_or(vec![], |follow| follow(a));
        // (after 180 slow images in a row the app thins its glass by itself, and after 300 fast ones it thickens it,
        // `App::frame_timing`: a scene that drew that many would give other screens on a slower computer)
        assert!(
            a.ui.frames < 150,
            "{} images drawn by one scene: past 180 in a row the app changes its glass by itself",
            a.ui.frames
        );
        Drawn {
            image,
            zones,
            shown,
            frames,
            ms: images,
            moving,
            kept,
            apart,
            settled,
            next,
            spent: [made, opened, ms(reading)],
        }
    })
}

/// A kind of change of the screens: the size of the window, the zones that changed (in words) and the tiles that
/// changed (their index). Two screens that changed alike have the same one.
type Kind = ((u32, u32), Vec<String>, Vec<usize>);

/// A copy of the test program at work: the references, what it found so far, the new images it wrote and the kinds
/// of change it wrote one for; `silent` while it draws a scene only to reach the one screen asked for ([`ONLY`]): no
/// image of that scene is written.
struct Share {
    reference: BTreeMap<String, Entry>,
    captures: Option<PathBuf>,
    entries: BTreeMap<String, Entry>,
    images: usize,
    kinds: BTreeSet<Kind>,
    silent: bool,
}

impl Share {
    /// Draws a scene, compares it with its reference and keeps what it found. Gives the scenes that follow it: the
    /// same screen opened and drawn until it stopped changing, then one gesture read on it.
    fn scene(&mut self, scene: &Scene) -> Vec<Scene> {
        let mut e = Entry::default();
        let mut after = vec![];
        let file = format!("safety-net-{}", scene.name.replace('/', "-"));
        let start = web_time::Instant::now();
        match draw(scene, true) {
            // (stopped where the app is known to: said, not a failure)
            Err(panic) => match STOPS_APART
                .iter()
                .find(|(_, file, said)| panic.starts_with(said) && panic.contains(&format!("{file}:")))
            {
                Some((what, ..)) => e.known.push(what.to_string()),
                None => e.problems.push(format!("it stopped: {panic}")),
            },
            Ok(d) => {
                let size = scene.form.size();
                e.print = Print::of(&d.image, size, &d.zones);
                (e.frames, e.ms) = (d.frames, d.ms);
                e.spent[..3].copy_from_slice(&d.spent);
                if let Some(what) = d.apart {
                    e.known.push(what.to_string());
                } else if let Some(before) = &d.moving {
                    let (zones, tiles) = Print::of(before, size, &d.zones).differences(&e.print);
                    e.problems.push(format!("still moving after {MAX_FRAMES} images, in {}", zones.join(", ")));
                    // (the last two images are written, to see what moves)
                    if !self.silent && self.images < MAX_IMAGES {
                        self.images += 1;
                        save_png(&references_folder().join(format!("{file}-before.nouveau.png")), before, size, &tiles);
                        save_png(&references_folder().join(format!("{file}-last.nouveau.png")), &d.image, size, &tiles);
                    }
                } else if let Some(kept) = &d.kept {
                    let (zones, tiles) = Print::of(kept, size, &d.zones).differences(&e.print);
                    e.problems.push(format!(
                        "the image the app keeps while only the mouse moves is not the one it draws whole, in {}",
                        zones.join(", ")
                    ));
                    // (both images are written, to see what the kept one misses)
                    if !self.silent && self.images < MAX_IMAGES {
                        self.images += 1;
                        save_png(&references_folder().join(format!("{file}-whole.nouveau.png")), &d.image, size, &tiles);
                        save_png(&references_folder().join(format!("{file}-kept.nouveau.png")), kept, size, &tiles);
                    }
                }
                for id in scene.shows.iter().filter(|id| !d.shown.contains(*id)) {
                    e.problems.push(format!("does not show {id}"));
                }
                let known = self.reference.get(&scene.name).map(|r| &r.print);
                let differs = known.is_none_or(|r| *r != e.print);
                let changed = known.filter(|_| differs && !updating());
                // drawn a second time from scratch (a new scene, or one that is not its reference any more): the same
                // scene gives the same image
                if differs {
                    match draw(scene, false) {
                        Ok(again) if again.image == d.image => {}
                        Ok(again) => {
                            let (zones, tiles) = Print::of(&again.image, size, &again.zones).differences(&e.print);
                            e.problems.push(format!("not stable, drawn twice it differs in {}", zones.join(", ")));
                            // (both images are written, to see what moved)
                            if !self.silent && self.images < MAX_IMAGES {
                                self.images += 1;
                                save_png(&references_folder().join(format!("{file}-first.nouveau.png")), &d.image, size, &tiles);
                                save_png(&references_folder().join(format!("{file}-second.nouveau.png")), &again.image, size, &tiles);
                            }
                        }
                        Err(panic) => e.problems.push(format!("drawn a second time, it stopped: {panic}")),
                    }
                }
                if let Some(r) = changed {
                    let (zones, tiles) = e.print.differences(r);
                    e.problems.push(format!("changed in {}", zones.join(", ")));
                    // (one image for each kind of change: a button every screen shows changes thousands of them alike)
                    if !self.silent && self.kinds.len() < MAX_KINDS && self.kinds.insert((e.print.size, zones, tiles.clone())) {
                        save_png(&references_folder().join(format!("{file}.nouveau.png")), &d.image, size, &tiles);
                    }
                }
                if let Some(dir) = self.captures.as_ref().filter(|_| !self.silent) {
                    save_png(&dir.join(format!("{file}.png")), &d.image, size, &[]);
                }
                e.shown = d.shown.into_iter().collect();
                let mut names = BTreeSet::new();
                for (what, then) in d.next {
                    // (two gestures of the same name: the second one is numbered)
                    let name = (1..)
                        .map(|k| if k == 1 { plain(&what) } else { format!("{}.{k}", plain(&what)) })
                        .find(|n| names.insert(n.clone()))
                        .unwrap_or_default();
                    after.push(Scene {
                        name: format!("{}/{name}", scene.name),
                        form: scene.form,
                        open: scene.open.clone(),
                        shows: vec![],
                        follow: None,
                        then: Some((d.settled, then)),
                        empty: scene.empty,
                    });
                }
            }
        }
        e.spent[3] = start.elapsed().as_secs_f64() * 1000.0;
        self.entries.insert(scene.name.clone(), e);
        after
    }
}

/// A copy of the test program: draws the scenes it is the first to take and the scenes that follow them, compares each
/// with its reference and writes what it found in `out`. The copies share the list as they go (a file made in the
/// folder of `out` says a scene is taken): a copy on a long scene leaves the rest of the list to the others. The
/// scenes followed by others, the long ones, are taken first: the copies end together, on short scenes.
fn draw_share(out: &Path) {
    on_its_own();
    assert_eq!(
        std::env::var("ARGOS_SERVICE_CLES").as_deref(),
        Ok(KEYRING),
        "a copy never uses the app's keyring"
    );
    let mut share = Share {
        reference: std::fs::read_to_string(reference_path()).map(|t| parse(&t)).unwrap_or_default(),
        captures: std::env::var("ARGOS_CAPTURES").ok().map(PathBuf::from).inspect(|folder| {
            std::fs::create_dir_all(folder).expect("the folder of the captures");
        }),
        entries: BTreeMap::new(),
        images: 0,
        kinds: BTreeSet::new(),
        silent: false,
    };
    let only = only();
    let folder = out.parent().expect("the folder of the shares");
    let take = |k: usize| {
        let mine = folder.join(format!("{k}.taken"));
        std::fs::OpenOptions::new().write(true).create_new(true).open(mine).is_ok()
    };
    let all = scenes::all();
    let named = |name: &str| only.as_ref().is_none_or(|text| name.contains(text.as_str()));
    // (the text names a screen that follows this scene: the scene is opened for it, and only for it)
    let leads_to = |scene: &Scene| !named(&scene.name) && only.as_ref().is_some_and(|text| text.starts_with(&format!("{}/", scene.name)));
    let asked = |scene: &&(usize, &Scene)| named(&scene.1.name) || (scene.1.follow.is_some() && leads_to(scene.1));
    let (long, short): (Vec<_>, Vec<_>) = all.iter().enumerate().partition(|(_, scene)| scene.follow.is_some());
    for (k, scene) in long.iter().chain(&short).filter(asked) {
        if take(*k) {
            share.silent = leads_to(scene);
            let follow = share.scene(scene);
            if std::mem::take(&mut share.silent) {
                share.entries.remove(&scene.name);
                for next in follow.iter().filter(|next| named(&next.name)) {
                    share.scene(next);
                }
            } else {
                for next in follow {
                    share.scene(&next);
                }
            }
        }
    }
    // (the images of the emoji folder this copy read: they are the computer's, not the app's, and an image read
    // stays for every scene after it. A computer whose folder emoji/ lacks one of them draws other screens: the
    // references are written on a computer that has them all, and a number smaller than usual says one is missing)
    let text = format!("{}# emoji {}\n", write(&share.entries, false), argos_render::emoji::arrivees());
    std::fs::write(out, text).expect("the share's results are written");
}

/// A copy of this test program, ready to start: it runs the one test `test`, alone and on one thread. A copy cannot
/// reach outside the app, whatever a scene clicks on: no program to start (a link in the browser is `xdg-open`), no
/// system keyring (its bus is not there, and the service is not the app's), no personal folder (`nowhere`: an empty
/// folder), and the emoji images of the project only. What it says goes to the file `log`. Its input is a pipe the
/// test program holds ([`on_its_own`]).
fn copy(test: &str, nowhere: &Path, log: &Path) -> std::process::Command {
    let mut copy = std::process::Command::new(std::env::current_exe().expect("the path of the test program"));
    copy.args([test, "--exact", "--nocapture", "--test-threads", "1"])
        .env("PATH", nowhere)
        .env("DBUS_SESSION_BUS_ADDRESS", format!("unix:path={}", nowhere.join("no-bus").display()))
        .env("ARGOS_SERVICE_CLES", KEYRING)
        .envs(["HOME", "XDG_RUNTIME_DIR", "XDG_CONFIG_HOME", "XDG_CACHE_HOME", "XDG_DATA_HOME"].map(|k| (k, nowhere)))
        .env_remove("ARGOS_EMOJI")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::fs::File::create(log).expect("the copy's log"));
    copy
}

/// What a copy of the test program does first: it will stop when the test program that started it is gone, and it
/// proves that it cannot open the browser.
fn on_its_own() {
    // (the parent holds the other end of this copy's input: when the parent is gone, stopped or not, the copy stops)
    std::thread::spawn(|| {
        let _ = std::io::Read::read(&mut std::io::stdin(), &mut [0]);
        std::process::exit(1);
    });
    // (the proof that a click on a link opens nothing: this copy cannot start the program that opens the browser)
    assert!(
        std::process::Command::new("xdg-open").arg("--version").output().is_err(),
        "a copy of the test program must not be able to open the browser"
    );
}

/// Copies of the test program to start: one per processor (or per test thread asked for), 4 at most.
fn copies() -> usize {
    let processors = std::thread::available_parallelism().map_or(1, |n| n.get());
    let asked = std::env::var("RUST_TEST_THREADS").ok().and_then(|v| v.parse().ok()).unwrap_or(processors);
    processors.min(asked).clamp(1, 4)
}

/// Every scene drawn (once for the two tests): the shares are given to copies of this test program, each one
/// running the first test below alone.
fn results() -> &'static BTreeMap<String, Entry> {
    static RESULTS: OnceLock<BTreeMap<String, Entry>> = OnceLock::new();
    RESULTS.get_or_init(|| {
        let test = format!(
            "{}::every_screen_matches_its_reference",
            module_path!().split_once("::").map_or("", |(_, module)| module)
        );
        let folder = argos_core::dossiers::de_test("safety-net-shares");
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).expect("the folder of the shares");
        // (the new images of the run before are out of date)
        for old in std::fs::read_dir(references_folder()).into_iter().flatten().flatten() {
            let name = old.file_name().to_string_lossy().into_owned();
            if name.starts_with("safety-net-") && name.ends_with(".nouveau.png") {
                let _ = std::fs::remove_file(old.path());
            }
        }
        let nowhere = folder.join("nowhere");
        std::fs::create_dir_all(&nowhere).expect("the empty folder of the copies");
        let of = copies();
        let started: Vec<_> = (0..of)
            .map(|part| {
                let (out, log) = (folder.join(format!("{part}.txt")), folder.join(format!("{part}.log")));
                let mut child = copy(&test, &nowhere, &log)
                    .env(SHARE_OUT, &out)
                    .spawn()
                    .expect("a copy of the test program starts");
                // (its input stays open as long as this parent lives: `wait` would close it first)
                let input = child.stdin.take();
                (child, input, out, log)
            })
            .collect();
        let mut all = BTreeMap::new();
        let mut emojis = 0;
        for (mut child, input, out, log) in started {
            let status = child.wait().expect("a copy of the test program ends");
            drop(input);
            let said = std::fs::read_to_string(&log).unwrap_or_default();
            assert!(status.success(), "a copy of the test program failed ({status}):\n{said}");
            let text = std::fs::read_to_string(&out).expect("the share's results");
            emojis += text
                .lines()
                .filter_map(|l| l.strip_prefix("# emoji "))
                .filter_map(|n| n.parse::<usize>().ok())
                .sum::<usize>();
            all.extend(parse(&text));
        }
        if emojis > 0 {
            eprintln!("🕸 {emojis} image(s) of the folder emoji/ read by the copies: a computer whose folder lacks one of them draws other screens");
        }
        let _ = std::fs::remove_dir_all(&folder);
        all
    })
}

/// The file this copy of the test program was given by its parent for its results, if it is one.
fn share() -> Option<PathBuf> {
    std::env::var(SHARE_OUT).ok().map(PathBuf::from)
}

/// Where the tiles `changed` are on a screen `cols` tiles wide: how many, and the columns and the rows they span
/// (counted from 1, from the top left).
fn spanned(changed: &[usize], cols: usize) -> String {
    let span = |one: &str, of: &dyn Fn(usize) -> usize| {
        let all = changed.iter().map(|&k| of(k) + 1);
        match (all.clone().min(), all.max()) {
            (Some(first), Some(last)) if first == last => format!("{one} {first}"),
            (Some(first), Some(last)) => format!("{one}s {first} to {last}"),
            _ => String::new(),
        }
    };
    let (columns, rows) = (span("column", &|k| k % cols.max(1)), span("row", &|k| k / cols.max(1)));
    format!("{} tile(s), {columns}, {rows}", changed.len())
}

/// The screens that changed, in short, ahead of their list: each kind of change (the same zones and the same tiles of
/// a form), the one with the most screens first, with how many screens it has and one of them to look at (one whose
/// new image is written, when there is one). A button every screen shows changes thousands of screens alike: they
/// read as one line here, and what else changed is not lost among them. Nothing when no two screens changed alike.
fn in_short(results: &BTreeMap<String, Entry>, known: &BTreeMap<String, Entry>) -> String {
    let mut kinds: BTreeMap<(&str, Kind), Vec<&String>> = BTreeMap::new();
    for (name, e) in results {
        // (the screens the list below says « changed in … » of: not a scene that stopped, or a new one)
        let Some(r) = known.get(name).filter(|_| e.problems.iter().any(|problem| problem.starts_with("changed in "))) else {
            continue;
        };
        let (zones, tiles) = e.print.differences(&r.print);
        let form = name.split('/').next().unwrap_or_default();
        kinds.entry((form, (e.print.size, zones, tiles))).or_default().push(name);
    }
    let mut kinds: Vec<_> = kinds.into_iter().collect();
    kinds.sort_by_key(|(_, names)| std::cmp::Reverse(names.len()));
    let written = |name: &&&String| references_folder().join(format!("safety-net-{}.nouveau.png", name.replace('/', "-"))).exists();
    let changed: usize = kinds.iter().map(|(_, names)| names.len()).sum();
    if changed == kinds.len() {
        return String::new();
    }
    let mut short = format!(
        "In short, {changed} screen(s) changed in {} way(s) (the same zones and the same tiles of a form), the one with the most \
         screens first:\n",
        kinds.len()
    );
    for ((form, ((w, _), zones, tiles)), names) in &kinds {
        let one = names.iter().find(written).unwrap_or(&names[0]);
        let tiles = if tiles.is_empty() {
            String::new()
        } else {
            format!(" ({})", spanned(tiles, w.div_ceil(TILE) as usize))
        };
        let _ = writeln!(short, "{:>6} on {form}, in {}{tiles}: see {one}", names.len(), zones.join(", "));
    }
    short + "Each of them:\n"
}

/// 🕸 Every scene gives the image its reference describes (exactly), is at rest and shows the screen it is there for;
/// a new scene, or one that is not its reference any more, gives the same image when drawn twice.
/// `ARGOS_REFERENCES=maj` (or no reference file yet) writes the file.
#[test]
fn every_screen_matches_its_reference() {
    if let Some(out) = share() {
        return draw_share(&out);
    }
    let results = results();
    let path = reference_path();
    let known = if updating() {
        None
    } else {
        std::fs::read_to_string(&path).ok().map(|t| parse(&t))
    };
    let mut wrong: Vec<String> = results
        .iter()
        .flat_map(|(name, e)| e.problems.iter().map(move |p| format!("{name}: {p}")))
        .collect();
    let (frames, ms) = results.values().fold((0, 0.0), |(f, t), e| (f + e.frames, t + e.ms));
    // (the time of an image on each form: the starting picture of the split into modules reads it here)
    let on_each: Vec<String> = Form::ALL
        .into_iter()
        .chain(Form::TABLET)
        .filter_map(|form| {
            let of = results.iter().filter(|(name, _)| name.split('/').next() == Some(form.tag()));
            let (frames, ms) = of.fold((0, 0.0), |(f, t), (_, e)| (f + e.frames, t + e.ms));
            (frames > 0).then(|| format!("{} {:.0}", form.tag(), ms / frames as f64))
        })
        .collect();
    eprintln!(
        "🕸 safety net: {} scenes, {frames} images drawn, {:.0} ms an image ({}), {} with a known difference of the app",
        results.len(),
        ms / frames.max(1) as f64,
        on_each.join(", "),
        results.values().filter(|e| !e.known.is_empty()).count()
    );
    // (where the time of the copies went: what to shorten first, the day the net is too long)
    let [made, opened, read, whole] = results.values().fold([0.0; 4], |sum, e| std::array::from_fn(|k| sum[k] + e.spent[k]));
    let part = |of: f64| 100.0 * of / whole.max(1.0);
    eprintln!(
        "🕸 {:.0} s of work in the copies: the app made at rest {:.0} %, the screen opened {:.0} %, its images {:.0} %, what \
         follows read {:.0} %, the rest (compared, drawn twice when new) {:.0} %",
        whole / 1000.0,
        part(made),
        part(opened),
        part(ms),
        part(read),
        part(whole - made - opened - ms - read)
    );
    // (and by family of scenes, « pc/list »: the eight longest)
    let mut families: BTreeMap<String, (usize, f64)> = BTreeMap::new();
    for (name, e) in results {
        let family = name.match_indices('/').nth(1).map_or(name.as_str(), |(at, _)| &name[..at]);
        let (scenes, time) = families.entry(family.to_string()).or_default();
        (*scenes, *time) = (*scenes + 1, *time + e.spent[3]);
    }
    let mut longest: Vec<_> = families.into_iter().collect();
    longest.sort_by(|a, b| b.1 .1.total_cmp(&a.1 .1));
    let said: Vec<String> = longest
        .iter()
        .take(8)
        .map(|(family, (scenes, time))| format!("{family} {:.0} s ({scenes} scenes)", time / 1000.0))
        .collect();
    eprintln!("🕸 the longest families: {}", said.join(", "));
    if let Some(only) = only() {
        // a look at a part of the screens says what it found, and is never a pass
        let new = results.keys().filter(|n| known.as_ref().is_some_and(|k| !k.contains_key(*n)));
        wrong.extend(new.map(|n| format!("{n}: no reference yet")));
        let short = known.as_ref().map(|known| in_short(results, known)).unwrap_or_default();
        panic!(
            "{ONLY}={only}: {} scene(s) drawn, {} thing(s) to see:\n{short}{}\nA part of the screens is never a pass, and writes \
             no reference: run the test again without {ONLY}.",
            results.len(),
            wrong.len(),
            wrong.join("\n")
        );
    }
    // (a known difference of the app no scene shows any more: it was fixed, its line goes)
    let seen = |what: &str| results.values().any(|e| e.known.iter().any(|known| known == what));
    for (list, apart) in [("KEPT_APART", &KEPT_APART[..]), ("MOVING_APART", &MOVING_APART[..])] {
        wrong.extend(
            apart
                .iter()
                .filter(|(what, _)| !seen(what))
                .map(|(what, _)| format!("no scene shows any more that {what}: take that line out of {list}")),
        );
    }
    wrong.extend(
        STOPS_APART
            .iter()
            .filter(|(what, ..)| !seen(what))
            .map(|(what, ..)| format!("no scene shows any more that {what}: take that line out of STOPS_APART")),
    );
    let Some(known) = known else {
        assert!(wrong.is_empty(), "scenes to fix before their references are written:\n{}", wrong.join("\n"));
        std::fs::write(&path, write(results, true)).expect("the reference file is written");
        return eprintln!("🕸 references written: {}", path.display());
    };
    wrong.extend(results.keys().filter(|n| !known.contains_key(*n)).map(|n| format!("{n}: no reference yet")));
    wrong.extend(
        known
            .keys()
            .filter(|n| !results.contains_key(*n))
            .map(|n| format!("{n}: a reference without its scene")),
    );
    // (many screens that changed alike: said in short first)
    let short = in_short(results, &known);
    assert!(
        wrong.is_empty(),
        "{} screen(s) differ from their reference. A wanted change: ARGOS_REFERENCES=maj, then read and commit the \
         diff of {} (the new images are next to it, changed tiles framed in red):\n{short}{}",
        wrong.len(),
        path.display(),
        wrong.join("\n")
    );
}

/// 🕸 Every screen the code's own lists know is shown by at least one scene, on each form where it exists; and a
/// screen said not to exist on a form (`universe::exceptions`) is shown by no scene of that form.
#[test]
fn every_screen_has_a_scene() {
    if only().is_some() {
        return eprintln!("🕸 {ONLY}: a part of the screens only, nothing to say about the others");
    }
    let results = results();
    let of = |form: Form| results.iter().filter(move |(name, _)| name.split('/').next() == Some(form.tag()));
    let missing: Vec<String> = universe::all()
        .into_iter()
        .filter(|(form, id)| !of(*form).any(|(_, e)| e.shown.contains(id)))
        .map(|(form, id)| format!("{} · {id}", form.tag()))
        .collect();
    let wrong: Vec<String> = universe::exceptions()
        .into_iter()
        .filter_map(|(form, what, why)| {
            let (scene, _) = of(form).find(|(_, e)| e.shown.iter().any(|id| universe::is(what, id)))?;
            Some(format!("{} · {what} is shown by {scene}, but: {why}", form.tag()))
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "{} exception(s) of safety_net/universe.rs to remove:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
    assert!(
        missing.is_empty(),
        "{} screen(s) without a scene (add one in safety_net/scenes.rs, or say in safety_net/universe.rs why this form \
         does not have it):\n{}",
        missing.len(),
        missing.join("\n")
    );
}
