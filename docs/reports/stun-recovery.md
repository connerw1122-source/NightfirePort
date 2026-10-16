# stun-recovery — report

Task 713, branch `s/stun-recovery`. Playtest 1015-1738 (07000005, the stunner): "guards get stunned
infinitely and don't complete the animation."

## First check: master already walks the timeline

The base symptom is **already fixed on master** — task 654 (`s/stun-ai-recover`, merged) ported the
taser family and its tests quote this exact playtest. Verified against the decompile, master already
behaves like the game on:

- the **stun state + timer**: TASER 0x4a → STUNNED 0x4b arms +0xCC4 = now + +0xCE0 where +0xCE0 is
  `DroneFunc_RecoverTime` 0x147188's stunner arm (FRAME_RATE_INT·10 → 1200 ticks at the run-time
  stat +0xB9 = 0 [L 07000005]) [M 0x162eb0 case 1];
- the **re-stun rule**: STUNNED case 7 re-falls (0x4a) unless the stored fall anim is 0x32, which
  falls through to case 0x19's RecoverTime re-arm of the NEW hit [M 0x162eb0 case 7 → 0x19]; inside
  TASER itself a re-tase re-arms the fall wait [M 0x162b98 case 7]; the accumulator +0xBC never
  resets, so a charged guard re-falls instantly off the recover state [M 0x1469b8:72 — its only
  writer]; AnyoneNear(2.0 → < 4.0 u) keeps him down with 300-tick re-arms [M 0x162eb0 case 0xc];
- the **recovery**: STUNNED_RECOVER 0x4c plays the get-up 0x76 with the follow-on Attack 0x56 —
  the guard returns to combat [M 0x1630b8 case 1];
- bullets while stunned → Death_Anim 0x44 (he dies, no damage math), punch → the KO 0x42 [M
  0x162b98:79-82 / 0x162eb0:61-69 / 0x1630b8:41-44].

Pinned by 654's tests (`stun_ai_recover_live.rs` — `the_stunned_guard_rides_the_game_timers_and_returns_to_combat`
would fail on the pre-654 stalled walk; `the_player_near_keeps_him_down_and_a_retase_behaves_like_the_game`
pins the kept-down + re-tase rules). Nothing was rewritten for the sake of it.

## What this task found and fixed

1. **The fall wait ran 60× the game's** — the port armed `timer_c = frame + 1800` in DSTATE_Taser's
   ENTER and case 7, reading `FRAME_RATE_DIV·30` as 60·30. The disassembly settles it: 0x162CD4 and
   0x162E5C load the float at **0x30D0D4 = FRAME_RATE_DIV = 60·(1/60) ≈ 1.0** [L 0x30d0d4] and
   multiply 30.0f — the game's fall wait is **30 ticks (0.5 s)** [M 0x162b98:50-56]. Fixed both
   sites to `frame + 30`; the guard now falls for half a second, then twitches for the 1200-tick
   RecoverTime, exactly the game's shape.
2. **The interrupted-anim re-arm** — DSTATE_Taser case 0xc's else arm re-armed 300; the game re-arms
   **now + 5** [M 0x162b98:83-95]. Fixed.
3. **The TASER enter's weapon drop was missing** — the game drops the guard's gun at the fall
   (`DroneWeap_DropWeapon` + `ChangeWeapon(1)` unless civilian or +0xDA == 4) [M 0x162b98:27-34].
   Ported with the flashbang family's existing shape (`drop_weapon` + `weapon = 1`), in the game's
   case-1 order: the drop's draws come BEFORE the Rand(2) fall-anim draw, so the shared stream's
   order is the game's. (The ChangeWeapon slot itself is not modelled — the port's held-weapon byte
   takes the fists id; noted below.)
4. **The re-tase test's aim was a coin flip** — the downed body's shootable volume is the low
   sphere centred 0.3 above the origin [M NDrone2_Collision 0x140e90:65-90]; the test's
   root-height ray grazed that sphere at its exact radius (the measured miss was d² = 0.0949 vs
   r² = 0.09), so the re-tase landed only when unrelated state shifts nudged the geometry. The
   re-tase now aims at the sphere's centre (what a player aims at), and holds the beam a few ticks
   like the game's re-stun.

FAIL-FIRST: the updated `the_stunned_guard_rides_the_game_timers_and_returns_to_combat` fails on
the pre-fix code (the guard still holds weapon 36 — his gun — and the fall completes at rel+≈1830,
not ≤ 40); after the fix all five stun tests are green.

## Verification

- `crates/nf-view/tests/stun_ai_recover_live.rs` (2/2) + `fix_stunner_press_live.rs` (3/3) — the
  one-hit fall → the 0.5 s wait → the 1200-tick twitch → the recover → combat; the player-near
  kept-down re-arms; the re-tase re-fall/re-arm by the stored anim; no damage from the tase; the
  P2K hit is not a tase; the drop.
- Full suite on the final tree (fast runner): **(total below)**.

## Suite total

(filled in when the run completes)

## What's left

- The game's **ChangeWeapon slot system** — Taser enter does ChangeWeapon(1) (fists) and the
  recovery get-up does ChangeWeapon(6) (a sidearm re-draw) [M 0x162b98:27-34, 0x1630b8 case 1]. The
  port models the drop (the gun leaves his hands, the pickup spawns) but not the slot semantics: a
  recovered guard fights with the fists id until something re-arms him. A weapon-slot port is its
  own task.
- The **below-threshold tase re-entry** — HandleImpact case 7's SetState(current) re-flinch below
  2·health is skipped (noted at `on_tase`; the accumulator and the fall timing are unaffected).
- The **StunDart family 0x50-0x52** (weapons 0x43/0x44, msg 0x19) — SP never enters them [M
  DealWithObjHit routes them but the SP levels carry no dart weapon]; MP-only, low priority.
- **Live checks for the main agent** (no GPU here):
  1. PCSX2, 07000005, tase a guard once: the fall pose should last ≈ 0.5 s, then the twitch — the
     old port stretched the fall to 30 s.
  2. The tased guard's gun should be on the ground next to him (the drop), and he should fight with
     a re-drawn sidearm after the get-up (the game's ChangeWeapon(6) — the port shows fists).
  3. A re-tase on the downed guard re-falls/re-arms per the stored anim — matches the port's
     `the_player_near_keeps_him_down_and_a_retase_behaves_like_the_game`.
