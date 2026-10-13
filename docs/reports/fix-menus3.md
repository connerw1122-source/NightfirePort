# fix-menus3 — the remaining front-end gaps (2026-10-13)

Branch `s/fix-menus3` (based on `origin/s/fix-menus2` — master does not carry the menus2 rebuild yet,
and every gap below is measured against it). Files: `crates/nf-view/src/menu.rs`,
`crates/nf-formats/src/hud.rs` (the font glyph walk only), `crates/nf-view/tests/menus_flow.rs`,
`docs/systems/frontend.md`, `docs/WORKLOG.md`.

## Done

1. **4:3 centred framing** [L the main agent's PCSX2 captures 2026-10-13]. The overlay drew textured
   quads with `Anchor::Stretch`, which fills the whole window — the menus stretched across 16:9.
   Every menu quad and text call now goes through `Anchor::Centre`, the window's central 4:3 band
   (`menu.rs` `quad640`/`quad_uv`/`plate`; identical to Stretch at a 4:3 window, so 4:3 renders are
   unchanged). Test: `the_menus_draw_inside_a_centred_4_3_band` (fails on the old Stretch — a vertex
   reached |x| > 0.75 at 16:9 — and pins the 4:3 sister).
2. **Title page** [M menu-def page 0x40000009: prompt 0x100000fa, legal plate 0x10000197
   (50,332,540×108)]. The legal text wraps inside its plate with `wrap_px` (font-measured greedy
   wrap) and every line is centred at the plate's centre x=320 — the old 92-column run went off the
   right edge. The "Â©" bug: `Font::layout` walked BYTES of the CP-1252-decoded UTF-8, so "©" (C2 A9)
   drew glyphs 0xC2 + 0xA9; `layout` walks chars now — one glyph per source byte, matching
   `__Font_DrawText` 0x1ee598's own byte walk over the raw bank. Tests: `the_legal_line_wraps_...`
   (real 0x375 from the disc, ≥ 3 lines, all ≤ 500 px, © = one glyph 0xa9) and the difficulty/
   mission description wraps in `the_descriptions_wrap_inside_their_plates`.
3. **Main menu backdrop** [L the menu-def PAGE records]: page 0x40000002 has NO static art control —
   its backdrop is the aux movie 0x07330048 (list pages 0x07350048), i.e. the looping menu FMV. The
   trimmed ISO stubs every movie (0-byte PSS files), so the port keeps the dark plate; drawing a
   still through `movie.rs` needs the retail ISO (`NF_MOVIE_ISO`) — listed under Left. The item rows
   are LBL text controls (167,262/294/326,306×28) per the page data — the port's text rows are
   correct data; the stylised logo look is the game's own menu font (not parsed; reported).
4. **Headers** [L the page data]: "Select Codename / Select Difficulty / Single Player" are LBL text
   controls (0x1f2/0x1f8/0x14c at (50,40,376×17)) — NOT images. Same menu-font caveat as 3.
5. **Hint ✕** [M `specialchar` 0x2dd8d8, 17 × 0x18 entries, found via the ACTION.ELF symtab]: the
   game draws the ~A/~B/~X/~Y/~V/~H/~S/~W/... tokens from texture 0x03000075 with per-token
   (u, v, w, h, advance). `hint_bar` now uses that table (✕ = 235,1,19×19); the guessed 0x03000042
   atlas tiles and the letter-X fallback are gone. Tests: `the_hint_icons_come_from_the_specialchar_
   table` (✕ and △ uv pins; fails on the old code which had no 0x03000075 quads at all).
6. **Ring preview clip** [M the emblem controls: preview f1 0x1d/0x1e UNDER ring f1 0x1c — lower
   layers draw on top]. The preview draws as 24 chord strips of the emblem rect's inscribed circle
   (`circle_clipped_quad`), so nothing pokes out of the ring; the strip corners stay inside the
   circle (the first cut clipped at each strip's centre and poked out at its edges — the test caught
   it). Test: `the_ring_preview_is_clipped_to_the_ring_circle` (every preview vertex inside the
   640-space circle; fails on the old full square).
7. **Mission previews** [M `Menu_UpdateWheel` 0x1f8388 sends `sp_level[cursor]+0x00` as msg 0x24 to
   the preview control 0x10000020; the menu-def has no per-row texture]. `sp_level+0x00` IS the
   preview texture hash — the port's table had dropped that lane. CAMPAIGN is now 12 × (preview,
   name, description, level) with the 12 hashes read from the EE snapshot at 0x2df2e0 [L], all 12
   present in the 07000048.bin texture archive [L]. Locked rows dim to 0x60 and show +0x14's
   "Continue playing to unlock this mission." [L USATxt 0x01000009]. A fresh profile unlocks rows 0
   AND 1 [D sp_level's boot state in ACTION.ELF .data, file offset 0x1d91e0]. Test: row 0 draws
   0x03000085 and NOT 0x03000084 (the exact reported symptom), plus the EE cross-check in
   `the_campaign_is_sp_level_on_the_disc`.
8. **Descriptions wrap** inside their 540px plates (`wrap_px`; the mission description's old call
   even re-joined its wrapped lines back into one string).

Full suite (`NF_ISO=... cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats
-p nf-audio -p nf-view -- --test-threads=1`): nf-game 206/206, nf-formats + nf-audio + nf-view green
except the 3 allowed `disc_golden` failures (the onfoot ISO's stubbed /DRIVING/MIS01.MUS BIGF files;
pre-existing, not touched by this task). 17/17 `menus_flow`.

## Render commands for the main agent's comparison

```
cargo build --offline --release -p nf-view
./target/release/nf-view $NF_ISO --page start --shot /tmp/m3_start.png
./target/release/nf-view $NF_ISO --page main --shot /tmp/m3_main.png
./target/release/nf-view $NF_ISO --page codename --shot /tmp/m3_codename.png
./target/release/nf-view $NF_ISO --page difficulty --shot /tmp/m3_difficulty.png
./target/release/nf-view $NF_ISO --page missions --shot /tmp/m3_missions.png   # cursor on Paris Prelude
./target/release/nf-view $NF_ISO --page missions --unlock-all --shot /tmp/m3_missions_row1.png
```
What each should show vs PCSX2: a centred 4:3 picture with black side bars; the title's legal block
inside its plate with ©; the codename/difficulty/missions headers at (50,40) in the game's own font
(ours is the stand-in SerpLight); the ✕/△/○/□ hint icons from the specialchar tiles; the missions
ring showing Paris Prelude's own art, clipped to the circle; descriptions wrapped inside their plates.

## Live checks for the main agent

- The main menu backdrop: confirm the real page's background is the 0x07330048 FMV (pause on it in
  PCSX2 vs the movie's first frames). If so, the port's follow-up is a movie-still backdrop when
  NF_MOVIE_ISO points at the retail image.
- The menu font: the headers/rows in the captures look like a stylised logo font, but the menu-def
  controls are LBL text. A `gdbwatch.py` check on the font-draw call during the Select Codename page
  (which FONT table does `__Font_DrawText` 0x1ee598 read — FONTSerpLight or another?) would settle
  whether the port should parse a second font.

## Left

- The main-menu FMV still (needs the retail ISO's 0x07330048.PSS; the code path is the dark plate
  today).
- The game's menu font (the stylised headers/rows look).
- The 640×480→512×448 mapping keeps the port's mx/my scaling; a pixel-exact check against PCSX2
  might want `FixupResolution` 0x21b908's own numbers instead (the current mapping preserves
  fractions of the screen, so proportions are right).
- The specialchar icon baseline: the icons draw at (y − h + 12) as a suggested alignment; the
  captures should confirm.
