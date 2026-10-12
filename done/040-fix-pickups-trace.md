Read `docs/playtests/2026-10-11.md` first: it's the user's multi-level playtest, and the item ids below refer to it.
10. **Push your report with your branch.** Write the plain-language report (the same text you give the user) to `docs/reports/<your task id>.md` and commit it as the last commit on your branch. The main agent reads it from there, so the user doesn't have to paste it. Never put credentials or URLs in it.
11. **Write everything in English:** code, comments, docs, the WORKLOG, commit messages and your report to the user.

## Your task: `fix-pickups-trace`, the Raptor drops, against the real game's trace

Read `tests/traces/01-raptor.trace` and the "PCSX2 trace check" section at the end of `docs/atlas/pickup.md`. The real game:
- the first Raptor drop gives **weapon 0x0E** with clip 9 (not weapon 6, as the port calls it);
- every later drop gives ammo type 3 (+6, +7, +7, +5 rounds), and **every drop the user walked over was taken**.

Do this:
1. Find why the game equips 0x0E for a guard's Raptor (the dropped pickup's id, the guard's weapon id, `Upgrade_Weapon` 0x1a8f30, the base-weapon mapping `weapon_data+2`, and the ammo type `+0x90`), and make the port match: the same weapon id, clip, ammo type and HUD.
2. Reproduce the user's port bug (some later drops can't be picked up) with an NF_ISO test through the real path: kill Raptor guards on 07000001 and walk over each drop. Assert that every drop is taken and the inventory changes match the trace (weapon 0x0E with clip 9 first, then ammo type 3 increments within the game's round range).
3. Check the drop rounds formula against the trace's +6/+7/+7/+5: clip·(FRand(0.5) + factor), with the factor and the RNG draw in game order.

Files: `crates/nf-game/src/{pickup,gun}.rs` and the drop path in `drone.rs`.

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

**Branch:** `s/fix-pickups-trace`.

---
claimed-by: sandbox2 at 2026-10-12T11:31Z

---
## Report (done, 2026-10-12T13:12Z)

