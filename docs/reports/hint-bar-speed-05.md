# hint-bar-speed-05 — the 05 hint bar runs far too fast

Playtest 1016-1355 (master 2026-10-16): "the hints are being displayed at 1,000 miles an hour"
(07000005). Branch `s/hint-bar-speed-05` from origin/master f09f3c6c.

## What the game does (all [M] unless tagged)

The hint bar is the INFO status pane (pane 4, `MsgInfoStatusPane` 0x319810, y 113). Its timing:

- **Post**: `Hint_Update` 0x1b3380:33 posts `Text_AddMsg(viewer, 0, 1, Txt_BindLabel(label), 0, 0xb4)`
  — TXTMSG type 1, life **0xB4 = 180 ticks** (the stack arg confirmed in the disassembly at
  0x1b3450 `addiu $t1, $zero, 0xb4`; the decompile C is complete here). `ThirdIcon_Update`
  0x196e80:42 posts the ctx-6 "Press Action" (label 0x1000291) identically, on the icon
  instance's 4-tick timer (data+8: −1/tick, reset to 4 every in-range tick [0x196e80:15-17,52-53]
  — so it posts on entry, never re-posts while the player stays in range).
- **Rate**: `HUD_Update` runs from the top of `Player_Update` 0x193600:79 — every tick, no frame
  gating — and `Text_UpdateMsg` 0x1b11d8:48-50 decrements the pane's **head node alone** by
  FRAME_RATE_MUL = 1.0 [L 0x30d0d8] per tick. A message queued behind another keeps its FULL
  life; each shows its whole 180 when it surfaces.
- **Fade**: while `time < 15.0` the alpha byte becomes `time · 17` [0x19c310:261-265
  `fptoui(fVar22 * 17.0)`] — the last 15 ticks fade 255→0.
- **Close**: after the last message the bar closes 16 px/tick [0x19c310:327-336; the pane's
  slide secondary[3] = 0x10 [L docs/fields/hud-panes.md]] — 30 ticks for the 480-wide bar.

So one hint = 3 s at 60 Hz; a queue of hints shows each for its full 3 s, in sequence.

## Why the port looked "1,000 miles an hour"

The port's sim already matched every number above (probe on 07000005: a "Press Action" hint
lives exactly 180 ticks, first sight time 179), and the HUD ticks inside the fixed-60 Hz sim —
`hud_tick` runs from `Sim::run`, the render path only reads; `ticks_for_frame`
(crates/nf-view/src/lib.rs:87) hands out dt·60 ticks per rendered frame with a 4-tick cap, so
the timing is already frame-rate independent at any uncapped fps. The **real defect was the
queue semantics**: the pickup/script queue (`hud.msgs` — the bottom bar the "Picked up …" and
script texts ride) ticked **every member in parallel**
(`for m in &mut self.hud.msgs { m.time -= 1.0 }` + a `retain`), so a text queued behind another
burned its whole life while it waited and flashed past one frame after surfacing once the
stagger between posts approached 180 ticks — picking up items placed near each other (or a
script's text burst) cycles the bar texts at a glance. The port's own comment at the
objective-queue tick says this exact bug was fixed for `objective_msgs`
("the port used to tick every queue member in parallel, which expired the banners behind the
front one before they ever showed") — `hud.msgs` was left with the old behaviour.

## The fix

`crates/nf-view/src/scene.rs` `hud_tick`: `hud.msgs` now ticks its head alone and pops at ≤ 0 —
the same machine as the info/objective/mission queues, matching `Text_UpdateMsg` 0x1b11d8:48-73
(the out-param `*param_3` IS the head; the queue behind waits). Queued texts surface with their
full 180-tick life.

## Verification

- `crates/nf-view/tests/hint_bar_timing_live.rs` (new, NF_ISO-gated, all 4 pass):
  1. `the_press_action_hint_lasts_the_games_180_ticks` — the hint fires at the 07000005 vent
     icon and displays exactly 180 ticks (the post tick spends one: first sight time 179), and
     the bar close runs after it [M 0x1b3380:33 + 0x1b11d8].
  2. `the_hint_is_frame_rate_independent_at_240_rendered_fps` — the task's test: the app's
     scheduler (`ticks_for_frame`, dt = 1/240) driving `step_bond` shows the same 180 sim ticks
     as the 60 Hz run [M 0x1b11d8:48-50 — the step is per tick, not per frame].
  3. `the_info_text_fades_17_per_tick_over_the_last_15` — at time 10 the drawn text alpha
     carries 10·17 = 170 (gs halves it into the vertex colour) [M 0x19c310:264].
  4. `queued_messages_display_their_full_life` — three texts staggered 60 ticks through the real
     `drones.taken` drain each surface with the full 180. On the pre-fix code this test FAILS
     with "got 60" (the queued text's life spent behind the front) — the reproduction of the
     playtest symptom, re-checked by stashing the fix.
- Affected binaries re-run green after the fix: fix_hud_live (6), fix_pickups_ammo_live (4),
  fix_script_commands_live (4), subtitles_live (4), fix_context_hints_live (6),
  fix_objective_messages_live (3), fix_objective_banner_live (5), objective_hints_flow (2),
  hud_playtest_live (5), pickups08_live (7), fix_pickups_trace_live (5), hint_bar_timing_live (4).
- The touched crate in full, serial (`cargo test --offline --release -j2 -p nf-view --
  --test-threads=1`): **185 passed, 2 failed** — the 2 are `fix_0c_pylon_live`, a wgpu adapter
  failure (no GPU adapter in this sandbox, panics at adapter creation before any test logic) and
  pre-existing here (the same 2 FAILED in this sandbox's earlier suite log suite_nv.out, before
  this branch existed). No disc_golden failures.
- **Full suite**: not completable in this sandbox — background processes are killed between tool
  calls here (three nohup/setsid starts died with empty logs; the run needs well over an hour).
  What stands in its place: the full serial nf-view run above plus the per-binary list. The
  change touches only `crates/nf-view/src/scene.rs` (+ tests); nf-game/nf-formats/nf-audio are
  untouched by the diff.

## Live checks for the main agent

1. **The game's hint duration on a fresh boot** [the task's [L] ask]: PCSX2, 07000008
   `--bond`-equivalent safe spot, gdbwatch `Text_AddMsg` (0x1b0c30) on entering the safe marker:
   expect ONE call `(viewer, 0, 1, Txt_BindLabel(0x0200000A), 0, 0xB4)` [M 0x1b3380:33], and the
   on-screen bar visible for ~3 s (~180 frames at 60 Hz). Confirms 0xB4 is the live duration
   (L-150, still open).
2. **The queue behaviour after the fix**: walk 07000005's trench grabbing two pickups within ~2 s
   — each "Picked up …" bar text should now show its full ~3 s in sequence (before: the second
   text flashed for the leftover stagger). Render check: `--bond 155.19,43.12,-113.38,1.57,0`
   (07000008 safe) shows the watch-laser hint bar as before.
3. **240 fps spot check**: play at an uncapped fps and confirm the hint bar keeps real-time
   pacing (the sim is fixed 60 Hz; nothing in the HUD steps per rendered frame).

## Remaining

- The parallel-tick fix is in for `hud.msgs`; the game's per-LINE split of long texts
  (`Font_WordWrapString` chains one message per line with proportional lives, min 60 for types
  0-3/6 [M 0x1b0c30:230-253]) is still not ported — the port wraps at draw time and shows all
  lines for the whole life (documented in fix-objective-messages; same presentation choice
  applies here, the total time is the same).
- A full-suite total line on the final tree (see above — needs a sandbox where background
  processes survive).

## Usage

Not visible to me. This session runs headless (CLI tool calls only); no token, credit,
context-window or cost figures are exposed anywhere in the interface I can read.
