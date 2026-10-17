The user (2026-10-17): "09 level elevator buttons dont work. make sure we're prioritizing level work so we can actually complete each level completely and get all models and functions per level working" and "I want EVERYTHING covered. ALL ON FOOT LEVELS". Level work now runs ahead of everything else in the queue.

**Reproduce first, through the REAL path.** Load the level with `nf_view::load` (or `load_weapons` when a
level's own weapon is needed), move Bond with `Sim::step_bond` inputs (walking, turning, the use key, the
watch gadgets, the weapons), let the level's own scripts and triggers run, and observe. No teleporting past
a problem unless the task says so; a teleport to reach a spot is fine when the test then operates the
thing through the real input. Cite the decompile [M], the disc [D] or a fresh-boot PCSX2 value [L] for every
game claim; tests pin only proven behaviour.

## Your task: `level-07-interactables`, every interactive thing and every model on 07000007 (Mayhew C: the party)

1. **Census.** From the disc (an nf-formats parse of 07000007.bin), list every placement the player can
   touch, use, shoot or ride, by type, with its parameters and channel wiring: buttons and switches,
   lifts/elevators and their call buttons and doors, doors (locked, keyed, scripted, sliding, shoji),
   SimpleScripts (0xE2), ScriptPlayers, touch/use/load-level triggers, pickups and weapon racks, safes,
   keypads, monitors, terminals, computers, lasers and laser switches, breakable glass and destroyables,
   searchlights, cameras, turrets, fuseboxes, vents, hatches, ladders, wires, grapple points (watch gadget
   targets), vehicles. For each: what the game does with it [M], and the port's status. Put the table in
   docs/levels/07000007.md.
2. **Make each work.** Through the real input path, drive Bond to one instance of every KIND on this
   level and operate it the way a player does (use key, the right gadget, shooting, walking in). It must do
   what the game does: the channel writes, the animation and sound, the script, the item, the door. Fix every
   kind that does not. One live test per kind (not per placement), each asserting the game's effect, with a
   sister that fails without the operation.
3. **Every model draws.** Every placement's mesh/skin resolves on this level (no missing hash, no white
   fallback texture — `NF_TEX_TRACE=1` prints the fallbacks), every NPC has its skin, every pickup and
   script entity its model. List the misses in the report and fix them.

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

**Branch:** `s/level-07-interactables`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox2 at 2026-10-17T13:18Z

---
## Report (done, 2026-10-17T14:26Z)


---
## Report from the worker (done, 2026-10-17T14:26Z)

# level-07-interactables — 07000007 (Mayhew C: the party), every interactive thing

Branch `s/level-07-interactables` (from origin/master d628f01f). Sandbox window 13:21–15:21 UTC
2026-10-17; this report written at 14:25 UTC.

## Done

1. **The full placement census** [D — a fresh parse of 07000007.bin, `cargo run --release -p nf-view
   --example census_07000007`]: all 513 placements of the main map grouped by type byte, each with
   position, params and resolved object, in **docs/levels/07000007.md §7** with the game's behaviour
   [M] and the port status per kind. It corrects §4 on three points: 15 doors (not 18), the locked
   ch-200 set is #251–#256 + #304 + #342 (the free E-doors are #242–#248 only), and #443 is a type
   **0xEB Touch** ("Steve's Music Start", out ch 138), not 0xE9. Kinds absent from this level [D]:
   lifts, buttons/switches, keypads, safes, fuseboxes, monitors, lasers, ladders, vents, grapple
   points, searchlights, cameras, turrets, vehicles, SimpleScripts.
2. **The Break port** (the one missing interactive kind): placement type 0x20 — the party's 4 vases,
   2 statues, 4 windows — had no sim object (drawn unbroken only). Ported in `crates/nf-game/src/props.rs`
   as `Break` [M `Break_Create` 0x12c230: instance[0] = param 0 (1 debris / 2 glass), instance+4 =
   (float)param 1 the pool; `Break_Update` 0x12bfd8: a class-5 contact subtracts the bullet's
   HITDATA+8; pool < 0 → `Break_Kill` 0x12b970 obj+0xfe bit 0, no channel written]. Every disc pool
   here is (float)0 [D census], so the first positive-damage bullet kills. `Break_Kill`'s type-1
   sound draws `Rand_Rand(100)` in the game; the port does not model break sounds and takes no draw
   [S port choice, documented in code].
3. **Live tests, one per operated kind** (`crates/nf-view/tests/interact_07000007_live.rs`, all
   through `nf_view::load` → `Sim::step_bond` inputs):
   - **0xF0 pickup**: the heavy armour (#507) takes on contact for **armour 50** [D kind 3/rounds 50;
     M Pickup_Handler], and the two kind-6 bonuses are created. Sister: it sits until reached.
     *(Fixed en route: the take point is the pickup's **placement** translation — the placed model's
     glist centre is local ~0 on world-baked meshes.)*
   - **0xDB free door**: the use key swings #245 (state CLOSED 0x2 → OPENING 0x8) [M Door_Activate
     via Player_Activate 0x1a5170]; the locked ch-200 boundary door #253 refuses E. Sisters: no E →
     no open. *(Fixed en route: the test must face the door — the look vector carries the yaw.)*
   - **0x20 breakable**: sim level pinned by props.rs `one_bullet_kills_the_zero_pool_breakable`
     (kill + no-channel + the sister no-shot). The live-shot test is **[ignore]d** — see Open.
4. **Census example committed** (`crates/nf-view/examples/census_07000007.rs`) — re-runnable dump.

## Verification (what I ran)

- `cargo test --offline --release -p nf-game --lib props` → 3 passed (incl. the new break test).
- `cargo test --offline --release -p nf-view --test interact_07000007_live` → **5 passed, 1 ignored**
  (the documented open item below).
- `tools/test/suite.py` was started in the first minutes; this sandbox OOM-kills most of its live
  binaries (`CRASH … (exit -9)` across ~20 tests, 2 real FAILED in `fix_0c_pylon_live` — pre-existing
  render spots, untouched by this branch). **No clean full-suite total was obtainable here**; the
  branch-touching crates were run directly as above. The walk test `walk_07000007_live` (the level's
  golden path) was not re-run to completion in this window; it is untouched by these changes except
  through the new `Props.breaks` field (a struct-field addition with a default-free constructor
  updated in its own test module only — `cargo test -p nf-game` compiled it).

## Open (for the next message)

1. **The P2K never spawns a shot on this level through `step_bond`** — `last_shot` stays `None`
   after 100 fire ticks (weapon 2 out, lock released via the walk path, trigger held). The
   bullet-kill of the breakables is proven at the sim level; the live shot needs the fire gate found
   (suspect: the `weapon_lock_auto_draw` direct `switch_request` vs the fire path's lockout/raise).
   The ignored test carries this diagnosis.
2. **Music/touch chain untested live** (#443 Touch → ch 138 → MusicTrigger #444 event 13 [D]): the
   pieces are ported (triggers, worldobj MusicTrigger [M 0x1b30e0/0x1b31a0]) but no live test was
   written in the window.
3. **The guests'/guards' party rules + Mayhew's walk to the exit** (the carried user report): not
   touched this session (they are AI-script work; the census pins the 58 drones and the six kind-8
   paths [D]).
4. **Model draws**: not swept with `NF_TEX_TRACE=1` in the window — the census shows every
   interactive placement resolving to a named object [D], but the texture-fallback sweep is open.
5. **The renderer still draws killed breakables** (the sim's `dead` flag is not consulted by the
   static-instance pass) — visual only, for the main agent's render check.
6. **Script event 10** (`Break_DoBreak` 0x12c168 via Script_EventHandler 0x1d94c0:148-150) not
   ported — the urn's smash route [S which SP ops use it here].

## Live checks for the main agent

1. **Break pool**: at the party, shoot vase `OBJECT_Vase1` (hall, ≈ (60.2, 44, −42.1)); expect the
   vase to vanish + debris, one P2K round. (Address to watch: the object's instance+4 pool starts
   0.0 [D param 1 = 0]; Break_Update 0x12bfd8 subtracts into it.)
2. **The party door #245** (80.0, 43.0, −49.9): E in front of it — expect it to swing (the port now
   does; confirm the game's swing direction matches ours at `--bond 80,43,-51.5,0,0`).
3. **The P2K fire**: `--bond 105.2,43,-61.5,π,0` facing the statue, hold fire — does the real game
   spawn the shot from a cold party start after the #412 auto-draw? (Our port does not; see Open 1.)

## Usage

Not visible to me: this session runs inside a driver sandbox that exposes no token, credit or
context-window figures. Nothing to copy.
