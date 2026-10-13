## Your task: `menus3`, close the remaining gaps between the port's menus and the real game

fix-menus2 (merged) rebuilt the front end from the disc's own page data. The main agent rendered it
(`nf-view <iso> --page <start|main|codename|difficulty|missions> --shot`) next to PCSX2 captures of the real menus and
found these differences [L, 2026-10-13]:

1. **Framing:** the real game draws the 640×480 menu space as a centred 4:3 picture with black bars at the sides; the
   port stretches it across 16:9, so every page is wider and things land off-place. Draw the menus 4:3, centred.
2. **Title:** in the real game the logo, "Press the START button" and the legal block are all horizontally centred, the
   legal text wraps inside its plate (50,332,540×108), and "©" shows as ©. The port's legal text runs off the right
   edge, isn't centred, and shows "Â©" (the CP-1252 decode fixed in `Lang` doesn't reach this draw path, or the
   font's glyph lookup maps the byte wrong). Fix the text layout (justify/wrap per the control's fields) and the
   glyph.
3. **Main menu:** the real page has a full backdrop (Bond with the gun, the red/orange art and the round 007 emblem) and
   the items are the stylised NightFire/Multiplayer/Codenames logo-text, centred under the emblem. The port shows a
   plain dark page with plain-font items on the left. Find the main page's background/emblem/item-art controls in
   the page data (or the background movie the PAGE record's aux code names, `M_PAGE+0xC4`: the main menu may be a
   looping background FMV in the real game; if so, a still from the movie path once movies land, and the right static
   art meanwhile).
4. **Headers:** "Select Codename / Select Difficulty / Single Player" are stylised logo-text images in the real game,
   plain font in the port.
5. **Hints:** the ✕ button is drawn as the letter "X"; use the atlas tile like the others.
6. **The ring preview:** a visible square box around the green emblem / the mission preview. Clip the preview to the
   ring (the game's sprite has an alpha mask or a circular UV; check the COMPONENT/SPRITE data).
7. **Mission previews:** with the cursor on Paris Prelude the port shows the snowy castle; in the real game that picture
   belongs to The Exchange. Check the per-row preview mapping (sp_level's record fields).
8. **Descriptions** run off the right edge (difficulty and missions): wrap them inside their control's width.

Render each page with `--shot` and compare against the PCSX2 references the main agent will take. List the commands in
your report.

Files: `crates/nf-view/src/{menu,lib}.rs`, `crates/nf-formats/src/hud.rs` (text/glyph only), `crates/nf-view/tests/menus_flow.rs`.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail. Report the count.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/menus3`.

---
claimed-by: sandbox6 at 2026-10-13T16:54Z

---
## Report (done, 2026-10-13T18:24Z)

