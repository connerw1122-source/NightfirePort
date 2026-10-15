## Your task: `profile-options`, the codename options' data and their effects (no drawing)

The front end's Codenames → Edit Codename → "Edit <name>" wheel leads to option pages the port does not have yet. The
main agent draws the pages; you port **what the options are and what they do**.

Pages and handlers (ACTION.ELF; decomp export in `work/decomp/ACTION.ELF/`, menu-def page records in 07000048.bin
kind-8 entry 08000002 — `crates/nf-view/src/menu.rs` header documents the tag format):
- `P_CNMENU_Handler` 0x213288 / `C_SBCNOPTIONS_Handler` 0x213528: the "Edit <name>" wheel — rows Secret Unlocks,
  Controller Setup, Driving Controller, Advanced Options, Multiplayer Options, AV Options, Codenames (rename).
- `P_CNOPTIONS_Handler` 0x213f08 (Advanced Options): Vibration, Auto Aim, Crosshairs, Crouch (Toggle/Hold), Manual Aim
  (Hold/Toggle), Weapon Auto Switch, Flashing Objects, HUD Always On — each row's value labels and the profile field
  it writes, plus the per-row description label shown under the list.
- `P_CNAVOPTIONS_Handler` 0x214818 (AV Options): Music Volume, Effects Volume (sliders), Subtitles, Multiplayer Split
  Screen, Speaker, Widescreen, Screen Adjust, Restore Defaults, Credits, Die Another Day Trailer.
- `P_CNMPOPTIONS_Handler` 0x2144d8 (Multiplayer Options): Radar, Health Handicap, Auto Aim.
- `P_CNNAME_Handler` 0x213790 / `C_KEYBOARD_Handler` 0x213a98: codename entry (the letter grid).

Deliver:
1. `docs/systems/profile-options.md`: per page, the rows (label ids), the value choices (label ids), the profile byte
   each row reads/writes (offset in the profile/save record, default value) and where the game reads it at runtime
   (e.g. which function checks Auto Aim, Crosshairs, Crouch toggle, Weapon Auto Switch, HUD Always On, the volumes).
2. In the port: an options model on the profile (`nf_game::profile` or a new module) with those fields and defaults,
   saved/loaded with the existing profile code, and the runtime effects wired where the port already has the system
   (auto-aim on/off, crosshair on/off, crouch toggle vs hold, manual-aim hold vs toggle, weapon auto-switch on pickup,
   HUD always-on, music/effects volume into nf-audio). Where a system is missing, say so in GAPS and leave a hook.
3. Tests for each effect (headless sim: e.g. auto-aim off → no lock; crouch hold → standing again on release) and
   for profile save/load round-trips of every field, each with a sister assertion that can fail.

Not in scope: drawing any page (the main agent does), gamepad/vibration behaviour beyond storing the flag (gamepad work
is lowest priority), Secret Unlocks codes (store the field only).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/profile-options`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox6 at 2026-10-15T17:12Z

---
## Report (done, 2026-10-15T18:21Z)

