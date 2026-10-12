//! Sérialisation texte de la sauvegarde (machine à écrire).

use super::*;

const MAGIC: &str = "MDM2";

fn zstate_code(s: ZState) -> u32 {
    match s {
        ZState::Eating => 0,
        ZState::Idle => 1,
        ZState::Dying | ZState::Dead => 3,
        ZState::Intro => 4,
        ZState::Lying => 5,
        _ => 2,
    }
}

fn zstate_from(c: u32) -> ZState {
    match c {
        0 => ZState::Eating,
        1 => ZState::Idle,
        2 => ZState::Chase,
        4 => ZState::Intro,
        5 => ZState::Lying,
        _ => ZState::Dead,
    }
}

fn weapon_code(w: Weapon) -> u32 {
    match w {
        Weapon::Knife => 0,
        Weapon::Pistol => 1,
        Weapon::Shotgun => 2,
    }
}

pub(super) fn encode(st: &State) -> String {
    let mut o: Vec<String> = vec![MAGIC.to_string()];
    let mut f = |v: f32| o.push(format!("{v:.3}"));
    f(st.room as f32);
    f(st.player.pos.x);
    f(st.player.pos.z);
    f(st.player.yaw);
    f(st.player.hp as f32);
    let inv = &st.inv;
    f(weapon_code(inv.weapon) as f32);
    f(inv.mag as f32);
    f(inv.reserve as f32);
    f(inv.has_shotgun as u8 as f32);
    f(inv.shells_mag as f32);
    f(inv.shells as f32);
    f(inv.medkits as f32);
    f(inv.herbs as f32);
    f(inv.keys.len() as f32);
    for &k in &inv.keys {
        f(k as f32);
    }
    for list in [&st.taken, &st.unlocked, &st.flags] {
        f(list.len() as f32);
        for &b in list.iter() {
            f(b as u8 as f32);
        }
    }
    f(st.puzzle as f32);
    f(st.play_time);
    f(st.saves as f32);
    f(st.kills as f32);
    f(st.enemies.len() as f32);
    for z in &st.enemies {
        f(if z.room == NOWHERE { -1.0 } else { z.room as f32 });
        f(z.pos.x);
        f(z.pos.z);
        f(z.yaw);
        f(z.hp as f32);
        f(zstate_code(z.state) as f32);
        f(z.alerted as u8 as f32);
        f(z.fall_back as u8 as f32);
    }
    f(st.blood.len() as f32);
    for b in &st.blood {
        f(b.0 as f32);
        f(b.1.x);
        f(b.1.z);
        f(b.3);
    }
    o.join(" ")
}

pub(super) fn decode(s: &str, world: &World) -> Option<State> {
    let mut it = s.split_whitespace();
    if it.next()? != MAGIC {
        return None;
    }
    let mut nums = it.map(|t| t.parse::<f32>());
    let mut n = || -> Option<f32> { nums.next()?.ok() };
    let mut st = initial_state(world);
    st.room = n()? as usize;
    if st.room >= world.rooms.len() {
        return None;
    }
    let (x, z, yaw) = (n()?, n()?, n()?);
    st.player.pos = v3(x, 0.0, z);
    if world.rooms[st.room].outdoor {
        st.player.pos.y = outdoor::walk_h(x, z);
    }
    st.player.yaw = yaw;
    st.player.hp = n()? as i32;
    st.inv.weapon = match n()? as u32 {
        0 => Weapon::Knife,
        2 => Weapon::Shotgun,
        _ => Weapon::Pistol,
    };
    st.inv.mag = n()? as u32;
    st.inv.reserve = n()? as u32;
    st.inv.has_shotgun = n()? != 0.0;
    st.inv.shells_mag = n()? as u32;
    st.inv.shells = n()? as u32;
    st.inv.medkits = n()? as u32;
    st.inv.herbs = n()? as u32;
    let nk = n()? as usize;
    st.inv.keys = (0..nk).map(|_| n().map(|k| k as u8)).collect::<Option<_>>()?;
    for list in [&mut st.taken, &mut st.unlocked, &mut st.flags] {
        if n()? as usize != list.len() {
            return None;
        }
        for b in list.iter_mut() {
            *b = n()? != 0.0;
        }
    }
    st.puzzle = n()? as u8;
    st.play_time = n()?;
    st.saves = n()? as u32;
    st.kills = n()? as u32;
    if n()? as usize != st.enemies.len() {
        return None;
    }
    for zb in st.enemies.iter_mut() {
        let room = n()?;
        zb.room = if room < 0.0 { NOWHERE } else { room as usize };
        let (x, z) = (n()?, n()?);
        zb.pos = v3(x, 0.0, z);
        if zb.room < world.rooms.len() && world.rooms[zb.room].outdoor {
            zb.pos.y = outdoor::walk_h(x, z);
        }
        zb.yaw = n()?;
        zb.hp = n()? as i32;
        zb.state = zstate_from(n()? as u32);
        zb.alerted = n()? != 0.0;
        zb.fall_back = n()? != 0.0;
    }
    let nb = n()? as usize;
    st.blood = (0..nb)
        .map(|_| {
            let room = n()? as usize;
            let p = v3(n()?, 0.0, n()?);
            let max = n()?;
            Some((room, p, max, max))
        })
        .collect::<Option<_>>()?;
    Some(st)
}
