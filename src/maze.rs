//! Generated gardens. A recursive-backtracker maze on a 9x5 cell grid, then "braided" with
//! extra openings so there are loops to run around (and no hopeless dead ends), then the pen,
//! start, exit, leaves and joints are placed. A level is only accepted if it validates.

use crate::grid::*;
use crate::levels::Level;
use crate::rng::Rng;

/// Generate a valid level. `n` is the zero-based level number, `seed` makes it reproducible.
pub fn generate(seed: u64, n: usize, cops: usize, speed: f32) -> Level {
    for attempt in 0..2000u64 {
        let mut rng = Rng::new(seed.wrapping_mul(1_000_003).wrapping_add(attempt));
        let art = carve(&mut rng, n);
        let name = crate::themes::level_name(n);
        let par = 70 + (n as u32).min(24) * 2;
        if let Ok(level) = Level::parse(&name, &art, cops, speed, par) {
            return level;
        }
    }
    // Practically unreachable (the generator succeeds within a handful of attempts), but never panic in a game.
    Level::parse(
        &crate::themes::level_name(n),
        FALLBACK,
        cops.min(4),
        speed,
        80,
    )
    .expect("fallback level is valid")
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

    // No dead ends: wherever a corridor stops, knock through to the nearest other corridor,
    // so that (as in Pac-Man) you can always keep running.
    let walkable = |wall: &[[bool; W]; H], x: i32, y: i32| {
        x > 0
            && y > 0
            && x < W as i32 - 1
            && y < H as i32 - 1
            && !wall[y as usize][x as usize]
            && !pen.contains(&(x as usize, y as usize))
    };
    for _ in 0..50 {
        let mut changed = false;
        for y in 1..H as i32 - 1 {
            for x in 1..W as i32 - 1 {
                if !walkable(&wall, x, y) {
                    continue;
                }
                let exits = Dir::ALL
                    .iter()
                    .filter(|d| {
                        let (dx, dy) = d.delta();
                        walkable(&wall, x + dx, y + dy)
                    })
                    .count();
                if exits >= 2 {
                    continue;
                }
                let through: Vec<(usize, usize)> = Dir::ALL
                    .iter()
                    .filter_map(|d| {
                        let (dx, dy) = d.delta();
                        let (wx, wy) = (x + dx, y + dy);
                        let inside = wx > 0 && wy > 0 && wx < W as i32 - 1 && wy < H as i32 - 1;
                        let is_wall = inside && wall[wy as usize][wx as usize];
                        let beyond = walkable(&wall, x + 2 * dx, y + 2 * dy);
                        (is_wall && beyond).then_some((wx as usize, wy as usize))
                    })
                    .collect();
                if !through.is_empty() {
                    let (wx, wy) = through[rng.below(through.len())];
                    wall[wy][wx] = false;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }

    // Any stub that could not be knocked through is filled in. (If that cuts the pen off, the
    // level fails validation and another attempt is made.)
    loop {
        let stubs: Vec<(usize, usize)> = (1..H as i32 - 1)
            .flat_map(|y| (1..W as i32 - 1).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                walkable(&wall, x, y)
                    && Dir::ALL
                        .iter()
                        .filter(|d| {
                            let (dx, dy) = d.delta();
                            walkable(&wall, x + dx, y + dy)
                        })
                        .count()
                        < 2
            })
            .map(|(x, y)| (x as usize, y as usize))
            .collect();
        if stubs.is_empty() {
            break;
        }
        for (x, y) in stubs {
            wall[y][x] = true;
        }
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
