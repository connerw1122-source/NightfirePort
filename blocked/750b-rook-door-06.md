**Reproduce first, on the newest origin/master, through the REAL path.** Earlier fixes for some of these passed
their own headless tests and the user still hit the bug in the game. Before changing code, write a live test that
drives the same path the player takes (the level loaded through `nf_view::load`, Bond moved by `step_bond` inputs —
walking, the use key, the ladder input — the real scripts and triggers running, no teleports past the problem
spot unless the report says so) and FAILS on current master the way the user describes. Put the failing output in
the report. Only then fix it, and the same test must pass. If you cannot reproduce it, say exactly what you tried
and where the path diverges from the report; do not mark it done on a guess.

## Your task: `rook-door-06`, Rook gets stuck in a doorway on 07000006

The user reported it on 2026-10-15 (playtest lag05d: "rook gets stuck on a door") and again 2026-10-16: "rook gets
stuck in the doorway on 06. again". 663 npc-doors (merged 10-16 03:40, "the route follow never stepped a node back")
did not fix it. Find Rook on 07000006 (the ally who leads Bond; `docs/levels/07000006.md`), run the level's real
path until Rook walks his route, and reproduce the stall at the doorway (log his position/state/route node per tick).
Port what the game does at that door (the door object's open state vs the drone's route/door wait states, the
NPC door logic in the decompile — NDrone2 door/wait states, the AI-net door links) with [M] citations. The test:
Rook passes the doorway and reaches his next scripted point within the game's time.

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

**Branch:** `s/rook-door-06`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox7 at 2026-10-17T01:32Z

---
## Report (blocked, 2026-10-17T02:38Z)

