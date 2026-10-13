# fix-spawner-p7 — the spawner's param 7 is the KILL channel (live-measured)

Branch `s/fix-spawner-p7`, based on 32af1c6, master merged (0cad1d9) before the push. Task 046b.

## What was done

The port's `DroneSpawner_Control` (0x137e68) now matches the game as the 07000005 live measurement
(docs/fields/hitdata-spawner-ally.md "Live result") and the decompile + disasm say:

- **p7 (+0x14) is the kill channel**: `ch[p7]` latches the byte at +0x20 (:50-52); the latched path skips
  the trigger wait and the whole spawn machinery and runs the cleanup, never spawning again.
  fix-actors' "ARM channel" reading is refuted; the original triggers.md reading stands, refined below.
- **The cleanup's level split** (disasm 0x137fbc-0x1380b4; `s7` through the delay slot at 0x137fe8):
  kill-all for level < 0x07000011, 0x07000014/15, and > 0x07000016 except 0x0700004A — every live spawned
  drone removed (`obj+0xFE |= 1`, slot zeroed) and the spawner deletes itself in the same pass. Keep
  levels (0x07000011..13, 0x07000016, 0x0700004A) remove only drones whose `obj+0xFC` byte is 0 and linger
  while any drawn drone remains, deleting themselves once none remain. On 0x07000016 each removal first
  raises the drone's death channel (`jal 0x13e660` at 0x138074). This split is what explains fix-actors'
  07000011 playtest: the stairwell guards were drawn when the channel fired, so the game kept them.
- **`obj+0xFC` identified** (the brief asked for every writer): the renderer sets it to 2 for every object
  added to the draw list (`View_AddObjects` 0x1e62d8:71, `View_AddForcedObjects` 0x1e5fb8:174); the
  per-frame object pass decrements it while non-zero (`control_movement_object_handler` 0x1336f8:239-247,
  core-loop item 11). So 2 = drawn this frame, 1 = drawn last frame, 0 = not drawn for 2+ frames. (Bullets
  0x12d890 and cars 0x130060 use the same offset as a float timer — a per-class field.) The port's
  per-drone `drawn` visibility flag stands in, **snapshotted at control-walk start**: the game's writer is
  outside the control walk, so the scan must see last frame's value (a spawned guard whose per-drone pass
  runs before his spawner's would otherwise be judged on a same-walk recompute).
- **The early-outs ported in Control's order**: `NPCGlobals == 0` (the port's `npc_enabled` at the call
  site), `Drone_bDisableSystem` 0x30ca9e (new `npc_system_disabled`, wired but always false — the writers
  are the options-menu "drones off" setting `switch_NO_DRONES` 0x30ce2c via `Drone_PauseNoDraw` 0x13a710 ←
  `Drone_InitComms` 0x139520:287 and `ResetMap_GameInit` 0x1d5080:14; the port has no options menu), the
  spawner's own `obj+0xFE` bit 0 (the port's `gone`), and the block's mode < 2 / group == 0 / slot pointer
  checks (:38-46). The stop channel un-triggers and pauses a latched spawner for the tick, deletes an
  un-latched one (:77-84). The done path waits for the spawned to die when p6 OR p7 is set, then sets
  `ch[p6]` and deletes (:126-143).
- **The decompiler trap**: the cleanup loop's comma expression (`sVar8 = sVar7 + 1, *(char *)(iVar2 + 0xfc)
  == 0`) reads as if the linger count included removed drones; the disasm settles it — the increment is the
  delay slot at 0x138064 and `s6` only updates at 0x138098 when the drawn-`bnez` is taken, so `s6` is the
  KEPT count. The port follows the disasm.

## How it was verified

- **Reproduce before fix**: with the fix stashed (master's code), the same 07000005 flow through the real
  input path left both gate guards alive after ch 0x43 — "SYMPTOM: ch 0x43 removed nothing — master's arm
  reading leaves both guards alive" (assert left: 2, right: 0). The failure and its message are quoted in
  the WORKLOG entry.
- **`nf-game/tests/spawner_kill.rs`** (5 unit tests, render-independent): the kill-all removal + self-delete
  + never-spawns-again; the keep-level split (only the undrawn removed, the spawner lingers, then deletes
  itself once the survivor goes undrawn); the full level-split table; the stop-channel pause/delete split;
  the done-path wait (p6 OR p7). All cite the decompile/disasm lines.
- **`nf-view/tests/spawner_kill_live.rs`** (4 NF_ISO tests through `step_bond` and the disc's touch
  volumes): the 07000005 live-measurement replay (ch 0x22 → the two gate guards; ch 0x43 → SP 298's script
  0x06000AD4 freezes the NPCs → both removed, the spawner gone, never spawns again), the sister (no ch 0x43
  → they stay), the NPCGlobals gate (a fired kill channel does nothing while the NPCs are frozen and lands
  on re-enable), and the 07000011 stairwell (the guard arrives on trigger touch #182; kill touch #285 keeps
  the DRAWN guard [pre-fix-actors this is where he died]; once undrawn the next scan removes him and the
  spawner deletes itself).
- **`fix_actors_live.rs::the_spawner_arrives_on_its_channels`** re-driven through the trigger (touch #182,
  ch 0x15) — the arrival is the playtest symptom; the final assert now reads `sp.kill == 0x1d && !sp.gone`.
- **Full suite** (NF_ISO, post-merge): **109 test binaries green; only the 3 allowed `disc_golden`
  failures** (the slim ISO).

## Test re-baselines the corrected semantics forced (documented, not silent)

1. **The 07000005 wall-hug window**: the port's script player runs its first commands one tick after
   `start` (nis.rs — a pre-existing deviation: the game's walk runs SP #298, placement 298, before the
   spawner, placement 268, in the same frame, so the game's freeze lands within the touch frame and holds
   the latch off through the whole script [L]). In the port the removal therefore lands on the touch tick
   and the freeze one tick later; the end state matches the measurement, and the freeze-window latch hold
   is pinned separately by the NPCGlobals-gate test. A same-frame SP start would close the gap — nis.rs is
   outside this task's file list, so it is documented here instead.
2. **`fix_finale_live.rs`'s sister**: the launch chain's ch 0x15 is the group-1 spawner's kill channel and
   07000016 is a keep level, so the chain's own cleanup removes its undrawn spawns before the sister runs —
   her old fixed-position throw at a frozen pit-squad spawn no longer has a target. She now re-reads the
   nearest remaining non-chaser's position per throw (the test's own pattern for the walking chaser); the
   assertions are unchanged: a non-chaser corpse, and ch 0x90 stays off [D AND #271].

## Files

Task files: `crates/nf-game/src/spawners.rs` (the rewrite), `docs/systems/actors.md` §5 (corrected),
`docs/systems/triggers.md` (the spawner section rewritten). Tests: `crates/nf-game/tests/spawner_kill.rs`,
`crates/nf-view/tests/spawner_kill_live.rs` (new).

Shared files touched (minimal, listed per the rules):
- `crates/nf-game/src/drone.rs` — the call site (level id + the walk-start `drawn` snapshot + the
  bDisableSystem gate), the 0x07000016 death-channel hook in the Remove handler, the `npc_system_disabled`
  field, and one comment.
- `crates/nf-view/tests/fix_actors_live.rs` — the header bullet's p7 correction + the re-driven arrival
  test (`sp.arm` no longer exists; the old flow asserted the refuted arm semantics).
- `crates/nf-view/tests/fix_finale_live.rs` — the sister re-baseline above.
- `docs/GAPS.md` — G-025's one-line correction (p7 = the kill channel, live-measured).
- `docs/fields/hitdata-spawner-ally.md` — appended the porting note + the obj+0xFC finding (no rewrite of
  the existing sections).
- `docs/WORKLOG.md` — the fix-spawner-p7 entry appended at the end.

## What's left (not this task)

- The port's script player start timing (the one-tick gap above) — a nis.rs task; closing it makes the
  07000005 wall-hug window match the game frame-for-frame.
- The options menu's "drones off" setting (would drive `npc_system_disabled`; the gate is ready).
- The mode-3 wave-delay byte (+0x1F) semantics beyond Init's immediate flag, and the power-station spawn
  SFX branch (0x0700000c/0d) — pre-existing G-025 remainders, unchanged.

## Live checks for the main agent

Nothing blocking — the headless tests pin the behaviour end-to-end. Optional confirmations, all on the
GDB-server PCSX2 (the same harness as the 07000005 measurement):

1. **The keep-level lingering** (07000011, the stairwell): after ch 0x1D fires with a guard drawn, watch
   the spawner's data block (+0x34 slot array): each pass zeroes only slots whose drone reads `obj+0xFC ==
   0` (watchpoint: `lbu` at 0x13805c), the block stays allocated while any drawn drone remains, and the
   spawner's `obj+0xFE` gains bit 0 only after the last slot clears. Expected: a guard on screen survives
   the touch; walking out of his view for 2+ frames then re-firing the scan removes him.
2. **0x07000016's death-channel raise**: same level, chain ch 0x15 (the group-1 spawner pl254): each
   removed drone's `switch_channels[Drone+0x13C]` write (`DroneFunc_SetDeathChannel` 0x13e660) lands before
   its `obj+0xFE |= 1`. Expected: the group-1 spawns' death channels (23..26 for the pit squad) pulse as
   they are removed — and the mission's AND #271 does NOT complete (0x90 stays 0).
3. **Drone_bDisableSystem**: set the options-menu "drones off" (or poke `switch_NO_DRONES` 0x30ce2c = 1),
   load any level with spawners, and watch `DroneSpawner_Create` 0x1378d0 — no spawner objects are created
   (`control_create_object` 0x3c never runs), and a pre-existing spawner's `Control` returns at 0x137e68:24.
