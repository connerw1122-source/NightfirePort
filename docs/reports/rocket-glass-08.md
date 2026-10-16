# rocket-glass-08 — the 08 corridor rocket stops ~5 units short

Task 721, branch `s/rocket-glass-08` (continuation; sandbox restart wiped the first session after
the worklog commit — this session re-ran setup steps 1/3/4, merged `origin/master` 170bcf01, and
did the whole investigation).

## What was wrong

The side-by-side (main agent, 2026-10-16) had the PS2 rocket exploding ~24 units down the corridor
(~72 frames at the measured 0.3367 u/frame [L]) while the port exploded it at 19.25 units after 56
ticks — on the far window.

A probe through the real input path (`load_weapons("07000008.bin")`, equip 0x2d, fire from the level
start with the default facing) walked the flight ray and found what the port's rocket hits:

- the aim ray's **first hit is the far window's pane, 19.62 u from the spawn, surface byte 0x10
  (glass material)** — no 0x40/0x80 bits, node pass clear (the port's own loader, i.e. the disc's
  collision data [D 07000008]);
- behind it, the **plain wall (surface byte 0x0a) at 22.87 u** — the plane z −100.0.

The port's per-hit pass walk (`flush_world_impacts` → `penetration_roll`) only knew the node pass
flag and the surface byte's 0x40/0x80 bits, so the round stopped on the pane. **The game's
`Bullet_CollisionHandler` (0x12ce80) has a glass rule the port lacked** [M disasm 0x12cf44..0x12cf64]:

```c
if (!(weapon_data+0x6c & 0x800) && (HITDATA+0x50 & 0x3f) == 0x10) pass = 1;
```

Material index 0x10 lets every bullet through, with no RNG draw. Two things the decompile hides
that the disasm settles:

- The gate object is the **shooter's weapon record**, not a level object: the decompiler's
  `*(pauVar3[4] + 4)` is `*(BU+0x44)`, and its fields are the weapon's — +2 is the weapon type word
  (0x50/0x51 = the Q-Claw), +0x6c/+0x70 the weapon flags (the Laser 0x4E's +0x6c = 0x900 [L
  ee_start_05.bin] carries the 0x800 gate, so the beam burns the pane instead of passing).
- The pane-pass is checked **before** the Q-Claw's type-word check in the game's order, so even the
  grapple bolt flies through glass; the surface-byte 0x40/0x80 rules are what the Q-Claw skips.

With the rule in place the walk pierces the pane (no RNG draw) and the rocket flies on ~10 more
ticks to the wall face — the blast moved 19.25 → ~22.9 u, matching the PS2's ~24 u / ~72 frames
within the task's ±1. The explode arms are gated on the contact not being a pierce [M
0x12d068..0x12d188], so no blast happens at the glass.

## The fix

- `crates/nf-game/src/effect.rs` — `penetration_roll` / `penetration_roll_node` take the shooter's
  `weapon_data+0x6c` word and apply the glass rule in the game's order (before the 0x50/0x51
  early-out). No RNG drawn; the 0x80 roll stays the only draw. Comment cites the disasm range.
- `crates/nf-game/src/drone.rs` — both callers pass the weapon's +0x6c: the player-bullet world-hit
  walk (`flush_world_impacts`, which already loaded the word for the laser check — hoisted above
  the roll) and the drone-bullet walk.
- `crates/nf-view/tests/impact_remainder_live.rs` — the existing roll tests updated for the new
  parameter (assertions unchanged).

Master did **not** already behave like the game (the pre-fix probe explodes at the pane), so this is
a fix, not a pin; the repro test below fails on the old behaviour (it asserts the blast is past the
pane plane, z < −98.5, while the old tree stops at z −96.75).

## How it was verified

- `rocket_glass08_live` (new, 3 tests, all green):
  - `glass_passes_unless_the_weapon_burns_it` — the unit rule: material 0x10 pierces for 0x2d
    (+0x6c = 0x1a200 [L]) with **no RNG draw**; the Laser's 0x900 keeps the pane; a plain 0x0a wall
    still stops the round.
  - `the_08_corridor_rocket_explodes_past_the_window` — the task's shot: the rocket crosses the
    pane plane and explodes on the wall face z −100.0 (±0.5), flight 21.9–24.4 u; the surface under
    the blast is the 0x0a wall, not the pane. Fails on the pre-fix tree.
  - `a_shot_at_a_plain_wall_stops_on_its_face` — sister: a floor shot still stops on the aimed
    surface's face (precondition asserts the aim surface is plain first).
- Full suite (`tools/test/suite.py`, final tree = branch merged with origin/master 170bcf01), run in
  foreground chunks (background jobs freeze between tool calls in this sandbox — the first two
  nohup/setsid attempts were killed, so the run was chunked with `--only`; every chunk re-used the
  same built tree):

  | chunk | range | result |
  |---|---|---|
  | A | blast radius: rockets, foliage, Q-Claw, grenades, determinism, 08 walks, pickups08, safe08, wallhug, sentinel | 86 passed, 0 failed, 4 ignored (1 min) |
  | B | ai..fix_world_gravity (killed at the 9-min window mid-range, no FAILED lines printed) | prefix green; range re-run in G/H/I |
  | C | fix_world_gravity..memcard | 145 passed, 0 failed (6 min) |
  | D | lib units (nf_game 395 tests) + weapons/wire/worldobj/trig/tuning/textures/subtitles | 395 passed, 0 failed, 29 ignored (2 min) |
  | E | mp/net/menus/models/movie/mission | 238 passed, 0 failed, 8 ignored (1 min) |
  | F | npc_doors..view_model_unlit (objectives, pickups, portals, progress, pss, render, save, searchlight, stun…) | 202 passed, 0 failed, 1 ignored (7 min) |
  | G | re-run ai..fix_cutscene_actors | 328 passed, 5 failed, 6 ignored (5 min) |
  | H | re-run fix_cutscene_brightness..fix_world_gravity (killed mid-range, no FAILED lines) | prefix green; tail re-run in I |
  | I | re-run fix_pickups..fix_world_gravity | 150 passed, 0 failed, 4 ignored (4 min) |
  | J | walk_*/walkreal_* smoke tests | 80 passed, 0 failed, 9 ignored (2 min) |

  Chunk totals: **1624 passed, 5 failed, 61 ignored**. The 5 failures are the documented master
  baseline: the 3 `disc_golden` tests (allowed on the slim ISO) and the 2 pre-existing
  `fix_0c_pylon_live` render probes (pylon render probes were already failing on master per the
  uncited-audit worklog). No failure touches the bullet/glass change.

  Not run in the window: ~30 scratch probe/dump binaries (`probe_*`, `walk_probe`,
  `walk_tower2_probe`, `walkreal_probe*`, `walkreal_dump_live`) — renderer/level dump probes, none
  of which exercise the bullet walk (chunk A covered the two 08 walk binaries and all
  weapon/impact live tests). `mp_lockstep` and `net_match_live` ran in chunk E.

## Live checks for the main agent

1. **The pane surface byte**: during the 08 corridor shot, watch `Intersect_RayGeom`'s write of
   `HITDATA+0x50` (0x10a8c8:326) — expect 0x10 on the pane contact (the tick the rocket crosses the
   window) and 0x0a on the wall contact ~10 ticks later.
2. **The pass decision**: watch `BU+0xF4` (the bullet's state) across the pane-crossing tick — the
   handler's branch-likely at 0x12cfc0 means state stays 1 (flying) on the pass; the port models
   this as the pierce continuation. Expected: no write of 2 at the pane tick, write of 2 at the
   wall tick, then `BU+0xF0 |= 0x10`.
3. **Q-Claw vs a window**: fire the Q-Claw (0x50) at a pane in 08 — the glass rule precedes the
   type-word check [M 0x12cf44 vs 0x12cf68], so the bolt should pass the pane (the port now does
   too). Confirm on hardware; the port's Q-Claw tests only pin non-glass latches.
4. **The laser at a window**: the Laser 0x4E (+0x6c = 0x900) must NOT pass the pane — the 0x800
   gate. A quick PCSX2 shot at any window confirms the beam stops/burns instead of flying through.

## What's left

- Nothing functional on the task's statement. The walk's object-hit rules (`obj+0xf0 & 0x60`, the
  shooter skip, class-0x28 windows vs `weapon_data+0x70 & 0x100`) remain unmodelled — the port's
  world walk sees world hits only, and the port handles object contacts through its own
  prop/drone paths; noted in the code comment.
- The ~30 scratch probe binaries listed above, if the driver wants a single-line full-suite total.

## Usage

Token, context-window and cost figures for this session: not visible to me. The environment shows
no per-session token or cost counters I can read; the chat UI is not accessible from the sandbox.
