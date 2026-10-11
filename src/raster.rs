//! Rasteriseur logiciel imitant le GPU de la PlayStation 1 :
//! - résolution 320x240,
//! - sommets "snappés" sur la grille de pixels (tremblement caractéristique),
//! - placage de texture affine (pas de correction de perspective),
//! - éclairage de Gouraud par sommet, brouillard par sommet,
//! - couleurs quantifiées en 15 bits avec tramage ordonné 4x4.

use crate::math::*;
use crate::mesh::{Blend, Mesh};

pub const W: usize = 320;
pub const H: usize = 240;
const NEAR: f32 = 0.06;

/// Matrice de tramage du GPU de la PS1.
const DITHER: [[f32; 4]; 4] = [
    [-4.0, 0.0, -3.0, 1.0],
    [2.0, -2.0, 3.0, -1.0],
    [-3.0, 1.0, -4.0, 0.0],
    [3.0, -1.0, 2.0, -2.0],
];

#[inline]
pub fn pack_dither(r: f32, g: f32, b: f32, x: usize, y: usize) -> u32 {
    let d = DITHER[y & 3][x & 3];
    let q = |c: f32| ((c * 255.0 + d).clamp(0.0, 255.0) as u32) & 0xF8;
    (q(r) << 16) | (q(g) << 8) | q(b)
}

#[inline]
pub fn unpack(c: u32) -> [f32; 3] {
    [
        ((c >> 16) & 0xFF) as f32 / 255.0,
        ((c >> 8) & 0xFF) as f32 / 255.0,
        (c & 0xFF) as f32 / 255.0,
    ]
}

#[inline]
pub fn pack(r: f32, g: f32, b: f32) -> u32 {
    let q = |c: f32| (c * 255.0).clamp(0.0, 255.0) as u32;
    (q(r) << 16) | (q(g) << 8) | q(b)
}

pub struct Texture {
    pub w: usize,
    pub h: usize,
    pub px: Vec<[f32; 4]>,
}

impl Texture {
    pub fn new(w: usize, h: usize) -> Texture {
        Texture { w, h, px: vec![[0.0, 0.0, 0.0, 1.0]; w * h] }
    }
    pub fn set(&mut self, x: usize, y: usize, c: [f32; 3]) {
        let i = (y % self.h) * self.w + (x % self.w);
        self.px[i] = [c[0].clamp(0.0, 1.0), c[1].clamp(0.0, 1.0), c[2].clamp(0.0, 1.0), 1.0];
    }
    pub fn set_a(&mut self, x: usize, y: usize, c: [f32; 4]) {
        let i = (y % self.h) * self.w + (x % self.w);
        self.px[i] = c;
    }
    pub fn get(&self, x: usize, y: usize) -> [f32; 4] {
        self.px[(y % self.h) * self.w + (x % self.w)]
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub eye: Vec3,
    pub fwd: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub focal: f32,
}

impl Camera {
    pub fn look_at(eye: Vec3, target: Vec3, fov_y_deg: f32) -> Camera {
        let fwd = (target - eye).norm();
        let right = Vec3::UP.cross(fwd).norm();
        let up = fwd.cross(right);
        let focal = (H as f32 * 0.5) / (fov_y_deg.to_radians() * 0.5).tan();
        Camera { eye, fwd, right, up, focal }
    }
    pub fn to_view(&self, p: Vec3) -> Vec3 {
        let d = p - self.eye;
        v3(d.dot(self.right), d.dot(self.up), d.dot(self.fwd))
    }
    /// Projette un point du monde en coordonnées écran (x, y, profondeur).
    pub fn project(&self, p: Vec3) -> Option<(f32, f32, f32)> {
        let v = self.to_view(p);
        if v.z < NEAR {
            return None;
        }
        Some((
            W as f32 * 0.5 + v.x / v.z * self.focal,
            H as f32 * 0.5 - v.y / v.z * self.focal,
            v.z,
        ))
    }
    /// Rayon (normalisé) passant par un pixel de l'écran.
    pub fn ray(&self, sx: f32, sy: f32) -> Vec3 {
        (self.right * (sx - W as f32 * 0.5) + self.up * (H as f32 * 0.5 - sy) + self.fwd * self.focal).norm()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Fog {
    pub start: f32,
    pub end: f32,
    pub color: [f32; 3],
}

#[derive(Clone, Copy, Debug)]
pub struct Light {
    pub pos: Vec3,
    pub color: [f32; 3],
    pub radius: f32,
}

#[derive(Clone, Debug, Default)]
pub struct Lighting {
    pub ambient: [f32; 3],
    pub lights: Vec<Light>,
    /// Lumière directionnelle (direction vers la source, couleur).
    pub sun: Option<(Vec3, [f32; 3])>,
}

impl Lighting {
    pub fn at(&self, p: Vec3, n: Vec3) -> [f32; 3] {
        let mut c = self.ambient;
        for l in &self.lights {
            let d = l.pos - p;
            let dist2 = d.dot(d);
            if dist2 >= l.radius * l.radius {
                continue;
            }
            let dist = dist2.sqrt().max(1e-4);
            let ndl = (n.dot(d * (1.0 / dist)) * 0.75 + 0.25).max(0.0);
            let att = 1.0 - dist / l.radius;
            let k = ndl * att * att;
            c[0] += l.color[0] * k;
            c[1] += l.color[1] * k;
            c[2] += l.color[2] * k;
        }
        if let Some((dir, col)) = self.sun {
            let k = (n.dot(dir) * 0.7 + 0.3).max(0.0);
            c[0] += col[0] * k;
            c[1] += col[1] * k;
            c[2] += col[2] * k;
        }
        c
    }
}

#[derive(Clone, Copy, Default)]
struct RV {
    p: Vec3, // espace vue
    u: f32,
    v: f32,
    c: [f32; 3],
    f: f32, // facteur de brouillard
}

#[derive(Clone, Copy, Default)]
struct SV {
    x: f64,
    y: f64,
    iz: f32,
    u: f32,
    v: f32,
    c: [f32; 3],
    f: f32,
}

pub struct Raster {
    pub fb: Vec<u32>,
    pub depth: Vec<f32>,
    pub tex: Vec<Texture>,
    pub cam: Camera,
    pub fog: Fog,
    /// Rectangle de découpe (x0, y0, x1, y1), bornes hautes exclues.
    pub scissor: (i32, i32, i32, i32),
    scratch_v: Vec<RV>,
    scratch_w: Vec<Vec3>,
}

impl Raster {
    pub fn new(tex: Vec<Texture>) -> Raster {
        Raster {
            fb: vec![0; W * H],
            depth: vec![0.0; W * H],
            tex,
            cam: Camera::look_at(Vec3::ZERO, v3(0.0, 0.0, 1.0), 60.0),
            fog: Fog { start: 10.0, end: 30.0, color: [0.0; 3] },
            scissor: (0, 0, W as i32, H as i32),
            scratch_v: Vec::new(),
            scratch_w: Vec::new(),
        }
    }

    pub fn clear(&mut self, color: u32) {
        self.fb.fill(color);
        self.depth.fill(0.0);
        self.reset_scissor();
    }

    pub fn reset_scissor(&mut self) {
        self.scissor = (0, 0, W as i32, H as i32);
    }

    #[inline]
    fn fog_factor(&self, z: f32) -> f32 {
        clamp01((z - self.fog.start) / (self.fog.end - self.fog.start))
    }

    /// Dessine un maillage transformé par `xf`. Si `light` est `None`, les
    /// couleurs des sommets sont utilisées telles quelles (pré-éclairage).
    pub fn draw_mesh(&mut self, mesh: &Mesh, xf: &Mat4, light: Option<&Lighting>) {
        let mut sv = std::mem::take(&mut self.scratch_v);
        let mut sw = std::mem::take(&mut self.scratch_w);
        sv.clear();
        sw.clear();
        for v in &mesh.v {
            let wp = xf.point(v.p);
            let c = match light {
                Some(l) => {
                    let wn = xf.dir(v.n).norm();
                    let li = l.at(wp, wn);
                    [
                        v.c[0] * (li[0] + v.glow),
                        v.c[1] * (li[1] + v.glow),
                        v.c[2] * (li[2] + v.glow),
                    ]
                }
                None => v.c,
            };
            let vp = self.cam.to_view(wp);
            sw.push(wp);
            sv.push(RV { p: vp, u: v.uv[0], v: v.uv[1], c, f: self.fog_factor(vp.z) });
        }
        let eye = self.cam.eye;
        for t in &mesh.t {
            let [a, b, c] = t.i;
            let (a, b, c) = (a as usize, b as usize, c as usize);
            if !t.two_sided {
                let n = (sw[c] - sw[a]).cross(sw[b] - sw[a]);
                if n.dot(eye - sw[a]) <= 0.0 {
                    continue;
                }
            }
            self.tri(sv[a], sv[b], sv[c], t.tex, t.blend);
        }
        self.scratch_v = sv;
        self.scratch_w = sw;
    }

    /// Triangle en coordonnées monde avec couleurs déjà calculées.
    pub fn draw_tri_world(&mut self, p: [Vec3; 3], uv: [[f32; 2]; 3], c: [[f32; 3]; 3], tex: i16, blend: Blend) {
        let mut r = [RV::default(); 3];
        for i in 0..3 {
            let vp = self.cam.to_view(p[i]);
            r[i] = RV { p: vp, u: uv[i][0], v: uv[i][1], c: c[i], f: self.fog_factor(vp.z) };
        }
        self.tri(r[0], r[1], r[2], tex, blend);
    }

    /// Billboard carré face caméra (flammes, éclats, flash de tir...).
    pub fn billboard(&mut self, center: Vec3, size: f32, c: [f32; 3], tex: i16, blend: Blend) {
        let r = self.cam.right * size;
        let u = self.cam.up * size;
        let p0 = center - r - u;
        let p1 = center + r - u;
        let p2 = center + r + u;
        let p3 = center - r + u;
        self.draw_tri_world([p0, p1, p2], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0]], [c; 3], tex, blend);
        self.draw_tri_world([p0, p2, p3], [[0.0, 1.0], [1.0, 0.0], [0.0, 0.0]], [c; 3], tex, blend);
    }

    /// Point (particule) de `size` pixels avec test de profondeur.
    pub fn point(&mut self, p: Vec3, size: i32, c: [f32; 3], blend: Blend) {
        let Some((sx, sy, z)) = self.cam.project(p) else { return };
        let f = self.fog_factor(z);
        let iz = 1.0 / z;
        let x0 = sx as i32 - size / 2;
        let y0 = sy as i32 - size / 2;
        for y in y0.max(self.scissor.1)..(y0 + size).min(self.scissor.3) {
            for x in x0.max(self.scissor.0)..(x0 + size).min(self.scissor.2) {
                let i = y as usize * W + x as usize;
                if iz <= self.depth[i] {
                    continue;
                }
                self.put(i, x as usize, y as usize, c, f, blend, iz);
            }
        }
    }

    #[inline]
    fn put(&mut self, i: usize, x: usize, y: usize, c: [f32; 3], f: f32, blend: Blend, iz: f32) {
        let fc = self.fog.color;
        Self::put_px(&mut self.fb, &mut self.depth, fc, i, x, y, c, f, blend, iz);
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    fn put_px(fbuf: &mut [u32], depth: &mut [f32], fc: [f32; 3], i: usize, x: usize, y: usize, c: [f32; 3], f: f32, blend: Blend, iz: f32) {
        match blend {
            Blend::Opaque | Blend::Decal => {
                let r = c[0] + (fc[0] - c[0]) * f;
                let g = c[1] + (fc[1] - c[1]) * f;
                let b = c[2] + (fc[2] - c[2]) * f;
                fbuf[i] = pack_dither(r, g, b, x, y);
                depth[i] = iz;
            }
            Blend::Add => {
                let d = unpack(fbuf[i]);
                let k = 1.0 - f;
                fbuf[i] = pack_dither(d[0] + c[0] * k, d[1] + c[1] * k, d[2] + c[2] * k, x, y);
            }
            Blend::Mul => {
                let d = unpack(fbuf[i]);
                let k = 1.0 - f;
                let m = |s: f32| 1.0 + (s - 1.0) * k;
                fbuf[i] = pack(d[0] * m(c[0]), d[1] * m(c[1]), d[2] * m(c[2]));
            }
        }
    }

    fn tri(&mut self, a: RV, b: RV, c: RV, tex: i16, blend: Blend) {
        if a.p.z < NEAR && b.p.z < NEAR && c.p.z < NEAR {
            return;
        }
        // Rejet rapide hors du champ de vision.
        let kx = (W as f32 * 0.5) / self.cam.focal * 1.02;
        let ky = (H as f32 * 0.5) / self.cam.focal * 1.02;
        let out = |f: &dyn Fn(&RV) -> bool| f(&a) && f(&b) && f(&c);
        let all_front = a.p.z > NEAR && b.p.z > NEAR && c.p.z > NEAR;
        if all_front && out(&|v: &RV| v.p.x > v.p.z * kx)
            || out(&|v: &RV| v.p.x < -v.p.z * kx)
            || out(&|v: &RV| v.p.y > v.p.z * ky)
            || out(&|v: &RV| v.p.y < -v.p.z * ky)
        {
            return;
        }
        if !matches!(blend, Blend::Opaque | Blend::Decal) && a.f >= 1.0 && b.f >= 1.0 && c.f >= 1.0 {
            return;
        }

        // Découpage contre le plan proche.
        let mut poly = [RV::default(); 4];
        let mut n = 0;
        let src = [a, b, c];
        if a.p.z >= NEAR && b.p.z >= NEAR && c.p.z >= NEAR {
            poly[..3].copy_from_slice(&src);
            n = 3;
        } else {
            for i in 0..3 {
                let p = src[i];
                let q = src[(i + 1) % 3];
                let pin = p.p.z >= NEAR;
                let qin = q.p.z >= NEAR;
                if pin {
                    poly[n] = p;
                    n += 1;
                }
                if pin != qin && n < 4 {
                    let t = (NEAR - p.p.z) / (q.p.z - p.p.z);
                    poly[n] = RV {
                        p: p.p.lerp(q.p, t),
                        u: p.u + (q.u - p.u) * t,
                        v: p.v + (q.v - p.v) * t,
                        c: [
                            p.c[0] + (q.c[0] - p.c[0]) * t,
                            p.c[1] + (q.c[1] - p.c[1]) * t,
                            p.c[2] + (q.c[2] - p.c[2]) * t,
                        ],
                        f: p.f + (q.f - p.f) * t,
                    };
                    n += 1;
                }
            }
            if n < 3 {
                return;
            }
        }

        let (tw, th) = if tex >= 0 {
            let t = &self.tex[tex as usize];
            (t.w as f32, t.h as f32)
        } else {
            (1.0, 1.0)
        };
        let hw = W as f32 * 0.5;
        let hh = H as f32 * 0.5;
        let focal = self.cam.focal;
        let mut s = [SV::default(); 4];
        for i in 0..n {
            let r = &poly[i];
            let iz = 1.0 / r.p.z;
            // Snapping des sommets sur la grille entière : le "jitter" PS1.
            s[i] = SV {
                x: ((hw + r.p.x * iz * focal) as f64).round(),
                y: ((hh - r.p.y * iz * focal) as f64).round(),
                iz,
                u: r.u * tw,
                v: r.v * th,
                c: r.c,
                f: r.f,
            };
        }
        self.raster(&s[0], &s[1], &s[2], tex, blend);
        if n == 4 {
            self.raster(&s[0], &s[2], &s[3], tex, blend);
        }
    }

    fn raster(&mut self, v0: &SV, v1: &SV, v2: &SV, tex: i16, blend: Blend) {
        let area = (v1.x - v0.x) * (v2.y - v0.y) - (v1.y - v0.y) * (v2.x - v0.x);
        if area.abs() < 0.5 {
            return;
        }
        let (sx0, sy0, sx1, sy1) = self.scissor;
        let minx = (v0.x.min(v1.x).min(v2.x).floor() as i64 - 1).max(sx0 as i64);
        let maxx = (v0.x.max(v1.x).max(v2.x).ceil() as i64 + 1).min(sx1 as i64 - 1);
        let miny = (v0.y.min(v1.y).min(v2.y).floor() as i64 - 1).max(sy0 as i64);
        let maxy = (v0.y.max(v1.y).max(v2.y).ceil() as i64 + 1).min(sy1 as i64 - 1);
        if minx > maxx || miny > maxy {
            return;
        }
        let inv = 1.0 / area;
        // Fonctions d'arête normalisées : w0 = E12/aire, etc.
        let edge = |a: &SV, b: &SV, px: f64, py: f64| (b.x - a.x) * (py - a.y) - (b.y - a.y) * (px - a.x);
        let px0 = minx as f64 + 0.5;
        let py0 = miny as f64 + 0.5;
        let mut w0r = edge(v1, v2, px0, py0) * inv;
        let mut w1r = edge(v2, v0, px0, py0) * inv;
        let mut w2r = edge(v0, v1, px0, py0) * inv;
        let w0dx = -(v2.y - v1.y) * inv;
        let w1dx = -(v0.y - v2.y) * inv;
        let w2dx = -(v1.y - v0.y) * inv;
        let w0dy = (v2.x - v1.x) * inv;
        let w1dy = (v0.x - v2.x) * inv;
        let w2dy = (v1.x - v0.x) * inv;

        let texture = if tex >= 0 { Some(&self.tex[tex as usize]) } else { None };
        let fogc = self.fog.color;
        // Les surfaces opaques débordent légèrement de leurs arêtes (en pixels)
        // pour boucher les fissures dues au snapping et aux jonctions en T.
        let opaque = matches!(blend, Blend::Opaque | Blend::Decal);
        let tol = if opaque { 0.6 } else { 0.0 };
        let len = |a: &SV, b: &SV| ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
        let ia = inv.abs();
        let e0 = -tol * len(v1, v2) * ia - 1e-7;
        let e1 = -tol * len(v2, v0) * ia - 1e-7;
        let e2 = -tol * len(v0, v1) * ia - 1e-7;
        // Décalques (tapis, sang, ombres) : léger biais de profondeur.
        let bias = if matches!(blend, Blend::Decal | Blend::Mul) { 1.008 } else { 1.0 };

        for y in miny..=maxy {
            let (mut w0, mut w1, mut w2) = (w0r, w1r, w2r);
            let yu = y as usize;
            for x in minx..=maxx {
                if w0 >= e0 && w1 >= e1 && w2 >= e2 {
                    // Dans la marge anti-fissures, on n'extrapole pas : les poids
                    // sont ramenés sur le triangle (sinon un triangle vu par la
                    // tranche produit des profondeurs et couleurs aberrantes).
                    let (a, b, c) = if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                        let (x0, x1, x2) = (w0.max(0.0), w1.max(0.0), w2.max(0.0));
                        let sum = (x0 + x1 + x2).max(1e-9);
                        ((x0 / sum) as f32, (x1 / sum) as f32, (x2 / sum) as f32)
                    } else {
                        (w0 as f32, w1 as f32, w2 as f32)
                    };
                    let i = yu * W + x as usize;
                    let iz = a * v0.iz + b * v1.iz + c * v2.iz;
                    if iz * bias > self.depth[i] {
                        let mut col = [
                            a * v0.c[0] + b * v1.c[0] + c * v2.c[0],
                            a * v0.c[1] + b * v1.c[1] + c * v2.c[1],
                            a * v0.c[2] + b * v1.c[2] + c * v2.c[2],
                        ];
                        let mut visible = true;
                        if let Some(t) = texture {
                            // Interpolation affine en espace écran (pas de perspective).
                            let u = a * v0.u + b * v1.u + c * v2.u;
                            let v = a * v0.v + b * v1.v + c * v2.v;
                            let tx = (u.floor() as i32).rem_euclid(t.w as i32) as usize;
                            let ty = (v.floor() as i32).rem_euclid(t.h as i32) as usize;
                            let texel = t.px[ty * t.w + tx];
                            if texel[3] < 0.5 {
                                visible = false;
                            }
                            col[0] *= texel[0];
                            col[1] *= texel[1];
                            col[2] *= texel[2];
                        }
                        if visible {
                            let f = a * v0.f + b * v1.f + c * v2.f;
                            let xu = x as usize;
                            match blend {
                                Blend::Opaque | Blend::Decal => {
                                    let r = col[0] + (fogc[0] - col[0]) * f;
                                    let g = col[1] + (fogc[1] - col[1]) * f;
                                    let bl = col[2] + (fogc[2] - col[2]) * f;
                                    self.fb[i] = pack_dither(r, g, bl, xu, yu);
                                    self.depth[i] = iz;
                                }
                                _ => Self::put_px(&mut self.fb, &mut self.depth, fogc, i, xu, yu, col, f, blend, iz),
                            }
                        }
                    }
                }
                w0 += w0dx;
                w1 += w1dx;
                w2 += w2dx;
            }
            w0r += w0dy;
            w1r += w1dy;
            w2r += w2dy;
        }
    }

    /// Remplit le ciel sur les pixels restés vides (profondeur nulle).
    pub fn fill_sky(&mut self, sky: &dyn Fn(Vec3, usize, usize) -> [f32; 3]) {
        let (x0, y0, x1, y1) = self.scissor;
        for y in y0.max(0)..y1.min(H as i32) {
            for x in x0.max(0)..x1.min(W as i32) {
                let i = y as usize * W + x as usize;
                if self.depth[i] == 0.0 {
                    let d = self.cam.ray(x as f32 + 0.5, y as f32 + 0.5);
                    let c = sky(d, x as usize, y as usize);
                    self.fb[i] = pack_dither(c[0], c[1], c[2], x as usize, y as usize);
                    self.depth[i] = 1e-9;
                }
            }
        }
    }

    // ---------------------------------------------------------------- 2D / interface

    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: [f32; 3], alpha: f32) {
        for yy in y.max(0)..(y + h).min(H as i32) {
            for xx in x.max(0)..(x + w).min(W as i32) {
                let i = yy as usize * W + xx as usize;
                let d = unpack(self.fb[i]);
                self.fb[i] = pack(
                    d[0] + (c[0] - d[0]) * alpha,
                    d[1] + (c[1] - d[1]) * alpha,
                    d[2] + (c[2] - d[2]) * alpha,
                );
            }
        }
    }

    pub fn fill_circle(&mut self, cx: f32, cy: f32, r: f32, c: [f32; 3]) {
        let col = pack(c[0], c[1], c[2]);
        for y in (cy - r).floor() as i32..=(cy + r).ceil() as i32 {
            for x in (cx - r).floor() as i32..=(cx + r).ceil() as i32 {
                let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                if dx * dx + dy * dy <= r * r && x >= 0 && y >= 0 && (x as usize) < W && (y as usize) < H {
                    self.fb[y as usize * W + x as usize] = col;
                }
            }
        }
    }

    /// Petit triangle plein (flèches d'interface). `dir` : 0 gauche, 1 droite, 2 haut, 3 bas.
    pub fn arrow(&mut self, cx: i32, cy: i32, size: i32, dir: u8, c: [f32; 3]) {
        for i in 0..=size {
            let half = size - i;
            for j in -half..=half {
                let (x, y) = match dir {
                    0 => (cx + size / 2 - i, cy + j),
                    1 => (cx - size / 2 + i, cy + j),
                    2 => (cx + j, cy + size / 2 - i),
                    _ => (cx + j, cy - size / 2 + i),
                };
                self.fill_rect(x, y, 1, 1, c, 1.0);
            }
        }
    }

    pub fn rect_outline(&mut self, x: i32, y: i32, w: i32, h: i32, c: [f32; 3]) {
        self.fill_rect(x, y, w, 1, c, 1.0);
        self.fill_rect(x, y + h - 1, w, 1, c, 1.0);
        self.fill_rect(x, y, 1, h, c, 1.0);
        self.fill_rect(x + w - 1, y, 1, h, c, 1.0);
    }

    fn glyph(ch: char) -> [u8; 8] {
        // Les capitales accentuées 8x8 ressemblent à des minuscules : on les simplifie.
        let ch = match ch {
            'À' | 'Â' | 'Ä' => 'A',
            'É' | 'È' | 'Ê' | 'Ë' => 'E',
            'Î' | 'Ï' => 'I',
            'Ô' | 'Ö' => 'O',
            'Ù' | 'Û' | 'Ü' => 'U',
            'Ç' => 'C',
            c => c,
        };
        // Accent circonflexe redessiné (le glyphe d'origine est peu lisible).
        let circ = match ch {
            'â' => Some('a'),
            'ê' => Some('e'),
            'î' => Some('i'),
            'ô' => Some('o'),
            'û' => Some('u'),
            _ => None,
        };
        if let Some(base) = circ {
            let mut g = font8x8::legacy::BASIC_LEGACY[base as usize];
            g[0] = 0x0C;
            g[1] = 0x12;
            return g;
        }
        let code = ch as u32;
        if code < 128 {
            font8x8::legacy::BASIC_LEGACY[code as usize]
        } else if (0xA0..0x100).contains(&code) {
            font8x8::legacy::LATIN_LEGACY[(code - 0xA0) as usize]
        } else {
            font8x8::legacy::BASIC_LEGACY['?' as usize]
        }
    }

    /// Texte en police 8x8, avec ombre portée.
    pub fn text(&mut self, x: i32, y: i32, s: &str, c: [f32; 3], scale: i32) {
        self.text_raw(x + scale, y + scale, s, [0.0, 0.0, 0.0], scale);
        self.text_raw(x, y, s, c, scale);
    }

    pub fn text_raw(&mut self, x: i32, y: i32, s: &str, c: [f32; 3], scale: i32) {
        let col = pack(c[0], c[1], c[2]);
        let mut cx = x;
        for ch in s.chars() {
            let g = Self::glyph(ch);
            for (row, bits) in g.iter().enumerate() {
                for bit in 0..8 {
                    if bits & (1 << bit) != 0 {
                        for sy in 0..scale {
                            for sx in 0..scale {
                                let px = cx + bit * scale + sx;
                                let py = y + row as i32 * scale + sy;
                                if px >= 0 && py >= 0 && (px as usize) < W && (py as usize) < H {
                                    self.fb[py as usize * W + px as usize] = col;
                                }
                            }
                        }
                    }
                }
            }
            cx += 8 * scale;
        }
    }

    pub fn text_width(s: &str, scale: i32) -> i32 {
        s.chars().count() as i32 * 8 * scale
    }

    pub fn text_center(&mut self, y: i32, s: &str, c: [f32; 3], scale: i32) {
        let x = (W as i32 - Self::text_width(s, scale)) / 2;
        self.text(x, y, s, c, scale);
    }

    /// Multiplie toute l'image (fondu au noir) et ajoute une teinte.
    pub fn post(&mut self, mul: f32, tint: [f32; 3]) {
        if (mul - 1.0).abs() < 1e-3 && tint == [0.0; 3] {
            return;
        }
        for (i, p) in self.fb.iter_mut().enumerate() {
            let d = unpack(*p);
            let x = i % W;
            let y = i / W;
            *p = pack_dither(d[0] * mul + tint[0], d[1] * mul + tint[1], d[2] * mul + tint[2], x, y);
        }
    }
}
