//! Modèles low-poly articulés (héroïne, zombies) et poses procédurales.

use crate::math::*;
use crate::mesh::*;
use crate::raster::{Lighting, Raster};
use crate::textures::*;

pub struct Body {
    pub pelvis: Mesh,
    pub torso: Mesh,
    pub head: Mesh,
    pub ponytail: Option<Mesh>,
    pub upper_arm: Mesh,
    pub forearm: Mesh,
    pub thigh: Mesh,
    pub shin: Mesh,
    pub gun: Mesh,
    pub holster: Mesh,
    pub scale: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Pose {
    pub root_y: f32,
    pub root_pitch: f32,
    pub root_roll: f32,
    pub torso_pitch: f32,
    pub torso_yaw: f32,
    pub torso_roll: f32,
    pub head_pitch: f32,
    pub head_yaw: f32,
    pub head_roll: f32,
    pub arm_fwd: [f32; 2],
    pub arm_out: [f32; 2],
    pub elbow: [f32; 2],
    pub hip: [f32; 2],
    pub hip_out: [f32; 2],
    pub knee: [f32; 2],
    pub pony: f32,
    pub gun: bool,
}

fn l(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
fn l2(a: [f32; 2], b: [f32; 2], t: f32) -> [f32; 2] {
    [l(a[0], b[0], t), l(a[1], b[1], t)]
}

impl Pose {
    pub fn lerp(&self, o: &Pose, t: f32) -> Pose {
        Pose {
            root_y: l(self.root_y, o.root_y, t),
            root_pitch: l(self.root_pitch, o.root_pitch, t),
            root_roll: l(self.root_roll, o.root_roll, t),
            torso_pitch: l(self.torso_pitch, o.torso_pitch, t),
            torso_yaw: l(self.torso_yaw, o.torso_yaw, t),
            torso_roll: l(self.torso_roll, o.torso_roll, t),
            head_pitch: l(self.head_pitch, o.head_pitch, t),
            head_yaw: l(self.head_yaw, o.head_yaw, t),
            head_roll: l(self.head_roll, o.head_roll, t),
            arm_fwd: l2(self.arm_fwd, o.arm_fwd, t),
            arm_out: l2(self.arm_out, o.arm_out, t),
            elbow: l2(self.elbow, o.elbow, t),
            hip: l2(self.hip, o.hip, t),
            hip_out: l2(self.hip_out, o.hip_out, t),
            knee: l2(self.knee, o.knee, t),
            pony: l(self.pony, o.pony, t),
            gun: if t < 0.5 { self.gun } else { o.gun },
        }
    }

    /// Pose de repos, bras le long du corps avec respiration.
    pub fn idle(t: f32) -> Pose {
        let b = (t * 1.6).sin();
        Pose {
            arm_out: [0.12, 0.12],
            elbow: [0.15, 0.15],
            arm_fwd: [0.03 * b, 0.03 * b],
            torso_pitch: 0.02 * b,
            head_pitch: 0.03 * (t * 0.7).sin(),
            head_yaw: 0.15 * (t * 0.31).sin(),
            ..Default::default()
        }
    }

    /// Cycle de marche/course. `phase` en radians, `run` dans [0,1].
    pub fn walk(phase: f32, run: f32) -> Pose {
        let s = phase.sin();
        let leg = l(0.45, 0.75, run);
        let arm = l(0.35, 0.8, run);
        let knee_l = ((-phase).sin().max(0.0)) * l(0.7, 1.3, run) + 0.1;
        let knee_r = (phase.sin().max(0.0)) * l(0.7, 1.3, run) + 0.1;
        Pose {
            root_y: -(phase * 2.0).cos().abs() * 0.0 + (phase * 2.0).sin().abs() * l(0.025, 0.06, run) - l(0.0, 0.04, run),
            torso_pitch: l(0.04, 0.22, run),
            torso_yaw: s * l(0.08, 0.15, run),
            head_pitch: -l(0.02, 0.15, run),
            arm_fwd: [s * arm, -s * arm],
            arm_out: [0.1, 0.1],
            elbow: [l(0.25, 1.3, run), l(0.25, 1.3, run)],
            hip: [-s * leg, s * leg],
            knee: [knee_l, knee_r],
            pony: 0.15 + (phase * 2.0).sin() * l(0.1, 0.3, run),
            ..Default::default()
        }
    }

    /// Visée au pistolet, à deux mains.
    pub fn aim(recoil: f32, pitch: f32) -> Pose {
        Pose {
            torso_yaw: 0.25,
            torso_pitch: -0.02,
            head_yaw: -0.2,
            head_pitch: -pitch * 0.5,
            arm_fwd: [1.45 + recoil * 0.3 + pitch, 1.57 + recoil * 0.35 + pitch],
            arm_out: [-0.62, -0.32],
            elbow: [0.25, 0.0],
            hip: [0.12, -0.1],
            hip_out: [0.08, 0.12],
            knee: [0.15, 0.08],
            gun: true,
            ..Default::default()
        }
    }

    /// Bras tendus en avant : démarche de zombie.
    pub fn shamble(phase: f32, t: f32) -> Pose {
        let s = phase.sin();
        Pose {
            root_roll: s * 0.08,
            torso_pitch: 0.18,
            torso_roll: -s * 0.08,
            head_roll: 0.35 + 0.1 * (t * 0.9).sin(),
            head_pitch: 0.2,
            arm_fwd: [1.25 + 0.15 * (t * 2.1).sin(), 1.1 + 0.15 * (t * 1.7).cos()],
            arm_out: [-0.1, -0.05],
            elbow: [0.25, 0.4],
            hip: [-s * 0.35, s * 0.35],
            knee: [(-s).max(0.0) * 0.5 + 0.15, s.max(0.0) * 0.5 + 0.15],
            root_y: -0.03,
            ..Default::default()
        }
    }

    /// Zombie agenouillé, penché sur un cadavre.
    pub fn eating(t: f32) -> Pose {
        let chew = (t * 7.0).sin();
        Pose {
            root_y: -0.42,
            torso_pitch: 0.9 + 0.05 * chew,
            head_pitch: 0.4 + 0.12 * chew,
            head_roll: 0.2,
            arm_fwd: [1.3 + 0.1 * chew, 1.1],
            arm_out: [-0.1, 0.05],
            elbow: [0.8, 0.9],
            hip: [0.0, 0.1],
            knee: [1.55, 1.45],
            ..Default::default()
        }
    }

    /// Corps allongé sur le dos.
    pub fn lying(twist: f32) -> Pose {
        Pose {
            root_pitch: -1.5,
            root_y: 0.12,
            head_yaw: 0.6 + twist,
            arm_out: [0.9, 0.5 + twist],
            elbow: [0.4, 1.2],
            hip_out: [0.2, 0.1],
            hip: [0.15, 0.4],
            knee: [0.1, 0.6],
            ..Default::default()
        }
    }
}

fn skin_box(y0: f32, y1: f32, bot: (f32, f32), top: (f32, f32), m: &Mat) -> Mesh {
    let mut me = Mesh::new();
    me.taper(y0, y1, bot, top, (0.0, 0.0), m, None);
    me
}

pub fn heroine() -> Body {
    // Agente d'unité spéciale : béret, gilet tactique, rangers. Couleurs sourdes.
    let skin = Mat::col([0.80, 0.62, 0.50]);
    let shirt = Mat::tex(T_FABRIC, [0.30, 0.36, 0.42]);
    let pants = Mat::tex(T_FABRIC, [0.19, 0.21, 0.26]);
    let vest = Mat::tex(T_VEST, [1.0, 1.0, 1.0]);
    let pad = Mat::tex(T_FABRIC, [0.17, 0.17, 0.15]);
    let beret = Mat::tex(T_FABRIC, [0.13, 0.18, 0.30]);
    let belt = Mat::col([0.10, 0.08, 0.07]);
    let leather = Mat::col([0.16, 0.10, 0.06]);
    let boot = Mat::col([0.08, 0.07, 0.06]);
    let sole = Mat::col([0.04, 0.04, 0.04]);
    let glove = Mat::col([0.07, 0.07, 0.07]);
    let hair = Mat::tex(T_HAIR, [0.75, 0.75, 0.75]);
    let face = Mat::tex(T_FACE, [1.0, 1.0, 1.0]);
    let metal = Mat::col([0.18, 0.18, 0.2]);
    let brass = Mat::col([0.62, 0.50, 0.20]);

    // Bassin : pantalon, ceinturon à poches.
    let mut pelvis = Mesh::new();
    pelvis.prism(-0.14, 0.06, (0.30, 0.21), (0.28, 0.19), (0.0, 0.0), 8, &pants, true);
    pelvis.prism(-0.01, 0.06, (0.31, 0.22), (0.30, 0.21), (0.0, 0.0), 8, &belt, false);
    pelvis.block(v3(-0.03, -0.005, 0.1), v3(0.03, 0.055, 0.115), &brass);
    for x in [-0.15, 0.15] {
        pelvis.block(v3(x - 0.035, -0.06, -0.06), v3(x + 0.035, 0.05, 0.04), &leather);
    }
    pelvis.block(v3(-0.06, -0.05, -0.13), v3(0.06, 0.05, -0.09), &leather);

    // Torse : chemise, gilet tactique, col, cou.
    let mut torso = Mesh::new();
    torso.prism(0.0, 0.52, (0.25, 0.17), (0.34, 0.2), (0.0, 0.0), 8, &shirt, false);
    torso.prism(0.05, 0.46, (0.28, 0.2), (0.36, 0.235), (0.0, 0.015), 8, &vest, true);
    for x in [-0.115, 0.115] {
        torso.block(v3(x - 0.035, 0.44, -0.105), v3(x + 0.035, 0.505, 0.12), &pad);
    }
    torso.prism(0.47, 0.54, (0.13, 0.115), (0.12, 0.105), (0.0, 0.0), 8, &shirt, false);
    torso.prism(0.5, 0.6, (0.085, 0.085), (0.08, 0.08), (0.0, 0.0), 6, &skin, false);

    // Tête : visage, carré court, béret incliné.
    let mut head = Mesh::new();
    head.prism(0.0, 0.25, (0.115, 0.145), (0.165, 0.19), (0.008, 0.0), 8, &face, true);
    head.taper(-0.01, 0.25, (0.18, 0.1), (0.19, 0.11), (-0.06, -0.055), &hair, None);
    for x in [-0.088, 0.088] {
        head.block(v3(x - 0.014, -0.01, -0.075), v3(x + 0.014, 0.25, 0.05), &hair);
    }
    let mut cap = Mesh::new();
    cap.prism(0.0, 0.04, (0.2, 0.215), (0.27, 0.27), (0.0, -0.01), 10, &beret, false);
    cap.prism(0.04, 0.075, (0.27, 0.27), (0.17, 0.17), (-0.01, -0.02), 10, &beret, true);
    head.append(&cap, &(Mat4::translate(v3(0.0, 0.225, -0.005)) * Mat4::rot_z(-0.28) * Mat4::rot_x(-0.1)));

    // Bras : manche, épaulette, avant-bras nu, gant mitaine.
    let mut upper_arm = Mesh::new();
    upper_arm.prism(-0.28, 0.02, (0.08, 0.08), (0.105, 0.105), (0.0, 0.0), 8, &shirt, false);
    upper_arm.prism(-0.1, 0.035, (0.135, 0.135), (0.12, 0.12), (0.0, 0.0), 8, &pad, true);
    let mut forearm = Mesh::new();
    forearm.prism(-0.06, 0.015, (0.09, 0.09), (0.095, 0.095), (0.0, 0.0), 8, &shirt, true);
    forearm.prism(-0.25, -0.04, (0.06, 0.06), (0.078, 0.078), (0.0, 0.0), 6, &skin, false);
    forearm.prism(-0.335, -0.23, (0.07, 0.045), (0.072, 0.07), (0.0, 0.0), 6, &glove, true);
    forearm.prism(-0.375, -0.33, (0.055, 0.03), (0.066, 0.04), (0.0, 0.0), 6, &skin, true);

    // Jambes : treillis, genouillères, rangers.
    let mut thigh = Mesh::new();
    thigh.prism(-0.43, 0.0, (0.11, 0.12), (0.16, 0.17), (0.0, 0.0), 8, &pants, false);
    let mut shin = Mesh::new();
    shin.prism(-0.32, 0.0, (0.095, 0.105), (0.115, 0.125), (0.0, 0.0), 8, &pants, true);
    shin.block(v3(-0.045, -0.09, 0.045), v3(0.045, 0.04, 0.08), &pad);
    shin.prism(-0.47, -0.27, (0.11, 0.13), (0.115, 0.125), (0.005, 0.0), 8, &boot, false);
    shin.prism(-0.3, -0.255, (0.125, 0.135), (0.125, 0.135), (0.0, 0.0), 8, &leather, true);
    shin.block(v3(-0.05, -0.47, 0.02), v3(0.05, -0.4, 0.15), &boot);
    shin.block(v3(-0.058, -0.485, -0.07), v3(0.058, -0.465, 0.158), &sole);

    // Pistolet tenu dans la main (repère de l'avant-bras : -y = canon).
    let mut gun = Mesh::new();
    gun.block(v3(-0.018, -0.5, 0.0), v3(0.018, -0.28, 0.055), &metal);
    gun.block(v3(-0.016, -0.34, -0.09), v3(0.016, -0.28, 0.01), &Mat::col([0.1, 0.1, 0.1]));
    // Holster de cuisse (jambe droite) et ses sangles.
    let mut holster = Mesh::new();
    holster.block(v3(0.075, -0.27, -0.05), v3(0.115, -0.04, 0.05), &belt);
    holster.block(v3(0.08, -0.06, -0.06), v3(0.11, 0.03, -0.02), &metal);
    for y in [-0.11, -0.22] {
        holster.prism(y - 0.013, y + 0.013, (0.15, 0.16), (0.155, 0.165), (0.0, 0.0), 8, &belt, false);
    }

    Body { pelvis, torso, head, ponytail: None, upper_arm, forearm, thigh, shin, gun, holster, scale: 1.0 }
}

pub fn zombie(variant: u32) -> Body {
    let tones = [[0.52, 0.56, 0.44], [0.6, 0.58, 0.5], [0.46, 0.5, 0.46]];
    let skin = Mat::col(tones[variant as usize % 3]);
    let shirt = Mat::tex(T_BLOODY, if variant % 2 == 0 { [1.0, 1.0, 1.0] } else { [0.6, 0.75, 0.9] });
    let pants = Mat::col(if variant % 2 == 0 { [0.22, 0.2, 0.17] } else { [0.15, 0.15, 0.2] });
    let shoe = Mat::col([0.08, 0.06, 0.05]);
    let face = Mat::tex(T_ZFACE, [1.0, 1.0, 1.0]);
    let hair = Mat::col([0.1, 0.09, 0.08]);

    let pelvis = skin_box(-0.13, 0.05, (0.34, 0.22), (0.32, 0.2), &pants);
    let mut torso = Mesh::new();
    torso.taper(0.0, 0.52, (0.3, 0.2), (0.42, 0.22), (0.0, 0.0), &shirt, None);
    torso.taper(0.52, 0.6, (0.1, 0.1), (0.09, 0.09), (0.0, 0.0), &skin, None);
    let mut head = Mesh::new();
    head.taper(0.0, 0.25, (0.18, 0.2), (0.2, 0.22), (0.01, 0.0), &skin, Some(&face));
    head.taper(0.2, 0.27, (0.21, 0.22), (0.17, 0.19), (-0.01, -0.02), &hair, None);
    let mut upper_arm = skin_box(-0.3, 0.0, (0.085, 0.085), (0.11, 0.11), &shirt);
    upper_arm.taper(-0.3, -0.18, (0.09, 0.09), (0.1, 0.1), (0.0, 0.0), &skin, None);
    let mut forearm = skin_box(-0.27, 0.0, (0.07, 0.07), (0.085, 0.085), &skin);
    forearm.taper(-0.37, -0.26, (0.05, 0.04), (0.08, 0.05), (0.0, 0.0), &skin, None);
    let thigh = skin_box(-0.43, 0.0, (0.12, 0.13), (0.17, 0.17), &pants);
    let mut shin = skin_box(-0.4, 0.0, (0.1, 0.11), (0.12, 0.13), &pants);
    shin.taper(-0.47, -0.38, (0.11, 0.13), (0.11, 0.12), (0.01, 0.0), &shoe, None);
    let mut toe = Mesh::new();
    toe.block(v3(-0.05, -0.47, 0.0), v3(0.05, -0.41, 0.14), &shoe);
    shin.append(&toe, &Mat4::IDENTITY);
    Body {
        pelvis,
        torso,
        head,
        ponytail: None,
        upper_arm,
        forearm,
        thigh,
        shin,
        gun: Mesh::new(),
        holster: Mesh::new(),
        scale: 1.04,
    }
}

/// Matrices des articulations calculées pour une pose.
pub struct Skeleton {
    pub pelvis: Mat4,
    pub torso: Mat4,
    pub head: Mat4,
    pub upper: [Mat4; 2],
    pub fore: [Mat4; 2],
    pub thigh: [Mat4; 2],
    pub shin: [Mat4; 2],
}

pub fn skeleton(pose: &Pose, pos: Vec3, yaw: f32, scale: f32) -> Skeleton {
    let root = Mat4::translate(pos)
        * Mat4::rot_y(yaw)
        * Mat4::scale(v3(scale, scale, scale))
        * Mat4::translate(v3(0.0, pose.root_y, 0.0))
        * Mat4::rot_x(pose.root_pitch)
        * Mat4::rot_z(pose.root_roll);
    let pelvis = root * Mat4::translate(v3(0.0, 0.95, 0.0));
    let torso = pelvis
        * Mat4::translate(v3(0.0, 0.05, 0.0))
        * Mat4::rot_y(pose.torso_yaw)
        * Mat4::rot_x(pose.torso_pitch)
        * Mat4::rot_z(pose.torso_roll);
    let head = torso * Mat4::translate(v3(0.0, 0.57, 0.0)) * Mat4::rot_y(pose.head_yaw) * Mat4::rot_x(pose.head_pitch) * Mat4::rot_z(pose.head_roll);
    let mut upper = [Mat4::IDENTITY; 2];
    let mut fore = [Mat4::IDENTITY; 2];
    let mut thigh = [Mat4::IDENTITY; 2];
    let mut shin = [Mat4::IDENTITY; 2];
    for i in 0..2 {
        let s = if i == 0 { -1.0 } else { 1.0 };
        upper[i] = torso * Mat4::translate(v3(s * 0.2, 0.46, 0.0)) * Mat4::rot_x(-pose.arm_fwd[i]) * Mat4::rot_z(s * pose.arm_out[i]);
        fore[i] = upper[i] * Mat4::translate(v3(0.0, -0.28, 0.0)) * Mat4::rot_x(-pose.elbow[i]);
        thigh[i] = pelvis * Mat4::translate(v3(s * 0.09, -0.05, 0.0)) * Mat4::rot_x(-pose.hip[i]) * Mat4::rot_z(s * pose.hip_out[i]);
        shin[i] = thigh[i] * Mat4::translate(v3(0.0, -0.43, 0.0)) * Mat4::rot_x(pose.knee[i]);
    }
    Skeleton { pelvis, torso, head, upper, fore, thigh, shin }
}

pub fn draw_body(r: &mut Raster, b: &Body, pose: &Pose, pos: Vec3, yaw: f32, light: &Lighting) -> Skeleton {
    let sk = skeleton(pose, pos, yaw, b.scale);
    r.draw_mesh(&b.pelvis, &sk.pelvis, Some(light));
    r.draw_mesh(&b.torso, &sk.torso, Some(light));
    r.draw_mesh(&b.head, &sk.head, Some(light));
    if let Some(p) = &b.ponytail {
        let m = sk.head * Mat4::translate(v3(0.0, 0.2, -0.11)) * Mat4::rot_x(pose.pony + 0.25);
        r.draw_mesh(p, &m, Some(light));
    }
    for i in 0..2 {
        r.draw_mesh(&b.upper_arm, &sk.upper[i], Some(light));
        r.draw_mesh(&b.forearm, &sk.fore[i], Some(light));
        r.draw_mesh(&b.thigh, &sk.thigh[i], Some(light));
        r.draw_mesh(&b.shin, &sk.shin[i], Some(light));
    }
    if pose.gun {
        r.draw_mesh(&b.gun, &sk.fore[1], Some(light));
    }
    r.draw_mesh(&b.holster, &sk.thigh[1], Some(light));
    sk
}
