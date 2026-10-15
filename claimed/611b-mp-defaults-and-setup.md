## Your task: `mp-defaults-and-setup`, the MP menus' default values and the per-agent setup steps

The main agent drew the MP pages (merged 611 data) and compared them with PCSX2 on a fresh start (2026-10-15). Three
of `FrontEnd`'s MP defaults (crates/nf-view/src/menu.rs, `FrontEnd::new`) differ from what PCSX2 shows on its first
visit to the pages:
- Player Mods row 4 "Location Damage" (`mp_player_mods[1]`, MPSettings+0x1C8): PCSX2 **On**, the port Off.
- Enviro-Mods row 1 "Respawn" (`mp_envmods[0]`, +0x1C0): PCSX2 **Random** (value 2), the port Far (1).
- Enviro-Mods row 3 "Explosive Scenery" (`mp_envmods[2]`, +0x1D4): PCSX2 **Locked** (value 0x10 → label 0x1000076;
  P_MPENVIROMODS 0x210030 binds that item), the port Off. Find the reward gate that makes it 0x10 and port it.
Find where the game initialises MPSettings (the boot defaults, the codename profile's saved MP settings) and make
the port's defaults the game's, with evidence. Also: the per-agent setup step after Select Map (P_MPSETUP 0x20d7a8 +
C_RBMPSETUP 0x20de58) shows THREE prompts in PCSX2 on Team Arena — Choose Team (MI6/Phoenix image + ◀ ▶), Choose
Character (the character portrait), then **Health Handicap** (green cross image, values like 0) — the port's
`mp_setup_state` has team/character only; add the handicap step (its values/labels/textures from the handler) and
getters for each step's prompt label, image texture hash and value text, so the main agent can draw it. Tests for
the defaults and the three-step flow (ids/numbers only).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/mp-defaults-and-setup`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox3 at 2026-10-15T21:02Z
