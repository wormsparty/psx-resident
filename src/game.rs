//! Logique de jeu : joueuse, zombies, objets, portes, menus, sauvegarde.

use crate::actors::{self, Body, Pose};
use crate::audio::{SharedMixer, Sfx};
use crate::input::Input;
use crate::options::{Options, Request, Storage};
use crate::math::*;
use crate::mesh::*;
use crate::outdoor::{self, Outdoor};
use crate::raster::{Camera, Light, Lighting, Raster, H, W};
use crate::textures::{self, *};
use crate::world::{self, *};
use std::collections::VecDeque;
use std::f32::consts::{PI, TAU};

mod menu;
mod save;

const MAG_SIZE: u32 = 15;
const PLAYER_R: f32 = 0.28;
const ZOMBIE_R: f32 = 0.3;
const LIGHT_GAIN: f32 = 1.6;
const AMBIENT_GAIN: f32 = 2.4;

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
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ZState {
    Eating,
    Rising,
    Idle,
    Chase,
    Grab,
    Stagger,
    Dying,
    Dead,
}

#[derive(Clone, Debug)]
struct Zombie {
    room: usize,
    pos: Vec3,
    yaw: f32,
    hp: i32,
    state: ZState,
    t: f32,
    phase: f32,
    groan_t: f32,
    attack_cd: f32,
    variant: u32,
    alerted: bool,
    bitten: bool,
    fall_back: bool,
}

#[derive(Clone, Debug, Default)]
struct Inventory {
    mag: u32,
    reserve: u32,
    medkits: u32,
    keys: Vec<u8>,
}

#[derive(Clone, Debug)]
struct State {
    room: usize,
    player: Player,
    zombies: Vec<Zombie>,
    taken: Vec<bool>,
    unlocked: Vec<bool>,
    inv: Inventory,
    blood: Vec<(usize, Vec3, f32, f32)>,
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
    GameOver { t: f32 },
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
    zbodies: Vec<Body>,
    corpse_body: Body,
    items_mesh: Vec<Mesh>,
    door_scene: Mesh,
    door_leaf: Mesh,
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

fn initial_state(world: &World) -> State {
    State {
        room: ROOM_HALL,
        player: Player {
            pos: v3(0.0, 0.0, 2.4),
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
        },
        zombies: world
            .zombies
            .iter()
            .enumerate()
            .map(|(i, z)| Zombie {
                room: z.room,
                pos: z.pos,
                yaw: z.yaw,
                hp: 4 + (i as i32 % 3),
                state: if z.eating { ZState::Eating } else { ZState::Idle },
                t: 0.0,
                phase: i as f32 * 1.3,
                groan_t: 2.0 + i as f32 * 1.7,
                attack_cd: 0.0,
                variant: i as u32,
                alerted: false,
                bitten: false,
                fall_back: true,
            })
            .collect(),
        taken: vec![false; world.items.len()],
        unlocked: vec![false; 4],
        inv: Inventory { mag: 12, reserve: 0, medkits: 0, keys: vec![] },
        blood: world
            .rooms
            .iter()
            .enumerate()
            .flat_map(|(ri, r)| r.corpses.iter().map(move |c| (ri, c.0 + v3(0.0, 0.0, 0.3), 0.9, 0.9)))
            .collect(),
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
    vec![ammo, med, key]
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
        let st = initial_state(&world);
        let (ds, dl) = door_scene();
        let mut g = Game {
            outdoor: Outdoor::new(),
            hero: actors::heroine(),
            zbodies: (0..3).map(actors::zombie).collect(),
            corpse_body: actors::zombie(1),
            items_mesh: item_meshes(),
            door_scene: ds,
            door_leaf: dl,
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
        self.st.player.pos = pos;
        self.st.player.yaw = yaw;
        self.st.player.speed = 0.0;
        self.st.player.aiming = false;
        self.st.player.aim_t = 0.0;
        self.particles.clear();
        self.eyes.clear();
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
        self.enter_room(ROOM_HALL, v3(0.0, 0.0, 2.4), 0.0);
        self.mode = Mode::Play;
        self.fade = 1.0;
        self.msgs.clear();
        self.message("Le manoir Ashcroft. Les portes se sont refermées derrière vous...");
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
                    self.play(Sfx::DoorShut, 0.6, 0.0, 1.0);
                }
                if t2 >= 2.3 {
                    let d = self.world.rooms[self.st.room].doors[door].clone();
                    self.enter_room(d.to_room, d.to_pos, d.to_yaw);
                    self.mode = Mode::Play;
                    self.fade = 1.0;
                } else {
                    self.mode = Mode::Door { t: t2, door, shut };
                }
            }
            Mode::GameOver { t } => {
                let t2 = t + dt;
                self.mode = Mode::GameOver { t: t2 };
                if t2 > 2.0 && (inp.confirm || inp.interact || inp.pause) {
                    self.load_checkpoint();
                }
            }
        }

        self.update_particles(dt);
        self.update_ambient(dt);
        self.render();
    }

    fn update_play(&mut self, dt: f32, inp: &Input) {
        self.update_player(dt, inp);
        self.update_zombies(dt);
        self.update_camera(false);
        for b in &mut self.st.blood {
            b.2 = (b.2 + dt * 0.25).min(b.3);
        }
        if inp.pause && self.mode == Mode::Play {
            self.mode = Mode::Pause;
            self.sel = 0;
        }
        if inp.inventory && self.mode == Mode::Play && self.st.player.hp > 0 && self.st.player.grabbed <= 0.0 {
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
            let p = &mut self.st.player;
            p.reload_t -= dt;
            if p.reload_t <= 0.0 {
                let need = MAG_SIZE - self.st.inv.mag;
                let n = need.min(self.st.inv.reserve);
                self.st.inv.mag += n;
                self.st.inv.reserve -= n;
            }
        }

        if self.st.player.grabbed > 0.0 {
            let p = &mut self.st.player;
            p.grabbed -= dt;
            p.speed = 0.0;
            p.aiming = false;
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
                    if ray.y.abs() > 1e-3 {
                        let t = (1.0 - self.cam.eye.y) / ray.y;
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
                    let z = &self.st.zombies[zi];
                    if matches!(z.state, ZState::Dying | ZState::Dead) {
                        self.st.player.lock = self.find_target();
                    } else {
                        target_yaw = Some(((z.pos - self.st.player.pos).yaw(), 7.0));
                    }
                }
            }
            if let Some((ty, rate)) = target_yaw {
                let p = &mut self.st.player;
                p.yaw = approach_angle(p.yaw, ty, rate * dt);
            }
            if inp.fire && self.st.player.aim_t > 0.6 {
                self.shoot();
            }
        } else {
            let p = &mut self.st.player;
            p.aim_t = (p.aim_t - dt * 6.0).max(0.0);
            p.lock = None;
            let danger = p.hp < 34;
            let slow = if danger { 0.7 } else { 1.0 };
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
            self.play(Sfx::Step, vol, 0.0, pitch);
        }

        // Collisions.
        let room = &self.world.rooms[room_idx];
        let mut pos = room.collide(self.st.player.pos, PLAYER_R);
        for z in &self.st.zombies {
            if z.room != room_idx || matches!(z.state, ZState::Dead | ZState::Dying | ZState::Eating) {
                continue;
            }
            let d = pos.flat() - z.pos.flat();
            let l = d.len();
            let min = PLAYER_R + ZOMBIE_R;
            if l < min && l > 1e-4 {
                pos += d * ((min - l) / l);
            }
        }
        self.st.player.pos = room.collide(pos, PLAYER_R);

        if inp.reload {
            self.start_reload();
        }
        if inp.interact && !aiming {
            self.interact();
        }
    }

    fn find_target(&self) -> Option<usize> {
        let p = &self.st.player;
        let fwd = Vec3::from_yaw(p.yaw);
        let mut best: Option<(usize, f32)> = None;
        for (i, z) in self.st.zombies.iter().enumerate() {
            if z.room != self.st.room || matches!(z.state, ZState::Dying | ZState::Dead) {
                continue;
            }
            let d = (z.pos - p.pos).flat();
            let dist = d.len();
            if dist > 14.0 {
                continue;
            }
            let ang = d.norm().dot(fwd);
            if ang < 0.3 {
                continue;
            }
            let score = dist * (2.0 - ang);
            if best.map_or(true, |b| score < b.1) {
                best = Some((i, score));
            }
        }
        best.map(|b| b.0)
    }

    fn start_reload(&mut self) {
        let p = &self.st.player;
        if p.reload_t > 0.0 || self.st.inv.mag >= MAG_SIZE || self.st.inv.reserve == 0 {
            return;
        }
        self.st.player.reload_t = 1.2;
        self.play(Sfx::Reload, 0.6, 0.0, 1.0);
    }

    fn shoot(&mut self) {
        if self.st.player.fire_cd > 0.0 || self.st.player.reload_t > 0.0 {
            return;
        }
        if self.st.inv.mag == 0 {
            self.play(Sfx::Empty, 0.6, 0.0, 1.0);
            self.st.player.fire_cd = 0.35;
            self.start_reload();
            return;
        }
        self.st.inv.mag -= 1;
        self.st.player.fire_cd = 0.33;
        self.st.player.recoil = 1.0;
        self.muzzle = 0.07;
        let pitch = self.rng.range(0.92, 1.05);
        self.play(Sfx::Shot, 0.9, 0.0, pitch);

        let p = self.st.player.clone();
        let fwd = Vec3::from_yaw(p.yaw);
        let origin = p.pos + v3(0.0, 1.35, 0.0);
        let wall_t = self.room().ray_dist(origin, fwd, 40.0);
        let mut best: Option<(usize, f32)> = None;
        for (i, z) in self.st.zombies.iter().enumerate() {
            if z.room != self.st.room || matches!(z.state, ZState::Dying | ZState::Dead) {
                continue;
            }
            let rel = (z.pos - origin).flat();
            let t = rel.dot(fwd);
            if t < 0.0 {
                continue;
            }
            let perp2 = rel.dot(rel) - t * t;
            let r = 0.42;
            if perp2 < r * r {
                let th = t - (r * r - perp2).sqrt();
                if th < wall_t && best.map_or(true, |b| th < b.1) {
                    best = Some((i, th));
                }
            }
        }
        // Le coup de feu alerte toute la pièce.
        for z in &mut self.st.zombies {
            if z.room == self.st.room {
                z.alerted = true;
            }
        }
        if let Some((zi, th)) = best {
            let crit = self.rng.chance(0.1) && th < 4.0;
            let fall_back = self.rng.chance(0.6);
            let low = self.st.zombies[zi].state == ZState::Eating;
            let z = &mut self.st.zombies[zi];
            z.hp -= if crit { 10 } else { 1 };
            let hit_y = if low { 0.7 } else if crit { 1.65 } else { 1.3 };
            let hit = v3(origin.x, 0.0, origin.z) + fwd * th + v3(0.0, hit_y, 0.0);
            let zpos = z.pos;
            if z.hp <= 0 {
                z.state = ZState::Dying;
                z.t = 0.0;
                z.fall_back = fall_back;
                self.schedule(0.75, Sfx::Fall, 0.6, 0.0, 1.0);
            } else if z.state != ZState::Grab && z.state != ZState::Eating && z.state != ZState::Rising {
                z.state = ZState::Stagger;
                z.t = 0.0;
            }
            let pitch = self.rng.range(0.9, 1.1);
            self.play_at(Sfx::Hit, zpos, 0.7, pitch);
            let n = if crit { 40 } else { 14 };
            for _ in 0..n {
                let v = fwd * self.rng.range(0.5, 2.5) + v3(self.rng.range(-1.0, 1.0), self.rng.range(0.0, 2.0), self.rng.range(-1.0, 1.0));
                self.particles.push(Particle { pos: hit, vel: v, life: self.rng.range(0.4, 0.9), col: [0.5, 0.02, 0.02], size: 2 });
            }
        } else {
            let hit = origin + fwd * (wall_t - 0.05);
            for _ in 0..8 {
                let v = -fwd * self.rng.range(0.3, 1.5) + v3(self.rng.range(-0.8, 0.8), self.rng.range(0.0, 1.5), self.rng.range(-0.8, 0.8));
                self.particles.push(Particle { pos: hit, vel: v, life: self.rng.range(0.2, 0.5), col: [0.5, 0.45, 0.4], size: 1 });
            }
        }
    }

    // ------------------------------------------------------------ interactions

    fn interact(&mut self) {
        let p = self.st.player.clone();
        let fwd = Vec3::from_yaw(p.yaw);
        let front = p.pos + fwd * 0.6;
        // Objets.
        let mut best: Option<(usize, f32)> = None;
        for (i, it) in self.world.items.iter().enumerate() {
            if it.room != self.st.room || self.st.taken[i] {
                continue;
            }
            let d = front.dist_xz(it.pos).min(p.pos.dist_xz(it.pos) + 0.15);
            if d < 0.85 && best.map_or(true, |b| d < b.1) {
                best = Some((i, d));
            }
        }
        if let Some((i, _)) = best {
            self.pick_up(i);
            return;
        }
        // Portes.
        let room = self.room();
        let door = room
            .doors
            .iter()
            .enumerate()
            .find(|(_, d)| p.pos.dist_xz(d.pos) < 1.3 && fwd.dot(-d.normal) > 0.4)
            .map(|(i, d)| (i, d.clone()));
        // Points d'intérêt.
        let found = room.interacts.iter().find(|it| front.dist_xz(it.pos) < it.radius).cloned();
        if let Some((di, d)) = door {
            self.use_door(di, &d);
            return;
        }
        if let Some(it) = found {
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
                InteractKind::Note => {
                    self.message("Une note griffonnée à la hâte, tachée de sang :");
                    self.message("« Les chiens du marais hurlent chaque nuit depuis que le Docteur a ouvert la crypte. »");
                    self.message("« Ceux qui sont morts dans cette maison ne restent pas morts. Ne descendez pas au sous-sol. »");
                    self.message("FIN DE LA VERSION DE DÉMONSTRATION. Merci d'avoir joué !");
                }
            }
        }
    }

    fn pick_up(&mut self, i: usize) {
        let kind = self.world.items[i].kind;
        self.st.taken[i] = true;
        self.play(Sfx::Pickup, 0.5, 0.0, 1.0);
        match kind {
            ItemKind::Ammo(n) => {
                self.st.inv.reserve += n;
                self.message(format!("Vous avez trouvé : {}.", kind.name()));
            }
            ItemKind::Medkit => {
                self.st.inv.medkits += 1;
                self.message(format!("Vous avez trouvé : {}.", kind.name()));
                self.message("Utilisez-la depuis l'inventaire pour restaurer votre santé.");
            }
            ItemKind::Key(k) => {
                self.st.inv.keys.push(k);
                self.message(format!("Vous avez trouvé : {}.", kind.name()));
                self.message("Le blason gravé ressemble à celui de la porte nord du hall.");
            }
        }
    }

    fn use_door(&mut self, di: usize, d: &Door) {
        if let Some((lock, key)) = d.lock {
            if !self.st.unlocked[lock] {
                if let Some(k) = self.st.inv.keys.iter().position(|&k| k == key) {
                    self.st.inv.keys.remove(k);
                    self.st.unlocked[lock] = true;
                    self.play(Sfx::Reload, 0.5, 0.0, 0.7);
                    self.message("Vous utilisez la CLÉ AU BLASON. La serrure cède dans un déclic.");
                    self.message("La clé n'est plus utile. Vous la jetez.");
                } else {
                    self.play(Sfx::Locked, 0.6, 0.0, 1.0);
                    self.message("La porte est verrouillée. Un blason est gravé sur la serrure.");
                }
                return;
            }
        }
        let pitch = self.rng.range(0.9, 1.1);
        self.play(Sfx::Creak, 0.6, 0.0, pitch);
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
                Slot::Gun => {
                    if self.st.inv.mag < MAG_SIZE && self.st.inv.reserve > 0 {
                        let n = (MAG_SIZE - self.st.inv.mag).min(self.st.inv.reserve);
                        self.st.inv.mag += n;
                        self.st.inv.reserve -= n;
                        self.play(Sfx::Reload, 0.6, 0.0, 1.0);
                    } else {
                        self.play(Sfx::Locked, 0.4, 0.0, 1.3);
                    }
                }
                Slot::Medkit => {
                    if self.st.player.hp >= 100 {
                        self.prev_mode = Mode::Inventory;
                        self.message("Vous êtes en pleine forme.");
                        self.after_msg = Some(Mode::Inventory);
                    } else {
                        self.st.player.hp = 100;
                        self.st.inv.medkits -= 1;
                        self.play(Sfx::Pickup, 0.5, 0.0, 0.8);
                        self.message("Vous vous soignez. Vous vous sentez beaucoup mieux.");
                        self.after_msg = Some(Mode::Inventory);
                        self.sel = 0;
                    }
                }
                Slot::Key => {
                    self.message("Une clé ornée d'un blason. Elle s'utilise automatiquement sur la bonne porte.");
                    self.after_msg = Some(Mode::Inventory);
                }
            }
        }
    }

    fn inv_slots(&self) -> Vec<(Slot, String)> {
        let mut v = vec![(Slot::Gun, format!("PISTOLET  {:>2}/{}", self.st.inv.mag, self.st.inv.reserve))];
        if self.st.inv.medkits > 0 {
            v.push((Slot::Medkit, format!("TROUSSE SOINS x{}", self.st.inv.medkits)));
        }
        for _ in &self.st.inv.keys {
            v.push((Slot::Key, "CLÉ AU BLASON".to_string()));
        }
        v
    }

    // ------------------------------------------------------------ zombies

    fn update_zombies(&mut self, dt: f32) {
        let room_idx = self.st.room;
        let n = self.st.zombies.len();
        for i in 0..n {
            if self.st.zombies[i].room != room_idx {
                continue;
            }
            let ppos = self.st.player.pos;
            let player_alive = self.st.player.hp > 0;
            let player_free = self.st.player.grabbed <= 0.0;
            let mut sounds: Vec<(Sfx, f32, f32)> = Vec::new();
            {
                let z = &mut self.st.zombies[i];
                z.attack_cd = (z.attack_cd - dt).max(0.0);
                z.t += dt;
                let to_p = ppos - z.pos;
                let dist = to_p.flat().len();
                match z.state {
                    ZState::Eating => {
                        if dist < 4.5 || z.alerted {
                            z.state = ZState::Rising;
                            z.t = 0.0;
                            sounds.push((Sfx::Groan2, 0.8, 0.9));
                        }
                    }
                    ZState::Rising => {
                        z.yaw = approach_angle(z.yaw, to_p.yaw(), 1.5 * dt);
                        if z.t > 1.3 {
                            z.state = ZState::Chase;
                        }
                    }
                    ZState::Idle => {
                        if dist < 8.0 || z.alerted {
                            z.state = ZState::Chase;
                            sounds.push((Sfx::Groan, 0.7, 1.0));
                        }
                    }
                    ZState::Chase => {
                        z.yaw = approach_angle(z.yaw, to_p.yaw(), 1.9 * dt);
                        z.phase += dt * 3.0;
                        let speed = 0.5 + (z.variant % 3) as f32 * 0.08;
                        if player_alive && dist > 0.6 {
                            z.pos += Vec3::from_yaw(z.yaw) * (speed * (0.55 + 0.45 * z.phase.sin().abs()) * dt);
                        }
                        z.groan_t -= dt;
                        if z.groan_t <= 0.0 {
                            z.groan_t = 4.0 + (z.phase * 7.3).sin().abs() * 5.0;
                            sounds.push((if z.variant % 2 == 0 { Sfx::Groan } else { Sfx::Groan2 }, 0.6, 0.9 + (z.variant as f32) * 0.07));
                        }
                        let facing = wrap_angle(to_p.yaw() - z.yaw).abs() < 0.7;
                        if dist < 0.9 && facing && z.attack_cd <= 0.0 && player_free && player_alive {
                            z.state = ZState::Grab;
                            z.t = 0.0;
                            z.bitten = false;
                        }
                    }
                    ZState::Grab => {}
                    ZState::Stagger => {
                        if z.t < 0.2 {
                            z.pos -= Vec3::from_yaw(z.yaw) * (0.8 * dt);
                        }
                        if z.t > 0.5 {
                            z.state = ZState::Chase;
                        }
                    }
                    ZState::Dying => {
                        if z.t > 1.2 {
                            z.state = ZState::Dead;
                        }
                    }
                    ZState::Dead => {}
                }
            }

            // Saisie de la joueuse.
            if self.st.zombies[i].state == ZState::Grab {
                let zt = self.st.zombies[i].t;
                let zpos = self.st.zombies[i].pos;
                if zt < dt * 1.5 {
                    self.st.player.grabbed = 1.7;
                    self.st.player.yaw = (zpos - ppos).yaw();
                    self.st.player.aiming = false;
                    sounds.push((Sfx::Groan2, 0.9, 1.1));
                }
                if zt > 0.65 && !self.st.zombies[i].bitten {
                    self.st.zombies[i].bitten = true;
                    let dmg = 18 + (self.rng.f() * 8.0) as i32;
                    self.st.player.hp -= dmg;
                    self.st.player.hurt_t = 1.0;
                    self.hurt_flash = 0.6;
                    self.play(Sfx::Bite, 0.8, 0.0, 1.0);
                    let pitch = self.rng.range(0.95, 1.1);
                    self.play(Sfx::Hurt, 0.7, 0.0, pitch);
                    let neck = ppos + v3(0.0, 1.5, 0.0);
                    for _ in 0..20 {
                        let v = v3(self.rng.range(-1.0, 1.0), self.rng.range(0.5, 2.0), self.rng.range(-1.0, 1.0));
                        self.particles.push(Particle { pos: neck, vel: v, life: 0.8, col: [0.5, 0.02, 0.02], size: 2 });
                    }
                }
                if zt > 1.7 {
                    let z = &mut self.st.zombies[i];
                    z.state = ZState::Stagger;
                    z.t = 0.0;
                    z.attack_cd = 2.8;
                    self.st.player.grabbed = 0.0;
                    if self.st.player.hp > 0 {
                        let away = (ppos - zpos).flat().norm();
                        let np = ppos + away * 0.6;
                        self.st.player.pos = self.world.rooms[room_idx].collide(np, PLAYER_R);
                    }
                }
            }

            // Mort : flaque de sang.
            if self.st.zombies[i].state == ZState::Dead && !self.st.blood.iter().any(|b| b.0 == room_idx && b.1.dist_xz(self.st.zombies[i].pos) < 0.05) {
                let z = &self.st.zombies[i];
                let off = Vec3::from_yaw(z.yaw) * if z.fall_back { -0.8 } else { 0.8 };
                let bp = z.pos + off;
                self.st.blood.push((room_idx, bp, 0.1, 0.8));
                // Marqueur pour ne pas recréer la flaque.
                self.st.blood.push((room_idx, z.pos, 0.0, 0.0));
            }

            // Collisions.
            let room = &self.world.rooms[room_idx];
            let mut pos = room.collide(self.st.zombies[i].pos, ZOMBIE_R);
            for j in 0..n {
                if j == i {
                    continue;
                }
                let o = &self.st.zombies[j];
                if o.room != room_idx || matches!(o.state, ZState::Dead | ZState::Dying) {
                    continue;
                }
                let d = pos.flat() - o.pos.flat();
                let l = d.len();
                if l < ZOMBIE_R * 2.0 && l > 1e-4 {
                    pos += d * ((ZOMBIE_R * 2.0 - l) / l * 0.5);
                }
            }
            if !matches!(self.st.zombies[i].state, ZState::Dead | ZState::Dying | ZState::Eating) {
                self.st.zombies[i].pos = room.collide(pos, ZOMBIE_R);
            }
            let zpos = self.st.zombies[i].pos;
            for (s, v, p) in sounds {
                self.play_at(s, zpos, v, p);
            }
        }
    }

    // ------------------------------------------------------------ ambiance

    fn update_particles(&mut self, dt: f32) {
        for p in &mut self.particles {
            p.life -= dt;
            if p.pos.y > 0.02 {
                p.vel.y -= 9.0 * dt;
                p.pos += p.vel * dt;
            } else {
                p.pos.y = 0.015;
            }
        }
        self.particles.retain(|p| p.life > 0.0);
        for e in &mut self.eyes {
            e.1 -= dt;
        }
        self.eyes.retain(|e| e.1 > 0.0);
    }

    fn update_ambient(&mut self, dt: f32) {
        let windows = !self.room().windows.is_empty();
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
        if let Ok(mut m) = self.mixer.lock() {
            m.set_loop(1, Sfx::Wind, if windows { 0.22 } else { 0.12 } * amb, 0.0);
            m.set_loop(2, Sfx::Drone, if title { 0.12 } else { 0.2 } * amb, 0.0);
            m.set_loop(3, Sfx::Fire, fire_vol * sfx, fire_pan);
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

        // Chiens au loin.
        self.dog_t -= dt;
        if self.dog_t <= 0.0 {
            self.dog_t = self.rng.range(6.0, 15.0);
            let a = self.rng.range(0.0, TAU);
            let dir = v3(a.sin(), 0.0, a.cos());
            let pan = dir.dot(self.cam.right.flat().norm()) * 0.9;
            let muffle = if windows { 1.0 } else { 0.55 };
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
            self.frog_t = self.rng.range(0.6, 3.0);
            let vol = self.rng.range(0.04, 0.12) * if windows { 1.0 } else { 0.5 };
            let pan = self.rng.range(-0.9, 0.9);
            let pitch = self.rng.range(0.75, 1.3);
            self.play_amb(Sfx::Frog, vol, pan, pitch);
        }
    }

    // ------------------------------------------------------------ rendu

    fn lighting(&self) -> Lighting {
        let room = self.room();
        let t = self.time;
        let mut lights: Vec<Light> = room
            .lights
            .iter()
            .enumerate()
            .map(|(i, l)| {
                let k = if l.flicker > 0.0 {
                    let f = (t * 13.0 + i as f32 * 3.1).sin() * 0.5 + (t * 23.0 + i as f32).sin() * 0.3 + (t * 7.0).sin() * 0.2;
                    1.0 + f * l.flicker
                } else {
                    1.0
                };
                let k = k * LIGHT_GAIN;
                Light { pos: l.light.pos, color: [l.light.color[0] * k, l.light.color[1] * k, l.light.color[2] * k], radius: l.light.radius * 1.3 }
            })
            .collect();
        if self.muzzle > 0.0 {
            let p = &self.st.player;
            lights.push(Light { pos: p.pos + Vec3::from_yaw(p.yaw) * 0.7 + v3(0.0, 1.4, 0.0), color: [2.2, 1.6, 0.8], radius: 8.0 });
        }
        let a = room.ambient;
        Lighting { ambient: [a[0] * AMBIENT_GAIN, a[1] * AMBIENT_GAIN, a[2] * AMBIENT_GAIN], lights, sun: None }
    }

    fn player_pose(&self) -> Pose {
        let p = &self.st.player;
        let t = self.time;
        if p.hp <= 0 {
            let k = smoothstep(0.0, 1.0, p.dead_t);
            let mut lying = Pose::lying(0.0);
            lying.root_pitch = -1.45;
            return Pose { root_y: 0.0, ..Pose::idle(t) }.lerp(&lying, k);
        }
        if p.grabbed > 0.0 {
            let s = (t * 18.0).sin() * 0.08;
            return Pose {
                torso_pitch: -0.3 + s,
                head_pitch: -0.3,
                head_yaw: 0.5,
                arm_fwd: [1.2 + s, 1.0 - s],
                arm_out: [-0.2, -0.2],
                elbow: [1.2, 1.4],
                hip: [0.2, -0.1],
                knee: [0.3, 0.2],
                pony: 0.4,
                ..Default::default()
            };
        }
        let idle = Pose::idle(t);
        let walk = Pose::walk(p.phase, p.run_amt);
        let mut pose = idle.lerp(&walk, p.move_amt);
        if p.hp < 34 {
            pose.arm_fwd[0] = 0.5;
            pose.elbow[0] = 1.8;
            pose.arm_out[0] = -0.25;
            pose.torso_pitch += 0.12;
            pose.torso_roll = 0.08;
        }
        if p.aim_t > 0.0 {
            let aim = Pose::aim(p.recoil, 0.0);
            pose = pose.lerp(&aim, smoothstep(0.0, 1.0, p.aim_t));
        }
        if p.reload_t > 0.0 {
            pose.arm_fwd[1] = 0.9;
            pose.elbow[1] = 1.4;
            pose.arm_fwd[0] = 0.8 + (p.reload_t * 6.0).sin() * 0.2;
            pose.elbow[0] = 1.5;
            pose.arm_out = [-0.3, -0.2];
            pose.gun = true;
            pose.head_pitch = 0.4;
        }
        pose.torso_pitch -= p.hurt_t * 0.3;
        pose
    }

    fn zombie_pose(&self, z: &Zombie) -> Pose {
        let t = self.time + z.variant as f32 * 2.0;
        match z.state {
            ZState::Eating => Pose::eating(t),
            ZState::Rising => Pose::eating(t).lerp(&Pose::shamble(z.phase, t), smoothstep(0.0, 1.3, z.t)),
            ZState::Idle => {
                let mut p = Pose::shamble(0.0, t);
                p.arm_fwd = [0.2, 0.3];
                p.head_roll = 0.5 + 0.1 * (t * 0.6).sin();
                p
            }
            ZState::Chase => Pose::shamble(z.phase, t),
            ZState::Grab => {
                let mut p = Pose::shamble(0.0, t);
                p.arm_fwd = [1.5, 1.5];
                p.arm_out = [-0.35, -0.35];
                p.elbow = [0.9, 0.9];
                let bite = smoothstep(0.4, 0.7, z.t) * (1.0 - smoothstep(1.2, 1.6, z.t));
                p.torso_pitch = 0.2 + bite * 0.35 + (z.t * 20.0).sin() * 0.03 * bite;
                p.head_pitch = 0.2 + bite * 0.4;
                p.head_roll = 0.2;
                p
            }
            ZState::Stagger => {
                let mut p = Pose::shamble(z.phase, t);
                let k = (1.0 - z.t / 0.5).max(0.0);
                p.torso_pitch -= 0.6 * k;
                p.head_pitch -= 0.5 * k;
                p.arm_fwd = [p.arm_fwd[0] - 0.8 * k, p.arm_fwd[1] - 0.6 * k];
                p
            }
            ZState::Dying | ZState::Dead => {
                let k = if z.state == ZState::Dead { 1.0 } else { smoothstep(0.0, 1.1, z.t) };
                let mut end = Pose::lying(z.variant as f32 * 0.3);
                if !z.fall_back {
                    end.root_pitch = 1.5;
                    end.head_yaw = 0.8;
                    end.arm_fwd = [2.6, 2.9];
                }
                let mut start = Pose::shamble(z.phase, t);
                start.knee = [0.3, 0.3];
                start.lerp(&end, k)
            }
        }
    }

    fn render(&mut self) {
        if let Mode::Door { t, .. } = self.mode {
            self.render_door(t);
        } else {
            let title = matches!(self.mode, Mode::Title) || matches!(self.mode, Mode::Options { title: true } | Mode::Controls { title: true, .. });
            self.render_scene(title);
            let fade = match self.mode {
                Mode::GameOver { t } => 1.0 - (t * 0.8).min(0.75),
                _ => 1.0 - self.fade,
            };
            let tint = [self.hurt_flash * 0.5, 0.0, 0.0];
            self.raster.post(fade, tint);
            self.render_ui();
        }
        if self.opts.show_fps {
            let s = format!("{:.0} FPS", self.fps);
            self.raster.text(W as i32 - 4 - Raster::text_width(&s, 1), 4, &s, [0.9, 0.9, 0.3], 1);
        }
    }

    fn render_scene(&mut self, title: bool) {
        let light = self.lighting();
        let room_idx = self.st.room;
        let t = self.time;
        let pose = self.player_pose();
        let zposes: Vec<(usize, Pose)> = self
            .st
            .zombies
            .iter()
            .enumerate()
            .filter(|(_, z)| z.room == room_idx)
            .map(|(i, z)| (i, self.zombie_pose(z)))
            .collect();
        // Les personnages reçoivent un peu plus de lumière pour rester lisibles.
        let mut char_light = light.clone();
        for c in &mut char_light.ambient {
            *c += 0.07;
        }
        let room = &self.world.rooms[room_idx];
        let r = &mut self.raster;
        r.clear(0);
        r.cam = self.cam;
        r.fog = room.fog;
        r.draw_mesh(&room.mesh, &Mat4::IDENTITY, Some(&light));

        // Flaques de sang.
        for (ri, pos, rad, _) in &self.st.blood {
            if *ri != room_idx || *rad < 0.01 {
                continue;
            }
            let segs = 10;
            for k in 0..segs {
                let a0 = k as f32 / segs as f32 * TAU;
                let a1 = (k + 1) as f32 / segs as f32 * TAU;
                let j0 = 0.75 + 0.35 * hash2(k as i32, (pos.x * 10.0) as i32, 3);
                let j1 = 0.75 + 0.35 * hash2(((k + 1) % segs) as i32, (pos.x * 10.0) as i32, 3);
                let c = *pos + v3(0.0, 0.012, 0.0);
                let p0 = c + v3(a0.sin(), 0.0, a0.cos()) * (rad * j0);
                let p1 = c + v3(a1.sin(), 0.0, a1.cos()) * (rad * j1);
                let col = [0.16, 0.0, 0.0];
                r.draw_tri_world([c, p1, p0], [[0.0; 2]; 3], [col; 3], -1, Blend::Decal);
            }
        }

        // Cadavres du décor.
        for (cp, cy) in &room.corpses {
            let mut pz = Pose::lying(0.4);
            pz.arm_out = [1.2, 0.3];
            actors::draw_body(r, &self.corpse_body, &pz, *cp, *cy, &char_light);
        }

        // Objets.
        for (i, it) in self.world.items.iter().enumerate() {
            if it.room != room_idx || self.st.taken[i] {
                continue;
            }
            let m = match it.kind {
                ItemKind::Ammo(_) => &self.items_mesh[0],
                ItemKind::Medkit => &self.items_mesh[1],
                ItemKind::Key(_) => &self.items_mesh[2],
            };
            let xf = Mat4::translate(it.pos) * Mat4::rot_y(i as f32 * 1.7);
            r.draw_mesh(m, &xf, Some(&char_light));
        }

        // Zombies.
        for (i, zp) in &zposes {
            let z = &self.st.zombies[*i];
            let body = &self.zbodies[z.variant as usize % self.zbodies.len()];
            actors::draw_body(r, body, zp, z.pos, z.yaw, &char_light);
        }

        // Héroïne.
        let p = &self.st.player;
        let hero_pos = if self.debug_hide_player { p.pos - v3(0.0, 50.0, 0.0) } else { p.pos };
        let sk = actors::draw_body(r, &self.hero, &pose, hero_pos, p.yaw, &char_light);

        // Ombres portées.
        let shadow = [0.45, 0.45, 0.45];
        let mut blobs: Vec<(Vec3, f32)> = vec![(p.pos, 0.38)];
        for (i, _) in &zposes {
            let z = &self.st.zombies[*i];
            if z.state != ZState::Dead {
                blobs.push((z.pos, 0.4));
            }
        }
        for (c, rad) in blobs {
            let c = c + v3(0.0, 0.016, 0.0);
            for k in 0..8 {
                let a0 = k as f32 / 8.0 * TAU;
                let a1 = (k + 1) as f32 / 8.0 * TAU;
                let p0 = c + v3(a0.sin(), 0.0, a0.cos()) * rad;
                let p1 = c + v3(a1.sin(), 0.0, a1.cos()) * rad;
                r.draw_tri_world([c, p1, p0], [[0.0; 2]; 3], [shadow; 3], -1, Blend::Mul);
            }
        }

        // Paysage extérieur par les fenêtres.
        let mut sc: Option<(f32, f32, f32, f32)> = None;
        for w in &room.windows {
            let mut behind = 0;
            let mut rect = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
            for c in w {
                match r.cam.project(*c) {
                    Some((x, y, _)) => {
                        rect = (rect.0.min(x), rect.1.min(y), rect.2.max(x), rect.3.max(y));
                    }
                    None => behind += 1,
                }
            }
            if behind == 4 {
                continue;
            }
            if behind > 0 {
                rect = (0.0, 0.0, W as f32, H as f32);
            }
            if rect.2 < 0.0 || rect.3 < 0.0 || rect.0 > W as f32 || rect.1 > H as f32 {
                continue;
            }
            sc = Some(match sc {
                None => rect,
                Some(s) => (s.0.min(rect.0), s.1.min(rect.1), s.2.max(rect.2), s.3.max(rect.3)),
            });
        }
        if let Some(s) = sc {
            r.scissor = (
                (s.0.floor() as i32 - 1).max(0),
                (s.1.floor() as i32 - 1).max(0),
                (s.2.ceil() as i32 + 1).min(W as i32),
                (s.3.ceil() as i32 + 1).min(H as i32),
            );
            r.fog = outdoor::GROUND_FOG;
            r.draw_mesh(&self.outdoor.ground, &Mat4::IDENTITY, None);
            for (i, f) in self.outdoor.fireflies.iter().enumerate() {
                let ph = t * 0.7 + i as f32 * 2.3;
                let blink = ((t * 1.3 + i as f32 * 0.77).sin() * 0.5 + 0.5).powi(3);
                if blink < 0.2 {
                    continue;
                }
                let pos = *f + v3(ph.sin() * 0.8, (ph * 1.3).sin() * 0.3, (ph * 0.8).cos() * 0.8);
                r.point(pos, 1, [0.5 * blink, 0.7 * blink, 0.2 * blink], Blend::Add);
            }
            for (e, life) in &self.eyes {
                let k = (life / 4.0 * PI).sin().max(0.0);
                let blink = if (t * 2.0 + e.x).sin() > 0.95 { 0.0 } else { 1.0 };
                let side = r.cam.right * 0.12;
                let c = [0.9 * k * blink, 0.7 * k * blink, 0.1 * k * blink];
                r.point(*e + side + v3(0.0, 0.6, 0.0), 1, c, Blend::Add);
                r.point(*e - side + v3(0.0, 0.6, 0.0), 1, c, Blend::Add);
            }
            r.fog = outdoor::MOUNTAIN_FOG;
            r.draw_mesh(&self.outdoor.mountains, &Mat4::IDENTITY, None);
            let od = &self.outdoor;
            r.fill_sky(&|d, _, _| od.sky(d, t));
            r.reset_scissor();
            r.fog = room.fog;
        }

        // Transparences de la pièce (vitres...).
        r.draw_mesh(&room.trans, &Mat4::IDENTITY, Some(&light));

        // Flammes.
        for (k, (pos, size)) in room.flames.iter().enumerate() {
            let fl = 1.0 + 0.2 * (t * 17.0 + k as f32 * 1.9).sin();
            let s = size * fl;
            let c = [0.9, 0.85, 0.8];
            r.billboard(*pos + v3(0.0, s * 0.4, 0.0), s, c, T_FLAME, Blend::Add);
            if *size > 0.1 {
                r.billboard(*pos + v3(0.03 * (t * 9.0).sin(), s * 0.9, 0.0), s * 0.6, c, T_FLAME, Blend::Add);
            }
        }

        // Éclats sur les objets à ramasser.
        if !title {
            for (i, it) in self.world.items.iter().enumerate() {
                if it.room != room_idx || self.st.taken[i] {
                    continue;
                }
                let ph = (t * 1.1 + i as f32 * 0.9) % 2.2;
                if ph < 0.35 {
                    let k = (ph / 0.35 * PI).sin();
                    r.billboard(it.pos + v3(0.0, 0.12, 0.0), 0.12 * k, [k, k, k * 0.9], T_GLINT, Blend::Add);
                }
            }
        }

        // Flash du tir.
        if self.muzzle > 0.0 {
            let tip = sk.fore[1].point(v3(0.0, -0.56, 0.03));
            r.billboard(tip, 0.16, [1.0, 0.8, 0.4], T_FLAME, Blend::Add);
            r.billboard(tip, 0.09, [1.0, 1.0, 0.8], T_GLINT, Blend::Add);
        }

        for p in &self.particles {
            r.point(p.pos, p.size, p.col, Blend::Opaque);
        }
    }

    fn render_door(&mut self, t: f32) {
        let r = &mut self.raster;
        r.clear(0);
        let dolly = smoothstep(0.5, 2.3, t) * 2.2;
        r.cam = Camera::look_at(v3(0.0, 1.5, -3.0 + dolly), v3(0.0, 1.3, 2.0), 60.0);
        r.fog = crate::raster::Fog { start: 1.0, end: 5.5, color: [0.0; 3] };
        let light = Lighting {
            ambient: [0.1, 0.09, 0.09],
            lights: vec![Light { pos: v3(0.4, 2.2, -1.6 + dolly), color: [1.6, 1.2, 0.8], radius: 6.0 }],
            sun: None,
        };
        r.draw_mesh(&self.door_scene, &Mat4::IDENTITY, Some(&light));
        let open = smoothstep(0.35, 1.5, t) * 1.75;
        let xf = Mat4::translate(v3(-0.6, 0.0, 0.0)) * Mat4::rot_y(-open);
        r.draw_mesh(&self.door_leaf, &xf, Some(&light));
        let fade_in = smoothstep(0.0, 0.3, t);
        let fade_out = 1.0 - smoothstep(1.9, 2.3, t);
        let k = fade_in * fade_out;
        r.post(k, [0.0; 3]);
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

    /// Diagnostic de cadrage : pour chaque pièce, vérifie que la joueuse, les
    /// portes et les objets sont visibles par la caméra active quand on s'en
    /// approche, et compte les pixels non dessinés (trous) hors fenêtres.
    pub fn debug_visibility(&mut self) -> Vec<String> {
        let mut report = Vec::new();
        self.debug_setup(0, v3(0.0, 0.0, 2.4), 0.0, false);
        let saved_zombies: Vec<usize> = self.st.zombies.iter().map(|z| z.room).collect();
        // On retire les zombies pour ne mesurer que le décor.
        for z in &mut self.st.zombies {
            z.room = usize::MAX;
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
            for (name, stand, point) in targets {
                let yaw = (point - stand).flat().yaw();
                self.debug_setup(ri, stand, if name == "#" { 0.0 } else { yaw }, false);
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
                if !full {
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
                                    eprintln!("trou room {ri} cam {} stand ({:.1},{:.1}) pixel ({px},{py}) d={}", self.cam_idx, stand.x, stand.z, self.raster.depth[py * W + px]);
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
                if name == "#" && problem.is_some() && std::env::var("HOLES").is_ok() {
                    if let Some((sx, sy, z)) = r.cam.project(point) {
                        eprintln!("cache {:?} cam {} -> ({sx:.0},{sy:.0}) z={z:.2} depth={:.4} attendu={:.4}", (stand.x, stand.z), self.cam_idx, r.depth[sy as usize * W + sx as usize], 1.0 / z);
                    }
                }
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
        for (z, r) in self.st.zombies.iter_mut().zip(saved_zombies) {
            z.room = r;
        }
        report
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Slot {
    Gun,
    Medkit,
    Key,
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
mod tests {
    use super::*;
    use crate::audio::Mixer;

    const DT: f32 = 1.0 / 60.0;

    fn press() -> Input {
        Input { confirm: true, interact: true, ..Default::default() }
    }

    fn run(g: &mut Game, inp: Input, secs: f32) {
        for _ in 0..(secs / DT) as usize {
            g.frame(DT, &inp);
        }
    }

    fn start() -> Game {
        let mut g = Game::new(Mixer::shared(), Box::new(crate::options::MemStorage::default()), true);
        g.frame(DT, &press());
        assert_eq!(g.mode, Mode::Message);
        g.frame(DT, &press());
        assert_eq!(g.mode, Mode::Play);
        g
    }

    fn dismiss(g: &mut Game) {
        while g.mode == Mode::Message {
            g.frame(DT, &press());
        }
    }

    #[test]
    fn movement_is_camera_relative() {
        let mut g = start();
        // La caméra de départ regarde vers le sud : "haut" = vers -z.
        let z0 = g.st.player.pos.z;
        run(&mut g, Input { mv: (0.0, 1.0), ..Default::default() }, 0.8);
        assert!(g.st.player.pos.z < z0 - 0.5, "z = {}", g.st.player.pos.z);
        // "Droite" à l'écran = ouest (-x) quand on regarde vers le sud.
        let x0 = g.st.player.pos.x;
        run(&mut g, Input { mv: (1.0, 0.0), ..Default::default() }, 0.8);
        assert!(g.st.player.pos.x < x0 - 0.5, "x = {}", g.st.player.pos.x);
    }

    #[test]
    fn door_leads_to_next_room() {
        let mut g = start();
        g.st.player.pos = v3(-5.2, 0.0, 6.2);
        g.st.player.yaw = -PI / 2.0;
        g.frame(DT, &press());
        assert!(matches!(g.mode, Mode::Door { .. }));
        run(&mut g, Input::default(), 2.6);
        assert_eq!(g.st.room, ROOM_DINING);
        assert_eq!(g.mode, Mode::Play);
    }

    #[test]
    fn locked_door_needs_key() {
        let mut g = start();
        g.st.player.pos = v3(0.0, 0.0, 13.2);
        g.st.player.yaw = 0.0;
        g.frame(DT, &press());
        assert_eq!(g.mode, Mode::Message);
        dismiss(&mut g);
        assert_eq!(g.st.room, ROOM_HALL);
        // On ramasse la clé dans le bureau.
        g.enter_room(ROOM_STUDY, v3(10.6, 0.0, 24.1), 0.0);
        g.frame(DT, &press());
        assert!(g.st.inv.keys.contains(&KEY_EMBLEM));
        dismiss(&mut g);
        g.enter_room(ROOM_HALL, v3(0.0, 0.0, 13.2), 0.0);
        g.frame(DT, &press());
        dismiss(&mut g);
        assert!(g.st.unlocked[0]);
        assert!(g.st.inv.keys.is_empty());
        g.frame(DT, &press());
        assert!(matches!(g.mode, Mode::Door { .. }));
        run(&mut g, Input::default(), 2.6);
        assert_eq!(g.st.room, ROOM_LIBRARY);
    }

    #[test]
    fn shooting_kills_zombie() {
        let mut g = start();
        g.enter_room(ROOM_CORRIDOR, v3(7.5, 0.0, 9.0), 0.0);
        let zi = g.st.zombies.iter().position(|z| z.room == ROOM_CORRIDOR).unwrap();
        let mag0 = g.st.inv.mag;
        let aim = Input { aim: true, ..Default::default() };
        let fire = Input { aim: true, fire: true, ..Default::default() };
        for _ in 0..12 {
            run(&mut g, aim, 0.4);
            g.frame(DT, &fire);
            if g.st.zombies[zi].hp <= 0 {
                break;
            }
        }
        run(&mut g, Input::default(), 1.5);
        assert_eq!(g.st.zombies[zi].state, ZState::Dead);
        assert!(g.st.inv.mag < mag0);
    }

    #[test]
    fn ammo_pickup_and_reload() {
        let mut g = start();
        g.st.inv.mag = 3;
        g.enter_room(ROOM_CORRIDOR, v3(7.5, 0.0, 0.1), PI);
        g.frame(DT, &press());
        dismiss(&mut g);
        assert_eq!(g.st.inv.reserve, 10);
        g.frame(DT, &Input { reload: true, ..Default::default() });
        run(&mut g, Input::default(), 1.5);
        assert_eq!(g.st.inv.mag, 13);
        assert_eq!(g.st.inv.reserve, 0);
    }

    #[test]
    fn zombie_bites_and_medkit_heals() {
        let mut g = start();
        g.enter_room(ROOM_CORRIDOR, v3(7.6, 0.0, 14.4), 0.0);
        run(&mut g, Input::default(), 3.0);
        assert!(g.st.player.hp < 100, "hp = {}", g.st.player.hp);
        g.st.inv.medkits = 1;
        g.mode = Mode::Play;
        g.frame(DT, &Input { inventory: true, ..Default::default() });
        assert_eq!(g.mode, Mode::Inventory);
        g.frame(DT, &Input { down: true, ..Default::default() });
        g.frame(DT, &press());
        assert_eq!(g.st.player.hp, 100);
        assert_eq!(g.st.inv.medkits, 0);
    }

    /// Stockage partagé entre deux instances de jeu (simule un redémarrage).
    #[derive(Clone, Default)]
    struct Shared(std::rc::Rc<std::cell::RefCell<std::collections::HashMap<String, String>>>);
    impl Storage for Shared {
        fn load(&self, key: &str) -> Option<String> {
            self.0.borrow().get(key).cloned()
        }
        fn save(&mut self, key: &str, data: &str) {
            self.0.borrow_mut().insert(key.into(), data.into());
        }
    }

    #[test]
    fn save_and_options_persist_across_restart() {
        let disk = Shared::default();
        let mut g = Game::new(Mixer::shared(), Box::new(disk.clone()), true);
        g.frame(DT, &press());
        dismiss(&mut g);
        // État modifié puis sauvegarde à la machine à écrire.
        g.st.inv.reserve = 27;
        g.st.taken[0] = true;
        g.st.zombies[0].state = ZState::Dead;
        g.st.player.pos = v3(-4.6, 0.0, 10.5);
        g.st.player.yaw = -PI / 2.0;
        g.frame(DT, &press());
        dismiss(&mut g);
        g.frame(DT, &press());
        dismiss(&mut g);
        // Option modifiée depuis le menu pause.
        g.frame(DT, &Input { pause: true, ..Default::default() });
        assert_eq!(g.mode, Mode::Pause);
        g.sel = 2;
        g.frame(DT, &press());
        assert_eq!(g.mode, Mode::Options { title: false });
        let row = g.option_rows().iter().position(|&o| o == menu_opt_fps()).unwrap();
        g.sel = row;
        g.frame(DT, &Input { right: true, ..Default::default() });
        assert!(g.opts.show_fps);
        g.sel = 0;
        g.frame(DT, &Input { right: true, ..Default::default() });
        assert!(!g.opts.fullscreen);
        let reqs = g.take_requests();
        assert!(reqs.contains(&Request::Fullscreen(false)));
        drop(g);

        // « Redémarrage » : options et sauvegarde rechargées.
        let mut g = Game::new(Mixer::shared(), Box::new(disk.clone()), true);
        assert!(g.opts.show_fps && !g.opts.fullscreen);
        assert!(g.save_exists);
        assert_eq!(g.mode, Mode::Title);
        g.frame(DT, &press()); // CONTINUER
        assert_eq!(g.mode, Mode::Play);
        assert_eq!(g.st.inv.reserve, 27);
        assert!(g.st.taken[0]);
        assert_eq!(g.st.zombies[0].state, ZState::Dead);
        assert!(g.st.player.pos.dist_xz(v3(-4.6, 0.0, 10.5)) < 0.01);
    }

    fn menu_opt_fps() -> menu::OptPub {
        menu::OptPub::Fps
    }

    #[test]
    fn classic_controls_rotate_and_advance() {
        let mut g = start();
        g.opts.classic = true;
        let yaw0 = g.st.player.yaw;
        run(&mut g, Input { mv: (1.0, 0.0), ..Default::default() }, 0.5);
        assert!(wrap_angle(g.st.player.yaw - yaw0) > 1.0, "pivot vers la droite");
        let p0 = g.st.player.pos;
        let fwd = Vec3::from_yaw(g.st.player.yaw);
        run(&mut g, Input { mv: (0.0, 1.0), ..Default::default() }, 0.5);
        assert!((g.st.player.pos - p0).dot(fwd) > 0.4, "avance dans la direction du regard");
        // Demi-tour rapide : bas + course.
        let y1 = g.st.player.yaw;
        g.frame(DT, &Input { mv: (0.0, -1.0), ..Default::default() });
        run(&mut g, Input { mv: (0.0, -1.0), run: true, ..Default::default() }, 0.5);
        assert!(wrap_angle(g.st.player.yaw - y1).abs() > 2.8);
    }

    #[test]
    fn death_reloads_checkpoint() {
        let mut g = start();
        // Sauvegarde à la machine à écrire.
        g.st.player.pos = v3(-4.6, 0.0, 10.5);
        g.st.player.yaw = -PI / 2.0;
        g.frame(DT, &press());
        dismiss(&mut g);
        assert_eq!(g.mode, Mode::SaveChoice);
        g.frame(DT, &press());
        dismiss(&mut g);
        assert!(g.has_saved);
        g.st.inv.mag = 0;
        g.st.player.hp = 0;
        run(&mut g, Input::default(), 3.2);
        assert!(matches!(g.mode, Mode::GameOver { .. }));
        run(&mut g, Input::default(), 2.1);
        g.frame(DT, &press());
        assert_eq!(g.mode, Mode::Play);
        assert_eq!(g.st.player.hp, 100);
        assert_eq!(g.st.inv.mag, 12);
        assert!(g.st.player.pos.dist_xz(v3(-4.6, 0.0, 10.5)) < 0.5);
    }
}
