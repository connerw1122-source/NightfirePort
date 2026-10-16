# oddjob-hat — report

**Task:** the three Oddjob's-hat playtest items — the hit damage ("should be a one-shot kill but
it acts like a grenade"), the wall-hit sound ("weird sound when it hits a wall"), and the
projectile pool ("if you throw enough of them, you stop seeing them flying").

## What the disc says (all independently re-verified this session)

The hat is weapon 0x45. Its record [L ee_start_05.bin @0x2c398c, re-read byte-for-byte this
session]: **damage +0x0C = 100.0** (one touch kills any 100-health guard), speed +0x20 = 0.6667,
range +0x1C = 1000, fire interval +0x40 = 26, throw release frame +0x44 = 13, rounds per throw
+0x91 = 100, flags +0x6C = 0x20C08 (gravity, the HOMING bit, no flesh pass-through), flags
+0x70 = 0x100 (the stick — no fuse, no blast bits: **the hat never explodes**).

- **Damage — one-shot, not a grenade:** a character hit takes the standard bullet path
  (`Bullet_CollisionHandler` 0x12ce80, the damage → msg 8 arm) at the record's full 100.0; the
  hat has NO blast (the +0x70 fuse/blast bits are clear, `Bullet_Delete`'s effect kind +0x70 & 7
  = 0 spawns nothing [M 0x12e7b0:363-371, 0x12ce80:315]).
- **The wall sound:** the +0x100 stick arm plays exactly ONE sound — `Sound_Play3D(0x4a7, obj+0x30,
  96.0, −1.0)` — visible in the decompile at 0x12ce80:315-317 (the hat writes the 0x44160000 =
  1500.0 post-stick lifetime right next to it). No per-material impact sound (the hat's
  `Effect_Create` flags are 0 — the 0x40-flag arm of `Effect_Bullet` 0x1ce8a8:86-92 never runs),
  no 0x51c bounce (the bounce block LAB_0012d4bc needs +0x70 & 0x80 or a char hit).
- **The pool:** the sim keeps every hat until its own expiry — a stuck hat holds its 1500-tick
  lifetime and nothing recycles it early. The only placed-cap in `Bullet_CollisionHandler` counts
  the SATCHEL 0x3a / MINE 0x37 same-weapdef set (6 max, the oldest marked +0xFE |= 1
  [M 0x12ce80:335-370, 0x12ce80:296-330]) — the hat 0x45 is not part of it. Every flying or stuck
  hat draws [M `Bullet_update` state 0 → `hashtable_set_object_to_entity_gfx` 0x12e8f0-0x12e900]
  through the per-view 512-entry draw ring [M `View_AddObjects` 0x1e62d8
  `cursor = (cursor+1) & (size-1)`; [L ee_snapshot.bin viewer[0] @0x1fc6c10 sizes 512/512/128] —
  a 10-hat spree never comes near the ring's reuse.

## What's done

All three legs are implemented on master (the merged fix-oddjob-hat and fix-hat-details work) and
this task **verified each green on this branch through the real input path** (`nf_view::load_weapons`
+ `Sim::step_bond`), then closed the one coverage gap against the task's exact test spec:

1. **One-shot kill** — `oddjob_hat_live::the_thrown_hat_kills_the_guard_in_one_hit_and_never_bounces`
   (2.7k+ ticks of flight: the guard dies to the touch, the 0x51c grenade bounce is never heard,
   no explosion ever spawns, nobody beside the victim takes damage) and
   `gadgets_live::oddjobs_hat_kills_on_contact_and_never_explodes`. Both **green**.
2. **The wall sound** — `hat_details_live::the_hats_wall_contact_is_the_single_game_clang`: at the
   stick tick, exactly one 0x4a7 clang at the contact point and nothing else. **Green**.
   `oddjob_hat_live::the_hat_sticks_where_it_lands_clangs_once_and_kills_no_more` adds: the hat
   freezes, never bounces, kills no more (a guard moved onto it survives), expires silently at
   1500 ticks, and the frag 0x34's 0x51c still plays (the per-record routing). **Green**.
3. **The pool / "stop seeing them flying"** — `hat_details_live::every_thrown_hat_still_draws_while_it_flies`
   (12 thrown hats, all drawing) **green**, and **NEW this task**:
   `oddjob_hat_spree_live::ten_rapid_throws_keep_the_games_count_of_live_hats` — the task's exact
   ask. Ten press-release throws at the throw cadence (the record's 26-tick interval floor; the
   throw anim spans a press cycle, so the measured rapid cadence is a regular 52 ticks per hat —
   the assert rejects starvation, not the cadence): **10/10 hats stay live in the sim** (nothing
   recycles the oldest below the game's count), **10/10 hat-mesh instances draw** at the hats'
   positions, and the count **still 10 after 300 more ticks** (well inside the stuck hats'
   1500-tick lifetime). **Green** (2.3 s).

No code changes were needed: the playtests were filed against a tree older than the merged
fix-oddjob-hat/fix-hat-details work, and this recheck confirms the merged behaviour against the
disc evidence. The new file is test-only (`crates/nf-view/tests/oddjob_hat_spree_live.rs`).

## What ran

- `cargo test -p nf-view --test oddjob_hat_spree_live` — 1/1 green.
- `cargo test -p nf-view --test oddjob_hat_live --test hat_details_live --test gadgets_live`
  — 2/2, 2/2, 4/4 green.
- The full fast suite (`tools/test/suite.py`) on this branch (this tree = origin/master + one
  docs commit + the test-only spree file; the build pre-dates the spree file, which was then run
  standalone): total line and failures recorded below.

## What's left

Nothing for the three playtest items. If the main agent wants one more proof: a render pass at a
stuck-hat spot (the spree test prints the hat positions) would show ten hats on the wall.

## Suite (final tree)

The full fast suite (`tools/test/suite.py`) on this branch, run to completion:

```
1835 passed, 6 failed, 72 ignored  (43 min)
```

The 6: the 3 allowed `disc_golden` slim-ISO failures, plus `probe_pylon_render::
probe_render_pylon_only` and the 2 `fix_0c_pylon_live` pylon-render tests — the same set the
cross-process-determinism session's suite showed on the unmodified merged tree before any of this
branch's files existed; pre-existing here, out of this task's scope.

The task's own binaries through the runner on the final tree (`--only
oddjob_hat_spree_live,oddjob_hat_live,hat_details_live,gadgets_live`): **9 passed, 0 failed**.

## Live checks for the main agent

None required — the three legs are covered by deterministic sim tests. Optional: render
`07000005.bin` at a stuck-hat spot after the spree test's 10 throws (`--bond` at the thrown hats'
printed positions) to eyeball ten hats stuck on the floor ahead of Bond.
