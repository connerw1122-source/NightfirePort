## Your task: `menus2`, make the front end match the real game

Read `docs/reports/fix-menus.md` (merged) first. The flow works, but the main agent compared it with the real game in
PCSX2 (2026-10-13), and the port's screens are plain text on an empty background with the wrong order and names.
**What the real game shows, in order** [L PCSX2, NTSC-U]:

1. **Title:** a full-screen art image (Bond, red/orange), the "007 Nightfire" logo, "Press the START button" (yellow),
   and the EA/Danjaq legal text below it, all centred. (The port: "007: NIGHTFIRE / PRESS START" text, right of centre.)
2. Start → a short **movie** (a gun-barrel/eye zoom), skippable, then:
3. **Main menu:** a backdrop image (Bond with gun, a round 007 emblem), the logo, the items as stylised logo-text images:
   **NightFire / Multiplayer / Codenames** (not "Options"), a one-line description of the highlighted item ("Start your
   007 adventure."), and button hints at the bottom ("Scroll", "✕ Select").
4. NightFire → **Select Codename** (the save profile): "New Codename / Play without saving / <saved names>", a ring
   emblem on the left, the description line, hints "✕ Select □ Create ↕ Scroll △ Back". "Play without saving" pops a
   confirm box ("You won't be able to save your progress using this default profile. This controller style is a
   traditional one-thumb setup…  Press ✕ to continue or △ to cancel.").
5. → **Select Difficulty** (BEFORE the mission list): **Operative / Agent / 00 Agent** (the port has Agent/Operative/007),
   each with its description ("For trainee Agents. Enemies are slower and weaker, and deal less damage. In-game hints
   are provided."), the ring emblem turns green.
6. → **Single Player** (the mission list): the ring shows the highlighted mission's preview image, the names include
   "Paris Prelude", "The Exchange", "Alpine Escape", "**Enemies Vanquished**" (the port shows "Enemies Vanish", so the
   port's names are off), the description line ("Drop in on Raphael Drake's reception and rendezvous with undercover
   agents."), locked missions are dimmed and can't be scrolled to, hints "✕ Select □ Dossier ↕ Scroll △ Back".

**Do:**
- Take every string from the game's own language bank: fix-explosions2 reimplemented `Txt_LoadLanguage` 0x1caad0 /
  `Txt_BindLabel` 0x1cad60 exactly (the FixupTable over USATxt.dat). Use that, not hand-typed names. Pin the label ids of
  every menu string in a test.
- Find each page's layout and assets from the page table (the 16-byte entries the report found; the handlers
  P_START 0x20a408, P_MAIN 0x20a7b0, P_NFSELECT 0x20aec0, P_NFDFCTY 0x216bf8, the codename page) and the front-end map
  07000048's objects/sprites: the backdrop images, the ring emblem, the stylised menu-item images, the button-hint icons.
  Draw them at the game's positions on the 512×448 screen.
- The order Title → movie → Main → Codename → Difficulty → Single Player; the unlock rule for the list (what a fresh
  profile can pick) and the Dossier page; the Codename page can be a stub that offers "Play without saving" only (the
  save file is `210-fix-progress`'s) but it must look like the game's.
- The ASCII ✕/□/△ glyphs: use the game's own button-icon sprites.
- `--shot` must draw the menu overlay too (today the headless shot path skips it), so the main agent can compare.

Tests: the page sequence, the label ids per page, and the sprite hashes per page (numbers only), with sisters. List
`nf-view <iso> --shot` commands per page for the main agent's side-by-side with PCSX2.

Files: `crates/nf-view/src/{menu,lib}.rs`, the HUD/sprite drawing it needs, `docs/systems/frontend.md` (new).

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail. Report the count.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/menus2`.

---
claimed-by: sandbox3 at 2026-10-13T14:38Z
