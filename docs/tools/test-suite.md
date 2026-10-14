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

## The `movies` feature: check those tests separately

The suite (and the rule-5 `cargo test` line) build **without** the opt-in `movies` feature, so the movies-gated tests
(`crates/nf-view/tests/menu_movies.rs`, `movie_playback_flow.rs`, `crates/nf-formats/tests/pss_movies_live.rs`) never
build there — a movies-only build break (e.g. menu_movies.rs dereferencing the `u32`s `Player::present` returns,
2026-10-14) goes unnoticed. After any change near the movie code, build and run them explicitly (fix-movies-tests):

    cargo test --offline --release -j2 -p nf-view -p nf-formats --features nf-view/movies --no-run
    NF_SILENT=1 NF_ISO=$NF_ISO target/release/deps/menu_movies-* --test-threads=1
    NF_SILENT=1 NF_ISO=$NF_ISO target/release/deps/movie_playback_flow-* --test-threads=1
    NF_SILENT=1 NF_ISO=$NF_ISO target/release/deps/pss_movies_live-* --test-threads=1

Notes:
- `--features nf-view/movies` (not bare `movies`) selects the feature unambiguously when both `nf-view` and `nf-formats`
  are in the command; it enables `nf-formats/movies` through the dependency.
- `NF_SILENT=1` keeps movie audio off the speakers (the same rule the suite enforces).
- On the slim on-foot ISO the PSS files are stubs, so the PSS-data tests **skip** (`... is a stub on this image`) —
  they only really run with `NF_MOVIE_ISO` pointed at an image with movie data. `pss_movies_live` is expected to fail
  on the slim image per the suite rules.
- Building with the feature recompiles `nf-view`/`nf-formats` (feature churn against the default build); run the
  binaries directly from `target/release/deps/` afterwards instead of re-invoking cargo per binary.
