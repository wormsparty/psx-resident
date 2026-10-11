//! Maillages et outils de construction (quads subdivisés, boîtes...).

use crate::math::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Blend {
    Opaque,
    /// Opaque posé à plat sur une autre surface (tapis, sang) : biais de profondeur.
    Decal,
    Add,
    Mul,
}

#[derive(Clone, Copy, Debug)]
pub struct MV {
    pub p: Vec3,
    pub n: Vec3,
    pub uv: [f32; 2],
    pub c: [f32; 3],
    /// Émission propre (ajoutée à l'éclairage reçu).
    pub glow: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct MT {
    pub i: [u32; 3],
    pub tex: i16,
    pub blend: Blend,
    pub two_sided: bool,
}

/// Matériau utilisé à la construction.
#[derive(Clone, Copy, Debug)]
pub struct Mat {
    pub tex: i16,
    pub c: [f32; 3],
    pub blend: Blend,
    pub two_sided: bool,
    pub glow: f32,
}

impl Mat {
    pub fn tex(tex: i16, c: [f32; 3]) -> Mat {
        Mat { tex, c, blend: Blend::Opaque, two_sided: false, glow: 0.0 }
    }
    pub fn col(c: [f32; 3]) -> Mat {
        Mat::tex(-1, c)
    }
    pub fn two(mut self) -> Mat {
        self.two_sided = true;
        self
    }
    pub fn add(mut self) -> Mat {
        self.blend = Blend::Add;
        self
    }
    pub fn decal(mut self) -> Mat {
        self.blend = Blend::Decal;
        self
    }
    pub fn mul(mut self) -> Mat {
        self.blend = Blend::Mul;
        self
    }
    pub fn glow(mut self, g: f32) -> Mat {
        self.glow = g;
        self
    }
}

#[derive(Clone, Default, Debug)]
pub struct Mesh {
    pub v: Vec<MV>,
    pub t: Vec<MT>,
}

impl Mesh {
    pub fn new() -> Mesh {
        Mesh::default()
    }

    fn push_v(&mut self, p: Vec3, n: Vec3, uv: [f32; 2], m: &Mat) -> u32 {
        self.v.push(MV { p, n, uv, c: m.c, glow: m.glow });
        (self.v.len() - 1) as u32
    }

    /// Triangle orienté pour que sa face visible regarde vers `want`.
    pub fn tri(&mut self, p: [Vec3; 3], uv: [[f32; 2]; 3], m: &Mat, want: Vec3) {
        let mut n = (p[2] - p[0]).cross(p[1] - p[0]);
        let (p, uv) = if n.dot(want) < 0.0 {
            n = -n;
            ([p[0], p[2], p[1]], [uv[0], uv[2], uv[1]])
        } else {
            (p, uv)
        };
        let n = n.norm();
        let a = self.push_v(p[0], n, uv[0], m);
        let b = self.push_v(p[1], n, uv[1], m);
        let c = self.push_v(p[2], n, uv[2], m);
        self.t.push(MT { i: [a, b, c], tex: m.tex, blend: m.blend, two_sided: m.two_sided });
    }

    /// Quadrilatère (sommets dans l'ordre du périmètre) orienté vers `want`.
    pub fn quad(&mut self, p: [Vec3; 4], uv: [[f32; 2]; 4], m: &Mat, want: Vec3) {
        let mut n = (p[2] - p[0]).cross(p[1] - p[0]);
        let (p, uv) = if n.dot(want) < 0.0 {
            n = -n;
            ([p[0], p[3], p[2], p[1]], [uv[0], uv[3], uv[2], uv[1]])
        } else {
            (p, uv)
        };
        let n = n.norm();
        let i: Vec<u32> = (0..4).map(|k| self.push_v(p[k], n, uv[k], m)).collect();
        let t = |a, b, c| MT { i: [a, b, c], tex: m.tex, blend: m.blend, two_sided: m.two_sided };
        self.t.push(t(i[0], i[1], i[2]));
        self.t.push(t(i[0], i[2], i[3]));
    }

    /// Quad subdivisé en grille. `du`/`dv` sont les arêtes complètes,
    /// `uv0`/`uv_span` les coordonnées de texture du coin et de l'étendue.
    #[allow(clippy::too_many_arguments)]
    pub fn grid(&mut self, o: Vec3, du: Vec3, dv: Vec3, uv0: [f32; 2], uv_span: [f32; 2], cell: f32, m: &Mat, want: Vec3) {
        let nu = ((du.len() / cell).ceil() as usize).max(1);
        let nv = ((dv.len() / cell).ceil() as usize).max(1);
        for j in 0..nv {
            for i in 0..nu {
                let (a, b) = (i as f32 / nu as f32, (i + 1) as f32 / nu as f32);
                let (c, d) = (j as f32 / nv as f32, (j + 1) as f32 / nv as f32);
                let p = |s: f32, t: f32| o + du * s + dv * t;
                let uv = |s: f32, t: f32| [uv0[0] + uv_span[0] * s, uv0[1] + uv_span[1] * t];
                self.quad([p(a, c), p(b, c), p(b, d), p(a, d)], [uv(a, c), uv(b, c), uv(b, d), uv(a, d)], m, want);
            }
        }
    }

    /// Quad subdivisé avec texture plaquée en coordonnées monde
    /// (`tpm` = répétitions de texture par mètre).
    pub fn wall(&mut self, o: Vec3, du: Vec3, dv: Vec3, tpm: f32, uoff: f32, m: &Mat, want: Vec3) {
        let ul = du.len();
        let vl = dv.len();
        // v=0 en haut de la texture : on part du haut du mur.
        let vtop = (o.y + vl) * tpm;
        self.grid(o, du, dv, [uoff * tpm, -vtop + vl * tpm], [ul * tpm, -vl * tpm], 1.0, m, want);
    }

    /// Boîte alignée sur les axes, faces vers l'extérieur.
    pub fn cube(&mut self, min: Vec3, max: Vec3, m: &Mat, tpm: f32) {
        self.cube_faces(min, max, m, tpm, [true; 6]);
    }

    /// Boîte avec choix des faces : [-x, +x, -y, +y, -z, +z].
    pub fn cube_faces(&mut self, min: Vec3, max: Vec3, m: &Mat, tpm: f32, faces: [bool; 6]) {
        let s = max - min;
        let cell = 1.2;
        let uv = |a: f32, b: f32| [a * tpm, b * tpm];
        if faces[0] {
            self.grid(v3(min.x, max.y, min.z), v3(0.0, 0.0, s.z), v3(0.0, -s.y, 0.0), [0.0, 0.0], uv(s.z, s.y), cell, m, v3(-1.0, 0.0, 0.0));
        }
        if faces[1] {
            self.grid(v3(max.x, max.y, max.z), v3(0.0, 0.0, -s.z), v3(0.0, -s.y, 0.0), [0.0, 0.0], uv(s.z, s.y), cell, m, v3(1.0, 0.0, 0.0));
        }
        if faces[2] {
            self.grid(v3(min.x, min.y, min.z), v3(s.x, 0.0, 0.0), v3(0.0, 0.0, s.z), [0.0, 0.0], uv(s.x, s.z), cell, m, v3(0.0, -1.0, 0.0));
        }
        if faces[3] {
            self.grid(v3(min.x, max.y, min.z), v3(s.x, 0.0, 0.0), v3(0.0, 0.0, s.z), [0.0, 0.0], uv(s.x, s.z), cell, m, v3(0.0, 1.0, 0.0));
        }
        if faces[4] {
            self.grid(v3(max.x, max.y, min.z), v3(-s.x, 0.0, 0.0), v3(0.0, -s.y, 0.0), [0.0, 0.0], uv(s.x, s.y), cell, m, v3(0.0, 0.0, -1.0));
        }
        if faces[5] {
            self.grid(v3(min.x, max.y, max.z), v3(s.x, 0.0, 0.0), v3(0.0, -s.y, 0.0), [0.0, 0.0], uv(s.x, s.y), cell, m, v3(0.0, 0.0, 1.0));
        }
    }

    /// Boîte dont chaque face reçoit la texture entière (pour objets).
    pub fn block(&mut self, min: Vec3, max: Vec3, m: &Mat) {
        let full = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let x0 = min.x;
        let x1 = max.x;
        let y0 = min.y;
        let y1 = max.y;
        let z0 = min.z;
        let z1 = max.z;
        // Chaque face : haut-gauche, haut-droit, bas-droit, bas-gauche (vu de face).
        self.quad([v3(x1, y1, z1), v3(x0, y1, z1), v3(x0, y0, z1), v3(x1, y0, z1)], full, m, v3(0.0, 0.0, 1.0));
        self.quad([v3(x0, y1, z0), v3(x1, y1, z0), v3(x1, y0, z0), v3(x0, y0, z0)], full, m, v3(0.0, 0.0, -1.0));
        self.quad([v3(x1, y1, z0), v3(x1, y1, z1), v3(x1, y0, z1), v3(x1, y0, z0)], full, m, v3(1.0, 0.0, 0.0));
        self.quad([v3(x0, y1, z1), v3(x0, y1, z0), v3(x0, y0, z0), v3(x0, y0, z1)], full, m, v3(-1.0, 0.0, 0.0));
        self.quad([v3(x0, y1, z1), v3(x1, y1, z1), v3(x1, y1, z0), v3(x0, y1, z0)], full, m, v3(0.0, 1.0, 0.0));
        self.quad([v3(x0, y0, z0), v3(x1, y0, z0), v3(x1, y0, z1), v3(x0, y0, z1)], full, m, v3(0.0, -1.0, 0.0));
    }

    /// Boîte effilée (section inférieure / supérieure différentes), centrée en x/z,
    /// de y0 à y1. Utilisée pour les membres des personnages.
    /// `front` : matériau optionnel de la face avant (+z), ex. visage.
    pub fn taper(&mut self, y0: f32, y1: f32, bot: (f32, f32), top: (f32, f32), zoff: (f32, f32), m: &Mat, front: Option<&Mat>) {
        let (bw, bd) = (bot.0 * 0.5, bot.1 * 0.5);
        let (tw, td) = (top.0 * 0.5, top.1 * 0.5);
        let b = [v3(-bw, y0, -bd + zoff.0), v3(bw, y0, -bd + zoff.0), v3(bw, y0, bd + zoff.0), v3(-bw, y0, bd + zoff.0)];
        let t = [v3(-tw, y1, -td + zoff.1), v3(tw, y1, -td + zoff.1), v3(tw, y1, td + zoff.1), v3(-tw, y1, td + zoff.1)];
        let full = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
        let center = v3(0.0, (y0 + y1) * 0.5, (zoff.0 + zoff.1) * 0.5);
        let mut side = |a: Vec3, bb: Vec3, c: Vec3, d: Vec3, mm: &Mat, uv: [[f32; 2]; 4]| {
            let mid = (a + bb + c + d) * 0.25;
            self.quad([a, bb, c, d], uv, mm, mid - center);
        };
        // Avant (+z) : bas-gauche (+x vu de face = gauche du personnage... on garde l'ordre du périmètre)
        let fm = front.unwrap_or(m);
        let front_uv = [[1.0, 1.0], [0.0, 1.0], [0.0, 0.0], [1.0, 0.0]];
        side(b[3], b[2], t[2], t[3], fm, front_uv);
        side(b[1], b[0], t[0], t[1], m, full);
        side(b[2], b[1], t[1], t[2], m, full);
        side(b[0], b[3], t[3], t[0], m, full);
        side(t[0], t[1], t[2], t[3], m, full);
        side(b[0], b[1], b[2], b[3], m, full);
    }

    pub fn append(&mut self, o: &Mesh, xf: &Mat4) {
        let base = self.v.len() as u32;
        for v in &o.v {
            self.v.push(MV { p: xf.point(v.p), n: xf.dir(v.n).norm(), ..*v });
        }
        for t in &o.t {
            self.t.push(MT { i: [t.i[0] + base, t.i[1] + base, t.i[2] + base], ..*t });
        }
    }

    /// Disque horizontal (ombre portée, flaque...).
    pub fn disc(&mut self, center: Vec3, r: f32, segs: usize, m: &Mat) {
        for i in 0..segs {
            let a0 = i as f32 / segs as f32 * std::f32::consts::TAU;
            let a1 = (i + 1) as f32 / segs as f32 * std::f32::consts::TAU;
            let p0 = center + v3(a0.sin() * r, 0.0, a0.cos() * r);
            let p1 = center + v3(a1.sin() * r, 0.0, a1.cos() * r);
            self.tri([center, p0, p1], [[0.5, 0.5], [0.0, 0.0], [1.0, 0.0]], m, Vec3::UP);
        }
    }

    /// Applique une fonction de pré-éclairage aux couleurs des sommets.
    pub fn prelight(&mut self, f: &dyn Fn(Vec3, Vec3) -> [f32; 3]) {
        for v in &mut self.v {
            let l = f(v.p, v.n);
            v.c = [v.c[0] * (l[0] + v.glow), v.c[1] * (l[1] + v.glow), v.c[2] * (l[2] + v.glow)];
        }
    }
}
