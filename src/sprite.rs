//! The ninja as 8x8 pixel art, drawn right-facing ('X' is filled) and mirrored for
//! left. The art is sampled onto whatever dot grid the board has, so it stays
//! readable from about 5 dots across and up.

use crate::ninja::Pose;

pub type Frame = [&'static str; 8];

const STAND: Frame = ["..XXX...", "X.XXX...", "..XXX...", "..XX....", ".XXXX...", "..XX....", "..XX....", ".XX.XX.."];

const RUN: [Frame; 4] = [
    ["...XX...", "XX.XXX..", "...XX...", "..XXXX..", ".XXXX.X.", "..XX.X..", ".XX..XX.", "XX....X."],
    ["...XX...", "XX.XXX..", "...XX...", "..XXX...", "..XXXX..", "..XX.X..", "..XX.X..", "..X..XX."],
    ["...XX...", "XX.XXX..", "...XX...", "..XXXX..", ".X.XXXX.", "..XX.X..", ".XX..XX.", ".X....X."],
    ["...XX...", "XX.XXX..", "...XX...", "..XXX...", "..XXXX..", "..XX.X..", "..XX.X..", "..X..XX."],
];

const JUMP: Frame = ["..XXX.X.", ".XXXXXX.", "..XXX.X.", "...XX...", "..XXXX..", "..XXXX..", "..X..X..", ".XX..XX."];

const FALL: Frame = ["X.XXX...", "..XXX...", ".XXXXX..", "X.XX..X.", "..XXXX..", "..XXXX..", "..X..X..", ".X....X."];

const SLIDE: Frame = ["..XXX...", "X.XXXX.X", "..XXX..X", "..XX..X.", "..XXXXX.", "..XX....", "..XX....", ".XXXX..."];

pub fn frame(pose: Pose) -> &'static Frame {
    match pose {
        Pose::Stand => &STAND,
        Pose::Run(i) => &RUN[i % 4],
        Pose::Jump => &JUMP,
        Pose::Fall => &FALL,
        Pose::Slide => &SLIDE,
    }
}

pub const ALL: [(&str, Pose); 8] = [
    ("stand", Pose::Stand),
    ("run 0", Pose::Run(0)),
    ("run 1", Pose::Run(1)),
    ("run 2", Pose::Run(2)),
    ("run 3", Pose::Run(3)),
    ("jump", Pose::Jump),
    ("fall", Pose::Fall),
    ("slide", Pose::Slide),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_frame_is_8_by_8_and_not_empty() {
        for (name, pose) in ALL {
            let f = frame(pose);
            assert_eq!(f.len(), 8, "{name}");
            for row in f.iter() {
                assert_eq!(row.len(), 8, "{name}: {row}");
                assert!(row.bytes().all(|b| b == b'X' || b == b'.'), "{name}: {row}");
            }
            assert!(f.iter().map(|r| r.bytes().filter(|&b| b == b'X').count()).sum::<usize>() >= 14, "{name}");
        }
    }
}
