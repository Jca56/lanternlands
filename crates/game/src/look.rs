//! Which drawing each thing in the world is shown with: the props, the
//! player and what they wear, the monsters, the people, and the few
//! odd things that lie about (a tuft, a gravestone, a ring).

use sim::{Metal, PersonKind, PropKind, Slot};

use crate::anim::WalkSprites;
use crate::assets::{SpriteId, SpriteSet};

/// Which sprite each thing in the world is drawn with.
#[derive(Clone, Copy, Debug, Default)]
pub struct Look {
    /// What each kind of prop is drawn with, in the order of
    /// `PropKind::ALL`: the sprite called what the kind is.
    props: [Option<SpriteId>; PropKind::ALL.len()],
    /// The rails from a fence's post to the fence east, west and south
    /// of it.
    pub fence_east: Option<SpriteId>,
    pub fence_west: Option<SpriteId>,
    pub fence_south: Option<SpriteId>,
    /// A rock and a herb that have given all they had.
    pub rock_spent: Option<SpriteId>,
    herb_picked: Option<SpriteId>,
    pub tuft: Option<SpriteId>,
    /// What marks where the player died.
    pub gravestone: Option<SpriteId>,
    /// The ring on the ground under what the player is set on, and a
    /// wider one for under what is wider than a tile.
    pub target_ring: Option<SpriteId>,
    pub target_ring_wide: Option<SpriteId>,
    pub player: WalkSprites,
    pub goblin: WalkSprites,
    /// What each person is drawn with, in the order of `PersonKind::ALL`.
    people: [WalkSprites; PersonKind::ALL.len()],
    /// What gear is drawn with, slot by slot.
    gear: [GearSprites; Slot::ALL.len()],
}

/// The drawings of what is worn in one slot, laid out like the player's
/// own: what goes over the body, and what shows from behind it.
#[derive(Clone, Copy, Debug, Default)]
pub struct GearSprites {
    pub over: WalkSprites,
    pub under: WalkSprites,
}

/// The order gear is drawn in over the body, bottom first.
pub const GEAR_ORDER: [Slot; 7] = [Slot::Legs, Slot::Feet, Slot::Body, Slot::Hands, Slot::Head, Slot::Shield, Slot::Weapon];

/// The colour gear's plain greys are multiplied by: its metal's.
pub const fn metal_tint(metal: Metal) -> [f32; 4] {
    match metal {
        Metal::Bronze => [0.878, 0.627, 0.353, 1.0],
    }
}

/// The name of a person's sheet in the assets.
fn person_sheet(kind: PersonKind) -> String {
    format!("person_{}", kind.name())
}

impl Look {
    pub fn find(set: &SpriteSet) -> Look {
        Look {
            props: PropKind::ALL.map(|kind| set.id(kind.name())),
            fence_east: set.id("fence_east"),
            fence_west: set.id("fence_west"),
            fence_south: set.id("fence_south"),
            rock_spent: set.id("rock_spent"),
            herb_picked: set.id("herb_picked"),
            tuft: set.id("tuft"),
            gravestone: set.id("gravestone"),
            target_ring: set.id("target_ring"),
            target_ring_wide: set.id("target_ring_wide"),
            player: WalkSprites::find(set, "player"),
            goblin: WalkSprites::find(set, "goblin"),
            people: PersonKind::ALL.map(|kind| WalkSprites::find(set, &person_sheet(kind))),
            gear: Slot::ALL.map(|slot| GearSprites { over: WalkSprites::find(set, &format!("gear_{}", slot.name())), under: WalkSprites::find(set, &format!("gear_{}_under", slot.name())) }),
        }
    }

    /// The names of the sprites it looked for and did not find.
    pub fn missing(&self) -> Vec<String> {
        let props = PropKind::ALL.into_iter().zip(self.props).map(|(kind, id)| (kind.name(), id));
        let all = props.chain([("fence_east", self.fence_east), ("fence_west", self.fence_west), ("fence_south", self.fence_south), ("rock_spent", self.rock_spent), ("herb_picked", self.herb_picked), ("tuft", self.tuft), ("gravestone", self.gravestone), ("target_ring", self.target_ring), ("target_ring_wide", self.target_ring_wide)]);
        let people = PersonKind::ALL.into_iter().zip(&self.people).flat_map(|(kind, sprites)| sprites.missing(&person_sheet(kind)));
        all.filter(|(_, id)| id.is_none()).map(|(name, _)| name.to_owned()).chain(self.player.missing("player")).chain(self.goblin.missing("goblin")).chain(people).collect()
    }

    /// The sprite a prop is drawn with once it has given all it had.
    pub fn spent(&self, kind: PropKind) -> Option<SpriteId> {
        match kind {
            PropKind::Rock | PropKind::TinRock => self.rock_spent,
            PropKind::HerbPurple | PropKind::HerbCyan => self.herb_picked,
            _ => None,
        }
    }

    /// The ring that lies under a kind of prop: the wide one under what
    /// is drawn wider than a tile, so it shows all round.
    pub fn ring_under(&self, set: &SpriteSet, kind: PropKind) -> Option<SpriteId> {
        let wide = self.prop(kind).is_some_and(|id| set.get(id).size.x > 1.0);
        self.target_ring_wide.filter(|_| wide).or(self.target_ring)
    }

    /// The drawings of what is worn in a slot.
    pub fn gear(&self, slot: Slot) -> &GearSprites {
        &self.gear[Slot::ALL.iter().position(|s| *s == slot).expect("every slot is listed")]
    }

    /// The drawings a person is shown with.
    pub fn person(&self, kind: PersonKind) -> &WalkSprites {
        &self.people[PersonKind::ALL.iter().position(|k| *k == kind).expect("everyone is listed")]
    }

    /// The drawings a kind of monster is shown with.
    pub fn monster(&self, kind: sim::MonsterKind) -> &WalkSprites {
        match kind {
            sim::MonsterKind::Goblin => &self.goblin,
        }
    }

    /// The sprite a kind of prop is drawn with, if the assets have it.
    pub fn prop(&self, kind: PropKind) -> Option<SpriteId> {
        self.props[PropKind::ALL.iter().position(|k| *k == kind).expect("every kind is listed")]
    }
}
