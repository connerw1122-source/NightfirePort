# fix-0c-hatch-latches2 (109n2) — the merged 109n holds on today's master

Task: `s/fix-0c-hatch-latches` (109n, NOT merged) failed on current master — its own two live tests
and two older 0c suites (6 failures). Merge it into master, find what changed underneath, make its
fixes hold without breaking the older suites. Branch: `s/fix-0c-hatch-latches2` @ bd844cf2
(master 1288f0d6 + 109n + my fixes, then merged with master dc44c0e4).

## What was actually wrong (not "master changed under it")

I merged `origin/s/fix-0c-hatch-latches` (base 2cbfd7e3) into master 1288f0d6 and reproduced all six
failures headless. Master's only code changes under 109n were casing.rs/drone.rs (sg5-casings'
`eject_gun`) and the stunner-press taser family — none of them touches the SS or cel paths. The six
failures decompose into 109n's own two open defects plus its re-pointing of the older suites' pins
to the decomp truth, with three real root causes:

### 1. The bullet could never drive a solid SS (fix_0c_hatch_latches_live::a_real_shot_starts_the_winch — "got t 0 → 0")

Two stacked causes, both verified by a tick-by-tick probe:

- **The shot never fired.** The test held the trigger for 8 ticks right after `Bond::stand_at` —
  the loadout weapon was still finishing its draw, and `player_bullets` stayed empty. It also aimed
  level from the standing eye (y ≈ 16.4) at a winch whose dynamic box tops at y 14.47, so even a
  fired round passed 1.9 over the box.
- **The port's contact stand-in structurally dropped the decisive contact.** 109n's ss_contacts
  block scanned `player_bullets` skipping dead ones — but `update_player_bullets` REMOVES the dead
  bullets (`retain(|b| !b.dead)`, drone.rs) before the SS block runs, and a bullet that reaches the
  solid winch dynamic box dies on it as a world hit in the same pass. In the game the same collision
  is what registers the contact on the hit object's `obj+0xD0`, which `SS_Operate` 0x1a9df8:12-45
  then reads [M]. So the winch (a solid SS box) was unreachable by construction.

Fix (drone.rs, additive): `update_player_bullets` records each flying bullet's flight-segment
overlap against the SS dynamic boxes into a new per-tick `ss_bullet_contacts: Vec<u8>` (bit 1),
cleared at the pass's head; the tick's SS block merges it into the `SS_Operate` contacts. Dead
bullets register exactly like the game's collision pass — the death IS the contact. The winch test
also settles 150 ticks, aims down at the winch box (the `aim_at` convention), and holds the trigger
through the draw. Green: the shot starts the sweep AND the ride assertion runs (the platform carries
Bond).

### 2. The measured straddle facts (fix_0c_hatch_latches_live::the_two_bottom_hinge_latches — "hinge #140's sphere straddles … left: true")

109n's HINGES table pinned (140, false)/(142, false) — "the top hinges live in cel 7 alone" — written
from the theory, not the probe. Measured on the disc data: **all four hinges straddle** (bottom
#139/#141: 5 cels 7/11/13/14/16, home cel 16 → cell 212 NOT in the walkway chain — the user's two
missing latches; top #140/#142: 4 cels 7/11/13/14, home cel 7 → cell 91 IN the chain — they drew on
master through the home cel). The test now pins the measured facts: straddle for all four [D], the
bottom pair's `!visible` repro, and the post-fix `hidden = !visible && !straddle` draw rule
(`View_AddForcedObjects` 0x1e5fb8:85-145) holds for all four.

### 3. The older suites pinned the scrub model's clamped playhead, not the game's end-of-sweep state (fix_level_0c_live ×2, fix_0c_vent2 ×2)

The decomp's `SS_Update` 0x1a9948:119-128 [M]: after the sweep the playhead WRAPS to 0 and the state
goes 0x40 (rearm 0) or dead (rearm ≠ 0); what persists is the POSE at the last key (states 0/0x40
never interp). Master's old scrub model clamped t at the last key forever, so the older suites'
`t == 5.0` / `t == 1.0` pins passed for the wrong reason. With 109n's correct machine the sweeps
complete and wrap — t reads 0 at check time. Re-pointed to the game's observables, all asserted with
decomp citations:

- `burning_the_four_hinges_opens_the_display_vent`: the posed matrix equals the last-key interp
  bit-exact, state 0 (rearm = 1 [D #138/#179]), t == 0.0, ch 18 up, the ray passes. The burn/laser
  path is untouched (it passed as before).
- `the_warehouse_vent_latch_opens_its_hatch`: same end-of-sweep pins for #180/#184.
- `from_the_duct_top_the_hole_drops_bond_onto_the_balcony`: the t==5.0 pin replaced by the open-pose
  matrix + the wrapped playhead; the drop/walk legs unchanged and green.
- `the_pulley_winches_sweep_once_channel_0_rises` → `the_pulley_winches_do_not_sweep_on_channel_0_alone`:
  the old premise (ch 0 rising drives the winches) is the read-stand-in 109n refuted — `SS_Operate`
  0x1a9df8:46-51 returns 0 on the channel path for a channel-0 SS [M], and the winches are mask 0x2
  (the bullet bit) [D #351/#364/#377 params 2]. The rewrite keeps the suite asserting: the winches'
  mask words, and that a ch-0 raise alone leaves every playhead at rest over 40 ticks. The bullet
  start is pinned by 109n's own real-shot test (now green). `channel_0_is_a_real_channel` keeps the
  ch-0 plumbing regression.

### The scratch probes

`zz_latch_probe.rs` and `zz_ss_dump.rs` are `#[ignore]`d (kept in-tree for their data citations,
zero suite cost; run with `--ignored --nocapture`).

## Verification

- Failing-first: all six failures reproduced on the merged tree before any fix (the run above).
- The four 0c suites on the final tree (master dc44c0e4 merged): fix_0c_hatch_latches_live 3/3,
  fix_level_0c_live 4/4, fix_0c_vent2_live 3/3, fix_0c_hatch_live 2/2 (+2 pre-existing ignores).
- nf-game lib 162/162 on the final tree (my drone.rs field survived master's auto-merge).
- Full suite (every binary, `--test-threads=1`, foreground chunks — the sandbox reaps nohup):
  **1217 passed, 6 failed, 43 ignored across 311/311 binaries.** The 6: the allowed slim-ISO
  `disc_golden` trio (rule 5) + `fix_0c_pylon_live` (2) and `probe_pylon_render` (1), all four
  panicking at `wgpu request_adapter` — the known headless GPU-adapter environmental class, not a
  code path (these render tests need the main agent's GPU run).
- Checkpoints pushed: e3ca621a (the 109n merge), ba1af7e9 (the fixes), bd844cf2 (master dc44c0e4).

## Live checks for the main agent (PCSX2)

1. **The winch shot** (0c, the storage room pulley): shoot `obj_stor1_pully02` #351 with any gun —
   gdbwatch the SS's ext+0x22 (state) and ext+0x00 (playhead): the armed 0x40 → 1 on the hit tick,
   the playhead stepping (count −1)/rate = 3/75 per tick, wrapping to 0 with the state 0x40 (rearm 0)
   after ~100 ticks, the POSE holding the last key.
2. **The bullet contact list**: gdbwatch the winch obj+0xD0 through the shot — the class-5 (bullet)
   entry appears on the tick the round dies on the pulley, including the killing hit.
3. **The four vent hinges** (0c display room, from the walkway): all four `hingelock` latches
   #139-142 visible; from inside the lower display room the same four.
4. **The vent end state**: after burning all four hinges, the hatch sits OPEN (its playhead back at 0)
   and stays open indefinitely (dead state, rearm 1) — not snapping shut after ~1 s.

## nf-view render spots (`--bond`)

- `--bond 5.8,15.8,-107.5,3.14,0` (the walkway): the display hatch's four hinge latches visible.
- `--bond 5.92,16.4,-112.9,3.14,-0.65` looking at the storage pulley: the winch, then after a shot
  the sweep runs and the platform rises; Bond standing on it rides.

## Shared files touched

- `crates/nf-game/src/drone.rs` (task file): the `ss_bullet_contacts` field (own line) + the
  recording in `update_player_bullets` + the merge in the SS block. Additive; no existing field or
  arm repurposed.
- `crates/nf-view/src/scene.rs`, `gpu.rs` — untouched by me (109n's merged changes stand as merged).

## What's left

Nothing from the task list. Open observations carried in the tests' comments: 109n's riding infra
works (the ride assert runs); the drive-level question of WHICH weapon the player is meant to shoot
the pulleys with is out of scope (any bullet works — mask 0x2).
