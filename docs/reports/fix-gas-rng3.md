# Report: `fix-gas-rng3` — landing the gas RNG work (111d's four failures)

Branch `s/fix-gas-rng3` (from `origin/s/fix-gas-rng2` 5aac9292 + `origin/master` f0284f45 merged,
WORKLOG both kept). Sandbox window 18:26–20:25 UTC; the drive-crp-format session's 503 queue claim
was closed first (the driver had already moved it to done/ with the report attached).

## The verdict on the held question

111d asked: is the draw order right and the tests must be re-derived, or is the order wrong?
**Both arms were true, for different tests** — and one missing draw was hiding it:

1. **110o's impact-site model was correct all along.** The re-read of `Effect_Bullet` 0x1ce8a8 that
   tempted a "correction" (the `pfVar6` pointer selection between `afStack_140`, `&fStack_120` and
   `&CONSTANT_ZERO_VECTOR` at ACTION.ELF 0x2d6200) resolves to **`Gas_Init`'s 5th parameter — the
   puff's orientation quaternion**; the velocity pointer is the 6th (`param_6`) and the decompile's
   call sites pass the literal 0 there in EVERY `Gas_Init` [M: muzzle 0x1a4298:101, guard fire op
   0x1c1b38:461, impact loop 0x1ce8a8] — so the NULL arm (spin, sign, kind-7 wind ×3; kinds 9/10
   position-copy with kind 10's life halving) is the game's real dispatch everywhere. `gas.rs` is
   untouched from 111d. (A bisect — swapping master's draw-free `init_ex` into the branch — had
   first proved the cover-test shift rides the impact draws; the re-read then proved those draws
   are game-real, so the shift had to be a *missing-draw* asymmetry elsewhere.)
2. **The real gap: the guards' fire path is one draw short per shot.** The game spawns guard
   bullets in the ANIM fire op — `AnimProcessScriptCmds` 0x1c1b38 case 7 (`Bullet_init` :451, the
   muzzle bone via `AnimGetBoneWorldTrans`), then `Light_Create`, then the weapon+0x68 & 0x2000
   no-smoke gate, then `Rand_FRand(2π)` 0x1c1b38:456 (the puff quat-z) and `Gas_Init`'s 5 = **6
   shared-stream draws per guard shot** [M]. The port's `fire_weapon` consumed 5 (hit roll +
   light's draw + spread ×3). fix-guard-weapons3's "flash-sprite jitter draws" (FRand(2π) 0x179268
   + FRandHalf(0.2) ×2, inside the kind-0x40 sprite-create branch of the DSTATE-driven
   `FireWeapon` 0x178f68) are a different site — not the anim op's. **Fix: the op's 2π draw added
   per guard shot in `fire_weapon` (after the light, gated on the 0x2000 bit) [crates/nf-game/src/
   drone.rs].** With 6=6 the port's per-shot stream count equals the game's.
3. **fix_gas_rng_live's age expectation was wrong.** `gases_smoke` 0x1ab2d0 is control_funcs class
   14's UPDATE handler [M: the .data table at 0x26f890 + 14·0xc — classes 3 = Player_Update/
   0x1949a8 collide, 5 = bullets/0x12ce80, 14 = gases_smoke], so the pool ages in the update pass
   which runs BEFORE `Collide_Update` spawns the muzzle puff in the same frame. The frame-end read
   shows the raw creation state (life 128, scale 0.05, alpha 0x80); the old 127/aged expectation
   assumed a post-collide aging nothing in the frame performs. **Fix: the test re-pinned to the
   game's frame order, with a next-tick sister (life 127 after one more `step_bond`) proving the
   aging schedule [crates/nf-view/tests/fix_gas_rng_live.rs].**

## Each of the four, before → after

| Test | On master+111d | On this branch | Why |
|---|---|---|---|
| `nf-game/tests/ai_live.rs:249` guard_takes_cover | `UnderCoverAim lasted 90 (live 37)` | **GREEN** — the whole test: the state list 0x58/0x5a/0x8c/0x93/0x94/0x95/0x96, Init 28..=34, Aim 32..=40, cover node 1, the live fire position, the sister | the per-shot stream now carries the game's 6 draws; the guard's cover rolls land back on the game's trajectory |
| `nf-view/tests/fix_gas_rng_live.rs:84` | `the muzzle smoke puff exists` (no puff matched kind7+rate+life127) | **GREEN 2/2** (main + the no-smoke sister; the k-offset replay still lands) | the age re-derivation above; the puff exists with life 128 and ages to 127 on the next tick |
| `nf-view/tests/fix_guard_weapons_live.rs:156` | `no candidate guard with weapon 0x12 completed the collection` | **STILL FAILING, differently**: the weapon-0xe collection now COMPLETES and its gap analysis reads min gap 60 (the game's interval clamp — the test's first half works); every weapon-0x12 candidate then fails the harness scan (`no level parking spot` ×6, `no fire in 6000 ticks (shots 0, seen 0)` ×2) | the patrol trajectories de-tuned from the master-era parks once the streams carry the game's real draws; the harness re-derives its parking from live positions at scan time. Needs a harness re-derivation (a fresh live-capture park / wider candidate scan) — a mechanic fix, not a tolerance change |
| `nf-view/tests/walkreal_07000001_live.rs:218` | `the contact must walk his route to the LoadPointActivator plate` | **inconclusive in the window** (the run exceeds the sandbox's remaining minutes; mid-run diagnostics show the contact walking and a guard shooting/flanking — `[walk] guard 2: shooting … no exposed part for 540 ticks — flanking`), left running in the background; result lands on the branch's `walkreal.out` scratch only | same stream realignment as the cover test; needs one full run on the main agent's machine to judge |

## What was run

- `nf-game --lib`: **190 passed / 0 failed** (includes the 5 gas unit tests).
- `models_misc_live`: **10 passed / 0 failed** — the 128-tick puff-death pin holds with the unaged
  creation.
- `fix_gas_rng_live`: **2 passed / 0 failed**; `ai_live` cover test: **1 passed / 0 failed**
  (34 others filtered).
- The full fast-runner suite was restarted on the final tree but the sandbox window (build 601 s +
  342 binaries) cannot fit it; the affected binaries were run directly instead. The suite's earlier
  partial on the pre-fix tree had already reproduced the four failures (ai_live FAILED at 18:39).

## What's left (the next session picks up)

1. **fix_guard_weapons_live**: re-derive the 0x12 candidate harness (the parking/sight scan) —
   the fire-rate analysis itself (the 60-tick clamp) already reads correctly on the new stream.
2. **walkreal**: one full run; if the contact still stalls, trace where his route diverges (the
   same op-draw asymmetry may remain in the CONTACT's fire path — he is also an NPC with a weapon).
3. **The guard's muzzle puff visual**: the game's fire op creates a real kind-7 puff at the guard's
   muzzle bone; the port consumes the draws stream-side (nf-game) but does not create the guard puff
   (nf-view renders nothing) — a visual follow-up with the same `init_muzzle_smoke` call.
4. **FireWeapon 0x178f68's flash-sprite draws** (0x179268-0x1792b0, the 3 jitter draws inside the
   sprite-create branch): still unconsumed [the port creates no sprite object]; fix-guard-weapons3's
   live check for the main agent (gdbwatch the kind-0x40 object create on a guard shot) settles
   whether the game's sim stream carries them.

## Live checks for the main agent

1. PCSX2, breakpoint `AnimProcessScriptCmds` 0x1c1b38 case 7 (the fire op) on a guard shot: confirm
   `Rand_FRand(0x40c90fdb)` at 0x1c1b38:456 executes once per shot with weapon+0x68 & 0x2000 == 0,
   and that `Gas_Init`'s 6th argument (the velocity pointer) is 0 — the port's 6-draw-per-shot model
   hinges on both.
2. PCSX2, gdbwatch `control_funcs` class 14 slot (EE 0x26f890 + 14·0xc → `gases_smoke` 0x1ab2d0):
   confirm it runs in the update pass (before `Collide_Update` 0x1ea398) of the frame the muzzle puff
   is created — the re-derived 128-at-frame-end expectation hinges on it.
3. nf-view render (`--bond` spots, after the guard puff visual lands): a guard firing at the gate in
   07000001 — a small kind-7 puff at the guard's muzzle bone for ~2 s.

## ntfy

(sent at finish; see the WORKLOG line if it failed)
