# Worklog

Numbered entries, newest last. Each says what was done, what was measured, and what failed and why.

## 0001 — Bootstrap: build identity, disc reader, golden tests (2026-10-08)

**Done**
- Found the image at `C:\Users\Yoda\Downloads\PS2\007 - Nightfire (USA).iso`; identified SLUS-20579 NTSC-U v1.00
  (docs/build-identity.md). Image SHA-1 `03717059…`; ELF SHA-1s recorded.
- New repo `NightfirePort`; copied from RC2Port: `buf`, `iso9660`, `sha1` (rerac ISC), `vif`, `pss`, `vag`, `png`, the
  three Ghidra scripts and `build-project.ps1` (adapted to the three Nightfire ELFs). `pss.rs`: dropped the RC2 movie
  test (RC2 paths) and marked its RC2 facts *suggested* for Nightfire.
- New readers: `elf` (header/sections/`.symtab`/extent), `bigf` (EA BIGF), `symfile` (DRIVING.SYM), `disc`
  (manifest, BIGF manifest, unlisted sectors), `golden` (TSV diff). CLI `nf-extract`.
- Golden snapshots `crates/nf-formats/golden/disc_files.tsv` (191 entries) and `bigf_entries.tsv` (1,499 entries, 23
  archives). Opt-in `NF_ISO` test `disc_golden` (4 tests, each with a tamper sister assertion): all pass.
- **Cross-check:** `tools/crosscheck/iso-7zip.sh` hashes every file through 7-Zip's own ISO 9660 handler: 177 files,
  **0 differences** in path, size and SHA-1 against our manifest.

**Measured**
- ACTION.ELF is unstripped (5,603 symbols); DRIVING.ELF and the loader are stripped. BASE.ELF == SLUS_205.79.
- BIGF: header-size field = entry-table end (`.VIV`) or table end + 8 (`.MUS`/`.SPE`: trailer `L225` + BE 2); LE size
  field = file size except MISC.VIV (U-004). Offsets sorted.
- DRIVING.SYM: 12,719 sorted `(addr, name)` entries; does **not** match our DRIVING.ELF (U-003).
- `FileList` (ACTION.ELF 0x2446a0, 118 × 32: name[16], offset, size, 2 × pad) indexes FILES.BIN: offsets 2048-aligned,
  no overlaps, last end 764,086,163 ≤ 764,086,272. First reported by NightfireResearch (*sourced*), now measured.
- Our ACTION.ELF and DRIVING.ELF SHA-256 match NightfireResearch's published SLUS-20579 values; the image SHA-1 does
  not (U-001).

**Failed / corrected**
- I first concluded DRIVING.ELF was absent from the directory tree and "unlisted at LBA 1153". Cause: my listing
  command piped through `grep -v DRIVING` (meant to hide the `/DRIVING/` folder), which also hid `/DRIVING.ELF`.
  Code and docs written on that premise were corrected the same session (`disc::unlisted_elfs` removed).
- The Bash tool collapses `\\` to `\` in heredocs: `"\\t"` in a Rust test became a real tab (test failed), the same
  happened through a Python heredoc writing Rust, and a `tr '\\' '/'` in a shell script. Fixed with the Edit tool;
  rule added to CLAUDE.md.
- `unlisted_ranges` test expected `(0, 20)`; the root directory's own sector is not a child entry, so it is
  `(0, 21)`. Test expectation was wrong, not the code.
- The BIGF "entry past the end" test used `len − 1`, which still contained the padded entry; now uses the entry's
  exact end.

## 0002 — Ghidra project and input decomp (2026-10-08)

**Done**
- `tools/ghidra/build-project.ps1` → `work/ghidra/NightfirePort.gpr`; exports: SLUS_205.79 406 functions,
  ACTION.ELF 3,038 functions (0 decompile failures), DRIVING.ELF 6,655 functions (stripped; Aggressive Instruction
  Finder + pointer-target functions).
- Drafted `docs/systems/input.md` from the decomp: pad state machine, linear dead zone (40, ×127/87), 8 controller
  styles → 40 action floats, radial `MapAnalogStick` curve (table 0x2a3840), axis snap, invert, clamp, aim/turn ramps,
  auto-aim gating, and the frame pacing.

**Measured (highlights)**
- Logic is variable-step: one update per 1 or 2 vblanks, `FRAME_RATE_MUL` = 1 or 2 (`main` 0x109080,
  `GS_SetRefreshRate` 0x17d2b8). Ramps are per update, so not frame-rate independent.
- `ReadTuningVars` (0x1d3a80) overwrites the `.sdata` tuning defaults (e.g. `Plr_AimSpeed_X` 0.15 → 0.1275,
  `Autoaim_Range` 35 → 25, Hard auto-aim multiplier 0); those values match the uncommented lines of `TuningVars.txt`
  in FILES.BIN.
- The curve-mapped stick copy in `psiInput_PollDevices` is overwritten in the same loop (no effect).

**Failed / corrected**
- My first reading took the `.sdata` initial values of `Plr_*` as the live tuning; `ReadTuningVars` replaces them.
  input.md uses the live values and lists both.
- Read `psiInput_MapInputs`' second argument as the controller style; it is the player count (style is per player at
  `PlayerSetting`+0xe).
