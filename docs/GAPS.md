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
| G-015 | ~~Skinned PS2_GFX objects~~ decoded in bind pose (WORKLOG 0013, docs/formats/character.md). | done |
| G-020 | Character posing and animation: skeleton bone data, the skin's per-bone matrices, `SkinIt`'s VU0 microprograms (bone weights in source qw1), sequences (types 4/5) and scripts (7). | Characters are T-pose. |
| G-021 | Drone skins missing from their archive (33 placements): load the common animation file (`CommonAnimFile`, LoaderLoad type 4) / other archives; sleeve substitutes (`AnimSleeveGetEntity`). | |
| G-016 | World assembly: place objects using map static data (block 0x1A, `parsemap_create_dynamic_objects`), cels, portals (0x21). Needed for a level viewer. | |
| G-017 | Port `PS2Sinf3` (the game's sine approximation) so object matrices are bit-exact. | placement.md. |
| G-018 | nf-view: per-object GS state (alpha blend/test, CLAMP, filtering from the first CNT's `DIRECT` A+D writes), animated textures, skinned meshes, sky, fog. Currently: alpha test at 0.5, repeat, bilinear. | U-021. |
| G-019 | Sky: done in nf-view (type-0x2A placements, slot order, viewer-centred by glist flag bit 2; WORKLOG 0014). Remaining: lightning variants (parameter 1, random flicker), the rotating sky of 0x0700001B, GS blend/fog state of the sky pass. | |
| G-011 | ghidra-mcp: `connect_instance` cannot select a second server over UDS on Windows ("no instance matching" while listing it), so ours runs on 8089 instead of RC2's. Report upstream or add a second bridge entry if both projects are needed at once. | WORKLOG 0004. |
