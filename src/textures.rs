//! Textures procédurales basse résolution (64x64 / 32x32), générées au démarrage.

use crate::math::*;
use crate::raster::Texture;

pub const T_WOOD: i16 = 0;
pub const T_MARBLE: i16 = 1;
pub const T_PAPER_RED: i16 = 2;
pub const T_WAINSCOT: i16 = 3;
pub const T_CEILING: i16 = 4;
pub const T_STONE: i16 = 5;
pub const T_DOOR: i16 = 6;
pub const T_BOOKS: i16 = 7;
pub const T_CARPET: i16 = 8;
pub const T_CLOTH: i16 = 9;
pub const T_PAINTING: i16 = 10;
pub const T_GRASS: i16 = 11;
pub const T_METAL: i16 = 12;
pub const T_FACE: i16 = 13;
pub const T_ZFACE: i16 = 14;
pub const T_BARK: i16 = 15;
pub const T_PAPER_GREEN: i16 = 16;
pub const T_MEDKIT: i16 = 17;
pub const T_NOTE: i16 = 18;
pub const T_CURTAIN: i16 = 19;
pub const T_HAIR: i16 = 20;
pub const T_FLAME: i16 = 21;
pub const T_PANEL: i16 = 22;
pub const T_AMMO: i16 = 23;
pub const T_BLOODY: i16 = 24;
pub const T_GLINT: i16 = 25;
pub const T_LANDSCAPE: i16 = 26;
pub const T_PAPER_BLUE: i16 = 27;
pub const T_KEY: i16 = 28;
pub const T_FABRIC: i16 = 29;
pub const T_VEST: i16 = 30;

type C = [f32; 3];

fn mix(a: C, b: C, t: f32) -> C {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}
fn sc(a: C, s: f32) -> C {
    [a[0] * s, a[1] * s, a[2] * s]
}

fn gen(w: usize, h: usize, f: impl Fn(usize, usize) -> C) -> Texture {
    let mut t = Texture::new(w, h);
    for y in 0..h {
        for x in 0..w {
            t.set(x, y, f(x, y));
        }
    }
    t
}

fn wood(x: usize, y: usize, seed: u32, base: C) -> C {
    let plank = y / 16;
    let tone = 0.75 + 0.35 * hash2(plank as i32, 0, seed);
    let seam = (hash2(plank as i32, 7, seed) * 64.0) as usize;
    let g = fbm(x as f32 / 22.0, y as f32 / 2.5 + plank as f32 * 10.0, 3, seed);
    let grain = 0.7 + 0.45 * ((x as f32 * 0.18 + g * 9.0).sin() * 0.5 + 0.5);
    let mut c = sc(base, tone * grain);
    if y % 16 == 0 || x == seam {
        c = sc(c, 0.35);
    } else if y % 16 == 1 {
        c = sc(c, 1.15);
    }
    c
}

fn wallpaper(x: usize, y: usize, base: C, motif: C, seed: u32) -> C {
    // Damassé : losanges répétés + rayures fines.
    let fx = (x % 16) as f32 - 7.5;
    let fy = (y % 32) as f32 - 15.5;
    let diamond = (fx.abs() / 7.5 + fy.abs() / 15.5) < 0.75;
    let inner = (fx.abs() / 7.5 + fy.abs() / 15.5) < 0.35;
    let stripe = x % 16 == 0;
    let mut c = base;
    if diamond && !inner {
        c = mix(c, motif, 0.3);
    }
    if stripe {
        c = sc(c, 0.7);
    }
    let stain = fbm(x as f32 / 12.0, y as f32 / 12.0, 3, seed);
    sc(c, 0.65 + 0.45 * stain)
}

pub fn build() -> Vec<Texture> {
    let mut v: Vec<Texture> = Vec::new();

    // T_WOOD : parquet.
    v.push(gen(64, 64, |x, y| wood(x, y, 1, [0.46, 0.29, 0.16])));

    // T_MARBLE : damier usé.
    v.push(gen(64, 64, |x, y| {
        let tile = ((x / 32) + (y / 32)) % 2 == 0;
        let n = fbm(x as f32 / 9.0, y as f32 / 9.0, 4, 2);
        let vein = ((x as f32 * 0.12 + y as f32 * 0.05 + n * 7.0).sin()).abs();
        let mut c: C = if tile { [0.78, 0.75, 0.68] } else { [0.18, 0.17, 0.17] };
        if vein < 0.08 {
            c = if tile { sc(c, 0.75) } else { [0.32, 0.30, 0.30] };
        }
        c = sc(c, 0.8 + 0.25 * n);
        if x % 32 == 0 || y % 32 == 0 {
            c = [0.1, 0.09, 0.08];
        }
        c
    }));

    // T_PAPER_RED : papier peint bordeaux.
    v.push(gen(32, 64, |x, y| wallpaper(x, y, [0.36, 0.07, 0.08], [0.62, 0.36, 0.18], 3)));

    // T_WAINSCOT : lambris.
    v.push(gen(64, 32, |x, y| {
        let base = wood(x, y * 2 % 64, 4, [0.30, 0.17, 0.09]);
        let px = x % 32;
        let border = px < 3 || px > 28 || y < 4 || y > 27;
        let bevel = px == 3 || y == 4;
        let shadow = px == 28 || y == 27;
        if bevel {
            sc(base, 1.4)
        } else if shadow {
            sc(base, 0.4)
        } else if border {
            sc(base, 0.85)
        } else {
            base
        }
    }));

    // T_CEILING : plafond à caissons.
    v.push(gen(64, 64, |x, y| {
        let n = fbm(x as f32 / 8.0, y as f32 / 8.0, 3, 5);
        let beam = x % 32 < 5 || y % 32 < 5;
        let c: C = if beam { [0.22, 0.13, 0.08] } else { [0.30, 0.25, 0.20] };
        sc(c, 0.7 + 0.4 * n)
    }));

    // T_STONE : moellons.
    v.push(gen(64, 64, |x, y| {
        let row = y / 16;
        let off = if row % 2 == 0 { 0 } else { 12 };
        let bx = (x + off) / 24;
        let n = fbm(x as f32 / 6.0, y as f32 / 6.0, 4, 6);
        let tone = 0.75 + 0.3 * hash2(bx as i32, row as i32, 6);
        let mortar = y % 16 == 0 || (x + off) % 24 == 0;
        if mortar {
            [0.12, 0.11, 0.1]
        } else {
            sc([0.45, 0.43, 0.40], tone * (0.75 + 0.4 * n))
        }
    }));

    // T_DOOR : porte à panneaux (64 de large, 128 de haut).
    v.push(gen(64, 128, |x, y| {
        let base = wood(x, (y / 2) % 64, 7, [0.38, 0.21, 0.11]);
        let in_panel = |x0: usize, y0: usize, x1: usize, y1: usize| x >= x0 && x <= x1 && y >= y0 && y <= y1;
        let panels = [(8, 8, 28, 56), (36, 8, 56, 56), (8, 68, 28, 118), (36, 68, 56, 118)];
        let mut c = base;
        for &(x0, y0, x1, y1) in &panels {
            if in_panel(x0, y0, x1, y1) {
                if x == x0 || y == y0 {
                    c = sc(base, 0.45);
                } else if x == x1 || y == y1 {
                    c = sc(base, 1.4);
                } else {
                    c = sc(base, 0.85);
                }
            }
        }
        // Poignée en laiton.
        let dx = x as f32 - 58.0;
        let dy = y as f32 - 64.0;
        if dx * dx + dy * dy < 7.0 {
            c = if dx + dy < -1.0 { [0.95, 0.8, 0.4] } else { [0.6, 0.45, 0.15] };
        }
        if x < 2 || x > 61 {
            c = sc(c, 0.5);
        }
        c
    }));

    // T_BOOKS : rayonnages.
    v.push(gen(64, 64, |x, y| {
        let shelf = y / 16;
        let ly = y % 16;
        if ly >= 13 {
            return wood(x, y, 8, [0.25, 0.14, 0.07]);
        }
        // Largeur de livres pseudo-aléatoire.
        let mut bx = 0usize;
        let mut idx = 0i32;
        loop {
            let wdt = 2 + (hash2(idx, shelf as i32, 9) * 4.0) as usize;
            if x < bx + wdt {
                break;
            }
            bx += wdt;
            idx += 1;
        }
        let top = 1 + (hash2(idx, shelf as i32, 10) * 5.0) as usize;
        if ly < top {
            return [0.03, 0.02, 0.02];
        }
        let palette: [C; 6] = [
            [0.45, 0.08, 0.06],
            [0.10, 0.25, 0.12],
            [0.12, 0.14, 0.35],
            [0.40, 0.28, 0.12],
            [0.20, 0.10, 0.20],
            [0.50, 0.42, 0.30],
        ];
        let c = palette[(hash2(idx, shelf as i32, 11) * 6.0) as usize % 6];
        let mut c = sc(c, 0.7 + 0.5 * hash2(idx, shelf as i32, 12));
        if ly == top + 2 || ly == 10 {
            c = [0.7, 0.55, 0.2];
        }
        if x == bx {
            c = sc(c, 0.5);
        }
        c
    }));

    // T_CARPET : tapis rouge à motifs.
    v.push(gen(32, 32, |x, y| {
        let border = x < 3 || x > 28;
        let fx = (x as f32 - 15.5).abs();
        let fy = ((y % 16) as f32 - 7.5).abs();
        let n = fbm(x as f32 / 5.0, y as f32 / 5.0, 2, 13);
        let mut c: C = [0.45, 0.06, 0.07];
        if border {
            c = [0.55, 0.42, 0.15];
        } else if (fx / 10.0 + fy / 7.5) < 0.6 && (fx / 10.0 + fy / 7.5) > 0.4 {
            c = [0.6, 0.45, 0.18];
        } else if (fx / 10.0 + fy / 7.5) < 0.25 {
            c = [0.08, 0.10, 0.25];
        }
        sc(c, 0.75 + 0.35 * n)
    }));

    // T_CLOTH : nappe blanche tachée.
    v.push(gen(32, 32, |x, y| {
        let fold = ((x as f32 * 0.4).sin() * 0.5 + 0.5) * 0.15;
        let stain = fbm(x as f32 / 7.0, y as f32 / 7.0, 3, 14);
        let mut c = sc([0.85, 0.82, 0.74], 0.85 + fold);
        if stain > 0.68 {
            c = mix(c, [0.45, 0.12, 0.08], 0.6);
        }
        c
    }));

    // T_PAINTING : portrait inquiétant (32x48).
    v.push(gen(32, 48, |x, y| {
        let frame = x < 3 || x > 28 || y < 3 || y > 44;
        if frame {
            let b = if x == 1 || y == 1 { 1.3 } else { 0.9 };
            return sc([0.6, 0.45, 0.15], b);
        }
        let n = fbm(x as f32 / 6.0, y as f32 / 6.0, 3, 15);
        let mut c: C = sc([0.12, 0.08, 0.05], 0.7 + 0.6 * n);
        let fx = (x as f32 - 15.5) / 6.0;
        let fy = (y as f32 - 17.0) / 8.0;
        if fx * fx + fy * fy < 1.0 {
            c = sc([0.75, 0.66, 0.55], 0.75 + 0.2 * n);
            // Yeux noirs.
            if (y == 15 || y == 16) && (x == 13 || x == 18) {
                c = [0.0, 0.0, 0.0];
            }
            if y == 22 && (14..=17).contains(&x) {
                c = [0.25, 0.05, 0.05];
            }
        }
        // Épaules / habit sombre.
        let sy = y as f32 - 34.0;
        if sy > 0.0 && (x as f32 - 15.5).abs() < 4.0 + sy * 1.5 {
            c = sc([0.08, 0.08, 0.12], 0.8 + 0.4 * n);
        }
        c
    }));

    // T_GRASS : herbe de marais.
    v.push(gen(32, 32, |x, y| {
        let n = fbm(x as f32 / 4.0, y as f32 / 4.0, 3, 16);
        let blade = hash2(x as i32, y as i32, 17);
        let c: C = mix([0.10, 0.14, 0.06], [0.22, 0.28, 0.12], n);
        sc(c, 0.75 + 0.5 * blade)
    }));

    // T_METAL.
    v.push(gen(16, 16, |x, y| {
        let n = hash2(x as i32, y as i32, 18);
        sc([0.42, 0.42, 0.45], 0.8 + 0.3 * n)
    }));

    // T_FACE : tête de l'héroïne enroulée sur 8 pans (32x22, u = 0,5 au milieu du visage) :
    // visage au centre, frange, yeux enfoncés, traits sobres ; joues et nuque sur les côtés.
    // Les 6 dernières lignes forment le menton.
    v.push(gen(32, 22, |x, y| {
        let skin: C = [0.80, 0.62, 0.50];
        let hair: C = [0.16, 0.10, 0.07];
        let n = hash2(x as i32, y as i32, 32);
        // Arrière et côtés : cheveux (carré court jusqu'à la mâchoire).
        if !(8..24).contains(&x) {
            return sc(hair, 0.8 + 0.5 * n);
        }
        if y >= 16 {
            let edge = (x as f32 - 15.5).abs() / 16.0;
            return sc(skin, (1.0 - 0.04 * (y - 16) as f32 - 0.35 * edge * edge) * (0.96 + 0.06 * n));
        }
        let x = x - 8;
        // Frange en mèches irrégulières, cheveux sur les tempes.
        let fringe = 3 + (hash2(x as i32, 0, 33) * 2.5) as usize;
        if y < fringe || (x < 2 && y < 12) || (x > 13 && y < 12) {
            return sc(hair, 0.8 + 0.5 * n);
        }
        // Ombre des arcades et sourcils fins.
        if y == 6 && ((4..=6).contains(&x) || (9..=11).contains(&x)) {
            return mix(hair, skin, 0.35);
        }
        if y == 7 && (x == 3 || x == 12) {
            return sc(skin, 0.75);
        }
        if y == 8 && ((4..=5).contains(&x) || (10..=11).contains(&x)) {
            return if x == 5 || x == 10 { [0.12, 0.16, 0.18] } else { [0.78, 0.74, 0.70] };
        }
        if y == 8 && (x == 3 || x == 6 || x == 9 || x == 12) {
            return sc(skin, 0.75);
        }
        // Arête du nez éclairée, narines dans l'ombre.
        if (8..=10).contains(&y) && x == 7 {
            return sc(skin, 1.08);
        }
        if y == 11 && (7..=8).contains(&x) {
            return sc(skin, 0.7);
        }
        // Bouche fine.
        if y == 13 && (6..=9).contains(&x) {
            return if x == 6 || x == 9 { sc(skin, 0.8) } else { [0.68, 0.36, 0.36] };
        }
        if y == 14 && (7..=8).contains(&x) {
            return sc(skin, 0.86);
        }
        // Pommettes et contour du visage ombrés.
        let edge = (x as f32 - 7.5).abs() / 7.5;
        let jaw = if y >= 15 { 0.85 } else { 1.0 };
        sc(skin, (1.04 - 0.3 * edge * edge) * jaw * (0.96 + 0.06 * n))
    }));

    // T_ZFACE : visage de zombie.
    v.push(gen(16, 16, |x, y| {
        let n = hash2(x as i32, y as i32, 19);
        let skin: C = sc([0.48, 0.52, 0.40], 0.8 + 0.3 * n);
        if y < 3 {
            return sc([0.15, 0.13, 0.1], 0.8 + 0.4 * n);
        }
        if (7..=9).contains(&y) && ((3..=6).contains(&x) || (9..=12).contains(&x)) {
            if y == 8 && (x == 5 || x == 10) {
                return [0.8, 0.75, 0.3];
            }
            return [0.05, 0.03, 0.03];
        }
        if (12..=14).contains(&y) && (5..=10).contains(&x) {
            return if y == 12 { [0.7, 0.65, 0.5] } else { [0.4, 0.04, 0.04] };
        }
        if y > 13 && n > 0.5 {
            return [0.45, 0.05, 0.05];
        }
        skin
    }));

    // T_BARK : écorce.
    v.push(gen(16, 32, |x, y| {
        let n = fbm(x as f32 / 2.0, y as f32 / 9.0, 3, 20);
        sc([0.20, 0.16, 0.12], 0.5 + 0.8 * n)
    }));

    // T_PAPER_GREEN : papier peint vert.
    v.push(gen(32, 64, |x, y| wallpaper(x, y, [0.10, 0.20, 0.13], [0.42, 0.40, 0.22], 21)));

    // T_MEDKIT : trousse de soins (blanche, croix verte).
    v.push(gen(16, 16, |x, y| {
        let cross = ((6..=9).contains(&x) && (3..=12).contains(&y)) || ((3..=12).contains(&x) && (6..=9).contains(&y));
        if cross {
            [0.15, 0.7, 0.25]
        } else if x == 0 || y == 0 || x == 15 || y == 15 {
            [0.6, 0.6, 0.6]
        } else {
            [0.92, 0.92, 0.9]
        }
    }));

    // T_NOTE : papier manuscrit.
    v.push(gen(16, 16, |x, y| {
        let line = y % 3 == 2 && x > 1 && x < 14 && hash2(x as i32, y as i32, 22) > 0.25;
        if line {
            [0.2, 0.15, 0.25]
        } else {
            [0.85, 0.80, 0.65]
        }
    }));

    // T_CURTAIN : velours à plis.
    v.push(gen(32, 32, |x, y| {
        let fold = (x as f32 * 0.6).sin() * 0.5 + 0.5;
        let n = hash2(x as i32, y as i32, 23);
        sc([0.40, 0.05, 0.08], 0.45 + 0.65 * fold + 0.1 * n)
    }));

    // T_HAIR : cheveux.
    v.push(gen(16, 16, |x, y| {
        let n = hash2(x as i32, (y / 4) as i32, 24);
        sc([0.24, 0.13, 0.07], 0.7 + 0.5 * n)
    }));

    // T_FLAME : flamme (additive, le noir est neutre).
    v.push(gen(32, 32, |x, y| {
        let fx = (x as f32 - 15.5) / 16.0;
        let fy = y as f32 / 32.0; // 0 haut, 1 bas
        let width = (fy * 1.1).min(1.0) * (1.0 - fy * 0.35);
        let d = fx.abs() / width.max(0.01);
        if d > 1.0 {
            return [0.0, 0.0, 0.0];
        }
        let k = (1.0 - d) * (0.3 + fy);
        let core = (1.0 - d * 1.6).max(0.0) * fy;
        [k * 1.0 + core, k * 0.45 + core * 0.8, k * 0.08 + core * 0.4]
    }));

    // T_PANEL : boiseries sombres.
    v.push(gen(32, 64, |x, y| {
        let base = wood(y, x * 2, 25, [0.24, 0.14, 0.08]);
        let px = x % 32;
        let py = y % 32;
        if px == 2 || py == 2 {
            sc(base, 1.4)
        } else if px == 29 || py == 29 {
            sc(base, 0.4)
        } else {
            base
        }
    }));

    // T_AMMO : boîte de munitions.
    v.push(gen(16, 16, |x, y| {
        if y > 4 && y < 11 && x > 1 && x < 14 {
            if y == 7 && x % 2 == 0 {
                [0.1, 0.1, 0.1]
            } else {
                [0.85, 0.7, 0.2]
            }
        } else {
            sc([0.18, 0.28, 0.14], 0.85 + 0.2 * hash2(x as i32, y as i32, 26))
        }
    }));

    // T_BLOODY : chemise déchirée et ensanglantée.
    v.push(gen(32, 32, |x, y| {
        let n = fbm(x as f32 / 6.0, y as f32 / 6.0, 3, 27);
        let mut c = sc([0.55, 0.52, 0.45], 0.7 + 0.4 * n);
        if n > 0.58 {
            c = mix(c, [0.35, 0.03, 0.03], ((n - 0.58) * 6.0).min(1.0));
        }
        if hash2(x as i32, y as i32, 28) > 0.93 {
            c = [0.05, 0.04, 0.04];
        }
        c
    }));

    // T_GLINT : étincelle (additive).
    v.push(gen(16, 16, |x, y| {
        let fx = (x as f32 - 7.5).abs();
        let fy = (y as f32 - 7.5).abs();
        let k = ((1.0 - fx / 8.0) * (fy < 1.0) as i32 as f32).max((1.0 - fy / 8.0) * (fx < 1.0) as i32 as f32);
        let r = (1.0 - (fx * fx + fy * fy).sqrt() / 4.0).max(0.0);
        let v = (k + r).min(1.0);
        [v, v, v * 0.9]
    }));

    // T_LANDSCAPE : tableau de paysage nocturne (48x32).
    v.push(gen(48, 32, |x, y| {
        if x < 3 || x > 44 || y < 3 || y > 28 {
            return sc([0.55, 0.42, 0.15], if x == 1 || y == 1 { 1.3 } else { 0.9 });
        }
        let ridge = 16.0 + 4.0 * (x as f32 * 0.3).sin() + 2.0 * (x as f32 * 0.9).sin();
        let dx = x as f32 - 34.0;
        let dy = y as f32 - 9.0;
        if dx * dx + dy * dy < 9.0 {
            return [0.85, 0.82, 0.65];
        }
        if (y as f32) > ridge {
            sc([0.08, 0.10, 0.07], 0.8 + 0.4 * hash2(x as i32, y as i32, 29))
        } else {
            mix([0.05, 0.06, 0.14], [0.16, 0.18, 0.26], y as f32 / 16.0)
        }
    }));

    // T_PAPER_BLUE : papier peint bleu nuit.
    v.push(gen(32, 64, |x, y| wallpaper(x, y, [0.10, 0.12, 0.24], [0.40, 0.38, 0.30], 30)));

    // T_KEY : métal doré.
    v.push(gen(8, 8, |x, y| {
        let n = hash2(x as i32, y as i32, 31);
        sc([0.9, 0.72, 0.3], 0.7 + 0.5 * n)
    }));

    // T_FABRIC : toile neutre (teintée par le matériau), trame et coutures.
    v.push(gen(16, 16, |x, y| {
        let weave = if (x + y) % 2 == 0 { 1.04 } else { 0.94 };
        let n = fbm(x as f32 / 4.0, y as f32 / 4.0, 2, 34);
        let seam = if x == 0 { 0.7 } else { 1.0 };
        let v = weave * (0.78 + 0.32 * n) * seam;
        [v, v, v]
    }));

    // T_VEST : gilet tactique enroulé autour du torse (u = 0,5 à l'avant).
    v.push(gen(64, 32, |x, y| {
        let n = hash2(x as i32, y as i32, 35);
        let cloth: C = sc([0.17, 0.17, 0.15], 0.85 + 0.25 * n);
        let pouch: C = [0.24, 0.23, 0.17];
        let strap: C = [0.09, 0.08, 0.07];
        let (xi, yi) = (x as i32, y as i32);
        // Fermeture éclair au centre avant.
        if (31..=32).contains(&xi) {
            return if y % 2 == 0 { [0.42, 0.42, 0.40] } else { strap };
        }
        // Porte-cartouches et poches de poitrine.
        for c in [22, 41] {
            let dx = xi - c;
            if (-5..=5).contains(&dx) && (8..=21).contains(&yi) {
                if yi == 8 || yi == 9 {
                    return sc(pouch, 1.25); // rabat
                }
                if yi == 10 || dx.abs() == 5 || yi == 21 {
                    return sc(pouch, 0.55);
                }
                if yi == 9 + 1 && dx == 0 {
                    return [0.05, 0.05, 0.05];
                }
                return sc(pouch, 0.9 + 0.2 * n);
            }
        }
        // Sangles horizontales sur les flancs.
        if (yi == 24 || yi == 25) && !(26..=37).contains(&xi) {
            return strap;
        }
        // Écusson dans le dos : bouclier sombre, bordure or, barre rouge.
        let bx = ((xi + 32) % 64) - 32;
        if (-5..=5).contains(&bx) && (5..=15).contains(&yi) {
            let half = 5 - (yi - 11).max(0);
            if bx.abs() <= half {
                if bx.abs() == half || yi == 5 {
                    return [0.62, 0.50, 0.20];
                }
                if (9..=10).contains(&yi) {
                    return [0.55, 0.10, 0.08];
                }
                return [0.10, 0.12, 0.20];
            }
        }
        // Surpiqûres en bordure.
        if y == 0 || y == 31 {
            return sc(cloth, 0.6);
        }
        cloth
    }));

    v
}
