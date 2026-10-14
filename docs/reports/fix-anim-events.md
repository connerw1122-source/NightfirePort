# fix-anim-events (s/fix-anim-events, Task 288, G-030)

The animation scripts' events with effects: census the disc's event opcodes, port the missing
`AnimProcessScriptCmds` (0x1c1b38) / `DroneAnim_EventFunc` (0x13b750) handling, test per event
kind on real scripts.

## What's done

**Census (complete, 0 parse failures).** All 99 `.bin` archives in FILES.BIN parsed
(`disc::files_bin_entries` + `LevelBin`; 33 carry kind-5 scripts): **18,231 scripts, 50,279
commands, 1,258 distinct hashcodes**. Only opcodes **0 / 1 / 4** exist — op 2 and op 3 never
occur, so no port work hides there. Op-4 kinds: 0 (34) / 4 (6,925) / 5 (6,976) footsteps,
1 player toggle (52), **2 = `DroneAnim_EventFunc` (12,947)**, 3 stop-sfx (53; ids 18/61/1479
mirror the op-1 plays), 6 effect (1,294), 7 fire bullet (4 — one script, 07000016/06000858).
Kind-2 args run 0–16 and 18; **the fire flags 0xF/0x10/0x12 (args 15/16/18) are 86% of the
traffic** (11,192 commands). Full tables: the census scratch was /tmp (not committed); the
summary lives in docs/systems/anim-engine.md.

**Ported (all in crates/nf-game/src/drone.rs — the task's named file; no other source file
touched).**

1. `DroneAnim_EventFunc` (0x13b750) cases 2, 5, 9, 0xB, plus the game's gates:
   - **2 → `DroneWeap_ChangeWeapon(drone+0x45)`** — the disassembly (0x13b7dc) settles what the
     decompile dropped: `a1 = lb 0x45(a2)` sits in the dispatch's shared delay slot, so the
     weapon comes from the **drone's** `+0x45` byte (DefaultInit 0x14b300:266 = setup word
     0x17), not the event's argument. `DroneWeap_ChangeWeapon` (0x17a6e0) is a 55-entry
     per-weapon jump table at 0x2fb740; the port carries the two entries the SP states verify
     (1 = Surrender_Anim 0x1626a0's weapon 1 / set 0xC / empty, 6 = DrawWeapon 0x166000 EXIT's
     weapon 6 / set 4 / 16 rounds); other entries change nothing yet.
   - **5 / 10 / 0xB → `DroneWeap_CloseCombatImpact`** (0x1789c0): `Bullet_init(-1, obj, obj,
     record, obj+0x30, facing, 0xffff)` — record 99 at the 2.0 range for the weapon-class-7
     kick, else 100 / 0x65 (type 1) / 0x66 (type 2) at 1.75 with the `|drone+0x1D4| ≤ 60°`
     facing gate. No accuracy roll, no muzzle light, no sound — straight from the record.
   - **9 → `DroneWeap_PlaceTertiary`** (0x17a848), which the decompile shows is a tail call to
     `DroneWeap_ThrowGrenade` — the event throws the grenade.
   - **Gates**: the game gates cases 1 / 5 / 6 / 9 / 0xB / 0xE on `0.0 < drone+0xAC` and
     `script == *(drone+0x530)`. The port's `script_commands` only ever runs the current
     script, so the script gate holds structurally; the health gate is now explicit
     (`alive` in `event_func`).
2. **`DroneAnim_PlayFiringAnim` (0x13da48) inside `handle_firing`** — the +0x20/+0x21 flags are
   read at last: for the recoil weapons (`+0x3E`, which `DefaultInit` 0x14b300:801-811 sets
   when the anim datum's weapon byte is 0x1C/0x1D; the port keys it on the drone's weapon id),
   a due shot fires **only** when the anim's 0x12 event set `+0x21` (with the game's
   `+0x558 == 0` / `+0x538 == 0` pending-call / blend-out gates); the shot then plays the 0x5C
   recoil danim (`SetDAnimInternal(0x5c, 0, 0)`), zeroes the pending call's completion action
   (+0x596 / +0x598) and sets +0x57B. Non-recoil weapons fire exactly as before (the game's
   `+0x3E == 0` early-out), so no live firing path changed — the guards carry 0x16 / 1 / 6.
3. New `Drone` fields (own lines, merge lesson 4): `weapon_swap` (+0x45), `pending_melee`,
   `pending_weapon_swap`, `recoil_weapon` (+0x3E); drained in `Drones::tick`.

**Tests** (`crates/nf-game/tests/anim_events_live.rs`, NF_ISO opt-in, one per ported kind on a
real disc script, driven through the real path `Drones::tick` → `update_anim` →
`script_commands` → `event_func` → the drains):

- `melee_event_creates_the_close_combat_bullet` — 07000005/060002ba's arg-10 event (frame 47):
  the punch connects (one `player_hits` from that guard) at dist 1.5, and **nothing** at 3.0
  (the sister: outside the 1.75 range).
- `weapon_change_event_swaps_by_the_setup_byte` — 07000001/06000330's arg-2 event: byte 6 →
  weapon 6 / anim set 4 / clip 16; byte 3 (an unported table entry) → no change (sister).
- `fire_flag_gates_the_recoil_weapon_and_plays_the_recoil` — 07000005/0600029f's arg-18 (0x12)
  event sets both `+0x20`/`+0x21` through the real script (the fire-flag plumbing).
- `tertiary_event_throws_the_grenade` — 07000012/060007ac's arg-9 event: the grenade spawns,
  the count drops 2 → 1, the owner is the guard.
- `play_firing_anim_gates_the_recoil_weapon_shot` — **`#[ignore]`**, see "What's left".

Suite: baseline master f512bf0 **856 passed, 5 failed, 26 ignored (22 min)** — the 3
`disc_golden` failures allowed on this ISO plus 2 `movie_playback_flow` failures that already
fail on untouched master. Final tree (merged with origin/master 086009a): see the total line
below. `cargo check` clean; no new RNG draws (the melee/grenade/swap paths draw only where the
game does — the grenade's existing `Rand(3)` stun roll stays mode-gated).

## What's left

- **op-4 kind 1** (52 cmds): toggles the player SMI's `+0x964` (`Player_SetWeaponAnim` 0x1a9338
  sets it from `weapon_data+0x68 & 8`; `Player_LaserPointer` 0x1a65d8 reads it) — the port's
  player anim objects don't run script commands at all yet, so there is no hook.
- **op-4 kind 6** (1,294 cmds): `Effect_Create(1.0, *(obj+0xdc), obj, 0xc0)` (0x1c1b38:406-410);
  the port has no general effect system (only the bullet-impact effects in effect.rs).
- **op-4 kind 7** (4 cmds): the anim-driven `Bullet_init` + the muzzle `AnimDatumSetEntity`
  flash + `Light_Create` + `Gas_Init` smoke (0x1c1b38:411-457).
- **EventFunc 0xE**: `Drone_SM_SendMsg(0xB, 10, …)` to the `+0x2B0` partner's SMI +0x108 — the
  port has no partner (drone→drone) messaging; msg 0xB has no `mod msg` name yet.
- **The other 53 `ChangeWeapon` jump-table entries** (0x2fb740), and the **setup-word read of
  `+0x45`** (0x14b300:266 reads setup `[0x17]`; the port's `weapon_swap` defaults 0 and is
  test-settable — wiring it needs the nf-formats setup tables, outside this task's files).
- **`play_firing_anim_gates_the_recoil_weapon_shot`** (ignored): the gate itself is implemented
  in `handle_firing` (`fire_now`), but the harness's neutral-state isolation does not reach the
  fire block — with Bond 1.5 away and `firing`/`burst`/`next_shot`/`class` forced, the shot
  never becomes due inside the tick, so some state-machine or ordering piece still resets the
  setup. The flag-set half of the same script passes through the real path.
- **Footprint decals**: kinds 0/4/5 fire the footstep sound (ported earlier); the decal side
  (`ScriptFootstepFoot` / `ScriptFootstepType` → `AnimObjectUpdate` 0x1befd8's
  `Effect_Create(floor hit, 0x42 | foot)`) needs a decal system the port doesn't have.

## Live checks for the main agent

1. **`drone+0xDA` (the kick gate)** [M decomp says class 7 = kick; [S] it equals the port's
   `weapon` id]: gdbwatch `ACTION.ELF` drone+0xDA on a 07000005 guard during a melee swap-in —
   expect the weapon id (0x16), never 7, so the kick record 99 stays unreachable in SP.
2. **`drone+0x45` (the ChangeWeapon index)**: fieldprobe a 07000001 guard playing 06000330 —
   expect the byte to be 6 for the pistol swap (the test assumes entry 6 = weapon 6/set 4/16).
3. **The recoil weapons**: which weapons are 0x1C/0x1D (USATxt.dat names) and does any SP enemy
   carry one — if yes, their fire timing is now anim-event-driven and the
   `--bond` spots below should show shots on the anim's muzzle frame.
4. **nf-view renders**: `--bond` at a 07000005 guard mid-melee (the punch should connect when
   Bond stands inside 1.75 in front of him) and a 07000012 tertiary-throw guard (the grenade
   leaves on the anim's release frame).

## Suite (final tree, merged with origin/master 086009a)

**871 passed, 3 failed, 27 ignored (23 min)** — the 3 failures are exactly the allowed
`disc_golden` set on this ISO (baseline master f512bf0 was 856/5/26; its 2
`movie_playback_flow` failures are gone on the merged tree, and the anim-events work
introduced none).
