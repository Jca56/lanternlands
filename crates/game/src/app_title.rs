//! The title screen's doings and the characters' saves: picking,
//! making and deleting characters, and keeping the one in play saved.

use sim::{Character, Saved};

use super::*;
use crate::title::{self, Chosen, Slot};

/// The dialog content that asks for a new character's name.
pub(super) const NAME_FIELD: &str = "title.name";

impl Game {
    /// Look at the save folder again: what each slot holds.
    pub(super) fn refresh_slots(&mut self) {
        self.slots = std::array::from_fn(|slot| match self.saves.read(slot) {
            None => Slot::Empty,
            Some(Ok(saved)) => Slot::Held { name: saved.character.name, total_level: saved.skills.total_level(), played: saved.character.played },
            Some(Err(why)) => {
                log_error!("a save cannot be read: {why}");
                Slot::Broken(why)
            }
        });
    }

    /// The title screen, and what picking something on it does.
    pub(super) fn draw_title(&mut self, ui: &mut Ui, cx: &mut AreaCx<()>) {
        match title::draw(ui, self.view, self.title_picture, &self.slots) {
            Some(Chosen::Continue(slot)) => match self.saves.read(slot) {
                Some(Ok(saved)) => self.enter(slot, saved.character.clone(), Some(&saved)),
                // It went, or went bad, since the slots were last looked at.
                _ => self.refresh_slots(),
            },
            Some(Chosen::New(slot)) => {
                self.naming = Some((slot, String::new()));
                let dialog = Dialog::new("New character", "").content(NAME_FIELD).button("Cancel", None).button("Start", Some(Action::new(CREATE))).default_button(1);
                cx.request(ShellRequest::Dialog(dialog));
            }
            Some(Chosen::Delete(slot)) => {
                self.deleting = Some(slot);
                let who = match &self.slots[slot] {
                    Slot::Held { name, .. } => name.as_str(),
                    _ => "this save",
                };
                cx.request(ShellRequest::Dialog(Dialog::confirm(&format!("Delete {who}?"), "This character will be gone for good.", "Delete", Action::new(DELETE))));
            }
            Some(Chosen::Quit) => self.quit(&mut cx.host()),
            None => {}
        }
        if self.mode != Mode::Title {
            // A character was entered: draw the game, not one more title frame.
            cx.rebuild();
        }
    }

    /// The name field of the new-character dialog. Enter in it presses
    /// the dialog's Start.
    pub(super) fn draw_name_field(&mut self, ui: &mut Ui, cx: &mut HostCx) -> bool {
        let Some((_, name)) = self.naming.as_mut() else {
            return false;
        };
        let id = ui.id("Name");
        ui.request_focus_once(id);
        let field = ui.text_field("Name", name);
        if field.committed {
            cx.request(ShellRequest::DialogDefault);
        }
        field.changed
    }

    /// Whether the name typed so far would do.
    pub(super) fn name_ready(&self) -> bool {
        self.naming.as_ref().is_some_and(|(_, name)| title::valid_name(name).is_some())
    }

    /// Start the character whose name was just typed, in the slot that
    /// was picked for them.
    pub(super) fn create_character(&mut self) {
        let Some((slot, typed)) = self.naming.take() else { return };
        let Some(name) = title::valid_name(&typed) else { return };
        self.enter(slot, Character { name, played: 0 }, None);
        self.autosave();
    }

    /// Delete the character that was asked about.
    pub(super) fn delete_character(&mut self, cx: &mut HostCx) {
        let Some(slot) = self.deleting.take() else { return };
        if let Err(why) = self.saves.delete(slot) {
            log_error!("the save was not deleted: {why}");
            cx.request(ShellRequest::Dialog(Dialog::notice("The save was not deleted", &why)));
        }
        self.refresh_slots();
    }

    /// Start playing a character: a new one, or `saved` as it was left.
    pub(super) fn enter(&mut self, slot: usize, character: Character, saved: Option<&Saved>) {
        let world = std::mem::take(&mut self.sim.world);
        self.sim = match saved {
            Some(saved) => Sim::resume(world, saved),
            None => Sim::new(world),
        };
        if self.dice_from_clock {
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
            self.sim.reseed(now.map_or(0, |since| since.as_nanos() as u64));
        }
        self.played = character.played as f64;
        self.character = character;
        self.mode = Mode::Playing { slot };
        let (x, y) = self.sim.player.tile;
        self.camera.center = Vec2::new(x as f64 + 0.5, y as f64 + 0.5);
        self.facing = Facing::default();
        self.phase = TICK_SECONDS;
        self.hud = Hud::default();
        self.unsaved = false;
        self.saved_at = self.clock.unwrap_or(0.0);
    }

    /// Leave the game for the title screen, saving on the way.
    pub(super) fn back_to_title(&mut self) {
        self.autosave();
        self.mode = Mode::Title;
        self.refresh_slots();
    }

    /// Write the character in play to their slot. Nothing to do on the
    /// title screen.
    pub(super) fn autosave(&mut self) {
        let Mode::Playing { slot } = self.mode else { return };
        self.character.played = self.played as u64;
        let now = self.clock.unwrap_or(0.0);
        match self.saves.write(slot, &self.sim.snapshot(&self.character)) {
            Ok(()) => (self.unsaved, self.saved_at) = (false, now),
            Err(why) => {
                log_error!("the game was not saved: {why}");
                self.hud.say("The game could not be saved", now);
                // Not at once again: the next try is a while off.
                self.saved_at = now;
            }
        }
    }
}
