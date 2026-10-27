//! `entity --edit [file [level]]`: the level editor. The board is drawn by the game's own
//! renderer (`render::paint`), so a level looks exactly as it will in play; a cursor tile
//! is shown in reverse video. Every tile shape and every object can be placed and erased,
//! the level is saved in the text format `Level::parse` reads, and `p` test-plays it
//! through the real game loop (Esc comes back). Keys are listed in `HELP`.

use crate::art;
use crate::game::Game;
use crate::level::{centre, tile_of, Exit, Level, Obj, FULL, TILE};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::style::{Color, Modifier, Style};
use ratatui::DefaultTerminal;
use std::io;
use std::time::{Duration, Instant};

const TEXT: Color = Color::Rgb(255, 176, 0);
const DIM: Color = Color::Rgb(0, 160, 60);

const HELP: [&str; 2] = [
    "arrows move (shift: 4)  space/enter place  x/del erase  tab/[ ] brush  , . variant  r turn  e pick tile",
    "p test play (Esc back)  s save  n name  u undo  mouse: left place, right erase  Esc/q quit",
];

/// What the place key puts down.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Brush {
    Tile,
    Spawn,
    Exit,
    Gold,
    Mine,
    Drone,
    Turret,
    FloorGuard,
    Thwomp,
    Launcher,
    LaunchPad,
    BounceBlock,
    OneWay,
    TrekDoor,
    LockedDoor,
    TrapDoor,
}

const BRUSHES: [Brush; 16] = [
    Brush::Tile,
    Brush::Spawn,
    Brush::Exit,
    Brush::Gold,
    Brush::Mine,
    Brush::Drone,
    Brush::Turret,
    Brush::FloorGuard,
    Brush::Thwomp,
    Brush::Launcher,
    Brush::LaunchPad,
    Brush::BounceBlock,
    Brush::OneWay,
    Brush::TrekDoor,
    Brush::LockedDoor,
    Brush::TrapDoor,
];

/// Drone kinds as (name, movement, chaser, weapon) in the original's record layout.
const DRONES: [(&str, f32, f32, f32); 9] = [
    ("zap, follows walls clockwise", 0.0, 0.0, 0.0),
    ("zap, follows walls anticlockwise", 1.0, 0.0, 0.0),
    ("zap, wanders clockwise", 2.0, 0.0, 0.0),
    ("zap, wanders anticlockwise", 3.0, 0.0, 0.0),
    ("zap chaser", 0.0, 1.0, 0.0),
    ("laser, follows walls clockwise", 0.0, 0.0, 1.0),
    ("laser, wanders", 2.0, 0.0, 1.0),
    ("chaingun, follows walls clockwise", 0.0, 0.0, 2.0),
    ("chaingun, wanders", 2.0, 0.0, 2.0),
];

/// Launch pad directions (surface normal), up first.
const PADS: [(&str, (i32, i32)); 8] = [
    ("up", (0, -1)),
    ("up and right", (1, -1)),
    ("right", (1, 0)),
    ("down and right", (1, 1)),
    ("down", (0, 1)),
    ("down and left", (-1, 1)),
    ("left", (-1, 0)),
    ("up and left", (-1, -1)),
];

/// Directions 0 right, 1 down, 2 left, 3 up (drones, thwomps, one-way platforms).
const DIRS: [&str; 4] = ["right", "down", "left", "up"];

impl Brush {
    fn name(self) -> &'static str {
        match self {
            Brush::Tile => "tile",
            Brush::Spawn => "spawn",
            Brush::Exit => "exit door, then its switch",
            Brush::Gold => "gold",
            Brush::Mine => "mine",
            Brush::Drone => "drone",
            Brush::Turret => "gauss turret",
            Brush::FloorGuard => "floor guard",
            Brush::Thwomp => "thwomp",
            Brush::Launcher => "homing launcher",
            Brush::LaunchPad => "launch pad",
            Brush::BounceBlock => "bounce block",
            Brush::OneWay => "one-way platform",
            Brush::TrekDoor => "door",
            Brush::LockedDoor => "locked door, then its key",
            Brush::TrapDoor => "trap door, then its trigger",
        }
    }

    fn variants(self) -> usize {
        match self {
            Brush::Tile => 33,
            Brush::Drone => DRONES.len(),
            Brush::LaunchPad => PADS.len(),
            Brush::FloorGuard => 2,
            Brush::Thwomp | Brush::OneWay => 4,
            Brush::TrekDoor | Brush::LockedDoor | Brush::TrapDoor => 2,
            _ => 1,
        }
    }

    fn variant_name(self, v: usize) -> String {
        match self {
            Brush::Tile => tile_name(v as u8 + 1),
            Brush::Drone => DRONES[v].0.to_string(),
            Brush::LaunchPad => format!("launches {}", PADS[v].0),
            Brush::FloorGuard => ["starts right", "starts left"][v].to_string(),
            Brush::Thwomp => format!("slams {}", DIRS[v]),
            Brush::OneWay => format!("solid side {}", DIRS[v]),
            Brush::TrekDoor | Brush::LockedDoor | Brush::TrapDoor => ["on the right edge", "on the bottom edge"][v].to_string(),
            _ => String::new(),
        }
    }
}

/// A readable name for a tile code (see `shapes`), with the letter the text format uses.
fn tile_name(code: u8) -> String {
    let letter = match code {
        FULL => '#',
        27 => 'k',
        c => (b'0' + c) as char,
    };
    let facing = |c: u8| ["up and right", "up and left", "down and left", "down and right"][((c - 2) % 4) as usize];
    let name = match code {
        FULL => "full".to_string(),
        2..=5 => format!("45 degree slope facing {}", facing(code)),
        6..=9 => format!("concave curve facing {}", facing(code)),
        10..=13 => format!("convex curve facing {}", facing(code)),
        14..=17 => format!("22 degree slope, low half, facing {}", facing(code)),
        18..=21 => format!("22 degree slope, high half, facing {}", facing(code)),
        22..=25 => format!("67 degree slope, thin half, facing {}", facing(code)),
        26..=29 => format!("67 degree slope, thick half, facing {}", facing(code)),
        30 => "half tile, bottom".to_string(),
        31 => "half tile, right".to_string(),
        32 => "half tile, top".to_string(),
        _ => "half tile, left".to_string(),
    };
    format!("{letter} {name}")
}

#[derive(Clone, Debug, PartialEq)]
enum Mode {
    Edit,
    /// The first half of a two part object is down; the place key puts the second.
    Second(Brush),
    Naming(String),
    ConfirmQuit,
}

pub struct Editor {
    path: String,
    /// The level lives in the game, which draws it (and test plays a copy of it).
    game: Game,
    cursor: (i32, i32),
    brush: usize,
    variants: [usize; 16],
    turn: usize,
    mode: Mode,
    dirty: bool,
    undo: Vec<String>,
    message: String,
    quit: bool,
    /// Where the board was last drawn (top left) and its size, for the mouse.
    origin: (u16, u16),
}

impl Editor {
    pub fn new(path: &str, level: Level) -> Editor {
        let cursor = (level.w / 2, level.h / 2);
        Editor {
            path: path.to_string(),
            game: Game::new(vec![level], 0),
            cursor,
            brush: 0,
            variants: [0; 16],
            turn: 0,
            mode: Mode::Edit,
            dirty: false,
            undo: Vec::new(),
            message: String::new(),
            quit: false,
            origin: (0, 0),
        }
    }

    pub fn level(&self) -> &Level {
        &self.game.levels[0]
    }

    fn level_mut(&mut self) -> &mut Level {
        &mut self.game.levels[0]
    }

    fn brush(&self) -> Brush {
        BRUSHES[self.brush]
    }

    fn variant(&self) -> usize {
        self.variants[self.brush]
    }

    /// Remember the level before a change (for undo).
    fn checkpoint(&mut self) {
        self.undo.push(self.level().to_text());
        if self.undo.len() > 200 {
            self.undo.remove(0);
        }
        self.dirty = true;
    }

    /// After a change: rebuild the board art (tiles) and the objects of the attempt.
    fn changed(&mut self, tiles: bool) {
        if tiles {
            self.game.art = art::build(&self.game.levels[0], self.game.art.geo);
        }
        self.game.respawn();
    }

    fn set_level(&mut self, level: Level) {
        self.game.levels[0] = level;
        self.changed(true);
    }

    pub fn move_cursor(&mut self, dx: i32, dy: i32) {
        let (w, h) = (self.level().w, self.level().h);
        // the frame tiles stay solid: only the playfield can be edited
        self.cursor = ((self.cursor.0 + dx).clamp(1, w - 2), (self.cursor.1 + dy).clamp(1, h - 2));
    }

    /// Put the current brush at the cursor.
    pub fn place(&mut self) {
        let (tx, ty) = self.cursor;
        let (cx, cy) = centre(tx, ty);
        let (brush, v) = (self.brush(), self.variant());
        if let Mode::Second(first) = self.mode.clone() {
            self.place_second(first);
            return;
        }
        self.checkpoint();
        let obj = |kind: &str, params: Vec<f32>| Obj { kind: kind.to_string(), x: params[0], y: params[1], params };
        let mut tiles = false;
        match brush {
            Brush::Tile => {
                self.clear_objects_in(tx, ty);
                self.level_mut().set_code(tx, ty, v as u8 + 1);
                tiles = true;
            }
            Brush::Spawn => self.level_mut().spawn = crate::level::spawn_on(tx, ty),
            Brush::Exit => {
                self.level_mut().exits.push(Exit { door: (cx, cy), switch: (cx, cy) });
                self.mode = Mode::Second(Brush::Exit);
            }
            Brush::Gold => self.level_mut().objects.push(obj("gold", vec![cx, cy])),
            Brush::Mine => self.level_mut().objects.push(obj("mine", vec![cx, cy])),
            Brush::Drone => {
                let (_, mv, chaser, weapon) = DRONES[v];
                let turn = self.turn as f32;
                self.level_mut().objects.push(obj("drone", vec![cx, cy, mv, chaser, weapon, turn]));
            }
            Brush::Turret => self.level_mut().objects.push(obj("turret", vec![cx, cy])),
            // a floor guard stands on the floor of its tile
            Brush::FloorGuard => self.level_mut().objects.push(obj("floorguard", vec![cx, (ty + 1) as f32 * TILE - 6.0, [1.0, -1.0][v]])),
            Brush::Thwomp => self.level_mut().objects.push(obj("thwomp", vec![cx, cy, v as f32])),
            Brush::Launcher => self.level_mut().objects.push(obj("hominglauncher", vec![cx, cy])),
            Brush::LaunchPad => {
                // a straight pad sits on the tile side behind it, a diagonal one in the middle
                // of the tile (on a 45 degree slope)
                let (nx, ny) = PADS[v].1;
                let (x, y) = if nx == 0 || ny == 0 { (cx - nx as f32 * TILE / 2.0, cy - ny as f32 * TILE / 2.0) } else { (cx, cy) };
                let k = if nx != 0 && ny != 0 { std::f32::consts::FRAC_1_SQRT_2 } else { 1.0 };
                self.level_mut().objects.push(obj("launchpad", vec![x, y, nx as f32 * k, ny as f32 * k]));
            }
            Brush::BounceBlock => self.level_mut().objects.push(obj("bounceblock", vec![cx, cy])),
            Brush::OneWay => self.level_mut().objects.push(obj("onewayplatform", vec![cx, cy, v as f32])),
            Brush::TrekDoor | Brush::LockedDoor | Brush::TrapDoor => {
                let (trap, locked) = match brush {
                    Brush::TrapDoor => (1.0, 0.0),
                    Brush::LockedDoor => (0.0, 1.0),
                    _ => (0.0, 0.0),
                };
                let params = vec![cx, cy, v as f32, trap, tx as f32, ty as f32, locked, 0.0, 0.0];
                self.level_mut().objects.push(obj("testdoor", params));
                if brush != Brush::TrekDoor {
                    self.mode = Mode::Second(brush);
                }
            }
        }
        log::debug!("place {:?} variant {} at {:?}", brush, v, self.cursor);
        self.message = match self.mode {
            Mode::Second(Brush::Exit) => "now place the switch (Esc cancels)".into(),
            Mode::Second(Brush::LockedDoor) => "now place the key (Esc cancels)".into(),
            Mode::Second(_) => "now place the trigger (Esc cancels)".into(),
            _ => String::new(),
        };
        self.changed(tiles);
    }

    /// The switch of an exit, or the key or trigger of a door, just placed first.
    fn place_second(&mut self, first: Brush) {
        let at = centre(self.cursor.0, self.cursor.1);
        let level = self.level_mut();
        if first == Brush::Exit {
            if let Some(e) = level.exits.last_mut() {
                e.switch = at;
            }
        } else if let Some(o) = level.objects.iter_mut().rev().find(|o| o.kind == "testdoor") {
            o.params[0] = at.0;
            o.params[1] = at.1;
            o.x = at.0;
            o.y = at.1;
        }
        log::debug!("place second part of {:?} at {:?}", first, self.cursor);
        self.mode = Mode::Edit;
        self.message.clear();
        self.changed(false);
    }

    /// Esc while placing the second part: take the first part away again.
    fn cancel_second(&mut self) {
        if let Some(text) = self.undo.pop() {
            self.set_level(Level::parse(&text));
        }
        self.mode = Mode::Edit;
        self.message = "cancelled".into();
        log::debug!("two part placement cancelled");
    }

    /// Objects touching the tile: in it, or a door whose barrier or trigger is in it, or
    /// an exit whose door or switch is in it.
    fn clear_objects_in(&mut self, tx: i32, ty: i32) -> bool {
        let level = self.level_mut();
        let (n_obj, n_exit) = (level.objects.len(), level.exits.len());
        level.objects.retain(|o| {
            let barrier = o.kind == "testdoor" && o.params.len() >= 9 && ((o.params[4] + o.params[7]) as i32, (o.params[5] + o.params[8]) as i32) == (tx, ty);
            // a pad or floor guard on a tile edge belongs to the tile it was placed in
            let inside = tile_of((o.x, o.y)) == (tx, ty) || tile_of((o.x - 0.01, o.y - 0.01)) == (tx, ty);
            !(barrier || inside)
        });
        level.exits.retain(|e| tile_of(e.door) != (tx, ty) && tile_of(e.switch) != (tx, ty));
        level.objects.len() != n_obj || level.exits.len() != n_exit
    }

    /// Erase at the cursor: the objects there first, then the tile.
    pub fn erase(&mut self) {
        let (tx, ty) = self.cursor;
        self.checkpoint();
        let tiles = if self.clear_objects_in(tx, ty) {
            false
        } else if self.level().code(tx, ty) != 0 {
            self.level_mut().set_code(tx, ty, 0);
            true
        } else {
            self.undo.pop();
            return;
        };
        log::debug!("erase at {:?} (tile {})", self.cursor, tiles);
        self.changed(tiles);
    }

    pub fn undo(&mut self) {
        match self.undo.pop() {
            Some(text) => {
                let name = self.level().name.clone();
                let mut l = Level::parse(&text);
                if l.name.is_empty() {
                    l.name = name;
                }
                self.set_level(l);
                self.message = "undone".into();
            }
            None => self.message = "nothing to undo".into(),
        }
        log::debug!("undo, {} steps left", self.undo.len());
    }

    pub fn save(&mut self) {
        let text = self.level().to_text();
        match std::fs::write(&self.path, text) {
            Ok(()) => {
                self.dirty = false;
                self.message = format!("saved {}", self.path);
            }
            Err(e) => self.message = format!("could not save {}: {e}", self.path),
        }
        log::info!("save {}: {}", self.path, self.message);
    }

    fn cycle_brush(&mut self, d: i32) {
        self.brush = (self.brush as i32 + d).rem_euclid(BRUSHES.len() as i32) as usize;
        log::debug!("brush {:?}", self.brush());
    }

    fn cycle_variant(&mut self, d: i32) {
        let n = self.brush().variants() as i32;
        self.variants[self.brush] = (self.variants[self.brush] as i32 + d).rem_euclid(n) as usize;
        log::debug!("variant {} of {:?}", self.variant(), self.brush());
    }

    /// Takes the tile under the cursor as the brush.
    fn pick(&mut self) {
        let code = self.level().code(self.cursor.0, self.cursor.1);
        if code != 0 {
            self.brush = 0;
            self.variants[0] = code as usize - 1;
        }
    }

    /// One key press (or repeat). Returns true when the level should be test played.
    pub fn on_key(&mut self, k: KeyEvent) -> bool {
        if k.kind == KeyEventKind::Release {
            return false;
        }
        log::debug!("editor key {:?} mode {:?}", k, self.mode);
        if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c') {
            self.quit = true;
            return false;
        }
        match self.mode.clone() {
            Mode::Naming(mut name) => {
                match k.code {
                    KeyCode::Enter => {
                        self.checkpoint();
                        let level = self.level_mut();
                        level.name = name.trim().to_string();
                        level.label = level.name.clone();
                        self.mode = Mode::Edit;
                        log::debug!("named {:?}", self.level().name);
                        return false;
                    }
                    KeyCode::Esc => self.mode = Mode::Edit,
                    KeyCode::Backspace => {
                        name.pop();
                        self.mode = Mode::Naming(name);
                    }
                    KeyCode::Char(c) if name.chars().count() < 40 => {
                        name.push(c);
                        self.mode = Mode::Naming(name);
                    }
                    _ => {}
                }
                return false;
            }
            Mode::ConfirmQuit => {
                match k.code {
                    KeyCode::Char('y') => {
                        self.save();
                        self.quit = !self.dirty;
                    }
                    KeyCode::Char('n') => self.quit = true,
                    _ => {}
                }
                if !self.quit {
                    self.mode = Mode::Edit;
                }
                return false;
            }
            _ => {}
        }
        let step = if k.modifiers.contains(KeyModifiers::SHIFT) { 4 } else { 1 };
        match k.code {
            KeyCode::Left => self.move_cursor(-step, 0),
            KeyCode::Right => self.move_cursor(step, 0),
            KeyCode::Up => self.move_cursor(0, -step),
            KeyCode::Down => self.move_cursor(0, step),
            KeyCode::Char(' ') | KeyCode::Enter => self.place(),
            KeyCode::Esc if matches!(self.mode, Mode::Second(_)) => self.cancel_second(),
            _ if matches!(self.mode, Mode::Second(_)) => {}
            KeyCode::Char('x') | KeyCode::Delete | KeyCode::Backspace => self.erase(),
            KeyCode::Tab | KeyCode::Char(']') => self.cycle_brush(1),
            KeyCode::BackTab | KeyCode::Char('[') => self.cycle_brush(-1),
            KeyCode::Char('.') => self.cycle_variant(1),
            KeyCode::Char(',') => self.cycle_variant(-1),
            KeyCode::Char('r') => {
                self.turn = (self.turn + 1) % 4;
                log::debug!("drone start direction {}", DIRS[self.turn]);
            }
            KeyCode::Char('e') => self.pick(),
            KeyCode::Char('u') => self.undo(),
            KeyCode::Char('s') => self.save(),
            KeyCode::Char('n') => self.mode = Mode::Naming(self.level().name.clone()),
            KeyCode::Char('p') => return true,
            KeyCode::Esc | KeyCode::Char('q') => {
                if self.dirty {
                    self.mode = Mode::ConfirmQuit;
                } else {
                    self.quit = true;
                }
            }
            _ => {}
        }
        false
    }

    /// Left button places, right button erases, at the tile under the pointer (dragging too).
    fn on_mouse(&mut self, m: MouseEvent) {
        let geo = self.game.art.geo;
        let (cw, ch) = geo.cell_size(self.level());
        let (lc, row) = (m.column as i32 - self.origin.0 as i32 - 1, m.row as i32 - self.origin.1 as i32 - 1);
        if lc < 0 || row < 0 || lc >= geo.n || row >= geo.m {
            return;
        }
        let (tx, ty) = tile_of((TILE + (lc as f32 + 0.5) * cw, TILE + (row as f32 + 0.5) * ch));
        let moved = (tx, ty) != self.cursor;
        match m.kind {
            MouseEventKind::Down(b) | MouseEventKind::Drag(b) => {
                log::debug!("editor mouse {:?} at tile ({tx},{ty})", m.kind);
                self.cursor = (tx, ty);
                self.move_cursor(0, 0);
                let fresh = matches!(m.kind, MouseEventKind::Down(_));
                if b == MouseButton::Left && (fresh || moved) {
                    self.place();
                } else if b == MouseButton::Right && (fresh || moved) {
                    self.erase();
                }
            }
            MouseEventKind::Moved => self.cursor = (tx, ty),
            _ => {}
        }
    }

    fn status(&self) -> String {
        let b = self.brush();
        let variant = b.variant_name(self.variant());
        let turn = if b == Brush::Drone { format!(", starts {}", DIRS[self.turn]) } else { String::new() };
        let brush = if variant.is_empty() { b.name().to_string() } else { format!("{}: {variant}{turn}", b.name()) };
        let mut s = format!("[{}{}] [{brush}] [{},{}]", self.path, if self.dirty { " *" } else { "" }, self.cursor.0, self.cursor.1);
        match &self.mode {
            Mode::Naming(n) => s = format!("[name: {n}_]  Enter keeps it, Esc cancels"),
            Mode::ConfirmQuit => s = "[unsaved changes] y save and quit  n quit without saving  other keys keep editing".into(),
            _ if !self.message.is_empty() => s.push_str(&format!(" {}", self.message)),
            _ => {}
        }
        s
    }

    /// Draw the board, the cursor, the status line and the key help.
    fn draw(&mut self, buf: &mut Buffer) {
        let area = buf.area;
        let help_rows = if area.height >= art::MIN_BOARD.1 + 4 { 2 } else { 0 };
        let Some(geo) = art::fit_with(self.game.fit_mode, self.level(), area.width, area.height - help_rows - 1, cell_aspect()) else {
            buf.set_string(0, 0, "Terminal too small for the editor (Esc quits)", Style::default().fg(TEXT));
            return;
        };
        self.game.set_geometry(geo);
        let (bw, bh) = geo.board();
        let origin = ((area.width - bw) / 2, (area.height - help_rows - 1 - bh) / 2);
        self.origin = origin;
        crate::render::paint(buf, origin, &self.game, None, false);
        // the cursor: every cell the tile covers, in reverse video
        let (cw, ch) = geo.cell_size(self.level());
        let (tx, ty) = self.cursor;
        let (x0, y0) = (((tx - 1) as f32 * TILE / cw).floor() as i32, ((ty - 1) as f32 * TILE / ch).floor() as i32);
        let (x1, y1) = (((tx as f32 * TILE / cw).ceil() as i32).max(x0 + 1), ((ty as f32 * TILE / ch).ceil() as i32).max(y0 + 1));
        for row in y0..y1.min(geo.m) {
            for lc in x0..x1.min(geo.n) {
                if let Some(cell) = buf.cell_mut((origin.0 + 1 + lc as u16, origin.1 + 1 + row as u16)) {
                    cell.modifier.toggle(Modifier::REVERSED);
                    if cell.symbol() == " " {
                        cell.set_fg(TEXT);
                    }
                }
            }
        }
        let status = self.status();
        let y = origin.1 + bh;
        buf.set_stringn(0, y, format!("{status:^w$}", w = area.width as usize), area.width as usize, Style::default().fg(TEXT));
        for (i, h) in HELP.iter().enumerate().take(help_rows as usize) {
            buf.set_stringn(0, y + 1 + i as u16, format!("{h:^w$}", w = area.width as usize), area.width as usize, Style::default().fg(DIM));
        }
    }

    /// The level as the test run plays it: on an episode clock, so the time it takes shows.
    fn for_test_play(&self) -> Level {
        let mut l = self.level().clone_level();
        l.episode = Some((0, 0));
        l
    }
}

fn cell_aspect() -> f32 {
    crate::cell_aspect()
}

pub fn run(terminal: &mut DefaultTerminal, path: &str, from: Option<Level>, kitty: bool) -> io::Result<()> {
    let level = match std::fs::read_to_string(path) {
        Ok(text) => Level::parse(&text),
        Err(_) => from.unwrap_or_else(Level::blank),
    };
    log::info!("editor opens {path} ({})", level.label);
    let mut ed = Editor::new(path, level);
    execute!(io::stdout(), EnableMouseCapture)?;
    let mut last_snapshot = Instant::now();
    while !ed.quit {
        terminal.draw(|f| ed.draw(f.buffer_mut()))?;
        if event::poll(Duration::from_millis(250))? {
            let mut play = false;
            loop {
                match event::read()? {
                    Event::Key(k) => play |= ed.on_key(k),
                    Event::Mouse(m) => ed.on_mouse(m),
                    _ => {}
                }
                if !event::poll(Duration::ZERO)? {
                    break;
                }
            }
            if play {
                log::info!("test play of {}", ed.level().label);
                crate::run(terminal, vec![ed.for_test_play()], 0, kitty, false, true)?;
                ed.message = "back from the test run".into();
            }
        }
        if last_snapshot.elapsed() >= Duration::from_secs(1) {
            log::debug!(
                "editor snapshot cursor={:?} brush={:?} variant={} mode={:?} dirty={} objects={} exits={}",
                ed.cursor,
                ed.brush(),
                ed.variant(),
                ed.mode,
                ed.dirty,
                ed.level().objects.len(),
                ed.level().exits.len()
            );
            last_snapshot = Instant::now();
        }
    }
    execute!(io::stdout(), DisableMouseCapture)?;
    log::info!("editor closes {path}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyEventState;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent { code, modifiers: KeyModifiers::NONE, kind: KeyEventKind::Press, state: KeyEventState::NONE }
    }

    fn editor() -> Editor {
        Editor::new("unused.txt", Level::blank())
    }

    fn select(ed: &mut Editor, brush: Brush) {
        while ed.brush() != brush {
            ed.on_key(key(KeyCode::Tab));
        }
    }

    #[test]
    fn every_brush_and_variant_places_something_that_survives_a_save_and_load() {
        let mut ed = editor();
        for (i, &b) in BRUSHES.iter().enumerate() {
            select(&mut ed, b);
            for v in 0..b.variants() {
                ed.variants[i] = v;
                ed.cursor = (1 + (v as i32 % 31), 1 + (i as i32 + v as i32 / 31) % 23);
                ed.place();
                if matches!(ed.mode, Mode::Second(_)) {
                    ed.move_cursor(1, 0);
                    ed.place();
                }
                assert_eq!(ed.mode, Mode::Edit);
            }
        }
        let text = ed.level().to_text();
        let back = Level::parse(&text);
        assert_eq!(back.to_text(), text);
        assert_eq!(back.objects.len(), ed.level().objects.len());
        assert_eq!(back.exits.len(), 1);
        // and the game can build every object from it
        let mut g = Game::new(vec![back], 0);
        for _ in 0..120 {
            g.update(&crate::ninja::Controls::default());
        }
    }

    #[test]
    fn erase_takes_objects_first_then_the_tile_and_undo_brings_them_back() {
        let mut ed = editor();
        ed.cursor = (5, 5);
        ed.place(); // a full tile
        select(&mut ed, Brush::Gold);
        ed.place();
        assert_eq!(ed.level().code(5, 5), FULL);
        assert_eq!(ed.level().objects.len(), 1);
        ed.erase();
        assert_eq!(ed.level().objects.len(), 0, "the gold goes first");
        assert_eq!(ed.level().code(5, 5), FULL);
        ed.erase();
        assert_eq!(ed.level().code(5, 5), 0, "then the tile");
        ed.undo();
        ed.undo();
        assert_eq!((ed.level().code(5, 5), ed.level().objects.len()), (FULL, 1));
    }

    #[test]
    fn an_exit_takes_two_places_and_esc_cancels_it() {
        let mut ed = editor();
        select(&mut ed, Brush::Exit);
        ed.cursor = (20, 23);
        ed.place();
        assert_eq!(ed.mode, Mode::Second(Brush::Exit));
        ed.on_key(key(KeyCode::Esc));
        assert!(ed.level().exits.is_empty());
        ed.place();
        ed.cursor = (10, 23);
        ed.place();
        assert_eq!(ed.level().exits.len(), 1);
        assert_eq!(ed.level().exits[0].door, centre(20, 23));
        assert_eq!(ed.level().exits[0].switch, centre(10, 23));
    }

    #[test]
    fn the_cursor_stays_inside_the_frame() {
        let mut ed = editor();
        ed.move_cursor(-100, -100);
        assert_eq!(ed.cursor, (1, 1));
        ed.move_cursor(100, 100);
        assert_eq!(ed.cursor, (31, 23));
    }

    #[test]
    fn saving_writes_the_text_format_and_loading_gives_the_same_level() {
        let dir = std::env::temp_dir().join(format!("entity-editor-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("level.txt");
        let mut ed = Editor::new(path.to_str().unwrap(), Level::parse(include_str!("../levels/001.txt")));
        select(&mut ed, Brush::Mine);
        ed.cursor = (8, 21);
        ed.place();
        ed.save();
        assert!(!ed.dirty);
        let back = Level::parse(&std::fs::read_to_string(&path).unwrap());
        assert_eq!(back.to_text(), ed.level().to_text());
        assert_eq!(back.objects[0].kind, "mine");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_editor_draws_the_board_cursor_and_help() {
        let mut ed = Editor::new("x.txt", Level::parse(include_str!("../levels/001.txt")));
        let mut buf = Buffer::empty(ratatui::layout::Rect::new(0, 0, 110, 32));
        ed.draw(&mut buf);
        let text: String = (0..32).map(|y| (0..110).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>() + "\n").collect();
        assert!(text.contains("practice"), "{text}");
        assert!(text.contains("[x.txt]"), "{text}");
        assert!(text.contains("test play"), "{text}");
    }
}
