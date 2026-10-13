## Your task: `swim-vent5`, the third-icon scope (E at the 05 gate door creeps instead of opening it)

Read `docs/reports/fix-swim-vent4.md` on `s/fix-swim-vent4` (c463614). Its two fixes are right (the AllyLead RUN anim,
[M 0x15e698 case 3]; the 0c route from disc evidence). The main agent merged it and the full suite went **764 passed,
4 failed**, all the same symptom: E at 07000005's castle gate door (the 0x1D LOAD_LEVEL trigger at (30.0, 38.84, 38.2))
no longer loads 07000006: `level_end_live::route_b_on_foot_gate_door_loads_level_on_e`,
`level_carry_live::{castle_door_route_spawns_at_the_matched_entry, use_key_still_activates_gated_triggers_and_load_level}`,
`walk_07000005_live::cold_start_reaches_the_castle_door_and_exits_to_the_castle`.

**Diagnosis (main agent):** the use key enters the creep arm and Bond starts crawling (substate 7) on a creepwall
~31 units away (squared distance 971 < 1000) instead of activating the door.
1. The port's creep arm (scene.rs, two sites: the use-key block and the second activation site) was gated on
   `icon_ctx == 6 || best.is_some()`. The game gates on the icon context ONLY: `if (BL+0x95f == 6)` [M
   `Player_Activate` 0x1a5170:66]. Remove the proximity fallback at both sites.
2. Even with that, the port computes `icon_ctx == 6` at the gate door. **Live [L, PCSX2 2026-10-13]: the real game's
   BL+0x95F is 0xFF (no icon) at the gate door (30.0, 38.84, 37.70) and at the level start (104.95, 25.12, 108.83)**,
   held there 40 frames each. So the port's third-icon scope is too broad: `scene.rs`'s loop sets the context from
   any enabled icon whose cel equals Bond's (`same_cel`), and the 05 courtyard is one big cel. Port the game's real
   scope from `ThirdIcon_Update` 0x196e80 and `Player_CollisionHandler` 0x1949a8:22 (the icon's room object list /
   straddle cels / its own radius or volume, and the enable channel), so the context is 0xFF at those two spots and 6
   only where the game shows the shimmy/crawl icon. Pin both spots in a test (the game's values), and keep the vent
   crawl tests (`fix_swim_vent_live`) green with the game's gate.
3. Then the four tests above pass unchanged. Run the full four-crate suite (in the background, first) and report the count.

Files: as fix-swim-vent4, plus the icon loop in `crates/nf-view/src/scene.rs`.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail. Report the count.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/swim-vent5` (start from `s/fix-swim-vent4`, then merge `origin/master` into it).

---
claimed-by: sandbox2 at 2026-10-13T19:00Z

---
## Report (done, 2026-10-13T19:57Z)

