//! Single-cell glyph art for the board.
//!
//! The playfield (31 x 23 tiles of 24 units) is divided into an `n` x `m` grid of
//! terminal cells; `n` and `m` come from the terminal size (see `fit`), so a cell is
//! `cw` x `ch` world units and need not line up with tile edges. Each cell is matched
//! against a palette of block-element glyphs (lower and left eighths, quadrants, plus
//! their colour-inverted forms) by sampling the level at 8x8 points and picking the
//! glyph with the nearest coverage.

use crate::level::{Level, FULL, TILE};
use std::sync::OnceLock;

/// Smallest board (frame included) the game is drawn at.
pub const MIN_BOARD: (u16, u16) = (64, 25);

/// Inner cell grid of the playfield; the drawn board is two cells larger each way
/// (the frame line hugs the playfield).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geometry {
    pub n: i32,
    pub m: i32,
}

impl Geometry {
    pub fn board(&self) -> (u16, u16) {
        ((self.n + 2) as u16, (self.m + 2) as u16)
    }

    /// World units per cell.
    pub fn cell_size(&self, level: &Level) -> (f32, f32) {
        (((level.w - 2) as f32 * TILE) / self.n as f32, ((level.h - 2) as f32 * TILE) / self.m as f32)
    }
}

/// Largest board that fits a `cols` x `rows` terminal with square tiles, given the
/// cell aspect (cell height / cell width in pixels, about 2). None when the terminal
/// is below `MIN_BOARD`.
pub fn fit(level: &Level, cols: u16, rows: u16, aspect: f32) -> Option<Geometry> {
    if cols < MIN_BOARD.0 || rows < MIN_BOARD.1 {
        return None;
    }
    let (w, h) = ((level.w - 2) as f32 * TILE, (level.h - 2) as f32 * TILE);
    let max_n = cols as i32 - 2;
    let max_m = rows as i32 - 2;
    let n_from_height = ((max_m as f32) * aspect * w / h + 1e-3).floor() as i32;
    let n = n_from_height.max(MIN_BOARD.0 as i32 - 2).min(max_n);
    let m = ((n as f32) * h / (aspect * w)).round() as i32;
    Some(Geometry { n, m: m.clamp(20, max_m) })
}

/// How the board is fitted to the terminal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FitMode {
    /// Fill the terminal, leaving one cell of margin all round. Tiles are stretched wider or
    /// taller than square when the terminal's shape differs from the level's, by at most 30%.
    Fill,
    /// Square tiles; the board is centred and may leave wide margins.
    Square,
}

impl FitMode {
    /// `ENTITY_FIT=square` starts in square mode; the default is to fill.
    pub fn from_env() -> FitMode {
        if std::env::var("ENTITY_FIT").is_ok_and(|v| v == "square") {
            FitMode::Square
        } else {
            FitMode::Fill
        }
    }
}

/// Largest stretch of a tile from square that `FitMode::Fill` allows.
const MAX_STRETCH: f32 = 1.3;

pub fn fit_with(mode: FitMode, level: &Level, cols: u16, rows: u16, aspect: f32) -> Option<Geometry> {
    match mode {
        FitMode::Square => fit(level, cols, rows, aspect),
        FitMode::Fill => fit_fill(level, cols, rows, aspect),
    }
}

fn fit_fill(level: &Level, cols: u16, rows: u16, aspect: f32) -> Option<Geometry> {
    if cols < MIN_BOARD.0 || rows < MIN_BOARD.1 {
        return None;
    }
    // one cell of margin all round when the terminal has room for it
    let margin = if cols >= MIN_BOARD.0 + 2 && rows >= MIN_BOARD.1 + 2 { 1 } else { 0 };
    let mut n = cols as i32 - 2 * margin - 2;
    let mut m = rows as i32 - 2 * margin - 2;
    // on screen a tile is (n / tiles across) cells wide and (m / tiles down) cells tall
    let (tw, th) = ((level.w - 2) as f32, (level.h - 2) as f32);
    let wide = |n: i32, m: i32| (n as f32 / tw) / (m as f32 / th) / aspect;
    if wide(n, m) > MAX_STRETCH {
        n = ((MAX_STRETCH * m as f32 * tw * aspect) / th).floor() as i32;
    } else if wide(n, m) < 1.0 / MAX_STRETCH {
        m = ((n as f32 * th) / (tw * aspect / MAX_STRETCH * 1.0)).floor() as i32;
        m = m.min(rows as i32 - 2 * margin - 2);
    }
    Some(Geometry { n: n.max(MIN_BOARD.0 as i32 - 2), m: m.max(20) })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell {
    pub ch: char,
    /// Draw with foreground and background swapped (solid is the background).
    pub inverted: bool,
    /// Full tile buried in solid: drawn dim.
    pub interior: bool,
    /// Braille dots (2x4) of the solid part, to merge under the ninja.
    pub dots: u8,
}

impl Cell {
    const EMPTY: Cell = Cell { ch: ' ', inverted: false, interior: false, dots: 0 };
}

pub struct Art {
    pub geo: Geometry,
    cells: Vec<Cell>,
}

impl Art {
    /// Cell of the inner grid (column 0..n, row 0..m).
    pub fn cell(&self, ic: i32, ir: i32) -> Cell {
        if ic < 0 || ir < 0 || ic >= self.geo.n || ir >= self.geo.m {
            return Cell::EMPTY;
        }
        self.cells[(ir * self.geo.n + ic) as usize]
    }
}

struct Glyph {
    ch: char,
    inverted: bool,
    mask: u64,
}

fn bit(r: usize, c: usize) -> u64 {
    1u64 << (r * 8 + c)
}

fn region(f: impl Fn(usize, usize) -> bool) -> u64 {
    let mut m = 0;
    for r in 0..8 {
        for c in 0..8 {
            if f(r, c) {
                m |= bit(r, c);
            }
        }
    }
    m
}

fn palette() -> &'static [Glyph] {
    static P: OnceLock<Vec<Glyph>> = OnceLock::new();
    P.get_or_init(|| {
        let mut base: Vec<(char, u64)> = Vec::new();
        for (i, ch) in "\u{2581}\u{2582}\u{2583}\u{2584}\u{2585}\u{2586}\u{2587}".chars().enumerate() {
            base.push((ch, region(|r, _| r >= 7 - i)));
        }
        for (i, ch) in "\u{258F}\u{258E}\u{258D}\u{258C}\u{258B}\u{258A}\u{2589}".chars().enumerate() {
            base.push((ch, region(|_, c| c <= i)));
        }
        let ul = |r: usize, c: usize| r < 4 && c < 4;
        let ur = |r: usize, c: usize| r < 4 && c >= 4;
        let ll = |r: usize, c: usize| r >= 4 && c < 4;
        let lr = |r: usize, c: usize| r >= 4 && c >= 4;
        base.push(('\u{2598}', region(ul)));
        base.push(('\u{259D}', region(ur)));
        base.push(('\u{2596}', region(ll)));
        base.push(('\u{2597}', region(lr)));
        base.push(('\u{259A}', region(|r, c| ul(r, c) || lr(r, c))));
        base.push(('\u{259E}', region(|r, c| ur(r, c) || ll(r, c))));
        base.push(('\u{2599}', region(|r, c| ul(r, c) || ll(r, c) || lr(r, c))));
        base.push(('\u{259B}', region(|r, c| ul(r, c) || ur(r, c) || ll(r, c))));
        base.push(('\u{259C}', region(|r, c| ul(r, c) || ur(r, c) || lr(r, c))));
        base.push(('\u{259F}', region(|r, c| ur(r, c) || ll(r, c) || lr(r, c))));
        let mut p = vec![Glyph { ch: ' ', inverted: false, mask: 0 }, Glyph { ch: '\u{2588}', inverted: false, mask: !0 }];
        for &(ch, mask) in &base {
            p.push(Glyph { ch, inverted: false, mask });
        }
        for &(ch, mask) in &base {
            p.push(Glyph { ch, inverted: true, mask: !mask });
        }
        p
    })
}

/// Best palette glyph for a coverage mask, and its error in samples (of 64).
pub fn best_glyph(mask: u64) -> (char, bool, u32) {
    let g = palette().iter().min_by_key(|g| (g.mask ^ mask).count_ones()).unwrap();
    (g.ch, g.inverted, (g.mask ^ mask).count_ones())
}

pub fn build(level: &Level, geo: Geometry) -> Art {
    const BIT: [[u8; 4]; 2] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];
    let (cw, ch) = geo.cell_size(level);
    let mut cells = Vec::with_capacity((geo.n * geo.m) as usize);
    for ir in 0..geo.m {
        for ic in 0..geo.n {
            let (x0, y0) = (TILE + ic as f32 * cw, TILE + ir as f32 * ch);
            let mut mask = 0u64;
            for r in 0..8 {
                for c in 0..8 {
                    if level.point_solid(x0 + (c as f32 + 0.5) / 8.0 * cw, y0 + (r as f32 + 0.5) / 8.0 * ch) {
                        mask |= bit(r, c);
                    }
                }
            }
            let mut dots = 0u8;
            for (a, col) in BIT.iter().enumerate() {
                for (b, &d) in col.iter().enumerate() {
                    if level.point_solid(x0 + (a as f32 + 0.5) / 2.0 * cw, y0 + (b as f32 + 0.5) / 4.0 * ch) {
                        dots |= d;
                    }
                }
            }
            cells.push(if mask == 0 {
                Cell::EMPTY
            } else if mask == !0 {
                let (tx, ty) = (((x0 + cw / 2.0) / TILE).floor() as i32, ((y0 + ch / 2.0) / TILE).floor() as i32);
                if level.code(tx, ty) == FULL && !level.is_surface(tx, ty) {
                    Cell { ch: '\u{2592}', inverted: false, interior: true, dots }
                } else {
                    Cell { ch: '\u{2588}', inverted: false, interior: false, dots }
                }
            } else {
                let (ch, inverted, _) = best_glyph(mask);
                Cell { ch, inverted, interior: false, dots }
            });
        }
    }
    Art { geo, cells }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::n_levels;

    #[test]
    fn exact_shapes_get_exact_glyphs() {
        assert_eq!(best_glyph(!0), ('\u{2588}', false, 0));
        assert_eq!(best_glyph(0), (' ', false, 0));
        assert_eq!(best_glyph(region(|r, _| r >= 4)).2, 0);
    }

    #[test]
    fn fit_gives_square_tiles_and_respects_the_terminal() {
        let lv = &n_levels()[0];
        let g = fit(lv, 100, 30, 2.18).unwrap();
        assert_eq!(g.board(), (84, 30), "100x30 with 2.18 cells: width is the spare, not the limit");
        let g = fit(lv, 156, 47, 2.18).unwrap();
        assert_eq!(g.board(), (134, 47));
        let (cw, ch) = g.cell_size(lv);
        assert!((ch / cw - 2.18).abs() < 0.08, "cell {cw} x {ch} world units");
        assert_eq!(fit(lv, 64, 25, 2.0).unwrap().board(), (64, 25));
        assert!(fit(lv, 63, 25, 2.0).is_none() && fit(lv, 64, 24, 2.0).is_none());
        // very wide, short terminal: height limits
        assert_eq!(fit(lv, 300, 30, 2.0).unwrap().board().1, 30);
        // never larger than the terminal
        for (c, r) in [(64, 25), (80, 40), (200, 50), (90, 90)] {
            let b = fit(lv, c, r, 2.0).unwrap().board();
            assert!(b.0 <= c && b.1 <= r, "{b:?} in {c}x{r}");
        }
    }

    #[test]
    fn fill_mode_reaches_one_cell_from_every_edge_within_the_stretch_limit() {
        let lv = &n_levels()[0];
        // David's window: 156 x 47 cells of 7.9 x 17.2 px
        let g = fit_with(FitMode::Fill, lv, 156, 47, 2.18).unwrap();
        let (bw, bh) = g.board();
        assert_eq!((bw, bh), (154, 45), "one cell of margin on every side");
        let wide = (g.n as f32 / 31.0) / (g.m as f32 / 23.0) / 2.18;
        assert!(wide > 1.0 && wide <= 1.3 + 1e-3, "tiles are {wide:.2} times wider than tall");
        // a terminal exactly the minimum size has no room for a margin
        assert_eq!(fit_with(FitMode::Fill, lv, 64, 25, 2.0).unwrap().board(), (64, 25));
        assert!(fit_with(FitMode::Fill, lv, 63, 25, 2.0).is_none());
        // a very wide terminal does not stretch tiles past the limit
        let g = fit_with(FitMode::Fill, lv, 400, 40, 2.0).unwrap();
        let wide = (g.n as f32 / 31.0) / (g.m as f32 / 23.0) / 2.0;
        assert!(wide <= 1.3 + 1e-3, "{wide}");
        assert!(g.board().0 <= 400 && g.board().1 <= 40);
        // square mode is unchanged
        assert_eq!(fit_with(FitMode::Square, lv, 156, 47, 2.18).unwrap().board(), (134, 47));
    }

    #[test]
    fn glyph_error_across_all_levels_and_sizes() {
        // Sum error over every partly filled cell: how far the nearest glyph is from the
        // true coverage, in samples of 64. Reported, with a loose ceiling.
        let levels = n_levels();
        let mut worst = 0u32;
        let mut total = 0u64;
        let mut cells = 0u64;
        for geo in [Geometry { n: 62, m: 23 }, Geometry { n: 84, m: 30 }, Geometry { n: 124, m: 46 }] {
            let l = &levels[5];
            let (cw, ch) = geo.cell_size(l);
            for ir in 0..geo.m {
                for ic in 0..geo.n {
                    let (x0, y0) = (TILE + ic as f32 * cw, TILE + ir as f32 * ch);
                    let mask = region(|r, c| {
                        l.point_solid(x0 + (c as f32 + 0.5) / 8.0 * cw, y0 + (r as f32 + 0.5) / 8.0 * ch)
                    });
                    if mask != 0 && mask != !0 {
                        let e = best_glyph(mask).2;
                        worst = worst.max(e);
                        total += e as u64;
                        cells += 1;
                    }
                }
            }
        }
        println!("partial cells {cells}, mean error {:.1}, worst {worst} (of 64 samples)", total as f32 / cells as f32);
        assert!(worst <= 20, "worst {worst}");
        assert!((total as f32 / cells as f32) < 8.0);
    }
}
