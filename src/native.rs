//! Frontend natif : fenêtre winit, framebuffer softbuffer, son cpal.

use psx_resident::audio::{Mixer, SharedMixer};
use psx_resident::game::Game;
use psx_resident::input::{Input, InputCollector, Key};
use psx_resident::math::v3;
use psx_resident::options::{MemStorage, Request, Storage};
use std::path::PathBuf;
use psx_resident::raster::{H, W};
use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Fullscreen, Window, WindowId};

const FRAME: Duration = Duration::from_micros(16_667);

fn map_key(code: KeyCode) -> Option<Key> {
    Some(match code {
        KeyCode::KeyW => Key::W,
        KeyCode::KeyA => Key::A,
        KeyCode::KeyS => Key::S,
        KeyCode::KeyD => Key::D,
        KeyCode::ArrowUp => Key::Up,
        KeyCode::ArrowDown => Key::Down,
        KeyCode::ArrowLeft => Key::Left,
        KeyCode::ArrowRight => Key::Right,
        KeyCode::ShiftLeft | KeyCode::ShiftRight => Key::Shift,
        KeyCode::ControlLeft | KeyCode::ControlRight => Key::Ctrl,
        KeyCode::Space => Key::Space,
        KeyCode::KeyE => Key::E,
        KeyCode::Enter | KeyCode::NumpadEnter => Key::Enter,
        KeyCode::KeyR => Key::R,
        KeyCode::Tab => Key::Tab,
        KeyCode::KeyI => Key::I,
        KeyCode::Escape => Key::Esc,
        KeyCode::KeyP => Key::P,
        KeyCode::KeyF => Key::F,
        _ => return None,
    })
}

fn start_audio(mixer: &SharedMixer) -> Option<cpal::Stream> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    let host = cpal::default_host();
    let dev = host.default_output_device()?;
    let cfg = dev.default_output_config().ok()?;
    let fmt = cfg.sample_format();
    let config: cpal::StreamConfig = cfg.into();
    let ch = config.channels as usize;
    {
        let mut m = mixer.lock().ok()?;
        m.set_rate(config.sample_rate as f32);
        m.enabled = true;
    }
    let stream = match fmt {
        cpal::SampleFormat::F32 => {
            let m = mixer.clone();
            dev.build_output_stream(
                &config,
                move |d: &mut [f32], _| match m.lock() {
                    Ok(mut m) => m.render(d, ch),
                    Err(_) => d.fill(0.0),
                },
                |e| eprintln!("audio : {e}"),
                None,
            )
            .ok()?
        }
        cpal::SampleFormat::I16 => {
            let m = mixer.clone();
            let mut tmp: Vec<f32> = Vec::new();
            dev.build_output_stream(
                &config,
                move |d: &mut [i16], _| {
                    tmp.resize(d.len(), 0.0);
                    if let Ok(mut m) = m.lock() {
                        m.render(&mut tmp, ch);
                    }
                    for (o, s) in d.iter_mut().zip(&tmp) {
                        *o = (s * 32767.0) as i16;
                    }
                },
                |e| eprintln!("audio : {e}"),
                None,
            )
            .ok()?
        }
        other => {
            eprintln!("audio : format {other:?} non pris en charge");
            return None;
        }
    };
    stream.play().ok()?;
    Some(stream)
}

/// Stockage dans le dossier de configuration de l'utilisateur.
struct FileStorage {
    dir: PathBuf,
}

impl FileStorage {
    fn new() -> FileStorage {
        let base = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from))
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .unwrap_or_else(|| PathBuf::from("."));
        FileStorage { dir: base.join("manoir-des-marais") }
    }
}

impl Storage for FileStorage {
    fn load(&self, key: &str) -> Option<String> {
        std::fs::read_to_string(self.dir.join(format!("{key}.txt"))).ok()
    }
    fn save(&mut self, key: &str, data: &str) {
        let r = std::fs::create_dir_all(&self.dir).and_then(|_| std::fs::write(self.dir.join(format!("{key}.txt")), data));
        if let Err(e) = r {
            eprintln!("sauvegarde impossible ({}) : {e}", self.dir.display());
        }
    }
}

fn mem_game() -> Game {
    Game::new(Mixer::shared(), Box::new(MemStorage::default()), true)
}

struct App {
    window: Option<Rc<Window>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    game: Game,
    input: InputCollector,
    last: Instant,
    next_frame: Instant,
    _stream: Option<cpal::Stream>,
    /// Rectangle de l'image dans la fenêtre (x, y, largeur, hauteur).
    view: (u32, u32, u32, u32),
    quit: bool,
}

impl App {
    fn redraw(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f32();
        self.last = now;
        let inp = self.input.poll();
        self.game.frame(dt, &inp);
        for req in self.game.take_requests() {
            match req {
                Request::Quit => self.quit = true,
                Request::Fullscreen(on) => {
                    if let Some(w) = &self.window {
                        w.set_fullscreen(if on { Some(Fullscreen::Borderless(None)) } else { None });
                    }
                }
                Request::WindowSize(ww, hh) => {
                    if let Some(w) = &self.window {
                        if w.fullscreen().is_none() {
                            let _ = w.request_inner_size(winit::dpi::PhysicalSize::new(ww, hh));
                        }
                    }
                }
            }
        }

        let (Some(window), Some(surface)) = (self.window.as_ref(), self.surface.as_mut()) else { return };
        let size = window.inner_size();
        let (w, h) = (size.width.max(1), size.height.max(1));
        if surface.resize(NonZeroU32::new(w).unwrap(), NonZeroU32::new(h).unwrap()).is_err() {
            return;
        }
        let (ox, oy, vw, vh) = self.game.options().viewport(w, h);
        self.view = (ox, oy, vw, vh);
        let Ok(mut buf) = surface.buffer_mut() else { return };
        buf.fill(0);
        let fb = &self.game.raster.fb;
        let (w, h) = (w as usize, h as usize);
        let (ox, oy, vw, vh) = (ox as usize, oy as usize, vw as usize, vh as usize);
        // Échantillonnage au plus proche (pixels nets).
        let xmap: Vec<usize> = (0..vw).map(|x| x * W / vw).collect();
        let mut row = vec![0u32; vw];
        let mut last_src = usize::MAX;
        for y in 0..vh {
            let dy = oy + y;
            if dy >= h {
                break;
            }
            let sy = y * H / vh;
            if sy != last_src {
                let src = &fb[sy * W..sy * W + W];
                for (d, &sx) in row.iter_mut().zip(&xmap) {
                    *d = src[sx];
                }
                last_src = sy;
            }
            let cw = vw.min(w.saturating_sub(ox));
            buf[dy * w + ox..dy * w + ox + cw].copy_from_slice(&row[..cw]);
        }
        let _ = buf.present();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let opts = self.game.options().clone();
        let (ww, hh) = opts.window_size();
        let attrs = Window::default_attributes()
            .with_title("Le Manoir des Marais")
            .with_inner_size(winit::dpi::PhysicalSize::new(ww, hh))
            .with_fullscreen(if opts.fullscreen { Some(Fullscreen::Borderless(None)) } else { None });
        let window = Rc::new(el.create_window(attrs).expect("création de la fenêtre"));
        let ctx = softbuffer::Context::new(window.clone()).expect("contexte softbuffer");
        let surface = softbuffer::Surface::new(&ctx, window.clone()).expect("surface softbuffer");
        self.window = Some(window);
        self.surface = Some(surface);
        self.last = Instant::now();
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    if let Some(k) = map_key(code) {
                        let down = event.state == ElementState::Pressed;
                        if !(down && event.repeat) {
                            self.input.key(k, down);
                        }
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let (ox, oy, vw, vh) = self.view;
                let x = (position.x as f32 - ox as f32) * W as f32 / vw.max(1) as f32;
                let y = (position.y as f32 - oy as f32) * H as f32 / vh.max(1) as f32;
                self.input.mouse_move(x, y);
            }
            WindowEvent::CursorLeft { .. } => self.input.mouse_leave(),
            WindowEvent::MouseInput { state, button, .. } => {
                let b = match button {
                    MouseButton::Left => 0,
                    MouseButton::Right => 2,
                    _ => 1,
                };
                self.input.mouse_button(b, state == ElementState::Pressed);
            }
            WindowEvent::Focused(false) => self.input.reset(),
            WindowEvent::RedrawRequested => self.redraw(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        if self.quit {
            el.exit();
            return;
        }
        let now = Instant::now();
        if now >= self.next_frame {
            if let Some(w) = &self.window {
                w.request_redraw();
            }
            self.next_frame = (self.next_frame + FRAME).max(now);
        }
        el.set_control_flow(ControlFlow::WaitUntil(self.next_frame));
    }
}

fn write_ppm(path: &str, fb: &[u32]) {
    let mut out = format!("P6\n{W} {H}\n255\n").into_bytes();
    for &c in fb {
        out.extend_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, c as u8]);
    }
    std::fs::write(path, out).expect("écriture de la capture");
}

/// Captures d'écran automatiques (développement) : `--shots <dossier>`.
fn screenshots(dir: &str) {
    std::fs::create_dir_all(dir).unwrap();
    let mut g = mem_game();
    let none = Input::default();
    g.frame(1.0 / 60.0, &none);
    write_ppm(&format!("{dir}/00_title.ppm"), &g.raster.fb);
    let shots: &[(&str, usize, (f32, f32), f32, bool)] = &[
        ("01_hall_start", 0, (0.0, 2.4), 0.0, false),
        ("02_hall_north", 0, (0.5, 10.5), 0.5, false),
        ("03_dining_east", 1, (-7.5, 6.2), -1.57, false),
        ("04_dining_west", 1, (-13.0, 4.5), -2.5, true),
        ("05_corridor_s", 2, (7.5, 5.0), 0.0, true),
        ("06_corridor_n", 2, (7.5, 17.0), std::f32::consts::PI, false),
        ("07_study_a", 3, (11.0, 23.0), 0.4, false),
        ("08_study_b", 3, (7.5, 21.5), 0.0, false),
        ("09_library_s", 4, (0.0, 15.5), 0.0, false),
        ("10_library_n", 4, (0.0, 22.5), 0.0, false),
    ];
    for (name, room, (x, z), yaw, aim) in shots {
        g.debug_setup(*room, v3(*x, 0.0, *z), *yaw, *aim);
        for _ in 0..3 {
            g.frame(1.0 / 60.0, &none);
        }
        if *aim {
            g.debug_shoot();
            g.frame(0.0, &none);
        }
        write_ppm(&format!("{dir}/{name}.ppm"), &g.raster.fb);
    }
    g.debug_door(1.2);
    g.frame(0.0, &none);
    write_ppm(&format!("{dir}/11_door.ppm"), &g.raster.fb);
    g.debug_setup(0, v3(0.0, 0.0, 2.4), 0.0, false);
    g.debug_inventory();
    g.frame(1.0 / 60.0, &none);
    write_ppm(&format!("{dir}/12_inventory.ppm"), &g.raster.fb);
    g.debug_message("Vous avez trouvé : MUNITIONS 9MM (15). La brume rampe sur l'eau noire.");
    g.frame(1.0 / 60.0, &none);
    write_ppm(&format!("{dir}/13_message.ppm"), &g.raster.fb);

    // Menus, au clavier puis à la manette.
    for (pad, tag) in [(false, "kb"), (true, "pad")] {
        let inp = Input { using_pad: pad, ..Default::default() };
        for m in ["title", "pause", "options", "controls"] {
            g.debug_mode(m);
            g.frame(1.0 / 60.0, &inp);
            write_ppm(&format!("{dir}/20_{m}_{tag}.ppm"), &g.raster.fb);
        }
        g.debug_setup(0, v3(0.0, 0.0, 0.9), std::f32::consts::PI, false);
        g.frame(1.0 / 60.0, &inp);
        write_ppm(&format!("{dir}/21_hint_{tag}.ppm"), &g.raster.fb);
        g.debug_inventory();
        g.frame(1.0 / 60.0, &inp);
        write_ppm(&format!("{dir}/22_inventory_{tag}.ppm"), &g.raster.fb);
    }

    // Mesure de performance.
    g.debug_setup(0, v3(0.0, 0.0, 2.4), 0.0, false);
    let t0 = Instant::now();
    let n = 200;
    for _ in 0..n {
        g.frame(1.0 / 60.0, &none);
    }
    println!("rendu : {:.2} ms/image", t0.elapsed().as_secs_f64() * 1000.0 / n as f64);
}

/// Planche du modèle de l'héroïne (développement) : `--model <dossier>`.
fn model_sheet(dir: &str) {
    use psx_resident::actors::{self, Pose};
    use psx_resident::math::Vec3;
    use psx_resident::raster::{Camera, Fog, Light, Lighting, Raster};
    std::fs::create_dir_all(dir).unwrap();
    let mut r = Raster::new(psx_resident::textures::build());
    let hero = actors::heroine();
    let zombie = actors::zombie(0);
    let light = Lighting {
        ambient: [0.3, 0.27, 0.26],
        lights: vec![Light { pos: v3(1.2, 2.4, 1.8), color: [1.3, 1.0, 0.7], radius: 6.0 }],
        sun: None,
    };
    let views: &[(&str, f32, f32, Pose)] = &[
        ("front", 0.0, 2.2, Pose::idle(0.0)),
        ("three_quarter", 0.7, 2.0, Pose::idle(0.0)),
        ("side", 1.57, 2.0, Pose::walk(0.8, 0.0)),
        ("back", 3.14, 2.0, Pose::idle(0.0)),
        ("aim", 0.9, 2.0, Pose::aim(0.0, 0.0)),
        ("run", 2.2, 2.0, Pose::walk(1.3, 1.0)),
        ("face", 0.25, 0.9, Pose::idle(0.0)),
        ("face_front", 0.0, 0.7, Pose::idle(0.0)),
        ("face_side", 1.2, 0.7, Pose::idle(0.0)),
        ("face_low", -0.5, 0.7, Pose::idle(2.0)),
    ];
    for (name, ang, dist, pose) in views {
        r.clear(0x101014);
        let target = if name.starts_with("face") { v3(0.0, 1.62, 0.0) } else { v3(0.0, 0.95, 0.0) };
        r.cam = Camera::look_at(target + v3(ang.sin() * dist, 0.15, ang.cos() * dist), target, 60.0);
        r.fog = Fog { start: 20.0, end: 40.0, color: [0.0; 3] };
        actors::draw_body(&mut r, &hero, pose, Vec3::ZERO, 0.0, &light);
        if *name == "front" {
            actors::draw_body(&mut r, &zombie, &Pose::shamble(0.0, 0.0), v3(1.1, 0.0, -0.6), 0.0, &light);
        }
        write_ppm(&format!("{dir}/{name}.ppm"), &r.fb);
    }
    // Tête à distance de jeu, tout autour.
    for k in 0..8 {
        let ang = k as f32 * std::f32::consts::FRAC_PI_4;
        r.clear(0x101014);
        let target = v3(0.0, 1.5, 0.0);
        r.cam = Camera::look_at(target + v3(ang.sin() * 3.0, 1.0, ang.cos() * 3.0), target, 60.0);
        actors::draw_body(&mut r, &hero, &Pose::idle(0.0), Vec3::ZERO, 0.0, &light);
        write_ppm(&format!("{dir}/ring{k}.ppm"), &r.fb);
    }
}

pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--shot") {
        // --shot <pièce> <x> <z> <fichier.ppm>
        let f = |k: usize| args[i + k].parse::<f32>().unwrap();
        let mut g = mem_game();
        g.debug_setup(f(1) as usize, v3(f(2), 0.0, f(3)), 0.0, false);
        g.frame(1.0 / 60.0, &Input::default());
        write_ppm(&args[i + 4], &g.raster.fb);
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--model") {
        // --model <dossier> : héroïne en gros plan sous plusieurs angles.
        model_sheet(&args[i + 1]);
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--door") {
        // --door <t> <fichier.ppm>
        let mut g = mem_game();
        g.debug_setup(0, v3(0.0, 0.0, 2.4), 0.0, false);
        g.debug_door(args[i + 1].parse().unwrap());
        g.frame(0.0, &Input::default());
        write_ppm(&args[i + 2], &g.raster.fb);
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--film") {
        // --film <dossier> : début de partie réel, une image toutes les 0,5 s.
        let dir = &args[i + 1];
        std::fs::create_dir_all(dir).unwrap();
        let mut g = mem_game();
        let press = Input { confirm: true, interact: true, ..Default::default() };
        g.frame(1.0 / 60.0, &press);
        g.frame(1.0 / 60.0, &press);
        for k in 0..40 {
            for _ in 0..30 {
                g.frame(1.0 / 60.0, &Input::default());
            }
            write_ppm(&format!("{dir}/f{k:02}.ppm"), &g.raster.fb);
        }
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--probe") {
        // --probe <x> <y> : position monde du pixel dans le plan de départ.
        let mut g = mem_game();
        let press = Input { confirm: true, interact: true, ..Default::default() };
        g.frame(1.0 / 60.0, &press);
        g.frame(1.0 / 60.0, &press);
        if args.iter().any(|a| a == "--hide") {
            g.debug_hide_player(true);
            g.frame(1.0 / 60.0, &Input::default());
        }
        let (px, py): (usize, usize) = (args[i + 1].parse().unwrap(), args[i + 2].parse().unwrap());
        for dy in 0..3 {
            for dx in 0..6 {
                let (x, y) = (px + dx, py + dy);
                let d = g.raster.depth[y * W + x];
                let cam = g.raster.cam;
                let ray = cam.ray(x as f32 + 0.5, y as f32 + 0.5);
                let z = 1.0 / d;
                let p = cam.eye + ray * (z / ray.dot(cam.fwd));
                println!("({x},{y}) couleur {:06x} -> ({:.2}, {:.2}, {:.2})", g.raster.fb[y * W + x], p.x, p.y, p.z);
            }
        }
        return;
    }
    if args.iter().any(|a| a == "--visibility") {
        let mut g = mem_game();
        for line in g.debug_visibility() {
            println!("{line}");
        }
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--shots") {
        screenshots(args.get(i + 1).map(String::as_str).unwrap_or("shots"));
        return;
    }
    let mixer = Mixer::shared();
    let stream = start_audio(&mixer);
    if stream.is_none() {
        eprintln!("audio indisponible, le jeu continue sans son");
    }
    let event_loop = EventLoop::new().expect("boucle d'événements");
    let mut app = App {
        window: None,
        surface: None,
        game: Game::new(mixer, Box::new(FileStorage::new()), true),
        input: InputCollector::new(),
        last: Instant::now(),
        next_frame: Instant::now(),
        _stream: stream,
        view: (0, 0, W as u32 * 3, H as u32 * 3),
        quit: false,
    };
    event_loop.run_app(&mut app).expect("exécution");
}
