# fix-grenade-release — the grenade release timing and the blast's "thin wall" (109e)

Playtest 1014-1725 (user 07000015, frag/smoke): "Grenade still gets released while I'm still
pulling the pin. The explosion has this like very thin wall that isn't accurate." — the third
report of the release timing. Branch `s/fix-grenade-release` from `origin/master` (bf2bd176,
merged to 38c88df1 mid-task).

## Done

### 1. The release timing: audited end-to-end; the tested path is correct — the report carries what was tried, no fix was guessed

The game's mechanism [M]: the fire press arms `shots_left = shots_per_pull` and `timer = 1.0`
(`Player_WeaponFiring` 0x1a5460:601-626, re-armed on the action-9 press edge / held read when the
timer expired); the shot tick decrements the timer, gates on the view state ∈ {0, 9, 0xb}
(0x1a5460:631-648), spends via `Player_RoundToFire`, sets `timer = +0x40`, and for `+0x44 != 0`
weapons defers the spawn to `Player_SetWeaponAnimObj` case 9 (0x1a2e38:416-420): the round spawns
when the fire script's time (`AnimList+0x90`, advanced by `AnimScriptTick` 0x1c2630 as
`+= speed·FRAME_RATE_MUL`, speed 1.0 for the fire script) reaches `weapon_data+0x44`, latched at
`+0x95C`. The port implements exactly this (gun.rs `update_with`/`release_frame_reached`, scene.rs
"the release frame" block).

Measured on the merged tree through the REAL input path (`nf_view::load_weapons` on the user's
**07000015**, the panel give, `switch_to` in, `step_bond` with the trigger held — the same Input
lib.rs builds from the mouse): press tick queues fire script 0x0600049d at view_time 0, the clip
spends on the press frame, the grenade spawns at press+25 ticks = script frame 26 = `+0x44` [L the
record: +0x44 = 26, +0x40 = 450, shots_per_pull 1, flags68 0x14080; the smoke 0x36 identical
except its scripts 0x0600702/3/4]. The pin sound (script op-1 frame 16) precedes the spawn. That
is the game's own timing: the PCSX2 captures show the pin pull at screenshots 01-02, the raise at
03, the flight at 07 — release ≈ frame 26 at 1 frame/tick; `view_frame` samples seq 0x04000414
(70 frames, window [1,70]) at frame = 1+view_time, 1:1 with the script clock.

**What I tried for the "releases while still pulling the pin" symptom — no early spawn
reproduced headlessly:**
- the press-tick spawn arm (`fire_anim_len == 0`): the frag's +0x44 is 26, not 0 — the arm can't
  fire;
- the script's frame count vs its rate: `AnimScriptTick` mode 0 advances `+0x90` by
  `speed·FRAME_RATE_MUL` (1.0); the fire script's queue speed is 1.0; seq 0x04000414 is 70 frames
  over the [1,70] window — 1:1, no desync;
- a held-fire repeat: the port re-arms only on the press edge; the game re-arms every held tick at
  timer expiry (0x1a5460:601-624 — a 450-tick auto-repeat the port lacks — throws FEWER grenades,
  not early);
- play-only states: firing during the draw/switch (`can_fire = !switching` blocks), the second tap
  inside the 450-tick interval (both refuse), the dev-mode infinite-ammo clip pin (dev.rs — it
  tops the clip, never touches the fire path), the tick accounting (`ticks_for_frame`, dev
  `time_scale` 1.0 default), the warm asset reload path, the aim/scope state.
- The one decomp-backed discrepancy I could not close: the game's re-arm arm ALSO fires on
  `param_2 == 1` (0x1a5460:603 — the caller's second argument; the call site in `Player_Update`
  was not traced in time). If that argument is the held-fire read, the game's held trigger
  re-throws every 450 ticks — the port never does. This changes the repeat cadence, not the first
  release frame.

**Conclusion: the first-throw release timing in the port matches the game (frame 26 ≈ 0.43 s).
The user-visible desync did not reproduce headlessly. No code change was made to the fire path —
per the rules, nothing was guessed.** The live checks below are the fastest way to see what the
user saw.

### 2. The blast's "thin wall": the puffs were drawn in their authored plane — now camera-facing

- The sim's blast plays effect script 0x06000052 at scale = the radius (`spawn_blast` →
  `spawn_once_scaled(…, radius)`; [M `Explode_Create` 0x17cbd0 stores param_1 into
  SCRIPTINFO+0x94, `Script_GetInterp` 0x1da158:117 multiplies every key scale by it]).
- The game's DRAW billboards the blast meshes: `Control_SetGList` (0x134960) copies the glist's
  own flags (`celglist+0x44`) into `obj+0xF0` for every script entity (the mesh lookup
  `hashtable_set_object_to_entity_gfx` runs it), and `View_DrawObjectGlist` (0x1e6780:43-65) draws
  flag-0x8 objects with the **viewer's rotation** (`Mat_CopyRot(viewer+0x1a0)` — the interp
  matrix's rotation is ignored), keeping the interp translation and the interp scale
  (`Mat_Scale3f(s,s,s)` 0x1e35f0 with `s = obj+0xE8` — the key scale × SCRIPTINFO+0x94). The port
  drew the raw interp matrix: the flat puff quads sat in their authored plane and read edge-on as
  "a very thin wall".
- **Fix (scene.rs, additive):** `Scene::script_billboards` — the entity meshes whose glist flags
  carry 0x8, collected at build time; `frame_state`'s script-entity arm replaces such a matrix's
  rotation with the camera basis (opts.right/up, the same source the gas-puff billboards use),
  keeping the interp scale (|column 0| — the interp builds rot·s with s uniform) and translation.
  The obj+0x58 spin arm (the interp X-axis re-applied over the viewer rotation,
  `Script_EntityStart` 0x1d8d78's `Mat_GetNorm`/`Vec_NormToRot`) is NOT ported — [S] sub-visible
  on the blast; live check below.
- `fix-blast-scripts` (master) extended the build's script sweep to
  0x06000052/0x27/0x4f/0x7c4; the merge keeps that and adds the **op-0x1b sub-script walk**:
  0x06000052's `Script_SubScriptStart` (0x1db050) spawns 0x06000047 [L the live frag blast's
  `Obj::Entity` src.0 = 0x06000047 — the lib key], whose streams carry the puff meshes
  (0x020003cc/cd/ce/c7). The walk is in; whether those meshes' glists carry 0x8 is verified only
  for the meshes the sweep already collected (0x0200021a, 0x020003d7 ∈ the billboard set).

## Tests (`crates/nf-view/tests/fix_grenade_release_live.rs`, both green on the merged tree)

- `the_frag_leaves_the_hand_on_the_release_frame_after_the_pin_pull` — 07000015, real input: the
  +0x44/+0x40 record values, the press queues the fire script at frame ≤ 1, the clip spends on the
  press frame, the spawn lands on script frame 26 (23..27 window) — the pin pull (frame 16)
  precedes it.
- `the_blast_puffs_draw_camera_facing` — 07000005 (the archive that carries 0x06000052's entity
  streams [D probe; 15's does not]): the billboard set contains 0x0200021a/0x020003d7, the scene
  builds the blast's entity attachments, a real grenade explodes, and every drawn billboard
  attachment's X/Y axes are the camera's right/up with the interp scale intact.
- Pre-fix state: the test file does not compile on master (`no field script_billboards`) — the
  billboard assertions are new state; the release assertions pin the existing (correct) behaviour
  as the sister guard.

## Suite

- Baseline (branch point, started at task start): see `suite.out` in the sandbox (the pre-task
  run's tail: the run was still in flight at the wrap-up mark).
- Final tree (merged with origin/master): `suite_grenade_release.out` — started at the wrap-up;
  **the total line was not available before the deadline** — read `tail -1` of that file for the
  verdict. On the slim ISO only the 3 `disc_golden` tests may fail.
- The two new tests + the touched suites (`fix_grenade_throw_live` 4/4, `fix_grenade_types_live`,
  the fire-modes suite) ran green individually on the merged tree.

## Shared files touched

- `crates/nf-view/src/scene.rs` — 3 additive hooks: the `script_billboards` field (+ collection at
  build, + the op-0x1b recursion over the existing sweep), the billboard swap in `frame_state`'s
  script-entity arm. No reordering; new field on its own lines.
- No changes to `drone.rs`, `player.rs`, `lib.rs`, `gun.rs` (the fire path was verified correct
  and left alone).

## Open items / not ported

- The held-fire auto-repeat (the game re-arms every held tick at timer expiry — 450 ticks between
  grenades while held; the port throws once per pull) — needs the `param_2 == 1` call-site trace
  (0x1a5460:603) before porting.
- The sub-script puff meshes (0x06000047's 0x020003cc family): collected by the new walk; their
  billboard flag membership and the alpha fade (the game writes the key alpha to `obj+0x106`,
  0x1d9a08 case 1 — the port stores `Obj::Entity.alpha` but the draw ignores it) need the main
  agent's render to confirm.
- The obj+0x58 spin arm of the billboard draw (the interp X-axis over the viewer rotation).

## Live checks for the main agent

1. **PCSX2, 07000015, frag in hand, press-and-hold fire (~2 s):** watch the grenade leave the hand
   vs the pin-pull frames. Expected (the port should match): the pin sound ≈ 0.27 s, the release ≈
   0.43 s (frame 26). Then WITHOUT releasing, wait 7.5 s: the game throws a SECOND grenade while
   held; the port does not. (`gdbwatch.py` on `weapon_data+0x40`'s read at 0x1a5f70 and the
   `+0x92C` re-arm store at 0x1a6358-ish; the param_2 value passed to `Player_WeaponFiring` at its
   call site in `Player_Update` 0x193600 — mask-4 vs mask-1 semantics settle the auto-repeat.)
2. **PCSX2, throw a frag at a wall up close:** the blast — a camera-facing flash + puffs that
   always face the view while Bond strafes around them. If the game's puffs visibly SPIN
   (obj+0x58 arm), note the direction/speed.
3. **nf-view `--bond` render (before/after the billboard fix):** `--bond <throw spot>`, throw a
   frag at the floor, freeze within the blast's life (~0.3-2 s):
   - spot A: the blast centred, camera looking down the corridor — BEFORE: flat sheets edge-on
     ("thin wall"); AFTER: camera-facing quads/puffs of the same size as the game's.
   - spot B: the same blast with the camera yawed ~60° — BEFORE: the sheet rotates with the world;
     AFTER: it stays screen-facing (the billboard), size unchanged (the interp scale × radius 8).
4. **nf-view `--bond` (07000015):** the same throw as the user's playtest — the release frame vs
   the pin pull (should already match the game's ~0.43 s).

## Verification trail

- `crates/nf-view/tests/zz_grenade_release_probe.rs`, `zz_grenade_release_timeline.rs` — scratch
  probes (uncommitted): the record dumps, the release timeline (spawn at t=25), the live blast
  entity dump (src.0 = 0x06000047).
- Decompile: `work/decomp/ACTION.ELF/001a5460_Player_WeaponFiring*`, `001a2e38_Player_SetWeaponAnimObj*`,
  `001c2630_AnimScriptTick*`, `001d9a08_Script_Interp*`, `001da158_Script_GetInterp*`,
  `001e35f0_Mat_Scale3f*`, `001e6780_View_DrawObjectGlist*`, `00134960_Control_SetGList*`,
  `001d8d78_Script_EntityStart*`, `001d8968_Script_CreateEntity*`, `0012d890_Bullet_DoTrails*`,
  `0012ce80_Bullet_CollisionHandler*`.
- [L] docs/live/2026-10-14-weapon-measurements-pcsx2.md (the frag's frame-by-frame, the press-frame
  clip spend).
