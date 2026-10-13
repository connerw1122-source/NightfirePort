Read `docs/playtests/2026-10-11.md` first: it's the user's multi-level playtest, and the item ids below refer to it.
10. **Push your report with your branch.** Write the plain-language report (the same text you give the user) to `docs/reports/<your task id>.md` and commit it as the last commit on your branch. The main agent reads it from there, so the user doesn't have to paste it. Never put credentials or URLs in it.
11. **Write everything in English:** code, comments, docs, the WORKLOG, commit messages and your report to the user.

## Your task: `fix-swim-vent`, swimming from the bank, and the vent crawl (left over from `fix-ladder2`)

Read `fix-ladder2`'s report (`docs/reports/fix-ladder2.md`) and its WORKLOG entry first.
1. **Swimming from a bank:** on 07000002, Bond walks on the water instead of entering it. `fix-ladder2` recorded a repro, an `#[ignore]`d test asserting today's broken behaviour. Find why the water cel's surface and the floor collision under it keep Bond on top (the water surface `cel+0x90`, whether the water floor triangles are skipped or ignored by the game while swimming, `Player_InWater`), port it, and un-ignore the test with the correct expectation.
2. **The vent crawl:** there's no grate class. The crawlable vent is a **creepwall** (placement 0x3F, class 0x2B: use → crawl substate 7, with its music event and channel writes), and the grate panel is an ordinary door. Port the creepwall object and the crawl substate (enter on use, crawl movement and camera, exit), and the door-grate link if any.
3. The alarm music's last step: `fix-ladder2` traced `DroneFunc_CheckAlarmRaised` → `Music_Event(2,1)` → the per-level music updater, and left the final change in `music.rs`. Make it.

Files: `crates/nf-game/src/{player,ladder,music}.rs`, a new creepwall module, and the scene wiring.

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

**Branch:** `s/fix-swim-vent`.

---
claimed-by: sandbox3 at 2026-10-13T12:03Z
