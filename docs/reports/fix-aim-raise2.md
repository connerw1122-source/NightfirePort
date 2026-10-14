# fix-aim-raise2 — the aim gate rebuilt on master's weapon-anims raise/lower

Branch `s/fix-aim-raise2` (from `origin/s/fix-aim-raise`, `origin/master` merged twice: 8d096810 at
19:59 UTC, then 4041d239 at 20:42 UTC over fire-modes3b / casings-16b / handcannon / 0c-vent2 /
character-room-light). The merge the 104f hold warned about is resolved: **one queue point, one
advance**, and the finish of master's raise sets the aim bit.

## What the game does (the evidence chain)

- `Player_Weapon` 0x1a24c8:33-64 [M]: the hold-aim input clears the bit (`+0x60 & 1`) and re-sets
  it while the button is held — a weapon WITHOUT a raise script (+0xB8) aims at once.
- 0x1a24c8:66-99 [M]: the raise/lower arms are unreachable in the refused view states
  ({1,2,3,4,0xa,0xd,0xe,0xf} unscoped; +{5,6,7} when scoped, `+0x68 & 0x40`) — state 0xF (the
  raise playing) can never re-fire its own queue arm, so the queue is edge-only.
- 0x1a24c8:104-152 [M]: when the input flips the bit and the record has +0xB8: the PRESS re-clears
  the input-set bit and queues the raise +0xB8 at 1.25 (view state 0xF); the RELEASE queues the
  lower +0xBC at 1.25 (state 0x10) **from view state 0 only**; with no +0xBC the raise plays
  REVERSED (`AnimScriptAddReverse`, disasm 0x1a26f0).
- `Player_SetWeaponAnimObj` case 0xf 0x1a2e38:554-561 [M]: when the raise stops, the aim bit sets
  (and +0x959 = 1); case 0x10 :562-567: the lower's stop leaves the bit clear.
- 0x1a24c8:168-187 [M]: the zoom target +0x8D4 is written ON THE BIT'S RISE (the max-zoom write
  gated on `+0x60 & 1`); the reset to 1.0 on the bit's fall — both read the bit, not the button.

## The rebuild

Files: `crates/nf-view/src/scene.rs` (the four conflict sites + the wiring), and one test-input
correction in `crates/nf-view/tests/fire_modes3_live.rs`. `crates/nf-formats/src/weapon.rs` merged
clean (the +0xB8/+0xBC accessors both sides added were identical).

1. **The Sim fields** (conflict 1): master's `view_alert` / `prev_aiming` / `view_aim_anim` kept,
   the branch's `view_aim_raising` appended on its own line (the finish needs raise-vs-lower:
   case 0xf sets the bit, case 0x10 does not).
2. **One queue point**: the view chain's edge arm (fix-weapon-anims') kept — driven by
   `aim_on_edge`/`aim_off_edge` from run's aiming param, which is now the RAW button; the branch's
   duplicate queue inside `aim_gate` deleted.
3. **One advance + finish**: `aim_gate` (hoisted to the top of the tick, before the cursor/zoom/
   auto-aim consumers) advances the raise/lower and finishes it; the raise's finish sets the aim
   bit, the lower's leaves it clear; the shared advance skips while the aim anim plays
   (`switch_state == 0 && view_aim_anim.is_none()` — conflict 3). Master's duplicate in-chain
   finish block deleted.
4. **The wiring**: `step`/`step_bond` pass the RAW button to `run` (the gated bit's rise at the
   raise's finish would otherwise re-fire the aim-on edge and queue the raise twice); the
   auto-aim scan/ease gate and `bond_aiming` read `self.aiming` (the bit); run's tail no longer
   clobbers the bit with the raw button.
5. **The +0x93a counter** (conflict 2): master's form (`!fire_state` — the 103k2 every-tick
   reset), still gated on `view_aim_anim.is_none()` so the raise counts as non-idle.

## Real bugs found on the way

- **The reverse lower started at the wrong length** (master's arm carried it): the start frame was
  `(len - 1)` where `len` is the script playing AT THE QUEUE MOMENT (the idle's), so the sniper's
  AnimScriptAddReverse lower finished instantly. Now starts at the raise's own last frame. Caught
  by `fix_aim_raise_live::aim_release_drops_the_bit_then_plays_the_reverse_lower` (failed first:
  "lower tick 1: the lower left early", idle 0x0600_04AB vs the raise 0x0600_07b9).
- **fire_modes3_live zoomed without aiming** (new master test): the game writes the zoom target
  only on the bit's rise [M 0x1a24c8:168-187]; a raise-script sniper's bit waits for the raise.
  The test's ramp now holds aim+zoom (the game's pad scheme); every assertion unchanged — it
  failed first ("the scope zoomed in (zoom 1)").

## Verified (NF_ISO, real input path via `nf_view::load` + `Sim::step_bond`)

On the final tree 4041d239: fix_aim_raise_live 5/5, weapon_anims_live 7/7 (master's raise incl.
the reverse lower + the fidget), fix_guard_misc_live 6/6 (the 0c sniper), fire_modes3_live 5/5,
fix_weapon_sounds_live 3/3, fix_weapon_fire_modes_live 6/6, casings16_live 3/3,
fix_handcannon_live 2/2, fix_0c_vent2_live 3/3, plus the suite chunks below.

## The suite (fast runner `tools/test/suite.py`, foreground chunks — this sandbox kills
background jobs between tool calls, the 102c/100f/103k2/104e precedent)

- On the first merge 8d096810, chunks A–J (lib tests + AI/world/weapon/gadget/player/camera/
  portal/models/render/audio/probes/disc_golden): **894 passed, 6 failed, 23 ignored**.
  The 6 fails: the 3 allowed `disc_golden` (slim ISO) + 3 GPU-adapter environmental fails
  (`fix_0c_pylon_live` ×2, `probe_pylon_render` — `adapter: NotFound` at device creation, the
  same 3 fix-aim-raise reported).
- On the final tree 4041d239, the master-touched surface re-ran: **312 passed, 0 failed**
  (nf_game lib + grenade + doors ×3 + ai + trig + oddjob: 196; the 9 weapons/modes/aim binaries
  above: 40; the final-tree extras — level_end, menus_flow, devwin_flow, mp_arenas,
  fix_guest_reactions, fix_mission, fix_party_mission and the walk_07000005/0a/0c walks: 76).
- Excluded for the window: the rest of the walk_*/walkreal_* family (the 103l2 walkreal-01 walk
  is a documented pre-existing merged-tree fail; walkreal 0a/0c/0d + walk 05/0a/0c ran green
  here), weapon_matrix_live (>600 s, documented), levels_sweep/portal_sweep. The next session
  can sweep them.

## Live checks for the main agent

PCSX2 (07000005, give the Winter Covert Sniper 0x24):
1. Hold aim (L1): the scope takes ~19 ticks (0.32 s) to raise — the zoom/FOV kick begins only as
   the raise completes, not at the press. gdbwatch: `BLData+0xF4` = 0xF while it plays;
   `(obj+0xdc)+0x60 & 1` sets exactly on the raise's last tick; `+0x93a` stays 0 during the raise.
2. Release aim: the bit and the zoom-out begin AT ONCE, the gun lowers over ~19 ticks (the raise
   script reversed — the record has no +0xBC); `+0xF4` = 0x10 while it plays.
3. The P2K 0x02 (no +0xB8): the bit sets on the press tick, no raise.
4. Press aim during a reload: no raise queues, the bit stays clear (the port is conservative —
   the game's arm can fire from some non-idle states [S]).

nf-view `--bond` spots: `--bond 105.9,25.3,108.1,0,0` (07000005 courtyard) with aim held — the
arms/scope raise, then the crosshair + FOV change land together on the raise's last tick.

## What's left

- The walk_*/walkreal_* suite family + weapon_matrix/levels_sweep on this tree (window).
- Press-during-the-lower does not displace the lower with a raise (the game's case D has no state
  check [M 0x1a24c8:145-152]; the port's queue arm waits for the idle view — conservative,
  matches fix-aim-raise's documented deviation).
- A weapon switch during the raise is allowed (the game refuses it at state 0xF
  [M 0x1a6f80:33-47]); the raise is displaced and the bit stays clear.
- The raise's stop sets +0x959 = 1 in the game [M 0x1a2e38:561] — unmodeled (master parity).
- The toggle-aim option (DAT_002a38cb per-slot) is unmodeled (hold only), pre-existing.
