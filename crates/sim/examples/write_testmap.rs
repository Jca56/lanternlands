//! Writes the made-up test map out as a real map:
//! `cargo run -p sim --example write_testmap -- assets/maps/overworld`.
//! It was run once, to give the editor something to start from.

use std::path::PathBuf;

fn main() {
    let Some(dir) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: write_testmap <folder>");
        std::process::exit(2);
    };
    let mut world = sim::testmap::build();
    // Every tile was just set, so every chunk is written.
    match world.save(&dir) {
        Ok(files) => println!("wrote {files} chunk files to {}", dir.display()),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
