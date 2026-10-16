# dev-panel-spawns — the dev panel spawns what it says

Branch `s/dev-panel-spawns` (from origin/master a4156da0; origin/master de6110cc merged before the
final suite). Files touched: `crates/nf-view/src/panel.rs` (the catalog + the give), `crates/nf-view/
src/dev.rs` (stale labels only), new `crates/nf-view/tests/dev_panel_spawns_live.rs`, updated
`crates/nf-view/tests/fix_dev_panel_live.rs` + `models_weapons_live.rs`, `docs/fields/weapon_data.md`
(+0x5C doc fix). Shared files: docs/WORKLOG.md (append-only, conflict resolved keeping master's
entries first).

## Playtests 1012-2011 / 1011-1934, root-caused

Both sessions predate the 273-fix-dev-panel merge (2026-10-13), which already fixed the hardcoded
name table (the game's own `Txt_BindLabel(+0x38)` names) and the consumables ids (the frag is 0x34,
not 0x35). What REMAINED wrong, found by this audit:

1. **"A lot of the spawnable stuff is gadgets" / "I can't tell what's what"** — the catalog iterated
   ids 1..0x72 and kept every named-or-meshed record, so the panel listed **0x62** (the AI rocket
   record), and the **0x68..0x6F drop/AI variants that borrow OTHER weapons' name labels** — 0x69
   binds the Smoke Grenade label, 0x6A/0x6E the Q-Claw's, 0x6D the AT-600 Scorpion's, 0x6F the
   Militek MGL's [L ee_start_05.bin @0x2bf150: +0x38 of each]. The game never equips any of them on
   Bond: they sit outside both of `Player_WeaponChange` 0x1a6f80's cycles — the weapons 0x01..0x46
   (0x47 is "no weapon") and the gadgets 0x4A..0x5E (disasm 0x1a707c `addiu s6,zero,73` low clamp;
   0x1a72ac/b0 `addiu fp,s6,1` / `addiu s7,a2,-1` wrap) [M]. The catalog now lists exactly the two
   cycles (~91 rows; 0x48/0x49 and 0x5F..0x71 are gone).
2. **The gadget flag contradicted the game** — `gadget: id >= 0x48` (panel.rs) vs its own comment
   "ids 0x4A and up [M 0x1a6f80]". Now `gadget: (0x4a..=0x5e).contains(&id)` — the F2 Weapons page
   (all non-gadgets) and the Gadgets page (0x4A..0x5E) group exactly as the game cycles; the
   Grenades page stays the game's four throwables 0x34..0x37 (NDrone2_DefaultInit 0x14b300's
   grenade types 1/2/4 → 0x36/0x35/0x34 + the remote mine 0x37; all class 4 [L +0x06]).
3. **"No clue what the rocket launcher is"** — already fixed by fix-dev-panel (0x2C AT-420 Sentinel,
   0x2E AT-600 Scorpion); this task pins both names + the Sentinel's rocket mesh 0x02000432 [L +0x5C]
   in the new test.
4. **"A frag grenade spawns what looks to be a stun grenade that explodes like a frag grenade"** —
   the id was fix-dev-panel's fix; the LOOK is `weapon_data[fired]+0x5C` (the thrown grenade's drawn
   mesh — `Bullet_update` 0x12e7b0 disasm 0x12e8f0 `lw a1,0x5c(v0)` →
   `hashtable_set_object_to_entity_gfx` [M]). The frag's is 0x020002af, the stun's 0x02000726 [L],
   and the port's throw (scene.rs Grenade { mesh: projectile_mesh(fired) }) already uses the fired
   record's own +0x5C — the new test drives the REAL input path and locks it: the panel-given frag
   flies with 0x020002af, the stun with 0x02000726. (The frag leg runs on 07000015 — the frag's
   throw anim scripts 0x06000703/0x704 live only in some level archives [D scan; fix-weapons-all].)

## The give is the level pickups' path

`panel::apply`'s "weapon" arm used to poke `equip_weapon` + an unconditional `equip_ammo` top-up +
raw twin-slot surgery. It now calls **`Inventory::take`** — the port shape of `Pickup_Handler`
0x18e650, the exact function the level pickups run (drone.rs's take call): the `Upgrade_Weapon` remap
[0x18e650:142 — 6 → 2, 10 → 0xC at slot 0 [L 0x2bee08]; every other id identity], the owned-check
dispatch, the fresh-own full clip + spill [M Player_EquipWeapon 0x1a8a58], the owned-arm
ammo-take with the cap refusal [M Player_EquipAmmo 0x1a8d18], and the `Taken` result (the raw id for
display; the status line says the remap out loud). The clock is the sim tick — the port's stand-in
for `GameState+0x52` (the 10 → 0xC gate). Documented divergences (the panel's acquire affordances,
kept on top of the take and applied to the parity test's reference too): `loaded = true` (a dev give
does not depend on the level's model set — the row's "(no fp model)" tag carries that fact), the
switch request (a pickup only switches on `Player_IsBetterWeapon` 0x1a6eb0), the twin-clip fill
(fix-aims20-modes-lights) and the 3-clip spare carry (a reload right after a give must have spare
rounds — fix_reload_hang's precondition, caught by the suite and restored).

## Mislabels fixed

- dev.rs's dead B-10 scenarios called 0x36 "stun" and promised a smoke explosion: 0x36 is the bank's
  Smoke Grenade and never explodes (`Bullet_DoTrails` 0x12d890's 0x20000 arm) — words corrected, ids
  untouched.
- models_weapons_live.rs's thrown-grenade test called 0x35 "frag" throughout (variable, comments,
  messages) — renamed to the stun it is; the numbers were always right.
- fix_dev_panel_live.rs's header still cited the old atlas claim ("0x35 = Frag Grenade [L+M]") —
  now cites the bank re-read (fix-explosions2).
- docs/fields/weapon_data.md +0x5C "no reader found" → the reader is Bullet_update 0x12e8f0
  (annotated doc-fixed, the fields-player style). docs/port-map/weapon-switching.md:11 has the
  last-weapon/last-gadget stash slots swapped (decomp: BL+0x930 = last weapon, +0x932 = last gadget)
  — NOT touched this task (shared doc, different area), flagged for the next pass.

## Tests (crates/nf-view/tests/dev_panel_spawns_live.rs, NF_ISO, 4 green)

- `panel_entries_match_the_weapon_table` — every catalog row: name == the bank's bind(+0x38), held/
  tp/proj == the record's +0x7C/+0x80/+0x5C, cls == the raw +0x06, gadget == the game's cycle bounds;
  the noise ids (0x48/0x49/0x62/0x68/0x69/0x6A/0x6D/0x6E/0x6F) absent; the four grenades named by the
  bank with cls 4 and their own meshes (0x34→0x020002af, 0x35→0x02000726); the launchers named.
- `the_f2_pages_group_by_the_games_own_cycles` — the F2 Weapons/Gadgets/Grenades pages select by the
  game's own cycles; the grenades page gives exactly [0x34,0x35,0x36,0x37] at ×5.
- `the_panel_give_is_the_level_pickup_take` — the give and a synthesized WEAPON pickup through
  `Inventory::take` land byte-identical owned/clip/carried (for 0x34/0x16/0x2c).
- `the_frag_gives_the_frag_mesh_and_the_stun_the_stun_mesh` — panel-give → real switch → real fire:
  the flying grenade carries the fired id and draws the fired record's +0x5C mesh, frag ≠ stun.

Pre-push runs of the crates the tests touch (nf-view only — nf-game/nf-formats untouched):
dev_panel_spawns_live 4/4, fix_dev_panel_live 6/6, panel_live 2/2, models_weapons_live 4/4,
dev_menu_flow 5/5, fix_reload_hang_live 6/6 (after the carry restore), fix_weapons_all_live,
fix_grenade_release_live, fix_aims20_modes_lights_live — green in the suite (below).

## Suite (final tree = merge of origin/master de6110cc)

The fast runner, chunked foreground in pairs (the sandbox kills lone background jobs and OOM-killed
two parallel heavy chunks): **419/425 binaries completed — 1846 passed, 13 failed, 48 ignored**.
The chunk phase's 13 split three ways: the 3 allowed `disc_golden` (slim ISO), `driving_ai_follow`
(slim ISO data 360/811) and the 3 no-GPU pylon probes (`fix_0c_pylon_live` ×2, `probe_pylon_render`)
are the known environmental set; `fix_reload_hang` ×2 was a REAL regression from the first give cut
(the missing spare carry — fixed, re-run green); `remote_mine_detonate` ×2 + `fix_remote_mine_play` +
`weapon_anims_live::mgl_magazine_reload` were timing flakes under the OOM-collided parallel chunks
(all four binaries green on a clean solo re-run). The 5 binaries the OOM killed mid-run were then
re-run solo on the final tree — ALL GREEN: `weapon_matrix_live` 6/0 (the hit matrix),
`async_level_load_live` 3/0, `fix_aim_raise_live` 5/0, `fix_death_restart_live` 8/0,
`walk_07000013_live` 2/0. **Final-tree tally: 1870 passed, 7 failed, 48 ignored over 424/425
binaries** — the 7 are exactly the known environmental set; `nf_cpd_child` is the cross-process
guard's child runner (no standalone tests), not a skipped failure.

## What's left

1. docs/port-map/weapon-switching.md's swapped stash slots (one-line doc fix).
2. Live checks for the main agent: none required by this task; the standing L-205 item (which
   canister the game draws for 0x34 vs 0x35 in PCSX2) would close the last name doubt visually —
   `gdbwatch` a frag throw on 07000005 and screenshot the in-flight mesh vs the stun's.
