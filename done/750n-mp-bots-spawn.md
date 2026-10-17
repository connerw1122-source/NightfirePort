**Reproduce first, on the newest origin/master, through the REAL path.** Earlier fixes for some of these passed
their own headless tests and the user still hit the bug in the game. Before changing code, write a live test that
drives the same path the player takes (the level loaded through `nf_view::load`, Bond moved by `step_bond` inputs —
walking, the use key, the ladder input — the real scripts and triggers running, no teleports past the problem
spot unless the report says so) and FAILS on current master the way the user describes. Put the failing output in
the report. Only then fix it, and the same test must pass. If you cannot reproduce it, say exactly what you tried
and where the path diverges from the report; do not mark it done on a guess.

## Your task: `mp-bots-spawn`, multiplayer bots do not spawn

The user, 2026-10-16 evening: "bots wont spawn" (earlier: "There's no bots in this lobby"). Through the real
front-end path (Multiplayer → set AI bots on the options/bots page → start an arena match), find where the bot
count stops reaching the match: the settings (MpSettings player/bot counts), the arena load (`load_mp_carry`), the
bot spawn in the MP update (the mp-bots feature — the game binary is built WITHOUT it per the 110s merge note; check
whether the window build needs it enabled and why it was left off), and the spawn points. Fix it to the game's flow
[M] and pin: N bots requested on the page → N bots alive in the arena after the start, moving and fighting.

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Tests pin only proven behaviour.** Every assertion about what the game does cites its evidence: [M] a decompile
address that actually says it, [D] disc bytes, or [L] a live PCSX2 measurement from a FRESH boot (a save-state resume
skips the boot path). A guess, a port choice or an [S] item is never a hard assertion: mark it `// [S] unverified` and
keep it out of pass/fail. If an existing test contradicts the decompile, the TEST is wrong: fix it, with the address, and
say so in the report. Never weaken a decompile-backed test to make code pass.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/mp-bots-spawn`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox6 at 2026-10-17T03:51Z

---
## Report (done, 2026-10-17T05:09Z)

# mp-bots-spawn — the bots' starved loop is fed; the play pin is one assert short

**Status: IN PROGRESS (branch `s/mp-bots-spawn`, head = the feed-the-starved-loop commit).**
The window's first hour went to setup (the sandbox was reset: clone, `work/` + the 1 GB ISO,
rustup, the ALSA prefix, the queue worktree) and the decompile reading; the second hour to the
fixes. The bots now walk pickup routes, acquire opponents, aim, strafe and enter the fire chain —
the 3600-tick play test still fails on its movement assert (3.2 walked < 5.0, bot 1; combat pins
them near the spawn). Everything below cites its evidence.

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

## Verified

- The RED above on pristine master; after the fixes the same test:
  `bot 0: walked 177.9 ... opp Some(0) state 0xd2` mid-run (the bots fetch pickups, acquire the
  human and each other, strafe in combat) — final table `bot 1: walked 3.2` still fails the
  > 5.0 movement assert. The other two asserts in front of it (the bots exist, the movement)
  pass for bot 0.
- `mp_bots_live::bots_disabled_leaves_only_the_human` (the sister) green.
- `cargo build --release -p nf-game -p nf-view` clean.
- **Not run (the window ended first): the full fast suite, the neighbour MP binaries
  (mp_menu_flow / mp_host_match / net_match), nf-game's lib tests.** The next session must run
  `tools/test/suite.py` in its first minutes; `create_bots` grew a `&WeaponData` argument (the
  only caller is the arena load in nf-view/lib.rs) and `handle_firing`'s RNG order changed (the
  DoFiring re-arm rolls where the old code did not) — any lockstep golden that pins bot RNG
  draws must re-record.

## What's left (the next session's first hour)

1. **The movement assert** — combat pins the bots near the spawn (they acquire at spawn LOS and
   the 0xD2/strafe loop holds them). The game's combat bots break off (the 5 s sight latch
   expires, the goal machine re-runs); the port's `unseen_ticks` latch may never expire because
   the straight-line walker pins them inside a clear-LOS stand-off. Instrument with NF_BOTLOG=1,
   then either give 0xD2's msg-3 roll its game weight (the `rand(3)` fall-through to
   chooseCombatMove) or verify the heartbeat's 5 s latch path.
2. **Deaths/score** — the shot resolution (body-sphere sweep, radius 0.6 [S]) needs the bots to
   actually fire; the clip/weapon arming now allows it, but no shot landed before the window
   closed (the aim cone + burst deadline need a trace). The friendly-fire gate on bot-vs-bot
   damage is not applied in the driver's sweep (Quick Game bots are all Phoenix — the game's own
   team words make them mutual enemies only via FFA rules [M §5]; decide and cite).
3. **The human damaging bots** — the player-bullet path still hits drones only (the 704 report's
   left item 3).
4. **The render** — no meshes (the main agent's render checks apply once skins resolve; the
   standing "No drawing" rule keeps this off this task's plate).
5. **The full fast suite** on the final tree, then the finish steps.

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
