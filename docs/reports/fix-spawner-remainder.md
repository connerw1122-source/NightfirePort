# fix-spawner-remainder — G-025's spawner tail and G-040's "missing guard"

## What was done

### G-025: the "+0x1F wave-delay byte" and the power-station spawn SFX

**The wave-delay byte does not exist.** The decompile settles +0x1F as the *initial-fill /
no-stealth-checks* flag, not a delay: `DroneSpawner_Init` 0x137aa0:33-38 sets it to a fixed 1 at
load iff the trigger channel (p4) is absent-or-already-on AND the stop channel (p5) is
absent-or-off; `DroneSpawner_Control` clears it once, at the first wave rollover (0x137e68:148).
While set it overrides mode 3's nobody-alive rule (0x137e68:184) and skips `DroneSpawner_SpawnDrone`'s
screens (0x137cb0:30/:43 — the min-spot-distance test and the `Drone_PositionVisible` abort). No
frame counter, no seconds, no timer reload, no RNG draw anywhere in the family — the gap between
mode-3 waves is purely the alive-scan == 0 wait. The port already modelled all of this as
`immediate`; the fix is the documentation (actors.md §5, GAPS G-025) plus a census-backed comment
in spawners.rs.

**The power-station stinger is ported** [M `DroneSpawner_Control` 0x137e68:191-200]: on
0x0700000c/0x0700000d only (the double compare on the current level), a mode-3 spawner whose whole
last wave is dead with at least one wave completed runs the branch EVERY tick — `Rand_Rand(2)`
picks `Drone_SfxInfo[0xD6]` or `[0xD7]` (sfx ids 0x549/0x526, 100.0 vol, outer 15/inner 10, from
the table at 0x2812d0 [L ee_snapshot]) and `NDrone2_PlaySFX(1.0, NULL, id, NULL, enforceDelay=1)`
plays it through the entry's 600-frame repeat delay (stamped before the play, [M 0x141a1c]). The
draw sits between the alive scan and the spawn attempt, so the port's RNG order is: stinger draw,
then the selection draw. The first-ever wave is silent. `Spawner::update` sets `sfx_request`
(a new field, own line) and `Drones::tick` drains it through the existing `foley_entry` gate
+ `play3d` path.

**Reachability finding**: the census of every 0xF1 placement on all 07* FILES.BIN levels shows
mode-3 spawners on 08/11/12/13/06/15 — **none on 0x0700000c/0d**, so the branch is unreachable on
the retail disc. No NF_ISO test can fire it end-to-end; the unit tests pin the semantics instead
(what I tried: the full-placement census via the nf-formats parser, all 24 story levels).

**Two real divergences found and fixed along the way** (both in the same spawn path):

1. **The selection order** [M 0x137cb0:21-43]: the min-distance test is a PRE-selection usability
   test — the candidate loop only decrements the `Rand_Rand` draw for templates that pass it, so a
   too-close spot is skipped *without consuming the draw* and the attempt never fails on distance.
   The old port drew over ALL unused templates and aborted after the pick — a different RNG
   sequence whenever any spot was too close. `update` now takes `dist_ok` (pre-selection) and
   `visible` (the post-selection abort, which still fails the attempt and retries).
2. **Wave exhaustion** [M 0x137e68:126-143]: the game's check is the pure counter compare
   `puVar1[3] <= puVar1[0xb]` (max waves vs waves completed) — the port's extra
   `|| used.all()` term fired one tick before the game's rollover and **killed single-template
   multi-wave spawners** (07000012 placement 647: mode 3, max 2, waves 5, 1 template — it died
   after wave 1 instead of respawning). Removed; a full wave rolls over first (which also zeroes
   the templates' spawned-this-epoch flags, [M :145-172]).

### G-040: 07000001's "missing second guard"

**The premise was a misdiagnosis; the guard is not missing on master.** The evidence chain:

- The hash in GAPS/U-049 is a misread: `0x0600007f` exists nowhere on 07000001 (not in the
  archive's 616 entries, the golden TSVs, or the EE snapshot's 874-slot script hashtable); the
  only disc hit is inside 07900002.bin — level 02's pack. The real hashes: placement 277 →
  **0x06000087**, 276 → 0x06000086, 79 → 0x06000016/17 [D].
- Placement 277 is not a guard — it is the class-0xD9 **script player** running the intro
  cut-scene script 0x06000087 (auto-start, delete-on-end). That script is a type-0xC loadable
  streamed from pack **07800001.bin** (`SP_LoadScript` 0x1dbc08 → `isLoadable` 0x1d69a0 →
  `LoadableLoad` 0x1d69e8; the snapshot has the player mid-script [L ee_start_07000001.bin
  0x18fbf70]). There is **no global script archive**: every LoaderLoad call site is enumerated and
  no 06-nibble archive exists on NTSC-U; kind 5 is the *animation*-script engine (AnimLoadFile) —
  a different format, and loading it into the NIS lib would be wrong.
- The port has streamed pack scripts since fix-cutscenes (WORKLOG 0092). The script's stream 8
  writes ch 0xB at its tick-847 event; ch 0xB gates six 0xF drones (the "gate guards") AND the
  mode-2 gate spawner (start ch 0xB) [D the placement census].
- **The new test proves the symptom closed**: `walk_07000001_live::the_second_gate_guard_wakes_on_the_intro_scripts_channel`
  loads 07000001 through `nf_view::load`, watches the intro to its ch-0xB event (tick 2119 in the
  run), and asserts every standing gate guard leaves WAIT_SWITCH and the spawner arms. It extends
  the pre-existing full-route walk test (contact → plate → exit).

Docs: GAPS G-025/G-040 rewritten (G-040 closed as a misdiagnosis), UNKNOWNS U-049 marked resolved
with the corrected hashes, actors.md §5 got the stinger, the +0x1F flag, the selection order and
the wave-exhaustion paragraphs.

## Files

- `crates/nf-game/src/spawners.rs` — the stinger branch, `sfx_request`, the selection split, the
  counter-based exhaustion (the task's named file).
- `crates/nf-game/src/drone.rs` — **shared file**, small additive hooks, listed here: the
  `update` call site's closure split (the same two screen tests, re-plugged) and the
  `sfx_request` drain (one `if let` through `foley_entry`/`play_sound_handle`).
- `crates/nf-game/tests/spawner_kill.rs` — the four new unit tests + the two call-site updates.
- `crates/nf-view/tests/walk_07000001_live.rs` — the G-040 test.
- `docs/GAPS.md`, `docs/UNKNOWNS.md`, `docs/systems/actors.md`, `docs/WORKLOG.md`.

## How it was verified

- `spawner_kill.rs` 9/9: the stinger (draw order `vec![2, 1]`, request 0xD6/0xD7 by the draw
  value, silent first wave, no draws on the rollover tick, no draws while alive>0), the level gate
  (0x05/0x0b/0x0e/0x09-0c never request), the pre-selection distance skip (one draw over ONE
  usable template, the close spot never picked-and-failed), the post-selection visible abort (the
  draw consumed, the slot persists, the retry spawns).
- `walk_07000001_live.rs` 3/3 on the slim ISO (the new G-040 test included).
- Full suite: see the count below; the slim ISO allows the 3 `disc_golden` failures.
- The census probe (`nf-formats/examples/spawner_census.rs`) is scratch — untracked, not committed.

## What's left

- The stinger's sound POSITION: the game passes a NULL position (no DCVars, no drone) — the port
  plays at the group's first template spot [S]. Unreachable on retail, so no live check can settle
  it; it only matters for modded/MP maps.
- `DroneFunc_NewSightTalk` 0x13f420 plays the same 0xD6/0xD7 pair on the same two levels for a
  guard's first-sight line (same gate) — a speech-side twin, not ported (speech tables are
  level-data-driven in the port; noted for the next speech task).
- The modes > 3 question from the research: the decompile's mode arm leaves their gate untouched,
  yet actors.md lists mode-4/5 placements — a disc-side census found NO 0xF1 placement with mode
  > 3, so actors.md §4's mode-4/5 rows describe placement modes that do not exist as spawner modes
  on the disc (they may be dtype start-states, not spawner modes); worth a look when actors.md §4
  is next touched.

## Live checks for the main agent

1. PCSX2, 07000012 (007 Agent), the courtyard: placement 647's spawner (mode 3, max 2, waves 5,
   1 template) — kill the lone guard and watch a replacement arrive in the same spot family
   (pre-fix the port spawned him once and never again).
2. PCSX2, 07000001: the second gate guard at the level's gate stands and reacts after the intro
   cut-scene finishes (ch 0xB), alongside the contact — `--bond -4.9,6.4,-0.1,0.0,0.0` area at the
   level start; the guards are on the gate route.
3. nf-view (after merge): `nf-view 07000001` — after the intro (~35 s of sim ticks), the gate
   guards leave their wait poses.

## ntfy
