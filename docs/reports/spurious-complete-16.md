# spurious-complete-16 — report

Task 699, branch `s/spurious-complete-16`. Playtest 1013-1825 (07000016): "I also saw a mission
complete thing pop up for some reason in this mission, but I still couldn't continue."

## What posts the stray message

Not a script writing channel 100, not the type-3 banner, not an inherited channel: it is the
**objective-complete toast** (the player reads "OBJECTIVE COMPLETE" as a mission-complete thing)
posted **by the mission-state fabricator on a cold load**, once per carried prior-part record.

- When a level starts without a mission snapshot, `Mission::init` fabricates a state for every
  prior-part objective record this level still monitors (the carried chain from 07000014/15 on
  07000016). The port fabricated **every** prior-part record as 3 (announced-complete) [M
  crates/nf-game/src/mission.rs, the pre-fix line `self.states[i][j] = if o.flags & 2 != 0 { 2 }
  else { 3 }` in the `level > i` arm]. State 3 is the state whose *arrival* queues the
  "OBJECTIVE COMPLETE" toast, so the first monitor tick after any cold load re-completed every
  carried record and stacked N toasts at the arrival, while the mission itself stayed open —
  exactly the playtest's "mission complete thing … still couldn't continue".
- **The game's own shape proves 3 is wrong for a held condition** [L `work/ee_start_0700000a.bin`
  @ 0x2a3f78 — the `.data` mission records at 0700000a's start, a mid-mission arrival after
  07000009]: 07000009's carried prior-part record whose channel (0x47) had already fired sits at
  **4 (complete, no message)** at 0x2a3f90+0x14; the unfired ch-0x46 record sits at 3 (announced);
  the fail record at 2. The earlier level's own monitor folded the fired record to 4 before the
  unload [M Mission_Update 0x1827d8:78-85 — the fold marks 4 without queueing]. So on the game,
  arriving at 0700000a/07000016 cold does NOT re-announce the carried parts.
- The load paths that hit the fabricator: every load without a mission snapshot — the pause-menu
  Restart, the driving return, boot level-select (the whole pre-map-tower2 campaign flow). The
  warm exit's RamSave rides `with_mission` and keeps the real states.

## The fix

`crates/nf-game/src/mission.rs`, the fabricator in `Mission::init` (the `level > i` arm): a
prior-part record whose condition **already holds** (its channel is ON at fabricate time) is
fabricated **4 (complete)**, matching the snapshot's shape; unfired records stay 3 (announced);
fail records stay 2 either way — the channel-ON fail case re-arms through the state-2 arm and the
standing hooks, exactly like the game's own record [M 0x1827d8:68-77, 98-117]. No message is
queued for a fabricated 4 — the fold already happened on the earlier level [M 0x1827d8:78-85].

## Verification

- `crates/nf-game/src/mission.rs` unit test
  `prior_parts_fabricate_complete_when_their_condition_already_holds`: the fabricator's shape
  matches the game `.data` at a mid-mission arrival — the fired ch-0x47 record 4, the unfired
  ch-0x46 record 3, the fail record 2 [L ee_start_0700000a.bin @ 0x2a3f78]; plus 07000016's
  cold-load shape (the carried 07000014/15 part-0 channels all sit 4, the unfired 0x8e record
  waits announced at 3, the current part waits on its enable channel at 1).
- `crates/nf-view/tests/spurious_complete_16_live.rs` (NF_ISO, both green):
  - `the_arrival_posts_one_objective_complete_and_no_mission_banner` — the 07000016 arrival with
    the carried 07000014/15 chain posts **exactly ONE** "OBJECTIVE COMPLETE" (record 21's fold
    when the spawn plate #252 fires ch 0x8e) and **no type-3 "Mission Complete" banner** in the
    3400-tick walk. FAIL-FIRST: the old fabricator re-completed the five carried records on the
    first monitor tick and the burst asserts failed against it.
  - `the_real_end_condition_posts_the_mission_complete_banner` — the real end still posts it: the
    launch chain (ch 0x81 [D-driven]) spawns the chaser, her defeat raises ch 0x90 [S kiko.rs] →
    AND #271 → ch 100 [D #271] → MissionState 6 [M 0x182c60:85-116], the type-3 "Mission
    Complete" banner posts [M 0x182c60:85-122] bound to the game's 0x0200_0006 text, the driving
    hand-off `to_end = 0x07300017` [M 0x182c60:111-112], and **no mission banner before the end
    condition**.
- Full suite on the final tree (merged with origin/master @ de6110cc), fast runner
  `python3 tools/test/suite.py`: **(total filled in below)**.

## Suite total

See "SUITE (spurious-complete-16 final tree)" in the WORKLOG entry; the run's final line and any
FAILED lines are recorded there.

## What's left

Nothing for this task. The sister question — whether the real game ALSO hides the carried parts'
announce row on a cold arrival (the unfired ch-0x46-style record shows "announced" without the
message ever having been seen on-screen) — is exactly the snapshot's shape [L 0x2a3f78], so no
change was made there.

## Live checks for the main agent

1. **Cold-arrival burst is gone**: boot level-select into 07000016 (or pause-menu Restart from
   mid-16, both no-snapshot paths) and watch the arrival. Expected: exactly ONE "OBJECTIVE
   COMPLETE" toast in the first ~60 s (the spawn plate's record 21 / ch 0x8e fold), NOT the
   stacked burst of ~5–6 toasts; no "Mission Complete" banner.
2. **Mid-mission-16 arrival shape**: with the carried chain ON (channels 0x47/0x48/0x4a/0x54/0xce
   set by 14/15 play), the mission list at the 07000016 arrival should show the five part-0
   objectives as already-complete (state 4, no toast), "Locate the launch silo" style record 21
   completing off the spawn plate, and "Reach Drake's launch silo" as the live announce.
3. **The real end**: at the silo, let the chaser spawn (ch 0x81 chain) and kill her — expected:
   ch 0x90 → "Mission Complete" banner (text 0x0200_0006) → hand-off to the next level
   (0x07300017). The banner must NOT appear before her death.
4. **Warm path unchanged**: die → reload from a mid-16 snapshot (with_mission path): no carried
   toasts, no banner (this path never touched the fabricator; confirm no regression).
