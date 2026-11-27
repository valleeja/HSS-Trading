//! 🫧 Style verre (« liquid glass ») : par-dessus n'importe quel thème, les menus et fenêtres flottantes laissent voir
//! la chart floutée derrière eux, avec un reflet clair sur le bord, une ombre douce et des coins plus arrondis.
//! Verre laiteux en thème clair, fumé en thème sombre. Désactivé par défaut (réglage dans 🎨 Thème).
//!
//! Le flou est un flou en boîte sur une image réduite ([`DOWN`] fois plus petite, rayon [`RADIUS`]) : peu de calcul,
//! même sur téléphone. La teinte est ensuite bornée en luminance pour que le texte secondaire garde au moins 4,5:1 de
//! contraste, quoi qu'il y ait derrière (le pire cas est testé).

use crate::palette::{contrast, luminance};
use crate::text::Rgb;

// (🏎 lot A: the glass's switch and quality belong to the thread that draws, like the scale (`px`): the app draws on
// one thread; each test has its own, so the speed guard's glass never reaches another test's image)
thread_local! {
    static ON: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static LEVEL: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Allume / éteint le style verre pour toute l'app.
pub fn set(on: bool) {
    ON.with(|o| o.set(on));
}

/// Style verre allumé ?
pub fn on() -> bool {
    ON.with(|o| o.get())
}

/// 🫧 Qualité du verre, réglée toute seule (taille de l'écran et vitesse de l'appareil) : 0 = fine, 1 et 2 = grille du
/// flou 2 et 4 fois plus grossière (4 et 16 fois moins de calcul ; le flou cache la différence).
/// 🖐 Geste en cours (glisser, pincer) : le verre réutilise le flou de la dernière image au repos (même place, même
/// taille) au lieu de le recalculer à chaque image ; au repos, il est recalculé comme toujours (image identique).
/// Le geste commence (`true`) ou finit (`false`) (pour le fil qui dessine).
pub fn set_fige(f: bool) {
    FIGE.with(|c| c.set(f));
}

/// A gesture is under way: the glass reuses its last blur.
pub fn fige() -> bool {
    FIGE.with(|c| c.get())
}

/// Le flou réduit d'une carte (largeur de la petite grille, couleurs), par place, taille, thème et finesse.
type Flou = (usize, Vec<Rgb>);
/// 🚀 (fluidité, lot A) The glass of a card spread to every pixel (the bilinear mix of its small grid), kept with that
/// grid: (width of the grid, the grid, the glass of each pixel as 0x00RRGGBB).
type Etale = (usize, Vec<Rgb>, std::rc::Rc<Vec<u32>>);
thread_local! {
    static FIGE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static FLOUS: std::cell::RefCell<std::collections::HashMap<(usize, usize, usize, usize, bool, usize), Flou>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
    static ETALES: std::cell::RefCell<std::collections::HashMap<(usize, usize, usize, usize, bool, usize), Etale>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Niveau le plus grossier.
pub const MAX_LEVEL: usize = 2;

/// Règle la qualité du verre (0 à [`MAX_LEVEL`]).
pub fn set_level(k: usize) {
    LEVEL.with(|l| l.set(k.min(MAX_LEVEL)));
}

/// Qualité du verre en cours.
pub fn level() -> usize {
    LEVEL.with(|l| l.get())
}

/// Niveau de départ pour un écran de `pixels` pixels réels (grand écran : on commence plus grossier).
pub fn level_for(pixels: f64) -> usize {
    if pixels > 6_000_000.0 {
        2
    } else if pixels > 2_500_000.0 {
        1
    } else {
        0
    }
}

/// Pas de la grille du flou en pixels de l'écran (DOWN points × échelle, × 2 par niveau).
fn step() -> usize {
    ((DOWN as f32 * crate::px::scale()).round() as usize).max(1) << level()
}

/// Réduction de l'image avant le flou.
pub const DOWN: usize = 4;
/// Rayon du flou en boîte (pixels de l'image réduite : ≈ 12 px à l'écran).
pub const RADIUS: usize = 3;
/// Au-delà de cette surface (pixels), pas de flou : la teinte seule (limite de calcul par image).
pub const MAX_AREA: usize = 1_400_000;

/// Teinte du verre et son opacité : laiteux (clair) ou fumé (sombre).
pub fn tint(light: bool) -> (Rgb, f64) {
    // assez translucide pour voir la chart floutée derrière ; le contraste est garanti ensuite par `over`
    if light {
        (Rgb(0xf6, 0xf7, 0xfa), 0.55)
    } else {
        // recette de la maquette verre : rgba(20, 22, 30, .55)
        (Rgb(0x14, 0x16, 0x1e), 0.55)
    }
}

/// Texte le moins contrasté posé sur le verre (texte secondaire) : gris clair en sombre, gris foncé en clair.
pub fn weakest_text(light: bool) -> Rgb {
    if light {
        Rgb(0x5d, 0x60, 0x6b)
    } else {
        crate::palette::GREY
    }
}

/// Couleur du verre sur la couleur `c` de derrière : teinte, puis luminance bornée (≥ 4,5:1 pour le texte secondaire).
pub fn over(c: Rgb, light: bool) -> Rgb {
    let (t, a) = tint(light);
    let mix = |x: u8, y: u8| (x as f64 * a + y as f64 * (1.0 - a)).round().clamp(0.0, 255.0) as u8;
    let mut g = Rgb(mix(t.0, c.0), mix(t.1, c.1), mix(t.2, c.2));
    let txt = weakest_text(light);
    // assombrit (verre fumé) ou éclaircit (verre laiteux) par petits pas jusqu'au contraste voulu
    for _ in 0..40 {
        if contrast(txt, g) >= 4.6 {
            break;
        }
        g = if light {
            let up = |v: u8| v.saturating_add((255 - v) / 8 + 1);
            Rgb(up(g.0), up(g.1), up(g.2))
        } else {
            Rgb((g.0 as f64 * 0.88) as u8, (g.1 as f64 * 0.88) as u8, (g.2 as f64 * 0.88) as u8)
        };
    }
    g
}

fn rgb(p: u32) -> Rgb {
    Rgb((p >> 16) as u8, (p >> 8) as u8, p as u8)
}
fn pack(c: Rgb) -> u32 {
    (c.0 as u32) << 16 | (c.1 as u32) << 8 | c.2 as u32
}

/// Floute la zone (x, y, w, h) d'une image 0x00RRGGBB de largeur `ww` : réduction par [`DOWN`], flou en boîte de
/// rayon [`RADIUS`] (horizontal puis vertical), agrandissement. Rend les pixels floutés de la zone (w × h).
pub fn blur_region(buf: &[u32], ww: usize, wh: usize, x: usize, y: usize, w: usize, h: usize) -> Vec<Rgb> {
    blur_region_r(buf, ww, wh, x, y, w, h, RADIUS)
}

/// Comme [`blur_region`], avec le rayon `r` (pixels de l'image réduite : r × [`DOWN`] points à l'écran).
#[allow(clippy::too_many_arguments)]
pub fn blur_region_r(buf: &[u32], ww: usize, wh: usize, x: usize, y: usize, w: usize, h: usize, r: usize) -> Vec<Rgb> {
    let (w, h) = (w.min(ww.saturating_sub(x)), h.min(wh.saturating_sub(y)));
    if w == 0 || h == 0 {
        return vec![];
    }
    let (sw, small) = blur_small(buf, ww, x, y, w, h, r);
    let sh = small.len() / sw;
    let step = step();
    // agrandi en mélange bilinéaire (flou lisse, sans carrés)
    let pos = |k: usize, n: usize| {
        let f = ((k as f32 + 0.5) / step as f32 - 0.5).max(0.0);
        let a = (f.floor() as usize).min(n - 1);
        (a, (a + 1).min(n - 1), f - f.floor())
    };
    let cols: Vec<(usize, usize, f32)> = (0..w).map(|i| pos(i, sw)).collect();
    let mut out = Vec::with_capacity(w * h);
    for py in 0..h {
        let (r0, r1, ty) = pos(py, sh);
        for &(c0, c1, tx) in &cols {
            let p = |r: usize, c: usize| small[r * sw + c];
            let l = |k: fn(Rgb) -> u8| {
                let top = k(p(r0, c0)) as f32 + (k(p(r0, c1)) as f32 - k(p(r0, c0)) as f32) * tx;
                let bas = k(p(r1, c0)) as f32 + (k(p(r1, c1)) as f32 - k(p(r1, c0)) as f32) * tx;
                (top + (bas - top) * ty).round() as u8
            };
            out.push(Rgb(l(|c| c.0), l(|c| c.1), l(|c| c.2)));
        }
    }
    out
}

/// L'image réduite et floutée d'une zone (déjà bornée à l'image) : (largeur réduite, pixels).
fn blur_small(buf: &[u32], ww: usize, x: usize, y: usize, w: usize, h: usize, rayon: usize) -> (usize, Vec<Rgb>) {
    // un pixel pris au milieu de chaque bloc de DOWN points (DOWN × échelle pixels) : le coût ne dépend que de la
    // petite grille, même à ×3 sur iPhone ; le flou cache l'échantillonnage
    let step = step();
    let (sw, sh) = (w.div_ceil(step), h.div_ceil(step));
    let mut small = vec![[0f32; 3]; sw * sh];
    for sy in 0..sh {
        for sx in 0..sw {
            let (px, py) = ((x + sx * step + step / 2).min(x + w - 1), (y + sy * step + step / 2).min(y + h - 1));
            let c = rgb(buf[py * ww + px]);
            small[sy * sw + sx] = [c.0 as f32, c.1 as f32, c.2 as f32];
        }
    }
    // flou en boîte séparable (bords répétés)
    let pass = |src: &[[f32; 3]], horiz: bool| -> Vec<[f32; 3]> {
        let mut out = vec![[0f32; 3]; src.len()];
        let r = rayon as isize;
        for j in 0..sh {
            for i in 0..sw {
                let mut acc = [0f32; 3];
                for k in -r..=r {
                    let (ii, jj) = if horiz {
                        ((i as isize + k).clamp(0, sw as isize - 1) as usize, j)
                    } else {
                        (i, (j as isize + k).clamp(0, sh as isize - 1) as usize)
                    };
                    let c = src[jj * sw + ii];
                    acc[0] += c[0];
                    acc[1] += c[1];
                    acc[2] += c[2];
                }
                out[j * sw + i] = acc.map(|v| v / (2 * r + 1) as f32);
            }
        }
        out
    };
    let small = pass(&pass(&small, true), false);
    (sw, small.into_iter().map(|c| Rgb(c[0] as u8, c[1] as u8, c[2] as u8)).collect())
}

/// 🫧 Une couleur qui se voit comme `c` mais que `blit` ne prend pas pour le fond en verre (écart > 6) : pour la marge
/// et les coins autour d'une carte en verre, qui restent pleins.
pub fn hors_verre(c: Rgb) -> Rgb {
    let d = |v: u8| if v >= 4 { v - 4 } else { v + 4 };
    Rgb(d(c.0), d(c.1), d(c.2))
}

/// Pose une carte en style verre : là où la carte a sa couleur de fond (`key`, prise sur la carte elle-même), on voit
/// la chart floutée et teintée ; ailleurs (texte, icônes, boutons), la carte. Reflet clair en haut, ombre douce dessous.
pub fn blit(buf: &mut [u32], ww: usize, wh: usize, pm: &tiny_skia::Pixmap, x: f64, y: f64, light: bool) {
    let (pw, ph) = (pm.width() as usize, pm.height() as usize);
    // position en points → pixels de l'écran (l'image et le tampon sont en pixels)
    let (x0, y0) = (crate::px::dev(x).round().max(0.0) as usize, crate::px::dev(y).round().max(0.0) as usize);
    if x0 >= ww || y0 >= wh || pw == 0 || ph == 0 {
        return;
    }
    let (w, h) = (pw.min(ww - x0), ph.min(wh - y0));
    crate::px::compter_pixels((w * h) as u64);
    // ombre douce sous la carte (8 px qui s'estompent)
    for d in 1..=8usize {
        let yy = y0 + h + d - 1;
        if yy >= wh {
            break;
        }
        let k = 0.22 * (1.0 - d as f64 / 9.0);
        for xx in x0 + 6..(x0 + w).saturating_sub(6).min(ww) {
            let c = rgb(buf[yy * ww + xx]);
            let f = |v: u8| (v as f64 * (1.0 - k)) as u8;
            buf[yy * ww + xx] = pack(Rgb(f(c.0), f(c.1), f(c.2)));
        }
    }
    let px = pm.pixels();
    // couleur de fond de la carte : sa couleur la plus fréquente (relevée sur une grille)
    // (🚀 lot A: counted by sorting, not in a hash table; when two colours are found as often, the smaller one — the
    // table took either, by chance)
    let key = {
        let mut vues: Vec<u32> = Vec::with_capacity((ph / 7 + 1) * (pw / 7 + 1));
        for j in (0..ph).step_by(7) {
            for i in (0..pw).step_by(7) {
                let p = px[j * pw + i];
                if p.alpha() == 255 {
                    vues.push((p.red() as u32) << 16 | (p.green() as u32) << 8 | p.blue() as u32);
                }
            }
        }
        vues.sort_unstable();
        let mut best: Option<(usize, u32)> = None;
        for run in vues.chunk_by(|a, b| a == b) {
            if best.is_none_or(|(n, _)| run.len() > n) {
                best = Some((run.len(), run[0]));
            }
        }
        best.map_or(Rgb(0, 0, 0), |(_, c)| rgb(c))
    };
    // verre calculé sur l'image réduite (16 fois moins de pixels), puis étalé : peu de calcul par image
    let step = step();
    let cle = (x0, y0, w, h, light, step);
    let garde = fige().then(|| FLOUS.with(|f| f.borrow().get(&cle).cloned())).flatten();
    let (sw, glass_small) = if let Some(f) = garde {
        f
    } else if w * h <= MAX_AREA * (step * step) / (DOWN * DOWN) {
        let (sw, small) = blur_small(buf, ww, x0, y0, w, h, RADIUS);
        // une couleur de verre par teinte voisine (5 bits par couleur) : le fond, souvent uni, ne se calcule qu'une fois
        let mut memo: std::collections::HashMap<u32, Rgb> = std::collections::HashMap::new();
        let glass = small
            .into_iter()
            .map(|c| {
                let k = (c.0 as u32 >> 3) << 10 | (c.1 as u32 >> 3) << 5 | c.2 as u32 >> 3;
                *memo.entry(k).or_insert_with(|| over(c, light))
            })
            .collect::<Vec<_>>();
        // gardé pour le prochain geste (au repos seulement : c'est la dernière image juste)
        if !fige() {
            FLOUS.with(|f| {
                let mut f = f.borrow_mut();
                if f.len() > 64 {
                    f.clear();
                }
                f.insert(cle, (sw, glass.clone()));
            });
        }
        (sw, glass)
    } else {
        (1, vec![over(tint(light).0, light)])
    };
    // 🚀 (fluidité, lot A) the glass of every pixel: kept while the small grid stays the same (a menu or a list over
    // a chart that does not move), worked out again otherwise — the same numbers either way
    let etale = ETALES.with(|e| {
        let e = e.borrow();
        e.get(&cle)
            .filter(|(w0, g, _)| *w0 == sw && *g == glass_small)
            .map(|(_, _, v)| std::rc::Rc::clone(v))
    });
    let etale = etale.unwrap_or_else(|| {
        let v = std::rc::Rc::new(etaler(sw, &glass_small, w, h, step));
        ETALES.with(|e| {
            let mut e = e.borrow_mut();
            if e.len() > 16 {
                e.clear();
            }
            e.insert(cle, (sw, glass_small.clone(), std::rc::Rc::clone(&v)));
        });
        v
    });
    let near = |c: Rgb| (c.0 as i32 - key.0 as i32).abs() + (c.1 as i32 - key.1 as i32).abs() + (c.2 as i32 - key.2 as i32).abs() <= 6;
    // (the card's own top and left edges: the first glass pixel of each column, of each row)
    let mut col_vue = vec![false; w];
    let donnees = pm.data();
    for j in 0..h {
        let mut ligne_vue = false;
        let verre = &etale[j * w..][..w];
        let ligne = &donnees[j * pw * 4..][..w * 4];
        let sortie = &mut buf[(y0 + j) * ww + x0..][..w];
        for (i, (q, o)) in ligne.as_chunks::<4>().0.iter().zip(sortie.iter_mut()).enumerate() {
            // (🚀 lot A: read straight from the image's bytes; a full pixel is its own colour)
            let alpha = q[3];
            if alpha == 0 {
                continue; // hors de la carte (coins arrondis)
            }
            let p = if alpha == 255 {
                tiny_skia::ColorU8::from_rgba(q[0], q[1], q[2], 255)
            } else {
                px[j * pw + i].demultiply()
            };
            let c = Rgb(p.red(), p.green(), p.blue());
            let pres = near(c);
            // 🚀 (fluidité, lot A) the card's own full pixels (text, icons, buttons): the card itself, as the mix below
            // gives at full alpha, without working out the glass under them
            if alpha == 255 && !pres {
                *o = pack(c);
                continue;
            }
            let glass = rgb(verre[i]);
            // fond de la carte : le verre ; le reste par-dessus le verre (bords lissés compris)
            let mut out = if alpha == 255 {
                // (🚀 a full glass pixel: the mix at full alpha is the glass itself)
                glass
            } else {
                let a = alpha as f64 / 255.0;
                let top = if pres { glass } else { c };
                let mix = |f: u8, b: u8| arrondi64(f as f64 * a + b as f64 * (1.0 - a));
                let under = if pres { rgb(*o) } else { glass };
                Rgb(mix(top.0, under.0), mix(top.1, under.1), mix(top.2, under.2))
            };
            // reflet : une ligne claire sur le haut et, plus légère, à gauche — of the glass card itself (its first
            // glass pixel from the top / from the left), never the image's straight edges: a card drawn inside a
            // rectangle (the floating side panel) got a square frame from them (Jee, 17 Nov)
            let bord_haut = near(c) && p.alpha() == 255 && !col_vue[i];
            let bord_gauche = near(c) && p.alpha() == 255 && !ligne_vue;
            if bord_haut {
                col_vue[i] = true;
            }
            if bord_gauche {
                ligne_vue = true;
            }
            let shine = if bord_haut {
                if light {
                    0.7
                } else {
                    0.28
                }
            } else if bord_gauche && j < h / 2 {
                if light {
                    0.4
                } else {
                    0.12
                }
            } else {
                0.0
            };
            if shine > 0.0 && p.alpha() == 255 {
                let s = |v: u8| (v as f64 + (255.0 - v as f64) * shine) as u8;
                out = Rgb(s(out.0), s(out.1), s(out.2));
            }
            // liseré de 1 px : blanc à 12 % sur le tour de la carte (bord de l'image ou voisin hors de la carte) — the
            // glass card's own edge only (not a straight edge of the image around it)
            if p.alpha() == 255 && near(c) && on_edge(px, pw, ph, i, j) {
                let s = |v: u8| (v as f64 + (255.0 - v as f64) * EDGE_ALPHA) as u8;
                out = Rgb(s(out.0), s(out.1), s(out.2));
            }
            *o = pack(out);
        }
    }
}

/// 🫧 The glass of each pixel of a `w` × `h` card from its small grid (`sw` wide, a cell every `step` pixels): between
/// the cells a bilinear mix (smooth glass, no squares); horizontal weights worked out once, the vertical mix once a row.
fn etaler(sw: usize, glass_small: &[Rgb], w: usize, h: usize, step: usize) -> Vec<u32> {
    let sh = glass_small.len() / sw.max(1);
    let pos = |k: usize, n: usize| {
        let f = ((k as f32 + 0.5) / step as f32 - 0.5).max(0.0);
        let a = (f.floor() as usize).min(n - 1);
        (a, (a + 1).min(n - 1), f - f.floor())
    };
    let cols: Vec<(usize, usize, f32)> = (0..w).map(|i| pos(i, sw)).collect();
    let mut row: Vec<[f32; 3]> = vec![[0.0; 3]; sw];
    let mut out = Vec::with_capacity(w * h);
    for j in 0..h {
        let (r0, r1, ty) = pos(j, sh);
        for (x, cell) in row.iter_mut().enumerate() {
            let (a, b) = (glass_small[r0 * sw + x], glass_small[r1 * sw + x]);
            let l = |p: u8, q: u8| p as f32 + (q as f32 - p as f32) * ty;
            *cell = [l(a.0, b.0), l(a.1, b.1), l(a.2, b.2)];
        }
        out.extend(cols.iter().map(|&(c0, c1, tx)| {
            let (a, b) = (row[c0], row[c1]);
            let l = |k: usize| arrondi(a[k] + (b[k] - a[k]) * tx);
            pack(Rgb(l(0), l(1), l(2)))
        }));
    }
    out
}

/// 🚀 (fluidité, lot A) `v.round() as u8` for a glass value (0 to 255), without the call `round` costs in the browser
/// (WebAssembly has no rounding half away from zero): its whole part, plus one from a half up — the same number.
#[inline]
fn arrondi(v: f32) -> u8 {
    if !(0.0..256.0).contains(&v) {
        return v.round() as u8;
    }
    let t = v as u32 as f32;
    (t as u32 + (v - t >= 0.5) as u32).min(255) as u8
}

/// 🚀 (fluidité, lot A) `v.round() as u8` for a mix of two colours (0 to 255), without the call `round` costs in the
/// browser: the same number (see [`arrondi`]).
#[inline]
fn arrondi64(v: f64) -> u8 {
    if !(0.0..256.0).contains(&v) {
        return v.round() as u8;
    }
    let t = v as u32 as f64;
    (t as u32 + (v - t >= 0.5) as u32).min(255) as u8
}

/// Opacité du liseré blanc de 1 px autour des cartes en verre (maquette : rgba(255, 255, 255, .12)).
pub const EDGE_ALPHA: f64 = 0.12;

/// Le pixel (i, j) d'une carte est-il sur son tour (bord de l'image, ou un voisin transparent : coin arrondi) ?
fn on_edge(px: &[tiny_skia::PremultipliedColorU8], pw: usize, ph: usize, i: usize, j: usize) -> bool {
    if i == 0 || j == 0 || i + 1 >= pw || j + 1 >= ph {
        return true;
    }
    [(i - 1, j), (i + 1, j), (i, j - 1), (i, j + 1)]
        .iter()
        .any(|&(x, y)| px[y * pw + x].alpha() < 255)
}

/// Luminance relative (pour les tests et la mesure du contraste).
pub fn lum(c: Rgb) -> f64 {
    luminance(c)
}

#[cfg(test)]
mod tests {

    #[test]
    fn qualite_selon_l_ecran_et_niveaux() {
        use super::*;
        assert_eq!(level_for(375.0 * 812.0 * 9.0), 1, "iPhone ×3 : 2,7 M pixels");
        assert_eq!(level_for(1280.0 * 800.0), 0, "écran d'ordi ordinaire : fin");
        assert_eq!(level_for(2560.0 * 1600.0 * 4.0), 2, "grand écran Retina : grossier d'abord");
        let before = level();
        set_level(0);
        let fine = step();
        set_level(2);
        assert_eq!(step(), fine * 4, "niveau 2 : grille 4 fois plus grossière");
        set_level(9);
        assert_eq!(level(), MAX_LEVEL);
        set_level(before);
    }
    use super::*;

    #[test]
    fn reglage_et_bornes_du_flou() {
        let was = on();
        set(true);
        assert!(on());
        set(false);
        assert!(!on());
        set(was);
        assert!((2..=6).contains(&RADIUS) && (2..=8).contains(&DOWN), "flou borné (coût par image)");
        const { assert!(MAX_AREA <= 2_000_000) };
        // une image noire avec un point blanc : le flou l'étale sans dépasser la zone
        let (ww, wh) = (40usize, 40usize);
        let mut buf = vec![0u32; ww * wh];
        for (dx, dy) in (0..4).flat_map(|a| (0..4).map(move |b| (a, b))) {
            buf[(20 + dy) * ww + 20 + dx] = 0x00ff_ffff;
        }
        let b = blur_region(&buf, ww, wh, 8, 8, 24, 24);
        assert_eq!(b.len(), 24 * 24);
        let max = b.iter().map(|c| c.0).max().unwrap();
        assert!(max > 0 && max < 255, "le point est étalé : {max}");
        assert!(blur_region(&buf, ww, wh, 50, 50, 10, 10).is_empty(), "zone hors de l'image");
    }

    #[test]
    fn contraste_sur_le_verre_dans_le_pire_cas() {
        // derrière le verre : tout ce que la chart peut montrer, du noir au blanc en passant par les couleurs vives
        let backs = [
            Rgb(0, 0, 0),
            Rgb(255, 255, 255),
            Rgb(0xfd, 0xe7, 0x25),
            Rgb(0x08, 0x99, 0x81),
            Rgb(0xf2, 0x36, 0x45),
            Rgb(0x29, 0x62, 0xff),
            Rgb(0x9c, 0x27, 0xb0),
        ];
        for light in [false, true] {
            for &b in &backs {
                let g = over(b, light);
                let txt = weakest_text(light);
                assert!(
                    contrast(txt, g) >= 4.5,
                    "verre {} sur {b:?} : {:.2}",
                    if light { "laiteux" } else { "fumé" },
                    contrast(txt, g)
                );
                // le texte principal, encore mieux
                let main = if light { Rgb(0x13, 0x17, 0x22) } else { crate::palette::WHITE };
                assert!(contrast(main, g) >= 7.0);
            }
        }
        // laiteux en clair, fumé en sombre
        assert!(lum(over(Rgb(128, 128, 128), true)) > 0.5);
        assert!(lum(over(Rgb(128, 128, 128), false)) < 0.1);
    }

    #[test]
    fn carte_posee_en_verre() {
        // fond de chart rayé ; carte : fond uni (la clé) avec un trait de texte au milieu
        let (ww, wh) = (60usize, 40usize);
        let mut buf: Vec<u32> = (0..ww * wh).map(|i| if (i % ww) % 6 < 3 { 0x00ff_ffff } else { 0x0010_1010 }).collect();
        let mut pm = tiny_skia::Pixmap::new(30, 20).unwrap();
        pm.fill(tiny_skia::Color::from_rgba8(30, 34, 45, 255));
        let white = tiny_skia::ColorU8::from_rgba(230, 232, 238, 255).premultiply();
        for x in 10..20 {
            pm.pixels_mut()[10 * 30 + x] = white;
        }
        blit(&mut buf, ww, wh, &pm, 10.0, 10.0, false);
        let at = |x: usize, y: usize| rgb(buf[y * ww + x]);
        assert_eq!(at(25, 20), Rgb(230, 232, 238), "le texte de la carte reste");
        let g = at(14, 15);
        assert_ne!(g, Rgb(30, 34, 45), "le fond devient du verre");
        assert!(contrast(crate::palette::GREY, g) >= 4.5);
        // les rayures derrière sont adoucies (flou) : deux pixels voisins du verre se ressemblent
        let (a, b) = (at(13, 16), at(14, 16));
        assert!((a.0 as i32 - b.0 as i32).abs() < 40, "{a:?} / {b:?}");
        assert_eq!(at(1, 5), rgb(0x00ff_ffff), "hors de la carte : inchangé");
    }

    /// 🖐 Pendant un geste, le verre réutilise le flou de la dernière image au repos ; au repos, l'image est la même au
    /// pixel près que sans cette mémoire.
    #[test]
    fn verre_fige_pendant_le_geste_identique_au_repos() {
        let (ww, wh) = (60usize, 40usize);
        let fond = |decale: usize| -> Vec<u32> {
            (0..ww * wh)
                .map(|i| if ((i % ww) + decale) % 6 < 3 { 0x00ff_ffff } else { 0x0010_1010 })
                .collect()
        };
        let mut pm = tiny_skia::Pixmap::new(30, 20).unwrap();
        pm.fill(tiny_skia::Color::from_rgba8(30, 34, 45, 255));
        let pose = |b: &mut Vec<u32>| blit(b, ww, wh, &pm, 10.0, 10.0, false);
        let carte = |b: &[u32]| -> Vec<u32> { (10..30).flat_map(|y| b[y * ww + 10..y * ww + 40].to_vec()).collect() };
        set_fige(false);
        // au repos : deux fois la même image (et la mémoire est remplie)
        let (mut a, mut a2) = (fond(0), fond(0));
        pose(&mut a);
        pose(&mut a2);
        assert_eq!(a, a2);
        // la chart glisse sous la carte : au repos, le verre suit
        let mut juste = fond(3);
        pose(&mut juste);
        assert_ne!(carte(&juste), carte(&a), "le fond a changé : le verre aussi");
        // pendant le geste : le flou d'avant est réutilisé (pas recalculé)
        let mut b = fond(0);
        pose(&mut b);
        set_fige(true);
        let mut g = fond(3);
        pose(&mut g);
        assert_eq!(carte(&g), carte(&b), "pendant le geste : le verre de la dernière image au repos");
        // au lâcher : recalculé, identique au pixel près à l'image sans mémoire
        set_fige(false);
        let mut r = fond(3);
        pose(&mut r);
        assert_eq!(r, juste, "au repos : identique au pixel près");
    }

    /// 🚀 (fluidité, lot A) `arrondi64` is `round() as u8`: every f64 near each whole number and each half from 0 to
    /// 256 (where rounding turns), many in between, and the odd ones.
    #[test]
    fn arrondi64_same_as_round() {
        for k in 0..=512u32 {
            let b = (k as f64 / 2.0).to_bits();
            for d in 0..=2000u64 {
                for v in [f64::from_bits(b.saturating_sub(d)), f64::from_bits(b + d)] {
                    assert_eq!(arrondi64(v), v.round() as u8, "{v}");
                }
            }
        }
        for k in 0..2_000_000u32 {
            let v = k as f64 * (256.0 / 2_000_000.0) + 1e-7 * (k % 7) as f64;
            assert_eq!(arrondi64(v), v.round() as u8, "{v}");
        }
        for v in [f64::NAN, -0.4, -0.5, -3.7, 1e12, f64::INFINITY, f64::NEG_INFINITY, 255.5, 255.49999999999997] {
            assert_eq!(arrondi64(v), v.round() as u8, "{v}");
        }
    }

    /// 🚀 (fluidité, lot A) `arrondi` is `round() as u8`: every f32 within 4096 steps of each whole number and each
    /// half from 0 to 256 (where rounding turns), one in 97 elsewhere, and the odd ones (negative, NaN, huge).
    #[test]
    fn arrondi_same_as_round() {
        let pres = (0..=512u32).flat_map(|k| {
            let b = (k as f32 / 2.0).to_bits();
            b.saturating_sub(4096)..=b + 4096
        });
        let ailleurs = (0..=256.0f32.to_bits() + 1000).step_by(97);
        let bizarres = [f32::NAN, -0.4, -0.5, -3.7, 1e9, f32::INFINITY, f32::NEG_INFINITY].map(f32::to_bits);
        for b in pres.chain(ailleurs).chain(bizarres) {
            let v = f32::from_bits(b);
            assert_eq!(arrondi(v), v.round() as u8, "{v}");
        }
    }
}
