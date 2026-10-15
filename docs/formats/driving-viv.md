# Driving mission archives — content census (`DRIVING/*.VIV`, `*.MUS`, `*EN.SPE`)

The BIGF container itself is `docs/formats/bigf.md` (layout *measured*; golden
`golden/bigf_entries.tsv`: 23 archives, 1,499 entries, per-entry content SHA-1 — that golden was
measured through the ISO reader on the full disc image). This file records what is *inside* the
entries: the census the `research-driving` task measured over every one of the 1,499 entries
(*measured*, [D]; repro: `crates/nf-formats/tests/driving_golden.rs`, golden
`golden/driving_census.tsv`).

## Per-archive census (*measured*, [D])

| Archive | Entries | Payload B | Notes |
|---|---|---|---|
| `MIS01.VIV` | 152 | 9,051,097 | Paris |
| `MIS11.VIV` | 91 | 9,243,326 | Underwater |
| `MIS13A.VIV` | 132 | 9,101,769 | Jungle A |
| `MIS13B.VIV` | 155 | 9,093,758 | Jungle B |
| `MIS13C.VIV` | 117 | 6,229,698 | Jungle C |
| `MIS3.VIV` | 191 | 9,867,390 | Snow 1 |
| `MIS4.VIV` | 175 | 9,093,388 | Snow 2 |
| `RACE.VIV` | 161 | 8,674,828 | race on the snow2a map |
| `MISC.VIV` | 89 | 189,091,798 | shared: render modules, HUD, locale, cutscenes, car models — all 89 entries plain (no RefPack) |
| `*.MUS` (7) | 4–6 each | 5.9–12.0 MB each | `.asf` music streams |
| `*EN.SPE` (7) | 6–41 each | 7.0–37.4 MB each | `.asf` speech streams |

## Every entry is one of three things (*measured*, [D], all 1,499 entries)

1. **RefPack-compressed** (`10fb` first two bytes; EA's RefPack/QFS window flag) — the majority
   (e.g. 146 of 152 in `MIS01.VIV`).
2. **ASF audio** (`.asf` entries inside `.MUS`/`.SPE`, magic `SC`…).
3. **Plain**, with these first bytes (*measured* per extension):
   - `.atr` `[` (INI-style sections, e.g. `[sentry]`);
   - `.crp` `PRAC` (the car/track physics container; note `CARP` is the driving engine's car-physics
     namespace — reading "PRAC"/"CARP" as related is [S]);
   - `.gal` `LAGG` (HUD galleries — `hudNTSC.gal`, 251,059 B, identical in every mission .VIV);
   - `.loc` `LOCH` (locale), `.mpc` `MPCh` (cutscenes), `.ssh` `SHPS` (sprite/texture sheets),
     `.sfn` `FNTS` (fonts), `.def` `acti`/`POV ` (control configs), `.ini` `[def`;
   - `.tun` `fogS` (`data\tuning\Render\Fog\*.tun`) and `Farp` (`data\tuning\Render\<level>.tun`);
   - `.o` `\x7fELF` — **MISC.VIV's render modules are real ELF objects**
     (`data\render\eaglrm.o`, `data\render\bondrm.o` [D]);
   - `.sub` (loading screens) — plain text starting with a line count (`14\r\nF…`).
   - `.bnk` — most RefPack-compressed; some plain `BNKl` (e.g. `a1_paris.bnk` [D]).

A RefPack decompressor is **not** in the repo yet; through the compressed stream these literals
are visible (*measured*, [D]): `default.tun` contains `GRAVITY=-34`, `SLEEP_VEL=5`,
`GROUND_FRICT…` (key=value text lines); `loading.sfn` starts `FNTS`; the track `.crp` decompressee
starts `ACRZ`…/`PRAC`-family bytes (interpretation [S], needs the decompressor).

## Entry tree of a mission .VIV (stable across all seven, *measured*, [D])

- `data\track\<level>.crp` — **the road/track data**, by far the largest entry
  (`paris_mis01.crp` 2,123,627 B compressed → 6,826,240 B announced decompressed size [D];
  the header's 3-byte BE size field reads `68 29 00` — an earlier draft of this file said 6,825,216,
  a transcription error fixed by the phase-0 census, golden/driving_refpack.tsv row
  `/DRIVING/MIS01.VIV#2`);
- `data\car\model\<car>.crp` — car models (`paris_henchbigbox.crp`, `alpha_sub.crp`,
  `jungle_hench.crp`, …);
- `data\audio\g_common.bnk` + one level bank per mission (`g1_paris.bnk`, `g11_isap.bnk`,
  `g13a_jn1.bnk`, …) + `banks.ini` + `g_common.h` + a per-level `.h` + `<level>.ini`;
- `data\render\common.ssh` (228,002 B, identical everywhere) + per-level particle `.ssh`;
- `data\gfx\data\hudNTSC.gal` (identical everywhere);
- `data\sim\attrib\attrib.dir` (1,456 B, identical) + `world\default.atr` + per-level `.atr`;
- `data\actors\anims\ALookup.bin` (544 B) + `driving.dat` (4,459 B) + `driving.rel` (419 B) +
  `data\actors\skeleton\0skel.o`/`1skel.o` (identical everywhere) — the same animation system the
  on-foot side uses;
- `data\tuning\Physics\Rigid\default.tun` (298 B) / `Physical\default.tun` (365 B) — identical
  physics tuning in every mission;
- `data\Control\Master.def` (80 B) / `FrontEnd.def` (548 B) — control configs (G-004 relevance);
- `data\loading\loading.sfn` (13,470 B) — loading-screen font.

The shared-everywhere set (skeletons, ALookup, common.ssh, hudNTSC.gal, attrib.dir, physics
default.tun) means a port needs to parse them **once** and can pin them as fixed goldens.

## What the current parsers do with this (*numbers*, *measured*)

| Parser | Over the driving data | Result |
|---|---|---|
| `bigf.rs` | all 23 archives | parses: 1,499/1,499 entries, golden-pinned (`driving_golden.rs`) |
| `symfile.rs` | `DRIVING.SYM` | parses: 12,719 entries (`driving_golden.rs`) |
| everything else | entry contents | **not yet**: contents are RefPack-compressed or plain formats with no reader in the repo (see the table above) |

## RefPack decompression (phase 0, *implemented* — `crates/nf-formats/src/refpack.rs`)

The game's own decompressor [M] (disassembly-verified; EE = file offset + 0x107080):
`DRIVING.ELF 0x00323bb0` gates (`stream[1] == 0xFB`; flags `0x10`/`0x90` accepted, `0x1E`/`0x9E` would
route to a pluggable decoder that is never registered — the pointer at EE `0x3C8150` reads 0 and has
only loads), `0x00323e98` decodes (header: byte-0 bit `0x80` → 4-byte BE decompressed size instead of
3-byte; bit `0x01` → a compressed-size field of the same width sits first and is skipped, never used by
driving data), `0x00323cc8` is the size probe (it ignores bit `0x01` — a quirk the port does not copy).
Callers: `FUN_003069c0`/`FUN_00306b38` (read → probe → alloc → decompress), the VIV layer
`FUN_002d7338` → `FUN_002d7138`/`FUN_002d7400` → `FUN_003069c0`. ACTION.ELF contains **no** RefPack
decompressor (negative search over its decompile).

Opcode stream (all offsets +1, all copies forward byte-wise so overlap/RLE behaves):

| op range | stream | length | offset |
|---|---|---|---|
| `00-7F` | `[op][D1][lit × (op & 3)]` | `((op >> 2) & 7) + 3` | `(D1 \| ((op & 0x60) << 3)) + 1` (10-bit) |
| `80-BF` | `[op][D1][D2][lit × (D1 >> 6)]` | `(op & 0x3F) + 4` | `((D1 & 0x3F) << 8 \| D2) + 1` (14-bit) |
| `C0-DF` | `[op][D1][D2][D3][lit × (op & 3)]` | `((op & 0x0C) << 6 \| D3) + 5` | `(D2 \| D1 << 8 \| bit) + 1` (17-bit, `bit = 0x10000` if `op & 0x10`) |
| `E0-FB` | `[op][lit × ((op & 0x1F) << 2) + 4]` | — | — |
| `FC-FF` | stop, `[lit × (op & 3)]` tail, end | — | — |

**Divergence from the published spec** (segaretro.org "RefPack compression", the `refpack-rs`
crate flavors and the Raikaru/nightfire-engine docs — docs/research/driving-engine-ea-formats.md §2.2):
the public ordering of the `00-7F` command is `(op & 3) + 1` literals **first**, then D1. The game reads
**D1 first**, then exactly `(op & 3)` literals. Both orderings were run over every RefPack entry of the
driving archives: the game's ordering decodes **1,126/1,126** streams to exactly their header-announced
size with the stream consumed to the last byte; the public ordering decodes **3/1,126**. The game wins;
the port follows the game.

## Decompressed census (`golden/driving_refpack.tsv`, *measured*, [D])

Every RefPack stream of the 8 mission archives — the only archives that carry any (MISC.VIV is all
plain, `*.MUS`/`*EN.SPE` are all ASF audio — the census split above) — decompressed through the port:
**1,126 streams, 70,285,629 compressed → 132,706,995 decompressed bytes**, each one matching its
header-announced size exactly and consuming the stream to the last byte. Per-archive counts (match the
census's `refpack` column): MIS01 146, MIS11 82, MIS13A 125, MIS13B 150, MIS13C 114, MIS3 184, MIS4
168, RACE 157. Golden rows are keyed `archive#table-index` (the same index as `bigf_entries.tsv`, so
the compressed and decompressed goldens cross-reference). Repro:
`crates/nf-formats/tests/driving_golden.rs` (`driving_refpack_entries_decompress_to_the_golden` +
the coverage test against the census), folder via `NF_DRIVING_REFPACK` (or `NF_DRIVING`/`DRIVING/`).
First decoded contents confirm the doc's earlier observations: `default.tun` → `GRAVITY=-34` /
`SLEEP_VEL=5` key=value text; `loading.sfn` → `FNTS`; the track `.crp` → `ACRZ…`/`PRAC`-family bytes.
