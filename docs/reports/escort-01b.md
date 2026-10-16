# escort-01b — the held s/escort-01 merge failure, root-caused and fixed (2026-10-16)

Branch `s/escort-01b` = `origin/s/escort-01` (f499138d) merged with `origin/master` (22be196b);
the WORKLOG merge conflict resolved append-only (both sides' entries kept).

## Verdict: the door was never the bug

The merge suite's failure text ("an activation every tick kept it ping-ponging 0x8↔0x10") is the
test's own STALE assert message from an older failure. The real failure, reproduced with the
drone's own transition log (`NF_AILOG`): **Mayhew never reached the courtyard shoji at all.**

- fix_mission_live::the_07000001_exit_works_end_to_end: exactly ONE
  `[ailog] escort 1 → LeadWait (path -9.1)` entry, then the escort stood in AllyLeadWait (0x20)
  for the rest of the 5000 ticks, ping-ponging ~2 units around the courtyard junction. The shoji
  stayed 0x2 (CLOSED) the whole run.
- npc_doors_live::the_escort_opens_his_door_without_bonds_help: the escort parked in 0x20 five
  units from the spawn (later, after that was fixed, cowered at the spawn in 0x1f behind an armed
  guard the old recovery could not kill).

## Why: the game's own follow bands vs the tests' teleport Bond

The escort-01 port is game-faithful — all of it, including the door handler:

- `NDrone2_OpenDoor` 0x14aa38 activates only a door whose `+0xF4 & 0x12` (CLOSED 2 | CLOSING
  0x10) — **a door already OPENING (8) is never re-activated** [M]. The port's drain matches.
- `DSTATE_OpenDoor` 0x1681f8 case 3 exits only on `NDrone2_DoorIsOpen` 0x14aaa0
  (`route != 0 && +0xBA8 != 0 && class 0x1c → state == 4`); case 0xc (timer C) re-arms at +2
  ticks [M]. The port matches.
- The follow bands: enter Wait at `path < −9.0`, resume at `path > −7.0`, where path is
  `DroneMove_ObjectMissionPathDistance` 0x153f50 → SM+0x20 [M 0x15e698:60, 0x15eda8:35]. The
  port's ALLY_LEAD/ALLY_LEAD_WAIT match the decomp arm for arm (verified this session).

The failure is the two harnesses' Bond: he was TELEPORTED 3.9 straight-line behind the lead.
On the folded courtyard route 3.9 straight-line is **9.1 arc** — a lag the game's real walking
player never holds. Under the game's own bands that parks the escort in Wait forever (the game
would do the same with a player frozen at that offset). escort_follow_07000001_live (walking
Bond, 6587 band checks) and walkreal_07000001_live were green on the same tree, which localizes
the defect to the two teleport-Bond harnesses.

## What changed (tests only — no sim code)

Both harnesses (`fix_mission_live.rs`, `npc_doors_live.rs`) now ride Bond on the ROUTE polyline
3.9 arc-units behind the lead: his nearest route node tracks the chain, so the arc the bands read
stays a true ~3.9 lag, and when the lead parks at his node the node scan's 5.0 cutoff
([M 0x153f50]) makes the lead's own node the nearest → path 0.0 → the game's own resume fires
(Bond inside the 5.0 straight-line check). npc_doors additionally:

- a walk-past arm for a lead stuck in the PlayerInWay stay band at the courtyard hairpin: the
  3.9 arc is ~2.5 straight-line across the fold — inside the π/3 × 3.5 stay cone
  ([M 0x15ea28 case 3]) forever; Bond rides 4.2 AHEAD once the park holds (piw_ticks > 900,
  anchored on 1.0 of cumulative travel so the idle's root motion cannot reset it) — the game's
  player walks past his halted guide, breaking the cone.
- guard clearing gated on the cower (LEAD_HIDE) — clearing at a PlayerInWay park burned ~360k
  ticks on the unenabled WAIT_SWITCH pair; the cower's sway no longer resets the stall watch;
  the recovery kills (resupply + retry, the escort_follow pattern); the tick cap 60k → 240k (the
  kill loops run inside the budget check's blind spot).
- fix_mission: the stale `at_plate` 2.0-ball proxy dropped — the walking lead stops 3.6 from the
  touch volume's centre while the game's REAL condition (the AND plate touch, ch 0x10, both
  bodies) already fired; `level_exit == Some(0x07000002)` was already asserted right below.

## Verification (probe evidence, on this tree)

- fix_mission: the escort enters 0x81 at t≈1348 with the door bound; the shoji reaches 0x4
  (OPEN, pos01 1.00) at t≈1378 — **the door reaches OPEN**; it auto-closes 0x10 → 0x2 at t≈1738
  (= opened + 360, the game's AUTO_CLOSE [M]); the escort leads the whole loop to the plate,
  ch 0x10 fires, `level_exit` breaks the loop at t≈2900+.
- npc_doors: `0x81 at +173888, hit_door Some(1)` → `shoji OPENED (bond 4.6 away, no use key)` at
  +173895 → `escort crossed the shoji` at +173949 — the escort works the door himself.

## Tests run on the final tree (nf-view only; no sim code changed)

- fix_mission_live 4/4 · npc_doors_live 7/7 · escort_follow_07000001_live 1/1 ·
  walkreal_07000001_live 2/2 — all green (`cargo test --offline --release -p nf-view -- --test-threads=1`).
- Full suite: started in the background on the final tree (`tools/test/suite.py`, NF_ISO set);
  the total line is appended below if it finished inside the sandbox window.

## What's left / follow-ons

- The unenabled WAIT_SWITCH guards resist Walker::shoot in npc_doors' recovery (each kill loop
  burns its 10800-tick cap; the run only passes because the cap now covers it). Whether the
  GAME's bullets damage an unenabled guard is unverified — see the live check below.
- The courtyard junction still needs the nav-gap stand-in (the port's creep gives up where the
  game slides); fix_mission carries the nudge, npc_doors carries the walk-past ride.

## Live checks for the main agent

1. Courtyard WAIT_SWITCH guard vs bullets: `tools/trace/fieldprobe.py` — walk Bond to a
   courtyard guard (e.g. the pair near (5,6)–(7,8) region of 07000001), fire, watch the guard's
   health obj+0x?? drop. Expected (port): damage applies regardless of `enabled`. If the game
   ignores bullets on unenabled guards, npc_doors' recovery should relocate them (the
   fix_mission +60/+60 stand-in) instead of shooting.
2. The escort's Wait band with a REAL walking player: gdbwatch the escort's SM+0x20 (path) and
   +0x9F0 (route index) on 07000001 while Bond follows at walk pace through the courtyard
   hairpin — expected: path dips toward −9 entering the fold and recovers > −7 as Bond closes;
   PlayerInWay (0x1e) fires only transiently (< ~3 s), never parks.
