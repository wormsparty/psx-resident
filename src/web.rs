//! Frontend WebAssembly : canvas 2D 320x240 (mis à l'échelle en CSS),
//! événements DOM pour clavier/souris, gilrs pour la manette, cpal (WebAudio).

use crate::audio::{Mixer, SharedMixer};
use crate::game::Game;
use crate::input::{InputCollector, Key};
use crate::options::{Request, Storage};
use crate::raster::{H, W};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::{Clamped, JsCast};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData, KeyboardEvent, MouseEvent};

fn map_code(code: &str) -> Option<Key> {
    Some(match code {
        "KeyW" => Key::W,
        "KeyA" => Key::A,
        "KeyS" => Key::S,
        "KeyD" => Key::D,
        "ArrowUp" => Key::Up,
        "ArrowDown" => Key::Down,
        "ArrowLeft" => Key::Left,
        "ArrowRight" => Key::Right,
        "ShiftLeft" | "ShiftRight" => Key::Shift,
        "ControlLeft" | "ControlRight" => Key::Ctrl,
        "Space" => Key::Space,
        "KeyE" => Key::E,
        "Enter" | "NumpadEnter" => Key::Enter,
        "KeyR" => Key::R,
        "Tab" => Key::Tab,
        "KeyI" => Key::I,
        "Escape" => Key::Esc,
        "KeyP" => Key::P,
        "KeyF" => Key::F,
        _ => return None,
    })
}

/// Stockage persistant dans le localStorage du navigateur.
struct LocalStorage;

impl Storage for LocalStorage {
    fn load(&self, key: &str) -> Option<String> {
        let ls = web_sys::window()?.local_storage().ok()??;
        ls.get_item(&format!("manoir-des-marais:{key}")).ok()?
    }
    fn save(&mut self, key: &str, data: &str) {
        if let Some(Ok(Some(ls))) = web_sys::window().map(|w| w.local_storage()) {
            let _ = ls.set_item(&format!("manoir-des-marais:{key}"), data);
        }
    }
}

fn is_fullscreen() -> bool {
    web_sys::window().and_then(|w| w.document()).and_then(|d| d.fullscreen_element()).is_some()
}

fn set_fullscreen(on: bool) {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return };
    if on && !is_fullscreen() {
        if let Some(el) = doc.document_element() {
            let _ = el.request_fullscreen();
        }
    } else if !on && is_fullscreen() {
        doc.exit_fullscreen();
    }
}

struct App {
    game: Game,
    input: InputCollector,
    ctx: CanvasRenderingContext2d,
    canvas: HtmlCanvasElement,
    rgba: Vec<u8>,
    last: Option<f64>,
    mixer: SharedMixer,
    stream: Option<cpal::Stream>,
    audio_tried: bool,
    /// Taille CSS appliquée au canvas (pour ne la changer qu'au besoin).
    css: (u32, u32, u32, u32),
}

impl App {
    /// Le navigateur n'autorise le son qu'après un geste de l'utilisateur.
    fn ensure_audio(&mut self) {
        if self.audio_tried {
            return;
        }
        self.audio_tried = true;
        // Premier geste : on peut aussi passer en plein écran si demandé.
        if self.game.options().fullscreen {
            set_fullscreen(true);
        }
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
        let host = cpal::default_host();
        let Some(dev) = host.default_output_device() else { return };
        let Ok(cfg) = dev.default_output_config() else { return };
        let config: cpal::StreamConfig = cfg.into();
        let ch = config.channels as usize;
        if let Ok(mut m) = self.mixer.lock() {
            m.set_rate(config.sample_rate as f32);
            m.enabled = true;
        }
        let m = self.mixer.clone();
        let stream = dev.build_output_stream(
            &config,
            move |d: &mut [f32], _| match m.try_lock() {
                Ok(mut m) => m.render(d, ch),
                Err(_) => d.fill(0.0),
            },
            |e| web_sys::console::log_1(&format!("audio : {e}").into()),
            None,
        );
        match stream {
            Ok(s) => {
                let _ = s.play();
                self.stream = Some(s);
            }
            Err(e) => web_sys::console::log_1(&format!("audio indisponible : {e}").into()),
        }
    }

    fn tick(&mut self, ts: f64) {
        let dt = match self.last {
            Some(l) => ((ts - l) / 1000.0) as f32,
            None => 1.0 / 60.0,
        };
        self.last = Some(ts);
        let inp = self.input.poll();
        if inp.any {
            self.ensure_audio();
        }
        self.game.frame(dt, &inp);
        for req in self.game.take_requests() {
            if let Request::Fullscreen(on) = req {
                set_fullscreen(on);
            }
        }
        self.layout();
        for (i, &c) in self.game.raster.fb.iter().enumerate() {
            let o = i * 4;
            self.rgba[o] = (c >> 16) as u8;
            self.rgba[o + 1] = (c >> 8) as u8;
            self.rgba[o + 2] = c as u8;
            self.rgba[o + 3] = 255;
        }
        if let Ok(img) = ImageData::new_with_u8_clamped_array_and_sh(Clamped(&self.rgba), W as u32, H as u32) {
            let _ = self.ctx.put_image_data(&img, 0.0, 0.0);
        }
    }

    /// Dimensionne le canvas selon le mode de mise à l'échelle.
    fn layout(&mut self) {
        let Some(win) = web_sys::window() else { return };
        let ww = win.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(960.0) as u32;
        let wh = win.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(720.0) as u32;
        let ratio = win.device_pixel_ratio().max(1.0);
        // En mode entier, on raisonne en pixels physiques pour rester net.
        let (pw, ph) = ((ww as f64 * ratio) as u32, (wh as f64 * ratio) as u32);
        let (x, y, w, h) = self.game.options().viewport(pw, ph);
        let v = (x, y, w, h);
        if v == self.css {
            return;
        }
        self.css = v;
        let st = self.canvas.style();
        let px = |p: u32| format!("{}px", p as f64 / ratio);
        let _ = st.set_property("width", &px(w));
        let _ = st.set_property("height", &px(h));
    }

    fn mouse_pos(&self, e: &MouseEvent) -> (f32, f32) {
        let r = self.canvas.get_bounding_client_rect();
        let x = (e.client_x() as f64 - r.left()) / r.width().max(1.0) * W as f64;
        let y = (e.client_y() as f64 - r.top()) / r.height().max(1.0) * H as f64;
        (x as f32, y as f32)
    }
}

fn listen<E: wasm_bindgen::convert::FromWasmAbi + 'static>(target: &web_sys::EventTarget, name: &str, f: impl FnMut(E) + 'static) -> Result<(), JsValue> {
    let cb = Closure::<dyn FnMut(E)>::new(f);
    target.add_event_listener_with_callback(name, cb.as_ref().unchecked_ref())?;
    cb.forget();
    Ok(())
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let window = web_sys::window().ok_or("pas de window")?;
    let doc = window.document().ok_or("pas de document")?;
    let canvas: HtmlCanvasElement = doc.get_element_by_id("screen").ok_or("canvas #screen introuvable")?.dyn_into()?;
    canvas.set_width(W as u32);
    canvas.set_height(H as u32);
    let ctx: CanvasRenderingContext2d = canvas.get_context("2d")?.ok_or("pas de contexte 2d")?.dyn_into()?;

    let mixer = Mixer::shared();
    let app = Rc::new(RefCell::new(App {
        game: Game::new(mixer.clone(), Box::new(LocalStorage), false),
        input: InputCollector::new(),
        ctx,
        canvas: canvas.clone(),
        rgba: vec![0; W * H * 4],
        last: None,
        mixer,
        stream: None,
        audio_tried: false,
        css: (0, 0, 0, 0),
    }));
    {
        let app = app.clone();
        let dt: &web_sys::EventTarget = doc.as_ref();
        listen(dt, "fullscreenchange", move |_: web_sys::Event| app.borrow_mut().game.sync_fullscreen(is_fullscreen()))?;
    }

    let wt: &web_sys::EventTarget = window.as_ref();
    {
        let app = app.clone();
        listen(wt, "keydown", move |e: KeyboardEvent| {
            if let Some(k) = map_code(&e.code()) {
                e.prevent_default();
                let mut a = app.borrow_mut();
                if !e.repeat() {
                    a.input.key(k, true);
                }
                a.ensure_audio();
            }
        })?;
    }
    {
        let app = app.clone();
        listen(wt, "keyup", move |e: KeyboardEvent| {
            if let Some(k) = map_code(&e.code()) {
                e.prevent_default();
                app.borrow_mut().input.key(k, false);
            }
        })?;
    }
    {
        let app = app.clone();
        listen(wt, "blur", move |_: web_sys::Event| app.borrow_mut().input.reset())?;
    }
    let ct: &web_sys::EventTarget = canvas.as_ref();
    {
        let app = app.clone();
        listen(ct, "mousemove", move |e: MouseEvent| {
            let mut a = app.borrow_mut();
            let (x, y) = a.mouse_pos(&e);
            a.input.mouse_move(x, y);
        })?;
    }
    {
        let app = app.clone();
        listen(ct, "mouseleave", move |_: MouseEvent| app.borrow_mut().input.mouse_leave())?;
    }
    {
        let app = app.clone();
        listen(ct, "mousedown", move |e: MouseEvent| {
            e.prevent_default();
            let mut a = app.borrow_mut();
            let (x, y) = a.mouse_pos(&e);
            a.input.mouse_move(x, y);
            a.input.mouse_button(e.button() as u32, true);
            a.ensure_audio();
        })?;
    }
    {
        let app = app.clone();
        listen(wt, "mouseup", move |e: MouseEvent| app.borrow_mut().input.mouse_button(e.button() as u32, false))?;
    }
    listen(ct, "contextmenu", move |e: MouseEvent| e.prevent_default())?;

    // Boucle requestAnimationFrame.
    let f: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let g = f.clone();
    *g.borrow_mut() = Some(Closure::new(move |ts: f64| {
        app.borrow_mut().tick(ts);
        if let Some(w) = web_sys::window() {
            if let Some(cb) = f.borrow().as_ref() {
                let _ = w.request_animation_frame(cb.as_ref().unchecked_ref());
            }
        }
    }));
    window.request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref())?;
    Ok(())
}
