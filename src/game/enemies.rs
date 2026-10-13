//! Ennemis : zombies, chiens du marais, le Sujet 7, l'Abomination
//! (le Docteur) et le Molosse. Chaque type a son IA ; les attaques des
//! boss sont télégraphiées (armé visible) pour rester esquivables.

use super::*;

/// Sujet 7 : durée du coup de griffe, de l'armé de la charge, vitesse de charge.
pub(super) const BRUTE_SLASH_TIME: f32 = 1.35;
pub(super) const BRUTE_WINDUP: f32 = 0.9;
const BRUTE_CHARGE_SPEED: f32 = 5.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum ZState {
    Eating,
    Lying,
    Rising,
    Idle,
    Chase,
    Grab,
    Stagger,
    Attack,
    Leap,
    Recover,
    Charge,
    Roar,
    /// En attente de sa cinématique d'apparition.
    Intro,
    Dying,
    Dead,
}

#[derive(Clone, Debug)]
pub(super) struct Enemy {
    pub kind: EnemyKind,
    pub room: usize,
    pub pos: Vec3,
    pub yaw: f32,
    pub hp: i32,
    pub max_hp: i32,
    pub state: ZState,
    pub t: f32,
    pub phase: f32,
    pub groan_t: f32,
    pub attack_cd: f32,
    pub variant: u32,
    /// Apparence (index dans `Game::bodies` pour les humanoïdes).
    pub look: u32,
    pub alerted: bool,
    /// Le coup de l'attaque en cours a déjà été porté.
    pub bitten: bool,
    pub fall_back: bool,
    /// Hauteur au-dessus du sol (bonds).
    pub lift: f32,
    /// Type de l'attaque en cours (dépend de l'ennemi).
    pub atk: u8,
    /// Dégâts encaissés depuis la dernière titubation (boss).
    pub poise: i32,
    /// Délai avant le prochain rugissement / la prochaine charge.
    pub roar_t: f32,
    pub from_lying: bool,
    /// Contournement d'un obstacle : côté choisi (signe) et durée restante.
    pub detour: f32,
}

impl Enemy {
    pub fn spawn(i: usize, s: &EnemySpawn) -> Enemy {
        let hp = match s.kind {
            // Tous les zombies encaissent le même nombre de balles (Edgar, plus
            // costaud, se reconnaît à sa livrée).
            EnemyKind::Zombie => if s.look == 3 { 8 } else { 5 },
            EnemyKind::Dog => 3,
            EnemyKind::Brute => 26,
            EnemyKind::Boss => 66,
            EnemyKind::Hound => 24,
        };
        let state = match (s.kind, s.state) {
            (EnemyKind::Brute | EnemyKind::Boss | EnemyKind::Hound, _) => ZState::Intro,
            (_, SpawnState::Eating) => ZState::Eating,
            (_, SpawnState::Lying) => ZState::Lying,
            _ => ZState::Idle,
        };
        Enemy {
            kind: s.kind,
            room: if s.wake.is_some() { NOWHERE } else { s.room },
            pos: s.pos,
            yaw: s.yaw,
            hp,
            max_hp: hp,
            state,
            t: 0.0,
            phase: i as f32 * 1.3,
            groan_t: 2.0 + i as f32 * 1.7,
            attack_cd: 0.0,
            variant: i as u32,
            look: s.look,
            alerted: false,
            bitten: false,
            fall_back: true,
            lift: 0.0,
            atk: 0,
            poise: 0,
            roar_t: 9.0,
            from_lying: false,
            detour: 0.0,
        }
    }

    pub fn is_boss(&self) -> bool {
        matches!(self.kind, EnemyKind::Brute | EnemyKind::Boss | EnemyKind::Hound)
    }

    pub fn alive(&self) -> bool {
        !matches!(self.state, ZState::Dying | ZState::Dead)
    }

    /// Peut être visé et touché.
    pub fn targetable(&self, room: usize) -> bool {
        self.room == room && self.alive() && self.state != ZState::Intro
    }

    /// Bloque les déplacements.
    pub fn solid(&self) -> bool {
        self.alive() && !matches!(self.state, ZState::Eating | ZState::Lying | ZState::Intro)
    }

    pub fn radius(&self) -> f32 {
        match self.kind {
            EnemyKind::Zombie => 0.3,
            EnemyKind::Dog => 0.32,
            EnemyKind::Brute => 0.42,
            EnemyKind::Boss => 1.0,
            EnemyKind::Hound => 0.6,
        }
    }

    pub fn hit_radius(&self) -> f32 {
        match self.kind {
            EnemyKind::Zombie => 0.42,
            EnemyKind::Dog => 0.45,
            EnemyKind::Brute => 0.55,
            EnemyKind::Boss => 1.15,
            EnemyKind::Hound => 0.85,
        }
    }

    /// Hauteur où gicle le sang.
    pub fn hit_height(&self) -> f32 {
        match (self.kind, self.state) {
            (EnemyKind::Zombie, ZState::Eating) => 0.7,
            (EnemyKind::Zombie, ZState::Lying) => 0.25,
            (EnemyKind::Zombie, _) => 1.3,
            (EnemyKind::Dog, _) => 0.55 + self.lift,
            (EnemyKind::Brute, _) => 1.7,
            (EnemyKind::Boss, _) => 2.3,
            (EnemyKind::Hound, _) => 1.1 + self.lift,
        }
    }

    pub fn name(&self) -> &'static str {
        match self.kind {
            EnemyKind::Brute => "SUJET 7",
            EnemyKind::Boss => "LE DOCTEUR ASHCROFT",
            EnemyKind::Hound => "LE MOLOSSE",
            _ => "",
        }
    }

    /// Le cœur de l'Abomination est à découvert (dégâts triplés).
    pub fn weak_point_open(&self) -> bool {
        self.kind == EnemyKind::Boss && ((self.state == ZState::Roar && self.t > 0.35 && self.t < 2.3) || self.state == ZState::Stagger)
    }

    fn enraged(&self) -> bool {
        self.hp * 2 < self.max_hp
    }

    /// Accélération du Sujet 7 à mi-vie.
    pub fn brute_rage(&self) -> f32 {
        if self.enraged() { 1.15 } else { 1.0 }
    }

    pub fn die(&mut self, fall_back: bool) {
        self.state = ZState::Dying;
        self.t = 0.0;
        self.fall_back = fall_back;
        self.lift = 0.0;
    }

    /// Réaction à un coup non mortel.
    pub fn react(&mut self, dmg: i32, blast: bool) {
        match self.kind {
            EnemyKind::Zombie => match self.state {
                ZState::Eating | ZState::Lying => {
                    self.from_lying = self.state == ZState::Lying;
                    self.state = ZState::Rising;
                    self.t = 0.0;
                }
                ZState::Grab | ZState::Rising => {}
                _ => {
                    self.state = ZState::Stagger;
                    self.t = 0.0;
                    self.atk = blast as u8;
                }
            },
            EnemyKind::Dog => {
                if self.state != ZState::Leap || blast {
                    self.state = ZState::Stagger;
                    self.t = 0.0;
                    self.lift = 0.0;
                }
            }
            _ => {
                self.poise += dmg;
                let threshold = match self.kind {
                    EnemyKind::Brute => 6,
                    EnemyKind::Boss => 12,
                    _ => 6,
                };
                let busy = matches!(self.state, ZState::Intro | ZState::Roar | ZState::Stagger);
                if self.poise >= threshold && !busy {
                    self.poise = 0;
                    self.state = ZState::Stagger;
                    self.t = 0.0;
                    self.atk = 0;
                    self.lift = 0.0;
                }
            }
        }
    }

    /// Fige l'ennemi en pleine attaque (captures d'écran).
    pub fn debug_attack(&mut self, t: f32) {
        self.state = match self.kind {
            EnemyKind::Dog | EnemyKind::Hound => ZState::Leap,
            _ => ZState::Attack,
        };
        self.t = t;
        self.atk = 0;
    }
}

/// Événement produit par l'IA, appliqué après la mise à jour de l'ennemi.
enum Act {
    Sound(Sfx, f32, f32),
    Hurt(i32, bool),
    Shake(f32),
    Dust(Vec3, f32),
    Died,
}

impl Game {
    pub(super) fn update_enemies(&mut self, dt: f32) {
        let room_idx = self.st.room;
        for i in 0..self.st.enemies.len() {
            if self.st.enemies[i].room != room_idx {
                continue;
            }
            let mut z = self.st.enemies[i].clone();
            z.attack_cd = (z.attack_cd - dt).max(0.0);
            z.t += dt;
            let mut acts: Vec<Act> = Vec::new();
            let before = z.pos;
            match z.kind {
                EnemyKind::Zombie => self.ai_zombie(&mut z, dt, &mut acts),
                EnemyKind::Dog => self.ai_dog(&mut z, dt, &mut acts, false),
                EnemyKind::Hound => self.ai_dog(&mut z, dt, &mut acts, true),
                EnemyKind::Brute => self.ai_brute(&mut z, dt, &mut acts),
                EnemyKind::Boss => self.ai_boss(&mut z, dt, &mut acts),
            }

            // Collisions avec le décor et les autres ennemis.
            let room = &self.world.rooms[room_idx];
            if z.solid() {
                let mut pos = room.collide(z.pos, z.radius());
                for (j, o) in self.st.enemies.iter().enumerate() {
                    if j == i || o.room != room_idx || !o.solid() {
                        continue;
                    }
                    let d = pos.flat() - o.pos.flat();
                    let l = d.len();
                    let min = z.radius() + o.radius();
                    if l < min && l > 1e-4 {
                        pos += d * ((min - l) / l * 0.5);
                    }
                }
                z.pos = room.collide(pos, z.radius());
                // Charge du Sujet 7 stoppée net par un mur : il reste sonné.
                if z.kind == EnemyKind::Brute && z.state == ZState::Charge && z.t > BRUTE_WINDUP {
                    let moved = z.pos.dist_xz(before);
                    if moved < BRUTE_CHARGE_SPEED * dt * 0.4 {
                        z.state = ZState::Stagger;
                        z.t = 0.0;
                        z.atk = 1;
                        acts.push(Act::Sound(Sfx::Thud, 0.9, 0.9));
                        acts.push(Act::Shake(0.5));
                    }
                }
            }
            z.pos.y = self.floor_y(room_idx, z.pos);

            // Flaque de sang une fois mort.
            if z.state == ZState::Dead && !self.st.blood.iter().any(|b| b.0 == room_idx && b.1.dist_xz(z.pos) < 0.05) {
                let size = match z.kind {
                    EnemyKind::Boss => 2.2,
                    EnemyKind::Brute | EnemyKind::Hound => 1.2,
                    EnemyKind::Dog => 0.5,
                    EnemyKind::Zombie => 0.8,
                };
                let off = if z.kind == EnemyKind::Zombie { Vec3::from_yaw(z.yaw) * if z.fall_back { -0.8 } else { 0.8 } } else { Vec3::ZERO };
                self.st.blood.push((room_idx, z.pos + off, 0.1, size));
                // Marqueur pour ne pas recréer la flaque.
                self.st.blood.push((room_idx, z.pos, 0.0, 0.0));
            }
            let zpos = z.pos;
            self.st.enemies[i] = z;
            for a in acts {
                match a {
                    Act::Sound(s, v, p) => self.play_at(s, zpos, v, p),
                    Act::Hurt(dmg, knock) => {
                        if self.hurt_player(dmg, zpos, knock) && !knock {
                            self.play(Sfx::Bite, 0.7, 0.0, 1.1);
                        }
                    }
                    Act::Shake(s) => self.shake = self.shake.max(s),
                    Act::Dust(at, r) => {
                        for k in 0..30 {
                            let a = k as f32 / 30.0 * TAU;
                            let d = v3(a.sin(), 0.0, a.cos());
                            let v = d * self.rng.range(1.0, 3.0) + v3(0.0, self.rng.range(0.5, 2.0), 0.0);
                            let life = self.rng.range(0.4, 0.9);
                            self.particles.push(Particle { pos: at + d * r * 0.5 + v3(0.0, 0.1, 0.0), vel: v, life, col: [0.35, 0.33, 0.3], size: 2 });
                        }
                    }
                    Act::Died => self.on_enemy_finished(i),
                }
            }
        }
    }

    /// Contexte commun : position de la joueuse, peut-elle être saisie/mordue.
    fn prey(&self) -> (Vec3, bool, bool) {
        let p = &self.st.player;
        let alive = p.hp > 0;
        let free = alive && p.grabbed <= 0.0 && p.knock <= 0.0 && p.invuln <= 0.0;
        (p.pos, alive, free)
    }

    fn ai_zombie(&mut self, z: &mut Enemy, dt: f32, acts: &mut Vec<Act>) {
        let (ppos, alive, free) = self.prey();
        let to_p = ppos - z.pos;
        let dist = to_p.flat().len();
        match z.state {
            ZState::Eating => {
                if dist < 4.5 || z.alerted {
                    z.state = ZState::Rising;
                    z.t = 0.0;
                    z.from_lying = false;
                    acts.push(Act::Sound(Sfx::Groan2, 0.8, 0.9));
                }
            }
            ZState::Lying => {
                if dist < 3.0 || z.alerted {
                    z.state = ZState::Rising;
                    z.t = 0.0;
                    z.from_lying = true;
                    acts.push(Act::Sound(Sfx::Groan2, 0.8, 0.8));
                }
            }
            ZState::Rising => {
                z.yaw = approach_angle(z.yaw, to_p.yaw(), 1.5 * dt);
                if z.t > if z.from_lying { 1.8 } else { 1.3 } {
                    z.state = ZState::Chase;
                }
            }
            ZState::Idle => {
                if dist < 8.0 || z.alerted {
                    z.state = ZState::Chase;
                    acts.push(Act::Sound(Sfx::Groan, 0.7, 1.0));
                }
            }
            ZState::Chase => {
                z.yaw = approach_angle(z.yaw, to_p.yaw(), 1.9 * dt);
                z.phase += dt * 3.0;
                // Edgar, fraîchement mort, est plus vif que les autres.
                let speed = if z.look == 3 { 0.78 } else { 0.5 + (z.variant % 3) as f32 * 0.08 };
                if alive && dist > 0.6 {
                    z.pos += Vec3::from_yaw(z.yaw) * (speed * (0.55 + 0.45 * z.phase.sin().abs()) * dt);
                }
                z.groan_t -= dt;
                if z.groan_t <= 0.0 {
                    z.groan_t = 4.0 + (z.phase * 7.3).sin().abs() * 5.0;
                    acts.push(Act::Sound(if z.variant % 2 == 0 { Sfx::Groan } else { Sfx::Groan2 }, 0.6, 0.9 + (z.variant % 5) as f32 * 0.07));
                }
                let facing = wrap_angle(to_p.yaw() - z.yaw).abs() < 0.7;
                if dist < 0.9 && facing && z.attack_cd <= 0.0 && free {
                    z.state = ZState::Grab;
                    z.t = 0.0;
                    z.bitten = false;
                }
            }
            ZState::Grab => {
                if z.t < dt * 1.5 {
                    let p = &mut self.st.player;
                    p.grabbed = 1.7;
                    p.yaw = (z.pos - ppos).yaw();
                    p.aiming = false;
                    p.reload_t = 0.0;
                    acts.push(Act::Sound(Sfx::Groan2, 0.9, 1.1));
                }
                if z.t > 0.65 && !z.bitten {
                    z.bitten = true;
                    let dmg = 18 + (self.rng.f() * 8.0) as i32;
                    let p = &mut self.st.player;
                    p.hp -= dmg;
                    p.hurt_t = 1.0;
                    self.hurt_flash = 0.6;
                    self.play(Sfx::Bite, 0.8, 0.0, 1.0);
                    let pitch = self.rng.range(0.95, 1.1);
                    self.play(Sfx::Hurt, 0.7, 0.0, pitch);
                    self.blood_burst(ppos + v3(0.0, 1.5, 0.0), Vec3::ZERO, 20, [0.5, 0.02, 0.02]);
                }
                if z.t > 1.7 {
                    z.state = ZState::Stagger;
                    z.t = 0.0;
                    z.atk = 0;
                    z.attack_cd = 2.8;
                    self.st.player.grabbed = 0.0;
                    self.st.player.invuln = 0.4;
                    if self.st.player.hp > 0 {
                        let away = (ppos - z.pos).flat().norm();
                        let np = self.world.rooms[self.st.room].collide(ppos + away * 0.6, PLAYER_R);
                        self.st.player.pos = v3(np.x, self.floor_y(self.st.room, np), np.z);
                    }
                }
            }
            ZState::Stagger => {
                // Le fusil à bout portant renverse presque.
                let long = z.atk == 1;
                if z.t < if long { 0.35 } else { 0.2 } {
                    z.pos -= Vec3::from_yaw(z.yaw) * ((if long { 1.4 } else { 0.8 }) * dt);
                }
                if z.t > if long { 1.1 } else { 0.5 } {
                    z.state = ZState::Chase;
                    z.atk = 0;
                }
            }
            ZState::Dying => {
                if z.t > 1.2 {
                    z.state = ZState::Dead;
                }
            }
            _ => {}
        }
        if z.state == ZState::Dying && z.t < dt * 1.5 {
            acts.push(Act::Sound(Sfx::Fall, 0.5, 1.0));
        }
    }

    /// Chiens du marais et Molosse (`hound`) : course, bond, morsure.
    /// Le chemin droit de `from` dans la direction `yaw` est-il libre sur
    /// `len` mètres pour un corps de rayon `r` ?
    fn path_clear(&self, from: Vec3, yaw: f32, len: f32, r: f32) -> bool {
        let room = self.room();
        let dir = Vec3::from_yaw(yaw);
        (1..=4).all(|k| {
            let p = from + dir * (len * k as f32 / 4.0);
            if p.x < room.min.0 + r || p.x > room.max.0 - r || p.z < room.min.1 + r || p.z > room.max.1 - r {
                return false;
            }
            room.obstacles.iter().all(|o| {
                let (cx, cz) = (p.x.clamp(o.min.0, o.max.0), p.z.clamp(o.min.1, o.max.1));
                (p.x - cx).powi(2) + (p.z - cz).powi(2) >= r * r
            })
        })
    }

    /// Direction à prendre pour aller vers `desired` sans buter contre le
    /// décor : on longe l'obstacle du même côté tant qu'il gêne.
    fn steer(&self, z: &mut Enemy, desired: f32, dist: f32, dt: f32) -> f32 {
        let r = z.radius() + 0.05;
        let look = dist.min(1.6);
        z.detour = if z.detour > 0.0 { (z.detour - dt).max(0.0) } else { (z.detour + dt).min(0.0) };
        if look < 0.4 || self.path_clear(z.pos, desired, look, r) {
            return desired;
        }
        let pick = |side: f32| (1..=8).map(|k| desired + side * k as f32 * 0.35).find(|&y| self.path_clear(z.pos, y, look, r));
        let side = if z.detour != 0.0 {
            z.detour.signum()
        } else {
            // Côté le plus court, sinon celui du flanc habituel.
            let steps = |side: f32| (1..=8).position(|k| self.path_clear(z.pos, desired + side * k as f32 * 0.35, look, r)).unwrap_or(99);
            let (l, rr) = (steps(1.0), steps(-1.0));
            if l < rr || (l == rr && z.variant % 2 == 0) { 1.0 } else { -1.0 }
        };
        match pick(side).map(|y| (y, side)).or_else(|| pick(-side).map(|y| (y, -side))) {
            Some((y, s)) => {
                z.detour = s * 0.8;
                y
            }
            None => desired,
        }
    }

    fn ai_dog(&mut self, z: &mut Enemy, dt: f32, acts: &mut Vec<Act>, hound: bool) {
        let (ppos, alive, free) = self.prey();
        let to_p = ppos - z.pos;
        let dist = to_p.flat().len();
        let facing = |z: &Enemy, lim: f32| wrap_angle(to_p.yaw() - z.yaw).abs() < lim;
        let pitch = if hound { 0.62 } else { 1.0 + (z.variant % 3) as f32 * 0.08 };
        match z.state {
            ZState::Eating => {
                z.phase += dt * 2.0;
                if dist < 6.0 || z.alerted {
                    z.state = ZState::Chase;
                    z.alerted = true;
                    acts.push(Act::Sound(Sfx::Snarl, 0.8, pitch));
                }
            }
            ZState::Idle => {
                // Rôde en rond en reniflant.
                z.yaw += dt * 0.6;
                z.phase += dt * 4.0;
                z.pos += Vec3::from_yaw(z.yaw) * (0.7 * dt);
                if dist < 11.0 || z.alerted {
                    z.state = ZState::Chase;
                    z.alerted = true;
                    acts.push(Act::Sound(Sfx::Snarl, 0.8, pitch));
                }
            }
            ZState::Chase => {
                // Les chiens contournent un peu leur proie avant de bondir.
                let flank = if z.variant % 2 == 0 { 0.6 } else { -0.6 };
                let desired = to_p.yaw() + if dist > 3.5 && !hound { flank } else { 0.0 };
                // Contourne meubles et arbres au lieu de s'y coller.
                let desired = self.steer(z, desired, dist, dt);
                z.yaw = approach_angle(z.yaw, desired, if hound { 3.2 } else { 5.5 } * dt);
                let speed = if !alive {
                    0.0
                } else if hound {
                    if z.enraged() {
                        5.2
                    } else {
                        4.5
                    }
                } else {
                    3.9
                };
                z.phase += dt * speed * if hound { 2.2 } else { 3.2 };
                z.pos += Vec3::from_yaw(z.yaw) * (speed * dt);
                z.groan_t -= dt;
                if z.groan_t <= 0.0 {
                    z.groan_t = self.rng.range(2.0, 4.0);
                    acts.push(Act::Sound(Sfx::Snarl, if hound { 0.9 } else { 0.6 }, pitch));
                }
                let reach = if hound { 4.6 } else { 2.8 };
                // Pas de bond à travers un meuble.
                let open = self.path_clear(z.pos, to_p.yaw(), (dist - 0.6).max(0.0), z.radius() * 0.8);
                if dist < reach && facing(z, 0.5) && z.attack_cd <= 0.0 && free && open {
                    z.state = ZState::Leap;
                    z.t = 0.0;
                    z.bitten = false;
                    acts.push(Act::Sound(Sfx::Snarl, 1.0, pitch * 1.1));
                }
                if hound {
                    z.roar_t -= dt;
                    if z.roar_t <= 0.0 && dist > 5.0 {
                        z.state = ZState::Roar;
                        z.t = 0.0;
                        z.roar_t = self.rng.range(10.0, 14.0);
                        acts.push(Act::Sound(Sfx::Howl, 1.0, 0.7));
                    }
                }
            }
            ZState::Leap => {
                let (crouch, flight, speed, height) = if hound { (0.38, 0.62, 8.5, 0.9) } else { (0.22, 0.48, 6.2, 0.45) };
                if z.t < crouch {
                    z.yaw = approach_angle(z.yaw, to_p.yaw(), 6.0 * dt);
                } else if z.t < crouch + flight {
                    let k = (z.t - crouch) / flight;
                    z.pos += Vec3::from_yaw(z.yaw) * (speed * dt);
                    z.lift = (k * PI).sin() * height;
                    if !z.bitten && dist < if hound { 1.3 } else { 0.85 } {
                        z.bitten = true;
                        if hound {
                            acts.push(Act::Hurt(24 + (self.rng.f() * 6.0) as i32, true));
                        } else {
                            acts.push(Act::Hurt(10 + (self.rng.f() * 5.0) as i32, false));
                        }
                    }
                } else {
                    z.lift = 0.0;
                    z.state = ZState::Recover;
                    z.t = 0.0;
                    z.attack_cd = self.rng.range(0.9, 1.5) * if hound { 1.2 } else { 1.0 };
                }
            }
            ZState::Recover => {
                z.pos -= Vec3::from_yaw(z.yaw) * (1.3 * dt);
                z.phase += dt * 5.0;
                if z.t > if hound { 0.8 } else { 0.5 } {
                    z.state = ZState::Chase;
                }
            }
            ZState::Roar => {
                if z.t > 1.8 {
                    z.state = ZState::Chase;
                }
            }
            ZState::Stagger => {
                if z.t < 0.15 {
                    z.pos -= Vec3::from_yaw(z.yaw) * (1.5 * dt);
                }
                if z.t > if hound { 0.6 } else { 0.4 } {
                    z.state = ZState::Chase;
                }
            }
            ZState::Dying => {
                if z.t < dt * 1.5 {
                    acts.push(Act::Sound(Sfx::Yelp, 0.9, pitch));
                }
                if z.t > if hound { 1.6 } else { 0.9 } {
                    z.state = ZState::Dead;
                    if hound {
                        acts.push(Act::Died);
                    }
                }
            }
            _ => {}
        }
    }

    /// Le Sujet 7 : coups de griffe et charges qui projettent au sol.
    fn ai_brute(&mut self, z: &mut Enemy, dt: f32, acts: &mut Vec<Act>) {
        let (ppos, alive, free) = self.prey();
        let to_p = ppos - z.pos;
        let dist = to_p.flat().len();
        let ang = wrap_angle(to_p.yaw() - z.yaw).abs();
        let rage = z.brute_rage();
        match z.state {
            ZState::Chase => {
                z.yaw = approach_angle(z.yaw, to_p.yaw(), 2.3 * rage * dt);
                let speed = if alive { 1.3 * rage } else { 0.0 };
                let old = z.phase;
                z.phase += dt * speed * 2.6;
                z.pos += Vec3::from_yaw(z.yaw) * (speed * dt);
                if (old / PI).floor() != (z.phase / PI).floor() {
                    acts.push(Act::Sound(Sfx::Step, 0.8, 0.55));
                }
                z.roar_t -= dt;
                if dist < 2.1 && ang < 0.6 && z.attack_cd <= 0.0 && free {
                    z.state = ZState::Attack;
                    z.t = 0.0;
                    z.bitten = false;
                    acts.push(Act::Sound(Sfx::Brute, 0.7, 1.3));
                } else if dist > 3.5 && dist < 9.0 && z.attack_cd <= 0.0 && z.roar_t <= 0.0 && free {
                    z.state = ZState::Charge;
                    z.t = 0.0;
                    z.bitten = false;
                    z.roar_t = self.rng.range(7.0, 10.0);
                    acts.push(Act::Sound(Sfx::Brute, 1.0, 1.0));
                }
            }
            ZState::Attack => {
                let s = z.t * rage / BRUTE_SLASH_TIME;
                if s < 0.45 {
                    z.yaw = approach_angle(z.yaw, to_p.yaw(), 2.0 * dt);
                }
                if s >= 0.56 && !z.bitten {
                    z.bitten = true;
                    acts.push(Act::Sound(Sfx::Slash, 0.9, 0.7));
                    if dist < 2.4 && ang < 0.9 {
                        acts.push(Act::Hurt(22, true));
                    }
                }
                if s >= 1.0 {
                    z.state = ZState::Chase;
                    z.attack_cd = 1.3;
                }
            }
            ZState::Charge => {
                if z.t < BRUTE_WINDUP {
                    // Il gratte le sol, tête baissée : c'est le moment d'esquiver.
                    z.yaw = approach_angle(z.yaw, to_p.yaw(), 2.6 * dt);
                } else {
                    let old = z.phase;
                    z.phase += dt * 15.0;
                    z.pos += Vec3::from_yaw(z.yaw) * (BRUTE_CHARGE_SPEED * dt);
                    if (old / PI).floor() != (z.phase / PI).floor() {
                        acts.push(Act::Sound(Sfx::Step, 1.0, 0.5));
                    }
                    if !z.bitten && dist < 1.15 {
                        z.bitten = true;
                        acts.push(Act::Hurt(20, true));
                        z.state = ZState::Recover;
                        z.t = 0.0;
                    } else if z.t > 2.1 {
                        z.state = ZState::Recover;
                        z.t = 0.0;
                    }
                }
            }
            ZState::Recover => {
                if z.t > 1.2 {
                    z.state = ZState::Chase;
                    z.attack_cd = 1.4;
                }
            }
            ZState::Stagger => {
                // atk == 1 : sonné après avoir percuté un mur.
                let dur = if z.atk == 1 { 2.2 } else { 0.6 };
                if z.t < 0.2 {
                    z.pos -= Vec3::from_yaw(z.yaw) * (0.8 * dt);
                }
                if z.t > dur {
                    z.state = ZState::Chase;
                    z.atk = 0;
                }
            }
            ZState::Dying => {
                if z.t < dt * 1.5 {
                    acts.push(Act::Sound(Sfx::Brute, 1.0, 0.6));
                }
                if z.t > 2.2 {
                    z.state = ZState::Dead;
                    acts.push(Act::Sound(Sfx::Thud, 0.9, 1.0));
                    acts.push(Act::Died);
                }
            }
            _ => {}
        }
    }

    /// L'Abomination : griffe, lame d'os, écrasement à deux bras, rugissements
    /// qui découvrent son cœur. Plus rapide quand elle est à moitié morte.
    fn ai_boss(&mut self, z: &mut Enemy, dt: f32, acts: &mut Vec<Act>) {
        let (ppos, alive, free) = self.prey();
        let to_p = ppos - z.pos;
        let dist = to_p.flat().len();
        let ang = wrap_angle(to_p.yaw() - z.yaw).abs();
        let rage = z.enraged();
        let k = if rage { 1.3 } else { 1.0 };
        let fwd = Vec3::from_yaw(z.yaw);
        match z.state {
            ZState::Chase => {
                z.yaw = approach_angle(z.yaw, to_p.yaw(), if rage { 2.0 } else { 1.4 } * dt);
                let speed = if alive { 0.95 * k } else { 0.0 };
                let old = z.phase;
                z.phase += dt * speed * 2.0;
                if dist > 2.2 {
                    z.pos += fwd * (speed * dt);
                }
                if (old / PI).floor() != (z.phase / PI).floor() {
                    acts.push(Act::Sound(Sfx::Thud, 0.7, 1.2));
                    acts.push(Act::Shake(0.12));
                }
                z.roar_t -= dt;
                if z.roar_t <= 0.0 {
                    z.state = ZState::Roar;
                    z.t = 0.0;
                    z.roar_t = if rage { self.rng.range(7.0, 10.0) } else { self.rng.range(10.0, 14.0) };
                    acts.push(Act::Sound(Sfx::Roar, 1.0, 1.0));
                } else if z.attack_cd <= 0.0 && free {
                    if dist < 3.7 && ang < 0.75 {
                        z.state = ZState::Attack;
                        z.t = 0.0;
                        z.bitten = false;
                        z.variant ^= 1;
                        z.atk = (z.variant & 1) as u8;
                        acts.push(Act::Sound(Sfx::Brute, 0.6, 0.55));
                    } else if dist < 8.0 && ang < 0.5 && self.rng.chance(dt * if rage { 1.2 } else { 0.7 }) {
                        z.state = ZState::Attack;
                        z.t = 0.0;
                        z.bitten = false;
                        z.atk = 2;
                        acts.push(Act::Sound(Sfx::Roar, 0.6, 1.4));
                    }
                }
            }
            ZState::Attack => {
                let (dur, hit) = match z.atk {
                    0 => (1.75, 1.05),
                    1 => (1.65, 1.0),
                    _ => (2.1, 1.35),
                };
                let tt = z.t * k;
                if tt < hit - 0.25 {
                    z.yaw = approach_angle(z.yaw, to_p.yaw(), 1.1 * dt);
                }
                if tt >= hit && !z.bitten {
                    z.bitten = true;
                    match z.atk {
                        0 => {
                            acts.push(Act::Sound(Sfx::Slash, 1.0, 0.5));
                            if dist < 3.9 && ang < 1.0 {
                                acts.push(Act::Hurt(32, true));
                            }
                        }
                        1 => {
                            acts.push(Act::Sound(Sfx::Slash, 1.0, 0.4));
                            if dist < 4.7 && ang < 0.38 {
                                acts.push(Act::Hurt(30, true));
                            }
                        }
                        _ => {
                            let impact = z.pos + fwd * 2.7;
                            acts.push(Act::Sound(Sfx::Thud, 1.0, 0.7));
                            acts.push(Act::Shake(0.9));
                            acts.push(Act::Dust(impact, 2.6));
                            if ppos.dist_xz(impact) < 2.7 {
                                acts.push(Act::Hurt(40, true));
                            }
                        }
                    }
                }
                if tt >= dur {
                    z.state = ZState::Chase;
                    z.attack_cd = if rage { 0.55 } else { 1.1 };
                }
            }
            ZState::Roar => {
                if z.t < 0.5 {
                    acts.push(Act::Shake(0.2));
                }
                if z.t > 2.6 {
                    z.state = ZState::Chase;
                    z.attack_cd = 0.4;
                }
            }
            ZState::Stagger => {
                if z.t > 1.1 {
                    z.state = ZState::Chase;
                    z.attack_cd = 0.3;
                }
            }
            ZState::Dying => {
                if z.t < dt * 1.5 {
                    acts.push(Act::Sound(Sfx::Roar, 1.0, 0.7));
                }
                if z.t > 2.6 && z.t - dt <= 2.6 {
                    acts.push(Act::Sound(Sfx::Thud, 1.0, 0.6));
                    acts.push(Act::Shake(0.8));
                    acts.push(Act::Dust(z.pos, 2.5));
                }
                if z.t > 4.5 {
                    z.state = ZState::Dead;
                    acts.push(Act::Died);
                }
            }
            _ => {}
        }
    }
}
