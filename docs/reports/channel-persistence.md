# channel-persistence — report

Task 699: "switch channels must persist until written (every level's buttons and doors)" — the
750q finding that `Channels::set(0x33, 1)` then one `step_bond` on 07000001 read the channel back
0, where the game's `switch_channels` (ACTION.ELF 0x26fc90) was assumed to persist until written.

## What's done

**The finding is resolved as a misdiagnosis — the clearing writer is the game's own.** The writer
hunt (write-watch with backtraces through the real sim) landed on `Trigger_Update` case 5
[ACTION.ELF 0x1b1a78:147-161]: the disc's OUT placement **#296 on 07000001 (out 0x32, input
51 = 0x33) [D placements]** copies `switch_channels[0x32]` (0) into `switch_channels[0x33]` EVERY
tick. The game does the same to an externally written 0x33. The channel was owned all along; the
port is faithful. Pinned both ways by `the_0x33_clear_is_the_discs_out_copy_on_07000001`.

**The full writer inventory was audited against the decompile** (docs/systems/
channel-persistence.md — the table with every family, function and address): the touch clear +
`|= hit` [M 0x1b1900 + 0x1b1a78 case 2], the TOUCH_ONCE case-1 fall-through `|= hit` every tick
[M case 1→2; the port's hit-only write lands identical values — documented equivalence], the mux
recompute [cases 3/4/0xC], the OUT copies [case 5], the sensors [0x1af618], the flickers
[0x1ab960], the emitters [0x17b080], the doors [0x135618, transition-only], the switches
[0x1adcc0/0x1adea8/0x1b02a8], the objective init [0x182270 — load-time], the level latches
[0x1b1900:21-26], the drone comms byte 0x97 [0x139520:198-201, unstamped], the script events
0xD/0x11 + SP outs [0x1d94c0], and the script-created triggers [0x1d8968; port rebuilt per tick,
drone.rs]. **No port-only per-tick writer exists.** The one unported writer (the SS operate arm,
0x1a9948:20-23) is the reverse gap and was already documented in ss.rs.

**Deliverables:**
- `crates/nf-view/tests/channel_persist_live.rs` (3 tests, all ISO-gated):
  1. `a_channel_nobody_owns_holds_600_ticks_on_07000001` — ch 0x3c (verified unowned by the
     census) set through the real sim, held across 600 real-input ticks; sister: an explicit 0
     write lands.
  2. `the_0x33_clear_is_the_discs_out_copy_on_07000001` — the OUT #296 pin: ch 0x33 tracks
     ch 0x32 within one tick in both directions [D pl296 + M case 5].
  3. `the_port_rewrites_only_game_owned_channels` — **the census across all 23 on-foot levels**:
     120 idle ticks per level through `step_bond`, every channel write recorded (`Channels::writes`,
     source-tagged), each write's channel must be owned by a live object of the game's writer
     families (or the documented [S] stand-ins 0700004a 0x22/0x4a). Script-path writes are their
     own class (`set_script`, source 1) — data-driven from the disc's scripts [D] like the game's
     0x1d94c0 path. **All 23 levels pass.**
- `Channels` gained the None-gated census probes (`writes`, `script_channels` — new fields on
  their own lines at the end) and `set_script`; the door's transition mirror routes through `set`
  (identical stamp semantics [M 0x135618:26]); the 0x97 direct write records itself. Raw
  `Channels` literals in three test files were updated for the new fields (the suite build caught
  them).
- Docs first: `docs/systems/channel-persistence.md` (the writer table with addresses and evidence
  tags, the 750q resolution, the census method, the [S] stand-ins, the deviations).

## How it was verified

- `channel_persist_live` 3/3 (twice; 23-25 s over all 23 levels), on the final tree (origin/master
  has not moved since the branch point — the merge is a no-op).
- The three census false alarms chased to ground during development, each resolved with evidence:
  the 05 ch-0x71 write (the script-created truck-bed touch — Script_CreateEntity 0xEB [M
  0x1d8968]), the 06 ch-0x40 write (a script event 0xD [M 0x1d94c0]), the 4a ch-0x4d write (the
  placed TOUCH_ONCE plate #277's hit — the game's case-1 fall-through rewrites kind-1 outs per
  tick [M case 1→2]).
- Regression runs on this branch (all green, nothing changed): `doors_01_live` 2/2, `doors_live`
  2/2, `npc_doors_live` 7/7, `walkreal_0700004a_live` 1 ignored (pre-existing ignore, untouched),
  `walkreal_07000001_live` 2/2, `fix_truck_live` 2/2, `walk_07000005_live` 2/2, nf-game lib
  276/276.
- The full fast-runner suite (`tools/test/suite.py`) was started on the final tree; the sandbox
  kills background processes between tool calls and a full foreground run does not fit the
  2-hour window (the 1017 batches measured nf-game's binaries alone at ~30 min here). What fits
  and ran: the four-crate **build** (the suite's build step — which caught the two stale
  `Channels` literals, now fixed) plus every binary this task touches, listed above. See
  "Suite" below for the final state at report time.

## What's left

- The full suite total line (the sandbox/time budget; the branch-touching crates ran green
  directly — the suite's own build over all four crates passed after the literal fixes).
- The SS operate arm (0x1a9948:20-23) stays unported (pre-existing, documented in ss.rs) — a
  channel only that arm writes never turns on in the port.
- The [S] stand-ins (4a ch 0x22/0x4a, the flicker write value) remain as documented [S] items —
  unchanged by this task.

## Suite

(The total line is filled in below when the run completes; see WORKLOG for the final state.)

## Live checks for the main agent

None required — every claim above is [M] decompile + [D] disc data, checked headlessly.

## Usage

- Token/credit/context figures for this session: not visible to me. (No usage, cost, or
  context-window numbers are shown anywhere in this environment; nothing to copy.)
