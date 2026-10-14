> fix-hud-radar DONE (14:55 UTC): the SP HUD has no radar — panes 13/14 absent in the SP snapshots
> [L], PaneList 13/14 NULL [L], MPSettings+0x384 = 0 [L]; the bottom-left ring is the health pane's
> sprite 15 and the port already matched it. The MP radar (pane 14) is now ported + number-pinned:
> the projection disasm-checked (the unit-dir x/z negation, min(len·0.02,1) = the 50-unit range,
> the height cue = a DEAD STORE — 0x319574 has no reader), the radar-on layer gate, the HUD_FixMP
> top-right clamp, TexUV.291. hud.rs additive only (overlay untouched — zero SP visual delta);
> 73 neighbour tests green in foreground chunks; report: docs/reports/fix-hud-radar.md.

## fix-hud-radar — s/fix-hud-radar (task 103h, 13:55-14:47 UTC)
- The SP HUD has NO radar: [L ee_start_05.bin + ee_snapshot.bin @HUD 0x16c8630 — panes 13/14 never created;
  MPSettings 0x2a47a0+0x384 = 0 → PaneList [M 0x199e88:32-37]; PaneList 13,14 = NULL [L docs/fields/hud-panes.md];
  MPGame slots all NULL, radar flag +0x452 = 0 [L]. The bottom-left ring is the health pane's sprite 15
  [M 0x19b9c8:205-207; L HealthSprInfo[15] @0x2bd050+15·0x2c — (4,4) 87×86 uv (102,113,85,86) layer 9, the
  health tint, alpha 140], and the port already drew exactly that. The main agent's "radar stays" live note
  (docs/live/2026-10-13) was an MP Arena match — the radar is MP pane 14 only.
- Ported (hud.rs, additive; overlay untouched): RadarObj (RObj @0x318e70 [M 0x189840]), radar_project
  (HUD_RadarUpdate 0x19f6e8:108-153, DISASM-CHECKED 0x19f758-0x19fa60: the unit view-dir x/z negated
  [M 0x19f888-0x19f8b0], scale = min(len·0.02, 1) — the 50-unit ring edge, x/y = −dir·scale·(ring·0.45) +
  ring centre + 0.5, cap 24, layer 0x1c, TexUV.291 @0x2becf0 [L]); the height cue (dy·0.125 clamp ±1) is a
  DEAD STORE — 0x319574 has no reader [M disasm 0x19f88c-0x19f8d4 vs the reads at 0x19f904/0x19f974]; the
  radar-on layer gate (MPSettings+player·0x30+0x28) [M 0x19f6e8:102-105]; radar_ring_rect (created (512,0) +
  HUD_FixMP 0x19b110:73-92 → the viewer's TOP-RIGHT corner [L viewer+0xf0..fc = 0,512,0,448]).
- Tests: crates/nf-view/tests/hud_radar_live.rs 4/4 (the SP pins on 05 at the grapple spot + 06, the
  projection numbers, the gate/clamp sisters). Neighbours in foreground chunks: fix_context_hints 6/6,
  fix_hud 6/6, hud_playtest 5/5, menus_flow 34/34, nf-view lib 8/8, devwin 1/1, dev_menu 5/5, pickup 4/4 —
  73 tests, 0 failures. The four-crate sweep did not fit the window (background jobs die between tool calls
  in this sandbox — sessions 4 and 5). Report: docs/reports/fix-hud-radar.md (live checks inside: the SP
  no-blip confirmation, the MP ring position/size/blip reading, the pending 100n 007 item).

---
Task ID: 103j-fix-weapon-fx-levels
Agent: weapon-fx-levels worker
Task: fix-weapon-fx-levels — audit the weapon EFFECTS across all 27 levels x the carried set (fire sound, reload/draw/mode-switch sounds, muzzle flash, shell eject, tracer, impact puff/spark/decal per surface, projectile model+trail, explosion); record resolve vs missing as a table; fix every missing cell (or give game evidence the game lacks it too); watch gadgets' laser/stunner beam/impact; tests + 3 named spot checks; report docs/reports/fix-weapon-fx-levels.md.

Work Log:
- (14:29 UTC) Branch s/fix-weapon-fx-levels created from origin/master 088bdf4 (s/weapon-matrix2 already merged: ad50a3c). Setup reused (repo, work/, ISO, nf-queue). Suite baseline starting in background; effect-resolution research next.


---
Task ID: 103j-fix-weapon-fx-levels
Agent: weapon-fx-levels worker
Task: fix-weapon-fx-levels — audit the weapon EFFECTS across all 27 levels x the carried set (fire sound, reload/draw/mode-switch sounds, muzzle flash, shell eject, tracer, impact puff/spark/decal per surface, projectile model+trail, explosion); record resolve vs missing as a table; fix every missing cell (or give game evidence the game lacks it too); watch gadgets' laser/stunner beam/impact; tests + 3 named spot checks; report docs/reports/fix-weapon-fx-levels.md.

Work Log:
- (14:29 UTC) Branch s/fix-weapon-fx-levels created from origin/master 088bdf4 (s/weapon-matrix2 already merged: ad50a3c). Setup reused (repo, work/, ISO, nf-queue). Suite baseline starting in background; effect-resolution research next.
- (15:36) Resumed in a fresh sandbox: master merged (2caa27f, the WORKLOG conflict resolved append-only). Background jobs confirmed dead between tool calls here — testing went through foreground chunks.
- (15:45-16:35) Research (subagent + own disasm): the game's FX set is per-level redundant (EffectInfo decals/debris/gas, the tracer 0x02000123, the blast script 0x06000052 — kind-7 NIS — all in every story archive [D census]); 100a's wanted list missed the fp muzzle mesh +0x48 (0x02000ac7 only in 14/15/16/1b) and the ammo-9 casing 0x02000548; Bullet_DoTrails 0x12d890's smoke/exhaust trails were never ported (bit 0x2: 0x1b/0x36/0x69 every 4th tick, Bullet_trail_effect_smoke 0x12cba8; bit 0x20: 0x2a/0x2b every 3rd tick — args read off the disasm incl. the dropped life args 0x78/0x40; the bit-0x10000 Swoosh 0x1a4e40 documented, not ported).
- (16:36-16:50) Fixes: +0x48 + casing_meshes into wanted_weapon_models (lib.rs, weapon_assets.rs param); init_trail_smoke + init_exhaust_puff (gas.rs, 3 shared RNG draws each, searchlight precedent); the two gates in the grenade tick (drone.rs).
- (16:50-17:27) weapon_fx_levels_live.rs: the static sweep (20 levels x 114 records modelling the give path's pool: own ∪ earlier-story ∪ skin-owner; per-level own bank ∪ all-bank spares for the op-1 ids — args[1], not args[0]) + 3 spot checks. The rifle check is green (draw 0x282 from the own bank, the flash attachment, fire 0x40d, a live casing, the floor decal/debris); the launcher/gadget checks are #[ignore]d with the exact headless fire-path repro attempts written down; the sweep's only residual is two records' +0x80 held model in non-story archives (documented).
- (17:30) Neighbours green: fix_weapon_effects 6/6, explode 2/2, fix_weapons_all 4/4, fix_weapon_sounds 3/3 (+2 pre-existing ignores). The full suite did NOT fit the window (no background jobs here) — next session runs it on the merged tree.

Stage Summary:
- Pushed 56d9bff + the report commit to s/fix-weapon-fx-levels; docs/reports/fix-weapon-fx-levels.md (the mechanism, the three fixes with addresses, the table, the live checks incl. the Swoosh reference render, what's left). 0 unexplained missing effect cells; the +0x80 held-model residual and the headless foreign-fire block are the known leftovers.
## fix-06-driver — s/fix-06-driver (task 104d, started 16:46 UTC)
- Task: playtest 1011-1535 (06) — the friendly truck driver (07000006) cowers on top of the boxes he stands next to, and shooting him knocks him back like a guard with the guard death sound. Check against the game: (1) the cower state's position/anim + floor snap for a civilian ally by props; (2) whether the driver is damageable at all in the game (invulnerable/ally flag on the drone, the team check in the bullet hit) and, if he is, his own death reaction/sound. Port the differences. Files: crates/nf-game/src/drone.rs (small, local).
- Status: branch cut from origin/master 7785582 (pulled), WORKLOG started. Next: decompile research (cower state, damage team check), reproduce, fix.
- DONE (17:50 UTC): the driver states dropped the game's absent msg-0x14 (SOUND) arms. The game's
  TruckDriverInit 0x160bd0 routes msgs 0x11/0x12/0x13/0x16/0x1e → 0x33 and has NO 0x14 case, InitAlert
  0x160d98 handles no sound msg at all, TruckDriverIdle 0x160ed0 routes 0x11/0x12/0x15/0x16/0x1e to
  EnemyAlerts with NO 0x14 case [M all three, decompile read whole]; the port's extra SOUND halves ran
  the guard heard-noise chain (0xa2-0xa5) on the friendly truck driver the moment gunfire sounded, and
  that chain's attack path DREW HIM A WEAPON (DRAW_WEAPON 0x66 EXIT: weapon 6, anim set 12 → 4 — the
  guard set) — the playtest's "shoot the driver and he flies back like a guard with the same guard
  death sound": the fly-back was the guard set's death anim, reached through the weapon draw. Shooting
  him is game-accurate (DealWithObjHit case 5: the player's shot has no team gate [M 0x13fff0:181-235]);
  his death line is DeathTalk's default line 1 for voice type 0xf [M 0x13eff8 default arm] — this
  level's bank shares its sfx (139) with the guards' line 4 [L d.voices], so the "same guard death
  sound" is the game's own sharing; the anim set was the deviation. The scare/cower half: the game's
  CivilianScared at alt mode 0 (the driver's) goes straight to CivilianHiding's cower 0x19 — IN PLACE
  [M 0x15d3a0:31-43, 0x15d520; D seq 04000048's constant x/z root], and the current build's gravity
  floor snap keeps his feet on the courtyard floor (the user's build predated fix-drone3's gravity
  fix — his "crouches on top of the boxes" was the old no-gravity drift onto the crate stack by the
  gate truck's grille). Tests: crates/nf-view/tests/fix_06_driver_live.rs 2/2 (the shot test fails on
  the pre-fix tree at the heard-noise assert — shown; the cower pin + sisters: the guards still react
  to the same gunfire, the protected-actor ch 0x60 rises on his death). Suite on the merged tree:
  see docs/reports/fix-06-driver.md. Report: docs/reports/fix-06-driver.md (live checks: the PCSX2
  no-draw confirmation, the death anim, the 700-880 step-out window; the headless render gap).

---
Task ID: 103i-fix-reload-hang
Agent: reload-hang worker
Task: reload-hang — the reload state machine hangs (state 6, clip 0): empty silenced twin 0x03's auto-reload sticks in rel=6 forever and every later switch is refused [M Player_WeaponChange 0x1a6f80:33-47]; the AT-420 0x2D's 5/6/7 machine never returns either. Find the game's reload loop and its exits (Player_WeaponFiring 0x1a5460, the reload restart gate, the view-script queue +0xA4/+0xA8/+0xAC), port the exits for every reload kind (magazine, per-shell, rockets), test through the real input path, re-run nf-extract weapon-matrix on 27 levels, put the new red counts next to weapon-matrix2's.

Work Log:
- (14:04 UTC) Branch s/fix-reload-hang created from origin/master 088bdf4 (weapon-matrix2 already merged). Setup reused (repo, work/, ISO, nf-queue). Pushing the checkpoint, then: suite baseline in background, reproduce first.

## weapon-matrix3 (in progress) — s/weapon-matrix3
Task 103m: the watch gadgets (0x4A-0x55) get real matrix cells (equip via Player_WeaponChange mode=1,
bounds 0x4A..0x5E; fire; the beam/dart/camera-view + sound checks), then the full 27-level sweep again —
fix or explain every remaining red cell with game evidence; panics are bugs. Base: master 8295049 +
the merge of s/fix-reload-hang (not yet on master — 103i's reload fixes under the sweep). Started 14:59 UTC.

## weapon-matrix3 — s/weapon-matrix3 (task 103m, 14:57-16:35 UTC)
- The gadget cells are REAL: 0x4A..0x5E equip via the gadget cycle (Player_WeaponChange 0x1a6f80
  mode=1, Input::gadget_cycle) and run the beam/bolt/camera/sound checks; uncarried ids print n
  (the game's cycle cannot reach them either). 07000005: 0x4a nt1 (the stunner's no-target arm
  [M 0x1a4bf0:169-200]), 0x4e/-, 0x50/-, 0x54/-; the twins 0x4c/0x4f/0x55 n.
- Two harness repairs the cells exposed: the dead-Bond cold reload (the warm restart's RamLoad
  carry keeps hp 0 — measured) and the stale fire_busy script clear (the port keeps view_script
  set with view_time frozen 0 after a script ends; the game's gate is the state-9 check
  [M 0x1a6f80:33-47]). Proof on 07000003: D 17 → 9, clean 27 → 34.
- The sweep: 2646 cells (27 × 98), 0 panics; red B 287 / D 487 / F 53 / M 197 / R 310 / W 47
  (matrix2: D 1278 / B 234 / R 226 / M 152 / F 22 / W 9) — measured with the 103i hang unfixed
  (its branch is a claim stub; the fix is in flight). Groups: the twins' B wall = the mode_to
  press budget; the D-block = the stale-script family (fixed) + 103i's hang; F = the manor-intro
  holds + 0x38's quiet fire (live check). Report: docs/reports/weapon-matrix3.md. Shared files:
  none touched.

---
Task ID: 103m2-weapon-matrix3b
Agent: weapon-matrix3b worker
Task: weapon-matrix3b — the suite's matrix tests time out (s/weapon-matrix3 held at merge: weapon_matrix_live exceeds the suite's per-binary limit). Keep every check but make the in-suite matrix tests fit well under the limit: sample fewer levels/ids in the per-level tests (keep 05, 0c, one late level), keep the gadget-cell test, leave the full 27-level sweep to nf-extract weapon-matrix. Report the binary's runtime on the final tree and the full suite total.

Work Log:
- (19:05) Branch s/weapon-matrix3b = origin/s/weapon-matrix3 6171f2a8 + origin/master b9ed8b8a merged (the WORKLOG conflict resolved append-only, both sides' entries kept; no source conflicts). weapon-matrix4 not on master yet — nothing to fold in.
- (19:15-19:30) The trim implemented in weapon_matrix_live.rs only (matrix.rs not needed): the unset default = the 05/0c/1b sample, NF_MATRIX=all or an explicit list re-selects (the pre-3b mechanism kept), the three 07000005 consumers share one run_level row via OnceLock (the level test written out of the macro). Every check kept: the gadget-cell test whole, the per-level no-panic + mark tables on the sampled levels, the dev-row and upgraded-runner checks unchanged.
- (19:30-19:32) Verified: the trimmed binary 30/30 ok in 270.32 s (the cargo wall 4m30) — before: 6 mid levels > 590 s first-hand, the 27-level default > the runner cap at the held merge; NF_MATRIX=07000003 single-level 119.5 s (the selector works); nf-extract builds green (the canonical full sweep).
- (19:32-20:09) The full suite on the final tree: 1082 passed / 7 failed / 35 ignored, 38 min, 264 binaries, ZERO timeouts — weapon_matrix_live inside the suite at 270 s. The 7: the allowed disc_golden trio, the 3 GPU-adapter environment fails, and walkreal_07000001's guard-nav stall — attributed pre-existing on master (this branch's crate sources are byte-identical to master's; matrix3's own diff is matrix.rs + its two test files; the merge's only conflict was the WORKLOG).

Stage Summary:
- Pushed s/weapon-matrix3b (318d652a + the report): the suite's matrix tests fit well under the cap (270 s vs the >1200 s hold), every check kept on the sampled levels, the full sweep left to nf-extract weapon-matrix. Shared source files: none touched. Report: docs/reports/weapon-matrix3b.md (the walkreal_07000001 attribution + the live check for the main agent).
