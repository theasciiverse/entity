//! Gamepads through gilrs (feature `gamepad`, on by default).
//!
//! A pad reports real presses and releases, so it plays like the exact keyboard tier
//! in any terminal: held jump for height, a tap for a hop. Every connected pad drives
//! the one ninja.
//!
//! - Run: d-pad or left stick.
//! - Jump: A or B, or d-pad up.
//! - Restart the attempt: Y or Back.
//! - Previous / next level: LB / RB.
//! - Start: next level after finishing one, the episode again after running out of time.
//!
//! Only SDL-mapped devices count as pads (gilrs also lists drawing-tablet key pads and
//! 3D mice); `ENTITY_ANY_PAD=1` accepts everything.

use crate::input::Input;
#[cfg(feature = "gamepad")]
use crate::input::PadCmd;

/// Stick deflection that counts as running.
#[cfg(feature = "gamepad")]
const STICK: f32 = 0.4;

pub struct Pad {
    #[cfg(feature = "gamepad")]
    gilrs: Option<gilrs::Gilrs>,
    /// No poll yet: report the pads plugged in at start even before they send anything.
    #[cfg(feature = "gamepad")]
    first: bool,
}

#[cfg(feature = "gamepad")]
fn usable(p: &gilrs::Gamepad) -> bool {
    p.is_connected()
        && (p.mapping_source() == gilrs::MappingSource::SdlMappings
            || std::env::var("ENTITY_ANY_PAD").is_ok_and(|v| v == "1"))
}

impl Pad {
    pub fn open() -> Pad {
        #[cfg(feature = "gamepad")]
        {
            let gilrs = match gilrs::Gilrs::new() {
                Ok(g) => {
                    let all: Vec<_> = g.gamepads().map(|(_, p)| (p.name().to_string(), usable(&p))).collect();
                    log::info!("gamepads at start (name, used): {all:?}");
                    Some(g)
                }
                Err(e) => {
                    log::warn!("gamepad support unavailable: {e}");
                    None
                }
            };
            Pad { gilrs, first: true }
        }
        #[cfg(not(feature = "gamepad"))]
        Pad {}
    }

    /// Drain pad events into `input`: the held state of all pads, then buttons as commands.
    pub fn poll(&mut self, input: &mut Input) {
        #[cfg(feature = "gamepad")]
        {
            use gilrs::{Axis, Button, EventType};
            let Some(g) = &mut self.gilrs else { return };
            let mut changed = std::mem::take(&mut self.first);
            let mut presses = Vec::new();
            while let Some(ev) = g.next_event() {
                let pad = g.gamepad(ev.id);
                if !usable(&pad) && !matches!(ev.event, EventType::Disconnected) {
                    continue;
                }
                match ev.event {
                    EventType::Connected => log::info!("gamepad connected: {}", pad.name()),
                    EventType::Disconnected => log::info!("gamepad disconnected: {}", pad.name()),
                    EventType::ButtonPressed(b, _) => {
                        log::debug!("pad press {b:?}");
                        let cmd = match b {
                            Button::North | Button::Select => Some(PadCmd::Restart),
                            Button::LeftTrigger => Some(PadCmd::PrevLevel),
                            Button::RightTrigger => Some(PadCmd::NextLevel),
                            Button::Start => Some(PadCmd::Start),
                            _ => None,
                        };
                        presses.push(cmd);
                    }
                    EventType::ButtonReleased(b, _) => log::debug!("pad release {b:?}"),
                    _ => {}
                }
                changed = true;
            }
            if !changed {
                return;
            }
            let (mut dir, mut jump, mut any) = (0i8, false, false);
            for (_, p) in g.gamepads().filter(|(_, p)| usable(p)) {
                any = true;
                let x = p.axis_data(Axis::LeftStickX).map_or(0.0, |d| d.value());
                if p.is_pressed(Button::DPadLeft) || x < -STICK {
                    dir -= 1;
                }
                if p.is_pressed(Button::DPadRight) || x > STICK {
                    dir += 1;
                }
                jump |= p.is_pressed(Button::South) || p.is_pressed(Button::East) || p.is_pressed(Button::DPadUp);
            }
            // held state first: a press that answers the quit question is then not a jump
            input.on_pad(any, dir.clamp(-1, 1), jump);
            for cmd in presses {
                input.on_pad_press(cmd);
            }
        }
        #[cfg(not(feature = "gamepad"))]
        let _ = input;
    }
}
