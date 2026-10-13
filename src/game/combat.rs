//! Armes (pistolet, fusil à pompe, couteau), visée automatique, dégâts
//! infligés aux ennemis et reçus par la joueuse.

use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Weapon {
    Knife,
    #[default]
    Pistol,
    Shotgun,
}

/// Coup porté à un ennemi.
pub(super) struct Hit {
    pub dmg: i32,
    pub at: Vec3,
    pub dir: Vec3,
    /// Recul (fusil à bout portant).
    pub push: f32,
    pub crit: bool,
}

impl Game {
    pub(super) fn find_target(&self) -> Option<usize> {
        let p = &self.st.player;
        let fwd = Vec3::from_yaw(p.yaw);
        let mut best: Option<(usize, f32)> = None;
        for (i, z) in self.st.enemies.iter().enumerate() {
            // Un mort allongé n'attire pas la visée : la surprise reste entière.
            if !z.targetable(self.st.room) || z.state == ZState::Lying {
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
            // Les gros ennemis attirent la visée.
            let score = dist * (2.0 - ang) / if z.is_boss() { 1.6 } else { 1.0 };
            if best.map_or(true, |b| score < b.1) {
                best = Some((i, score));
            }
        }
        best.map(|b| b.0)
    }

    /// Recharge immédiatement (depuis l'inventaire). Faux si impossible.
    pub(super) fn reload_now(&mut self) -> bool {
        let inv = &mut self.st.inv;
        match inv.weapon {
            Weapon::Pistol if inv.mag < MAG_SIZE && inv.reserve > 0 => {
                let n = (MAG_SIZE - inv.mag).min(inv.reserve);
                inv.mag += n;
                inv.reserve -= n;
                self.play(Sfx::Reload, 0.6, 0.0, 1.0);
                true
            }
            Weapon::Shotgun if inv.shells_mag < SHELL_MAG && inv.shells > 0 => {
                let n = (SHELL_MAG - inv.shells_mag).min(inv.shells);
                inv.shells_mag += n;
                inv.shells -= n;
                self.play(Sfx::Pump, 0.6, 0.0, 1.0);
                true
            }
            _ => false,
        }
    }

    pub(super) fn start_reload(&mut self) {
        let p = &self.st.player;
        let inv = &self.st.inv;
        let can = match inv.weapon {
            Weapon::Pistol => inv.mag < MAG_SIZE && inv.reserve > 0,
            Weapon::Shotgun => inv.shells_mag < SHELL_MAG && inv.shells > 0,
            Weapon::Knife => false,
        };
        if p.reload_t > 0.0 || !can {
            return;
        }
        // Le fusil se recharge cartouche par cartouche : plus long.
        self.st.player.reload_t = if inv.weapon == Weapon::Shotgun { 1.9 } else { 1.2 };
        self.play(Sfx::Reload, 0.6, 0.0, if inv.weapon == Weapon::Shotgun { 0.8 } else { 1.0 });
    }

    pub(super) fn finish_reload(&mut self) {
        let inv = &mut self.st.inv;
        match inv.weapon {
            Weapon::Pistol => {
                let n = (MAG_SIZE - inv.mag).min(inv.reserve);
                inv.mag += n;
                inv.reserve -= n;
            }
            Weapon::Shotgun => {
                let n = (SHELL_MAG - inv.shells_mag).min(inv.shells);
                inv.shells_mag += n;
                inv.shells -= n;
                self.play(Sfx::Pump, 0.5, 0.0, 1.0);
            }
            Weapon::Knife => {}
        }
    }

    pub(super) fn fire(&mut self) {
        if self.st.player.fire_cd > 0.0 || self.st.player.reload_t > 0.0 {
            return;
        }
        match self.st.inv.weapon {
            Weapon::Knife => self.slash(),
            Weapon::Pistol => self.shoot_pistol(),
            Weapon::Shotgun => self.shoot_shotgun(),
        }
    }

    /// Ennemi visé : celui sous le curseur de la souris, sinon le plus proche
    /// devant la joueuse (manette, clavier).
    pub(super) fn enemy_under_cursor(&self, sx: f32, sy: f32) -> Option<usize> {
        let mut best: Option<(usize, f32)> = None;
        for (i, z) in self.st.enemies.iter().enumerate() {
            if !z.targetable(self.st.room) || z.state == ZState::Lying {
                continue;
            }
            // Silhouette approchée par un segment vertical, des pieds à la tête.
            let foot = z.pos + v3(0.0, 0.1, 0.0);
            let head = z.pos + v3(0.0, z.hit_height() + 0.35, 0.0);
            let (Some(a), Some(b)) = (self.cam.project(foot), self.cam.project(head)) else { continue };
            let (ax, ay, bx, by) = (a.0, a.1, b.0, b.1);
            let (dx, dy) = (bx - ax, by - ay);
            let l2 = (dx * dx + dy * dy).max(1e-3);
            let u = (((sx - ax) * dx + (sy - ay) * dy) / l2).clamp(0.0, 1.0);
            let (px, py) = (ax + dx * u - sx, ay + dy * u - sy);
            let d = (px * px + py * py).sqrt();
            // Largeur à l'écran : rayon de touche, plus une marge généreuse.
            let r = (z.hit_radius() * self.cam.focal / a.2).max(4.0) + 6.0;
            if d < r && best.map_or(true, |b| d < b.1) {
                best = Some((i, d));
            }
        }
        best.map(|b| b.0)
    }

    /// Point d'origine et direction du tir. Si la cible verrouillée est
    /// presque en face, le tir part droit sur elle : on touche ce qu'on vise.
    fn origin(&self) -> (Vec3, Vec3) {
        let p = &self.st.player;
        let origin = p.pos + v3(0.0, 1.35, 0.0);
        let fwd = Vec3::from_yaw(p.yaw);
        if let Some(zi) = p.lock {
            let z = &self.st.enemies[zi];
            if z.targetable(self.st.room) {
                let d = (z.pos - p.pos).flat();
                if d.len() > 0.1 && d.norm().dot(fwd) > 0.94 {
                    return (origin, d.norm());
                }
            }
        }
        (origin, fwd)
    }

    /// Le coup de feu alerte toute la pièce.
    fn alert_room(&mut self) {
        let room = self.st.room;
        for z in &mut self.st.enemies {
            if z.room == room {
                z.alerted = true;
            }
        }
    }

    fn dry_fire(&mut self) {
        self.play(Sfx::Empty, 0.6, 0.0, 1.0);
        self.st.player.fire_cd = 0.35;
        self.start_reload();
    }

    fn impact_wall(&mut self, origin: Vec3, fwd: Vec3, wall_t: f32) {
        let hit = origin + fwd * (wall_t - 0.05);
        for _ in 0..8 {
            let v = -fwd * self.rng.range(0.3, 1.5) + v3(self.rng.range(-0.8, 0.8), self.rng.range(0.0, 1.5), self.rng.range(-0.8, 0.8));
            let life = self.rng.range(0.2, 0.5);
            self.particles.push(Particle { pos: hit, vel: v, life, col: [0.5, 0.45, 0.4], size: 1 });
        }
    }

    fn shoot_pistol(&mut self) {
        if self.st.inv.mag == 0 {
            self.dry_fire();
            return;
        }
        self.st.inv.mag -= 1;
        self.st.player.fire_cd = 0.33;
        self.st.player.recoil = 1.0;
        self.muzzle = 0.07;
        let pitch = self.rng.range(0.92, 1.05);
        self.play(Sfx::Shot, 0.9, 0.0, pitch);
        let (origin, fwd) = self.origin();
        let wall_t = self.room().ray_dist(origin, fwd, 40.0);
        let mut best: Option<(usize, f32)> = None;
        for (i, z) in self.st.enemies.iter().enumerate() {
            if !z.targetable(self.st.room) {
                continue;
            }
            let rel = (z.pos - origin).flat();
            let t = rel.dot(fwd);
            if t < 0.0 {
                continue;
            }
            let perp2 = rel.dot(rel) - t * t;
            let r = z.hit_radius();
            if perp2 < r * r {
                let th = t - (r * r - perp2).sqrt();
                if th < wall_t && best.map_or(true, |b| th < b.1) {
                    best = Some((i, th.max(0.0)));
                }
            }
        }
        self.alert_room();
        match best {
            Some((zi, th)) => {
                // Pas de coup critique aléatoire : une balle, un point de dégât.
                let at = v3(origin.x, self.st.enemies[zi].pos.y, origin.z) + fwd * th;
                self.damage_enemy(zi, Hit { dmg: 1, at, dir: fwd, push: 0.0, crit: false });
            }
            None => self.impact_wall(origin, fwd, wall_t),
        }
    }

    fn shoot_shotgun(&mut self) {
        if self.st.inv.shells_mag == 0 {
            self.dry_fire();
            return;
        }
        self.st.inv.shells_mag -= 1;
        self.st.player.fire_cd = 1.05;
        self.st.player.recoil = 1.0;
        self.muzzle = 0.1;
        self.shake = self.shake.max(0.25);
        let pitch = self.rng.range(0.95, 1.03);
        self.play(Sfx::Shotgun, 1.0, 0.0, pitch);
        self.schedule(0.45, Sfx::Pump, 0.55, 0.0, 1.0);
        let (origin, fwd) = self.origin();
        let wall_t = self.room().ray_dist(origin, fwd, 25.0);
        // Gerbe de plombs : cône qui s'élargit, jusqu'à trois cibles.
        let mut hits: Vec<(usize, f32)> = Vec::new();
        for (i, z) in self.st.enemies.iter().enumerate() {
            if !z.targetable(self.st.room) {
                continue;
            }
            let rel = (z.pos - origin).flat();
            let t = rel.dot(fwd);
            if !(0.0..16.0).contains(&t) {
                continue;
            }
            let perp = (rel.dot(rel) - t * t).max(0.0).sqrt();
            if perp < z.hit_radius() + 0.15 + t * 0.12 && t - z.hit_radius() < wall_t {
                hits.push((i, (t - z.hit_radius()).max(0.0)));
            }
        }
        hits.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        self.alert_room();
        if hits.is_empty() {
            self.impact_wall(origin, fwd, wall_t);
        }
        for (k, (zi, th)) in hits.into_iter().take(3).enumerate() {
            let falloff = if k == 0 { 1.0 } else { 0.5 };
            let base = if th < 2.5 {
                6.0
            } else if th < 5.0 {
                4.0
            } else if th < 9.0 {
                2.0
            } else {
                1.0
            };
            let dmg = ((base * falloff) as i32).max(1);
            let at = v3(origin.x, self.st.enemies[zi].pos.y, origin.z) + fwd * th;
            let push = if th < 4.0 { 0.9 } else { 0.3 };
            self.damage_enemy(zi, Hit { dmg, at, dir: fwd, push, crit: th < 2.5 });
        }
    }

    fn slash(&mut self) {
        self.st.player.fire_cd = 0.5;
        self.st.player.swing = 0.01;
        let pitch = self.rng.range(0.9, 1.15);
        self.play(Sfx::Slash, 0.7, 0.0, pitch);
        let p = self.st.player.clone();
        let fwd = self.origin().1;
        let mut best: Option<(usize, f32)> = None;
        for (i, z) in self.st.enemies.iter().enumerate() {
            if !z.targetable(self.st.room) {
                continue;
            }
            let rel = (z.pos - p.pos).flat();
            let d = rel.len() - z.hit_radius();
            if d < 0.95 && rel.norm().dot(fwd) > 0.45 && best.map_or(true, |b| d < b.1) {
                best = Some((i, d));
            }
        }
        // Le couteau est silencieux : il n'alerte personne.
        if let Some((zi, _)) = best {
            let at = p.pos + fwd * 0.7 + v3(0.0, 1.1, 0.0);
            self.damage_enemy(zi, Hit { dmg: 1, at, dir: fwd, push: 0.0, crit: false });
        }
    }

    /// Applique un coup à l'ennemi `i` : dégâts, réaction, mort.
    pub(super) fn damage_enemy(&mut self, i: usize, hit: Hit) {
        let weak = self.st.enemies[i].weak_point_open();
        let dmg = if weak { hit.dmg * 3 } else { hit.dmg };
        let pitch = self.rng.range(0.9, 1.1);
        let kind = self.st.enemies[i].kind;
        let fall_back = self.rng.chance(0.6);
        let room = self.st.room;
        let mut at = hit.at;
        if kind == EnemyKind::Boss {
            let e = &self.st.enemies[i];
            at = if weak { actors::boss_heart(&self.boss_pose(e), e.pos, e.yaw) } else { e.pos + v3(0.0, 2.2, 0.0) - hit.dir * 0.8 };
        } else {
            at.y += self.st.enemies[i].hit_height();
        }
        let killed = {
            let z = &mut self.st.enemies[i];
            z.hp -= dmg;
            z.alerted = true;
            if hit.push > 0.0 && !z.is_boss() {
                z.pos += hit.dir.flat() * hit.push;
            }
            if z.hp <= 0 {
                z.die(fall_back);
                true
            } else {
                z.react(dmg, hit.push > 0.0);
                false
            }
        };
        if self.room().outdoor {
            let p = self.st.enemies[i].pos;
            self.st.enemies[i].pos.y = outdoor::walk_h(p.x, p.z);
        }
        let _ = room;
        self.play_at(Sfx::Hit, at, 0.7, pitch);
        let n = if hit.crit || weak { 40 } else { 14 };
        let col = if weak { [0.7, 0.05, 0.08] } else { [0.5, 0.02, 0.02] };
        self.blood_burst(at, hit.dir, n, col);
        if weak {
            self.play_at(Sfx::Roar, at, 0.5, 1.6);
        }
        if killed {
            self.st.kills += 1;
            self.on_enemy_killed(i);
        }
    }

    /// Dégâts reçus. `knock` : coup puissant qui projette au sol.
    pub(super) fn hurt_player(&mut self, dmg: i32, from: Vec3, knock: bool) -> bool {
        let p = &self.st.player;
        if p.invuln > 0.0 || p.hp <= 0 || p.knock > 0.0 {
            return false;
        }
        let ppos = p.pos;
        let p = &mut self.st.player;
        p.hp -= dmg;
        p.hurt_t = 1.0;
        p.reload_t = 0.0;
        p.swing = 0.0;
        self.hurt_flash = 0.6;
        let pitch = self.rng.range(0.95, 1.1);
        self.play(Sfx::Hurt, 0.7, 0.0, pitch);
        let away = (ppos - from).flat().norm();
        if knock && self.st.player.hp > 0 {
            let p = &mut self.st.player;
            p.knock = KNOCK_TIME;
            p.invuln = KNOCK_TIME + 0.6;
            p.yaw = (-away).yaw();
            p.aiming = false;
            self.shake = self.shake.max(0.5);
            let np = ppos + away * 1.1;
            let room = &self.world.rooms[self.st.room];
            let np = room.collide(np, PLAYER_R);
            self.st.player.pos = v3(np.x, self.floor_y(self.st.room, np), np.z);
            self.schedule(0.35, Sfx::Fall, 0.5, 0.0, 1.1);
        } else {
            self.st.player.invuln = 0.6;
            let np = ppos + away * 0.35;
            let np = self.world.rooms[self.st.room].collide(np, PLAYER_R);
            self.st.player.pos = v3(np.x, self.floor_y(self.st.room, np), np.z);
        }
        let neck = ppos + v3(0.0, 1.3, 0.0);
        self.blood_burst(neck, away, 20, [0.5, 0.02, 0.02]);
        true
    }
}
