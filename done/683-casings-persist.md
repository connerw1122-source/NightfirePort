## Your task: `casings-persist` — what spent casings do after landing (and an optional keep setting)

Playtest 1015-1738: "Do you think you can keep the bullet casings as like physical objects ... Maybe like keeps a
certain amount." First port exactly what the GAME does after a casing lands: its rest state, its lifetime, the
fade, the 32-casing pool's recycling order [M Bullet_InitCasingLists 0x12fd48, the casing update / deletion,
`nf_game::casing`]. Then add a PC-port option (default = the game's behaviour), "Keep shell casings: N" (0, 32,
64, 128), which keeps the newest N resting casings, still recycling oldest-first. Deterministic: no wall-clock.
Tests: fire 40 rounds and assert the game's count and lifetime by default; with keep = 64 the resting casings are
held and the pool recycles oldest-first. Sister: casings in flight still collide and bounce the same. Data/sim
only (the casings already draw).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/casings-persist`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox1 at 2026-10-16T10:14Z

---
## Report (done, 2026-10-16T11:17Z)

Closed by the driver: the report is on `s/casings-persist` (docs/reports/casings-persist.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-16T12:09Z)


---
## Report from the worker (done, 2026-10-16T12:09Z)

# casings-persist — what spent casings do after landing (and the keep setting)

Queue 683, branch `s/casings-persist`. Playtest 1015-1738 asked to keep casings around. The brief's
framing ("rest state, lifetime, fade") needed correcting against the disc first: **the game's casings
never rest, never bounce and never fade.** Everything the game does is now ported exactly (one [S]
upgraded to [M]); the PC-port "Keep shell casings: N" option rests and holds them.

## What the game actually does [M]

- **The pool**: `Bullet_InitCasingLists` 0x12fd48 creates 32 objects — the 0x40 there is the ALLOC
  size (`control_create_object` mallocs param+0x110; the class byte is separate) — each parked state 2,
  `NextCasing` (0x30ca92) = 0. `Bullet_init_casing` 0x12fde0:16-21 wraps `NextCasing` with the ±0x20
  trick (round-robin, oldest-first) and sets the slot's class byte to **0x0d** (0x12fde0:24).
- **Class 0x0d in `control_funcs`** (0x26f890, 12 bytes per class: update/collide/aux; read from
  `work/elf/ACTION.ELF` at the symbol's address) = update 0x12faa0 (`Bullet_casing_update`),
  **collide NULL**, aux NULL. **The casings never collide with the world** — no bounce, no rest, no
  floor. The class-5 bullets carry `Bullet_CollisionHandler` 0x12ce80; a casing object never reaches
  it (`Collide_Update` 0x1ea398 dispatches `control_funcs[class*12+4]`, null for 0x0d).
- **`Bullet_casing_update` 0x12faa0 per tick** (disassembly checked; the decomp matches):
  1. age (data+0x24) += FRAME_RATE_MUL (0x12fb44-58); past 5 ticks `obj+0xf0 &= ~0x2000`
     (0x12fb5c-6c — a draw-pass flag; only the player's eject sets 0x2900, `Bullet_init_casing_ex`
     0x12f8f0-0x12f900. The port's renderer does not bucket casings by obj+0xf0 — documented
     deviation, no sim reader).
  2. If pos.y <= plane (data+0x20, tested against the PRE-integration pos, 0x12fb70-0x12fb80): play
     the material sound (the mesh/material switch 0x12fb84..0x12fbcc) and park the plane at
     **0xc9742400 = −1000000.0** (0x12fb88-0x12fb94) — the sound fires **exactly once**. The old [S]
     ("the game re-triggers the sound while below the plane every tick") is **wrong**; the port now
     stores the plane exactly like the game and derives `landed` from that store.
  3. If age > 240: state 2, `View_SetDrawInNoViews`, age = 32767.0 (0x12fc58-0x12fc94) — and the tick
     still falls through to the flight integration (0x12fc98); the port matches (no skip).
  4. pos += vel; euler (obj+0x50..0x5b) += spin (data+0x10); vel.y += WldGravity·REC² — the state-1
     arm scales gravity ×0.1 but nothing sets state 1 on a class-0x0d object: **unreachable for
     casings** (the state-1 writers are the class-5 bullet paths: `Bullet_CollisionHandler`
     0x12ce80, `Bullet_update` 0x12e7b0).
- Net: a spent casing flies, plays its landing sound once at the plane (shooter y − 1..1.5), keeps
  falling **through the floor**, and vanishes at age > 240. **No fade** (no alpha on class 0x0d).

## The keep option (PC-port)

`Casings::set_keep(n)` (crates/nf-game/src/casing.rs). **0 = the game's behaviour** — the default;
every new Sim starts there. n > 0:

- the pool grows to max(32, n) (the new slots start dead, like `Bullet_InitCasingLists`' do);
- a casing that crosses its landing plane **rests**: pos.y = the plane, vel/spin zeroed, the flight
  integration stops, and the 240-tick despawn no longer applies to it (in-flight casings still
  despawn at 240 exactly as the game's do);
- recycling is untouched: the game's NextCasing slot order continues over the extension slots — the
  newest n resting casings are held; the oldest goes when the cursor wraps.

Deterministic: tick-counted, no wall-clock, no RNG draws (the rest snaps on the landing tick — the
same tick the game plays the sound). `Casings::ejected` counts actual slot takeovers (one per
reserve/`eject_bone`; the mesh-less early returns take no slot [M 0x12fde0:13-15]) for tests and the
eventual options UI. The option surface is sim-level by scope (data/sim only): the host calls
`sim.drones.casings.set_keep(n)` after a scene load (a new Sim starts at 0).

## The draw side (one line, so the option is visible)

The Effect layer's casing attachment pool sizes to `KEEP_MAX` (128) instead of `POOL` (32)
(crates/nf-view/src/scene.rs): the extension slots need draw slots. The per-frame pose guards (state
0 + mesh match) keep the default visual identical — at keep = 0 slots 32..127 pose `far`.

## Tests

- crates/nf-game/src/casing.rs (unit, deterministic, no ISO) — the existing 4 kept (one updated to
  the parked-plane store) plus 3 new:
  - `default_keep_forty_rounds_cap_at_32_and_all_despawn_at_240` — 40 rounds: the pool never exceeds
    32, slot 0 recycled by the 33rd shot (its age-8 signature), everything despawns at 240;
  - `keep_64_holds_the_resting_casings_and_recycles_oldest_first` — 40 held resting past 240, frozen
    on their planes; 65 rounds → 64 held, slot 0 recycled by the 65th, the recycled slot flies again;
  - `in_flight_flight_is_identical_regardless_of_keep` (**sister**) — the same seed with keep 0/64:
    bit-identical flight (pos, vel) up to each landing and the identical sound schedule; the paths
    split only at the plane (the game's casing sinks past it, the keep=64 casing rests on it); each
    casing plays its landing sound exactly once.
- crates/nf-view/tests/impacts_live.rs (live, NF_ISO 07000005, through `step_bond` — the real input
  path): `forty_rounds_default_caps_at_32_and_keeps_64_holds_the_rest` — 104 real rounds (the
  carried-ammo pool topped up so the clip/reload cycle runs, never dry — the inventory is the source
  of truth [M BLData +0x170]): with the default, alive never exceeds 32 and drains to 0 past the
  lifetime; with keep = 64, every pool slot fills once, all 64 rest and are held far past 240 (the
  firing span alone exceeds it); the 105th round recycles slot 8 — the pool's oldest resting slot —
  and the pool stays at 64.

## What ran

- `cargo test --offline --release -p nf-game --lib` — 240 passed (includes the 7 casing tests).
- `cargo test --offline --release -p nf-view --test impacts_live` — 5 passed;
  `-p nf-view --test fix_shell_eject_live` — 2 passed.
- The full four-crate suite via `tools/test/suite.py` on the final tree (master aa3bc1f5 + this
  branch, nothing else pending): **1783 passed, 6 failed, 72 ignored (43 min)**. The 6: the 3
  `disc_golden` tests the slim ISO always fails, plus `probe_pylon_render::probe_render_pylon_only`
  and `fix_0c_pylon_live::{the_0c_pylon_fills_the_view_from_the_closeup_spot,
  the_0c_pylon_is_on_screen_at_the_user_spot}` — the identical 3 pylon failures the pre-session
  baseline run shows (suite.log 2026-10-16 09:50: 1729 passed, the same 6 failed), i.e. pre-existing
  on master, not from this branch (this branch touches only the casing pool and its tests).

## Live checks for the main agent

None required for the sim (the post-landing behaviour is fully determined from the code/disassembly;
the class table is data, read from the ELF). The keep rest itself is a PC-port extra — the game has
no ground truth for it by design ([S], flagged in the module docs).

**Render check** (the rest attitude is visual work): place Bond at the 07000005 start
(`--bond` spot: the level's default start position), fire ~20 rounds with
`sim.drones.casings.set_keep(64)`, then look down at the feet: the shells should rest near the
landing plane (≈ foot level), roughly horizontal with varied yaws (deterministic per position),
one per shot, none sinking. The flying shells' look is unchanged (the in-flight reconstruction
path is untouched).

## What's left

- The front-end binding: a "Keep shell casings" row (0/32/64/128) calling
  `sim.drones.casings.set_keep(n)` after each scene load. Deliberately out of scope here (data/sim
  only, per the brief).
- If "casings in flight still collide and bounce" was meant as a required behaviour (not a regression
  guard): the disc says otherwise (control_funcs 0x26f890 class 0x0d collide = NULL; the landing
  plane is a sound-only trigger parked at −1e6). If a playtest shows PS2 shells bouncing, that is a
  new finding against the table and the next session should port `Bullet_CollisionHandler`'s bounce
  arm for class 0x0d.
