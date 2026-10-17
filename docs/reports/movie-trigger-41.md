# movie-trigger-41 — the movie-player trigger (action 0xD) and NIS events 0x08 / 0x0a

Branch `s/movie-trigger-41`, 2026-10-17. Two fully specified gaps from the script census
(docs/systems/script-census.md), ported from the decompile with the real-path tests pinned to
decompile/dic evidence only.

## What was done

### 1. The movie-player trigger (0xF4 → `Trigger_Update`/`Trigger_Activate` action 0xd)

**The create wrapper** [M 0x1b2a08, disasm]: `Trigger_MoviePlayer` backs the 0x84-byte param block
up to `TmpParam` (0x319b88), then writes slot 0 = 0xd (`sw v0,44(s0)` @0x1b2a58), slot 1 = old p0
(@0x1b2a6c), slot 2 = old p2 (@0x1b2a74), slot 9 = old p1 (@0x1b2a7c) and tail-calls
`Trigger_Create` 0x1b1720 — whose shared layout reads out = slot 1 (a full u32 into ext+0x20,
0x1b1720:68), inputs = slots 2..8 (ext+0x26..0x32), mask = slot 9 (default 2), lock = slot 10,
hide = slot 11 [M 0x1b1720:67-88]. This is NOT the mux shift (slot 2 keeps old p2; slot 9 is
rewritten with old p1). Port: `triggers::kind::MOVIE = 0xd`, the `0xf4` dispatch in `build()`, and
the `from_params` arm mapping out = p0 (kept whole in the new `movie_code: u32` — the port's
`out: u8` would truncate a real movie hashcode), mask = p1, list = p2..p8, lock = p9, hide = p10.

**The update arm** [M 0x1b1a78 case 0xd, disasm 0x1b1f24-0x1b1f78]: fires only when the FIRST
input slot (ext+0x26) is non-zero AND that channel is on; then it CLEARS the channel
(`sb zero,0(a0)` @0x1b1f48), latches ext+0x20 into `GameState+0x10` and 0 into `GameState+0x14`
(GameState = 0x2a3768 [L symtab]) and pushes GameFlow states 6 (f12 = 0.0) and 7 — which
`GameFlow_PushState` 0x1c9f38:16-22 turns into `Camera_SetFade(6, 30.0, 1)` + state 8
(0x41f00000 = 30.0f @0x1b1f6c). No spent latch. Port: the `kind::MOVIE` arm in `Trigger::update`
clears the channel and raises `movie_fired`; the control-list walk (drone.rs) drains it into the
new `Drones::movie_request: Option<u32>`.

**The E-key arm** [M 0x1b2318:53-61]: the `mask & 4` gate, then the same GameState+0x10/+0x14
latch and push 6 + push 7 (0x43960000 = 300.0f). No channel writes, no level load, NO spent latch
(re-fires every press). Port: `player_use_triggers` raises `movie_request` for a kind-0xd trigger
whose `mask & 4` passes; `activate_request`'s `_` arm correctly writes nothing for 0xd.

**The flow finding that reshaped the wiring** — the task's "wire it into the port's
`resolve_exit_code` path like LOAD_LEVEL" does not survive the decompile: nothing on the trigger
path starts a movie or loads a level. State 6's body pops the same tick when `BGFMVPlaying == 0`
(`psiMovieFinished` 0x1082e8; `GameFlow_Main` 0x1c98c8 case 6) and nothing starts one; only
`ResetMap_Load` phase 2 (0x1d6360:173) ever reads `GameState+0x10`, and phase 0 overwrites it
first (0x1d6360:103). So the port carries the code in `movie_request` and the view consumes and
drops it (nf-view/src/lib.rs) — no `level_exit`, no load. The observable game effect is the
state-8 fade (a view concern; see Live checks).

**The disc's one record** [D 07000041.bin main-map placement #81, measured bytes]: params
(0, 4, 0) — out = 0, mask = 4, input slot = 0. So its update path is provably inert (the first
guard `lhu v0,38(s0); beq v0,zero` @0x1b1f24-28 fails every frame, even with channel 0 armed),
and only the E-path can fire — pushing movie code 0 (not in `psiStartBackgroundMovie`'s whitelist
[D docs/systems/movies.md], so no movie even if the path could start one).

### 2. NIS event 0x08 — `Drone_CoderCreate` 0x13a108

[M 0x1d94c0 case 8, disasm 0x1d96d4-0x1d96f4]: `Drone_CoderCreate(prev_obj+0xC0,
prev_obj+0x50, p0, p1, p2, p3, 4)` — the position/rotation source is the PREVIOUS stream record's
object (base = SCRIPTINFO + byte[0xAB3]·0x28 + 0x80 = SSTREAM[i−1]; the object only when that
record's kind is 1/2 [M 0x1d94c0:75-96]), NULL skips the call (`beq s5,zero` @0x1d96d4), the 7th
argument is hard-coded 4 (`addiu t2,zero,4` @0x1d96ec → `DCVars+0x5A2`, 0x13a1a0) = start state
IDLE. `Drone_CoderCreate` 0x13a108 builds a zeroed level_tag with slot 0 = p0 (skin), slot 1 = p1
(start script), slot 9 = p3 (setup/weapon selector) [M 0x13a150-0x13a184]; **param 2 is dead** —
the callee never reads its 5th argument (no t0 use in 0x13a108-0x1cc) — so the disc's 0x10 is
discarded. `Drone_Create` → `NDrone2_CreateObj` (class 2, `LList_Add` 0x298E14 [M 0x14d7c8]) →
`NDrone2_PostLoad_Init` [M 0x14d890].

Port: `Request::CoderCreate { obj, skin, script, setup }` from the same previous-record gate; the
drain (`drain_script_requests`) spawns via the port's `Drone::new` with the param map
{0: skin, 1: start script, 9: setup} — the exact slots `NDrone2_DefaultInit` reads back — sets
state IDLE, and registers present/enabled/llist + `PlrStat_LogEnemySpawned` [M 0x14b300:609]. The
yaw comes from the object matrix's forward row [S — the port's script objects keep only the keyed
matrix; a live check could confirm the game's rot→yaw read].

**The game-proven headline**: the disc's only firing (0600005b stream 9, tick 224) is a NO-OP in
the game itself — its previous record is stream 8, which creates no object, so the gate yields
NULL and case 8 skips [M gate + D record bytes]. The port mirrors that exactly; the spawn path is
exercised through a synthetic script whose stream 0 creates the object.

### 3. NIS event 0x0a — `Break_DoBreak` 0x12c168

[M 0x1d94c0 case 0xa, disasm 0x1d96bc-0x1d96c8]: `Break_DoBreak(prev_obj+0xC0)`, the same NULL
skip (the event passes no params — `Event(0x0a, [])` [D]). `Break_DoBreak` 0x12c168 scans the
control list for the first class-0x20 object whose world sphere (obj+0x80/+0x8C) contains the
position; `Break_Kill` 0x12b970 deletes it (obj+0xFE |= 1), plays 0x2e (glass) / 0x5f2-0x5f3 on
`Rand_Rand(100) < 41` (wood) at the object, then shatter polys or `Debris_Create_Loop` 0x1ed548.

Port: `Request::DoBreak { at }`; the drain resolves the position via `script_obj_pos` and scans
the port's script-entity store for kind-0x20 entities (the Entity command's obj_type is the
game's class byte). **Documented stand-in [S]**: the port does not simulate placed 0x20 glass
panels (`Break_Create`), and script entities carry no bounding sphere — the match radius is 1.0
around the entity's matrix translation. The disc's firing (position = stream 7's anim object)
finds no script-side 0x20 either way; the debris/shatter are draw-side and not modelled.

### 4. Census sets

`crates/nf-formats/tests/script_census.rs`: `HANDLED_EVENT_KINDS` 15 → 17, `HANDLED_TRIGGER_ACTIONS`
7 → 8, `HANDLED_PLACEMENT_TYPES` 61 → 62, with the unhandled lists and comments updated;
`docs/systems/script-census.md` tables + the Top-10 entry marked done with the measured record
params.

## How it was verified

- **Unit (nf-game, all pass)**: `movie_trigger_fires_on_armed_input_and_the_discs_record_cannot`
  (fire clears the channel + carries the full u32 code + no spent latch; sisters: input slot 0 →
  the disc's record shape never fires even with channel 0 armed; channel off → nothing),
  `movie_trigger_activate_gates_on_mask_bit4` (no channel writes, no load; sister: mask 8 fails
  the gate).
- **Live (nf-view, all pass, NF_ISO)**: `movie_trigger_live.rs` — the 07000041 record parses with
  the wrapper's mapping and the measured volume box [D], never fires from update with channel 0
  armed, and the E-path at its volume raises `movie_request = Some(0)` twice (no latch) with
  `level_exit` staying None. `nis_coder_break_live.rs` — the archive bytes pin both firings [D];
  the real script run spawns nothing (the game's own NULL gate) and breaks nothing; the spawn
  path (skin/start-script/state IDLE/registration) and the kill path (glass dies, far glass
  survives) run through the real script player with synthetic scripts built as the parsed form.
- **Census (nf-formats, all 6 pass)**.
- **What I ran**: `cargo test --offline --release -p nf-game --lib triggers` (12 pass) and
  `--lib nis` (9 pass); `-p nf-view --test movie_trigger_live` (2 pass) and
  `--test nis_coder_break_live` (3 pass); `-p nf-formats --test script_census` (6 pass). The
  full four-crate suite (`cargo test --offline --release -j2 -p nf-game -p nf-formats -p
  nf-audio -p nf-view -- --test-threads=1`) was started on the final tree: through nf-formats
  every binary is green except `disc_golden`'s 3 expected slim-ISO failures (the task's allowed
  set); the run was still working through the nf-game/nf-view live binaries when the sandbox
  deadline (07:00 UTC) cut it off — the individual runs above are the pushed verification.

## What's left / notes for the next task

- The state-8 fade (Camera_SetFade mode 6, 30.0f/300.0f, viewer 1) is view-side and not drawn —
  the task boundary puts drawing with the main agent.
- The placed-0x20 breakables (glass panels; 0x20 count on 07000001 alone is large) remain
  unmodelled — the event-0xa scan documents the stand-in.
- The movie trigger's out slot CAN carry a real 0x0720xxxx hashcode in principle (a different
  record or live poking); the port's `movie_request` would deliver it unchanged — still no level
  load, matching the game.

## Live checks for the main agent

1. `gdbwatch` (or fieldprobe) `GameState+0x10` (0x2A3778) and `GameState+0x14` (0x2A377C) on a
   fresh 07000041 boot, walk Bond into the 0xF4 volume at (16.13, 0, −0.41) and press E: expect
   +0x10 = 0, +0x14 = 0, and a ~5 s screen fade (300 ticks) — confirming the E-path's
   0x1b2318:53-61 behavior end to end.
2. Same spot, do NOT press E but arm `switch_channels[0]` (poke 0x26FC90 = 1): expect NO state
   push — the slot-0 guard (0x1b1f24) keeps the update path inert.
3. If convenient, poke a real movie code (e.g. 0x0720004a) into the trigger's ext+0x20 before
   pressing E and confirm state 6 still pops immediately with no movie (nothing starts one) —
   the port's consume-and-drop then matches the game byte for byte.

## Usage

not visible to me (no token/credit/context figures are shown in this environment).
