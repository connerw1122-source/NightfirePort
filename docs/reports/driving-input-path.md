# Report: driving-input-path (G-004, U-003)

Branch `s/driving-input-path`, cut from origin/master cf4f33cd. Task: decode the driving levels'
input path from DRIVING.ELF, document it, port it as a pure function module with bit-exact tests,
and map the port's keyboard+mouse bindings onto it (driving levels stay unwired).

## What's done

1. **The decode** (docs/systems/driving-input.md, every fact [M]-cited to function+address,
   arithmetic cross-checked in the disassembly):
   - `PS2PadDevice::PollDevice` = FUN_00169288 (our build matches the Raikaru shipped-ELF address;
     the DRIVING.SYM addresses themselves remain from an older build, confirmed absent again).
   - The device vtable is 8-byte-stride (adjustment, function) at 0x374880 with the vptr at
     object+0x18 [M the vt+0x20/vt+0x24 call pattern in FUN_00167bc8]; slots 2/3/4/5 identified
     (register-32-values / poll / value-count=32 / HasChanged).
   - The 32-value message: sticks through `b·0x3c008081f − 1.0` (0x3c008081 = exactly f32(2/255)),
     Y negated; the all-sticks-zero guard zeroes only the axes; dead zone `FUN_00167c48(−0.3, 0.3)`
     = strict `lo < v < hi → 0` with **no rescale**; pressure bytes `min(1, b/192)` [0x3baaaaab];
     16 digital bits; digital-mode (mode==4) remap of the DButton floats over the AButton slots.
   - The action layer: the InputTable records (0x34 bytes: kind, name, four buffer pointers), the
     binding evaluator FUN_00168678 (types 0–9 with the change gate; auto-repeat with the strict
     `last + delay < now` clock and its state kept in message buffers C/D), the seven condition
     helpers with their exact thresholds (0.5 edges, −0.5 axis release, ±0.2 centre = 0x3e4ccccd),
     `SendGameMessage` FUN_00165850 (ids 1..0x40, the 0x1f FIRESECONDARY suppression behind
     DAT_00339f54), and the action-id pairs table at 0x32c098 (STEER=1 … RESUMEAUTODRIVE=0x40).
   - **The answer to G-004's threshold/scale question: different.** On-foot uses the byte-space
     dead zone (threshold 40, scale 127/87, input.md §2); driving uses a float-space ±0.3 chop
     with no re-stretch. **No ramps exist in the driving input path** [M-negative] — the only
     time-based behaviour is the type-8/9 auto-repeat clock.
2. **The port**: `crates/nf-game/src/input/driving.rs` — pure module: `stick_to_f32`,
   `pressure_to_f32`, `chop`, `poll_device` (the bit-exact 32-value message, order pinned), the
   seven condition helpers, `Binding`/`evaluate_value` (types 0–5, 8/9; 6/7 take the global flag
   as a parameter since the game's `*(u32*)(0x3c8780+0x20)` semantics are unpinned), the
   `ActionMsg` send with the game's id-range check, the action-id constants, and a `KeyboardMouse`
   → `RawPadSlot` mapping.
3. **Keyboard+mouse mapping** [S, clearly marked]: W/S → LY 0/255 (throttle/brake through the Y
   negation), A/D → LX 0/255 (steer), Space → Cross (handbrake), mouse buttons → △/○ (primary/
   secondary fire), arrows → the D-pad bits, R → Square (look back). The shipped bindings live in
   `data\Control\Master.def`/`FrontEnd.def` inside the mission VIVs — not extractable from this
   sandbox's slim ISO (zero-byte DRIVING extents), so these are the port's bindings, not decoded
   ones. A driving level would call `poll_device` on its slot each tick where the engine runs
   FUN_00168280; per the task the driving levels themselves are not wired.

## How it was verified

- 11 unit tests on the decompiled arithmetic: golden tables from the formulas, the exact bit
  patterns (0x3c008081, 0x3baaaaab, 0xbe99999a/0x3e99999a, 0x3e4ccccd), the digital remap order,
  the all-sticks-zero guard, the strict edges, and sister assertions on every threshold
  (`nf_game` lib: **181 passed / 0 failed**, the 11 inside).
- Full four-crate suite on the final tree (this branch = origin/master + 3 commits):
  nf-formats 64/0 (+ integration: driving_golden **4 skips** — it prints "skip" and passes
  without the DRIVING/ disc folder, which this sandbox cannot extract; weapon_live 1/0;
  pss_movies_live 3/0; exact_vs_host_probe 3/0; facial_probe 1/0), nf-audio 9/0, nf-view 8/0,
  nf-game integration 23 binaries 99/0. **Total: 373 passed / 3 failed / 1 ignored — the 3
  failures are exactly the allowed slim-ISO `disc_golden` trio.**
- Caveat: this sandbox kills background jobs between tool calls, so `tools/test/suite.py` under
  nohup could not survive; the same binaries ran as foreground chunks with the runner's
  per-binary `--test-threads=1` semantics reproduced by scripts/run_suite_chunks.py (kept out of
  the repo). NF_ISO pointed at Nightfire-onfoot.iso throughout.

## Shared files touched (minimal, additive)

- `crates/nf-game/src/input.rs`: +5 lines — one `pub mod driving;` with a doc line. Nothing else.
- `docs/WORKLOG.md`: the `driving-input-path` entry appended at the end (twice: started/final),
  no existing lines touched.
- `docs/GAPS.md`: the G-004 row rewritten (closed); `docs/UNKNOWNS.md`: the U-003 row updated
  (partial: input-path names transfer by shape; the SYM build itself still unmatched).
- No `drone.rs`/`scene.rs`/`player.rs`/`lib.rs` changes; no new struct fields on shared
  one-line constructors.

## What's left

- The shipped binding DATA (which value index → which GAMEACTION, which types/thresholds) parses
  from Master.def/FrontEnd.def inside the mission .VIVs — needs the DRIVING disc folder (U-003
  residue, listed in driving-input.md §11 with the .tun `PAD_DEAD_ZONE` default as a follow-up).
- The IOP pad-fill function behind `FUN_0025a4a8` (the 0x419840 slot structs) and the USB
  keyboard/mouse device (`FUN_00259668` case table) are shape-level only.
- The type-6/7 global flag `*(u32*)(0x3c8780+0x20)` and the `DAT_00339f54` FIRESECONDARY gate
  semantics.

## Live checks for the main agent

The port is pure/unwired, so no `--bond` render spots are needed. PCSX2 checks that would close
the residue, if the driving levels are ever booted (needs the retail image's DRIVING data):

1. Watch address 0x419840 (the pad slot struct, 0x20 bytes) while pressing inputs in a driving
   level: expect the button u16 at +0x0c to match the on-foot bit layout (0x0040 = ✕ held), the
   four stick bytes +0x10..0x13 to move with the left/right sticks, and the mode word at +0x00 to
   read 4 in digital mode / not-4 in analog.
2. Break on `FUN_00165850` (0x165850) and confirm the `{action, device, value}` triples after a
   steer/gas press — expect action 1 (STEER) with value = the dead-zoned ALX, and 0x16 (GAS)
   while LY is held up.
3. The `PAD_DEAD_ZONE` default: read the loaded value of the "Physics:Physical" tuning key
   (search the .tun parser's table for the key hash) and compare against the hard-coded ±0.3 chop
   at 0x16931c — if the tuning value exists and differs, the chop may read it at runtime.

## ntfy

(sent after this commit — see the WORKLOG; on failure the exact error goes here.)
