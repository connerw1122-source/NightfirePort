The user (2026-10-17): "09 level elevator buttons dont work. make sure we're prioritizing level work so we can actually complete each level completely and get all models and functions per level working" and "I want EVERYTHING covered. ALL ON FOOT LEVELS". Level work now runs ahead of everything else in the queue.

**Reproduce first, through the REAL path.** Load the level with `nf_view::load` (or `load_weapons` when a
level's own weapon is needed), move Bond with `Sim::step_bond` inputs (walking, turning, the use key, the
watch gadgets, the weapons), let the level's own scripts and triggers run, and observe. No teleporting past
a problem unless the task says so; a teleport to reach a spot is fine when the test then operates the
thing through the real input. Cite the decompile [M], the disc [D] or a fresh-boot PCSX2 value [L] for every
game claim; tests pin only proven behaviour.

## Your task: `level-07-complete`, play 07000007 (Mayhew C: the party) from start to finish

1. A live walk test (extend `crates/nf-view/tests/walkreal_07000007_live.rs`, un-ignore it) that plays the
   level from its start through the real input path and completes EVERY objective — the optional ones too —
   and exits the way the game does (the load into the next level, or the mission results). Every cut-scene
   on the route plays (skipped only with the game's own skip action), every scripted event fires.
2. Fix every blocker the walk hits: collision, doors, triggers, scripts, AI that stops progress, missing
   channel writers. Each fix gets its own focused test with a sister that fails on the old code.
3. In the report: the objective list with how each completes [M]/[D], the route, and anything the walk
   still cannot do with the exact reason.

Known open items on this level (start here): the guests' and guards' party rules; Mayhew's walk to the exit before the mid cut-scene (user report).

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Tests pin only proven behaviour.** Every assertion about what the game does cites its evidence: [M] a decompile
address that actually says it, [D] disc bytes, or [L] a live PCSX2 measurement from a FRESH boot (a save-state resume
skips the boot path). A guess, a port choice or an [S] item is never a hard assertion: mark it `// [S] unverified` and
keep it out of pass/fail. If an existing test contradicts the decompile, the TEST is wrong: fix it, with the address, and
say so in the report. Never weaken a decompile-backed test to make code pass.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/level-07-complete`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox3 at 2026-10-17T13:30Z

---
## Report (done, 2026-10-17T14:31Z)

Closed by the driver: the report is on `s/level-07-complete` (docs/reports/level-07-complete.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-17T15:04Z)


---
## Report from the worker (done, 2026-10-17T15:05Z)

# level-07-complete — 07000007 (Mayhew C: the party) played start to finish

Branch `s/level-07-complete`. The deliverable is `crates/nf-view/tests/walkreal_07000007_live.rs`
(the file did not exist on master — created and un-ignored, two tests), playing the level from its
start through the REAL input path (`nf_view::load` + `Sim::step_bond` inputs, no teleports after the
spawn) to every objective and the game's own exit.

## What's done

1. **`walk_the_party_start_to_finish`** — the full route (below). Both objectives complete
   (the mission record folds to state 4 twice), both cut-scenes play and are skipped only with the
   game's own skip action (`Scripts::skip_request` = `Input_Action(−1, 0x19, 1)` [M nis.rs]), every
   scripted event on the route fires through its real trigger, the exit #418 loads 07000008, and the
   carried load's intro plate fires ch 0x49 so the second objective completes exactly the way the
   game's reveal chain implies. 4734 walked ticks on 07 + 253 on 08, no falls.
2. **`the_group_plate_wakes_the_walkers_and_the_guards_stay_calm`** — the party's zoning rules
   (user report item 1): 12 guests stand in WaitSwitch on switch channel 6 until TouchOnce #476
   (49.26, 45.4, −46.71 [D]) fires as Bond walks through; they wake into PartyGirlInit/CivilianInit,
   the two prop-0x1D walkers take their kind-8 mission routes (the test walks them 14.6/9.9 units),
   and no bouncer enters combat against an unarmed Bond crossing half the party.
3. A wrong evidence note struck: `drone.rs`'s `assign_path` claimed "[S] 07000007 has no group-8
   points or kind-8 paths" — the disc HAS six kind-8 paths ([D] the archive's nav dump: paths 2-7,
   kind 0x8). Replaced with the cited facts.

## The objectives and how each completes

The record is `mission/c07000007.rs` [L ee_start_05.bin @0x2a3e58 — 2 objectives; there are no
optional ones]:

- **0x4000002 "Rendezvous with undercover agents and maintain cover as party guest"** (title
  0x400005d) — completes on **ch 0x48**, hidden until **ch 0x47** [L record]. [D] placements: the
  spawn plate TouchOnce **#411** (33.6, 43.7, −70.6) fires ch 0x47 under Bond at the party start
  (the walk asserts the announce), and the hall walk-in TouchOnce **#412** (94.3, 43.7, −45.9)
  fires ch 0x48 when Bond walks into the hall → the objective folds to 4 on the mission monitor's
  own tick. Both writers fire through the walked route — **[D]+[M Trigger_Activate 0x1b2318]**.
- **0x4000003 "Spy on secret meeting"** (title 0x400005e) — completes on **ch 0x49**, hidden until
  ch 0x48 [L record]. Its writer is on the NEXT level: **07000008**'s intro plate TouchOnce **#237**
  (139.2, 49.5, −81.2) [D 07000008 placements]. The walk exits via #418 (the game's own exit),
  carries the RamSave + mission states [M 0x1b2318:111-116], walks to #237 on 07000008 (the gondola
  intro NIS skipped with the game's skip action), ch 0x49 fires, the carried record folds to 4.

## The route (all real input)

1. Spawn **#0** (33.4, 44.5, −70.6) [D placements kind 0x24], the fists settle [M Player_InitWeapon
   0x1a7ef8 case 0x7000007]; **#411** fires ch 0x47.
2. The west aisle (35.7,−70.6) → (42.6,−70.5) → the west wall run to (42.6,−45.0) → the north door
   (49.4,−44.7) (door 5 E-opened by the stuck handler) → the north row (62.7,−44.3) → (76.5,−44.9)
   → the hall (83.1,−50.5) → (93.3,−50.4). The group plates fire on the way (ch 6 #476, ch 0x17
   #415 [D census §4]) and the party wakes group by group.
3. The hall's E-door (94.9,−47.2) opened with the use key; into **#412** → **ch 0x48**: the weapon
   lock lifts [M Player_Weapon 0x1a24c8:243-246 — the walk asserts the cycle survives the lock],
   objective 1 completes, and **SP #420** (0x06000073, the control-taking cut-scene) plays and is
   skipped with the game's skip action. Bond hands back at the stream-6 entity's key-498 spot
   (91.52, 43.85, −31.09) [D — the same number fix_cutscene_actors_live pins].
4. The hall nodes back south (95.2,−44.6) → (93.3,−50.4) → (83.1,−50.5) → (83.1,−56.2) → the west
   stair foot (83.1,−61.1) → **up the west stairs** ([D path-0 nodes 41→42→43]:
   (83.1,43.4,−61.1) → (83.2,46.4,−70.2) → (95.6,49.4,−70.6)).
5. The balcony south into **Drake's reveal #356** (box 92.9–98.0, 49.0–53.8, −79.8–−78.9 [D]) →
   **ch 0x15** → **SP #419** (0x06000072, the Drake fly-through, 3000 ticks [M levels.md §5.1])
   plays, skipped with the game's skip action.
6. The exit run — the upper floor splits into four rooms ([D the path-0 nodes + the walk's wedges]):
   the west north balcony (x 92-98) and the west south wing behind door 4 (94.8,−73.2); the east
   north room and the east south wing behind door 6 (132.4,−73.2). The wings meet only on the GROUND
   floor: down the west stairs, east along the north row (105,−55.9) → (111,−61.8) → (126.5,−55.8)
   → the east stair foot (145.1,−61) (doors 1/0 E-opened), up the east stairs ([D nodes 62→50]:
   (144,46.4,−70.2) → (132.9,49.4,−70.5)), south through door 6 (the use key), to the exit
   **LOAD_LEVEL #418** (135.9, 50.1, −76.5), E → `level_exit 0x07000008` [D §5 — gate 0,
   unconditional].
7. The carry into 07000008 → the gondola intro NIS (skipped with the game's own skip action) →
   walk to **#237** → **ch 0x49** → objective 2 completes.

## The user report's two open items

1. **"the guests' and guards' party rules"** — pinned by the zone test + the walk:
   - The guests stand in WaitSwitch groups until their plate fires ([M] `DSTATE_Global` 0x15b830
     ENTER → WaitSwitch on the placement's switch channel; `DSTATE_WaitSwitch` 0x15b960 case 3
     wakes on the channel; the port refreshes `channel_on` from `switch_channels` each tick).
   - The prop-0x1D walkers then take their kind-8 mission routes ([M] PartyGirlInit 0x1728c0 /
     CivilianInit 0x172730 ENTER: prop 0x1D → `AssignAIPath(8)` 0x154190 → CivilianMission
     0x15d860 walks `UpdateMissionRoute` 0x154a80).
   - The guests never attack and are deaf to sound alerts ([M 0x14c938 dtype 10/0xb on 07000007
     writes prop 0x1f = 0, 0x54 = 1; 0x148a30:45-49]); the bouncers attack only an armed, seen Bond
     ([M CivilianGuard 0x1603a8 case 3]) — the walk crosses the whole party unarmed with ZERO
     combat states.
2. **"Mayhew's walk to the exit before the mid cut-scene"** — what the disc data actually holds:
   the six figures standing around the hall (dtype 1, prop 0x4b) are DOOR GUARDS, not walkers.
   [M] GetDTYPENEW 0x14c938 gives dtype 0-9 on 07000007 mode 0x16 when prop 0x4b is set;
   [L] DroneTypeSettings[0x16].start = 0x31 (0x29aee8+0x16·12 = 0x29b0a8 reads 0x0031) and
   [L] StateFuncs[0x31] = CivilianDoorGuard 0x160648 (0x29b300+0x31·4) — a stand-at-the-door state
   (`SetDronePartner(0x16)` + `DoorGuardHandler`). Their standing posts ARE the game's behaviour.
   The walkers that DO walk are the prop-0x1D guests — the zone test walks two of them once their
   plate fires. If the playtest expected a specific Mayhew figure to walk out the exit door before
   the hall scene, the disc's mission layer gives him no such writer: no placement, trigger, mux or
   script event on 07000007 moves a dtype-1 drone ([D the census §7; the kind-8 paths' owners are
   the dtype-10 guests]). A live check below asks for the one thing the port cannot see.

## Blockers hit on the way, and their disposition

- The AI nav graph is per-path and this level's path 0 splits into 8 link components (the guests'
  walk roads) — the walk bridges them with the nodes' own positions. Harness routing, no port
  change: the game's own A* is per-path by design [M walker.rs `nav_route` doc].
- The free E-doors auto-close behind Bond; the walk pulses the use key only on a CLOSED door —
  `Door_Activate` is a TOGGLE ([M the door group pass; door.rs states 2/4/8/0x10]).
- The P2K auto-draw (`Upgrade_Weapon(6)` [M 0x1a24c8:247-250]) is owned by the VIEW loop's frame
  pass (nf-view lib.rs, `switch_channels_prev` semantics) — the walk harness steps the Sim
  directly, so the rising edge is not reproducible there. The unit is pinned by level_carry_live;
  the walk asserts the lock LIFT (the cycle request survives) with Bond staying on the fists —
  also the cover-safe play. No port change.

## What the walk still cannot do (with the exact reasons)

- Watch the cut-scenes unskipped: the harness carries the game's skip action by design (the task
  allows exactly that). The scenes' full playback is pinned elsewhere (cutscene_skip_audio_live
  runs the Kimiko scene's lines to their own end without a skip).
- The Drake fly-through's per-actor guest anims during #419: the scene RUNS through the real
  trigger (the player reaches its end state; the test asserts it), but the per-actor anim/skin
  verification lives in cutscene_models_live / cutscene_faces_live, which pin 0x06000072's skins.

## Live checks for the main agent

None required — no port code changed this task; both deliverable tests pass on the current tree.
One optional check if the "Mayhew's walk" playtest item is still wanted: gdbwatch
`NDrone2_AssignAIPath` (0x154190) and `NDrone2_DSTATE_CivilianDoorGuard` (0x160648) during the real
07 party open — the access type is a call on each hall dtype-1 drone at level start; the expected
result is the DOOR GUARD state 0x31 (never a kind-8 route), confirming the port's reading.

## What I ran

- `cargo test --release --offline -p nf-view --test walkreal_07000007_live`: **2 passed**.
- `cargo test --release --offline -p nf-game`: **509 passed, 0 failed**.
- The sibling 07000007 binaries (`party_start_rules_live`, `level_carry_live`,
  `fix_guest_reactions_live`, `fix_party_mission_live`, `fix_cutscene_actors_live`,
  `cutscene_skip_audio_live`): **34 passed, 0 failed** (1 pre-existing ignore: the sight pass's
  headless gate, documented in that test).
- The fast runner (`tools/test/suite.py`) was started in the first minutes per the task, but this
  sandbox kills every background process between tool calls (suite.out empty; the mp-screens
  WORKLOG entry records the same behaviour). The FULL suite then ran in the foreground on the
  final tree, chunked to fit the sandbox's 10-minute foreground cap:
  - nf-formats + nf-audio: **107 passed, 3 failed — exactly the 3 `disc_golden` tests the task
    allows to fail on the slim ISO** (bigf_header_fields_behave_as_documented,
    bigf_tables_match_the_golden_manifest, directory_tree_matches_the_golden_manifest).
  - nf-game: **509 passed, 0 failed**.
  - nf-view: 39 + 25 + 70 + 45 + 77 = **256 test binaries run, 364+157+238+370 = ~1128 tests
    passed** across the chunks, with **3 failures, all render-probe/pixel tests in this GPU-less
    sandbox**: `fix_0c_pylon_live` (the_0c_pylon_fills_the_view_from_the_closeup_spot,
    the_0c_pylon_is_on_screen_at_the_user_spot) and `probe_render_pylon_only`. My diff is a
    comment in drone.rs + a new test file — it cannot affect a render binary; both failures are
    the environment (no GPU — the task's own rule: visual work needs the main agent's render).
    The remaining ~101 nf-view binaries (the later walkreal_*/render*/spot chunks) did not fit
    the sandbox's remaining window; the new walkreal_07000007_live binary itself ran standalone
    (2/2) and every binary sharing the 07000007 surface ran green.

## Usage

not visible to me (no token, context-window or cost figures are shown in this environment).
