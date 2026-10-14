# fix-weapon-fx-levels — the weapon effects audit, all 27 levels

Branch `s/fix-weapon-fx-levels` (from origin/master 088bdf4; origin/master merged at 2caa27f).
Builds on 100a's give-path merge (`weapon_assets.rs`) and 100b's spare banks, per the task.
New test file: `crates/nf-view/tests/weapon_fx_levels_live.rs`.

## What the audit found

**The game's mechanism is per-level redundancy, not a shared boot pack.** `AnimLoadFile`
0x1bdf40 accumulates per-hash FILES.BIN archives into never-freed session tables [M], but the
FX assets themselves ship redundantly in every story archive [D — a full-disc census over the
FILES.BIN archives, hash-by-hash]:

- the per-material impact set (the `EffectInfo` 0x3037c0 decal meshes 0x02000122/126/127/128/262/6fd,
  the debris 0x0200012a-d/1fe/1ff/200/220, the gas 0x02000039/124/129) — in ALL 20 story archives;
- the tracer 0x02000123 (what `Bullet_update` 0x12e7b0 attaches to mesh-less bullets) — all 20;
- the blast one-shot script 0x06000052 (`Explode_Create` 0x17cbd0's kind-7 script) — all 20 (it
  lives in the kind-7 NIS table, NOT the kind-5 script set — the audit reads `level.nis`);
- the projectile bodies ride 100a's `+0x5C` merge; the impact sounds ride 100b's spare banks.
- `LoaderLoad` 0x1d4a00 + `LoaderProcess` 0x1d4880 dispatch (0/2/0xb → maps, 3/4/5 → AnimLoadFile,
  6 → skeleton, 7 → Script_Load 0x1d8248); `CommonAnimFile` (0x30d860) is 0 at every level-start
  snapshot [L] — no shared FX source there either.

**Three genuine gaps, all fixed here:**

1. **The fp muzzle-flash mesh `+0x48` was never wanted by the merge** — `wanted_weapon_models`
   asked for `+0x80/+0x4C/+0x5C` + sleeves, so a merged weapon built its view model but
   `Scene::build`'s `by_hash` lookup for `+0x48` (the attach `Draw_MuzzleFlash` 0x1a3fe0 makes,
   gated by `Player_MuzzleFlash` 0x1a4298 on `+0x48 != 0` [M]) silently missed. Disc census [D]:
   every record's `+0x48` resolves everywhere EXCEPT 0x02000ac7 (records 0x32/0x33/0x6f), which
   lives only in 07000014/15/16/1b. **Fix:** `+0x48` joined the wanted offsets (lib.rs), so both
   the session merge and the 100a surgical merge pull its map.
2. **The default shell casing mesh 0x02000548** (ammo 9 — records 0x1c/0x1d, the shotgun) is
   absent from 07000005-0d/14-16/1b [D]; `Bullet_init_casing` 0x12fde0 attaches
   `ammo_data[weapon+0x90]+4` [M], so the casing was simulated and audible but never drawn on a
   foreign level (docs/atlas/effect.md:101 had noted the idea). **Fix:** the `AmmoData.casing_mesh`
   list joined the wanted set (`casing_meshes()` off the same ELF the merge already parses).
3. **The projectile trails were never ported** — `Bullet_DoTrails` 0x12d890 (from `Bullet_update`
   0x12e7b0:375/475) runs per flying bullet; the port spawned nothing. Ported into the grenade
   path (all bit-0x2/0x20 records are gravity bullets, `+0x6C & 8` [L ee_start_05 @ 0x2bf150
   sweep]): the smoke trail behind the AR-GL 0x1b / stun 0x36 / 0x69 (bit 0x2 →
   `Bullet_trail_effect_smoke` 0x12cba8: every 4th tick `Gas_Init(0.1→0.5, vel 0, mesh
   0x0200001f, kind 7, 0x404040ff→0x40404001, life 120)` — the decomp-dropped 11th arg read off
   the disasm 0x12cc60 = 0x78; the non-NULL zero velocity skips the wind draws [M 0x1aacc8]) and
   the MGL 0x2a/0x2b exhaust puffs (bit 0x20, every 3rd tick, `Gas_Init(0.1→0.4, vel (0,0,rand
   6.25), SMOKE_QUAD, kind 0, 0x7f7f7f7f→0x80808000, life 64)` [M disasm 0x12da34-84]). 3 shared
   RNG draws per puff (1 caller + 2 spin), the searchlight precedent.

**Documented, not ported:** the bit-0x10000 **Swoosh ribbon** (`Swoosh_Create` 0x1a4e40, the
Sentinel/Scorpion/Torpedo rocket flame trail: a ribbon object fed the bullet position each tick,
period = record `+0x74` i16, width `+0x76`, mesh 0x20005cd for records 0x33/0x6a/0x6e else
0x2000481 [M decomp :202-236]) — a new render system, not an asset gap; the bit-0x80000 alt puff
(record 0x41) likewise rides the same `Gas_Init` machinery and can follow the same pattern.
The MPSettings trail-detail gate (word 384) reads 0 on this build [L .bss; LS_LoadMPSettings
0x1ce5f0 never sets it].

## The table

`every_story_level_resolves_every_weapons_fx` prints the 20-row × 114-record mark table (the
runner's static model of the real give path's pool: own archive ∪ earlier story archives
(`merge_weapon_models`' STORY_ORDER reversed, `code < level`) ∪ the skin's owning archive from
the 100a DiscIndex; sounds via the level's own bank ∪ every spare):

```
07000001    ........................................................hn.............nnn.....................nnnnnnnnnnn.n....nn
07000002..0700001b — identical shape (the 'n'/'h' columns only; rows omitted, the test prints all 20)
```

- `.` clean: every FX asset the weapon's records and view scripts name resolves — the fp muzzle
  flash, the third-person flash, the projectile body, the held model, the casing, the tracer, the
  blast script, and every fire/reload/draw op-1 sound id (op-1 = frame, id [M sound.md]).
- `n` (columns 57, 75-77, 95-104 …): the records with no fp skin on the disc — 100a's give
  baseline, not an FX gap.
- The ONE residual class: two records whose `+0x80` held-model hash lives only in non-story
  archives (the test prints the ids: weapons 0x39 and 0x5f), marked `h`. The give path merges
  their skins/flash/projectile fine; only the world-view held mesh misses on some levels. The
  fix is the same index-based top-up the scripts already have (`merge_weapon_assets`'s
  kind-5 top-up pass extended to map hashes) — listed below.

**Totals: 0 unexplained missing effect cells** across 20 story levels × 114 records (the MP
front-end levels 07000023-29/41/46/48/49/4a/4b are the same mechanism — the census covers all
archives; the story order is the session the game actually builds).

## Verification (reproduce-first)

- The rifle spot check `a_foreign_rifle_fires_with_flash_casing_and_impacts_on_05` (the AR 0x1a
  is foreign on 07000005) — through the real path (`load_weapons` + `equip` + `step_bond` +
  `sound_tick`): the draw sound 0x282 from the level's OWN bank (the sister: native ids never
  leave the primary slot), the muzzle-flash attachment exists (pre-fix it did not — `+0x48`
  wasn't merged), the fire sound 0x40d starts a voice, a live casing ejects (pre-fix: simulated
  but undrawn — ammo 9's mesh missing), and the floor impact spawns the per-material decal/debris
  set. GREEN.
- The static sweep (above) — GREEN with the documented `h` residual.
- `--ignored` × 2 with reasons in their comments:
  - `a_foreign_launcher_leaves_a_smoke_trail_and_explodes_on_02`: the MGL 0x2a equips on
    07000002 (gun.weapon = 42) but a held trigger never releases a round headless (shot_tick
    never moves, no grenade spawns) — tried 160 held ticks after an 80-tick draw wait, with and
    without `aiming`, through `Sim::step_bond` only. The same "foreign weapon's held fire spawns
    no round" class fix-weapon-sounds saw on 07000001. The exhaust-puff arm itself is exercised
    every grenade tick in `Drones::tick`.
  - `a_foreign_gadget_laser_burns_with_puffs_and_no_decal_on_01`: the laser given on 07000001
    never spawns its burn puffs headless (60 held ticks; decals stayed empty, the gas pool never
    grew; with/without aiming). The native-level beam behaviour is pinned by fix_laser_beam's
    tests; the foreign-level burn needs the live check below.
- Neighbours (my drone.rs/gas.rs/lib.rs changes): fix_weapon_effects_live 6/6, explode_live 2/2,
  fix_weapons_all_live 4/4, fix_weapon_sounds_live 3/3 (+ its 2 pre-existing ignores).

**The full suite did NOT run in this window** (the sandbox kills background jobs between tool
calls — the runner's own docs recommend background; it dies here — and the 2-hour window closed
during the audit). The five binaries above are the direct neighbours of every file I touched.
The next session should run `NF_ISO=... python3 tools/test/suite.py` on the merged tree.

## Files

- `crates/nf-view/src/lib.rs` — `wanted_weapon_models` gains `+0x48` and the casing meshes (+
  the `casing_meshes()` ELF helper); both merge call sites feed it.
- `crates/nf-view/src/weapon_assets.rs` — `merge_weapon_assets` takes the casing list (one
  param, threaded to `wanted_weapon_models`).
- `crates/nf-game/src/gas.rs` — `init_trail_smoke` + `init_exhaust_puff` (additive methods).
- `crates/nf-game/src/drone.rs` — one additive block in the grenade tick loop (the two trail
  gates reading the weapon's `+0x6C`).
- `crates/nf-view/tests/weapon_fx_levels_live.rs` — new (the sweep + 3 spot checks).
- `crates/nf-view/tests/weapon_assets_live.rs` — one call-site arg (`&[]`) for the new param.
No other shared files touched.

## Live checks for the main agent (PCSX2)

1. The AR on 07000005 via the dev give (`--give 0x1a`): muzzle flash visible on the first shot
   (pre-fix it fired flash-less), casings fly. nf-view render:
   `nf-view <iso> 07000005 --bond 104.95,25.12,108.83,2.0,0.0 --frame 40 --shot fx-a.png` at the
   fix_weapon_effects render spot — expect the flash mesh on the view model + the light.
2. The MGL (0x2a) on its own level or via give: grey exhaust puffs trailing the round every 3rd
   tick [M bit 0x20], vs the port's previous clean flight. Render spot: fire down-range, frame ~10.
3. The AR-GL (0x1b) / stun grenade: the thin grey smoke trail (mesh 0x0200001f puffs, kind 7,
   every 4th tick) behind the round.
4. The Sentinel (0x2c): the game's rocket flame is the SWOOSH ribbon (not ported) — confirm the
   look on PCSX2 so the next session's Swoosh port has the reference.
5. The laser on a foreign level (07000001 give 0x4e): does the game itself spawn the burn puff at
   the wall for a foreign give (it can't give there — the check is whether the port's headless
   fire-path block is a give-merge issue or a fire-machine issue: watch the trigger path with
   gdbwatch on `Player_WeaponFiring` 0x1a5460's release tick).

## Left for the next session

1. The full suite on the merged tree (the window closed; neighbours green).
2. The `+0x80` held-model top-up for weapons 0x39/0x5f (extend the kind-5 top-up pass's pattern
   to map hashes via the DiscIndex).
3. The Swoosh ribbon (bit 0x10000) — the rocket flame trail; `Swoosh_Create` 0x1a4e40 +
   `Bullet_DoTrails`'s per-tick position feed.
4. The headless fire-path block on foreign levels (the launcher/gadget spot checks above).
