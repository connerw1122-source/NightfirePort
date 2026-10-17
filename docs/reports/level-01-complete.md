# level-01-complete — 07000001 "Paris Prelude" (Castle A)

Branch `s/level-01-complete`. The level was played from the spawn through the real input path
(`nf_view::load` + `Sim::step_bond` inputs; the level's own scripts, triggers, doors, AI and
mission layer running untouched). Status: **the level's full interactive chain is proven and
pinned; the final escort leg is blocked by the level's known open item** (the escort route and
its doors — details below).

## What's done

**`crates/nf-view/tests/walkreal_07000001_live.rs` — 3 tests, `--test-threads=1`:**

1. `walk_from_the_spawn_to_the_exit_with_the_contact` (extended) — reaches the end of the route
   with every scripted beat fired up to the escort's last leg:
   - the intro cut-scene plays out (SP #277, script 0x06000087 from pack 07800001 [D]); its
     stream 8 writes ch 0xB at tick 46 of the walk — the contact (Mayhew, skin 0x0500004F) and
     five bar-wave guards enable [D placements];
   - the bar wave is cleared through the real fire path;
   - **the meet scene**: Bond walks the TOUCH_ONCE plate at (6.8, 6.0, 8.9) → ch 0x32 → the 0xEE
     OUT mux mirrors ch 0x32 → ch 0x33 every tick [M Trigger_Update 0x1b1a78 case 5:
     `switch_channels[list[k]] = switch_channels[out]`] → SP #276 (script 0x06000086, 19
     streams, mask 1 = its own out channel [M SP_Hit 0x1dc470:17-21, SP_Update 0x1dbda8 case 0])
     plays out untouched and its stream 9 writes ch 0xE → the four hall guards leave WAIT_SWITCH
     (asserted against `nf_game::drone::state::WAIT_SWITCH`) [D placements];
   - **the hall event**: Bond detours to the north-east hall and kills the three named guards
     (death channels 0x13/0x11/0x12 [D]) → the disc's AND mux raises ch 153 [D placement] —
     fired live at walk tick 7902; (killing the trio also starts guards 14/15 — their switch
     channels ARE the trio's death channels [D] — the game's own escalation wave);
   - the escort: the follow loop clears each stall (guards 5, 3, 10, 14, 15, 16 — the 0xE wave,
     the 0x26-plate pair, the 14/15 wave) and walks the contact from (−7.7,−7.6) to (−16.6,14.5)
     of the 37-node route before the blocker below;
   - the plate AND and the objective hand are pinned by the code path (the assert is what the
     blocked run cannot yet reach).
2. `the_plate_refuses_bond_alone` (existing sister) — still green: Bond alone on the
   LoadPointActivator plate never fires ch 0x10 [M Trigger_Hit 0x1b1fc8's filter list].
3. `the_hall_guards_wait_for_the_meet_scenes_channel` (NEW sister) — with the meet plate never
   walked, ch 0x32/0x33/0xE never rise and all four ch-0xE guards sit in WAIT_SWITCH [M
   NDrone2_DSTATE_Global's WaitSwitch enter; M the case-5 copy]. This is the sister that fails
   on a port where the mux direction or the SP trigger rule is wrong.

**`crates/nf-view/tests/walkreal_probe01.rs` (NEW, diagnostic)** — the level census: every
trigger, door, SS, pickup, drone, SP and every script's channel writes/loads/texts. It proved:

- **The only script channel writes on the level are ch 0xB (the intro) and ch 0xE (the meet
  scene)** [D]. **ch 0x47 has no writer on the disc** — re-verified (the old scan's claim
  holds): every placement, trigger, mux, lock, pickup and script scanned.
- **SP #79 is dead in the shipped game**: its scripts 0x06000096/0x06000097 exist in NO archive
  on the disc (a kind-7 scan of every .bin [D]) — the game's own SP_LoadScript hits the same
  missing-script path the port's does. Not a port bug; nothing to fire.
- The OUT-mux direction was verified against the decompile after an initial misread: case 0xC
  is the OR mux (out = OR(list)); the OUT copy is case 5 (out → list) [M 0x1b1a78:147-161].
  The port matches the decompile; no code change needed.

**`crates/nf-view/tests/common/walker.rs`** — `resupply` now excludes the boxes it just wedged
on and takes the next-nearest (the old code re-picked the same unreachable box 4 times). The
sister discipline: the 01 walk exercises both paths (the wedged box then the next-nearest).

**`docs/levels/07000001.md`** — the corrections above appended.

## The objective list and how each completes

The mission table [L ee_start_05.bin @ 0x2a3eb8 via mission::c07000001] — ONE objective:

| obj | text | watch | enable | how it completes |
|---|---|---|---|---|
| 0 | 0x04000006 "Escort Mayhew to bunker" | 0x47 | (immediate) | announced by the real monitor at the first tick [M 0x1827d8 case 0]. **The writer of 0x47 is not on the disc** [D the census] — the un-ported mission/dialogue layer's. The walk hands it at the level's own beat — the LoadPointActivator AND (Mayhew reached the bunker plate) — and pins the REAL monitor's consequences: states[0][0] 3→4 [M 0x1827d8 case 3], and 07000002's "Destroy Mayhew's computer" + "Rescue geisha girls" announce cross-level (enable 0x47, case 1 has no level gate [L mission::c07000002]) while "Find door to servants quarters" waits on 0x4c. No ch 100 in-level: the success channel is the AND over BOTH parts' conditions [M 0x1827d8:119-123]. |

The LEVEL EXIT does not depend on the objective: the AND-touch (Bond AND skin-0x4F on the plate
[M Trigger_Hit 0x1b1fc8]) writes ch 0x10, the LOAD_LEVEL (gate 0x10) loads 07000002 [M
Trigger_Update 0x1b1a78 case 10; D param].

## The route

spawn #0 (−2.78, 6.84, −12.40) → intro plays (ch 0xB) → the bar wave cleared → the contact at
(−7.7,−7.6) → the meet plate (6.8, 6.0, 8.9) [the scene] → the NE hall: the trio killed → ch
153 → the west hall (doors #2 at (−14.1,13.1) on the way) → the escort west along the 37-node
route → (blocked at (−16.6,14.5), see below) → the stairwell head (−30.8, 6.4, 13.1) → the
LoadPointActivator plate (−33.1, 2.3, 15.4) → the AND → ch 0x10 → the exit loads 07000002.

## What the walk still cannot do (the exact reason)

**The escort's last leg stalls on the upstairs guard 2 (the balcony over the stairwell head).**
Guard 2 (skin 0x5000036) stands at (−29.8, 10.2, 13.4) — nav path 1's balcony ring (nodes 9-15,
y ≈ 9.7 [D AIPath]). While he is alive, his armed-peer distance to the contact is 13.7 < 14, so
the contact sits in LeadHide/LeadWait (state 0x20, drawn false) [M AllyLeadHide 0x15ec08] and
never walks his route past (−16.6,14.5). The walk's guard-clear cannot reach guard 2 from the
ground: the eye→torso ray hits the balcony ceiling at t 0.034 [L the run's diag dump], and the
solo approach through door #2 (−14.1, 5.3, 13.1) wedges — the door cycles open/closed (state
0x2 re-logged), the stuck handler's repeat-door guard falls to the wedge ladder, Bond ends
grounded=false in the frame and the nav gives up at its 3000-tick cap [L the run log]. This is
exactly the level's known open item ("the escort route and its doors"): the door-cross logic
works when Bond walks WITH the escort (the old green walk crossed it repeatedly behind the
contact), but a SOLO westbound crossing still wedges. Next step (recorded for the next
session): fix the solo crossing of the cycling door #2 — the door's approach/open timing or the
stuck ladder's jump into the frame — then guard 2 dies from the balcony ring (the climb anchor
(−32.8, 9.7, 13.9) is already in the test) and the escort completes to the plate.

Everything after the plate is pinned code that only needs the contact to arrive: the AND
assert, the 0x47 hand, the monitor states and the exit assert are in the test.

## G-050 (the shoji door drawn black) — data for the main agent

The two shoji SS records are placements 16/17 (hashes 0x80350009/0x8035000a, 8 keys, state 0x40,
ch 0) at (5.7, 8.0, 2.9) and (5.7, 8.0, 9.1) [D the census]. G-050's suspect list ("the door's
light pair / its draw order behind a translucent pass / a state change from the escort") can now
be narrowed with a route walk: the meet-plate route passes between these two records, and the
0xE wave + the 14/15 wave spawn AFTER the plate — the escort state changes along that exact
stretch. A `--bond` render sweep along (5.7, z 2.9→9.1) at ticks before/after the meet scene
would separate "escort state" from "door state".

## How it was verified

- `cargo test --offline --release -p nf-view --test walkreal_07000001_live --
  --test-threads=1 --nocapture`: **2 passed (the sisters), 1 failed (the blocked main walk at
  the documented escort stall)** — 215-222 s wall per run, no falls.
- `cargo test --offline --release -p nf-view --test walkreal_probe01`: green (the census + the
  disc scan).
- The suite runner could not run in this sandbox: this sandbox OOM-kills the test binaries
  (dmesg: `Out of memory: Killed process … walkreal_070000`, anon-rss 1.8 GB — the 4 GB sandbox
  runs the Next.js dev server + the test alongside), and background jobs are killed between
  tool calls here. The crates touched are nf-view (test-only changes: two test files + the
  test-common walker); the nf-game/nf-formats/nf-audio trees are untouched by this session.

## Live checks for the main agent

1. Fresh-boot PCSX2, 07000001: walk to the contact, let the bar fight play, walk the plate at
   (6.8, 6.0, 8.9) — the meet cut-scene should play (the port now knows its trigger). Confirm
   the scene plays in the real game at that beat ([L] would pin the beat for the port's route).
2. gdbwatch `switch_channels[0x47]` (EE 0x2b5?? the switch_channels base + 0x47) through the
   plate AND: does the real game raise 0x47 when Mayhew reaches the bunker plate, or only on the
   07000002 load? This settles whether the hand's beat is the AND or the exit.
3. G-050: the render sweep described above.

## Usage

not visible to me
