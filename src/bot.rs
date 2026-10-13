//! A simple autopilot. It commits to the nearest leaf (by real walking distance), eats it, picks
//! the next, then heads for the gate. It is used for the title-screen demo and in the tests, where
//! it proves that every level can be completed from start to finish.

use crate::game::{Mover, World};
use crate::grid::*;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Default)]
pub struct Autopilot {
    target: Option<Pos>,
}

type Walk = (HashMap<Pos, (Pos, Dir)>, Vec<Pos>);

fn can_enter(w: &World, p: Pos) -> bool {
    match w.level.tile(p) {
        Tile::Floor => true,
        Tile::Gate => w.gate_open,
        _ => false,
    }
}

/// Breadth-first walk from `from`: how each tile was reached, and the order they were visited.
fn walk(w: &World, from: Pos) -> Walk {
    let mut prev: HashMap<Pos, (Pos, Dir)> = HashMap::new();
    let mut order = Vec::new();
    let mut seen = HashSet::from([from]);
    let mut q = VecDeque::from([from]);
    while let Some(p) = q.pop_front() {
        order.push(p);
        for d in Dir::ALL {
            let n = step(p, d);
            if !seen.contains(&n) && can_enter(w, n) {
                seen.insert(n);
                prev.insert(n, (p, d));
                q.push_back(n);
            }
        }
    }
    (prev, order)
}

fn path_len(prev: &HashMap<Pos, (Pos, Dir)>, from: Pos, goal: Pos) -> i32 {
    let mut cur = goal;
    let mut n = 0;
    while cur != from {
        match prev.get(&cur) {
            Some(&(pp, _)) => {
                cur = pp;
                n += 1;
            }
            None => return 10_000,
        }
    }
    n
}

fn first_step(prev: &HashMap<Pos, (Pos, Dir)>, from: Pos, goal: Pos) -> Option<Dir> {
    if from == goal {
        return None;
    }
    let mut cur = goal;
    let mut first = None;
    while let Some(&(pp, d)) = prev.get(&cur) {
        first = Some(d);
        cur = pp;
        if cur == from {
            break;
        }
    }
    first
}

impl Autopilot {
    pub fn new() -> Self {
        Autopilot { target: None }
    }

    /// What to press this tick.
    pub fn decide(&mut self, w: &World) -> Option<Dir> {
        let m: &Mover = &w.player;
        let from = match (m.dir, m.progress > 0.0) {
            (Some(d), true) => step(m.tile, d),
            _ => m.tile,
        };
        let done = self.target.is_none_or(|t| {
            if w.leaves_left > 0 {
                !w.leaves[t.1 as usize][t.0 as usize]
            } else {
                t != w.level.exit
            }
        });
        if done {
            let (_, order) = walk(w, from);
            self.target = if w.leaves_left > 0 {
                order
                    .into_iter()
                    .find(|p| w.leaves[p.1 as usize][p.0 as usize] && *p != from)
                    .or(Some(from))
            } else {
                Some(w.level.exit)
            };
        }
        let goal = self.target?;
        match (m.dir, m.progress > 0.0) {
            (Some(d), true) => {
                let ahead = step(m.tile, d);
                let p = m.progress;
                let (prev_ahead, _) = walk(w, ahead);
                let (prev_back, _) = walk(w, m.tile);
                let via_ahead = (1.0 - p) + path_len(&prev_ahead, ahead, goal) as f32;
                let via_back = p + path_len(&prev_back, m.tile, goal) as f32;
                // Only turn around with room to spare: a reversal that arrives within the same tick
                // leaves no time to turn.
                if p > 0.4 && via_back + 0.001 < via_ahead {
                    Some(d.opposite())
                } else if ahead == goal {
                    Some(d)
                } else {
                    // Pressing the opposite key reverses at once, so never press it just to plan a
                    // U-turn at the next tile: keep going until turning around is safe.
                    match first_step(&prev_ahead, ahead, goal) {
                        Some(next) if next == d.opposite() => Some(d),
                        other => other,
                    }
                }
            }
            _ => {
                let (prev, _) = walk(w, m.tile);
                first_step(&prev, m.tile, goal)
            }
        }
    }
}
