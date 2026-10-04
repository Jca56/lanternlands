//! The simulation: the world and the rules that run on it. Pure std, no
//! window and no GPU, so every rule can be tested headless. The game
//! crate draws what is here and feeds it input; nothing here knows how
//! it looks.

/// How long a tick lasts, in seconds. One step on foot takes one tick.
pub const TICK_SECONDS: f64 = 0.4;

pub mod abilities;
pub mod combat;
pub mod edit;
pub mod items;
pub mod mapfile;
pub mod monsters;
pub mod nodes;
pub mod noise;
pub mod path;
pub mod player;
pub mod rng;
pub mod skills;
pub mod save;
pub mod terrain;
pub mod testmap;
mod wire;
pub mod world;

pub use abilities::Ability;
pub use items::{Inventory, Item, Stack};
pub use nodes::Node;
pub use combat::Grave;
pub use monsters::{Monster, MonsterKind};
pub use player::{Activity, Event, Player, Sim, Who};
pub use save::{Character, Saved};
pub use skills::{Skill, Skills};
pub use terrain::Terrain;
pub use world::{CHUNK, Marker, Prop, PropKind, Tile, World};
