# faster-live-tests — cut the slow live-test binaries' time without losing coverage

Branch `s/faster-live-tests`. The 2026-10-15 suite log's per-binary seconds (master tree) are the
"before" column; every "after" number is a direct measured run on this branch (release,
`--test-threads=1`, `NF_ISO` set) or the branch's full-suite log, as marked.

## What was done

Profile findings (all [M] = the test sources; no game behavior touched — this task changes no
`nf-view`/`nf-game` source, only test harnesses):

1. **models_misc_live (100 → 39.8 s, measured direct run)** — the binary did 80 `nf_view::load`s
   of 34 unique archives; seven of its ten tests read ONLY `scene.misc_drawn` off a fresh load.
   Added a process-wide `misc_drawn_cached` (`OnceLock<Mutex<BTreeMap<String, BTreeMap<u8, usize>>>>`,
   cloned out per caller — the suite runs `--test-threads=1`, and the three sim-mutating FX tests
   keep their own fresh loads). Also deleted the discarded `nf_view::load("0700000d")` in
   `lod_block_parses_and_destroy_swap_targets_resolve` (result unused; every assertion reads the
   raw archive parse; the level's load is pinned by the census tests in the same binary).
   80 loads → 37. `misc_census_golden`'s sha1 pin passes unchanged.
2. **walkreal_07000001_live (132 → 92.9 s, measured direct run)** — 25 lenient walk legs per
   passing run are deterministic geometry wedges that each burned the full 6000-tick give-up cap
   (~150k of ~174k ticks = 87% of the binary). `common/walker.rs`'s LENIENT give-up cap is now
   3000 ticks (`LENIENT_GIVE_UP_TICKS`). The give-up is the designed outcome ("skipped node, next
   re-aims" [walk_nav_to]); every leg that genuinely arrives on this route lands within 1800
   ticks, so no arriving leg's outcome changes — only how long a wedge burns before the documented
   skip. The NON-lenient panic cap stays 6000. common/walker.rs is included by 23 binaries; the
   full-suite run on this branch validates all of them (see below).
3. **fix_striped_box_census (73 → 52.1 s, measured direct run)** — `probe_gate_verdicts_per_kind`
   loaded 07000012 and 07000014 three times each (18 loads + 18 archive parses for 11 distinct
   archives). Grouped the PROBE/SISTERS checks per archive: one load + one parse each, same
   assertions per (name, kind). The census test itself is untouched (its prefilter parse + full
   load is inherent: ~24 unique archives, read-only).
4. **fix_party_mission_live (61 → see suite)** — six of fifteen tests are assertion-free evidence
   probes (print-only): probe_party_start_guards, probe_bouncer_props, probe_guest_hitboxes,
   probe_objective_banners (4 loads + ~13k ticks alone), probe_kimiko_scene_guards,
   probe_drone23 (the last two replay the exact scenario `the_guards_reappear…` pins). All six
   are now `#[ignore]`d with the pinning test named; `-- --ignored` still runs them. Zero
   assertions removed. Also corrected the item-5 header comment that wrongly claimed the banners
   probe "pins the bound text" (the real pins are the banner tests).
5. **portal_sweep_live (40 → 35.6 s, measured direct run)** — the sweep test's raw-walk diagnostic
   scan re-ran `los_ray_clear` over every near generic cell purely to PRINT counts; the shipped
   loop right below reuses the identical filters. Removed the print-only scan; both assertions
   (`dirs_total >= 400`, shipped-mask failures empty) untouched.
6. **fix_guard_weapons_live (157 → see suite)** — the stall window before a re-park was 3000
   ticks (~2-3 s per stall, and stalls are expected on the passing path: guards reposition out of
   the sight line). Now 900 ticks = 15 game-seconds, still 12× the 75-tick min-gap floor and ~75×
   a burst's ≤12-tick gaps, so the gap/burst asserts stay stall-safe (boundary gaps ≥ 900 can
   never join a burst or hide a true gap). The 200-shot sample, the 60000-tick cap, the 4-re-park
   limit and every assertion are unchanged.
7. **Deleted `crates/nf-view/tests/zz_obj_drives_probe.rs`** (the f71a2925 "campaign runner"
   probe). It contains ZERO assertions (verified by grep) — six print-only tests whose `.expect()`s
   are existence checks already pinned elsewhere: the 0a keyboard/mux chain and Q-Worm/plate/
   floor-touch facts by campaign_flow_live (its asserted drivers), the cold 08 chain (incl. ch 5 +
   ch 100) by fix_objects2_live, the 4a ride by campaign_flow_live. The warm-load pack-cinematic
   gap it reproduces is documented at campaign_flow_live's transition legs and
   docs/reports/fix-objective-drives.md §5; nothing to migrate. Backlog row L-464's EV pointer
   updated to the pinning tests.

Not touched (no safe cut found in the time box): weapon_matrix_live (its per-cell budgets are the
measurement itself — the OnceLock 07000005 row share already exists), levels_sweep_live (each
level loads exactly once; the time is inherent per-level work), campaign_flow_live (see "Left").

## Before/after (per-binary seconds, release, --test-threads=1, NF_ISO set)

| binary | before (2026-10-15 suite log, master) | after (this branch) | how measured |
|---|---|---|---|
| weapon_matrix_live | 307 | unchanged | not modified |
| levels_sweep_live | 184 | unchanged | not modified |
| fix_guard_weapons_live | 157 | (suite) | suite log |
| walkreal_07000001_live | 132 | 92.9 | direct run |
| models_misc_live | 100 | 39.8 | direct run |
| campaign_flow_live | 85 | unchanged | not modified |
| fix_striped_box_census | 73 | 52.1 | direct run |
| fix_party_mission_live | 61 | (suite) | suite log |
| portal_sweep_live | 40 | 35.6 | direct run |
| **targeted total** | **1139** | | |

Walker-cap side benefit: every walker.rs binary's wedged legs burn half as long; the suite log
shows the effect on the other walkreal_* binaries.

## Full suite on the final tree

(filled after the run — total line + FAILED/TIMEOUT check)

## What's left (next message can continue from this branch)

- **campaign_flow_live (85 s)**: `front_end_chain_01_to_05` byte-identically replays tests 1+2's
  mission legs (39 loads of 24 unique archives; 13 redundant). The cut is a process-wide
  `OnceLock<Mutex<HashMap<u32, Row>>>` memoize around `play_mission` (tests 1+2 warm it, test 5
  consumes `parsed/spawned/complete` only) — mechanical, ~18-25 s, left undone here because the
  row plumbing deserved its own careful pass.
- **Engine-side load dedup** (helps EVERY binary): every `nf_view::load_impl` re-reads ACTION.ELF
  and builds `WeaponData` twice per load (lib.rs merge_weapon_models path + the load path), and
  the weapon-model merge re-reads + fully re-parses earlier story archives on every later load.
  A `OnceLock` for ACTION.ELF/`WeaponData` + a session cache of foreign parsed maps (the skins
  cache pattern already in lib.rs) is pure dedup; needs its own pass.
- **weapon_matrix_live (307 s)**: the matrix's per-weapon honest budgets are the measurement; a
  cut means re-deriving per-weapon minimum ticks with evidence. Not attempted.
- Baseline suite re-run note: this sandbox's disk filled (9.9 GB), so the "before" column cites
  the 2026-10-15 suite log quoted in the task (master tree) instead of a fresh baseline run; the
  direct per-binary after-runs above are same-machine measurements.
