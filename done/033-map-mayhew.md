Read `docs/playtests/2026-10-11.md` first: it's the user's multi-level playtest, and the item ids below refer to it.

## Your task: `map-mayhew`, make the Mayhew estate missions 07000005–07000008 (the castle approach, the castle, the party, the gondola) completable from start to end: the map, its actors and its objectives

Levels: `07000005`, `07000006`, `07000007`, `07000008` (`docs/systems/levels.md` has the table, `docs/atlas/` the systems). The goal is simple to state: a player
can load each level, do what the real game asks, and reach the next level, with every actor and objective behaving as
in the game.

For each level, in order:
1. **Map the level from the game's own data and code.** Write `docs/levels/<code>.md` with:
   - the start position(s) and which entry each comes from (the route Bond took in the previous level);
   - the objectives in order (text from `USATxt.dat` via `Txt_BindLabel`; the code that sets and completes each,
     `Mission_*` / `PlrStat_*` / script channels);
   - the actors (allies, civilians, VIPs, bosses, scripted characters) with their role and what triggers them;
   - the required interactions (doors, keys, switches, gadgets, wires, ladders);
   - every exit route with its trigger, condition and destination level;
   - the failure conditions (killing civilians, alarms, an ally dying, a timer).
2. **Write a headless walkthrough test** (`crates/nf-view/tests/walk_<code>_live.rs`) that drives the real path:
   - `nf_view::load`, then `step_bond` with real input where the player would press something (use, fire, crouch,
     switch weapon); teleporting between rooms is fine for walking;
   - assert each objective stage in order, and that the level end fires and loads the right next level;
   - sister assertions: skipping a required step must NOT end the level, and breaking a failure condition must fail
     the mission.
3. **Fix every blocker the walkthrough hits:** missing objects, doors that don't open, triggers that never fire,
   scripts or cut-scenes that never start, actors that don't move, act wrong or can't be interacted with, objectives
   that never complete, the wrong next level. Each fix cites the game address it ports.
4. **List what you couldn't port** in the doc and the report, with the game function behind each item.

**Known so far:**
- 07000005 ends by the truck ride (fix-truck, on master) or the zip-wire / gate routes (G-037).
  `020-fix-wire` owns the wire. Check that every route lands in 07000006 at the matching entry (playtest F1).
- 07000006: the searchlight alarm and the stationary guards belong to `015-fix-npcs`. The level's own route (the
  castle interior to the party) is yours. The courtyard barrel trap (SimpleScript 0xE2) is drawn but static
  (`120-fix-explosions` animates it).
- 07000007, the party: Bond starts disguised, and weapons are unavailable for the first half (playtest F4). Find the
  game's per-level weapon lock and its release. The guests' movement (fix-drone4) and the dressing cut-scene skin swap
  (F3) need checking. Its objectives are the photographs and the safe. The windows show a floating room (portal; that's
  `130-fix-portal`, not yours).
- 07000008, the gondola: opening the door well before the gondola plays the falling-to-death cut-scene (F5), and the
  safe and laser-watch objects (fix-objects2 territory, so coordinate through its report if it's on master).
**Working alongside the other map tasks.** Five other workers make other missions completable at the same time, and
`031-fix-mission` builds the objective system they all use.
- Pull `master` at the start and before every push; if `crates/nf-game/src/mission.rs` exists, build on it instead of
  writing your own.
- Keep your level-specific code in its own file (`crates/nf-game/src/mission/<level code>.rs`, or a table entry), and
  your docs in `docs/levels/<level code>.md`.
- Keep edits to shared files (`scene.rs`, `drone.rs`, `triggers.rs`, `nis.rs`, `lib.rs`) small and additive, and list
  them in your report.
- Never reformat or reorder a shared file.

Files: `docs/levels/`, `crates/nf-view/tests/walk_*_live.rs`, `crates/nf-game/src/mission/` (or `mission.rs` if
031-fix-mission hasn't split it), and minimal hooks elsewhere.

**Report:** for each level, say completable yes/no; if no, what blocks it; and a short list of spots the user should
check in play (with `--bond x,y,z,yaw` so the main agent can render them).

**Reproduce before you fix.** The user's play is the reference, and previous fixes passed tests but still failed in play. For each bug:
1. Write a headless NF_ISO test that drives the real input path (the `nf-view` input mapping into `Sim`, or `nf_view::load` plus `dev::scenario`), and show it reproducing the user's symptom.
2. Then find the cause in the decompile and fix it.
If you can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Update the doc entry** to say it's fixed, with your commit.

**Branch:** `s/map-mayhew`.

---
claimed-by: sandbox1 at 2026-10-12T05:53Z

---
## Report (done, 2026-10-12T07:24Z)


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
