//! L'extérieur visible par les fenêtres : marais, arbres morts, grille,
//! montagnes, ciel nocturne et lune.

use crate::math::*;
use crate::mesh::*;
use crate::raster::{Fog, Lighting};
use crate::textures::*;

pub struct Outdoor {
    pub ground: Mesh,
    pub mountains: Mesh,
    /// Extérieur du manoir, vu seulement depuis le parc et le marais.
    pub facade: Mesh,
    /// Arbres morts (x, z, rayon du tronc).
    pub trees: Vec<(f32, f32, f32)>,
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

/// Chemin de terre du portail au ponton, à travers le marais.
const PATH: [(f32, f32); 7] = [(0.0, -8.0), (1.5, -15.0), (-3.0, -22.0), (-2.0, -29.0), (2.5, -36.0), (4.0, -41.0), (4.0, -43.0)];

/// Ponton (x0, z0, x1, z1) et hauteur de son plancher.
pub const PIER: (f32, f32, f32, f32) = (3.0, -47.2, 5.0, -42.0);
pub const PIER_Y: f32 = -0.28;
/// Hauteur de la cour pavée devant le perron.
pub const COURT_Y: f32 = -0.42;

fn seg_dist(x: f32, z: f32, a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dz) = (b.0 - a.0, b.1 - a.1);
    let t = (((x - a.0) * dx + (z - a.1) * dz) / (dx * dx + dz * dz)).clamp(0.0, 1.0);
    let (px, pz) = (a.0 + dx * t, a.1 + dz * t);
    ((x - px).powi(2) + (z - pz).powi(2)).sqrt()
}

/// Distance au chemin du marais.
pub fn path_dist(x: f32, z: f32) -> f32 {
    PATH.windows(2).map(|w| seg_dist(x, z, w[0], w[1])).fold(f32::MAX, f32::min)
}

/// 1 dans la cour devant le manoir, 0 au-delà (transition douce).
fn court(x: f32, z: f32) -> f32 {
    let dx = (x.abs() - 12.0).max(0.0);
    let dz = (-9.6 - z).max(0.0).max(z - 0.5);
    1.0 - smoothstep(0.0, 2.5, (dx * dx + dz * dz).sqrt())
}

fn ground_h(x: f32, z: f32) -> f32 {
    let n = water_n(x, z);
    let bumps = (fbm(x / 5.0, z / 5.0, 2, 78) - 0.5) * 0.35;
    let far = ((x * x + z * z).sqrt() - 120.0).max(0.0) * 0.04;
    let mut h = -0.75 + smoothstep(0.36, 0.46, n) * (0.25 + bumps) + far;
    // Chemin surélevé à travers l'eau.
    let path = 1.0 - smoothstep(1.4, 3.0, path_dist(x, z));
    h += (-0.38 + bumps * 0.3 - h).max(0.0) * path;
    let c = court(x, z);
    h + (COURT_Y - h) * c
}

/// Le terrain est-il de l'eau à cet endroit ?
fn is_water(x: f32, z: f32) -> bool {
    water_n(x, z) < 0.40 && path_dist(x, z) > 2.2 && court(x, z) < 0.5
}

/// Grille fine autour des zones jouables (parc et marais).
const FINE: (f32, f32, f32, f32) = (-15.0, -50.0, 15.0, 0.0);
const FINE_CELL: f32 = 1.25;
const CELL: f32 = 2.5;

/// Hauteur exacte du maillage de terrain (interpolation par triangle).
pub fn terrain_y(x: f32, z: f32) -> f32 {
    let fine = x >= FINE.0 && x < FINE.2 && z >= FINE.1 && z < FINE.3;
    let (ox, oz, cell) = if fine { (FINE.0, FINE.1, FINE_CELL) } else { (-50.0, -50.0, CELL) };
    let gx = (x - ox) / cell;
    let gz = (z - oz) / cell;
    let (i, j) = (gx.floor(), gz.floor());
    let (fx, fz) = (gx - i, gz - j);
    let (x0, z0) = (ox + i * cell, oz + j * cell);
    let h00 = ground_h(x0, z0);
    let h10 = ground_h(x0 + cell, z0);
    let h11 = ground_h(x0 + cell, z0 + cell);
    let h01 = ground_h(x0, z0 + cell);
    // Triangles (00, 10, 11) et (00, 11, 01).
    if fx >= fz {
        h00 + (h10 - h00) * fx + (h11 - h10) * fz
    } else {
        h00 + (h01 - h00) * fz + (h11 - h01) * fx
    }
}

/// Hauteur des pieds : terrain, ponton ou perron.
pub fn walk_h(x: f32, z: f32) -> f32 {
    if x >= PIER.0 && x <= PIER.2 && z >= PIER.1 && z <= PIER.3 {
        return PIER_Y;
    }
    if x.abs() < 2.0 && z > -1.4 {
        return -0.05;
    }
    terrain_y(x, z)
}

/// Eau assez profonde pour ralentir la marche.
pub fn wading(x: f32, z: f32) -> bool {
    walk_h(x, z) < -0.62
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
        // Terrain : grille fine autour des zones jouables, moyenne près du manoir,
        // grossière au loin.
        let add_grid = |g: &mut Mesh, x0: f32, z0: f32, nx: i32, nz: i32, cell: f32, skip: &dyn Fn(f32, f32) -> bool| {
            for j in 0..nz {
                for i in 0..nx {
                    let x0 = x0 + i as f32 * cell;
                    let z0 = z0 + j as f32 * cell;
                    let (x1, z1) = (x0 + cell, z0 + cell);
                    let cx = (x0 + x1) * 0.5;
                    let cz = (z0 + z1) * 0.5;
                    if skip(cx, cz) || in_manor(cx, cz, -1.0) {
                        continue;
                    }
                    let water = is_water(cx, cz);
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
                            col = [0.06 + 0.03 * k2, 0.08 + 0.035 * k2, 0.11 + 0.06 * k2];
                        } else if path_dist(c.x, c.z) < 1.6 || court(c.x, c.z) > 0.5 {
                            // Terre battue et gravier.
                            col = [0.2, 0.18, 0.15];
                        }
                        g.v.push(MV { p: *c, n: Vec3::UP, uv, c: col, glow: 0.0 });
                    }
                    g.t.push(MT { i: [base, base + 1, base + 2], tex, blend: Blend::Opaque, two_sided: false });
                    g.t.push(MT { i: [base, base + 2, base + 3], tex, blend: Blend::Opaque, two_sided: false });
                }
            }
        };
        let in_fine = |x: f32, z: f32| x > FINE.0 && x < FINE.2 && z > FINE.1 && z < FINE.3;
        let fine_n = |a: f32, b: f32| ((b - a) / FINE_CELL) as i32;
        add_grid(&mut ground, FINE.0, FINE.1, fine_n(FINE.0, FINE.2), fine_n(FINE.1, FINE.3), FINE_CELL, &|_, _| false);
        add_grid(&mut ground, -50.0, -50.0, 40, 40, CELL, &in_fine);
        add_grid(&mut ground, -300.0, -300.0, 40, 40, 15.0, &|x, z| x.abs() < 50.0 && z.abs() < 50.0);

        // Arbres morts.
        let bark = Mat::tex(T_BARK, [0.55, 0.5, 0.45]);
        let mut placed = 0;
        let mut trees = Vec::new();
        while placed < 34 {
            let a = rng.range(0.0, std::f32::consts::TAU);
            let r = rng.range(16.0, 95.0);
            let x = -2.0 + a.sin() * r;
            let z = 13.0 + a.cos() * r;
            if in_manor(x, z, 4.0) || path_dist(x, z) < 3.5 || court(x, z) > 0.0 || (x.abs() < 6.0 && z < -40.0) {
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
            trees.push((x, z, w * 0.5 + 0.1));
        }

        // Roseaux autour des mares.
        let reed = Mat::col([0.16, 0.18, 0.08]).two();
        let mut clusters = 0;
        let mut tries = 0;
        while clusters < 260 && tries < 20000 {
            tries += 1;
            let x = rng.range(-48.0, 48.0);
            let z = rng.range(-35.0, 60.0);
            if in_manor(x, z, 1.5) || path_dist(x, z) < 1.8 || court(x, z) > 0.0 {
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
        // Allée pavée menant au perron.
        let gravel = Mat::tex(T_STONE, [0.38, 0.37, 0.36]).decal();
        ground.grid(v3(-1.5, COURT_Y + 0.01, fz0 + 0.3), v3(3.0, 0.0, 0.0), v3(0.0, 0.0, -1.4 - fz0 - 0.3), [0.0, 0.0], [1.5, 4.0], 1.25, &gravel, Vec3::UP);
        // Perron.
        ground.cube(v3(-2.0, -0.7, -1.4), v3(2.0, -0.05, 0.0), &stone, 1.0);

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

        let mut facade = facade();
        facade.prelight(&|p, n| moonlight.at(p, n));

        Outdoor { ground, mountains, facade, trees, moon_dir, moon_u, moon_v, fireflies }
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

/// Façade du manoir : murs de pierre, fenêtres éclairées, toiture, portique.
fn facade() -> Mesh {
    let mut m = Mesh::new();
    let stone = Mat::tex(T_STONE, [0.55, 0.55, 0.6]);
    let roof = Mat::tex(T_STONE, [0.22, 0.22, 0.27]);
    let (x0, x1, z0, z1, h) = (-18.0f32, 14.0f32, 0.0f32, 27.0f32, 9.0f32);
    let up = Vec3::UP;
    m.wall(v3(x0, -1.0, z0), v3(x1 - x0, 0.0, 0.0), up * (h + 1.0), 0.5, 0.0, &stone, v3(0.0, 0.0, -1.0));
    m.wall(v3(x1, -1.0, z0), v3(0.0, 0.0, z1 - z0), up * (h + 1.0), 0.5, 0.0, &stone, v3(1.0, 0.0, 0.0));
    m.wall(v3(x0, -1.0, z1), v3(0.0, 0.0, z0 - z1), up * (h + 1.0), 0.5, 0.0, &stone, v3(-1.0, 0.0, 0.0));
    m.wall(v3(x1, -1.0, z1), v3(x0 - x1, 0.0, 0.0), up * (h + 1.0), 0.5, 0.0, &stone, v3(0.0, 0.0, 1.0));
    // Toit à deux pans, pignons, corps central plus haut.
    let ridge = 13.5;
    let zm = (z0 + z1) * 0.5;
    m.quad([v3(x0, h, z0), v3(x1, h, z0), v3(x1, ridge, zm), v3(x0, ridge, zm)], [[0.0, 0.0], [8.0, 0.0], [8.0, 3.0], [0.0, 3.0]], &roof, v3(0.0, 1.0, -1.0));
    m.quad([v3(x0, h, z1), v3(x1, h, z1), v3(x1, ridge, zm), v3(x0, ridge, zm)], [[0.0, 0.0], [8.0, 0.0], [8.0, 3.0], [0.0, 3.0]], &roof, v3(0.0, 1.0, 1.0));
    for x in [x0, x1] {
        m.tri([v3(x, h, z0), v3(x, h, z1), v3(x, ridge, zm)], [[0.0, 0.0], [6.0, 0.0], [3.0, 2.0]], &stone, v3(x.signum(), 0.0, 0.0));
    }
    // Corps central en avancée avec fronton.
    m.cube_faces(v3(-6.5, -1.0, -0.6), v3(6.5, 10.5, 0.0), &stone, 0.5, [true, true, false, true, true, false]);
    m.tri([v3(-6.5, 10.5, -0.6), v3(6.5, 10.5, -0.6), v3(0.0, 12.8, -0.6)], [[0.0, 0.0], [4.0, 0.0], [2.0, 1.0]], &stone, v3(0.0, 0.0, -1.0));
    m.quad([v3(-6.5, 10.5, -0.6), v3(0.0, 12.8, -0.6), v3(0.0, 12.8, 1.0), v3(-6.5, 10.5, 1.0)], [[0.0; 2]; 4], &roof, v3(-0.3, 1.0, 0.0));
    m.quad([v3(6.5, 10.5, -0.6), v3(0.0, 12.8, -0.6), v3(0.0, 12.8, 1.0), v3(6.5, 10.5, 1.0)], [[0.0; 2]; 4], &roof, v3(0.3, 1.0, 0.0));
    // Fenêtres : chaudes au rez-de-chaussée, une seule éclairée à l'étage.
    let warm = Mat::col([0.5, 0.32, 0.13]).glow(0.9);
    let cold = Mat::col([0.04, 0.05, 0.08]);
    let n = v3(0.0, 0.0, -1.0);
    let win = |m: &mut Mesh, xa: f32, xb: f32, ya: f32, yb: f32, z: f32, lit: bool| {
        let mat = if lit { &warm } else { &cold };
        m.quad([v3(xa, ya, z), v3(xb, ya, z), v3(xb, yb, z), v3(xa, yb, z)], [[0.0; 2]; 4], mat, n);
        let bar = Mat::col([0.05, 0.04, 0.03]);
        let xm = (xa + xb) * 0.5;
        m.quad([v3(xm - 0.04, ya, z - 0.01), v3(xm + 0.04, ya, z - 0.01), v3(xm + 0.04, yb, z - 0.01), v3(xm - 0.04, yb, z - 0.01)], [[0.0; 2]; 4], &bar, n);
        m.quad([v3(xa, ya - 0.12, z - 0.15), v3(xb, ya - 0.12, z - 0.15), v3(xb, ya, z - 0.15), v3(xa, ya, z - 0.15)], [[0.0; 2]; 4], &Mat::tex(T_STONE, [0.7, 0.7, 0.72]), n);
    };
    for (xa, xb) in [(-4.8f32, -2.6f32), (2.6, 4.8)] {
        win(&mut m, xa, xb, 1.0, 5.6, -0.62, true);
        win(&mut m, xa, xb, 7.0, 9.2, -0.62, false);
    }
    for (xa, xb, lit) in [(-15.5f32, -13.5f32, true), (-10.5, -8.5, true), (8.0, 10.0, false)] {
        win(&mut m, xa, xb, 1.0, 3.3, -0.02, lit);
        win(&mut m, xa, xb, 5.6, 7.6, -0.02, xa > 0.0);
    }
    // Porte d'entrée et portique.
    let door = Mat::tex(T_DOOR, [0.8, 0.8, 0.8]);
    m.grid(v3(-1.2, -0.05, -0.63), v3(1.2, 0.0, 0.0), up * 3.2, [0.0, 1.0], [1.0, -1.0], 0.6, &door, n);
    m.grid(v3(0.0, -0.05, -0.63), v3(1.2, 0.0, 0.0), up * 3.2, [1.0, 1.0], [-1.0, -1.0], 0.6, &door, n);
    for x in [-2.0f32, 1.7] {
        m.cube(v3(x, -0.05, -1.4), v3(x + 0.3, 3.8, -1.1), &stone, 1.0);
    }
    m.cube(v3(-2.2, 3.8, -1.55), v3(2.2, 4.2, -0.6), &stone, 1.0);
    m
}

impl Default for Outdoor {
    fn default() -> Self {
        Self::new()
    }
}
