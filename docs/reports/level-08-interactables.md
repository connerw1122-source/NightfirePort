# level-08-interactables — every interactive thing and every model on 07000008

Branch `s/level-08-interactables` (from origin/master d628f01f). Task 708a. All work through the
real input path (`nf_view::load` + `Sim::step_bond` walking/turning/use/fire).

## What's done

**1. The census** (`docs/levels/07000008.md` §7, new). A disc probe
(`crates/nf-view/tests/census_07000008.rs --ignored`) dumps all 308 main-map placements by kind
with params/positions/hashes, plus a second `--ignored` dump of the Hurt volumes' world triangles.
The §7 table lists every touch/use/shoot/ride kind with the game's law [M + addresses] and the
port's status per kind. Headline: the level's interactive set is doors (26), glass (3),
ladders (2), safe locks (2), the copter, the fusebox family, the zip line + hand-over-hand,
destroyables incl. the SmallExplosionBox (#234, the alternate ch-137 writer), 2 SimpleScripts,
**3 Hurt volumes**, 14 triggers, 1 LoadPointScript, 4 muxes, 8+2 pickups, 5 spawners,
6 ScriptPlayers, 2 sound triggers, 1 music trigger, 2 hint plates. Two stale rows fixed: the
census's older "the gadget is not ported" note is wrong — the laser watch/decryptor burn the safe
bolts through the real path (`gadget_laser_live`, 7 tests); the 0x20 glass kind is bullet-broken
on 08 (`rocket_glass08_live`).

**2. The Hurt kind (0xE4) — implemented** (`crates/nf-game/src/hurt.rs`, spec
docs/systems/hurt-volumes.md carried from the level-06 work; every field cites the decompile).
07000008's #42/#43/#44 are three 2-triangle quads at **y −24.10** tiling the valley floor
(x −55.8..286.1, z −260.4..−110.9 [D the collision dumps]) — type 6 (fall), 1000 damage, no gate,
who 0x18 (classes 3+2: drones AND the player), one-shot 0 [D params]: **the level's kill floor,
LIVE at load**. `Hurt_Create` 0x17e870's field map, `Hurt_Update` 0x17e5d0's who-mask law (gate →
the channel value; no gate + who 0 → starts at once; no gate + who ≠ 0 → `who & {8, 0x10, 2}`
collected — the who-1 inert law), the anim clock, the one-shot retirement, and the damage pass
(Bond through `Drones::health.pain_generic`, a drone through the explosion path's flat subtract)
are ported and wired into the sim tick right after `Trigger_Calc`. Unit tests pin the row law,
the who-mask law (incl. the inert who-1), the touch/stand separation and the one-shot retirement.

**3. Models**: **4486 materials / 562 textures / 0 white fallbacks** on 07000008
(`models_sweep_07000008_live`, the scene material table — the same condition `NF_TEX_TRACE=1`
prints); all 9 placed drones carry their skins (0x05000012/13/14/2e/2f/30/3b [D oDrone]);
`models_world_live`'s golden row pins 07000008 at 0 unresolved meshes. Nothing to fix.

**4. The real-path finding (the open items)**: walking Bond off the gondola boarding platform
(x 111.4, z −127.2, yaw π — the real input, the walk-off in `hurt_07000008_live`) reproduces the
fall: he descends past the lip, the plunge TouchOnce #54 fires (ch 0x7C ✓, the game's F5 law),
and then **the death NIS (SP #307, 0x06000991, `Script_IsDeathNIS` [M nis.rs]) holds the body
forever at (111.40, 17.56, −136.89)** — the port has no death after a death NIS (the game fails
the mission: Bond's death → ch 0x62 → MissionState 3 [M 0x182c60:21-43]), so the body never
reaches the y −24.1 kill floor. The hold is pinned by the live test (it asserts the hold —
the symptom the next task fixes). The other three open items are cross-level or already-covered:
the guards breaking the glass at the start is 07000001's Mayhew-estate entry (fixed on the
unmerged branch `s/glass-breach-guards`, level-gated to 01 by `Break_Update`'s own law
[M 0x12bfd8:23-34]); the safe/briefcase chain is pinned green (`pickups08_live` 5,
`fix_safe08_live` 5, `gadget_laser_live` 7); the rocket blast lighting (GAPS G-049, the 08 blast
light behind the pane) is the main agent's render lane — the travelling rocket light itself is
merged (merge-log 2026-10-16 11:47).

## How it was verified

- `nf-game --lib`: **264 passed, 0 failed** (incl. the 4 new `hurt.rs` unit tests).
- The whole 08 family, NF_ISO on: `walk_07000008_live` 3, `pickups08_live` 5,
  `fix_safe08_live` 5, `rocket_glass08_live` 3, `gadget_laser_live` 7, `fix_fusebox_cut_live` 4,
  `hurt_07000008_live` 2 (new), `models_sweep_07000008_live` 2 (new), `models_world_live` 3 —
  **all passed, 0 failed**.
- `census_07000008.rs --ignored` (assertion-free dumps) produced the §7 table and the hurt
  geometry above.
- The fast runner (`tools/test/suite.py`, started in the background at 14:12 UTC) was still
  running at report time; its two `FAILED fix_0c_pylon_live …` lines are the same family as the
  documented under-load timing flakes (merge-log 2026-10-16 16:01). **No final total line yet** —
  the foreground runs above stand in; the totals line is owed to the next message.

## What's left

- **The death-NIS terminal fail** (the new finding above): after a death NIS ends, the game fails
  the mission (the ch 0x62 → MissionState 3 path [M 0x182c60:21-43]); the port freezes instead.
  Fixing it makes `hurt_07000008_live`'s kill-floor assertion (currently documented in the test
  header) activatable: the fall then reaches the kill floor and the 1000 type-6 lands.
- The 08 per-instance door E-open + the #122-after-ch-137 unlock pair (the kind's path is pinned
  by the doors family; the instances are not), the 08 zip-line ride (#19), the spawner-wave
  releases end-to-end (the 42/43/44 → waves chain).
- **Zoe shooting at Bond at the gondola (750r)**: the fix exists on the unmerged branch
  `s/zoe-gondola-08b` (the team-2 opponent gates [M FindOpponent 0x142e0c] + the PLAY_SCRIPT
  sound arms [M 0x15ba68]); its own report records the one remaining conflict — the escort test
  fails with those arms because the courtyard guards hold PLAY_SCRIPT; the next decode is
  `NDrone2_NearArmedDrone` 0x177420's candidate flags and/or DSTATE_PlayScript's msg-3 `+0xcdc`
  latch (07000001's scripts 0x6000168/0x6000169 zero it in case 1). Master still has the revert,
  so on master Zoe targets Bond.
- `s/glass-breach-guards` (01's breach) and `s/zoe-gondola-08b` are both done-and-unmerged; the
  queue's merge lane should pick them up.

## Live checks for the main agent

1. PCSX2 fresh boot of 07000008: walk Bond off the gondola boarding platform's south edge.
   Expected: the fall plays the death cut-scene and the mission FAILS (the death NIS ends the
   level in a fail state). This settles whether the death-NIS end is the fail writer, or the
   body continues to the kill floor after the cut-scene (the port currently holds the body
   forever).
2. The same boot: confirm the lower station (#8's start, (118.7, 35.0, −113.2)) is walkable
   terrain — the port wedges Bond's walk input there entirely (position frozen, grounded).

## Usage

not visible to me
