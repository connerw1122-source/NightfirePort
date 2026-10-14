# fix-hud-radar — the HUD radar: what the game actually shows

**The headline: the game's single-player HUD has no radar.** The bottom-left ring the task asks
about is the health pane's decorative ring, and the port already draws it with the game's exact
numbers. The radar — blips, range, rotation, the radar-on gate — exists only as the multiplayer
pane 14, which a single-player level never creates. That answer is not from memory: it is a
snapshot-, decompile- and disassembly-backed chain, listed below. The MP radar itself is now
ported and number-pinned in `crates/nf-view/src/hud.rs`, ready for the MP milestone; the SP draw
path is unchanged (zero visual delta in SP, which is what the game does too).

## The SP evidence chain (why there is no SP radar)

1. `HUD_Init` 0x199e88:32-37 walks `PaneList` when `MPSettings+0x384 == 0`, else `MPPaneList` [M].
2. The SP boot snapshot: `MPSettings` 0x2a47a0+0x384 = **0** → PaneList [L ee_start_05.bin].
3. The SP PaneList slots **13 and 14 are NULL** [L docs/fields/hud-panes.md, decoded from 0x2be5b8];
   the RadarPane (0x19f4e8 create / 0x19f6e8 update, RadarSprInfo 1·24) is `MPPaneList[14]` only
   [L 0x2be9e0+14·4 → CREATE at 0x2bdd90].
4. The SP snapshots' runtime HUD (BL 0x16cf0f0 → HUD 0x16c8630): panes {0..12, 15..21} created —
   **13/14 absent** in both ee_start_05.bin and ee_snapshot.bin [L].
5. Even the radar's inputs are absent in SP: MPGame 0x2a4980's eight player slots all NULL and the
   radar flag MPSettings+0x452 = 0 [L].
6. `HUD_UpdateHealthPane` 0x19b9c8 — the SP bottom-left pane — contains no blip logic at all: the
   ring is sprite 15, tinted the health colour with alpha 140 [M 0x19b9c8:205-207]. `HUD_Update`
   0x19a088 adds nothing radar-like [M].

So on 05/06 with guards around, the game shows: the health ring, the health/ armour bars, no blips,
no rotation, no range. The "radar stays" line in docs/live/2026-10-13-weapon-modes-pcsx2.md was
observed **in a multiplayer Arena match** (Skyrail, a bot) — the MP radar, consistent with all of
the above. The task's "context ring swap" hide condition is the open 100n item: the only 007-logo
draw path drives sprite 16 at (256, 80) top-centre (the Bond-Moment zoom), and nothing in
ACTION.ELF rewrites the ring sprite's uv or position [M, fix-context-hints2 item 2] — no swap
mechanism was ported and none is guessed here; that item's live checks stay pending.

## What was ported (crates/nf-view/src/hud.rs, additive)

- `RadarObj` — the `RObj` record (@0x318e70, stride 0x20): world pos, ARGB
  (0x7f7f7fff no-team / 0xd22d35ff vs 0x2d61d2ff per team), uv-table kind 0/1/2/3/4/6/7
  [M MP_GetRadarObjects 0x189840:53-231].
- `radar_project` — the `HUD_RadarUpdate` 0x19f6e8:108-153 projection, **verified against the
  disassembly 0x19f758-0x19fa60** (the decompiler's scratch-vec aliases `Target_293`/
  `DAT_00319574/78` are wrong there): view = R·(p − cam) (`Mat_Inverse` 0x1e3578 = {Rᵀ, −t} +
  `RotPreTransVec` 0x1e49e8); the direction's unit x/z negated [M disasm 0x19f888-0x19f8b0];
  scale = min(len·0.02, 1) — the ring's edge at 50 units [M :122-125]; x = −ux·scale·(ring_w·0.45)
  + ring.x + 0.5 + ring_w/2, y the same with z and ring_h [M :130-136]; cap 24 [M :108,153];
  layer 0x1c; uv/w/h from `TexUV.291` @0x2becf0 (kind 0 = 4×4 players, the objectives 8×8)
  [L ee_start_05.bin].
- **The height cue is a dead store.** The game computes clamp((obj.y − bond.y)·0.125, ±1) and
  stores it to the scratch vec's y slot (0x319574) — and nothing in ACTION.ELF ever reads it; the
  position formulas read 0x319570 (x) and 0x1978' (z) only [M disasm 0x19f88c-0x19f8d4 vs the reads
  at 0x19f904/0x19f974; repo-wide grep: no reader]. The port computes and drops it the same way,
  and a test pins the drop (a blip 8 units up lands exactly where a level one lands).
- `radar_ring_layer` — the ring sprite hides (layer 0xff) unless `MPSettings + player·0x30 + 0x28`
  (the per-player radar flag the MP menus' "Radar" toggle writes), then 0x1e [M 0x19f6e8:102-105].
- `radar_ring_rect` — the ring's created rect (512, 0) + the texture's size [L RadarSprInfo[0]
  @0x2bdd60], clamped by `HUD_FixMP` 0x19b110:73-92 into the viewer's rect
  (viewer+0xf0/0xf4/0xf8/0xfc = left/right/top/bottom [L ee_start_05 viewer+0xf0..fc = 0, 512, 0,
  448; HUD_ValidateXY 0x19a250's 0x80/0x100 arms pin +0xf0/+0xf8 as the viewer's screen x/y]).
  For the full-screen viewer that lands the ring at **(512 − w, 0) — the top-right corner**, not
  the bottom-left [M chain].
- `radar_frame` — the pane's quads for one frame (ring + centred blips, flag 0x800 [M 0x19f4e8:58]).
  NOT called by `overlay`: the Sim has no MP match state, exactly as the SP game has none.

## Verification

- New tests `crates/nf-view/tests/hud_radar_live.rs`, 4/4 green:
  `the_sp_hud_on_05_shows_the_health_ring_and_no_radar` (07000005, guards around the spawn: no
  radar texture loaded, the ring quad at (20,341) 87×86, colour 0x52a88b, alpha 140),
  `the_sp_hud_on_06_shows_the_health_ring_and_no_radar` (the 07000006 sister),
  `the_radar_projection_matches_the_game` (hand-derived numbers: the 0.02 range clamp at 50 units,
  the 0.45 half-extents, the centre +0.5 offsets, the dead height cue, TexUV sizes 4×4/8×8, the
  0x1c layer, the team colour), `the_ring_layer_gate_and_the_fixmp_clamp` (0xff/0x1e gate; the
  full-screen and split-screen clamps).
- The SP pins are honest positives (the port already matched the game — that IS the finding): they
  would have caught a port that drew blips or misplaced the ring, and they pin the game's numbers
  for the future MP work.
- Neighbour chunks in the foreground (the nohup'd runner dies between tool calls in this sandbox —
  the same limitation the last three sessions recorded): hud_radar_live 4/4, fix_context_hints_live
  6/6, fix_hud_live 6/6, hud_playtest_live 5/5, menus_flow 34/34, nf-view lib 8/8, devwin_flow 1/1,
  dev_menu_flow 5/5, pickup_live (nf-game) 4/4 — 73 tests, 0 failures. The full four-crate suite did
  not fit the window; the touched surface is `overlay`-adjacent nf-view code and `overlay` itself is
  byte-identical (the diff adds functions below it), so the blast radius is the HUD families above.
- No RNG draws added; no wall-clock in Sim; `drone.rs`/`scene.rs`/`player.rs`/`lib.rs` untouched.

## Visual check

Nothing visible changed: the SP overlay draw path is unchanged and the MP radar has no MP mode to
render in. The before/after renders at any spot are identical by construction; the SP pins above
pin the ring's numbers headlessly. The main agent's PCSX2 renders remain the arbiter for the two
items below.

## What's left / live checks for the main agent

1. **Confirm the SP truth on the retail ISO** (settles the task's premise): PCSX2, 07000005 at the
   spawn with guards visible — the bottom-left ring must be the plain health-tinted ring, NO blips
   at any alert level. nf-view: `--bond 16.5,29.6,58.6,-2.01,0.24` on 07000005 — same.
2. **The MP radar's on-screen truth** (needs a MP match, Skyrail): (a) is the ring top-right?
   [M chain: created (512,0) + HUD_FixMP → (512−w, 0)]; (b) read pane 14's sprite 0 fields
   (+0x2c/+0x2e) for the ring texture 0x03000088's real size — the port's `radar_ring_rect` takes
   it as an input; (c) a bot ~25 units ahead: the blip should sit between the centre and the edge
   (scale 0.5), and rotate opposite Bond's turn [M the negated view x/z].
3. **The pending 100n 007 check** (unchanged): the Bond-Moment channel watch + the full-res
   bottom-left screenshot, per fix-context-hints2's live checks — this task adds nothing to it.
4. The MP milestone wiring: `radar_frame` needs an MPGame/MPSettings stand-in in the Sim, the
   0x03000088/0x0300016f textures from the MP map archives, and the floating name-tag half of
   `HUD_RadarUpdate` 0x19f6e8:176-314 (View_3DPoint2Screen + the LOS check + the per-player text) —
   deliberately not ported here (no MP state to test it against; ~140 lines of the update).
