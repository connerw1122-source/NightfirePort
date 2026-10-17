# unproven-tests-gameplay — report (2026-10-17, branch s/unproven-tests-gameplay)

Task 755: settle the 2026-10-16 audit's unproven gameplay-test numbers — every row gets evidence
([M] decompile address, [D] disc bytes, [L] EE-snapshot/live measurement) or `// [S] unverified`
out of pass/fail; a contradiction fixes the test. All 12 rows are handled; two produced real
code/test fixes (rows 2 and 9), the rest are evidence upgrades. The row/evidence/change table:

| Row | Evidence found | Change |
|---|---|---|
| fix_hud_live.rs ~89-97 (bar fade 15, close 16/tick) | Close rate 16 = the pickup pane's state+0xc written by `HUD_CreatePickupStatusPane` 0x1a1140:10 (info/objective panes write 0x10 too, mission 0x20); consumed per state-4 tick at `HUD_UpdateStatusPane` 0x19c310:331 (read `piVar4[3]` at :37) [M]. Fade = alpha `life*17` while `life < 15` at 0x19c310:262-268, life drained 1/tick by `Text_UpdateMsg` 0x1b11d8:49 (−FRAME_RATE_MUL = 1.0 at 60 Hz [M GS_SetRefreshRate 0x17d2b8:15; L ee_snapshot 0x30d0d8]). Life 180 = `Pickup_Handler` 0x18e650:326 `fVar22 = 180.0` → Text_AddMsg [M]. | Comment block added with all four citations; assertions unchanged. |
| hud_playtest_live.rs ~108 (crouch icon "centred (48,48)", 0x800) | **CONTRADICTION — the audit row is wrong.** The icon's flags are its template's: `HealthSprInfo[19]` at 0x2bd050+19·0x2c reads flags **0x1000** (clip-to-screen), NOT 0x800 [L ee_snapshot]. The game centres only under `rt+0x22 & 0x800` — `View_DrawSprites` 0x1e5290:69-74 subtracts w/2,h/2 solely on that bit (the crosshair/blips carry 0x800; no writer ORs it into the crouch sprite; `Sprite_Create2` 0x1de3d8:132 copies template+0xA once). The game writes the position (48,48) [M `HUD_UpdateHealthPane` 0x19b9c8:307-313] → (48,48) is the sprite's TOP-LEFT. | Test fixed to pin the top-left (quad starts at 48,48) with the citations; port `hud.rs` crouch draw changed from centred-on-(48,48) to top-left-at-(48,48). |
| hud_playtest_live.rs ~127-154 (16:9 pickup-bar centring) | The game's own 4:3 layout centres the bar: `PickupStatusSprInfo[2]` (BSS 0x319830, boot-copied from 0x2FC460 [M 0x1a0058:77-104]) reads x=128 w=256 → 128..384 centred on 512 [L ee_snapshot]. 16:9 is the port's own design (the PS2 had no 16:9). | The 16:9 assertions demoted to a logged `[S] unverified` observation (out of pass/fail); a proven 4:3 centring pin added ([L] template data). |
| loading.rs / async_load_frame_loop.rs `dots_lit` (11 vblanks) | `VBlankInt` 0x124088 **is in the decomp export now** (`00124088_VBlankInt__Fi.c` — no ghidra export needed): per vblank the u8 `BlobTime` (0x30d721) decrements, on underflow reloads 10 and steps the u8 `BlobToDraw` (0x30d720) by the s8 `BlobDir` (0x30d722); direction flips at BlobToDraw 0/6 (:18-29); `ResetLoadingBlobs` 0x1249c0:6-8 seeds 0 [M]. 10 countdown ticks + the underflow tick = one step per 11 vblanks. | Citation upgraded in `loading.rs` `dots_lit` to the exact lines/addresses; the port's 11-tick ping-pong matches; assertions unchanged. |
| models_misc_live.rs ~338-345 / fix_weapon_effects_live.rs ~196/205 (muzzle smoke 8/7) | models_misc_live.rs already pins the proven 128 [M disasm: `0x1a43f8 addiu $v0,$zero,0x80` → `0x1a442c sw $v0,($sp)` — re-verified in the ELF myself; same call stores the type 7 at 0x1a4434 and rgba 0x48403080 at 0x1a4400/0x1a4438]. fix_weapon_effects_live.rs still said "~8 ticks [*suggested*]" — stale and contradicting. | Both stale comments updated to 128 [M] with the disassembly addresses; the window assertions unchanged. |
| fix_laser_watch_live.rs ~149-174 (drain 31±2/60, idle refill) | The state-0xb writer IS in 0x1a5460: disasm `0x1a6538 andi $v0,$v0,0x400` → `0x1a6558 sh 0xb,0xf4(view obj)` [M]. **But the audit's suspected charge writer is not there**: that arm's `swc1 1.5f,0x98(anim instance)` (0x1a6568) is the held-fire ANIM's play speed (`AnimScriptAddSpeed` 0x1c6e20:31 writes +0x98 as its speed param), not the charge. The pane reads the beam instance's u16 +0x110 / f32 +0x104 via the hashtable 0x2000194 walk [M `HUD_UpdateAmmoPane` 0x19a7f0:160-172]; the +0x104/+0x110 writer remains unlocated in the decompile. | Comment upgraded: state-0xb writer cited [M], the checked-and-rejected 1.5f documented; drain stays [D] (measured), refill stays [S]; live check listed below. |
| fix_context_hints_live.rs ~177/196 (prompt window 6.0) | The arm chain [M]: `ThirdIcon_Update` 0x196e80:20-26 walks the icon obj's +0xd0 contact list and writes the type into `BLData+0x95F` (the once-only 0xb4 post at :42); the list is `Collide_Update` 0x1ea398:138-148's per-tick distance sort (+0xc, 1e8 = miss). No flat 6.0 constant exists — the window is the collision sphere test against the icon's level radius. | Comments cite the chain; the 6.0 window stays [S] between the two live pins; assertions unchanged. |
| fix_satchel_tripbomb_live.rs ~235-237 (7th removes oldest, obj+0xEC) | [M `Bullet_CollisionHandler` 0x12ce80:336-378]: the walk collects class-5 objects matching weapon (+0xe0→+0x44) and owner (+0xe0→+0x30), counts (`bVar4 = iVar20 < 6`), tracks the MINIMUM +0xEC and on the 7th marks it `+0xfe |= 1` — the dead bit (`Explode_Propagate` 0x17c518:78, `MP_isPlayerOrBotDead` 0x18abc0:20 skip on it). obj+0xEC = the birth tick: `Pickup_Handler` 0x18e650:347 writes `GameState+0x34` (the global tick counter [L ee_start_05: 0x2a3768+0x34 = 55165 mid-game]; same stamp in `MP_ReSpawn` 0x185368:90). | Doc comment rewritten with the mechanism; "spawn order ≡ birth-tick order" tie-break marked [S]; assertions unchanged. |
| drive_hud.rs ~99 / driving_ai_physics.rs ~309-310 (countdown floor, reverse) | **Timer: the game has NO value clamp.** DRIVING.ELF `FUN_00209c88` 0x209c88:9-24 computes `value + rate·Δticks` unclamped — a countdown descends past zero; only the DISPLAY is gated (`FUN_0023e580` 0x23e580:58 draws nothing at `integer seconds ≤ 0`) and beeps stop at 0 (:19). **Reverse: no hard cap and no runaway** — gear-state gate (brake held >29 ticks with vel ≤ 0, `FUN_0018cef8` 0x18cef8:184-208), thrust decay `MAXREVACC − max(0,|v|−SPEED_CUTOFF)·RATE` floored at 0 (:397-407; vanquish 2.4/10/0.075 [D vanquish.atr] → zero thrust by ≈42 m/s), and the VU0 integrator's global |v| ≤ 100·scale clamp (`FUN_001f89e0` 0x1f89e0:73-79,145-164). | **Port fix**: `MissionTimer::update` no longer floors (the raw signed value descends; a later add recovers real time as in the game); new `display_seconds()` getter models the 0x23e580 gate for the draw code; header docs rewritten [M]; unit test `the_countdown_descends_past_zero_and_the_display_gates_it` added (passes). The reverse side already matched — `drive_accel`'s doc now cites the full mechanism. |
| walk_07000005_live.rs ~66 (channel 0x45 set by hand) | The game's channel writers [M]: `Trigger_Activate` 0x1b2318:30, the script event ops (`Script_EventHandler` 0x1d94c0:166,187 — `switch_channels[ch] = value` from the SSTREAM), and mission init (`Mission_Init` 0x182270:106-116, `switch_channels[rec+0xc] = rec+0xd`). 07000005's mission entry [L ee_start_05: base 0x2a4354, records 0x2a3e28, count 0x2a4360 = 1] holds `46 00 01 45 …` — its init write CLEARS ch 0x46 and sets the objective's state, it does NOT set 0x45. So 0x45's setter is the level's SCRIPT stream — the mechanism is [M], the exact script/op is not extracted yet. | Test comment cites all three writers and the record finding; the hand-set stays with the [S] caveat; docs/levels/07000005.md §2 updated; live check listed below. |
| fps_determinism_live.rs ~76 (4-tick catch-up cap) | The game runs at most ONE `GameFlow_Main` per main-loop iteration; `PS2FramesToSkip` is clamped [1,2] with a resync (`GameCount = VBlankCount`) [M `main` 0x109080:71-79, 95; BGFMV path :64-67] — the game catches up by speeding the clock (MUL=2), never by bursting ticks. | The 4-tick cap labelled a PORT CHOICE [S] in the test doc (the assertions pin the port's own scheduler arithmetic, which is legitimate pass/fail — the game-behaviour claim is gone). |
| subtitles_live.rs ~218-223 (1 step per tick) | Parity holds [M]: `Script_Update` 0x1da998:33,39 adds FRAME_RATE_MUL to the script and per-stream timers; `FRAME_RATE_MUL = 60.0/FRAME_RATE` (`GS_SetRefreshRate` 0x17d2b8:15) = **1.0 at 60 Hz** [L ee_snapshot 0x30d0d8 = 1.0] — the port's fixed-60 Hz +1/tick matches the fresh-boot game. The FMV path's MUL=2 (GS_SetRefreshRate 30) is not modelled by the fixed clock — noted [S], no behaviour change needed. | Comment upgraded with the parity citations; assertions unchanged. |

## How it was verified

- Every [M] citation above was read from `work/decomp/ACTION.ELF/` (and `DRIVING.ELF/`) in this
  sandbox; the four disassembly claims (muzzle Gas_Init's 0x80 at 0x1a43f8/0x1a442c, the 0x400 arm's
  state write and 1.5f) were re-verified against `work/elf/ACTION.ELF` with capstone in this session.
- Every [L ee_snapshot / ee_start_05] byte claim (HealthSprInfo[19] flags 0x1000, PickupStatusSprInfo[2]
  x=128 w=256, FRAME_RATE_MUL = 1.0 @ 0x30d0d8, GameState+0x34 = 55165, the 07000005 mission record
  `46 00 01 45`) was read directly from `work/ee_snapshot.bin` / `work/ee_start_05.bin`
  (file offset = EE address) in this session.
- Tests run: `cargo test --offline --release -p nf-game --lib driving_hud` → **2 passed, 0 failed**
  (the fresh-snapshot pin + the new no-floor countdown test). `cargo build -p nf-game` (release, dev
  profile) clean.
- The full suite (`tools/test/suite.py`) was started in the background at 06:47 but its build phase
  silently failed on this sandbox: the ALSA dev headers were installed into a user prefix and
  `alsa-sys`'s pkg-config probe emitted no `-L` for it, so every binary linking alsa failed at link
  (`rust-lld: error: unable to find library -lasound`). Fixed at 08:09 (edited the extracted
  `alsa.pc`'s libdir to the prefix, cleaned alsa-sys/alsa, restarted the suite) — but the sandbox
  hit its stop-work time before the rebuild finished, so **the suite's total line is not available
  for this tree**. The edits are comment/citation-only except rows 2 and 9; row 9's changed code is
  covered by the passing unit test above, and row 2's draw change is covered by the existing
  `the_crouch_icon_sits_top_left_at_48_48` assertion (updated in the same commit).

## What's left

1. Run the full suite on this tree (the runner works now that alsa links) and confirm the total;
   the crouch-icon change also deserves the main agent's render check (below).
2. The laser charge writer (+0x104/+0x110) remains unlocated — the live check below should settle it.
3. 07000005's ch-0x45 script op extraction (the mechanism is proven; the exact script isn't).
4. `Mission_Init`'s full record-table walk (the 0x2a4354 mission table → per-mission init records)
   could be ported so the port clears ch 0x46 at mission start the way the game does.

## Live checks for the main agent

1. **Laser charge writer** (fix_laser_watch_live): gdbwatch `readwrite` on the beam instance's
   charge fields during a held watch-laser beam on 07000005. The pane walks view-obj `+0x878` →
   (its `+0xd8` == hashtable(0x2000194) ? `+0xe0` : `*(+0x1c)+0xe0`) and reads u16 `+0x110` / f32
   `+0x104` [M 0x19a7f0:160-172]. Expected: +0x104 drains ~0.5/tick while the beam runs and refills
   while idle; the writer's address settles the last open piece of the drain/refill model. If
   gdbwatch can't watch a heap instance, a `fieldprobe` poke of +0x104 (e.g. to 50.0) should move
   the on-screen meter.
2. **ch 0x45 writer** (walk_07000005_live): gdbwatch `write` on `switch_channels+0x45` = 0x26fc90+0x45
   (0x26fcd5) from a FRESH boot of 07000005 through the intro. Expected: one write of 1 from the
   script system (`Script_EventHandler` 0x1d94c0) shortly after the truck-ride objective arms —
   the recorded PC gives the script op to extract.
3. **Crouch icon anchor** (render check): `--bond` at the 07000005 start, crouched. The icon (tex
   0x03000173) should now draw with its top-left corner at (48,48) on the 512×448 frame — i.e. its
   centre right/below that point by half the texture — matching `HUD_UpdateHealthPane` 0x19b9c8's
   (48,48) write with NO 0x800 centring ([L] template flags 0x1000).
4. **Driving timer** (drive_hud): in a race mission, let a countdown pass zero and then trigger the
   script's `EAddTimer`. Expected (game): the clock face stays hidden (≤ 0 integer seconds) but the
   raw value keeps descending, and an add on the negative value brings the face back at
   `added − overshoot` seconds [M 0x209c88:9-24 + 0x23e580:58] — confirming the port's new raw-value
   model.

## Usage

not visible to me (no token, cost or context-window figures are exposed in this environment).
