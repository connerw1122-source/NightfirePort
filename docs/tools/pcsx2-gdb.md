# PCSX2 with a GDB server (live watchpoints)

The field map's live census needs every instruction that reads or writes a field during real play. Official PCSX2
(v2.8.2) has no remote debugger. The [snowyegret23/pcsx2](https://github.com/snowyegret23/pcsx2) fork adds a GDB
Remote Serial Protocol server for the EE (127.0.0.1:10501) and the IOP (10502), plus `qPcsx2:` commands
(watchpoints, pause/resume, save states, screenshots). Its MCP client is [PCSX2_MCP](https://github.com/snowyegret23/PCSX2_MCP) (MIT).

## The local build (2026-10-12)

- Source: `C:\Users\Yoda\Desktop\pcsx2-gdb` (the fork at 7235a04 = official PCSX2 056de99 + 11 commits). The fork's diff
  against official was reviewed before building: the servers are hard-coded to 127.0.0.1; no process launching or
  downloads; file deletions only in its own time-save pruning window.
- **Local fix**: `pcsx2/DebugTools/GDBServer.cpp` `split_args` kept a `string_view` into a destroyed temporary, so every
  argument after the first was garbage ("ERR invalid watchpoint type"). Fixed to own the string.
- Dependencies: PCSX2's own `build-dependencies.bat` (official sources, SHA-256 pinned). A local copy
  `build-dependencies-nf.bat` asks vswhere for an install with the C++ tools and has CRLF line endings: the clone's
  LF endings made `cmd` silently skip half the script. `build-shaderc-only.bat` re-runs shaderc with retried renames,
  because Defender briefly locks freshly extracted folders.
- PCSX2: `build-pcsx2-nf.bat` (CMake + Ninja, Release, Qt).
- Install: `C:\Users\Yoda\Downloads\PS2\PCSX2-gdb` (portable; copies of the user's BIOS, inis, memcards and the resume
  state; `[EmuCore] EnableEEGDBServer/EnableIOPGDBServer = true`, ports 10501/10502; PINE stays on 28011, so the
  existing tools work against either PCSX2, one at a time). The user's own PCSX2 is untouched.

## Use

`python -I tools/trace/gdbwatch.py <start> <end> <read|write|readwrite> --seconds N` prints every instruction that
touched the range, named with ACTION.ELF's symbol table. Notes:
- Attaching pauses the VM, and the server's `D` (detach) pauses it too; the tool ends with `qPcsx2 resume`.
- Watchpoints use "break" mode (each hit pauses, the PC is read with `p25`, then continue). "log" mode prints through
  `DevCon`, which only exists in PCSX2 developer builds.
- `debug_registers` overflows the server's packet buffer; read single registers with `p<n>` (PC = 0x25).
- While a busy field is watched the game runs below full speed (each hit is a round trip).
- The resume save state sits on the in-game pause menu: load a level first (`tools/trace/goto_level.py 07000005`).
