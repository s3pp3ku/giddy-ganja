//! Saving high scores and settings. Plain text files in the platform's usual data folder, with no
//! extra dependencies. On the web (WebAssembly) there is no filesystem, so everything stays in memory.

use std::path::PathBuf;

pub const MAX_SCORES: usize = 10;
pub const NAME_LEN: usize = 3;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScoreEntry {
    pub name: String,
    pub score: u32,
    pub level: usize,
}

#[derive(Clone, Debug, Default)]
pub struct HighScores {
    pub entries: Vec<ScoreEntry>,
}

impl HighScores {
    /// Would `score` make the table?
    pub fn qualifies(&self, score: u32) -> bool {
        score > 0
            && (self.entries.len() < MAX_SCORES
                || self.entries.last().is_none_or(|e| score > e.score))
    }

    /// Insert a score, keeping the table sorted and trimmed. Returns its rank (0 = best).
    pub fn add(&mut self, name: &str, score: u32, level: usize) -> Option<usize> {
        if !self.qualifies(score) {
            return None;
        }
        let name: String = name
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .take(NAME_LEN)
            .collect::<String>()
            .to_uppercase();
        let name = if name.is_empty() {
            "???".to_string()
        } else {
            name
        };
        let rank = self
            .entries
            .iter()
            .position(|e| score > e.score)
            .unwrap_or(self.entries.len());
        self.entries.insert(rank, ScoreEntry { name, score, level });
        self.entries.truncate(MAX_SCORES);
        Some(rank)
    }

    pub fn serialize(&self) -> String {
        self.entries
            .iter()
            .map(|e| format!("{}\t{}\t{}\n", e.name, e.score, e.level))
            .collect()
    }

    pub fn parse(text: &str) -> HighScores {
        let mut entries: Vec<ScoreEntry> = text
            .lines()
            .filter_map(|line| {
                let mut parts = line.split('\t');
                let name = parts.next()?.to_string();
                let score = parts.next()?.parse().ok()?;
                let level = parts.next()?.parse().ok()?;
                Some(ScoreEntry { name, score, level })
            })
            .collect();
        entries.sort_by(|a, b| b.score.cmp(&a.score));
        entries.truncate(MAX_SCORES);
        HighScores { entries }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    /// Index into [`SPEEDS`].
    pub speed: usize,
    /// Volume 0..=7 (the original used the number keys 0 to 7).
    pub volume: u8,
    pub stop_on_release: bool,
    pub fullscreen: bool,
    /// The retro "Classic" look (pastel hedges on black, blue menus) instead of the "Modern" one.
    pub classic: bool,
}

pub const SPEEDS: [(&str, f32); 4] = [
    ("Relaxed", 0.8),
    ("Normal", 1.0),
    ("Fast", 1.2),
    ("Frantic", 1.45),
];

impl Default for Config {
    fn default() -> Self {
        Config {
            speed: 1,
            volume: 5,
            stop_on_release: false,
            fullscreen: false,
            classic: true,
        }
    }
}

impl Config {
    pub fn speed_factor(&self) -> f32 {
        SPEEDS[self.speed.min(SPEEDS.len() - 1)].1
    }

    pub fn serialize(&self) -> String {
        format!(
            "speed={}\nvolume={}\nstop_on_release={}\nfullscreen={}\nclassic={}\n",
            self.speed, self.volume, self.stop_on_release, self.fullscreen, self.classic
        )
    }

    pub fn parse(text: &str) -> Config {
        let mut c = Config::default();
        for line in text.lines() {
            if let Some((k, v)) = line.split_once('=') {
                match k.trim() {
                    "speed" => c.speed = v.trim().parse().unwrap_or(c.speed).min(SPEEDS.len() - 1),
                    "volume" => c.volume = v.trim().parse().unwrap_or(c.volume).min(7),
                    "stop_on_release" => c.stop_on_release = v.trim() == "true",
                    "fullscreen" => c.fullscreen = v.trim() == "true",
                    "classic" => c.classic = v.trim() == "true",
                    _ => {}
                }
            }
        }
        c
    }
}

/// The folder where this game keeps its files, or `None` where there is no filesystem.
pub fn data_dir() -> Option<PathBuf> {
    if cfg!(target_arch = "wasm32") {
        return None;
    }
    let base = if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join("Library").join("Application Support"))
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("share"))
            })
    };
    base.map(|b| b.join("giddy-ganja"))
}

fn read(name: &str) -> Option<String> {
    std::fs::read_to_string(data_dir()?.join(name)).ok()
}

fn write(name: &str, text: &str) {
    if let Some(dir) = data_dir() {
        // Failing to save must never crash the game.
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join(name), text);
    }
}

pub fn load_scores() -> HighScores {
    read("scores.txt")
        .map(|t| HighScores::parse(&t))
        .unwrap_or_default()
}

pub fn save_scores(scores: &HighScores) {
    write("scores.txt", &scores.serialize());
}

pub fn load_config() -> Config {
    read("config.txt")
        .map(|t| Config::parse(&t))
        .unwrap_or_default()
}

pub fn save_config(config: &Config) {
    write("config.txt", &config.serialize());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scores_stay_sorted_and_trimmed() {
        let mut s = HighScores::default();
        for i in 1..=15u32 {
            s.add("AAA", i * 100, 1);
        }
        assert_eq!(s.entries.len(), MAX_SCORES);
        assert_eq!(s.entries[0].score, 1500);
        assert!(s.entries.windows(2).all(|w| w[0].score >= w[1].score));
        assert!(!s.qualifies(100));
        assert!(s.qualifies(1_000_000));
    }

    #[test]
    fn rank_and_name_cleanup() {
        let mut s = HighScores::default();
        assert_eq!(s.add("bob!!!", 500, 2), Some(0));
        assert_eq!(s.entries[0].name, "BOB");
        assert_eq!(s.add("", 900, 3), Some(0));
        assert_eq!(s.entries[0].name, "???");
        assert_eq!(s.add("zed", 700, 1), Some(1));
        assert_eq!(s.add("nul", 0, 1), None);
    }

    #[test]
    fn scores_round_trip() {
        let mut s = HighScores::default();
        s.add("ABC", 1234, 5);
        s.add("XYZ", 99, 1);
        let back = HighScores::parse(&s.serialize());
        assert_eq!(back.entries, s.entries);
    }

    #[test]
    fn parsing_garbage_is_harmless() {
        let s = HighScores::parse("nonsense\n\t\t\nA\tB\tC\nOK\t5\t1\n");
        assert_eq!(s.entries.len(), 1);
        assert_eq!(s.entries[0].name, "OK");
        let c = Config::parse("speed=99\nvolume=200\nwat\nfullscreen=true\n");
        assert_eq!(c.speed, SPEEDS.len() - 1);
        assert_eq!(c.volume, 7);
        assert!(c.fullscreen);
    }

    #[test]
    fn config_round_trips() {
        let c = Config {
            speed: 2,
            volume: 3,
            stop_on_release: true,
            fullscreen: true,
            classic: false,
        };
        assert_eq!(Config::parse(&c.serialize()), c);
    }
}
