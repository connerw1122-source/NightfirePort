# fix-ammo-pane2 — the ammo-pane tests read the disc, not the local snapshot

Branch `s/fix-ammo-pane2` = `s/fix-ammo-pane` + `origin/master` (c9ada8c8) merged first, per the task base.

## Held symptom

`crates/nf-view/tests/ammo_pane_live.rs` panicked at line 17 reading `work/ee_snapshot.bin`
("The system cannot find the path specified") in the merge checkouts: the file is a local PCSX2 EE
snapshot, not in the repo, and CLAUDE.md says ISO-reading tests may depend only on `NF_ISO` (no game
bytes in the repo). The same held for `game_bullet`'s read of `work/elf/ACTION.ELF` (line 24). Two
tests died (`the_four_weapons_pane_text_labels_and_icons_match_the_game`,
`the_kind_1f_display_is_the_depleted_clip_of_max_minus_12`); the timer test (NF_ISO-only) passed.

## Reproduced first

With `work/ee_snapshot.bin` and `work/elf/ACTION.ELF` hidden (exactly the merge-checkout condition):

```
thread 'the_kind_1f_display_is_the_depleted_clip_of_max_minus_12' panicked at
crates/nf-view/tests/ammo_pane_live.rs:17:57:
work/ee_snapshot.bin: Os { code: 2, kind: NotFound, ... }
test result: FAILED. 1 passed; 2 failed
```

## What changed (test file only)

Every game number the tests need now comes from the disc through the port's own loaders:

- **Weapon records** → `nf_formats::weapon::WeaponData::from_action_elf` over the ISO's own
  `ACTION.ELF` (`IsoImage::open` + `find("/ACTION.ELF")`). That loader runs the game's boot static
  initialiser `_GLOBAL_$I$weapon_data` (0x1b9900) over the user's ELF — the same fill that put these
  bytes at `weapon_data` 0x2bf150 in boot memory (docs/formats/weapon-data.md, "How we get it").
- **`BulletImg.189` rows** → raw f32 rows sliced from the disc's ACTION.ELF at
  `0x2BEA38 − 0x106100 + kind·16` [D], independent of the port's `HudTables` parser (which resolves
  the symbol by name through the ELF symbol table — two disjoint mechanisms, so the cross-check
  stays meaningful; audited).
- **Every assertion kept.** Independent audit (assertion-by-assertion against the pre-rewrite file):
  none dropped; the count-text/percent-sister/timer/decay/rest-pane assertions are byte-identical;
  the label-id assertions now assert BOTH the sim's table and a fresh disc parse against the
  boot-memory [L] literals (`ee_snapshot 0x2BF150 + w·0x10C`: 0x06 → 0x0500000e/0x05000036, 0x1e →
  0x05000010/0x05000029, 0x2a → 0x05000005/0x05000032, 0x4a → 0x05000042/0x05000042 — read once from
  the sandbox's snapshot and baked, same convention as fix_weapon_modes_live's hardcoded label ids),
  so their independence from the port's own parse is preserved.
- **The boot-memory cross-check is an `#[ignore]`d probe** with the reason in the attribute:
  `probe_weapon_table_matches_the_boot_snapshot` — compares the port's disc parse against
  `work/ee_snapshot.bin` for all 115 records on the pane's fields (+0x00, +0x30..+0x3c, +0x90,
  +0x92; +0xDC excluded — the skin hashcode the game rewrites at run time, per
  docs/formats/weapon-data.md). Run with `--ignored` on a sandbox that has the snapshot.

## Verification

- Pre-fix: the two tests panic at :17 with `work/` hidden (above) — the held symptom, reproduced.
- Post-fix, `work/` still hidden: `3 passed; 0 failed; 1 ignored` — the tests depend only on NF_ISO.
- Post-fix, snapshot present: the `--ignored` probe passes — all 115 records × the pane fields equal
  boot memory (the disc parse is the boot fill, as documented).
- Full `tools/test/suite.py` on the final tree (merged with origin/master): **1160 passed, 7 failed,
  40 ignored** — zero new failures. `ammo_pane_live` itself: `3 passed; 0 failed; 1 ignored` (the
  ignored one is the snapshot probe). The 7 failures are exactly the allowed/pre-existing set: the 3
  `disc_golden` (slim ISO), the 3 pre-existing pylon failures (`probe_pylon_render`,
  `fix_0c_pylon_live` ×2), and `weapon_matrix_live` TIMEOUT under the 240 s per-binary cap (the
  binary needs ~341 s — the same documented cap artifact the fix-ammo-pane baseline had; with the
  runner's default 600 s cap the binary passes, per the weapon-matrix4b report).
  Sandbox note: this sandbox kills background processes between tool calls (nohup and setsid both
  died silently during the compile phase), so the suite ran with the per-binary-identical chunked
  runner `/home/z/my-project/scripts/run_suite_chunks.py` — same binary list from the same
  `cargo test --no-run --message-format=json` build, same `--test-threads=1` + `NF_SILENT=1` +
  240 s per-binary limit, same result-line parsing/counting and the same total/FAILED/TIMEOUT/CRASH
  output format, foreground chunks resuming from a state file (per-binary logs in
  `/home/z/my-project/scripts/suite_binlogs/`). All 289 test binaries of the four crates ran.

## Touched files

- `crates/nf-view/tests/ammo_pane_live.rs` — the task's named file.
- `docs/WORKLOG.md` — the `fix-ammo-pane2` entry (appended only).
- `docs/reports/fix-ammo-pane2.md` — this report.
- No production code; **no shared files touched** (drone.rs / scene.rs / player.rs / lib.rs clean).

## Live checks for the main agent

This task changes no rendering — the pane render the previous task verified should be unchanged:

- nf-view `0700000c` `--bond 14.22,16.28,-139.54,0.986,-0.269` (frame 30+): the ammo pane still shows
  the icon, "5-30" and "Single" (the fix-ammo-pane render, unchanged).
- PCSX2 0700000c, same spot: the same three elements — regression check only.

## Left

Nothing for this task. Noted, not fixed (out of scope): with `NF_ISO` unset the tests print
"skipping" and pass — a pre-existing repo-wide convention for opt-in live tests; and the test reads
the ISO's ACTION.ELF twice per run (perf only, ~0.3 s).
