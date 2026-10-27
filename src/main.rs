mod art;
mod bot;
mod demo;
mod drones;
mod editor;
mod game;
mod hazards;
mod input;
mod level;
mod ninja;
mod objects;
mod pad;
mod render;
mod shapes;
mod sprite;

use game::Game;
use input::Input;
use level::Level;
use ratatui::crossterm::event::{self, Event, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::supports_keyboard_enhancement;
use ratatui::DefaultTerminal;
use std::io;
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_micros(16_667);

fn main() -> io::Result<()> {
    // Session logging: RUST_LOG=debug entity 2>entity.log
    env_logger::init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = args.first().cloned();
    match arg.as_deref() {
        Some("--glyphs") => {
            render::print_glyph_page(&all_levels());
            return Ok(());
        }
        // --prove <file>...: find each level's route with the route bot and store it in the file
        Some("--prove") => {
            let mut ok = true;
            for path in &args[1..] {
                if let Err(e) = bot::prove(path) {
                    eprintln!("{e}");
                    ok = false;
                }
            }
            std::process::exit(if ok { 0 } else { 1 });
        }
        // --render <level or file> [cols rows]: print the board as plain text
        Some("--render") => {
            let levels = all_levels();
            let level = match args.get(1) {
                Some(a) if std::path::Path::new(a).is_file() => Level::parse(&std::fs::read_to_string(a)?),
                Some(a) => levels[find_level(&levels, a)].clone_level(),
                None => levels[0].clone_level(),
            };
            let size = |i: usize, d: u16| args.get(i).and_then(|v| v.parse().ok()).unwrap_or(d);
            render::print_board(&level, size(2, 100), size(3, 30));
            return Ok(());
        }
        // --edit [file]: the level editor (a new level when the file does not exist yet)
        // --edit [file [level]]: the level editor; a file that does not exist yet starts as
        // an empty level, or as a copy of the named level
        Some("--edit") => {
            let path = args.get(1).cloned().unwrap_or_else(|| "levels/new.txt".into());
            let from = args.get(2).map(|a| {
                let levels = all_levels();
                levels[find_level(&levels, a)].clone_level()
            });
            return session(false, |terminal, kitty| editor::run(terminal, &path, from, kitty));
        }
        _ => {}
    }
    // levels: our campaign (episodes of five), the practice course, then N's 150 levels.
    // Optional arg: start index, name fragment or "episode:slot" (e.g. `entity 3:2`).
    let mut levels = all_levels();
    // --demo [level]: the game plays a level by itself, over and over (levels with a
    // route; the practice course by default). A number counts in the order the demo
    // always used, practice then N's levels, so `--demo 2` stays cloud city.
    let demo = arg.as_deref() == Some("--demo");
    if demo {
        let pick = match args.get(1) {
            Some(a) if a.parse::<usize>().is_ok() => {
                let mut legacy = vec![practice()];
                legacy.extend(level::n_levels());
                let i = a.parse::<usize>().unwrap().min(legacy.len() - 1);
                legacy.swap_remove(i)
            }
            Some(a) => levels.swap_remove(find_level(&levels, a)),
            None => practice(),
        };
        let pick = if pick.route.is_some() || demo::route_for(&pick.label).is_some() { pick } else { practice() };
        levels = vec![pick];
    }
    let start = if demo { 0 } else { arg.map_or(0, |a| find_level(&levels, &a)) };
    session(demo, |terminal, kitty| run(terminal, levels, start, kitty, demo, false))
}

/// Runs `f` in the alternate screen, with the kitty keyboard protocol on when the terminal
/// speaks it (the demo skips the probe: about 2 s on a terminal that never answers, which
/// would open a recording with a blank screen).
fn session(demo: bool, f: impl FnOnce(&mut DefaultTerminal, bool) -> io::Result<()>) -> io::Result<()> {
    let mut terminal = ratatui::init();
    let kitty = !demo && supports_keyboard_enhancement().unwrap_or(false);
    if kitty {
        let flags = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
            | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
            | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES;
        execute!(io::stdout(), PushKeyboardEnhancementFlags(flags))?;
    }
    log::info!("start kitty_keyboard={kitty}");
    let result = f(&mut terminal, kitty);
    if kitty {
        let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
    }
    ratatui::restore();
    result
}

/// The game loop. `test_play` is the editor's test run: Esc goes straight back to the editor.
fn run(terminal: &mut DefaultTerminal, levels: Vec<Level>, start: usize, kitty: bool, demo: bool, test_play: bool) -> io::Result<()> {
    let first = levels[0].clone_level();
    let mut game = Game::new(levels, start);
    // the demo needs the keyboard only for Esc and Ctrl-C, so no input-mode note
    let mut input = if demo { Input::with_mode(input::Mode::Exact) } else { Input::new(kitty) };
    let route = first.route.clone().unwrap_or_else(|| demo::route_for(&first.label).unwrap_or(demo::PRACTICE).to_string());
    let mut player = demo.then(|| demo::Demo::new(&route));
    let mut pad = pad::Pad::open();
    let mut finished_ticks = 0u32;
    let mut next = Instant::now() + TICK;
    while !input.quit {
        let size = terminal.size()?;
        if let Some(geo) = art::fit_with(game.fit_mode, game.level(), size.width, size.height, cell_aspect()) {
            game.set_geometry(geo);
        }
        terminal.draw(|f| render::draw(f, &game, &input))?;
        if event::poll(next.saturating_duration_since(Instant::now()))? {
            loop {
                if let Event::Key(k) = event::read()? {
                    log::debug!("key {:?}", k);
                    input.on_key(k, Instant::now());
                }
                if !event::poll(Duration::ZERO)? {
                    break;
                }
            }
            if test_play && input.confirm_quit {
                log::debug!("test play ends");
                return Ok(());
            }
        }
        if player.is_none() {
            pad.poll(&mut input);
        }
        let now = Instant::now();
        let mut steps = 0;
        while now >= next && steps < 5 {
            if std::mem::take(&mut input.restart) {
                game.restart();
            }
            if input.ninja_steps != 0 {
                game.resize_ninja(std::mem::take(&mut input.ninja_steps));
            }
            if std::mem::take(&mut input.fit_toggle) {
                game.toggle_fit();
            }
            if std::mem::take(&mut input.pad_start) {
                if game.complete {
                    game.step_level(1);
                } else if game.game_over {
                    game.restart();
                }
            }
            if input.level_step != 0 {
                game.step_level(std::mem::take(&mut input.level_step));
            }
            let controls = match player.as_mut() {
                Some(p) => p.controls(!game.ninja.grounded),
                None => input.controls(now, !game.ninja.grounded),
            };
            if !input.confirm_quit {
                game.update(&controls);
            }
            if player.is_some() && game.complete {
                // the demo lets the "Level complete" sink in, then plays again
                finished_ticks += 1;
                if finished_ticks == 180 {
                    game = Game::new(vec![first.clone_level()], 0);
                    player = Some(demo::Demo::new(&route));
                    finished_ticks = 0;
                }
            }
            next += TICK;
            steps += 1;
        }
        if steps == 5 {
            next = now + TICK;
        }
    }
    Ok(())
}

fn practice() -> Level {
    Level::parse(include_str!("../levels/001.txt"))
}

/// Everything the game can play: the campaign, the practice course, then N's levels.
fn all_levels() -> Vec<Level> {
    let mut levels = level::campaign();
    levels.push(practice());
    levels.extend(level::n_levels());
    levels
}

pub fn find_level(levels: &[Level], arg: &str) -> usize {
    if let Ok(i) = arg.parse::<usize>() {
        return i;
    }
    let want = arg.to_lowercase().replace([' ', '_'], "");
    levels
        .iter()
        .position(|l| l.label.starts_with(&format!("{arg} ")) || l.label.to_lowercase().replace(' ', "").contains(&want))
        .unwrap_or(0)
}

/// Cell height / width in pixels as reported by the terminal; 2.0 when it does not say.
pub fn cell_aspect() -> f32 {
    match ratatui::crossterm::terminal::window_size() {
        Ok(w) if w.width > 0 && w.height > 0 && w.columns > 0 && w.rows > 0 => {
            (w.height as f32 / w.rows as f32) / (w.width as f32 / w.columns as f32)
        }
        _ => 2.0,
    }
}
