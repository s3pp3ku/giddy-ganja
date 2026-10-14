//! Background music, composed and synthesized in code at startup (no audio files).
//!
//! * **Game**: acid rave at 138 BPM. A squelchy resonant acid bassline whose filter slowly opens
//!   and closes, offbeat supersaw rave stabs, four-on-the-floor drums, a climbing arpeggio and a
//!   noise riser into every new phrase.
//! * **Menu**: the same rave heard from the chill-out room: slower, no drums, mostly pads.
//! * **Power**: when the cops are smoked out the music drops straight into dubstep: half-time
//!   drums, wobbling and "talking" growl basses, screeches, a sub, and snare rolls. When the smoke
//!   clears the rave comes back.
//!
//! Notes are written into a buffer exactly one loop long and wrap round its end, so echoes and
//! tails carry over into the start and the loop repeats without a seam.

use crate::sfx::{wav, RATE};
use macroquad::audio::{
    load_sound_from_bytes, play_sound, set_sound_volume, stop_sound, PlaySoundParams, Sound,
};
use std::f32::consts::{PI, TAU};

const SR: f32 = RATE as f32;

fn hz(midi: f32) -> f32 {
    440.0 * 2f32.powf((midi - 69.0) / 12.0)
}

/// A tiny deterministic noise source.
struct Noise(u32);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// Band-limited sawtooth (polyBLEP), so high notes do not fizz.
fn saw(phase: f32, dt: f32) -> f32 {
    let p = phase.rem_euclid(1.0);
    let mut v = 2.0 * p - 1.0;
    if p < dt {
        let t = p / dt;
        v -= t + t - t * t - 1.0;
    } else if p > 1.0 - dt {
        let t = (p - 1.0) / dt;
        v -= t * t + t + t + 1.0;
    }
    v
}

fn square(phase: f32, dt: f32) -> f32 {
    0.5 * (saw(phase, dt) - saw(phase + 0.5, dt))
}

/// State-variable filter with lowpass and bandpass outputs.
#[derive(Default, Clone)]
struct Svf {
    low: f32,
    band: f32,
}

impl Svf {
    /// `damp` 0.05 (screaming resonance) .. 1.4 (soft).
    fn run(&mut self, x: f32, cutoff: f32, damp: f32) -> (f32, f32) {
        let f = (2.0 * (PI * cutoff.clamp(20.0, SR * 0.22) / SR).sin()).min(1.1);
        // Two half-steps per sample keep it stable at high cutoffs.
        for _ in 0..2 {
            self.low += f * 0.5 * self.band;
            let high = x - self.low - damp * self.band;
            self.band += f * 0.5 * high;
        }
        (self.low, self.band)
    }
}

struct Track {
    buf: Vec<f32>,
    beat: f32,
}

impl Track {
    fn new(bpm: f32, bars: usize) -> Track {
        let beat = 60.0 / bpm;
        let len = (bars as f32 * 4.0 * beat * SR) as usize;
        Track {
            buf: vec![0.0; len],
            beat,
        }
    }

    fn at(&self, beats: f32) -> usize {
        (beats * self.beat * SR) as usize
    }

    fn samples(&self, beats: f32) -> usize {
        self.at(beats).max(1)
    }

    /// Add a sound starting at `beats`, wrapping round the end of the loop.
    fn add(&mut self, beats: f32, samples: &[f32]) {
        let start = self.at(beats);
        let n = self.buf.len();
        for (i, s) in samples.iter().enumerate() {
            self.buf[(start + i) % n] += s;
        }
    }

    /// Echoes of the whole track, `delay` beats apart, each `feedback` quieter.
    fn echo(&mut self, delay: f32, feedback: f32, repeats: usize) {
        let d = self.at(delay);
        let n = self.buf.len();
        let dry = self.buf.clone();
        let mut gain = feedback;
        for k in 1..=repeats {
            for (i, v) in dry.iter().enumerate() {
                self.buf[(i + d * k) % n] += v * gain;
            }
            gain *= feedback;
        }
    }

    fn mix(&mut self, other: &Track, gain: f32) {
        for (a, b) in self.buf.iter_mut().zip(&other.buf) {
            *a += b * gain;
        }
    }

    /// Saturate and level the whole mix: glues it together and keeps peaks in check.
    fn master(&mut self, drive: f32, peak: f32) {
        let m = self
            .buf
            .iter()
            .fold(0.0f32, |m, v| m.max(v.abs()))
            .max(1e-6);
        for s in &mut self.buf {
            *s = (*s / m * drive).tanh();
        }
        let m = self
            .buf
            .iter()
            .fold(0.0f32, |m, v| m.max(v.abs()))
            .max(1e-6);
        for s in &mut self.buf {
            *s = *s / m * peak;
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Drums

fn kick(punch: f32) -> Vec<f32> {
    let n = (0.3 * SR) as usize;
    let mut phase = 0.0f32;
    (0..n)
        .map(|i| {
            let t = i as f32 / SR;
            phase += (46.0 + 120.0 * (-t * 35.0).exp()) / SR;
            let body = ((phase * TAU).sin() * 1.5).tanh() * (-t * 8.0).exp();
            let click = if i < 50 {
                (1.0 - i as f32 / 50.0) * 0.6
            } else {
                0.0
            };
            (body * punch + click) * 0.95
        })
        .collect()
}

fn clap(seed: u32) -> Vec<f32> {
    let n = (0.22 * SR) as usize;
    let mut noise = Noise(seed);
    let mut f = Svf::default();
    (0..n)
        .map(|i| {
            let t = i as f32 / SR;
            let slap = [0.0, 0.011, 0.022]
                .iter()
                .map(|&o| {
                    if t >= o {
                        (-(t - o) * 180.0).exp()
                    } else {
                        0.0
                    }
                })
                .sum::<f32>()
                + 0.6 * (-t * 22.0).exp();
            let (_, band) = f.run(noise.next(), 1300.0, 0.6);
            band * slap * 1.4
        })
        .collect()
}

/// A huge dubstep snare: a tonal crack, a noise body and a long tail.
fn big_snare(seed: u32) -> Vec<f32> {
    let n = (0.45 * SR) as usize;
    let mut noise = Noise(seed);
    let mut f = Svf::default();
    let mut phase = 0.0f32;
    (0..n)
        .map(|i| {
            let t = i as f32 / SR;
            phase += (190.0 + 80.0 * (-t * 40.0).exp()) / SR;
            let tone = (phase * TAU).sin() * (-t * 25.0).exp();
            let (low, band) = f.run(noise.next(), 2500.0, 0.8);
            let body = (band * 1.2 + low * 0.4) * (-t * 9.0).exp();
            ((tone * 0.8 + body * 1.3) * 1.5).tanh() * 0.9
        })
        .collect()
}

fn hat(seed: u32, open: bool, loud: f32) -> Vec<f32> {
    let n = ((if open { 0.16 } else { 0.035 }) * SR) as usize;
    let mut noise = Noise(seed);
    let mut prev = 0.0f32;
    let decay = if open { 22.0 } else { 120.0 };
    (0..n)
        .map(|i| {
            let x = noise.next();
            let hp = x - prev;
            prev = x;
            hp * 0.09 * loud * (-(i as f32 / SR) * decay).exp()
        })
        .collect()
}

/// A crash for the start of a drop.
fn crash(seed: u32) -> Vec<f32> {
    let n = (1.6 * SR) as usize;
    let mut noise = Noise(seed);
    let mut prev = 0.0f32;
    (0..n)
        .map(|i| {
            let x = noise.next();
            let hp = x - prev * 0.6;
            prev = x;
            hp * 0.22 * (-(i as f32 / SR) * 2.6).exp()
        })
        .collect()
}

/// Rising filtered noise: the build-up into a new phrase.
fn riser(track: &Track, beats: f32, seed: u32) -> Vec<f32> {
    let n = track.samples(beats);
    let mut noise = Noise(seed);
    let mut f = Svf::default();
    (0..n)
        .map(|i| {
            let k = i as f32 / n as f32;
            let (_, band) = f.run(noise.next(), 300.0 + 6000.0 * k * k, 0.35);
            band * 0.3 * k * k
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Synths

/// One 303-style step: semitones above the root (None = rest), accent, slide into it.
type Step = (Option<f32>, bool, bool);

/// The acid bassline for one bar: a resonant sawtooth whose filter snaps open on each note.
/// `open` (0..1) is the slow twist of the cutoff knob across the phrase.
fn acid_bar(track: &Track, root: f32, steps: &[Step; 16], open: f32, damp: f32) -> Vec<f32> {
    let step_len = track.samples(0.25);
    let mut out = Vec::with_capacity(step_len * 16);
    let mut f = Svf::default();
    let mut phase = 0.0f32;
    let mut freq = hz(root);
    for (k, &(note, accent, slide)) in steps.iter().enumerate() {
        let next_slides = steps[(k + 1) % 16].2;
        let Some(n) = note else {
            for _ in 0..step_len {
                out.push(f.run(0.0, 200.0, damp).0 * 0.5);
            }
            continue;
        };
        let target = hz(root + n);
        for i in 0..step_len {
            let t = i as f32 / SR;
            // A slide glides into the new pitch instead of jumping.
            freq += (target - freq) * if slide { 0.004 } else { 1.0 };
            let dt = freq / SR;
            phase += dt;
            let gate = if next_slides || i < step_len * 7 / 10 {
                1.0
            } else {
                0.0
            };
            let amp = gate * if accent { 1.0 } else { 0.7 };
            let env = (-t * if accent { 9.0 } else { 14.0 }).exp();
            let cutoff = 180.0
                + 2600.0 * open * open
                + (900.0 + 2200.0 * open) * env * if accent { 1.6 } else { 1.0 };
            let (low, _) = f.run(saw(phase, dt), cutoff, damp);
            out.push((low * 1.8).tanh() * amp * 0.5);
        }
    }
    out
}

/// A chord of detuned saws through a filter: long and swelling for pads, short for rave stabs.
fn supersaw(track: &Track, chord: &[f32], beats: f32, stab: bool, bright: f32) -> Vec<f32> {
    let n = track.samples(beats);
    let detune = [-0.14, -0.06, 0.0, 0.07, 0.15];
    let mut phases: Vec<f32> = (0..chord.len() * detune.len())
        .map(|k| k as f32 * 0.37)
        .collect();
    let mut f = Svf::default();
    (0..n)
        .map(|i| {
            let t = i as f32 / SR;
            let mut v = 0.0;
            for (c, &m) in chord.iter().enumerate() {
                for (d, &cents) in detune.iter().enumerate() {
                    let fr = hz(m + cents);
                    let p = &mut phases[c * detune.len() + d];
                    *p += fr / SR;
                    v += saw(*p, fr / SR);
                }
            }
            v /= (chord.len() * detune.len()) as f32;
            let release = ((n - i) as f32 / (SR * if stab { 0.02 } else { 0.3 })).min(1.0);
            if stab {
                let env = (-t * 10.0).exp();
                let (low, _) = f.run(v, 600.0 + 5000.0 * bright * env, 0.5);
                low * 0.9 * (0.35 + 0.65 * env) * release
            } else {
                let lfo = 0.5 + 0.5 * (t * 0.5 * TAU).sin();
                let (low, _) = f.run(v, 400.0 + bright * 1600.0 * (0.4 + 0.6 * lfo), 0.7);
                low * 0.5 * (t / 0.4).min(1.0) * release
            }
        })
        .collect()
}

/// A short plucked note for arpeggios.
fn pluck(track: &Track, midi: f32, beats: f32) -> Vec<f32> {
    let n = track.samples(beats);
    let fr = hz(midi);
    let mut phase = 0.0f32;
    let mut f = Svf::default();
    (0..n)
        .map(|i| {
            let t = i as f32 / SR;
            let dt = fr / SR;
            phase += dt;
            let env = (-t * 8.0).exp();
            let (low, _) = f.run(square(phase, dt), 800.0 + 5000.0 * env, 0.45);
            let release = ((n - i) as f32 / (SR * 0.01)).min(1.0);
            low * 0.32 * (0.3 + 0.7 * env) * release
        })
        .collect()
}

/// How a dubstep bass hit is shaped.
#[derive(Clone, Copy)]
enum Growl {
    /// A lowpass opening and closing: wub-wub-wub.
    Wobble,
    /// Vowel formants sliding between "oh" and "ee": the talking yoi growl.
    Talk,
    /// A screaming, pitch-bending resonant lead an octave or two up.
    Screech,
}

/// One dubstep bass hit: `rate` wobbles per beat, `semis` above the root.
struct Hit {
    at: f32,
    len: f32,
    semis: f32,
    rate: f32,
    kind: Growl,
}

/// Render the dubstep bass for one bar from its hits, with a sine sub underneath.
fn dub_bar(track: &Track, root: f32, hits: &[Hit]) -> Vec<f32> {
    let n = track.samples(4.0);
    let mut out = vec![0.0f32; n];
    for (h_i, h) in hits.iter().enumerate() {
        let start = track.at(h.at);
        let len = track.samples(h.len);
        let f0 = hz(root + h.semis);
        let (mut p1, mut p2, mut p3, mut sub) = (0.0f32, 0.31f32, 0.0f32, 0.0f32);
        let (mut lp, mut f1, mut f2, mut hp) = (
            Svf::default(),
            Svf::default(),
            Svf::default(),
            Svf::default(),
        );
        let mut noise = Noise(77 + h_i as u32);
        for i in 0..len {
            let t = i as f32 / SR;
            let beats = t / track.beat;
            let lfo = 0.5 - 0.5 * (beats * h.rate * TAU).cos();
            let fade =
                (i as f32 / (SR * 0.004)).min(1.0) * ((len - i) as f32 / (SR * 0.012)).min(1.0);
            let v = match h.kind {
                Growl::Wobble | Growl::Talk => {
                    let fr = f0;
                    let dt = fr / SR;
                    // FM growl: a modulator whose depth follows the wobble.
                    p3 += fr * 2.0 / SR;
                    let fm = (p3 * TAU).sin() * (0.15 + 0.6 * lfo);
                    p1 += dt;
                    p2 += dt * 1.007;
                    let raw =
                        saw(p1 + fm * 0.3, dt) + saw(p2, dt) + square(p1 * 0.5, dt * 0.5) * 0.6;
                    match h.kind {
                        Growl::Wobble => {
                            let (low, _) = lp.run(raw, 120.0 + 3200.0 * lfo * lfo, 0.22);
                            low * 1.4
                        }
                        _ => {
                            // Formants from "oh" (450, 800 Hz) to "ee" (300, 2300 Hz).
                            let (_, b1) = f1.run(raw, 450.0 - 150.0 * lfo, 0.18);
                            let (_, b2) = f2.run(raw, 800.0 + 1500.0 * lfo, 0.15);
                            (b1 + b2 * 0.9) * 1.3
                        }
                    }
                }
                Growl::Screech => {
                    let bend = 1.0
                        + 0.06 * (beats * h.rate * TAU).sin()
                        + 0.12 * (t / (h.len * track.beat)).powi(2);
                    let fr = f0 * 4.0 * bend;
                    let dt = fr / SR;
                    p1 += dt;
                    p2 += dt * 1.012;
                    let raw = saw(p1, dt) + saw(p2, dt) + noise.next() * 0.08;
                    let (_, band) = hp.run(raw, 1800.0 + 2600.0 * lfo, 0.12);
                    band * 0.9
                }
            };
            // Sub: a clean sine an octave below, so it still thumps on small speakers' bigger cousins.
            sub += f0 * 0.5 / SR;
            let s = (sub * TAU).sin() * 0.55;
            let idx = start + i;
            if idx < n {
                out[idx] += ((v * 2.6).tanh() * 0.55 + s) * fade;
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------
// The tunes

const fn s(n: f32) -> Step {
    (Some(n), false, false)
}
const fn a(n: f32) -> Step {
    (Some(n), true, false)
}
const fn sl(n: f32) -> Step {
    (Some(n), false, true)
}
const R: Step = (None, false, false);

const ACID_A: [Step; 16] = [
    a(0.0),
    s(0.0),
    sl(12.0),
    s(0.0),
    R,
    s(0.0),
    s(10.0),
    a(12.0),
    s(0.0),
    R,
    s(7.0),
    sl(0.0),
    a(3.0),
    s(0.0),
    s(12.0),
    s(5.0),
];
const ACID_B: [Step; 16] = [
    a(0.0),
    sl(12.0),
    s(0.0),
    s(3.0),
    a(0.0),
    R,
    sl(15.0),
    s(12.0),
    s(0.0),
    s(0.0),
    a(10.0),
    s(0.0),
    sl(7.0),
    s(12.0),
    R,
    a(1.0),
];

/// The rave (`chill` = the chill-out room version for the menus).
fn compose_rave(chill: bool) -> Vec<f32> {
    let bpm = if chill { 100.0 } else { 138.0 };
    let bars = 8;
    let mut drums = Track::new(bpm, bars);
    let mut acid = Track::new(bpm, bars);
    let mut pads = Track::new(bpm, bars);
    let mut stabs = Track::new(bpm, bars);
    let mut arp = Track::new(bpm, bars);
    // A minor with a dark Phrygian turn: Am Am F G Am Am Bb G.
    let prog: [([f32; 3], f32); 8] = [
        ([57.0, 60.0, 64.0], 33.0),
        ([57.0, 60.0, 64.0], 33.0),
        ([57.0, 60.0, 65.0], 29.0),
        ([55.0, 59.0, 62.0], 31.0),
        ([57.0, 60.0, 64.0], 33.0),
        ([57.0, 60.0, 64.0], 33.0),
        ([58.0, 62.0, 65.0], 34.0),
        ([55.0, 59.0, 62.0], 31.0),
    ];
    let mut seed = 7u32;
    for (bar, (chord, root)) in prog.iter().enumerate() {
        let b0 = bar as f32 * 4.0;
        let open = 0.5 - 0.5 * (bar as f32 / bars as f32 * TAU).cos();
        let steps = if bar % 2 == 0 { &ACID_A } else { &ACID_B };
        let line = acid_bar(
            &acid,
            root + 12.0,
            steps,
            if chill { open * 0.4 } else { 0.2 + open * 0.8 },
            if chill { 0.5 } else { 0.15 },
        );
        acid.add(b0, &line);
        pads.add(
            b0,
            &supersaw(&pads, chord, 4.0, false, if chill { 0.5 } else { 0.6 }),
        );
        let tones = [
            chord[0],
            chord[1],
            chord[2],
            chord[0] + 12.0,
            chord[1] + 12.0,
            chord[2] + 12.0,
        ];
        if chill {
            for k in (0..16).step_by(2) {
                arp.add(
                    b0 + k as f32 * 0.25,
                    &pluck(&arp, tones[(k / 2 * 5 + bar) % 6] + 12.0, 0.4),
                );
            }
            continue;
        }
        // Rave stabs on the off-beats, an octave up.
        let up: Vec<f32> = chord.iter().map(|n| n + 12.0).collect();
        for beat in 0..4 {
            stabs.add(
                b0 + beat as f32 + 0.5,
                &supersaw(&stabs, &up, 0.4, true, 1.0),
            );
        }
        for k in 0..16 {
            arp.add(
                b0 + k as f32 * 0.25,
                &pluck(&arp, tones[(k * 5 / 2 + bar) % 6] + 24.0, 0.2),
            );
        }
        for beat in 0..4 {
            if !(bar == 7 && beat >= 2) {
                drums.add(b0 + beat as f32, &kick(1.0));
            }
            if beat % 2 == 1 {
                seed += 1;
                drums.add(b0 + beat as f32, &clap(seed));
            }
            seed += 1;
            drums.add(b0 + beat as f32 + 0.5, &hat(seed, true, 1.0));
            for q in [0.25, 0.75] {
                seed += 1;
                drums.add(b0 + beat as f32 + q, &hat(seed, false, 0.7));
            }
        }
        if bar == 7 {
            drums.add(b0, &riser(&drums, 4.0, 99));
            for k in 0..8 {
                seed += 1;
                let mut hit = clap(seed);
                let g = 0.3 + k as f32 * 0.09;
                hit.iter_mut().for_each(|v| *v *= g);
                drums.add(b0 + 2.0 + k as f32 * 0.25, &hit);
            }
        }
    }
    acid.echo(0.75, if chill { 0.45 } else { 0.22 }, 3);
    stabs.echo(0.75, 0.3, 2);
    arp.echo(0.75, 0.4, 4);
    pads.mix(&acid, if chill { 0.8 } else { 1.0 });
    pads.mix(&stabs, 0.55);
    pads.mix(&arp, if chill { 0.5 } else { 0.25 });
    pads.mix(&drums, 1.0);
    pads.master(1.6, 0.85);
    pads.buf
}

/// The drop: half-time dubstep at 140 BPM, starting with a crash and straight into the bass.
fn compose_dubstep() -> Vec<f32> {
    let bpm = 140.0;
    let bars = 4;
    let mut drums = Track::new(bpm, bars);
    let mut bass = Track::new(bpm, bars);
    let mut lead = Track::new(bpm, bars);
    let root = 29.0; // F1: the low, nasty end
    let h = |at: f32, len: f32, semis: f32, rate: f32, kind: Growl| Hit {
        at,
        len,
        semis,
        rate,
        kind,
    };
    use Growl::*;
    let bars_hits: [Vec<Hit>; 4] = [
        vec![
            h(0.0, 1.0, 0.0, 2.0, Wobble),
            h(1.0, 0.5, 0.0, 4.0, Talk),
            h(1.5, 0.5, 3.0, 8.0, Wobble),
            h(2.5, 0.75, 0.0, 1.5, Talk),
            h(3.25, 0.25, 12.0, 6.0, Wobble),
            h(3.5, 0.5, 7.0, 3.0, Screech),
        ],
        vec![
            h(0.0, 0.75, 0.0, 6.0, Wobble),
            h(0.75, 0.75, 5.0, 2.0, Talk),
            h(1.5, 0.5, 3.0, 4.0, Talk),
            h(2.5, 0.5, 0.0, 12.0, Wobble),
            h(3.0, 1.0, 10.0, 2.0, Screech),
        ],
        vec![
            h(0.0, 1.0, 0.0, 2.0, Wobble),
            h(1.0, 0.5, 0.0, 4.0, Talk),
            h(1.5, 0.5, 3.0, 8.0, Wobble),
            h(2.5, 0.75, 7.0, 1.5, Talk),
            h(3.25, 0.75, 8.0, 4.0, Screech),
        ],
        vec![
            h(0.0, 0.5, 0.0, 8.0, Wobble),
            h(0.5, 0.5, 0.0, 16.0, Wobble),
            h(1.0, 1.0, 5.0, 3.0, Talk),
            h(2.5, 0.5, 3.0, 6.0, Screech),
            h(3.0, 1.0, 0.0, 24.0, Wobble),
        ],
    ];
    let mut seed = 501u32;
    drums.add(0.0, &crash(9));
    for (bar, hits) in bars_hits.iter().enumerate() {
        let b0 = bar as f32 * 4.0;
        bass.add(b0, &dub_bar(&bass, root, hits));
        // Half-time: kick on 1 (and a skip before 3), the big snare on 3.
        drums.add(b0, &kick(1.2));
        drums.add(b0 + 1.75, &kick(0.9));
        if bar % 2 == 1 {
            drums.add(b0 + 3.5, &kick(0.8));
        }
        seed += 1;
        drums.add(b0 + 2.0, &big_snare(seed));
        for k in 0..8 {
            seed += 1;
            drums.add(b0 + k as f32 * 0.5, &hat(seed, k % 2 == 1, 0.9));
        }
        if bar == 3 {
            // Triplet hats, then a snare roll back into the drop.
            for k in 0..6 {
                seed += 1;
                drums.add(b0 + 1.0 + k as f32 / 3.0, &hat(seed, false, 1.0));
            }
            for k in 0..8 {
                seed += 1;
                let mut hit = big_snare(seed);
                let g = 0.25 + k as f32 * 0.07;
                hit.truncate((0.12 * SR) as usize);
                hit.iter_mut().for_each(|v| *v *= g);
                drums.add(b0 + 3.0 + k as f32 * 0.125, &hit);
            }
        }
        // A siren-like screech line floating over the top every other bar.
        if bar % 2 == 0 {
            let line = dub_bar(
                &lead,
                root + 12.0,
                &[Hit {
                    at: 2.0,
                    len: 1.0,
                    semis: 12.0,
                    rate: 1.0,
                    kind: Screech,
                }],
            );
            lead.add(b0, &line);
        }
    }
    lead.echo(0.75, 0.3, 2);
    bass.mix(&lead, 0.5);
    bass.mix(&drums, 1.1);
    bass.master(2.2, 0.9);
    bass.buf
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tune {
    Menu,
    Game,
    /// While the cops are smoked out.
    Power,
}

pub struct Music {
    menu: Option<Sound>,
    game: Option<Sound>,
    power: Option<Sound>,
    playing: Option<Tune>,
    /// 0..=7
    pub volume: u8,
}

impl Music {
    pub async fn load(volume: u8) -> Music {
        let menu = load_sound_from_bytes(&wav(&compose_rave(true))).await.ok();
        let game = load_sound_from_bytes(&wav(&compose_rave(false))).await.ok();
        let power = load_sound_from_bytes(&wav(&compose_dubstep())).await.ok();
        Music {
            menu,
            game,
            power,
            playing: None,
            volume,
        }
    }

    fn level(&self, tune: Tune) -> f32 {
        // Music sits under the sound effects; the drop is meant to hit harder.
        let boost = if tune == Tune::Power { 1.25 } else { 1.0 };
        (self.volume as f32 / 7.0).powi(2) * 0.55 * boost
    }

    fn sound(&self, tune: Tune) -> Option<&Sound> {
        match tune {
            Tune::Menu => self.menu.as_ref(),
            Tune::Game => self.game.as_ref(),
            Tune::Power => self.power.as_ref(),
        }
    }

    /// Make sure `tune` is the one playing (or nothing, for `None`).
    pub fn set(&mut self, tune: Option<Tune>) {
        if self.playing == tune {
            return;
        }
        if let Some(old) = self.playing.and_then(|t| self.sound(t)) {
            stop_sound(old);
        }
        self.playing = tune;
        if let Some(t) = tune {
            if let Some(s) = self.sound(t) {
                play_sound(
                    s,
                    PlaySoundParams {
                        looped: true,
                        volume: self.level(t),
                    },
                );
            }
        }
    }

    pub fn set_volume(&mut self, volume: u8) {
        self.volume = volume.min(7);
        if let Some(t) = self.playing {
            if let Some(s) = self.sound(t) {
                set_sound_volume(s, self.level(t));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(s: &[f32], bpm: f32, bars: usize) {
        let expected = (bars as f32 * 4.0 * 60.0 / bpm * SR) as usize;
        assert_eq!(s.len(), expected);
        assert!(s.iter().all(|v| v.is_finite()));
        let peak = s.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(peak > 0.5 && peak <= 0.91, "peak {peak}");
        let bar = s.len() / bars;
        for b in 0..bars {
            let e: f32 = s[b * bar..(b + 1) * bar].iter().map(|v| v * v).sum::<f32>() / bar as f32;
            assert!(e > 1e-4, "bar {b} is silent");
        }
    }

    #[test]
    fn every_loop_is_audible_finite_and_the_right_length() {
        check(&compose_rave(false), 138.0, 8);
        check(&compose_rave(true), 100.0, 8);
        check(&compose_dubstep(), 140.0, 4);
    }

    #[test]
    fn the_drop_is_louder_than_the_rave() {
        let rms = |s: &[f32]| (s.iter().map(|v| v * v).sum::<f32>() / s.len() as f32).sqrt();
        assert!(rms(&compose_dubstep()) > rms(&compose_rave(false)));
    }
}

#[cfg(test)]
mod export {
    /// `GIDDY_MUSIC_DIR=/some/dir cargo test --release -- --ignored export_music` writes the loops as WAV.
    #[test]
    #[ignore]
    fn export_music() {
        let dir = std::env::var("GIDDY_MUSIC_DIR").expect("set GIDDY_MUSIC_DIR");
        for (name, samples) in [
            ("game", super::compose_rave(false)),
            ("menu", super::compose_rave(true)),
            ("power", super::compose_dubstep()),
        ] {
            std::fs::write(format!("{dir}/{name}.wav"), crate::sfx::wav(&samples)).unwrap();
        }
    }
}
