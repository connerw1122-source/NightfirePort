## Your task: `grenade-rattle2` — the grenade bounce work, without the broken bounce-cap test

Your earlier branch `s/grenade-rattle` (task 693) was HELD at merge (2026-10-16). Start from
`origin/s/grenade-rattle` merged with the current `origin/master`.

With grenade-rattle alone on master, its own unit test fails:

    nf_game::grenade::tests::grenade_sits_after_max_bounces_and_explodes_only_at_the_fuse
    grenade.rs:556 — the speed zeroes at the bounce max [M 0x12ce80]
      left: [0.0, -30.002722, 0.0]   right: [0.0, 0.0, 0.0]

In the merge batch, two more failed:
- `fix_grenade_types_live::the_stun_grenade_whites_bond_out_at_the_games_time_and_alpha`
- `fix_weapon_fire_modes_live::grenade_events_match_the_records`

Decide from the disassembly which is right: at the new 20-bounce cap, does the game zero the speed (the old test),
or keep falling (your change)? [M 0x12ce80 and the park order]. Then:
- fix the code, or update the test with the [M] lines that justify it;
- keep the stun timing and fire-mode event tests green.

Run the FULL suite before you finish, and list every failure.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/grenade-rattle2`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox4 at 2026-10-16T15:13Z
