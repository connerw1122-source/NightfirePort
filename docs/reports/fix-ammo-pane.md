# fix-ammo-pane — the HUD ammo pane: counts, the mode label, and when the name shows

Task: port `HUD_UpdateAmmoPane` 0x19a7f0 and its create (`HUD_CreateAmmoPane` 0x1a0ed8) so the bottom-right
pane matches the game: at 0700000c's walkway with the sniper out the game shows the bullet-strip icon,
"5-30" and the fire-mode label "Single", while the port showed only the weapon NAME. Branch
`s/fix-ammo-pane` (the task's header, rule 1 and the push sections all say `s/fix-ammo-pane`; the final
line of the task said `s/ammo-pane` — I used the name repeated four times and noted the mismatch here).

## What the pane actually is (evidence)

`AmmoPane` 0x2BCFB8 = the pane 0 CREATE tag {512, 448, 0x1a0ed8, 0x19a7f0, `AmmoSprInfo` 0x2BCE58, 8·4,
0x18} → the pane at (496, 394) after `HUD_ValidateXY` [L data, docs/fields/hud-panes.md — the port's
`AMMO_AT (496,394)` already matched]. Its 8 sprites: [0] the count text, [1] the fire mode, [2] the weapon
name, [3] a HIDDEN text sprite (layer 255, style 0x30cc80 — no reader in 0x19a7f0), [4] a HIDDEN 68×22
bar (layer 255, no reader), [5]/[6] the filled/empty bullet strips (`BulletImg.189` 0x2BEA38 rows per ammo
kind), [7] the Q-Claw/grapple 3D marker. So the visible "ammo icon" is the bullet strip — the port already
drew it. The findings below are from the disassembly (`scripts/mdis.py`, capstone) because the decompiler
output for this function is mangled (`unaff_s0..s8` prologue junk) and it DROPS a store:

- **0x19aa9c (delay slot of `b 0x19ab44`): `sh $zero, 0x20($s6)`** — while `BL+0x95E` runs, the count
  text's rt+0x20 (the text length) is cleared, i.e. the counts hide during a switch. sprite.md had this
  live-verified; the C decompile does not show it. The port already hid the counts while switching — no
  change needed, the comment now cites the real instruction.
- **The timer rule (0x19abbc–0x19ac30)**: every frame, if `drone+0x63` (the switch REQUEST, per
  docs/port-map/weapon-switching.md) ≠ `drone+0x62` (the current weapon) AND the two weapon records' +2
  words differ, `BL+0x95E = (u8)(int)FRAME_RATE` = 60 [L WORKLOG:588 FRAME_RATE 60]; then in the SAME
  update the timer decrements (0x19ac04–0x19ac14). Net: pending frames read 59, the completion frame 58,
  and the name covers the whole draw animation plus the decay. While the timer runs the name is on layer
  0x1c and the fire mode on 0xff; at rest the reverse.
- **The name label reads the REQUESTED weapon**: 0x19ac58/0x19ac98 load `drone+0x63` and pick
  `weapon_data[w]+0x38` (SP; MP reads +0x3c behind the `MPSettings` 0x2a4920 check) — during the pending
  draw the pane announces the weapon being drawn TO.
- **Ammo kind 0x1f (0x19a7f0:129-136)**: `clip' = min(max − clip, max − 12)`, `max' = max − 12` (u16
  arithmetic) — a DEPLETED display — used by the percent text (kinds 0x14/0x1a/0x1f → `clip'·100/max'`)
  and by both strips (filled height clip'·px/round of the full max'·px/round column, the filled strip's
  v starting (max'−clip')·px/round down the texture). The kind-0x1f weapons are the Phoenix Samurai pair
  0x32/0x33 (+ the MP ids) [L ee_snapshot +0x90 = 0x1f, +0x92 = 100].

## What was wrong in the port, and the fixes

1. **The name window missed the pending draw and started at the flip** (`scene.rs` `hud_tick`, shared
   file — small hook, listed here): the old code armed `weapon_label_timer = 60` ONCE when the weapon id
   CHANGED (the completion tick), tracked through a `hud_weapon` last-seen byte, and that byte's 0 init
   armed a spurious 60 at level load. The game re-arms every pending frame. Fix: arm while
   `switch_request != weapon && switch_request != 0` and the records' +2 words differ; drop the flip arm
   (`hud_weapon` kept as a retired field). Pre-fix, the headless timer test failed at tick 0 (the timer
   already decaying instead of re-arming); post-fix pending frames read 59 and completion 58.
2. **The pending name was the OLD weapon's** (`hud.rs`): the pane's switching branch used
   `name_label(gun.weapon)`; the game reads `weapon_data[drone+0x63]+0x38` — the requested weapon. Fix:
   while switching with a differing request, resolve the name label of `switch_request`.
3. **Kind 0x1f had no transform** (`hud.rs`): the port showed `clip·100/max` and strips of `clip`/`max`
   raw for the Samurai's overcharge display. Fix: the depleted `min(max−clip, max−12)` of `max−12` for
   both the count text and the strips (the >87 → 16× strip scale now also checks the transformed max, as
   the game's `sltiu $v0, $s0, 0x58` at 0x19ad10 does on the transformed value).

The rest of the pane already matched the game and now carries the disassembly citations: the counts/strip
hidden while switching, the `flags68` bit-5 "no ammo" count clear (0x19aa84 delay-slot store), the
percent kinds, the "%d-%d" clip-spare with the spare at `BL+0x170+kind·2` (0x19ab30 `lh $a3, 0x170($v0)`),
the strip geometry (`x = count.x − width·k`, `h = clip·px/round`, v-start `(max−clip)·px/round + v`).

## How it was verified

- **NF_ISO tests** (`crates/nf-view/tests/ammo_pane_live.rs`, 3 tests, all green post-fix; the timer test
  failed pre-fix — "tick 0: left 59, right 60" then every pending tick):
  - `the_four_weapons_pane_text_labels_and_icons_match_the_game` — P2K 0x06 (kind 2, cap 16, "16-0",
    mode-0 label 0x0500000e = "Semi"), Winter Tactical Sniper 0x1e (kind 7, cap 5, "5-0", 0x05000010 =
    "Single", name 0x05000029 = "Winter Tactical Sniper"), Militek MGL 0x2a (kind 0xd, cap 6, "6-0") and
    the Stunner gadget 0x4a (kind 0x14, cap 100 → the percent text "100" — the charge display). The label
    ids and icon rows assert against the game's own bytes [L `work/ee_snapshot.bin` 0x2BF150;
    `BulletImg.189` 0x2BEA38: kind 1 = (26, 0, 14, 4), kind 7 = (0, 111, 24, 5), kind 0xd = (32, 179, 17, 6),
    kind 0x14 = (62, 121, 1, 1), kind 0 = all-zero]. Sister assertions: percent kinds never show
    "clip-spare"; kind 0's strip row is empty (weapons without ammo draw no strip).
  - `the_level_start_switch_pins_the_timer_then_it_decays_to_the_rest_pane` — the real repro path: the
    level's own loadout switches to the sniper (the load line "switch request 30"), every pending frame
    reads 59, completion 58, decay to 0, and the pane's inputs at rest are the sniper's — clip 5, spare
    30, count text "5-30" (the compare capture's numbers).
  - `the_kind_1f_display_is_the_depleted_clip_of_max_minus_12` — the Samurai's display: full clip → "0",
    clip 56 → "50", clip 12 → "100". Sisters: the formula never exceeds 100; the Samurai pair's kind/cap
    asserted against the snapshot.
- **Renders** (0700000c `--bond 14.22,16.28,-139.54,0.986,-0.269 --skip-cutscenes --give 0x1e`, software
  vulkan; before = pre-fix binary, after = post-fix):
  - frame 90 (at rest): before and after identical — bullet strip + "5-30" + "Single" at the bottom
    right, the task's game capture ✓ (`shots_tmp/before_f90.png`, `after_f90.png`).
  - frame 30 (inside the name window): before and after show the name alone ("Winter Tactical Sniper") —
    the pane correctly hides counts/strip/fire mode while `BL+0x95E` runs ✓.
  - frame 2 (the pending draw, after only — this is where the fixes are visible): the name of the
    REQUESTED weapon already shows while the P2K is still in hand; pre-fix this frame had no name at all
    (the old timer only started at the flip). Note nf-view's PCSX2-free renderer has no CRT filter; the
    main agent's captures remain the colour/placement authority.
- **Neighbouring regressions**: `fix_hud_live` 6/6 and `hud_playtest_live` 5/5 (the old switch test
  "a_weapon_switch_shows_the_name_alone_then_the_ammo_and_fire_mode" included).
- **Suite**: see the total line below (final tree = origin/master eb52fb56 + this branch; master had not
  moved when the run started).

## Suite

Final tree (origin/master eb52fb56 + this branch; master had not moved when the run started), the fast
runner `tools/test/suite.py` (per-binary limit 240 s):

```
1122 passed, 7 failed, 39 ignored  (39 min)
```

The 7 are exactly the allowed/pre-existing set — no new failures: the 3 `disc_golden` tests (allowed on
the slim ISO), `TIMEOUT weapon_matrix_live` (it also timed out in the last published baseline at the
600 s limit; capped at 240 s here), and the 3 pre-existing pylon failures
(`probe_pylon_render::probe_render_pylon_only`, `fix_0c_pylon_live` × 2 — the last baseline's
"3 pre-existing pylon"). The previous baseline on the production code was 1065 passed / 7 failed /
35 ignored; this tree adds this task's 3 tests (and other workers' merged tests) with the same failure
set. (Bookkeeping note: an earlier suite launch died on a missing PATH and a second overlapped it on the
shared `suite.log` before I caught it; the numbers above are from the clean final run's own `suite.out`
total line, and its FAILED list is quoted verbatim above.)

## What's left

- The `obj+0xF6` gadget-device displays (0x19a7f0:137-171): the player-object scratch values 0xb/0xc/0x10
  replace the counts with device charge numbers (0xb also short-circuits the whole pane after two
  sprintfs). The port does not model the player's obj+0xF6 scratch yet (no reader/writer in nf-game), so
  this needs that state first — not a hud.rs-only change.
- Sprite 7, the Q-Claw/grapple 3D marker (weapons 0x50/0x51): nearest 'Q'-tagged object with line of
  sight, `View_3DPoint2Screen` projection, white → magenta within `weapon_data+0x53DC` range and
  `BL+0x95F == 2` (0x19a7f0:301-350). Needs a 3D→screen hook in the overlay.
- The MP name label (`weapon_data+0x3C` behind the `MPSettings` 0x2a4920 check) — SP-only port.
- The fire-mode byte source: the game reads `BL+0x1BB + Player_AmmoIndex()·0xC` (per ammo slot); the port
  keeps `inventory.fire_mode` per weapon id — equivalent for every single-ammo weapon, divergent only if
  two share-a-slot twins can hold different modes at the same instant.

## Live checks for the main agent

1. **The pending-name rule** (PCSX2, 0700000c start): `gdbwatch`-style watch on `BL+0x95E` (the BLData
   base is obj+0xE0; obj from `PLAYERS` 0x2D88E0) during the level's opening sniper draw — expected: 60
   re-written every frame while `drone+0x63 (0x1e) != drone+0x62`, then −1/frame from 59; the screen
   shows "Winter Tactical Sniper" alone, then "5-30"/"Single".
2. **The Samurai depleted display** (any level, give 0x32): fire the beam a few times — expected: the
   count text = `min(88, 100−clip)·100/88` (NOT clip·100/100), the strip depletes from the top of an
   88-row column, both from `HUD_UpdateAmmoPane` kind 0x1f.
3. **The Stunner charge** (07000005, give 0x4a): the count text should be the raw charge number (clip of
   cap 100 → percent = charge), the strip 1 px wide per charge (kind 0x14: w = 1, hpr = 1).
4. **nf-view `--bond` spots** (0700000c): `14.22,16.28,-139.54,0.986,-0.269` frame 2 (name during the
   pending draw), frame 30 (name alone, counts hidden), frame 90 (strip + "5-30" + "Single").
