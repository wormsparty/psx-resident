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
    pub shotgun: Mesh,
    pub knife: Mesh,
    pub holster: Mesh,
    /// Toujours dessiné sur l'avant-bras droit (griffe du Sujet 7...).
    pub right_hand: Mesh,
    pub scale: f32,
}

/// Objet tenu en main droite.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Held {
    #[default]
    None,
    Pistol,
    Shotgun,
    Knife,
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
    pub held: Held,
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
            held: if t < 0.5 { self.held } else { o.held },
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
            held: Held::Pistol,
            ..Default::default()
        }
    }

    /// Fusil épaulé : main gauche sous le canon, main droite à la crosse.
    pub fn aim_shotgun(recoil: f32) -> Pose {
        Pose {
            torso_yaw: 0.45,
            torso_pitch: -0.04 - recoil * 0.12,
            head_yaw: -0.35,
            head_pitch: 0.1,
            arm_fwd: [1.2 + recoil * 0.25, 0.75 + recoil * 0.3],
            arm_out: [-0.75, -0.25],
            elbow: [0.35, 0.95],
            hip: [0.2, -0.15],
            hip_out: [0.1, 0.15],
            knee: [0.25, 0.1],
            held: Held::Shotgun,
            ..Default::default()
        }
    }

    /// Garde au couteau ; `swing` dans [0,1] : armé, coup, retour.
    pub fn knife(swing: f32) -> Pose {
        let wind = smoothstep(0.0, 0.3, swing) * (1.0 - smoothstep(0.3, 0.55, swing));
        let cut = smoothstep(0.3, 0.55, swing) * (1.0 - smoothstep(0.7, 1.0, swing));
        Pose {
            root_y: -0.06,
            torso_yaw: -0.3 + wind * 0.8 - cut * 0.7,
            torso_pitch: 0.2 + cut * 0.15,
            head_pitch: 0.05,
            head_yaw: 0.2 - wind * 0.4,
            arm_fwd: [0.6, 1.0 + wind * 0.6 + cut * 0.4],
            arm_out: [0.3, -0.2 + wind * 0.9 - cut * 1.1],
            elbow: [1.4, 0.7 - cut * 0.5 + wind * 0.6],
            hip: [0.35, -0.25],
            knee: [0.45, 0.35],
            held: Held::Knife,
            ..Default::default()
        }
    }

    /// Projetée au sol (`k` = 1 : à plat sur le dos), puis relevée.
    pub fn knocked(k: f32) -> Pose {
        let mut lying = Pose::lying(0.2);
        lying.root_pitch = -1.45;
        Pose::idle(0.0).lerp(&lying, k)
    }

    /// Assis contre un mur, jambes allongées (majordome blessé).
    pub fn sitting(t: f32) -> Pose {
        let b = (t * 1.2).sin();
        Pose {
            root_y: -0.78,
            torso_pitch: -0.25 + 0.03 * b,
            head_pitch: 0.45,
            head_roll: 0.25,
            arm_fwd: [0.55, 0.35],
            arm_out: [0.25, 0.15],
            elbow: [1.2, 0.4],
            hip: [1.5, 1.4],
            hip_out: [0.12, 0.2],
            knee: [0.1, 0.3],
            ..Default::default()
        }
    }

    /// Course lourde du Sujet 7, bras griffu en arrière.
    pub fn brute_run(phase: f32, t: f32) -> Pose {
        let mut p = Pose::walk(phase, 1.0);
        p.torso_pitch = 0.4;
        p.head_pitch = -0.2;
        p.arm_out = [0.25, 0.35];
        p.elbow = [0.9, 0.5];
        p.head_roll = 0.05 * (t * 3.0).sin();
        p
    }

    /// Coup de griffe du Sujet 7 ; `s` dans [0,1] : armé puis frappe.
    pub fn brute_slash(s: f32) -> Pose {
        let wind = smoothstep(0.0, 0.45, s) * (1.0 - smoothstep(0.5, 0.65, s));
        let cut = smoothstep(0.5, 0.65, s) * (1.0 - smoothstep(0.8, 1.0, s));
        Pose {
            torso_yaw: wind * 0.7 - cut * 0.8,
            torso_pitch: 0.25 + cut * 0.35,
            head_pitch: -0.1,
            arm_fwd: [0.8, 0.4 + wind * 2.2 + cut * 0.4],
            arm_out: [0.3, 0.2 + wind * 0.8 - cut * 0.9],
            elbow: [0.6, 0.3 + wind * 0.8],
            hip: [0.4, -0.3],
            knee: [0.5, 0.3],
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
    // Fusil à pompe : canon et magasin tubulaire vers l'avant, crosse vers le coude.
    let mut shotgun = Mesh::new();
    let walnut = Mat::tex(T_WOOD, [0.55, 0.38, 0.28]);
    shotgun.block(v3(-0.02, -1.0, 0.02), v3(0.02, -0.3, 0.06), &metal);
    shotgun.block(v3(-0.022, -0.8, -0.03), v3(0.022, -0.5, 0.02), &walnut);
    shotgun.block(v3(-0.03, -0.38, -0.05), v3(0.03, -0.22, 0.07), &metal);
    shotgun.block(v3(-0.028, -0.25, -0.07), v3(0.028, 0.12, 0.03), &walnut);
    // Couteau de combat : lame vers l'avant.
    let mut knife = Mesh::new();
    knife.block(v3(-0.012, -0.42, -0.02), v3(0.012, -0.32, 0.03), &Mat::col([0.08, 0.08, 0.08]));
    knife.block(v3(-0.004, -0.62, -0.015), v3(0.004, -0.42, 0.025), &Mat::col([0.75, 0.76, 0.8]));
    // Holster de cuisse (jambe droite) et ses sangles.
    let mut holster = Mesh::new();
    holster.block(v3(0.075, -0.27, -0.05), v3(0.115, -0.04, 0.05), &belt);
    holster.block(v3(0.08, -0.06, -0.06), v3(0.11, 0.03, -0.02), &metal);
    for y in [-0.11, -0.22] {
        holster.prism(y - 0.013, y + 0.013, (0.15, 0.16), (0.155, 0.165), (0.0, 0.0), 8, &belt, false);
    }

    Body {
        pelvis,
        torso,
        head,
        ponytail: None,
        upper_arm,
        forearm,
        thigh,
        shin,
        gun,
        shotgun,
        knife,
        holster,
        right_hand: Mesh::new(),
        scale: 1.0,
    }
}

/// Corps sans accessoires (PNJ, créatures).
#[allow(clippy::too_many_arguments)]
fn plain(pelvis: Mesh, torso: Mesh, head: Mesh, upper_arm: Mesh, forearm: Mesh, thigh: Mesh, shin: Mesh, scale: f32) -> Body {
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
        shotgun: Mesh::new(),
        knife: Mesh::new(),
        holster: Mesh::new(),
        right_hand: Mesh::new(),
        scale,
    }
}

/// Edgar, le vieux majordome : queue-de-pie noire, gilet, chemise blanche.
/// `turned` : version zombifiée (teint verdâtre, chemise ensanglantée).
pub fn butler(turned: bool) -> Body {
    let skin = Mat::col(if turned { [0.55, 0.58, 0.47] } else { [0.74, 0.62, 0.55] });
    let coat = Mat::tex(T_FABRIC, [0.08, 0.08, 0.1]);
    let shirt = if turned { Mat::tex(T_BLOODY, [1.0, 1.0, 1.0]) } else { Mat::tex(T_FABRIC, [0.85, 0.85, 0.82]) };
    let pants = Mat::tex(T_FABRIC, [0.1, 0.1, 0.12]);
    let shoe = Mat::col([0.04, 0.04, 0.04]);
    let face = if turned { Mat::tex(T_ZFACE, [1.0, 1.0, 1.0]) } else { Mat::tex(T_OLDFACE, [1.0, 1.0, 1.0]) };
    let grey = Mat::col([0.6, 0.6, 0.58]);
    let pelvis = skin_box(-0.13, 0.05, (0.32, 0.21), (0.3, 0.2), &pants);
    let mut torso = Mesh::new();
    torso.taper(0.0, 0.52, (0.29, 0.19), (0.38, 0.21), (0.0, 0.0), &coat, None);
    // Plastron blanc et nœud papillon.
    torso.block(v3(-0.07, 0.22, 0.09), v3(0.07, 0.5, 0.115), &shirt);
    torso.block(v3(-0.05, 0.47, 0.11), v3(0.05, 0.51, 0.125), &Mat::col([0.05, 0.05, 0.05]));
    // Basques de la queue-de-pie.
    torso.taper(-0.45, 0.02, (0.26, 0.04), (0.3, 0.05), (-0.11, -0.1), &coat, None);
    torso.taper(0.52, 0.6, (0.09, 0.09), (0.085, 0.085), (0.0, 0.0), &skin, None);
    let mut head = Mesh::new();
    head.taper(0.0, 0.24, (0.17, 0.19), (0.19, 0.21), (0.01, 0.0), &skin, Some(&face));
    head.taper(0.19, 0.26, (0.2, 0.21), (0.15, 0.17), (-0.01, -0.02), &grey, None);
    let upper_arm = skin_box(-0.3, 0.0, (0.085, 0.085), (0.105, 0.105), &coat);
    let mut forearm = skin_box(-0.27, 0.0, (0.075, 0.075), (0.085, 0.085), &coat);
    forearm.taper(-0.36, -0.26, (0.05, 0.04), (0.07, 0.05), (0.0, 0.0), &skin, None);
    let thigh = skin_box(-0.43, 0.0, (0.11, 0.12), (0.15, 0.16), &pants);
    let mut shin = skin_box(-0.4, 0.0, (0.09, 0.1), (0.11, 0.12), &pants);
    shin.block(v3(-0.05, -0.47, -0.06), v3(0.05, -0.4, 0.14), &shoe);
    plain(pelvis, torso, head, upper_arm, forearm, thigh, shin, 1.0)
}

/// Le Sujet 7 : colosse chauve à la peau grise, bras droit terminé par une griffe osseuse.
pub fn brute() -> Body {
    let skin = Mat::col([0.5, 0.46, 0.48]);
    let scar = Mat::tex(T_FLESH, [0.9, 0.8, 0.8]);
    let pants = Mat::tex(T_FABRIC, [0.32, 0.3, 0.26]);
    let strap = Mat::col([0.12, 0.09, 0.07]);
    let bone = Mat::col([0.8, 0.76, 0.62]);
    let face = Mat::tex(T_BRUTEFACE, [1.0, 1.0, 1.0]);
    let mut pelvis = skin_box(-0.13, 0.05, (0.38, 0.24), (0.36, 0.22), &pants);
    pelvis.taper(-0.01, 0.06, (0.39, 0.25), (0.38, 0.24), (0.0, 0.0), &strap, None);
    let mut torso = Mesh::new();
    torso.prism(0.0, 0.55, (0.34, 0.24), (0.52, 0.3), (0.0, 0.0), 8, &skin, false);
    // Cicatrice de dissection au sternum, sangles de contention.
    torso.block(v3(-0.06, 0.1, 0.11), v3(0.06, 0.5, 0.16), &scar);
    torso.prism(0.3, 0.36, (0.48, 0.3), (0.5, 0.31), (0.0, 0.0), 8, &strap, false);
    torso.prism(0.48, 0.62, (0.16, 0.16), (0.14, 0.14), (0.0, 0.0), 6, &skin, false);
    let mut head = Mesh::new();
    head.taper(0.0, 0.24, (0.19, 0.21), (0.21, 0.23), (0.02, 0.0), &skin, Some(&face));
    let upper_arm = skin_box(-0.3, 0.0, (0.12, 0.12), (0.16, 0.16), &skin);
    let mut forearm = skin_box(-0.27, 0.0, (0.1, 0.1), (0.13, 0.13), &skin);
    forearm.taper(-0.36, -0.26, (0.07, 0.05), (0.1, 0.07), (0.0, 0.0), &skin, None);
    let thigh = skin_box(-0.43, 0.0, (0.15, 0.16), (0.2, 0.21), &pants);
    let mut shin = skin_box(-0.4, 0.0, (0.12, 0.13), (0.15, 0.16), &skin);
    shin.block(v3(-0.06, -0.47, -0.06), v3(0.06, -0.4, 0.15), &skin);
    let mut b = plain(pelvis, torso, head, upper_arm, forearm, thigh, shin, 1.32);
    // Griffe : trois lames d'os qui prolongent la main droite.
    let mut claw = Mesh::new();
    claw.taper(-0.3, -0.1, (0.18, 0.1), (0.14, 0.14), (0.0, 0.0), &scar, None);
    for (k, x) in [-0.06f32, 0.0, 0.06].iter().enumerate() {
        let len = if k == 1 { 0.62 } else { 0.5 };
        claw.taper(-0.3 - len, -0.28, (0.005, 0.005), (0.035, 0.05), (0.06, 0.0), &bone, None);
        let last = claw.v.len();
        for v in &mut claw.v[last - 24..] {
            v.p.x += x;
        }
    }
    b.right_hand = claw;
    b
}

/// Le Docteur fondu dans l'Abomination (buste seul, sans jambes).
fn doctor_torso() -> Body {
    let skin = Mat::col([0.7, 0.66, 0.6]);
    let coat = Mat::tex(T_BLOODY, [0.95, 0.95, 0.9]);
    let face = Mat::tex(T_ZFACE, [1.1, 1.1, 1.1]);
    let mut torso = Mesh::new();
    torso.taper(0.0, 0.5, (0.26, 0.18), (0.36, 0.2), (0.0, 0.0), &coat, None);
    torso.taper(0.5, 0.58, (0.09, 0.09), (0.085, 0.085), (0.0, 0.0), &skin, None);
    let mut head = Mesh::new();
    head.taper(0.0, 0.24, (0.17, 0.19), (0.19, 0.21), (0.01, 0.0), &skin, Some(&face));
    head.taper(0.18, 0.25, (0.2, 0.21), (0.16, 0.17), (-0.01, -0.02), &Mat::col([0.75, 0.75, 0.72]), None);
    let upper_arm = skin_box(-0.3, 0.0, (0.08, 0.08), (0.1, 0.1), &coat);
    let mut forearm = skin_box(-0.27, 0.0, (0.07, 0.07), (0.08, 0.08), &skin);
    forearm.taper(-0.4, -0.26, (0.03, 0.02), (0.07, 0.05), (0.0, 0.0), &skin, None);
    plain(Mesh::new(), torso, head, upper_arm, forearm, Mesh::new(), Mesh::new(), 1.0)
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
        shotgun: Mesh::new(),
        knife: Mesh::new(),
        holster: Mesh::new(),
        right_hand: Mesh::new(),
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
    skeleton_xf(pose, Mat4::translate(pos) * Mat4::rot_y(yaw) * Mat4::scale(v3(scale, scale, scale)))
}

/// Squelette à partir d'un repère racine quelconque (pieds à l'origine).
pub fn skeleton_xf(pose: &Pose, base: Mat4) -> Skeleton {
    let root = base
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
    draw_body_xf(r, b, pose, Mat4::translate(pos) * Mat4::rot_y(yaw) * Mat4::scale(v3(b.scale, b.scale, b.scale)), light)
}

pub fn draw_body_xf(r: &mut Raster, b: &Body, pose: &Pose, base: Mat4, light: &Lighting) -> Skeleton {
    let sk = skeleton_xf(pose, base);
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
    match pose.held {
        Held::Pistol => r.draw_mesh(&b.gun, &sk.fore[1], Some(light)),
        Held::Shotgun => r.draw_mesh(&b.shotgun, &sk.fore[1], Some(light)),
        Held::Knife => r.draw_mesh(&b.knife, &sk.fore[1], Some(light)),
        Held::None => {}
    }
    r.draw_mesh(&b.right_hand, &sk.fore[1], Some(light));
    r.draw_mesh(&b.holster, &sk.thigh[1], Some(light));
    sk
}

// ---------------------------------------------------------------- chiens

pub struct DogBody {
    pub torso: Mesh,
    pub head: Mesh,
    pub jaw: Mesh,
    pub upper_leg: Mesh,
    pub lower_leg: Mesh,
    pub tail: Mesh,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DogPose {
    /// Phase du galop (radians) et amplitude (0 : arrêt, 1 : course).
    pub phase: f32,
    pub gait: f32,
    pub body_pitch: f32,
    pub head_pitch: f32,
    pub jaw: f32,
    /// Hauteur du saut.
    pub lift: f32,
    /// 0 : debout, 1 : couché sur le flanc (mort).
    pub dead: f32,
    pub crouch: f32,
}

/// Chien du marais. `hound` : le Molosse (pelage noir, yeux rouges, échine osseuse).
pub fn dog(hound: bool) -> DogBody {
    let fur = Mat::tex(T_FUR, if hound { [0.55, 0.5, 0.5] } else { [1.0, 1.0, 1.0] });
    let dark = Mat::col(if hound { [0.05, 0.04, 0.04] } else { [0.1, 0.08, 0.07] });
    let gum = Mat::col([0.45, 0.08, 0.08]);
    let tooth = Mat::col([0.85, 0.8, 0.65]);
    let eye = Mat::col(if hound { [1.0, 0.15, 0.05] } else { [0.95, 0.75, 0.15] }).glow(2.0);
    let rot = Mat4::rot_x(std::f32::consts::FRAC_PI_2);
    // Torse le long de +z (prisme sur y, couché par rotation).
    let mut t = Mesh::new();
    t.prism(-0.48, 0.5, (0.28, 0.3), (0.34, 0.4), (0.0, 0.02), 8, &fur, true);
    let mut torso = Mesh::new();
    torso.append(&t, &(Mat4::translate(v3(0.0, 0.58, 0.0)) * rot));
    if hound {
        // Vertèbres saillantes sur l'échine.
        let bone = Mat::col([0.75, 0.7, 0.58]);
        for k in 0..6 {
            let z = -0.4 + k as f32 * 0.16;
            torso.taper(0.72, 0.84, (0.06, 0.06), (0.01, 0.01), (0.0, 0.0), &bone, None);
            let last = torso.v.len();
            for v in &mut torso.v[last - 24..] {
                v.p.z += z;
                v.p.y += (k as f32 * 0.5).sin() * 0.02;
            }
        }
    }
    // Tête : crâne, museau, oreilles, yeux luisants. Origine à la nuque.
    let mut head = Mesh::new();
    head.block(v3(-0.1, -0.08, 0.0), v3(0.1, 0.1, 0.2), &fur);
    head.block(v3(-0.06, -0.08, 0.2), v3(0.06, 0.03, 0.38), &fur);
    head.block(v3(-0.03, 0.0, 0.37), v3(0.03, 0.035, 0.395), &dark);
    for x in [-0.07f32, 0.07] {
        head.tri([v3(x - 0.035, 0.1, 0.06), v3(x + 0.035, 0.1, 0.06), v3(x, 0.2, 0.02)], [[0.0; 2]; 3], &dark.two(), v3(0.0, 0.0, 1.0));
        head.block(v3(x - 0.022, 0.035, 0.195), v3(x + 0.022, 0.06, 0.205), &eye);
    }
    // Mâchoire et crocs (articulée).
    let mut jaw = Mesh::new();
    jaw.block(v3(-0.05, -0.05, 0.0), v3(0.05, 0.0, 0.17), &fur);
    jaw.block(v3(-0.045, 0.0, 0.0), v3(0.045, 0.01, 0.16), &gum);
    for x in [-0.035f32, 0.035] {
        jaw.block(v3(x - 0.008, 0.0, 0.13), v3(x + 0.008, 0.05, 0.15), &tooth);
    }
    let mut upper_leg = Mesh::new();
    upper_leg.prism(-0.3, 0.04, (0.07, 0.09), (0.11, 0.15), (0.0, 0.0), 6, &fur, true);
    let mut lower_leg = Mesh::new();
    lower_leg.prism(-0.28, 0.0, (0.045, 0.05), (0.06, 0.07), (0.0, 0.0), 6, &fur, false);
    lower_leg.block(v3(-0.035, -0.31, -0.02), v3(0.035, -0.26, 0.08), &dark);
    let mut tl = Mesh::new();
    tl.prism(-0.38, 0.0, (0.02, 0.02), (0.06, 0.06), (0.0, 0.0), 5, &fur, false);
    let tail = tl;
    DogBody { torso, head, jaw, upper_leg, lower_leg, tail }
}

pub fn draw_dog(r: &mut Raster, b: &DogBody, pose: &DogPose, pos: Vec3, yaw: f32, scale: f32, light: &Lighting) {
    let p = pose;
    let root = Mat4::translate(pos + v3(0.0, p.lift - p.dead * 0.3 * scale - p.crouch * 0.12 * scale, 0.0))
        * Mat4::rot_y(yaw)
        * Mat4::scale(v3(scale, scale, scale))
        * Mat4::rot_z(p.dead * 1.45)
        * Mat4::translate(v3(0.0, 0.0, 0.0))
        * Mat4::rot_x(p.body_pitch);
    r.draw_mesh(&b.torso, &root, Some(light));
    let neck = root * Mat4::translate(v3(0.0, 0.68 - p.crouch * 0.08, 0.46)) * Mat4::rot_x(p.head_pitch - 0.15);
    r.draw_mesh(&b.head, &neck, Some(light));
    let jaw = neck * Mat4::translate(v3(0.0, -0.06, 0.2)) * Mat4::rot_x(p.jaw * 0.7);
    r.draw_mesh(&b.jaw, &jaw, Some(light));
    // Pattes : avant (z +) et arrière (z -), galop décalé.
    let legs = [(-0.1f32, 0.36f32, 0.0f32), (0.1, 0.36, 0.35), (-0.1, -0.38, 3.3), (0.1, -0.38, 3.65)];
    for (i, &(x, z, off)) in legs.iter().enumerate() {
        let s = (p.phase + off).sin();
        let front = i < 2;
        let amp = 0.25 + 0.4 * p.gait;
        let swing = s * amp * p.gait.min(1.0) + if p.dead > 0.0 { p.dead * if front { 0.6 } else { -0.5 } } else { 0.0 };
        let knee = ((p.phase + off + 1.2).sin().max(0.0) * 0.9 * p.gait + p.crouch * 0.9) * if front { -1.0 } else { 1.0 };
        let hip = root * Mat4::translate(v3(x, 0.56, z)) * Mat4::rot_x(-swing + if front { 0.0 } else { p.crouch * 0.6 });
        r.draw_mesh(&b.upper_leg, &hip, Some(light));
        let low = hip * Mat4::translate(v3(0.0, -0.28, 0.0)) * Mat4::rot_x(knee * 0.8 + if front { 0.15 } else { -0.25 });
        r.draw_mesh(&b.lower_leg, &low, Some(light));
    }
    let wag = (p.phase * 0.5).sin() * 0.3;
    let tail = root * Mat4::translate(v3(0.0, 0.66, -0.48)) * Mat4::rot_x(-2.2 + p.gait * 0.6) * Mat4::rot_z(wag);
    r.draw_mesh(&b.tail, &tail, Some(light));
}

// ---------------------------------------------------------------- l'Abomination

/// Le Docteur Ashcroft, fondu dans une masse de chair de près de quatre mètres.
pub struct BossBody {
    pub pelvis: Mesh,
    pub torso: Mesh,
    pub heart: Mesh,
    pub flap: Mesh,
    pub head: Mesh,
    pub jaw: Mesh,
    pub upper_arm: Mesh,
    pub claw_arm: Mesh,
    pub blade_arm: Mesh,
    pub thigh: Mesh,
    pub shin: Mesh,
    pub doctor: Body,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BossPose {
    pub phase: f32,
    pub walk: f32,
    /// Coup de griffe (bras droit) : 0 repos, ]0,1] armé → frappe → retour.
    pub swipe: f32,
    /// Coup de lame (bras gauche).
    pub stab: f32,
    /// Écrasement à deux bras.
    pub slam: f32,
    /// Rugissement : gueule ouverte, plastron écarté sur le cœur.
    pub roar: f32,
    pub flinch: f32,
    /// Effondrement final.
    pub death: f32,
    /// Sortie de la fosse (intro) : 0 enfoui, 1 debout.
    pub rise: f32,
    pub t: f32,
}

pub fn abomination() -> BossBody {
    let flesh = Mat::tex(T_FLESH, [1.0, 1.0, 1.0]);
    let pale = Mat::tex(T_FLESH, [1.2, 1.15, 1.05]);
    let bone = Mat::col([0.8, 0.75, 0.6]);
    let dark = Mat::col([0.12, 0.02, 0.03]);
    let eye = Mat::col([1.0, 0.85, 0.2]).glow(2.5);
    let mut pelvis = Mesh::new();
    pelvis.prism(-0.35, 0.3, (1.1, 0.8), (1.25, 0.9), (0.0, 0.0), 10, &flesh, true);
    let mut torso = Mesh::new();
    // Masse du torse, bosse dorsale, épaules noueuses.
    torso.prism(0.0, 1.35, (1.15, 0.85), (1.7, 1.1), (0.0, 0.05), 10, &flesh, true);
    torso.prism(0.6, 1.6, (1.1, 0.9), (0.6, 0.5), (-0.35, -0.45), 8, &pale, true);
    for x in [-0.8f32, 0.8] {
        let start = torso.v.len();
        torso.prism(1.0, 1.45, (0.6, 0.6), (0.4, 0.4), (0.0, 0.0), 8, &flesh, true);
        for v in &mut torso.v[start..] {
            v.p.x += x;
        }
    }
    // Cavité thoracique : fond sombre et côtes.
    torso.quad([v3(-0.32, 0.45, 0.47), v3(0.32, 0.45, 0.47), v3(0.38, 1.15, 0.6), v3(-0.38, 1.15, 0.6)], [[0.0; 2]; 4], &dark, v3(0.0, 0.0, 1.0));
    for k in 0..4 {
        let y = 0.55 + k as f32 * 0.17;
        torso.block(v3(-0.42, y, 0.5 + k as f32 * 0.03), v3(-0.08, y + 0.05, 0.6 + k as f32 * 0.03), &bone);
        torso.block(v3(0.08, y, 0.5 + k as f32 * 0.03), v3(0.42, y + 0.05, 0.6 + k as f32 * 0.03), &bone);
    }
    // Yeux greffés sur les épaules.
    for (x, y, z) in [(-0.7f32, 1.3f32, 0.4f32), (0.75, 1.25, 0.42), (0.55, 1.38, 0.3), (-0.5, 1.12, 0.52)] {
        torso.block(v3(x - 0.04, y - 0.03, z), v3(x + 0.04, y + 0.03, z + 0.05), &eye);
    }
    // Cœur : point faible, pulsant (dessiné à part).
    let mut heart = Mesh::new();
    heart.prism(-0.18, 0.18, (0.2, 0.18), (0.26, 0.22), (0.0, 0.0), 8, &Mat::col([0.9, 0.12, 0.1]).glow(1.6), true);
    heart.prism(0.14, 0.3, (0.08, 0.08), (0.05, 0.05), (0.0, 0.0), 6, &Mat::col([0.5, 0.1, 0.25]).glow(0.6), true);
    // Plastron d'os qui protège le cœur (deux battants).
    let mut flap = Mesh::new();
    flap.block(v3(0.0, -0.38, -0.04), v3(0.4, 0.38, 0.05), &bone);
    flap.block(v3(0.05, -0.3, 0.05), v3(0.35, 0.3, 0.09), &pale);
    // Tête : crâne allongé, gueule immense.
    let mut head = Mesh::new();
    head.taper(-0.1, 0.42, (0.45, 0.55), (0.35, 0.4), (0.1, 0.0), &pale, None);
    head.taper(-0.05, 0.12, (0.4, 0.15), (0.42, 0.18), (0.38, 0.4), &bone, None);
    for x in [-0.12f32, 0.12] {
        head.block(v3(x - 0.05, 0.2, 0.32), v3(x + 0.05, 0.27, 0.38), &eye);
    }
    for k in 0..5 {
        let x = -0.16 + k as f32 * 0.08;
        head.taper(-0.2, -0.04, (0.01, 0.01), (0.04, 0.04), (0.42, 0.42), &bone, None);
        let last = head.v.len();
        for v in &mut head.v[last - 24..] {
            v.p.x += x;
        }
    }
    let mut jaw = Mesh::new();
    jaw.taper(-0.12, 0.02, (0.36, 0.5), (0.4, 0.55), (0.2, 0.22), &flesh, None);
    jaw.block(v3(-0.17, 0.02, 0.3), v3(0.17, 0.05, 0.48), &dark);
    for k in 0..4 {
        let x = -0.14 + k as f32 * 0.09;
        jaw.taper(0.02, 0.2, (0.04, 0.04), (0.005, 0.005), (0.44, 0.44), &bone, None);
        let last = jaw.v.len();
        for v in &mut jaw.v[last - 24..] {
            v.p.x += x;
        }
    }
    // Bras : épaule massive, avant-bras droit à trois griffes, gauche en lame d'os.
    let mut upper_arm = Mesh::new();
    upper_arm.prism(-1.05, 0.15, (0.32, 0.32), (0.5, 0.5), (0.0, 0.0), 8, &flesh, true);
    let mut claw_arm = Mesh::new();
    claw_arm.prism(-1.0, 0.05, (0.42, 0.34), (0.3, 0.3), (0.0, 0.0), 8, &flesh, true);
    for (k, x) in [-0.13f32, 0.0, 0.13].iter().enumerate() {
        claw_arm.taper(-1.75 - k as f32 * 0.05, -0.95, (0.01, 0.01), (0.09, 0.11), (0.12, 0.0), &bone, None);
        let last = claw_arm.v.len();
        for v in &mut claw_arm.v[last - 24..] {
            v.p.x += x;
        }
    }
    let mut blade_arm = Mesh::new();
    blade_arm.prism(-0.9, 0.05, (0.26, 0.26), (0.3, 0.3), (0.0, 0.0), 8, &pale, true);
    blade_arm.taper(-2.0, -0.85, (0.02, 0.02), (0.08, 0.3), (0.05, 0.0), &bone, None);
    let mut thigh = Mesh::new();
    thigh.prism(-0.95, 0.1, (0.34, 0.34), (0.55, 0.55), (0.0, 0.0), 8, &flesh, true);
    let mut shin = Mesh::new();
    shin.prism(-0.85, 0.0, (0.28, 0.28), (0.38, 0.38), (0.0, 0.0), 8, &flesh, false);
    shin.block(v3(-0.25, -0.98, -0.2), v3(0.25, -0.82, 0.45), &pale);
    for x in [-0.15f32, 0.0, 0.15] {
        shin.block(v3(x - 0.04, -0.98, 0.45), v3(x + 0.04, -0.9, 0.62), &bone);
    }
    BossBody { pelvis, torso, heart, flap, head, jaw, upper_arm, claw_arm, blade_arm, thigh, shin, doctor: doctor_torso() }
}

/// Repère du cœur dans le monde (pour viser et faire gicler le sang).
pub fn boss_heart(p: &BossPose, pos: Vec3, yaw: f32) -> Vec3 {
    boss_frames(p, pos, yaw).1.point(v3(0.0, 0.8, 0.45))
}

fn boss_frames(p: &BossPose, pos: Vec3, yaw: f32) -> (Mat4, Mat4) {
    let sink = (1.0 - p.rise) * 3.4 + p.death * 1.2;
    let bob = (p.phase * 2.0).sin().abs() * 0.08 * p.walk;
    let root = Mat4::translate(pos + v3(0.0, -sink + bob, 0.0))
        * Mat4::rot_y(yaw)
        * Mat4::rot_z((p.phase).sin() * 0.05 * p.walk + p.death * 0.25)
        * Mat4::rot_x(p.death * 0.15);
    let lean = 0.35 + p.slam * 0.5 - p.roar * 0.45 + p.flinch * -0.3 + p.death * 0.9 + (1.0 - p.rise) * 0.4;
    let torso = root * Mat4::translate(v3(0.0, 1.95, 0.0)) * Mat4::rot_y(p.swipe * 0.4 - p.stab * 0.35) * Mat4::rot_x(lean);
    (root, torso)
}

pub fn draw_boss(r: &mut Raster, b: &BossBody, p: &BossPose, pos: Vec3, yaw: f32, light: &Lighting) {
    let (root, torso) = boss_frames(p, pos, yaw);
    let pelvis = root * Mat4::translate(v3(0.0, 1.8, 0.0));
    r.draw_mesh(&b.pelvis, &pelvis, Some(light));
    // Jambes digitigrades.
    for (i, x) in [-0.55f32, 0.55].iter().enumerate() {
        let s = (p.phase + i as f32 * std::f32::consts::PI).sin() * 0.4 * p.walk;
        let kneel = p.death * 1.2;
        let thigh = pelvis * Mat4::translate(v3(*x, -0.1, 0.0)) * Mat4::rot_x(-0.45 - s - kneel) * Mat4::rot_z(x.signum() * 0.12);
        r.draw_mesh(&b.thigh, &thigh, Some(light));
        let shin = thigh * Mat4::translate(v3(0.0, -0.9, 0.0)) * Mat4::rot_x(0.85 + s.max(0.0) * 0.6 + kneel * 0.8);
        r.draw_mesh(&b.shin, &shin, Some(light));
    }
    r.draw_mesh(&b.torso, &torso, Some(light));
    // Cœur pulsant et plastron.
    let beat = 1.0 + 0.12 * ((p.t * 6.5).sin().max(0.0)).powi(3) + p.roar * 0.15;
    let heart = torso * Mat4::translate(v3(0.0, 0.8, 0.45)) * Mat4::scale(v3(beat, beat, beat));
    r.draw_mesh(&b.heart, &heart, Some(light));
    let open = p.roar.max(p.flinch * 0.4) * 1.9;
    for s in [-1.0f32, 1.0] {
        let hinge = torso * Mat4::translate(v3(s * 0.42, 0.82, 0.62)) * Mat4::rot_y(-s * open) * Mat4::scale(v3(-s, 1.0, 1.0));
        r.draw_mesh(&b.flap, &hinge, Some(light));
    }
    // Tête et mâchoire.
    let head = torso * Mat4::translate(v3(0.0, 1.25, 0.45)) * Mat4::rot_x(-0.3 - p.roar * 0.5 + p.slam * 0.2 + (p.t * 1.3).sin() * 0.05);
    r.draw_mesh(&b.head, &head, Some(light));
    let jaw = head * Mat4::translate(v3(0.0, -0.05, 0.0)) * Mat4::rot_x(0.15 + p.roar * 0.9 + (p.t * 2.1).sin().max(0.0) * 0.1);
    r.draw_mesh(&b.jaw, &jaw, Some(light));
    // Bras.
    let idle = (p.t * 1.1).sin() * 0.08;
    let swing_r = {
        let w = smoothstep(0.0, 0.55, p.swipe) * (1.0 - smoothstep(0.6, 0.75, p.swipe));
        let c = smoothstep(0.6, 0.75, p.swipe) * (1.0 - smoothstep(0.85, 1.0, p.swipe));
        (w, c)
    };
    let swing_l = {
        let w = smoothstep(0.0, 0.55, p.stab) * (1.0 - smoothstep(0.6, 0.72, p.stab));
        let c = smoothstep(0.6, 0.72, p.stab) * (1.0 - smoothstep(0.85, 1.0, p.stab));
        (w, c)
    };
    let slam_up = smoothstep(0.0, 0.6, p.slam) * (1.0 - smoothstep(0.62, 0.72, p.slam));
    let slam_down = smoothstep(0.62, 0.72, p.slam) * (1.0 - smoothstep(0.85, 1.0, p.slam));
    for side in 0..2 {
        let s = if side == 0 { -1.0f32 } else { 1.0 };
        let (w, c) = if side == 1 { swing_r } else { swing_l };
        let fwd = 0.5 + idle + w * 2.2 + c * 0.6 + slam_up * 2.6 + slam_down * 1.1 + p.roar * 0.6 - p.death * 0.3 + (1.0 - p.rise) * 2.3;
        let out = 0.35 + w * 0.6 - c * 0.7 + p.roar * 1.0 + slam_up * 0.2;
        let elbow = 0.6 + w * 0.7 - c * 0.4 + slam_up * 0.3 + p.death * 0.6;
        let sh = torso * Mat4::translate(v3(s * 0.95, 1.15, 0.0)) * Mat4::rot_x(-fwd) * Mat4::rot_z(s * out);
        r.draw_mesh(&b.upper_arm, &sh, Some(light));
        let fore = sh * Mat4::translate(v3(0.0, -1.0, 0.0)) * Mat4::rot_x(-elbow);
        r.draw_mesh(if side == 1 { &b.claw_arm } else { &b.blade_arm }, &fore, Some(light));
    }
    // Le Docteur émerge de la bosse dorsale et se tord.
    let tw = (p.t * 2.3).sin();
    let dp = Pose {
        torso_pitch: -0.5 + 0.15 * tw - p.roar * 0.4,
        torso_roll: 0.2 * (p.t * 1.7).sin(),
        head_pitch: -0.4 + 0.3 * (p.t * 3.1).sin(),
        head_roll: 0.4,
        arm_fwd: [2.4 + 0.4 * tw + p.roar * 0.4, 2.0 - 0.5 * tw + p.roar * 0.6],
        arm_out: [0.9, 0.7],
        elbow: [0.6 + 0.3 * tw, 0.9],
        ..Default::default()
    };
    let base = torso * Mat4::translate(v3(0.0, 1.45, -0.6)) * Mat4::rot_x(-0.55) * Mat4::scale(v3(0.95, 0.95, 0.95)) * Mat4::translate(v3(0.0, -1.0, 0.0));
    draw_body_xf(r, &b.doctor, &dp, base, light);
}
