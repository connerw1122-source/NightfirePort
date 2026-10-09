# Sound banks (`SB_n.SFX` / `.SHF` / `.SBF`, `SBINFO.SBI`)

Build: SLUS-20579, SFX.IRX `65c4c97e…` (MODULES/SFX.IRX, 444,251 bytes, with `.symtab` and `.mdebug`; decompiled to
`work/decomp/SFX.IRX/` by headless Ghidra as MIPS LE 32, project `work/ghidra-iop`). Tags: [M] decomp, [D] our disc,
[S] suggested (NightfireResearch notes, facts only, written on the PAL disc). Reader:
`crates/nf-formats/src/soundbank.rs`. Runtime: docs/systems/sound.md.

## Files [M, D]

- `PS2/SBINFO.SBI`: 150 × u32 bank hashcodes (`SoundBankInfo`); 25 used, the rest 0xFFFFFFFF. A bank's slot is the
  index of its hash (`FindSoundBankSlot`, 0x2070). Slots 0–24 hold hashes 12, 56, 22, 4, 5, 6, 7, 39, 43, 40, 1, 2, 3,
  8, 26, 55, 11, 37, 23, 24, 25, 30, 31, 32, 33.
- Bank files: `<path><language>/SB_<slot >> 3>/SB_<slot>.{SFX,SHF,SBF}` (`SFXLoadSoundBank` 0x25a4; an `.SHG` path is
  formatted but not loaded). Language `ENGLISH` on our disc.
- `SoundBankData[slot]` (0x24 bytes): +0 SHF file, +4 sample count, +8 headers, +0xC SFX file, +0x10 effect count,
  +0x14 effect table, +0x18 loaded, +0x1C hash, +0x20 (an extra file, 0 here).

## `.SHF`: sample headers [M, D]

u32 count, count × 36 bytes (`SampleHeaderData`):

| Off | Meaning |
|---|---|
| +0x00 | loop: 1 when the sample's end frame has the ADPCM repeat flag (252 / 252 agree on bank 4 [D]) |
| +0x04 | byte offset in the `.SBF`; the loader overwrites it with the SPU address = start + the running sum of sizes, which equals the stored offset on our disc (252 / 252 [D]) |
| +0x08 | size in bytes (the end-flag frame lies inside it for all 252; 0–64 bytes follow it) |
| +0x0C | SPU pitch, 0x1000 = 48 kHz (bank 4: 941 ≈ 11.03 kHz ×102, 1882 ≈ 22.05 kHz ×99, 1024 = 12 kHz ×49, 1621 ×2) |
| +0x10 | ? (slightly below the size) |
| +0x14 | 1 (channels [S]) |
| +0x18 | 4 (bits [S]) |
| +0x1C | 0x60 × index (per-sample slot data, set by `psiLoadSampleInitialise`?) |
| +0x20 | 0 mostly |

## `.SBF` [D]

SPU ADPCM, the samples back to back (sizes sum to the file: bank 4 0x17F340).

## `.SFX`: effect parameters [M, D]

u32 count, count × (u32 effect id, u32 offset). The offset points at the effect's `SFXParameters` block in the same
file, followed by its sub-track records — layout: docs/systems/sound.md. Bank 4 (level 07000005): 225 effects.

## Level → bank [M]

`Sound_Ready` (ACTION.ELF 0x1dceb8) after `SFXReset`: one `SFXLoadSoundBank(hash)` per level — 07000001 → 1,
02 → 2, 03 → 3, 04 → 8, **05 → 5 (slot 4)**, 06 → 4, 07 → 6, 08 → 7, 09 → 0x17, 0A → 0x18, 0B → 0x19, 0C → 0x1A,
0D → 0x37, 11 → 0x1E, 12 → 0x1F, 13 → 0x20, 14 → 0x27, 15 → 0x28, 16 → 0x2B, 1A / 1B → 0x25, 21–29 / 4B / 4C →
0x16, 41 → 0xB, 48 → 0xC / 0x38 alternately, 4A → 0x21. Banks duplicate shared effects (weapons) rather than
loading a common bank.
