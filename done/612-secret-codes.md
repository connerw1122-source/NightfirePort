## Your task: `secret-codes`, the Secret Unlocks codes and what they unlock (no drawing)

Codenames → Edit <name> → Secret Unlocks opens a letter grid (0–9, A–Z, ← → ✓) where the player types a code. The
main agent draws the page; you port **the codes and their effects**.

Handlers (ACTION.ELF, decomp export `work/decomp/ACTION.ELF/`): the page behind `C_SBCNOPTIONS_Handler` 0x213528 row 0
(page 0x40000020), `C_KEYPAD_Handler` 0x216810 and whatever validates the entered string; the cheat flags the
`C_CHCH*_Handler` family reads (0x215bf0..0x216520: music, fly, drones, blind, weapons, health, HUD, unlock, …) and the
progress/unlock fields a valid code sets.

Deliver:
1. `docs/systems/secret-codes.md`: the code table (the strings as the game stores them — they are game data, so cite
   the address; numbers/identifiers in tests only), what each unlocks (missions, MP characters/scenarios/maps, cheat
   flags), where the result is saved, and the feedback the page gives (labels for valid/invalid).
2. In the port: a `FrontEnd` (crates/nf-view/src/menu.rs) code-entry model — the grid cursor, the typed string,
   ← (delete) / → / ✓ (submit) — and `submit()` applying the unlock to the profile/progress the port already has
   (`nf_game::progress`, the mission unlock flags). Gaps go in GAPS.
3. Tests: entering a valid code sets its unlock and an invalid one sets nothing (compare by hash/identifier, not by
   committing the strings in goldens if they are long — short game strings in a test are fine as identifiers), each
   with a sister assertion that can fail.

Not in scope: drawing the grid (the main agent does).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/secret-codes`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox1 at 2026-10-15T18:09Z

---
## Report (done, 2026-10-15T19:19Z)


---
## Report from the worker (done, 2026-10-15T19:51Z)

