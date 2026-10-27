use crate::shapes::shape;
use serde::Deserialize;

/// World units follow N++: one tile is 24 units, the ninja radius is 10.
pub const TILE: f32 = 24.0;

pub const FULL: u8 = 1;
/// The tile code whose grid letter would be `K`, the switch; it is written `k`.
const TILE_K: u8 = 27;

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
    /// The level's own name (the label adds the episode and slot in the campaign).
    pub name: String,
    pub w: i32,
    pub h: i32,
    tiles: Vec<u8>,
    pub spawn: (f32, f32),
    pub objects: Vec<Obj>,
    pub exits: Vec<Exit>,
    /// Episode and slot within it (campaign and N levels; the practice course has none).
    pub episode: Option<(u32, u32)>,
    /// Waypoints for the route bot, `x,y;x,y|x,y` in px (see `bot::waypoint_route`).
    pub waypoints: Option<String>,
    /// A route that finishes the level, as `demo` plays it (found by `entity --prove`).
    pub route: Option<String>,
}

impl Level {
    /// Test helper: add objects from lines like `mine 300 600` or `door 100 640 0 0 20 25 1 0 0`.
    #[cfg(test)]
    pub fn add_test_objects(&mut self, text: &str) {
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            self.objects.push(object_line(line).expect("test object"));
        }
    }

    pub fn clone_level(&self) -> Level {
        Level {
            label: self.label.clone(),
            name: self.name.clone(),
            w: self.w,
            h: self.h,
            tiles: self.tiles.clone(),
            spawn: self.spawn,
            objects: self.objects.iter().map(|o| Obj { kind: o.kind.clone(), x: o.x, y: o.y, params: o.params.clone() }).collect(),
            exits: self.exits.clone(),
            episode: self.episode,
            waypoints: self.waypoints.clone(),
            route: self.route.clone(),
        }
    }

    /// Text format. First the tile grid: `#` full tile, `2`..`Q` N tile codes
    /// (`chr(48 + code)`, except 27, which is `k` because `K` is the switch), `S` spawn (ninja rests on the tile floor below it), `X` exit door
    /// and `K` its switch (paired in reading order), anything else empty. Short lines are
    /// padded with empty tiles. The grid ends at the first empty line; after it come
    /// lines of `key values`: `name <text>`, `spawn x y`, `exit dx dy sx sy`, objects in
    /// the original's parameter layout (`gold x y`, `mine x y`, `drone x y movement chaser
    /// weapon dir`, `door ...`, see "Objects" in CLAUDE.md), `waypoints` and `route` for
    /// the route bot. Positions are px, frame included. `;` starts a comment line.
    pub fn parse(src: &str) -> Level {
        let all: Vec<&str> = src.lines().collect();
        let split = all.iter().position(|l| l.trim().is_empty()).unwrap_or(all.len());
        let (lines, extra) = (&all[..split], &all[split..]);
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
                    'S' => spawn = spawn_on(tx as i32, ty as i32),
                    'X' => doors.push(centre(tx as i32, ty as i32)),
                    'K' => switches.push(centre(tx as i32, ty as i32)),
                    'k' => tiles[i] = TILE_K,
                    '2'..='Q' => tiles[i] = ch as u8 - b'0',
                    _ => {}
                }
            }
        }
        let mut exits: Vec<Exit> = doors.iter().zip(&switches).map(|(&door, &switch)| Exit { door, switch }).collect();
        let mut level = Level {
            label: String::new(),
            name: String::new(),
            w,
            h,
            tiles,
            spawn,
            objects: Vec::new(),
            exits: Vec::new(),
            episode: None,
            waypoints: None,
            route: None,
        };
        for line in extra.iter().map(|l| l.trim()).filter(|l| !l.is_empty() && !l.starts_with(';')) {
            let (key, rest) = line.split_once(char::is_whitespace).map_or((line, ""), |(k, r)| (k, r.trim()));
            let nums: Vec<f32> = rest.split_whitespace().filter_map(|v| v.parse().ok()).collect();
            match key {
                "name" => level.name = rest.to_string(),
                "waypoints" => level.waypoints = Some(rest.to_string()),
                "route" => level.route = Some(rest.to_string()),
                "spawn" if nums.len() >= 2 => level.spawn = (nums[0], nums[1]),
                "exit" if nums.len() >= 4 => exits.push(Exit { door: (nums[0], nums[1]), switch: (nums[2], nums[3]) }),
                _ => match object_line(line) {
                    Some(o) => level.objects.push(o),
                    None => log::warn!("level line not understood: {line}"),
                },
            }
        }
        level.exits = exits;
        level.label = if level.name.is_empty() { "untitled".into() } else { level.name.clone() };
        level
    }

    /// The level in the text format `parse` reads; `parse(to_text())` gives the same level.
    pub fn to_text(&self) -> String {
        let mut grid: Vec<Vec<char>> = (0..self.h)
            .map(|ty| {
                (0..self.w)
                    .map(|tx| match self.code(tx, ty) {
                        0 => ' ',
                        FULL => '#',
                        TILE_K => 'k',
                        c => (b'0' + c) as char,
                    })
                    .collect()
            })
            .collect();
        let mut extra = Vec::new();
        if !self.name.is_empty() {
            extra.push(format!("name {}", self.name));
        }
        // spawn, door and switch go in the grid when they sit where its letters put them
        let free = |grid: &Vec<Vec<char>>, (tx, ty): (i32, i32)| tx >= 0 && ty >= 0 && tx < self.w && ty < self.h && grid[ty as usize][tx as usize] == ' ';
        let st = tile_of(self.spawn);
        if spawn_on(st.0, st.1) == self.spawn && free(&grid, st) {
            grid[st.1 as usize][st.0 as usize] = 'S';
        } else {
            extra.push(format!("spawn {} {}", self.spawn.0, self.spawn.1));
        }
        let single = self.exits.len() == 1 && {
            let (e, (dt, kt)) = (self.exits[0], (tile_of(self.exits[0].door), tile_of(self.exits[0].switch)));
            centre(dt.0, dt.1) == e.door && centre(kt.0, kt.1) == e.switch && dt != kt && free(&grid, dt) && free(&grid, kt)
        };
        if single {
            let (dt, kt) = (tile_of(self.exits[0].door), tile_of(self.exits[0].switch));
            grid[dt.1 as usize][dt.0 as usize] = 'X';
            grid[kt.1 as usize][kt.0 as usize] = 'K';
        } else {
            for e in &self.exits {
                extra.push(format!("exit {} {} {} {}", e.door.0, e.door.1, e.switch.0, e.switch.1));
            }
        }
        for o in &self.objects {
            let kind = match o.kind.as_str() {
                "testdoor" => "door",
                "onewayplatform" => "oneway",
                k => k,
            };
            let params: Vec<String> = o.params.iter().map(|v| v.to_string()).collect();
            extra.push(format!("{kind} {}", params.join(" ")));
        }
        if let Some(w) = &self.waypoints {
            extra.push(format!("waypoints {w}"));
        }
        if let Some(r) = &self.route {
            extra.push(format!("route {r}"));
        }
        let mut out: String = grid.iter().map(|row| row.iter().collect::<String>() + "\n").collect();
        if !extra.is_empty() {
            out.push('\n');
            for l in extra {
                out.push_str(&l);
                out.push('\n');
            }
        }
        out
    }

    /// Set a tile code (editor); out of bounds is ignored.
    pub fn set_code(&mut self, tx: i32, ty: i32, code: u8) {
        if tx >= 0 && ty >= 0 && tx < self.w && ty < self.h {
            self.tiles[(ty * self.w + tx) as usize] = code;
        }
    }

    /// An empty 33 x 25 level inside a one tile solid frame, spawn bottom left (editor).
    pub fn blank() -> Level {
        let (w, h) = (N_COLS as i32 + 2, N_ROWS as i32 + 2);
        let mut l = Level::parse(&(0..h).map(|ty| if ty == 0 || ty == h - 1 { "#".repeat(w as usize) } else { format!("#{}#", " ".repeat(w as usize - 2)) }).collect::<Vec<_>>().join("\n"));
        l.spawn = spawn_on(2, h - 2);
        l
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

/// Centre of a tile, in px.
pub fn centre(tx: i32, ty: i32) -> (f32, f32) {
    ((tx as f32 + 0.5) * TILE, (ty as f32 + 0.5) * TILE)
}

/// The tile a point lies in.
pub fn tile_of(p: (f32, f32)) -> (i32, i32) {
    ((p.0 / TILE).floor() as i32, (p.1 / TILE).floor() as i32)
}

/// The spawn point of an `S` in tile (tx, ty): resting on the floor of that tile.
pub fn spawn_on(tx: i32, ty: i32) -> (f32, f32) {
    ((tx as f32 + 0.5) * TILE, (ty as f32 + 1.0) * TILE - crate::ninja::RADIUS)
}

/// One object line, `kind p0 p1 ...` (the original's parameter layout; `door` and `oneway`
/// stand for its `testdoor` and `onewayplatform`). None when it is not one.
fn object_line(line: &str) -> Option<Obj> {
    let mut it = line.split_whitespace();
    let kind = match it.next()? {
        "door" => "testdoor",
        "oneway" => "onewayplatform",
        k @ ("gold" | "mine" | "drone" | "turret" | "floorguard" | "thwomp" | "hominglauncher" | "launchpad" | "bounceblock" | "testdoor" | "onewayplatform") => k,
        _ => return None,
    };
    let params: Vec<f32> = it.map(|v| v.parse().ok()).collect::<Option<_>>()?;
    (params.len() >= 2).then(|| Obj { kind: kind.to_string(), x: params[0], y: params[1], params })
}

// ---- the campaign: Entity's own levels ----

/// Entity's own levels in campaign order, five per episode (`levels/EE-S.txt`, episode and
/// slot from 0). Add new files here; a test checks that none in `levels/` is left out.
pub const CAMPAIGN: &[(&str, &str)] = &[];

pub fn campaign() -> Vec<Level> {
    CAMPAIGN
        .iter()
        .enumerate()
        .map(|(i, (_, src))| {
            let mut l = Level::parse(src);
            let (e, s) = ((i / 5) as u32, (i % 5) as u32);
            l.label = format!("{e}:{s} {}", l.name);
            l.episode = Some((e, s));
            l
        })
        .collect()
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
                name: l.name,
                w: w as i32,
                h: h as i32,
                tiles,
                spawn,
                objects,
                exits,
                episode: Some((l.episode, l.slot)),
                waypoints: None,
                route: None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn same(a: &Level, b: &Level) {
        assert_eq!((a.w, a.h), (b.w, b.h));
        assert_eq!(a.tiles, b.tiles);
        assert_eq!(a.spawn, b.spawn);
        assert_eq!(a.name, b.name);
        assert_eq!(a.exits.len(), b.exits.len());
        for (x, y) in a.exits.iter().zip(&b.exits) {
            assert_eq!((x.door, x.switch), (y.door, y.switch));
        }
        assert_eq!(a.objects.len(), b.objects.len());
        for (x, y) in a.objects.iter().zip(&b.objects) {
            assert_eq!((&x.kind, x.x, x.y, &x.params), (&y.kind, y.x, y.y, &y.params));
        }
        assert_eq!((&a.waypoints, &a.route), (&b.waypoints, &b.route));
    }

    #[test]
    fn the_practice_course_saves_as_it_was_written() {
        let src = include_str!("../levels/001.txt");
        let l = Level::parse(src);
        assert_eq!(l.label, "practice");
        assert_eq!(l.to_text(), src, "the canonical text is the file itself");
    }

    #[test]
    fn every_n_level_survives_a_save_and_load() {
        for l in n_levels() {
            let text = l.to_text();
            same(&l, &Level::parse(&text));
            assert_eq!(Level::parse(&text).to_text(), text, "{}: saving twice changes nothing", l.label);
        }
    }

    #[test]
    fn objects_names_and_routes_round_trip() {
        let mut l = Level::blank();
        l.name = "a test".into();
        l.set_code(5, 20, 3);
        l.exits.push(Exit { door: centre(10, 22), switch: centre(4, 22) });
        l.exits.push(Exit { door: (100.0, 101.0), switch: centre(6, 6) });
        l.add_test_objects("gold 300 612\nlaunchpad 36 576 0.70710677 -0.70710677\ndoor 204 204 0 0 1 14 1 0 0\noneway 180 156 0");
        l.waypoints = Some("10,20;30,40|50,60".into());
        l.route = Some("R.RjL._.".into());
        let back = Level::parse(&l.to_text());
        same(&l, &back);
        assert_eq!(back.objects[2].kind, "testdoor");
        assert_eq!(back.objects[3].kind, "onewayplatform");
    }

    #[test]
    fn spawn_door_and_switch_use_the_grid_letters_when_they_can() {
        let mut l = Level::blank();
        l.exits.push(Exit { door: centre(30, 23), switch: centre(15, 23) });
        let text = l.to_text();
        assert!(text.contains('S') && text.contains('X') && text.contains('K'), "{text}");
        assert!(!text.contains("exit ") && !text.contains("spawn "));
        l.spawn = (50.0, 60.0);
        assert!(l.to_text().contains("spawn 50 60"));
    }
}
