# drive-ptp — the PTP handover block and the ACTION↔DRIVING level transition

Branch `s/drive-ptp`. Everything below is measured from the decompile [M], the ELFs [D] or the
EE snapshots [L]; the [S] gaps are listed with the live checks that replace them.

## What's done

**1. The PTP block as a Rust struct — `crates/nf-game/src/ptp.rs` (new).**

`PtpBlock` models every field the two sides' handover code touches, each with its offset in the
doc comment:

- `Boot_GetPTPData` 0x1c8198 (968 B) writes the block at `PTPDATA` 0x31c090, size 0xa50
  [M 0x1c8198:97-98]: the 32-byte mission-name copy from 0x2e1ae8 (+0x000), the mini-mission
  kind +0x080, the DAT_002a38xx cheat/settings bytes +0x088..+0x0ac, `SFXGetLanguage()` +0x0a4,
  the MP settings words +0x0b0..+0x0e4, `SFXGetVolume()`/`SFXMusicGetVolume()` +0x0ec/+0x0e8,
  the 56-byte settings copy +0x0f0..+0x128, `menu_unlock_everything` +0x128, `CheatInfo` +0x12c,
  `Menu_GetNightfireStatus()` +0x138, `Menu_GetBonus(0)` +0x13c/+0x140, the mission-state byte
  +0x144 (+0x984), `GameState._8_4_` +0x14c, `GameState[0x52]` +0x150, the 12 stats slots
  +0x154/+0x184, and the tail constants (the valid magic +0x960 = 1, the volumes /2 +0x964/+0x968,
  the index +0x974 = `GameState._8_4_ - 0x9000000`, `Txt_GetLanguage()` +0x988, `psiGetTVType()`
  +0xa1c, the medal bit +0x9a0, `Menu_GetBestMedal` +0xa3c, `psiGetScreenPos` +0xa30/+0xa34 …).
- The DRIVING-side readers [M]: DRIVING main 0x174bf0 gates the retail path on +0x960, reads the
  mission index +0x974 into its key switch (1=paris_mis01 … 8=snow2a_race, default paris_map —
  the key strings [D] at 0x374da8..0x374e28, the same order as the four DRIVING tables at
  0x25c948/0x25ce30/0x2602b8/0x2622f0 [D]), maps the status +0x984 (1→0, 2→1, 3/4→2), and writes
  the level key into +0x4f4 (15 chars + NUL); `FUN_00175a28` 0x175a28 opens the archive from the
  key's suffix after the last `_`. `psiLaunchDriving` 0x120af0 pre-sets +0x97c = 1 after the
  copy to EE 0x100000; the exit `FUN_00300058` 0x300058 gates on the ended flag +0x4b4 and
  overwrites +0x97c with its result arg.
- The return mirror `Boot_LoadPTPData` 0x1c8560 [M]: status byte +0x978 → `Mission_SetStatus`
  (0 → 6, 4 → 3, else → 2), `GameState+0x28` (0x9000001-with-flag → 0x40000043; +0x978 0 →
  0x40000036; 1/2/4 → 0x4000001c; 3/5+ → none), and — unconditionally —
  `ResetMap_LevelToLoad(0x7000048, 0, 0)`: **the return always resumes at the front end**
  0x07000048, with `PlrStats_SetDrivingData(0x31c1e4, 0x31c214)` carrying the 12 slots.

**2. The 12 `PlrStats` driving slots.** `DrivingStats` ports `PlrStats_GetDrivingData`
0x199ac8 / `PlrStats_SetDrivingData` 0x199b18 [M]: the 12-entry table rooted at the pointer at
0x2bc658 (table 0x2bbda0, stride 0x3c, +0x00 record pointer, +0x18 id), values at record+8; ids
and values travel in the PTP at +0x154/+0x184. Boot state: every record+8 is 0 [L ee_start_05.bin
@ 0x2bbda0+8 ×12].

**3. The level-transition hook (nf-view, minimal).**

- The mission layer's existing `Event::LevelEnd { next: 0x0730xxxx }` (mission.rs case 6's
  measured pairs [M 0x182c60:88-116]) used to be logged and dropped — the mission chain stopped
  at the castle. Now the view's LevelEnd arm calls the new pub `Sim::ptp_handover` (scene.rs):
  it builds the block from the Sim's state (the handover map id `0x900000X` from
  `interlude_drive_index`, `GameState+0x28` from `Drones.difficulty` — the port's name for that
  word [M bootup 0x1c7f88]), stands in for the drive with `Drones.driving_result` (Success →
  result 1, the DRIVING main exit tail's `FUN_00300058(0x100000, 1)` path [M 0x174bf0:315-323];
  Fail → result 0), stashes the `DriveReturn` on `Drones.driving_return`, and arms
  `level_exit = Some(0x07000048)` cold — the return's resume level [M 0x1c8560 tail].
- lib.rs's reload arm takes the stashed return from the old sim and applies `DriveReturn::apply`
  to the new one: `Mission_SetStatus` per the status byte + the 12 slots carried —
  `Boot_LoadPTPData`'s mirror. (The game's own `Mission_Init` skips 0x7000048 entirely
  [M 0x182270:20-22], so the Boot-set status stands at the hub.)

**4. Shared files touched (small, additive only).**

- `crates/nf-game/src/lib.rs`: one `pub mod ptp;` line.
- `crates/nf-game/src/drone.rs`: three `Drones` fields (`driving_stats`, `driving_result`,
  `driving_return`), each on its own line in the struct and the constructor (merge lesson 4).
- `crates/nf-view/src/scene.rs`: the LevelEnd arm's interlude branch + the pub `Sim::ptp_handover`
  (the body extracted so the headless tests drive the exact path the view drains).
- `crates/nf-view/src/lib.rs`: the reload arm's take/apply pair for `driving_return`.

**5. Tests.**

- Unit (nf-game/src/ptp.rs, 9): the key table against the ELF strings, the suffix rule [M
  0x175a28], the index math [M 0x1c8198:62], the interlude map [M 0x182c60], the block built
  from a known state (field values), the non-base handover, the return mapping (0→6/0x40000036,
  4→3/0x4000001c, else 2/none), the stats round trip, the resume target for each driving mission.
- NF_ISO headless (`crates/nf-view/tests/walk_07000004_ptp_live.rs`, 4, through the real
  `nf_view::load` + `Sim::step_bond` path): the 07000004 kill route (the grenade at the target —
  walk_07000004_live's chain) runs the mission-complete banner into the handover: `level_exit`
  = 0x07000048 cold, the return stashed (result 1 → MissionState 6, index 1, "paris_mis01",
  PTP+0x4f4 bytes verified); the Fail sister (result 0 → MissionState 3, the same resume level);
  the 12 slots ride the handover and the apply mirror.

**6. `docs/atlas/driving.md`**: §2 gained the "Ported (drive-ptp)" status block; the header
status line names the task.

## How it was verified

- `cargo test -p nf-game ptp::` — 9/9 green.
- `NF_ISO=… cargo test --release -p nf-view --test walk_07000004_ptp_live` — 4/4 green
  (pre-fix, the first two fail: the handover never ran and the level_exit stayed `None` — that
  is the reproduce-first record; the probe run printed `internal=5 state=6` with no resume).
- The merged tree (origin/master at a1cb5b36) compiles clean (`cargo check -p nf-game -p nf-view`).
- The full suite (tools/test/suite.py) on the final merged tree: see the total line below.

## What's left / open

- **[S] The interlude → drive-index pairing.** The retail flow reaches `ResetMap_LevelToLoad
  (0x900000X)` from the hub menu's mission list (`sp_level`, read by `Menu_GetLevelIndex`
  0x2023e8), which this window did not read; the port pairs the 8 interludes in `Mission_Update`'s
  order with the 8 drives in DRIVING's table order [D 0x25c948] — the castle's 0x7300004 → 1
  (paris_mis01) per the task's example. `ptp::interlude_drive_index` is the single swap point.
- **[S] PTP+0x978's DRIVING-side writer** (the status byte the return maps into Mission_SetStatus)
  is unidentified — the decompile of the writer is not in `work/decomp/DRIVING.ELF/`'s read set.
  The dev stand-in writes 0 (Success) / 4 (Fail).
- The drive itself: the port skips it; `Drones.driving_result` is the knob (no dev-menu page yet).
- The `-t/-T` dev-host key path (DRIVING main 0x174bf0:243-252) is ported as the key table, not
  the argv parse (the port has no DRIVING process).

## Live checks for the main agent

1. **The drive-index pairing** — in PCSX2, from the hub, launch the drive that follows the castle
   arc; break on DRIVING main 0x174bf0 after `iRam00100960 != 0` (the +0x960 gate), read
   `uRam00100974` (PTP+0x974) before the switch. Expected for the castle → drive handover: the
   value my table assumes (1). Repeat for one more arc (e.g. after 0x07000008) to confirm the
   order; send the observed (interlude code, index) pairs.
2. **PTP+0x978's writer** — set a gdbwatch on 0x100978 (writes) during a drive session; the
   writing function's address identifies the status-byte source my stand-in replaces.
3. **The volumes** — at the drive's start, read 0x100964/0x100968; they should be
   `SFXGetVolume()/2` / `SFXMusicGetVolume()/2` from the ACTION session [M 0x1c8198:72-76].
4. **nf-view render spots** (`--bond`): on 07000004 after the kill, at the "Mission Complete"
   banner end the scene should reload the hub 07000048 cold (the log line `[ptp] drive handover`
   then `level: loading 07000048.bin (cold)`). Suggested spots: the castle exit overlook
   (before the banner) and the hub's spawn platform (after the reload).

## Suite

Run on the final tree (merged with origin/master a1cb5b36; the merge committed before the runs),
foreground chunks of `tools/test/suite.py` (background jobs die in this sandbox):

- All of nf-game, nf-formats, nf-audio (the canonical `cargo test --release -- --test-threads=1`
  pass): **77 passed / 0 failed** + nf-formats' **3 `disc_golden` failures** — the allowed
  slim-ISO set.
- nf-view by `--only` chunks over the 293 test binaries, in priority order (the mission/level/
  menu paths this task touches first): chunk A (mission/level/menu/restart/progress, 23
  binaries) 141/0 + 1 crash; chunk B (1–70) 211/2; chunk C (71–150) 158/0 before the cut +
  its remainder 122/0; chunk D1 (151–230) 254/1 before the cut.
- **Case total: 886 passed, 3 failed, 26 ignored + 1 binary crash.** The 3 failures are
  `fix_0c_pylon_live` ×2 + `probe_pylon_render` — the known no-GPU environmental set (wgpu
  "adapter: NotFound", the sandbox has no GPU). The crash is `fix_death_restart_live` — **OOM**,
  not a test failure: the kernel killed it at 2.7 GB RSS mid cold-load of 07000004 (dmesg:
  `oom-kill ... global_oom task=fix_death_resta`); 14 of its 15 tests passed before the kill and
  it reproduces standalone on a quiet box (4 GB total RAM here). It does not intersect this
  task's changes (three additive struct fields; the death path never sees a driving interlude).
- Not reached in the window: the ~58 binaries behind chunk D1's cut and the 231–293 tail
  (mostly the `probe_*` / `dump_*` exploratory dumps).

## ntfy failed

All three fallbacks hit the daily quota (429) at 2026-10-15T18:08Z:
1. `curl` → `{"code":42908,"http":429,"error":"limit reached: daily message quota reached; increase your limits with a paid plan, see https://ntfy.sh","link":"https://ntfy.sh/docs/publish/#limitations"}`
2. `python3 urllib` → `urllib.error.HTTPError: HTTP Error 429: Too Many Requests`
3. `wget --post-data` → silent, exit 0 (same 429 body; no output)
