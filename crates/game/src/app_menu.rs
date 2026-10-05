//! The right-click menus: everything that can be done with whatever is
//! under the pointer, in the world or in a panel, and the doing of what
//! was picked. A plain press does the first thing on the list; the menu
//! is how the rest is reached.

use lntrn_props::Value;
use lntrn_ui::{ContextMenu, Item as Row};
use sim::{Skill, Slot};

use super::*;
use crate::drag::Place;

const WALK: &str = "menu.walk";
const USE: &str = "menu.use";
const ATTACK: &str = "menu.attack";
const TALK: &str = "menu.talk";
const BAG_USE: &str = "menu.bag.use";
const TAKE_OFF: &str = "menu.worn.off";
const PUT: &str = "menu.bank.put";
const TAKE: &str = "menu.bank.take";
const PUT_SOME: &str = "menu.bank.put_some";
const TAKE_SOME: &str = "menu.bank.take_some";
/// The button of the dialog that asks how many.
pub(super) const AMOUNT: &str = "menu.amount";
/// The dialog's field, where the number is typed.
pub(super) const AMOUNT_FIELD: &str = "menu.amount.field";

/// How many a menu offers to move at once, before "X" and "all".
const AMOUNTS: [u32; 3] = [1, 5, 10];
/// The most digits an amount is typed in.
const AMOUNT_DIGITS: usize = 7;

/// Some of one kind of thing going between the bag and the bank: into
/// the bank from a slot of the bag, or out of a slot of the bank.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Move {
    put: bool,
    slot: usize,
    item: Item,
}

fn act(id: &str, args: &[(&str, i64)]) -> Action {
    args.iter().fold(Action::new(id), |action, &(name, value)| action.with(name, Value::I64(value)))
}

fn int(action: &Action, name: &str) -> Option<i64> {
    match action.arg(name)? {
        Value::I64(value) => Some(*value),
        _ => None,
    }
}

fn index_of(item: Item) -> i64 {
    Item::ALL.iter().position(|i| *i == item).unwrap_or(0) as i64
}

impl Game {
    /// The menu for a point of the world's area: what can be done with
    /// the monster, the person and the thing under it, and walking
    /// there. The first row is what a plain press would have done.
    pub(super) fn world_menu(&self, pointer: Vec2) -> ContextMenu {
        let at = self.camera.to_world(pointer, self.view, self.scale);
        let tile = (at.x.floor() as i32, at.y.floor() as i32);
        let place = [("x", tile.0 as i64), ("y", tile.1 as i64)];
        let mut rows = Vec::new();
        let mut title = None;
        if let Some(i) = self.monster_under(pointer) {
            let name = self.sim.monsters[i].kind.label();
            rows.push(Row::action(&format!("Attack {name}"), act(ATTACK, &[("who", i as i64)])));
            title.get_or_insert(name);
        } else if let Some(i) = self.person_under(pointer) {
            let name = self.sim.people[i].kind.label();
            rows.push(Row::action(&format!("Talk to {name}"), act(TALK, &[("who", i as i64)])));
            title.get_or_insert(name);
        }
        if self.sim.grave.as_ref().is_some_and(|g| g.tile == tile) {
            rows.push(Row::action("Open your grave", act(USE, &place)));
            title.get_or_insert("Your grave");
        } else if let Some(prop) = self.sim.world.prop_at(tile.0, tile.1) {
            let name = panel::prop_label(prop.kind);
            let verb = match prop.kind.node().map(|node| node.skill) {
                Some(Skill::Mining) => Some("Mine"),
                Some(Skill::Herbalism) => Some("Pick"),
                Some(_) => Some("Gather"),
                None if prop.kind.station().is_some() => Some("Use"),
                None if prop.kind.is_bank() => Some("Open"),
                None => None,
            };
            if let Some(verb) = verb {
                rows.push(Row::action(&format!("{verb} {name}"), act(USE, &place)));
            }
            title.get_or_insert(name);
        }
        rows.push(Row::action("Walk here", act(WALK, &place)));
        let ground = self.sim.world.terrain(tile.0, tile.1).map_or("Nothing", panel::terrain_label);
        ContextMenu::new(title.unwrap_or(ground), pointer).tab("", rows)
    }

    /// The menu for what a slot of the bag holds: wearing or drinking
    /// it, and at the bank, putting some of it in. `None` when there is
    /// nothing to be done with it.
    pub(super) fn bag_menu(&self, slot: usize, pointer: Vec2) -> Option<ContextMenu> {
        let stack = self.sim.inventory.slots().get(slot).copied().flatten()?;
        let held = [("slot", slot as i64), ("item", index_of(stack.item))];
        let mut rows = Vec::new();
        if stack.item.slot().is_some() {
            rows.push(Row::action("Wear", act(BAG_USE, &held)));
        } else if stack.item.potion().is_some() {
            rows.push(Row::action("Drink", act(BAG_USE, &held)));
        }
        if self.sim.banking() {
            rows.extend(self.move_rows(Move { put: true, slot, item: stack.item }));
        }
        (!rows.is_empty()).then(|| ContextMenu::new(stack.item.label(), pointer).tab("", rows))
    }

    /// The menu for what a slot of the bank holds: taking some out.
    pub(super) fn bank_menu(&self, slot: usize, pointer: Vec2) -> Option<ContextMenu> {
        let stack = self.sim.bank.slots().get(slot).copied().flatten()?;
        let rows = self.move_rows(Move { put: false, slot, item: stack.item });
        Some(ContextMenu::new(stack.item.label(), pointer).tab("", rows))
    }

    /// The menu for what is worn in a slot.
    pub(super) fn worn_menu(&self, slot: Slot, pointer: Vec2) -> Option<ContextMenu> {
        let item = self.sim.gear.worn(slot)?;
        let place = Slot::ALL.iter().position(|s| *s == slot)? as i64;
        Some(ContextMenu::new(item.label(), pointer).tab("", vec![Row::action("Take off", act(TAKE_OFF, &[("slot", place)]))]))
    }

    /// The rows that move some of a kind of thing: the usual amounts,
    /// the one last typed, one to be typed, and all of it.
    fn move_rows(&self, moved: Move) -> Vec<Row> {
        let (some, all, id, id_some) = if moved.put { ("Put {} in", "Put all in", PUT, PUT_SOME) } else { ("Take {} out", "Take all out", TAKE, TAKE_SOME) };
        let what = [("slot", moved.slot as i64), ("item", index_of(moved.item))];
        let counted = |n: u32| Row::action(&some.replace("{}", &n.to_string()), act(id, &[what[0], what[1], ("count", n as i64)]));
        let mut rows: Vec<Row> = AMOUNTS.into_iter().map(counted).collect();
        rows.extend(self.last_amount.filter(|n| !AMOUNTS.contains(n)).map(counted));
        rows.push(Row::action(&(some.replace("{}", "X") + "…"), act(id_some, &what)));
        rows.push(Row::action(all, act(id, &[what[0], what[1], ("count", u32::MAX as i64)])));
        rows
    }

    /// Do what was picked from a menu. `false` when the action is not
    /// one of the menus'.
    pub(super) fn run_menu(&mut self, action: &Action, cx: &mut HostCx) -> bool {
        let int = |name: &str| int(action, name);
        let place = || Some((int("x")? as i32, int("y")? as i32));
        let moved = |put: bool| Some(Move { put, slot: int("slot")? as usize, item: *Item::ALL.get(int("item")? as usize)? });
        let from_rest = !self.sim.player.busy();
        match action.id.as_str() {
            WALK | USE | ATTACK | TALK if from_rest => self.phase = TICK_SECONDS,
            _ => {}
        }
        match action.id.as_str() {
            WALK => place().into_iter().for_each(|tile| self.sim.walk_to(tile)),
            USE => place().into_iter().for_each(|tile| self.sim.click(tile)),
            ATTACK => int("who").into_iter().for_each(|who| self.sim.attack(who as usize)),
            TALK => int("who").into_iter().for_each(|who| self.sim.talk_to(who as usize)),
            BAG_USE => {
                // Only if the slot still holds what the menu was about.
                if let Some(held) = moved(true).filter(|m| self.holds(*m)) {
                    self.sim.use_slot(held.slot);
                    self.autosave();
                }
            }
            TAKE_OFF => {
                if let Some(slot) = int("slot").and_then(|i| Slot::ALL.get(i as usize)) {
                    self.sim.unequip(*slot);
                    self.autosave();
                }
            }
            PUT | TAKE => {
                if let (Some(moved), Some(count)) = (moved(action.id == PUT), int("count")) {
                    self.shift(moved, count.clamp(0, u32::MAX as i64) as u32);
                }
            }
            PUT_SOME | TAKE_SOME => {
                if let Some(moved) = moved(action.id == PUT_SOME) {
                    self.moving = Some((moved, String::new()));
                    let title = if moved.put { "Put how many in?" } else { "Take how many out?" };
                    let dialog = Dialog::new(title, moved.item.label()).content(AMOUNT_FIELD).button("Cancel", None).button("Move", Some(Action::new(AMOUNT))).default_button(1);
                    cx.request(ShellRequest::Dialog(dialog));
                }
            }
            AMOUNT => {
                if let Some((moved, typed)) = self.moving.take()
                    && let Some(count) = typed.parse::<u32>().ok().filter(|n| *n > 0)
                {
                    self.last_amount = Some(count);
                    self.shift(moved, count);
                }
            }
            _ => return false,
        }
        true
    }

    /// Whether the slot a move is from still holds that kind of thing.
    fn holds(&self, moved: Move) -> bool {
        let held = if moved.put { self.sim.inventory.slots().get(moved.slot) } else { self.sim.bank.slots().get(moved.slot) };
        held.copied().flatten().is_some_and(|stack| stack.item == moved.item)
    }

    /// Move up to `count` of a kind of thing between the bag and the
    /// bank, and keep it at once, like anything else earned or lost.
    pub(super) fn shift(&mut self, moved: Move, count: u32) {
        if !self.holds(moved) {
            return;
        }
        if moved.put {
            self.sim.deposit(moved.slot, count);
        } else {
            self.sim.withdraw(moved.slot, count);
        }
        self.autosave();
    }

    /// Put down what was carried from one slot and let go over another:
    /// within the bag or the bank, the two slots trade; between them,
    /// the whole of what was carried goes across; gear dropped on what
    /// is worn goes on, and what is worn dropped on the bag comes off.
    pub(super) fn put_down(&mut self, from: Place, to: Place) {
        let sim = &mut self.sim;
        match (from, to) {
            (Place::Bag(a), Place::Bag(b)) => sim.inventory.swap(a, b),
            (Place::Bank(a), Place::Bank(b)) => sim.tidy_bank(a, b),
            (Place::Bag(a), Place::Bank(b)) => drop(sim.deposit_into(a, b)),
            (Place::Bank(a), Place::Bag(b)) => drop(sim.withdraw_into(a, b)),
            // Let go over any slot of the gear, it goes on where it belongs.
            (Place::Bag(a), Place::Worn(_)) => sim.equip(a),
            (Place::Worn(slot), Place::Bag(b)) => sim.unequip_into(slot, b),
            (Place::Worn(_), _) | (Place::Bank(_), Place::Worn(_)) => return,
        }
        self.autosave();
    }

    /// One of what a slot of the bag holds, into the bank.
    pub(super) fn put_one(&mut self, slot: usize) {
        if let Some(stack) = self.sim.inventory.slots().get(slot).copied().flatten() {
            self.shift(Move { put: true, slot, item: stack.item }, 1);
        }
    }

    /// One of what a slot of the bank holds, out into the bag.
    pub(super) fn take_one(&mut self, slot: usize) {
        if let Some(stack) = self.sim.bank.slots().get(slot).copied().flatten() {
            self.shift(Move { put: false, slot, item: stack.item }, 1);
        }
    }

    /// The field of the dialog that asks how many: digits only. Enter in
    /// it presses the dialog's Move.
    pub(super) fn draw_amount_field(&mut self, ui: &mut Ui, cx: &mut HostCx) -> bool {
        let Some((_, typed)) = self.moving.as_mut() else {
            return false;
        };
        let id = ui.id("How many");
        ui.request_focus_once(id);
        let field = ui.text_field("How many", typed);
        typed.retain(|c| c.is_ascii_digit());
        typed.truncate(AMOUNT_DIGITS);
        if field.committed {
            cx.request(ShellRequest::DialogDefault);
        }
        field.changed
    }

    /// Whether what is typed so far is an amount.
    pub(super) fn amount_ready(&self) -> bool {
        self.moving.as_ref().is_some_and(|(_, typed)| typed.parse::<u32>().is_ok_and(|n| n > 0))
    }
}
