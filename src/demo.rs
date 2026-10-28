//! `entity --demo [level]`: the game plays a level by itself (the practice course, or
//! a level that has a route below), for the catalog preview recording and as an
//! attract mode. It replays the route the machine
//! playtest found (see "Playtesting by machine" in CLAUDE.md) as key presses and
//! releases through the real `Input` in exact mode, so what is shown is exactly what
//! the tests verify finishes the course. The route is one letter pair per 100 ms
//! (6 ticks): L, _ or R for the direction held, j or . for jump held.

use crate::game::Game;
use crate::input::{Input, Mode};
use crate::level::Level;
use crate::ninja::Controls;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::time::{Duration, Instant};

/// Found by `print_the_winning_routes` (an ignored test in ninja.rs); regenerate it there
/// when the course or the physics changes on purpose.
pub const PRACTICE: &str = "R.R.R.R.R._jRjRj_jRjRjRjR.R.R.R.RjRjRjRjRjRjRjRjR.R.R.R.RjRjRjRjRj_.RjRjRjRj_.RjRjRjRj_.RjRjRjRj_.RjRjRjRj_.RjRjRjR._jRjRjRjRjRjRjRjR.R.R._._.L.RjRjR.R.R.R.R.R.R.R.";

/// Levels with a demo route, by label.
pub const ROUTES: [(&str, &str); 1] = [("practice", PRACTICE)];

pub fn route_for(label: &str) -> Option<&'static str> {
    ROUTES.iter().find(|(l, _)| *l == label).map(|(_, r)| *r)
}

pub fn decode(s: &str) -> Vec<(i8, bool)> {
    s.as_bytes().chunks(2).map(|c| (match c[0] { b'L' => -1, b'R' => 1, _ => 0 }, c[1] == b'j')).collect()
}

const TICKS_PER_STEP: usize = 6;

pub struct Demo {
    route: Vec<(i8, bool)>,
    tick: usize,
    input: Input,
    dir: i8,
    jump: bool,
    clock: Instant,
}

impl Demo {
    pub fn new(route: &str) -> Demo {
        Demo { route: decode(route), tick: 0, input: Input::with_mode(Mode::Exact), dir: 0, jump: false, clock: Instant::now() }
    }

    fn key(&mut self, code: KeyCode, kind: KeyEventKind) {
        self.input.on_key(KeyEvent::new_with_kind(code, KeyModifiers::NONE, kind), self.clock);
    }

    /// The controls for the next tick; no keys once the route is over.
    pub fn controls(&mut self, airborne: bool) -> Controls {
        self.clock += Duration::from_micros(16_667);
        let (dir, jump) = self.route.get(self.tick / TICKS_PER_STEP).copied().unwrap_or((0, false));
        self.tick += 1;
        if dir != self.dir {
            if self.dir != 0 {
                self.key(if self.dir < 0 { KeyCode::Left } else { KeyCode::Right }, KeyEventKind::Release);
            }
            if dir != 0 {
                self.key(if dir < 0 { KeyCode::Left } else { KeyCode::Right }, KeyEventKind::Press);
            }
            self.dir = dir;
        }
        if jump != self.jump {
            self.key(KeyCode::Char(' '), if jump { KeyEventKind::Press } else { KeyEventKind::Release });
            self.jump = jump;
        }
        self.input.controls(self.clock, airborne)
    }
}

/// Plays a route through the real `Game` the way `--demo` does, until the level is complete
/// or the route runs out; the game is returned to be inspected (complete, deaths, gold, tick).
pub fn replay(level: &Level, route: &str) -> Game {
    let mut game = Game::new(vec![level.clone_level()], 0);
    let mut demo = Demo::new(route);
    for _ in 0..decode(route).len() * TICKS_PER_STEP {
        let c = demo.controls(!game.ninja.grounded);
        game.update(&c);
        if game.complete {
            break;
        }
    }
    game
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plays_through(level: Level, route: &str) -> Game {
        let game = replay(&level, route);
        assert!(game.complete, "the route did not finish {} ({} deaths)", game.level().label, game.deaths);
        game
    }

    #[test]
    fn the_demo_finishes_the_practice_course() {
        let g = plays_through(Level::parse(include_str!("../levels/001.txt")), PRACTICE);
        assert_eq!(g.deaths, 0);
    }
}
