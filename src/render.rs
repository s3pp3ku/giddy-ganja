//! Drawing. Everything is built from shapes at a fixed 640x450 logical size and tessellated for
//! the real window, so the game stays smooth at any size. No image files are used.

use crate::art::{self, CopMood, INK};
pub use crate::gfx::*;
use giddy_ganja::game::*;
use giddy_ganja::grid::*;
use giddy_ganja::levels::Level;
use giddy_ganja::themes::Theme;
use macroquad::prelude::*;

pub const TILE: f32 = 32.0;
pub const LW: f32 = 640.0;
pub const LH: f32 = 450.0;
pub const FIELD_H: f32 = TILE * H as f32;

/// The look of one level, picked by its theme.
pub struct Palette {
    pub theme: Theme,
    pub floor_top: Color,
    pub floor_bottom: Color,
    pub speck: Color,
    pub wall_top: Color,
    pub wall_side: Color,
    pub wall_hi: Color,
    pub pattern: Color,
    pub leaf: Color,
    pub accent: Color,
}

pub fn palette(level: usize) -> Palette {
    let theme = Theme::of_level(level);
    let p =
        |floor_top, floor_bottom, speck, wall_top, wall_side, wall_hi, pattern, accent| Palette {
            theme,
            floor_top,
            floor_bottom,
            speck,
            wall_top,
            wall_side,
            wall_hi,
            pattern,
            leaf: c(84, 222, 88),
            accent,
        };
    match theme {
        Theme::Joint => p(
            c(18, 34, 22),
            c(8, 16, 11),
            c(70, 120, 80),
            c(246, 238, 214),
            c(170, 150, 112),
            c(255, 252, 240),
            c(196, 184, 150),
            c(255, 214, 90),
        ),
        Theme::RedBong => p(
            c(10, 22, 40),
            c(4, 10, 22),
            c(110, 170, 230),
            c(222, 44, 62),
            c(120, 14, 32),
            c(255, 140, 150),
            c(255, 190, 196),
            c(255, 120, 130),
        ),
        Theme::GoldPipe => p(
            c(34, 22, 14),
            c(16, 10, 6),
            c(110, 76, 44),
            c(240, 192, 70),
            c(140, 92, 22),
            c(255, 240, 170),
            c(200, 148, 40),
            c(255, 220, 110),
        ),
        Theme::Acid => p(
            c(30, 8, 44),
            c(10, 2, 22),
            c(200, 120, 255),
            c(240, 90, 200),
            c(110, 30, 120),
            c(255, 220, 250),
            c(255, 255, 255),
            c(255, 120, 230),
        ),
        Theme::Mushrooms => p(
            c(22, 12, 32),
            c(8, 4, 14),
            c(190, 140, 255),
            c(84, 128, 62),
            c(36, 64, 30),
            c(150, 196, 110),
            c(60, 98, 46),
            c(210, 160, 255),
        ),
        Theme::Hookah => p(
            c(46, 12, 24),
            c(20, 4, 10),
            c(170, 80, 110),
            c(46, 176, 172),
            c(16, 90, 98),
            c(150, 236, 226),
            c(240, 196, 80),
            c(120, 240, 230),
        ),
        Theme::Brownies => p(
            c(34, 20, 44),
            c(12, 6, 18),
            c(200, 150, 220),
            c(146, 86, 46),
            c(70, 36, 16),
            c(196, 132, 84),
            c(92, 48, 22),
            c(255, 196, 140),
        ),
        Theme::Blunt => p(
            c(24, 22, 20),
            c(10, 9, 8),
            c(110, 104, 96),
            c(156, 96, 44),
            c(86, 48, 18),
            c(204, 146, 84),
            c(110, 64, 26),
            c(255, 176, 90),
        ),
        Theme::Peyote => p(
            c(54, 22, 54),
            c(18, 8, 28),
            c(255, 220, 180),
            c(232, 160, 100),
            c(150, 80, 44),
            c(255, 214, 160),
            c(196, 118, 70),
            c(255, 170, 120),
        ),
        Theme::SpaceCake => p(
            c(8, 10, 34),
            c(2, 2, 12),
            c(220, 230, 255),
            c(255, 172, 214),
            c(150, 86, 64),
            c(255, 226, 240),
            c(255, 255, 255),
            c(150, 210, 255),
        ),
    }
}

fn hash(x: i32, y: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1) ^ (y as u32).wrapping_mul(0x85EB_CA77);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^ (h >> 12)
}

fn unit(h: u32, k: u32) -> f32 {
    ((h >> (k * 4)) & 255) as f32 / 255.0
}

fn is_wall(level: &Level, x: i32, y: i32) -> bool {
    matches!(level.tile((x, y)), Tile::Wall)
}

// ---------------------------------------------------------------------------------------------
// Walls: neighbouring wall tiles merge into one rounded block with a lit top and a darker side.

const WALL_GAP: f32 = 2.0;
const WALL_ROUND: f32 = 9.0;

/// A rectangle with the corner on side (sx, sy) rounded.
fn corner_rect(x0: f32, y0: f32, x1: f32, y1: f32, sx: i32, sy: i32, r: f32, col: Color) {
    let (w, h) = (x1 - x0, y1 - y0);
    if sx < 0 {
        draw_rectangle(x0 + r, y0, w - r, h, col);
    } else {
        draw_rectangle(x0, y0, w - r, h, col);
    }
    let bx = if sx < 0 { x0 } else { x1 - r };
    if sy < 0 {
        draw_rectangle(bx, y0 + r, r, h - r, col);
    } else {
        draw_rectangle(bx, y0, r, h - r, col);
    }
    let cx = if sx < 0 { x0 + r } else { x1 - r };
    let cy = if sy < 0 { y0 + r } else { y1 - r };
    quarter_circle(cx, cy, r, sx as f32, sy as f32, col);
}

fn wall_shape(level: &Level, x: i32, y: i32, off: Vec2, col: Color) {
    let px = x as f32 * TILE + off.x;
    let py = y as f32 * TILE + off.y;
    let h = TILE / 2.0;
    for (sx, sy) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
        let hx = is_wall(level, x + sx, y);
        let hy = is_wall(level, x, y + sy);
        let gx = if hx { 0.0 } else { WALL_GAP };
        let gy = if hy { 0.0 } else { WALL_GAP };
        let (x0, x1) = if sx < 0 {
            (px + gx, px + h)
        } else {
            (px + h, px + TILE - gx)
        };
        let (y0, y1) = if sy < 0 {
            (py + gy, py + h)
        } else {
            (py + h, py + TILE - gy)
        };
        if !hx && !hy {
            corner_rect(x0, y0, x1, y1, sx, sy, WALL_ROUND, col);
        } else {
            draw_rectangle(x0, y0, x1 - x0, y1 - y0, col);
        }
    }
}

fn wall_top_color(pal: &Palette, x: i32, y: i32, t: f32) -> Color {
    match pal.theme {
        Theme::Acid => hsl(x as f32 * 0.035 + y as f32 * 0.05 + t * 0.06, 0.75, 0.62),
        _ => pal.wall_top,
    }
}

/// The printed/moulded pattern on top of a wall tile.
fn wall_pattern(level: &Level, x: i32, y: i32, pal: &Palette, t: f32) {
    let px = x as f32 * TILE;
    let py = y as f32 * TILE;
    let (cx, cy) = (px + TILE / 2.0, py + TILE / 2.0);
    let h = hash(x, y);
    let exposed_top = !is_wall(level, x, y - 1);
    let pat = pal.pattern;
    match pal.theme {
        Theme::Joint => {
            // Rolling-paper print: fine lines and a little leaf now and then.
            for k in 0..3 {
                let yy = py + 7.0 + k as f32 * 9.0;
                draw_line(px + 3.0, yy, px + TILE - 3.0, yy, 0.6, alpha(pat, 0.6));
            }
            if h % 3 == 0 {
                // A faint printed leaf, in the paper's own ink so it never looks like a pickup.
                let ink = alpha(pat, 0.9);
                for (a, l) in [
                    (-1.2f32, 5.0f32),
                    (-0.6, 7.0),
                    (0.0, 8.5),
                    (0.6, 7.0),
                    (1.2, 5.0),
                ] {
                    art::leaflet(vec2(cx, cy + 4.0), a, l, 1.6, ink, ink);
                }
            }
        }
        Theme::RedBong => {
            ellipse(cx - 5.0, cy - 4.0, 2.0, 7.0, 0.5, alpha(pal.wall_hi, 0.35));
            for k in 0..2 {
                let ph = (t * 0.3 + unit(h, k)).fract();
                ring(
                    px + 8.0 + unit(h, k + 2) * 16.0,
                    py + 28.0 - ph * 24.0,
                    1.5 + unit(h, k + 4) * 1.5,
                    0.8,
                    alpha(pat, 0.7 * (1.0 - ph)),
                );
            }
        }
        Theme::GoldPipe => {
            draw_line(
                px + 4.0,
                py + 22.0,
                px + 14.0,
                py + 8.0,
                1.6,
                alpha(pal.wall_hi, 0.55),
            );
            draw_line(
                px + 12.0,
                py + 26.0,
                px + 24.0,
                py + 10.0,
                1.0,
                alpha(pal.wall_hi, 0.4),
            );
            if h % 2 == 0 {
                circle(cx + 6.0, cy + 6.0, 1.6, alpha(pal.wall_side, 0.8));
                circle(cx + 5.6, cy + 5.6, 0.8, alpha(pal.wall_hi, 0.9));
            }
        }
        Theme::Acid => {
            let col = alpha(WHITE, 0.55);
            if h % 4 == 0 {
                circle(cx, cy, 5.5, alpha(c(255, 230, 60), 0.95));
                circle(cx - 1.8, cy - 1.2, 0.8, INK);
                circle(cx + 1.8, cy - 1.2, 0.8, INK);
                let mut pts = Vec::new();
                for i in 0..=6 {
                    let a = 0.5 + i as f32 / 6.0 * 2.14;
                    pts.push(vec2(cx + a.cos() * 3.0, cy + a.sin() * 2.6 + 0.4));
                }
                polyline(&pts, 0.7, INK);
            } else {
                ring(cx, cy, 6.0 + (t * 2.0 + h as f32).sin() * 1.5, 1.0, col);
            }
        }
        Theme::Mushrooms => {
            for k in 0..3 {
                circle(
                    px + 6.0 + unit(h, k) * 20.0,
                    py + 6.0 + unit(h, k + 3) * 20.0,
                    2.0,
                    alpha(pal.wall_hi, 0.5),
                );
            }
            if h % 4 == 0 {
                let (mx, my) = (cx + 3.0, cy + 2.0);
                rounded_rect(mx - 1.4, my - 1.0, 2.8, 5.0, 1.0, c(240, 230, 210));
                let mut cap = vec![vec2(mx - 5.0, my)];
                for i in 0..=10 {
                    let a = std::f32::consts::PI * (1.0 + i as f32 / 10.0);
                    cap.push(vec2(mx + a.cos() * 5.0, my + a.sin() * 4.0));
                }
                fan(vec2(mx, my - 1.5), &cap, c(220, 50, 60));
                circle(mx - 1.8, my - 2.0, 0.8, WHITE);
                circle(mx + 1.6, my - 2.6, 0.7, WHITE);
            }
        }
        Theme::Hookah => {
            let d = 8.0;
            let pts = [
                vec2(cx, cy - d),
                vec2(cx + d, cy),
                vec2(cx, cy + d),
                vec2(cx - d, cy),
            ];
            outline(&pts, 1.2, alpha(pat, 0.9));
            circle(cx, cy, 1.6, alpha(pat, 0.9));
        }
        Theme::Brownies => {
            for k in 0..3 {
                let a = vec2(
                    px + 4.0 + unit(h, k) * 22.0,
                    py + 5.0 + unit(h, k + 3) * 20.0,
                );
                let b = a + vec2(4.0 + unit(h, k + 1) * 4.0, -1.0 + unit(h, k + 5) * 4.0);
                draw_line(a.x, a.y, b.x, b.y, 0.9, alpha(pat, 0.9));
            }
            if exposed_top {
                for k in 0..4 {
                    circle(
                        px + 5.0 + k as f32 * 7.0,
                        py + 4.0 + (k % 2) as f32,
                        1.8,
                        alpha(c(80, 200, 90), 0.7),
                    );
                }
            }
        }
        Theme::Blunt => {
            for k in 0..3 {
                let o = k as f32 * 10.0;
                draw_line(px + o, py + TILE, px + o + 12.0, py, 1.0, alpha(pat, 0.8));
            }
        }
        Theme::Peyote => {
            for k in 0..3 {
                let yy = py + 8.0 + k as f32 * 8.0 + (x as f32 * 1.3 + k as f32).sin() * 1.5;
                draw_line(
                    px + 2.0,
                    yy,
                    px + TILE - 2.0,
                    yy + (x as f32 * 0.7).cos() * 1.5,
                    1.2,
                    alpha(pat, 0.6),
                );
            }
        }
        Theme::SpaceCake => {
            for k in 0..5 {
                let q = vec2(
                    px + 5.0 + unit(h, k) * 22.0,
                    py + 5.0 + unit(h, k + 2) * 22.0,
                );
                let a = unit(h, k + 4) * 3.0;
                let col = hsl(unit(h, k + 1), 0.85, 0.6);
                draw_line(q.x, q.y, q.x + a.cos() * 3.0, q.y + a.sin() * 3.0, 1.4, col);
            }
        }
    }
}

fn draw_walls(level: &Level, pal: &Palette, t: f32) {
    let walls: Vec<(i32, i32)> = (0..H as i32)
        .flat_map(|y| (0..W as i32).map(move |x| (x, y)))
        .filter(|&(x, y)| is_wall(level, x, y))
        .collect();
    let shadow = alpha(BLACK, 0.45);
    for &(x, y) in &walls {
        wall_shape(level, x, y, vec2(3.0, 6.0), shadow);
    }
    for &(x, y) in &walls {
        wall_shape(level, x, y, vec2(0.0, 4.0), INK);
    }
    for &(x, y) in &walls {
        let side = match pal.theme {
            Theme::Acid => darker(wall_top_color(pal, x, y, t), 0.45),
            _ => pal.wall_side,
        };
        wall_shape(level, x, y, vec2(0.0, 3.0), side);
    }
    for &(x, y) in &walls {
        wall_shape(level, x, y, vec2(0.0, -0.6), INK);
    }
    for &(x, y) in &walls {
        wall_shape(
            level,
            x,
            y,
            Vec2::ZERO,
            lighter(wall_top_color(pal, x, y, t), 0.25),
        );
    }
    for &(x, y) in &walls {
        wall_shape(level, x, y, vec2(0.0, 1.4), wall_top_color(pal, x, y, t));
    }
    for &(x, y) in &walls {
        wall_pattern(level, x, y, pal, t);
    }
}

/// The corridor floor: a gradient with a few themed specks.
fn draw_floor(pal: &Palette, t: f32) {
    vgradient(0.0, 0.0, LW, FIELD_H, pal.floor_top, pal.floor_bottom);
    glow(
        LW / 2.0,
        FIELD_H * 0.45,
        LW * 0.55,
        alpha(lighter(pal.floor_top, 0.12), 0.5),
    );
    for i in 0..70 {
        let h = hash(i, 977);
        let x = unit(h, 0) * LW;
        let y = unit(h, 2) * FIELD_H;
        let tw = 0.5 + 0.5 * (t * (0.6 + unit(h, 4)) + i as f32).sin();
        match pal.theme {
            Theme::SpaceCake | Theme::Peyote => {
                circle(
                    x,
                    y,
                    0.6 + unit(h, 5) * 0.9,
                    alpha(pal.speck, 0.25 + 0.6 * tw),
                );
            }
            Theme::RedBong => {
                let ph = (t * 0.08 + unit(h, 5)).fract();
                ring(
                    x,
                    (y - ph * FIELD_H).rem_euclid(FIELD_H),
                    1.5 + unit(h, 6) * 2.0,
                    0.8,
                    alpha(pal.speck, 0.3),
                );
            }
            Theme::Mushrooms => {
                let d = vec2(
                    (t * 0.3 + i as f32).sin(),
                    (t * 0.25 + i as f32 * 0.7).cos(),
                ) * 6.0;
                glow(x + d.x, y + d.y, 4.0, alpha(pal.speck, 0.5 * tw));
            }
            Theme::Acid => {
                if i < 8 {
                    ring(
                        x,
                        y,
                        30.0 + 20.0 * tw,
                        6.0,
                        alpha(hsl(t * 0.1 + unit(h, 3), 0.8, 0.5), 0.07),
                    );
                }
            }
            _ => circle(x, y, 1.0, alpha(pal.speck, 0.25)),
        }
    }
}

/// The donut shop the cops start in (and get sent back to).
fn draw_donut_shop(level: &Level, t: f32) {
    let mut lo = (W as i32, H as i32);
    let mut hi = (-1, -1);
    for y in 0..H as i32 {
        for x in 0..W as i32 {
            if matches!(level.tile((x, y)), Tile::Pen) {
                lo = (lo.0.min(x), lo.1.min(y));
                hi = (hi.0.max(x), hi.1.max(y));
            }
        }
    }
    if hi.0 < 0 {
        return;
    }
    let (x0, y0) = (lo.0 as f32 * TILE, lo.1 as f32 * TILE);
    let (w, h) = (
        (hi.0 - lo.0 + 1) as f32 * TILE,
        (hi.1 - lo.1 + 1) as f32 * TILE,
    );
    rounded_rect(x0 + 1.0, y0 + 1.0, w - 2.0, h - 2.0, 6.0, c(255, 236, 244));
    // Checked shop floor.
    let n = 8.0;
    for j in 0..(h / n) as i32 {
        for i in 0..(w / n) as i32 {
            if (i + j) % 2 == 0 {
                draw_rectangle(
                    x0 + i as f32 * n + 1.0,
                    y0 + j as f32 * n + 1.0,
                    n - 0.5,
                    n - 0.5,
                    c(250, 196, 214),
                );
            }
        }
    }
    // A big donut sign in the middle.
    let (cx, cy) = (x0 + w / 2.0, y0 + h / 2.0 + 4.0);
    circle(cx, cy + 1.5, 15.0, INK);
    circle(cx, cy, 14.0, c(222, 160, 90));
    circle(cx, cy - 1.0, 12.0, c(255, 110, 180));
    for k in 0..10 {
        let a = k as f32 * 0.63;
        let col = hsl(k as f32 * 0.17, 0.9, 0.6);
        let q = vec2(cx + a.cos() * 8.5, cy - 1.0 + a.sin() * 8.5);
        draw_line(
            q.x,
            q.y,
            q.x + (a + 1.0).cos() * 2.4,
            q.y + (a + 1.0).sin() * 2.4,
            1.3,
            col,
        );
    }
    circle(cx, cy, 4.6, INK);
    circle(cx, cy, 3.8, c(250, 196, 214));
    // Striped awning on top.
    let stripes = 8;
    let sw = w / stripes as f32;
    for i in 0..stripes {
        let col = if i % 2 == 0 { c(255, 80, 150) } else { WHITE };
        draw_rectangle(x0 + i as f32 * sw, y0 - 2.0, sw, 7.0, col);
        circle(x0 + i as f32 * sw + sw / 2.0, y0 + 5.0, sw / 2.0, col);
    }
    rounded_rect_lines(x0 + 1.0, y0 + 1.0, w - 2.0, h - 2.0, 6.0, 2.0, INK);
    let wob = (t * 3.0).sin() * 0.5;
    text_centered(
        Face::Display,
        "DONUTS",
        cx,
        y0 + h - 4.0 + wob,
        9.0,
        c(120, 40, 80),
    );
}

/// The exit: police tape until every leaf is gone, then a glowing way out.
fn draw_gate(px: f32, py: f32, open: bool, t: f32) {
    if open {
        let g = 0.5 + 0.5 * (t * 5.0).sin();
        glow(
            px + TILE / 2.0,
            py + TILE / 2.0,
            30.0 + g * 6.0,
            alpha(c(120, 255, 140), 0.55),
        );
        rounded_rect(px + 3.0, py + 2.0, TILE - 6.0, TILE - 2.0, 5.0, INK);
        rounded_rect(
            px + 5.0,
            py + 4.0,
            TILE - 10.0,
            TILE - 6.0,
            4.0,
            c(140, 255, 160),
        );
        let cx = px + TILE / 2.0;
        let lift = g * 2.0;
        let arrow = [
            vec2(cx, py + 7.0 - lift),
            vec2(cx + 7.0, py + 15.0 - lift),
            vec2(cx + 2.6, py + 15.0 - lift),
            vec2(cx + 2.6, py + 25.0 - lift),
            vec2(cx - 2.6, py + 25.0 - lift),
            vec2(cx - 2.6, py + 15.0 - lift),
            vec2(cx - 7.0, py + 15.0 - lift),
        ];
        outline(&arrow, 1.6, INK);
        fan(vec2(cx, py + 15.0 - lift), &arrow, c(20, 110, 40));
    } else {
        draw_rectangle(px, py, TILE, TILE, c(20, 20, 26));
        for (k, yy) in [(0, 9.0f32), (1, 20.0)] {
            let a = vec2(px - 2.0, py + yy + 3.0 * k as f32);
            let b = vec2(px + TILE + 2.0, py + yy - 3.0 + 3.0 * k as f32);
            stroke(a, b, 8.0, INK);
            stroke(a, b, 6.4, c(255, 214, 40));
            for i in 0..3 {
                let q = a + (b - a) * (0.2 + i as f32 * 0.3);
                draw_line(q.x - 2.0, q.y - 2.2, q.x + 1.0, q.y + 2.2, 1.6, INK);
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Effects

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
                life: 0.6,
                max: 0.6,
                color,
                size: 1.8 + (i % 3) as f32,
            });
        }
    }

    pub fn floater(&mut self, pos: Vec2, text: String, color: Color) {
        self.floaters.push((pos, text, 1.2, color));
    }

    pub fn update(&mut self, dt: f32) {
        for p in &mut self.particles {
            p.life -= dt;
            p.pos += p.vel * dt;
            p.vel *= 0.93;
            p.vel.y += 30.0 * dt;
        }
        self.particles.retain(|p| p.life > 0.0);
        for f in &mut self.floaters {
            f.2 -= dt;
            f.0.y -= 24.0 * dt;
        }
        self.floaters.retain(|f| f.2 > 0.0);
        self.shake = (self.shake - dt * 2.2).max(0.0);
    }

    pub fn draw(&self) {
        for p in &self.particles {
            let k = p.life / p.max;
            circle(p.pos.x, p.pos.y, p.size * k + 0.6, alpha(INK, k * 0.6));
            circle(p.pos.x, p.pos.y, p.size * k, alpha(p.color, k));
        }
        for (pos, text, life, color) in &self.floaters {
            let a = (*life * 1.6).min(1.0);
            let pop = 1.0
                + ((1.2 - life) * 12.0).min(1.0) * 0.0
                + (1.0 - ((1.2 - life) * 6.0).min(1.0)) * 0.4;
            text_outlined(
                Face::Display,
                text,
                pos.x,
                pos.y,
                15.0 * pop,
                alpha(*color, a),
                alpha(INK, a),
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The playfield

/// Draw the whole playfield (tiles, pickups, actors, effects) in logical coordinates.
pub fn draw_playfield(world: &World, time: f64, pal: &Palette, fx: &Effects) {
    let t = time as f32;

    draw_floor(pal, t);
    draw_donut_shop(&world.level, t);
    for y in 0..H as i32 {
        for x in 0..W as i32 {
            if matches!(world.level.tile((x, y)), Tile::Gate) {
                draw_gate(x as f32 * TILE, y as f32 * TILE, world.gate_open, t);
            }
        }
    }
    draw_walls(&world.level, pal, t);

    // Pickups.
    for y in 0..H {
        for x in 0..W {
            let cx = x as f32 * TILE + TILE / 2.0;
            let cy = y as f32 * TILE + TILE / 2.0;
            if world.leaves[y][x] {
                let col = if pal.theme == Theme::Acid {
                    hsl(t * 0.2 + (x + y) as f32 * 0.05, 0.75, 0.55)
                } else {
                    pal.leaf
                };
                art::draw_bud(cx, cy, time, (x * 7 + y * 3) as f32, col);
            }
            if world.joint_tiles[y][x] {
                let bounce = (t * 3.0 + x as f32).sin() * 1.5;
                glow(
                    cx,
                    cy,
                    20.0,
                    alpha(pal.accent, 0.35 + 0.15 * (t * 4.0).sin()),
                );
                art::draw_item(pal.theme, cx, cy + bounce, 0.9, time);
            }
        }
    }
    // Cops.
    for (i, w) in world.cops.iter().enumerate() {
        let (x, y) = w.mover.pos_f();
        let mood = match w.state {
            CopState::Stunned { until } => {
                let left = until.saturating_sub(world.t);
                CopMood::Dazed {
                    left: left as f32 / stun_ticks(world.level_index) as f32,
                    ending: left < 3 * TICKS_PER_SEC,
                }
            }
            CopState::Waiting { .. } => CopMood::Waiting,
            CopState::Hunting => CopMood::Hunting,
        };
        art::draw_cop(
            x * TILE + TILE / 2.0,
            y * TILE + TILE / 2.0,
            w.kind,
            mood,
            w.mover.dir,
            time,
            pal.theme,
            i,
        );
    }
    if let Some(p) = world.pulse {
        let age = p.age as f32 / PULSE_TICKS as f32;
        art::draw_stun_cloud(
            pal.theme,
            p.center.0 * TILE + TILE / 2.0,
            p.center.1 * TILE + TILE / 2.0,
            age,
            STUN_RADIUS * TILE,
            time,
        );
    }
    // Giddy on top.
    let (px, py) = world.player.pos_f();
    let (cx, cy) = (px * TILE + TILE / 2.0, py * TILE + TILE / 2.0 - 4.0);
    match world.phase {
        Phase::Dying(n) => {
            let k = 1.0 - n as f32 / DYING_TICKS as f32;
            let s = 0.95 * (1.0 - k * 0.9);
            art::draw_giddy(cx, cy - k * 16.0, None, s, time, false, true);
        }
        _ => {
            let moving = world.player.dir.is_some() && matches!(world.phase, Phase::Playing);
            art::draw_giddy(cx, cy, world.player.dir, 0.95, time, moving, false);
        }
    }
    fx.draw();
    draw_smoke_timer(world, time, pal);
}

/// "COPS SMOKED OUT" with a bar that runs down while the smoke lasts, flashing near the end.
fn draw_smoke_timer(world: &World, time: f64, pal: &Palette) {
    let Some((left, total)) = world.smoked_out() else {
        return;
    };
    let k = left as f32 / total.max(1) as f32;
    let secs = left.div_ceil(TICKS_PER_SEC);
    let ending = left < 3 * TICKS_PER_SEC;
    let flash = ending && ((time * 10.0).sin() > 0.0);
    let (w, h) = (220.0, 26.0);
    let (x, y) = (LW / 2.0 - w / 2.0, 40.0);
    rounded_rect(x - 2.0, y - 2.0, w + 4.0, h + 4.0, h / 2.0 + 2.0, INK);
    rounded_rect(x, y, w, h, h / 2.0, alpha(c(30, 30, 50), 0.92));
    let fill = if flash {
        WHITE
    } else {
        art::theme_smoke(pal.theme, 0.0, time as f32)
    };
    if k > 0.0 {
        rounded_rect(x, y, (w * k).max(h), h, h / 2.0, alpha(fill, 0.9));
    }
    let label = format!("COPS SMOKED OUT  {secs}");
    text_centered(
        Face::Display,
        &label,
        LW / 2.0,
        y + 19.0,
        15.0,
        if flash { c(255, 80, 80) } else { INK },
    );
}

// ---------------------------------------------------------------------------------------------
// HUD and banners

pub fn draw_hud(world: &World, time: f64, pal: &Palette) {
    let top = FIELD_H;
    let hh = LH - top;

    vgradient(0.0, top, LW, hh, c(30, 34, 60), c(12, 14, 28));
    draw_rectangle(0.0, top, LW, 2.0, INK);
    draw_rectangle(0.0, top + 2.0, LW, 1.5, alpha(pal.accent, 0.8));
    let label = c(170, 184, 230);
    let value = c(250, 250, 255);
    let ly = top + 20.0;
    let vy = top + 52.0;

    text(Face::Body, "TIME", 16.0, ly, 13.0, label);
    let secs = world.elapsed_seconds() as u32;
    text(
        Face::Display,
        &format!("{}:{:02}", secs / 60, secs % 60),
        16.0,
        vy,
        26.0,
        value,
    );

    text(Face::Body, pal.theme.short(), 100.0, ly, 13.0, label);
    for i in 0..MAX_JOINTS {
        let x = 112.0 + i as f32 * 30.0;
        rounded_rect(x - 13.0, top + 27.0, 26.0, 30.0, 6.0, alpha(BLACK, 0.35));
        if i < world.joints {
            art::draw_item(pal.theme, x, top + 42.0, 0.62, time);
        } else {
            rounded_rect_lines(
                x - 13.0,
                top + 27.0,
                26.0,
                30.0,
                6.0,
                1.0,
                alpha(label, 0.3),
            );
        }
    }

    text(Face::Body, "SCORE", 222.0, ly, 13.0, label);
    text(
        Face::Display,
        &format!("{}", world.score),
        222.0,
        vy,
        26.0,
        value,
    );

    let lvl = format!("LEVEL {}", world.level_index + 1);
    text(Face::Body, &lvl, 350.0, ly, 13.0, label);
    let lw = text_width(Face::Body, &lvl, 13.0);
    text(
        Face::Body,
        &world.level.name,
        358.0 + lw,
        ly,
        13.0,
        pal.accent,
    );
    let total = world.level.leaf_count().max(1) as f32;
    let done = 1.0 - world.leaves_left as f32 / total;
    rounded_rect(350.0, top + 28.0, 160.0, 11.0, 5.5, alpha(BLACK, 0.5));
    if done > 0.0 {
        rounded_rect(
            350.0,
            top + 28.0,
            (160.0 * done).max(11.0),
            11.0,
            5.5,
            pal.leaf,
        );
        rounded_rect(
            353.0,
            top + 29.5,
            (154.0 * done).max(5.0),
            3.0,
            1.5,
            alpha(WHITE, 0.35),
        );
    }
    let (status, col) = if world.gate_open {
        ("THE COAST IS CLEAR - GO!".to_string(), c(130, 255, 150))
    } else {
        (format!("{} leaves to go", world.leaves_left), label)
    };
    text(Face::Body, &status, 350.0, top + 56.0, 13.0, col);

    text(Face::Body, "LIVES", 540.0, ly, 13.0, label);
    for i in 0..world.lives.min(3) {
        art::draw_giddy(
            556.0 + i as f32 * 26.0,
            top + 44.0,
            None,
            0.42,
            time + i as f64,
            false,
            false,
        );
    }
    if world.lives > 3 {
        text(
            Face::Display,
            &format!("+{}", world.lives - 3),
            620.0,
            top + 52.0,
            16.0,
            value,
        );
    }
}

fn banner(cy: f32, h: f32) {
    vgradient(
        0.0,
        cy - h / 2.0,
        LW,
        h / 2.0,
        alpha(BLACK, 0.2),
        alpha(BLACK, 0.75),
    );
    vgradient(0.0, cy, LW, h / 2.0, alpha(BLACK, 0.75), alpha(BLACK, 0.2));
}

/// Banner text drawn over the playfield for each phase.
pub fn draw_overlay(world: &World, time: f64, pal: &Palette) {
    let cx = LW / 2.0;
    let cy = FIELD_H / 2.0;
    let t = time as f32;
    match world.phase {
        Phase::Ready(_) => {
            banner(cy, 120.0);
            let pulse = 1.0 + (t * 6.0).sin() * 0.03;
            art::draw_item(pal.theme, cx - 230.0, cy, 2.0, time);
            art::draw_item(pal.theme, cx + 230.0, cy, 2.0, time + 0.5);
            text_outlined(
                Face::Display,
                &format!("LEVEL {}", world.level_index + 1),
                cx,
                cy - 22.0,
                22.0,
                WHITE,
                INK,
            );
            text_outlined(
                Face::Display,
                &world.level.name.to_uppercase(),
                cx,
                cy + 12.0,
                34.0 * pulse,
                pal.accent,
                INK,
            );
            text_centered(
                Face::Body,
                &format!("Grab the {}. {}", pal.theme.item(), pal.theme.effect()),
                cx,
                cy + 40.0,
                14.0,
                c(235, 240, 255),
            );
        }
        Phase::Cleared(_) => {
            banner(cy, 110.0);
            text_outlined(
                Face::Display,
                "MUNCHIES TIME!",
                cx,
                cy + 4.0,
                46.0,
                c(130, 255, 150),
                INK,
            );
            text_centered(
                Face::Body,
                &format!("Score {}", world.score),
                cx,
                cy + 36.0,
                18.0,
                WHITE,
            );
        }
        Phase::GameOver => {
            banner(cy, 110.0);
            text_outlined(
                Face::Display,
                "BUSTED!",
                cx,
                cy + 6.0,
                56.0,
                c(255, 96, 96),
                INK,
            );
            text_centered(
                Face::Body,
                &format!("Final score {}", world.score),
                cx,
                cy + 38.0,
                18.0,
                WHITE,
            );
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------------------------
// Menus

pub fn centered(s: &str, cx: f32, y: f32, size: f32, color: Color) {
    text_centered(Face::Body, s, cx, y, size, color);
}

/// A screen heading in the chunky display face.
pub fn heading(s: &str, cx: f32, y: f32, size: f32) {
    text_outlined(Face::Display, s, cx, y, size, title_pink(), INK);
}

pub fn dim() -> Color {
    c(200, 210, 255)
}

pub fn bright() -> Color {
    WHITE
}

pub fn accent() -> Color {
    c(255, 220, 90)
}

pub fn title_pink() -> Color {
    c(255, 86, 196)
}

pub fn leaf_green() -> Color {
    c(130, 255, 150)
}

/// The menu background: the original's royal blue, deepened into a glow, with drifting leaves.
pub fn draw_backdrop(time: f64) {
    let t = time as f32;
    vgradient(0.0, 0.0, LW, LH, c(26, 40, 210), c(40, 10, 120));
    glow(LW * 0.5, LH * 0.35, LW * 0.6, alpha(c(90, 120, 255), 0.5));
    for i in 0..16 {
        let h = hash(i, 31);
        let speed = 10.0 + unit(h, 0) * 16.0;
        let x = (unit(h, 1) * (LW + 80.0) + (t * 0.3 + i as f32).sin() * 20.0)
            .rem_euclid(LW + 80.0)
            - 40.0;
        let y = (unit(h, 2) * (LH + 80.0) - t * speed).rem_euclid(LH + 80.0) - 40.0;
        let a = (t * 0.5 + i as f32).sin() * 0.6;
        let s = 0.6 + unit(h, 3) * 0.8;
        let col = alpha(c(120, 255, 140), 0.12);
        for (k, l) in [
            (-1.3f32, 6.0f32),
            (-0.65, 9.0),
            (0.0, 11.0),
            (0.65, 9.0),
            (1.3, 6.0),
        ] {
            art::leaflet(vec2(x, y), a + k, l * s * 2.0, 2.6 * s * 2.0, col, col);
        }
    }
}

/// A rounded push-button. Returns its width so the caller can hit-test it.
pub fn button(
    label: &str,
    cx: f32,
    y: f32,
    size: f32,
    min_w: f32,
    selected: bool,
    time: f64,
) -> (f32, f32) {
    let w = text_width(Face::Body, label, size).max(min_w) + 36.0;
    let h = size * 1.55;
    let (x, top) = (cx - w / 2.0, y - size * 1.1);
    let lift = if selected {
        2.0 + (time as f32 * 6.0).sin() * 0.8
    } else {
        0.0
    };
    rounded_rect(x + 2.0, top + 4.0, w, h, h / 2.0, alpha(BLACK, 0.35));
    rounded_rect(
        x - 2.0,
        top - 2.0 - lift,
        w + 4.0,
        h + 4.0,
        h / 2.0 + 2.0,
        INK,
    );
    let face = if selected {
        c(130, 255, 150)
    } else {
        c(250, 250, 255)
    };
    rounded_rect(x, top - lift, w, h, h / 2.0, face);
    rounded_rect(
        x + 6.0,
        top + 3.0 - lift,
        w - 12.0,
        h * 0.32,
        h * 0.16,
        alpha(WHITE, 0.5),
    );
    let tw = text_width(Face::Body, label, size);
    text(Face::Body, label, cx - tw / 2.0, y - lift, size, INK);
    if selected {
        art::draw_bud(
            x - 14.0,
            y - size * 0.35 - lift,
            time,
            0.0,
            c(110, 240, 120),
        );
        art::draw_bud(
            x + w + 14.0,
            y - size * 0.35 - lift,
            time,
            1.0,
            c(110, 240, 120),
        );
    }
    (w, h)
}

pub fn panel(x: f32, y: f32, w: f32, h: f32) {
    rounded_rect(x + 4.0, y + 6.0, w, h, 16.0, alpha(BLACK, 0.4));
    rounded_rect(x - 3.0, y - 3.0, w + 6.0, h + 6.0, 18.0, INK);
    vgradient(x, y + 16.0, w, h - 32.0, c(40, 60, 220), c(30, 20, 140));
    rounded_rect(x, y, w, 18.0, 15.0, c(40, 60, 220));
    rounded_rect(x, y + h - 18.0, w, 18.0, 15.0, c(30, 20, 140));
}

/// The big Giddy and wavy title on the title screen.
pub fn draw_title_art(time: f64) {
    let t = time as f32;
    let bob = (t * 2.4).sin() * 4.0;
    art::draw_giddy(118.0, 92.0 + bob, Some(Dir::Right), 2.2, time, false, false);
    // Wavy, outlined title letters.
    let title = "GIDDY GANJA";
    let size = 62.0;
    let total = text_width(Face::Display, title, size);
    let mut x = LW / 2.0 + 70.0 - total / 2.0;
    for (i, ch) in title.chars().enumerate() {
        let s = ch.to_string();
        let w = text_width(Face::Display, &s, size);
        let y = 96.0 + (t * 3.0 + i as f32 * 0.6).sin() * 5.0;
        let col = if i < 5 {
            title_pink()
        } else {
            c(120, 250, 130)
        };
        text_outlined(Face::Display, &s, x + w / 2.0, y, size, col, INK);
        x += w;
    }
}

/// A small card for the level select: theme colours, the stash item and the place name.
pub fn level_card(i: usize, x: f32, y: f32, w: f32, h: f32, selected: bool, time: f64) {
    let pal = palette(i);
    let lift = if selected { 3.0 } else { 0.0 };
    rounded_rect(x + 2.0, y + 4.0, w, h, 10.0, alpha(BLACK, 0.35));
    rounded_rect(
        x - 2.0,
        y - 2.0 - lift,
        w + 4.0,
        h + 4.0,
        12.0,
        if selected { WHITE } else { INK },
    );
    vgradient(
        x,
        y + 8.0 - lift,
        w,
        h - 16.0,
        pal.floor_top,
        pal.floor_bottom,
    );
    rounded_rect(x, y - lift, w, 12.0, 10.0, pal.floor_top);
    rounded_rect(x, y + h - 12.0 - lift, w, 12.0, 10.0, pal.floor_bottom);
    rounded_rect(x, y - lift, w, 14.0, 10.0, pal.wall_top);
    draw_rectangle(x, y + 10.0 - lift, w, 4.0, pal.wall_side);
    art::draw_item(pal.theme, x + w - 20.0, y + h / 2.0 + 4.0 - lift, 0.8, time);
    text_outlined(
        Face::Display,
        &format!("{}", i + 1),
        x + 20.0,
        y + h / 2.0 + 14.0 - lift,
        22.0,
        WHITE,
        INK,
    );
    let name = giddy_ganja::themes::level_name(i);
    let size = (9.5 * (w - 8.0) / text_width(Face::Body, &name, 9.5)).min(9.5);
    text_centered(
        Face::Body,
        &name,
        x + w / 2.0,
        y + h - 5.0 - lift,
        size,
        lighter(pal.accent, 0.3),
    );
}
