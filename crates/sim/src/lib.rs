//! The simulation: the world and the rules that run on it. Pure std, no
//! window and no GPU, so every rule can be tested headless. The game
//! crate draws what is here and feeds it input; nothing here knows how
//! it looks.

/// How long a tick lasts, in seconds. One step on foot takes one tick.
pub const TICK_SECONDS: f64 = 0.4;

pub mod edit;
pub mod mapfile;
pub mod noise;
pub mod path;
pub mod player;
pub mod terrain;
pub mod testmap;
pub mod world;

pub use player::{Player, Sim};
pub use terrain::Terrain;
pub use world::{CHUNK, Marker, Prop, PropKind, Tile, World};
