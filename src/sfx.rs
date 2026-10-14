//! Sound effects, synthesized in code at startup: there are no audio files in this game.

use macroquad::audio::{load_sound_from_bytes, play_sound, PlaySoundParams, Sound};

pub const RATE: u32 = 22_050;

/// Encode mono samples (-1..1) as a 16-bit PCM WAV file in memory.
pub fn wav(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for &s in samples {
        out.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32000.0) as i16).to_le_bytes());
    }
    out
}

#[derive(Clone, Copy)]
enum Wave {
    Sine,
    Square,
    Triangle,
}

fn osc(wave: Wave, phase: f32) -> f32 {
    let p = phase.fract();
    match wave {
        Wave::Sine => (p * std::f32::consts::TAU).sin(),
        Wave::Square => {
            if p < 0.5 {
                0.6
            } else {
                -0.6
            }
        }
        Wave::Triangle => 4.0 * (p - 0.5).abs() - 1.0,
    }
}

/// One note that glides from `f0` to `f1` Hz over `secs`, with a short attack and a decaying tail.
fn note(wave: Wave, f0: f32, f1: f32, secs: f32, volume: f32) -> Vec<f32> {
    let n = (secs * RATE as f32) as usize;
    let mut phase = 0.0f32;
    (0..n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let f = f0 + (f1 - f0) * t;
            phase += f / RATE as f32;
            let attack = (i as f32 / (RATE as f32 * 0.004)).min(1.0);
            let decay = (1.0 - t).powf(1.6);
            osc(wave, phase) * volume * attack * decay
        })
        .collect()
}

fn concat(parts: Vec<Vec<f32>>) -> Vec<f32> {
    parts.into_iter().flatten().collect()
}

fn noise_burst(secs: f32, volume: f32, seed: u32) -> Vec<f32> {
    let n = (secs * RATE as f32) as usize;
    let mut state = seed.max(1);
    (0..n)
        .map(|i| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            let r = (state as f32 / u32::MAX as f32) * 2.0 - 1.0;
            let t = i as f32 / n as f32;
            r * volume * (1.0 - t).powi(2)
        })
        .collect()
}

fn mix(a: Vec<f32>, b: Vec<f32>) -> Vec<f32> {
    let n = a.len().max(b.len());
    (0..n)
        .map(|i| a.get(i).copied().unwrap_or(0.0) + b.get(i).copied().unwrap_or(0.0))
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sfx {
    LeafA,
    LeafB,
    JointPick,
    JointUse,
    NoTarget,
    Bust,
    Release,
    Caught,
    GateOpen,
    Cleared,
    ExtraLife,
    Ready,
    MenuMove,
    MenuSelect,
    GameOver,
    /// A wood-block tock: the smoke is wearing off.
    Tick,
}

pub struct Sounds {
    sounds: Vec<(Sfx, Sound)>,
    /// 0..=7
    pub volume: u8,
}

fn build(sfx: Sfx) -> Vec<f32> {
    match sfx {
        Sfx::LeafA => note(Wave::Triangle, 660.0, 880.0, 0.07, 0.5),
        Sfx::LeafB => note(Wave::Triangle, 880.0, 1100.0, 0.07, 0.5),
        Sfx::JointPick => concat(vec![
            note(Wave::Square, 660.0, 660.0, 0.06, 0.28),
            note(Wave::Square, 880.0, 880.0, 0.06, 0.28),
            note(Wave::Square, 1320.0, 1320.0, 0.12, 0.28),
        ]),
        Sfx::JointUse => mix(
            note(Wave::Sine, 300.0, 1800.0, 0.45, 0.5),
            noise_burst(0.35, 0.35, 7),
        ),
        Sfx::NoTarget => note(Wave::Square, 200.0, 160.0, 0.12, 0.25),
        Sfx::Bust => concat(vec![
            note(Wave::Square, 988.0, 988.0, 0.07, 0.3),
            note(Wave::Square, 1319.0, 1319.0, 0.22, 0.3),
        ]),
        Sfx::Release => note(Wave::Triangle, 220.0, 330.0, 0.1, 0.3),
        Sfx::Caught => {
            let n = (0.85 * RATE as f32) as usize;
            let mut phase = 0.0f32;
            (0..n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    let vib = (t * 40.0).sin() * 18.0;
                    let f = 620.0 * (1.0 - t).powf(1.4) + 90.0 + vib;
                    phase += f / RATE as f32;
                    osc(Wave::Square, phase) * 0.35 * (1.0 - t)
                })
                .collect()
        }
        Sfx::GateOpen => concat(
            [392.0, 523.0, 659.0, 784.0]
                .iter()
                .map(|&f| note(Wave::Triangle, f, f, 0.1, 0.45))
                .collect(),
        ),
        Sfx::Cleared => concat(vec![
            note(Wave::Square, 523.0, 523.0, 0.11, 0.3),
            note(Wave::Square, 659.0, 659.0, 0.11, 0.3),
            note(Wave::Square, 784.0, 784.0, 0.11, 0.3),
            note(Wave::Square, 1046.0, 1046.0, 0.4, 0.3),
        ]),
        Sfx::ExtraLife => concat(
            [523.0, 659.0, 784.0, 1046.0, 1319.0, 1568.0]
                .iter()
                .map(|&f| note(Wave::Triangle, f, f, 0.07, 0.5))
                .collect(),
        ),
        Sfx::Ready => note(Wave::Triangle, 440.0, 440.0, 0.16, 0.5),
        Sfx::MenuMove => note(Wave::Square, 520.0, 520.0, 0.04, 0.18),
        Sfx::MenuSelect => concat(vec![
            note(Wave::Square, 660.0, 660.0, 0.05, 0.22),
            note(Wave::Square, 990.0, 990.0, 0.09, 0.22),
        ]),
        Sfx::Tick => mix(
            note(Wave::Sine, 1250.0, 1000.0, 0.06, 0.55),
            note(Wave::Triangle, 2500.0, 2000.0, 0.03, 0.15),
        ),
        Sfx::GameOver => concat(
            [392.0, 349.0, 311.0, 262.0]
                .iter()
                .map(|&f| note(Wave::Triangle, f, f * 0.97, 0.28, 0.5))
                .collect(),
        ),
    }
}

impl Sounds {
    pub async fn load(volume: u8) -> Sounds {
        let all = [
            Sfx::LeafA,
            Sfx::LeafB,
            Sfx::JointPick,
            Sfx::JointUse,
            Sfx::NoTarget,
            Sfx::Bust,
            Sfx::Release,
            Sfx::Caught,
            Sfx::GateOpen,
            Sfx::Cleared,
            Sfx::ExtraLife,
            Sfx::Ready,
            Sfx::MenuMove,
            Sfx::MenuSelect,
            Sfx::GameOver,
            Sfx::Tick,
        ];
        let mut sounds = Vec::new();
        for sfx in all {
            // If a device cannot decode a sound the game simply stays quiet for it.
            if let Ok(s) = load_sound_from_bytes(&wav(&build(sfx))).await {
                sounds.push((sfx, s));
            }
        }
        Sounds { sounds, volume }
    }

    pub fn play(&self, sfx: Sfx) {
        if self.volume == 0 {
            return;
        }
        if let Some((_, s)) = self.sounds.iter().find(|(k, _)| *k == sfx) {
            // Squared so that each volume step sounds like an even step.
            let v = (self.volume as f32 / 7.0).powi(2);
            play_sound(
                s,
                PlaySoundParams {
                    looped: false,
                    volume: v,
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header_is_well_formed() {
        let bytes = wav(&[0.0, 0.5, -0.5, 1.0]);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(bytes.len(), 44 + 8);
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 8);
    }

    #[test]
    fn every_sound_is_audible_and_not_clipping() {
        for sfx in [
            Sfx::LeafA,
            Sfx::LeafB,
            Sfx::JointPick,
            Sfx::JointUse,
            Sfx::NoTarget,
            Sfx::Bust,
            Sfx::Release,
            Sfx::Caught,
            Sfx::GateOpen,
            Sfx::Cleared,
            Sfx::ExtraLife,
            Sfx::Ready,
            Sfx::MenuMove,
            Sfx::MenuSelect,
            Sfx::GameOver,
            Sfx::Tick,
        ] {
            let s = build(sfx);
            let peak = s.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(s.len() > 500, "{sfx:?} is too short");
            assert!(peak > 0.03, "{sfx:?} is silent");
            assert!(peak <= 1.2, "{sfx:?} would clip hard ({peak})");
        }
    }
}
