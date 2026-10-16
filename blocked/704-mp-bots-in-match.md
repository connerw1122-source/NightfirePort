## Your task: `mp-bots-in-match` — bots actually play a local MP match

A local MP Quick Game now loads its arena with 3 bots (the menu repair, 2026-10-15). Prove the bots play.
Headless, from the menus' `mp_match_settings` through `load_mp_carry`, tick 3600 frames of the arena and check each
bot:
- it moves its route (positions change);
- it acquires the human and other bots and fires;
- it takes damage and dies;
- it respawns per the game's rules;
- the scores go up.

Use the MP core (`nf_game::mp`, the bots module) and the game's bot AI [M BOT_update and friends]. Fix anything
inert: bots that never spawn, never move, or never fire. Table the per-bot stats after 3600 ticks. Sister: with bots
disabled, only the human is in the match.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/mp-bots-in-match`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox7 at 2026-10-16T16:31Z

---
## Report (blocked, 2026-10-16T17:37Z)


---
## Report from the worker (blocked, 2026-10-16T17:38Z)

# mp-bots-in-match — the Quick Game's bots: wired, not yet playing

**Status: BLOCKED (the window, not the design).** The sandbox was reset at 17:00 UTC — the full
setup (clone, `work/` + the 1 GB ISO download+extract, cargo, the ALSA prefix) consumed the first
~30 minutes of the 2-hour window. The bot entities and the per-frame driver are wired and pushed;
the brains tick but stay in BotIdle for one missing input each. What is left is enumerated below
and is continue-able directly from the branch.

## What landed (branch `s/mp-bots-in-match`, from origin/master 3bb7be9a)

1. **The bots' half of `MP_Start`** — `MpMatch::create_bots` (nf-game/mp/mod.rs): the count
   clamp ≤ 4 [M 0x183a50:138-140], per bot the slot `4 + i`, the record's +0x0F team byte into
   `MPSettings + slot·0x30 + 0x20` [M 0x183a50:143-145], a `MP_GetSpawnPoint` pick and
   `BOT_init`'s port (`BotBrain::bot_init` — the 0xC3 chain into state 0xF9, the stats copy,
   the goal-slot latches [M 0x1249e8]) [M 0x183a50:147-160]. Called from the arena load right
   after the humans' creates (the game's order: humans first, then bots — the spawn picks' RNG
   draw order follows).
2. **The per-frame driver** — `Sim::step_mp_bots` (nf-view/scene.rs), called from `step_bond`
   next to the `MP_Update` block: the 8-slot `others` table per `BOT_setOtherPlayerInfo`
   (0x125390 [M]: the slot's obj pos, the alive gate, the team word, the health/score for the
   trait picks), then `BotBrain::step` (the ported `BOT_update` 0x126448 family: the perception
   pass, the 0xC5 heartbeat, the msg-3 state dispatch, `DroneWeap_HandleFiring`) per bot per
   tick on the shared bit-exact RNG, then the shot resolution (the raw `weapon_data+0x0C`
   damage word — the same word the drone fire path reads), the frag credit into
   `MPGame+slot·0x30+0x18`/`+0x04`/`+0x08` [M 0x188f70] and the `BOT_respawn` re-init
   (0x124e38 [M]) at a re-picked spawn point after 180 ticks [S — the death handler's delay].
3. **The cfg gates lifted** — bots.rs's 18 `#[cfg(any(test, feature = "mp-bots"))]` markers
   removed: the phase-4 machinery is now the sim's live code, not test-only.
4. **The test** — `crates/nf-view/tests/mp_bots_live.rs`: the 3600-tick Quick Game proof (the
   per-bot stats table) and the bots-disabled sister.

## Verified

- The Quick Game load creates exactly 3 bots (Drake/Kiko/Rook's records, slots 4/5/6) at the
  spawn points — `mp_bot_count() == 3` [M 0x183a50:134-160].
- `bots_disabled_leaves_only_the_human` (the sister) **green**: bot_count 0 → no bot entities,
  still none after 600 ticks, the human unhurt.
- The wiring breaks nothing: `net_match_live::two_sims_1000_ticks_same_state` 1/1 (the lockstep
  hash with the new driver in the tick), `mp_menu_flow` 9/9, `nf-game --lib mp::` 12/12,
  `mp_bots_live` 1 passed + 1 ignored.
- What I ran: `-p nf-view --test mp_bots_live`, `-p nf-view --test net_match_live`,
  `-p nf-view --test mp_menu_flow`, `-p nf-game --lib`. **The full fast suite did not run** —
  the reset + setup + wiring consumed the window (no `suite.out` total; the next session should
  start it in its first minutes per the standing instruction).

## The per-bot table after 3600 ticks (the honest RED)

| bot | walked | shots | deaths | respawns | score |
|-----|--------|-------|--------|----------|-------|
| 0   | 0.0    | 0     | 0      | 0        | 0     |
| 1   | 0.0    | 0     | 0      | 0        | 0     |
| 2   | 0.0    | 0     | 0      | 0        | 0     |

`quick_game_bots_play_the_match` asserts every row moves/fires and at least one bot dies; it is
committed `#[ignore]` with the reason (a RED test on the branch would break every later suite
run for the other tasks). The trace evidence (`NF_BOTLOG=1`): all three brains sit in state
0xF9 (BotIdle) with the init transition done and no further transitions — the driver works, the
brains starve.

## Exactly what's left (the next session's first hour)

1. **Feed the goal picker** — the brains' `pick_goal` Phase C scores `ctx.pickups`; the driver
   passes `&[]`. The arena's MP pickup placements are already parsed (the load's 0x26 walk;
   the deathmatch pickup type lands in `MpObjects` — confirm the type word from the arena's
   placements [D 07000023]) — lift them into `BotPickup` rows (pos, class, weapon id, the
   visit stamps) and pass the slice. The brain then picks a pickup goal, enters
   BotGotoGoalPosition (0xEB) and walks (`move_to_goal_position`'s straight-line stand-in —
   the ported `NDrone2_MoveToPosition` 0x1517d8 [M]) — movement + route ticks follow.
2. **Feed the acquisition** — `find_opponent` never sets `vars.opponent`: fill the `others`
   rows' `dist2` (the field the ported pick/trait code reads) and check the ported vision gate
   against the port's `world.segment` LOS (the brain already runs one in its perception pass);
   with an opponent found, the grudge/hunt path (pick_goal Phase B) and the attack states
   produce the `BotShot`s the driver resolves (the shot→damage→frag→respawn bookkeeping is
   already wired and unit-shaped — the first kill lights the score table).
3. **The human damaging bots** — the player bullet path hits drones only; the bots are not
   drone objects, so Bond's shots cannot hurt them yet. Either host the bots as drones with
   the MP character skins (the render-faithful route; the arena's character-mesh census is
   unverified) or extend the player-bullet hit resolution to the bot bodies (a smaller, [S]
   stand-in).
4. **The render** — the bots have no meshes (the sim-only hosting); the main agent's render
   checks apply once skins resolve.
5. **The full fast suite** on the final tree (the standing first-minutes instruction), then
   the finish steps this report skips.

## Live checks for the main agent

1. PCSX2, Quick Game 07000023 with 3 bots: do the game's bots MOVE within the first 10 s of
   the match (they should head for pickups/roam) — settles whether the real game's bots route
   immediately or idle while the match gates warm up.
2. gdbwatch `BOTSTATE_pickGoal` 0x126b88 during the first 600 frames: the pickup-candidate
   count and the picked goal's type (expect a pickup goal with a non-empty MPpickup table —
   the port's Phase-C starve point).
3. gdbwatch `find_opponent`'s write (the `BOT_vars` opponent word) when a bot spawns facing
   the human: expect an acquisition inside the vision cone + LOS — the port's find_opponent
   never fires, and the game-side evidence settles whether the gate is the cel data or the
   dist2 table.
