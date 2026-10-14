//! Cop artificial intelligence, done the way the Pac-Man ghosts do it: deliberately not too clever.
//!
//! At every junction a cop looks at the ways it could go (never straight back the way it came)
//! and takes the one whose next tile is closest *as the crow flies* to its target. It does not
//! plan routes, so it can be led astray by walls, and it never stops. Cops alternate between
//! chasing and "scattering" to their own corner of the maze, which gives you breathing room.
//! The four personalities differ only in what they chase:
//!
//! * **Hunter**   - straight at you.
//! * **Ambusher** - the spot a few tiles ahead of where you are heading.
//! * **Flanker**  - a point mirrored through you from the Hunter, so the two pincer you.
//! * **Prowler**  - chases you from afar, but loses interest and wanders home when close.

use crate::grid::*;
use crate::levels::Level;
use std::collections::VecDeque;

pub type DistMap = [[i32; W]; H];
pub const FAR: i32 = i32::MAX / 4;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Personality {
    Hunter,
    Ambusher,
    Flanker,
    Prowler,
}

impl Personality {
    pub const ROSTER: [Personality; 4] = [
        Personality::Hunter,
        Personality::Ambusher,
        Personality::Flanker,
        Personality::Prowler,
    ];
}

/// What the AI is allowed to know about the player.
#[derive(Copy, Clone, Debug)]
pub struct PlayerView {
    pub tile: Pos,
    pub heading: Option<Dir>,
}

pub fn cop_can_enter(level: &Level, p: Pos) -> bool {
    matches!(level.tile(p), Tile::Floor | Tile::Pen)
}

/// Shortest path length from every tile to `target`, walking as a cop would.
pub fn distances_to(level: &Level, target: Pos) -> DistMap {
    let mut dist = [[FAR; W]; H];
    let target = nearest_walkable(level, target);
    dist[target.1 as usize][target.0 as usize] = 0;
    let mut queue = VecDeque::from([target]);
    while let Some(p) = queue.pop_front() {
        let d = dist[p.1 as usize][p.0 as usize];
        for dir in Dir::ALL {
            let n = step(p, dir);
            if in_bounds(n) && cop_can_enter(level, n) && dist[n.1 as usize][n.0 as usize] == FAR {
                dist[n.1 as usize][n.0 as usize] = d + 1;
                queue.push_back(n);
            }
        }
    }
    dist
}

/// The walkable (non-pen floor preferred) tile closest to `p`.
pub fn nearest_walkable(level: &Level, p: Pos) -> Pos {
    let p = (p.0.clamp(0, W as i32 - 1), p.1.clamp(0, H as i32 - 1));
    if matches!(level.tile(p), Tile::Floor) {
        return p;
    }
    let mut best = p;
    let mut best_d = FAR;
    for y in 0..H as i32 {
        for x in 0..W as i32 {
            if matches!(level.tile((x, y)), Tile::Floor) {
                let d = manhattan(p, (x, y));
                if d < best_d {
                    best_d = d;
                    best = (x, y);
                }
            }
        }
    }
    best
}

/// The corner each cop drifts to while scattering (targets may lie outside the maze).
pub fn home_corner(kind: Personality) -> Pos {
    match kind {
        Personality::Hunter => (W as i32 - 2, -2),
        Personality::Ambusher => (1, -2),
        Personality::Flanker => (W as i32 - 1, H as i32 + 1),
        Personality::Prowler => (0, H as i32 + 1),
    }
}

/// Where this cop wants to be right now.
pub fn pick_target(
    kind: Personality,
    me: Pos,
    hunter: Pos,
    player: PlayerView,
    scatter: bool,
) -> Pos {
    if scatter {
        return home_corner(kind);
    }
    match kind {
        Personality::Hunter => player.tile,
        Personality::Ambusher => ahead(player, 4),
        Personality::Flanker => {
            let a = ahead(player, 2);
            (a.0 + (a.0 - hunter.0), a.1 + (a.1 - hunter.1))
        }
        Personality::Prowler => {
            let (dx, dy) = (me.0 - player.tile.0, me.1 - player.tile.1);
            if dx * dx + dy * dy > 36 {
                player.tile
            } else {
                home_corner(kind)
            }
        }
    }
}

fn ahead(player: PlayerView, tiles: i32) -> Pos {
    match player.heading {
        Some(d) => {
            let (dx, dy) = d.delta();
            (player.tile.0 + dx * tiles, player.tile.1 + dy * tiles)
        }
        None => player.tile,
    }
}

/// The order Pac-Man ghosts break ties in.
const TIE_ORDER: [Dir; 4] = [Dir::Up, Dir::Left, Dir::Down, Dir::Right];

/// Pick the next direction at a tile centre. Always returns a direction if any move exists.
///
/// Out in the maze: the open way (not back) whose next tile is nearest the target in a straight
/// line. Inside the donut shop the cop just takes the shortest way out, and once out it never
/// walks back in by itself.
pub fn choose_direction(
    level: &Level,
    tile: Pos,
    heading: Option<Dir>,
    target: Pos,
) -> Option<Dir> {
    let in_pen = matches!(level.tile(tile), Tile::Pen);
    if in_pen {
        let exit = distances_to_exit_of_pen(level);
        return Dir::ALL
            .into_iter()
            .filter(|&d| {
                let n = step(tile, d);
                in_bounds(n) && cop_can_enter(level, n)
            })
            .min_by_key(|&d| {
                let n = step(tile, d);
                exit[n.1 as usize][n.0 as usize]
            });
    }
    let open = |d: Dir| {
        let n = step(tile, d);
        in_bounds(n) && matches!(level.tile(n), Tile::Floor)
    };
    let back = heading.map(Dir::opposite);
    let mut options: Vec<Dir> = TIE_ORDER
        .into_iter()
        .filter(|&d| open(d) && Some(d) != back)
        .collect();
    if options.is_empty() {
        // A dead end (there should be none): turning round is the only way.
        options = TIE_ORDER.into_iter().filter(|&d| open(d)).collect();
    }
    options.into_iter().min_by_key(|&d| {
        let n = step(tile, d);
        let (dx, dy) = (n.0 - target.0, n.1 - target.1);
        dx * dx + dy * dy
    })
}

/// Path length from every tile to the nearest corridor tile just outside the pen.
fn distances_to_exit_of_pen(level: &Level) -> DistMap {
    let mut dist = [[FAR; W]; H];
    let mut queue = VecDeque::new();
    for y in 0..H as i32 {
        for x in 0..W as i32 {
            let p = (x, y);
            let next_to_pen = Dir::ALL
                .iter()
                .any(|&d| matches!(level.tile(step(p, d)), Tile::Pen));
            if matches!(level.tile(p), Tile::Floor) && next_to_pen {
                dist[y as usize][x as usize] = 0;
                queue.push_back(p);
            }
        }
    }
    while let Some(p) = queue.pop_front() {
        let d = dist[p.1 as usize][p.0 as usize];
        for dir in Dir::ALL {
            let n = step(p, dir);
            if in_bounds(n)
                && matches!(level.tile(n), Tile::Pen)
                && dist[n.1 as usize][n.0 as usize] == FAR
            {
                dist[n.1 as usize][n.0 as usize] = d + 1;
                queue.push_back(n);
            }
        }
    }
    dist
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::levels::level;

    #[test]
    fn distances_are_consistent() {
        let l = level(0);
        let d = distances_to(&l, l.start);
        assert_eq!(d[l.start.1 as usize][l.start.0 as usize], 0);
        // every walkable tile can reach the target
        for y in 0..H {
            for x in 0..W {
                if cop_can_enter(&l, (x as i32, y as i32)) {
                    assert!(d[y][x] < FAR, "{x},{y} is cut off");
                }
            }
        }
    }

    #[test]
    fn cops_never_turn_back_by_themselves_and_always_have_a_move() {
        for n in 0..20 {
            let l = level(n);
            for y in 0..H as i32 {
                for x in 0..W as i32 {
                    if !matches!(l.tile((x, y)), Tile::Floor) {
                        continue;
                    }
                    for heading in Dir::ALL {
                        // Only headings a cop could actually arrive with.
                        if !matches!(
                            l.tile(step((x, y), heading.opposite())),
                            Tile::Floor | Tile::Pen
                        ) {
                            continue;
                        }
                        let d = choose_direction(&l, (x, y), Some(heading), (0, 0))
                            .unwrap_or_else(|| panic!("level {n}: stuck at {x},{y}"));
                        assert_ne!(d, heading.opposite(), "level {n}: U-turn at {x},{y}");
                    }
                }
            }
        }
    }

    #[test]
    fn cops_walk_out_of_the_donut_shop() {
        let l = level(0);
        let mut tile = l.pen[0];
        let mut heading = None;
        for _ in 0..6 {
            if matches!(l.tile(tile), Tile::Floor) {
                return;
            }
            let d = choose_direction(&l, tile, heading, (0, 0)).expect("a way out");
            tile = step(tile, d);
            heading = Some(d);
        }
        panic!("still inside");
    }

    #[test]
    fn nearest_walkable_snaps_walls_to_floor() {
        let l = level(0);
        let p = nearest_walkable(&l, (0, 0));
        assert!(matches!(l.tile(p), Tile::Floor));
    }
}
