//! Dehors, sous la pleine lune : la cour devant le perron, puis le marais
//! jusqu'au ponton où attend une barque.

use super::*;
use crate::outdoor::{terrain_y, PIER, PIER_Y};

/// Brume bleutée du plein air.
const MIST: Fog = Fog { start: 5.0, end: 36.0, color: crate::outdoor::HAZE };

fn ground(x: f32, z: f32) -> Vec3 {
    v3(x, terrain_y(x, z), z)
}

/// Lanterne sur un poteau ou un pilier, avec sa lumière.
fn lantern(b: &mut Builder, top: Vec3, post: bool) {
    let iron = Mat::col([0.08, 0.08, 0.09]);
    if post {
        let g = ground(top.x, top.z);
        b.mesh.cube(v3(top.x - 0.06, g.y - 0.2, top.z - 0.06), v3(top.x + 0.06, top.y, top.z + 0.06), &Mat::tex(T_BARK, [0.6, 0.5, 0.45]), 1.0);
        b.obstacle(top.x - 0.15, top.z - 0.15, top.x + 0.15, top.z + 0.15, true);
    }
    b.mesh.cube(top + v3(-0.14, 0.0, -0.14), top + v3(0.14, 0.05, 0.14), &iron, 1.0);
    b.mesh.cube(top + v3(-0.11, 0.05, -0.11), top + v3(0.11, 0.35, 0.11), &Mat::col([0.9, 0.65, 0.3]).glow(1.2), 1.0);
    b.mesh.cube(top + v3(-0.15, 0.35, -0.15), top + v3(0.15, 0.42, 0.15), &iron, 1.0);
    b.flames.push((top + v3(0.0, 0.15, 0.0), 0.06));
    b.light(top + v3(0.0, 0.2, 0.0), [1.0, 0.65, 0.3], 7.5, 0.15);
}

fn outdoor_room(b: Builder, name: &'static str, cameras: Vec<CamZone>, doors: Vec<Door>, corpses: Vec<(Vec3, f32)>) -> Room {
    let mut r = b.build(name, cameras, doors, [0.05, 0.055, 0.075], corpses);
    r.outdoor = true;
    r.fog = MIST;
    r
}

pub fn porch() -> Room {
    let mut b = Builder::new((-8.0, -8.7), (8.0, -0.65), 4.0);
    // Lanternes du portail.
    for x in [-3.2f32, 3.2] {
        lantern(&mut b, v3(x, 2.6, -9.0), false);
    }
    // Lanternes du perron.
    for x in [-1.85f32, 1.85] {
        lantern(&mut b, v3(x, 2.4, -1.25), false);
    }
    // Fontaine asséchée et sa statue d'ange.
    let stone = Mat::tex(T_STONE, [0.55, 0.55, 0.58]);
    let (fx, fz) = (4.7f32, -5.0f32);
    let y0 = crate::outdoor::COURT_Y;
    for (a, bb) in [((-1.3f32, -1.3f32), (1.3f32, -1.0f32)), ((-1.3, 1.0), (1.3, 1.3)), ((-1.3, -1.0), (-1.0, 1.0)), ((1.0, -1.0), (1.3, 1.0))] {
        b.mesh.cube(v3(fx + a.0, y0, fz + a.1), v3(fx + bb.0, y0 + 0.55, fz + bb.1), &stone, 1.0);
    }
    b.mesh.quad([v3(fx - 1.0, y0 + 0.2, fz - 1.0), v3(fx + 1.0, y0 + 0.2, fz - 1.0), v3(fx + 1.0, y0 + 0.2, fz + 1.0), v3(fx - 1.0, y0 + 0.2, fz + 1.0)], [[0.0; 2]; 4], &Mat::col([0.05, 0.07, 0.06]), Vec3::UP);
    b.mesh.cube(v3(fx - 0.25, y0, fz - 0.25), v3(fx + 0.25, y0 + 1.6, fz + 0.25), &stone, 1.0);
    b.mesh.cube(v3(fx - 0.18, y0 + 1.6, fz - 0.15), v3(fx + 0.18, y0 + 2.4, fz + 0.15), &stone, 2.0);
    b.mesh.cube(v3(fx - 0.11, y0 + 2.4, fz - 0.1), v3(fx + 0.11, y0 + 2.65, fz + 0.1), &stone, 2.0);
    for s in [-1.0f32, 1.0] {
        b.mesh.quad([v3(fx + s * 0.15, y0 + 2.3, fz + 0.1), v3(fx + s * 0.75, y0 + 2.6, fz + 0.25), v3(fx + s * 0.55, y0 + 1.7, fz + 0.2), v3(fx + s * 0.15, y0 + 1.8, fz + 0.1)], [[0.0; 2]; 4], &stone.two(), v3(0.0, 0.0, -1.0));
    }
    b.obstacle(fx - 1.3, fz - 1.3, fx + 1.3, fz + 1.3, false);
    b.msg(fx - 1.7, fz, 0.9, "Une fontaine asséchée. L'ange de pierre a le visage griffé jusqu'à l'os.");
    // Haies taillées.
    let hedge = Mat::tex(T_GRASS, [0.55, 0.75, 0.5]);
    for (x0, x1) in [(-8.0f32, -7.0f32), (7.0, 8.0)] {
        for (z0, z1) in [(-8.4f32, -5.6f32), (-4.2, -1.2)] {
            b.mesh.cube(v3(x0, y0, z0), v3(x1, y0 + 1.4, z1), &hedge, 1.0);
            b.obstacle(x0, z0, x1, z1, true);
        }
    }
    // Banc de pierre.
    b.mesh.cube(v3(-5.6, y0 + 0.4, -6.8), v3(-3.8, y0 + 0.5, -6.3), &stone, 1.0);
    for x in [-5.5f32, -4.1] {
        b.mesh.cube(v3(x, y0, -6.75), v3(x + 0.2, y0 + 0.4, -6.35), &stone, 1.0);
    }
    b.obstacle(-5.6, -6.8, -3.8, -6.3, false);
    // Vasques du perron.
    for x in [-2.6f32, 2.3] {
        b.mesh.cube(v3(x, y0, -1.0), v3(x + 0.3, y0 + 0.9, -0.7), &stone, 1.0);
        b.obstacle(x, -1.0, x + 0.3, -0.7, true);
    }
    b.msg(-1.8, -6.9, 0.9, "Le gardien du domaine. Les morsures sont énormes. Il tient encore son fusil... vide.");
    let cameras = vec![
        // Près du perron : la façade et la porte, vues de biais.
        cam((-BIG, -3.8), (1.5, BIG), v3(3.4, 2.3, -8.3), v3(-1.0, 1.4, 0.0), 64.0),
        cam((1.5, -3.8), (BIG, BIG), v3(-3.4, 2.3, -8.3), v3(1.5, 1.4, 0.0), 64.0),
        // Dans la cour : vers le portail, la lune et le marais.
        cam((-BIG, -BIG), (-3.0, -3.8), v3(4.5, 2.8, -1.2), v3(-6.5, 0.0, -7.5), 66.0),
        cam((-3.0, -BIG), (3.0, -3.8), v3(0.0, 2.9, -0.9), v3(0.4, 0.0, -9.5), 72.0),
        cam((3.0, -BIG), (BIG, -3.8), v3(-4.5, 2.8, -1.2), v3(6.5, 0.0, -7.5), 66.0),
    ];
    let doors = vec![
        door(v3(0.0, 0.0, -0.63), v3(0.0, 0.0, -1.0), ROOM_HALL, v3(0.0, 0.0, 0.9), 0.0, Some((LOCK_FRONT, KEY_FRONT))),
        door(v3(0.0, 0.0, -8.7), v3(0.0, 0.0, 1.0), ROOM_MARSH, v3(0.0, 0.0, -10.0), PI, None).gate(),
    ];
    outdoor_room(b, "LE PARC", cameras, doors, vec![(v3(-1.8, 0.0, -7.6), 0.7)])
}

pub fn marsh() -> Room {
    let mut b = Builder::new((-7.0, -48.0), (9.0, -9.3), 4.0);
    for p in [(2.6f32, -16.5f32), (-4.6, -27.0), (5.4, -37.5)] {
        let g = ground(p.0, p.1);
        lantern(&mut b, v3(p.0, g.y + 2.2, p.1), true);
    }
    // Panneau.
    let wood = Mat::tex(T_WOOD, [0.5, 0.42, 0.35]);
    let g = ground(-2.4, -12.4);
    b.mesh.cube(v3(-2.45, g.y - 0.2, -12.45), v3(-2.35, g.y + 1.6, -12.35), &wood, 1.0);
    b.mesh.cube(v3(-2.8, g.y + 1.2, -12.5), v3(-1.7, g.y + 1.5, -12.42), &wood, 1.0);
    b.obstacle(-2.55, -12.55, -2.25, -12.25, false);
    b.msg(-2.4, -13.0, 0.8, "Un panneau vermoulu : « EMBARCADÈRE ». La flèche pointe vers le sud, dans la brume.");
    // Cabane en ruine (ouverte vers le chemin, à l'ouest).
    let planks = Mat::tex(T_WOOD, [0.38, 0.32, 0.28]).two();
    let gs = ground(5.3, -23.1).y;
    let (x0, x1, z0, z1) = (3.6f32, 7.2f32, -24.8f32, -21.4f32);
    b.mesh.wall(v3(x0 + 1.4, gs - 0.3, z0), v3(x1 - x0 - 1.4, 0.0, 0.0), Vec3::UP * 2.3, 1.0, 0.0, &planks, v3(0.0, 0.0, 1.0));
    // Mur nord à moitié effondré.
    b.mesh.wall(v3(x0, gs - 0.3, z1), v3(x1 - x0, 0.0, 0.0), Vec3::UP * 1.2, 1.0, 0.0, &planks, v3(0.0, 0.0, -1.0));
    b.mesh.wall(v3(x1, gs - 0.3, z0), v3(0.0, 0.0, z1 - z0), Vec3::UP * 2.3, 1.0, 0.0, &planks, v3(-1.0, 0.0, 0.0));
    // Le toit s'est effondré : quelques planches au sol.
    for (k, (x, z)) in [(4.4f32, -22.4f32), (5.6, -23.8), (6.2, -22.2)].iter().enumerate() {
        let a = k as f32 * 1.3;
        let d = v3(a.cos(), 0.0, a.sin()) * 0.9;
        let w = v3(-a.sin(), 0.0, a.cos()) * 0.15;
        let c = v3(*x, gs + 0.03, *z);
        b.mesh.quad([c - d - w, c + d - w, c + d + w, c - d + w], [[0.0, 0.0], [2.0, 0.0], [2.0, 0.3], [0.0, 0.3]], &planks, Vec3::UP);
    }
    b.obstacle(x0 + 1.4, z0, x1, z0 + 0.15, true);
    b.obstacle(x0, z1 - 0.15, x1, z1, true);
    b.obstacle(x1 - 0.15, z0, x1, z1, true);
    b.msg(5.8, -23.1, 0.8, "La cabane du passeur. Un filet de pêche, des cordes... et des chaînes à chien arrachées du mur.");
    // Charrette renversée.
    let gc = ground(-4.4, -14.8).y;
    b.mesh.cube(v3(-5.4, gc + 0.3, -15.5), v3(-3.4, gc + 0.75, -14.1), &wood, 1.0);
    let wheel = Mat::col([0.18, 0.13, 0.1]);
    b.mesh.cube(v3(-5.2, gc - 0.1, -15.65), v3(-4.2, gc + 0.9, -15.55), &wheel, 1.0);
    b.mesh.cube(v3(-3.4, gc + 0.4, -14.85), v3(-2.2, gc + 0.5, -14.75), &wood, 1.0);
    b.obstacle(-5.4, -15.65, -3.4, -14.1, false);
    // Petit cimetière.
    let stone = Mat::tex(T_STONE, [0.5, 0.52, 0.55]);
    for (k, (x, z)) in [(-4.4f32, -29.6f32), (-6.0, -29.2), (-5.4, -31.2), (-6.3, -32.8), (-4.8, -33.0)].iter().enumerate() {
        let gy = ground(*x, *z).y;
        let lean = Mat4::translate(v3(*x, gy - 0.1, *z)) * Mat4::rot_z((k as f32 * 1.7).sin() * 0.15) * Mat4::rot_x((k as f32 * 2.3).cos() * 0.1);
        let mut t = Mesh::new();
        t.cube(v3(-0.3, 0.0, -0.08), v3(0.3, 0.9, 0.08), &stone, 1.0);
        t.cube(v3(-0.2, 0.9, -0.08), v3(0.2, 1.05, 0.08), &stone, 1.0);
        b.mesh.append(&t, &lean);
        b.obstacle(x - 0.3, z - 0.15, x + 0.3, z + 0.15, true);
    }
    b.msg(-4.0, -31.0, 0.8, "Des tombes de domestiques. Elles ont toutes été ouvertes. De l'intérieur.");
    // Ponton et barque.
    let deck = Mat::tex(T_WOOD, [0.45, 0.4, 0.35]);
    let (px0, pz0, px1, pz1) = PIER;
    b.mesh.grid(v3(px0, PIER_Y, pz0), v3(px1 - px0, 0.0, 0.0), v3(0.0, 0.0, pz1 - pz0), [0.0, 0.0], [1.0, 3.0], 1.0, &deck, Vec3::UP);
    b.mesh.grid(v3(px0, PIER_Y - 0.15, pz1), v3(px1 - px0, 0.0, 0.0), v3(0.0, 0.15, 0.0), [0.0, 0.0], [1.0, 0.1], 2.0, &deck, v3(0.0, 0.0, 1.0));
    for z in [pz0 + 0.1, (pz0 + pz1) * 0.5, pz1 - 0.2] {
        for x in [px0 - 0.1, px1 - 0.05] {
            b.mesh.cube(v3(x, -1.2, z), v3(x + 0.15, PIER_Y + 0.6, z + 0.15), &Mat::tex(T_BARK, [0.5, 0.45, 0.4]), 1.0);
        }
    }
    lantern(&mut b, v3(px1 + 0.03, PIER_Y + 0.6, pz0 + 0.18), false);
    let hull = Mat::tex(T_WOOD, [0.35, 0.3, 0.28]);
    let (bx, bz) = (6.0f32, -45.6f32);
    b.mesh.cube_faces(v3(bx - 0.5, -0.8, bz - 1.3), v3(bx + 0.5, -0.45, bz + 1.3), &hull, 1.0, [true, true, false, false, true, true]);
    b.mesh.quad([v3(bx - 0.45, -0.7, bz - 1.25), v3(bx + 0.45, -0.7, bz - 1.25), v3(bx + 0.45, -0.7, bz + 1.25), v3(bx - 0.45, -0.7, bz + 1.25)], [[0.0; 2]; 4], &Mat::col([0.08, 0.06, 0.05]), Vec3::UP);
    b.mesh.cube(v3(bx - 0.45, -0.62, bz - 0.1), v3(bx + 0.45, -0.55, bz + 0.15), &hull, 1.0);
    b.obstacle(px1, bz - 1.3, bx + 0.6, bz + 1.3, false);
    b.interacts.push(Interact { pos: v3(px1 - 0.3, 0.0, bz), radius: 1.1, kind: InteractKind::Boat });
    let cameras = vec![
        // Sous le portail : le manoir se dresse derrière la grille.
        cam((-BIG, -16.0), (BIG, BIG), v3(1.6, 3.0, -23.5), v3(0.5, 1.2, -9.0), 70.0),
        cam((-BIG, -24.0), (BIG, -16.0), v3(-5.5, 3.2, -11.0), v3(2.0, 0.0, -24.0), 66.0),
        cam((-BIG, -32.0), (BIG, -24.0), v3(-6.5, 3.4, -20.0), v3(3.0, 0.0, -31.0), 66.0),
        cam((-BIG, -40.0), (BIG, -32.0), v3(-5.5, 3.4, -27.0), v3(3.5, 0.0, -40.0), 66.0),
        // Le ponton, la lune au-dessus de l'eau.
        cam((-BIG, -BIG), (BIG, -40.0), v3(-2.5, 2.8, -36.0), v3(4.0, 0.2, -46.5), 64.0),
    ];
    // Eau trop profonde au sud-ouest du ponton, roseaux derrière la cabane.
    b.obstacle(-7.0, -48.0, -3.0, -40.0, false);
    b.obstacle(7.2, -26.5, 9.0, -20.0, true);
    let doors = vec![door(v3(0.0, 0.0, -9.3), v3(0.0, 0.0, -1.0), ROOM_PORCH, v3(0.0, 0.0, -7.9), 0.0, None).gate()];
    outdoor_room(b, "LE MARAIS", cameras, doors, vec![(v3(-4.4, 0.0, -34.8), 2.2)])
}

pub fn items() -> Vec<ItemSpawn> {
    use ItemKind::*;
    let on = |x: f32, z: f32, dy: f32| v3(x, terrain_y(x, z) + dy, z);
    vec![
        item(ROOM_PORCH, on(-4.7, -6.55, 0.5), Ammo(10)),
        item(ROOM_PORCH, on(-1.2, -7.4, 0.0), Shells(3)),
        item(ROOM_MARSH, on(5.0, -23.2, 0.0), Shells(4)),
        item(ROOM_MARSH, on(-4.2, -14.8, 0.75), Ammo(15)),
        item(ROOM_MARSH, on(-5.6, -30.4, 0.0), Herb),
    ]
}

pub fn enemies() -> Vec<EnemySpawn> {
    let dog = |room, x, z, yaw| enemy(room, x, z, yaw, EnemyKind::Dog);
    vec![
        EnemySpawn { state: SpawnState::Eating, ..dog(ROOM_PORCH, -2.5, -7.0, 2.2) },
        dog(ROOM_PORCH, 5.0, -8.0, -0.5),
        dog(ROOM_MARSH, -5.5, -19.0, 0.5),
        dog(ROOM_MARSH, 6.5, -28.0, -1.0),
        EnemySpawn { state: SpawnState::Eating, ..dog(ROOM_MARSH, -5.2, -34.2, 2.0) },
        EnemySpawn { wake: Some(F_HOUND_INTRO), ..enemy(ROOM_MARSH, 4.0, -42.0, PI, EnemyKind::Hound) },
    ]
}
