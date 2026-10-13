## Your task: `drive-elf-map`, driving phase 3: the DRIVING.ELF map

Read `docs/atlas/driving.md` first; this is §6 phase 3 (research). DRIVING.ELF has 9,170 decompiled functions
(`work/decomp/DRIVING.ELF`) and a symbol hint table (`DRIVING.SYM`, 12,719 entries, already parsed).
1. The main loop from `FUN_00174bf0` 0x174bf0: the per-frame order (input, physics, AI, camera, render, audio) and the
   timestep.
2. A function inventory by subsystem (vehicle physics `CARP`, actors `Act*`, AI, camera, HUD, audio, the loader, the
   transfer/return), with the SYM hint names matched to addresses where the shapes agree, tagged [S] until checked.
3. Shape-match the functions shared with ACTION.ELF (math, BIGF, sound) so the port can reuse its ported versions:
   a list with both addresses and the evidence.
4. Answer §7's open question 5 from the code: the `.crp` track's consumer (function + address).

Deliverable: `docs/atlas/driving-elf.md` and the phase 4 (vehicle physics) task description written at the end.

**This is a research task: docs and tests, no gameplay code.** The rules above still apply, plus:
- Every fact is tagged [M] code (function + address), [D] disc bytes, [L] live/trace, [S] suggested.
- Golden numbers only in tests (counts, SHA-1s, ids, never game bytes), each with a sister assertion that can fail.
- Run the crates your tests touch before pushing, and say which you ran.
- End the doc with the phased port plan and the live checks for the main agent.

**Branch:** `s/drive-elf-map`.
