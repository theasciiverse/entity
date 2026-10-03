use crate::shapes::shape;
use serde::Deserialize;

/// World units follow N++: one tile is 24 units, the ninja radius is 10.
pub const TILE: f32 = 24.0;

pub const FULL: u8 = 1;

pub struct Obj {
    pub kind: String,
    pub x: f32,
    pub y: f32,
    /// Raw parameters as extracted; their meaning is not decoded yet.
    #[allow(dead_code)]
    pub params: Vec<f32>,
}

/// An exit door that opens when its switch is touched (as in N).
#[derive(Clone, Copy, Debug)]
pub struct Exit {
    pub door: (f32, f32),
    pub switch: (f32, f32),
}

pub struct Level {
    pub label: String,
    pub w: i32,
    pub h: i32,
    tiles: Vec<u8>,
    pub spawn: (f32, f32),
    pub objects: Vec<Obj>,
    pub exits: Vec<Exit>,
    /// Episode and slot within it (N levels only; the practice course has none).
    pub episode: Option<(u32, u32)>,
}

impl Level {
    /// Test helper: add objects from lines like `mine 300 600` or `door 100 640 0 0 20 25 1 0 0`.
    #[cfg(test)]
    pub fn add_test_objects(&mut self, text: &str) {
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let mut it = line.split_whitespace();
            let kind = match it.next().unwrap() {
                "door" => "testdoor",
                "oneway" => "onewayplatform",
                other => other,
            };
            let params: Vec<f32> = it.map(|v| v.parse().unwrap()).collect();
            self.objects.push(Obj { kind: kind.to_string(), x: params[0], y: params[1], params });
        }
    }

    pub fn clone_level(&self) -> Level {
        Level {
            label: self.label.clone(),
            w: self.w,
            h: self.h,
            tiles: self.tiles.clone(),
            spawn: self.spawn,
            objects: self.objects.iter().map(|o| Obj { kind: o.kind.clone(), x: o.x, y: o.y, params: o.params.clone() }).collect(),
            exits: self.exits.clone(),
            episode: self.episode,
        }
    }

    /// Text format: `#` full tile, `2`..`Q` N tile codes (`chr(48 + code)`), `S` spawn
    /// (ninja rests on the tile floor below it), `X` exit door and `K` its switch (paired
    /// in reading order), anything else empty. Short lines are
    /// padded with empty tiles.
    pub fn parse(src: &str) -> Level {
        let lines: Vec<&str> = src.lines().collect();
        let h = lines.len() as i32;
        let w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0) as i32;
        let mut tiles = vec![0u8; (w * h) as usize];
        let mut spawn = (TILE * 1.5, TILE * 1.5);
        let (mut doors, mut switches) = (Vec::new(), Vec::new());
        for (ty, line) in lines.iter().enumerate() {
            for (tx, ch) in line.chars().enumerate() {
                let i = ty * w as usize + tx;
                match ch {
                    '#' => tiles[i] = FULL,
                    'S' => spawn = ((tx as f32 + 0.5) * TILE, (ty as f32 + 1.0) * TILE - crate::ninja::RADIUS),
                    'X' => doors.push(((tx as f32 + 0.5) * TILE, (ty as f32 + 0.5) * TILE)),
                    'K' => switches.push(((tx as f32 + 0.5) * TILE, (ty as f32 + 0.5) * TILE)),
                    '2'..='Q' => tiles[i] = ch as u8 - b'0',
                    _ => {}
                }
            }
        }
        let exits = doors.iter().zip(&switches).map(|(&door, &switch)| Exit { door, switch }).collect();
        Level { label: "practice".into(), w, h, tiles, spawn, objects: Vec::new(), exits, episode: None }
    }

    /// Tile code. Out of bounds counts as full.
    pub fn code(&self, tx: i32, ty: i32) -> u8 {
        if tx < 0 || ty < 0 || tx >= self.w || ty >= self.h {
            return FULL;
        }
        self.tiles[(ty * self.w + tx) as usize]
    }

    /// A full tile with at least one non-full neighbour, or any partial tile (rendering).
    pub fn is_surface(&self, tx: i32, ty: i32) -> bool {
        match self.code(tx, ty) {
            0 => false,
            FULL => [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|(dx, dy)| self.code(tx + dx, ty + dy) != FULL),
            _ => true,
        }
    }

    /// True when the point is inside solid material.
    pub fn point_solid(&self, x: f32, y: f32) -> bool {
        let (tx, ty) = ((x / TILE).floor() as i32, (y / TILE).floor() as i32);
        match self.code(tx, ty) {
            0 => false,
            FULL => true,
            c => crate::shapes::inside(&shape(c).poly, x / TILE - tx as f32, y / TILE - ty as f32),
        }
    }
}

// ---- N levels, extracted by tools/extract_n.py ----

#[derive(Deserialize)]
struct NFile {
    levels: Vec<NLevel>,
}

#[derive(Deserialize)]
struct NLevel {
    name: String,
    episode: u32,
    slot: u32,
    tiles: Vec<String>,
    objects: Vec<NObj>,
}

#[derive(Deserialize)]
struct NObj {
    name: String,
    params: Vec<f64>,
}

/// N playfield: 31 x 23 tiles inside a one tile solid frame.
const N_COLS: usize = 31;
const N_ROWS: usize = 23;

pub fn n_levels() -> Vec<Level> {
    let file: NFile = serde_json::from_str(include_str!("../data/n-levels.json")).expect("data/n-levels.json");
    file.levels
        .into_iter()
        .map(|l| {
            let (w, h) = (N_COLS + 2, N_ROWS + 2);
            let mut tiles = vec![FULL; w * h];
            for (r, row) in l.tiles.iter().enumerate() {
                for (c, ch) in row.chars().enumerate() {
                    tiles[(r + 1) * w + c + 1] = (ch as u8).saturating_sub(b'0');
                }
            }
            let mut spawn = (TILE * 2.0, TILE * 2.0);
            let mut objects = Vec::new();
            let mut exits = Vec::new();
            for o in l.objects {
                let p: Vec<f32> = o.params.iter().map(|&v| v as f32).collect();
                if o.name == "player" {
                    spawn = (p[0], p[1]);
                } else if o.name == "exit" && p.len() >= 4 {
                    exits.push(Exit { door: (p[0], p[1]), switch: (p[2], p[3]) });
                } else {
                    objects.push(Obj { kind: o.name, x: p[0], y: p[1], params: p });
                }
            }
            Level {
                label: format!("{}:{} {}", l.episode, l.slot, l.name),
                w: w as i32,
                h: h as i32,
                tiles,
                spawn,
                objects,
                exits,
                episode: Some((l.episode, l.slot)),
            }
        })
        .collect()
}
