# 103l2-fix-fire-modes3b — the weapon-sound sweep fails on the merge

Branch `s/103l2-fix-fire-modes3b` = `s/fix-fire-modes3` (aca97652, the four fire-mode fixes) merged with
`origin/master` (7785582a, later 8bcda967 re-merged ed718f81). **The failing behaviour is game-correct; the
fix is test-only** (`crates/nf-view/tests/fix_weapon_sounds_live.rs`). No port code changed.

## The regression, and which of the four changes moves it

**Change #2 — the quarter-speed spawn (`Bullet_init` 0x12c5a0:188-190, `+0x6C & 0x200` → speed × 0.25).**
The chain, reproduced and instrumented (NF_GUNTRACE + a per-weapon clip/bullet/camera sweep log):

1. The sweep fires the AT-420 Sentinel's guided rocket (0x2c) at its segment. Its record `+0x6C =
   0x12204` [L ee_start_05.bin @ 0x2bf150] carries BOTH the camera bit (0x4) and the quarter-speed bit
   (0x200) — it is the only camera-bit carrier among the weapons [L same sweep; the port's camera handover
   gates on `+0x6C & 4` [M `Bullet_init` 0x12c5a0:248-253 → `Player_ChangeSubState(owner, 10)`]].
2. Substate 10 runs no weapon arms and the port (like the game) keeps the trigger gated until the missile
   dies and a 15-tick restore elapses [M `Bullet_update` 0x12e660:28 `+0x944 = 0xf`; `Player_Update`
   0x193600:242-256; the port's gate: `trigger && missile_cam.is_none()`].
3. Before the branch, the rocket was modelled at the raw `+0x20` speed (0.25 [L]) and impacted within its
   own segment, so the camera cleared and every later weapon's fire window worked. With the branch's
   quarter speed the rocket flies its true 0.0625 u/tick over its 200-unit range [L +0x1C] — ≈3200 ticks
   of camera ride. The per-weapon log shows `fired=false` for **0x2e, 0x2f, 0x30, 0x32, 0x34, 0x35, 0x36,
   0x37** (8 segments; the leftover `player_bullets` entry through the ride IS the guided rocket; it dies
   at 0x37's t15, the camera clears ~15 ticks later).
4. The mine (0x37) therefore never fires, and its detonation sound **0x458 (1112)** never starts a voice
   anywhere in the sweep (`seen` accumulates across all weapons, so a late blast would still count — the
   sound genuinely never happens). Weapon 0x37 fails its only missing id; weapon 0x59's ids [0x448, 0x458]
   pass on 0x458 via the accumulated `seen` and fail with it. Isolation probes (fresh scene per weapon)
   confirm every path works alone on the merged tree: 0x37 → 0x458 (slot 2), 0x3b → 0x448, 0x59 → 0x458.

**Verdict: the new behaviour is game-correct.** A player who fires the guided Sentinel rides the missile
camera for the missile's true (slow) flight and cannot fire until it resolves — the game's own substate-10
rules. The other three changes are not involved: the mine's flags (`0x6C = 0x8c08`, `0x70 = 0x18300` [L])
carry neither the 0x200 speed bit nor the 0x100000/0x200000 blast arms; the zoom reset and the Samurai
drain don't touch these weapons.

## The fix (test-only)

`every_weapon_on_07000005_draws_fires_and_reloads_with_sounds` now **waits out the missile camera before
each weapon's fire window** (bounded at 3400 ticks = the ≈3200-tick flight + the 15-tick restore + margin;
each comment cites the addresses above). The range death is the silent state-4 death [M `Bullet_update`
0x12e848], so the wait adds no blast near Bond. The window itself stays 25 ticks; no ids were dropped.

Documented while there (not a regression — true on `origin/master` too, verified by running the same
instrumented sweep on a master worktree): **0x448 (the satchel family's detonation sound) has never been
observed by this sweep on any tree.** The charge spawns at its fire script's RELEASE frame (`+0x44 = 76`
[L record]; `Player_SetWeaponAnimObj` case 9 0x1a2e38:416-420), 51 ticks past the 25-tick window, and the
next weapon's holster cuts the script before it. Holding the sweep to the release would land the
8-radius/2000-damage charge ~5 units from Bond — `Explode_Propagate` 0x17c518 → the player target →
`Player_Hurt` [M drone.rs `spawn_blast`] — a game-faithful self-kill that voids the rest of the sweep.
0x3b's row passes on 0x447 (the plant sound); 0x59's row passes on 0x458 (the mine's). This is written
into the test's `MISSING_ON_05` doc comment.

## Verification

- RED first: the merged tree failed exactly `0x37: none of its missing sounds [1112] ever played;
  0x59: none of its missing sounds [1096, 1112] ever played` (11 s sweep, reproduced twice).
- GREEN after the fix: `fix_weapon_sounds_live` 3 passed / 2 ignored (both ignores pre-existing and
  documented in their comments); re-verified green after re-merging `origin/master` (ed718f81, the
  fx-levels grenade-tick changes).
- Neighbours: `fire_modes3_live` 5 passed / 4 ignored (pre-existing), `fix_weapon_fire_modes_live`
  5 passed, `weapon_fx_levels_live` 2 passed / 2 ignored (pre-existing).

## The suite total (tools/test/suite.py loop on the final tree)

The sandbox kills background jobs between tool calls, so the run went through `tools/test/suite_chunk.py`
(the same per-binary loop, foreground chunks; log `/home/z/suite.log`). **267 of 268 test binaries ran:
TOTAL 1063 passed, 6 failed, 39 ignored.** The 6 failures, none caused by this branch:

- 3 × `disc_golden` — the allowed slim-ISO failures (no full ISO).
- `probe_pylon_render::probe_render_pylon_only` and `fix_0c_pylon_live` (2 tests) — verified failing on
  `origin/master` (ran both on a master worktree at 7785582a): pre-existing, not this branch's.

Two binaries hit the chunk's per-binary wall-clock limit and are **not counted above** (both are slow
level walks/sweeps, not hangs — the same binaries passed in the merge-hold baseline):
`weapon_matrix_live` (needs > 300 s) and `walkreal_07000001_live` (needs > 440 s; its
`walk_from_the_spawn_to_the_exit_with_the_contact` was still running when the limit hit). Re-run each with
the canonical 600 s limit for the exact 268-binary total.

## Shared files

None. The only changed files are `crates/nf-view/tests/fix_weapon_sounds_live.rs` (the cam wait + the
0x448 note) and `docs/WORKLOG.md` (this entry). The scratch probes (`zz_*.rs`, `suite_chunk.py`) are
untracked and not committed.

## Left for the next message

1. Re-run `weapon_matrix_live` and `walkreal_07000001_live` with the 600 s limit for the full 268-binary
   total (expected +N passed; neither is touched by this branch).
2. The 0x448 blind spot: observing the satchel detonation through gameplay needs either a movement input
   in the sweep (walk Bond out of the 8-unit radius after the release) or the main agent's live check.
3. The 07000001 foreign-fire gap (`a_foreign_weapon_fires_with_sounds_on_the_indoor_level_07000001`,
   ignored) is still 100a/100c's.

## Live checks for the main agent

PCSX2 (07000005, give the Sentinel 0x2c via the dev menu, fire once into the courtyard):
- Expected: the view hands to the rocket (substate 10) and rides it for a LONG flight (the true quarter
  speed, 0.0625 u/tick) — several seconds over the courtyard — during which the trigger is dead; the view
  restores ~15 ticks (0.25 s) after the rocket dies [M 0x12e660:28, 0x193600:242-256]. If the real game's
  ride is much SHORTER than ≈3200 ticks (e.g. the rocket impacts or dies sooner), the port's flight-time
  model needs a look; the camera-wait bound (3400) would still hold.
- Then give the Remote Mine 0x37 and fire: the mine's detonation sound plays (0x458 — bank 5 lacks it; on
  real hardware the level bank would miss it too — this is the dev-give scenario the sweep models).

nf-view renders (nothing visual changed; no --bond spots required by this task).
