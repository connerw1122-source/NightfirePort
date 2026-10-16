# grates-02b — land 677 without breaking the 0c vent latches

Queue 677b, branch `s/grates-02b` (origin/master e416e5f0 + the re-applied s/grates-02 merge + one
test re-point). The brief's hypothesis — the ss.rs change for the 02 grates altered an arm the 0c
hatches share, to be narrowed by class/flags — was tested against the disc and **came apart**: there
is no per-class or per-flags arm in the game's SS code that separates grates from hatches. The actual
regression was one stale test file asserting the old port invention. Both the 02 grates and the 0c
latches now pass with the 677 state machine unchanged.

## The evidence (why there is no "shared arm" to narrow)

- `SS_Update` 0x1a9948 (decomp + disasm 0x1a9b90-0x1a9c18) is a state machine: 0 idle, 1 playing,
  8 looping, 0x40 armed-rest. The playhead is never channel-seeked; the only channel *input* is
  inside `SS_Operate`.
- `SS_Operate` 0x1a9df8: presence bits (mask 0x2/0x8/0x10 vs the obj+0xD0 contact chain's class
  bytes 2/3/5) — and **only when no presence bit matches**, the channel fallback (`+0x10 != 0` &&
  `switch_channels[+0x10] != 0`; the `== 0` short-circuit at 0x1a9df8:47-48 means a ch-0 SS can
  never be channel-driven). The class reads here are the *contact chain's* classes, not the SS's own
  class or flags — nothing in the SS code branches on grate-vs-hatch.
- `SS_Create` 0x1a9c58 field map [M]: ext+0x22 = record+0x2C (state, p0), ext+0x24 = record+0x30
  (wrap flag, p1), ext+0x1E = record+0x34 (mask, p2), ext+0x10 = record+0x38 (channel, p3),
  ext+0x14 = record+0x3C (sound, p4), ext+0x08 = record+0x40 (rest, p5), ext+0x26 = record+0x44
  (spline, p6), ext+0x1C = key count − 1, ext+0x04 = last/rate (the port's step).
- The 0c disc data [D, probe over FILES.BIN → 0700000c.bin main map, all eight 0xE2 placements]:
  display vent pair (the tests' #138/#179 = record #96/#114) **state 0x40, p1=1, mask 1, ch 18**;
  warehouse pair (#180/#184 = #115/#119) **state 0x40, p1=0, mask 1, ch 84**. The 02 grates
  (grates-02's census): state 0x40, p1=1, mask 0x4/0x1, ch 0/1/2.
- So the merged ss.rs already does exactly what the game does, with ONE arm for both: the hatches
  are channel-driven (mask 1 carries no presence bits → the channel fallback), the ch-0 grates are
  not (the short-circuit) and open via `SS_Activate` 0x1a9ea8's E path. The wrap: t wraps to 0 past
  the last key [M 0x1a9948:119-128]; p1=1 disarms to state 0 — one sweep, then the object holds its
  end pose (states 0/0x40 never re-pose [M 0x1a9948:75-83]); p1=0 re-arms to 0x40 — the sweep
  re-runs while the channel stays latched (ch 84 latched by the lock's write [M 0x1ae3d8:194];
  nothing clears it).

## The actual break and the fix

`fix_level_0c_live` (the earlier 0c-hatch task's file) asserted `(t138 − 5.0) < 0.01` and
`(t180 − 5.0) < 0.01` — the **parked playhead of the old channel-seek model**, which the game never
produces. The 677 merge re-pointed its own 0c files (`fix_0c_hatch_live`, `fix_0c_vent2_live`) to
the pose observable but missed this older file — that is the whole regression, not an ss.rs arm.

Fix (test-only): `fix_level_0c_live`'s two tests now assert the game's observables, keeping their
intent (the burn opens the vent):

- the reached-last-key tick (≤ 55 for the rate-50 hatch, ≤ 13 for the rate-10 blocker [M 0x1a9c58's
  ext+0x04 = last/rate per tick]);
- the wrap: the display pair rests at t = 0 with the held end pose (interp at the last key) and the
  ray through the opening passes (assert unchanged); the warehouse pair's playhead comes back below
  its last key (the sweep re-runs, p1=0 [D #115/#119]);
- the burn/mux asserts unchanged (`burned == 4`, ch 18 via the AND mux, ch 84 latch).

## Verification

- `cargo test --offline --release -p nf-view --test fix_level_0c_live --test fix_ss_grate_live
  --test fix_0c_hatch_live --test fix_0c_vent2_live` — fix_level_0c_live 4/4 (was 2 failing),
  fix_ss_grate_live 4/4 (the 02 grates), fix_0c_hatch_live 2/2 (+2 pre-existing `#[ignore]` legs with their own filed live checks),
  fix_0c_vent2_live 3/3.
- Full suite via `tools/test/suite.py` on the final tree: **(total below, filled at finish)**.

## Live checks for the main agent

1. The warehouse pair's re-sweep: in PCSX2 0700000c, burn the warehouse `VentLatch` and watch
   `ventmesh`/`VentmeshBlockingPloy` for ~2 s. The port's reading (p1=0 → re-arm 0x40, ch 84
   latched) says they REPLAY their sweeps. If the real game plays them once and holds, the next
   session should trace what clears ch 84 (a script) — the ss.rs mapping itself is disasm-verified.
   Tools: `gdbwatch` on `switch_channels[84]` (0x30ca92-base array) for writes after the burn, and
   on the SS ext+0x22 state word for the 0x40→1→wrap transitions.

## What's left

- Nothing blocking. The suite total lands below before finish.
