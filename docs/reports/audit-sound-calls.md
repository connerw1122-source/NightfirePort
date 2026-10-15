# audit-sound-calls — report

**Branch** `s/audit-sound-calls`. The task: enumerate every ACTION.ELF sound call vs the port,
write `docs/systems/sound-coverage.md`, port the 5 most-noticeable MISSING on-foot sounds with
NF_ISO tests.

## Done

1. **The audit** (docs/systems/sound-coverage.md): 195 direct call sites (`jal`-scan of the ELF's
   .text via a capstone script + the decomp's call lines; the decompiler scrambles the float
   prototypes so ids were confirmed against the disassembly's `$a0` and the decomp's id slot —
   e.g. Break_Kill's two sites are the 0x5F3 fall-through and the 0x5F2 taken-branch delay slot,
   [M dis 0x12b9d4-0x12ba50]). Every site classified PORTED / MISSING / N/A with the trigger and
   the port cite; summary counts per class. Headline: the player/bullet/world-object layers were
   already heavily ported by earlier fixes; the whole-class gaps are the security camera
   (Sensor 0x29), SpaceLaser (0x50), SS playback (0x25), Break (0x20), the sky thunder wiring
   (G-019), the picklock minigame beeps, and the front-end UI sounds (N/A).
2. **Five ports, four hooks, each cited and tested** (`crates/nf-game/tests/sound_coverage_live.rs`,
   all 4 pass on the NF_ISO image; sisters included):
   - **0x10E** the grenade-ammo launch whoosh [M Bullet_init 0x12c5a0:285-291] —
     `Drones::fire_player` plays it when the weapon's ammo byte (+0x90) is 0x0E [L ammo-data.md
     rec 14]. Test: the frag-weapon fire lands 0x10E in `drones.audio`; the P2K does not.
   - **0xA3 / 0x5FB** the jump launch + head bump [M Player_HandleJump 0x1916b0:184,195-198] —
     `Bond::sfx_out` outbox drained by the Sim into `drones.audio`
     (`player.rs` + `nf-view/src/scene.rs:2322`). Test: the jump tick queues 0xA3; idle does not.
     The 0x5FB gate (`level_tag+0x50` ∈ {0xD,0xE}) is a byte the loader does not parse — plumbed
     as `LadderCtx::level_bump_5fb` (default the 0xA3 arm) with a live check in the doc.
   - **0x249 + 0x5DD** the laser burn-through [M Lock_Update 0x1ae3d8:90-93; Bullet_DoTrails
     0x12d890:473-478] — the watch-bullet pass plays both when `Lock::burn` completes. Test: the
     0x4F bullet through the lock sphere → both ids in the pool; a re-burn adds none.
   - **0x148** the scripted special-death cry [M NDrone2_DSTATE_SpecialDeath_Anim 0x163c68 case 5]
     — queued from the SPECIAL_DEATH ENTER arm when `special_death == 0x0600021F`, drained by
     `Drones::tick`. Test: the ENTER queues and the pool holds 0x148; another script id queues
     nothing.
3. **WORKLOG** entry `audit-sound-calls`.

## Shared-file hooks (small, additive — merge lesson 4)

- `drone.rs`: `fire_player` +3 lines; the watch-burn block +9; the SPECIAL_DEATH ENTER arm +5;
  the `sfx_reqs: Vec<(u32, V3)>` field (own line) + both constructor literals + the tick drain.
- `player.rs`: `Bond::sfx_out` field (own line) + constructor; `LadderCtx::level_bump_5fb`
  (own line + default); the two pushes in the jump/bump branches; the three test-only
  `LadderCtx` literals got the default field.
- `nf-view/src/scene.rs`: the 6-line drain after `bond.tick_ctx`; `level_bump_5fb: false` in the
  one full `LadderCtx` literal.

## Verification

- `cargo test --offline --release -p nf-game --test sound_coverage_live` → **4 passed** on the
  NF_ISO slim image (2026-10-15 18:09 UTC).
- `cargo check --offline -p nf-game -p nf-view` clean.
- The full fast suite (`tools/test/suite.py`) was started at 16:20 UTC on the pre-edit tree and
  killed at ~17:55 when the sandbox's disk filled (9.9 G, 100%); its partial read showed only the
  known environmental fails (the 3 slim-ISO `disc_golden` + the no-GPU pylon set). **The final
  tree's full-suite run is NOT done** — the next session should run it (the in-repo stray
  `target/` dir was deleted to free 1.9 G; use `CARGO_TARGET_DIR` outside the repo).

## What's left

- The full suite on the final tree (above).
- The doc's "Live checks for the main agent" list: the head-bump `level_tag+0x50` byte per
  level, the thunder wiring point, and an objective-jingle A/B against PCSX2.
- The audit's MISSING-but-unhooked classes (camera beeps, SpaceLaser, SS playback, Break smash,
  picklock beeps, RB shot-thud, SSys_Msg alarms) are whole-object/class ports — each needs its
  class first, per the table.

## ntfy

(see the finish step output; on failure the exact error lands here)

## ntfy failed

All three fallbacks failed with the daily-quota 429 (2026-10-15 18:16 UTC):
1. curl → `{"code":42908,"http":429,"error":"limit reached: daily message quota reached; increase your limits with a paid plan..."}`
2. python urllib → `HTTPError: HTTP Error 429: Too Many Requests`
3. wget → empty body, same 429 service-side (the finish.sh `OK done 111f-audit-sound-calls` line was still queued).
