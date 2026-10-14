//! Background music, composed and synthesized in code at startup (no audio files): a laid-back
//! one-drop reggae loop for playing, and a mellower dub version of it for the menus.
//!
//! Notes are written into a buffer exactly one loop long and wrap around its end, so echoes and
//! long tails carry over into the start and the loop repeats without a seam.

use crate::sfx::{wav, RATE};
use macroquad::audio::{
    load_sound_from_bytes, play_sound, set_sound_volume, stop_sound, PlaySoundParams, Sound,
};

const BPM: f32 = 78.0;
const BARS: usize = 8;

fn beat_len() -> f32 {
    60.0 / BPM
}

fn hz(midi: f32) -> f32 {
    440.0 * 2f32.powf((midi - 69.0) / 12.0)
}

/// A tiny deterministic noise source for drums.
struct Noise(u32);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

struct Track {
    buf: Vec<f32>,
}

impl Track {
    fn new() -> Track {
        let len = (BARS as f32 * 4.0 * beat_len() * RATE as f32) as usize;
        Track {
            buf: vec![0.0; len],
        }
    }

    /// Sample index of a position given in beats from the start of the loop.
    fn at(&self, beats: f32) -> usize {
        (beats * beat_len() * RATE as f32) as usize
    }

    /// Add a sound starting at `beats`, wrapping round the end of the loop.
    fn add(&mut self, beats: f32, samples: impl Iterator<Item = f32>) {
        let start = self.at(beats);
        let n = self.buf.len();
        for (i, s) in samples.enumerate() {
            self.buf[(start + i) % n] += s;
        }
    }

    /// A dub echo: repeats of the whole track, `delay` beats later, each `feedback` quieter.
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

    fn normalize(&mut self, peak: f32) {
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

fn env(i: usize, n: usize, attack: f32, curve: f32) -> f32 {
    let t = i as f32 / n as f32;
    let a = (i as f32 / (RATE as f32 * attack)).min(1.0);
    a * (1.0 - t).powf(curve)
}

/// Round, slightly overdriven bass.
fn bass(midi: f32, beats: f32) -> impl Iterator<Item = f32> {
    let n = (beats * beat_len() * RATE as f32) as usize;
    let f = hz(midi);
    (0..n).map(move |i| {
        let t = i as f32 / RATE as f32;
        let p = t * f * std::f32::consts::TAU;
        let v = p.sin() + 0.35 * (2.0 * p).sin() + 0.12 * (3.0 * p).sin();
        (v * 1.4).tanh() * 0.5 * env(i, n, 0.006, 0.7)
    })
}

/// A short, choppy chord: the reggae guitar "skank".
fn skank(chord: &[f32]) -> impl Iterator<Item = f32> {
    let n = (0.16 * RATE as f32) as usize;
    let freqs: Vec<f32> = chord.iter().map(|&m| hz(m + 12.0)).collect();
    let mut lp = 0.0f32;
    (0..n).map(move |i| {
        let t = i as f32 / RATE as f32;
        let raw: f32 = freqs
            .iter()
            .map(|&f| {
                let ph = (t * f).fract();
                0.55 * (4.0 * (ph - 0.5).abs() - 1.0) + 0.45 * if ph < 0.5 { 0.5 } else { -0.5 }
            })
            .sum::<f32>()
            / freqs.len() as f32;
        lp += (raw - lp) * 0.35;
        lp * 0.55 * env(i, n, 0.002, 3.0)
    })
}

/// A soft organ chord for the "bubble".
fn organ(chord: &[f32], beats: f32) -> impl Iterator<Item = f32> {
    let n = (beats * beat_len() * RATE as f32) as usize;
    let freqs: Vec<f32> = chord.iter().map(|&m| hz(m)).collect();
    (0..n).map(move |i| {
        let t = i as f32 / RATE as f32;
        let v: f32 = freqs
            .iter()
            .map(|&f| {
                let p = t * f * std::f32::consts::TAU;
                p.sin() + 0.4 * (2.0 * p).sin() + 0.2 * (4.0 * p).sin()
            })
            .sum::<f32>()
            / freqs.len() as f32;
        v * 0.16 * env(i, n, 0.01, 1.2)
    })
}

/// A singing lead with vibrato.
fn lead(midi: f32, beats: f32) -> impl Iterator<Item = f32> {
    let n = (beats * beat_len() * RATE as f32) as usize;
    let f = hz(midi);
    let mut phase = 0.0f32;
    (0..n).map(move |i| {
        let t = i as f32 / RATE as f32;
        let vib = 1.0 + 0.006 * (t * 5.5 * std::f32::consts::TAU).sin() * (t * 3.0).min(1.0);
        phase += f * vib / RATE as f32;
        let p = phase * std::f32::consts::TAU;
        (p.sin() + 0.25 * (2.0 * p).sin()) * 0.3 * env(i, n, 0.02, 0.9)
    })
}

fn kick() -> impl Iterator<Item = f32> {
    let n = (0.22 * RATE as f32) as usize;
    let mut phase = 0.0f32;
    (0..n).map(move |i| {
        let t = i as f32 / n as f32;
        phase += (45.0 + 90.0 * (1.0 - t).powi(4)) / RATE as f32;
        (phase * std::f32::consts::TAU).sin() * 0.9 * (1.0 - t).powf(1.5)
    })
}

fn rim(seed: u32) -> impl Iterator<Item = f32> {
    let n = (0.09 * RATE as f32) as usize;
    let mut noise = Noise(seed);
    (0..n).map(move |i| {
        let t = i as f32 / RATE as f32;
        let tone = (t * 820.0 * std::f32::consts::TAU).sin() * 0.5;
        (tone + noise.next() * 0.5) * 0.45 * env(i, n, 0.001, 4.0)
    })
}

fn hat(seed: u32, loud: f32) -> impl Iterator<Item = f32> {
    let n = (0.035 * RATE as f32) as usize;
    let mut noise = Noise(seed);
    let mut prev = 0.0f32;
    (0..n).map(move |i| {
        let x = noise.next();
        let hp = x - prev; // crude high-pass: keep the sizzle
        prev = x;
        hp * 0.07 * loud * env(i, n, 0.0005, 2.0)
    })
}

/// The chord for each bar (MIDI notes) and its bass root: Am | Dm | G | Em, twice.
const CHORDS: [([f32; 3], f32); 4] = [
    ([57.0, 60.0, 64.0], 45.0), // A minor
    ([57.0, 62.0, 65.0], 50.0), // D minor
    ([55.0, 59.0, 62.0], 43.0), // G
    ([55.0, 59.0, 64.0], 40.0), // E minor
];

/// The melody: (bar, beat, MIDI note, length in beats), A minor pentatonic.
const TUNE: [(usize, f32, f32, f32); 14] = [
    (4, 0.0, 76.0, 1.0),
    (4, 1.5, 74.0, 0.5),
    (4, 2.0, 72.0, 1.5),
    (5, 0.0, 74.0, 0.5),
    (5, 0.5, 72.0, 0.5),
    (5, 1.0, 69.0, 2.0),
    (6, 0.0, 67.0, 1.0),
    (6, 1.0, 69.0, 0.5),
    (6, 1.5, 72.0, 1.5),
    (6, 3.0, 74.0, 1.0),
    (7, 0.0, 76.0, 0.5),
    (7, 0.5, 74.0, 0.5),
    (7, 1.0, 72.0, 1.0),
    (7, 2.0, 69.0, 2.0),
];

/// Compose the loop. `chill` leaves out the drums and the busy parts (for the menus).
fn compose(chill: bool) -> Vec<f32> {
    let mut rhythm = Track::new();
    let mut chops = Track::new();
    let mut melody = Track::new();
    let mut seed = 1u32;
    for bar in 0..BARS {
        let (chord, root) = CHORDS[bar % 4];
        let b0 = bar as f32 * 4.0;
        // Bass: a lazy one-drop line around the root, fifth and octave.
        let line: &[(f32, f32, f32)] = if bar % 2 == 0 {
            &[
                (0.0, 0.0, 1.25),
                (1.5, 0.0, 0.5),
                (2.0, 7.0, 0.75),
                (3.0, 3.0, 0.5),
                (3.5, 0.0, 0.5),
            ]
        } else {
            &[
                (0.0, 0.0, 1.0),
                (1.0, 12.0, 0.5),
                (1.5, 10.0, 0.5),
                (2.0, 7.0, 1.0),
                (3.5, -2.0, 0.5),
            ]
        };
        for &(at, interval, len) in line {
            rhythm.add(
                b0 + at,
                bass(root + interval, len * if chill { 1.4 } else { 1.0 }),
            );
        }
        // Skank on beats 2 and 4, organ bubble on the off-beats.
        for beat in [1.0, 3.0] {
            chops.add(b0 + beat, skank(&chord));
        }
        if !chill {
            for k in 0..8 {
                if k % 2 == 1 {
                    rhythm.add(b0 + k as f32 * 0.5, organ(&chord, 0.4));
                }
            }
        } else {
            rhythm.add(b0, organ(&chord, 4.0));
        }
        // One-drop drums: kick and rim together on beat 3, hats on the eighths.
        if !chill {
            rhythm.add(b0 + 2.0, kick());
            seed += 1;
            rhythm.add(b0 + 2.0, rim(seed));
            if bar % 4 == 3 {
                seed += 1;
                rhythm.add(b0 + 3.5, rim(seed));
            }
            for k in 0..8 {
                seed += 1;
                rhythm.add(
                    b0 + k as f32 * 0.5,
                    hat(seed, if k % 2 == 1 { 1.0 } else { 0.55 }),
                );
            }
        }
    }
    for &(bar, beat, note, len) in &TUNE {
        let bar = if chill { (bar + 4) % BARS } else { bar };
        melody.add(
            bar as f32 * 4.0 + beat,
            lead(note - if chill { 12.0 } else { 0.0 }, len),
        );
    }
    // Dub echoes on the guitar and the tune.
    chops.echo(0.75, 0.38, 4);
    melody.echo(0.75, 0.4, 4);
    rhythm.mix(&chops, if chill { 0.7 } else { 1.0 });
    rhythm.mix(&melody, if chill { 0.8 } else { 0.9 });
    rhythm.normalize(0.85);
    rhythm.buf
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tune {
    Menu,
    Game,
}

pub struct Music {
    menu: Option<Sound>,
    game: Option<Sound>,
    playing: Option<Tune>,
    /// 0..=7
    pub volume: u8,
}

impl Music {
    pub async fn load(volume: u8) -> Music {
        let menu = load_sound_from_bytes(&wav(&compose(true))).await.ok();
        let game = load_sound_from_bytes(&wav(&compose(false))).await.ok();
        Music {
            menu,
            game,
            playing: None,
            volume,
        }
    }

    fn level(&self) -> f32 {
        // Music sits under the sound effects.
        (self.volume as f32 / 7.0).powi(2) * 0.55
    }

    fn sound(&self, tune: Tune) -> Option<&Sound> {
        match tune {
            Tune::Menu => self.menu.as_ref(),
            Tune::Game => self.game.as_ref(),
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
        if let Some(s) = tune.and_then(|t| self.sound(t)) {
            play_sound(
                s,
                PlaySoundParams {
                    looped: true,
                    volume: self.level(),
                },
            );
        }
    }

    pub fn set_volume(&mut self, volume: u8) {
        self.volume = volume.min(7);
        if let Some(s) = self.playing.and_then(|t| self.sound(t)) {
            set_sound_volume(s, self.level());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_loops_are_audible_and_loop_length() {
        for chill in [false, true] {
            let s = compose(chill);
            let expected = (BARS as f32 * 4.0 * beat_len() * RATE as f32) as usize;
            assert_eq!(s.len(), expected);
            let peak = s.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(peak > 0.5 && peak <= 0.86, "peak {peak}");
            // Something is playing in every bar.
            let bar = s.len() / BARS;
            for b in 0..BARS {
                let e: f32 =
                    s[b * bar..(b + 1) * bar].iter().map(|v| v * v).sum::<f32>() / bar as f32;
                assert!(e > 1e-4, "bar {b} is silent");
            }
        }
    }
}

#[cfg(test)]
mod export {
    /// `GIDDY_MUSIC_DIR=/some/dir cargo test --release -- --ignored export_music` writes both loops as WAV.
    #[test]
    #[ignore]
    fn export_music() {
        let dir = std::env::var("GIDDY_MUSIC_DIR").expect("set GIDDY_MUSIC_DIR");
        for (name, chill) in [("game", false), ("menu", true)] {
            let bytes = crate::sfx::wav(&super::compose(chill));
            std::fs::write(format!("{dir}/{name}.wav"), bytes).unwrap();
        }
    }
}
