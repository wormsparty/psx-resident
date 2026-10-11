//! Sérialisation texte de la sauvegarde (machine à écrire).

use super::*;

const MAGIC: &str = "MDM1";

fn zstate_code(s: ZState) -> u32 {
    match s {
        ZState::Eating => 0,
        ZState::Idle => 1,
        ZState::Rising | ZState::Chase | ZState::Grab | ZState::Stagger => 2,
        ZState::Dying | ZState::Dead => 3,
    }
}

fn zstate_from(c: u32) -> ZState {
    match c {
        0 => ZState::Eating,
        1 => ZState::Idle,
        2 => ZState::Chase,
        _ => ZState::Dead,
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
    f(st.inv.mag as f32);
    f(st.inv.reserve as f32);
    f(st.inv.medkits as f32);
    f(st.inv.keys.len() as f32);
    for &k in &st.inv.keys {
        f(k as f32);
    }
    f(st.taken.len() as f32);
    for &t in &st.taken {
        f(t as u8 as f32);
    }
    f(st.unlocked.len() as f32);
    for &u in &st.unlocked {
        f(u as u8 as f32);
    }
    f(st.zombies.len() as f32);
    for z in &st.zombies {
        f(z.room as f32);
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
    st.player.yaw = yaw;
    st.player.hp = n()? as i32;
    st.inv.mag = n()? as u32;
    st.inv.reserve = n()? as u32;
    st.inv.medkits = n()? as u32;
    let nk = n()? as usize;
    st.inv.keys = (0..nk).map(|_| n().map(|k| k as u8)).collect::<Option<_>>()?;
    if n()? as usize != st.taken.len() {
        return None;
    }
    for t in st.taken.iter_mut() {
        *t = n()? != 0.0;
    }
    if n()? as usize != st.unlocked.len() {
        return None;
    }
    for u in st.unlocked.iter_mut() {
        *u = n()? != 0.0;
    }
    if n()? as usize != st.zombies.len() {
        return None;
    }
    for zb in st.zombies.iter_mut() {
        zb.room = n()? as usize;
        zb.pos = v3(n()?, 0.0, n()?);
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
            let r = n()?;
            let max = n()?;
            Some((room, p, r, max))
        })
        .collect::<Option<_>>()?;
    Some(st)
}
