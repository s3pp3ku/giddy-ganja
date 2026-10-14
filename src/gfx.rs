//! Small drawing toolkit on top of macroquad: smooth circles and ellipses, gradients, rounded
//! rectangles, outlined polygons and crisp text. Everything is drawn in logical 640x450 units;
//! curves and glyphs are tessellated/rasterized for the real window size, so nothing looks blocky.

use macroquad::models::{draw_mesh, Mesh, Vertex};
use macroquad::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};

static VIEW_SCALE: AtomicU32 = AtomicU32::new(0x3f80_0000); // 1.0

/// How many window pixels one logical unit covers. Set once per frame by the app.
pub fn set_view_scale(s: f32) {
    VIEW_SCALE.store(s.max(0.1).to_bits(), Ordering::Relaxed);
}

pub fn view_scale() -> f32 {
    f32::from_bits(VIEW_SCALE.load(Ordering::Relaxed))
}

pub fn c(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgba(r, g, b, 255)
}

pub fn alpha(col: Color, a: f32) -> Color {
    Color::new(col.r, col.g, col.b, col.a * a.clamp(0.0, 1.0))
}

pub fn mix(a: Color, b: Color, t: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

pub fn darker(col: Color, t: f32) -> Color {
    mix(col, Color::new(0.0, 0.0, 0.0, col.a), t)
}

pub fn lighter(col: Color, t: f32) -> Color {
    mix(col, Color::new(1.0, 1.0, 1.0, col.a), t)
}

/// A colour from hue (0..1, wrapping), saturation and lightness.
pub fn hsl(h: f32, s: f32, l: f32) -> Color {
    let c = macroquad::color::hsl_to_rgb(h.rem_euclid(1.0), s, l);
    Color::new(c.r, c.g, c.b, 1.0)
}

fn sides_for(r: f32) -> u8 {
    ((r * view_scale()).max(1.0).sqrt() * 5.0).clamp(10.0, 90.0) as u8
}

/// A circle with enough segments to look round at any size.
pub fn circle(x: f32, y: f32, r: f32, col: Color) {
    if r <= 0.0 {
        return;
    }
    draw_poly(x, y, sides_for(r), r, 0.0, col);
}

pub fn ring(x: f32, y: f32, r: f32, thick: f32, col: Color) {
    if r <= 0.0 {
        return;
    }
    draw_poly_lines(x, y, sides_for(r), r, 0.0, thick, col);
}

/// An ellipse with radii `rx`, `ry`, rotated by `rot` radians.
pub fn ellipse(x: f32, y: f32, rx: f32, ry: f32, rot: f32, col: Color) {
    let n = sides_for(rx.max(ry)) as usize;
    let (s, co) = rot.sin_cos();
    let mut pts = Vec::with_capacity(n);
    for i in 0..n {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        let (px, py) = (a.cos() * rx, a.sin() * ry);
        pts.push(vec2(x + px * co - py * s, y + px * s + py * co));
    }
    fan(vec2(x, y), &pts, col);
}

/// Fill a polygon that is star-shaped around `center` (true for all the shapes used here).
pub fn fan(center: Vec2, pts: &[Vec2], col: Color) {
    if pts.len() < 2 {
        return;
    }
    let mut vertices = Vec::with_capacity(pts.len() + 1);
    vertices.push(Vertex::new(center.x, center.y, 0.0, 0.0, 0.0, col));
    for p in pts {
        vertices.push(Vertex::new(p.x, p.y, 0.0, 0.0, 0.0, col));
    }
    let mut indices = Vec::with_capacity(pts.len() * 3);
    let n = pts.len() as u16;
    for i in 0..n {
        indices.extend_from_slice(&[0, 1 + i, 1 + (i + 1) % n]);
    }
    draw_mesh(&Mesh {
        vertices,
        indices,
        texture: None,
    });
}

/// A closed outline through `pts`, with round joins.
pub fn outline(pts: &[Vec2], thick: f32, col: Color) {
    for i in 0..pts.len() {
        let a = pts[i];
        let b = pts[(i + 1) % pts.len()];
        draw_line(a.x, a.y, b.x, b.y, thick, col);
        circle(a.x, a.y, thick / 2.0, col);
    }
}

/// An open polyline with round joins and caps.
pub fn polyline(pts: &[Vec2], thick: f32, col: Color) {
    for w in pts.windows(2) {
        draw_line(w[0].x, w[0].y, w[1].x, w[1].y, thick, col);
    }
    for p in pts {
        circle(p.x, p.y, thick / 2.0, col);
    }
}

/// A thick line with round caps.
pub fn stroke(a: Vec2, b: Vec2, thick: f32, col: Color) {
    draw_line(a.x, a.y, b.x, b.y, thick, col);
    circle(a.x, a.y, thick / 2.0, col);
    circle(b.x, b.y, thick / 2.0, col);
}

/// A rectangle shaded from `top` to `bottom`.
pub fn vgradient(x: f32, y: f32, w: f32, h: f32, top: Color, bottom: Color) {
    let vertices = vec![
        Vertex::new(x, y, 0.0, 0.0, 0.0, top),
        Vertex::new(x + w, y, 0.0, 0.0, 0.0, top),
        Vertex::new(x + w, y + h, 0.0, 0.0, 0.0, bottom),
        Vertex::new(x, y + h, 0.0, 0.0, 0.0, bottom),
    ];
    draw_mesh(&Mesh {
        vertices,
        indices: vec![0, 1, 2, 0, 2, 3],
        texture: None,
    });
}

/// A soft radial glow: opaque-ish `inner` at the centre fading to transparent at radius `r`.
pub fn glow(x: f32, y: f32, r: f32, inner: Color) {
    let n = sides_for(r) as usize;
    let mut vertices = Vec::with_capacity(n + 1);
    vertices.push(Vertex::new(x, y, 0.0, 0.0, 0.0, inner));
    let edge = alpha(inner, 0.0);
    for i in 0..n {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        vertices.push(Vertex::new(
            x + a.cos() * r,
            y + a.sin() * r,
            0.0,
            0.0,
            0.0,
            edge,
        ));
    }
    let mut indices = Vec::with_capacity(n * 3);
    for i in 0..n as u16 {
        indices.extend_from_slice(&[0, 1 + i, 1 + (i + 1) % n as u16]);
    }
    draw_mesh(&Mesh {
        vertices,
        indices,
        texture: None,
    });
}

/// A rectangle with rounded corners.
pub fn rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32, col: Color) {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    draw_rectangle(x + r, y, w - 2.0 * r, h, col);
    draw_rectangle(x, y + r, r, h - 2.0 * r, col);
    draw_rectangle(x + w - r, y + r, r, h - 2.0 * r, col);
    for (cx, cy, sx, sy) in [
        (x + r, y + r, -1.0, -1.0),
        (x + w - r, y + r, 1.0, -1.0),
        (x + r, y + h - r, -1.0, 1.0),
        (x + w - r, y + h - r, 1.0, 1.0),
    ] {
        quarter_circle(cx, cy, r, sx, sy, col);
    }
}

/// The quarter of a circle at (cx, cy) that faces away from (mx, my).
fn quarter(cx: f32, cy: f32, r: f32, col: Color, mx: f32, my: f32) {
    let sx = if cx < mx { -1.0 } else { 1.0 };
    let sy = if cy < my { -1.0 } else { 1.0 };
    let n = (sides_for(r) / 4).max(3) as usize;
    let mut pts = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let a = i as f32 / n as f32 * std::f32::consts::FRAC_PI_2;
        pts.push(vec2(cx + a.cos() * r * sx, cy + a.sin() * r * sy));
    }
    let mut vertices = vec![Vertex::new(cx, cy, 0.0, 0.0, 0.0, col)];
    for p in &pts {
        vertices.push(Vertex::new(p.x, p.y, 0.0, 0.0, 0.0, col));
    }
    let mut indices = Vec::new();
    for i in 0..n as u16 {
        indices.extend_from_slice(&[0, 1 + i, 2 + i]);
    }
    draw_mesh(&Mesh {
        vertices,
        indices,
        texture: None,
    });
}

/// A rounded rectangle outline.
pub fn rounded_rect_lines(x: f32, y: f32, w: f32, h: f32, r: f32, thick: f32, col: Color) {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    let mut pts = Vec::new();
    let n = (sides_for(r) / 4).max(3);
    for (cx, cy, start) in [
        (x + w - r, y + r, -std::f32::consts::FRAC_PI_2),
        (x + w - r, y + h - r, 0.0),
        (x + r, y + h - r, std::f32::consts::FRAC_PI_2),
        (x + r, y + r, std::f32::consts::PI),
    ] {
        for i in 0..=n {
            let a = start + i as f32 / n as f32 * std::f32::consts::FRAC_PI_2;
            pts.push(vec2(cx + a.cos() * r, cy + a.sin() * r));
        }
    }
    outline(&pts, thick, col);
}

// ---------------------------------------------------------------------------------------------
// Text
//
// Glyphs are rasterized with fontdue into one texture per (face, pixel size), holding the whole
// character set, and those textures are never replaced. (macroquad's own font atlas grows by
// deleting and recreating its texture, which with miniquad 0.4's texture-binding cache left text
// rendering as solid black boxes when it happened mid-frame.)

const CHARSET: &str = " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\u{2013}\u{2014}\u{2018}\u{2019}\u{201C}\u{201D}\u{2026}\u{2022}";

struct Glyph {
    rect: Rect,
    xmin: f32,
    ymin: f32,
    advance: f32,
}

struct SizedFace {
    texture: Texture2D,
    glyphs: HashMap<char, Glyph>,
}

struct FaceData {
    font: fontdue::Font,
    sizes: HashMap<u16, SizedFace>,
}

thread_local! {
    static FACES: RefCell<Vec<FaceData>> = const { RefCell::new(Vec::new()) };
}

/// Load the bundled fonts. Call once at start-up; without it no text is drawn.
pub fn load_fonts() {
    let load =
        |bytes: &[u8]| fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()).ok();
    let display = load(include_bytes!("../assets/fonts/LuckiestGuy-Regular.ttf"));
    let body = load(include_bytes!("../assets/fonts/Fredoka-SemiBold.ttf"));
    if let (Some(display), Some(body)) = (display, body) {
        FACES.with(|f| {
            *f.borrow_mut() = vec![
                FaceData {
                    font: display,
                    sizes: HashMap::new(),
                },
                FaceData {
                    font: body,
                    sizes: HashMap::new(),
                },
            ]
        });
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Face {
    /// Chunky cartoon capitals for titles and big numbers.
    Display,
    /// Rounded and readable, for everything else.
    Body,
}

/// The pixel size to rasterize at, quantized so that animated sizes and window resizes only
/// ever produce a modest number of cached sizes.
fn raster(size: f32) -> (u16, f32) {
    let px = (size * view_scale()).max(4.0);
    let q = match px {
        p if p < 24.0 => 1.0,
        p if p < 48.0 => 2.0,
        p if p < 96.0 => 4.0,
        _ => 8.0,
    };
    let px = ((px / q).round() * q).clamp(4.0, 256.0);
    (px as u16, size / px)
}

fn build(font: &fontdue::Font, px: u16) -> SizedFace {
    let rendered: Vec<(char, fontdue::Metrics, Vec<u8>)> = CHARSET
        .chars()
        .map(|ch| {
            let (m, bmp) = font.rasterize(ch, px as f32);
            (ch, m, bmp)
        })
        .collect();
    // Shelf-pack into a power-of-two-wide strip.
    let area: usize = rendered
        .iter()
        .map(|(_, m, _)| (m.width + 2) * (m.height + 2))
        .sum();
    let width = ((area as f32).sqrt() * 1.25).clamp(64.0, 4096.0) as usize;
    let width = width.next_power_of_two();
    let (mut x, mut y, mut row) = (1usize, 1usize, 0usize);
    let mut placed = Vec::with_capacity(rendered.len());
    for (_, m, _) in &rendered {
        if x + m.width + 1 > width {
            x = 1;
            y += row + 2;
            row = 0;
        }
        placed.push((x, y));
        x += m.width + 2;
        row = row.max(m.height);
    }
    let height = (y + row + 2).next_power_of_two();
    let mut bytes = vec![0u8; width * height * 4];
    let mut glyphs = HashMap::new();
    for ((ch, m, bmp), (gx, gy)) in rendered.into_iter().zip(placed) {
        for j in 0..m.height {
            for i in 0..m.width {
                let o = ((gy + j) * width + gx + i) * 4;
                bytes[o..o + 4].copy_from_slice(&[255, 255, 255, bmp[j * m.width + i]]);
            }
        }
        glyphs.insert(
            ch,
            Glyph {
                rect: Rect::new(gx as f32, gy as f32, m.width as f32, m.height as f32),
                xmin: m.xmin as f32,
                ymin: m.ymin as f32,
                advance: m.advance_width,
            },
        );
    }
    let texture = Texture2D::from_rgba8(width as u16, height as u16, &bytes);
    texture.set_filter(FilterMode::Linear);
    SizedFace { texture, glyphs }
}

/// Lay out `s` and call `emit` for every visible glyph; returns the advance width.
fn layout(
    face: Face,
    s: &str,
    size: f32,
    mut emit: impl FnMut(&Texture2D, &Glyph, f32, f32),
) -> f32 {
    let (px, scale) = raster(size);
    FACES.with(|faces| {
        let mut faces = faces.borrow_mut();
        let Some(fd) = faces.get_mut(face as usize) else {
            return 0.0;
        };
        if !fd.sizes.contains_key(&px) {
            let sized = build(&fd.font, px);
            fd.sizes.insert(px, sized);
        }
        let sized = &fd.sizes[&px];
        let mut pen = 0.0;
        let mut prev = None;
        for ch in s.chars() {
            let ch = if sized.glyphs.contains_key(&ch) {
                ch
            } else {
                '?'
            };
            if let Some(p) = prev {
                pen += fd.font.horizontal_kern(p, ch, px as f32).unwrap_or(0.0) * scale;
            }
            let g = &sized.glyphs[&ch];
            if g.rect.w > 0.0 {
                emit(&sized.texture, g, pen, scale);
            }
            pen += g.advance * scale;
            prev = Some(ch);
        }
        pen
    })
}

/// Draw text with its baseline at `y`.
pub fn text(face: Face, s: &str, x: f32, y: f32, size: f32, col: Color) {
    layout(face, s, size, |tex, g, pen, scale| {
        draw_texture_ex(
            tex,
            x + pen + g.xmin * scale,
            y - (g.ymin + g.rect.h) * scale,
            col,
            DrawTextureParams {
                dest_size: Some(vec2(g.rect.w * scale, g.rect.h * scale)),
                source: Some(g.rect),
                ..Default::default()
            },
        );
    });
}

pub fn text_width(face: Face, s: &str, size: f32) -> f32 {
    layout(face, s, size, |_, _, _, _| {})
}

/// Centred text with a soft drop shadow.
pub fn text_centered(face: Face, s: &str, cx: f32, y: f32, size: f32, col: Color) {
    let w = text_width(face, s, size);
    let d = (size * 0.06).clamp(1.0, 3.0);
    text(
        face,
        s,
        cx - w / 2.0 + d,
        y + d,
        size,
        alpha(BLACK, 0.55 * col.a),
    );
    text(face, s, cx - w / 2.0, y, size, col);
}

/// Centred text with a solid outline (cartoon-title style).
pub fn text_outlined(face: Face, s: &str, cx: f32, y: f32, size: f32, col: Color, edge: Color) {
    let w = text_width(face, s, size);
    let x = cx - w / 2.0;
    let o = (size * 0.06).clamp(1.0, 4.0);
    text(
        face,
        s,
        x + o * 0.6,
        y + o * 1.6,
        size,
        alpha(BLACK, 0.5 * col.a),
    );
    for k in 0..8 {
        let a = k as f32 * std::f32::consts::FRAC_PI_4;
        text(face, s, x + a.cos() * o, y + a.sin() * o, size, edge);
    }
    text(face, s, x, y, size, col);
}

/// A quarter circle at (cx, cy) bulging towards (sx, sy) (each -1 or 1).
pub fn quarter_circle(cx: f32, cy: f32, r: f32, sx: f32, sy: f32, col: Color) {
    quarter(cx, cy, r, col, cx - sx, cy - sy);
}
