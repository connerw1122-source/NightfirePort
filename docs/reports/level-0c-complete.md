# level-0c-complete — 0700000c (the power station, part 1) walked start → finish

Branch `s/level-0c-complete` (from origin/master 92db2533, merged batch 1018b at c4e47e0d).

## The objectives and how each completes

Record 11 (part 0 of mission 0x0700000c) has exactly **3 objectives, none optional** (flags 0 in the
record [L ee_start_05.bin @0x2a4080]; `crates/nf-game/src/mission/c0700000c.rs`):

| # | Label | Complete ch | How it completes |
|---|---|---|---|
| 1 | 0x400002c "Investigate building ST-1" | 0x1b | the TOUCH plates #506/#274 on the lower walkway (out ch 0x1b [D]); the walkreal's leg 1b walks Bond onto the pad and the plate fires [M Trigger_Update case 2's OR-write]. The monitor completes it the tick after [M 0x1827d8:88-99]. |
| 2 | 0x400002d "Photograph the Jetpack Prototype" | 0x1c (reveal 0x1b) | the camera gadget (0x54, in the loadout [M 0x1a7ef8 case 0x700000c]) fired at the `CamSubject` display (#414/#454, param 0 = 0x1c [D]) — the bullet enters the subject volume and writes the channel [M CamSubject_Update 0x1bc668:14-24]. The shot goes through the real trigger path. |
| 3 | 0x400002e "Advance through Warehouse TR-2" | 0x23 (reveal 0x1c) | completes on 0700000d (its arrival start trigger [D 0700000d]) — on 0c it is REVEALED only; the walk asserts states[11][2] == 1-equivalent reveal state via the monitor's states (the walk pins [11][0]=4, [11][1]=4, the exit fires). |

The mission cannot END on 0c (the next level's channels are still off [L record 12]) — the exit is
the LOAD_LEVEL hand-off, which the walk performs through the real E press at the gate.

## The route (what runs for real in `walkreal_0700000c_live`)

1. The ST-1 walkway start #1 [D] → the raised corner — walked.
2. The display stand-in (the vent/pulley 0xE2 gap — see the blockers) onto the plate pad → the ST-1
   plate → ch 0x1b → objective 1 — the touch is real.
3. The display stand-in (the same gap) → the camera photograph through the real fire path → ch 0x1c
   → objective 2 — the shot is real.
4. **The secret door — REAL this task** (was a stand-in): Bond stands on the co-located plates
   (52.24..53.29, 0.90..3.20, −174.45..−172.73 [D #558/#559]); #559 (mask 8) holds the body-touch
   ch 2; **#558 (mask 4) is the E-interactable arm** — `Trigger_Activate` 0x1b2318:23-33's type-2
   `mask & 4` gate (docs/atlas/trigger.md; mask bit 0x4 is the use-key flag, not a contact class —
   `Trigger_Hit` 0x1b1fc8's contact bits are only 0x2/0x8/0x10). The level doc's "a guard's patrol
   is part of the key" was wrong — corrected below. The E press → ch 3 → the AND #560 [D: out 8,
   inputs 2, 3] → ch 8 → `Door_secret1` #544 (ch 8, 2 keys, rate 400, rest 0.75, wrap 1 [D]) sweeps
   to the rest point (state 0x40 [M 0x1a9948:109-115]), the second press resumes it to the wrap
   whose pose holds the LAST key — full open, spent (wrap 1 → state 0 [M 0x1a9948:119-128]); the
   posed slab's collision follows (`set_dynamic`, fix-party-mission item 7). Bond walks east past
   the slab plane for real (x > 53.5 asserted).
5. The yard stand-in (the doorway ledge → the yard floor descent defeats the straight walker) →
   the yard walk west to the ladder base — walked.
6. **The yard ladder — KNOWN OPEN ITEM**: the grab at the base never engages (900 ticks pushing
   forward into the ladder at (9.7, −187.3), the substate never leaves SS_WALK). The walk tries the
   climb first and takes the documented stand-in onto the circle when the grab does not engage.
7. **The circle walk — KNOWN OPEN ITEM**: the nav-routed walk loses the planks on the south-west
   bend (Bond fell to the yard at (3.4, 1.1, −211.1) in this walk). Documented stand-in onto the
   circle's south-east, then the walked approach.
8. The pulley stand-in (the band gap x 14-38 has no collision floor [D probes]; the winch cars are
   ST-1-interior SSs — #351/#364/#377 at z −115..−146 [D probe], not band vehicles) onto the band
   east piece → the band walk to the gate — walked.
9. The gate #556 (LOAD_LEVEL, next 0x0700000d, key gate ch 0x1c [D]) — the real E press →
   `level_exit 0x0700000d` [M 0x1b2318:111]. **Walked out in 2917 ticks; stuck 0; falls 0.**

## The fixes (each with its game address and its failing-on-old-code test)

1. **The use key's trigger half was wiped before the muxes read it.** The port ran the E
   activation in the scene's use block, BEFORE the sim tick's `Trigger_Calc` — whose first pass
   clears every Touch trigger's out channel [M Trigger_Calc 0x1b1900's type-2 zero pass] — so the
   E write died before the AND's update ever saw it. The game runs the E press at Bond's turn in
   the object walk [M Player_Activate 0x1a5170 called from the walk], after Trigger_Calc and
   before the later muxes in the SAME frame; the next frame's clear only follows the consumption.
   Fix: the trigger half of `player_use_doors` defers through `use_trigger_request` and consumes
   right after `Trigger_Calc` (`drone.rs`). Failing-on-old-code test: the walkreal's secret-door
   leg (ch 8 never rose before); sisters inside it (ch 3/ch 8 stay off on the body touch alone).
2. **The `SS_Operate` presence half** (`ss.rs` + the contacts in `drone.rs`, `world.rs`'s
   `dynamic_boxes`): the obj+0xD0 contact classes (3 player → 0x8, 2 drone → 0x10, 5 bullet → 0x2)
   AND'd with the SS's mask word [M 0x1a9df8:12-45], the port feeding the player's capsule box
   (bit 0) and the player-bullet flight segments (bit 1, dead bullets included [M the collision
   pass's product]); plus the channel latch write [M SS_Update 0x1a9948:18-23: mask bit 0 clear,
   channel non-zero, value 0 → the operate result]. Focused test
   `fix_ss_operate_0c_live`: the disc's mask-0x2 winch (#351, ch 0 — never channel-driven
   [M 0x1a9df8:46-51's short-circuit]) sweeps on a real sniper shot from the walkway; the sister
   pins 300 idle ticks never moving it. (Drone contacts stay unported — no disc SS carries a
   0x10 mask [D census, fix-0c-hatch-latches].)

## What the walk still cannot do (the exact reasons)

- **The vent/pulley route into ST-1's interior** (the real descent to the display ground): the
  hatch chain is ported (fix-level-0c/fix-0c-hatch), but the ascent leg (the walkway 14.01 → the
  duct top 17.2-17.5) has no ported route — the game's way in is unknown (the live checks filed in
  docs/reports/fix-0c-hatch.md still stand). The winch SSs now sweep on bullets (this task), but
  they are ST-1-interior cars [D probe], and the band gap crossing has no collision floor [D
  probes]. The walk keeps the documented reach stand-ins.
- **The yard ladder grab** (the known open item): Bond at the base, pushing forward into the
  ladder, never enters SS_CLIMB. `Player_CollLadder` 0x196118's pick needs live tracing —
  Live check below. The walk tries the climb first, then takes the stand-in.
- **The circle's south-west bend**: the nav-routed walk falls off the planks. Edge handling in the
  walk route (or the nav path's bounds) — the stand-in hops to the circle's south-east.
- **The doorway ledge → the yard floor descent**: the straight walker caps out against geometry
  0.7 units ahead of (55.0, 2.05, −175.3); the route needs waypointing (a ported-route gap, not
  the door).

## Live checks for the main agent

1. **The yard ladder** (the known open item): PCSX2 0700000c, Bond at the yard ladder base
   (9.7, 0.8..1.2, −187.3) pushing forward: `gdbwatch.py` on Bond's substate (obj+0xF6) — does
   substate 1 (climb) engage, and what does `Player_CollLadder` 0x196118 read at the pick? The
   ladder objects are #164/#495 (y 0.8..12.1 [D probes]).
2. **The secret door's second press**: in the game, ONE E press on the plate opens the door 75 %
   (rest 0.75 [D #544 param 5]) and a second press completes it — confirm the playtest feel (does
   the door visibly stop part-way and need a second E?).
3. **The vent ascent** (carried from fix-0c-hatch): the walkway → the duct top route.

## Verification (what ran)

On the final tree: `nf-game` lib 537 passed / 0 failed; `nf-view` targeted: walkreal_0700000c_live
1/0 (the full walk, 2917 ticks), walk_0700000c_live 4/0, fix_ss_operate_0c_live 1/0 (new),
npc_doors_live 6/0, walkreal_0700000a_live 6/0, keyboard_0700000a_live 1/0, level_end_live 1/0,
fix_0c_hatch_live 4/0 (2 ignored), fix_level_0c_live 1/0, fix_ss_grate_live 9/0 (6 ignored),
fix_party_mission_live 4/0 — the E-deferral and the SS contact changes' blast radius.

**The full fast suite did not complete in this sandbox window** (the baseline run was killed at
~40 % to free the 2 cores for the dev cycle; the final-tree rerun did not fit). Baseline partial:
the only failures were the allowed disc_golden trio + probe_pylon_render (a wgpu-GPU probe that
cannot pass headless — environmental) + frame_cost_live and fix_death_restart_live CRASHes
(600 s per-binary timeouts under my concurrent build; not reproduced alone). The next message
should start `tools/test/suite.py` fresh and read the total.

## Doc corrections

- `docs/levels/0700000c.md` §4: the secret-door AND's writer is the E-interactable plate #558
  (mask 4 = the use-key flag [M 0x1b2318:23-33]) + the body-touch #559 — NOT "a guard's patrol is
  part of the key"; no drone touch is involved [D #558 p1 = 4 raw].

## Usage

- Input tokens / output tokens / context used / cost: **not visible to me** (no token or cost
  figures are shown in this environment).
