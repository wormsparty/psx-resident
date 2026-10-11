//! Options du joueur, stockage persistant (fichiers en natif, localStorage
//! sur le web) et requêtes du jeu vers la plateforme.

use std::collections::HashMap;

/// Tailles de fenêtre proposées (mode fenêtré, natif uniquement).
pub const RESOLUTIONS: [(u32, u32); 5] = [(640, 480), (960, 720), (1280, 960), (1600, 1200), (1920, 1440)];

#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    pub fullscreen: bool,
    pub resolution: usize,
    /// Mise à l'échelle par multiples entiers (pixels parfaits) ou étirée en 4:3.
    pub integer_scale: bool,
    pub master: u8,
    pub sfx: u8,
    pub ambient: u8,
    /// Contrôles « tank » des épisodes PS1.
    pub classic: bool,
    pub show_fps: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options { fullscreen: true, resolution: 1, integer_scale: false, master: 8, sfx: 8, ambient: 7, classic: false, show_fps: false }
    }
}

impl Options {
    pub fn encode(&self) -> String {
        let b = |v: bool| if v { 1 } else { 0 };
        format!(
            "fullscreen={}\nresolution={}\ninteger_scale={}\nmaster={}\nsfx={}\nambient={}\nclassic={}\nshow_fps={}\n",
            b(self.fullscreen),
            self.resolution,
            b(self.integer_scale),
            self.master,
            self.sfx,
            self.ambient,
            b(self.classic),
            b(self.show_fps)
        )
    }

    /// Décode en partant des valeurs par défaut ; les clés inconnues sont ignorées.
    pub fn decode(s: &str) -> Options {
        let mut o = Options::default();
        for line in s.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            let n: u32 = v.trim().parse().unwrap_or(0);
            match k.trim() {
                "fullscreen" => o.fullscreen = n != 0,
                "resolution" => o.resolution = (n as usize).min(RESOLUTIONS.len() - 1),
                "integer_scale" => o.integer_scale = n != 0,
                "master" => o.master = n.min(10) as u8,
                "sfx" => o.sfx = n.min(10) as u8,
                "ambient" => o.ambient = n.min(10) as u8,
                "classic" => o.classic = n != 0,
                "show_fps" => o.show_fps = n != 0,
                _ => {}
            }
        }
        o
    }

    pub fn window_size(&self) -> (u32, u32) {
        RESOLUTIONS[self.resolution.min(RESOLUTIONS.len() - 1)]
    }

    /// Rectangle (x, y, largeur, hauteur) de l'image 4:3 dans une surface donnée.
    pub fn viewport(&self, w: u32, h: u32) -> (u32, u32, u32, u32) {
        let (bw, bh) = (crate::raster::W as u32, crate::raster::H as u32);
        let (vw, vh) = if self.integer_scale {
            let s = (w / bw).min(h / bh).max(1);
            (bw * s, bh * s)
        } else {
            let vw = w.min(h * 4 / 3).max(1);
            (vw, (vw * 3 / 4).max(1))
        };
        (w.saturating_sub(vw) / 2, h.saturating_sub(vh) / 2, vw, vh)
    }
}

/// Stockage clé/valeur persistant fourni par la plateforme.
pub trait Storage {
    fn load(&self, key: &str) -> Option<String>;
    fn save(&mut self, key: &str, data: &str);
}

/// Stockage en mémoire (tests, captures).
#[derive(Default)]
pub struct MemStorage(pub HashMap<String, String>);

impl Storage for MemStorage {
    fn load(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }
    fn save(&mut self, key: &str, data: &str) {
        self.0.insert(key.to_string(), data.to_string());
    }
}

/// Actions que le jeu demande à la plateforme.
#[derive(Clone, Debug, PartialEq)]
pub enum Request {
    Fullscreen(bool),
    WindowSize(u32, u32),
    Quit,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_roundtrip() {
        let o = Options { fullscreen: false, resolution: 3, integer_scale: true, master: 4, sfx: 10, ambient: 0, classic: true, show_fps: true };
        assert_eq!(Options::decode(&o.encode()), o);
        assert_eq!(Options::decode("n'importe quoi"), Options::default());
    }
}
