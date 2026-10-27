//! The route bot: plays a level by machine through the real `Input` code with a simulated
//! keyboard (presses plus OS-style repeats), the real physics and the real object world,
//! searching for a way to finish it (see "Playtesting by machine" in CLAUDE.md). Used by
//! the playtest tests and by `entity --prove`, which stores what it finds in the level file.

use crate::input::{Input, Mode};
use crate::level::Level;
use crate::ninja::{Controls, Event, Ninja, RADIUS};
use crate::objects::{Objects, WorldEvent};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashSet};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct Human {
    pub n: Ninja,
    pub input: Input,
    pub world: Objects,
    pub frame: u32,
    pub delay_frames: u32,
    pub dir_key: i8,
    pub dir_since: u32,
    pub jump_since: u32,
    pub last_dir_event: u32,
    pub last_jump_event: u32,
    pub switches: Vec<bool>,
}

/// Keys held per 6 frame step: direction (-1, 0, 1) and jump.
pub type Path = Vec<(i8, bool)>;
/// A leg's target test.
type Goal<'a> = Box<dyn Fn(&Human) -> bool + 'a>;

const REPEAT_EVERY: u32 = 2; // about 33 ms

fn press(i: &mut Input, code: KeyCode, kind: KeyEventKind, at: Instant) {
    i.on_key(KeyEvent::new_with_kind(code, KeyModifiers::NONE, kind), at);
}

/// One frame with the keys in the given state. Returns false if the ninja died.
pub fn frame(h: &mut Human, level: &Level, want_dir: i8, want_jump: bool, base: Instant, exact: bool) -> bool {
    frame_ctl(h, level, want_dir, want_jump, base, exact).0
}

/// Same, also returning the controls the input code produced.
pub fn frame_ctl(h: &mut Human, level: &Level, want_dir: i8, want_jump: bool, base: Instant, exact: bool) -> (bool, Controls) {
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
        let repeat = !exact && h.dir_since > h.delay_frames && (h.dir_since - h.delay_frames).is_multiple_of(REPEAT_EVERY);
        if h.dir_since == 1 || repeat {
            press(&mut h.input, code, KeyEventKind::Press, now);
            h.last_dir_event = h.frame;
        }
    }
    if want_jump {
        h.jump_since += 1;
        let repeat = !exact && h.jump_since > h.delay_frames && (h.jump_since - h.delay_frames).is_multiple_of(REPEAT_EVERY);
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

pub fn finished(h: &Human, level: &Level) -> bool {
    level
        .exits
        .iter()
        .zip(&h.switches)
        .any(|(e, &on)| on && (e.door.0 - h.n.x).powi(2) + (e.door.1 - h.n.y).powi(2) <= (RADIUS + 12.0).powi(2))
}

pub type Key = ((i32, i32, i32, i32, bool, i8, bool, u32), (i8, u32, u32), u32);

/// Merge equivalent situations: for each key only its phase matters.
pub fn key(h: &Human, exact: bool) -> Key {
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

pub fn start_state(level: &Level, exact: bool, delay_frames: u32) -> Human {
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
pub fn search_from(
    level: &Level,
    start: Human,
    base: Instant,
    exact: bool,
    max_nodes: usize,
    weight: f32,
    target: (f32, f32),
    goal: &dyn Fn(&Human) -> bool,
) -> (Option<(Path, Human)>, usize) {
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
pub fn play_level(level: &Level, exact: bool, delay_frames: u32) -> (Option<Vec<(i8, bool)>>, usize) {
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

pub fn summarise(path: &[(i8, bool)]) -> String {
    let jumps = path.windows(2).filter(|w| !w[0].1 && w[1].1).count() + path.first().is_some_and(|p| p.1) as usize;
    let turns = path.windows(2).filter(|w| w[0].0 != 0 && w[1].0 != 0 && w[0].0 != w[1].0).count();
    format!("{:.1} s, {} jump presses, {} direction reversals", path.len() as f32 * 6.0 / 60.0, jumps, turns)
}

/// A key schedule as text: two characters per 6 frames, direction (L, _, R) then jump (j or .).
pub fn encode(path: &[(i8, bool)]) -> String {
    path.iter().map(|&(d, j)| format!("{}{}", ['L', '_', 'R'][(d + 1) as usize], if j { 'j' } else { '.' })).collect()
}



/// Searches leg by leg through waypoints a person picked, then to the switch, then
/// through more waypoints to the door. Each leg tries a strong, then a weak, then no
/// pull toward its target.
pub fn waypoint_route(level: &Level, spec: &str) -> (Option<Vec<(i8, bool)>>, usize) {
    let e = level.exits[0];
    let base = Instant::now();
    let parse = |part: &str| -> Vec<(f32, f32)> {
        part.split(';').filter(|p| !p.trim().is_empty()).map(|p| {
            let mut it = p.split(',').map(|v| v.trim().parse::<f32>().unwrap_or(0.0));
            (it.next().unwrap_or(0.0), it.next().unwrap_or(0.0))
        }).collect()
    };
    let mut halves = spec.split('|');
    let (before, after) = (parse(halves.next().unwrap_or("")), parse(halves.next().unwrap_or("")));
    let mut state = start_state(level, true, 30);
    let mut all = Vec::new();
    let mut total = 0;
    let mut legs: Vec<((f32, f32), Goal)> = Vec::new();
    for w in before {
        legs.push((w, Box::new(move |h: &Human| (h.n.x - w.0).powi(2) + (h.n.y - w.1).powi(2) < 14.0 * 14.0)));
    }
    legs.push((e.switch, Box::new(|h: &Human| h.switches[0])));
    for w in after {
        legs.push((w, Box::new(move |h: &Human| (h.n.x - w.0).powi(2) + (h.n.y - w.1).powi(2) < 14.0 * 14.0)));
    }
    legs.push((e.door, Box::new(move |h: &Human| finished(h, level))));
    for (i, (target, goal)) in legs.iter().enumerate() {
        let mut found = None;
        for weight in [3.0f32, 1.0, 0.0] {
            let (leg, n) = search_from(level, state.clone(), base, true, 1_500_000, weight, *target, goal.as_ref());
            total += n;
            if leg.is_some() {
                found = leg;
                break;
            }
        }
        let Some((p, end)) = found else {
            println!("leg {i} to {target:?} failed");
            return (None, total);
        };
        println!("leg {i} to {target:?}: {:.1} s", p.len() as f32 / 10.0);
        all.extend(p);
        state = end;
    }
    (Some(all), total)
}


/// `entity --prove <file>`: searches the level's route (through its `waypoints` when it has
/// them), checks it in the real game and stores it in the file as `route`.
pub fn prove(path: &str) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let mut level = Level::parse(&text);
    if level.exits.is_empty() {
        return Err(format!("{path}: the level has no exit"));
    }
    let (route, nodes) = match level.waypoints.clone() {
        Some(w) => waypoint_route(&level, &w),
        None => play_level(&level, true, 30),
    };
    let route = route.ok_or_else(|| format!("{path}: no route found ({nodes} states)"))?;
    let code = encode(&route);
    let g = crate::demo::replay(&level, &code);
    println!(
        "{path} ({}): {}; in the game: complete {}, deaths {}, gold {}/{}, {:.1} s",
        level.label,
        summarise(&route),
        g.complete,
        g.deaths,
        g.gold_collected(),
        g.gold_total(),
        g.tick as f32 / 60.0
    );
    if !g.complete {
        return Err(format!("{path}: the route the bot found does not finish the level in the game"));
    }
    level.route = Some(code);
    std::fs::write(path, level.to_text()).map_err(|e| format!("{path}: {e}"))
}
