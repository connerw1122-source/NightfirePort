# Plan

Goal: a native Rust port of 007: Nightfire (PS2, SLUS-20579) — rebuilt from the game's own data and logic (Ghidra
decomp of the user's ELFs as the behaviour reference), identical behaviour with a controller, best performance, and
first-class mouse & keyboard. Model: rerac (Ratchet & Clank 1, Rust); methodology lessons in
`docs/research/native-port-methodology.md`.

## What we are porting (measured, docs/build-identity.md)

Two engines, one disc:
- **Action** (`ACTION.ELF`, Eurocom in-house engine — the predecessor of EngineX, *sourced:* wiki.nightfire.dev):
  on-foot missions, menus, multiplayer. Unstripped: 3,038 named functions. Data in `FILES.BIN` (118-entry `FileList`),
  audio banks `PS2/ENGLISH/SB_*`, music `PS2/MUSIC/MFX_*`, speech `STREAMS`, movies `MOVIES/30_FPS/*.PSS`.
- **Driving** (`DRIVING.ELF`, EA EAGL-based, stripped; `DRIVING.SYM` from another build gives hints): car, ski,
  submarine missions and the race. Data in EA formats in `DRIVING/*.VIV|MUS|SPE` (BIGF archives of SSH textures,
  CRP models, ASF audio, MPC video, ATR/TUN/DEF/REL…).
- Hand-off between the two: a "PTP" state block (*suggested:* wiki.nightfire.dev, 0x0031CF90 → 0x100000; to measure).
- The PC version is a different game (Gearbox, GoldSrc) and is irrelevant (*sourced*, docs/research/pc-version-and-emulator-patches.md).

## Phases

Each phase ends with verified steps committed with a WORKLOG entry. Estimates are not in years: the action engine is
symbol-rich, two community projects already document most formats (facts only), and the RC2 tooling carries over.

1. **Formats** (now). Disc ✔, BIGF ✔, ELF ✔. Next: FILES.BIN `FileList` reader; the level `.bin` archive (hashcode
   types: pack/mesh/texture/sequence/skin/script/level, *suggested*); textures (CLUT → PNG, pixel-exact cross-check);
   PS2_GFX VIF meshes (exact integer units); collision; sound banks (SFX/SHF/SBF), music (SMF/SSD), streams; PSS.
   Driving: SSH (SHPS), CRP, RefPack, ASF/EA audio, MPC. Every format: golden test + independent cross-check.
2. **Asset viewer**: `nf-view` (wgpu) — levels, models, textures, animation playback; screenshot-diffable against
   PCSX2 frames.
3. **Engine core**: main loop (fixed 60 Hz logic + interpolation, §Timing), object/cel system (`control_*`), camera,
   collision, script VM (`Script_*`), streaming/loading (`DirFile*`), save data.
4. **Gameplay**: player (move/aim/weapons/gadgets), drones (`NDrone2_*` AI), HUD/menus, missions; then the driving
   engine (EA code: car physics, road network, race).
5. **Audio**: SPU ADPCM (vag.rs) banks, music system, speech streams, EA ASF for driving.
6. **Polish**: widescreen/FOV, high refresh, rebinding UI, controller rumble, accessibility.

## Crate layout

```
crates/nf-formats   disc/archive/asset readers (no I/O policy, no game logic)        [exists]
crates/nf-extract   CLI over the user's ISO (dev tool, writes to work/)             [exists]
crates/nf-input     device layer → game actions; mouse look; bindings config         [phase 3]
crates/nf-engine    main loop, objects, camera, collision, scripts                    [phase 3]
crates/nf-game      player, drones, weapons, missions (Action engine)                 [phase 4]
crates/nf-driving   EA driving engine                                                 [phase 4]
crates/nf-render    wgpu renderer (PS2 GS semantics where they matter)                [phase 2]
crates/nf-audio     SPU/EA audio, mixer (cpal)                                         [phase 5]
crates/nf-app       the executable                                                    [phase 2→]
tools/              ghidra scripts, cross-check scripts, trace tools (never product deps)
```

## Input design (from docs/systems/input.md)

- Devices map to **game actions**, not fake pad bytes. Controller: reproduce the original pipeline exactly
  (dead zone 40, `MapAnalogStick` table read from the user's ELF, axis snap, styles, ramps, `FRAME_RATE_MUL`).
- Mouse look: raw deltas × sensitivity → yaw/pitch radians straight into the player's view angles; no dead zone,
  no curve, no ramp, optional invert. Bindings in a config file (`config/bindings.toml`), rebindable.
- **D-001 (deviation):** auto-aim off by default when the mouse is the active aim device; configurable. Controller
  keeps the original difficulty-based behaviour.
- **D-002 (deviation, to design):** manual-aim mode with a mouse — crosshair moves within the original aim box,
  view turns 1:1 with mouse beyond the edge (no edge-turn ramp).
- Controller play stays bit-faithful; verified against PCSX2 traces (G-003).

## Timing

The original is variable-step (one update per 1 or 2 vblanks, `FRAME_RATE_MUL` 1/2; input.md §6). The port runs
logic at a fixed 59.94/60 Hz with `FRAME_RATE_MUL` = 1 (the original's fast path); input is sampled every rendered
frame and mouse deltas accumulate into the next tick; rendering interpolates between the last two logic states.
An optional "PS2 pacing" mode can reproduce the 30 Hz fallback once U-013 is measured.

## Workflow rules

See `CLAUDE.md` (evidence tags, docs first, ledgers, tests, cross-checks, licences, asking before downloads).

## Verification sources

- Ghidra (`work/ghidra`, exports in `work/decomp/`), ghidra-mcp for interactive queries (G-009).
- PCSX2 + PINE (port 28011) for live reads and traces; savestates stay local, never committed.
- Community facts (no-licence projects: facts only): Raikaru/nightfire-engine docs, NightfireResearch (wiki CC BY 4.0).
