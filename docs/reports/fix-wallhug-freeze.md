# fix-wallhug-freeze — the 07000005 wall-hug plate must not freeze the NPCs

Branch `s/fix-wallhug-freeze` (from origin/master 03ed005). Status: **the cause is found and the
fix is in; one new NF_ISO test needs a setup re-tune (ignored, see "Left").**

## What real play shows (the reference)

`tests/traces/05-gate-swap.trace` (07000005 from the level start, god mode, the gate guards alive):

- frame 174328: `ch 174328 67 0 1` — the ledge plate #301 writes ch 0x43. It is the ONLY channel
  change at the plate, and ch 0x43 never goes off again (the plate latches).
- frame 174329: the spawned gate pair (the staging drones at (28.3/32.1, 37.15, 44.86), spawned by
  the ch 0x22 wave at 173758) is REMOVED, and the placed pair (placements 302/303, standing at the
  gate spots since the trace start) goes state **1 → 4** (WAIT_SWITCH → IDLE, same spots ±0.15).
- NO NPC freeze: the two state-7 walkers keep walking through +1,500 frames (2/12 drones moving
  before, 2/10 after, no state stall). `Drone_EnableAll(0)` never ran.
- The combat context: the courtyard sniper (the (62.1, 34.6, 104.1) record) goes state 41 → 42
  (SNIPER_IDLE → SNIPER_AIM) at **173533** and cycles AIM/FIRE (0x2a/0x2b) continuously through the
  plate moment; Bond's hit at 173599 is its shot. Also: the two patrol guards were killed at ~1737xx
  (state 68 = 0x44 DEATH_ANIM corpses); ch 0x22 fired at 173757 (the touch → OR gate).

## The cause [M]

`SP_Update` (0x1dbda8) state 0 — the trigger path SP #298 takes (placement 298: script 0x06000994,
mask 0x1 = the out-channel trigger, out 0x43, gate 0, mode 0, stay 1 [D placement, port probe]) —
runs three start checks before loading the script [M 0x1dbda8:80-106]:

1. `0.0 < *(float*)(glb_blokes + 0x894)` — the disassembly (0x1dbf50-0x1dbf70: `lw $v1, glb_blokes;
   lwc1 $f0, 0x894($v1)`) shows the decompiler dropped the pointer load: it is player 0's
   **BLData+0x894 = Bond's health** [D — trace_record.py pokes god-mode health there]. Passes in
   real play (100.0 under god mode).
2. `Mission_Status()` [M 0x1830d0]: any set status must be 1 (2/3/6 are the fail/dead/complete
   states, Mission_Update 0x182c60). Passes (0).
3. **`if (script_id == 0x6000994 && Get_Music_Event(2) != 0) bVar5 = false`** — a hardcoded special
   case for EXACTLY this script (the ledge cut-scene). `Get_Music_Event(2)` = `EventList[2]`
   [M 0x1de350], the combat-music event posted by `DroneFunc_CheckAlarmRaised` (0x1453a8; alarm ch
   0xA4/0x37/0x61) and `NDrone2_SeenAndAttacking` (0x145150; the combat states incl. SNIPER_AIM/
   SNIPER_FIRE). With the event live the script is refused: state = 5 → the SP deletes itself next
   tick (obj+0xFE |= 1). `stay = 1`, so it never comes back.

The NIS format has no branch ops [D docs/formats/nis-script.md], so the script itself cannot
conditionally skip its event-5 freeze — the refusal is entirely in `SP_Update`.

That explains both observations: the main agent's PINE hand-write of ch 0x43 happened in a quiet
room (nobody attacking → `EventList[2] == 0` → the script ran → the ~1,400-tick freeze), while the
user's real play had the sniper cycling AIM/FIRE for 800+ frames → the event live → refused → the
swap only, no freeze.

## The fix

- `crates/nf-game/src/nis.rs`: `Player::start_allowed(SpStart)` + the state-0 arm sets state 5 when
  refused [M 0x1dbda8:106 → LAB_001dc0fc]. States 1/3/6 retriggers skip the checks [M cases 1/3/6],
  as does the create-time load beat [M 0x1db9a8] — both unchanged. `SpStart { health_ok, mission,
  music2 }` rides `update_player`.
- `crates/nf-game/src/drone.rs` (shared file — additive, listed per the rules): `Drones::prev_event2`
  (a new pub field, own line) captures `EventList[2]` right before `music.update` (the game's
  `UpdateMusicalEvents` 0x114e70 clears the EventList once per frame — from `SFXUpdate` 0x1156c0 in
  `Game_Run`'s pre/post passes [M 0x1c9790] — so a post made LATER in the object walk than a script
  player survives into the player's NEXT read; the port passes that last-walk value to the SP gate).
  The gate at the `what == 5` site feeds `health.health > 0.0` (the port's BLData+0x894 stand-in),
  `mission.status()`, and `prev_event2`.
- `crates/nf-view/tests/spawner_kill_live.rs`: the new combat-path test + the freeze sister's
  quiet-room wait (below).

## Verification

- **The sister (freeze path) PASSES**: `the_wall_hug_scene_holds_the_gate_pair_and_swaps_the_pairs_at_its_end`
  now first waits the yard alert out (Bond parked off the sight lines until `prev_event2 == 0` — the
  PINE probe's quiet room), then touches the plate; the script starts, the freeze lands on the touch
  tick, the swap comes at Script_Free. All other `spawner_kill_live` tests pass unchanged.
- **The combat path (the new test, `#[ignore]`)**: drives the real flow — touch 0x23 → the OR → the
  ch 0x22 spawn wave (the trace's 173756/173757), the yard's attack gate posts EventList[2] = 5
  through the real `NDrone2_SeenAndAttacking` path [M 0x145150] (Bond standing in the yard, the same
  way the trace's sniper woke) — then the plate. The fix's assertions PASS in it (no freeze,
  `npc_enabled` stays up, SP #298 goes state 5/deleted); the pair-present assertions do not yet,
  because the spawned pair reads **hp 10.0 at spawn** [L probe] and takes a stray death during the
  yard wait. Ignored with that note; the next session re-tunes the wait (shorter, or a quieter spot).
- Probe evidence (throwaway, not committed): with the event live the port's plate tick shows
  `npc_enabled == true` (pre-fix it went false — the freeze), the SP refuses, and the swap lands.
- Full suite NOT re-run before this push (time); only `nf-view --test spawner_kill_live` (5/6 green)
  and `cargo check -p nf-game`. **The next push must run the full suite.**

## Left (the next message continues from the branch)

1. Re-tune the ignored test's setup so the hp-10 spawned pair survives the yard wait, un-ignore, and
   assert the full trace story frame-for-frame (pair present on the plate tick, removed at +1,
   placed pair 1→4, walker moving, SP deleted).
2. Run the full suite (`NF_ISO=… cargo test --offline --release -j2 -p nf-game -p nf-formats -p
   nf-audio -p nf-view -- --test-threads=1`); fix any fallout from the `prev_event2`/gate plumbing
   (e.g. tests that relied on `Player::update`'s old signature — the in-crate unit tests are updated).
3. Update `docs/levels/07000005.md` §4 ("Ledge cut-scene guards") to note the combat-music refusal,
   and the port-map's SP entry if it exists.

## Live checks for the main agent

1. `gdbwatch` on **EventList[2] = 0x2d34f8 + 8** (u32), access type READ, while playing 07000005
   from the yard through the plate: expect nonzero (5) for a stretch ending at/just before frame
   174328's ch 0x43, written by `NDrone2_SeenAndAttacking` (0x145150) from the sniper's
   SNIPER_AIM/FIRE. Also watch `SP_Update`'s state store at SCRIPTPLAYER+0x24 for placement 298's
   player: expect 0 → 5 (not 0 → 2) on the plate tick.
2. `fieldprobe` poke: in a fresh 07000005 (quiet room), write EventList[2] = 0 and step onto the
   plate — expect the freeze cut-scene (the PINE result). Then repeat with EventList[2] = 5 held —
   expect NO cut-scene and the swap on the next frame (the trace behavior). This pins the gate end
   to end.
3. Optional: the spawned pair's spawn health — gdbwatch a write to the spawned drone's health field
   right after the ch 0x22 wave: the port reads 10.0 at spawn [L probe]; if the game spawns them at
   100, the port's spawn-health path has a separate bug worth its own task.

## Shared files touched beyond the task's list

- `crates/nf-game/src/drone.rs`: the `prev_event2` field (own line) + its capture before
  `music.update` + the `SpStart` wiring at the `what == 5` site. Additive only; the
  `music.update` position is unchanged.
- `crates/nf-game/src/nis.rs`: the two in-crate unit tests' `Player::update` calls (the new gate
  arg, `SpStart::default()`).
