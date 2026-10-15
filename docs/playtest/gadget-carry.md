# Which on-foot levels carry the thrown gadgets

Task `gadget-carry-map` (2026-10-15, branch `s/gadget-carry-map`). The question: for every on-foot
story level, which of these the level carries — **Laser Tripbomb 0x3a, Satchel Charge 0x3b-0x40,
Remote Mine 0x37-0x39, the grenade family, the Micro-Camera 0x54, the Q-Decryptor 0x56/0x57, the
Q-Worm 0x58, the AT-600 Scorpion 0x2e** — so the main agent can `give` a weapon in PCSX2 on a level
that already carries it (the level's own models, textures and sounds must be loaded). Non-visual,
data only; no game code changed.

Tags: [M] decompile/disassembly (work/decomp/ACTION.ELF + the addresses cited), [L] measured bytes
(EE snapshots, USATxt.dat labels), [D] disc (the level archives through `Level::parse`, the same
records the sim builds). Proof harness: `crates/nf-view/tests/gadget_carry_live.rs` (6 tests, NF_ISO).

## 1. The id table (reconciled with the task's names)

The disc name labels (`Txt_BindLabel` of `weapon_data+0x38` against USATxt.dat, recorded in
grenade.rs and fix_weapons_all_live) name the grenade family [L]:

| id | the game's name | the task's name |
|---|---|---|
| 0x34 | Frag Grenade | — |
| 0x35 | Stun Grenade (the flash branch, `+0x70 & 0x2000`) | "Frag 0x35" |
| 0x36 | Smoke Grenade | "Smoke/Flash grenades" (0x35 is the flash) |
| 0x37-0x39 | Remote Mine (+ the detonator twin) | Remote Mine 0x37 |
| 0x3a | Laser Tripbomb | Laser Tripbomb 0x3a |
| 0x3b-0x40 | Satchel Charge (fuse `((id−0x3B)·5+5)·60` [M 0x12c5a0:199-208]) | Satchel Charge 0x3b |
| 0x2e | AT-600 Scorpion (heat-seeker; twin 0x2f) | the Scorpion 0x2e |
| 0x54 | Micro-Camera | the Micro-Camera 0x54 |
| 0x56/0x57 | Q-Decryptor (twin family) | Q-Decryptor |
| 0x58 | Q-Worm | Q-Worm |

So the task's "Frag 0x35" is the game's **stun/flash grenade**; the frag is 0x34. Both are mapped
below. Placement-kind ids (0x2E lock, 0x3A grapple, 0x3B dynamic object, 0x35 PCQ worm —
docs/models/misc.md) are a DIFFERENT namespace and are not used here.

## 2. Where a level gets a weapon (the sources, with addresses)

1. **The start loadout** [M `Player_InitWeapon` 0x1a7ef8, the jump table behind `sltiu v1,a0,74`
   at 001a8268]: the per-level cases (01-09, 0c, 11, 14, 18-1b fresh kits; 02-08/0d/12/13/4a/15-17
   `Player_RamLoad` 0x1957a8 falling back to the fresh kit when the gate weapon is unowned; the
   default case's give-everything list for ids outside the table). Ported as
   `nf_game::player::init_level_loadout` (fix-start-loadouts, docs/reports/fix-start-loadouts.md).
   The common tail [M 0x1a7ef8:318-325] equips laser 0x4E 999 + fists and runs
   `Player_CheckWeaponsLoaded` 0x1a97c8 (a selectable weapon whose fp skin +0xDC is missing from the
   session hashtable loses owned). **No story kit carries the tripbomb, the satchel or the
   Scorpion** — only the default (MP) kit does.
2. **Pickups placed in the level archive** [D]: placement kind 0xF0, param 0 = the pickup kind
   (0 weapon, 1 ammo) and param 1 = the weapon id; created by `Pickup_Create` 0x18dda0 through
   `parsemap_create_dynamic_objects` (the port's scene build, `Pickup::create_placed`; the rounds
   override for 0700008's 0x2c [M 0x18dda0]).
3. **Script-spawned pickups** [D]: the NIS streams' `Cmd::Entity` obj_type 0xF0 (`Script_CreateEntity`;
   the port's `script_pickup_reservations`), same params. **The scan found none carrying a tracked
   gadget in any story level** (the port's reservation path exists for them, the data is just empty
   of gadgets).
4. **The captain grenade drop** [M `DroneFunc_OnInitDeath` 0x145bd0, the `a2` read at 0x145de8]:
   a captain (`Drone+0x19 ≠ 0`) drops grenades via `DroneFunc_DropGrenade` → `Pickup_CreateSimple`
   0x18f348: **07000001-08 drop weapon 0x35 ×2; 07000009-0B / 11-16 / 4A drop 0x34 ×1**
   (docs/systems/pickups.md).
5. **The weapons a dead drone drops** [M `DroneWeap_DropWeapon` 0x17a2a0]: the held weapon's base
   id (`weapon_data+0x02`) at the held gun's world matrix — the param-9 weapon of the placement.
   A tracked base here means the level carries the gadget as a drop.
6. **The gadgets drones throw** [M `NDrone2_DefaultInit` 0x14b300:348-376, placement param 11's
   selection nibble]: 1 → 0x36, 2 → 0x35, 4 → 0x34, **5 → 0x3a + low nibble** (0x3a tripbomb,
   0x3b satchel, ...), 6 → 0x6c; the 0x07000014 clamp at :374-376 pins the family to 0x34.
   Informational for the carry map (the thrown gadget's mesh must still load with the level), but
   it is why a level can "carry" a gadget the player never picks up.
7. **`Pickup_CreateFromSet`** (types 7-11 [M 0x18dda0 → 0x18f3xx]): `PICKUP_MATRIX_ROW0` =
   [0x6, 0x1e, 0x2c, 0x6, 0x10] [M pickup.rs] — **no gadgets**, listed for completeness.

The mesh side: the view model is `weapon_data[id]+0xDC` (a character skin —
`Player_EquipWeapon` 0x1a8a58 refuses to equip when the session hashtable misses it; the port checks
`level.skins`), the projectile mesh is `+0x5C` (`Bullet_update` 0x12e7b0 disasm 0x12e8f0 sets it on
the flying object; 0 = no projectile — the camera/decryptor/worm throw none). The game's hashtable
accumulates every archive loaded in the session (docs/models/weapons-items.md §2), so "session" =
resolved through an earlier story archive; the port's `load_impl` merges the same weapon-model
hashes. The gadget fp skins on 0c-13's 0x35/0x58 and 16's 0x56 resolve ONLY through that table
(their own archives don't hold them) — this is the game's own mechanism, not a port defect.

## 3. The table (level id → the gadgets it carries)

From the archive scan + `init_level_loadout` (the full data:
`cargo test -p nf-view --test gadget_carry_live print_the_carry_map -- --nocapture`). fp = the
view model, proj = the projectile mesh; local = in the level's own archive, session = through an
earlier story archive (§2). Positions are the placement translations (x,y,z).

| Level (name) | Carries (source → ids) |
|---|---|
| 07000001 Paris Prelude A | **0x35** captain ×2 [M 0x145bd0], fp local / proj local; 0x36 thrown by drones only — fp/proj on NO archive (the game's own invisible-stun artifact, weapons-items §3) |
| 07000002 The Exchange A | **0x35** pickup ×1 rounds 5 @ (−21.41, 0.00, 29.29) [D], captain ×2; fp/proj local |
| 07000003 Alpine Escape A | **0x35** captain ×2; 0x36 thrown only (meshes missing, as 01) |
| 07000004 Enemies Vanish A | **0x35** captain ×2; drones throw 0x35; fp/proj local |
| 07000005 Mayhew A (**the test level**) | **0x35** captain ×2 (fp/proj local); **0x54** loadout [M case 05] (fp local, proj none). No placed gadget pickups — the tripbomb/satchel ride `--give` |
| 07000006 Mayhew B | = 05 cold [M case 06 → the 05 kit]: 0x35 captain, 0x54 loadout |
| 07000007 Mayhew C | = 05 kit minus the P2K request (fists): 0x35 captain, 0x54 loadout |
| 07000008 Mayhew D | = 05 kit: 0x35 captain, 0x54 loadout |
| 07000009 Tower A | **0x34** captain ×1; **0x56** + **0x58** loadout [M case 09]; fp local, proj none (gadgets throw no projectile) |
| 0700000a Tower B | = 09 cold: 0x34 captain, 0x56 + 0x58 loadout |
| 0700000b Tower C | + **0x2e Scorpion** pickup ×1 rounds 8 @ (−22.50, 47.96, 29.30) [D], fp/proj local; 0x34 captain; 0x56/0x58 loadout |
| 0700000c Power Station A1 | **0x54** + **0x58** loadout [M case 0c]; 0x58 fp session (09-0b's skin); no grenades |
| 0700000d Power Station A2 | = 0c cold [M case 0d]: 0x54 + 0x58 (0x58 fp session) |
| 07000011 Tower 2 A | **0x3b Satchel** pickup ×1 rounds 5 @ (14.79, 15.11, 5.33) [D], fp/proj local; **0x34** pickups ×6 rounds 1 @ (14.14-24.53, 15.13/46.78, 4.31-23.80), captain ×1; 0x35 thrown by drones (fp/proj session — 02's archive) |
| 07000012 Tower 2 B | **0x2e** pickups ×2 rounds 5 @ (−0.04, 0.38, −4.06)/(−0.69, 0.36, −3.70); **0x3b** pickup ×1 rounds 1 @ (−31.86, 0.49, −27.44); **0x34** pickups ×3 @ (−14.63…−14.95, 0.07, 11.60-11.86); 0x35/0x36/0x3b thrown by drones; captain 0x34 |
| 07000013 Tower 2 C | **0x3a Tripbomb** pickups ×3 rounds 1 @ (−0.97/−0.84/−0.39, −369.27, 13.94-14.46) [D], fp/proj local; 0x34 captain + thrown; 0x35 session |
| 07000014 Evil Base | **0x34 ×5 + 0x37 ×3** loadout [M case 14] and pickups (0x34 ×5 @ (−32.31, 1.38, 30.71-36.29); **0x37 mine** ×3 rounds 5 @ (38.72, 0, 33.60)/(−28.00, 2.69, 6.88)/(33.76, 3.59, −2.98)); **0x3a ×2** pickups rounds 5 @ (−32.19, 1.41, 31.12)/(−32.19, 1.41, 35.86), fp/proj local; 0x54 + 0x56 loadout |
| 07000015 Evil Base B | = 14 kit cold [M case 15 → case 14]: 0x34/0x37/0x54/0x56; 0x34 pickups ×5 @ (57.62-58.39, 40.40, 70.33-70.96); 0x35 thrown (fp/proj LOCAL here) |
| 07000016 Evil Silo | = 14 kit + 0x34 pickups ×6 @ (0.43-0.77, 0.64, −132.56…−133.68); 0x36 + 0x35 thrown (fp/proj local); 0x56 fp session (14/15's skin) |
| 0700001b Space Station D | **nothing tracked** (the 0x33-only kit [M case 18-1b], no captain drop, no placements) |

Read as the main agent's rule: **`--give 0x3a` works natively on 13/14** (and 11/12 for 0x3b, 0b/12
for 0x2e); on every other level the give still draws and throws — the fp/projectile meshes resolve
through the session table on the story path (§2) — but the level archive itself carries no gadget
assets, so a cold PCSX2 boot straight into the level is the case to check first. The safest
always-carried gadgets are 0x35 (captains drop it on 01-08) and 0x34 (09-16/4a).

## 4. The wall spots (tripbomb + satchel)

07000005 is the best level for the throw tests (the gadget playtest level; the `--give 0x3b,0x3a`
workflow from playtest 1015-0046). Headless checks (the test): grounded after 60 ticks
(floor_ny 0.995), a wall hit within 2 m ahead, normal (−0.914, 0, −0.407) — a flat vertical face;
the 48-ray direction fan shows 12 rays hit this one wall and every other direction is clear ≥ 3 m.

- **Spot** `--bond 106.29,24.87,110.25,1.178,-0.166` — standing 1.72 m from the courtyard wall
  (measured: `wall spot: hit at 1.72 m ahead, normal [-0.9135, -0.0, -0.4068]`), clear space in
  front and around. Throw the tripbomb or the satchel here: it must STICK to the wall face
  (fix-satchel-stick-play's verdict: the port's stick arm runs before the bounce arm on any segment
  hit [M grenade.rs:334-349, `Bullet_CollisionHandler` 0x12ce80:255-266]).
- The tripbomb arms its laser (the 0x80000 bit [L record 0x80100]); the satchel just sticks and
  waits for the detonate press.

## 5. What the test pins (and the sisters)

`crates/nf-view/tests/gadget_carry_live.rs`, 6 tests, all green on this branch:

1. `print_the_carry_map` — the data source for §3 (kept: it reprints the map on every run).
2. `loadout_gadget_rows_match_the_initweapon_table` — the tracked subset of the kit per level:
   14 = [0x34, 0x37, 0x54, 0x56], 09 = [0x56, 0x58], 0c = [0x54, 0x58], 05 = [0x54], 01 = [];
   sister: 15's cold load IS the 14 kit [M case 15 → case 14].
3. `captain_grenade_drops_match_the_decompile` — the 0x145bd0 table; sister: the captain grenade is
   never 0x3a/0x3b.
4. `tripbomb_is_absent_where_the_map_says` — 01 and 09 carry no 0x3a from ANY source (the task's
   required sister assertion).
5. `placed_gadget_pickups_match_the_archive_scan` — the §3 pickup rows for 11/13/14/0b pinned
   (counts, ids, rounds, positions); sisters: 05 has no placed gadget pickups, the tripbomb is
   placed on 13/14 only.
6. `the_07000005_wall_spot_holds_for_the_tripbomb_and_the_satchel` — §4's checks through the real
   loader (`load_weapons` + the give path + `step_bond`).

## 6. Live checks for the main agent (PCSX2)

- **Spot 07000014 spawn area, action: open the gadget cycle.** Expected: the cycle lists Frag →
  Remote Mine (→ the kit's camera/decryptor); the tripbomb pickups sit on the wall table at
  (−32.19, 1.41, 31.12)/(−32.19, 1.41, 35.86) [D] — walk there and confirm the models draw.
- **Spot 07000013 tower (y ≈ −369), action: pick up the tripbomb placement at (−0.97, −369.27,
  13.94).** Expected: "Laser Tripbomb" message, the gadget joins the cycle.
- **Spot 0700000b (−22.50, 47.96, 29.30), action: pick up the Scorpion placement.** Expected: the
  AT-600 Scorpion joins the loadout and draws (fp skin local to 0b).
- **Spot 07000005 `--bond 106.29,24.87,110.25,1.178,-0.166`, action: `--give 0x3a`, throw at the
  wall, then `--give 0x3b`, throw.** Expected: both STICK to the wall face (normal (−0.91, 0,
  −0.41)); the tripbomb's laser beam arms. (fix-satchel-stick-play's live checks cover the rest.)
- **Spot 07000005, action: `--give 0x2e` (the Scorpion — NOT carried by 05).** Expected per the
  map: the give draws it only if the session merged the fp skin from an earlier archive — a cold
  boot straight into 05 is the case of interest; report whether the Scorpion draws and fires.
- **nf-view `--bond` spots**: the same 05 spot for the headless stick check (the test runs it);
  07000011 (14.79, 15.11, 5.33) — the satchel pickup draws; 07000012 (−31.86, 0.49, −27.44) — the
  satchel pickup draws.

## 7. What's left

- The MP levels (07000023-29/4b/4c…) are outside the 24-record mission table (mission::LEVELS) and
  outside this task's on-foot story scope; their `--give` story differs (the default give-everything
  kit carries every tracked gadget [M 0x1a7ef8 default]).
- The sounds the gadgets play (the stick beep 0x446 [M], the pickup 0xF5) were not per-level
  catalogued — the task's scope was the meshes.
- `GT_Update` / `Player_CheckForDeath` pickups (G-029, docs/systems/pickups.md) were not read — no
  evidence they create tracked-gadget pickups; the placement + script + drop sources cover the
  story levels' data (the map above is complete for them).
