# level-06-interactables — every interactive thing and every model on 07000006

Branch `s/level-06-interactables` (from origin/master d628f01f). Task 706a. All work through the
real input path (`nf_view::load`/`load_with_carry` + `Sim::step_bond` walking/turning/use/fire).

## What's done

**1. The census** (`docs/levels/07000006.md`, rewritten). A new probe
(`crates/nf-view/tests/census_07000006.rs --ignored`) dumps all 813 main-map placements by kind
with params/positions/hashes; the doc's §5 table lists every touch/use/shoot/ride kind with the
game's law [M + addresses] and the port's status. Headline corrections to the previous doc rows:

- **FuseBox #747's script is 0x0600004B** (the old rows and alarm-panel-06 said 0x0600006B — the
  disc says 0x0600004B), params 57/55/103/55: group-0 cut → ch 57 (searchlights off) + ch 103;
  group-1 cut → ch 55 (the alarm) + ch 55, and no live-wire hurt because the hurt gate (+0x56 =
  param 2) is 55 ≠ 0 [M FuseBox_Create 0x1b0448 / FuseBox_Update 0x1adfe8:99-123].
- **The well's Hurt_A #373 is GAME-INERT** [M Hurt_Update 0x17e5d0 + D #373]: the state-0 contact
  scan collects only the class bits 8/0x10/2 and its who-mask (param 4) is 1 — `1 & collected == 0`
  always, so the 1000-damage type-6 volume can never start on the disc as shipped. The shaft's
  player hazard is the plain fall path (Player_HandlePain type 6 [M]). Full kind spec written
  docs-first for the implementer: `docs/systems/hurt-volumes.md` (the create field map, the state
  machine, the damage-consumer question, two live checks for the main agent).

**2. Per-KIND real-input live tests** — new files, all green:

- `fusebox_06_live.rs` (4): the Laser (0x4E, switch_to input) burns the group-0 bundle → ch 57 +
  ch 103 one-shot, the wire's entity removed, no hurt; the group-1 bundle → ch 55 (the alarm
  channel), still no hurt (+0x56 = 55); sisters: aim-without-fire keeps every channel low, and E
  presses do nothing [M Player_Activate 0x1a5170 has no class-0x3C arm].
- `interact_07000006_live.rs` (9): the E-door #47 opens on the use key (+ no-E sister); the locked
  #369 refuses until ch 200 is carried, then opens (the pair through the real use key); the radio
  SP #807 is the mode-8 ambient SP — its script pair [0x06000055, 0x06000056] PLAYS FROM LOAD and
  it is NOT E-usable (mask 2 fails Player_Activate's mask-&-4 gate [M 0x1dc338; port from_placement
  / useobj.rs]), with the mode-0 differential sister (#810 holds no script until its trigger); the
  body armour #755 is taken by walking over it (+ 5 m sister); the well pair (the mouth is open,
  the measured ~5 m drop survivable; beside the well nothing happens).
- `models_sweep_07000006_live.rs` (2): **5940 materials / 447 textures / 0 white fallbacks** —
  every material resolves a texture on 07000006; every live drone carries its skin
  (0x0500002e/2f/30/35 [D oDrone p0]). Level-wide mesh resolution is models_world_live's.

Already-covered kinds pinned by earlier 06 tasks and re-run green here: the starts/objective/
LoadPoint/dressing-exit chain (walk_07000006_live), the alarm-panel chain #765/#749/#702 +
the searchlight ch-55 raise/latch (alarm_panel_06_live), the dressing cut-scene timeline
(cutscene_06_live), the searchlight beam continuity (searchlight_flip_07000006_live), the driver's
state law (fix_06_driver_live), the 07000008/14 fusebox system (fix_fusebox_cut_live), NPC door
navigation (npc_doors_live, fix_npc_doors_live), pickups (pickup_live). The generic systems the
census marks ✓ re-ran green: doors_live/doors_01_live, simplescript_live, explode_live,
spawner_kill, worldobj_live, cover_nodes_live, fix_context_hints_live, textures_live.

**3. Models**: no missing hash, no white fallback (the sweep above); the level's meshes resolve
through the disc hash table [D census probe]. Nothing to fix.

## How it was verified

- The full four-crate build (`cargo test --offline --release -j2 --no-run`): 500 test binaries,
  all linked (this took three attempts — see Suite below).
- Targeted fast-runner chunk (`tools/test/suite.py --only …`, per-binary 600 s caps, NF_SILENT):
  **nf_game, nf_formats + the whole 06 family + the new files → 377 passed, 0 failed, 1 ignored**;
  **nf_audio, nf_view + the census-claimed generic systems → 59 passed, 0 failed, 2 ignored**.
- Every game claim in the census carries its [M]/[D]/[L] tag; the two findings that change play
  (the inert well volume; the ambient radio) are decompile-read with addresses, not inferred.

## What's left

- **The Hurt (0xE4) kind is still unimplemented** — with the crucial nuance that 07000006's only
  placement is game-inert (the who-mask law above), so implementing it changes nothing on THIS
  level. The spec (`docs/systems/hurt-volumes.md`) is ready; levels that configure who = 0x10
  (if any) are where it becomes visible — a grep over the disc's 0xE4 placements by param 4 would
  find them.
- **Live checks for the main agent** (also in the spec): (1) PCSX2 fresh boot, drop Bond down the
  well — measure the real shaft depth and the post-landing health (the port's shaft floor is ~5 m
  below the lip and the drop is survivable; is the real one a kill?); (2) the `Hurt_GetDamage`
  0x17ea18 caller during any hurt-volume touch, to settle the damage-application point.
- **"Rook stuck in the doorway" (user report ×2)**: Rook is 07000014's elevator-jumper boss
  (docs/systems/actors.md), not a 07000006 actor — the driver's item list may carry the level
  label over from the queue. On 07000006 the doorway NPCs are the driver #494 + escort #450 (both
  wait ch 185 at the truck park); their scare/door behavior is pinned by fix_06_driver_live and
  npc_doors_live/fix_npc_doors_live, all green here. I did not reproduce a doorway stall in the
  port this session; if the playtest meant 07000014's Rook, that is the map-tower2 line's
  territory. Render ask: `--bond 60.0,41.0,26.0,2.2,0` (the truck park) — the driver and escort
  should stand clear of the archway, not inside its plane.
- **The full suite on the final tree**: not completed in this sandbox — see below.

## Suite

The background fast runner was started twice (`nohup … suite.py`) and was killed by the sandbox
between tool calls both times (empty `suite.out`; the same reaper every prior sandbox hit). The
foreground chunks above are the substitute. Mid-session the sandbox's disk hit 100% (944 K free),
which presented as rustc/link failures at -j4 AND -j2; clearing the npm/bun/agent-browser caches
freed ~1.1 GB and every binary then built and ran. The final tree == the pushed tree (no merge of
origin/master happened during the session; nothing upstream moved it). The full-suite total line
is owed to the next message.

## Usage

not visible to me
