Read `docs/playtests/2026-10-11.md` first: it's the user's multi-level playtest, and the item ids below refer to it.
10. **Push your report with your branch.** Write the plain-language report (the same text you give the user) to `docs/reports/<your task id>.md` and commit it as the last commit on your branch. The main agent reads it from there, so the user doesn't have to paste it. Never put credentials or URLs in it.
11. **Write everything in English:** code, comments, docs, the WORKLOG, commit messages and your report to the user.

## Your task: `fix-player4`, fists and melee

`fix-player3` gathered the evidence but ran out of time:
- the punch is a bullet with 1.75 range;
- the knock-out state is at 0x172a30;
- the port still does 5.0 damage with no knock-out.

1. Port non-lethal knock-out punches: the punch hit path for weapon 1, the drone KO state machine (fall, lie unconscious, and recover or not, as the game does), whether KO'd guards count as kills, and their drops.
2. Port the punch animation's left/right alternation, if the game has it (the fists' animation script).
3. Test through input: punch a guard N times and assert he enters the KO state and isn't dead.

Files: `crates/nf-game/src/{gun,player}.rs`, and the KO states in `drone.rs` (pull often; `fix-drone4` may be editing it).

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

**Branch:** `s/fix-player4`.

---
claimed-by: sandbox3 at 2026-10-12T04:08Z

---
## Report (done, 2026-10-12T05:31Z)

