//! The passcode gate: a full-window cover with eight passcode cells and an error line that opens under them
//! only when there is an error (no toasts on the gate). "Forgot passcode" recovers by the primary identity's
//! kind (its twelve words, or its key file and that file's password, then a new passcode twice); restoring a
//! whole-machine backup is a third way.
//!
//! The gate only opens the vault; actions that need a key are refused by the action layer either way.

use super::*;

impl Win {
    pub(super) fn gate(&mut self, ctx: &egui::Context, now: f64) {
        let state = self.shell.vault.clone();
        // `Vault::gate_up` decides whether the gate or the shell draws.
        if !state.gate_up() {
            return;
        }
        // The store file exists but cannot be read: show the store's refusal and offer nothing that would build a
        // new store over the keys it holds (no wizard, no passcode cells).
        if let crate::shell::Vault::Damaged(f) = &state {
            full::cover(ctx, "gate", 0.0, |ui, drop| {
                let rect = ui.max_rect();
                full::centered(ui, "gate", rect, tk::SHEET_W, drop, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        paint::text(ui, t(Key::AppName), Type::Small, c(C::Ink3));
                        paint::text(ui, t(Key::VaultDamagedTitle), Type::Page, c(C::Ink));
                        ui.add_space(18.0);
                    });
                    states::err_box(ui, "gate-damaged", f.human(), &f.next(), t(Key::U3RawError), &f.raw());
                });
            });
            return;
        }
        let locked_out = state.is(crate::keybox::State::LockedOut);
        if locked_out {
            self.ux.gate_recover = true;
        }
        // An empty vault that is locked out (passcode set, no identity) cannot be recovered and holds nothing to
        // lose, so the only way on is to reset it and run the wizard again.
        let empty_out = locked_out && !self.shell.vault_recoverable;
        let busy = self.shell.tasks.in_flight(crate::task::Kind::Vault);
        let recover = self.ux.gate_recover;
        // The recovery way follows the primary identity's kind, read from the vault header before unlocking: a
        // recovery-word primary recovers with its words, or (folded under them) with a key file either of its
        // roles exported; an imported-key primary with the key file it exported. Without a primary there is no
        // recovery seal, and only a whole-machine backup remains.
        let (by_words, by_file) = match self.shell.primary.as_ref().map(|(_, k)| *k) {
            Some(crate::keybox::PrimaryKind::Words) => (true, true),
            Some(crate::keybox::PrimaryKind::KeyFile) => (false, true),
            None => (false, false),
        };
        let mut from_backup = false;
        let len = crate::keybox::PIN_LEN;
        let mut first_pin = false;
        let mut act: Option<Action> = None;
        full::cover(ctx, "gate", 0.0, |ui, drop| {
            let rect = ui.max_rect();
            full::centered(ui, "gate", rect, tk::SHEET_W, drop, |ui| {
                ui.vertical_centered(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    paint::text(ui, t(Key::AppName), Type::Small, c(C::Ink3));
                    let title = if !recover {
                        Key::PinGateTitle
                    } else if locked_out {
                        Key::PinLockedTitle
                    } else {
                        Key::PinRecoverTitle
                    };
                    paint::text(ui, t(title), Type::Page, c(C::Ink));
                });
                if !recover {
                    ui.vertical_centered(|ui| {
                        ui.add_space(28.0);
                        let row = pin::pin_row(ui, "gate-pin", &mut self.ux.pin, len, self.ux.pin_shake, !busy, false);
                        // Typing again clears the last refusal, except on the reseal path, whose sentence says what
                        // the next passcode will do.
                        if row.changed && !self.ux.gate_reseal {
                            self.ux.gate_trouble = None;
                        }
                        if row.full && !busy {
                            let pin = std::mem::take(&mut self.ux.pin);
                            act = Some(if self.ux.gate_reseal { Action::Reseal { pin } } else { Action::Unlock { pin } });
                        }
                        gate_message(ui, busy, self.ux.gate_trouble.as_deref(), 18.0);
                        ui.add_space(16.0);
                        if key::key(ui, t(Key::PinForgot), Role::Plain, true).clicked() {
                            self.ux.gate_recover = true;
                            self.ux.gate_trouble = None;
                        }
                    });
                    return;
                }
                ui.vertical_centered(|ui| {
                    ui.add_space(6.0);
                    if empty_out {
                        paint::text(ui, t(Key::PinEmptySay), Type::Note, c(C::Ink2));
                    } else if by_words || by_file {
                        let how = t(if by_words { Key::GateByWords } else { Key::GateByFile });
                        let say = if locked_out { format!("{}{}", t(Key::GateLockedFive), how) } else { how.to_string() };
                        paint::text(ui, &say, Type::Note, c(C::Ink2));
                    }
                });
                ui.add_space(18.0);
                if empty_out {
                    ui.vertical_centered(|ui| {
                        if key::key(ui, t(Key::PinDoReset), Role::Secondary, true).clicked() {
                            act = Some(Action::ResetEmptyKeybox);
                        }
                        gate_message(ui, false, self.ux.gate_trouble.as_deref(), 12.0);
                    });
                    return;
                }
                ui.spacing_mut().item_spacing.y = tk::S3;
                if by_words {
                    // The two ways exclude each other: touching the key file fold makes it the active one.
                    let by_file = !self.ux.pin_ks_path.trim().is_empty() || !self.ux.pin_ks_pw.is_empty();
                    let title = t(if self.ux.pin_again.is_empty() { Key::WizPinTitle } else { Key::PinAgain });
                    let row = words_way(ui, &mut self.ux.pin_words, &mut self.ux.gate_words_ready, &mut self.ux.pin, len, self.ux.pin_shake, title, !by_file && !busy);
                    if row.is_some_and(|r| r.full) {
                        if self.ux.pin_again.is_empty() {
                            first_pin = true;
                        } else if !busy {
                            act = Some(Action::RecoverWords { words: joined(&self.ux.pin_words), pin: self.ux.pin_again.clone(), again: std::mem::take(&mut self.ux.pin) });
                        }
                    }
                }
                if by_words {
                    gate_message(ui, busy, self.ux.gate_trouble.as_deref(), 4.0);
                }
                // An imported-key primary recovers with the key file it exported and that file's password (shown
                // open); a recovery-word primary's key files are folded under its words.
                let by_key_file = |me: &mut Self, ui: &mut egui::Ui, say: Key, password: Key, first_pin: &mut bool, act: &mut Option<Action>| {
                    let title = t(if me.ux.pin_again.is_empty() { Key::WizPinTitle } else { Key::PinAgain });
                    let row = file_way(ui, &mut me.ux.pin_ks_path, &mut me.ux.pin_ks_pw, &mut me.ux.gate_file_ready, &mut me.ux.pin, len, me.ux.pin_shake, t(say), t(password), title, busy);
                    if row.full && me.ux.pin_again.is_empty() {
                        *first_pin = true;
                    } else if row.full && !busy {
                        *act = Some(Action::RecoverKeystore {
                            path: me.ux.pin_ks_path.clone(),
                            password: me.ux.pin_ks_pw.clone(),
                            pin: me.ux.pin_again.clone(),
                            again: std::mem::take(&mut me.ux.pin),
                        });
                    }
                };
                if by_file && !by_words {
                    by_key_file(self, ui, Key::GateDropKeyFile, Key::GateFilePassword, &mut first_pin, &mut act);
                    gate_message(ui, busy, self.ux.gate_trouble.as_deref(), 4.0);
                }
                if by_file && by_words {
                    fold::fold(ui, "gate-keystore", t(Key::PinByFile), |ui| by_key_file(self, ui, Key::PinByFileSay, Key::IdPassword, &mut first_pin, &mut act));
                }
                // The third way: restore from a whole-machine backup (new passcode, everything resealed).
                ui.vertical_centered(|ui| {
                    from_backup = key::key(ui, t(Key::BackupRestoreTitle), Role::Plain, !busy).clicked();
                });
                if !locked_out {
                    ui.vertical_centered(|ui| {
                        ui.add_space(2.0);
                        if key::key(ui, t(Key::CfBack), Role::Secondary, true).clicked() {
                            self.ux.gate_back();
                        }
                    });
                }
            });
        });
        if from_backup {
            self.bk_open(Bk::Restore(RestoreFrom::Locked));
        }
        if first_pin {
            self.take_first_pin(now, true);
        }
        if let Some(a) = act {
            let r = self.act(a, now);
            self.vault_or(VaultSite::Gate, r, now);
        }
    }
}

/// The twelve recovery word cells and, once all twelve are filled, the new passcode row under `title` (taking
/// keys when `row_on`); the row's reading when it is drawn. The cells are always masked, a pasted phrase fills
/// them all and an unknown word turns red without being shown.
///
/// Focus goes to the passcode row only when the user says so, never because of what the cells hold (a word may be
/// the start of another, `act` of `action`, and a cell holding one is not done): Enter in a cell lets it go and the
/// row, enabled once all twelve are filled, takes it; Tab moves on to the row; a click on the row takes it; one
/// pasted phrase that fills the twelve hands it over here. `was_full` is whether the cells were all filled the
/// frame before.
fn words_way(ui: &mut egui::Ui, words: &mut [crate::secret::Secret; 12], was_full: &mut bool, pin: &mut crate::secret::Secret, len: usize, shake: Option<f64>, title: &str, row_on: bool) -> Option<pin::PinRow> {
    ui.spacing_mut().item_spacing.y = tk::S3;
    let bad = crate::cryptx::strangers(words);
    let phrase = ui.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Paste(x) if x.split_whitespace().count() > 1)));
    let changed = pin::words_grid_marked(ui, "gate-words", words, &bad);
    hint(ui, t(Key::PinWordsHint));
    let full = words.iter().all(|w| !w.expose().trim().is_empty());
    if changed && phrase && full && !*was_full {
        if let Some(id) = ui.memory(|m| m.focused()) {
            ui.memory_mut(|m| m.surrender_focus(id));
        }
    }
    *was_full = full;
    full.then(|| field(ui, title, None, |ui| pin::pin_row(ui, "gate-new-pin", pin, len, shake, row_on, true)))
}

/// The key file way: the file, its password, and the new passcode row under `title`, enabled only once file and
/// password are both filled (only one row may hold focus, or the two entries would split between rows and never
/// match); `ready` keeps whether they are. Focus goes to the row only when the user says so, never because the
/// password field holds something (its length says nothing of being done): Enter in the password field lets it go
/// and the enabled row takes it, Tab moves on to the row, a click on the row takes it.
fn file_way(ui: &mut egui::Ui, path: &mut String, password: &mut crate::secret::Secret, ready: &mut bool, pin: &mut crate::secret::Secret, len: usize, shake: Option<f64>, say: &str, password_title: &str, title: &str, busy: bool) -> pin::PinRow {
    ui.spacing_mut().item_spacing.y = tk::S3;
    paint::text(ui, say, Type::Note, c(C::Ink2));
    pick_path(ui, path, crate::platform::Pick::File);
    field(ui, password_title, None, |ui| input::secret_line(ui, password, ""));
    *ready = !path.trim().is_empty() && !password.is_empty();
    field(ui, title, None, |ui| pin::pin_row(ui, "gate-file-pin", pin, len, shake, *ready && !busy, true))
}

/// The line under the gate's cells: nothing (no height), "verifying…", or the refusal in a grey box.
/// It opens and closes by animating its height.
fn gate_message(ui: &mut egui::Ui, busy: bool, trouble: Option<&str>, gap: f32) {
    let id = ui.id().with("gate-message");
    let on = busy || trouble.is_some();
    let open = motion::flag(ui.ctx(), id, on, tk::MID);
    if open <= 0.0 {
        return;
    }
    let h_id = id.with("h");
    let full_h = ui.ctx().data(|d| d.get_temp::<f32>(h_id)).unwrap_or(40.0);
    let shown_h = (gap + full_h) * open;
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, shown_h), egui::Sense::hover());
    let inner = egui::Rect::from_min_size(egui::pos2(rect.left(), rect.top() + gap * open), egui::vec2(w, full_h));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::top_down(egui::Align::Center)));
    child.set_clip_rect(rect.intersect(ui.clip_rect()));
    child.multiply_opacity(open);
    let r = child.vertical_centered(|ui| {
        if busy {
            sheet::busy_note(ui, t(Key::VaultBusy), true);
        } else if let Some(s) = trouble {
            states::note_box(ui, s);
        }
    });
    let got = r.response.rect.height();
    if on && (got - full_h).abs() > 0.5 {
        ui.ctx().data_mut(|d| d.insert_temp(h_id, got));
        // Lay out again at once with the new size so no frame is drawn with the old one.
        ui.ctx().request_discard("gate message height changed");
    }
}

#[cfg(test)]
mod tests {
    //! The recovery cells hand focus to the passcode row only when the user says so (Enter, Tab, a click, one
    //! pasted phrase that fills the twelve), never because of what a cell holds. Driven over the gate's own two
    //! ways in windowless frames: keys typed one frame each, as a keyboard sends them.
    use super::*;
    use crate::secret::Secret;

    /// Eleven words of the list and a twelfth that a shorter word of the list begins (`act`, `action`).
    const WORDS: [&str; 12] = ["abandon"; 12];
    const LAST: &str = "action";

    struct Drive {
        ctx: egui::Context,
        t: f64,
    }

    impl Drive {
        fn new() -> Drive {
            let ctx = egui::Context::default();
            let _ = zikaron_ui::skin::dress(&ctx);
            Drive { ctx, t: 0.0 }
        }

        fn frame(&mut self, events: Vec<egui::Event>, draw: &mut dyn FnMut(&mut egui::Ui)) {
            self.t += 0.05;
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(520.0, 900.0));
            let input = egui::RawInput { screen_rect: Some(screen), time: Some(self.t), events, ..Default::default() };
            let _ = self.ctx.run_ui(input, |ui| {
                egui::CentralPanel::default().show_inside(ui, |ui| draw(ui));
            });
        }

        fn typed(&mut self, text: &str, draw: &mut dyn FnMut(&mut egui::Ui)) {
            for ch in text.chars() {
                self.frame(vec![egui::Event::Text(ch.to_string())], draw);
            }
        }
    }

    fn key(k: egui::Key) -> egui::Event {
        egui::Event::Key { key: k, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE }
    }

    #[derive(Default)]
    struct Words {
        words: [Secret; 12],
        full: bool,
        pin: Secret,
    }

    fn words_draw(w: &mut Words) -> impl FnMut(&mut egui::Ui) + '_ {
        move |ui| {
            let _ = words_way(ui, &mut w.words, &mut w.full, &mut w.pin, crate::keybox::PIN_LEN, None, "passcode", true);
        }
    }

    /// Type the twelve words cell by cell (Tab between cells), the twelfth one key at a time.
    fn type_twelve(d: &mut Drive, w: &mut Words) {
        for _ in 0..2 {
            d.frame(Vec::new(), &mut words_draw(w));
        }
        d.frame(vec![key(egui::Key::Tab)], &mut words_draw(w));
        for (i, word) in WORDS.iter().enumerate() {
            d.typed(if i == 11 { LAST } else { word }, &mut words_draw(w));
            if i < 11 {
                d.frame(vec![key(egui::Key::Tab)], &mut words_draw(w));
            }
        }
    }

    /// Typing the twelfth word key by key, a word of the list that is the start of it does not move focus: the
    /// whole word lands in the twelfth cell and nothing in the passcode row.
    #[test]
    fn the_twelfth_word_typed_key_by_key_keeps_the_focus() {
        let (mut d, mut w) = (Drive::new(), Words::default());
        type_twelve(&mut d, &mut w);
        assert_eq!(w.words[11].expose(), LAST);
        assert!(w.words[..11].iter().all(|x| x.expose() == "abandon"));
        assert!(w.pin.is_empty(), "nothing reached the passcode row");
        // More keys still go to the cell.
        d.typed("s", &mut words_draw(&mut w));
        assert_eq!(w.words[11].expose(), "actions");
        assert!(w.pin.is_empty());
    }

    /// Enter in the twelfth cell, or Tab, moves focus to the passcode row: the next keys are the passcode's.
    #[test]
    fn enter_or_tab_in_the_last_cell_moves_to_the_passcode_row() {
        for k in [egui::Key::Enter, egui::Key::Tab] {
            let (mut d, mut w) = (Drive::new(), Words::default());
            type_twelve(&mut d, &mut w);
            d.frame(vec![key(k)], &mut words_draw(&mut w));
            d.typed("ab12", &mut words_draw(&mut w));
            assert_eq!(w.pin.expose(), "ab12", "{k:?}");
            assert_eq!(w.words[11].expose(), LAST, "{k:?}");
        }
    }

    /// One pasted phrase that fills the twelve moves focus to the passcode row; a phrase that fills fewer does not.
    #[test]
    fn a_pasted_phrase_that_fills_the_twelve_moves_to_the_passcode_row() {
        let phrase = format!("{} {LAST}", WORDS[..11].join(" "));
        let (mut d, mut w) = (Drive::new(), Words::default());
        for _ in 0..2 {
            d.frame(Vec::new(), &mut words_draw(&mut w));
        }
        d.frame(vec![key(egui::Key::Tab)], &mut words_draw(&mut w));
        d.frame(vec![egui::Event::Paste(phrase)], &mut words_draw(&mut w));
        assert_eq!(w.words[11].expose(), LAST);
        d.typed("ab12", &mut words_draw(&mut w));
        assert_eq!(w.pin.expose(), "ab12");

        let (mut d, mut w) = (Drive::new(), Words::default());
        for _ in 0..2 {
            d.frame(Vec::new(), &mut words_draw(&mut w));
        }
        d.frame(vec![key(egui::Key::Tab)], &mut words_draw(&mut w));
        d.frame(vec![egui::Event::Paste("abandon abandon".into())], &mut words_draw(&mut w));
        d.typed("x", &mut words_draw(&mut w));
        assert!(w.pin.is_empty() && w.words[0].expose() == "abandonx", "a short paste leaves focus in the cell");
    }

    #[derive(Default)]
    struct File {
        path: String,
        password: Secret,
        ready: bool,
        pin: Secret,
    }

    fn file_draw(f: &mut File) -> impl FnMut(&mut egui::Ui) + '_ {
        move |ui| {
            let _ = file_way(ui, &mut f.path, &mut f.password, &mut f.ready, &mut f.pin, crate::keybox::PIN_LEN, None, "file", "password", "passcode", false);
        }
    }

    /// With the key file chosen, the password typed key by key stays in the password field (its first key no
    /// longer hands focus over); Enter, or Tab, then moves focus to the passcode row.
    #[test]
    fn the_key_file_password_typed_keeps_the_focus_until_enter_or_tab() {
        for k in [egui::Key::Enter, egui::Key::Tab] {
            let mut d = Drive::new();
            let mut f = File { path: "/somewhere/key.json".into(), ..Default::default() };
            for _ in 0..2 {
                d.frame(Vec::new(), &mut file_draw(&mut f));
            }
            // Tab on to the password field (past the choose key), then type it.
            for _ in 0..3 {
                d.frame(vec![key(egui::Key::Tab)], &mut file_draw(&mut f));
                d.typed("h", &mut file_draw(&mut f));
                if !f.password.is_empty() {
                    break;
                }
            }
            d.typed("unter2-long", &mut file_draw(&mut f));
            assert_eq!(f.password.expose(), "hunter2-long", "{k:?}");
            assert!(f.pin.is_empty(), "{k:?}: nothing reached the passcode row");
            d.frame(vec![key(k)], &mut file_draw(&mut f));
            d.typed("ab12", &mut file_draw(&mut f));
            assert_eq!(f.pin.expose(), "ab12", "{k:?}");
            assert_eq!(f.password.expose(), "hunter2-long", "{k:?}");
        }
    }
}
