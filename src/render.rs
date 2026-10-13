//! Board renderer. One fixed layout: the level's tiles are 2 cells wide by 1 cell
//! tall, drawn as single block glyphs (see `art`), inside a box-drawing frame whose
//! top edge carries the title and whose bottom edge carries score, gold and deaths.
//! The ninja and particles are Braille (2x4 dots per cell, 6 world units per dot).

use crate::art::{Geometry, MIN_BOARD};
use crate::game::Game;
use crate::input::Input;
use crate::level::{Level, TILE};
use crate::ninja::{Ninja, RADIUS};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::Frame;

const NINJA: Color = Color::Rgb(255, 255, 255);
const PARTICLE: Color = Color::Rgb(255, 176, 0);
const WALL: Color = Color::Rgb(0, 200, 70);
const INTERIOR: Color = Color::Rgb(0, 90, 35);
const FRAME: Color = Color::Rgb(0, 160, 60);
const TEXT: Color = Color::Rgb(255, 176, 0);
const GOLD: Color = Color::Rgb(255, 220, 60);
const DANGER: Color = Color::Rgb(255, 80, 80);
const OBJECT: Color = Color::Rgb(0, 200, 255);
const BLACK: Color = Color::Rgb(0, 0, 0);

pub fn draw(f: &mut Frame, g: &Game, input: &Input) {
    let area = f.area();
    let (bw, bh) = g.art.geo.board();
    if area.width < MIN_BOARD.0 || area.height < MIN_BOARD.1 {
        too_small(f, area, MIN_BOARD.0, MIN_BOARD.1);
        return;
    }
    if area.width < bw || area.height < bh {
        // the board is re-fitted before each draw; a stale one just waits for it
        too_small(f, area, bw, bh);
        return;
    }
    let origin = (area.x + (area.width - bw) / 2, area.y + (area.height - bh) / 2);
    paint(f.buffer_mut(), origin, g, input.mode_note(), input.confirm_quit);
}

fn too_small(f: &mut Frame, area: Rect, bw: u16, bh: u16) {
    let lines = [
        "Terminal too small".to_string(),
        format!("Entity needs at least {bw} x {bh} cells"),
        format!("this one is {} x {}", area.width, area.height),
        "Please make the window bigger (q quits)".to_string(),
    ];
    let top = area.y + area.height.saturating_sub(lines.len() as u16) / 2;
    for (i, l) in lines.iter().enumerate() {
        let w = l.chars().count() as u16;
        let x = area.x + area.width.saturating_sub(w) / 2;
        let y = top + i as u16;
        if y < area.y + area.height {
            f.buffer_mut().set_stringn(x, y, l, (area.width.saturating_sub(x - area.x)) as usize, Style::default().fg(TEXT));
        }
    }
}

fn put(buf: &mut Buffer, origin: (u16, u16), lc: i32, row: i32, ch: char, style: Style) {
    if lc < 0 || row < 0 {
        return;
    }
    if let Some(cell) = buf.cell_mut((origin.0 + lc as u16, origin.1 + row as u16)) {
        cell.set_char(ch).set_style(style);
    }
}

fn text(buf: &mut Buffer, origin: (u16, u16), lc: i32, row: i32, s: &str, style: Style) {
    for (i, ch) in s.chars().enumerate() {
        put(buf, origin, lc + i as i32, row, ch, style);
    }
}

const BIT: [[u8; 4]; 2] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];

/// Braille dot accumulator over the board (cell index = row * bw + lc). A dot is
/// `dw` x `dh` world units: a cell is 2 dots wide and 4 tall.
struct Dots {
    bw: i32,
    bh: i32,
    dw: f32,
    dh: f32,
    cells: Vec<u8>,
}

impl Dots {
    fn new(bw: i32, bh: i32, cell: (f32, f32)) -> Dots {
        Dots { bw, bh, dw: cell.0 / 2.0, dh: cell.1 / 4.0, cells: vec![0; (bw * bh) as usize] }
    }

    /// Set the dot at dot coordinates (dx, dy), counted from the playfield corner.
    /// Board column and row 0 are the frame line, so the playfield starts at 1.
    fn set(&mut self, dx: i32, dy: i32) {
        let (lc, row) = (dx.div_euclid(2) + 1, dy.div_euclid(4) + 1);
        if lc < 0 || row < 0 || lc >= self.bw || row >= self.bh {
            return;
        }
        self.cells[(row * self.bw + lc) as usize] |= BIT[dx.rem_euclid(2) as usize][dy.rem_euclid(4) as usize];
    }

    /// A straight line of dots between two world points.
    fn line(&mut self, a: (f32, f32), b: (f32, f32)) {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = (dx * dx + dy * dy).sqrt();
        let step = (self.dw.min(self.dh) * 0.5).max(0.5);
        let n = (len / step).ceil().max(1.0) as i32;
        for i in 0..=n {
            let t = i as f32 / n as f32;
            self.set_world(a.0 + dx * t, a.1 + dy * t);
        }
    }

    fn set_world(&mut self, x: f32, y: f32) {
        self.set(((x - TILE) / self.dw).floor() as i32, ((y - TILE) / self.dh).floor() as i32);
    }

    /// A disc of world radius `r`; an ellipse in dot space when dots are not square.
    fn disc(&mut self, x: f32, y: f32, r: f32) {
        let (cx, cy) = ((x - TILE) / self.dw, (y - TILE) / self.dh);
        let (rx, ry) = (r / self.dw, r / self.dh);
        for dy in (cy - ry).floor() as i32..=(cy + ry).ceil() as i32 {
            for dx in (cx - rx).floor() as i32..=(cx + rx).ceil() as i32 {
                let (ex, ey) = ((dx as f32 + 0.5 - cx) / rx, (dy as f32 + 0.5 - cy) / ry);
                if ex * ex + ey * ey <= 1.0 {
                    self.set(dx, dy);
                }
            }
        }
    }

    /// The ninja as a stick figure (see `sprite`). Below about 5 dots across a figure is
    /// too small to read, so it falls back to a disc.
    /// `scale` enlarges only the drawing (the hitbox stays radius 10); the figure is anchored so
    /// its feet stay at the bottom of the hitbox and it grows up and sideways.
    fn ninja(&mut self, n: &Ninja, scale: f32) {
        if 2.0 * RADIUS * scale / self.dw < 5.0 {
            self.disc(n.x, n.y, RADIUS);
            return;
        }
        self.figure(crate::sprite::figure(n.pose()), (n.x, n.y), n.facing, n.tilt, scale);
    }

    /// N's stick figure: mirrored when facing left, then turned by `tilt` (clockwise on
    /// screen) about the hitbox centre, as the original's clip is; `scale` grows it from
    /// the feet.
    fn figure(&mut self, f: &crate::sprite::Figure, at: (f32, f32), facing: i8, tilt: f32, scale: f32) {
        let flip = if facing < 0 { -1.0 } else { 1.0 };
        let (sin, cos) = tilt.sin_cos();
        let world = |p: crate::sprite::P| {
            let (x, y) = (flip * spread(p.0) * scale, RADIUS + (p.1 - RADIUS) * scale);
            (at.0 + x * cos - y * sin, at.1 + x * sin + y * cos)
        };
        for (a, b) in f.segments() {
            self.thin_line(world(a), world(b));
        }
        // the head: the original's small triangle is about one dot here, so mark its
        // centre and fill whatever dots it covers
        let c = world(f.head_centre());
        self.set_world(c.0, c.1);
        let h = f.head.map(|q| world((q[0], q[1])));
        let (x0, x1) = (h.iter().map(|q| q.0).fold(f32::MAX, f32::min), h.iter().map(|q| q.0).fold(f32::MIN, f32::max));
        let (y0, y1) = (h.iter().map(|q| q.1).fold(f32::MAX, f32::min), h.iter().map(|q| q.1).fold(f32::MIN, f32::max));
        let ((dx0, dy0), (dx1, dy1)) = (self.dot_of(x0, y0), self.dot_of(x1, y1));
        for dy in dy0..=dy1 {
            for dx in dx0..=dx1 {
                let (px, py) = (TILE + (dx as f32 + 0.5) * self.dw, TILE + (dy as f32 + 0.5) * self.dh);
                let side = |a: (f32, f32), b: (f32, f32)| (b.0 - a.0) * (py - a.1) - (b.1 - a.1) * (px - a.0);
                let s = [side(h[0], h[1]), side(h[1], h[2]), side(h[2], h[0])];
                if s.iter().all(|&v| v >= 0.0) || s.iter().all(|&v| v <= 0.0) {
                    self.set(dx, dy);
                }
            }
        }
    }

    fn dot_of(&self, x: f32, y: f32) -> (i32, i32) {
        (((x - TILE) / self.dw).floor() as i32, ((y - TILE) / self.dh).floor() as i32)
    }

    /// A one dot wide line between two world points (Bresenham in dot space), so limbs stay
    /// thin where `line` would double up dots on diagonals.
    fn thin_line(&mut self, a: (f32, f32), b: (f32, f32)) {
        let ((mut x, mut y), (x1, y1)) = (self.dot_of(a.0, a.1), self.dot_of(b.0, b.1));
        let (dx, dy) = ((x1 - x).abs(), -(y1 - y).abs());
        let (sx, sy) = ((x1 - x).signum(), (y1 - y).signum());
        let mut err = dx + dy;
        loop {
            self.set(x, y);
            if (x, y) == (x1, y1) {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    fn at(&self, lc: i32, row: i32) -> u8 {
        self.cells[(row * self.bw + lc) as usize]
    }
}

/// Pulls the figure's limbs apart sideways without moving its edges: N's ninja is about
/// 6 px wide inside a 20 px hitbox, so at a few dots per figure its arms fuse with its body.
/// x maps to x * |x|-dependent gain: 0 stays 0, the hitbox edge (10) stays 10, the middle
/// is widened (gain `SPREAD` at the centre). `ENTITY_NINJA_SPREAD=1` draws the original.
fn spread(x: f32) -> f32 {
    use std::sync::OnceLock;
    static K: OnceLock<f32> = OnceLock::new();
    let k = *K.get_or_init(|| std::env::var("ENTITY_NINJA_SPREAD").ok().and_then(|v| v.parse().ok()).unwrap_or(SPREAD).max(1.0));
    let t = (x.abs() / RADIUS).min(1.0);
    x.signum() * RADIUS * (1.0 - (1.0 - t).powf(k)) + x.signum() * (x.abs() - RADIUS).max(0.0)
}

const SPREAD: f32 = 1.6;

fn object_glyph(kind: &str) -> (char, Color) {
    match kind {
        "gold" => ('\u{25C6}', GOLD),
        "exit" => ('\u{25A3}', OBJECT),
        "testdoor" => ('+', OBJECT),
        "mine" => ('\u{2731}', DANGER),
        "drone" => ('\u{25C9}', DANGER),
        "turret" => ('\u{25B2}', DANGER),
        "floorguard" => ('\u{25BC}', DANGER),
        "thwomp" => ('\u{25A0}', DANGER),
        "hominglauncher" => ('\u{25C8}', DANGER),
        "launchpad" => ('^', OBJECT),
        "bounceblock" => ('\u{25A1}', OBJECT),
        "onewayplatform" => ('=', OBJECT),
        _ => ('?', OBJECT),
    }
}

/// Paint the whole board with its top left corner at `origin`.
pub fn paint(buf: &mut Buffer, origin: (u16, u16), g: &Game, keys_note: Option<&str>, confirm_quit: bool) {
    let level = g.level();
    let geo: Geometry = g.art.geo;
    let (bw, bh) = geo.board();
    let (bw, bh) = (bw as i32, bh as i32);
    let (cw, ch_) = geo.cell_size(level);
    let frame = Style::default().fg(FRAME);

    // frame
    for lc in 0..bw {
        let (top, bottom) = match lc {
            0 => ('\u{250C}', '\u{2514}'),
            c if c == bw - 1 => ('\u{2510}', '\u{2518}'),
            _ => ('\u{2500}', '\u{2500}'),
        };
        put(buf, origin, lc, 0, top, frame);
        put(buf, origin, lc, bh - 1, bottom, frame);
    }
    for row in 1..bh - 1 {
        put(buf, origin, 0, row, '\u{2502}', frame);
        put(buf, origin, bw - 1, row, '\u{2502}', frame);
    }

    // tiles, one glyph per cell
    for row in 1..bh - 1 {
        for lc in 1..bw - 1 {
            let c = g.art.cell(lc - 1, row - 1);
            if c.ch == ' ' {
                continue;
            }
            let style = if c.inverted {
                // reverse video: the empty part shows the terminal's own background colour
                Style::default().fg(WALL).add_modifier(Modifier::REVERSED)
            } else if c.interior {
                Style::default().fg(INTERIOR)
            } else {
                Style::default().fg(WALL)
            };
            put(buf, origin, lc, row, c.ch, style);
        }
    }

    // objects, one glyph each, in empty cells
    for (o, &got) in level.objects.iter().zip(&g.collected) {
        if got || matches!(o.kind.as_str(), "mine" | "launchpad" | "testdoor" | "onewayplatform" | "bounceblock" | "drone" | "turret" | "floorguard" | "thwomp" | "hominglauncher") {
            continue;
        }
        // Some objects sit exactly on the edge of the playfield (cloud city has a trail of
        // gold along the floor line), so clamp them onto the nearest inner cell.
        let ic = (((o.x - TILE) / cw).floor() as i32).clamp(0, geo.n - 1);
        let ir = (((o.y - TILE) / ch_).floor() as i32).clamp(0, geo.m - 1);
        let (ch, color) = object_glyph(&o.kind);
        put(buf, origin, ic + 1, ir + 1, ch, Style::default().fg(color));
    }

    // world objects (mines, pads, doors, one-way platforms, bounce blocks)
    let cell_of = |x: f32, y: f32| -> Option<(i32, i32)> {
        let (ic, ir) = (((x - TILE) / cw).floor() as i32, ((y - TILE) / ch_).floor() as i32);
        (ic >= 0 && ic < geo.n && ir >= 0 && ir < geo.m).then_some((ic + 1, ir + 1))
    };
    for m in &g.world.mines {
        if let Some((lc, row)) = cell_of(m.x, m.y) {
            let (ch, color) = if m.exploded { ('\u{00B7}', DANGER) } else { ('\u{2731}', DANGER) };
            put(buf, origin, lc, row, ch, Style::default().fg(color));
        }
    }
    for p in &g.world.pads {
        if let Some((lc, row)) = cell_of(p.x, p.y) {
            let ch = match ((p.nx * 1.5).round() as i32, (p.ny * 1.5).round() as i32) {
                (0, -1) => '\u{2191}',
                (1, -1) => '\u{2197}',
                (1, 0) => '\u{2192}',
                (1, 1) => '\u{2198}',
                (0, 1) => '\u{2193}',
                (-1, 1) => '\u{2199}',
                (-1, 0) => '\u{2190}',
                _ => '\u{2196}',
            };
            put(buf, origin, lc, row, ch, Style::default().fg(OBJECT));
        }
    }
    for o in &g.world.oneways {
        if let Some((lc, row)) = cell_of(o.x + o.dir.0 * TILE / 2.0, o.y + o.dir.1 * TILE / 2.0) {
            let ch = if o.dir.0 == 0.0 { '\u{2550}' } else { '\u{2551}' };
            put(buf, origin, lc, row, ch, Style::default().fg(OBJECT));
        }
    }
    for b in &g.world.blocks {
        if let Some((lc, row)) = cell_of(b.pos.0, b.pos.1) {
            put(buf, origin, lc, row, '\u{25A4}', Style::default().fg(GOLD));
        }
    }
    for d in &g.world.doors {
        use crate::objects::DoorKind::*;
        if !d.open {
            // the barrier along its tile edge, one glyph per cell it crosses
            let horizontal = d.seg.a.1 == d.seg.b.1;
            let steps = if horizontal { ((d.seg.b.0 - d.seg.a.0) / cw).ceil() as i32 } else { ((d.seg.b.1 - d.seg.a.1) / ch_).ceil() as i32 }.max(1);
            for k in 0..steps {
                let t = (k as f32 + 0.5) / steps as f32;
                let (x, y) = (d.seg.a.0 + (d.seg.b.0 - d.seg.a.0) * t, d.seg.a.1 + (d.seg.b.1 - d.seg.a.1) * t);
                if let Some((lc, row)) = cell_of(x, y) {
                    put(buf, origin, lc, row, if horizontal { '\u{2501}' } else { '\u{2503}' }, Style::default().fg(TEXT));
                }
            }
        }
        if d.kind != Trek && !d.spent && !(d.kind == Locked && d.open) {
            if let Some((lc, row)) = cell_of(d.trigger.0, d.trigger.1) {
                put(buf, origin, lc, row, '\u{25C9}', Style::default().fg(if d.kind == Trap { DANGER } else { GOLD }));
            }
        }
    }

    // turrets (with their crosshair), floor guards, thwomps, homing launchers and rockets
    for t in &g.world.turrets {
        if let Some((lc, row)) = cell_of(t.pos.0, t.pos.1) {
            put(buf, origin, lc, row, '\u{25B2}', Style::default().fg(DANGER));
        }
        if t.targeting() {
            if let Some((lc, row)) = cell_of(t.aim.0, t.aim.1) {
                put(buf, origin, lc, row, '\u{2295}', Style::default().fg(DANGER));
            }
        }
    }
    for gd in &g.world.guards {
        if let Some((lc, row)) = cell_of(gd.pos.0, gd.pos.1) {
            put(buf, origin, lc, row, if gd.chasing { '\u{25D9}' } else { '\u{25CF}' }, Style::default().fg(DANGER));
        }
    }
    for th in &g.world.thwomps {
        if let Some((lc, row)) = cell_of(th.pos.0, th.pos.1) {
            put(buf, origin, lc, row, '\u{2593}', Style::default().fg(if th.moving() { DANGER } else { TEXT }));
        }
    }
    for l in &g.world.launchers {
        if let Some((lc, row)) = cell_of(l.base.0, l.base.1) {
            put(buf, origin, lc, row, '\u{25C7}', Style::default().fg(DANGER));
        }
        if l.flying() {
            if let Some((lc, row)) = cell_of(l.rocket.0, l.rocket.1) {
                put(buf, origin, lc, row, '\u{2737}', Style::default().fg(PARTICLE));
            }
        }
    }

    // drones: one glyph each; their beams and tracers are Braille lines
    for d in &g.world.drones {
        use crate::drones::Weapon;
        if let Some((lc, row)) = cell_of(d.pos.0, d.pos.1) {
            let (ch, color) = match (d.weapon, d.chaser) {
                (Weapon::Zap, false) => ('\u{2726}', DANGER),
                (Weapon::Zap, true) => ('\u{2605}', DANGER),
                (Weapon::Laser, _) => ('\u{25C8}', Color::Rgb(255, 80, 200)),
                (Weapon::Chaingun, _) => ('\u{25A0}', DANGER),
            };
            put(buf, origin, lc, row, ch, Style::default().fg(color));
        }
    }

    // exits: the switch, and the door (closed until its switch is touched)
    for (e, &on) in level.exits.iter().zip(&g.switches) {
        // the goal must stand out among the gold: bold and bright
        let switch_color = if on { Color::Rgb(80, 255, 120) } else { Color::Rgb(255, 150, 0) };
        for (pos, ch, color) in [(e.switch, '\u{25C9}', switch_color), (e.door, if on { '\u{25A1}' } else { '\u{25A3}' }, Color::Rgb(0, 255, 255))] {
            let ic = (((pos.0 - TILE) / cw).floor() as i32).clamp(0, geo.n - 1);
            let ir = (((pos.1 - TILE) / ch_).floor() as i32).clamp(0, geo.m - 1);
            put(buf, origin, ic + 1, ir + 1, ch, Style::default().fg(color).add_modifier(Modifier::BOLD));
        }
    }

    // particles and ninja as Braille
    let mut particles = Dots::new(bw, bh, (cw, ch_));
    for p in &g.particles {
        particles.set_world(p.x, p.y);
    }
    let mut warn = Dots::new(bw, bh, (cw, ch_));
    let mut live = Dots::new(bw, bh, (cw, ch_));
    for d in &g.world.drones {
        if let Some((a, b)) = d.beam {
            if d.firing() {
                live.line(a, b);
            } else {
                warn.line(a, b);
            }
        }
        if let Some((a, b, left)) = d.tracer {
            if left > 0 {
                live.line(a, b);
            }
        }
    }
    for t in &g.world.turrets {
        if let Some((a, b, left)) = t.tracer {
            if left > 0 {
                live.line(a, b);
            }
        }
    }
    let mut ninja = Dots::new(bw, bh, (cw, ch_));
    ninja.ninja(&g.ninja, g.ninja_scale);
    for row in 1..bh - 1 {
        for lc in 1..bw - 1 {
            let (n, p) = (ninja.at(lc, row), particles.at(lc, row));
            if n != 0 {
                // keep the wall's shape under the ninja by merging its dots
                let mask = n | g.art.cell(lc - 1, row - 1).dots;
                put(buf, origin, lc, row, braille(mask), Style::default().fg(NINJA));
            } else if live.at(lc, row) != 0 && g.art.cell(lc - 1, row - 1).ch == ' ' {
                put(buf, origin, lc, row, braille(live.at(lc, row)), Style::default().fg(Color::Rgb(255, 60, 60)));
            } else if warn.at(lc, row) != 0 && g.art.cell(lc - 1, row - 1).ch == ' ' {
                put(buf, origin, lc, row, braille(warn.at(lc, row)), Style::default().fg(Color::Rgb(150, 50, 50)));
            } else if p != 0 && g.art.cell(lc - 1, row - 1).ch == ' ' {
                put(buf, origin, lc, row, braille(p), Style::default().fg(PARTICLE));
            }
        }
    }

    // titles in the frame
    let bright = Style::default().fg(TEXT);
    let title = format!("[Entity Level {}]", level.label);
    text(buf, origin, (bw - title.chars().count() as i32) / 2, 0, &title, bright);
    if let Some(note) = keys_note {
        let note = format!("[{note}]");
        text(buf, origin, bw - 2 - note.len() as i32, 0, &note, bright);
    }
    let score = format!("[Score {}]", g.score);
    let gold = format!("[Gold {}/{}]", g.gold_collected(), g.gold_total());
    let deaths = format!("[Deaths {}]", g.deaths);
    let time = g.seconds_left().map_or("[Time --]".to_string(), |t| format!("[Time {t:.1}]"));
    text(buf, origin, 3, bh - 1, &score, bright);
    let mid = format!("{time}\u{2500}\u{2500}{gold}");
    text(buf, origin, (bw - mid.chars().count() as i32) / 2, bh - 1, &mid, bright);
    text(buf, origin, bw - 3 - deaths.chars().count() as i32, bh - 1, &deaths, bright);

    let episode_msg = format!("  Episode complete!  Score {}  ", g.score);
    let overlay: Option<[&str; 2]> = if confirm_quit {
        Some(["  Quit Entity?  ", "  y = quit   any other key = keep playing  "])
    } else if g.game_over {
        Some(["  Out of time!  ", "  r = try the episode again  "])
    } else if g.complete && g.episode_done {
        Some([&episode_msg, "  n = next episode   r = play again  "])
    } else if g.complete {
        Some(["  Level complete!  ", "  n = next level   r = play again  "])
    } else {
        None
    };
    if let Some(lines) = overlay {
        let w = lines.iter().map(|l| l.chars().count()).max().unwrap() as i32;
        let (x0, y0) = ((bw - w - 2) / 2, bh / 2 - 2);
        let boxed = Style::default().fg(TEXT).bg(BLACK);
        for dy in 0..4 {
            for dx in 0..w + 2 {
                let ch = match (dy, dx) {
                    (0, 0) => '\u{250C}',
                    (0, d) if d == w + 1 => '\u{2510}',
                    (3, 0) => '\u{2514}',
                    (3, d) if d == w + 1 => '\u{2518}',
                    (0, _) | (3, _) => '\u{2500}',
                    (_, 0) => '\u{2502}',
                    (_, d) if d == w + 1 => '\u{2502}',
                    _ => ' ',
                };
                put(buf, origin, x0 + dx, y0 + dy, ch, boxed);
            }
        }
        for (i, l) in lines.iter().enumerate() {
            let pad = (w as usize - l.chars().count()) / 2;
            text(buf, origin, x0 + 1 + pad as i32, y0 + 1 + i as i32, l, boxed);
        }
    }
}

fn braille(mask: u8) -> char {
    char::from_u32(0x2800 + mask as u32).unwrap()
}

// ---- glyph preview page: `entity --glyphs` ----

fn ansi(buf: &Buffer, area: Rect) -> String {
    let mut out = String::new();
    for y in 0..area.height {
        for x in 0..area.width {
            let c = &buf[(x, y)];
            let rgb = |col: Color, base: u8| match col {
                Color::Rgb(r, g, b) => format!("\x1b[{base};2;{r};{g};{b}m"),
                _ => String::new(),
            };
            out.push_str(&rgb(c.fg, 38));
            out.push_str(&match c.bg {
                Color::Rgb(..) => rgb(c.bg, 48),
                _ => "\x1b[49m".into(),
            });
            out.push_str(c.symbol());
            out.push_str("\x1b[0m");
        }
        out.push('\n');
    }
    out
}

pub fn print_glyph_page(levels: &[Level]) {
    let row = |label: &str, s: &str| println!("{label:<22}{s}");
    println!("Glyph preview. Every glyph below should look like a clean block or shape that fills its cell.");
    println!("A box, a gap, or a glyph that sticks out sideways means your terminal font lacks it.\n");
    row("lower eighths", "\u{2581}\u{2582}\u{2583}\u{2584}\u{2585}\u{2586}\u{2587}\u{2588}");
    row("left eighths", "\u{258F}\u{258E}\u{258D}\u{258C}\u{258B}\u{258A}\u{2589}\u{2588}");
    row("quadrants", "\u{2596}\u{2597}\u{2598}\u{259D}\u{259A}\u{259E}\u{2599}\u{259B}\u{259C}\u{259F}");
    row("shades", "\u{2591}\u{2592}\u{2593}");
    row("frame", "\u{250C}\u{2500}\u{2510}\u{2502}\u{2514}\u{2518}");
    row("gold, exit, switch", "\u{25C6}  \u{25A3} \u{25A1}  \u{25C9}");
    row("mine, launch pads", "\u{2731}  \u{2191}\u{2197}\u{2192}\u{2198}\u{2193}\u{2199}\u{2190}\u{2196}");
    row("doors, one-way, block", "\u{2501} \u{2503}  \u{2550} \u{2551}  \u{25A4}");
    row("drones", "\u{2726} zap  \u{2605} chaser  \u{25C8} laser  \u{25A0} chaingun");
    row("turret, aim", "\u{25B2} \u{2295}");
    row("guard, thwomp", "\u{25CF} \u{25D9}  \u{2593}");
    row("launcher, rocket", "\u{25C7} \u{2737}");
    row("braille", "\u{2801}\u{2803}\u{2807}\u{2847}\u{28C7}\u{28E7}\u{28F7}\u{28FF} \u{2880}\u{28C0}\u{2840}");
    let range = |a: u32, n: u32| (a..a + n).filter_map(char::from_u32).collect::<String>();
    row("sextants (optional)", &range(0x1FB00, 16));
    row("wedges (optional)", &range(0x1FB3C, 16));
    row("octants (optional)", &range(0x1CD00, 16));
    println!("\nNinja poses (the stick figure at about the size of a 156x47 window, 2.8 x 3.1 world units per dot;");
    println!("the game draws it at the board's own dot size and mirrors it when facing left):");
    let poses: Vec<(&str, Vec<String>)> = crate::sprite::ALL.iter().map(|(n, p)| (*n, figure_braille(*p, (2.8, 3.1)))).collect();
    for r in 0..poses[0].1.len() {
        println!("  {}", poses.iter().map(|(_, b)| format!("{:<8}", b[r])).collect::<String>());
    }
    println!("  {}", poses.iter().map(|(n, _)| format!("{n:<8}")).collect::<String>());
    println!("\nBoard preview (cloud city):\n");
    let idx = levels.iter().position(|l| l.label.contains("cloud city")).unwrap_or(0);
    let g = Game::new(levels_clone_one(levels, idx), 0);
    let (bw, bh) = g.art.geo.board();
    let area = Rect::new(0, 0, bw, bh);
    let mut buf = Buffer::empty(area);
    paint(&mut buf, (0, 0), &g, None, false);
    print!("{}", ansi(&buf, area));
}

/// One pose drawn on a small dot grid with dots of `dot` world units, as rows of Braille.
fn figure_braille(pose: crate::ninja::Pose, dot: (f32, f32)) -> Vec<String> {
    let (bw, bh) = (6, 4);
    let mut d = Dots::new(bw, bh, (dot.0 * 2.0, dot.1 * 4.0));
    // the hitbox centre, placed so its bottom sits on the last dot row
    let at = (TILE + 2.5 * 2.0 * dot.0, TILE + 3.0 * 4.0 * dot.1 - RADIUS);
    d.figure(crate::sprite::figure(pose), at, 1, 0.0, 1.0);
    (1..bh).map(|row| (1..bw).map(|lc| braille(d.at(lc, row))).collect()).collect()
}

fn levels_clone_one(levels: &[Level], idx: usize) -> Vec<Level> {
    vec![levels[idx].clone_level()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::art;
    use crate::level::n_levels;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn render_with(cols: u16, rows: u16, name: &str, aspect: f32) -> (Vec<String>, (u16, u16)) {
        let levels = n_levels();
        let idx = levels.iter().position(|l| l.label.contains(name)).unwrap();
        let mut g = Game::new(levels, idx);
        if let Some(geo) = art::fit(g.level(), cols, rows, aspect) {
            g.set_geometry(geo);
        }
        let board = g.art.geo.board();
        let input = Input::new(true);
        let mut t = Terminal::new(TestBackend::new(cols, rows)).unwrap();
        t.draw(|f| draw(f, &g, &input)).unwrap();
        let b = t.backend().buffer();
        let lines = (0..rows).map(|y| (0..cols).map(|x| b[(x, y)].symbol().to_string()).collect::<String>()).collect();
        (lines, board)
    }

    fn render(cols: u16, rows: u16, name: &str) -> Vec<String> {
        render_with(cols, rows, name, 2.0).0
    }

    #[test]
    fn small_terminal_gets_a_message() {
        let s = render(63, 25, "cloud city").join("\n");
        assert!(s.contains("Terminal too small"), "{s}");
        assert!(s.contains("64 x 25"));
        assert!(!s.contains('\u{250C}'), "no board should be drawn");
        let s = render(80, 24, "cloud city").join("\n");
        assert!(s.contains("Terminal too small"), "24 rows is one short");
    }

    #[test]
    fn the_board_fills_the_terminal_and_is_centred() {
        for (cols, rows) in [(64u16, 25u16), (80, 30), (100, 30), (156, 47), (200, 50)] {
            let (lines, (bw, bh)) = render_with(cols, rows, "cloud city", 2.18);
            assert!(bw <= cols && bh <= rows);
            // it uses all of the limiting dimension
            assert!(bw == cols || bh == rows || (cols - bw <= 2 && rows - bh <= 2), "{cols}x{rows} -> {bw}x{bh}");
            let (x, y) = ((cols - bw) as usize / 2, (rows - bh) as usize / 2);
            assert_eq!(lines[y].chars().nth(x), Some('\u{250C}'), "{cols}x{rows}: {}", lines[y]);
            assert_eq!(lines[y].chars().nth(x + bw as usize - 1), Some('\u{2510}'));
            assert_eq!(lines[y + bh as usize - 1].chars().nth(x), Some('\u{2514}'));
            assert!(lines[y].contains("[Entity Level 0:1 cloud city]"), "{}", lines[y]);
            let bottom = &lines[y + bh as usize - 1];
            assert!(bottom.contains("[Score 0]") && bottom.contains("[Gold 0/") && bottom.contains("[Deaths 0]"), "{bottom}");
        }
    }

    #[test]
    fn exact_minimum_fits() {
        let rows = render(64, 25, "cloud city");
        assert_eq!(rows[0].chars().next(), Some('\u{250C}'));
        assert_eq!(rows[24].chars().last(), Some('\u{2518}'));
    }

    #[test]
    fn gold_stays_one_character_at_any_size() {
        // cloud city has 32 gold pieces; 20 of them lie on the floor line and are clamped
        // onto the bottom row, so on a small board several can share a cell
        for (cols, rows, at_least) in [(64u16, 25u16, 12usize), (156, 47, 28)] {
            let (lines, _) = render_with(cols, rows, "cloud city", 2.18);
            let gold = lines.iter().flat_map(|l| l.chars()).filter(|&c| c == '\u{25C6}').count();
            assert!(gold >= at_least && gold <= 32, "{cols}x{rows}: {gold} gold glyphs");
        }
    }

    #[test]
    fn quit_asks_first() {
        let levels = n_levels();
        let g = Game::new(levels, 1);
        let mut t = Terminal::new(TestBackend::new(64, 25)).unwrap();
        let mut input = Input::new(true);
        input.confirm_quit = true;
        t.draw(|f| draw(f, &g, &input)).unwrap();
        let b = t.backend().buffer();
        let all: String = (0..25).map(|y| (0..64).map(|x| b[(x, y)].symbol().to_string()).collect::<String>()).collect::<Vec<_>>().join("\n");
        assert!(all.contains("Quit Entity?") && all.contains("y = quit"), "{all}");
    }

    #[test]
    fn prints_the_board() {
        for r in render(64, 25, "cloud city") {
            println!("{r}");
        }
    }

    #[test]
    fn prints_the_ninja_in_three_poses_on_a_big_board() {
        use crate::ninja::Controls;
        let levels = n_levels();
        let mut g = Game::new(levels, 1);
        let geo = art::fit(g.level(), 156, 47, 2.18).unwrap();
        g.set_geometry(geo);
        let input = Input::new(true);
        let show = |g: &Game, title: &str| {
            let mut t = Terminal::new(TestBackend::new(156, 47)).unwrap();
            t.draw(|f| draw(f, g, &input)).unwrap();
            let b = t.backend().buffer();
            let (bw, bh) = g.art.geo.board();
            let (ox, oy) = ((156 - bw) / 2, (47 - bh) / 2);
            // the ninja's cell
            let (cw, ch) = g.art.geo.cell_size(g.level());
            let (lc, lr) = (((g.ninja.x - 24.0) / cw) as u16 + 1, ((g.ninja.y - 24.0) / ch) as u16 + 1);
            println!("{title}:");
            for dy in -2i32..=1 {
                let line: String = (-4i32..=4)
                    .map(|dx| b[((ox as i32 + lc as i32 + dx) as u16, (oy as i32 + lr as i32 + dy) as u16)].symbol().to_string())
                    .collect();
                println!("   {line}");
            }
        };
        for _ in 0..60 {
            g.update(&Controls::default());
        }
        show(&g, "standing");
        for _ in 0..40 {
            g.update(&Controls { dir: 1, ..Default::default() });
        }
        show(&g, "running right");
        g.update(&Controls { jump_pressed: true, jump_held: true, dir: 1 });
        for _ in 0..6 {
            g.update(&Controls { jump_held: true, dir: 1, ..Default::default() });
        }
        show(&g, "jumping");
    }

    #[test]
    fn prints_the_practice_course() {
        let levels = vec![crate::level::Level::parse(include_str!("../levels/001.txt"))];
        let mut g = Game::new(levels, 0);
        let geo = art::fit(g.level(), 120, 40, 2.18).unwrap();
        g.set_geometry(geo);
        let input = Input::new(true);
        let mut t = Terminal::new(TestBackend::new(120, 40)).unwrap();
        t.draw(|f| draw(f, &g, &input)).unwrap();
        let b = t.backend().buffer();
        for y in 0..40 {
            println!("{}", (0..120).map(|x| b[(x, y)].symbol().to_string()).collect::<String>().trim_end());
        }
    }

    #[test]
    fn prints_a_laser_drone_charging_and_firing() {
        use crate::ninja::Controls;
        let mut lv = crate::level::Level::parse(include_str!("../levels/001.txt"));
        lv.add_test_objects("drone 100 500 4 0 1 0");
        let mut g = Game::new(vec![lv], 0);
        let geo = art::fit(g.level(), 100, 32, 2.18).unwrap();
        g.set_geometry(geo);
        let input = Input::new(true);
        let show = |g: &Game, title: &str| {
            let mut t = Terminal::new(TestBackend::new(100, 32)).unwrap();
            t.draw(|f| draw(f, g, &input)).unwrap();
            let b = t.backend().buffer();
            println!("--- {title}");
            for y in 18..31 {
                println!("{}", (0..100).map(|x| b[(x, y)].symbol().to_string()).collect::<String>().trim_end());
            }
        };
        g.ninja.x = 300.0;
        for _ in 0..30 {
            g.update(&Controls::default());
        }
        show(&g, "charging (thin warning line)");
        for _ in 0..40 {
            g.update(&Controls { dir: 0, ..Default::default() });
        }
        show(&g, "firing (beam)");
    }

    #[test]
    fn partly_filled_cells_use_reverse_video_not_black() {
        let levels = n_levels();
        let idx = levels.iter().position(|l| l.label.contains("cloud city")).unwrap();
        let mut g = Game::new(levels, idx);
        g.set_geometry(art::fit(g.level(), 156, 47, 2.18).unwrap());
        let (bw, bh) = g.art.geo.board();
        let mut buf = Buffer::empty(Rect::new(0, 0, bw, bh));
        paint(&mut buf, (0, 0), &g, None, false);
        let inverted = (0..bh).flat_map(|y| (0..bw).map(move |x| (x, y))).filter(|&(x, y)| buf[(x, y)].modifier.contains(Modifier::REVERSED)).count();
        assert!(inverted > 0, "this level has cells filled from the top or right");
        let black = (0..bh).flat_map(|y| (0..bw).map(move |x| (x, y))).filter(|&(x, y)| buf[(x, y)].fg == BLACK && buf[(x, y)].symbol() != " ").count();
        assert_eq!(black, 0, "no glyph is drawn in black any more");
    }

    #[test]
    fn a_bigger_drawn_ninja_has_more_dots_but_keeps_its_feet_on_the_floor() {
        let levels = n_levels();
        let mut g = Game::new(levels, 1);
        g.set_geometry(art::fit_with(art::FitMode::Square, g.level(), 156, 47, 2.18).unwrap());
        let (bw, bh) = g.art.geo.board();
        let (cw, ch) = g.art.geo.cell_size(g.level());
        let count = |scale: f32| {
            let mut d = Dots::new(bw as i32, bh as i32, (cw, ch));
            d.ninja(&g.ninja, scale);
            let mut dots = 0u32;
            let mut lowest = i32::MIN;
            for row in 0..bh as i32 {
                for lc in 0..bw as i32 {
                    let m = d.at(lc, row);
                    if m != 0 {
                        dots += m.count_ones();
                        lowest = lowest.max(row);
                    }
                }
            }
            (dots, lowest)
        };
        let (small, low_small) = count(1.0);
        let (big, low_big) = count(1.5);
        assert!(big > small + small / 4, "{small} dots at 1x against {big} at 1.5x");
        assert!(low_big <= low_small + 1, "feet stay at the hitbox bottom: row {low_small} against {low_big}");
        // the hitbox is untouched by the drawing size
        assert_eq!(crate::ninja::RADIUS, 10.0);
    }

    #[test]
    fn prints_the_ninja_at_several_drawn_sizes() {
        let levels = n_levels();
        let mut g = Game::new(levels, 1);
        g.set_geometry(art::fit_with(art::FitMode::Fill, g.level(), 156, 47, 2.18).unwrap());
        let input = Input::new(true);
        for scale in [1.0f32, 1.5, 2.0] {
            g.ninja_scale = scale;
            let mut t = Terminal::new(TestBackend::new(156, 47)).unwrap();
            t.draw(|f| draw(f, &g, &input)).unwrap();
            let b = t.backend().buffer();
            let (bw, bh) = g.art.geo.board();
            let (ox, oy) = ((156 - bw) / 2, (47 - bh) / 2);
            let (cw, ch) = g.art.geo.cell_size(g.level());
            let (lc, lr) = (((g.ninja.x - 24.0) / cw) as u16 + 1, ((g.ninja.y - 24.0) / ch) as u16 + 1);
            println!("drawn at {scale}x:");
            for dy in -3i32..=1 {
                println!("   {}", (-4i32..=4).map(|dx| b[((ox as i32 + lc as i32 + dx) as u16, (oy as i32 + lr as i32 + dy) as u16)].symbol().to_string()).collect::<String>());
            }
        }
    }
}
#[cfg(test)]
mod figure_dump {
    use super::*;

    /// The stick figure's poses magnified, one '#' per dot. DW and DH set the dot size in
    /// world units (2.8 x 3.1 is a 156x47 window at 1x; divide by the drawn scale for more):
    /// `DW=1.87 DH=2.07 cargo test --release dump_figures -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn dump_figures_as_ascii() {
        let dot: (f32, f32) = (std::env::var("DW").ok().and_then(|v| v.parse().ok()).unwrap_or(2.8), std::env::var("DH").ok().and_then(|v| v.parse().ok()).unwrap_or(3.1));
        let mut rows = vec![String::new(); 12];
        for (_, pose) in crate::sprite::ALL {
            let mut d = Dots::new(6, 4, (dot.0 * 2.0, dot.1 * 4.0));
            let at = (TILE + 2.5 * 2.0 * dot.0, TILE + 3.0 * 4.0 * dot.1 - RADIUS);
            d.figure(crate::sprite::figure(pose), at, 1, 0.0, 1.0);
            for y in 0..12 {
                for x in 0..10 {
                    let (lc, row) = (x / 2 + 1, y / 4 + 1);
                    let on = d.at(lc, row) & BIT[(x % 2) as usize][(y % 4) as usize] != 0;
                    rows[y as usize].push_str(if on { "##" } else { ". " });
                }
                rows[y as usize].push_str("   ");
            }
        }
        println!("{}", crate::sprite::ALL.iter().map(|(n, _)| format!("{n:<23}")).collect::<String>());
        for r in rows { println!("{r}"); }
    }
}
