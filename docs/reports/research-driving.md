# Report: research-driving

Task: map the driving side (DRIVING.ELF + the driving missions) so a later phase can plan the
port. Research-only; no game code changed. Branch `s/research-driving`.

## What's done

1. **The handover is measured end to end** (docs/atlas/driving.md §2–3):
   - ACTION.ELF: `GameFlow_Main` 0x1c98c8 state 9 → `Boot_GetPTPData` 0x1c8198 fills the PTP
     block `PTPDATA` at 0x31c090, size **0xa50** (game state, cheats, MP settings, language,
     volumes ÷2, medals) → `psiLaunchDriving` 0x120af0 copies it to **EE 0x100000**, sets the
     +0x97c flag, `RunNextModule("cdrom0:\\DRIVING.ELF;1")`.
   - DRIVING.ELF: `entry` 0x24c0d8 → **main = `FUN_00174bf0` 0x174bf0** (argv: `-streams`,
     `-ntsc`, `-pal`, `-pal60`, `-t<level>`); the level key lives at 0x1004f4 (PTP+0x4f4); the
     init chain `FUN_00174198` 0x174198 opens `MISC.VIV` and the level archive
     (`FUN_00175a28`: `driving\<suffix>.viv` + `driving\<suffix>.spe`).
   - Return: `FUN_00300058` 0x300058 writes the result flag (+0x97c), optionally a next level key
     (+0x4f4), and launches **BASE.ELF** (the boot loader), which re-dispatches;
     `psiGetDrivingData` 0x120a88 reads 0x1000 bytes back from 0x100000 on the ACTION side, and
     `PlrStats_Get/SetDrivingData` 0x199ac8/0x199b18 carry 12 driving stat slots.
2. **The levels are mapped** (archive → mission key → ACTION menu strings): paris_mis01, uw_mis11,
   junglea/b/c_mis13a/b/c, snow1a_mis3, snow2a_mis4, snow2a_race (RACE.VIV), MISC.VIV shared.
3. **Every driving archive is censused and golden-pinned** (new test
   `crates/nf-formats/tests/driving_golden.rs`, 4 tests, all passing):
   - BIGF layer of all 23 archives pinned (`golden/driving_census.tsv`: counts, table ends,
     payload bytes, refpack/plain/audio split, names+offsets+sizes SHA-1);
   - **all 1,499 entry contents re-hashed against the disc-measured `golden/bigf_entries.tsv`** —
     proves the extracted DRIVING/ folder is byte-identical to the disc;
   - the content-magic classification pinned (RefPack `10fb` vs plain magics vs ASF audio);
   - `DRIVING.SYM` parse pinned (12,719 entries).
   Note: the slim NF_ISO image has the DRIVING directory entries but zero-byte extents, so the
   archives are read from the extracted `DRIVING/` folder (`NF_DRIVING` env or `<repo>/DRIVING`);
   the folder is proven disc-identical by the test itself.
4. **New format notes**: docs/formats/driving-viv.md — the full census, the RefPack finding
   (`10fb` majority; `.tun` contains `GRAVITY=-34`/`SLEEP_VEL=5`, `.sfn` starts `FNTS`, `.atr`
   INI text), the per-extension magic table, the stable entry tree of a mission .VIV (track
   `data\track\<level>.crp` is the big new format; car models `data\car\model\*.crp`; identical
   physics tuning/skeletons/HUD gallery across all missions), and what current parsers pass
   (numbers): bigf 1,499/1,499, symfile 12,719, everything else not yet.
5. **The atlas**: docs/atlas/driving.md — levels, handover, main/boot/exit, subsystem symbol
   inventory (`EAGL`, `RView`, `Act*`, `CARP`, `IOModule`, `InputDevice`), sound/music/speech,
   port estimate (phased, sized), live checks for the main agent.
6. **G-004 groundwork**: the SYM hint addresses for `PS2PadDevice`/`InputDevice::DeadZoneChop/
   Scale` are confirmed **absent** in our build (no decompile files at 0x180530/0x180560/...);
   the names remain the shape-search seeds; the control configs (`Master.def`/`FrontEnd.def`,
   plain `POV 8…` text) are on disc inside the mission VIVs. The functions themselves: next phase.

## How it was verified

- Every code fact cites function+address from `work/decomp/{ACTION,DRIVING}.ELF/` or the ELF
  binaries; every disc fact cites the file+offsets (no memory assertions).
- The golden test is the reproducible core: 4 tests, sister assertions included (tamper a digest →
  caught; a flipped content hash → caught).
- `cargo test --offline --release -p nf-formats --test driving_golden`: **4 passed**.
- Suite state before the final push: `nf-formats`: 11 passed / 3 failed — the 3 failures are
  exactly the allowed `disc_golden` tests (the slim ISO has the directory entries but zero-byte
  extents; the rules permit these 3). `nf-game`, `nf-audio`, `nf-view`: no code changed by this
  task; all three compile clean (`cargo test --no-run`), but their full run exceeds the sandbox's
  10-minute tool timeout here, so they were not re-executed in this session (last known green on
  master).

## What's left (the next phase's list)

- The per-frame game loop and the timestep (main's tail after `Entering GameLoop`).
- The vehicle physics: CARP structs, integration, collision (from the track `.crp`).
- The `.crp` format (RefPack decompressor first — phase 0 of the estimate).
- The shared-code census (shape-matching known ACTION functions into DRIVING).
- G-004 proper: find the deadzone functions in our build, the pad read path, the mapping.
- The boot loader's dispatch rule (live check), the DRIVING-side result-block writer,
  the PTP field-by-field map.

## Shared files touched

- `docs/WORKLOG.md` — appended (`research-driving` entries only).
- No other shared file was modified. New files: `docs/atlas/driving.md`,
  `docs/formats/driving-viv.md`, `crates/nf-formats/tests/driving_golden.rs`,
  `crates/nf-formats/golden/driving_census.tsv`, this report.

## Live checks for the main agent

1. Boot-loader dispatch: after DRIVING exits into `BASE.ELF`, watch the loader's reads of EE
   0x100000..0x101000 and which string (`cdrom0:\ACTION.ELF;1` vs `cdrom0:\DRIVING.ELF;1`) is
   loaded; expected: the +0x97c flag and/or the +0x4f4 key decide.
2. Dump EE 0x100000..0x101000 (a) right after `psiLaunchDriving`'s memcpy (ACTION 0x120af0),
   (b) right before `psiGetDrivingData` 0x120a88 — diff them: the changed fields are the result
   block.
3. During a driving mission, confirm the per-frame tick: count vblank interrupts vs a fixed
   in-game timer to pin the timestep.
4. Track data consumer: breakpoint the BIGF read of `data\track\mis4.crp` (inside
   `driving\mis4.viv`) and record the function that takes the buffer (the physics/road init).

## ntfy

Sent (curl, first fallback): message id `VQUJbZX28pn8` on `nightfire-yoda-8k3x`.
