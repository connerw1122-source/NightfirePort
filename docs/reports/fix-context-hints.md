# fix-context-hints (100h) — the top hint bar and the context icon

Two PCSX2 symptoms (main agent, 2026-10-13): standing at a usable thing the real game shows
(1) a hint bar at the top — "Hint: Use Watch Laser" in front of the 07000008 safe — and (2) a
HUD context indicator; the port showed neither (its HUD stayed quiet and the radar unchanged).
Both systems are now ported from the decompile with the game's own label ids, textures, timing
and gating. One visual caveat at the end (the "orange 007" description) with the live checks
that settle it.

## The game's context system (what the decompile says)

**Hint markers** — placement kind 0x33, `Hint_Create` 0x1b3248 → class 0x43, `Hint_Update`
0x1b3380 [M]. The instance is built off the placement's params: param 1 = the sound id
(`Sound_Play(100.0, sound, 0, 0)` on fire; 0 = silent), param 0 = the text label
(`Txt_BindLabel(param 0)`), param 3 = the on-channel, param 4 = the off-channel
[0x1b3248:33-37, 0x1b3380:17-22]. `Hint_Update` fires ONCE when a class-3 object (the player)
is in the marker's obj+0xD0 chain (the straddle-cel occupancy `Collide_Update` fills) with the
channels satisfied: it posts `Text_AddMsg(…, 1, label, …, 0xb4)` — TXTMSG **type 1**, life
0xB4 = 180 ticks — and sets obj+0xFE bit 0, the delete flag [0x1b3380:24-42; obj-tag.md].
The 07000008 safe's marker is label **0x0200000A** ("Hint: Use Watch Laser" [D USATxt.dat
entry 1822]); a second marker by the safe carries 0x010001E9 ("Press Action to open the
control panel" [D USATxt.dat entry 1489]) with a r 3.889 sphere that does not reach the safe
spot.

**Third icons** — `ThirdIcon_Update` 0x196e80 (placement kind 0xFA, class 0x42, ported earlier
by fix-wire2/fix-08-walls) writes the context id into the player's `BLData+0x95F` while a
class-3 object is in the icon's chain, and the type-6 icon posts the generic "Press Action"
hint (label **0x1000291** [D USATxt.dat fixups[1]+0x291]) through the same `Text_AddMsg` on the
icon's 4-tick instance timer while the player is not creeping [0x196e80:15-17, 38-53].
`+0x95F`'s only non-0xFF writer in the whole ELF is 0x196e80:52 (the other two accesses are
resets: `Player_CollisionHandler` 0x1949a8:22 and `Player_StandAtNewPosition` 0x190d88:78);
`Hint_Create`'s only caller is `parsemap_create_dynamic_objects` case 0xfa — no script-created
icons [D decompile grep].

**The INFO pane** — `MsgInfoStatusPane` 0x319810, `InfoStatusSprInfo` 0x319730 [L
ee_start_05.bin], driven by `HUD_UpdateStatusPane` 0x19c310 — the same state machine the pickup
pane already ports. The bar is `InfoStatusSprInfo[2]`: 480×26 at (16, 0) — the screen's TOP
edge — a 1-texel column of the HUD sheet 0x0300007A (u 190, v 0..56) with 5-px caps (u 185/191)
riding the ends [0x19c310:342-347], colour 0x7F7F7F alpha 0x8D, layer 30. The text is the
centre-justify style sprite at template (128,16) + pane 128 = screen 256 (centred), font 2,
colour 0x7D6D59, black shadow, layer 29, wrapping at `TextMsgFormats[1]`'s 480 px [L
0x2BF110+4], fading over the last 15 ticks (alpha = time·17); the close shrinks 16 px/tick
(the CREATE's slide 0x10 [L 0x319810+0x14]).

**The context icon** — `HUD_UpdateHealthPane` 0x19b9c8:273-303. While `BLData+0x95F != 0xFF`
the pane's sprite texture comes from the table at **0x2FC168**[ctx] = 0x03000064+ctx
(`hashtable_set_sprite` 0x1ca3d8 fills w/h/uv from the texture), layer +0x38 = 0x1d (29 — the
draw layer, not alpha), colour 0x7f7f7fff, the runtime position **(400, 48)** is written every
shown frame, and `ThirdIconTimer` (0x30CCE8) = 30; on 0xFF the icon fades out over that timer
(alpha = timer·255/30) [0x19c160-0x19c1d4]. Only three of the table's ids exist on the disc
[D tex-list]: 0x03000066 (winch — ctx 2, Q-Claw grapple), 0x03000067 (hanging man — ctx 3,
wire grab), 0x0300006A (standing figure — ctx 6, use); the others never resolve and the sprite
stays hidden, exactly like the game's failed lookup.

The x/y order of the runtime write (+0x24 = 400, +0x26 = 0x30) is pinned three ways [M]:
the disasm 0x19c148/0x19c14c (`sh v1,0x24(s4)` with v1=400; `sh a0,0x26(s4)` with a0=0x30);
the ammo pane's bullet column (0x19a7f0:255-262) where the per-bullet COUNT-scaled term — the
vertical stacking — goes to +0x26 and the width-scaled term to +0x24; and `HUD_FixMP`
0x19b110's x-clamp of +0x24 against the viewport width. The crouch icon writes (0x30, 0x30)
into the same fields and the user's PCSX2 play puts it top-left, so the HUD space is 640×448
y-down: the context icon draws at x 400, y 48 — top-centre-right, NOT the bottom-left ring.

## The port

- `crates/nf-game/src/player.rs` (shared file, additive): the `Hint` struct + `hints()` — every
  kind-0x33 placement resolved through its map object's world sphere exactly like `wire::icons`
  [M `Control_BuildWorldSph` 0x133070 via `build_LinkToRoom` 0x1c9680:40]; unresolved
  placements are skipped (the game never creates their object).
- `crates/nf-view/src/scene.rs` (shared file, additive): per-hint straddle-cel sets + fired
  flags (the one-shot), the arm test = the straddle set AND the sphere volume (the game's
  obj+0xD0 chain is the coarser cel test; the intersection keeps fix-08-walls's wall gate and
  the control-panel marker silent at the safe spot — its [S] measured pin); the type-6
  "Press Action" post on the icon's 4-tick timer while not creeping; the INFO queue
  (`info_labels` → `info_msgs` drained in `hud_tick`, each `Msg` carrying its label id for the
  tests), the INFO bar's close animation, and the context icon's `ThirdIconTimer` + last-ctx
  fade state.
- `crates/nf-view/src/hud.rs`: the INFO bar draw (480×26 top bar + caps from 0x0300007A, the
  centred wrapped text in font 2 0x7D6D59 with the shadow, the fade and the 16 px/tick close)
  and the context icon draw (the three ctx textures added to `TEXTURES`, (400,48), layer 29,
  the 30-tick fade on 0xFF).
- `docs/systems/hud.md`: the "Context icon (health pane sprite 18)" and "Hint / info bar
  (pane 4, TXTMSG type 1)" sections with all of the above citations.

## Verification

`crates/nf-view/tests/fix_context_hints_live.rs` — 5 tests, all green on the final tree,
asserting NUMBERS and LABEL IDS only (never the disc's strings):

- `the_safe_shows_the_watch_laser_hint_once`: at `--bond 155.19,43.12,-113.38,1.57,0` the
  0x0200000A marker fires once — the INFO queue's front label == 0x0200000A, life in
  (100, 180] (the 0xB4 ticks down only by the steps since it fired), no second copy after 60
  more ticks (the one-shot); sister: `icon_ctx == 0xFF` at the safe and no context-icon quad
  (07000008's only 0xFA placement is the zipline's ctx-3 icon, 21 units away in cell 12 —
  fix-08-walls's straddle set, across the wall).
- `the_control_panel_hint_stays_silent_from_the_safe_spot`: 0x010001E9 does NOT fire from the
  safe spot (its r 3.889 sphere ends 6.7 away) — the spatial gate.
- `the_grapple_spot_reads_context_6_and_shows_the_figure_icon`: at
  `--bond 16.5,29.6,58.6,-2.01,0.24`, `icon_ctx == 6`, the INFO front label == 0x1000291 (the
  type-6 post), and the HUD frame draws texture 0x0300006A; sister: walking to
  (104.95, 24.96, 108.82) clears ctx to 0xFF and the icon is gone within the 30-tick timer.
- `away_from_contexts_the_hud_shows_none`: at the 07000005 spawn — no ctx, no INFO label, no
  message, no context-icon quad.
- `dump_context_data_07000008_and_05`: the data dump (kept runnable for `--nocapture`).

**Fail-first**: the pre-fix port (5247021^) has no `info_msgs`, no `hints`, no context draw at
all (`git grep` on that tree: zero matches for `info_msgs|0x0300_0064|fn hints` in
crates/{nf-view,nf-game}/src) — neither symptom had anything to assert against, which is the
reproduced symptom itself; the tests target the new APIs, so a pre-fix run cannot compile
rather than print red. The three positive assertions encode exactly the two missing elements.

## The "orange 007" caveat (the one open visual)

The driver described the indicator as "the HUD ring bottom-left turns into an ORANGE '007'
icon (at the safe, and at the 05 grapple point)". The decompile says the 0x95F-driven sprite:

- draws at (400, 48) — top-centre-right in the HUD space the crouch icon verified top-left;
- shows the 0x2FC168[ctx] texture — and the three disc textures are TAN BADGES (winch /
  hanging man / standing figure; dumped from the ISO during this session and inspected as
  images — none is an orange 007 logo).

So the orange-007-at-the-ring observation is NOT explained by the `BLData+0x95F` system, and
the port follows the decompile. Two possible resolutions, both settleable live (below): the
game has a second, script-driven indicator (scripts CAN create sprites — `Script_SpriteStart`
0x1d9300), or the description conflates the ctx badge with a Bond-moment flash. Note also the
safe: 07000008's only third icon is the zipline's, so the port shows NO icon at the safe —
if the real machine shows one there, `BLData+0x95F`'s live value names the context and we find
the missing source.

## Live checks for the main agent

PCSX2 (tools/trace/fieldprobe.py, tools/trace/gdbwatch.py):

1. **The icon's screen position** — at the 05 grapple spot, gdbwatch the health-pane sprite's
   +0x24/+0x26 halfwords while in range: expected 400 then 48 every frame [M 0x19c148-0x19c14c],
   +0x38 = 0x1d, +0x10 = 0x7f7f7fff. Screenshot: does the badge appear top-centre-right?
2. **`BLData+0x95F` write watch at BOTH spots**: at the grapple, expected writes of 6 from
   0x196e80; at the safe, expected NO non-0xFF writes [M the only writer]. If the safe's value
   moves, the value names the ctx of the missing icon source.
3. **fieldprobe poke at the safe**: set 0x95F = 6 → the expected result per the decompile is
   the tan figure badge 0x0300006A at (400, 48); = 2 → the winch badge. If an orange 007
   appears at the iris instead, the table/position read is wrong and this report's caveat is
   confirmed against the code.
4. **ThirdIconTimer 0x30CCE8**: expected 30 while in range, counting down per frame after
   leaving [M 0x19c1a4].
5. **The safe hint**: watch `Text_AddMsg` on entering the safe marker: expected ONE call
   (vol, 0, 1, Txt_BindLabel(0x0200000A), 0, 0xB4) [M 0x1b3380:33]; confirm the on-screen bar
   is at the TOP.

nf-view renders (`--bond x,y,z,yaw,pitch` — what each should show):

- `07000008 155.19,43.12,-113.38,1.57,0`: the TOP hint bar with the text bound from
  0x0200000A; NO context badge.
- `07000005 16.5,29.6,58.6,-2.01,0.24`: the tan figure badge at top-centre-right (400, 48) +
  "Press Action" in the top bar; compare position and art with PCSX2 (checks 1 and 3).
- `07000005 104.95,24.96,108.82,0,0`: neither.

## The full suite (final tree = origin/master merged, commit e511da5)

This sandbox kills background processes (the nohup'd fast runner died twice between tool
calls), so the suite ran in foreground chunks with `tools/test/suite_chunk.py` — the same loop
as `tools/test/suite.py` (per-binary `--test-threads=1`, incremental suite.log), resuming
across calls:

- 226 test binaries; logged total **948 passed / 3 failed / 31 ignored** — the 3 are exactly
  the allowed `disc_golden` trio (`bigf_header_fields_behave_as_documented`,
  `bigf_tables_match_the_golden_manifest`, `directory_tree_matches_the_golden_manifest`).
- `weapon_matrix_live` hit the 300 s per-binary cap on this slow sandbox (the cap, not a
  failure — it had already passed matrix_01..14 in that run) and was re-run in full separately:
  17 (01..14) + 4 (15,16,1b,41) + 7 (43,46,48,49,4a,4b, the sequence) = **28/28 green**.
- Grand total: **976 passed, 3 failed (allowed), 31 ignored**.
- `fix_context_hints_live` 5/5 on the final tree.

## What's left

- The orange-007 identity and the safe's icon (if any) — the live checks above.
- The Q-Claw ammo-pane marker turning magenta while the context is 2
  [M `HUD_UpdateAmmoPane` 0x19a7f0:330-336] is documented in hud.md but not drawn (its
  3D-positioned marker sprite isn't ported) — a small follow-up.
- The hint TEXT binds from the disc's USATxt.dat through the port's `lang` (the strings are
  never asserted, per the task: numbers/label ids only).

## Files

`crates/nf-view/src/hud.rs`, `crates/nf-view/src/scene.rs` (shared: additive hooks — the hint
markers, the icon timers, the INFO queue, the fade state, one `label` field on `Msg`),
`crates/nf-game/src/player.rs` (shared: additive — `Hint` + `hints()`),
`crates/nf-view/tests/fix_context_hints_live.rs` (new), the merged `Msg` literals in
`fix_objective_banner_live.rs` / `fix_objective_messages_live.rs` (`label: 0` one-liners),
`docs/systems/hud.md`, this report, the WORKLOG entries.
