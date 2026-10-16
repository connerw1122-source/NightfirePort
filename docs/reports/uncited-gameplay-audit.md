# Report: uncited-gameplay-audit (task 710, 2026-10-16)

Branch `s/uncited-gameplay-audit` (cut from master c10b530f; origin/master has not moved since —
re-verified at 18:45 UTC, so the branch already is master + this task's commits). Deliverable:
`docs/systems/uncited-audit.md` (ranked by player impact, top-10 ready-to-queue task
descriptions, live checks for the main agent).

## What's done

1. **Inventory.** Re-derived the coverage pass: 3038 ACTION.ELF functions from the decomp
   filenames; a function counts as cited when its base name or address appears in
   `crates/`/`docs/`/`tools/`/`tests/`. Before: 1984 cited / 1054 uncited (65%) — within noise
   of the pass's 1979. Found and documented two inventory false positives (`TextBox_Update`,
   `C_NIS_Handler` were "cited" only by WORKLOG mentions).
2. **Citations added (commit b4df396a, comment-only).** Where the port already implements the
   game's logic under another name, the full game symbol + address is now cited at the
   equivalent: ~79 functions newly counted → **2063 cited / 975 uncited (68%)**. Biggest
   clusters: the NDrone2 nav/vision/combat helpers, the AINetwork route/emitter family, the
   MP_get*Obj getter family, the Radio widget family, TextBox_Update, DroneData anim tables.
3. **The audit** (`docs/systems/uncited-audit.md`): per-group (a) retail reachability with
   caller-chain evidence from the decomp, (b) port equivalent or none, (c) missing behaviour;
   10 ready-to-queue tasks ranked by player impact; runners-up; a dormant/dead list
   (camera_fixed/snake/chasecam are unreachable dead code — no SetCamMode writer for modes
   4/5/6/0xa; Creature_* ships on no audited level; DSTATE_Investigate 0x7E has no entry);
   5 live checks for the main agent.
4. Top gaps, one line each (full descriptions in the audit doc):
   ① AI alarm-raising (FindAlarmPoint 0x153008 → DSTATE_PressAlarm 0x168ea8 — guards never
   press alarm switches); ② blast stagger + smoke states for guards (0x54, 0x7B-0x7D —
   grenades/smoke do nothing to guards today); ③ Ninja melee chain 0xab-0xb3; ④ live MP host
   never runs the match logic (update_objects/player_killed/respawn are test-only); ⑤ bot
   objective-goal scan (BOTSTATE_pickGoal 0x126b88 slot 1); ⑥ bot combat moves + weapon
   switching (Rand_Rand(5) stream divergence noted); ⑦ astronaut zero-G guards for 0700001b;
   ⑧ frontend menu-def keyframe runtime (the uncited Script_* family — all menu animation);
   ⑨ guard barks + FirstAlertState 0x39; ⑩ NIS event 9 cam2mode + the C_NIS_Handler
   mission-driven cut-scene route.

## How it was verified

- Claims from group research were decomp-grounded: every cited equivalent carries the game
  address, and the key state-machine claims were spot-checked against the decompiled C
  (e.g. PressAlarm: CallAnim 0x21/0x22, timer 0x3c → follow-on 0x8b, +0x594=0x32,
  SetMissionFailReason 0x10 in `00168ea8_NDrone2_DSTATE_PressAlarm`; FindAlarmPoint:
  ClosestAIPoint type 8 in `00153008_NDrone2_FindAlarmPoint`).
- `cargo build --offline --release -p nf-game -p nf-view -p nf-formats` — clean (pre-existing
  warnings only). The branch's code diff is comment-only, so no behaviour change is possible.
- **Full suite** (fast runner, `tools/test/suite.py`, started 17:52 UTC on this branch):
  `1894 passed, 6 failed, 72 ignored (47 min)`. The 6 failures are the allowed 3 `disc_golden`
  tests (slim ISO) + the 3 pre-existing pylon render probes (`fix_0c_pylon_live` ×2,
  `probe_pylon_render` ×1) — identical to the master baseline recorded in the WORKLOG.
- A second suite run was started 18:42 UTC on the final tree (`suite2.out`); the only delta
  from the tested tree is this report + the WORKLOG close-out (markdown, no binary inputs).
  If its total differs it is appended below before the finish step.
- Final tree vs origin/master: no merge needed — master was unchanged at branch cut and at
  finish time (`git log HEAD..origin/master` empty).

## What's left

- The 10 queued tasks above (descriptions with acceptance criteria ready in
  `docs/systems/uncited-audit.md`). None started — this task was audit-only by design.
- The 5 live checks for the main agent (OLD-chain behaviour-id census, interrogate
  reachability on 07000007, the C_NIS_Handler/event-9 disc census, an alarm-switch watch for
  the PressAlarm acceptance test, an astronaut render spot). They gate two of the top-10
  priorities.
- Task 704's `s/mp-bots-in-match` branch (blocked on its window) overlaps gap ④/⑤; rebase
  those onto it rather than redoing the wiring.
