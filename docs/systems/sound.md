# Sound

Build: ACTION.ELF `e7ffdcb3…`, SFX.IRX `65c4c97e…` (MODULES/SFX.IRX; symbols and stabs types in `.symtab` / `.mdebug`;
decompiled to `work/decomp/SFX.IRX/`). Formats: docs/formats/sound-bank.md. Port: `nf_game::sound` (EE layer),
`nf_audio::sfx` (IOP driver), `nf_audio::Output` (SPU stand-in, cpal). Tags: [M] code, [L] snapshot, [D] disc,
[S] suggested. Two research passes (subagents, 2026-10-09) wrote the sections below from the decompilation; the
port's implementation was checked against them and the positions' unit conversion was re-read here (`SFXStart3D` /
`SFXSetupListener` on the EE pack world units × 1000 as integers [M]).

## Port status (WORKLOG 0063)

Done: banks (load by `Sound_Ready`'s table), the EE pool (deferred start, per-frame positions, grace period, stop),
the driver's start path (`SFXSetup` budget / stealing / per-id limit, random / sequence / polyphonic choice,
`StartSample` pitch / volume / pan with the driver's RNG), the 3-D math with the driver's integer `PS2_SquareRoot`,
`ES_CombineVolumes`, front + phase-inverted rear voices (the rear keyed 500 µs later, WORKLOG 0064), `SFXUpdate` (ends, sequences, loops via cue timers, kill pass,
active list), `SFXRemove` / `SetPositionAndVelocity`; sources: the first-person gun's script sounds (op 1: shot, reload), Bond's footsteps by floor surface, bullet
impacts by surface, guard body hits (0x413), guards' gunfire (`weapon_data+0x60`),
animation-script sounds, pickups, script sounds (moving with their object). Music: docs/systems/music.md. Speech and streamed effects: docs/systems/speech.md. Not yet: casings, the 0x601 body-hit variant (`HITDATA+0x52` = 5), drone foley (`Drone_SfxInfo`, `NDrone2_PlaySFX`), the cel/room-driven `SFXSetEnvironment` value (G-016), the exact SPU2 Gaussian coefficient table (hardware; currently Catmull-Rom 4-tap), precache, pause in cut-scenes, the per-weapon op-1 id remaps (other weapons). **Now ported (WORKLOG 0075–0076):** the ducker, fades, reverb/environment, the SPU ADSR release and 4-tap interpolation (0075), and bullet fly-bys (0076: `Bullet_DoTrails`'s `+0x6C & 0x40000` check, the closest-point-to-viewer test, the `Rand_Rand(5)` 1-in-5 roll, sound 0x1dc — confirmed against the decompile at `0x12d890`).

---

# Part 1: EE layer

Tags: **[M]** = read from code (function @ address), **[L]** = value read from `ee_start_05.bin` (level 07000005 start), **[S]** = inference.
Ghidra parameter names for the float-taking functions are scrambled. All argument mappings below were checked against the MIPS EABI registers in the disassembly (a0..a3,t0.. = ints, f12..f14 = floats).

---------------------------------------------------------------------------------------------------

## 0. Architecture overview

```
game code ──Sound_Play / Sound_Play3D──► DYNAMICSOUNDS node (state 0 = pending)      (no IOP traffic yet)
Game_Run ─► Sound_UpdateListeners (once per game frame, before everything else)       [M Game_Run @1c9790]
              ├─ SFXSetupListener (per viewer)
              ├─ GlobalListenerPos = viewer pos
              ├─ SFXSetEnvironment(room env, underwater)
              ├─ HandleMapSoundAllocation (map ambients start/stop)
              ├─ Sound_UpdateSounds (node state machine → SFXStart/SFXStart3D/SFXSetPositionAndVelocity/SFXRemove)
              └─ SFXUpdate (once per VBlank: NIS state + end marker, RPC flush, TTL decay, UpdateMusicalEvents/music)
```
- Every `SFX*` EE call just appends 32-bit words to `SFXBatchCommands` (0x250340, 8 KB = 2048 words). `SFXBatchPos` (0x30c934) is the word index [M]. If `SFXBatchPos >= 0x79d` a command is dropped and a flush is forced instead (`VBlankCount2 -= 9999; SFXUpdate()`) [M all SFX* wrappers].
- Flush = `PS2_SFX_SendBatchCommandsToIOP` @1155d0: appends opcode 5 (end), `sceSifCallRpc(client 0x2502e8, fn 1, mode 1(nowait), send=SFXBatchCommands, size=pos*4, recv=0x24ff80, 0x154)`, resets `SFXBatchPos=0` and `LimitSFXSetupListenerCount=0` [M]. If the previous RPC is still busy and `AllowSkip` is set, it gives up after 11 T0 ticks; the batch is then kept and sent next time [M].
- Bank loads use a second buffer `SFXBatchCommands2` (0x252340) and a synchronous RPC [M SFXLoadSoundBank @115938].
- Receive buffer `SFXActiveData` @0x24ff80 (0x154 bytes): +0 = count of active IOP SFX, +4.. = pairs {sfxId, handle}; +0x14c (0x2500cc) = current music marker reported by the IOP [M SFXIsSFXPlaying @111b90, UpdateMusicalEvents @114e70].

### Batch opcode table (EE side) [M, from each wrapper]
| op | EE function | payload words |
|---|---|---|
| 0x00 | SFXInitialise | — (hello/init) |
| 0x01 | SFXLoadSoundBank (buffer 2) | bank |
| 0x03 | SFXStart | id, handle |
| 0x04 | SFXUpdate frame end | — |
| 0x05 | end of batch | — |
| 0x06 | SFXFadeDown | flag (0 = music fade at end/death markers, 1 = menus) |
| 0x08 | SFXSetVolume | sfx master volume (0..100) |
| 0x09 / 0x0a | SFXPause / SFXUnPause | — |
| 0x0b | SFXSetPositionAndVelocity | pos.xyz*1000, vel.xyz*1000 (int), handle, id |
| 0x0c | SFXRemove | handle, id |
| 0x0d | SFXStart3D | id, pos.xyz*1000, vel.xyz*1000, handle, volume, inner, outer |
| 0x0e | SFXSetupListener | pos*1000, vel*1000, dir*1000, up*1000, right*1000, listenerIndex |
| 0x0f | SFXReset2 | — |
| 0x11 | SFXStartMusic | track, startMarker |
| 0x12 | SFXStopMusic | — |
| 0x13 | SFXJumpToMusicMarker | marker, flag |
| 0x14 | SFXSetFilePath | path |
| 0x15 | SFXMusicSetVolume | music master volume (0..100) |
| 0x16 / 0x17 | SFXPauseAllStreams / SFXUnPauseAllStreams | — |
| 0x18 | SFXSetNumberOfListeners | n |
| 0x19 | NIS state | NisPlaying, NisQuitten (emitted every VBlank by SFXUpdate, and before SFXStart/Start3D when it changed) |
| 0x1b | SFXSetLanguage | lang |
| 0x1c | SFXSetMode | mode (stereo/mono/...) |
| 0x1d | SFXSetEnvironment | envId, underwaterFlag |
| 0x24 / 0x25 | SFXSuspendFileAccess / SFXUnSuspendFileAccess | — |
| 0x26 | SFXPreCache | id, frames |

Special case: `SFXStart`/`SFXStart3D` with id **0x172** send nothing and only set `SfxNisMusicTrigger = 1` (a music trigger used by NIS) [M SFXStart @115a68, SFXStart3D @115bf0].

`SFXRequestedActiveData` (0x24f230, 40 entries × 12 bytes {id, handle, ttl}): each SFXStart/Start3D stores {id, handle, ttl=5}. SFXUpdate decrements ttl once per VBlank [M]. `SFXIsSFXPlaying(handle, id|-1)` returns true if the handle is in that list with ttl>0 OR in the IOP's active list (`id==-1` means any id) [M @111b90]. A just-started sound therefore counts as "playing" for at least 5 VBlanks, so it cannot be reaped before the IOP has reported it.

---------------------------------------------------------------------------------------------------

## 1. SFXOutputData (0x2c94a0, 0x60D rows × 0x18) — complete field map

| off | type | meaning | evidence |
|---|---|---|---|
| +0x00 | u32 | sfx id (== row index; 0 for 154 unused rows) | [L]/[M ELF data, identical in snapshot] |
| +0x04 | f32 | inner radius (full-volume distance). Fallback 10.0 if id ≥ 0x60D | [M Sound_Play3D @1dd548] |
| +0x08 | f32 | outer radius (silent / cull distance). Fallback 20.0 | [M] |
| +0x0C | f32 | AI alert weight (copied to node+0x48) | [M Sound_Play3D, Sound_Alertness @1ddc48] |
| +0x10 | f32 | **duration in seconds** used for the subtitle display time (`dur * g_fSubtitleDurationModification * 60` frames, default 3.0 s) | [M Sound_DoSubtitle @1dde58] |
| +0x14 | u8 | looped flag | [M Sound_IsLooping @1de260, Sound_UpdateSounds] |
| +0x15 | u8 | **distance-cull flag**: in single player, a one-shot with this flag is not started if listener distance > 1.1 × outer; also limits its subtitle to within 0.5 × outer | [M Sound_ReqestPlaySfx @1dcc68] |
| +0x16..17 | — | padding (always 0) | [L: only flag patterns 00000000×774, 00010000×672, 01010000×81, 01000000×22] |

So 103 rows are looped and 753 are distance-culled [L].

**Is radius/volume sent to the IOP?**
- **Radii:** sent only when the node has the "custom radii" flag (node+0x30 = 1), which needs `Sound_Play3D(..., custom=1)` and a caller radius > 0. Otherwise the EE sends **-1, -1** and the IOP uses the inner/outer stored in its own bank sample header (+8 / +0xC) [M Sound_ReqestPlaySfx; SFX.IRX SFXStart3D @0x3aac]. In the game **only map ambient sounds** pass custom=1 [M: every other Sound_Play3D call site checked has t0=0]. The EE copies of +4/+8 are used only for EE-side decisions (culling, subtitles, AI alertness) [M].
- **Volume:** the EE sends `(int)node.volume` in the Start3D packet, but the IOP's SFXStart3D overwrites it with 100 on entry (`param_5 = 100`) [M SFX.IRX @0x3aac]. SFXStart (2D) has no volume field at all [M]. **Per-sound volume therefore has no effect.** The only volume controls that work are SfxMasterVolume and MusicMasterVolume.
- **Pitch (short param):** stored in node+0x4c and never read [M: the only accesses to +0x4c in the sound code are stores].

---------------------------------------------------------------------------------------------------

## 2. DYNAMICSOUNDS node and pool

Pool: `DynamicSoundList` DLISTINFO @0x349b00, `DList_Init(0x349b00, size 0x60, count 0x100)` → **256 nodes of 0x60 bytes**, allocated in Sound_Init @1ddf88 [M]. Header: +0 free head, +4 free ptr, +8 u16 freeCount, +0xa u16 nodeSize, +0xc in-use head, +0x10 in-use tail, +0x14 u16 inUseCount [M DList_Init @1cc778 / DList_MoveFromFree2InUse @1cccb8]. New nodes are appended at the tail [M]. At level start: 251 free, 5 in use [L].

**The handle returned to game code is the node pointer** (also used as the IOP handle) [M]. Handles are not generation-checked, so a stale handle can alias a reused node [S].

| off | type | field |
|---|---|---|
| +0x00 / +0x04 | ptr | prev / next (in-use list) |
| +0x10 | VECTOR | position (x,y,z,w) |
| +0x20 | VECTOR | velocity. Always set to CONSTANT_ZERO_VECTOR and never changed afterwards [M] |
| +0x30 | u32 | custom-radii flag (1 = send inner/outer to IOP) |
| +0x34 | u32 | id as passed by the caller, including high flag bits (e.g. 0x40000000 on map sounds). IOP calls use `id & 0xFFFFF` |
| +0x38 | u32 | "ref" (owner tag). Checked by Sound_Stop_Ref / Sound_Playing_Ref (no callers); zeroed when the node is freed |
| +0x3c | f32 | volume (stored, effectively unused, see §1) |
| +0x40 | f32 | inner radius |
| +0x44 | f32 | outer radius |
| +0x48 | f32 | AI alert weight |
| +0x4c | s16 | pitch / variation (never read) |
| +0x4e | u8 | is3D |
| +0x4f | u8 | dirty (set by SetPosition/SetVolume, cleared each update; it does not gate anything) |
| +0x50 | u8 | state: 0 pending, 1 playing, 2 stop requested / finished, 3 freed |
| +0x51 | u8 | age counter (+1 per update) |
| +0x52 | u8 | show subtitle |
| +0x53 | u8 | custom flag copy (write-only) |
| +0x54 | u8 | 0 (write-only) |
| +0x55 | u8 | Sound_FrameDelay snapshot (write-only; Sound_FrameDelay is never set non-zero) [M]/[S] |

Snapshot nodes at level start [L]: 0x10b (3D, in 10 / out 100), 0xff (3D, 4/40), map ambients 0x40000085 (custom 182.6/365.3) and 0x40000174 (custom 160/200), and 0x282 pending 2D (weapon-select sound, state 0).

---------------------------------------------------------------------------------------------------

## 3. EE API

### Sound_Play(uint id, float vol, short pitch, uint ref) → handle  @1dd880 [M]
- a0 = id, f12 = vol, a1 = pitch, a2 = ref.
- `(id & 0xFFFFF) == 0xFFFF` → returns 0. Otherwise allocates a node (returns 0 if the pool is full) with: inner/outer/weight from the table (fallbacks 10/20/0), is3D = 0, custom = 0, state = 0, pos = vel = 0.
- **Nothing goes to the IOP here.** The next Sound_UpdateSounds calls `SFXStart(id & 0xFFFFF, handle)`.
- All callers pass vol = 100.0, pitch = 0, ref = 0 [M grep].

### Sound_Play3D(uint id, VECTOR* pos, float vol, float outer, float inner, short pitch, uint ref, int custom) → handle  @1dd548 [M]
- a0 = id, a1 = pos, f12 = vol, f13 = outer, f14 = inner, a2 = pitch, a3 = ref, t0 = custom. This order was verified in the disassembly at @1dd594..1dd5c4 and at the call sites.
- `custom == 0`: the radii arguments are ignored and the table values are used; node.custom = 0.
- `custom != 0`: inner = arg if > 0, else table +4. Outer = arg if > 0, else table +8. node.custom = 1 if either argument was > 0.
- weight = table +0xC; is3D = 1; state = 0; pos = *pos; vel = 0; +0x53 = custom.
- The callers' "-1.0, -1.0" radii (0xbf800000) mean "use the defaults". Since custom = 0 almost everywhere, any explicit radii are ignored too (DroneWeap passes vol 25 / outer 100; NDrone2_PlaySFX passes Drone_SfxInfo radii × scale) [M].

### Sound_ReqestPlaySfx(DYNAMICSOUNDS*) @1dcc68 (called from UpdateSounds on state 0) [M]
- 2D: `SFXStart(id & 0xFFFFF, handle)`; showSubtitle = 1.
- 3D:
  - `d = |GlobalListenerPos − pos|`. `nListeners = MPSettings+0x180 ? MPSettings+0x1ac : 1`.
  - If the sound is not looped, nListeners == 1, the cull flag (+0x15) is set and `d > 1.1·outer`, it **returns without starting**. The node then dies on the next update, because IsSFXPlaying is false.
  - showSubtitle = (d < 0.5·outer). It is forced to 1 when ScriptCam ≠ 0 or the cull flag is 0.
  - `SFXStart3D(id & 0xFFFFF, &pos, &vel(=0), handle, (int)vol, custom ? (int)inner : -1, custom ? (int)outer : -1)`.

### Sound_UpdateSounds(VECTOR* listener) @1dda70 [M] — walks the in-use list once per frame
For each node: `age++`.
- **state 0** → ReqestPlaySfx; state = 1; if showSubtitle → `Sound_DoSubtitle(id)`.
- **state 1** → liveness test. Looped (table +0x14, using the *unmasked* id < 0x60D) → stays alive. Otherwise `SFXIsSFXPlaying(handle, -1)`; if false → state = 2.
- **state 2** → `SFXRemove(handle, -1)`, node returned to the free list, state = 3, ref = 0.
- Afterwards, if is3D and state == 1: `SFXSetPositionAndVelocity(&pos, &vel(=0), handle, -1)` is sent **every frame** (a newly started sound also gets one in the same frame). The dirty flag is just cleared.
- There is no per-frame distance culling, no delay queue and no restarting. Looped sounds live until Sound_Stop / StopAllWithId. One-shots die when the IOP stops reporting them.
- Quirk: ids carrying high flag bits (map sounds, 0x40000xxx) fail the `< 0x60D` table checks, so they are treated as non-looped and un-culled. Their liveness comes from the IOP, which loops them according to its own sample header [S].

### Sound_Stop(handle) @1de0a8 [M]
`if (h) h->state = 2`. The IOP SFXRemove goes out in the next update. Returns 0.

### Sound_StopAllWithId(id) @1de0c8 [M]
Sets state = 2 on every in-use node whose +0x34 equals `id`, including flag bits. Callers: Player_Init / Player_ChangeSubState (ids 0x12, 0x3d, 0xf0), AnimProcessScriptCmds op 2 / sub 3.

### Sound_SetPosition(handle, VECTOR*) → bool playing @1de108 [M]
Copies pos and sets dirty. Returns looped ? (state == 1) : SFXIsSFXPlaying(handle, -1). The IOP gets the new position via that frame's update. Callers: Bullet_update (projectile loops), Car_Update, Script_Interp (sound stream follows its script object +0xc0).

### Sound_SetVolume(handle, float) @1de190 [M]
Stores +0x3c and sets dirty. Nothing is sent to the IOP, and the function has no callers.

### Sound_IsFinnished(handle) @1de240 [M]
`h ? state == 2 : true`. Note that state 3 (already freed) returns false.

### Sound_IsLooping(handle) @1de260 [M]
`h == 0 → 1`; otherwise table +0x14 for the unmasked id < 0x60D, else 0.

### Sound_SetAlertness(f, h) / Sound_ModAlertness(f, h) [M]
`+0x48 = f` / `+0x48 *= f`. Footstep sounds use SetAlertness(0); Effect_Body uses Mod(0.5).

### Sound_Alertness(VECTOR* p, float* outMax, DYNAMICSOUNDS** outLoudest) → int @1ddc48 [M]
Sum over live (state < 2), non-looped nodes with weight ≠ 0 and `|p − pos|² ≤ outer²` of `w · (d < inner ? 1 : (outer − d)/(outer − inner))`. Special cases: id 0x11c/0x11d are skipped on levels 07000009/0700000a; id 0x248 sets inner = outer and weight = 10. Returns (int)sum and writes the running maximum and the loudest node. Used by DroneFunc_HandleSoundAlerts @148a30 (drone hearing). `Sound_ZeroAlertness` @1de2a8 zeros the weights of all live non-looped nodes; it is called from Drone_InitComms [M].

### Sound_DoSubtitle(id) @1dde58 [M]
Looks up `Snd2Lbl` (0x2d25d8, 0x1E4 × {u32 label|0x80000000 force, u32 sfxId}). Shows the text if subtitles are on (`GameState+0x60`, 0 in [L]) or the force bit is set. Duration = `table+0x10 · g_fSubtitleDurationModification · 60` frames. `g_fSubtitleDurationModification = 0.92` [L].

### Init / teardown [M]
- `Sound_Init` @1ddf88: pool init, `SFXInitialise(0)` (once: bind RPC 0x40013575, then SetLanguage, SetMode, SetVolume(SfxMasterVolume), MusicSetVolume(MusicMasterVolume), op 0), clears EventList.
- `Sound_Ready` @1dceb8 (called by LoaderLoad): `SFXReset`, `SFXSetNumberOfListeners(nListeners)`, `SFXLoadSoundBank(levelBank)`, SoundInit = 1, `Music_Event(0, levelId)`, `Music_Event(1, 1)`. **Level 07000005 → bank 5.** Other banks: 01→1, 02→2, 03→3, 04→8, 06→4, 07→6, 08→7, 09→0x17, 0a→0x18, 0b→0x19, 0c→0x1a, 0d→0x37, 11→0x1e, 12→0x1f, 13→0x20, 14→0x27, 15→0x28, 16→0x2b, 1a/1b→0x25, 21–29/4b/4c→0x16, 41→0xb, 48→0xc/0x38 alternating, 4a→0x21.
- `Sound_Kill`: SFXReset if SoundInit.

### Wrappers [M]
- `MPSound_Play(id)` @18bee0 = `Sound_Play(id, 100, 0, 0)`.
- `Menu_PlaySound(MENU_SOUND s)` @1f3278 maps s to a 2D sound: 0,1,2,7 → 0x1da; 3,5 → 0x1d8; 4,6,8 → 0x1d9. s = 0/1/2/8 are gated by `enabled_sound_flags` bits 1/2/4/8.

---------------------------------------------------------------------------------------------------

## 4. Listener — Sound_UpdateListeners @1dd278 [M]

- `n = MPSettings+0x180 ? (u16)MPSettings+0x1ac : 1`. In single player n = 1 [L: MP+0x180 = 0].
- For i in 0..n-1, using `v = glb_viewer[i]` (0x2d88b0, per-player **camera/viewer** object, not the player body):
  `SFXSetupListener(pos = v+0x150, vel, dir = row2 of matrix v+0x120 (Mat_GetDir, +0x20), up = row1 (+0x10), right = row0 (Mat_GetNorm, +0x00), index i)`.
  The matrix is the camera's world matrix and +0x150 is its translation row, i.e. the eye position.
- Cut-scene override: if `glb_viewer[4]` (DAT_002d88c0) ≠ 0 and its byte +0x29 ≠ 0 (script camera active), a second `SFXSetupListener(..., index 0)` is issued from that viewer.
- Inside SFXSetupListener (EE):
  - The **velocity is always overwritten with zero** (the computed `v+0xc0 − v+0xd0` is discarded).
  - **Only the first SFXSetupListener per batch is encoded**, because `LimitSFXSetupListenerCount < 1` and the counter is reset only by a flush. So in 1P the cut-scene override call is dropped while the player viewer is non-null, and in MP only listener 0 reaches the IOP per batch [M]. Whether that is intended is [S].
- The last viewer used (player viewer, or the script viewer when it is active) becomes `GlobalListenerPos` (0x2d3600) = v+0x150 [M]. All EE distance tests use it.
- Room environment: with `cel = v+4` (current room) non-null: `SFXSetEnvironment(cel->s16 +0x98, underwater = (cel+0x3c & 0x10000000) || env ≥ 20)`, then HandleMapSoundAllocation. Then `Sound_UpdateSounds(v+0x150)` and finally `SFXUpdate()`.
- Snapshot [L]: glb_viewer[0] = 0x1fc6c10; pos (104.954, 25.663, 108.825); right (−0.545, 0, 0.839), up (0, 1, 0), dir (−0.839, 0, −0.545); cel env 0, underwater 0; glb_viewer[4] = 0x1fbf760 with +0x29 = 0 (inactive). GlobalListenerPos equals the viewer pos. LimitSFXSetupListenerCount = 1 (one listener encoded in the pending batch).

---------------------------------------------------------------------------------------------------

## 5. Map ambient sounds

- **Source:** map block id **0x28** in `parsemap_handle_block_id` @1d1020 → `Sound_LoadMapSounds(FileNextBlock + 4)` (only when param_1 ≠ 0) [M].
- **Loader** @1ddff8: `u32 count`, then count × 32-byte records copied verbatim to `GMapSounds` (0x2d3610, capacity 50 = 1600 bytes). `GNumMapSounds` (0x30d07c) = count. `GMapSoundActive[50]` (0x2d3c50) is cleared. `GMapSoundHandle[50]` lives at 0x2d3c88 [M]. There is no bounds check against 50 [M].
- **Record layout** [M HandleMapSoundAllocation @1dd118, offsets confirmed in the disassembly]:

| off | type | field |
|---|---|---|
| +0x00 | u32 | unused by the sound code (0 in [L]) |
| +0x04 | u32 | sfx id with flag 0x40000000 (masked to 0xFFFFF for the IOP) |
| +0x08 | f32[3] | position x, y, z |
| +0x14 | f32 | converted to s16 and passed as the "pitch" short. Value 100.0 in both records [L]; effectively unused |
| +0x18 | f32 | outer radius, which is also the trigger radius |
| +0x1c | f32 | inner radius (≤ 0 → table default) |

- **Per frame** (from UpdateListeners, before UpdateSounds), with `d² = |GlobalListenerPos − pos|²`:
  - If not active and `d² < R²` (R = +0x18): `handle = Sound_Play3D(id, &pos, vol 100.0, outer R, inner +0x1c, (short)+0x14, ref 0, custom 1)` and active = 1.
  - If active and `d² > 1.2·R²`: Sound_Stop(handle) and active = 0 (hysteresis).
  - Positions are static. This is the only path that sends explicit radii to the IOP.
  - Quirk: if the IOP stops reporting a one-shot ambient, its node is reaped but `active` stays 1, so it only restarts after leaving 1.2·R² and re-entering [M]/[S].
- **Level 07000005** [L]: GNumMapSounds = 2.
  - #0: id 0x40000085 (sfx 0x85: looped, table inner 100 / outer 500), pos (36.21, 0.00, 5.74), +0x14 = 100, outer 365.28, inner 182.64.
  - #1: id 0x40000174 (sfx 0x174: not looped, 100/500), pos (28.25, 12.90, 42.99), +0x14 = 100, outer 200, inner 160.
  - Both are active at level start with handles 0x1fddd70 / 0x1fddd10.
- **Other level sound objects** [M]:
  - `SoundTrigger_Create/Update` @1b2628/@1b2790 (object type 0x14, from map entities). Parameters: switch channel to start (+0x2c), channel to stop (+0x3c), sfx id (+0x34; 0xFFFF disables), "is 2D" (+0x38), mode (+0x30; 1 = self-delete after finishing), voice flag (+0x40, only one voice at a time via GlobalVoiceHandle), stop-on-channel-off (+0x48), difficulty filter (+0x44).
  - When its channel turns on it plays `Sound_Play3D(id, obj+0x30, 100, -1, -1, 0, 0, 0)` or `Sound_Play(id, 100, 0, 0)`. It stops when the stop channel turns on or (unless +0x48) the start channel turns off. Alarms and klaxons are most likely these switch-channel SoundTriggers [S].
  - `MusicTrigger` @1b30e0/@1b31a0 posts `Music_Event(+0x34, 2)` for 30 frames after its channel fires.

---------------------------------------------------------------------------------------------------

## 6. Music

### Event board [M Music_Event @1de328, Get_Music_Event @1de350, UpdateMusicalEvents @114e70]
- `EventList[64]` (0x2d34f8). `Music_Event(i, v)` sets `EventList[i] = v` (i < 64).
- **All 64 entries are cleared at the end of every UpdateMusicalEvents.** That function runs once per VBlank from SFXUpdate, so events are one-frame pulses and must be re-posted every frame to stay active [M]. All zero in [L], as expected.
- Copies are kept in BackupEventList / LastEventList.

| idx | posted by | meaning |
|---|---|---|
| 0 | Sound_Ready (levelId) | with [1] ≠ 0: start the level's music |
| 1 | Sound_Ready (1) | start enable |
| 2 | Drone AI: SeenAndAttacking (5), CheckAlarmRaised (07000005: 3 for the first 60 frames of switch channel 0xa4; else 1), Copter/GT/Sub/MiniSub (1) | alert level: 0 calm, 1 alerted, 2 attack, 3 alarm (CastleA) |
| 3 | Trigger_Activate (1) | level end / objective trigger → end markers |
| 4 | — | jump to marker 9 (CastleA) |
| 5 | Script_FFwd (script id) | NIS skipped → NisPlaying = 0, NisQuitten = 5 |
| 6 | Mission_Update fail, Player_CheckForDeath, Script_Play (1) | jump to MapMusic +0xc marker + FadeDown(0), wait for end |
| 7 | Mission_Update (1) | complete → +0x10 marker + FadeDown(0) |
| 8 | Mission_MonitorObjectives / Mission_Update (1) | end markers (+0x14/+0x18) |
| 0xa | — | MusicVars = 1 |
| 0xb | Script_CameraStart (script id) | NIS start (sets NisPlaying = 1) |
| 0xc | Script_KillStream (script id) | NIS end |
| 0xd | Player_Creep / Player_Wire | stealth action |
| 0xe | Drone_InitComms; 0xf Drone_EnableAll / DroneVision; 0x10 HostageSaved | misc |
| n | Player_Update: room cel+0xa1 | per-room music zone |

### MapMusic table (0x24f4d8, 0x23 entries × 76 bytes) [M structure / L values]
`{levelId, track, startMarker, mkFail(+0xc), mkComplete(+0x10), mkEndA(+0x14), mkEndB(+0x18), underAttackCount2(+0x1c), underAttackCount(+0x20), combatMarkers[5](+0x24, -1 terminated), stealthMarkers[5](+0x38, -1 terminated)}`

**07000005**: track **15**, start marker 0, fail 0xb, complete 0xa, endA 8, endB 9, counts 10 / 160, combat {4, 0xc, 0xe}, stealth {6, 0x10, 0x12} [L]. MusicNowPlaying = 15 and MapMusicData = 0x24f608 at level start [L].

### State machine (UpdateMusicalEvents, once per VBlank) [M]
1. If `EventList[1] && EventList[0]` (and the level is not 07000048): find the MapMusic entry with that level id (track ≠ 0), then `SFXStopMusic(); SFXStartMusic(track, startMarker)`. Reset MusicLastJumpRequest = −1, MusicLastJump = startMarker, under-attack counters from +0x20/+0x1c, temps, WhichStealth / WhichCombat, and MusicVars from MapDroneData (see the AI note below). Set IOP_DelayUpdateTimer = 10.
2. `IOP_DelayUpdateTimer--`. Once it is negative, `MusicLastJump = IOP-reported marker` (0x2500cc).
3. NIS bookkeeping from events 0xb / 0xc / 5 (NisPlaying, NisQuitten countdown of 5). These go to the IOP via op 0x19.
4. Under-attack logic (outside NIS): `[2]==2` sets MusicUnderAttack = 1 with a 600-frame bonus. While calm, the counters decay and MusicUnderAttack clears after `underAttackCount` frames. While alerted, `underAttackCount2` frames lead to MusicUnderAttack = 1.
5. `[6]` → `SFXJumpToMusicMarker(+0xc, 1)`, FadeDown(0), MusicWaitForEnd = 1. `[7]` → the same with +0x10.
6. If not waiting for the end: run the per-level script. For 07000005 that is `UpdateMFX_01CastleA` @112d78:
   - `[2]==3` (alarm) the first time: PickRandomCombat(0), then hold for 1200 frames (MusicTemp3).
   - `[0xc]==0x6000994` (end of a specific NIS) → hold 66 frames. Then (or when `[0xb]==0x6000994`) jump to marker 6 if currently in a combat marker, else 2.
   - MusicUnderAttack → PickRandomCombat(0). Else, if `[0xd]` (creep / wire) → marker 6 (if in combat) or 2. Else PickRandomStealth unless the last request was 2.
   - `[8]` or `[3]` → jump(+0x14 if in combat else +0x18, 1), FadeDown(0), wait for end. `[4]` → jump(9, 0), wait for end.
   - PickRandomCombat / Stealth @111dc8 / @111c70: if neither the requested nor the current marker is already in the target set, request `set[Which++ % count]` via `SFXJumpToMusicMarker(marker, flag)`.
   - The flag passed is 0 for normal transitions and 1 for end / fail. Its IOP meaning is [S]: probably "jump immediately" versus "at the next bar".
7. EventList is cleared.

- **MFXINFO.MXI** is read on the IOP only, not by ACTION.ELF [M: no EE references; SFX.IRX SFXInitialiseAudioStreamSystem @0x65d8]. It holds 150 s32 values (`MusicTrackInfo`). `FindMusicTrackNumber(track)` returns `index+1` of the matching value and is used when a stream starts [M SFX.IRX @0x6288/@0x92dc].
  - Local copy (work/audio/MFXINFO.MXI) [L]: `15,16,17,18,19,20,21,22,23,24,36,34,35,25,30,39,-1…`.
  - So the EE's track 15 for 07000005 becomes **music stream #1**. **Corrected (WORKLOG 0068):** the files are `PS2/MUSIC/MFX_0/MFX_1.{SMF,SSD}` (MXI index + 1); STREAMS.* hold streamed effects only. Music: docs/systems/music.md.
- **AI note:** `MusicVars` (0x2707a0) is not audio. It is drone-comms tuning taken from MapDroneData (0x24f418); for 07000005 that is {2, 2, 35.0} [L]. NDrone2_SeenAndAttacking posts `Music_Event(2, 5)` once `count ≥ MusicVars` [M].
- Volumes: options menu `SFXSetVolume(sfx_slider*5)` / `SFXMusicSetVolume(music_slider*5)` (P_CNAVOPTIONS). Loaded from settings in LS_LoadGlobalSettings / Boot_LoadPTPData [M].

---------------------------------------------------------------------------------------------------

## 7. Gameplay call sites relevant to 07000005

All of these are `Sound_Play3D(id, pos, 100, -1, -1, 0, 0, 0)` (default radii) unless noted [M].

- **Player gun (first person):** fired by the weapon animation script, AnimProcessScriptCmds @1c1b38 **op 1** (u16 cmd[0] = op|len, cmd[1] = frame, cmd[2] = sfx id).
  - Triggers when the anim frame crosses cmd[1]. SFXPreCache(id) is sent up to 0x33 frames early for flagged scripts.
  - Plays `Sound_Play3D(id, pos, vol 100, 100, 0, pitch = Rand(500)−250, 0, 0)`. pos = the object's transformed position; for the player's weapon anim object (player+0x7e8) it is a bone or 1.5 units forward.
  - Id remaps by the player's weapon (+0xdc → +0x62): 1→2 (weapon 7/9), 0x403→2 (3/5), 0xfa→0x1fb (0x14), 0x106→0x284 (0x1e..0x23), 0x1e1→0x602 (0x10), 4→0x604 (0x16/0x17).
  - op 2 / sub 3 = `Sound_StopAllWithId(cmd[3])`.
  - Extra player weapon sounds (2D `Sound_Play(id, 100, 0, 0)`):
    - Player_WeaponInitBullet @1a4bf0: 0xec for weapon 0x54.
    - Player_SetWeaponAnimObj @1a2e38: 0x280–0x283 (weapon select / holster).
    - Player_WeaponChange: 0x1db.
  - Bullet_init @12c5a0: `param_7` sound at the bullet start (callers pass 0xffff = none for player and drone guns); 0x10e for weapon class 0xe; 0x5ef for weapon 0x45.
- **Footsteps:** AnimObjectUpdate @1befd8.
  - Player: player+0x924 accumulates movement and a step fires every 1.65 units.
  - If obj+0xf6 == 1 (stride 0.4125): `Sound_Play3D(0x286, …)` + SetAlertness(0). Otherwise `Effect_Create(1.0, hitdata, obj, 0x42|(foot ? 8 : 4))` → Effect_Player @1cec48 → `Sound_Play3D(EffectInfo[surface].u16 +0x16, hit+0x40, vol 1.0, …)` + SetAlertness(0). EffectInfo is at 0x3037c0, 0x88 bytes per surface (hit+0x50 & 0x3f).
  - NPCs: anim script op 2 sub 0/4/5 sets ScriptFootstepFoot / Type (no sound in itself).
  - Suppressed while ScriptCam ≠ 0.
- **Bullet impacts:**
  - Effect_Bullet @1ce8a8: EffectInfo +0x14 (surface impact).
  - Effect_Body @1cf400: +0x18 + ModAlertness(0.5).
  - Bullet_CollisionHandler: 0x4a7, 0x446, 0x51c.
  - Bullet_DoTrails: 0x1dc fly-by, 0x16, 0x14, 0x447, 0x5dd.
  - Bullet_Delete: 0x28d. Bullet_update: 0x27c. Casings: Bullet_casing_update (id per casing).
  - Break_Kill: 0x5f2 or its own id.
  - NDrone2_DoHitEffects: 0x601 / 0x413.
- **Drone weapons:** DroneWeap_FireWeapon @178f68 plays the id from `AnimDatumGetWeaponInfo` (skipped if 0x5ef): `Sound_Play3D(id, drone+0x30, vol 25, outer 100, inner rand, 0, 0, custom 0)`, so the radii are ignored.
- **Drone voice / foley:** `NDrone2_PlaySFX(scale f12, DCVars*, idx, VECTOR* pos|null, Sc enforceDelay)` @141848.
  - Uses `Drone_SfxInfo[idx]` (0x2812d0, 346 × 32 bytes): {u32 sfxId, u32 facialAnimId, u32 lastFrame, u32 repeatDelay(frames), f32 vol, f32 outer, f32 inner, s16 pitchMin, s16 pitchMax}. 336 entries are in use, 61 with a facial anim [L]; typical entry vol 100, outer 15, inner 10.
  - Blocked during the first 30 frames after `DAT_00298fb8` and inside `repeatDelay` (when enforced). The pitch is random in [min, max).
  - Calls `Sound_Play3D(sfxId, pos | drone+0x30, vol, outer·scale, inner·scale, pitch, ref = *(DCVars[3]), 0)`, so the radii are ignored.
  - Keeps a 16-entry recent-play ring at 0x29aa9c.
  - Callers use scale 1.0: BOT_handlePain, BOT_soundEffect, DroneSpawner, CivilianPatrol 0x48, Interogate 0x4a, OpenDoor 0x27, KickObject 0x28, SeenSurrendered 0x23, HostageDie 10.
- **Drone speech:** `NDrone2_SpeechSFX(DCVars*, idx, Sc force, Sc useFacial, Sc facialOnly)` @141b00.
  - Disabled in MP (MPSettings+0x184). Same 30-frame and repeat gates.
  - If the entry has a facial anim (hashtable_getitem): `AnimScriptPeekSfx` finds the anim script's first op-1 sfx id, `SFXPreCache(id, 0x14)` is sent, and the line plays 20 frames later through the facial anim script's op 1 (a positional, **bank sample, not a stream** [S]). Otherwise `AnimFacialAppend` is used.
  - Without a facial anim: `NDrone2_PlaySFX(scale 2.0, …, drone+0x30, 0)`. The handle is stored at drone+0x4d0, and a non-forced line is skipped while that handle is set.
  - Talk functions poll Sound_IsFinnished on it.
  - `DroneFunc_ChooseSFX(a, b)` @1497e8 = `a + Rand(b − a + 1)` (random line index).
- **Player:**
  - Pain: Player_HandlePain 0x88. Death: Player_CheckForDeath 0x89 (+ Music_Event 6).
  - Jump / land: Player_HandleJump 0xa3, 0x5fb; 2D 0x1db.
  - Air: Player_MonitorAir (2D 0x60c / 0x60b, 3D 0x27).
  - Object hits: Player_DealWithObjHit (2D 0x600 / 0x412, 3D 0x171).
  - Objective chimes: HUD_UpdateStatusPane, 2D `ObjectiveSFXPlayMe1st / 2nd`.
- **Pickups:** Pickup_Handler @18e650: `Sound_Play3D(pickupDef.u16 +0x2a, obj+0xc0, …)`.
- **World objects:**
  - Doors: Door_SetState / Door_Activate, door def u16 +0x60 / +0x62 / +0x64.
  - Locks: Lock_Update / Lock_Activate / Lock_Unlock (0x249, 0x5da / 0x5db 2D).
  - Sensor_Update: 0x4aa, 0x4a9, 0x4dd.
  - Searchlight_Update: 0xe5 / 0xe6.
  - Light_Update: light +0x2c.
  - Hurt_Update: def +0x1a.
  - GunImp_Update: 0x5e4 / 0x5e5 / 0x5e6.
  - Rigid bodies: RB_RestingContact(Rot).
  - View_DrawSky: 2D 0x5d / 0x5e (thunder).
- **Script player (cut-scenes / scripted sounds):** Script_SoundStart @1d8820 reads {u32 sfx, u8 force2D, u8 hasObj, u8 objIdx}.
  - Plays `Sound_Play(sfx, 100, 0, 0)` if 2D or there is no object; otherwise `Sound_Play3D(sfx, obj+0xc0, 100, -1, -1, 0, 0, 0)`.
  - Skipped if script flag 0xab5 & 0x20 is set.
  - Each frame Script_Interp case 5 does `Sound_SetPosition(h, obj+0xc0)` and kills the stream when `Sound_IsFinnished`.
  - Script_SoundEnd kills looping sounds. Script_KillStream does Sound_Stop.
- **Alarm:** the 07000005 alarm flows through switch channel 0xa4 → `Music_Event(2, 3)` [M DroneFunc_CheckAlarmRaised @1453a8]. The audible alarm itself is not in the drone DSTATE code [M grep]; it is presumably a map SoundTrigger on that channel [S].

---------------------------------------------------------------------------------------------------

## 8. Globals at level start (ee_start_05.bin) [L]

| global | addr | value |
|---|---|---|
| SfxMasterVolume | 0x30c95c | 100 |
| MusicMasterVolume | 0x30c958 | 70 |
| start_sfx_vol / start_music_vol / music_vol | 0x30d8e0 / 0x30d8e4 / 0x30d558 | 0 / 0 / 0 (menu temporaries) |
| SoundInfo | 0x30cb50 | two s16 values: 100, 100 |
| enabled_sound_flags | 0x30d184 | 0x0b (menu sounds 0, 1, 8 enabled; 2 disabled) |
| switch_CONFIG_SOUND | 0x30ceac | 1 |
| SoundInit | 0x30d078 | 1 |
| number of listeners | MPSettings+0x180 / +0x1ac | 0 / 1 → 1 listener |
| SFXModeNumber / SFXLanguageNumber | | 1 / 0 |
| GNumMapSounds | | 2 (see §5) |
| Sound_FrameDelay | | 0 |
| g_fSubtitleDurationModification | | 0.92 |
| subtitles on (GameState+0x60) | | 0 |
| ScriptCam / NisPlaying / NisQuitten / SFXPaused | | 0 / 0 / 0 / 0 |
| MusicNowPlaying / MapMusicData | | 15 / 0x24f608 |
| MusicLastJumpRequest / MusicLastJump | | −1 / 0 |
| MusicUnderAttack / MusicWaitForEnd | | 0 / 0 |
| SFXBatchPos | | 88 words pending |
| FRAME_RATE_INT / FRAME_RATE_DIV | | 60 / 1.0 |
| GameState+0x34 (frame) / DAT_00298fb8 | | 108153 / 108150 (drone voice gate still closed for 27 more frames) |
| DynamicSoundList | | 5 in use / 251 free |
| GlobalListenerPos | | (104.954, 25.663, 108.825) |

## 9. Port notes [S]
- A faithful port can ignore per-sound volume and pitch, as the original does. Use bank (IOP) inner/outer radii for every 3D sound except map ambients, which supply their own.
- Reproduce the deferred start (one frame of latency between Sound_Play and the IOP start) and the 5-VBlank "requested" grace period before a one-shot can be reaped.
- One listener, the camera eye. Its velocity is zero, so there is no Doppler.
- Music is a marker-driven stream state machine with one-frame event pulses.

---

# Part 2: IOP driver (SFX.IRX)

Sources: decompiled C in `work/decomp/SFX.IRX/` and the raw module `work/audio/SFX.IRX`.
The module's `.mdebug` section contains full **stabs type info**, so all struct and field names below are the
original ones (from `mdebug_strings.txt`, extracted from the module). gp = `_gp` = 0x3B880, and gp-relative
globals were resolved through `.symtab` (`syms.txt`).
Tags: **[M]** read in code (function@addr), **[D]** measured on SB_4.* bank files, **[S]** inference.
All arithmetic is 32-bit signed int with C truncating division unless noted. No floats anywhere in the SFX path [M].

Helpers: `tools/audio/sfxparse.py` (SB_4.SFX/SHF parser over `work/audio/`, `python -I tools/audio/sfxparse.py 139 138 1523`) and `tools/audio/sqrtsim.py`.

---------------------------------------------------------------------------------------------------
## 0. Globals (gp-relative, initial .sdata values) [M symtab + .sdata dump]

| gp off | name | init | use |
|---|---|---|---|
| -0x7ff0 | ES_MuteMusic | 1 | multiplier for the music-class SFX ids |
| -0x7fec | ES_MuteSFX | 1 | 0/1 multiplier (SFXMuteSFX / SFXUnMuteSFX) |
| -0x7fe4 | SFXModeNumber | 1 | |
| -0x7fdc | SFXStartedCount | 0 | voice-budget pressure |
| -0x7fd8 | MusicMasterVolume | 70 | |
| -0x7fd4/-0x7fd0/-0x7fcc/-0x7fc8 | DuckerDuration / DuckerVolume / DuckerCurrentVolume / DuckerStartTime | 1/100/100/0 | |
| -0x7fc0 | SfxMasterVolume | 100 | SFXSetVolume (clamped 0..100) |
| -0x7fbc / -0x7fb8 | FadeVolume / FadeMusicVolume | 10000 | 0..10000 |
| -0x7fb4 / -0x7fb0 / -0x7fac | FadeStatus / FadeRate / FadeMusic | 0 / 30 / 1 | |
| -0x7fa8 / -0x7fa4 | SFXNewReverb / SFXReverb | 0 | reverb depth target / current, 0..100 |
| -0x7fa0 / -0x7f9c | SFXNewIndoorsVolume / SFXIndoorsVolume | 100 | |
| -0x7f98 | SFXUnderWater | 0 | |
| -0x7f94 | FadeStartTimer | | |
| -0x7f60 / -0x7f5c | randomseed1 / randomseed2 | 12341234 / 13123123 | |
| -0x7f54 | SFXListenerCount | 1 | |
| -0x7f18 | SFXKeyOnFrameDelay | 0 | precache |
| -0x7f08 | SFXMode | **4** | 0 = mono downmix; any other value = stereo/surround |
| -0x7e9c | SFXTimerValue | | +1 every 100 000 us (TimerThread@12a84); psiSFXGetTimer() = value*100 → **ms in 100 ms steps** |
| -0x7e78 / -0x7e74 | BaseSFXActive / BaseSFXFree | | SFXItem lists |
| 0x339b0 / 0x339b4 / 0x339b8 | NisPlaying / NisQuitten / SFXPaused | 0 | |

Data: ListenerPos 0x160F8, ListenerVel 0x16104, ListenerNorm 0x16110 (init -1000,0,0), ListenerDir 0x1611C (init 0,1000,0),
ListenerUp 0x16128 (init 0,0,1000), ListenerPositions[4][3] 0x17FAC, NormVector/DirVector/UpVector 0x160D4.. = (1000,0,0)/(0,1000,0)/(0,0,1000),
SoundBankData[150] (36 B) 0x16134, ES_VoiceHandleData[100] (12 B) 0x17AFC, SFXActive pool = 40 SFXItems (InitialiseSFXList@3034),
VoiceHandleData[48] (20 B) 0x24F8C, VoiceLooping[48] 0x24DD0. [M]

---------------------------------------------------------------------------------------------------
## 1. File structures

### 1.1 SFXParameters (0x44 bytes, stabs `SFXParameters:t63=s68`) [M stabs]
| off | type | name | used by IOP? |
|---|---|---|---|
| 0x00 | int | ReverbSend | yes: voice reverb bit on if !=0 at key-on (psiSampleKeyOn → PS2_VoiceSetReverb@ebcc). Only zero vs non-zero matters [M] |
| 0x04 | int | TrackingType | 0 = 2D, 1 = distance-only (centered), 2 = full 3D, 3 = listener-attached (see §3) |
| 0x08 | int | InnerRadiusReal | default for SFXItem.InnerRadius |
| 0x0C | int | OuterRadiusReal | default for SFXItem.OuterRadius |
| 0x10 | int | MaxVoices | max concurrent items with same id (0 = unlimited) |
| 0x14 | int | Priority | "importance" (SFXGetImportance@41e0 returns it) |
| 0x18 | int | Group | **unused** by IOP [M: no reads] |
| 0x1C | short | MaxReject | !=0 → reject the new sound instead of stealing |
| 0x1E | short | Action2 | unused [M] |
| 0x20 | short | IgnoreAge | unused [M] |
| 0x22 | short | Ducker | duck amount (music ducks to 100-Ducker); also "is a ducker" flag |
| 0x24 | int | DuckerLength | duck hold time in ms |
| 0x28 | int | MasterVolume | per-sfx volume % |
| 0x2C | int | MinDelay | cue timer min (frames) |
| 0x30 | int | MaxDelay | cue timer max (frames) |
| 0x34 | short | MultiSample | 0 = pick one sample at random; !=0 = use all samples (sequence or polyphonic) |
| 0x36 | short | RandomPick | **unused** [M]; random pick is implied by MultiSample==0 |
| 0x38 | short | Shuffled | sequence order random (only when MultiSample!=0 && Polyphonic==0) |
| 0x3A | short | Loop | re-trigger the whole sfx after it ends |
| 0x3C | short | Polyphonic | start all samples at once (only when MultiSample!=0) |
| 0x3E | char | Outdoors | scaled by SFXIndoorsVolume |
| 0x3F | char | PauseInNis | paused while an NIS (cutscene) plays |
| 0x40 | int | SampleCount | number of SFXSamplePoolFiles records |
| 0x44 | | SFXSamplePoolFiles[SampleCount] | |

### 1.2 SFXSamplePoolFiles (28 bytes) [M stabs + StartSample@3524]
| off | name | meaning |
|---|---|---|
| +0 | FileRef | >=0: sample index into the bank's SHF headers. **<0: streamed sound** → `SFXInitialiseStream(FileRef, Is3d)`, base pitch fixed 0x759 (=22.05 kHz) [M] |
| +4 | PitchOffset | linear pitch offset, 0x3000 = +100 % (see §4) |
| +8 | RandomPitchOffset | ± random range, same units |
| +C | BaseVolume | 0..100 |
| +10 | RandomVolumeOffset | ± range |
| +14 | Pan | -100..100 (2D only), clamped |
| +18 | RandomPan | ± range |

### 1.3 Bank facts (SB_4.SFX, 225 entries) [D] (`sfxparse.py`)
* Table: u32 count, count×{u32 HashCode(sfx id), u32 offset}. First block at 4+8×225 = 1804. Blocks are in offset order and
  **each block is 0x44 + 28×(SampleCount+1) bytes**: there is always one extra all-zero 28-byte record after the last subtrack (225/225 entries),
  so the last block ends at 36368 = file size. [D] The IOP never reads it, except that for SampleCount==0, `SFXrnd(0)`=0 selects record 0, i.e. this zero
  record (FileRef 0, BaseVolume 0) → plays SHF sample 0 at volume 0 (silent, but it still takes a voice). 8 entries have SampleCount 0 (ids 279,278,277,233,10,237,9,34). [M+D]
* Total subtracks 463; FileRef range -823..251, all 252 SHF samples referenced, all non-negative refs < 252. 35 negative (stream) refs, always with SampleCount 1.
* Value sets: TrackingType {0:50,1:5,2:158,3:12}; Inner {1..400}; Outer {10..500}, always Inner<Outer; MaxVoices {1,2,3,5,8,10};
  Priority 10..100; Group/MaxReject/Action2 all 0; Ducker {0,10,30,80}; DuckerLength -1500..5700 (negative values occur);
  MasterVolume {70,75,100}; MinDelay/MaxDelay 0 except id 372 (300/600); ReverbSend {0,1,50}.
  PitchOffset {0, -1024}; RandomPitchOffset {0,512,1024,1536,2048}; BaseVolume {75,80,90,95,100}; RandomVolumeOffset {0,5,10,15,20};
  Pan {0, ±102400} (only id 133; clamped to ±100); RandomPan always 0.
* Special entries: MultiSample=1 only in ids 133 (2× looping sample 13, Pan -102400/+102400, Polyphonic) and 267 (Polyphonic, 1 sample).
  Polyphonic=1 also on id 137, but MultiSample=0 there, so it plays as a random single pick. Shuffled=1 on ids 5 and 372, but both have MultiSample=0, so Shuffled has no effect.
  Loop=1 only on id 372 (random pick of 2, re-trigger after 300..599 frames).
* SHF: 7 samples have Flags bit0 (loop) = 1: #1,13,198,199,201,204,251 → ids 329,133,18,61,267,260,256 play forever until removed.
  Base pitches: 941 (0x3AD, 11.0 kHz) ×102, 1882 (22.05 kHz) ×99, 1024 (12 kHz) ×49, 1621 ×2.

### 1.4 Example entries [D]
* **id 139** (off 0x70C): TrackingType 2, Inner 2, Outer 30, MaxVoices 5, Priority 50, ReverbSend 0, MasterVolume 100, MultiSample 0 → random 1 of 4:
  FileRef 185/186/187/188, all BaseVolume 80, no pitch/vol/pan randomisation; SHF pitch 0x3AD, sizes 3712/5440/10368/8320.
* **id 138** (off 0x7DC): same parameters, 14 subtracks: 193,190,191,192,189 then **9× FileRef 0** (all BaseVolume 80). Sample 0 is a 640-byte, nearly silent ADPCM clip (pitch 0x400),
  so 9 times in 14 the sound is effectively nothing [D/S: intentional "chance of silence"].
* **id 1523** (off 0x9C4): TrackingType 2, Inner 1, Outer 100, MaxVoices 5, Priority 40, ReverbSend 50, 1 subtrack: FileRef 11, PitchOffset 0, RandomPitchOffset 1536 (±12.5 %),
  BaseVolume 100, RandomVolumeOffset 15, SHF pitch 0x3AD.

---------------------------------------------------------------------------------------------------
## 2. SFXItem (0x2C0 = 704 bytes, stabs `SFXItem:t77=s704`) [M stabs, cross-checked against code offsets]
| off | name | notes |
|---|---|---|
| 0x000/0x004 | Next/Last | active list (NewSFX pushes at **head**) |
| 0x008 | HashCode | sfx id |
| 0x00C | Counter | age in frames (+1 per SFXUpdate while not paused) |
| 0x010 | StreamHandle | |
| 0x014 | VoiceHandle[10] | ES voice handles (index into ES_VoiceHandleData), -1 = none |
| 0x03C | BaseVolume[10] | per channel, after randomisation |
| 0x064 | Volume3D | unused |
| 0x068 | Pitch[10] | SPU pitch per channel |
| 0x090 | Pan[10] | per channel |
| 0x0B8 | CuePos | remaining samples in sequence |
| 0x0BC | CueSampleWaiting[50] | [0] = cue/loop timer; [i] = per-channel start delay (polyphonic) |
| 0x184 | CuedSamples[50] | SFXSamplePoolFiles* |
| 0x24C | SFXData | SFXParameters* |
| 0x250 | SoundBankSlot | |
| 0x254 | UpdatedCounter | per-frame re-entry guard |
| 0x258 | RealPos (SFXPoint3 int x,y,z) | world position from EE |
| 0x264 | pos | effective position used for 3D (= RealPos, or synthesised, §3.2) |
| 0x270 | vel | |
| 0x27C | Handle | opaque EE handle (`void*`), param_4 of SFXStart3D |
| 0x280 | VolumeOverRide | forced 100 by SFXStart3D |
| 0x284 | OuterRadius | |
| 0x288 | InnerRadius | |
| 0x28C | DeleteMe | killed at end of SFXUpdate |
| 0x290 | Paused | |
| 0x294/0x298 | StartedWhenPaused / StartedWhenNisPlaying | |
| 0x29C/0x2A0 | FadeAndKill / FadeAndKillVolume (100) | |
| 0x2A4 | Specials {Volume3D, Pan1(L), Pan2(R), Pan3(Rear)} | output of PS2_SFXCalculate3D |
| 0x2B4 | KeyOnFrameDelay | precache |
| 0x2B8 | FirstTransferComplete | |
| 0x2BC | IsStreamed | |

psiSFX (52 B, per-voice packet built by ES_FillpsiSFXData@329c) [M]: Handle (psi voice), InnerRadius (=item+0x288), OuterRadius (=item+0x284),
Volume = ES_CombineVolumes(BaseVolume[ch]) (for streams ×ES_GetStreamShutdownVolume/100), Frequency = Pitch[ch], Is3d = Is3dTracked = (TrackingType!=0),
IsStreamed, Pan = Pan[ch], pos = &item.pos, vel = &item.vel, Specials = &item.Specials, TrackingType.

---------------------------------------------------------------------------------------------------
## 3. Start path

### 3.1 RPC (sifrpc_server@f600) [M]
* cmd 0x0D: `SFXStart3D(id, pos[3], vel[3], handle, volume, inner, outer)` (words 1..11). cmd 3: `SFXStart(id, handle)` = SFXStart3D(id, Zero3Dpos, Zero3Dpos, handle, -1,-1,-1).
* cmd 0x0B: `SFXSetPositionAndVelocity(pos[3], vel[3], handle, id)`; cmd 0x0C: `SFXRemove(handle, id)`; cmd 0x0E: `SFXSetupListener(pos,vel,dir,up,norm,index)`; cmd 4: `SFXUpdate()`;
  cmd 8 SFXSetVolume; 6/7 FadeDown/FadeUp(fadeMusic); 9/10 Pause/UnPause; 0x18 SetNumberOfListeners; 0x19 NisPlaying/NisQuitten; 0x1C SetMode; 0x1D SFXSetEnvironment(reverb, indoors);
  0x26 SFXPreCache(id, frames).
* **Return values of SFXStart3D are discarded.** Every RPC batch ends (cmd 5 or 0x10) by returning `&SFXActiveData`:
  `{int Count; {int HashCode; void* Handle;}[40]; ...}` rebuilt at the end of each SFXUpdate from the active list (§6). The EE refers to a
  playing sound only by **(Handle, HashCode)**: SFXSetPositionAndVelocity and SFXRemove match `item.Handle==handle || handle==0` **and**
  `item.HashCode==id || id==-1` (every match is affected) [M SFXRemove@1f88, SFXSetPositionAndVelocity@1994].
* Key-on/key-off bits are batched: `PS2_ResetKeyONKeyOff` at RPC entry, `PS2_SendKeyONKeyOff@d544` at batch end (§7).

### 3.2 SFXStart3D@3aac [M]
1. `kd = SFXKeyOnFrameDelay; SFXKeyOnFrameDelay = 0; volume = 100` (the caller's volume is ignored).
2. Scan slots 0..149 with Loaded!=0, then that slot's (HashCode, Offset) table; the first match wins → `P = bank.SFXBuffer + Offset`. Not found → return 1.
3. Precache special case if `P.SampleCount==1`:
   * stream (`sub[0].FileRef<0`): if kd==0, find an item with the same id and KeyOnFrameDelay!=0: clear the delay; if VoiceHandle[0]!=-1 → refresh StartedWhen*, `SampleKeyOn(VoiceHandle[0], P.ReverbSend)`, **return 0 (no new item)**, else DeleteMe=1 and keep scanning.
     If kd!=0 and such a precached item already exists, return 0.
   * non-stream and kd!=0: return 0 (only streams can be precached).
4. `NewSFX()` (pop free list, push at head of active list). None free → "SFX list full", return 2. Max 40 items.
5. Init: StartedWhenNisPlaying=NisPlaying, StartedWhenPaused=SFXPaused, FadeAndKillVolume=100, FadeAndKill=0, Handle=handle,
   pos=RealPos=*pos (if non-null), vel=*vel (if non-null), VolumeOverRide=100,
   **InnerRadius = (inner==-1 ? P.InnerRadiusReal : inner)**, **OuterRadius = (outer==-1 ? P.OuterRadiusReal : outer)**,
   SFXData=P, HashCode=id, SoundBankSlot, Counter=0, Paused=0, KeyOnFrameDelay=kd.
6. `SFXSetup(item)`; it returns 0 on success (setup ok) and 1 on failure (the item already has DeleteMe set).

SFXPreCache(id, n) [M@39f4]: `SFXKeyOnFrameDelay = n+5; SFXStart(id,0)`. The voice is set up but not keyed on (StartSample skips KeyOn when KeyOnFrameDelay!=0).
HasSampleEnded returns 0 while KeyOnFrameDelay>=1. The item counts KeyOnFrameDelay down once per frame, and when it reaches 0 it is deleted, unless a later SFXStart3D of the same stream id took it over (step 3).

### 3.3 SFXSetup@4220: voice budget, stealing and sample choice [M]
```
P = item.SFXData; subs = (SFXSamplePoolFiles*)(P+0x44)
item.VoiceHandle[0..9] = -1
if SFXStartedCount > 2000: SFXStartedCount = 2000
item.DeleteMe = 0; item.IsStreamed = 0          // param_1[0xaf] = +0x2BC
budget = 15 - SFXStartedCount/400                  // 15..10
n = CountAllVoicesActive(item)                     // # VoiceHandle[]!=-1 over all OTHER items (ES handles, not SPU voices)
// (a) distance/priority rejection, TrackingType 2 only, Priority < 100
if P.TrackingType==2 && P.Priority<100:
    d = ES_GetDist(item)                           // §5.1, units = world/1000
    reject = d > item.OuterRadius
    if !reject:
        r = max(item.OuterRadius - item.InnerRadius, 1)
        v = clamp(100 - ((d - item.InnerRadius)*100)/r, 0, 100)
        reject = v*P.Priority < (n*budget*2000)/(budget*budget)
    if reject: item.DeleteMe=1; return 0
// (b) global voice budget
if n > budget:
    n2 = CountAllVoicesActive(item)
    repeat (n2-budget) times:
        victim = item in active list (head→tail, includes the new item itself, which is at the head) with
                 DeleteMe==0 && IsStreamed==0 && TrackingType!=3 (streams are never stolen) and minimum Priority (strict <, so on ties the first in list order, i.e. the newest, is picked)
        if victim: victim.DeleteMe = 1
    if P.MaxReject: item.DeleteMe=1; return 0
    if item.DeleteMe: return 0                     // the new sound lost
// (c) per-id instance limit
if P.MaxVoices != 0:
    c = # other items with same HashCode (DeleteMe'd ones included)
    if c >= P.MaxVoices:
        if P.MaxReject: item.DeleteMe=1; return 0
        oldest = same-id other item with max Counter (first in list on ties); oldest.DeleteMe = 1
SFXStartedCount += 100
item.CuePos = 0
if P.Ducker && (P.TrackingType==0 || P.TrackingType==3): ES_StartDucker(100-P.Ducker, P.DuckerLength)
// (d) sample selection
if P.MultiSample == 0:                              // random single
    if !StartSample(&subs[SFXrnd(P.SampleCount)], slot, item, ch=0): item.DeleteMe=1; return 0
    item.CueSampleWaiting[0] = SetCueTimer(P)
elif P.Polyphonic == 0:                             // sequence
    item.CuePos = P.SampleCount; item.CueSampleWaiting[0] = 0
    if !P.Shuffled: CuedSamples[k] = &subs[SampleCount-1-k]   // played in order 0,1,2..
    else:            CuedSamples[k] = &subs[perm(k)]            // see shuffle below
    StartSample(CuedSamples[--CuePos], slot, item, 0)         // return value NOT checked
    item.CueSampleWaiting[0] = SetCueTimer(P)
else:                                               // polyphonic: nothing starts here
    for i < SampleCount: CueSampleWaiting[i] = SetCueTimer(P); CuedSamples[i] = &subs[i]
return 1
```
Victims are only marked; they are killed (key-off) in the kill pass at the end of the next/current SFXUpdate.
Shuffle (exact) [M]: `used[50]=0; rem=N; for k<N: r=SFXrnd(rem); idx=0; do{ if(r>0) idx++; while(used[idx]) idx++; }while(r-- > 0); used[idx]=1; rem--; CuedSamples[k]=&subs[idx]`
(the 1st-played sample is CuedSamples[N-1]). Brute force for N≤7 shows it always yields a valid permutation (biased) [D sim].
Polyphony: at most 10 channels (VoiceHandle[10]); SampleCount>10 with Polyphonic would overflow (no such entry in SB_4) [M/D].

RNG [M SFXrnd@2efc, verified in disassembly]: `if n<1 return 0 (no state change); s1 += s2; s2 += s1 + (s1>>31 /*arith: 0 or -1*/); return ((uint32)s1 >> 16) % n` (unsigned modulo).
`SFXsfrnd() = SFXrnd(0x801) - 0x400` ∈ [-1024, 1024]. SFXfrnd = SFXrnd(0x401) (unused). `SetCueTimer(P) = P.MinDelay + SFXrnd(P.MaxDelay - P.MinDelay)` (frames).

### 3.4 StartSample@3524 (sub, slot, item, ch) [M]
```
hdrs = SoundBankData[slot].SampleHeader
item.VoiceHandle[ch] = ES_RequestVoiceHandle(hdrs, sub.FileRef, Is3d = (P.TrackingType!=0))
if == -1: return 0
if sub.FileRef < 0: base = 0x759; item.IsStreamed = 1  else: base = hdrs[FileRef].Frequency (SHF +0xC); item.IsStreamed = 0
r1 = SFXsfrnd(); r2 = SFXsfrnd(); r3 = SFXsfrnd()          // call order: pitch, volume, pan
vol   = clamp(sub.BaseVolume + ((r2*sub.RandomVolumeOffset) >> 10), 0, 100)     // >> is arithmetic (floor)
pitch = base + (base * (sub.PitchOffset + ((r1*sub.RandomPitchOffset) >> 10))) / 0x3000   // truncating /
pan   = clamp(sub.Pan + ((r3*sub.RandomPan) >> 10), -100, 100)
item.BaseVolume[ch]=vol; item.Pitch[ch]=pitch; item.Pan[ch]=pan
item.pos = item.RealPos; ES_GetMyPos(item)                // §5.2
fill psiSFX; psiInitialiseSound(psi)                     // 2D: Setup2D; 3D: Calculate3D + Setup3D
if item.KeyOnFrameDelay == 0: SampleKeyOn(VoiceHandle[ch], P.ReverbSend)
return 1
```
Voice allocation: ES_RequestVoiceHandle@31c → psiRequestVoiceHandle@e648: picks the first of 48 psi slots that is not keyed on or locked, then
psiRequestVoiceHandle2@e420 takes the first free SPU hardware voice (0..23 core0, 24..47 core1; free = not playing and not locked), sets
SSA = header Address, ADSR1=0x000F, ADSR2=0x1FC0 (instant attack, max sustain, fastest linear release), and VoiceLooping[v] = SHF Flags&1.
**3D sounds (Is3d) take a second hardware voice for the rear channel** (VoiceHandle2); if it fails, the whole request fails. ES handle pool: 100 [M].
Bug note: if all 48 psi slots are busy, psiRequestVoiceHandle indexes slot 48 (out of bounds) [M].

---------------------------------------------------------------------------------------------------
## 4. Pitch [M StartSample, psiSetSampleFrequency@eeec]
`SPU_pitch = base + base*(PitchOffset + ((sfrnd*RandomPitchOffset)>>10)) / 12288` with base = SHF +0xC (0x1000 = 48 kHz) or 0x759 for streams.
The scale is **linear**: 12288 (=12×1024) = +100 %. The data uses multiples of 512 (PitchOffset -1024 = -8.33 %, which is about a semitone) [S: "1024 per semitone", applied linearly].
**SFXpow is never called** (no jal to 0xC64 in .text) [M]. No doppler (§5.3). The value is written each frame to SD_VP_PITCH (param|0x200) of both voices as a u16 (truncated) and
stored in VoiceHandleData.Frequency. Pause sets pitch 0 and remembers the old value; on unpause the stored value is restored (psi handles 0 and 1 get 0xAAA instead, a quirk) [M psiSampleUnPause@f4cc].

---------------------------------------------------------------------------------------------------
## 5. 3D math (all ints)

### 5.1 PS2_SquareRoot@10250 (non-exact; port it bit-exactly) [M + sim]
```
uint32 PS2_SquareRoot(uint32 n): if n==0 return 0
  int64 x = 1, N = (int64)(int32)n
  repeat 10: x = x - (x*x - N) / (2*x)        // int64, truncating signed division (__divdi3)
  return (uint32)x
```
It often overshoots by 1 (sqrt(100)=11, sqrt(10000)=101, sqrt(d²)≈d+1 for d≤300). It is **unconverged for large n**:
sqrt(250000)=518, sqrt(1e6)=1297, sqrt(4e6)=4243. Small n: 2→1, 3→2, 5→3, 8→3, 15→4, 24→5 (sqrtsim.py) [D sim].

### 5.2 Positions, listener, ES_GetDist / ES_GetMyPos [M]
* SFXSetupListener@5efc (count==1): ListenerPos=pos, ListenerVel=vel, ListenerDir=dir, ListenerUp=up, ListenerNorm=norm, ListenerPositions[0]=pos.
  If count>1: Norm/Dir/Up are reset to the constants (1000,0,0)/(0,1000,0)/(0,0,1000) and only ListenerPositions[index]=pos is stored (ListenerPos is not updated).
  The direction vectors are treated as length 1000 (/1000 everywhere) [S: the EE sends unit vectors ×1000].
  Pan1 uses +Norm and Pan2 uses -Norm. The defaults (Norm=-X, Dir=+Y, Up=+Z) make **Norm the listener's left vector**, so Pan1 = left, matching psiSetSampleVolumeLeft [S].
* `ES_GetDist(item)` = `PS2_SquareRoot(min over listeners L of Σ((L - RealPos)/100)²) / 10` → world units/1000. It is compared with Inner/OuterRadius in SFXSetup.
* `ES_GetMyPos(item)`: sets item.pos (RealPos was already copied into it):
  * TrackingType 3: InnerRadius=2, OuterRadius=3 (overwrites the item fields), `pos = ListenerPos + ListenerDir`, `vel = 0`.
  * if SFXListenerCount>1 **or** TrackingType==1: `D = PS2_SquareRoot(min_L Σ((L-RealPos)/100)²)*100`; `pos = ListenerPos + (D*ListenerDir)/1000` (per component).
    The sound is moved straight ahead of the listener at the same distance, so it is centered and only distance-attenuated.
  * TrackingType 0/2 with one listener: pos = RealPos.

### 5.3 PS2_SFXCalculate3D@1060c → Specials Pan1/Pan2/Pan3 (0..100) [M; dead code checked in disassembly]
```
dx,dy,dz = pos - ListenerPos
d   = PS2_SquareRoot((dx/100)² + (dy/100)² + (dz/100)²)
// doppler: dv = PS2_SquareRoot(Σ(((pos+vel)-(ListenerPos+ListenerVel))/100)²); (d - dv)*10 is computed and stored but NEVER used → no doppler
d   = max(0, d - Inner*10)
rng = max(10, (Outer - Inner)*10)
a   = max(0, 1000 - (d*1000)/rng) / 10; a = min(a, 100)
att = (a*a)/100                                   // quadratic falloff, 0..100
u   = (dx/100, dy/100, dz/100); PS2_NormaliseVector(u)   // u_i = u_i*1000/PS2_SquareRoot(|u|²), skipped if that sqrt is 0
dl  = (ListenerNorm·u)/1000 ; dr = (-ListenerNorm·u)/1000 ; db = (-ListenerDir·u)/1000     // PS2_DotProduct = Σ/1000
rear  = ((db+1000)*att)/2000
front = 100 - rear
L = ((((dl+1000)*att)/2000)*front)/100
R = ((((dr+1000)*att)/2000)*front)/100
B = (rear*75)/100
if SFXMode == 0: L = R = B = (L+R+B)/3
clamp L,R,B to 0..100 → Specials.Pan1, Pan2, Pan3
```
Here Inner/Outer are the item's InnerRadius/OuterRadius (SFXParameters defaults or SFXStart3D overrides). Distance d is in world/100 units, so the radii are in world/1000 units.
There is no underwater low-pass filter. Underwater only halves volume (§5.5). Specials.Volume3D is never written.

### 5.4 Output to SPU (PS2_SFXSetup3D@10eb8, PS2_SFXSetup2D@11328, psiSetSampleVolume*) [M]
`PS2_GetSampleVolume(v) = clamp((v*0x3FFF)/100, 0, 0x3FFF)`. VOLL = voiceRef|0x000, VOLR = voiceRef|0x100, PITCH = |0x200; voiceRef = (v%24)<<1 | core.
* **3D, TrackingType != 3**: front voice VOLL = GSV((Pan1*Vol)/100), VOLR = GSV((Pan2*Vol)/100); rear voice: s = GSV((Pan3*Vol)/100), VOLL = s,
  VOLR = s if SFXMode==0, **else 0x7FFF - s** (phase-inverted → Pro-Logic-style surround). Both voices PITCH = Frequency.
* **TrackingType 3**: L = R = Vol/2, Rear = (Vol*30)/100 (Pan values ignored).
* **2D (TrackingType 0, one voice)**: `l = (-((Pan-100)/2)*Vol)/100`, `r = (((Pan+100)/2)*Vol)/100`, each clamped 0..100; if SFXMode==0: l=r=(l+r)/2. VOLL=GSV(l), VOLR=GSV(r).
  Pan 0 → l = r = 50*Vol/100, so a centered 2D sound peaks at half scale (0x1FFF).
* Streams use the psiSetStream* equivalents with the same numbers.

### 5.5 Volume: ES_CombineVolumes@1300 (v = BaseVolume[ch], 0..100) [M]
```
if id == 0x1D7 (471) || id == 0x470 (1136):    // music-class sfx
    v = (v * MusicMasterVolume * ES_MuteMusic) / 100
else:
    v = (((v * SfxMasterVolume * ES_MuteSFX) / 100) * item.VolumeOverRide) / 100     // VolumeOverRide is always 100
    if P.Outdoors: v = (v * SFXIndoorsVolume) / 100
    if SFXUnderWater && !IsUnderWaterSFX(id): v = v / 2
v = (((((v * FadeVolume) / 10000) * item.FadeAndKillVolume) / 100) * P.MasterVolume) / 100
if P.Ducker == 0: v = (v * (100 - (100 - DuckerCurrentVolume)/3)) / 100    // SFX get 1/3 of the music duck
return (v*v)/100                                                             // square-law
```
Underwater-exempt ids (BonbUnderWaterSFX, 23 entries): 335,331,332,333,334,330,279,278,277,253,1303,137,1492,1491,136,514,10,40,33,310,311,39,9.
Final SPU level is therefore `0x3FFF * pan% * ((v²)/100) / 10000` (integer steps as above).

* **Fades** (SFXFadeUpdate@f8c, FadeDown/FadeUp(fadeMusic)): when FadeStatus!=0, `FadeVolume = 10000 - 5*(t-t0)` (down) or `5*(t-t0)` (up), with t in ms (100 ms ticks).
  When it reaches 0 or 10000 it is clamped and FadeStatus=0, so a fade lasts 2 s. Fade-up always starts from 0. FadeMusic copies FadeVolume into FadeMusicVolume.
  The preceding `FadeVolume += 10000/(status*rate)` is dead (overwritten).
* **Ducker** (ES_StartDucker@83c8 / ES_UpdateDucker@8438, once per SFXUpdate): StartDucker(target=100-Ducker, len=DuckerLength): StartTime=now, Duration=len, DuckerVolume=target.
  Update: `passed = (now-StartTime >= -0x40000000) && (now-StartTime > Duration)`; `s = (DuckerVolume - DuckerCurrentVolume)*1024`;
  if s!=0: `s = (passed ? s/15 : s/6) >> 10`, and if that gives 0 then s = sign; `DuckerCurrentVolume += s`, then ES_MusicSetVolume(). If passed: DuckerVolume = 100.
  Music volume = `((((MusicMaster*DuckCur*stream.BaseVolume)/10000)*FadeMusicVolume)/10000)*Streams[0].field_0x3C8)/100` and then squared/100 (ES_MusicSetVolume@8218).
  Negative DuckerLength (present in data) counts as already passed, so the duck barely happens. NisQuitten → StopDucker (Duration=0).
* **Environment**: SFXSetEnvironment(reverb, indoors): reverb == -0x6768 (-26472) → SFXUnderWater=1 and reverb target 0; otherwise UnderWater=0 and target=clamp(reverb,0,100).
  SFXNewIndoorsVolume = indoors ? 50 : 100. Per frame (SFXUpdateEnvironment@e60): SFXReverb moves toward the target by at most +16/-4, then
  psiSetReverb(SFXReverb): EVOL = (((r*80)/100)*0x3FFF)/100 written to EVOLL/EVOLR of both cores. SFXIndoorsVolume moves by at most ±4/frame.
  PS2_SetupReverb: effect mode 4, delay 30, feedback 200 on both cores (from the sceSdEffectAttr fields in the decompile; slightly ambiguous). A voice is sent to reverb when ReverbSend!=0 (VMIXEL/VMIXER bits, 0x1900/0x1B00).

---------------------------------------------------------------------------------------------------
## 6. Per-frame SFXUpdate@4ee4 (RPC cmd 4, once per EE frame [S]) [M]
1. `if SFXStartedCount>0: SFXStartedCount = SFXStartedCount*9/10`.
2. SFXUpdateEnvironment; ES_UpdateDucker; SFXFadeUpdate; if NisQuitten: ES_StopDucker.
3. For all items: if NisQuitten && StartedWhenNisPlaying: FadeAndKill=1. UpdatedCounter=0. Then SFXUpdateStreams().
4. For each item (`next` captured first; SFXUpdateEnding may set next=item to re-run it):
   if !Paused:
   * FadeAndKill: FadeAndKillVolume -= 5; if <0: =0, DeleteMe=1 (about 21 frames).
   * UpdatedCounter++, Counter++. If KeyOnFrameDelay: --; reaching 0 → DeleteMe.
   * **Single (MultiSample==0)**: if HasSampleEnded(VH[0]): unlock, VH[0]=-1, SFXUpdateEnding.
   * **Sequence (MultiSample && !Polyphonic)**: if HasSampleEnded(VH[0]): unlock, VH[0]=-1; if CuePos==0 → SFXUpdateEnding;
     else `if (CueSampleWaiting[0]-- < 1) { StartSample(CuedSamples[--CuePos],..,0); CueSampleWaiting[0]=SetCueTimer(P); }`
     (VH[0] stays -1 during the gap, so HasSampleEnded(-1)=1 keeps the countdown going: about CueTimer+1 frames between samples).
   * **Polyphonic**: for i<SampleCount: if CSW[i]==0 → StartSample(CuedSamples[i],..,ch=i), CSW[i]=-1; elif CSW[i]<1 → if ended: unlock, VH[i]=-1 (else still busy); else CSW[i]--.
     When no channel started this frame and none is still playing (or waiting) → SFXUpdateEnding.
   * `pos = RealPos; ES_GetMyPos(item)`; for each ch with VH[ch]!=-1: refill psiSFX (volume recombined, Pitch[ch] and Pan[ch] are the values fixed at start) → psiUpdateSound
     (2D → Setup2D; 3D → Calculate3D (Is3dTracked is always true) + Setup3D). **So 3D pan/attenuation and all global volumes are recomputed every frame. Pitch is re-sent but never changes.**
   * Always: ES_HandleNisPauseSfx (PauseInNis sounds pause while NisPlaying or file access is suspended, unless globally SFXPaused).
5. Kill pass: every item with DeleteMe → KillSFX (for each VH != -1: SampleKeyOff + ES_UnLockVoiceHandle; then return the item to the free list).
6. Rebuild SFXActiveData: Count, then {HashCode, Handle} for each active item (list order, head = newest).

SFXUpdateEnding@4e20 [M]: if !P.Loop → DeleteMe=1. Else `if (CueSampleWaiting[0]-- < 1 && SFXSetup(item) && UpdatedCounter < 2) next = item;`
(re-trigger after the cue delay: re-picks a sample, re-runs all voice/limit checks, and processes the item again this frame).
HasSampleEnded@6ac: handle -1 → 1; KeyOnFrameDelay≥1 → 0; sample → !psiIsSamplePlaying (front or rear hardware voice playing); stream → ES_HasStreamEnded.

**Looped samples (SHF Flags&1) never "end"** (PS2_CheckKeyOn2 returns true while VoiceLooping[v]), so they play until the EE calls SFXRemove(handle,id)
(KillSFX → key-off), or until stolen, FadeAndKill'ed or reset. A natural end does not key-off; the voice is just unlocked [M].
"Playing" test PS2_CheckKeyOn2@da1c: true if ENVX != 0 at the last batch-end snapshot, or keyed on in this batch / the delayed batch / the previous batch (LastKeyOn), or looping.

## 7. Key-on dispatch (PS2_SendKeyONKeyOff@d544, end of every RPC batch except the stream-only commands) [M]
Snapshots ENVX (SD_VP_ENVX = 0x500) of all 48 voices (bit set = silent) → writes VMIXEL/VMIXER (reverb) → removes key-off voices from the key-on sets → KON = front voices.
KOFF = key-offs. If SFXMode != 0: DelayThread(500 us), then KON = rear voices (delayed rear). If SFXMode == 0, rear voices are keyed together with the front ones.

## 8. Pause
SFXPause/UnPause (global) and NIS pause: per item, every live voice gets pitch 0 (pause) or its saved pitch back (unpause). Paused items are skipped by SFXUpdate (no 3D update, no end test, Counter frozen) [M]. (Steal victims are filtered by IsStreamed, not Paused.)

Port (fix-pause-audio, s/fix-pause-audio): the pause is driven by the pause page, not by GameState+0x50 — GS_PauseGame 0x17d178 only sets GameState[0x50] + the MPGame pause words and calls no sound function, and no Sound_*/SFX* code reads +0x50; the actual sound pause is `MenuManager_Create` 0x21b908:121-124 (`pause_sound` set → `SFXPause` op 9 + `SFXPauseAllStreams` op 0x16) and `MenuManager_Delete` 0x21c2b0:11-22 (op 0xa + op 0x17) [M]. The port mirrors this: `Sound::set_paused` (nf-audio/sfx.rs) pitch-0+volume-0s every item live at the pause (the NIS-pause edge shape; the game's `psiSamplePause` 0xf46c saves/restores the pitch) and skips those items in `Driver::update` (the item's `Paused` word, +0x290); items started while paused are NOT paused — the menu's own sounds play under the pause [M §3.2 `StartedWhenPaused`]. The unpause emits nothing: the next full update's per-channel recompute restores pitch and volumes. The music queue's playhead freezes through `Output::set_paused` (op 0x16's stream hold, `GamePausedMe` — docs/systems/music.md §3.7) and resumes exactly. Host hook: `nf_view`'s `RedrawRequested` applies the hold while any menu page is up or the viewer pause holds the sim (the fail/results menu opens with `pause_sound` = 0 in the game — sound keeps playing [M Mission_Update 0x182c60:78-79]; the port's demo results hook rides the same gate, listed as remaining work). The EE keeps its batch flowing while paused in the game; the port's host stops ticking under a menu, so the pause edge is flushed to the output at once instead of waiting for the next batch.

## 9. Ambiguities / open points
* World units of positions are not visible in the IOP: distances use /100 and /1000 integer scaling. Check the EE-side conversion (SLUS) before mapping the radii to engine units [S].
* PS2_SetupReverb's sceSdEffectAttr local layout is partially mangled in the decompile (an extra 0x6FE0 local) [M ambiguous].
* SFXMode values other than 0 are not distinguished in the SFX path (init value 4) [M].
* SFXSetup's voice budget counts ES handles, not hardware voices. A 3D sound uses two SPU voices (front + rear) [M].
