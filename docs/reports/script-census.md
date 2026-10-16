# Report: script-census — the scripts and events the port doesn't handle yet

Task 709, branch `s/script-census`. The user asked "are there things like scripts and whatnot
that we haven't found yet?" — this task measured it. Deliverable:
**`docs/systems/script-census.md`** (the census) + `crates/nf-formats/tests/script_census.rs`
(the pinning test). Both pushed.

## What's done

Every 07* archive on the disc was enumerated and parsed: 34 level archives (07000001-07000999,
front end 07000048 included), 27 loadable pack archives (078/079/07A), 38 per-level anim
archives (07F). Per archive the census counts every NIS script, its command opcodes and event
kinds, every anim script (kind 5) and its opcodes, every trigger placement and action code,
every dynamic-object placement type, and every archive entry kind. Each item is classified
handled (port handler cited) or unhandled (game handler cited [M] with what it does), and the
doc ranks the unhandled by occurrences with the top 10 written up as ready-to-queue tasks.

Headline findings:

- **NIS scripts: nothing unhandled hides in the opcode space.** 468 scripts, 5,263 streams,
  227,544 commands; all 20 opcodes that occur are handled. The only unhandled NIS surface on
  the entire disc is **three event commands** (kinds 0x08 coder-drone spawn, 0x09 camera mode,
  0x0a breakable break — one each, all in script 0600005b of 07000001).
- **Triggers: one unhandled action.** 676 trigger placements over 8 action codes; the port
  lacks only action 0xd (movie player, `Trigger_MoviePlayer` 0x1b2a08) — exactly 1 record,
  in 07000041. 33 of the 36 raw 0xDC triggers are action-0 (inert by design, [M] zeroed param
  buffer).
- **Placements: the real gap is AI + volumes.** 7,372 dynamic placements across 62 types; 1,399
  records sit on 14 types the game handles and the port does not — cover nodes (0xE5/0xE6,
  968 records, every mission level), AI points (0xE9, 162), hurt volumes (0xE4, 64), sensors
  (0x28, 62), second start-slot (0x2D, 51), GT objects (0x27, 41), music triggers (0xFB, 15),
  lightning (0x2C, 15), plus 8 single-digit families. Another 138 records on 4 one-per-level
  marker types (0xDD/0xF6/0xF7/0xFF) and 165 flags-0xFFFFFFFF records are skipped by the game
  itself — not port debt.
- **Anim scripts (the biggest family): 18,231 scripts / 50,279 commands, ops 0/1/4 all
  handled.** Op-4's kind sub-switch replays kinds 0/4/5/6; kinds 1/2/3/7 (13,056 commands) are
  not replayed — kind 2 goes through animobj+0x70, which is zeroed for player scripts
  [M 0x1c1600:51], so its live reach needs the gdbwatch check listed in the doc.
- **Entry kinds: 13 seen, all recognized by the game** (`LoaderProcess` 0x1d4880). The port's
  runtime handles 11: kind 8 (menu defs, 34 — one per level) is parsed only by test helpers
  (menus are transcribed constants), and kind 15 (memcard icon, 1) is absent outright.

## How it was verified

- The census parser (scratch Python, not committed) mirrors the port's own parsers
  byte-for-byte; it reproduces **three independent golden totals exactly**: 5,263 NIS streams
  (nis.rs:120-125's census), 23,329 placement records (placement.rs:3), 2,230 map entries
  (placement.rs:37). Zero decode truncations, zero unknown NIS opcodes, zero placement
  overruns disc-wide.
- The game-side opcode/event tables were verified against the raw ELF jump tables (28-entry
  `Script_Run` table at 0x3056d0, 17-entry `Script_EventHandler` table at 0x305740), so "no
  unhandled X exists" claims are about complete spaces, not decompiler output.
- `crates/nf-formats/tests/script_census.rs` re-runs the census through the port's own parsers
  (levelbin/map/placement/nis/anim) and pins the handled sets (20 NIS ops, 14 event kinds, 7
  trigger actions, 56 placement types) and every histogram. **6/6 green on the final tree.**
  A regression or a newly handled opcode now fails a test with a number diff.

## What I ran

- Full fast-runner suite started 18:16 UTC on the pre-census tree (`NF_ISO` set, merged base =
  origin/master + WORKLOG): build 435 binaries in 435 s; result line appended below when the
  run completed. The final tree's only delta is docs + the new test file (no lib code), and
  the new test binary is green on the final tree.
- `cargo test -p nf-formats --test script_census` (6/6) on the final tree.

## What's left

- Nothing blocking. Follow-ups listed in the doc's "Not censused" section: per-SimpleScript
  state census, DRIVING.ELF-side surfaces, EE-side script invocations that bypass level data,
  text-label resolution.
- The top-10 queue-ready task descriptions (AI cover nodes first — 968 placements) are in the
  doc, each with the game address to port and a verify step.

## Live checks for the main agent

1. gdbwatch a drone anim object's **+0x70** (word, read) during walk anims: 0 for player
   scripts [M 0x1c1600:51]; non-zero for drones would make the 12,947 kind-2 anim callbacks
   live code paths.
2. Render 07000041 at the **0xF4 movie trigger** volume with its input channel armed: the real
   game pushes movie states 6/7; the port currently does nothing (action 0xd unhandled).
3. Render the **0600005b cut-scene** in 07000001: the real game spawns the gadget drone
   (event 0x08) and overrides the camera (event 0x09); the port drops both.

## Suite result

Fast runner (`tools/test/suite.py`, NF_ISO set) on the merged base tree (build 435 binaries in
435 s, then 46 min of tests):

```
1894 passed, 6 failed, 72 ignored  (46 min)
```

The 6 failures are the allowed set on the slim ISO: 3 `disc_golden` (full-ISO goldens) and the
3 pre-existing `probe_pylon_render` probes. The final tree adds only the census doc + the
pinning test (no library code); `script_census` 6/6 green on that tree.

## ntfy

sent to https://ntfy.sh/nightfire-yoda-8k3x at the finish step (curl, first fallback) —
"script-census: done - census of all 07* archives: NIS ops fully handled; gaps = AI cover/waypoints, hurt/sensor volumes, 1 movie trigger, 3 train-level events".
