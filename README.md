# Entity

**ONE NINJA. ONE HUNDRED ROOMS. ONE SECOND IS A LONG TIME.**

Run, jump and wall-jump through rooms full of mines, drones, lasers,
turrets, thwomps and homing rockets. Touch the switch, the door opens,
reach it. Gold buys you time; every five levels share one clock. Death is
instant, and so is the next try.

Entity is a precision platformer for the terminal after N (Metanet
Software, 2004): the same momentum, wall jumps and objects, with a
campaign of 100 levels of its own, twenty episodes of five. The whole
room is always on one screen, drawn in block glyphs, with the ninja in
Braille dots.

## Play

With [av](https://asciiverse.sh), the Asciiverse client:

```
av install entity
av play entity
```

To build it yourself: `cargo run --release`.

## Controls

- **Arrows**, **WASD** or **HJKL**: run
- **Up**, **W**, **K**, **Space** or **Z**: jump; hold for height
- **Down**, **S** or **J**: stop
- **R**: try again (after running out of time: the episode)
- **N** / **P**: next / previous level
- **F**: fill the terminal or keep square tiles
- **[** / **]**: draw the ninja smaller or bigger
- **Esc**: quit (it asks first); **Ctrl-C** quits at once
- **Gamepad**: d-pad or left stick to run; A, B or d-pad up to jump; Y or
  Back to try again; LB / RB for the previous / next level; A, B or Start
  for the next level after finishing one

Best in a terminal that reports key releases (kitty, foot, Ghostty,
Alacritty and others), or with a gamepad. Elsewhere Entity learns your
keyboard's repeat rate and counts a key as held while it repeats.
`ENTITY_INPUT=latch` keeps a direction on until the opposite key or stop.

## Starting somewhere

`entity 4:0` starts at episode 4, level 0; `entity "last door"` finds a
level by name. `entity --demo 3:4` watches the computer play a level.

## Making levels

- `entity --edit levels/mine.txt`: the level editor (p to test play)
- `entity --prove levels/mine.txt`: the route bot finds a way through and
  stores it in the file
- `entity --render levels/mine.txt`: the board as plain text

## Files

The game saves nothing; the editor writes only the level file it is given.
`RUST_LOG=debug entity 2>entity.log` writes a session log.

## Requirements

A UTF-8 terminal with truecolour and a font with Braille (U+2800 to
U+28FF), 64x25 or bigger. On Linux, gamepads need libudev1.

## Building

```bash
cargo build --release
```

## Licence

MIT; see [LICENSE](LICENSE).
