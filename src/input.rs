//! Keyboard state, in three tiers.
//!
//! 1. **Exact**: the terminal speaks the kitty keyboard protocol (or the platform
//!    reports key-up, as on Windows), so presses and releases are real.
//! 2. **Timing**: any other terminal only sends presses plus OS key repeat. A key
//!    counts as held while its events keep arriving; the repeat delay and interval
//!    are learned from the events themselves (the technique doom-cli uses). Before
//!    the first repeat a tap and a hold look identical, so that window is capped.
//! 3. **Latched**: direction stays on until the opposite key or stop. Selected with
//!    `ENTITY_INPUT=latch`.
//!
//! A gamepad (see `pad.rs`) is exact whatever the terminal, and is merged in on top.

use crate::ninja::Controls;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::time::{Duration, Instant};

/// Longest a key is assumed held before its first repeat arrives (tap versus hold is
/// unknowable until then). A slow OS repeat delay therefore shows as a short stall.
const PRE_REPEAT_CAP: Duration = Duration::from_millis(220);
const REPEAT_DELAY_DEFAULT: Duration = Duration::from_millis(500);
const REPEAT_INTERVAL_DEFAULT: Duration = Duration::from_millis(33);
const RELEASE_MIN: Duration = Duration::from_millis(60);
const RELEASE_MAX: Duration = Duration::from_millis(160);
/// Frame note while a gamepad is connected (render.rs shows pad hints with it).
pub const PAD_NOTE: &str = "pad";

/// Gamepad buttons that are commands rather than play.
#[cfg_attr(not(feature = "gamepad"), allow(dead_code))]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PadCmd {
    Restart,
    PrevLevel,
    NextLevel,
    /// Next level after finishing one, the episode again after running out of time.
    Start,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mode {
    Exact,
    Timing,
    Latched,
}

/// Learned OS key repeat timing, shared by all keys.
#[derive(Clone, Copy, Debug)]
struct Repeat {
    delay: Duration,
    interval: Duration,
}

/// One key as seen through presses only (timing mode).
#[derive(Clone, Copy, Default, Debug)]
struct Hold {
    since: Option<Instant>,
    last: Option<Instant>,
    events: u32,
}

impl Hold {
    /// Feed one press/repeat event; returns true when it should count as a fresh press
    /// (a jump edge, a turn). The first repeat of a held key arrives one repeat delay
    /// after the press, so it is indistinguishable from a deliberate second tap: it
    /// extends the hold AND counts as fresh. Only a tight repeat stream does not.
    fn event(&mut self, now: Instant, rep: &mut Repeat) -> bool {
        if let Some(last) = self.last {
            let gap = now.duration_since(last);
            let tight = gap <= rep.interval * 3;
            let continues = if self.events == 1 {
                // first repeat arrives about one repeat delay after the press
                gap >= rep.delay.mul_f32(0.5) && gap <= rep.delay.mul_f32(1.6)
            } else {
                gap <= RELEASE_MAX.max(rep.interval * 4)
            };
            if continues {
                self.events += 1;
                let blend = |old: Duration, new: Duration| (old.mul_f32(0.5) + new.mul_f32(0.5)).max(Duration::from_millis(10));
                if self.events == 2 {
                    rep.delay = blend(rep.delay, gap.clamp(Duration::from_millis(100), Duration::from_millis(1000)));
                } else if gap < Duration::from_millis(200) {
                    rep.interval = blend(rep.interval, gap);
                }
                self.last = Some(now);
                return !tight;
            }
        }
        self.since = Some(now);
        self.last = Some(now);
        self.events = 1;
        true
    }

    fn held(&self, now: Instant, rep: &Repeat, pre_cap: Duration) -> bool {
        let Some(last) = self.last else { return false };
        let timeout = if self.events >= 2 {
            (rep.interval * 5 / 2).clamp(RELEASE_MIN, RELEASE_MAX)
        } else {
            pre_cap.min(rep.delay + rep.interval * 2)
        };
        now.duration_since(last) <= timeout
    }

    fn clear(&mut self) {
        *self = Hold::default();
    }
}

#[derive(Clone)]
pub struct Input {
    mode: Mode,
    // exact mode
    left: bool,
    right: bool,
    jump_down: bool,
    // timing mode
    rep: Repeat,
    hold_left: Hold,
    hold_right: Hold,
    hold_jump: Hold,
    // latched mode
    latch: i8,
    jump_last: Instant,
    jump_edge: bool,
    // gamepad, held state of all pads together
    pad_connected: bool,
    pad_dir: i8,
    pad_jump: bool,
    /// Timing mode tuning (see the playtest in `ninja.rs`).
    pub(crate) pre_cap: Duration,
    pub(crate) air_pre_cap: Duration,
    /// When set, a jump press counts as held this long whatever the repeats say.
    pub(crate) jump_window: Option<Duration>,
    pub quit: bool,
    /// Esc was pressed: show the quit question and wait for y.
    pub confirm_quit: bool,
    pub restart: bool,
    pub fit_toggle: bool,
    pub ninja_steps: i32,
    pub level_step: i32,
    /// The pad's Start button, which main resolves against the game's state.
    pub pad_start: bool,
}

impl Input {
    pub fn new(kitty: bool) -> Input {
        let mode = if kitty {
            Mode::Exact
        } else if std::env::var("ENTITY_INPUT").is_ok_and(|v| v == "latch") {
            Mode::Latched
        } else {
            Mode::Timing
        };
        Input::with_mode(mode)
    }

    pub fn with_mode(mode: Mode) -> Input {
        Input {
            mode,
            left: false,
            right: false,
            jump_down: false,
            rep: Repeat { delay: REPEAT_DELAY_DEFAULT, interval: REPEAT_INTERVAL_DEFAULT },
            hold_left: Hold::default(),
            hold_right: Hold::default(),
            hold_jump: Hold::default(),
            latch: 0,
            pre_cap: PRE_REPEAT_CAP,
            air_pre_cap: PRE_REPEAT_CAP,
            // without release events a tap cannot be cut short, so a jump is full length
            jump_window: if mode == Mode::Timing { Some(Duration::from_millis(750)) } else { None },
            jump_last: Instant::now() - Duration::from_secs(10),
            jump_edge: false,
            pad_connected: false,
            pad_dir: 0,
            pad_jump: false,
            quit: false,
            confirm_quit: false,
            restart: false,
            fit_toggle: false,
            ninja_steps: 0,
            level_step: 0,
            pad_start: false,
        }
    }

    /// Short note for the frame when input is not exact.
    pub fn mode_note(&self) -> Option<&'static str> {
        if self.pad_connected {
            return Some(PAD_NOTE);
        }
        match self.mode {
            Mode::Exact => None,
            Mode::Timing => Some("keys: timing"),
            Mode::Latched => Some("keys: latched"),
        }
    }

    pub fn on_key(&mut self, k: KeyEvent, now: Instant) {
        let down = k.kind != KeyEventKind::Release;
        let press = k.kind == KeyEventKind::Press;
        if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c') {
            self.quit = true;
            return;
        }
        if self.confirm_quit {
            // modal: y quits, any other key press goes back to the game
            if press {
                if k.code == KeyCode::Char('y') {
                    self.quit = true;
                }
                self.confirm_quit = false;
            }
            return;
        }
        match k.code {
            KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('h') => self.direction(true, down, press, now),
            KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('l') => self.direction(false, down, press, now),
            KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('k') | KeyCode::Char(' ') | KeyCode::Char('z') => {
                self.jump(down, press, now)
            }
            KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('j') if press => {
                self.latch = 0;
                self.hold_left.clear();
                self.hold_right.clear();
            }
            KeyCode::Char('r') if press => self.restart = true,
            KeyCode::Char('f') if press => self.fit_toggle = true,
            KeyCode::Char('[') if press => self.ninja_steps -= 1,
            KeyCode::Char(']') if press => self.ninja_steps += 1,
            KeyCode::Char('n') if press => self.level_step += 1,
            KeyCode::Char('p') if press => self.level_step -= 1,
            KeyCode::Esc if press => self.confirm_quit = true,
            _ => {}
        }
    }

    /// Held state of the pads, after every batch of pad events. `connected`: a usable pad
    /// is plugged in.
    #[cfg_attr(not(feature = "gamepad"), allow(dead_code))]
    pub fn on_pad(&mut self, connected: bool, dir: i8, jump: bool) {
        if dir != self.pad_dir || jump != self.pad_jump {
            log::debug!("pad state dir {dir} jump {jump}");
        }
        // a press that answers the quit question is not also a jump
        if jump && !self.pad_jump && !self.confirm_quit {
            self.jump_edge = true;
        }
        if dir != 0 && dir != self.pad_dir {
            // the pad takes over from a key still counted as held
            self.left = false;
            self.right = false;
            self.hold_left.clear();
            self.hold_right.clear();
            self.latch = 0;
        }
        self.pad_connected = connected;
        self.pad_dir = dir;
        self.pad_jump = jump;
    }

    /// A pad button went down. While the quit question shows, any button keeps playing.
    #[cfg_attr(not(feature = "gamepad"), allow(dead_code))]
    pub fn on_pad_press(&mut self, cmd: Option<PadCmd>) {
        if self.confirm_quit {
            self.confirm_quit = false;
            return;
        }
        match cmd {
            Some(PadCmd::Restart) => self.restart = true,
            Some(PadCmd::PrevLevel) => self.level_step -= 1,
            Some(PadCmd::NextLevel) => self.level_step += 1,
            Some(PadCmd::Start) => self.pad_start = true,
            None => {}
        }
    }

    fn direction(&mut self, left: bool, down: bool, press: bool, now: Instant) {
        match self.mode {
            Mode::Exact => {
                *(if left { &mut self.left } else { &mut self.right }) = down;
            }
            Mode::Timing => {
                if down {
                    let rep = &mut self.rep;
                    let (mine, other) = if left {
                        (&mut self.hold_left, &mut self.hold_right)
                    } else {
                        (&mut self.hold_right, &mut self.hold_left)
                    };
                    // a fresh press of one direction ends the other (turning around)
                    if mine.event(now, rep) {
                        other.clear();
                    }
                }
            }
            Mode::Latched => {
                if press {
                    self.latch = if left { -1 } else { 1 };
                }
            }
        }
    }

    fn jump(&mut self, down: bool, press: bool, now: Instant) {
        match self.mode {
            Mode::Exact => {
                if press {
                    self.jump_edge = true;
                }
                self.jump_down = down;
            }
            Mode::Timing => {
                if down && self.hold_jump.event(now, &mut self.rep) {
                    self.jump_edge = true;
                }
            }
            Mode::Latched => {
                if down {
                    let was_held = now.duration_since(self.jump_last) < Duration::from_millis(150);
                    if press && !was_held {
                        self.jump_edge = true;
                    }
                    self.jump_last = now;
                }
            }
        }
    }

    /// `airborne`: the ninja is off the ground, where steering matters less than keeping
    /// the direction through a stall, so a longer pre-repeat window can apply.
    pub fn controls(&mut self, now: Instant, airborne: bool) -> Controls {
        let cap = if airborne { self.air_pre_cap } else { self.pre_cap };
        let (dir, jump_held) = match self.mode {
            Mode::Exact => (self.right as i8 - self.left as i8, self.jump_down),
            Mode::Timing => {
                let l = self.hold_left.held(now, &self.rep, cap);
                let r = self.hold_right.held(now, &self.rep, cap);
                let dir = match (l, r) {
                    (true, false) => -1,
                    (false, true) => 1,
                    // both still counted: the most recent event is the one being held
                    (true, true) => {
                        if self.hold_left.last >= self.hold_right.last {
                            -1
                        } else {
                            1
                        }
                    }
                    _ => 0,
                };
                (dir, self.jump_window.map_or_else(|| self.hold_jump.held(now, &self.rep, cap), |w| self.hold_jump.last.is_some_and(|t| now.duration_since(t) <= w) && self.hold_jump.events == 1 || self.hold_jump.held(now, &self.rep, cap)))
            }
            Mode::Latched => (self.latch, now.duration_since(self.jump_last) < Duration::from_millis(150)),
        };
        let dir = if self.pad_dir != 0 { self.pad_dir } else { dir };
        Controls { dir, jump_held: jump_held || self.pad_jump, jump_pressed: std::mem::take(&mut self.jump_edge) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(base: Instant, t: u64) -> Instant {
        base + Duration::from_millis(t)
    }

    fn press(i: &mut Input, code: KeyCode, at: Instant) {
        i.on_key(KeyEvent::new(code, KeyModifiers::NONE), at);
    }

    fn dir(i: &mut Input, at: Instant) -> i8 {
        i.controls(at, false).dir
    }

    #[test]
    fn a_tap_is_a_short_nudge() {
        let t0 = Instant::now();
        let mut i = Input::with_mode(Mode::Timing);
        press(&mut i, KeyCode::Right, t0);
        assert_eq!(dir(&mut i, ms(t0, 100)), 1);
        assert_eq!(dir(&mut i, ms(t0, 400)), 0, "no repeat arrived: it was a tap");
    }

    #[test]
    fn a_real_hold_with_the_default_slow_repeat() {
        // 500 ms delay then 30 ms interval, like GNOME's default
        let t0 = Instant::now();
        let mut i = Input::with_mode(Mode::Timing);
        press(&mut i, KeyCode::Right, t0);
        assert_eq!(dir(&mut i, ms(t0, 100)), 1);
        assert_eq!(dir(&mut i, ms(t0, 400)), 0, "the unavoidable stall while tap and hold look alike");
        for k in 0..20 {
            press(&mut i, KeyCode::Right, ms(t0, 500 + 30 * k));
        }
        assert_eq!(dir(&mut i, ms(t0, 505)), 1, "repeats started, held again");
        assert_eq!(dir(&mut i, ms(t0, 1000)), 1);
        // last repeat at 500 + 30 * 19 = 1070; released shortly after
        assert_eq!(dir(&mut i, ms(t0, 1100)), 1);
        assert_eq!(dir(&mut i, ms(t0, 1300)), 0, "release noticed within about 100 ms of the real one");
    }

    #[test]
    fn a_short_repeat_delay_has_no_stall() {
        let t0 = Instant::now();
        let mut i = Input::with_mode(Mode::Timing);
        i.rep.delay = Duration::from_millis(150);
        press(&mut i, KeyCode::Right, t0);
        for step in [0u64, 60, 120, 180, 240, 300, 345] {
            if step > 0 {
                let at = if step == 60 { 150 } else { 150 + 30 * (step / 60) };
                press(&mut i, KeyCode::Right, ms(t0, at));
            }
        }
        for t in [20, 100, 149, 160, 200] {
            assert_eq!(dir(&mut i, ms(t0, t)), 1, "continuous at {t} ms");
        }
    }

    #[test]
    fn the_repeat_timing_is_learned() {
        let t0 = Instant::now();
        let mut i = Input::with_mode(Mode::Timing);
        for round in 0..3u64 {
            let base = 2000 * round;
            press(&mut i, KeyCode::Right, ms(t0, base));
            for k in 0..12 {
                press(&mut i, KeyCode::Right, ms(t0, base + 250 + 40 * k));
            }
        }
        let d = i.rep.delay.as_millis() as i64;
        let iv = i.rep.interval.as_millis() as i64;
        assert!((d - 250).abs() < 60, "delay {d}");
        assert!((iv - 40).abs() < 10, "interval {iv}");
    }

    #[test]
    fn turning_around_switches_direction_at_once() {
        let t0 = Instant::now();
        let mut i = Input::with_mode(Mode::Timing);
        press(&mut i, KeyCode::Right, t0);
        for k in 0..10 {
            press(&mut i, KeyCode::Right, ms(t0, 500 + 30 * k));
        }
        assert_eq!(dir(&mut i, ms(t0, 790)), 1);
        press(&mut i, KeyCode::Left, ms(t0, 800));
        assert_eq!(dir(&mut i, ms(t0, 810)), -1);
    }

    #[test]
    fn stop_key_stops_now() {
        let t0 = Instant::now();
        let mut i = Input::with_mode(Mode::Timing);
        press(&mut i, KeyCode::Right, t0);
        press(&mut i, KeyCode::Char('s'), ms(t0, 50));
        assert_eq!(dir(&mut i, ms(t0, 60)), 0);
    }

    #[test]
    fn jump_edges_and_hold() {
        let t0 = Instant::now();
        let mut i = Input::with_mode(Mode::Timing);
        press(&mut i, KeyCode::Char(' '), t0);
        let c = i.controls(ms(t0, 10), false);
        assert!(c.jump_pressed && c.jump_held);
        assert!(!i.controls(ms(t0, 20), false).jump_pressed, "edge fires once");
        // a quick second tap is a new jump
        press(&mut i, KeyCode::Char(' '), ms(t0, 120));
        assert!(i.controls(ms(t0, 125), false).jump_pressed);
    }

    #[test]
    fn re_tapping_at_about_the_repeat_delay_still_jumps() {
        // the regression the playtest found: taps 250 to 800 ms apart were read as repeats
        for gap in [250u64, 400, 500, 650, 800] {
            let t0 = Instant::now();
            let mut i = Input::with_mode(Mode::Timing);
            press(&mut i, KeyCode::Char(' '), t0);
            assert!(i.controls(ms(t0, 10), false).jump_pressed);
            press(&mut i, KeyCode::Char(' '), ms(t0, gap));
            assert!(i.controls(ms(t0, gap + 5), false).jump_pressed, "second tap {gap} ms later was swallowed");
        }
    }

    #[test]
    fn a_tight_repeat_stream_is_not_a_new_jump_each_time() {
        let t0 = Instant::now();
        let mut i = Input::with_mode(Mode::Timing);
        press(&mut i, KeyCode::Char(' '), t0);
        let mut fresh = 0;
        for k in 0..30 {
            press(&mut i, KeyCode::Char(' '), ms(t0, 500 + 33 * k));
            if i.controls(ms(t0, 502 + 33 * k), false).jump_pressed {
                fresh += 1;
            }
        }
        assert!(fresh <= 1, "{fresh} jumps from one held key (only the first repeat may count)");
    }

    #[test]
    fn exact_mode_follows_real_press_and_release() {
        let t0 = Instant::now();
        let mut i = Input::with_mode(Mode::Exact);
        i.on_key(KeyEvent::new_with_kind(KeyCode::Right, KeyModifiers::NONE, KeyEventKind::Press), t0);
        assert_eq!(dir(&mut i, ms(t0, 900)), 1, "held for as long as it is down");
        i.on_key(KeyEvent::new_with_kind(KeyCode::Right, KeyModifiers::NONE, KeyEventKind::Release), ms(t0, 950));
        assert_eq!(dir(&mut i, ms(t0, 951)), 0);
    }

    #[test]
    fn q_does_not_quit_and_esc_asks_first() {
        let t0 = Instant::now();
        let mut i = Input::with_mode(Mode::Timing);
        press(&mut i, KeyCode::Char('q'), t0);
        assert!(!i.quit && !i.confirm_quit);
        press(&mut i, KeyCode::Esc, t0);
        assert!(i.confirm_quit && !i.quit);
        press(&mut i, KeyCode::Char('n'), t0);
        assert!(!i.confirm_quit && !i.quit, "any other key cancels");
        press(&mut i, KeyCode::Esc, t0);
        press(&mut i, KeyCode::Char('y'), t0);
        assert!(i.quit);
    }

    #[test]
    fn a_pad_holds_and_releases_exactly_in_the_timing_tier() {
        let t0 = Instant::now();
        let mut i = Input::with_mode(Mode::Timing);
        i.on_pad(true, 1, true);
        let c = i.controls(ms(t0, 900), false);
        assert_eq!(c.dir, 1, "held with no repeats needed");
        assert!(c.jump_pressed && c.jump_held);
        let c = i.controls(ms(t0, 950), false);
        assert!(!c.jump_pressed && c.jump_held, "one edge, then held");
        i.on_pad(true, 0, false);
        let c = i.controls(ms(t0, 951), false);
        assert_eq!(c.dir, 0);
        assert!(!c.jump_held, "a release cuts the jump short, unlike a key tap");
        assert_eq!(i.mode_note(), Some(PAD_NOTE));
    }

    #[test]
    fn the_pad_overrides_a_key_still_counted_as_held() {
        let t0 = Instant::now();
        let mut i = Input::with_mode(Mode::Timing);
        press(&mut i, KeyCode::Right, t0);
        i.on_pad(true, -1, false);
        assert_eq!(dir(&mut i, ms(t0, 10)), -1);
        i.on_pad(true, 0, false);
        assert_eq!(dir(&mut i, ms(t0, 20)), 0, "the stale key does not come back");
    }

    #[test]
    fn a_pad_button_answers_the_quit_question_without_jumping() {
        let t0 = Instant::now();
        let mut i = Input::with_mode(Mode::Exact);
        press(&mut i, KeyCode::Esc, t0);
        i.on_pad(true, 0, true);
        i.on_pad_press(None);
        assert!(!i.confirm_quit && !i.quit, "keeps playing");
        assert!(!i.controls(t0, false).jump_pressed);
        i.on_pad_press(Some(PadCmd::Restart));
        assert!(i.restart);
    }
}
