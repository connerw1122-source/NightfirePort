# profile-options — report

**Branch** `s/profile-options`. The codename options' data and their effects (no drawing).

## Done

1. **docs/systems/profile-options.md** — the data layer for all five pages:
   - the "Edit \<name\>" wheel (`C_SBCNOPTIONS_Handler` 0x213528): the 7 rows with the page hashes
     they jump to (0x40000020/22/3e/2d/2e/31 + the rename arm) [M 0x213528:28-60];
   - **Advanced Options** (`P_CNOPTIONS_Handler` 0x213f08): all 8 rows — control id, title label,
     value items (0x1B7/0x1B6 On/Off, 0x2A2/0x2A3 Toggle/Hold), the `PlayerSetting+off` byte each
     writes, the default, and the runtime reader. Two [S] guesses in the old model got resolved by
     the page rows + the decomp readers: **+0x0C = Flashing Objects** (`View_SetupRenderModes_
     Tweak` 0x1e5888:24 — flag-0x80000 objects within 4 u flash while set) and **+0x0B = the HUD
     auto-hide gate** ("HUD Always On", inverted items [M 0x213f08:66-71]: byte 0 pins the pane
     timer BL+0x8BC at 1.0, byte 1 decays it 2·0.0020833334·MUL/tick [M Player_Update
     0x193600:414-425]; `HUD_UpdateHealthPane` 0x19b9c8:118-122 scales/pops it). `+0x03`'s old
     `reload_manual` [S] name resolved to **Manual Aim Toggle/Hold** (`Player_Weapon` 0x1a24c8:45).
   - **MP Options** (0x2144d8): Radar (`MPSettings+0x28`), Health Handicap (+0x2c, the items run
     **−75..+100** — the field is signed, the old `health_extra: u32` became `health_handicap:
     i32`), MP Auto Aim (`PlayerSetting+0x02`, the handler's `DAT_002a38ca`).
   - **AV Options** (0x214818): the two sliders (0..20, ×5 on exit-confirm msg 0x50), Subtitles
     (GameState+0x60), Split Screen (the `DrawInfo` 0x2a37c0), Speaker (SFXSetMode values 0/1/4),
     Widescreen (GameState+0x5C), Screen Adjust (psiGet/SetScreenPos), Restore Defaults (70/100/
     per-language subs/DrawInfo 0/mode 1/pos restore [M 0x214818:96-110]), Credits, DAD Trailer.
   - **Codename entry** (0x213790/0x213a98): the validation/save flow summary (the model already
     lives in `fix_save_profile_live.rs`).
2. **The options model + bridge** (`crates/nf-game/src/profile.rs`): the field renames above
   (bit layout unchanged — the game's own chain still decodes [L]), a new `profile::Options`
   runtime snapshot (`Options::of(&Profile)`, `Options::defaults()` = the boot values), carried by
   `Drones::options`.
3. **The runtime effects wired**:
   - auto-aim: `Sim::auto_aim` seeds from the snapshot each tick (the flag used to be a dead
     `false` — the lock path existed but never ran);
   - crosshair: the HUD reticle draw skips when off [M 0x19e6d0:96's gate];
   - crouch toggle vs hold: `LadderCtx::crouch_toggle` (from the options) drives `Bond::physics_
     tick` — toggle flips on the press edge (`Bond::crouch_prev`), hold crouches while held and
     stands on the release;
   - manual aim hold vs toggle: the input path latches free-aim on the press edge when +0x03 is 1
     (`Sim::aim_prev/aim_latch`), else rides the button (the old behaviour);
   - weapon auto-switch: the pickup take pass gates `weapon_switch_request` on +0x0A;
   - HUD Always On: `Drones::hud_fade` (BL+0x8BC) pins/decays per tick, resets at both
     player-damage sites;
   - volumes: `nf_view::sound_tick` copies `options.{music,sfx}_volume` into the `nf_audio`
     driver's `music_master`/`master` every frame.
4. **Tests** (all pass on the NF_ISO slim image, 19:07 UTC):
   - `crates/nf-game/tests/profile_options_live.rs` — 5/5: the full round-trip of every field
     (with sisters: a hand-built bit-layout case, the boot defaults), the options snapshot,
     crouch toggle-vs-hold through `Bond::tick_ctx` (both modes + the hold-release sister), the
     HUD fade pin/decay/gradual sisters through `Drones::tick`, the auto-aim bridge;
   - `crates/nf-view/tests/profile_volumes_live.rs` — 1/1: the sliders reach the driver masters
     through the real `sound_tick` (with the re-sync sister);
   - the pre-existing `profile.rs` unit tests 5/5 (the game's own chain [L InBufData] still
     decodes after the renames), `nf-game --lib` 186/186.

## Shared files touched (additive; merge lesson 4 fields on their own lines)

`drone.rs` (options/hud_fade fields + the tick decay + the take-pass gate + 2 damage-site
resets), `player.rs` (crouch block + LadderCtx/Bond fields), `nf-view scene.rs` (free-aim toggle,
auto-aim seed, crouch plumb, 2 literal fields), `nf-view hud.rs` (the crosshair gate),
`nf-view lib.rs` (the 2-line volume sync in `sound_tick`), `profile.rs` (the renames + Options).

## What's left / notes

- **The full suite on the final tree was not run** — the sandbox's background suite (started at
  17:56 on the pre-edit tree) hit OOM kills from the parallel builds (its CRASH lines are exit -9
  under disk pressure, not test failures). What ran instead: every test target this task touches
  (`profile_options_live`, `profile_volumes_live`, `nf-game --lib`, the profile unit tests) — all
  green. The next session should run `tools/test/suite.py` on the merged tree.
- **GAPS** (hooks left, doc §GAPS): the flashing-objects render tweak, the HUD pane alpha draw,
  split-screen/widescreen/speaker presentation, Secret Unlocks codes (fields only), vibration
  (stored), the MP runtime (radar/handicap/mp-auto-aim), and the MP page's working-copy slot
  ([S], live check listed).
- **Live checks for the main agent** (doc §Live checks): the HUD fade timings, the crouch modes,
  the manual-aim latch, and the MP page's write slot.

## ntfy

(sent after the push; on failure the exact error lands here)

## ntfy failed

All three fallbacks failed with the daily-quota 429 (2026-10-15 19:16 UTC):
1. curl → `{"code":42908,"http":429,"error":"limit reached: daily message quota reached; increase your limits with a paid plan..."}`
2. python urllib → `HTTPError: HTTP Error 429: Too Many Requests`
3. wget → empty body, exit 8 (the same 429 service-side).
The finish step itself queued fine: `OK done 610-profile-options`.
