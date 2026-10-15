# fix-start-loadouts — every level's start loadout from the game

Branch `s/fix-start-loadouts`. The port used to give nearly every story level the same owned set
`[1, 2, 4a, 4e, 50, 54]` (the 07000005 kit) with the P2K drawn. The game sets a different kit per
level in `Player_InitWeapon` (0x1a7ef8) — a jump table on the level id (`sltiu v1,a0,74` at
001a8268, ids 0x0700001..0x070004a, anything else the give-everything default) — and carries
inventory between a mission's levels through `Player_RamSave`/`Player_RamLoad` (0x195670/0x1957a8;
`IsWarmReset == 0` skips the whole RamLoad body, so a cold load runs the level's fresh case).

## What's done

`init_level_loadout` (crates/nf-game/src/player.rs) now ports the whole switch:

- **Every fresh case's equip list with its round counts** (weapons, gadgets, clips and spare ammo
  through `Player_EquipWeapon` 0x1a8a58: a fresh own fills the clip to the record's size, the extra
  rounds carry, capped at the ammo type's max [M 0x1a8d18:69-78]).
- **The held weapon** — the request byte the game writes to `M_CONTROL+0x63` (the draw machine
  holsters and draws it).
- **The RamLoad cases** (02-04, 08, 0d, 12/13/4a, 15-17): the carry is restored and the saved weapon
  re-selected; each has its continuation gate reading the previous level's key weapon
  (`owned[2]` for 0d [M 001a82a0-1a82b4], `owned[0x10]` for 12/13/4a, the P2K for 02-04/08/15-17);
  unowned → the previous level's fresh kit re-runs (a cold 15 IS the 14 kit).
- **The tail**: laser 0x4e, fists with 0 rounds, `Player_CheckWeaponsLoaded` (0x1a97c8 — a selectable
  weapon whose first-person skin the hashtable misses loses owned), the request falling to fists when
  its base is unowned [M 001a89c4-89fc], the re-save.
- **The 60 Hz pick** in case 01: `movz s4,v0,a0` at 001a8244 selects 0x5b (NTSC) over 0x59 (PAL).
- The fp-skin refusal (`Player_EquipWeapon` 0x1a8a58:20) is what keeps the Stunner out of
  14/15/16's gadget cycles — the equip is there, the archive just doesn't hold the skin; the port's
  skin census is the same check [D].

## Table: level → the game's loadout (now ported) vs the port before

| level | the game's loadout [M 0x1a7ef8 cases] | the port before |
|---|---|---|
| 07000001 | P2K 48, taser 999, **Shaver 0x5b 999** + tail; held P2K | the 05 kit (no Shaver) |
| 07000002-04 | RamLoad carry; no P2K → the fresh 01 kit; held = saved | the 05 kit, held P2K, no carry |
| 07000005 | P2K 48, taser 999, grapple 0, camera 0 + tail; held P2K | (already right) |
| 07000006 | RamLoad; no P2K → the fresh 05 kit | (already right) |
| 07000007 | RamLoad, armour 0, P2K 48/taser/laser/watch 0; **held fists (disguise)** | held P2K |
| 07000008 | RamLoad; no P2K → the 07 kit | the 05 kit |
| 07000009 | P2K 48, taser, decryptor 0, **Q-Worm 0x58 0**, grapple 0, **dart gun 0x43 999**, Shaver 999; **held dart gun** | the 05 kit |
| 0700000c | P2K 48, **sniper 0x1e 999**, taser 999, Q-Worm/camera/grapple 0; **held sniper** | the 05 kit |
| 0700000d | RamLoad; `owned[2]` gate → the fresh 0c kit | the 05 kit |
| 07000011 | **0x10 with 7 rounds**, grapple 0 — no P2K; **held 0x10** | the 05 kit |
| 07000012/13/4a | RamLoad; `owned[0x10]` gate → the fresh 11 kit | the 05 kit |
| 07000014 | P2K 48, **crossbow 0x11 12 rounds**, frag 5, mine 5, taser 999, grapple/camera/decryptor 0; **held crossbow** [L the 2026-10-14 measurement] | the 05 kit, no crossbow, held P2K |
| 07000015-17 | RamLoad; no P2K → the fresh 14 kit (a cold 15 starts crossbow-in-hand; the gadget cycle Laser → Micro-Camera → Decryptor, no stunner [L]) | the 05 kit with the stunner in the cycle |
| 07000018-1b | **0x33 with 999 rounds only**, armour 50.0; held 0x33 | the 05 kit, armour 0 |
| non-story default | give-everything list ending 0x16 999; held P2K [M 001a897c/001a8990] | the 05 kit |

Cases 0x0700000a/0b and 0x07000019/1a/1b's warm forms sit inside the same arms (0a/0b → the 09
fallback, 19/1a/1b → the 18 fresh kit) — the decompile groups them; the port follows.

## How it was verified

- **Fail-first shown** (this session, re-run on the pre-fix tree 366eb7b2^ with the test file copied
  over): `fix_start_loadouts_live` — **5 failed / 1 passed** (07000005 alone passed, the level the
  port already had right). On the fixed tree: **6 passed / 0 failed**:
  the 07000014 crossbow kit (owned set, clips, spares, held 0x11, the stunner NOT owned — sister:
  its fp skin hash is genuinely absent from the loaded skins); the cold 15 fallback (held 0x11, same
  kit); the warm 15 carry (the 14-exit inventory rides the RamSave, the saved weapon re-selected, the
  grapple legitimately revoked by CheckWeaponsLoaded); the 05 measured-set pin; the 01 kit (the 05
  kit's grapple/camera must NOT be owned); the default give-everything pin (owned = the list's bases
  and nothing else, EquipAmmo caps at the ammo maxes).
- **Full suite** on the final tree (origin/master 78b49b8f merged): the fast runner's build +
  run phase — **317/317 test binaries: 1238 passed, 7 failed, 66 ignored**. The 7 failures: the 3
  `disc_golden` tests (the allowed slim-ISO set: bigf_header/bigf_tables/directory_tree),
  `probe_pylon_render` + `fix_0c_pylon_live` ×2 (wgpu `adapter: NotFound` — this sandbox has no GPU),
  and `casings16_live::walking_backwards_never_ejects` (below). The runner was driven in foreground
  chunks because this sandbox reaps background jobs between tool calls (nohup and setsid both died
  silently twice); the chunk driver mirrors suite.py's per-binary env/args/limit exactly (scratch,
  uncommitted).
- **`models_weapons_live`'s golden re-pinned** (4/4 green after): the proj/proj_miss lanes count the
  START LOADOUT's own projectile meshes, so every story level's proj grew with the corrected kits
  (05: 0→10, 14: 7→18, …). The misses are kit meshes the session union has not loaded; the game's
  `Bullet_update` skips those draws too (the `iVar14 == 0` arm [M 0x12e7b0]).

## What's left (known issue, out of my file set)

`casings16_live::walking_backwards_never_ejects` fails at its burst premise (`fired >= 1`): the test
fired the OLD loadout's P2K on a cold 07000016; the game's cold 16 is the 14 kit with the crossbow in
hand (a bolt weapon ejects no casing — probe: request 17, owned {1, 2, 0x11, 0x34, 0x37, 0x4e, 0x50,
0x54}). The re-pin attempt (switch to the kit's own P2K via the real number-key input; owned[2] and
its fp skin verified present) exposed a deeper, PRE-EXISTING draw-machine issue: the switch STARTS
(weapon 2, state 1) and then the machine re-selects the loadout weapon (back to 0x11, state 3) — the
loadout draw's saved-weapon re-select (`Player_SetWeaponAnimObj` 0x1a2e38's idle path re-running
`Player_WeaponSelect`) fights a manual switch made during/after the initial draw. I reverted my test
edit rather than half-land it; this needs the 0x1a2e38/0x1a2cc0 decompile read and belongs to the
draw-machine owner (probe scratch `zz_loadout_switch_probe.rs`, uncommitted).

## Live checks for the main agent

PCSX2 (fresh save, the L2 weapon-cycle layout from the 2026-10-14 measurements):

1. **07000014 cold boot**: in the crawl vent, expected: the Delta Repeater crossbow in hand, HUD
   "3-9 Single"; cycle □-free: the owned set should be P2K/crossbow/frags/mines (+ the gadgets the
   archive holds; no stunner in the gadget cycle). Watch `M_CONTROL+0x63` (0x2b9440+0x63): write 0x11
   at init.
2. **07000015 cold boot**: gadget cycle G: Laser → Micro-Camera → Decryptor (no stunner); the P2K and
   crossbow owned; held = crossbow 0x11 (the fresh-14 fallback — check +0x63 = 0x11).
3. **07000015 warm transition from 14** (finish 14 normally): the 14-exit inventory carries (the
   crossbow's clip count survives; the saved weapon re-draws).
4. **07000001 cold boot**: the gadget cycle should include the Shaver 0x5b (999) — the 50 Hz build
   would pick 0x59 instead; NTSC expects 0x5b [M 001a8244].
5. **07000007 cold boot**: Bond holds FISTS (the disguise) — no auto-draw of the P2K.
6. **07000018 cold boot**: held 0x33 with 999 rounds, armour 50.0, nothing else owned.
7. **The draw-machine re-select bug** (the known issue): on 14, after the crossbow finishes drawing,
   press the P2K's number key — if the view snaps BACK to the crossbow, the 0x1a2e38 re-select fires
   in the real game too (then it's a port-faithful behavior and casings16's premise needs a different
   anchor); if the game STAYS on the P2K, the port's re-select is the bug to fix.

nf-view (`--bond x,y,z,yaw,pitch`, after `--level`):

- 07000014 spawn vent: crossbow view model up, 3 bolts; `--bond 38.7,0,33.6,0,0` for the mine pickup
  corner (the mine HUD count 5).
- 07000015 spawn: gadget HUD shows the Laser first in the cycle.
- 07000001 spawn: the Shaver in the gadget cycle; the P2K drawn.

## Files

- `crates/nf-game/src/player.rs` — the switch (task-named file).
- `crates/nf-view/tests/fix_start_loadouts_live.rs` — new, the 6 NF_ISO tests.
- `crates/nf-view/tests/models_weapons_live.rs` — the golden re-pin (the proj lanes are
  loadout-driven).
- `pickup.rs`/`progress.rs` untouched; no shared file (drone/scene/lib) touched.
