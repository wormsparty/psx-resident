//! Rendu d'une image : pièce ou plein air, personnages, créatures,
//! paysage par les fenêtres, transitions de porte et d'escalier.

use super::*;
use crate::actors::{BossPose, DogPose, Held};
use crate::raster::{Fog, Light, Lighting, H, W};

impl Game {
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
        // Lueur du cœur de l'Abomination quand il est à découvert.
        if let Some(e) = self.st.enemies.iter().find(|e| e.room == self.st.room && e.kind == EnemyKind::Boss && e.weak_point_open()) {
            let h = actors::boss_heart(&self.boss_pose(e), e.pos, e.yaw);
            lights.push(Light { pos: h + Vec3::from_yaw(e.yaw) * 0.6, color: [1.4, 0.15, 0.1], radius: 5.0 });
        }
        let a = room.ambient;
        let sun = if room.outdoor { Some((self.outdoor.moon_dir, [0.38, 0.42, 0.58])) } else { None };
        Lighting { ambient: [a[0] * AMBIENT_GAIN, a[1] * AMBIENT_GAIN, a[2] * AMBIENT_GAIN], lights, sun }
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
        if p.knock > 0.0 {
            let e = KNOCK_TIME - p.knock;
            let k = smoothstep(0.0, 0.3, e) * (1.0 - smoothstep(KNOCK_TIME - 0.55, KNOCK_TIME, e));
            return Pose::knocked(k);
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
            let aim = match self.st.inv.weapon {
                Weapon::Pistol => Pose::aim(p.recoil, 0.0),
                Weapon::Shotgun => Pose::aim_shotgun(p.recoil),
                Weapon::Knife => Pose::knife(p.swing),
            };
            pose = pose.lerp(&aim, smoothstep(0.0, 1.0, p.aim_t));
        }
        if p.reload_t > 0.0 {
            pose.arm_fwd[1] = 0.9;
            pose.elbow[1] = 1.4;
            pose.arm_fwd[0] = 0.8 + (p.reload_t * 6.0).sin() * 0.2;
            pose.elbow[0] = 1.5;
            pose.arm_out = [-0.3, -0.2];
            pose.held = if self.st.inv.weapon == Weapon::Shotgun { Held::Shotgun } else { Held::Pistol };
            pose.head_pitch = 0.4;
        }
        pose.torso_pitch -= p.hurt_t * 0.3;
        pose
    }

    fn zombie_pose(&self, z: &Enemy) -> Pose {
        let t = self.time + z.variant as f32 * 2.0;
        match z.state {
            ZState::Eating => Pose::eating(t),
            ZState::Lying => Pose::lying(z.variant as f32 * 0.3),
            ZState::Rising => {
                if z.from_lying {
                    let mut kneel = Pose::eating(t);
                    kneel.torso_pitch = 0.4;
                    let a = smoothstep(0.0, 0.9, z.t);
                    let b = smoothstep(0.9, 1.8, z.t);
                    Pose::lying(0.3).lerp(&kneel, a).lerp(&Pose::shamble(z.phase, t), b)
                } else {
                    Pose::eating(t).lerp(&Pose::shamble(z.phase, t), smoothstep(0.0, 1.3, z.t))
                }
            }
            ZState::Idle => {
                let mut p = Pose::shamble(0.0, t);
                p.arm_fwd = [0.2, 0.3];
                p.head_roll = 0.5 + 0.1 * (t * 0.6).sin();
                p
            }
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
                let dur = if z.atk == 1 { 1.1 } else { 0.5 };
                let k = (1.0 - z.t / dur).max(0.0) * if z.atk == 1 { 1.5 } else { 1.0 };
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
            _ => Pose::shamble(z.phase, t),
        }
    }

    fn brute_pose(&self, z: &Enemy) -> Pose {
        let t = self.time;
        match z.state {
            ZState::Intro => {
                // Flotte dans la cuve, puis titube au dehors.
                let float = Pose { head_pitch: 0.5, arm_out: [0.3, 0.4], elbow: [0.4, 0.3], hip: [0.1, -0.1], knee: [0.3, 0.2], root_y: 0.4 + 0.05 * (t * 1.3).sin(), ..Default::default() };
                let k = smoothstep(0.4, 1.2, z.t);
                float.lerp(&Pose::brute_run(z.phase, t), k).lerp(&Pose::walk(z.phase, 0.0), k * 0.5)
            }
            ZState::Chase => {
                let mut p = Pose::walk(z.phase, 0.2);
                p.torso_pitch = 0.3;
                p.arm_out = [0.25, 0.4];
                p.elbow = [0.6, 0.4];
                p.head_pitch = -0.1;
                p
            }
            ZState::Attack => Pose::brute_slash(z.t * if z.hp * 2 < z.max_hp { 1.25 } else { 1.0 } / 1.15),
            ZState::Charge => {
                if z.t < 0.7 {
                    // Tête baissée, il gratte le sol.
                    let mut p = Pose::brute_run(0.0, t);
                    p.torso_pitch = 0.7;
                    p.hip = [0.5 + 0.2 * (t * 14.0).sin(), -0.4];
                    p.knee = [0.6, 0.8];
                    p
                } else {
                    Pose::brute_run(z.phase, t)
                }
            }
            ZState::Recover => Pose::idle(t).lerp(&Pose::brute_slash(0.95), 0.6),
            ZState::Stagger => {
                let mut p = Pose::idle(t);
                let k = if z.atk == 1 { 1.0 } else { (1.0 - z.t / 0.6).max(0.0) };
                p.torso_pitch = -0.4 * k + 0.5 * (z.atk == 1) as i32 as f32;
                p.head_pitch = -0.5 * k + 0.6 * (z.atk == 1) as i32 as f32;
                p.head_roll = 0.3 * (t * 3.0).sin() * k;
                p.knee = [0.4 * k, 0.6 * k];
                p
            }
            ZState::Dying | ZState::Dead => {
                let k = if z.state == ZState::Dead { 1.0 } else { smoothstep(0.3, 2.0, z.t) };
                let mut kneel = Pose::eating(t);
                kneel.torso_pitch = 0.2;
                let mut end = Pose::lying(0.4);
                end.root_pitch = 1.5;
                end.arm_fwd = [2.6, 2.9];
                Pose::idle(t).lerp(&kneel, smoothstep(0.0, 0.8, k * 2.0)).lerp(&end, smoothstep(0.5, 1.0, k))
            }
            _ => Pose::idle(t),
        }
    }

    pub(super) fn boss_pose(&self, z: &Enemy) -> BossPose {
        let mut p = BossPose { phase: z.phase, rise: 1.0, t: self.time, ..Default::default() };
        let k = if z.hp * 2 < z.max_hp { 1.3 } else { 1.0 };
        match z.state {
            ZState::Intro => {
                p.rise = smoothstep(0.6, 4.3, z.t);
                p.roar = smoothstep(4.0, 4.4, z.t);
            }
            ZState::Chase => p.walk = 1.0,
            ZState::Attack => {
                let (dur, hit) = match z.atk {
                    0 => (1.75, 1.05),
                    1 => (1.65, 1.0),
                    _ => (2.1, 1.35),
                };
                let tt = z.t * k;
                // Normalisé pour que l'impact tombe à 0,68 de l'animation.
                let s = if tt < hit { tt / hit * 0.68 } else { 0.68 + (tt - hit) / (dur - hit) * 0.32 };
                match z.atk {
                    0 => p.swipe = s,
                    1 => p.stab = s,
                    _ => p.slam = s,
                }
            }
            ZState::Roar => p.roar = smoothstep(0.0, 0.35, z.t) * (1.0 - smoothstep(2.3, 2.6, z.t)),
            ZState::Stagger => p.flinch = (z.t / 0.2).min(1.0) * (1.0 - smoothstep(0.8, 1.1, z.t)),
            ZState::Dying => {
                p.roar = smoothstep(0.0, 0.3, z.t) * (1.0 - smoothstep(1.5, 2.2, z.t));
                p.death = smoothstep(1.2, 3.5, z.t);
            }
            ZState::Dead => p.death = 1.0,
            _ => {}
        }
        p
    }

    fn dog_pose(&self, z: &Enemy) -> DogPose {
        let mut p = DogPose { phase: z.phase, lift: z.lift, ..Default::default() };
        let hound = z.kind == EnemyKind::Hound;
        match z.state {
            ZState::Eating => {
                p.head_pitch = 0.9;
                p.body_pitch = 0.15;
                p.jaw = (self.time * 9.0).sin().max(0.0) * 0.6;
            }
            ZState::Idle => {
                p.gait = 0.25;
                p.head_pitch = 0.5;
            }
            ZState::Intro => {
                if z.t < 2.4 {
                    // Hurle à la lune.
                    p.head_pitch = -0.9 * smoothstep(0.2, 0.6, z.t) * (1.0 - smoothstep(2.0, 2.4, z.t));
                    p.jaw = 0.8 * smoothstep(0.3, 0.6, z.t) * (1.0 - smoothstep(1.8, 2.2, z.t));
                    p.body_pitch = -0.15;
                } else {
                    p.gait = 1.0;
                    p.jaw = 0.4;
                }
            }
            ZState::Chase => {
                p.gait = 1.0;
                p.jaw = 0.3 + 0.2 * (self.time * 6.0).sin();
                p.head_pitch = 0.1;
            }
            ZState::Leap => {
                let crouch = if hound { 0.38 } else { 0.22 };
                if z.t < crouch {
                    p.crouch = smoothstep(0.0, crouch, z.t);
                    p.jaw = 0.6;
                } else {
                    p.body_pitch = -0.35;
                    p.gait = 0.4;
                    p.jaw = 1.0;
                    p.head_pitch = -0.2;
                }
            }
            ZState::Recover => {
                p.gait = 0.4;
                p.jaw = 0.5;
            }
            ZState::Roar => {
                p.head_pitch = -0.9 * smoothstep(0.0, 0.3, z.t) * (1.0 - smoothstep(1.5, 1.8, z.t));
                p.jaw = 0.8 * smoothstep(0.1, 0.3, z.t) * (1.0 - smoothstep(1.4, 1.7, z.t));
            }
            ZState::Stagger => {
                p.body_pitch = 0.2;
                p.head_pitch = -0.4;
                p.jaw = 0.8;
            }
            ZState::Dying | ZState::Dead => {
                p.dead = if z.state == ZState::Dead { 1.0 } else { smoothstep(0.0, 0.7, z.t) };
                p.jaw = 0.5;
            }
            _ => {}
        }
        p
    }

    pub(super) fn render(&mut self) {
        if let Mode::Door { t, door, .. } = self.mode {
            self.render_door(t, door);
        } else {
            let title = matches!(self.mode, Mode::Title) || matches!(self.mode, Mode::Options { title: true } | Mode::Controls { title: true, .. });
            if let Mode::Ending { t } = self.mode {
                // Vue depuis la barque qui s'éloigne du ponton.
                let back = t * 0.6;
                self.cam = Camera::look_at(v3(6.5 + back * 0.2, 1.0, -49.0 - back), v3(1.0, 4.0, -10.0), 58.0);
            }
            self.render_scene(title);
            let fade = match self.mode {
                Mode::GameOver { t } => 1.0 - (t * 0.8).min(0.75),
                Mode::Ending { t } => (1.0 - self.fade) * (1.0 - smoothstep(4.0, 9.0, t) * 0.7),
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

    pub(super) fn render_scene(&mut self, title: bool) {
        let light = self.lighting();
        let room_idx = self.st.room;
        let t = self.time;
        let pose = self.player_pose();
        let in_room: Vec<usize> = (0..self.st.enemies.len()).filter(|&i| self.st.enemies[i].room == room_idx).collect();
        // Les personnages reçoivent un peu plus de lumière pour rester lisibles.
        let mut char_light = light.clone();
        for c in &mut char_light.ambient {
            *c += 0.07;
        }
        let cam = match self.mode {
            Mode::Cutscene { cine, t } => cine.camera(t),
            _ => self.cam,
        };
        // Secousse de caméra.
        let mut cam = cam;
        if self.shake > 0.0 {
            let s = self.shake * self.shake * 0.12;
            cam.eye += cam.right * ((t * 61.0).sin() * s) + cam.up * ((t * 47.0).cos() * s);
        }
        let room = &self.world.rooms[room_idx];
        let r = &mut self.raster;
        r.clear(0);
        r.cam = cam;
        r.fog = room.fog;
        if room.outdoor {
            // Plein air : terrain prééclairé, façade du manoir.
            r.draw_mesh(&self.outdoor.ground, &Mat4::IDENTITY, None);
            r.draw_mesh(&self.outdoor.facade, &Mat4::IDENTITY, None);
        }
        r.draw_mesh(&room.mesh, &Mat4::IDENTITY, Some(&light));
        for c in &room.cond {
            if self.st.flags[c.flag] == c.when {
                r.draw_mesh(&c.mesh, &Mat4::IDENTITY, Some(&light));
            }
        }

        // Flaques de sang.
        for (ri, pos, rad, _) in &self.st.blood {
            if *ri != room_idx || *rad < 0.01 {
                continue;
            }
            let y = if room.outdoor { outdoor::walk_h(pos.x, pos.z) } else { 0.0 };
            let segs = 10;
            for k in 0..segs {
                let a0 = k as f32 / segs as f32 * TAU;
                let a1 = (k + 1) as f32 / segs as f32 * TAU;
                let j0 = 0.75 + 0.35 * hash2(k as i32, (pos.x * 10.0) as i32, 3);
                let j1 = 0.75 + 0.35 * hash2(((k + 1) % segs) as i32, (pos.x * 10.0) as i32, 3);
                let c = v3(pos.x, y + 0.012, pos.z);
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
            let y = if room.outdoor { outdoor::walk_h(cp.x, cp.z) } else { 0.0 };
            actors::draw_body(r, &self.corpse_body, &pz, v3(cp.x, y, cp.z), *cy, &char_light);
        }

        // Edgar, vivant, dans la cuisine.
        if room_idx == ROOM_KITCHEN && !self.st.flags[F_CRYPT_KEY] {
            actors::draw_body(r, &self.bodies[4], &Pose::sitting(t), world::EDGAR_POS, std::f32::consts::FRAC_PI_2, &char_light);
        }

        // Objets.
        for (i, it) in self.world.items.iter().enumerate() {
            if it.room != room_idx || self.st.taken[i] || it.flag.is_some_and(|f| !self.st.flags[f]) {
                continue;
            }
            let m = match it.kind {
                ItemKind::Ammo(_) => &self.items_mesh[0],
                ItemKind::Medkit => &self.items_mesh[1],
                ItemKind::Key(_) => &self.items_mesh[2],
                ItemKind::Shells(_) => &self.items_mesh[3],
                ItemKind::Herb => &self.items_mesh[4],
                ItemKind::Shotgun => &self.items_mesh[5],
            };
            let rot = if matches!(it.kind, ItemKind::Shotgun) { std::f32::consts::FRAC_PI_2 } else { i as f32 * 1.7 };
            let xf = Mat4::translate(it.pos) * Mat4::rot_y(rot);
            r.draw_mesh(m, &xf, Some(&char_light));
        }

        // Ennemis.
        let mut blobs: Vec<(Vec3, f32)> = Vec::new();
        for &i in &in_room {
            let z = &self.st.enemies[i];
            match z.kind {
                EnemyKind::Zombie => {
                    let body = &self.bodies[if z.look == 3 { 3 } else { z.variant as usize % 3 }];
                    let zp = self.zombie_pose(z);
                    actors::draw_body(&mut self.raster, body, &zp, z.pos, z.yaw, &char_light);
                }
                EnemyKind::Brute => {
                    let bp = self.brute_pose(z);
                    actors::draw_body(&mut self.raster, &self.bodies[5], &bp, z.pos, z.yaw, &char_light);
                }
                EnemyKind::Dog | EnemyKind::Hound => {
                    let dp = self.dog_pose(z);
                    let (body, scale) = if z.kind == EnemyKind::Hound { (&self.hound_body, 2.0) } else { (&self.dog_body, 1.0) };
                    actors::draw_dog(&mut self.raster, body, &dp, z.pos, z.yaw, scale, &char_light);
                }
                EnemyKind::Boss => {
                    let bp = self.boss_pose(z);
                    actors::draw_boss(&mut self.raster, &self.boss_body, &bp, z.pos, z.yaw, &char_light);
                }
            }
            if z.state != ZState::Dead && z.state != ZState::Intro {
                blobs.push((z.pos, z.radius() * 1.3));
            }
        }
        // Le Sujet 7 endormi dans sa cuve.
        if room_idx == ROOM_LAB && !self.st.flags[F_CRYPT_KEY] {
            let float = Pose { head_pitch: 0.5, arm_out: [0.3, 0.4], elbow: [0.4, 0.3], hip: [0.1, -0.1], knee: [0.3, 0.2], root_y: 0.45 + 0.05 * (t * 1.3).sin(), ..Default::default() };
            let mut green = char_light.clone();
            green.ambient = [0.08, 0.3, 0.12];
            actors::draw_body(&mut self.raster, &self.bodies[5], &float, v3(20.7, 0.0, 23.4), -std::f32::consts::FRAC_PI_2, &green);
        }

        // Héroïne.
        let p = &self.st.player;
        let hero_pos = if self.debug_hide_player { p.pos - v3(0.0, 50.0, 0.0) } else { p.pos };
        let sk = actors::draw_body(&mut self.raster, &self.hero, &pose, hero_pos, p.yaw, &char_light);
        blobs.push((p.pos, 0.38));

        // Ombres portées.
        let r = &mut self.raster;
        let shadow = [0.45, 0.45, 0.45];
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

        let room = &self.world.rooms[room_idx];
        if room.outdoor {
            // Lucioles, montagnes et ciel sur tout l'écran.
            for (i, f) in self.outdoor.fireflies.iter().enumerate() {
                let ph = t * 0.7 + i as f32 * 2.3;
                let blink = ((t * 1.3 + i as f32 * 0.77).sin() * 0.5 + 0.5).powi(3);
                if blink < 0.2 {
                    continue;
                }
                let pos = *f + v3(ph.sin() * 0.8, (ph * 1.3).sin() * 0.3, (ph * 0.8).cos() * 0.8);
                r.point(pos, 1, [0.5 * blink, 0.7 * blink, 0.2 * blink], Blend::Add);
            }
            r.fog = outdoor::MOUNTAIN_FOG;
            r.draw_mesh(&self.outdoor.mountains, &Mat4::IDENTITY, None);
            let od = &self.outdoor;
            r.fill_sky(&|d, _, _| od.sky(d, t));
            r.fog = room.fog;
        } else {
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
                // À l'étage, le paysage est vu de plus haut.
                let lower = v3(0.0, -room.level, 0.0);
                let xf = Mat4::translate(lower);
                r.fog = outdoor::GROUND_FOG;
                r.draw_mesh(&self.outdoor.ground, &xf, None);
                for (i, f) in self.outdoor.fireflies.iter().enumerate() {
                    let ph = t * 0.7 + i as f32 * 2.3;
                    let blink = ((t * 1.3 + i as f32 * 0.77).sin() * 0.5 + 0.5).powi(3);
                    if blink < 0.2 {
                        continue;
                    }
                    let pos = *f + lower + v3(ph.sin() * 0.8, (ph * 1.3).sin() * 0.3, (ph * 0.8).cos() * 0.8);
                    r.point(pos, 1, [0.5 * blink, 0.7 * blink, 0.2 * blink], Blend::Add);
                }
                for (e, life) in &self.eyes {
                    let k = (life / 4.0 * PI).sin().max(0.0);
                    let blink = if (t * 2.0 + e.x).sin() > 0.95 { 0.0 } else { 1.0 };
                    let side = r.cam.right * 0.12;
                    let c = [0.9 * k * blink, 0.7 * k * blink, 0.1 * k * blink];
                    r.point(*e + lower + side + v3(0.0, 0.6, 0.0), 1, c, Blend::Add);
                    r.point(*e + lower - side + v3(0.0, 0.6, 0.0), 1, c, Blend::Add);
                }
                r.fog = outdoor::MOUNTAIN_FOG;
                r.draw_mesh(&self.outdoor.mountains, &Mat4::IDENTITY, None);
                let od = &self.outdoor;
                r.fill_sky(&|d, _, _| od.sky(d, t));
                r.reset_scissor();
                r.fog = room.fog;
            }
        }

        // Transparences de la pièce (vitres, liquides...).
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
                if it.room != room_idx || self.st.taken[i] || it.flag.is_some_and(|f| !self.st.flags[f]) {
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
            let len = if self.st.inv.weapon == Weapon::Shotgun { -1.05 } else { -0.56 };
            let tip = sk.fore[1].point(v3(0.0, len, 0.03));
            let big = if self.st.inv.weapon == Weapon::Shotgun { 1.8 } else { 1.0 };
            r.billboard(tip, 0.16 * big, [1.0, 0.8, 0.4], T_FLAME, Blend::Add);
            r.billboard(tip, 0.09 * big, [1.0, 1.0, 0.8], T_GLINT, Blend::Add);
        }

        for p in &self.particles {
            r.point(p.pos, p.size, p.col, Blend::Opaque);
        }
    }

    fn render_door(&mut self, t: f32, door: usize) {
        let anim = self.world.rooms[self.st.room].doors.get(door).map_or(DoorAnim::Door, |d| d.anim);
        let r = &mut self.raster;
        r.clear(0);
        let light;
        match anim {
            DoorAnim::Stairs { up } => {
                // On gravit (ou descend) les marches dans la pénombre.
                let k = smoothstep(0.2, 2.3, t);
                let (eye, target) = if up {
                    (v3(0.0, 1.6 + k * 2.6, -1.5 + k * 3.6), v3(0.0, 2.4 + k * 2.6, 4.0 + k * 3.6))
                } else {
                    (v3(0.0, 5.6 - k * 2.6, 6.6 - k * 3.6), v3(0.0, 3.4 - k * 2.6, 0.6 - k * 3.6))
                };
                r.cam = Camera::look_at(eye, target, 60.0);
                r.fog = Fog { start: 1.0, end: 6.0, color: [0.0; 3] };
                light = Lighting { ambient: [0.08, 0.07, 0.07], lights: vec![Light { pos: eye + v3(0.4, 0.6, 1.5), color: [1.4, 1.0, 0.7], radius: 6.0 }], sun: None };
                r.draw_mesh(&self.stairs_scene, &Mat4::IDENTITY, Some(&light));
            }
            DoorAnim::Door => {
                let dolly = smoothstep(0.5, 2.3, t) * 2.2;
                r.cam = Camera::look_at(v3(0.0, 1.5, -3.0 + dolly), v3(0.0, 1.3, 2.0), 60.0);
                r.fog = Fog { start: 1.0, end: 5.5, color: [0.0; 3] };
                light = Lighting { ambient: [0.1, 0.09, 0.09], lights: vec![Light { pos: v3(0.4, 2.2, -1.6 + dolly), color: [1.6, 1.2, 0.8], radius: 6.0 }], sun: None };
                r.draw_mesh(&self.door_scene, &Mat4::IDENTITY, Some(&light));
                let open = smoothstep(0.35, 1.5, t) * 1.75;
                let xf = Mat4::translate(v3(-0.6, 0.0, 0.0)) * Mat4::rot_y(-open);
                r.draw_mesh(&self.door_leaf, &xf, Some(&light));
            }
            DoorAnim::Gate => {
                // Deux battants de fer s'ouvrent sur la brume bleutée.
                let dolly = smoothstep(0.5, 2.3, t) * 2.4;
                r.cam = Camera::look_at(v3(0.0, 1.6, -3.2 + dolly), v3(0.0, 1.2, 3.0), 60.0);
                let haze = outdoor::HAZE;
                r.fog = Fog { start: 1.5, end: 8.0, color: haze };
                light = Lighting { ambient: [0.16, 0.18, 0.24], lights: vec![Light { pos: v3(-1.4, 3.2, -0.8), color: [1.3, 0.85, 0.4], radius: 5.0 }], sun: Some((self.outdoor.moon_dir * -1.0, [0.25, 0.28, 0.4])) };
                let fb = crate::raster::pack(haze[0], haze[1], haze[2]);
                r.clear(fb);
                r.draw_mesh(&self.gate_scene, &Mat4::IDENTITY, Some(&light));
                let open = smoothstep(0.35, 1.5, t) * 1.6;
                let left = Mat4::translate(v3(-0.75, 0.0, 0.0)) * Mat4::rot_y(-open);
                let right = Mat4::translate(v3(0.75, 0.0, 0.0)) * Mat4::rot_y(open) * Mat4::translate(v3(-0.75, 0.0, 0.0));
                r.draw_mesh(&self.gate_leaf, &left, Some(&light));
                r.draw_mesh(&self.gate_leaf, &right, Some(&light));
            }
        }
        let fade_in = smoothstep(0.0, 0.3, t);
        let fade_out = 1.0 - smoothstep(1.9, 2.3, t);
        let k = fade_in * fade_out;
        r.post(k, [0.0; 3]);
    }
}
