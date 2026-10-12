//! Logique de jeu : joueuse, objets, portes, inventaire, sauvegarde.
//! Les ennemis, le combat, l'histoire et le rendu ont leurs propres modules.

use crate::actors::{self, Body, BossBody, DogBody, Pose};
use crate::audio::{SharedMixer, Sfx};
use crate::input::Input;
use crate::math::*;
use crate::mesh::*;
use crate::options::{Options, Request, Storage};
use crate::outdoor::{self, Outdoor};
use crate::raster::{Camera, Raster, H, W};
use crate::textures::{self, *};
use crate::world::{self, *};
use std::collections::VecDeque;
use std::f32::consts::{PI, TAU};

mod combat;
mod enemies;
mod menu;
mod render;
mod save;
mod story;

use combat::Weapon;
use enemies::{Enemy, ZState};
use story::Cine;

const MAG_SIZE: u32 = 15;
const SHELL_MAG: u32 = 6;
const PLAYER_R: f32 = 0.28;
const LIGHT_GAIN: f32 = 1.6;
const AMBIENT_GAIN: f32 = 2.4;
/// Durée d'une chute au sol (coup puissant).
const KNOCK_TIME: f32 = 1.7;

// ---------------------------------------------------------------- état sauvegardable

#[derive(Clone, Debug)]
pub struct Player {
    pub pos: Vec3,
    pub yaw: f32,
    pub hp: i32,
    phase: f32,
    speed: f32,
    run_amt: f32,
    move_amt: f32,
    aim_t: f32,
    aiming: bool,
    lock: Option<usize>,
    recoil: f32,
    reload_t: f32,
    fire_cd: f32,
    grabbed: f32,
    hurt_t: f32,
    dead_t: f32,
    /// Au sol après un coup puissant (temps restant).
    knock: f32,
    /// Invulnérabilité après un coup.
    invuln: f32,
    /// Coup de couteau en cours (0 → 1).
    swing: f32,
}

#[derive(Clone, Debug, Default)]
struct Inventory {
    weapon: Weapon,
    mag: u32,
    reserve: u32,
    has_shotgun: bool,
    shells_mag: u32,
    shells: u32,
    medkits: u32,
    herbs: u32,
    keys: Vec<u8>,
}

#[derive(Clone, Debug)]
struct State {
    room: usize,
    player: Player,
    enemies: Vec<Enemy>,
    taken: Vec<bool>,
    unlocked: Vec<bool>,
    flags: Vec<bool>,
    /// Portraits enfoncés dans le bon ordre.
    puzzle: u8,
    inv: Inventory,
    blood: Vec<(usize, Vec3, f32, f32)>,
    play_time: f32,
    saves: u32,
    kills: u32,
}

// ---------------------------------------------------------------- modes

#[derive(Clone, Debug, PartialEq)]
enum Mode {
    Title,
    Play,
    Message,
    SaveChoice,
    Inventory,
    Pause,
    Options { title: bool },
    Controls { title: bool, options: bool },
    Door { t: f32, door: usize, shut: bool },
    Cutscene { cine: Cine, t: f32 },
    GameOver { t: f32 },
    Ending { t: f32 },
}

struct Particle {
    pos: Vec3,
    vel: Vec3,
    life: f32,
    col: [f32; 3],
    size: i32,
}

struct Scheduled {
    at: f32,
    sfx: Sfx,
    vol: f32,
    pan: f32,
    pitch: f32,
}

pub struct Game {
    world: World,
    outdoor: Outdoor,
    hero: Body,
    /// Apparences humanoïdes : zombies 0-2, Edgar zombifié, Edgar vivant, Sujet 7.
    bodies: Vec<Body>,
    dog_body: DogBody,
    hound_body: DogBody,
    boss_body: BossBody,
    corpse_body: Body,
    items_mesh: Vec<Mesh>,
    door_scene: Mesh,
    door_leaf: Mesh,
    gate_scene: Mesh,
    gate_leaf: Mesh,
    stairs_scene: Mesh,
    pub raster: Raster,
    pub mixer: SharedMixer,
    st: State,
    checkpoint: State,
    mode: Mode,
    prev_mode: Mode,
    rng: Rng,
    time: f32,
    cam_idx: usize,
    cam: Camera,
    basis_yaw: f32,
    prev_dir: Option<f32>,
    cam_changed: bool,
    particles: Vec<Particle>,
    muzzle: f32,
    hurt_flash: f32,
    /// Secousse de caméra (pas du boss, impacts).
    shake: f32,
    msgs: VecDeque<String>,
    after_msg: Option<Mode>,
    sel: usize,
    dog_t: f32,
    frog_t: f32,
    scheduled: Vec<Scheduled>,
    eyes: Vec<(Vec3, f32)>,
    fade: f32,
    banner_t: f32,
    window_msg: usize,
    has_saved: bool,
    death_sound: bool,
    debug_hide_player: bool,
    pub opts: Options,
    storage: Box<dyn Storage>,
    requests: Vec<Request>,
    native: bool,
    fps: f32,
    fps_acc: (f32, u32),
    /// Dernier périphérique utilisé : manette (true) ou clavier/souris.
    pad: bool,
    /// Lignes de menu cliquables (x0, y0, x1, y1) de l'image précédente.
    menu_rows: Vec<(i32, i32, i32, i32)>,
    last_mouse: Option<(f32, f32)>,
    prev_run: bool,
    quick_turn: f32,
    save_exists: bool,
}

const START_POS: Vec3 = v3(0.0, 0.0, 2.4);

fn initial_state(world: &World) -> State {
    State {
        room: ROOM_HALL,
        player: Player {
            pos: START_POS,
            yaw: 0.0,
            hp: 100,
            phase: 0.0,
            speed: 0.0,
            run_amt: 0.0,
            move_amt: 0.0,
            aim_t: 0.0,
            aiming: false,
            lock: None,
            recoil: 0.0,
            reload_t: 0.0,
            fire_cd: 0.0,
            grabbed: 0.0,
            hurt_t: 0.0,
            dead_t: 0.0,
            knock: 0.0,
            invuln: 0.0,
            swing: 0.0,
        },
        enemies: world.enemies.iter().enumerate().map(|(i, s)| Enemy::spawn(i, s)).collect(),
        taken: vec![false; world.items.len()],
        unlocked: vec![false; NLOCKS],
        flags: vec![false; NFLAGS],
        puzzle: 0,
        inv: Inventory { weapon: Weapon::Pistol, mag: 12, ..Default::default() },
        blood: world
            .rooms
            .iter()
            .enumerate()
            .flat_map(|(ri, r)| r.corpses.iter().map(move |c| (ri, c.0 + v3(0.0, 0.0, 0.3), 0.9, 0.9)))
            .collect(),
        play_time: 0.0,
        saves: 0,
        kills: 0,
    }
}

fn item_meshes() -> Vec<Mesh> {
    let mut ammo = Mesh::new();
    ammo.block(v3(-0.12, 0.0, -0.08), v3(0.12, 0.1, 0.08), &Mat::tex(T_AMMO, [1.0, 1.0, 1.0]));
    let mut med = Mesh::new();
    med.block(v3(-0.14, 0.0, -0.1), v3(0.14, 0.12, 0.1), &Mat::tex(T_MEDKIT, [1.0, 1.0, 1.0]));
    med.block(v3(-0.04, 0.12, -0.02), v3(0.04, 0.15, 0.02), &Mat::col([0.3, 0.3, 0.3]));
    let mut key = Mesh::new();
    let gold = Mat::tex(T_KEY, [1.0, 1.0, 1.0]).glow(0.3);
    key.block(v3(-0.1, 0.0, -0.012), v3(0.06, 0.02, 0.012), &gold);
    key.block(v3(0.06, 0.0, -0.045), v3(0.13, 0.02, 0.045), &gold);
    key.block(v3(-0.1, 0.0, 0.012), v3(-0.07, 0.02, 0.04), &gold);
    key.block(v3(-0.05, 0.0, 0.012), v3(-0.03, 0.02, 0.035), &gold);
    let mut shells = Mesh::new();
    shells.block(v3(-0.1, 0.0, -0.07), v3(0.1, 0.09, 0.07), &Mat::tex(T_SHELLS, [1.0, 1.0, 1.0]));
    let mut herb = Mesh::new();
    herb.block(v3(-0.07, 0.0, -0.07), v3(0.07, 0.1, 0.07), &Mat::col([0.45, 0.22, 0.12]));
    let leaf = Mat::tex(T_HERB, [1.0, 1.0, 1.0]).two();
    for k in 0..4 {
        let a = k as f32 * std::f32::consts::FRAC_PI_2 + 0.4;
        let d = v3(a.sin(), 0.0, a.cos());
        herb.tri([v3(0.0, 0.1, 0.0), d * 0.16 + v3(0.0, 0.2, 0.0) + v3(d.z, 0.0, -d.x) * 0.05, d * 0.12 + v3(0.0, 0.26, 0.0)], [[0.5, 1.0], [0.0, 0.0], [1.0, 0.0]], &leaf, Vec3::UP);
    }
    let mut gun = Mesh::new();
    let metal = Mat::col([0.18, 0.18, 0.2]);
    let walnut = Mat::tex(T_WOOD, [0.55, 0.38, 0.28]);
    gun.block(v3(-0.5, 0.02, -0.02), v3(0.25, 0.06, 0.02), &metal);
    gun.block(v3(-0.3, 0.0, -0.025), v3(0.0, 0.04, 0.025), &walnut);
    gun.block(v3(0.15, -0.02, -0.03), v3(0.5, 0.05, 0.03), &walnut);
    vec![ammo, med, key, shells, herb, gun]
}

fn door_scene() -> (Mesh, Mesh) {
    let mut m = Mesh::new();
    let wall = Mat::tex(T_PAPER_RED, [0.8, 0.8, 0.8]);
    let low = Mat::tex(T_WAINSCOT, [0.8, 0.8, 0.8]);
    let n = v3(0.0, 0.0, -1.0);
    let up = Vec3::UP;
    for (x0, x1) in [(-4.0f32, -0.75f32), (0.75, 4.0)] {
        m.wall(v3(x0, 0.0, 0.0), v3(x1 - x0, 0.0, 0.0), up * 1.1, 1.0, x0, &low, n);
        m.wall(v3(x0, 1.1, 0.0), v3(x1 - x0, 0.0, 0.0), up * 3.0, 0.5, x0, &wall, n);
    }
    m.wall(v3(-0.75, 2.55, 0.0), v3(1.5, 0.0, 0.0), up * 1.6, 0.5, -0.75, &wall, n);
    let frame = Mat::tex(T_WOOD, [0.5, 0.35, 0.28]);
    m.cube(v3(-0.75, 0.0, -0.08), v3(-0.6, 2.55, 0.05), &frame, 1.0);
    m.cube(v3(0.6, 0.0, -0.08), v3(0.75, 2.55, 0.05), &frame, 1.0);
    m.cube(v3(-0.75, 2.4, -0.08), v3(0.75, 2.55, 0.05), &frame, 1.0);
    m.grid(v3(-4.0, 0.0, -5.0), v3(8.0, 0.0, 0.0), v3(0.0, 0.0, 5.0), [0.0, 0.0], [4.0, 2.5], 0.5, &Mat::tex(T_WOOD, [0.7, 0.7, 0.7]), up);
    let mut leaf = Mesh::new();
    let dm = Mat::tex(T_DOOR, [1.0, 1.0, 1.0]).two();
    // Battant subdivisé : limite la déformation des textures affines vue de près.
    leaf.grid(v3(0.0, 0.0, 0.0), v3(1.2, 0.0, 0.0), v3(0.0, 2.4, 0.0), [0.0, 1.0], [1.0, -1.0], 0.2, &dm, n);
    (m, leaf)
}

/// Portail du parc : piliers, grille, sol d'herbe ; battant de 0,75 m.
fn gate_scene() -> (Mesh, Mesh) {
    let mut m = Mesh::new();
    let up = Vec3::UP;
    let n = v3(0.0, 0.0, -1.0);
    let stone = Mat::tex(T_STONE, [0.5, 0.5, 0.55]);
    let iron = Mat::col([0.1, 0.1, 0.12]).two();
    m.grid(v3(-6.0, 0.0, -5.0), v3(12.0, 0.0, 0.0), v3(0.0, 0.0, 14.0), [0.0, 0.0], [6.0, 7.0], 1.0, &Mat::tex(T_GRASS, [0.8, 0.9, 0.8]), up);
    for x in [-1.15f32, 0.75] {
        m.cube(v3(x, 0.0, -0.2), v3(x + 0.4, 2.9, 0.2), &stone, 1.0);
        m.cube(v3(x - 0.05, 2.9, -0.25), v3(x + 0.45, 3.05, 0.25), &stone, 1.0);
    }
    // Grille fixe de part et d'autre.
    for side in [-1.0f32, 1.0] {
        for k in 0..14 {
            let x = side * (1.25 + k as f32 * 0.3);
            m.quad([v3(x, 0.0, 0.0), v3(x + 0.04, 0.0, 0.0), v3(x + 0.04, 2.0, 0.0), v3(x, 2.0, 0.0)], [[0.0; 2]; 4], &iron, n);
        }
        let (a, b) = if side < 0.0 { (-6.0f32, -1.15f32) } else { (1.15, 6.0) };
        for y in [0.3f32, 1.7] {
            m.quad([v3(a, y, 0.0), v3(b, y, 0.0), v3(b, y + 0.05, 0.0), v3(a, y + 0.05, 0.0)], [[0.0; 2]; 4], &iron, n);
        }
    }
    let mut gate = Mesh::new();
    for k in 0..6 {
        let x = 0.04 + k as f32 * 0.13;
        gate.quad([v3(x, 0.0, 0.0), v3(x + 0.035, 0.0, 0.0), v3(x + 0.035, 2.4, 0.0), v3(x, 2.4, 0.0)], [[0.0; 2]; 4], &iron, n);
        gate.tri([v3(x - 0.03, 2.4, 0.0), v3(x + 0.065, 2.4, 0.0), v3(x + 0.017, 2.6, 0.0)], [[0.0; 2]; 3], &iron, n);
    }
    for y in [0.2f32, 1.2, 2.25] {
        gate.quad([v3(0.0, y, 0.0), v3(0.75, y, 0.0), v3(0.75, y + 0.06, 0.0), v3(0.0, y + 0.06, 0.0)], [[0.0; 2]; 4], &iron, n);
    }
    (m, gate)
}

/// Escalier vu en montant (transition d'étage).
fn stairs_scene() -> Mesh {
    let mut m = Mesh::new();
    let stone = Mat::tex(T_STONE, [0.6, 0.57, 0.53]);
    let wall = Mat::tex(T_PAPER_RED, [0.6, 0.55, 0.55]);
    for k in 0..16 {
        let (y0, y1) = (k as f32 * 0.3, (k + 1) as f32 * 0.3);
        let z = k as f32 * 0.4;
        m.quad([v3(-0.9, y0, z), v3(0.9, y0, z), v3(0.9, y1, z), v3(-0.9, y1, z)], [[0.0, 0.0], [1.0, 0.0], [1.0, 0.2], [0.0, 0.2]], &stone, v3(0.0, 0.0, -1.0));
        m.quad([v3(-0.9, y1, z), v3(0.9, y1, z), v3(0.9, y1, z + 0.4), v3(-0.9, y1, z + 0.4)], [[0.0, 0.0], [1.0, 0.0], [1.0, 0.3], [0.0, 0.3]], &stone, Vec3::UP);
    }
    for (x, n) in [(-0.9f32, 1.0f32), (0.9, -1.0)] {
        m.wall(v3(x, -1.0, -2.0), v3(0.0, 0.0, 8.5), Vec3::UP * 8.0, 0.5, 0.0, &wall, v3(n, 0.0, 0.0));
    }
    m
}

/// Sépare les triangles opaques des triangles transparents (dessinés après le paysage).
fn split_mesh(m: &Mesh) -> (Mesh, Mesh) {
    let mut a = Mesh::new();
    let mut b = Mesh::new();
    for t in &m.t {
        let dst = if matches!(t.blend, Blend::Opaque | Blend::Decal) { &mut a } else { &mut b };
        let base = dst.v.len() as u32;
        for k in 0..3 {
            dst.v.push(m.v[t.i[k] as usize]);
        }
        dst.t.push(MT { i: [base, base + 1, base + 2], ..*t });
    }
    (a, b)
}

impl Game {
    /// `native` : vrai hors navigateur (résolution et « Quitter » disponibles).
    pub fn new(mixer: SharedMixer, storage: Box<dyn Storage>, native: bool) -> Game {
        let opts = storage.load("options").map(|s| Options::decode(&s)).unwrap_or_default();
        let mut world = world::build();
        for room in &mut world.rooms {
            let (o, t) = split_mesh(&room.mesh);
            room.mesh = o;
            room.trans = t;
        }
        let outdoor = Outdoor::new();
        // Les arbres morts du marais sont des obstacles.
        for room in world.rooms.iter_mut().filter(|r| r.outdoor) {
            for &(x, z, r) in &outdoor.trees {
                if x > room.min.0 - r && x < room.max.0 + r && z > room.min.1 - r && z < room.max.1 + r {
                    room.obstacles.push(Obstacle { min: (x - r, z - r), max: (x + r, z + r), tall: true });
                }
            }
        }
        let st = initial_state(&world);
        let (ds, dl) = door_scene();
        let (gs, gl) = gate_scene();
        let mut bodies: Vec<Body> = (0..3).map(actors::zombie).collect();
        bodies.push(actors::butler(true));
        bodies.push(actors::butler(false));
        bodies.push(actors::brute());
        let mut g = Game {
            outdoor,
            hero: actors::heroine(),
            bodies,
            dog_body: actors::dog(false),
            hound_body: actors::dog(true),
            boss_body: actors::abomination(),
            corpse_body: actors::zombie(1),
            items_mesh: item_meshes(),
            door_scene: ds,
            door_leaf: dl,
            gate_scene: gs,
            gate_leaf: gl,
            stairs_scene: stairs_scene(),
            raster: Raster::new(textures::build()),
            mixer,
            checkpoint: st.clone(),
            st,
            world,
            mode: Mode::Title,
            prev_mode: Mode::Title,
            rng: Rng::new(0x5EED),
            time: 0.0,
            cam_idx: 0,
            cam: Camera::look_at(Vec3::ZERO, v3(0.0, 0.0, 1.0), 60.0),
            basis_yaw: 0.0,
            prev_dir: None,
            cam_changed: false,
            particles: Vec::new(),
            muzzle: 0.0,
            hurt_flash: 0.0,
            shake: 0.0,
            msgs: VecDeque::new(),
            after_msg: None,
            sel: 0,
            dog_t: 3.0,
            frog_t: 1.0,
            scheduled: Vec::new(),
            eyes: Vec::new(),
            fade: 0.0,
            banner_t: 0.0,
            window_msg: 0,
            has_saved: false,
            death_sound: false,
            debug_hide_player: false,
            opts,
            storage,
            requests: Vec::new(),
            native,
            fps: 0.0,
            fps_acc: (0.0, 0),
            pad: false,
            menu_rows: Vec::new(),
            last_mouse: None,
            prev_run: false,
            quick_turn: 0.0,
            save_exists: false,
        };
        if let Some(saved) = g.storage.load("save").and_then(|s| save::decode(&s, &g.world)) {
            g.checkpoint = saved;
            g.save_exists = true;
        }
        g.apply_audio();
        g.update_camera(true);
        g
    }

    pub fn options(&self) -> &Options {
        &self.opts
    }

    /// Requêtes (plein écran, taille de fenêtre, quitter) à traiter par la plateforme.
    pub fn take_requests(&mut self) -> Vec<Request> {
        std::mem::take(&mut self.requests)
    }

    /// La plateforme signale un changement de plein écran (ex. touche du navigateur).
    pub fn sync_fullscreen(&mut self, on: bool) {
        if self.opts.fullscreen != on {
            self.opts.fullscreen = on;
            self.save_options();
        }
    }

    fn save_options(&mut self) {
        let data = self.opts.encode();
        self.storage.save("options", &data);
    }

    fn apply_audio(&self) {
        if let Ok(mut m) = self.mixer.lock() {
            m.master = self.opts.master as f32 / 10.0;
        }
    }

    // ------------------------------------------------------------ utilitaires

    fn room(&self) -> &Room {
        &self.world.rooms[self.st.room]
    }

    fn flag(&self, f: usize) -> bool {
        self.st.flags[f]
    }

    /// Hauteur du sol sous un point d'une pièce.
    fn floor_y(&self, room: usize, p: Vec3) -> f32 {
        if self.world.rooms[room].outdoor {
            outdoor::walk_h(p.x, p.z)
        } else {
            0.0
        }
    }

    fn play(&self, s: Sfx, vol: f32, pan: f32, pitch: f32) {
        let vol = vol * self.opts.sfx as f32 / 10.0;
        if let Ok(mut m) = self.mixer.lock() {
            m.play(s, vol, pan, pitch);
        }
    }

    /// Bruitages d'ambiance (chiens, grenouilles) : volume « ambiance ».
    fn play_amb(&self, s: Sfx, vol: f32, pan: f32, pitch: f32) {
        let vol = vol * self.opts.ambient as f32 / 10.0;
        if let Ok(mut m) = self.mixer.lock() {
            m.play(s, vol, pan, pitch);
        }
    }

    /// Volume et panoramique d'une source dans la pièce.
    fn spatial(&self, pos: Vec3) -> (f32, f32) {
        let rel = pos - self.cam.eye;
        let pan = rel.flat().norm().dot(self.cam.right.flat().norm()) * 0.8;
        let d = pos.dist_xz(self.st.player.pos);
        ((1.0 - d / 16.0).clamp(0.15, 1.0), pan)
    }

    fn play_at(&self, s: Sfx, pos: Vec3, vol: f32, pitch: f32) {
        let (v, p) = self.spatial(pos);
        self.play(s, vol * v, p, pitch);
    }

    fn schedule(&mut self, delay: f32, sfx: Sfx, vol: f32, pan: f32, pitch: f32) {
        self.scheduled.push(Scheduled { at: self.time + delay, sfx, vol, pan, pitch });
    }

    fn message(&mut self, s: impl Into<String>) {
        self.msgs.push_back(s.into());
        if self.mode != Mode::Message {
            self.prev_mode = self.mode.clone();
            self.mode = Mode::Message;
        }
    }

    fn blood_burst(&mut self, at: Vec3, dir: Vec3, n: usize, col: [f32; 3]) {
        for _ in 0..n {
            let v = dir * self.rng.range(0.5, 2.5) + v3(self.rng.range(-1.0, 1.0), self.rng.range(0.0, 2.0), self.rng.range(-1.0, 1.0));
            let life = self.rng.range(0.4, 0.9);
            self.particles.push(Particle { pos: at, vel: v, life, col, size: 2 });
        }
    }

    fn update_camera(&mut self, force: bool) {
        let room = &self.world.rooms[self.st.room];
        let p = self.st.player.pos;
        // Hystérésis : on garde le plan courant tant qu'on reste près de sa zone,
        // pour éviter les bascules répétées à la frontière.
        if !force && self.cam_idx < room.cameras.len() {
            let c = &room.cameras[self.cam_idx];
            let m = 0.7;
            if p.x >= c.min.0 - m && p.x < c.max.0 + m && p.z >= c.min.1 - m && p.z < c.max.1 + m {
                return;
            }
        }
        let idx = room.camera_for(p).unwrap_or(self.cam_idx.min(room.cameras.len() - 1));
        if idx != self.cam_idx || force {
            self.cam_idx = idx;
            let c = &room.cameras[idx];
            self.cam = Camera::look_at(c.eye, c.target, c.fov);
            self.cam_changed = true;
        }
    }

    fn enter_room(&mut self, room: usize, pos: Vec3, yaw: f32) {
        self.st.room = room;
        self.st.player.pos = v3(pos.x, self.floor_y(room, pos), pos.z);
        self.st.player.yaw = yaw;
        self.st.player.speed = 0.0;
        self.st.player.aiming = false;
        self.st.player.aim_t = 0.0;
        self.st.player.knock = 0.0;
        self.particles.clear();
        self.eyes.clear();
        self.shake = 0.0;
        self.update_camera(true);
        self.basis_yaw = self.cam.fwd.flat().yaw();
        self.prev_dir = None;
        self.banner_t = 2.5;
    }

    fn new_game(&mut self) {
        self.st = initial_state(&self.world);
        self.checkpoint = self.st.clone();
        self.has_saved = false;
        self.death_sound = false;
        self.enter_room(ROOM_HALL, START_POS, 0.0);
        self.mode = Mode::Play;
        self.fade = 1.0;
        self.msgs.clear();
        self.message("Le manoir Ashcroft. La porte s'est refermée derrière vous, et la serrure a claqué...");
        self.message("Votre équipe a disparu dans le marais. Vous êtes seule. Trouvez un moyen de sortir.");
    }

    fn load_checkpoint(&mut self) {
        self.st = self.checkpoint.clone();
        self.death_sound = false;
        let p = self.st.player.clone();
        self.enter_room(self.st.room, p.pos, p.yaw);
        self.mode = Mode::Play;
        self.fade = 1.0;
        self.hurt_flash = 0.0;
    }

    // ------------------------------------------------------------ boucle

    pub fn frame(&mut self, dt: f32, inp: &Input) {
        let dt = dt.clamp(0.0, 0.05);
        self.time += dt;
        self.fade = (self.fade - dt * 2.5).max(0.0);
        self.hurt_flash = (self.hurt_flash - dt * 1.5).max(0.0);
        self.banner_t = (self.banner_t - dt).max(0.0);
        self.muzzle = (self.muzzle - dt).max(0.0);
        self.shake = (self.shake - dt * 2.0).max(0.0);
        self.pad = inp.using_pad;
        self.fps_acc.0 += dt;
        self.fps_acc.1 += 1;
        if self.fps_acc.0 >= 0.5 {
            self.fps = self.fps_acc.1 as f32 / self.fps_acc.0;
            self.fps_acc = (0.0, 0);
        }

        match self.mode.clone() {
            Mode::Title | Mode::Pause | Mode::Options { .. } | Mode::Controls { .. } => self.update_menu(inp),
            Mode::Play => self.update_play(dt, inp),
            Mode::Message => {
                if inp.confirm || inp.interact || inp.cancel {
                    self.msgs.pop_front();
                    self.play(Sfx::Cursor, 0.3, 0.0, 1.0);
                    if self.msgs.is_empty() {
                        self.mode = self.after_msg.take().unwrap_or_else(|| {
                            if matches!(self.prev_mode, Mode::Message) {
                                Mode::Play
                            } else {
                                self.prev_mode.clone()
                            }
                        });
                    }
                }
            }
            Mode::SaveChoice => {
                if inp.left || inp.right || inp.up || inp.down {
                    self.sel ^= 1;
                    self.play(Sfx::Cursor, 0.3, 0.0, 1.0);
                }
                if inp.cancel {
                    self.mode = Mode::Play;
                } else if inp.confirm || inp.interact {
                    if self.sel == 0 {
                        self.play(Sfx::Typewriter, 0.6, -0.2, 1.0);
                        self.st.saves += 1;
                        self.checkpoint = self.st.clone();
                        self.has_saved = true;
                        self.save_exists = true;
                        let data = save::encode(&self.checkpoint);
                        self.storage.save("save", &data);
                        self.mode = Mode::Play;
                        self.message("Progression sauvegardée.");
                    } else {
                        self.mode = Mode::Play;
                    }
                }
            }
            Mode::Inventory => self.update_inventory(inp),
            Mode::Door { t, door, shut } => {
                let t2 = t + dt;
                let mut shut = shut;
                if t2 > 1.75 && !shut {
                    shut = true;
                    let anim = self.world.rooms[self.st.room].doors[door].anim;
                    if !matches!(anim, DoorAnim::Stairs { .. }) {
                        self.play(Sfx::DoorShut, 0.6, 0.0, if anim == DoorAnim::Gate { 1.4 } else { 1.0 });
                    }
                }
                if t2 >= 2.3 {
                    let d = self.world.rooms[self.st.room].doors[door].clone();
                    self.enter_room(d.to_room, d.to_pos, d.to_yaw);
                    self.mode = Mode::Play;
                    self.fade = 1.0;
                    self.on_enter_room();
                } else {
                    self.mode = Mode::Door { t: t2, door, shut };
                }
            }
            Mode::Cutscene { cine, t } => self.update_cutscene(cine, t, dt, inp),
            Mode::GameOver { t } => {
                let t2 = t + dt;
                self.mode = Mode::GameOver { t: t2 };
                if t2 > 2.0 && (inp.confirm || inp.interact || inp.pause) {
                    self.load_checkpoint();
                }
            }
            Mode::Ending { t } => {
                let t2 = t + dt;
                self.mode = Mode::Ending { t: t2 };
                if t2 > story::ENDING_LEN && (inp.confirm || inp.interact || inp.pause) {
                    self.st = initial_state(&self.world);
                    self.enter_room(ROOM_HALL, START_POS, 0.0);
                    self.open(Mode::Title);
                }
            }
        }

        self.update_particles(dt);
        self.update_ambient(dt);
        self.render();
    }

    fn update_play(&mut self, dt: f32, inp: &Input) {
        self.st.play_time += dt;
        self.update_player(dt, inp);
        self.update_enemies(dt);
        self.update_camera(false);
        self.check_triggers();
        for b in &mut self.st.blood {
            b.2 = (b.2 + dt * 0.25).min(b.3);
        }
        if inp.pause && self.mode == Mode::Play {
            self.mode = Mode::Pause;
            self.sel = 0;
        }
        let p = &self.st.player;
        if inp.inventory && self.mode == Mode::Play && p.hp > 0 && p.grabbed <= 0.0 && p.knock <= 0.0 {
            self.mode = Mode::Inventory;
            self.sel = 0;
        }
    }

    // ------------------------------------------------------------ joueuse

    fn update_player(&mut self, dt: f32, inp: &Input) {
        let prev_run = self.prev_run;
        self.prev_run = inp.run;
        let cam_yaw = self.cam.fwd.flat().yaw();
        let room_idx = self.st.room;
        {
            let p = &mut self.st.player;
            p.fire_cd = (p.fire_cd - dt).max(0.0);
            p.recoil = (p.recoil - dt * 6.0).max(0.0);
            p.hurt_t = (p.hurt_t - dt * 2.5).max(0.0);
            p.invuln = (p.invuln - dt).max(0.0);
            if p.swing > 0.0 {
                p.swing += dt / 0.45;
                if p.swing >= 1.0 {
                    p.swing = 0.0;
                }
            }
        }

        if self.st.player.hp <= 0 {
            let p = &mut self.st.player;
            p.dead_t += dt;
            p.aiming = false;
            if !self.death_sound {
                self.death_sound = true;
                self.play(Sfx::Death, 0.8, 0.0, 1.0);
                self.schedule(0.9, Sfx::Fall, 0.6, 0.0, 1.0);
            }
            if self.st.player.dead_t > 3.0 {
                self.mode = Mode::GameOver { t: 0.0 };
            }
            return;
        }

        // Rechargement en cours.
        if self.st.player.reload_t > 0.0 {
            self.st.player.reload_t -= dt;
            if self.st.player.reload_t <= 0.0 {
                self.finish_reload();
            }
        }

        if self.st.player.grabbed > 0.0 || self.st.player.knock > 0.0 {
            let p = &mut self.st.player;
            p.grabbed = (p.grabbed - dt).max(0.0);
            p.knock = (p.knock - dt).max(0.0);
            p.speed = 0.0;
            p.aiming = false;
            p.swing = 0.0;
            p.aim_t = (p.aim_t - dt * 6.0).max(0.0);
            return;
        }

        // Base de déplacement relative à la caméra, conservée lors d'un
        // changement de plan tant que la direction ne change pas.
        let (mx, my) = inp.mv;
        let mag = (mx * mx + my * my).sqrt();
        let local_dir = mx.atan2(my);
        if mag < 0.15 {
            self.basis_yaw = cam_yaw;
            self.prev_dir = None;
        } else {
            match self.prev_dir {
                Some(pd) if !self.cam_changed => {
                    if wrap_angle(local_dir - pd).abs() > 0.6 {
                        self.basis_yaw = cam_yaw;
                    }
                }
                Some(_) => {}
                None => self.basis_yaw = cam_yaw,
            }
            self.prev_dir = Some(local_dir);
        }
        self.cam_changed = false;

        let mut step_sound: Option<f32> = None;
        let run_edge = inp.run && !prev_run;
        let was_aiming = self.st.player.aiming;
        let aiming = inp.aim;
        self.st.player.aiming = aiming;
        let wading = self.room().outdoor && outdoor::wading(self.st.player.pos.x, self.st.player.pos.z);

        if aiming {
            {
                let p = &mut self.st.player;
                p.aim_t = (p.aim_t + dt * 6.0).min(1.0);
                p.speed = 0.0;
                p.move_amt = (p.move_amt - dt * 6.0).max(0.0);
            }
            if !was_aiming {
                self.st.player.lock = self.find_target();
            }
            let pyaw = self.st.player.yaw;
            let mut target_yaw: Option<(f32, f32)> = None;
            if inp.aim_mouse {
                if let Some((sx, sy)) = inp.mouse {
                    let ray = self.cam.ray(sx, sy);
                    let plane = 1.0 + self.st.player.pos.y;
                    if ray.y.abs() > 1e-3 {
                        let t = (plane - self.cam.eye.y) / ray.y;
                        if t > 0.0 {
                            let hit = self.cam.eye + ray * t;
                            let d = hit - self.st.player.pos;
                            if d.flat().len() > 0.2 {
                                target_yaw = Some((d.yaw(), 14.0));
                                self.st.player.lock = None;
                            }
                        }
                    }
                }
            } else if mag > 0.3 {
                if inp.using_pad && !self.opts.classic {
                    target_yaw = Some((self.basis_yaw + local_dir, 4.0));
                    self.st.player.lock = None;
                } else if mx.abs() > 0.1 {
                    target_yaw = Some((pyaw + mx.signum() * 1.0, 2.6));
                    self.st.player.lock = None;
                }
            }
            if target_yaw.is_none() {
                if let Some(zi) = self.st.player.lock {
                    if !self.st.enemies[zi].targetable(self.st.room) {
                        self.st.player.lock = self.find_target();
                    } else {
                        let tp = self.st.enemies[zi].pos;
                        target_yaw = Some(((tp - self.st.player.pos).yaw(), 7.0));
                    }
                }
            }
            if let Some((ty, rate)) = target_yaw {
                let p = &mut self.st.player;
                p.yaw = approach_angle(p.yaw, ty, rate * dt);
            }
            if inp.fire && self.st.player.aim_t > 0.6 {
                self.fire();
            }
        } else {
            let p = &mut self.st.player;
            p.aim_t = (p.aim_t - dt * 6.0).max(0.0);
            p.lock = None;
            let danger = p.hp < 34;
            let slow = if danger { 0.7 } else { 1.0 } * if wading { 0.62 } else { 1.0 };
            if self.opts.classic {
                // Contrôles « tank » : gauche/droite pivotent, haut/bas avancent/reculent.
                if self.quick_turn > 0.0 {
                    let step = (dt / 0.4).min(self.quick_turn / 0.4) * PI;
                    p.yaw = wrap_angle(p.yaw + step);
                    self.quick_turn -= dt;
                    p.speed = 0.0;
                } else if my < -0.5 && run_edge {
                    self.quick_turn = 0.4;
                } else {
                    if mx.abs() > 0.15 {
                        p.yaw = wrap_angle(p.yaw + mx * 2.8 * dt);
                    }
                    let target = if my > 0.15 {
                        (if inp.run { 3.4 } else { 1.45 }) * slow * my.min(1.0)
                    } else if my < -0.15 {
                        -0.9 * slow
                    } else {
                        0.0
                    };
                    p.speed += (target - p.speed) * (dt * 8.0).min(1.0);
                    p.pos += Vec3::from_yaw(p.yaw) * (p.speed * dt);
                }
                let running = if inp.run && p.speed > 2.0 { 1.0 } else { 0.0 };
                p.run_amt += (running - p.run_amt) * (dt * 5.0).min(1.0);
                if p.speed.abs() > 0.1 || mx.abs() > 0.15 {
                    p.move_amt = (p.move_amt + dt * 6.0).min(1.0);
                } else {
                    p.move_amt = (p.move_amt - dt * 5.0).max(0.0);
                }
            } else if mag > 0.15 {
                let desired = self.basis_yaw + local_dir;
                let diff = wrap_angle(desired - p.yaw);
                p.yaw = approach_angle(p.yaw, desired, 10.0 * dt);
                let run = inp.run;
                let max_speed = if run { 3.4 } else { 1.45 } * slow;
                let target = max_speed * mag.min(1.0);
                p.speed += (target - p.speed) * (dt * 8.0).min(1.0);
                let fwd = diff.cos().max(0.0);
                let step = Vec3::from_yaw(p.yaw) * (p.speed * fwd * dt);
                p.pos += step;
                p.run_amt += ((if run { 1.0 } else { 0.0 }) - p.run_amt) * (dt * 5.0).min(1.0);
                p.move_amt = (p.move_amt + dt * 6.0).min(1.0);
            } else {
                p.speed *= (1.0 - dt * 10.0).max(0.0);
                p.move_amt = (p.move_amt - dt * 5.0).max(0.0);
            }
            let stride = 1.15 + p.run_amt * 0.9;
            let old = p.phase;
            p.phase += p.speed * dt / stride * TAU;
            if (old / PI).floor() != (p.phase / PI).floor() && p.speed.abs() > 0.3 {
                step_sound = Some(0.25 + p.run_amt * 0.2);
            }
            if p.phase.abs() > TAU * 100.0 {
                p.phase %= TAU;
            }
        }

        if let Some(vol) = step_sound {
            let pitch = self.rng.range(0.85, 1.1);
            if wading {
                self.play(Sfx::Splash, vol * 1.4, 0.0, pitch);
            } else {
                self.play(Sfx::Step, vol, 0.0, pitch);
            }
        }

        // Collisions.
        let room = &self.world.rooms[room_idx];
        let mut pos = room.collide(self.st.player.pos, PLAYER_R);
        for z in &self.st.enemies {
            if z.room != room_idx || !z.solid() {
                continue;
            }
            let d = pos.flat() - z.pos.flat();
            let l = d.len();
            let min = PLAYER_R + z.radius();
            if l < min && l > 1e-4 {
                pos += d * ((min - l) / l);
            }
        }
        if room_idx == ROOM_KITCHEN && !self.flag(F_CRYPT_KEY) {
            // Edgar, assis contre le mur.
            let c = world::EDGAR_POS + v3(0.35, 0.0, 0.0);
            let d = pos.flat() - c.flat();
            let l = d.len();
            if l < PLAYER_R + 0.4 && l > 1e-4 {
                pos += d * ((PLAYER_R + 0.4 - l) / l);
            }
        }
        let mut pos = room.collide(pos, PLAYER_R);
        pos.y = self.floor_y(room_idx, pos);
        self.st.player.pos = pos;

        if inp.reload {
            self.start_reload();
        }
        if inp.interact && !aiming {
            self.interact();
        }
    }

    // ------------------------------------------------------------ interactions

    /// Objet ramassable devant la joueuse.
    fn item_in_reach(&self) -> Option<usize> {
        let p = &self.st.player;
        let front = p.pos + Vec3::from_yaw(p.yaw) * 0.6;
        let mut best: Option<(usize, f32)> = None;
        for (i, it) in self.world.items.iter().enumerate() {
            if it.room != self.st.room || !self.item_visible(i) {
                continue;
            }
            let d = front.dist_xz(it.pos).min(p.pos.dist_xz(it.pos) + 0.15);
            if d < 0.85 && best.map_or(true, |b| d < b.1) {
                best = Some((i, d));
            }
        }
        best.map(|b| b.0)
    }

    fn item_visible(&self, i: usize) -> bool {
        !self.st.taken[i] && self.world.items[i].flag.map_or(true, |f| self.flag(f))
    }

    fn door_in_reach(&self) -> Option<usize> {
        let p = &self.st.player;
        let fwd = Vec3::from_yaw(p.yaw);
        self.room().doors.iter().position(|d| p.pos.dist_xz(d.pos) < 1.3 && fwd.dot(-d.normal) > 0.4)
    }

    fn interact_in_reach(&self) -> Option<Interact> {
        let p = &self.st.player;
        let front = p.pos + Vec3::from_yaw(p.yaw) * 0.6;
        let npc_gone = self.flag(F_CRYPT_KEY);
        self.room().interacts.iter().find(|it| front.dist_xz(it.pos) < it.radius && !(npc_gone && matches!(it.kind, InteractKind::Npc))).cloned()
    }

    fn interact(&mut self) {
        if let Some(i) = self.item_in_reach() {
            self.pick_up(i);
            return;
        }
        if let Some(di) = self.door_in_reach() {
            let d = self.room().doors[di].clone();
            self.use_door(di, &d);
            return;
        }
        if let Some(it) = self.interact_in_reach() {
            match it.kind {
                InteractKind::Message(s) => self.message(s),
                InteractKind::Typewriter => {
                    self.message("Une vieille machine à écrire. Une feuille vierge est engagée.");
                    self.after_msg = Some(Mode::SaveChoice);
                    self.sel = 0;
                }
                InteractKind::Window => {
                    const MSGS: [&str; 3] = [
                        "Dehors, la lune éclaire le marais. Des chiens aboient au loin.",
                        "La brume rampe sur l'eau noire. Les montagnes se découpent sous la lune.",
                        "La fenêtre est condamnée. Les barreaux sont scellés dans la pierre.",
                    ];
                    let m = MSGS[self.window_msg % MSGS.len()];
                    self.window_msg += 1;
                    self.message(m);
                }
                InteractKind::Note(pages) => {
                    for p in pages {
                        self.message(*p);
                    }
                }
                InteractKind::Portrait(id) => self.press_portrait(id),
                InteractKind::Npc => self.talk_edgar(),
                InteractKind::MusicBox => self.music_box(),
                InteractKind::Boat => self.board_boat(),
            }
        }
    }

    fn pick_up(&mut self, i: usize) {
        let kind = self.world.items[i].kind;
        self.st.taken[i] = true;
        self.play(Sfx::Pickup, 0.5, 0.0, 1.0);
        let found = format!("Vous avez trouvé : {}.", kind.name());
        match kind {
            ItemKind::Ammo(n) => {
                self.st.inv.reserve += n;
                self.message(found);
            }
            ItemKind::Shells(n) => {
                self.st.inv.shells += n;
                self.message(found);
                if !self.st.inv.has_shotgun {
                    self.message("Des cartouches de calibre 12. Il faudrait un fusil pour s'en servir.");
                }
            }
            ItemKind::Shotgun => {
                self.st.inv.has_shotgun = true;
                self.st.inv.shells_mag = 4;
                self.message(found);
                self.message("Un fusil à pompe de chasse, chargé de 4 cartouches. Équipez-le depuis l'inventaire.");
                self.message("Dévastateur de près. Chaque cartouche compte.");
            }
            ItemKind::Medkit => {
                self.st.inv.medkits += 1;
                self.message(found);
                self.message("Utilisez-la depuis l'inventaire pour restaurer toute votre santé.");
            }
            ItemKind::Herb => {
                self.st.inv.herbs += 1;
                self.message(found);
                self.message("Une plante aux vertus cicatrisantes. Elle soigne un peu.");
            }
            ItemKind::Key(k) => {
                self.st.inv.keys.push(k);
                self.message(found);
                self.on_key_found(k);
            }
        }
    }

    fn use_door(&mut self, di: usize, d: &Door) {
        if let Some(why) = self.door_blocked() {
            self.play(Sfx::Locked, 0.6, 0.0, 0.8);
            self.message(why);
            return;
        }
        if let Some((lock, key)) = d.lock {
            if !self.st.unlocked[lock] {
                if let Some(k) = self.st.inv.keys.iter().position(|&k| k == key) {
                    self.st.inv.keys.remove(k);
                    self.st.unlocked[lock] = true;
                    self.play(Sfx::Reload, 0.5, 0.0, 0.7);
                    self.message(format!("Vous utilisez la {}. La serrure cède dans un déclic.", key_name(key)));
                    if key == KEY_FRONT {
                        self.message("La porte d'entrée s'ouvre en grinçant. L'air froid du marais s'engouffre dans le hall.");
                    } else {
                        self.message("La clé n'est plus utile. Vous la jetez.");
                    }
                } else {
                    self.play(Sfx::Locked, 0.6, 0.0, 1.0);
                    self.message(lock_msg(key));
                }
                return;
            }
        }
        let pitch = self.rng.range(0.9, 1.1);
        match d.anim {
            DoorAnim::Stairs { .. } => self.play(Sfx::Step, 0.4, 0.0, 0.8),
            DoorAnim::Gate => self.play(Sfx::Creak, 0.6, 0.0, pitch * 1.3),
            DoorAnim::Door => self.play(Sfx::Creak, 0.6, 0.0, pitch),
        }
        self.mode = Mode::Door { t: 0.0, door: di, shut: false };
    }

    fn update_inventory(&mut self, inp: &Input) {
        let slots = self.inv_slots();
        if inp.up {
            self.sel = (self.sel + slots.len() - 1) % slots.len();
            self.play(Sfx::Cursor, 0.3, 0.0, 1.0);
        }
        if inp.down {
            self.sel = (self.sel + 1) % slots.len();
            self.play(Sfx::Cursor, 0.3, 0.0, 1.0);
        }
        if inp.cancel || inp.inventory || inp.pause {
            self.mode = Mode::Play;
            return;
        }
        if inp.confirm || inp.interact {
            match slots[self.sel.min(slots.len() - 1)].0 {
                Slot::Weapon(w) => {
                    if self.st.inv.weapon != w {
                        self.st.inv.weapon = w;
                        self.st.player.reload_t = 0.0;
                        self.play(if w == Weapon::Shotgun { Sfx::Pump } else { Sfx::Reload }, 0.6, 0.0, 1.0);
                    } else if !self.reload_now() {
                        self.play(Sfx::Locked, 0.4, 0.0, 1.3);
                    }
                }
                Slot::Medkit => self.heal(100, true),
                Slot::Herb => self.heal(35, false),
                Slot::Key(k) => {
                    self.message(key_desc(k));
                    self.message("Elle s'utilise automatiquement sur la bonne porte.");
                    self.after_msg = Some(Mode::Inventory);
                }
            }
        }
    }

    fn heal(&mut self, amount: i32, medkit: bool) {
        if self.st.player.hp >= 100 {
            self.message("Vous êtes en pleine forme.");
            self.after_msg = Some(Mode::Inventory);
            return;
        }
        self.st.player.hp = (self.st.player.hp + amount).min(100);
        if medkit {
            self.st.inv.medkits -= 1;
            self.message("Vous vous soignez. Vous vous sentez beaucoup mieux.");
        } else {
            self.st.inv.herbs -= 1;
            self.message("Vous mâchez les feuilles amères. La douleur s'atténue un peu.");
        }
        self.after_msg = Some(Mode::Inventory);
        self.play(Sfx::Pickup, 0.5, 0.0, 0.8);
        self.sel = 0;
    }

    fn inv_slots(&self) -> Vec<(Slot, String)> {
        let inv = &self.st.inv;
        let mark = |w: Weapon| if inv.weapon == w { ">" } else { " " };
        let mut v = vec![(Slot::Weapon(Weapon::Pistol), format!("{}PISTOLET  {:>2}/{}", mark(Weapon::Pistol), inv.mag, inv.reserve))];
        if inv.has_shotgun {
            v.push((Slot::Weapon(Weapon::Shotgun), format!("{}FUSIL     {:>2}/{}", mark(Weapon::Shotgun), inv.shells_mag, inv.shells)));
        }
        v.push((Slot::Weapon(Weapon::Knife), format!("{}COUTEAU", mark(Weapon::Knife))));
        if inv.medkits > 0 {
            v.push((Slot::Medkit, format!(" TROUSSE SOINS x{}", inv.medkits)));
        }
        if inv.herbs > 0 {
            v.push((Slot::Herb, format!(" HERBE MÉDIC. x{}", inv.herbs)));
        }
        for &k in &inv.keys {
            v.push((Slot::Key(k), format!(" {}", key_name(k))));
        }
        v
    }

    // ------------------------------------------------------------ ambiance

    fn update_particles(&mut self, dt: f32) {
        let floor = self.floor_y(self.st.room, self.st.player.pos);
        for p in &mut self.particles {
            p.life -= dt;
            if p.pos.y > floor + 0.02 {
                p.vel.y -= 9.0 * dt;
                p.pos += p.vel * dt;
            } else {
                p.pos.y = floor + 0.015;
            }
        }
        self.particles.retain(|p| p.life > 0.0);
        for e in &mut self.eyes {
            e.1 -= dt;
        }
        self.eyes.retain(|e| e.1 > 0.0);
    }

    fn update_ambient(&mut self, dt: f32) {
        let outside = self.room().outdoor;
        let windows = !self.room().windows.is_empty() || outside;
        let room = self.st.room;
        let fire_pos = v3(-12.0, 0.5, 11.6);
        let fire_vol = if room == ROOM_DINING && !matches!(self.mode, Mode::Door { .. } | Mode::Title) {
            (1.0 - self.st.player.pos.dist_xz(fire_pos) / 11.0).max(0.0) * 0.5
        } else {
            0.0
        };
        let fire_pan = if room == ROOM_DINING { self.spatial(fire_pos).1 } else { 0.0 };
        let title = self.mode == Mode::Title;
        let amb = self.opts.ambient as f32 / 10.0;
        let sfx = self.opts.sfx as f32 / 10.0;
        let in_game = matches!(self.mode, Mode::Play | Mode::Pause | Mode::Message | Mode::Inventory);
        let fight = if in_game && self.boss_active().is_some() { 0.32 } else { 0.0 };
        let crypt = matches!(room, ROOM_CRYPT | ROOM_CRYPT_STAIRS);
        if let Ok(mut m) = self.mixer.lock() {
            m.set_loop(1, Sfx::Wind, if outside { 0.4 } else if windows { 0.22 } else { 0.12 } * amb, 0.0);
            m.set_loop(2, Sfx::Drone, if title { 0.12 } else if crypt { 0.35 } else { 0.2 } * amb, 0.0);
            m.set_loop(3, Sfx::Fire, fire_vol * sfx, fire_pan);
            m.set_loop(4, Sfx::BossLoop, fight * sfx, 0.0);
        }

        // Sons programmés.
        let now = self.time;
        let due: Vec<Scheduled> = {
            let (a, b): (Vec<Scheduled>, Vec<Scheduled>) = self.scheduled.drain(..).partition(|s| s.at <= now);
            self.scheduled = b;
            a
        };
        for s in due {
            if matches!(s.sfx, Sfx::Bark | Sfx::Bark2 | Sfx::Howl) {
                self.play_amb(s.sfx, s.vol, s.pan, s.pitch);
            } else {
                self.play(s.sfx, s.vol, s.pan, s.pitch);
            }
        }

        // Chiens au loin (à peine audibles dans la crypte).
        self.dog_t -= dt;
        if self.dog_t <= 0.0 {
            self.dog_t = self.rng.range(6.0, 15.0);
            let a = self.rng.range(0.0, TAU);
            let dir = v3(a.sin(), 0.0, a.cos());
            let pan = dir.dot(self.cam.right.flat().norm()) * 0.9;
            let muffle = if outside {
                1.2
            } else if crypt {
                0.25
            } else if windows {
                1.0
            } else {
                0.55
            };
            let r = self.rng.f();
            let (sfx, vol) = if r < 0.18 {
                (Sfx::Howl, 0.3)
            } else if r < 0.6 {
                (Sfx::Bark, 0.4)
            } else {
                (Sfx::Bark2, 0.45)
            };
            let pitch = self.rng.range(0.85, 1.15);
            let vol = vol * muffle * self.rng.range(0.6, 1.0);
            self.play_amb(sfx, vol, pan, pitch);
            if self.rng.chance(0.4) {
                let pitch = self.rng.range(0.8, 1.2);
                let pan2 = (pan + self.rng.range(-0.5, 0.5)).clamp(-1.0, 1.0);
                let delay = self.rng.range(0.6, 1.4);
                self.schedule(delay, Sfx::Bark2, 0.3 * muffle, pan2, pitch);
            }
            // Yeux luisants dans l'obscurité.
            let center = v3(-2.0, 0.0, 13.0);
            let d = self.rng.range(26.0, 42.0);
            self.eyes.push((center + dir * d + v3(0.0, -0.1, 0.0), 4.0));
        }
        // Grenouilles.
        self.frog_t -= dt;
        if self.frog_t <= 0.0 {
            self.frog_t = self.rng.range(0.6, 3.0) * if outside { 0.5 } else { 1.0 };
            let k = if outside {
                1.6
            } else if crypt {
                0.0
            } else if windows {
                1.0
            } else {
                0.5
            };
            let vol = self.rng.range(0.04, 0.12) * k;
            let pan = self.rng.range(-0.9, 0.9);
            let pitch = self.rng.range(0.75, 1.3);
            self.play_amb(Sfx::Frog, vol, pan, pitch);
        }
    }

    // ------------------------------------------------------------ débogage

    /// Place la joueuse pour des captures d'écran automatiques.
    pub fn debug_setup(&mut self, room: usize, pos: Vec3, yaw: f32, aim: bool) {
        if self.mode == Mode::Title {
            self.new_game();
        }
        self.msgs.clear();
        self.mode = Mode::Play;
        self.enter_room(room, pos, yaw);
        self.fade = 0.0;
        self.banner_t = 0.0;
        self.st.player.aim_t = if aim { 1.0 } else { 0.0 };
        self.muzzle = 0.0;
    }

    pub fn debug_door(&mut self, t: f32) {
        self.mode = Mode::Door { t, door: 0, shut: true };
    }

    /// Transition de porte numéro `door` de la pièce courante, à l'instant `t`.
    pub fn debug_door_of(&mut self, door: usize, t: f32) {
        self.mode = Mode::Door { t, door, shut: true };
    }

    pub fn debug_title(&mut self) {
        self.mode = Mode::Title;
    }

    pub fn debug_mode(&mut self, name: &str) {
        self.sel = 0;
        self.mode = match name {
            "title" => Mode::Title,
            "pause" => Mode::Pause,
            "options" => Mode::Options { title: false },
            "controls" => Mode::Controls { title: false, options: false },
            "gameover" => Mode::GameOver { t: 3.0 },
            "ending" => Mode::Ending { t: story::ENDING_LEN + 1.0 },
            _ => Mode::Play,
        };
    }

    pub fn debug_inventory(&mut self) {
        self.mode = Mode::Inventory;
    }

    pub fn debug_message(&mut self, s: &str) {
        self.msgs.clear();
        self.message(s);
    }

    pub fn debug_hide_player(&mut self, hide: bool) {
        self.debug_hide_player = hide;
    }

    pub fn debug_shoot(&mut self) {
        self.muzzle = 0.07;
    }

    /// Donne l'équipement complet et lève les drapeaux d'histoire voulus.
    pub fn debug_progress(&mut self, flags: &[usize]) {
        for &f in flags {
            self.set_flag(f);
        }
        let inv = &mut self.st.inv;
        inv.has_shotgun = true;
        inv.shells_mag = 6;
        inv.shells = 12;
        inv.reserve = 45;
        inv.medkits = 2;
        inv.herbs = 1;
        inv.keys = vec![KEY_LAB, KEY_CRYPT];
    }

    pub fn debug_weapon(&mut self, w: &str) {
        self.st.inv.weapon = match w {
            "shotgun" => Weapon::Shotgun,
            "knife" => Weapon::Knife,
            _ => Weapon::Pistol,
        };
    }

    /// Lance une cinématique et la fait avancer de `t` secondes.
    pub fn debug_cutscene(&mut self, which: &str, t: f32) {
        let cine = match which {
            "boss" => Cine::Boss,
            "hound" => Cine::Hound,
            _ => Cine::Brute,
        };
        self.start_cutscene(cine);
        let none = Input::default();
        for _ in 0..(t / 0.05) as usize {
            if let Mode::Cutscene { cine, t } = self.mode.clone() {
                self.update_cutscene(cine, t, 0.05, &none);
                self.update_particles(0.05);
            }
        }
    }

    /// Amène le premier ennemi du type voulu devant la joueuse, en pleine attaque.
    pub fn debug_enemy(&mut self, kind: EnemyKind, offset: Vec3, attack: f32) {
        let room = self.st.room;
        let ppos = self.st.player.pos;
        self.st.player.invuln = 10.0;
        if let Some(e) = self.st.enemies.iter_mut().find(|e| e.kind == kind) {
            e.room = room;
            e.pos = ppos + offset;
            e.yaw = (ppos - e.pos).yaw();
            e.alerted = true;
            e.debug_attack(attack);
        }
    }

    /// Diagnostic de cadrage : pour chaque pièce, vérifie que la joueuse, les
    /// portes et les objets sont visibles par la caméra active quand on s'en
    /// approche, et compte les pixels non dessinés (trous) hors fenêtres.
    pub fn debug_visibility(&mut self) -> Vec<String> {
        use crate::raster::{H, W};
        let mut report = Vec::new();
        self.debug_setup(0, START_POS, 0.0, false);
        let saved_enemies: Vec<usize> = self.st.enemies.iter().map(|z| z.room).collect();
        // On retire les ennemis pour ne mesurer que le décor.
        for z in &mut self.st.enemies {
            z.room = NOWHERE;
        }
        for ri in 0..self.world.rooms.len() {
            let room = &self.world.rooms[ri];
            let center = v3((room.min.0 + room.max.0) * 0.5, 0.0, (room.min.1 + room.max.1) * 0.5);
            let mut targets: Vec<(String, Vec3, Vec3)> = Vec::new();
            for (di, d) in room.doors.iter().enumerate() {
                let to = self.world.rooms[d.to_room].name;
                targets.push((format!("porte {di} (vers {to})"), d.pos + d.normal * 0.8, d.pos + d.normal * 0.05 + v3(0.0, 1.3, 0.0)));
            }
            for (ii, it) in self.world.items.iter().enumerate() {
                if it.room != ri {
                    continue;
                }
                let toward = (center - it.pos).flat().norm();
                targets.push((format!("objet {ii} ({})", it.kind.name()), it.pos.flat() + toward * 0.6, it.pos + v3(0.0, 0.08, 0.0)));
            }
            for it in &room.interacts {
                if let InteractKind::Window = it.kind {
                    continue;
                }
                targets.push(("point d'intérêt".to_string(), it.pos, it.pos + v3(0.0, 0.9, 0.0)));
            }
            let (mut grid_total, mut grid_hidden) = (0, 0);
            let mut hidden_spots = Vec::new();
            let mut holes_max = 0usize;
            let mut x = room.min.0 + 0.4;
            while x < room.max.0 - 0.3 {
                let mut z = room.min.1 + 0.4;
                while z < room.max.1 - 0.3 {
                    let p = v3(x, 0.0, z);
                    if room.collide(p, PLAYER_R).dist_xz(p) < 1e-3 {
                        targets.push(("#".to_string(), p, p + v3(0.0, 1.1, 0.0)));
                    }
                    z += 0.8;
                }
                x += 0.8;
            }
            let outdoor = room.outdoor;
            for (name, stand, point) in targets {
                let yaw = (point - stand).flat().yaw();
                self.debug_setup(ri, stand, if name == "#" { 0.0 } else { yaw }, false);
                let point = if name == "#" || name.starts_with("porte") || name.starts_with("point") { point + v3(0.0, self.floor_y(ri, point), 0.0) } else { point };
                self.debug_hide_player = name != "#";
                self.render_scene(false);
                let r = &self.raster;
                // Trous : pixels jamais couverts hors des fenêtres.
                let mut polys: Vec<[(f32, f32); 4]> = Vec::new();
                let mut full = false;
                for w in &self.world.rooms[ri].windows {
                    let pr: Vec<_> = w.iter().map(|c| r.cam.project(*c)).collect();
                    if pr.iter().any(|p| p.is_none()) {
                        full = true;
                    } else {
                        let q: Vec<(f32, f32)> = pr.iter().map(|p| (p.unwrap().0, p.unwrap().1)).collect();
                        polys.push([q[0], q[1], q[2], q[3]]);
                    }
                }
                if !full && !outdoor {
                    let mut holes = 0;
                    for py in 0..H {
                        for px in 0..W {
                            if self.raster.depth[py * W + px] > 2e-9 {
                                continue;
                            }
                            let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
                            let inside = polys.iter().any(|q| {
                                let mut sign = 0.0f32;
                                (0..4).all(|k| {
                                    let (ax, ay) = q[k];
                                    let (bx, by) = q[(k + 1) % 4];
                                    let c = (bx - ax) * (fy - ay) - (by - ay) * (fx - ax);
                                    if c.abs() < 2.0 {
                                        return true;
                                    }
                                    if sign == 0.0 {
                                        sign = c.signum();
                                    }
                                    c.signum() == sign
                                })
                            });
                            if !inside {
                                holes += 1;
                                if std::env::var("HOLES").is_ok() && holes <= 3 {
                                    eprintln!("trou room {ri} cam {} stand ({:.1},{:.1}) pixel ({px},{py})", self.cam_idx, stand.x, stand.z);
                                }
                            }
                        }
                    }
                    holes_max = holes_max.max(holes);
                }
                let r = &self.raster;
                let problem = match r.cam.project(point) {
                    None => Some("derrière la caméra"),
                    Some((sx, sy, z)) => {
                        if sx < 6.0 || sy < 6.0 || sx > W as f32 - 6.0 || sy > H as f32 - 6.0 {
                            Some("hors champ")
                        } else {
                            let d = r.depth[sy as usize * W + sx as usize];
                            if d > (1.0 / z) * 1.06 {
                                Some("masqué")
                            } else {
                                None
                            }
                        }
                    }
                };
                if name == "#" {
                    grid_total += 1;
                    if problem.is_some() {
                        grid_hidden += 1;
                        hidden_spots.push(format!("({:.1},{:.1})", stand.x, stand.z));
                    }
                } else if let Some(pb) = problem {
                    report.push(format!("[{}] {} : {} (caméra {})", self.world.rooms[ri].name, name, pb, self.cam_idx));
                }
            }
            report.push(format!(
                "[{}] joueuse invisible sur {}/{} positions {} ; trous max {} px",
                self.world.rooms[ri].name,
                grid_hidden,
                grid_total,
                hidden_spots.join(" "),
                holes_max
            ));
        }
        self.debug_hide_player = false;
        for (z, r) in self.st.enemies.iter_mut().zip(saved_enemies) {
            z.room = r;
        }
        report
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Slot {
    Weapon(Weapon),
    Medkit,
    Herb,
    Key(u8),
}

/// Retour à la ligne simple sur les espaces.
fn wrap(s: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in s.split_whitespace() {
        if !cur.is_empty() && cur.chars().count() + 1 + word.chars().count() > width {
            lines.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(word);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

#[cfg(test)]
mod tests;
