## Your task: `cross-process-determinism` — the sim must give the same result in every process

664 (guard-reaction-delay) found that `fix_guard_weapons_live::the_3x3_hit_rate_and_damage_matrix_matches_the_game`
measures DIFFERENT streams in different processes of the same binary: the same cell gave 125/200 in a suite run and
153/200 standalone, with the same inputs. Something in the sim depends on per-process state. The usual suspects:
- std `HashMap`/`HashSet` iteration order (RandomState is seeded per process) feeding sim state or RNG draw order;
- pointer/address-based ordering;
- an uninitialised or thread-local value;
- the wall clock;
- the async loader's timing leaking into the sim.

This also breaks online lockstep (two machines are two processes).
1. Reproduce: run that test binary 10 times standalone and collect the per-cell numbers.
2. Find the source: audit every HashMap/HashSet iteration that feeds sim state, RNG draws or AI decisions, in
   nf-game and nf-view's Sim glue. Replace them with BTreeMap/IndexMap or sorted iteration where order matters.
3. Add a guard test: run a 600-tick 07000005 scenario with combat in two child processes (or twice with different
   HashMap seeds via a test-only hasher switch) and require bit-identical per-tick state hashes (mp/hash.rs has a
   hasher).

Sister: a deliberately order-dependent toy map trips the guard. Report every order-dependent site found.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/cross-process-determinism`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox5 at 2026-10-16T11:51Z
