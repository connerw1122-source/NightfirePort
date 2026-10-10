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
