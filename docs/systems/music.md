# Music

Build: ACTION.ELF `e7ffdcb3…`, SFX.IRX `65c4c97e…`. Tags: [M] code, [D] disc, [L] snapshot, [S] suggested. Written by a
research pass (subagent, 2026-10-09) from the decompilation and disc files; the port re-checked
`DroneFunc_CheckAlarmRaised` (0x1453a8) and `NDrone2_SeenAndAttacking` (0x145150) here. Port: `nf_formats::music`
(MXI, SMF, SSD, MapMusic / MapDroneData), `nf_audio::music::Player` (the IOP stream player), `nf_game::music`
(event board, `UpdateMusicalEvents`, `UpdateMFX_01CastleA`, the attack gate). Helper: `tools/audio/smf.py`.

## Port status (WORKLOG 0068)

Done: track → files, SMF / SSD, the buffer plan of `UpdateMusicMarkers` (laps, loop / goto / end, deferred jumps cut
at the last eligible marker of the first buffer after Transfer, instant jumps 0x4000 ring bytes ahead), the reported
section, the music volume (square law, base volume, master 70), the EE event board and hysteresis, the 07000005
updater, round-robin combat / stealth picks, events from the attack gate (`MusicVars` 2 / 2 s / 35, per-frame reset),
the alarm channels (0xA4 → 3 for 58 frames; 0x37 / 0x61 → 1 for 598), Bond's death, cut-scene camera start / end.
*Simplified* [S]: `HasJustJumped` lockout = one ring lap after the cut (the game arms a CanJump test marker); the
instant jump is reported at once (the game reports it when the SPU copy completes, a few frames); reads complete in
the frame they are planned. Not yet: events 3 / 4 / 7 / 8 / 0xD (level exit, room triggers, objectives, creep / wire —
those systems are not ported), the per-room `cel+0xA1` events, MusicTrigger objects (event 2 = 2), the ducker, fades.

---


Tags: **[M]** read in code (function@address; EE = ACTION.ELF, IOP = SFX.IRX), **[D]** measured on disc files,
**[L]** read from the snapshot `work/ee_start_05.bin`, **[S]** inference.
Struct and field names come from the SFX.IRX `.mdebug` stabs. gp-relative names come from SFX.IRX `.symtab` (gp = 0x3B880).
IOP integer math is 32-bit signed with C truncating division.

Scratch helpers (scratchpad, not part of the repo):
- `smf.py <file.SMF>`: dumps the SMF.
- `graph.py <file.SMF>`: section flow graph.
- `ssd.py` / `dec.py <file.SSD>`: frame histogram / PS-ADPCM decode check.
- `syms.py` / `annot.py`: IRX symtab and gp annotation.

---------------------------------------------------------------------------------------------------
## 0. Corrections to docs/systems/sound.md §6

1. A music track maps to **`/PS2/MUSIC/MFX_<n>>4>/MFX_<n>.SMF` + `.SSD`**, where `n = FindMusicTrackNumber(track)` = index+1 in MFXINFO.MXI.
   It does **not** go through STREAMS.LUT / STREAMS.BIN. Those two files serve only negative (sound-effect) stream ids.
   - Level 07000005 uses track 15 → n = 1 → **`PS2/MUSIC/MFX_0/MFX_1.{SMF,SSD}`** [M SFXInitialiseStreamUpdate@92dc, D].
   - MFX_15 is track 30 (levels 07000013 and 07000023), not track 15.
2. MapMusic `+0x10` is the **[7] = mission failed by objective** marker (switch 99), not "complete".
   **[8] = all objectives complete** (switch 100) [M Mission_Update@182c60, Mission_MonitorObjectives@1827d8].
3. Event [2] values seen in code: 5 (NDrone2_SeenAndAttacking), 3 (07000005 alarm), 1 (other alerts), and 2 (only from a `MusicTrigger` map object whose event index is 2).
   - Values 5 / 3 / 1 all take the same "alerted" branch.
   - Only value 2 is the instant "under attack" with the 600-frame bonus [M UpdateMusicalEvents@114e70, MusicTrigger_Update@1b31a0].
4. The SFXJumpToMusicMarker flag means "**jump instantly**" (UpdateInstantPlus). 0 means "at the next marker", unless the target section is itself flagged instant [M].
5. "Markers" in the EE API are **section numbers** (indices into the SMF start-marker table), not raw marker indices [M].
6. SFXMuteMusic (ES_MuteMusic) does **not** affect the music stream. It only scales the music-class SFX ids 0x1D7 / 0x470 [M ES_MusicSetVolume@8218, ES_CombineVolumes@1300].

---------------------------------------------------------------------------------------------------
## 1. Track → files

### 1.1 MFXINFO.MXI (`/PS2/MFXINFO.MXI`, 600 bytes) [M SFXInitialiseAudioStreamSystem@65d8, D]
- 150 little-endian s32, copied verbatim into `MusicTrackInfo[150]` (IOP 0x178A4).
- Loaded once, guarded by `HaveCalledOnce.248` (gp-0x7f44), from `SFXFilePath` = `\PS2\` + `MFXINFO.MXI`.
- Contents [D]: `15,16,17,18,19,20,21,22,23,24,36,34,35,25,30,39`, then 134 × −1.

`FindMusicTrackNumber(t)`@6288 = first index i with `MusicTrackInfo[i] == t`, returned as **i+1**; −1 if none [M].

| EE track | n | files | EE track | n | files |
|---|---|---|---|---|---|
| 15 | 1 | MFX_0/MFX_1 | 24 | 10 | MFX_0/MFX_10 |
| 16 | 2 | MFX_0/MFX_2 | 36 | 11 | MFX_0/MFX_11 |
| 17 | 3 | MFX_0/MFX_3 | 34 | 12 | MFX_0/MFX_12 |
| 18 | 4 | MFX_0/MFX_4 | 35 | 13 | MFX_0/MFX_13 |
| 19 | 5 | MFX_0/MFX_5 | 25 | 14 | MFX_0/MFX_14 |
| 20 | 6 | MFX_0/MFX_6 | 30 | 15 | MFX_0/MFX_15 |
| 21 | 7 | MFX_0/MFX_7 | 39 | 16 | MFX_1/MFX_16 (credits: P_CREDITS calls SFXStartMusic(0x27, 0)) |
| 22 | 8 | MFX_0/MFX_8 | | | |
| 23 | 9 | MFX_0/MFX_9 | | | |

### 1.2 Paths [M SFXInitialiseStreamUpdate@92dc, SFXSetFilePath@1b18, .data dump]
- `StreamFileName = sprintf("%sMFX_%d/MFX_%d.SSD", MFXFilePath2, n>>4, n)` and `MarkerFileName = sprintf("%sMFX_%d/MFX_%d.SMF", MFXFilePath, n>>4, n)`.
- `MFXFilePath` and `MFXFilePath2` are both `\PS2\MUSIC\` (initial .data value, and also rewritten by SFXSetFilePath, op 0x14).
- So the disc paths are `/PS2/MUSIC/MFX_0/MFX_1.SMF` and `/PS2/MUSIC/MFX_0/MFX_1.SSD` [D manifest].
- Negative stream ids are not music. For them, index = −1−id into STREAMS.LUT (3000 × {MarkerOffset, MarkerSize, AudioOffset, AudioSize}) inside `/PS2/<LANG>/STREAMS/STREAMS.BIN`, mono, 22050 Hz (0x5622).
- If the track is not in the MXI, FindMusicTrackNumber returns −1 and the IOP plays STREAMS entry 0 as mono [M]. This is an edge case and never happens in-game.

### 1.3 Stream slot setup for music [M SFXStartMusic@6eb4, SFXInitialiseStream@9030, SFXInitialiseStreamUpdate@92dc, ES_InitialStreamBuffer@68fc]
`SFXStartMusic(track, section)`:
- `MusicStartSection = section`; `SFXActiveData.MusicLastJump (IOP 0x160c4) = SFXActiveData.MusicNowPlaying (0x160cc) = section`.
- `SFXInitialiseStream(track, Is3d=0)`. Music always gets slot 0 (`Streams[0]`, 0x3FC bytes), searching from 0 for Status == 0. Status = 1 (queued), KeyOn = 0.
- `UpdateHashCode = -1`, `UpdateInstantPlus = 0` (any pending jump request is cancelled).

SFXInitialiseStreamUpdate runs synchronously inside that call:
1. The StreamData is memset, keeping its buffer pointers. Then: `HashCode = n`, `Channels = 2`, `Frequency = 32000`, `KeyOn = 1`, `MusicStream = 1`, `BufferSize = PlayBackBufferSize = 2*0x3C00 = 0x7800`, `StreamJumpTo = -1`.
2. The SMF is loaded whole (psiSFXLoadFile) → `StreamMarkerHeader`. Status = 3.
3. `BaseMusicMarker = hdr + hdr.MarkerOffset`; `BaseMusicMarkerStart = hdr + hdr.StartMarkerOffset`.
4. The SSD is opened. `MusicMarkerStart = BaseMusicMarkerStart + section*52`; `MusicMarker = BaseMusicMarker + MusicMarkerStart.MarkerPos*32`.
5. `FilePos = StreamAlign(MusicMarkerStart.Marker.Pos)`, where `StreamAlign(x) = x & ~0xFF` (round down to 256).
6. `HasJustLoadedJump = 1`.
7. `BaseVolume = hdr.BaseVolume`; `ShutDownVolume = 100`; ES_MusicSetVolume().
8. Status = 2 (running); `FinishedInitialise = 1`.

Hardware (created once): one stereo playback buffer `psiStreamCreatePlaybackBuffer(32000 Hz, 2 ch, 0x7800)`.
- That is two SPU voices, each with a 0x3C00-byte ADPCM ring.
- Voice pitch = `(32000<<12)/48000` = 0xAAA.
- Left voice pans fully left, right voice fully right.
- Plus 8 file buffers of 0x7800 bytes (`Read[0..7].DataBuffer`).
- Music uses `FileBufferCount = 8`, `LimitBufferMax = 7` (ES_SetLimitBufferMax@6cb8) [M].

---------------------------------------------------------------------------------------------------
## 2. File formats

### 2.1 .SMF (marker file) [M stabs + code, D all 16 files]
All little-endian s32. Layout: header, then start-marker table, then marker table. Verified on all 16 files: StartMarkerOffset = 20, MarkerOffset = 20 + 52·StartMarkerCount, file size = MarkerOffset + 32·MarkerCount [D].

**StreamMarkerHeaderData** (20 bytes)

| off | field | use |
|---|---|---|
| 0x00 | StartMarkerCount | not read by the driver (no bounds checks) |
| 0x04 | MarkerCount | not read |
| 0x08 | StartMarkerOffset | file-relative offset of the start table |
| 0x0C | MarkerOffset | file-relative offset of the marker table |
| 0x10 | BaseVolume | 0..100 stream base volume (99 or 94 on disc) |

**MusicMarkerData** (32 bytes, the marker table; markers are sorted by Pos)

| off | field | meaning |
|---|---|---|
| 0x00 | Name | section number this marker belongs to. For type 6 / 7 the section of the *target* is taken from the target marker's Name |
| 0x04 | Pos | **byte offset in the .SSD file** (interleaved stereo bytes). The driver always uses `StreamAlign(Pos)` (rounded down to a 256-byte block) |
| 0x08 | MType | see below |
| 0x0C | Flags | always 2 on disc, not read [D, M] |
| 0x10 | Extra | always 0, not read |
| 0x14 | LoopStart | for type 6 / 7: SSD byte position to continue from (= Pos of the target marker). Present but **unused** on type 0 / 10 markers |
| 0x18 | MarkerCount | its own index (or the previous one); not read |
| 0x1C | LoopMarkerCount | for type 6 / 7: index of the target marker (scan resumes there) |

**MType** values [M UpdateMusicMarkers@b560, ES_CheckEndMarker; D census over all 16 SMFs: 0 × 2038, 10 × 303, 6 × 77, 7 × 61, 9 × 61, no 5]:
- **0 = bar / beat point.** A legal jump point ("jump-eligible").
- **10 = section start.** Also jump-eligible. It does nothing when playback flows through it, and the "current section" reported to the EE does not change.
- **6 = loop.** Jump-eligible. On reaching it, data continues at `StreamAlign(LoopStart)` and the scan resumes at `LoopMarkerCount`. Used for a section's own loop.
- **7 = goto.** **Not** jump-eligible; otherwise the same as 6. It ends a one-shot sting and continues in another section, e.g. stealth sting 6 → section 1.
- **9 = end.** Playback stops (key-off, stream closed) when the playhead passes it.
- **5 = pause.** Not present on disc. The stream pauses there until the next jump request, which then plays instantly (but see §3.8 quirk).

**MusicMarkerStartData** (52 bytes, the start table; one entry per section, indexed by section number)

| off | field | meaning |
|---|---|---|
| 0x00 | Marker (MusicMarkerData copy) | `.Pos` = section start position. Other fields copy the marker |
| 0x20 | MarkerPos | index into the marker table of the section's first marker |
| 0x24 | IsInstant | ≠ 0 (−1 on disc) → a jump *to* this section is always instant |
| 0x28 | InstantBuffer | 0 on disc, not read |
| 0x2C | State[2] | ADPCM predictor seed for software-ADPCM streams. 0 on disc; ignored for music (SoftwareAdpcm = 0) |

### 2.2 .SSD (audio) [M psiStreamMemCpy@13e10, psiStreamCreatePlaybackBuffer@12e60, SFXInitialiseStreamUpdate; D MFX_1]
- Raw headerless **Sony PS-ADPCM** (VAG body), **stereo, 32000 Hz**.
- **Interleave: 0x100-byte blocks = 0x80 bytes L (8 frames) + 0x80 bytes R**.
- 16-byte frame: `b0 = filter<<4 | shift`, `b1 = flags`, 14 data bytes = 28 samples. Standard filter table {0,0},{60,0},{115,−52},{98,−55},{122,−60}.
- Marker Pos values count interleaved file bytes, so `seconds = Pos/2/16*28/32000`.
- Loop / flags:
  - The driver ignores the file's flag bytes and builds the loop itself: it ORs flags 6 (loop-start + repeat) into ring frame 0 and 3 (end + repeat) into the last frame of each per-channel 0x3C00 ring, so the SPU loops the ring forever.
  - On disc all flag bytes are 0 except four frames in the last block (L: 1, 7; R: 1, 7), which lie past the last marker and are never streamed [D].
- MFX_1.SSD measurements [D]:
  - 10 749 696 bytes = 41 991 blocks exactly; 5 374 848 bytes/channel = 9 405 984 samples = 293.94 s.
  - Filter nibbles 0..4 only. Decodes to distinct L/R channels: RMS about 2.2k for section 1, 4k for section 5, 6–7k for the end stings, peak 32061.
- Every SSD on disc is a multiple of 256 bytes [D].

### 2.3 MFX_1 (07000005) section graph [D graph.py]
Positions are 256-aligned SSD bytes; times are at 32 kHz.

| sec | instant | start (marker) | flow |
|---|---|---|---|
| 0 | yes | 1024 (M0) | intro, flows into sec 1 at M3 (10.6 s) |
| 1 | no | 387072 (M3) | **stealth / ambient loop**: M90 type 6 → back to 387072 (96.7 s loop; 88 bar markers about 1.11 s apart) |
| 2 | yes | 3921920 (M91) | flows into sec 3 (16.7 s) |
| 3 | no | 4531968 (M102) | loop: M113 → 4531968 (20.5 s) |
| 4 | yes | 5283328 (M114) | **combat intro A** (4.3 s), flows into sec 5 |
| 5 | no | 5442048 (M115) | **combat loop**: M149 → 5442048 (86.7 s) |
| 6 | no | 8614400 (M150) | **stealth sting A** 8.1 s → M152 type 7 → sec 1 |
| 8 | yes | 8911104 (M153) | end sting 4.9 s → M154 type 9 END |
| 9 | yes | 9104384 (M155) | end sting 4.7 s → END |
| 10 | yes | 9287424 (M157) | end sting 5.4 s → END |
| 11 | yes | 9488896 (M159) | end sting 5.3 s → END |
| 12 | yes | 9710080 (M161) | **combat intro B** 4.0 s → M162 type 7 → sec 5 |
| 14 | yes | 9855488 (M163) | **combat intro C** 4.3 s → M164 type 7 → sec 5 |
| 16 | no | 10014464 (M165) | **stealth sting B** 6.9 s → M167 type 7 → sec 1 |
| 18 | no | 10267904 (M168) | **stealth sting C** 5.4 s → M169 type 7 → sec 1 |
| 20 | yes | 10506752 (M170) | 3.3 s into sec 21 |
| 21 | no | 10628608 (M171) | loop M172 → 10628608 (3.1 s) |

- Sections 7, 13, 15, 17 and 19 are start entries that sit on a type-7 goto marker (zero length).
- Section 22 starts at the last marker with no terminator, so the scan would run off the table. It is unused here.
- Full marker dump: `python -I smf.py work/audio/MFX_1.SMF`.

---------------------------------------------------------------------------------------------------
## 3. IOP music stream player (SFX.IRX)

### 3.1 Data [M stabs]
StreamData offsets used below:

| off | field | | off | field |
|---|---|---|---|---|
| 0x08 | Status | | 0x98 | EndingSoon |
| 0x20 | PauseNow | | 0x9C | DataEndMarker |
| 0x24 | Paused | | 0xA8 | FilePos |
| 0x28 | GamePausedMe | | 0xB8 | BufferSize |
| 0x2C | BaseVolume | | 0xBC | PlayBackBufferSize |
| 0x34 | MusicStream | | 0xC0 | PlayBackPos |
| 0x48 | StartedPlaying | | 0xC4 | LastPlayBackPos |
| 0x4C | AllowStartedPlaying | | 0xC8 | Read[8] (92 bytes each) |
| 0x50 | KeyOn | | 0x3B4 | ReadBufferNumber |
| 0x64 | SetJumpRequestCompleteTimer | | 0x3B8 | TransferBufferNumber |
| 0x68 | SetEndMarker | | 0x3C8 | ShutDownVolume |
| 0x6C | SetCanJumpMarker | | 0x3D4 | MusicMarker* |
| 0x70 | SetJumpRequestCompleteMarker | | 0x3DC | MusicMarkerStart* |
| 0x74 | EndMarker (TestMarker) | | 0x3E8 | StreamJumpTo |
| 0x80 | CanJumpMarker | | 0x3EC | HasJustJumped |
| 0x8C | JumpRequestCompleteMarker | | 0x3F0 | HasJustRequestedJump |
| | | | 0x3F4 | HasJustLoadedJump |
| | | | 0x3F8 | HasJustRequestedLoadedJump |

StreamReadData `Read[i]` fields:

| off | field |
|---|---|
| +0x00 | DataBuffer |
| +0x04 | FilePos (file position *after* this buffer, i.e. where the next read starts) |
| +0x08 | ByteOffset (start offset of this buffer inside the 0x7800 playback ring) |
| +0x0C | ByteSize |
| +0x10 | SetToBlank |
| +0x14 | NoDataToRead |
| +0x30 | EndMarker |
| +0x3C | CanJumpMarker |
| +0x48 | JumpRequestCompleteMarker |
| +0x54 | MarkerPassed |
| +0x58 | MarkerPassedByteSize |

- TestMarker = {CheckMe, EndMustToggle, EndHere (ring offset)}.
- Globals: `UpdateHashCode` (gp-0x7f3c, the requested section) and `UpdateInstantPlus` (gp-0x7f38, the instant flag).
- The two s32 values at IOP 0x160c4 / 0x160cc sit at the tail of `SFXActiveData`, which every RPC returns to the EE at 0x24ff80. EE 0x2500c4 = "**IOPLastJump**" (accepted / scheduled target) and EE 0x2500cc = "**IOPNowPlaying**" (completed target) [M].

Status values: 0 free, 1 queued, 3 loading header, **2 running / idle**, **4 file read in flight**, **5 instant-jump read in flight**, **9 instant-jump SPU copy**, 10 stopped.

### 3.2 Units and timing [M]
- Ring = `PlayBackBufferSize` = 0x7800 "interleaved bytes" = 0x3C00 bytes per channel = 26 880 samples = **0.84 s** at 32 kHz.
- `PlayBackPos = psiStreamGetPlayPos()` = `(SPU NAX of L voice − ring start) * 2`. It is a ring offset in interleaved bytes, sampled once per IOP SFXUpdate (once per EE frame), and is 0 while the voice is silent.
- psiSFXGetTimer is ms in 100 ms steps. Only fades and ducker use it.
- One EE frame (60 Hz) ≈ 533 samples ≈ 610 interleaved bytes of playhead advance.

### 3.3 Per-frame order [M SFXUpdate@4ee4 → SFXUpdateStreams@a1a0]
EE VBlank N:
1. The EE batch is sent: commands queued by the previous frame's UpdateMusicalEvents (0x11 start, 0x12 stop, 0x13 jump, 0x15 volume, 6 / 7 fades), then `0x19 NisPlaying, NisQuitten`, then `4` = IOP SFXUpdate.
2. The IOP RPC thread executes them in order.
3. SFXUpdate: ES_UpdateDucker, SFXFadeUpdate, … then SFXUpdateStreams. SFXUpdateStreams does, in order:
   1. ES_InitialStreamBuffer.
   2. **SFXJumpToMusicMarkerUpdate**.
   3. SFXInitialiseStreamUpdate.
   4. **ES_MusicSetVolume**.
   5. For each live stream:
      - ES_SetLimitBufferMax; **ES_CheckFileBuffersForJumpMarkers**.
      - Underrun / timeout bookkeeping.
      - CanJump / JumpRequestComplete / End test markers.
      - Read scheduling via **UpdateMusicMarkers**; read completion; SPU copy start; arming of test markers.
      - Instant-jump states 5 / 9; key-on.
   6. ES_UpdateStreamMemCpy (one ≤0x1800-byte SPU chunk per stream per frame).
4. Reply = SFXActiveData. The EE then runs UpdateMusicalEvents, which reads `0x2500cc`.

The EE therefore sees IOP state with a one-frame lag, and its commands act one frame later.

### 3.4 Steady-state streaming (what plays from where) [M UpdateMusicMarkers@b560, ES_HandleStreamBufferLoaded@9fe8, ES_UpdateStreamMemCpy@8778, SFXUpdateStreams]
- File buffers form an 8-slot ring indexed by `ReadBufferNumber` (next to fill) and `TransferBufferNumber` (next to copy to SPU).
  - Reading continues while `Read+1 != Transfer`, so up to **7 buffers** are queued, about 5.9 s of read-ahead.
  - Each buffer covers the playback ring from its `ByteOffset` to the ring end. Size = `0x7800 − ByteOffset` (capped at BufferSize). In steady state that is a whole ring lap (0.84 s).
- **UpdateMusicMarkers(stream, jumpStart)** prepares buffer `R = ReadBufferNumber` (called when Status == 2 and file access is not suspended):
  1. Clear R's three test markers and MarkerPassed. If jumpStart, `R.ByteOffset = 0`. Then `size = min(0x7800 − R.ByteOffset, 0x7800)`; `R.ByteSize = size`.
  2. If `EndingSoon`: `R.ByteSize = min(size, 0x1800)`, `SetToBlank = 1` (silence is written instead of reading). Return.
  3. Skip markers with `Pos < FilePos`.
  4. For each marker `m` with `m.Pos < FilePos + size`, in table order:
     - If `m.MType != 7`:
       - If `HasJustLoadedJump` and the stream-level CanJumpMarker is idle: arm `R.CanJumpMarker` at `R.ByteOffset + R.ByteSize`; `HasJustLoadedJump = 0`.
       - If `HasJustRequestedLoadedJump`: `HasJustLoadedJump = 1`; `HasJustRequestedLoadedJump = 0`.
     - If `m.MType == 9 || m.MType == 5`: `R.ByteSize = StreamAlign(m.Pos − FilePos)`. If 5, `PauseNow = 1`. Arm `R.EndMarker` at `ByteOffset + ByteSize`; `EndingSoon = 1`. **Return.**
     - If `StreamJumpTo >= 0 && m.MType != 7` (**pending deferred jump executes at this marker**):
       - `IOPLastJump = StreamJumpTo`; `R.ByteSize = StreamAlign(m.Pos − FilePos)`.
       - `MusicMarkerStart = Start[StreamJumpTo]`; `MusicMarker = Markers[Start.MarkerPos]`.
       - `R.FilePos(next) = StreamAlign(MusicMarker.Pos)`; `StreamJumpTo = -1`.
       - Arm `R.JumpRequestCompleteMarker` at `ByteOffset + ByteSize`; `HasJustRequestedLoadedJump = 1`. **Return.**
     - If `m.MType != 7`: `R.MarkerPassed = 1`; `R.MarkerPassedByteSize = StreamAlign(m.Pos − FilePos)`. This runs for every eligible marker, so the **last** eligible marker in the buffer wins.
     - If `m.MType == 6 || 7` (loop / goto):
       - `R.ByteSize = StreamAlign(m.Pos − FilePos)`; `R.FilePos(next) = StreamAlign(m.LoopStart)`.
       - `MusicMarker = Markers[m.LoopMarkerCount]`; `MusicMarkerStart = Start[MusicMarker.Name]`. **Return.**
  5. Otherwise `R.FilePos(next) = FilePos + size`.
- Then: if `ByteSize != 0`, read `ByteSize` bytes at `FilePos` from the SSD into `R.DataBuffer` (or zero-fill if SetToBlank). Status 4.
- When the read is done (status 4 → 2), `ES_HandleStreamBufferLoaded`:
  - `Read++`.
  - `next.ByteOffset = R.ByteOffset + R.ByteSize`, wrapping to 0 at 0x7800.
  - `FilePos = R.FilePos(next)`.
- **SPU copy** (ES_StartStreamMemCpy / ES_UpdateStreamMemCpy):
  - When the previous copy is finished and Status ∉ {5, 9}, start copying buffer `Transfer` into the ring at its ByteOffset.
  - At most one chunk of ≤0x1800 bytes is copied per frame, and a chunk is written only when the playhead is **not inside** `[dst, dst+chunk]`.
  - In steady state, buffer k+1 is therefore written just *behind* the playhead while buffer k plays. Buffer k's copy completes right after the playhead wraps into buffer k's lap.
  - When a copy completes, `Transfer++`. At the next copy start, the test markers of buffer `Transfer−1` (the buffer whose lap is now playing) are copied to the stream-level EndMarker / CanJumpMarker / JumpRequestCompleteMarker, each only if that stream-level marker is idle. JumpRequestComplete also sets `SetJumpRequestCompleteTimer = 240`.
  - After each chunk, `DataEndMarker` is set to the end of the written data, for underrun detection.
  - Key-on happens once the first chunk is in (`AllowStartedPlaying`).
- **Test markers fire** (ES_CheckEndMarker@9e0c) when the playhead passes `EndHere`:
  - It must first be seen at or before EndHere (or wrap); it then fires when `PlayBackPos > EndHere` or on the next wrap.
  - A marker at EndHere = 0x7800 fires on the wrap.
- **Net playback semantics:** audio plays linearly through the SSD.
  - At a type-6 / 7 marker it continues at the marker's LoopStart.
  - At a type-9 marker the stream is stopped (see 3.7).
  - Type 0 / 10 markers do nothing unless a deferred jump is pending.
  - Every boundary (loop, goto, jump, end) is cut at `StreamAlign(Pos)`, the 256-byte block that contains the marker, so it is sample-exact to that block.

### 3.5 Jump requests [M SFXJumpToMusicMarker@75c0, SFXJumpToMusicMarkerUpdate@75fc, SFXCheckStreamJumpInstant@72d4]
`SFXJumpToMusicMarker(section, instant)` only latches `UpdateHashCode = section`, `UpdateInstantPlus = instant`. The latch persists until the next jump command or SFXStartMusic, so the request is **level-triggered and re-evaluated every frame**:
```
SFXJumpToMusicMarkerUpdate():                         // every IOP SFXUpdate
  req = UpdateHashCode; inst = UpdateInstantPlus
  if (!Streams[0].MusicStream || req == -1) return
  if (req == IOPLastJump) return                      // already scheduled / jumped
  if (req == IOPNowPlaying) return                    // already playing
  if (Streams[0].HasJustJumped && !inst) return       // "Not till 1st Marker passed" lockout
  Streams[0].StreamJumpTo = req                       // latest request overwrites an unscheduled one
  if (Paused && !GamePausedMe) { ES_UnPauseStream; Paused = PauseNow = 0; CheckJumpInstant(stream, 1) }
  else CheckJumpInstant(stream, inst)
  if (StreamJumpTo >= 0) HasJustRequestedJump = 1

SFXCheckStreamJumpInstant(s, inst):
  T = Start[s.StreamJumpTo]
  if ((s.Status == 2 || s.Status == 8) && !s.EndMarker.CheckMe):
     if (s.StreamJumpTo >= 0 && (T.IsInstant || inst)):     // ---- INSTANT JUMP ----
        IOPLastJump = StreamJumpTo
        ReadBufferNumber = (TransferBufferNumber - 4) & 7    // read into a slot 4 past the copy slot
        Status = 5; StreamJumpTo = -1
        MusicMarkerStart = T; MusicMarker = Markers[T.MarkerPos]; FilePos = StreamAlign(T.Marker.Pos)
        UpdateMusicMarkers(s, jumpStart=1)                   // fills Read[R] from the target (ByteOffset 0, 0x7800)
        start async read; HasJustRequestedJump = 1
  if (T.IsInstant || inst) s.StreamJumpTo = -1               // instant requests that could not start are dropped
                                                             // this frame (the latch retries them next frame)
```
- **Deferred jump** (target not instant and flag 0): `StreamJumpTo` stays set. It is executed by:
  - **ES_CheckFileBuffersForJumpMarkers@c2c8** (each frame, only when Status == 2, StreamJumpTo != −1 and the target is not instant):
    - Scan the already-read but not-yet-copied buffers `Transfer+1 … Read−1` in order (**Transfer itself is skipped**; it is being written into the ring for the next lap).
    - At the first buffer `B` with `MarkerPassed`:
      - `Read = B`; clear B's test markers; `B.ByteSize = B.MarkerPassedByteSize` (cut at that buffer's last eligible marker).
      - `IOPLastJump = StreamJumpTo`; set MusicMarkerStart / MusicMarker to the target; `B.FilePos(next) = StreamAlign(target.Pos)`; `StreamJumpTo = −1`.
      - Arm `B.JumpRequestCompleteMarker` at the cut; `HasJustRequestedLoadedJump = 1`; `HandleStreamBufferLoaded`.
      - All later queued buffers are discarded and re-read from the target.
  - **or by UpdateMusicMarkers** at the first eligible marker of a newly read buffer (see 3.4).
- **Effective rule:** a deferred jump takes effect at the first jump-eligible marker (type 0 / 10 / 6; not 7, 9 or 5) whose buffer comes after the next ring lap. It is cut at the *last* eligible marker in that buffer.
  - Because Transfer is skipped, the earliest possible cut is about one to two laps (0.84–1.7 s) after the request.
  - With MFX_1's bars about 1.1 s apart, a stealth transition lands about 1–3 s after the request, on a bar line.
  - Inside sections with no inner eligible markers (e.g. combat intros 12 / 14), it waits for the next section's markers.
  - Jump-at-loop: a pending jump is checked *before* the loop handling, so it overrides a type-6 loop.
- **Instant jump timing** (status 5 → 9 → 2, SFXUpdateStreams):
  1. While the 0x7800-byte target read is in flight, the old music keeps playing (the in-progress copy continues).
  2. When the read is done:
     - `dst = StreamAlign((PlayBackPos/2 + 0x2000) * 2)` → ring offset **0x4000 interleaved bytes (0x2000 per channel = 14 336 samples = 0.448 s) ahead of the playhead**, wrapped.
     - If `dst + size` crosses the ring end, the buffer is split into two: the second part goes to offset 0.
     - `Transfer = R`; the old copy is stopped; copying of the new data starts at dst. Status = 9.
  3. When that copy part is done:
     - `Read++`; start copying the split remainder (if any).
     - **ES_JumpRequestComplete** (`IOPNowPlaying = IOPLastJump`; `HasJustJumped = HasJustRequestedJump (=1)`; `HasJustRequestedJump = 0`).
     - Status = 2; HandleStreamBufferLoaded.
  4. **The new section becomes audible about 0.448 s after the CD read finishes.** Old material plays until then. There is no crossfade; the switch happens at the 256-byte-aligned block boundary.
- **Deferred jump completion:** `IOPNowPlaying` is updated when the playhead passes the cut (stream JumpRequestCompleteMarker). Fallback: 240 IOP frames after that marker was armed (`SetJumpRequestCompleteTimer`). `HasJustJumped = 1` from then on.
- **Lockout (`HasJustJumped`):**
  - While set, non-instant requests are ignored; instant ones still pass.
  - It is cleared when the playhead passes the stream CanJumpMarker. That marker is armed at the end of the read buffer containing the 2nd eligible marker after a deferred jump's target, or the 1st (normally the target's own start marker) after an instant jump or SFXStartMusic.
  - In practice: about one ring lap after the jump point (instant jump), or about the first bar plus up to one lap (deferred).
- Requests equal to `IOPLastJump` or `IOPNowPlaying` are no-ops. The "now playing" section is the last *jump target*. It is **not** updated when playback flows or loops into another section (e.g. 4 → 5, 6 → 1).
- Instant requests are dropped while `Status ∉ {2, 8}` (initialising, or in the middle of another instant jump) or while an end / pause marker is armed (`EndMarker.CheckMe`), and are retried each frame via the latch.

### 3.6 End / pause markers [M SFXUpdateStreams, ES_StreamPauseMarkerPassed@9d30]
- When the playhead passes the stream EndMarker (armed from the buffer that contained a type-9 / 5 marker), one of two things happens:
  - `PauseNow == 0` → **SFXStopStream**: unpause, volume 0, close the file, stop copying, `psiStreamStop` (key-off both voices), free the SMF, Status = 10.
  - `PauseNow == 1` → pause the voices; `Paused = 1`.
- After `EndingSoon`, further "reads" are 0x1800-byte silence, so anything after the end point within that lap is silent.
- Once Status = 10 the stream is dead. ES_MusicSetVolume and jumps no longer apply, and only a new SFXStartMusic restarts music [M ES_IsStreamInitialised@7c68].

### 3.7 Stop, pause, underrun [M SFXStopMusic@79ec, SFXPauseAllStreams@7fe0, SFXUpdateStreams]
- `SFXStopMusic` (op 0x12) = `SFXStopStream(Streams[0])`; `Status = 0`. This is immediate (key-off). The EE always sends it right before SFXStartMusic.
- `SFXPauseAllStreams` / `SFXUnPauseAllStreams` (op 0x16 / 0x17; pause menu MenuManager_Create/Delete, NIS / FMV menus) pause the voices and set `GamePausedMe`. `SFXPause` (op 9) does **not** pause music.
- Underrun: after 30 frames of playback, if no buffer is queued (`Read == Transfer`) and the playhead passed `DataEndMarker`, the music voices are paused ("No Data Pause Music Stream!") and resumed when data arrives. A port reading synchronously never hits this.

### 3.8 Quirks to keep in mind [M]
- No bounds checks on the marker table: a section without a 6 / 7 / 9 terminator (MFX_1 section 22) runs off the table.
- `EndingSoon` is never cleared after a pause marker. Resuming from a type-5 pause would play silence. No SMF on disc has type 5 [D], so this is harmless.
- A deferred request made after an end marker has been read but not yet played could rewind past the end and leave `EndingSoon` set (silence). The EE never does this in 07000005: end requests are instant / flag 1, and MusicWaitForEnd stops all further requests.

### 3.9 Recommended port model [S]
Port §3.4–3.6 literally, against a virtual SPU:
- A per-channel 0x3C00-byte ring (or keep interleaved bytes), 8 file buffers, and a playhead advancing 32000 samples/s.
- Sample `PlayBackPos` once per game frame, before SFXUpdateStreams, as the PS2 does.
- Reads may complete within the same or the next frame (the PS2 took a few frames; one frame of latency is a fine approximation).
- Decode ADPCM when copying into the ring, or decode the SSD on the fly in the audio thread using the same "data queue" order.

This reproduces exactly where loops, jumps and ends cut, and with which latency. Simplified equivalent if needed:
- Deferred jump → at the last eligible marker of the first file buffer that starts after the next ring lap.
- Instant jump → the new audio starts at `align256(playpos + 0x4000)`, about 0.45 s after the request plus read time.

---------------------------------------------------------------------------------------------------
## 4. EE music logic (ACTION.ELF)

### 4.1 Batch commands [M EE wrappers @116858.., IOP sifrpc_server@f600]

| op | EE call | IOP |
|---|---|---|
| 0x11 | SFXStartMusic(track, section) | §1.3 |
| 0x12 | SFXStopMusic() | §3.7 |
| 0x13 | SFXJumpToMusicMarker(section, instant) | latch (§3.5) |
| 0x15 | SFXMusicSetVolume(v) | `MusicMasterVolume = v` (EE copy too); ES_MusicSetVolume |
| 6 / 7 | SFXFadeDown / SFXFadeUp(fadeMusic) | §5 |
| 0x16 / 0x17 | SFXPause / UnPauseAllStreams | §3.7 |
| 0x19 | NisPlaying, NisQuitten (appended by EE SFXUpdate every frame) | NIS flags for SFX |

Commands are queued during frame N and sent at the start of the frame N+1 batch [M EE SFXUpdate@1156c0]. The EE SFXUpdate runs once per VBlank:
1. Append `0x19, NisPlaying, NisQuitten` and `4`.
2. PS2_SFX_SendBatchCommandsToIOP (the reply is copied to SFXActiveData 0x24ff80).
3. UpdateMusicalEvents.

### 4.2 Event board [M Music_Event@1de328, Get_Music_Event@1de350]
- `EventList[64]` @0x2d34f8. `Music_Event(i, v)`: if i < 64, `EventList[i] = v`.
- Cleared at the end of every UpdateMusicalEvents, so events are one-frame pulses; posters re-post every frame.
- Exception: when `SFXPaused` (EE 0x30c92c) is set and no level-start event is present, UpdateMusicalEvents returns immediately and events persist.

Producers [M]:

| idx | value | source |
|---|---|---|
| 0 | levelId | Sound_Ready@1dceb8 (with [1] = 1) at level load |
| 2 | 5 | NDrone2_SeenAndAttacking@145150 (drone combat states; gated by MusicVars counts) |
| 2 | 3 | DroneFunc_CheckAlarmRaised@1453a8, 07000005 only: switch channel 0xA4 on (the window-plate alarm, see triggers.md). Posted on the first **58** frames after it turns on (u16 counter 1→2.., posts while < FRAME_RATE_INT = 60) |
| 2 | 1 | same function for 07000005 / 07000006, after the 0xA4 window: channel 0x37 on → first 598 frames; channel 0x61 on → first 598 frames (counter goes 2..599 while < 600). Also Copter / GT / MiniSub / Sub attack code |
| 2 | 2 | MusicTrigger map object (type 0xC) whose event index is 2, for 30 frames after its channel. None is documented for 07000005 [S] |
| 3 | 1 | Trigger_Activate@1b2318 (level-exit trigger: ResetMap_LevelToLoad + GameFlow) |
| 5 | scriptId | Script_FFwd (NIS skipped) |
| 6 | 1 | Player_CheckForDeath (SP); Mission_Update switch 0x62 (forced fail, kills the player); Script_Play of camera scripts 0x600098a / 0x6000952 / 0x6000991 |
| 7 | 1 | Mission_Update when switch 99 = mission failed (objective failed / FailLabel) |
| 8 | 1 | Mission_MonitorObjectives / Mission_Update when switch 100 = all objectives complete |
| 0xB | scriptId | Script_CameraStart (NIS start) |
| 0xC | scriptId | Script_KillStream (NIS end) |
| 0xD | 1 | Player_Creep@1969d0, Player_Wire@197658 |
| 0xE / 0xF / 0x10 | 1 | Drone_InitComms / Drone_EnableAll, DroneVision / HostageSaved (unused by CastleA) |
| n | 1 | Player_Update: current room cel+0xA1 ≠ 0 → Music_Event(cel+0xA1, 1) (per-room zones, level data) |

### 4.3 MapMusic (0x24f4d8, 35 × 76 bytes) [M UpdateMusicalEvents, PickRandom*; L]
```
+0x00 levelId
+0x04 track                (MXI value → MFX file)
+0x08 startSection
+0x0C onDeathSection       ([6], instant)
+0x10 onFailSection        ([7], instant)
+0x14 endInCombatSection   ([8]/[3] when IOPNowPlaying ∈ combat set, instant)
+0x18 endCalmSection       ([8]/[3] otherwise, instant)
+0x1C alertFramesInit      (MusicUnderAttackCount2 reload)
+0x20 calmFramesInit       (MusicUnderAttackCount reload)
+0x24 combat[5]            (-1 terminated; up to 5)
+0x38 stealth[5]           (-1 terminated)
```
**07000005 entry @0x24f608** [L]:

| field | value |
|---|---|
| levelId | 0x07000005 |
| track | 15 → MFX_0/MFX_1 |
| startSection | 0 |
| onDeath | 11 |
| onFail | 10 |
| endInCombat | 8 |
| endCalm | 9 |
| alertFramesInit | 10 |
| calmFramesInit | 160 |
| combat | {4, 12, 14, −1, 0} |
| stealth | {6, 16, 18, −1, 0} |

Every 07000005 section used is consistent with MFX_1's graph (§2.3):
- combat sets → intros into the combat loop (5);
- stealth sets → stings into the ambient loop (1);
- 8 / 9 / 10 / 11 → end stings (type 9).

The full table dump is in the snapshot (35 entries, see the script line in §7). Snapshot state [L]:
- MusicNowPlaying = 15, MapMusicData = 0x24f608, MusicLastJumpRequest = −1, MusicLastJump = 0.
- IOP_DelayUpdateTimer = 7, MusicUnderAttackCount = 158, Count2 = 10, BonusCount = −3. This is consistent with 3 frames after the start.

### 4.4 UpdateMusicalEvents@114e70 (exact)
```
E = EventList
if (E[1] && E[0]):
   if E[0] == 0x7000048: goto main                       // no music on that level
   find MapMusic entry with levelId == E[0] (35 entries); if none: fallthrough to pause check
   if entry.track == 0: goto main
   NisPlaying = NisQuitten = NisEndedNow = 0
   MapMusicData = entry
   SFXStopMusic()
   MusicLastJumpRequest = -1
   MusicNowPlaying = entry.track; MusicLastJump = entry.startSection
   SFXStartMusic(track, startSection)
   MusicWaitForEnd = 0
   MusicUnderAttackCount = entry.calmFramesInit; MusicUnderAttackCount2 = entry.alertFramesInit
   SfxNisMusicTrigger = MusicWaitForNisSfxTrigger = MusicTemp1..9 = 0
   MusicUnderAttack = WhichStealth = WhichCombat = MusicUnderAttackBonusCount = 0
   (MusicVars ← MapDroneData[levelId] — AI tuning, not audio)
   IOP_DelayUpdateTimer = 10
   goto main
if SFXPaused: return                                       // events kept
main:
 t = IOP_DelayUpdateTimer - 1
 if IOP_DelayUpdateTimer < 0: MusicLastJump = IOPNowPlaying (EE 0x2500cc)   // from the 12th call after start
 if E[0xB]: NisPlaying = 1
 if E[0xC] || E[5]: NisPlaying = 0
 if E[5]: NisQuitten = 5  else if NisQuitten: NisQuitten--
 (E[0xA] → MusicVars = 1; MusicVars bookkeeping — AI only)
 if !NisPlaying:
    if E[2] == 2: MusicUnderAttackBonusCount = 600; MusicUnderAttack = 1
    if E[2] == 0 || E[2] == 2:                             // "calm" frame (2 also counts here!)
        if (BonusCount-- < 0)                              // post-decrement test
           if (UnderAttackCount-- < 0): MusicUnderAttack = 0; UnderAttackCount2 = alertFramesInit
    else:                                                  // 1, 3, 5, ... = alerted
        if (UnderAttackCount2-- < 0): MusicUnderAttack = 1; BonusCount = 0; UnderAttackCount = calmFramesInit
 IOP_DelayUpdateTimer = t
 if E[6]: MusicLastJumpRequest = onDeath;  SFXJumpToMusicMarker(onDeath, 1); SFXFadeDown(0); MusicWaitForEnd = 1
 if E[7]: MusicLastJumpRequest = onFail;   SFXJumpToMusicMarker(onFail, 1);  SFXFadeDown(0); MusicWaitForEnd = 1
 BackupEventList = E
 if !MusicWaitForEnd: per-level updater by MapMusicData.levelId (07000005 → UpdateMFX_01CastleA)
 clear E[0..63]; LastEventList = BackupEventList
```
Hysteresis, worked out [M]:
- **Into combat via alert:** `UnderAttackCount2` (10) is decremented on *every* non-calm frame and is only reloaded when combat clears. MusicUnderAttack therefore becomes 1 on the **12th cumulative alerted frame** (not necessarily consecutive). From then on, *each* alerted frame re-asserts it and reloads `UnderAttackCount = 160`, `BonusCount = 0`.
- **Into combat via E[2]==2:** immediate. BonusCount = 600, then decremented the same frame to 599.
- **Out of combat:** needs consecutive calm frames (an alerted frame reloads Count and Bonus when Count2 < 0).
  - After the alert path: Bonus 0 → it clears on the **163rd** calm frame (1 frame for Bonus, then Count 160 → −1).
  - After E[2]==2 (Bonus 599): Bonus reaches −1 after **601** calm frames, then Count counts down.
  - Count is *not* reloaded when combat clears. If combat clears and E[2]==2 later re-triggers combat without an intervening alert-path trigger, Count is already negative and it clears right after the 601-frame bonus.
- Everything in this block is skipped while NisPlaying (counters frozen).
- `MusicLastJump` = the IOP's completed jump target. It equals startSection until the 12th call after the start, and lags the actual audio change by up to one frame.

### 4.5 UpdateMFX_01CastleA@112d78 (07000005; runs only while !MusicWaitForEnd)
`inCombat(x)` = x ∈ MapMusicData.combat (scan up to 5 entries, stop at −1) = x ∈ {4, 12, 14}.
```
if E[2] == 3 && !MusicTemp4:            // alarm, first time only (per level load)
    MusicTemp4 = 1; MusicTemp3 = 1200; PickRandomCombat(0); return
if MusicTemp3 > 0: MusicTemp3--; return  // 1200-frame (20 s) freeze of ALL CastleA logic incl. [3]/[8]/[4]
if E[0xC] == 0x6000994: MusicTemp1 = 0; MusicTemp2 = 66        // that NIS ended
if MusicTemp2 > 0: MusicTemp2--; return                        // 66-frame freeze
if E[0xB] == 0x6000994 || MusicTemp1:                          // during NIS 0x6000994
    s = inCombat(MusicLastJump) ? 6 : 2
    MusicLastJumpRequest = s; SFXJumpToMusicMarker(s, 0); MusicTemp1 = 1; return
if MusicUnderAttack:      PickRandomCombat(0)
elif E[0xD]:              s = inCombat(MusicLastJump) ? 6 : 2; MusicLastJumpRequest = s; SFXJumpToMusicMarker(s, 0)
elif MusicLastJumpRequest != 2: PickRandomStealth()
else:                     MusicLastJumpRequest = 20; SFXJumpToMusicMarker(20, 0)     // (EE symbol o_left = 0x14)
if E[8] || E[3]:
    s = inCombat(MusicLastJump) ? MapMusicData+0x14 (8) : +0x18 (9)
    MusicLastJumpRequest = s; SFXJumpToMusicMarker(s, 1)
    if levelId != 0x7000006: SFXFadeDown(0)
    MusicWaitForEnd = 1
if !MusicWaitForEnd && E[4]: MusicLastJumpRequest = 9; SFXJumpToMusicMarker(9, 0); MusicWaitForEnd = 1
```
PickRandomCombat(flag)@111dc8:
- If `MusicLastJumpRequest ∉ combat` **and** `MusicLastJump ∉ combat`:
  - `n` = count of combat entries before −1 (max 5);
  - `MusicLastJumpRequest = combat[WhichCombat % n]`; `SFXJumpToMusicMarker(it, flag)`; `WhichCombat++`.
- For 07000005 this is round-robin, not random: **4, 12, 14, 4, …** (restarts at 4 every level load).

PickRandomStealth()@111c70:
- Fires only if `MusicLastJumpRequest ∉ stealth` **and** `MusicLastJump ∉ stealth` **and `MusicLastJump ∈ combat`**, i.e. only when coming *out of combat* (the IOP confirms a combat section is playing).
- Then `MusicLastJumpRequest = stealth[WhichStealth++ % n]`, `SFXJumpToMusicMarker(it, 0)`.
- Sequence **6, 16, 18, 6, …**.

### 4.6 What 07000005 does, concretely [M + D]

| situation | EE request | IOP effect on MFX_1 |
|---|---|---|
| level load (Sound_Ready) | StopMusic; StartMusic(15, 0) | plays sec 0 from byte 1024 (intro, 10.6 s), flows into **sec 1** ambient loop (387072 ↔ 3921920) |
| calm, never fought | nothing (PickRandomStealth needs IOPNowPlaying ∈ combat) | sec 1 loops forever |
| alerted ≥12 cumulative frames, or [2]==2 | PickRandomCombat(0) → 4 (then 12, 14 on later fights) | sections 4 / 12 / 14 are **instant**: switch ≈ 0.45 s after the read; intro 4–4.3 s then **sec 5** combat loop |
| alarm (0xA4) first time | PickRandomCombat(0) immediately + 20 s freeze | as above; nothing else (incl. creep / end triggers) is processed for 1200 frames |
| combat clears (163 / 601+ calm frames) and IOP reports a combat section | PickRandomStealth → 6 (then 16, 18) | **deferred**: cut at a bar (eligible marker) about 1–3 s later; sting 5.4–8.1 s; goto **sec 1** |
| creeping / wire (E[0xD]) | 6 if IOPNowPlaying ∈ combat, else 2 | 6 deferred (sting → sec 1); 2 **instant** (sec 2 → sec 3 loop) |
| first frame after creep stops (last request 2) | 20 | instant → sec 20 → sec 21 loop (3.1 s). Next frame PickRandomStealth does nothing (not from combat), so this stays until combat or creep |
| NIS 0x6000994 start (E[0xB]) | 6 if in combat, else 2 (every frame while MusicTemp1) | as above |
| NIS 0x6000994 end (E[0xC]) | 66-frame freeze; then normal logic (last request 2 → request 20) | sec 20/21 |
| player death / forced-fail switch 0x62 / fail camera scripts (E[6]) | jump(11, 1) + FadeDown(0) + WaitForEnd | instant sec 11, 5.3 s sting then END (stream stopped). No more music logic |
| objective failed, switch 99 (E[7]) | jump(10, 1) + FadeDown(0) + WaitForEnd | instant sec 10 (5.4 s) → END |
| objectives complete (E[8]) or level-exit trigger (E[3]) | jump(8 if IOPNowPlaying ∈ {4, 12, 14} else 9, 1) + FadeDown(0) + WaitForEnd | instant end sting → END |
| E[4] (room / music trigger) | jump(9, 0) + WaitForEnd | 9 is instant → END |

- "In combat" always means the last *jump target* reported by the IOP (4 / 12 / 14). It stays true after flowing into sec 5. It is false right after the request until the IOP completes the jump (≥2 frames).
- SFXFadeDown(0) fades **only SFX** (not music) over 2 s. The end sting plays at full music volume.

---------------------------------------------------------------------------------------------------
## 5. Music volume [M ES_MusicSetVolume@8218, psiStreamSetVolume@13364, PS2_GetSampleVolume, SFXFadeUpdate@f8c, ES_UpdateDucker@8438]
Recomputed every IOP frame (in SFXUpdateStreams and whenever the ducker moves), only while the stream is initialised:
```
v = MusicMasterVolume * DuckerCurrentVolume * BaseVolume / 10000      // 70*100*99/10000 = 69
v = v * FadeMusicVolume / 10000                                       // 10000 unless a fadeMusic fade
v = v * ShutDownVolume / 100                                          // always 100 for music
level = v*v / 100                                                     // square law: 69 → 47
SFXMode != 0 (default 4): Lvoice.VOLL = GSV(level), Lvoice.VOLR = 0; Rvoice.VOLL = 0, Rvoice.VOLR = GSV(level)
SFXMode == 0 (mono):      both voices VOLL = VOLR = GSV(level/2)
GSV(x) = clamp(x*0x3FFF/100, 0, 0x3FFF)                               // 47 → 7700 (0x1E14)
```
- At 07000005 start [L]: MusicMasterVolume = 70 (`music_slider*5`), BaseVolume = 99 (MFX_1 header), Ducker 100, Fade 10000.
  So **level 47, SPU 7700 / 16383 ≈ 0.470 linear** per channel. SPU main volume = GSV(100) = 0x3FFF.
- Treating 0x3FFF as unity: `gain = (v*v/100)*0x3FFF/100/0x3FFF`.
- Integer truncation at every step matters for exactness (69, not 69.3).

**Fades** (op 6 / 7):
- `SFXFadeDown(m)` starts only if FadeStatus ≠ −1 and FadeVolume ≠ 0. Then `FadeVolume = 10000 − 5*(t−t0)` (t in ms, 100 ms steps), clamped → 2 s.
- Music follows only if `m != 0` (`FadeMusicVolume = FadeVolume` while fading).
- Gameplay uses FadeDown(0) (SFX only). Menus use FadeDown(1) (GCPAUSE quit, end-mission, map screens).
- Quirk: after a finished FadeDown(0), FadeVolume = 0, so a later FadeDown(1) is ignored and music is not faded.
- FadeUp is never called by the EE. SFXReset → ES_FadeReset restores 10000.

**Ducker** (from SFX with a `Ducker` parameter; see sound.md Part 2 §5.5):
- An SFX start with `P.Ducker` and TrackingType 0 / 3 calls `ES_StartDucker(100−Ducker, DuckerLength ms)`.
- Each IOP frame: `s = (target − cur)*1024`; if s ≠ 0: `s = (passed ? s/15 : s/6) >> 10`, and if that is 0, s = ±1; `cur += s`.
- `passed = (now − start) > Duration`. Once passed, target = 100.
- So music eases toward (100−Ducker)% at 1/6 per frame for DuckerLength ms, then recovers at 1/15 per frame. Negative lengths count as already passed. NisQuitten → StopDucker (Duration 0).
- DuckerCurrentVolume multiplies the music volume *before* squaring. Other SFX get 1/3 of the duck.

---------------------------------------------------------------------------------------------------
## 6. Open points [S]
- Which map objects in 07000005 post E[2]==2 (MusicTrigger), E[4], or per-room `cel+0xA1` events: check the level data. CastleA reacts to indices 2, 3, 4, 8, 0xB, 0xC, 0xD.
- The identity of NIS script 0x6000994 (SP_Update@1dbda8 refuses to start it while E[2] ≠ 0 that frame).
- Real CD read latency on PS2 (a few frames) shifts instant jumps by that amount. A port can use 1 frame.

## 7. Reproduce
```
python -I smf.py  work/audio/MFX_1.SMF          # markers
python -I graph.py work/audio/MFX_1.SMF         # sections
python -I ssd.py  work/audio/MFX_1.SSD          # frame/flag census
python -I dec.py  C:/.../work/audio/MFX_1.SSD   # decode sanity
python -I -c "import struct;d=open('work/ee_start_05.bin','rb').read();print([struct.unpack_from('<19i',d,0x24f4d8+i*76) for i in range(35)])"
```
Disc files extracted to `work/audio/`: MFX_1.SMF/.SSD, MFX_15.SMF, MFX_16.SMF, and `smf/` (all 16 SMFs + ssd_sizes.txt).
