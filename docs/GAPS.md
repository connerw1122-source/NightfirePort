# Gaps (unfinished work)

| ID | Item | Notes |
|---|---|---|
| G-003 | Remaining live checks for input.md: aim/turn ramps (`Player_Aiming` state +0x8f4/+0x8f8/+0x900), auto-aim, other controller styles | Pad layout, action mapping, curve and pacing done (WORKLOG 0006). |
| G-012 | Verify the §2 dead zone live: the game overwrites the raw analog bytes in place, so read them between `scePadRead` and the dead-zone loop (PCSX2 breakpoint) or derive from PCSX2's own input display | |
| G-013 | The mcp-pine bridge was stopped (it holds PCSX2's single PINE connection); restart Claude Code to get the PINE MCP tools back. `tools/trace/pine.py` replaces it for sampling. | WORKLOG 0006. |
| G-004 | Driving input path (DRIVING.ELF `PS2PadDevice`, `InputDevice::DeadZoneChop/Scale` names from DRIVING.SYM are hints only) | U-003. |
| G-005 | Generate the full style × action mapping table for all 8 controller styles mechanically from `psiInput_MapInputs` | |
| G-006 | `Check_AutoAim`, lock-on (`Autoaim_LockOnMul`), difficulty multipliers, `Player_ClearAimInertia` | input.md §7. |
| G-007 | Add the OpenGOAL ISC notice for `vag.rs` | Fetch the LICENSE text (read-only web fetch). |
| G-008 | `pss.rs` and `vag.rs` are RC2-verified only: re-check on `MOVIES/30_FPS/*.PSS` | opt-in NF_ISO test. |
| G-014 | Read the VU1 microcode (ACTION.ELF `.vutext` 0x241c80, 0x2880 bytes, + DVP overlays) to confirm the batch semantics: signed V4-8 = normal + no-kick flag, UV use (ST·Q?), entry points 0 (clipped) / 2 (unclipped), lighting. Needs a VU disassembler (Ghidra EE extension covers VU? check). | ps2gfx.md confidence "medium" for these. |
| G-015 | Skinned PS2_GFX objects (582 leaves; G's skin info ≠ 0): REF tags with UNPACK codes whose data the skinning code (`CacheSkin` 0x1112c0, `SkinIt` 0x10bf30) provides at run time. | |
| G-016 | World assembly: place objects using map static data (block 0x1A, `parsemap_create_dynamic_objects`), cels, portals (0x21). Needed for a level viewer. | |
| G-011 | ghidra-mcp: `connect_instance` cannot select a second server over UDS on Windows ("no instance matching" while listing it), so ours runs on 8089 instead of RC2's. Report upstream or add a second bridge entry if both projects are needed at once. | WORKLOG 0004. |
