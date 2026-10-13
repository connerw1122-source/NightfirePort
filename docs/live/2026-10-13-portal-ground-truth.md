# Portal culling: the real game at three spots (2026-10-13, main agent)

Captured in PCSX2 (NTSC-U) with `tools/trace/pcsx2_shot.py --pos x y z yaw --pitch p --wait 120 --god`: the same
placement jump the port's `--bond x,y,z,yaw,pitch --frame 120` makes (position, yaw and pitch written directly,
held 120 frames). Compared with the port's walk on / `NF_NOPORTAL=1` renders at master a5bff8b + fix-portal2 [L].

| Spot | Real game | Port, walk on (fix-portal2) | Verdict |
|---|---|---|---|
| 07000006 `7.24,42.03,-39.31,10.161,0.428` (the courtyard, looking up at the walkway and the castle) | the masonry under the left walkway and the lower right castle wall draw **solid**; no sky through them | both missing, sky shows through | **port bug**: not a teleport artifact; the game keeps those cels visible straight after the jump |
| 07000007 `61.11,44.12,-52.58,-2.541,-0.010` (the ballroom window) | only the snowy mountain through the window; **no lit building** | no building (with `NF_NOPORTAL=1` the building draws) | port walk **matches**: the game hides that room |
| 07000007 `34.25,44.13,-70.71,1.549,-0.070` (the clock hall) | two guards, the clock, the shields | identical | match |

Task 130c-fix-portal3 owns the 07000006 fix.

## The game's visible-cel list at the 07000006 courtyard spot (GDB, later the same day)

`tools/trace/gdbcall.py 0x1e86e8 --spec "in:a0:0x10 in:@(a0+4)+0x80:0x10 out:0x30d0bc:4 out:0x34b3b0:0x200"` in the
GDB-server PCSX2, Bond held at `7.24,42.03,-39.31` yaw 10.161 pitch 0.428 (god mode), frames 27567-27569 [L]:

- `Vision_Portal_Recurse` runs **three times per frame**: viewer 0x01fc6c10 (the main view), 0x01fc69b0 and
  0x01fc3080 (two other viewers whose cels 0x1fc68f0 / 0x1fc2fc0 are not world cels). So a PINE read of
  VisCelList/VisCelCount between frames shows the LAST viewer's list, not the main view's (the atlas's
  "VisCelList holds a name/world pointer" puzzle). VisCelCount is the u16 at **0x30d0bc**; VisCelList is 0x34b3b0,
  8-byte entries {cel, u16 depth}.
- **Main view: VisCelCount = 2, identical every frame**: the eye cel **0x0179f230** (class 0x0804c047, sphere
  (4.5, 43.5, −42.1) r 11.4: the courtyard strip, the port's cell 33 per fix-portal3) at depth 0, and
  **0x0179fb30** (class 0x0804c047, sphere (−13.0, 49.0, −37.0) r 44.0) at depth 1.
- With only those two rooms the game draws the walls the port drops, so they come in through these rooms' own
  glists or their `+0x18` chains. **0x0179f230's chain is empty; 0x0179fb30's chain has 40 generic nodes:**

| node | class | sphere (x, y, z) r |
|---|---|---|
| 0x179e930 | 0x08008019 | (3.0, 46.6, −32.8) 4.5 |
| 0x179b4b0 | 0x00008019 | (3.0, 46.6, −26.2) 4.5 |
| 0x179b270 | 0x08008019 | (3.0, 51.1, −33.7) 17.6 |
| 0x1799830 … 0x17983f0 (28 nodes) | 0x00008019 | r 0.4 each, x −0.7..6.6 step 0.5, y 46.5, z −32.8 and −26.2 (two railings) |
| 0x1793830 / 0x1793770 / 0x17936b0 / 0x17935f0 | 0x00008019 | (−26.0 / −23.0 / −20.0 / −6.5, 48.9, −59.3) 2.3 |
| 0x17926f0 | 0x08008019 | (4.7, 46.0, −27.0) 8.4 |
| 0x1792570 | 0x08008019 | (−31.5, 49.7, −26.0) 1.5 |
| 0x17923f0 | 0x08008019 | (−18.5, 52.1, −59.0) 15.5 |
| 0x1790cb0 | 0x08008019 | (−34.0, 51.3, −41.5) 20.0 |
| 0x17905f0 | 0x08008019 | (−17.0, 46.5, −38.6) 1.5 |

The chain walks `+0x18` from the room record; node class 0x8019 with or without 0x08000000, as fix-portal2's census
said. This is the [L] answer to the chain-builder window question: compare the port's `CelGraph::chains` for its
equivalent of 0x0179fb30 against this list.

## View_AddForcedObjects at the courtyard spot: not the wall path (PINE, later the same day)

fix-portal4 decoded `View_AddForcedObjects` 0x1e5fb8 (draws frustum-tested objects independent of the VisCelList) and
proposed it as the courtyard-wall path. Checked live at the same spot [L]: the global object list
(`DynamicObjList` 0x2705a0 + 0x14 = `control_first_object` 0x134300, next = obj+0x14) has **710 objects**; with the main
viewer's mask (`1 << (glb_viewer[0]+0x28 & 0x1f)` = **0x1**; the global `object_display_mask` 0x30d87c reads 0x80
between frames, the last viewer's) only **12 pass the function's static gates** (`+0xf8 & mask`, `+0xd8 != 0`,
`+0xf0 & 0x8000`, not `+0xf0 & 0x10`):

| class (+0xff) | count | spheres |
|---|---|---|
| 0x3d | 6 | at Bond (7.0..7.7, 42.3..43.0, −38.8..−40.0), r ≤ 0.2 (the player's own parts) |
| 0x00 | 2 | (3.8, 42.4, −36.4) r 4.9; (28.1, 42.0, −18.2) r 2.0 |
| 0x1c | 4 | (4.6, 50.1, −64.9), (4.4, 43.1, −51.1), (27.9, 41.1, −23.6), (35.9, 41.1, −16.3), all r 1.3 (doors) |

None is a wall (the masonry spans tens of units). **So the forced-objects list does not draw the courtyard walls.** With
VisCelCount = 2 (the eye cel and 0x0179fb30), the wall geometry must reach the draw through those two rooms: room
0x0179fb30's own glist or its 40-node chain. Where the port attributes those wall meshes to other rooms (its cells
20/22/38/13), the attribution is what differs from the game.
