## Your task: `zero-g-movement-1b` — Bond's zero-G movement on 0700001b

Playtest 1016-1355 (the user, on master 2026-10-16): "we're in space ... there's a different movement system that you're missing". Port Bond's zero-G substate: `Player_ZeroG` 0x192ab0 (dispatch cases 8/9), the thruster controls, inertia, the auto-stabilisers ('Press Action to activate spacesuit auto-stabilizers') and the camera rules [M]. The level already starts in zero-g gravity (p3 == 2). Tests: movement integrates the game's way (drift with inertia; the stabiliser damps it); sister: on-foot levels are unchanged.

**Tests pin only proven behaviour.** Every assertion about what the game does cites its evidence: [M] a decompile
address that actually says it, [D] disc bytes, or [L] a live PCSX2 measurement from a FRESH boot (a save-state resume
skips the boot path). A guess, a port choice or an [S] item is never a hard assertion: mark it `// [S] unverified` and
keep it out of pass/fail. If an existing test contradicts the decompile, the TEST is wrong: fix it, with the address, and
say so in the report. Never weaken a decompile-backed test to make code pass.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/zero-g-movement-1b`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox3 at 2026-10-16T23:42Z

---
## Report (done, 2026-10-17T02:38Z)

## zero-g-movement-1b — Bond's zero-G movement on 0700001b

Branch `s/zero-g-movement-1b` (final commit merges origin/master as of 2026-10-17 ~01:35 UTC).
Playtest 1016-1355: "we're in space ... there's a different movement system that you're missing".

### What's done

`Player_ZeroG` 0x192ab0 is ported as dispatch cases 8/9 (`SS_ZEROG_8`/`SS_ZEROG_9`, new arm in
`Bond::tick_ctx`; crates/nf-game/src/player.rs):

- **Body-frame orientation.** ZeroG zeroes the scalar view pitch BL+0x8a8 every tick [M 0x192ab0:74]
  and rotates the object matrix `m := m·RotMatrix((pitch, yaw, 0))` [M 0x192ab0:160-162; RotMatrix
  0x1e2368; Concat 0x1e4a90 ⇒ `ab = a·ab`, i.e. the BODY frame — why the auto-stabiliser exists].
  The port carries this as `Bond::zero_g_rot` (rows right/up/forward, built from the walk yaw/pitch
  on entry — ChangeSubState 9 touches no rotation [M 0x190630:59-63]), and `Bond::look()` returns
  the matrix's forward row while in the substates (the camera/aim follow).
- **Thrusters + inertia.** The three smoothers: strafe = action 1·0.1 → BL+0x20, forward = action
  2·0.1 → BL+0x28 (the walk `local` pair), vertical = (action 7 − action 8)·0.1 → BL+0x24 (the new
  `zero_g_lift`); each eases `v += (target−v)·FRAME_RATE_MUL·0.04` with an overshoot snap and a
  ±0.05 clamp, and NO Player_Move dead-zone [M 0x192ab0:92-159; FRAME_RATE_MUL = 1.0 at 60 Hz, .sdata
  0x30d0d8]. Nothing clears them on entry or exit [M ChangeSubState 0x190630] — releasing the thrust
  decays at 0.96^n: the drift the playtest missed.
- **Integration.** `pos += local.x·row0 + local.y·row1 + local.z·row2` [M Player_Update 0x193600:
  284-295] — pitched forward + forward thrust flies along the look. `vel` (BL+0x50) zeroes EVERY
  tick [M 0x192ab0:71]: no gravity, nothing integrates. The collision tail (capsule push, floor
  ray) still runs like the game's Player_Collision [M 0x193600:361]; jump/crouch logic and fall
  damage are skipped (HandleJump is SSWalk-only [M 0x193600 dispatch]).
- **Auto-stabilisers.** The action-0xe read arms BL+0xd0 = FRAME_RATE·1.5 = 90 ticks [M 0x192ab0:
  163-166]; while it counts down, the orientation slerps toward `Mat_Align2Up(m, (0,1,0), fwd −
  (0,1,0)·(fwd·(0,1,0)))` (world up, same heading; the ±0.995 degenerate arms swap in ∓up [M
  :177-184]) at Quat_Slerp_Acc t = REC_FRAME_RATE·3.0 = 0.05 per tick [M :196], then Mat_Normalize
  [M :199, 0x1df5f8]. Ported via the decompiled Quat_MatrixToQuaternion 0x1e0600 /
  Quat_QuaternionToMatrix 0x1e02a0 and the existing `anim::slerp`.
- **Air.** `Player_MonitorAir`'s drain is disabled for 7 < substate < 10 — the suit refills even
  underwater [M 0x191a70:57, :98]; the port's `monitor_air` now gates on it.
- **Input.** `Input` gains `lift` (the held a7−a8 analog [M 0x192ab0:136-138] — pad: `a[7]-a[8]`,
  keyboard: Space minus held C/Ctrl) and `stabilise` (the action-0xe read — pad digital, and the
  Sim ORs its E/R edges in, both being action 0xe on the pad). Both are local view state, not
  net-encoded (the `zoom_cycle` precedent — the nf-net wire format is unchanged); remotes' Input
  defaults them off (mpnet.rs/replay.rs).

**Entry is still test-driven.** The level's zero-G is script-driven: C_CHCHZEROG 0x216388 answers
its 0x4b event with `Set_214 = Menu_SendMessage(ctrl, 0x40, 0, 0)` then
`Player_ChangeSubState(glb_players, Set_214 ? 9 : 0)` [M 0x216388:10-16] — the port has no
M_CONTROL message dispatch to hook, so tests assign substate 9 directly (the swim-entry
precedent). Nothing in ACTION.ELF ever writes substate 8 (no caller, no store) — only 9 is
reachable from scripts; the dispatch honours both.

### How it was verified

- Unit tests (player.rs, no ISO): thrust parallel to the look + the ±0.05 smoother cap + the
  release glide (0.04 ease, no dead-zone); the vertical thruster rides the body's UP row (straight
  up level, tilted when pitched) with `vel` pinned [0,0,0] and the hover settling (no gravity);
  the stabiliser's ×0.95-per-tick decay for exactly 90 ticks then expiry with the heading kept;
  the MonitorAir refill gate (sister: walk drains); walk ignores `lift`/`stabilise`. 246/246
  nf-game lib tests pass (debug and release).
- Live tests on the real disc (`crates/nf-view/tests/zerog_0700001b_live.rs`): through
  `Sim::step_bond` on 0700001b, after skipping the arrival script chain the way the Walker harness
  does — Bond hangs at the spawn (vel zero, y frozen), the lift thruster raises him >1 unit in 60
  ticks, and the walk sister (same lift input) stands and stays substate 0. Sisters green:
  `fix_world_gravity_live` 5/5, `walkreal_0700001b_live` (the hub-floor route never enters 8/9).
- Merge: origin/master (G-046's PS2Sinf `look()`, the visual batch) merged; the one code conflict
  (player.rs `look()`) resolved keeping both — master's scalar path stays the non-zero-g branch.
- **Full suite, chunked.** This sandbox kills every background process between tool calls (the
  nohup'd suite died twice, output empty; `nohup sleep 300` died the same way), so the fast
  runner ran in foreground chunks via `--only` (`--limit 45/90`, the default 600 is impossible in
  a 10-minute tool call):
  - chunk 1 of 6 (bins a-c): **96 passed, 0 failed**, 8 ignored;
  - chunk 2 (c-f): **88 passed, 0 failed**, 4 ignored — plus one TIMEOUT at the 45 s chunk limit:
    `fix_guard_weapons_live`, re-run directly: **3/3 pass in 69.6 s** (limit artifact, not a
    regression);
  - chunks 3-4 (f-s): **358 passed, 0 failed**, 3 ignored — two TIMEOUTs at the 90 s limit:
    `frame_cost_live` and `levels_sweep_live`, both whole-disc sweep tests that run minutes under
    the runner's default 600 s limit (not re-run — no time before the deadline; neither touches
    player movement);
  - chunks 5-6 (bins s-z, 56 binaries) did not run: the sandbox deadline hit first.
  No real failure anywhere in the 542 passed.

### What's left

1. **The script entry**: port `C_CHCHZEROG` 0x216388 into the level-script/control-message system
   (msg 0x4b in, the `Menu_SendMessage(ctrl, 0x40, 0, 0)` query, `Player_ChangeSubState(9|0)`,
   the msg 0x51 → 0x2e exit arm). Until then 0700001b plays with zero gravity but the spacewalk
   substate only under test.
2. **Substate 8** has no writer in ACTION.ELF; if the msg-0x40 query turns out to gate a
   weapon-out variant, the port may need the 8/9 distinction beyond the anim gate [M 0x192ab0:201].
3. **AnimSetUpdate** for substate 9 (the movement anim set, args (|+0x18|, +0x18, 0.1·FRM, +0x10,
   0.075·FRM) [M 0x192ab0:201-205, AnimSetUpdate 0x1c0938]) — visual, needs the main agent's
   render pass; the port's Bond body anims are not driven per-substate today.
4. **Camera**: the port's eye offset rides world-Y for every substate; the game positions the
   camera off the object matrix. Fine while level (the stabiliser's job), wrong upside-down —
   a render-pass item.
5. Suite chunks 5-6 (56 bins, alphabetical tail) not run; the two sweep TIMEOUTs re-run under the
   default limit.

### Live checks for the main agent

1. On 0700001b, walk into the spacewalk trigger zone and watch `obj+0xF6` (the substate): expect
   it to become 9 [M C_CHCHZEROG 0x216388] — and record whether the msg-0x40 query returned
   nonzero at that moment (the 9-vs-0 arm).
2. Press R2 (action 0xe) while in zero-g: watch `BLData+0xd0` — expect 90, decrementing each tick
   [M 0x192ab0:163-168], and the object matrix's up row easing toward (0,1,0).
3. Thrust forward in zero-g: watch the BL+0x10 quad — expect z ≈ +0.05 saturated after ~17 ticks
   (the 0.04 ease from 0 to the 0.1 target, clamped ±0.05 [M 0x192ab0:92-159]).
4. Press/hold jump (action 7) in zero-g: the BL+0x14 lane should ease toward +0.1 (clamped ±0.05)
   and Bond should drift up along the body's up row, gliding after release.

## Usage

not visible to me — this session exposes no token, credit, context-window or cost figures in any
UI or environment output I can read.

