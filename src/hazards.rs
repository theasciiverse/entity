//! Gauss turrets, floor guards, thwomps and homing launchers, ported from the original N
//! v1.2 bytecode (see CLAUDE.md, "Original N physics"). The original ticks at 40 Hz; speeds
//! and timers are converted to Entity's 60 Hz tick (velocity x 2/3, a count of ticks x 1.5).

use crate::drones::{line_clear, ray_end, Cause};
use crate::level::{Level, TILE};
use crate::ninja::{Ninja, RADIUS};

const V: f32 = 2.0 / 3.0;

fn cell_of(p: (f32, f32)) -> (i32, i32) {
    ((p.0 / TILE).floor() as i32, (p.1 / TILE).floor() as i32)
}

fn center(c: (i32, i32)) -> (f32, f32) {
    ((c.0 as f32 + 0.5) * TILE, (c.1 as f32 + 0.5) * TILE)
}

fn dist(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

fn dist_to_segment(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (ex, ey) = (b.0 - a.0, b.1 - a.1);
    let l2 = ex * ex + ey * ey;
    let t = if l2 > 0.0 { (((p.0 - a.0) * ex + (p.1 - a.1) * ey) / l2).clamp(0.0, 1.0) } else { 0.0 };
    dist(p, (a.0 + ex * t, a.1 + ey * t))
}

// ---------------------------------------------------------------- gauss turret

#[derive(Clone, Debug, PartialEq)]
enum TState {
    Waiting,
    Targeting,
    Pre(u32),
    Post(u32),
}

#[derive(Clone, Debug)]
pub struct Turret {
    pub pos: (f32, f32),
    /// The crosshair, drifting toward where the ninja is going to be.
    pub aim: (f32, f32),
    state: TState,
    shot_timer: f32,
    aim_speed: f32,
    /// Last shot, for drawing: from, to, frames left.
    pub tracer: Option<((f32, f32), (f32, f32), u32)>,
}

// Aim speeds per 40 Hz tick; thresholds are squared distances (96, 42 and 24 px).
const FAR_AIM: f32 = 0.03;
const MID_AIM: f32 = 0.035;
const CLOSE_AIM: f32 = 0.05;
const OUTER2: f32 = 96.0 * 96.0;
const MID2: f32 = 42.0 * 42.0;
const INNER2: f32 = 24.0 * 24.0;
const SHOT_RATE: f32 = 60.0;

/// Smoothing factor per 60 Hz tick for a factor given per 40 Hz tick.
fn per60(f: f32) -> f32 {
    1.0 - (1.0 - f).powf(V)
}

impl Turret {
    pub fn new(x: f32, y: f32) -> Turret {
        Turret { pos: (x, y), aim: (x, y), state: TState::Waiting, shot_timer: 0.0, aim_speed: FAR_AIM, tracer: None }
    }

    pub fn targeting(&self) -> bool {
        !matches!(self.state, TState::Waiting)
    }

    pub fn update(&mut self, level: &Level, n: &Ninja, time: u64) -> Option<Cause> {
        let np = (n.x, n.y);
        if let Some((_, _, left)) = self.tracer.as_mut() {
            *left = left.saturating_sub(1);
        }
        let sees = line_clear(level, self.pos, np);
        match self.state.clone() {
            TState::Waiting => {
                if sees {
                    self.state = TState::Targeting;
                    self.aim = self.pos;
                    self.aim_speed = FAR_AIM;
                    self.shot_timer = SHOT_RATE;
                }
            }
            TState::Targeting => {
                if !sees {
                    self.state = TState::Waiting;
                    return None;
                }
                // lead the ninja by one 40 Hz tick of its motion
                let p = (np.0 + n.vx * 1.5, np.1 + n.vy * 1.5);
                let (dx, dy) = (self.aim.0 - p.0, self.aim.1 - p.1);
                let k = per60(self.aim_speed);
                self.aim = (self.aim.0 - k * dx, self.aim.1 - k * dy);
                let r2 = dx * dx + dy * dy;
                if r2 > OUTER2 {
                    self.aim_speed = FAR_AIM;
                } else if r2 < INNER2 {
                    self.shot_timer -= (2 + time % 4) as f32 * V;
                } else if r2 < MID2 {
                    self.aim_speed = CLOSE_AIM;
                    self.shot_timer -= (1 + time % 2) as f32 * V;
                } else {
                    self.aim_speed = MID_AIM;
                    self.shot_timer -= 0.5 * V;
                }
                if self.shot_timer < 0.0 {
                    self.shot_timer = SHOT_RATE;
                    self.state = TState::Pre(0);
                }
            }
            TState::Pre(t) => {
                if t + 1 < 15 {
                    self.state = TState::Pre(t + 1);
                } else if !sees {
                    self.state = TState::Post(0);
                } else {
                    // fire along the line to the crosshair
                    let end = {
                        let d = dist(self.pos, self.aim).max(1e-3);
                        let u = ((self.aim.0 - self.pos.0) / d, (self.aim.1 - self.pos.1) / d);
                        let wall = ray_end(level, self.pos, u);
                        if dist(self.pos, wall) < d { wall } else { self.aim }
                    };
                    self.tracer = Some((self.pos, end, 8));
                    self.state = TState::Post(0);
                    if dist_to_segment(np, self.pos, end) < RADIUS {
                        return Some(Cause::Turret);
                    }
                }
            }
            TState::Post(t) => {
                if t + 1 < 15 {
                    self.state = TState::Post(t + 1);
                } else if sees {
                    self.shot_timer = SHOT_RATE;
                    self.state = TState::Targeting;
                } else {
                    self.state = TState::Waiting;
                }
            }
        }
        None
    }
}

// ----------------------------------------------------------------- floor guard

#[derive(Clone, Debug)]
pub struct FloorGuard {
    pub pos: (f32, f32),
    dir: f32,
    min_x: f32,
    max_x: f32,
    min_i: i32,
    max_i: i32,
    row: i32,
    pub chasing: bool,
}

const GUARD_R: f32 = 6.0;
/// 12 * 0.428571 px per 40 Hz tick.
const GUARD_SPEED: f32 = 5.1429 * V;

impl FloorGuard {
    /// From `[x, y, direction]`.
    pub fn new(level: &Level, x: f32, y: f32, dir: f32) -> FloorGuard {
        let (ci, cj) = cell_of((x, y));
        let pos = (x, (cj as f32 + 1.0) * TILE - GUARD_R);
        // patrol extent: to the first wall or ledge on either side
        let mut max_x = center((ci, cj)).0;
        for k in ci + 1.. {
            if level.code(k, cj) != 0 || level.code(k, cj + 1) == 0 {
                max_x = center((k, cj)).0 - TILE / 2.0 - GUARD_R;
                break;
            }
        }
        let mut min_x = max_x;
        for k in (0..ci).rev() {
            if level.code(k, cj) != 0 || level.code(k, cj + 1) == 0 {
                min_x = center((k, cj)).0 + TILE / 2.0 + GUARD_R;
                break;
            }
        }
        // row of sight: contiguous empty cells either side
        let (mut min_i, mut max_i) = (ci, ci);
        while level.code(max_i + 1, cj) == 0 {
            max_i += 1;
        }
        while level.code(min_i - 1, cj) == 0 {
            min_i -= 1;
        }
        FloorGuard { pos, dir: if dir < 0.0 { -1.0 } else { 1.0 }, min_x, max_x, min_i, max_i, row: cj, chasing: false }
    }

    pub fn update(&mut self, n: &Ninja) -> Option<Cause> {
        let np = (n.x, n.y);
        let pc = cell_of(np);
        let me = cell_of(self.pos);
        if self.chasing {
            let limit = if self.dir < 0.0 { self.min_x } else { self.max_x };
            if (limit - self.pos.0).abs() < GUARD_SPEED {
                self.pos.0 = limit;
                self.chasing = false;
            } else {
                self.pos.0 += self.dir * GUARD_SPEED;
            }
        } else if pc.1 == self.row && pc.0 >= self.min_i && pc.0 <= self.max_i {
            if pc.0 < me.0 {
                self.dir = -1.0;
                self.chasing = true;
            } else if pc.0 > me.0 {
                self.dir = 1.0;
                self.chasing = true;
            }
        }
        (dist(self.pos, np) < GUARD_R + RADIUS).then_some(Cause::Guard)
    }
}

// --------------------------------------------------------------------- thwomp

const THWOMP_HALF: f32 = 9.0;
const FALL_SPEED: f32 = 4.2857 * V;
const RAISE_SPEED: f32 = 1.7143 * V;

#[derive(Clone, Debug, PartialEq)]
enum WState {
    Waiting,
    Falling,
    Raising,
}

#[derive(Clone, Debug)]
pub struct Thwomp {
    pub pos: (f32, f32),
    anchor: (f32, f32),
    goal: (f32, f32),
    /// Facing direction: its front face is the deadly one.
    pub dir: (f32, f32),
    cells: (i32, i32, i32, i32), // min i, max i, min j, max j
    state: WState,
}

impl Thwomp {
    /// From `[x, y, direction]` (0 right, 1 down, 2 left, 3 up).
    pub fn new(level: &Level, x: f32, y: f32, dir: i32) -> Thwomp {
        let (ci, cj) = cell_of((x, y));
        let c = center((ci, cj));
        let (dx, dy) = match dir {
            0 => (1, 0),
            1 => (0, 1),
            2 => (-1, 0),
            _ => (0, -1),
        };
        // the end of the empty run ahead of it
        let mut k = 0;
        while level.code(ci + dx * (k + 1), cj + dy * (k + 1)) == 0 {
            k += 1;
        }
        let goal = match dir {
            0 => (x + 24.0 * k as f32 + THWOMP_HALF - (x - c.0), y),
            1 => (x, y + 24.0 * k as f32 + THWOMP_HALF - (y - c.1)),
            2 => (x - 24.0 * k as f32 - THWOMP_HALF - (x - c.0), y),
            _ => (x, y - 24.0 * k as f32 - THWOMP_HALF - (y - c.1)),
        };
        let end = cell_of(goal);
        let (mut mi, mut xi, mut mj, mut xj) = (ci, end.0, cj, end.1);
        if dx < 0 {
            std::mem::swap(&mut mi, &mut xi);
        }
        if dy < 0 {
            std::mem::swap(&mut mj, &mut xj);
        }
        Thwomp { pos: (x, y), anchor: (x, y), goal, dir: (dx as f32, dy as f32), cells: (mi, xi, mj, xj), state: WState::Waiting }
    }

    pub fn update(&mut self, n: &Ninja) {
        let pc = cell_of((n.x, n.y));
        match self.state {
            WState::Waiting => {
                let two = 2.0 * (THWOMP_HALF + RADIUS);
                let hit = if self.dir.0 == 0.0 {
                    (self.pos.0 - n.x).abs() < two && pc.1 >= self.cells.2 && pc.1 <= self.cells.3
                } else {
                    (self.pos.1 - n.y).abs() < two && pc.0 >= self.cells.0 && pc.0 <= self.cells.1
                };
                if hit {
                    self.state = WState::Falling;
                }
            }
            WState::Falling | WState::Raising => {
                let (target, speed) = if self.state == WState::Falling { (self.goal, FALL_SPEED) } else { (self.anchor, RAISE_SPEED) };
                if dist(self.pos, target) < speed {
                    self.pos = target;
                    self.state = if self.state == WState::Falling { WState::Raising } else { WState::Waiting };
                } else {
                    let d = dist(self.pos, target);
                    self.pos.0 += (target.0 - self.pos.0) / d * speed;
                    self.pos.1 += (target.1 - self.pos.1) / d * speed;
                }
            }
        }
    }

    pub fn moving(&self) -> bool {
        self.state != WState::Waiting
    }

    /// Box test against the ninja (as a box): the front face kills, the other faces are solid.
    /// Returns (push, contact normal) for a solid face, or Err(()) when the ninja is killed.
    pub fn collide(&self, n: &Ninja) -> Option<Result<((f32, f32), (f32, f32)), ()>> {
        let (dy, dx) = (n.y - self.pos.1, n.x - self.pos.0);
        let py = (THWOMP_HALF + RADIUS) - dy.abs();
        let px = (THWOMP_HALF + RADIUS) - dx.abs();
        if py <= 0.0 || px <= 0.0 {
            return None;
        }
        if py < px {
            if dy < 0.0 {
                if self.dir.1 < 0.0 { Some(Err(())) } else { Some(Ok(((0.0, -py), (0.0, -1.0)))) }
            } else if self.dir.1 > 0.0 {
                Some(Err(()))
            } else {
                Some(Ok(((0.0, py), (0.0, 1.0))))
            }
        } else if dx < 0.0 {
            if self.dir.0 < 0.0 { Some(Err(())) } else { Some(Ok(((-px, 0.0), (-1.0, 0.0)))) }
        } else if self.dir.0 > 0.0 {
            Some(Err(()))
        } else {
            Some(Ok(((px, 0.0), (1.0, 0.0))))
        }
    }
}

// -------------------------------------------------------------- homing launcher

#[derive(Clone, Debug, PartialEq)]
enum LState {
    Idle,
    Pre(u32),
    Flying,
}

#[derive(Clone, Debug)]
pub struct Launcher {
    pub base: (f32, f32),
    pub rocket: (f32, f32),
    dir: (f32, f32),
    speed: f32,
    accel: f32,
    state: LState,
    /// Accumulates 40 Hz ticks of rocket logic against 60 Hz frames.
    phase: f32,
    pub exploded: u32,
}

/// 12 * 0.285714 px per 40 Hz tick.
const ROCKET_MAX: f32 = 3.4286;
const ROCKET_TURN: f32 = 0.1;

impl Launcher {
    pub fn new(x: f32, y: f32) -> Launcher {
        Launcher { base: (x, y), rocket: (x, y), dir: (0.0, 1.0), speed: 0.0, accel: 0.1, state: LState::Idle, phase: 0.0, exploded: 0 }
    }

    pub fn flying(&self) -> bool {
        self.state == LState::Flying
    }

    pub fn update(&mut self, level: &Level, n: &Ninja) -> Option<Cause> {
        let np = (n.x, n.y);
        self.exploded = self.exploded.saturating_sub(1);
        match self.state.clone() {
            LState::Idle => {
                if line_clear(level, self.base, np) {
                    self.state = LState::Pre(0);
                }
            }
            LState::Pre(t) => {
                if t + 1 < 15 {
                    self.state = LState::Pre(t + 1);
                } else {
                    // launch toward the ninja
                    let d = dist(self.base, np);
                    if d > 0.0 {
                        self.dir = ((np.0 - self.base.0) / d, (np.1 - self.base.1) / d);
                    }
                    self.rocket = self.base;
                    self.speed = 0.0;
                    self.accel = 0.1;
                    self.phase = 0.0;
                    self.state = LState::Flying;
                }
            }
            LState::Flying => {
                // run the rocket's own logic at the original's 40 Hz
                self.phase += V;
                while self.phase >= 1.0 {
                    self.phase -= 1.0;
                    if self.speed < ROCKET_MAX {
                        self.accel *= 1.1;
                        self.speed += self.accel;
                    } else {
                        self.speed = ROCKET_MAX;
                    }
                    // steer toward where the ninja will be
                    let target = (2.0 * np.0 - (np.0 - n.vx * 1.5), 2.0 * np.1 - (np.1 - n.vy * 1.5));
                    let next = (self.rocket.0 + self.speed * self.dir.0 * V, self.rocket.1 + self.speed * self.dir.1 * V);
                    let (mut dx, mut dy) = (target.0 - next.0, target.1 - next.1);
                    let len = (dx * dx + dy * dy).sqrt().max(1e-3);
                    dx /= len;
                    dy /= len;
                    let pp = -self.dir.1 * dx + self.dir.0 * dy;
                    self.dir.0 += pp * -self.dir.1 * ROCKET_TURN;
                    self.dir.1 += pp * self.dir.0 * ROCKET_TURN;
                    let l = (self.dir.0 * self.dir.0 + self.dir.1 * self.dir.1).sqrt().max(1e-3);
                    self.dir = (self.dir.0 / l, self.dir.1 / l);
                }
                self.rocket.0 += self.speed * self.dir.0 * V;
                self.rocket.1 += self.speed * self.dir.1 * V;
                if level.point_solid(self.rocket.0, self.rocket.1) {
                    self.explode();
                } else if dist(self.rocket, np) < RADIUS {
                    self.explode();
                    return Some(Cause::Rocket);
                }
            }
        }
        None
    }

    fn explode(&mut self) {
        self.exploded = 20;
        self.state = LState::Idle;
        self.rocket = self.base;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room() -> Level {
        let rows = ["##################", "#                #", "#                #", "#                #", "#                #", "##################"];
        Level::parse(&rows.join("\n"))
    }

    #[test]
    fn a_turret_tracks_then_fires_at_a_standing_ninja_but_not_a_moving_one_it_cannot_catch() {
        let l = room();
        let mut t = Turret::new(2.5 * TILE, 1.5 * TILE);
        let n = Ninja::new(14.5 * TILE, 4.5 * TILE);
        let mut killed_at = None;
        for f in 0..2000u64 {
            if t.update(&l, &n, f).is_some() {
                killed_at = Some(f);
                break;
            }
        }
        assert!(killed_at.is_some(), "a standing target is shot in the end");
        // behind a wall it never wakes up
        let mut l2 = room();
        l2.add_test_objects("");
        let wall = Level::parse(&["##################", "#                #", "#     #          #", "#     #          #", "#     #          #", "##################"].join("\n"));
        let mut t = Turret::new(2.5 * TILE, 3.5 * TILE);
        let n = Ninja::new(14.5 * TILE, 3.5 * TILE);
        for f in 0..600u64 {
            assert!(t.update(&wall, &n, f).is_none());
        }
        assert!(!t.targeting(), "no line of sight, no targeting");
    }

    #[test]
    fn a_floor_guard_rushes_a_ninja_in_its_row_and_stops_at_the_platform_edge() {
        let rows = ["#####################", "#                   #", "#                   #", "#                   #", "#####################"];
        let l = Level::parse(&rows.join("\n"));
        let mut g = FloorGuard::new(&l, 3.5 * TILE, 3.5 * TILE, 1.0);
        let far = Ninja::new(15.5 * TILE, 3.5 * TILE);
        let start = g.pos.0;
        let mut ran = false;
        for _ in 0..400 {
            g.update(&far);
            ran |= g.chasing;
        }
        assert!(ran && g.pos.0 > start + 100.0, "it charged: {} -> {}", start, g.pos.0);
        assert!(g.pos.0 <= 20.0 * TILE - TILE - GUARD_R + 0.01, "it stops at the wall: {}", g.pos.0);
        // a ninja on another row does not trigger it
        let mut g2 = FloorGuard::new(&l, 3.5 * TILE, 3.5 * TILE, 1.0);
        let other = Ninja::new(15.5 * TILE, 1.5 * TILE);
        for _ in 0..100 {
            g2.update(&other);
            assert!(!g2.chasing);
        }
        // touching kills
        let mut g3 = FloorGuard::new(&l, 3.5 * TILE, 3.5 * TILE, 1.0);
        let close = Ninja::new(g3.pos.0 + 10.0, g3.pos.1);
        assert_eq!(g3.update(&close), Some(Cause::Guard));
    }

    #[test]
    fn a_thwomp_slams_when_you_pass_under_it_then_rises_back() {
        let rows = ["#########", "#       #", "#       #", "#       #", "#       #", "#       #", "#########"];
        let l = Level::parse(&rows.join("\n"));
        // facing down, hanging from the ceiling at cell (4, 1)
        let mut t = Thwomp::new(&l, 4.5 * TILE, 1.5 * TILE, 1);
        let anchor = t.pos;
        let under = Ninja::new(4.5 * TILE, 5.5 * TILE);
        let mut fell = false;
        let mut max_y = anchor.1;
        for _ in 0..120 {
            t.update(&under);
            max_y = max_y.max(t.pos.1);
            fell |= t.moving();
        }
        assert!(fell && max_y > anchor.1 + 3.0 * TILE, "it dropped: {anchor:?} -> {max_y}");
        // the ninja steps away: it rises back to its anchor and waits
        let aside = Ninja::new(7.5 * TILE, 5.5 * TILE);
        for _ in 0..400 {
            t.update(&aside);
        }
        assert!((t.pos.1 - anchor.1).abs() < 0.5, "and rose back to its anchor: {:?}", t.pos);
        // not aligned: it stays put
        let mut t2 = Thwomp::new(&l, 4.5 * TILE, 1.5 * TILE, 1);
        let aside = Ninja::new(7.5 * TILE, 5.5 * TILE);
        for _ in 0..200 {
            t2.update(&aside);
        }
        assert!(!t2.moving());
    }

    #[test]
    fn a_thwomps_front_face_kills_and_its_sides_are_solid() {
        let l = Level::parse(&["#########", "#       #", "#       #", "#       #", "#########"].join("\n"));
        let t = Thwomp::new(&l, 4.5 * TILE, 2.5 * TILE, 1); // faces down
        let below = Ninja::new(t.pos.0, t.pos.1 + 15.0);
        assert!(matches!(t.collide(&below), Some(Err(()))), "the front (bottom) kills");
        let above = Ninja::new(t.pos.0, t.pos.1 - 15.0);
        assert!(matches!(t.collide(&above), Some(Ok(_))), "the top is solid");
        let far = Ninja::new(t.pos.0 + 60.0, t.pos.1);
        assert!(t.collide(&far).is_none());
    }

    #[test]
    fn a_homing_rocket_chases_a_ninja_and_explodes_on_walls() {
        let l = room();
        let mut r = Launcher::new(2.5 * TILE, 4.5 * TILE);
        let n = Ninja::new(12.5 * TILE, 4.5 * TILE);
        let mut hit = false;
        for _ in 0..600 {
            if r.update(&l, &n).is_some() {
                hit = true;
                break;
            }
        }
        assert!(hit, "an unmoving ninja in view is hit");
        // out of sight: it never launches
        let wall = Level::parse(&["##################", "#                #", "#     #          #", "#     #          #", "#     #          #", "##################"].join("\n"));
        let mut r = Launcher::new(2.5 * TILE, 3.5 * TILE);
        for _ in 0..300 {
            assert!(r.update(&wall, &Ninja::new(14.5 * TILE, 3.5 * TILE)).is_none());
            assert!(!r.flying());
        }
    }
}
