# light-create-census — every Light_Create call site in ACTION.ELF

`Light_Create` is 0x1cc3d8 [M]. Its argument ABI, pinned from the function's own prologue/body
([M 0x1cc3d8 disasm]): `a0` = VECTOR\* position; `a1/a2/a3` = r/g/b bytes (`andi 0xff` each);
`f12` = radius (moved to callee-saved `f20` at 0x1cc43c, stored +0x3C at 0x1cc4d4); `a4($8)` =
channel u16 (+0x22); `a5($9)` = life s16, −1 → 0 = permanent, else arg+1 (+0x20);
`a6($10)` = kind byte (+0x47); `f13` = intensity (→ `f21`, stored +0x40 at 0x1cc4f8);
`a7($11)` = flicker period u16 (+0x24); stack+0 = seed u16 (`Rand_Rand(seed)` is drawn
unconditionally at 0x1cc50c [jal 0x1e3738], `+0x26 = seed>>1`, `+0x28 = seed + drawn`);
stack+8 = flicker mask u16 (+0x2A); stack+16 = sound u32, 0 → 0xffff (+0x2C). Stack slots are
8-byte spaced. This matches the pins already in `docs/WORKLOG.md` (parsemap 0x4E: period→a7,
seed→stack+0, mask→stack+8, sound→stack+0x10).

## The census (all 14 `jal 0x1cc3d8` sites, from a full static scan of ACTION.ELF)

Scanned every word of the single PT_LOAD segment (vaddr 0x107100, 0x206570 bytes) for the `jal`
encoding; 14 sites, 13 owner functions (the `searchlight`-neighbour site the driver expected is
actually inside **Light_SetAmbientRadiators**, and **Searchlight_Create builds its light through
`psiLight_Create` 0x10e788 directly** (jal at 0x1cb88c), not through Light_Create).

| # | site | function | args (radius, intensity, rgb, life, kind, period/seed/mask) | condition | ported? |
|---|------|----------|----------|-----------|---------|
| 1 | 0x12de38 | Bullet_DoTrails 0x12d890 | r 10.0, i 5.0, (255,255,255), life 10→11 ticks, kind 1, all 0 | the flashbang arm (Bullet_DoTrails case 0x12e0c0 → flash) | **yes** — `drone.rs` `lights.create(10.0, 5.0, …, 0, 10, 1, 0)` (the stun/flashbang white light) |
| 2 | 0x12ed48 | Bullet_update 0x12e7b0 | r 4.0, i 2.0, rgb = weapon rec +0x54/55/56, life 0→1 tick, kind 1 | bullet rec `+0x6C & 0x2000` (rockets), every flying tick | **yes** — `drone.rs` `lights.create(4.0, 2.0, b.pos, [w[0x56],…])` with the same `& 0x2000` gate |
| 3 | 0x1328a8 | GunImp_Update 0x1320d8 | r 9.0, i `Rand_FRand(1.0)+1.5` ∈ [1.5,2.5), rgb (0,255,255), life 0→1 tick, kind 1 | mounted-gun fire tick, turret gun type 0x67 (rocket) or 0x70 (grenade); preceded by draws `Rand_Rand(128)` (decal byte) and `Rand_FRand(2π)` (gas dir); fires `Explode_Propagate(4.0, 5.0, …)` at the ray hit when type == 0x67 | **no** — the port has no mounted-gun/turret firing (no GunImp 0x47 objects); see "Remaining" |
| 4 | 0x132b4c | GunImp_Update 0x1320d8 | r 9.0, i `Rand_FRand(1.0)+1.5`, rgb (255,96,0), life 0→1 tick, kind 1 | same, gun type ≠ 0x67/0x70 | **no** — same |
| 5 | 0x179234 | DroneWeap_FireWeapon 0x178f68 | r 7.0, i 1.0, rgb = colour-table at 0x2BF1A4 + 268·(signed byte weapon rec +98): r=tbl[+2], g=255, b=tbl[+0], life 0→1, kind 1 | every AI shot | **yes** — `drone.rs:5499` `lights.create(7.0, 1.0, muzzle, [w[0x56], w[0x55], w[0x54]], 0, 0, 1, 0, rng)`. Deviation flagged: the port reads the weapon record's own +0x54..56 bytes; the game reads the 268-byte colour table indexed by weapon rec +98. Same *shape*, possibly different bytes — needs a live check (below) |
| 6 | 0x19d328 | HUD_UpdateNightSightPane 0x19d218 | r 32.0, i 2.5, (255,255,255), life 0→1, kind 1 | night-sight pane visible (global 0x2a4920 == 0), pos from a per-weapon table | **no** — the port has no night-sight HUD pane re-render; the light only ever lights that pane's re-render. Listed, not ported |
| 7 | 0x1a43a8 | Player_MuzzleFlash 0x1a4298 | r = weapon rec +0x58 (fp_flash_radius, f32), i 2.0, rgb = rec +0x56/55/54, life 0→1, kind 1 | every player shot (both sound branches converge) | **yes** — `nf-view/src/scene.rs:3623` (`radius` from `w[0x58]`, i 2.0, rgb [w[0x56], w[0x55], w[0x54]]) |
| 8 | 0x1a58ec | Player_WeaponFiring 0x1a5460 | r 5.0, i = 2·((charge/falen)·0.75) (the charge gauge), rgb (0,255,255), life 0→1, kind 1 | view-anim state 9 with the charge weapon (charge glow) | **yes** — `nf-view/src/scene.rs:3083` (`create(5.0, 1.5*frame/falen, pos, [0, 0xff, 0xff], …)`) |
| 9 | 0x1a5a8c | Player_WeaponFiring 0x1a5460 | r 1.5, i `Rand_FRand(0.5)+0.5`, rgb (255,127,0), life 0→1, kind 1 | gate reads 0x55 on `s3` + weapon view-anim state 0 | **no** — see "Remaining"; needs the Player_WeaponFiring context around 0x1a5a0c to name the weapon (the branch follows the `0x55` compare at 0x1a5a10) |
| 10 | 0x1ad7ec | Sub_Create 0x1ad6c0 | r 44.5 (0x42340000), i 1.0, rgb (0,255,255), life **−1 permanent**, kind 1 | Sub_Create (creates the class-0x4a "sub" object 0x44; the game's submarine) | **no** — the port has no Sub_Create/0x4a object; the sub levels' ride is not simulated |
| 11 | 0x1c2534 | AnimProcessScriptCmds 0x1c1b38 | r = anim-object rec +88 (f32), i 2.0, rgb = rec +86/85/84, life 3→4 ticks, kind 1 | anim script command with arg at 176($sp) ≠ 0 (a "light" script cmd) | **no** — the port's anim script interpreter (anim.rs) does not handle the light command; listed |
| 12 | 0x1caf7c | Light_SetAmbientRadiators 0x1caeb8 | i 1.0, rgb = ((int)(rec+20/24/28 × 255)) & 0xff, life −1, kind 0, all 0; **radius = `sqrt.s f12, f0` = sqrt(rec+20 × 255)** [M 0x1caf30, raw word 0x460c0304] | loop over block-0x26 radiator records | **yes, with a flagged discrepancy** — `light.rs level_lights` passes `r.radius` from `level.rs`'s parse, which reads **rec+12 and sqrt's it** (comment: "+12 = radius²"). The disasm sqrt's **rec+20 × 255** (the red channel). Both agree only if the data stores +12 == +20·255. Live check requested below; do not change the parse until the data verdict |
| 13 | 0x1d0c00 | parsemap_create_dynamic_objects 0x1d04f0 (case 0x4E) | r = **(float)(unsigned)placement +0x2C**, i 1.0, rgb bytes +0x30/34/38, life −1, kind 0, period +0x40, seed +0x44, mask +0x48, sound +0x4C | type-0x4E placement, flags & 0xa000 == 0 | **yes — FIXED this task**: the port read the radius as float BITS (`f32::from_bits`); the game converts an unsigned int (`lw` → `mtc1` → `cvt.s.w`, with the 0x1d0bb4 bltz arm the uint→float expansion). Old code made every 0x4E radius ~1e-44, so `1/radius²` overflowed and `atten = max(0, 1/d² − 1/r²)` stayed 0 — **every 0x4E dynamic light contributed nothing**. Fixed to `e(2) as f32` in `light.rs placement_light` |
| 14 | 0x1db254 | Script_LightStart 0x1db1b0 | r 255.0 (0x437f), i 1.0, rgb (255, 2, 3), life **−1 permanent**, kind = (3rd script byte == 1); pos = the global VECTOR at 0x2D6200 | NIS script light command; 3 stream bytes (id 0 → none, stored byte, kind flag) | **yes** — `nf-game/src/nis.rs:541` (script light objects with parent pair), census confirms the game's odd rgb (255,2,3) is what the game passes |

## The 08 explosion brightness

No `Explode_*` function calls `Light_Create` (the 14 sites above are exhaustive). The rocket
explosion's light on 08 is the **per-tick rocket light (site 2)** — already ported — plus the
placement/ambient lights. With site 13 fixed, the 0x4E lights actually contribute again; if the
corridor still renders ~2.5× dimmer than PCSX2 after this fix, the next suspect is site 3 (the
turret-rocket's muzzle light and its `Explode_Propagate(4.0, 5.0, …)` path), which needs the
mounted-gun system first.

## What this task changed

- `crates/nf-game/src/light.rs` — `placement_light`: 0x4E radius is `(float)(uint)` not
  `f32::from_bits` [M 0x1d0b8c-0x1d0bd0]. This is the one behavioural fix.
- `crates/nf-game/tests/light_create_census_live.rs` — new opt-in live test: loads 0700000c from
  the disc, walks every type-0x4E placement through the real `placement_light`, and asserts the
  uint radius conversion plus the rgb/channel/period/life/kind pins. Fails against the old
  `from_bits` reading (radius ~1e-44 ≠ raw-as-float). It passes with the fix (the 0c data includes
  a radius-0 placement, hence the `>= 0.0` bound).

## Verification run

- `cargo test --offline --release -j2 -p nf-game --test light_create_census_live` — 1 passed (with `NF_ISO`).
- `cargo test --offline --release -j2 -p nf-game --lib` — 241 passed, 0 failed.
- The full four-crate suite could not be kept alive in this sandbox (background processes are
  killed between tool calls; `nohup`/`setsid` both died), so it was run per-binary. No other
  crates' tests were run this session — the change touches only `nf-game`'s light creation.

## Remaining (not done, next session)

- Sites 3/4 (GunImp_Update turret muzzle lights + the rocket `Explode_Propagate` arm): blocked on
  the port having mounted-gun firing at all.
- Site 6 (HUD night-sight pane light): blocked on the pane.
- Site 9 (Player_WeaponFiring 0x1a5a8c, the (255,127,0) r1.5 light): name the weapon via the
  Player_WeaponFiring decomp around 0x1a5a0c, then port it next to scene.rs's charge glow.
- Site 10 (Sub_Create's permanent cyan r44.5 light): blocked on the sub object.
- Site 11 (AnimProcessScriptCmds light cmd): add the anim-script light command to anim.rs
  (rgb rec+86/85/84, radius rec+88, i 2.0, life 4 ticks, kind 1).
- Site 5's rgb source (colour table 0x2BF1A4 vs weapon rec +0x54..56): resolve with the live
  check below.

## Live checks for the main agent

1. Radiator radius source: gdbwatch (read) the radiator record during 07000005 load — address =
   the block-0x26 payload entry +12 and +20 (EE addr of the parsed block): expected +12 and
   +20·255 equal if the port's parse is coincidentally right; if they differ, `level.rs`'s
   radiator radius must become `sqrt(rec+20 × 255)` [M 0x1caf30].
2. DroneWeap_FireWeapon muzzle-light rgb: gdbwatch the `lbu a1/a3` at 0x179230/0x17922c during a
   guard shot — the table base 0x2BF1A4 + 268·(weapon rec +98) entries +0/+2 vs the weapon rec's
   +0x54/+0x56 bytes. If they differ, `drone.rs:5499` should read the table.
3. Site 9 weapon: gdbwatch `lhu 0($s3)` at 0x1a5a0c while firing each weapon — which weapon hits
   the `0x55` compare (expected: the MI6 .44? the fragmentation grenade launcher?).

## Usage

not visible to me (no token, credit, context-window or cost figures are exposed to this session).
