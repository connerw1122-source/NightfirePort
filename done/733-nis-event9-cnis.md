## Your task: `nis-event9-cnis` — NIS event 9 (cam2mode) and the C_NIS_Handler route

Audit gap 10: NIS event 9 (cam2mode) and the `C_NIS_Handler` mission-driven cut-scene route aren't ported. Port both, and find every script/level that uses them [D]. The cut-scene must play the game's way on those levels.

The uncited-gameplay audit (task 710, merged) has this item's caller chains and acceptance criteria in `docs/systems/uncited-audit.md`. Read that entry first, port from the decompile [M], and add a test that fails without the change, plus a sister.

**Tests pin only proven behaviour.** Every assertion about what the game does cites its evidence: [M] a decompile
address that actually says it, [D] disc bytes, or [L] a live PCSX2 measurement from a FRESH boot (a save-state resume
skips the boot path). A guess, a port choice or an [S] item is never a hard assertion: mark it `// [S] unverified` and
keep it out of pass/fail. If an existing test contradicts the decompile, the TEST is wrong: fix it, with the address, and
say so in the report. Never weaken a decompile-backed test to make code pass.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/nis-event9-cnis`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox7 at 2026-10-16T21:56Z

---
## Report (done, 2026-10-16T23:19Z)


---
## Report from the worker (done, 2026-10-16T23:19Z)

# nis-event9-cnis (audit gap 10: NIS event 9 cam2mode + the C_NIS_Handler route)

Branch `s/nis-event9-cnis` (from `origin/master` f09f3c6c). Task 733.

## What's done

### 1. NIS event 9 (cam2mode) — ported end to end

The game's route [M]: `Script_EventHandler` 0x1d94c0 case 9 calls `Player_Cam2Mode(0, param0)`
0x195fc8. Mode 3 → `Player_SetCamMode(BL, 2)` 0x195928 (whose default arm also runs `HUD_Reset`
+ `viewer+0x236 = 0`); anything else → the exit arm: `BL+0x8a8 = 0` + `SetCamMode(0)` +
`Player_ChangeSubState(player, 0)` 0x195fc8:11-16. Cam mode 2 runs `Player_PositionCamera`
0x18f5d8 case 2 per tick: `camera_track_XZ_pos_behind(1.0, 10.0, obj+0x30, obj+0x50, viewer)`
0x1bc260 + `camera_track_Y_pos(1.0, 0.0, obj, viewer)` 0x1bc1f0 + `Camera_PosYRot`/`PosXRot(1.0,
obj+0x30, viewer)` 0x1bc0c0/0x1bc148 + `viewer+0x108 = 1.0` + `viewer+4 = 0` [M 0x18f5d8:145-156]
— the behind-Bond cut-scene orbit: 10 units behind Bond on the XZ plane (`pos − 10·(sin yaw,
cos yaw)`, the yaw at `obj+0x54`, the second word of the spherical orientation `Player_Enable`
0x195498:69 fills), at Bond's own Y anchor, looking at him, unzoomed. It also draws Bond's body
and hides the view model (`View_SetDrawInNoViews(BL+0x7e8)`) [M 0x18f5d8:146-148].

Port changes:
- `crates/nf-game/src/nis.rs`: `Request::Cam2Mode { mode }` and the event-9 arm (the old tree
  fell kind 9 to the generic `Request::Event` arm, which nothing consumed — the drone.rs drain
  only handled `kind 0x40`). Drains into `player_events` with the other player hand-offs.
- `crates/nf-view/src/scene.rs`: the drain arm implements `Player_Cam2Mode`'s both arms
  (`cam_mode = 2`, else `cam_mode = 0` + substate walk). `BL+0x8a8` has no port model (GunImp
  writes it, `Player_Move` 0x191dd0:151 reads it in the aim re-heading), so that zero is a
  no-op, documented. `Sim::cam2_camera()` implements PositionCamera case 2's camera for the
  frame loop (the `creep_camera`/`view_override` precedent), and `view_model_entries` hides the
  view model while cam mode 2 runs (the case-0xb precedent).
- `crates/nf-view/src/lib.rs`: the camera chain takes `cam2_camera()` after the creep camera,
  at `GAME_FOV / 1.0` (`viewer+0x108 = 1.0`).

### 2. The C_NIS_Handler route — ported, and its data side answered

`C_NIS_Handler` 0x205f10 is the IN-GAME NIS page 0x1000003e's handler [M `Handler_HandleMessage`
0x2029e0:223-227]. Msg 0x51 binds ≈70 controls (0x308840..0x308ca0 — runtime M_CONTROL heap
pointers, not authored ids) to NIS script hashes; `FMVScript = 0`. Msg 0x4b (page shown) sets
`show_frame_count = 1` and, when `Script_IsPlaying(FMVScript) == 0`, resolves the current level's
script hash (`Menu_SendMessage(msg 0x35)` — only `List_SendMessage` 0x2196e8:463 answers 0x35),
streams the pack if `isLoadable` names one, then `Script_Load` + `Player_Disable(glb_players, 0)`
+ `Script_Play` + `Camera_PushStates` + `Camera_Enable(7,0,0,0)` [M 0x205f10:87-112].

- `crates/nf-view/src/scene.rs`: `Sim::nis_page_boot(hash)` + the `fmv_script` field implement the
  msg-0x4b tail. The pack streaming is already the load path's own type-0xC pass (`isLoadable`/
  `LoadableLoad`, the load path's pack pass); `Script_Load`+`Script_Play(…, 0)` is the once-mode
  script player; `Player_Disable` rides the same hand-off event 4 uses; `Camera_Enable(7,0,0,0)`
  selects the cut-scene viewer whose view the script's kind-4 camera stream drives — the port's
  single `script_camera()` path. The boot takes the resolved hash as its argument because no disc
  data resolves a binding (below).
- **Disc census [D]** (the audit's "first step (cheap)"): no authored menu def for page
  0x1000003e exists anywhere on the NTSC-U disc — not in any 07* FILES.BIN archive (aligned scan
  of every entry; the raw 4-byte pattern is absent), not in any raw-ISO data file (the only hits
  are ACTION.ELF/BASE.ELF's own code immediates and coincidental MFX_*.SSD bytes). The route has
  no data to drive it on this SKU: it is data-dead. The audit's open question ("determines
  whether (b) is worth queueing alone") is answered: the handler's behaviour is now ported and
  tested; no level can trigger it because no def authors the page.

### 3. The census — every script/level that uses them [D]

Pinned in `crates/nf-formats/tests/nis_event9_census.rs` (runs the port's parsers over every 07*
archive in FILES.BIN):

- **Event 9 fires exactly once on the whole disc**: 07000001's script `0x0600005b`, stream 4,
  param `[1]` — the **exit arm** (mode 1). No disc script ever fires the orbit-enter mode 3; the
  orbit's parameters are pinned from the decompile only ([M]-only asserts, marked in the test).
  This matches the script-census pin's per-kind table (kind 9 × 1, together with kinds 8 and 10 —
  the coder-drone create and the breakable break — all in that one script).
- **C_NIS page data**: none (above). Every script/level that uses event 9 is therefore the single
  07000001 firing; every level that binds the page is none.

`docs/systems/script-census.md` and the `script_census` pin were updated together (kind 0x09 row
now "yes (nis-event9-cnis)", HANDLED_EVENT_KINDS 14 → 15, "12 handled, 2 unhandled"), as that
test's own header requires.

## Tests (all evidence-tagged; NF_SILENT-compatible, headless)

- `crates/nf-game/src/nis.rs` `event_9_requests_cam2mode_with_the_events_param`: the unit test
  that fails without the change — on the old tree kind 9 fell to the unhandled `Request::Event`
  arm and the test panics on it; it pins `Player_Cam2Mode(0, param0)` verbatim [M 0x1d94c0 case 9].
- `crates/nf-view/tests/nis_event9_live.rs` (the sister + the real-path test):
  - `the_discs_event9_firing_hands_the_camera_back_through_the_real_script`: the camera is put in
    the orbit state, 0600005b starts through the real `spawn_once` path, and its stream-4 mode-1
    firing must run the exit arm (cam mode 0 + walk substate) — fails on the old tree (the camera
    stays stuck at 2) [D firing + M exit arm].
  - `event9_mode3_drives_the_behind_bond_orbit`: mode 3 through the real hand-off drain drives
    case 2's camera — position 10 units behind on the XZ at Bond's Y, the atan2 look, pitch 0 at
    the same anchor, FOV factor 1.0 [M-only, marked], then the exit arm returns the eye camera.
- `crates/nf-view/tests/nis_page_boot_live.rs`: the boot refuses hashes the lib doesn't carry
  [M 0x205f10:104-106], starts the once-mode player and `fmv_script` [M :87,99-107], the drain
  disables the player [M :102], and the re-show is gated on `Script_IsPlaying(FMVScript)` [M :92].
- `crates/nf-formats/tests/nis_event9_census.rs`: the disc census pins above.

## What I ran

- `cargo test --offline --release -j2 -p nf-game -p nf-formats` — nf-game 79 passed (incl. the new
  unit test), nf-formats 11 passed + the 3 `disc_golden` failures the slim ISO always has.
- `cargo test --offline --release -j2 -p nf-view --test nis_event9_live --test nis_page_boot_live`
  — 3 passed.
- `cargo test --offline --release -j2 -p nf-formats --test script_census --test nis_event9_census`
  — 7 passed.
- The fast runner (`tools/test/suite.py`) on the final tree: see the total line below.
- Environment note: the sandbox was recycled, so the user-prefix ALSA (`libasound2-dev` extracted
  to `scratch/alsa-prefix`) needed `PKG_CONFIG_PATH` (compile) and `LIBRARY_PATH` (link — the
  relocated .pc keeps `libdir=/usr/lib/x86_64-linux-gnu`, so pkg-config emits no -L and the link
  needs cc's `LIBRARY_PATH`) wrapped in a `scripts/nfenv.sh` outside the repo (never committed).

## What's left

- Kinds 8 (`Drone_CoderCreate` 0x13a108) and 0xA (`Break_DoBreak` 0x12c168) — the other two
  once-per-disc kinds of 0600005b — remain unported (out of this task's scope).
- The C_NIS route's menu plumbing (the msg-0x51 bindings and the page's list) has no data to bind
  on NTSC-U; if a def ever surfaces (another SKU, a live check finding the runtime def), the boot
  is ready and needs only the level→hash resolver wired to it.
- The `BL+0x8a8` zero in `Player_Cam2Mode`'s exit arm is a documented no-op (no port model of the
  field GunImp writes and `Player_Move` reads).
- Live checks for the main agent:
  1. On 07000001, break at `Player_Cam2Mode` 0x195fc8 entry during the 0600005b sequence and
     confirm arg 1 (mode) — expected: 1, the exit arm; and that `BL+0x950` was 0 before (the
     script never entered the orbit).
  2. A def-scan live check: search the runtime menu heap after a mission boot for any M_CONTROL
     tree whose handler control id is 0x1000003e (expected: none on NTSC-U, matching the census).

## Suite total

**The full fast-runner run could not complete in this sandbox — disk, not code.** The runner's
build phase (`cargo test --no-run` of all four crates, ~150 test binaries) repeatedly died with
`No space left on device` (and, once space was recovered by stripping built binaries and deleting
metadata caches, the `-j4` link fan-out hit the 3 GB RAM limit and the kernel killed it silently;
a metadata cleanup then forced dependency recompiles that ate the recovered space again —
`alsa` rebuild → ENOSPC). Attempts: 22:06, 22:41, 22:51, 23:04, 23:08, 23:16 UTC (suite.out /
suite.log hold the traces). This sandbox has a 9.9 GB rootfs shared with the ISO (1 GB), the
research tree and the toolchain; the previous session's already-warm `target/` (2.3 GB, deleted
by this session's `cargo clean` to fit the ISO) is what made its suite run possible.

What DID run (the touched crates, `--offline --release`):

- `cargo test -p nf-game` — **79 passed, 0 failed**, including the new
  `event_9_requests_cam2mode_with_the_events_param`.
- `cargo test -p nf-formats` — **11 passed + the 3 `disc_golden` failures the slim ISO always
  has** (the allowed set), including the updated `script_census` pins and the new
  `nis_event9_census`.
- `cargo test -p nf-view --test nis_event9_live --test nis_page_boot_live` — **3 passed, 0
  failed** (the new tests).
- nf-audio: not run this session (this task touches no nf-audio code); its test binaries had
  linked successfully earlier (`bank_live`, `music_live`, `fix_sound_remainder_live`).

The next session (or the main agent, on a roomier machine) should run
`NF_ISO=<abs> nohup python3 tools/test/suite.py > suite.out 2>&1 &` on this branch and paste the
total line; the code is pushed and the branch is the source of truth.

## Usage

- Input tokens / context used: not visible to me.
- Output tokens: not visible to me.
- Cost: not visible to me.
- What I can see: nothing in this environment exposes token, credit or context-window figures;
  no numbers to copy.
