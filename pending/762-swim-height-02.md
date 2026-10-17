**Reproduce first, on the newest origin/master, through the REAL path.** Earlier fixes for some of these passed
their own headless tests and the user still hit the bug in the game. Before changing code, write a live test that
drives the same path the player takes (the level loaded through `nf_view::load`, Bond moved by `step_bond` inputs —
walking, the use key, the ladder input — the real scripts and triggers running, no teleports past the problem
spot unless the report says so) and FAILS on current master the way the user describes. Put the failing output in
the report. Only then fix it, and the same test must pass. If you cannot reproduce it, say exactly what you tried
and where the path diverges from the report; do not mark it done on a guess.

## Your task: `swim-height-02`, the swimmer floats ~0.5 too high in the 02 pond

PCSX2 truth, 2026-10-16 [L, fresh 07000002 boot, Bond placed by PINE in the pond cel 0x1708870, stick
forward]: swimming into the hut pocket under the hut ladder (centre (-74.84, ·, 22.22)) the PS2 root settles
at y 3.17 at (-76.27, 3.17, 22.14); in open water near (-71.1, ·, 27.9) it sits at y 3.48. The port, swimming
the same way, rises to 3.70 in the pocket and 3.60 in open water; its log shows the swim capsule touching the
bed and pushing up (grounded true at tick 12 of the swim-in). The swim root height animobj+0xCC is the same in
both (PS2 1.0304, port 1.035), so the capsule size is not the cause.

Find where the port's swim tick differs from `Player_Collision` 0x194df0 (the swim path that jumps past the
gravity into the capsule switch, disasm 0x194f4c-0x194f70), the surface clamp in 0x193600:201-220, and the
swim buoyancy/sink in the swim substate update. Pin the PS2 heights in a real-path test (swim in from open
water through `step_bond`), within 0.1. Keep `ladder_underwater_02_live`, `ladder_02_live`,
`fix_swim_live` and `fix_ladder_water_live` green.

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

**Branch:** `s/swim-height-02`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
