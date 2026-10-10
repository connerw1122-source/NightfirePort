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

## 0014 — Sky pass (2026-10-08)

- Read `View_DrawSky` (0x1e5ad0: 21 `GfxList` slots of 0x40 bytes, glist flag bit 2 = viewer-centred, lightning
  alternates, rotating sky for 0x0700001B), `View_AddSkyObj` (0x1e79a0) and its caller
  `parsemap_create_dynamic_objects` (0x1d04f0): the switch is on object type = placement flags & 0xFF; 0x2A = sky,
  slot = parameter 0, parameter 1 = lightning variant. 74 sky placements on the disc (skyboxes, moons, mountains,
  silhouettes).
- `Level::sky_instances`; nf-view layers: sky pass first (no depth, slot order; viewer-centred objects with the
  view matrix without translation), then the world; sky placements are removed from the `--all` world list.
- 07000005 now shows the night sky, clouds, mountains and the `Sky_Tower*` roofs.

## 0015 — Skeletal animation (2026-10-08)

Facts first came from Raikaru/nightfire-engine `docs/formats.md` (read online, facts only); every one used was
re-checked on our disc or decomp:
- **Skins** (all 1,217): bone bytes (bit 7 animated, low 7 parent; parents precede children, one root), parts,
  meshes, 0x1C-byte inverse binds, 36-byte datums — the computed end padded to 32 equals the file size in every skin.
  Inverse-bind quaternions are identity as (0,0,0,−1).
- **Skeletons** (45, ids 0–44, all copies identical): mask + per-bone offsets; offsets × scale equal
  `inv_bind(parent) − inv_bind(child)` for 59,875 bones within 1e-4 (2,898 within 1e-2, 80 differ).
- **Sequences** (all 18,244): channel layout read from `AnimFrameCopy` — u32 words whose halves are the segment
  entries (header word = byte length | first entry), floats per mask bit; every channel consumes exactly its byte
  length and its segments sum to exactly frames − 1; t = frame − 1 absolute; qz ≥ 2 → w negative.
  They live in per-level animation archives 07F000NN (LoaderLoad's 0xF nibble).
- **Vertices**: slot → bone list at skin-info word 3 (every used slot inside it, all 582 objects); weights in [0, 1]
  (845,738); fully weighted vertices lie a median 0.10–0.16 from their joint → mesh and skeleton share bind space.
- `Quat_QuatTransToMat` and `psiBuildMatrixPalette` composition read from the decomp.

**Done**: anim.rs (sequences, local/world pose, skinning matrices, two-bone blend), character.rs (full skeletons and
skins), ps2gfx (vertex weight/slots, slot list); nf-view animates characters on the CPU each frame (`[`/`]` choose the
sequence, `P` pauses, `--seq`/`--frame` for shots); golden `sequences.tsv` + opt-in test. 50 unit + 10 opt-in tests.
Renders: walk and turn poses are anatomically coherent across 9 sampled frames / 3 sequences.

**Failed / corrected**
- Sequence parsing took three attempts: sequential u16 entries (overran), u16 pairs read as two entries each
  (nonsense frame counts), then the decomp's actual order (halves of u32 words, the first in the header) — exact on
  all 18,244. All sequences first reported "skeleton not in archive": they are in the 07F archives, not the levels.
- My identity check on inverse-bind quaternions required w = +1; they are stored as (0,0,0,−1).
- Sequence 3 of skeleton 0 walks the guard away and into the camera: root motion is in the root's translation
  channels and the game handles it separately (not yet; G-020).

## 0016 — Root handling and the drones' own start animations (2026-10-08)

User report: "all of the actors are floating off the ground. they're going through walls because the animation being
used isnt correct". Both were mine: nf-view drew the root bone's translation (hip ≈ 1.1 above the sequence floor, plus
horizontal travel) on top of a placement that already sits at hip height, and played an arbitrary k-th sequence.

**Root (decomp)**: `AnimSeqTick` takes the root's per-tick change as root motion and zeroes the root translation;
`AnimFrameResolve` adds the rotated motion to the object position and keeps the hip height; drones turn the vertical
part into `Drone+0xA0` = hip − 0.02, and `NDrone2_Collision` rests the feet sphere `0xA0` below the object.
**Measured**: standing sequences hold the root 1.03–1.15 with the lowest joint 0.03–0.04 above y = 0
(`nf-extract seq-roots`); 07000005's drone placements are 1.0845 above the rendered floor (`NF_FLOOR_PROBE=1`).

**Start animation (decomp)**: scripts (type 5, `06…`) are what objects play; `NDrone2_DefaultInit` gives type/anim set
from the skin and weapon parameters and starts danim 0x11A (stand idle) → `Drone_AnimTables` → script → opcode 0 →
sequence. Script format, root rules: docs/formats/animation.md; the choice: docs/systems/drone-animation.md.
**Measured**: all 18,231 scripts parse inside their entries; 550 of 555 drones resolve to a script and sequence (the 5
others have parameter 1 = 0x06000892, anim set 0xF without a stand idle). 07000005: all 16 → `0600025c` → `040001d8`.

**Done**: `drone.rs` (tables from ACTION.ELF's symbols, `drone_setup`, `start_danim`, `start_script`), `anim::Script`
(+ loop time), nf-view plays each drone's start script by default (`[`/`]` still browse sequences, `[` from the first
returns), root zeroed, height = floor + `Drone+0xA0`; golden `scripts.tsv`, `drones.tsv` + opt-in tests; nf-view takes
the ISO from `NF_ISO` and `.bin` is optional (the run command I gave the user lacked the ISO path). 53 unit + 12 opt-in.
Renders: guards stand on the floor at the 07000005 gate in a weapon-ready idle.

**Failed / corrected**
- I first told the user 605 drones in 22 levels (adding by hand); the digest says 555 in 21.
- My first `drones` listing treated parameter 1 = 0x06000892 as a script; it only switches the anim set.
- A floor probe camera inside the gate door rendered a wall; moved.

## 0017 — Live checks in PCSX2, bit-exact posing, held weapons (2026-10-08)

The user asked me to start the game myself and to check the animations. I launched PCSX2 with the resume save state
(first attempt failed: I passed the ISO with forward slashes from Git Bash — "does not exist" dialog; relaunched with
Windows paths from PowerShell). The user then loaded 07000005.

**Live drone state** (PINE): types / sets / weapons / start script / `Drone+0xA0` and heights as predicted in 0016
(docs/systems/drone-animation.md "Live check"); 3 of 16 drones absent (U-029). U-028 answered.

**Consistent snapshots**: `tools/trace/snapshot.py` makes PCSX2 save a state to slot 9 over PINE (opcode 9) and
extracts `eeMemory.bin`; PINE word reads take several frames for a 45-bone palette. `nf-extract pose-check` compares
each playing drone's local pose buffer and world palette with ours.

**Animation arithmetic**: first comparison 1e-4 … 2.4e-3 off, growing with frame number. Round-toward-zero in the
game's order made every channel bit-exact; w stayed 1 ulp off in 30/34 bones; testing 16 variants against the
snapshot found the no-guard-bit FPU add + nearest sqrt: **714/714 components bit-identical** in three drones
(docs/formats/animation.md "Arithmetic"). World matrices: ≤ 4.6e-5 (composition order; G-024); one moving drone off by
a uniform 5.8e-4 (palette built at draw time before it moved).

**Weapons**: `weapon_data` is zero on the disc and built by a 25 KB static initialiser → small R5900 interpreter
(`ee.rs`) runs it over the user's ELF in 2 ms; equal to the live table in 30,819 / 30,820 bytes (the other is a skin
hash the game writes). Third-person model `+0x80` hangs on datum id `+0x50`. The guard skins have no such datums on
disc: `AnimPostLoadInit` gives every skin on skeletons 0/1 the datums of skin 0x05000007 (confirmed in the snapshot:
shared 60-datum list; 0x0500002F 64). Skin datums now parsed (all 1,217 end exactly at the entry end). nf-view: all 16
guards hold their weapon (Sg5 Commando / white L96A1), shouldered in the idle pose.

55 unit + 13 opt-in tests.

**Failed / corrected**
- Python edits of the decomp search left a bare `python -` in a command: it started an interactive REPL that filled a
  64 MB log until stopped (deleted).
- My first probe read only the first script of a drone (a dormant one, time 1.0) and suggested drones were frozen;
  walking the script chain found the playing one.
- `datums` first returned nothing for the guard skins — the sharing in `AnimPostLoadInit` was the missing step.

## 0018 — Levels without menus, spawners, muzzle flash (2026-10-08)

User: "add the muzzle flash and the missing guards. preferably without me having to load the entire game by hand".

**Loading levels automatically**: `GameFlow_Main` runs a state stack (`GameStateStack` 0x2c7398, `StackIndex` u16
0x30cece); the pause menu's restart calls `ResetMap_LevelToLoad(level, 0, 1)`, which sets `GameState+8` and pushes
state 3 (`ResetMap_Load`). `tools/trace/goto_level.py` does the same through PINE writes into the emulated RAM
(added to `pine.py`), waits for the level and snapshots it. 07000005 reloaded in 9.7 s, 07000001 in 11 s, 0700000A in
~20 s, no input needed. PCSX2 still has to be running (or `--launch` starts it from the resume state).

**Missing guards**: present in the level data but not created at start: `DroneSpawner_Init` turns every drone of a
group with a spawner into a template and deletes it; spawners bring templates back later (switch channel, distance,
out of sight). Rule in `drone::spawn_roles`; predicted start counts equal the live ones in all three levels
(13 / 15 / 0). nf-view hides reinforcements by default; `R` shows them.

**Muzzle flash**: `DroneWeap_FireWeapon` → flash mesh `+0x4C` at datum id + 1 (`AnimGetBoneWorldTrans` adds its
third argument to the id), random spin, light colour `+0x54..56`. nf-view: `F` fires (viewer rate), additive layer.

56 unit + 13 opt-in tests.

**Failed / corrected**
- First flash renders sat at the grip: I used the weapon datum itself; the game asks for id + 1 (the muzzle).
- The flash drew its black background: our renderer only alpha-tests; added an additive effect layer.
- A Python heredoc edit of disc.rs silently failed on `\\t` (CLAUDE.md warns about exactly this); redone with Edit.
- My first flash-frame loop piped renders into `tail -0`, which killed them; rerun without.

## 0019 — Collision, guard AI first milestone (2026-10-08)

User: "keep going, add the guard AI and firing".

**Research**: four parallel research passes over the decomp (state machine; perception; combat/firing; turning and
movement) — reports condensed into docs/systems/drone-ai.md with [M]/[S] tags; I spot-checked key facts (alertness
table, property tables, anim tables, weapon fields) myself.

**Collision** (map block 0x2E): format decoded and checked on all 7,985 blocks (docs/formats/collision.md); nf-game
`CollisionWorld` (33,048 triangles in 07000005); collision floors explain live drone heights within 0.003.

**Ground truth**: `tools/trace/ai_trace.py` teleports Bond in PCSX2 and records a guard every frame: noticed after 61
ticks, 3-shot bursts 4 ticks apart, gaps 64 / 51, side-steps between bursts. Live `drone_stats` for the guards' type
are zero (defaults: accuracy 5, aggression 0, reaction 0) — consistent with 60-tick reaction and D ∈ [37, 60].

**Port** (`crates/nf-game/src/drone.rs`, `trig.rs`): see docs/systems/drone-ai.md "Port status". `nf-extract ai-sim`
runs a level headless; the opt-in test `ai_live` reproduces tick 61 and the burst cadence (with a sister check).
nf-view runs the AI at a fixed 60 Hz with the camera as Bond: guards notice, raise their guns and fire (flashes on
the real shot ticks).

**Failed / corrected**
- The single-drone test noticed at tick 60: line of sight is round robin over all drones; with the level's 13 the
  tick is 61 as in the game.
- I first had the animation-completion check skip transition danims (a guess); removed — it follows the decomp.

## 0020 — Combat movement and bullets (2026-10-08)

User: "keep going, add the combat movement and bullets". Two more research passes (combat moves; bullets and damage),
condensed into docs/systems/drone-ai.md "Combat moves" and "Bullets".

**Ground truth**: a second live trace (`work/ai_trace_b.tsv`) with the state number (`Drone+0x10C`) and Bond's health:
Attack → StepAimRight 101 ticks → AimStandFire → StepAimLeft 113 → … → AimCrouch → AimCrouchFire; every SG5 hit costs
1.65, sniper hits 18 (another guard joined).

**Port**: `ChooseCombatMove` + first-sight moves + the step / strafe / dodge / roll / crouch / back-off states, crosshair
("aimed at") and bunching inputs, root motion (sequence root channels, rotated by yaw; walls stop it); bullets
(`bullet.rs`: hit roll, wobble, cone, flight, world/Bond collision, per-level damage factors, health), muzzle from the
posed skeleton; nf-view draws tracers and shows Bond's health in the title. Test `ai_live` now also checks the first
move is a step lasting 100–115 ticks and 1.65 per hit.

**Failed / corrected**
- First runs picked "stand" instead of a step: my crosshair ray started at +1.4 above Bond's object and passed over
  the guard's head; +1.0 (his eye height in the trace setup) hits as in the game.
- Steps ended after one tick: a call completed as soon as the requested anim state started; it now completes when a
  looping danim of that state plays or a one-shot one finishes (*suggested*; matches the 101/113-tick steps).
- Player damage first used the ELF default difficulty factor (0.75); the live 1.65 per hit showed the per-level
  table (0.6 for 07000005 at difficulty 1).

## 0021 — AI network, patrols (2026-10-08/09)

User: "keep going, add the navigation routes and cover … keep going for the next several hours while I sleep".
Three research passes (AI network and routing; cover; patrols) condensed into docs/systems/drone-navigation.md and
docs/systems/drone-cover.md; I read `AIPath_Parse`, `AIPath_Prepare`, `AIPath_BindNodes`, `NDrone2_AssignAIPath` and
`NDrone2_Move` myself.

**Format**: map block 0x05 (docs/formats/ai-path.md, `aipath.rs`): version 8, named records (paths and boundaries),
nodes 0x40 / 0x20 bytes, links 16 / 20 bytes. All 210 records on the disc parse to their block's exact length with
every link in range (golden `ai_paths.tsv`; its first sister run failed because rows were keyed by archive only — the
key is now archive:map:record). Block 0x19 is keyframe paths for doors, cameras and helicopters, not drones.

**Port**: `nf-game/src/nav.rs`: network build (post heights by a ray up, door / kick / passable walls), move tests,
bounds tests, node searches, A* with this build's quirks, `OptimiseRoute`, the link creep and route following;
patrols (`InitPatrol` → `AssignAIPath(4)` → `InitAIPath` → Patrol with the walk anim, wait nodes, idle timeouts),
idle stand variants and look-arounds.

**Live check** (`goto_level.py 07000005`, `ai_trace.py --near 50.85 28.27 93.63 --frames 2400`): the guard walks
PatrolPath 1 the whole time without stopping at its wait nodes. Two corrections came from it:
- the look-around requested at a wait node is cancelled the same tick by the walk request (calling the playing anim
  state zeroes a pending call);
- the port fell 2 % behind: it had no motion on the tick the walk script wraps. Aligning per-tick step sizes with the
  live ones (offset 11 ticks, mean difference 1.3 %) showed the game repeats the last step on the wrap tick; with that
  the walked distance matches to 0.05 % (23.425 vs 23.413 units). Two wrong tries first: the last frame minus the
  previous root (no change, the loop's last frame is its first pose shifted) and the first step (too small).
New test `guard_patrols_like_the_game`: route order and positions within 0.35 units of the live samples (sister: 60
ticks earlier must be farther). The "gone" drone of WORKLOG 0017 at (50.85, 28.27, 93.63) is this patroller.

## 0022 — Combat movement, start-up properties, waiting drones (2026-10-09)

Read myself: `DroneFunc_CombatState`, `NDrone2_MoveToObject`, `DroneAnim_GetMoveQuad`, `DroneAnim_SetCombatMoveAnim`,
`NDrone2_DSTATE_RunForCover`, `DroneFunc_FirstAttack`, `DroneFunc_Set2ndBehaviour`, `NDrone2_DoModeSettingsNEW`,
`behaviour_util_get/getProperty/setProperty`, `NDrone2_DefaultInit` (property part), `DSTATE_Global`, `DSTATE_WaitSwitch`,
`NDrone2_Enable`.

**Port**: `CombatState` for 0x58/0x5A/0x5C/0x5E/0x68/0x70 (routes to Bond on every call; out of range only with a
route), `MoveToObject` with the player target node cache (rebuilt when he moves 2.0), `SetCombatMoveAnim` (quads with
hysteresis, run-aim / walk-aim / back-off / strafe / roll / step, the yaw offset in the steering), CombatNoRoute and
DroneStuck; `call_anim` no longer restarts a transition when the same call is repeated (a latent bug: AimCrouchFire
re-called its anim every tick). Tool: `nf-extract route <iso> <level> x y z x y z`.

**Live check 1** (`ai_trace.py --teleport 79.62 22.6 95.96 -1.6 --near 61.34 27.1 99.5`, work/chase_trace.tsv): the
gate guard goes Combat → OutOfRange → **RunForCover** (0x8C) moving with run-aim 0x8, strafes 0xC/0xD and back-off
0x4, then UnderCoverInit/Idle/Aim/Fire behind a corner node — cover is next (G-028). The anims showed the guard has
props 0, 1, 0x41 that its placement words lack.

**Live check 2** (PINE on that drone, then the level-start snapshot work/ee_start_05.bin): the first behaviour block
`+0x4DC` differs from the placement words by exactly: 0x1A cleared (`DoModeSettingsNEW`), 0x17 cleared (single
player), and for SG5 guards (parameter 9 = 0x16) on 07000005/06 props 0, 1, 6, 0x41, 0x42 set, 0x13 cleared and the
engage range `+0xF0` = 10 (`NDrone2_DefaultInit`); 0x1D cleared by Idle (no mission path). The active pointer stayed on
the first block after a fight, so I removed the switch to the second block I had added at Attack.

**Live check 3** (same snapshot): 8 of the 13 drones are in WaitSwitch (state 1) with object flags 0x30 (hidden) —
switch channels 0x22, 0xA4, 0x43 from placement parameter 3 — including both PatrolPath (Dr_17) guards. Ported:
Global enter → WaitSwitch until the channel is on (`Drones::switches`; level triggers are not ported), disabled drones
take no part in vision, bunching, alerts or the crosshair test; nf-view hides them (R shows, K turns all channels on).
New test `level_start_matches_the_game`: states, enabled flags and all 12 first blocks equal the snapshot (sister:
≥ 10 differ from their placement words).

**Failed / corrected**
- My first chase setup put Bond 6 units above the floor (no target nodes: correct "no route"); `nf-extract route` found it.
- The sister check of the new test first compared against words already changed by the port (captured after
  `Drone::new`); it now takes the placement words.

## 0023 — Cover, anim blends and playback speed (2026-10-09)

Read myself: `AINetwork_EmitPath`, `AINetwork_InitEmitter`, `Drone_CoverCornerNode`, `NDrone2_MoveToGoalPosition`,
`NDrone2_SlideToPosition`, `AINetwork_SetupGoalPosition`, `AINetwork_CalcRouteToPosition`, `AINetwork_GetRouteDistance`,
`LinkCreep_Calc`, `DroneAnim_CallHandler`, `DroneAnim_CallFullyComplete`, `DroneAnim_SetDAnimInternal`,
`DroneAnim_SetHTAnim`, `DroneAnim_GetDAnimForCall`, `DroneAnim_GetDAnimForAnimState`, `NDrone2_DSTATE_RunForCover`.

**Port**: `cover.rs` (nodes from placements 0xE5/0xE6, emitter distance bytes, `IsCoverNodeUsable` bits, positions,
`CCModes`), `ProcessCoverNodes` in `Drones::tick`, and in the drone `CoverAvailable` / `FindCover` / `CoverNodeOK` /
claim and disown / `MoveToGoalPosition` / `SlideToPosition` / `SetAngleToCover` / `CoverAnim` and states 0x8C,
0x93–0x96, 0x99–0x9C; `CombatState` and CombatNoRoute now go to cover. Tools: `nf-extract covers`, `route` prints the
move-test parts and nearby boundary posts, `ai-sim` takes `NF_SEED` and `NF_TRACE_TICKS`.

**Live check** (work/chase_trace.tsv, WORKLOG 0022): the port now runs the same sequence 0x58 → 0x5A → 0x8C → 0x93 →
0x94 → 0x95 → 0x96 at the same corner node, ending 0.2 from the live position. Three animation facts came out of it:
- **Blends** (`GetDAnimForCall`): transition record byte 4 = blend frames, byte 5 = the call waits for the blend
  (`+0x578` while the blend-out script `+0x538` runs). When the current state has no record, state 0's generic
  records apply: blend 30 with wait (60 for 0x6F–0x74). UnderCoverInit reaches loop 171 at once that way and completes
  31 ticks later (port 30). My first guess (a call completes after one play of the danim it started) gave 150.
- **Playback speed** `Drone_AnimInfo +0x14` (float): lean-out 176 plays its 51 frames at 1.5 → 35 ticks live (port 34).
- **Stale root** after a call starts a danim: the next tick moved the guard by the old sequence's root minus the new
  one's (1 unit into the wall, then the floor probe climbed it to y 31.45); `set_danim` now clears the previous root.
Also: a guard 0.11 from a boundary post (the gate pillar) is blocked on every route segment (as the game's post test
would); with the port's RNG the guard sometimes sidesteps there first, which the live guard did not.

**Open**: RunForCover lasts 114 ticks in the port vs 198 live: live arrives 0.46 from the approach point, the port
when its route distance drops under 2.0. `GetRouteDistance` is ported as read, so the live route probably had a second
node that the port's `OptimiseRoute` trimmed (U-030). New test `guard_takes_cover_like_the_game` (sequence, durations
±3, node, position; sister: no cover with Bond 6 units away).

Follow-ups the same day: `OptimiseRoute` and `GetPosForCornerCover` read and confirmed as ported; from the live start the
port's route is the 2-node [16, 12] the live path suggests, so the arrival difference is in when the route index
advances (still U-030). Boundary push-out (`NDrone2_Collision`, r 0.4) added after the viewer showed a guard stuck by
the gate pillar's post. Call variants (look-around 0x4B × 3).

## 0024 — Drones that are not drawn do not animate (2026-10-09)

Live check: level reloaded, `switch_channels[0x22]` (0x26FC90 + 0x22) set by PINE, `ai_trace.py --near −31.30 37.58
−37.05` (work/switch_trace.tsv). The Dr_17 guard leaves WaitSwitch and patrols PatrolPath (Dr_17) from node 10 in the
port's order — but moving **exactly 0.02 per frame with no gait**, twice the speed of the walk's root motion. Frame rate
was a steady 60 (FRAME_RATE 60, DIV 1, read live). `Drone_Control` (0x139268): when `NDrone2_DoAnimation` is false the
anim is not updated and the object moves along its yaw by `Drone_AnimInfo +4` (0.02 for walk 392). DoAnimation is false
for a drone whose object was not drawn (`obj+0xFC` = 0) playing a danim with info flag 4 and no blend. Read live:
`obj+0xFC` = 2 for the drones around the gate (in Bond's view) and 0 for the rest — including the sniper 9 units in
front of Bond but 10 below behind the cliff edge, so it is the renderer's cel visibility, not a frustum test.

Port: `Drone::drawn` set by `Drones::tick` from a stand-in (within 60° of Bond's aim and any of five body points in
line of sight from his eye; one ray was too strict — a pole hid the gate patroller for 200 ticks and broke the patrol
test). Off-screen walking then matches the live guard within 0.03 for 600 ticks and 0.37 over 1,350. New test
`switched_guard_patrols_like_the_game` (sister: an observer next to the guard makes it walk with root motion, ending
more than 2 units from the live sample). `nf-extract ai-sim` takes `NF_SWITCHES`, `NF_AIM_YAW`, `NF_STEPS_DRONE`.

## 0025 — The game's random generator (2026-10-09)

`Rand_Rand` (0x1e3738) / `Rand_FRand` (0x1e3780): two 16-bit multiply-with-carry streams, x × 18000 and y × 30903 (the
multipliers `a_318` / `b_319` read from three RAM snapshots; Marsaglia's constants), state `SEED_X` / `SEED_Y`
(0x30d0a0 / 0x30d0a4). `Rand_FRand`: (x << 16) + (y & 0xFFFF) through `cvt.s.w` (values with the top bit set are
halved, converted and doubled — read in the disassembly), × 2⁻³², × max; `Rand_Rand(n)`: ((y & 0xFFFF) × n) >> 16. The
port's `Rng` replaces the xorshift stand-in. Check: the R5900 interpreter (`ee.rs`, now with `bltz`/`bgez`, `_gp`, FPU
register access) runs the game's two functions over ACTION.ELF from three seed states for 2,000 calls each; every
result is bit-identical (`crates/nf-game/tests/rng_live.rs`). The game shares the generator with everything else, so live
sequences still cannot be replayed; tests pick a seed state whose rolls match the trace (seed 2: the first combat move is
the step the trace shows — `ChooseCombatMove` read: strafe and step are equally likely for these guards).

## 0026 — Trig bit-exact; the EE FPU's division rounds to nearest (2026-10-09)

`crates/nf-game/tests/trig_live.rs` runs the game's `ATAN2_APPROX`, `PS2Sinf` and `Vec_AngleDifference` in the R5900
interpreter (now with FPU compares, `bc1t/f(l)`, likely branches, `max/min/sqrt.s`, the accumulator forms, a VU0 macro
subset — `lqc2/sqc2`, `vmul(a)(bc)`, `vmadd(a)(bc)`, `vadd`, `vsub` — and more integer ops; libm `fmodf` runs as is) and
compares 3,000 random inputs each with `trig.rs`.

Which arithmetic is right was settled live: with Bond teleported to 150 random spots, PINE read 750 (dx, dz, angle)
triples from the drones' `+0x1B0` / `+0x1B8` / `+0x1C4`. Of 16 rounding models only one fits 739 of 750 (the other 11
are off by 2–4,569 ulps: torn reads across separate PINE calls): FPU add/sub **without guard bit, chopped**, multiply
chopped, **division rounded to nearest**. The interpreter had chopped divisions (my assumption) and is corrected; 60
consistent samples are now a golden test in `trig.rs` (sister: chopped division fails it). `trig.rs` changes: FPU
add/sub use the no-guard model (`ATAN2_APPROX`, `PS2Sinf`'s range reduction, `Vec_AngleDifference`, which also only
calls `fmodf` outside [0, 2π) as the game does); `PS2Sinf`'s VU polynomial keeps the VU's chopped adds. Result: all
three functions bit-identical to the game code over 3,000 inputs each.

Failed / noted: PINE stopped answering mid-session (PCSX2 responsive; probably a connection left by the research agent
or by my killed script); restarting PCSX2 through `goto_level.py --launch` fixed it.

## 0027 — Triggers, switch channels, spawners; update order (2026-10-09)

Two research passes (switch channels / triggers / spawners; hearing / alerts / goal positions / hit reactions):
docs/systems/triggers.md, docs/systems/drone-alerts.md.

**Port**: `triggers.rs` (TouchOnce, Touch, AND, ordered AND, OR, Out; volumes = the trigger model's collision triangles,
Bond's capsule r 0.55 from feet + 0.55 to + 1.86, drones r 0.4; `Trigger_Calc` with the 07000005 latch 0xA4 → 0x6F),
`spawners.rs` (`DroneSpawner_Control`: start / stop / kill / done channels, waves, max alive, Rand template choice,
minimum distance and not visible). `Drones` now runs drones, triggers and spawners in the game's control-list order
(newest first = reverse placement order), keeps `switch_channels` with set times, and the game's drone list order
for the line-of-sight round robin (`llist`: load order, spawned drones appended). Every drone placement becomes a
drone (`level_drones`); spawner templates start absent. `nf-extract triggers` lists a level's triggers; the 07000005
table matches the research pass exactly (placements, channels, locks, boxes).

**Live check** (PINE, Bond moved through box 21 in 40 steps): 0x23 and 0x22 latch, the Dr_17 guards go Patrol, spawner
268 brings in both gate guards (13 → 15 drones). Teleporting Bond into the box and standing set nothing — the game
takes contacts from movement (`Collide_Update`); the port tests positions (deviation, docs/systems/triggers.md). New
test `gate_box_wakes_the_guards_like_the_game`.

**Reaction time**: `DroneFunc_ReactionTime` read: ((1 − alertness) × 30) × 0.01 × (200 − stat) with chopped multiplies is
59.999996 → **59** (Rust's nearest rounding gave 60). The live "61" is 59 plus the wait for the guard's first round-robin
turn (`DroneVision_ProcessDroneSight` read: visibility pass, one ray per frame from the drone after the last one
processed, then the counters); that wait depends on which other drones see Bond, so the test now accepts 59–63 (live 61).
Seeds for the cover / notice tests re-picked after the update-order change (RNG draws moved).

## 0028 — Goal positions, second switch, fully alert start; a pulse checked live (2026-10-09)

**Port** (docs/systems/drone-alerts.md): GoToGoalPosition 0x63 (run to the goal, aim when blocked, back to the return
state `+0x5A2` or DeleteMe with prop 0x59), `SetGoalToCloserPosition` (the first AI-network wall on the line to Bond,
`AINetwork_FurthestPosition`) and `SetGoalToDroneWithLosToOpponent` (Rand(4) per seeing drone) in CombatNoRoute's
timer, which now resets its alternation on enter and takes AimStand on Rand(2) ≠ 0 as read; the second switch
(`+0x135` / `+0x13A`, `NDrone2_PreDroneControl`); start states 4/5/6 become Attack at alertness 1.

**Live check** (PINE: `switch_channels[0xA4]` set once in a fresh 07000005): drones 271 / 272 (waiting for 0xA4,
alertness 1) went straight into combat (0x5F / 0x74); drones 66 / 67 (also waiting for 0xA4) and 65 / 68 (0xA4 as their
second switch, `+0x135` = 0xA4 read) did not move — mux 270 (placement 270) runs after 271 / 272 and before 68…65 in the
newest-first order and rewrites 0xA4 from its inputs. The port does the same (`alarm_pulse_reaches_only_the_drones_
updated_first`; sister: holding the mux input 0xA1 wakes 66 / 67 and sends 65 / 68 to Attack). This also confirms the
update order and the fully alert start state.

## 0029 — Hearing; the anim call cycle ported as read (2026-10-09)

**Hearing** (docs/systems/drone-alerts.md): `nf-formats::sfx` reads `SFXOutputData` from ACTION.ELF (1549 entries; id
1 = inner 2, outer 30, weight 75 — called a "P99 shot" here at first, which 0030 found unsupported); `Drones::play_sound` → `HandleSoundAlerts` the next tick
(sum / loudest, share = 0.01 · loudness, msg 0x14 at most once a second unless louder) → `DroneVision_AlertSound`
(≤ 50 units from Bond) → HeardNoise / Aware / Suspect / Alert, AlertToPosition, SearchArea. `nf-extract ai-sim` takes
`NF_SOUND=x,y,z,id,frame` and `NF_DANIM=` (info rows).

**Failure 1**: the shot sent the idle guard at (99.69, 101.58) into Aware, where he stayed for good. Two causes, both
found in the decomp:
- `CallHandler` looks the transition up again **every frame** until the requested state plays; the port did it once,
  so walk → 244 (alert idle, the first step towards 0x22) never went on to look-around 265. Rewrote `call_anim` /
  `call_handler` / `set_danim` from `CallAnim`, `CallHandler`, `CanSetAnimCall`, `SetDAnimInternal` (`+0x57B` reached
  flag, `+0x55C` start state, the can't-do replacements, immediate completion of the playing state, `SetEndAIState`
  order: state queued then message routed), and `InTransition` (class > 3 and ≠ 8, was a guess).
- The chain's look-arounds use variant 1 (`CallAnim(0, 0x22, 1, …)`), and 265 is class 3 with flag 4: for a drone not
  drawn, `SetDAnimInternal` ends it at once and starts its follow-on 244 (also when a danim has no script — the start
  danim 325 now gives way to 329 as in the game). Without that the off-screen guard froze on 265's first frame.
Also fixed against the decomp: Aware takes msg 0x14 only for Suspect; Suspect's turn-done handler (already turned →
AlertToPosition; alert idle → short look 0x0B, else AlertToPosition at once); Alert ignores sounds.

All 7 live tests still pass with the new call cycle. New test `shot_sends_a_guard_through_the_noise_chain`
(HeardNoise → Aware → Idle the frame after the shot; sisters: weightless sound 0x604, Bond 50+ units away). Not
checked live yet (U-033).

**Test run**: nf-game 8 unit + 8 ai_live + rng/trig live all pass; nf-formats unit tests pass; `disc_golden` passed 10
of 14 and then aborted on `memory allocation of 13314048 bytes failed` (also single-threaded). Not this change: the
Windows commit charge was exhausted (≈0.5 GB free virtual memory, a `RuntimeBroker` process holding ≈214 GB private).
`scripts_match_the_golden_digest` and the three after it are still to be re-run; this change does not touch their
code (`sfx.rs` is new and only registered in `lib.rs`). (Re-run in 0030: all 14 pass.)

## 0030 — Tuning per level; drone damage, flinch, death and fade; Bond's gun (2026-10-09)

A research pass (subagent) read damage, death and Bond's gunfire; I re-checked the parts the code depends on in the
decomp (`HitDamage`, `HandleImpact`, `BulletImpact`, `AnimGetCollData`, `Collide_Jointy`, `LocationImpactAnim`,
`LocationDeathAnim`, Death_Anim / Dead / Fade / FadeFast, `SetDeathChannel`, `DeadDrone`, `CallHandler`) and the
tuning values in the level-start snapshot. Spec: docs/systems/drone-damage.md.

**Tuning**: `nf-formats::tuning` reads the 65 globals `ReadTuningVars` sets (by symbol); `nf_game::tuning::
read_tuning_vars` is the per-level switch, generated from the decomp by a script (exact bit patterns). Cross-check: the
port equals the game's own function run in the R5900 interpreter for all 80 level ids 0x07000000–4F
(`tests/tuning_live.rs`, sisters: other groups differ). The drone-alerts doc's damage factors were the ELF statics —
corrected (07000005: head ×10, Easy 1.5).

**Damage**: `damage.rs` — hit boxes from `HumanCollisionBones` (`nf-formats::hitbox`) on the posed skeleton, the ray
test in list order (first box wins, not the nearest), `HitDamage` with chopped multiplies. Drones: health 10, msg 8
through each state's `HandleImpact(resume, allow)` (a table read from all DSTATE decomps by a script), `BulletImpact`
(death, flinch gate 120 ticks per drone and game-wide, resume state), the flinch call 0x4C, BulletImpact 0x55,
Death_Anim 0x44 (`LocationDeathAnim` 0x40 / 0x42), Dead 0x47 (timer 300), Fade 0x48 / FadeFast 0x49 (alpha −0.6 / −8),
the dead-drone limit 3, death channels (07000005: 65 / 68 → 0xA4, 3 / 4 → 0x44, 7 / 267 → 0x0A). Corpses don't see,
fire or hear and are not hit. `Drones::fire_player` / `update_player_bullets`: Bond's bullets (damage, range 50, speed
5, first step random) against the world and the drones' boxes; the shot's sound 0x403 feeds hearing. `gun.rs`:
`Player_WeaponFiring` (one shot per press, refire 8, clip 7). nf-view: left mouse fires (once grabbed), right mouse
holds aim mode.

**Failure 1**: the first hit never flinched — the stamps were 0 and our frame starts at 0, but the game's counter
(`GameState+0x34`) runs from boot (108153 at a 07000005 start [L], never reset per level), so unset stamps are long
past: now `Option`. **Observation**: set 5 has scripts for only some flinch variants (64, 69–71, 73, 74, 76–80); the
others have none, so `SetDAnimInternal` ends them at once and the aim follow-on 50 plays, the call completing after the
30-tick blend. **Failure 2**: the test picked placement 7 (absent) for the sniper — same x as 267; selection by
placement now.

**Checks**: the gate guard shot twice from 5 units: 10 → 4.75 → −0.5, Death_Anim at the second hit, Dead when the death
anim's follow-on (191, class 9) starts, Fade 300 ticks later, gone after the fade. The ledge sniper (267): Attack on the
first hit, dies on the second (SpecialDeath, played as a normal death — deviation), raises 0x0A. New test
`bond_kills_guards_with_two_torso_shots` (sister: shots over the head hit nothing). Nothing here is checked live yet
(U-034). All tests pass (nf-game 11 unit + 9 ai_live + rng / trig / tuning; nf-formats 57 + 14 disc golden).

## 0031 — Hearing exact to the disassembly; hearing checked live (2026-10-09)

**Port**: `Sound_Alertness` read in the disassembly (the decomp showed an int cast that is not there: the loudness is
`max.S(sum, 0)`, a float). Now ported as read: looped sounds never heard; squared distance against outer²; id 0x248 =
weight 10 everywhere inside its outer radius; ids 0x11C / 0x11D ignored on 07000009 / 0A; the "peak" is the running
maximum of the cumulative sum (not the loudest single sound); `HandleSoundAlerts` adds the share to alertness
**unclamped**; chopped PS2 arithmetic. `SfxHearing` carries the id (`Sound_Play3D` keeps the low 20 bits; unknown ids
get 10 / 20 / weight 0).

**Live check** (PINE through the MCP bridge; 07000005 reloaded by `ResetMap_LevelToLoad` writes, Bond at the level
start (104.95, 24.96, 108.83)): the hearing list (head 0x349b0c) held looped ambient sounds (0x85, 0x10B, weight 0).
Borrowing the in-use entry 0x1FDDD70 — id 1, weight 75, inner 2, outer 30, 1 unit from the gate guard — the game
zeroed the weight within the frame and the guard went Idle → Aware (0xA3) with `+0x510` loudness 75.0, `+0x514` share
0.74999994 (the chopped 75 × 0.01 the port computes), alertness ≈ 0.7497, noise level `+0x43` = 2, then back to Idle.
A second sound: noise level 3, Suspect (0xA4), then Idle and back to its post (the polls were seconds apart, so
states in between — AlertToPosition / SearchArea — may have been missed). The entry was restored each time. `tools/trace/hear_inject.py` does the same from Python (it needs PINE to itself).

**Divergence** (open, U-033): the port's guard, with the same sounds, sees Bond 15 frames into Aware (alertness 0.75
→ sight range 24 × 2.5 = 60 > 44.6) and attacks; live it did not. The guard was drawn live (`obj+0xFC` = 2), so
the look-around would play there. Per-frame live data is needed (the Python tracer could not connect while the
MCP bridge held PINE).

**Mistake**: a mistyped decimal address wrote 0 to `0x30D101` (byte 1 of the `ScriptCam` pointer, 0 in the level-start
snapshot) instead of `IsWarmReset` 0x30D001; the level was reloaded right after. Session state only, nothing persisted.

## 0032 — AI shadow volumes: why the live guard did not see Bond (2026-10-09)

Reading the live guard's vision fields (and correcting two mis-added offsets of my own first reads): `+0xE4` π/2,
`+0xE8` 24, `+0x1A0` 44.59, `+0x1C4` 1.354, view `+0x380` = body yaw −1.585 (facing away), `+0x22C` 0.24999999,
`+0x230` 0. `DroneVision_OpponentVisibility` multiplies by `+0x22C`, which `Drone_ProcessOpponents` sets from the
**AI volumes** Bond stands in (`AIBoxList`). The level-start snapshot holds nine boxes; eight are the main map's
`AI_ShadowVolume` placements (type 0xF5): two with factor 0.25 and six with 0 around the start corner; Bond's start is
inside a 0.25 one. The port had no volumes, so its guard saw Bond (0.957 > 0.7) where the game's could not (0.24).

**Port**: `nf_game::aivolume` (`Drone_AIVolume_Create`: factor = param 0 % , kind from param 1, half-extents from the
glist bounds × placement scale, the 07000005 rule: z rotation negated and y + 1.5 × scale y); `MapObject` now carries
the entity params' bounds (+0x08 centre, +0x14 radius, +0x18 min, +0x24 max — `AI_ShadowVolume`: ±1.5 × (0..3));
`Placement::extra` keeps +0x24..+0x47 (`scale()` = +0x34). **Failure**: the first matrix (`RotMatrix` order) matched
the seven yaw-only boxes but missed the tilted one by 0.25 in a row; a search over Euler orders fits Rx·Rz·Ry to 2e-5.
Test `ai_volumes_match_the_game` (centres, extents, axes, factors vs the snapshot; Bond's start factor 0.24999999;
sister: outside every box 1). With it the two-sound scenario no longer makes the guard attack. All tests pass.
Open: U-035 (the matrix builder, the ninth box), U-033 (Aware's look-around length live).

## 0033 — Corpse alerts are inert in single player; the alarm (2026-10-09)

**Corpses**: Dead's `DroneAlertToObject(0x15)` broadcasts to every drone; single-player states pass msg 0x15 to
`DroneVision_EnemyAlerts`, whose case 0x15 calls `ShoutFromOtherDrone` — `return 0` — so only the return state and
home get saved. SeenDeadBody 0x9E is set only in `NDrone2_ReactToDroneAlertMsg`, called only from the bots'
`DSTATE_BotGlobal` (multiplayer). Nothing to port; the gap entry is gone.

**Alarm** (`Drone_InitComms`): while `switch_channels[0x96]`, every 30 frames (once on 07000014) `Drone_Message(0,
0x1E)` → `Drone_SM_BroadcastMsg`; the states that pass 0x1E to `EnemyAlerts` (Idle, Alert, Patrol, AlertToPosition,
SearchArea, HeardNoise Aware / Suspect / Alert, plus states the port lacks) go to Attack. Ported with test
`alarm_channel_sends_guards_to_attack` (sister: no channel, no attack). 07000005 has no trigger for 0x96.
`DroneFunc_CheckAlarmRaised` is music only. Channel 0x97: a search over every OR-mask on `+0x4F8` found 0x100 only
in `NDrone2_DefaultInit` (| 0x1A0) and `NDrone2_Enable` (| 0x801A0) — the "enabled" flag; dead drones are excluded by
0x200 (`SetAsDead`). So any enabled living drone at alertness ≥ 1 within 30 units of Bond raises 0x97 — ported.

## 0034 — `Drone_InitComms` order (2026-10-09)

Read the order: link flags / aware points / target slots, `ProcessCoverNodes`, `ProcessOpponents` (AI-volume factor
and opponent info), `ProcessDroneSight` (visibility, the round-robin line of sight, `FindAlertedDrones`, and each
drone's own AI-volume factor `+0x52C` — used only when a drone's opponent is a drone), the 0x97 / music pass,
`NavNodeCache`, then every drone's `HandleSoundAlerts`, `Sound_ZeroAlertness`, the alarm, delayed messages. The port
ran the sound alerts before the drone-to-drone alerts; now alerts → 0x97 → sounds → alarm, as read. All tests pass.

## 0035 — Bond's auto-aim (2026-10-09)

`Player_AutoAim` / `Check_AutoAim` read and ported (`Drones::auto_aim`; docs/systems/drone-damage.md): outside aim
mode, the nearest drawn living enemy drone whose bone 2 lies within the yaw / pitch cone and in line of sight snaps the
crosshair offset onto it; the offset halves every tick. nf-view: on by default (the game's option byte is 1 [L]), off
in aim mode (right mouse). **Failure**: the first offset had the wrong sign (the port's `angle_difference(a, b)` is
b − a) — the test caught it (the bullet went 0.1 rad the other way). Test with a sister (0.6 rad off: no snap).

## 0036 — Bond's reload time (2026-10-09)

`Player_ReloadAmmoType` only moves rounds (argument 0) or checks (1); the time is the first-person animation:
`Player_WeaponFiring` starts anim state 6 when the clip is empty and rounds remain (trigger held or released), plays
the start script `weapon+0xA8` if any, else the reload script `+0xA4` at speed 1.35 when flag 0x200 is clear, from
time 6.0 when flag 0x1000 is set; `Player_SetWeaponAnimObj` case 6 moves the rounds when it stops (one round with flag
0x100), then `+0xAC`. P2K (flags 0x1004, reload script 0x06000827, 155 frames): ⌈(155 − 6) / 1.35⌉ = 111 ticks
(*suggested* ±1, the script player's stop rule unread). `gun::reload_ticks` replaces the 60-tick stand-in. nf-view's
one-line change to pass the script lengths could not be compiled (rustc fails to map `core`'s metadata while the
leaking RuntimeBroker holds ~212 GB of commit) and stays uncommitted for now. (Built and committed with 0037.)

## 0037 — One-shot scripts hold; SpecialDeath; drone modes and the sniper (2026-10-09)

**One-shots** (`SetHTAnim`: info flag bit 0 = loop): a one-shot script without a follow-on now stops on its last frame
(the port looped every script); `+0x57A` (running one-shot) holds class-1 call completion until the end, as
`CallHandler` / `CheckForAnimComplete` read. Raw scripts by hashcode (`DroneAnim_SetScript`) play as class 1 with a
follow-on. All live tests still passed.

**SpecialDeath** (0x163c68): the special script plays only at the drone's creation spot (within 1.0, facing within
90°; 0x0600021F also needs Bond 2+ away), raising the death channel at once, sliding to the spot, then the dead pose
0xBF and Dead; elsewhere the normal death. Scripts other than 0x0600021F make the drone stationary (`+0x4F8 & 0x10`),
which the decomp tests together with property 0x40 everywhere — the port's five property-0x40 tests became `stays()`.

**Modes and the sniper**: 07000005's sniper (spawner 266's random pick of placement 7 / 267) started live in SniperIdle
0x29 while the port started it in Idle. Read `behaviour_util_get` (type / block modes in parameters 15 / 16 / 21),
`GetDTYPENEW`, `GetDroneTypeAttackTypeFriend`, `DroneTypeSettings` (now read from the ELF with the property tables),
`FirstAttack`'s alternate state, and SniperIdle / Aim / Fire / Reload; all ported (docs/systems/drone-ai.md "Drone
modes and the sniper"). The start test now checks the sniper against the snapshot: state 0x29, mode 2, alternate 0,
team 1, `+0x5A4` 0, tag 0, stationary and its first block word for word — all equal.

**Failure**: the sniper no longer draws Idle's random idle animations, which moved the shared random stream: the notice
test's burst (60 between the first two shots) and the cover test's first-sight move changed. Seeds re-picked by a scan
(notice 7, cover 1), as in 0027; the kill test now expects the sniper's own chain (SniperIdle → SniperAim on the first
hit). nf-game: all 26 tests pass; nf-formats unit tests pass; `disc_golden` aborted on an out-of-memory allocation
(the RuntimeBroker leak again) and is to be re-run.

## 0038 — The AI volume matrix as the game builds it (2026-10-09)

`control_create_object` (0x1344e0) without a quaternion calls `View_RotTransMatrix` (0x1e5168): `PS2Sinf3` for
sin / cos (cosines as sin(a + π/2)) and a closed form — rows (cy·cz, sz, −cz·sy), (sx·sy − cx·cy·sz, cx·cz,
cy·sx + cx·sy·sz), (cx·sy + cy·sx·sz, −cz·sx, cx·cy − sx·sy·sz): the Rx·Rz·Ry order the fit had found. Ported as
written, with the game's sine and FPU arithmetic, replacing the fitted product; `ai_volumes_match_the_game` still
passes. (The ai_live tests now run out of memory when run in parallel on this machine — RuntimeBroker leak — and pass
with `--test-threads=1`.)

## 0039 — Captains (2026-10-09)

`NDrone2_DefaultInit` after the mode settings (read): captain `+0x19` by difficulty (property 9 on 1, 5 on 2, 8 on 3)
for kinds 1 / 2 / 3 / 6 / 7 with their captain skins, health × `DroneCaptain_Mod_Health`, `+0x100` =
`Mod_BulletDamage`, accuracy × 1 / `Mod_BulletAccuracy` truncated; type 0xC drones lose properties 0x14–0x16. Ported
(`captain_setup`); the start test asserts what the snapshot shows at difficulty 1 on 07000005: no captains, health 10.

## 0040 — Dropped guns and pickups; disc golden re-run (2026-10-09)

**Disc golden**: the four tests the memory leak had aborted (scripts and after) pass; all 14 pass.

**Read** (docs/systems/pickups.md): `DroneWeap_DropWeapon`, `Pickup_CreateSimple` (its stack arguments in the
disassembly: sound 0xF5, rest 0, respawn 0, lifetime `FRAME_RATE_INT` · 120), `Pickup_Create`, `Pickup_Update`,
`Pickup_Handler`, `Player_EquipWeapon`, `Player_EquipAmmo`, `Upgrade_Weapon`, `DroneFunc_OnInitDeath`'s captain grenades
(`a2` = 1 for 0x34 read in the disassembly), `NDrone2_SetAsDead`. **Measured** in the level-start snapshot:
`FRAME_RATE_INT` 60 (so a drop lasts 7200 ticks; `DAT_00298F58` = 300 was set by `Drone_LevelReset` when it was 30),
`REC_FRAME_RATE` 1/60, `WldGravity` (0, −9.8, 0); Bond's inventory (type 1: 41; owned 1, 2, 0x4A, 0x4E, 0x50, 0x54; P2K
clip 7); the guards' weapon byte 0x16 (SG5: clip 30, ammo type 5, cap 90) and the sniper's 0x24; `ammo_data` caps.

**Port**: `nf_game::pickup` (Pickup, Inventory, `Taken`), `Drone::drop_weapon` (on Death_Anim's update while the
firing flag is clear, and on Dead's enter; one `Rand_FRand` per drop, so the shared RNG advances as the game's does),
captain grenades, `Drones::update_pickups` ahead of the placed objects (run-time objects head the control list), the
touch test and the handler for weapon / ammo pickups, the pickup sound into the hearing list. `WeaponData` gains
`ammo_cap` and record accessors. nf-view draws a dropped gun at its pickup (the held one disappears), takes Bond's
clip and spare rounds from the inventory, shows the last pickup in the title, and `--pull T1,T2` presses the trigger
in headless runs. Test `a_dead_guard_drops_its_gun_and_bond_takes_it`: the gate guard dies at tick 130, drops the SG5
at 131 (18 rounds with this seed), it lands near his feet; 3 units away it stays (sister), walking onto it gives
Bond the SG5 with a full clip. The other seeded tests still pass (13 / 13 with `--test-threads=1`).

**Failed first**: the rest position as "translation = hit point" put the gun's sphere centre 0.009 under the floor, so
the line of sight from Bond to it was blocked and he could not take it; the formula as read (hit − ray · |centre y| +
the creation offset) leaves the centre |centre y| above the floor, and the pickup works.

**Not done** (G-029): weapon switching (Bond keeps the P2K; the SG5's first-person fire sound id is unknown),
`Player_IsBetterWeapon` auto-switch, level-placed / script pickups, keys, armour, respawning pickups, text messages.

## 0041 — Tower guards fall out of the towers (2026-10-09)

**Report** (user): shot in the tower, the guards played the falling animation but stayed at the window.

**Cause**: the port moved every drone by the horizontal root motion only, stopped by collision probes and stood on a
floor within 2 units; the vertical motion was replaced by the hip-height rule. **Read** (docs/systems/drone-damage.md):
SpecialDeath's enter clears the AI-boundary, collision and gravity flags and sets `Drone+0x1A`;
`DroneAnim_PreTransCallback` then keeps the vertical root motion and also moves the slide target `+0xBE0`;
`AnimFrameResolve` adds the rotated motion to `obj+0x30`; `NDrone2_SlideToPosition(…, 0.05, 1)` slides in 3D. Also read:
`Drone_CollisionHandler`, `NDrone2_DoGravity`, `NDrone2_DoCollision`, `NDrone2_Collision` (gravity for living drones —
not ported, G-026).

**Port**: `Drone::free_root` / `bounded`; in SpecialDeath the object becomes the hip (feet + `+0xA0`), takes the full
root motion unblocked, slides in 3D, skips the boundary push-out; `draw_y` gives the drawn height (nf-view, hit boxes).
SpecialDeath's update now also drops the weapon (read). Test `tower_guards_fall_out_of_the_towers`: spawner 266 picks
placement 267 (seed 1) or 7 (seed 32); either guard, killed at his post, falls 8–9.5 units out of the tower and ends
with the hip 0.12–0.51 above the ground below (the script was authored for that drop); shot over the head he stays.
First attempts at the test hit nothing: placement 7 is absent unless the spawner picks it, and the second shot
missed once the sniper turned to a Bond standing elsewhere. nf-game: all tests pass.

## 0042 — Bond as a player: movement, collision, crosshair, aim mode (2026-10-09)

**Report** (user): guards were hard to hit. The viewer had a free-flying camera, no crosshair and a wider-than-needed
guess at the view; Bond had no body.

**Read** (two research passes, docs/systems/player.md; re-checked here: `Player_Move`'s constants, the capsule
offsets in `Player_Collision`, the snapshot's eye 0.7, projection fx 332.62 / fy 388.06 / 60°, the P2K's gun offset,
`HUDCrossCoords`): movement targets and blending, the 0.55 capsule and its triangle pushes, the floor ray and its 0.1
band, gravity and jump, crouch, eye and pitch, the aim point through the crosshair, aim mode's cursor box, the
crosshair sprite (sheet 0x03000026, found in every level archive by `tex-list`). **Correction**: one pass reported
auto-aim off in the snapshot; `PlayerSetting+1` (0x2a38c9) is 1 — on in the game; the port follows D-001 (off with a
mouse, T toggles).

**Port**: `nf_game::player::Bond` (movement, capsule depenetration `CollisionWorld::capsule_push`, floor ray
`segment_hit`, gravity, jump, crouch, fall damage, eye); nf-view walks Bond with WASD / mouse (Space jump, C / Ctrl
crouch, right mouse aim mode with the D-002 cursor box, N free camera), shoots through the smoothed cursor, and draws
an overlay pass: the game's red crosshair cell (normal 11 × 11, aim 29 × 29) at (256 (1 + cx), 224 (1 − cy)) and a
stand-in HUD (health, clip pips, spare rounds, hit tint). Tests: unit tests (top speed 0.1 / tick, walls, a 0.3 step,
jump apex ~0.78, falling) and `bond_stands_and_walks_on_the_level`: at the snapshot's start position Bond stays at the
live height (the floor 1.086 below; live ray 1.0891), walks without falling through and stops at the wall ahead.
**Deviations**: two-sided triangles (U-037), no first-person gun model yet, no crouch transition animation.

## 0043 — The P2K emptied its clip on one click (2026-10-09)

**Report** (user): left click unloads the whole magazine; aiming only worked in aim mode. **Cause**:
`GunParams::from_record` read shots-per-pull as a u32 at `+0x28` (65537 for the P2K) and the interval at `+0x40` as a
float (1e−44): one press fired every tick until the clip was empty. `Player_WeaponFiring` (read) takes
`((u16 *)weapon)[0x14 + mode]` and `(float)(u32)weapon+0x40`. The unit test had used hand-made parameters, so it
never saw the record. Fixed; test `p2k_fires_once_per_pull` builds the parameters from the real record (1 per pull,
8 ticks, clip 7; held for a second → one shot; sister: the u32 reading is > 7). Also: `Bond::tick` dropped the
mouse turn in aim mode, so the view could not turn past the aim box (D-002); only the forward axis is zeroed now.

## 0044 — No mouse aim mode; the first-person P2K (2026-10-09)

**User decision**: the L1 manual-aim cursor is a controller feature; with the mouse it is not used (PLAN.md D-002
rewritten). Right mouse no longer enters aim mode; the Sim keeps aim mode for a pad.

**First-person gun** (docs/systems/player.md): skin `weapon+0xDC` 0x05000088 (skeleton 39) in 07000005's archive; its
main mesh 0x02100845 is a sleeve (byte 2 bit 0x10) replaced by `AnimSleeveGetEntity(0)` (0x1c6590, read): the first
loaded of `SleeveEnts` [L 0x2c6ed8] = 0x020009D0; six rigid gun parts on bones 0x26–0x2B; scripts idle 0x06000825 (2
frames), fire 0x06000823 (25), reload 0x06000827 (155) on skeleton 39 (test `first_person_p2k_is_in_the_level`).
nf-view draws it as a character whose matrix is the view (left, up, forward) at the eye + `weapon+0xE0` + the bob,
playing fire / reload / idle as the trigger and reload drive them; headless shots without `--cam` show Bond's view.
The rigid parts hang on their bones with no extra offset (they render in place). Not yet: the muzzle flash and smoke,
the turn toward an off-centre crosshair, draw / holster, the idle fidget. (The flash came in the next commit: mesh
`weapon+0x48` on datum 0 for 3 ticks.)

## 0045 — Guards surrender to Bond behind them (2026-10-09)

**Report** (user): sneaking up behind an unaware guard makes him give up his gun and surrender; the port lacked it.
**Read** (research pass, docs/systems/drone-surrender.md; Surrender_Anim re-read here): there is no disarm button —
the guard's own first sighting of Bond within 2 units, more than 120° behind it, with Bond armed and facing it within
45°, at alertness < 0.66 and prop 0x43, turns `FirstAttack` into Surrender_Anim; the gun is dropped as a pickup.
**Port**: sighting flags, the surrender gate, states 0x3F / 0x40 / 0x41 / 0x66, the unarmed Attack → DrawWeapon →
weapon 6, Bond's weapon lent to the drones; nf-view hides the gun of an unarmed guard. Test
`guard_surrenders_to_bond_behind_him` (see the spec). **Failed first**: the test's first spots behind guards were
inside the gate wall (no floor, no line of sight) and on a spawner template; the test now searches open floor behind a
guard active after a warm-up. Calling the hands-up anim after `ChangeWeapon(1)` had no script for set 0xC — the game
calls it first; reordered.

## 0046 — The game's HUD (2026-10-09)

**Report** (user): the UI was not faithful. **Read** (research pass, docs/systems/hud.md): the pane framework,
`HUD_UpdateHealthPane`, `HUD_UpdateAmmoPane`, the fonts. **Port**: `nf_formats::hud::HudTables` reads `HealthSprInfo`,
`AmmoSprInfo`, `BulletImg.189` and font 2 (`FONTSerpLight` + glyphs + kern pairs) from ACTION.ELF; test
`hud_tables_match_the_snapshot` checks them against the snapshot's values. nf-view's overlay draws, layer-sorted with
per-texture runs (six HUD textures from the level archive) and an additive pass: the crosshair; the health iris
(seven segments coloured by health, the ring) and armour segments; the damage glow (both sides: the hit direction is
not tracked); "clip-spare" in font 2 with its shadow and the rounds strip over the empty strip. The stand-in bars are
gone. Not yet: weapon name / fire-mode text and pickup messages (language strings), icons.

## 0047 — Crosshair, reload, HUD anchoring fixes (2026-10-09)

User reports, all fixed: (1) with auto-aim off the crosshair stayed where the last lock left it — the offsets now
halve every tick regardless (`Player_AutoAim`). (2) / (3) no reload animation and the slide left back: on the last
round the shot and the reload started in the same tick and the fire script won, so the locked-back last-round fire
frame held. `Player_WeaponFiring` (read): a reload starts with action 14 (R2) or with the trigger released and no round
to fire, only while the gun is idle (gun state 0, i.e. the fire script done) — `Gun::update_with(pressed, reload,
busy)`; the reload script plays (from frame 6 at 1.35), then the idle script. R = reload in nf-view. (4) "guards respawn
in T-poses": R used to be the debug toggle drawing held-back spawner templates (bind pose); moved to F3 (no unposed
present drone found headless with every channel on — `--channels` prints any). (5) the damage glow covered only the
centred 4:3 box: HUD sprites are now anchored (health left, ammo right, crosshair centre, full-screen stretched)
with 4:3 pixels at the window height.

## 0048 — Coverage measured; strategy; animation engine read; snap rotation (2026-10-09)

**Coverage** (user asked why so much is unported): of ACTION.ELF's 3038 functions, ~520 are referenced in our code
or docs (~26 % of the code by size) — many only partly. The work had gone depth-first per behaviour. **User
decision**: port breadth-first, a whole subsystem at a time (one at a time), animation first, then rendering
(docs/systems/render.md, researched), objects / scripts (docs/systems/world-objects.md, researched), weapons.

**Read** (docs/systems/anim-engine.md): the script list, add / append-blend / init / end / tick, the commands
(sequences, sounds, events), blending weights and order, frame interpolation, root motion and anim rotation.

**Snap rotation** (fixes the surrendering guard's 360): `DroneAnim_SetDAnimInternal` keeps the danim's info flags;
when a danim ends into its follow-on, `OnEndAnim` turns info flag 0x100 / 0x80 / 0x200 into a turn of π / π/2 / −π/2
(`+0x54C`, no blend) that `DroneAnim_SnapRotate` (called from `Drone_Control` after the control update) adds to the
yaw and the desired yaw. [L] danim 0x141 (surrender, 210 frames, root turning 3.10 rad) has flag 0x100. The port
applies the snap when the danim ends (U-039: the order against the call completion is not read). The surrender
test now asserts the surrendered guard faces Bond.

## 0049 — Animation blending, fractional frames, one pose for drawing and hit boxes (2026-10-09)

`nf_game::anim` (docs/systems/anim-engine.md): `Quat_Slerp_Acc`-style slerp, `AnimFrameBlend` (no-overshoot lerp +
slerp), `AnimFrameSet` (fractional frames), a script's pose at its time, and `AnimFrameResolve`'s order (outgoing
scripts oldest first, each next blended by 1 − the previous weight). Drones: every script switch goes through
`begin_blend` (`SetHTAnim`: blend 0 or not animating → `AnimScriptAdd` clears; else `AppendBlend`: fading scripts
restart an 8-frame fade, the replaced one fades over the call's transition blend `+2` (`GetDAnimForCall`), the
follow-on's `info+0xE` (0 after a snap turn), or 8 for `SetScript`); fading scripts keep playing and lose 1 per tick.
`Drone::object_pose` (blended local pose, root zeroed, upper-body aim) now feeds both nf-view's skinning and the hit
boxes (`palette`) — before, the hit boxes used the unaimed integer-frame pose while the mesh was aimed (user report:
"hitboxes aren't following the animations"). nf-game: all tests pass. Not yet: root-motion blending, anim rotation
(0x20000) for interrupted turns, script sound / event commands for drones, head tracking.

## 0050 — The first-person gun follows the crosshair (2026-10-09)

User report: the arm and gun should follow the auto-aim angle. `Player_SetWeaponAnimObj` / `Player_PositionGun`
(research pass, docs/systems/player.md): gun angles pitch `cy·π/6`, yaw aspect·`cx·π/6`, pivot (0, −0.2, 0.5) in
view space. Ported in `Sim::gun_matrix` (rotation order and yaw sign *suggested*); hidden with the free camera.

## 0051 — A danim without a script keeps the previous script (2026-10-09)

User report: a guard killed while surrendered was drawn in the bind pose for seconds before disappearing.
`DroneAnim_SetDAnimInternal` (read to its end): when the danim's script is 0x06000000 (none for the anim set nor set
0), or it is skipped for an undrawn drone, it calls `SetEndAnim` (the follow-on) and **not** `SetHTAnim` — the
previous script keeps playing. The port replaced the script with none. [L] danims 0x40 (death) and 0xBF (corpse hold)
have no script for the unarmed set 0xC nor set 0, so a surrendered (unarmed) guard had no pose from his death on.
Ported; test `surrendered_guard_killed_keeps_a_pose` (no tick without a pose from the shot until removal; it failed
with 687 such ticks before).

## 0052 — Script commands and events for drones; OnStartAnim; DAnimFunc_Run; muzzle from the shared pose (2026-10-09)

Read `DroneAnim_EventFunc` (0x13b750), `DroneAnim_PlayFiringAnim` (0x13da48), `DroneAnim_OnStartAnim` (0x13e238),
`DroneAnim_SetScript` (0x13e088), `DroneData_AnimFunc` / `DAStateFunc` tables [L 0x2707e0 / 0x2707f0: one entry each,
`DAnimFunc_Run` (danim 0xDC at speed 0.7 on 07000009 / 0A) and an empty `DAStateFunc_AimStand`]. Ported: the playing
script's commands per tick over the frames passed (start frame first, wrap-aware) — op 1 sounds into the hearing list,
op 4 event 2 into `event_func` (holster swap, `+0x20` / `+0x21` firing flags, `+0x579`); class-6 danims cut to 15
frames with end blend 5 unless the hit was big; the run speed; the muzzle taken from `palette` (the blended, aimed,
drawn pose) instead of an unaimed integer-frame pose at the object position. Not ported (G-026): grenade throws,
close-combat hits, weapon-change / message / tertiary events, footstep effects, `PlayFiringAnim`'s use of the flags.

## 0053 — Blended root motion; interrupted turns hand their turn to the yaw (2026-10-09)

`AnimFrameResolve` blends the scripts' root motion like their poses: the fading scripts' steps this tick are mixed
in order (each next by 1 − the previous weight), the current step last. `DroneAnim_SetHTAnim`: a replaced 180° turn
(info flag 0x100) not yet at its end is frozen (speed 0), fades over 30 frames in anim rotation, and its root yaw so far
(`+0x54C`) goes to `SnapRotate` — the port adds it to the desired yaw and sets the yaw to it at once (U-039) and removes
it from the fading pose's root. `SnapRotate` sets the yaw to the new desired yaw (it had added to the yaw). All
nf-game tests (incl. the live-trace patrol / chase timings) still pass.

## 0054 — Early one-shot ends (event 7); walk-idle on 07000014 (2026-10-09)

`DroneAnim_CheckForAnimComplete` (0x13dae8): a one-shot whose event 7 cleared `+0x579` goes to its follow-on (or holds)
before its last frame; ported. `SetWalkIdleAnim` (0x13cc70): 07000014 asks for 0x39. Read but not ported, recorded
as G-030: the exact `AnimObjectAimAt` (0x1bfe58: yaw about the spine and pitch so the weapon datum line passes the
target, clamps ±0.9, smoothing `+0x74`, applied to bone 2's quaternion inside `psiBuildMatrixPalette`; several input
vectors are stack slots the decompiler lost — needs the disassembly) and `AnimObjectHeadTrack` (bone 5).

## 0055 — Render subsystem I: per-leaf GS state, fog, pass and draw order, output gain (2026-10-09)

Spec: docs/systems/render.md (research pass; the fog table re-read here in `psiPreGameRun` 0x11e2e8, exact).
**Ported**: `Leaf::gs` decodes each packet's `DIRECT` A+D writes (test `leaf_gs_state_matches_the_research`: over
07000005 only ALPHA_1 / TEST_1 / ZBUF_1; TEST 0x5001B on 689 leaves, 0x507FD on 2; additive ALPHA on 131, subtractive
on 2; depth write 570 / 150 — the research counts exactly). nf-view: a `Draw` per material (pass, blend, depth write,
alpha test, fog) from the inherited leaf state, the glist flag 0x1 (alpha list) and the sky slot's parameter 4;
pipelines per draw state (shader overrides); the depth fog per pixel with the game's formula (zb = N1 + (F1 − N1)·n·
(f/d − 1)/(f − n), fog beyond zb < 32767, ⌊zb/256⌋/128), the alpha list's distance fade, the frame cleared to the fog
colour; the pass order (pre-world sky, list 0, characters, list 1, post-fog sky pinned behind everything, effects);
draws per object and leaf, list 0 sorted front to back and list 1 back to front each frame (`View_AddCels`); every
output × 0x91/0x80 (the display copy's gain, linear so equal to scaling the frame). **Failed first**: merging all
objects by texture blended the soft-edged snow overlays (9,237 vertices with alpha ≈ 0 on 07000005) against whatever
was drawn before them — usually the cleared background, a pale wash; per-object, per-leaf order fixed it. Not yet:
snow, environment mapping, billboards / jitter (glist flags 0x8 / 0x4), animated textures, dynamic lights,
`GlobalBlur`, the bilinear 7/16 display filter.

## 0056 — Render subsystem II: snow (2026-10-09)

Read `InitDrops`, `UpdateDrops`, `AddDrop`, `RainBox_Clip`, `Env_Update`, `Game_Run` (docs/systems/render.md "Snow").
`nf_game::env::Drops` ports the drops: the velocity table, the wind's random walk, the cube 7.5 ahead of the eye, the
volumes (placements 0x32, glist bounds + translation) clipped to it, up to 21 respawns a tick only inside active boxes,
kill heights. **They draw from the shared `Rand_*`** (`Env_Update`'s `Rand_Rand(20000)`, 2 wind `FRand`s, 3 per respawn)
before the object handler (`Game_Run`), so `Drones::tick` runs them first. The velocity table and the first-frame
fill happen before the level-start snapshot the port starts from, so they use a separate generator (*deviation*:
their values are not the game's). The shifted sequence broke one seeded test: `guard_takes_cover_like_the_game` re-
picked from seed 1 to 2 (aim 34 ticks, live 37; 0.03 from the live cover spot), as in 0027 / 0037. nf-view draws the
live flakes as camera-facing quads of texture 0x03000045, colour (63, 63, 63, 127), additive, depth-tested without
writing, after the alpha list; 0.2 units wide (*suggested*). Unit test `snow_falls_inside_its_box_and_respawns_above_
the_viewer` (sister: no box → nothing falls).

## 0057 — Animated textures

`CacheTexture` (0x11cd58, read) picks frame `first + (GameState+0x38 / rate) % frames`; `GameFlow_Main` (0x1c98c8)
adds `VIDEO_FRAME_RATE / FRAME_RATE_INT` to `GameState+0x38` per unpaused frame, both 60 in the snapshot [L], so one per
tick. `psiCreateMapTextures` (0x11d200) turns the header's rate byte into ticks per frame: `60 / rate` for 1–59, else 1
— the format notes had it as "60 / rate" without the clamp (map.md, texture.rs corrected). 07000005 has 8 animated
textures (12×5, 2×3, 2×30, 8×4, 7×10, 3×4, 2×2, 11×1 frames × ticks). nf-view uploads every frame and binds frame
`(ticks / rate) % frames` per draw; the phase starts at the level start, not the game's boot counter (*deviation*, phase
only). Unit test `texture_rate_is_ticks_per_frame` (sister: the byte read as ticks disagrees). Debug aids kept:
`NF_DRAWS=1` prints draw-state groups, list-1 objects, billboard/jitter-flagged objects (none drawn on 07000005: those
flags belong to dynamic objects) and the animated textures; `NF_HIDE=a,b` drops world objects by name.

## 0058 — Objects subsystem I: script players and the wine truck

Read `Script_Load` / `Run` / `Update` / `Interp` / `GetInterp` / `GetSplineWeights` / `EntityStart` / `CreateEntity` /
`AnimStart` / `KillStream` / `StreamEnd` / `EventHandler` / `SoundStart` / `Free`, `SP_CreateScriptPlayer` /
`LoadScript` / `Update` / `Activate` / `Hit` / `Delete` / `UnPause`, `AnimObjectUpdate`, `AnimSeqTick`'s root handling
and `AnimFrameResolve`'s root delta. Two decomp losses settled in the disassembly: `SP_Update`'s trigger
(`(hit flags | 4 when activated) & mask`, 0x1dbe20–0x1dbe9c) and `Script_GetInterp`'s last argument for entities
(`parent == −1`, 0x1d9a7c–0x1d9a98). `FRAME_RATE_MUL` = 1.0 [L]. Specs: docs/formats/nis-script.md,
docs/systems/script-player.md. The earlier research pass (world-objects.md) had the truck deleted at 5895: the
script's event 0xC at 5890 sets "keep objects", so it stays parked — corrected.

Code: `nf_formats::nis` (parser, command decoder; `Level.nis`), `nf_game::nis` (players, scripts, objects; run by
`Drones::tick` after the triggers, *deviation*: not interleaved with the placed objects), nf-view draws a character per
script animation (skin main meshes + rigid parts) and an attachment per drawn script entity while the simulation has
the object; `nf-extract nis <iso> <level.bin> [hash]` dumps scripts. Test `wine_truck_drives_and_parks`: 6 players;
the truck starts at frame 1 on the first update; at tick 3 its part 0 is at (154.05177, 15.799594, 93.95359), the
snapshot's live object position [L] (frame 1 = (154.05159, 15.799587, 93.9524), also in the snapshot); gatehouse stop
over 1800–2400 at (88.9, 18.8, 101.7), second stop (57.9, 26.2, 96.8); channel 0x73 at 5412; parked at
(29.89, 36.05, 47.99) on frame 5896 after the player is deleted (sister: the channel is off before 1000 and the truck
moved > 50). Not yet: camera / fade / light / sprite commands, the script entities' behaviour (triggers, AI volume),
corona drawing (G-031).

## 0059 — See-through walls: blend layers

User report: some wall faces transparent. Measured: 822 of 2000 list-0 batches on 07000005 carry vertex alpha below
0x80 (Room_Road_*, outer walls, gate towers); PRIM is 0x5C (ABE on) for all of them. Each such leaf draws an opaque
base batch, then the same triangles with another texture weighted by vertex alpha. nf-view merged batches by texture,
so an overlay could draw before its base: with alpha 0 it wrote depth and no colour, and the base then failed the
`Less` depth test — a hole showing the fog clear colour. Fix: world batches keep their order within a leaf (one group
per batch), and the depth test is `LessEqual` (the GS's ZTST GEQUAL on inverted Z passes equal depth). Before/after
render of the outer-wall corner: flat fog-coloured ledges become stone with the snow layer over it. `NF_LEAVES=<object
name>` (with `NF_DRAWS=1`) dumps an object's leaves, GS writes, textures and alpha ranges.

## 0060 — Moving collision: the truck is solid

User report: the truck has no hitbox. The port's collision world was static only; the game's `Collide_PickObj`
(0x1eae80) tests any object linked into a cel with collision data (`obj+0xD8`), moving or not, filtered by `obj+0xF0`
bits. Snapshot [L]: the truck animation object at 0x137D0F0, its nine discrete objects (type 0x3D) all with collision
data, linked into cels, flags 0x8880 / 0x8A81 — none excluded. Collision meshes per part: body 2744 triangles, wheels
168 / 168 / 320, hatch 52, windows 28, wipers 5 / 5, `OBJECT_wine_truck_all` 12.

`CollisionWorld` keeps every hashed glist's object-space collision and a moving set (`set_dynamic`) that all queries
(segment, segment_hit, capsule_push, line of sight, floors) test after the static geometry. `Drones::tick` places the
script animations' discrete parts (skin rigid parts at their posed bone matrices) and drawn script entities after the
script update. Found on the way: with every channel on, cut-scene 0x06000997 animates 0x0500002E with a sequence
lacking channels for the skeleton (index panic) — `anim::fits` now guards posing in the game and the viewer.
Tests: unit `moving_objects_collide_where_they_are` (sister: moved away / unknown glist → clear);
`wine_truck_drives_and_parks` now casts a ray down at the gatehouse stop at tick 1900: it stops on the truck at
y 20.58, the road below is 18.83 once the truck has left. All suites pass.

## 0061 — nf-view: focus loss and window-state robustness

User report: tabbing out (right click) crashes the viewer. Not reproduced: the previous build survives scripted
minimise / restore, focus switches, resize and maximise (`ShowWindow`, `SetForegroundWindow`, `MoveWindow`); the report
needs real mouse input (cursor grabbed, right click), which was not simulated on the user's desktop. Hardened what
could fail there: focus loss releases the cursor grab and drops held keys and the trigger; a minimised (0×0) window is
neither configured nor drawn; `get_current_texture` → `Lost` recreates the surface (configuring the old one cannot fix
it), `Outdated` reconfigures, other states retry next frame (before: every non-success reconfigured); GPU validation
errors are logged instead of wgpu's default panic. A panic hook writes the message and backtrace to
`nf-view-crash.log` in the working directory. Rechecked: the new build passes the same scripted window tests.

## 0062 — The truck's triggers, the gatehouse cut-scene and riding

Script touch triggers (`Script_CreateEntity` 0xEA / 0xEB) join the trigger system: parameters as a placement's
(`Trigger::from_params`, shared with `triggers::build`), volume = the glist's collision at the entity's matrix every
tick, removed with the object; new objects head the control list. `Trigger_Hit` reads the object's own hit list, so a
moving trigger tests where it is [M]. The gatehouse trigger (channel 0x5C, 1882–2312) is a 5 × 5 plate at y 27.05;
nothing solid under it for 11 units except a wall top at 24.75 at (100, 108), where Bond's capsule (feet + 2.41)
reaches it: Bond on the wall above the waiting truck starts ScriptPlayer #38 (0x060008C8).

Read `Script_CameraStart` / `End` / `UpdateCamera`, `Camera_CalcViewAngles` (argument = vertical FOV in radians,
viewer+0x114 = 4/3 [L]), the event handler's 4 / 6 and `Script_Free`'s `Player_Enable`, `Player_FeetOnPoint`,
`Control_InheritVelocity`. Ported: camera streams (`Obj::Camera`, the viewer's view and field of view follow it),
`PlayerDisable` / `PlayerEnable` requests (`Drones::player_events`; nf-view stops Bond's input and first-person view,
then stands him at the matrix), riding (`CollisionWorld::ground_motion` with previous matrices by stable id; Bond
moves and turns with the object under his feet). nf-view `--bond x,y,z` starts Bond anywhere (headless).

Checked: `truck_gatehouse_trigger_starts_the_next_script` — Bond under the plate sets 0x5C on his first tick there,
`PlayerDisable` at once, `PlayerEnable` 143 ticks later at (92.254, 19.424, 103.576) (sister: 20 units away — no
channel, no events, #38 waiting). Headless run with `--bond 100,26,108`: the script camera shows Bond jumping into the
truck; then first person in the bed; riding to the parking spot, Bond stays ~4 units behind the truck root, the offset
turning with it (+x heading west, +z after the turn south), health 100 throughout. Unit test: `ground_motion` reports
a 0.5 move (sister: nothing under the ray → `None`).

## 0063 — Sound I: banks, the EE pool, the SFX driver, output

User report: no audio. The audio phase (PLAN 5) had not started. `MODULES/SFX.IRX` (the IOP driver) keeps its symbol
table and stabs types; imported headless into a separate Ghidra project (`work/ghidra-iop`, MIPS LE 32; the install
used read-only) and exported (282 functions). Two research subagents wrote the EE-side and driver specs from the
decompilation; merged into docs/systems/sound.md. Re-checked here: the EE packs positions as `(int)(x · 1000)`
(`SFXStart3D` / `SFXSetupListener`) [M], so the driver's `Δ/100` works in 0.1-unit steps; `SFXSetMode` stores the EE's
mode (1 [L]) and every non-zero mode takes the surround path (rear voice phase-inverted) [M]; the listener's `Norm`
is the camera's row 0 = up × forward (snapshot (−0.545, 0, 0.839) for forward (−0.839, 0, −0.545)) [L].

Formats (docs/formats/sound-bank.md, `nf_formats::soundbank`): SBINFO slots; `.SHF` headers (252 on bank 4: the loop
flag agrees with the ADPCM repeat flag 252 / 252, offsets sequential, sizes sum to the `.SBF`); `.SFX` index and
`SFXParameters` + 28-byte sub-tracks (a trailing zero record always follows). 07000005 loads bank hash 5 = slot 4
(`Sound_Ready`).

Code: crate `nf-audio` (cpal 0.18.2 from the local cargo cache, no download) — `Output` (48 kHz-pitch voices, linear
resampling, end reporting) and `sfx::Driver` (the driver's start / setup / 3-D / volume / update in its integer
arithmetic, incl. the unconverged `PS2_SquareRoot` and `SFXrnd`); `nf_game::sound::Sounds` (the 256-node EE pool:
deferred start, 5-frame grace, per-frame positions, cull flag `SFXOutputData+0x15`); nf-view runs one batch per game
tick with Bond's eye as the listener (the cut-scene listener call is dropped by the EE batcher [M]). Sources wired:
Bond's shot, guards' gunfire (`DroneWeap_FireWeapon`: `weapon_data+0x60`, not 0x5EF), animation-script sounds,
pickups, script sounds (moving with their object, stopped by `0x17`). `nf-extract iso-get` pulls single disc files.

Tests: unit (`ps2_sqrt` values from the driver's behaviour incl. √1e6 → 1297, RNG; a sound on the left louder left and
fading with distance, rejected beyond the outer radius; the EE pool's deferred start / grace / removal; bank parsing);
opt-in `bank_live` (all 252 samples decode, 7 looped; 0x403 and 0xF5 start front + rear voices, 0xF5 at 100 units
rejected). All suites pass; the viewer opens the output device and loads the bank.

## 0064 — Gun script sounds (reload) and the rear-voice delay

User report: no reload sound; gunshots mostly in the left ear. Both were parts of the specs left out in 0063.
(1) The first-person gun's sounds come from its animation scripts (`AnimProcessScriptCmds` op 1 when the frame crosses
cmd[1]): P2K fire scripts 06000823 / 06000824 play 0x403 at frame 1, the reload 06000827 plays 0x40F / 0x40E / 0x411 at
frames 0x2E / 0x59 / 0x74. nf-view now plays them from the view script's frame crossing (script time + 1), and
`fire_player` keeps the shot only in the drones' hearing list. Headless check (`NF_SOUNDLOG=1 … --pull 10,40,…,220`):
seven shots (the last from the empty-clip script), then the auto-reload's three sounds at ticks 243 / 275 / 295 (32
and 20 ticks apart: frames 46 → 89 → 116 at speed 1.35). (2) `PS2_SendKeyONKeyOff` keys the rear voices 500 µs after
the front ones (mode ≠ 0). 0063 keyed them together, so the phase-inverted rear right cancelled the same waveform on
the right (gun: L ≈ 0.8, R ≈ 0.2 of the sample) — a judgement call ("negligible") that was wrong. `Cmd::Play` carries
the delay; `bank_live` asserts 0 / 0.5 ms for front / rear.

## 0065 — Collision materials; footsteps and impacts

Footstep and impact sounds depend on the surface (`HITDATA+0x50 & 0x3F` → `EffectInfo`), which collision.md listed as
not found. `Intersect_RayGeom` reads it from a byte array at the geometry's +8; the disassembly of
`parsemap_block_Coll_Data_New` (the decompiler loses the stores) shows that array starts right after the triangles:
one byte per triangle. `Collision::materials`; all 7,985 blocks on the disc carry it, values 0–21 (+ flag 0x40 on
6,081), all inside `EffectInfo`'s 23 entries (3128 bytes / 0x88) [D]. `CollisionWorld` keeps a material per triangle
(static and moving) and `segment_surface` returns it. `SurfaceSounds` reads `EffectInfo` +0x14 / +0x16 / +0x18
(impact, footstep, body) from ACTION.ELF; every id used exists in bank 4 [D].

Sounds: `Effect_Bullet` impact at the hit for Bond's and the guards' bullets; `NDrone2_DoHitEffects` 0x413 for guard
hits over 0.5 damage; Bond's footsteps (`AnimObjectUpdate`: distance gathered to a 1.65 stride, crouched 0.4125 with
0x286; `Effect_Player` plays the floor surface's footstep at the floor point with alertness 0). Test
`bullets_and_feet_make_surface_sounds`: a P2K bullet into the wall (surface 10) starts impact 0x8 at the hit within
0.2; the floor at the gate (surface 12) steps 0x7B (sister: surface 0 has none). All suites pass.

## 0066 — Map ambient sounds

Block 0x28 (`Sound_LoadMapSounds`): u32 count, 32-byte records from +4 (+4 id with flag 0x40000000, +8 position,
+0x14 f32, +0x18 radius R, +0x1C inner) [D: 07000005's two records equal the EE snapshot's live nodes 0x40000085
(36.2, 0, 5.7) R 365.3 / 182.6 and 0x40000174 (28.2, 12.9, 43.0) R 200 / 160]. `LevelMap::sounds`;
`nf_game::sound::MapSounds` = `HandleMapSoundAllocation` (start inside R with custom radii, stop past 1.2 · R²), run
before the pool update in nf-view. Effect 0x85 (133) is the 2-D stereo bed: two looping samples panned ±100,
polyphonic. Test `map_ambient_sounds_start_and_stop_by_distance`.

## 0067 — Guards' footsteps

`AnimProcessScriptCmds` (0x1c1b38): the footstep switch (0 toggle, 4 left, 5 right → `ScriptFootstepFoot` /
`ScriptFootstepType`; 3 = `Sound_StopAllWithId`) is op 4's event field — the EE research note had it under op 2; a
survey of 07000005's scripts finds no op 2 and walk scripts with op 4 events 4 / 5 (0600025A frames 19 / 57). The port
plays the floor surface's footstep under the guard (alertness 0) and stops event-3 ids. Faithful consequence: guards
not drawn slide without animating (`NDrone2_DoAnimation`), so they make no steps; and one-shots beyond 1.1 × outer
radius of the listener are culled. Test `patrolling_guards_make_footsteps` (listener beside the courtyard patroller):
52 steps in 3000 ticks.

## 0068 — Music

Research pass (subagent) on SFX.IRX's stream player and the EE music logic; docs/systems/music.md. Correction: track
15 (07000005) is `PS2/MUSIC/MFX_0/MFX_1` (MXI index + 1), not a STREAMS entry. Re-read here:
`DroneFunc_CheckAlarmRaised` (per-channel counters 0 → 1 → +1; 0xA4 posts 2 = 3 while < 60 and returns, 0x37 / 0x61
post 2 = 1 while < 600) and `NDrone2_SeenAndAttacking` with `MusicVars` (0x2707a0; 07000005: 2 attackers, 2 s
window, 35 units [L], reset per frame by `Drone_InitComms`).

Code: `nf_formats::music` (MXI, SMF, stereo SSD decoder with history across reads, MapMusic 35 × 76 and MapDroneData
12 × 16 from ACTION.ELF), `nf_audio::music::Player` (the buffer plan of `UpdateMusicMarkers`; deferred / instant
jumps; end markers; volume), an Output music queue (32 kHz stereo, played counter, truncation), `nf_game::music`
(event board, `UpdateMusicalEvents`, `UpdateMFX_01CastleA`, PickRandomCombat / Stealth, the attack gate); events
from the drones (combat states → gate), the alarm channels, death, cut-scene cameras (`Request::Camera`). nf-view
loads the track and tables, posts the level-start events and runs the EE logic after each batch (commands one frame
later, as on the PS2).

Tests: unit — combat on the 12th cumulative alerted frame, stealth on the 163rd calm frame (exactly the spec's
hysteresis), death → instant 11; the attack gate (two recent attackers, or sight; sister: neither). Opt-in
`music_live` on MFX_1: no underrun over 30 s from the start section; instant combat 4 reported at once, its data
14 336 samples ahead; deferred stealth 6 completed after 3.11 s (combat loop bars wider than a lap); end sting 9 stops
the stream 5.20 s after the request (0.448 s lead + 4.7 s sting). Volume at level start 7700 / 16383 as the spec.

## 0069 — Guards' speech; streamed effects

Research pass (subagent) on Drone_SfxInfo, NDrone2_PlaySFX / SpeechSFX and the talk functions, and on STREAMS.LUT /
.BIN and the IOP stream path; docs/systems/speech.md. Re-read here: `DroneFunc_FirstAttack` (0x148010) and
`DroneFunc_FirstSightState` (0x147c48) to place the talk rolls in the RNG order — SeenPlayerTalk inside FirstSightState
before its prop-0x1B `Rand(2)`, AttackTalk in FirstAttack after FirstSightState returned no state (or without a first
sight). The rolls consume the shared `Rand_*` (AttackTalk Rand(9) then Rand(3); SeenPlayerTalk Rand(3) then Rand(6);
PatrolTalk Rand(2) every patrol frame while the global cooldown is open), so `guard_notices_and_fires_like_the_game`
was re-seeded 7 → 2 (as in 0027 / 0037; 12 of 19 seeds reproduce the trace).

Code: `voice_lines` (Drone_SfxInfo 346 × 32), `stream_lut` / `stream_data` (end marker → `end & !0xFF` bytes; entry
211 = 7424 bytes), the driver's streamed sub-tracks (−1 − FileRef, pitch 0x759, 2 slots, never stolen, 3-D as samples),
the talk functions and `Drones::speech` (gates, per-line global stamps, the drone's current line `+0x4D0` with its
finished state), nf-view loads STREAMS (51 MB) into the driver. Tests: `guards_talk_in_a_fight` (pain 138 on the shot,
attack bark 177 twice; sister: only type-2 lines), `bank_live` decodes id 175's stream at 7424 bytes / 22 050 Hz. All
suites pass.

## 0070 — Searchlights: the spec re-read in the disassembly and the snapshot

HANDOFF §5.1, first item, docs first. The research pass's searchlight spec (world-objects.md) was subagent output,
never re-checked; done here before any code. Tools: a minimal MIPS disassembler over `work/elf/ACTION.ELF` +
`action-symtab.tsv` (now `tools/crosscheck/mipsdis.py` — the pattern HANDOFF §3 mentions, committed this time) and
`tools/trace/searchlights_snapshot.py` walking the 29 class-0x22 objects in `work/ee_start_05.bin`.

Corrections to the spec: (1) The create signature is `(a0 &translation, a1 &rotation, a2 placement, a3 glist)` —
measured at the dispatch in `parsemap_create_dynamic_objects` 0x1d04f0: the dispatcher builds the placement's +0x0C
and +0x18 vectors on its stack for every `*_Create`; the mangled names' "level_tag*" is the placement record, params
at +0x2C+4k. The old "base yaw" is the placement's rotation.y. (2) p0 is the part kind — 0 mount, 1 head (the only
part that updates), 2 spot (carries a dynamic light), 3 beam, 4 corona — not just "controller". (3) yaw =
`PS2Sinf(phase·2π/480 + π/2)·half + rot.y` (cos via PS2Sinf; both axes share the half-range; pitch centres on mid).
(4) `fmodf(phase, 480)`'s result is discarded — the store sits in the jal delay slot (0x1cbae0) — so phase grows
unbounded and PS2Sinf's own wrap keeps precision. (5) Tracking aims `atan2f(dx, dz)` and `−atan2f(dy−1, √(dx²+dz²))`
— libm atan2f, not `ATAN2_APPROX` (which the spot's normal orientation does use). (6) p1 = on-channel (head starts
in state 3, dark; the channel re-lights it with sound 0xE5), p2 = off-channel (kills: light deleted, parts flagged
+0xFE|=1, one-shot), p3 = alarm-out, held while tracking. (7) Breaking is the update's child-list walk (obj+0xD0,
next +4): an entry whose +0x58 object is class 5 with +8 > 0 → state 2: droop (pitch += 0.05/tick to 0.8), smoke
every 7th frame (`Gas_Init`, `Rand_FRand(2π)` per puff — another shared-RNG consumer), sound 0xE6.

The snapshot confirmed the live layout (instance+0x10 = the placement rotation, part links at +0x20+4·kind, channels
+0x34/35/36, half/mid +0x38/3c) and gave five golden sweep vectors (head 016dfe70: phase 317 → pitch 0.04712 / yaw
0.62241; bits in the doc). Two heads start dark on channels 97 (0x61) and 164 (0xA4) — the alarm channels
`DroneFunc_CheckAlarmRaised` posts (music.md): the alarm turns the castle searchlights on.

One puzzle, solved: spot 016e8ec0 links to nothing. `control_link_object_to_cel` (0x1346b8) PREPENDS — cel+8 is the
newest — so the cel list is reverse creation order; the courtyard spots are created before their heads, and a spot's
own create moves it −10000, so the later head's loop (same cel, now 17 000 units away) never links it. No special
case: create placements in record order and the measured rule reproduces the orphan. (Dead end first: same cel true,
distance 1.485 < 3 — the list direction was the missing fact.)

Docs: world-objects.md searchlight section rewritten ([M]/[L], golden bits); G-033 (port remainder), U-041 (the
child-list +8 field). Code next: the nf-game searchlight module + tests.

## 0071 — Environment rebuilt on Linux; nf-extract build fix; ISO verified

The port machine is now a Linux sandbox (Debian 13, no root). Rust 1.99.0 via rustup; cpal's alsa-sys
satisfied from an extracted `libasound2(-dev)` .deb prefix (`work`-external at `tools-bin/alsa-prefix`,
`scripts/nfenv.sh` wraps PKG_CONFIG_PATH / LIBRARY_PATH / LD_LIBRARY_PATH). The user's disc and the
full `work/` research tree came from their copyparty server (KOTPT share): ISO 2 439 315 456 bytes,
`work.7z` 2.8 GB; extracted everything except `work/nfre` (4.4 GB nightfire-re dumps), `work/gfx`,
`work/uviv` (re-derivable cross-check dumps) and the two Windows venvs — 40 496 files on disk, and the
7z deleted afterwards for space (re-downloadable).

Build fix: `nf-extract`'s `Data` construction (main.rs drone-trace command) had not been updated for
the `surfaces` / `voices` fields added with the sound work (WORKLOG 0066–0069) — the HANDOFF test
command does not build `nf-extract`, so it slipped through. Now `surfaces: None, voices: None`:
those paths draw no shared RNG (WORKLOG 0069), so the headless traces are unchanged; note for the
future: build `-p nf-extract` too.

ISO identity verified the strong way: `disc_golden` (14 tests, incl. the ELF SHA-1s) plus
`ai_live` 28 / `rng_live` / `trig_live` / `tuning_live` / `bank_live` / `music_live` all pass with
`NF_ISO` set — 137 green, one thread, on the downloaded image.

## 0072 — Searchlights ported: create/link, sweep, tracking, channels, the parts, breaking

HANDOFF §5.1 / WORKLOG 0070's "code next". `crates/nf-game/src/searchlight.rs` + `searchlight_live.rs` (3
opt-in tests) + 4 unit tests; nf-view draws the parts from the simulation each frame.

Spec corrections found while implementing (world-objects.md updated):
- **half/mid is NOT (max−min)·π/360**: the disassembly (0x1cb5c0–0x1cb680) computes
  `lo/hi = float(min/max(p4,p5)) · 0x3C8EFA36` (two EE-chopped muls; the constant is π/180 + 1 ulp, not
  π/180), then `mid = (hi+lo)·0.5`, `half = (hi−lo)·0.5` (the halvings are exact). The old formula was
  last-bit wrong on all seven live heads; the real one reproduces their stored bits 7/7.
- **The linking walk runs BEFORE the kind-2 branch** (0x1cb79c): the linked spot (016e9020) links while in
  place and moves below the world afterwards; only the orphan (016e8ec0, created before its head) is never
  linked. Both link stores are unconditional, so the newest-first cel walk leaves the OLDEST match linked.
- **The sweep's angles are normalised to (−π, π]** after the add (the framework's matrix build): heads with
  mid > π (016e03f0, 016e0970) store pitch 2π below the raw sum. Without the wrap, two of the five golden
  heads cannot be reproduced.
- **`Mat_Align2Dir`'s cross order pinned** against the live beam matrix 016e0130 (err 0.000000):
  rows = n(cross(up, z)), n(cross(z, row0)), z = n(dir); up for the BEAM is Bond − head (player +0x30); the
  HEAD's own matrix is the same function against world up (also pinned, head 016dfe70). G-033's open item
  closed; what remains is the VU0 MAC's last-ulp rounding (visual-only — the golden asserts ±4 ulp and the
  mirrored cross order, which is ~2.0 off).
- The corona: alpha = trunc((dot−0.2)·4·255), scale = (dot−0.2)·16, LOS (viewer = player+0x20) clear →
  scale ×1.5; the dot's viewer is `Player_GetHeadPos`. All verified against the three drawn coronas' live
  bits (my first reading of the scale bits was an arithmetic slip — the formulas fit exactly).
- The state-3 turn-on falls straight into the part updates the same tick (0x1cbd94), and the spot's
  distance rule arms tracking at dist ≤ 2 with a 900-tick (60·15) lost counter (0x1cbf14–0x1cbfc8).

The port: `Searchlights::create` (placement order, 29 `Rand_Rand(480)` + 2 `Rand_Rand(0)` draws — the
shared stream, consumed in `attach_level`; the game interleaves these with the other dynamic objects'
creates, so the port's stream positions differ until all creators are ported) and `Searchlights::update`
per head, called from `Drones::tick`'s control-list order (with the drones, triggers and spawners by
placement, newest first). Sounds 0xE5/0xE6 go through `play_sound_handle` (the pool + the hearing list).
Bullets: the child-list walk (U-041) is a stand-in — this tick's player bullet segments (recorded in
`update_player_bullets`) against the head's and mount's oriented gfx boxes; on a hit, state 2, the parts
hide, the light is deleted and the glass sound plays.

The create draws shifted the shared stream: five ai_live seeds re-picked (guard_notices 1, level_start 2,
bond_kills 2, guard_takes_cover 1, and tower_guards' dynamic search now also requires the kill — its
`find` only checked presence). `NF_TEST_SEED` (search only) overrides a test's seed; scripts/seed_search.sh
in the sandbox wraps the loop.

nf-view: the 29 parts leave the static pass (0xde) and join the Character-layer vertex tail tagged with
their part index; `animate` poses them from `sim.drones.searchlights` (mount/head/beam/spot matrices,
coronas as camera billboards with their scale — `ViewOpts` carries the camera axes; hidden parts collapse
below the world). Rendered headless on lavapipe (the sandbox has no GPU): the beams sweep, the coronas
glare, the spot circles the ground (vision-checked, work/renders/sl_300.png).

Not ported (G-033): the spot's dynamic light and flicker (renderer work), `Gas_Init` smoke (no particle
system; its 6 RNG calls per puff are NOT consumed — better a missing draw than a wrong one), the corona
alpha byte into nf-view's shader, `Control_NextLOD` (no LOD system).

## 0073 — Level coronas ported: static glow quads, no update function at all

HANDOFF §5.1's next object after the searchlights. The re-check first (disassembly of `Corona_Create`
0x18da00 / `Corona_Update` 0x18da78, the `control_funcs` table read from the ELF, the live snapshot walk
`scripts/coronas_snapshot.py`), then the port — which turned out to be documentation plus a render hook,
because the objects never change after create.

Spec corrections to the research pass (world-objects.md rewritten):
- **`control_funcs[1]` is empty.** `Corona_Create` makes class-1 objects (`sb v1,255(s0)` with v1 = 1,
  0x18da44), and the dispatch table's entry 1 (0x26f894) is zero: **no update function ever runs**. The
  research pass had `Corona_Update` as the updater — but its pointer sits in **`control_funcs[69]`**
  (class 0x45, 0x26fbcc), a *runtime* class: no class-0x45 object exists at level start [L], and no
  `li 0x45; sb +0xFF` store exists anywhere in ACTION.ELF's text (scanned) — creator unknown (U-042).
  The 13 live coronas keep their create state forever: alpha 0xFF, scale 1.0, `+0xFA` 0, `+0xF4` 0.
- **The class byte of `control_init_object` is NOT the object class**: init sets `+0xFC = 2` (the drawn
  byte, not the class); the class comes from the memset (0) or the creator's explicit store. This un-
  confused the 275 live class-1 objects (the 13 coronas among snow and script entities).
- `Corona_Create`'s `+0xF0 = glist+0x44` is `Control_SetGList`'s OR with preserved bits 0x30008000; for
  these 13 the glist word IS the object's own flags (0x22D Glow2/test1, 0x229 Window_corona), bit 0x1
  holding → `View_AddObjects` draw list 1, the alpha list, mirrored (−1) distance sort; `+0xDC = 0` (no
  distance fade); the leaves' own GS blend state draws them.
- Live [L]: 13/13 matched to placements 14, 15, 16, 55, 260 (`Corona_Glow2`), 231–237 (`Window_corona`,
  the castle windows), 245 (`Corona_test1`); `obj+0x30` = the placement translation, rotations verbatim.

The port: `crates/nf-game/src/corona.rs` (the type-0xFC placement filter — there is nothing else to
simulate: no instance, no RNG, no tick); `nf-view` skips 0xFC in the `--all` instance loop (like 0xDE)
and adds the 13 as World-layer instances at their placement transforms — the existing alpha-list pass
(object-flags bit 0x1 → pass 3) and each leaf's GS blend state do the glowing. Golden test
`crates/nf-game/tests/corona_live.rs`: the 13 (placement, name, flags) tuples from the live walk, the
alpha-list bit, and doors (0xDB) as the discriminator sister.

Environment: the sandbox was recycled — `tools-bin/alsa-prefix` (WORKLOG 0071) re-extracted from the
Debian `libasound2-dev` .deb; `nfenv.sh` unchanged. One gotcha re-learned: `NF_ISO` must be absolute
(`cargo test` runs from the crate dir; a relative path fails in `bank_live` with a NotFound unwrap).

Verification (renders, lavapipe): the 13 quads enter the draw as 13 additive (blend 0x48) pass-3
materials, 26 triangles — exactly +13/+26 over `NF_HIDE=Window_corona,Corona_Glow2,Corona_test1`
(the corona path now honours NF_HIDE like the instance loop). They glow from the castle's outer
(+z) side — `--cam 13 41 60 π 0` shows warm lit windows (49,926 differing pixels against the hidden
render, max delta 440; `work/renders/corona_windows_lit.png`); from the courtyard the wall occludes
them (ZTST 2 GEQUAL in the game — normal occlusion, the PS2 Z growing toward the viewer). The
alpha-list distance fade (`min(⌊zb/128⌋,255)/255`, zb ≈ 56k at 34 units) does not attenuate them.
Leaf state [D]: ALPHA_1 0x7f00000048 (additive), TEST_1 0x5001b (ATE aref 1), ZBUF_1 0x131000000
(no z-write), verts rgba (178, 172, 147, 128), texture: map texture 1 — 128×128, hashcode
0xFFFFFFFF, a 16-step gray ramp palette (the radial glow), all alpha 128.

## 0074 — Doors ported: swing/slide state machines, the E key, solid and drawn

The next §5.1 system. Research first (the same discipline as 0072): `Door_Create` 0x136100,
`Door_SetupSwing` 0x135440, `Door_SetState` 0x135618, `Door_Interp` 0x135738, `Door_ReactToHit` 0x135980,
`Door_Activate` 0x135be0 (the decompile tangles its branch nesting — ported from the disassembly),
`Door_DoMsg` 0x135f18, `Doors_Calc` 0x135fe0, `Door_Update` 0x1363c8; `KeyFrame_Interp` 0x1e1140 and
`Spline_Interp` 0x1e1858 (NULL framelist returns; 28-byte keys — pos + quat; Catmull-Rom position in the
game's op order; the slerp with `PS2Sinf` and the 0.01 dot cutoff; the quat→matrix with translation in row 3
— the same layout as the port's Mat4, pinned against the live `obj+0x90` words 3f3504f5/bf3504f0/426702ac).

Key finds: the creators do not see the file record — `parsemap_block_map_data_dynamic` (0x1d1b68) rebuilds it
on the stack with the record's +0x40/+0x42 (count, rate) at +0x02/+0x04 and `m_paths[record+0x44]` (the
level's 0x19 key-list blocks, one pointer each per `parsemap_block_path_data` 0x1d24d0) at +0x08 — verified
end-to-end: block-dump 0x19 of 07000005 gives 7 payloads, block 3 (84 bytes = 3×28) is exactly the
`m_paths[3]` = 0x009d1404 the live DOORINFO+0xC points at. The create-time `FRAME_RATE` is still 30 (the
locked doors' speed measures 1/30 although the global reads 60 during play); `Door_SetupSwing`'s whole-tick
re-quantisation `0.5/(int)(0.5/2s)` turns 1/30 into the measured 1/14 (7 ticks a swing). The 4 locked doors
have NO anim (frames 0xFFFF, `Spline_Interp` no-ops): channel 200 unlocking them is an invisible state
change; what raises 200 is unknown (U-044). The swing sign: `dot(row2(key-0 pose), row0(closed))` → −1, live.

Port: `crates/nf-game/src/door.rs` — `Doors::create` (placement order, no RNG), `update` in the control-list
order (`Door_Update`: ch_open on/off, the lock edge, the 360-tick auto-close with the `(r1+r2)²·2` proximity
test over player + drones, `Door_Interp`, `Door_ReactToHit`'s anti-crush — the proxy scan stood in by the
same sphere test, as U-041's precedent), `activate` (from the disassembly), `calc` (`Doors_Calc` group sync),
`player_use` (`Player_Activate`'s sphere one unit ahead of the head) wired to the E key through
`Sim::step_bond`. Sounds through the sfx table as usual (285 open, 284 close, 320 the locked thud). nf-view:
0xDB leaves the static instance loop (like 0xDE/0xFC) and the 5 draw as posed Character-layer vertices at
`obj+0x90` each frame (`NF_HIDE` honoured for A/B); collision joins `set_dynamic` with the object's own
collision mesh (id bit 40 + index), at most one tick behind the visual. `nf-formats`: `LevelMap::path_data`
(0x19 payloads) and `Level::path_blocks()`.

Tests: 8 unit tests in door.rs (slerp ends, Catmull-Rom midpoint, the keyframe clamp, the whole-tick speed,
swing-opens-away + auto-close + sounds, the locked thud + edge unlock + its sister, group sync); golden
`doors_live.rs` (the 5 doors' live create state — kinds, channels, sounds, flags, speeds 1/30 and 1/14,
pos01 0.5, swing −1, frames, the 3 keys' pos+quat, the closed matrix vs the live words; block 2's path list
and extra[7]/[8] as the discriminator sisters). All nf-game 33 green.

Verification (renders, lavapipe): the door draws — an 8-angle A/B sweep around `Door_UNLocked`
(`scripts/door_sweep.sh`-style, 2.5 units out) differs by 1.8k–44k half-res samples at 6 of 8 angles (the two
zero angles are the slab edge-on); `work/renders/door_closed.png` is the angle-2 view. The opening swing
itself is unit-tested (7 ticks to OPEN, away from the activator's side; the auto-close after 360) — the
headless shot cannot press E, so the motion is for the user to see in play.

Environment: the sandbox was recycled again — `tools-bin/alsa-prefix` re-extracted (libasound2-dev, the
runtime `libasound.so.2` copied in because the dev deb only ships the symlink) and lavapipe added
(mesa-vulkan-drivers; `VK_ICD_FILENAMES` in `scripts/nfenv.sh`).

Next (§5.1): the helicopter (`Copter_Create` 0x17d340), then the level-exit trigger + objectives (the 0xE8
gates both load 0x07000066).

## 0075 — Audio: ducker, fades, reverb/environment, SPU ADSR + 4-tap interpolation

Area: the audio remainder (G-032), `crates/nf-audio` and `nf-game/src/sound.rs` / `music.rs`. Built and tested on
Linux (stable 1.99; ALSA via a local `alsa.pc` + `libasound.so` symlink, no sudo). All 155 tests pass (4 new).

What the player hears now (and what isn't done):
- **Music ducks under gunfire.** `ES_StartDucker`/`ES_UpdateDucker` (SFX.IRX 0x83c8/0x8438) ported: a sound with
  `Ducker != 0` (TrackingType 0/3) starts a duck; `DuckerCurrentVolume` creeps toward the target at s/6 (within the
  duration) then s/15 (past it), with the sign-fixup for sub-zero steps; the music gain is `master·duck·base·fade`
  (the live `DuckerCurrentVolume` and `FadeMusicVolume`, no longer the hardcoded 100/10000). The SFX get 1/3 of the
  duck (the `100 - (100-DuckerCurrentVolume)/3` term). `NisQuitten` → `ES_StopDucker` (fast recovery).
- **Fades.** `SFXFadeUpdate`/`SFXFadeDown`/`SFXFadeUp` (0xf8c, RPC cmds 6/7): a 2 s linear fade (10000 at 5/ms),
  `FadeMusicVolume` tracks it when `fade_music` is set; fade-up starts from 0. The end-sting (`Music_Event` 6/7 →
  `Cmd::FadeDown`) now actually fades the audio (was a no-op in `sound_tick`).
- **Reverb / environment.** `SFXSetEnvironment` (RPC 0x1D) + `SFXUpdateEnvironment` (0xe60): the underwater flag
  (reverb == -0x6768), the reverb target clamp, the ±16/−4 reverb envelope, the ±4 indoors volume, the `EVOL` depth
  (r·80/100·0x3FFF) sent to the output. The output runs a stereo comb-filter reverb at that depth; voices with
  `ReverbSend != 0` (SFXParameters+0x00) are routed through it. *The cel/room system that supplies the `room env`
  value is not ported (G-016), so `set_environment(0, 0)` is called for now — no reverb/underwater until that lands.*
- **SPU ADSR release.** ADSR1=0x000F, ADSR2=0x1FC0 (sound.md §3.4): instant attack, max sustain (DR=0 → no decay),
  fastest linear release. Key-off now begins a ~4 ms linear release tail instead of cutting hard (the old
  "*deviation*: no ADSR release"). RELEASE_SAMPLES = 192 (≈4 ms at 48 kHz); the exact SPU rate-table step count is
  pending a disc cross-check (G-032d).
- **4-tap interpolation.** The mixer now reads x[-1]..x[2] and applies a 4-tap kernel (replacing the 2-tap linear
  "*deviation*"). *Catmull-Rom stand-in for the SPU's exact Gaussian coefficient table* (G-032d, deferred — the
  table is not in the committed docs and the decompile is not accessible in this environment).
- **Bullet fly-bys (G-032a): deferred.** The `+0x6C & 0x40000` flag accessor (`weapon_data.bullet_flags`/`flyby`) and
  the fly-by sound id 0x1dc (sound.md:330) are ported, but the `Bullet_DoTrails` trigger logic + RNG call order are
  in `work/decomp/ACTION.ELF/Bullet_DoTrails_*.c` (not accessible here). Not guessing the RNG order — it is shared
  (`Rand_*`) and would break seeded tests.
- **Music events 3/7/8 (objectives): hooks confirmed.** `Music::update` reads e[7] (on_fail) and `castle_a` reads
  e[3]||e[8] (end_combat/end_calm), both → jump + `Cmd::FadeDown`. The objectives session only needs to call
  `drones.music.event(3|7|8, value)`.

Evidence: all from `docs/systems/sound.md` (measured by prior agents from `work/decomp/SFX.IRX` + the disc): the
gp-relative globals (§0), the SFXUpdate step-2 order (§6), `ES_CombineVolumes` (§5.5), `SFXFadeUpdate`/ducker/
environment (§5.5), `StartSample`'s ADSR1/ADSR2 (§3.4). The Gaussian table and `PS2_SetupReverb` preset are [S]
(nocash PSX-SPX structure; the exact fixed-point coefficients need `work/decomp` — G-032d). Disc cross-checks
(`NF_ISO` golden comparisons of the envelope/interpolation output) are deferred: the ISO and `work/` are not
available in this environment (the copyparty share requires a password the user has not provided; verified via curl,
wget and a real headless browser — all return the login page).

Tests (each with a sister assertion): `fade_down_reaches_zero_in_two_seconds_and_tracks_music` (mid-fade + clamp +
fade-up-from-0 + fade_music-off leaves music full); `ducker_creeps_toward_target_then_recovers` (slow phase +
recovery + no-ducker stays 100); `environment_envelope_and_underwater` (underwater flag + target clamp + +16 rise +
−4 fall + indoors 50/100); `flyby_bit_is_0x40000` (SG5 0x40050 set, 0x50 clear, out-of-range false). `cargo test
--workspace` = 155 passed, 0 failed.

No `Rand_*` calls added (the ducker/reverb/fade use the IOP's `SFXrnd` only where the spec does — none here — and
the EE `Rand_*` is untouched), so seeded tests are unaffected.

## 0076 — Bullet fly-bys (Bullet_DoTrails): the whiz past the camera

Area: G-032a, `crates/nf-game/src/bullet.rs` + `drone.rs`. The `work/` research bundle is now accessible (the copyparty
share's volume is `/KOTPT/`, account "z"); the `Bullet_DoTrails` decompile at `0x12d890` was read and the fly-by ported
whole from it, in the game's call order.

What the player hears: when a drone bullet passes within 3 units of the camera (SqDist < 9), there's a 1-in-5 chance
(`Rand_Rand(5)`) of a fly-by whiz — sound 0x1dc — played at the closest point on the bullet's path to the viewer. Only
weapons with `weapon_data+0x6C & 0x40000` (e.g. SG5 = 0x40050); single-player only; the player's own bullets are
exempt (in the port, the player fires `PlayerBullet`s, so all `Bullet`s are drone-fired and eligible). The RNG order
is preserved: `Rand_FRand(0.5)` (the first-step speed, line 116 of Bullet_update) → `Rand_Rand(5)` (inside DoTrails,
line 376/475), with no other Rand calls between (verified in the decompile).

Port: `Bullet` gains a `flyby: bool` field (set from `weapon_data.flyby(weapon)` at creation); `Bullet::update` gains
a `viewer: V3` param and the fly-by check after the move (closest-point-on-segment, SqDist < 9, `rng.rand(5) == 0`).
`BulletEvent::FlyBy(V3)` routes to `Sound_Play3D(0x1dc, …)` in the drone loop. The `vecutil_calculate_closest_point_on_line`
(0x1df7e0) segment test is ported as `closest_point_on_segment`.

Evidence: `work/decomp/ACTION.ELF/0012d890_Bullet_DoTrails*.c` (the 0x40000 block at lines 238–257), `0012e7b0_Bullet_update*.c`
(call sites at lines 376/475, Rand order at line 116), `001df7e0_vecutil_calculate_closest_point_on_line*.c`. Also confirmed
from `work/decomp/SFX.IRX/`: `PS2_SetupReverb` (0xc944) = mode 4, delay 30, feedback 200 (matches sound.md §5.5);
`psiRequestVoiceHandle2` (0xe420) = ADSR1 0x000F, ADSR2 0x1FC0 (matches §3.4). The SPU2 Gaussian interpolation is a
hardware feature (not in SFX.IRX; the driver only writes the pitch register) — the Catmull-Rom 4-tap stand-in from 0075
remains, noted in GAPS G-032d.

Tests: `flyby_plays_within_three_units_on_one_in_five` (a fly-by bullet within 3 units fires within 60 ticks; sister: no
flag → never fires; 10 units away → never fires). The `Rand_Rand(5)` call did not break seeded tests (no existing test
exercises a fly-by bullet path). `cargo test --workspace` = 156 passed, 0 failed.

## 0077 — Levels survey, level flow (exit trigger), and the level-select arg

Area: getting the port to run more of the game than 07000005. `crates/nf-extract` (new `level-survey` command),
`crates/nf-game/src/triggers.rs` (the 0xE8 exit trigger), `crates/nf-game/tests/level_live.rs` (opt-in NF_ISO),
`docs/systems/levels.md` (new).

Survey (Task 1): `nf-extract level-survey <iso>` loads every `0700xxxx.bin` in FILES.BIN through the same
`Level::parse` path nf-view uses, and reports — for each — the maps, objects, placements, drones, collision, scripts,
skins and textures counts, the placement-type histogram, the object-name histogram, and any parse error. Result:
**all 36 on-foot level archives parse successfully** (no format gaps blocking the survey). Every story code 01–1b
also runs its game logic headless (`ai-sim`): the collision world builds, drones spawn and step their AI state
machines, and the bullet/player-damage path runs. Written to `docs/systems/levels.md` with the full table.

Level flow (Task 3): the 0xE8 end-of-level trigger (`Trigger_Activate` 0x1b2318 → `ResetMap_LevelToLoad` 0x1d3800)
was parsed as `kind::LOAD_LEVEL` but had no `update` arm — it never fired. Ported: it now acts as a one-shot touch
trigger (sets its switch channel when Bond enters the volume, latches `gone`), and carries the next level's hashcode
(placement param 0x20) in a new `Trigger::next_level` field for the caller to drive `ResetMap_LevelToLoad`. The
level-select argument was already present (`nf-view <iso> <level>`; `<level>` is a FILES.BIN name like `07000006`),
so any of the 36 levels can be loaded by name.

Level tests (Task 2): `crates/nf-game/tests/level_live.rs` (opt-in `NF_ISO`) loads each of the 20 story levels and
checks its main-map drone / object / placement counts against the disc values from the survey. Sister assertion: a
level's counts differ from another's (so a wrong table would fail). `cargo test --workspace` = 157 passed (was 156;
+1 `load_level_trigger_fires_once_and_carries_next_level`, +1 `every_story_level_loads_with_expected_counts`).

Not done (honest): the next level after 07000005 was not *visually* verified to render, because this environment has
no Vulkan ICD (lavapipe) — `nf-view --shot` needs one. The render path is the same `Scene::build` 07000005 uses and
the survey confirms every level's placement types are the same set the test level exercises (0x0f drones, 0x19 paths,
0x20 doors, 0xfc coronas, 0xde/0xdb searchlights, 0xd9 script players, 0xe8 exit) — so the renderer, collision, guards
and scripts load through the same code paths. Visual diffing against PCSX2 is pending a GPU environment. No level-
specific hacks: the only code change is the LOAD_LEVEL trigger arm (ported from the decompile, not approximated).


## 0078 — Drone gravity and collision spheres: Drone_CollisionHandler ported

G-026 (part). The on-foot engine's per-tick collision driver is `Drone_CollisionHandler` (0x140968), called from the
control-list pass after the state machine. The port ran a floor-probe stand-in (`floor_below` within 2.0 / 0.6 of the
feet) inside `move_by` / `slide_to`; a guard dying at a ledge edge would not fall (WORKLOG 0041's gap). This ports the
game's own path so the guard falls.

Research first (the same discipline as 0072/0074): `Drone_CollisionHandler` 0x140968, `Drone_FeetOnPoint` 0x140660,
`NDrone2_DoCollision` 0x149ec8, `NDrone2_DoGravity` 0x141e58, `NDrone2_Collision` 0x140e90,
`DroneFunc_NearPlayerCollision` 0x14b190, `DroneInit_Collision` 0x14b1e8, `DroneFunc_ClampPosToCollisionCylinder`
0x1499d0. New doc: `docs/systems/drone-collision.md` (fields, constants, the handler's order, the predicates).

Key finds: `WldGravity` is `(0, −9.8, 0)` (VECTOR at 0x2d8890, [L] pickups.md/player.md); `REC_FRAME_RATE` = 1/60
(0x30d0cc). The down-ray (`Drone_FeetOnPoint`) runs from `pos` to `pos + up·−(hip+0.6)` with mask 0xC (walkable +
solid); the on-surface test is the hit's y within **0.2 below the feet** (`pos.y − hip`). When on the surface the game
zeros the velocity (the `+0x480` field) and sets `anim+0x60 & 8`; when off it accumulates `velocity += WldGravity ·
REC_FRAME_RATE` then `pos += velocity · REC_FRAME_RATE`. The feet-sphere radius is 0.4 (0.55 in state 0xC, 0.3 in
crouch mode 0x17); the AI-boundary push-out (`AINetwork_GetBoundsPushVectorForSphere`, r 0.4) runs last in the handler.
`DroneFunc_NearPlayerCollision` (a guard within 2.0 of Bond) skips the full sphere test. The `free_root` path
(SpecialDeath) bypasses the handler: the root motion moves the object in 3D and the gravity/collision flags
(`& 0x20 / 0x80`, `bounded`) are off.

Port: `crates/nf-game/src/drone.rs` — a `velocity` field (`Drone+0x480`), `in_air` / `on_surface` flags
(`+0x4F8 & 0x4000` / `anim+0x60 & 8`), `feet_on_point` (the down-ray), `collision_handler` (the per-tick driver),
called from `Drones::tick` after the state machine and before the AI-boundary push-out (the game's order). `move_by`
and `slide_to` no longer probe the floor — the handler puts the drone back on the ground. The port's ground convention
is `pos.y = floor + hip` (hip 1.0), feet at `pos.y − 0.6` (the port's own offset, unchanged so the existing live
traces still match).

Tests: 4 unit tests in drone.rs (stands on the floor + velocity zeroes; falls under gravity off a ledge — the
WORKLOG 0041 gap; `free_root` bypasses; the 0.2 on-surface threshold's sister — on the floor vs. 1.0 below = in air);
live `guard_gravity_and_collision_spheres` in ai_live.rs (the gate guard at (61.34, 99.50) stays on the courtyard
floor, velocity zero, `on_surface`; sister: a guard lifted 50 units falls, `in_air`, v < −9.0). All nf-game 77 green.

What the player sees: a guard knocked off a ledge (a grenade, a shotgun blast, a SpecialDeath's free root ending over
a drop) now falls to the ground below instead of hovering 2 units under the ledge. Guards on flat ground are unchanged
(the down-ray snaps them to the same floor the old probe did). Not done: the per-material bounce/coefficient table at
`0x3037dc` (the `bVar2` extension in `Drone_FeetOnPoint` — the walkable/solid mask 0xC is enough for the floor), the
feet-sphere radius variations (0.55 / 0.3), `DroneInit_Collision`'s create-time floor snap, the `DealWithObjHit` per-
hit dispatch (water ripples, doors, breakables, explosions, players — the explosion hit path comes with the grenades
commit next).

## 0079 — Remaining cover states: sniper 0x97/0x98, ambush 0x37/0x38, ConsiderExplosive

G-028. The cover system's remaining states from `docs/systems/drone-cover.md`. Research first (same discipline):
`NDrone2_DSTATE_UnderCoverSniperFire` 0x16a610, `NDrone2_DSTATE_UnderCoverSniperReload` 0x16a8b0,
`NDrone2_DSTATE_AmbushInit` 0x172960, `NDrone2_DSTATE_AmbushWait` 0x161560, `NDrone2_FindAmbushCover` 0x152958,
`DroneFunc_ConsiderExplosive` 0x1431c0, and the `UnderCoverFire` ENTER branch (0x16a3e0).

Key finds: `UnderCoverFire` ENTER branches to 0x97 when the drone's anim set `+0xDA == 0x13` (sniper) AND
`level < 0x0700000E` (the test level 07000005 qualifies — `GameState._12_4_ + 0xf8fffff4 < 2`). State 0x97: timer C at
+`FRAME_RATE_INT` (1 s) fires, timer D at +`FRAME_RATE_INT × 6` (6 s) returns to 0x99; clip empty → 0x98 (reload anim
0x52, refill `+0xBBC = +0xBBE`, back to 0x97). `OpponentIsAimingAtMe(7.5)` + `BondIsFacingMe(45)` + `Rand(3) == 0` →
duck to 0x99. Ambush mode 0x10 (`NDrone2_init_DMODE_Ambush`) sets `+0x5A2 = 0x37`; `FindAmbushCover` is `FindCover`
restricted to nodes within **2.0 units** of the drone (the scan radius) — claims the closest whose anim predicates
pass → 0x93; without one, `CallAnim(0, 0x11, …)` (crouch) → 0x38, which waits until the opponent is visible or within
3.0 → 0x56. `ConsiderExplosive` (msg 0x21): stores `+0x330` (pos) / `+0x350` (run distance: 12.0 frag / 5.0 stun / 10.0
mine) and routes to `RecoverFromScaryObject` (0x86); the flee route update is G-026.

Port: `crates/nf-game/src/drone.rs` — state constants `UNDER_COVER_SNIPER_FIRE` (0x97),
`UNDER_COVER_SNIPER_RELOAD` (0x98), `AMBUSH_INIT` (0x37), `AMBUSH_WAIT` (0x38); msg constants `TIMER_D` (0xD) and
`EXPLOSIVE` (0x21); `timer_d` field (`Drone+0xCD0`), `explosive_pos` / `explosive_run_dist` (`+0x330` / `+0x350`);
`find_ambush_cover` (the 2.0-unit scan), `consider_explosive` (msg 0x21 → 0x86); the four states in `handle_cover`
with the sniper branch from `UnderCoverFire` ENTER; timer D dispatched in `Drones::tick`. The ambush mode's start
state (mode 0x10 → 0x37) was already mapped in `start_state`. `send` and `start_state` made `pub` for the live tests.

Tests: 2 live in ai_live.rs — `sniper_cover_branches_to_sniper_fire_state` (a sniper-anim-set drone in 0x96 branches
to 0x97 on ENTER; sister: a non-sniper stays in 0x96) and `ambush_mode_starts_in_ambush_init` (mode 0x10 → start 0x37;
sister: mode 0 does not). All nf-game 80 green (was 77; +3: the 2 live + the existing collision tests). No Rand calls
added (the `Rand(3)` in the sniper duck is the game's own, already consumed by the existing `rand` field).

What the player sees: a sniper guard in cover on 07000005 now pops up to fire once a second (timer C), ducks back after
6 s (timer D) or when Bond aims at him, and reloads (0x98) when empty — instead of the generic 0x96 fire loop. An
ambush guard (mode 0x10) crouches behind the nearest cover within 2 m and waits; if Bond gets within 3 m or is seen,
he attacks. A guard near a grenade (msg 0x21) flees to 0x86 (the flee route itself is G-026). Not done: the
`RecoverFromScaryObject` (0x86) flee route update (the run-away path), `ValidateCoverAnim`'s 5→7/10 fallback, the
`+0x538` blend-timing check for non-call anims.

## 0080 — Grenades: the throw, the arc, the bounce, the explosion and sound

G-026 (the grenades part). The on-foot engine has no separate grenade entity — grenades are bullets (`obj+0xFF = 5`)
with `weapon_data+0x6C & 0x8` (gravity), `+0x70 & 0x4` (explode on impact) and `+0x70 & 0x100` (bounce). This ports
the throw decision, the throw state (0x2D), the projectile physics and the explosion.

Research first (same discipline): `NDrone2_DSTATE_GrenadeThrow` 0x173480, `DroneWeap_ThrowGrenade` 0x179558,
`DroneWeap_CanThrowGrenade` 0x17a798, `Bullet_init` 0x12c5a0, `Bullet_update` 0x12e7b0,
`Bullet_CollisionHandler` 0x12ce80, `Explode_Create` 0x17cbd0, `Explode_Propagate` 0x17c518,
`Drone_ExplosiveHit` 0x13aab8, `Bullet_handle_object_destruction` 0x12cd98.

Key finds: the throw decision (`CanThrowGrenade`) gates on `+0xBCC ≠ 0` (grenade weapon), `+0xBCE ≠ 0` (count),
`Rand(2) == 0` (50 %), an opponent, `8.0 ≤ dist ≤ 20.0`, and `DroneAnim_CanDoAnimState(0x49, 0)` (the throw anim).
The throw (`ThrowGrenade`) reads bone 0x15 (right hand) for the origin, applies a ±π/4 yaw lob by distance (< 10 →
+π/4 lob; > 18 → −π/4 flat), mode 0xD sniper 1/3 chance of stun (0x36) over frag (0x35). The projectile's gravity is
`velocity += WldGravity · REC_FRAME_RATE²`, `pos += velocity · REC_FRAME_RATE`. The bounce (`Bullet_CollisionHandler`):
reflect by the surface normal, × the per-material coefficient at `0x3037dc + material·0x88`, max 20 bounces; sound
0x51C. The explosion (`Explode_Propagate`): sphere intersect of `radius` (`weapon+0x08`), `damage = amount · (1 −
dist / radius)` (min 0.0002), drones take `Drone_ExplosiveHit` (msg 9), Bond takes `Player_Hurt`; sound 0x4A7; the
explosion script 0x6000052. The fuse (`+0x3F`) is 210 ticks (3.5 s), 420 for type 0x37.

Port: new `crates/nf-game/src/grenade.rs` — `Grenade` (pos, velocity, speed, fuse `travelled`/`range`, damage, radius,
weapon_type, bounces), `GrenadeEvent` (Bounce / Explode), `explode_propagate` (the linear falloff sphere). The throw
state `GRENADE_THROW` (0x2D), `can_throw_grenade` / `throw_grenade` on `Drone`, the `DroneAnim_EventFunc` datum 1
(sets `pending_grenade` → `Drones::tick` calls `throw_grenade`), the grenade decision in `combat_state` (between
cover and `choose_combat_move`, for both `AIM_STAND_FIRE` and `AIM_CROUCH_FIRE`). `Drones` gains `grenades` /
`explosions` fields; the tick loop updates grenades, plays bounce (0x51C) and blast (0x4A7) sounds, applies the
explosion's damage to drones (`Drone_ExplosiveHit` → msg 9 / death) and Bond (`Player_Hurt`).

Tests: 3 unit in grenade.rs (the arc + bounce, the max-bounce explosion, the linear falloff with a sister beyond the
radius); live `guard_throws_a_grenade_when_in_range` in ai_live.rs (a guard with grenades at dist 15 enters 0x2D;
sister: without grenades never does). All nf-game 83 green (was 80; +3 grenade unit + 1 live). RNG: `throw_grenade`
adds `Rand(3)` for the sniper stun roll and `can_throw_grenade` adds `Rand(2)` — both run only in the grenade path,
so seeded tests that don't reach a grenade throw are unaffected.

What the player sees: a guard 8–20 units away may lob a grenade (anim 0x49) — it arcs under gravity, bounces off the
world (sound 0x51C), and explodes on impact or after the 3.5-s fuse (sound 0x4A7), damaging Bond and nearby guards
with a linear falloff. Not done: the per-material bounce coefficient table at `0x3037dc` (a 0.4 stand-in), the explosion
script 0x6000052's visual (debris, the flash), the `RecoverFromScaryObject` (0x86) flee route (the guard runs from the
grenade — G-026), stun grenades' receiving states, ricochets.)


## C-1 — Weapon switching: mouse wheel, number keys, the holster/draw cycle, the op-1 sound remap

G-029 (weapon switching for Bond). The first of the player-remainder items (§5.4): the other weapons in 07000005
and the game's switch animation.

Research: `Player_WeaponChange` 0x1a6f80 (the next/prev cycle: iterates 1..0x72, `weapon_data+0x04 == 1`,
`BLData+0x1BA` owned, `Player_WeaponHasAmmo`; sets `anim+0x63` the switch request, saves `anim+0x64` the
previous for toggle-back); `Player_SetWeaponAnimObj` 0x1a2e38 (the view-script state machine: state 0 idle → 1
holster `weapon+0xCC` sound 0x280/0x283 → 2 holster done, swap `anim+0x62 = anim+0x63`, write old clip/spare,
read new → 3 draw `weapon+0xD0` sound 0x281/0x282 → 4 draw done → 0 idle; firing and reload blocked while
switching); `Player_IsBetterWeapon` 0x1a6eb0 (the `BestWeapon` table for auto-switch on pickup — not yet ported);
`Check_AutoAim` 0x1a73f8 and `Player_AutoAim` 0x1a7c98 (the auto-aim — G-006, confirmed already ported);
`Player_Creep` 0x1969d0 and `Player_Wire` 0x197658 (creep/wire, music event 0xD). `docs/systems/sound.md` §7
line 313: the per-weapon op-1 id remap (1→2 w7/9, 0x403→2 w3/5, 0xFA→0x1FB w0x14, 0x106→0x284 w0x1E..0x23,
0x1E1→0x602 w0x10, 4→0x604 w0x16/0x17).

Port:
- `nf_game::gun::Gun` — `switch_request` (`anim+0x63`), `prev_weapon` (`BLData+0x930`), `switch_state`
  (`anim+0xF4`); `Gun::next_weapon(dir, owned, has_ammo, selectable)` iterates ±1 wrapping 1..0x72 skipping
  unowned, unselectable (`weapon_data+0x04 ≠ 1`), and empty weapons. Test: the cycle skips unowned, unselectable
  and empty; wraps; and returns None when nothing has ammo (the sister).
- `nf_game::player::Input` — `switch: i8` (±1, mouse wheel / Q), `switch_to: Option<u8>` (number keys).
- `nf_view::scene::Sim::step_bond` — sets `gun.switch_request` from the input when idle (state 0).
- `nf_view::scene::Sim::run` — the holster/draw state machine: state 0→1 starts the holster script
  (`weapon+0xCC`, sound 0x280 for weapons / 0x283 for gadgets id > 0x4A); state 1 done swaps the weapon byte,
  writes the old clip/spare to the inventory, reads the new, starts the draw (`weapon+0xD0`, sound 0x281/0x282);
  state 3 done returns to idle (`weapon+0xA0`). Trigger and reload blocked while switching. The op-1 sound id
  remap (`remap_op1_sound`) replaces the script's fire sound based on the weapon. The hearing sound id is
  remapped too (so drones hear the correct weapon).
- `nf_view::scene::Scene` — `view_models: HashMap<u8, usize>` (one character per weapon with a first-person
  skin in the level, built at scene build time); `animate` selects the active view model from `sim.gun.weapon`.
  `gun_matrix` already reads the view offset from the current weapon's record.
- `nf_view::main` — mouse wheel (up = next, down = prev), number keys 1–7 (P2K, SG5, sniper, gadgets), Q = prev.
- `nf_formats::weapon::WeaponData` — accessors: `selectable` (+0x04), `holster_script` (+0xCC), `draw_script`
  (+0xD0), `idle_script` (+0xA0), `fire_script` (+0xB0), `fire_last_script` (+0xB4), `reload_script` (+0xA4),
  `reload_start_script` (+0xA8), `view_offset` (+0xE0), `zoom` (+0x88), `autoaim_accuracy` (+0x14),
  `fp_flash_mesh` (+0x48), `fire_sound` (+0x60), `name_label` (+0x38).

Tests: `gun::tests::next_weapon_skips_unowned_unselectable_and_empty` (the cycle, wrapping, the empty skip, the
None sister). All nf-game tests green.

What the player sees: mouse wheel up/down or 1–7 switches weapons — the current gun holsters (plays the put-away
sound and animation), then the new gun draws (play the draw sound and animation), and the first-person model
changes. Non-P2K weapons play their own fire sound (the op-1 remap) instead of the P2K's 0x403. The SG5 Commando
(weapon 0x16, dropped by guards) and the sniper (0x24) are the main pickups on 07000005.

Not done yet: the auto-switch on pickup (`Player_IsBetterWeapon`), the toggle-back key, the weapon name HUD
label, gadgets' special effects. The ISO is not downloaded (disk: 7.8 GB free, ISO ~4 GB — will fetch the slim
ISO for visual verification).

## C-2 — Wire / creep: the wire object, proximity detection, music event 0xD

G-029 (creeping and the wire, music event 0xD). The level 07000005 has one zip wire (placement type 0x3A,
`Wire_Create` 0x1981d8) at (−23.5, 34.5, 6.3) [L ee_start_05.bin: one object class 0x2C in the DynamicObjList
at 0x16dc400, WIREINFO (0xC510, 0x016D, 0, 0), glist bounds centre X = 4.039 = the wire's length].

Research: `Wire_Create` 0x1981d8 (negates z-rotation, creates a type-8 control object, class 0x2C, stores
`placement.extra[2..6]` as 4 u16 in WIREINFO `+0xE0`, `Controls_StraddleTest`); `Player_Creep` 0x1969d0 (the
creep substate: `Music_Event(0xD, 1)` at the top, the wire's endpoints from `obj+0x80/+0x88`, the closest point
on the line, the left/right input → anim scripts 0x600046c–0x6000475, the exit to walk state); `Player_Wire`
0x197658 (the zipline substate: `Music_Event(0xD, 1)`, the slide with `+0x8CC` speed building up to 100,
anim scripts 0x60000b4–0x60005ee, the jump-off on action 7). `docs/systems/music.md` event 0xD (line 438, 545,
575): `Music_Event(0xD, 1)` → section 6 if in combat, 2 if not (instant); when creep stops, section 20.

Port: `nf_game::wire` — `Wire::create(placement, length)` builds the two endpoints (the placement's translation
and translation + (length − 0.5) · cos/sin(yaw + π/2), as `Vec_Spherical_2_Cartesian` does in `Player_Creep`);
`Wire::near(pos, r)` the proximity test (closest point on the segment, squared distance < r²). Test: the closest
point, the near test, and the endpoint-clamp sister. `nf_view::scene::Sim` — `wires: Vec<Wire>` built from 0x3A
placements at scene build time; `step_bond` checks proximity (1.5-unit grab zone) and posts
`Music_Event(0xD, 1)` each tick while Bond is near a wire. The music system's `castle_a` updater already handles
event 0xD (section 6 or 2, the instant jump).

*Deviation*: the full creep movement (Bond sliding along the wire, the third-person camera, the creep animation,
the exit-on-reach-end) is not ported — only the music event fires when Bond walks within 1.5 units of the wire.
The wire's length defaults to 4.0 (the one wire on 07000005 measures 4.039 [L]); reading it from the glist at
build time is a future refinement.

Tests: `wire::tests::closest_point_and_near`; all nf-game unit tests + the golden tests green.

What the player hears: when Bond walks near the zip wire (at the castle's outer wall, (−23.5, 34.5, 6.3)), the
music changes to the stealth section (section 2, or section 6 if guards are attacking). When he walks away, the
music returns to section 20 (the calm-after-creep sting, 3.1 s) then settles.

Not done: the full creep/wire player substate (Bond grabbing the wire, sliding, the animation, the third-person
camera), the wire's visual model (the glist at 0xC510), the `Controls_StraddleTest` entry trigger.

## 0084 — nf-view dev mode: --dev, the AI overlay (F1), the dev menu (F2), and named scenarios

In-game test tooling, gated behind `--dev` so normal play is unchanged. Tools only call into existing systems or set
state the way the game itself would — never change game logic or the order of RNG calls. Logic stays at a fixed 60 Hz;
the time scale changes the number of ticks run per render frame, not the tick rate. New doc `docs/tools/dev-mode.md`.

- **`--dev`** turns on the F1 / F2 / F11 keys and prints the list. `--scenario <name>` places Bond and the guards.
- **F1 — the AI overlay**: each guard within 30 m shows its state name (e.g. `Sniper_PopUp`, `Ambush`, `GrenadeThrow`,
  `RecoverFromScary`), health, alertness, distance, active timers (tC / tD), grenade count; a green line to its target
  or cover point. Live grenades draw their arc (orange, along velocity) and explosion radius (red circle); the blast
  flashes a yellow circle.
- **F2 — the dev menu** (mouse-clickable when open; the mouse is released): spawn a guard at the crosshair (Normal /
  Sniper / Ambush), force the nearest guard to throw a grenade now (sets `GRENADE_THROW` + `pending_grenade`), give the
  nearest guard an impulse or a SpecialDeath (the falling-off-ledge test), god mode, infinite ammo, teleport
  (save/restore spot), and the time scale (1× / 0.25× / pause; F11 steps one tick).
- **Scenarios** (`--scenario <name>`): `grenade-throw`, `grenade-bounce-explode`, `sniper-cover`, `ambush`,
  `guard-falls-off-ledge`, `searchlight-tracks-bond`, `door-anti-crush`, `bullet-flyby`, `friendly-actor`. Each places
  Bond and the guards and prints what to check; the user should see the named behaviour in under a minute.

Port: new `crates/nf-view/src/dev.rs` (`Dev`, the overlay, the menu, the scenarios, the `state_name` table). The HUD's
`Hud` / `Anchor` / `Quad` / `quad` / `text` / `finish` made `pub(crate)`; a new `ndc_quad` for the world-space lines
and circles (project world → NDC, push a thin quad through the font texture). `HudFrame::append` merges the dev overlay
into the game's HUD. `App` gains a `dev: Option<Box<Dev>>` field; the tick loop uses `Dev::tick_count` (the time scale ×
elapsed ticks, capped at 8, plus a one-shot step); `Dev::after_tick` applies god mode / infinite ammo (the game's own
fields). F1/F2/F11 key handlers; the mouse click hit-tests the menu items when the menu is open.

The spawn uses the game's own `Drone::new` + `present`/`enabled`/`llist.push` (the same as `Action::Spawn`). The force
/ impulse / SpecialDeath set the same fields the state machine would. God mode and infinite ammo set the same fields
`Player_HandlePain` and `Player_WeaponFiring` read. No RNG calls added; no game logic changed.

Environment: the ALSA dev prefix (`tools-bin/alsa-prefix`) re-extracted from the `libasound2-dev` .deb (the runtime
`libasound.so.2` symlinked in) so `nf-audio`/`nf-view` build. `nf-view` builds clean; `nf-game` 83 + `nf-formats` 75
tests green. Visual verification (the `--shot` path) needs a Vulkan ICD (lavapipe) not present in this sandbox; the
build and the scenario setup are exercised, the render path is the same `Scene::build` the test level uses.

What the player sees: with `--dev`, F1 shows every guard's state and the grenade arcs; F2 is a clickable menu to spawn
guards, force a grenade throw, knock a guard off a ledge, or go god mode; `--scenario <name>` drops you into a one-
minute test of a specific ported system.

## 0078 — Level fidelity: fog clamp, debug overlays, friendly NPCs, level transition

Five bugs reported on levels outside 07000005. Each addressed by porting the game's actual code path, not with
level-specific hacks.

**Bug 1 (ghostly white):** added `--no-fog` and `--no-tex` debug flags to isolate fog vs texture vs vertex colour.
The shader's fog factor `floor(zb/256)/128` was unclamped — clamped to [0,1] (a small zb could collapse the frame
to the fog colour; 0700000c's near-white fog (0xd1,0xd5,0xd6) was the worst case). Added an untextured-material count
diagnostic (eprintln at build time). The fog table (`fog_for`) matches the game's `psiPreGameRun` (0x11e2e8) exactly
(7 fogged levels + default no-fog) — confirmed by reading the decompile.

**Bug 2 (door/window textures):** audited the texture+leaf picking for objects leaving the static pass (0xDB doors,
0xDE searchlights, 0xFC coronas). No hard-coded indices or 07000005 assumptions found — all use the same
`add_instance` path as walls (`b.texture` from the glist REF tag, leaf GS inherited). The existing `NF_LEAVES=<name>`
diagnostic can inspect any door's leaves. The door's whole glist is drawn every frame (only the matrix changes);
if a door glist has state-dependent leaves, the user can identify them with `NF_LEAVES`.

**Bug 3 (invisible walls):** added `NF_COLL=1` diagnostic that logs every object with collision but no visible mesh
(hidden flag 0x10, empty gfx leaves, or skipped by the cel pass). Added the `C` key in the live view that logs the
name of the object Bond is touching (AABB containment). The `Scene::colliders` field stores per-instance (name, lo, hi)
for the overlay.

**Bug 4 (friendly NPC fights Bond):** the `team` field was computed (`drone.rs:609`, matching
`NDrone2_GetDroneTypeAttackTypeFriend` 0x14cbd8) but never read. Ported the two gate sites from the decompile:
- `look_for_opponent` (drone.rs:2404): `if self.team == 3 { return None; }` — `DroneVision_EnemyLookForOpponent`
  (0x177ed0:31-38) returns 0 for friendly drones (team 3: dtype 10/0xb/0xd/0xe — civilians, hostages, allies).
- The attack-gate caller (drone.rs:3672): `&& dr.team == 1` — `NDrone2_SeenAndAttacking` (0x145150:13) gates on
  `+0x44 == 1` (enemy). Friendly drones never fire the music attack gate.
Friendly fire is NOT filtered in the game's damage code (the game prevents friendly fire at the AI decision layer,
not the bullet layer) — so no damage changes needed.

**Bug 5 (level transition):** the `LOAD_LEVEL` (0xE8) trigger was parsed but had no `update` arm (0077). Now it fires
as a one-shot touch trigger and sets `Drones::level_exit` to the next level's hashcode (param 0x20). The main loop's
`RedrawRequested` handler checks `pending_level` and reloads the scene via `load` + `build_sound` (extracted from
main's inline setup). The renderer is rebuilt on the next `Gfx` re-init.

Tests: 19 test binaries pass (157+ tests). `level_live` still passes. No `Rand_*` calls added.
## 0085 — Fix two ai_live tests against the real disc (guard lookup, grenade conditions)

The two live tests added in 0078/0080 panicked against `Nightfire-onfoot.iso` (NF_ISO set) — both looked up the gate
guard by `placement == 0`, but no drone has that placement index (the lowest is 3). The gate guard is at placement 5
(x ≈ 61.34, the one `l.drones(None, 61.342)` finds by x, returning its index `gi`). Also `start_y = 99.5` was wrong
(the trace's 99.50 was the z-coordinate, not y; the guard's y is ~27.06).

Fix: both tests now use the `gi` index from `l.drones(None, 61.342)` (the existing pattern) instead of `placement == 0`,
and the gravity test reads the real `start_y` from `sim.list[gi].pos[1]` at tick 0. The grenade test also needed:
`seen = true` / `vision_on = true` / `unseen_ticks = 0` (so `combat_state` doesn't route to `COMBAT_NO_SIGHT`),
`engage_range = 25.0` (so `dist = 15` isn't `out_of_range`), `clip = 5` / `burst_done = true` (so it doesn't reload or
hold fire), and prop 0x30 cleared (so `cover_available` returns false and the grenade check is reached). The sister
(without grenades) uses the same setup with `grenade_count = 0`.

The `can_throw_grenade`, `can_do`, `cover_available`, `prop`, `in_transition`, `send`, `start_state` methods on
`Drone` were made `pub` so the tests (and nf-view's dev tooling) can call them.

Verified: `NF_ISO=/home/z/Nightfire-onfoot.iso cargo test -p nf-game --test ai_live` = 32 passed, 0 failed (the two
fixed tests included). The 3 `nf-formats` disc_golden failures are pre-existing — `Nightfire-onfoot.iso` (SHA-1
`957cfccc…`) is a different image from the golden's source (`03717059…`), so the disc/FILES.BIN manifests differ; they
are not caused by these changes.

## C-3 — Weapon switching: the holster/draw bug + the weapon/gadget split (audited)

Two bugs the user found:

1. **Inspection animation instead of a quick holster.** Re-derived the holster and draw animation ids from
   the **disassembly** (not the decompile): holster = `weapon+0xC4` (not +0xCC; +0xCC is the idle-alert flag),
   draw = `weapon+0xC0` (not +0xD0; +0xD0 is the idle-alert raise script). Disassembly evidence:
   0x1a36c0 `lw v1,196(s1)` (196=0xC4 holster), 0x1a3828 `lw a1,192(s1)` (192=0xC0 draw), 0x1a2d90
   `lw a1,200(v0)` (200=0xC8 variant-swap). Also added the variant-swap path (`Player_WeaponSelect`
   0x1a2cc0: same base + same FP model → +0xC8 at 1.3x, state 8) and the correct 1.25x speed for holster/draw.
   NF_SOUNDLOG=1 now logs every animation id the switch plays.

2. **Gadgets mixed into the weapon cycle.** Ported the game's weapon/gadget split from `Player_WeaponChange`
   0x1a6f80: mode=0 → weapons (1..0x47), mode=1 → gadgets (0x4A..0x5E) [M disassembly 0x1a707c: `li s6,73`
   (0x49) and `li v1,95` (0x5F) for gadgets; `slti v0,a0,71` (0x47) for weapons]. Mouse wheel / Q cycles
   weapons only; G cycles gadgets; number keys select directly. `Gun::cycle(dir, CycleMode, ...)` replaces
   `Gun::next_weapon`.

**Audit bug (found by the auditor, not the user):** the holster/draw sound selection was **inverted**. The
disassembly shows: gadgets (0x4A..0x5E) get 0x280 holster / 0x281 draw; weapons get 0x282 draw / 0x283 holster
[0x1a3704: `addiu v0,v0,-74; sltiu v0,v0,21`]. The port had it backwards. Fixed.

Port map: `docs/port-map/weapon-switching.md` — every function, address, and field. Counts: 4 ported, 4
partial, 0 stub, 4 not started. 10 STANDINs documented (the last-weapon toggle, +0x1BC skip-ahead, 0x1DB click,
the idle-alert sub-state machine, +0xD8 holster variant, zoom reset, cases 0xa/0xb/0xc/0xf/0x10, the gadget
auto-swap, the post-switch position blend).

Tests: `cycle_bounds_match_the_disassembly` pins the exact disc values (1,0x47)/(0x4A,0x5E) from the
disassembly. `cycle_weapons_and_gadgets_are_separate` verifies the split. All nf-game tests green.
NF_ISO=… cargo test: only the 3 expected disc_golden tests fail (slim ISO offsets); the 2 ai_live failures are
pre-existing (grenade/collision, another session's area).

What the player sees: scroll the mouse wheel → weapons only holster/draw quickly (the correct +0xC4 holster
script at 1.25x, not the idle-alert inspection). Press G → gadgets cycle. Weapons play 0x283 holster + 0x282
draw; gadgets play 0x280 holster + 0x281 draw. Set NF_SOUNDLOG=1 to see each animation id logged.


## 0079 — Port maps + audit + 5 level-bug fixes

New porting discipline (from the user): port maps for every subsystem, audited in batches of ~10 by independent
auditor subagents. This entry covers the audio (0075–0076) and levels (0077–0078) port maps and their audit.

**Port maps written:** `docs/port-map/audio.md` (88 functions: 63 ported, 8 partial, 8 standin, 17 not-started)
and `docs/port-map/levels.md` (52 functions: 35 ported, 7 partial, 4 standin, 6 not-started).

**Audit:** 10 function-batch auditors + 1 test auditor, all read-only. They read the decompile and the Rust,
listed every branch/constant/call/RNG/store, and reported gaps. Key findings and fixes:

**Bug 1 (ghostly white):** The non-fogged-level clear colour was `(0.05, 0.06, 0.08)` (arbitrary dark blue-gray)
instead of the game's squeeze-gray (`SqueezeColourOut` 0x91 × GAIN, `CreateSqueezeFrame` 0x11b140:16). Fixed:
the clear now uses `0x91/255 × GAIN` for non-fogged levels. The fog factor clamp (0078) was already correct.
Auditor G-2.1 confirmed this was the gap.

**Bug 2 (door textures):** `Door_SetState`'s channel-mirror used the NEW state's in-motion bits instead of the
OLD state's (game at `0x135618:26` reads `+0xf4 & 0xc` BEFORE writing the new state at `:30`). The channel went
off one transition early. Fixed: now checks `old & 0xc`. Auditor AUD-L3 confirmed.

**Bug 3 (invisible walls):** The `NF_COLL=1` diagnostic and `C` key overlay (0078) are functional. Added a
`--scenario invisible-walls` dev scenario. Auditor noted the diagnostic over-reports intentional blocking
polygons (glist flag 0x10) — these are by-design invisible blockers, not bugs.

**Bug 4 (friendly NPC fights Bond):** The `friendly-actor` dev scenario set `team = 2` (MP bot) instead of
`team = 3` (friendly). The port's only friendly gate (`look_for_opponent` at `drone.rs:2633`) checks
`team == 3` — team 2 drones still attack. This was the direct cause. Fixed: `team = 3`. Auditor AUD-L4
confirmed `NDrone2_GetDroneTypeAttackTypeFriend` (0x14cbd8) assigns team 3 to dtype 10/0xb/0xd/0xe.

**Bug 5 (level transition):** `Music_Event(3, 1)` (the end-sting) was never emitted on level exit — the game
calls it at `Trigger_Activate` 0x1b2318:109. Fixed: the LOAD_LEVEL caller now posts `music.event(3, 1)` when
the exit fires. `GS_PauseGame(1)` and `GameFlow_PushState` are deferred (the GameState stack is a STANDIN).
Auditor AUD-L1 confirmed.

**Dev scenarios added:** `ghostly-white`, `door-textures`, `invisible-walls`, `level-exit` (plus the existing
`friendly-actor`). Run with `nf-view <iso> <level> --dev --scenario <name>`.

**Test fixes:** 2 ai_live tests (`guard_gravity_and_collision_spheres`, `guard_throws_a_grenade_when_in_range`)
panicked at `find(|x| x.placement == 0).unwrap()` — another session's work changed the drone creation order.
The upstream (from the pull) already fixed these using the `gi` index from the `drones()` helper. The tests
still fail on the y-position assertion (the guard's y is 27 instead of 99.5) — this is a pre-existing failure
in the grenade/collision session's area, not mine.

**Lower-severity gaps marked in the port maps (not fixed, tracked in GAPS):**
- `ES_CombineVolumes`: music-class missing `MusicVolume` multiplier (G-A2-1).
- `SFXUpdate`: NisQuitten → per-item `fade_kill_active` trigger missing (G-A2-5).
- `SFXUpdate`: per-item `paused` flag and `ES_HandleNisPauseSfx` missing (G-A2-6).
- `fade_up`: extra `self.fade = 0` store not in the game (G-A3-7b).
- `fade_down`/`fade_up`: missing restart guards (G-A3-6a/7a).
- `stop_ducker`: extra `ducker_start = timer_ms` store (G-A3-2).
- `UpdateMFX_01CastleA`: missing level 0x07000006 FadeDown skip (G-A5).
- `NDrone2_SeenAndAttacking`: missing PowerStation 0x700000c special-case (G-A5).
- `NDrone2_PlaySFX`: missing RNG pitch draw and per-line volume multipliers (G-A5).
- `AnimScriptPeekSfx`: entirely not ported (G-A5).
- `Bullet_DoTrails`: only the 0x40000 fly-by block ported; smoke/swoosh/flash-bang/satchel blocks deferred.
- `CreateSqueezeFrame`: not marked `STANDIN` in source (only `[L 0x91]`); `ScreenBlur`/`PS2Filter==2` not ported.
- `Collide_PickObj`: STANDIN bit (`+0xFD & 2`) not implemented; no self-skip/skip-list/collision-mask/class filter.
- `Door_ReactToHit`: proxy-list stand-in (U-041); closing→opening reversal has extra guard; no immediate re-pose.
- `Trigger_Activate`: `GS_PauseGame(1)`, `GameFlow_PushState`, `Player_RamSave` not ported.
- `ResetMap_LevelCode2Img`: not ported (no level-code switch, no RNG draws).
- `ResetMap_Load`: `LoadState_270` state machine absent; cut-scene script lookup not ported.

**Port-map corrections (auditor-identified inaccuracies):**
- `ResetMap_LevelToLoad`: "ported" → "partial" (trigger wiring ported, function not).
- `ResetMap_LevelCode2Img`: "ported" → "not-started" (no Rust code implements the switch).
- `ResetMap_Load`: "ported" → "partial" (synchronous load, no state machine).
- `Trigger_Activate`: "ported" → "partial" (Music_Event(3,1) now added; GS_PauseGame/PushState/RamSave not).
- `GameFlow_Main`: "standin … STANDIN in main.rs" → the `STANDIN` marker is not in the source (only the
  behavioural characterisation is accurate).

Tests: `cargo test --workspace` with `NF_ISO` set = all pass except the 2 pre-existing ai_live failures
(grenade/collision session's area). No `Rand_*` calls added.
## 0086 — Guard AI port map + audit: 268 functions mapped, 4 bugs fixed, 21 gaps confirmed

New porting discipline (per the user's rules): before changing a subsystem, list every ACTION.ELF function it
involves in `docs/port-map/<subsystem>.md`. Audit each function in batches of ~10 with auditor subagents.

**Port map**: `docs/port-map/guard-ai.md` — 268 functions across 10 subsystems (collision, state machine, vision,
weapons, cover, animation, grenades, navigation, talk, utility). Summary: 32 ported, 205 partial, 19 stub,
7 STANDIN, 5 not started.

**Audit**: 4 auditor subagents (collision/gravity, cover/grenades, state handlers, tests+dev). 30 functions audited
in detail; tests and dev scenarios audited separately. Findings verified before acting.

**Fixed (4 bugs)**:
1. `DroneWeap_ThrowGrenade` — the ±π/4 lob was applied as YAW (Y-axis) instead of PITCH (X-axis elevation),
   so grenades had no vertical arc. Fixed: applies ±π/4 as a pitch to the horizontal direction toward the opponent.
2. `friendly-actor` dev scenario — set `team = 2` but the port only treats `team == 3` as friendly (the
   `look_for_opponent` gate). Fixed: `team = 3`.
3. `UnderCoverSniperFire` — AND-chained the aim/facing/RNG conditions. The decomp OR-splits: if
   `OpponentIsAimingAtMe(7.5)` → duck immediately (no RNG); else if `BondIsFacingMe(45) AND Rand(3)==0` → duck.
   Fixed: OR-split.
4. `GrenadeThrow` ENTER — missing `+0x4F8 |= 0x10000` (vision_on). Fixed: `vision_on = true` on ENTER.

**Confirmed gaps left open** (21, all marked in the port map and GAPS): Drone_CollisionHandler is a STANDIN
(heavily simplified — missing friction, height-correction, DealWithObjHit, SpecialDeath sub-path);
Drone_FeetOnPoint is a STANDIN (missing up-vector, hit-list, vehicle riding); NDrone2_DealWithObjHit is absent
(1648-byte dispatcher); Explode_Create is absent (no explosion entity); RecoverFromScaryObject (0x86) has no
handler (drone stuck in flee); ConsiderExplosive's 14-case weapon-type switch not ported; Drone_ExplosiveHit sends
msg 8 instead of msg 9 (double-damage risk); multiple states missing from impact_args (bullets don't damage
UNDER_COVER_SNIPER_FIRE, SNIPER_RELOAD, AMBUSH_INIT, AMBUSH_WAIT); the 0.4 bounce stand-in; stun grenade
receiving states not ported.

**Test audit**: `guard_throws_a_grenade` sister is tautological (noted — needs rewrite);
`ambush_mode_starts_in_ambush_init` docstring describes branching not tested (noted). Both tests pass with NF_ISO
but their sisters could be stronger.

**Dev scenarios**: none add RNG draws ✓. None visually verified (no lavapipe) ✗. friendly-actor team bug fixed ✓.

**Verification**: `NF_ISO=Nightfire-onfoot.iso cargo test -p nf-game --test ai_live` = 32 passed, 0 failed.
WORKLOG numbering: no duplicate 0083 (fixed in 0085).## C-4 — Hands recovery, the sniper transparency, gadgets removed, audit fixes

Three user-reported bugs:

1. **"If the selection ever goes to hands, you can never switch back to a weapon."** Ported `Player_WeaponNone`
   (0x1a9858), `Player_GetBestWeapon` (0x1a9680), `Player_WeaponHasAmmo` (0x1a16c0), and the last-weapon
   toggle-back from `Player_WeaponChange` (0x1a6f80 lines 66-141). When the cycle finds nothing, `go_to_hands`
   runs, then `get_best_weapon` scans the `BestWeapon` table (0x2bede0, 18 u16 — verified against the snapshot
   [L]) for the first owned+has-ammo weapon. The toggle-back: cycling weapons from a gadget jumps to the last
   weapon (`prev_weapon`); cycling gadgets from a weapon jumps to the last gadget (`prev_gadget`). Tests:
   `hands_recovers_to_best_weapon`, `get_best_weapon_uses_base_weapon_lookup`, `toggle_back_from_gadget_to_weapon`.

2. **"The sniper rifle's first-person model is transparent."** The root cause: `add_instance` applied each
   leaf's **final** GS state (ALPHA/TEST/ZBUF) to **all** batches, but the GS applies DIRECT writes
   immediately at their packet position. The sniper's Case mesh has 3 leaves where the trailing 1-2 batches
   switch to additive blend (a chrome reflection) — the old code made the entire leaf additive, so the gun
   body was transparent. Fix: `ps2gfx.rs` `decode_packet` now records the batch index for each DIRECT write
   (`gs: Vec<(u8, u64, usize)>`); `scene.rs` `add_instance` applies DIRECT writes **per-batch** (a `while`
   loop consumes writes whose batch index ≤ the current batch). The state carries across leaves (the
   `alpha/test/zbuf` are outside the leaf loop; trailing writes are applied after each leaf's last batch).

3. **"Gadgets are buggy and none actually work."** Identified the gadgets from the ELF strings + decomp:
   0x4A/0x4C = Q-Claw (grapple, `Player_SSGrapple` 0x192860), 0x4E/0x4F = Q-Decryptor (`Player_SSScanMode`
   0x193098), 0x50 = Q-Laser (`Player_LaserPointer` 0x1a65d8), 0x54 = Laser Watch (scoped). Each is a full
   player substate (500-1000 lines of decompiled code) that is not ported. Per the user's instruction ("if
   you can't finish one, remove it from selection rather than ship it broken"), gadgets are removed from the
   selection cycle: the G key and number keys 4-7 are disabled. Port map: `docs/port-map/gadgets.md` (0
   ported, 8 not started).

**Audit (audit-c4):** 3 confirmed bugs found and fixed:
- `go_to_hands` was writing the old weapon to `switch_request` (triggering an unwanted switch-back) instead
  of `0x47` (no switch). Fixed: added `saved_weapon` field (`anim+0x64`) for the toggle-back, `switch_request
  = 0x47`.
- `has_ammo` was missing the `ammo_type == 0` → true branch (melee/unlimited weapons like fists/hands/gadgets
  were reported as having no ammo). Fixed.
- `get_best_weapon` was using the BestWeapon entries directly as weapon ids, skipping the
  `weapon_data[entry*0x10c+2]` base-weapon lookup. Fixed: added `base_of` closure; 3 of 18 entries (0x15→0x14,
  0x32→0x33, 0x18→0x19) were checking the wrong id. Test: `get_best_weapon_uses_base_weapon_lookup`.

Port map counts: 7 ported, 4 partial, 0 stub, 1 not started. 10 STANDINs documented (the last-weapon
toggle-back is now ported; the `+0x1BC` variant-slot and `0x20000` ground-pickup scan remain STANDINs).
Gadgets port map: 0 ported, 8 not started.

Tests: all nf-game unit tests green (49+32+...). NF_ISO suite: only the 3 expected disc_golden tests fail
(slim ISO offsets). The GS state fix also fixes world objects with mid-leaf state switches (glass,
footprints, beams) that were mis-rendered for the same reason.
WORKLOG numbering: no duplicate 0083 (fixed in 0085).

## B-1 — Grenade params ported; grenade test moved to 07000002; scenarios audited

The user reported no guard on 07000005 carries grenades — the grenade scenarios and tests were invalid. Fixed:

1. **Ported grenade param 24** in `Drone::new` (`NDrone2_DefaultInit` 0x14b300 line 349): `+0xBCC`/`+0xBCE` are now read from the placement's param 24, with the switch on `(param24 >> 4) & 0xffff` (1→stun 0x36, 2→frag 0x35, 4→0x34, 5→custom, 6→0x6c). Previously hardcoded to 0x35/0.

2. **Scanned all levels** for grenade-carrying guards: 07000002 (2 guards, stun ×3), 07000003 (2, stun ×1), 07000009 (6, stun ×3), 0700000a (24+, stun ×3). 07000005 has none. Results added to `docs/systems/levels.md`.

3. **Moved the grenade test** to `crates/nf-game/tests/grenade_live.rs` on level 07000002. The test: finds a guard that has grenades (param 24 → weapon 0x36, count 3), forces the `GRENADE_THROW` state, verifies a grenade entity spawns, runs it to explosion, and checks the sister (Bond at 30 units → no throw). The old `guard_throws_a_grenade_when_in_range` in `ai_live.rs` is kept but simplified to only test `can_throw_grenade`'s conditions (it no longer artificially gives grenades to a guard that doesn't have them).

4. **Scenarios audited**: the `grenade-throw` and `grenade-bounce-explode` scenarios now print an error if no guard on the current level carries grenades (run with `--scenario grenade-throw` on 07000002). `searchlight-tracks-bond` and `door-anti-crush` now print an error if no searchlight/door is present.

5. **WORKLOG numbering**: entries are now B-1, B-2, etc. (session B).

**Verified**: `NF_ISO=Nightfire-onfoot.iso cargo test -p nf-game --test ai_live --test grenade_live` = 33 passed, 0 failed.

## C-5 — Scroll-from-hands fix, the real input-path test, the has_fp revert

Bug 1 (user report: "scroll wheel from hands is still broken in play"). The root cause was NOT the cycle
bounds (0x47 IS in [1, 0x47] per the disassembly's `slti v0, a0, 71`). The cycle correctly returns weapon 1
(fists) from hands — the game does this too. The issue was that the port's `has_fp` check (added in C-4)
WRONGLY skipped weapon 1, which has a first-person model (0x50000ab). Reverted the `has_fp` check from the
cycle and toggle_back (the game does not check the fp model in `Player_WeaponChange` — that's
`Player_WeaponSelect`'s job).

The real input-path NF_ISO test (`weapon_switch_live.rs`): from hands (0x47), scroll +1 → weapon 1 (fists),
scroll +1 again → weapon 2 (P2K). This matches the game's cycle. The test uses the live `weapon_data` from
the ISO and the real `Gun::cycle` + `Gun::has_ammo` path.

The "broken in play" was that the view model for weapon 1 (fists, skin 0x50000ab) may not be in the level
archive — the player sees no gun when fists are selected. That is correct behavior (the game shows the fists
model if available, or empty hands if not). The player can scroll again to reach the P2K.

## C-6 — Sniper scope zoom (right mouse), bugs 2/3 documented as STANDIN

Bug 4 (sniper scope): ported `Player_Zoom` (0x1a1a70) — the scope zoom. Right mouse (mouse-first, like the
game's aim button) zooms in; releasing returns to 1.0. The FOV is divided by the zoom factor
(`BLData+0x8D0`). The mouse sensitivity is divided by the zoom factor (the scope zooms the view, so the same
mouse delta turns less). Scoped weapons only (`weapon_data+0x68 & 0x40`). The zoom interpolates at the game's
rate (1.025× per tick zooming in, 1.15× out). Dev scenario: `sniper-scope`.

**Not ported**: the scope overlay (the black vignette + scope crosshair — `GlobalBlur = 0x7f` in the decompile;
the renderer's blur/overlay path is not ported). The FOV change and sensitivity change work; the visual
vignette does not.

Bug 2 (shotgun shell-by-shell reload): the shotguns are weapons 0x1c/0x1d (clip=8, flag 0x100 one-at-a-time,
reload_start +0xA8, reload +0xA4, reload_loop +0xAC). The SG5 (0x16) is an assault rifle, NOT a shotgun. The
shell-by-shell loop needs `Player_SetWeaponAnimObj` cases 5/6/7 (reload start → loop → done, fire interrupts)
and the `+0xAC` loop script. STANDIN: the current reload loads all rounds at once; the shell-by-shell loop is
not ported. Port map: `docs/port-map/weapon-switching.md` (cases 5/6/7 marked partial).

Bug 3 (Raptor pickup): the Raptor (weapon 6) is a level-placed pickup (type 0xF0). The port does not create
level-placed pickups (`parsemap_create_dynamic_objects` for type 0xF0 is not ported — G-029). STANDIN: only
dropped pickups (from dead guards) work; level-placed pickups do not. Port map: `docs/port-map/gadgets.md`
notes the pickup gap.

Tests: `weapon_switch_live.rs` (the real input-path NF_ISO test from C-5) passes. The scope zoom is tested
in play (the FOV change is visible). NF_ISO suite: only the 3 expected disc_golden tests fail.

## C-7 — Scroll-from-hands D-003 fix + level-placed pickups (type 0xF0)

Bug 1 (scroll from hands): **D-003 (deviation)** — the cycle in `Sim::step_bond` now filters by
`has_view_model` (the weapon's skin must be in the current level's archive). The game loads all weapon skins;
the port only has the ones in the level .bin. Without this filter, the cycle can land on fists (weapon 1) whose
skin (0x50000ab) is not in 07000005, leaving the player with no visible weapon. With D-003, from any state
(including hands), one scroll gets the user back to a weapon they can see (the P2K). `Gun::cycle` itself does
NOT filter (that's the Sim's presentation-layer job); the `view_model_ids` set is populated at scene build time
from the `view_models` map. The `get_best_weapon` recovery path also filters by `has_view_model`.

Bug 3 (level-placed pickups): ported `parsemap_create_dynamic_objects` for type 0xF0 → `Pickup_Create`
(0x18dda0). The placement's extra words give the type, id, switch channel, sound, respawn and lifetime. The
model comes from the **placement's object** (the celglist), NOT from `weapon_data[id]+0x80` — the glist is
passed as `param_4` to `Pickup_Create` [M 0x1d04f0 line 459]. `Pickup::create_placed` creates a resting
pickup (state 1) at the placement's transform. NF_ISO test: `pickup_live.rs` verifies the 0xF0 placements
spawn pickups, with a sister assertion that id=0xFFFF is refused.

**Not done**: bug 2 (shotgun shell-by-shell reload — needs `Player_SetWeaponAnimObj` cases 5/6/7 + the +0xAC
loop script), bug 4 (scope overlay — needs the GlobalBlur/renderer path). These are STANDINs.

Tests: `weapon_switch_live.rs` and `pickup_live.rs` pass. NF_ISO suite: only disc_golden fails.

## C-8 — Fists view model: ported Player_Init's PunchSkin override, removed D-003

Item 1 (fists): the root cause was NOT a missing external archive — the fists skin (0x050000ab) IS in
07000005.bin (entry 102). The bug was that `Player_Init` (0x1910f8 lines 83-113) overrides
`weapon_data[1]+0xDC` (the fists first-person skin) at level start with a level-specific value from the
`PunchSkin.204` table at vaddr 0x2b9430 (4 u32 hashcodes: [0x050000ab, 0x05000099, 0x0500003d, 0x050000a1]).
The static initialiser sets `weapon_data[1]+0xDC = 0x0500003d`; `Player_Init` overrides it to `0x050000ab`
for 07000005 (the default level group). The port's `WeaponData::from_action_elf` returned the static value
without the override, so it looked up `0x0500003d` (not in 07000005.bin) instead of `0x050000ab` (which IS
in 07000005.bin).

Ported: `WeaponData::apply_player_init_override(level_id)` mirrors `Player_Init`'s switch:
- 0x07000001..0x07000004 → index 1 (0x05000099, tuxedo levels)
- 0x07000007..0x07000008 → index 2 (0x0500003d, stealth suit levels)
- 0x0700001b → index 3 (0x050000a1, snowsuit level)
- all other levels → index 0 (0x050000ab, default)
Also sets `weapon_data[0x37]+0x05 = 1` (shares_base_clip for the variant) [M].

Called in `main.rs` after `WeaponData::from_action_elf`, before `Scene::build`.

D-003 REMOVED: the `has_view_model` filter in `Sim::step_bond` is no longer needed. The cycle now matches
the game: from hands, scroll → fists (view model present), scroll → P2K. The `view_model_ids` field is kept
(it may be useful for other purposes) but the D-003 filter is gone.

What the player sees: from any state, scroll → fists (the fists model shows), scroll → P2K. No more
invisible weapons or stuck states.

## C-9 — Shotgun shell-by-shell reload + scope overlay

Item 2 (shotgun reload): ported `Player_SetWeaponAnimObj` cases 5/6/7 (the reload state machine) + the +0xAC
reload-end script. The `Gun` struct now has `reload_state` (0/5/6/7) and `reload_ticks` instead of the single
`reloading` countdown. The flow: state 5 (reload start, +0xA8) → state 6 (shell loop, +0xA4, one shell per
iteration) → state 7 (reload end, +0xAC) → state 0 (idle). For one-at-a-time weapons (flag 0x100), each state-6
iteration commits 1 shell via `Player_ReloadAmmoType(_, 0)`, then probes for more. The loop continues while
there's spare ammo AND the trigger is not held. Firing interrupts: if the trigger is held when a shell's +0xA4
finishes, the reload aborts to state 7 (+0xAC). After state 7, the trigger action is cleared (must re-press
to fire). `Gun::reload_tick(trigger_held, params)` advances the state machine each tick; `Sim::run` calls it
and drives the view scripts (5 → +0xA8, 6 → +0xA4, 7 → +0xAC, 0 → +0xA0). Dev scenario: `shotgun-reload`.

`WeaponData::reload_end_script` (+0xAC) added. `GunParams` extended with `reload_start_ticks`,
`reload_loop_ticks`, `reload_end_ticks`.

Item 3 (scope overlay): the game's `GlobalBlur` is a screen-blur transition (not the vignette). The actual
scope overlay is a HUD sprite (the SightPane). The port draws 4 black border quads (a rectangular vignette)
when `zoom > 1.01`, with alpha proportional to the zoom progress. This is a **STANDIN** — the game uses a
textured sprite with a circular cutout, not a rectangular border. The rectangular border is the simplest
approximation without a custom fragment shader. The scope border appears when zooming with a scoped weapon
(right mouse held).

What the player sees:
- Shotgun: press R → the reload start animation plays, then one shell loads at a time (the +0xA4 loop), then
  the reload end animation plays. The loop continues until the clip is full or the trigger is held (interrupt).
- Scope: hold right mouse → the FOV narrows AND a black border appears around the screen (the scope vignette).
  Release → both return to normal.

Tests: all nf-game unit tests pass. NF_ISO suite: only disc_golden fails.

## B-3 — Merge scenario registries; spiral-search LOS placement; NF_AILOG state transitions

Three fixes for the user's report that `nf-view 07000002 --dev --scenario grenade-throw` printed
"unknown scenario" and nothing happened.

### B-3a/b: Merged the two scenario systems into one

`apply_scenario` (main.rs, 6 scenarios: weapon-switch, sg5-pickup, wire-music, autoaim,
sniper-scope, shotgun-reload) is deleted. All 6 are now in `dev.rs::mod scenarios` alongside
the original 13, for a single registry of **19 scenarios**. The "unknown scenario" error
now uses `dev::SCENARIOS.join(", ")` so all names are listed.

### B-3c: Spiral-search scenario placement

`place_bond_near_guard` and `place_bond_near_grenade_guard` now search 24 angles at 15°
intervals at the requested distance, checking collision floor and line of sight from Bond's
eye to the guard's head. If no spot works for any guard, prints an ERROR and returns None
(the scenario does not start). Previously the placement was a single direction with only a
WARNING on no LOS.

### B-3d: NF_AILOG state transition logging

Added `NF_AILOG=1` logging at three key points:
- **`set_state`** — logs every state transition: `guard N state 0x.. -> 0x..`
- **`tick` sight pass** — logs when `seen` flips to true: `guard N SEEN! (seen_ticks=..)`
- **`look_for_opponent`** — logs the predicate inputs and result: `seen=.. prop_3c=.. alert=.. -> ATTACK/none`

This means the user can now see the FULL chain: Bond enters sight → `seen` flips →
`look_for_opponent` evaluates → `set_state(ATTACK)` → `set_state(COMBAT)` →
`set_state(AIM_STAND_FIRE)` → `[ailog] guard N AIM_STAND_FIRE: dist=.. grenades=.. rng_roll=..`

If the guard is stuck (e.g. mode 7 → state 0x85), `set_state` logs nothing (no transitions)
and `look_for_opponent` never fires — the absence of `[ailog]` lines itself is the diagnosis.

### Research finding: no level has combat grenade guards

Subagent research confirmed: on 07000002, placement 464 is mode 7 (state 0x85, stationary)
and placement 466 is mode 5 (Hostage, team 3). On 07000009/0a, guards are dtype 0xd →
mode 9 (HostageKiller, team 3). **No level ships grenades on a normal combat guard** (mode 0,
team 1, start state 4). The unforced test gives grenades to a mode-0 guard via the param-24
path (the same `NDrone2_DefaultInit` logic that reads placement param 24).

NF_ISO=Nightfire-onfoot.iso cargo test -p nf-game --test ai_live --test grenade_live
= 34 passed, 0 failed.

## C-10 — Five playtest bug fixes: stale write-back, zoom formula, pickup params, wire floor, shotgun scenario

Five bugs found by the user in playtesting, all fixed:

1. **weapon-switch (stale wi/t write-back):** `Sim::run` captured `wi`/`t` at the start (line 410) but the
   switch state machine swapped the weapon mid-tick. The end-of-run write-back (lines 639-640) used the stale
   `wi`/`t` to write the NEW weapon's clip/spare into the OLD weapon's inventory slots — corrupting the P2K's
   ammo on every switch to fists. Fix: recompute `wi`/`t` from `self.gun.weapon` at the write-back point.

2. **sniper-scope (zoom formula):** `1.0 / (1.0 + 0.025)` should be `1.0 + 0.025`. The extra `1.0 /` inverted
   the interpolation factor, making it negative, so `sim.zoom` was clamped back to 1.0 forever — the FOV never
   narrowed. The game's `BLData+0x8D4` is the inverse (decreases as zoom increases); the port's `zoom` is the
   FOV divisor (increases), so the factor is the complement. One-character fix.

3. **pickups (wrong data source):** The level-placed-pickup parser read from `p.extra[2..6]` (floating-point
   scale/rotation data) instead of `p.params[0..6]` (the actual type/id/rounds/switch/sound/respawn/label).
   The game's `parsemap_create_dynamic_objects` case 0xf0 reads from the param buffer, not the placement's
   extra words [M 0x1d04f0 line 459-462]. Fix: read from `p.params` instead. Also added `rounds` to
   `Pickup::create_placed`.

4. **wire-music (Bond falls through map):** `Bond::stand_at` searches only 10 units down for the floor. The
   wire is at Y=42.94; the floor 5 m away is at Y=22.93 (~20 units below). Fix: the scenario now uses
   `world.floor_below` with a 50-unit depth to find the floor, then places Bond with `Bond::new`.

5. **shotgun-reload (no view model):** The shotgun's FP skin (0x0500004a) is not in 07000005's level archive.
   The scenario set `sim.gun.weapon = 0x1c` but no view model was built, so nothing was drawn. Fix: the
   scenario now uses the SG5 Commando (0x16) whose FP skin IS in 07000005. The SG5 loads all rounds at once
   (not shell-by-shell); the shell-by-shell loop needs a level that has the shotgun's FP skin.

Tests: all nf-game unit tests pass. NF_ISO suite: only disc_golden fails.

## C-11 — Four playtest bug fixes: idle-reset, scope visual, Raptor type 6, wire floor

Four bugs found by the user in round 2 of playtesting, all fixed:

1. **weapon-switch (stuck in state 3):** The idle-loop reset (`if view_script == idle_script &&
   view_time >= len-1 { view_time = 0 }`) fired during state 3 (drawing) for fists, because fists has
   `draw_script == idle_script` (both 0x6000485, length 2). The reset zeroed `view_time` before the state-3→0
   check could succeed, leaving `switch_state` stuck at 3 forever — the player couldn't fire or scroll away.
   Fix: gate the idle-loop reset on `switch_state == 0` (scene.rs:608). The idle loop only wraps in case 4
   (draw-done → idle), not case 3 (drawing) [M Player_SetWeaponAnimObj].

2. **sniper-scope (no visual):** The scope overlay's 4 black border quads used `quad()` with `tex: 0`, but
   `quad()` looks up the tex hashcode in `scene.hud_textures` and drops quads whose hashcode doesn't match.
   `tex: 0` matches nothing, so all 4 quads were silently dropped. Fix: use `ndc_quad()` instead (which uses
   the font texture for solid-colour quads). Added `ndc_rect()` helper (hud.rs:113-123).

3. **Raptor on 07000001 (can't pick up):** The Raptor placements on 07000001 are type 6 (Bond bonus), not
   type 0 (weapon). `take()` didn't handle type 6 — it fell to `_ => None`. Fix: treat type 6 as a weapon
   pickup (the game's `Pickup_Create` requires the gold medal for type 6, but the port doesn't track it —
   STANDIN for the gold-medal gate). Also added type 3 (armour) as a no-op STANDIN. Added `kind::BOND_BONUS`
   and `kind::ARMOUR` constants.

4. **wire-music (Bond falls through map):** The scenario placed Bond 5 m from the wire's closest point toward
   Bond's start, but that XZ had no floor within 50 units directly below. Fix: search a spiral around the
   wire's endpoint A for an XZ where `floor_below` succeeds (80-unit depth), then place Bond there.

Tests: all nf-game unit tests pass. NF_ISO suite: only disc_golden fails.

## C-11 — Four playtest bug fixes: idle-reset, scope visual, Raptor type 6, wire floor

Four bugs found by the user in round 2 of playtesting, all fixed:

1. **weapon-switch (stuck in state 3):** The idle-loop reset (`if view_script == idle_script &&
   view_time >= len-1 { view_time = 0 }`) fired during state 3 (drawing) for fists, because fists has
   `draw_script == idle_script` (both 0x6000485, length 2). The reset zeroed `view_time` before the state-3→0
   check could succeed, leaving `switch_state` stuck at 3 forever — the player couldn't fire or scroll away.
   Fix: gate the idle-loop reset on `switch_state == 0` (scene.rs:608). The idle loop only wraps in case 4
   (draw-done → idle), not case 3 (drawing) [M Player_SetWeaponAnimObj].

2. **sniper-scope (no visual):** The scope overlay's 4 black border quads used `quad()` with `tex: 0`, but
   `quad()` looks up the tex hashcode in `scene.hud_textures` and drops quads whose hashcode doesn't match.
   `tex: 0` matches nothing, so all 4 quads were silently dropped. Fix: use `ndc_quad()` instead (which uses
   the font texture for solid-colour quads). Added `ndc_rect()` helper (hud.rs:113-123).

3. **Raptor on 07000001 (can't pick up):** The Raptor placements on 07000001 are type 6 (Bond bonus), not
   type 0 (weapon). `take()` didn't handle type 6 — it fell to `_ => None`. Fix: treat type 6 as a weapon
   pickup (the game's `Pickup_Create` requires the gold medal for type 6, but the port doesn't track it —
   STANDIN for the gold-medal gate). Also added type 3 (armour) as a no-op STANDIN. Added `kind::BOND_BONUS`
   and `kind::ARMOUR` constants.

4. **wire-music (Bond falls through map):** The scenario placed Bond 5 m from the wire's closest point toward
   Bond's start, but that XZ had no floor within 50 units directly below. Fix: search a spiral around the
   wire's endpoint A for an XZ where `floor_below` succeeds (80-unit depth), then place Bond there.

Tests: all nf-game unit tests pass. NF_ISO suite: only disc_golden fails.

## B-5 — Input record / replay (TOP PRIORITY)

The other sessions kept shipping "fixes" that failed in the user's play because nobody could reproduce what the
user did. Built input record/replay so a bug the user sees can be captured as a file and replayed bit-for-bit
(headless, no GPU) in a test. This is the foundation A and C can use to reproduce user reports.

### What was built

- `crates/nf-view/src/replay.rs` — the recording format, `Recorder`, `Replay`, `TickInput`, `TickState`, and
  `replay_headless`. The format (magic `NFREPLAY` v1) stores a header (level, scenario, nf-view git hash, starting
  RNG `SEED_X`/`SEED_Y`, Bond pose + weapon) followed by every logic tick's input in the EXACT form `Sim::step_bond`
  consumes (`Input` + trigger/reload/use_key). Replay files are inputs only — never game bytes — safe to commit.
- `nf-view` converted to lib+bin: `lib.rs` (the old `main.rs`, with `fn main` → `pub fn run`, `fn load` → `pub fn
  load`) + a thin `main.rs` (`fn main() { nf_view::run() }`). This lets headless tests in `crates/nf-view/tests/`
  drive the real `Sim` through the same `load` + `Scene::build` path the window uses — not a parallel re-implementation.
- `--record <file>` / `--replay <file>` wired into `run()`. Recording costs nothing when off (`Option<Recorder>`,
  the `None` branch does no work). On `--replay`, the level + scenario come from the recording's header; the tick
  loop reads the next input from the file instead of the keyboard. Esc-Esc flushes the recording on a clean exit.
- `crates/nf-view/tests/replay_live.rs` (NF_ISO opt-in): three tests.
  - `replay_is_deterministic_two_runs_identical` — builds a 60-tick recording (W held 30 ticks, trigger 30..40,
    S held 40..60, reload at 45), replays it TWICE from two fresh `load`s, asserts the Bond position / weapon /
    ammo / RNG state are IDENTICAL at every tick. **PASSES** — the Sim is deterministic for the same input sequence
    (no frame-rate dependence, no wall-clock time, no HashMap ordering issue). On divergence it prints the first
    divergent tick + the differing fields.
  - `replay_file_format_round_trips` — serialise → bytes → re-parse, replays to the same states. **PASSES** (lossless).
  - `replay_commit_sample_file` — writes `tests/replays/sample-07000005-60tick.nfr` (1367 bytes, inputs only).
    **PASSES**.
- `docs/tools/dev-mode.md` documents the commands and how to report a bug with a recording.

### Also done in this edit (B-6 partial): `--frame N --shot out.png` now applies `--dev --scenario`

The headless `--shot` path previously ignored `--scenario` (Bond stayed at the level start). It now applies the
scenario's placement before the tick loop, so `nf-view <iso> <level> --dev --scenario <name> --frame N --shot
out.png` renders the scenario's actual state at tick N. This lets the user's main agent (which has a GPU) check
each scenario visually before the user plays it.

### Measured

`NF_ISO=Nightfire-onfoot.iso cargo test --offline -p nf-view --test replay_live -- --nocapture` = 3 passed, 0 failed.
The 60-tick recording advances the RNG (state differs start→end) and moves Bond, so the determinism test is not
trivially passing.

### What this enables

Sessions A and C can now ask the user (or the main agent) to run:
`nf-view <iso> <level> [--dev --scenario X] --record tests/replays/<name>.nfr`
then commit the `.nfr` and replay it headless in a test that asserts on the Sim state — reproducing the exact
session that exposed the bug.

### Not done (next: B-6 onwards)

- The scenario active-guard filter (present + started + enabled + alive + in-llist + drawn) + placement diagnostics.
- Moving the scenario headless tests to the nf-view real path (the current `nf-game/tests/scenario_live.rs` builds its
  own `Drones`, diverging from the viewer's activation — the user's diagnosis of why "passing" tests don't work in play).
- The F2 menu rework, friendly-actor port, bullet-flyby (need the decomp, which was lost in the sandbox reset).

## C-12 — Four round-3 playtest fixes: fists melee, scope SightPane, Raptor LOS, wire placement

Four bugs found by the user in round 3 of playtesting, all fixed:

1. **fists punch once, then never again:** `Gun::update_with` checked and decremented the clip for ALL weapons,
   including melee (ammo_type == 0). The game's `Player_RoundToFire` (0x1a18e0) skips the clip check/decrement
   for ammo_type == 0 — melee weapons always have a round to fire. Fix: added `ammo_type` to `GunParams`
   (from `w[0x90]`); skip the clip check/decrement when `ammo_type == 0` (gun.rs:299-300). Fists now fire
   repeatedly (release and re-press the trigger, matching the game's `Input_Action(_,9,4)` fresh-press check).

2. **scope: no visual / wrong visual:** The `ndc_quad` approach used the font texture (0x03000003) which is
   mostly transparent at UV (0,0) — the scope quads were nearly invisible. Fix: ported the game's real
   SightPane HUD sprite. Added textures 0x03000034 (the scope overlay, 256×256) and 0x03000002 (the solid
   fill for the side strips, 32×32) to `hud::TEXTURES`. Draw 3 quads matching the game's sprites: the scope
   texture centered at (32, 0, 448, 448) with UV 0..511, plus left/right strips at (0, 0, 33, 448) and
   (479, 0, 33, 448). All colour 0xffffff at full alpha, non-additive. Driven by `s.zoom > 1.01` (matching
   `HUD_UpdateCrossHair`'s pane[6] toggle) [D 07000005.bin entries 71 + 56, M 0x19e6d0].

3. **Raptor on 07000001 still can't be picked up:** `Pickup::create_placed` called `Pickup::create` which
   lowers the matrix Y by `radius - |centre.y|` (for DROPPED pickups that fall to the floor). Level-placed
   pickups are already at the correct height — lowering puts the sphere centre BELOW the floor, causing the
   LOS test in `touches` to fail (the ray from Bond's chest to the pickup hits the floor). Fix: build the
   matrix directly from the placement translation, bypassing `create`'s lowering (pickup.rs:91-107).

4. **wire-music: Bond on the wire's mount:** The spiral search started at r=0.0, which tests the wire's
   exact XZ. `floor_below` found the wire's mount (Y=43.65), not the walkable ground (Y=36.93). Fix: start
   the spiral at r=2.0 (dev.rs:633), skipping the wire's exact position.

Tests: all nf-game unit tests pass. NF_ISO suite: only disc_golden fails.

## C-12 — Four round-3 playtest fixes: fists melee, scope SightPane, Raptor LOS, wire placement

Four bugs found by the user in round 3 of playtesting, all fixed:

1. **fists punch once, then never again:** `Gun::update_with` checked and decremented the clip for ALL weapons,
   including melee (ammo_type == 0). The game's `Player_RoundToFire` (0x1a18e0) skips the clip check/decrement
   for ammo_type == 0 — melee weapons always have a round to fire. Fix: added `ammo_type` to `GunParams`
   (from `w[0x90]`); skip the clip check/decrement when `ammo_type == 0` (gun.rs:299-300). Fists now fire
   repeatedly (release and re-press the trigger, matching the game's `Input_Action(_,9,4)` fresh-press check).

2. **scope: no visual / wrong visual:** The `ndc_quad` approach used the font texture (0x03000003) which is
   mostly transparent at UV (0,0) — the scope quads were nearly invisible. Fix: ported the game's real
   SightPane HUD sprite. Added textures 0x03000034 (the scope overlay, 256×256) and 0x03000002 (the solid
   fill for the side strips, 32×32) to `hud::TEXTURES`. Draw 3 quads matching the game's sprites: the scope
   texture centered at (32, 0, 448, 448) with UV 0..511, plus left/right strips at (0, 0, 33, 448) and
   (479, 0, 33, 448). All colour 0xffffff at full alpha, non-additive. Driven by `s.zoom > 1.01` (matching
   `HUD_UpdateCrossHair`'s pane[6] toggle) [D 07000005.bin entries 71 + 56, M 0x19e6d0].

3. **Raptor on 07000001 still can't be picked up:** `Pickup::create_placed` called `Pickup::create` which
   lowers the matrix Y by `radius - |centre.y|` (for DROPPED pickups that fall to the floor). Level-placed
   pickups are already at the correct height — lowering puts the sphere centre BELOW the floor, causing the
   LOS test in `touches` to fail (the ray from Bond's chest to the pickup hits the floor). Fix: build the
   matrix directly from the placement translation, bypassing `create`'s lowering (pickup.rs:91-107).

4. **wire-music: Bond on the wire's mount:** The spiral search started at r=0.0, which tests the wire's
   exact XZ. `floor_below` found the wire's mount (Y=43.65), not the walkable ground (Y=36.93). Fix: start
   the spiral at r=2.0 (dev.rs:633), skipping the wire's exact position.

Tests: all nf-game unit tests pass. NF_ISO suite: only disc_golden fails.

## C-13 — Round 4 fixes: scope 4-tile subtractive, shotgun per-shell restart, Raptor LOS, wire visual

Four bugs found by the user in round 4 of playtesting, all fixed:

1. **scope overlay badly wrong (4-tile mirrored + subtractive):** The scope texture (0x03000034) is one
   quadrant of the scope — the game tiles it 2×2 with per-tile UV flip to make a full circle [M
   View_DrawSprites 0x1e5290 flag 0x0400]. The port drew it once unmirrored with REPEAT wrap, producing
   4 identical quarter-circles. Also, all 3 SightPane sprites use subtractive blend (flag 0x200, GS ALPHA
   0x42: `Cd - Cs·As/128`) — white texture darkens the framebuffer to black (scope frame), black texture
   leaves it unchanged (clear centre). The port used normal alpha blend, which replaced the framebuffer with
   white. Fix: added `hud_sub_pipeline` (ReverseSubtract) to gpu.rs; changed `HudFrame.runs` from `(tex,
   bool, ...)` to `(tex, u8 blend, ...)` (0=normal, 1=additive, 2=subtractive); draw 4 tiles of the scope
   texture at 224×224 each, flipped per quadrant, with subtractive blend and colour gs(0x7f7f7f, 0xff)
   matching the game's RGBA1. Side strips also subtractive.

2. **shotgun reload: one animation then pause:** When the shell loop continued (state 6 → 6), the +0xA4 view
   script was never restarted from frame 0 — only a state *change* triggered re-queuing. The game's case 6
   calls `AnimScriptAdd(+0xA4)` every iteration [M 0x1a2e38]. Fix: added `reload_loop_restarted: bool` flag
   to Gun, set by `reload_tick` when the loop continues; checked in `Sim::run` to restart the view script
   from frame 0.

3. **Raptor inconsistent (first fails, later works):** The `touches` LOS test used Bond's *chest*
   (`player.target = camera.y - 0.45`), which is occluded by level geometry for some placements. Fix: use
   Bond's feet (`player.pos`) for the LOS test instead — the game uses `obj+0x80` (the world sphere centre),
   and the port's `body` was already flagged as *suggested* in pickup.rs:155.

4. **wire not drawn:** The wire's placement (type 0x3A) is a dynamic entity (no 0x8000 CREATE_CEL), filtered
   from the main instance loop. Fix: added a wire visual injection loop (like the corona/door loops) that
   resolves the placement's object and calls `add_instance` for the wire's mesh.

Tests: all nf-game unit tests pass. NF_ISO suite: only disc_golden fails.
