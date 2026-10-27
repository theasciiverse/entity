mod art;
mod demo;
mod drones;
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
    // levels: the practice slab, then all 150 N levels. Optional arg: start index,
    // N level name fragment, or "episode:slot" (e.g. `entity cloudcity`, `entity 3:2`).
    let mut levels = vec![Level::parse(include_str!("../levels/001.txt"))];
    levels.extend(level::n_levels());
    let arg = std::env::args().nth(1);
    if arg.as_deref() == Some("--glyphs") {
        render::print_glyph_page(&levels);
        return Ok(());
    }
    // --demo [level]: the game plays a level by itself, over and over (levels with a
    // route in demo.rs; the practice course by default)
    let demo = arg.as_deref() == Some("--demo");
    if demo {
        let pick = std::env::args().nth(2).map_or(0, |a| find_level(&levels, &a));
        let pick = if demo::route_for(&levels[pick].label).is_some() { pick } else { 0 };
        levels = vec![levels.swap_remove(pick)];
    }
    let start = if demo { 0 } else { arg.map_or(0, |a| find_level(&levels, &a)) };
    let mut terminal = ratatui::init();
    // the demo plays itself, so it skips the protocol probe (about 2 s on a terminal that
    // never answers, which would open a recording with a blank screen)
    let kitty = !demo && supports_keyboard_enhancement().unwrap_or(false);
    if kitty {
        let flags = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
            | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
            | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES;
        execute!(io::stdout(), PushKeyboardEnhancementFlags(flags))?;
    }
    log::info!("start kitty_keyboard={kitty}");
    let result = run(&mut terminal, levels, start, kitty, demo);
    if kitty {
        let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
    }
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal, levels: Vec<Level>, start: usize, kitty: bool, demo: bool) -> io::Result<()> {
    let first = levels[0].clone_level();
    let mut game = Game::new(levels, start);
    // the demo needs the keyboard only for Esc and Ctrl-C, so no input-mode note
    let mut input = if demo { Input::with_mode(input::Mode::Exact) } else { Input::new(kitty) };
    let route = demo::route_for(&first.label).unwrap_or(demo::PRACTICE);
    let mut player = demo.then(|| demo::Demo::new(route));
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
                    player = Some(demo::Demo::new(route));
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

fn find_level(levels: &[Level], arg: &str) -> usize {
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
fn cell_aspect() -> f32 {
    match ratatui::crossterm::terminal::window_size() {
        Ok(w) if w.width > 0 && w.height > 0 && w.columns > 0 && w.rows > 0 => {
            (w.height as f32 / w.rows as f32) / (w.width as f32 / w.columns as f32)
        }
        _ => 2.0,
    }
}
