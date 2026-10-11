//! L'extérieur visible par les fenêtres : marais, arbres morts, grille,
//! montagnes, ciel nocturne et lune.

use crate::math::*;
use crate::mesh::*;
use crate::raster::{Fog, Lighting};
use crate::textures::*;

pub struct Outdoor {
    pub ground: Mesh,
    pub mountains: Mesh,
    pub moon_dir: Vec3,
    moon_u: Vec3,
    moon_v: Vec3,
    pub fireflies: Vec<Vec3>,
}

/// Brume du marais (un peu plus sombre que l'horizon du ciel).
pub const HAZE: [f32; 3] = [0.09, 0.1, 0.13];
pub const GROUND_FOG: Fog = Fog { start: 10.0, end: 140.0, color: HAZE };
pub const MOUNTAIN_FOG: Fog = Fog { start: 200.0, end: 1400.0, color: [0.16, 0.18, 0.25] };

/// Emprise du manoir (aucun décor extérieur dedans).
fn in_manor(x: f32, z: f32, margin: f32) -> bool {
    x > -18.0 - margin && x < 14.0 + margin && z > -1.0 - margin && z < 27.0 + margin
}

fn water_n(x: f32, z: f32) -> f32 {
    fbm(x / 16.0, z / 16.0, 3, 77)
}

fn ground_h(x: f32, z: f32) -> f32 {
    let n = water_n(x, z);
    let bumps = (fbm(x / 5.0, z / 5.0, 2, 78) - 0.5) * 0.35;
    let far = ((x * x + z * z).sqrt() - 120.0).max(0.0) * 0.04;
    -0.75 + smoothstep(0.36, 0.46, n) * (0.25 + bumps) + far
}

fn ground_col(x: f32, z: f32) -> [f32; 3] {
    let n = water_n(x, z);
    let moss = fbm(x / 7.0, z / 7.0, 2, 79);
    let land = [0.12 + 0.1 * moss, 0.17 + 0.12 * moss, 0.08 + 0.03 * moss];
    let water = [0.06, 0.08, 0.11];
    let t = smoothstep(0.38, 0.45, n);
    [
        water[0] + (land[0] - water[0]) * t,
        water[1] + (land[1] - water[1]) * t,
        water[2] + (land[2] - water[2]) * t,
    ]
}

impl Outdoor {
    pub fn new() -> Outdoor {
        let moon_dir = v3(-0.22, 0.21, -0.95).norm();
        let moon_u = Vec3::UP.cross(moon_dir).norm();
        let moon_v = moon_dir.cross(moon_u);
        let mut rng = Rng::new(4242);
        let mut ground = Mesh::new();

        // Terrain : grille fine près du manoir, grossière au loin.
        let add_grid = |g: &mut Mesh, ext: f32, cell: f32, skip: f32| {
            let n = (ext * 2.0 / cell) as i32;
            for j in 0..n {
                for i in 0..n {
                    let x0 = -ext + i as f32 * cell;
                    let z0 = -ext + j as f32 * cell;
                    let (x1, z1) = (x0 + cell, z0 + cell);
                    let cx = (x0 + x1) * 0.5;
                    let cz = (z0 + z1) * 0.5;
                    if skip > 0.0 && cx.abs() < skip && cz.abs() < skip {
                        continue;
                    }
                    if in_manor(cx, cz, -1.0) {
                        continue;
                    }
                    let water = water_n(cx, cz) < 0.40;
                    let tex = if water { -1 } else { T_GRASS };
                    let p = |x: f32, z: f32| v3(x, ground_h(x, z), z);
                    let corners = [p(x0, z0), p(x1, z0), p(x1, z1), p(x0, z1)];
                    let base = g.v.len() as u32;
                    for (k, c) in corners.iter().enumerate() {
                        let uv = [c.x * 0.5, c.z * 0.5];
                        let mut col = ground_col(c.x, c.z);
                        if water {
                            // Reflet de lune sur l'eau.
                            let k2 = (k as f32 * 0.37 + cx * 0.13).sin() * 0.5 + 0.5;
                            col = [col[0] + 0.03 * k2, col[1] + 0.035 * k2, col[2] + 0.06 * k2];
                        }
                        g.v.push(MV { p: *c, n: Vec3::UP, uv, c: col, glow: 0.0 });
                    }
                    g.t.push(MT { i: [base, base + 1, base + 2], tex, blend: Blend::Opaque, two_sided: false });
                    g.t.push(MT { i: [base, base + 2, base + 3], tex, blend: Blend::Opaque, two_sided: false });
                }
            }
        };
        add_grid(&mut ground, 50.0, 2.5, 0.0);
        add_grid(&mut ground, 300.0, 15.0, 50.0);

        // Arbres morts.
        let bark = Mat::tex(T_BARK, [0.55, 0.5, 0.45]);
        let mut placed = 0;
        while placed < 34 {
            let a = rng.range(0.0, std::f32::consts::TAU);
            let r = rng.range(16.0, 95.0);
            let x = -2.0 + a.sin() * r;
            let z = 13.0 + a.cos() * r;
            if in_manor(x, z, 4.0) {
                continue;
            }
            placed += 1;
            let h = rng.range(3.5, 9.0);
            let w = rng.range(0.25, 0.5);
            let mut t = Mesh::new();
            t.taper(0.0, h, (w, w), (w * 0.3, w * 0.3), (0.0, 0.0), &bark, None);
            for _ in 0..rng.range(3.0, 6.0) as i32 {
                let mut br = Mesh::new();
                let bl = rng.range(1.2, 3.0);
                br.taper(0.0, bl, (w * 0.35, w * 0.35), (0.04, 0.04), (0.0, 0.0), &bark, None);
                let y = rng.range(h * 0.4, h * 0.95);
                let xf = Mat4::translate(v3(0.0, y, 0.0)) * Mat4::rot_y(rng.range(0.0, std::f32::consts::TAU)) * Mat4::rot_x(-rng.range(0.5, 1.2));
                t.append(&br, &xf);
            }
            let lean = Mat4::rot_x(rng.range(-0.15, 0.15)) * Mat4::rot_z(rng.range(-0.15, 0.15));
            ground.append(&t, &(Mat4::translate(v3(x, ground_h(x, z) - 0.2, z)) * lean));
        }

        // Roseaux autour des mares.
        let reed = Mat::col([0.16, 0.18, 0.08]).two();
        let mut clusters = 0;
        let mut tries = 0;
        while clusters < 260 && tries < 20000 {
            tries += 1;
            let x = rng.range(-48.0, 48.0);
            let z = rng.range(-35.0, 60.0);
            if in_manor(x, z, 1.5) {
                continue;
            }
            let n = water_n(x, z);
            if !(0.36..0.47).contains(&n) {
                continue;
            }
            clusters += 1;
            let y = ground_h(x, z);
            for _ in 0..6 {
                let ox = x + rng.range(-0.5, 0.5);
                let oz = z + rng.range(-0.5, 0.5);
                let hgt = rng.range(0.7, 1.7);
                let lx = rng.range(-0.3, 0.3);
                let lz = rng.range(-0.3, 0.3);
                let a = rng.range(0.0, std::f32::consts::PI);
                let side = v3(a.cos(), 0.0, a.sin()) * 0.03;
                ground.tri([v3(ox, y, oz) - side, v3(ox, y, oz) + side, v3(ox + lx, y + hgt, oz + lz)], [[0.0; 2]; 3], &reed, v3(0.0, 0.0, 1.0));
            }
        }

        // Grille en fer forgé autour du domaine.
        let iron = Mat::col([0.07, 0.07, 0.08]).two();
        let (fx0, fz0, fx1, fz1) = (-26.0f32, -9.0f32, 22.0f32, 35.0f32);
        let fence_side = |g: &mut Mesh, a: Vec3, b: Vec3| {
            let len = a.dist_xz(b);
            let n = (len / 0.35) as i32;
            let dir = (b - a) * (1.0 / len);
            let nrm = v3(-dir.z, 0.0, dir.x);
            for i in 0..=n {
                let p = a + dir * (i as f32 * 0.35);
                let y = ground_h(p.x, p.z);
                let w = dir * 0.025;
                let top = 1.9 + if i % 8 == 0 { 0.4 } else { 0.0 };
                g.quad([p + w + v3(0.0, y, 0.0), p - w + v3(0.0, y, 0.0), p - w + v3(0.0, y + top, 0.0), p + w + v3(0.0, y + top, 0.0)], [[0.0; 2]; 4], &iron, nrm);
                g.tri([p + w * 2.0 + v3(0.0, y + top, 0.0), p - w * 2.0 + v3(0.0, y + top, 0.0), p + v3(0.0, y + top + 0.15, 0.0)], [[0.0; 2]; 3], &iron, nrm);
            }
            for ry in [0.3f32, 1.6] {
                let ya = ground_h(a.x, a.z) + ry;
                let yb = ground_h(b.x, b.z) + ry;
                g.quad([a + v3(0.0, ya, 0.0), b + v3(0.0, yb, 0.0), b + v3(0.0, yb + 0.05, 0.0), a + v3(0.0, ya + 0.05, 0.0)], [[0.0; 2]; 4], &iron, nrm);
            }
        };
        fence_side(&mut ground, v3(fx0, 0.0, fz0), v3(-3.0, 0.0, fz0));
        fence_side(&mut ground, v3(3.0, 0.0, fz0), v3(fx1, 0.0, fz0));
        fence_side(&mut ground, v3(fx1, 0.0, fz0), v3(fx1, 0.0, fz1));
        fence_side(&mut ground, v3(fx1, 0.0, fz1), v3(fx0, 0.0, fz1));
        fence_side(&mut ground, v3(fx0, 0.0, fz1), v3(fx0, 0.0, fz0));
        // Piliers du portail.
        let stone = Mat::tex(T_STONE, [0.5, 0.5, 0.55]);
        for x in [-3.4f32, 3.0] {
            ground.cube(v3(x, -0.8, fz0 - 0.2), v3(x + 0.4, 2.6, fz0 + 0.2), &stone, 1.0);
        }
        // Allée de gravier menant au perron.
        let gravel = Mat::tex(T_STONE, [0.35, 0.34, 0.33]);
        ground.grid(v3(-1.5, -0.45, fz0), v3(3.0, 0.0, 0.0), v3(0.0, 0.0, -1.0 - fz0), [0.0, 0.0], [1.5, 4.0], 2.0, &gravel, Vec3::UP);
        // Perron.
        ground.cube(v3(-2.0, -0.7, -1.4), v3(2.0, -0.05, -1.0), &stone, 1.0);

        let moonlight = Lighting { ambient: [0.1, 0.11, 0.14], lights: vec![], sun: Some((moon_dir, [0.45, 0.5, 0.7])) };
        ground.prelight(&|p, n| moonlight.at(p, n));

        // Montagnes : deux couches de crêtes.
        let mut mountains = Mesh::new();
        for (layer, (radius, hmin, hmax, col)) in [(240.0f32, 10.0f32, 34.0f32, [0.2f32, 0.22, 0.27]), (380.0, 22.0, 75.0, [0.3, 0.33, 0.42])].iter().enumerate() {
            let segs = 120;
            let ridge: Vec<f32> = (0..=segs)
                .map(|i| {
                    let a = i as f32 / segs as f32;
                    let n = fbm(a * 9.0, layer as f32 * 7.0, 4, 90 + layer as u32);
                    let jag = hash2(i as i32, layer as i32, 91) * 0.15;
                    hmin + (hmax - hmin) * (n + jag).min(1.0)
                })
                .collect();
            let m = Mat::col(*col);
            for i in 0..segs {
                let a0 = i as f32 / segs as f32 * std::f32::consts::TAU;
                let a1 = (i + 1) as f32 / segs as f32 * std::f32::consts::TAU;
                let c = v3(-2.0, 0.0, 13.0);
                let b0 = c + v3(a0.sin() * radius, -6.0, a0.cos() * radius);
                let b1 = c + v3(a1.sin() * radius, -6.0, a1.cos() * radius);
                let t0 = c + v3(a0.sin() * radius * 0.94, ridge[i], a0.cos() * radius * 0.94);
                let t1 = c + v3(a1.sin() * radius * 0.94, ridge[i + 1], a1.cos() * radius * 0.94);
                mountains.quad([b0, b1, t1, t0], [[0.0; 2]; 4], &m, -(b0 - c).flat());
            }
        }
        let ml = Lighting { ambient: [0.1, 0.11, 0.14], lights: vec![], sun: Some((moon_dir, [0.2, 0.22, 0.3])) };
        mountains.prelight(&|p, n| ml.at(p, n));

        let mut fireflies = Vec::new();
        for _ in 0..40 {
            let x = rng.range(-40.0, 40.0);
            let z = rng.range(-30.0, 55.0);
            if in_manor(x, z, 2.0) {
                continue;
            }
            fireflies.push(v3(x, ground_h(x, z) + rng.range(0.3, 1.5), z));
        }

        Outdoor { ground, mountains, moon_dir, moon_u, moon_v, fireflies }
    }

    /// Couleur du ciel dans la direction `d` (normalisée).
    pub fn sky(&self, d: Vec3, time: f32) -> [f32; 3] {
        let e = d.y;
        let t = smoothstep(-0.02, 0.55, e);
        let hor = [0.17, 0.19, 0.27];
        let zen = [0.015, 0.02, 0.05];
        let mut c = [
            hor[0] + (zen[0] - hor[0]) * t,
            hor[1] + (zen[1] - hor[1]) * t,
            hor[2] + (zen[2] - hor[2]) * t,
        ];
        if e < 0.0 {
            return hor;
        }
        let m = d.dot(self.moon_dir);
        // Étoiles.
        let az = d.x.atan2(d.z);
        let el = e.asin();
        let sx = (az * 140.0).floor() as i32;
        let sy = (el * 140.0).floor() as i32;
        let h = hash2(sx, sy, 5);
        if h > 0.985 && e > 0.04 && m < 0.99 {
            let tw = 0.6 + 0.4 * (time * (2.0 + h * 40.0) + h * 100.0).sin();
            let b = (h - 0.985) / 0.015 * 0.7 * tw;
            c = [c[0] + b, c[1] + b, c[2] + b * 1.1];
        }
        // Lune.
        let cos_r = 0.9988f32;
        let mut cloud_light = 0.0;
        if m > cos_r {
            let u = d.dot(self.moon_u);
            let v = d.dot(self.moon_v);
            let r = (1.0 - cos_r * cos_r).sqrt();
            let (lu, lv) = (u / r, v / r);
            let rr = (lu * lu + lv * lv).min(1.0);
            let crater = fbm(lu * 3.0 + 10.0, lv * 3.0 + 10.0, 3, 33);
            let limb = 1.0 - rr * 0.35;
            let k = (0.72 + 0.4 * crater) * limb;
            c = [0.98 * k, 0.95 * k, 0.82 * k];
        } else if m > 0.9 {
            let g = ((m - 0.9) / (cos_r - 0.9)).powi(6) * 0.4 + ((m - 0.9) / 0.1).powi(2) * 0.08;
            c = [c[0] + g * 0.8, c[1] + g * 0.82, c[2] + g * 0.95];
            cloud_light = g;
        }
        // Nuages effilochés.
        if e > 0.015 {
            let px = d.x / (e + 0.08);
            let pz = d.z / (e + 0.08);
            let n = fbm(px * 0.35 + time * 0.012, pz * 0.9, 4, 44);
            let k = smoothstep(0.52, 0.72, n) * 0.85;
            let cc = [0.045 + cloud_light * 1.5, 0.05 + cloud_light * 1.5, 0.065 + cloud_light * 1.6];
            c = [c[0] + (cc[0] - c[0]) * k, c[1] + (cc[1] - c[1]) * k, c[2] + (cc[2] - c[2]) * k];
        }
        c
    }
}

impl Default for Outdoor {
    fn default() -> Self {
        Self::new()
    }
}
