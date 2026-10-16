## Your task: `script-negative-delay` — held scripts (GAPS G-047): the 0700001b solar panels

On 0700001b the PS2 shows 16 translucent solar panels in the girder frames below Bond. In the port all 16 shatter at
t=5 and the frames are empty (side by side against PCSX2, 2026-10-16). The main agent's investigation (GAPS G-047,
docs/WORKLOG) found the cause:
- 16 `ScriptPlayer_SolarPanel` placements (41-56, type 0xD9, mode 1, hit points 20 in param 7) play script
  0x06000832.
- Its stream 11, Event 3 with p0=0 at t0, sets `SCRIPTINFO+0x9C` (the delay) to −1.
- A NEGATIVE delay HOLDS the script [M `Script_Update`: 0x1da9c4 loads +0x9c, `beql` on zero runs, `blez` at
  0x1da9d0 returns; only a positive delay counts down] until `SP_UnPause` 0x1dca58 clears it. For the panels that
  happens when the panel is shot: `SP_Update`'s damage path 0x1dc1c0-0x1dc330 accumulates `SP_GetHitDamage` 0x1dc5f8
  until it reaches 20.
- The port's `Script::update` (crates/nf-game/src/nis.rs) runs a negative delay as 0. The fix-finale-blockers commit
  added that, and its comment calls it the game's behaviour. It isn't.

Do all of:
1. A negative delay holds, as the game does.
2. Port `SP_GetHitDamage` and `SP_Update`'s damage path, so 20 damage unpauses the panel and its stream 1-10 debris
   plays.
3. A held entity takes its first key's position. An experiment showed held entities stacked at the origin, because
   `entity_start` creates them with the identity matrix and only `interp` positions them. Check how the game places
   a held script's objects [M].
4. Re-check EVERY script with Event 3 p0=0, which now holds where it used to run: 0x0600006f (on every level), the MP
   set 0x0600003c..45, and the 0700004a lift 0x06000571 (mask 1, out 0x22, resumes only when channel 0x22 is on).
   Also check the other unpause callers (`Switch_Update`, `NDrone2_KickObject`). Each must resume the game's way.
   fix_finale_live and the 4a lift test must stay green.

Tests:
- 0700001b: all 16 panels present and in place after 600 ticks.
- Shooting one with enough damage shatters only that one.
- The held-script rule on a synthetic script.
- Sister: a positive delay still counts down.

Run the FULL suite and list every failure. The main agent will compare the panels against PCSX2.

**Tests pin only proven behaviour.** Every assertion about what the game does cites its evidence: [M] a decompile
address that actually says it, [D] disc bytes, or [L] a live PCSX2 measurement from a FRESH boot (a save-state resume
skips the boot path). A guess, a port choice or an [S] item is never a hard assertion: mark it `// [S] unverified` and
keep it out of pass/fail. If an existing test contradicts the decompile, the TEST is wrong: fix it, with the address, and
say so in the report. Never weaken a decompile-backed test to make code pass.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/script-negative-delay`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox3 at 2026-10-16T21:37Z
