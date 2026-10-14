//! The cast and the props, drawn as outlined cartoon shapes: Giddy the leaf, the cops, the little
//! leaves you collect, the stash item of every level, and the smoke that comes out of it.

use crate::gfx::*;
use giddy_ganja::ai::Personality;
use giddy_ganja::grid::Dir;
use giddy_ganja::themes::Theme;
use macroquad::models::{draw_mesh, Mesh, Vertex};
use macroquad::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

pub const INK: Color = Color::new(0.07, 0.10, 0.07, 1.0);

fn dir_vec(dir: Option<Dir>) -> Vec2 {
    dir.map_or(Vec2::ZERO, |d| {
        let (x, y) = d.delta();
        vec2(x as f32, y as f32)
    })
}

/// Unit vector for an angle measured clockwise from straight up.
fn up_angle(a: f32) -> Vec2 {
    vec2(a.sin(), -a.cos())
}

fn rot(v: Vec2, a: f32) -> Vec2 {
    let (s, c) = a.sin_cos();
    vec2(v.x * c - v.y * s, v.x * s + v.y * c)
}

// ---------------------------------------------------------------------------------------------
// Leaves

/// One serrated cannabis leaflet growing from `base` in direction `ang` (clockwise from up).
/// The left half is `light`, the right half `dark`, which gives it a folded, 3D look.
pub fn leaflet(base: Vec2, ang: f32, len: f32, half_w: f32, light: Color, dark: Color) {
    let dir = up_angle(ang);
    let n = vec2(-dir.y, dir.x);
    let m = ((len * view_scale() / 3.0) as usize).clamp(6, 16) & !1;
    let width = |t: f32| half_w * (t.powf(0.7) * (1.0 - t)) / 0.315;
    let mut vertices = Vec::with_capacity((m + 1) * 4);
    for k in 0..=m {
        let mut t = k as f32 / m as f32;
        let tooth = k % 2 == 1;
        let rib = base + dir * len * t;
        if tooth {
            t = (t + 0.03).min(1.0);
        }
        let w = width(t) * if tooth { 1.15 } else { 0.86 };
        let edge_base = base + dir * len * t;
        vertices.push(Vertex::new(rib.x, rib.y, 0.0, 0.0, 0.0, light));
        let l = edge_base - n * w;
        vertices.push(Vertex::new(l.x, l.y, 0.0, 0.0, 0.0, light));
        vertices.push(Vertex::new(rib.x, rib.y, 0.0, 0.0, 0.0, dark));
        let r = edge_base + n * w;
        vertices.push(Vertex::new(r.x, r.y, 0.0, 0.0, 0.0, dark));
    }
    let mut indices = Vec::with_capacity(m * 12);
    for k in 0..m as u16 {
        let a = k * 4;
        let b = a + 4;
        indices.extend_from_slice(&[a, b, b + 1, a, b + 1, a + 1]);
        indices.extend_from_slice(&[a + 2, b + 2, b + 3, a + 2, b + 3, a + 3]);
    }
    draw_mesh(&Mesh {
        vertices,
        indices,
        texture: None,
    });
}

/// A leaflet drawn twice: a slightly bigger dark silhouette first, so it gets an ink outline.
fn inked_leaflet(base: Vec2, ang: f32, len: f32, half_w: f32, light: Color, dark: Color, ink: f32) {
    leaflet(base, ang, len + ink * 1.6, half_w + ink, INK, INK);
    leaflet(base, ang, len, half_w, light, dark);
}

/// The little leaves you collect: five fingers, a stem and a soft shine.
pub fn draw_bud(cx: f32, cy: f32, time: f64, seed: f32, col: Color) {
    let pulse = 1.0 + (time as f32 * 2.6 + seed).sin() * 0.07;
    let base = vec2(cx, cy + 4.0);
    let light = lighter(col, 0.2);
    let dark = darker(col, 0.3);
    let fingers = [
        (-1.35f32, 5.0f32, 1.9f32),
        (-0.7, 7.0, 2.3),
        (0.0, 9.0, 2.6),
        (0.7, 7.0, 2.3),
        (1.35, 5.0, 1.9),
    ];
    stroke(base, base + vec2(0.0, 3.5), 1.6, INK);
    for (a, l, w) in fingers {
        leaflet(base, a, (l + 1.3) * pulse, w + 0.9, INK, INK);
    }
    for (a, l, w) in fingers {
        leaflet(base, a, l * pulse, w, light, dark);
    }
}

// ---------------------------------------------------------------------------------------------
// Giddy

/// One cartoon eye. `look` moves the pupil, `lid` (0..1) closes it from the top.
fn eye(x: f32, y: f32, rx: f32, ry: f32, look: Vec2, lid: f32, white: Color, lid_col: Color) {
    ellipse(x, y, rx + 0.9, ry + 0.9, 0.0, INK);
    ellipse(x, y, rx, ry, 0.0, white);
    let p = vec2(x, y) + look * vec2(rx * 0.42, ry * 0.4);
    circle(p.x, p.y + ry * 0.12, rx * 0.5, INK);
    circle(p.x - rx * 0.18, p.y - ry * 0.12, rx * 0.17, WHITE);
    if lid > 0.0 {
        // The lid: the top part of the eye, down to `cut`.
        let cut = y - ry + 2.0 * ry * lid;
        let n = 16;
        let mut pts = Vec::with_capacity(n + 2);
        for i in 0..=n {
            let a = PI + i as f32 / n as f32 * PI;
            let px = x + a.cos() * (rx + 0.9);
            let py = (y + a.sin() * (ry + 0.9)).min(cut);
            pts.push(vec2(px, py));
        }
        let half = ((cut - y) / ry).clamp(-1.0, 1.0);
        let w = (rx + 0.9) * (1.0 - half * half).max(0.0).sqrt();
        pts.push(vec2(x + w, cut));
        pts.push(vec2(x - w, cut));
        fan(vec2(x, (y - ry + cut) / 2.0), &pts, lid_col);
        draw_line(x - w, cut, x + w, cut, 1.1, INK);
    }
}

/// Giddy, a cannabis leaf with a face on the big middle leaflet and smaller leaflets for
/// arms and legs. `dizzy` swaps the eyes for spirals (when the cops catch you).
pub fn draw_giddy(
    cx: f32,
    cy: f32,
    facing: Option<Dir>,
    s: f32,
    time: f64,
    moving: bool,
    dizzy: bool,
) {
    let t = time as f32;
    let walk = if moving { t * 13.0 } else { 0.0 };
    let swing = if moving {
        walk.sin()
    } else {
        (t * 1.6).sin() * 0.25
    };
    let hop = if moving {
        -walk.sin().abs() * 1.6 * s
    } else {
        0.0
    };
    let look = dir_vec(facing);
    let tilt = look.x * 0.12 + (t * 1.3).sin() * 0.04;
    let base = vec2(cx, cy + 9.0 * s + hop);

    let light = c(88, 214, 92);
    let mid = c(50, 168, 64);
    let dark = c(30, 120, 46);
    let ink = 1.2 * s.max(0.6);

    // Soft shadow on the ground.
    ellipse(cx, cy + 15.5 * s, 11.0 * s, 3.0 * s, 0.0, alpha(BLACK, 0.3));

    // Stem.
    stroke(base, base + vec2(0.0, 5.0 * s), 3.2 * s, INK);
    stroke(base, base + vec2(0.0, 5.0 * s), 1.6 * s, dark);

    // Legs, then arms, then the upper leaflets, then the big head leaflet on top.
    let legs = [
        (-2.55 + swing * 0.32, 11.0, 4.2),
        (2.55 + swing * 0.32, 11.0, 4.2),
    ];
    let arms = [
        (-1.6 - swing * 0.35, 15.0, 4.8),
        (1.6 - swing * 0.35, 15.0, 4.8),
    ];
    let upper = [
        (-0.82 + tilt * 0.5, 19.0, 6.0),
        (0.82 + tilt * 0.5, 19.0, 6.0),
    ];
    for (a, l, w) in legs {
        inked_leaflet(base, a, l * s, w * s, mid, dark, ink);
        let tip = base + up_angle(a) * l * s;
        ellipse(
            tip.x + up_angle(a).x * 1.5 * s,
            tip.y + 0.5 * s,
            3.4 * s,
            2.0 * s,
            0.0,
            INK,
        );
        ellipse(
            tip.x + up_angle(a).x * 1.5 * s,
            tip.y,
            2.4 * s,
            1.3 * s,
            0.0,
            c(250, 250, 240),
        );
    }
    for (a, l, w) in arms {
        inked_leaflet(base, a, l * s, w * s, light, mid, ink);
    }
    for (a, l, w) in upper {
        inked_leaflet(base, a, l * s, w * s, light, mid, ink);
    }
    let head_len = 27.0 * s;
    inked_leaflet(base, tilt, head_len, 10.5 * s, light, mid, ink);
    // A vein down the middle.
    let dir = up_angle(tilt);
    stroke(
        base + dir * head_len * 0.66,
        base + dir * head_len * 0.92,
        0.9 * s,
        alpha(dark, 0.8),
    );

    // The face sits on the widest part of the head leaflet.
    let f = base + dir * head_len * 0.42;
    let at = |dx: f32, dy: f32| f + rot(vec2(dx * s, dy * s), tilt);
    let face_shift = look.x * 1.2 * s;
    let e1 = at(-3.9, -2.0) + vec2(face_shift, 0.0);
    let e2 = at(3.9, -2.0) + vec2(face_shift, 0.0);
    if dizzy {
        for e in [e1, e2] {
            ellipse(e.x, e.y, 3.4 * s, 3.8 * s, 0.0, INK);
            ellipse(e.x, e.y, 2.6 * s, 3.0 * s, 0.0, WHITE);
            let mut pts = Vec::new();
            for k in 0..20 {
                let a = k as f32 * 0.6 + t * 10.0;
                let r = k as f32 / 20.0 * 2.6 * s;
                pts.push(e + vec2(a.cos() * r, a.sin() * r));
            }
            polyline(&pts, 0.8 * s, INK);
        }
    } else {
        // Happy, heavy-lidded, a bit bloodshot.
        let white = c(255, 240, 236);
        let lid = 0.42 + (t * 0.7).sin().max(0.0).powi(30) * 0.5; // the occasional slow blink
        eye(e1.x, e1.y, 2.9 * s, 3.4 * s, look, lid, white, light);
        eye(e2.x, e2.y, 2.9 * s, 3.4 * s, look, lid, white, light);
        // Raised eyebrows.
        for (e, sgn) in [(e1, -1.0f32), (e2, 1.0)] {
            let a = e + vec2(-2.4 * s, -5.2 * s);
            let b = e + vec2(2.4 * s, -5.2 * s + sgn * 0.6 * s);
            stroke(a, b, 1.1 * s, darker(dark, 0.3));
        }
    }
    // Rosy cheeks.
    for e in [at(-6.3, 1.8), at(6.3, 1.8)] {
        circle(
            e.x + face_shift,
            e.y,
            1.9 * s,
            alpha(c(255, 120, 140), 0.45),
        );
    }
    // A huge grin with a tongue.
    let m = at(0.0, 2.6) + vec2(face_shift, 0.0);
    let n = 14;
    let mut pts = vec![m + rot(vec2(-5.2 * s, 0.0), tilt)];
    for i in 0..=n {
        let a = i as f32 / n as f32 * PI;
        pts.push(m + rot(vec2(-a.cos() * 5.2 * s, a.sin() * 4.6 * s), tilt));
    }
    let mouth_ink: Vec<Vec2> = pts.clone();
    outline(&mouth_ink, 1.6 * s, INK);
    fan(m + rot(vec2(0.0, 2.0 * s), tilt), &pts, c(90, 20, 30));
    let teeth = m + rot(vec2(0.0, 0.7 * s), tilt);
    ellipse(teeth.x, teeth.y, 4.4 * s, 1.0 * s, tilt, WHITE);
    let tongue = m + rot(vec2(0.8 * s, 3.1 * s), tilt);
    ellipse(tongue.x, tongue.y, 2.4 * s, 1.4 * s, tilt, c(240, 110, 130));
}

// ---------------------------------------------------------------------------------------------
// The cops

pub enum CopMood {
    Hunting,
    Waiting,
    /// Smoked out. `left` is the share of the effect still to go (1 to 0); `ending` flashes
    /// in the last few seconds.
    Dazed {
        left: f32,
        ending: bool,
    },
}

fn star(x: f32, y: f32, r: f32, rot0: f32, col: Color) {
    let mut pts = Vec::with_capacity(10);
    for i in 0..10 {
        let a = rot0 - FRAC_PI_2 + i as f32 * PI / 5.0;
        let rr = if i % 2 == 0 { r } else { r * 0.45 };
        pts.push(vec2(x + a.cos() * rr, y + a.sin() * rr));
    }
    fan(vec2(x, y), &pts, col);
}

/// A cartoon police officer. Each personality gets a different look so you can tell them apart.
pub fn draw_cop(
    cx: f32,
    cy: f32,
    kind: Personality,
    mood: CopMood,
    facing: Option<Dir>,
    time: f64,
    theme: Theme,
    id: usize,
) {
    let t = time as f32 + id as f32 * 1.7;
    let dazed = matches!(mood, CopMood::Dazed { .. });
    let moving = matches!(mood, CopMood::Hunting);
    let look = if dazed { Vec2::ZERO } else { dir_vec(facing) };
    let walk = if moving { t * 14.0 } else { t * 3.0 };
    let bob = if dazed {
        (t * 2.0).sin() * 1.2
    } else {
        -walk.sin().abs() * 1.2
    };
    let sway = if dazed { (t * 2.3).sin() * 0.12 } else { 0.0 };
    let cy = cy + bob;

    let (uniform, skin) = match kind {
        Personality::Hunter => (c(38, 60, 150), c(241, 194, 150)),
        Personality::Ambusher => (c(58, 92, 196), c(198, 134, 90)),
        Personality::Flanker => (c(34, 104, 150), c(255, 214, 178)),
        Personality::Prowler => (c(72, 62, 160), c(141, 88, 58)),
    };
    let uniform_dark = darker(uniform, 0.35);
    let chubby = if matches!(kind, Personality::Prowler) {
        1.15
    } else {
        1.0
    };

    ellipse(cx, cy + 14.5 - bob, 10.5, 3.0, 0.0, alpha(BLACK, 0.3));
    // Shoes.
    for (i, sx) in [-4.0f32, 4.0].into_iter().enumerate() {
        let step = if moving {
            (walk + i as f32 * PI).sin() * 1.6
        } else {
            0.0
        };
        ellipse(cx + sx, cy + 12.5 + step.min(0.0), 3.6, 2.2, 0.0, INK);
    }
    // Ponytail behind the head.
    if matches!(kind, Personality::Flanker) {
        let p = vec2(cx - look.x * 6.0, cy - 6.0);
        ellipse(
            p.x,
            p.y + 4.0,
            3.8,
            6.0,
            0.3 * if look.x > 0.0 { -1.0 } else { 1.0 },
            c(120, 70, 30),
        );
    }
    // Torso.
    ellipse(cx, cy + 4.0, 11.2 * chubby, 10.0, 0.0, INK);
    ellipse(cx, cy + 4.0, 10.0 * chubby, 8.8, 0.0, uniform);
    ellipse(cx - 3.0, cy + 1.0, 4.0, 3.0, -0.5, alpha(WHITE, 0.14));
    draw_rectangle(cx - 8.6 * chubby, cy + 7.6, 17.2 * chubby, 2.6, INK);
    rounded_rect(cx - 1.6, cy + 7.5, 3.2, 2.8, 0.6, c(240, 200, 70));
    star(cx - 4.4, cy + 2.2, 2.6, 0.0, c(255, 214, 70));
    for b in [0.0, 3.6] {
        circle(cx + 1.6, cy - 0.4 + b, 0.7, c(220, 220, 240));
    }
    // Arms (with a nightstick or a donut).
    let arm_swing = if moving { walk.sin() * 2.0 } else { 0.0 };
    for (sx, sw) in [(-1.0f32, arm_swing), (1.0, -arm_swing)] {
        let hand = vec2(cx + sx * 11.5 * chubby, cy + 7.0 + sw);
        stroke(vec2(cx + sx * 8.0, cy + 1.0), hand, 5.4, INK);
        stroke(vec2(cx + sx * 8.0, cy + 1.0), hand, 3.4, uniform_dark);
        circle(hand.x, hand.y, 2.4, INK);
        circle(hand.x, hand.y, 1.6, skin);
    }
    let right_hand = vec2(cx + 11.5 * chubby, cy + 7.0 - arm_swing);
    match kind {
        Personality::Hunter if !dazed => {
            stroke(
                right_hand + vec2(-1.0, 2.0),
                right_hand + vec2(4.0, -8.0),
                2.8,
                INK,
            );
            stroke(
                right_hand + vec2(-1.0, 2.0),
                right_hand + vec2(4.0, -8.0),
                1.4,
                c(70, 60, 50),
            );
        }
        Personality::Prowler => {
            circle(right_hand.x + 2.0, right_hand.y - 2.0, 4.0, INK);
            circle(
                right_hand.x + 2.0,
                right_hand.y - 2.0,
                3.2,
                c(255, 130, 190),
            );
            circle(right_hand.x + 2.0, right_hand.y - 2.0, 1.2, INK);
        }
        _ => {}
    }

    // Head.
    let h = vec2(cx + look.x * 0.8, cy - 7.0);
    circle(h.x, h.y, 8.2, INK);
    circle(h.x, h.y, 7.1, skin);
    // Ears.
    for sx in [-1.0f32, 1.0] {
        circle(h.x + sx * 7.4, h.y + 1.0, 1.8, INK);
        circle(h.x + sx * 7.4, h.y + 1.0, 1.1, skin);
    }
    let face = h + vec2(look.x * 1.5, look.y * 1.0 + 1.0);
    match mood {
        CopMood::Dazed { .. } => {
            // Red, droopy, very relaxed eyes and a big dopey smile.
            for sx in [-1.0f32, 1.0] {
                eye(
                    face.x + sx * 2.9,
                    face.y - 1.4,
                    2.1,
                    2.3,
                    vec2(0.0, 0.6),
                    0.55,
                    c(255, 170, 170),
                    skin,
                );
            }
            let m = face + vec2(0.0, 2.6);
            let mut pts = vec![];
            for i in 0..=10 {
                let a = i as f32 / 10.0 * PI;
                pts.push(m + vec2(-a.cos() * 3.4, a.sin() * 2.6));
            }
            fan(m + vec2(0.0, 1.0), &pts, c(110, 30, 40));
            ellipse(m.x + 0.6, m.y + 1.8, 1.5, 0.9, 0.0, c(240, 110, 130));
        }
        _ => {
            if matches!(kind, Personality::Ambusher) {
                // Mirrored aviators.
                for sx in [-1.0f32, 1.0] {
                    ellipse(face.x + sx * 3.0, face.y - 1.4, 2.9, 2.2, 0.0, INK);
                    ellipse(
                        face.x + sx * 3.0 - 0.7,
                        face.y - 2.1,
                        1.0,
                        0.6,
                        -0.4,
                        alpha(WHITE, 0.6),
                    );
                }
                draw_line(
                    face.x - 1.0,
                    face.y - 2.0,
                    face.x + 1.0,
                    face.y - 2.0,
                    0.8,
                    INK,
                );
            } else {
                for sx in [-1.0f32, 1.0] {
                    circle(
                        face.x + sx * 2.9 + look.x * 0.6,
                        face.y - 1.4 + look.y * 0.6,
                        1.2,
                        INK,
                    );
                }
            }
            // Stern brows.
            for sx in [-1.0f32, 1.0] {
                stroke(
                    vec2(face.x + sx * 1.3, face.y - 3.6),
                    vec2(face.x + sx * 4.5, face.y - 4.6),
                    1.1,
                    INK,
                );
            }
            match kind {
                Personality::Hunter => {
                    // A proper moustache.
                    ellipse(face.x - 1.9, face.y + 2.0, 2.4, 1.2, 0.25, c(90, 54, 30));
                    ellipse(face.x + 1.9, face.y + 2.0, 2.4, 1.2, -0.25, c(90, 54, 30));
                }
                Personality::Flanker => {
                    for sx in [-1.0f32, 1.0] {
                        for k in 0..3 {
                            circle(
                                face.x + sx * (3.6 + k as f32 * 0.7),
                                face.y + 0.8 + (k % 2) as f32 * 0.6,
                                0.35,
                                c(170, 100, 70),
                            );
                        }
                    }
                    stroke(
                        vec2(face.x - 1.6, face.y + 3.0),
                        vec2(face.x + 1.6, face.y + 3.0),
                        0.9,
                        INK,
                    );
                }
                _ => {
                    stroke(
                        vec2(face.x - 1.8, face.y + 3.2),
                        vec2(face.x + 1.8, face.y + 2.8),
                        1.0,
                        INK,
                    );
                }
            }
        }
    }
    // The cap: crown, band, visor and badge. It slides askew when dazed.
    let hat = h + vec2(if dazed { 2.0 } else { 0.0 }, -5.0);
    let tilt = sway + if dazed { 0.25 } else { 0.0 };
    let p = |x: f32, y: f32| hat + rot(vec2(x, y), tilt);
    let crown = [
        p(-7.0, 0.0),
        p(-9.5, -6.0),
        p(-7.0, -8.2),
        p(7.0, -8.2),
        p(9.5, -6.0),
        p(7.0, 0.0),
    ];
    outline(&crown, 2.2, INK);
    fan(p(0.0, -4.0), &crown, c(26, 34, 74));
    let band = [p(-7.2, 0.0), p(-7.6, -2.4), p(7.6, -2.4), p(7.2, 0.0)];
    fan(p(0.0, -1.2), &band, INK);
    let vis = p(look.x * 1.5, 0.6);
    ellipse(vis.x, vis.y, 8.0, 2.2, tilt, INK);
    let badge = p(look.x * 1.5, -4.8);
    star(badge.x, badge.y, 2.2, tilt, c(255, 214, 70));

    if let CopMood::Dazed { left, ending } = mood {
        dazed_aura(theme, h, t);
        // A timer ring over the cap that empties as the smoke clears, then flashes like a
        // Pac-Man ghost about to recover.
        let flash = ending && (t * 10.0).sin() > 0.0;
        let (rx, ry) = (cx, cy - 25.0);
        circle(rx, ry, 6.2, alpha(INK, 0.75));
        let col = if flash {
            WHITE
        } else {
            theme_smoke(theme, 0.0, t)
        };
        let n = 24;
        let mut pts = vec![vec2(rx, ry)];
        for i in 0..=n {
            let a = -FRAC_PI_2 + left.clamp(0.0, 1.0) * TAU * i as f32 / n as f32;
            pts.push(vec2(rx + a.cos() * 5.0, ry + a.sin() * 5.0));
        }
        if left > 0.0 {
            fan(vec2(rx, ry), &pts[1..], col);
            fan(vec2(rx, ry), &[pts[0], pts[1], pts[2]], col);
        }
        if flash {
            ring(cx, cy, 16.0, 2.0, alpha(WHITE, 0.9));
        }
    }
}

/// What floats around a dazed cop's head depends on what got them.
fn dazed_aura(theme: Theme, h: Vec2, t: f32) {
    let smoke = theme_smoke(theme, 0.0, t);
    for i in 0..4 {
        let a = t * 1.8 + i as f32 * FRAC_PI_2;
        let p = h + vec2(a.cos() * 11.0, -8.0 + a.sin() * 3.0);
        match theme {
            Theme::Acid => circle(p.x, p.y, 1.8, hsl(t * 0.5 + i as f32 * 0.25, 0.9, 0.6)),
            Theme::Mushrooms | Theme::Peyote | Theme::SpaceCake => {
                star(p.x, p.y, 2.6, t * 3.0, alpha(lighter(smoke, 0.3), 0.95));
            }
            _ => {
                let rise = (t * 0.7 + i as f32 * 0.25).fract();
                circle(
                    p.x,
                    p.y - rise * 7.0,
                    3.4 - rise * 1.8,
                    alpha(smoke, 0.6 * (1.0 - rise)),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Smoke

/// The colour of a theme's smoke. `k` varies it per puff (only the acid one really cares).
pub fn theme_smoke(theme: Theme, k: f32, t: f32) -> Color {
    match theme {
        Theme::Joint => c(214, 226, 214),
        Theme::RedBong => c(220, 230, 245),
        Theme::GoldPipe => c(222, 214, 196),
        Theme::Acid => hsl(k * 0.13 + t * 0.4, 0.85, 0.7),
        Theme::Mushrooms => c(204, 160, 255),
        Theme::Hookah => c(255, 196, 226),
        Theme::Brownies => c(232, 210, 190),
        Theme::Blunt => c(196, 192, 182),
        Theme::Peyote => c(255, 196, 150),
        Theme::SpaceCake => c(180, 220, 255),
    }
}

/// A soft cloud: a few overlapping puffs.
fn puff(x: f32, y: f32, r: f32, col: Color) {
    circle(x, y, r, col);
    circle(x - r * 0.6, y + r * 0.25, r * 0.7, col);
    circle(x + r * 0.6, y + r * 0.2, r * 0.75, col);
}

/// The cloud that rolls out when you light up. `age` runs 0..1, `reach` is its final radius.
pub fn draw_stun_cloud(theme: Theme, cx: f32, cy: f32, age: f32, reach: f32, time: f64) {
    let t = time as f32;
    let r = reach * (1.0 - (1.0 - age).powi(2));
    let fade = 1.0 - age;
    glow(
        cx,
        cy,
        r * 1.1,
        alpha(theme_smoke(theme, 0.0, t), 0.35 * fade),
    );
    for i in 0..20 {
        let a = i as f32 * 0.314 + (i as f32 * 1.7).sin() * 0.4;
        let d = r * (0.35 + 0.65 * (((i * 7) % 11) as f32 / 11.0));
        let size = 10.0 + 8.0 * (((i * 5) % 7) as f32 / 7.0) + 14.0 * age;
        let col = theme_smoke(theme, i as f32, t);
        puff(
            cx + a.cos() * d,
            cy + a.sin() * d,
            size * 0.7,
            alpha(col, 0.32 * fade),
        );
    }
    match theme {
        Theme::RedBong => {
            for i in 0..12 {
                let a = i as f32 * 0.52;
                let d = r * (0.3 + 0.7 * ((i * 3 % 5) as f32 / 5.0));
                ring(
                    cx + a.cos() * d,
                    cy + a.sin() * d - age * 20.0,
                    2.0 + (i % 3) as f32,
                    1.2,
                    alpha(WHITE, 0.7 * fade),
                );
            }
        }
        Theme::Hookah => {
            for k in 0..3 {
                let rr = r * (0.4 + k as f32 * 0.3);
                ring(
                    cx,
                    cy,
                    rr,
                    5.0 * fade + 1.0,
                    alpha(theme_smoke(theme, 0.0, t), 0.4 * fade),
                );
            }
        }
        Theme::Mushrooms | Theme::Acid => {
            for i in 0..24 {
                let a = i as f32 * 0.9 + t;
                let d = r * ((i * 13 % 17) as f32 / 17.0);
                let col = if theme == Theme::Acid {
                    hsl(i as f32 * 0.07 + t, 0.9, 0.65)
                } else {
                    c(230, 200, 255)
                };
                circle(cx + a.cos() * d, cy + a.sin() * d, 1.6, alpha(col, fade));
            }
        }
        Theme::SpaceCake | Theme::Peyote => {
            for i in 0..10 {
                let a = i as f32 * 0.63 - t * 0.5;
                let d = r * (0.5 + 0.5 * ((i * 7 % 9) as f32 / 9.0));
                star(
                    cx + a.cos() * d,
                    cy + a.sin() * d,
                    3.0,
                    t * 2.0,
                    alpha(c(255, 250, 200), fade),
                );
            }
        }
        _ => {}
    }
}

/// A wisp of smoke curling up from a lit item.
fn wisp(x: f32, y: f32, t: f32, col: Color, s: f32) {
    for i in 0..3 {
        let ph = (t * 0.8 + i as f32 * 0.33).fract();
        let px = x + (ph * 7.0 + i as f32 * 2.0).sin() * 2.6 * s;
        let py = y - ph * 13.0 * s;
        circle(px, py, (1.3 + ph * 2.6) * s, alpha(col, 0.55 * (1.0 - ph)));
    }
}

fn ember(x: f32, y: f32, t: f32, s: f32) {
    let g = 0.5 + 0.5 * (t * 6.0).sin();
    glow(x, y, (6.0 + g * 2.0) * s, alpha(c(255, 140, 40), 0.55));
    circle(x, y, 2.3 * s, c(255, 90, 30));
    circle(x, y, 1.1 * s, c(255, 230, 140));
}

// ---------------------------------------------------------------------------------------------
// Stash items

/// A filled quad with an ink outline.
fn quad(pts: [Vec2; 4], col: Color, ink: f32) {
    let ctr = (pts[0] + pts[1] + pts[2] + pts[3]) / 4.0;
    outline(&pts, ink, INK);
    fan(ctr, &pts, col);
}

/// The stash item of a theme, centred at (cx, cy). At `s` = 1 it fits in one 32px tile.
pub fn draw_item(theme: Theme, cx: f32, cy: f32, s: f32, time: f64) {
    let t = time as f32;
    let o = vec2(cx, cy);
    let p = |x: f32, y: f32| o + vec2(x * s, y * s);
    let ink = 2.0 * s;
    match theme {
        Theme::Joint => {
            // A cone of paper, fatter at the lit end, with a crutch.
            let a = p(-10.0, 7.0);
            let b = p(8.0, -6.0);
            let d = (b - a).normalize();
            let n = vec2(-d.y, d.x);
            quad(
                [
                    a + n * 1.8 * s,
                    b + n * 3.4 * s,
                    b - n * 3.4 * s,
                    a - n * 1.8 * s,
                ],
                c(250, 246, 234),
                ink,
            );
            let k = a + d * 4.0 * s;
            quad(
                [
                    a + n * 1.8 * s,
                    k + n * 2.1 * s,
                    k - n * 2.1 * s,
                    a - n * 1.8 * s,
                ],
                c(214, 176, 120),
                0.0,
            );
            stroke(
                a + d * 6.0 * s + n * 0.5 * s,
                b - d * 2.0 * s + n * 1.0 * s,
                0.6 * s,
                alpha(c(170, 160, 140), 0.8),
            );
            circle(b.x, b.y, 3.4 * s, c(120, 120, 120));
            ember(b.x + d.x * s, b.y + d.y * s, t, s);
            wisp(b.x + 2.0 * s, b.y - 4.0 * s, t, c(220, 225, 220), s);
        }
        Theme::Blunt => {
            let a = p(-11.0, 6.0);
            let b = p(9.0, -5.0);
            let d = (b - a).normalize();
            let n = vec2(-d.y, d.x);
            quad(
                [
                    a + n * 3.4 * s,
                    b + n * 3.8 * s,
                    b - n * 3.8 * s,
                    a - n * 3.4 * s,
                ],
                c(132, 78, 38),
                ink,
            );
            for k in 0..5 {
                let q = a + d * (3.0 + k as f32 * 3.8) * s;
                stroke(
                    q + n * 3.2 * s,
                    q - n * 3.2 * s + d * 2.4 * s,
                    0.9 * s,
                    c(96, 54, 24),
                );
            }
            let band = a + d * 4.0 * s;
            quad(
                [
                    band + n * 3.6 * s,
                    band + d * 2.4 * s + n * 3.6 * s,
                    band + d * 2.4 * s - n * 3.6 * s,
                    band - n * 3.6 * s,
                ],
                c(240, 196, 70),
                0.0,
            );
            ember(b.x, b.y, t, s * 1.1);
            wisp(b.x + 2.0 * s, b.y - 4.0 * s, t, c(200, 196, 186), s * 1.2);
        }
        Theme::RedBong => {
            let glass = c(226, 40, 58);
            // Down-stem and bowl.
            stroke(p(4.0, 7.0), p(11.0, 0.0), 3.6 * s, INK);
            stroke(p(4.0, 7.0), p(11.0, 0.0), 2.0 * s, c(170, 180, 190));
            ellipse(
                p(11.5, -1.0).x,
                p(11.5, -1.0).y,
                3.6 * s,
                2.2 * s,
                -0.7,
                INK,
            );
            ellipse(
                p(11.5, -1.5).x,
                p(11.5, -1.5).y,
                2.6 * s,
                1.4 * s,
                -0.7,
                c(80, 160, 70),
            );
            // Tube and base.
            rounded_rect(
                p(-4.6, -14.0).x,
                p(-4.6, -14.0).y,
                9.2 * s,
                18.0 * s,
                2.0 * s,
                INK,
            );
            ellipse(p(0.0, 8.0).x, p(0.0, 8.0).y, 9.4 * s, 6.8 * s, 0.0, INK);
            rounded_rect(
                p(-3.4, -12.8).x,
                p(-3.4, -12.8).y,
                6.8 * s,
                16.0 * s,
                1.4 * s,
                glass,
            );
            ellipse(p(0.0, 8.0).x, p(0.0, 8.0).y, 8.2 * s, 5.6 * s, 0.0, glass);
            ellipse(
                p(0.0, 10.0).x,
                p(0.0, 10.0).y,
                6.6 * s,
                3.0 * s,
                0.0,
                alpha(c(120, 200, 255), 0.75),
            );
            rounded_rect(
                p(-5.4, -15.0).x,
                p(-5.4, -15.0).y,
                10.8 * s,
                2.6 * s,
                1.2 * s,
                darker(glass, 0.2),
            );
            stroke(p(-2.0, -11.0), p(-2.0, 1.0), 1.2 * s, alpha(WHITE, 0.6));
            ellipse(
                p(-4.0, 6.0).x,
                p(-4.0, 6.0).y,
                1.6 * s,
                2.4 * s,
                0.4,
                alpha(WHITE, 0.5),
            );
            for k in 0..3 {
                let ph = (t * 1.2 + k as f32 * 0.33).fract();
                ring(
                    p(-0.5 + k as f32 * 1.5, 9.0 - ph * 18.0).x,
                    p(0.0, 9.0 - ph * 18.0).y,
                    (0.8 + ph * 0.6) * s,
                    0.6 * s,
                    alpha(WHITE, 0.8 * (1.0 - ph)),
                );
            }
            wisp(p(0.0, -16.0).x, p(0.0, -16.0).y, t, c(235, 240, 250), s);
        }
        Theme::GoldPipe => {
            let gold = c(246, 196, 64);
            let mut stem = Vec::new();
            for i in 0..=8 {
                let k = i as f32 / 8.0;
                stem.push(p(-12.0 + k * 14.0, -3.0 + (k * PI).sin() * -2.0 + k * 6.0));
            }
            polyline(&stem, 4.6 * s, INK);
            polyline(&stem, 2.6 * s, gold);
            stroke(p(-12.5, -3.0), p(-10.0, -3.0), 3.0 * s, INK);
            rounded_rect(
                p(2.0, -7.0).x,
                p(2.0, -7.0).y,
                10.0 * s,
                14.0 * s,
                3.6 * s,
                INK,
            );
            rounded_rect(
                p(3.0, -6.0).x,
                p(3.0, -6.0).y,
                8.0 * s,
                12.0 * s,
                2.8 * s,
                gold,
            );
            stroke(
                p(5.0, -4.0),
                p(5.0, 4.0),
                1.4 * s,
                alpha(c(255, 250, 210), 0.9),
            );
            stroke(p(9.5, -3.0), p(9.5, 4.0), 1.0 * s, darker(gold, 0.3));
            ellipse(
                p(7.0, -6.2).x,
                p(7.0, -6.2).y,
                4.0 * s,
                1.6 * s,
                0.0,
                c(70, 150, 60),
            );
            ember(p(7.0, -6.6).x, p(7.0, -6.6).y, t, s * 0.8);
            wisp(p(7.0, -9.0).x, p(7.0, -9.0).y, t, c(230, 222, 204), s);
        }
        Theme::Acid => {
            let wob = (t * 1.5).sin() * 0.12;
            let corners: Vec<Vec2> = [(-9.0, -9.0), (9.0, -9.0), (9.0, 9.0), (-9.0, 9.0)]
                .iter()
                .map(|&(x, y)| o + rot(vec2(x * s, y * s), wob))
                .collect();
            outline(&corners, ink, INK);
            fan(o, &corners, WHITE);
            for k in 0..4 {
                circle(
                    cx,
                    cy,
                    (8.2 - k as f32 * 1.7) * s,
                    hsl(t * 0.3 + k as f32 * 0.2, 0.85, 0.6),
                );
            }
            circle(cx, cy, 3.6 * s, c(255, 228, 60));
            for sx in [-1.0f32, 1.0] {
                circle(cx + sx * 1.2 * s, cy - 0.9 * s, 0.5 * s, INK);
            }
            let mut smile = Vec::new();
            for i in 0..=6 {
                let a = 0.4 + i as f32 / 6.0 * (PI - 0.8);
                smile.push(vec2(
                    cx + a.cos() * 2.1 * s,
                    cy + a.sin() * 1.8 * s + 0.2 * s,
                ));
            }
            polyline(&smile, 0.5 * s, INK);
            // Perforations.
            for i in 0..2 {
                for k in 0..7 {
                    let f = -9.0 + k as f32 * 3.0;
                    let a = o + rot(vec2(f * s, (if i == 0 { -9.0 } else { 9.0 }) * s), wob);
                    circle(a.x, a.y, 0.6 * s, alpha(INK, 0.5));
                }
            }
        }
        Theme::Mushrooms => {
            glow(
                cx,
                cy,
                16.0 * s,
                alpha(c(190, 120, 255), 0.35 + 0.15 * (t * 2.0).sin()),
            );
            let mush = |x: f32, y: f32, k: f32| {
                let q = o + vec2(x * s, y * s);
                rounded_rect(
                    q.x - 3.6 * s * k,
                    q.y - 1.0 * s * k,
                    7.2 * s * k,
                    11.0 * s * k,
                    3.0 * s * k,
                    INK,
                );
                rounded_rect(
                    q.x - 2.6 * s * k,
                    q.y - 1.0 * s * k,
                    5.2 * s * k,
                    10.0 * s * k,
                    2.4 * s * k,
                    c(250, 240, 214),
                );
                let mut cap = vec![q + vec2(-11.0, 1.0) * s * k];
                for i in 0..=14 {
                    let a = PI + i as f32 / 14.0 * PI;
                    cap.push(q + vec2(a.cos() * 11.0, a.sin() * 9.0 + 1.0) * s * k);
                }
                outline(&cap, ink * k, INK);
                fan(q + vec2(0.0, -3.0) * s * k, &cap, c(222, 40, 46));
                ellipse(
                    q.x - 4.0 * s * k,
                    q.y - 5.0 * s * k,
                    2.4 * s * k,
                    1.6 * s * k,
                    -0.3,
                    alpha(WHITE, 0.35),
                );
                for (dx, dy, r) in [
                    (-5.0, -3.0, 1.9),
                    (1.0, -6.0, 2.2),
                    (6.0, -2.0, 1.6),
                    (-1.0, -1.5, 1.2),
                ] {
                    circle(
                        q.x + dx * s * k,
                        q.y + dy * s * k,
                        r * s * k,
                        c(255, 250, 240),
                    );
                }
            };
            mush(6.5, 3.0, 0.55);
            mush(-1.5, 1.0, 0.9);
        }
        Theme::Hookah => {
            stroke(p(4.0, 2.0), p(12.0, 4.0), 3.2 * s, INK);
            let mut hose = Vec::new();
            for i in 0..=10 {
                let k = i as f32 / 10.0;
                hose.push(p(4.0 + k * 9.0, 2.0 + (k * PI * 1.4).sin() * 5.0 + k * 6.0));
            }
            polyline(&hose, 3.0 * s, INK);
            polyline(&hose, 1.6 * s, c(180, 70, 200));
            ellipse(p(0.0, 8.5).x, p(0.0, 8.5).y, 8.4 * s, 6.6 * s, 0.0, INK);
            ellipse(
                p(0.0, 8.5).x,
                p(0.0, 8.5).y,
                7.2 * s,
                5.4 * s,
                0.0,
                c(40, 180, 170),
            );
            ellipse(
                p(-2.4, 6.5).x,
                p(-2.4, 6.5).y,
                1.6 * s,
                2.4 * s,
                0.4,
                alpha(WHITE, 0.5),
            );
            rounded_rect(
                p(-2.4, -10.0).x,
                p(-2.4, -10.0).y,
                4.8 * s,
                15.0 * s,
                1.4 * s,
                INK,
            );
            rounded_rect(
                p(-1.4, -9.0).x,
                p(-1.4, -9.0).y,
                2.8 * s,
                13.0 * s,
                1.0 * s,
                c(240, 190, 70),
            );
            ellipse(p(0.0, -6.0).x, p(0.0, -6.0).y, 6.4 * s, 1.9 * s, 0.0, INK);
            ellipse(
                p(0.0, -6.0).x,
                p(0.0, -6.0).y,
                5.6 * s,
                1.2 * s,
                0.0,
                c(240, 190, 70),
            );
            quad(
                [p(-3.6, -14.0), p(3.6, -14.0), p(2.0, -9.6), p(-2.0, -9.6)],
                c(190, 90, 60),
                1.6 * s,
            );
            ember(p(0.0, -14.5).x, p(0.0, -14.5).y, t, s * 0.8);
            wisp(p(0.0, -17.0).x, p(0.0, -17.0).y, t, c(255, 200, 230), s);
        }
        Theme::Brownies => {
            rounded_rect(
                p(-10.0, -7.0).x,
                p(-10.0, -7.0).y,
                20.0 * s,
                16.0 * s,
                3.0 * s,
                INK,
            );
            rounded_rect(
                p(-9.0, -6.0).x,
                p(-9.0, -6.0).y,
                18.0 * s,
                14.0 * s,
                2.4 * s,
                c(92, 50, 26),
            );
            rounded_rect(
                p(-9.0, -6.0).x,
                p(-9.0, -6.0).y,
                18.0 * s,
                6.0 * s,
                2.4 * s,
                c(122, 70, 38),
            );
            for (a, b) in [
                ((-6.0, -4.0), (-2.0, -3.0)),
                ((1.0, -5.0), (5.0, -3.5)),
                ((-3.0, -2.0), (2.0, -1.5)),
            ] {
                stroke(p(a.0, a.1), p(b.0, b.1), 0.8 * s, c(70, 36, 18));
            }
            for (x, y) in [(-6.0, 3.0), (4.0, 4.0), (-1.0, 5.0)] {
                circle(p(x, y).x, p(x, y).y, 0.9 * s, c(60, 30, 14));
            }
            let base = p(2.0, -4.0);
            for (a, l) in [(-1.0f32, 5.0f32), (0.0, 7.0), (1.0, 5.0)] {
                leaflet(base, a, (l + 1.0) * s, 2.6 * s, INK, INK);
                leaflet(base, a, l * s, 1.9 * s, c(90, 220, 90), c(50, 160, 60));
            }
        }
        Theme::Peyote => {
            ellipse(cx, cy + 3.0 * s, 11.4 * s, 8.6 * s, 0.0, INK);
            ellipse(cx, cy + 3.0 * s, 10.2 * s, 7.4 * s, 0.0, c(110, 170, 120));
            for k in -2..=2 {
                let x = k as f32 * 3.6;
                let mut rib = Vec::new();
                for i in 0..=6 {
                    let f = i as f32 / 6.0;
                    rib.push(p(x * (0.3 + f * 0.7), -3.0 + f * 12.0 - (x * 0.1).abs()));
                }
                polyline(&rib, 0.9 * s, c(70, 120, 84));
            }
            for (x, y) in [
                (-6.0, 1.0),
                (-2.0, -1.0),
                (2.0, -1.0),
                (6.0, 1.0),
                (-4.0, 5.0),
                (4.0, 5.0),
                (0.0, 3.0),
            ] {
                circle(p(x, y).x, p(x, y).y, 0.9 * s, WHITE);
            }
            for k in 0..5 {
                let a = k as f32 / 5.0 * TAU + t * 0.4;
                let q = p(0.0, -5.0) + vec2(a.cos(), a.sin()) * 2.6 * s;
                ellipse(q.x, q.y, 2.6 * s, 1.6 * s, a, c(255, 130, 180));
            }
            circle(p(0.0, -5.0).x, p(0.0, -5.0).y, 1.4 * s, c(255, 220, 90));
        }
        Theme::SpaceCake => {
            quad(
                [p(-7.0, 1.0), p(7.0, 1.0), p(5.0, 10.0), p(-5.0, 10.0)],
                c(140, 200, 255),
                ink,
            );
            for k in -2..=2 {
                stroke(
                    p(k as f32 * 2.4, 2.0),
                    p(k as f32 * 1.8, 9.0),
                    0.8 * s,
                    c(90, 150, 220),
                );
            }
            for (y, w) in [(0.0, 8.6), (-3.6, 6.8), (-6.8, 4.4)] {
                ellipse(p(0.0, y).x, p(0.0, y).y, (w + 1.0) * s, 3.6 * s, 0.0, INK);
            }
            for (y, w) in [(0.0, 8.6), (-3.6, 6.8), (-6.8, 4.4)] {
                ellipse(
                    p(0.0, y).x,
                    p(0.0, y).y,
                    w * s,
                    2.7 * s,
                    0.0,
                    c(255, 170, 214),
                );
            }
            for k in 0..7 {
                let a = k as f32 * 2.1;
                let q = p(a.cos() * 5.0, -2.0 + a.sin() * 2.4);
                draw_line(
                    q.x,
                    q.y,
                    q.x + 1.2 * s,
                    q.y + 0.6 * s,
                    0.9 * s,
                    hsl(k as f32 * 0.14, 0.9, 0.6),
                );
            }
            let base = p(0.0, -9.0);
            for (a, l) in [(-0.9f32, 4.0f32), (0.0, 6.0), (0.9, 4.0)] {
                leaflet(base, a, (l + 1.0) * s, 2.2 * s, INK, INK);
                leaflet(base, a, l * s, 1.6 * s, c(90, 220, 90), c(50, 160, 60));
            }
            // A tiny orbiting moon.
            let a = t * 2.0;
            circle(
                cx + a.cos() * 12.0 * s,
                cy - 2.0 * s + a.sin() * 4.0 * s,
                1.6 * s,
                c(255, 250, 200),
            );
        }
    }
}
