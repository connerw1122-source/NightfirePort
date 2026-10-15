# `BOT_vars_t` — the MP-bot instance record (0x780 bytes × 4 @ 0x26D660)

Proof levels: see [README.md](README.md). Evidence tags: [M] = decompile (`work/decomp/ACTION.ELF/`,
function address = filename prefix, `:NN` = line), [L] = EE snapshot read (`work/ee_snapshot.bin`,
file offset == EE address), [S] = suggested. Worker task `mp-bots-data` 2026-10-15.

ELF symbols [M-sym]: `BOT_vars` @0x26D660, size 7680 = 4 × 0x780 (player slots 4..7, slot−4 =
index). An MP bot is an ordinary campaign Drone (`Drone_tag` = obj+0xE0/0x110, see
[drone.md](drone.md)); `BOT_init` 0x1249e8 memsets the slot, then links
`Drone+0xD1C → &BOT_vars[slot−4]*` and `BOT_vars+0x754 → Drone`. Every access below is through
one of those two directions. Immediately past the array (0x26F460, i.e. `BOT_vars + 0x1E00`)
sits the 0x37-byte state→type table read by `BOTSTATE_getStateType` 0x1291f0 — see
`crates/nf-game/src/mp/bots.rs` for the lifted copy.

Method: mechanical census (every `BOT_vars[...]` global reference — 19 files — plus every
`Drone+0xD1C` deref chain — 81 files reference `+0xd1c`, 59 carry BOT_vars-range offsets on the
deref), then every candidate row was verified by reading the citing function. The census
aliases two same-shaped structs: `Drone_tag` offsets and `MP_PICKUP` offsets appear on the same
lines; rows below only include offsets whose base is provably the BOT_vars pointer in that
function. Where readers disagree or the decompile is unclear, the row says so.

## The config-record copies (stats block, +0xA0..0xAD)

`BOT_init` 0x1249e8 copies the 0xE-byte stats subset of the bot's config record
(`0x2DEEAA + (slot−4)*0x12`, see `crates/nf-game/src/mp/bots.rs` for the record layout and
boot bytes) into `BOT_vars+0xA0` (`amemcpy(0x26d700 + i*0x780, rec, 0xE)`).

| off | type | meaning | proof |
|---|---|---|---|
| +0xA0 | u8 | aggression copy (`BOT_setDroneStats` 0x126318 also copies the record into Drone+0xB4; the float `(f32)aggression` → Drone+0xC0) | [M*] |
| +0xA2 | u8 | aim copy (→ Drone+0xB5) | [M*] |
| +0xA4 | u16 | max-HP copy — `BOTSTATE_setGoalPickPrefs` 0x128298 divides it by `Drone+0xAC` health to weight the health pickup class | [M*] |
| +0xA6 | u8 | speed level copy (→ Drone+0xB6; note Drone+0xB7 := 3 hardcoded in 0x126318) | [M*] |
| +0xA7/+0xA8 | u8 | recovery A/B copies (→ Drone+0xB8/+0xB9) | [M*] |
| +0xA9 | u8 | stats[9] copy (→ Drone+0xBA) | [M*] |
| +0xAA | u8 | preferred weapon class copy — no direct reader found in the bot family [S: consumed via the record by the menus] | [M1] |
| +0xAB | u8 | **trait** copy — `BOTSTATE_getPreferredTraitOpponentObjIndex` 0x128b50 switches on it; also read by `BOTSTATE_processGoals` 0x127c78 (melee gate `== 2`) and `BOTSTATE_combatWeaponChangeChoice` 0x128708 | [M*] |
| +0xAC | u8 | stats **flags**: bit4 0x10 = HP regen (0x1249e8:67 arms the +0x738 timer), bit3 0x08 = 360° vision (atlas §4; see the +0xAC alias warning below) | [M*] |

⚠️ **+0xAC alias trap**: `Drone_tag+0xAC` is the health f32 (`setGoalPickPrefs` reads
`*(float*)(drone+0xAC)`), while `BOT_vars+0xAC` is the flags byte. Same offset, different
structs — always check which base the decompile line derefs.

## The two goal slots (+0x000..0x4F, +0x050..0x9F)

Two 0x50-byte goal slots (`BOTSTATE_initGoal` 0x1294c0 indexes `slot * 0x50`; slot 1 is the
mode-objective slot — `BOT_init` 0x1249e8:113-119 sets slot1+0x99 = 1, slot0+0x49 = 0). All
offsets below are goal-relative; add 0x00 or 0x50.

| off | type | meaning | proof |
|---|---|---|---|
| +0x00..0x1F | 0x20 bytes | `CelPos_tag` copy (goal position; `initGoal` copies celpos+0x00..0x1F, or zeroes +0x10..0x1F when the celpos is null) | [M 0x1294c0:24-38] |
| +0x20 | u32 | runtime scratch — cleared by `uninitGoal` 0x129438; `BotGlobal` msg 0x45 (missile opponent) zeroes the *active* goal's +0x20 | [M*] |
| +0x24 | f32 | goal start timestamp — `processGoals` 0x127c78 expires the goal when `MPGame+0x412 <= +0x24 + +0x28` | [M 0x127c78:71] |
| +0x28 | f32 | goal duration — `gotoGoal` 0x1278a8:50,58 writes `VIDEO_FRAME_RATE * 5` (= 300.0 at 60 Hz) against the *seconds* clock of `MPGame+0x412`, so the validity window is ~300 s, not the atlas's "5 s" gloss (5 s × frame-rate vs a seconds clock — original-quirk-or-bug undecided, live check below re-pinned to 300.0) | [M 0x1278a8:50,58; reader 0x127c78:70] |
| +0x2C | f32 | pickup-class-3 weight (health), clamped ≤ 25.0 (`setGoalPickPrefs` writes params 1..3) | [M*] |
| +0x30 | f32 | pickup-class-1 weight (ammo) | [M*] |
| +0x34 | f32 | pickup-class-0 weight (weapons) | [M*] |
| +0x38 | u32/f32 | written by `setGoalPickPrefs` param 4; **read as f32** by `pickGoal` (the objective score multiplier, `pickGoal` 0x126b88:144,292 — BotIdle writes 0x3f800000 = 1.0) | [M*] |
| +0x3C | u32 | target object: type 1 → MP_PICKUP, type 2 → MP objective obj, type 3 → hunted player obj (`processGoals` 0x127c78:60-77 dispatches on +0x45) | [M*] |
| +0x40 | u32 | state to switch to when the goal fires (0xF9 goto / 0xF7 guard-friend — `processGoals` writes both) | [M 0x127c78:100,113] |
| +0x44 | u8 | flags: bit0 = complete (`setGoalComplete` 0x129510 sets; `uninitGoal` clears) | [M*] |
| +0x45 | u8 | goal type: 1 pickup, 2 objective, 3 hunt player; 0 = empty (`processGoals` treats 0 as "no goal" and clears +0x765) | [M*] |
| +0x46 | u8 | pickup pref byte (`setGoalPickPrefs` param 8) | [M1] |
| +0x47 | s8 | pickup pref s8 (param 7; 0 is stored as −2, and a nonzero +0x49 turns the return true) | [M 0x128298:70-77] |
| +0x48 | u8 | cleared by `uninitGoal` — purpose [S] | [M1] |
| +0x49 | u8 | slot-active flag (slot1 := 1 at init; read by `setGoalPickPrefs`) | [M*] |
| +0x4A | u8 | subtype: 1 flag … 8 demo/protect, 9 hunt (atlas §4); `uninitGoal` sets `BOT_vars+0x734` bit2 when the dying goal's subtype is 1 | [M 0x129438:11-19] |

## Field map (+0xB0..0x77F)

| off | type | meaning | proof |
|---|---|---|---|
| +0xB0+i*0x10 (i<8) | 8 × 0x10 | **per-player info block** filled by `BOT_setOtherPlayerInfo` 0x125390 from `MPGame+slot*0x30+0x1C`: +0x00/? , +0x04 f32 sq-dist to the bot, +0x08/?, +0x0C u32 flags — bit1 0x2 = candidate (obj alive, not self, not 0x11/0x12), bit3 0x8 = same team as me. Readers: `getPreferredTraitOpponentObjIndex` (trait 2 wants bit3 set — same-team pool; traits 4/8 want it clear — enemies), `processGoals` (dist² < 20.25 melee gate, bits `& 10`) | [M* mechanics; trait-name reading [S]] |
| +0x130..0x13F | vec4 | last-known opponent position, copied from `opponent obj+0x30` by `BOT_opponentTargetting` 0x125b30:79-84 (the aim-sway target) | [M*] |
| +0x144+w*0xC | s16 | loaded-in-clip rounds per weapon slot (`BOTWEAP_AmmoInGun` 0x12ae40, `_ReloadAmmoType` 0x129d28:37, `_decrRounds` 0x12a470:22-28, `_EquipWeapon` 0x12a138:38) | [M*] |
| +0x146+w*0xC | u8 | "weapon present in slot" flag (`EquipWeapon` 0x12a138:20-22; `Pickup_Handler` / `MP_BluePrintReachedBase` read it) | [M*] |
| +0x6DC..0x71B | u32[16] | **opponent-history ring** (`BOT_handleOpponentHistory` 0x1257c0:66-110): entries = opponent obj ptrs; dead entries and duplicates zeroed; the current target (`Drone+0x170`) counts ×5; winner = max count | [M*] |
| +0x71C/0x720/0x724 | u32×3 | combat-range triplet, saved/restored with `Drone+0xEC/0xF0/0xFC` (`BOT_postLoadInit` 0x1262b8:11-13 copies Drone→BOT_vars, `defaultCombatRange` 0x129b98:9-11 copies back) | [M*] |
| +0x728 | f32 | distraction accumulator (`increaseDistraction` 0x129bb8 adds while the current state is 0xEB and a goal is active; `isDistracted` 0x129c58 compares against `MPGame+0x20` threshold) | [M*] |
| +0x72C | u32 | last requested bot state (`BOTSTATE_setStateChange` 0x129ce8 := the SetState arg; read by `BotGotoGoalPosition` 0x171240) | [M*] |
| +0x730 | u32 | pending-state scratch (fellOutMap := 0xF9) | [M1] |
| +0x734 | u32 | bot flag word: **bit1 0x2 = recovery pending** (`startRecovery` 0x1299d0 sets with the +0x748 deadline), **bit2 0x4 = flag-carrier goal active** (`uninitGoal` sets/clears on subtype 1); `fellOutMap` zeroes the word | [M* for bits 1/2; other bits unwritten in the readers found] |
| +0x738 | u32 | HP-regen next-tick deadline (`BOT_init` 0x1249e8:67: `GameState+52 + FRAME_RATE_INT` when stats flags & 0x10) | [M1 writer] |
| +0x73C | u32 | pain scratch (`BOT_handlePain` 0x125ed8 stores + clears; `BOT_soundEffect` 0x126618 reads) | [M1] |
| +0x740 | u32 | GameState timestamp stamp (`BotGlobal` 0x16E060:524,533,542) | [M1] |
| +0x748 | u32 | recovery deadline (`startRecovery` := now + delay, gated by +0x734 bit1) | [M*] |
| +0x74C | u32 | last ammo-decrement time (`BOTWEAP_decrRounds` 0x12a470:31 — the fire-rate gate vs GameState) | [M1] |
| +0x750 | u32 | pointer to this slot's `MPSettings+slot*0x30` record (`BOT_init` 0x1249e8:79; `BOT_reactToDroneAlertMsg` 0x1263c0 reads) | [M*] |
| +0x754 | u32 | pointer to the owner `Drone_tag` (`BOT_init` 0x1249e8:85; `gotoGoal`/`processGoals`/`listHeldLoadedWeapons`/`uninitGoal` deref it for the +0xD1C round trip) | [M*] |
| +0x758 | u32 | preferred-opponent obj ptr (`getPreferredTraitOpponentObjIndex` case 2 / `processGoals` hunt target / `BotGlobal` 0x2E, 0x43 clear it) | [M*] |
| +0x75C | i16 | player slot (4..7) (`BOT_init` 0x1249e8:81; `BOT_respawn`/`SetHealth`/`fellOutMap`/`BotGlobal` 0x2E read) | [M*] |
| +0x75E | i16 | bot index (slot−4) (`BOT_init` 0x1249e8:80; `BOTSTATE_setPickupVisitTime` 0x129958 indexes `MP_PICKUP+0x80 + idx*4`) | [M*] |
| +0x760 | u16 | nearest-nav-node id, 0xFFFF = invalid (`setNearestNavNode` 0x129420 := 0xFFFF; `fellOutMap` same) | [M*] |
| +0x762 | u16 | state mirror — written 0xF9 by `fellOutMap`; read by `Drone_SM_SetState` 0x1758c0:50 (0 means "no state yet") | [M1 reader + 1 writer] |
| +0x764 | u8 | character index (from record+0x10; `BOT_setGender` 0x126718 reads) | [M*] |
| +0x765 | u8 | **active goal index** (0xFF none): `getActiveGoal` 0x129548 returns `slot + idx*0x50`; written by `processGoals`, cleared by `uninitGoal`/`fellOutMap`; 0xFF also gates `increaseDistraction` | [M*] |
| +0x766 | u8 | type of the last requested state (`BotInit` := 0 before 0xF9; `BotGlobal` 0x2E := `getStateType(state)`; `fellOutMap` := 9) | [M*] |
| +0x767 | u8 | the round-robin vision cursor: `setOtherPlayerInfo` 0x125390:154,171,174 advances it (one candidate per call gets the `CanSeeObject` query; the +0xB0 scan skips self, resets >7 to 0) — the earlier "no writer" note is closed | [M reader+writer] |
| +0x768 | u8 | **current weapon id** (index into `weapon_data`, 0x10C stride): `ReloadAmmoType` 0x129d28:14, `setGoalPickPrefs` 0x128298:34, `changeWeapon`, `PostLoad_Init`, `BotGlobal` 0x45 (`== 0x3B` close-combat); `BOT_init` copies it to `Drone+0xC58` | [M*] |
| +0x769 | u8 | **armour pool** — `BOT_handlePain` 0x125ed8 subtracts absorbed damage; `Pickup_Handler` adds. (`BOTWEAP_setDroneArmour` 0x12af70 is empty — the atlas's suspicion confirmed) | [M*] |
| +0x76A | u8 | announced held weapon (`BotGlobal` msg 0x42 := msg byte; the BOTWEAP layer's team radio) | [M1] |
| +0x76B | s8 | grudge slot: killer's player index or 0xFF (`BOT_init` := 0xFF; `FindOpponent` 0x1424e0:280 requires −1 to consider others; `BotGlobal` 0x43 clears when the killer dies) | [M*] |
| +0x76C | u8 | opponent-history head index (`handleOpponentHistory` 0x1257c0:64-69 wraps at 16; `fellOutMap` := 0) | [M*] |
| +0x76D | u8 | cleared by `fellOutMap` — purpose [S] | [M1] |
| +0x76E | s8 | the last chosen pickup index: `pickGoal` 0x126b88:496 writes the winner, :397 skips it on the next scan; `BOT_init` := 0xFF | [M reader+writer] |
| +0x76F | u8 | `BotIdle` 0x171b80:96 := 1 when the +0x771 gate passes | [M1] |
| +0x770 | u8 | player-scan flag (`setOtherPlayerInfo` 0x125390:35,61 := 0/1 — set when the scan finds a live candidate; `fellOutMap` clears) | [M1] |
| +0x771 | u8 | attack re-entry latch (`BotIdle` 0x171b80:67,94 sets/tests; `BotGlobal` 0x45 → SetState(0xCF) when set) | [M*] |
| +0x772..0x77F | — | no access found in the decompile | [M-census] |

## Coverage

Census: 19 files touch the `BOT_vars` global; 81 files deref `Drone+0xD1C`; 59 carry
BOT_vars-range offsets through the deref. Mapped above: the stats block (9 fields), both goal
slots (17 goal-relative fields), and 36 instance fields ≥ +0xB0. Unmapped: the goal-slot
+0x08/+0x0C bytes inside the CelPos copy (those are `CelPos_tag` fields — documented in
[drone-statemachine.md](drone-statemachine.md)'s AINetwork section) and the +0xB0 block's
+0x00/+0x08 words (never read individually — only the flags word and dist² are consumed).

## Live checks for the main agent

1. **Goal timeout constant**: `gdbwatch` `0x26D660+0x28` (w) — expect **300.0** (`gotoGoal`
   0x1278a8:58 writes `VIDEO_FRAME_RATE * 5`; the write site is now known — re-pinned from the
   original 5.0 expectation).
2. **+0x767 self-index writer**: `gdbwatch` `0x26D660+0x767` (w) — expect a write during
   `BOT_init`/menu prepare with the slot value (4..7 mapped to 0..7?); the decompile only shows
   the reader.
3. **Trait 2 = same-team pool?**: `fieldprobe` poke `BOT_vars+0xAB := 2` on a bot in a *team*
   deathmatch, screenshot — expect the bot shadowing a teammate rather than hunting; then in
   FFA expect no behavior change (bit3 never sets in FFA per `setOtherPlayerInfo`). This
   settles the trait-name table the atlas marks [S].
4. **+0x769 armour absorb**: `fieldprobe` poke `BOT_vars+0x769 := 100` then shoot the bot —
   expect the visible HP bar to drop slower and +0x769 to decrease by the absorbed amount.
5. **Pickup visit deadline**: `gdbwatch` `MP_PICKUP+0x80 + botidx*4` (w) when a bot grabs a
   pickup — expect `now + 45.0` (atlas §4's 45 s cooldown; the write is
   `BOTSTATE_setPickupVisitTime` 0x129958:24).

Note: the atlas (docs/atlas/multiplayer.md §4) says the per-bot visit floats sit at
`MP_PICKUP+0x70`; the decompile writes `+0x80 + idx*4` [M 0x129958:24] — the atlas row needs
the +0x80 correction (applied to the atlas in this branch).
