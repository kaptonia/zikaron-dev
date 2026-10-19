//! The lit block under a row (hovered, pressed or chosen), the one shape every list in a card, a sheet or a
//! menu lights with. It keeps one gap on all four sides to what bounds it: the card's edge, or the line between
//! two rows. Where it faces the card's edge its corners are the card's corners less that gap, so the two curves
//! share their centres; where it meets a line it has no corner.

use egui::{CornerRadius, Rect, pos2};

/// The gap between the block and the card's edge or the line beside it, the same on every side.
pub const GAP: f32 = 6.0;

/// What lies beyond one side of a row's slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bound {
    /// The card's own edge (or, in a sheet or a menu, the end of the list).
    Edge,
    /// A line between this row and the next.
    Line,
    /// The next row, with no line between (a menu's rows): the block reaches the slot's edge.
    Row,
}

/// The block for a row whose slot spans the card from edge to edge across and runs from what bounds it above to
/// what bounds it below; `corner` is the card's corner. Returns the block and its corners.
pub fn block(slot: Rect, corner: f32, top: Bound, bottom: Bound) -> (Rect, CornerRadius) {
    let inset = |b: Bound| if b == Bound::Row { 0.0 } else { GAP };
    let r = pos2(slot.left() + GAP, slot.top() + inset(top));
    let s = pos2(slot.right() - GAP, slot.bottom() - inset(bottom));
    let rect = Rect::from_min_max(r, s);
    (rect, corners(corner, rect.height(), top, bottom))
}

/// The block's corners alone, for a list that lays its rows `GAP` in from the card's sides itself (a menu):
/// the card's corner less the gap, none on a side that meets a line.
pub fn corners(corner: f32, height: f32, top: Bound, bottom: Bound) -> CornerRadius {
    let round = (corner - GAP).max(0.0).min(height / 2.0).round() as u8;
    let at = |b: Bound| if b == Bound::Line { 0 } else { round };
    CornerRadius { nw: at(top), ne: at(top), sw: at(bottom), se: at(bottom) }
}
