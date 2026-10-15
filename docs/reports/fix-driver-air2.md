# fix-driver-air2 — report

Branch `s/fix-driver-air2`, cut from `origin/master` ca2d63f5, then merged `origin/s/fix-driver-air`
(109j, unmerged) — the merge was clean in `drone.rs`; only `docs/WORKLOG.md` conflicted (both entries
kept, mine last). The task: 109j's sitting-hip + soft-settle fix regressed the merge suite's
`ai_live guard_takes_cover_like_the_game` (UnderCoverAim lasted 90, live 37) — redo the driver fix
without touching an ordinary guard's cover timing.

## What was wrong (the diagnosis, both halves evidenced)

**The failure's shape.** Reproduced on the merged tree exactly (`UnderCoverAim lasted 90 (live 37)`).
A scratch probe through the real path showed the Aim window playing **danim 178 at speed 1.0** (90
ticks) instead of lean-out **176 at 1.5** (37) — the exact flip signature the fix-anim-blend2 comment
in `drone.rs` already documents: the cover-type Rand picks the step-out type when the seeded RNG
stream shifts. 109j left guards' `hip_anim` at 1.0, so the only guard-visible change was the settle
itself: its per-tick y transients during the run-to-cover flip a y-gated check, the RNG consumption
order moves, the pick flips, UnderCoverAim runs 90 @ 1.0 instead of 37 @ 1.5.

**The decompile (which drones really run the tail — the task's fork, resolved).**

- `Drone_CollisionHandler` 0x140968 branches on `drone+0x23` [M 0x140968:75]: path A (`+0x23 = 1`)
  never reaches the settle tail (its `Drone_FeetOnPoint` call passes no correction out-param and the
  tail sits behind the early returns); path B (`+0x23 = 0`) runs it. `NDrone2_DefaultInit` 0x14b300
  sets `+0x23 = 1` only for skins 0x05000078/0x05000094/0x050000ba and dtypes 0x1b/0x1c
  (the remapped `+0xc5`), and `NDrone2_DSTATE_AbseilSlide` toggles it — so **the settle is not
  driver-only in the game**: the driver's dtypenew 0x17 and the ordinary guards are all path B.
  The task's second fork applies: the game's settle genuinely runs for guards, and the live trace's
  37 must be explained by the gate.
- The tail's gate is **not** the correction. The disassembly (0x140c44-60) builds
  `c2 = hit+0x40 − drone+0x400` and compares it against −0.2 / 0 (the 0x8 window) and −0.1
  (0x140e30-48, `$f20 ← sp+0x14`), while the ×0.125 feed is `Drone_FeetOnPoint` 0x140660's correction
  out-param (0x140e50, `sp+0x50` — written through the pointer the handler passed at the path-B call).
- `NDrone2_Collision` 0x140e90 writes `drone+0x400 = obj.pos + up·(h − hip)` with h = 0.4
  (0.55 when `+0xd8 == 0xc`; 0.3 when `+0x2f ≠ 0` — never written by DefaultInit, so the default 0.4).
  Substituting: `c2 = 0.4 − hip − correction`, so the gate `c2 > −0.1` is
  **`correction < 0.5 − hip`**.
- Through the ANIMATED hip (`DroneAnim_PreTransCallback` 0x13b490 rewrites `+0xA0` per resolve as
  `clamp(animObj+0x5C − 0.02, 0.2)`) that gate is the game's own driver-only gating: the sit's 0.2
  settles the whole on-surface band (correction < 0.3 — the trace's ×0.125 decay [L 05-truck.trace
  39.305 → 39.19 → 39.2]), while the standing hip 1.0 settles only the deep-sink band
  (correction < −0.5, a 0.1-wide slice of the down-ray's [−0.6, 0.2] reach) and is otherwise NOT
  re-adjusted — the game owns the standing y through the animated hip, which the port implements only
  for the sit (the `+0x60` addend is still unread — 109j's residual 1). 109j's port gate
  (`correction > −0.1`) settled every standing drone's ±correction at ×0.125 — y transients the game
  never produces.

## The fix (`crates/nf-game/src/drone.rs`, the only source file)

`collision_handler`'s on-surface arm now gates the settle exactly as the game does:
`correction < 0.5 − hip` → `pos.y −= correction × 0.125` [M 0x140968 @0x140c44-60 + 0x140e30-48, the
probe 0x140e90]; the un-settling band keeps the port's snap (`pos.y = floor + hip`) — the documented
stand-in for the unported animated hip (without it a no-op would float standing drones up to 0.2 and
the traces pin the snap's heights). Consequences:

- The driver's sit is unchanged: hip 0.2 settles the whole band, converging to the crate top + 0.2
  (the trace's decay) — the user's "sitting in the air" stays fixed.
- Standing drones (guards, the standing driver) are bit-identical to master's snap whenever
  correction ∈ [−0.5, 0.2] — the whole normal walking band; the cover RNG stream and the 31/37 cycle
  are restored.
- The deep-sink band (correction ∈ [−0.6, −0.5)) now rises at ×0.125 — the game's own deep path,
  which replaces 109j's `≤ −0.1` snap fallback (whose stated rationale, "the deep-correction path is
  unread", is now read).

`hip_anim` itself is untouched (109j's field, still set only by `TruckDriverInit` and restored on its
exits). No new fields; no scene.rs/player.rs/lib.rs changes.

## The verification

- **Fail-first**: the merged pre-fix tree failed `ai_live guard_takes_cover_like_the_game` with the
  merge suite's exact panic (`UnderCoverAim lasted 90 (live 37)`, reproduced this session at
  crates/nf-game/tests/ai_live.rs:249). Post-fix it passes.
- `drone_stands_on_the_floor` (nf-game lib, re-pinned): the standing case takes the snap at
  correction 0.1; **sister** — the sit's hip 0.2 settles one tick by correction × 0.125 exactly;
  **sister** — the deep-sink case (correction −0.55) rises by 0.55 × 0.125 exactly.
- `fix_driver_air_live` 2/2: test 1 unchanged (the sit rests at 39.2 ± 0.05, holds x, steps into
  0x34); test 2 re-pinned to the corrected gate — the **sitting** driver lifted 0.15 mid-sit moves
  exactly correction × 0.125 on the next step_bond tick (the trace's rate [L]), and the **standing**
  driver (state 0x34, hip 1.0) lifted 0.15 is NOT settled (the gate needs correction < −0.5) and
  takes the snap. (109j's version pinned the standing driver settling — an over-broad reading of the
  tail that its own test comment carried; the corrected pin documents the hip-dependence instead.)
- Sisters, all green on the fixed tree: `fix_06_driver_live` 2/2, `fix_drone3_live` 5/5,
  `fix_truck_live` 2/2, `ai_traces_live` 4/4, `facial3_live` 3/3.

## Full suite

The fast runner (`tools/test/suite.py`) was started on the final tree after the fix landed; see the
WORKLOG for the total line if it finished. (It could not be started "in the first minutes" as the
card asks — reproducing and reading the decompile came first, and a run on the pre-fix tree would
have been wasted.) On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the
retail image) may fail.

## What's left

1. The full-suite total line (in the WORKLOG if the run finished inside the window; otherwise the
   next session: `tail -1 suite_air2.out`).
2. 109j's residuals carry over unchanged: the animated hip's per-anim rule (the `+0x60` addend —
   with it the port could drop the snap stand-in entirely), the cower's pos.y semantics, the sit's
   ≤ 0.03 x/z drift, the driver's post-script walk.
3. The deep-sink rise (correction ∈ [−0.6, −0.5)) is the game's behavior per the disassembly but is
   newly exercised in the port — the full suite is the check for any spawn that used to rely on the
   old instant recovery.

## Shared files

None beyond the task's two files: `crates/nf-game/src/drone.rs` (the settle gate + the re-pinned
unit test) and `crates/nf-view/tests/fix_driver_air_live.rs` (test 2 re-pinned). A scratch probe test
file was used for the diagnosis and deleted before commit.

## Live checks for the main agent

1. **The settle gate (gdbwatch.py)** — watch `obj+0x34` (y), `drone+0xA0` (hip) and the anim object's
   `+0x5C`/`+0x60` for (a) the 06 driver through the sit (expected [L-derived]: y decays ×0.125 to
   crate+0.2 with +0xA0 = 0.2) and (b) a walking 05 guard on flat floor (expected: +0xA0 ≈ 1.02-1.17
   and **no per-tick y re-adjustment** — the gate `correction < 0.5 − 1.0 = −0.5` should never fire
   during normal walking; confirm by watching y hold steady between gravity events). This directly
   validates the ported gate's core claim.
2. **The probe constant** — confirm `drone+0x400` holds `obj.pos + up·(0.4 − hip)` during play
   ([M 0x140e90]; 0.55/0.3 variants per `+0xd8`/`+0x2f`). A one-shot watch right after a level load
   suffices.
3. **The deep-sink band** — poke a standing guard's y to floor + 0.45 (correction −0.55): expect the
   game to raise him ×0.125 per tick (≈ 8 ticks to recover), not snap.

## Renders for the main agent

The visible behavior is unchanged from 109j (the sit renders identically; standing drones are
bit-identical to master). 109j's report's `--bond` spots still apply: `--bond 62.0,40.6,36.5` yaw 3.6
(driver seated on the crate, pelvis ≈ 39.2), `--bond 64.5,41.0,24.0` yaw 0.4 (the cab view), and the
same spot at `--pull 1000` (standing at the standing height after the script).
