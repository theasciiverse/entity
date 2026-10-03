//! Ninja movement and collision. Units are pixels (24 unit tiles) and the tick is 60 Hz.
//! The constants equal the original N v1.2 values read from its bytecode (a 40 Hz game:
//! velocity x 2/3, acceleration x 4/9, per-tick factor f -> f^(2/3)); see CLAUDE.md,
//! "Original N physics". `n_rules` turns on the original's structure (wall kick variants,
//! persistent wall sliding, skid friction, landing-only deaths, wall and ground detection);
//! `ENTITY_PHYSICS=classic` switches it off for comparison.

use crate::level::{Level, TILE};
use crate::objects::{Objects, Seg};
use crate::shapes::{edge_side, shape, BOTTOM, LEFT, RIGHT, TOP};

pub const RADIUS: f32 = 10.0;

const GRAVITY: f32 = 0.0666;
const GRAVITY_JUMP: f32 = 0.0111;
const GROUND_ACCEL: f32 = 0.0666;
const AIR_ACCEL: f32 = 0.0444;
const DRAG: f32 = 0.9933;
const FRICTION_GROUND: f32 = 0.9459;
/// Standing friction once nearly stopped (original: 0.8 per 40 Hz tick).
const FRICTION_STAND: f32 = 0.8619;
/// Along-surface speed below which a skid ends and the ninja is standing (0.1 per 40 Hz tick).
const STAND_SPEED: f32 = 0.0667;
const FRICTION_WALL: f32 = 0.9113;
const MAX_SPEED: f32 = 3.333;
const JUMP_FLOOR_VY: f32 = -2.0;
const JUMP_WALL_VX: f32 = 1.0;
const JUMP_WALL_VY: f32 = -1.4;
/// Wall jump while pressing INTO the wall is weaker: out 1.0 and up 0.5 x3 per 40 Hz tick.
const JUMP_WALL_INTO_VX: f32 = 0.6667;
const JUMP_WALL_INTO_VY: f32 = -1.0;
/// Ground jump upward impulse per unit of surface normal (3 x 2/3), and the straight-up
/// jump when pressing toward a slope's rising side (0.7 x 3 x 2/3).
const JUMP_NORMAL: f32 = 2.0;
const JUMP_UPHILL_VY: f32 = -1.4;
/// Landing speed along the floor normal that kills: 6 - 1.333 * |ny| (original: 9 - 2|ny| per 40 Hz tick).
const DEATH_LAND_BASE: f32 = 6.0;
const DEATH_LAND_SLOPE: f32 = 1.3333;
/// Wall sensing distance beyond the radius (the original queries a point at r + 0.1).
const WALL_SENSE: f32 = 0.1;
const JUMP_HOLD_FRAMES: u32 = 45;
/// A jump pressed this many frames before landing or touching a wall still happens.
const JUMP_BUFFER_FRAMES: u32 = 6;
/// A jump still works this many frames after leaving the ground or a wall.
const COYOTE_FRAMES: u32 = 6;
/// Impact speed (into the surface) above which the ninja dies.
const DEATH_IMPACT_FLOOR: f32 = 6.0;
const DEATH_IMPACT_OTHER: f32 = 5.0;
/// Contact probe distance beyond the radius, so resting contact is detected.
const PROBE: f32 = 0.6;

#[derive(Clone, Copy, Default)]
pub struct Controls {
    /// -1 left, 0 none, 1 right.
    pub dir: i8,
    pub jump_held: bool,
    pub jump_pressed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    Jump,
    WallJump,
    Land(f32),
    Died(f32),
}

#[derive(Clone)]
pub struct Ninja {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub grounded: bool,
    /// +1 wall on the right, -1 wall on the left, 0 none.
    pub wall: i8,
    /// Last horizontal direction the ninja was heading or kicked: +1 right, -1 left.
    pub facing: i8,
    /// Distance run on the ground, drives the run animation.
    stride: f32,
    jumping: bool,
    jump_timer: u32,
    jump_buffer: u32,
    ground_coyote: u32,
    wall_coyote: u32,
    /// Original N rules on (default); `ENTITY_PHYSICS=classic` turns them off.
    pub n_rules: bool,
    /// Sliding down a wall (started by pressing toward it while falling; persists until
    /// pressing away or leaving the wall).
    pub sliding: bool,
    /// Average normal of the floor under the ninja (0, -1 when flat).
    floor_n: (f32, f32),
    /// Side of the wall touched most recently (valid while `wall_coyote > 0`).
    wall_memory: i8,
}

struct Contact {
    nx: f32,
    ny: f32,
    depth: f32,
}

/// Closest point on segment pq to (x, y).
fn closest(p: (f32, f32), q: (f32, f32), x: f32, y: f32) -> (f32, f32) {
    let (ex, ey) = (q.0 - p.0, q.1 - p.1);
    let len2 = ex * ex + ey * ey;
    let t = if len2 > 0.0 { (((x - p.0) * ex + (y - p.1) * ey) / len2).clamp(0.0, 1.0) } else { 0.0 };
    (p.0 + ex * t, p.1 + ey * t)
}

/// All circle contacts within `reach` of (x, y) against the tile polygons.
/// Edges lying on a tile border are skipped when the tile across it is flush
/// solid there, so seams between tiles do not produce ghost bumps.
fn contacts(level: &Level, extra: &[Seg], x: f32, y: f32, reach: f32, out: &mut Vec<Contact>) {
    out.clear();
    for sg in extra {
        let c = closest(sg.a, sg.b, x, y);
        let d2 = (x - c.0) * (x - c.0) + (y - c.1) * (y - c.1);
        if d2 >= reach * reach {
            continue;
        }
        let dist = d2.sqrt();
        let one_sided = sg.n != (0.0, 0.0);
        if one_sided && (x - c.0) * sg.n.0 + (y - c.1) * sg.n.1 < -1e-3 {
            continue; // behind the facing side
        }
        if dist > 1e-4 {
            out.push(Contact { nx: (x - c.0) / dist, ny: (y - c.1) / dist, depth: reach - dist });
        } else if one_sided {
            out.push(Contact { nx: sg.n.0, ny: sg.n.1, depth: reach });
        } else {
            // centre exactly on a two-sided barrier: leave along its perpendicular
            let (ex, ey) = (sg.b.0 - sg.a.0, sg.b.1 - sg.a.1);
            let l = (ex * ex + ey * ey).sqrt().max(1e-6);
            out.push(Contact { nx: -ey / l, ny: ex / l, depth: reach });
        }
    }
    let tx0 = ((x - reach) / TILE).floor() as i32;
    let tx1 = ((x + reach) / TILE).floor() as i32;
    let ty0 = ((y - reach) / TILE).floor() as i32;
    let ty1 = ((y + reach) / TILE).floor() as i32;
    for ty in ty0..=ty1 {
        for tx in tx0..=tx1 {
            let code = level.code(tx, ty);
            if code == 0 {
                continue;
            }
            let poly = &shape(code).poly;
            let (ox, oy) = (tx as f32 * TILE, ty as f32 * TILE);
            let mut best: Option<(f32, (f32, f32))> = None;
            for i in 0..poly.len() {
                let (p, q) = (poly[i], poly[(i + 1) % poly.len()]);
                if let Some(side) = edge_side(p, q) {
                    let (nx, ny, opp) = match side {
                        LEFT => (tx - 1, ty, RIGHT),
                        RIGHT => (tx + 1, ty, LEFT),
                        TOP => (tx, ty - 1, BOTTOM),
                        _ => (tx, ty + 1, TOP),
                    };
                    let nc = level.code(nx, ny);
                    if nc != 0 && shape(nc).side_full[opp] {
                        continue;
                    }
                }
                let c = closest((ox + p.0 * TILE, oy + p.1 * TILE), (ox + q.0 * TILE, oy + q.1 * TILE), x, y);
                let d2 = (x - c.0) * (x - c.0) + (y - c.1) * (y - c.1);
                if best.map_or(true, |(b, _)| d2 < b) {
                    best = Some((d2, c));
                }
            }
            let Some((d2, c)) = best else { continue };
            let dist = d2.sqrt();
            let inside = crate::shapes::inside(poly, (x - ox) / TILE, (y - oy) / TILE);
            if inside {
                // centre inside solid: leave through the nearest visible edge
                if dist > 1e-6 {
                    out.push(Contact { nx: (c.0 - x) / dist, ny: (c.1 - y) / dist, depth: dist + reach });
                }
            } else if dist < reach && dist > 1e-6 {
                out.push(Contact { nx: (x - c.0) / dist, ny: (y - c.1) / dist, depth: reach - dist });
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pose {
    Stand,
    Run(usize),
    Jump,
    Fall,
    Slide,
}

/// `ENTITY_PHYSICS=classic` disables the original-N rules.
fn n_rules_default() -> bool {
    use std::sync::OnceLock;
    static V: OnceLock<bool> = OnceLock::new();
    *V.get_or_init(|| std::env::var("ENTITY_PHYSICS").map_or(true, |v| v != "classic"))
}

impl Ninja {
    pub fn pose(&self) -> Pose {
        if self.grounded {
            if self.vx.abs() < 0.3 {
                Pose::Stand
            } else {
                Pose::Run((self.stride / 6.0) as usize % 4)
            }
        } else if (self.sliding || (!self.n_rules && self.wall != 0)) && self.vy > 0.0 {
            Pose::Slide
        } else if self.vy < 0.0 {
            Pose::Jump
        } else {
            Pose::Fall
        }
    }

    pub fn new(x: f32, y: f32) -> Ninja {
        Ninja { x, y, vx: 0.0, vy: 0.0, grounded: false, wall: 0, facing: 1, stride: 0.0, jumping: false, jump_timer: 0, jump_buffer: 0, ground_coyote: 0, wall_coyote: 0, n_rules: n_rules_default(), sliding: false, floor_n: (0.0, -1.0), wall_memory: 0 }
    }

    /// Step without level objects (used by the tests).
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn step(&mut self, level: &Level, c: &Controls) -> Vec<Event> {
        self.step_in(level, c, &mut Objects::none())
    }

    /// Replace the velocity (launch pads): the original zeroes the old velocity and adds the
    /// launch, then the ninja is falling.
    pub fn launch(&mut self, vx: f32, vy: f32) {
        self.vx = vx;
        self.vy = vy;
        self.jumping = false;
        self.sliding = false;
        self.grounded = false;
        self.ground_coyote = 0;
        self.wall_coyote = 0;
        self.jump_buffer = 0;
    }

    /// One tick inside a world of level objects.
    pub fn step_in(&mut self, level: &Level, c: &Controls, world: &mut Objects) -> Vec<Event> {
        let mut events = Vec::new();

        if c.jump_pressed {
            self.jump_buffer = JUMP_BUFFER_FRAMES;
        }
        if self.jump_buffer > 0 {
            if self.grounded || self.ground_coyote > 0 {
                if self.n_rules {
                    // original: jump along the floor normal, or straight up (and weaker) when
                    // pressing toward the slope's rising side
                    let (nx, ny) = self.floor_n;
                    let (ix, iy) = if (c.dir as f32) * nx < 0.0 { (0.0, JUMP_UPHILL_VY) } else { (nx * JUMP_NORMAL * 0.3333, ny * JUMP_NORMAL) };
                    self.impulse(ix, iy);
                } else {
                    self.vy = JUMP_FLOOR_VY;
                }
                self.start_jump();
                events.push(Event::Jump);
            } else if self.wall != 0 || self.wall_coyote > 0 {
                let side = if self.wall != 0 { self.wall } else { self.wall_memory };
                self.facing = -side;
                if self.n_rules {
                    let toward = c.dir == side;
                    let (out, up) = if toward { (JUMP_WALL_INTO_VX, JUMP_WALL_INTO_VY) } else { (JUMP_WALL_VX, JUMP_WALL_VY) };
                    self.impulse(-(side as f32) * out, up);
                } else {
                    self.vx = -(side as f32) * JUMP_WALL_VX;
                    self.vy = JUMP_WALL_VY;
                }
                self.start_jump();
                events.push(Event::WallJump);
            } else {
                self.jump_buffer -= 1;
            }
        }

        let dir = c.dir as f32;
        self.vx += dir * if self.grounded { GROUND_ACCEL } else { AIR_ACCEL };

        if self.jumping && (!c.jump_held || self.vy >= 0.0 || self.jump_timer >= JUMP_HOLD_FRAMES) {
            self.jumping = false;
        }
        self.vy += if self.jumping { GRAVITY_JUMP } else { GRAVITY };
        self.jump_timer += 1;

        if self.n_rules {
            // wall slide: starts when falling toward a wall you press into, then persists
            // (friction even with no key held) until you press away or leave the wall
            if self.wall != 0 && !self.grounded {
                if self.sliding {
                    if c.dir == -self.wall {
                        self.sliding = false;
                    }
                } else if self.vy > 0.0 && c.dir == self.wall {
                    self.sliding = true;
                }
            } else {
                self.sliding = false;
            }
            if self.sliding && self.vy > 0.0 {
                self.vy *= FRICTION_WALL;
            }
            // pressing up a slope adds acceleration along it (the original's running state)
            if self.grounded && (c.dir as f32) * self.floor_n.0 < 0.0 && self.vx.abs() < MAX_SPEED {
                let (nx, ny) = self.floor_n;
                let tx = if nx < 0.0 { -ny } else { ny };
                let ty = -nx.abs();
                let amt = 0.5 * ny.abs();
                self.vx += tx * amt * GROUND_ACCEL;
                self.vy += ty * amt * GROUND_ACCEL;
            }
            // skid: releasing or reversing on the ground brakes; nearly stopped it stops hard
            if self.grounded {
                let braking = c.dir == 0 || (c.dir as f32) * self.vx < 0.0;
                if braking {
                    let (nx, ny) = self.floor_n;
                    let along = (self.vx * -ny + self.vy * nx).abs();
                    self.vx *= if along < STAND_SPEED { FRICTION_STAND } else { FRICTION_GROUND };
                }
            }
        } else {
            if self.grounded && c.dir == 0 {
                self.vx *= FRICTION_GROUND;
            }
            if self.wall != 0 && self.vy > 0.0 && c.dir == self.wall {
                self.vy *= FRICTION_WALL;
            }
        }
        if !self.grounded {
            self.vx *= DRAG;
            self.vy *= DRAG;
        }
        self.vx = self.vx.clamp(-MAX_SPEED, MAX_SPEED);

        let was_grounded = self.grounded;
        let prev = (self.x, self.y);
        self.x += self.vx;
        self.y += self.vy;
        let touching = world.collide_blocks(self);
        let extra = world.colliders(self, prev);
        let (floor_impact, other_impact, lethal_landing) = self.resolve(level, &extra);
        self.probe(level, &extra, &touching);
        world.tick_blocks();
        self.ground_coyote = if self.grounded { COYOTE_FRAMES } else { self.ground_coyote.saturating_sub(1) };
        if self.wall != 0 {
            self.wall_coyote = COYOTE_FRAMES;
            self.wall_memory = self.wall;
        } else {
            self.wall_coyote = self.wall_coyote.saturating_sub(1);
        }
        if c.dir != 0 {
            self.facing = c.dir;
        }
        if self.wall != 0 && !self.grounded && self.vy > 0.0 {
            self.facing = self.wall; // sliding down a wall, facing it
        }
        if self.grounded {
            self.stride += self.vx.abs();
        }

        let dies = if self.n_rules {
            // original: only a landing from the air kills; walls and ceilings never do
            lethal_landing && !was_grounded
        } else {
            floor_impact > DEATH_IMPACT_FLOOR || other_impact > DEATH_IMPACT_OTHER
        };
        if dies {
            events.push(Event::Died(floor_impact.max(other_impact)));
        } else if self.grounded && !was_grounded && floor_impact > 0.5 {
            events.push(Event::Land(floor_impact));
        }
        events
    }

    fn start_jump(&mut self) {
        self.jumping = true;
        self.jump_timer = 0;
        self.jump_buffer = 0;
        self.grounded = false;
        self.wall = 0;
        self.ground_coyote = 0;
        self.wall_coyote = 0;
    }

    /// Push out of solid tiles, deepest contact first; returns the largest
    /// (floor, non-floor) impact speeds.
    fn resolve(&mut self, level: &Level, extra: &[Seg]) -> (f32, f32, bool) {
        let mut buf = Vec::new();
        let (mut floor_impact, mut other_impact) = (0.0f32, 0.0f32);
        let mut lethal = false;
        for _ in 0..8 {
            contacts(level, extra, self.x, self.y, RADIUS, &mut buf);
            let Some(c) = buf.iter().max_by(|a, b| a.depth.total_cmp(&b.depth)) else { break };
            if c.depth < 1e-3 {
                break;
            }
            self.x += c.nx * c.depth;
            self.y += c.ny * c.depth;
            let vn = self.vx * c.nx + self.vy * c.ny;
            if vn < 0.0 {
                self.vx -= vn * c.nx;
                self.vy -= vn * c.ny;
                if c.ny < -0.5 {
                    floor_impact = floor_impact.max(-vn);
                } else {
                    other_impact = other_impact.max(-vn);
                }
                if c.ny < -0.01 && -vn > DEATH_LAND_BASE - DEATH_LAND_SLOPE * c.ny.abs() {
                    lethal = true;
                }
            }
        }
        (floor_impact, other_impact, lethal)
    }

    /// Add an impulse as the original's `Jump(x, y)` does: a velocity component that
    /// opposes the impulse is cancelled first, then the impulse is added.
    fn impulse(&mut self, ix: f32, iy: f32) {
        if self.vx * ix < 0.0 {
            self.vx = 0.0;
        }
        if self.vy * iy < 0.0 {
            self.vy = 0.0;
        }
        self.vx += ix;
        self.vy += iy;
    }

    fn probe(&mut self, level: &Level, extra: &[Seg], touching: &[(f32, f32)]) {
        let mut buf = Vec::new();
        contacts(level, extra, self.x, self.y, RADIUS + PROBE, &mut buf);
        for &(nx, ny) in touching {
            buf.push(Contact { nx, ny, depth: 0.0 });
        }
        if self.n_rules {
            // original: any floor normal pointing even slightly up is ground (67 degree slopes
            // included); walls are point queries just outside the radius, plus vertical faces
            let floors: Vec<&Contact> = buf.iter().filter(|c| c.ny < -0.01).collect();
            self.grounded = !floors.is_empty();
            if self.grounded {
                let (sx, sy) = floors.iter().fold((0.0, 0.0), |a, c| (a.0 + c.nx, a.1 + c.ny));
                let len = (sx * sx + sy * sy).sqrt();
                self.floor_n = if len > 1e-6 { (sx / len, sy / len) } else { (0.0, -1.0) };
            } else {
                self.floor_n = (0.0, -1.0);
            }
            let reach = RADIUS + WALL_SENSE;
            self.wall = if level.point_solid(self.x + reach, self.y) {
                1
            } else if level.point_solid(self.x - reach, self.y) {
                -1
            } else {
                buf.iter().find(|c| c.ny.abs() < 0.01 && c.nx.abs() > 0.99).map_or(0, |c| -(c.nx.signum() as i8))
            };
        } else {
            self.grounded = buf.iter().any(|c| c.ny < -0.7);
            self.wall = buf.iter().find(|c| c.nx.abs() > 0.85).map_or(0, |c| -(c.nx.signum() as i8));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tall_room() -> Level {
        // 12 wide, 30 tall, floor at row 28 and 29, 2 tile wide walls not needed.
        let mut s = String::new();
        for y in 0..30 {
            if y >= 28 {
                s.push_str(&"#".repeat(40));
            } else {
                s.push('#');
                s.push_str(&" ".repeat(38));
                s.push('#');
            }
            s.push('\n');
        }
        Level::parse(&s)
    }

    fn floor_y() -> f32 {
        28.0 * TILE - RADIUS
    }

    fn settle(level: &Level, n: &mut Ninja) {
        for _ in 0..120 {
            n.step(level, &Controls::default());
        }
    }

    #[test]
    fn lands_and_rests_on_floor() {
        let level = tall_room();
        let mut n = Ninja::new(5.0 * TILE, 24.0 * TILE);
        settle(&level, &mut n);
        assert!(n.grounded);
        assert!((n.y - floor_y()).abs() < 0.5, "y = {}", n.y);
    }

    #[test]
    fn runs_to_max_speed_without_bumps_on_flat_floor() {
        let level = tall_room();
        let mut n = Ninja::new(3.0 * TILE, floor_y());
        settle(&level, &mut n);
        let y0 = n.y;
        let c = Controls { dir: 1, ..Default::default() };
        for _ in 0..80 {
            n.step(&level, &c);
            assert!((n.y - y0).abs() < 0.01, "bump at x = {}: y = {}", n.x, n.y);
            assert!(n.grounded);
        }
        assert!(n.vx > 3.2 && n.vx <= MAX_SPEED, "vx = {}", n.vx);
    }

    #[test]
    fn jump_height_is_a_few_tiles_and_hold_matters() {
        let level = tall_room();
        let peak = |hold: bool| {
            let mut n = Ninja::new(5.0 * TILE, floor_y());
            settle(&level, &mut n);
            let start = n.y;
            let mut top = start;
            for i in 0..120 {
                let c = Controls { dir: 0, jump_held: hold, jump_pressed: i == 0 };
                n.step(&level, &c);
                top = top.min(n.y);
            }
            (start - top) / TILE
        };
        let (high, low) = (peak(true), peak(false));
        assert!(high > 3.0 && high < 5.0, "held jump = {high} tiles");
        assert!(low < high - 1.0, "tap jump = {low} tiles");
    }

    #[test]
    fn wall_is_detected_at_rest_and_wall_jump_kicks_away() {
        let level = tall_room();
        let mut n = Ninja::new(24.0 * 1.0 + RADIUS + 1.0, 20.0 * TILE);
        // Fall while pressing into the left wall until sliding.
        let into = Controls { dir: -1, ..Default::default() };
        for _ in 0..20 {
            n.step(&level, &into);
        }
        assert_eq!(n.wall, -1, "x = {}", n.x);
        let jump = Controls { dir: -1, jump_held: true, jump_pressed: true };
        let ev = n.step(&level, &jump);
        assert!(ev.contains(&Event::WallJump));
        assert!(n.vx > 0.5 && n.vy < 0.0, "v = ({}, {})", n.vx, n.vy);
    }

    #[test]
    fn long_fall_is_fatal_short_fall_is_not() {
        let level = tall_room();
        let mut n = Ninja::new(5.0 * TILE, 2.0 * TILE);
        let mut died = false;
        for _ in 0..300 {
            if n.step(&level, &Controls::default()).iter().any(|e| matches!(e, Event::Died(_))) {
                died = true;
                break;
            }
        }
        assert!(died);

        let mut n = Ninja::new(5.0 * TILE, 22.0 * TILE);
        for _ in 0..300 {
            let ev = n.step(&level, &Controls::default());
            assert!(!ev.iter().any(|e| matches!(e, Event::Died(_))));
        }
    }

    fn run_until(level: &Level, n: &mut Ninja, c: Controls, frames: usize) {
        for _ in 0..frames {
            n.step(level, &c);
        }
    }

    #[test]
    fn slides_off_a_45_degree_slope() {
        // code 2 (45 degree, normal up-right) is high on the left, low on the right
        let level = Level::parse(
            "############\n#          #\n#          #\n#          #\n#          #\n#     2    #\n############\n",
        );
        let mut n = Ninja::new(6.2 * TILE, 2.0 * TILE);
        run_until(&level, &mut n, Controls::default(), 200);
        assert!(n.grounded);
        assert!(n.x > 7.0 * TILE, "x = {}", n.x / TILE);
        assert!((n.y - (6.0 * TILE - RADIUS)).abs() < 1.0, "y = {}", n.y);
    }

    #[test]
    fn climbs_a_22_degree_ramp() {
        // nnS (15 = '?') then nnB (19, set below) rise one tile over two; a full tile tops it off
        let mut s = String::new();
        let rows = ["############", "#          #", "#          #", "#          #", "#      #   #", "#   ?3#    #", "############"];
        for r in rows {
            s.push_str(r);
            s.push('\n');
        }
        let s = s.replace('3', &((b'0' + 19) as char).to_string());
        let level = Level::parse(&s);
        let mut n = Ninja::new(2.0 * TILE, 6.0 * TILE - RADIUS);
        settle(&level, &mut n);
        run_until(&level, &mut n, Controls { dir: 1, ..Default::default() }, 120);
        assert!(n.x > 6.0 * TILE && n.y < 5.0 * TILE, "pos = ({}, {}) tiles", n.x / TILE, n.y / TILE);
    }

    #[test]
    fn runs_up_a_45_degree_slope_from_a_standstill() {
        // code 3 (45 degree, normal up-left) rises to the right; three of them make a ramp of
        // three tiles up, with a full tile row on top
        let rows = [
            "####################",
            "#                  #",
            "#                  #",
            "#           3######",
            "#          3#######",
            "#         3########",
            "####################",
        ];
        let rows: Vec<String> = rows.iter().map(|r| r.replace('#', "#")).collect();
        let level = Level::parse(&rows.join("\n"));
        let mut n = Ninja::new(3.0 * TILE, 6.0 * TILE - RADIUS);
        settle(&level, &mut n);
        run_until(&level, &mut n, Controls { dir: 1, ..Default::default() }, 400);
        assert!(n.x > 14.0 * TILE && n.y < 4.0 * TILE, "got to ({:.1}, {:.1}) tiles", n.x / TILE, n.y / TILE);
    }

    #[test]
    fn rolls_off_a_hill_crest() {
        // convex nn (11 = ';') and pn (10 = ':') meet at an apex on the shared tile edge
        let rows = [
            "################",
            "#              #",
            "#              #",
            "#              #",
            "#     ;:       #",
            "#   ##########  #",
            "################",
        ];
        let level = Level::parse(&rows.join("\n"));
        // dropped just right of the apex (tile x = 7.0), it must slide down the right side
        let mut n = Ninja::new(7.15 * TILE, 2.0 * TILE);
        run_until(&level, &mut n, Controls::default(), 600);
        assert!(n.grounded);
        assert!(n.x > 8.5 * TILE, "x = {}", n.x / TILE);
        assert!((n.y - (5.0 * TILE - RADIUS)).abs() < 1.0, "y = {}", n.y);
    }

    #[test]
    fn every_n_level_loads_with_a_sane_spawn() {
        let levels = crate::level::n_levels();
        assert_eq!(levels.len(), 150);
        for l in &levels {
            let mut n = Ninja::new(l.spawn.0, l.spawn.1);
            // spawn must not be buried: first contact depth stays small
            for _ in 0..60 {
                let ev = n.step(l, &Controls::default());
                assert!(n.x.is_finite() && n.y.is_finite(), "{}", l.label);
                assert!(!ev.iter().any(|e| matches!(e, Event::Died(_))), "{} dies idle at spawn", l.label);
            }
            assert!(n.x > 0.0 && n.y > 0.0 && n.x < (l.w as f32) * TILE && n.y < (l.h as f32) * TILE, "{} left the level", l.label);
        }
    }
}

#[cfg(test)]
mod walljump {
    use super::*;

    /// A shaft `gap` tiles wide, 40 tiles tall, with a floor.
    fn shaft(gap: usize) -> Level {
        let w = gap + 2;
        let mut rows = Vec::new();
        for y in 0..44 {
            let mut r = String::new();
            for x in 0..w {
                r.push(if y >= 41 || x == 0 || x == w - 1 { '#' } else { ' ' });
            }
            rows.push(r);
        }
        Level::parse(&rows.join("\n"))
    }

    /// Zig-zag: kick off whichever wall is touched toward the other, holding jump.
    fn zigzag(gap: usize, frames: usize) -> (f32, usize) {
        let level = shaft(gap);
        let mut n = Ninja::new(TILE + RADIUS + 0.5, 41.0 * TILE - RADIUS);
        let (mut top, mut hops, mut dir) = (n.y, 0, 1i8);
        let start = n.y;
        for f in 0..frames {
            let touching = n.wall != 0;
            if touching {
                dir = -n.wall;
            }
            let c = Controls { dir, jump_held: true, jump_pressed: f == 0 || (touching && f % 3 == 0) };
            let ev = n.step(&level, &c);
            hops += ev.iter().filter(|e| matches!(e, Event::Jump | Event::WallJump)).count();
            top = top.min(n.y);
        }
        ((start - top) / TILE, hops)
    }

    #[test]
    fn wall_jumping_climbs_shafts_of_every_ordinary_width() {
        for gap in 1..=4 {
            let (tiles, hops) = zigzag(gap, 900);
            assert!(tiles > 25.0, "gap {gap}: only climbed {tiles:.1} tiles in {hops} jumps");
        }
    }

    #[test]
    fn a_single_wall_can_be_climbed_by_hopping_back_into_it() {
        // Measured, not a design rule: kick off one wall while holding back toward it and
        // the ninja keeps returning and re-kicking. About 20 tiles in 600 frames (10 s) with
        // perfect inputs. Whether the original allows this, and how fast, is unchecked; this
        // pins the current behaviour so tuning changes show up.
        let level = shaft(40);
        let mut n = Ninja::new(TILE + RADIUS + 0.5, 20.0 * TILE);
        let start = n.y;
        let mut top = start;
        for f in 0..600 {
            let touching = n.wall != 0;
            let c = Controls { dir: -1, jump_held: true, jump_pressed: touching && f % 4 == 0 };
            n.step(&level, &c);
            top = top.min(n.y);
        }
        let tiles = (start - top) / TILE;
        assert!(tiles > 5.0 && tiles < 30.0, "climbed {tiles:.1} tiles up one wall");
    }

    #[test]
    fn poses_follow_what_the_ninja_is_doing() {
        let level = shaft(3);
        let mut n = Ninja::new(2.5 * TILE, 41.0 * TILE - RADIUS);
        for _ in 0..30 {
            n.step(&level, &Controls::default());
        }
        assert_eq!(n.pose(), Pose::Stand);
        for _ in 0..30 {
            n.step(&level, &Controls { dir: 1, ..Default::default() });
        }
        assert!(matches!(n.pose(), Pose::Run(_)) || n.wall != 0, "{:?}", n.pose());
        // rising, then falling
        let mut n = Ninja::new(2.5 * TILE, 41.0 * TILE - RADIUS);
        n.step(&level, &Controls::default());
        n.step(&level, &Controls { jump_pressed: true, jump_held: true, dir: 0 });
        assert_eq!(n.pose(), Pose::Jump);
        for _ in 0..80 {
            n.step(&level, &Controls::default());
            if n.vy > 0.5 && !n.grounded {
                assert_eq!(n.pose(), Pose::Fall);
                break;
            }
        }
        // sliding down a wall faces the wall
        let mut n = Ninja::new(TILE + RADIUS + 0.5, 20.0 * TILE);
        for _ in 0..30 {
            n.step(&level, &Controls { dir: -1, ..Default::default() });
        }
        assert_eq!(n.pose(), Pose::Slide);
        assert_eq!(n.facing, -1);
    }
}

#[cfg(test)]
mod playtest {
    //! Plays a level by machine through the real `Input` code with a simulated keyboard
    //! (presses plus OS-style repeats), the real physics and the real object world,
    //! searching for any way to finish it.
    use super::*;
    use crate::input::{Input, Mode};
    use crate::objects::WorldEvent;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
    use std::cmp::Reverse;
    use std::collections::{BinaryHeap, HashSet};
    use std::time::{Duration, Instant};

    #[derive(Clone)]
    struct Human {
        n: Ninja,
        input: Input,
        world: Objects,
        frame: u32,
        delay_frames: u32,
        dir_key: i8,
        dir_since: u32,
        jump_since: u32,
        last_dir_event: u32,
        last_jump_event: u32,
        switches: Vec<bool>,
    }

    const REPEAT_EVERY: u32 = 2; // about 33 ms

    fn press(i: &mut Input, code: KeyCode, kind: KeyEventKind, at: Instant) {
        i.on_key(KeyEvent::new_with_kind(code, KeyModifiers::NONE, kind), at);
    }

    /// One frame with the keys in the given state. Returns false if the ninja died.
    fn frame(h: &mut Human, level: &Level, want_dir: i8, want_jump: bool, base: Instant, exact: bool) -> bool {
        frame_ctl(h, level, want_dir, want_jump, base, exact).0
    }

    /// Same, also returning the controls the input code produced.
    fn frame_ctl(h: &mut Human, level: &Level, want_dir: i8, want_jump: bool, base: Instant, exact: bool) -> (bool, Controls) {
        let now = base + Duration::from_micros(h.frame as u64 * 16_667);
        h.frame += 1;
        if want_dir != h.dir_key {
            if exact && h.dir_key != 0 {
                press(&mut h.input, if h.dir_key < 0 { KeyCode::Left } else { KeyCode::Right }, KeyEventKind::Release, now);
            }
            h.dir_key = want_dir;
            h.dir_since = 0;
        }
        if h.dir_key != 0 {
            h.dir_since += 1;
            let code = if h.dir_key < 0 { KeyCode::Left } else { KeyCode::Right };
            let repeat = !exact && h.dir_since > h.delay_frames && (h.dir_since - h.delay_frames) % REPEAT_EVERY == 0;
            if h.dir_since == 1 || repeat {
                press(&mut h.input, code, KeyEventKind::Press, now);
                h.last_dir_event = h.frame;
            }
        }
        if want_jump {
            h.jump_since += 1;
            let repeat = !exact && h.jump_since > h.delay_frames && (h.jump_since - h.delay_frames) % REPEAT_EVERY == 0;
            if h.jump_since == 1 || repeat {
                press(&mut h.input, KeyCode::Char(' '), KeyEventKind::Press, now);
                h.last_jump_event = h.frame;
            }
        } else if h.jump_since != 0 {
            if exact {
                press(&mut h.input, KeyCode::Char(' '), KeyEventKind::Release, now);
            }
            h.jump_since = 0;
        }
        let c = h.input.controls(now, !h.n.grounded);
        let ev = h.n.step_in(level, &c, &mut h.world);
        let mut alive = !ev.iter().any(|e| matches!(e, Event::Died(_)));
        if h.world.update(level, &mut h.n, h.frame as u64).iter().any(|w| matches!(w, WorldEvent::Mine | WorldEvent::Killed(_))) {
            alive = false;
        }
        for (e, on) in level.exits.iter().zip(h.switches.iter_mut()) {
            *on |= (e.switch.0 - h.n.x).powi(2) + (e.switch.1 - h.n.y).powi(2) <= (RADIUS + 6.0).powi(2);
        }
        (alive, c)
    }

    fn finished(h: &Human, level: &Level) -> bool {
        level
            .exits
            .iter()
            .zip(&h.switches)
            .any(|(e, &on)| on && (e.door.0 - h.n.x).powi(2) + (e.door.1 - h.n.y).powi(2) <= (RADIUS + 12.0).powi(2))
    }

    type Key = ((i32, i32, i32, i32, bool, i8, bool, u32), (i8, u32, u32), u32);

    /// Merge equivalent situations: for each key only its phase matters.
    fn key(h: &Human, exact: bool) -> Key {
        let n = &h.n;
        let phase = |down: bool, since: u32, last: u32| -> u32 {
            if exact {
                return down as u32;
            }
            if down {
                if since <= 13 {
                    0
                } else if since < h.delay_frames + 3 {
                    1
                } else {
                    2
                }
            } else {
                3 + ((h.frame - last).min(24) / 8)
            }
        };
        let flags = h.switches.iter().fold(0u32, |a, &b| a * 2 + b as u32) * 64 + h.world.doors.iter().fold(0u32, |a, d| a * 2 + d.open as u32) % 64;
        (
            (
                (n.x / 4.0) as i32,
                (n.y / 4.0) as i32,
                (n.vx * 2.0).round() as i32,
                (n.vy * 2.0).round() as i32,
                n.grounded,
                n.wall,
                n.jumping,
                n.jump_timer / 6,
            ),
            (h.dir_key, phase(h.dir_key != 0, h.dir_since, h.last_dir_event), phase(h.jump_since > 0, h.jump_since, h.last_jump_event)),
            flags,
        )
    }

    fn start_state(level: &Level, exact: bool, delay_frames: u32) -> Human {
        Human {
            n: Ninja::new(level.spawn.0, level.spawn.1),
            input: Input::with_mode(if exact { Mode::Exact } else { Mode::Timing }),
            world: Objects::from_level(level),
            frame: 1000,
            delay_frames,
            dir_key: 0,
            dir_since: 0,
            jump_since: 0,
            last_dir_event: 0,
            last_jump_event: 0,
            switches: vec![false; level.exits.len()],
        }
    }

    /// Best-first search from `start`; keys are re-decided every 6 frames. `weight` scales
    /// the pull toward `target` (0 is plain breadth-first). Returns the keys and end state.
    #[allow(clippy::too_many_arguments)]
    fn search_from(
        level: &Level,
        start: Human,
        base: Instant,
        exact: bool,
        max_nodes: usize,
        weight: f32,
        target: (f32, f32),
        goal: &dyn Fn(&Human) -> bool,
    ) -> (Option<(Vec<(i8, bool)>, Human)>, usize) {
        let h = |s: &Human| ((s.n.x - target.0).abs() * 0.5 + (s.n.y - target.1).abs()) / 20.0 * weight;
        let mut states = vec![(start.clone(), 0u32)];
        let mut parents: Vec<(usize, (i8, bool))> = vec![(usize::MAX, (0, false))];
        let mut seen = HashSet::new();
        let mut heap = BinaryHeap::new();
        seen.insert(key(&start, exact));
        heap.push(Reverse(((h(&start) * 100.0) as i64, 0usize)));
        let mut count = 0;
        while let Some(Reverse((_, idx))) = heap.pop() {
            count += 1;
            if count > max_nodes {
                break;
            }
            let (s, depth) = states[idx].clone();
            if goal(&s) {
                let (mut path, mut i) = (Vec::new(), idx);
                while parents[i].0 != usize::MAX {
                    path.push(parents[i].1);
                    i = parents[i].0;
                }
                path.reverse();
                return (Some((path, s)), count);
            }
            if depth >= 420 {
                continue;
            }
            for want_dir in [-1i8, 0, 1] {
                for want_jump in [false, true] {
                    let mut t = s.clone();
                    let mut alive = true;
                    for _ in 0..6 {
                        if !frame(&mut t, level, want_dir, want_jump, base, exact) {
                            alive = false;
                            break;
                        }
                    }
                    if alive && seen.insert(key(&t, exact)) {
                        states.push((t.clone(), depth + 1));
                        parents.push((idx, (want_dir, want_jump)));
                        heap.push(Reverse((((depth as f32 + 1.0 + h(&t)) * 100.0) as i64, states.len() - 1)));
                    }
                }
            }
        }
        (None, count)
    }

    /// Whole level in two legs: breadth-first to the first exit's switch, greedy from there to
    /// its door (a few weights are tried; the greedy pull can get stuck under platforms).
    fn play_level(level: &Level, exact: bool, delay_frames: u32) -> (Option<Vec<(i8, bool)>>, usize) {
        let e = level.exits[0];
        let base = Instant::now();
        let mut total = 0;
        for w1 in [0.0f32, 0.5] {
            let (leg1, n1) = search_from(level, start_state(level, exact, delay_frames), base, exact, 2_500_000, w1, e.switch, &|s| s.switches[0]);
            total += n1;
            let Some((path, mid)) = leg1 else { continue };
            for w2 in [3.0f32, 1.0, 0.0] {
                let (leg2, n2) = search_from(level, mid.clone(), base, exact, 2_500_000, w2, e.door, &|s| finished(s, level));
                total += n2;
                if let Some((p2, _)) = leg2 {
                    let mut all = path.clone();
                    all.extend(p2);
                    return (Some(all), total);
                }
            }
        }
        (None, total)
    }

    fn summarise(path: &[(i8, bool)]) -> String {
        let jumps = path.windows(2).filter(|w| !w[0].1 && w[1].1).count() + path.first().is_some_and(|p| p.1) as usize;
        let turns = path.windows(2).filter(|w| w[0].0 != 0 && w[1].0 != 0 && w[0].0 != w[1].0).count();
        format!("{:.1} s, {} jump presses, {} direction reversals", path.len() as f32 * 6.0 / 60.0, jumps, turns)
    }

    /// A key schedule as text: two characters per 6 frames, direction (L, _, R) then jump (j or .).
    fn encode(path: &[(i8, bool)]) -> String {
        path.iter().map(|&(d, j)| format!("{}{}", ['L', '_', 'R'][(d + 1) as usize], if j { 'j' } else { '.' })).collect()
    }

    fn decode(s: &str) -> Vec<(i8, bool)> {
        s.as_bytes().chunks(2).map(|c| (match c[0] { b'L' => -1, b'R' => 1, _ => 0 }, c[1] == b'j')).collect()
    }

    /// Replay a schedule through the input code AND the real `Game`; returns whether the
    /// game declared the level complete.
    fn replay_in_game(level: &Level, schedule: &str, exact: bool, delay_frames: u32) -> bool {
        let mut game = crate::game::Game::new(vec![level.clone_level()], 0);
        let base = Instant::now();
        let mut h = start_state(level, exact, delay_frames);
        for (d, j) in decode(schedule) {
            for _ in 0..6 {
                let (_, c) = frame_ctl(&mut h, level, d, j, base, exact);
                game.update(&c);
                if game.complete {
                    return true;
                }
            }
        }
        game.complete
    }

    fn practice() -> Level {
        Level::parse(include_str!("../levels/001.txt"))
    }

    #[test]
    #[ignore]
    fn print_the_winning_routes() {
        let level = practice();
        for (name, exact, delay) in [("EXACT", true, 30u32), ("TIMING_500", false, 30)] {
            let (path, _) = play_level(&level, exact, delay);
            println!("const {name}: &str = \"{}\";", encode(&path.expect("a route")));
        }
    }

    // Routes found by `play_level` (regenerate with the ignored `print_the_winning_routes`
    // if the course or the physics is changed on purpose).
    const EXACT: &str = "R.R.R.R._jRjRjRjRjRjRjR.R.R.R.R.RjRjRjRjRjRjRjRjR.R.L.L.L.R.Rj_._jRjRjRjRjRjRjRj_._jRjRjRjRjRjRjRj_.RjRjRjRj_.RjRjRjRj_.Rj_j_j_jR.R.R.R.R.R.LjR.R.R.R.R.R.R.R.";
    const TIMING_500: &str = "R._.R._.R.LjR._.R._.R._.R._._.L.R._jR._.R._.R._.R._._.L.L.Rj_.LjR._.R._.R._.R._.R._jR._.R._._j_.Rj_.R._.R._jR._.R.L.LjL.R._jR._.R._.R._.R._.R.";

    #[test]
    fn the_practice_course_can_be_finished_with_exact_keys() {
        assert!(replay_in_game(&practice(), EXACT, true, 30), "exact route no longer finishes the course");
    }

    #[test]
    fn the_practice_course_can_be_finished_with_the_timing_keys_at_gnome_defaults() {
        assert!(replay_in_game(&practice(), TIMING_500, false, 30), "timing route (500 ms repeat delay) no longer finishes the course");
    }

    #[test]
    #[ignore]
    fn can_a_person_finish_it() {
        let level = practice();
        for (name, exact, delay) in [
            ("exact key events (kitty or Windows)", true, 30),
            ("timing input, 500 ms repeat delay (GNOME default)", false, 30),
            ("timing input, 200 ms repeat delay", false, 12),
        ] {
            let (path, nodes) = play_level(&level, exact, delay);
            match path {
                Some(p) => println!("{name}: FINISHABLE, {} ({nodes} states)", summarise(&p)),
                None => println!("{name}: not found ({nodes} states)"),
            }
        }
    }

    /// Real N levels whose objects are all implemented: can each be finished (exact keys)?
    #[test]
    #[ignore]
    fn the_supported_n_levels_can_be_finished() {
        let levels = crate::level::n_levels();
        let want = ["the motherlode", "cloud city", "the box, the box!", "shaft..s", "step lightly", "longjump", "hold it", "don't look down", "ditched", "get down", "descent"];
        for l in levels.iter().filter(|l| want.iter().any(|w| l.label.ends_with(w))) {
            let (path, nodes) = play_level(l, true, 30);
            match path {
                Some(p) => println!("{:28} FINISHABLE  {} ({nodes} states)", l.label, summarise(&p)),
                None => println!("{:28} not found ({nodes} states)", l.label),
            }
        }
    }
}

#[cfg(test)]
mod reference_numbers {
    use super::*;

    fn flat() -> Level {
        let mut rows = Vec::new();
        for y in 0..30 {
            rows.push(if y >= 28 { "#".repeat(80) } else { format!("#{}#", " ".repeat(78)) });
        }
        Level::parse(&rows.join("\n"))
    }

    /// (apex rise in tiles, horizontal distance covered by the whole jump in tiles)
    fn jump(run_up: usize, hold_frames: u32) -> (f32, f32) {
        let level = flat();
        let floor = 28.0 * TILE - RADIUS;
        let mut n = Ninja::new(3.0 * TILE, floor);
        for _ in 0..90 {
            n.step(&level, &Controls::default());
        }
        for _ in 0..run_up {
            n.step(&level, &Controls { dir: 1, ..Default::default() });
        }
        let (x0, mut top) = (n.x, n.y);
        for f in 0..300u32 {
            let c = Controls { dir: 1, jump_held: f < hold_frames, jump_pressed: f == 0 };
            n.step(&level, &c);
            top = top.min(n.y);
            if f > 5 && n.grounded {
                break;
            }
        }
        ((floor - top) / TILE, (n.x - x0) / TILE)
    }

    #[test]
    fn print_reference_numbers() {
        for (name, run, hold) in [("standing jump, full hold", 0, 45), ("standing jump, tap (13 frames)", 0, 13), ("running jump (1 s run-up), full hold", 60, 45), ("running jump, tap", 60, 13)] {
            let (rise, dist) = jump(run, hold);
            println!("{name}: rises {rise:.2} tiles, travels {dist:.2} tiles");
        }
    }
}

#[cfg(test)]
mod direct_check {
    use super::*;

    /// Lands and stands on platform A (row 19, columns 10 to 15)?
    fn on_platform(n: &Ninja) -> bool {
        n.grounded && (n.y - (19.0 * TILE - RADIUS)).abs() < 1.5 && n.x > 10.0 * TILE && n.x < 16.0 * TILE
    }

    #[test]
    fn can_a_simple_schedule_land_on_the_switch_platform() {
        let level = Level::parse(include_str!("../levels/001.txt"));
        let mut wins = Vec::new();
        let mut tried = 0;
        for run1 in (0..120).step_by(2) {
            for run2 in (0..80).step_by(4) {
                for dir2 in [1i8, 0] {
                    tried += 1;
                    let mut n = Ninja::new(level.spawn.0, level.spawn.1);
                    let mut ok = false;
                    // run right, jump onto the step, wait to settle, run a little, jump again
                    for f in 0..500usize {
                        let j1 = f == run1;
                        let j2 = f == run1 + 70 + run2;
                        let c = Controls {
                            dir: if f < run1 + 70 { 1 } else { dir2 },
                            jump_held: (f >= run1 && f < run1 + 45) || (f >= run1 + 70 + run2 && f < run1 + 115 + run2),
                            jump_pressed: j1 || j2,
                        };
                        n.step(&level, &c);
                        if on_platform(&n) {
                            ok = true;
                            break;
                        }
                    }
                    if ok {
                        wins.push((run1, run2, dir2));
                    }
                }
            }
        }
        println!("{} of {tried} two-jump schedules end standing on the switch platform; first: {:?}", wins.len(), &wins[..wins.len().min(4)]);
        assert!(wins.len() * 10 > tried, "the switch platform should be easy to reach: only {} of {tried} schedules did", wins.len());
    }
}
