**Reproduce first, on the newest origin/master, through the REAL path.** Earlier fixes for some of these passed
their own headless tests and the user still hit the bug in the game. Before changing code, write a live test that
drives the same path the player takes (the level loaded through `nf_view::load`, Bond moved by `step_bond` inputs —
walking, the use key, the ladder input — the real scripts and triggers running, no teleports past the problem
spot unless the report says so) and FAILS on current master the way the user describes. Put the failing output in
the report. Only then fix it, and the same test must pass. If you cannot reproduce it, say exactly what you tried
and where the path diverges from the report; do not mark it done on a guess.

## Your task: `guard-first-shot-timeline`, how long a guard takes from noticing Bond to his first shot

The user, repeatedly (playtests 1016): "guards still shoot way too quickly" and "literally cant even stun the
starting guard without him shooting me instantly before he can even turn". Earlier fixes (reaction delay, the
stunner hearing) did not end the reports.

Build the full decompile timeline for one guard, the first guard of 07000001 (and one 06 guard), from the tick
he first registers Bond (sight or hearing) to the tick his first bullet spawns: every state he passes through
(the alert/notice state, the turn to face, the weapon raise/draw, the aim, the fire gate), each one's duration
or exit condition, with [M] addresses. Then measure the port's timeline through the real path (Bond walking
into view; Bond firing the stunner from behind) and put both side by side in the report. Fix each step where
the port is shorter, with a test per step. Do NOT change the cover-under-fire timing pinned by
`cover_nodes_live` (it is measured). If the decompile says the port already matches, say so and list what
you measured.

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

**Branch:** `s/guard-first-shot-timeline`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
