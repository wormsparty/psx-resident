//! Menus (titre, pause, options, commandes), interface en jeu et invites
//! contextuelles affichant les touches du dernier périphérique utilisé.

use super::*;
use crate::options::RESOLUTIONS;

/// Actions affichables dans une invite.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Act {
    Move,
    Turn,
    Run,
    Aim,
    Fire,
    Reload,
    Interact,
    Inventory,
    Pause,
    Confirm,
    Back,
    Change,
    QuickTurn,
}

#[derive(Clone, Copy)]
enum PadIcon {
    A,
    B,
    X,
    Y,
    LT,
    RT,
    Start,
    LStick,
    DPadH,
    DownB,
}

#[derive(Clone, Copy)]
enum Icon {
    Key(&'static str),
    Arrows,
    Pad(PadIcon),
}

#[derive(Clone, Copy, PartialEq)]
enum TitleItem {
    Continue,
    New,
    Options,
    Quit,
}

#[derive(Clone, Copy, PartialEq)]
enum PauseItem {
    Resume,
    Load,
    Options,
    Controls,
    MainMenu,
    Quit,
}

#[cfg(test)]
pub(super) use Opt as OptPub;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Opt {
    Display,
    Resolution,
    Scaling,
    Master,
    Sfx,
    Ambient,
    Controls,
    Fps,
    ShowControls,
    Back,
}

const ICON_H: i32 = 11;
const TEXT: [f32; 3] = [0.85, 0.83, 0.78];
const DIM: [f32; 3] = [0.5, 0.5, 0.48];
const GOLD: [f32; 3] = [1.0, 0.85, 0.4];

impl Game {
    // ------------------------------------------------------------ icônes

    fn icon_for(&self, act: Act) -> Icon {
        if self.pad {
            Icon::Pad(match act {
                Act::Move | Act::Turn => PadIcon::LStick,
                Act::Run | Act::Back => PadIcon::B,
                Act::Aim => PadIcon::LT,
                Act::Fire => PadIcon::RT,
                Act::Reload => PadIcon::X,
                Act::Interact | Act::Confirm => PadIcon::A,
                Act::Inventory => PadIcon::Y,
                Act::Pause => PadIcon::Start,
                Act::Change => PadIcon::DPadH,
                Act::QuickTurn => PadIcon::DownB,
            })
        } else {
            match act {
                Act::Change => Icon::Arrows,
                _ => Icon::Key(match act {
                    Act::Move => {
                        if self.opts.classic {
                            "W/S"
                        } else {
                            "WASD"
                        }
                    }
                    Act::Turn => "A/D",
                    Act::Run => "MAJ",
                    Act::Aim => "CLIC D",
                    Act::Fire => "CLIC G",
                    Act::Reload => "R",
                    Act::Interact => "E",
                    Act::Inventory => "TAB",
                    Act::Pause | Act::Back => "ÉCHAP",
                    Act::Confirm => "ENTRÉE",
                    Act::QuickTurn => "S+MAJ",
                    Act::Change => unreachable!(),
                }),
            }
        }
    }

    fn icon_width(icon: Icon) -> i32 {
        match icon {
            Icon::Key(s) => Raster::text_width(s, 1) + 6,
            Icon::Arrows => 25,
            Icon::Pad(p) => match p {
                PadIcon::LT | PadIcon::RT => 19,
                PadIcon::Start => 15,
                PadIcon::DownB => 24,
                _ => 11,
            },
        }
    }

    fn keycap(r: &mut Raster, x: i32, y: i32, w: i32) {
        r.fill_rect(x, y, w, ICON_H, [0.82, 0.8, 0.74], 1.0);
        r.fill_rect(x + 1, y + 1, w - 2, ICON_H - 3, [0.95, 0.93, 0.88], 1.0);
        r.fill_rect(x, y + ICON_H - 1, w, 1, [0.4, 0.38, 0.35], 1.0);
    }

    fn draw_icon(&mut self, x: i32, y: i32, icon: Icon) -> i32 {
        let w = Self::icon_width(icon);
        let r = &mut self.raster;
        let ink = [0.08, 0.08, 0.1];
        match icon {
            Icon::Key(s) => {
                Self::keycap(r, x, y, w);
                r.text_raw(x + 3, y + 1, s, ink, 1);
            }
            Icon::Arrows => {
                Self::keycap(r, x, y, 12);
                Self::keycap(r, x + 13, y, 12);
                r.arrow(x + 6, y + 5, 4, 0, ink);
                r.arrow(x + 19, y + 5, 4, 1, ink);
            }
            Icon::Pad(p) => {
                let disc = |r: &mut Raster, cx: i32, col: [f32; 3], ch: &str| {
                    r.fill_circle(cx as f32 + 5.5, y as f32 + 5.5, 5.6, [0.1, 0.1, 0.12]);
                    r.fill_circle(cx as f32 + 5.5, y as f32 + 5.5, 4.6, col);
                    r.text_raw(cx + 2, y + 2, ch, [1.0, 1.0, 1.0], 1);
                };
                match p {
                    PadIcon::A => disc(r, x, [0.3, 0.7, 0.2], "A"),
                    PadIcon::B => disc(r, x, [0.85, 0.18, 0.15], "B"),
                    PadIcon::X => disc(r, x, [0.15, 0.42, 0.9], "X"),
                    PadIcon::Y => disc(r, x, [0.9, 0.7, 0.1], "Y"),
                    PadIcon::LT | PadIcon::RT => {
                        r.fill_rect(x, y, w, ICON_H, [0.1, 0.1, 0.12], 1.0);
                        r.fill_rect(x + 1, y + 1, w - 2, ICON_H - 2, [0.32, 0.32, 0.36], 1.0);
                        r.text_raw(x + 2, y + 2, if matches!(p, PadIcon::LT) { "LT" } else { "RT" }, [1.0, 1.0, 1.0], 1);
                    }
                    PadIcon::Start => {
                        r.fill_circle(x as f32 + 7.5, y as f32 + 5.5, 5.6, [0.1, 0.1, 0.12]);
                        r.fill_circle(x as f32 + 7.5, y as f32 + 5.5, 4.6, [0.32, 0.32, 0.36]);
                        for k in 0..3 {
                            r.fill_rect(x + 5, y + 3 + k * 2, 5, 1, [1.0, 1.0, 1.0], 1.0);
                        }
                    }
                    PadIcon::LStick => {
                        r.fill_circle(x as f32 + 5.5, y as f32 + 5.5, 5.6, [0.1, 0.1, 0.12]);
                        r.fill_circle(x as f32 + 5.5, y as f32 + 5.5, 4.6, [0.4, 0.4, 0.44]);
                        r.fill_circle(x as f32 + 5.5, y as f32 + 5.5, 3.0, [0.25, 0.25, 0.28]);
                        r.text_raw(x + 2, y + 2, "L", [1.0, 1.0, 1.0], 1);
                    }
                    PadIcon::DPadH | PadIcon::DownB => {
                        r.fill_rect(x + 4, y, 3, ICON_H, [0.35, 0.35, 0.4], 1.0);
                        r.fill_rect(x, y + 4, ICON_H, 3, [0.35, 0.35, 0.4], 1.0);
                        if matches!(p, PadIcon::DPadH) {
                            r.fill_rect(x, y + 4, 3, 3, [1.0, 1.0, 1.0], 1.0);
                            r.fill_rect(x + 8, y + 4, 3, 3, [1.0, 1.0, 1.0], 1.0);
                        } else {
                            r.fill_rect(x + 4, y + 8, 3, 3, [1.0, 1.0, 1.0], 1.0);
                            r.text_raw(x + 11, y + 2, "+", [1.0, 1.0, 1.0], 1);
                            disc(r, x + 13, [0.85, 0.18, 0.15], "B");
                        }
                    }
                }
            }
        }
        w
    }

    fn prompt_width(&self, act: Act, label: &str) -> i32 {
        Self::icon_width(self.icon_for(act)) + 4 + Raster::text_width(label, 1)
    }

    /// Dessine « icône + libellé », renvoie la largeur occupée.
    fn prompt(&mut self, x: i32, y: i32, act: Act, label: &str, col: [f32; 3]) -> i32 {
        let w = self.draw_icon(x, y, self.icon_for(act));
        self.raster.text(x + w + 4, y + 2, label, col, 1);
        w + 4 + Raster::text_width(label, 1)
    }

    /// Barre d'invites centrée (ou alignée à droite si `right`).
    fn prompt_bar(&mut self, y: i32, items: &[(Act, &str)], right: bool) {
        let gap = 12;
        let total: i32 = items.iter().map(|(a, l)| self.prompt_width(*a, l)).sum::<i32>() + gap * (items.len() as i32 - 1).max(0);
        let mut x = if right { W as i32 - 8 - total } else { (W as i32 - total) / 2 };
        for (a, l) in items {
            x += self.prompt(x, y, *a, l, TEXT) + gap;
        }
    }

    // ------------------------------------------------------------ listes de menu

    fn title_items(&self) -> Vec<TitleItem> {
        let mut v = Vec::new();
        if self.save_exists {
            v.push(TitleItem::Continue);
        }
        v.push(TitleItem::New);
        v.push(TitleItem::Options);
        if self.native {
            v.push(TitleItem::Quit);
        }
        v
    }

    fn pause_items(&self) -> Vec<PauseItem> {
        let mut v = vec![PauseItem::Resume, PauseItem::Load, PauseItem::Options, PauseItem::Controls, PauseItem::MainMenu];
        if self.native {
            v.push(PauseItem::Quit);
        }
        v
    }

    pub(super) fn option_rows(&self) -> Vec<Opt> {
        let mut v = vec![Opt::Display];
        if self.native {
            v.push(Opt::Resolution);
        }
        v.extend([Opt::Scaling, Opt::Master, Opt::Sfx, Opt::Ambient, Opt::Controls, Opt::Fps, Opt::ShowControls, Opt::Back]);
        v
    }

    /// Ligne de menu inactive (résolution de fenêtre en plein écran).
    fn row_disabled(&self, i: usize) -> bool {
        matches!(self.mode, Mode::Options { .. }) && self.opts.fullscreen && self.option_rows().get(i) == Some(&Opt::Resolution)
    }

    fn menu_len(&self) -> usize {
        match self.mode {
            Mode::Title => self.title_items().len(),
            Mode::Pause => self.pause_items().len(),
            Mode::Options { .. } => self.option_rows().len(),
            _ => 0,
        }
    }

    // ------------------------------------------------------------ logique des menus

    pub(super) fn update_menu(&mut self, inp: &Input) {
        let n = self.menu_len();
        // Survol à la souris.
        let mut hovered = None;
        if let Some((mx, my)) = inp.mouse {
            let (mx, my) = (mx as i32, my as i32);
            hovered = self.menu_rows.iter().position(|&(x0, y0, x1, y1)| mx >= x0 && mx < x1 && my >= y0 && my < y1);
            let moved = self.last_mouse != inp.mouse;
            if let Some(h) = hovered {
                if moved && h != self.sel && h < n && !self.row_disabled(h) {
                    self.sel = h;
                    self.play(Sfx::Cursor, 0.25, 0.0, 1.0);
                }
            }
        }
        self.last_mouse = inp.mouse;
        if n > 0 {
            // Les lignes grisées sont sautées.
            if inp.up {
                self.sel = (self.sel + n - 1) % n;
                if self.row_disabled(self.sel) {
                    self.sel = (self.sel + n - 1) % n;
                }
                self.play(Sfx::Cursor, 0.3, 0.0, 1.0);
            }
            if inp.down {
                self.sel = (self.sel + 1) % n;
                if self.row_disabled(self.sel) {
                    self.sel = (self.sel + 1) % n;
                }
                self.play(Sfx::Cursor, 0.3, 0.0, 1.0);
            }
            self.sel = self.sel.min(n - 1);
        }
        // Un clic hors d'une ligne ne valide rien.
        let confirm = (inp.confirm || inp.interact) && (!inp.click || hovered.is_some());

        match self.mode.clone() {
            Mode::Title => {
                if confirm {
                    match self.title_items()[self.sel] {
                        TitleItem::Continue => {
                            if let Some(st) = self.storage.load("save").and_then(|s| save::decode(&s, &self.world)) {
                                self.checkpoint = st;
                                self.has_saved = true;
                                self.play(Sfx::Pickup, 0.5, 0.0, 0.7);
                                self.load_checkpoint();
                            }
                        }
                        TitleItem::New => {
                            self.play(Sfx::Pickup, 0.5, 0.0, 0.7);
                            self.new_game();
                        }
                        TitleItem::Options => self.open(Mode::Options { title: true }),
                        TitleItem::Quit => self.requests.push(Request::Quit),
                    }
                }
            }
            Mode::Pause => {
                if inp.pause || inp.cancel {
                    self.mode = Mode::Play;
                } else if confirm {
                    match self.pause_items()[self.sel] {
                        PauseItem::Resume => self.mode = Mode::Play,
                        PauseItem::Load => self.load_checkpoint(),
                        PauseItem::Options => self.open(Mode::Options { title: false }),
                        PauseItem::Controls => self.open(Mode::Controls { title: false, options: false }),
                        PauseItem::MainMenu => {
                            self.st = initial_state(&self.world);
                            self.enter_room(ROOM_HALL, START_POS, 0.0);
                            self.open(Mode::Title);
                        }
                        PauseItem::Quit => self.requests.push(Request::Quit),
                    }
                }
            }
            Mode::Options { title } => {
                let rows = self.option_rows();
                let row = rows[self.sel.min(rows.len() - 1)];
                let back = |g: &mut Game| {
                    if title {
                        g.open(Mode::Title);
                        g.sel = g.title_items().iter().position(|&i| i == TitleItem::Options).unwrap_or(0);
                    } else {
                        g.open(Mode::Pause);
                        g.sel = 2;
                    }
                };
                if inp.cancel || inp.pause {
                    back(self);
                    return;
                }
                let delta = if inp.left {
                    -1
                } else if inp.right {
                    1
                } else if confirm {
                    2 // validation : bascule / incrémente
                } else {
                    0
                };
                if delta != 0 {
                    match row {
                        Opt::ShowControls => {
                            if confirm {
                                self.open(Mode::Controls { title, options: true });
                            }
                        }
                        Opt::Back => {
                            if confirm {
                                back(self);
                            }
                        }
                        _ if self.row_disabled(self.sel) => {}
                    _ => self.change_option(row, delta),
                    }
                }
            }
            Mode::Controls { title, options } => {
                if inp.confirm || inp.cancel || inp.pause || inp.interact {
                    if options {
                        self.open(Mode::Options { title });
                        self.sel = self.option_rows().iter().position(|&o| o == Opt::ShowControls).unwrap_or(0);
                    } else if title {
                        self.open(Mode::Title);
                    } else {
                        self.open(Mode::Pause);
                        self.sel = 3;
                    }
                }
            }
            _ => {}
        }
    }

    pub(super) fn open(&mut self, m: Mode) {
        self.mode = m;
        self.sel = 0;
        self.menu_rows.clear();
        self.play(Sfx::Cursor, 0.3, 0.0, 0.8);
    }

    fn change_option(&mut self, row: Opt, delta: i32) {
        let step = |v: u8, d: i32| -> u8 {
            if d == 2 {
                if v >= 10 {
                    0
                } else {
                    v + 1
                }
            } else {
                (v as i32 + d).clamp(0, 10) as u8
            }
        };
        match row {
            Opt::Display => {
                self.opts.fullscreen = !self.opts.fullscreen;
                self.requests.push(Request::Fullscreen(self.opts.fullscreen));
                if !self.opts.fullscreen {
                    let (w, h) = self.opts.window_size();
                    self.requests.push(Request::WindowSize(w, h));
                }
            }
            Opt::Resolution => {
                let n = RESOLUTIONS.len() as i32;
                let d = if delta == 2 { 1 } else { delta };
                self.opts.resolution = ((self.opts.resolution as i32 + d).rem_euclid(n)) as usize;
                if !self.opts.fullscreen {
                    let (w, h) = self.opts.window_size();
                    self.requests.push(Request::WindowSize(w, h));
                }
            }
            Opt::Scaling => self.opts.integer_scale = !self.opts.integer_scale,
            Opt::Master => {
                self.opts.master = step(self.opts.master, delta);
                self.apply_audio();
            }
            Opt::Sfx => self.opts.sfx = step(self.opts.sfx, delta),
            Opt::Ambient => self.opts.ambient = step(self.opts.ambient, delta),
            Opt::Controls => self.opts.classic = !self.opts.classic,
            Opt::Fps => self.opts.show_fps = !self.opts.show_fps,
            Opt::ShowControls | Opt::Back => {}
        }
        // Aperçu sonore au volume choisi.
        let preview = if row == Opt::Ambient { Sfx::Frog } else { Sfx::Cursor };
        if row == Opt::Ambient {
            self.play_amb(preview, 0.6, 0.0, 1.0);
        } else {
            self.play(preview, 0.5, 0.0, 1.0);
        }
        self.save_options();
    }

    fn option_text(&self, o: Opt) -> (&'static str, String) {
        let onoff = |b: bool| if b { "OUI".to_string() } else { "NON".to_string() };
        match o {
            Opt::Display => ("AFFICHAGE", if self.opts.fullscreen { "PLEIN ÉCRAN" } else { "FENÊTRÉ" }.to_string()),
            Opt::Resolution => {
                let (w, h) = self.opts.window_size();
                let s = format!("{w} x {h}");
                ("RÉSOLUTION", s)
            }
            Opt::Scaling => ("MISE À L'ÉCHELLE", if self.opts.integer_scale { "ENTIÈRE" } else { "ÉTIRÉE 4:3" }.to_string()),
            Opt::Master => ("VOLUME GÉNÉRAL", String::new()),
            Opt::Sfx => ("VOLUME EFFETS", String::new()),
            Opt::Ambient => ("VOLUME AMBIANCE", String::new()),
            Opt::Controls => ("CONTRÔLES", if self.opts.classic { "CLASSIQUES" } else { "MODERNES" }.to_string()),
            Opt::Fps => ("AFFICHER LES FPS", onoff(self.opts.show_fps)),
            Opt::ShowControls => ("VOIR LES COMMANDES", String::new()),
            Opt::Back => ("RETOUR", String::new()),
        }
    }

    // ------------------------------------------------------------ rendu

    fn text_box(&mut self, lines: &[String]) -> i32 {
        let r = &mut self.raster;
        let h = 16 + lines.len() as i32 * 11;
        let y0 = H as i32 - h - 8;
        r.fill_rect(8, y0, W as i32 - 16, h, [0.0, 0.0, 0.02], 0.8);
        r.rect_outline(8, y0, W as i32 - 16, h, [0.45, 0.4, 0.3]);
        for (i, l) in lines.iter().enumerate() {
            r.text(16, y0 + 8 + i as i32 * 11, l, [0.92, 0.9, 0.85], 1);
        }
        y0
    }

    /// Liste centrée avec sélection ; enregistre les zones cliquables.
    fn menu_list(&mut self, items: &[String], y0: i32, spacing: i32) {
        self.menu_rows.clear();
        for (i, it) in items.iter().enumerate() {
            let y = y0 + i as i32 * spacing;
            let sel = self.sel == i;
            let w = Raster::text_width(it, 1);
            let x = (W as i32 - w) / 2;
            if sel {
                self.raster.fill_rect(x - 14, y - 3, w + 28, 13, [0.35, 0.05, 0.04], 0.75);
                self.raster.text(x - 11, y, ">", GOLD, 1);
            }
            self.raster.text(x, y, it, if sel { GOLD } else { DIM }, 1);
            self.menu_rows.push((40, y - 3, W as i32 - 40, y - 3 + spacing));
        }
    }

    pub(super) fn render_ui(&mut self) {
        let t = self.time;
        match self.mode.clone() {
            Mode::Play => {
                if self.banner_t > 0.0 && self.fade < 0.5 {
                    let name = self.room().name;
                    let k = self.banner_t.min(1.0);
                    self.raster.text(10, 10, name, [0.8 * k, 0.75 * k, 0.6 * k], 1);
                }
                self.render_boss_bar();
                let p = self.st.player.clone();
                if p.aim_t > 0.5 {
                    let inv = &self.st.inv;
                    let (loaded, spare, full) = match inv.weapon {
                        Weapon::Pistol => (inv.mag, inv.reserve, MAG_SIZE),
                        Weapon::Shotgun => (inv.shells_mag, inv.shells, SHELL_MAG),
                        Weapon::Knife => (0, 0, 0),
                    };
                    let s = match inv.weapon {
                        Weapon::Knife => "COUTEAU".to_string(),
                        _ => format!("{loaded:>2} / {spare}"),
                    };
                    let c = if inv.weapon != Weapon::Knife && loaded == 0 { [0.9, 0.2, 0.2] } else { [0.85, 0.85, 0.75] };
                    self.raster.text(W as i32 - 8 - Raster::text_width(&s, 1), H as i32 - 16, &s, c, 1);
                    let mut x = 8;
                    let verb = if inv.weapon == Weapon::Knife { "FRAPPER" } else { "TIRER" };
                    x += self.prompt(x, H as i32 - 18, Act::Fire, verb, TEXT) + 10;
                    if spare > 0 && loaded < full {
                        self.prompt(x, H as i32 - 18, Act::Reload, "RECHARGER", TEXT);
                    }
                }
                if p.reload_t > 0.0 {
                    self.raster.text(W as i32 - 104, H as i32 - 30, "RECHARGEMENT", [0.8, 0.7, 0.3], 1);
                }
                if let Some(hint) = self.interact_hint() {
                    let w = self.prompt_width(Act::Interact, hint);
                    let k = 0.75 + 0.25 * (t * 4.0).sin();
                    self.prompt((W as i32 - w) / 2, H as i32 - 20, Act::Interact, hint, [k, k, k * 0.92]);
                }
            }
            Mode::Cutscene { cine, t: ct } => {
                // Bandes noires de cinéma et nom du monstre.
                let bar = (ct / 0.4).min(1.0);
                let h = (26.0 * bar) as i32;
                self.raster.fill_rect(0, 0, W as i32, h, [0.0, 0.0, 0.0], 1.0);
                self.raster.fill_rect(0, H as i32 - h, W as i32, h, [0.0, 0.0, 0.0], 1.0);
                let (name, at) = match cine {
                    story::Cine::Brute => ("SUJET 7", 1.4),
                    story::Cine::Boss => ("L'ABOMINATION", 4.3),
                    story::Cine::Hound => ("LE MOLOSSE", 1.0),
                };
                if ct > at {
                    let k = ((ct - at) / 0.6).min(1.0);
                    self.raster.text_center(H as i32 - 52, name, [0.8 * k, 0.08 * k, 0.05 * k], 2);
                }
                if ct > 1.0 {
                    self.prompt_bar(H as i32 - 18, &[(Act::Confirm, "PASSER")], true);
                }
            }
            Mode::Ending { t: et } => self.render_ending(et),
            Mode::Message => {
                if let Some(m) = self.msgs.front().cloned() {
                    let lines = wrap(&m, 36);
                    let y0 = self.text_box(&lines);
                    let _ = y0;
                    if (t * 3.0).sin() > -0.5 {
                        let w = Self::icon_width(self.icon_for(Act::Interact));
                        self.draw_icon(W as i32 - 14 - w, H as i32 - 22, self.icon_for(Act::Interact));
                    }
                }
            }
            Mode::SaveChoice => {
                self.text_box(&["Sauvegarder votre progression ?".to_string(), String::new()]);
                let y = H as i32 - 25;
                self.menu_rows.clear();
                for (i, o) in ["OUI", "NON"].iter().enumerate() {
                    let x = 100 + i as i32 * 80;
                    let sel = self.sel == i;
                    let c = if sel { GOLD } else { DIM };
                    if sel {
                        self.raster.text(x - 12, y, ">", c, 1);
                    }
                    self.raster.text(x, y, o, c, 1);
                }
            }
            Mode::Inventory => self.render_inventory(),
            Mode::Pause => {
                self.raster.fill_rect(0, 0, W as i32, H as i32, [0.0, 0.0, 0.0], 0.65);
                self.raster.text_center(36, "PAUSE", [0.8, 0.1, 0.08], 3);
                let items: Vec<String> = self
                    .pause_items()
                    .iter()
                    .map(|i| {
                        match i {
                            PauseItem::Resume => "REPRENDRE",
                            PauseItem::Load => "CHARGER LA SAUVEGARDE",
                            PauseItem::Options => "OPTIONS",
                            PauseItem::Controls => "COMMANDES",
                            PauseItem::MainMenu => "MENU PRINCIPAL",
                            PauseItem::Quit => "QUITTER LE JEU",
                        }
                        .to_string()
                    })
                    .collect();
                self.menu_list(&items, 92, 17);
                self.prompt_bar(H as i32 - 18, &[(Act::Confirm, "VALIDER"), (Act::Back, "REPRENDRE")], false);
            }
            Mode::Options { .. } => self.render_options(),
            Mode::Controls { .. } => self.render_controls(),
            Mode::GameOver { t: gt } => {
                let k = (gt / 1.5).min(1.0);
                self.raster.text_center(90, "VOUS ÊTES MORTE", [0.75 * k, 0.05 * k, 0.03 * k], 2);
                if gt > 2.0 {
                    let s = if self.has_saved { "Reprise à la dernière sauvegarde" } else { "Reprise au début" };
                    self.raster.text_center(126, s, [0.5, 0.5, 0.5], 1);
                    if (t * 2.0).sin() > -0.3 {
                        self.prompt_bar(150, &[(Act::Confirm, "CONTINUER")], false);
                    }
                }
            }
            Mode::Title => self.render_title(),
            Mode::Door { .. } => {}
        }
    }

    fn interact_hint(&self) -> Option<&'static str> {
        let p = &self.st.player;
        if p.aiming || p.grabbed > 0.0 || p.knock > 0.0 || p.hp <= 0 {
            return None;
        }
        if self.item_in_reach().is_some() {
            return Some("RAMASSER");
        }
        if let Some(d) = self.door_in_reach() {
            return Some(match self.room().doors[d].anim {
                DoorAnim::Stairs { up: true } => "MONTER",
                DoorAnim::Stairs { up: false } => "DESCENDRE",
                _ => "OUVRIR",
            });
        }
        match self.interact_in_reach().map(|i| i.kind) {
            Some(InteractKind::Npc) => Some("PARLER"),
            Some(InteractKind::Note(_)) => Some("LIRE"),
            Some(InteractKind::Portrait(_)) => Some("EXAMINER"),
            Some(InteractKind::Typewriter) => Some("UTILISER"),
            Some(InteractKind::Boat) => Some("EMBARQUER"),
            Some(_) => Some("EXAMINER"),
            None => None,
        }
    }

    /// Barre de vie du boss affronté.
    fn render_boss_bar(&mut self) {
        let Some(i) = self.boss_active() else { return };
        let e = &self.st.enemies[i];
        let k = (e.hp.max(0) as f32 / e.max_hp as f32).clamp(0.0, 1.0);
        let name = e.name();
        let (w, x, y) = (180, (W as i32 - 180) / 2, 10);
        let r = &mut self.raster;
        r.text_center(y, name, [0.85, 0.75, 0.6], 1);
        r.fill_rect(x - 1, y + 11, w + 2, 6, [0.0, 0.0, 0.0], 0.8);
        r.rect_outline(x - 1, y + 11, w + 2, 6, [0.45, 0.4, 0.3]);
        r.fill_rect(x, y + 12, (w as f32 * k) as i32, 4, [0.65, 0.06, 0.04], 1.0);
        if e.weak_point_open() && (self.time * 8.0).sin() > 0.0 {
            r.text_center(y + 21, "LE COEUR EST À DÉCOUVERT !", [1.0, 0.35, 0.2], 1);
        }
    }

    fn render_ending(&mut self, et: f32) {
        let r = &mut self.raster;
        r.fill_rect(0, 0, W as i32, H as i32, [0.0, 0.0, 0.0], (et / 6.0).min(0.55));
        let mut y = 24;
        for (i, line) in story::ENDING_TEXT.iter().enumerate() {
            let at = 1.5 + i as f32 * 1.6;
            let k = ((et - at) / 1.0).clamp(0.0, 1.0);
            for l in wrap(line, 36) {
                if k > 0.0 {
                    r.text_center(y, &l, [0.85 * k, 0.82 * k, 0.75 * k], 1);
                }
                y += 10;
            }
            y += 4;
        }
        if et > 11.0 {
            let k = ((et - 11.0) / 1.5).min(1.0);
            r.text_center(134, "FIN", [0.75 * k, 0.06 * k, 0.04 * k], 3);
        }
        if et > 13.0 {
            let st = &self.st;
            let secs = st.play_time as u32;
            let rank = if secs < 40 * 60 && st.saves <= 3 {
                "A"
            } else if secs < 70 * 60 && st.saves <= 8 {
                "B"
            } else {
                "C"
            };
            let lines = [
                format!("TEMPS DE JEU    {:02}:{:02}:{:02}", secs / 3600, secs / 60 % 60, secs % 60),
                format!("SAUVEGARDES     {}", st.saves),
                format!("ENNEMIS ABATTUS {}", st.kills),
                format!("RANG            {rank}"),
            ];
            for (i, l) in lines.iter().enumerate() {
                self.raster.text_center(168 + i as i32 * 11, l, GOLD, 1);
            }
        }
        if et > story::ENDING_LEN {
            self.prompt_bar(H as i32 - 16, &[(Act::Confirm, "MENU PRINCIPAL")], false);
        }
    }

    fn render_title(&mut self) {
        let t = self.time;
        let r = &mut self.raster;
        r.fill_rect(0, 0, W as i32, H as i32, [0.0, 0.0, 0.0], 0.45);
        let flick = if (t * 7.3).sin() > 0.97 { 0.6 } else { 1.0 };
        r.text_center(30, "LE MANOIR", [0.7 * flick, 0.05, 0.03], 4);
        r.text_center(68, "DES MARAIS", [0.55 * flick, 0.04, 0.03], 3);
        r.text_center(98, "~ survival horror ~", [0.5, 0.48, 0.42], 1);
        let items: Vec<String> = self
            .title_items()
            .iter()
            .map(|i| {
                match i {
                    TitleItem::Continue => "CONTINUER",
                    TitleItem::New => "NOUVELLE PARTIE",
                    TitleItem::Options => "OPTIONS",
                    TitleItem::Quit => "QUITTER",
                }
                .to_string()
            })
            .collect();
        self.menu_list(&items, 130, 17);
        self.prompt_bar(H as i32 - 18, &[(Act::Confirm, "VALIDER")], false);
    }

    fn render_options(&mut self) {
        let rows = self.option_rows();
        self.raster.fill_rect(0, 0, W as i32, H as i32, [0.0, 0.0, 0.02], 0.8);
        self.raster.text_center(12, "OPTIONS", [0.8, 0.1, 0.08], 2);
        self.menu_rows.clear();
        let y0 = 40;
        let sp = 16;
        for (i, &o) in rows.iter().enumerate() {
            let y = y0 + i as i32 * sp;
            let sel = self.sel == i;
            if sel {
                self.raster.fill_rect(10, y - 3, W as i32 - 20, 13, [0.35, 0.05, 0.04], 0.75);
            }
            let (name, val) = self.option_text(o);
            let disabled = self.row_disabled(i);
            let c = if disabled {
                [0.26, 0.26, 0.26]
            } else if sel {
                GOLD
            } else {
                DIM
            };
            self.raster.text(18, y, name, c, 1);
            let vol = match o {
                Opt::Master => Some(self.opts.master),
                Opt::Sfx => Some(self.opts.sfx),
                Opt::Ambient => Some(self.opts.ambient),
                _ => None,
            };
            let vx = 176;
            if let Some(v) = vol {
                for k in 0..10 {
                    let on = k < v as i32;
                    let col = if on { if sel { GOLD } else { [0.7, 0.6, 0.35] } } else { [0.2, 0.2, 0.22] };
                    self.raster.fill_rect(vx + k * 11, y, 9, 8, col, 1.0);
                }
            } else if !val.is_empty() {
                let w = Raster::text_width(&val, 1);
                let x = vx + (110 - w) / 2;
                let vc = if disabled {
                    [0.26, 0.26, 0.26]
                } else if sel {
                    [1.0, 1.0, 1.0]
                } else {
                    TEXT
                };
                self.raster.text(x, y, &val, vc, 1);
                if sel {
                    self.raster.arrow(vx - 6, y + 3, 4, 0, GOLD);
                    self.raster.arrow(vx + 116, y + 3, 4, 1, GOLD);
                }
            }
            self.menu_rows.push((10, y - 3, W as i32 - 10, y - 3 + sp));
        }
        let hint = match rows.get(self.sel) {
            Some(Opt::Display) => "Bascule plein écran / fenêtre.",
            Some(Opt::Resolution) => "Taille de la fenêtre en mode fenêtré.",
            Some(Opt::Scaling) => "Entière : pixels nets, bandes noires. Étirée : remplit l'écran.",
            Some(Opt::Controls) => {
                if self.opts.classic {
                    "Classiques : gauche/droite pivotent, haut avance, bas recule."
                } else {
                    "Modernes : la direction du stick est relative à la caméra."
                }
            }
            Some(Opt::Fps) => "Affiche le nombre d'images par seconde.",
            _ => "",
        };
        for (i, l) in wrap(hint, 38).iter().enumerate() {
            self.raster.text(10, 42 + rows.len() as i32 * sp + 2 + i as i32 * 10, l, [0.55, 0.55, 0.5], 1);
        }
        self.prompt_bar(H as i32 - 18, &[(Act::Change, "MODIFIER"), (Act::Back, "RETOUR")], false);
    }

    fn render_controls(&mut self) {
        self.raster.fill_rect(0, 0, W as i32, H as i32, [0.0, 0.0, 0.0], 0.85);
        self.raster.text_center(10, "COMMANDES", [0.8, 0.1, 0.08], 2);
        let dev = if self.pad { "MANETTE" } else { "CLAVIER / SOURIS" };
        let scheme = if self.opts.classic { "CLASSIQUES" } else { "MODERNES" };
        self.raster.text_center(32, &format!("{dev} - CONTRÔLES {scheme}"), GOLD, 1);
        let mut rows: Vec<(Act, &str)> = Vec::new();
        if self.opts.classic {
            rows.push((Act::Move, "AVANCER / RECULER"));
            rows.push((Act::Turn, "PIVOTER"));
            rows.push((Act::QuickTurn, "DEMI-TOUR"));
        } else {
            rows.push((Act::Move, "SE DÉPLACER"));
        }
        rows.extend([
            (Act::Run, "COURIR"),
            (Act::Aim, "VISER (MAINTENIR)"),
            (Act::Fire, "TIRER EN VISANT"),
            (Act::Reload, "RECHARGER"),
            (Act::Interact, "EXAMINER / OUVRIR"),
            (Act::Inventory, "INVENTAIRE"),
            (Act::Pause, "PAUSE"),
        ]);
        let y0 = 50;
        for (i, (a, l)) in rows.iter().enumerate() {
            let y = y0 + i as i32 * 15;
            let icon = self.icon_for(*a);
            let w = Self::icon_width(icon);
            self.draw_icon(100 - w, y, icon);
            self.raster.text(108, y + 2, l, TEXT, 1);
        }
        let note = if self.pad {
            if self.opts.classic {
                "En visant, gauche/droite font pivoter."
            } else {
                "Déplacement relatif à la caméra. En visant, le stick oriente le tir."
            }
        } else {
            "En visant à la souris, l'héroïne se tourne vers le curseur. ZQSD sur clavier AZERTY."
        };
        for (i, l) in wrap(note, 38).iter().enumerate() {
            self.raster.text(10, 196 + i as i32 * 10, l, [0.55, 0.55, 0.5], 1);
        }
        self.prompt_bar(H as i32 - 14, &[(Act::Back, "RETOUR")], false);
    }

    fn render_inventory(&mut self) {
        let slots = self.inv_slots();
        let hp = self.st.player.hp;
        let t = self.time;
        let r = &mut self.raster;
        r.fill_rect(0, 0, W as i32, H as i32, [0.0, 0.01, 0.03], 0.85);
        r.text(16, 14, "INVENTAIRE", [0.8, 0.75, 0.6], 2);
        // Électrocardiogramme.
        let (label, col, speed) = if hp > 66 {
            ("BIEN", [0.2, 0.9, 0.3], 1.0)
        } else if hp > 33 {
            ("ATTENTION", [0.95, 0.75, 0.15], 1.5)
        } else {
            ("DANGER", [0.95, 0.15, 0.1], 2.2)
        };
        let (bx, by, bw, bh) = (16, 46, 130, 52);
        r.fill_rect(bx, by, bw, bh, [0.0, 0.0, 0.0], 0.9);
        r.rect_outline(bx, by, bw, bh, [0.3, 0.3, 0.35]);
        let mut prev: Option<i32> = None;
        for x in 0..bw - 4 {
            let u = (x as f32 / 40.0 - t * speed).rem_euclid(1.0);
            let beat = if (0.3..0.34).contains(&u) {
                -1.0
            } else if (0.34..0.38).contains(&u) {
                1.0
            } else if (0.38..0.4).contains(&u) {
                -0.3
            } else if (0.5..0.6).contains(&u) {
                -0.15 * ((u - 0.5) / 0.1 * PI).sin()
            } else {
                0.0
            };
            let y = by + bh / 2 + (beat * 18.0) as i32;
            let head = ((t * speed * 40.0) as i32).rem_euclid(bw - 4);
            let age = (head - x).rem_euclid(bw - 4) as f32 / (bw - 4) as f32;
            let k = (1.0 - age).powf(1.5);
            let c = [col[0] * k, col[1] * k, col[2] * k];
            let (y0, y1) = match prev {
                Some(py) => (py.min(y), py.max(y)),
                None => (y, y),
            };
            r.fill_rect(bx + 2 + x, y0, 1, y1 - y0 + 1, c, 1.0);
            prev = Some(y);
        }
        r.text(bx, by + bh + 6, label, col, 2);

        let lx = 154;
        self.menu_rows.clear();
        // Liste resserrée quand l'inventaire se remplit.
        let sp = match slots.len() {
            0..=5 => 22,
            6..=7 => 17,
            _ => 14,
        };
        for (i, (_, name)) in slots.iter().enumerate() {
            let y = 48 + i as i32 * sp;
            let sel = self.sel == i;
            let r = &mut self.raster;
            let bh = sp - 3;
            r.fill_rect(lx, y - 4, 158, bh, if sel { [0.25, 0.2, 0.1] } else { [0.08, 0.08, 0.1] }, 0.9);
            r.rect_outline(lx, y - 4, 158, bh, if sel { [0.9, 0.75, 0.3] } else { [0.3, 0.3, 0.35] });
            r.text(lx + 6, y + (bh - 8) / 2 - 4, name, if sel { [1.0, 0.95, 0.8] } else { [0.7, 0.7, 0.7] }, 1);
            self.menu_rows.push((lx, y - 4, lx + 158, y - 4 + bh));
        }
        let equipped = self.st.inv.weapon;
        let (desc, action) = match slots.get(self.sel).map(|s| s.0) {
            Some(Slot::Weapon(w)) => {
                let desc = match w {
                    Weapon::Pistol => "Pistolet semi-automatique 9 mm. Chargeur de 15 balles.",
                    Weapon::Shotgun => "Fusil à pompe. 6 cartouches. Dévastateur à bout portant, inutile de loin.",
                    Weapon::Knife => "Couteau de combat. Silencieux, inusable, mais il faut s'approcher...",
                };
                (desc, if w == equipped { if w == Weapon::Knife { "" } else { "RECHARGER" } } else { "ÉQUIPER" })
            }
            Some(Slot::Medkit) => ("Trousse de premiers secours. Restaure toute la santé.", "UTILISER"),
            Some(Slot::Herb) => ("Herbe médicinale. Soigne légèrement les blessures.", "UTILISER"),
            Some(Slot::Key(k)) => (key_desc(k), "EXAMINER"),
            None => ("", ""),
        };
        for (i, l) in wrap(desc, 38).iter().enumerate() {
            self.raster.text(10, 180 + i as i32 * 10, l, [0.75, 0.75, 0.7], 1);
        }
        if action.is_empty() {
            self.prompt_bar(H as i32 - 18, &[(Act::Back, "FERMER")], false);
        } else {
            self.prompt_bar(H as i32 - 18, &[(Act::Confirm, action), (Act::Back, "FERMER")], false);
        }
    }
}
