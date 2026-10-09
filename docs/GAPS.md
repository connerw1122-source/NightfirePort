# Gaps (unfinished work)

| ID | Item | Notes |
|---|---|---|
| G-001 | FILES.BIN reader from `FileList` (0x2446a0, 118 × 32) with a golden test, and a cross-check against an independent extractor | NightfireResearch's extractor exists but has no licence (facts only); running it needs the user's OK (download). |
| G-002 | BIGF cross-check against a community EA BIG tool | Needs a download; ask the user. |
| G-003 | Confirm docs/systems/input.md with PCSX2 traces (PINE): pad buffer layout, action values, ramps, `PS2FramesToSkip` | U-010, U-013. |
| G-004 | Driving input path (DRIVING.ELF `PS2PadDevice`, `InputDevice::DeadZoneChop/Scale` names from DRIVING.SYM are hints only) | U-003. |
| G-005 | Generate the full style × action mapping table for all 8 controller styles mechanically from `psiInput_MapInputs` | |
| G-006 | `Check_AutoAim`, lock-on (`Autoaim_LockOnMul`), difficulty multipliers, `Player_ClearAimInertia` | input.md §7. |
| G-007 | Add the OpenGOAL ISC notice for `vag.rs` | Fetch the LICENSE text (read-only web fetch). |
| G-008 | `pss.rs` and `vag.rs` are RC2-verified only: re-check on `MOVIES/30_FPS/*.PSS` | opt-in NF_ISO test. |
| G-009 | Wire `bethington/ghidra-mcp` to `work/ghidra/NightfirePort.gpr` (the registered MCP points at RC2's project on port 8089) | Needs a second headless server on another port, or a switch; ask before changing MCP config. |
