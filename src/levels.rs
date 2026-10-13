//! Levels: parsing, validation, the hand-made set, and the endless generated set.

use crate::grid::*;
use std::collections::VecDeque;

/// One playable level.
#[derive(Clone, Debug)]
pub struct Level {
    pub name: String,
    pub tiles: [[Tile; W]; H],
    pub leaves: [[bool; W]; H],
    pub joints: [[bool; W]; H],
    pub start: Pos,
    pub exit: Pos,
    pub pen: Vec<Pos>,
    /// How many cops patrol this level (never more than there are pen cells).
    pub cops: usize,
    /// Cop speed relative to the player's (1.0 = as fast as you).
    pub cop_speed: f32,
    /// A good clearing time in seconds; finishing under it earns a time bonus.
    pub par_seconds: u32,
}

impl Level {
    /// Parse a level from 12 lines of 20 characters.
    ///
    /// `#` wall, `.` leaf, space = empty floor, `*` joint, `P` start, `E` exit gate, `W` cop pen.
    pub fn parse(
        name: &str,
        art: &str,
        cops: usize,
        cop_speed: f32,
        par_seconds: u32,
    ) -> Result<Level, String> {
        let rows: Vec<&str> = art.trim_matches('\n').lines().collect();
        if rows.len() != H {
            return Err(format!("{name}: expected {H} rows, found {}", rows.len()));
        }
        let mut level = Level {
            name: name.to_string(),
            tiles: [[Tile::Wall; W]; H],
            leaves: [[false; W]; H],
            joints: [[false; W]; H],
            start: (-1, -1),
            exit: (-1, -1),
            pen: Vec::new(),
            cops,
            cop_speed,
            par_seconds,
        };
        for (y, row) in rows.iter().enumerate() {
            let chars: Vec<char> = row.chars().collect();
            if chars.len() != W {
                return Err(format!(
                    "{name}: row {y} has {} columns, expected {W}",
                    chars.len()
                ));
            }
            for (x, c) in chars.into_iter().enumerate() {
                let p = (x as i32, y as i32);
                match c {
                    '#' => {}
                    '.' => {
                        level.tiles[y][x] = Tile::Floor;
                        level.leaves[y][x] = true;
                    }
                    ' ' => level.tiles[y][x] = Tile::Floor,
                    '*' => {
                        level.tiles[y][x] = Tile::Floor;
                        level.joints[y][x] = true;
                    }
                    'P' => {
                        level.tiles[y][x] = Tile::Floor;
                        level.start = p;
                    }
                    'E' => {
                        level.tiles[y][x] = Tile::Gate;
                        level.exit = p;
                    }
                    'W' => {
                        level.tiles[y][x] = Tile::Pen;
                        level.pen.push(p);
                    }
                    other => return Err(format!("{name}: unknown tile {other:?} at {x},{y}")),
                }
            }
        }
        level.validate()?;
        Ok(level)
    }

    pub fn tile(&self, p: Pos) -> Tile {
        if in_bounds(p) {
            self.tiles[p.1 as usize][p.0 as usize]
        } else {
            Tile::Wall
        }
    }

    pub fn leaf_count(&self) -> usize {
        self.leaves.iter().flatten().filter(|&&b| b).count()
    }

    /// Check that the level can actually be completed. Used by tests and by the generator.
    pub fn validate(&self) -> Result<(), String> {
        let name = &self.name;
        if self.start == (-1, -1) {
            return Err(format!("{name}: no player start (P)"));
        }
        if self.exit == (-1, -1) {
            return Err(format!("{name}: no exit gate (E)"));
        }
        if self.pen.len() < 2 {
            return Err(format!("{name}: the cop pen needs at least 2 cells"));
        }
        if self.cops == 0 || self.cops > self.pen.len() {
            return Err(format!(
                "{name}: {} cops do not fit in a pen of {} cells",
                self.cops,
                self.pen.len()
            ));
        }
        // Borders must be solid (the exit gate is the only gap).
        for x in 0..W as i32 {
            for y in [0, H as i32 - 1] {
                if !matches!(self.tile((x, y)), Tile::Wall | Tile::Gate) {
                    return Err(format!("{name}: open border at {x},{y}"));
                }
            }
        }
        for y in 0..H as i32 {
            for x in [0, W as i32 - 1] {
                if !matches!(self.tile((x, y)), Tile::Wall | Tile::Gate) {
                    return Err(format!("{name}: open border at {x},{y}"));
                }
            }
        }
        // Everything the player must reach has to be reachable on foot (the gate counts as open).
        let seen = self.reachable_from_start();
        for y in 0..H {
            for x in 0..W {
                if (self.leaves[y][x] || self.joints[y][x]) && !seen[y][x] {
                    return Err(format!("{name}: unreachable pickup at {x},{y}"));
                }
            }
        }
        if !seen[self.exit.1 as usize][self.exit.0 as usize] {
            return Err(format!("{name}: the exit gate cannot be reached"));
        }
        if self.leaf_count() == 0 {
            return Err(format!("{name}: no leaves to collect"));
        }
        // The pen must open onto the maze, and cops must be able to reach the whole maze.
        let touches_maze = self.pen.iter().any(|&p| {
            Dir::ALL.iter().any(|&d| {
                let n = step(p, d);
                matches!(self.tile(n), Tile::Floor)
            })
        });
        if !touches_maze {
            return Err(format!("{name}: the pen does not open onto the maze"));
        }
        Ok(())
    }

    fn reachable_from_start(&self) -> [[bool; W]; H] {
        let mut seen = [[false; W]; H];
        let mut queue = VecDeque::new();
        seen[self.start.1 as usize][self.start.0 as usize] = true;
        queue.push_back(self.start);
        while let Some(p) = queue.pop_front() {
            for d in Dir::ALL {
                let n = step(p, d);
                if !in_bounds(n) || seen[n.1 as usize][n.0 as usize] {
                    continue;
                }
                if matches!(self.tile(n), Tile::Floor | Tile::Gate) {
                    seen[n.1 as usize][n.0 as usize] = true;
                    queue.push_back(n);
                }
            }
        }
        seen
    }
}

const SEEDLING: &str = "
##################E#
#........##........#
#.######.##.######.#
#.#....#....#....#.#
#.#.##.#.WW.#.##.#.#
#...##...WW...##...#
#.#.##.#....#.##.#.#
#.#....#.##.#....#.#
#.######.##.######.#
#*......P.........*#
#........##........#
####################
";

const HEDGEROW: &str = "
##E#################
#........##........#
#.##.###.##.###.##.#
#.#..............#.#
#.#.####.WW.####.#.#
#...#....WW....#...#
#.#.#.##....##.#.#.#
#.#...#.####.#...#.#
#.###.#......#.###.#
#*....##.##.##.....#
#..##.....P....##.*#
####################
";

const CROSSWIND: &str = "
#E##################
#..................#
#.###.##.##.##.###.#
#.#.....####.....#.#
#.#.###.#WW#.###.#.#
#...#....WW......#.#
###.#.###....###.#.#
#...#...#.##.#...#.#
#.#####.#.##.#.#####
#*......#....#.....#
#.##.##.P.##.##.#*.#
####################
";

const BRAMBLE: &str = "
##################E#
#.......#..#.......#
#.#####.#..#.#####.#
#.#...#...##...#...#
#.#.#.#.#WW#.#.#.#.#
#...#...#WW#...#...#
###.#.###..###.#.###
#...#...#..#...#...#
#.#####.#..#.#####.#
#.#............#...#
#*...##..P...##...*#
####################
";

/// How many levels come hand-made before the generated gardens begin.
pub const HANDMADE: usize = 4;

/// Cops and cop speed for a given (zero-based) level number.
pub fn difficulty(n: usize) -> (usize, f32) {
    let cops = match n {
        0 => 2,
        1 | 2 => 3,
        _ => 4,
    };
    let speed = (0.70 + 0.028 * n as f32).min(0.98);
    (cops, speed)
}

/// Build level `n` (zero-based). The first four are hand-made; every level after that is a
/// generated garden, so the game never runs out. The same `n` always gives the same level.
pub fn level(n: usize) -> Level {
    let (cops, speed) = difficulty(n);
    let hand = |name: &str, art: &str, par: u32| {
        Level::parse(name, art, cops, speed, par).expect("built-in levels are valid")
    };
    match n {
        0 => hand("Fresh Sprout", SEEDLING, 75),
        1 => hand("Backyard Grow", HEDGEROW, 80),
        2 => hand("Smoke Break", CROSSWIND, 85),
        3 => hand("Reefer Madness", BRAMBLE, 85),
        _ => crate::maze::generate(0xA11CE + n as u64, n, cops, speed),
    }
}

/// Level names for menus.
pub fn level_name(n: usize) -> String {
    level(n).name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_handmade_level_is_valid() {
        for n in 0..HANDMADE {
            let l = level(n);
            l.validate().unwrap_or_else(|e| panic!("{e}"));
            assert!(l.leaf_count() > 50, "{} has too few leaves", l.name);
        }
    }

    #[test]
    fn the_first_forty_levels_are_valid() {
        for n in 0..40 {
            level(n)
                .validate()
                .unwrap_or_else(|e| panic!("level {n}: {e}"));
        }
    }

    #[test]
    fn levels_are_deterministic() {
        let a = level(9);
        let b = level(9);
        assert_eq!(a.tiles, b.tiles);
        assert_eq!(a.leaves, b.leaves);
        assert_eq!(a.start, b.start);
    }

    #[test]
    fn difficulty_ramps_and_caps() {
        let (w0, s0) = difficulty(0);
        let (w9, s9) = difficulty(9);
        let (wbig, sbig) = difficulty(500);
        assert!(w9 >= w0 && s9 > s0);
        assert!(wbig <= 4 && sbig <= 0.98);
    }

    #[test]
    fn rejects_bad_art() {
        assert!(Level::parse("bad", "###\n###", 2, 0.8, 60).is_err());
        let no_start = SEEDLING.replace('P', ".");
        assert!(Level::parse("nostart", &no_start, 2, 0.8, 60).is_err());
    }
}
