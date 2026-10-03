//! Drones, ported from the original N v1.2 bytecode (see CLAUDE.md, "Original N physics").
//! They travel the tile-cell grid along open edges, decide turns from a fixed priority list,
//! and carry one of three weapons: zap (touch), laser (a fixed beam after a charge) or
//! chaingun (a burst of hitscan bullets). The original ticks at 40 Hz; speeds and timers
//! are converted to Entity's 60 Hz tick.

use crate::level::{Level, TILE};
use crate::ninja::{Ninja, RADIUS};
use crate::objects::Door;

pub const DRONE_R: f32 = 9.0;
/// Base speed 12 * 0.0714 = 0.857 px per 40 Hz tick, as px per 60 Hz tick.
const BASE_SPEED: f32 = 0.857_14 * 2.0 / 3.0;
/// Convert a count of 40 Hz ticks to 60 Hz frames.
fn frames(ticks40: u32) -> u32 {
    (ticks40 as f32 * 1.5).round() as u32
}

/// R, D, L, U (screen coordinates, so +1 is a clockwise turn).
pub const DIRS: [(i32, i32); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Weapon {
    Zap,
    Laser,
    Chaingun,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cause {
    #[allow(dead_code)]
    Mine,
    Zap,
    Laser,
    Bullet,
    Turret,
    Guard,
    Thwomp,
    Rocket,
}

// movement types: 0 surface cw, 1 surface ccw, 2 wander cw, 3 wander ccw, 4 alternating, 5 random
const CHUCHU_CW: [usize; 4] = [0, 1, 3, 2];
const CHUCHU_CCW: [usize; 4] = [0, 3, 1, 2];
const SURFACE_CW: [usize; 4] = [1, 0, 3, 2];
const SURFACE_CCW: [usize; 4] = [3, 0, 1, 2];

#[derive(Clone, Debug)]
enum State {
    Moving,
    /// Charging (laser: warning beam shown; chaingun: eye turns to the target).
    Pre(u32),
    /// Laser: beam live for this many frames. Chaingun: bullets fired so far and a timer.
    Firing(u32),
    Post(u32),
}

#[derive(Clone, Debug)]
pub struct Drone {
    pub pos: (f32, f32),
    pub weapon: Weapon,
    pub chaser: bool,
    mv: u8,
    goal: (f32, f32),
    dir: usize,
    speed: f32,
    pub chasing: bool,
    ai_counter: u32,
    ai_counter2: u32,
    /// After a chase, wall-followers turn this way (None for wanderers).
    surface_future: Option<usize>,
    state: State,
    /// Laser beam: from the drone to where it stops.
    pub beam: Option<((f32, f32), (f32, f32))>,
    // chaingun burst
    aim: (f32, f32),
    aim_perp: (f32, f32),
    shots: u32,
    max_shots: u32,
    spread: f32,
    shot_timer: u32,
    /// The most recent bullet's end point, for drawing.
    pub tracer: Option<((f32, f32), (f32, f32), u32)>,
}

fn cell_of(p: (f32, f32)) -> (i32, i32) {
    ((p.0 / TILE).floor() as i32, (p.1 / TILE).floor() as i32)
}

fn center(c: (i32, i32)) -> (f32, f32) {
    ((c.0 as f32 + 0.5) * TILE, (c.1 as f32 + 0.5) * TILE)
}

/// Can a drone pass from cell `c` one step in direction `dir`? Both cells must be empty and
/// no closed door may stand on the edge between them.
pub fn edge_open(level: &Level, doors: &[Door], c: (i32, i32), dir: usize) -> bool {
    let n = (c.0 + DIRS[dir].0, c.1 + DIRS[dir].1);
    if level.code(c.0, c.1) != 0 || level.code(n.0, n.1) != 0 {
        return false;
    }
    !doors.iter().any(|d| !d.open && ((d.edge.0 == c && d.edge.1 == n) || (d.edge.0 == n && d.edge.1 == c)))
}

/// True when no solid tile lies on the straight line a to b.
pub fn line_clear(level: &Level, a: (f32, f32), b: (f32, f32)) -> bool {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = (dx * dx + dy * dy).sqrt();
    let steps = (len / 3.0).ceil().max(1.0) as i32;
    (0..=steps).all(|i| {
        let t = i as f32 / steps as f32;
        !level.point_solid(a.0 + dx * t, a.1 + dy * t)
    })
}

/// Where a ray from `a` along unit direction `d` first meets solid ground (at most 3000 px).
pub fn ray_end(level: &Level, a: (f32, f32), d: (f32, f32)) -> (f32, f32) {
    let mut t = 0.0;
    while t < 3000.0 {
        let p = (a.0 + d.0 * t, a.1 + d.1 * t);
        if level.point_solid(p.0, p.1) {
            return (a.0 + d.0 * (t - 2.0).max(0.0), a.1 + d.1 * (t - 2.0).max(0.0));
        }
        t += 2.0;
    }
    (a.0 + d.0 * 3000.0, a.1 + d.1 * 3000.0)
}

fn dist_to_segment(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (ex, ey) = (b.0 - a.0, b.1 - a.1);
    let l2 = ex * ex + ey * ey;
    let t = if l2 > 0.0 { (((p.0 - a.0) * ex + (p.1 - a.1) * ey) / l2).clamp(0.0, 1.0) } else { 0.0 };
    let c = (a.0 + ex * t, a.1 + ey * t);
    ((p.0 - c.0).powi(2) + (p.1 - c.1).powi(2)).sqrt()
}

impl Drone {
    /// From the original record `[x, y, movement, chaser, weapon, start direction]`.
    pub fn from_params(p: &[f32]) -> Drone {
        let weapon = match p[4] as i32 {
            0 => Weapon::Zap,
            1 => Weapon::Laser,
            _ => Weapon::Chaingun,
        };
        let c = cell_of((p[0], p[1]));
        let pos = center(c);
        let base = BASE_SPEED;
        let speed = match weapon {
            Weapon::Zap => base * 2.0,
            Weapon::Laser => base * 0.5,
            Weapon::Chaingun => base * 0.75,
        };
        Drone {
            pos,
            weapon,
            chaser: p[3] != 0.0 && weapon == Weapon::Zap,
            mv: p[2] as u8,
            goal: pos,
            dir: (p[5] as usize).min(3),
            speed,
            chasing: false,
            ai_counter: 0,
            ai_counter2: 0,
            surface_future: None,
            state: State::Moving,
            beam: None,
            aim: (1.0, 0.0),
            aim_perp: (0.0, 1.0),
            shots: 0,
            max_shots: 6,
            spread: 0.3,
            shot_timer: 0,
            tracer: None,
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn charging(&self) -> bool {
        matches!(self.state, State::Pre(_))
    }

    pub fn firing(&self) -> bool {
        matches!(self.state, State::Firing(_))
    }

    fn list(&self) -> &'static [usize; 4] {
        match self.mv {
            0 => &SURFACE_CW,
            1 => &SURFACE_CCW,
            2 => &CHUCHU_CW,
            _ => &CHUCHU_CCW,
        }
    }

    /// First open direction from the priority list (each entry is a turn relative to now).
    fn new_goal(&mut self, level: &Level, doors: &[Door], list: &[usize; 4]) -> Option<usize> {
        let c = cell_of(self.pos);
        list.iter().map(|rot| (self.dir + rot) % 4).find(|&d| edge_open(level, doors, c, d))
    }

    fn pick_dir(&mut self, level: &Level, doors: &[Door]) -> Option<usize> {
        match self.mv {
            4 => {
                let list = if self.ai_counter2 == 0 { &CHUCHU_CW } else { &CHUCHU_CCW };
                let d = self.new_goal(level, doors, list);
                if d.is_some_and(|d| d != self.dir) {
                    self.ai_counter2 = 1 - self.ai_counter2;
                }
                d
            }
            5 => {
                let list = if self.ai_counter % 2 == 0 { &CHUCHU_CW } else { &CHUCHU_CCW };
                let d = self.new_goal(level, doors, list);
                if d.is_some_and(|d| d != self.dir) {
                    self.ai_counter = if self.ai_counter % 2 == 0 { 1 } else { 0 };
                }
                d
            }
            _ => {
                let list = self.list();
                self.new_goal(level, doors, list)
            }
        }
    }

    fn set_dir(&mut self, level: &Level, doors: &[Door], d: usize) {
        self.dir = d;
        let c = cell_of(self.pos);
        if edge_open(level, doors, c, d) {
            self.goal = center((c.0 + DIRS[d].0, c.1 + DIRS[d].1));
        }
    }

    /// Zap chasers: when the ninja is in the same row or column with a clear corridor, charge
    /// to the far end of it at double speed.
    fn chase(&mut self, level: &Level, doors: &[Door], ninja: &Ninja) -> bool {
        if !self.chaser {
            return false;
        }
        if let Some(d) = self.surface_future.take() {
            // finishing a chase as a wall follower: grab the surface by turning
            self.set_dir(level, doors, d);
            return false;
        }
        let (me, pl) = (cell_of(self.pos), cell_of((ninja.x, ninja.y)));
        let (di, dj) = (pl.0 - me.0, pl.1 - me.1);
        let (dir, t) = if di == 0 {
            if ninja.y < self.pos.1 {
                if self.dir == 1 {
                    return false;
                }
                (3, dj.abs())
            } else {
                if self.dir == 3 {
                    return false;
                }
                (1, dj.abs())
            }
        } else if dj == 0 {
            if ninja.x < self.pos.0 {
                if self.dir == 0 {
                    return false;
                }
                (2, di.abs())
            } else {
                if self.dir == 2 {
                    return false;
                }
                (0, di.abs())
            }
        } else {
            return false;
        };
        // clear corridor up to the ninja, then on to the end of it
        let mut c = me;
        let mut steps = 0;
        for _ in 0..t {
            if !edge_open(level, doors, c, dir) {
                return false;
            }
            c = (c.0 + DIRS[dir].0, c.1 + DIRS[dir].1);
            steps += 1;
        }
        while edge_open(level, doors, c, dir) {
            c = (c.0 + DIRS[dir].0, c.1 + DIRS[dir].1);
            steps += 1;
        }
        if steps == 0 {
            return false;
        }
        self.dir = dir;
        self.goal = center(c);
        if self.mv < 2 {
            self.surface_future = Some((dir + if self.mv == 0 { 3 } else { 1 }) % 4);
        }
        true
    }

    fn step_move(&mut self, level: &Level, doors: &[Door], ninja: &Ninja) {
        self.ai_counter += 1;
        let (dx, dy) = (self.goal.0 - self.pos.0, self.goal.1 - self.pos.1);
        if dx * dx + dy * dy < self.speed * self.speed {
            self.pos = self.goal;
            if self.chase(level, doors, ninja) {
                self.chasing = true;
            } else {
                if let Some(d) = self.pick_dir(level, doors) {
                    self.set_dir(level, doors, d);
                }
                self.chasing = false;
            }
        } else {
            let s = if self.chasing { self.speed * 2.0 } else { self.speed };
            self.pos.0 += DIRS[self.dir].0 as f32 * s;
            self.pos.1 += DIRS[self.dir].1 as f32 * s;
        }
    }

    /// Advance one 60 Hz tick. `time` is a running frame count (used for the chaingun's
    /// pseudo-random burst length and spread, as the original uses its clock).
    pub fn update(&mut self, level: &Level, doors: &[Door], ninja: &Ninja, time: u64) -> Option<Cause> {
        let np = (ninja.x, ninja.y);
        let sees = |me: &Drone| line_clear(level, me.pos, np);
        match self.state.clone() {
            State::Moving => {
                self.step_move(level, doors, ninja);
                if self.weapon == Weapon::Zap {
                    let d = ((self.pos.0 - np.0).powi(2) + (self.pos.1 - np.1).powi(2)).sqrt();
                    if d < DRONE_R + RADIUS {
                        return Some(Cause::Zap);
                    }
                } else if sees(self) {
                    // start charging; the laser fixes its beam now, so it can be dodged
                    let (dx, dy) = (np.0 - self.pos.0, np.1 - self.pos.1);
                    let len = (dx * dx + dy * dy).sqrt().max(1e-3);
                    let unit = (dx / len, dy / len);
                    if self.weapon == Weapon::Laser {
                        self.beam = Some((self.pos, ray_end(level, self.pos, unit)));
                    }
                    self.state = State::Pre(0);
                }
            }
            State::Pre(t) => {
                let delay = if self.weapon == Weapon::Laser { frames(30) } else { frames(35) };
                if t + 1 >= delay {
                    if self.weapon == Weapon::Chaingun {
                        self.begin_burst(ninja, time);
                    }
                    self.shot_timer = 0;
                    self.state = State::Firing(0);
                } else {
                    self.state = State::Pre(t + 1);
                }
            }
            State::Firing(t) => match self.weapon {
                Weapon::Laser => {
                    if let Some((a, b)) = self.beam {
                        if dist_to_segment(np, a, b) < RADIUS {
                            self.finish_firing();
                            return Some(Cause::Laser);
                        }
                    }
                    if t + 1 >= frames(80) {
                        self.finish_firing();
                    } else {
                        self.state = State::Firing(t + 1);
                    }
                }
                _ => {
                    self.shot_timer += 1;
                    if let Some((_, _, left)) = self.tracer.as_mut() {
                        *left = left.saturating_sub(1);
                    }
                    if self.shot_timer >= frames(6) {
                        self.shot_timer = 0;
                        if self.shots > self.max_shots {
                            self.finish_firing();
                            return None;
                        }
                        let i = (self.shots as f32 / self.max_shots as f32 - 0.5) * self.spread;
                        let d = (self.aim.0 + i * self.aim_perp.0, self.aim.1 + i * self.aim_perp.1);
                        let len = (d.0 * d.0 + d.1 * d.1).sqrt().max(1e-3);
                        let d = (d.0 / len, d.1 / len);
                        let end = ray_end(level, self.pos, d);
                        self.tracer = Some((self.pos, end, 6));
                        self.shots += 1;
                        if dist_to_segment(np, self.pos, end) < RADIUS {
                            self.finish_firing();
                            return Some(Cause::Bullet);
                        }
                    }
                }
            },
            State::Post(t) => {
                let delay = if self.weapon == Weapon::Laser { frames(40) } else { frames(60) };
                if t + 1 >= delay {
                    self.state = State::Moving;
                    self.tracer = None;
                } else {
                    self.state = State::Post(t + 1);
                }
            }
        }
        None
    }

    fn begin_burst(&mut self, ninja: &Ninja, time: u64) {
        self.max_shots = 4 + (time / 2 % 5) as u32;
        self.spread = 0.1 + 0.1 * (1.0 + (time / 2 % 3) as f32);
        self.shots = 0;
        let (dx, dy) = (ninja.x - self.pos.0, ninja.y - self.pos.1);
        let len = (dx * dx + dy * dy).sqrt().max(1e-3);
        self.aim = (dx / len, dy / len);
        // the sweep runs across the ninja's motion
        let dp = ninja.vx * -self.aim.1 + ninja.vy * self.aim.0;
        self.aim_perp = if dp < 0.0 { (self.aim.1, -self.aim.0) } else { (-self.aim.1, self.aim.0) };
    }

    fn finish_firing(&mut self) {
        self.beam = None;
        self.state = State::Post(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::Objects;

    /// A closed corridor room: 12 tiles wide, 3 tall inside, frame around.
    fn room() -> Level {
        let rows = ["##############", "#            #", "#            #", "#            #", "##############"];
        Level::parse(&rows.join("\n"))
    }

    fn ninja_at(x: f32, y: f32) -> Ninja {
        Ninja::new(x, y)
    }

    #[test]
    fn edges_open_between_empty_cells_and_closed_by_walls() {
        let l = room();
        assert!(edge_open(&l, &[], (2, 2), 0));
        assert!(!edge_open(&l, &[], (1, 1), 2), "the wall is to the left of the first cell");
        assert!(!edge_open(&l, &[], (1, 3), 1), "the floor is below the bottom row");
    }

    #[test]
    fn a_drone_stays_on_the_grid_and_in_the_room() {
        let l = room();
        let mut d = Drone::from_params(&[3.5 * TILE, 1.5 * TILE, 2.0, 0.0, 0.0, 0.0]);
        let far = ninja_at(3.0 * TILE, 3.5 * TILE);
        for t in 0..3000 {
            d.update(&l, &[], &far, t);
            assert!(d.pos.0 > TILE && d.pos.0 < 13.0 * TILE && d.pos.1 > TILE && d.pos.1 < 4.0 * TILE, "left the room: {:?}", d.pos);
        }
    }

    #[test]
    fn a_wall_follower_circles_the_room_keeping_the_wall_on_one_side() {
        let l = room();
        // clockwise surface follower in a rectangular room visits all four walls
        let mut d = Drone::from_params(&[1.5 * TILE, 1.5 * TILE, 0.0, 0.0, 0.0, 0.0]);
        let far = ninja_at(1000.0, 1000.0);
        let mut seen = [false; 4]; // left, right, top, bottom wall reached
        for t in 0..6000 {
            d.update(&l, &[], &far, t);
            let c = cell_of(d.pos);
            seen[0] |= c.0 == 1;
            seen[1] |= c.0 == 12;
            seen[2] |= c.1 == 1;
            seen[3] |= c.1 == 3;
        }
        assert!(seen.iter().all(|&s| s), "walls reached: {seen:?}");
    }

    #[test]
    fn a_zap_drone_kills_on_touch_and_not_at_a_distance() {
        let l = room();
        let mut d = Drone::from_params(&[3.5 * TILE, 2.5 * TILE, 2.0, 0.0, 0.0, 0.0]);
        let near = ninja_at(d.pos.0 + 12.0, d.pos.1);
        assert_eq!(d.clone().update(&l, &[], &near, 0), Some(Cause::Zap));
        let far = ninja_at(d.pos.0 + 60.0, d.pos.1);
        assert_eq!(d.update(&l, &[], &far, 0), None);
    }

    #[test]
    fn a_zap_chaser_charges_down_a_clear_row_at_double_speed() {
        let l = room();
        let mut d = Drone::from_params(&[2.5 * TILE, 2.5 * TILE, 2.0, 1.0, 0.0, 0.0]);
        let n = ninja_at(10.5 * TILE, 2.5 * TILE);
        let mut chased = false;
        for t in 0..300 {
            d.update(&l, &[], &n, t);
            chased |= d.chasing;
        }
        assert!(chased, "it should notice a ninja standing in its row");
    }

    #[test]
    fn a_laser_drone_charges_then_fires_a_fixed_beam_that_can_be_dodged() {
        let l = room();
        let mut d = Drone::from_params(&[2.5 * TILE, 2.5 * TILE, 2.0, 0.0, 1.0, 0.0]);
        let mut n = ninja_at(10.5 * TILE, 2.5 * TILE);
        let mut charging = false;
        for t in 0..frames(30) as u64 - 2 {
            assert_eq!(d.update(&l, &[], &n, t), None, "no harm while charging");
            charging |= d.charging();
        }
        assert!(charging);
        assert!(d.beam.is_some(), "the beam is fixed when the charge starts");
        // the ninja steps out of the line before it fires: no kill
        n.y -= 30.0;
        let mut killed = false;
        // the first beam: 45 charge frames in total, then 120 live, so 200 frames cover it
        for t in 0..150 {
            killed |= d.update(&l, &[], &n, 100 + t).is_some();
        }
        assert!(!killed, "a dodged beam does not kill");
        // standing in the line it does
        let mut d2 = Drone::from_params(&[2.5 * TILE, 2.5 * TILE, 2.0, 0.0, 1.0, 0.0]);
        let stay = ninja_at(10.5 * TILE, 2.5 * TILE);
        let mut got = None;
        for t in 0..400 {
            if let Some(c) = d2.update(&l, &[], &stay, t) {
                got = Some(c);
                break;
            }
        }
        assert_eq!(got, Some(Cause::Laser));
    }

    #[test]
    fn a_chaingun_drone_fires_a_burst_that_eventually_hits_a_standing_ninja() {
        let l = room();
        let mut d = Drone::from_params(&[2.5 * TILE, 2.5 * TILE, 2.0, 0.0, 2.0, 0.0]);
        let stay = ninja_at(9.5 * TILE, 2.5 * TILE);
        let mut got = None;
        for t in 0..600 {
            if let Some(c) = d.update(&l, &[], &stay, t) {
                got = Some(c);
                break;
            }
        }
        assert_eq!(got, Some(Cause::Bullet));
    }

    #[test]
    fn a_closed_door_blocks_a_drone_and_an_open_one_does_not() {
        let l = room();
        let mut w = Objects::from_level(&{
            let mut lv = room();
            // a locked door on the right edge of cell (6, 2)
            lv.add_test_objects("door 0 0 0 0 6 2 1 0 0");
            lv
        });
        let c = (6, 2);
        assert!(!edge_open(&l, &w.doors, c, 0), "closed door blocks");
        w.doors[0].open = true;
        assert!(edge_open(&l, &w.doors, c, 0), "open door passes");
    }
}
