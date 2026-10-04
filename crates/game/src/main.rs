//! Explore the World: the window, the renderer and the interface over
//! the simulation in the `sim` crate.

mod anim;
mod app;
mod assets;
mod atlas;
mod camera;
mod editor;
mod gear_panel;
mod hotbar;
mod hud;
mod making;
mod palette;
mod panel;
mod render;
mod saves;
mod scene;
mod title;
mod tonics;

use lntrn_app::{AppConfig, run};
use lntrn_ui::Shell;

use app::{Game, View};

fn main() {
    // No title bar: the world has the whole window.
    let config = AppConfig { title: title::GAME_NAME.into(), app_id: app::APP_ID.into(), title_bar: false, ..AppConfig::default() };
    run(config, Game::new(), Shell::new(View::World));
}
