# level-0b-interactables (task 711a) — 0700000b, Tower C (executive floor + roof)

Branch `s/level-0b-interactables`, cut from origin/master 57f43077. Every claim here is tagged:
[M] decompile (function + address), [D] disc bytes, [L] live measurement, [S] suggested.

## What's done

1. **The census** (`docs/levels/0700000b.md`, the "Census (task 711a)" section): all 86 dynamic
   placements on 0700000b by type, each with its parameters, channel wiring, the game's function
   [M] and the port's status. Types: 0x24 player ×1, 0x0F oDrone ×10, 0x27 GT turret ×5, 0x29
   Switch ×1, 0x2E Lock ×1, 0x35 PCQWorm ×1, 0xD9 ScriptPlayer ×10, 0xDB Door ×9, 0xE2 SS ×1,
   0xE6 "Cover Low Node" ×11 (roof cover nodes — the old doc's "11 hurt volumes" was a misread;
   the port's `hurts` list on 0b is empty [D probe]), 0xEA TouchOnce ×17, 0xEB Touch ×30, 0xF0
   Pickup ×5, 0xFB MusicTrigger ×2, plus the 0x2F Copter ×1 and the 0xF1 spawners ×2. No safes,
   keypads besides the 0x2E, monitors, terminals, fuseboxes, vents besides the grill SS, hatches,
   ladders, wires, grapple points or vehicles exist on this level's main map [D placements dump].

2. **The two known open items:**
   - **The executive-floor locks — both now pinned green through the real input:**
     - The keypad (0x2E "Lock - KeypadLocked" #178, burn out ch 0x2F [D p5]) burns under the
       Laser watch 0x4E through the real fire path — `Lock_Update` 0x1ae3d8 case 0's
       laser-family arm, the open writes `switch_channels[47]` [M :194-195] — and door #151
       "door_waitingroom_exec_office" (lock ch 47 [D]) refuses the E before the burn and opens
       after it (sister in the same test). The lock system is master's fix-objects2 port; the 0b
       instance needed no code change, only the test.
     - The keycard lock (0x29 Switch #249) E-toggles ch 9 [M Switch_Activate 0x1b02a8] and the
       locked sdoor1 (lock ch 9 [D #286]) slides on the same E — the walked approach stands at
       the lock's front (the walkreal's boardroom-table blocker pins the OTHER approach; this one
       is table-free).
   - **The roof helicopter (0x2F Copter #328)** — present and held at its placement by the
     closed gate ch 0x2A [D p6; M Copter_Update 0x17dbb8 head], no drift over 240 ticks, the
     arrival ch 0x66 unwritten, the parachute pickup #280 untouched on its deck. The gate's
     writer is not in the level's dumps [D trig/nis dumps] — the copter hovers for the whole
     level as the data stands; the parachute is taken from the hover (the walkreal's real
     pickup touch + the deck jump both green).

3. **One live test per KIND** (`crates/nf-view/tests/interactables_0700000b_live.rs`, 9 tests,
   each through the real input path with its sister):
   - `keycard_switch_e_opens_the_sdoors` — E #249 → ch 9 → the sdoor slides [M 0x1b02a8; D
     #286 lock ch 9]. Sister: ch 9 off before.
   - `keypad_lock_burns_open_the_exec_office` — the watch burns #178 → ch 0x2F → door #151
     opens; sister: the same E refused pre-burn.
   - `waiting_room_door_opens_on_use` — the plain door #13 E [M Door_Activate]. (The walked
     approach crosses the boardroom furniture row walkreal pinned; the documented stand-in
     transit reaches the door front, the E is real.)
   - `vent_grill_ss_e_sweeps_it_open` — the interior vent grill SS #98 (state 0x40, mask 4 [D])
     E-sweeps off key 0 and the wrap holds the end pose [M SS_Activate 0x1a9ea8;
     0x1a9948:121-126]. Sister: the key-0 pose before.
   - `dome_laserbeam_crossing_fires_ch0d` — walking through beam #272 fires ch 13 [D out param;
     M triggers.rs]; the lock ch 0x3B is off at boot so the fence is live [D probe]. The band is
     0.28 m and the re-firing touch's off-write clears the channel on exit — the test polls
     during the crossing. Sister: silent before.
   - `exec_floor_pickups_taken_by_walking_over` — BodyArmourHeavy 50 [M 0x18e650 case 3], the
     RocketLauncher arms weapon 0x2E [D p1 = 46], a BOND_BONUS is consumed [M :234-236];
     sisters: armour 0 / rocket un-owned / the bonus sitting before.
   - `rooftop_vent_use_pulses_ch77_and_spawns_the_cover` — E plate #327 (mask 4 [D]) from the
     vent platform fires ch 0x77; SP #326 (mode 64) plays 0x060009F9: the cover entity (kept by
     the script's Event 0xC at its tick 31 [D nis]) remains, and the script's own Event 0xD
     writes ch 0x77 = 0 at its tick 10 [D nis dump] — the pulse + clear asserted.
   - `roof_helicopter_gated_hover` — above.
   - `music_triggers_created_with_the_disc_params` — the two 0xFB placements create with the
     disc's wiring (ch 9 → event 41, ch 0x4E → event 17 [D #269/#283]); the countdown behaviour
     is WorldObjects' own unit pin (music_trigger_tests). The board slot is consumed inside the
     frame, so the audible beat is the audio layer's (live check below).
   - The Q-Worm install kind (0x35) is already the real gun shot in `walkreal_0700000b_live`
     (green here too); the TouchOnce/Touch touch kinds are the walkreal's arrival/terminal
     plates and my laserbeam; the spawner templates are the models sweep's pin.

4. **Every model draws** (`crates/nf-view/tests/models_sweep_0700000b_live.rs`, 2 tests):
   - 5630 materials, **0 white-fallback** — every material resolves a texture (the 08 sweep's
     pattern; `NF_TEX_TRACE=1` is the GPU pass and not runnable headless — this is the same
     condition through the scene's material table).
   - All 10 oDrone guard templates exist with non-zero skins, the two AI_DroneSpawners (#267
     group 2 / #268 group 3 [D]) hold them absent until their spawn conditions run [M
     DroneSpawner_Control 0x137e68] — the sweep asserts the templates' skins, not the AI spawn
     timing. `textures_live` (nf-game, includes 0x0700000b) green.

## How it was verified

All green on this tree (nf-view tests `--test-threads=1`, NF_ISO set):
- `interactables_0700000b_live` 9/0, `models_sweep_0700000b_live` 2/0,
- the neighbours: `walkreal_0700000b_live` 1/0, `walk_0700000b_live` 4/0,
- `nf-game` lib 279/0, `nf-game` `textures_live` 1/0, `pickup_live` 1/0, `nf-view` lib 21/0.

**The full suite.py did not fit the window** — the sandbox kills background processes between
tool calls (the runner died twice in the build phase here, the same shape 709c/710a reported),
and the foreground window is 10 minutes. The foreground runs above cover every crate my changes
touch: nf-view (tests only) and nf-game (source untouched; lib + textures_live + pickup_live).

## What's left

- **The GT turrets (0x27 "GunSamuri-Placement" ×5)** are not ported — no case in the port's
  dynamic dispatch (`worldobj.rs` handles 0x41/0xd2/0x3b/0xe7/0x71/0xfb). The game's side:
  `case 0x27 → GT_Create` 0x17ffa0 [M 0x1d04f0:113-115], `GT_Update` 0x180498 tracks/fires,
  `GT_ApplyDamage` 0x1816d8. Params: p2/p3 = scripts (0x06000421/22, 0x0600041C/1E [D]), p4 = 14
  (terminal room ×2) / 28 (exec floor ×3), p5 = 0/78. Two sit in the Secure Terminal Room
  (#174/#175) — they guard the files room. A port needs the GUNTURRET struct map, the state
  machine and the death chain; that is a task of its own (the 0a report's "not ported, off the
  required route" note applies to its 6; on 0b two of the five are ON the objective route).
- **The lift ride (SP #232, mode 8, out ch 0x75)** — the port's SP machinery plays the loop
  (mode 8 = SCRIPTINFO+0xAB2 bit 3, `Script_Run` 0x1d8548:130 restarts the stream [M]) but does
  not carry a Bond standing on the cab; walkreal teleports (the documented stand-in). The
  carry needs the game's standing-on-moving-object rule (the ladder carry
  `pos += ladder(+0xC0) − ladder(+0x40)` [M ladder.rs] is the port's precedent).
- **The writers of ch 0x54 / 0x06 / 0x59** (the files obj9's OR leg, obj10) — still not in the
  dumps [D; unchanged from the doc's open questions]; the walkthrough drives them.
- **The music board slot visibility in the sim** — see the live check below.

## Live checks for the main agent

1. **The lift carry** — in PCSX2 on 0700000b, ride the executive-floor lift from y ≈ 47 to the
   roof and gdbwatch Bond's BLData position while the cab moves: does the game carry the player
   (pos follows the cab per tick), and does the mid-shaft plate (#171, out ch 0x53) fire on the
   way? Also whether the lift_stop_trigger #245 (out ch 0x75, y 60.3) fires at the top.
2. **The music beat** — on the same run, E the keycard lock: does the tower music beat change
   within a second (the MusicTrigger's event 41)? If the port's board consumption is right the
   beat is audible-only; a `gdbwatch switch_channels`+music-event check settles it.
3. **The GT turrets** — do the Secure Terminal Room turrets (#174/#175) track and fire on a real
   pass, and what kills them? (Feeds the port task's priorities.)
4. **The copter gate** — does anything on the real run write ch 0x2A before the roof (the vent
   route's SP #326 plays 0x060009F9 which writes ch 0x77; the 0x2A writer may be one of the
   pack scripts the port's dumps did not reach)?

## Usage

not visible to me — this environment exposes no token, credit, context-window or cost figures
for the session.
