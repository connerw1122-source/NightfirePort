# fix-impact-remainder2 — landing 110q: the impact draws' frame order

Branch `s/fix-impact-remainder2` (from `origin/s/fix-impact-remainder` 6ed08abb = 110q, merged with
origin/master a2958c03). Task: 110q was HELD with **1333 passed / 2 failed** on master+110q (master
alone green) — `ai_live::guard_takes_cover_like_the_game` (`UnderCoverAim lasted 90 (live 37)`) and
`walkreal_07000001_live::walk_from_the_spawn_to_the_exit_with_the_contact`. Both come from the
branch's new RNG consumption. Do: verify WHEN in the frame the new draws happen against the game's
order with a per-tick RNG call log, fix the interleave, show both tests green and the full suite
green.

## The root cause, with the game's evidence

**The game's object list is NEWEST-FIRST, and the fresh emitter updates FIRST on the tick after
its creation — before the drones' AI.**

- `control_create_object` 0x1344e0 → `control_add_object_to_list(0x2705a0, obj)` 0x134698 [M]:
  `obj->next = head->next; head->next = obj` — every new object is PREPENDED. `docs/fields/obj-tag.md`
  pins it ([M*] 0x134698 / 0x133178, "add prepends", snapshot-verified 523-object walk [L]).
- `control_movement_object_handler` 0x1336f8's pass 1 walks the list from head+0x14 via obj+0x14 and
  never revisits the head, so an object created MID-WALK (the impact emitter, during the bullet's
  collide phase inside `Bullet_CollisionHandler` 0x12ce80) is **not updated its creation frame** [M
  Collide_Update 0x1ea398 vs the class walk — 110q's own citation] and is the **FIRST object
  updated the next frame** — ahead of the older drones (`Drone_Control` 0x139268 is a control_funcs
  handler in the same walk) [M].
- `Game_Run` 0x1c9790:24-34: `Env_Update` (one `Rand_Rand(20000)`) runs BEFORE the object handler.

**The port's bug (110q):** `Drones::tick` runs the fx pass (`WorldFx::tick`) BEFORE its bullet
collide phase in the same tick. An emitter created during `update_player_bullets` at tick N is
first seen by the fx pass at N+1 — which IS the game's first `Emitter_Update`. 110q's `age_ticks`
gate ("skip the creation tick") assumed the fx pass ran after the collide phase and skipped that
N+1 visit too: **every burst drew one tick late**, and the AI rolls in the N+1 window saw the
stream 4·batch draws short — the cover roll picked the generic 90-tick anim instead of the
lean-out 37, and the walkreal contact's route rolls shifted.

## The fix

- `world_fx.rs`: the `age_ticks` field, the creation-tick `continue` and the `age_ticks > 1` spawn
  gate are REMOVED — the first fx tick after the creation tick draws the burst, exactly the game's
  newest-first first update. Comments carry the full order citation.
- `drone.rs`: the env block (`rand(20000)` + `UpdateDrops`) moved ABOVE the fx pass — `Game_Run`
  runs `Env_Update` before the handler, so the env draw precedes the burst as the game orders it.
- `effect.rs` + `world_fx.rs`: `NF_RNGLOG` caller tags (env-gated eprintln, no behaviour change) at
  `penetration_roll`, `debris_count`, `ricochet_roll`, `emitter_burst`.

## The per-tick RNG call log (the task's requested evidence)

Scratch diagnostic over `guard_takes_cover_like_the_game`'s scenario (seed 1, 700 ticks,
NF_RNGLOG=1), on the fixed tree:

- tick 0: state 0x4 (delta 13 — the snow baseline); tick 61: 0x58 Combat (72); 62: 0x5a (66); 63:
  0x8c RunForCover (67); tick 190: 0x93 UnderCoverInit (93 — the cover roll's tick); 220: 0x94
  (init 30, live 31); 280: 0x95; 370: 0x96 (aim 34, live 37 — inside 32..=40).
- Tag totals over the run: **620 `emitter_burst` draws (155 spawns), 20 `debris_count`,
  4 `ricochet_roll`, 0 `penetration_roll`** (no 0x80 walls in the gate area). The bursts begin at
  draw #4405 — i.e. the guard's own misses draw the game's impact stacks from Combat onward, each
  burst landing on the tick AFTER its wall hit (pairs of +4 tags 4 apart per batch-2 emitter).
- Pre-fix the same log showed the burst a tick late (the new fail-first test: at hit+1 **0 of 5**
  batch slots spawned; post-fix 5/5 and the tick's delta ≥ 4·batch).

## Test results

- NEW `impact_remainder_live::the_burst_lands_on_the_first_update_after_the_hit_like_the_games_walk`
  — **fail-first shown**: on the pre-fix tree `left: 0, right: 5` (the first update after the hit
  spawned nothing). Post-fix: 3/3 green (with the existing two).
- `ai_live`: **35 passed / 0 failed** — `guard_takes_cover_like_the_game` green again. Its seed is
  re-picked **1 → 4** per the test's own documented discipline (the seed is re-picked whenever the
  RNG consumption changes: "fix-ai-remainder had 1, fix-impacts 3, 0072 1, 0056 2…"). The G-041
  draws are REAL game draws (the log above); master's seed was tuned to master's smaller draw set,
  so no ordering fix alone could restore it. Seed 4: states 0x58,0x5a,0x8c,0x93,0x94,0x95,0x96,
  init 30 (28..=34), aim 34 (32..=40, live 37), cover node 1, position err 0.02, sister holds.
  Nine seeds pass fully (0, 4, 6, 7, 8, 9, 16, 20, 21) — 4 chosen mid-range.
- `walkreal_07000001_live::walk_from_the_spawn_to_the_exit_with_the_contact` — **still red**
  (ran once post-fix, 575 s). No seed to re-pick: the walk uses the real level-start stream. The
  contact's route rolls are shifted by the (game-accurate) impact draws, and something in
  07000001 still diverges from the game's stream. Prime suspect (next session): 110q's [S]
  ε 0.01 retrace over-piercing — a bullet that the game stops may pass through and change the AI
  situation; a NF_RNGLOG run of the walk + a PCSX2 spot on a known 0x80/0x40 wall would settle it.

## Suite

Chunked foreground runs (background jobs die between tool calls in this sandbox). On the FINAL tree
(merged with origin/master a1cb5b36's line, push e8e3f882):

- `nf-game`: **308 passed / 0 failed** (ai_live 35/0 inside — the re-seeded cover test green).
- `nf-formats`: **75 passed / 3 failed** — the 3 are the `disc_golden` slim-ISO allowance.
- `nf-audio`: **13 passed / 0 failed**.
- `nf-view` lib 8/0; `impact_remainder_live` 3/0 (the new fail-first test green); `lighting_live`
  5/0; `rng_live`/`light_selection`/`diff_calls` are nf-game bins (inside the 308/0).
- `walkreal_07000001_live`: still **1 passed / 1 failed** — the contact walks r0 further than
  before but "stopped short of the goal at (3.5, 6.3, 7.3) after +1252 ticks (10 s still)". Left
  for the next session (see What's left); everything else on the merged tree is green.

Not run in the window: the remaining long nf-view live binaries (the weapon-matrix class). The
suite runner command for the driver's full pass: `NF_ISO=<abs> python3 tools/test/suite.py`.

## What's left

1. The walkreal contact walk (above) — diagnose with NF_RNGLOG on 07000001 + the PCSX2 pierce spot.
2. The far-surface mirror pin from 110q's report (the bullet-position retrace) — unchanged.
3. If the main agent's full suite shows anything new against the merged tree, take it from this
   branch.

## Live checks for the main agent

- PCSX2, 07000005, the gate area: shoot the P2K past a guard so the miss hits a wall; the spark
  burst (the impact emitter) must fly off the surface on the frame AFTER the hit (not two frames),
  and the guard's subsequent behaviour must match the live trace (cover behind corner node 1).
- PCSX2, a known 0x80-flagged wall (the walkreal suspect): shoot it point-blank — if the game
  STOPS the bullet, the port must stop too; if it pierces, the effects land on the far surface.
  This pins the [S] ε 0.01 retrace.
- nf-view `--bond` spots: the 07000005 start facing the first wall (yaw 0.0): no decal on the
  penetrable near plane, the puff/debris on the far surface, the one-shot spark burst on the next
  frame; and the impact-emitter burst must not appear on the creation frame.

## Files

`crates/nf-game/src/world_fx.rs` (the age gate removed + the burst tag), `crates/nf-game/src/
drone.rs` (the env move + the fx-pass comment), `crates/nf-game/src/effect.rs` (the roll/debris/
ricochet tags), `crates/nf-game/tests/ai_live.rs` (the seed re-pick + the doc), `crates/nf-view/
tests/impact_remainder_live.rs` (the new fail-first test), `docs/WORKLOG.md`, this report.
Shared-file hooks: `drone.rs` only — two comment blocks + the env block move (listed per the task's
shared-file rule). No game bytes, no credentials committed.

## ntfy failed

All three fallbacks failed with the daily-quota 429:
- curl: `{"code":42908,"http":429,"error":"limit reached: daily message quota reached; increase your limits with a paid plan, see https://ntfy.sh","link":"https://ntfy.sh/docs/publish/#limitations"}`
- python3 urllib: `urllib.error.HTTPError: HTTP Error 429: Too Many Requests`
- wget: no output, non-zero (the same 429).
