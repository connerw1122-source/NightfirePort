# music-triggers — music trigger placements (0xFB)

Task 750. Branch `s/music-triggers`. The census (task 709) listed the 15 music trigger placements
as unhandled. They are now ported: `MusicTrigger_Create` 0x1b30e0's gates create the trigger at
level load and `MusicTrigger_Update` 0x1b31a0's per-tick logic raises `Music_Event(p2, 2)` into
the music system.

## What the game does [M]

- `MusicTrigger_Create` 0x1b30e0:12-16: **no object** when p2 == 0xff or p0 == 0; otherwise a
  class-0xC control object whose data block carries p0 (u16, the re-arm channel), p1 (u16, the
  flag channel), p2 (u32, the `Music_Event` board slot), p3 (u8, a flag byte). All 15 disc
  placements sit on main maps ([D] probed through `nf-extract placements` with NF_ALL_MAPS); the
  objects are gate-inert — 07000011's record (18, 0, 255, 0) creates nothing.
- `MusicTrigger_Update` 0x1b31a0, in order: while switch channel p1 is on it sets obj+0xfe bit 0;
  while switch channel p0 is on it re-arms the countdown short to **0x1e (30)**; while the
  countdown is nonzero it decrements and calls **`Music_Event(p2, 2)`** [M 0x1b31a0:19-27], with
  p3 ≠ 0 also setting obj+0xfe bit 0. So the event fires every tick the channel holds and keeps
  firing for the remaining 30 ticks after it clears.
- `Music_Event` 0x1de328 writes `EventList[id] = 2` (id < 0x40); `UpdateMusicalEvents` 0x114e70
  consumes the board per frame — slot 2 is the alarm hysteresis (`e[2] == 2 → bonus = 600,
  under_attack = true`, then the same frame's decay), which the per-level updaters turn into the
  combat music demand.

## The port change

- `crates/nf-game/src/worldobj.rs`: `MusicTrigger` (fields on their own lines) created in
  `WorldObjects::create`'s `0xfb` arm with the game's gates; `WorldObjects::tick` runs the
  update in the game's order and now carries `&mut crate::music::Music` — the EventList write
  lands inside the control-object pass, before the same-tick `Music::update` consume
  (drone.rs's `Drones::tick` passes `&mut self.music`). No RNG draws added.
- `crates/nf-game/src/music.rs`: two small accessors for tests (`has_entry`, `alarm_state`) —
  the board and its consume were already ported.
- `crates/nf-formats/src/weapon.rs`: `WeaponData` derives `Default` (the headless test tick).

## Verification (fail-first, real path)

- Unit (`nf-game --lib`, `worldobj::music_trigger_tests`):
  - `create_gates_and_param_map` — the disc's real param shapes [(13,0,2,1) → trigger;
    (18,0,255,0) → none by the 0xff gate; p0 == 0 → none] [M 0x1b30e0:12-16, D placement bytes].
  - `countdown_rearms_fires_and_expires` — re-arm while the channel holds, fire every tick,
    and the event stops exactly when the 0x1e countdown expires after the channel clears
    [M 0x1b31a0].
- Live (`crates/nf-view/tests/music_triggers_live.rs`, 07000001 through `nf_view::load` +
  `Sim::step_bond`): the level's single trigger exists after the load with the disc's params;
  baseline clean; raising channel 13 through the real channel writer pins
  `under_attack` + bonus (599 after the set-then-decay frame [M 0x114e70]); clearing the channel
  lets the countdown expire at the game's tick and bonus decay below the pin (it would stay
  pinned while the event kept firing).
- **Fail-first proven**: with the `0xfb` create arm disabled, all three fail (3/3); restored,
  all three pass.

## Observation

07000001's intro leaves `nis_playing` latched after the settle, and the game's alarm logic runs
only while no NIS camera holds (`!nis_playing` gate [M music.rs's port of 0x114e70]). The test
clears it the game's way — the NIS end's `Music_Event(0xC, script)` board write. If the level's
own NIS end event should have arrived naturally, that is the NIS/script system's gap, not the
trigger's; flagged here for the main agent.

## Census handled sets

`HANDLED_PLACEMENT_TYPES` 56 → 57 ("49 sim + 8 draw-only"); 0xFB (15) moved from the
unhandled list to its own pin; unhandled sum 1537 → 1522. `docs/systems/script-census.md`'s
"does not act on" table and Top-10 item 5 updated. No decompile-backed assertion was weakened.

## What I ran

- `cargo test --offline --release -p nf-game --lib` — 253/253 (final tree, post-merge).
- `cargo test --offline --release -p nf-formats --test script_census` — 6/6.
- `cargo test --offline --release -p nf-view --test music_triggers_live` — 1/1.
- The fail-first stash-style run (3/3 fail → 3/3 pass).
- The full suite (`tools/test/suite.py`, NF_ISO set) on the final tree (origin/master 31dd6161
  merged) — see the suite line below.

## Suite result (final tree)

`tools/test/suite.py` on the final tree (origin/master 31dd6161 merged): **2012 passed, 6 failed,
73 ignored (48 min)**. The 3 `disc_golden` failures are the slim ISO's allowed set. The other 3
are the 0x0C pylon render probes (`fix_0c_pylon_live` ×2, `probe_pylon_render` ×1) — they failed
identically on the cover-nodes task's final tree (a different branch over the same master base,
s/music-triggers touches no render code), so they are inherited from master's visual batch, not
from this task; the main agent may want a master-baseline render look before the next merge.
(`escort_follow_07000001_live` failed once on the cover-nodes tree's run and passed here — flaky,
unrelated.)

## What's left

- Nothing known for the placement family itself. The obj+0xfe bit has no port-side reader (the
  game's readers of control-object +0xfe are outside this task's decompile pass) — the port
  stores it as `playing`.

## Live checks for the main agent

- **EventList write [M 0x1b31a0:24]**: on a fresh 07000001 boot, watch `EventList+0x8` (the
  board slot for id 2, base 0x2f4... — the `Music_Event` writer's `(&EventList)[id]`) while
  switch channel 13 is raised by the level's own scripts: it should read 2 for the 30-tick
  window after the channel clears.
- **Music trigger visibility**: none — the trigger draws nothing (no render checks needed).

## Usage

not visible to me
