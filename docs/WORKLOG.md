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

## 0003 — FILES.BIN via FileList (2026-10-08)

**Done**
- `filelist.rs`: reads `FileList` from ACTION.ELF through its own `.symtab` entry (no hard-coded address), parses
  118 × 32-byte entries, validates alignment/bounds/overlap. `disc::files_bin_manifest`, CLI `files-manifest` /
  `files-extract`, golden `golden/files_bin.tsv` and the opt-in test `files_bin_matches_the_golden_manifest` (with
  sister). All 5 opt-in disc tests and 34 unit tests pass.
- `.gitattributes` pins LF (git had warned it would convert to CRLF on checkout, which breaks the bash script).

**Measured**
- Entries are contiguous (0 gaps); 117/118 sizes are 2048-multiples; `psiFileLoad`/`psiFileOpen` pass the table size
  through as the length (U-006 closed). Docs: docs/formats/files-bin.md.

**Not done**
- Independent cross-check (G-001) needs the user's approval to download a community extractor.

## 0004 — Independent cross-checks with community tools; Ghidra MCP (2026-10-08)

The user approved the downloads. Both repos were cloned into `work/third_party/` (git-ignored) and run in their own
venvs; nothing of theirs is copied into the repo (licences: nightfire-re none → run only; unvivtool GPL-3 → run only).

**Done / measured**
- `tools/crosscheck/nfre_files_bin.py` drives NightfireResearch `nightfire-re` (commit acd94c2): pycdlib dumps the
  disc; their PS2 handler reads FileList at their hard-coded offset. **FILES.BIN: 118 files, 0 differences** (name,
  size, SHA-1). Their tool refuses our image by hash (U-001), so the driver calls the two handlers directly.
- The same pycdlib dump vs our directory manifest: **177 files, 0 differences** (second independent ISO reader after
  7-Zip).
- `tools/crosscheck/unviv_bigf.py` drives `unvivtool` 3.12 (commit 38a63fe, built from source): **all 23 BIGF
  archives, 1,499 entries, 0 differences** (path, name, offset, size, SHA-1).
- unvivtool reads MISC.VIV's odd size field big-endian: 0x0b455c29 = file size − 2,407 (U-004 refined).
- ghidra-mcp: `tools/ghidra/start-ghidra-mcp.ps1` serves `work/ghidra/NightfirePort.gpr`; the registered bridge
  connects (`connect_instance("NightfirePort")`, TCP 127.0.0.1:8089, 67 tools in default groups).

**Failed / corrected**
- unvivtool `unviv()` extracted 0 of 152 files from MIS01.VIV: it does not create subdirectories for `data\…` names.
  Pre-creating them fixed that, then `data\Render\Actors.ssh` landed in the existing `data\render` folder
  (case-insensitive NTFS). Matching by lower-cased name then failed on MIS3EN.SPE, which really contains two
  different `garage.asf` entries (U-015). Final driver extracts each entry by 1-based index into its own folder.
- First ghidra-mcp server on port 8090: `list_instances` showed it, but `connect_instance` reported "no instance
  matching 'NightfirePort'" while listing it as available (UDS selection fails on Windows; the bridge's TCP fallback
  is fixed to 8089). Moved our server to 8089 (G-011).
- PINE: PCSX2 ini now has `EnablePINE = true` but nothing listens on 28011 until PCSX2 restarts (pending).
