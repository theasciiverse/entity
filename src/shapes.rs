//! Solid polygon of each N tile code, in unit tile coordinates (x right, y down).
//!
//! Tile ids come from the TID_* constants in the original SWF: 0 empty, 1 full,
//! 2..5 45 degree, 6..9 concave, 10..13 convex, 14..21 22 degree (small, big),
//! 22..29 67 degree (small, big), 30..33 half tiles (d, r, u, l). The letter
//! suffixes (pn, nn, np, pp) are the signs of the surface normal. Orientation and
//! the small/big naming were checked against all 150 levels: slopes line up
//! across tile borders (1249 matches, 1 mismatch) and slopes in the bottom row
//! sit on the frame floor (505 of 524).

use std::sync::OnceLock;

pub type Pt = (f32, f32);

const EPS: f32 = 1e-4;
const ARC_STEPS: usize = 6;

/// Border sides: 0 left, 1 right, 2 top, 3 bottom.
pub const LEFT: usize = 0;
pub const RIGHT: usize = 1;
pub const TOP: usize = 2;
pub const BOTTOM: usize = 3;

pub struct Shape {
    pub poly: Vec<Pt>,
    /// True when the polygon has an edge covering the whole border side.
    pub side_full: [bool; 4],
}

fn build(code: u8) -> Vec<Pt> {
    match code {
        0 => vec![],
        1 => vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)],
        30 => vec![(0.0, 0.5), (1.0, 0.5), (1.0, 1.0), (0.0, 1.0)],
        31 => vec![(0.5, 0.0), (1.0, 0.0), (1.0, 1.0), (0.5, 1.0)],
        32 => vec![(0.0, 0.0), (1.0, 0.0), (1.0, 0.5), (0.0, 0.5)],
        33 => vec![(0.0, 0.0), (0.5, 0.0), (0.5, 1.0), (0.0, 1.0)],
        2..=29 => {
            let grp = (code - 2) / 4;
            let (sx, sy) = [(1.0f32, -1.0f32), (-1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)][((code - 2) % 4) as usize];
            match grp {
                0 => {
                    let kx = if sx > 0.0 { 0.0 } else { 1.0 };
                    let ky = if sy < 0.0 { 1.0 } else { 0.0 };
                    vec![(kx, ky), (1.0 - kx, ky), (kx, 1.0 - ky)]
                }
                1 => {
                    // concave: open quarter disc centred on the normal side corner
                    let cx = if sx > 0.0 { 1.0 } else { 0.0 };
                    let cy = if sy < 0.0 { 0.0 } else { 1.0 };
                    let mut p = arc(cx, cy);
                    p.push((1.0 - cx, 1.0 - cy));
                    p
                }
                2 => {
                    // convex: solid quarter disc centred on the corner opposite the normal
                    let cx = if sx > 0.0 { 0.0 } else { 1.0 };
                    let cy = if sy < 0.0 { 1.0 } else { 0.0 };
                    let mut p = vec![(cx, cy)];
                    p.extend(arc(cx, cy));
                    p
                }
                3 | 4 => {
                    let big = grp == 4;
                    let slope = if sx * sy < 0.0 { 0.5 } else { -0.5 };
                    let solid_below = sy < 0.0;
                    let (lo, hi) = if big == solid_below { (0.0, 0.5) } else { (0.5, 1.0) };
                    let (yl, yr) = if slope > 0.0 { (lo, hi) } else { (hi, lo) };
                    if solid_below {
                        vec![(0.0, yl), (1.0, yr), (1.0, 1.0), (0.0, 1.0)]
                    } else {
                        vec![(0.0, 0.0), (1.0, 0.0), (1.0, yr), (0.0, yl)]
                    }
                }
                _ => {
                    let big = grp == 6;
                    let slope = if sx * sy < 0.0 { 0.5 } else { -0.5 };
                    let (lo, hi) = if (sx > 0.0) == big { (0.5, 1.0) } else { (0.0, 0.5) };
                    let (xt, xb) = if slope > 0.0 { (lo, hi) } else { (hi, lo) };
                    if sx > 0.0 {
                        vec![(0.0, 0.0), (xt, 0.0), (xb, 1.0), (0.0, 1.0)]
                    } else {
                        vec![(xt, 0.0), (1.0, 0.0), (1.0, 1.0), (xb, 1.0)]
                    }
                }
            }
        }
        _ => vec![],
    }
}

/// Quarter circle of radius 1 around corner (cx, cy), between its two neighbouring
/// corners, with exact endpoints.
fn arc(cx: f32, cy: f32) -> Vec<Pt> {
    let a = (1.0 - cx, cy);
    let b = (cx, 1.0 - cy);
    let a0 = (a.1 - cy).atan2(a.0 - cx);
    let a1 = (b.1 - cy).atan2(b.0 - cx);
    let d = (a1 - a0 + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    let mut p: Vec<Pt> = (0..=ARC_STEPS)
        .map(|i| {
            let t = a0 + d * i as f32 / ARC_STEPS as f32;
            (cx + t.cos(), cy + t.sin())
        })
        .collect();
    p[0] = a;
    p[ARC_STEPS] = b;
    p
}

fn on_side(p: Pt, q: Pt, side: usize) -> bool {
    let (a, b, v) = match side {
        LEFT => (p.0, q.0, 0.0),
        RIGHT => (p.0, q.0, 1.0),
        TOP => (p.1, q.1, 0.0),
        _ => (p.1, q.1, 1.0),
    };
    (a - v).abs() < EPS && (b - v).abs() < EPS
}

/// Which border side the edge p to q lies on, if any.
pub fn edge_side(p: Pt, q: Pt) -> Option<usize> {
    (0..4).find(|&s| on_side(p, q, s))
}

pub fn shapes() -> &'static [Shape] {
    static SHAPES: OnceLock<Vec<Shape>> = OnceLock::new();
    SHAPES.get_or_init(|| {
        (0..34u8)
            .map(|code| {
                let poly = build(code);
                let mut side_full = [false; 4];
                for i in 0..poly.len() {
                    let (p, q) = (poly[i], poly[(i + 1) % poly.len()]);
                    if let Some(s) = edge_side(p, q) {
                        let len = (p.0 - q.0).abs() + (p.1 - q.1).abs();
                        if len > 1.0 - EPS {
                            side_full[s] = true;
                        }
                    }
                }
                Shape { poly, side_full }
            })
            .collect()
    })
}

pub fn shape(code: u8) -> &'static Shape {
    &shapes()[(code as usize).min(33)]
}

pub fn inside(poly: &[Pt], x: f32, y: f32) -> bool {
    let mut c = false;
    for i in 0..poly.len() {
        let (p, q) = (poly[i], poly[(i + 1) % poly.len()]);
        if (p.1 > y) != (q.1 > y) && x < p.0 + (y - p.1) * (q.0 - p.0) / (q.1 - p.1) {
            c = !c;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(p: &[Pt]) -> f32 {
        (0..p.len()).map(|i| p[i].0 * p[(i + 1) % p.len()].1 - p[(i + 1) % p.len()].0 * p[i].1).sum::<f32>().abs() / 2.0
    }

    #[test]
    fn areas_make_sense() {
        assert_eq!(area(&shape(1).poly), 1.0);
        assert!((area(&shape(2).poly) - 0.5).abs() < 1e-5); // 45 degree
        assert!((area(&shape(30).poly) - 0.5).abs() < 1e-5); // half
        // 22 degree: small piece is 1/4, big piece 3/4
        assert!((area(&shape(14).poly) - 0.25).abs() < 1e-5);
        assert!((area(&shape(18).poly) - 0.75).abs() < 1e-5);
        // 67 degree: same, rotated
        assert!((area(&shape(22).poly) - 0.25).abs() < 1e-5);
        assert!((area(&shape(26).poly) - 0.75).abs() < 1e-5);
        // convex quarter disc is a bit under pi/4, concave is the complement
        let (cx, cv) = (area(&shape(10).poly), area(&shape(6).poly));
        assert!(cx > 0.74 && cx < std::f32::consts::FRAC_PI_4 + 1e-5, "convex {cx}");
        assert!((cx + cv - 1.0).abs() < 1e-4, "convex {cx} concave {cv}");
    }

    #[test]
    fn hill_crest_is_convex_up() {
        // nn (11) is the left half of a hill: solid below-right of its arc, so the
        // bottom-right corner is solid and the top-left corner is not.
        let s = shape(11);
        assert!(inside(&s.poly, 0.99, 0.99));
        assert!(!inside(&s.poly, 0.05, 0.05));
    }
}
