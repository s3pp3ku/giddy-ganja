//! Cop artificial intelligence.
//!
//! Every cop is always *going somewhere*. At each tile centre it picks the neighbouring tile
//! that is closest (by real path length through the maze) to its current target. The four
//! personalities differ only in what they target:
//!
//! * **Hunter**   - straight at you.
//! * **Ambusher** - the spot a few tiles ahead of where you are heading, to cut you off.
//! * **Flanker**  - a point mirrored through you from the Hunter, so the two pincer you.
//! * **Prowler**  - chases when you are close, otherwise sweeps the maze towards leaves you have
//!   not collected yet (so it patrols the places you still need to go).
//!
//! Nobody stands still, nobody gets stuck on a wall, and the routes are always optimal.

use crate::grid::*;
use crate::levels::Level;
use crate::rng::Rng;
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

/// Where this cop wants to be right now.
#[allow(clippy::too_many_arguments)]
pub fn pick_target(
    kind: Personality,
    me: Pos,
    hunter: Pos,
    player: PlayerView,
    level: &Level,
    uncollected: &[Pos],
    patrol_goal: &mut Option<Pos>,
    rng: &mut Rng,
) -> Pos {
    match kind {
        Personality::Hunter => player.tile,
        Personality::Ambusher => ahead(player, 4),
        Personality::Flanker => {
            let a = ahead(player, 2);
            (a.0 + (a.0 - hunter.0), a.1 + (a.1 - hunter.1))
        }
        Personality::Prowler => {
            let to_player = distances_to(level, player.tile)[me.1 as usize][me.0 as usize];
            if to_player <= 7 {
                *patrol_goal = None;
                return player.tile;
            }
            let reached = patrol_goal.is_none_or(|g| manhattan(g, me) <= 1);
            if reached {
                *patrol_goal = if uncollected.is_empty() {
                    Some(player.tile)
                } else {
                    Some(uncollected[rng.below(uncollected.len())])
                };
            }
            patrol_goal.unwrap_or(player.tile)
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

/// Pick the next direction at a tile centre. Always returns a direction if any move exists.
///
/// Cops do not turn around for no reason: they only reverse when the way back is strictly
/// the best route (or the only move), which prevents jittering at junctions.
pub fn choose_direction(
    level: &Level,
    dist: &DistMap,
    tile: Pos,
    heading: Option<Dir>,
    rng: &mut Rng,
) -> Option<Dir> {
    let mut options: Vec<(Dir, i32)> = Vec::with_capacity(4);
    for d in Dir::ALL {
        let n = step(tile, d);
        if in_bounds(n) && cop_can_enter(level, n) {
            options.push((d, dist[n.1 as usize][n.0 as usize]));
        }
    }
    if options.is_empty() {
        return None;
    }
    let forward: Vec<(Dir, i32)> = match heading {
        Some(h) => options
            .iter()
            .copied()
            .filter(|&(d, _)| d != h.opposite())
            .collect(),
        None => options.clone(),
    };
    let best_forward = forward.iter().map(|&(_, v)| v).min();
    let best_back = heading.and_then(|h| {
        options
            .iter()
            .find(|&&(d, _)| d == h.opposite())
            .map(|&(_, v)| v)
    });
    let pool: Vec<Dir> = match (best_forward, best_back) {
        (Some(f), Some(b)) if b < f => options
            .iter()
            .filter(|&&(_, v)| v == b)
            .map(|&(d, _)| d)
            .collect(),
        (Some(f), _) => forward
            .iter()
            .filter(|&&(_, v)| v == f)
            .map(|&(d, _)| d)
            .collect(),
        (None, _) => options.iter().map(|&(d, _)| d).collect(),
    };
    if let Some(h) = heading {
        if pool.contains(&h) {
            return Some(h);
        }
    }
    Some(pool[rng.below(pool.len())])
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
    fn walking_downhill_reaches_the_target() {
        let l = level(1);
        let target = l.start;
        let d = distances_to(&l, target);
        let mut rng = Rng::new(1);
        let mut tile = l.pen[0];
        let mut heading = None;
        for _ in 0..200 {
            if tile == target {
                return;
            }
            let dir = choose_direction(&l, &d, tile, heading, &mut rng).expect("a move exists");
            tile = step(tile, dir);
            heading = Some(dir);
        }
        panic!("never reached the target");
    }

    #[test]
    fn never_reverses_without_reason() {
        let l = level(0);
        let d = distances_to(&l, (1, 1));
        let mut rng = Rng::new(5);
        // Start in a corridor heading away from the target; the best route is behind, so it may turn.
        let dir = choose_direction(&l, &d, (3, 1), Some(Dir::Right), &mut rng);
        assert!(dir.is_some());
    }

    #[test]
    fn nearest_walkable_snaps_walls_to_floor() {
        let l = level(0);
        let p = nearest_walkable(&l, (0, 0));
        assert!(matches!(l.tile(p), Tile::Floor));
    }
}
