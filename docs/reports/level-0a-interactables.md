# level-0a-interactables (task 710a) — 0700000a, Tower B (the office floor)

Branch `s/level-0a-interactables`, based on origin/master a8401b57, finished merged with
origin/master batch 1017z (4e3c3cf5).

## What's done

1. **The census** (`docs/levels/0700000a.md`, the "Census (task 710a)" section): every main-map
   placement type with its `parsemap_create_dynamic_objects` creator [M 0x1d04f0], the
   placements' parameters and channel wiring [D], what the game does with it [M], and the port's
   status. Counts: 0x24 player ×1, 0xEA TouchOnce ×19, 0xE8 LoadLevel ×1, 0x35 PCQWorm ×3,
   0xD9 ScriptPlayer ×5, 0x29 Switch ×9, 0xEB Touch (the laser web) ×30, 0x20 Break ×45,
   0xF0 Pickup ×5, 0xDB Door ×19, 0xE2 SS ×3, 0xE4 Hurt ×5, 0x22 Ladder ×1, plus the off-route
   0x27 turrets ×6 (not ported), 0x30 monitor ×1 (origin), 0x71 rotor, 0xFB MusicTrigger,
   0xFA ThirdPersonIcon and the AI infrastructure. No safes, keypads, terminals, searchlights,
   fuseboxes, vents, wires, grapple points or vehicles exist on this level's main map [D — the
   placements dump lists none].

2. **The two known open items, both resolved:**
   - **The cab-mouth SS lift doors (placements 524/525)** — the walkreal_0700000a_live test's
     `#[ignore]` premise is stale. The doors' state param is **1 (armed)** [D placements dump],
     so `SS_Update` 0x1a9948's state-1 sweep runs at create and the wrap (wrap_flag 1) holds the
     open end pose [M, grates-02's state model, landed 10-16 AFTER the ignore was written 10-13].
     There never was a missing writer; the [L] start snapshot (leaves closed) caught t = 0
     before the first update. The test is un-ignored and walks the full route: spawn → the
     Q-Worm PC → the security keyboard → the hatch → level_exit 0700000b.
   - **The keyboard lasers** (switch #643, out ch 0x3B, gate ch 0x3C) — the switch is provably
     **inert on fresh-boot data**: no trigger, switch or script event names channel 60 [D the
     triggers/nis dumps — no 0xD/0x11 events at all in 0a's script pack], the load snapshot
     carries ch 0x47 only [L ee_start_0700000a.bin @0x26fc90], and `Switch_Activate` 0x1b02a8
     refuses the toggle while the gate channel reads high [M]. The walkreal asserts the
     end-to-end inertness (ch 0x3B never on through the whole route).

3. **One live test per KIND** (`crates/nf-view/tests/interactables_0700000a_live.rs`, 7 tests,
   each through the real input path with its sister):
   - `ss_lift_doors_sweep_open_at_create` — the leaves pose at key 0 at load, sweep aside within
     the 75-tick create sweep, and hold the open pose.
   - `armour_pickup_taken_by_walking_over_it` — BodyArmourHeavy #730 grants armour 50.0 on the
     walk-over [M 0x18e650 case 3; D param 2 = 50]; sister: 0 before, the other copy untouched.
   - `vending_glass_breaks_under_fire` — the vending-machine glass #379 shatters under the real
     fire path [M Break_Kill 0x12b970]; sister: the far machine #378 stands.
   - `laser_web_crossing_fires_the_alert_channel` — walking through beam #607 fires ch 0x1F [D
     triggers dump]; the lock = disable-while-on [M triggers.rs's lock gate], so fresh-boot beams
     are live; sister: ch 0x1F off before the crossing.
   - `alarm_switch_refuses_the_player_use` — E on the alarm switch #619 does nothing: a class-3
     (player) activator never toggles ch 0x61 [M 0x1b02a8's `cVar1 != 3 || ch != 0x61`]; the
     control arm proves the use path alive (the same E opens a door).
   - `vent_ladder_grabs_and_climbs` — walking into the vent ladder #686 attaches the climb
     [M Player_CollLadder 0x196118] and ascends [M Player_Climb 0x196310].
   - `lift_shaft_hurt_volume_deals_the_fall_damage` — the shaft HurtPoly (#745-749, 1000 damage
     [D]) kills on the fall-in; sister: no touch on the office floor.

4. **Every model draws**: the loader's per-level census on 0700000a prints "0 unresolved" world
   instances, no "materials have no texture (white fallback)" line, 0 drones with a skin from
   another archive, "0 without mesh or datum" for the held weapons. `textures_live`
   (every_texture_resolves_and_every_mesh_is_drawn, which includes 0x0700000a) is green. The
   GPU-side `NF_TEX_TRACE=1` sweep is not runnable headless (the flag lives in the gpu pass);
   the headless checks above are the model/texture verification.

## How it was verified

All green on the final tree (merged with origin/master batch 1017z):
- `interactables_0700000a_live` 7/7, `walkreal_0700000a_live` 1/1 (un-ignored),
  `walk_0700000a_live` 3/3, `walkreal_07000009_live` 3/3 (the neighbour), nf-game lib 279/0,
  `pickup_live` 1/1, `textures_live` 1/1.

**The full suite.py did not fit the window.** The sandbox kills background processes between
tool calls (two starts died in the build phase, one in the test phase after ~77 of 522 test
binaries with 0 failures up to that point), and the foreground tool window is capped at 10
minutes. The foreground runs listed above are the verification; the crates my changes touch are
nf-view (tests only) and nf-game (untouched source; lib + textures_live + pickup_live run).

## What's left

- **The Q-Worm PC's walked approach** (unchanged, documented): the port walls off the PC's
  cubicle at every height, so the walked route cannot fire the install through the wall; the
  install KIND is proven through the real gun path in `walk_0700000a_live::qworm_install_by_firing_the_gadget`
  (the game's own walkthrough also hand-places Bond inside the cubicle). Live check for the main
  agent (unchanged): in PCSX2, walk to Q-WORM_PC_1 on 0700000a and report whether the real game
  lets Bond inside the cubicle — if yes, the port's collision for that office cell is the bug
  and the fix belongs to the collision owner.
- **The keyboard SP #60's use-target** (unchanged, documented): the placement transform is the
  origin [D]; the game's own E dispatch cannot reach class-0x1B SP bodies [M 0x1db9a8:71] and
  the keyboard's indicator entities exist only after its script plays [D nis]. The activation
  writer stays a live check: gdbwatch `switch_channels` (0x26fc90) ch 0x3A on a real run while
  E-ing the security keyboard.
- **The GT turrets (0x27 ×6)** are not ported (off the required route). Live check: do the
  sky-bridge turrets track/shoot on a real pass?
- **walkreal_probe13 hung in this sandbox** (a master-tree test binary, > 590 s vs 4 s in the
  14:28 log) — worth a look by whoever runs the next full suite; not from this branch's changes.
- The ch 0x37 (laser disarm lock) writer on 0a is likely carried from 07000009 [S].

## Usage

not visible to me — this environment exposes no token, credit, context-window or cost figures
for the session.
