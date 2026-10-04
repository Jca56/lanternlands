//! The simulation: the world and the rules that run on it. Pure std, no
//! window and no GPU, so every rule can be tested headless. The game
//! crate draws what is here and feeds it input; nothing here knows how
//! it looks.

pub mod edit;
pub mod mapfile;
pub mod noise;
pub mod terrain;
pub mod testmap;
pub mod world;

pub use terrain::Terrain;
pub use world::{CHUNK, Prop, PropKind, Tile, World};
