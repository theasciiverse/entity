use crate::shapes::shape;

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
pub const CAMPAIGN: &[(&str, &str)] = &[
    ("00-0.txt", include_str!("../levels/00-0.txt")),
    ("00-1.txt", include_str!("../levels/00-1.txt")),
    ("00-2.txt", include_str!("../levels/00-2.txt")),
    ("00-3.txt", include_str!("../levels/00-3.txt")),
    ("00-4.txt", include_str!("../levels/00-4.txt")),
    ("01-0.txt", include_str!("../levels/01-0.txt")),
    ("01-1.txt", include_str!("../levels/01-1.txt")),
    ("01-2.txt", include_str!("../levels/01-2.txt")),
    ("01-3.txt", include_str!("../levels/01-3.txt")),
    ("01-4.txt", include_str!("../levels/01-4.txt")),
    ("02-0.txt", include_str!("../levels/02-0.txt")),
    ("02-1.txt", include_str!("../levels/02-1.txt")),
    ("02-2.txt", include_str!("../levels/02-2.txt")),
    ("02-3.txt", include_str!("../levels/02-3.txt")),
    ("02-4.txt", include_str!("../levels/02-4.txt")),
    ("03-0.txt", include_str!("../levels/03-0.txt")),
    ("03-1.txt", include_str!("../levels/03-1.txt")),
    ("03-2.txt", include_str!("../levels/03-2.txt")),
    ("03-3.txt", include_str!("../levels/03-3.txt")),
    ("03-4.txt", include_str!("../levels/03-4.txt")),
    ("04-0.txt", include_str!("../levels/04-0.txt")),
    ("04-1.txt", include_str!("../levels/04-1.txt")),
    ("04-2.txt", include_str!("../levels/04-2.txt")),
    ("04-3.txt", include_str!("../levels/04-3.txt")),
    ("04-4.txt", include_str!("../levels/04-4.txt")),
    ("05-0.txt", include_str!("../levels/05-0.txt")),
    ("05-1.txt", include_str!("../levels/05-1.txt")),
    ("05-2.txt", include_str!("../levels/05-2.txt")),
    ("05-3.txt", include_str!("../levels/05-3.txt")),
    ("05-4.txt", include_str!("../levels/05-4.txt")),
    ("06-0.txt", include_str!("../levels/06-0.txt")),
    ("06-1.txt", include_str!("../levels/06-1.txt")),
    ("06-2.txt", include_str!("../levels/06-2.txt")),
    ("06-3.txt", include_str!("../levels/06-3.txt")),
    ("06-4.txt", include_str!("../levels/06-4.txt")),
    ("07-0.txt", include_str!("../levels/07-0.txt")),
    ("07-1.txt", include_str!("../levels/07-1.txt")),
    ("07-2.txt", include_str!("../levels/07-2.txt")),
    ("07-3.txt", include_str!("../levels/07-3.txt")),
    ("07-4.txt", include_str!("../levels/07-4.txt")),
    ("08-0.txt", include_str!("../levels/08-0.txt")),
    ("08-1.txt", include_str!("../levels/08-1.txt")),
    ("08-2.txt", include_str!("../levels/08-2.txt")),
    ("08-3.txt", include_str!("../levels/08-3.txt")),
    ("08-4.txt", include_str!("../levels/08-4.txt")),
    ("09-0.txt", include_str!("../levels/09-0.txt")),
    ("09-1.txt", include_str!("../levels/09-1.txt")),
    ("09-2.txt", include_str!("../levels/09-2.txt")),
    ("09-3.txt", include_str!("../levels/09-3.txt")),
    ("09-4.txt", include_str!("../levels/09-4.txt")),
    ("10-0.txt", include_str!("../levels/10-0.txt")),
    ("10-1.txt", include_str!("../levels/10-1.txt")),
    ("10-2.txt", include_str!("../levels/10-2.txt")),
    ("10-3.txt", include_str!("../levels/10-3.txt")),
    ("10-4.txt", include_str!("../levels/10-4.txt")),
    ("11-0.txt", include_str!("../levels/11-0.txt")),
    ("11-1.txt", include_str!("../levels/11-1.txt")),
    ("11-2.txt", include_str!("../levels/11-2.txt")),
    ("11-3.txt", include_str!("../levels/11-3.txt")),
    ("11-4.txt", include_str!("../levels/11-4.txt")),
    ("12-0.txt", include_str!("../levels/12-0.txt")),
    ("12-1.txt", include_str!("../levels/12-1.txt")),
    ("12-2.txt", include_str!("../levels/12-2.txt")),
    ("12-3.txt", include_str!("../levels/12-3.txt")),
    ("12-4.txt", include_str!("../levels/12-4.txt")),
    ("13-0.txt", include_str!("../levels/13-0.txt")),
    ("13-1.txt", include_str!("../levels/13-1.txt")),
    ("13-2.txt", include_str!("../levels/13-2.txt")),
    ("13-3.txt", include_str!("../levels/13-3.txt")),
    ("13-4.txt", include_str!("../levels/13-4.txt")),
    ("14-0.txt", include_str!("../levels/14-0.txt")),
    ("14-1.txt", include_str!("../levels/14-1.txt")),
    ("14-2.txt", include_str!("../levels/14-2.txt")),
    ("14-3.txt", include_str!("../levels/14-3.txt")),
    ("14-4.txt", include_str!("../levels/14-4.txt")),
    ("15-0.txt", include_str!("../levels/15-0.txt")),
    ("15-1.txt", include_str!("../levels/15-1.txt")),
    ("15-2.txt", include_str!("../levels/15-2.txt")),
    ("15-3.txt", include_str!("../levels/15-3.txt")),
    ("15-4.txt", include_str!("../levels/15-4.txt")),
    ("16-0.txt", include_str!("../levels/16-0.txt")),
    ("16-1.txt", include_str!("../levels/16-1.txt")),
    ("16-2.txt", include_str!("../levels/16-2.txt")),
    ("16-3.txt", include_str!("../levels/16-3.txt")),
    ("16-4.txt", include_str!("../levels/16-4.txt")),
    ("17-0.txt", include_str!("../levels/17-0.txt")),
    ("17-1.txt", include_str!("../levels/17-1.txt")),
    ("17-2.txt", include_str!("../levels/17-2.txt")),
    ("17-3.txt", include_str!("../levels/17-3.txt")),
    ("17-4.txt", include_str!("../levels/17-4.txt")),
    ("18-0.txt", include_str!("../levels/18-0.txt")),
    ("18-1.txt", include_str!("../levels/18-1.txt")),
    ("18-2.txt", include_str!("../levels/18-2.txt")),
    ("18-3.txt", include_str!("../levels/18-3.txt")),
    ("18-4.txt", include_str!("../levels/18-4.txt")),
    ("19-0.txt", include_str!("../levels/19-0.txt")),
    ("19-1.txt", include_str!("../levels/19-1.txt")),
    ("19-2.txt", include_str!("../levels/19-2.txt")),
    ("19-3.txt", include_str!("../levels/19-3.txt")),
    ("19-4.txt", include_str!("../levels/19-4.txt")),
];

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

/// The playfield: 31 x 23 tiles inside a one tile solid frame, as in N.
const N_COLS: usize = 31;
const N_ROWS: usize = 23;

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
    fn every_campaign_level_survives_a_save_and_load() {
        for l in campaign() {
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

    /// Every campaign level carries a route that finishes it in the real game, with the switch,
    /// without dying and collecting all of its gold; the episode totals are printed against
    /// the 90 s clock (`cargo test --release campaign -- --nocapture`).
    #[test]
    fn every_campaign_level_is_finished_by_its_route() {
        let levels = campaign();
        let mut episode = (0.0, 0);
        for l in &levels {
            let route = l.route.as_deref().unwrap_or_else(|| panic!("{}: no route (run entity --prove)", l.label));
            let g = crate::demo::replay(l, route);
            assert!(g.complete, "{}: the route does not finish the level", l.label);
            assert_eq!(g.deaths, 0, "{}: the route dies", l.label);
            assert_eq!(g.gold_collected(), g.gold_total(), "{}: the route leaves gold behind", l.label);
            let secs = g.tick as f32 / 60.0;
            println!("{:28} {secs:5.1} s  gold {:2}", l.label, g.gold_total());
            episode = (episode.0 + secs, episode.1 + g.gold_total());
            if l.episode.is_some_and(|(_, s)| s == 4) {
                let net = episode.0 - 2.0 * episode.1 as f32;
                println!("  episode: {:.1} s, {} gold, {net:.1} s of the 90 s clock", episode.0, episode.1);
                assert!(net < 45.0, "episode {:?} uses {net:.1} s of the clock at the bot's pace", l.episode);
                episode = (0.0, 0);
            }
        }
    }

    /// How much each level's hazards act while its route plays: ticks with a laser or chaingun
    /// charging or firing, a turret targeting, a rocket flying, a thwomp moving. A hazard that
    /// never acts on the route is scenery (`cargo test --release hazards_on_the_routes -- --ignored --nocapture`).
    #[test]
    #[ignore]
    fn hazards_on_the_routes() {
        use crate::game::Game;
        for l in campaign() {
            let route = l.route.as_deref().unwrap_or("");
            let mut game = Game::new(vec![l.clone_level()], 0);
            let mut demo = crate::demo::Demo::new(route);
            let (mut aim, mut fire, mut turret, mut rocket, mut thwomp) = (0, 0, 0, 0, 0);
            for _ in 0..crate::demo::decode(route).len() * 6 {
                let c = demo.controls(!game.ninja.grounded);
                game.update(&c);
                let w = &game.world;
                aim += w.drones.iter().filter(|d| d.charging()).count();
                fire += w.drones.iter().filter(|d| d.firing()).count();
                turret += w.turrets.iter().filter(|t| t.targeting()).count();
                rocket += w.launchers.iter().filter(|r| r.flying()).count();
                thwomp += w.thwomps.iter().filter(|t| t.moving()).count();
                if game.complete {
                    break;
                }
            }
            if aim + fire + turret + rocket + thwomp > 0 || !game.world.drones.is_empty() {
                println!("{:26} drones {} aim {aim:4} fire {fire:4} turret {turret:4} rocket {rocket:4} thwomp {thwomp:4}", l.label, game.world.drones.len());
            }
        }
    }

    #[test]
    fn every_level_file_is_in_the_campaign() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("levels");
        for f in std::fs::read_dir(dir).unwrap() {
            let name = f.unwrap().file_name().into_string().unwrap();
            if name != "001.txt" && name.ends_with(".txt") {
                assert!(CAMPAIGN.iter().any(|(n, _)| *n == name), "levels/{name} is not in CAMPAIGN");
            }
        }
        for (i, (name, _)) in CAMPAIGN.iter().enumerate() {
            assert_eq!(*name, format!("{:02}-{}.txt", i / 5, i % 5), "campaign order");
        }
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
