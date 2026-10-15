# fix-aims20-lights-render

Branch `s/fix-aims20-lights-render`. Playtest ground truth (docs/live/2026-10-14-weapon-measurements-pcsx2.md,
"The side lights"): the AIMS-20's four round receiver lights are **lit red from the moment the gun
is drawn and STAY LIT WHILE IDLE**; the ammo pattern is the only thing that darkens one. The main
agent's render of master (07000005, `--dev --give 0x1a`) showed the row flat grey, never red, in
Burst and Grenade, standing still and strafing. Two things were asked: the render, and the idle
all-off gate (the 109d report's "What's left" flagged the `sVar5` chain at 0x1a5460:305-314).

## 1. The idle all-off gate — FIXED (the decompile was wrong; the disasm is not)

**The C's `sVar5` chain is a decompiler artifact.** `Player_WeaponFiring` 0x1a5460's C at lines
309-314 shows both arms of the last record check loading `+0x93a` before the `FRAME_RATE * 45.0 <`
compare. The disassembly (work/elf/ACTION.ELF, capstone; every instruction cited):

```
001a5bf0:  lhu  $v1, ($s3)            ; the held record's ID (+0x00)
001a5bf4:  addiu $v0, $zero, 0x1a
001a5bf8:  bnel $v1, $v0, 0x1a5c40    ; record != 0x1a -> the 45-compare; BNEL EXECUTES its
001a5bfc:  lh   $v0, 0x2f0($s2)       ;   delay slot: $v0 = clip[0x1A] (BL+0x2f0)  [M]
001a5c00:  lh   $v0, 0x2f0($s2)       ; (record == 0x1a arm: the <7 blink reads +0x2f0)
...
001a5c3c:  lh   $v0, 0x93a($s2)       ; (record == 0x1a arm: reload the idle counter)  [M]
001a5c40:  lwc1 $f0, -0x2f30($at)     ; FRAME_RATE (0x30e0d0)
001a5c48:  lui  $at, 0x4234           ; 45.0f
001a5c5c:  mul.s $f0, $f0, $f1
001a5c60:  c.olt.s $f0, $f2           ; FRAME_RATE*45 < $v0 ?
001a5c68:  bc1fl 0x1a5ca8             ; -> all four AnimDatumSetEntity(vm, 8..0xB, 0, 0xff)
```

MIPS branch-likely semantics: a taken BNEL executes its delay slot, a not-taken one nullifies it.
So the 45-gate compares against **clip[0x1A] (BL+0x2f0, the burst slot)** when the held record is
0x1b, and against **+0x93a (the idle counter)** only when it is 0x1a (the reload at 0x1a5c3c sits
on the fall-through path). The disc's burst mag is 30 [L weapon_data 0x1a +0x92], and the clip
never exceeds its mag, so **in grenade mode the gate can never fire** — the row stays lit while
idle, exactly the measured behaviour. In burst mode the port's existing 0.75 s dead-still all-off
is correct and stays.

Also settled from the same disassembly: the whole aims block runs on the record's **base weapon**
(+2) == 0x1a [M 0x1a5a94-9c] — 0x1b's base is 0x1a, so the attach + gates run in grenade mode too
(the port already did this); bone 8's blink gate reads +0x93a before any record branch [M
0x1a5ad8] — it blinks while idle in both modes (the port matches).

Fix (`crates/nf-view/src/scene.rs`, `frame_state`'s aims arm): the all-off gate is
`idle > 45` when `s.gun.weapon == 0x1a`, else `clip[0x1a] > 45` (never on disc data).

## 2. The render — root-caused to the mesh/colour chain, the last mile is OPEN

Verified facts (each with the probe that pinned it, `crates/nf-view/examples/probe_aims_*.rs`):

- The mesh the game attaches is **0x02000132 "gunsight3d"** in the level archive [D 07000005.bin
  map 10 obj 13]: 8 verts, ONE batch, **additive** (ALPHA_1 0x48), zwrite off, atest As≥1, texture
  (map,12), vertex rgba **(150,150,150,128) grey**, glist flags 0x321.
- **The texture is pure red** [D probe_aims_light_tex]: 64×64, 16-palette, every entry [r,0,0,255]
  (centre 185-253 red, background (2,0,0)). The game's red is the texture × grey vertices — **not**
  a tint: the tint source `psiSetTweakARGB` at AnimObjectDraw 0x1c4490:337-345 (disasm
  0x1c4a44-64, the celglist+0x44 bit-0x800 arm) feeds the WEAPON OBJECT's +0x103..0x106, which
  `control_init_object` 0x134630 sets to 0xff (white) and nothing rewrites (the red-tinted
  0x2000132 object at animobj+0x86c, Player_InitWeapon 0x1a7ef8:103-110 R=0x7f/G=0/B=0, is the
  **laser dot**: hidden at 0x1a5460:98, positioned by Player_LaserPointer 0x1a65d8:92).
- The port's chain is sound piecewise: the aims material exists with key (10,12) blend 1 (dev
  build material 3051), `scene.textures` holds (10,12) as the red image (avg (20,0,0)), the
  palette entries are posed for the lit bones (det 1.0; bone 10 correctly collapsed for the dry
  grenade slot), the gates pass with strafe held, and the colour multiplier is the owner's night
  ambient (0.22,0.27,0.33) — red × anything stays red.
- **But the headless render shows zero pixel contribution from material 3051** (NF_SKIP_MAT=3051
  pixel-diff = 0 on the strafing run): the quads never rasterize. A CPU projection through
  `fs.attach` put bone 8 at fwd −0.15 relative to Bond's eye with default ViewOpts axes — the
  probe needs a re-run with the render's `cam_axes(cam)` to say whether the port's view-model
  pose puts the light quads at/behind the eye plane (the game's are in front of the near plane).
  The grey dots visible in the render are the receiver texture's own light wells, not the mesh.

**What's left**: the follow-up probe run (with the render's cam_axes and a char-50 skinned-vert
projection added) sharpened the seam: under the probe's `frame_state(None, 60.0, opts)` the GUN
BODY verts project off-screen too (0/401 on-screen) while the actual `--shot` render poses the gun
correctly — so the probe's palette ≠ the render's palette with seemingly identical inputs (same
load_dev build, same give/switch, same 60 strafe ticks, same time=60). The next session should diff
`update_skin`'s exact invocation (the `vis` mask, `opts` fields beyond right/up, the App-side
`anim_time`/`choice` plumbing) against the probe's bare `frame_state` call; once the probe
reproduces the render's palette, the aims quads' projected position settles whether the bones
8..0xB sit at the wells (GPU-state bug) or behind the eye (pose bug). Live check 3 below settles
the game side in one screenshot either way.

## Verification

- `crates/nf-view/tests/fix_aims20_lights_render.rs` (new, through `Sim::step_bond` with the real
  Input): grenade mode dead-still 60 ticks with clip[0x1A]=30 — **FAILED on the pre-fix tree**
  (the row collapsed at idle>45; the fail-first panic at line 99 is on record), **green after**;
  sister assertions: the burst side's dead-still collapse still fires (0x1a5c3c), the idle counter
  really climbed, the burst slot really held 30.
- Regression sweep (the aims/weapon-mode neighbours): fix_aims20_modes_lights_live 5, gadgets_live
  3, fix_weapon_modes_live 2, fire_modes3_live 2, fix_silencer_modes_live 6, fix_weapons_all_live
  4, fix_auto_switch_live 4 — **26 passed, 0 failed**.
- Full suite: this sandbox reaps background processes (the nohup/setsid suite died twice here as
  in the previous sessions; the last restart at 05:29 UTC — read `tail -1 suite.out`, expected
  still running or reaped). **No total line this session** — the next session should run it on the
  final tree.

## Files

- `crates/nf-view/src/scene.rs` — the aims arm's record-dependent all-off gate (task file).
- `crates/nf-view/src/lib.rs` — the headless `--shot` path now honours `--give` (equip +
  switch_request BEFORE the ticks, mirroring the window path's block) and gained `--strafe-hold`
  (the analog stick reset held every tick, the `--zoom-hold` pattern). Shared file, additive
  hooks, listed here per the rules. These exist because the main agent's render spot
  (`nf-view <iso> 07000005 --dev --give 0x1a`) silently never reached the give block in the
  headless path — the shot rendered the level's starting gun.
- `crates/nf-view/tests/fix_aims20_lights_render.rs` — the new test.
- `crates/nf-view/examples/probe_aims_{light_mesh,light_tex,light_uv,scene_mat,fs,fs2,project}.rs`
  — the evidence probes (untracked scratch, not part of the fix; committed so the next session
  can re-run them).

## Render: the view model with the lights

1. `nf-view <iso> 07000005 --dev --give 0x1a --strafe-hold --shot out.png --frame 60` — the AIMS-20
   drawn in Burst, strafing: the row should be **red** once the render bug closes; today it shows
   the receiver's grey light wells (the mesh quads add nothing).
2. Same with `--frame 600` (dead-still, no `--strafe-hold`) in Burst: the row goes dark ~0.75 s in
   [M 0x1a5c3c] — the burst half the port keeps.
3. Grenade mode (`--give 0x1b`): the row stays lit while idle [M 0x1a5c40's clip read]; bone 10
   (the grenade light) goes dark with the slot dry (red, red, dark, red).

## Live checks for the main agent

PCSX2 (gdbwatch.py/fieldprobe.py, 07000005 or 14, the AIMS-20 drawn):

1. **Burst dead-still**: gun drawn in Burst, no input 2 s — do the four lights go out at ~0.75 s?
   Expected YES per the disasm [M 0x1a5c3c: the 45-gate reads +0x93a]; the ground truth's
   "staying lit while idle" may have been observed in grenade mode. Watch BL+0x93a climbing while
   idle (re-find BLData on the level).
2. **Grenade dead-still**: press ○ (needs the 0x1b rounds), no input 2 s — the row must stay lit
   [M 0x1a5c40 reads clip[0x1A] ≤ 30]. This is the decisive check for the gate fix.
3. **The light quads' bone positions**: `AnimGetBoneWorldTrans(vm, 8..0xB)` world translations —
   screenshot with the gun drawn; expected: the four quads sit ON the receiver's light wells, in
   front of the near plane. This settles whether the port's view-model pose for bones 8..0xB is
   what keeps the quads behind the eye plane.
4. fieldprobe poke: write 60 into clip[0x1A] (BL+0x2f0) with 0x1b held and idle > 1 s — expected:
   the row goes DARK (the gate reads the burst slot; >45 fires) — the odd-but-faithful reading of
   the disasm, worth one screenshot.

nf-view `--bond` spots: the `--give`/`--strafe-hold` shots above (spot 1-3); the aims matrices are
the test's `frame_state` check.
