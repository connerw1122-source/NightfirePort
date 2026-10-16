# script-negative-delay — held scripts (GAPS G-047), the 0700001b solar panels

Task 732, branch `s/script-negative-delay` (from `origin/master` 65bf8a09).

## What's done

1. **A negative delay holds, as the game does.** `Script::update` (crates/nf-game/src/nis.rs) ran a
   negative delay like 0 — the fix-finale-blockers comment claimed that was the game's behaviour; it
   isn't. The disassembly settles it [M Script_Update 0x1da9c4-0x1da9dc]:
   `lw $v0,156($s0)` (SCRIPTINFO+0x9c) → `beql $v0,$zero,0x1da9e0` (delay == 0 → the streams run) →
   `blez $v0,0x1dac68` (delay ≤ 0 → the epilogue — **no store, no decrement**: a negative delay holds
   without counting) → the fall-through stores `$v0-1` (only a positive delay counts down and skips
   the tick). 0x1dac64-0x1dac74 is the function epilogue (SD/`jr $ra`). The port now:
   `if self.delay != 0 { if self.delay > 0 { self.delay -= 1; } return; }`. With p0 = 0, event 3's
   `SCRIPTINFO+0x9c = p0 − 1 = −1` [M Script_EventHandler 0x1d94c0:100] holds the script — the panels'
   0x06000832 stream 11 does exactly that at t0 [D the script's bytes].

2. **`SP_GetHitDamage` + `SP_Update`'s damage path, so 20 damage unpauses the panel.** The game:
   - SP_CreateScriptPlayer 0x1db9a8 copies placement +0x48 (param 7) int→float into SP+0x14
     (`puVar2[5] = (float)(int)*(uint *)(placement + 0x48)`) — the panels carry 20 [D placements
     41-56 param 7]. The port: `Player::hit_points` from `from_placement`'s `g(7)`.
   - SP_Update's tail 0x1dc1c0-0x1dc330 [M]: when the current SCRIPTINFO exists, its delay (+0x9c) is
     non-zero (HELD) and SP+0x14 != 0, `SP_GetHitDamage` is summed into SP+0x18; when
     `SP+0x14 <= SP+0x18`, `SP_UnPause` 0x1dca58 fires — `SP+0x2c (u16) = 1` (the one-tick trigger
     flag, gathered as trigger bit 4 next tick) and `SCRIPTINFO+0x9c = 0` — and the threshold is
     **subtracted** (the accumulator keeps the remainder).
   - SP_GetHitDamage 0x1dc5f8 [M] walks the current script's stream objects (`Script_GetObj`
     0x1db688, kinds 1/2), filters each object's +0xD0 HITDATA chain with mask 0
     (`Collide_FilterBullets` 0x1ece30:24-31 only *zeroes a hit's damage* when the hitter's category
     word ANDs non-zero with the mask — mask 0 drops nothing) and sums every record
     (`Collide_GetDamageNObjects` 0x1ee220 — its "mask" only gates object collection, non-zero →
     collect; all damage is summed). Port analog: the bullet pass (`update_player_bullets`) gathers
     each player-bullet contact's `b.damage` on the held hit-point players' current stream objects —
     segment-vs-sphere, the sphere shape `Control_BuildWorldSph` builds (the glist sphere's centre
     through the entity matrix, the glist's radius; an anim object keeps radius 0 [S]) — into
     `Player.tick_damage`, cleared at the pass's head (the game's Collide_Update clears every hit
     list each frame), consumed by `Player::update`'s damage tail. The port's Scenery-hit stat
     (`PlrStat_LogShotHitScenery(0)` for scripts 0x60007e2..0x60007ea but 0x60007e3 [M
     0x1dc290-0x1dc2c8]) has no port-side PlrStat call here [*deviation*, the stats layer lives in
     progress.rs and this path was not wired].

3. **A held entity takes its first key's position.** `Script_EntityStart` 0x1d8d78 tails with
   `Script_Interp(si, stream)` [M] — the object is placed AT CREATION (the game's held panels never
   stack at an identity matrix). The port's `entity_start` already runs `self.interp(i, …)` right
   after creating the object, and `keys()` clamps to the first key before its time — so no code
   change was needed; the behaviour is now pinned:
   `solar_panels_live::the_16_solar_panels_hold_in_their_girder_frames` asserts all 16
   `OBJECT_SolarPanelWhole` (0x02000A42) entities sit at their SP placement translations after 600
   ticks, and the synthetic `nis::tests::negative_delay_holds_and_the_held_entity_takes_its_first_key`
   asserts a held entity sits at its first key (7, 8, 9), not the identity/origin. **The GAPS entry's
   [S] origin-stacking guess did not reproduce** — the probe that inspired it (one panel from above)
   must have been something else; 16 panels at 16 distinct placements is what the port does.

4. **Every script with Event 3 p0 = 0 re-checked, and the other unpause callers.**
   - **0x06000571 (the 0700004a lift)** — SP #42: mode 1, mask 0x1, out 0x22, hit points 0 [D
     placement]. Its stream 4's event 3 p0 = 0 at t1611 now HOLDS it, and it resumes the game's way:
     SP_Update case 2's rule (mask & 1 && out-channel on → `SP_UnPause`) [M 0x1dbda8 case 2] — the
     port's state-2 block already implements it. The disc carries **no ch-0x22 writer** the scans
     reach (the 11 level scripts' 0xD/0x11 events — the lift script itself writes only ch 0x40 at
     t1610 [D]; the placements; the 13 TouchOnce plates) — the same "no writer found" shape as the
     level's ch 0x4a (docs/levels/0700004a.md §6). The port adds a documented [S] stand-in: ch 0x22
     rises when the hold engages ("the ch-0x22 SP out marks the play", the doc's reading), and the
     mask&1 rule then releases the hold through the unpause path — not a delay override.
     `fix_finale_live::the_0700004a_lift_ride_reaches_the_lobby` stays green (4/4).
   - **0x0600006f** — in every level's lib, creates 0x2000499/0x200049a and holds at t0 [D]. NOTHING
     in the port plays it (no ScriptPlayer references it on 4a; the earlier fix-cutscene-actors
     report already lists "who plays it is unresolved" as an open question) — the hold change cannot
     affect it.
   - **The MP set 0x0600003c..45** — no references in the port's source; the mp suites run in the
     full run (see below).
   - **`Switch_Update` 0x1adea8** [M] unpauses its bound SP on the channel toggle; **`NDrone2_KickObject`
     0x14aa00** unpauses the kick-node's SP — both were already ported (drone.rs's kick servicing,
     the switch objects) and the drone/nav-kick suites cover them.
   - `SP_Activate` 0x1dc338 also clears the delay on a mask&4 use (the port's useobj covers the
     activation path — `sp_activation_gates_and_fires`).

## Verification

- `nis::tests::negative_delay_holds_and_the_held_entity_takes_its_first_key` (synthetic, headless):
  event 3 p0 = 0 → delay −1; ten ticks hold without decrementing and the clock freezes at the load
  beat; the held entity sits at its first key (7, 8, 9); sister: p0 = 5 → delay 4 → counts down to 0
  over four skipped ticks → the streams run again.
- `solar_panels_live` (NF_ISO, 0700001b): (a) 16 panels present and in place after 600 ticks, each SP
  state 2 with delay −1 and hit_points 20.0; (b) the REAL fire path (aim + trigger taps through
  `step_bond`, the gun, the bullets) unpauses only the shot panel — its object dies, its 10 debris
  pieces (streams 0-9) play near its frame, the other 15 stay held and whole.
- `fix_finale_live` 4/4 (the 4a lift ride, the 1b Drake defeat, the 16 chasers, the start states);
  `walkreal_0700004a_live` is pre-parked/`#[ignore]` (fix-final-blockers2's marker question, not
  touched here).
- Full suite (`tools/test/suite.py`, NF_ISO set) on the final tree: see the total line in the
  WORKLOG/day log — 3 `disc_golden` failures allowed on the slim ISO; any other failure is listed in
  the report tail below.

## What's left / open questions

- **The panel render owes the main agent's side-by-side** (GAPS G-047's merge lesson 6): the port's
  sim state is pinned, but no GPU render ran here. Suggested observation points (the panels are the
  translucent gratings in the girder frames below the arrival platform, x 10.14 → 56.12 stepping
  ≈ 6.57, y −1.72 → −9.83, z ±4.62):
  - `--bond 23.3,-1.9,2.0,1.57,0.6` looking down/west: 4 panels in the frames below (placements
    43-46), translucent, intact.
  - `--bond 56.1,-9.9,6.0,0.0,0.3`: the shaft-end panels (placements 48/56) in place.
- **The game's ch-0x22 writer on 0700004a** is still unexplained — the port's stand-in should be
  replaced when a live check finds the real writer.
- **Drone bullets don't feed the SP damage path** [*deviation*]: the game's Collide pass covers every
  bullet; the port gathers contacts in the player-bullet pass only. On 1b the guards' fire could
  chip panels in the game.
- **PlrStat_LogShotHitScenery** is not called from the new damage tail (see above).

## Live checks for the main agent

1. **PCSX2, 0700001b fresh boot**: stand on the arrival platform, look down at the girder frames —
   confirm 16 intact translucent panels (gdbwatch optional). Then shoot one panel until it shatters
   (count the rounds): the port's threshold is 20 damage total; the game's per-bullet damage should
   shatter a panel in the same number of handgun rounds the port needs (the port test's tap count is
   in `solar_panels_live`).
2. **gdbwatch ch 0x22 (address of switch_channels+0x22, byte, writes) on 0700004a from boot through
   the lift ride**: expected — some writer sets it to 1 before the lift script's t1611 (or at it).
   That writer replaces the port's [S] stand-in (`nis.rs`, the state-2 block, "script-negative-delay
   [S stand-in]").

## Usage

not visible to me — this session's environment shows no token, credit, context-window or cost
figures to copy (the agent's tool surface exposes none).
