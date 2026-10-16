## Your task: `faster-live-tests` — cut the slow live test binaries' time without losing coverage

The main agent's full suite (371 binaries, 3 at once, `--test-threads=1` each) takes about 20 minutes. Most of that
is about 2880 binary-seconds of test time (suite log 2026-10-15). The six slowest binaries are a third of it:
- weapon_matrix_live 307 s
- levels_sweep_live 184 s
- fix_guard_weapons_live 157 s
- walkreal_07000001_live 132 s
- models_misc_live 100 s
- campaign_flow_live 85 s

Next come fix_striped_box_census 73 s, fix_party_mission_live 61 s and portal_sweep_live 40 s.

For each one, profile where the time goes: repeated `nf_view::load` / `load_dev` of the same archive per test, the
weapon-asset merges, redundant tick loops, debug-only work. Then make it faster while keeping every assertion and
its sister assertions:
- share a loaded level across the tests of one binary (a `OnceLock` cache of the parsed archive / a cheap scene
  clone, if the tests mutate it);
- cut tick counts only where the assertion provably doesn't need them (say why in the report);
- split a giant test into independent ones only if it lets them share setup.
Never weaken or remove an assertion. Tests run with `--test-threads=1`, so a shared cache must not leak state
between tests (clone per test).

Also delete `crates/nf-view/tests/zz_obj_drives_probe.rs` (a probe committed by f71a2925 "campaign runner"). If it
pins something real, move that assertion into a proper test first.

Report a before/after table of per-binary seconds (release, `--test-threads=1`, NF_ISO set) and the total.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/faster-live-tests`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
