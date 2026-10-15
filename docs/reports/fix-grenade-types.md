# fix-grenade-types — grenades: stun, smoke, gas behave like the game

Branch `s/fix-grenade-types` (from origin/master eb52fb56), 2026-10-14, worker 107a. The playtest
symptom: "smoke grenade didn't work" (1013-1825). The thrown set on the disc [L — swept from the
game's own `weapon_data` initialiser over ACTION.ELF]: **0x34 frag** (r8/d100), **0x35 stun** (r10/d0,
`+0x70` = fuse+FLASH), **0x36 smoke** (fuse+SMOKE), **0x69 second smoke** (the same name label as
0x36; the 3600 family). No separate flash/gas records — the gas puff and the white light ride the
stun branch; satchels/tripbombs (0x3a-0x40) are 107b's scope and were left alone.

## What was broken, and the fixes

All numbers from `Bullet_DoTrails` 0x12d890 (the fuse/expiry arms), `DroneFunc_RecoverTime`
0x147188, `StunGrenadeImpact` 0x172bb0, `Player_SetFlashBang` 0x196040, `Player_Update` 0x193600,
`Bullet_init` 0x12c5a0, `Occlude_Create/Update` 0x17ea80/0x17eba0.

1. **The stun's white-out was invisible.** The port wrote `flashbang = 1.0` and decayed it by
   1/6 per tick (≈6 ticks of faint white). The game: at the blast, Bond's head takes
   `f = clamp01(1 − (dist−5)/85)` (5→90 u) × the facing factor `1 − (|yawDiff| − π/6)·0.81851107`
   (blast at his xz → 1.0; negative → 0) × 0.5 without LOS, then
   `Player_SetFlashBang(FRAME_RATE·f·10, BL, f·255)` = **600·f ticks** at alpha f [M 0x12e0b8,
   0x196040; FRAME_RATE = 60 at 60 Hz [D docs/systems/input.md:126]]. `Player_Update` renders a
   full-alpha plateau for the first half of the timer, then a fade [M 0x193600:328-352]. Ported:
   `nf_game::drone::flash_amount` (the pure formula, unit-tested), the FLASH branch feeds
   `PlayerHealth.flashbang/flash_total/flash_alpha` (new fields, own lines), the tick decrements
   1/tick, and `hud::script_overlay` paints the white quad through `Scene::script_overlay_hud`
   (GlobalBlur has no port chain — documented gap).
2. **The stun held a guard 2× too long and never disarmed him.** `RecoverTime`'s switch gives the
   0x35 arm `FRAME_RATE_INT·30·0.01·(200−Drone+0xB9)` = **3600** ticks at the stats default
   b9 = 0 (`+0xB9 = BOT_stats[8]`, zero at run time [L 07000005]); the port used the switch's
   DEFAULT arm (36·(200−b9) = 7200). And `StunGrenadeImpact` ENTER **drops the held weapon**
   (`DroneWeap_DropWeapon` + `ChangeWeapon(1)`) unless he holds the unarmed 4 [M 0x172bb0 case 1]
   — the port left the rifle in his hands. Also ported: the prop-0x1C immunity
   [M `DroneFunc_HandleImpact` 0x1469b8 case 0x18's first gate] and the alert set.
3. **A stunned guard could be held down forever by a stale distance.** The Loop's case 0xC is the
   game's LIVE `DroneFunc_AnyoneNear` positional query [M 0x14a718]; the port read the drone's
   cached `self.dist`, which a stunned guard (perception frozen) never refreshes. Now the live
   player position rides the tick's shared pass (`Drone.player_pos`) and the query is
   `SqDistXZ < 16 && |dy| ≤ 3` exactly.
4. **The 0x69 smoke was a 0x36 clone.** `Bullet_init` arms its fuse to **5.0** [M 0x12c5a0:212-215]
   (the port gave 210); the pop re-arms to **3600.0** and `Occlude_Create(pos, 3600, 1)` — the
   cloud's lifetime IS the re-armed fuse [M 0x12e114-0x12e1b4] (the port hard-coded 1200 for both).
   Both the guard throw path and the player fire path carry the 5-tick fuse now.

Verified-unchanged (the audit's clean side): the frag fuse 210 / blast 8·100 from the record (the
existing fix_grenade_throw_live tests still pass), the smoke pop sound 0x14 + the 1200 cloud for
0x36, the stun's sound 0x16 / light (10, 5, white) / gas puff (0.5, 1.68, fx 0x200001f, 0xB) /
shake 2.0, the SP player-thrown-only broadcast gate, and `Occlude_Create`'s kind→mesh table.

## How it was verified

- `crates/nf-view/tests/fix_grenade_types_live.rs` (new, NF_ISO, through `nf_view::load_weapons` +
  the panel give + `Sim::step_bond`): the 0x69 fuse-5/3600-cloud test; the at-feet flash test
  (asserts the ported formula on the OBSERVED blast geometry, on both its LOS branches — the old
  flat 1.0 fails either); the retreated-throw distance test (d ≈ 38 → alpha ≈ 0.30, the ×0.5 LOS
  arm — the old flat 1.0 fails); the placed-guard stun test (0x4D, weapon dropped to 1, the armed
  span). **3 passed / 1 ignored** — the ignored one is the open item below.
- `nf_game` lib 152/152 (the `stun_loop_recovers…` unit re-pinned to the live-pos query, plus a new
  |dy| sister); `fix_grenade_throw_live` 3/3, `facial3_live`, `fix_cutscenes2_live` green
  (no neighbour regressions).
- **Full suite** (`tools/test/suite.py`, 282 binaries, NF_SILENT, slim ISO, foreground chunks —
  the nohup background run dies between tool calls here, the 102c/100f precedent):
  **1073 passed, 3 failed, 35 ignored, 1 TIMEOUT**. The 3 failures are exactly the allowed
  `disc_golden` trio; the TIMEOUT is `weapon_matrix_live` at its 600 s cap — the same open item the
  base branch's final run carried (a volume test on this 2-core sandbox; the weapon-matrix workers
  are still landing on it — s/weapon-matrix4b/5 in flight today).
- One suite regression CAUGHT and fixed during the run: the `stun_loop_recovers…` unit pinned the
  old cached-`dist` semantics; re-pinned to the live-pos query (the fix above is the game's rule).

## What's left (the ignored test's findings)

`the_stun_stuns_the_guard_for_3600_and_drops_his_weapon` (`#[ignore]`, reason in the file): the
arm, the drop and the duration all verify (a placed guard: 0x4D, weapon 1, `stun_until =
frame+3600`), but the RELEASE chain stalls in 0x4D for guards whose anim set lacks the 0x2d/0x2e
stun anims — the ailog shows guard 0 advancing 0x4d → 0x4e (the Loop) while the chosen guard never
leaves Impact. `DroneAnim_CallAnim`'s no-datum fallback needs a chase; the game's `CallAnim` with a
missing datum falls through to the follow-on state. The stun_kept_down fix above removed the OTHER
forever-stun (the frozen dist), so a flashed guard now reaches the timer.

Also left, documented not fixed: the smoke trail while flying (`+0x6C & 2` →
`Bullet_trail_effect_smoke` — particles), the GlobalBlur post-FX channel, and any AI-sight
occlusion through the clouds (no decompile reader of the class-0x2A objects was found in the
vision path within the window — the clouds are visual-only in the port, as the evidence shows).

## Files

- `crates/nf-game/src/drone.rs` — the FLASH branch (the flash math + the guard stun fixes), the
  smoke re-arm/cloud by type, the throw fuse 0x69, `flash_amount`, `Drone.player_pos` +
  `stun_kept_down` live query, the tick decay, the `stun_loop_recovers…` unit re-pin.
- `crates/nf-game/src/bullet.rs` — `PlayerHealth.flash_total` / `flash_alpha` (own lines).
- `crates/nf-view/src/scene.rs` — the player-path fuse 0x69; `script_overlay_hud` computes and
  passes the flash alpha.
- `crates/nf-view/src/hud.rs` — `script_overlay`'s white flashbang quad (one parameter + one quad).
- `crates/nf-view/tests/fix_grenade_types_live.rs` — the new live tests.
- Shared-file hooks: `drone.rs` is a named task file; `scene.rs`/`hud.rs` are the nf-view "small
  screen effect" the task names; no other shared files touched.

## Live checks for the main agent

- PCSX2, 07000005 (the security office start): throw the STUN grenade at a nearby guard — he
  whites out if facing it, the screen stays white ~2-3 s then fades, the guard drops his rifle and
  staggers (~60 s at the stats default), then gets up rubbing his eyes (anim 0x76).
- PCSX2: throw the stun at your OWN feet from a distance ~40 with a wall between → the white-out
  is roughly half the clear-line brightness (the ×0.5 LOS arm).
- PCSX2, level 14's gas canisters (the 0x69 family): the canister pops ~immediately after the
  throw (5 ticks) and the smoke cloud lingers ~60 s (3600 ticks), vs the 0x36 grenade's 20 s.
- nf-view render spots (the whiteout is a HUD overlay; the cloud draws in-scene):
  `nf-view <iso> 07000005 --bond <guard-facing spot> --frame <blast tick> --shot` — the white
  overlay should wash the frame at alpha ≈ f; compare with the same spot pre-pop.
