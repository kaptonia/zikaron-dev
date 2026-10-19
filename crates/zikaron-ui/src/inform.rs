//! What a form card's rows ([`crate::fields::rows`]) show besides their own parts: the quiet word after a row's
//! title ("optional"), a chosen place at a row's end, and a mark with words.

use crate::palette::{c, C};
use crate::tokens::{self, Type};
use egui::{pos2, vec2, Rect};

/// The quiet word after the title of a row (`row`, as [`card::Form::line`] answers it, with no lead and no
/// second line): 8 after the title, 13, the third ink.
pub fn after_title(ui: &egui::Ui, row: Rect, title: &str, word: &str) {
    let p = ui.painter();
    let tw = p.layout_no_wrap(title.to_string(), egui::FontId::new(15.0, crate::fonts::medium()), c(C::Ink)).size().x;
    // The row's words stand in by the card's inset, or by none in a group laid without a ground of its own.
    let inset = if crate::sheet::inside(ui.ctx()) { 0.0 } else { tokens::FORM_PAD_X };
    let x = row.left() + inset + tw + 8.0;
    p.text(pos2(x, row.center().y), egui::Align2::LEFT_CENTER, word, crate::tokens::Type::Small.font(), c(C::Ink3));
}

/// What a row holds at its right end before its key (laid right to left, after the key): the place or file
/// chosen (monospace 13, elided in the middle within `room`, whole on hover), else `none` in the quiet ink.
pub fn place(ui: &mut egui::Ui, path: &str, none: &str, room: f32) {
    if path.trim().is_empty() {
        ui.label(egui::RichText::new(none).font(Type::Note.font()).color(c(C::Ink3)));
        return;
    }
    let font = Type::MonoSmall.font();
    let shown = crate::width::elide_mid(ui, path, font.clone(), room);
    let r = ui.label(egui::RichText::new(&shown).font(font).color(c(C::Ink)));
    if shown != path {
        let _ = crate::layer::tip(r, path);
    }
}

/// A mark and the words after it at a row's right end (a record's state on chain): 8 apart, the words 14 in the
/// second ink. Draw it in the row's trail (laid right to left).
pub fn mark_words(ui: &mut egui::Ui, m: crate::mark::Mark, words: &str) {
    if !words.is_empty() {
        ui.label(egui::RichText::new(words).font(Type::Note.font()).color(c(C::Ink2)));
    }
    let (r, _) = ui.allocate_exact_size(vec2(tokens::MARK, tokens::MARK), egui::Sense::hover());
    crate::mark::paint_mark(ui.ctx(), ui.painter(), r.center(), m, tokens::MARK);
}
