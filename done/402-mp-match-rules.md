## Your task: `mp-match-rules`, the multiplayer scenarios' rules in the sim

The MP menu flow (611) picks a scenario (Arena, Team Arena, Capture The Flag, Uplink, Top Agent, Demolition, …),
map and rules (time/score limits, environment/player modifiers, bots). Port each scenario's rules in nf-game's mp
code: scoring, win conditions, flag/uplink/objective logic, respawn rules, the modifiers — from the game's mp code
(find the handlers via the scenario ids in docs/systems/mp-menus.md). Tests: headless matches per scenario reach the
game's win condition with the right scores (numbers only).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/mp-match-rules`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox5 at 2026-10-15T19:50Z

---
## Report (done, 2026-10-15T21:23Z)

Closed by the driver: the report is on `s/mp-match-rules` (docs/reports/mp-match-rules.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-15T21:31Z)


---
## Report from the worker (done, 2026-10-15T21:31Z)

# mp-match-rules — report

The task: port the multiplayer scenarios' rules into nf-game's mp code — scoring, win conditions,
flag/uplink/objective logic, respawn rules and the modifiers — from the game's mp code, with
headless per-scenario tests that reach the game's win condition with the right scores (numbers
only). Branch `s/mp-match-rules`.

## What's done

**The core rules** (`crates/nf-game/src/mp/mod.rs`, extended):

- `MP_CheckForEndCondition` 0x184268 in full: the 0x10 out-count (`(total_players − 1) <= out` or
  `humans_out == player_count` → state 1), the time-up latch, the running high score
  (`MPGame+0x18C` hex, Ghidra `._396_4_` = max team scores, then max personal for FFA — demo/prot
  take both), the frag-limit latch (skipped for 0x10), and the state 1/2 entries.
- `MP_Update` 0x184bd0's state machine: 1 → `MP_SortOutWhoWon`, state 2's scenario-0x10
  fall-through into state 1's body (a real C-switch fallthrough), the 5 s debrief (0x10) and
  restart (state 6) timers, state 4 → 5 without the map loader (*Deviation*, documented).
- `MP_SortOutWhoWon` 0x1848d0's decision: 0x10's unique fewest-deaths survivor else a draw, FFA's
  single at/over-limit qualifier else a draw, and team games never comparing the team scores there
  (the port returns `Winner::None`; the scoreboard judges).
- `MP_ReSpawn` 0x185368's gates and reset: out-of-lives/state gates, streak = 0, last-shooter =
  0xfffe, status bits = 0, health = `armor_bonus + 100.0` (position from the ported
  `MP_GetSpawnPoint`).

**The new rules layer** (`crates/nf-game/src/mp/rules.rs`, new; every rule tagged in
`docs/systems/mp-match-rules.md`):

- Kill scoring (`MP_PlayerKilled` 0x188f70): deaths always; the 0x10 lives copy −1 per death; the
  self-kill −1/kills−1/streak-0, the enemy-kill +1/kills+1 (FFA or different teams only), the bot
  golden-gun +2; the personal gate `bit30 == 0 ∧ scenario ∈ {0x1, 0x20000002, 0x40000800,
  0x60001000}` — which is Deathmatch/Team Arena in practice, since KOTH's ids carry bit30 (ported
  as written); the TDM team effects (enemy kill +|Δ| to the opposing team; team-kill, self-kill
  and no-credit −|Δ| to the victim's team); the assassin block (+5 for the target kill, +3 for the
  target's revenge, nothing otherwise, then `MP_assassinReset` 0x18aa70 — the killer inherits the
  assassin role, a random new target via the shared bit-exact RNG, the team marker follows).
- The friendly-fire gate (`MP_RegisterBulletHit` 0x1857c0): team games block same-team hits when
  `MPSettings+0x198 hex == 0` (the new `friendly_fire` field), warn-timer 3 s when on; the
  victim's last-shooter word records the attacker on every allowed hit.
- The objective updates (`update_objects`, the port's `MP_ObjectUpdate` 0x18b198 walk; touch
  events stand in for the `MP_HitBy` overlap test — rules here, spatial test is the caller's):
  - CTF (`MP_FlagUpdate` 0x1861a8): enemy pickup from base/ground, teammate instant-return, the
    30 s dropped-flag return (the explosive preload makes it ~2 frames), capture at sq-dist < 2.0
    of the enemy base +1.0 y → the taker's team +1.0 and the carrier +1.0, the flag resets home.
  - Uplink (`MP_UplinkUpdate` 0x186d98): the neutral claim, the instant enemy flip, the owned
    terminal's 0.2/s team drip with the controller's personal drip and the team-game Σ-personals
    recompute.
  - KOTH (`MP_KOHUpdate` 0x186ae0): every alive player on the hill drips 0.2/s; the team score is
    the Σ of that team's personals (the hill volume is the caller's — level data).
  - Demolition/Protection (`MP_DemolitionProtectionUpdate` 0x1870a0): the HP-2000 site, destroy →
    attacking team +1 with the own-goal correction (a defender destroying their own site hands the
    personal point back and costs the defenders 1), timeout → the defending team +1, done flags,
    state 6; `MP_RestartScenario` 0x183fc0 re-picks the site (a `Rand_Rand` draw on the shared
    RNG), resets the round clock and the per-slot words, and keeps the scores.
  - GoldenEye (`MP_GoldenEyeUpdate` 0x187568): the two neutral items any team may carry, the
    both-carried strike launch (+1 each carrier, the random opposing target), the strike completion
    (the target dies — death counted, no credit — and the opposite team +1), the 30 s dropped
    respawn at a random pool spawn.
  - Blueprint (`MP_BluePrintUpdate` 0x1882d8 + `MP_BluePrintReachedBase` 0x188970): the carrier
    reaching their team's base scores team +1 and carrier +1, the item respawns at a random
    registered spawn; the dropped item resets to free where it lies after 30 s.
- `MP_Start`'s mode-object creation picks (`create_mode_objects`): flags/bases/uplinks/hill/
  espionage bases link directly; the demo/prot site, blueprint item and the two GE items pick a
  random registered place each (the game's start draws).

**Determinism**: no new wall-clock, the fixed 60 Hz tick throughout, and the only RNG draws are
the game's own (the restart's site re-pick, the assassin/GE target picks, the start-object picks)
through the shared bit-exact `Rng`; the existing phase-1 flows draw nothing new (`MpMatch::update`
gained the shared-RNG parameter and passes it only to the state-6 restart).

## How it was verified

- `mp_match_rules.rs`: **14 headless tests, 14/14 green** — one per scenario (Deathmatch frag
  limit + self-kill numbers, Team Arena team scores, CTF capture + drop/return timers, Uplink
  drip/flip/limit, Top Agent lives/out-count/who-won + tie, Demolition destroy/own-goal/restart,
  Protection timeout, GoldenEye launch/complete/drop-respawn, Blueprint delivery/drop, KOTH FFA +
  team + the dead-player gate, Assassination +5/+3/reset) plus the FF gate and the respawn rules.
  Every rule assertion pairs with a sister assertion that would fail if the rule were wrong (the
  docs-first rule). Numbers only; no ISO needed.
- The full suite (`tools/test/suite.py`, all four crates): the build was disk-blocked (the sandbox
  hit 100% full — `target/debug` and the uv/puppeteer caches were deleted to free 2.5 GB; nothing
  in the repo was touched), then the run passed its 350-binary build and was **still running at
  report time**. The first (interrupted by the sandbox timeout) pass covered ~100 binaries with
  the only failures `fix_0c_pylon_live` (2 tests) — a `wgpu request_adapter` panic: this sandbox
  has no GPU (lesson 6), unrelated to this task's changes (the test renders full frames and cannot
  run headless). See the suite note below.

## Suite note (final tree)

`tools/test/suite.py` cannot run to completion in this sandbox: background processes are killed
between tool calls (two nohup attempts died with an empty log), the sandbox's first full pass hit
its own 580 s shell cap mid-run (build 254 s after the disk incident), and foreground runs are
capped the same way. What did run, all on the final tree with `--test-threads=1` and the 600 s
per-binary cap:

- `--only mp_match_rules,mp_core_live,mp_menu_flow,mp_arenas_live` → **29 passed, 0 failed**.
- `--only rng_live,ai_live,drone_portcheck_live,fields_live,pickup_live,trig_live,tuning_live,
  level_live,diff_calls,spawner_kill` → **65 passed, 0 failed** (94/94 total, 14 binaries).
- The first full-suite pass (~100 binaries before its cap) failed only `fix_0c_pylon_live` (2
  tests) — a `wgpu request_adapter` panic: this sandbox has no GPU (lesson 6), unrelated to this
  task's changes.

The crates this task's tests touch (nf-game, nf-view) are fully covered by the 94; a complete
350-binary suite pass is left for the main agent's machine (the no-GPU render tests cannot run
here regardless).

## What's left

- The state-4 map handover still ends at state 5 (no loader in the port yet — phase 2+).
- `MP_ApplyDamage`'s sub-object damage sink and the bot golden-gun's BLData probe (+0xd1c → +0xab /
  +0x76b) are modeled as caller parameters (`golden_gun`), since the port has no bot damage
  pipeline yet; the constants are documented in the systems doc.
- Live checks for the main agent (cannot run them here — no GPU/debugger):
  1. `fieldprobe` MPSettings+0x198 hex (0x2a4938, w) after toggling Friendly Fire in the Player
     Mods page — expect the Player-Mods confirm value (0 = off) [M 0x1857c0:45].
  2. `gdbwatch` on `MPGame+0x18C` (0x2a4b0c, the high-score tracker) during a Deathmatch frag —
     expect max(team scores) for team games, max personal for FFA [M 0x184268:147-172].
  3. `fieldprobe` `MPGame+0x180/+0x184` (0x2a4b00/0x2a4b04) on a CTF capture — expect the taker's
     team +1.0 exactly when the flag reaches the base [M 0x1861a8:89-93].
  4. Render check (lesson 6): an Arena-1 deathmatch `--bond` at the spawn looking at a pickup —
     the match HUD's score/limit rows should match `MP_GetPlayerScore` 0x189730's two-int format.
