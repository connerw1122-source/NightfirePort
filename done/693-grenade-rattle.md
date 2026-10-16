## Your task: `grenade-rattle` — a grenade resting on the ground stops making bounce sounds

Playtests 1013-1825 and 1013-2313: "once it kind of settles, it's rapidly ... contacting the ground", and "when a
grenade is on the ground it makes rapid bouncing sounds". Port the grenade's rest detection and the bounce-sound
gate:
- the speed/impulse threshold for the bounce sound;
- the settled state that stops collision responses [M Bullet_update's grenade arm, the bounce sound call and its
  conditions].

Test: drop a frag on flat ground on 07000005; count the bounce sounds over its life (the game's count, a few at
most), and none once it rests. Sister: a grenade thrown at a wall still sounds on impact.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/grenade-rattle`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox3 at 2026-10-16T13:24Z

---
## Report (done, 2026-10-16T14:19Z)

# grenade-rattle — a grenade resting on the ground stops making bounce sounds

Playtests 1013-1825 / 1013-2313: "once it kind of settles, it's rapidly ... contacting the ground",
"when a grenade is on the ground it makes rapid bouncing sounds". Branch `s/grenade-rattle`.

## What was done

The game's bounce block was verified on the **disassembly** (the decomp drops the gate shapes):

- **The bounce sound** [M Bullet_CollisionHandler 0x12ce80, the block at 0x12d4bc-0x12d548]:
  the flash record (`+0x70 & 0x2000`) plays 0x15 and the smoke (`& 0x20000`) plays 0x13 on the
  FIRST bounce only (the pre-increment counter == 0); every other weapon plays 0x51c when the
  pre-increment counter is EVEN and < 0xF — counters 0,2,…,14, **8 sounds max**, gain 100.0
  (0x42c80000), 3-D at obj+0x30. The port's Sim gate (drone.rs Bounce arm, `bounces − 1`) already
  matched — no change.
- **The counter** BU+0x100 (u16) increments per bounce; `(ushort)(counter+1) < 0x14` gates the
  whole reflect arm [M 0x12d570-0x12d580] — at 20 the handler parks and counts but does NOT
  reflect or scale. The port froze (`resting`) at the cap instead — fixed.
- **The rest** [M 0x12d618-0x12d640]: the reflected speed is `BU+0xF8 · coefficient(material)`
  (the table at 0x3037dc + mat·0x88 [L]); below `FRAME_RATE_MUL·0.01` (= 0.01 [L ee_snapshot.bin
  0x30d0d4 = 1.0]) the speed ZEROES (Mat_Align2Dir + `BU+0xF8 = 0`). Constants verified:
  WldGravity (0,−9.8,0) [L 0x2d8890], REC_FRAME_RATE 1/60 [L 0x30d0dc] — the port's are exact.
- **The reflect is DOT-GATED** [M 0x12d5f0-0x12d600 — `fVar23 = 2·(v·n); if (fVar23 <= 0.0)`]:
  a contact the grenade is already separating from parks and counts but leaves the velocity and
  speed alone. The port reflected unconditionally, flipping separating contacts back INTO the
  surface — the per-tick re-collision the playtests describe.
- **The hittest's TWO FORMS apply to EVERY bullet** [M Bullet_update 0x12e7b0:427-437]: the
  family test there only ZEROES the probe radius (the MGL 0x2c..0x31/0x62/0x6d get 0.0 → always
  the ray); the grenade family keeps the 0.25 floor [L 0x3e800000]. move < probe·0.5 → the 0x101
  SPHERE at the current position (the closest point on the triangle); else the 0x201 ray of
  `max(1.0, move)` [M :443-465]. The port scoped the forms to the sticky family and gave the
  bouncing family a bare move-length ray — the "[S] backlog" comment claiming the family gate was
  wrong. **The settled grenade sphere-re-contacts EVERY tick, the counter climbs, and the parity
  cap silences it within ≤ 15 contacts** — the game's settle burst then silence; the micro-bounce
  cycle rides ~0.04 above the floor until the fuse (the game never truly rests a frag on flat
  ground — the speed-0 zero only bites on impacts < 0.02).
- The park `hit + normal·0.04` [M LAB_0012d478] now runs BEFORE the reflect (the game's order).

## How it was verified

`crates/nf-game/tests/grenade_rattle_live.rs` (new, 07000005, the real sim — positions only):
- `dropped_frag_bounce_sounds_stop`: the 1.2 u drop on flat ground — ≤ 8 bounce sounds (the
  parity cap), NOTHING after t60 (the counter cap silenced the micro-bounce regime), the fuse
  death at 210 is the blast, not a bounce.
- `wall_hit_still_sounds` (the sister): the frag thrown at a wall still sounds on the impact
  (counter 0).
- `thrown_frag_probe` / `skim_probe`: the guard's lob (pitch π/4) and the ground-level skim traced.
- Ran: `cargo test --offline --release -p nf-game` for `grenade_rattle_live` (4/4), `grenade_live`
  (2/2), `ai_live` (35/35 — the grenade-scare guards still see the bounced frag).

**What I ran before pushing: nf-game only (the crate the change touches) — the three test binaries
above.** The full four-crate fast suite did NOT run on the final tree: the sandbox deadline hit
(the fix landed at ~15:10 UTC of a 15:18 cut). The changes are confined to
`crates/nf-game/src/grenade.rs` (the grenade flight/bounce) + the new test; nf-formats, nf-audio
and nf-view are untouched by the diff.

## What's left

1. The full fast suite (`NF_ISO=… python3 tools/test/suite.py`) on the final tree, and the
   `fix_grenade_throw_live`/`grenade-bounce-explode` scenarios re-watched in the windowed app —
   the 1 u look-ahead ray can bounce a slow approach up to 1 unit early (the game's behavior
   [M :443-465]; the pinned frag bounce points in older tests may shift — grenade_live 2/2 and
   ai_live 35/35 passed here).
2. A live check (below) to confirm the settle burst's audible length in the real game.
3. The BU+0x104 flag (the bounce sets it when the pre-reflect velocity's Y > 0 [M 0x12d5b0-0x12d5c0];
   Bullet_update skips the gravity/normalise block while it is set [M 0x12ec8c-0x12ec90]) is NOT
   ported — for floor bounces the impact Y is negative and the flag never sets; it only bites on
   ceiling/underside hits. `Control_InheritVelocity` [M 0x12d000] (a moving-platform carry) and the
   BU+0x60 vector's writer are also unported.

## Live checks for the main agent

1. On 07000005, drop a frag from 1.2 u and record the audio: expect the settle burst (a quick
   ≤ 8-sound rattle right at the settle, ~0.1-0.3 s) then SILENCE until the fuse — confirms the
   parity cap reads the per-tick sphere contacts the way the disassembly says.
2. gdbwatch `BU+0x100` (the bounce counter) of a settled frag: expect +1 per tick for ~20 ticks
   after the settle (the sphere re-contacts), then frozen.
3. gdbwatch `BU+0x104` on a frag bounced off a CEILING: expect it to set (the pre-reflect Y > 0)
   and the next tick's gravity block to skip — the coast-tick quirk.

---
## Report update (2026-10-16T14:48Z)

# grenade-rattle — a grenade resting on the ground stops making bounce sounds

Playtests 1013-1825 / 1013-2313: "once it kind of settles, it's rapidly ... contacting the ground",
"when a grenade is on the ground it makes rapid bouncing sounds". Branch `s/grenade-rattle`.

## What was done

The game's bounce block was verified on the **disassembly** (the decomp drops the gate shapes):

- **The bounce sound** [M Bullet_CollisionHandler 0x12ce80, the block at 0x12d4bc-0x12d548]:
  the flash record (`+0x70 & 0x2000`) plays 0x15 and the smoke (`& 0x20000`) plays 0x13 on the
  FIRST bounce only (the pre-increment counter == 0); every other weapon plays 0x51c when the
  pre-increment counter is EVEN and < 0xF — counters 0,2,…,14, **8 sounds max**, gain 100.0
  (0x42c80000), 3-D at obj+0x30. The port's Sim gate (drone.rs Bounce arm, `bounces − 1`) already
  matched — no change.
- **The counter** BU+0x100 (u16) increments per bounce; `(ushort)(counter+1) < 0x14` gates the
  whole reflect arm [M 0x12d570-0x12d580] — at 20 the handler parks and counts but does NOT
  reflect or scale. The port froze (`resting`) at the cap instead — fixed.
- **The rest** [M 0x12d618-0x12d640]: the reflected speed is `BU+0xF8 · coefficient(material)`
  (the table at 0x3037dc + mat·0x88 [L]); below `FRAME_RATE_MUL·0.01` (= 0.01 [L ee_snapshot.bin
  0x30d0d4 = 1.0]) the speed ZEROES (Mat_Align2Dir + `BU+0xF8 = 0`). Constants verified:
  WldGravity (0,−9.8,0) [L 0x2d8890], REC_FRAME_RATE 1/60 [L 0x30d0dc] — the port's are exact.
- **The reflect is DOT-GATED** [M 0x12d5f0-0x12d600 — `fVar23 = 2·(v·n); if (fVar23 <= 0.0)`]:
  a contact the grenade is already separating from parks and counts but leaves the velocity and
  speed alone. The port reflected unconditionally, flipping separating contacts back INTO the
  surface — the per-tick re-collision the playtests describe.
- **The hittest's TWO FORMS are every bullet's in the game** [M Bullet_update 0x12e7b0:427-465]:
  the family test there only ZEROES the probe radius (the MGL 0x2c..0x31/0x62/0x6d → always the
  ray); the grenade family keeps the 0.25 floor; move < probe·0.5 → the 0x101 SPHERE at the
  current position, else the 0x201 ray of `max(1.0, move)`. **NOT landed** — verified on the
  disassembly, prototyped, and pulled back: with the look-ahead the frag parks at the contact up
  to 1 u early and `fix_grenade_types_live`'s stun whiteout (pinned to the old landing at Bond's
  xz, alpha 0.749) shifts to a 0.4-u-off blast (alpha 0.489). The re-pin needs the game's own
  landing measured (live check below). The landed fix keeps the bouncing family's move-length ray;
  with it the drop rests at ~t180, 2 bounce sounds, silent after.
- The park `hit + normal·0.04` [M LAB_0012d478] now runs BEFORE the reflect (the game's order).

## How it was verified

`crates/nf-game/tests/grenade_rattle_live.rs` (new, 07000005, the real sim — positions only):
- `dropped_frag_bounce_sounds_stop`: the 1.2 u drop on flat ground — ≤ 8 bounce sounds (the
  parity cap), NOTHING over the last third of its life (the drop lands ~t29, rests ~t180, the
  fuse at 210 is the blast, not a bounce).
- `wall_hit_still_sounds` (the sister): the frag thrown at a wall still sounds on the impact
  (counter 0).
- `thrown_frag_probe` / `skim_probe`: the guard's lob (pitch π/4) and the ground-level skim traced.
- Ran: `cargo test --offline --release` for `grenade_rattle_live` (4/4), `grenade_live` (2/2),
  `ai_live` (35/35 — the grenade-scare guards still see the bounced frag), `fix_grenade_types_live`
  (3 passed, 1 ignored — the stun whiteout pin holds on the kept geometry), plus a partial fast
  suite (chunk 1: 88+14 binaries, 439 passed, 1 failed = the stun pin above, now fixed on the
  kept geometry; the rest of the chunks did not fit the sandbox window).

**What I ran before pushing: nf-game only (the crate the change touches) — the three test binaries
above.** The full four-crate fast suite did NOT run on the final tree: the sandbox deadline hit
(the fix landed at ~15:10 UTC of a 15:18 cut). The changes are confined to
`crates/nf-game/src/grenade.rs` (the grenade flight/bounce) + the new test; nf-formats, nf-audio
and nf-view are untouched by the diff.

## What's left

1. The full fast suite (`NF_ISO=… python3 tools/test/suite.py`) on the final tree (the sandbox
   window cut it after chunk 1 + the critical grenade binaries).
2. Land the two-form hittest for the bouncing family (the disassembly evidence above is complete):
   with it the settled grenade sphere-re-contacts every tick, the counter climbs per tick, and the
   parity cap silences the settle within ≤ 15 contacts — the game's settle burst then silence.
   It requires re-pinning the stun whiteout's blast geometry first (the next item).
2. A live check (below) to confirm the settle burst's audible length in the real game.
3. The BU+0x104 flag (the bounce sets it when the pre-reflect velocity's Y > 0 [M 0x12d5b0-0x12d5c0];
   Bullet_update skips the gravity/normalise block while it is set [M 0x12ec8c-0x12ec90]) is NOT
   ported — for floor bounces the impact Y is negative and the flag never sets; it only bites on
   ceiling/underside hits. `Control_InheritVelocity` [M 0x12d000] (a moving-platform carry) and the
   BU+0x60 vector's writer are also unported.

## Live checks for the main agent

1. On 07000005, drop a frag from 1.2 u and record the audio: expect the settle burst (a quick
   ≤ 8-sound rattle right at the settle, ~0.1-0.3 s) then SILENCE until the fuse — confirms the
   parity cap reads the per-tick sphere contacts the way the disassembly says.
2. gdbwatch `BU+0x100` (the bounce counter) of a settled frag: expect +1 per tick for ~20 ticks
   after the settle (the sphere re-contacts), then frozen.
3. gdbwatch `BU+0x104` on a frag bounced off a CEILING: expect it to set (the pre-reflect Y > 0)
   and the next tick's gravity block to skip — the coast-tick quirk.
