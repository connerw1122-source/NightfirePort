# fix-aims20-modes-lights

Branch `s/fix-aims20-modes-lights`. Playtest 1014-1725: "Ames 20 wouldn't swap modes", "lights on
the aim 20 arent on" / "The lights aren't working on the gun" — the AIMS-20 (0x1a Burst / 0x1b
Grenade). Ground truth: `docs/live/2026-10-14-weapon-measurements-pcsx2.md`, the AIMS-20 section.

## What was broken, and the fixes

### 1. The side lights never stayed on (the render bug)

The game attaches mesh 0x2000132 to the view model's bones 8..0xB every tick while 0x1a/0x1b is
held and clears each bone per gate [M `Player_WeaponFiring` 0x1a5460:256-327]. The port attached
the mesh and kept bones 9/10/11 always lit, but killed the whole row in play: the row's idle
counter BL+0x93a has TWO reset sources and the port only modeled one.

- Modeled: any non-idle view anim (fire/reload/switch/raise) resets the counter [M
  `Player_SetWeaponAnimObj` 0x1a2e38:40-58].
- **Missed: any deflected analog input.** `Input_Update` 0x17ee88:54 clears the per-slot byte
  DAT_002a3a1c each frame and 0x17ee88:80-84 sets it when any of the pad's analog inputs reads
  non-zero; 0x1a2e38:41-44 RESETS +0x93a on such ticks instead of counting [L ee_start_05/
  ee_snapshot byte at 0x2a3a1c = 0 for slot 0 at rest]. The port never reset on input, so in play
  (walking, aiming) the counter climbed past the 15-frame bone-8 blink and the 45-frame all-off
  gate [M 0x1a5460:264, 314] and the row went dark ~0.75 s into any move.

Fix (`crates/nf-view/src/scene.rs`, the `Sim::run` aims block): the analog-deflection half — the
step computes `stick` from the Input the window builds (forward/strafe/turn/look non-zero — the
move and aim sticks; [S] that the digital buttons are not part of the game's 0x28 analog inputs)
and the counter resets on it. The same block now owns BL+0x93e as `drones.aims_raise` (new field,
`crates/nf-game/src/drone.rs`, on its own line per merge lesson 4): decremented while the view is
idle (0x1a2e38:47-50), refilled to 1200 on every non-idle tick (0x1a2e38:54-57; [L ee_start_05
BL+0x93e = 1200]).

The bone-9/10/11 gates, previously "unmeasured — the port keeps them lit", are in
(`scene.rs` `frame_state`'s aims arm), each citing 0x1a5460:

- bone 9 dark when +0x93e has drained (0x1a5460:275-281) — the ground truth's "red, dark, red,
  red" state after a long stretch without a view anim;
- bone 10 (the grenade light) dark when the 0x1b clip slot is dry (0x1a5460:282-288), blinking in
  grenade mode below 3 rounds on FlashMod > 0.75 (0x1a5460:289-294) — the ground truth's "with
  grenade rounds 0 → red, red, dark, red";
- bone 11 (the burst light) dark when the 0x1a clip slot is dry (0x1a5460:298-304), blinking in
  burst mode below 7 rounds (0x1a5460:305-308). FlashMod is the port's `flash_mod(frame)` stand-in
  [S], its 0.5..1.0 sweep crossing 0.75 like the game's pulse.

With the stick reset, play matches the measured display: lit from the draw, the ammo pattern
visible, the row only going dark when Bond stands dead-still (no stick) for 0.75 s — that last
part is what the code says [M 0x1a5460:314]; live check below.

### 2. The mode swap: the variant hop corrupted the grenade magazine

The press itself was correct (the twin rules [M `Player_Weapon` 0x1a24c8:246-260 +
`Player_WeaponHasAmmo` 0x1a16c0], the gate the ground truth measured: with no grenade rounds the
press does nothing — the port matches). The corruption came at the swap itself: the variant-swap
arm (same base, +0xC8 script, view state 8) carried the live magazine across the 0x1a→0x1b hop
("the same physical gun, the magazine continues"), and the end-of-run sync then wrote the burst
slot's 30 rounds into clip[0x1b] and the 9mm spare into the type-12 carried — 30 "grenades". The
twins do NOT share the slot: their ammo types differ (+0x90: 5 vs 12 [L ee_snapshot weapon_data]),
so `Player_AmmoInGun` 0x1a9190 picks each record's OWN slot [M; bldata.md +0x2F0=[0x1A],
+0x2FC=[0x1B]] and the game's HUD after the swap reads "4-0 Grenade" — the grenade slot's count
[L ground truth]. With the burst magazine squatting in the grenade slot the pair was unusable in
play (the grenade side fired 9mm counts; the mode byte and the lights read nonsense ammo).

Fix (`scene.rs`, both variant arms — with and without the +0xC8 script): flush the live cache to
the old slot, swap the weapon byte, re-read the new weapon's own slot. For same-ammo twins (P2K
0x02↔0x03) `clip_slot` resolves both sides to the base, so the round-trip is a no-op and the
P2K's magazine-continues behavior is unchanged (the original reason the round-trip was removed —
clip[0x03] zeroing — came from the pre-ammo-type slot rule; `clip_slot` now gates on the ammo
types, and fix_silencer_modes_live still passes).

### 3. The give (the port's acquire affordance) left the twin dry

The dev menu's give (`do=weapon&id=0x1a`) filled only the given record's clip + spare, so a
give-started session could never test the swap — the ground truth anticipated exactly this
("may be this gate if the port's give didn't fill the grenade slot — check both"). Fix
(`crates/nf-view/src/panel.rs`): a twin-linked give fills the twin's own slot once (the way a
taken 0x1b pickup does — 07000014 carries the 0x1b pickups; the real-game acquire path is
unchanged, this is the panel's affordance).

## Verification

- `crates/nf-view/tests/fix_aims20_modes_lights_live.rs` (new, 3 tests, all through
  `Sim::step_bond` with the `Input` the window builds): **all 3 FAILED on the pre-fix tree**
  (the stick reset missing; bone 10 lit with a dry grenade slot; the give leaving the twin dry),
  all 3 green after. Sister assertions inside: dead-still 60 ticks still kills the row
  (0x1a5460:314), the drained +0x93e pattern (bone 9 dark, ammo lights lit), the dry-twin press
  reading the burst side back, and the ground-truth no-swap gate.
- The real 07000014 path checked headlessly: the armoury's 0x1b pickup (kind 1, 4 rounds) fills
  clip[0x1b] = 4 through the real take path (`Drones::update_pickups` → `Inventory::take`); both
  view models (0x1a → char, 0x1b → char) exist on 14; the pair shares the +0xC8 swap script
  0x6000944 and the fp model 0x0500007d [L ee_snapshot weapon_data], so the swap completes there.
- Regression sweep on the merged tree (the variant-swap change touches the shared switch path):
  fix_silencer_modes_live, fix_weapon_modes_live, fire_modes3_live, fix_weapon_fire_modes_live,
  weapon_switch_live (nf-game), fix_reload_hang_live, gadgets_live, fix_weapons_all_live,
  fix_grenade_throw_live, fix_auto_switch_live — all pass (43 tests, 0 failures).

## Files

- `crates/nf-view/src/scene.rs` — the aims block (stick reset + aims_raise), the frame_state
  bone-9/10/11 gates, the two variant-swap clip handoffs (task file).
- `crates/nf-game/src/drone.rs` — `Drones::aims_raise: u16` added on its own line (shared file,
  additive hook only).
- `crates/nf-view/src/panel.rs` — the give's twin-slot fill (shared file, additive hook only).
- `crates/nf-view/tests/fix_aims20_modes_lights_live.rs` — the new test file.

## Suite

The background fast suite died at startup twice in this sandbox (the nohup shell lost the cargo
env; `suite.out` shows `FileNotFoundError: 'cargo'`). Restarted with the env at ~03:05 UTC; it was
still in its build phase at the deadline, so **the full-suite total line is not available this
session** — the next session should read `tail -1 suite.out` (and its FAILED/TIMEOUT lines) and
re-run on the final tree. What DID run: the targeted regression sweep above (all green) and the
task's own tests, on the tree already merged with origin/master (merge 04eed4b1).

## Render: the view model with the lights on

The main agent (no GPU in the sandbox — the port's headless `frame_state` arithmetic is my
verification; the picture needs the render):

1. `nf-view <iso> 07000014.bin --dev`, give the AIMS-20 (dev menu → Weapons → AIMS-20; the give
   now fills the grenade slot too). What should show: **a row of four round red lights on the
   left side of the receiver, lit from the draw** (no delay), staying lit while moving/aiming.
2. In the same session open the panel and drain the grenade slot (`do=weapon&id=0x1a` re-give
   after firing the grenades dry): the third light from one end goes dark — the ground truth's
   "red, red, dark, red" (bone 10, the 0x1b clip gate).
3. After picking up the armoury's 0x1b pickup (at (-35.2, 1.9, 31.0)): all four lit; press B —
   the gun swaps, HUD "6-0 Grenade" (or "4-0" with a fresh pickup), the row stays lit.
4. Stand dead-still (no keys, no mouse) for ~1 s: the row goes dark [M 0x1a5460:314 — the code's
   45-frame all-off gate]; any move brings it back. PCSX2 live check below settles whether the
   real game matches.

## Live checks for the main agent

PCSX2 (gdbwatch.py/fieldprobe.py, 07000014, the armoury):

1. BL+0x93a (0x16cf0f0+0x93a on the 05 layout; re-find BLData on 14), WATCH write+read while
   Bond walks (stick deflected) vs stands still: expected — pinned 0 while any stick moves,
   climbing ~1/tick while dead-still [M 0x17ee88:80-84 + 0x1a2e38:41-44].
2. BL+0x93e, WATCH while idle: expected — decrementing 1/tick from 1200, refilled to 1200 on any
   fire/switch/raise [M 0x1a2e38:47-57].
3. The dead-still lights: with the AIMS drawn, no input for 2 s — do the four lights go out at
   ~0.75 s (0x1a5460:314's `FRAME_RATE * 45.0 < +0x93a`)? The ground-truth capture says "staying
   lit while idle"; if the real game keeps them lit while dead-still, the 45-gate's counter read
   needs a second look (the decompile's `sVar5` chain at 0x1a5460:305-314 — disasm, not the C).
4. The swap HUD: with clip[0x1b] = 4 (a taken 0x1b pickup), press ○ — expected HUD "4-0 Grenade"
   [L ground truth] and the port now matches (clip[0x1b] intact across the hop).

nf-view `--bond` spots: first-person view model — the dev-menu give is the spot (step 1 above);
the lights' bone matrices are the test's `frame_state` check (gadgets_live pattern).

## What's left

- The full-suite total line (the sandbox deadline hit mid-build — see Suite above).
- Live check 3 (the dead-still all-off arm) — the one [S] stand-in in the light gates is the
  FlashMod phase; everything else is [M]+[L].
- The exact analog-input set for the stick reset: the port resets on forward/strafe/turn/look;
  whether the game's 0x28-input sweep includes the digital buttons' analog lanes is unmeasured
  (harmless either way for the lights — extra resets only keep the row lit longer).
