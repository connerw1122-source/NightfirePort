# Playtest spots — where to stand, what to carry

Task `playtest-spots` (branch `s/playtest-spots`). The user runs playtest checks from a local launcher
(one nf-view launch per check): `nf-view Nightfire-onfoot.iso <level> --bond x,y,z,yaw,pitch --give id[,id…]`.
This file gives, for every feature the playtests exercise, a spot where Bond spawns **standing on a floor
facing the feature**, the `--give` list, and the object the check should find within reach.

The headless proof harness is `crates/nf-view/tests/playtest_spots_live.rs` (NF_ISO): one test per level,
each spot placed through the same contract the window's `--bond` uses (`Bond::new(pos, yaw)` + raw
`bond.pitch`), 60 fixed-60 Hz ticks, then asserts **grounded** (or SS_SWIM in the pond) and the feature
object's **distance + line of sight**. Numbers only — no game bytes.

## How to read a spot line

- `--bond x,y,z,yaw,pitch` — Bond's ROOT at exactly that point (`Bond::new`, `crates/nf-view/src/lib.rs:2977-2982`;
  standing root = floor + 1.0347 [M player.rs ROOT_HEIGHT]). In the air he falls onto the floor.
- yaw radians, **0 faces +Z** (forward = (sin yaw, ·, cos yaw)); pitch in ±1 = ±90° units (the camera
  multiplies by π/2, lib.rs:2472).
- `--give id[,id…]` (hex or decimal, < 0x72) — equips + 3 clips + twin fill, **draws the first id**
  (lib.rs:2986-3024). A weapon the level already owns prints "already owned — drawing it" and draws it
  (the Q-Claw 0x50 on 05); a weapon foreign to the level merges its first-person skin from its owning
  archive (fix-weapon-assets' `--give` merge).
- Reach numbers cited per row come from the game's own constants: laser range 4.0 (`weapon_data[0x4E]+0x1C`
  [L]), Q-Claw bolt range 50.0 ([M Bullet_update 0x12e7b0:158]), door/E use reach 2.5 ([M Player_Activate
  0x1a5170]), Q-Worm shot range 2.0 (`weapon_data[0x58]+0x1C` [D probe]).

## 07000005 — Mayhew A (the castle approach)

Cold start #26 (104.95, 25.12, 108.83) yaw −2.15; no intro blocks this level.

| feature | spot | give | the object to find in reach |
|---|---|---|---|
| **Q-Claw grapple #282** | `--bond 16.5,29.6,58.6,-2.01,0.24` | `0x50` | grapple point **#282** (13.75, 32.29, 57.30) — 4.05 away, aim up-right, hold fire to latch, hang < 2.0; release detaches |
| **Q-Claw grapple #281** | `--bond -18.28,38.19,-71.83,-4.717,0.151` | `0x50` | grapple point **#281** (−3.40, 42.94, −71.77) — 15.4 away (still inside the 50 bolt range); yaw −4.717 ≡ 1.566 wrapped |
| **The 05 cable (wire #251)** | `--bond -23.58,32.24,2.65,0,0` | — | wire **#251 "Wire_Cable"** centre (−23.49, 34.47, 6.26), world sphere r 4.04 — the context-icon mouth; grab with the **JUMP key** (E never grabs wires [M Player_Activate 0x1a5170 has no wire arm; Player_HandleJump 0x1916b0:118-147]), ride with strafe, dismount with jump |
| **Gate guards' ledge** | `--bond 28.3,38.2,41.9,0,0` | a gun (e.g. `0x02`) | the yard-guard posts (28.34, 37.12, 44.86) and (32.06, 37.12, 44.86) — ~3.0 away, ledge floor ~37.1. **The pair spawns only after ch 0x22** (the courtyard approach TouchOnces #21/#22/#23) — spawn on the courtyard first, then walk up, or the ledge is empty [D docs/levels/07000005.md §4] |
| **AT-600 Scorpion 0x2E (guard in view)** | `--bond 79.86,22.92,96.0,-1.38,0` | `0x2e` | the courtyard patrol guard, live at load and walking its route past (61.34, 27.06, 99.49) — 19.3 away at the spot [D §4 + L the real game's own chase-trace stand: Bond (79.86, 22.92, 96.0) vs the gate guard (61.34, 99.50), docs/systems/drone-cover.md:98]. Homing bends inside the 0.66-dot cone; LOS scans at muzzle height pos.y+0.7, not the eye [M fix_weapon_modes_live.rs:103-109] |

Sources: grapple #282 = `crates/nf-view/tests/grapple_hook_live.rs:3-4,35-38` + `docs/reports/fix-grapple-hook.md:151` (the main agent's rendered playtest spot); #281 = `gadget_grapple_user_spot.rs:22-25`. Cable = `fix_wire2_live.rs:31-33,65` (the icon-1 stand, floor ~31.0) + `docs/levels/07000005.md §5` (wire #251 [D]). Yard guards = `docs/models/characters.md:59` ("3 units north of the first placed drone group, facing it") + `docs/levels/07000005.md §4`. Scorpion = `docs/systems/drone-cover.md:98` (the [L] trace pairing) — the fix-melee guard-2 stand (60.4, 27.1, 100.4) has no floor in the sim (the spot was render-only); the trace stand grounds at floor_ny 0.946 with the guard in view. 0x2E is NOT carried on 05 (gadget-carry.md §2) — the give's skin merge is what makes it draw [S: verify the draw on the cold level].

## 07000008 — Mayhew D (the safe)

| feature | spot | give | the object to find in reach |
|---|---|---|---|
| **The safe bolts (laser)** | `--bond 162.24,43.33,-115.64,3.142,-0.063` | `0x4e` | top bolt **#207** (162.24, 43.88, −117.14) 1.5 ahead (yaw π faces −z); hold fire — the burn is instant on first beam contact [M 0x1ae1f8]. Bottom bolt **#208** (162.24, 42.70, −117.14): same spot, pitch −0.46. Both AND into ch 136 → safe door #205 opens (~420 ticks) |
| **The guard-room fusebox** | `--bond 155.19,43.03,-119.39,-1.889,-0.224` | `0x4e` | fusebox **#219** wire (154.35, 43.41, −119.67) — 0.95 away, below eye (aim down). Burn → ch 137 → doorway bolt #217 + door #122 unlock. **E can never cut it** — the Laser's beam is the only cutter [M 0x1a5170 has no class-0x3C arm] |

Sources: `crates/nf-view/tests/fix_safe08_live.rs:421,440-447,460-462`, `fix_party_mission_live.rs:692-693`, `fix_fusebox_cut_live.rs:30-64` (the measured green stance). Bolt/fusebox positions [D placements #207/#208/#219 + object centres].

## 07000014 — the tower sabotage

14's only start is #666/#667 (40.81, 2.40, 65.08) yaw π — these rooms are spawn-teleports (`--bond`).

| feature | spot | give | the object to find in reach |
|---|---|---|---|
| **Fusebox #383 (west room)** | `--bond -88.66,-1.17,31.41,-2.339,-0.698` | `0x4e` | box **#383** (−89.31, −0.22, 30.92), wire (−89.31, −2.22, 30.79) — 1.97 away, pitch ≈ −0.7 (the wires sit ~1.75 under head height) |
| **Fusebox #385** | `--bond -89.35,-1.17,48.59,0.067,-0.739` | `0x4e` | box **#385** (−89.30, −0.22, 49.49), wire (−89.30, −2.22, 49.36) — 1.92 away. Both boxes must burn → 0x87 + 0x88 → AND #391 → ch 0x8C |

Sources: `crates/nf-view/tests/fix_fusebox_cut_live.rs` (the 07000014 sabotage test's measured stances) + the placement dump in `walk_tower2_probe::probe_14_fuseboxes` [D #383/#385].

## 0700000c — the power station

| feature | spot | give | the object to find in reach |
|---|---|---|---|
| **The display vent + hatch** | `--bond 6.0,12.94,-110.5,0,0` | `0x4e` | hatch **#138 "ventmesh-front"** (5.78, 16.40, −112.32) — 3.81 out from the burn stance (the lower walkway, floor 11.84 [L measured]). **Burn the four hinges #139–142 with the Laser** (E does nothing; Lock_Update 0x1ae3d8) → ch 18 sweeps the hatch down in ~50 ticks; crouch + forward through the opening. NOTE: the higher ST-1 walkway spot (5.8, 15.8, −108.5) faces the display but its end wall (z −110.02) blocks the eye ray — the opening sits 3.2 up at the duct (17.2–17.5); the ascent leg stays the parked open check [D fix-0c-hatch] |
| **The hatch, open-side view** | `--bond 5.8,18.4,-110.8,3.14,0` | `0x4e` | the duct ramp above the vent (opening y 15.05..17.50) — the crouch-drop-through spot [D fix-0c-vent2.md:112] |
| **The jetpack photograph (camera 0x54)** | `--bond 5.4,7.35,-113.0,0,0` | `0x54` | camera subjects **#414 "Camera_Subject"** (5.50, 7.56, −110.37) and **#454 "Camera_Subject_Big"** (5.32, 8.77, −109.14) — ~2.9 ahead on the display ground (floor 6.27). **FIRE the camera** (any camera shot touching the subject volume writes ch 0x1c — one shot per object; no E, no LOS test beyond contact [M CamSubject_Update 0x1bc668]) → objective 2 + exit gate #556 |

Sources: the burn stance = `crates/nf-view/tests/fix_level_0c_live.rs:89` (the suite-green burn), the floor [L measured by `playtest_spots_live`], `docs/reports/fix-0c-vent2.md:112-117`, `walkreal_0700000c_live.rs:106` + `walk_0700000c_live.rs:81-84` (the display spot). Hinge/subject positions [D placements]. The hatch's **posed render** is render-side open (fix-0c-hatch.md:117) — the collision moves even if the picture lags; and the ascent walkway→duct leg is unidentified (two `#[ignore]`d traversal tests, live check filed).

## 07000006 — the courtyard (searchlights)

| feature | spot | give | the object to find in reach |
|---|---|---|---|
| **The searchlight beams** | `--bond 4.8,42.09,-16.0,-2.21,0` | — (a gun only to fight the spawned guards) | head **#2** (−1.13, 47.57, −17.33) sweeping the courtyard entrance; standing in the beam's landing spot (≤ 2.0 [M Searchlight_Update 0x1cb900]) arms the head → **alarm ch 55** every tick → siren #702 (sound 329 looped) + combat music within ~1 s [L tests/traces/06-searchlight.trace frame 140866+]. The heads boot dark and light in sequence from script 0x06000066 (ch 64 @ tick 1, 63 @ 59, 61 @ 116, 62 @ 167) — wait ~3 s before expecting the beam |
| **Beam close-ups (render checks)** | `--bond 7.1,48.6,-23.6,-1.4,0.2` (head #4), `--bond 5.5,56.7,-50.1,0.2,0.2` (head #8) | — | the swept beams + coronas on screen |

Sources: `crates/nf-view/tests/fix_06_alarm_live.rs:29,53` (BEAM_SPOT — the real game's alarm spot), `fix_searchlight_remainder_live.rs:19-28`, `docs/reports/fix-searchlight-alarm.md:67-72`. Head positions [D 07000006 placements 2/4/6/8].

## 07000002 — the Exchange (water)

The level's intro NIS grabs control — the launcher's wait-for-control covers it; the tests skip with
`sim.drones.scripts.skip_request = true` while `sim.disabled` [D fix_swim_live.rs:37-40].

| feature | spot | give | the object to find in reach |
|---|---|---|---|
| **The water bank (walk into the pond)** | `--bond -82.3,5.3,32.05,1.57,0` | — | the pond's west edge (water cel 17 x −80.3..−54.1, surface 3.765) — 2.0 ahead (+x, yaw 1.57); walk in → swim (root − 0.3 < surface [M Player_InWater 0x195e50]) |
| **Swimming + the ladder out** | `--bond -75.7,3.0,21.4,0.785,0` | — | hut ladder **#381 "Ladder_medium"** (−74.84, 3.40, 22.22, bottom 0.94 into the pond) — ~1.2 away in the water (SS_SWIM); **hold forward** grabs from the swim and climbs out |
| **The 02 "vent"** | see the note | — | **02 has no crawl vent** [D kind-census, fix-swim-vent.md:58-60]: its "grate/hatch" props are the bathhouse SimpleScripts **#308/#309** (−49.75, 4.76, 6.13 / −49.76, 4.76, 16.94) + grill #440 — drawn props on a script, not crawlable. The real crawl-vent feature is on **05** — see the row below |

Sources: `crates/nf-view/tests/fix_swim_live.rs:43` (BANK — the user's own reported spot, yaw 1.57 walks +X into the pond), `fix_swim_entry_live.rs:36-59,102-106`, `fix_ladder_water_live.rs:63-64` + `docs/reports/fix-ladder-water.md:35` (the ladder stance: centre + (sin,cos)(yaw)·1.2, root 3.0, facing yaw+π = 0.785). Pond/ladder numbers [D water cels + placement #381].

## 07000005 — the crawl vent (the real one)

| feature | spot | give | the object to find in reach |
|---|---|---|---|
| **The wall-hug window (crawl)** | `--bond -34.10,37.28,-28.65,3.142,-0.1` | — | creepwall **#35 "Wall Hug Window"** (−34.10, 37.28, −31.65, ch 113) — ~3.0 ahead (yaw π faces −z); **E** with the crawl third-icon context (12.0 scope) enters SS_CREEP [M Player_CollCreepWall 0x196758]; crawl to the far end. Gotcha: the port's grate door re-writes ch 113 every tick, so `channels.on(0x71)` reads 0 while crawling (fix-swim-vent.md:74-79) |

Source: `crates/nf-view/tests/fix_swim_vent_live.rs:48-50` (the stand spot computed from end B pushed out on the wall's axis) + [D placement #35]. The crawl's channel readback is a known open fix.

## 07000007 — the party (Kimiko scene)

| feature | spot | give | the object to find in reach |
|---|---|---|---|
| **The Kimiko scene trigger** | `--bond 94.2,44.6,-45.9,0,0` | — | TouchOnce **#412** (box 93.3..95.2, 43.2..44.1, −46.4..−45.4, out **ch 0x48**), centre (94.25, 43.65, −45.90) — Bond lands inside the box; the scene 0x06000073 (4326 ticks) starts within ~90 ticks, completes the party objective, releases the weapon lock and auto-draws the P2K. Movement keys never skip it — only the game's 0x19 action [M/D fix_cutscenes2_live.rs:82-96] |

Source: `crates/nf-view/tests/fix_kimiko_handback_live.rs:11` (the repo's canonical rendered spot — WORKLOG 10267/10318) + `docs/levels/07000007.md` + `docs/systems/levels.md:149` [D trigger #412].

## 07000016 — the launch centre (corridor door)

| feature | spot | give | the object to find in reach |
|---|---|---|---|
| **The corridor door** | `--bond 4.9,0.9,-73.0,3.142,0` | — | door[7] = placement **183 "Door_5"**, closed-leaf plane z ≈ −74.2 (leaf x 4.35..5.62) — 1.2 ahead (yaw π faces −z), inside the 2.5 E reach; E opens it, walk through x ≥ 4.4 (the doorway's WEST half x 2.8..4.2 is sealed static geometry [D probe]) |

Source: `crates/nf-view/tests/walkreal_07000016_live.rs:44-54` + the leaf bbox from `walkreal_probe_fb3::scan_world_space_seal` [D].

## 0700000a — Tower B (the Q-Worm PC)

| feature | spot | give | the object to find in reach |
|---|---|---|---|
| **The Q-Worm PC** | `--bond -16.38,0.50,-31.65,-2.356,0` | `0x58` | PC **#688 "Q-WORM_PC_1"** (−16.88, 1.20, −32.15), contact sphere r 0.7, out **ch 0x48** — eye level with the PC, 0.71 away (inside the shot's 2.0 range). **FIRE, not E** [M PCQWorm_Update 0x1aee20 waits for a weapon-0x58 bullet's contact; param 2 = 0 means no E use]: 2–3 trigger taps install the worm → ch 0x48 |

Source: `crates/nf-view/tests/walk_0700000a_live.rs:74-85` (the suite-green install spot). Gotchas: the port's cubicle blocker walls the PC's desk at every height for BOND'S BODY only — the eye-ray is blocked but the Q-Worm's shot installs clean from the spot (the install IS the shot-path proof; walkreal's 112 aim misses were Bond never reaching the desk) — the walk-in is a filed PCSX2 live check. A settled Bond slides ~1.1 off the desk-edge spot (a slope), so fire straight after spawning. A cold 0a load already owns 0x58 (fresh_09 kit) — the give just draws it.

## 07000012 — the tower lift

| feature | spot | give | the object to find in reach |
|---|---|---|---|
| **The lift call buttons** | `--bond -1.55,0.5,7.2,0,0` | — | call button **#702** (TouchOnce, mask 4 = E, volume −1.80..−1.30 × 0.86..0.91 × 8.07..8.34, out **ch 0x59**; the same volume carries the override console #731 → ch 0x49) — ~1.0 ahead (yaw 0 faces +z); mash E (~8 presses) → ch 0x49 + 0x59 → lift doors #677/#678 slide 0.86 apart. The console's lock ch 0x87 does NOT block the E arm [M 0x1b2318 type 1] |

Source: `crates/nf-view/tests/walk_07000012_live.rs:110-150` (the exact stance + the SS sweep assertions) + `docs/levels/07000012.md:45-59` [D #702/#731/#677/#678]. #699's wall button has no dumped volume [S: approximate at (1.1, 8.2)].

## Live checks for the main agent (PCSX2)

1. **05 Q-Claw draw on the cold level**: spot `16.5,29.6,58.6` with `--give 0x50` — does the Q-Claw's first-person skin draw and does fire latch #282? (The give's skin-merge path has no game precedent; the tests equip through the inventory, not the viewer.)
2. **0a Q-Worm walk-in**: can Bond walk into the Q-Worm PC's cubicle in the real game from the corridor, and does aim-fire from the desk edge install? (The port walls it; walkreal 112 shots missed.)
3. **0c vent ascent**: how does the real game get Bond from the ST-1 walkway (14.01) up to the duct ramp (17.2-17.5) at the display vent? (No ladder/creepwall near it on disc; the port's two traversal tests are parked.)
4. **14 fusebox approach**: spawn #666 → the west rooms — is there a walkable route to the #383/#385 rooms, or is the `--bond` teleport the only entry? (14's wedges were probed but not walked.)
5. **02 bathhouse hatches**: do #308/#309 react to anything in the real game (E, walking over), or are they pure script props? (The port treats them as props.)
6. **06 siren sound** with the retail image only: `pss_movies_live`-class check — the beam spot's alarm should start the looped klaxon 329 within ~1 s [L trace frame 140866+].

## nf-view render checks (no GPU in the sandbox)

- Every `--bond` spot above with `--frame 40 --shot spot.png`: the feature object on screen, floor under Bond (no fall-through), the HUD alive.
- The two 06 beam close-ups: the swept beam + corona visible.
- The 05 grapple #282 spot after a latch: Bond hanging below the point, the rope drawn to the muzzle.
