## Your task: `menus-real2`, the front end: the main panel art, title font, button glyphs, the dossier rows

101k-fix-menus-real (merged after its suite) rebuilt the front end: the rings and their pictures, the list layout, the
centred wrapped descriptions, the hint rows and the title page now match PCSX2. The main agent's side-by-side
(`nf-view <iso> 07000048 --page <p> --cursor 1 --frame 60 --shot`) against the PCSX2 captures still shows:
1. **The main page's panel art:** the real main menu has the big orange curved lens panel (gold rim along its lower arc)
   with Bond in a tuxedo holding a pistol on the right and the 007 gun-barrel ring with the globe above the logo; the
   port's main page shows only the logo and the three items over the backdrop. Find the panel and the Bond art (page
   0x? controls, their textures) and draw them.
2. **The page-title font:** "Select Codename", "Select Difficulty", "Single Player", "Dossier" are in the game's
   bold ITALIC header font in the real game; the port uses the body font.
3. **The footer glyphs:** the real hints show the PS2 button icons (blue X, pink square, the D-pad, green triangle);
   the port's are tiny dots. Draw the game's button sprites at their size.
4. **The Dossier list:** the real page lists Records / Rewards / Gadgets (three rows); the port adds "Weapons".
   Check the page data (it may be locked/hidden by progress) and match.
5. **The backdrop:** the report says it's the aux FMV on the movie plane, drawn only with the `movies` feature and
   the retail PSS. Verify with `--features movies` and `--movie-frame 12` that every page shows the orange streak
   backdrop like PCSX2, and that the no-movies fallback isn't a near-black plate (use the movie's first frame or a
   still if the game has one).
Tests per item (element lists, font ids, sprite hashes, row lists vs the page data); sister assertions; `--shot`
commands per page.

Files: `crates/nf-view/src/{menu,hud}.rs`, `crates/nf-view/tests/menus_flow.rs`.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md). Read `tail -1 suite.out` and
  its FAILED/TIMEOUT lines before you push, and run it once more on your final tree (merged with `origin/master`). Report the total
  line. On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/menus-real2`.

---
claimed-by: sandbox1 at 2026-10-14T06:51Z

---
## Report (done, 2026-10-14T10:03Z)


---
## Report from the worker (done, 2026-10-14T10:04Z)

# fix-menus-real2 — the front end's second pass (2026-10-14)

Branch `s/fix-menus-real2` (from origin/master a4164b9, then merged 6cbe522). Task files:
`crates/nf-view/src/menu.rs`, `crates/nf-view/src/hud.rs`, `crates/nf-view/tests/menus_flow.rs`,
`crates/nf-view/tests/menu_movies.rs` (call-site fix only). **Shared-file hooks (2, both small and
additive, forced by the fixes):**

1. `crates/nf-formats/src/hud.rs` — `HudTables` gains one field `font_header: Font` (own line at the
   end) and `from_action_elf` refactors the existing font parse into a closure used twice
   (`FONTSerpLight`, then `FONTNFont2`). No behaviour change for the existing fields.
2. `crates/nf-view/src/lib.rs` — the four `menu_overlay` call sites pass the new header-font
   argument; the `--shot` movie-still call honours `NF_MOVIE_ISO` (one line) and `--movie-frame`
   defaults to 12 (one line, inside the existing `cfg(feature = "movies")` block).

Evidence tags: [M] decompile function+address, [L] EE snapshot / page-data / tex-list data,
[D] disc, [S] suggestion. The runtime truth for the menu data tables lives in the EE snapshots —
the pristine ACTION.ELF holds **pre-init bytes** at `specialchar`/`FontTable`/`DAT_003096c0`
(they're runtime-initialised), which tripped the first research pass; all table claims below are
from `work/ee_snapshot.bin` (file offset = EE address) and were cross-checked across all four
snapshots (`ee_start_05`, `ee_start_07000001`, `ee_start_0700000a`, `ee_snapshot`) — identical.

## Item 4 — the Dossier list: no gating, a cursor bug (fixed)

The real game's Dossier wheel always has **four** rows. `C_SBDOSSIER_Handler` 0x216eb0:40-43 sets
the scroll range to [0,3] unconditionally on control-finalize; the four table entries at EE
0x2dfcd0 (Records/Rewards/Gadgets/Weapons, 0x280/0x281/0x282/0x19c) all carry unlock flag = 1 [L
ee_snapshot 0x2dfcd0..0x2dfd2f; the Weapons flag byte at 0x2dfd28 = 1, and an ELF-wide scan finds
no reader or writer of it — the only code touching 0x2dfcc0-0x2dfd3f is C_SBDOSSIER's table
constant 0x216f38 and the adjacent SBBOTS table]. `Menu_AddItemsToControl` (the function that skips
flag-0 rows) is never called with this table.

The playtest's "three rows" is the wheel's own 5-slot centred window: `Menu_UpdateWheel`
0x1f8388:148-168 fills slot i with item cursor+i−2 and blanks the out-of-range slots — at cursor 0
that's blank/blank/Records/Rewards/Gadgets, with Weapons one Down press away. Two port bugs put
Weapons on screen:

- `menu.rs` MissionSelect `Action::Side` entered `Page::Dossier` **keeping the missions cursor**;
  the game resets the wheel to 0 whenever the page opens from missions/results/stats [M
  P_DOSSIER_Handler 0x216de0:9-16 — msg 0x2e param 0 → Scroll_SendMessage 0x222810:98-106 rejects
  out-of-range indices].
- Up/Down wrapped with `%` (Records → Weapons on Up); the game clamps [M Scroll_Update
  0x2221b20:186-194 (Down at max) / :202-210 (Up at min) — the wrap needs the +0x144 flag, set only
  by msg 0x69 (0x222810:136-139), which nothing sends to control 0x1000010c].

**Fix:** `cursor = 0` on entry + saturating clamps that return `false` at the ends. Test
`the_dossier_wheel_opens_at_records_and_clamps`: **shown failing on the pre-fix code** (reverted
the two hunks, ran, watched `assert_eq!(fe.cursor, 0)` panic, re-applied), then passing; sisters:
the window at cursor 0 yields rows [0,1,2] only, Up at 0 / Down at 3 are no-ops.

## Item 2 — the page-title font: FONTNFont2, the second face (fixed)

The engine has no italic/embolden flag — `__Font_DrawText` 0x1ee598 draws axis-aligned quads and
`Font_DrawText` 0x1eee48's only format bits are measure/centre/right/shadow/outline. The bold-
italic look is a second font, reached through the page data:

- `Label_Init` 0x218638 stamps the default format on every label: `DAT_003096c0` = `ff 02 fe 03 fc
  64 fb 64 00` = SerpLight + centre [L ee_snapshot 0x3096c0].
- The menu-def's NAME tag (−8) carries a 32-byte **UTF-16** format string per control, compacted
  and sent as msg 0x23 [M MenuManager_Create 0x21b908:255-274; Label_SendMessage 0x218890:100-104
  sprintf's it into the label's format buffer and points label+0xc0 at it — the only per-control
  font override path; no other writer of the default exists].
- I re-walked the menu-def tag stream (kind-8 entry `08000002` at `07000048.bin`+494208; 56 pages,
  891 NAME strings; the walk replicates 0x21b908's cases, CONTROL = 30-byte payload, KEYFRAME =
  5 words + dword — both wider/shorter than the decompiler's C suggests): **the header controls
  (50,40,376×17, name 0x10000001) all carry NAME = `ff 00 01 00 00 00 …`** — compacted `\xff\x01`
  = font select 1. `Font_ParseFormat` 0x1ee3e0 walks to the first NUL only, so the effective
  format is exactly the font select.
- FontTable[0] = {tex 0x03000005, ptr 0x305a58 = FONTNFont2} [L ee_snapshot 0x2dd8b0], and
  0x03000005's atlas (512×128, front-end archive entry 3 sub 2 [L nf-extract tex-list]) **is** the
  heavy italic face [D texture export].

**Fix:** `nf-formats` parses `FONTNFont2`/`FontDataNFont2` (sym-based, same layout as the existing
SerpLight parse) as `HudTables.font_header`; `hud.rs` gains `text_scaled_tex` (the font's own atlas
— `Font_ParseFormat` case 5 loads the selected font's texture); `menu.rs` `page_header` draws the
headers through tex 0x03000005 (0x03000005 appended to `MENU_TEXTURES`). Tests:
`the_header_font_is_the_games_second_face` (both faces parse from ACTION.ELF, same 149-glyph
range, metrics differ) and `the_page_headers_draw_the_header_font_atlas` (the headers' quads ride
the 0x03000005 atlas run; would have found none before the fix — the header drew on 0x03000003).

## Item 3 — the footer glyphs: already correct; pitch pinned (no render bug found)

The port's `SPECIALCHAR` table matches the game's **runtime** `specialchar` exactly: 17 entries ×
0x18 at EE 0x2dd8d8, letter tokens 'A'-'Y', every entry texture 0x03000075, per-token uv/ext/w/h
[L ee_snapshot, all four snapshots identical]. The atlas carries the glyphs at exactly the table's
rects [D crops from the 07000048.bin texture 5 sub 5: (235,1) blue ✕, (235,21) green △, (235,41)
orange ○, (235,61) pink □, plus the D-pad/L-R/R1/L2 tiles the other tokens point at], and
`hud.rs` `quad()` normalises uvs by the texture's real pixel dims. The token scan (~ + next byte)
and the letter tokens both match the game [M `__Font_DrawText` 0x1ee598:211-269; the bound USATxt
strings carry literal `~A`/`~V`/`~B` bytes — dumped 0x218/0x49e/0x54e/0x216/0x208/0x2d0 from the
disc]. The "tiny dots" symptom matches the port's **old** 0x03000042 64×64-atlas guess (uv 235 on a
64-wide texture clamps to the corner pixel); that guess is gone from master (fix-menus-real).

What the port had wrong were test-only numbers, now pinned to the disc: the test fake gave
0x03000075 a 256×256 pitch — the real texture is **256×128** [L tex-list 07000048.bin entry 5 sub
5; identical SHA in all 34 archives] — so every uv-based icon assertion normalised wrongly
(`menus_flow.rs` crate_texture, the ✕/△ test, and pause-menu2's ~J marker test 57/256 → 57/128;
all now cite the tex-list). The wrong glyph identities in the SPECIALCHAR comments (B "circle", X
"square", Y "triangle") are corrected to the atlas's actual glyphs (B=△, X=○, Y=□). If the main
agent's current render still shows dots, that render predates fix-menus-real's table or used a
stale binary — the re-render commands below settle it.

## Items 1 + 5 — the backdrop: the art is in the FMVs; one real --shot bug (fixed)

The main page's orange lens panel, Bond in the tuxedo and the 007 ring are **inside the aux FMV**
(the PAGE records' aux field: main 0x07330048, list pages 0x07350048 [L the parsed pages;
Manager_SendMessage 0x21c858:331-341 plays them looping via Menu_PlayMovie 0x215990 →
psiStartBackgroundMovie 0x1081a8]); the pages carry no static art control for them. The port's
movie plane (movie.rs `Player`, one plane under the overlay) implements that correctly and its
tests pin it. The near-black renders came from three stacked causes:

1. The shipped `target/release/nf-view` is built **without** `--features movies` (the decoder is a
   documented opt-in; not changed).
2. `--movie-frame` defaulted to 0 — frame 0 of the aux FMVs is the fade-in [L fix-menus-real's live
   check; the movie reports pin decodes on frame 12]. Now defaults to 12.
3. A real bug [M lib.rs front_end_menu]: the `--shot` still path read the movies off
   `scene_iso_name()` (NF_ISO) instead of honouring `NF_MOVIE_ISO` like the live loop's
   `App::movie_iso_path`. NF_ISO here is the on-foot image whose 22 PSS entries are all **0-byte
   stubs** [D ISO9660 parse: `/MOVIES/30_FPS`, sizes 0, LBA 0] — so even a movies-enabled binary
   showed the plate. Fixed: `NF_MOVIE_ISO` wins, like the live loop.

**No retail image exists in this sandbox**, so the FMV stills could not be decoded here — the
main agent verifies with a retail image (commands below). The no-movies fallback remains the
flat `0x180c08` plate by design (pinned both ways by `tests/menu_movies.rs`); the game has no
still backdrop (the title page's own full-screen TEX 0x03000074 art is separate), and baking
stills would need retail PSS data — left to the main agent if wanted [S].

## Suite

- Baseline before the work (a4164b9 + WORKLOG commit, this sandbox): **937 passed, 4 failed,
  29 ignored (33 min)** — the 3 allowed `disc_golden` + `weapon_matrix_live` TIMEOUT (600 s,
  matrix_07000015) on the untouched baseline tree — pre-existing, not from this task.
- Final tree (merged with origin/master 6cbe522, commit 052ffde, fast runner `tools/test/suite.py`,
  NF_ISO = the onfoot ISO): **975 passed, 4 failed, 31 ignored (35 min)** — the 3 allowed
  `disc_golden` (`bigf_header_fields_behave_as_documented`, `bigf_tables_match_the_golden_manifest`,
  `directory_tree_matches_the_golden_manifest`) + the `weapon_matrix_live` TIMEOUT (600 s,
  matrix_07000015) that also fails on the untouched baseline tree above — pre-existing, not from
  this task. No CRASH; no other FAILED lines.

- menus_flow on the final tree: **34/34** (all fix-menus-real + fix-menus-real2 + pause-menu2
  tests). nf-formats disc_golden failures = the 3 allowed.

## Render commands for the main agent

```
cargo build --offline --release -p nf-view --features movies   # the decoder is opt-in
NF_MOVIE_ISO=<retail.iso> ./target/release/nf-view <retail.iso> 07000048 --page main      --frame 60 --shot m.png
NF_MOVIE_ISO=<retail.iso> ./target/release/nf-view <retail.iso> 07000048 --page difficulty --frame 60 --shot d.png
NF_MOVIE_ISO=<retail.iso> ./target/release/nf-view <retail.iso> 07000048 --page codename  --frame 60 --cursor 1 --shot c.png
NF_MOVIE_ISO=<retail.iso> ./target/release/nf-view <retail.iso> 07000048 --page missions  --frame 60 --shot s.png
NF_MOVIE_ISO=<retail.iso> ./target/release/nf-view <retail.iso> 07000048 --page dossier   --frame 60 --shot do.png
NF_MOVIE_ISO=<retail.iso> ./target/release/nf-view <retail.iso> 07000048 --page dossier   --frame 60 --cursor 1 --shot do1.png
```

What each should show vs PCSX2: the header in its (50,40,376×17) band in the **bold-italic**
face; the footer glyphs as the PS2 buttons (✕/△/○/□ from the atlas); the dossier at cursor 0
showing **Records/Rewards/Gadgets** with two blank slots above (cursor 1 adds Weapons at the
bottom slot, no wrap); the main page with the FMV's panel/Bond/007 ring behind the rows (needs the
retail PSS); frame 60 ≠ frame 12 (the FMV plays).

## Live checks for the main agent

PCSX2 (spot / action / expected result):
1. Main menu → Dossier (from Single Player, cursor on any mission): the wheel opens with
   Records centred (row 0); Down once → Rewards; Down ×3 → Gadgets is the LAST reachable row at
   3 downs?? — no: rows are Records/Rewards/Gadgets/Weapons, so Down ×3 lands on Weapons (bottom
   slot); Up at Records does nothing (no wrap). Confirm row 0 opens Records; Back returns with the
   wheel where it was.
2. Select Codename / Select Difficulty / Single Player / Dossier headers: confirm the bold-italic
   face (compare glyph shapes with the body rows — the header font's 'S' has the heavy italic cut).
3. Footer hints on every page: the icons are the PS2 buttons (✕ blue, △ green, ○ orange, □ pink,
   gold D-pad), vertically centred on the text line, advancing by their own width.
4. If the port's dossier looks right in PCSX2 but the port's render doesn't: check the selected
   row's dark bar (still the unported runtime art from fix-menus-real's left list).

nf-view (`--page`, no movies needed): the six shots above — the header band centred at
x≈238 (640-space) in the italic face; the dossier window at cursor 0 = three rows; cursor 1 adds
Weapons; no wrap at either end.

## What's left

1. The FMV-backed backdrop render (needs a retail image; the pipeline is fixed and tested).
2. The selected-row dark bar and the gold rule under the header (fix-menus-real's live-check
   items, still unported runtime art).
3. Records/Rewards/Gadgets/Weapons sub-pages other than Records (the port opens Records; the rest
   are reported in fix-menus-real).
4. `pss_movies_live`'s golden sweep still needs a 22-movie image (unchanged from fix-menu-movies2).
