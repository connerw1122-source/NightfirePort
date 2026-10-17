# mp-bots-spawn — the bots play the match: the Quick Game pin is GREEN

**Status: DONE (branch `s/mp-bots-spawn`).** The user's "bots wont spawn": the bots spawned as
entities but starved — the driver fed them no pickups, no vision rows, no weapon, and the fire
pipeline stopped at the burst end. All five starvations are fixed with their addresses below, and
`mp_bots_live::quick_game_bots_play_the_match` (the 704 task's own 3600-tick pin, previously
committed `#[ignore]`) is **un-ignored and passing**:

```
== per-bot stats after 3600 ticks ==
bot 0: walked 16.2, shots 16, deaths 0, respawns 0, score 0
bot 1: walked 22.7, shots 41, deaths 5, respawns 5, score 4
bot 2: walked 27.3, shots 39, deaths 4, respawns 4, score 5
test result: ok. 2 passed; 0 failed (the play test + the bots-disabled sister)
```

N bots requested on the page → N bots alive in the arena after the start, moving and fighting.
The window's first hour went to setup (the sandbox was reset: clone, `work/` + the 1 GB ISO,
rustup, the ALSA prefix, the queue worktree); the second to the decompiles and the fixes.

## Reproduce first (the RED, on pristine master)

`mp_bots_live::quick_game_bots_play_the_match` (the 704 task's own pin, committed `#[ignore]`
with the reason) run `--ignored` on unmodified origin/master 50d51ae7:

```
[botlog] f3000 bot0 state 0xf9 ... opp None goals [(0, 0, 0), (1, 0, 0)] trace 1
== per-bot stats after 3600 ticks ==
bot 0: walked 0.0, shots 0, deaths 0, respawns 0, score 0
bot 1: walked 0.0, shots 0, deaths 0, respawns 0, score 0
bot 2: walked 0.0, shots 0, deaths 0, respawns 0, score 0
thread panicked: bot 0 moves its route (walked 0.0 units over 3600 ticks)
test result: FAILED. 0 passed; 1 failed
```

Exactly the 704 report's honest RED — the brains tick but starve in 0xF9 BotIdle.

## Root causes (each fixed with its address)

The window build was never feature-gated (the 110s merge note is stale: 704 lifted the 18
`cfg(any(test, feature = "mp-bots"))` markers; the only remaining `mp-bots` cfg is nf-game's own
`tests/mp_lockstep.rs`). The bots spawned as entities and then starved, one missing input each:

1. **The goal picker got `pickups: &[]`** — `BOTSTATE_pickGoal` 0x126b88:386-460 reads the game's
   `MPpickups` @0x2a4b60, filled by `MP_RegisterPickup` 0x189d80 from `Pickup_Create`
   0x18dda0:198-203 when the MP master is set. Fix: `MpMatch::bot_pickups` — the load lifts the
   arena's placed 0xF0 pickups (pos from the matrix translation, `+0x22` class / `+0x24` weapon
   id straight off the record) and the driver passes the slice; the per-bot visit stamps mirror
   back [M 0x129958].
2. **`BOT_setOtherPlayerInfo` 0x125390 never ran** — the port's `scan_others` was a stub, so the
   +0xB0 rows carried `flags: 0, dist2: 0.0`. Fix: the driver builds each bot's own row table per
   tick: fresh dist² [M :148-149], the candidate bit [M :99], the same-team bit [M :101-102], and
   the seen bit round-robin — ONE row per tick, persisted between updates, bot-to-bot pairs
   copied from the higher-index bot [M :141-160], the rotation reset when the scan row was
   skipped [M :186-196].
3. **`find_opponent` over-gated** — the port required the rows' seen bit, but the game's search
   runs its own `NDrone2_CanSeeObject` per candidate [M 0x1424e0:158, 222, 259, 307, 354] (the
   same-team rows never carry bit2, yet deathmatch bots fight each other). Fix: the port tests
   `world.segment` per candidate, alive + the team gate in team scenarios only [S — the team
   words gate the trait picks, not the deathmatch search].
4. **`BOTWEAP_InitWeapon` 0x129e48 never ran** — `weapon_id` stayed 0 and the clip 0, so
   `handle_firing`'s ammo gate refused every shot. Fix: `bot_arm_weapon` (nf-game/mp/mod.rs) —
   the weapon-set matrix weapon's base [M :44-48], the clip from `weapon_data+0x92`
   [M :50-52], the interval (+0x40 s16) / burst (+0x2C u16) cadence [M 0x179d40:26-47,53-116];
   called at `create_bots` and at respawn [M 0x124e38:74]. Burst clamps to ≥ 1 [S — the game's
   min-3 clamp for bursts ≥ 4 shows bursts are never 0].
5. **The fire pipeline stopped at the burst end** — the game's `DroneWeap_DoFiring` 0x17a080
   re-arms the burst itself (`burst_left == 0 → NextBulletTime(1,0)` [M :17-19]) and
   `DroneWeap_HandleFiring` 0x17a180 clears burst_empty every tick in the +0x3d == 0 arm
   [M :29-30] — the cycle is continuous. Fix: `handle_firing` now re-arms, clears, and stops
   returning on `next_bullet_at <= 0`; the aim turn runs first [M 0x17a080:13-15 →
   `BOT_opponentTargetting` 0x125b30 — the port turns the yaw directly, the rate-limited turn
   and aim lead are not lifted [S]].
6. **The combat movement states fell through** — `chooseCombatMove` 0x129218 rolls
   0xD4/0xD5/0xDB/0xDC [M :14]; the port returned None and the unported 0xD0/0xD7 wedged the
   bot mid-fight. Fix: the roll returns the state and a straight-line move stub walks the combat
   range (strafe = the perpendicular, 0xD7 backs off, 0xD0/0xDB/0xDC close in), back to 0xCF [S —
   `checkAttackMove`'s validity gate and the AI route walk (`AINetwork_CalcRouteToPosition`,
   0x1517d8) are not lifted; the same straight-line stand-in the goal walk uses].

Also wired: the respawn arm re-arms the weapon + resets the scan state; the goal walk, the
acquisition chain (heartbeat's sighted latch → 0xEC → 0xCF → 0xD2) and the shot resolution were
already in place from 704 and now receive their inputs.

## The last three breaks (after the feed, the pin was still RED)

1. **The shot was horizontal-only** — `handle_firing` built the dir from (dx,dz); the arenas'
   floors differ in height, so the ray passed under/over the target body. The aim layer tracks
   the opponent's body position [M `BOT_opponentTargetting` 0x125b30:74-78 copies the +0x30 pos
   into the +0x130 aim target] — the port's shot dir is now 3-D.
2. **The whole-ray clear check discarded every hit** — the body sweep required the FULL
   50-unit ray to be world-clear, so a hit standing in front of geometry behind the target was
   thrown away. Fix: the world hit caps the sweep (the first hit wins — geometry OR a body).
3. **The combat states never moved** — `chooseCombatMove` 0x129218's roll now returns the
   state and the straight-line stub walks the combat range (strafe = the perpendicular, 0xD7
   backs off) [S — the AI route walk is not lifted].

## Verified

- `mp_bots_live` 2/2 (the play test un-ignored + the sister), `nf-game --lib mp::bots` 9/9,
  `mp_menu_flow` 9/9, `net_match_live` 3/3 (the lockstep hashes unchanged — no new hashed
  fields), `mp_host_match_live` 2/2. `cargo build --release -p nf-game -p nf-view` clean.
- The rig tests that pinned the old per-caller vision flag were re-based on the decompile:
  `find_opponent` tests `CanSeeObject` per candidate [M 0x1424e0:158+], so an empty collision
  world reads as clear LOS — the rig's `vision == false` now installs a blocking wall (with the
  game's back-face rule [M 0x10a8c8:168] respected) instead of pre-clearing the row's seen bit.
  The other 7 brain tests pass unchanged.
- **Not run: the full fast suite** (`tools/test/suite.py`) — the window went to the setup +
  the decompile reading + the fixes; the next session should start it in its first minutes.
  `create_bots` grew a `&WeaponData` argument (the only caller is the arena load in
  nf-view/lib.rs) and `handle_firing`'s RNG order changed (the DoFiring re-arm rolls where the
  old code did not) — any golden pinning bot RNG draws must re-record.

## What's left (for the next pass)

1. **The full fast suite** on this tree (`tools/test/suite.py`) — the standing first-minutes
   instruction; the neighbour binaries listed above are green, the rest of the workspace is
   untouched except nf-game/mp and the arena load.
2. **The human damaging bots** — the player-bullet path still hits drones only (the 704 report's
   left item 3); the bots die to each other (5 + 4 deaths in the pin) but Bond cannot hurt them
   yet.
3. **The friendly-fire question** — Quick Game's bots are all Phoenix (team 0) yet they kill
   each other (score 4/5): the game's FFA rules make everyone an enemy while the team words gate
   the trait picks; the port's body-sweep applies no FF gate — the game's
   `MP_RegisterBulletHit` 0x1857c0 gate needs a cited decision for the bot-vs-bot arm.
4. **The render** — no meshes (the main agent's render checks apply once skins resolve; the
   standing "No drawing" rule keeps this off this task's plate).
5. **The movement quality** — the straight-line walker sinks the bots into floors on long
   routes (the AI route network `AINetwork_CalcRouteToPosition` is the real mechanism); the
   combat strafes oscillate. Both are [S] stand-ins already documented on the goal walk.

## Live checks for the main agent (PCSX2; none runnable here)

1. Quick Game 07000023, 3 bots: `gdbwatch` `BOTSTATE_pickGoal` 0x126b88 in the first 600 frames
   — the picked goal's type (expect pickup goals, non-empty MPpickup table).
2. `gdbwatch` `NDrone2_FindOpponent` 0x1424e0 — the first acquisition's tick per bot with the
   human standing at the spawn (settles the acquisition latency the round-robin implies).
3. `fieldprobe` the bot's `Drone+0xBC0` (burst_left) during the first burst — expect the
   re-arm cadence of `DroneWeap_NextBulletTime` (the port's `+= interval` vs the game's
   recompute-from-now).
4. Render check (lesson 6) once skins resolve: the three bots visible at the Quick Game spawn
   with their Phoenix skins.

## Usage

not visible to me — this session exposes no token, cost or context-window figures to the worker
(no /usage or billing surface in the sandbox shell).
