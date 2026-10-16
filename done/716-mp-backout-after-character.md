## Your task: `mp-backout-after-character` — local MP setup breaks after backing out

Playtest 1015-2231: "multiplayer menu breaks if you back out after setting a character in local mp".

Walk the local MP setup by actions only (menus_e2e.rs style): pick a character for player 1, back out one page,
then back to the main MP menu, then forward again. Repeat with 2 joined players. Find the state that goes wrong:
- the join slots, the cursor or the team/character prompts (`enter_mp_setup` states 2/3);
- the pair or overlay stack.

Compare with the game's page handlers for the character page and its Back case [M the MP character-select page
handler]. Fix it.

Tests: every back-out sequence above returns to a working page, and a match can still be started afterwards.

First check whether current master already behaves like the game. If it does, pin it with a test that would fail on the old behaviour, and say so in the report. Don't change working code to make a task look busy.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/mp-backout-after-character`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox1 at 2026-10-16T18:53Z

---
## Report (done, 2026-10-16T20:40Z)


---
## Report from the worker (done, 2026-10-16T20:40Z)

# mp-backout-after-character (queue 716)

The playtest 1015-2231 symptom: "multiplayer menu breaks if you back out after setting a
character in local mp". This task walked the exact sequence by actions only — pick a character
for player 1, back out one page, back to the main MP menu, forward again — with 1 joined human
and with 2 joined players, found the two states that went wrong, fixed both, and pinned the
whole walk in tests that fail on the old behaviour.

## What was wrong (found on master, both fixed)

Master already had the 1015-2231 first-pass fix (the options hub's Back re-runs the setup page's
enter, so the prompt re-derives from the scenario's team bit — commit 99c30d0d, on master). The
task's longer walk exposed two further divergences behind it:

1. **The setup page's enter kept the agents' ready bytes.** The game clears mpjoin+0x01 (ready)
   for every one of the 4 agents on every P_MPSETUP enter [M P_MPSETUP 0x20d7a8's 0x4c loop —
   `pcVar5[1] = '\0'` per slot]. The port's `enter_mp_setup` re-derived the prompt but left
   `mp_join[i].ready` set. Fix: the enter clears all four ready bytes.
2. **The re-entered Join Game page kept stale records.** The game memsets the whole mpjoin array
   (0x40 bytes) on EVERY P_MPJOIN 0x4c — the gate reads DAT_002e1b09, set to 2 in the same arm —
   then zeroes the per-agent states at mp_stuff+0x360 and re-seats the defaults: skins 0/1/2/3
   (DAT_002dee70/80/90/a0) and teams MI6/Phoenix/MI6/Phoenix (DAT_002dee6c/7c/8c/9c = 1/0/1/0)
   [M P_MPJOIN 0x20ca00's 0x4c]. The port entered the page without any of it, so after backing
   out of a half-done setup to Main and walking forward again, the join page still showed
   "Player Ready" and that panel's ✕ fell through (`5 if !self.mp_join[0].ready` could not fire)
   — the "menu breaks" moment. Fix: the MpMode-Local route runs a new `enter_mp_join()` (fresh
   records + the game's defaults) instead of the bare page assignment. The host-lobby path
   (`mp_host_begin`) keeps setting its own panel after the page change, so net hosting is
   unaffected; the net mirror only runs while hosting/in-lobby, so the local walk never races it.

Both fixes cite the game addresses above; no state constants or match arms were renamed.

## Tests

- **New** `crates/nf-view/tests/menus_backout_after_character.rs` (actions only, headless):
  - `ffa_backout_after_character_returns_to_a_working_menu_and_a_match_still_starts` — boot →
    Main → Multiplayer → Local → join/codename/ready → Arena → Skyrail → the FFA character
    prompt → pick → handicap → the options hub → Back (the setup re-enters on the character
    prompt AND every ready byte is cleared) → Back to Main through the map and scenario wheels →
    forward again (the join page is a FRESH join; the agent state, joined and ready bytes are 0)
    → re-pick → add a bot → Continue → the match starts with Skyrail and the settings ride.
  - `two_joined_players_survive_the_same_backout_and_the_match_starts` — the same walk with 2
    joined humans through the host lobby (the port's only two-human path — the game joins agents
    2-4 by pad, which the port has none of); ends with `human_count == 2` riding the request.
  Both FAILED on the pre-fix tree at the ready-byte assertions (the first divergence each), and
  pass now — they are the "would fail on the old behaviour" pins. They also pin the earlier
  99c30d0d fix (the hub's Back re-enters on the character prompt for FFA / the team prompt for
  team games).
- **Updated** `crates/nf-view/tests/menus_e2e_mp.rs`:
  - the re-entered join page in `back_out_of_every_page_and_walk_forward_again_team` now asserts
    the fresh join [M P_MPJOIN's 0x4c memset] instead of the old stale-panel behaviour (that
    assertion contradicted the game's enter);
  - the game's default team for agent 1 is MI6 [M DAT_002dee6c = 1], so the blind team Confirm
    lands MI6 — that test now picks Phoenix explicitly (◀) to keep the Alura bot supplying the
    other side of C_SBMPOPTIONS' both-sides check, and three "team rides" assertions were
    re-seated with the citation.

## What ran

- The touched crate: `cargo test --offline --release -p nf-view` — 180 passed, 0 failed.
- The full suite on the final tree (merge origin/master: already up to date — the branch cut
  from c8186aa1, master unchanged since) via `tools/test/suite.py` in 7 foreground chunks
  (the sandbox kills background jobs between tool calls, so the nohup runner could not survive):
  **1911 passed, 6 failed, 72 ignored**. The 6 failures are exactly the allowed baseline: the 3
  slim-ISO `disc_golden` tests + the 3 pre-existing pylon render probes (`the_0c_pylon_*` ×2,
  `probe_render_pylon_only` — the master baseline recorded in the WORKLOG). No TIMEOUT, no CRASH.

## What's left

- Nothing for this task. Sister observations for a later pass (not bugs in this walk):
  - the port's MpJoin Back resets the agent to the idle panel while the game's C_RBMPSTART 0x6b
    lands on the codename list (state 1) [M 0x20d048's 0x6b arm] — outside this task's walk;
  - the codename strings live in MPSettings+slot·0x30 in the game and survive a join-page
    re-enter; the port keeps them in the join records and re-picks them in the flow, so the
    observable walk is identical.

## Live checks for the main agent

None required — the walk is fully headless. Optional (only if the merged picture should be
spot-checked on hardware): on the Join Game page after backing out of a half-done local-MP
setup, panel 1 must show "Press ~A to join" (0x1e8), not "Player Ready" (0x39c)
[M P_MPJOIN 0x20ca00's 0x4c memset]; and after the hub's Back, the setup prompt must be the
character again with ready cleared (mpjoin+1 @0x2dee69 == 0) [M P_MPSETUP 0x20d7a8's 0x4c].

## Usage

not visible to me — this sandbox exposes no token, cost or context-window figures (no
`/status`-style panel; the CLI prints none of these numbers).
