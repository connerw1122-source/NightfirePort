## Your task: `fix-finale-blockers`, the remaining mission blockers the map tasks found

The per-mission walkthroughs (032–037, merged) made most levels completable. These are the items that still stop a
mission. Read each level's `docs/levels/<code>.md` and its `walk_<code>_live.rs` first: they show exactly where the
walkthrough stops. Work through them in this order, and extend each level's walkthrough test to reach the end.

1. **07000016: Kiko's defeat (ch 0x90).** The final Omega Sector mission stops after objective 2. Port the
   KikoMission state family (0x15df28 / 0x15e0d8 and its neighbours in the state table at 0x29b300 [L]): her chase
   route to the silo mouth, the fail tripwire (#266, mask 0x10, ch 0x7b) and the defeat that writes ch 0x90.
   `038-fix-actors` (merged) owns the shared ally/boss framework (`ally.rs`, `docs/systems/actors.md`), so build on it.
2. **0700001b: the missile consoles.** "Disable the missiles" needs the console AND (#139: ch 21 & 22 & 27). Those
   come from class-0x38 `Monitor` script entities (`Script_CreateEntity` 0x1d8968) used with E (`Player_Activate`
   0x1a5170 case 0x38 → `Monitor_Activate`). Port the entity creation and the activation. Also: 11 of the level's
   74 ScriptPlayers wait on gate channels and 11 drones never enable within 1800 ticks (fix-levels-sweep's table).
   Find their writers.
3. **0700004a: the lift's lobby arrival (ch 0x4a).** The lift is a moving platform. Find what moves it and what
   writes ch 0x4a when it arrives (a SimpleScript / path object; `ss.rs` from map-tower2 has the SS playback), and
   make the descent work.
4. **The unported drone start states** fix-levels-sweep found: **0x85** (mode 7; 07000011/12/13/15), **0xa7**
   (07000004) and **0xb4** (07000007). Find each handler in the DSTATE dispatcher (`NDrone2_ControlSTANDARD`
   0x1490f0 and the per-dtype controls) and port them, or show they're idle-equivalent and map them.
5. **07000002/03's exit key gates (ch 0x4C / 0x52) and 07000008's boarding and ride (ch 0x4A / 0x4B):** writers not found
   by map-castle / map-mayhew. fix-levels-sweep's writer scan (`docs/systems/levels.md` §7) has the current state.
   Use it, and port what you can prove (likely dialogue events and the cable-car pack script's channel events).

Files: the drone AI modules (new files preferred: `kiko.rs`, `monitor.rs`), `crates/nf-game/src/nis.rs` for the
entity class, the `walk_*_live.rs` tests of the levels you unblock, and `docs/levels/`. Shared files (`drone.rs`,
`scene.rs`, `triggers.rs`): small additive hooks only, listed in your report.

**Reproduce before you fix.** The user's play is the reference, and previous fixes passed tests but still failed in play. For each bug:
1. Write a headless NF_ISO test that drives the real input path (the `nf-view` input mapping into `Sim`, or `nf_view::load` plus `dev::scenario`), and show it reproducing the user's symptom.
2. Then find the cause in the decompile and fix it.
If you can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Update the doc entry** to say it's fixed, with your commit.

**Branch:** `s/fix-finale-blockers`.

---
claimed-by: sandbox2 at 2026-10-12T13:05Z

---
## Report (done, 2026-10-12T16:36Z)


### Result (one line)
The finale missions complete: 07000016's chaser defeat (kiko.rs; the ch-0x90 writer is a documented stand-in), 0700001b's Drake defeat (ch 0x0e → the defeat NIS writes ch 0x0b + ch 100 — no stand-in), 0700004a's lift ride (the negative-delay Script_Update fix; the ch-0x4a arrival is a documented stand-in), the 0x85/0xa7/0xb4 start states ported or proven idle-equivalent, Death_Anim raises the death channel per 0x1637f8 case 2, and the 02/03/08 gate writers are bounded by the full switch_channels-writer enumeration. Tests: fix_finale_live (4, each failing pre-fix), full suite green except the 3 allowed disc_golden.
