//! Lanternlands: the window, the renderer and the interface over
//! the simulation in the `sim` crate.

mod anim;
mod app;
mod assets;
mod atlas;
mod bag_panel;
mod bank_panel;
mod camera;
mod drag;
mod editor;
mod gear_panel;
mod hotbar;
mod hud;
mod look;
mod making;
mod palette;
mod panel;
mod parchment;
mod render;
mod saves;
mod scene;
mod skills_panel;
mod talk;
mod title;
mod tonics;
mod wear;

use lntrn_app::{AppConfig, run};
use app::Game;

fn main() {
    // No title bar: the world has the whole window.
    let config = AppConfig { title: title::GAME_NAME.into(), app_id: app::APP_ID.into(), title_bar: false, ..AppConfig::default() };
    run(config, Game::new(), Game::shell());
}
