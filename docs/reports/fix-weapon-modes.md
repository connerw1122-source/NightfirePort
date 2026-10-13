# fix-weapon-modes — the fire-mode switch, the heat-seeker homing, the per-weapon fire sounds, gadget switching

Branch `s/fix-weapon-modes` (from `origin/master` 6519d6f). Every fact is tagged [M] decompile
(file:line of `work/decomp/ACTION.ELF/`), [L] measured bytes (`work/ee_start_05.bin` / `USATxt.dat`),
[D] disc, [S] suggested.

## What the game does (the mechanics this task ports)

**The fire-mode button is the pad's ○ (action 12, mask 4 — newly pressed)**, read in `Player_Weapon`
0x1a24c8:204-260 [M], gated on the view-anim state 0 and not (aiming ∧ weapon `+0x68` & 0x40);
for the watch pair 0x54/0x55 the fire press ORs in (`Input_Action(slot, 9, 4)`) [M :186-197].

- `weapon_data[w]+0x05 == 0` → the **mode byte** at `BL + w*0xC + 0x1BB` (the per-weapon clip
  record's byte 3) cycles: `mode + 1`, wrapping to 0 at the record's mode count (`+0x2E`, u8)
  [M :210-215]. Every gun has count 1 [L ee_start_05.bin]; only the grenades 0x60/0x61 have 2.
- `weapon_data[w]+0x05 ≠ 0` (s8 twin) → the **twin-variant switch**: `new = w + twin`; when
  `Player_WeaponHasAmmo` 0x1a16c0 passes — carried rounds of the record's ammo type (`BL+0x170 +
  t·2`), rounds in the clip, ammo type 0, or a matching ground pickup for flag-0x20000 weapons; it
  does NOT read the owned byte [M] — the game writes it to `obj+0x63` (the switch request) and
  resets the zoom floats [M :246-260].

The twins that matter (names [L USATxt.dat via the port's `Lang::bind`]):

| pair | records | labels (`+0x30` mode lane / `+0x38` SP name) | `+0x6C` |
|---|---|---|---|
| P2K silencer | 0x02 ↔ 0x03 (twin +1/−1) | 'Semi'/'Silenced', name 'P2K' lane 0x0500000e/0x0500000f | 0x40050 both |
| AT-600 Scorpion | 0x2E ↔ 0x2F | **'Heatseeker'**/'Unguided', name 0x05000048 'AT-600 Scorpion' | **0x32200 (0x20000 homing) / 0x1a200** |
| Torpedo launcher | 0x2C ↔ 0x2D | 'Guided'/'Unguided', name 0x0500004a 'Torpedo Launcher' | 0x12204 / 0x1a200 |
| SG5 family | 0x16 ↔ 0x17 | 'Burst'?/'Semi' lane 0x05000001/0x0500000e | — |
| Watch | 0x54 ↔ 0x55 | 'Micro-Camera' both; ○ or fire toggles | — |

**The heat-seeker homing**: `Bullet_update` 0x12e7b0:483 runs `Bullet_homing` 0x12f358 on every tick
after the move when the fired record's `+0x6C` carries 0x20000 [M] — only the Scorpion's
'Heatseeker' 0x2E (and the Torpedo 'Guided' 0x30's 0x20200) set it [L]. `Bullet_homing`: keep the
target (`BU+0x40`), else scan every object for `Bullet_heatseekable` 0x12f248 — the guards (class 2)
while `+0xFC` set and health `+0xAC` > 0 and anim state ≠ 1, the copter bodies (class 0), Bond
(class 3, `BL+0x894` > 0) [M] — nearest within 1000 units, inside the 0.66-dot cone of the bullet's
direction, `Collide_LineOfSight(…, 8)` clear [M :100-150]; the target's death (`+0xfe & 1`) clears
and re-scans [M :152-160]. Steering: `dir += clamp((to_target − dir)·0.0333, ±0.01)` per component,
normalised, matrix re-aligned [M :200-240]. No RNG draws anywhere in the homing (determinism holds).

**The per-weapon fire sound** is the fire anim script's op-1 id, remapped by
`AnimProcessScriptCmds` 0x1c1b38:251-277 [M]: 1→2 (weapons 7/9), 0x403→2 (3/5), 0xfa→0x1fb (0x14),
0x106→0x284 (0x1e..0x23), 0x1e1→0x602 (0x10), 4→0x604 (0x16/0x17); every other (id, weapon) plays
raw. **The P2K is weapon 0x02 and is NOT in the table: its fire scripts 0x06000823/0x06000824 carry
0x403 at frame 1 and 0x403 is the real, final id** [M WORKLOG 0064; L script hashes in the snapshot].
The port's `remap_op1_sound` already matched the table row for row (a doc gap, not a code gap) — this
task pins it with a test.

**Gadget identities** (a correction the repo needs — `docs/port-map/gadgets.md`'s id table is
shifted; code evidence overrules): 0x4A/0x4C Stunner, 0x4E/0x4F **Q-Laser** (burn), 0x50/0x51
**Q-Claw** (grapple), **0x52 briefcase** (deploy 'Phoenix Ronin' minigun, `GT_DeployMiniGun`
0x181560 [M 0x1a4bf0:268-286]), **0x54 Micro-Camera** (aim + fire = a photo bullet; `CamSubject`
writes the placement channel [M 0x1bc668:16-21]), 0x56/0x57 Q-Decryptor, **0x58 Q-Worm** (fire at a
class-0x49 PC installs the virus [M 0x1a5460:530-547, 0x1aee20:61-73]), 0x59/0x5b Shaver. The
user-facing symptom "picked up the briefcase and it wouldn't let me switch to it" maps to 0x52: the
level pickups are on **0700000c** (54.0, 1.3, −173.5) and 07000012 — **07000008 has no 0x52 pickup**
(its `oBriefcase_closed` is a kind-4 LABEL item) [D full-disc placement sweep].

## What I changed

1. **The mode byte lives in the Inventory** (`pickup.rs`, `fire_mode: Vec<u8>`, own line) — the
   game's `BL + 0x1B8 + w·0xC + 3`. Every reader keys it: `GunParams::from_record(w, mode)` picks
   the shots-per-pull pair `+0x28 + mode·2` (already modelled, now fed), and the HUD label reads
   `+0x30 + mode·4` with the live byte (`hud.rs`, was hardcoded 0).
2. **The button** (`gun.rs` `fire_mode_switch`, `scene.rs`, `player.rs` `Input.mode_switch`,
   `lib.rs`): keyboard **M** (the pad's ○; the game's action-12 semantics — view idle, not
   (aiming ∧ scoped), the watch's fire OR-in via the port's `prev_trigger` edge). A twin result
   goes through the normal switch request (`obj+0x63` → the port's holster/draw machinery); a mode
   result writes `fire_mode[w]`.
3. **The homing** (`drone.rs`): `PlayerBullet.homing` (the record's `+0x6C & 0x20000`, set in
   `fire_player`) + `Bullet_homing`'s scan/steer after the move in `update_player_bullets`
   (targets = the port's live drones: `present ∧ enabled ∧ ¬corpse ∧ health > 0`; LOS via
   `world.segment` — the `Collide_LineOfSight` stand-in [S]). No RNG draws.
4. **The island loadout** (`player.rs` `init_level_loadout`): `Player_InitWeapon` 0x1a7ef8 case
   0x0700009's grants (handgun 48, Stunner 999, Decryptor 0, **Q-Worm 0x58 0**, Q-Claw 0, dart 999,
   Shaver 999, selecting the dart) and the 0x070000a/b `Player_RamLoad` + unowned-handgun fallback —
   the Q-Worm was never granted before (the user could not select it on 0700000a; the walk tests
   hand-granted it).
5. **A test-export wrapper** `remap_op1_sound_for_test` (the `map_analog_stick_for_test` pattern);
   `replay.rs` pins `mode_switch: false` (the replay format predates the button).

**Shared files touched (small, additive, per the lesson-4 rule — every new field on its own
line):** `player.rs` (one `Input` field + the loadout cases), `scene.rs` (the mode block, one
`Sim.prev_trigger` field, the mode passed into `from_record`, one pub test fn), `drone.rs` (two
`PlayerBullet` fields + the homing block). No renames, no reordering, no shared-file reformatting.

## Verification

`crates/nf-view/tests/fix_weapon_modes_live.rs` (NF_ISO headless, the real `load_with_carry` +
`Sim::step_bond` path):

- `p2k_mode_button_toggles_the_silenced_twin` — ○ on 0x02 requests 0x03, the switch completes,
  ○ again requests 0x02. Sister inside: the no-twin/no-mode-count weapon 0x0c ignores the button.
- `the_heat_seeker_homes_onto_a_guard_and_the_unguided_twin_does_not` — the same off-axis shot:
  0x2E bends onto the guard (a Drone impact within 400 ticks), 0x2F flies straight and misses.
  The guard is LOS-placement-checked in the test (the same gate the game's scan applies).
- `fire_mode_labels_follow_the_record` — 0x2E→'Heatseeker' 0x05000047, 0x2F→'Unguided'
  0x05000015, 0x02→'Semi', 0x03→'Silenced' [L].
- `the_island_loadout_grants_the_q_worm` — a cold 0700000a owns 0x58/0x56/0x5b.
- `op1_remap_table_matches_the_decompile` — the six table rows + the P2K's 0x403-unchanged + the
  0x24-sniper edge (0x106 raw outside 0x1e..0x23) [M 0x1c1b38:251-277].

Suite: `NF_ISO=… cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats
-p nf-audio -p nf-view -- --test-threads=1` — see the WORKLOG entry for the final count (run
started 18:20 UTC; only the 3 `disc_golden` tests may fail on the slim ISO).

## What's left / honest gaps

- **The briefcase's fire side (the 'Phoenix Ronin' deployable minigun) is NOT ported** —
  `GT_DeployMiniGun` 0x181560 → `GT_Create`/`GT_Update` 0x180498 (the turret slerps to the nearest
  enemy and fires `weapon_data[0x60]` bullets; E mans it, substate 0x10). Selecting 0x52 and firing
  runs the port's generic bullet path (harmless, no game effect). A new `gt.rs` module is the right
  home; it needs the nearest-enemy scan + a per-0.1 s bullet pipe (`drones.fire_player`-style).
- The mode byte's **spread effect** (`Bullet_init` 0x12c5a0:153 reads `+0x28 + mode·2` vs the shot
  counter for the inaccuracy) is not ported — the port's spread model doesn't grow within a pull.
- Grenade mode cycle (0x60/0x61, the only mode-count-2 records) flows through the new byte but the
  grenade throw path (`grenade.rs`) ignores `shots_per_pull` mode differences beyond the existing
  `GunParams` feed.
- **The decryptor's decode mini-game** (states 3/4/5, `MenuManager_Create(0x40000021)`, the
  0x2a37d8 KeyCode globals) is unported (pre-existing; only the burn path works).
- The camera hint message (0x01000216 "Hold manual aim to use camera", first equip) is not posted.
- `docs/port-map/gadgets.md`'s id table + `gadget_select_live.rs` naming need the §gadget-identity
  corrections above (docs-only; I did not rewrite the shared doc — merge-lesson 5).
- Repo contradiction flagged, not resolved: `weapon_data +0x60` is 'fire_sound' in
  `docs/atlas/weapon-data.md` (read by `DroneWeap_FireWeapon` 0x178f68:274-278 and played [M]) but
  'view_anim_datum' in `crates/nf-formats/src/weapon.rs:272` (the fields-player retyping). Both
  cite readers; the main agent should settle it (an [L] watch on 0x2bf150+w·0x10C+0x60 vs the
  played id settles which consumer owns the bits).

## Live checks for the main agent (PCSX2)

1. **The ○ toggle**: on 07000005 with the P2K out, press ○ — the HUD's fire-mode label flips
   'Semi' ↔ 'Silenced' and the next shot is suppressed (no muzzle flash: 0x03's +0x48 = 0 [L]).
   Spot: `--bond` at the start, fire once per mode.
2. **The heat-seeker**: grant the AT-600 (or find the pickup level), press ○ until the label reads
   'Heatseeker' (record 0x2E), aim NEAR (inside ~45°) but not at a guard and fire — the rocket
   bends onto him. With 'Unguided' (0x2F) the same shot flies straight. `gdbwatch` on the guard's
   `Drone_tag+0x150` (last damage) should show 100.0 (0x2E's +0x0C) on the homed hit.
3. **The Q-Worm on 0700000a**: the loadout grants 0x58 (owned byte at `BL+0x1BA+0x58·0xC` = 1 at
   load, alongside 0x56); cycle gadgets (d-pad Up/Down) to 'Q-Worm', aim at Q-WORM_PC_1 (the office
   PC), fire — ch 0x48 (objective 4) goes on [M 0x1aee20].
4. **The briefcase on 0700000c**: pick up at (54.0, 1.3, −173.5), G to 'Phoenix Ronin' (0x52), fire
   — in the game a `SuitCase-Frame` turret (0x2000359) drops out and opens fire. The port will NOT
   do this yet (the gap above); a render spot there shows whether the pickup+select half works.
5. **nf-view renders**: `--bond` spots from the level start — (a) fire-mode label present at rest,
   hidden while the weapon name shows on a switch; (b) after M, the label text changes to the
   twin's; (c) the Scorpion's muzzle light colour (0x2E's +0x54..56 = 00 8A FF-ish orange [L]).

## ntfy

(_sent after the finish steps; recorded in the WORKLOG.)
