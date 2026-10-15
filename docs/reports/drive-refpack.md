# drive-refpack — driving phase 0: the RefPack decompressor and the decompressed census

Branch `s/drive-refpack` (from origin/master @ a2958c03; merged origin/master mid-task, WORKLOG both
kept). Task: 500-drive-refpack. Everything below cites its evidence.

## What's done

1. **`crates/nf-formats/src/refpack.rs` (new)** — the EA RefPack (QFS) decompressor, implemented from
   **the game's own decompressor** [M, disassembly-verified; EE = file offset + 0x107080]:
   - `DRIVING.ELF 0x00323bb0` — the gate: `stream[1] == 0xFB`; flags `0x10`/`0x90` accepted (bit 0
     masked), `0x1E`/`0x9E` route to a pluggable decoder that is never registered in this build (the
     pointer at EE `0x3C8150` reads 0 and has only loads: 0x323c80/0x323e04/0x323e34) — those streams
     always fail here too;
   - `DRIVING.ELF 0x00323e98` — the decode: header byte 0 bit `0x80` → 4-byte BE decompressed size
     instead of 3-byte; bit `0x01` → a compressed-size field of the same width sits first and is
     skipped (never used by driving data); the opcode loop;
   - `DRIVING.ELF 0x00323cc8` — the size probe (it ignores bit `0x01` — a quirk the port does NOT
     copy; the port follows the real decompressor's header code);
   - callers measured: `FUN_003069c0`/`FUN_00306b38` (read → probe → alloc → decompress), the VIV
     layer `FUN_002d7338` → `FUN_002d7138`/`FUN_002d7400` → `FUN_003069c0`.
   ACTION.ELF contains **no** RefPack decompressor (negative search over its whole decompile) — the
   on-foot side consumes its packs raw.

2. **The format, as the game implements it** (doc table in `docs/formats/driving-viv.md`):
   opcodes `00-7F` `[op][D1][lit×(op&3)]` len `((op>>2)&7)+3` off `(D1|((op&0x60)<<3))+1`;
   `80-BF` `[op][D1][D2][lit×(D1>>6)]` len `(op&0x3F)+4` off `((D1&0x3F)<<8|D2)+1`;
   `C0-DF` `[op][D1][D2][D3][lit×(op&3)]` len `((op&0x0C)<<6|D3)+5` off `(D2|D1<<8|bit)+1` with
   `bit=0x10000` if `op&0x10`; `E0-FB` pure literal run `((op&0x1F)<<2)+4`; `FC-FF` stop with an
   `(op&3)`-byte literal tail. All offsets +1; all copies forward byte-wise (overlap/RLE safe).

   **Measured divergence from the published spec** (segaretro.org RefPack, refpack-rs flavors,
   Raikaru/nightfire-engine — docs/research/driving-engine-ea-formats.md §2.2): the public ordering of
   the `00-7F` command puts `(op&3)+1` literals FIRST, then D1. The game reads **D1 first**, then
   exactly `(op&3)` literals. Both orderings were run over every RefPack entry of the driving
   archives: the game's ordering decodes **1,126/1,126**; the public ordering decodes **3/1,126**
   (first failure: MIS01 `data\sim\attrib\attrib.dir`, "offset 866 before start at out=28"). The port
   follows the game; the 3 double-passes are streams that happen not to exercise the divergence.

3. **Unit tests (9)** on hand-built streams covering every opcode class: header variants `0x10/0x90/
   0x11/0x91` + wrong-magic rejection; pure literal runs; `00-7F` with 0 and 1 literals + 10-bit
   offset; `80-BF` with prefixed literals + 14-bit offset; `C0-DF` RLE + the 17-bit offset with the
   `0x10000` bit (a 69,491-byte hand-built stream); all four stop variants; six malformed-stream
   error cases (no panic, unlike the game, which trusts the data); a round-trip on the measured
   `default.tun` keys (`GRAVITY=-34…`).

4. **The decompressed census golden** — `crates/nf-formats/golden/driving_refpack.tsv` (numbers +
   names only, keyed `archive#table-index`, the same index as `bigf_entries.tsv` so the compressed and
   decompressed goldens cross-reference): **every RefPack stream of the 8 mission archives — 1,126
   streams, 70,285,629 compressed → 132,706,995 decompressed bytes** — each one matching its
   header-announced size exactly AND consuming the stream to the last byte. The 8 mission .VIVs are
   the only archives that carry RefPack streams (MISC.VIV all plain, `*.MUS`/`*EN.SPE` all ASF audio —
   the pinned census split [D]); the new test
   `refpack_golden_covers_every_refpack_entry_of_all_23_archives` asserts the golden covers exactly
   the census's per-archive `refpack` counts (MIS01 146, MIS11 82, MIS13A 125, MIS13B 150, MIS13C 114,
   MIS3 184, MIS4 168, RACE 157) and that no `.MUS`/`.SPE` archive appears. Per-archive counts match
   the census exactly. Anchor row: `paris_mis01.crp` 2,123,627 → **6,826,240** (size-field bytes
   `10 fb 68 29 00`) — this also **fixes a transcription error** in driving-viv.md, which said
   6,825,216. First decoded contents confirm the doc's observations: `default.tun` → `GRAVITY=-34` /
   `SLEEP_VEL=5` key=value text; `loading.sfn` → `FNTS`; the track `.crp` → `ACRZ…`/`PRAC`-family
   bytes.

5. **Tests** (`crates/nf-formats/tests/driving_golden.rs`, appended):
   `driving_refpack_entries_decompress_to_the_golden` (decode all 1,126, compare sizes+SHA-1s, the
   paris anchor, tamper sister) and `refpack_golden_covers_every_refpack_entry_of_all_23_archives`
   (coverage vs the census split, with its own sister). Read path: `NF_DRIVING_REFPACK`, else
   `NF_DRIVING`, else `<repo>/DRIVING`; skip (with a message) when absent — the slim ISO carries no
   driving bytes (DRIVING directory extents are zeroed [D]).

6. **Cross-check against an independent decompressor**: none existed runnable here (no PyPI package —
   `pip install refpack` has no matching distribution; no venv tree in `work/third_party/`), so the
   cross-check is a scratch Python implementation (NOT committed) written from the disassembly,
   run over the same 1,126 streams: identical sizes, and its SHA-1s are exactly the golden's (the
   Rust golden test re-derives the same hashes from the same bytes). Independent of that, the
   header-announced-size equality and the full-stream-consumption equality over all 1,126 real
   streams, plus the census-count agreement, pin the semantics three ways. On the one disagreement
   found (the public ordering, above) the game's disassembly settles it.

## How it was verified

- `cargo test --offline --release -p nf-formats --lib refpack` → 9/9 pass.
- `NF_DRIVING_REFPACK=… cargo test --offline --release -p nf-formats --test driving_golden` → 6/6
  pass (4 pre-existing + the 2 new, the new decode test with the real 8 archives).
- The chunked suite (`tools/test/suite.py`, foreground chunks — this sandbox reaps background jobs):
  chunk 1 85 binaries (all green but the known no-GPU pylon trio), chunk 2 the core set
  (`nf_game`/`nf_formats`/`nf_audio`/`nf_view` libs + `driving_golden` + `disc_golden` +
  `facial_probe` + `pss_movies_live`) → **289 passed / 3 failed** — the 3 are exactly the allowed
  slim-ISO `disc_golden` set; chunk 3 49 more binaries, zero failures. `driving_golden` re-run with
  the real folder inside the suite window: 6/6. **~196 binaries were not reached** (the
  integration/live tail of nf-game/nf-view — list below); they touch none of this task's code (the
  change is additive to nf-formats only).

## What's left

- The ~196 unrun suite binaries (nf-game/nf-view integration + live tests, e.g. everything after
  `ear_job_live` alphabetically plus `arena_smoke`… in the third chunk's tail): run the full suite on
  the merged tree before merge — standard for every merge anyway. Chunks 1-3 logs: /home/z/suite_chunk*.log (scratch, not committed).
- `.crp`/`.gal`/`.ssh`/… content formats (driving phase 1) — untouched, as scoped.
- The 0x1E/0x9E callback decoder: unregistered in this build, unimplementable from it — the port
  errors on those flags like the game does.

## Live checks for the main agent

No rendering is touched (data-layer task; nothing to `--bond`). PCSX2, optional, ties the decompile to
runtime:
1. During a driving mission load (e.g. Paris), breakpoint `0x00323cc8` (the size probe) with a watch
   on `a0`/return `v0`: for the `paris_mis01.crp` entry the return must be **6,826,240** (0x682900);
   access type: execute; expected: one call per compressed entry, return = the entry's announced size.
2. At the same breakpoint log byte `[a0+0]` (the flag byte) across a full mission load: expected
   **every value 0x10** (bit 0x80/0x01/0x1E/0x9E never appear — the census and the header parse both
   say so; this confirms it at runtime).

## Shared files touched

- `crates/nf-formats/src/lib.rs`: one additive line — `pub mod refpack;`.
- `docs/WORKLOG.md`: appended `drive-refpack` entries only.
- `crates/nf-formats/tests/driving_golden.rs` and `docs/formats/driving-viv.md` are named by the task
  (appended; the one edit inside driving-viv.md is the 6,825,216 → 6,826,240 correction with its [D]
  evidence).

## ntfy

Sent after the final push (or the exact error recorded here if all three fallbacks fail).
