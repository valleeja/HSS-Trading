//! Texte (prix, heures, titre). Trois polices libres sont incluses dans Argos (lues et dessinées par swash, avec hinting) :
//! Inter (SIL OFL 1.1), DejaVu Sans (licence DejaVu / Bitstream Vera) et Liberation Sans (SIL OFL 1.1) ; DejaVu Sans
//! Mono pour le code (`Mono`).
//! Si un caractère manque dans la police choisie, il est pris dans la suivante (repli automatique).
//! Les émojis (« 🚀 GOLD BUY », « ✅ ») sont dessinés en couleur avec les images Twemoji (`emoji`).

use std::sync::{Arc, OnceLock};
use tiny_skia::{Pixmap, PremultipliedColorU8};

#[cfg(test)]
thread_local! {
    // 🧪 Tests: every text written by `Text::draw` on this thread (to check what a screen shows).
    static ECRITS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// 🧪 Tests: the texts written on this thread since the last call (and forget them).
#[cfg(test)]
pub fn ecrits() -> Vec<String> {
    ECRITS.with(|e| std::mem::take(&mut *e.borrow_mut()))
}

/// 🧪 Tests: where a text was written: its left and right edges and its baseline, in points, and its size.
#[cfg(test)]
#[derive(Debug, Clone)]
pub struct Boite {
    pub texte: String,
    pub x0: f32,
    pub x1: f32,
    pub y: f32,
    pub size: f32,
}

#[cfg(test)]
thread_local! {
    static BOITES: std::cell::RefCell<Vec<Boite>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// 🧪 Tests: the places of the texts written on this thread since the last call (and forget them), to check that
/// nothing is written over another text or past a panel's edge.
#[cfg(test)]
pub fn boites() -> Vec<Boite> {
    BOITES.with(|e| std::mem::take(&mut *e.borrow_mut()))
}

/// 🇫🇷 (tableaux-1 tour 3, H4) « de » before a name, elided before a vowel or a mute h: « d'Argos Murs »,
/// « d'indicateur », « de MACD », « de Heikin Ashi ». Leading emojis and signs are skipped to find the first letter;
/// the h is aspirated (« de ») except for the few French words that start with a mute one.
pub fn de(nom: &str) -> String {
    let premier = nom.chars().find(|c| c.is_alphanumeric());
    let voyelle = premier.is_some_and(|c| "aeiouyàâäéèêëîïôöùûüœæ".contains(c.to_lowercase().next().unwrap_or(c)));
    let bas = nom.trim_start_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
    let h_muet = ["heur", "hist", "horai", "horiz", "hyper", "hypo", "hebdo", "homo"]
        .iter()
        .any(|p| bas.starts_with(p));
    if voyelle || h_muet {
        format!("d'{nom}")
    } else {
        format!("de {nom}")
    }
}

/// Couleur RVB.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl From<(u8, u8, u8)> for Rgb {
    fn from((r, g, b): (u8, u8, u8)) -> Self {
        Rgb(r, g, b)
    }
}

impl Rgb {
    /// « #rrggbb » → couleur (gris si invalide).
    pub fn hex(s: &str) -> Self {
        let h = s.trim_start_matches('#');
        let p = |i: usize| u8::from_str_radix(h.get(i..i + 2).unwrap_or("80"), 16).unwrap_or(0x80);
        Rgb(p(0), p(2), p(4))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// Les polices incluses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontFamily {
    Inter,
    DejaVuSans,
    LiberationSans,
}

impl FontFamily {
    pub const ALL: [FontFamily; 3] = [FontFamily::Inter, FontFamily::DejaVuSans, FontFamily::LiberationSans];

    fn bytes(self) -> &'static [u8] {
        match self {
            FontFamily::Inter => include_bytes!("../assets/Inter-Regular.otf"),
            FontFamily::DejaVuSans => include_bytes!("../assets/DejaVuSans.ttf"),
            FontFamily::LiberationSans => include_bytes!("../assets/LiberationSans-Regular.ttf"),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            FontFamily::Inter => "Inter",
            FontFamily::DejaVuSans => "DejaVu Sans",
            FontFamily::LiberationSans => "Liberation Sans",
        }
    }
}

/// Police choisie + les deux autres en repli. Les polices sont lues **une seule fois** pour toute l'app (les lire
/// coûte des dizaines de ms) : un `Text` ne fait que les partager, il se crée en un rien de temps.
pub struct Text {
    fonts: Vec<Arc<Font>>,
    /// Letters slanted (italic drawn from the upright font: Argos has no italic font).
    pub italic: bool,
}

/// Les polices incluses, lues au premier usage (même ordre que `FontFamily::ALL`).
fn loaded() -> &'static [Arc<Font>] {
    static FONTS: OnceLock<Vec<Arc<Font>>> = OnceLock::new();
    FONTS.get_or_init(|| {
        FontFamily::ALL
            .iter()
            .map(|f| Arc::new(Font::from_bytes(f.bytes()).unwrap_or_else(|e| panic!("police {} : {e}", f.name()))))
            .collect()
    })
}

/// Graisse d'Inter (verre-2, comme la maquette) : Regular (texte), Medium (valeurs, tuiles), Bold (titres).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Graisse {
    Regular,
    Medium,
    Bold,
}

/// Inter Medium et Bold, lus une seule fois au premier usage (paquet fonts-inter 4.1, licence OFL 1.1).
fn inter(g: Graisse) -> Option<Arc<Font>> {
    static MEDIUM: OnceLock<Arc<Font>> = OnceLock::new();
    static BOLD: OnceLock<Arc<Font>> = OnceLock::new();
    let (cell, bytes): (&OnceLock<Arc<Font>>, &'static [u8]) = match g {
        Graisse::Regular => return None,
        Graisse::Medium => (&MEDIUM, include_bytes!("../assets/Inter-Medium.otf")),
        Graisse::Bold => (&BOLD, include_bytes!("../assets/Inter-Bold.otf")),
    };
    Some(
        cell.get_or_init(|| Arc::new(Font::from_bytes(bytes).unwrap_or_else(|e| panic!("police Inter {g:?} : {e}"))))
            .clone(),
    )
}

impl Default for Text {
    fn default() -> Self {
        Self::new(FontFamily::Inter)
    }
}

impl Text {
    pub fn new(main: FontFamily) -> Self {
        let all = loaded();
        let order = std::iter::once(main).chain(FontFamily::ALL.into_iter().filter(|f| *f != main));
        let fonts = order
            .filter_map(|f| FontFamily::ALL.iter().position(|x| *x == f).map(|i| all[i].clone()))
            .collect();
        Self { fonts, italic: false }
    }

    /// Le texte en Inter de cette graisse (les autres polices restent en repli pour les caractères absents).
    pub fn graisse(g: Graisse) -> Self {
        let mut t = Self::default();
        if let Some(f) = inter(g) {
            t.fonts.insert(0, f);
        }
        t
    }

    /// Some font has this character (otherwise it is drawn as the « missing » box).
    pub fn has_char(&self, c: char) -> bool {
        self.fonts.iter().any(|f| f.lookup_glyph_index(c) != 0)
    }

    /// Première police qui a ce caractère (la principale sinon, qui dessinera son carré « manquant »).
    fn font_for(&self, c: char) -> &Font {
        self.fonts.iter().find(|f| f.lookup_glyph_index(c) != 0).unwrap_or(&self.fonts[0])
    }

    fn metrics(&self, c: char, size: f32) -> Metrics {
        self.font_for(c).metrics(c, size)
    }

    /// Largeur du texte en pixels (dans la langue choisie).
    pub fn width(&self, s: &str, size: f32) -> f32 {
        self.raw_width(&crate::i18n::tr(s), size)
    }

    fn raw_width(&self, s: &str, size: f32) -> f32 {
        pieces(s)
            .map(|p| match p {
                Piece::Emoji(_) => emoji_advance(size),
                Piece::Char(c) => self.metrics(c, size).advance_width,
            })
            .sum()
    }

    /// Texte (dans la langue choisie) coupé avec « … » pour tenir dans `max` pixels.
    pub fn fit(&self, s: &str, size: f32, max: f64) -> String {
        let mut out = crate::i18n::tr(s).into_owned();
        if self.raw_width(&out, size) as f64 <= max {
            return out;
        }
        while !out.is_empty() && self.raw_width(&format!("{out}…"), size) as f64 > max {
            out.pop();
        }
        format!("{out}…")
    }

    /// Écrit `s` (dans la langue choisie) avec sa ligne de base à `y`, aligné sur `x` (`at` = `(x, y)`).
    pub fn draw(&self, pm: &mut Pixmap, s: &str, at: (f32, f32), size: f32, color: Rgb, align: Align) {
        self.draw_avec(pm, s, at, size, color, align, 0.0);
    }

    /// Comme `draw` (aligné à gauche), les lettres écartées de `ecart` points de plus (`letter-spacing` des petits
    /// titres en capitales de la maquette).
    pub fn draw_espace(&self, pm: &mut Pixmap, s: &str, at: (f32, f32), size: f32, color: Rgb, ecart: f32) {
        self.draw_avec(pm, s, at, size, color, Align::Left, ecart);
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_avec(&self, pm: &mut Pixmap, s: &str, (x, y): (f32, f32), size: f32, color: Rgb, align: Align, ecart: f32) {
        let s = crate::i18n::tr(s);
        let s = s.as_ref();
        #[cfg(test)]
        ECRITS.with(|e| e.borrow_mut().push(s.to_string()));
        let w = self.raw_width(s, size);
        let mut pen = match align {
            Align::Left => x,
            Align::Center => x - w / 2.0,
            Align::Right => x - w,
        };
        #[cfg(test)]
        BOITES.with(|e| {
            let tout = w + ecart * pieces(s).count() as f32;
            e.borrow_mut().push(Boite {
                texte: s.to_string(),
                x0: pen,
                x1: pen + tout,
                y,
                size,
            })
        });
        for piece in pieces(s) {
            let c = match piece {
                Piece::Emoji(e) => {
                    // émoji : image carrée posée sur la ligne de base, un peu plus haute que les lettres
                    let px = crate::px::dev((size * 1.1) as f64).round() as u32;
                    if let Some(img) = crate::emoji::image(e, px) {
                        crate::px::put(pm, (pen + size * 0.05) as f64, (y - size * 0.92) as f64, &img);
                    }
                    pen += emoji_advance(size) + ecart;
                    continue;
                }
                Piece::Char(c) => c,
            };
            pen += blit(pm, self.font_for(c), c, pen, y, size, color, self.italic) + ecart;
        }
    }
}

/// Dessine un caractère (ligne de base `y`, en points) ; rend sa largeur (en points). La lettre est dessinée à la
/// taille de l'écran (`px::scale()` fois plus grande) : nette sur iPhone. 🔍 Hinted (its stems on the pixel grid,
/// like a browser's text) and placed to a quarter of a screen pixel; the advance stays the font's own (layout unchanged).
#[allow(clippy::too_many_arguments)]
fn blit(pm: &mut Pixmap, font: &Font, c: char, pen: f32, y: f32, size: f32, color: Rgb, italic: bool) -> f32 {
    let s = crate::px::scale();
    let advance = font.metrics(c, size).advance_width;
    let gid = font.lookup_glyph_index(c);
    let x = pen * s;
    // 🔍 small text (under 20 screen pixels): on a whole pixel, its hinted stems stay crisp (like a browser's interface
    // text); larger text keeps its quarter pixel
    let (x0, frac) = if size * s < SNAP_PX {
        (x.round(), 0.0)
    } else {
        (x.floor(), ((x - x.floor()) * 4.0).round() / 4.0)
    };
    let Some(g) = glyphs::get(font, gid, size * s, frac) else {
        return advance;
    };
    let (pw, ph) = (pm.width() as i32, pm.height() as i32);
    let boost = renfort(color);
    let pixels = pm.pixels_mut();
    let gx = x0 as i32 + g.left;
    let gy = (y * s).round() as i32 - g.top;
    let base = (y * s).round() as i32;
    for row in 0..g.height as i32 {
        // italic: each row moved right by a fifth of its height above the baseline
        let slant = if italic { ((base - (gy + row)) as f32 * 0.2).round() as i32 } else { 0 };
        for col in 0..g.width as i32 {
            let a = boost[g.data[(row * g.width as i32 + col) as usize] as usize] as u32;
            let (px, py) = (gx + col + slant, gy + row);
            if a == 0 || px < 0 || py < 0 || px >= pw || py >= ph {
                continue;
            }
            let p = &mut pixels[(py * pw + px) as usize];
            let mix = |src: u8, dst: u8| ((src as u32 * a + dst as u32 * (255 - a)) / 255) as u8;
            *p = PremultipliedColorU8::from_rgba(mix(color.0, p.red()), mix(color.1, p.green()), mix(color.2, p.blue()), 255).unwrap();
        }
    }
    advance
}

/// 🔍 Under this size (screen pixels), letters are put on a whole pixel.
const SNAP_PX: f32 = 20.0;

/// 🔍 The coverage table for a text colour (Jee, 19 Nov: text softer than a browser's). Coverage is mixed in sRGB
/// here, which makes light text on a dark background look thinner and greyer than a browser draws it (it mixes with
/// a gamma and a contrast boost, like Skia); light text gets its coverage raised (`a^(1/(1 + 0.45 × lightness))`):
/// white ½ → 0.62, black unchanged. 0 and full coverage never change.
fn renfort(color: Rgb) -> &'static [u8; 256] {
    static TABLES: OnceLock<Vec<[u8; 256]>> = OnceLock::new();
    let tables = TABLES.get_or_init(|| {
        (0..=16)
            .map(|l| {
                let e = 1.0 / (1.0 + 0.45 * l as f32 / 16.0);
                std::array::from_fn(|a| ((a as f32 / 255.0).powf(e) * 255.0).round() as u8)
            })
            .collect()
    });
    let lum = (299 * color.0 as u32 + 587 * color.1 as u32 + 114 * color.2 as u32) / 1000;
    &tables[(lum * 16 / 255) as usize]
}

/// Une police incluse (ses octets, lus par swash).
pub struct Font {
    data: &'static [u8],
    offset: u32,
    key: swash::CacheKey,
}

/// Largeur d'avance d'un caractère (points).
pub struct Metrics {
    pub advance_width: f32,
}

impl Font {
    fn from_bytes(data: &'static [u8]) -> Result<Self, String> {
        let f = swash::FontRef::from_index(data, 0).ok_or("police illisible")?;
        Ok(Self {
            data,
            offset: f.offset,
            key: f.key,
        })
    }

    fn as_ref(&self) -> swash::FontRef<'static> {
        swash::FontRef {
            data: self.data,
            offset: self.offset,
            key: self.key,
        }
    }

    /// Glyphe du caractère (0 : absent de la police).
    pub fn lookup_glyph_index(&self, c: char) -> u16 {
        self.as_ref().charmap().map(c)
    }

    /// Avance du caractère à la taille `size` (sans hinting : la mise en page ne bouge pas).
    pub fn metrics(&self, c: char, size: f32) -> Metrics {
        let f = self.as_ref();
        Metrics {
            advance_width: f.glyph_metrics(&[]).scale(size).advance_width(f.charmap().map(c)),
        }
    }
}

/// 🔍 Cache of rendered letters (font, glyph, size, quarter-pixel offset): a letter is hinted and rasterised once.
mod glyphs {
    use super::Font;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;
    use swash::scale::{image::Content, Render, ScaleContext, Source};
    use swash::zeno::{Format, Vector};

    /// A rendered letter: coverage (one byte per pixel) and its place from the pen on the baseline.
    pub(super) struct Glyph {
        pub left: i32,
        pub top: i32,
        pub width: u32,
        pub height: u32,
        pub data: Vec<u8>,
    }

    type Key = (u64, u16, u32, u8);

    thread_local! {
        static CTX: RefCell<ScaleContext> = RefCell::new(ScaleContext::new());
        static CACHE: RefCell<HashMap<Key, Option<Rc<Glyph>>>> = RefCell::new(HashMap::new());
    }

    /// The letter `gid` of `font` at `px` screen pixels, moved right by `frac` (0, ¼, ½, ¾) of a pixel.
    pub(super) fn get(font: &Font, gid: u16, px: f32, frac: f32) -> Option<Rc<Glyph>> {
        let key = (font.key.value(), gid, px.to_bits(), (frac * 4.0) as u8);
        if let Some(g) = CACHE.with(|c| c.borrow().get(&key).cloned()) {
            return g;
        }
        let g = CTX.with(|ctx| {
            let mut ctx = ctx.borrow_mut();
            let mut scaler = ctx.builder(font.as_ref()).size(px).hint(true).build();
            let img = Render::new(&[Source::Outline])
                .format(Format::Alpha)
                .offset(Vector::new(frac, 0.0))
                .render(&mut scaler, gid)?;
            (img.content == Content::Mask).then(|| {
                Rc::new(Glyph {
                    left: img.placement.left,
                    top: img.placement.top,
                    width: img.placement.width,
                    height: img.placement.height,
                    data: img.data,
                })
            })
        });
        CACHE.with(|c| {
            let mut c = c.borrow_mut();
            // (bounded: many sizes × letters; refilled as needed)
            if c.len() > 20_000 {
                c.clear();
            }
            c.insert(key, g.clone());
        });
        g
    }
}

/// Police à chasse fixe pour le code (DejaVu Sans Mono, licence DejaVu) : tous les caractères ont la même largeur,
/// et le texte n'est jamais traduit.
pub struct Mono;

fn mono_font() -> &'static Font {
    static F: OnceLock<Font> = OnceLock::new();
    F.get_or_init(|| Font::from_bytes(include_bytes!("../assets/DejaVuSansMono.ttf")).unwrap_or_else(|e| panic!("police mono : {e}")))
}

impl Mono {
    /// Largeur d'un caractère.
    pub fn advance(size: f32) -> f32 {
        mono_font().metrics('M', size).advance_width
    }

    /// Écrit `s` à partir de `x` (ligne de base `y`) ; un caractère qui manque à la police mono vient des autres.
    /// (tableaux-1 tour 2, F8) An emoji (« 🧱 ») is drawn in colour as in the rest of the app (Twemoji), at its column;
    /// the columns stay one per character (the cursor's).
    pub fn draw(pm: &mut Pixmap, s: &str, (x, y): (f32, f32), size: f32, color: Rgb) {
        let adv = Self::advance(size);
        let fallback = Text::default();
        let mut k = 0usize;
        for piece in pieces(s) {
            let c = match piece {
                Piece::Emoji(e) => {
                    let px = crate::px::dev((size * 1.1) as f64).round() as u32;
                    if let Some(img) = crate::emoji::image(e, px) {
                        crate::px::put(pm, (x + k as f32 * adv + size * 0.05) as f64, (y - size * 0.92) as f64, &img);
                    }
                    k += e.chars().count();
                    continue;
                }
                Piece::Char(c) => c,
            };
            if c != ' ' {
                let font = if mono_font().lookup_glyph_index(c) != 0 {
                    mono_font()
                } else {
                    fallback.font_for(c)
                };
                blit(pm, font, c, x + k as f32 * adv, y, size, color, false);
            }
            k += 1;
        }
    }
}

/// Morceau de texte : un caractère, ou un émoji (une ou plusieurs lettres Unicode).
enum Piece<'a> {
    Char(char),
    Emoji(&'a str),
}

fn pieces(s: &str) -> impl Iterator<Item = Piece<'_>> {
    let mut i = 0;
    std::iter::from_fn(move || {
        let rest = s.get(i..).filter(|r| !r.is_empty())?;
        if let Some(n) = crate::emoji::at_start(rest) {
            i += n;
            return Some(Piece::Emoji(&rest[..n]));
        }
        let c = rest.chars().next()?;
        i += c.len_utf8();
        Some(Piece::Char(c))
    })
}

/// Place prise par un émoji.
fn emoji_advance(size: f32) -> f32 {
    (size * 1.2).round()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn de_s_elide_devant_une_voyelle() {
        assert_eq!(super::de("Argos Murs"), "d'Argos Murs");
        assert_eq!(super::de("Argos TVD Pro"), "d'Argos TVD Pro");
        assert_eq!(super::de("indicateur"), "d'indicateur");
        assert_eq!(super::de("Heikin Ashi"), "de Heikin Ashi");
        assert_eq!(super::de("MACD"), "de MACD");
        assert_eq!(super::de("🧱 Argos Murs"), "d'🧱 Argos Murs");
        assert_eq!(super::de("Écart-type"), "d'Écart-type");
        assert_eq!(super::de("historique"), "d'historique");
        assert_eq!(super::de(""), "de ");
    }

    #[test]
    fn emoji_en_couleur_dans_le_texte() {
        let t = Text::default();
        assert_eq!(t.width("🚀", 12.0), emoji_advance(12.0));
        let mut pm = crate::px::new(120, 30).unwrap();
        t.draw(&mut pm, "🚀 GO", (4.0, 22.0), 14.0, Rgb(255, 255, 255), Align::Left);
        // la fusée Twemoji a du rouge / orange : pas juste du blanc
        assert!(
            pm.pixels().iter().any(|p| p.red() > 150 && p.blue() < 120 && p.alpha() > 200),
            "émoji en couleur"
        );
    }

    #[test]
    fn trois_polices_se_chargent_et_dessinent() {
        for f in FontFamily::ALL {
            let t = Text::new(f);
            assert!(t.width("80 660,0", 12.0) > 30.0, "{}", f.name());
            let mut pm = crate::px::new(120, 30).unwrap();
            t.draw(&mut pm, "84 850", (4.0, 20.0), 14.0, Rgb(255, 255, 255), Align::Left);
            assert!(pm.pixels().iter().any(|p| p.red() > 200), "{} n'a rien dessiné", f.name());
        }
    }

    #[test]
    fn repli_sur_une_autre_police() {
        let t = Text::new(FontFamily::Inter);
        // le symbole « ⚠ » n'est pas dans Inter : il doit venir d'une police de repli
        let c = '⚠';
        let has_inter = t.fonts[0].lookup_glyph_index(c) != 0;
        let used = t.font_for(c) as *const Font;
        if !has_inter {
            assert_ne!(used, &*t.fonts[0] as *const Font);
        }
        assert!(t.fonts.iter().any(|f| f.lookup_glyph_index(c) != 0), "aucune police n'a ⚠");
    }

    /// 🅱 Les graisses d'Inter : le gras est plus large que le texte normal, Medium entre les deux, et tout se lit.
    #[test]
    fn graisses_inter() {
        let (r, m, b) = (Text::default(), Text::graisse(Graisse::Medium), Text::graisse(Graisse::Bold));
        let w = |t: &Text| t.width("Bonjour Académie 100 412 $", 20.0);
        assert!(w(&r) < w(&m) && w(&m) < w(&b), "{} {} {}", w(&r), w(&m), w(&b));
        assert_eq!(Text::graisse(Graisse::Regular).width("abc", 14.0), r.width("abc", 14.0));
        let mut pm = crate::px::new(200, 40).unwrap();
        b.draw(&mut pm, "Gras ⚠", (4.0, 30.0), 20.0, Rgb(255, 255, 255), Align::Left);
        assert!(pm.pixels().iter().any(|p| p.alpha() > 0));
    }

    /// 🔍 Jee, 19 Nov: light text was softer than a browser's. Its coverage is raised (never 0 nor full), dark text is
    /// unchanged, and small text sits on a whole pixel (the same picture at x 10 and x 10.3).
    #[test]
    fn texte_net_comme_un_navigateur() {
        let blanc = renfort(Rgb(255, 255, 255));
        assert_eq!((blanc[0], blanc[255]), (0, 255));
        assert!(blanc[128] >= 155, "white ½ raised ({})", blanc[128]);
        assert!(blanc.windows(2).all(|w| w[0] <= w[1]), "the table keeps the order");
        let noir = renfort(Rgb(0, 0, 0));
        assert!((0..256).all(|a| noir[a] as usize == a), "dark text unchanged");
        let t = Text::default();
        let draw = |x: f32| {
            let mut pm = Pixmap::new(80, 24).unwrap();
            t.draw(&mut pm, "l", (x, 16.0), 12.0, Rgb(230, 230, 230), Align::Left);
            pm.data().to_vec()
        };
        assert_eq!(draw(10.0), draw(10.3), "a 12 pt letter on a whole pixel");
    }

    /// The code before its test module (`#[cfg(test)] mod tests { … }`; a `mod tests;` line is not the end).
    fn sans_tests(code: &str) -> &str {
        let mut depuis = 0;
        while let Some(k) = code[depuis..].find("#[cfg(test)]").map(|k| depuis + k) {
            let apres = code[k + 12..].trim_start();
            if let Some(m) = apres.strip_prefix("mod ") {
                if m.find(['{', ';']).is_some_and(|j| m.as_bytes()[j] == b'{') {
                    return &code[..k];
                }
            }
            depuis = k + 12;
        }
        code
    }

    /// The string literals of Rust code, with their line: escapes read (`\u{…}`, `\n`, `\"`…); comments, chars and
    /// byte strings skipped.
    fn chaines(code: &str) -> Vec<(usize, String)> {
        let c: Vec<char> = code.chars().collect();
        let at = |k: usize| c.get(k).copied().unwrap_or('\0');
        let (mut i, mut ligne, mut out) = (0, 1, vec![]);
        while i < c.len() {
            let mot = i > 0 && (c[i - 1].is_alphanumeric() || c[i - 1] == '_');
            match c[i] {
                '\n' => ligne += 1,
                '/' if at(i + 1) == '/' => {
                    while i + 1 < c.len() && c[i + 1] != '\n' {
                        i += 1;
                    }
                }
                '/' if at(i + 1) == '*' => {
                    let mut prof = 0;
                    loop {
                        match (at(i), at(i + 1)) {
                            ('\0', _) => break,
                            ('/', '*') => {
                                prof += 1;
                                i += 1;
                            }
                            ('*', '/') => {
                                prof -= 1;
                                i += 1;
                                if prof == 0 {
                                    break;
                                }
                            }
                            ('\n', _) => ligne += 1,
                            _ => {}
                        }
                        i += 1;
                    }
                }
                // a char, '\…' or 'x' (a lifetime 'a has no closing quote)
                '\'' if at(i + 1) == '\\' => {
                    i += 3;
                    while i < c.len() && c[i] != '\'' {
                        i += 1;
                    }
                }
                '\'' if at(i + 2) == '\'' => i += 2,
                // "…", r"…", r#"…"#, b"…", br"…"
                '"' | 'r' | 'b' if c[i] == '"' || !mot => {
                    let octets = c[i] == 'b';
                    let mut j = i + usize::from(octets);
                    let brut = at(j) == 'r';
                    j += usize::from(brut);
                    let mut dieses = 0;
                    while brut && at(j) == '#' {
                        dieses += 1;
                        j += 1;
                    }
                    if at(j) != '"' {
                        i += 1;
                        continue;
                    }
                    let (debut, mut s) = (ligne, String::new());
                    j += 1;
                    while j < c.len() {
                        match c[j] {
                            '"' if (1..=dieses).all(|k| at(j + k) == '#') => {
                                j += dieses;
                                break;
                            }
                            '\\' if !brut => {
                                j += 1;
                                match at(j) {
                                    'n' => s.push('\n'),
                                    't' => s.push('\t'),
                                    'u' => {
                                        let fin = (j..c.len()).find(|&k| c[k] == '}').unwrap_or(j);
                                        let hexa: String = c[(j + 2).min(fin)..fin].iter().collect();
                                        s.extend(u32::from_str_radix(&hexa, 16).ok().and_then(char::from_u32));
                                        j = fin;
                                    }
                                    '\n' => {
                                        ligne += 1;
                                        while at(j + 1).is_whitespace() {
                                            j += 1;
                                            ligne += usize::from(c[j] == '\n');
                                        }
                                    }
                                    e => s.push(e),
                                }
                            }
                            ch => {
                                ligne += usize::from(ch == '\n');
                                s.push(ch);
                            }
                        }
                        j += 1;
                    }
                    if !octets {
                        out.push((debut, s));
                    }
                    i = j;
                }
                _ => {}
            }
            i += 1;
        }
        out
    }

    #[test]
    fn chaines_du_code() {
        let code = "let a = \"x ⏯ \\u{fe0f}\\\"\"; // \"pas ça\"\n/* \"ni ça\" */ let b = r#\"y \"z\"\"#; let c = '\"';\n\
                    let d = b\"octets\"; fn f<'a>(s: &'a str) {} let e = \"\\\n    suite\";";
        let v = chaines(code);
        let attendu = [(1, "x ⏯ \u{fe0f}\""), (2, "y \"z\""), (3, "suite")];
        assert_eq!(v, attendu.map(|(l, s)| (l, s.to_string())));
        assert_eq!(
            sans_tests("a\n#[cfg(test)]\nmod tests;\nb\n#[cfg(test)]\nmod tests {\nc"),
            "a\n#[cfg(test)]\nmod tests;\nb\n"
        );
    }

    /// 🔍 Jee, 21 Nov: the ⏯ of the Simulateur's replay message was an empty box (no font has it, and without FE0F it
    /// is not an emoji). Every text written in the code (the crates' string literals, the Académie's lessons) draws:
    /// each character is in a font or part of an emoji. Not checked: the tests, and the icons drawn otherwise (Phosphor
    /// icons, the ⤢ arrows).
    #[test]
    fn chaque_texte_du_code_se_dessine() {
        let t = Text::default();
        let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut fichiers = vec![racine.clone()];
        let (mut vus, mut cases) = (0, vec![]);
        while let Some(p) = fichiers.pop() {
            let nom = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            if p.is_dir() {
                if nom != "tests" && nom != "target" {
                    fichiers.extend(std::fs::read_dir(&p).unwrap().map(|e| e.unwrap().path()));
                }
                continue;
            }
            let lecon = p.components().any(|c| c.as_os_str() == "contenu");
            let textes = match p.extension().and_then(|e| e.to_str()) {
                Some("rs") if nom != "tests.rs" => chaines(sans_tests(&std::fs::read_to_string(&p).unwrap())),
                Some("md") if lecon => {
                    let md = std::fs::read_to_string(&p).unwrap();
                    md.lines().enumerate().map(|(k, l)| (k + 1, l.to_string())).collect()
                }
                _ => continue,
            };
            for (ligne, s) in textes {
                vus += 1;
                if s == "⤢" || crate::phosphor::from_emoji(&s).is_some() {
                    continue;
                }
                let vides: String = pieces(&s)
                    .filter_map(|x| match x {
                        Piece::Char(c) if !c.is_ascii() && !t.has_char(c) => Some(c),
                        _ => None,
                    })
                    .collect();
                if !vides.is_empty() {
                    let f = p.strip_prefix(&racine).unwrap_or(&p).display();
                    cases.push(format!("{f}:{ligne} « {vides} » in {s:?}"));
                }
            }
        }
        assert!(vus > 10_000, "the guard must see the code's texts ({vus})");
        assert!(
            cases.is_empty(),
            "drawn as an empty box (FE0F makes an emoji, or another sign):\n{}",
            cases.join("\n")
        );
    }
}
