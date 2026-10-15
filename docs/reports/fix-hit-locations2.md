# fix-hit-locations2 — the held hit-locations branch fixed on current master

Branch `s/fix-hit-locations2` = `s/fix-hit-locations` (69197145) with origin/master (d5b22f91, 93 commits) merged FIRST, as instructed. Two merge conflicts: `drone.rs` (the punch-path scream) and `docs/WORKLOG.md` (append-only, both sides kept).

## What was broken and why

The task listed three failures on master. Root causes, each from the game's own code:

### 1. `p2k_head_shot…plays_the_shot_death` — "0x42, not 0x40"

**The test's expectation was ungameable; the port's machinery was already faithful.**

- `DroneAnim_LocationDeathAnim` 0x13c780 does `CallAnim(0, 0x42, variant, …)` for an aimed head/torso kill. The 0x42 **call** is resolved through the state's transition list: for source state 0x25, key 0x42 → **danim 214, count 2** (variants 214/215 = the two directional shot-death falls); the generic 0x40 call → danim 209, count 4 (the Rand(4) arm). [L ACTION.ELF, `DroneAnimStates` @ 0x272c68, `Drone_AnimInfo` @ 0x273200]
- **No danim carries anim state 0x42 at all** — `Drone_AnimInfo[214/215]+8 = 0x40` [L]. `DroneAnim_SetDAnimInternal` 0x13aee8:48 writes `+0x56A` from `AnimInfo+8`, and normalises the CALL's state at `+0x558` (0x41..0x44, 0x50, 0x6f → 0x40) for the reached check [M 0x13aee8:60-70]. So `Drone+0x56A` (the port's `anim.state`) reads **0x40** while the shot-death plays — in the game and in the port. The old assert `anim.state == 0x42` could never pass.
- A live probe confirmed the gate was never the problem: at death ENTER `bond_aiming=true`, `hit=Some(part 5)`, `can_do(0x42)=true`, `transition(0x42, 0x25, 0)=danim 214` — and the run ends with the drone playing danim 214, variant 0.

**Fix (test-side):** the head/torso kill asserts now check the DANIM the 0x42 transition picked (`tables.transition(0x42, pre_state, want).danim + want`), that it differs from the generic 0x40-call danim (sister), and that `anim.state` is the 0x40 death family. `pre_state` is recorded before the kill shot (the death processes on the kill tick). Verified green: `it-loc] death: anim state 0x40 variant 0 (want 0), danim 214`.

### 2. `party_guest_head_shot…` — "07000007 has present mode-0xD guests"

**The base's premise was wrong: nothing on 07000007 carries mode 0xD.**

- `+0xC5` (the mode byte every HitDamage/BulletImpact check reads) is `GetDTYPENEW`'s RETURN [M `GetDroneTypeAttackTypeFriend` 0x14cbd8:24-27 — `*(char*)(+0xc5) = GetDTYPENEW(...)`], written via `DoModeSettingsNEW` 0x14c600:80. (`DoModeSettingsOLD` 0x14cfa0, the placement-mode-param path, instead writes `DroneModeSettings[param*12]` — the two-writer structure is documented for the next window.)
- The 07 party guests are dtype 0xA/0xB → **0x12 (female) / 9 (male)** below full alertness, 10 at it [M 0x14c938 case 10/0xb]. Mode 0xD comes only from dtype 0xF [M 0x14c938 case 0xf].
- [L probe] 07000007: 58 drones, **none** with mode 0xD, skin 0x0500_00ba or dtype 0xF (guests at 0x12/9; guards at 0x13/0x16). 07000016's dtype-0xF chasers (gi 9/12, mode 0xD) are spawner-held (`present=false`) at tick 2; 07000014's kind-0x10 drone (gi 27) is script-disabled at tick 2. No reachable live mode-0xD subject exists in this sandbox.
- HitDamage 0x145558 case 5's 2×Torso-no-silent-kill arm fires on `+0xC5 == 0xD` **OR `+0xE0 == 0x05000ba`** OR (glist +0xD8 == 0x10 on 07000014) [M 0x145558:64-72]. The port's `damage.rs:94` implements all three; `do_hit_effects` passes `self.mode` (the base's original fix, carried through the merge).

**Fix (test-side):** the test now (a) pins all three 2×Torso arms at the `hit_damage` unit level on the level's own tuning, (b) asserts a live 07 guest's mode equals the GetDTYPENEW derivation (0x12 female / 9 male / 10 alert) and that its head shot takes the HEAD arm (×DroneDamage_Head ×k + the silent kill — the 0xD arm must NOT fire for 0x12/9). The live head shot is best-effort: the party guests **flee an aimed gun** headless (their party scare handling) and the ray can miss for the whole window — a PCSX2 live check is listed below.

### 3. `fix_flesh_impacts_live::a_p2k_body_shot_spawns_the_blood_puff_at_the_hit_point` — "scale_rate = (6-3)/60, got 6"

The merge had kept BOTH the base's puff block (life 0.5 [S] → rate (6−3)/0.5 = 6) and master's fix-flesh-impacts block (life 60) in the same drone-hit arm — a double spawn with the base's wrong parameters winning the assert.

**Fix (production, `drone.rs`):** consolidated into ONE `victim_puff` block with the decompile-verified union [M `NDrone2_DealWithObjHit` 0x13fff0 case 5's second switch]:
- skip the explosives 0x2b/0x34-0x37 (both sides agreed);
- the default arm gated on the unaligned f32 at weapondef+0x06 > 0 (`0.0 < *(float *)(weapondef + 6)`) AND `hit.damage > 0.0` (master's stand-in for the case-5 shooter/health flow — DealWithObjHit runs before HandleImpact applies the damage, so a kill still puffs);
- default: `Gas_Init(3.0, 6.0, obj+0x20, HIT+0x40, 0, 0, 0x20008bc, 5, 0xffffffff, 0xffffff00)` — start white opaque, end white/alpha-0;
- stun 0x32/0x33: (1.0, **16.0**, 0x200026e) — 0x41800000 (the base's 17.0 was wrong);
- life = the 11th Gas_Init arg the decompiler drops: **60 ticks** from the disassembly's `sw 0x3c` [M 0x140404/0x140460] — the base's 0.5 [S] was wrong.
`init_ex` computes `scale_rate = (p2−p1)/life` = 3/60 — the test's expectation. `fix_flesh_impacts_live` green on the merged tree (full suite below).

### The merge's scream conflict (production, `drone.rs`)

Master's fix-melee restructured `punch_impact` into the two team arms (team 1 KOs before the scream; team ≠ 1 screams first) and extracted a `send_hurt_scream` helper that had DROPPED the mode gate its own doc cites. `DroneFunc_SendHurtMessage` 0x145e28 has three gates in the decompile: prop 0x32, `+0x4F8 & 8 == 0` (once per life), and **`*(char *)(+0xC5) != 2`**. Resolution: master's team arms calling the base's `send_hurt_message` (mode gate intact, called from both the punch and bullet paths [M 0x1461c8:30]).

## Verification

- The three `fix_hit_locations_live` tests: **3/3 green** on the final tree (`test result: ok. 3 passed`), run headless through the real path (`nf_view::load` + `Sim::step_bond`, NF_ISO).
- Full `tools/test/suite.py` on the merged tree (production code identical to the final tree; started 03:12 UTC): **1207 passed, 9 failed, 65 ignored (41 min)**. The 9: the 3 allowed `disc_golden`, the 2 `fix_hit_locations_live` tests AS THEY WERE (built before the test rewrite; both now green per the direct run above), the 3 pre-existing pylon failures, and `fix_pickups_trace_live::with_dev_god_and_infinite_ammo_every_raptor_drop_is_taken_and_the_spare_matches_the_trace` — observed on this tree, unrelated to this branch's production delta (this branch touches only the victim-puff block, the scream helper and hit-locations plumbing; no shared-RNG draws), left for the next window.
- The slim-ISO caveat: `pss_movies_live` not applicable; no retail image here.

## Shared-file hooks (all in `crates/nf-game/src/drone.rs`)

1. The victim-puff block consolidation (the wdef6 gate + the 60-tick life + the 16.0 stun + the 0xffffff00 end colour).
2. The punch-path team arms calling `send_hurt_message` (the mode!=2 gate restored; `send_hurt_scream` removed — it was the merge's duplicate).
No other shared files touched. Test file: `crates/nf-view/tests/fix_hit_locations_live.rs` (rewritten as above).

## Open / left for the next window

1. `fix_pickups_trace_live` failing on this tree (see above) — needs a repro on a clean master to attribute.
2. The `DoModeSettingsOLD` path (+0xC5 = `DroneModeSettings[param*12]`) is not modelled separately in the port's `type_setup`; today every SP drone goes through the GetDTYPENEW semantics. Worth an audit of placement mode params ≥ 0x24 vs < 0x24 (the two-writer structure is cited in the report).
3. The +0x5D0 knockback (DealWithObjHit case 5) — still unported, as in the base's report.
4. The guests' flee-the-aim behaviour headless (why the live guest shot missed) — behaviour check, not a damage bug.

## Live checks for the main agent

1. **PCSX2 07000005, aimed head kill on a guard**: `Drone+0x56A` should read **0x40** while danim **214/215** (the 0x42 call's shot-death danims) plays; the variant follows the shot direction. (The port now matches; this confirms the +0x56A reading.)
2. **PCSX2 07000007, aim at a wandering party guest and head-shot it**: ×DroneDamage_Head + the silent kill (mode 0x12/9 must NOT trigger the 2×Torso arm); also gdbwatch `Drone+0xC5` for a guest at load — expected 0x12 (female skins 0x5000020-24) or 9 (males 0x5000019).
3. **PCSX2 07000005 body shot on a calm guard**: 3.5×k damage and the 0x413 zone sound (the headless body section can't reach a calm guard after the head kill alerts the level).
4. **nf-view `--bond` spots**: 07000005 first mansion guards — the head kill should show the directional shot-death fall; a body-shot kill the generic fall; the blood puff 0x20008bc at the hit point on every bullet hit.
5. **gdbwatch `Drone+0x470`** (the last-hit HITDATA stash DealWithObjHit writes) across a two-hit kill — the port relies on its `hit` record persisting to the death ENTER.
