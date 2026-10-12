//! La cuisine (où se terre Edgar, le majordome), l'escalier de la crypte
//! et la crypte des Ashcroft, antre du Docteur.

use super::*;

pub const NOTE_COOK: &[&str] = &[
    "Une note du cuisinier, punaisée près des crochets :",
    "« Le Docteur exige que les chiens soient nourris avec ce qu'il descend du laboratoire. Je refuse de savoir ce que c'est. »",
    "« Depuis, ils ne dorment plus. Ils tournent autour du manoir toute la nuit, sous la pleine lune, et ils ne mangent plus que de la viande crue. »",
];

/// Position d'Edgar, assis contre le mur ouest de la cuisine.
pub const EDGAR_POS: Vec3 = v3(-17.45, 0.0, 15.3);

/// Centre de la fosse rituelle de la crypte (le boss en surgit, la clé y tombe).
pub const PIT: Vec3 = v3(-7.5, 0.0, 35.5);

pub fn kitchen() -> Room {
    let mut b = Builder::new((-18.0, 12.0), (-9.0, 20.0), 3.2);
    let style = Style {
        lower: Mat::tex(T_TILE, [0.9, 0.88, 0.82]),
        upper: Mat::tex(T_STONE, [0.72, 0.68, 0.62]),
        floor: Mat::tex(T_TILE, [0.85, 0.82, 0.78]),
        ceil: Mat::tex(T_CEILING, [0.6, 0.6, 0.6]),
        wainscot_h: 1.3,
        floor_tpm: 0.5,
    };
    b.shell(
        &style,
        &[
            op(Side::S, 1.6, 3.0, 0.0, 2.6, OpenKind::Door),
            op(Side::W, 1.0, 2.6, 1.2, 2.6, OpenKind::Window),
            op(Side::N, 4.5, 6.0, 1.2, 2.6, OpenKind::Window),
            op(Side::E, 4.5, 5.9, 0.0, 2.6, OpenKind::Door),
        ],
    );
    // Plans de travail le long du mur nord, évier sous la fenêtre.
    let top = Mat::tex(T_STONE, [0.75, 0.73, 0.7]);
    let cab = Mat::tex(T_WAINSCOT, [0.75, 0.68, 0.6]);
    b.mesh.cube_faces(v3(-17.9, 0.0, 19.35), v3(-9.4, 0.85, 20.0), &cab, 1.0, [true, true, false, false, true, false]);
    b.mesh.cube_faces(v3(-17.95, 0.85, 19.3), v3(-9.35, 0.92, 20.0), &top, 1.0, [true, true, true, true, true, false]);
    b.mesh.quad([v3(-13.3, 0.925, 19.45), v3(-12.2, 0.925, 19.45), v3(-12.2, 0.925, 19.9), v3(-13.3, 0.925, 19.9)], [[0.0; 2]; 4], &Mat::col([0.05, 0.05, 0.06]), Vec3::UP);
    b.obstacle(-17.95, 19.3, -9.35, 20.0, false);
    // Fourneau en fonte, braises rougeoyantes.
    let iron = Mat::tex(T_METAL, [0.22, 0.2, 0.2]);
    b.mesh.cube_faces(v3(-18.0, 0.0, 17.3), v3(-17.1, 0.9, 18.9), &iron, 1.0, [false, true, false, true, true, true]);
    b.mesh.cube(v3(-17.8, 0.9, 17.9), v3(-17.55, 3.2, 18.15), &iron, 1.0);
    b.mesh.quad([v3(-17.09, 0.15, 17.6), v3(-17.09, 0.15, 18.6), v3(-17.09, 0.45, 18.6), v3(-17.09, 0.45, 17.6)], [[0.0; 2]; 4], &Mat::col([0.9, 0.25, 0.05]).glow(1.5), v3(1.0, 0.0, 0.0));
    b.flames.push((v3(-17.2, 0.22, 18.1), 0.1));
    b.light(v3(-16.6, 0.5, 18.1), [0.9, 0.3, 0.08], 4.0, 0.3);
    b.obstacle(-18.0, 17.3, -17.1, 18.9, false);
    // Table de travail : couperet et quartier de viande.
    b.table(-15.2, 15.0, -12.2, 16.4, 0.85, Mat::tex(T_WOOD, [0.55, 0.42, 0.32]), false);
    b.mesh.cube(v3(-14.4, 0.85, 15.4), v3(-13.6, 1.05, 15.9), &Mat::tex(T_FLESH, [1.0, 1.0, 1.0]), 1.0);
    b.mesh.cube(v3(-13.2, 0.86, 15.5), v3(-12.7, 0.88, 15.62), &Mat::col([0.7, 0.7, 0.72]), 1.0);
    b.mesh.cube(v3(-12.95, 0.85, 15.9), v3(-12.6, 0.86, 16.2), &Mat::tex(T_NOTE, [1.0, 1.0, 1.0]), 1.0);
    b.interacts.push(Interact { pos: v3(-12.75, 0.0, 16.8), radius: 0.7, kind: InteractKind::Note(NOTE_COOK) });
    // Crochets de boucher et carcasses.
    let hook = Mat::col([0.3, 0.3, 0.32]);
    let flesh = Mat::tex(T_FLESH, [0.9, 0.8, 0.8]);
    for (x, z) in [(-9.95f32, 15.65f32), (-10.05, 16.2)] {
        b.mesh.cube(v3(x - 0.01, 2.2, z - 0.01), v3(x + 0.01, 3.2, z + 0.01), &hook, 1.0);
        let mut c = Mesh::new();
        c.taper(1.05, 2.2, (0.18, 0.12), (0.34, 0.2), (0.0, 0.0), &flesh, None);
        b.mesh.append(&c, &Mat4::translate(v3(x, 0.0, z)));
    }
    b.obstacle(-10.3, 15.4, -9.0, 16.45, false);
    b.msg(-10.8, 15.9, 0.7, "Des carcasses suspendues. Ce ne sont pas des porcs.");
    // Étagère de conserves (mur est).
    let shelf = Mat::tex(T_WOOD, [0.5, 0.38, 0.3]);
    b.mesh.cube_faces(v3(-9.5, 0.0, 12.3), v3(-9.0, 2.0, 15.2), &shelf, 1.0, [true, false, false, true, true, true]);
    for k in 0..8 {
        let z = 12.5 + (k % 4) as f32 * 0.65;
        let y = 0.9 + (k / 4) as f32 * 0.6;
        b.mesh.cube(v3(-9.6, y, z), v3(-9.48, y + 0.25, z + 0.3), &Mat::col([0.5, 0.35 + 0.1 * (k % 2) as f32, 0.2]), 1.0);
    }
    b.obstacle(-9.5, 12.3, -9.0, 15.2, true);
    // Lampe à huile suspendue.
    b.mesh.cube(v3(-13.72, 2.4, 15.68), v3(-13.68, 3.2, 15.72), &hook, 1.0);
    b.mesh.cube(v3(-13.85, 2.2, 15.55), v3(-13.55, 2.4, 15.85), &Mat::col([0.9, 0.7, 0.35]).glow(1.0), 1.0);
    b.light(v3(-13.7, 2.1, 15.7), [0.85, 0.62, 0.32], 6.5, 0.12);
    b.moon_spill(v3(-17.5, 2.0, 13.8), 4.0);
    b.moon_spill(v3(-12.7, 2.0, 19.5), 3.5);
    b.interacts.push(Interact { pos: EDGAR_POS + v3(0.75, 0.0, 0.0), radius: 0.9, kind: InteractKind::Npc });
    let cameras = vec![
        cam((-BIG, -BIG), (-13.5, 16.0), v3(-9.5, 2.8, 19.5), v3(-16.5, 0.5, 13.0), 64.0),
        cam((-13.5, -BIG), (BIG, 16.0), v3(-17.5, 2.8, 19.5), v3(-10.5, 0.5, 13.0), 64.0),
        cam((-BIG, 16.0), (-13.5, BIG), v3(-9.5, 2.8, 12.5), v3(-16.5, 0.5, 19.2), 64.0),
        cam((-13.5, 16.0), (BIG, BIG), v3(-17.5, 2.8, 12.5), v3(-10.5, 0.5, 19.2), 64.0),
    ];
    let doors = vec![
        door(v3(-15.7, 0.0, 12.0), v3(0.0, 0.0, 1.0), ROOM_DINING, v3(-15.7, 0.0, 11.1), PI, None),
        door(v3(-9.0, 0.0, 17.2), v3(-1.0, 0.0, 0.0), ROOM_CRYPT_STAIRS, v3(-7.5, 0.0, 14.9), 0.0, Some((LOCK_CRYPT, KEY_CRYPT))).stairs(false),
    ];
    let corpses = vec![(v3(-10.4, 0.0, 18.2), -2.0)];
    b.build("CUISINE", cameras, doors, [0.045, 0.045, 0.05], corpses)
}

pub fn crypt_stairs() -> Room {
    let mut b = Builder::new((-9.5, 14.0), (-5.5, 26.0), 3.4);
    let stone = Mat::tex(T_STONE, [0.55, 0.53, 0.5]);
    let style = Style {
        lower: Mat::tex(T_STONE, [0.45, 0.43, 0.4]),
        upper: stone,
        floor: Mat::tex(T_STONE, [0.5, 0.48, 0.46]),
        ceil: Mat::tex(T_STONE, [0.35, 0.34, 0.33]),
        wainscot_h: 0.4,
        floor_tpm: 0.5,
    };
    b.shell(&style, &[op(Side::S, 1.3, 2.7, 0.0, 2.6, OpenKind::Arch(true)), op(Side::N, 0.8, 3.2, 0.0, 3.0, OpenKind::DoubleDoor)]);
    // Arcs de voûte.
    for z in [17.0f32, 20.0, 23.0] {
        b.mesh.cube_faces(v3(-9.5, 0.0, z), v3(-9.2, 3.4, z + 0.4), &stone, 1.0, [false, true, false, false, true, true]);
        b.mesh.cube_faces(v3(-5.8, 0.0, z), v3(-5.5, 3.4, z + 0.4), &stone, 1.0, [true, false, false, false, true, true]);
        b.mesh.cube_faces(v3(-9.5, 3.0, z), v3(-5.5, 3.4, z + 0.4), &stone, 1.0, [false, false, true, false, true, true]);
        b.obstacle(-9.5, z, -9.2, z + 0.4, true);
        b.obstacle(-5.8, z, -5.5, z + 0.4, true);
    }
    // Machine à écrire sur une dalle.
    b.table(-9.45, 19.0, -8.6, 19.9, 0.8, Mat::tex(T_STONE, [0.6, 0.58, 0.55]), false);
    let metal = Mat::tex(T_METAL, [0.35, 0.33, 0.3]);
    b.mesh.cube(v3(-9.25, 0.8, 19.2), v3(-8.8, 0.95, 19.7), &metal, 1.0);
    b.mesh.cube(v3(-9.27, 0.95, 19.25), v3(-9.1, 1.05, 19.65), &metal, 1.0);
    b.interacts.push(Interact { pos: v3(-8.1, 0.0, 19.45), radius: 0.8, kind: InteractKind::Typewriter });
    b.candle_light(v3(-8.75, 0.92, 19.8), 4.5);
    // Inscription au-dessus de la porte de la crypte.
    b.msg(-7.5, 25.3, 1.0, "Gravé dans la pierre : « ICI REPOSENT LES ASHCROFT. QUE NUL NE TROUBLE LEUR SOMMEIL. » Quelque chose respire derrière la porte.");
    for z in [15.5f32, 24.5] {
        b.sconce(v3(-5.55, 2.0, z), v3(-1.0, 0.0, 0.0), 4.5);
    }
    let cameras = vec![
        cam((-BIG, -BIG), (BIG, 20.0), v3(-6.0, 3.0, 25.6), v3(-8.4, 0.3, 14.4), 62.0),
        cam((-BIG, 20.0), (BIG, BIG), v3(-9.0, 3.0, 14.4), v3(-6.6, 0.4, 25.9), 62.0),
    ];
    let doors = vec![
        door(v3(-7.5, 0.0, 14.0), v3(0.0, 0.0, 1.0), ROOM_KITCHEN, v3(-9.9, 0.0, 17.2), -FRAC_PI_2, None).stairs(true),
        door(v3(-7.5, 0.0, 26.0), v3(0.0, 0.0, -1.0), ROOM_CRYPT, v3(-7.5, 0.0, 27.0), 0.0, None),
    ];
    let mut r = b.build("ESCALIER DE LA CRYPTE", cameras, doors, [0.04, 0.04, 0.045], vec![]);
    r.fog = Fog { start: 6.0, end: 16.0, color: [0.0; 3] };
    r
}

pub fn crypt() -> Room {
    let mut b = Builder::new((-16.0, 26.0), (1.0, 44.0), 6.5);
    let stone = Mat::tex(T_STONE, [0.5, 0.5, 0.48]);
    let style = Style {
        lower: Mat::tex(T_STONE, [0.4, 0.4, 0.38]),
        upper: stone,
        floor: Mat::tex(T_STONE, [0.45, 0.45, 0.43]),
        ceil: Mat::tex(T_STONE, [0.25, 0.25, 0.25]),
        wainscot_h: 0.6,
        floor_tpm: 0.35,
    };
    b.shell(&style, &[op(Side::S, 7.3, 9.7, 0.0, 3.0, OpenKind::DoubleDoor)]);
    // Piliers engagés dans les murs, entre les sarcophages.
    for z in [32.0f32, 38.5] {
        for (x0, x1) in [(-16.0f32, -14.9f32), (-0.1, 1.0)] {
            let west = x0 < -5.0;
            b.mesh.cube_faces(v3(x0, 0.0, z - 0.6), v3(x1, 6.5, z + 0.6), &stone, 1.0, [!west, west, false, false, true, true]);
            let (b0, b1) = if west { (x0, x1 + 0.15) } else { (x0 - 0.15, x1) };
            b.mesh.cube_faces(v3(b0, 0.0, z - 0.75), v3(b1, 0.5, z + 0.75), &stone, 1.0, [!west, west, false, true, true, true]);
            b.obstacle(b0, z - 0.75, b1, z + 0.75, true);
        }
    }
    // Sarcophages le long des murs.
    let lid = Mat::tex(T_STONE, [0.62, 0.6, 0.56]);
    for z in [29.0f32, 35.5, 42.0] {
        for (x0, x1) in [(-15.9f32, -14.7f32), (-0.1, 0.9)] {
            b.mesh.cube_faces(v3(x0, 0.0, z - 1.0), v3(x1, 0.8, z + 1.0), &lid, 1.0, [x0 > -5.0, x0 < -5.0, false, true, true, true]);
            b.mesh.cube(v3(x0 + 0.1, 0.8, z - 0.9), v3(x1 - 0.1, 0.9, z + 0.9), &lid, 1.0);
            b.obstacle(x0, z - 1.0, x1, z + 1.0, false);
        }
    }
    b.msg(-14.2, 35.5, 0.9, "« Eleanor Ashcroft ». Le couvercle a été descellé, puis remis en place à la hâte.");
    b.msg(0.4 - 1.0, 29.0, 0.9, "« Brutus, fidèle compagnon ». Le sarcophage est vide. Des poils noirs collent à la pierre.");
    b.msg(-14.2, 42.0, 0.9, "« Alice Ashcroft ». Une petite tombe. Des fleurs fanées, toutes fraîches pourtant.");
    // Autel et cercle rituel.
    b.mesh.cube_faces(v3(-9.5, 0.0, 42.8), v3(-5.5, 1.0, 44.0), &lid, 1.0, [true, true, false, true, true, false]);
    b.obstacle(-9.5, 42.8, -5.5, 44.0, false);
    for x in [-9.2f32, -8.4, -6.6, -5.8] {
        b.candle_light(v3(x, 1.12, 43.4), 3.0);
    }
    let pit = Mat::col([0.02, 0.02, 0.02]).decal();
    b.mesh.disc(PIT + v3(0.0, 0.012, 0.0), 2.2, 16, &pit);
    let rune = Mat::col([0.15, 0.6, 0.25]).add().two().glow(1.0);
    for k in 0..16 {
        let a0 = k as f32 / 16.0 * std::f32::consts::TAU;
        let a1 = a0 + 0.25;
        let p = |a: f32, r: f32| PIT + v3(a.sin() * r, 0.02, a.cos() * r);
        b.mesh.quad([p(a0, 2.3), p(a1, 2.3), p(a1, 2.55), p(a0, 2.55)], [[0.0; 2]; 4], &rune, Vec3::UP);
    }
    b.light(PIT + v3(0.0, 0.6, 0.0), [0.2, 0.75, 0.3], 8.0, 0.2);
    // Braseros.
    let iron = Mat::col([0.15, 0.13, 0.12]);
    for (x, z) in [(-13.0f32, 27.5f32), (-2.0, 27.5), (-13.0, 42.5), (-2.0, 42.5)] {
        b.mesh.cube(v3(x - 0.05, 0.0, z - 0.05), v3(x + 0.05, 1.0, z + 0.05), &iron, 1.0);
        b.mesh.cube(v3(x - 0.3, 1.0, z - 0.3), v3(x + 0.3, 1.2, z + 0.3), &iron, 1.0);
        b.flames.push((v3(x, 1.3, z), 0.22));
        b.light(v3(x, 1.8, z), [1.0, 0.45, 0.15], 9.5, 0.3);
        b.obstacle(x - 0.3, z - 0.3, x + 0.3, z + 0.3, false);
    }
    // Caméras hautes près du centre, plongeant vers chaque quart : les
    // piliers, près des murs, ne masquent pas l'arène.
    let cameras = vec![
        cam((-BIG, -BIG), (-7.5, 35.0), v3(-5.0, 6.3, 39.0), v3(-12.5, 0.0, 28.0), 72.0),
        cam((-7.5, -BIG), (BIG, 35.0), v3(-10.0, 6.3, 39.0), v3(-2.5, 0.0, 28.0), 72.0),
        cam((-BIG, 35.0), (-7.5, BIG), v3(-5.0, 6.3, 31.0), v3(-12.5, 0.0, 42.0), 72.0),
        cam((-7.5, 35.0), (BIG, BIG), v3(-10.0, 6.3, 31.0), v3(-2.5, 0.0, 42.0), 72.0),
    ];
    let doors = vec![door(v3(-7.5, 0.0, 26.0), v3(0.0, 0.0, 1.0), ROOM_CRYPT_STAIRS, v3(-7.5, 0.0, 25.1), PI, None)];
    let mut r = b.build("CRYPTE DES ASHCROFT", cameras, doors, [0.06, 0.065, 0.06], vec![]);
    r.fog = Fog { start: 12.0, end: 34.0, color: [0.0; 3] };
    r
}

pub fn items() -> Vec<ItemSpawn> {
    use ItemKind::*;
    vec![
        item(ROOM_KITCHEN, v3(-16.6, 0.92, 19.6), Herb),
        item(ROOM_KITCHEN, v3(-10.2, 0.92, 19.6), Ammo(10)),
        item(ROOM_CRYPT_STAIRS, v3(-6.0, 0.0, 15.2), Ammo(15)),
        item(ROOM_CRYPT_STAIRS, v3(-9.0, 0.0, 24.6), Shells(6)),
        item(ROOM_CRYPT_STAIRS, v3(-6.0, 0.0, 22.0), Medkit),
        item(ROOM_CRYPT, v3(-15.2, 0.0, 26.8), Ammo(15)),
        item(ROOM_CRYPT, v3(0.3, 0.0, 26.8), Shells(4)),
        item(ROOM_CRYPT, v3(-10.0, 0.0, 43.4), Herb),
        ItemSpawn { flag: Some(F_BOSS_DEAD), ..item(ROOM_CRYPT, PIT + v3(0.0, 0.02, 0.0), Key(KEY_FRONT)) },
    ]
}

pub fn enemies() -> Vec<EnemySpawn> {
    vec![
        EnemySpawn { state: SpawnState::Eating, ..zombie(ROOM_KITCHEN, -10.9, 17.6, -2.4) },
        EnemySpawn { look: 2, ..zombie(ROOM_KITCHEN, -12.4, 13.2, -0.4) },
        // Edgar, une fois mort.
        EnemySpawn { look: 3, state: SpawnState::Lying, wake: Some(F_CRYPT_KEY), ..zombie(ROOM_KITCHEN, EDGAR_POS.x + 0.6, EDGAR_POS.z, FRAC_PI_2) },
        enemy(ROOM_CRYPT, PIT.x, PIT.z, PI, EnemyKind::Boss),
    ]
}
