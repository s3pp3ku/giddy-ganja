//! The game rules and simulation. No graphics, no audio, no input devices: the front end feeds
//! this an [`Input`] each tick (60 per second) and reads back the state and a list of
//! [`Event`]s to show and sound. That keeps the rules fully testable.

use crate::ai::{self, Personality, PlayerView};
use crate::grid::*;
use crate::levels::{self, Level};

pub const TICKS_PER_SEC: u64 = 60;
/// Player speed in tiles per second at the Normal speed setting.
pub const PLAYER_TILES_PER_SEC: f32 = 5.4;
pub const MAX_JOINTS: u32 = 3;
/// How long a cop stays smoked out on level `n` (zero-based): 10 seconds on the first level,
/// half a second less each level after, never under 6. (Pac-Man's power pellets shrink from
/// 6 seconds to nothing; this is kinder, and always worth lighting up.)
pub fn stun_ticks(level_index: usize) -> u64 {
    let secs = (10.0 - 0.5 * level_index as f32).max(6.0);
    (secs * TICKS_PER_SEC as f32) as u64
}
pub const STUN_RADIUS: f32 = 6.0;
pub const EXTRA_LIFE_EVERY: u32 = 5000;
pub const READY_TICKS: u32 = 90;
pub const DYING_TICKS: u32 = 100;
pub const CLEARED_TICKS: u32 = 150;
/// How close (in tiles, centre to centre) a cop has to get to grab you.
const CATCH_DISTANCE: f32 = 0.7;
pub const LEAF_POINTS: u32 = 10;
pub const SPARKLER_POINTS: u32 = 50;
pub const PULSE_TICKS: u32 = 36;

/// What the player is doing this tick.
#[derive(Copy, Clone, Debug, Default)]
pub struct Input {
    /// A direction currently pressed (the newest one if several).
    pub dir: Option<Dir>,
    /// The action key (use a joint) is held. The game only reacts to the press, not the hold.
    pub action: bool,
}

#[derive(Clone, Debug)]
pub struct Settings {
    /// Global speed multiplier (the "Set Speed" menu): everyone's speed scales together.
    pub speed: f32,
    /// If true you stop when you let go of the keys; if false you keep going until you hit a wall.
    pub stop_on_release: bool,
    pub lives: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            speed: 1.0,
            stop_on_release: false,
            lives: 3,
        }
    }
}

/// What carries over from one level to the next.
#[derive(Copy, Clone, Debug)]
pub struct Carry {
    pub score: u32,
    pub lives: u32,
    pub joints: u32,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Phase {
    /// "Get ready": nobody moves yet.
    Ready(u32),
    Playing,
    /// You were caught: the death animation plays.
    Dying(u32),
    /// You reached the open gate.
    Cleared(u32),
    GameOver,
}

/// Something that happened this tick; the front end turns these into sounds and effects.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Event {
    LeafEaten,
    JointPicked,
    JointUsed,
    /// The action key was pressed but no cop was in range, so no joint was spent.
    NothingToStun,
    CopBusted(u32),
    CopReleased,
    PlayerCaught,
    GateOpened,
    LevelCleared {
        time_bonus: u32,
    },
    ExtraLife,
    GameOver,
}

/// A thing that moves along the grid: at tile `tile`, heading `dir`, `progress` of the way to the next tile.
#[derive(Copy, Clone, Debug)]
pub struct Mover {
    pub tile: Pos,
    pub dir: Option<Dir>,
    pub progress: f32,
}

impl Mover {
    pub fn new(tile: Pos) -> Self {
        Mover {
            tile,
            dir: None,
            progress: 0.0,
        }
    }

    /// Position in tile units, e.g. `(3.5, 2.0)` is halfway between tile (3,2) and (4,2).
    pub fn pos_f(&self) -> (f32, f32) {
        let (dx, dy) = self.dir.map_or((0, 0), |d| d.delta());
        (
            self.tile.0 as f32 + dx as f32 * self.progress,
            self.tile.1 as f32 + dy as f32 * self.progress,
        )
    }

    /// The tile this mover is closest to.
    pub fn nearest_tile(&self) -> Pos {
        match self.dir {
            Some(d) if self.progress >= 0.5 => step(self.tile, d),
            _ => self.tile,
        }
    }

    /// Turn around in place (possible at any moment, not only at tile centres).
    pub fn reverse(&mut self) {
        if let Some(d) = self.dir {
            if self.progress == 0.0 {
                self.dir = Some(d.opposite());
            } else {
                self.tile = step(self.tile, d);
                self.dir = Some(d.opposite());
                self.progress = 1.0 - self.progress;
            }
        }
    }

    /// Move `dist` tiles, asking `choose(tile, heading)` for a direction at every tile centre.
    /// Returns the last tile centre arrived at, if any.
    pub fn advance(
        &mut self,
        dist: f32,
        mut choose: impl FnMut(Pos, Option<Dir>) -> Option<Dir>,
    ) -> Option<Pos> {
        let mut remaining = dist;
        let mut arrived = None;
        // A tile is at most one unit, and speeds are well under one tile per tick, so this loop is short.
        for _ in 0..8 {
            match self.dir {
                None => match choose(self.tile, None) {
                    Some(d) => self.dir = Some(d),
                    None => break,
                },
                Some(d) => {
                    let need = 1.0 - self.progress;
                    if remaining < need {
                        self.progress += remaining;
                        break;
                    }
                    remaining -= need;
                    self.tile = step(self.tile, d);
                    self.progress = 0.0;
                    arrived = Some(self.tile);
                    self.dir = choose(self.tile, Some(d));
                    if self.dir.is_none() {
                        break;
                    }
                }
            }
        }
        arrived
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum CopState {
    /// Inside the pen until `release_at` (a tick count).
    Waiting {
        release_at: u64,
    },
    Hunting,
    /// Frozen by a joint until `until`. Touch it to bust it.
    Stunned {
        until: u64,
    },
}

#[derive(Clone, Debug)]
pub struct Cop {
    pub mover: Mover,
    pub kind: Personality,
    pub state: CopState,
    pub pen_cell: Pos,
}

#[derive(Copy, Clone, Debug)]
pub struct Pulse {
    pub center: (f32, f32),
    pub age: u32,
}

pub struct World {
    pub level_index: usize,
    pub level: Level,
    pub leaves: [[bool; W]; H],
    pub joint_tiles: [[bool; W]; H],
    pub player: Mover,
    pub cops: Vec<Cop>,
    pub score: u32,
    pub lives: u32,
    pub joints: u32,
    pub leaves_left: u32,
    pub gate_open: bool,
    pub phase: Phase,
    /// Ticks spent in `Phase::Playing` on this level.
    pub t: u64,
    pub pulse: Option<Pulse>,
    pub events: Vec<Event>,
    pub settings: Settings,
    /// Practice/debug: cops cannot catch you. Never on in normal play.
    pub invulnerable: bool,
    desired: Option<Dir>,
    was_scattering: bool,
    prev_action: bool,
    chain: u32,
    next_extra_life: u32,
}

impl World {
    pub fn new(level_index: usize, settings: Settings, carry: Option<Carry>) -> World {
        let level = levels::level(level_index);
        let carry = carry.unwrap_or(Carry {
            score: 0,
            lives: settings.lives,
            joints: 0,
        });
        let leaves = level.leaves;
        let leaves_left = level.leaf_count() as u32;
        let mut world = World {
            level_index,
            player: Mover::new(level.start),
            cops: Vec::new(),
            joint_tiles: level.joints,
            leaves,
            leaves_left,
            gate_open: false,
            score: carry.score,
            lives: carry.lives,
            joints: carry.joints.min(MAX_JOINTS),
            phase: Phase::Ready(READY_TICKS),
            t: 0,
            pulse: None,
            events: Vec::new(),
            desired: None,
            was_scattering: true,
            prev_action: false,
            chain: 0,
            next_extra_life: (carry.score / EXTRA_LIFE_EVERY + 1) * EXTRA_LIFE_EVERY,
            settings,
            invulnerable: false,
            level,
        };
        world.reset_actors();
        world
    }

    pub fn carry(&self) -> Carry {
        Carry {
            score: self.score,
            lives: self.lives,
            joints: self.joints,
        }
    }

    /// While any cop is smoked out: ticks until the last one wakes up, and the full duration.
    pub fn smoked_out(&self) -> Option<(u64, u64)> {
        self.cops
            .iter()
            .filter_map(|c| match c.state {
                CopState::Stunned { until } => Some(until.saturating_sub(self.t)),
                _ => None,
            })
            .max()
            .map(|left| (left, stun_ticks(self.level_index)))
    }

    pub fn elapsed_seconds(&self) -> f32 {
        self.t as f32 / TICKS_PER_SEC as f32
    }

    fn stagger(&self) -> u64 {
        (110u64.saturating_sub(self.level_index as u64 * 6)).max(45)
    }

    fn reset_actors(&mut self) {
        self.player = Mover::new(self.level.start);
        self.desired = None;
        self.chain = 0;
        self.pulse = None;
        let stagger = self.stagger();
        self.cops = (0..self.level.cops)
            .map(|i| {
                let cell = self.level.pen[i % self.level.pen.len()];
                Cop {
                    mover: Mover::new(cell),
                    kind: Personality::ROSTER[i % Personality::ROSTER.len()],
                    state: CopState::Waiting {
                        release_at: self.t + i as u64 * stagger,
                    },
                    pen_cell: cell,
                }
            })
            .collect();
    }

    fn player_speed(&self) -> f32 {
        PLAYER_TILES_PER_SEC * self.settings.speed / TICKS_PER_SEC as f32
    }

    fn cop_speed(&self) -> f32 {
        // Cops speed up a little when only a few leaves remain.
        let rush = if self.leaves_left <= 12 { 1.07 } else { 1.0 };
        self.player_speed() * self.level.cop_speed * rush
    }

    fn add_score(&mut self, points: u32) {
        self.score += points;
        while self.score >= self.next_extra_life {
            self.next_extra_life += EXTRA_LIFE_EVERY;
            self.lives += 1;
            self.events.push(Event::ExtraLife);
        }
    }

    /// Advance the game by one tick (1/60 s).
    pub fn tick(&mut self, input: Input) {
        self.events.clear();
        if let Some(p) = &mut self.pulse {
            p.age += 1;
            if p.age > PULSE_TICKS {
                self.pulse = None;
            }
        }
        match self.phase {
            Phase::Ready(n) => {
                self.remember_input(input);
                self.phase = if n <= 1 {
                    Phase::Playing
                } else {
                    Phase::Ready(n - 1)
                };
            }
            Phase::Playing => {
                self.t += 1;
                self.remember_input(input);
                self.update_player(input);
                self.update_cops();
                self.check_contacts();
            }
            Phase::Dying(n) => {
                if n <= 1 {
                    if self.lives == 0 {
                        self.phase = Phase::GameOver;
                        self.events.push(Event::GameOver);
                    } else {
                        self.reset_actors();
                        self.phase = Phase::Ready(READY_TICKS);
                    }
                } else {
                    self.phase = Phase::Dying(n - 1);
                }
            }
            Phase::Cleared(n) => {
                if n > 0 {
                    self.phase = Phase::Cleared(n - 1);
                }
            }
            Phase::GameOver => {}
        }
        self.prev_action = input.action;
    }

    /// True once the level-cleared pause is over and the next level can start.
    pub fn ready_for_next_level(&self) -> bool {
        matches!(self.phase, Phase::Cleared(0))
    }

    fn remember_input(&mut self, input: Input) {
        if let Some(d) = input.dir {
            self.desired = Some(d);
        } else if self.settings.stop_on_release {
            self.desired = None;
        }
    }

    fn player_can_enter(level: &Level, gate_open: bool, p: Pos) -> bool {
        match level.tile(p) {
            Tile::Floor => true,
            Tile::Gate => gate_open,
            _ => false,
        }
    }

    fn update_player(&mut self, input: Input) {
        if input.action && !self.prev_action {
            self.use_joint();
        }
        if let (Some(cur), Some(want)) = (self.player.dir, self.desired) {
            if want == cur.opposite() {
                self.player.reverse();
            }
        }
        let speed = self.player_speed();
        let level = &self.level;
        let gate_open = self.gate_open;
        let desired = self.desired;
        let stop = self.settings.stop_on_release;
        let arrived = self.player.advance(speed, |tile, heading| {
            if let Some(d) = desired {
                if Self::player_can_enter(level, gate_open, step(tile, d)) {
                    return Some(d);
                }
            }
            if stop && desired.is_none() {
                return None;
            }
            heading.filter(|&h| Self::player_can_enter(level, gate_open, step(tile, h)))
        });
        if let Some(tile) = arrived {
            self.on_player_arrive(tile);
        }
    }

    fn on_player_arrive(&mut self, tile: Pos) {
        let (x, y) = (tile.0 as usize, tile.1 as usize);
        if self.leaves[y][x] {
            self.leaves[y][x] = false;
            self.leaves_left -= 1;
            self.events.push(Event::LeafEaten);
            self.add_score(LEAF_POINTS);
            if self.leaves_left == 0 {
                self.gate_open = true;
                self.events.push(Event::GateOpened);
            }
        }
        if self.joint_tiles[y][x] {
            self.joint_tiles[y][x] = false;
            self.joints = (self.joints + 1).min(MAX_JOINTS);
            self.events.push(Event::JointPicked);
            self.add_score(SPARKLER_POINTS);
        }
        if tile == self.level.exit && self.gate_open {
            let secs = self.elapsed_seconds() as u32;
            let time_bonus = self.level.par_seconds.saturating_sub(secs) * 10;
            self.add_score(time_bonus);
            self.events.push(Event::LevelCleared { time_bonus });
            self.phase = Phase::Cleared(CLEARED_TICKS);
        }
    }

    fn use_joint(&mut self) {
        if self.joints == 0 {
            return;
        }
        let (px, py) = self.player.pos_f();
        let in_range = |w: &Cop| {
            if matches!(w.state, CopState::Waiting { .. }) {
                return false;
            }
            let (wx, wy) = w.mover.pos_f();
            ((wx - px).powi(2) + (wy - py).powi(2)).sqrt() <= STUN_RADIUS
        };
        if !self.cops.iter().any(in_range) {
            self.events.push(Event::NothingToStun);
            return;
        }
        self.joints -= 1;
        self.chain = 0;
        let until = self.t + stun_ticks(self.level_index);
        for w in self.cops.iter_mut() {
            let (wx, wy) = w.mover.pos_f();
            let near = ((wx - px).powi(2) + (wy - py).powi(2)).sqrt() <= STUN_RADIUS;
            if near && !matches!(w.state, CopState::Waiting { .. }) {
                w.state = CopState::Stunned { until };
            }
        }
        self.pulse = Some(Pulse {
            center: (px, py),
            age: 0,
        });
        self.events.push(Event::JointUsed);
    }

    /// Pac-Man style mode timer: scatter 7s, chase 20s, scatter 7s, chase 20s, scatter 5s, then
    /// chase for good. Cops spend later levels scattering for less time.
    fn scattering(&self) -> bool {
        let secs = self.elapsed_seconds();
        let short = if self.level_index >= 4 { 2.0 } else { 0.0 };
        let mut t = 0.0;
        for (scatter, len) in [
            (true, 7.0 - short),
            (false, 20.0),
            (true, 7.0 - short),
            (false, 20.0),
            (true, 5.0 - short),
        ] {
            t += len;
            if secs < t {
                return scatter;
            }
        }
        false
    }

    fn update_cops(&mut self) {
        let speed = self.cop_speed();
        let player = PlayerView {
            tile: self.player.nearest_tile(),
            heading: self.player.dir.or(self.desired),
        };
        let hunter = self
            .cops
            .first()
            .map_or(player.tile, |w| w.mover.nearest_tile());
        let scatter = self.scattering();
        // Like the ghosts, cops turn round when the mode changes, which is your cue.
        let flipped = scatter != self.was_scattering;
        self.was_scattering = scatter;
        for i in 0..self.cops.len() {
            let mut w = self.cops[i].clone();
            match w.state {
                CopState::Waiting { release_at } if self.t >= release_at => {
                    w.state = CopState::Hunting;
                    self.events.push(Event::CopReleased);
                }
                CopState::Stunned { until } if self.t >= until => w.state = CopState::Hunting,
                _ => {}
            }
            if w.state == CopState::Hunting {
                let level = &self.level;
                if flipped && matches!(level.tile(w.mover.tile), Tile::Floor) {
                    w.mover.reverse();
                }
                let kind = w.kind;
                w.mover.advance(speed, |tile, heading| {
                    let target = ai::pick_target(kind, tile, hunter, player, scatter);
                    ai::choose_direction(level, tile, heading, target)
                });
            }
            self.cops[i] = w;
        }
    }

    fn check_contacts(&mut self) {
        let (px, py) = self.player.pos_f();
        for i in 0..self.cops.len() {
            let w = &self.cops[i];
            if matches!(w.state, CopState::Waiting { .. }) {
                continue;
            }
            let (wx, wy) = w.mover.pos_f();
            if ((wx - px).powi(2) + (wy - py).powi(2)).sqrt() >= CATCH_DISTANCE {
                continue;
            }
            if matches!(w.state, CopState::Stunned { .. }) {
                let points = 100u32 << self.chain.min(3);
                self.chain += 1;
                self.add_score(points);
                self.events.push(Event::CopBusted(points));
                let back = self.t + 4 * TICKS_PER_SEC;
                let w = &mut self.cops[i];
                w.mover = Mover::new(w.pen_cell);
                w.state = CopState::Waiting { release_at: back };
            } else if !self.invulnerable {
                self.lives = self.lives.saturating_sub(1);
                self.phase = Phase::Dying(DYING_TICKS);
                self.events.push(Event::PlayerCaught);
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(dir: Option<Dir>) -> Input {
        Input { dir, action: false }
    }

    fn skip_ready(w: &mut World) {
        for _ in 0..READY_TICKS + 2 {
            w.tick(Input::default());
        }
        assert_eq!(w.phase, Phase::Playing);
    }

    /// A world with the cops parked in the pen, so movement/scoring can be tested in peace.
    fn calm_world(level: usize) -> World {
        let mut w = World::new(level, Settings::default(), None);
        for ward in &mut w.cops {
            ward.state = CopState::Waiting {
                release_at: u64::MAX,
            };
        }
        skip_ready(&mut w);
        w
    }

    #[test]
    fn starts_in_ready_phase_then_plays() {
        let mut w = World::new(0, Settings::default(), None);
        assert!(matches!(w.phase, Phase::Ready(_)));
        let before = w.player.pos_f();
        w.tick(input(Some(Dir::Left)));
        assert_eq!(w.player.pos_f(), before, "nobody moves during READY");
        skip_ready(&mut w);
    }

    #[test]
    fn walls_block_the_player() {
        let mut w = calm_world(0);
        // Seedling's start (8,9) has a wall to the south-east region; push up into the wall at (8,8).
        let start = w.level.start;
        for _ in 0..60 {
            w.tick(input(Some(Dir::Up)));
        }
        let (x, y) = w.player.pos_f();
        assert!(
            y >= 8.99 || w.level.tile((8, 8)) != Tile::Wall,
            "walked into a wall: {x},{y}"
        );
        assert_eq!(start, (8, 9));
    }

    #[test]
    fn collecting_leaves_scores_points() {
        let mut w = calm_world(0);
        let before = (w.score, w.leaves_left);
        for _ in 0..120 {
            w.tick(input(Some(Dir::Left)));
        }
        assert!(
            w.score > before.0,
            "ate nothing walking along a leaf corridor"
        );
        assert!(w.leaves_left < before.1);
        assert_eq!(w.score % LEAF_POINTS, 0);
    }

    #[test]
    fn continuous_movement_keeps_going() {
        let mut w = calm_world(0);
        w.tick(input(Some(Dir::Left)));
        let a = w.player.pos_f();
        for _ in 0..20 {
            w.tick(Input::default());
        }
        assert!(
            w.player.pos_f().0 < a.0,
            "should keep moving left with no keys held"
        );
    }

    #[test]
    fn stop_on_release_stops_at_a_tile() {
        let mut w = World::new(
            0,
            Settings {
                stop_on_release: true,
                ..Settings::default()
            },
            None,
        );
        for ward in &mut w.cops {
            ward.state = CopState::Waiting {
                release_at: u64::MAX,
            };
        }
        skip_ready(&mut w);
        w.tick(input(Some(Dir::Left)));
        for _ in 0..60 {
            w.tick(Input::default());
        }
        let (x, _) = w.player.pos_f();
        assert!(
            (x - x.round()).abs() < 1e-3,
            "should rest exactly on a tile, at {x}"
        );
    }

    #[test]
    fn can_reverse_mid_corridor() {
        let mut w = calm_world(0);
        for _ in 0..6 {
            w.tick(input(Some(Dir::Left)));
        }
        let x1 = w.player.pos_f().0;
        for _ in 0..4 {
            w.tick(input(Some(Dir::Right)));
        }
        assert!(
            w.player.pos_f().0 > x1 - 1e-3 + 0.0 || w.player.pos_f().0 != x1,
            "did not turn around"
        );
    }

    #[test]
    fn gate_opens_when_the_last_leaf_is_eaten() {
        let mut w = calm_world(0);
        w.leaves = [[false; W]; H];
        let (sx, sy) = (w.level.start.0 as usize, w.level.start.1 as usize);
        w.leaves[sy][sx - 1] = true;
        w.leaves_left = 1;
        for _ in 0..40 {
            w.tick(input(Some(Dir::Left)));
            if w.gate_open {
                break;
            }
        }
        assert!(w.gate_open);
        assert_eq!(w.leaves_left, 0);
    }

    #[test]
    fn the_closed_gate_blocks_and_the_open_gate_clears_the_level() {
        let mut w = calm_world(0);
        assert!(!World::player_can_enter(&w.level, false, w.level.exit));
        assert!(World::player_can_enter(&w.level, true, w.level.exit));
        // Teleport next to the gate, open it, and walk in.
        w.leaves = [[false; W]; H];
        w.leaves_left = 0;
        w.gate_open = true;
        w.player = Mover::new((w.level.exit.0, w.level.exit.1 + 1));
        w.desired = Some(Dir::Up);
        for _ in 0..40 {
            w.tick(input(Some(Dir::Up)));
        }
        assert!(matches!(w.phase, Phase::Cleared(_)), "phase {:?}", w.phase);
    }

    #[test]
    fn a_cop_touching_you_costs_a_life() {
        let mut w = calm_world(0);
        let lives = w.lives;
        w.cops[0].state = CopState::Hunting;
        w.cops[0].mover = Mover::new(w.player.tile);
        w.tick(Input::default());
        assert_eq!(w.lives, lives - 1);
        assert!(matches!(w.phase, Phase::Dying(_)));
        for _ in 0..DYING_TICKS + 2 {
            w.tick(Input::default());
        }
        assert!(
            matches!(w.phase, Phase::Ready(_)),
            "respawns when lives remain: {:?}",
            w.phase
        );
        assert_eq!(w.player.tile, w.level.start);
    }

    #[test]
    fn losing_the_last_life_is_game_over() {
        let mut w = World::new(
            0,
            Settings {
                lives: 1,
                ..Settings::default()
            },
            None,
        );
        skip_ready(&mut w);
        for ward in &mut w.cops {
            ward.state = CopState::Waiting {
                release_at: u64::MAX,
            };
        }
        w.cops[0].state = CopState::Hunting;
        w.cops[0].mover = Mover::new(w.player.tile);
        w.tick(Input::default());
        for _ in 0..DYING_TICKS + 2 {
            w.tick(Input::default());
        }
        assert_eq!(w.phase, Phase::GameOver);
    }

    #[test]
    fn a_joint_stuns_nearby_cops_and_touching_them_busts_them() {
        let mut w = calm_world(0);
        w.joints = 1;
        let near = (w.player.tile.0 + 2, w.player.tile.1);
        w.cops[0].state = CopState::Hunting;
        w.cops[0].mover = Mover::new(near);
        w.tick(Input {
            dir: None,
            action: true,
        });
        assert!(matches!(w.cops[0].state, CopState::Stunned { .. }));
        assert_eq!(w.joints, 0);
        assert!(w.events.contains(&Event::JointUsed));
        // Bring the stunned cop onto the player: busted, not caught.
        let lives = w.lives;
        w.cops[0].mover = Mover::new(w.player.tile);
        w.tick(Input::default());
        assert_eq!(w.lives, lives);
        assert!(w.events.iter().any(|e| matches!(e, Event::CopBusted(100))));
        assert!(matches!(w.cops[0].state, CopState::Waiting { .. }));
    }

    #[test]
    fn a_joint_is_not_wasted_when_nobody_is_near() {
        let mut w = calm_world(0);
        w.joints = 2;
        w.tick(Input {
            dir: None,
            action: true,
        });
        assert_eq!(w.joints, 2);
        assert!(w.events.contains(&Event::NothingToStun));
    }

    #[test]
    fn being_smoked_out_lasts_ten_seconds_at_first_and_never_under_six() {
        assert_eq!(stun_ticks(0), 10 * TICKS_PER_SEC);
        assert_eq!(stun_ticks(4), 8 * TICKS_PER_SEC);
        assert_eq!(stun_ticks(30), 6 * TICKS_PER_SEC);
        let mut w = calm_world(0);
        w.joints = 1;
        w.cops[0].state = CopState::Hunting;
        w.cops[0].mover = Mover::new((w.player.tile.0 + 3, w.player.tile.1));
        assert!(w.smoked_out().is_none());
        w.tick(Input {
            dir: None,
            action: true,
        });
        let (left, total) = w.smoked_out().expect("a cop is smoked out");
        assert_eq!(total, stun_ticks(0));
        assert!(left > total - 3);
    }

    #[test]
    fn stunned_cops_wake_up_again() {
        let mut w = calm_world(0);
        w.joints = 1;
        w.cops[0].state = CopState::Hunting;
        w.cops[0].mover = Mover::new((w.player.tile.0 + 3, w.player.tile.1));
        w.tick(Input {
            dir: None,
            action: true,
        });
        for _ in 0..stun_ticks(0) + 5 {
            w.tick(Input::default());
            if matches!(w.phase, Phase::Dying(_)) {
                return; // woke up and caught us: also proves it woke
            }
        }
        assert!(matches!(w.cops[0].state, CopState::Hunting));
    }

    #[test]
    fn bust_points_chain_upwards() {
        let mut w = calm_world(0);
        w.chain = 0;
        for expected in [100u32, 200, 400, 800, 800] {
            w.cops[0].state = CopState::Stunned { until: u64::MAX };
            w.cops[0].mover = Mover::new(w.player.tile);
            w.tick(Input::default());
            assert!(
                w.events.contains(&Event::CopBusted(expected)),
                "expected {expected}: {:?}",
                w.events
            );
        }
    }

    #[test]
    fn extra_life_every_five_thousand() {
        let mut w = calm_world(0);
        let lives = w.lives;
        w.add_score(4990);
        assert_eq!(w.lives, lives);
        w.add_score(20);
        assert_eq!(w.lives, lives + 1);
        assert!(w.events.contains(&Event::ExtraLife));
    }

    #[test]
    fn the_whole_simulation_is_deterministic() {
        let run = || {
            let mut w = World::new(2, Settings::default(), None);
            let mut sig = Vec::new();
            for i in 0..1500 {
                let dir = [Dir::Left, Dir::Up, Dir::Right, Dir::Down][(i / 40) % 4];
                w.tick(Input {
                    dir: Some(dir),
                    action: i % 300 == 299,
                });
                sig.push((
                    w.score,
                    w.lives,
                    w.player.tile,
                    w.cops.iter().map(|x| x.mover.tile).collect::<Vec<_>>(),
                ));
            }
            sig
        };
        assert_eq!(run(), run());
    }

    /// The headline requirement: officers never stand around. Whenever a cop is hunting,
    /// it must keep moving (no long pauses) and cover ground at close to its full speed.
    #[test]
    fn cops_never_idle() {
        for level in [0usize, 1, 2, 3, 6, 11] {
            let mut w = World::new(
                level,
                Settings {
                    lives: 100_000,
                    ..Settings::default()
                },
                None,
            );
            w.invulnerable = true;
            let n = w.cops.len();
            let mut prev_pos: Vec<Option<(f32, f32)>> = vec![None; n];
            let mut still_for = vec![0u32; n];
            let mut hunting_ticks = vec![0u64; n];
            let mut distance = vec![0f32; n];
            let mut playing = 0u64;
            for i in 0..60 * 150u64 {
                // The player wanders so cops have something to chase.
                let dir = [Dir::Left, Dir::Up, Dir::Right, Dir::Down][((i / 70) % 4) as usize];
                w.tick(Input {
                    dir: Some(dir),
                    action: false,
                });
                if w.phase != Phase::Playing {
                    prev_pos.iter_mut().for_each(|p| *p = None);
                    still_for.iter_mut().for_each(|c| *c = 0);
                    continue;
                }
                playing += 1;
                for (idx, ward) in w.cops.iter().enumerate() {
                    if ward.state != CopState::Hunting {
                        prev_pos[idx] = None;
                        still_for[idx] = 0;
                        continue;
                    }
                    let pos = ward.mover.pos_f();
                    if let Some(prev) = prev_pos[idx] {
                        let moved = ((pos.0 - prev.0).powi(2) + (pos.1 - prev.1).powi(2)).sqrt();
                        hunting_ticks[idx] += 1;
                        distance[idx] += moved;
                        if moved < 1e-4 {
                            still_for[idx] += 1;
                        } else {
                            still_for[idx] = 0;
                        }
                        assert!(
                            still_for[idx] < 10,
                            "level {level}: cop {idx} stood still for {} ticks at {:?}",
                            still_for[idx],
                            ward.mover.tile
                        );
                    }
                    prev_pos[idx] = Some(pos);
                }
            }
            assert!(
                playing > 1000,
                "level {level}: the test barely ran ({playing} playing ticks)"
            );
            let speed = PLAYER_TILES_PER_SEC / TICKS_PER_SEC as f32 * w.level.cop_speed;
            for idx in 0..n {
                assert!(
                    hunting_ticks[idx] > 200,
                    "level {level}: cop {idx} barely hunted ({})",
                    hunting_ticks[idx]
                );
                let expected = speed * hunting_ticks[idx] as f32;
                assert!(
                    distance[idx] >= expected * 0.9,
                    "level {level}: cop {idx} covered {:.1} tiles but should have covered about {:.1}",
                    distance[idx],
                    expected
                );
            }
        }
    }

    #[test]
    fn every_cop_leaves_the_pen_soon() {
        for level in [0usize, 5, 20] {
            let mut w = World::new(level, Settings::default(), None);
            for _ in 0..60 * 12 {
                w.tick(Input::default());
                if matches!(w.phase, Phase::Dying(_) | Phase::GameOver) {
                    break;
                }
            }
            if matches!(w.phase, Phase::Playing) {
                assert!(w
                    .cops
                    .iter()
                    .all(|x| x.state != CopState::Waiting { release_at: 0 }));
                assert!(
                    w.cops
                        .iter()
                        .all(|x| !matches!(x.state, CopState::Waiting { .. })),
                    "someone is still in the pen after 12 s"
                );
            }
        }
    }

    #[test]
    fn the_autopilot_can_clear_every_early_level() {
        // With the cops parked the autopilot must finish: this proves each level is completable
        // end to end, with real movement, real corners and the real gate.
        for level in [0usize, 1, 2, 3, 4, 9, 15] {
            let mut w = calm_world(level);
            let mut bot = crate::bot::Autopilot::new();
            let mut guard = 0;
            while !matches!(w.phase, Phase::Cleared(_)) && guard < 60 * 400 {
                guard += 1;
                let dir = bot.decide(&w);
                w.tick(Input { dir, action: false });
            }
            assert!(
                matches!(w.phase, Phase::Cleared(_)),
                "level {level}: autopilot stuck after {guard} ticks, {} leaves left; player at {:?} dir {:?} progress {}",
                w.leaves_left,
                w.player.tile,
                w.player.dir,
                w.player.progress
            );
            assert_eq!(w.leaves_left, 0);
        }
    }
}

#[cfg(test)]
mod demo_quality {
    use super::*;

    /// The demo autopilot plays for real (the cops can catch it); it should still put on a show.
    fn play(level: usize, cautious: bool, joints: bool) -> (u32, u32, f32) {
        let mut w = World::new(level, Settings::default(), None);
        let mut bot = if cautious {
            crate::bot::Autopilot::cautious()
        } else {
            crate::bot::Autopilot::new()
        };
        let total = w.level.leaf_count() as u32;
        let mut cleared = 0;
        for _ in 0..60 * 240 {
            if w.phase == Phase::GameOver {
                break;
            }
            if matches!(w.phase, Phase::Cleared(_)) {
                cleared = 1;
                break;
            }
            let near = w.cops.iter().any(|c| {
                let (cx, cy) = c.mover.pos_f();
                let (px, py) = w.player.pos_f();
                ((cx - px).powi(2) + (cy - py).powi(2)).sqrt() < 2.2
            });
            let dir = bot.decide(&w);
            w.tick(Input {
                dir,
                action: joints && near && w.t % 7 == 0,
            });
        }
        (cleared, total - w.leaves_left, w.elapsed_seconds())
    }

    #[test]
    fn the_demo_clears_the_first_level_against_real_cops() {
        assert_eq!(play(0, true, true).0, 1);
    }

    /// `cargo test --release -- --ignored --nocapture report` prints how the demo fares.
    #[test]
    #[ignore]
    fn report() {
        for level in 0..10 {
            let a = play(level, false, true);
            let b = play(level, true, true);
            println!("level {level}: plain cleared={} leaves={} secs={:.0} | cautious cleared={} leaves={} secs={:.0}", a.0, a.1, a.2, b.0, b.1, b.2);
        }
    }
}
