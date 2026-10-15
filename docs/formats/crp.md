# The `.crp` CARP container (the driving side's core resource format)

Status: **decoded and pinned** (task `drive-crp-format`, 2026-10-15). Parser:
`crates/nf-formats/src/crp.rs`; goldens: `crates/nf-formats/golden/crp_golden.tsv` (all 89 `.crp`
entries of the 23 driving archives) in `crates/nf-formats/tests/driving_golden.rs`. Evidence tags:
[M] code (function + address), [D] disc bytes, [S] suggested. DRIVING.ELF is stripped; EE address =
file offset + 0x107080 (docs/atlas/driving.md). Every structure below was measured on our disc
data *and* matched to a consumer in the decompile; nothing is asserted from memory.

## 1. Where the files live [D]

`data\track\<level>.crp` — one per mission archive (MIS01/MIS11/MIS13A/MIS13B/MIS13C/MIS3/MIS4/RACE,
entry index #4 in each VIV, 0.8–2.9 MB compressed → 2.4–7.2 MB decompressed); `data\car\model\<car>.crp`
(~30 entries, cars/helicopters/boats/subs); `data\actors\weapons\<weapon>.crp` (10 small entries,
2.3–12 KB decompressed). Total: **89 entries over the 23 archives**; the five `MISC.VIV` car models
(`taxi`, `bus_paris`, `bus_tokyo`, `junglesnow`, `jungle_truck`) are the only **plain** ones — every
other `.crp` is RefPack-compressed (`10fb`, `nf_formats::refpack`). The container loads through the
BIGF reader; the track path is [M] `FUN_0022f898` 0x22f898 (called from the boot chain
`FUN_00174198` 0x174198 line 328 after the `Loading track file` progress string 0x375b60): it opens
`data\track\<key>.crp` inside the mission `.VIV` (the key comes from PTP+0x4f4, docs/atlas/driving.md §2).

## 2. The decompressed image: a pool of 16-byte records [M + D]

The decompressed bytes are a flat array of **16-byte records**. A record is:

```text
+0x0  u32 tag      a big-endian 4-char code — "CARP" (0x43415250), "Arti" (0x41727469),
                   "DBN " (0x44424e20), "ELFd" (0x454c4664), "sn  " (0x736e2020), …
+0x4  u32 flags    bit 0x01: short tag — the low 16 bits are an instance index; compares
                             normalize the low half to 0x2020 ("sn  ") [M 0x1a9600 lines 87-95]
                   bit 0x02: the reference is relative to this record's own address;
                             table references scale dref by 0x10 (record units)
                             [M 0x2d6940: lw 4($a0); andi $v0,2; beq; lw 12($a0);
                             sll $v0,$v0,4 (word 0x00021100); addu $v0,$a0,$v0],
                             payload references use raw bytes [M 0x1a9600/0x22f898 inline]
                   bits 5+:  the record's scan-window skip, in records [M 0x2d5a10/0x2d5920]
                   bits 8+:  the payload size in bytes [M 0x1a9600: (flags >> 8) = the ELF size]
+0x8  i32 count    the record's scan-window length, in records [M 0x2d5a10]
+0xc  i32 dref     the table/payload reference (see flags bit 0x02)
```

⚠ **Two relative-reference modes share flags bit 0x02** — this cost us a false start: the C
decompile of `FUN_002d6940` 0x2d6940 shows `*(int *)(param_1 + 0xc) * 0x10` and the consumers at
0x1a9600/0x22f898/0x2dc1b0 show a plain byte add. The disassembly settles it: **both are real**.
`FUN_002d6940` (the *table* resolver used by the window walkers `FUN_002d5a10` 0x2d5a10 and
`FUN_002d5920` 0x2d5920) scales relative drefs by 16; the chunk consumers' inline resolution
(`(int)puVar1 + puVar1[3]` at 0x1a9600, `iVar2 + *(int *)(iVar2 + 0xc)` at 0x22f898/0x2dc1b0) does
not. Absolute mode (bit 0x02 clear) never scales.

The **root record** (offset 0, tag `CARP`) points at its record table; the walkers
`FUN_002d5a10(root, tag)` / `FUN_002d5920(root, tag, 0, -1)` scan the window
`[table + (flags>>5)*0x10, + count*0x10)` — the records before the window are the **pool**, the
window itself is the **chunk directory**. Both finders walk 16 bytes at a time comparing the raw tag
word (`FUN_002d5920`'s 4-arg form additionally matches a normalized short tag with
`index <= param_4`).

## 3. The record kinds and their consumers [M]

| Tag | Meaning | Consumer (address) |
|---|---|---|
| `CARP` | the root record | every consumer |
| `Arti` | an **article**; each article's own window holds a `Name` record whose payload is its string | namer `FUN_002dc1b0` 0x2dc1b0 (line 97-108: `FUN_002d5a10(arti, "Name")`) |
| `Shar` | "shared" article | namer 0x2dc1b0 (`<<Shared>>`) |
| `CDat` | **collision data** article | namer 0x2dc1b0 (`<<Collision>>`) — confirms driving.md §3's [S] that the collision world comes out of the track `.crp` |
| `RNgp` | **road network** article | namer 0x2dc1b0 (`<<RoadNetwork>>`) |
| `Map ` | map article | namer 0x2dc1b0 (`<<Map>>`) |
| `DBN ` | the track **name** (payload = NUL-terminated string, size = flags>>8) | track loader `FUN_0022f898` 0x22f898: scans the root window for 0x44424e20, takes the payload string and registers the track under it (`FUN_001697e0` 0x1697e0) |
| `ELFd` | the embedded **ELF module**: a 32-bit little-endian MIPS relocatable object (`e_type=1 ET_REL`, `e_machine=8 MIPS`) | module loader `FUN_001a9600` 0x1a9600 → `FUN_0029f7c8`/`FUN_0029f820` (load + relocate) |
| `ELFr` | the module's relocations | 0x1a9600 (`FUN_0029f820(..., elfr, 1, ...)`) |
| `MapN` | map name (payload string) | present in track directories (measured; consumer not yet pinned) |
| `Sect` | sections descriptor, size = flags>>8 | 0x1a9600 tail → `FUN_002daa18` 0x2daa18 (with `Shar`) |
| `sn  ` | instance-indexed sub-resource (tag = `sn` + index in the low half) | 0x1a9600 loop: names via format string 0x37f318 + registry `FUN_001cfd80`, indexes `(&PTR_PTR_003336a0)[index]` |

## 4. What is inside each kind of container [D, golden/crp_golden.tsv]

- **Track** (8 files): pool `Arti×(231..494) + CDat + Map  + RNgp + Shar`, directory
  `DBN  + ELFd + ELFr + MapN + Sect + sn#0`. The `DBN ` names are exactly the level-key suffixes:
  `paris` (MIS01), `underwater` (MIS11), `jungle` (MIS13A/B/C), `snow1a` (MIS3), `snow2a`
  (MIS4 and RACE). The embedded ELFs are the largest chunks (paris: ELFd 3,926,032 B at 0x99c00 +
  ELFr 678,800 B); the paris ELFd has 6 sections, `t1.m8.s6`.
- **Car/weapon** (81 files): pool `Arti×1 + Shar×1` (weapons: `Arti×(1..3) + Shar×1`), directory
  `ELFd + ELFr + Sect + sn#0` — no `DBN `, no `MapN`, no `RNgp`/`CDat` pool.
- Every one of the 89 files embeds a MIPS ET_REL module (`t1.m8.*`), section counts 3–8.

## 5. What the game does with it (the consumer chain) [M]

1. Boot: `FUN_00174198` 0x174198 (init chain) prints `Loading track file` (0x375b60), allocates a
   0x3c-byte track object (`FUN_002d0910(0x3c)` + ctor `FUN_0022f820` 0x22f820), stores it at
   `DAT_0033aac8`, and calls `FUN_0022f898(obj, "data\track\", <level key buffer>)`.
2. `FUN_0022f898` 0x22f898 builds `<key>.crp` (format `%s.crp` 0x39b620), opens it inside the
   mission archive (`FUN_001a9cc8`/`FUN_001a9d70` — the archive layer that also runs RefPack
   decompression [M 0x323bb0/0x323e98]), then:
   - scans the root window for `DBN ` (`FUN_002d5a10(root, 0x44424e20)`);
   - if the DBN record exists and is not the window end, takes its payload string as the track's
     name and registers it (`FUN_001697e0(obj, name, 1)`).
3. `FUN_001a9600` 0x1a9600 (module loader, called with the same record root): walks the `sn  `
   records, resolves `ELFd` (+ optional `ELFr`) and loads/relocates the MIPS object
   (`FUN_0029f7c8`/`FUN_0029f820`), then `FUN_002daa18` consumes `Sect`+`Shar`.
4. The pool articles (`Arti`/`CDat`/`RNgp`/`Map `/`Shar`) are looked up by name through
   `FUN_002dc1b0` 0x2dc1b0 (the namer above) — the driving engine reads its collision world, road
   network and map data out of these articles [S: the per-article binary layouts are *not* decoded
   yet; that is the next phase's work].

## 6. The port

`crates/nf-formats/src/crp.rs`: `Crp::parse` (root validation), `Record` (`tag_code`, `instance`,
`skip`, `size`, `table`, `data_addr`, `window`), `pool`/`directory`/`find`/`find_instance`/
`payload`/`track_name`/`article_name` — the game's arithmetic with bounds checks (malformed data
errors instead of overrunning; the game trusts its data [M]). Unit tests on a hand-built container
cover absolute/relative table refs (both scalings), short tags + instances, payload bounds, and
bad-magic rejection. `lib.rs`: one-line `pub mod crp;`.

Goldens (`crates/nf-formats/golden/crp_golden.tsv`, 89 rows + header; key =
`/DRIVING/<archive>#<table index>`, cross-referencing `golden/bigf_entries.tsv`):
`decompressed` size, `sha1` of the decompressed image, `root_flags`, the pool histogram, the chunk
directory (`CODE:size:payload_off`), the ELF header triple `t<e_type>.m<e_machine>.s<e_shnum>`, the
`DBN ` track name, and a SHA-1 of each embedded `ELFd` module. Tests in
`crates/nf-formats/tests/driving_golden.rs`: `driving_crp_files_parse_to_the_golden` (byte-exact
re-diff + tamper sister) and `driving_crp_structure_matches_the_consumer_split` (the consumer split
above: 8 tracks with DBN+MapN+RNgp+CDat, 5 plain MISC cars, 89 ELF modules).

## 7. Open questions / live checks for the main agent

1. **Track name registration** (PCSX2): breakpoint the `FUN_002a9760` (strlen) call in
   `FUN_0022f898` 0x22f898 right after the `DBN ` scan; confirm the string at the resolved payload
   equals the golden's `name` (`paris`, `snow2a`, …) for a couple of levels.
2. **The `sn  ` instance names**: the format string at 0x37f318 and the pointer table at
   0x3336a0 — dump both in PCSX2 (what do `sn` sub-resources *mean*? sounds? [S]).
3. **The `MapN` payload**: track directories carry it (14 B for paris, string) — confirm its
   consumer (grep candidates `FUN_0022faf8`/`FUN_00230730`/`FUN_00230ab0`/`FUN_00176658`/`FUN_00212ce8`,
   which reference `Map ` constants) and that the string matches the golden.
4. **Article payloads** (`CDat`, `RNgp`, `Map `, `Arti`): the binary layouts inside the payloads are
   the next format task; the goldens pin their container offsets and sizes so the next pass starts
   from measured ground.
5. **ELF module identity**: `ELFd` payloads start `\x7fELF` ET_REL MIPS — worth a disassembly pass
   (`readelf`-style) against `work/elf/DRIVING.ELF` to see whether the embedded modules are compiled
   driving-side game code (like MISC.VIV's `eaglrm.o`/`bondrm.o` render modules [D]) or data tables.
