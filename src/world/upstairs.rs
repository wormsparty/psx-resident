//! L'étage : galerie des portraits, chambre des maîtres, chambre d'Alice
//! et laboratoire du Docteur.

use super::*;

/// Hauteur de l'étage au-dessus du sol extérieur.
const UPPER: f32 = 5.2;

pub const NOTE_DIARY: &[&str] = &[
    "Le journal intime d'Alice, d'une écriture appliquée d'enfant :",
    "« Papa dit que je suis malade mais Brutus reste toujours avec moi. C'est le plus gentil des chiens. »",
    "Les dernières pages sont d'une autre main, celle de Lady Eleanor :",
    "« Alice s'en est allée la première. Brutus l'a suivie, il refusait de manger. Je les rejoindrai bientôt. Que mon mari ne touche jamais à nos tombes. »",
];

pub const NOTE_LADY: &[&str] = &[
    "Une lettre inachevée sur la coiffeuse :",
    "« Mon amour, tu passes tes nuits dans la crypte avec tes chiens. Je t'entends parler à Alice à travers la pierre. »",
    "« Dans la galerie, tu as fait accrocher nos portraits dans l'ordre où la mort nous prendra, disais-tu. Le tien en dernier, car tu ne comptes pas mourir. »",
];

pub const NOTE_LAB: &[&str] = &[
    "Le journal du Docteur, dernières pages :",
    "« Sujet 7 : force décuplée, aucune douleur, aucune obéissance. Je l'ai plongé dans la cuve. Qu'il y dorme. »",
    "« Mes chiens ont mangé ce que la cuve rejetait. Ils ne meurent plus. Le grand, celui que j'appelle le Molosse, garde maintenant le ponton. »",
    "« Il ne me reste qu'un cobaye. Moi. Ce soir je descends dans la crypte et je rejoins Alice. La clé du manoir restera sur mon cœur. »",
];

pub fn gallery() -> Room {
    let mut b = Builder::new((-10.0, 20.0), (10.0, 26.0), 3.6);
    let style = Style {
        lower: Mat::tex(T_WAINSCOT, [0.8, 0.72, 0.68]),
        upper: Mat::tex(T_PAPER_RED, [0.75, 0.7, 0.75]),
        floor: Mat::tex(T_WOOD, [0.65, 0.6, 0.55]),
        ceil: Mat::tex(T_CEILING, [0.7, 0.7, 0.7]),
        wainscot_h: 1.1,
        floor_tpm: 0.5,
    };
    b.shell(
        &style,
        &[
            op(Side::S, 1.5, 3.3, 1.0, 2.8, OpenKind::Window),
            op(Side::S, 6.0, 7.4, 0.0, 2.6, OpenKind::Door),
            op(Side::S, 14.0, 15.6, 0.0, 2.7, OpenKind::Arch(false)),
            op(Side::S, 16.8, 18.6, 1.0, 2.8, OpenKind::Window),
            op(Side::W, 2.3, 3.7, 0.0, 2.6, OpenKind::Door),
            op(Side::E, 2.3, 3.7, 0.0, 2.6, OpenKind::Door),
        ],
    );
    let carpet = Mat::tex(T_CARPET, [0.75, 0.75, 0.85]).decal();
    b.mesh.grid(v3(-9.6, 0.01, 23.8), v3(19.2, 0.0, 0.0), v3(0.0, 0.0, -1.6), [0.0, 0.0], [9.6, 0.8], 1.0, &carpet, Vec3::UP);
    // Les quatre portraits de la famille, dans de lourds cadres dorés munis
    // d'une plaque : bien plus imposants que les tableaux du reste du manoir.
    // Un cadre enfoncé recule dans le mur et sa plaque s'allume.
    let n = v3(0.0, 0.0, -1.0);
    // De gauche à droite : Brutus, Eleanor, Alice, le Docteur (pas dans l'ordre !).
    let mut portraits = Vec::new();
    for (id, x, tex) in [(1u8, -7.5f32, T_HOUND_PORTRAIT), (2, -4.0, T_LADY), (0, 4.0, T_GIRL)] {
        let rest = family_portrait(v3(x, 2.0, 26.0), tex, false);
        let pressed = family_portrait(v3(x, 2.0, 26.0), tex, true);
        portraits.push((id, rest, pressed));
    }
    for (id, x) in [(1u8, -7.5f32), (2, -4.0), (0, 4.0), (3, 7.5)] {
        b.interacts.push(Interact { pos: v3(x, 0.0, 25.3), radius: 0.75, kind: InteractKind::Portrait(id) });
    }
    // Le portrait du Docteur : fermé, puis pivoté sur une niche secrète.
    let doc = Mat::tex(T_DOCTOR, [1.0, 1.0, 1.0]);
    let closed = family_portrait(v3(7.5, 2.0, 26.0), T_DOCTOR, false);
    let mut open = Mesh::new();
    // Niche sombre et cadre ouvert à 80°.
    let dark = Mat::col([0.04, 0.03, 0.03]);
    // La niche est creusée derrière la toile, à hauteur de main.
    open.quad([v3(7.1, 1.4, 25.98), v3(7.9, 1.4, 25.98), v3(7.9, 2.2, 25.98), v3(7.1, 2.2, 25.98)], [[0.0; 2]; 4], &dark, n);
    open.quad([v3(7.1, 1.4, 25.7), v3(7.9, 1.4, 25.7), v3(7.9, 1.4, 25.98), v3(7.1, 1.4, 25.98)], [[0.0; 2]; 4], &Mat::tex(T_STONE, [0.5, 0.5, 0.5]), Vec3::UP);
    let hinge = v3(6.9, 2.0, 25.97);
    let dir = v3(-0.17, 0.0, -0.98);
    open.quad(
        [hinge + v3(0.0, -0.75, 0.0), hinge + dir + v3(0.0, -0.75, 0.0), hinge + dir + v3(0.0, 0.75, 0.0), hinge + v3(0.0, 0.75, 0.0)],
        [[1.0, 1.0], [0.0, 1.0], [0.0, 0.0], [1.0, 0.0]],
        &doc.two(),
        v3(1.0, 0.0, 0.0),
    );
    // Le cadre doré pivote avec la toile.
    let gilt = Mat::col([0.62, 0.47, 0.18]);
    let side = v3(0.98, 0.0, -0.17);
    for (a, h) in [(0.0f32, 0.12f32), (0.88, 0.12)] {
        let p0 = hinge + dir * a + v3(0.0, -0.87, 0.0);
        open.quad([p0, p0 + dir * h, p0 + dir * h + v3(0.0, 1.74, 0.0), p0 + v3(0.0, 1.74, 0.0)], [[0.0; 2]; 4], &gilt.two(), side);
    }
    for yy in [-0.87f32, 0.75] {
        let p0 = hinge + v3(0.0, yy, 0.0);
        open.quad([p0, p0 + dir, p0 + dir + v3(0.0, 0.12, 0.0), p0 + v3(0.0, 0.12, 0.0)], [[0.0; 2]; 4], &gilt.two(), side);
    }
    // Machine à écrire au centre du mur nord.
    b.table(-0.6, 25.15, 0.6, 26.0, 0.8, Mat::tex(T_WOOD, [0.6, 0.45, 0.35]), false);
    let metal = Mat::tex(T_METAL, [0.35, 0.33, 0.3]);
    b.mesh.cube(v3(-0.25, 0.8, 25.4), v3(0.25, 0.95, 25.85), &metal, 1.0);
    b.mesh.cube(v3(-0.2, 0.95, 25.7), v3(0.2, 1.05, 25.88), &metal, 1.0);
    let paper = Mat::tex(T_NOTE, [1.0, 1.0, 1.0]).two();
    b.mesh.quad([v3(-0.15, 1.0, 25.75), v3(0.15, 1.0, 25.75), v3(0.15, 1.22, 25.85), v3(-0.15, 1.22, 25.85)], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]], &paper, v3(0.0, 0.3, -1.0));
    b.interacts.push(Interact { pos: v3(0.0, 0.0, 24.7), radius: 0.9, kind: InteractKind::Typewriter });
    b.candle_light(v3(0.45, 0.92, 25.6), 4.0);
    // Console et buste.
    b.table(-6.2, 20.05, -4.6, 20.55, 0.85, Mat::tex(T_WOOD, [0.5, 0.36, 0.28]), false);
    let stone = Mat::tex(T_STONE, [0.85, 0.85, 0.82]);
    b.mesh.cube_faces(v3(9.2, 0.0, 25.2), v3(9.9, 1.1, 25.9), &stone, 1.0, [true, false, false, true, true, false]);
    b.mesh.cube(v3(9.38, 1.1, 25.38), v3(9.72, 1.45, 25.72), &stone, 2.0);
    b.mesh.cube(v3(9.44, 1.45, 25.44), v3(9.66, 1.75, 25.66), &stone, 2.0);
    b.obstacle(9.2, 25.2, 10.0, 26.0, true);
    b.msg(8.9, 25.0, 0.7, "Le buste de Lord Ashcroft. Quelqu'un lui a brisé le nez.");
    for x in [-5.75f32, -1.8, 1.8, 5.75] {
        b.sconce(v3(x, 2.2, 25.95), v3(0.0, 0.0, -1.0), 4.8);
    }
    b.moon_spill(v3(-7.6, 2.0, 20.4), 4.0);
    b.moon_spill(v3(7.7, 2.0, 20.4), 4.0);
    let cameras = vec![
        // Le long du mur sud (portes, escalier) : vus depuis le nord.
        cam((-BIG, -BIG), (0.0, 22.0), v3(3.5, 3.4, 24.9), v3(-6.0, 0.3, 20.2), 64.0),
        cam((0.0, -BIG), (BIG, 22.0), v3(-3.5, 3.4, 24.9), v3(6.0, 0.3, 20.2), 64.0),
        // Le long des portraits : vus depuis le sud.
        cam((-BIG, 22.0), (-3.3, BIG), v3(-0.8, 3.4, 21.0), v3(-9.0, 0.6, 24.8), 64.0),
        cam((-3.3, 22.0), (3.3, BIG), v3(-6.5, 3.4, 21.0), v3(2.5, 0.6, 25.6), 64.0),
        cam((3.3, 22.0), (BIG, BIG), v3(0.8, 3.4, 21.0), v3(9.0, 0.6, 24.8), 64.0),
    ];
    let doors = vec![
        door(v3(4.8, 0.0, 20.0), v3(0.0, 0.0, 1.0), ROOM_LIBRARY, v3(4.75, 0.0, 20.5), PI, None).stairs(false),
        door(v3(-3.3, 0.0, 20.0), v3(0.0, 0.0, 1.0), ROOM_NURSERY, v3(-3.3, 0.0, 19.1), PI, None),
        door(v3(-10.0, 0.0, 23.0), v3(1.0, 0.0, 0.0), ROOM_BEDROOM, v3(-10.9, 0.0, 23.0), -FRAC_PI_2, None),
        door(v3(10.0, 0.0, 23.0), v3(-1.0, 0.0, 0.0), ROOM_LAB, v3(10.9, 0.0, 23.0), FRAC_PI_2, Some((LOCK_LAB, KEY_LAB))),
    ];
    b.light(v3(0.0, 3.2, 23.0), [0.55, 0.42, 0.3], 9.0, 0.06);
    let mut r = b.build("GALERIE DES PORTRAITS", cameras, doors, [0.06, 0.055, 0.07], vec![]);
    r.level = UPPER;
    r.cond.push(CondMesh { flag: F_PORTRAITS, when: false, mesh: closed });
    r.cond.push(CondMesh { flag: F_PORTRAITS, when: true, mesh: open });
    r.portraits = portraits;
    r
}

/// Portrait de famille de la galerie (toile 1 x 1,5 m sur le mur nord) :
/// cadre doré en relief et plaque de laiton, qui luit quand le cadre est
/// enfoncé dans le mur.
fn family_portrait(c: Vec3, tex: i16, pressed: bool) -> Mesh {
    let mut m = Mesh::new();
    let n = v3(0.0, 0.0, -1.0);
    let depth = if pressed { 0.0 } else { 0.07 };
    let z = c.z - 0.02 - depth;
    let (hw, hh) = (0.5f32, 0.75f32);
    let canvas = Mat::tex(tex, [1.0, 1.0, 1.0]);
    let q = |x: f32, y: f32| v3(c.x + x, c.y + y, z);
    m.quad([q(hw, -hh), q(-hw, -hh), q(-hw, hh), q(hw, hh)], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]], &canvas, n);
    // Moulure : quatre baguettes en relief autour de la toile.
    let gilt = Mat::col([0.62, 0.47, 0.18]);
    let (f, d) = (0.12f32, 0.05f32);
    let zf = z - d;
    m.cube(v3(c.x - hw - f, c.y - hh - f, zf), v3(c.x + hw + f, c.y - hh, z + 0.01), &gilt, 1.0);
    m.cube(v3(c.x - hw - f, c.y + hh, zf), v3(c.x + hw + f, c.y + hh + f, z + 0.01), &gilt, 1.0);
    m.cube(v3(c.x - hw - f, c.y - hh, zf), v3(c.x - hw, c.y + hh, z + 0.01), &gilt, 1.0);
    m.cube(v3(c.x + hw, c.y - hh, zf), v3(c.x + hw + f, c.y + hh, z + 0.01), &gilt, 1.0);
    // Plaque gravée sous le cadre, sur le mur (elle ne bouge pas).
    let plate = if pressed { Mat::col([1.0, 0.82, 0.4]).glow(1.0) } else { Mat::col([0.5, 0.4, 0.2]) };
    let py = c.y - hh - f - 0.12;
    m.cube(v3(c.x - 0.22, py - 0.07, c.z - 0.02), v3(c.x + 0.22, py + 0.07, c.z), &plate, 1.0);
    m
}

pub fn bedroom() -> Room {
    let mut b = Builder::new((-20.0, 18.0), (-10.0, 28.0), 3.4);
    let style = Style {
        lower: Mat::tex(T_PANEL, [0.9, 0.85, 0.85]),
        upper: Mat::tex(T_PAPER_GREEN, [0.95, 0.85, 0.95]),
        floor: Mat::tex(T_WOOD, [0.7, 0.62, 0.58]),
        ceil: Mat::tex(T_CEILING, [0.7, 0.7, 0.7]),
        wainscot_h: 1.1,
        floor_tpm: 0.5,
    };
    b.shell(
        &style,
        &[
            op(Side::E, 4.3, 5.7, 0.0, 2.6, OpenKind::Door),
            op(Side::W, 1.6, 3.4, 1.0, 2.8, OpenKind::Window),
            op(Side::W, 6.6, 8.4, 1.0, 2.8, OpenKind::Window),
            op(Side::N, 6.5, 8.3, 1.0, 2.8, OpenKind::Window),
        ],
    );
    let carpet = Mat::tex(T_CARPET, [0.6, 0.7, 0.9]).decal();
    b.mesh.grid(v3(-17.0, 0.01, 20.5), v3(4.5, 0.0, 0.0), v3(0.0, 0.0, 4.0), [0.0, 0.0], [2.2, 2.0], 1.0, &carpet, Vec3::UP);
    // Lit à baldaquin contre le mur nord.
    let wood = Mat::tex(T_WOOD, [0.45, 0.3, 0.22]);
    let sheet = Mat::tex(T_CLOTH, [0.9, 0.88, 0.85]);
    let velvet = Mat::tex(T_CURTAIN, [0.8, 0.8, 0.9]).two();
    b.mesh.cube_faces(v3(-18.6, 0.0, 25.0), v3(-16.0, 0.35, 28.0), &wood, 1.0, [true, true, false, true, true, false]);
    b.mesh.cube_faces(v3(-18.55, 0.35, 25.05), v3(-16.05, 0.6, 27.95), &sheet, 1.0, [true, true, false, true, true, false]);
    b.mesh.cube(v3(-18.3, 0.6, 27.2), v3(-16.3, 0.75, 27.8), &Mat::tex(T_CLOTH, [1.0, 1.0, 1.0]), 1.0);
    for (x, z) in [(-18.6f32, 25.0f32), (-16.15, 25.0), (-18.6, 27.85), (-16.15, 27.85)] {
        b.mesh.cube(v3(x, 0.0, z), v3(x + 0.15, 2.4, z + 0.15), &wood, 1.0);
    }
    b.mesh.cube(v3(-18.6, 2.4, 25.0), v3(-16.0, 2.55, 28.0), &wood, 1.0);
    for x in [-18.55f32, -16.05] {
        b.mesh.quad([v3(x, 0.7, 25.1), v3(x, 0.7, 25.6), v3(x, 2.4, 25.6), v3(x, 2.4, 25.1)], [[0.0, 3.0], [1.0, 3.0], [1.0, 0.0], [0.0, 0.0]], &velvet, v3(0.0, 0.0, -1.0));
    }
    b.obstacle(-18.6, 25.0, -16.0, 28.0, false);
    b.msg(-17.3, 24.6, 0.9, "Les draps sont imprégnés d'une odeur douceâtre de chair. Une forme humaine y est restée creusée.");
    // Table de chevet.
    b.mesh.cube_faces(v3(-15.85, 0.0, 27.3), v3(-15.25, 0.6, 27.95), &wood, 1.0, [true, true, false, true, true, false]);
    b.obstacle(-15.85, 27.3, -15.25, 28.0, false);
    b.candle_light(v3(-15.4, 0.72, 27.7), 3.5);
    // Cheminée froide, fusil accroché au-dessus.
    let stone = Mat::tex(T_STONE, [0.6, 0.57, 0.55]);
    b.mesh.cube_faces(v3(-20.0, 0.0, 21.9), v3(-19.4, 1.3, 22.4), &stone, 1.0, [false, true, false, true, true, true]);
    b.mesh.cube_faces(v3(-20.0, 0.0, 23.6), v3(-19.4, 1.3, 24.1), &stone, 1.0, [false, true, false, true, true, true]);
    b.mesh.cube_faces(v3(-20.0, 1.3, 21.8), v3(-19.3, 1.45, 24.2), &stone, 1.0, [false, true, true, true, true, true]);
    let soot = Mat::col([0.04, 0.03, 0.03]).decal();
    b.mesh.quad([v3(-19.98, 0.0, 22.4), v3(-19.98, 0.0, 23.6), v3(-19.98, 1.3, 23.6), v3(-19.98, 1.3, 22.4)], [[0.0; 2]; 4], &soot, v3(1.0, 0.0, 0.0));
    let rack = Mat::col([0.3, 0.2, 0.12]);
    for z in [22.5f32, 23.5] {
        b.mesh.cube(v3(-19.98, 1.85, z - 0.04), v3(-19.85, 1.95, z + 0.04), &rack, 1.0);
    }
    b.obstacle(-20.0, 21.8, -19.3, 24.2, false);
    // Coiffeuse et miroir.
    b.mesh.cube_faces(v3(-14.5, 0.0, 18.0), v3(-13.0, 0.8, 18.55), &wood, 1.0, [true, true, false, true, false, true]);
    b.mesh.quad([v3(-14.2, 1.0, 18.02), v3(-13.3, 1.0, 18.02), v3(-13.3, 2.0, 18.02), v3(-14.2, 2.0, 18.02)], [[0.0; 2]; 4], &Mat::col([0.25, 0.3, 0.35]).glow(0.3), v3(0.0, 0.0, 1.0));
    b.mesh.cube(v3(-13.6, 0.8, 18.2), v3(-13.3, 0.81, 18.45), &Mat::tex(T_NOTE, [1.0, 1.0, 1.0]), 1.0);
    b.obstacle(-14.5, 18.0, -13.0, 18.55, false);
    b.interacts.push(Interact { pos: v3(-13.6, 0.0, 19.0), radius: 0.8, kind: InteractKind::Note(NOTE_LADY) });
    // Armoire.
    b.mesh.cube_faces(v3(-19.9, 0.0, 18.0), v3(-18.3, 2.3, 18.7), &Mat::tex(T_WAINSCOT, [0.7, 0.6, 0.55]), 1.0, [true, true, false, true, false, true]);
    b.obstacle(-19.9, 18.0, -18.3, 18.7, true);
    b.msg(-19.1, 19.2, 0.8, "L'armoire est pleine de robes noires. Toutes de deuil.");
    // Fauteuil.
    let leather = Mat::col([0.25, 0.08, 0.1]);
    b.mesh.cube(v3(-12.2, 0.0, 20.2), v3(-11.3, 0.45, 21.1), &leather, 1.0);
    b.mesh.cube(v3(-11.55, 0.45, 20.2), v3(-11.3, 1.1, 21.1), &leather, 1.0);
    b.obstacle(-12.2, 20.2, -11.3, 21.1, false);
    b.sconce(v3(-10.05, 2.1, 20.5), v3(-1.0, 0.0, 0.0), 4.0);
    b.moon_spill(v3(-19.5, 2.0, 20.5), 4.0);
    b.moon_spill(v3(-19.5, 2.0, 25.5), 4.0);
    b.moon_spill(v3(-12.6, 2.0, 27.5), 4.0);
    let cameras = vec![
        cam((-15.0, -BIG), (BIG, 23.0), v3(-15.0, 3.1, 27.7), v3(-11.5, 0.4, 18.6), 66.0),
        cam((-BIG, -BIG), (-15.0, 23.0), v3(-10.6, 3.0, 27.4), v3(-19.0, 0.6, 18.8), 64.0),
        cam((-15.0, 23.0), (BIG, BIG), v3(-19.4, 3.0, 18.6), v3(-11.0, 0.6, 27.0), 64.0),
        cam((-BIG, 23.0), (-15.0, BIG), v3(-10.6, 3.0, 18.6), v3(-19.0, 0.6, 27.0), 64.0),
    ];
    let doors = vec![door(v3(-10.0, 0.0, 23.0), v3(-1.0, 0.0, 0.0), ROOM_GALLERY, v3(-9.1, 0.0, 23.0), FRAC_PI_2, None)];
    // Lustre de fer au-dessus du tapis.
    let iron = Mat::col([0.2, 0.18, 0.15]);
    b.mesh.cube(v3(-14.78, 2.6, 22.47), v3(-14.72, 3.4, 22.53), &iron, 1.0);
    for i in 0..5 {
        let a = i as f32 / 5.0 * std::f32::consts::TAU;
        let p = v3(-14.75 + a.sin() * 0.45, 2.6, 22.5 + a.cos() * 0.45);
        b.mesh.cube(p - v3(0.03, 0.02, 0.03), p + v3(0.03, 0.02, 0.03), &iron, 1.0);
        b.flames.push((p + v3(0.0, 0.07, 0.0), 0.04));
    }
    b.light(v3(-14.75, 2.3, 22.5), [0.8, 0.58, 0.34], 8.5, 0.08);
    let mut r = b.build("CHAMBRE DES MAÎTRES", cameras, doors, [0.05, 0.048, 0.062], vec![]);
    r.level = UPPER;
    r
}

pub fn nursery() -> Room {
    let mut b = Builder::new((-8.0, 12.0), (0.0, 20.0), 3.2);
    let style = Style {
        lower: Mat::tex(T_WAINSCOT, [1.0, 0.95, 0.9]),
        upper: Mat::tex(T_PAPER_BLUE, [1.3, 1.0, 1.1]),
        floor: Mat::tex(T_WOOD, [0.8, 0.75, 0.7]),
        ceil: Mat::tex(T_CEILING, [0.8, 0.8, 0.8]),
        wainscot_h: 1.0,
        floor_tpm: 0.5,
    };
    b.shell(
        &style,
        &[
            op(Side::N, 4.0, 5.4, 0.0, 2.6, OpenKind::Door),
            op(Side::W, 2.5, 4.3, 1.0, 2.6, OpenKind::Window),
            op(Side::S, 3.0, 4.8, 1.0, 2.6, OpenKind::Window),
        ],
    );
    let wood = Mat::tex(T_WOOD, [0.7, 0.55, 0.45]);
    // Petit lit, poupée.
    b.mesh.cube_faces(v3(-7.7, 0.0, 17.0), v3(-6.3, 0.45, 19.7), &wood, 1.0, [true, true, false, true, true, false]);
    b.mesh.cube(v3(-7.65, 0.45, 17.05), v3(-6.35, 0.55, 19.65), &Mat::tex(T_CLOTH, [1.0, 0.92, 0.95]), 1.0);
    b.mesh.cube(v3(-7.5, 0.55, 19.1), v3(-6.5, 0.65, 19.55), &Mat::tex(T_CLOTH, [1.0, 1.0, 1.0]), 1.0);
    let doll = Mat::col([0.85, 0.75, 0.7]);
    b.mesh.cube(v3(-7.15, 0.55, 18.6), v3(-6.95, 0.75, 18.75), &Mat::col([0.6, 0.15, 0.2]), 1.0);
    b.mesh.cube(v3(-7.12, 0.75, 18.62), v3(-6.98, 0.88, 18.74), &doll, 1.0);
    b.obstacle(-7.7, 17.0, -6.3, 20.0, false);
    b.mesh.cube(v3(-7.2, 0.55, 17.4), v3(-6.85, 0.57, 17.8), &Mat::tex(T_NOTE, [1.0, 0.95, 0.9]), 1.0);
    b.interacts.push(Interact { pos: v3(-6.0, 0.0, 17.6), radius: 0.8, kind: InteractKind::Note(NOTE_DIARY) });
    // Cheval à bascule.
    let horse = Mat::tex(T_WOOD, [0.9, 0.85, 0.8]);
    b.mesh.cube(v3(-2.6, 0.35, 14.2), v3(-2.4, 0.6, 15.1), &horse, 1.0);
    b.mesh.cube(v3(-2.62, 0.55, 15.0), v3(-2.38, 0.95, 15.25), &horse, 1.0);
    b.mesh.cube(v3(-2.7, 0.0, 14.0), v3(-2.3, 0.08, 15.3), &Mat::col([0.4, 0.15, 0.1]), 1.0);
    for z in [14.3f32, 15.0] {
        b.mesh.cube(v3(-2.55, 0.08, z), v3(-2.45, 0.35, z + 0.08), &horse, 1.0);
    }
    b.obstacle(-2.8, 14.0, -2.2, 15.3, false);
    b.msg(-2.5, 15.9, 0.7, "Un cheval à bascule. Il se balance encore, doucement, tout seul.");
    // Table et boîte à musique.
    b.table(-4.6, 12.1, -3.4, 12.8, 0.6, Mat::tex(T_WOOD, [0.75, 0.6, 0.5]), true);
    b.mesh.cube(v3(-4.15, 0.6, 12.3), v3(-3.85, 0.75, 12.55), &Mat::col([0.5, 0.25, 0.3]), 1.0);
    b.mesh.cube(v3(-4.03, 0.75, 12.4), v3(-3.97, 0.85, 12.46), &Mat::col([0.85, 0.75, 0.4]), 1.0);
    b.interacts.push(Interact { pos: v3(-4.0, 0.0, 13.3), radius: 0.8, kind: InteractKind::MusicBox });
    // Panier du chien.
    let basket = Mat::col([0.45, 0.32, 0.18]);
    b.mesh.cube_faces(v3(-1.7, 0.0, 12.2), v3(-0.3, 0.25, 13.2), &basket, 1.0, [true, true, false, true, true, true]);
    b.mesh.cube(v3(-1.55, 0.25, 12.4), v3(-0.45, 0.27, 13.0), &Mat::tex(T_CURTAIN, [0.7, 0.7, 0.9]), 1.0);
    b.mesh.cube(v3(-1.2, 0.27, 12.6), v3(-0.8, 0.31, 12.66), &Mat::col([0.85, 0.82, 0.7]), 1.0);
    b.obstacle(-1.7, 12.2, -0.3, 13.2, false);
    b.msg(-1.0, 13.7, 0.7, "Un panier de chien. Une plaque de collier : « BRUTUS ». Un os rongé traîne au fond.");
    // Coffre à jouets.
    b.mesh.cube_faces(v3(-0.6, 0.0, 17.0), v3(0.0, 0.5, 18.3), &Mat::tex(T_WAINSCOT, [1.0, 0.8, 0.8]), 1.0, [true, false, false, true, true, true]);
    b.obstacle(-0.6, 17.0, 0.0, 18.3, false);
    b.candle_light(v3(-3.6, 0.72, 12.3), 3.5);
    b.moon_spill(v3(-7.5, 2.0, 15.4), 4.5);
    b.moon_spill(v3(-4.1, 2.0, 12.5), 3.5);
    let cameras = vec![
        cam((-BIG, -BIG), (BIG, 16.0), v3(-0.3, 2.9, 19.7), v3(-5.0, 0.3, 12.5), 72.0),
        cam((-BIG, 16.0), (BIG, BIG), v3(-7.7, 2.9, 12.3), v3(-2.5, 0.3, 19.5), 72.0),
    ];
    let doors = vec![door(v3(-3.3, 0.0, 20.0), v3(0.0, 0.0, -1.0), ROOM_GALLERY, v3(-3.3, 0.0, 20.9), 0.0, None)];
    b.light(v3(-4.0, 2.6, 16.0), [0.6, 0.5, 0.42], 8.0, 0.0);
    b.mesh.cube(v3(-4.25, 2.95, 15.75), v3(-3.75, 3.2, 16.25), &Mat::col([0.9, 0.8, 0.6]).glow(1.0), 1.0);
    let mut r = b.build("CHAMBRE D'ALICE", cameras, doors, [0.055, 0.05, 0.065], vec![]);
    r.level = UPPER;
    r
}

/// Cuve de verre (rayon `r`) remplie de liquide vert.
fn tank(m: &mut Mesh, x: f32, z: f32, full: bool) {
    let metal = Mat::tex(T_METAL, [0.4, 0.42, 0.4]);
    let mut t = Mesh::new();
    t.prism(0.0, 0.45, (1.4, 1.4), (1.4, 1.4), (0.0, 0.0), 10, &metal, true);
    t.prism(2.65, 2.9, (1.4, 1.4), (1.3, 1.3), (0.0, 0.0), 10, &metal, true);
    t.prism(2.9, 3.8, (0.15, 0.15), (0.15, 0.15), (0.0, 0.0), 6, &metal, false);
    if full {
        t.prism(0.45, 2.65, (1.25, 1.25), (1.25, 1.25), (0.0, 0.0), 10, &Mat::col([0.05, 0.3, 0.12]).add().two().glow(1.0), false);
    }
    m.append(&t, &Mat4::translate(v3(x, 0.0, z)));
}

/// Cuve brisée : armature, débris de verre, flaque.
fn broken_tank(m: &mut Mesh, x: f32, z: f32) {
    let metal = Mat::tex(T_METAL, [0.4, 0.42, 0.4]);
    let mut t = Mesh::new();
    t.prism(0.0, 0.45, (1.4, 1.4), (1.4, 1.4), (0.0, 0.0), 10, &metal, true);
    t.prism(2.65, 2.9, (1.4, 1.4), (1.3, 1.3), (0.0, 0.0), 10, &metal, true);
    t.prism(2.9, 3.8, (0.15, 0.15), (0.15, 0.15), (0.0, 0.0), 6, &metal, false);
    let glass = Mat::col([0.2, 0.3, 0.3]).add().two();
    for k in 0..7 {
        let a = k as f32 * 0.9;
        let (sa, ca) = a.sin_cos();
        let p = v3(sa * 0.65, 0.45, ca * 0.65);
        t.tri([p, p + v3(ca * 0.25, 0.0, -sa * 0.25), p + v3(ca * 0.1, 0.35 + (k % 3) as f32 * 0.25, -sa * 0.1)], [[0.0; 2]; 3], &glass, v3(sa, 0.0, ca));
    }
    t.disc(v3(-0.9, 0.012, 0.0), 1.0, 10, &Mat::col([0.06, 0.2, 0.08]).decal());
    m.append(&t, &Mat4::translate(v3(x, 0.0, z)));
}

pub fn lab() -> Room {
    let mut b = Builder::new((10.0, 16.0), (22.0, 28.0), 3.8);
    let style = Style {
        lower: Mat::tex(T_TILE, [0.8, 0.85, 0.85]),
        upper: Mat::tex(T_STONE, [0.6, 0.62, 0.6]),
        floor: Mat::tex(T_TILE, [0.75, 0.78, 0.78]),
        ceil: Mat::tex(T_CEILING, [0.55, 0.6, 0.6]),
        wainscot_h: 1.5,
        floor_tpm: 0.6,
    };
    b.shell(
        &style,
        &[
            op(Side::W, 6.3, 7.7, 0.0, 2.6, OpenKind::Door),
            op(Side::N, 2.5, 4.5, 1.2, 3.0, OpenKind::Window),
            op(Side::E, 1.5, 3.3, 1.2, 3.0, OpenKind::Window),
        ],
    );
    // Cuves le long du mur est : celle du Sujet 7 au milieu.
    tank(&mut b.mesh, 20.7, 26.4, true);
    broken_tank(&mut b.mesh, 20.7, 20.4);
    for z in [20.4f32, 23.4, 26.4] {
        b.obstacle(20.0, z - 0.7, 21.4, z + 0.7, true);
        b.light(v3(19.6, 1.6, z), [0.2, 0.75, 0.35], 4.5, 0.06);
    }
    b.msg(19.2, 20.4, 0.9, "Une cuve brisée depuis longtemps. Des traces de griffes partent vers la fenêtre.");
    b.msg(19.2, 26.4, 0.9, "Dans le liquide trouble flotte un chien écorché. Il ouvre un œil quand vous approchez.");
    let mut intact = Mesh::new();
    tank(&mut intact, 20.7, 23.4, true);
    let mut shattered = Mesh::new();
    broken_tank(&mut shattered, 20.7, 23.4);
    // Table d'opération et corps sous un drap.
    let metal = Mat::tex(T_METAL, [0.5, 0.52, 0.5]);
    b.table(14.6, 21.6, 16.8, 22.5, 0.9, metal, false);
    b.mesh.cube(v3(14.8, 0.9, 21.7), v3(16.6, 1.1, 22.4), &Mat::tex(T_CLOTH, [0.9, 0.9, 0.85]), 1.0);
    b.mesh.cube(v3(16.25, 0.95, 21.85), v3(16.5, 1.2, 22.25), &Mat::tex(T_CLOTH, [0.9, 0.9, 0.85]), 1.0);
    b.msg(15.7, 21.1, 0.9, "Un corps sous un drap. Il a été recousu avec des morceaux de plusieurs autres.");
    // Paillasse et verrerie (mur sud).
    b.table(12.5, 16.05, 18.5, 16.8, 0.9, Mat::tex(T_WOOD, [0.45, 0.35, 0.3]), false);
    let jar = Mat::col([0.25, 0.6, 0.3]).glow(0.8);
    for x in [13.0f32, 13.4, 15.2, 16.0, 16.4, 18.1] {
        b.mesh.cube(v3(x - 0.08, 0.9, 16.2), v3(x + 0.08, 1.15, 16.4), &jar, 1.0);
    }
    // Étagère de bocaux (mur nord).
    b.mesh.cube_faces(v3(15.5, 0.0, 27.4), v3(19.8, 2.3, 28.0), &Mat::tex(T_WOOD, [0.4, 0.3, 0.25]), 1.0, [true, true, false, true, true, false]);
    for k in 0..12 {
        let x = 15.8 + (k % 6) as f32 * 0.65;
        let y = 1.0 + (k / 6) as f32 * 0.7;
        b.mesh.cube(v3(x, y, 27.25), v3(x + 0.25, y + 0.35, 27.4), &Mat::col([0.3 + 0.05 * (k % 3) as f32, 0.55, 0.3]).glow(0.7), 1.0);
    }
    b.obstacle(15.5, 27.4, 19.8, 28.0, true);
    b.msg(17.6, 26.9, 0.9, "Des bocaux étiquetés : « cœur, sujet 3 », « yeux, sujet 5 »... L'un d'eux bat encore.");
    // Bureau du Docteur.
    b.table(10.05, 17.0, 11.3, 19.4, 0.8, Mat::tex(T_WOOD, [0.5, 0.35, 0.27]), false);
    b.mesh.cube(v3(10.3, 0.8, 17.3), v3(10.75, 0.81, 17.7), &Mat::tex(T_NOTE, [1.0, 1.0, 1.0]), 1.0);
    b.interacts.push(Interact { pos: v3(11.8, 0.0, 17.5), radius: 0.7, kind: InteractKind::Note(NOTE_LAB) });
    b.light(v3(15.5, 3.5, 22.0), [0.75, 0.8, 0.7], 9.0, 0.35);
    b.mesh.cube(v3(15.2, 3.7, 21.8), v3(15.8, 3.8, 22.2), &Mat::col([0.9, 0.95, 0.85]).glow(1.0), 1.0);
    b.moon_spill(v3(14.0, 2.4, 27.5), 4.0);
    // Pas de caméra dans les coins est : les cuves boucheraient la vue.
    let cameras = vec![
        cam((-BIG, -BIG), (16.0, 22.0), v3(18.8, 3.5, 27.6), v3(11.5, 0.4, 16.8), 64.0),
        cam((16.0, -BIG), (BIG, 22.0), v3(10.6, 3.5, 27.6), v3(19.5, 0.4, 17.0), 64.0),
        cam((-BIG, 22.0), (16.0, BIG), v3(18.8, 3.5, 16.4), v3(11.0, 0.5, 27.2), 64.0),
        cam((16.0, 22.0), (BIG, BIG), v3(10.6, 3.5, 16.4), v3(19.5, 0.5, 27.2), 64.0),
    ];
    let doors = vec![door(v3(10.0, 0.0, 23.0), v3(1.0, 0.0, 0.0), ROOM_GALLERY, v3(9.1, 0.0, 23.0), -FRAC_PI_2, None)];
    let mut r = b.build("LABORATOIRE", cameras, doors, [0.035, 0.05, 0.045], vec![]);
    r.level = UPPER;
    r.cond.push(CondMesh { flag: F_CRYPT_KEY, when: false, mesh: intact });
    r.cond.push(CondMesh { flag: F_CRYPT_KEY, when: true, mesh: shattered });
    r
}

pub fn items() -> Vec<ItemSpawn> {
    use ItemKind::*;
    vec![
        item(ROOM_GALLERY, v3(-5.4, 0.85, 20.3), Ammo(15)),
        ItemSpawn { flag: Some(F_PORTRAITS), ..item(ROOM_GALLERY, v3(7.5, 1.42, 25.82), Key(KEY_LAB)) },
        item(ROOM_BEDROOM, v3(-19.75, 1.85, 23.0), Shotgun),
        item(ROOM_BEDROOM, v3(-15.55, 0.6, 27.45), Shells(5)),
        item(ROOM_BEDROOM, v3(-17.0, 0.0, 18.5), Medkit),
        item(ROOM_NURSERY, v3(-0.3, 0.5, 17.6), Herb),
        item(ROOM_NURSERY, v3(-7.5, 0.0, 12.5), Ammo(10)),
        item(ROOM_LAB, v3(10.6, 0.8, 18.6), Key(KEY_CRYPT)),
        item(ROOM_LAB, v3(14.0, 0.9, 16.45), Shells(6)),
        item(ROOM_LAB, v3(17.4, 0.9, 16.45), Medkit),
        item(ROOM_LAB, v3(12.0, 0.0, 27.5), Ammo(15)),
    ]
}

pub fn enemies() -> Vec<EnemySpawn> {
    vec![
        zombie(ROOM_GALLERY, -8.3, 24.6, 0.8),
        EnemySpawn { look: 1, ..zombie(ROOM_GALLERY, 8.4, 21.4, -0.6) },
        EnemySpawn { look: 2, wake: Some(F_CRYPT_KEY), ..zombie(ROOM_GALLERY, -1.0, 21.0, 2.5) },
        EnemySpawn { state: SpawnState::Lying, look: 2, ..zombie(ROOM_BEDROOM, -14.6, 25.8, 1.2) },
        EnemySpawn { look: 1, ..zombie(ROOM_NURSERY, -5.6, 15.4, 2.6) },
        EnemySpawn { wake: Some(F_CRYPT_KEY), ..enemy(ROOM_LAB, 20.7, 23.4, -FRAC_PI_2, EnemyKind::Brute) },
    ]
}
