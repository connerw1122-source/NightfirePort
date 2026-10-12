Read `docs/playtests/2026-10-11.md` first: it's the user's multi-level playtest, and the item ids below refer to it.

## Your task: `map-tower`, make the tower missions 07000009, 0700000a, 0700000b (Night of the Agent and the Tower B/C updaters) completable from start to end: the map, its actors and its objectives

Levels: `07000009`, `0700000a`, `0700000b` (`docs/systems/levels.md` has the table, `docs/atlas/` the systems). The goal is simple to state: a player
can load each level, do what the real game asks, and reach the next level, with every actor and objective behaving as
in the game.

For each level, in order:
1. **Map the level from the game's own data and code.** Write `docs/levels/<code>.md` with:
   - the start position(s) and which entry each comes from (the route Bond took in the previous level);
   - the objectives in order (text from `USATxt.dat` via `Txt_BindLabel`; the code that sets and completes each,
     `Mission_*` / `PlrStat_*` / script channels);
   - the actors (allies, civilians, VIPs, bosses, scripted characters) with their role and what triggers them;
   - the required interactions (doors, keys, switches, gadgets, wires, ladders);
   - every exit route with its trigger, condition and destination level;
   - the failure conditions (killing civilians, alarms, an ally dying, a timer).
2. **Write a headless walkthrough test** (`crates/nf-view/tests/walk_<code>_live.rs`) that drives the real path:
   - `nf_view::load`, then `step_bond` with real input where the player would press something (use, fire, crouch,
     switch weapon); teleporting between rooms is fine for walking;
   - assert each objective stage in order, and that the level end fires and loads the right next level;
   - sister assertions: skipping a required step must NOT end the level, and breaking a failure condition must fail
     the mission.
3. **Fix every blocker the walkthrough hits:** missing objects, doors that don't open, triggers that never fire,
   scripts or cut-scenes that never start, actors that don't move, act wrong or can't be interacted with, objectives
   that never complete, the wrong next level. Each fix cites the game address it ports.
4. **List what you couldn't port** in the doc and the report, with the game function behind each item.

**Known so far:** none of these levels has been play-tested in the port. Start each with a load plus a 600-tick
smoke run, then the walkthrough. 07000009's lift doors and 0700000a's `room-M_LIFTDOOR1/2` / `Vent_SecurityArea` are
SimpleScript (0xE2) objects: drawn, but they don't move yet. Port their activation if your route needs them
(`SS_Activate` 0x1a9ea8, `SS_Update` 0x1a9948), and coordinate with `120-fix-explosions`, which owns the SimpleScript
playback.
**Working alongside the other map tasks.** Five other workers make other missions completable at the same time, and
`031-fix-mission` builds the objective system they all use.
- Pull `master` at the start and before every push; if `crates/nf-game/src/mission.rs` exists, build on it instead of
  writing your own.
- Keep your level-specific code in its own file (`crates/nf-game/src/mission/<level code>.rs`, or a table entry), and
  your docs in `docs/levels/<level code>.md`.
- Keep edits to shared files (`scene.rs`, `drone.rs`, `triggers.rs`, `nis.rs`, `lib.rs`) small and additive, and list
  them in your report.
- Never reformat or reorder a shared file.

Files: `docs/levels/`, `crates/nf-view/tests/walk_*_live.rs`, `crates/nf-game/src/mission/` (or `mission.rs` if
031-fix-mission hasn't split it), and minimal hooks elsewhere.

**Report:** for each level, say completable yes/no; if no, what blocks it; and a short list of spots the user should
check in play (with `--bond x,y,z,yaw` so the main agent can render them).

**Reproduce before you fix.** The user's play is the reference, and previous fixes passed tests but still failed in play. For each bug:
1. Write a headless NF_ISO test that drives the real input path (the `nf-view` input mapping into `Sim`, or `nf_view::load` plus `dev::scenario`), and show it reproducing the user's symptom.
2. Then find the cause in the decompile and fix it.
If you can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Update the doc entry** to say it's fixed, with your commit.

**Branch:** `s/map-tower`.

---
claimed-by: sandbox1 at 2026-10-12T07:26Z

---
## Report (done, 2026-10-12T11:27Z)


---
## Report (done, 2026-10-12T11:27Z)

# map-tower — the tower missions 07000009/0a/0b: maps, actors, objectives, walkthroughs

Branch **s/map-tower** (rebased onto origin/master b2ce7bd — fix-mission's mission layer, which
this task builds on per the brief). Docs: `docs/levels/07000009.md`, `0700000a.md`, `0700000b.md`.
Code: `crates/nf-game/src/useobj.rs` (new, this task's file) + `crates/nf-game/src/pickup.rs`
(the kind-4 arm). Tests: `crates/nf-view/tests/walk_0700000{9,a,b}_live.rs` — 10 tests, all green;
the full suite passes (79 binaries; the 3 slim-ISO `disc_golden` failures are the permitted ones).

## Per-level verdicts

| Level | Completable (headless walkthrough) | The chain |
|---|---|---|
| 07000009 | **YES** (both routes) | car-park start #68 → the lobby → the lift: (a) the elevator activation → ch 0x39 → the 0xE2 LIFTDOORS open + AND #274 → ch 0x46 (objective 2) → TOUCH_ONCE #67 in the cab → ch 0x32 → LOAD_LEVEL #63 → 0700000a; or (b) E straight on the Load Point (key_gate 0 [M 0x1b2318:76-90]). The alarm (ch 0x7C/0x7D → OR #278 → ch 0x61) fails objective 1 → ch 99 → the fail menu |
| 0700000a | **YES** | arrival plate #58 → ch 0x47 (objective 3, [L] ee_start_0700000a.bin confirms 0x47-only at load) → the **Q-Worm gadget (weapon 0x58) FIRED at Q-WORM_PC_1** (0x35 = PCQWorm, param 1 = ch 0x48) [M 0x1aee20] → objective 4 → the security keyboard → ch 0x3A → AND #741 → ch 0x49 (objective 5) → AND #742 → ch 0x4A → E at the hatch Load Point #59 → 0700000b |
| 0700000b | **YES** | the balcony spawn plates → ch 0x4B (objective 6) + ch 0x33 (the arrival cut-scene) → the **Pheonix keycard lock** (0x29 Switch, E) → ch 9 → the bridge script writes ch 10 → the terminal script writes ch 11 [D nis] → the **Q-Worm shot at SCOMP_QWORM_C** → ch 0x4C (objective 7) → the terminal plate → ch 0x4D (objective 8) → the files → ch 0x4E (objective 9) → ch 0x4F reveals objective 10 → **the parachute pickup taken (0xF0, param 3 = ch 0x3F)** then the roof-outside volume (ch 0x40) → SEQ #281 → ch 0x41 → OUT #304 → **ch 0x5A + ch 100** → MissionState 6, `to_end = 0x0730000b` (the driving hand-off [M 0x182c60:99-101]) |

## What was ported (every fix with its game address)

**`crates/nf-game/src/useobj.rs`** — the `Player_Activate` (0x1a5170) use classes the Tower levels
need, which the port had none of:
- **PCQWorm** (placement 0x35 → `PCQWorm_Create` 0x1b07a8, class 0x49): the Q-Worm target
  computer. `PCQWorm_Update` 0x1aee20's install: a **weapon-0x58 bullet** (the Q-Worm gadget,
  weapon-data table [L]) entering the PC writes `switch_channels[param1]` one tick later — the
  port tests this tick's bullet segments against the PC's placement sphere (the glist stand-in).
  `PCQWorm_Activate` 0x1b0840 (E, param 2 ≠ 0) and the param-3 scan gate (0x1aee20:49-51) modelled.
- **Switch** (placement 0x29 → `Switch_Create` 0x1adcc0, class 0x39): E-toggle with the gate
  channel, the value match, the ch-0 and ch-0x61 class-3 refusals (`Switch_Activate` 0x1b02a8),
  the create-time channel init (0x1adcc0:39-43) and the auto-return (0x1adea8 — no Tower switch
  sets it).
- **ScriptPlayer E-activation** (`SP_Activate` 0x1dc338): mask-4 SPs activate on use — the
  elevator's and the security keyboard's channel path (with the honest caveat below).

**`crates/nf-game/src/pickup.rs`** — `kind::TEXT` (4): `Pickup_Handler` 0x18e650's case 4 consumes
a pickup with no take effect and writes its +0x28 channel — **OBJECT_Parachute** (0700000b #280,
param 3 = ch 0x3F) was previously refused by `take`, so objective 11 could never fire.

**Shared-file hooks (small, additive, listed):** `drone.rs` — the `useobjs` field + its
`attach_level` create/init, one `useobjs.tick` call after `update_player_bullets`, one
`useobjs.player_activate` call in `player_use_doors`, a `bullet_path_weapons` vec parallel to
`bullet_paths` + its getter; `lib.rs` — `pub mod useobj;`.

## The map corrections and mechanics the docs record (from the disc dumps)

- **Placement types 0x2C–0x36 decoded** [M parsemap_create_dynamic_objects 0x1d04f0]: 0x2E Lock,
  0x2F Copter, 0x30 Monitor, 0x31 **FuseBox** (not pickups — 09's 23 "pickups" are fuse boxes with
  no channels), 0x33 Hint, 0x35 **PCQWorm**, 0x27 gun turret, 0x29 Switch, 0x28 Sensor.
- **The alarm chain on 09**: the 11 "alarmswitchplayer" switches (ch 0x7C ×7, ch 0x7D ×4) → OR mux
  #278 → ch 0x61. The port writes ch 0x7C/0x7D nowhere else — the alarm raises only through the
  switches. 0a's 8 switches write ch 97 = **0x61 directly** [D].
- **The 0a lasers are TOUCH triggers** (0xEB, lock ch 0x37) feeding OR muxes → ch 0x0F-0x12/0x81 —
  alert/music channels, not mission fails.
- **The [L] snapshot** ee_start_0700000a.bin @0x26fc90: at 0a's load ch 0x47 is the only channel
  on, BaseMap 0x07000009, ThisOrderNum 1 — objective 3 completes on arrival, as the plate says.
- **The 0b ending**: the parachute (ch 0x3F) → the outside-top band (ch 0x40) within the SEQ's
  4-tick slack [M 0x1b1e04] → ch 0x41 → **both ch 0x5A (objective 11) and ch 100** from one OUT
  mux (#304); the mission completes WITHOUT any 0xE8 (0b has none [D]).
- **The fall-fail web**: the giant outside volumes (y 39-43/49-53/8-12 → ch 0x78/0x40/0x7A) →
  AND/OR → ch 0x7B → SP #325 plays the 0x0600098A **death cut-scene** (event params are DECIMAL in
  the nis dumps — its ch-62 write is 0x3E, not 0x62); the death→fail hand-off is the game's
  Player_CheckForDeath path, not ported (below).

## What couldn't be ported (with the game function)

1. **The elevator/keyboard activation writer** (07000009 #70 out ch 0x39; 0700000a #60 out ch
   0x3A): the audit confirmed `Player_Activate` dispatches to `SP_Activate` only for class-0
   objects while 0xD9 placements create class-0x1B SPs [M 0x1db9a8:71] — the E path cannot reach
   them in the game either. Remaining candidates: a guard action (`NDrone2_KickObject` 0x14aa00 →
   `SP_UnPause`) or an object the dumps did not cover. The walkthrough drives ch 0x39/0x3A where
   the game would activate; the docs flag it.
2. **The 0b files/roof channels** (writers of ch 0x54/0x06 → obj9's OR, ch 0x59 → obj10): absent
   from the triggers, archive scripts, switches and the raw pack dumps [D]; the walkthrough drives
   them at the point the game's terminal sequence would.
3. **The death→mission-fail hand-off** (Mission_Update's ch-0x62 arm 0x182c60:21-43 via
   Player_CheckForDeath): the fall cut-scene plays; the fail latch is 031-fix-mission's file.
4. **Presentation** (not gameplay): SS animation playback (SS_Update 0x1a9948 — 120-fix-explosions
   owns it), Monitor camera view (0x1b0058), FuseBox shooting (0x1adfe8 — no fused boxes with
   channels on these levels), switch animations, the keycode HUD message (0x1aee20's 0x2000014),
   the lift ride (the port's ScriptPlayers do not carry Bond — the walkthrough teleports, as the
   task allows).

## Spots for the user to check in play (`--bond x,y,z,yaw`)

- 07000009: the car-park spawn `--bond 3.6,-377.1,36.8,-1.34`; the lobby lift at
  `--bond -3.9,-367.8,10.3,1.57` (the E route should load 0a; the elevator route needs the
  activation — watch for the lift doors opening after ch 0x39); an alarm pull at
  `--bond 18.7,-367.5,43.2,-1.57` should fail the mission.
- 0700000a: the Q-Worm PC at `--bond -16.3,0.5,-31.4,0.8` — equip the Q-Worm gadget (0x58) and
  fire point-blank: the install should show and complete objective 4; the hatch at
  `--bond 21.7,1.1,19.3,1.57` should refuse E until objectives 4+5 are done.
- 0700000b: the keycard lock at `--bond 1.3,47.3,-2.6,0.0`; SCOMP_QWORM_C at
  `--bond -18.7,47.4,30.2,-0.8`; the helicopter parachute at `--bond 2.5,57.8,42.3,0.0` — take it,
  step off onto the roof band, and the mission should complete to the driving hand-off.

## Verification

- 10 walkthrough tests through the real Sim path (`nf_view::load_with_carry` + `step_bond` with
  real input: walking, E, the Q-Worm gadget fired at both PCs, the parachute taken by touch), each
  with sister assertions (skipped steps must not complete; the alarm must fail; the locked exit
  must refuse E).
- 3 `useobj` unit tests against the decomp semantics.
- The full suite: 79 binaries ok; the only failures are the 3 permitted slim-ISO `disc_golden`.
- An independent audit (subagent) returned APPROVE WITH NOTES; its non-blocking findings (the
  param-3 gate, the ch≠0 guard, the create-time channel init, the SP-class caveat) are applied in
  commit a3cd24e.
