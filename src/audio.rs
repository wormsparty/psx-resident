//! Sons synthétisés au démarrage (22 kHz, mono, lo-fi) et mixeur stéréo.
//! Le mixeur est partagé avec le thread audio (cpal) via `Arc<Mutex<_>>`.

use crate::math::Rng;
use std::f32::consts::TAU;
use std::sync::{Arc, Mutex};

pub const RATE: f32 = 22050.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Sfx {
    Shot,
    Empty,
    Reload,
    Step,
    Bark,
    Bark2,
    Howl,
    Groan,
    Groan2,
    Bite,
    Hurt,
    Creak,
    DoorShut,
    Pickup,
    Cursor,
    Typewriter,
    Hit,
    Fall,
    Frog,
    Wind,
    Fire,
    Drone,
    Death,
    Locked,
}

const COUNT: usize = Sfx::Locked as usize + 1;

pub struct Bank {
    sounds: Vec<Vec<f32>>,
}

// ---------------------------------------------------------------- outils DSP

struct Lp(f32);
impl Lp {
    fn run(&mut self, x: f32, k: f32) -> f32 {
        self.0 += (x - self.0) * k;
        self.0
    }
}

/// Filtre à variable d'état (passe-bande résonant).
struct Svf {
    low: f32,
    band: f32,
}
impl Svf {
    fn new() -> Svf {
        Svf { low: 0.0, band: 0.0 }
    }
    fn band(&mut self, x: f32, freq: f32, q: f32) -> f32 {
        let f = 2.0 * (std::f32::consts::PI * freq / RATE).sin();
        self.low += f * self.band;
        let high = x - self.low - q * self.band;
        self.band += f * high;
        self.band
    }
}

fn len(sec: f32) -> usize {
    (sec * RATE) as usize
}

fn normalize(v: &mut [f32], peak: f32) {
    let m = v.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
    if m > 1e-6 {
        for s in v.iter_mut() {
            *s *= peak / m;
        }
    }
}

/// Écho simple pour donner de l'espace.
fn echo(v: &mut Vec<f32>, delay: f32, fb: f32, extra: f32) {
    let d = len(delay);
    v.resize(v.len() + len(extra), 0.0);
    for i in d..v.len() {
        v[i] += v[i - d] * fb;
    }
}

fn gunshot(rng: &mut Rng) -> Vec<f32> {
    let n = len(0.6);
    let mut v = vec![0.0; n];
    let mut lp = Lp(0.0);
    let mut lp2 = Lp(0.0);
    for (i, s) in v.iter_mut().enumerate() {
        let t = i as f32 / RATE;
        let noise = rng.range(-1.0, 1.0);
        let crack = noise * (-t * 60.0).exp();
        let body = lp.run(noise, 0.25) * (-t * 14.0).exp() * 1.6;
        let thump = (TAU * (90.0 - t * 80.0) * t).sin() * (-t * 18.0).exp() * 1.2;
        let tail = lp2.run(noise, 0.05) * (-t * 5.0).exp() * 0.8;
        *s = crack + body + thump + tail;
    }
    echo(&mut v, 0.11, 0.25, 0.4);
    normalize(&mut v, 0.95);
    v
}

fn click(rng: &mut Rng, freq: f32, dur: f32) -> Vec<f32> {
    let n = len(dur);
    let mut svf = Svf::new();
    let mut v: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / RATE;
            svf.band(rng.range(-1.0, 1.0), freq, 0.2) * (-t * 120.0).exp() * 2.0
        })
        .collect();
    normalize(&mut v, 0.8);
    v
}

fn reload(rng: &mut Rng) -> Vec<f32> {
    let mut v = vec![0.0; len(0.9)];
    for (off, f) in [(0.0, 2200.0), (0.08, 1500.0), (0.55, 1800.0), (0.62, 3000.0)] {
        let c = click(rng, f, 0.06);
        let o = len(off);
        for (i, s) in c.iter().enumerate() {
            v[o + i] += s;
        }
    }
    normalize(&mut v, 0.6);
    v
}

fn step(rng: &mut Rng) -> Vec<f32> {
    let n = len(0.12);
    let mut lp = Lp(0.0);
    let mut v: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / RATE;
            let thud = (TAU * 70.0 * t).sin() * (-t * 50.0).exp();
            let scuff = lp.run(rng.range(-1.0, 1.0), 0.3) * (-t * 40.0).exp();
            thud * 0.8 + scuff * 1.2
        })
        .collect();
    normalize(&mut v, 0.5);
    v
}

/// Aboiement : fondamental qui chute, formant et souffle.
fn bark(rng: &mut Rng, base: f32, count: usize) -> Vec<f32> {
    let mut v = vec![0.0; len(0.42 * count as f32 + 0.2)];
    for k in 0..count {
        let off = len(k as f32 * rng.range(0.32, 0.45));
        let dur = rng.range(0.14, 0.2);
        let mut svf = Svf::new();
        let mut svf2 = Svf::new();
        let mut ph = 0.0f32;
        let f0 = base * rng.range(0.9, 1.1);
        for i in 0..len(dur) {
            let t = i as f32 / RATE;
            let x = t / dur;
            let f = f0 * (1.25 - 0.55 * x) * (1.0 + 0.05 * (t * 90.0).sin());
            ph = (ph + f / RATE) % 1.0;
            let saw = ph * 2.0 - 1.0 + rng.range(-0.35, 0.35);
            let env = (x * 25.0).min(1.0) * (1.0 - x).powf(1.6);
            let s = svf.band(saw, 900.0, 0.35) + svf2.band(saw, 2200.0, 0.5) * 0.5;
            if off + i < v.len() {
                v[off + i] += s * env;
            }
        }
    }
    // Lointain : passe-bas + écho de la vallée.
    let mut lp = Lp(0.0);
    for s in v.iter_mut() {
        *s = lp.run(*s, 0.35);
    }
    echo(&mut v, 0.27, 0.3, 0.8);
    normalize(&mut v, 0.8);
    v
}

fn howl(rng: &mut Rng) -> Vec<f32> {
    let dur = 2.6;
    let mut ph = 0.0f32;
    let mut svf = Svf::new();
    let mut v: Vec<f32> = (0..len(dur))
        .map(|i| {
            let t = i as f32 / RATE;
            let x = t / dur;
            let f = 420.0 + 260.0 * (x * 3.0).min(1.0) - 180.0 * ((x - 0.6).max(0.0) * 2.5) + 8.0 * (t * 30.0).sin();
            ph = (ph + f / RATE) % 1.0;
            let s = (ph * TAU).sin() + 0.3 * (ph * TAU * 2.0).sin() + rng.range(-0.08, 0.08);
            let env = (x * 6.0).min(1.0) * (1.0 - x).powf(0.8);
            svf.band(s, 800.0, 0.6) * env
        })
        .collect();
    echo(&mut v, 0.35, 0.35, 1.2);
    normalize(&mut v, 0.6);
    v
}

fn groan(rng: &mut Rng, base: f32, dur: f32) -> Vec<f32> {
    let mut ph = 0.0f32;
    let mut s1 = Svf::new();
    let mut s2 = Svf::new();
    let mut v: Vec<f32> = (0..len(dur))
        .map(|i| {
            let t = i as f32 / RATE;
            let x = t / dur;
            let f = base * (1.0 + 0.15 * (t * 5.3).sin() + 0.1 * (x * 3.0).sin()) + rng.range(-6.0, 6.0);
            ph = (ph + f / RATE) % 1.0;
            let saw = ph * 2.0 - 1.0 + rng.range(-0.5, 0.5) * 0.6;
            let env = (x * 5.0).min(1.0) * (1.0 - x).powf(0.7);
            let formant = 500.0 + 200.0 * (t * 1.7).sin();
            (s1.band(saw, formant, 0.25) + s2.band(saw, 1100.0, 0.4) * 0.4) * env
        })
        .collect();
    normalize(&mut v, 0.75);
    v
}

fn bite(rng: &mut Rng) -> Vec<f32> {
    let mut svf = Svf::new();
    let mut v: Vec<f32> = (0..len(0.45))
        .map(|i| {
            let t = i as f32 / RATE;
            let crunch = if rng.f() < 0.1 { rng.range(-1.0, 1.0) } else { 0.0 };
            svf.band(rng.range(-1.0, 1.0) + crunch * 3.0, 1200.0 - t * 1500.0, 0.3) * (-t * 6.0).exp()
        })
        .collect();
    normalize(&mut v, 0.8);
    v
}

/// Cri de douleur (voyelle "a" synthétique).
fn hurt(rng: &mut Rng, base: f32, dur: f32) -> Vec<f32> {
    let mut ph = 0.0f32;
    let mut f1 = Svf::new();
    let mut f2 = Svf::new();
    let mut f3 = Svf::new();
    let mut v: Vec<f32> = (0..len(dur))
        .map(|i| {
            let t = i as f32 / RATE;
            let x = t / dur;
            let f = base * (1.15 - 0.35 * x) * (1.0 + 0.02 * (t * 40.0).sin());
            ph = (ph + f / RATE) % 1.0;
            let src = ph * 2.0 - 1.0 + rng.range(-0.15, 0.15);
            let env = (x * 20.0).min(1.0) * (1.0 - x).powf(1.2);
            (f1.band(src, 800.0, 0.2) + f2.band(src, 1250.0, 0.25) * 0.7 + f3.band(src, 2800.0, 0.3) * 0.25) * env
        })
        .collect();
    normalize(&mut v, 0.7);
    v
}

fn creak(rng: &mut Rng) -> Vec<f32> {
    let dur = 1.5;
    let mut svf = Svf::new();
    let mut svf2 = Svf::new();
    let mut ph = 0.0f32;
    let mut v: Vec<f32> = (0..len(dur))
        .map(|i| {
            let t = i as f32 / RATE;
            let x = t / dur;
            // Frottement stick-slip : impulsions à fréquence variable.
            let f = 35.0 + 45.0 * (x * 4.0).sin().abs() + 20.0 * x;
            ph += f / RATE;
            let imp = if ph >= 1.0 {
                ph -= 1.0;
                1.0
            } else {
                0.0
            };
            let src = imp * 4.0 + rng.range(-0.05, 0.05);
            let env = (x * 8.0).min(1.0) * (1.0 - x).powf(0.5);
            (svf.band(src, 1100.0 + 400.0 * x, 0.08) + svf2.band(src, 2400.0, 0.12) * 0.5) * env
        })
        .collect();
    normalize(&mut v, 0.6);
    v
}

fn door_shut(rng: &mut Rng) -> Vec<f32> {
    let mut lp = Lp(0.0);
    let mut v: Vec<f32> = (0..len(0.7))
        .map(|i| {
            let t = i as f32 / RATE;
            (TAU * 55.0 * t).sin() * (-t * 9.0).exp() + lp.run(rng.range(-1.0, 1.0), 0.15) * (-t * 20.0).exp() * 2.0
        })
        .collect();
    echo(&mut v, 0.09, 0.3, 0.3);
    normalize(&mut v, 0.8);
    v
}

fn tones(notes: &[(f32, f32, f32)], total: f32) -> Vec<f32> {
    let mut v = vec![0.0; len(total)];
    for &(start, freq, dur) in notes {
        let o = len(start);
        for i in 0..len(dur) {
            let t = i as f32 / RATE;
            let s = ((TAU * freq * t).sin() + 0.3 * (TAU * freq * 2.0 * t).sin()) * (-t * 5.0 / dur).exp();
            if o + i < v.len() {
                v[o + i] += s;
            }
        }
    }
    normalize(&mut v, 0.45);
    v
}

fn typewriter(rng: &mut Rng) -> Vec<f32> {
    let mut v = vec![0.0; len(1.6)];
    let mut t = 0.0;
    while t < 1.3 {
        let f = rng.range(1800.0, 3200.0);
        let c = click(rng, f, 0.05);
        let o = len(t);
        for (i, s) in c.iter().enumerate() {
            v[o + i] += s;
        }
        t += rng.range(0.07, 0.16);
    }
    let bell = tones(&[(0.0, 2600.0, 0.4)], 0.4);
    let o = len(1.2);
    for (i, s) in bell.iter().enumerate() {
        if o + i < v.len() {
            v[o + i] += s * 0.5;
        }
    }
    normalize(&mut v, 0.6);
    v
}

fn hit(rng: &mut Rng) -> Vec<f32> {
    let mut lp = Lp(0.0);
    let mut v: Vec<f32> = (0..len(0.18))
        .map(|i| {
            let t = i as f32 / RATE;
            lp.run(rng.range(-1.0, 1.0), 0.2) * (-t * 25.0).exp() + (TAU * 120.0 * t).sin() * (-t * 30.0).exp()
        })
        .collect();
    normalize(&mut v, 0.7);
    v
}

fn fall(rng: &mut Rng) -> Vec<f32> {
    let mut lp = Lp(0.0);
    let mut v: Vec<f32> = (0..len(0.6))
        .map(|i| {
            let t = i as f32 / RATE;
            let a = (TAU * 48.0 * t).sin() * (-t * 7.0).exp();
            let b = lp.run(rng.range(-1.0, 1.0), 0.08) * (-t * 9.0).exp() * 2.0;
            a + b
        })
        .collect();
    normalize(&mut v, 0.8);
    v
}

fn frog(rng: &mut Rng) -> Vec<f32> {
    let mut v = vec![0.0; len(0.6)];
    let mut svf = Svf::new();
    let pulses = 2 + (rng.f() * 3.0) as usize;
    for k in 0..pulses {
        let o = len(k as f32 * 0.09);
        for i in 0..len(0.07) {
            let t = i as f32 / RATE;
            let s = (TAU * 210.0 * t).sin().signum() * (-t * 40.0).exp();
            v[o + i] += svf.band(s, 650.0, 0.3);
        }
    }
    normalize(&mut v, 0.5);
    v
}

/// Boucle de vent : bruit filtré avec modulation lente (raccord sans clic).
fn wind(rng: &mut Rng) -> Vec<f32> {
    let n = len(8.0);
    let mut lp = Lp(0.0);
    let mut svf = Svf::new();
    let mut v: Vec<f32> = (0..n)
        .map(|i| {
            let x = i as f32 / n as f32;
            let m = 0.5 + 0.5 * (x * TAU * 2.0).sin() * (x * TAU * 3.0).cos();
            let s = lp.run(rng.range(-1.0, 1.0), 0.04);
            svf.band(s, 300.0 + 250.0 * m, 0.7) * (0.4 + 0.6 * m)
        })
        .collect();
    crossfade_loop(&mut v, len(0.5));
    normalize(&mut v, 0.5);
    v
}

fn fire(rng: &mut Rng) -> Vec<f32> {
    let n = len(4.0);
    let mut lp = Lp(0.0);
    let mut v: Vec<f32> = (0..n)
        .map(|_| {
            let rumble = lp.run(rng.range(-1.0, 1.0), 0.02) * 0.6;
            let pop = if rng.f() < 0.0008 { rng.range(-1.0, 1.0) * 2.5 } else { 0.0 };
            rumble + pop
        })
        .collect();
    // Les craquements laissent une petite traîne.
    let mut svf = Svf::new();
    for s in v.iter_mut() {
        *s += svf.band(*s, 2500.0, 0.2) * 0.8;
    }
    crossfade_loop(&mut v, len(0.3));
    normalize(&mut v, 0.6);
    v
}

/// Nappe sourde et inquiétante.
fn drone() -> Vec<f32> {
    let n = len(12.0);
    let mut v: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / RATE;
            let x = i as f32 / n as f32;
            let lfo = (x * TAU).sin();
            // Fréquences choisies pour boucler exactement sur 12 s.
            let a = (TAU * 55.0 * t).sin();
            let b = (TAU * (55.0 * 1.5 + 1.0 / 12.0 * 2.0) * t).sin() * 0.5;
            let c = (TAU * 58.25 * t).sin() * (0.3 + 0.3 * lfo);
            let d = (TAU * 110.0 * t).sin() * 0.15 * (0.5 + 0.5 * (x * TAU * 3.0).cos());
            a + b + c + d
        })
        .collect();
    crossfade_loop(&mut v, len(1.0));
    normalize(&mut v, 0.35);
    v
}

fn death(rng: &mut Rng) -> Vec<f32> {
    let mut v = hurt(rng, 260.0, 1.1);
    let tail = tones(&[(0.0, 110.0, 2.0), (0.0, 116.5, 2.0)], 2.0);
    v.resize(tail.len().max(v.len()), 0.0);
    for (i, s) in tail.iter().enumerate() {
        v[i] += s * 0.6;
    }
    normalize(&mut v, 0.8);
    v
}

fn crossfade_loop(v: &mut Vec<f32>, fade: usize) {
    let n = v.len();
    if n <= fade * 2 {
        return;
    }
    for i in 0..fade {
        let k = i as f32 / fade as f32;
        v[i] = v[i] * k + v[n - fade + i] * (1.0 - k);
    }
    v.truncate(n - fade);
}

impl Bank {
    pub fn new() -> Bank {
        let mut rng = Rng::new(1234);
        let mut sounds: Vec<Vec<f32>> = vec![Vec::new(); COUNT];
        let mut set = |s: Sfx, v: Vec<f32>| sounds[s as usize] = v;
        set(Sfx::Shot, gunshot(&mut rng));
        set(Sfx::Empty, click(&mut rng, 2500.0, 0.08));
        set(Sfx::Reload, reload(&mut rng));
        set(Sfx::Step, step(&mut rng));
        set(Sfx::Bark, bark(&mut rng, 520.0, 3));
        set(Sfx::Bark2, bark(&mut rng, 380.0, 2));
        set(Sfx::Howl, howl(&mut rng));
        set(Sfx::Groan, groan(&mut rng, 85.0, 1.6));
        set(Sfx::Groan2, groan(&mut rng, 65.0, 2.2));
        set(Sfx::Bite, bite(&mut rng));
        set(Sfx::Hurt, hurt(&mut rng, 330.0, 0.35));
        set(Sfx::Creak, creak(&mut rng));
        set(Sfx::DoorShut, door_shut(&mut rng));
        set(Sfx::Pickup, tones(&[(0.0, 880.0, 0.25), (0.12, 1318.5, 0.5)], 0.7));
        set(Sfx::Cursor, tones(&[(0.0, 1500.0, 0.05)], 0.06));
        set(Sfx::Typewriter, typewriter(&mut rng));
        set(Sfx::Hit, hit(&mut rng));
        set(Sfx::Fall, fall(&mut rng));
        set(Sfx::Frog, frog(&mut rng));
        set(Sfx::Wind, wind(&mut rng));
        set(Sfx::Fire, fire(&mut rng));
        set(Sfx::Drone, drone());
        set(Sfx::Death, death(&mut rng));
        set(Sfx::Locked, {
            let mut v = click(&mut rng, 900.0, 0.1);
            let c2 = click(&mut rng, 700.0, 0.1);
            let o = v.len();
            v.resize(o + len(0.08), 0.0);
            v.extend(c2);
            normalize(&mut v, 0.6);
            v
        });
        Bank { sounds }
    }
}

impl Default for Bank {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------- mixeur

struct Voice {
    snd: usize,
    pos: f64,
    step: f64,
    vl: f32,
    vr: f32,
    tl: f32,
    tr: f32,
    looping: bool,
    tag: u32,
}

pub struct Mixer {
    bank: Bank,
    voices: Vec<Voice>,
    out_rate: f32,
    pub enabled: bool,
    pub master: f32,
}

pub type SharedMixer = Arc<Mutex<Mixer>>;

/// Volumes gauche/droite à puissance constante.
fn pan_gains(vol: f32, pan: f32) -> (f32, f32) {
    let p = (pan.clamp(-1.0, 1.0) + 1.0) * 0.25 * std::f32::consts::PI;
    (vol * p.cos(), vol * p.sin())
}

impl Mixer {
    pub fn new() -> Mixer {
        Mixer { bank: Bank::new(), voices: Vec::new(), out_rate: 48000.0, enabled: false, master: 0.8 }
    }

    pub fn shared() -> SharedMixer {
        Arc::new(Mutex::new(Mixer::new()))
    }

    pub fn set_rate(&mut self, rate: f32) {
        self.out_rate = rate.max(8000.0);
    }

    pub fn play(&mut self, s: Sfx, vol: f32, pan: f32, pitch: f32) {
        if !self.enabled || vol <= 0.001 {
            return;
        }
        if self.voices.len() >= 40 {
            if let Some(i) = self.voices.iter().position(|v| !v.looping) {
                self.voices.remove(i);
            }
        }
        let (l, r) = pan_gains(vol, pan);
        self.voices.push(Voice {
            snd: s as usize,
            pos: 0.0,
            step: RATE as f64 / self.out_rate as f64 * pitch as f64,
            vl: l,
            vr: r,
            tl: l,
            tr: r,
            looping: false,
            tag: 0,
        });
    }

    /// Crée ou met à jour une boucle identifiée par `tag`.
    pub fn set_loop(&mut self, tag: u32, s: Sfx, vol: f32, pan: f32) {
        let (l, r) = pan_gains(vol, pan);
        if let Some(v) = self.voices.iter_mut().find(|v| v.tag == tag) {
            v.tl = l;
            v.tr = r;
            return;
        }
        if !self.enabled {
            return;
        }
        self.voices.push(Voice {
            snd: s as usize,
            pos: 0.0,
            step: RATE as f64 / self.out_rate as f64,
            vl: 0.0,
            vr: 0.0,
            tl: l,
            tr: r,
            looping: true,
            tag,
        });
    }

    pub fn stop_all(&mut self) {
        self.voices.retain(|v| v.looping);
    }

    /// Remplit un tampon entrelacé de `channels` canaux.
    pub fn render(&mut self, out: &mut [f32], channels: usize) {
        out.fill(0.0);
        let channels = channels.max(1);
        let frames = out.len() / channels;
        let master = self.master;
        for v in &mut self.voices {
            let data = &self.bank.sounds[v.snd];
            if data.is_empty() {
                v.pos = f64::MAX;
                continue;
            }
            let n = data.len() as f64;
            for f in 0..frames {
                if v.pos >= n {
                    if v.looping {
                        v.pos -= n;
                    } else {
                        break;
                    }
                }
                let i = v.pos as usize;
                let fr = (v.pos - i as f64) as f32;
                let a = data[i];
                let b = if i + 1 < data.len() { data[i + 1] } else if v.looping { data[0] } else { 0.0 };
                let s = (a + (b - a) * fr) * master;
                v.vl += (v.tl - v.vl) * 0.0005;
                v.vr += (v.tr - v.vr) * 0.0005;
                if channels == 1 {
                    out[f] += s * (v.vl + v.vr) * 0.5;
                } else {
                    out[f * channels] += s * v.vl;
                    out[f * channels + 1] += s * v.vr;
                }
                v.pos += v.step;
            }
        }
        self.voices.retain(|v| v.looping || v.pos < self.bank.sounds[v.snd].len() as f64);
        for s in out.iter_mut() {
            // Saturation douce.
            *s = s.tanh();
        }
    }
}

impl Default for Mixer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sounds_are_valid() {
        let b = Bank::new();
        for (i, s) in b.sounds.iter().enumerate() {
            assert!(!s.is_empty(), "son {i} vide");
            assert!(s.iter().all(|x| x.is_finite() && x.abs() <= 1.0), "son {i} invalide");
            let peak = s.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
            assert!(peak > 0.1, "son {i} trop faible ({peak})");
        }
    }

    #[test]
    fn mixer_outputs_audio() {
        let mut m = Mixer::new();
        m.enabled = true;
        m.set_rate(48000.0);
        m.play(Sfx::Shot, 1.0, 0.3, 1.0);
        m.set_loop(1, Sfx::Wind, 0.5, 0.0);
        let mut buf = vec![0.0f32; 4096];
        m.render(&mut buf, 2);
        assert!(buf.iter().all(|x| x.is_finite()));
        assert!(buf.iter().any(|x| x.abs() > 0.05));
    }
}
