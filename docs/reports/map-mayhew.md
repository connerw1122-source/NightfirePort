# map-mayhew — the Mayhew estate missions 07000005–07000008: maps, objectives, walkthroughs

Branch: **s/map-mayhew** (from origin/master 6a012ee, rebased onto d91f5bd). Docs: `docs/levels/07000005.md` …
`07000008.md`. Code: `crates/nf-game/src/mission.rs` (new, this task's own file) + one additive hook in
`crates/nf-game/src/drone.rs` (the `Drones::mission` field + one monitor call per tick) + the volume-containment
branch in `crates/nf-game/src/triggers.rs::touched_by` (shared file — the one behavioural change outside my files,
kept to 10 lines). Tests: `crates/nf-view/tests/walk_0700000{5,6,7,8}_live.rs` (9 tests).

## Per-level verdicts

| Level | Completable (headless walkthrough) | The chain |
|---|---|---|
| 07000005 | **YES** (both campaign routes) | the truck ride parks on the ch-0x70 plate → objective 1 → ch 0x1D → exit #118 → 07000006; or E at the gate door → fallback ch 0x1F → the same exit at the matched entry (F1, level_carry_live's spawn test) |
| 07000006 | **YES** | the LoadPoint #746 volume → ch 0x83 → the dressing script 0x06000071 → at its tick 1129 ch 0x02 (objective 2) + event 0x12 → 07000007 |
| 07000007 | **YES** | the spawn plate #411 (ch 0x47) → the hall plate #412 (ch 0x48: the weapon lock releases, the P2K auto-draws, objective 1) → the roof exit #418 → 07000008 |
| 07000008 | **YES, with one driven gap** | the intro plate #237 (ch 0x49 — completes 07000007's objective 2 through the carried records) → the boarding ch 0x4A + the ride ch 0x4B → all six Mayhew objectives → ch 100 → **level_exit 0x07300008** (the driving hand-off [M 0x182c60:100]) — 07000008 has NO LOAD_LEVEL placement, the mission layer is its only end |

## What was ported

**The objective layer — `Mission_MonitorObjectives` 0x1827d8 / `Mission_Update` 0x182c60** (031-fix-mission had not
landed; the brief says build my own in my file). The game's table is 18 records at EE 0x2a4354 ([L] ee_start_05.bin
dumped — every mission's records carry the FIRST level's code as their level key, and the monitor matches against
BaseMap, so `base_map()` maps a level to its chain start). Per objective: label, complete channel (+0x0C), reveal
channel (+0x0F), inverted flag (+0x10 bit 1), state. Per tick: reveal → complete → (inverted) fail; the all-done
AND is **live** over every objective's channel; ch 0x61 (alarm) / 0x60 (civilians) → fail (ch 99); all on → ch 100.
`Mission_Update` maps ch 99 → the fail end (0x7000048) and ch 100 → the complete end — **0x07000008 → 0x0730008**
(the driving level; my first constant had an extra nibble — the walkthrough caught it). The Mayhew chain [L]:
0x70 → 0x02 → 0x48 → 0x49 → 0x4A → 0x4B. The whole 18-record table is in mission.rs so the other map sessions
build on it.

**The trigger-volume containment fix** — `Trigger_Hit` 0x1b1fc8 does NO geometry: it classifies the collision
pass's contact list (obj+0xD0), so anything the collision system registers against the trigger's glist bounds is a
contact — including a toucher standing INSIDE a room-shaped volume where no triangle is within reach. The port's
triangle-only `touched_by` missed exactly that: 07000006's LoadPoint #746 is an open 6-triangle box [D measured];
Bond standing in it touched nothing and the dressing scene never started. `touched_by` now also hits when the
toucher capsule is fully contained in the trigger's bounds (fail-before: walk_07000006; the full trigger suite
stays green — flat plates are unaffected).

## The map corrections the docs record (from the disc dumps)

- **07000007 HAS a LOAD_LEVEL** (#418 at (135.9, 50.1, −76.5), gate 0 → 07000008) — the research note "no recorded
  exit" was wrong; the placement dump found it. Its start plates: #411 (ch 0x47, at the spawn) and #412 (ch 0x48,
  the hall).
- **0x1E IS reachable**: E on 07000005's #112 writes fallback 0x1E → 07000006's start #511 (channels.md's
  "unreachable" note is wrong).
- **07000006's exit has no 0xE8** — it is the LoadPoint script chain; the dressing script 0x06000071 lives IN the
  level bin (07000006 has no loadable records — the packs are only 07/08's, and both ARE on the slim ISO [D
  FILES.BIN]).
- The Mayhew music updaters are keyed 07 → `UpdateMFX_01CastleC`, 08 → `UpdateMFX_01CastleD` [M 0x114e70:219-244].

## What's left (with the game function behind each)

- **ch 0x4A/0x4B's writers** (07000008's boarding and ride): SP #295 "ScriptPlayer_CableCar" 0x0600004C behind mux
  #235 = AND(0x4A, ch 4) [D levels.md §5.3] — the pack scripts load but their channel-event timeline isn't dumped;
  the walkthrough drives the two channels and the doc flags it. Next step: dump 07800008.bin/07900008.bin's event
  streams (the same probe I used for 06) and wire the gondola interaction (`Player_Activate` 0x1a5170's hoist
  branch).
- **The mission-fail reason plumbing** (`DroneFunc_SetMissionFailReason` 0x13e560 — the unarmed-impact fails on
  07000007, the killed-civilian fails; `Mission_Update`'s fail presentation, MenuManager_Create): the port's ch 99
  path exists but no writer yet — 031-fix-mission's file.
- **The objective HUD** (Txt_BindLabel 0x1cad60 labels 0x4000000-5 / the 0x2000053 "Objective Complete" messages
  via Text_AddMsg): presentation, the HUD session's.
- **F3's tux skin** (0x050000d third-person swap [M Player_Init 0x1910f8:96-107]) and **BondTalk** (0x1446b0) —
  noted, unported.
- **F5's walk-in graze**: the port's toucher capsule can graze cliff plunge volumes on foot (fix-level2's open
  item); the E path is closed (the mask & 4 gate [M 0x1b2318:38-40] — asserted by walk_07000008's sister).
- **The FuseBox** (07000006 #747, `FuseBox_Update` 0x1adfe8) — 060-fix-objects2's; the doc records the channel
  web (p1=57 the searchlight gate, p2=55).

## Spots for the user to check in play (`--bond x,y,z,yaw`)

- 07000005: the gate door at `--bond 30,38.8,38.2` — E opens the castle route (fallback ch 0x1F); the truck ride
  end (fix-truck) should now also complete the objective banner ("Breach the castle walls").
- 07000006: the LoadPoint room at `--bond 35.5,44.5,-54` — standing there should start the dressing cut-scene and
  load the party (F1's spawn matching + F2's carry are level_carry_live's).
- 07000007: the spawn `--bond 33.4,44.5,-70.6` (fists, no guns — F4) → the hall at `--bond 94.3,43.6,-45.9` (the
  P2K auto-draws) → the roof exit `--bond 135.9,50.1,-76.5` (E loads the gondola).
- 07000008: the intro plate `--bond 139.2,49.5,-81.2` → the gondola boarding (the interaction itself is the
  unported gap — in play the E-on-gondola may not raise ch 0x4A yet; that's the report's first "left" item).

## Verification

- The 9 walkthrough tests + the 2 mission unit tests (the chain order + the skip sisters), all NF_ISO on the slim
  ISO, all shown failing before their fixes (the containment branch, the driving-code nibble, the monitor-tick
  ordering).
- Full suite: nf-game 20 ok, nf-formats/nf-audio ok (only the 3 allowed slim-ISO `disc_golden` failures), nf-view
  40 ok — zero unexpected failures.
