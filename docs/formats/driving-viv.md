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
  (`paris_mis01.crp` 2,123,627 B compressed → 6,825,216 B announced decompressed size [D]);
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
