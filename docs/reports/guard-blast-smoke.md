# guard-blast-smoke — guards react to blasts and smoke

Task 727, branch `s/guard-blast-smoke` (from `origin/master` 170bcf01, merged `origin/master`
f09f3c6c for the final tree). The uncited-audit gap ②: grenades staggered nobody and smoke did
nothing to guards — the port applied the blast as direct damage (`ddmg · damage_taken`) and sent
msg 8 (the bullet route), and the smoke cloud was visual-only.

## What's done

**The explosive path (msg 9).** `spawn_blast`'s drone arm now mirrors `Explode_Propagate` case 2 →
`Drone_ExplosiveHit` 0x13aab8: the attacker stamp, then `SendMsgSelf(9, DroneImpact*, 0, 0)`. The
msg-9 handler is `DroneFunc_HandleImpact` case 9 (0x1469b8:173-201) → `NDrone2_ExplosiveImpact`
0x146480:
- allow = 0 (the idle/patrol/waiting guards' (0x56, 0) pairs): the damage + `PainTalk` + the
  resume entry — a punched-idle guard goes hostile at once, no stagger state.
- allow = 1 (the firing family's (current, 1) pairs): the +0x118 resume stamp and state 0x54,
  whose ENTER re-runs the impact with allow = 1 — the 0x47 stagger anim with the variant by the
  blast's bearing (`atan2(dx, dz)` vs the drone's yaw; 45°/135° bands [M 0x146480:143-159,
  `NDrone2_ExplosionDir` 0x14aba8]) and follow-on `+0x118`.
- The damage is the raw falloff amount through `HitDamage` with part −1 (→ 1): the game passes
  `*(float*)impact` straight in f12 [M disasm 0x14654c] — **no `+0x14C` multiply on this path**
  (that is the bullet callers' job; the old ×0.25 armour factor made frags 4× weak).
- A kill routes per kind [M 0x1465e4-0x14697c]: 0x0D → 0x44, 0x1B/0x1C → 0x92, 0x1D → 0xBF, else
  **0x45 DeathByExplosion** — ported (0x163a80): corpse, alert 3, the special-death route,
  OnInitDeath's halves, the death-channel latch, the launch anim 0x44 with the ExplosionDir
  variant, follow-on 0x47 DEAD; EXIT/timer C raise the death channel; update drops the weapon.
- Per-state (resume, allow): a scan of every DSTATE carrying a case 9 (2026-10-16) shows the pairs
  equal the msg-8 table's in every state — the port reuses `impact_args`. States without a case 9
  (Attack/Combat/WaitSwitch, the mission states' Global fallback) drop the message — the game's
  own routing [M 0x15b830 has no case 9]. The stun-grenade and taser families' case 8/9 → 0x44
  arms [M 0x172bb0/0x1631c0/0x163350/0x162eb0] and the KO/surrender arms are kept.
- Disabled guards take the blast through the port's established GLOBAL-fallback convention (the
  msg lands on HandleImpact's (0, false) arm): the merged frag pin
  `guard_drops_flashbang_live::frag_at_the_same_spot_kills_the_trio` requires it, and it passes.
  Whether the game's own `Collide_SphereIntersect` list skips `obj+0xF0 & 0x20` for DISABLED
  guards is an open live check below — the bullet pick's drop [M 0x1eae80:26-30] is documented,
  the sphere-list build (`Collide_SphereIntersect` 0x1ec940 walks the per-frame hit list at
  obj+0xD0) is not pinned either way.

**The smoke path (msg 0x17).** The cloud (`Occlude_Create(pos, 1200, 1)` 0x17ea80, class 0x2A) is
touched in the drone's per-tick collision scan (`Drone_CollisionHandler` 0x140968:85 walks the hit
list) and `NDrone2_DealWithObjHit` case 0x2A sends `Drone_Message(obj, 0x17, 0, 0)` [M disasm
0x1405a8 `addiu a1, 0x17` → 0x13a678]. The port scans the live clouds each tick (the cloud's
glist bounds — `Data.models[0x2000273]`, centre +1.1 up, radius 1.74 [L] — against the port's 1.05
object-contact sphere [S, live check below]) and delivers msg 0x17 before the timers. The handler
is `HandleImpact` case 0x17 (0x1469b8:197-249): prop 0x36/0x34 guards are immune; everyone else
stamps the resume and enters 0x7B (no allow branch — the arm never reads `allow`). The family:
- 0x7B SmokedOut (0x173c50): ENTER = AlertStatusSet(2) + the cough anim 0x20, follow-on 0x7C.
- 0x7C Loop (0x16b878): ENTER coughs again (no follow-on) and arms timer C at frame + 5; TIMER_C
  → 0x7D; a further 0x17 re-arms the +5 (the "until it clears" behaviour); 6/8/9 route (0, 0),
  7/0x19 (0, 1).
- 0x7D Recover (0x173d30): ENTER sets `+0x4F8 |= 0x10000` and plays anim 10 with follow-on =
  SMI+0x10 (the stamped resume) — the exit back to combat.
- 0x7B/0x7D have no 0x17 case: the game drops the msg there (the GLOBAL fallback has none either
  [M 0x15b830]); the port handles it in place (a false return would re-smoke through the port's
  GLOBAL stand-in forever).
- No damage anywhere in the 0x17 arm.

## How it was verified

`crates/nf-game/tests/blast_smoke_live.rs` (4 tests, all pass, `--test-threads=1`):
1. `frag_kill_routes_death_by_explosion_and_the_control_is_untouched` — a real 0x34 (radius 8 /
   damage 100 [L record]) popped at the midpoint of the two enabled guards (pl 5/6, ~5.9 each)
   kills both THROUGH the game's route: the state log carries 0x45 DeathByExplosion (not the old
   0x44), then DEAD, health ≤ 0; the control (pl 8, ~89 away) keeps health, state and alert.
   Fail-first: on the old code the kills were `DEATH_ANIM` from `spawn_blast` directly.
2. `smoke_cloud_puts_guards_in_the_smoke_states_until_it_clears` — two 0x36s popped at pl 7 (no
   immunity props) and pl 5 (prop 0x34, gas-proof): pl 7 enters the family within the first ticks,
   is still in it at pop+90 (the per-tick 0x17 re-arms the 5-tick timer), health untouched;
   pl 5 never smokes (the immunity gate); the far control (pl 267, 15.7 away) untouched; after the
   clouds' life is spent the Loop's timer runs out → 0x7D → the stamped resume (the sniper-idle
   pair's 0x2A SniperAim). Fail-first: the old code had no smoke states at all.
3. `blast_on_a_firing_guard_staggers_through_0x54_back_to_the_resume` — a survivable 5.0 impact on
   a guard in AimStandFire (an allow = 1 state): enters 0x54, stamps the resume, loses exactly the
   raw 5.0·1.5 (the level's Easy difficulty factor — no +0x14C), stays staggered until the 0x47
   anim completes, then returns to the resume. Driven through `send(msg::EXPLOSION)` — no
   07000005 blast is survivable at 10 hp (the frag's minimum in-radius damage is 12.5).
4. `blast_on_a_stunned_guard_dies_the_bullet_way` — the taser family's case 9 → 0x44, not 0x45.

Crates run before pushing: `nf-game` (the tests above + the crate's suite through the runner).
Full suite on the final tree (merged `origin/master` f09f3c6c): see the total line below.

## Suite total

(to be filled from the final run)

## What's left

- The `0x92`/`0xBF` death states are not ported (their kinds — 0x1B/0x1C/0x1D — do not occur in
  the port's roster [*suggested*]); those kills land in 0x44 like the port's bullet path.
- The prop-0x32 hurt broadcast (`Drone_SM_BroadcastMsg(0x12, …)` in `ExplosiveImpact` 0x146480:136)
  is not modelled — matching the port's bullet path (the hurt-broadcast gap is documented in
  docs/systems/drone-damage.md).
- The `+0xD8` gate (`== 0x14` skips the stagger anim) is read through the port's `kind` stand-in
  (the same approximation `HandleImpact` case 8's `0xC → 0x6E` uses).
- The 07000009/0A stamped arm in HandleImpact's cases 7/0x17/0x18/0x19 (the `+0x89` mission setup)
  is not modelled — those levels' mission states are not ported.
- Bond's own smoke cough (`Player_DealWithObjHit` case 0x2A → the player's whiteout/cough) is a
  separate gap (the whiteout for the stun grenade exists; the smoke side is not ported).

## Live checks for the main agent

1. The smoke cloud's collision sphere: run 07000005, throw the smoke grenade, `gdbwatch` the
   cloud object (class 0x2A) at `obj+0x80`/`obj+0x8C` — expected: `+0x8C` ≈ the glist radius 1.74
   [L models] (or whatever the collide stamp is), `+0x80` = the cloud pos + (0, 1.1, 0.12). Also
   `fieldprobe` a guard inside the cloud: `obj+0xF4` (state) should read 0x7B/0x7C within a second
   of the pop, and the guard's `Drone+0xCC4` (timer C deadline) = frame + 5 while in 0x7C.
2. The frag's per-state pairs on a WAITING (disabled) guard: throw a frag next to a WaitSwitch
   guard before his channel opens — expected (per the state table): NO damage, NO state change
   (the msg drops). The port now matches via the enabled filter; a live poke confirms.
3. The 0x47 stagger anim variant: `fieldprobe` `Drone+0x57B`/the anim state of a guard blasted
   from the front vs behind — expected variants 1 vs 0 (the bearing bands).

## Usage

not visible to me
