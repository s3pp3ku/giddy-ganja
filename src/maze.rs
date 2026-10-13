//! Generated gardens. A recursive-backtracker maze on a 9x5 cell grid, then "braided" with
//! extra openings so there are loops to run around (and no hopeless dead ends), then the pen,
//! start, exit, leaves and joints are placed. A level is only accepted if it validates.

use crate::grid::*;
use crate::levels::Level;
use crate::rng::Rng;

const WORDS: [&str; 8] = [
    "Munchies",
    "Couch Lock",
    "Cloud Nine",
    "Pipe Dreams",
    "Hotbox",
    "Mellow Yellow",
    "Blazed Trails",
    "Grow Room",
];

/// Generate a valid level. `n` is the zero-based level number, `seed` makes it reproducible.
pub fn generate(seed: u64, n: usize, cops: usize, speed: f32) -> Level {
    for attempt in 0..2000u64 {
        let mut rng = Rng::new(seed.wrapping_mul(1_000_003).wrapping_add(attempt));
        let art = carve(&mut rng, n);
        let name = format!("{} {}", WORDS[n % WORDS.len()], n + 1);
        let par = 70 + (n as u32).min(24) * 2;
        if let Ok(level) = Level::parse(&name, &art, cops, speed, par) {
            return level;
        }
    }
    // Practically unreachable (the generator succeeds within a handful of attempts), but never panic in a game.
    Level::parse("Fresh Sprout", FALLBACK, cops.min(4), speed, 80).expect("fallback level is valid")
}

const FALLBACK: &str = "
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

fn carve(rng: &mut Rng, n: usize) -> String {
    // true = wall
    let mut wall = [[true; W]; H];
    let cells_x = 9usize;
    let cells_y = 5usize;
    let cell_pos = |cx: usize, cy: usize| -> (usize, usize) { (1 + cx * 2, 1 + cy * 2) };

    // Depth-first carve.
    let mut visited = [[false; 9]; 5];
    let mut stack = vec![(rng.below(cells_x), rng.below(cells_y))];
    visited[stack[0].1][stack[0].0] = true;
    let (sx, sy) = cell_pos(stack[0].0, stack[0].1);
    wall[sy][sx] = false;
    while let Some(&(cx, cy)) = stack.last() {
        let mut options = Vec::new();
        for d in Dir::ALL {
            let (dx, dy) = d.delta();
            let nx = cx as i32 + dx;
            let ny = cy as i32 + dy;
            if nx >= 0
                && ny >= 0
                && (nx as usize) < cells_x
                && (ny as usize) < cells_y
                && !visited[ny as usize][nx as usize]
            {
                options.push((nx as usize, ny as usize));
            }
        }
        if options.is_empty() {
            stack.pop();
            continue;
        }
        let (nx, ny) = options[rng.below(options.len())];
        visited[ny][nx] = true;
        let (px, py) = cell_pos(cx, cy);
        let (qx, qy) = cell_pos(nx, ny);
        wall[qy][qx] = false;
        wall[(py + qy) / 2][(px + qx) / 2] = false;
        stack.push((nx, ny));
    }

    // Braid: open some of the remaining walls between neighbouring cells to make loops.
    let braid = (0.20 + 0.01 * n.min(20) as f32).min(0.40);
    for cy in 0..cells_y {
        for cx in 0..cells_x {
            let (px, py) = cell_pos(cx, cy);
            if cx + 1 < cells_x && wall[py][px + 1] && rng.chance(braid) {
                wall[py][px + 1] = false;
            }
            if cy + 1 < cells_y && wall[py + 1][px] && rng.chance(braid) {
                wall[py + 1][px] = false;
            }
        }
    }

    // The pen: a 2x2 block in the middle.
    let pen = [(9usize, 5usize), (10, 5), (9, 6), (10, 6)];
    for &(x, y) in &pen {
        wall[y][x] = false;
    }

    // Start and exit on opposite sides.
    let start_left = rng.chance(0.5);
    let start = if start_left {
        (1usize, 9usize)
    } else {
        (17, 9)
    };
    let exit_x = if start_left { 17usize } else { 1 };

    // Render to ASCII.
    let mut art: Vec<Vec<char>> = vec![vec!['#'; W]; H];
    for y in 0..H {
        for x in 0..W {
            if !wall[y][x] {
                art[y][x] = '.';
            }
        }
    }
    for &(x, y) in &pen {
        art[y][x] = 'W';
    }
    art[start.1][start.0] = 'P';
    art[0][exit_x] = 'E';

    // Joints on far-away floor tiles.
    let want = 2 + (n / 6).min(2);
    let mut spots: Vec<(usize, usize)> = Vec::new();
    for y in 1..H - 1 {
        for x in 1..W - 1 {
            if art[y][x] == '.'
                && manhattan((x as i32, y as i32), (start.0 as i32, start.1 as i32)) >= 6
                && manhattan((x as i32, y as i32), (9, 5)) >= 3
            {
                spots.push((x, y));
            }
        }
    }
    rng.shuffle(&mut spots);
    for &(x, y) in spots.iter().take(want) {
        art[y][x] = '*';
    }

    art.iter()
        .map(|row| row.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_levels_validate_for_many_seeds() {
        for n in 4..80usize {
            let l = generate(0xA11CE + n as u64, n, 4, 0.9);
            l.validate().unwrap_or_else(|e| panic!("level {n}: {e}"));
            assert!(l.leaf_count() > 60, "level {n} is nearly empty");
            assert_eq!(l.pen.len(), 4);
        }
    }

    #[test]
    fn different_levels_look_different() {
        let a = generate(1, 4, 4, 0.9);
        let b = generate(2, 5, 4, 0.9);
        assert_ne!(a.tiles, b.tiles);
    }

    #[test]
    fn generation_is_reproducible() {
        let a = generate(42, 6, 4, 0.9);
        let b = generate(42, 6, 4, 0.9);
        assert_eq!(a.tiles, b.tiles);
        assert_eq!(a.joints, b.joints);
    }
}
