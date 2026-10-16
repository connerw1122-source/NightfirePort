# grenade-rattle2 — the grenade bounce work, without the broken bounce-cap test

Task 693's `s/grenade-rattle` was HELD at merge: with it alone on master its own unit test
`grenade_sits_after_max_bounces_and_explodes_only_at_the_fuse` failed (grenade.rs:556 — at the
bounce max the port kept the velocity, left `[0.0, -30.002722, 0.0]` vs the test's
`[0.0, 0.0, 0.0]`), and the merge batch listed
`fix_grenade_types_live::the_stun_grenade_whites_bond_out_at_the_games_time_and_alpha` and
`fix_weapon_fire_modes_live::grenade_events_match_the_records` as failing too.

## What was done

- Started from `origin/s/grenade-rattle` (e6bc9124) merged with `origin/master` (a4156da0);
  the WORKLOG conflict resolved by keeping both sides' entries appended.
- **The disassembly settles the task's question: the game ZEROES the speed at the 20-bounce
  cap — the old test was right.** The gate is `sltiu $v1, $v1, 0x14; beqz -> 0x12d6a0`
  [M Bullet_CollisionHandler 0x12ce80, disasm 0x12d578-0x12d57c; the counter store
  `sh $v0, 0x100($s0)` in the delay slot]. The branch target is NOT the tail's state write —
  it is the cap ARM: `Mat_Align2Dir(obj+0x90, velocity)` then `sw $zero, 0xf8($s0)` —
  BU+0xF8 (the speed scalar) = 0 [M 0x12d6a0-0x12d6bc], falling into the state write
  `sh $s7, 0xf4($s3)` at 0x12d6c0. A scan of every branch in the function
  (0x12ce80-0x12d8a0) finds exactly ONE entry into 0x12d6a0: that beqz — the arm runs only at
  `(counter+1) >= 0x14`, with NO dot gate and NO reflect/scale. grenade-rattle's reading ("the
  cap gates the whole arm — the handler parks and counts but does NOT reflect or scale") got
  the no-reflect half right and missed the zero half: the decomp's `else` arm
  (decomp lines 481-485) is exactly this, and the function's own doc comment already said
  "the `(ushort)(count+1) < 0x14` else-branch likewise [zeroes]".
- Fixed `crates/nf-game/src/grenade.rs` (commit 4d54f7e8): the cap arm restores
  velocity = 0 + `resting = true` (the port's model of the game's zero-speed bullet —
  `Bullet_update`'s re-normalise pins the move at zero [M 0x12f020-0x12f030], so a rested
  grenade never re-collides and never sounds again; the fuse keeps counting and explodes at
  the fuse). The rattle's DOT-GATED reflect stays for the sub-cap arm (dot > 0 -> the gate
  skips reflect, scale AND zero [M 0x12d5f8 `bc1f 0x12d6c0`]); the slow-bounce zero tail is
  unchanged [M 0x12d674 `bc1f 0x12d6c0` -> 0x12d67c-0x12d69c: Mat_Align2Dir +
  `swc1 $f20, 0xf8($s0)` = 0.0].
- The two merge-batch live failures PASS on the merged tree with no further change:
  `the_stun_grenade_whites_bond_out_at_the_games_time_and_alpha` and
  `grenade_events_match_the_records` both green — verified before the cap fix on the bare
  merge and again after (the cap fix touches only the 20-bounce arm, which those tests never
  reach). The failures belonged to the unmerged batch tree, not to anything in this task.

## How it was verified

- `cargo test --offline --release -p nf-game --lib grenade` — 12/12, including the previously
  failing `grenade_sits_after_max_bounces_and_explodes_only_at_the_fuse`.
- The crates the tests touch, before pushing: `grenade_rattle_live` 4/4 and `grenade_live` 2/2
  (nf-game); `fix_grenade_types_live` 3 passed + 1 ignored (the pre-existing 107a ignore) and
  `fix_weapon_fire_modes_live` 5/5 (nf-view).
- **FULL SUITE on the final tree** (origin/master merged in; all 423 test binaries of the four
  crates, `--test-threads=1`, per-binary limit 600 s, NF_SILENT=1 — the fast runner's exact
  semantics, run in foreground chunks with a resumable runner because this sandbox kills
  background jobs between tool calls; every binary ran):

  **`1866 passed, 6 failed, 72 ignored`**

  The 6 failures are the known-allowed baseline, none from this task:
  - `disc_golden` — 3 failed: the slim-ISO sector-LBA differences (the file-content hashes
    match; only the LBA columns differ) — the 3 disc_golden failures the slim ISO allows.
  - `fix_0c_pylon_live` (2) and `probe_pylon_render` (1) — `wgpu adapter: NotFound`: this
    sandbox has no GPU. The same 3 pylon render probes the ladder-02 final suite documented
    as pre-existing environment failures.

## What's left

- Nothing blocking for this task. grenade-rattle's pulled-back 1 u look-ahead (the two-form
  hittest) is still parked pending the game's own landing measurement; the old report's
  live checks and the re-pin note stand.

## Live checks for the main agent

- Confirm the cap arm from the live side: in 07000005 play, poke BU+0x100 (the bounce
  counter) = 0x12 on an in-flight frag grenade (find the bullet obj via the owner walk as in
  prior grenade probes), let it contact the floor twice more, and watch BU+0x100 (u16) and
  BU+0xF8 (f32) with gdbwatch on writes. Expected: at the 20th contact (counter+1 = 0x14) the
  handler writes BU+0xF8 = 0.0 from Bullet_CollisionHandler 0x12d6bc (NOT the slow-bounce tail
  at 0x12d69c), obj+0xF4 stays 1, the grenade sits without further movement or bounce sounds
  until the fuse blast (Explo_Create from the fuse arm).
