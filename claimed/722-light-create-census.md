## Your task: `light-create-census` — every Light_Create site, ported or not

The rocket comparison showed missing dynamic lights (the per-tick bullet light was never ported; it is now). Make the
full census of `Light_Create` 0x1cc3d8 call sites in ACTION.ELF. For each site list:
- the arguments (radius, intensity, colour source, life, kind, flicker);
- the condition;
- whether the port creates the same light, with a file:line.

Known sites: Bullet_DoTrails 0x12d890, Bullet_update 0x12e7b0 (ported), GunImp_Update 0x1320d8,
DroneWeap_FireWeapon 0x178f68, HUD_UpdateNightSightPane 0x19d218, Player_MuzzleFlash 0x1a4298, Player_WeaponFiring
0x1a5460, Sub_Create 0x1ad6c0, AnimProcessScriptCmds 0x1c1b38, Searchlight_Create, parsemap_create_dynamic_objects,
Script_LightStart.

Port every missing light on the SIM side (the `LightList` entries, with the game's RNG draw). The renderer already
consumes the list. Pay particular attention to explosions: the PS2's rocket explosion lights the end of the 08
corridor ~2.5x brighter than the port's.

Test per newly ported site: its light appears with the game's arguments on the trigger tick.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/light-create-census`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox3 at 2026-10-16T19:53Z
