//! Deterministic framebuffer fixtures, independent of Android's display scale.
use std::{
    env, fs,
    io::{self, Write},
    path::Path,
};
use tetris_nes_core::{Game, Scene, A, DOWN, RIGHT, START};

fn write_frame(game: &Game, path: &Path) -> io::Result<()> {
    let mut pixels = vec![0u32; 256 * 240];
    game.render(&mut pixels);
    let mut file = fs::File::create(path)?;
    file.write_all(b"P6\n256 240\n255\n")?;
    for pixel in pixels {
        file.write_all(&[(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8])?;
    }
    Ok(())
}
fn press(game: &mut Game, mask: u32) {
    game.tick(mask);
    game.tick(0);
}
fn main() -> io::Result<()> {
    let output = env::args()
        .nth(1)
        .unwrap_or_else(|| "target/screenshots".to_string());
    let dir = Path::new(&output);
    fs::create_dir_all(dir)?;
    let mut game = Game::with_seed(0x8889);
    write_frame(&game, &dir.join("title.ppm"))?;
    press(&mut game, START);
    write_frame(&game, &dir.join("game_type.ppm"))?;
    press(&mut game, RIGHT);
    press(&mut game, START);
    press(&mut game, A);
    press(&mut game, DOWN);
    write_frame(&game, &dir.join("level_b.ppm"))?;
    press(&mut game, START);
    for _ in 0..60 {
        game.tick(0);
    }
    write_frame(&game, &dir.join("playing_b.ppm"))?;
    press(&mut game, START);
    write_frame(&game, &dir.join("paused.ppm"))?;
    game.scene = Scene::Won;
    game.start_level = 9;
    write_frame(&game, &dir.join("ending_b9.ppm"))?;
    game.scene = Scene::NameEntry;
    write_frame(&game, &dir.join("name_entry.ppm"))?;
    game.scene = Scene::GameOver;
    for _ in 0..144 {
        game.tick(0);
    }
    write_frame(&game, &dir.join("game_over.ppm"))?;
    Ok(())
}
