//! Entrées unifiées : clavier, souris et manette (gilrs, natif comme web).
//! Les frontends transmettent des événements bruts ; le jeu lit un `Input`
//! consolidé une fois par image.

use gilrs::{Axis, Button, Gilrs};

/// Touches physiques utilisées (position QWERTY : ZQSD en AZERTY = WASD ici).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Key {
    W,
    A,
    S,
    D,
    Up,
    Down,
    Left,
    Right,
    Shift,
    Ctrl,
    Space,
    E,
    Enter,
    R,
    Tab,
    I,
    Esc,
    P,
    F,
}

const NKEYS: usize = Key::F as usize + 1;

#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    /// Déplacement analogique : x = droite, y = avant (haut de l'écran).
    pub mv: (f32, f32),
    pub run: bool,
    pub aim: bool,
    /// La visée suit le curseur de la souris.
    pub aim_mouse: bool,
    pub fire: bool,
    pub interact: bool,
    pub reload: bool,
    pub inventory: bool,
    pub pause: bool,
    pub confirm: bool,
    pub cancel: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    /// Position de la souris en pixels du framebuffer 320x240.
    pub mouse: Option<(f32, f32)>,
    /// Clic gauche (front montant).
    pub click: bool,
    pub any: bool,
    pub using_pad: bool,
}

#[derive(Clone, Copy, Default)]
struct PadState {
    lx: f32,
    ly: f32,
    south: bool,
    east: bool,
    west: bool,
    north: bool,
    l1: bool,
    l2: bool,
    r1: bool,
    r2: bool,
    start: bool,
    select: bool,
    up: bool,
    down: bool,
    left: bool,
    right: bool,
}

pub struct InputCollector {
    down: [bool; NKEYS],
    pressed: [bool; NKEYS],
    mouse_pos: Option<(f32, f32)>,
    mouse_l: bool,
    mouse_r: bool,
    mouse_l_pressed: bool,
    mouse_r_pressed: bool,
    gilrs: Option<Gilrs>,
    pad: PadState,
    prev_pad: PadState,
    prev_stick_dir: (i32, i32),
    last_pad_use: bool,
}

impl InputCollector {
    pub fn new() -> InputCollector {
        InputCollector {
            down: [false; NKEYS],
            pressed: [false; NKEYS],
            mouse_pos: None,
            mouse_l: false,
            mouse_r: false,
            mouse_l_pressed: false,
            mouse_r_pressed: false,
            gilrs: Gilrs::new().ok(),
            pad: PadState::default(),
            prev_pad: PadState::default(),
            prev_stick_dir: (0, 0),
            last_pad_use: false,
        }
    }

    pub fn key(&mut self, k: Key, down: bool) {
        let i = k as usize;
        if down && !self.down[i] {
            self.pressed[i] = true;
        }
        self.down[i] = down;
        self.last_pad_use = false;
    }

    pub fn mouse_move(&mut self, x: f32, y: f32) {
        if let Some((ox, oy)) = self.mouse_pos {
            if (ox - x).abs() + (oy - y).abs() > 3.0 {
                self.last_pad_use = false;
            }
        }
        self.mouse_pos = Some((x, y));
    }

    pub fn mouse_leave(&mut self) {
        self.mouse_pos = None;
    }

    /// `button` : 0 = gauche, 2 = droite.
    pub fn mouse_button(&mut self, button: u32, down: bool) {
        match button {
            0 => {
                if down && !self.mouse_l {
                    self.mouse_l_pressed = true;
                }
                self.mouse_l = down;
            }
            2 => {
                if down && !self.mouse_r {
                    self.mouse_r_pressed = true;
                }
                self.mouse_r = down;
            }
            _ => {}
        }
        self.last_pad_use = false;
    }

    /// Relâche tout (perte de focus).
    pub fn reset(&mut self) {
        self.down = [false; NKEYS];
        self.mouse_l = false;
        self.mouse_r = false;
    }

    fn poll_pad(&mut self) {
        self.prev_pad = self.pad;
        let Some(g) = self.gilrs.as_mut() else { return };
        while g.next_event().is_some() {}
        let mut st = PadState::default();
        for (_id, gp) in g.gamepads() {
            if !gp.is_connected() {
                continue;
            }
            let lx = gp.value(Axis::LeftStickX);
            let ly = gp.value(Axis::LeftStickY);
            if lx.abs() > st.lx.abs() {
                st.lx = lx;
            }
            if ly.abs() > st.ly.abs() {
                st.ly = ly;
            }
            st.south |= gp.is_pressed(Button::South);
            st.east |= gp.is_pressed(Button::East);
            st.west |= gp.is_pressed(Button::West);
            st.north |= gp.is_pressed(Button::North);
            st.l1 |= gp.is_pressed(Button::LeftTrigger);
            st.l2 |= gp.is_pressed(Button::LeftTrigger2) || gp.value(Axis::LeftZ) > 0.4;
            st.r1 |= gp.is_pressed(Button::RightTrigger);
            st.r2 |= gp.is_pressed(Button::RightTrigger2) || gp.value(Axis::RightZ) > 0.4;
            st.start |= gp.is_pressed(Button::Start);
            st.select |= gp.is_pressed(Button::Select);
            st.up |= gp.is_pressed(Button::DPadUp);
            st.down |= gp.is_pressed(Button::DPadDown);
            st.left |= gp.is_pressed(Button::DPadLeft);
            st.right |= gp.is_pressed(Button::DPadRight);
        }
        self.pad = st;
    }

    /// Consolide l'état de l'image et remet à zéro les fronts montants.
    pub fn poll(&mut self) -> Input {
        self.poll_pad();
        let down = self.down;
        let pressed = self.pressed;
        let k = |key: Key| down[key as usize];
        let kp = |key: Key| pressed[key as usize];
        let p = self.pad;
        let pp = self.prev_pad;
        let edge = |now: bool, before: bool| now && !before;

        // Déplacement clavier.
        let mut mx = 0.0f32;
        let mut my = 0.0f32;
        if k(Key::A) || k(Key::Left) {
            mx -= 1.0;
        }
        if k(Key::D) || k(Key::Right) {
            mx += 1.0;
        }
        if k(Key::W) || k(Key::Up) {
            my += 1.0;
        }
        if k(Key::S) || k(Key::Down) {
            my -= 1.0;
        }
        let keyboard_move = mx != 0.0 || my != 0.0;
        let l = (mx * mx + my * my).sqrt();
        if l > 1.0 {
            mx /= l;
            my /= l;
        }
        // Manette (zone morte radiale).
        let mut px = p.lx;
        let mut py = p.ly;
        if p.left {
            px = -1.0;
        }
        if p.right {
            px = 1.0;
        }
        if p.up {
            py = 1.0;
        }
        if p.down {
            py = -1.0;
        }
        let pl = (px * px + py * py).sqrt();
        let pad_move = pl > 0.22;
        let (px, py) = if pad_move {
            let m = ((pl - 0.22) / 0.78).min(1.0) / pl;
            (px * m, py * m)
        } else {
            (0.0, 0.0)
        };
        let pad_any = pad_move
            || p.south
            || p.east
            || p.west
            || p.north
            || p.l1
            || p.l2
            || p.r1
            || p.r2
            || p.start
            || p.select;
        if pad_any {
            self.last_pad_use = true;
        }

        let mv = if keyboard_move { (mx, my) } else { (px, py) };
        let pad_mag = (px * px + py * py).sqrt();

        // Navigation de menu : fronts sur le stick.
        let sdir = (
            if px > 0.6 { 1 } else if px < -0.6 { -1 } else { 0 },
            if py > 0.6 { 1 } else if py < -0.6 { -1 } else { 0 },
        );
        let stick_up = sdir.1 == 1 && self.prev_stick_dir.1 != 1;
        let stick_down = sdir.1 == -1 && self.prev_stick_dir.1 != -1;
        let stick_left = sdir.0 == -1 && self.prev_stick_dir.0 != -1;
        let stick_right = sdir.0 == 1 && self.prev_stick_dir.0 != 1;
        self.prev_stick_dir = sdir;

        let aim_kb = k(Key::Ctrl) || self.mouse_r;
        let input = Input {
            mv,
            run: k(Key::Shift) || p.east || (pad_move && pad_mag > 0.85),
            aim: aim_kb || p.l2 || p.l1,
            aim_mouse: self.mouse_r && !p.l2 && !p.l1,
            fire: self.mouse_l_pressed || kp(Key::Space) || kp(Key::F) || edge(p.r2, pp.r2) || edge(p.r1, pp.r1),
            interact: kp(Key::E) || kp(Key::Enter) || edge(p.south, pp.south),
            reload: kp(Key::R) || edge(p.west, pp.west),
            inventory: kp(Key::Tab) || kp(Key::I) || edge(p.north, pp.north) || edge(p.select, pp.select),
            pause: kp(Key::Esc) || kp(Key::P) || edge(p.start, pp.start),
            confirm: kp(Key::E) || kp(Key::Enter) || kp(Key::Space) || self.mouse_l_pressed || edge(p.south, pp.south) || edge(p.start, pp.start),
            cancel: kp(Key::Esc) || kp(Key::Tab) || kp(Key::I) || self.mouse_r_pressed || edge(p.east, pp.east) || edge(p.north, pp.north),
            up: kp(Key::W) || kp(Key::Up) || edge(p.up, pp.up) || stick_up,
            down: kp(Key::S) || kp(Key::Down) || edge(p.down, pp.down) || stick_down,
            left: kp(Key::A) || kp(Key::Left) || edge(p.left, pp.left) || stick_left,
            right: kp(Key::D) || kp(Key::Right) || edge(p.right, pp.right) || stick_right,
            mouse: self.mouse_pos,
            click: self.mouse_l_pressed,
            any: pressed.iter().any(|&b| b) || self.mouse_l_pressed || (pad_any && !(pp.south || pp.start || pp.east)),
            using_pad: self.last_pad_use,
        };
        self.pressed = [false; NKEYS];
        self.mouse_l_pressed = false;
        self.mouse_r_pressed = false;
        input
    }
}

impl Default for InputCollector {
    fn default() -> Self {
        Self::new()
    }
}
