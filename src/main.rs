//! Giddy Ganja: window, menus and the main loop. The rules live in the library crate.

mod render;
mod sfx;

use giddy_ganja::bot::Autopilot;
use giddy_ganja::game::*;
use giddy_ganja::grid::Dir;
use giddy_ganja::levels;
use giddy_ganja::storage::{self, Config, HighScores, SPEEDS};
use macroquad::prelude::*;
use render::*;
use sfx::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "Giddy Ganja".to_string(),
        window_width: 960,
        window_height: 675,
        window_resizable: true,
        high_dpi: true,
        sample_count: 4,
        ..Default::default()
    }
}

const TICK: f32 = 1.0 / TICKS_PER_SEC as f32;
const IS_WEB: bool = cfg!(target_arch = "wasm32");
const LEVEL_CHOICES: usize = 24;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Title,
    Play,
    Paused,
    Options,
    LevelSelect,
    Scores,
    Help,
    EnterName,
}

struct Play {
    world: World,
    fx: Effects,
    leaf_toggle: bool,
    announced_ready: bool,
    game_over_timer: f32,
    /// Demo mode: the autopilot plays and you just watch (press any key to stop).
    bot: Option<Autopilot>,
}

/// Keyboard and touch input, turned into what the simulation wants.
#[derive(Default)]
struct Controls {
    /// Directions currently held, oldest first; the newest one wins.
    held: Vec<Dir>,
    action_latch: bool,
    swipe: Option<Dir>,
    touch_start: Option<Vec2>,
    swipe_used: bool,
}

const DIR_KEYS: [(KeyCode, Dir); 8] = [
    (KeyCode::Up, Dir::Up),
    (KeyCode::W, Dir::Up),
    (KeyCode::Down, Dir::Down),
    (KeyCode::S, Dir::Down),
    (KeyCode::Left, Dir::Left),
    (KeyCode::A, Dir::Left),
    (KeyCode::Right, Dir::Right),
    (KeyCode::D, Dir::Right),
];

impl Controls {
    fn poll(&mut self) {
        for (key, dir) in DIR_KEYS {
            if is_key_pressed(key) {
                self.held.retain(|d| *d != dir);
                self.held.push(dir);
            }
        }
        self.held
            .retain(|d| DIR_KEYS.iter().any(|(k, dd)| dd == d && is_key_down(*k)));
        if is_key_pressed(KeyCode::Space)
            || is_key_pressed(KeyCode::Z)
            || is_key_pressed(KeyCode::LeftShift)
        {
            self.action_latch = true;
        }
        // Touch: a swipe sets a direction, a two-finger tap uses a joint.
        let touches = touches();
        if touches.len() >= 2 && touches.iter().any(|t| t.phase == TouchPhase::Started) {
            self.action_latch = true;
        }
        for t in &touches {
            match t.phase {
                TouchPhase::Started => {
                    self.touch_start = Some(t.position);
                    self.swipe_used = false;
                }
                TouchPhase::Moved => {
                    if let (Some(start), false) = (self.touch_start, self.swipe_used) {
                        let d = t.position - start;
                        if d.length() > 24.0 {
                            self.swipe = Some(if d.x.abs() > d.y.abs() {
                                if d.x > 0.0 {
                                    Dir::Right
                                } else {
                                    Dir::Left
                                }
                            } else if d.y > 0.0 {
                                Dir::Down
                            } else {
                                Dir::Up
                            });
                            self.touch_start = Some(t.position);
                        }
                    }
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    self.touch_start = None;
                }
                _ => {}
            }
        }
    }

    fn input(&mut self) -> Input {
        let dir = self.held.last().copied().or_else(|| self.swipe.take());
        let action = self.action_latch || is_key_down(KeyCode::Space) || is_key_down(KeyCode::Z);
        Input { dir, action }
    }

    fn end_tick(&mut self) {
        self.action_latch = false;
    }
}

struct App {
    screen: Screen,
    menu: usize,
    config: Config,
    scores: HighScores,
    sounds: Sounds,
    play: Option<Play>,
    controls: Controls,
    start_level: usize,
    name: String,
    last_score: (u32, usize),
    new_rank: Option<usize>,
    acc: f32,
    scale: f32,
    origin: Vec2,
    quit: bool,
}

impl App {
    fn pointer(&self) -> Vec2 {
        let (mx, my) = mouse_position();
        vec2(
            self.origin.x + mx / self.scale,
            self.origin.y + my / self.scale,
        )
    }

    fn clicked(&self) -> bool {
        is_mouse_button_pressed(MouseButton::Left)
            || touches().iter().any(|t| t.phase == TouchPhase::Started)
    }

    fn settings(&self) -> Settings {
        Settings {
            speed: self.config.speed_factor(),
            stop_on_release: self.config.stop_on_release,
            lives: 3,
        }
    }

    fn save_config(&self) {
        storage::save_config(&self.config);
    }

    fn start_game(&mut self, level: usize) {
        let world = World::new(level, self.settings(), None);
        self.play = Some(Play {
            world,
            fx: Effects::default(),
            leaf_toggle: false,
            announced_ready: false,
            game_over_timer: 0.0,
            bot: None,
        });
        self.screen = Screen::Play;
        self.acc = 0.0;
        self.controls = Controls::default();
    }

    fn start_demo(&mut self, level: usize) {
        self.start_game(level);
        if let Some(p) = &mut self.play {
            p.world.invulnerable = true;
            p.bot = Some(Autopilot::new());
        }
    }

    /// Draws a vertical list of choices and returns the one chosen (Enter, Space or a click).
    fn menu(
        &mut self,
        items: &[String],
        cx: f32,
        y0: f32,
        spacing: f32,
        size: f32,
    ) -> Option<usize> {
        let n = items.len();
        if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S) {
            self.menu = (self.menu + 1) % n;
            self.sounds.play(Sfx::MenuMove);
        }
        if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W) {
            self.menu = (self.menu + n - 1) % n;
            self.sounds.play(Sfx::MenuMove);
        }
        let mut chosen = None;
        let pointer = self.pointer();
        let moved = mouse_delta_position().length() > 0.0001;
        for (i, item) in items.iter().enumerate() {
            let y = y0 + i as f32 * spacing;
            let w = measure_text(item, None, size as u16, 1.0).width.max(160.0);
            let hit = pointer.x > cx - w / 2.0 - 12.0
                && pointer.x < cx + w / 2.0 + 12.0
                && pointer.y > y - size
                && pointer.y < y + size * 0.4;
            if hit && moved && self.menu != i {
                self.menu = i;
                self.sounds.play(Sfx::MenuMove);
            }
            if hit && self.clicked() {
                self.menu = i;
                chosen = Some(i);
            }
            let selected = self.menu == i;
            if classic() {
                // White push-buttons with black text; the chosen one gets a heavy black frame.
                let (bx, by, bw, bh) =
                    (cx - w / 2.0 - 14.0, y - size * 0.85, w + 28.0, size * 1.15);
                draw_rectangle(bx, by, bw, bh, WHITE);
                draw_rectangle_lines(bx, by, bw, bh, if selected { 4.0 } else { 1.5 }, BLACK);
                if selected {
                    draw_rectangle_lines(bx - 4.0, by - 4.0, bw + 8.0, bh + 8.0, 3.0, c_green());
                }
                let tw = measure_text(item, None, size as u16, 1.0).width;
                draw_text(item, cx - tw / 2.0, y, size, BLACK);
            } else {
                let color = if selected { title_green() } else { dim() };
                if selected {
                    draw_rectangle(
                        cx - w / 2.0 - 14.0,
                        y - size * 0.85,
                        w + 28.0,
                        size * 1.15,
                        Color::new(0.1, 0.3, 0.15, 0.55),
                    );
                    centered(">", cx - w / 2.0 - 4.0, y, size, accent_for(0));
                }
                centered(item, cx, y, size, color);
            }
        }
        if is_key_pressed(KeyCode::Enter)
            || is_key_pressed(KeyCode::Space)
            || is_key_pressed(KeyCode::KpEnter)
        {
            chosen = Some(self.menu.min(n - 1));
        }
        if chosen.is_some() {
            self.sounds.play(Sfx::MenuSelect);
        }
        chosen
    }

    fn backdrop(&self, time: f64) {
        draw_rectangle(0.0, 0.0, LW, LH, menu_bg());
        if classic() {
            return;
        }
        for i in 0..14 {
            let t = time as f32 * 0.25 + i as f32 * 0.37;
            let x = ((t * 53.0 + i as f32 * 71.0) % (LW + 60.0)) - 30.0;
            let y = ((i as f32 * 97.0 + t * 38.0) % (LH + 60.0)) - 30.0;
            draw_circle(
                x,
                y,
                18.0 + (i % 4) as f32 * 7.0,
                Color::new(0.1, 0.35, 0.18, 0.10),
            );
        }
    }

    fn title(&mut self, time: f64) {
        self.backdrop(time);
        draw_title_art(LW / 2.0, 92.0, time);
        centered("GIDDY GANJA", LW / 2.0 + 60.0, 96.0, 64.0, title_green());
        centered(
            "gather every bud  -  dodge the cops  -  slip out the exit",
            LW / 2.0,
            136.0,
            18.0,
            dim(),
        );
        let mut items = vec![
            "Play",
            "Level Select",
            "Options",
            "High Scores",
            "How to Play",
        ];
        if !IS_WEB {
            items.push("Quit");
        }
        let items: Vec<String> = items.into_iter().map(String::from).collect();
        if let Some(i) = self.menu(&items, LW / 2.0, 214.0, 38.0, 26.0) {
            match items[i].as_str() {
                "Play" => self.start_game(0),
                "Level Select" => {
                    self.screen = Screen::LevelSelect;
                    self.menu = self.start_level.min(LEVEL_CHOICES - 1);
                }
                "Options" => {
                    self.screen = Screen::Options;
                    self.menu = 0;
                }
                "High Scores" => {
                    self.new_rank = None;
                    self.screen = Screen::Scores;
                }
                "How to Play" => self.screen = Screen::Help,
                _ => self.quit = true,
            }
        }
        centered(
            "Arrow keys or WASD to move  -  Space uses a joint  -  P pauses  -  F11 fullscreen",
            LW / 2.0,
            LH - 28.0,
            15.0,
            dim(),
        );
    }

    fn options(&mut self, time: f64) {
        self.backdrop(time);
        centered("OPTIONS", LW / 2.0, 70.0, 44.0, title_green());
        let items: Vec<String> = vec![
            format!("Speed:  < {} >", SPEEDS[self.config.speed].0),
            format!("Volume:  < {} >", self.config.volume),
            format!(
                "Movement:  < {} >",
                if self.config.stop_on_release {
                    "Stop when keys released"
                } else {
                    "Keep going"
                }
            ),
            format!(
                "Fullscreen:  < {} >",
                if self.config.fullscreen { "On" } else { "Off" }
            ),
            format!(
                "Look:  < {} >",
                if self.config.classic {
                    "Classic"
                } else {
                    "Modern"
                }
            ),
            "Back".to_string(),
        ];
        let left = is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::A);
        let right = is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::D);
        let step: i32 = if right {
            1
        } else if left {
            -1
        } else {
            0
        };
        let activated = self.menu(&items, LW / 2.0, 140.0, 44.0, 22.0);
        let step = if activated.is_some() && step == 0 {
            1
        } else {
            step
        };
        let mut changed = false;
        if step != 0 {
            match self.menu {
                0 => {
                    self.config.speed =
                        (self.config.speed as i32 + step).rem_euclid(SPEEDS.len() as i32) as usize;
                    changed = true;
                }
                1 => {
                    self.config.volume = (self.config.volume as i32 + step).clamp(0, 7) as u8;
                    self.sounds.volume = self.config.volume;
                    self.sounds.play(Sfx::MenuSelect);
                    changed = true;
                }
                2 => {
                    self.config.stop_on_release = !self.config.stop_on_release;
                    changed = true;
                }
                3 => {
                    self.config.fullscreen = !self.config.fullscreen;
                    set_fullscreen(self.config.fullscreen);
                    changed = true;
                }
                4 => {
                    self.config.classic = !self.config.classic;
                    set_classic(self.config.classic);
                    changed = true;
                }
                _ => {
                    if activated.is_some() {
                        self.screen = Screen::Title;
                        self.menu = 2;
                    }
                }
            }
        }
        if changed {
            self.save_config();
        }
        if is_key_pressed(KeyCode::Escape) {
            self.screen = Screen::Title;
            self.menu = 2;
        }
        centered(
            "Left / Right changes a setting  -  Esc goes back",
            LW / 2.0,
            LH - 28.0,
            15.0,
            dim(),
        );
    }

    fn level_select(&mut self, time: f64) {
        self.backdrop(time);
        centered("LEVEL SELECT", LW / 2.0, 56.0, 40.0, title_green());
        // A grid of level numbers.
        let cols = 6;
        let pointer = self.pointer();
        let mut chosen = None;
        if is_key_pressed(KeyCode::Right) {
            self.menu = (self.menu + 1) % LEVEL_CHOICES;
            self.sounds.play(Sfx::MenuMove);
        }
        if is_key_pressed(KeyCode::Left) {
            self.menu = (self.menu + LEVEL_CHOICES - 1) % LEVEL_CHOICES;
            self.sounds.play(Sfx::MenuMove);
        }
        if is_key_pressed(KeyCode::Down) {
            self.menu = (self.menu + cols) % LEVEL_CHOICES;
            self.sounds.play(Sfx::MenuMove);
        }
        if is_key_pressed(KeyCode::Up) {
            self.menu = (self.menu + LEVEL_CHOICES - cols) % LEVEL_CHOICES;
            self.sounds.play(Sfx::MenuMove);
        }
        for i in 0..LEVEL_CHOICES {
            let x = 70.0 + (i % cols) as f32 * 86.0;
            let y = 96.0 + (i / cols) as f32 * 62.0;
            let hit =
                pointer.x > x && pointer.x < x + 76.0 && pointer.y > y && pointer.y < y + 52.0;
            if hit && self.clicked() {
                self.menu = i;
                chosen = Some(i);
            }
            let pal = palette(i);
            let selected = self.menu == i;
            draw_rectangle(
                x,
                y,
                76.0,
                52.0,
                if selected {
                    pal.wall_mid
                } else {
                    pal.wall_dark
                },
            );
            if selected {
                draw_rectangle_lines(x - 2.0, y - 2.0, 80.0, 56.0, 3.0, WHITE);
            }
            centered(&format!("{}", i + 1), x + 38.0, y + 26.0, 26.0, WHITE);
            centered(
                &levels::level_name(i),
                x + 38.0,
                y + 45.0,
                12.0,
                pal.wall_light,
            );
        }
        if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space) {
            chosen = Some(self.menu);
        }
        if let Some(i) = chosen {
            self.sounds.play(Sfx::MenuSelect);
            self.start_level = i;
            self.start_game(i);
        }
        if is_key_pressed(KeyCode::Escape) {
            self.screen = Screen::Title;
            self.menu = 1;
        }
        centered(
            "Arrow keys to choose  -  Enter to play  -  Esc goes back",
            LW / 2.0,
            LH - 28.0,
            15.0,
            dim(),
        );
    }

    fn scores_screen(&mut self, time: f64) {
        self.backdrop(time);
        centered("HIGH SCORES", LW / 2.0, 62.0, 44.0, title_green());
        if self.scores.entries.is_empty() {
            centered(
                "No scores yet. Go get stoned, dude!",
                LW / 2.0,
                200.0,
                22.0,
                dim(),
            );
        }
        for (i, e) in self.scores.entries.iter().enumerate() {
            let y = 110.0 + i as f32 * 30.0;
            let hot = self.new_rank == Some(i);
            let color = if hot {
                accent_for(0)
            } else if i == 0 {
                bright()
            } else {
                dim()
            };
            if hot {
                draw_rectangle(150.0, y - 22.0, 340.0, 28.0, Color::new(0.2, 0.4, 0.2, 0.5));
            }
            draw_text(format!("{:>2}.", i + 1), 160.0, y, 24.0, color);
            draw_text(&e.name, 210.0, y, 24.0, color);
            draw_text(format!("{}", e.score), 320.0, y, 24.0, color);
            draw_text(format!("lvl {}", e.level + 1), 430.0, y, 20.0, color);
        }
        if let Some(dir) = storage::data_dir() {
            centered(
                &format!("Saved in {}", dir.display()),
                LW / 2.0,
                LH - 52.0,
                12.0,
                dim(),
            );
        }
        centered(
            "Press Enter or Esc to go back",
            LW / 2.0,
            LH - 28.0,
            15.0,
            dim(),
        );
        if is_key_pressed(KeyCode::Enter)
            || is_key_pressed(KeyCode::Escape)
            || is_key_pressed(KeyCode::Space)
            || self.clicked()
        {
            self.screen = Screen::Title;
            self.menu = 3;
        }
    }

    fn help(&mut self, time: f64) {
        self.backdrop(time);
        centered("HOW TO PLAY", LW / 2.0, 52.0, 40.0, title_green());
        let lines = [
            "Gather every bud in the maze, then slip out the exit.",
            "Cops patrol the maze and they never stop hunting. Touch one and you lose a life.",
            "Joints (pick them up!) cloud nearby cops in smoke and leave them dazed.",
            "Press Space to set one off. Touch a dazed cop to send him back to the donut shop:",
            "100 points, then 200, 400, 800 for each one busted by the same joint.",
            "Buds are 10 points, joints 50. Finish under the par time for a bonus.",
            "An extra life every 5000 points. The cops speed up when few buds remain.",
            "",
            "Move: arrow keys or WASD      Joint: Space or Z      Pause: P",
            "Esc: pause menu      F11: fullscreen      0-7: volume      M: mute",
            "On a touch screen: swipe to move, tap with two fingers for a joint.",
        ];
        for (i, l) in lines.iter().enumerate() {
            centered(
                l,
                LW / 2.0,
                100.0 + i as f32 * 28.0,
                17.0,
                if i >= 8 { accent_for(0) } else { bright() },
            );
        }
        centered(
            "Press Enter or Esc to go back",
            LW / 2.0,
            LH - 28.0,
            15.0,
            dim(),
        );
        if is_key_pressed(KeyCode::Enter)
            || is_key_pressed(KeyCode::Escape)
            || is_key_pressed(KeyCode::Space)
            || self.clicked()
        {
            self.screen = Screen::Title;
            self.menu = 4;
        }
    }

    fn enter_name(&mut self, time: f64) {
        self.backdrop(time);
        centered("NEW HIGH SCORE!", LW / 2.0, 100.0, 48.0, accent_for(0));
        centered(
            &format!(
                "{} points on level {}",
                self.last_score.0,
                self.last_score.1 + 1
            ),
            LW / 2.0,
            150.0,
            22.0,
            bright(),
        );
        centered(
            "Type your initials, then press Enter",
            LW / 2.0,
            200.0,
            20.0,
            dim(),
        );
        while let Some(ch) = get_char_pressed() {
            if ch.is_ascii_alphanumeric() && self.name.len() < storage::NAME_LEN {
                self.name.push(ch.to_ascii_uppercase());
                self.sounds.play(Sfx::MenuMove);
            }
        }
        if is_key_pressed(KeyCode::Backspace) {
            self.name.pop();
        }
        let blink = (time * 2.5) as i64 % 2 == 0;
        let shown = format!(
            "{}{}",
            self.name,
            if blink && self.name.len() < storage::NAME_LEN {
                "_"
            } else {
                " "
            }
        );
        centered(&shown, LW / 2.0, 290.0, 80.0, title_green());
        if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
            self.new_rank = self
                .scores
                .add(&self.name, self.last_score.0, self.last_score.1);
            storage::save_scores(&self.scores);
            self.name.clear();
            self.screen = Screen::Scores;
            self.sounds.play(Sfx::MenuSelect);
        }
    }

    fn handle_events(&mut self, time: f64) {
        let Some(play) = &mut self.play else { return };
        let w = &play.world;
        let (px, py) = w.player.pos_f();
        let pos = vec2(px * TILE + TILE / 2.0, py * TILE + TILE / 2.0);
        let pal = palette(w.level_index);
        let events = w.events.clone();
        for e in events {
            match e {
                Event::LeafEaten => {
                    play.leaf_toggle = !play.leaf_toggle;
                    self.sounds.play(if play.leaf_toggle {
                        Sfx::LeafA
                    } else {
                        Sfx::LeafB
                    });
                    play.fx.burst(pos, pal.leaf, 4, 46.0);
                }
                Event::JointPicked => {
                    self.sounds.play(Sfx::JointPick);
                    play.fx.burst(pos, pal.accent, 12, 70.0);
                    play.fx.floater(pos, "+50".into(), pal.accent);
                }
                Event::JointUsed => {
                    self.sounds.play(Sfx::JointUse);
                    play.fx.shake = 0.35;
                }
                Event::NothingToStun => {
                    self.sounds.play(Sfx::NoTarget);
                    play.fx
                        .floater(pos - vec2(0.0, 18.0), "no one nearby".into(), dim());
                }
                Event::CopBusted(points) => {
                    self.sounds.play(Sfx::Bust);
                    play.fx
                        .floater(pos - vec2(0.0, 12.0), format!("BUSTED +{points}"), bright());
                    play.fx.burst(pos, WHITE, 14, 90.0);
                }
                Event::CopReleased => self.sounds.play(Sfx::Release),
                Event::PlayerCaught => {
                    self.sounds.play(Sfx::Caught);
                    play.fx.shake = 1.0;
                    play.fx.burst(pos, pal.leaf, 18, 110.0);
                }
                Event::GateOpened => {
                    self.sounds.play(Sfx::GateOpen);
                    play.fx.floater(
                        vec2(LW / 2.0, FIELD_H / 2.0),
                        "THE GATE IS OPEN!".into(),
                        c_green(),
                    );
                }
                Event::LevelCleared { time_bonus } => {
                    self.sounds.play(Sfx::Cleared);
                    play.fx.floater(
                        pos - vec2(0.0, 20.0),
                        format!("TIME BONUS +{time_bonus}"),
                        pal.accent,
                    );
                }
                Event::ExtraLife => {
                    self.sounds.play(Sfx::ExtraLife);
                    play.fx
                        .floater(pos - vec2(0.0, 30.0), "EXTRA LIFE!".into(), c_green());
                }
                Event::GameOver => self.sounds.play(Sfx::GameOver),
            }
        }
        let _ = time;
    }

    fn play_screen(&mut self, dt: f32, time: f64) {
        // In demo mode any key (or click) ends the demo.
        if self.play.as_ref().is_some_and(|p| p.bot.is_some())
            && (get_last_key_pressed().is_some() || self.clicked()) {
                self.play = None;
                self.screen = Screen::Title;
                self.menu = 0;
                return;
            }
        // Global keys while playing.
        if is_key_pressed(KeyCode::P) || is_key_pressed(KeyCode::Escape) {
            self.screen = Screen::Paused;
            self.menu = 0;
            return;
        }
        self.controls.poll();
        let clicked = self.clicked();
        let Some(play) = &mut self.play else {
            self.screen = Screen::Title;
            return;
        };

        if matches!(play.world.phase, Phase::Ready(_)) && !play.announced_ready {
            play.announced_ready = true;
            self.sounds.play(Sfx::Ready);
        }
        if !matches!(play.world.phase, Phase::Ready(_)) {
            play.announced_ready = false;
        }

        self.acc += dt;
        let mut ticks = 0;
        while self.acc >= TICK && ticks < 6 {
            self.acc -= TICK;
            ticks += 1;
            let Some(play) = &mut self.play else { return };
            let input = match &mut play.bot {
                Some(bot) => {
                    // Autopilot: follow the maze, and set off a joint when a cop gets close.
                    let near = play.world.cops.iter().any(|w| {
                        let (wx, wy) = w.mover.pos_f();
                        let (px, py) = play.world.player.pos_f();
                        ((wx - px).powi(2) + (wy - py).powi(2)).sqrt() < 2.2
                    });
                    Input {
                        dir: bot.decide(&play.world),
                        action: near && play.world.t % 7 == 0,
                    }
                }
                None => self.controls.input(),
            };
            play.world.tick(input);
            self.controls.end_tick();
            self.handle_events(time);
        }
        if ticks == 6 {
            self.acc = 0.0;
        }

        let Some(play) = &mut self.play else { return };
        play.fx.update(dt);

        // Next level.
        if play.world.ready_for_next_level() {
            let next = play.world.level_index + 1;
            let carry = play.world.carry();
            let world = World::new(
                next,
                Settings {
                    ..play.world.settings.clone()
                },
                Some(carry),
            );
            play.world = world;
            play.fx = Effects::default();
            self.acc = 0.0;
        }

        // Game over.
        if play.world.phase == Phase::GameOver {
            play.game_over_timer += dt;
            if play.game_over_timer > 1.0
                && (is_key_pressed(KeyCode::Enter)
                    || is_key_pressed(KeyCode::Space)
                    || is_key_pressed(KeyCode::Escape)
                    || clicked)
            {
                self.last_score = (play.world.score, play.world.level_index);
                self.name.clear();
                if self.scores.qualifies(self.last_score.0) {
                    self.screen = Screen::EnterName;
                } else {
                    self.screen = Screen::Title;
                    self.menu = 0;
                }
                self.play = None;
                return;
            }
        }

        let Some(play) = &self.play else { return };
        let pal = palette(play.world.level_index);
        let shake = play.fx.shake;
        let off = if shake > 0.0 {
            vec2(
                (time as f32 * 90.0).sin() * 5.0 * shake,
                (time as f32 * 70.0).cos() * 5.0 * shake,
            )
        } else {
            Vec2::ZERO
        };
        // Shake only the playfield, not the HUD.
        set_camera(&self.camera_offset(off));
        draw_playfield(&play.world, time, &pal, &play.fx);
        draw_overlay(&play.world, time, &pal);
        set_camera(&self.camera());
        draw_hud(&play.world, time, &pal);
        if play.bot.is_some() {
            centered(
                "DEMO  -  press any key to play",
                LW / 2.0,
                22.0,
                18.0,
                c_green(),
            );
        }
        if play.world.phase == Phase::GameOver && play.game_over_timer > 1.0 {
            centered("press Enter", LW / 2.0, FIELD_H / 2.0 + 62.0, 18.0, dim());
        }
    }

    fn paused(&mut self, time: f64) {
        if let Some(play) = &self.play {
            let pal = palette(play.world.level_index);
            draw_playfield(&play.world, time, &pal, &play.fx);
            draw_hud(&play.world, time, &pal);
        }
        draw_rectangle(0.0, 0.0, LW, LH, Color::new(0.0, 0.0, 0.0, 0.6));
        panel(LW / 2.0 - 170.0, 96.0, 340.0, 250.0);
        centered("PAUSED", LW / 2.0, 150.0, 42.0, title_green());
        let items: Vec<String> = ["Resume", "Restart level", "Quit to title"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        if is_key_pressed(KeyCode::P) || is_key_pressed(KeyCode::Escape) {
            self.screen = Screen::Play;
            return;
        }
        if let Some(i) = self.menu(&items, LW / 2.0, 214.0, 44.0, 24.0) {
            match i {
                0 => self.screen = Screen::Play,
                1 => {
                    let level = self.play.as_ref().map_or(0, |p| p.world.level_index);
                    self.start_game(level);
                }
                _ => {
                    self.play = None;
                    self.screen = Screen::Title;
                    self.menu = 0;
                }
            }
        }
    }

    fn camera(&self) -> Camera2D {
        self.camera_offset(Vec2::ZERO)
    }

    /// The view onto the 640x450 logical screen, centred in the window, optionally nudged (screen shake).
    fn camera_offset(&self, off: Vec2) -> Camera2D {
        let vis_w = screen_width() / self.scale;
        let vis_h = screen_height() / self.scale;
        let mut cam = Camera2D::from_display_rect(Rect::new(
            self.origin.x + off.x,
            self.origin.y + off.y,
            vis_w,
            vis_h,
        ));
        // Screen coordinates here run downwards from the top left, so flip the vertical axis.
        cam.zoom.y = -cam.zoom.y;
        cam
    }

    fn update_view(&mut self) {
        let sw = screen_width();
        let sh = screen_height();
        self.scale = (sw / LW).min(sh / LH).max(0.1);
        let vis_w = sw / self.scale;
        let vis_h = sh / self.scale;
        self.origin = vec2(-(vis_w - LW) / 2.0, -(vis_h - LH) / 2.0);
    }
}

fn c_green() -> Color {
    Color::from_rgba(130, 255, 150, 255)
}

#[macroquad::main(window_conf)]
async fn main() {
    let config = storage::load_config();
    set_fullscreen(config.fullscreen);
    set_classic(config.classic);
    let sounds = Sounds::load(config.volume).await;
    let mut app = App {
        screen: Screen::Title,
        menu: 0,
        config,
        scores: storage::load_scores(),
        sounds,
        play: None,
        controls: Controls::default(),
        start_level: 0,
        name: String::new(),
        last_score: (0, 0),
        new_rank: None,
        acc: 0.0,
        scale: 1.0,
        origin: Vec2::ZERO,
        quit: false,
    };

    // Command line (desktop only): `--demo [level]` starts the self-playing demo, `--level N` starts at level N.
    let args: Vec<String> = std::env::args().collect();
    let level_arg = |flag: &str| -> Option<usize> {
        let i = args.iter().position(|a| a == flag)?;
        args.get(i + 1)
            .and_then(|v| v.parse::<usize>().ok())
            .map(|n| n.saturating_sub(1))
    };
    // `--screen options|levels|scores|help|paused|name` opens a screen directly (handy for screenshots).
    if let Some(i) = args.iter().position(|a| a == "--screen") {
        match args.get(i + 1).map(String::as_str) {
            Some("options") => app.screen = Screen::Options,
            Some("levels") => app.screen = Screen::LevelSelect,
            Some("scores") => {
                app.scores.add("SPR", 4820, 3);
                app.scores.add("LEF", 3110, 2);
                app.scores.add("AAA", 1240, 1);
                app.new_rank = Some(1);
                app.screen = Screen::Scores;
            }
            Some("help") => app.screen = Screen::Help,
            Some("paused") => {
                app.start_game(1);
                app.screen = Screen::Paused;
            }
            Some("name") => {
                app.last_score = (5210, 4);
                app.name = "SP".to_string();
                app.screen = Screen::EnterName;
            }
            _ => {}
        }
    }
    if args.iter().any(|a| a == "--demo") {
        app.start_demo(level_arg("--demo").unwrap_or(0));
    } else if let Some(n) = level_arg("--level") {
        app.start_game(n);
    }

    let debug_keys = std::env::var_os("GIDDY_DEBUG_KEYS").is_some();

    loop {
        let dt = get_frame_time().min(0.1);
        let time = get_time();
        app.update_view();
        clear_background(Color::from_rgba(6, 10, 8, 255));
        set_camera(&app.camera());
        // The logical screen area gets a slightly lighter backing so letterbox bars are visible.
        draw_rectangle(
            0.0,
            0.0,
            LW,
            LH,
            if classic() {
                BLACK
            } else {
                Color::from_rgba(12, 20, 15, 255)
            },
        );

        if debug_keys {
            if let Some(k) = get_last_key_pressed() {
                eprintln!("[key] {k:?}");
            }
        }
        // Global shortcuts.
        if is_key_pressed(KeyCode::F11) {
            app.config.fullscreen = !app.config.fullscreen;
            set_fullscreen(app.config.fullscreen);
            app.save_config();
        }
        for (i, key) in [
            KeyCode::Key0,
            KeyCode::Key1,
            KeyCode::Key2,
            KeyCode::Key3,
            KeyCode::Key4,
            KeyCode::Key5,
            KeyCode::Key6,
            KeyCode::Key7,
        ]
        .iter()
        .enumerate()
        {
            if is_key_pressed(*key) && app.screen != Screen::EnterName {
                app.config.volume = i as u8;
                app.sounds.volume = i as u8;
                app.sounds.play(Sfx::MenuSelect);
                app.save_config();
            }
        }
        if is_key_pressed(KeyCode::M) && app.screen != Screen::EnterName {
            app.sounds.volume = if app.sounds.volume == 0 {
                app.config.volume.max(3)
            } else {
                0
            };
        }

        match app.screen {
            Screen::Title => app.title(time),
            Screen::Options => app.options(time),
            Screen::LevelSelect => app.level_select(time),
            Screen::Scores => app.scores_screen(time),
            Screen::Help => app.help(time),
            Screen::EnterName => app.enter_name(time),
            Screen::Play => app.play_screen(dt, time),
            Screen::Paused => app.paused(time),
        }

        set_default_camera();
        if app.quit && !IS_WEB {
            break;
        }
        next_frame().await;
    }
}
