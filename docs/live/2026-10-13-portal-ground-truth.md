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
