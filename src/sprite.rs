//! The ninja as N's own stick figure: the frames of the original's ninja clip, extracted
//! by `tools/extract_ninja.py` into `data/n-ninja.json`. Each frame is ten limb lines
//! (torso, thighs, shins, upper arms, forearms) and a small triangle for the head, in
//! world units around the hitbox centre (radius 10, feet at about y = +10), facing right
//! with y down. The renderer draws the lines as Braille dots, mirrored for left and
//! tilted with the floor.

use crate::ninja::Pose;
use serde::Deserialize;
use std::sync::OnceLock;

/// A point relative to the hitbox centre, in world units.
pub type P = (f32, f32);

#[derive(Deserialize)]
pub struct Figure {
    lines: Vec<[f32; 4]>,
    pub head: [[f32; 2]; 3],
}

impl Figure {
    pub fn segments(&self) -> impl Iterator<Item = (P, P)> + '_ {
        self.lines.iter().map(|l| ((l[0], l[1]), (l[2], l[3])))
    }

    pub fn head_centre(&self) -> P {
        let h = &self.head;
        ((h[0][0] + h[1][0] + h[2][0]) / 3.0, (h[0][1] + h[1][1] + h[2][1]) / 3.0)
    }
}

#[derive(Deserialize)]
struct Clip {
    frames: Vec<Figure>,
}

fn clip() -> &'static Clip {
    static C: OnceLock<Clip> = OnceLock::new();
    C.get_or_init(|| serde_json::from_str(include_str!("../data/n-ninja.json")).expect("data/n-ninja.json"))
}

/// The clip frame for a pose (STAND 0..10, SKID 11, RUN 12..83, JUMP 84..102, WALLSLIDE 103).
pub fn frame_index(pose: Pose) -> usize {
    match pose {
        Pose::Stand(i) => i.min(10),
        Pose::Skid => 11,
        Pose::Run(i) => 12 + i % 72,
        Pose::Air(i) => 84 + i.min(18),
        Pose::Slide => 103,
    }
}

pub fn figure(pose: Pose) -> &'static Figure {
    &clip().frames[frame_index(pose)]
}

pub const ALL: [(&str, Pose); 10] = [
    ("stand", Pose::Stand(10)),
    ("skid", Pose::Skid),
    ("run 0", Pose::Run(0)),
    ("run 18", Pose::Run(18)),
    ("run 36", Pose::Run(36)),
    ("run 54", Pose::Run(54)),
    ("rise", Pose::Air(0)),
    ("top", Pose::Air(9)),
    ("fall", Pose::Air(18)),
    ("slide", Pose::Slide),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_clip_has_every_frame_and_the_figures_stand_in_the_hitbox() {
        assert_eq!(clip().frames.len(), 104);
        for (i, f) in clip().frames.iter().enumerate() {
            assert_eq!(f.lines.len(), 10, "frame {i}");
            for (a, b) in f.segments() {
                for p in [a, b] {
                    assert!(p.0.abs() <= 11.0 && p.1.abs() <= 11.0, "frame {i}: {p:?}");
                }
            }
        }
        // the rest pose stands on the floor line with its head at the top
        let rest = figure(Pose::Stand(10));
        let feet = rest.segments().map(|(a, b)| a.1.max(b.1)).fold(f32::MIN, f32::max);
        assert!((feet - 10.0).abs() < 0.5, "feet at {feet}");
        assert!(rest.head_centre().1 < -8.0);
    }
}
