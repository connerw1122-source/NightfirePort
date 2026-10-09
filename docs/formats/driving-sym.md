# `DRIVING/DRIVING.SYM`

Reader: `crates/nf-formats/src/symfile.rs`; dump: `nf-extract drivingsym <iso>` (to `work/`, not committed).

```
(u32 LE address, u32 LE name offset) × n, sorted by address; n = first name offset / 8
NUL-terminated names follow the table
```

*Measured:* 12,719 entries; linker symbols `_umemory_size` (0), `_stack_size` (0x40000), `main` 0x190590,
`_fbss` 0x4e4f80, `_gp` 0x4ecb70, `_end` 0x55cf04, `_heap_size` 0xffffffff; object paths
`d:\ToBurn\BondPS2\PS2_EE_Release\*.obj`; demangled C++ (`PS2PadDevice::PollDevice(void)`,
`InputDevice::DeadZoneChop(float &, float, float)`, `ARaceEngine`, `EAGL_VD_PS2_SINGLE_BUFFERD`).

**It does not describe our DRIVING.ELF** (*measured*): our ELF's `.bss` ends at 0x4279b4 and `$gp` is 0x3cfdf0;
of 7,992 SYM addresses inside our `.text`, 342 follow a `jr ra` and 180 start with `addiu sp,sp,-n`. NightfireResearch
and Raikaru report the same (offsets from an older build; *sourced*). Use names only after matching function shapes
(U-003).
