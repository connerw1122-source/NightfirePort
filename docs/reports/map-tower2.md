# map-tower2 — the second tower missions, mapped and made completable

Branch `s/map-tower2`. The four levels 07000011 (Tower2 A), 07000012 (Tower2 B), 07000013 (Tower2 C) and
07000014 (Tower2 Elevator) are mapped from the game's own data in `docs/levels/07000011.md` …
`07000014.md`, and every one of them is walkthrough-tested end to end in the port:
`crates/nf-view/tests/walk_070000{11,12,13,14}_live.rs` (11 tests, all green; opt-in via NF_ISO).

I built on fix-mission's mission layer (the task's instruction): my branch carries it cherry-picked, and
master has since merged it — my branch merged origin/master cleanly mid-session.

## Per level: completable?

| Level | Completable | The end |
|---|---|---|
| 07000011 | **yes** | E at the Load Point #158 loads 07000012; the atrium fall is the scripted death (already ported, now pinned by a test) |
| 07000012 | **yes** | the override console (ch 0x49) → the shaft touch holds 0x1F → the gated exit loads 0700004A (the lobby connector, part 2 of the mission — not one of my four, but mapped and walked through) |
| 07000013 | **yes** | reached warm through the chain; the front entrance completes the mission → `LevelToEndTo` 0x07300013 (the driving interlude — requested faithfully, not loadable: DRIVING.ELF is out of scope) |
| 07000014 | **yes** | the five-stage chain (vent cut-scene → Kiko → the fusebox sabotage → the server-room escape → Rook) → the exit loads 07000015 |

## What I fixed (each shown failing first, each citing the game address)

1. **The mission globals ride the RamSave** (`player.rs`, `drone.rs`, `lib.rs` — additive). The warm chain
   07000012 → 0700004A → 07000013 dropped previous parts' objective states, so the success AND could never
   see them. The game keeps them in `.data` across `ResetMap_LevelToLoad` and `Mission_Init` 0x182270 only
   resets the current and future parts in place [M 0x182270:68-129]; the port now carries a `MissionSave`
   (the LS_MakeMission/LS_LoadMission 0x1cd720/0x1cd880 payload shape) and restores it after the init latch.
2. **The fuseboxes** (`fusebox.rs` new; hooked in `drone.rs`). The 07000014 sabotage had no writer:
   `FuseBox_Create` 0x1b0448, `FuseBox_Register` 0x1b0970 and `FuseBox_Update` 0x1adfe8 ported — the two
   wire groups, both channel words written when a group empties, and the live-wire 5.0 hurt through the
   game's `Player_Hurt` path. Stand-in (noted in the doc): the wires are one per group, cut with E after
   the lid press — the SP-entity bullet path is not ported.
3. **The SimpleScript open/closed swap** (`ss.rs` new; `level.rs` 3 lines; `scene.rs` additive) — the named
   07000012 item. The disc evidence: every pair member's key 0 equals its placement transform and the pairs
   are complementary (07000012's 0x19 blocks 12–20, decoded key by key). `SS_Create` 0x1a9c58 creates the
   animated ones; `SS_Update` 0x1a9948 poses them via `KeyFrame_Interp` 0x1e1140. The port now runs the
   playhead: key 0 at rest, the trigger channel (0x58/0x59 — the lift call buttons) runs it to the last key
   and back. The lift leaves slide 0.86 apart on ch 0x59 (verified by test); the animated 0xE2s left the
   static pass (no double draw); the key-less ones keep the old static draw.
4. **The CPU pose path** (`scene.rs`): the merge surfaced a regression — ss-indexed door slots dropped to a
   far-away pose on the CPU path (`gpu_skin_live` caught it); it now falls through to `drones.sss.list`.

## What I couldn't port (with the game function behind each)

- **The driving hand-off**: 07000013's exits target 0x07300013 (DRIVING.ELF; `ResetMap_LevelToLoad`). The
  port requests it faithfully; the level itself cannot load (PLAN §5 phase 4).
- **The fusebox wire sub-objects**: `FuseBox_Update` 0x1adfe8's bullet-filter path on the SP-created wire
  entities — modelled as one abstract wire per group (the channels, the group order and the hurt are exact).
- **The Q-Worm computers** (07000011's "Computer to hack", 07000012's Q-Worm): `PCQWorm_Activate` 0x1aee20
  unported — bonus props, not on the completion path.
- **The fake-lift chain on 07000013** (ch 0x23 → ScriptPlayer #161 + Shutters2): **no writer exists on the
  disc** [D — every trigger out, kind-5/7 script stream, door/pickup/drone param checked]; dead data, left.
- **07000013's exit gate 0x4E**: no writer on the disc [D] — the E8 works through the E path (the game's E
  arm falls through to the load with the gate off [M 0x1b2318 case 10]), which the port does.
- Cut-scene cameras/anims play through the port's script player (channel chains verified end to end);
  pixel-level checks need a GPU (none in this sandbox).

## Spots the user should check in play (`--bond x,y,z,yaw`)

- 07000011: the atrium fall from (2, 46, 30) — the death cut-scene's camera; the stairwell descent
  (--bond 14.6, 46.5, -4, yaw π).
- 07000012: the lift lobby call buttons (-1.55, 0.5, 7.2, yaw 0) — press E: the doors should slide and the
  screens swap; the override console (-1.6, 0.5, 7.2 → the wall panel at (-1.55, 0.9, 8.2)).
- 07000013: the front entrance run (-6.4, -368, 66) — the objective + mission-complete banners.
- 07000014: the west generator room fuseboxes (--bond -88.7, -2.2, 30.9, yaw -π/2 and -88.7, -2.2, 49.5) —
  two presses each, the second one zaps for 5 HP; then Rook's arena (20.3, 0.9, -23.5).

## Verification

- Full suite green except the 3 allowed `disc_golden` failures on the slim ISO (bigf/directory goldens).
- Shared-file edits, all additive and listed: `player.rs` (+1 field +1 builder), `drone.rs` (+2 fields,
  the attach/tick/use hooks), `lib.rs` (+2 module lines +2 carry lines), `level.rs` (+3-line filter),
  `scene.rs` (the ss draw block + the two matrix-upload extensions). No shared file was reordered or
  reformatted.
