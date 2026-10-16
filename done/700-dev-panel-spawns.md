## Your task: `dev-panel-spawns` — the dev panel spawns what it says

Playtests 1012-2011 and 1011-1934: "the dev panel needs work on what it actually spawns. a frag grenade spawns what looks
to be a stun grenade that explodes like a frag grenade. I cant tell whats what. no clue what the rocket launcher is",
and "a lot of the spawnable stuff is gadgets". Audit the dev panel's weapon/gadget/grenade/ammo lists (crates/nf-view/
src/dev.rs) against the game's weapon table [L weapon_data 0x2bf150 + the label ids]:
- each entry's id, its displayed name (the game's own label), its projectile mesh and its class (weapon/gadget/grenade).

Fix mislabelled or misgrouped entries, and make each give use the same path the level pickups use. Tests: for every
panel entry, the given weapon id's record name, class and projectile mesh match the table; the frag gives 0x34 with
the frag mesh, the stun 0x35, etc.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/dev-panel-spawns`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox3 at 2026-10-16T15:23Z

---
## Report (done, 2026-10-16T17:34Z)

