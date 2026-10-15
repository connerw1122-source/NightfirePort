# fix-death-anim-events — the death script's op-4 events for Bond

Branch `s/fix-death-anim-events`, task 110i. Everything below is [M] (function + address in the
ACTION.ELF decompile), [D] (disc data), [L] (the driver's PCSX2 evidence) or [S].

## What the task asked

Bond's death plays anim script 0x06000151 (`Player_ChangeSubState` 0x190630 case 0xd). Find what
op 4 does in `AnimProcessScriptCmds` 0x1c1b38 and what handles its args for the player, document
it, and fire the death script's four op-4 events on the dead-tick clock, with an NF_ISO test
(kill Bond → each event fires once on the tick the clock passes its frame; nothing while alive).

## What op 4 is (docs/systems/anim-engine.md now carries the full table)

`AnimProcessScriptCmds` 0x1c1b38 case 4 dispatches on the command's kind word
(`[frame, kind, arg?]`):

- **0/4/5** — the footsteps: `ScriptFootstepFoot` toggle/clear/set + `ScriptFootstepType` = 1 for
  scripts 0x6000099/0x60000ae else 2 [M 0x1c1b38:391-403].
- **1** — the player toggle: the SMI's +0x964 weapon-anim byte flips [M :374-379].
- **2** — the script's +0x70 on-event callback [M :382-385] (drones: `DroneAnim_EventFunc`,
  fix-anim-events). `AnimScriptInit` 0x1c1600:51 zeroes +0x70, so the player's death script has
  **no handler** — the driver's "player's anim callback" guess is answered: the death script's
  kinds (5/4/4/6) are handled inside `AnimProcessScriptCmds`/`AnimObjectUpdate` themselves, no
  player callback involved.
- **3** — `Sound_StopAllWithId(arg)` [M :387-389].
- **6** — `Effect_Create(1.0, *(obj+0xdc), obj, 0xc0)` [M :406-410] → `Effect_Body` 0x1cf400:
  the surface's **body** sound (`EffectInfo + material·0x88 + 0x18`) + `Sound_ModAlertness(0.5)`
  [M 0x1de220].
- **7** — the anim-driven bullet + muzzle light + smoke [M :411-457].

The dead player's footstep consumption [M `AnimObjectUpdate` 0x1befd8:159-205]: the death marker
sets `obj+0xff = 0x12` [M 0x190810:72], so the dead Bond is NOT the class-3 arm (whose walk
accumulator plays the hardcoded 0x286) — the script's `ScriptFootstepType` sets the anim object's
datum bit 4 and the bit-4 block calls `Effect_Create(1.0, animobj, Bond, 0x42 | foot)` →
`Effect_Player` 0x1cec48's 0x40 arm: the surface's **footstep** sound (`EffectInfo+0x16`) at the
feet contact with `Sound_SetAlertness(0, h)` [M 0x1de210]. `Effect_Create`'s guard
`material != 0` [M 0x1ce688] reads the **anim object's +0x50** — the HITDATA shadow
`Player_FeetOnPoint` 0x194188:183-191 copies from the feet ray's hit while grounded (the same
shadow `Bullet_init_casing_ex` 0x12f678:186 reads for casings). All of it is skipped while a
script camera is up (`ScriptCam != 0`) [M 0x1befd8:188-189].

## The clock and the events

- 2.0 frames per dead tick = the script's speed 1.0 [M `AnimScriptInit` 0x1c1600] ×
  `FRAME_RATE_MUL` 2.0 [M `GS_SetRefreshRate` 0x17d2b8:15 = 60/FRAME_RATE] — matching the [L]
  PCSX2 trace (+0x90 = 12/32/52 at dead frames 8/18/28, exactly +2.0/tick).
- The death tick itself ticks the just-added script through the alive path's tail
  `AnimObjectUpdate` [M 0x193600:358] — the just-started window (1, 1], empty for this script —
  so the events land on **dead ticks 3 / 5 / 77 / 140** (frames 5 / 8 / 152 / 278; the clock at
  2·tick−1, windows (3,5] / (7,9] / (151,153] / (277,279]). Frame 5 → dead tick 3 is the driver's
  own mapping.
- **The task's "frames 98/116" are hex**: `nf-extract script-info` prints args as hex (the
  sequence arg prints `a3` = 163), so the real frames are 0x98 = **152** and 0x116 = **278**
  (the port's decimal `Debug` of the parsed script shows it; [D] the raw script). The kinds are
  unaffected (5, 4, 4, 6).

## The port (`crates/nf-game/src/death.rs` only — no drone.rs/scene.rs/lib.rs changes)

`Drone::script_commands` (fix-anim-events) is a per-drone danim path; Bond has no port Animator,
so the reuse is the **semantics** (the same (from, to] windows, the same kind effects, the same
SurfaceSounds columns), implemented as `Drones::death_script_events` in death.rs:

- New additive state on `Death` (each on its own line): `script_time: f32` (the script's +0x90),
  `events: Vec<DeathEvent>` (every event fired — frame, kind, dead_tick, sound, foot),
  `footstep_right: bool` (`ScriptFootstepFoot`). `DEATH_SCRIPT = 0x0600_0151`,
  `DEATH_SCRIPT_FRAMES_PER_TICK = 2.0`.
- `death_tick` sets `script_time = 1.0` on the death tick (the empty just-started window) and
  calls `death_script_events` per dead tick **before** the pane/ch62 logic — the game's dead
  branch is `AnimObjectUpdate` → `Player_HandleDeath` [M 0x193600:317-319] and nothing gates the
  anim afterwards (the `GS_PausePlayer` 0x17d280 flag is MP-only and unread there).
- Each tick: `from = script_time`, advance +2.0 capped at the script's 297 frames (end mode 0
  finishes it [M 0x1c1930]), fire the op-4 commands with `from < frame <= to` (the port's
  verified op-1 window semantics; the game's +0x7C fired bits make each fire once — the
  monotonic clock does it structurally).
- Kind 4/5 → the footstep sound (`SurfaceSounds::footstep` at the floor material) at the floor
  point under Bond, played but NOT entered in the hearing list (SetAlertness 0); kind 0 toggles
  the foot. Kind 6 → the body sound (`SurfaceSounds::body`) played AND entered in the hearing
  list at **half weight** (ModAlertness 0.5). Material 0 (no surface) → the event fires with no
  sound, per `Effect_Create`'s guard. The sounds play at the feet contact (the port's floor
  query — the drone-footstep pattern standing in for the game's +0x40 shadow).

## Verification

- **Fail-first shown**: with death.rs stashed, `fix_death_anim_events_live` died at "dead tick 3:
  frame 5's footstep sound 123 queued" (pre-fix the death script's events never fire).
- `crates/nf-view/tests/fix_death_anim_events_live.rs` 2/2 on the merged tree:
  - `death_events_fire_once_at_the_clock_ticks`: kill Bond (`health.health = 0.0` + step_bond),
    145 dead ticks → exactly 4 events, `5@3 kind 5 foot 8`, `8@5 kind 4 foot 4`, `152@77 kind 4
    foot 4`, `278@140 kind 6`, each with the SurfaceSounds id at Bond's floor material. Sister:
    a non-fatal hit fires nothing.
  - `death_event_sounds_queue_on_their_ticks`: per-tick through `step_bond` + the audio flush —
    the footstep sound 123 at the contact on dead ticks 3/5/77, the body sound on 140, quiet on
    1/2/4 and the 6..76 / 78..139 stretches and after 140, nothing while alive. (Position-anchored
    within 1.5 of the contact so a nearby guard's own footstep on the same material doesn't count.)

## Suite

Full-tree (merged with origin/master d38e945d) fast-runner total: see the WORKLOG entry — the
sandbox reaps/limits long runs; the recorded line covers every binary the window fit, with the
FAILED/TIMEOUT lines listed there. On the slim ISO only the 3 `disc_golden` tests (and the
`pss_movies_live` without the retail image) may fail.

## What's left / notes for the next session

- The footprint **decals** (Effect_Player's 0xc arm) are not ported — same standing deviation as
  the drones' footsteps.
- The kind-2 on-event callback for PLAYER scripts stays unported (no player script carries one —
  the +0x70 is zeroed); the drone EventFunc path is fix-anim-events' scope.
- `docs/fields/animobject.md`'s +0x50 row ("movement action state") should gain the HITDATA-shadow
  reading (the contact material + contact point) — not my file to edit, evidence in
  anim-engine.md now.

## Live checks for the main agent

1. **The clock's absolute phase** [the one soft spot]: gdbwatch `sAnimScript+0x90` on the death
   script (0x6000151) tick by tick from the death tick, PCSX2 holding 60 fps vs 30 fps. Expected
   under my model: 1.0 on the death tick, then +2.0 per tick; frame 5's footstep sound on the
   tick +0x90 turns 5.0 (dead tick 3). The [L] trace (12/32/52 at dead frames 8/18/28) matches
   the RATE exactly but sits 2.5 frames off my phase — if the live watch pins a different phase
   (e.g. the first advance lands a tick later), shift `death_script_events`' start tick by the
   difference; the 2.0/tick rate and the window semantics stay.
2. **The body sound id** at Bond's 07000005 courtyard death: expected `EffectInfo[mat]+0x18` —
   the port queues 123 for the footsteps (mat's footstep column); check the body sound's id in
   the same spot (fieldprobe: break `Effect_Body` 0x1cf400, read the id at the 0x40-flag arm).
3. **The dead player's class byte**: gdbwatch `obj+0xff` (Bond) on death → expect 0x12 [M
   0x190810:72], and `animobj+0x60` bit 4 set on the script's footstep ticks.
4. **nf-view render spots** (the events are audio-only; nothing to render): `--bond
   104.954,25.119,108.825,0,0` then kill — the death anim plays (the sequence side is
   death_root's), the footstep/body sounds fire at the feet; no visual change expected from this
   task.
