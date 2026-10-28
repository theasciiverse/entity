# Entity

Asciiverse platformer. Rust + Ratatui + Drawille. N++-inspired.

Repo: private, github.com/theasciiverse/entity (branch `master`; commits as David, matching the sibling
Asciiverse repos). Licence: MIT (2026-10-28).

**THE NINJA ANIMATION STAYS AS IS (David, 2026-10-28, final).** Entity's Braille ninja is Entity's own
drawing and ships with the game. It is not an open question: never propose replacing, redrawing or
removing it, and never list it as a licensing concern.

Metanet's N levels were removed from the game on 2026-10-28 (David): the level list is our campaign
plus the practice course.

## Premise

A port/homage to N++ — minimalist precision platforming, momentum-based movement, instant death, ninja-on-a-grid aesthetic. Drawille rendering for the sub-cell resolution needed to make tight collision and movement feel crisp rather than mushy.

## Levels: 100 of our own

Entity gets 100 levels of its own design (20 episodes of 5); N's levels were removed on 2026-10-28. Every level
element stays: tiles of all shapes, mines, drones, turrets, doors, gold, switch and exit, and all of the
physics; the ninja and its animation stay exactly as they are (see the rule at the top). The tools are the level editor (`entity --edit`) and the route bot (`entity --prove`), which
proves each level finishable and stores the route in the level file.

## Why this matters technically

N++ lives or dies entirely on movement feel: precise momentum, exact collision edges, satisfying death particle bursts. This is a much harder rendering/feel problem than Bob-sleigh's more forgiving arcade kart physics — it's the project that really tests how far Ratatui (cell grid) + Drawille (Braille sub-pixel, 2x4 per cell) can be pushed for genuinely tight controls, not just nice-looking effects.

## Rendering approach (carried over from Bob-sleigh's split-layer idea)

- **Ratatui (cell grid):** HUD, level select, death counter, menus
- **Drawille (Braille sub-pixel):** the actual level geometry, the ninja's exact position/collision, death particle bursts, momentum trails

Open question: does the ninja itself render at Braille resolution (for precision) or cell-grid (for readability/iconic silhouette)? Likely needs prototyping to feel out — N++'s appeal is partly the clean silhouette, but sub-cell precision matters more for collision-feel than visual clarity here, unlike Bob-sleigh where sprite readability mattered more.

## Open questions / next steps

- [ ] Prototype core movement (run/jump/wall-jump/momentum) against a single static level slab before anything else — feel comes first, content second
- [ ] Decide ninja rendering resolution (cell vs Braille) — see note above
- [ ] Level format: hand-authored ASCII/text levels? Or a simple tile format with a level editor later?
- [ ] Death/respawn loop — instant respawn is core to N++ feel, needs to be near-zero latency
- [ ] Hazard types: mines, drones, lasers, gauss turrets — which subset for v1?
- [ ] Gold/collectible and exit-switch mechanic (core N++ loop: collect gold, hit switch, reach exit)
- [ ] Input handling latency in terminal — crossterm polling rate, is it good enough for precision platforming?
- [ ] Confirm crate choices: ratatui, drawille crate (or hand-rolled Braille buffer, possibly shared code with Bob-sleigh's renderer)

## Status

Playable prototype on all 150 N levels (2026-10-02). Confirmed as next-up after Bob-sleigh, 2026-06-30.
Since 2026-10-27 a new game starts with the campaign of our own levels (30 of the 100 so far, see
"The campaign" below); the practice course follows it in the level list.

**Display design (decided with David):** like the original, the whole level is always on ONE
screen, no scrolling. The playfield (31x23 tiles) is divided into an n x m grid of terminal cells. By
default (`FitMode::Fill`) the board fills the terminal with a one cell margin all round, stretching
tiles up to 30% wider or taller than square when the terminal's shape differs from the level's (about
20% wider in a 156x47 window); the `f` key or `ENTITY_FIT=square` switches to square tiles, where the
board is the largest that fits and is centred. The terminal's real cell shape is read from its pixel
size (`window_size`), falling back to 1:2. About 134x47 in a 156x47
window, 84x30 in 100x30. Cells need not line up with tile edges: each cell samples the level
directly. A box-drawing line frames the board (`┌──[Entity Level 3:4 the last door]──┐`); the bottom
edge carries score, gold and deaths. Minimum 64 x 25 cells; below that a "too small" message shows,
and a bigger terminal gets the board centred. A default 80x24 terminal is too small (one row short).
Objects stay ONE glyph each at every size (the field grows, targets do not). Objects centred on the
playfield edge are clamped onto the nearest cell.

Glyph policy: most things are ONE glyph per cell. Terrain is matched per cell against block
elements (lower and left eighths, quadrants, and their colour-inverted forms) by comparing an 8x8
sample of the cell with each glyph. Glyph error is checked across board sizes (see the art tests). Objects are single glyphs. Only the
ninja and death particles are Braille (6 world units per dot, ninja about 3 dots across), and the
terrain under the ninja is merged in as dots so slopes stay intact.

Fonts: GNOME Terminal (VTE 0.84, source read at that tag) draws block elements, box drawing, the
triangles, sextants, wedges and octants ITSELF, ignoring the font; it does not draw Braille or the
object shapes, which come from fontconfig fallback (DejaVu Sans for David's Ubuntu Sans Mono; the
Braille ninja was seen rendering cleanly in his terminal). So font choice there is about looks only.
Other terminals may differ. Earlier measurements, for when a font does matter: Checked on this machine: the system font (Ubuntu Sans Mono) lacks eighths, quadrants,
shapes and Braille; only FreeMono covers the whole palette, and nothing installed has sextants,
legacy wedges or octants. Downloaded and measured (fc-scan): **Cascadia Mono 2407.24** and
**JuliaMono** each cover 100% of eighths, quadrants, shades, box, shapes, Braille, sextants (60),
legacy wedges (44) and octants (230). Both are SIL OFL 1.1. Run `entity --glyphs` to eyeball the
palette in your own terminal. Wedge or octant glyphs would improve the steep and curved tiles but
are not used yet, so the game does not depend on them.

- `src/ninja.rs`: movement and circle-vs-polygon collision in pixel units (24 unit tile, radius 10,
  60 Hz). Contacts are resolved deepest first; edges flush against a solid neighbour are skipped so
  tile seams do not bump. The constants equal the original N v1.2 values (see "Original N physics"
  below) and `n_rules` (default on) applies the original's structure: two wall kick strengths, wall
  sliding that persists, skid friction on release and reversal, deaths only on landings, the original's
  wall and ground detection. `ENTITY_PHYSICS=classic` turns those rules off to compare the feel.
  Slopes use plain projection plus ground friction; N++'s own slope rules are not reproduced.
  Tile seams: the end of a tile's full side is not a corner when the surface carries straight on into
  the next tile, so it gives no contact (fixed 2026-10-13: before, a flat floor tilted the floor normal
  by up to about 7 degrees near every seam, which also tilted jumps; the stored practice routes were
  regenerated). That N's own tile collision ignores such seams is assumed, not read from its bytecode.
  Wall jumping (kick of 1.0 away and 1.4 up, sliding friction when pressing into the wall) is tested as
  a real climb: kicking between two walls climbs a 40 tile shaft for gaps of 1, 2, 3 and 4 tiles, and
  hopping back into a single wall climbs about 20 tiles in 10 s of perfect input. Whether the original
  allows single-wall climbing, and how fast, is unchecked.
- `src/sprite.rs`: the ninja is N's own stick figure. `tools/extract_ninja.py` reads the original's ninja
  clip (DefineSprite 582, `testNinjaMCm`) from the SWF into `data/n-ninja.json` (embedded): 104 frames of
  ten limb lines plus a triangle head, in pixels around the hitbox centre. Its wedge-shaped shins and
  forearms become lines. The clip's frames are picked as the original does (read from the bytecode,
  2026-10-13): STAND 0 to 10 plays once at the movie's 120 fps after landing or stopping (frame 10 is the
  rest pose), SKID 11 while braking, RUN 12 to 83 advancing one frame per 0.9 px along the floor and
  restarting at its first frame on entering a run, JUMP 84 to 102 picked from vertical speed (rising fast,
  the top at 93, falling fast; there is no separate fall), WALLSLIDE 103. On the ground the whole figure
  tilts to the floor normal; in the air the tilt eases out by 10% per 40 Hz tick and a jump or wall slide
  resets it. Not ported: the four celebration animations (frames 104 to 353) and the death ragdoll.
  Drawing: limbs are one dot wide lines (Bresenham in dot space), the head fills the dots its triangle
  covers. N's figure is only about 6 px wide inside the 20 px hitbox, so its middle is spread sideways
  (`spread` in render.rs, gain 1.6 at the centre, `ENTITY_NINJA_SPREAD=1` for the original) while the
  edges stay put, so wall-slide hands still meet the wall. On a 156x47 window the figure is only about 7
  dots tall: running, skidding and falling read, standing and the top of a jump are close to blobs; from
  about 10 dots (1.5x, or a bigger window) every pose reads. Below about 5 dots across it falls back to a
  disc. Preview with `cargo run --release -- --glyphs`, or magnified (one `##` per dot) with
  `DW=2.8 DH=3.1 cargo test --release dump_figures -- --ignored --nocapture` (dot size in world units).
  The ninja can also be drawn bigger ([ and ] keys, feet anchored at the hitbox bottom); David tried the
  old pixel-art ninja at 1.5x and found it HARDER to play (it looked like it touched walls it did not),
  so the default is 1.0. The hitbox is always radius 10 because the levels are built for it.
- `src/objects.rs`, `src/drones.rs`, `src/hazards.rs`: the level objects, ported from the bytecode.
  `Objects::from_level` builds them per attempt; one-sided and moving colliders feed the ninja's
  contact solver (`Ninja::step_in`); the rest act after the ninja has moved (`Objects::update`).
- `src/shapes.rs`: solid polygon for each of the 34 N tile codes.
- `src/art.rs`: polygon to single-cell glyph matching, built once per level.
- `src/level.rs`: tile grid plus objects, 33x25 (31x23 playfield inside a one tile solid frame).
- `src/render.rs`: frame, glyphs, objects, Braille ninja, centring and the too-small message.
- `src/input.rs`: three input tiers so the game works in ANY terminal. (1) Exact: real press and
  release when the terminal speaks the kitty keyboard protocol (kitty, alacritty, foot, ghostty confirmed
  from their own source; crossterm also reports release on Windows). (2) Timing, the default elsewhere:
  a key counts as held while its events keep arriving, with the OS repeat delay and interval LEARNED from
  the events (doom-cli's technique), release noticed about 60 to 160 ms after the last repeat, a fresh
  press of one direction ending the other, and the stop key clearing at once. Unavoidable limit: before
  the first repeat a tap and a hold look identical, so that window is capped at 220 ms; with GNOME's 500 ms
  default delay a hold shows a short stall before it resumes, and a repeat delay under about 200 ms removes
  it. (3) Latched (`ENTITY_INPUT=latch`): direction stays on until the opposite key or stop. The frame
  shows `[keys: timing]` or `[keys: latched]` when input is not exact.
  Prior art read: doom-cli (learned repeat timing, capped, one key at a time), doom-ascii `-kpsmooth`,
  Robert Elder's write-up (privileged /dev/input, X11 and console raw mode are the only exact options on
  Linux), and David's own mato and rusty-pinball (fixed 200 ms stale window; only the most recently
  pressed key keeps auto-repeating, which breaks two-key holds). GNOME Terminal does not answer the
  protocol; David's session is Wayland (no X11 key state), he is not in the `input` group, and his repeat
  is 500 ms delay / 30 ms interval. The timing tier has NOT been felt in play yet: tune
  `PRE_REPEAT_CAP` and the release bounds by feel. On a terminal that never answers the protocol query,
  startup waits about 2 seconds.
- `src/pad.rs`: gamepads through gilrs (cargo feature `gamepad`, on by default, the same pattern as Ceres,
  SIGPIPE and uXo). A pad reports real press and release, so it plays like the exact keyboard tier in ANY
  terminal (hold jump for height, release cuts it short), merged on top of the keyboard in `Input`
  (`on_pad`, `on_pad_press`); a pad direction overrides a key still counted as held. Every connected pad
  drives the one ninja. Only SDL-mapped devices count (`ENTITY_ANY_PAD=1` accepts everything); David's
  8BitDo SF30 Pro (X-input mode, seen as "Xbox 360 Controller") is detected and his 3Dconnexion receiver
  ignored (checked 2026-10-13). While a pad is connected the frame shows `[pad]` instead of the keyboard
  tier, and the level-complete overlays name the pad buttons. Not yet felt in play: the stick threshold
  (0.4) and the button layout below.
- `levels/001.txt`: the practice course, 33x25 like the N levels: a one tile step, a platform two tiles
  above it carrying the switch (K), a 22 degree ramp, then a 3 tile wide wall-jump shaft up to a ledge with
  the door (X). Design numbers measured from the physics (keep rises under about 2 tiles for margin):
  tallest jump rises 3.09 tiles, a tap rises 1.77 tiles, a running jump travels about 15 tiles.
- Goal: touch an exit's switch, the door opens, reach it to complete the level (an overlay offers n for
  the next level or r to play again).
- Gold pickup works (gold returns after a death, as in N). Score is a reserved field with no rules yet.
  Lives are not implemented; N itself has unlimited retries and an episode time limit.
- `src/demo.rs`, `entity --demo [level]`: the game plays a level by itself (and again every 3 s after
  the door), replaying a machine-found exact route as key presses and releases through the real `Input`,
  so it shows exactly what the tests prove finishes the level (`the_demo_finishes_*`). Routes exist for
  the practice course (the default) and every campaign level (the route stored in its file, e.g.
  `entity --demo "3:4"`); other levels fall back to the practice course. The argument is looked up like
  the game's: an index into the full list, a name fragment or `episode:slot`. The practice route is found
  by `print_the_winning_routes`; `print_a_route` (`ENTITY_ROUTE_LEVEL=lastdoor
  ENTITY_ROUTE_WAYPOINTS="x,y;...|x,y;..."`, px; before the bar the legs to the switch, after it the legs
  to the door) searches any level leg by leg. To plan waypoints, `ENTITY_PEEK=lastdoor,0,1,30,23
  ENTITY_PEEK_STEP=8 cargo test --release peek_region -- --ignored --nocapture` prints a level's solid
  shape. The catalog preview is 3:4 the last door (re-recorded 2026-10-28), at the hub's fixed 100x30
  with the ninja drawn at 2x (at 1x that board gives the figure only about 4 dots, below the size where
  it is drawn at all):
  `ENTITY_NINJA_SCALE=2.0 python3 ../../hub/tools/castrec.py ../../hub/site/casts/entity.cast . "w19.0" -- target/release/entity --demo "3:4"`
  (one run, the level completes at 18.0 s; in the website mockup `thumbnail_at = 9.0`, the ninja in
  mid-air between the drone and the thwomp). Build with `cargo build --release` first: `cargo test`
  does not rebuild the game binary.
- Drawing over walls: a cell is reset before anything is drawn into it (ratatui's `set_style` adds to the
  old modifiers, so the ninja over a reverse video wall cell became a white block), and over a solid
  wall cell the ninja's dots sit on the wall's colour as background.
- Session logging per the Asciiverse convention: `RUST_LOG=debug cargo run 2>entity.log`.

Run: `cargo run --release [level]` (or `--demo`, see above) where level is an index, a name fragment (`lastdoor`) or
`episode:slot` (`3:2`, the campaign's level when both have one). The list is the campaign, then the
practice course; a new game starts with campaign level 0:0. Tools: `--edit`,
`--prove` and `--render` (see "The campaign"). Keys: arrows/wasd/hjkl run, up/w/k/space/z jump (hold for height), down/s/j
stop, r restart the attempt (after game over: the episode), n/p next/previous level (after finishing a
level, n continues with the clock carried over), f toggles fill versus square tiles, [ and ] shrink or grow the DRAWN ninja (quarter steps from 1.0 to 2.5 times the hitbox; start with `ENTITY_NINJA_SCALE`, default 1.0). Gamepad: d-pad or left stick
run, A, B or d-pad up jump (hold for height), Y or Back restart the attempt, LB/RB previous/next level,
Start continues after a finished level (after game over: the episode again); any pad button answers the
quit question with "keep playing". Esc asks "Quit Entity?" and only y quits (a
bare q quit was removed: it sits next to the steering keys); Ctrl-C quits immediately.

Game flow (from the original): each episode of 5 levels shares a 90 s clock that starts on your first key
press, +2 s per gold, restored to the level-start value after a death, game over at zero (r retries the
episode); the score is the time left at the end of an episode. The practice course has no clock.
`cargo run --release -- --glyphs` prints the glyph preview page.

Objects: all 13 object types of the N levels are implemented (gold, exit and switch, mines, launch pads,
trek/locked/trap doors, one-way platforms, bounce blocks, zap/laser/chaingun drones, gauss turrets, floor
guards, thwomps, homing launchers); see "Objects" below. Not done yet: the episode timer and score,
the game over and episode flow. Deaths: landings, mines and every hazard; a death restarts the attempt
(gold, switches, doors and hazards reset, as in N).

## The campaign: our own levels

`levels/EE-S.txt` (episode and slot from 0), embedded through `level::CAMPAIGN`, which lists them in
order (a test fails when a file in `levels/` is missing from it). Episodes of five share the 90 s clock
as N's do. Each episode teaches one thing at a time, then mixes:

| Level | Name | What it teaches |
|---|---|---|
| 0:0 | first light | running and jumping: a block, a ledge, a platform |
| 0:1 | stepping stones | holding jump for height: a staircase of floating stones, two tiles each |
| 0:2 | chimney | wall jumps: a 2 wide shaft up beside a block, then ledges down to the door |
| 0:3 | rolling hills | slopes: 22 and 45 degree hills to run over |
| 0:4 | under lock and key | a locked door: the switch is in a room behind it, the key up a staircase |
| 1:0 | pocket money | gold: arcs of gold over two gaps, a hump and two pits |
| 1:1 | minefield | mines: two corridors of mines, a hanging one, a drop to the door |
| 1:2 | one way up | one-way platforms (a ladder) and a trap door that shuts you in with the door |
| 1:3 | springboard | bounce blocks over mines, a launch pad, blocks in the air back to the door |
| 1:4 | fuse box | a mix: zigzag down past mines and gold, a key, two launch pads back up |
| 2:0 | sentry | a zap drone circling the block the switch sits on |
| 2:1 | clockwork | three drones on stepping blocks over a floor of mines |
| 2:2 | the chase | chaser drones in three stacked corridors |
| 2:3 | laser light | a laser drone above tiers of platforms with gold |
| 2:4 | hall of thwomps | thwomps over a corridor (keep running), a floor guard to hop over |
| 3:0 | line of sight | a gauss turret: under the roofs and back over them past cover |
| 3:1 | homing | a homing launcher above tiered platforms and walls that stop rockets |
| 3:2 | chaingun | a chaingun drone circling a central block |
| 3:3 | crossfire | a one-way ladder between two zap drones under a turret, then a descent |
| 3:4 | the last door | everything: ramp, floor guard, bounce blocks, two pads, key, drone, thwomp, shaft |
| 4:0 | long jump | running jumps over widening pits of mines |
| 4:1 | slow descent | a climb, then a 17 tile drop that only a wall slide survives |
| 4:2 | ski jump | down a long 45 degree slope, up a ramp, over a pit to a high landing |
| 4:3 | low ceiling | three corridors with mines overhead and pits below: short hops only |
| 4:4 | obstacle course | slope, pit, low ceiling, chimney and back over the top |
| 5:0 | knock knock | rooms joined by trek doors, a drone kept in each until you open it |
| 5:1 | keyring | the switch behind three locked doors, the keys up a climb and on ledges |
| 5:2 | trapped | the crawlspace to the switch holds the exit's trap trigger; go over the top |
| 5:3 | airlock | three floors with locked hatches, each key on the floor above |
| 5:4 | vault | the key past a drone's room and up a climb, the switch in a locked vault |

Planned episodes (a proposal, not yet settled with David): 6 mines, harder; 7 launch pads and
bounce blocks; 8 zap drones and chasers; 9 thwomps and floor guards; 10 lasers; 11 gauss turrets;
12 homing rockets; 13 chainguns; 14 one-way platforms and doors as route puzzles; 15 to 18 mixes of
rising difficulty; 19 the finale.

Episodes 4 on are written as short Python scripts, `tools/campaign/epNN.py`, on `tools/mklevel.py`
(tile coordinates in, the text format out). Re-running a script keeps a level's stored route when the
level is unchanged; a changed level loses it and needs `entity --prove` again.

The text format (`Level::parse`, written back by `Level::to_text`): the 33x25 tile grid (`#` full,
`2`..`Q` tile codes as `chr(48 + code)` except 27, written `k` because `K` is the switch; `S` spawn,
`X` door, `K` switch), then after an empty line one item per line: `name`, objects in the original's
parameter layout (`gold x y`, `mine x y`, `drone x y movement chaser weapon dir`, `door ...`, `oneway
x y dir`, and so on, px with the frame included), `exit` and `spawn` lines when the grid letters cannot
say it, then `waypoints` and `route` for the route bot. Every N level and the practice course survive a
save and load unchanged (tests).

- `entity --edit [file [level]]` (`src/editor.rs`): the level editor, drawn by the game's own renderer.
  Arrows move the cursor (shift: 4 tiles), space or Enter places the brush, x/Delete erases (objects
  first, then the tile), Tab or [ ] picks the brush (tile, spawn, exit, gold, mine, drone, turret, floor
  guard, thwomp, homing launcher, launch pad, bounce block, one-way platform, door, locked door, trap
  door), , and . pick its variant (the 33 tile shapes, drone kinds, directions, door edge), r turns a
  drone's start direction, e picks up the tile under the cursor, u undoes, n names the level, s saves, p
  test plays it through the real game loop (on an episode clock, so the time shows; Esc comes back), Esc
  or q quits (asking when there are unsaved changes). Mouse: left places, right erases, dragging too. An
  exit, a locked door and a trap door take two places (the door, then its switch, key or trigger). A file
  that does not exist yet starts empty, or as a copy of the named level (`entity --edit levels/06-0.txt
  lastdoor`). Session logging covers keys, mouse, every edit, saves and test runs.
- `entity --prove <file>...` (`src/bot.rs`): the route bot searches the level (leg by leg through its
  `waypoints`, tile centres in px, `|` between the legs before and after the switch; without waypoints a
  plain search), replays what it found in the real game and writes it into the file as `route`.
  Waypoints on the gold make the route collect it.
- `entity --render <file or level> [cols rows [ticks]]`: the board as plain text, after `ticks` frames
  with no keys held (to see where drones have gone).
- Proof: `every_campaign_level_is_finished_by_its_route` replays every stored route in the real game and
  requires the level complete, no deaths and all gold collected, and an episode to use under 45 s of its
  clock net of gold at the bot's pace (`cargo test --release campaign -- --nocapture` prints the table).
  At the bot's pace the episodes use 38, 28, 36, 18, 18 and 22 s net. A route proves a level from a fresh start:
  the chaingun's spread and the turret's timing read the game's tick, which in play carries on from
  earlier levels, so in play those hazards fire with other timings than in the proof.

## Level data from N (removed 2026-10-28)

N v1.2's 150 levels were extracted from `Nv12.swf` (kept in `references/nRef/nv-12.zip`, outside this
repo) by `tools/extract_n.py`, and were playable here until 2026-10-28, when David had them removed.
The tool stays for studying the original locally (`python3 tools/extract_n.py path/to/Nv12.swf
out.json`); its output does not go back into the repo. What the data taught, still true of the game:
tile ids and object type ids come from the original's TID_* and OBJTYPE_* constants, and slope
orientation and small/big naming were checked on all 150 levels (slope borders line up between
neighbouring tiles in 1249 of 1250 cases).

## Playtesting by machine (how "can the level be finished" is checked)

The route bot (`src/bot.rs`, used by the playtest tests in `ninja.rs` and by `entity --prove`) searches
for key schedules through the REAL `Input` code with a simulated keyboard (presses plus OS style
repeats) and the real physics, then replays stored winning routes through the real `Game` as ordinary
tests (`the_practice_course_can_be_finished_with_*`). Added 2026-10-27 for the campaign: the state key
includes a coarse clock when a level has hazards that move or keep time (so waiting for a laser to stop
is possible), each waypoint leg keeps up to three ways to its goal and the next leg falls back on them
(a leg can end at the switch just as a laser fires through it), and the bot's world clock starts at 0
like a fresh game's (before, a chaingun route found by the bot died in the game). Findings so far
(2026-10-02):
- The course is finishable in about 8.5 s with exact keys and 8.9 s with GNOME's default 500 ms repeat
  delay (10 jump presses; the timing route is mostly "hold right, tap jump on a rhythm").
- Bugs it found: in timing mode a second tap about one repeat delay after the first was read as a key
  repeat, so rapid jump taps (wall hopping) were swallowed; timing mode jumps were cut to 13 frames
  (1.77 tiles) because a tap cannot be cut short anyway, so they are now full length by default; and the
  first practice course needed a 3.0 tile jump when the maximum is 3.09.
- The search itself needed care: a greedy heuristic gets stuck under platforms (use breadth-first for
  the switch leg, greedy for the door leg). A failed search proves nothing; check with simple scripted
  schedules before believing "impossible".
- Added forgiveness: 6 frame coyote time for ground AND wall jumps, and a 6 frame jump buffer.
- Ignored tests to regenerate or explore: `cargo test --release print_the_winning_routes -- --ignored
  --nocapture` (new fixtures), `can_a_person_finish_it` (exact vs timing).

## Original N physics (decoded from the SWF bytecode, 2026-10-03)

Read with `tools/asdec.py` (a small symbolic decompiler; `python3 tools/asdec.py Nv12.swf TAG`). The player
lives in DoAction tags 2707 (setup, `TickNormal`, collision hooks), 2709 (`Think`, the state machine) and
2710 (`Jump`, `Fall`, state changes). Everything below was READ from the bytecode, not remembered.

- The simulation is a fixed 40 Hz step (`APP_GAMETIME_TICKLEN = 25` ms); the SWF header's 120 fps is only
  the movie. The episode timer counts ticks. Units are pixels, tile 24, ninja radius 10 (`r = tiles.xw * 0.8333`).
- Verlet integration: `pos += drag * (pos - oldpos) (+ g)`, then collide, then `Think()`.
- Constants per 40 Hz tick: groundAccel 0.15, airAccel 0.1, normGrav 0.15, jumpGrav 0.025, normDrag 0.99,
  wallFriction 0.13, skidFriction 0.92, standFriction 0.8, maxspeed 5 (r * 0.5), jumpAmt 1, jump_y_bias 2,
  max_jump_time 30 ticks, terminal_vel 9 (r * 0.9). Converted to Entity's 60 Hz tick (velocity x 2/3,
  acceleration x 4/9, a per-tick factor f becomes f^(2/3)) they EQUAL the values already in `ninja.rs`
  (0.0666, 0.0444, 0.0111, 0.9933, 0.9113, 0.9459, 3.333), except standFriction 0.8 -> 0.8619.
- `Jump(x, y)` is a position impulse: first cancel the current velocity component if it opposes the jump
  (`if vx*x < 0: vx = 0`, same for y), then `pos += (x * 1, y * 3)`. Jump starts the low gravity phase;
  it ends (Fall) when the jump key is released or after 30 ticks. While JUMPING the wall logic is skipped.
- Ground jump: `Jump(floorN)` (along the surface normal, so on slopes it tilts and loses height), or
  `Jump(0, -0.7)` when pressing toward the slope's rising side.
- Wall jump (any airborne non-JUMPING state with a wall touching, on a fresh jump press):
  pressing TOWARD the wall: out 1.0, up 0.5 (x3); otherwise (neutral or away): out 1.5, up 0.7 (x3).
  In Entity's 60 Hz units that is (0.667, 1.0) against (1.0, 1.4). The second is what `ninja.rs` had.
- Wall slide starts when falling (vy > 0) while pressing toward the wall. Once sliding, vertical speed is
  cut by wallFriction every tick even with no key held; pressing AWAY from the wall ends it (Fall).
- Walls are found by point queries at (x +- (r + 0.1), y) and by collision normals with ny == 0.
  ANY floor normal with ny < 0 counts as ground, including 67 degree slopes.
- Ground states: standing (friction 0.8), running (accel while the keys push), skidding (friction 0.92 when
  the keys are released or reversed, until along-surface speed < 0.1, then standing).
- Death by impact is ONLY a landing: `oldv . floorN - 2*|floorN.y| < -9` (40 Hz units), i.e. in 60 Hz
  units a normal-direction speed above 6 - 1.333*|ny| (4.67 on flat floor, about a 6.8 tile fall). Walls and
  ceilings never kill. Crush: a collision push longer than sqrt(0.8) * r kills.
- Mines, drones and the other objects are in tags 2693 to 2706 (not yet decoded in detail).

## Objects (decoded from the SWF bytecode, 2026-10-03)

Record layouts are `type^params`; all positions are pixels in the frame-inclusive 33x25 tile grid.
Ticks are the original's 40 Hz; Entity converts (speed x 2/3, a count of ticks x 1.5).

- gold `[x,y]`: radius 6, picked up within 6 + 10 of the ninja.
- exit `[door x, door y, switch x, switch y]`: switch radius 6 (touch within 6 + 10 opens the door), door
  radius 12 (touch within 12 + 10 while open completes the level). Levels may have several exits.
- mine `[x,y]`: radius 4, kills within 4 + 10 of the ninja, once.
- launch pad `[x,y,nx,ny]`: radius 6; when the ninja is not more than a radius in front of it (along the
  normal) the ninja's velocity is SET to normal x 5.143 (x (1 + |ny|) when the pad points up) per 40 Hz tick.
  An upward pad lifts the ninja about 10 tiles (checked against the original's own numbers).
- door `[trigger x, trigger y, vertical, trap, I, J, locked, dI, dJ]`: a thin barrier on the RIGHT edge of
  tile (I+dI, J+dJ), or its BOTTOM edge when vertical is 1 (indices are frame-inclusive; 361 of 365 doors
  sit in open space on both sides). Trek (neither flag): opens within 10 + 10 of the barrier, closes
  5 ticks after the ninja leaves. Locked: opens for good when its separate switch (radius 5) is touched.
  Trap: starts open, slams shut for good when its trigger is touched. Closed doors also block drones.
- one-way platform `[x,y,dir]` (0 right, 1 down, 2 left, 3 up): a full tile that blocks only from its facing
  side and only a ninja that was outside the face before the move.
- bounce block `[x,y]`: 19.2 px box on a spring (stiffness 0.05, mass 0.2): the block takes 80% of an
  overlap, the ninja 20%; it sleeps 40 ticks after the last touch (Entity snaps it to its anchor then).
- drone `[x,y,movement,chaser,weapon,start dir]`: radius 9, speed 0.857 px/tick (zap x2, laser x0.5,
  chaingun x0.75, chasing x2); moves cell to cell over open edges (both cells empty, no closed door) and at
  each cell picks the first open direction from a turn priority list: surface follower cw [90,0,270,180],
  ccw [270,0,90,180]; wanderer cw [0,90,270,180], ccw [0,270,90,180]; alternating and "random" switch
  lists on every turn (random uses the tick parity). Weapons: zap kills on touch (chasers charge down a
  clear row or column at double speed); laser sees the ninja, fixes a beam toward it, charges 30 ticks
  (thin warning line), keeps the beam 80 ticks, kills when the ninja is within r of the beam, rests 40;
  chaingun charges 35 ticks, fires 4 to 8 hitscan bullets every 6 ticks in a spread of 0.2 to 0.4, rests 60.
- gauss turret `[x,y]`: waits for line of sight, then a crosshair drifts toward the ninja's position one
  tick ahead (speed 0.03 far, 0.035 mid, 0.05 close), a shot timer (60) drains faster the closer the
  crosshair is, then 10 ticks of charge and a hitscan along the line to the crosshair, 10 ticks rest.
- floor guard `[x,y,dir]`: radius 6, speed 5.14 px/tick, patrols its platform (to the first wall or ledge),
  rushes when the ninja is in its row, kills on touch.
- thwomp `[x,y,dir]`: 9 px half size, waits until the ninja is aligned (within 38 px across) and
  somewhere along its fall path, slams to the end of the empty run at 4.29 px/tick, rises at 1.71. Its front
  face kills, the other faces are solid.
- homing launcher `[x,y]`: line of sight, 10 ticks of charge, a rocket accelerates (0.1, x1.1 per tick) to
  3.43 px/tick and steers 0.1 toward the ninja's next position, exploding on a wall or on the ninja.
- Not yet checked against the original: tile edge states "interesting" (some 67 degree slopes) are
  treated as blocked for drones; partial tiles count as non-empty for the drone grid.

## Session log: 2026-10-03 (done while David was away)

What changed, in the order it happened. Everything below passes the test suite (71 tests).
- Input research: Claude Code and opencode need only key PRESSES (and modifiers), which is why they feel
  good; they probe the terminal and enable the kitty keyboard protocol when it answers (Claude Code's
  binary has a `kittyKeyboard` capability), the same as Entity's exact tier. Node would not help.
- Decoded the original's player code and every object from the SWF (see the two sections above), and
  applied what it showed: wall kick strengths, persistent wall sliding, skid friction, landing-only deaths,
  wall and ground detection, slope assist. `ENTITY_PHYSICS=classic` turns those rules off so the feel can
  be compared (the stored practice routes are for the original rules).
- Implemented all 13 object types, an episode clock, score and game over. All 150 real levels load and
  run for 15 seconds with every object active (a test).
- Not verified: a human has not played any of this; the kitty input path; the timing input feel; the
  ninja art; whether the original allows single wall climbing; the "interesting" edge states for drones.
- A search for routes through real N levels does not scale (they are 30 to 60 s puzzles); only the
  practice course is machine-verified finishable. Objects are verified by unit tests against numbers
  read from the original (for example the launch pad rise matches to within 2 px).
- Open question for David: what exactly felt wrong about wall jumping. The original's rules are in, so
  differences you feel now are either tuning I did not port (for example the original's collision on
  slopes) or Entity's own additions (the 6 frame coyote time and jump buffer).
- Suggested next steps: play 0:0 through 0:4 and report feel; install a terminal that reports key release
  (kitty, alacritty, foot or ghostty are all in apt) to feel the exact tier; decide what lives should do.
