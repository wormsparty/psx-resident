//! Le manoir : définition des pièces, de leur géométrie, des portes,
//! des caméras fixes, des lumières et des objets.

use crate::math::*;
use crate::mesh::*;
use crate::raster::{Fog, Light};
use crate::textures::*;
use std::f32::consts::{FRAC_PI_2, PI};

pub const KEY_EMBLEM: u8 = 0;

pub const ROOM_HALL: usize = 0;
pub const ROOM_DINING: usize = 1;
pub const ROOM_CORRIDOR: usize = 2;
pub const ROOM_STUDY: usize = 3;
pub const ROOM_LIBRARY: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ItemKind {
    Ammo(u32),
    Medkit,
    Key(u8),
}

impl ItemKind {
    pub fn name(&self) -> String {
        match self {
            ItemKind::Ammo(n) => format!("MUNITIONS 9MM ({n})"),
            ItemKind::Medkit => "TROUSSE DE SOINS".to_string(),
            ItemKind::Key(_) => "CLÉ AU BLASON".to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ItemSpawn {
    pub room: usize,
    pub pos: Vec3,
    pub kind: ItemKind,
}

#[derive(Clone, Debug)]
pub struct ZombieSpawn {
    pub room: usize,
    pub pos: Vec3,
    pub yaw: f32,
    pub eating: bool,
}

#[derive(Clone, Debug)]
pub struct Door {
    /// Centre de la porte au sol, dans le plan du mur.
    pub pos: Vec3,
    /// Normale vers l'intérieur de la pièce.
    pub normal: Vec3,
    pub to_room: usize,
    pub to_pos: Vec3,
    pub to_yaw: f32,
    /// Identifiant de serrure et clé requise.
    pub lock: Option<(usize, u8)>,
}

#[derive(Clone, Debug)]
pub enum InteractKind {
    Message(&'static str),
    Typewriter,
    Window,
    Note,
}

#[derive(Clone, Debug)]
pub struct Interact {
    pub pos: Vec3,
    pub radius: f32,
    pub kind: InteractKind,
}

#[derive(Clone, Debug)]
pub struct CamZone {
    pub min: (f32, f32),
    pub max: (f32, f32),
    pub eye: Vec3,
    pub target: Vec3,
    pub fov: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Obstacle {
    pub min: (f32, f32),
    pub max: (f32, f32),
    /// Bloque aussi les balles.
    pub tall: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct RoomLight {
    pub light: Light,
    pub flicker: f32,
}

pub struct Room {
    pub name: &'static str,
    pub mesh: Mesh,
    /// Triangles transparents (vitres), dessinés après le paysage.
    pub trans: Mesh,
    pub min: (f32, f32),
    pub max: (f32, f32),
    pub obstacles: Vec<Obstacle>,
    pub cameras: Vec<CamZone>,
    pub lights: Vec<RoomLight>,
    pub ambient: [f32; 3],
    pub doors: Vec<Door>,
    pub interacts: Vec<Interact>,
    /// Coins extérieurs des fenêtres (pour le rendu du paysage).
    pub windows: Vec<[Vec3; 4]>,
    pub flames: Vec<(Vec3, f32)>,
    pub corpses: Vec<(Vec3, f32)>,
    pub fog: Fog,
}

pub struct World {
    pub rooms: Vec<Room>,
    pub items: Vec<ItemSpawn>,
    pub zombies: Vec<ZombieSpawn>,
}

// ------------------------------------------------------------------ construction

#[derive(Clone, Copy, PartialEq)]
enum Side {
    S,
    N,
    W,
    E,
}

#[derive(Clone, Copy, PartialEq)]
enum OpenKind {
    Door,
    DoubleDoor,
    Window,
}

#[derive(Clone, Copy)]
struct Opening {
    side: Side,
    from: f32,
    to: f32,
    bottom: f32,
    top: f32,
    kind: OpenKind,
}

fn op(side: Side, from: f32, to: f32, bottom: f32, top: f32, kind: OpenKind) -> Opening {
    Opening { side, from, to, bottom, top, kind }
}

struct Style {
    lower: Mat,
    upper: Mat,
    floor: Mat,
    ceil: Mat,
    wainscot_h: f32,
    floor_tpm: f32,
}

struct Builder {
    mesh: Mesh,
    obstacles: Vec<Obstacle>,
    lights: Vec<RoomLight>,
    interacts: Vec<Interact>,
    windows: Vec<[Vec3; 4]>,
    flames: Vec<(Vec3, f32)>,
    min: (f32, f32),
    max: (f32, f32),
    h: f32,
}

const WALL_DEPTH: f32 = 0.3;

impl Builder {
    fn new(min: (f32, f32), max: (f32, f32), h: f32) -> Builder {
        Builder {
            mesh: Mesh::new(),
            obstacles: Vec::new(),
            lights: Vec::new(),
            interacts: Vec::new(),
            windows: Vec::new(),
            flames: Vec::new(),
            min,
            max,
            h,
        }
    }

    /// Origine, axe le long du mur et normale intérieure d'un côté.
    fn side_frame(&self, side: Side) -> (Vec3, Vec3, Vec3, f32) {
        let (x0, z0) = self.min;
        let (x1, z1) = self.max;
        match side {
            Side::S => (v3(x0, 0.0, z0), v3(1.0, 0.0, 0.0), v3(0.0, 0.0, 1.0), x1 - x0),
            Side::N => (v3(x0, 0.0, z1), v3(1.0, 0.0, 0.0), v3(0.0, 0.0, -1.0), x1 - x0),
            Side::W => (v3(x0, 0.0, z0), v3(0.0, 0.0, 1.0), v3(1.0, 0.0, 0.0), z1 - z0),
            Side::E => (v3(x1, 0.0, z0), v3(0.0, 0.0, 1.0), v3(-1.0, 0.0, 0.0), z1 - z0),
        }
    }

    fn shell(&mut self, style: &Style, openings: &[Opening]) {
        let (x0, z0) = self.min;
        let (x1, z1) = self.max;
        let h = self.h;
        // Sol et plafond.
        self.mesh.grid(
            v3(x0, 0.0, z0),
            v3(x1 - x0, 0.0, 0.0),
            v3(0.0, 0.0, z1 - z0),
            [x0 * style.floor_tpm, z0 * style.floor_tpm],
            [(x1 - x0) * style.floor_tpm, (z1 - z0) * style.floor_tpm],
            1.0,
            &style.floor,
            Vec3::UP,
        );
        self.mesh.grid(
            v3(x0, h, z0),
            v3(x1 - x0, 0.0, 0.0),
            v3(0.0, 0.0, z1 - z0),
            [x0 * 0.5, z0 * 0.5],
            [(x1 - x0) * 0.5, (z1 - z0) * 0.5],
            1.5,
            &style.ceil,
            -Vec3::UP,
        );

        for side in [Side::S, Side::N, Side::W, Side::E] {
            let (o, ax, n, len) = self.side_frame(side);
            let ops: Vec<&Opening> = openings.iter().filter(|o| o.side == side).collect();
            let mut cuts = vec![0.0, len];
            for p in &ops {
                cuts.push(p.from);
                cuts.push(p.to);
            }
            cuts.sort_by(|a, b| a.partial_cmp(b).unwrap());
            cuts.dedup_by(|a, b| (*a - *b).abs() < 1e-4);
            for w in cuts.windows(2) {
                let (sa, sb) = (w[0], w[1]);
                if sb - sa < 1e-4 {
                    continue;
                }
                let mid = (sa + sb) * 0.5;
                // Intervalles verticaux pleins.
                let mut solid = vec![(0.0f32, h)];
                for p in &ops {
                    if p.from <= mid && p.to >= mid {
                        let mut next = Vec::new();
                        for &(a, b) in &solid {
                            if p.top <= a || p.bottom >= b {
                                next.push((a, b));
                            } else {
                                if p.bottom > a {
                                    next.push((a, p.bottom));
                                }
                                if p.top < b {
                                    next.push((p.top, b));
                                }
                            }
                        }
                        solid = next;
                    }
                }
                for (ya, yb) in solid {
                    // Coupure lambris / papier peint.
                    let wh = style.wainscot_h;
                    let mut parts = Vec::new();
                    if ya < wh && yb > wh {
                        parts.push((ya, wh, &style.lower, 1.0));
                        parts.push((wh, yb, &style.upper, 1.0));
                    } else if yb <= wh {
                        parts.push((ya, yb, &style.lower, 1.0));
                    } else {
                        parts.push((ya, yb, &style.upper, 1.0));
                    }
                    for (a, b, m, tpm) in parts {
                        let m = *m;
                        let base = o + ax * sa + Vec3::UP * a;
                        self.mesh.wall(base, ax * (sb - sa), Vec3::UP * (b - a), tpm, sa, &m, n);
                    }
                }
            }
            // Cimaise (moulure) le long du mur.
            let rail = Mat::tex(T_WOOD, [0.55, 0.4, 0.3]);
            for w in cuts.windows(2) {
                let (sa, sb) = (w[0], w[1]);
                let mid = (sa + sb) * 0.5;
                let blocked = ops.iter().any(|p| p.from <= mid && p.to >= mid && p.bottom <= style.wainscot_h && p.top >= style.wainscot_h);
                if !blocked && sb - sa > 1e-3 {
                    let y = style.wainscot_h;
                    let a = o + ax * sa + n * 0.04 + Vec3::UP * y;
                    let du = ax * (sb - sa);
                    self.mesh.quad([a, a + du, a + du + Vec3::UP * 0.06, a + Vec3::UP * 0.06], [[0.0, 0.0], [1.0, 0.0], [1.0, 0.1], [0.0, 0.1]], &rail, n);
                    self.mesh.quad([a + Vec3::UP * 0.06, a + du + Vec3::UP * 0.06, a + du + Vec3::UP * 0.06 - n * 0.04, a + Vec3::UP * 0.06 - n * 0.04], [[0.0, 0.0], [1.0, 0.0], [1.0, 0.1], [0.0, 0.1]], &rail, Vec3::UP);
                }
            }
            for p in &ops {
                self.opening(p, o, ax, n);
            }
        }
    }

    fn opening(&mut self, p: &Opening, o: Vec3, ax: Vec3, n: Vec3) {
        let a = o + ax * p.from;
        let b = o + ax * p.to;
        let up = Vec3::UP;
        let out = -n * WALL_DEPTH;
        let jamb = Mat::tex(T_WOOD, [0.6, 0.45, 0.35]);
        match p.kind {
            OpenKind::Window => {
                let (yb, yt) = (p.bottom, p.top);
                // Embrasure : appui, linteau, côtés.
                let sill = Mat::tex(T_STONE, [0.7, 0.7, 0.72]);
                let ab = a + up * yb;
                let bb = b + up * yb;
                let at = a + up * yt;
                let bt = b + up * yt;
                self.mesh.quad([ab, bb, bb + out, ab + out], [[0.0, 0.0], [1.0, 0.0], [1.0, 0.3], [0.0, 0.3]], &sill, up);
                self.mesh.quad([at, bt, bt + out, at + out], [[0.0, 0.0], [1.0, 0.0], [1.0, 0.3], [0.0, 0.3]], &sill, -up);
                self.mesh.quad([ab, at, at + out, ab + out], [[0.0, 0.0], [1.0, 0.0], [1.0, 0.3], [0.0, 0.3]], &sill, ax);
                self.mesh.quad([bb, bt, bt + out, bb + out], [[0.0, 0.0], [1.0, 0.0], [1.0, 0.3], [0.0, 0.3]], &sill, -ax);
                // Rebord intérieur.
                let ledge = Mat::tex(T_WOOD, [0.7, 0.55, 0.45]);
                // Limité à l'ouverture et moins profond que les rideaux (0,12 m),
                // sinon il dépasse et semble flotter devant eux.
                let (l0, l1, d) = (ab, bb, 0.1);
                self.mesh.quad([l0, l1, l1 + n * d, l0 + n * d], [[0.0, 0.0], [1.0, 0.0], [1.0, 0.2], [0.0, 0.2]], &ledge, up);
                self.mesh.quad([l0 - up * 0.07 + n * d, l1 - up * 0.07 + n * d, l1 + n * d, l0 + n * d], [[0.0, 0.0], [1.0, 0.0], [1.0, 0.1], [0.0, 0.1]], &ledge, n);
                // Croisillons (meneaux) au milieu de l'épaisseur.
                let mid = out * 0.5;
                let bar = Mat::col([0.12, 0.09, 0.07]).two();
                let t = 0.04;
                {
                    let c = a.lerp(b, 0.5);
                    self.mesh.quad(
                        [c - ax * t + up * yb + mid, c + ax * t + up * yb + mid, c + ax * t + up * yt + mid, c - ax * t + up * yt + mid],
                        [[0.0; 2]; 4],
                        &bar,
                        n,
                    );
                }
                for k in [0.33f32, 0.66] {
                    let y = yb + (yt - yb) * k;
                    self.mesh.quad(
                        [a + up * (y - t) + mid, b + up * (y - t) + mid, b + up * (y + t) + mid, a + up * (y + t) + mid],
                        [[0.0; 2]; 4],
                        &bar,
                        n,
                    );
                }
                // Vitre : léger reflet additif bleuté.
                let glass = Mat::col([0.035, 0.045, 0.06]).add().two().glow(1.0);
                self.mesh.quad([ab + mid * 1.02, bb + mid * 1.02, bt + mid * 1.02, at + mid * 1.02], [[0.0; 2]; 4], &glass, n);
                // Rideaux.
                let cur = Mat::tex(T_CURTAIN, [1.0, 1.0, 1.0]).two();
                let ctop = (yt + 0.35).min(self.h - 0.05);
                for (s0, s1) in [(-0.45f32, 0.12f32), (-0.12, 0.45)] {
                    let base = if s0 < -0.2 { a } else { b };
                    let p0 = base + ax * s0 + n * 0.12;
                    let p1 = base + ax * s1 + n * 0.12;
                    self.mesh.quad(
                        [p0 + up * 0.05, p1 + up * 0.05, p1 + up * ctop, p0 + up * ctop],
                        [[0.0, 3.0], [1.0, 3.0], [1.0, 0.0], [0.0, 0.0]],
                        &cur,
                        n,
                    );
                }
                // Tringle.
                let rod = Mat::col([0.5, 0.4, 0.2]);
                let r0 = a - ax * 0.5 + n * 0.12 + up * ctop;
                let r1 = b + ax * 0.5 + n * 0.12 + up * ctop;
                self.mesh.quad([r0 - up * 0.04, r1 - up * 0.04, r1 + up * 0.04, r0 + up * 0.04], [[0.0; 2]; 4], &rod, n);

                self.windows.push([ab + out, bb + out, bt + out, at + out]);
                let center = (a + b) * 0.5 + n * 0.5;
                self.interacts.push(Interact { pos: center, radius: 1.1, kind: InteractKind::Window });
            }
            OpenKind::Door | OpenKind::DoubleDoor => {
                let yt = p.top;
                let rec = -n * 0.1;
                // Embrasure peu profonde.
                let ab = a;
                let bb = b;
                let at = a + up * yt;
                let bt = b + up * yt;
                self.mesh.quad([at, bt, bt + rec, at + rec], [[0.0; 2]; 4], &jamb, -up);
                self.mesh.quad([ab, at, at + rec, ab + rec], [[0.0; 2]; 4], &jamb, ax);
                // Seuil (sinon on voit un interstice sous la porte).
                self.mesh.quad([ab, bb, bb + rec, ab + rec], [[0.0; 2]; 4], &jamb, up);
                self.mesh.quad([bb, bt, bt + rec, bb + rec], [[0.0; 2]; 4], &jamb, -ax);
                // Battant(s).
                let door = Mat::tex(T_DOOR, [1.0, 1.0, 1.0]);
                let leaves: Vec<(Vec3, Vec3, bool)> = if p.kind == OpenKind::DoubleDoor {
                    let m = a.lerp(b, 0.5);
                    vec![(a, m, false), (m, b, true)]
                } else {
                    vec![(a, b, false)]
                };
                for (l0, l1, flip) in leaves {
                    let (u0, u1) = if flip { (1.0, 0.0) } else { (0.0, 1.0) };
                    self.mesh.grid(l0 + rec, l1 - l0, up * yt, [u0, 1.0], [u1 - u0, -1.0], 0.5, &door, n);
                }
                // Chambranle mouluré.
                let fw = 0.12;
                let trim = Mat::tex(T_WOOD, [0.45, 0.32, 0.25]);
                let f = n * 0.03;
                let q = |m: &mut Mesh, p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3| {
                    m.quad([p0 + f, p1 + f, p2 + f, p3 + f], [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]], &trim, n);
                };
                q(&mut self.mesh, a - ax * fw, a, at, at - ax * fw + up * fw * 0.0);
                q(&mut self.mesh, b, b + ax * fw, bt + ax * fw, bt);
                q(&mut self.mesh, at - ax * fw, bt + ax * fw, bt + ax * fw + up * fw, at - ax * fw + up * fw);
            }
        }
    }

    fn obstacle(&mut self, x0: f32, z0: f32, x1: f32, z1: f32, tall: bool) {
        self.obstacles.push(Obstacle { min: (x0.min(x1), z0.min(z1)), max: (x0.max(x1), z0.max(z1)), tall });
    }

    fn light(&mut self, pos: Vec3, color: [f32; 3], radius: f32, flicker: f32) {
        self.lights.push(RoomLight { light: Light { pos, color, radius }, flicker });
    }

    fn msg(&mut self, x: f32, z: f32, r: f32, text: &'static str) {
        self.interacts.push(Interact { pos: v3(x, 0.0, z), radius: r, kind: InteractKind::Message(text) });
    }

    // ---------------------------------------------------------- mobilier

    fn table(&mut self, x0: f32, z0: f32, x1: f32, z1: f32, h: f32, top: Mat, cloth: bool) {
        let m = Mat::tex(T_WOOD, [0.7, 0.55, 0.45]);
        self.mesh.cube(v3(x0, h - 0.06, z0), v3(x1, h, z1), &top, 1.0);
        if cloth {
            let c = Mat::tex(T_CLOTH, [1.0, 1.0, 1.0]);
            let d = 0.05;
            self.mesh.cube_faces(v3(x0 - d, h - 0.3, z0 - d), v3(x1 + d, h + 0.005, z1 + d), &c, 1.0, [true, true, false, true, true, true]);
        }
        let l = 0.07;
        for &(x, z) in &[(x0 + 0.05, z0 + 0.05), (x1 - 0.05 - l, z0 + 0.05), (x0 + 0.05, z1 - 0.05 - l), (x1 - 0.05 - l, z1 - 0.05 - l)] {
            self.mesh.cube_faces(v3(x, 0.0, z), v3(x + l, h - 0.06, z + l), &m, 1.0, [true, true, false, false, true, true]);
        }
        self.obstacle(x0, z0, x1, z1, false);
    }

    fn chair(&mut self, x: f32, z: f32, yaw: f32) {
        let m = Mat::tex(T_WOOD, [0.55, 0.4, 0.33]);
        let mut c = Mesh::new();
        c.cube(v3(-0.22, 0.42, -0.22), v3(0.22, 0.48, 0.22), &m, 1.0);
        c.cube(v3(-0.22, 0.48, -0.22), v3(0.22, 1.1, -0.17), &m, 1.0);
        for &(lx, lz) in &[(-0.2, -0.2), (0.15, -0.2), (-0.2, 0.15), (0.15, 0.15)] {
            c.cube_faces(v3(lx, 0.0, lz), v3(lx + 0.05, 0.42, lz + 0.05), &m, 1.0, [true, true, false, false, true, true]);
        }
        self.mesh.append(&c, &(Mat4::translate(v3(x, 0.0, z)) * Mat4::rot_y(yaw)));
        self.obstacle(x - 0.25, z - 0.25, x + 0.25, z + 0.25, false);
    }

    /// Bibliothèque. `fronts` : faces portant les livres ([-x, +x, -z, +z]).
    fn bookshelf(&mut self, x0: f32, z0: f32, x1: f32, z1: f32, h: f32, fronts: [bool; 4]) {
        let wood = Mat::tex(T_WOOD, [0.45, 0.32, 0.25]);
        let books = Mat::tex(T_BOOKS, [1.0, 1.0, 1.0]);
        self.mesh.cube_faces(v3(x0, 0.0, z0), v3(x1, h, z1), &wood, 1.0, [!fronts[0], !fronts[1], false, true, !fronts[2], !fronts[3]]);
        let up = Vec3::UP;
        let tpm = 0.5;
        if fronts[0] {
            self.mesh.wall(v3(x0, 0.0, z1), v3(0.0, 0.0, z0 - z1), up * h, tpm, 0.0, &books, v3(-1.0, 0.0, 0.0));
        }
        if fronts[1] {
            self.mesh.wall(v3(x1, 0.0, z0), v3(0.0, 0.0, z1 - z0), up * h, tpm, 0.0, &books, v3(1.0, 0.0, 0.0));
        }
        if fronts[2] {
            self.mesh.wall(v3(x0, 0.0, z0), v3(x1 - x0, 0.0, 0.0), up * h, tpm, 0.0, &books, v3(0.0, 0.0, -1.0));
        }
        if fronts[3] {
            self.mesh.wall(v3(x1, 0.0, z1), v3(x0 - x1, 0.0, 0.0), up * h, tpm, 0.0, &books, v3(0.0, 0.0, 1.0));
        }
        self.obstacle(x0, z0, x1, z1, true);
    }

    fn painting(&mut self, center: Vec3, n: Vec3, w: f32, h: f32, tex: i16) {
        let ax = Vec3::UP.cross(n).norm();
        let c = center + n * 0.03;
        let hw = ax * (w * 0.5);
        let hh = Vec3::UP * (h * 0.5);
        let m = Mat::tex(tex, [1.0, 1.0, 1.0]);
        // Vu de face, `ax` pointe vers la gauche de l'observateur.
        self.mesh.quad([c + hw - hh, c - hw - hh, c - hw + hh, c + hw + hh], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]], &m, n);
    }

    fn candle_light(&mut self, pos: Vec3, radius: f32) {
        let wax = Mat::col([0.9, 0.85, 0.7]).glow(0.6);
        self.mesh.cube(pos - v3(0.025, 0.12, 0.025), pos + v3(0.025, 0.0, 0.025), &wax, 1.0);
        self.flames.push((pos + v3(0.0, 0.05, 0.0), 0.05));
        self.light(pos + v3(0.0, 0.1, 0.0), [0.75, 0.48, 0.22], radius, 0.25);
    }

    fn sconce(&mut self, pos: Vec3, n: Vec3, radius: f32) {
        let brass = Mat::col([0.55, 0.42, 0.2]);
        self.mesh.cube(pos - v3(0.06, 0.25, 0.06) - n * 0.0, pos + v3(0.06, -0.15, 0.06), &brass, 1.0);
        self.candle_light(pos + n * 0.08, radius);
    }

    fn moon_spill(&mut self, pos: Vec3, radius: f32) {
        self.light(pos, [0.16, 0.2, 0.34], radius, 0.0);
    }

    fn build(self, name: &'static str, cameras: Vec<CamZone>, doors: Vec<Door>, ambient: [f32; 3], corpses: Vec<(Vec3, f32)>) -> Room {
        Room {
            name,
            mesh: self.mesh,
            trans: Mesh::new(),
            min: self.min,
            max: self.max,
            obstacles: self.obstacles,
            cameras,
            lights: self.lights,
            ambient,
            doors,
            interacts: self.interacts,
            windows: self.windows,
            flames: self.flames,
            corpses,
            fog: Fog { start: 9.0, end: 30.0, color: [0.0, 0.0, 0.0] },
        }
    }
}

fn cam(min: (f32, f32), max: (f32, f32), eye: Vec3, target: Vec3, fov: f32) -> CamZone {
    CamZone { min, max, eye, target, fov }
}

fn door(pos: Vec3, normal: Vec3, to_room: usize, to_pos: Vec3, to_yaw: f32, lock: Option<(usize, u8)>) -> Door {
    Door { pos, normal, to_room, to_pos, to_yaw, lock }
}

const BIG: f32 = 1000.0;

// ------------------------------------------------------------------ pièces

fn hall() -> Room {
    let mut b = Builder::new((-6.0, 0.0), (6.0, 14.0), 6.5);
    let style = Style {
        lower: Mat::tex(T_WAINSCOT, [0.9, 0.8, 0.75]),
        upper: Mat::tex(T_PAPER_RED, [1.0, 1.0, 1.0]),
        floor: Mat::tex(T_MARBLE, [0.9, 0.9, 0.9]),
        ceil: Mat::tex(T_CEILING, [0.8, 0.8, 0.8]),
        wainscot_h: 1.3,
        floor_tpm: 0.5,
    };
    b.shell(
        &style,
        &[
            op(Side::S, 4.8, 7.2, 0.0, 3.2, OpenKind::DoubleDoor),
            op(Side::S, 1.2, 3.4, 1.0, 5.6, OpenKind::Window),
            op(Side::S, 8.6, 10.8, 1.0, 5.6, OpenKind::Window),
            op(Side::W, 5.5, 6.9, 0.0, 2.6, OpenKind::Door),
            op(Side::E, 5.5, 6.9, 0.0, 2.6, OpenKind::Door),
            op(Side::N, 5.2, 6.8, 0.0, 2.8, OpenKind::Door),
        ],
    );
    // Tapis central.
    let carpet = Mat::tex(T_CARPET, [1.0, 1.0, 1.0]).decal();
    b.mesh.grid(v3(-1.0, 0.01, 0.3), v3(2.0, 0.0, 0.0), v3(0.0, 0.0, 13.4), [0.0, 0.0], [1.0, 6.7], 1.0, &carpet, Vec3::UP);
    // Piliers.
    let stone = Mat::tex(T_STONE, [0.75, 0.72, 0.7]);
    // Pilastres engagés dans les murs (rien ne peut se cacher derrière).
    for (x0, x1) in [(-6.0f32, -5.4f32), (5.4, 6.0)] {
        b.mesh.cube_faces(v3(x0, 0.0, 9.15), v3(x1, 6.5, 9.85), &stone, 1.0, [x0 > 0.0, x0 < 0.0, false, false, true, true]);
        b.mesh.cube_faces(v3(x0, 0.0, 9.0), v3(x1, 0.4, 10.0), &stone, 1.0, [x0 > 0.0, x0 < 0.0, false, true, true, true]);
        b.obstacle(x0, 9.0, x1, 10.0, true);
    }
    // Machine à écrire (point de sauvegarde).
    b.table(-5.9, 9.8, -5.0, 11.2, 0.8, Mat::tex(T_WOOD, [0.6, 0.45, 0.35]), false);
    let metal = Mat::tex(T_METAL, [0.35, 0.33, 0.3]);
    b.mesh.cube(v3(-5.7, 0.8, 10.25), v3(-5.25, 0.95, 10.75), &metal, 1.0);
    b.mesh.cube(v3(-5.72, 0.95, 10.3), v3(-5.55, 1.05, 10.7), &metal, 1.0);
    let paper = Mat::tex(T_NOTE, [1.0, 1.0, 1.0]).two();
    b.mesh.quad([v3(-5.65, 1.0, 10.35), v3(-5.65, 1.0, 10.65), v3(-5.75, 1.22, 10.65), v3(-5.75, 1.22, 10.35)], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]], &paper, v3(1.0, 0.3, 0.0));
    // Lampe.
    let shade = Mat::col([0.9, 0.7, 0.35]).glow(0.9);
    b.mesh.cube(v3(-5.55, 0.8, 10.85), v3(-5.5, 1.15, 10.9), &metal, 1.0);
    b.mesh.cube(v3(-5.68, 1.15, 10.72), v3(-5.37, 1.35, 11.03), &shade, 1.0);
    b.light(v3(-5.3, 1.3, 10.6), [0.8, 0.6, 0.32], 4.0, 0.0);
    b.interacts.push(Interact { pos: v3(-5.4, 0.0, 10.5), radius: 1.0, kind: InteractKind::Typewriter });
    // Statue sur piédestal.
    b.mesh.cube_faces(v3(4.1, 0.0, 12.1), v3(5.1, 1.0, 13.1), &stone, 1.0, [true, true, false, true, true, true]);
    let marble = Mat::tex(T_STONE, [0.95, 0.95, 0.9]);
    b.mesh.cube(v3(4.4, 1.0, 12.45), v3(4.8, 1.9, 12.75), &marble, 2.0);
    b.mesh.cube(v3(4.35, 1.9, 12.4), v3(4.85, 2.4, 12.8), &marble, 2.0);
    b.mesh.cube(v3(4.48, 2.4, 12.48), v3(4.72, 2.68, 12.72), &marble, 2.0);
    b.mesh.cube(v3(4.2, 2.1, 12.55), v3(4.35, 2.6, 12.65), &marble, 2.0);
    b.obstacle(4.1, 12.1, 6.0, 14.0, true);
    b.msg(4.6, 11.6, 1.0, "Une statue de femme tenant une épée. Son regard de pierre semble vous suivre.");
    // Horloge de parquet.
    let wood = Mat::tex(T_WOOD, [0.5, 0.33, 0.25]);
    b.mesh.cube_faces(v3(-6.0, 0.0, 2.0), v3(-5.5, 2.4, 2.7), &wood, 1.0, [false, true, false, true, true, true]);
    let face = Mat::col([0.85, 0.8, 0.6]);
    b.mesh.quad([v3(-5.49, 1.8, 2.15), v3(-5.49, 1.8, 2.55), v3(-5.49, 2.2, 2.55), v3(-5.49, 2.2, 2.15)], [[0.0; 2]; 4], &face, v3(1.0, 0.0, 0.0));
    b.obstacle(-6.0, 2.0, -5.5, 2.7, true);
    b.msg(-5.2, 2.35, 0.9, "Une horloge de parquet. Les aiguilles se sont arrêtées sur minuit.");
    // Tableaux.
    b.painting(v3(-3.4, 2.8, 14.0), v3(0.0, 0.0, -1.0), 1.2, 1.8, T_PAINTING);
    b.painting(v3(3.4, 2.8, 14.0), v3(0.0, 0.0, -1.0), 1.2, 1.8, T_PAINTING);
    b.msg(-3.4, 13.4, 0.9, "Le portrait d'un homme au teint blafard. Ses yeux ont été grattés.");
    b.msg(3.4, 13.4, 0.9, "Le portrait d'une femme en deuil. Une plaque indique : « Lady Ashcroft ».");
    // Lustre.
    let iron = Mat::col([0.2, 0.18, 0.15]);
    b.mesh.cube(v3(-0.03, 4.9, 6.97), v3(0.03, 6.5, 7.03), &iron, 1.0);
    for i in 0..8 {
        let a = i as f32 / 8.0 * std::f32::consts::TAU;
        let p = v3(a.sin() * 0.9, 4.9, 7.0 + a.cos() * 0.9);
        b.mesh.cube(v3(-0.05, -0.03, -0.05) + p, v3(0.05, 0.03, 0.05) + p, &iron, 1.0);
        let wax = Mat::col([0.9, 0.85, 0.7]).glow(0.7);
        b.mesh.cube(p + v3(-0.02, 0.03, -0.02), p + v3(0.02, 0.15, 0.02), &wax, 1.0);
        b.flames.push((p + v3(0.0, 0.2, 0.0), 0.05));
    }
    b.light(v3(0.0, 4.4, 7.0), [0.95, 0.68, 0.4], 11.0, 0.08);
    // Appliques près des portes.
    b.sconce(v3(-5.95, 2.2, 4.6), v3(1.0, 0.0, 0.0), 4.0);
    b.sconce(v3(5.95, 2.2, 4.6), v3(-1.0, 0.0, 0.0), 4.0);
    b.sconce(v3(-1.6, 2.2, 13.95), v3(0.0, 0.0, -1.0), 4.0);
    b.sconce(v3(1.6, 2.2, 13.95), v3(0.0, 0.0, -1.0), 4.0);
    // Clair de lune.
    b.moon_spill(v3(-3.7, 2.6, 0.8), 7.0);
    b.moon_spill(v3(3.7, 2.6, 0.8), 7.0);
    b.msg(0.0, 0.5, 1.2, "La porte d'entrée est bloquée de l'extérieur. Impossible de sortir.");

    let cameras = vec![
        // Entrée : plan sur la porte d'entrée et la lune par la fenêtre.
        cam((-BIG, -BIG), (BIG, 5.0), v3(-0.8, 2.7, 11.6), v3(-0.4, 1.6, 0.0), 66.0),
        // Reste du hall : vue d'ensemble depuis le haut de l'entrée.
        cam((-BIG, 5.0), (BIG, BIG), v3(0.0, 5.6, 0.3), v3(0.0, 0.2, 9.5), 76.0),
    ];
    let doors = vec![
        door(v3(-6.0, 0.0, 6.2), v3(1.0, 0.0, 0.0), ROOM_DINING, v3(-6.9, 0.0, 6.2), -FRAC_PI_2, None),
        door(v3(6.0, 0.0, 6.2), v3(-1.0, 0.0, 0.0), ROOM_CORRIDOR, v3(6.8, 0.0, 6.2), FRAC_PI_2, None),
        door(v3(0.0, 0.0, 14.0), v3(0.0, 0.0, -1.0), ROOM_LIBRARY, v3(0.0, 0.0, 14.9), 0.0, Some((0, KEY_EMBLEM))),
    ];
    b.build("HALL", cameras, doors, [0.05, 0.05, 0.07], vec![])
}

fn dining() -> Room {
    let mut b = Builder::new((-18.0, 2.0), (-6.0, 12.0), 4.2);
    let style = Style {
        lower: Mat::tex(T_WAINSCOT, [0.85, 0.75, 0.7]),
        upper: Mat::tex(T_PAPER_GREEN, [1.0, 1.0, 1.0]),
        floor: Mat::tex(T_WOOD, [0.8, 0.8, 0.8]),
        ceil: Mat::tex(T_CEILING, [0.8, 0.8, 0.8]),
        wainscot_h: 1.1,
        floor_tpm: 0.5,
    };
    b.shell(
        &style,
        &[
            op(Side::E, 3.5, 4.9, 0.0, 2.6, OpenKind::Door),
            op(Side::W, 1.5, 3.5, 1.0, 3.3, OpenKind::Window),
            op(Side::W, 6.5, 8.5, 1.0, 3.3, OpenKind::Window),
            op(Side::S, 2.5, 4.5, 1.0, 3.3, OpenKind::Window),
            op(Side::S, 7.5, 9.5, 1.0, 3.3, OpenKind::Window),
        ],
    );
    // Cheminée.
    let stone = Mat::tex(T_STONE, [0.7, 0.66, 0.62]);
    b.mesh.cube_faces(v3(-13.4, 0.0, 11.3), v3(-12.8, 1.4, 12.0), &stone, 1.0, [true, true, false, false, true, false]);
    b.mesh.cube_faces(v3(-11.2, 0.0, 11.3), v3(-10.6, 1.4, 12.0), &stone, 1.0, [true, true, false, false, true, false]);
    b.mesh.cube_faces(v3(-13.6, 1.4, 11.2), v3(-10.4, 1.65, 12.0), &stone, 1.0, [true, true, true, true, true, false]);
    b.mesh.cube_faces(v3(-12.8, 1.2, 11.3), v3(-11.2, 1.4, 12.0), &stone, 1.0, [false, false, true, false, true, false]);
    let soot = Mat::col([0.05, 0.04, 0.04]).decal();
    b.mesh.quad([v3(-12.8, 0.0, 11.95), v3(-11.2, 0.0, 11.95), v3(-11.2, 1.2, 11.95), v3(-12.8, 1.2, 11.95)], [[0.0; 2]; 4], &soot, v3(0.0, 0.0, -1.0));
    b.mesh.quad([v3(-12.8, 0.01, 11.3), v3(-11.2, 0.01, 11.3), v3(-11.2, 0.01, 12.0), v3(-12.8, 0.01, 12.0)], [[0.0; 2]; 4], &soot, Vec3::UP);
    let log = Mat::tex(T_BARK, [0.8, 0.6, 0.5]);
    b.mesh.cube(v3(-12.5, 0.0, 11.55), v3(-11.5, 0.12, 11.7), &log, 2.0);
    b.mesh.cube(v3(-12.3, 0.1, 11.62), v3(-11.6, 0.22, 11.78), &log, 2.0);
    b.flames.push((v3(-12.25, 0.38, 11.65), 0.28));
    b.flames.push((v3(-11.75, 0.34, 11.68), 0.24));
    b.flames.push((v3(-12.0, 0.45, 11.62), 0.32));
    b.light(v3(-12.0, 0.7, 11.0), [1.1, 0.5, 0.16], 8.5, 0.35);
    b.obstacle(-13.6, 11.2, -10.4, 12.0, false);
    b.painting(v3(-12.0, 2.5, 12.0), v3(0.0, 0.0, -1.0), 1.5, 1.0, T_LANDSCAPE);
    b.msg(-12.0, 10.7, 1.0, "Le feu crépite encore. Quelqu'un l'a allumé il n'y a pas longtemps...");
    // Grande table.
    b.table(-15.0, 6.3, -9.0, 7.7, 0.8, Mat::tex(T_WOOD, [0.6, 0.45, 0.35]), true);
    for x in [-13.0f32, -11.0] {
        let brass = Mat::col([0.6, 0.47, 0.22]);
        b.mesh.cube(v3(x - 0.04, 0.8, 6.96), v3(x + 0.04, 1.15, 7.04), &brass, 1.0);
        b.mesh.cube(v3(x - 0.2, 1.12, 6.98), v3(x + 0.2, 1.15, 7.02), &brass, 1.0);
        for dx in [-0.18f32, 0.0, 0.18] {
            let p = v3(x + dx, 1.3, 7.0);
            let wax = Mat::col([0.9, 0.85, 0.7]).glow(0.6);
            b.mesh.cube(p - v3(0.02, 0.15, 0.02), p + v3(0.02, 0.0, 0.02), &wax, 1.0);
            b.flames.push((p + v3(0.0, 0.05, 0.0), 0.045));
        }
        b.light(v3(x, 1.5, 7.0), [0.6, 0.38, 0.18], 4.5, 0.2);
    }
    // Assiettes.
    let plate = Mat::col([0.8, 0.8, 0.75]);
    for x in [-14.0f32, -12.5, -11.0, -9.6] {
        for z in [6.55f32, 7.45] {
            b.mesh.cube(v3(x - 0.13, 0.805, z - 0.13), v3(x + 0.13, 0.82, z + 0.13), &plate, 1.0);
        }
    }
    for x in [-14.0f32, -12.5, -11.0, -9.6] {
        b.chair(x, 5.85, 0.0);
        b.chair(x, 8.15, PI);
    }
    // Buffet avec munitions.
    b.mesh.cube_faces(v3(-8.6, 0.0, 11.35), v3(-6.4, 0.9, 12.0), &Mat::tex(T_WAINSCOT, [0.8, 0.7, 0.65]), 1.0, [true, true, false, true, true, false]);
    b.obstacle(-8.6, 11.35, -6.4, 12.0, false);
    // Lumière lunaire.
    for z in [4.5f32, 9.5] {
        b.moon_spill(v3(-17.5, 2.2, z), 4.5);
    }
    for x in [-14.5f32, -9.5] {
        b.moon_spill(v3(x, 2.2, 2.5), 4.5);
    }
    b.msg(-16.3, 3.2, 0.9, "Un cadavre... Le corps a été à moitié dévoré.");

    let cameras = vec![
        // Chaque quadrant est filmé depuis le coin opposé.
        cam((-11.0, -BIG), (BIG, 7.0), v3(-15.0, 3.2, 10.8), v3(-8.0, 0.8, 4.5), 64.0),
        cam((-11.0, 7.0), (BIG, BIG), v3(-14.5, 3.2, 3.0), v3(-7.0, 0.8, 10.5), 64.0),
        cam((-BIG, -BIG), (-11.0, 7.0), v3(-7.0, 3.2, 10.8), v3(-16.0, 0.6, 3.5), 64.0),
        cam((-BIG, 7.0), (-11.0, BIG), v3(-7.0, 3.2, 3.0), v3(-15.5, 0.8, 10.5), 64.0),
    ];
    let doors = vec![door(v3(-6.0, 0.0, 6.2), v3(-1.0, 0.0, 0.0), ROOM_HALL, v3(-5.1, 0.0, 6.2), FRAC_PI_2, None)];
    let corpses = vec![(v3(-16.3, 0.0, 3.0), 1.2)];
    let mut r = b.build("SALLE À MANGER", cameras, doors, [0.04, 0.04, 0.06], corpses);
    r.fog = Fog { start: 8.0, end: 24.0, color: [0.0; 3] };
    r
}

fn corridor() -> Room {
    let mut b = Builder::new((6.0, -1.0), (9.0, 20.0), 3.4);
    let style = Style {
        lower: Mat::tex(T_WAINSCOT, [0.8, 0.7, 0.65]),
        upper: Mat::tex(T_PAPER_BLUE, [1.0, 1.0, 1.0]),
        floor: Mat::tex(T_WOOD, [0.75, 0.72, 0.7]),
        ceil: Mat::tex(T_CEILING, [0.75, 0.75, 0.75]),
        wainscot_h: 1.1,
        floor_tpm: 0.5,
    };
    b.shell(
        &style,
        &[
            op(Side::W, 6.5, 7.9, 0.0, 2.6, OpenKind::Door),
            op(Side::E, 2.5, 4.1, 1.0, 2.8, OpenKind::Window),
            op(Side::E, 9.5, 11.1, 1.0, 2.8, OpenKind::Window),
            op(Side::E, 16.0, 17.6, 1.0, 2.8, OpenKind::Window),
            op(Side::N, 0.8, 2.2, 0.0, 2.6, OpenKind::Door),
        ],
    );
    let carpet = Mat::tex(T_CARPET, [0.8, 0.8, 0.8]).decal();
    b.mesh.grid(v3(6.7, 0.01, -0.6), v3(1.6, 0.0, 0.0), v3(0.0, 0.0, 20.2), [0.0, 0.0], [0.8, 10.1], 1.0, &carpet, Vec3::UP);
    // Commode au fond.
    b.mesh.cube_faces(v3(6.6, 0.0, -1.0), v3(8.4, 1.0, -0.5), &Mat::tex(T_WAINSCOT, [0.8, 0.7, 0.65]), 1.0, [true, true, false, true, false, true]);
    b.obstacle(6.6, -1.0, 8.4, -0.5, false);
    // Guéridon et vase.
    b.table(6.05, 13.3, 6.6, 14.3, 0.85, Mat::tex(T_WOOD, [0.6, 0.45, 0.35]), false);
    let vase = Mat::col([0.25, 0.3, 0.5]);
    b.mesh.cube(v3(6.2, 0.85, 13.68), v3(6.45, 1.2, 13.92), &vase, 1.0);
    let dead = Mat::col([0.25, 0.18, 0.1]).two();
    b.mesh.tri([v3(6.32, 1.2, 13.8), v3(6.2, 1.6, 13.7), v3(6.4, 1.55, 13.95)], [[0.0; 2]; 3], &dead, v3(1.0, 0.0, 0.0));
    b.painting(v3(6.0, 1.9, 9.8), v3(1.0, 0.0, 0.0), 0.9, 1.3, T_PAINTING);
    b.msg(6.6, 9.8, 0.8, "Un autre portrait. La peinture a coulé comme des larmes.");
    // Appliques.
    for z in [2.0f32, 10.5, 17.5] {
        b.sconce(v3(6.05, 2.1, z), v3(1.0, 0.0, 0.0), 4.2);
    }
    for z in [2.3f32, 9.3, 15.8] {
        b.moon_spill(v3(8.6, 2.0, z), 3.6);
    }
    let cameras = vec![
        // Extrémité sud (commode), regardée depuis le nord.
        cam((-BIG, -BIG), (BIG, 4.0), v3(8.6, 2.9, 8.5), v3(7.0, 0.6, -1.0), 62.0),
        cam((-BIG, 4.0), (BIG, 12.0), v3(8.6, 2.9, -0.7), v3(7.2, 0.9, 9.0), 62.0),
        // Extrémité nord (porte du bureau), regardée depuis le sud.
        cam((-BIG, 12.0), (BIG, BIG), v3(8.6, 2.9, 7.0), v3(7.3, 1.0, 20.0), 62.0),
    ];
    let doors = vec![
        door(v3(6.0, 0.0, 6.2), v3(1.0, 0.0, 0.0), ROOM_HALL, v3(5.1, 0.0, 6.2), -FRAC_PI_2, None),
        door(v3(7.5, 0.0, 20.0), v3(0.0, 0.0, -1.0), ROOM_STUDY, v3(7.5, 0.0, 20.9), 0.0, None),
    ];
    b.build("COULOIR EST", cameras, doors, [0.04, 0.04, 0.06], vec![])
}

fn study() -> Room {
    let mut b = Builder::new((6.0, 20.0), (14.0, 27.0), 3.5);
    let style = Style {
        lower: Mat::tex(T_PANEL, [0.9, 0.85, 0.8]),
        upper: Mat::tex(T_PAPER_GREEN, [0.9, 0.9, 0.9]),
        floor: Mat::tex(T_WOOD, [0.7, 0.65, 0.6]),
        ceil: Mat::tex(T_CEILING, [0.7, 0.7, 0.7]),
        wainscot_h: 1.2,
        floor_tpm: 0.5,
    };
    b.shell(
        &style,
        &[
            op(Side::S, 0.8, 2.2, 0.0, 2.6, OpenKind::Door),
            op(Side::E, 2.0, 4.0, 1.0, 2.8, OpenKind::Window),
            op(Side::N, 3.5, 5.5, 1.0, 2.8, OpenKind::Window),
        ],
    );
    let carpet = Mat::tex(T_CARPET, [0.7, 0.9, 0.7]).decal();
    b.mesh.grid(v3(8.2, 0.01, 21.4), v3(4.2, 0.0, 0.0), v3(0.0, 0.0, 2.8), [0.0, 0.0], [2.1, 1.4], 1.0, &carpet, Vec3::UP);
    // Bureau.
    b.table(10.0, 24.5, 12.2, 25.5, 0.8, Mat::tex(T_WOOD, [0.55, 0.38, 0.28]), false);
    let wood = Mat::tex(T_WOOD, [0.5, 0.35, 0.27]);
    b.mesh.cube(v3(11.5, 0.0, 24.6), v3(12.15, 0.74, 25.45), &wood, 1.0);
    b.chair(11.1, 23.9, 0.0);
    let metal = Mat::tex(T_METAL, [0.3, 0.3, 0.25]);
    b.mesh.cube(v3(11.78, 0.8, 25.15), v3(11.82, 1.15, 25.19), &metal, 1.0);
    let shade = Mat::col([0.25, 0.75, 0.35]).glow(0.8);
    b.mesh.cube(v3(11.62, 1.12, 25.0), v3(11.98, 1.25, 25.35), &shade, 1.0);
    b.light(v3(11.6, 1.1, 24.9), [0.75, 0.68, 0.4], 5.0, 0.0);
    let paper = Mat::tex(T_NOTE, [1.0, 1.0, 1.0]);
    b.mesh.cube(v3(10.9, 0.8, 24.75), v3(11.3, 0.81, 25.1), &paper, 1.0);
    // Bibliothèque murale.
    b.bookshelf(6.0, 21.0, 6.45, 26.6, 2.6, [false, true, false, false]);
    // Armoire à pharmacie.
    b.mesh.cube_faces(v3(13.35, 0.0, 20.25), v3(14.0, 1.0, 21.55), &Mat::tex(T_WAINSCOT, [0.75, 0.68, 0.65]), 1.0, [true, false, false, true, true, true]);
    b.obstacle(13.35, 20.25, 14.0, 21.55, false);
    // Globe / fauteuil.
    let leather = Mat::col([0.3, 0.1, 0.08]);
    b.mesh.cube(v3(8.3, 0.0, 25.4), v3(9.2, 0.45, 26.3), &leather, 1.0);
    b.mesh.cube(v3(8.3, 0.45, 26.1), v3(9.2, 1.1, 26.35), &leather, 1.0);
    b.obstacle(8.3, 25.4, 9.2, 26.35, false);
    b.painting(v3(11.5, 1.9, 20.0), v3(0.0, 0.0, 1.0), 1.4, 0.95, T_LANDSCAPE);
    b.msg(11.5, 20.6, 0.8, "Un paysage du marais au clair de lune. Le manoir y est peint... sans fenêtres.");
    b.msg(6.9, 23.5, 0.9, "Des traités d'anatomie, de botanique... et de nécromancie.");
    b.moon_spill(v3(13.5, 2.0, 23.0), 4.0);
    b.moon_spill(v3(10.5, 2.0, 26.5), 4.0);
    b.sconce(v3(9.4, 2.0, 20.05), v3(0.0, 0.0, 1.0), 3.5);
    let cameras = vec![
        cam((-BIG, -BIG), (10.0, 23.5), v3(13.6, 3.0, 26.6), v3(7.0, 0.6, 20.8), 64.0),
        cam((10.0, -BIG), (BIG, 23.5), v3(6.6, 3.0, 26.6), v3(13.5, 0.6, 20.5), 64.0),
        cam((10.0, 23.5), (BIG, BIG), v3(6.6, 3.0, 20.4), v3(12.5, 0.6, 25.5), 64.0),
        cam((-BIG, 23.5), (10.0, BIG), v3(13.6, 3.0, 20.4), v3(7.0, 0.6, 25.5), 64.0),
    ];
    let doors = vec![door(v3(7.5, 0.0, 20.0), v3(0.0, 0.0, 1.0), ROOM_CORRIDOR, v3(7.5, 0.0, 19.1), PI, None)];
    b.build("BUREAU", cameras, doors, [0.045, 0.045, 0.06], vec![])
}

fn library() -> Room {
    let mut b = Builder::new((-6.0, 14.0), (6.0, 26.0), 5.0);
    let style = Style {
        lower: Mat::tex(T_PANEL, [0.9, 0.85, 0.8]),
        upper: Mat::tex(T_PANEL, [0.75, 0.7, 0.65]),
        floor: Mat::tex(T_WOOD, [0.6, 0.55, 0.5]),
        ceil: Mat::tex(T_CEILING, [0.6, 0.6, 0.6]),
        wainscot_h: 1.2,
        floor_tpm: 0.5,
    };
    b.shell(
        &style,
        &[
            op(Side::S, 5.2, 6.8, 0.0, 2.8, OpenKind::Door),
            op(Side::N, 1.8, 4.0, 1.2, 4.2, OpenKind::Window),
            op(Side::N, 8.0, 10.2, 1.2, 4.2, OpenKind::Window),
            op(Side::W, 5.0, 7.0, 1.2, 3.6, OpenKind::Window),
        ],
    );
    b.bookshelf(-6.0, 14.6, -5.45, 18.5, 3.2, [false, true, false, false]);
    b.bookshelf(-6.0, 21.5, -5.45, 25.4, 3.2, [false, true, false, false]);
    // Pupitre de lecture.
    b.table(-3.4, 17.0, -2.2, 17.8, 1.0, Mat::tex(T_WOOD, [0.5, 0.36, 0.28]), false);
    b.mesh.cube(v3(-3.2, 1.0, 17.2), v3(-2.5, 1.06, 17.65), &Mat::tex(T_BOOKS, [0.9, 0.9, 0.9]), 1.0);
    // Lustre de fer.
    let iron = Mat::col([0.2, 0.18, 0.15]);
    b.mesh.cube(v3(-0.03, 3.6, 19.77), v3(0.03, 5.0, 19.83), &iron, 1.0);
    for i in 0..6 {
        let a = i as f32 / 6.0 * std::f32::consts::TAU;
        let p = v3(a.sin() * 0.6, 3.6, 19.8 + a.cos() * 0.6);
        b.mesh.cube(p - v3(0.04, 0.03, 0.04), p + v3(0.04, 0.03, 0.04), &iron, 1.0);
        b.flames.push((p + v3(0.0, 0.08, 0.0), 0.045));
    }
    b.light(v3(0.0, 3.3, 19.8), [0.85, 0.6, 0.35], 9.0, 0.08);
    b.bookshelf(5.45, 14.6, 6.0, 25.4, 3.2, [true, false, false, false]);
    // Table de lecture.
    b.table(-1.0, 23.4, 1.0, 24.4, 0.8, Mat::tex(T_WOOD, [0.55, 0.4, 0.3]), false);
    let metal = Mat::tex(T_METAL, [0.3, 0.3, 0.25]);
    b.mesh.cube(v3(0.68, 0.8, 24.08), v3(0.72, 1.15, 24.12), &metal, 1.0);
    let shade = Mat::col([0.25, 0.75, 0.35]).glow(0.8);
    b.mesh.cube(v3(0.52, 1.12, 23.92), v3(0.88, 1.25, 24.28), &shade, 1.0);
    b.light(v3(0.5, 1.2, 23.6), [0.7, 0.62, 0.38], 5.5, 0.0);
    let note = Mat::tex(T_NOTE, [1.0, 1.0, 1.0]);
    b.mesh.cube(v3(0.0, 0.8, 23.7), v3(0.32, 0.81, 24.05), &note, 1.0);
    b.interacts.push(Interact { pos: v3(0.15, 0.0, 23.0), radius: 0.9, kind: InteractKind::Note });
    for x in [-1.6f32, 1.6] {
        b.sconce(v3(x, 2.2, 14.05), v3(0.0, 0.0, 1.0), 4.0);
    }
    b.sconce(v3(-5.95, 2.2, 16.5), v3(1.0, 0.0, 0.0), 4.0);
    b.moon_spill(v3(-3.1, 2.6, 25.5), 5.5);
    b.moon_spill(v3(3.1, 2.6, 25.5), 5.5);
    b.moon_spill(v3(-5.5, 2.4, 20.0), 4.5);
    let cameras = vec![
        cam((-BIG, -BIG), (0.0, 20.0), v3(5.2, 3.4, 25.4), v3(-3.5, 0.6, 15.5), 64.0),
        cam((0.0, -BIG), (BIG, 20.0), v3(-5.2, 3.4, 25.4), v3(3.5, 0.6, 15.5), 64.0),
        cam((-BIG, 20.0), (0.0, BIG), v3(5.2, 3.4, 14.6), v3(-3.5, 0.6, 24.5), 64.0),
        cam((0.0, 20.0), (BIG, BIG), v3(-5.2, 3.4, 14.6), v3(3.5, 0.6, 24.5), 64.0),
    ];
    let doors = vec![door(v3(0.0, 0.0, 14.0), v3(0.0, 0.0, 1.0), ROOM_HALL, v3(0.0, 0.0, 13.1), PI, None)];
    b.build("BIBLIOTHÈQUE", cameras, doors, [0.05, 0.05, 0.065], vec![])
}

pub fn build() -> World {
    let rooms = vec![hall(), dining(), corridor(), study(), library()];
    let items = vec![
        ItemSpawn { room: ROOM_DINING, pos: v3(-7.5, 0.9, 11.65), kind: ItemKind::Ammo(15) },
        ItemSpawn { room: ROOM_CORRIDOR, pos: v3(7.5, 1.0, -0.75), kind: ItemKind::Ammo(10) },
        ItemSpawn { room: ROOM_STUDY, pos: v3(10.6, 0.8, 25.0), kind: ItemKind::Key(KEY_EMBLEM) },
        ItemSpawn { room: ROOM_STUDY, pos: v3(13.65, 1.0, 20.9), kind: ItemKind::Medkit },
        ItemSpawn { room: ROOM_LIBRARY, pos: v3(-0.55, 0.8, 23.9), kind: ItemKind::Ammo(15) },
        ItemSpawn { room: ROOM_LIBRARY, pos: v3(4.6, 0.0, 21.6), kind: ItemKind::Medkit },
    ];
    let zombies = vec![
        ZombieSpawn { room: ROOM_DINING, pos: v3(-15.9, 0.0, 3.8), yaw: PI + 0.5, eating: true },
        ZombieSpawn { room: ROOM_CORRIDOR, pos: v3(7.6, 0.0, 15.5), yaw: PI, eating: false },
        ZombieSpawn { room: ROOM_LIBRARY, pos: v3(-3.0, 0.0, 20.4), yaw: FRAC_PI_2, eating: false },
        ZombieSpawn { room: ROOM_LIBRARY, pos: v3(2.8, 0.0, 24.6), yaw: PI, eating: false },
    ];
    World { rooms, items, zombies }
}

impl Room {
    pub fn camera_for(&self, p: Vec3) -> Option<usize> {
        self.cameras
            .iter()
            .position(|c| p.x >= c.min.0 && p.x < c.max.0 && p.z >= c.min.1 && p.z < c.max.1)
    }

    /// Résout la collision d'un cercle avec les murs et les meubles.
    pub fn collide(&self, mut p: Vec3, r: f32) -> Vec3 {
        p.x = p.x.clamp(self.min.0 + r, self.max.0 - r);
        p.z = p.z.clamp(self.min.1 + r, self.max.1 - r);
        for o in &self.obstacles {
            let cx = p.x.clamp(o.min.0, o.max.0);
            let cz = p.z.clamp(o.min.1, o.max.1);
            let dx = p.x - cx;
            let dz = p.z - cz;
            let d2 = dx * dx + dz * dz;
            if d2 < r * r {
                if d2 > 1e-8 {
                    let d = d2.sqrt();
                    p.x = cx + dx / d * r;
                    p.z = cz + dz / d * r;
                } else {
                    // Centre à l'intérieur : on sort par le côté le plus proche.
                    let left = p.x - o.min.0;
                    let right = o.max.0 - p.x;
                    let down = p.z - o.min.1;
                    let up = o.max.1 - p.z;
                    let m = left.min(right).min(down).min(up);
                    if m == left {
                        p.x = o.min.0 - r;
                    } else if m == right {
                        p.x = o.max.0 + r;
                    } else if m == down {
                        p.z = o.min.1 - r;
                    } else {
                        p.z = o.max.1 + r;
                    }
                }
            }
        }
        p.x = p.x.clamp(self.min.0 + r, self.max.0 - r);
        p.z = p.z.clamp(self.min.1 + r, self.max.1 - r);
        p
    }

    /// Distance d'un rayon horizontal jusqu'au premier obstacle (murs, meubles hauts).
    pub fn ray_dist(&self, o: Vec3, d: Vec3, max: f32) -> f32 {
        let mut t = max;
        let slab = |o: f32, d: f32, lo: f32, hi: f32| -> Option<(f32, f32)> {
            if d.abs() < 1e-8 {
                if o < lo || o > hi {
                    None
                } else {
                    Some((f32::MIN, f32::MAX))
                }
            } else {
                let a = (lo - o) / d;
                let b = (hi - o) / d;
                Some((a.min(b), a.max(b)))
            }
        };
        // Murs : sortie de la boîte de la pièce.
        if let (Some((_, tx)), Some((_, tz))) = (slab(o.x, d.x, self.min.0, self.max.0), slab(o.z, d.z, self.min.1, self.max.1)) {
            t = t.min(tx).min(tz);
        }
        for ob in self.obstacles.iter().filter(|o| o.tall) {
            if let (Some((ax, bx)), Some((az, bz))) = (slab(o.x, d.x, ob.min.0, ob.max.0), slab(o.z, d.z, ob.min.1, ob.max.1)) {
                let enter = ax.max(az);
                let exit = bx.min(bz);
                if enter <= exit && enter > 0.0 {
                    t = t.min(enter);
                }
            }
        }
        t
    }
}


