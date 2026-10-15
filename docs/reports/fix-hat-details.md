# fix-hat-details — Oddjob's hat: the wall sound and the in-flight cap

Task `109o-fix-hat-details`, branch `s/fix-hat-details` (the message head; the task footer's
`s/hat-details` conflicts with it — the head names the branch everywhere else, so it wins, per the
established precedent). Playtest 1014-1725, real-input build (`nf-view <iso> <level> --dev`).

User: "It makes a weird sound when it hits a wall. I'm not sure if that's accurate to the game. And
also, if you throw enough of them, you stop seeing them flying in the air."

## What was found

### 1. The wall-contact sound — the port already matches the game (no repro, pinned by a test)

The game's hat-on-wall is **exactly one `Sound_Play3D(0x4a7, obj+0x30, 96.0, −1.0)` clang**:

- [M] `Bullet_CollisionHandler` 0x12ce80, the +0x100 sticky arm, hat sub-arm — **disassembly
  0x12d43c-0x12d470**: `lui $at, 0x4416` + `mtc1` + `swc1 …, 0xfc($s3)` writes the 1500.0
  post-stick lifetime into BU+0xFC, `oriu $a0, $zero, 0x4a7` loads the sound id, `oriu $a1, $s3,
  0x30` the position, f12 = 0x42c80000 (96.0), f13 = 0xbf800000 (−1.0), `jal Sound_Play3D`
  (0x1dd548). The satchel 0x3b plays the same id [:319-321].
- **No material sound joins it**: `Effect_Create` 0x1ce688 is called with flags 0 for the hat —
  the 0x61 needs `weapon_data+0x70 & 2` and the hat's +0x70 = 0x100 [L ee_start_05.bin
  @0x2c398c]; the 0x10 rewrites need `+0x6c & 0x100` (hat: 0x20c08 — clear) or `+0x70 & 0x1000`
  (clear). `Effect_Bullet` 0x1ce8a8:86-92 plays the per-material `EffectInfo[material&0x3f]+0x14`
  sound only under `flags & 0x40` — never for the hat. (For comparison, the floor's material
  impact id at the test's stick point is 292 [L EffectInfo] — nothing like it is heard.)
- **No bounce clang either**: the bounce block (LAB_0012d4bc, sound 0x51c) needs `+0x70 & 0x80`
  or a character-hit flag; a +0x100 record's world hit has neither [M 0x12ce80:398-407].

The port's stick path plays exactly that one clang (`GrenadeEvent::Stick` → `audio.play3d(0x4a7)`
in `crates/nf-game/src/drone.rs`, from the merged fix-oddjob-hat). **Headless probe through the
real input path** (`hat_details_live.rs::the_hats_wall_contact_is_the_single_game_clang`): the
sounds started at the hat's position from the stick tick onward = `[1191]` — one 0x4a7, nothing
else (the throw's own sounds — 0x5FD/0x85/0x174 at the press, the gun script 0600097e frame-15
0x41 whoosh at the camera — play at Bond, verified with `NF_SOUNDLOG`, and sit outside the
contact window). So what the user heard as "weird" is the game's own placement clang — the heavy
one the satchel shares. No change made; the test pins it so a future regression fails loudly.

### 2. The in-flight cap — FIXED: the draw pool was 8 slots; the game's draw list is a 512-entry ring

- The port's Sim never drops hats (`Drones::grenades` is a `Vec`, unbounded). The **draw** pooled
  the projectiles into static slots and drew only `grenades[0..8]` (`GRENADE_SLOTS = 8`,
  `crates/nf-view/src/scene.rs`, the models-weapons-items lane). The stuck hats occupy slots for
  their 1500-tick post-stick life [M `Bullet_update` case 3 0x12e7b0:363-371], so from the 9th
  live hat onward every new throw flew INVISIBLE — the user's symptom.
- **The game's limit is the per-view draw list**, not a projectile pool: `View_AddObjects`
  0x1e62d8 adds every in-view object with a glist to a per-viewer RING —
  `entry = *(viewer+8+ring·4) + cursor·0xc; cursor = (cursor + 1) & *(u16*)(viewer+0x1a+ring·2) − 1`
  [M, the decompile's `*puVar7 = uVar2 + 1 & *(short *)(iVar10 + uVar6 * 2 + 0x1a) - 1U`] — when a
  frame's adds pass the size, the OLDEST-ADDED entries are reused. Ring assignment: ring 2 is the
  `obj+0xf0 & 0x2000` billboards; a flying bullet has both 0x10 and 0x2000 cleared by
  `Bullet_update` state 0 [M 0x12e7b0], so it rides ring 0. **Sizes**: [L ee_snapshot.bin]
  viewer[0] @0x1fc6c10 and viewer[5] @0x1fc69b0 both carry `viewer+0x1a/0x1c/0x1e` = 512/512/128 —
  512 entries per ring (each entry 0xc bytes; `View_DrawObjList` 0x1e69a0 sorts the `cursor`
  entries by squared distance and draws them; `View_CaptureScene` 0x1e6df0 zeroes the cursors
  each frame, so the cap is per frame). The cel lists are newest-first [M
  `control_link_object_to_cel` 0x1346b8 prepends; docs/atlas/object.md], so a full ring keeps the
  NEWEST objects — with level 05's ~91 adds per frame at start [L cursor 91/43/8], no hat spree
  gets near 512: every flying hat draws in the game.
- **The fix** (`crates/nf-view/src/scene.rs`): `GRENADE_SLOTS` 8 → **512** (the game's ring size,
  the citations above), the projectile slot field widened u8 → u16 (512+32+8 slots no longer fit
  a u8), and the slot→grenade mapping made **newest-first** (`slot i` draws
  `grenades[len − 512 + i]` when len > 512, else `grenades[i]`) so an overflow drops the OLDEST
  exactly like the game's ring. One instance per (slot, mesh) as before; the per-mesh instance
  count grows accordingly (comparable to the existing 200-per-mesh decal pools).

### The fail-first proof (on master, before the fix)

`hat_details_live.rs::every_thrown_hat_still_draws_while_it_flies` — load 05, give 0x45, apply the
dev build's own infinite-ammo pin (the playtest ran `--dev`, where the pin is ON by default:
`lib.rs` builds `Dev { infinite_ammo: true, .. }`; without it each hat spends the record's
+0x91 = 100 [M `Player_WeaponFiring` 0x1a5460:654 `RoundToFire(+0x91, +0x91)`] and the reload
starves the spree; the hat's refire interval is 26 ticks, `+0x40 = 0x1a` [L @0x2c398c]), pulse the
trigger 12 times through `Sim::step_bond`:

- **master (pre-fix)**: `thrown=12 live_in_sim=12`, the draw built **8** hat-mesh instances, the
  four NEWEST flying hats (stuck=false) undrawn — `[draw] UNDRAWN hat at …` × 4. FAILED.
- **post-fix**: `[draw] hat-mesh instances drawing: 12` — every live hat's position matched a
  drawn attachment (≤ 0.5 tolerance). PASSED. The sister assertion in the same test: the count of
  live hats must exceed the old pool (12 > 8) and every one of them draws — stuck hats included
  (the game draws a stuck hat for its 1500-tick life).

The sound test (`the_hats_wall_contact_is_the_single_game_clang`) is the second fix's pin: it
FAILED the assert on the first probe run only because the shallow pitch put Bond's camera inside
the contact radius (the throw whoosh 0x41 leaked into the window) — re-aimed, it passes on master
and post-fix; it guards the exact one-clang contract.

## Verification

- `hat_details_live` 2/2 green post-fix (NF_ISO, real input path: `nf_view::load_weapons` +
  `Sim::step_bond`, the dev pin lines are `dev.rs`'s own infinite-ammo arm).
- Neighbouring suites on the fixed tree: models_weapons_live 4/4, oddjob_hat_live 2/2,
  gadgets_live 4/4, fix_grenade_release_live 2/2, fix_grenade_throw_live 4/4,
  fix_grenade_types_live 3/3 (+1 ignored), fix_remote_mine_play 3/3, fix_satchel_tripbomb_live
  5/5 — the projectile-slot change touches every grenade's draw; all green.
- The fast runner (`tools/test/suite.py`) was started in the first minutes but its first run
  stalled (0 bytes after ~70 min, no cargo process — the sandbox's background-process reaping,
  seen in earlier sessions); the restart at ~06:20 UTC was reaped the same way (`suite.out` 0
  bytes). **No full-suite total line this session.** The covered set is the foreground sweep on
  the final merged tree (origin/master 2e5a162b merged at 1380640c): nf-game lib 160/160,
  hat_details 2/2, oddjob_hat 2/2, models_weapons 2/2, fix_grenade_release 4/4,
  fix_sg5_casings 2/2, plus the pre-merge neighbour sweep (gadgets 4/4, grenade throw/types,
  remote mine 3/3, satchel tripbomb 5/5). On the slim ISO only the 3 `disc_golden` tests (and
  `pss_movies_live`) may fail; next session runs the full total per the standing rule.

## Shared files touched (small, additive)

- `crates/nf-view/src/scene.rs` (the draw — a task-named file): `GRENADE_SLOTS` 8→512 with the
  citation block, `Attachment.projectile` u8→u16 (one field type, its own doc line), the
  newest-first slot mapping (4 lines) in the projectile arm, and `slot as u16` at the one
  constructor. No renames, no reordering.
- `crates/nf-game/src/{grenade,drone}.rs`: **untouched** — the Sim side already matched the game
  (unbounded Vec, the stick, the sounds).

## Live checks for the main agent (PCSX2)

1. **The hat's wall clang** — give Oddjob's hat (weapon 0x45), throw at a wall from ~4 units:
   expect ONE heavy clang per contact (the same sound the satchel makes when placed), NO concrete/
   material thud layered with it, no bounce sounds. If a `gdbwatch` on `Sound_Play3D` 0x1dd548's
   id register is feasible: expect exactly one start with id 0x4a7 per wall contact, none with the
   material id of the wall's EffectInfo (+0x14) — the hat's `Effect_Create` flags are 0
   [M 0x12ce80 → 0x1ce688].
2. **The in-flight hats** — with the dev pin on, throw ~12 hats quickly at a wall/floor: expect
   ALL of them visible in flight and stuck (the game's 512-entry draw ring never bites). In
   nf-view the same spree via the panel give + tapping fire: all 12 visible.
3. `--bond` spots (nf-view render, level 07000005, hat given):
   - `105.9,25.3,108.1,0.0,-0.45` (Bond's start, aimed down-forward): a thrown hat mid-flight as
     a hat-shaped mesh at the aim point; after it lands, the hat visible on the floor aligned to
     the surface.
   - `105.9,25.3,108.1,0.9,0.9` (aimed up): during a 12-throw spree, 12 hats in the air at once —
     pre-fix only the first 8 existed visually.
4. The stuck hat's orientation is a known [S]-grade deviation (the port draws a stuck hat
   up-aligned via the zero-velocity fallback; the game aligns the matrix to the negated hit
   normal [M 0x12ce80:256-266]) — NOT changed here (it predates this task and the draw slot
   rework touches the same arm; flagged for the driver to schedule if the user reports it).

## What's left

- The full-suite total line on the final tree (the restarted runner; the neighbour suites above
  are all green).
- The hat's +0x68 = 0x180a0 [L] — bit 0x10000 is set and `Player_WeaponInitBullet`'s
  `+0x68 & 0x20000`… the 0x68-vs-0x6c homing-flag split (the spawn walk at 0x1a4bf0 reads +0x68,
  `Bullet_homing` gates on +0x6c) was not audited this session; the homing behaviour already
  merged and works, noted only as an observation.
