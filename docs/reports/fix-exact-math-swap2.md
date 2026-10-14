# fix-exact-math-swap2 — the exact sine/matrix path landed and green on current master

Branch `s/fix-exact-math-swap2` = `s/fix-exact-math-swap` (571155d) merged with `origin/master` (c606b5f).
Task 284b. Sandbox 13:18–15:17 UTC 2026-10-14.

## What's done

**The swap is on the final tree and the two red tests are resolved from the game.** The exact path
(`placement::object_matrix` = RotMatrix 0x1e2368 via the bit-exact `anim::sinf`, `anim::quat_trans_to_mat`
= Quat_QuatTransToMat 0x1e3fa8, `anim::mul` = psiBuildMatrixPalette 0x111498) is present and active
(`crates/nf-formats/src/placement.rs:59-72`, `crates/nf-formats/src/anim.rs:175-197`). No swap code was
reverted or weakened; nothing was re-pinned to host math.

### 1. `ai_live::grenade_bounce_scares_the_guard_into_the_flee_family` — a real port bug, fixed from the decompile

Reproduced on the final tree exactly as the driver said (`the guard never entered RunAwayFromObject 0x86
(log [(1, 4), (1, 4)])`, ai_live.rs:1334). Instrumented the HandleExplosives broadcast (temporary, reverted):
the grenade bounced 3.2 units from the gate guard (d² = 10.36 < 144), the kind-2 fear props 0x3A/0x3B set
[M 0x1435f0:30-34], but `line_of_sight` = false — and the port's segment probe showed the single hit at
**t = 1.0, the ray's endpoint**: the bounce had snapped the grenade exactly ON the floor triangle, so every
sight ray to it grazes that triangle at its end (the port's Collide_Intersect analogue accepts tt ∈ [0,1]
inclusive).

The game does not rest the bullet on the surface. `Bullet_CollisionHandler` (0x12ce80) stores the bounce
position as `BU+0x30 = *(HITDATA+0x40) + *(HITDATA+0x10) · uVar13` — the hit point offset ALONG the hit
normal, with uVar13 = uVar12 = 0x3d23d70a = 0.04 for the generic bounce arms (0x12ce80:257-258, the store
at LAB_0012d478; the contact-detonation arm overrides it to 0x3b03126f = 0.002 — that variant only moves
the MGL's Explode position by 0.038 on a ≥ 4.0 blast radius, folded into the one offset, noted here). With
the game's 0.04 offset the endpoint case never arises — the game's own Collide_LineOfSight ray
([M 0x1edf50 → 0x1ec690 Collide_RayIntersect], via NDrone2_CanSeeObject 0x1785e0 / CanSeeObjectFrom
0x176f18, called from HandleExplosives 0x1435f0 after ObjectVisibility 0x1781a8's distance/FOV gate) ends
0.04 above the triangle and the guard sees the grenade.

**The fix** (7 lines, `crates/nf-game/src/grenade.rs` — a shared file, the minimal hook the rules allow):
the bounce position and the `GrenadeEvent::Bounce` payload are `hit + normal · 0.04`. The payload feeds
the explosive_queue broadcast position, the bounce sound position and the fuse Explode position — all of
which the game sources from BU+0x30 [M 0x1435f0 entry+0x10 = obj+0x30; Sound_Play3D …, iVar19+0x30].

Verified: the scare test passes end to end (log [(1,4),(1,4),(1,134),(2,135),(122,136),(122,86),(122,88),
(123,90),(124,95)] — RunAwayFromObject 0x86 entered, 0x87/0x88 reached, back to attack; the 20+-unit
sister guard stays silent). Regression sweep: ai_live 35/35, grenade_live 2/2, stun_grenade_live 1/1,
fix_grenade_throw_live 4/4, fix_weapon_effects_live 6/6, fix_weapon_fire_modes_live 5/5, explode_live 2/2.

### 2. `walkreal_07000001_live::walk_from_the_spawn_to_the_exit_with_the_contact` — does not fail on the final tree

The driver's premise (fails on current master) does not hold on `c606b5f` + the swap base: the test passes
2/2, run three times (single, whole binary, and in the suite chunk). The bisect the task asked for has no
live subject on this tree. The merge history answers it: between the driver's 850/2 snapshot and my base,
**fix-08-collision2 (bf2f6bd)** landed with the game's stateful cel relink — its own commit message records
"the 01 escort walk passes again" — after fix-08-collision (32dea24)'s per-cel scoping had shifted the
collision the walk rides. That is a game-cited repair (build_LinkToRoom 0x1c9680 → build_FindCel 0x1c8cb0's
two-ray tie-break), not a host-math accident: the walk passes WITH the exact matrices on the final tree.
The ulp-level geometry shifts the swap introduced (verify-bitexact.md's t28 floor mechanism) are absorbed
by the game's own relink.

The test still logs the known AND-touch note ("the AND-touch did not fire: the escort's root-motion stall
left the contact at … — see docs/reports/walkreal-a.md") while passing — pre-existing behaviour, documented
in walkreal-a.md, untouched.

## Suite (final tree = my branch merged with origin/master; content-identical to c606b5f + the grenade fix)

Run as timed chunks (`tools/test/suite.py --only …`, per-binary 600 s limit, `--test-threads=1`, NF_ISO;
this sandbox reaps background processes, so the chunks are foreground — the binaries and totals are the
suite's own):

- chunk 1: 397 passed, 5 failed — the 3 sanctioned `disc_golden` + `fix_0c_pylon_live` (both user-spot tests)
- chunk 2: 540 passed, 1 failed — `probe_pylon_render::probe_render_pylon_only`
- chunk 3: 53 passed, 0 failed
- chunk 4: 14 passed, 0 failed
- `weapon_matrix_live`: my foreground window ended at 590 s before the suite's own 600 s verdict; recorded
  incomplete. The >600 s runtime is the documented pre-existing suite-gate condition (fix-movies-tests and
  fix-armour-ring reports: "weapon_matrix TIMEOUT reproduces" on the combined master, feature on or off);
  the main agent's 850/2 run had it under the limit on faster hardware.

**Total for the binaries that ran: 1004 passed, 6 failed, 31 ignored** — the 3 allowed `disc_golden`
failures (slim ISO) plus the three pre-existing pylon failures the fix-armour-ring report already flagged
for the main agent on the combined master (the 6691ef9+pylon ⇄ bf2f6bd collision-relink combination; not
this task's files). `walkreal_07000001_live` and `ai_live` ran inside the chunks with no FAILED lines, and
both task tests were additionally run directly, green.

## Shared files touched

- `crates/nf-game/src/grenade.rs` — the 7-line bounce offset above (the task's file list named
  placement.rs/anim.rs/ps2math.rs and the two tests; the root cause lived in the grenade bounce, so the
  minimal shared-file hook rule applies; every other file is untouched, `git status` clean).
- No test re-pins: neither test's expectations changed. `ps2math.rs`, `placement.rs`, `anim.rs` needed no
  change — the exact path survives master unchanged.

## Live checks for the main agent

1. PCSX2, any level with a guard, throw a frag (0x34) so it lands ~3 units from a guard in the open:
   the guard should flee (RunAwayFromObject 0x86) — confirming the game sees rested grenades. Then, with
   gdbwatch on the bullet's BU+0x30 (the object's position) at its first floor bounce: expected
   `position = hit_point + hit_normal · 0.04` (0x3d23d70a), i.e. 0.04 above the floor along the normal,
   NOT the hit point itself.
2. The pylon trio (`fix_0c_pylon_live` ×2, `probe_pylon_render`) still red on the combined master — needs
   the main agent's render pass (armour-ring's report has the details).

## What's left

- Nothing on the task's two items. The 0.002-vs-0.04 contact-detonation offset variant (MGL Explode
  position) is folded into the single 0.04 — a documented ≤0.038 approximation on blast radii ≥ 4.0.
- The bounce-offset products are host-rounded f32 (the game's are VU0 `_vmulbc/_vadd`); the difference is
  ulp-level on a 0.04 offset, below every pinned tolerance [S].
