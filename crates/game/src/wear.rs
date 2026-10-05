//! Dressing whoever stands in the world: the layers a body and what is
//! worn over it are drawn in, and what each person wears.

use lntrn_math::Vec2;
use sim::{Metal, PersonKind, Slot};

use crate::anim::{Facing, Pose, WalkSprites};
use crate::scene::{GearSprites, Placed, metal_tint};

/// The colours of what people wear that is no metal.
const LEATHER: [f32; 4] = [0.471, 0.322, 0.188, 1.0];
const DARK_LEATHER: [f32; 4] = [0.290, 0.204, 0.137, 1.0];

/// What a person is drawn wearing over their own clothes, bottom
/// first, and the colour of each piece: the drawings are the gear's.
pub const fn person_wear(kind: PersonKind) -> &'static [(Slot, [f32; 4])] {
    const BRONZE: [f32; 4] = metal_tint(Metal::Bronze);
    match kind {
        // An apron and gloves, against the sparks.
        PersonKind::Smith => &[(Slot::Body, LEATHER), (Slot::Hands, DARK_LEATHER)],
        PersonKind::Herbalist | PersonKind::Banker => &[],
        // Bronze from head to foot, sword and shield in hand.
        PersonKind::Guard => &[(Slot::Legs, BRONZE), (Slot::Feet, BRONZE), (Slot::Body, BRONZE), (Slot::Hands, BRONZE), (Slot::Head, BRONZE), (Slot::Shield, BRONZE), (Slot::Weapon, BRONZE)],
    }
}

/// Stand someone in `extras` in layers: what they wear that shows from
/// behind them, their body, then what they wear over it, each piece in
/// its own colour.
pub fn dress(extras: &mut Vec<Placed>, ground: Vec2, facing: Facing, pose: Pose, body: &WalkSprites, worn: &[(&GearSprites, [f32; 4])]) {
    let mut put = |sprites: &WalkSprites, tint| {
        if let Some((sprite, flip)) = sprites.sprite(facing, pose) {
            extras.push(Placed { sprite, ground, flip, tint });
        }
    };
    for &(gear, tint) in worn {
        put(&gear.under, tint);
    }
    put(body, [1.0; 4]);
    for &(gear, tint) in worn {
        put(&gear.over, tint);
    }
}
