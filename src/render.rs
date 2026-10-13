//! Drawing. Everything is built from shapes at a fixed 640x450 logical size, so the game stays
//! sharp at any window size. No image files are used.

use giddy_ganja::ai::Personality;
use giddy_ganja::game::*;
use giddy_ganja::grid::*;
use macroquad::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};

static CLASSIC: AtomicBool = AtomicBool::new(true);

/// Switch between the "Classic" look (pastel hedges on black, blue menus) and the "Modern" one.
pub fn set_classic(on: bool) {
    CLASSIC.store(on, Ordering::Relaxed);
}

pub fn classic() -> bool {
    CLASSIC.load(Ordering::Relaxed)
}

pub const TILE: f32 = 32.0;
pub const LW: f32 = 640.0;
pub const LH: f32 = 450.0;
pub const FIELD_H: f32 = TILE * H as f32;

fn c(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgba(r, g, b, 255)
}

fn alpha(col: Color, a: f32) -> Color {
    Color::new(col.r, col.g, col.b, col.a * a.clamp(0.0, 1.0))
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

pub struct Palette {
    pub floor: Color,
    pub floor_alt: Color,
    pub wall_dark: Color,
    pub wall_mid: Color,
    pub wall_light: Color,
    pub leaf: Color,
    pub accent: Color,
}

/// Each level gets its own look, cycling through a handful of palettes.
pub fn palette(level: usize) -> Palette {
    if classic() {
        // Pastel hedge tiles on black corridors, a different colour for each level.
        let (light, mid, dark) = match level % 6 {
            0 => (c(160, 255, 220), c(120, 236, 190), c(30, 140, 100)),
            1 => (c(255, 244, 160), c(240, 216, 110), c(150, 120, 30)),
            2 => (c(220, 196, 255), c(182, 152, 242), c(96, 66, 170)),
            3 => (c(255, 196, 220), c(240, 150, 186), c(170, 60, 110)),
            4 => (c(176, 228, 255), c(130, 196, 242), c(40, 104, 170)),
            _ => (c(255, 214, 172), c(240, 178, 120), c(166, 100, 40)),
        };
        return Palette {
            floor: c(0, 0, 0),
            floor_alt: c(0, 0, 0),
            wall_dark: dark,
            wall_mid: mid,
            wall_light: light,
            leaf: c(48, 230, 64),
            accent: c(255, 255, 255),
        };
    }
    match level % 6 {
        0 => Palette {
            floor: c(24, 30, 22),
            floor_alt: c(28, 35, 25),
            wall_dark: c(22, 74, 36),
            wall_mid: c(38, 130, 58),
            wall_light: c(84, 190, 96),
            leaf: c(110, 235, 110),
            accent: c(255, 214, 80),
        },
        1 => Palette {
            floor: c(34, 26, 20),
            floor_alt: c(40, 30, 23),
            wall_dark: c(120, 52, 18),
            wall_mid: c(190, 96, 28),
            wall_light: c(236, 160, 60),
            leaf: c(255, 198, 70),
            accent: c(110, 220, 255),
        },
        2 => Palette {
            floor: c(16, 18, 34),
            floor_alt: c(20, 22, 40),
            wall_dark: c(40, 40, 112),
            wall_mid: c(72, 76, 176),
            wall_light: c(132, 140, 236),
            leaf: c(170, 255, 190),
            accent: c(255, 140, 200),
        },
        3 => Palette {
            floor: c(18, 28, 34),
            floor_alt: c(22, 34, 42),
            wall_dark: c(40, 100, 128),
            wall_mid: c(84, 170, 204),
            wall_light: c(190, 236, 250),
            leaf: c(150, 255, 214),
            accent: c(255, 176, 96),
        },
        4 => Palette {
            floor: c(32, 20, 30),
            floor_alt: c(38, 24, 36),
            wall_dark: c(110, 36, 90),
            wall_mid: c(176, 70, 150),
            wall_light: c(240, 140, 214),
            leaf: c(190, 255, 150),
            accent: c(255, 236, 120),
        },
        _ => Palette {
            floor: c(38, 30, 18),
            floor_alt: c(44, 35, 21),
            wall_dark: c(120, 90, 40),
            wall_mid: c(186, 146, 70),
            wall_light: c(238, 208, 130),
            leaf: c(130, 230, 120),
            accent: c(120, 210, 255),
        },
    }
}

fn cop_colors(kind: Personality) -> (Color, Color) {
    match kind {
        Personality::Hunter => (c(46, 92, 214), c(24, 48, 130)),
        Personality::Ambusher => (c(150, 64, 200), c(86, 34, 124)),
        Personality::Flanker => (c(30, 168, 170), c(16, 100, 104)),
        Personality::Prowler => (c(230, 124, 32), c(144, 74, 16)),
    }
}

fn hash(x: i32, y: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1) ^ (y as u32).wrapping_mul(0x85EB_CA77);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^ (h >> 12)
}

fn is_wall(level: &giddy_ganja::levels::Level, x: i32, y: i32) -> bool {
    matches!(level.tile((x, y)), Tile::Wall)
}

/// Classic wall: a flat pastel tile with a little plant pattern, like a printed garden wall.
fn draw_wall_classic(x: i32, y: i32, pal: &Palette) {
    let px = x as f32 * TILE;
    let py = y as f32 * TILE;
    draw_rectangle(px, py, TILE, TILE, pal.wall_light);
    draw_rectangle_lines(px, py, TILE, TILE, 1.0, pal.wall_mid);
    let shift = ((x + y) % 2) as f32 * 8.0;
    for k in 0..2 {
        let cx = px + 8.0 + k as f32 * 16.0;
        let cy = py + 10.0 + ((k as f32 + shift / 8.0) % 2.0) * 12.0;
        draw_line(cx, cy + 8.0, cx, cy - 6.0, 1.6, pal.wall_dark);
        for (dx, dy) in [(-5.0f32, -2.0f32), (5.0, -2.0), (-4.0, 3.0), (4.0, 3.0)] {
            draw_triangle(
                vec2(cx, cy + dy * 0.6),
                vec2(cx + dx, cy + dy - 3.0),
                vec2(cx + dx * 0.9, cy + dy + 2.5),
                pal.wall_dark,
            );
        }
        draw_circle(cx, cy - 7.0, 2.2, pal.wall_dark);
    }
}

fn draw_wall(level: &giddy_ganja::levels::Level, x: i32, y: i32, pal: &Palette) {
    if classic() {
        return draw_wall_classic(x, y, pal);
    }
    let px = x as f32 * TILE;
    let py = y as f32 * TILE;
    draw_rectangle(px, py, TILE, TILE, pal.wall_dark);
    // Blocks merge with their wall neighbours so hedges read as long runs, not single tiles.
    let inset = 3.0;
    let l = if is_wall(level, x - 1, y) { 0.0 } else { inset };
    let r = if is_wall(level, x + 1, y) { 0.0 } else { inset };
    let t = if is_wall(level, x, y - 1) { 0.0 } else { inset };
    let b = if is_wall(level, x, y + 1) { 0.0 } else { inset };
    draw_rectangle(px + l, py + t, TILE - l - r, TILE - t - b, pal.wall_mid);
    // Highlight along exposed top/left edges.
    if t > 0.0 {
        draw_rectangle(px + l, py + t, TILE - l - r, 2.0, pal.wall_light);
    }
    if l > 0.0 {
        draw_rectangle(px + l, py + t, 2.0, TILE - t - b, pal.wall_light);
    }
    // A few leaf flecks, placed by hash so they never flicker.
    let h = hash(x, y);
    for i in 0..3u32 {
        let fx = ((h >> (i * 5)) & 15) as f32 / 15.0 * 20.0 + 6.0;
        let fy = ((h >> (i * 5 + 10)) & 15) as f32 / 15.0 * 20.0 + 6.0;
        draw_circle(px + fx, py + fy, 2.2, alpha(pal.wall_light, 0.55));
        draw_circle(px + fx + 1.5, py + fy + 1.0, 1.6, alpha(pal.wall_dark, 0.5));
    }
}

fn draw_leaf(cx: f32, cy: f32, time: f64, seed: f32, pal: &Palette) {
    // A tiny five-fingered cannabis leaf that gently pulses.
    let pulse = 1.0 + ((time as f32 * 3.0 + seed).sin() * 0.08);
    let dark = lerp_color(pal.leaf, BLACK, 0.35);
    draw_line(cx, cy + 6.0 * pulse, cx, cy + 1.0, 1.4, dark);
    for (ang, len, w) in [
        (-1.25f32, 6.0f32, 1.7f32),
        (-0.62, 8.0, 1.9),
        (0.0, 9.0, 2.0),
        (0.62, 8.0, 1.9),
        (1.25, 6.0, 1.7),
    ] {
        let base = vec2(cx, cy + 1.5);
        let dir = vec2(ang.sin(), -ang.cos());
        let tip = base + dir * len * pulse;
        let side = vec2(-dir.y, dir.x) * w;
        let mid = base + dir * len * 0.45 * pulse;
        draw_triangle(base, mid + side, tip, pal.leaf);
        draw_triangle(base, mid - side, tip, dark);
    }
}

fn draw_joint(cx: f32, cy: f32, time: f64, pal: &Palette) {
    let _ = pal;
    let t = time as f32;
    let a = vec2(cx - 8.0, cy + 6.0);
    let b = vec2(cx + 6.0, cy - 4.0);
    // Paper, a darker twisted end, a filter, and the ember.
    draw_line(a.x, a.y, b.x, b.y, 5.0, c(246, 240, 224));
    draw_line(b.x - 2.0, b.y + 1.4, b.x, b.y, 5.0, c(200, 190, 170));
    draw_line(a.x, a.y, a.x + 2.6, a.y - 1.9, 5.0, c(190, 150, 100));
    let glow = 0.5 + 0.5 * (t * 6.0).sin();
    draw_circle(
        b.x + 1.0,
        b.y - 0.6,
        5.0 + glow * 1.5,
        alpha(c(255, 140, 40), 0.35 + 0.25 * glow),
    );
    draw_circle(b.x + 1.0, b.y - 0.6, 2.6, c(255, 96, 30));
    draw_circle(b.x + 1.0, b.y - 0.6, 1.2, c(255, 230, 140));
    // Smoke curls upwards.
    for i in 0..3 {
        let phase = (t * 0.9 + i as f32 * 0.33).fract();
        let x = b.x + 1.0 + (phase * 9.0 + i as f32).sin() * 3.0;
        let y = b.y - 3.0 - phase * 13.0;
        draw_circle(
            x,
            y,
            1.6 + phase * 3.0,
            alpha(c(214, 220, 214), 0.5 * (1.0 - phase)),
        );
    }
}

fn draw_gate(px: f32, py: f32, open: bool, time: f64) {
    if open {
        let g = 0.5 + 0.5 * (time as f32 * 6.0).sin();
        draw_rectangle(px, py, TILE, TILE, c(20, 70, 30));
        draw_rectangle(
            px + 3.0,
            py + 3.0,
            TILE - 6.0,
            TILE - 6.0,
            alpha(c(120, 255, 140), 0.5 + 0.4 * g),
        );
        let cx = px + TILE / 2.0;
        draw_triangle(
            vec2(cx, py + 6.0),
            vec2(cx - 8.0, py + 18.0),
            vec2(cx + 8.0, py + 18.0),
            WHITE,
        );
        draw_rectangle(cx - 3.0, py + 17.0, 6.0, 10.0, WHITE);
    } else {
        draw_rectangle(px, py, TILE, TILE, c(30, 30, 34));
        for i in 0..5 {
            let o = i as f32 * 8.0 - 8.0;
            let col = if i % 2 == 0 {
                c(240, 190, 30)
            } else {
                c(30, 30, 34)
            };
            draw_triangle(
                vec2(px + o, py + TILE),
                vec2(px + o + 8.0, py + TILE),
                vec2(px + o + 16.0, py),
                col,
            );
            draw_triangle(
                vec2(px + o + 8.0, py + TILE),
                vec2(px + o + 24.0, py),
                vec2(px + o + 16.0, py),
                col,
            );
        }
        draw_rectangle_lines(
            px + 1.0,
            py + 1.0,
            TILE - 2.0,
            TILE - 2.0,
            2.0,
            c(70, 70, 76),
        );
    }
}

fn draw_pen(px: f32, py: f32, x: i32, y: i32) {
    // The donut shop: dim floor tiles with a donut on each.
    draw_rectangle(px, py, TILE, TILE, c(40, 24, 44));
    draw_rectangle_lines(px, py, TILE, TILE, 1.0, c(80, 50, 86));
    let (cx, cy) = (px + TILE / 2.0, py + TILE / 2.0 + 1.0);
    draw_circle(cx, cy + 1.2, 10.5, c(120, 70, 36));
    draw_circle(cx, cy, 10.0, c(214, 150, 80));
    let icing = [
        c(255, 120, 190),
        c(250, 236, 130),
        c(160, 220, 255),
        c(200, 160, 255),
    ][((x + y) as usize) % 4];
    draw_circle(cx, cy - 0.5, 8.2, icing);
    draw_circle(cx, cy, 3.2, c(40, 24, 44));
    for k in 0..5 {
        let a = k as f32 * 1.26 + (x * 3 + y) as f32;
        let col = [
            c(255, 255, 255),
            c(90, 220, 120),
            c(255, 90, 90),
            c(80, 140, 255),
            c(255, 210, 60),
        ][k];
        draw_rectangle(
            cx + a.cos() * 5.6 - 0.8,
            cy + a.sin() * 5.6 - 0.8,
            1.8,
            1.8,
            col,
        );
    }
}

pub struct Particle {
    pos: Vec2,
    vel: Vec2,
    life: f32,
    max: f32,
    color: Color,
    size: f32,
}

#[derive(Default)]
pub struct Effects {
    particles: Vec<Particle>,
    floaters: Vec<(Vec2, String, f32, Color)>,
    pub shake: f32,
}

impl Effects {
    pub fn burst(&mut self, pos: Vec2, color: Color, n: usize, speed: f32) {
        for i in 0..n {
            let a = (i as f32 / n as f32) * std::f32::consts::TAU + (i as f32 * 1.7).sin();
            let s = speed * (0.4 + ((i * 37 % 11) as f32) / 11.0);
            self.particles.push(Particle {
                pos,
                vel: vec2(a.cos() * s, a.sin() * s),
                life: 0.5,
                max: 0.5,
                color,
                size: 2.0 + (i % 3) as f32,
            });
        }
    }

    pub fn floater(&mut self, pos: Vec2, text: String, color: Color) {
        self.floaters.push((pos, text, 1.0, color));
    }

    pub fn update(&mut self, dt: f32) {
        for p in &mut self.particles {
            p.life -= dt;
            p.pos += p.vel * dt;
            p.vel *= 0.94;
        }
        self.particles.retain(|p| p.life > 0.0);
        for f in &mut self.floaters {
            f.2 -= dt;
            f.0.y -= 22.0 * dt;
        }
        self.floaters.retain(|f| f.2 > 0.0);
        self.shake = (self.shake - dt * 2.2).max(0.0);
    }

    pub fn draw(&self) {
        for p in &self.particles {
            draw_circle(
                p.pos.x,
                p.pos.y,
                p.size * (p.life / p.max),
                alpha(p.color, p.life / p.max),
            );
        }
        for (pos, text, life, color) in &self.floaters {
            let a = (*life * 1.6).min(1.0);
            let w = measure_text(text, None, 20, 1.0).width;
            draw_text(
                text,
                pos.x - w / 2.0 + 1.0,
                pos.y + 1.0,
                20.0,
                alpha(BLACK, a),
            );
            draw_text(text, pos.x - w / 2.0, pos.y, 20.0, alpha(*color, a));
        }
    }
}

fn draw_giddy(
    cx: f32,
    cy: f32,
    dir: Option<Dir>,
    scale: f32,
    time: f64,
    moving: bool,
    tint: Color,
) {
    let _ = tint;
    let bob = if moving {
        (time as f32 * 16.0).sin() * 1.2
    } else {
        (time as f32 * 2.0).sin() * 0.5
    };
    let cy = cy + bob;
    let sway = (time as f32 * 3.0).sin() * 0.06;
    let dark = c(18, 96, 40);
    let mid = c(52, 190, 84);
    let light = c(120, 236, 120);
    // Seven serrated leaflets fan out from behind the face, like a cannabis leaf.
    for (ang, len, w) in [
        (-1.45f32, 9.5f32, 3.0f32),
        (-0.95, 13.0, 3.6),
        (-0.42, 16.0, 4.0),
        (0.0, 17.0, 4.2),
        (0.42, 16.0, 4.0),
        (0.95, 13.0, 3.6),
        (1.45, 9.5, 3.0),
    ] {
        let a = ang + sway;
        let base = vec2(cx, cy - 1.0 * scale);
        let dir_v = vec2(a.sin(), -a.cos());
        let tip = base + dir_v * len * scale;
        let side = vec2(-dir_v.y, dir_v.x) * w * scale;
        let mid_p = base + dir_v * len * 0.5 * scale;
        draw_triangle(base, mid_p + side, tip, mid);
        draw_triangle(base, mid_p - side, tip, dark);
        draw_line(base.x, base.y, tip.x, tip.y, 1.0, light);
    }
    // Round face.
    draw_circle(cx, cy + 2.0 * scale, 10.8 * scale, dark);
    draw_circle(cx, cy + 1.5 * scale, 9.6 * scale, mid);
    draw_circle(
        cx - 3.0 * scale,
        cy - 1.5 * scale,
        3.0 * scale,
        alpha(WHITE, 0.2),
    );
    let (dx, dy) = dir.map_or((0.0, 0.0), |d| {
        let (x, y) = d.delta();
        (x as f32, y as f32)
    });
    // Heavy-lidded, delighted eyes.
    for side in [-1.0f32, 1.0] {
        let ex = cx + side * 3.9 * scale + dx * 1.2;
        let ey = cy - 0.4 * scale + dy * 1.2;
        draw_circle(ex, ey, 3.0 * scale, WHITE);
        draw_circle(
            ex + dx * 1.2,
            ey + dy * 1.2 + 0.4,
            1.4 * scale,
            c(20, 20, 30),
        );
        draw_rectangle(
            ex - 3.2 * scale,
            ey - 3.2 * scale,
            6.4 * scale,
            3.0 * scale,
            mid,
        );
        draw_line(
            ex - 3.2 * scale,
            ey - 0.2 * scale,
            ex + 3.2 * scale,
            ey - 0.2 * scale,
            1.2,
            dark,
        );
    }
    // A huge grin.
    let mouth = c(40, 24, 24);
    draw_rectangle(
        cx - 4.6 * scale,
        cy + 3.6 * scale,
        9.2 * scale,
        3.4 * scale,
        mouth,
    );
    draw_circle(cx, cy + 3.6 * scale, 4.6 * scale, mouth);
    draw_rectangle(
        cx - 4.0 * scale,
        cy + 3.4 * scale,
        8.0 * scale,
        1.4 * scale,
        WHITE,
    );
    draw_circle(cx, cy + 6.4 * scale, 2.0 * scale, c(240, 110, 130));
}

fn draw_cop(w: &Cop, tick: u64, time: f64) {
    let (x, y) = w.mover.pos_f();
    let (cx, cy) = (x * TILE + TILE / 2.0, y * TILE + TILE / 2.0);
    let (body, dark) = cop_colors(w.kind);
    let stunned = matches!(w.state, CopState::Stunned { .. });
    let waiting = matches!(w.state, CopState::Waiting { .. });
    let (body, dark) = if stunned {
        (c(170, 196, 230), c(110, 130, 170))
    } else {
        (body, dark)
    };
    let bob = if waiting {
        (time as f32 * 4.0 + cx).sin() * 1.5
    } else if stunned {
        (time as f32 * 30.0).sin() * 1.0
    } else {
        (time as f32 * 14.0 + cx).sin() * 1.0
    };
    let cy = cy + bob;
    // Body, head, cap.
    draw_circle(cx, cy + 4.0, 11.5, dark);
    draw_circle(cx, cy + 3.5, 10.0, body);
    draw_circle(cx, cy - 6.0, 7.2, c(238, 200, 160));
    draw_rectangle(cx - 8.0, cy - 15.0, 16.0, 6.0, dark);
    draw_rectangle(cx - 10.0, cy - 10.0, 20.0, 2.2, c(20, 20, 30));
    draw_circle(cx, cy + 6.0, 2.4, c(255, 214, 80)); // badge
    let (dx, dy) = w.mover.dir.map_or((0.0, 0.0), |d| {
        let (x, y) = d.delta();
        (x as f32, y as f32)
    });
    if stunned {
        // Swirly dazed eyes and a goofy grin.
        for s in [-1.0f32, 1.0] {
            let ex = cx + s * 3.2;
            for k in 0..14 {
                let a = k as f32 * 0.55 + time as f32 * 9.0;
                let r = 0.4 + k as f32 * 0.16;
                draw_circle(ex + a.cos() * r, cy - 6.0 + a.sin() * r, 0.7, c(30, 30, 60));
            }
        }
        draw_line(cx - 3.2, cy - 2.4, cx, cy - 1.0, 1.3, c(120, 60, 50));
        draw_line(cx, cy - 1.0, cx + 3.2, cy - 2.4, 1.3, c(120, 60, 50));
        // Smoke drifting around the head.
        for i in 0..4 {
            let a = time as f32 * 2.2 + i as f32 * 1.57;
            let rise = ((time as f32 * 0.8 + i as f32 * 0.25).fract()) * 8.0;
            draw_circle(
                cx + a.cos() * 11.0,
                cy - 15.0 - rise + a.sin() * 2.0,
                3.6 - rise * 0.25,
                alpha(c(220, 230, 220), 0.55),
            );
        }
        // Wobble warning near the end of the stun.
        if let CopState::Stunned { until } = w.state {
            if until.saturating_sub(tick) < 70 && (tick / 6) % 2 == 0 {
                draw_circle_lines(cx, cy - 2.0, 15.0, 2.0, alpha(WHITE, 0.8));
            }
        }
    } else {
        for s in [-1.0f32, 1.0] {
            draw_circle(cx + s * 3.0 + dx, cy - 6.0 + dy, 1.7, c(20, 20, 30));
        }
        draw_line(cx - 2.5, cy - 2.6, cx + 2.5, cy - 2.6, 1.2, c(120, 60, 50));
    }
}

/// Draw the whole playfield (tiles, pickups, actors, effects) in logical coordinates.
pub fn draw_playfield(world: &World, time: f64, pal: &Palette, fx: &Effects) {
    // Background tiles.
    for y in 0..H as i32 {
        for x in 0..W as i32 {
            let p = (x, y);
            let px = x as f32 * TILE;
            let py = y as f32 * TILE;
            match world.level.tile(p) {
                Tile::Wall => draw_wall(&world.level, x, y, pal),
                Tile::Floor => {
                    let col = if (x + y) % 2 == 0 {
                        pal.floor
                    } else {
                        pal.floor_alt
                    };
                    draw_rectangle(px, py, TILE, TILE, col);
                }
                Tile::Pen => draw_pen(px, py, x, y),
                Tile::Gate => draw_gate(px, py, world.gate_open, time),
            }
        }
    }
    // Pickups.
    for y in 0..H {
        for x in 0..W {
            let cx = x as f32 * TILE + TILE / 2.0;
            let cy = y as f32 * TILE + TILE / 2.0;
            if world.leaves[y][x] {
                draw_leaf(cx, cy, time, (x * 7 + y * 3) as f32, pal);
            }
            if world.joint_tiles[y][x] {
                draw_joint(cx, cy, time, pal);
            }
        }
    }
    // Cops, then the player on top.
    for w in &world.cops {
        draw_cop(w, world.t, time);
    }
    // The joint's smoke cloud rolls outwards and fades.
    if let Some(p) = world.pulse {
        let t = p.age as f32 / PULSE_TICKS as f32;
        let cx = p.center.0 * TILE + TILE / 2.0;
        let cy = p.center.1 * TILE + TILE / 2.0;
        let reach = STUN_RADIUS * TILE * t;
        for i in 0..18 {
            let a = i as f32 * 0.349 + (i as f32 * 1.7).sin();
            let r = reach * (0.45 + 0.55 * (((i * 7) % 11) as f32 / 11.0));
            let size = 14.0 + 10.0 * (((i * 5) % 7) as f32 / 7.0) + 18.0 * t;
            draw_circle(
                cx + a.cos() * r,
                cy + a.sin() * r,
                size,
                alpha(c(196, 220, 196), 0.34 * (1.0 - t)),
            );
        }
        draw_circle(
            cx,
            cy,
            reach * 0.5,
            alpha(c(210, 235, 210), 0.22 * (1.0 - t)),
        );
    }
    let (px, py) = world.player.pos_f();
    let (cx, cy) = (px * TILE + TILE / 2.0, py * TILE + TILE / 2.0);
    match world.phase {
        Phase::Dying(n) => {
            let t = 1.0 - n as f32 / DYING_TICKS as f32;
            let scale = (1.0 - t).max(0.05);
            draw_giddy(cx, cy - t * 14.0, None, scale, time, false, WHITE);
            for i in 0..6 {
                let a = i as f32 / 6.0 * std::f32::consts::TAU + t * 3.0;
                draw_poly(
                    cx + a.cos() * t * 22.0,
                    cy - t * 14.0 + a.sin() * t * 22.0,
                    4,
                    3.0,
                    45.0,
                    alpha(pal.leaf, 1.0 - t),
                );
            }
        }
        _ => {
            let moving = world.player.dir.is_some() && matches!(world.phase, Phase::Playing);
            draw_giddy(cx, cy, world.player.dir.or(None), 1.0, time, moving, WHITE);
        }
    }
    fx.draw();
}

fn text_centered(text: &str, cx: f32, y: f32, size: f32, color: Color) {
    let w = measure_text(text, None, size as u16, 1.0).width;
    draw_text(text, cx - w / 2.0 + 2.0, y + 2.0, size, alpha(BLACK, 0.7));
    draw_text(text, cx - w / 2.0, y, size, color);
}

pub fn draw_hud(world: &World, time: f64, pal: &Palette) {
    let top = FIELD_H;
    let is_classic = classic();
    draw_rectangle(
        0.0,
        top,
        LW,
        LH - top,
        if is_classic {
            c(0, 0, 0)
        } else {
            c(10, 14, 12)
        },
    );
    draw_rectangle(0.0, top, LW, 3.0, pal.wall_mid);
    let label = if is_classic {
        c(255, 255, 255)
    } else {
        c(150, 190, 160)
    };
    let value = c(240, 250, 240);

    draw_text("TIME", 16.0, top + 22.0, 16.0, label);
    let secs = world.elapsed_seconds();
    draw_text(
        format!("{}:{:02}", secs as u32 / 60, secs as u32 % 60),
        16.0,
        top + 52.0,
        32.0,
        value,
    );

    draw_text("JOINTS", 108.0, top + 22.0, 16.0, label);
    for i in 0..MAX_JOINTS {
        let x = 118.0 + i as f32 * 26.0;
        if i < world.joints {
            draw_joint(x, top + 44.0, time, pal);
        } else {
            draw_line(x - 7.0, top + 52.0, x + 3.0, top + 39.0, 2.0, c(50, 56, 52));
        }
    }

    draw_text("SCORE", 232.0, top + 22.0, 16.0, label);
    draw_text(format!("{}", world.score), 232.0, top + 52.0, 32.0, value);

    draw_text(
        format!("LEVEL {}  {}", world.level_index + 1, world.level.name),
        360.0,
        top + 22.0,
        16.0,
        label,
    );
    let total = world.level.leaf_count().max(1) as f32;
    let done = 1.0 - world.leaves_left as f32 / total;
    draw_rectangle(360.0, top + 32.0, 150.0, 12.0, c(30, 40, 34));
    draw_rectangle(360.0, top + 32.0, 150.0 * done, 12.0, pal.leaf);
    draw_rectangle_lines(360.0, top + 32.0, 150.0, 12.0, 1.5, c(90, 110, 96));
    let status = if world.gate_open {
        "THE COAST IS CLEAR! GO!".to_string()
    } else {
        format!("{} buds left", world.leaves_left)
    };
    draw_text(
        &status,
        360.0,
        top + 62.0,
        16.0,
        if world.gate_open {
            c(130, 255, 150)
        } else {
            label
        },
    );

    draw_text("LIVES", 540.0, top + 22.0, 16.0, label);
    for i in 0..world.lives.min(3) {
        draw_giddy(
            556.0 + i as f32 * 24.0,
            top + 46.0,
            None,
            0.6,
            time,
            false,
            WHITE,
        );
    }
    if world.lives > 3 {
        draw_text(
            format!("+{}", world.lives - 3),
            618.0,
            top + 52.0,
            16.0,
            value,
        );
    }
}

/// Banner text drawn over the playfield for each phase.
pub fn draw_overlay(world: &World, time: f64, pal: &Palette) {
    let cx = LW / 2.0;
    let cy = FIELD_H / 2.0;
    match world.phase {
        Phase::Ready(_) => {
            draw_rectangle(0.0, cy - 40.0, LW, 80.0, alpha(BLACK, 0.55));
            let pulse = 1.0 + ((time as f32) * 6.0).sin() * 0.04;
            text_centered(
                &format!("LEVEL {}", world.level_index + 1),
                cx,
                cy - 6.0,
                34.0 * pulse,
                WHITE,
            );
            text_centered(
                &format!(
                    "{}  -  get every bud, then slip out the exit",
                    world.level.name
                ),
                cx,
                cy + 24.0,
                18.0,
                pal.accent,
            );
        }
        Phase::Cleared(_) => {
            draw_rectangle(0.0, cy - 46.0, LW, 92.0, alpha(BLACK, 0.6));
            text_centered("MUNCHIES TIME!", cx, cy - 4.0, 44.0, c(130, 255, 150));
            text_centered(
                &format!("Score {}", world.score),
                cx,
                cy + 28.0,
                22.0,
                WHITE,
            );
        }
        Phase::GameOver => {
            draw_rectangle(0.0, cy - 46.0, LW, 92.0, alpha(BLACK, 0.7));
            text_centered("TOTALLY BUSTED", cx, cy - 4.0, 46.0, c(255, 110, 110));
            text_centered(
                &format!("Final score {}", world.score),
                cx,
                cy + 28.0,
                22.0,
                WHITE,
            );
        }
        _ => {}
    }
}

pub fn centered(text: &str, cx: f32, y: f32, size: f32, color: Color) {
    text_centered(text, cx, y, size, color);
}

pub fn accent_for(level: usize) -> Color {
    palette(level).accent
}

pub fn dim() -> Color {
    if classic() {
        c(206, 220, 255)
    } else {
        c(150, 190, 160)
    }
}

pub fn bright() -> Color {
    if classic() {
        c(255, 255, 255)
    } else {
        c(240, 250, 240)
    }
}

/// The colour of screen titles and the selected menu entry.
pub fn title_green() -> Color {
    if classic() {
        c(255, 64, 190)
    } else {
        c(110, 235, 110)
    }
}

/// The menu/screen background: royal blue for Classic, a dark garden green otherwise.
pub fn menu_bg() -> Color {
    if classic() {
        c(0, 0, 222)
    } else {
        c(12, 20, 15)
    }
}

pub fn panel(x: f32, y: f32, w: f32, h: f32) {
    if classic() {
        draw_rectangle(x, y, w, h, c(0, 160, 130));
        draw_rectangle_lines(x, y, w, h, 4.0, c(255, 255, 255));
        draw_rectangle_lines(x + 6.0, y + 6.0, w - 12.0, h - 12.0, 1.5, c(0, 0, 0));
    } else {
        draw_rectangle(x, y, w, h, alpha(c(8, 14, 10), 0.92));
        draw_rectangle_lines(x, y, w, h, 2.0, c(60, 130, 80));
    }
}

pub fn draw_title_art(cx: f32, y: f32, time: f64) {
    // A big bouncing giddy beside the title.
    let bob = (time as f32 * 3.0).sin() * 5.0;
    draw_giddy(
        cx - 150.0,
        y + bob,
        Some(Dir::Right),
        2.6,
        time,
        true,
        WHITE,
    );
}
