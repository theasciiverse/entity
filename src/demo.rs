//! `entity --demo [level]`: the game plays a level by itself (the practice course, or
//! a level that has a route below), for the catalog preview recording and as an
//! attract mode. It replays the route the machine
//! playtest found (see "Playtesting by machine" in CLAUDE.md) as key presses and
//! releases through the real `Input` in exact mode, so what is shown is exactly what
//! the tests verify finishes the course. The route is one letter pair per 100 ms
//! (6 ticks): L, _ or R for the direction held, j or . for jump held.

use crate::input::{Input, Mode};
use crate::ninja::Controls;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::time::{Duration, Instant};

/// Found by `print_the_winning_routes` (an ignored test in ninja.rs); regenerate it there
/// when the course or the physics changes on purpose.
pub const PRACTICE: &str = "R.R.R.R.R._jRjRj_jRjRjRjR.R.R.R.RjRjRjRjRjRjRjRjR.R.R.R.RjRjRjRjRj_.RjRjRjRj_.RjRjRjRj_.RjRjRjRj_.RjRjRjRj_.RjRjRjR._jRjRjRjRjRjRjRjR.R.R._._.L.RjRjR.R.R.R.R.R.R.R.";

/// 0:1 cloud city, found by the ignored `print_a_route` test in ninja.rs from waypoints a
/// person picked (fetch the key, through the locked door to the switch, down to the
/// floor and right to the door):
/// ENTITY_ROUTE_LEVEL=cloudcity ENTITY_ROUTE_WAYPOINTS="190,460;288,372;60,348;48,228;240,108;324,158;204,204;324,158;240,108;60,348|180,564;732,564"
pub const CLOUD_CITY: &str = "R.RjRjRjRjRjRjRjRjR.RjRjRjRjR.R.R.L.L.R.LjLjLjRjRjRjRjR.RjRjRjRjR.R.R.L.L.L.L.L.L.L.LjLjL.L.L.L.L.L.L.L.L.L.LjL.L.L.L.LjLjLjLjLjL.LjR.RjRjRjRj_jRjRjRjR.R.R.R.R.RjRjRjRjRjRjRjR.R.R.R.R.R.R._jL._.L.L.L.L.L.L.L._._.R.L.L.L.L.L.L.L.L.L.L.R.R.RjRjLj_.RjRjRjRjRjRjR.RjRjRjR.R.R.R._.R._.LjLjLjLjLjLjLjLjL.L.L.L.R.L.L.L.L.L.L.L.L.L.R.R.R.R.R.R.R.R.L.L.L.L.L.L.L.L.L._.R.L._.LjR.LjL.RjR.R.R.R.R.R.R.L.L.L.L.L.L.L.R.R.R.R.R.R.R.R.R.R.R.R.R.R.R.R.R.R.R.R.R.R.R.R.R.RjR.RjR.RjR.RjR.RjR._.LjLjLjLjLjLjL.L.L.";

/// Levels with a demo route, by label.
pub const ROUTES: [(&str, &str); 2] = [("practice", PRACTICE), ("0:1 cloud city", CLOUD_CITY)];

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;
    use crate::level::{n_levels, Level};

    fn plays_through(level: Level, route: &str) -> Game {
        let mut game = Game::new(vec![level], 0);
        let mut demo = Demo::new(route);
        for _ in 0..decode(route).len() * TICKS_PER_STEP {
            let c = demo.controls(!game.ninja.grounded);
            game.update(&c);
            if game.complete {
                return game;
            }
        }
        panic!("the route did not finish {} ({} deaths)", game.level().label, game.deaths);
    }

    #[test]
    fn the_demo_finishes_the_practice_course() {
        let g = plays_through(Level::parse(include_str!("../levels/001.txt")), PRACTICE);
        assert_eq!(g.deaths, 0);
    }

    #[test]
    fn the_demo_finishes_cloud_city_without_dying() {
        let level = n_levels().into_iter().find(|l| l.label == "0:1 cloud city").unwrap();
        let g = plays_through(level, CLOUD_CITY);
        assert_eq!(g.deaths, 0);
        assert!(!g.game_over);
        println!("cloud city: gold {}/{}, {:.1} s left", g.gold_collected(), g.gold_total(), g.seconds_left().unwrap_or(0.0));
    }
}
