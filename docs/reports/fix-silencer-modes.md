# fix-silencer-modes (102b) — the P2K silencer, the swap memory, the dev clip and the SSG's pace

Playtest 1013-1825 (07000006/07000005). Four symptoms; three fixed with the evidence below, one
(the silencer's on-screen persistence) reproduced and localized, its final gate needing the main
agent's live capture.

## 1. "Swap out, swap back — it didn't stay" — FIXED

**Cause.** The game remembers the equipped twin in `BLData+0x1BC + base·0xC` — written by
`Player_WeaponSelect` (0x1a2cc0:15-16 and again :27-28 after the weapon byte flips) on EVERY
same-base switch as `(equipped id − base id)` (1 when the P2K's 0x03 'Silenced' is on, 0 when the
base returns) — and `Player_WeaponChange`'s cycle walk (0x1a6f80:179-198) offers `cand +
skip[cand]` (the twin) first from every selectable, owned candidate, landing on it while
`Player_WeaponHasAmmo` passes, falling back to the candidate and CLEARING the marker when the
twin is dry. The toggle-back (+0x930/+0x932, 0x1a6f80:97-121) gates on the REMEMBERED id's BASE
(its selectable +4 and the base's owned slot — the twins share it per `Player_EquipWeapon`
0x1a8a58:39-41) and restores the remembered id VERBATIM (line 113), so a remembered 0x03 comes
back silenced. The port had none of this: the cycle always landed on 0x02 and the toggle-back
gated on the twin itself (whose owned slot is 0 — `weapon_data[0x03].selectable = 0` [L
ee_start_05.bin @ 0x2bf150 + 3·0x10C + 4] keeps `Player_InitAmmoWeapons` from marking it), so
every round trip lost the silencer.

**Fix.** `gun.rs`: `Gun::twin_skip` (the +0x1BC marker table), the cycle's twin step with the
dry-twin fallback + marker clear, and `toggle_back` re-gated through `base_of` (a new parameter).
`scene.rs`: the marker write on every same-base switch request (0x1a2cc0:15-16, before the
FP-model gate), and the two cycle call sites restructured (`cycle` is now `&mut self`).

**Test** (`fix_silencer_modes_live.rs`, both green): `the_silenced_twin_survives_a_weapon_swap_round_trip`
— attach through the real switch path (0x02 → 0x03, the +0xC8 attach at 1.3×, state 8), cycle up
to the SG5 and back → the live weapon is 0x03 with the marker still 1; sister: the fire-mode
toggle back to 0x02 clears the marker to 0 [M 0x1a2cc0:15-16]. Fail-first: the marker assert and
the round-trip assert both fail on the pre-fix tree (the marker stays 0 and the round trip lands
on 0x02 — the marker write is new code, so the pre-fix run fails at the first assert).

## 2. "The animation played, but then the silencer disappeared (bullets stayed silenced)" — reproduced, localized; the final gate needs live capture

The bullets stay silenced because the SIM is faithful: the variant swap flips the weapon byte to
0x03 during the +0xC8 attach (`Player_WeaponSelect` 0x1a2cc0:20) and everything downstream reads
the 0x03 record. The VIEW is the problem, and the disc data pins the shape: the twins share the
first-person skin (`weapon_data[0x02/0x03]+0xDC = 0x05000088`), the idle script (0x06000825) and
the fire script (0x06000823), and differ in the variant-swap pair (0x03's +0xC8 = 0x06000828
attach / 0x02's = 0x06000829 detach), the muzzle-flash meshes (+0x48/+0x4C zero on the silenced
twin) and the world models (+0x30/+0x80) [L ee_start_05.bin @ 0x2bf150]. The attach/detach are
201-frame sequences animating the part bones: bone 43's local translation (the suppressor) runs
(0.29, 0.33, −1.60) in the shared idle 0x040006fe [L via the port's `local_pose`, all 150
channels cover all 44 bones] versus (0.00, 0.09, 0.14) mounted at the attach's hold — so after
the attach completes and the game re-queues the shared idle (`Player_SetWeaponAnimObj` case 8
0x1a2e38:389-403 does re-queue +0xA0), a pure seq-driven draw would stow the suppressor again.
The game demonstrably keeps it mounted, so the persistent visibility is a draw-side gate this
session did not decode (the candidates — `AnimObjectDraw` 0x1c4490's fade bytes, the sleeve-type
family's weapon-byte reads, the skin's datum table — are enumerated in the WORKLOG). **Live
checks for the main agent** (PCSX2):
1. P2K equipped, attach the silencer, let the attach anim finish, idle: confirm the suppressor
   mesh stays mounted on the first-person model; then detach and confirm it disappears.
2. With `tools/trace/gdbwatch.py` on the view AnimObject's part/fade bytes (+0x110..0x113) and
   the skin's part meshes (0x0200083e/3f/41/44/43/42), attach and detach: which address changes
   with the weapon byte (anim+0x62 = 0x02 ↔ 0x03)? Expected: one gate flips per attach/detach.
3. nf-view renders (no GPU here): `--bond <05 spawn>` with the silencer attached then detached —
   today the port draws the suppressor at the idle's stowed bone pose after the attach (the
   user's "disappeared"); after the draw gate lands it should follow the weapon byte.

## 3. "Infinite ammo: the reserves filled, but the magazine didn't stay full" + 4. "Semi-auto doesn't shoot fast enough on the SSG" — one bug, FIXED

The dev cheat (`Dev::after_tick`) topped every OWNED weapon's inventory clip to the best-seen
value every 30 frames — but the live magazine drained between tops, and each drain of the
5-round sniper (0x24: clip 5, refire `weapon_data+0x40` = 120 frames, semi-auto — flags 0x68
without bit 2 [L weapon_data]) trips the auto-reload (`Player_WeaponFiring` 0x1a5460: the
released trigger with no round to fire → the reload states 5/6/7), so the sniper kept stopping
to reload: the user's "doesn't shoot fast enough". The magazine itself is the second half: the
game's own cheat shape (tools/trace/cheats.py) tops the store, not the gun.

**Fix** (dev.rs, dev only): with infinite ammo the CURRENT weapon's clip is pinned to its
`weapon_data+0x92` size EVERY tick — both the live `gun.clip` and the inventory slot; the spares
keep the trace-cheat's best-seen shape so no ammo cap refuses a pickup.

**Test**: `dev_infinite_ammo_keeps_the_snipers_magazine_full` — the sniper equipped, infinite
ammo on, tapping every 121 ticks for 900: the clip's minimum is 5 (every tick), no reload tick
fires, and every shot gap is 118–135 ticks (the table's 120 + the tap quantization) with ≥ 6
shots. Fail-first: on the pre-fix tree the clip dips below 5 between the 30-frame tops and the
reload states fire (the gap assert fails with the reload's ~150-tick hole). Sister: the P2K's
tapped cadence was already table-correct (10-tick taps → 10-tick gaps, the probe measured 39
shots with `interval` 8) — the fix changes only the drained-magazine behavior.

## Suite

- Baseline (pre-fix tree at the branch point): 959 passed, 3 failed (the allowed `disc_golden`
  trio), 29 ignored (25 min) — tools/test/suite.py, no TIMEOUT/CRASH.
- Targeted on the merged tree: nf-game lib 140/140 (the gun unit tests updated for the new
  signatures), `fix_silencer_modes_live` 2/2, `fix_swim_vent5_live` 2/2.
- Full suite on the final merged tree (origin/master merged at `89fbf2d`, the fix-08-walls
  lineage included): running in the background (suite_final.out) — the total line lands in the
  WORKLOG before the finish; the only game-code changes vs the green baseline are `gun.rs`,
  `scene.rs` (the swap arm + the call sites) and `dev.rs`.

## What's left

1. The silencer's draw-side gate (item 2) — needs the main agent's live capture above; then the
   port's FP view-model part filter follows the weapon byte.
2. The +0x1BC marker is per-BASE only in the port (`twin_skip[base]`); the game's byte lives in
   the same per-weapon slot and the walk reads it per candidate — equivalent for the disc's twin
   families (each base has one twin). If a base ever grows two variants, revisit.
3. The variant swap's clip-slot keying: the port's holster/draw path writes `gun.clip` to
   `inventory.clip[weapon]` — for a twin the game keys the clip to the BASE slot
   (`Player_EquipWeapon` 0x1a8a58:38-55). The variant-swap path (state 8) correctly touches
   neither. Worth folding into the draw-gate session.

## Files

- `crates/nf-game/src/gun.rs` — the twin marker, the cycle step, the toggle-back base gates (task file).
- `crates/nf-view/src/scene.rs` — the marker write on same-base switches + the two call sites
  (task file; small, additive).
- `crates/nf-view/src/dev.rs` — the infinite-ammo clip pin (task file).
- `crates/nf-view/tests/fix_silencer_modes_live.rs` — new (2 tests).
- `docs/WORKLOG.md` — the entry (append-only).
