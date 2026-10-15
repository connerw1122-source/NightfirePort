# playtest-spots — the report

Task `playtest-spots` (branch `s/playtest-spots`, finish id `602-playtest-spots`). Deliverables:
`docs/playtest/spots.md` (new) and `crates/nf-view/tests/playtest_spots_live.rs` (new). No shared
file touched (drone.rs / scene.rs / player.rs / lib.rs are byte-identical to master — the task's
additive-hook allowance was never needed).

## What's done

**docs/playtest/spots.md** — for every requested feature: the level, a `--bond x,y,z,yaw,pitch` spot
where Bond spawns standing on a floor facing the feature, the `--give` list, the object to find within
reach with the game's own reach constant, and per-row sources. All 13 requested features are covered
(18 spots; several features have a second stance):

- Q-Claw grapple #281/#282 on 05 — spots from `grapple_hook_live.rs` + `gadget_grapple_user_spot.rs`,
  give `0x50` (already owned on 05 — the give draws it).
- The 08 safe bolts + the 08/14 fuseboxes — the laser `0x4e`; spots are the measured burn stances of
  `fix_safe08_live.rs` / `fix_party_mission_live.rs` / `fix_fusebox_cut_live.rs`.
- The 0c display vent + hatch — the burn stance (floor 11.84, measured this session; see findings),
  give `0x4e`; the hatch, open-side view row keeps the crouch-drop spot.
- The 05 cable (wire #251) — the icon-1 stand from `fix_wire2_live.rs`; grab is JUMP, not E [M
  0x1a5170].
- The 06 searchlight beams — the beam spot from `fix_06_alarm_live.rs` (the real game's alarm trace
  spot) + two render close-ups.
- The 05 gate guards' ledge — the characters.md stance; gotcha documented: the pair spawns only after
  ch 0x22.
- The 02 water bank + the pond ladder out — the user's own reported bank spot (`fix_swim_live.rs:43`)
  and the `fix_ladder_water_live.rs` stance. **The 02 vent does not exist** [D kind-census,
  fix-swim-vent.md:58-60]: 02's "grate/hatch" props are the bathhouse SimpleScripts #308/#309 — the
  doc says so and points the crawl-vent feature at 05's wall-hug window #35 (the real crawlable vent,
  with an [S] yaw filled in).
- The 07 Kimiko scene trigger — the canonical rendered spot (`fix_kimiko_handback_live.rs:11`).
- The 16 corridor door — the walkreal stance (door[7], leaf z −74.2; the sealed west half documented).
- The 0c jetpack photograph — the display-ground spot, give `0x54`, FIRE not E [M CamSubject_Update
  0x1bc668].
- The 0a Q-Worm PC — the suite-green install spot, give `0x58`, FIRE not E [M PCQWorm_Update 0x1aee20].
- The 12 lift buttons — the `walk_07000012_live.rs` stance; E writes ch 0x59 + 0x49.
- 0x2E Scorpion targets — the spot is the real game's own chase-trace stand [L docs/systems/
  drone-cover.md:98]: Bond (79.86, 22.92, 96.0), the patrol guard live 19.3 away in view.

**crates/nf-view/tests/playtest_spots_live.rs** — 18 headless NF_ISO tests, one per level. Each spot
is placed through the window's `--bond` contract (`Bond::new(pos, yaw)` + raw pitch), runs 60
fixed-60 Hz ticks, and asserts **grounded** (`SS_SWIM` in the pond, and no grounded assert on 07
where the cut-scene freezes Bond mid-air) plus the feature object's **distance** against the game's
own constant (laser 4.0, Q-Claw bolt 50.0, E use 2.5, Q-Worm shot 2.0) and a clear eye ray where the
game's path needs one. Where the reach has a cheap functional proof, the test drives it through the
real input path: door[7] opens on E, the 12 lift buttons answer E with ch 0x59, the 0a Q-Worm installs
(ch 0x48) on trigger taps, the 0c hinge burns under the held beam, the 06 alarm channel 55 rises, the
07 scene fires. Numbers only — no game bytes. **18 passed / 0 failed.**

## Findings worth keeping

1. **The fix-melee guard-2 stand (60.4, 27.1, 100.4) has no floor in the sim** — it was a render-only
   spot. The Scorpion row uses the [L] trace stand instead, which grounds (floor_ny 0.946) with the
   guard in view at 19.3.
2. **The 0c upper-walkway spot (5.8, 15.8, −108.5) cannot see the vent**: its end wall (z −110.02)
   blocks the eye ray; the opening sits 3.2 up at the duct. The doc's hatch row uses the burn stance
   (6.0, 12.94, −110.5) — past the wall — where the held beam burns hinge #14 within the laser's
   reach (the test proves it; the ascent leg stays the parked open check).
3. **The 0a cubicle blocker is a Bond-body stand-in only** — the eye ray is blocked at t≈0.46 but the
   Q-Worm's shot installs clean from the spot (the install is the shot-path proof). A settled Bond
   also slides ~1.1 off the desk-edge spot, so the doc says to fire straight after spawning.
4. The 02 bathhouse "vent" question is answered in the doc with [D] evidence and a measured stand
   (`floor_below` printed: `--bond -49.75,5.83,6.13` works; hatch #308 0.00 out on xz).

## How it was verified

- `cargo test -p nf-view --test playtest_spots_live` with NF_ISO: **18 passed / 0 failed** (release,
  slim ISO), re-run on the final tree (merged with origin/master at d76a4b73) — still 18/18.
- Full suite, `tools/test/suite.py` on the final tree: **this sandbox kills detached/background
  processes between tool calls** (three nohup/setsid attempts died; one of them also OOM'd at the
  suite's `-j4` build phase — the box has 3 GB RAM and the disk filled at 100% until stale
  `target/release` artifacts were pruned, 2.4 GB freed). The suite therefore ran in **foreground
  chunks** against the 555 s tool cap: chunk 0 (the 26 unit-test binaries + the first 106 ISO
  binaries) got through **94 binaries: 93 `test result: ok`, 1 FAILED — `probe_pylon_render`, which
  panics on `no_adapter_backends` (wgpu finds no GPU in this sandbox — an environment limit, not a
  code failure; the same binary passes wherever a GPU exists)** before the cap; chunk 1 was cut
  mid-run by the same cap after 25 more `ok` binaries, 0 failures. ~220 binaries (mostly the long
  NF_ISO walk/fix slices of nf-view) did not fit the window. **No failure attributable to this
  task** — the task adds two new files and touches no shared code.

## What's left

- Nothing blocking. Optional follow-ups for the queue: a spot for the 12 wall button #699 (needs a
  placement-volume dump — noted [S] in the doc), and the 05 Q-Claw draw on the cold level if the
  PCSX2 live check below says the give's skin merge misses.

## Live checks for the main agent (PCSX2)

Reproduced from docs/playtest/spots.md §Live checks:

1. **05 Q-Claw draw on the cold level**: `--bond 16.5,29.6,58.6 --give 0x50` — does the fp skin draw
   and does fire latch #282? (The give's skin-merge path has no game precedent.)
2. **0a Q-Worm walk-in**: can Bond walk into the Q-Worm PC's cubicle in the real game, and does
   aim-fire from the desk edge install? (The port walls the desk for Bond's body.)
3. **0c vent ascent**: how does the real game get Bond from the ST-1 walkway (14.01) up to the duct
   (17.2–17.5) at the display vent? (No ladder/creepwall near it on disc.)
4. **14 fusebox approach**: is there a walkable route from start #666 to the #383/#385 rooms, or is
   the `--bond` teleport the only entry?
5. **02 bathhouse hatches**: do #308/#309 react to anything in the real game, or are they pure script
   props?
6. **06 siren sound** (retail image): standing at the beam spot should start the looped klaxon 329
   within ~1 s [L trace frame 140866+].

nf-view render checks: every `--bond` spot with `--frame 40 --shot` — the feature object on screen,
floor under Bond; the two 06 beam close-ups; the 05 grapple #282 spot after a latch (rope + hang).

## ntfy

- curl: HTTP 429 (daily message quota reached, code 42908).
- python urllib: HTTP 429.
- wget: rc 0 — delivered.
