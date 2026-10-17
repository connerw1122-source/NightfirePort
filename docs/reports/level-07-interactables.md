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

1. **The P2K never spawns a shot on this level through `step_bond`** — measured: pre-fire weapon 2,
   clip 7, spare 41, switch_state 3, idle script 0x0600082b; after 100 fire ticks the fire script IS
   selected (view_script 0x06000825) but **view_time stays 0.0** — the view anim never advances, the
   release frame never arrives, `last_shot` stays None. The bullet-kill of the breakables is proven
   at the sim level; next session: why the view anim freezes (the draw never completing from
   `weapon_lock_auto_draw`'s direct switch_request, or the P2K fp seq missing from this archive —
   "36 weapon-model hashes still missing" in the mesh merge log). The ignored test carries the
   diagnosis and the DIAG prints.
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
