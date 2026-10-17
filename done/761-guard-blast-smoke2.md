**Base:** merge `origin/master` first and check it contains `crates/nf-view/tests/briefing_plays_live.rs` (the
front-end audit + test fixes landed with it, 2026-10-16 evening). If it does not, fetch again before you start: the old
master still carried 28 tests that contradicted the decompile.

## Your task: `guard-blast-smoke2`, finish guard-blast-smoke (HELD at merge)

`s/guard-blast-smoke` (task 727) was held at the 2026-10-16 merge: on top of master it fails two existing tests:
- `nf-game` `guard_drops_flashbang_live::frag_at_the_same_spot_kills_the_trio` — a frag 1.9 u from placements
  3/4/302 on their level no longer kills them;
- `nf-view` `fix_actors_live::killing_a_protected_guest_fails_the_mission_but_scaring_him_does_not` — a grenade
  at the protected guest no longer kills him (line 80 area).
Its `on_explosion` drops msg 9 for any state `impact_args` does not list ("a state with no case 9 takes nothing").
The GLOBAL base `NDrone2_DSTATE_Global` 0x15b830 does return 0 for msg 9, so the rule may be right — but then the
question is which DSTATEs really have a case 9: list, for the states those guards/the guest are in when the blast
lands (idle, patrol, civilian, guest/party states, the flashbang family...), the game's own handler and whether its
switch has case 9 (and what it calls). Port every missing case 9 from the decompile; if a state truly has none,
show the address and fix the TEST with the evidence instead. Start from `origin/s/guard-blast-smoke` merged with
the newest origin/master. Also keep `astronaut_guards_live` green (the astronaut's kill rides msg 8 →
the death selector, see spawn_blast's comment on master) and the protected guest's ch-0x60 raise [M 0x145bd0].
Run the two tests above plus blast_smoke_live, astronaut_guards_live, ai_live before the full suite.

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Tests pin only proven behaviour.** Every assertion about what the game does cites its evidence: [M] a decompile
address that actually says it, [D] disc bytes, or [L] a live PCSX2 measurement from a FRESH boot (a save-state resume
skips the boot path). A guess, a port choice or an [S] item is never a hard assertion: mark it `// [S] unverified` and
keep it out of pass/fail. If an existing test contradicts the decompile, the TEST is wrong: fix it, with the address, and
say so in the report. Never weaken a decompile-backed test to make code pass.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/guard-blast-smoke2`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox1 at 2026-10-17T07:39Z

---
## Report (done, 2026-10-17T09:41Z)

