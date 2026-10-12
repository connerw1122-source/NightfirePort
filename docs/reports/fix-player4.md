# fix-player4 — fists and melee (the knock-out punch)

Branch `s/fix-player4`. Everything below was driven through the real input path (`nf_view::load` +
`Sim::step_bond`), with the game's decompile as the source of truth.

## What the user saw

Punching a guard with the fists did 5.0 damage per hit and then killed him like a bullet — no knock-out, ever.

## The cause (the evidence fix-player3 gathered, finished this session)

- The punch IS a bullet (fists: ammo_type 0, range 1.75, damage 5.0, speed 1.0 [L ee_start_05.bin record 1]), but
  the game routes it differently: when the SHOOTER is the player, `NDrone2_DealWithObjHit` case 5 (0x13fff0:269-306)
  picks the message by weapon — **1 (fists), 0x4B, 0x4D → msg 6 (punch)**; 0x4A/0x4C → msg 7 (taser); everything
  else msg 8 (the normal bullet). The port sent msg 8 for every weapon, so a punch was just a weak bullet.
- The knock-out state is **0x42 `NDrone2_DSTATE_KnockedOut_Anim`** (0x172a30; the state table at 0x29b300 [L]).
  `DroneFunc_HandleImpact` case 6 → `NDrone2_PunchImpact` (0x145f28): the punch applies its damage like any hit
  (a head punch reads as a torso, 0x145558:25-30), and at health ≤ 0 the guard goes to 0x42, not Death_Anim 0x44.
- The per-state (resume, allow) for msg 6 is IDENTICAL to msg 8's in all 46 states that carry a case 6 (scan of the
  250 DSTATE decomps), so the port's existing impact table just works for punches.

## What is ported now

1. **The punch hit path** — `drone.rs`: weapon 1/0x4B/0x4D player bullets send msg 6; `on_punch` reuses the msg-8
   impact table and routes to `punch_impact` (the `NDrone2_PunchImpact` port: resume stamped into +0x118 before the
   damage; alive → PainTalk + the 0x51 stagger with the flinch stamps when the state allows, else straight into the
   resume state — which is what turns a punched idle guard hostile at once, resume 0x56 = Attack).
2. **The KO state machine** — `handle_knockout`: 0x42 ENTER plays the fall (anim 0x50, follow-on Dead 0x47), zeroes
   health, sets the corpse flag and the KO flag (+0x4F8 & 0x1000000), drops the gun immediately (the same
   one-visible-kind-0 drop as death), and logs `PlrStat_LogEnemyDisabled` for enemies; EXIT fires the death channel
   on every level except {0x07000009, 0x0700000A} (the `sltiu` at 0x172b14 is UNSIGNED — checked in the disasm).
   There is no OnInitDeath in the path: no death talk, no grenade drop. 0x43 (`Knocked_Out`) and 0x53
   (`PunchImpact`) are ported too (0x43 is never set by C code).
3. **KO counts as disabled, not killed** — the KO flag makes `OnInitDeath` (0x145bd0:22,57-59) and
   `NDrone2_LogDroneDeath` (0x14ab00) skip `PlrStat_LogEnemyDispatched`. The port now models both counters
   (`disabled_logged` / `dispatched_logged`). The KO'd body is a corpse from the KO on (bullets skip it) and fades
   away like any death — the game's KO'd guards never get back up.
4. **The punch alternation** — `Player_SetFiringAnim` (0x1a3c70): the fists carry flag 0x80000 (+0x68 = 0x880A2
   [L]), so every punch draws `Rand(100)` from the shared RNG and plays the +0xB4 script (0x060006DD) on > 50,
   else +0xB0 (0x060006DE) — the left/right punches. The port's `Gun::firing_anim_script` implements the whole
   pick (last-round 0x1000/base-0x11, the 0x80000 random, the 0x40000 strict toggle via BLData+0x958); `scene.rs`
   calls it (a ~10-line shared-file touch).

## How it was verified

- Repro first (both tests fail on the pre-fix build):
  - `punching_a_guard_knocks_him_out_without_killing` — pre-fix: *"the punch KILLED the guard (state 0x44) — the
    port sent msg 8 instead of msg 6"*. Post-fix: two punches put the guard through 0x53 (the stagger, he was
    aiming) into 0x42; Death_Anim 0x44 is never entered; health is 0 (the KO enter zeroes it, as the game does);
    disabled-logged, NOT dispatched; exactly one visible gun drop (kind 0).
  - `the_punch_fire_script_alternates_left_and_right` — pre-fix: the port always played +0xB0 (mismatches where the
    RNG said +0xB4). Post-fix: 10 punches, every pick predicted from a clone of the shared RNG taken before the
    press matches, and both scripts play (5/5 on the level seed).
  - Sister assertion: a guard SHOT with the P2K goes through Death_Anim and logs dispatched, not disabled — the
    contrast the KO claims rest on.
- Full suite (`NF_ISO=... cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view --
  --test-threads=1`): nf-game 129 passed, nf-view 144 passed, formats/audio green — only the 3 allowed
  `disc_golden` failures on the slim ISO.

## Shared-file touches (per the task rules)

- `crates/nf-view/src/scene.rs` — only the fire-script pick in the view block (the task named gun/player/drone.rs;
  the pick lives in the view layer).
- Nothing else outside the named files.

## What is left (gaps, not regressions)

- The taser route (Q-Claw 0x4A/0x4C → msg 7, the 0x4A state) is not ported — noted in the routing comment.
- The bot-punch mode 0x1E branch (0xF2) and `DroneFunc_SendHurtMessage`'s hurt broadcast are not modelled (matching
  the port's existing bullet path); the level-7/9-A mission-fail reasons in the punch are not modelled.
- `NDrone2_DSTATE_Knocked_Out` (0x43) is reachable only from the state table / scripts; nothing in SP sets it.
- The punch's per-state stagger anim 0x51 plays only for the allow=1 states (aim states, per the game's table) — if
  the user expects every punched guard to stagger visibly, that is the game's own behaviour, not a port gap.
