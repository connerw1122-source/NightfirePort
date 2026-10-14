# The test suite runner (`tools/test/suite.py`)

Runs the full four-crate suite (nf-game, nf-formats, nf-audio, nf-view): builds every test binary once, then runs each
binary on its own (`--test-threads=1`) under a wall-clock limit (default 600 s). A hung test fails its binary with a
`TIMEOUT <binary> (last started: <test>)` line instead of stalling the run. Output: one `FAILED <binary>::<test>` /
`TIMEOUT` / `CRASH` line per problem, then the total `N passed, N failed, N ignored (M min)`. The full per-binary output
goes to the log (default `<repo>/../suite.log`).

It is faster than one `cargo test` over the four crates (13 min vs 30-40 min on the main agent's PC, 2026-10-13), and it
is what the main agent runs before every merge.

**Workers (2-hour sandboxes): start it in the background in your first minutes**, keep working, and read the result
before you push:

    NF_ISO=<abs path to the image> nohup python3 tools/test/suite.py > suite.out 2>&1 &
    tail -1 suite.out            # the total, when it's done
    grep -E "FAILED|TIMEOUT|CRASH" suite.out

Re-run only what you touched with `--only bin1,bin2` (the test binary names, e.g. `fix_portal_live`), but run the whole
suite at least once on your final tree (your branch merged with `origin/master`): that's what catches the regressions
that only show against current master. Report the total line in your report.
