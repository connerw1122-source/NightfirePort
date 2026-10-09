# Gaps (unfinished work)

| ID | Item | Notes |
|---|---|---|
| G-003 | Confirm docs/systems/input.md with PCSX2 traces (PINE): pad buffer layout, action values, ramps, `PS2FramesToSkip` | U-010, U-013. |
| G-004 | Driving input path (DRIVING.ELF `PS2PadDevice`, `InputDevice::DeadZoneChop/Scale` names from DRIVING.SYM are hints only) | U-003. |
| G-005 | Generate the full style × action mapping table for all 8 controller styles mechanically from `psiInput_MapInputs` | |
| G-006 | `Check_AutoAim`, lock-on (`Autoaim_LockOnMul`), difficulty multipliers, `Player_ClearAimInertia` | input.md §7. |
| G-007 | Add the OpenGOAL ISC notice for `vag.rs` | Fetch the LICENSE text (read-only web fetch). |
| G-008 | `pss.rs` and `vag.rs` are RC2-verified only: re-check on `MOVIES/30_FPS/*.PSS` | opt-in NF_ISO test. |
| G-011 | ghidra-mcp: `connect_instance` cannot select a second server over UDS on Windows ("no instance matching" while listing it), so ours runs on 8089 instead of RC2's. Report upstream or add a second bridge entry if both projects are needed at once. | WORKLOG 0004. |
