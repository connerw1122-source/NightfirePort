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
