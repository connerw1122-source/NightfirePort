## Your task: `uncited-gameplay-audit` — the ACTION.ELF gameplay functions nothing cites yet

A coverage pass (2026-10-16) counted 1979 of 3038 ACTION.ELF functions cited by the port's code/docs (80% of the code
bytes). The uncited gameplay groups include:
- 116 NDrone2 functions (DSTATE_PressAlarm + FindAlarmPoint, DSTATE_Investigate, the Interogate* states,
  DSTATE_AbseilHang, DSTATE_ElevatorJumper, DSTATE_CivilianChallenge/CivilianMissionWait, EvasiveMove +
  CanStrafe/CanRoll/CanStep, DoorGuardHandler, the Ninja* states, SetDronePartner);
- Script_RunFrame / Script_InterpolateSpline / Script_InterpolateLine / Script_CalculateSpline;
- camera_chasecam, camera_snake, camera_track_*; C_NIS_Handler;
- the MP objective helpers (MP_getObjExtObj, MP_objectBeingDeleted, MP_PostLoad_Init...);
- the BOTSTATE / BOTWEAP helpers;
- Radio_SendMessage / Radio_Init; Creature_*; TextBox_Update; the AINetwork route helpers.

For each group:
- (a) is it reachable on retail data — which levels/drones/scripts use it (cross-check with 709's census if it has
  landed, else your own data scan);
- (b) does the port have an equivalent under another name (then cite the address in the port so the coverage
  counts it);
- (c) if neither, what player-visible behaviour is missing.

Deliver docs/systems/uncited-audit.md ranked by player impact, with the citations added where the port already
covers the function, and ready-to-queue task descriptions for the top 10 gaps in the report.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/uncited-gameplay-audit`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox7 at 2026-10-16T17:39Z

---
## Report (done, 2026-10-16T18:39Z)

