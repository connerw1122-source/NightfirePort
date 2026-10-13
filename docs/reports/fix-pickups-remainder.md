# fix-pickups-remainder — report

Branch `s/fix-pickups-remainder` (from origin/master c72446f). Task: G-029 remainder — the census
over the 27 on-foot levels, then port what fails: the auto-switch, the toggle-back key, level-placed
and script pickups, `Pickup_CreateFromSet` (types 7–11), keys/armour/types 5–6, respawn (state 2),
spin, moving-platform landings, the creation push-out, text messages, `Upgrade_Weapon`'s upgrade
bytes.

## What was done

### 1. The census (the driver's first ask) — green

`crates/nf-view/tests/fix_pickups_remainder_live.rs` (`pickup_census_every_level_creates_and_offers_its_pickups`,
opt-in `NF_ISO`, `NF_PICKUPCENSUS=1` for the table): for every on-foot level (the levels-sweep's
27-level filter) it re-parses the level archive, reads every type-0xF0 placement's params
(`level_tag+0x2C+4k`, [M 0x1d04f0:457-462]), and asserts the sim created each pickup with the right
kind/id/rounds at the placement's position, then asserts each live pickup is **takeable from where a
player stands** — the game's own touch predicate from a ring of standing spots (`Pickup_Update`
0x18e148:203-214, radius + 1.1, LOS from the body centre) and `Pickup_Handler`'s consume rules
(0x18e650), with re-touches (a standing player touches every tick; `Player_EquipAmmo`'s clip-fill
can return 0 on the first touch — the game's own quirk, [M 0x1a8d18] — and takes on the next).

First run: **29 failures**. Final run: **0** (109 nf-view test binaries green). The kinds the disc's
SP levels place: {0: 84, 1: 43, 2: 2, 3: 51, 4: 2, 6: 50} live pickups at load.

### 2. What the census found and what was ported

- **Kinds 7–11 (`Pickup_CreateFromSet` 0x18f1b0) DO appear in SP** — 25 placements on 0700004b and
  one on 07000041 (the bonus/MP-flavoured maps). The port took them as unknown kinds (refused
  forever). Ported: `Pickup_Create`'s intercept ([M 0x18dda0:104-105]) — kinds 7–11 resolve at
  create time to a **kind-0 pickup of the weapon `PickupMatrix[row][(kind-7)*2]`, the BASE id,
  `clip_size·2` rounds, the WEAPON's own glist** (not the placement's object). `PickupMatrix`
  (0x2b9100, symtab 110 bytes) row = the u16 at `MPSettings+0x1B4` = 0 in SP [L ee_start_05.bin];
  row 0 = sets 7/8/9/10/11 → 0x6/0x1e/0x2c/0x6/0x10 [L]. When the set weapon is the AR (0x1a) the
  game also drops a 0x1B ammo pile — noted in the code, not ported (no SP placement hits it with
  row 0). `Pickup::create_from_set` + the create-path calls (view build, sim build, script
  reservations, the runtime script-spawn fallback in drone.rs).
- **The auto-switch (`Player_IsBetterWeapon` 0x1a6eb0 + the flag)** — decoded and ported:
  `Inventory::is_better_weapon` [M]. `BestWeapon` is the 36-byte symtab object at 0x2bede0
  (entries are weapon ids, ranked by their `weapon_data[entry].base`; live values [L ee_start_05]),
  the guard blocks the switch when the CURRENT weapon's base is a gadget ([0x4a,0x5e]), the verdict
  is `rank(pickup) < rank(current)`; a base outside the table (the rockets 0x2C/0x2E) never
  auto-switches. `Player_EquipWeapon`'s tail ([M 0x1a8a58]) applies it only on a WEAPON equip, with
  the per-player flag `DAT_002a38d2[idx·0x158]` (live 1 [L]) writing `anim+0x63`. The sim computes
  the request (`Drones::weapon_switch_request`, drone.rs update_pickups).
- **The 0x1f/0x25 ammo remap** — `Player_EquipAmmo` remaps its OWN id through the base's upgrade +1
  ([M 0x1a8d18:11-19]: 0x1f → `Upgrade_Weapon(0x1e)+1`, 0x25 → `Upgrade_Weapon(0x24)+1`); the port
  only had `Upgrade_Weapon` (EquipWeapon's remap). Added `Inventory::upgrade_ammo_id` and used it in
  the ammo path.
- **Kind 5 consumed** ([M 0x18e650 case 5:229-233]) — the port refused it (the pickup stayed on the
  floor forever); now `Taken::Message` (the shared tail: message 0x2000013, the take sound, the
  channel, consumed). HUD message bound in scene.rs + the window title path in lib.rs.
- **Respawn (state 2)** — `PICKUPINFO+0x2C` was parsed and ignored; taken pickups were always
  deleted. Ported the take tail ([M 0x18e650:338-348]): respawn 0 → deleted, 0xFFFF → stays
  takeable, else hidden for `FRAME_RATE_INT·respawn·10` ticks and restored ([M 0x18e148:182-189],
  `Pickup::respawn_wait` in the per-tick loop).
- **Spin (`+0x38`)** — kind-6 placed pickups spin about +Y by `REC_FRAME_RATE` per tick
  ([M 0x18dda0:138-144, 0x18e148:193-201], `Pickup::spin_tick`).
- **The toggle-back key** — already ported (`gun.rs toggle_back` + scene.rs wiring); verified this
  session against `Player_WeaponChange` 0x1a6f80 (the +0x930/+0x932 memory, mode 0/1 ranges). No
  change needed.
- **Text messages** — already ported (hud_tick's `Text_AddMsg` types, 180 ticks); kind 5's
  0x2000013 added.
- **Keys/armour/types 6** — already matched the decompile (verified this session against
  0x18e650:217-236); no change.

### 3. Gated OFF: the auto-switch's view drain (the one open item)

`Player_EquipWeapon`'s tail fires on the first Raptor drop take on 07000001 (rank 9 beats the P2K's
15) — with the drain live, `walkreal_07000001_live::walk_from_the_spawn_to_the_exit_with_the_contact`
(green on master, re-verified in a master worktree) wedges in the contact follow: Bond ends stuck
1.13 above the floor at d=3.89 from the contact after the draw machine swaps to the Raptor. The
request is computed in the sim and unit-tested; the scene.rs drain that would write `gun.switch_request`
is `if false`-gated with the full note. Re-opening it needs the interaction chased (the switch's
view-model/anim side effects vs the contact's script) — live checks below.

## Tests

- `nf-game --lib pickup` (9 tests): the auto-switch ranking (incl. the sisters: rockets never
  switch, gadgets block, the upgraded P2K has its own slot), `create_from_set` (the rocket set:
  kind 0/base/clip·2/the weapon's model), the respawn wait (60·60·10), plus the pre-existing suite.
- `fix_pickups_remainder_live` (the census): failed with 29 mismatches before the fixes, green
  after — the would-have-failed proof for every fix above.
- Full suite this session: nf-game 31 binaries green; nf-formats + nf-audio: only the 3 allowed
  `disc_golden` failures (this sandbox also shows `audio_sample_count_matches_stream_length`
  failing — pre-existing on the slim ISO, none of my diff touches those crates); nf-view 109
  binaries green, 0 failures.

## Not ported (with reasons)

- **Moving-platform landings** (`Control_InheritVelocity` on a dynamic hit, 0x18e148:118-149): the
  port's fall tests the static world only (the pre-existing deviation). No SP placement in the
  census rests on a mover; the game's dynamic-hit branch needs the moving-object velocity that the
  port's world does not expose yet.
- **The creation push-out** (`Collide_SphereIntersect` mask 0x50D, 0x18dda0:164-186) for DROPS: a
  drop spawned inside geometry rests inside it. The census takes placed pickups fine; drops come
  from drone deaths at open feet positions.
- **`Upgrade_Weapon`'s upgrade bytes (0x2BEEAA, the shop level)**: the port hardcodes `Upgrades = 0`
  ([L ee_start_05: 0x2beea8 = 0]); the shop UI is not ported. The tables are all in
  `upgrade_weapon`/`upgrade_ammo_id` already.
- **The AR set's second pile** (0x1a → a 0x1B ammo pile at +0.1, the 0.5 fall velocity, obj+0xE8 =
  0.5 [M 0x18f1b0:49-62]): no SP placement reaches it with row 0.
- **`+0x888` on the kind-5 take**: the port's Inventory has no such field (its meaning — the
  taker's "has bonus/gadget" word — is unchased); the take effect is otherwise complete.

## Live checks for the main agent

1. **The auto-switch drain re-open.** In PCSX2 on 07000001: take the first Raptor guard's drop with
   the P2K held — the game should holster the P2K and draw the Raptor (rank 9 < 15, flag
   0x2a38d2 = 1 [L]). Then watch `anim+0x63` (the byte at `obj+0xDC`+99) during the draw. In the
   port's nf-view: `--bond` near a Raptor guard drop and confirm the drawn gun changes (the drain
   is gated; flipping `if false` at scene.rs:999 re-enables).
2. **The walkreal-01 wedge with the switch live** (the regression): with the drain re-enabled, walk
   the contact follow (`--test walk_from_the_spawn_to_the_exit_with_the_contact`); watch whether
   Bond's capsule stands on the contact's head/crate 1.13 above the floor at (3.56, 6.47, 13.71)
   when it wedges — a screenshot of Bond's feet there decides whether the draw machine's anim change
   lifts the capsule or the contact's route changes.
3. **A respawning pickup** — none exists in the SP census (all +0x2C = 0 [L ee_start_05/01]); poke a
   placed armour's `PICKUPINFO+0x2C` = 60 with tools/trace/fieldprobe.py before taking: it should
   vanish and return after 600 ticks.
4. **The set pickups' visual** (0700004b): the kind 7–11 placements should draw the WEAPON's model
   (0x6/0x1e/0x2c/0x10), not the placement's own object. `--bond` spots: (140.7, 36.6, -118.3)
   (set 11 → 0x10), (132.5, 47.1, -102.7) (set 9 → the rocket launcher).
5. **The kind-5 pickup** (07000014's placements at (-28.0, 2.7, 6.9) is kind 0; the disc's kind-5
   placements: 07000041's #0? — the census's kind table shows 0 kind-5 placed; the script spawns
   would show message 0x2000013 on take).

## ntfy failed

All three fallbacks returned HTTP 429 (daily message quota, ntfy.sh "limit reached: daily message
quota reached; increase your limits with a paid plan"): the curl published `{"code":42908,"http":429}`,
the python fallback raised `HTTPError: HTTP Error 429: Too Many Requests`, and the wget fallback
returned empty with the same limit (the same 429 the fix-facial session recorded on this channel).
