//! The material: one language of surfaces by elevation, which every raised thing in the kit is painted with.
//!
//! The page ground is the lowest level and takes no surface. Above it: a card, a float (menus, search pickers,
//! the date picker and toasts open over the page beside what opened them), and a sheet over everything. Shadow,
//! ground, ring and the top edge's light change together with the level: the higher, the softer and longer the
//! shadow and the more the ground lets through (a float and a sheet are a little translucent with a faint tint, in
//! place of frosted glass). Every surface's ground runs almost unseen from its top to its foot, and its top edge
//! catches a thread of light. Small raised parts (keys, the segment and rail thumbs, the switch knob) take one
//! faint neutral shadow; solid keys are never haloed in their own colour.
//!
//! Painting only: no surface here changes where anything lies.

use crate::palette::{blend, c, mix, C};
use std::sync::atomic::{AtomicBool, Ordering};
use crate::tokens::Radius;
use egui::{pos2, Color32, Rect, Shape};

static GLASS: AtomicBool = AtomicBool::new(false);

/// Say whether the system's own material lies under the window (set once the window has it): the rail's ground
/// then lets it through.
pub fn set_glass(on: bool) {
    GLASS.store(on, Ordering::Relaxed);
}

pub fn glass() -> bool {
    GLASS.load(Ordering::Relaxed)
}

/// The rail's ground (the side rail, the first-run wizard's steps): over the system's material a translucent
/// tint of it, else the tint itself, opaque.
pub fn rail_ground() -> Color32 {
    if glass() { c(C::Rail).gamma_multiply(0.55) } else { c(C::Rail) }
}

/// A raised surface's level above the page ground.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
    Card,
    Float,
    Sheet,
}

impl Level {
    pub const ALL: [Level; 3] = [Level::Card, Level::Float, Level::Sheet];
}

/// One soft shadow: y offset, blur, alpha (of the shadow ink).
#[derive(Clone, Copy)]
struct Soft {
    y: f32,
    blur: f32,
    a: u8,
}

const fn soft(y: f32, blur: f32, a: u8) -> Soft {
    Soft { y, blur, a }
}

/// What a level is made of in one appearance: two shadows, the ring's alpha, how opaque the ground is, the tint
/// the ground leans to, and the top edge's light (alpha of white).
#[derive(Clone, Copy)]
struct Spec {
    shadows: [Soft; 2],
    ring: u8,
    opacity: f32,
    tint: f32,
    light: u8,
}

fn spec(level: Level, dark: bool) -> Spec {
    match (level, dark) {
        (Level::Card, false) => Spec { shadows: [soft(1.0, 2.0, 10), soft(6.0, 16.0, 10)], ring: 15, opacity: 1.0, tint: 0.0, light: 160 },
        (Level::Card, true) => Spec { shadows: [soft(1.0, 2.0, 77), soft(0.0, 0.0, 0)], ring: 15, opacity: 1.0, tint: 0.0, light: 14 },
        (Level::Float, false) => Spec { shadows: [soft(2.0, 4.0, 13), soft(12.0, 28.0, 20)], ring: 15, opacity: 0.94, tint: 0.04, light: 200 },
        (Level::Float, true) => Spec { shadows: [soft(12.0, 28.0, 89), soft(1.0, 3.0, 40)], ring: 20, opacity: 0.94, tint: 0.04, light: 18 },
        (Level::Sheet, false) => Spec { shadows: [soft(18.0, 48.0, 46), soft(2.0, 6.0, 15)], ring: 20, opacity: 0.97, tint: 0.03, light: 220 },
        (Level::Sheet, true) => Spec { shadows: [soft(18.0, 48.0, 140), soft(1.0, 3.0, 50)], ring: 20, opacity: 0.97, tint: 0.03, light: 20 },
    }
}

/// The shadow ink: a cool near-black in the light appearance, black in the dark.
fn ink(a: f32) -> Color32 {
    let t = blend();
    let light = Color32::from_rgba_unmultiplied(16, 24, 40, a.round().clamp(0.0, 255.0) as u8);
    let dark = Color32::from_rgba_unmultiplied(0, 0, 0, a.round().clamp(0.0, 255.0) as u8);
    mix(light, dark, t)
}

/// The shapes of a surface at `level` over `rect`, a card's hover (`hover` 0 to 1) lifting it toward a float.
pub fn shapes(rect: Rect, radius: Radius, level: Level, hover: f32) -> Vec<Shape> {
    build(rect, radius, level, hover, None)
}

/// A card-level plate whose ground is `ground` exactly, in both appearances (a QR code's white plate: any
/// reader must find pure white behind the modules).
pub fn plate(p: &egui::Painter, rect: Rect, radius: Radius, ground: Color32) {
    p.extend(build(rect, radius, Level::Card, 0.0, Some(ground)));
}

fn build(rect: Rect, radius: Radius, level: Level, hover: f32, exact: Option<Color32>) -> Vec<Shape> {
    let t = blend();
    let at = |dark: bool| {
        let s = spec(level, dark);
        if level == Level::Card && hover > 0.0 {
            let f = spec(Level::Float, dark);
            let m = |a: Soft, b: Soft| Soft { y: a.y + (b.y - a.y) * hover, blur: a.blur + (b.blur - a.blur) * hover, a: (a.a as f32 + (b.a as f32 - a.a as f32) * hover) as u8 };
            Spec { shadows: [m(s.shadows[0], f.shadows[0]), m(s.shadows[1], f.shadows[1])], ..s }
        } else {
            s
        }
    };
    let (l, d) = (at(false), at(true));
    let lerp = |a: f32, b: f32| a + (b - a) * t;
    let r = radius.egui();
    let mut out = Vec::new();
    for i in (0..2).rev() {
        let (a, b) = (l.shadows[i], d.shadows[i]);
        let alpha = lerp(a.a as f32, b.a as f32);
        if alpha < 0.5 {
            continue;
        }
        let sh = egui::Shadow { offset: [0, lerp(a.y, b.y).round() as i8], blur: lerp(a.blur, b.blur).round().clamp(0.0, 255.0) as u8, spread: 0, color: ink(alpha) };
        out.push(sh.as_shape(rect, r).into());
    }
    let ground = mix(c(C::Surface), c(C::Ground), lerp(l.tint, d.tint)).gamma_multiply(lerp(l.opacity, d.opacity));
    // Top to foot the ground runs from itself to a breath of the page ground: almost unseen.
    let foot = mix(ground, c(C::Ground).gamma_multiply(lerp(l.opacity, d.opacity)), 0.06);
    match exact {
        Some(g) => out.push(egui::epaint::RectShape::filled(rect, r, g).into()),
        None => out.push(gradient(rect, radius.px(), ground, foot, ground)),
    }
    let edge = mix(Color32::from_black_alpha(l.ring), Color32::from_white_alpha(d.ring), t);
    out.push(egui::epaint::RectShape::stroke(rect, r, egui::Stroke::new(0.5_f32, edge), egui::StrokeKind::Outside).into());
    let light = Color32::from_white_alpha(lerp(l.light as f32, d.light as f32).round() as u8);
    let inset = radius.px().min(rect.width() / 2.0);
    if rect.width() > 2.0 * inset {
        out.push(Shape::line_segment([pos2(rect.left() + inset, rect.top() + 0.5), pos2(rect.right() - inset, rect.top() + 0.5)], egui::Stroke::new(1.0_f32, light)));
    }
    out
}

/// Paint a surface at `level`.
pub fn paint(p: &egui::Painter, rect: Rect, radius: Radius, level: Level, hover: f32) {
    p.extend(shapes(rect, radius, level, hover));
}

/// The one faint neutral shadow of a small raised part (a key, a thumb, a knob), at `alpha` of its strength.
pub fn small(rect: Rect, radius: egui::CornerRadius, alpha: f32) -> Shape {
    let a = (14.0 + (90.0 - 14.0) * blend()) * alpha;
    egui::Shadow { offset: [0, 1], blur: 2, spread: 0, color: ink(a) }.as_shape(rect, radius).into()
}

/// A solid key's ground: its colour, the top a breath lighter than the foot (almost unseen), no halo.
pub fn solid(rect: Rect, radius: f32, fill: Color32) -> Shape {
    gradient(rect, radius, mix(fill, Color32::WHITE, 0.05), fill, fill)
}

/// A rounded rectangle filled from `top` to `foot`: a rectangle of `base`, the thing's own colour (its rim
/// smoothed as every rectangle's is, and the colour any reading of the ground finds), the run from top to foot
/// laid over it (the corners in eight steps each); a surface's ring is drawn over both.
pub fn gradient(rect: Rect, radius: f32, top: Color32, foot: Color32, base: Color32) -> Shape {
    let base: Shape = egui::epaint::RectShape::filled(rect, egui::CornerRadius::same(radius.round().clamp(0.0, 255.0) as u8), base).into();
    Shape::Vec(vec![base, run(rect, radius, top, foot)])
}

fn run(rect: Rect, radius: f32, top: Color32, foot: Color32) -> Shape {
    let r = radius.min(rect.width() / 2.0).min(rect.height() / 2.0).max(0.0);
    let h = rect.height().max(f32::EPSILON);
    let colour = |y: f32| mix(top, foot, ((y - rect.top()) / h).clamp(0.0, 1.0));
    let mut rim = Vec::new();
    let corners = [(rect.right() - r, rect.top() + r, -90.0_f32), (rect.right() - r, rect.bottom() - r, 0.0), (rect.left() + r, rect.bottom() - r, 90.0), (rect.left() + r, rect.top() + r, 180.0)];
    for (cx, cy, from) in corners {
        for k in 0..=8 {
            let a = (from + 90.0 * k as f32 / 8.0).to_radians();
            rim.push(pos2(cx + r * a.cos(), cy + r * a.sin()));
        }
    }
    let mut mesh = egui::Mesh::default();
    let centre = rect.center();
    mesh.colored_vertex(centre, colour(centre.y));
    for p in &rim {
        mesh.colored_vertex(*p, colour(p.y));
    }
    let n = rim.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    Shape::mesh(mesh)
}
