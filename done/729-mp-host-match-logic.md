## Your task: `mp-host-match-logic` — the live MP match runs the match logic

Audit gap ④: in a live match the host never runs the match logic. `update_objects`, `player_killed` and respawn are
only exercised by tests. Wire them into the running match loop (local and online host) the way `MP_Update`
drives them [M].

If task 704's `s/mp-bots-in-match` branch exists, rebase onto it rather than redoing its wiring.

Tests: a local deathmatch in the real app path. A kill scores, the victim respawns at the policy's point, and the
frag limit ends the match into the debriefing.

The uncited-gameplay audit (task 710, merged) has this gap's description, caller chains and acceptance criteria in `docs/systems/uncited-audit.md`. Read that section first and meet its criteria.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/mp-host-match-logic`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox6 at 2026-10-16T20:59Z

---
## Report (done, 2026-10-16T22:43Z)

# mp-host-match-logic — the live MP match now runs the match logic

Task 729, audit gap ④ (`docs/systems/uncited-audit.md` §4): in a live match the host never ran
the match logic — `update_objects`, `player_killed`, the respawn and the debriefing handover were
only exercised by tests, so no scenario could be scored or won in a real ride ("every MP mode
degrades to spawn + clocks + debrief"). This branch wires them into the running match loop — the
same `Sim::step_bond` path the local game and the online host tick — the way `MP_Update` and the
death path drive them in the game. Branch based on `s/mp-bots-in-match` (704, per the task note),
so the bot wiring is inherited, not redone; merged `origin/master` before the final suite run.

## What was wired (every site cites its game address)

1. **The scenario objects build at `MP_Start`** — `crates/nf-view/src/lib.rs` (the MP load path):
   `MpMatch::create_mode_objects` now runs right after `MpMatch::start`, BEFORE the humans' spawn
   picks — the game's draw order [M `MP_Start` 0x183a50:77-114 (the demo/prot/blueprint/GE
   `Rand_Rand` draws) → :120-123 (the humans' `MP_GetSpawnPoint` picks)]. Without it the live
   match's `objects_rt` stayed empty and no flag/uplink/hill/site/GE/blueprint could ever exist.
2. **`update_objects` runs every frame** — `Sim::step_bond` (`crates/nf-view/src/scene.rs`), the
   port's `MP_ObjectUpdate` 0x18b198 walk (the game's object system drives it every frame; the
   master/started gates are inside). Order = `update_objects` then `update` — the headless
   lockstep rig's recorded order, so the live host and the rigs agree bit-for-bit. `on_hill` is
   the caller's KOTH volume test [M `MP_KOHUpdate` 0x186ae0:88-93 — `Intersect_SphereBox` against
   the hill obj's trigger box]; the port parses no hill trigger box, so the flags pass false and
   KOTH hill scoring stays a rig-only rule (left item below).
3. **The humans' live positions ride the match every tick** — the game reads the live `obj+0x30`
   per comparison [M 0x185098:123-129]; the local path never refreshed `MpGame::slots[].pos`
   after the load (only the net path did), so the spawn policy and the rules saw stale positions.
4. **A kill scores through the real `MP_PlayerKilled` 0x188f70** — `Sim::step_mp_death_respawn`
   fires it on the death's start tick, exactly where `Player_CheckForDeath` 0x190810:91 calls it.
   The killer resolves from the slot's last-shooter word (+0x20, written per allowed bullet hit
   [M `MP_RegisterBulletHit` 0x1857c0:34]; 0xfffe/0xffff = no credit [M 0x183438:59,
   0x183fc0:81]) — the port's stand-in for the same death's HitByList walk. The bot frags in
   `Sim::step_mp_bots` now go through `player_killed` too (the inline score/kills bookkeeping is
   gone), and the bots' hits on the human register through `register_bullet_hit` first — a
   blocked (friendly-fire) hit deals no damage, the game's order [M 0x1857c0].
5. **The 5 s respawn** — `Player_HandleDeath` 0x190bb0:41-54's MP arm: after `FRAME_RATE_INT * 5`
   frames from the death stamp the host calls the new `MpMatch::mp_respawn` (`crates/nf-game/
   src/mp/rules.rs`) — the `MP_ReSpawn` 0x185368 composition: the gates [M :16-20], the policy's
   spawn pick [M :31], the apply (`do_respawn` — the streak/last-shooter/status resets), the
   health `armor_bonus + 100.0` [M :65-73], and the body moves (`Player_StandAtNewPosition`
   [M :97] — the port's `Bond::stand_at`, the load's convention). The bots keep their own
   `BOT_respawn` 0x124e38 path in `step_mp_bots`; the remote humans have no death pipeline in the
   port (their slots never die here — left item).
6. **The frag limit ends the match into the debriefing** — the app glue (`crates/nf-view/src/
   lib.rs`, the tick loop): when `MpGame::request_debriefing` latches (`MP_Update` 0x184bd0's
   case 4 — `GameState+0x1C = 0x40000033`), the host re-creates the front end (the load closed
   it; the same pattern `net_fail` uses) and calls `open_mp_debriefing` [M `P_MPDEBRIEFING`
   0x212198 msg 0x4c]. The frame loop's `fe.is_none()` gate then stops the sim ticks behind the
   page — the match freezes under the debriefing like the game's front-end takeover, and the
   debriefing page's own arms (mp-debriefing-data2) take over.

## Tests (both in `crates/nf-view/tests/mp_host_match_live.rs`, real app path —
`load_mp_carry` with the menu's settings, `Sim::step_bond` ticks; the scripted kill enters
through the game's own `MP_RegisterBulletHit` seam plus the raw `weapon_data+0x0C` damage word
the port's hit stand-in applies)

- **`local_deathmatch_kill_scores_respawns_and_ends_into_the_debriefing`** — a local deathmatch
  on arena 07000023 (frag limit 2, one Quick-Game bot). Kill 1: the hit registers (the
  last-shooter word records the attacker [M 0x1857c0:34]), one app tick scores it through
  `player_killed` (the killer's kills/streak/score, the victim's death count, the dead marker)
  while the match keeps playing; 299 dead ticks in the victim is still dead (the 5 s gate
  [M 0x190bb0:41-54]); the 300th respawns him — the death sequence ends, the health is exactly
  `armor_bonus + 100.0`, the last-shooter word resets [M 0x185368:56], and the body stands at
  the spawn policy's point (re-derived in the test from the match's public state: policy 0's
  least-far candidate [M 0x185098:141-146], the same first-of-equal rule, the unset fallback
  [M :165-176] — the bot is inert, its position stable). Kill 2: the frag limit latches
  [M 0x184268:170-172], the end states run 0→1→3→4→5 [M 0x184bd0], `request_debriefing` sets,
  the victim stays dead past the gate (the state gate refuses [M 0x185368:16-20]), and the
  host's handover lands on `Page::MpDebriefing` with the board built: the bot ranks first with
  the 2 frags, the victim's 2 deaths on the board, an FFA win line [M 0x1848d0:79-84].
- **`ctf_capture_scores_in_the_live_host_like_the_headless_rules`** — the audit's acceptance:
  the live host loads 07000023 with the CTF scenario (both flags and both bases build at
  `MP_Start` now), the carrier picks up the enemy flag through the touch seam, parks on his
  base, and one live tick scores the capture (`update_objects`' `MP_FlagUpdate` 0x1861a8 half:
  team +1.0, carrier +1.0, the flag resets home [M :89-112]) exactly like a headless `MpMatch`
  driven through the same events — the headless-vs-live parity the gap asks for. Sister: the
  untouched team scores nothing.

## Verification

- `cargo test --offline --release` (the merged final tree, `--test-threads=1`):
  - `mp_host_match_live` 2/2 (the new tests above);
  - nf-game: `mp_match_rules` 10/10, `mp_debriefing` 14/14;
  - nf-view: `net_match_live` 3/3 (the online lockstep holds — the wiring is deterministic and
    runs on every machine), `mp_bots_live` 1/1 (+1 ignored: 704's known-RED 3600-tick proof,
    still starving on the empty pickup/opponent inputs — not this task), `mp_core_live` 1/1
    (+1 ignored), `mp_menu_flow` 6/6, `mp_debriefing_flow` 9/9, `mp_net_lobby` 9/9,
    `menus_e2e_mp` 3/3.
- **Full suite (fast runner, final tree merged with origin/master): `1943 passed, 6 failed,
  73 ignored`.** The 6 failures are exactly the allowed set: the 3 `disc_golden` bigf/manifest
  tests (the slim ISO) and the 3 pre-existing `0c_pylon` GPU render probes (no GPU in the
  sandbox). Zero MP failures, zero regressions. Runner note: this sandbox kills background
  jobs between tool calls and the sandbox tool timed out on the foreground call, so the
  runner's own total line never printed — the totals above are the sum of all 451 test
  binaries' `test result:` lines in the runner's log (`~/suite.log`), the same arithmetic the
  runner's summary performs; every binary completed.

## Left items (the next task's list)

- **The bots still idle** (704's gap, untouched here): `ctx.pickups` is empty (the arenas' MP
  pickup placements are not lifted into `BotPickup` rows) and `find_opponent`'s dist/cel inputs
  are stand-ins — so a bot-driven kill is not deterministic yet; the test scripts the kill at
  the `MP_RegisterBulletHit` seam the bot pipeline will feed.
- **The KOTH hill volume**: the trigger box is the hill object's level data (obj +0x20/+0x30);
  not parsed, so the live host passes `on_hill = false` and KOTH scoring stays rig-only.
- **The remote humans' death/respawn**: the port's parked bodies have no health pipeline, so
  only the local human's death sequence drives the rules today.
- **The touches are event seams**: `touch_*` stand in for `MP_HitBy` 0x1830e0's room-contact
  walk; the live host calls them from script/tests, no spatial overlap pass exists yet.

## Live checks for the main agent

None required — the wiring is verifiable headless (the two tests above pin every rule against
the decompile's addresses). Optional confirmation on a real ride: a Quick Game deathmatch —
kill an enemy (or die), expect the 5 s respawn at a spawn point and the frag-limit debriefing
page; the death pane must NOT open the SP fail menu (the MP arm of `Player_HandleDeath`
0x190bb0 skips the ch-0x62 path — the port's SP death sequence runs but never reaches 448 ticks
before the respawn clears it).

## Usage

not visible to me
