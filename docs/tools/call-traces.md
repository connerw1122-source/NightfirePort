# Call traces: the real game's function calls, for bit-exact differential tests

`tools/trace/gdbcall.py` records calls of one game function in the GDB-server PCSX2 (docs/tools/pcsx2-gdb.md): the
inputs at entry and the outputs at return. The port replays each call through its own version of the function and
must produce the same bits (`crates/nf-game/tests/diff_calls.rs`, queued as `045n-verify-bitexact`). Files live in
`tests/calls/<function>.calls`. They hold runtime register and memory values only (positions, seeds, matrices): no
disc bytes.

## Recording

```
python -I tools/trace/gdbcall.py <function|0xaddr> --spec "<regions>" [--calls N] [--seconds S] [--out FILE]
```

PCSX2-gdb running with a level loaded (`tools/trace/goto_level.py 07000005`). A breakpoint at the entry captures
every register (`g`) and the `in`/`io` regions; a temporary breakpoint on `$ra`, matched on the entry `$sp`, captures
the return and the `out`/`io` regions. Nested calls of the same function are not recorded (the entry breakpoint is
off until the return). Close PCSX2 afterwards.

A region is `in:<expr>:<len>`, `out:<expr>:<len>` or `io:<expr>:<len>`. `<expr>` sums registers (`a0`, `sp`, `s1`…),
numbers and `@(<expr>)` (the u32 at that address, read at entry). The expression is always evaluated with the entry
registers, so an `out` region is the memory the caller passed in, read after the call.

## Format (v1, text)

```
# gdbcall v1 function=<name> addr=<entry> spec="<regions>" level=<level code>
call <n> frame <GameState+0x34> ra <hex> sp <hex>
gpr <32 × u32 hex: zero at v0 v1 a0..a3 t0..t7 s0..s7 t8 t9 k0 k1 gp sp fp ra, lower 32 bits>
fpr <32 × u32 hex: f0..f31 IEEE bit patterns>
in <expr> <address> <length hex> <bytes hex>          (one per in/io region)
ret v0 <hex> v1 <hex> f0 <hex>
out <expr> <address> <length hex> <bytes hex>         (one per out/io region)
<blank line>
```

Float arguments arrive in `f12..f19` (EE ABI); a float result is `f0`. 64/128-bit GPR upper halves are not recorded:
record the memory the function writes instead.

## Recorded so far (2026-10-12, 07000005 at the start, nobody playing)

| File | Spec | Calls | Checked |
|---|---|---|---|
| `Quat_QuatToMat.calls` (0x1e3e80) | `in:a0:16 out:a1:64` | 40 | every output row is unit length (within 6.2e-6) |
| `Rand_Rand.calls` (0x1e3738) | `io:0x30d0a0:4 io:0x30d0a4:4` (SEED_X, SEED_Y) | 40 | return == ((SEED_Y' & 0xffff) · a0) >> 16 on 40/40 |
| `PS2Sinf.calls` (0x120700) | registers only (`f12` in, `f0` out) | 40 | **differs from the true sine by up to 3.84e-5; bit-identical to f32(sin) on only 13/40**: live evidence for G-017 |
| `Player_AutoAim.calls` (0x1a7c98) | `io:@(a0+0xe0)+0x114:0x20 in:@(a0+0xdc)+0x60:4` | 80 | recorded under play in aim mode: the aim bit set on all 80, the cursor non-zero, the block changed by the 0.5 easing on 76 |
| `Check_Target.calls` (0x1a7810) | `io:@(a0+0xe0)+0x130:4` | 40 | no target under the crosshair (+0x132 = 0, +0x133 = 2) on all 40: the no-hit path only |

## Harness status (045n verify-bitexact, 2026-10-13; ledger added by verify-ledger 2026-10-15)

`crates/nf-game/tests/diff_calls.rs` replays every recording through a port adapter and requires the same bits:
`PS2Sinf` 40/40, `Quat_QuatToMat` 40/40 (EE-FPU op order from the decompile), `Rand_Rand` 40/40 including the
state after each call. `Check_Target` and `Player_AutoAim` are not covered yet (no adapter). Sisters: the
platform f32 sin differs from the game on 24/40 recorded calls; the same polynomial with FPU no-guard sums
differs on 11/40 (the VU0 guard bit is load-bearing); a flipped output bit fails, a caller-owned byte does not.
Format v1 carried everything the adapters needed.

**The ledger** (`docs/verify/ledger.md`) is the claim inventory over this machinery: one row per ported sim
function (address, name, port fn, status, recording), seeded from the port maps + the atlas, hot paths first —
with the exact `gdbcall.py --spec` command and the level/spot for every recording still missing. The guard test
`ledger_claims_match_the_recordings` (same file) fails if a row claims `bit-exact on N recorded calls` without
the recording behind it (≥ N calls, matching `function=`/entry address, adapter registered) or if any row's
status leaves the documented grammar. Recording requests live in the ledger's "Recording requests" section.
