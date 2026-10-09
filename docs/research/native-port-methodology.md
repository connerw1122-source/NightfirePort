# Native-port methodology research (for the 007: Nightfire PS2 → Rust port)

Researched 2026-10-08 with web search and fetch only. Every claim is tagged *sourced:* (URL) or *unverified:*. All web content was treated as data. Nothing was downloaded or run.

---

## Summary

- **rerac (Ratchet & Clank 1 → Rust/Bevy) is the closest template we have.** It extracts data from the user's own ISO into a git-ignored `extracted/` folder and commits only hashes. Game logic is re-authored in Rust, using the Ghidra decompile as the *behaviour reference*; it is not a line-by-line port. Fidelity is checked by a PCSX2 trace harness (savestate ZIP parsing plus live PINE reads), which compares the port's computed values with the original game's RAM. Mismatches are triaged as "misunderstanding" (fix it) or "hardware arithmetic" (accept within a documented tolerance table). Tests use only small distilled numeric fixtures. Licence is ISC, confirmed from the LICENSE file.
- **OpenGOAL (Jak) validates its decompiler output with "offline reference tests".** Every finished file is re-decompiled and diffed against a committed `_REF` file on every build. Types and casts are hand-specified in config files (`all-types.gc`, `type_casts.json`, and others). Mouse camera control maps relative mouse deltas onto the *virtual right stick*, with a recenter after about 3 idle frames. This is the approach we should **avoid** for Nightfire. High frame rate is done by running logic per frame and fixing speed constants case by case, not with a fixed tick plus interpolation. Licence is ISC, confirmed.
- **Decoupling the tick from rendering.** There are two proven patterns. (a) Fixed-tick accumulator with interpolation of previous and current state (Gaffer "Fix Your Timestep"; GoldenEye Native says it keeps the original pace). (b) Matrix-level interpolation of tagged transforms between two logic frames (Ship of Harkinian / 2Ship / RT64). In the second, logic stays at the original rate and only visuals are interpolated. rerac currently runs a fixed 60 Hz tick with one catch-up step and no interpolation.
- **Mouse look.** The fgsfdsfgs Perfect Dark port adds the mouse delta *after* the stick's squaring curve. It feeds that into the turn-speed term, scaled by `4/lvupdate240` so the result doesn't depend on frame rate. The MouseInjector (Dolphin) driver for **007: NightFire (GameCube, GO7E69)** goes further and writes the delta straight into the player's yaw and pitch floats, in radians. Yaw is wrapped to ±π and pitch is clamped to ±1. The delta is divided by FOV, and the injector disables "lookspring". This is a direct, Nightfire-specific hint about camera state layout, though it covers the GC build only.
- **Tooling.** bethington/ghidra-mcp v7.0.0 on the main README, stable release 6.0.0, Apache-2.0. It has 209 tools with lazy-loaded tool groups and a headless server on :8089 plus a bridge on :8081. PINE (MIT) listens on TCP 127.0.0.1:28011 on Windows. It reads and writes EE memory through `vtlb_ramRead`, and can save or load a numbered savestate slot. Max message is 650000 bytes and max batch reply count is 50000. `mcp-pine` (MIT) wraps it as MCP. PCSX2's debugger has exec and memory (read/write/change) breakpoints with conditions and a "Log When Hit" option with "Continue on hit" (logpoints), plus per-subsystem EE and IOP trace logging. ghidra-emotionengine-reloaded v2.1.38 targets Ghidra 12.1.4 (Apache-2.0).
- **Rust stack.** winit `DeviceEvent::MouseMotion` gives "raw, unfiltered" deltas, and on Windows winit registers Raw Input devices. SDL3 relative mode is unscaled by default (no OS acceleration). gilrs 0.11.2 defaults to Windows.Gaming.Input on Windows, which needs a focused window, or XInput via a feature flag. Bevy's `FixedUpdate` defaults to **64 Hz** and must be set explicitly to the game's rate. cpal 0.18.2 uses a callback stream. wgpu uses DX12 or Vulkan on Windows.

---

## 1. rerac: native port of Ratchet & Clank (2002)

### What it is
- An unofficial native PC port written in Rust on the Bevy engine. "The game's systems are ported and run natively, with no emulator in between." It is in early development and not yet completable. *sourced:* https://github.com/re-rac/rerac
- No game assets are included. The user extracts them from their own disc with `cargo xtask regen-data --iso "<path>.iso"`, which also checks the disc is supported and verifies the output. *sourced:* https://github.com/re-rac/rerac
- Only macOS on Apple Silicon has been tested so far. *sourced:* https://github.com/re-rac/rerac

### Crate layout
- Workspace members are `crates/rc-formats`, `rc-engine`, `rc-game`, `rc-extract`, `rc-data`, `rc-video`, `tools/trace`, `tools/repo-checks`, and `tools/xtask`. *sourced:* https://raw.githubusercontent.com/re-rac/rerac/main/Cargo.toml
- The README describes `crates/` as the game (file formats, game logic, engine, extractor), `tools/` as dev tools that never ship, and `docs/` as format specs, plans, and workflows. *sourced:* https://github.com/re-rac/rerac
- `docs/` contains `assets`, `formats`, `plan`, `test-requests`, and `workflows`. `docs/plan` has one document per subsystem and per planet, for example `render_pipeline.md`, `audio.md`, `trace_harness.md`, `hardware_fidelity_layers.md`, `decisions.md`, `decomp_workflow.md`, and `reference_opengoal.md`. *sourced:* https://github.com/re-rac/rerac/tree/main/docs/plan

### Key architecture decisions
- "Game logic is re-authored in Rust with the decompiled C++ as the behaviour reference, not ported line by line." *sourced:* https://raw.githubusercontent.com/re-rac/rerac/main/docs/plan/decisions.md
- The renderer uses wgpu through Bevy, with custom render pipelines and "WGSL shaders derived from the VU1 programs". Backends are Metal, Vulkan, and DX12. *sourced:* https://raw.githubusercontent.com/re-rac/rerac/main/docs/plan/decisions.md
- No copyrighted data enters the repo. Derived data goes to a git-ignored `extracted/` folder, and only hashes are committed. *sourced:* https://raw.githubusercontent.com/re-rac/rerac/main/docs/plan/decisions.md
- Wrench (GPL-3) is "a reference for format knowledge, not a code source". OpenGOAL is an architectural reference, not a code source. *sourced:* https://raw.githubusercontent.com/re-rac/rerac/main/docs/plan/decisions.md
- From OpenGOAL, rerac adopts build identification by boot-ELF serial and hash, one renderer class per VU1 microprogram, and asserting loudly on unsupported GS blend modes rather than approximating. It rejects OpenGOAL's `target_fps` and simulates only at the disc's native rate, though rendering may run faster. This doc predates the switch from C++ to Rust/Bevy on 2026-09-26. *sourced:* https://raw.githubusercontent.com/re-rac/rerac/main/docs/plan/reference_opengoal.md

### Timing and input
- "A fixed 60 Hz tick with one catch-up step, run as: pad → mobys → hero → camera → render." Extra fixed steps are dropped, so "the game slows down below 30 fps like the PS2." *sourced:* https://raw.githubusercontent.com/re-rac/rerac/main/docs/plan/player_controller.md
- No sub-tick render interpolation is described. *sourced:* same URL. Absence was checked across both fetched parts of the doc.
- Gamepads come through Bevy's `bevy_gilrs`. Keyboard input maps WASD to full stick deflection (bytes 0/255), with Shift for half. Axes are handled per axis, not radially, using the game's own dead zone. *sourced:* https://raw.githubusercontent.com/re-rac/rerac/main/docs/plan/player_controller.md
- Mouse look is "mouse motion while the right button is held (12 px per frame = full)" on the right stick. In other words, rerac also routes the mouse **through the stick**. *sourced:* https://raw.githubusercontent.com/re-rac/rerac/main/docs/plan/player_controller.md

### Audio
- The audio is "A software 989snd + SPU2 at 48 kHz". Voices are summed, and reverb is a native effect. "Exact SPU mixing is not achievable," so traces check which sounds play and their volume, pan, and pitch. *sourced:* https://raw.githubusercontent.com/re-rac/rerac/main/docs/plan/audio.md
- SPU ADPCM decoding uses the PCSX2 rounding form. The OpenGOAL form differs by up to 2 LSB. *sourced:* same URL.
- Output goes through Bevy 0.19's bevy_audio, rodio 0.22, and cpal. A `rodio::Source` wraps the mixer and is fed from a ring buffer that the game tick fills. *sourced:* same URL.
- Music is VAG streams with a Start/Loop pair. A unit test proves the hand-over is sample-identical. *sourced:* same URL.

### Fidelity verification (the most important part)
- The **trace harness (`rc-trace`)** is the "ground truth for every computation we port." It has two capture paths:
  - PCSX2 `.p2s` savestates. These are ZIP files containing `eeMemory.bin`, `vu1Memory.bin`, `Scratchpad.bin`, and `GS.bin`, with stored, Deflate, or Zstd entries.
  - Live PINE reads, using batched `MsgRead64` (50000 per message, 84 round trips for 32 MiB).
  *sourced:* https://raw.githubusercontent.com/re-rac/rerac/main/docs/plan/trace_harness.md
- RAM structures are located without hard-coded bases. A **locator** searches RAM for disc-known, load-invariant bytes and cross-checks the pointers. *sourced:* same URL.
- The comparison loads disc inputs, runs the port's function, and diffs the result against RAM. Output includes a per-class histogram of `ours - RAM` (−8…+8 plus a "far" bucket) and a CSV of mismatches. Exit codes are 0 (equal), 2 (mismatch), and 1 (error). *sourced:* same URL.
- **Triage policy:** "Bit-exactness is a diagnostic tool, not the goal." Mismatches are kind 1 (misunderstanding: fix natively) or kind 2 (pure hardware arithmetic, ±1 or 1-ULP: accept with a documented tolerance). "If a mismatch cannot be classified, treat it as kind 1." *sourced:* same URL.
- Results reported: tfrag lighting 51331/51331 vertices bit-exact, and shrubs 28992/28992. *sourced:* same URL.
- The tolerance table records each quantity, its bound, the reason, and the measurement. Examples are 2 ULP on cos(yaw) matrix entries, 3e-4 units on the follow camera, and ±1 per channel on tie palette colours. Phase 1 adds no new hardware-emulation layers. Visible PS2 arithmetic effects are reproduced "by result, not by mechanism". *sourced:* https://raw.githubusercontent.com/re-rac/rerac/main/docs/plan/hardware_fidelity_layers.md
- PCSX2 workflow:
  - PINE on slot 28011.
  - `record --seconds 90` records the hero live over PINE to a TSV.
  - `replay-hero` replays the recording through the port and diffs the result.
  - `distill-spawn` writes numbers-only fixtures to `tools/trace/tests/fixtures/`, and "A test never reads a savestate, an EE dump, or a recording."
  *sourced:* https://raw.githubusercontent.com/re-rac/rerac/main/docs/workflows/pcsx2.md
- Testing tiers:
  - Per-job unit tests take about 5 s. `test-full` takes 139 s.
  - Shared-code guards include a NO_IDLE hero **digest**, which is "a change detector, not a correctness check". Its baseline is written only by `cargo xtask digest-baseline`.
  - A guard test forbids personal paths in tests.
  *sourced:* https://raw.githubusercontent.com/re-rac/rerac/main/docs/workflows/testing.md
- Golden tests compare loader output for all 19 levels with a `loader_snapshots.tsv` file (count, bytes, SHA-1 per section). The snapshots were generated while the output was byte-identical to a retired C++ reference extractor. Determinism is checked with tests that run twice. *sourced:* https://raw.githubusercontent.com/re-rac/rerac/main/docs/workflows/testing.md (offset 100000)
- The **test-request process**: implementation agents write one file per job (Summary, Behaviours to verify, Not ported, In-game QA spots, Results). A separate test-expert agent writes and runs the tests. Status moves open → in-test → passed/failed → closed. *sourced:* https://github.com/re-rac/rerac/tree/main/docs/test-requests
- Ghidra workflow:
  - Boot ELF and level overlays are imported into **separate programs**, because their addresses overlap.
  - Names live in CSVs (`tools/ghidra/names/*.csv`), keyed by program, address, name, and kind, with a confidence of verified, inferred, or suggested. They can be re-applied to a fresh project, and every apply is logged so it can be undone.
  - Struct headers in the repo are the source of truth and are pushed *into* Ghidra.
  - Decompiled C is exported to `work/decomp/` for grepping.
  - It uses the GhidraMCP HTTP API on 127.0.0.1:8089 and the ghidra-emotionengine-reloaded processor module.
  *sourced:* https://raw.githubusercontent.com/re-rac/rerac/main/docs/plan/decomp_workflow.md and https://raw.githubusercontent.com/re-rac/rerac/main/docs/workflows/ghidra.md
- Licence: ISC, "Copyright (c) 2026 ReRAC contributors". *sourced:* https://raw.githubusercontent.com/re-rac/rerac/main/LICENSE

---

## 2. OpenGOAL (open-goal/jak-project)

- **Goal:** a "native application" on x86-64. "It shouldn't be emulated, interpreted, or transpiled." The project decompiles the original GOAL code and recompiles it with its own compiler. *sourced:* https://github.com/open-goal/jak-project
- **Type system and casts:**
  - `all-types.gc` defines every symbol, function, and type per game.
  - `type_casts.json` pins casts at IR operation indices, with ranges that are [inclusive, exclusive).
  - Other config files are `stack_structures.json`, `label_types.json`, `anonymous_function_types.jsonc`, `var_names.jsonc` (renames survive re-decompilation), and `hacks.jsonc` (skips inline-asm functions).
  *sourced:* https://opengoal.dev/docs/developing/decompiling/getting_started
- **Reference (offline) tests:** you copy the `*_disasm.gc` output to `test/decompiler/reference/…_REF.gc`, then run `task offline-tests`. Every build re-decompiles and compiles all finished files as regression tests. A file is "done" when it decompiles cleanly, passes the reference test, and the game builds. *sourced:* https://opengoal.dev/docs/developing/decompiling/getting_started
- **Runtime:** the C kernel "should be as close as possible to the game, and all differences should be noted with a comment". The renderer is OpenGL in `game/graphics`. *sourced:* https://raw.githubusercontent.com/open-goal/jak-project/master/docs/project-overview.md
- **Validation against the original:** no PCSX2 trace harness is documented in the pages fetched. *unverified:* the project may compare against PCSX2 informally. rerac notes that OpenGOAL "uses host IEEE floats everywhere". *sourced:* https://raw.githubusercontent.com/re-rac/rerac/main/docs/plan/reference_opengoal.md
- **Frame rate:**
  - Jak 2 got an experimental frame-rate menu, because it "has much better support for variable frame rates out of the box". *sourced:* https://opengoal.dev/blog/progress-report-sept-2023
  - High-FPS support is then a long tail of per-object speed fixes, such as camera turning speed, projectile speeds, texscroll, and `rigid-body` physics "to not depend on FPS". *sourced:* https://opengoal.dev/blog/progress-report-q4-2025
  - The input buffer was made to scale with frame rate. *sourced:* https://opengoal.dev/blog/progress-report-feb-2024
  - **Lesson:** OpenGOAL's variable-rate logic creates endless per-object bugs. A fixed tick with interpolation avoids that.
- **PC input:**
  - They switched from GLFW to SDL (now SDL3), with rebinding in the pause menu. *sourced:* https://opengoal.dev/blog/progress-report-june-2023
  - Keyboard is disabled by default because of Steam Input double-mapping. *sourced:* https://opengoal.dev/blog/progress-report-feb-2024
  - **Mouse camera:** SDL relative mode. On each motion event they compute `clamp(127 + xrel*sens, 0, 255)` and write it into analog channels 2 and 3 (the right stick). The stick recenters to 127 after more than 3 frames with no motion. Holding both mouse buttons moves forward. *sourced:* https://raw.githubusercontent.com/open-goal/jak-project/master/game/system/hid/devices/mouse.cpp
  - In other words, the mouse goes through the stick, is clamped to stick range, and is quantised to 8 bits. That is acceptable for a third-person platformer but poor for an FPS.
- Licence: ISC, "Copyright (c) 2020-2026 OpenGOAL Team". *sourced:* https://raw.githubusercontent.com/open-goal/jak-project/master/LICENSE

---

## 3. Other ports and decomps: techniques

### Fixed logic tick and render interpolation
- **Canonical loop** (Gaffer on Games):
  1. Accumulate real frame time.
  2. Step the simulation by a fixed `dt`, saving the previous state before each step.
  3. Compute `alpha = accumulator/dt` and blend the previous and current states for rendering (slerp for rotations).
  4. Clamp frame time (0.25 s) to avoid the "spiral of death".
  *sourced:* https://gafferongames.com/post/fix_your_timestep/
- **Ship of Harkinian / 2Ship (decomp-based Zelda ports):**
  - Logic stays at 20 FPS, and "this is all purely visual". *sourced:* https://github.com/HarbourMasters/2ship2harkinian/releases/tag/1.0.0
  - `frame_interpolation.cpp` builds a tree of transform "instructions", with matrices at the leaves. Each node carries enough identity to be matched in the next frame. The final matrices are interpolated by a factor in [0,1]. A known artifact is the "paper effect" near 180° rotations. The mitigation is to interpolate actor rotations in model space. *sourced:* https://code.moparisthebest.com/mirror/Shipwright/src/branch/develop/soh/soh/frame_interpolation.cpp (mirror of HarbourMasters/Shipwright)
- **RT64 / N64Recomp (Zelda64Recomp):**
  - Draw calls are matched between frames by render params, textures, position, orientation, and velocity. Transforms are decomposed before interpolation to avoid artifacts. One renderer stays synced at the native rate, and another replays at the high rate. *sourced:* https://github.com/rt64/rt64
  - "Changing framerate has no effect on gameplay." *sourced:* https://github.com/Zelda64Recomp/Zelda64Recomp
  - For our native port this is easier: we own the scene graph, so we can tag transforms by entity ID directly.
- **GoldenEye Native (decomp-based N64 port):** rendering is separated from the game's internal timing, so weapons, animation, and AI "keep their original pace". It has built-in mouse aiming. The repository was reportedly taken down after mass reports. *sourced:* https://www.generationamiga.com/2026/08/29/goldeneye-007-n64-native-port-released-for-windows-linux-and-macos/
- **Perfect Dark port (fgsfdsfgs):**
  - "Uncap Tickrate" gives up to 240 FPS, but the game "will have issues running faster than ~165 FPS". *sourced:* https://github.com/fgsfdsfgs/perfect_dark
  - This is a variable-tick approach (`lvupdate240`), not interpolation. *sourced:* https://raw.githubusercontent.com/fgsfdsfgs/perfect_dark/port/src/game/bondmove.c

### Mouse look without the stick acceleration curve
- **Perfect Dark port (fgsfdsfgs), `bmoveProcessInput()`:**
  - The mouse delta is read with `inputMouseGetScaledDelta`.
  - It is scaled by `mlookscale = 4/lvupdate240`, which normalises it per tick so it doesn't depend on frame rate.
  - It is **added after** the stick value has been normalised and squared: `fVar25 += movedata.freelookdx * mlookscale`. The sum then becomes `speedthetacontrol` and `speedverta`.
  - "Classic" aim mode instead moves the crosshair (`swivelpos`, clamped to ±1).
  - Mouse movement also enables natural pitch and turn when it is non-zero.
  - The stick curve is bypassed, but the result still runs through the game's speed integrator.
  *sourced:* https://raw.githubusercontent.com/fgsfdsfgs/perfect_dark/port/src/game/bondmove.c
- **MouseInjector for Dolphin, `games/nf.c` (007: NightFire, GameCube NTSC, ID `GO7E69`):**
  - It polls at 1000 Hz and reads a player pointer. From fixed offsets it gets camera **yaw and pitch floats (radians)**, FOV, health, crosshair sway, and a "lookspring" flag.
  - Yaw is incremented by delta × sensitivity × (2π / 360-ish scale) / FOV and wrapped to [−π, π].
  - Pitch is decremented by a similar term and clamped to [−1, 1].
  - Writes are gated on: values in range, FOV ≥ 1, health > 0, not paused.
  - It disables lookspring at spawn.
  - It falls back to a "sentry interface" pointer for vehicle and turret sections (helicopter, jet ski), which use different ranges and signs.
  - The compatibility table rates NightFire mouse support "Poor", with "Vehicle mode is semi-functional".
  *sourced:* https://raw.githubusercontent.com/dj0wns/MouseInjectorDolphin/master/games/nf.c and https://github.com/dj0wns/MouseInjectorDolphin
  - *unverified:* that the PS2 build (also by Eurocom) uses the same camera representation. This is a strong hypothesis to confirm in Ghidra and with PINE. Don't reuse the GC addresses.
- **KAMI** (mouse injector for PCSX2/RPCS3 over IPC on port 28012) uses a sensitivity "number of radians of change per pixel moved". Its compatibility list includes no Bond title. *sourced:* https://github.com/dio-gh/KAMI and https://isjuhn.github.io/index.html
- **Zelda64Recomp** offers mouse aiming mainly so that gyro works on Steam Deck. *sourced:* https://github.com/Zelda64Recomp/Zelda64Recomp
- **THPS PARTYMOD** replaced the input system with SDL2. *sourced:* https://www.speedrun.com/thps4/resources/19tbo

### PS2 decomp and port landscape
- **Sly 1 decomp:**
  - Matching decompilation split with splat, using the ProDG toolchain and objdiff, with progress shown on decomp.dev.
  - Licensed CC0-1.0, with no game assets or code.
  - It "was the first PS2 decompilation project that targeted the PS2 and utilized function matching".
  *sourced:* https://github.com/TheOnlyZac/sly1
- **Project Cane (Sly Cooper native PC port):** a work in progress built on RE and decompilation that uses data extracted from the disc. *sourced:* https://www.generationamiga.com/2026/08/31/sly-cooper-pc-port-is-taking-shape-as-project-cane-shows-new-gameplay/
- **RC1 decomp (Codeberg):** matching, splat, SCE/ProDG EE GCC, and it reportedly uses Sly1 as a reference. *sourced:* https://codeberg.org/bordplate/RC1 (via search summary, not fetched directly. Treat as *unverified* detail.)
- **PS2Recomp:**
  - A static recompiler that turns R5900 into C++, with an "Experimental" label and GPL-3.0 licence.
  - "Hardware emulation is partial and many paths are stubbed." "Performance is very bad for VU and GS."
  - Not our approach, but its Ghidra function-export workflow is noteworthy.
  *sourced:* https://github.com/ran-j/PS2Recomp
- **BFBB:** the bfbbpc decomp-based Windows port is based on the GameCube version. *sourced:* https://identichosting.duckdns.org/seilc/bfbbpc (mirror, *unverified* currency)
- No public decomp or native port was found for Sonic Heroes, Twinsanity, SotC, Spyro AHT, or Midnight Club in the searches run. *unverified:* these may exist under other names.

---

## 4. MCP tooling for game RE

### bethington/ghidra-mcp
- **Tools and transports:**
  - "209 MCP tools". The GUI plugin serves 205 endpoints and the headless server 190.
  - Categories cover listing, functions, data types, xrefs, comments, emulation, Ghidra Server version control, and TraceRmi debugger.
  - It uses **lazy tool groups** (`search_tools`, `list_tool_groups`, `load_tool_group`, …) because 209 tools exceed some providers' limits.
  - Transports are stdio (recommended), streamable-http, and SSE (deprecated).
  *sourced:* https://github.com/bethington/ghidra-mcp
- **Headless:** a standalone `GhidraMCPHeadlessServer`. Docker Compose runs the headless server on **8089** and the bridge on **8081** (`/mcp`). `GHIDRA_MCP_AUTH_TOKEN` is required when binding beyond loopback, and scripts are off unless `GHIDRA_MCP_ALLOW_SCRIPTS` is set. Multi-program work goes through `switch_program`/`open_program`, and instance discovery through `list_instances`/`connect_instance`. *sourced:* https://github.com/bethington/ghidra-mcp
- **Version:**
  - The README states 7.0.0, targeting Ghidra 12.1.4, with Java 21 and Python 3.10+. *sourced:* https://github.com/bethington/ghidra-mcp
  - The releases page shows **6.0.0 as the latest stable** (Ghidra 12.1.2) and **7.0.0-rc.1** as a pre-release. *sourced:* https://github.com/bethington/ghidra-mcp/releases
- Licence: Apache-2.0. *sourced:* https://github.com/bethington/ghidra-mcp
- *unverified:* the `mcp__ghidra-mcp__*` tool names visible in this Claude session (`list_tool_groups`, `load_tool_group`, `connect_instance`, …) match this server's lazy-loading interface.

### LaurieWired GhidraMCP
- A Ghidra plugin with an HTTP server (default port 8080) plus a Python bridge, `bridge_mcp_ghidra.py`. It can decompile, rename, and list methods, classes, imports, and exports. No headless mode is documented. Licence is Apache-2.0. *sourced:* https://github.com/LaurieWired/GhidraMCP
- Latest release is 1.4, which added Ghidra 11.3.2 support. *sourced:* https://github.com/LaurieWired/GhidraMCP/releases

### ghidra-emotionengine-reloaded
- **Features:**
  - EE-specific instructions (MMI, VU0 macro mode).
  - A STABS analyzer for `.mdebug` ELFs.
  - PCSX2 savestate import.
  - An R5900 constant-reference analyzer for globals.
  *sourced:* https://github.com/chaoticgd/ghidra-emotionengine-reloaded
- **Latest release v2.1.38** (04 Oct 2026) targets Ghidra **12.1.4**. v2.1.37 targets 12.1.3 and v2.1.36 targets 12.1.2, so each release tracks a single Ghidra version. *sourced:* https://github.com/chaoticgd/ghidra-emotionengine-reloaded/releases
- Licence: Apache-2.0. *sourced:* https://github.com/chaoticgd/ghidra-emotionengine-reloaded
- **Compatibility:** pair Ghidra 12.1.4 with EE-reloaded v2.1.38 and ghidra-mcp 7.0.0 (README target 12.1.4). ghidra-mcp 6.0.0 stable targets 12.1.2, which would need EE-reloaded v2.1.36. *(derived from the sourced facts above)*

### PCSX2 PINE protocol
- **Default slot is 28011** (`PINE_DEFAULT_SLOT`). *sourced:* https://raw.githubusercontent.com/PCSX2/pcsx2/master/pcsx2/PINE.h
- **Transport:**
  - On Windows, TCP bound to `INADDR_LOOPBACK` on port = slot.
  - On Unix, `pcsx2.sock` (plus `.<slot>` when not the default) under `XDG_RUNTIME_DIR` or `TMPDIR`.
  *sourced:* https://raw.githubusercontent.com/PCSX2/pcsx2/master/pcsx2/PINE.cpp
- **Wire format:** a u32 LE size, then a u8 opcode, then the address. Opcodes are:

  | Opcode | Command |
  |---|---|
  | 0–3 | Read8/16/32/64 |
  | 4–7 | Write8/16/32/64 |
  | 8 | Version |
  | 9 | SaveState |
  | 0xA | LoadState |
  | 0xB | Title |
  | 0xC | ID |
  | 0xD | UUID |
  | 0xE | GameVersion |
  | 0xF | Status |

  Results are IPC_OK = 0 and IPC_FAIL = 0xFF. Status values are Running = 0, Paused = 1, Shutdown = 2. Limits are MAX_IPC_SIZE 650000, MAX_IPC_RETURN_SIZE 450000, and MAX_BATCH_REPLY_COUNT 50000. Batching is "_very_ fast". *sourced:* https://raw.githubusercontent.com/GovanifY/pine/master/src/pine.h
- **Limitations:**
  - Reads and writes go through `vtlb_ramRead/Write` on a 32-bit **EE address** only. There is no IOP, VU, GS, or SPU2 memory access and no register access.
  - PINE itself does no range checks.
  - Every memory, state, and info command requires a valid VM.
  - SaveState and LoadState take a **numbered slot byte**, not a file path.
  - There is no pause, step, or breakpoint control.
  *sourced:* https://raw.githubusercontent.com/PCSX2/pcsx2/master/pcsx2/PINE.cpp
  - rerac works around the missing VU1, scratchpad, and GS access by parsing `.p2s` savestate ZIPs. *sourced:* https://raw.githubusercontent.com/re-rac/rerac/main/docs/plan/trace_harness.md
- PINE is MIT-licensed, and PCSX2 is the reference server. *sourced:* https://github.com/GovanifY/pine

### PINE / PCSX2 MCP servers
- **mcp-pine** (dmang-dev):
  - Tools are `pine_ping`, `pine_get_info`, `pine_get_status`, `pine_read8..64`, `pine_read_range` (≤4096 bytes), `pine_write8..64`, and `pine_save_state`/`pine_load_state` (slots 0–255).
  - Configured with env vars `PINE_TARGET`, `PINE_SLOT` (28011 by default for PCSX2), `PINE_HOST`, and `PINE_PIPELINE_BATCH`.
  - Needs Node 22+. MIT-licensed. Tested against PCSX2 v2.6.3.
  *sourced:* https://github.com/dmang-dev/mcp-pine
  - *unverified:* this matches the `mcp__pcsx2-pine__*` tools in this session.
- **PCSX2_MCP** (snowyegret23):
  - Uses PCSX2's EE and IOP **GDB RSP** endpoints (127.0.0.1:10501 and 10502) rather than PINE.
  - Provides breakpoints and watchpoints (exec, read, write; conditional), frame advance, JSONL background traces, memory snapshot diffing, savestates, and a rewind buffer.
  - Extended tools need a PCSX2 build implementing `qPcsx2` commands. MIT-licensed.
  *sourced:* https://glama.ai/mcp/servers/snowyegret23/PCSX2_MCP and https://github.com/snowyegret23/PCSX2_MCP
  - *unverified:* whether mainline PCSX2 ships these GDB servers. No PR or docs were found.

### PCSX2 debugger features useful for traces
- Open it with Debug → Open Debugger, which needs advanced settings enabled. *sourced:* https://pcsx2.net/docs/advanced/debugger
- Features *(sourced: same URL)*:
  - R5900 and R3000 layouts.
  - Register views (GPR, CP0, FPR, VU0f/i, GS).
  - Memory search with filter-refine.
  - Function stubbing.
  - Symbol import (`.symtab`, `.mdebug`, `.sndata`, and a text `.sym` format; no DWARF).
  - Function hashing that greys out symbols for overwritten overlays.
  - An expression language with `[addr,n]` memory reads.
- **Breakpoint dialog:**
  - Types are Execute or Memory, with Read, Write, and Change for memory.
  - A Condition field.
  - "**Log When Hit**", with a Log Format field and "**Continue on hit**". These are logpoints, so you get printf-style traces without stopping.
  *sourced:* https://raw.githubusercontent.com/PCSX2/pcsx2/master/pcsx2-qt/Debugger/Breakpoints/BreakpointDialog.ui
- **Trace logging** has an "Enable" switch and categories:
  - EE: COP0, COP1, COP2 (VU0 macro), R5900, Cache, Memory, HW Regs, Unknown MMIO, DMA Registers, DMA Control, MSKPATH3, SPR/MFIFO, IPU, Counters, VIFCodes, GIFTags, VIF, GIF, BIOS, SIF.
  - IOP: COP2, R3000A, Memcards, Pad, DMA, HW Regs, Counters, CDVD, MDEC, BIOS.
  *sourced:* https://raw.githubusercontent.com/PCSX2/pcsx2/master/pcsx2-qt/Settings/DebugLoggingSettingsTab.ui

---

## 5. Rust ecosystem notes (brief)

- **winit 0.30.13:**
  - `DeviceEvent::MouseMotion { delta }` is "raw, unfiltered physical motion", in unspecified units. *sourced:* https://docs.rs/winit/latest/winit/event/enum.DeviceEvent.html
  - On Windows, winit registers mice and keyboards through `RegisterRawInputDevices` (`HID_USAGE_GENERIC_MOUSE`, `RIDEV_INPUTSINK` depending on the `DeviceEvents` filter). *sourced:* https://github.com/rust-windowing/winit/blob/v0.30.13/src/platform_impl/windows/raw_input.rs
  - The filter options are `DeviceEvents::Always`, `WhenFocused`, and `Never`. *sourced:* https://docs.rs/winit/latest/winit/event_loop/enum.DeviceEvents.html
  - `CursorGrabMode::Locked` is unsupported on X11, and `Confined` is unsupported on macOS. You must hide the cursor yourself. *sourced:* https://docs.rs/winit/latest/winit/window/enum.CursorGrabMode.html
  - *unverified:* on Windows, use Confined plus hiding the cursor, and read `DeviceEvent::MouseMotion` for look.
- **SDL3:**
  - `SDL_SetWindowRelativeMouseMode` (3.2.0+) hides the cursor and reports continuous relative motion at window edges. *sourced:* https://wiki.libsdl.org/SDL3/SDL_SetWindowRelativeMouseMode
  - `SDL_HINT_MOUSE_RELATIVE_SYSTEM_SCALE` defaults to "0", meaning relative motion is unscaled and the OS acceleration curve is not applied. *sourced:* https://wiki.libsdl.org/SDL3/SDL_HINT_MOUSE_RELATIVE_SYSTEM_SCALE
  - The Rust bindings are the `sdl3` crate (forked from rust-sdl2): "Expect some bugs and missing features." *sourced:* https://github.com/vhspace/sdl3-rs
- **gilrs 0.11.2:**
  - On Windows it defaults to Windows.Gaming.Input, which "requires an in focus window". The `xinput` feature switches to XInput.
  - Uses SDL_GameControllerDB mappings and the `SDL_GAMECONTROLLERCONFIG` env var.
  - Supports force feedback on Windows and Linux.
  *sourced:* https://docs.rs/gilrs/latest/gilrs/
- **Bevy 0.20:** `FixedUpdate` defaults to **64 Hz** (15625 µs), chosen to avoid 60 Hz stutter patterns. It runs zero or more times per frame from accumulated overstep. *sourced:* https://docs.rs/bevy/latest/bevy/time/struct.Fixed.html
  - We must set 60 Hz (NTSC) or 50 Hz (PAL), or Nightfire's true rate, explicitly. *unverified:* Nightfire PS2's logic rate and whether it uses variable dt. Confirm in Ghidra.
- **wgpu (v30):** first-class DX12 and Vulkan on Windows, Metal on Apple, GL as a downlevel backend. MIT/Apache-2.0. *sourced:* https://github.com/gfx-rs/wgpu
- **cpal 0.18.2:** `build_output_stream` with a data callback on a high-priority thread. Streams start paused, so call `play()`. *sourced:* https://docs.rs/cpal/latest/cpal/
  - rerac feeds a cpal/rodio ring buffer from the game tick. *sourced:* https://raw.githubusercontent.com/re-rac/rerac/main/docs/plan/audio.md

---

## Licences

| Project | URL | Licence | Reuse policy |
|---|---|---|---|
| rerac | https://github.com/re-rac/rerac | ISC (verified, LICENSE file) | permissive: reuse with notice |
| OpenGOAL jak-project | https://github.com/open-goal/jak-project | ISC (verified, LICENSE file) | permissive: reuse with notice |
| Perfect Dark port (fgsfdsfgs) | https://github.com/fgsfdsfgs/perfect_dark | MIT (root LICENSE, © 2022 Ryan Dwyer; scope over decomp code unclear) | permissive: reuse with notice; treat game-derived code as facts only |
| MouseInjectorDolphin (nf.c) | https://github.com/dj0wns/MouseInjectorDolphin | GPL-2.0 (file header: GPLv2+) | GPL: reference only |
| KAMI | https://github.com/dio-gh/KAMI | MIT | permissive: reuse with notice |
| PS2Recomp | https://github.com/ran-j/PS2Recomp | GPL-3.0 | GPL: reference only |
| Zelda64Recomp | https://github.com/Zelda64Recomp/Zelda64Recomp | GPL-3.0 | GPL: reference only |
| RT64 | https://github.com/rt64/rt64 | MIT (sidebar) | permissive: reuse with notice |
| Ship of Harkinian (frame_interpolation.cpp) | https://code.moparisthebest.com/mirror/Shipwright/src/branch/develop/soh/soh/frame_interpolation.cpp | not verified | none: facts only |
| Sly 1 decomp | https://github.com/TheOnlyZac/sly1 | CC0-1.0 | permissive: reuse with notice (attribution courtesy) |
| Wrench (R&C tools, cited by rerac) | https://raw.githubusercontent.com/re-rac/rerac/main/docs/plan/decisions.md | GPL-3 (per rerac) | GPL: reference only |
| bethington/ghidra-mcp | https://github.com/bethington/ghidra-mcp | Apache-2.0 | permissive: reuse with notice |
| LaurieWired GhidraMCP | https://github.com/LaurieWired/GhidraMCP | Apache-2.0 | permissive: reuse with notice |
| ghidra-emotionengine-reloaded | https://github.com/chaoticgd/ghidra-emotionengine-reloaded | Apache-2.0 | permissive: reuse with notice |
| PINE | https://github.com/GovanifY/pine | MIT | permissive: reuse with notice |
| PCSX2 | https://github.com/PCSX2/pcsx2 | *unverified:* GPL-3.0+ (not fetched) | GPL: reference only |
| mcp-pine | https://github.com/dmang-dev/mcp-pine | MIT | permissive: reuse with notice |
| PCSX2_MCP (snowyegret23) | https://github.com/snowyegret23/PCSX2_MCP | MIT | permissive: reuse with notice |
| GoldenEye Native | https://github.com/SegfaultEvan/goldeneye-native | not stated; repo reportedly taken down | none: facts only |
| winit / gilrs / wgpu / cpal / bevy / sdl3-rs | (crates above) | *unverified:* Apache-2.0/MIT (wgpu verified MIT/Apache-2.0) | permissive: reuse with notice |

---

## Lessons to adopt

1. **Keep assets out of the repo** (rerac model). Use an extractor from the user's own ISO into a git-ignored `extracted/` folder and commit only hashes. Identify the disc build by boot-ELF serial plus hash, and print a database row for unknown builds.
2. **Behaviour reference, not transliteration.** Re-author logic in idiomatic Rust, citing the Ghidra program and address for each function. Keep names in re-applicable CSVs with confidence levels (verified, inferred, suggested). Treat repo struct headers as the source of truth and push them into Ghidra.
3. **Build a trace harness before porting gameplay.** Parse PCSX2 `.p2s` ZIPs (EE, VU1, scratchpad, GS) and use batched PINE `Read64` for live reads. Locate structures by searching for disc-known invariant bytes, never by hard-coded bases. Report histograms of `ours − RAM` and use exit codes 0, 2, and 1.
4. **Triage policy.** Treat bit-exactness as a diagnostic. Classify every mismatch as a misunderstanding (fix it) or a hardware-arithmetic difference (accept with a tolerance table that records the measured evidence). Unclassified mismatches default to misunderstanding.
5. **Tests use distilled numeric fixtures only.** Never put savestates, dumps, or recordings in tests or commits. Add a guard test against personal paths. Use golden loader snapshots (count, bytes, SHA-1), a determinism-twice test, and a behaviour digest as a change detector whose baseline is human-approved.
6. **Record and replay.** Record player state live over PINE, replay the same pad stream through a headless port sim, and report the first divergence.
7. **Fixed logic tick at the original rate plus render interpolation.** Set Bevy `FixedUpdate` explicitly (the default is 64 Hz, which is wrong for us). Store previous and current transforms per entity (or tag matrices by entity ID, as SoH does) and interpolate with `alpha = overstep/dt`. Clamp catch-up steps. Avoid OpenGOAL's variable-rate approach, which needed endless per-object speed fixes.
8. **Mouse look bypasses the stick entirely.** Read raw deltas (winit `DeviceEvent::MouseMotion` or SDL3 relative mode, which is unscaled by default). Convert them to radians per count. Accumulate per tick, scale by FOV (as the NightFire injector does), and apply directly to the player's yaw and pitch state, wrapping yaw and clamping pitch to the game's own limits. Disable lookspring and auto-centre while the mouse is active.
   - Do **not** encode the mouse into stick bytes. OpenGOAL and rerac do, which clamps and quantises the input.
   - If the game's turn code must be reused, inject the delta *after* the stick response curve, as the Perfect Dark port does.
   - Handle vehicle and turret ("sentry") cameras as separate cases.
9. **Use the NightFire GC injector as a map, not as code.** It tells us the Eurocom camera stores yaw and pitch as radians, alongside FOV, health, and a lookspring flag. Find the PS2 equivalents with PCSX2 memory search and "Log When Hit / Continue on hit" logpoints, then confirm with PINE reads. It is GPL, so use the facts only.
10. **Tool pinning.** Pin Ghidra 12.1.4, EE-reloaded v2.1.38, and ghidra-mcp 7.0.0, and record the exact versions in the docs. Keep boot ELF and overlays as separate Ghidra programs. Export decompiled C to a grep-able work folder.
11. **Test-request process.** One markdown file per job (behaviours to verify, not ported, QA spots, results) works well for agent-driven development, as rerac's `docs/test-requests/` shows.
12. **Audio.** Use software SPU2/ADPCM (PCSX2 rounding form) at 48 kHz into a cpal ring buffer filled by the tick. Match which sounds play and their volume, pan, and pitch, not exact SPU mixing.
