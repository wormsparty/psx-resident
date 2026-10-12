//! Le fil de l'histoire : drapeaux, déclencheurs, cinématiques des boss,
//! énigme des portraits, Edgar le majordome et la fin.

use super::*;
use crate::world::PIT;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Cine {
    /// Le Sujet 7 brise sa cuve.
    Brute,
    /// Le Docteur surgit de la fosse rituelle.
    Boss,
    /// Le Molosse hurle au bout du ponton.
    Hound,
}

/// Durée du défilement de la fin avant de pouvoir revenir au titre.
pub(super) const ENDING_LEN: f32 = 16.0;

pub(super) const ENDING_TEXT: [&str; 6] = [
    "La barque glisse sur l'eau noire.",
    "Derrière vous, le manoir Ashcroft s'enfonce dans la brume.",
    "Sur la rive, plus aucun chien ne hurle.",
    "La pleine lune pâlit. L'aube se lève sur le marais.",
    "",
    "Vous avez survécu.",
];

const TANK: Vec3 = v3(20.7, 0.0, 23.4);

impl Cine {
    fn duration(self) -> f32 {
        match self {
            Cine::Brute => 3.8,
            Cine::Boss => 6.2,
            Cine::Hound => 4.2,
        }
    }

    fn kind(self) -> EnemyKind {
        match self {
            Cine::Brute => EnemyKind::Brute,
            Cine::Boss => EnemyKind::Boss,
            Cine::Hound => EnemyKind::Hound,
        }
    }

    pub(super) fn camera(self, t: f32) -> Camera {
        match self {
            Cine::Brute => Camera::look_at(v3(14.2, 1.9 + t * 0.05, 20.2), v3(20.3, 1.5, 23.4), 58.0),
            Cine::Boss => {
                let k = smoothstep(0.0, 6.0, t);
                Camera::look_at(v3(-7.5 + k * 1.5, 1.2, 27.4 + k * 0.8), PIT + v3(0.0, 1.6 + k * 1.6, 0.0), 64.0)
            }
            Cine::Hound => Camera::look_at(v3(0.8, 1.3, -33.0), v3(4.0, 0.4, -43.5), 56.0),
        }
    }
}

impl Game {
    pub(super) fn set_flag(&mut self, f: usize) {
        if self.st.flags[f] {
            return;
        }
        self.st.flags[f] = true;
        for (i, s) in self.world.enemies.iter().enumerate() {
            if s.wake == Some(f) {
                self.st.enemies[i].room = s.room;
            }
        }
    }

    fn enemy_of(&self, kind: EnemyKind) -> Option<usize> {
        self.st.enemies.iter().position(|e| e.kind == kind)
    }

    /// Boss en plein combat dans la pièce (barre de vie, musique).
    pub(super) fn boss_active(&self) -> Option<usize> {
        self.st
            .enemies
            .iter()
            .position(|e| e.room == self.st.room && e.is_boss() && e.alive() && e.state != ZState::Intro && e.alerted)
    }

    /// Arrivée dans une pièce (après une porte).
    pub(super) fn on_enter_room(&mut self) {
        match self.st.room {
            ROOM_CRYPT if !self.flag(F_BOSS_INTRO) => {
                self.set_flag(F_BOSS_INTRO);
                self.start_cutscene(Cine::Boss);
            }
            ROOM_PORCH if !self.flag(F_PORCH_SEEN) => {
                self.set_flag(F_PORCH_SEEN);
                self.message("L'air glacé de la nuit. La pleine lune inonde le parc d'une lumière blafarde.");
                self.message("Près du portail, des grognements. Les chiens du marais vous attendaient.");
            }
            ROOM_KITCHEN if self.flag(F_CRYPT_KEY) && !self.flag(F_EDGAR_MET) => {
                // On n'avait jamais parlé à Edgar : il est trop tard.
                self.set_flag(F_EDGAR_MET);
            }
            _ => {}
        }
    }

    /// Déclencheurs liés à la position.
    pub(super) fn check_triggers(&mut self) {
        if self.mode != Mode::Play {
            return;
        }
        if self.st.room == ROOM_MARSH && !self.flag(F_HOUND_INTRO) && self.st.player.pos.z < -30.5 {
            self.set_flag(F_HOUND_INTRO);
            self.start_cutscene(Cine::Hound);
        }
    }

    pub(super) fn on_key_found(&mut self, k: u8) {
        match k {
            KEY_EMBLEM => self.message("Le blason gravé ressemble à celui de la porte nord du hall."),
            KEY_LAB => self.message("L'anneau figure un corbeau... comme celui de la porte est de la galerie."),
            KEY_CRYPT => {
                self.message("Une tête de mort orne l'anneau. La lourde porte de la cuisine, peut-être...");
                self.set_flag(F_CRYPT_KEY);
                self.prepare_cutscene(Cine::Brute);
                self.after_msg = Some(Mode::Cutscene { cine: Cine::Brute, t: 0.0 });
            }
            _ => {
                self.message("La grande clé du manoir. La porte d'entrée vous attend, dans le hall.");
                self.message("Quelque part au-dessus, des vitres explosent. Des pas traînants résonnent dans le manoir.");
            }
        }
    }

    /// Un ennemi vient de recevoir le coup fatal.
    pub(super) fn on_enemy_killed(&mut self, i: usize) {
        match self.st.enemies[i].kind {
            EnemyKind::Brute => self.set_flag(F_BRUTE_DEAD),
            EnemyKind::Hound => self.set_flag(F_HOUND_DEAD),
            _ => {}
        }
    }

    /// Un boss a fini de mourir (animation terminée).
    pub(super) fn on_enemy_finished(&mut self, i: usize) {
        match self.st.enemies[i].kind {
            EnemyKind::Boss => {
                self.set_flag(F_BOSS_DEAD);
                self.message("L'Abomination s'effondre dans la fosse et ne bouge plus.");
                self.message("Au milieu des chairs, quelque chose brille : une grande clé, celle du manoir.");
            }
            EnemyKind::Brute => self.message("Le Sujet 7 s'écroule dans un dernier râle. La porte est de nouveau libre."),
            EnemyKind::Hound => self.message("Le Molosse ne se relèvera plus. Plus rien ne vous sépare de la barque."),
            _ => {}
        }
    }

    /// Raison pour laquelle on ne peut pas quitter la pièce.
    pub(super) fn door_blocked(&self) -> Option<&'static str> {
        let alive = |k: EnemyKind| self.st.enemies.iter().any(|e| e.kind == k && e.alive() && e.room == self.st.room);
        match self.st.room {
            ROOM_CRYPT if self.flag(F_BOSS_INTRO) && alive(EnemyKind::Boss) => Some("Les portes de la crypte se sont scellées d'elles-mêmes. Il faut en finir."),
            ROOM_LAB if self.flag(F_CRYPT_KEY) && alive(EnemyKind::Brute) => Some("La porte est coincée : l'explosion de la cuve a tordu le chambranle !"),
            _ => None,
        }
    }

    // ------------------------------------------------------------ cinématiques

    /// Place l'acteur de la cinématique sans changer de mode.
    fn prepare_cutscene(&mut self, cine: Cine) {
        let room = self.st.room;
        if let Some(i) = self.enemy_of(cine.kind()) {
            let e = &mut self.st.enemies[i];
            e.state = ZState::Intro;
            e.t = 0.0;
            e.room = room;
            match cine {
                Cine::Brute => {
                    e.pos = TANK;
                    e.yaw = -std::f32::consts::FRAC_PI_2;
                }
                Cine::Boss => {
                    e.pos = PIT;
                    e.yaw = PI;
                }
                Cine::Hound => {
                    e.pos = v3(4.0, 0.0, -46.0);
                    e.yaw = 0.0;
                }
            }
            let p = e.pos;
            self.st.enemies[i].pos.y = self.floor_y(room, p);
        }
        self.st.player.aiming = false;
        self.st.player.aim_t = 0.0;
    }

    pub(super) fn start_cutscene(&mut self, cine: Cine) {
        self.prepare_cutscene(cine);
        self.mode = Mode::Cutscene { cine, t: 0.0 };
    }

    pub(super) fn update_cutscene(&mut self, cine: Cine, t: f32, dt: f32, inp: &Input) {
        let t2 = t + dt;
        let crossed = |at: f32| t < at && t2 >= at;
        let Some(i) = self.enemy_of(cine.kind()) else {
            self.mode = Mode::Play;
            return;
        };
        self.st.enemies[i].t = t2;
        match cine {
            Cine::Brute => {
                if crossed(0.4) {
                    self.play(Sfx::Glass, 1.0, 0.3, 1.0);
                    self.shake = 0.6;
                    for _ in 0..60 {
                        let v = v3(self.rng.range(-3.5, 0.5), self.rng.range(0.5, 3.0), self.rng.range(-1.5, 1.5));
                        let life = self.rng.range(0.5, 1.2);
                        let pos = TANK + v3(-0.5, self.rng.range(0.6, 2.4), self.rng.range(-0.5, 0.5));
                        self.particles.push(Particle { pos, vel: v, life, col: [0.55, 0.8, 0.65], size: 1 });
                    }
                }
                if crossed(1.3) {
                    self.play(Sfx::Brute, 1.0, 0.3, 0.9);
                }
                // Il sort de la cuve, d'un pas lourd.
                let e = &mut self.st.enemies[i];
                let k = smoothstep(0.8, 3.0, t2);
                e.pos = TANK + v3(-1.6 * k, 0.0, 0.0);
                e.phase = k * 6.0;
            }
            Cine::Boss => {
                if crossed(0.3) {
                    self.play(Sfx::Thud, 1.0, 0.0, 0.6);
                    self.shake = 0.5;
                }
                if crossed(1.6) || crossed(2.8) {
                    self.play(Sfx::Thud, 0.9, 0.0, 0.8);
                    self.shake = 0.6;
                    let pos = PIT;
                    for k in 0..24 {
                        let a = k as f32 / 24.0 * TAU;
                        let v = v3(a.sin() * 2.0, self.rng.range(1.0, 3.0), a.cos() * 2.0);
                        let life = self.rng.range(0.5, 1.0);
                        self.particles.push(Particle { pos: pos + v3(a.sin() * 1.5, 0.2, a.cos() * 1.5), vel: v, life, col: [0.3, 0.05, 0.05], size: 2 });
                    }
                }
                if crossed(4.2) {
                    self.play(Sfx::Roar, 1.0, 0.0, 0.9);
                    self.shake = 0.9;
                }
            }
            Cine::Hound => {
                if crossed(0.3) {
                    self.play(Sfx::Howl, 1.0, 0.2, 0.62);
                }
                if crossed(2.6) {
                    self.play(Sfx::Snarl, 1.0, 0.2, 0.6);
                }
                let e = &mut self.st.enemies[i];
                let k = smoothstep(2.4, 4.2, t2);
                e.pos = v3(4.0, 0.0, -46.0 + k * 6.0);
                e.pos.y = outdoor::walk_h(e.pos.x, e.pos.z);
                e.phase += dt * 10.0 * k;
            }
        }
        let skip = t2 > 1.0 && (inp.confirm || inp.interact || inp.pause);
        if t2 >= cine.duration() || skip {
            self.end_cutscene(cine, i);
        } else {
            self.mode = Mode::Cutscene { cine, t: t2 };
        }
    }

    fn end_cutscene(&mut self, cine: Cine, i: usize) {
        {
            let e = &mut self.st.enemies[i];
            e.state = ZState::Chase;
            e.t = 0.0;
            e.alerted = true;
            e.attack_cd = 1.0;
            e.roar_t = 6.0;
            match cine {
                Cine::Brute => e.pos = TANK + v3(-1.6, 0.0, 0.0),
                Cine::Hound => e.pos = v3(4.0, outdoor::walk_h(4.0, -40.0), -40.0),
                Cine::Boss => {
                    e.roar_t = 9.0;
                    e.attack_cd = 1.5;
                }
            }
        }
        self.mode = Mode::Play;
        self.fade = 0.6;
        self.update_camera(true);
        if cine == Cine::Boss {
            self.message("Ce qui fut le Docteur Ashcroft se dresse au-dessus de vous, cousu à ses propres expériences.");
            self.message("Sous le plastron d'os, un cœur énorme palpite. La grande clé du manoir y est enfoncée.");
        }
    }

    // ------------------------------------------------------------ interactions

    pub(super) fn press_portrait(&mut self, id: u8) {
        const DESC: [&str; 4] = [
            "« Alice Ashcroft, 1883-1891 ». Une fillette blonde en robe blanche.",
            "« Brutus ». Un grand chien noir au collier clouté. Son regard est presque humain.",
            "« Lady Eleanor Ashcroft ». Une femme en deuil au regard las.",
            "« Lord Ashcroft, docteur en médecine ». Ses yeux semblent suivre les vôtres.",
        ];
        self.message(DESC[id as usize]);
        if self.flag(F_PORTRAITS) {
            if id == 3 {
                self.message("Le portrait a pivoté sur des gonds dissimulés, révélant une niche.");
            }
            return;
        }
        self.message("Le cadre semble mobile. Vous l'enfoncez dans le mur...");
        self.play(Sfx::Reload, 0.5, 0.0, 0.6);
        if id == self.st.puzzle {
            self.st.puzzle += 1;
            if self.st.puzzle >= 3 {
                self.set_flag(F_PORTRAITS);
                self.play(Sfx::Creak, 0.7, 0.4, 0.8);
                self.message("Un déclic sourd. Le portrait du Docteur pivote lentement sur ses gonds. Une niche apparaît derrière.");
            } else {
                self.message("Le cadre reste enfoncé avec un léger déclic.");
            }
        } else {
            self.st.puzzle = 0;
            self.play(Sfx::Locked, 0.5, 0.0, 0.7);
            self.message("Un mécanisme claque : tous les cadres reviennent en place.");
        }
    }

    pub(super) fn talk_edgar(&mut self) {
        if !self.flag(F_EDGAR_MET) {
            self.set_flag(F_EDGAR_MET);
            self.message("Un vieil homme en livrée est adossé au mur, une main crispée sur son flanc ensanglanté.");
            self.message("EDGAR : « Vous... vous n'êtes pas l'une d'eux. Dieu soit loué. Je suis Edgar, le majordome de Lord Ashcroft. »");
            self.message("EDGAR : « Le maître a perdu la raison quand Mademoiselle Alice est morte. Il ramène les morts, en bas, dans la crypte. »");
            self.message("EDGAR : « La porte de la crypte est là, au fond. La clé... il la gardait dans son laboratoire, à l'étage, fermé par la clé au corbeau. »");
            self.message("EDGAR : « Madame disait que la galerie des portraits gardait le secret du maître. Je n'en sais pas plus. »");
            self.message("EDGAR : « Prenez ceci. Et ne vous approchez plus de moi si je... si je change. »");
            self.message("Edgar vous donne : MUNITIONS 9MM (10).");
            self.st.inv.reserve += 10;
            self.play(Sfx::Pickup, 0.5, 0.0, 1.0);
        } else {
            self.message("EDGAR : « Le froid me gagne... Je ne sens plus mes jambes. Dépêchez-vous, je vous en prie. »");
        }
    }

    pub(super) fn music_box(&mut self) {
        self.play(Sfx::MusicBox, 0.8, 0.0, 1.0);
        self.message("Vous remontez la boîte à musique. Une berceuse mélancolique s'élève dans la pièce...");
        self.message("Sous le mécanisme, gravé à la main : « Pour Alice, de Papa. Ton portrait sera toujours le premier. »");
    }

    pub(super) fn board_boat(&mut self) {
        if !self.flag(F_HOUND_DEAD) {
            self.message("Pas le temps de détacher la barque : le Molosse est sur vos talons !");
            return;
        }
        self.play(Sfx::Creak, 0.6, 0.0, 0.7);
        self.play(Sfx::Splash, 0.8, 0.0, 0.8);
        self.mode = Mode::Ending { t: 0.0 };
        self.fade = 1.0;
    }
}
