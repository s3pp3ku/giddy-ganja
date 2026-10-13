//! Shared grid types: sizes, positions, directions and tiles.

/// Playfield width in tiles. (The playfield is 20x12 tiles, like the classic arcade layout.)
pub const W: usize = 20;
/// Playfield height in tiles.
pub const H: usize = 12;

/// A tile position `(x, y)`, with `(0, 0)` at the top left.
pub type Pos = (i32, i32);

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
    pub const ALL: [Dir; 4] = [Dir::Up, Dir::Left, Dir::Down, Dir::Right];

    pub fn delta(self) -> (i32, i32) {
        match self {
            Dir::Up => (0, -1),
            Dir::Down => (0, 1),
            Dir::Left => (-1, 0),
            Dir::Right => (1, 0),
        }
    }

    pub fn opposite(self) -> Dir {
        match self {
            Dir::Up => Dir::Down,
            Dir::Down => Dir::Up,
            Dir::Left => Dir::Right,
            Dir::Right => Dir::Left,
        }
    }
}

pub fn step(p: Pos, d: Dir) -> Pos {
    let (dx, dy) = d.delta();
    (p.0 + dx, p.1 + dy)
}

pub fn in_bounds(p: Pos) -> bool {
    p.0 >= 0 && p.1 >= 0 && (p.0 as usize) < W && (p.1 as usize) < H
}

/// The permanent terrain. Leaves and joints are separate layers that get picked up.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Tile {
    Wall,
    Floor,
    /// The cop pen: cops start here and return here when busted. The player cannot enter.
    Pen,
    /// The exit gate. Closed (impassable) until every leaf is collected.
    Gate,
}

pub fn manhattan(a: Pos, b: Pos) -> i32 {
    (a.0 - b.0).abs() + (a.1 - b.1).abs()
}
