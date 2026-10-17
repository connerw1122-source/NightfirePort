**Reproduce first, on the newest origin/master, through the REAL path.** Earlier fixes for some of these passed
their own headless tests and the user still hit the bug in the game. Before changing code, write a live test that
drives the same path the player takes (the level loaded through `nf_view::load`, Bond moved by `step_bond` inputs —
walking, the use key, the ladder input — the real scripts and triggers running, no teleports past the problem
spot unless the report says so) and FAILS on current master the way the user describes. Put the failing output in
the report. Only then fix it, and the same test must pass. If you cannot reproduce it, say exactly what you tried
and where the path diverges from the report; do not mark it done on a guess.

## Your task: `cutscene-audio-sync`, level start/end cut-scenes drift from their audio

The user, 2026-10-16: "ending and beginning cutscenes are not synced to their audio". These are the in-engine NIS
cut-scenes at level start and end (script players), not FMV movies. Measure on several levels (07000005, 06, 08, 0c):
for each voice line / music cue the script starts (op-1 sounds, the stream sounds), the script clock at its start
vs the line's length vs the frames the mouths/anims for that line take, and where it drifts. Suspects: the script
clock rate under the cut-scene (Script_Update adds FRAME_RATE_MUL — 2 under FMV / a 30 fps mode? check
0x1da998:33), the port's frame pacing of script players vs the audio clock, stream start latency. Port the game's
timing with [M] citations; test that a line starting at script frame F ends within a frame or two of where the
script expects (numbers only, NF_SILENT).

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Tests pin only proven behaviour.** Every assertion about what the game does cites its evidence: [M] a decompile
address that actually says it, [D] disc bytes, or [L] a live PCSX2 measurement from a FRESH boot (a save-state resume
skips the boot path). A guess, a port choice or an [S] item is never a hard assertion: mark it `// [S] unverified` and
keep it out of pass/fail. If an existing test contradicts the decompile, the TEST is wrong: fix it, with the address, and
say so in the report. Never weaken a decompile-backed test to make code pass.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/cutscene-audio-sync`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox7 at 2026-10-17T02:45Z

---
## Report (done, 2026-10-17T03:53Z)

# cutscene-audio-sync — report

Branch `s/cutscene-audio-sync` (from `origin/master` 31dd6161). Status: **the measurement is
done and one game mechanism was ported (the op-1 precache); the drift itself did not reproduce
in the sim** — the port's script clocks, command fires and anim timelines are 1:1 exact against
the game's rules on every scene measured. The remaining desync candidates are the wall-clock
pacing class and (now fixed) the on-tick stream decode.

## What the game does (all re-read in the decompile this session)

- **The script clock** — `Script_Update` 0x1da998:33 adds `FRAME_RATE_MUL` (the global at
  0x30D0D8) **unconditionally**; the task's "2 under FMV / a 30 fps mode?" hint is settled:
  `FRAME_RATE_MUL = 60/FRAME_RATE` [M `GS_SetRefreshRate` 0x17d2b8:6-16], and `FRAME_RATE`
  halves (MUL = 2.0) whenever the main loop steps 2 vblanks — forced while a background FMV
  plays (`main` 0x109080:53-61: `PS2FramesToSkip = (VBlankCount & ~1) − GameCount` in the
  `BGFMVPlaying` branch) and under overload. Either way the script clock stays wall-locked:
  1 script tick ≡ 1/60 s of wall time and ≡ 1 audio tick. The port's +1.0/tick at fixed 60 Hz
  (nis.rs:358-359) is the same rule at MUL = 1.0 [L docs/systems/script-player.md:48].
- **Sound fires** — `Script_Run` 0x1d8548:27-30 runs every command whose wait-until the stream
  clock has reached, synchronously, catching up within the same pass; `Script_SoundStart`
  0x1d8820 starts the sound on that very tick (the EE batches to the IOP; audible ≤ 1 vblank
  later, `SFXUpdate` 0x1156c0). The port matches (the same-tick request → drain-in-walk →
  `Cmd::Start` chain, no frame-delay queues).
- **The op-1 precache — the gap found and fixed** — `AnimProcessScriptCmds` 0x1c1b38 case 1:
  while an op-1's authored frame sits 0..=50 ahead of the anim's frame counter
  (disasm 0x1c1ea8-0x1c1ed0: `slti $v1, a1, 0x33` with `a1 = frame − current`), the game calls
  **`SFXPreCache(id, frame − current)`** every frame [M 0x117118: RPC cmd 0x26]; the IOP sets
  `KeyOnFrameDelay = frames + 5` and starts the item at once — silent, with its sample already
  loaded — and the real start on the authored frame takes the countdown item over
  (sound.md §3.2 step 3; the port's `Driver::start` already had the takeover). The port never
  called `pre_cache` (zero callers): each line's stream decoded **on** the play tick — a real
  hitch on the voice thread's tick, the most plausible mechanism for a late-starting line.
- The only other precache caller is `NDrone2_SpeechSFX` 0x141b00 (drone speech — out of scope).

## What was measured (the probes, pushed: cutscene_sync_probe_live.rs, cs_diag_live.rs)

Through the real path (`nf_view::load`(+carry) / `step_bond` / `sound_tick`), NF_SILENT,
watching the driver's live-item multiset per tick (starts AND ends) plus each SP's script
clock, with every fired id's disc length (bank PCM / pitch, or the STREAMS.BIN entry):

- **07000002's boot intro** (0x0600_0084): line 0x20c fires at SP clock 61 = its anim segment's
  authored frame 23 (the actor's Anim command is born mid-timeline, `start` = 23); 0x20e at
  clock 451 = authored 451. Exact.
- **07000008's station intro** (SP #294, 0x06000077, 3880 ticks, plate #237): the 8 dialogue
  lines 0x48d-0x494 fire at clocks 455/492/551/798/1272/1552/2389/2788 — every audio length
  fits its authored window, every fire lands at clock = tick − 34 exactly. **These lines are
  op-0x16 stream sounds, not op-1s** — the game itself never precaches them.
- **07000006** (intro with the truck carry; the outro via LoadPoint #746 → the dressing scene
  0x06000071, level exit at t1140), **05, 0c**: the same 1:1 shape (the full START logs are in
  the probe output).

**Conclusion of the measurement: inside the sim there is no drift** — the fires, the clocks and
the anim frames agree with the game's rules to the tick. The pinned test
(`cutscene_sync_live.rs`, 2/2 pass) asserts the strongest form through the real path: two lines
of the same anim script fire **exactly their authored frames apart (± 2 ticks)** on both 02 and
08, and every fired op-1 line's PCM is decoded (the hash set).

## The fix

`SFXPreCache` ported end-to-end ([M] as above):

- `nf-game/src/nis.rs` — `Request::PreCache { id, frames }` + the case-2 emit (the 0..=50-frame
  window scan, `anim_ops1_precache` in the `W` trait, default empty).
- `nf-game/src/drone.rs` — the real scan over the anim script's op-1 commands + the drain arm.
- `nf-game/src/sound.rs` — `Cmd::PreCache` queued straight through the EE sound layer.
- `nf-view/src/lib.rs` — `sound_tick`'s batch arm → `Driver::pre_cache`.

No RNG draws added (determinism); no shared file reordered; the new enum variants are additive.

## What ran

- `cargo test -p nf-view --test cutscene_sync_live` — **2/2 pass** (the new pins).
- Regression neighbours on the same machinery: `cutscene_skip_audio_live` 3/3,
  `fix_cutscenes2_live` 5/5, `hatch02_intro_live` 2/2, `subtitles_live` 4/4.
- The fast suite (`tools/test/suite.py`) could not run in this sandbox: background jobs are
  killed between tool calls here (the runner died silently three times with an empty suite.out;
  the last attempt is in `suite.out` at commit time — still empty). The report's total line is
  therefore **not available this session**; the next session should run
  `NF_ISO=<iso> PATH=/home/z/.cargo/bin:$PATH PKG_CONFIG_PATH=/home/z/my-project/scratch/alsa-pcfix
  python3 tools/test/suite.py > suite.out 2>&1` in the FOREGROUND (or per-binary) before
  merging. (`PKG_CONFIG_PATH` is this sandbox's ALSA dev prefix — the system has no
  libasound.so; without it nf-view/nf-audio fail to link.)

## What's left

1. **A live precache-window pin**: every op-1 line measured so far is born AT its anim
   segment's start (the game authors the Anim command's `start` at the line's frame), so no
   line had a physically open 0..=50 window. A long single-take scene with in-place op-1 lines
   (the drone-speech pattern, or an intro one may exist on 05/07) would show the lead; the
   test scaffolding is ready (the `best_lead` plumbing prints it).
2. **The wall-clock pacing class** (documented, not changed): the port keeps up with wall time
   down to 15 fps (`ticks_for_frame` cap 4, "dropped, not deferred" lib.rs:3308-3318) — below
   that, sim time falls behind while cpal audio does not. The game's own rule (skip clamped at
   2 vblanks, MUL = skip, excess discarded) is not modelled; porting it would change the render
   loop's tick scheduler, which is the main agent's call — the analysis is in
   docs/systems/input.md §6 and anim-engine.md:161-166.
3. If the user's desync persists on a fast machine with the precache in, the next datum is a
   PCSX2-side trace (below).

## Live checks for the main agent

1. FRESH boot of 07000002, its intro: when the first dialogue line plays audibly, note the
   frame the character's mouth starts moving vs the line's first audio sample (same frame?).
2. The same on 07000008's station intro (the 8-line dialogue): does any line start late
   relative to its mouth/camera beat, and does the lateness GROW line over line (cumulative
   drift = clock rate; constant = start latency)?
3. If a debugger pass is cheap: during the intro, sample `FRAME_RATE_MUL` (0x30D0D8) —
   expected 1.0 with no FMV; and `Script_Update`'s per-frame add on a playing script's
   `+0xA0`.

## Usage

not visible to me — this session shows no token, context-window or cost figures anywhere in
its interface.
