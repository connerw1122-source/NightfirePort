# laser-scripts — the level laser-grid beams and their scripts

Task 750i, branch `s/laser-scripts`, 2026-10-17. The user: "lasers arent accurate to the port
look closely at their scripts".

## What was wrong

The laser-grid / tripwire beams were missing entirely — not mis-timed. Every grid level drives
its beams from a **switch** (placement type 0x29) whose **param 1 is a NIS script hash**:
`Switch_Create` 0x1adcc0:44-45 ends with `SP_Create(pos, rot, param 1, 0, 8, 0, obj, 0)` [M] —
an embedded ScriptPlayer playing that script **looping** (mode 8, `SP_LoadScript` 0x1dbc08:46-49
`Script_Play(loop=1)`, state 6). The port's `SwitchObj` (crates/nf-game/src/useobj.rs) read
params 0/2/3/4/5 but never param 1, so the embedded player never existed and neither did the
beams. The beams themselves are the script's entity streams: `Script_CreateEntity` 0x1d8968
creates a draw-only object with the laser art's glist and RE-POINTS a same-class stream object's
gfx on each re-create (0x1d8968:92-101); the art glists are modelled in world space (all three
09 art objects sit on origin 0x2031 placements in map 0100009f [D]), so an identity-matrix
entity draws the whole grid.

## The shows (all decoded from the disc; full mechanics in docs/systems/laser-scripts.md)

| level | driver | script | cycle |
|---|---|---|---|
| 07000009 | switch #71 `SCRIPTPLAYER_Lasers` (ch 0x37, p1 = 0x06000667) | 985 t, holds at t2 | ON 0x020005ed at t0; AMBER 0x02000a29 @31; OFF 0x020005ec @496; AMBER @527; deleted @985; loop |
| 0700000a | switch #643 `SCRIPTPLAYER_KeyboardLasers` (ch 0x3B, inhibit ch 0x3C, p1 = 0x06000664) | 985 t, holds at t2 | ON 0x020005e0 @0; AMBER 0x02000a2d @31; OFF 0x020005df @496; AMBER @527; deleted @985; loop |

The hold: the script's event kind 3 with p0 = 0 parks `SCRIPTINFO+0x9c = −1` [M
`Script_EventHandler` 0x1d94c0; `Script_Update` 0x1da998:17-18 runs only at delay == 0 and never
decrements a negative delay], so a fresh level shows the static ON art — exactly what the game
does. The switch's channel-change arm unpauses the player (`Switch_Update` 0x1adea8:19-24 →
`SP_UnPause` 0x1dca58 zeroes the delay), replaying one full cycle per toggle;
`Switch_Activate` 0x1b02a8:23 gates the E-toggle on `SP_GetState() == 0` (allowed exactly while
the show is held). The 0a/0b `laser`/`laserbeam` 0xEB touch triggers are alarm volumes, not
beams (their glist flags 0x50 carry the not-drawn bit [M docs/fields/glist.md]) — their lock
channel 0x37 is what the 09 switch toggles. `nf-extract nis` hashes 0x06000052/0x06000027 are
the explosion scripts, not lasers — already ported as the blast scripts.

## The fix (every change [M]-cited in the code and docs/systems/laser-scripts.md)

1. `Drones::attach_level` (crates/nf-game/src/drone.rs): mounts one `nis::Player` per main-map
   0x29 placement with a hash (≥ 0x06000000) param 1 — scripts [p1, 0], modes [8, 0], mask 1
   (the port's prime() skips mask-0 SPs; `SP_Create`'s own mask word is 0 — the same load-beat
   stand-in the fusebox player uses), out/gate 0, scale 1.
2. `SwitchObj` (crates/nf-game/src/useobj.rs): carries the ext+0x0C player handle (`sp`) + its
   placement index; the channel-mirror arm, the watchdog arm and the E-toggle unpause the linked
   player (delay = 0 + the activate latch, `SP_UnPause` 0x1dca58:10-14), and the E-toggle is
   gated on the script-idle rule [M 0x1b02a8:23].
3. `nf-view`'s script-entity attachment sweep (crates/nf-view/src/scene.rs) includes the
   switches' scripts, so the beam art gets attachment slots and renders through
   `Sim::script_entity` like the truck's entities.
4. `Scripts::entities()` (crates/nf-game/src/nis.rs) — the per-beam state getter for the draw
   code: `(script hash, stream, art hash, world matrix, alpha)` per live entity, i.e. which
   variant (ON/OFF/AMBER) is up, where, at what alpha, under which stream.

## Verification

- **Fail-first through the real path** (`crates/nf-view/tests/laser_scripts_live.rs`): loads
  07000009 / 0700000a via `nf_view::load`, walks Bond from the real spawn with `step_bond`
  inputs, and asserts the embedded player, the alive art hash and the t2 hold. On the pre-fix
  tree it failed exactly as the user's symptom implies:
  `panicked … no mode-8 embedded ScriptPlayer for 0x06000667 — the laser show never mounts`
  and `no embedded ScriptPlayer for 0x06000664 — the keyboard lasers never mount`
  (test result: FAILED. 0 passed; 2 failed). After the fix: 2 passed.
- Sister assertions that can fail: the alive-laser set after 800 ticks is EXACTLY
  `[0x020005ed]` (a broken hold would add the AMBER/OFF art), the delay < 0 hold pins, the
  switch↔player link pins, and the 0a level re-pins the same mechanics with its own script/art.
- Touched crates: **nf-game lib 251/251**, the seven nf-view script/fusebox/truck/cutscene
  suites **21/21** (fix_script_objects_live 3, fix_script_commands_live 4, fix_truck_live 2,
  gate_guard_truck_live 2, fix_fusebox_cut_live 4, nis_event9_live 2, fix_cutscenes_live 4),
  **nf-formats script_census 6/6** — no handled-set constant moved (0x29 was already a handled
  placement type and no new NIS op/event kind is dispatched; the census test passes unchanged).
- **Fast runner** (`tools/test/suite.py`) on this branch BEFORE the laser commit: build ok (478
  binaries, 675 s), then `2009 passed, 6 failed, 73 ignored (51 min)`. The 6: the 3 tolerated
  slim-ISO `disc_golden` failures plus 3 pylon render probes (`probe_pylon_render`,
  `fix_0c_pylon_live` ×2) — GPU-less-sandbox render tests that fail the same way on master.
  The final merged-with-origin/master re-run did NOT fit this sandbox window (a full run is
  ~51 min; the window closed at 04:46 UTC with the merge itself untested) — the next message
  should run it before merging.

## Per-beam change table (for the main agent's PCSX2 render checks)

| beam set | before | after |
|---|---|---|
| 07000009 lobby grid (`SCRIPT_Lasers_*`, world-space art) | nothing drawn — the art never spawned | ON art alive from the load beat, static; the draw path is the new switch-script attachments (`Sim::script_entity(src=(0x06000667,0))` → art 0x020005ed), alpha 1 |
| 0700000a keyboard lasers (`SCRIPT_KeyboardLasers_*`) | nothing drawn | ON art alive from the load beat, static (src=(0x06000664,0) → 0x020005e0) |
| both, after a switch toggle (channel change) | nothing | one full cycle: ON → +AMBER @+29 t → OFF @+494 → AMBER @+525 → empty @+983 → ON; sound 0x4a1/0x4a0 / 0x4a1 per the script's sound stream |

`--bond` observation spots for the render check: on 07000009 the lobby grid around the main
lift (x ≈ −4.5…−2, z ≈ 10…15, y ≈ −368.5 — the 0EB-free lobby floor; the beams span the lobby's
corridor openings); on 0700000a the keyboard area at the `SCRIPTPLAYER_KeyboardLasers` show's
world-space glist (the office floor around the security keyboard). The art is world-space, so
any lobby/keyboard vantage shows it.

## Deviations / what's left

- The port unpauses the show on the same tick as the channel change; the game's object-walk
  order makes it one tick later there (invisible at 983 ticks). [*deviation*, noted in code]
- The origin-placed laser switch is as un-E-usable here as in the game (both read the
  placement/origin transform [M `Switch_GetNearest` 0x1b0398]); ch 0x37 changes only through
  other writers in play.
- The final merged-tree suite re-run (see above) and a PCSX2 render comparison of the two grids
  are the outstanding items; the Live checks are listed in docs/systems/laser-scripts.md.

## Live checks for the main agent

1. 07000009 fresh boot: confirm the lobby laser grid draws from level start (the ON art
   0x020005ed alive, `SCRIPTINFO+0x9c = −1` held). Address: the switch's SP SCRIPTINFO delay —
   watch `switch_channels[0x37]` @0x26FC90+0x37 (read, expect 0) and the beam art visible.
2. Using the lobby-area laser switch (the security-room panel if the origin switch is
   unreachable in-game too): `switch_channels[0x37]` should flip 0→1 and the beams should stay
   drawn while one full ON→AMBER→OFF→AMBER cycle plays (~16 s), then freeze back on ON.
3. 0700000a fresh boot (ch 0x37 carried on from 09 or not): the office-web touch triggers
   (`laser`/`laserbeam` 0xEB) should be inert exactly while `switch_channels[0x37]` == 1 [M
   Trigger_Calc's lock gate] — walk into a beam with the channel 0 vs 1 and compare.

## Usage

not visible to me (no token/credit/context-window figures are exposed to this session).
