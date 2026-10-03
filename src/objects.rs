//! Level objects with physics: mines, launch pads, doors, one-way platforms and bounce
//! blocks. Behaviour is ported from the original N v1.2 bytecode (see CLAUDE.md,
//! "Original N physics"); the original ticks at 40 Hz, so rates here are converted to
//! Entity's 60 Hz tick (velocity x 2/3, acceleration x 4/9).

use crate::level::{Level, TILE};
use crate::drones::{Cause, Drone};
use crate::hazards::{FloorGuard, Launcher, Thwomp, Turret};
use crate::ninja::{Ninja, RADIUS};

/// A collision segment. `n` is the facing normal for one-sided segments, (0, 0) for two-sided.
#[derive(Clone, Copy, Debug)]
pub struct Seg {
    pub a: (f32, f32),
    pub b: (f32, f32),
    pub n: (f32, f32),
}

/// Converts the original's per-40-Hz-tick velocity to per-60-Hz-tick.
const V: f32 = 2.0 / 3.0;

#[derive(Clone, Debug)]
pub struct Mine {
    pub x: f32,
    pub y: f32,
    pub exploded: bool,
}
/// Mine radius 12 * 0.3333 = 4.
const MINE_R: f32 = 4.0;

#[derive(Clone, Debug)]
pub struct Pad {
    pub x: f32,
    pub y: f32,
    pub nx: f32,
    pub ny: f32,
}
const PAD_R: f32 = 6.0;
/// Launch strength: 12 * 0.428571 = 5.1429 per 40 Hz tick.
const PAD_STRENGTH: f32 = 5.1429 * V;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DoorKind {
    /// Opens as you approach, closes a moment after you leave.
    Trek,
    /// Opened for good by touching its separate switch.
    Locked,
    /// Starts open; touching its trigger slams it shut for good.
    Trap,
}

#[derive(Clone, Debug)]
pub struct Door {
    pub kind: DoorKind,
    /// Trigger position and radius (the door itself for trek doors).
    pub trigger: (f32, f32),
    pub trigger_r: f32,
    /// The barrier on a tile edge while closed.
    pub seg: Seg,
    /// The two tile cells the barrier separates (drones cannot cross it while closed).
    pub edge: ((i32, i32), (i32, i32)),
    pub open: bool,
    /// Trap doors lose their trigger once sprung.
    pub spent: bool,
    timer: u32,
}
/// Frames a trek door stays open after the ninja leaves (original: 5 ticks at 40 Hz).
const TREK_CLOSE_FRAMES: u32 = 8;

#[derive(Clone, Debug)]
pub struct OneWay {
    pub x: f32,
    pub y: f32,
    /// Facing direction: the side that is solid.
    pub dir: (f32, f32),
}

#[derive(Clone, Debug)]
pub struct Block {
    pub anchor: (f32, f32),
    pub pos: (f32, f32),
    pub old: (f32, f32),
    asleep: bool,
    sleep_timer: u32,
}
/// Half size 12 * 0.8.
pub const BLOCK_HALF: f32 = 9.6;
const BLOCK_MASS: f32 = 0.2;
/// Spring stiffness 0.05 per 40 Hz tick -> per 60 Hz tick (x 4/9).
const BLOCK_STIFF: f32 = 0.05 * V * V;
const BLOCK_DRAG: f32 = 0.9933;
const BLOCK_SLEEP_FRAMES: u32 = 60;

#[derive(Clone, Debug, Default)]
pub struct Objects {
    pub mines: Vec<Mine>,
    pub pads: Vec<Pad>,
    pub doors: Vec<Door>,
    pub oneways: Vec<OneWay>,
    pub blocks: Vec<Block>,
    pub drones: Vec<Drone>,
    pub turrets: Vec<Turret>,
    pub guards: Vec<FloorGuard>,
    pub thwomps: Vec<Thwomp>,
    pub launchers: Vec<Launcher>,
    /// Events raised during collision (a thwomp's deadly face), delivered by the next `update`.
    pending: Vec<WorldEvent>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WorldEvent {
    /// A mine went off on the ninja.
    Mine,
    /// Something other than a mine killed the ninja.
    Killed(Cause),
    Launched,
    DoorOpened,
    DoorClosed,
}

impl Objects {
    pub fn none() -> Objects {
        Objects::default()
    }

    /// Build from the level's raw object records (original parameter layouts).
    pub fn from_level(level: &Level) -> Objects {
        let mut w = Objects::default();
        for o in &level.objects {
            let p = &o.params;
            match o.kind.as_str() {
                "mine" => w.mines.push(Mine { x: o.x, y: o.y, exploded: false }),
                "launchpad" if p.len() >= 4 => w.pads.push(Pad { x: p[0], y: p[1], nx: p[2], ny: p[3] }),
                "onewayplatform" if p.len() >= 3 => {
                    // AI_DIR_R 0, D 1, L 2, U 3
                    let dir = match p[2] as i32 {
                        0 => (1.0, 0.0),
                        1 => (0.0, 1.0),
                        2 => (-1.0, 0.0),
                        _ => (0.0, -1.0),
                    };
                    w.oneways.push(OneWay { x: p[0], y: p[1], dir });
                }
                "drone" if p.len() >= 6 => w.drones.push(Drone::from_params(p)),
                "turret" => w.turrets.push(Turret::new(o.x, o.y)),
                "hominglauncher" => w.launchers.push(Launcher::new(o.x, o.y)),
                "floorguard" if p.len() >= 3 => w.guards.push(FloorGuard::new(level, p[0], p[1], p[2])),
                "thwomp" if p.len() >= 3 => w.thwomps.push(Thwomp::new(level, p[0], p[1], p[2] as i32)),
                "bounceblock" => w.blocks.push(Block {
                    anchor: (o.x, o.y),
                    pos: (o.x, o.y),
                    old: (o.x, o.y),
                    asleep: true,
                    sleep_timer: 0,
                }),
                "testdoor" if p.len() >= 9 => {
                    // [trigger x, trigger y, vertical, trap, I, J, locked, dI, dJ]; the barrier lies
                    // on the right edge of tile (I, J), or its bottom edge when vertical is 1
                    let (i, j) = ((p[4] + p[7]) * TILE, (p[5] + p[8]) * TILE);
                    let (a, b) = if p[2] as i32 == 1 { ((i, j + TILE), (i + TILE, j + TILE)) } else { ((i + TILE, j), (i + TILE, j + TILE)) };
                    let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
                    let (kind, trigger, trigger_r, open) = if p[6] != 0.0 {
                        (DoorKind::Locked, (p[0], p[1]), 12.0 * 0.416_667, false)
                    } else if p[3] != 0.0 {
                        (DoorKind::Trap, (p[0], p[1]), 12.0 * 0.416_667, true)
                    } else {
                        (DoorKind::Trek, mid, 12.0 * 0.833_333, false)
                    };
                    let cell = ((p[4] + p[7]) as i32, (p[5] + p[8]) as i32);
                    let edge = if p[2] as i32 == 1 { (cell, (cell.0, cell.1 + 1)) } else { (cell, (cell.0 + 1, cell.1)) };
                    w.doors.push(Door { kind, trigger, trigger_r, seg: Seg { a, b, n: (0.0, 0.0) }, edge, open, spent: false, timer: 0 });
                }
                _ => {}
            }
        }
        w
    }

    /// Segments the ninja can collide with this tick. `prev` is its centre before it moved.
    pub fn colliders(&self, n: &Ninja, prev: (f32, f32)) -> Vec<Seg> {
        let mut out = Vec::new();
        for d in &self.doors {
            if !d.open {
                out.push(d.seg);
            }
        }
        for p in &self.oneways {
            let (dx, dy) = (n.x - p.x, n.y - p.y);
            if dx.abs() >= TILE / 2.0 + RADIUS || dy.abs() >= TILE / 2.0 + RADIUS {
                continue;
            }
            // only blocks motion against the facing side, and only for a ninja that was
            // outside the face before this tick (the original's entry test)
            let (vx, vy) = (n.x - prev.0, n.y - prev.1);
            let along_v = if p.dir.0 == 0.0 { vy * p.dir.1 } else { vx * p.dir.0 };
            if along_v > 0.0 {
                continue;
            }
            let face = (p.x + p.dir.0 * TILE / 2.0, p.y + p.dir.1 * TILE / 2.0);
            let before = if p.dir.0 == 0.0 { (prev.1 - p.dir.1 * RADIUS) - face.1 } else { (prev.0 - p.dir.0 * RADIUS) - face.0 };
            let side = if p.dir.0 == 0.0 { before * p.dir.1 } else { before * p.dir.0 };
            if side < -0.5 {
                continue;
            }
            let (h, k) = (TILE / 2.0, TILE / 2.0);
            let (a, b) = if p.dir.0 == 0.0 {
                ((p.x - h, face.1), (p.x + h, face.1))
            } else {
                ((face.0, p.y - k), (face.0, p.y + k))
            };
            out.push(Seg { a, b, n: p.dir });
        }
        out
    }

    /// Bounce blocks push back: the block takes 80% of an overlap and the ninja 20%
    /// (the original resolves the ninja as a box). Returns contact normals for the ninja.
    pub fn collide_blocks(&mut self, n: &mut Ninja) -> Vec<(f32, f32)> {
        let mut normals = Vec::new();
        for b in &mut self.blocks {
            let (dy, dx) = (n.y - b.pos.1, n.x - b.pos.0);
            let py = (BLOCK_HALF + RADIUS) - dy.abs();
            let px = (BLOCK_HALF + RADIUS) - dx.abs();
            if py <= 0.0 || px <= 0.0 {
                continue;
            }
            if py < px {
                let (ny, py) = if dy < 0.0 { (-1.0, -py) } else { (1.0, py) };
                b.pos.1 -= (1.0 - BLOCK_MASS) * py;
                n.y += BLOCK_MASS * py;
                normals.push((0.0, ny));
            } else {
                let (nx, px) = if dx < 0.0 { (-1.0, -px) } else { (1.0, px) };
                b.pos.0 -= (1.0 - BLOCK_MASS) * px;
                n.x += BLOCK_MASS * px;
                normals.push((nx, 0.0));
            }
            b.sleep_timer = 0;
            b.asleep = false;
        }
        // thwomps: solid on three sides, deadly on the front
        for t in &self.thwomps {
            match t.collide(n) {
                Some(Ok((push, normal))) => {
                    n.x += push.0;
                    n.y += push.1;
                    normals.push(normal);
                }
                Some(Err(())) => self.pending.push(WorldEvent::Killed(Cause::Thwomp)),
                None => {}
            }
        }
        normals
    }

    /// Advance the springs of awake blocks.
    pub fn tick_blocks(&mut self) {
        for b in &mut self.blocks {
            if b.asleep {
                continue;
            }
            let (ox, oy) = b.old;
            b.old = b.pos;
            b.pos.0 += BLOCK_DRAG * (b.pos.0 - ox);
            b.pos.1 += BLOCK_DRAG * (b.pos.1 - oy);
            b.pos.0 += (b.anchor.0 - b.pos.0) * BLOCK_STIFF;
            b.pos.1 += (b.anchor.1 - b.pos.1) * BLOCK_STIFF;
            b.sleep_timer += 1;
            if b.sleep_timer > BLOCK_SLEEP_FRAMES {
                // the original freezes a sleeping block wherever it is; snapping it to its
                // anchor is a small deviation so blocks always come to rest properly
                b.asleep = true;
                b.pos = b.anchor;
                b.old = b.anchor;
            }
        }
    }

    /// Triggers that act on the ninja after it has moved: mines, launch pads, door switches,
    /// and the drones. `time` is a running frame count.
    pub fn update(&mut self, level: &Level, n: &mut Ninja, time: u64) -> Vec<WorldEvent> {
        let mut ev = std::mem::take(&mut self.pending);
        for t in &mut self.thwomps {
            t.update(n);
        }
        for t in &mut self.turrets {
            if let Some(c) = t.update(level, n, time) {
                ev.push(WorldEvent::Killed(c));
            }
        }
        for g in &mut self.guards {
            if let Some(c) = g.update(n) {
                ev.push(WorldEvent::Killed(c));
            }
        }
        for l in &mut self.launchers {
            if let Some(c) = l.update(level, n) {
                ev.push(WorldEvent::Killed(c));
            }
        }
        for d in &mut self.drones {
            if let Some(c) = d.update(level, &self.doors, n, time) {
                ev.push(WorldEvent::Killed(c));
            }
        }
        let (px, py) = (n.x, n.y);
        let dist = |p: (f32, f32)| ((p.0 - px).powi(2) + (p.1 - py).powi(2)).sqrt();
        for m in &mut self.mines {
            if !m.exploded && dist((m.x, m.y)) < MINE_R + RADIUS {
                m.exploded = true;
                ev.push(WorldEvent::Mine);
            }
        }
        for p in &self.pads {
            if dist((p.x, p.y)) < PAD_R + RADIUS {
                // the ninja must not be more than a radius in front of the pad
                let front = (px - p.x) * p.nx + (py - p.y) * p.ny;
                if front <= RADIUS {
                    let mult = if p.ny < 0.0 { 1.0 + p.ny.abs() } else { 1.0 };
                    n.launch(p.nx * PAD_STRENGTH, p.ny * PAD_STRENGTH * mult);
                    ev.push(WorldEvent::Launched);
                }
            }
        }
        for d in &mut self.doors {
            let near = dist(d.trigger) < d.trigger_r + RADIUS;
            match d.kind {
                DoorKind::Trek => {
                    if near {
                        d.timer = 0;
                        if !d.open {
                            d.open = true;
                            ev.push(WorldEvent::DoorOpened);
                        }
                    } else if d.open {
                        d.timer += 1;
                        if d.timer > TREK_CLOSE_FRAMES {
                            d.open = false;
                            ev.push(WorldEvent::DoorClosed);
                        }
                    }
                }
                DoorKind::Locked => {
                    if near && !d.open {
                        d.open = true;
                        ev.push(WorldEvent::DoorOpened);
                    }
                }
                DoorKind::Trap => {
                    if near && !d.spent {
                        d.open = false;
                        d.spent = true;
                        ev.push(WorldEvent::DoorClosed);
                    }
                }
            }
        }
        ev
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level_with(objects: &str) -> Level {
        // a flat room; objects are added through the JSON-free path below
        let mut rows = Vec::new();
        for y in 0..30 {
            rows.push(if y >= 28 { "#".repeat(60) } else { format!("#{}#", " ".repeat(58)) });
        }
        let mut l = Level::parse(&rows.join("\n"));
        l.add_test_objects(objects);
        l
    }

    #[test]
    fn mine_kills_on_touch_once() {
        let l = level_with("mine 300 600");
        let mut w = Objects::from_level(&l);
        let mut n = Ninja::new(300.0, 590.0);
        assert_eq!(w.update(&l, &mut n, 0), vec![WorldEvent::Mine]);
        assert!(w.update(&l, &mut n, 0).is_empty(), "an exploded mine is spent");
        let mut far = Ninja::new(300.0, 500.0);
        assert!(Objects::from_level(&l).update(&l, &mut far, 0).is_empty());
    }

    #[test]
    fn launch_pad_sets_velocity_along_its_normal_with_an_upward_bonus() {
        let l = level_with("launchpad 300 600 0 -1");
        let mut w = Objects::from_level(&l);
        let mut n = Ninja::new(300.0, 590.0);
        n.vx = 1.0;
        n.vy = 0.5;
        assert_eq!(w.update(&l, &mut n, 0), vec![WorldEvent::Launched]);
        // up pad: velocity is absolute, 5.1429 * 2 (up bonus) per 40 Hz tick = 6.86 per 60 Hz tick
        assert!((n.vy + 6.857).abs() < 0.02 && n.vx.abs() < 1e-4, "v = ({}, {})", n.vx, n.vy);
        // a side pad has no bonus
        let l = level_with("launchpad 300 600 1 0");
        let mut n = Ninja::new(290.0, 600.0);
        Objects::from_level(&l).update(&l, &mut n, 0);
        assert!((n.vx - 3.4286).abs() < 0.01, "vx = {}", n.vx);
    }

    #[test]
    fn locked_door_blocks_until_its_switch_is_touched() {
        // door on the right edge of tile (20, 27): x = 21 * 24 = 504, between y 648 and 672
        let l = level_with("door 100 662 0 0 20 27 1 0 0");
        let mut w = Objects::from_level(&l);
        assert_eq!(w.doors[0].kind, DoorKind::Locked);
        let mut n = Ninja::new(470.0, 28.0 * TILE - RADIUS);
        // run into it
        let level = &l;
        for _ in 0..200 {
            let extra = w.colliders(&n, (n.x, n.y));
            let _ = extra;
            n.step_in(level, &crate::ninja::Controls { dir: 1, ..Default::default() }, &mut w);
        }
        assert!(n.x < 504.0 - RADIUS + 1.0, "the closed door stops the ninja: x = {}", n.x);
        // touch the switch: the door opens for good
        let mut near = Ninja::new(100.0, 662.0);
        assert_eq!(w.update(&l, &mut near, 0), vec![WorldEvent::DoorOpened]);
        for _ in 0..200 {
            n.step_in(level, &crate::ninja::Controls { dir: 1, ..Default::default() }, &mut w);
        }
        assert!(n.x > 520.0, "an open door lets the ninja through: x = {}", n.x);
    }

    #[test]
    fn trap_door_starts_open_and_slams_shut() {
        let l = level_with("door 400 612 0 1 20 25 0 0 0");
        let mut w = Objects::from_level(&l);
        assert_eq!(w.doors[0].kind, DoorKind::Trap);
        assert!(w.doors[0].open);
        let mut n = Ninja::new(400.0, 612.0);
        assert_eq!(w.update(&l, &mut n, 0), vec![WorldEvent::DoorClosed]);
        assert!(!w.doors[0].open);
        assert!(w.update(&l, &mut n, 0).is_empty(), "it only springs once");
    }

    #[test]
    fn trek_door_opens_near_and_closes_after() {
        let l = level_with("door 0 0 0 0 20 25 0 0 0");
        let mut w = Objects::from_level(&l);
        let mid = w.doors[0].trigger;
        let mut n = Ninja::new(mid.0 - 15.0, mid.1);
        assert_eq!(w.update(&l, &mut n, 0), vec![WorldEvent::DoorOpened]);
        let mut away = Ninja::new(mid.0 - 100.0, mid.1);
        let mut closed = false;
        for _ in 0..30 {
            if w.update(&l, &mut away, 0).contains(&WorldEvent::DoorClosed) {
                closed = true;
                break;
            }
        }
        assert!(closed, "a trek door closes once the ninja has left");
    }

    #[test]
    fn one_way_platform_can_be_jumped_up_through_but_stood_on() {
        // an up-facing platform: centre (252, 630), face at y = 618
        let l = level_with("oneway 252 630 3");
        let mut w = Objects::from_level(&l);
        // dropped from above it lands and rests
        let mut n = Ninja::new(252.0, 540.0);
        for _ in 0..120 {
            n.step_in(&l, &crate::ninja::Controls::default(), &mut w);
        }
        assert!(n.grounded && (n.y - (630.0 - 12.0 - RADIUS)).abs() < 1.5, "rests on top: y = {}", n.y);
        // from below, a held jump passes through
        let mut n = Ninja::new(252.0, 28.0 * TILE - RADIUS);
        let mut top = n.y;
        for f in 0..90 {
            n.step_in(&l, &crate::ninja::Controls { jump_held: true, jump_pressed: f == 0, dir: 0 }, &mut w);
            top = top.min(n.y);
        }
        assert!(top < 630.0 - 12.0 - RADIUS - 5.0, "jumped through to above the platform: top y = {top}");
    }

    #[test]
    fn bounce_block_gives_way_and_springs_back() {
        let l = level_with("bounceblock 300 400");
        let mut w = Objects::from_level(&l);
        let mut n = Ninja::new(300.0, 400.0 - BLOCK_HALF - RADIUS + 6.0);
        let before = w.blocks[0].pos;
        for _ in 0..20 {
            w.collide_blocks(&mut n);
            w.tick_blocks();
        }
        assert!(w.blocks[0].pos.1 > before.1 + 1.0, "pushed down: {:?}", w.blocks[0].pos);
        let mut away = Ninja::new(50.0, 100.0);
        for _ in 0..400 {
            w.collide_blocks(&mut away);
            w.tick_blocks();
        }
        assert!((w.blocks[0].pos.1 - 400.0).abs() < 0.5, "springs back to its anchor: {:?}", w.blocks[0].pos);
    }

    #[test]
    fn a_launch_pad_lifts_the_ninja_as_high_as_the_original_does() {
        // original (40 Hz): vy = -5.1429 * 2 per tick, g 0.15, drag 0.99 -> apex rise computed in
        // the original's own units below and compared in tiles
        let (mut y, mut v, mut apex) = (0.0f32, -5.1429f32 * 2.0, 0.0f32);
        for _ in 0..400 {
            v = v * 0.99 + 0.15;
            y += v;
            apex = apex.min(y);
            if v > 0.0 {
                break;
            }
        }
        let original = -apex;
        let l = level_with("launchpad 300 640 0 -1");
        let mut w = Objects::from_level(&l);
        let mut n = Ninja::new(300.0, 640.0);
        w.update(&l, &mut n, 0);
        let start = n.y;
        let mut top = n.y;
        for _ in 0..400 {
            n.step(&l, &crate::ninja::Controls::default());
            top = top.min(n.y);
            if n.vy > 0.0 {
                break;
            }
        }
        let mine = start - top;
        assert!((mine - original).abs() < 12.0, "launch rise {mine:.0} px against the original's {original:.0} px");
    }
}
