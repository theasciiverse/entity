use crate::art::{self, Art, FitMode, Geometry};
use crate::level::Level;
use crate::ninja::{Controls, Event, Ninja, RADIUS};
use crate::objects::{Objects, WorldEvent};

pub struct Particle {
    pub x: f32,
    pub y: f32,
    vx: f32,
    vy: f32,
    life: u32,
}

/// Gold pickup radius in world units (the ninja radius is added).
const GOLD_RADIUS: f32 = 6.0;
/// The original's episode clock, in its 40 Hz ticks: 3600 (90 s), 80 back per gold.
const MAX_TIME: f32 = 3600.0;
const GOLD_BONUS: f32 = 80.0;
/// One 60 Hz frame is two thirds of an original tick.
const TICK40: f32 = 2.0 / 3.0;

pub struct Game {
    pub levels: Vec<Level>,
    pub cur: usize,
    pub art: Art,
    pub fit_mode: FitMode,
    /// Drawn size of the ninja relative to its hitbox (the hitbox never changes).
    pub ninja_scale: f32,
    pub ninja: Ninja,
    pub particles: Vec<Particle>,
    /// One flag per level object: gold already picked up this attempt.
    pub collected: Vec<bool>,
    /// Mines, launch pads, doors, one-way platforms and bounce blocks of the current attempt.
    pub world: Objects,
    /// One flag per exit: its switch has been touched.
    pub switches: Vec<bool>,
    /// The ninja reached an open door.
    pub complete: bool,
    pub deaths: u32,
    /// Score of the last finished episode: the time left, in seconds.
    pub score: u32,
    /// Clock in the original's 40 Hz ticks, shared by the five levels of an episode.
    pub time_left: f32,
    /// The clock when the current level began; a death restores it.
    start_time: f32,
    /// The clock starts on the first key press of an attempt.
    started: bool,
    pub game_over: bool,
    /// Set when the level just completed was the last of its episode.
    pub episode_done: bool,
    pub tick: u64,
    rng: u64,
}

impl Game {
    pub fn new(levels: Vec<Level>, start: usize) -> Game {
        let cur = start.min(levels.len() - 1);
        let ninja = Ninja::new(levels[cur].spawn.0, levels[cur].spawn.1);
        let geo = art::fit(&levels[cur], art::MIN_BOARD.0, art::MIN_BOARD.1, 2.0).expect("minimum board");
        let art = art::build(&levels[cur], geo);
        let mut g = Game {
            levels,
            cur,
            art,
            fit_mode: FitMode::from_env(),
            ninja_scale: std::env::var("ENTITY_NINJA_SCALE").ok().and_then(|v| v.parse().ok()).unwrap_or(1.0f32).clamp(1.0, 2.5),
            ninja,
            particles: Vec::new(),
            collected: Vec::new(),
            world: Objects::none(),
            switches: Vec::new(),
            complete: false,
            deaths: 0,
            score: 0,
            time_left: MAX_TIME,
            start_time: MAX_TIME,
            started: false,
            game_over: false,
            episode_done: false,
            tick: 0,
            rng: 0x9E37_79B9_7F4A_7C15,
        };
        g.load(cur);
        g
    }

    /// Step the drawn ninja size by a quarter, between 1.0 and 2.5 times the hitbox.
    pub fn resize_ninja(&mut self, steps: i32) {
        self.ninja_scale = (self.ninja_scale + 0.25 * steps as f32).clamp(1.0, 2.5);
        log::debug!("ninja drawn at {}x", self.ninja_scale);
    }

    pub fn toggle_fit(&mut self) {
        self.fit_mode = if self.fit_mode == FitMode::Fill { FitMode::Square } else { FitMode::Fill };
        log::debug!("fit mode {:?}", self.fit_mode);
    }

    pub fn level(&self) -> &Level {
        &self.levels[self.cur]
    }

    /// Rebuild the glyph art when the board size changes.
    pub fn set_geometry(&mut self, geo: Geometry) {
        if geo != self.art.geo {
            self.art = art::build(&self.levels[self.cur], geo);
            log::debug!("board geometry {}x{} inner cells", geo.n, geo.m);
        }
    }

    /// Load a level with a fresh clock (navigation, a new episode, a retry after game over).
    pub fn load(&mut self, idx: usize) {
        self.time_left = MAX_TIME;
        self.start_time = MAX_TIME;
        self.load_keeping_clock(idx);
    }

    fn load_keeping_clock(&mut self, idx: usize) {
        self.cur = idx;
        self.art = art::build(&self.levels[idx], self.art.geo);
        self.particles.clear();
        self.deaths = 0;
        self.game_over = false;
        self.episode_done = false;
        self.respawn();
        log::debug!("load level {} {}", idx, self.level().label);
    }

    pub fn step_level(&mut self, delta: i32) {
        let n = self.levels.len() as i32;
        let next = (self.cur as i32 + delta).rem_euclid(n) as usize;
        if self.complete && delta == 1 {
            self.advance(next);
        } else {
            self.load(next);
        }
    }

    /// After completing a level: the clock carries into the next level of the episode, and a
    /// new episode starts with a fresh one.
    fn advance(&mut self, next: usize) {
        let last_of_episode = self.level().episode.is_some_and(|(_, slot)| slot == 4);
        if last_of_episode {
            self.load(next);
        } else {
            self.start_time = self.time_left;
            self.load_keeping_clock(next);
        }
    }

    /// Restart: after game over from the first level of the episode, otherwise this attempt.
    pub fn restart(&mut self) {
        if self.game_over {
            let first = self.level().episode.map_or(self.cur, |(_, slot)| self.cur.saturating_sub(slot as usize));
            self.load(first);
        } else {
            self.respawn();
        }
    }

    /// Clock seconds for the display (None on levels with no clock).
    pub fn seconds_left(&self) -> Option<f32> {
        self.level().episode.map(|_| (self.time_left / 40.0).max(0.0))
    }

    pub fn gold_total(&self) -> usize {
        self.level().objects.iter().filter(|o| o.kind == "gold").count()
    }

    pub fn gold_collected(&self) -> usize {
        self.level().objects.iter().zip(&self.collected).filter(|(o, &c)| c && o.kind == "gold").count()
    }

    fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Restart the attempt: ninja back at spawn and all gold back in place (as in N).
    pub fn respawn(&mut self) {
        self.ninja = Ninja::new(self.level().spawn.0, self.level().spawn.1);
        self.collected = vec![false; self.level().objects.len()];
        self.world = Objects::from_level(self.level());
        self.switches = vec![false; self.level().exits.len()];
        self.complete = false;
        // a retry restores the clock the level began with (as in the original)
        self.time_left = self.start_time;
        self.started = false;
        log::debug!("respawn");
    }

    pub fn update(&mut self, c: &Controls) {
        if self.complete || self.game_over {
            self.step_particles();
            return;
        }
        self.tick += 1;
        if !self.started && (c.dir != 0 || c.jump_pressed || c.jump_held) {
            self.started = true;
        }
        if self.started && self.level().episode.is_some() {
            self.time_left -= TICK40;
            if self.time_left <= 0.0 {
                self.time_left = 0.0;
                self.game_over = true;
                self.burst();
                self.deaths += 1;
                return;
            }
        }
        let events = self.ninja.step_in(&self.levels[self.cur], c, &mut self.world);
        let mut killed = false;
        for w in self.world.update(&self.levels[self.cur], &mut self.ninja, self.tick) {
            log::debug!("world event {:?} at ({:.1},{:.1})", w, self.ninja.x, self.ninja.y);
            killed |= matches!(w, WorldEvent::Mine | WorldEvent::Killed(_));
        }
        if killed {
            self.deaths += 1;
            self.burst();
            self.respawn();
            self.step_particles();
            return;
        }
        for e in events {
            log::debug!("event {:?} at ({:.1},{:.1}) v=({:.2},{:.2})", e, self.ninja.x, self.ninja.y, self.ninja.vx, self.ninja.vy);
            if let Event::Died(_) = e {
                self.deaths += 1;
                self.burst();
                self.respawn();
            }
        }
        self.pick_up_gold();
        self.touch_exits();
        self.step_particles();
        if self.tick % 30 == 0 {
            let n = &self.ninja;
            log::debug!(
                "snapshot tick={} pos=({:.1},{:.1}) v=({:.2},{:.2}) grounded={} wall={} deaths={} gold={}/{} particles={}",
                self.tick,
                n.x,
                n.y,
                n.vx,
                n.vy,
                n.grounded,
                n.wall,
                self.deaths,
                self.gold_collected(),
                self.gold_total(),
                self.particles.len()
            );
        }
    }

    fn touch_exits(&mut self) {
        let was_complete = self.complete;
        let last_of_episode = self.levels[self.cur].episode.is_some_and(|(_, slot)| slot == 4);
        let (nx, ny) = (self.ninja.x, self.ninja.y);
        for (e, on) in self.levels[self.cur].exits.iter().zip(self.switches.iter_mut()) {
            let near = |p: (f32, f32), r: f32| (p.0 - nx).powi(2) + (p.1 - ny).powi(2) <= (RADIUS + r).powi(2);
            if !*on && near(e.switch, 6.0) {
                *on = true;
                log::debug!("switch touched at ({:.0},{:.0})", e.switch.0, e.switch.1);
            }
            if *on && near(e.door, 12.0) {
                self.complete = true;
                log::debug!("level complete after {} ticks", self.tick);
            }
        }
        if self.complete && !was_complete && last_of_episode {
            // the score is the time left at the end of an episode, in seconds
            self.episode_done = true;
            self.score = (self.time_left / 40.0).max(0.0) as u32;
        }
    }

    fn pick_up_gold(&mut self) {
        let (nx, ny) = (self.ninja.x, self.ninja.y);
        let reach = RADIUS + GOLD_RADIUS;
        let time = &mut self.time_left;
        for (o, got) in self.levels[self.cur].objects.iter().zip(self.collected.iter_mut()) {
            if o.kind == "gold" && !*got && (o.x - nx).powi(2) + (o.y - ny).powi(2) <= reach * reach {
                *got = true;
                *time += GOLD_BONUS;
                log::debug!("gold picked up at ({:.0},{:.0})", o.x, o.y);
            }
        }
    }

    fn burst(&mut self) {
        let (x, y, bvx, bvy) = (self.ninja.x, self.ninja.y, self.ninja.vx * 0.5, self.ninja.vy * 0.5);
        for _ in 0..48 {
            let a = self.rand() * std::f32::consts::TAU;
            let s = 0.5 + self.rand() * 3.0;
            let life = 90 + (self.rand() * 60.0) as u32;
            self.particles.push(Particle { x, y, vx: bvx + a.cos() * s, vy: bvy + a.sin() * s, life });
        }
    }

    fn step_particles(&mut self) {
        let level = &self.levels[self.cur];
        self.particles.retain_mut(|p| {
            p.vy += 0.05;
            let (nx, ny) = (p.x + p.vx, p.y + p.vy);
            if level.point_solid(nx, p.y) {
                p.vx = -p.vx * 0.5;
            } else {
                p.x = nx;
            }
            if level.point_solid(p.x, ny) {
                p.vy = -p.vy * 0.5;
                p.vx *= 0.8;
            } else {
                p.y = ny;
            }
            p.life -= 1;
            p.life > 0
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::n_levels;

    /// Every real level, with all of its objects, run through the whole game loop for several
    /// seconds with the ninja idle and then with it running about: nothing may panic, and every
    /// position must stay finite.
    #[test]
    fn all_150_levels_run_with_their_objects() {
        let levels = n_levels();
        let n = levels.len();
        let mut g = Game::new(levels, 0);
        for idx in 0..n {
            g.load(idx);
            for t in 0..900u32 {
                let dir = if t < 300 { 0 } else if (t / 60) % 2 == 0 { 1 } else { -1 };
                g.update(&Controls { dir, jump_held: t % 90 < 30, jump_pressed: t % 90 == 0 });
                assert!(g.ninja.x.is_finite() && g.ninja.y.is_finite(), "{}: ninja position {:?}", g.level().label, (g.ninja.x, g.ninja.y));
                for d in &g.world.drones {
                    assert!(d.pos.0.is_finite() && d.pos.1.is_finite(), "{}: drone {:?}", g.level().label, d.pos);
                }
                for th in &g.world.thwomps {
                    assert!(th.pos.0.is_finite() && th.pos.1.is_finite(), "{}: thwomp {:?}", g.level().label, th.pos);
                }
                for l in &g.world.launchers {
                    assert!(l.rocket.0.is_finite() && l.rocket.1.is_finite(), "{}: rocket {:?}", g.level().label, l.rocket);
                }
            }
        }
    }

    #[test]
    fn mines_kill_and_reset_the_attempt() {
        let mut lv = Level::parse(include_str!("../levels/001.txt"));
        lv.add_test_objects("mine 108 518");
        let mut g = Game::new(vec![lv], 0);
        let spawn = (g.level().spawn.0, g.level().spawn.1);
        g.ninja.x = 108.0;
        g.ninja.y = 514.0;
        g.update(&Controls::default());
        assert_eq!(g.deaths, 1, "the mine killed the ninja");
        assert_eq!((g.ninja.x, g.ninja.y), spawn, "and the attempt restarted");
        assert!(!g.world.mines[0].exploded, "the level reset, so the mine is live again");
    }

    /// A small episode of three identical levels, each with its own switch and door.
    fn episode_of(n: usize) -> Game {
        let levels: Vec<Level> = (0..n)
            .map(|k| {
                let mut l = Level::parse(include_str!("../levels/001.txt"));
                l.episode = Some((0, k as u32));
                l
            })
            .collect();
        Game::new(levels, 0)
    }

    #[test]
    fn the_clock_waits_for_the_first_key_then_runs_at_40_ticks_a_second() {
        let mut g = episode_of(2);
        for _ in 0..120 {
            g.update(&Controls::default());
        }
        assert_eq!(g.seconds_left(), Some(90.0), "no key yet, no clock");
        for _ in 0..120 {
            g.update(&Controls { dir: 1, ..Default::default() });
        }
        // 120 frames at 60 Hz = 2 s, so the 90 s clock reads about 88
        let t = g.seconds_left().unwrap();
        assert!((t - 88.0).abs() < 0.1, "clock reads {t}");
    }

    #[test]
    fn the_practice_course_has_no_clock() {
        let mut g = Game::new(vec![Level::parse(include_str!("../levels/001.txt"))], 0);
        for _ in 0..200 {
            g.update(&Controls { dir: 1, ..Default::default() });
        }
        assert_eq!(g.seconds_left(), None);
        assert!(!g.game_over);
    }

    #[test]
    fn gold_gives_two_seconds_and_a_death_restores_the_level_start_clock() {
        let mut lv = Level::parse(include_str!("../levels/001.txt"));
        lv.episode = Some((0, 0));
        lv.add_test_objects("gold 100 500\nmine 300 518");
        let mut g = Game::new(vec![lv], 0);
        g.ninja.x = 100.0;
        g.ninja.y = 500.0;
        g.update(&Controls { dir: 1, ..Default::default() });
        let t = g.seconds_left().unwrap();
        assert!(t > 91.9 && t < 92.1, "one gold adds 2 s: {t}");
        assert_eq!(g.gold_collected(), 1);
        // run into the mine: the attempt restarts with the clock the level began with
        g.ninja.x = 300.0;
        g.ninja.y = 514.0;
        g.update(&Controls { dir: 1, ..Default::default() });
        assert_eq!(g.deaths, 1);
        assert_eq!(g.seconds_left(), Some(90.0), "the clock is back to the level start");
        assert_eq!(g.gold_collected(), 0, "and the gold is back too");
    }

    #[test]
    fn finishing_a_level_carries_the_clock_and_a_new_episode_starts_fresh() {
        let mut g = episode_of(6);
        // spend 5 seconds in the first level, then finish it
        for _ in 0..300 {
            g.update(&Controls { dir: 1, ..Default::default() });
        }
        let spent = g.seconds_left().unwrap();
        assert!(spent < 87.0);
        g.complete = true;
        g.step_level(1);
        assert_eq!(g.cur, 1);
        let carried = g.seconds_left().unwrap();
        assert!((carried - spent).abs() < 0.01, "the clock carries: {spent} -> {carried}");
        // dying in level 2 restores the carried value, not 90
        g.ninja.y += 5000.0;
        g.respawn();
        assert!((g.seconds_left().unwrap() - spent).abs() < 0.01);
        // the fifth level (slot 4) completes the episode with a score
        g.load(4);
        for _ in 0..60 {
            g.update(&Controls { dir: 1, ..Default::default() });
        }
        let left = g.time_left;
        g.complete = true;
        g.episode_done = true;
        g.score = (left / 40.0) as u32;
        g.step_level(1);
        assert_eq!(g.cur, 5);
        assert_eq!(g.seconds_left(), Some(90.0), "a new episode starts with a fresh 90 s");
    }

    #[test]
    fn running_out_of_time_is_game_over_and_r_retries_the_episode_from_its_first_level() {
        let mut g = episode_of(5);
        g.load(3);
        g.time_left = 40.0; // one second
        g.start_time = 40.0;
        for _ in 0..200 {
            g.update(&Controls { dir: 1, ..Default::default() });
        }
        assert!(g.game_over, "the clock hit zero");
        g.restart();
        assert!(!g.game_over);
        assert_eq!(g.cur, 0, "back to the episode's first level");
        assert_eq!(g.seconds_left(), Some(90.0));
    }
}
