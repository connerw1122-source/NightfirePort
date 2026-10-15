# fix-hit-locations — guard hit locations: head, body, limbs

Task: check the game's hit-location damage on guards (hitbox set per skeleton, per-zone damage
multiplier, armour/helmets, the reaction anim per zone, the death anim choice by zone and
direction, the blood/impact effects per zone — `NDrone2_DealWithObjHit` 0x13fff0 case 5,
`DroneFunc_SendHurtMessage`, the hitbox data), port what differs. Test: a P2K head shot vs body
shot on a 05 guard deals the game's damage and plays the game's reaction/death anim ids.

Everything below is [M] read in `work/decomp/ACTION.ELF` (function + line) unless tagged otherwise.
Branch: `s/fix-hit-locations`. Files touched: `crates/nf-game/src/drone.rs` (the task's own file)
and the new `crates/nf-view/tests/fix_hit_locations_live.rs`. No other shared file was edited.

## Verified faithful — no change (the audit half of the task)

- `NDrone2_HitDamage` 0x145558 vs `nf_game::damage::hit_damage`: part<0 → 1; fists/taser weapons
  (1, 0x4A-0x4D) on the head read torso; the 0700001b all-torso rule; the damageable==0 && !forced
  early return recording only; torso ×Torso with Combat&8/Jacket&2/Vest&1; arms 0x14/0x15/0x17/
  0x20/0x23/0x27 ×Arms (Combat, Jacket); legs 0x31-0x38 ×Legs (Combat); head ×Head with the
  silent-kill byte +0x3A except the 2×Torso arm; the difficulty factor (1 Easy, 3-4 Hard, else
  Normal). The port matches line for line.
- **Armour/helmets**: the game reads `Drone+0xBB` (0x145558:46,83,98,101,112). The ONLY writer in
  the whole decomp is `NDrone2_DefaultInit` 0x14b300:566 storing **0** — the bits are never set in
  this build, so the port passing `armour: 0` is faithful; the multiplier plumbing (Helmet &4,
  Combat &8, Jacket &2, Vest &1) is already in `hit_damage` and would go live if any writer ever
  runs. No fix, evidence recorded.
- **Hitboxes per skeleton**: `AnimGetCollData` 0x1c4ac8 + `HumanCollisionBones` 0x2c6be8 (rows:
  5 head, 4 torso, 0x31-0x38 legs, 0x14/0x15/0x17/0x20/0x23/0x27 arms; entry 0's centre = bone +
  (0, bone.y − other.y, 0), the others midway; extents × skin scale) — ported (`damage::boxes`).
  `Collide_Jointy` 0x1ec180 takes the FIRST box in list order (:125-126, :173-174 — it stores
  `+0x50 = 0`, i.e. **material 0**), and `Collide_Update` 0x1ea398 sorts victims by distance —
  the port's per-victim `first_box_hit` + nearest-victim pick matches.
- **Death anim by zone and direction**: `DroneAnim_LocationDeathAnim` 0x13c780 — Bond aiming
  (`glb_players+0xdc → obj+0x60 & 1`) and part 4/5 → CallAnim(0, 0x42, 1 if
  |ATAN2(HIT+0x30, HIT+0x38) − obj yaw| ≤ 1.5707964 else 0, next); else +0x3A = 0, DeathTalk,
  0x40 with Rand(4) gated on `Drone+0x585` — and +0x585 IS `Drone_AnimInfo[danim+0x10]`
  (`DroneAnim_SetDAnimInternal` 0x13aee8:53), the same value the port reads from
  `tables.info[danim][0x10]`. Port matches.
- **The zone hit sounds**: `NDrone2_DoHitEffects` 0x1458e0:106-113 — damage > 0.5 and
  `weapon+0x70 & 0x180 == 0` → 0x601 (part 5) / 0x413 (else) at the hit point. Already ported
  (drone.rs bullet path); asserted by the new tests.
- `DroneFunc_InjuredTalk` 0x13fa20 is **MP-gated** (`MPSettings._388_4_ != 0` wraps the whole
  body) — correctly absent from the single-player port.
- The taser route (0x4A/0x4C → msg 7) and weapons 0x43/0x44 → msg 0x19 (the level-0x07000009/A
  photo/stun machine — `DroneFunc_HandleImpact` 0x1469b8 case 0x19 rewrites +0xC5/+0xC6 and goes
  to state 0x50 on those levels) remain unported (state machines of their own; both were already
  listed as known gaps — not regressed, not new).

## Fixed (each cites its address; NF_ISO status in the next section)

1. **The mode byte never reached `HitDamage`** — `do_hit_effects` passed `mode: 0` into
   `damage::Target`. The game reads `Drone+0xC5` (0x145558:64): mode 0xD (the party guests, via
   `GetDTYPENEW` 0x14c938 dtype 0xF → 0xD) sends a head hit down the **2×Torso arm with NO silent
   kill**. The port would have dealt ×10 head + silent kill on a guest. Now passes `self.mode`
   (the port's +0xC5). [M 0x145558:63-80]
2. **The bullet path never sent `DroneFunc_SendHurtMessage`** — `NDrone2_BulletImpact` 0x1461c8:30
   calls it on EVERY bullet hit (before the death branch, so a one-shot kill still screams). The
   punch path had it (fix-player4); the bullet path did not. Extracted the block into
   `send_hurt_message` (prop 0x32, the +0x4F8 & 8 once-per-life bit → the port's `screamed`,
   mode != 2) and call it from both. [M 0x145e28:19-57]
3. **The bullet death state ignored the mode byte** — the port always entered 0x44. The game:
   0x1E → 0xF2, 0x1B/0x1C → 0x92, 0x1D → 0xBF, else 0x44 (0x1461c8:31-53). Now matched. (The same
   modes' punch/KO arm 0x1E → 0xF2 was already handled in the KO path.)
4. **The blood/impact puff was missing** — `NDrone2_DealWithObjHit` case 5's second switch
   (0x140300-0x14034c): the default arm spawns `Gas_Init(3.0, 6.0, cel, HIT+0x40, 0, 0,
   0x20008bc, 5, -1, 0xffffff00)` gated on the UNALIGNED f32 at `weapondef+0x06` > 0 (bytes
   [class, pad, min-range lo/hi] — the class byte non-zero makes the denormal positive; the game
   reads it as a float and the port copies that read), skipped for the explosives
   {0x2b, 0x34-0x37}; the stun guns {0x32, 0x33} spawn `Gas_Init(1.0, 17.0, …, 0x200026e, 5, …)`
   un-gated; the default arm's gas object gets flags |= 8. The port spawns the puff into its
   `GasPool` (mesh, scale, kind exact; life 0.5 s [S] — the game's 11th `Gas_Init` argument is
   dropped by the decompiler at every call site, the known G-033 caveat; the packed colours
   start −1 / end 0xffffff00 map onto the pool's rgba start/end convention). [M 0x13fff0 case 5]
5. **The flinch-variant remap for modes 0x1B/0x1C** — `DroneAnim_LocationImpactAnim` 0x13c8b8:96-98
   remaps the computed variant through the static table copied from DAT_002f48b0 when
   `Drone+0xC5 ∈ {0x1B, 0x1C}`. The 18 u32s read from `work/elf/ACTION.ELF`
   (LOAD 0x107100@0x1000): **0,0,0,0,0,1,1,2,1,2,3,4,3,4,0,0,0,0**. Ported as the local constant.

## NF_ISO tests — `crates/nf-view/tests/fix_hit_locations_live.rs`

Driven through the real path (`nf_view::load` + `Sim::step_bond`, the P2K fired from the aim,
`bond_aiming` held). The difficulty on the port's 05 sim is Easy (factor 1.5), so the expected
numbers are ×1.5 the Normal sheet.

- **`p2k_leg_shot_deals_the_legs_multiplier_and_plays_the_leg_stagger` — GREEN end-to-end.**
  Waits for the guard to reach an aim-family state (the allow-1 set of `DroneFunc_HandleImpact`
  0x1469b8), aims at a leg box, fires once: the hit part lands in 0x31-0x38, the damage is
  3.5 × 0.75 × 1.5 exact, and the flinch is state 0x4C with variant 13/12 following the part
  (0x31-0x33 → 13, 0x35-0x37 → 12) [M 0x13c8b8:85-93]. Sister: the part→variant pairing.
- **`p2k_head_shot_deals_the_games_damage_and_plays_the_shot_death` — green through damage,
  sounds, gas, silent kill; ONE open assert.** The head shot: part 5, damage **52.5 = 3.5 ×
  DroneDamage_Head(10) × 1.5 exact**, the guard dies, the silent-kill byte sets, the zone sound
  0x601 plays, the blood puff 0x20008bc exists. The body shot: part 4, 3.5 × 1.0 × 1.5 exact,
  sound 0x413, no silent kill, and the aimed torso kill plays the shot death.
  **OPEN**: the death anim asserts 0x42 (DEATH_SHOT — Bond aims + part 4/5) but the port played
  0x40 (plain DEATH). Everything the 0x42 gate reads (bond_aiming, hit.part 5, the recorded ray)
  verified green in the same run, so the gap is either the drone's `bond_aiming` copy at the
  death ENTER or a second death entry — one focused probe left for the next window. All asserts
  BEFORE it pass, so this is a narrowing, not a blocker.
- **`party_guest_head_shot_deals_two_torso_without_the_silent_kill` — OPEN at the find step.**
  No present/enabled mode-0xD drone exists at tick 2 on 07000007 in the port (the guests likely
  resolve their mode through their start script or start disabled); the fix itself is cited
  directly from 0x145558:63-66 and is unit-level verifiable via `damage::hit_damage(Target
  { mode: 0xd, .. })`. What was tried: filtering present/enabled/health>0/mode==0xd (with and
  without `drawn`) at tick 2 — empty. Next window: find the guest's mode-setting tick (or run the
  party script) and re-shoot.

## Suite

The full suite did NOT run in this window: this sandbox has no root and no ALSA headers; the
audio crate's link (`-lasound`) needed a user-prefix `libasound2-dev` + a fixed `alsa.pc` +
runtime symlink before anything would link, and the sandbox hit its disk ceiling twice (10 GB
rootfs: the ISO + work + the release target do not all fit; `work/ee_*.bin` and caches were
freed to proceed). What ran: `nf-view`'s new test file (above, 1 green / 2 with the open items
described) on the merged-with-origin/master tree. The remaining suites are un-run in THIS sandbox
— the branch's commits are additive to drone.rs + a new test file; the next window should run the
full suite first (docs/tools/test-suite.md).

## Live checks for the main agent

1. **The mode byte at load** — PCSX2, 07000005 at level start: read `Drone+0xC5` of a guard
   (expect ≠ 0xD, e.g. 1-3); 07000007 at the party: read a guest's `Drone+0xC5` (expect 0xD).
   Access: read, address = the drone's obj+0xE0+0xC5. This settles the guest test's find step.
2. **The aimed head kill's death anim** — PCSX2, 07000005: shoot a guard in the head while aiming
   (L1): watch `Drone+0x56A` (the anim state) go 0x42 with variant 1/0 by direction, not 0x40.
   Spot: the first mansion guards. This is the port's one open assert.
3. **The blood puff** — PCSX2, 07000005 body shot on a guard at close range: the red puff gas
   (mesh 0x20008bc) at the hit point, none when the weapon is a grenade class (0x34-0x37).
4. **The hurt scream** — PCSX2, 07000007: shoot (not punch) a prop-0x32 civilian once non-fatally:
   the scream broadcast (msg 0x12) — nearby guards react as to the punch.

## nf-view `--bond` spots (visual, need the main agent's render)

- `--bond` 07000005 facing a guard ~4 units away, fire one body shot: a small red/white puff at
  the impact (mesh 0x20008bc) + the 0x413 flesh hit sound; a head shot: the 0x601 sound.
- The flinch: an aimed leg shot on an aiming guard plays the 0x4C leg-stagger variant (13/12),
  an aimed torso hit the variant-5 snap.

## Left for the next window

1. The 0x42-vs-0x40 death-anim open assert (one probe: print `bond_aiming` + `self.hit` at the
   death ENTER; suspect the drone's copy or a double death entry).
2. The guest test's find step (needs the guest mode-resolution tick on 07000007).
3. The full suite run on the final tree (blocked by the sandbox's disk/ALSA this window; the
   alsa-prefix recipe is in the WORKLOG).
4. The +0x5D0 knockback (`DealWithObjHit` case 5 adds `HIT+0x30 × 0.175` capped 0.175, then
   `NDrone2_Move` 0x14eff8 does `pos = pos − +0x5E0(prev) + +0x5D0(cur)`, copies cur→prev and
   decays ×0.95) — the per-tick recurrence reads as `pos += cur − prev` which would step
   backwards; not ported without a live check: **watch `Drone+0x5D0/0x5E0` and the guard's
   obj+0x30 for 3 ticks after one P2K hit** (access: read, expect: a one-way stagger ≤ 0.175/tick
   decaying 0.95).
