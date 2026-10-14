# fix-script-objects — the script-created objects G-031 left (task 289)

Branch `s/fix-script-objects` (from origin/master c5b17c0; merged c606b5f before the final push).
Files: `crates/nf-game/src/{nis,drone,aivolume}.rs`. Shared-file hooks (all additive, listed per
the rules): **drone.rs** — one line in the `sp_gate` literal (`blasts:` from `self.explosions`) +
one `glist_sphere` method on the existing `impl World for Data`; **scene.rs** — the `Sim` field
`glist_bounds` (own line), its build (3 lines where `models` is built), and the additive
pending-volume drain + script-volume matrix refresh right after `self.drones.tick`. New test file
`crates/nf-view/tests/fix_script_objects_live.rs` (NF_ISO, 3/3); probes kept:
`examples/census_script_entities.rs` (the disc census), `probe_urn_sp.rs`, `probe_script_objects.rs`.

## The census (the driver's first ask)

`examples/census_script_entities.rs` decodes every op-7 Entity command (`Script_EntityStart`
0x1d8d78) in every FILES.BIN archive and pack on the disc: **4,985 commands**, by raw obj_type →
`Script_CreateEntity` 0x1d8968's mapped class:

| kind | commands | levels | port state before this task |
|---|---|---|---|
| generic (default arm, mostly 0x31/0x19/0x27/0x2f/0xff/0xe4/0xd9/0xdb) | 4,907 | **all 31** | drawn since fix-render2's script-entity attachments (incl. Corona_2's glist 0x02000426 on 07000005 truck streams 4/5) |
| destroy 0xE1 (`Destroy_Create`) | 26 | 3 (07000008/12/1b) | entity only, no smash behavior |
| break 0x20 (`Break_Create`) | 34 | 2 (07000008/12) | entity only, no kill behavior |
| pickup 0xF0 (`Pickup_Create`) | 9 | 2 (07000003/08) | **already ported** (fix-safe08 `pending_pickups`) |
| trigger_once 0xEA / trigger 0xEB | 4 + 4 | 3 + 2 | **already ported** (WORKLOG 0058–0062 script touch triggers; the truck chain's 0x70/0x71/0x5c plates fire) |
| AI volume 0xF5 (`Drone_AIVolume_Create`) | **1** | 1 (07000005) | **ported this task** |
| ladder 0x22 / switch 0x29 / emitter 0xF2 | **0** | 0 | nothing to port (the disc has none) |

Priority order therefore ran: the generic draw (already done by earlier merges) → SP_Hit flags →
the AI volume arm; break/destroy kill-behavior and the riders did not fit the window (below).

## What was ported

1. **`SP_Hit` 0x1dc470's bit 0x2** — the explosion-hit flag. The port's trigger byte had only the
   channel (1) and activation (4) bits. The game's SP_Update 0x1dbda8:66 calls `SP_Hit` while the
   countdown byte is 0; it walks the current script's stream objects' +0xD0 hit chains and ORs
   0x2 for a class-5 object (an explosion). The port's analog: `SpStart.blasts` carries the alive
   blast records (centre, radius) from `Drones::explosions`, and `Player::update` ORs 0x2 when a
   blast's sphere reaches a stream object's own sphere (obj+0x80 = `Control_BuildWorldSph`
   0x133070 of the glist's local centre +0x10, obj+0x8C the radius, resolved through the new
   `World::glist_sphere` → `Data.models`). Disc case: **07000007's urn SP #312** (script
   0x0600007A, mode 64, mask 2) — the mode-64 prime leaves it in state 1 with the Wait(0) streams'
   shard entities; the blast replays the shatter (state 2, playing, 10 shards, the tick-1
   Sound(1523)). Deviations, listed: the +0xD0 HITDATA chains and the mask==2
   `Collide_FilterBullets(obj+0xD0, 0x106)` arming (0x1dc524-0x1dc534) have no port-side object
   collision chains to feed/arm — the blast list rides the same tick order instead (blast at T's
   end, SP pass reads it at T+1, matching the game's explosion-object update pass); the class-3
   (0x8) and bullet (0x10) chain bits stay unported (no object bullet chains).
2. **Script-created AI volumes** — the 0xF5 arm now does what `Drone_AIVolume_Create` 0x1376c0
   does: the create-path entity (a same-class stream object is re-pointed without a new volume,
   [M 0x1d8a68-0x1d8a80]) pushes `Objects::pending_volumes`; the sim drains them into
   `Sim::volumes` with half-extents |max−min|/2 × the identity scale triple from the glist bounds
   [M 0x1376c0:84-92], factor = param0×0.01 and kind = param1 (0→0/1→2/2→1/4→3 [M 0x1376c0:100-116]),
   and refreshes the matrix from the live stream entity every tick (the game keys obj+0x90 from
   the stream per `Script_Interp`). The create-time z-negate/+1.5y lift is NOT applied [S, in the
   code comment]: the script arm's object matrix is overwritten by the same tick's interp, so the
   lift never reaches the volume box. Disc case: 07000005's truck script 0x06000A22 stream 2
   (glist 0x0200040C, params [0]) — the census's only script-created volume, registered by the
   placed SP #37's mode-64 prime on the first tick.

## How it was verified

`fix_script_objects_live.rs`, each on a real disc script through `nf_view::load` + `step_bond`:

- `the_urn_sp_fires_when_a_blast_reaches_the_shards` — 07000007: the prime's state-1/mask-2
  asserts, the Wait(0) shard entities exist, the radius-3 blast at the SP base (49.26, 43.01,
  −53.46 [D the placement dump]) flips the SP to state 2 playing, the replay recreates the
  shards (10 live; stream 11's ends at its own EntityEnd), the Sound(1523) queues.
- `a_far_blast_never_fires_the_urn_sp` — sister: a blast 30 units east leaves it in state 1, no
  sound.
- `the_truck_script_creates_its_ai_volume` — 07000005: exactly one volume with
  src (0x06000A22, 2), placement usize::MAX, kind 0, factor 0.0, non-zero half-extents from the
  glist bounds, matrix tracking the live entity, and the volume count grew.

Fail-first: with the four source files stashed the test file does not COMPILE (`no method
entity_matrix`, `no field src` on the pre-fix tree) — the passes cannot happen there; restored,
3/3 pass. Suite on the merged tree (origin/master c606b5f): **fix_script_objects_live 3/3,
fix_script_commands_live 4/4, fix_truck_live 2/2, nf-game lib 145/145** (all `--test-threads=1`,
`--release`). The four-crate sweep did not fit this session's window again (suite.py could not
survive the background — the nohup'd run died on a PATH-less env and the rebuild after the merge
consumed the rest); the cover above is every family the touched code paths reach: the SP state
machine (truck = the SP-heavy chain test), the script runtime (fix-script-commands' streams), and
the whole nf-game unit set. NO RNG draws added; no wall-clock in Sim.

## What's left (the ordered tail)

- **Break (0x20) / destroy (0xE1) kill behavior** — `Break_Update` 0x12bfd8's per-tick
  `Collide_FilterBullets(obj+0xD0, inst[2]|0x106)`, the health accumulator (`level+0x30`), the
  class-2 distance damage (level-0700001 only, inst[0]==2), the class-5 damage from hitdata+8,
  and `Break_Kill` 0x12b970 (debris + the glist undraw); `Explode_Propagate`'s port already has
  the 0x20/0x21 cases but `Targets.breaks/destroys` are fed `&[]` at both call sites. The
  placement-created breakables (07000007's 8 kind-0x20 urn placements at the origin, glist-baked
  positions) need the same `WorldObjects` arm aivolume got.
- **Drone riders** — `Control_InheritVelocity` for guards standing on moving script objects (the
  player's ride is ported; the drone-side inherit is not).
- **The players' place in the object list order** — not investigated this session (time).
- **Corona_2** — the coronas draw through fix-render2's attachment path; the open visual question
  (below) is whether their alpha-list blend treatment matches the placement coronas'.

## Live checks for the main agent

PCSX2 (real game):
1. Spot: 07000007, the urn at (49.26, 43.01, −53.46) (the fountain room's vase cluster). Action:
   stand clear and shoot the floor next to it with the grenade launcher / let a guest-triggered
   blast go off near it. Expected: the urn shatters once (shards fly, sound 1523's cue) and
   cannot re-trigger from a second distant blast — the SP's script A runs 23 ticks then stops
   (state 1 → 2 → 3 on the next mask match? mask still 2 and a second blast while state 2
   re-arms the replay: `1`/`3` retriggers are the mask's own semantics, worth a gdbwatch on
   SCRIPTPLAYER+0x24 state transitions).
2. gdbwatch (optional): SP #312's placement record +0x2C (the activation u16) — expected 1 on
   the blast tick, cleared next tick.
3. Spot: 07000005, the gatehouse yard after the truck parks. Expected: unchanged — the AI volume
   is invisible; a guard's search of the truck-bed area should behave as before (the volume's
   factor is 0.0 — whether the game's `Drone_ProcessOpponents` treats factor 0 as "never visible
   here" or skips the box is U-045 material; the port applies factor×visibility directly).

nf-view (`--bond` spots; the render decides the visual questions my headless tests cannot):
4. `--bond 45,42,-50` on 07000007 with a blast forced near the urns (or the dev grenade): the
   shard glists 0x020004e5–020004ef should fly outward along their key paths and STAY (event 0xC
   keep-objects at tick 20).
5. `--bond 100,26,108` on 07000005 through the truck ride: the truck's headlight coronas (glist
   0x02000426) — confirm they draw with the alpha treatment (additive glow, not solid mesh)
   while the truck drives.
