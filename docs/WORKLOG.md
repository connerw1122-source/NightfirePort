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

## 0005 — First live reads over PINE (2026-10-08)

**Measured**
- PCSX2 v2.8.2 (portable) answers PINE on 28011. The 32 bytes at `MapAnalogStick` (0x17f958) in RAM equal
  ACTION.ELF's, so the running build is ours. `pine_get_info` (title/serial/CRC) times out mid-game; U-001's CRC
  still open.
- `GameState`+12 = 0x07000001 (first level; `HT_Level_HendersonA` per NightfireResearch, *suggested*).
- Frame pacing snapshots (0x30c708..): `PS2FramesToSkip` −1 (5×) or 1 (1×), `PS2SkipCount` 0, time words
  game 0x21–0x33, draw 0x22–0x26, frame 0x0f–0x12. Units are scanlines (timer 0 mode 0x83 set in `main`). U-012
  closed; U-013 needs continuous sampling (below).

**Failed / blocked**
- Our own fast sampler (`tools/trace/pine.py`, `frame_pacing.py`) connects but never gets a reply: PCSX2's PINE
  server serves one client at a time and the MCP bridge (mcp-pine) holds the connection (netstat: one ESTABLISHED
  client, ours stuck in CLOSE_WAIT). Needs the bridge stopped while sampling (asked the user).
- `GameCount` advanced only ~1 per MCP call, seconds apart — the emulator was not running gameplay at full speed
  at the time (paused / menu / unfocused?). Re-measure once confirmed.

## 0006 — Live pacing and pad traces (2026-10-08)

The user confirmed gameplay and approved stopping the mcp-pine bridge (node pid 38164, the single PINE client).

**Measured (PCSX2 2.8.2, level 0x07000001)**
- Pacing (`frame_pacing.py`, 15 s, 2,007 samples/s): 819 updates / 829 vblanks; 809 × 1-vblank, 10 × 2-vblank
  (98.8 % 60 Hz); PCSX2 at 55.3 vblanks/s. U-013 partly answered.
- Pad (`pad_log.py`, 1,756 change lines while the user pressed every input in a fixed order): stick bytes and all 16
  button bits as libpad (U-010 closed); style-0 button → action table matches `psiInput_MapInputs`; held buttons
  show the raw masked bit before the clamp (✕ → 31 = +64.0).
- Curve path: actions 3/4 recomputed in f32 from RX/RY with `StickCompensation2` + `MapAnalogStick` (table from our
  ELF): 364/407 exact, 34 explained by a neighbouring sample, 9 torn mid-update reads.

**Failed / corrected**
- `pine.py` first sent several framed requests per write: PCSX2 answered one and stalled. Now one request at a time
  (still ~2 kHz for 5 reads per sample).
- My first pad summary (stable segments) mixed lagged values; replaced by per-sample co-occurrence counts and a
  formula check that classifies every mismatch.
- The dead zone cannot be checked from these reads (raw bytes overwritten in place, G-012).
- The pad logger later exited with `PINE FAIL` when PCSX2 was closed; the log was complete by then.

## 0007 — Level .bin archives (2026-10-08)

**Done**
- docs/formats/level-bin.md from the decomp (`LoaderLoad` 0x1d4a00 state machine, `psiFileRead` 0x10bdf0) and all
  99 `.bin` files; `levelbin.rs` (header, directory, entries, loader-exact `DirFileHash`, `validate`);
  `disc::level_bin_entries` / `level_bin_digest`; CLI `bin-list` / `bin-digest`; golden `golden/level_bins.tsv`
  (96 archives) and opt-in test `level_bins_match_the_golden_digest` with sister. 35 unit + 6 opt-in tests pass.
- Cross-check `tools/crosscheck/nfre_level_bins.py` (nightfire-re `extract_bin`): **0 differences** in 96 archives /
  41,977 entries.

**Measured**
- `Europacked` (0x30d003) = 0 and never written → no compression on PS2. `MaxFilesToLoad` = 0xFFFFFFFF → one batch.
- `psiFileRead` reads whole sectors; on our disc `dir_size` is always a multiple of 2048, so the sector-rounded game
  view and a contiguous reading agree, and every archive ends exactly at the last entry.
- Type ↔ hashcode prefix is consistent (0/1/2/11 → 01, 3 → 05, 4 → 04 (8 × 00), 5/7 → 06, 8 → 08); type 6 =
  `SKELnnnn`, 12 = unnamed 32-byte entries, 15 = `007` (once), 16 = `woman`. Three archives (07F00048/500/999) are
  2048 zero bytes.

**Failed / corrected**
- An earlier scripted edit had turned `"bigf-extract" => bigf_extract` into `=>bigf_extract`, so my Edit anchor did
  not match; fixed with the editor.
- First cross-check run reported 122 differing archives: my driver turned their `07F00029.bin_extract` folders into
  `07F00029.bin.bin`; keying archives by the first 8 characters fixed it (no reader difference).
- I summed the entry count as 43,977 in conversation; the type counts total 41,977.

## 0008 — Map block files and textures (2026-10-08)

**Done**
- `LoaderProcess` (0x1d4880) dispatch read: types 0/1/2/11 → `parsemap_parsemap`; 3/4/5 → `AnimLoadFile`; 6 skeleton;
  7 script; 8 menu; 12 `LoadableFiles`; 15 icon; 16 woman (level-bin.md updated; U-016 closed).
- Map block file read from the decomp and measured on all 2,230 map entries (all version 1, offset table, every
  table ends at 0x1D, all blocks in bounds): docs/formats/map.md, `map.rs`.
- Textures: header/palette/pixel blocks from the decomp (`psiCreateMapTextures`), `texture.rs`, CLI
  `tex-list`/`tex-digest`/`tex-export`, golden `golden/textures.tsv`, opt-in test with sister. 37 unit + 7 opt-in tests.
- Decided by measurement (neighbour smoothness over every texture): 256-colour palettes are CSM1-swizzled; 4 bpp is
  low-nibble-first.
- **Cross-check** `tools/crosscheck/nfre_textures.py`: nightfire-re's own handlers + depalettizer, their WebP writer
  replaced by per-frame PNG: **27,130/27,130 frames pixel-exact**.

**Failed / corrected**
- My first PS2 smoothness script crashed on the one short texture (0700000d.bin 703/18); now skipped and logged
  (U-017).
- The cross-check caught a real bug: `tex-export` numbered textures by position among the decoded ones, so after the
  short texture the file names in that map were off by one. `Texture` now carries its header `index`.
- First comparison run left 385 animated textures unchecked (their writer saves lossy WebP) and 180 "missing" ones:
  their name table spells the folder `Weapons/` once and `weapons/` 64 times, and on NTFS both land in one folder;
  my reverse lookup was case-sensitive. Fixed both in the driver (their decoder untouched).

## 0009 — PS2_GFX static geometry (2026-10-08)

**Done**
- Read the decomp: `parsemap_block_PS2_GFX`, `parsemap_block_entity_params`, `psiCreateEntityGfx` (offset fix-ups,
  texture-REF patching), `RecurseAndDrawBoxes` / `DrawThisBox` (glist box tree; DMA CALL + `MSCALF` 2/0).
- Surveyed every packet: DMA chains of CNT (VIF stream), REF (texture index, zero VIF codes) and RET; skinned
  objects add REF tags with UNPACK codes filled at run time. Batch layout from the data: GIF tag header, V2-32 UV at
  B+0xC7, stride-3 V3-32 XYZ / V4-8 RGBA / signed V4-8 (normal + no-kick w), MSCNT.
- docs/formats/ps2gfx.md; `ps2gfx.rs` replays packets through a VU1-memory model with per-batch freshness checks;
  `disc::for_each_gfx` / `geometry_entries` / `geometry_digest`; CLI `gfx-list`, `gfx-digest`, `gfx-export` (OBJ),
  `gfx-boxes`; golden `golden/geometry.tsv` + opt-in test with sister. 40 unit + 8 opt-in tests pass.
- Checks: box min/max = decoded vertex bounds bit for bit (24,537 leaves; U-019 closed); visual (gun emplacement and
  mounted gun from three axes); **nightfire-re: 6,227/6,227 unique static objects identical** (triangles and UV
  corners), the 9,183 shared copies identical by our geometry SHA-1.

**Measured**
- Box +0x30 equals the decoded triangle count in 16,749 of 17,030 static objects (U-020).
- 644 leaves have a packet with a setup kick but no vertices.

**Failed / corrected**
- **Pairing bug:** I first paired each PS2_GFX with the entity-params block *before* it. The cross-check showed every
  object matching the previous one's name; the decomp (`entity_params` builds the gfx already stored in the cel)
  shows params *follow* their gfx. Fixed, and the reader now rejects any other order.
- Box +0x30 is not a vertex count (first hypothesis, 15,410 mismatches); nor the kick count; it is the triangle count
  for 98 %.
- Cross-check driver artefacts, each found by inspecting a single object: crashed on stray PNGs in `work/gfx`; UV
  comparison assumed their `1 − v` was invertible (not for v ≈ 1.7e-16) — now applies their transform to ours;
  parsed our shortest-f32 decimals as doubles — now via f32; NaN ≠ NaN (U-022); names with a trailing space lost
  it in my header split; their writer turns "/" into "__". Their decoder was never modified.
- Shell/Python edits of Rust strings hit the `\\` collapse twice more (a `sed` and a Python heredoc silently did not
  match); redone with the editor. CLAUDE.md already forbids it; I must stop trying.

## 0010 — World placement and the first viewer (2026-10-08)

**Done**
- Read `parsemap_block_map_data_static`, `parseentity_transform_bounding_box`, `RotMatrix`, `Mat_SetTrans`.
  docs/formats/placement.md; `placement.rs` (records, `object_matrix` with the decomp's exact entries); `level.rs`
  (archive assembly, index/hash resolution); CLI `level-summary`, `level-export` (world OBJ); golden `levels.tsv` +
  opt-in test (also asserts every placement resolves). 45 unit + 9 opt-in tests pass.
- Survey: 23,329 records; every block ends exactly; all index and hashcode references resolve (all 36 levels).
- With the user's approval added winit 0.30.13, wgpu 30.0.1, pollster 1.0.1, glam 0.34.1 and wrote `nf-view`:
  world-space buffers per material, GS-modulate shader (vertex colour 0x80 = 1.0), free-fly camera with raw mouse
  deltas → yaw/pitch, headless `--shot`. `png.rs` moved from nf-extract into nf-formats for screenshots.
- First render of level 07000001 from the `Player1` start: recognisably correct interior (textures, baked vertex
  lighting, alpha cut-outs, courtyard through the windows); 331 instances, 57,491 triangles, 199 materials, < 1 s.

**Failed / corrected**
- The geometry golden from WORKLOG 0009 had been generated before the params-follow-gfx pairing fix, so it failed
  now; the per-archive counts were identical and only the row hashes (names) changed. Regenerated. (I should have
  regenerated it right after that fix.)
- wgpu 30 / glam 0.34 API differences fixed from their sources: `Instance::new(InstanceDescriptor::
  new_without_display_handle())`, `queue.present(frame)`, `Option` vertex-buffer layouts; glam 0.34 moved camera
  helpers, so the viewer builds its projection and view matrices explicitly.

**Open**
- Handedness/yaw vs the game (U-023): needs a comparison with PCSX2.

## 0011 — Mission order; viewer start fix (2026-10-08)

**Failed / corrected**
- I told the user 07000001 was "the first level" because of its archive number. The user corrected me. The game's
  own `MissionData` table (0x2a4350, 24 × 40-byte records, docs/formats/mission-data.md) lists 07000005–08 first,
  then 07000001–04. Archive numbers are not play order; table order is assumed to be play order (U-024).
- My first attempt to read `MissionData` assumed 24-byte records (the hashcodes lined up by chance); the raw dump
  shows 40-byte records.
- nf-view without `--all` started inside the sky: `Player1` is not a 0x8000 cel, so the start fell back to a bounds
  camera. The start now always comes from the placement records.
- The user's `cargo run` failed with "could not find Cargo.toml": I had given a command that only works from the
  repo folder. Now given with `--manifest-path`.

**Measured**
- 07000005 (first mission in the table): 132 static cels, 43,718 triangles; with dynamic entities 363 instances.
  Striped panels remain among the static cels (U-025); flare sprites need blending (G-018).

## 0012 — Blockers hidden by the game's cel-pass rule (2026-10-08)

- The user confirmed in PCSX2 that the castle is not mirrored (U-023 partly answered).
- Listed 07000005's static cels: every blocker (`blocker1`, `Blocking_*`, `BlockingPoly*`, `blocker_sk*`) has bit
  0x10 in the entity-params word at payload +4 (= celglist+0x44 = cel+0x94). `View_AddCels` skips any cel whose glist
  flags have 0x10. `Room_Sky` has it too and is drawn by `View_DrawSky` instead. Over all levels no second bit
  separates skies from blockers, so nf-view applies exactly the game's cel rule and leaves the sky for G-019.
- `MapObject::flags` / `skipped_by_cel_pass`; nf-view hides those cels: 18 in 07000005, striped panels gone.

## 0013 — Characters: skinned meshes in bind pose, skins, drones (2026-10-08)

**Done**
- Read `CacheSkin`, `SkinIt`, `psiDrawSkinObjectMatrix`, `AnimLoadFile`, `AnimProcessSkinData`, `AnimSkeletonProcess`.
- ps2gfx.rs: skin REF tags read their data from the source vertex buffer (signed relative address); only inline
  unpacks may set the batch header; skinned batches keep the raw packed word. All 582 skinned leaves pass every VU
  freshness check; box min/max = decoded bounds bit for bit in all 582 → model-space bind pose.
- character.rs (skeletons, skins in loader order), `Level::skins`, `character_instances` (oDrone param 0 → skin →
  main meshes); nf-view draws them: 16 guards in 07000005, T-pose, feet on the floor.
- Goldens: geometry.tsv regenerated (per archive object/skinned counts identical; skinned rows now carry geometry:
  totals 7,154,267 vertices / 4,697,239 triangles). nightfire-re re-run: static objects still 6,227/6,227 identical;
  the 115 new unmatched are exactly the unique skinned objects (their tool does not export skinned meshes).
  47 unit + 9 opt-in tests pass.

**Measured**
- Skinning word bit 15 = no kick (51,664/51,664 strip starts); upper half-word = morph index.
- 1,217 skins parse; main meshes: 582 → skinned objects, 627 sleeve substitutes. Placements with a skin in
  parameter 0: 522 oDrone (skin present), 33 oDrone (skin elsewhere), 8 Tree.

**Failed / corrected**
- Kick bit: I first read the dumped bytes `00 80 ff ff` as the number 0x0080FFFF and used bit 23 — 63,571 triangles
  from 1.48 M vertices, obviously wrong. The bit histogram showed bit 15 (the word is 0xFFFF8000 little-endian).
- My first pass assumed `oDrone` positions might need a height offset; the render shows feet on the floor, but this
  is not verified against the game (U-026).
- Edits that replaced `"…" => ` with text ending in `=>` (no space) had glued two CLI match arms; fixed with the editor.
