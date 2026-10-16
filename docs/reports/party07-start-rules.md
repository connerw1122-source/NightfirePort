# party07-start-rules — 07000007's party rules from the first frame

Task: verify/fix the 07000007 (Drake's party) start rules against the original game — Bond holds
fists (not the gun) at the start, the guards are not hostile until the trigger, a punch on a guard
is an instant mission fail with the game's fail text, no guest ever fires, and the guests react to
gunfire the way the game does (they don't). Playtests 1011-1535, 1011-1616, 1012-1723.

## What was done

The four playtest symptoms were all start-state rules, and all four are now pinned by tests driven
through the real path (`nf_view::load` / `load_with_carry` + `Sim::step_bond`), from a COLD start
AND from the 06 → 07 carry (`RamSave::capture` → `load_with_carry`):

1. **"the guards immediately start shooting at me because I start with my gun drawn"** — the port
   already opened on the fists [M `Player_InitWeapon` 0x1a7ef8 case 0x7000007 → +0x63 = 1, armour 0]
   and the bouncers stay calm while Bond is unarmed [M CivilianGuard 0x1603a8 case 3: attack only
   when `NearArmedPerson` 0x1787c8 / `ObjectIsArmed`(Bond) 0x178768 + seen]. The tests now hold the
   port to that: after settle, `weapon == 1` (fists), armour 0, and no drone is in a combat state
   (0x56..=0x7e / 0x85..=0x99) while Bond is unarmed — cold AND carry, plus the spawn-plate
   position assert on the carry (#0 at 33.4, 44.5, −70.6).
2. **"If you punch a guard, it should be an automatic mission fail"** — punching a bouncer (fist
   swing fired through `fire_player`, SfxHearing 0x403 = the punch [M 0x13fff0 case 5 → msg 6])
   raises ch 99, sets FailLabel 0x4000033 [M `PunchImpact` 0x145f28:45-56 → reason 5 →
   `SetMissionFailReason` 0x13e560], reaches MissionState 2 [M 0x182c60:45-57], queues the game's
   "Mission Fail" HUD banner, and the level's lang table binds non-empty text to 0x4000033. Cold
   AND carry.
3. **guests T-posing / not reacting** — every guest on the party floor has a bound anim at the
   start (T-pose = `anim.script == None` in the port), and the guests' panic/cower path is the
   sight-side scare [M CivilianHiding / 0x160140:44-76], covered by fix_guest_reactions_live.
4. **"one male guest shooting"** — no guest holds a weapon class: dtype-10 guests take their weapon
   only from placement param 9, prop 0x1f = 0 (deaf to sound alerts, [M 0x148a30:45-49]) and
   prop 0x54 = 1 (protected) [M GetDTYPENEW 0x14c938]. The test fires 8 P2K shots among the guests
   (broad hearing radius): no guest panics, no guest gains a weapon, no guest ever attacks.

**The fix this task added: the tase fail (reason 7)** — the last unported party fail path.
`Drone::on_tase` now runs `DroneFunc_HandleImpact` case 7's mission-fail arm BEFORE the
accumulator [M 0x1469b8:58-68, dis 0x146a9c-0x146ad4 — `addiu a1,zero,7` precedes the accumulator
math; the decompiler had dropped the store]: on 07000007, tasing an UNARMED person
(`ObjectIsArmed` 0x178768 on the victim's class byte) → reason 7 → label 0x4000033 + ch 99
[M 0x13e560]. Game shape kept: the taser family's own case-7 arms bypass it (DSTATE_Taser
0x162b98 / DSTATE_Stunned 0x162eb0 don't call HandleImpact), while DSTATE_Stunned_Recover
0x1630b8 case 7 FORWARDS to `HandleImpact(msg, 0, 1)` — so the port's STUNNED_RECOVER arm calls
`on_tase(d)` and re-runs the check, exactly like the game.
Disassembly note: the level doc's old "bullet/impact → reason 7" was a mislabel — msg 7 is only
the Stunner dart (0x13fff0:269-274); a real bullet is msg 8 and has no fail hook in the game.

Docs refreshed: `docs/levels/07000007.md` §6 (every fail path ported; bullet non-hook noted) and
the §7 sister note; `docs/systems/mission.md` (SetMissionFailReason now wired; reasons 2 and 0xb
were already ported by fix-party-mission/fix-actors — only 7 was missing).

## Verification

`crates/nf-view/tests/party_start_rules_live.rs` — 6 tests, all green on the final merged tree:

- `cold_start_opens_on_fists_with_a_calm_animated_party` (rule 1 + 3, cold)
- `carry_from_06_opens_on_fists_at_the_party_start_plate` (rule 1 + 3, carry)
- `cold_punch_a_bouncer_fails_with_the_game_fail_text` (rule 2, cold)
- `carry_punch_a_bouncer_fails_the_mission` (rule 2, carry)
- `gunfire_does_not_panic_the_deaf_guests_and_no_guest_fires` (rule 4)
- `tasing_a_guest_fails_the_mission_at_once` (the reason-7 fix)

The touched crates were built and run before pushing: nf-game (the `on_tase` fix — its lib and
test binaries ran inside the suite) and nf-view (the tests, incl. this file: 6/6).

## What I ran

- `cargo test --offline --release -p nf-view --test party_start_rules_live -- --test-threads=1`:
  6 passed.
- The full four-crate suite (the `tools/test/suite.py` logic) on the final tree (this branch is
  exactly origin/master + this task's diff): **404 test binaries — 1784 passed, 7 failed,
  72 ignored**, each binary with `--test-threads=1` under the suite's 600 s per-binary limit.
  **This sandbox kills background jobs between tool calls**, so the suite ran in resumable
  foreground chunks with suite.py's exact per-binary logic (build once, then each binary
  separately). One duplicate target name (`fix_ai_remainder_live` exists in both nf-game and
  nf-view) was run explicitly in both copies (2 + 1 passed). The 7 failures are exactly the
  branch-point allowed set (same list as the 07:28 final-tree run of the merged master):
  `probe_pylon_render` × 1 and `fix_0c_pylon_live` × 2 (wgpu finds no GPU adapter in this
  sandbox), `driving_ai_follow::real_mis01_track_rngp_walks_when_nf_driving_is_set`,
  and the 3 slim-ISO `disc_golden` tests. No new failures against the branch point.

## Out of scope, noted

- DSTATE_Taser / StunDartImpact ENTER set fail reasons 0xd/0xe for victims with the byte +0xc5 ∈
  {0x11, 0x0c} [M 0x162b98 case 1 / 0x163430] — a separate mechanic (specific victim classes),
  not ported.
- The reason-7 arm's victims are the drones the game checks with `ObjectIsArmed` on their class
  byte; the port mirrors 0x178768's exact class ranges (`armed_class` in the test).

## Live checks for the main agent (optional visuals — the behavioral rules are headless-tested)

- `--bond 33.4,44.5,-70.6,<face the party floor>,0` right after the 07 load: Bond's hands are
  EMPTY (fists, no weapon model in hand), the guests on the floor are mid-idle/conversation anims
  (no T-pose), bouncers at their posts, nobody shooting.
- Same spot after walking up to a bouncer and punching: the "Mission Fail" banner is on screen
  (the 0x4000033 text) and the level ends to the fail wrapper 0x7000048.
- `--bond` near a guest while firing the P2K into the air: guests keep idling (no cower from
  sound alone — the cower is sight-side [M 0x160140:44-76]).
