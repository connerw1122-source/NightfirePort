Read `docs/playtests/2026-10-11.md` first: it's the user's multi-level playtest, and the item ids below refer to it.
10. **Push your report with your branch.** Write the plain-language report (the same text you give the user) to `docs/reports/<your task id>.md` and commit it as the last commit on your branch. The main agent reads it from there, so the user doesn't have to paste it. Never put credentials or URLs in it.
11. **Write everything in English:** code, comments, docs, the WORKLOG, commit messages and your report to the user.

## Your task: `fix-objects2`, 07000008 (the gondola level) objects

User playtest after 6faffb4, on 07000008:
1. **Invisible wall:** after the opening cut-scene, jumping down and going down the stairs is blocked by an invisible wall. The user has reported it several times. Find the collision with no drawn mesh there (log the blocking triangle's object and placement on contact, the way `NF_COLL` does), and port what it really is: a door, a barrier opened by a channel, or an object the cut-scene should remove.
2. **The safe and the gondola:** inside, there's a safe Bond opens with the laser watch, and from there he can activate the gondola so it runs. Find the safe and gondola objects (their placement types, scripts and channels), the laser-watch interaction (it depends on the gadget port; if that isn't ported yet, port the object side so a channel can drive it, and document the gadget hook), and the gondola's movement (a script-driven moving platform: collision and riding, like the 07000005 truck).

Files: object modules under `crates/nf-game/src/`, plus trigger and script hooks.

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

**Branch:** `s/fix-objects2`.

---
claimed-by: sandbox1 at 2026-10-12T12:19Z
