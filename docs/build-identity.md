# Build identity

Everything in this repo (addresses, offsets, symbol values) is for this exact build. Re-check before trusting any
number on another disc.

| Field | Value | Evidence |
|---|---|---|
| Title | 007: Nightfire | *measured:* volume id `NIGHTFIRE` |
| Serial | SLUS-20579 (boot file `SLUS_205.79`) | *measured:* `SYSTEM.CNF` = `BOOT2=cdrom0:\SLUS_205.79;1`, `VER=1.00`, `VMODE=NTSC` |
| Region / video | NTSC-U, NTSC | *measured:* `SYSTEM.CNF` |
| Disc version | 1.00 | *measured:* `SYSTEM.CNF` `VER=1.00` |
| Volume creation date | 2002-10-20 00:32:12 | *measured:* PVD+813 `2002102000321200` |
| Image | `C:\Users\Yoda\Downloads\PS2\007 - Nightfire (USA).iso`, 2,439,315,456 bytes = 1,191,072 sectors (matches PVD volume size) | *measured* |
| Image SHA-1 | `03717059f97630e9ee2fd56dcf537c6fe3ed96d1` | *measured* (`sha1sum`) |
| Image MD5 | `008302d82a305e9ad04e349f1f6982ee` | *measured* |
| Image CRC-32 | `848bacb6` | *measured* |
| Redump match | not yet checked (UNKNOWNS U-001) | — |

## Executables

| File | LBA | Size | SHA-1 | Entry | Load | Notes |
|---|---|---|---|---|---|---|
| `SLUS_205.79` | 486 | 116,792 | `d5a9bd3ee0e9f1faf519fbd27138a2cdf4e82efe` | 0xa00008 | 0xa00000 | boot loader; byte-identical to `BASE.ELF` (LBA 3721); stripped |
| `ACTION.ELF` | 2567 | 2,362,400 | `e7ffdcb38ccd309388046ab864ebde5e3414eaab` | 0x2242a8 | 0x107100 | Eurocom on-foot engine; **unstripped** (5,603 symbols, 3,038 functions); `$gp` 0x314670 |
| `DRIVING.ELF` | 1153 | 2,895,244 | `d7b8a57f3b8e17e980a6d9228b2ad02ed8eebc5c` | 0x24c0d8 | 0x107100 | EA driving engine; stripped; `$gp` 0x3cfdf0 |

All *measured* (`nf-extract info` / `elfs`, the `NF_ISO` golden test `executables_match_build_identity`).
The loader's strings name `cdrom0:\ACTION.ELF;1`, `cdrom0:\BASE.ELF;1`, `cdrom0:\DRIVING.ELF;1` and dev-host names
`host0:BondPS2_Release.elf`, `host0:BasePS2_Release.elf`, `host0:TestPS2_Release.elf` (*measured*, strings).

PCSX2's game CRC for this build: not yet recorded (UNKNOWNS U-001).
