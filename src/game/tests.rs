use super::*;
use std::f32::consts::FRAC_PI_2;
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
    dismiss(&mut g);
    assert_eq!(g.mode, Mode::Play);
    g
}

fn dismiss(g: &mut Game) {
    for _ in 0..200 {
        if g.mode != Mode::Message {
            return;
        }
        g.frame(DT, &press());
    }
}

/// Ramasse l'objet `kind` de la pièce `room` en se plaçant juste devant.
fn take(g: &mut Game, kind: ItemKind) {
    let i = g.world.items.iter().position(|it| it.kind == kind).expect("objet");
    let it = g.world.items[i].clone();
    let room = &g.world.rooms[it.room];
    // Cherche un point libre d'où l'objet est à portée.
    let mut spot = None;
    'outer: for r in [0.45f32, 0.6, 0.75, 0.9, 1.05, 1.2] {
        for k in 0..16 {
            let a = k as f32 / 16.0 * TAU;
            let p = it.pos.flat() + v3(a.sin(), 0.0, a.cos()) * r;
            if room.collide(p, PLAYER_R).dist_xz(p) < 1e-3 {
                spot = Some(p);
                break 'outer;
            }
        }
    }
    let p = spot.unwrap_or_else(|| panic!("objet {:?} inatteignable", kind));
    g.enter_room(it.room, p, (it.pos - p).flat().yaw());
    g.mode = Mode::Play;
    g.frame(DT, &press());
    assert!(g.st.taken[i], "objet {:?} non ramassé", kind);
    dismiss(g);
}

fn kill(g: &mut Game, kind: EnemyKind) {
    let i = g.st.enemies.iter().position(|e| e.kind == kind).unwrap();
    g.st.enemies[i].room = g.st.room;
    g.damage_enemy(i, combat::Hit { dmg: 999, at: Vec3::ZERO, dir: v3(0.0, 0.0, 1.0), push: 0.0, crit: false });
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
    take(&mut g, ItemKind::Key(KEY_EMBLEM));
    g.enter_room(ROOM_HALL, v3(0.0, 0.0, 13.2), 0.0);
    g.frame(DT, &press());
    dismiss(&mut g);
    assert!(g.st.unlocked[LOCK_LIBRARY]);
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
    let zi = g.st.enemies.iter().position(|z| z.room == ROOM_CORRIDOR).unwrap();
    let mag0 = g.st.inv.mag;
    let aim = Input { aim: true, ..Default::default() };
    let fire = Input { aim: true, fire: true, ..Default::default() };
    for _ in 0..12 {
        run(&mut g, aim, 0.4);
        g.frame(DT, &fire);
        if g.st.enemies[zi].hp <= 0 {
            break;
        }
    }
    run(&mut g, Input::default(), 1.5);
    assert_eq!(g.st.enemies[zi].state, ZState::Dead);
    assert!(g.st.inv.mag < mag0);
    assert_eq!(g.st.kills, 1);
}

#[test]
fn knife_works_without_ammo() {
    let mut g = start();
    g.st.inv.mag = 0;
    g.st.inv.weapon = Weapon::Knife;
    g.enter_room(ROOM_CORRIDOR, v3(7.5, 0.0, 14.4), 0.0);
    let zi = g.st.enemies.iter().position(|z| z.room == ROOM_CORRIDOR).unwrap();
    g.st.enemies[zi].pos = v3(7.5, 0.0, 15.3);
    g.st.enemies[zi].state = ZState::Stagger;
    let hp0 = g.st.enemies[zi].hp;
    let fire = Input { aim: true, fire: true, ..Default::default() };
    run(&mut g, Input { aim: true, ..Default::default() }, 0.3);
    g.frame(DT, &fire);
    assert!(g.st.enemies[zi].hp < hp0, "le couteau doit toucher");
    assert_eq!(g.st.inv.mag, 0);
}

#[test]
fn shotgun_pickup_and_blast() {
    let mut g = start();
    take(&mut g, ItemKind::Shotgun);
    assert!(g.st.inv.has_shotgun);
    assert_eq!(g.st.inv.shells_mag, 4);
    // Équipement depuis l'inventaire.
    g.mode = Mode::Inventory;
    g.sel = 1;
    g.frame(DT, &press());
    assert_eq!(g.st.inv.weapon, Weapon::Shotgun);
    g.mode = Mode::Play;
    // Un zombie à bout portant encaisse 6 points d'un coup.
    g.enter_room(ROOM_CORRIDOR, v3(7.5, 0.0, 13.5), 0.0);
    let zi = g.st.enemies.iter().position(|z| z.room == ROOM_CORRIDOR).unwrap();
    g.st.enemies[zi].pos = v3(7.5, 0.0, 15.0);
    run(&mut g, Input { aim: true, ..Default::default() }, 0.3);
    g.frame(DT, &Input { aim: true, fire: true, ..Default::default() });
    assert_eq!(g.st.inv.shells_mag, 3);
    assert!(g.st.enemies[zi].hp <= 0, "hp = {}", g.st.enemies[zi].hp);
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
    let row = g.inv_slots().iter().position(|s| s.0 == Slot::Medkit).unwrap();
    g.sel = row;
    g.frame(DT, &press());
    assert_eq!(g.st.player.hp, 100);
    assert_eq!(g.st.inv.medkits, 0);
}

#[test]
fn herb_heals_a_little() {
    let mut g = start();
    g.st.player.hp = 30;
    g.st.inv.herbs = 1;
    g.mode = Mode::Inventory;
    g.sel = g.inv_slots().iter().position(|s| s.0 == Slot::Herb).unwrap();
    g.frame(DT, &press());
    assert_eq!(g.st.player.hp, 65);
    assert_eq!(g.st.inv.herbs, 0);
}

#[test]
fn portrait_puzzle_reveals_lab_key() {
    let mut g = start();
    let lab_key = g.world.items.iter().position(|it| it.kind == ItemKind::Key(KEY_LAB)).unwrap();
    assert!(!g.item_visible(lab_key));
    // Mauvais ordre : tout se réarme.
    g.press_portrait(2);
    dismiss(&mut g);
    assert_eq!(g.st.puzzle, 0);
    // Alice, Brutus, Eleanor.
    g.press_portrait(0);
    g.press_portrait(1);
    assert!(!g.flag(F_PORTRAITS));
    g.press_portrait(3);
    assert_eq!(g.st.puzzle, 0, "le Docteur réarme le mécanisme");
    for id in [0, 1, 2] {
        g.press_portrait(id);
    }
    dismiss(&mut g);
    assert!(g.flag(F_PORTRAITS));
    assert!(g.item_visible(lab_key));
    take(&mut g, ItemKind::Key(KEY_LAB));
}

#[test]
fn crypt_key_wakes_brute_and_edgar() {
    let mut g = start();
    let edgar = g.world.enemies.iter().position(|e| e.look == 3).unwrap();
    assert_eq!(g.st.enemies[edgar].room, NOWHERE);
    take(&mut g, ItemKind::Key(KEY_CRYPT));
    // Après le message, la cinématique du Sujet 7.
    assert!(matches!(g.mode, Mode::Cutscene { cine: Cine::Brute, .. }), "{:?}", g.mode);
    run(&mut g, Input::default(), 4.0);
    assert_eq!(g.mode, Mode::Play);
    let brute = g.st.enemies.iter().position(|e| e.kind == EnemyKind::Brute).unwrap();
    assert_eq!(g.st.enemies[brute].room, ROOM_LAB);
    assert_eq!(g.st.enemies[brute].state, ZState::Chase);
    assert_eq!(g.st.enemies[edgar].room, ROOM_KITCHEN);
    // On ne quitte pas le labo tant qu'il vit.
    assert!(g.door_blocked().is_some());
    kill(&mut g, EnemyKind::Brute);
    assert!(g.flag(F_BRUTE_DEAD));
    assert!(g.door_blocked().is_none());
}

#[test]
fn brute_charge_knocks_down() {
    let mut g = start();
    g.enter_room(ROOM_LAB, v3(13.0, 0.0, 23.0), FRAC_PI_2);
    g.set_flag(F_CRYPT_KEY);
    let i = g.st.enemies.iter().position(|e| e.kind == EnemyKind::Brute).unwrap();
    {
        let e = &mut g.st.enemies[i];
        e.state = ZState::Charge;
        e.t = 0.0;
        e.pos = v3(18.5, 0.0, 23.0);
        e.yaw = -FRAC_PI_2;
        e.alerted = true;
    }
    run(&mut g, Input::default(), 2.0);
    assert!(g.st.player.hp <= 76, "hp = {}", g.st.player.hp);
    // Projetée au sol puis relevée.
    run(&mut g, Input::default(), 2.0);
    assert_eq!(g.st.player.knock, 0.0);
}

#[test]
fn boss_fight_and_front_door() {
    let mut g = start();
    g.set_flag(F_CRYPT_KEY);
    g.st.unlocked[LOCK_CRYPT] = true;
    // Entrée dans la crypte : cinématique, puis portes scellées.
    g.enter_room(ROOM_CRYPT_STAIRS, v3(-7.5, 0.0, 25.1), 0.0);
    g.mode = Mode::Play;
    g.frame(DT, &press());
    assert!(matches!(g.mode, Mode::Door { .. }));
    run(&mut g, Input::default(), 2.4);
    assert_eq!(g.st.room, ROOM_CRYPT);
    assert!(matches!(g.mode, Mode::Cutscene { cine: Cine::Boss, .. }));
    run(&mut g, Input::default(), 6.5);
    dismiss(&mut g);
    assert_eq!(g.mode, Mode::Play);
    let b = g.st.enemies.iter().position(|e| e.kind == EnemyKind::Boss).unwrap();
    assert!(g.boss_active().is_some());
    assert!(g.door_blocked().is_some());
    // Le cœur à découvert triple les dégâts.
    g.st.enemies[b].state = ZState::Roar;
    g.st.enemies[b].t = 1.0;
    let hp0 = g.st.enemies[b].hp;
    g.damage_enemy(b, combat::Hit { dmg: 1, at: Vec3::ZERO, dir: v3(0.0, 0.0, 1.0), push: 0.0, crit: false });
    assert_eq!(g.st.enemies[b].hp, hp0 - 3);
    // Mort : la clé tombe dans la fosse une fois l'animation finie.
    kill(&mut g, EnemyKind::Boss);
    assert!(!g.flag(F_BOSS_DEAD));
    g.st.player.pos = v3(-14.0, 0.0, 27.5);
    run(&mut g, Input::default(), 5.0);
    dismiss(&mut g);
    assert!(g.flag(F_BOSS_DEAD));
    take(&mut g, ItemKind::Key(KEY_FRONT));
    // La porte d'entrée s'ouvre sur le parc.
    g.enter_room(ROOM_HALL, v3(0.0, 0.0, 0.8), PI);
    g.mode = Mode::Play;
    g.frame(DT, &press());
    dismiss(&mut g);
    assert!(g.st.unlocked[LOCK_FRONT]);
    g.frame(DT, &press());
    run(&mut g, Input::default(), 2.4);
    dismiss(&mut g);
    assert_eq!(g.st.room, ROOM_PORCH);
}

#[test]
fn hound_guards_the_boat_then_ending() {
    let mut g = start();
    g.enter_room(ROOM_MARSH, v3(0.0, 0.0, -29.0), PI);
    g.mode = Mode::Play;
    for e in &mut g.st.enemies {
        if e.kind == EnemyKind::Dog {
            e.room = NOWHERE;
        }
    }
    run(&mut g, Input { mv: (0.0, 1.0), ..Default::default() }, 1.5);
    assert!(g.flag(F_HOUND_INTRO));
    assert!(matches!(g.mode, Mode::Cutscene { cine: Cine::Hound, .. }));
    run(&mut g, Input::default(), 4.5);
    assert_eq!(g.mode, Mode::Play);
    // La barque refuse tant que le Molosse vit.
    g.board_boat();
    assert_eq!(g.mode, Mode::Message);
    dismiss(&mut g);
    kill(&mut g, EnemyKind::Hound);
    assert!(g.flag(F_HOUND_DEAD));
    g.mode = Mode::Play;
    g.board_boat();
    assert!(matches!(g.mode, Mode::Ending { .. }));
    run(&mut g, Input::default(), story::ENDING_LEN + 0.5);
    g.frame(DT, &press());
    assert_eq!(g.mode, Mode::Title);
}

#[test]
fn dogs_leap_and_bite() {
    let mut g = start();
    g.enter_room(ROOM_PORCH, v3(0.0, 0.0, -3.0), PI);
    g.mode = Mode::Play;
    run(&mut g, Input::default(), 6.0);
    dismiss(&mut g);
    run(&mut g, Input::default(), 4.0);
    assert!(g.st.player.hp < 100, "hp = {}", g.st.player.hp);
}

#[test]
fn edgar_gives_ammo_once() {
    let mut g = start();
    g.enter_room(ROOM_KITCHEN, world::EDGAR_POS + v3(1.1, 0.0, 0.0), -FRAC_PI_2);
    g.mode = Mode::Play;
    let r0 = g.st.inv.reserve;
    g.frame(DT, &press());
    dismiss(&mut g);
    assert_eq!(g.st.inv.reserve, r0 + 10);
    assert!(g.flag(F_EDGAR_MET));
    g.frame(DT, &press());
    dismiss(&mut g);
    assert_eq!(g.st.inv.reserve, r0 + 10);
}

/// Chaque porte mène à une position libre de la pièce cible, près d'une
/// porte qui ramène à la pièce de départ.
#[test]
fn doors_are_consistent() {
    let g = start();
    for (ri, room) in g.world.rooms.iter().enumerate() {
        for d in &room.doors {
            let to = &g.world.rooms[d.to_room];
            let p = d.to_pos;
            assert!(p.x > to.min.0 && p.x < to.max.0 && p.z > to.min.1 && p.z < to.max.1, "{} -> {} : arrivée hors de la pièce", room.name, to.name);
            assert!(to.collide(p, PLAYER_R).dist_xz(p) < 0.05, "{} -> {} : arrivée dans un obstacle", room.name, to.name);
            let back = to.doors.iter().any(|b| b.to_room == ri && b.pos.dist_xz(p) < 1.3);
            assert!(back, "{} -> {} : pas de porte de retour", room.name, to.name);
            // On peut se tenir devant la porte pour l'ouvrir.
            let stand = d.pos + d.normal * 0.6;
            assert!(room.collide(stand, PLAYER_R).dist_xz(d.pos) < 1.3, "{} : porte vers {} inaccessible", room.name, to.name);
        }
    }
}

/// Tous les objets du jeu sont ramassables.
#[test]
fn every_item_is_reachable() {
    let mut g = start();
    for f in 0..NFLAGS {
        g.set_flag(f);
    }
    for it in g.world.items.clone() {
        take(&mut g, it.kind);
    }
}

/// Économie : assez de munitions pour tout tuer, mais sans gaspiller.
#[test]
fn ammo_budget() {
    let g = start();
    let mut pistol = 12 + 10; // chargeur initial + cadeau d'Edgar
    let mut shells = 4;
    for it in &g.world.items {
        match it.kind {
            ItemKind::Ammo(n) => pistol += n as i32,
            ItemKind::Shells(n) => shells += n as i32,
            _ => {}
        }
    }
    let hp: i32 = g.st.enemies.iter().map(|e| e.max_hp).sum();
    // Le fusil vaut en moyenne quatre balles par cartouche.
    let budget = pistol + shells * 4;
    assert!(budget > hp, "pas assez de munitions : {budget} pour {hp} PV");
    assert!(budget < hp * 3 / 2, "trop de munitions : {budget} pour {hp} PV");
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
    g.st.inv.has_shotgun = true;
    g.st.inv.shells = 5;
    g.st.inv.weapon = Weapon::Shotgun;
    g.st.taken[0] = true;
    g.st.enemies[0].state = ZState::Dead;
    g.set_flag(F_PORTRAITS);
    g.set_flag(F_CRYPT_KEY);
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
    let row = g.option_rows().iter().position(|&o| o == menu::OptPub::Fps).unwrap();
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
    assert_eq!(g.st.inv.shells, 5);
    assert_eq!(g.st.inv.weapon, Weapon::Shotgun);
    assert!(g.st.taken[0]);
    assert_eq!(g.st.enemies[0].state, ZState::Dead);
    assert!(g.flag(F_PORTRAITS) && g.flag(F_CRYPT_KEY));
    assert_eq!(g.st.saves, 1);
    let brute = g.st.enemies.iter().position(|e| e.kind == EnemyKind::Brute).unwrap();
    assert_eq!(g.st.enemies[brute].room, ROOM_LAB);
    assert!(g.st.player.pos.dist_xz(v3(-4.6, 0.0, 10.5)) < 0.01);
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
