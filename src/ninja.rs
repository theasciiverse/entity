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
    /// What the feet are doing, for the animation (see `pose`).
    ground: Ground,
    /// Distance run along the floor since the run started, drives the run cycle.
    run_dist: f32,
    /// Ticks since the ninja came to a stand, drives the landing settle.
    stand_ticks: u32,
    /// The drawing's rotation in radians (clockwise on screen): the floor's slope on the
    /// ground, easing back to upright in the air, as in the original.
    pub tilt: f32,
    pub jumping: bool,
    pub jump_timer: u32,
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
                let (pw, qw) = ((ox + p.0 * TILE, oy + p.1 * TILE), (ox + q.0 * TILE, oy + q.1 * TILE));
                let c = closest(pw, qw, x, y);
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
                    // The end of a full side whose surface carries straight on into the next
                    // tile is a seam, not a corner: the next tile's own side gives the contact.
                    // Without this a flat floor tilts the floor normal at every seam.
                    // Only when the centre lies past the end, so a centre exactly over the seam
                    // still touches both tiles.
                    let beyond = match side {
                        TOP | BOTTOM => (x < ox - 1e-3).then_some(-1).or((x > ox + TILE + 1e-3).then_some(1)),
                        _ => (y < oy - 1e-3).then_some(-1).or((y > oy + TILE + 1e-3).then_some(1)),
                    };
                    if let Some(dir) = beyond {
                        let (ax, ay) = match side {
                            TOP | BOTTOM => (tx + dir, ty),
                            _ => (tx, ty + dir),
                        };
                        let ac = level.code(ax, ay);
                        if ac != 0 && shape(ac).side_full[side] {
                            continue;
                        }
                    }
                }
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

/// The original's animation (frames of N's ninja clip, see `sprite`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pose {
    /// Settling after landing or stopping, frames 0 to 10 (10 is the rest pose).
    Stand(usize),
    Skid,
    /// Run cycle frame, 0 to 71.
    Run(usize),
    /// In the air, 0 (rising fast) to 9 (the top) to 18 (falling fast).
    Air(usize),
    Slide,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Ground {
    Air,
    Stand,
    Skid,
    Run,
}

/// The original's run cycle: 72 frames, one per 0.9 px along the floor.
const RUN_FRAMES: usize = 72;
const RUN_FRAME_PX: f32 = 0.9;
/// The stand settle plays at the movie's 120 fps: two frames per 60 Hz tick.
const STAND_FRAMES_PER_TICK: u32 = 2;
/// In the air the drawing loses 10% of its tilt per 40 Hz tick.
const TILT_KEEP: f32 = 0.9322; // 0.9^(2/3)

/// `ENTITY_PHYSICS=classic` disables the original-N rules.
fn n_rules_default() -> bool {
    use std::sync::OnceLock;
    static V: OnceLock<bool> = OnceLock::new();
    *V.get_or_init(|| std::env::var("ENTITY_PHYSICS").map_or(true, |v| v != "classic"))
}

impl Ninja {
    /// The original's ground states, which pick the animation: running while a key pushes
    /// along the motion, skidding while braking, standing once nearly stopped.
    fn animate(&mut self, c: &Controls) {
        let state = if !self.grounded {
            Ground::Air
        } else {
            let (nx, ny) = self.floor_n;
            let along = (self.vx * -ny + self.vy * nx).abs();
            if c.dir != 0 && (c.dir as f32) * self.vx >= 0.0 {
                if self.ground != Ground::Run {
                    self.run_dist = 0.0;
                }
                self.run_dist += along;
                Ground::Run
            } else if along >= STAND_SPEED {
                Ground::Skid
            } else {
                Ground::Stand
            }
        };
        self.stand_ticks = if state == Ground::Stand && self.ground == Ground::Stand { self.stand_ticks + 1 } else { 0 };
        self.ground = state;
        if self.grounded {
            let (nx, ny) = self.floor_n;
            self.tilt = ny.atan2(nx) + std::f32::consts::FRAC_PI_2;
        } else if self.jumping || self.sliding {
            self.tilt = 0.0;
        } else {
            self.tilt *= TILT_KEEP;
        }
    }

    pub fn pose(&self) -> Pose {
        match self.ground {
            Ground::Stand => Pose::Stand(((self.stand_ticks * STAND_FRAMES_PER_TICK) as usize).min(10)),
            Ground::Skid => Pose::Skid,
            Ground::Run => Pose::Run((self.run_dist / RUN_FRAME_PX) as usize % RUN_FRAMES),
            Ground::Air if (self.sliding || (!self.n_rules && self.wall != 0)) && self.vy > 0.0 => Pose::Slide,
            Ground::Air => {
                // the original's RenderInAir, with its vertical speed in px per 40 Hz tick
                let vy = self.vy * 1.5;
                let r = if vy < 0.0 { vy.max(-1.0) } else { (vy / 2.5).sqrt().min(1.0) };
                Pose::Air((9 + (r * 9.0).floor() as i32).clamp(0, 18) as usize)
            }
        }
    }

    pub fn new(x: f32, y: f32) -> Ninja {
        Ninja { x, y, vx: 0.0, vy: 0.0, grounded: false, wall: 0, facing: 1, ground: Ground::Air, run_dist: 0.0, stand_ticks: 0, tilt: 0.0, jumping: false, jump_timer: 0, jump_buffer: 0, ground_coyote: 0, wall_coyote: 0, n_rules: n_rules_default(), sliding: false, floor_n: (0.0, -1.0), wall_memory: 0 }
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
        self.animate(c);

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
    fn every_campaign_level_loads_with_a_sane_spawn() {
        let levels = crate::level::campaign();
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
        let level = Level::parse(&(0..12).map(|y| if y >= 10 { "#".repeat(40) } else { format!("#{}#", " ".repeat(38)) }).collect::<Vec<_>>().join("\n"));
        let floor = 10.0 * TILE - RADIUS;
        let mut n = Ninja::new(5.0 * TILE, floor);
        for _ in 0..30 {
            n.step(&level, &Controls::default());
        }
        assert_eq!(n.pose(), Pose::Stand(10), "settled into the rest pose");
        // the run cycle starts at its first frame and moves one frame per 0.9 px
        n.step(&level, &Controls { dir: 1, ..Default::default() });
        assert!(matches!(n.pose(), Pose::Run(0)), "{:?}", n.pose());
        let x0 = n.x;
        for _ in 0..30 {
            n.step(&level, &Controls { dir: 1, ..Default::default() });
        }
        let Pose::Run(f) = n.pose() else { panic!("{:?}", n.pose()) };
        assert!((f as f32 - (n.x - x0) / 0.9).abs() < 2.0, "frame {f} after {} px", n.x - x0);
        // letting go skids, then settles from frame 0 to the rest pose
        n.step(&level, &Controls::default());
        assert_eq!(n.pose(), Pose::Skid);
        for _ in 0..200 {
            n.step(&level, &Controls::default());
            if n.pose() != Pose::Skid {
                break;
            }
        }
        assert!(matches!(n.pose(), Pose::Stand(0..=2)), "{:?}", n.pose());
        assert!(n.tilt.abs() < 1e-4, "upright on a flat floor, seams included: {} {:?}", n.tilt, n.floor_n);
        // rising, the top, then falling
        n.step(&level, &Controls { jump_pressed: true, jump_held: true, dir: 0 });
        let Pose::Air(rising) = n.pose() else { panic!("{:?}", n.pose()) };
        assert!(rising < 9, "{rising}");
        for _ in 0..200 {
            n.step(&level, &Controls::default());
            if n.vy > 0.5 && !n.grounded {
                break;
            }
        }
        let Pose::Air(falling) = n.pose() else { panic!("{:?}", n.pose()) };
        assert!(falling > 9, "{falling}");
        // sliding down a wall faces the wall
        let mut n = Ninja::new(TILE + RADIUS + 0.5, 3.0 * TILE);
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
    use crate::bot::*;
    use crate::demo::decode;
    use std::time::Instant;

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

    /// A route through any level for `entity --demo <level>`: `ENTITY_ROUTE_LEVEL=lastdoor
    /// cargo test --release print_a_route -- --ignored --nocapture` (exact keys).
    #[test]
    #[ignore]
    fn print_a_route() {
        let want = std::env::var("ENTITY_ROUTE_LEVEL").unwrap_or_else(|_| "lastdoor".into());
        let levels = crate::level::campaign();
        let level = levels.iter().find(|l| l.label.replace(' ', "").contains(&want)).expect("no such level");
        // optional waypoints "x,y;x,y|x,y" (px): legs before the switch | legs before the door
        let (path, nodes) = match std::env::var("ENTITY_ROUTE_WAYPOINTS") {
            Ok(w) => waypoint_route(level, &w),
            Err(_) => play_level(level, true, 30),
        };
        match path {
            Some(p) => println!("{}: {} s, {nodes} states\nconst ROUTE: &str = \"{}\";", level.label, p.len() as f32 / 10.0, encode(&p)),
            None => println!("{}: no route found in {nodes} states", level.label),
        }
    }

    // Routes found by `play_level` (regenerate with the ignored `print_the_winning_routes`
    // if the course or the physics is changed on purpose). The exact route lives in demo.rs,
    // which plays it for `entity --demo`.
    const EXACT: &str = crate::demo::PRACTICE;
    const TIMING_500: &str = "R._.R._.Rj_.R._.R._.R._._.R._.R._jR._.R._.R._.R._.R._.R.L.Rj_.R._.R._jR._.R._.R._.Rj_.R._.R._jR._.R._.R._.Rj_.R._.R._._.R._.R._.R._._.Rj_.Rj_.R._.R._.R._.";

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

#[cfg(test)]
mod peek {
    use super::*;
    /// Solid map of a level region at 4 px per character: `ENTITY_PEEK=label,c0,r0,c1,r1`
    /// (playfield columns and printed rows, 1-based rows as in the tile dump).
    #[test]
    #[ignore]
    fn peek_region() {
        let spec = std::env::var("ENTITY_PEEK").unwrap_or_else(|_| "lastdoor,0,12,6,18".into());
        let v: Vec<&str> = spec.split(',').collect();
        let levels = crate::level::campaign();
        let level = levels.iter().find(|l| l.label.replace(' ', "").contains(v[0])).unwrap();
        let n = |i: usize| v[i].parse::<f32>().unwrap();
        let (x0, y0, x1, y1) = ((n(1) + 1.0) * TILE, n(2) * TILE, (n(3) + 2.0) * TILE, (n(4) + 1.0) * TILE);
        let step: f32 = std::env::var("ENTITY_PEEK_STEP").ok().and_then(|v| v.parse().ok()).unwrap_or(4.0);
        let mut y = y0 + 2.0;
        while y < y1 {
            let mut line = String::new();
            let mut x = x0 + 2.0;
            while x < x1 {
                let on_grid = ((x - 2.0) % TILE) < step - 3.0 && ((y - 2.0) % TILE) < step - 3.0;
                line.push(if level.point_solid(x, y) { '#' } else if on_grid { '+' } else { ' ' });
                x += step;
            }
            println!("{line}");
            y += step;
        }
    }
}
