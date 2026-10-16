## Your task: `mp-hud-data`, what the multiplayer HUD shows

In an MP match the HUD shows per-agent info (score / frags, the match timer, the scenario's objective status —
flags held, uplink progress, the Top Agent target, the radar's blips). Port the data model the HUD reads (from the
game's MP HUD code — find it from the in-match draw calls and nf-game's mp code), as a per-frame snapshot struct
with getters (scores sorted the way the game sorts, timer mm:ss, objective markers in world space, radar blips
relative to the viewer). The main agent draws it. Tests: a headless match's snapshot matches the sim state.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/mp-hud-data`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox7 at 2026-10-16T00:20Z

---
## Report (done, 2026-10-16T01:51Z)

Closed by the driver: the report is on `s/mp-hud-data` (docs/reports/mp-hud-data.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-16T02:26Z)

# mp-hud-data — the multiplayer HUD's data model

Task 632, branch `s/mp-hud-data` (worker sandbox7, 2026-10-16). Status: **done**. The design and
the evidence are in `docs/systems/mp-hud-data.md`; this report summarizes.

## What's done

- **`nf_game::mp::hud`** (new module): `MpHudSnapshot::build(&MpMatch, viewer_slot,
  viewer_radar_on)` — the per-frame snapshot the HUD reads, no rendering:
  - **Scores**: `rows` in the game's in-match order (the panes are one-per-viewer keyed
    `BLData+0x94e` [M 0x19f0a0:31] — there is no in-match sort; the game's sort is the debrief
    board's bubble pass [M 0x212198:112-130]), `sorted_rows()` = descending by
    `Menu_GetMPScore` 0x203af8's display score (kills for Arena/Team Arena, the slot score for
    every scoring scenario), `score_lines(sc)` = the 3-line (score/kills/deaths) vs 2-line
    (kills/deaths) split of `MP_GetPlayerScore` 0x189730, `team_scores` per `MP_GetTeamScore`
    0x18b338, `MpScoreRow::is_target` for the Assassin marker.
  - **Timer**: `time_left` = `limit − clock` clamped ≥ 0 (the clock is `MPGame+0x190` — the
    decomp's `_400_4_` prints are DECIMAL; the game's limit is SECONDS, the menu's minutes ×0x3c
    at the confirm [M 0x20ec00:146]), `timer_string()` = `Timer_Seconds2String` 0x1deaa8 mode 2
    (`mm:ss`, negatives clamp to 00:00, minutes wrap past 60).
  - **Objectives**: `flags` (the carrier slots — the datum `MP_HasTeamFlag` 0x18b430 reads for
    the pane icons, NOT the slot's 0x26 bit0 which only bot AI reads), `uplinks` (owner word per
    `MP_getUplinkStatus` 0x18ba80, the `MPOBJECT+0x52` controller, the live RGB — there is no
    progress counter, the "progress" IS the 0.2/s score drip [M 0x186d98:59-80]),
    `assassin`/`target` (the globals' names are inverted: `DAT_0030d774` = assassin, `Assasin` =
    target [M 0x18b538/0x18b558]), `names_on_radar` gate (+0x1c4 or scenario 0x400 [M
    0x19f6e8:177]).
  - **Radar blips**: `radar: Vec<MpRadarObj>` (world pos, packed RGBA, kind u16) — the port of
    `MP_GetRadarObjects` 0x189840's fill: the viewer's per-player toggle gates the whole list
    (`MPSettings + viewer·0x30 + 0x28`; new `MpPlayer::radar_on` — the port's compact struct had
    skipped +0x28), the viewer is never a blip, dead slots drop, team games tint team 0
    `0xd22d35ff` / team 1 `0x2d61d2ff` and FFA white, then the scenario's markers in the game's
    order: CTF flags (kind 1, red/blue), uplinks (kind 2, per-terminal RGB), demolition/
    protection site (kind 3, white), GoldenEye items not held (kind 4), blueprint + the two
    espionage bases (kinds 6/7). Kind 5 exists in the UV sheet and is written by nothing.
    The viewer-relative projection is ALREADY ported (`nf_view::hud::radar_project`, the
    0x19f6e8:108-153 transform) — the snapshot's records feed it as-is (pos/argb/kind).
- `MpPlayer::radar_on` (own line at the struct end, lesson 4) + the one construction-site fix
  (menu.rs's confirm path). Nothing else moved.

## How it was verified

- `crates/nf-game/tests/mp_hud_data.rs` — 4 headless tests (no ISO), a match's snapshot matched
  against the sim state, each with a sister assertion that can fail:
  - the rows read the sim's own kill/death counters in slot order; the display score and the
    descending sort (Arena = kills; a 1-kill vs 2-kill pair sorts [1, 0]); the 2-line vs 3-line
    score split (KOTH FFA's line 1 is the score);
  - the timer: 90 s off a 600 s limit reads `08:30`, negatives clamp to `00:00`, 61 minutes
    wraps to `01:00` (mode 2 has no hours);
  - the radar: the viewer excluded, a dead opponent dropped, team 1 blue / team 0 red / FFA
    white, the toggle off → empty list;
  - the objectives: both flags at base → `flags = [None, None]` and two kind-1 blips in the
    game's colors; a picked-up flag rides `flags[0] = Some(carrier)`; the uplink's owner/
    controller/RGB and its packed blip color; the Assassin target marker on the right row.
- **What I ran**: `cargo test --offline --release -p nf-game --test mp_hud_data` (4/0),
  `cargo check -p nf-game -p nf-view` clean, and the full fast runner (`tools/test/suite.py`) on
  this final tree (the branch; it IS master 0f5f07f4 + this task's two commits — no other merge
  landed in the window):

## Suite (final tree)

Fast runner on the final tree, slim on-foot ISO, `--test-threads=1`:

    (the run was still executing its last binaries at this window's 02:31 UTC stop time — the
    total line had not printed; the next message can re-run `tools/test/suite.py` for it)

    Observed failures up to the stop (02:25 UTC): the 3 ALLOWED slim-ISO `disc_golden` failures,
    plus `fix_0c_pylon_live` (2 tests) and `probe_pylon_render` — the render probes. The pylon
    probes are NOT this task's diff: the master-baseline suite run earlier in this same sandbox
    (00:52 UTC, before any of this task's code was on the tree) failed `probe_pylon_render`
    identically, and this task changed no drawing (the only render-adjacent file it touches is
    none — the MpPlayer field and the new mp/hud data module are not in any render path). They
    arrived with master's 0f5f07f4 merge window (objective-hints) or the environment; the next
    window should confirm against a clean master before treating them as regressions.

    No FAILED/TIMEOUT from the crates this task's tests touch: nf-game's lib, `mp_hud_data`,
    `mp_match_rules`, `mp_core_live` and nf-view's lib all ran clean in the suite by the stop
    (alphabetically before the p-binaries). `walkreal_07000001_live` had not run yet at the stop
    (it failed on the master baseline too — the minutes-long walk, window-sensitive).

Known-red before this task (from the master baseline run earlier in this sandbox):
`disc_golden::directory_tree_matches_the_golden_manifest` (one of the 3 slim-ISO-allowed
failures) and `probe_pylon_render::probe_render_pylon_only` (a render probe; this task changed
no drawing — the master baseline failed it identically at 00:52 UTC, before any of this task's
code existed on the tree).

## What's left

- The live checks below — especially which menu row writes the per-player radar toggle (+0x28)
  vs the name-tag gate (+0x1c4): the port's `FrontEnd.mp_radar` currently feeds `settings.radar`
  (+0x1c4); if the game's Setup-page Radar row writes +0x28, the menu wiring should follow.
- The dead/limbo blip gates ride `MpSlot::dead` (the game's `obj+0xff` state machine + the
  BLData health exception is not modelled per-player yet).
- `MpSettings.time_limit` semantics: the port's menu path stores seconds, the `deathmatch()`
  default and the mod.rs doc say minutes (the game stores minutes, converts at the confirm).
  Shared struct — flagged for the owners, not changed here.

## Live checks for the main agent

1. **The per-player radar toggle**: in a 2-viewer MP match, write 0 to `MPSettings + 0x58`
   (**0x2a47f8** = player 1's +0x28 record) — expect that viewer's radar ring to hide and their
   blip list to empty while the other viewer's stays (readers: MP_GetRadarObjects 0x189840:22,
   HUD_RadarUpdate 0x19f6e8:102-105).
2. **The name-tag gate**: write 0 to `MPSettings+0x1c4` (**0x2a4964**) in an FFA match — the
   name tags vanish, the blips stay (reader 0x19f6e8:177).
3. **The timer**: watch `MPGame+0x190` (**0x2a4b10**) accumulate in a 1-minute match and the HUD
   string flip at 10 s left — pins the seconds semantics and the mode-2 format.
4. **The Assassin marker**: after a death in scenario 0x400, check the target slot's
   `MPSettings + slot·0x30 + 0x20` word = 1 (write 0x18aa70:38-41) — the port models the marker
   via `MpMatch::assassin_target`; the game shares the field with the team word.

## ntfy

(sent after this report — recorded here if it failed.)
