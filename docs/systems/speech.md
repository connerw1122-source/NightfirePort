# Guards' speech and streamed effects

Build: ACTION.ELF `e7ffdcb3…`, SFX.IRX `65c4c97e…`. Tags: [M] code, [D] disc, [L] snapshot, [S] suggested. Written by a
research pass (subagent, 2026-10-09; PlaySFX / SpeechSFX / AnimScriptPeekSfx checked against its own disassembly) and
confirmed on all 823 STREAMS entries. Port: `nf_formats::sfx::voice_lines` (Drone_SfxInfo),
`nf_formats::soundbank::{stream_lut, stream_data}`, the driver's streamed sub-tracks (`nf_audio::sfx`, 2 slots),
the talk functions in `nf_game::drone` and `Drones::speech` (SpeechSFX / PlaySFX).

## Port status (WORKLOG 0069)

Done: Drone_SfxInfo, the voice set from the character id, PainTalk (hit leaves the drone alive), DeathTalk (OnInitDeath
unless a head-hit kill, LocationDeathAnim; the second skipped as the same line), AttackTalk (Attack entry, AimStandFire
and CombatNoMove entries, SniperAim entry, every SniperFire update), SeenPlayerTalk (first sight before the prop-0x1B
roll, CombatNewSighting entry), PatrolTalk's roll and global cooldown, SpeechSFX / PlaySFX gates (30-frame start, global
per-line repeat delay, facial lines not played directly, the per-drone current line), streamed effects (LUT / BIN, end
marker, 22 050 Hz, at most 2). *Suggested*: "first sight" = the drone's own sight on its first Attack entry (the game:
`+0x4F8 & 0x8000000` from `EnemyLookForOpponent`); the AimStandFire talk skips stationary (sniper) drones only (`+0xC5`,
`+0x4F8 & 0x10`, prop 0x40 not modelled); the stream's CD latency is not modelled. Not yet: the facial path (no facial
scripts load on 07000005), HitTalkExtra (type 0x10), the level-specific sets (0x0C / 0x0D / 0x14–0x16 / civilians),
drone foley via PlaySFX (doors 0x27, kicks 0x28 — those states are not ported).

---


Build: ACTION.ELF (decomp `work/decomp/ACTION.ELF/`, disassembled from `ee_start_05.bin` where the decomp was unclear),
SFX.IRX (decomp `work/decomp/SFX.IRX/`). Tags: **[M] fn@addr** read in code, **[D]** measured on disc files,
**[L]** read from `ee_start_05.bin` (07000005 level start), **[S]** inference. Music is out of scope
(the stream system is shared with music; only the SFX-stream side is specified here).

Helpers written for this pass (scratchpad): `dis.py` (small R5900 disassembler over the snapshot), `sfxinfo.py`
(Drone_SfxInfo dump + hashtable/anim-script peek + SB_4 lookup; full dump in `sfxinfo_all.txt`), `drones.py`
(snapshot drone fields), `streams_sb4.txt` (every stream ref in SB_4 with its LUT entry, end marker and length).
Disc files extracted to `work/audio/STREAMS.LUT` (26 896 B) and `work/audio/STREAMS.BIN` (51 394 240 B).

Common definitions:
- `now` = `GameState+0x34` (0x2a379c), u32 game-frame counter; not reset per level (108153 at 07000005 start) [L].
- `FRAME_RATE_INT` = 60, `FRAME_RATE_DIV` = 1.0 [L].
- `Rand(n)` = `Rand_Rand@1e3738`: `Y = b*(Y&0xffff) + (Y>>16); X = a*(X&0xffff) + (X>>16); return ((Y&0xffff)*n) >> 16`
  (u32 maths; `Rand(0)` = 0 but still advances the seeds) [M disasm]. It is the shared game RNG, so every talk roll
  below consumes it in the order given.
- `DCVars` (passed by value to all talk functions): `[0]` obj, `[1]` Drone*, `[3]` = &drone+0x108 (state-machine info)
  [M Drone_BrokenObject@138588 builds one by hand].
- `MP` = `MPSettings+0x184` (0x2a4924) ≠ 0 [M]. 0 in single player [L].

---------------------------------------------------------------------------------------------------

## 1. Drone_SfxInfo, NDrone2_PlaySFX, NDrone2_SpeechSFX(d), DroneFunc_ChooseSFX

### 1.1 Drone_SfxInfo (0x2812d0, 346 × 32 bytes) [M PlaySFX/SpeechSFX disasm; L values]

| off | type | field | notes |
|---|---|---|---|
| +0x00 | u32 | sfxId | 0 = no sound (PlaySFX returns 0) |
| +0x04 | u32 | facialAnimId | anim-script hash (0x06xxxxxx); 0 = none. **Written to 0 by SpeechSFX** when the hashtable lookup fails (permanent for the session) |
| +0x08 | u32 | lastFrame | `now` of the last accepted play; **global per entry, shared by all drones** |
| +0x0C | u32 | repeatDelay | frames (× FRAME_RATE_DIV) |
| +0x10 | f32 | volume | passed to Sound_Play3D (ignored downstream, sound.md §1) |
| +0x14 | f32 | outer | × scale, passed to Sound_Play3D (ignored: custom = 0) |
| +0x18 | f32 | inner | × scale (ignored) |
| +0x1C / +0x1E | s16 | pitchMin / pitchMax | if the u32 at +0x1C ≠ 0: pitch = (s16)(min + Rand(max−min)); pitch is ignored downstream |

[L] 336 of 346 entries are in use (index 0 is empty); **all** have vol 100, outer 15, inner 10, pitch 0/0. 61 have a facial
anim id; on 07000005 **none of the 61 anim scripts is in the hashtable** (all lookups fail). repeatDelay values:
0 ×107, 30 ×11, 60 ×4, 120 ×111, 180 ×79, 240 ×2, 280, 300 ×6, 600 ×10, 900 ×3, 960 ×2.
Some lastFrame values are non-zero at level start (e.g. idx 5/0x13 = 53899, idx 4 = 7999, 0x35 = 7388) [L] — left over
from earlier play, harmless because `now` only grows [S]. A port that restarts its frame counter per level must zero
lastFrame at level load [S].

### 1.2 NDrone2_PlaySFX(float scale, DCVars* dcv, uint idx, VECTOR* pos, Sc enforceDelay) → handle @141848 [M disasm]
Registers: f12 = scale, a0 = dcv, a1 = idx, a2 = pos, a3 = enforceDelay.
```
e = idx < 346 ? &Drone_SfxInfo[idx] : NULL          // NULL is dereferenced (crash) — no caller passes ≥346
if e.sfxId == 0: return 0
if (float)now < (float)DronePostLoadFrame + FRAME_RATE_DIV*30: return 0   // DronePostLoadFrame = *(0x298fb8)
if enforceDelay && now < fptoui((float)e.lastFrame + (float)e.repeatDelay*FRAME_RATE_DIV): return 0
pitch = (u32 at e+0x1c) ? (s16)(e.pitchMin + Rand(e.pitchMax - e.pitchMin)) : 0      // never taken [L]
e.lastFrame = now                                   // set before playing, i.e. even if the play fails
p = pos ? *pos : (dcv ? obj(dcv[0])+0x30 : (0,0,0)) // dcv==0 && pos==0 → origin, and dcv[3] below reads address 0xC
h = Sound_Play3D(e.sfxId, &p, vol=e.volume, outer=e.outer*scale, inner=e.inner*scale, pitch, ref=*(u32*)dcv[3], custom=0)
if h: e.lastFrame = now ; else idx = 0
*(drone(dcv[1])+0x4d4) = now                        // "last voice time", read by HitTalkExtra / SeenPlayerTalk(type 0x10)
ring[ringPos] = {idx, h}; ringPos = (ringPos+1) & 15  // ring @0x29aa9c (16×{u32,u32}), pos @0x29ab1c; write-only (debug)
return h
```
`DronePostLoadFrame` (0x298fb8) is set to `now` by `Drone_PostLoad_Init@136938` [M]; at the snapshot it is 108150, so
drone voices open at frame 108180 [L].
Position: the sound is fixed at the drone position **at the moment of the call**; nothing calls Sound_SetPosition on
drone voice handles (only Bullet_update / Car_Update / Script_Interp do, sound.md §3) [M].

Direct PlaySFX callers (all `scale 1.0, pos 0 (= obj+0x30), enforceDelay 1`) [M]: BOT_handlePain / BOT_soundEffect (MP
bots), DroneSpawner_Control (idx 0xd6 + Rand(2), dcv = 0, levels 0c/0d only), CivilianPatrol 0x48, Interogate 0x4a,
**OpenDoor 0x27** (id 1477, in SB_4 sample 10), **KickObject 0x28** (id 1478, not in SB_4 → IOP ignores it),
SeenSurrenderedDrone 0x23 (sfxId 0 → nothing), HostageDie 10.

### 1.3 NDrone2_SpeechSFX(DCVars* dcv, uint idx, Sc force, Sc useFacial, Sc facialOnly) → handle @141b00 [M disasm]
```
if MP: return 0
d = dcv[1]
if (float)now < (float)DronePostLoadFrame + 30*FRAME_RATE_DIV: return 0
e = &Drone_SfxInfo[idx]                                  // NULL if idx ≥ 346 (crash)
if now < fptoui((float)e.lastFrame + (float)e.repeatDelay*FRAME_RATE_DIV): return 0   // ALWAYS enforced, even with force
if useFacial:
    if e.facialAnimId == 0:
        if facialOnly: goto FACIAL(fid = 0)
        goto DIRECT
    if hashtable_getitem(e.facialAnimId) == 0:           // script not loaded on this level
        e.facialAnimId = 0; goto DIRECT                  // permanent
    goto FACIAL(fid = e.facialAnimId)
DIRECT:
    if e.sfxId == 0: return 0
    if e.facialAnimId != 0: return 0                     // an entry with a (loaded or not-yet-tested) facial anim never plays directly when useFacial = 0
    if !force && d+0x4d0 != 0: return 0                 // per-drone "a line of mine is still referenced"
    h = NDrone2_PlaySFX(scale 2.0, dcv, idx, obj+0x30, enforceDelay 0)   // radii ×2 are ignored anyway
    d+0x4d0 = h                                          // overwritten even when h == 0
    return h
FACIAL(fid):
    if fid != 0 && d.byte[0x36] == 0 && d.byte[0x37] == 0:      // (u64 at d+0x30) & 0xffff<<48: no facial line playing or pending
        sfx = AnimScriptPeekSfx(fid)
        if sfx != 0:                                     // the facial script carries its own sound (op 1)
            d+0x4c = fid; d.byte[0x37] = 1; d+0x48 = now + 20
            SFXPreCache(sfx, 20)                         // EE op 0x26 {id, 20}
        else:                                            // mouth-only facial anim, appended at once
            a = AnimFacialAppend(obj, fid); d+0x53c = a
            if a: d.byte[0x35] = 0; d.byte[0x36] = 1; a+0x60 = &d.byte[0x35] (done flag); a+0x64 = &d+0x53c;
                  a+0x78 &= 0x7fffffff (not looping); a.byte[0xb5] = 3 (stop at end)
            d.byte[0x37] = 0
    if d+0x53c != 0: e.lastFrame = now                  // repeat delay armed only if a facial anim object is attached
    return 0                                             // facial lines never return / store a handle
```
Notes:
- Facial lines do not touch drone+0x4d0, so `Drone_IsTalking` and the talk functions' "still talking" tests do not see
  them; the per-drone facial lock is bytes +0x36 (playing) / +0x37 (pending) [M].
- With `force` the previous line is **not stopped**; it keeps playing and +0x4d0 is overwritten (two lines can
  overlap; the IOP MaxVoices = 1 per id still applies to identical ids) [M].

`NDrone2_SpeechSFXd(Drone*, idx, force, useFacial, facialOnly)` @149d58: builds a DCVars from `drone+0xC`
(`Drone_DCVfromOBJ`) and calls SpeechSFX [M].

### 1.4 Pending facial line, per frame — NDrone2_ControlSTANDARD@1490f0 [M]
Every frame, for each controlled drone, after `Drone_SM_SendMsgSelf(3)` (the state's update, where most talk calls are):
```
if d+0x4d0 && Sound_IsFinnished(d+0x4d0): d+0x4d0 = 0
if d.byte[0x36]:                                   // facial anim playing
    if d.byte[0x35] || d+0x53c == 0 || (d+0x53c)->flags(+0x78) & 0x10000000: d.byte[0x36] = 0
if d+0x4c == 0: d.byte[0x37] = 0
if d.byte[0x37] && (u32)(d+0x48) < now:            // first frame with now > T+20, i.e. T+21
    a = AnimFacialAppend(obj, d+0x4c); (same field setup as above); d.byte[0x37] = 0
```
`Sound_IsFinnished(h)` = `h ? node.state == 2 : true` (state 3 = freed returns false) [M @1de240]. Because
Sound_UpdateSounds runs before the drones each frame (state 1 → 2 in frame N, freed in frame N+1), the drone sees state
2 in frame N and clears +0x4d0 [M Game_Run order, sound.md §0]. A drone not controlled in that frame would keep a stale
handle that may alias a reused node [S].

### 1.5 DroneFunc_ChooseSFX(a, b) @1497e8 [M]
`return a + Rand(b − a + 1)` (inclusive range). Used only by Drone_BrokenObject (0x40..0x43, level 07000001).

### 1.6 AnimScriptPeekSfx(hash) @1c6ca8 [M disasm]
`s = hashtable_getitem(hash)`; if none → 0. `n = u16(s+6) & 0x1ff`; commands start at s+8, each `c = u16`, op = c & 0xff,
len = c >> 8. For up to n commands: op 1 → return `u16 cmd[2]` (the sfx id); op 0 → advance `2*(len+4)` bytes;
op 2..4 → advance `2*(len+3)` bytes; op ≥ 5 → return 0. End → 0. (Only the **first** op-1 sound is precached.)

---------------------------------------------------------------------------------------------------

## 2. Talk functions

### 2.1 Voice-set selection (per drone) [M NDrone2_DefaultInit@14b300]
`drone+0xd8` (u16, "voice type") and `+0xda` come from the character id `drone+0xe0` (placement +0x34):

| character ids (0x05000000+) | +0xd8 | extra |
|---|---|---|
| 04, 06, 08, 4c | 3 | |
| 0e, 0f | 0x15 | female (+0x15 = 1) |
| 10, 11, 52 | 0x16 | female |
| 12, 13, 14 | 0x11 | |
| 15, 20, 22, 23, 24 | 0xd | female |
| 16, 8f, ac | 0x14 | |
| 17, 18 | 0x12 (da 0xe) | female |
| 19, 21 | 0xe | |
| 1b, 4f | 9 | |
| 1e | 0xb | |
| 25 | 5 | |
| 26, 66 | 0x10 | |
| 2b, 2d, 34 | 8 | female |
| **2e, 2f, 30** | **2** | (07000005 guards) |
| 35 | 0xf | |
| 36..3a | 1 | |
| 3b, 3c | 0xc | female |
| 27..2a, 31..33, 53 | 0x17 | |
| 54, 6d, 6e, 7e | 4 | |
| 5e | 0x13 (da 0xd) | female |
| 78, 94, ba | 0x18 | |
| 87, 90, 93 | 6 | |
| 8c, b1, b2 | 7 | |
| other | 0 | |

`drone+0x15` (female flag) changes the default pain/death lines; `*(u32*)(drone+0x108) & 1` (first word of the
state-machine info) picks the alternate set on levels 0x14..0x16 [M]. `drone+0xac` = health; every talk needs > 0
unless noted.

[L] **07000005**: all 13 drones (incl. the sniper) have character 0x0500002e/2f/30 → **+0xd8 = 2**, +0x15 = 0.

### 2.2 Gates common to the talk functions [M]
"Busy" test used by most talks: `if d+0x4d0 && !Sound_IsFinnished(d+0x4d0) → busy; else if finished → d+0x4d0 = 0`.
There is **no global "someone is talking" lock** in the speech path. Cross-drone limits are only:
(a) the per-entry `lastFrame/repeatDelay` (global per line), (b) `PatrolCooldown` 0x298fc4 (PatrolTalk),
(c) `counter_210` (HandleTalking, 0x7000008 only), (d) `CoverTalkCooldown` 0x298fc8 (written by the cover talks, never
read), (e) for HeardNoiseTalk/LostSightTalk, "no drone saw the opponent last frame" (0x298f84).

### 2.3 Per function

**NDrone2_PainTalk @13eeb8** — called when a hit leaves the drone alive: `NDrone2_HitDamage@145558` (bullets, via
DoHitEffects) if health > 0 after damage; `NDrone2_PunchImpact@145f28` (not MP, alive); `NDrone2_ExplosiveImpact@146480`
(both branches); Surrender_Anim / Surrendered / Stunned / StunGrenadeLoop / StunDartLoop / Ninja* states.
```
if MP: return 0
idx by +0xd8: 1→0x12, 2→0x13, 3→0x11, 7→0x14, 8/10→0x18, 9→0x15, 0xc→0x1b, 0x12/0x13→0x1c, 0x14→0x10,
              0x10→ (HitTalkExtra() ? return 0 : 0x16), default → (+0x15 ? 0x17 : 0xf)
cur = d+0x4d0 ? node(+0x34) & 0xfffff : 0xffffffff
if Drone_SfxInfo[idx].sfxId != cur: SpeechSFX(idx, force=1, 0, 0); return 1      // skipped only if the same id is the current line
```
No health check, no busy test, no random gate. Not suppressed by a line in progress (force).

**NDrone2_HitTalkExtra @13ed38** (only from PainTalk, type 0x10): MP → 0; busy → 0; health ≤ 0 → 0;
if `now < d+0x4d4 + 3*60` and `Rand(5) == 0` → 0; type 5..7 on levels 0x14/0x15/0x16 → 0x133 + Rand(5) (alt 0x138+);
type 0x10 → 0xec + Rand(7); SpeechSFX(idx, 0,0,0) → 1.

**NDrone2_DeathTalk @13eff8** — callers: `DroneFunc_OnInitDeath@145bd0` (only if `d.byte[0x3a] == 0`),
`DroneAnim_LocationDeathAnim@13c780` (sets `+0x3a = 0` and calls DeathTalk unless it takes the anim-0x42 branch, which
needs obj+0x470 set, the player weapon datum flag `(+0x60 & 1)` and impact type (+0x52) 4/5), `DSTATE_RunAwayFromObject`
(enter). `Death_Anim` (0x44) enter runs OnInitDeath then LocationDeathAnim [M 1637f8]. `+0x3a` is set to 1 by a head
hit in HitDamage (part 5), so OnInitDeath stays silent for head-shot kills but LocationDeathAnim still talks.
```
if MP: return 0
idx by +0xd8: 1→4, 2→5, 3→3, 8/10→10, 0xc→0xd, 0x10→8, 0x12/0x13→0xe, 0x14→2, default→ (+0x15 ? 9 : 1),
              5/6→ 1, or on levels 0c/0d: return 0 if the current line is entry 0xcd's or 0xd2's id, else Rand(2) ? 0xcd : 0xd2
same "skip if the same id is the current line" test as PainTalk; SpeechSFX(idx, force=1, 0, 0); return 1
```
The second DeathTalk of a death sees the first one's id and is skipped [M]; entry 5 has repeatDelay 30 (global).

**NDrone2_AttackTalk @13eac0** — callers: `DroneFunc_FirstAttack@148010` (from DSTATE_Attack 0x56 enter; reached when
FirstSightState returns no state, or when not a first sight), `AimStandFire` enter (non-sniper `+0xc5 ≠ 2`,
`+0x4f8 & 0x10` clear, no prop 0x40), `CombatNoMove` enter (non-sniper), `UnderCoverFire` enter, `SniperAim` enter,
**`SniperFire` msg 3 (every frame)**, `AbseilSlide`/`AbseilHang` msg 3 (every frame), `PlayScript` (several places).
```
if MP: return 0; if busy: return 0; if health ≤ 0: return 0
odds = 0; idx = 0
switch +0xd8:
  1: levels 0x11/0x0b/0x13 → 0xa3 + Rand(8); else odds 2, idx = 0x33 + Rand(4)
  2: odds 3, idx = 0x37 + Rand(9)                          ← 07000005
  3: if (d+0x228 & 8) return 0; idx = 0xac + Rand(7)
  5/6: levels 0c/0d and 3·60 < d+0x274 < 4·60 → 0xd3 + Rand(3)
  7: levels 0x14..0x16 → 0x11b + Rand(7) (alt 0x122+)
  0x11: level 0x07 → odds 3, 0x93 + Rand(2)
  default: return 0
if idx == 0: return 0
if odds == 0 || Rand(odds) == 0: SpeechSFX(idx, 0,0,0); DroneFunc_TargetSeen(d, d+0x250); return 1
return 0
```
RNG order for type 2: Rand(9) then Rand(3).

**NDrone2_SeenPlayerTalk @13f1d0** — callers: `DroneFunc_FirstSightState@147c48` (after the prop-0x43 surprise check and
the prop-0x1a level returns), `DSTATE_CombatNewSighting` (0x5B) enter.
```
if busy: return 0 (clears a finished +0x4d0); if health ≤ 0: return 0      // no MP test here (SpeechSFX has one)
if Rand(3) != 0: return 0
if +0xd8 == 0x10: return 0      // (then dead code for 0x10 below)
switch level: 05/06/07/08 → +0xd8 == 2 ? 0x49 + Rand(6) : 0x21            ← 07000005 type 2: 0x49..0x4e
              09/0a → 0xc3 + Rand(9); 0x12/0x4a → 0xac + Rand(7);
              0x14/0x15/0x16 → Rand(2) ? (0xf3/0xf7/0xfb + Rand(2), alt +2) : (0x10d + Rand(2), alt 0x10f+)
if idx == 0: return 0
d+0x4d0 = SpeechSFX(idx, 0,0,0); return 1
```
Entry 0x21 has sfxId 0 (silent) [L].

**NDrone2_PatrolTalk @13e988** — `DSTATE_Patrol` msg 3 (every frame, before UpdatePatrolRoute).
```
if MP or health ≤ 0 or busy: return 0
if now < PatrolCooldown(0x298fc4): return 0
if Rand(2) != 0: PatrolCooldown = now + 5·60; return 0
if +0xd8 != 3: return 0                      // types ≠ 3 roll every frame until a 1 comes up (no cooldown on that path)
idx = 0xb3 + Rand(5); PatrolCooldown = now + 10·60; SpeechSFX(idx, 0,0,0); return 1
```
On 07000005 (type 2) it never speaks but **consumes Rand(2) each patrol frame** while the cooldown is open and sets the
5-s global cooldown half the time [M].

**DroneFunc_HeardNoiseTalk @149920** — `HeardNoiseSuspect` (0xA4) enter (only when the noise level +0x43 < 3) and
`HeardNoiseAlert` (0xA5) enter. If no drone saw the opponent last frame (0x298f84 == 0), not busy, health > 0 and
+0xd8 == 1: `SpeechSFX(0x26, force 0, useFacial 1, facialOnly 0)` (id 114, repeat 180). Type 2 → nothing.

**DroneFunc_NewSightTalk @13f420 / DroneFunc_LostSightTalk @13f550** — from `DroneVision_ProcessDroneSight@176a28`:
NewSight when a drone has had sight for > 60 frames and the global "sighted" latch 0x298fa8 is clear (then latch
fa8 = 1, fa9 = 0); LostSight when a drone (`+0x228 & 8`) has been unsighted for > 120 frames, the "lost" latch fa9 is
clear and nobody saw the opponent last frame (then fa9 = 1, fa8 = 0). Both: MP / busy / health gates; only +0xd8 5/6 on
levels 0c/0d: NewSight 0xd6 + Rand(2); LostSight 0xd3 + Rand(3) if the drone's target (+0xc → +0x20) is the player;
then SpeechSFX(…,0,0,0) + DroneFunc_TargetSeen. Nothing on 07000005.

**DroneFunc_InjuredTalk @13fa20 (from NDrone2_BulletImpact), RunForCoverTalk @13fd08 (DSTATE_RunForCover),
UnderCoverTalk @13fe80 (UnderCoverInit / UnderCoverReturn), UnderCoverAimTalk @13fb98 (UnderCoverAim)** — all four
**return at once unless MP ≠ 0** (verified in the disassembly, `beq v0,zero` on MPSettings+0x184), and SpeechSFX refuses
in MP → **dead in both modes**. (Would-be ranges: levels 0c/0d, 0x11..0x13/0x4a, 0x14..0x16; final gate `Rand(0) == 0`
always true; set CoverTalkCooldown = now + 5 s.)

**NDrone2_HandleTalking @13f688** — every frame from ControlSTANDARD (if `Drone_bDisableSystem` clear). Level 0x7000008:
type 0xc only, player within 3 units (SqDist2D < 9), `+0xcf0 ≠ 99`, `now ≥ +0xcf4`, global throttle `counter_210`
(30 frames), `NDrone2_CanSeeObject(…,5,0)`: switch channel 0x4a → SpeechSFX(0x9e, force) and `+0xcf0 = 99`; else a
0→1→2→1… cycle of 0x9b / 0x9c / 0x9d (force 1, useFacial 1) with `+0xcf4 = now + 5 s` (first) / 15 s. Levels 0c/0d →
`NDrone2_PowerstationTalk`. Nothing on 07000005.

**NDrone2_BondTalk @1446b0** (from Civilian / CivilianMission(Wait) / PartyGirl / CivilianGuard states) — only level
0x7000007, unarmed civilians facing Bond (|+0x1d4| ≤ π/2) within 3 units; per +0xd8 (9: 0x95 + Rand(2); 0xd: 0x58 +
4·Rand(11) + variant by character; 0xe: DoBondTalk(0x84 + Rand(9), …); 0x10: 0x97 + Rand(4); 0x11: 0x8e; 0x12: a
7-step jump table) with SpeechSFX(…, 0, useFacial 1, facialOnly 1), head tracking, and cooldowns 480/600 frames in
`+0xcdc` / counters `+0xcd8`. **DroneFunc_DoBondTalk @144488** (idx, headTrackTime, delay, Sc again) is the shared form:
needs `+0x2a` (Bond in range), `now ≥ +0xcdc`, `+0xce0 == 0` or `again`; on success (handle or a facial anim attached)
`+0xcd8++`, `+0xcdc = now + delay`, `+0xce0++`. Not on 07000005.

**DroneVision_ConsiderAlerted @176250** — only on levels 0c/0d for +0xd8 5/6: if `!Drone_IsTalking(d)` →
`SpeechSFXd(d, 0xd0, 0,0,0)`. Otherwise no speech. `Drone_IsTalking@149798` = the busy test (clears a finished handle).

### 2.4 Summary for 07000005 guards (type 2)
| trigger | function | idx (inclusive) | odds | force |
|---|---|---|---|---|
| hit, survives | PainTalk | 0x13 | always (unless same id current) | 1 |
| death | DeathTalk (×2 calls, second skipped) | 5 | always | 1 |
| Attack enter / AimStandFire / CombatNoMove / SniperAim enter; **SniperFire every frame** | AttackTalk | 0x37..0x3f | Rand(9) then 1/3 | 0 |
| first sight / CombatNewSighting enter | SeenPlayerTalk | 0x49..0x4e | 1/3 then Rand(6) | 0 |
| patrol | PatrolTalk | — (type 3 only) | consumes Rand(2) | — |
| door open / kick | PlaySFX foley | 0x27 / 0x28 | repeat 0 | — |

---------------------------------------------------------------------------------------------------

## 3. Reachable Drone_SfxInfo entries on 07000005 [L snapshot; D SB_4.SFX + STREAMS]

Bank loaded for 07000005 = hash 5 = slot 4 = **SB_4** only (Sound_Ready, sound-bank.md). All rows: vol 100, outer 15,
inner 10, pitch 0, facial 0.

| idx | use | sfxId | repeat | SB_4 | sub-tracks / FileRef | kind |
|---|---|---|---|---|---|---|
| 0x13 | pain | 138 | 0 | yes (T2, in 2/out 30, MaxV 5, Pri 50) | random 1 of 14: 193,190,191,192,189 + 9× FileRef 0 (near-silent sample 0) | bank samples |
| 0x05 | death | 139 | 30 | yes (T2 2/30) | random 1 of 4: 185,186,187,188 | bank samples |
| 0x37 | attack | 175 | 180 | yes | −212 → LUT 211 | stream |
| 0x38 | attack | 177 | 180 | yes | −214 → LUT 213 | stream |
| 0x39 | attack | 179 | 180 | yes | −216 → LUT 215 | stream |
| 0x3a | attack | 185 | 180 | yes | −222 → LUT 221 | stream |
| 0x3b | attack | 192 | 180 | yes | −229 → LUT 228 | stream |
| 0x3c | attack | 193 | 180 | yes | −230 → LUT 229 | stream |
| 0x3d | attack | 194 | 180 | yes | −231 → LUT 230 | stream |
| 0x3e | attack | 195 | 180 | yes | −232 → LUT 231 | stream |
| 0x3f | attack | 198 | 180 | yes | −235 → LUT 234 | stream |
| 0x49 | seen | 191 | 0 | yes | −228 → LUT 227 | stream |
| 0x4a | seen | 192 | 0 | yes | −229 → LUT 228 | stream (same id as 0x3b) |
| 0x4b | seen | 194 | 0 | yes | −231 → LUT 230 | stream (= 0x3d) |
| 0x4c | seen | 195 | 0 | yes | −232 → LUT 231 | stream (= 0x3e) |
| 0x4d | seen | 186 | 0 | yes | −223 → LUT 222 | stream |
| 0x4e | seen | 178 | 0 | yes | −215 → LUT 214 | stream |
| 0x27 | OpenDoor foley | 1477 | 0 | yes (T2 1/25) | sample 10 | bank sample |
| 0x28 | KickObject foley | 1478 | 0 | **no** | — | IOP "not found" → silent |
| 0x21 | seen (non-type-2 on 05..08) | 0 | 0 | — | — | nothing |

Every speech stream entry above has SFXParameters: TrackingType 2, Inner 5, Outer 30, MaxVoices 1, Priority 99,
ReverbSend 50, Ducker 10 (no effect: duck only for tracking 0/3), DuckerLength 600..1500, MasterVolume 100, one
sub-track with BaseVolume 80 and no pitch/volume/pan randomisation [D sfxparse]. SFXOutputData for them [L]: EE inner 5,
outer 30, alert weight 0, cull flag 1 (not started if the listener is > 33 units away), +0x10 duration = exactly the
stream's end-marker length (e.g. id 175: 0.5944 s = 7490/16·28/22050).
Note that the repeat delay is per **entry**, while the same sfx id appears in two entries (192/194/195): the attack and
seen lines can repeat each other immediately [L].

Effective stream lengths (end marker, §4.3) [D]:
| id | LUT | data size | end marker (bytes) | bytes delivered = marker & ~255 | duration |
|---|---|---|---|---|---|
| 175 | 211 | 14816 | 7490 | 7424 | 0.589 s |
| 177 | 213 | 24608 | 14326 | 14080 | 1.117 s |
| 178 | 214 | 14544 | 8195 | 8192 | 0.650 s |
| 179 | 215 | 12720 | 12679 | 12544 | 0.996 s |
| 185 | 221 | 18688 | 14502 | 14336 | 1.138 s |
| 186 | 222 | 17728 | 12341 | 12288 | 0.975 s |
| 191 | 227 | 18688 | 9753 | 9728 | 0.772 s |
| 192 | 228 | 23216 | 18655 | 18432 | 1.463 s |
| 193 | 229 | 19568 | 17691 | 17664 | 1.402 s |
| 194 | 230 | 19568 | 19529 | 19456 | 1.544 s |
| 195 | 231 | 25824 | 10327 | 10240 | 0.813 s |
| 198 | 234 | 23216 | 17530 | 17408 | 1.382 s |
(durations = delivered/16·28/22050). The whole SB_4 stream list (35 refs) is in `streams_sb4.txt`.

---------------------------------------------------------------------------------------------------

## 4. Streamed SFX

### 4.1 STREAMS.LUT [D, M SFXInitialiseAudioStreamSystem@65d8]
`PS2/ENGLISH/STREAMS/STREAMS.LUT` (26 896 B = 1681 × 16). Path built as `"%s%s/STREAMS/STREAMS.LUT" (STRFilePath,
SFXLanguage)`; the IOP copies **3000** entries (reading past the file's 1681) into `StreamLookupFileDataStore`.
Entry: `{u32 headerOffset, u32 headerSize (=136 always), u32 dataOffset (= headerOffset + 4096 always), u32 dataSize}`.
Entries 0..822 tile STREAMS.BIN exactly (entry 822 ends at 51 394 240 = file size); 823..1680 point beyond the file
(other builds/languages; unused by SB_4, whose refs are −823..−102) [D].
**FileRef → entry: `index = −1 − FileRef`** [M SFXInitialiseStreamUpdate@92dc: `-1 - stream.track`].

### 4.2 STREAMS.BIN entry [D, M]
At `headerOffset`: a 136-byte header (rest of the 4096-byte block is zero); at `dataOffset`: mono SPU ADPCM.
Header (identical layout in all 823 entries):
| off | value | meaning [M UpdateMusicMarkers@b560 / SFXInitialiseStreamUpdate] |
|---|---|---|
| +0x00 | 1 | ? |
| +0x04 | 2 | number of 32-byte marker records [S] |
| +0x08 | 20 | offset of the 52-byte (0x34) record table (`stream+0x3e4`) |
| +0x0C | 72 | offset of the 32-byte marker table (`stream+0x3e0`) |
| +0x10 | 100 | copied to `stream+0x2c` (a volume; unused for SFX streams [S]) |
| +0x14..+0x47 | one 0x34 record: {0,0,10,2,0…} | jump/section record (music) |
| +0x48 | marker 0: {0, pos 0, type 10, 2, 0,0,0,0} | start |
| +0x68 | marker 1: {−1, **pos = end byte offset**, **type 9 (end)**, 2, 0, 0, 1, 0} | end |
Marker record = `{+0 id, +4 position (bytes into the data), +8 type (9 end, 5 pause, 6/7 jump/loop, 10 start), …, +0x14,
+0x1c link}` [M UpdateMusicMarkers].
Data: standard PS-ADPCM, 16-byte blocks; begins with a zero block and a `0c 00` silent block; **the first block with
the end flag (byte 1 bit 0) starts at `(marker // 16 + 1) * 16`** (all 823 entries [D]); it is followed by a `0c 07`
block. The bytes after that are stale leftovers of older/longer takes (several entries contain extra end-flagged
sections), so the LUT `dataSize` must **not** be used as the length [D]. Decoded example, entry 211 (id 175): header
words `1,2,20,72,100 | 0,0,10,2,0… | 0,0,10,2,0,0,0,0 | −1,7490,9,2,0,0,1,0`; end flags at data+7504 (1) and +7520 (7),
then again at 9408/9424 and 14784/14800 (stale) [D].
Format: **mono** (`stream+0x18 = 1`), **22050 Hz** (`stream+0x3c = 0x5622`; SPU pitch 0x759 = 22050·4096/48000) [M].
(Music: 2 ch, 32000 Hz, `MFX_n.SSD/.SMF` — not specified here.)

### 4.3 IOP path [M]
1. `StartSample@3524` with `FileRef < 0`: `ES_RequestVoiceHandle@31c` → `SFXInitialiseStream(FileRef, Is3d)`; then an
   ES handle with `type = 1 (stream), ref = stream slot`. Base pitch 0x759, `item.IsStreamed = 1`, volume/pitch/pan
   randomisation as for samples.
2. `SFXInitialiseStream@9030(track, is3d)`: slots are `Streams[3]` (0x3fc bytes each). **A negative track searches
   slots 1..2 only** (slot 0 is music); first slot with `state(+8) == 0`; none → −1 (the sound fails, item DeleteMe).
   So **at most 2 streamed SFX at once**. It queues `{track, slot}` in `StreamWaitingIntialise[64]` (ring), sets
   `+0x58 = is3d, state = 1, +0x50 = 0 (no key-on yet), +0x60 = queue index` and runs `SFXInitialiseStreamUpdate`.
   (If `ES_FindFreeVoiceHandle` then fails, the slot is left in state 1 — leak [M].)
3. `SFXInitialiseStreamUpdate@92dc` (also called every `SFXUpdateStreams`), one queued stream at a time
   (gp−0x7f1c: 2 = idle, 3 = loading): clears the slot (keeping buffers), `idx = −1 − track`, channels 1, rate 22050,
   opens `"%s%s/STREAMS/STREAMS.BIN"` and async-reads `LUT[idx].headerSize` bytes at `LUT[idx].headerOffset` into the
   slot's header buffer; `+0x44 = LUT[idx].dataOffset`. Next pass, when the read is done: table pointers from the header,
   reopen STREAMS.BIN for data, `+0xa8 = 0` (read position), `+0x3c8 = 100` (shutdown volume), state 2, `+0x40 = 1`.
4. `SFXUpdateStreams@a1a0` (per SFXUpdate) for each initialised slot: `UpdateMusicMarkers` plans the next read
   (≤ 0x3c00 B per buffer, 4 buffers for mono): walking the marker table it stops the read at
   **`StreamAlign(endPos) = endPos & ~0xff`** for a type-9 marker, sets an end marker at that buffer position and
   "no more reads" (+0x98). Reads go `psiAsyncReadFile(fd, dataOffset + pos, size)`; `ES_UpdateStreamMemCpy` copies
   ≤ 0x1800 B per update into the SPU ring (15360 B, voice from `PS2_CreateStreamVoiceHandles`), sets `+0x4c = 1` after
   the first chunk.
5. Key-on: `SampleKeyOn` → `ES_StreamKeyOn@7e84(slot, reverb)` sets `+0x50 = 1, +0x54 = ReverbSend` (or into the
   waiting-init record while the slot is still initialising). The voice actually starts
   (`psiStreamPlay` → `psiSampleKeyOn` of the stream voice, front + delayed rear) in SFXUpdateStreams once
   `+0x4c && !+0x48 && +0x50`; then `+0x48 = 1`. So a fresh stream starts a few IOP frames + CD seek after SFXStart3D
   [S: number of frames depends on disc timing].
6. End: each update `+0xc0 = psiStreamGetPlayPos` (SPU address − ring base); when the play position passes the end
   marker (`ES_CheckEndMarker`, wrap-aware) and no pause flag (`+0x20`), `SFXStopStream`: volume 0, stop, state 10.
   Also stopped: after 30+ updates with no data left and the "no data" marker passed ("No Data Stop Mono Stream"), or
   200 updates without progress ("Stream Fatal Error Time Out"). `HasSampleEnded` → `ES_HasStreamEnded` (state 10) →
   SFXUpdate unlocks the handle (`ES_UnlockStream`: state 0, slot free) → `SFXUpdateEnding` → DeleteMe; the item leaves
   the active list, so the EE node dies after its TTL. Audible length = `endPos & ~0xff` bytes of ADPCM (plus up to one
   update of ring data before the stop is seen [S]).
7. Early stop: `KillSFX` → `SampleKeyOff` → `ES_StopStream` → `SFXStopStream` (by SFXRemove, MaxVoices stealing,
   FadeAndKill, …). Streams are never voice-budget victims (`IsStreamed` excluded) [M SFXSetup].

### 4.4 Volume, 3-D, pitch of a streamed SFX [M]
Identical to a bank sample of the same SFXParameters: `ES_FillpsiSFXData` → Volume = `ES_CombineVolumes(BaseVolume)`
× `+0x3c8 (100)`/100; Is3d = TrackingType ≠ 0; every SFXUpdate `psiUpdateSound` → `PS2_SFXCalculate3D` +
`PS2_SFXSetup3D`, which calls `psiSetStreamFrequency/VolumeLeft/Right/Rear` with the same Pan1/Pan2/Pan3 numbers
(sound.md §5.3–5.4: quadratic distance falloff with the item's Inner/Outer = bank 5/30 since the EE sends −1, front
L/R + phase-inverted rear voice). Pitch = 0x759 (+ PitchOffset/random, 0 here) written every frame (skipped while the
stream is paused). Reverb on (ReverbSend 50). SFXSetup's TrackingType-2 distance/priority rejection and the per-id
MaxVoices = 1 (oldest same id killed) apply as for samples. The EE side keeps the position fixed at the drone (§1.2).

### 4.5 Precache (streams only) [M SFX.IRX SFXPreCache@39f4, SFXStart3D@3aac]
`SFXPreCache(id, n)`: `SFXKeyOnFrameDelay = n + 5; SFXStart(id, handle 0)` → item at **position (0,0,0), Handle 0**,
set up but not keyed on; it starts loading the stream. Its `KeyOnFrameDelay` counts down once per SFXUpdate; at 0 the
item is deleted (stream stopped). A later `SFXStart3D` of the same id with `SFXKeyOnFrameDelay == 0` finds it, clears
the delay and keys it on — **without updating Handle or position** — and returns without making a new item.
Non-stream ids: a precache returns at once (nothing happens).
Consequences [S from M]: for a TrackingType-2 line the precache item is usually rejected by SFXSetup's distance test
(it sits at the world origin), so the later start is a normal fresh start; if it survives, the taken-over line plays at
the origin and the EE node (different handle) cannot move or remove it.

---------------------------------------------------------------------------------------------------

## 5. Facial-animation sound path [M]
- `AnimFacialAppend(obj, hash)` @1c6490: `s = AnimScriptAppend(obj, hash)`; `s+0xb5 = 3` (stop at end),
  `s+0x80 = 0xe0000002`; `obj.animObject(+0xdc)+0x109 = 2` (facial state "speaking", read by AnimFacialUpdate, which
  otherwise runs idle/blink facial scripts).
- The script is ticked with the object's animation (AnimObjectUpdate → AnimScriptTick → AnimProcessScriptCmds):
  time starts at 1 and advances `speed · FRAME_RATE_MUL` per tick; a command fires when its frame lies in the passed
  interval (the start frame's commands run on the first tick) (anim-engine.md).
- **op 1** `{u16 op|len<<8, u16 frame, u16 sfx}`: `Sound_Play3D(sfx, pos, vol 100, outer 100, inner 0,
  pitch Rand(500)−250, ref 0, custom 0)`; pos = obj+0x30 when obj+0x1c (parent) is 0 — the case for drones; the
  handle is discarded (cannot be stopped, does not move, not in +0x4d0). The player-weapon id remaps do not apply
  (obj+0x1c = 0). The 0x33-frame op-1 precache only runs for scripts with flag 0x10000 (first-person weapons), not here.
- Timeline when the talk entry has a facial script with a sound (frame T = SpeechSFX call):
  T: SFXPreCache(sfx, 20) batched (IOP delay 25 updates) — d+0x37 = 1, d+0x48 = T+20.
  T+21: ControlSTANDARD sees `d+0x48 < now` → AnimFacialAppend.
  T+21 + (opFrame − 1) ticks: op 1 → Sound_Play3D (node pending) → next frame's Sound_UpdateSounds → SFXStart3D
  (takes over the precached stream only if it still exists, i.e. roughly opFrame ≤ 3 [S]).
  So the line is heard ≈ 22 + (opFrame − 1) frames after the talk call (plus stream latency if not precached) [S].
- Facial scripts without op 1 are appended immediately (no precache) and the entry's sfxId is not played at all.
- On 07000005 none of the 61 facial scripts is loaded, and the guards' talk calls pass useFacial = 0, so this path is
  not taken; the first SpeechSFX with useFacial on such an entry would zero its facialAnimId and play sfxId directly [L/M].

---------------------------------------------------------------------------------------------------

## 6. Port checklist [S]
1. Drone_SfxInfo table (lastFrame global per entry), the 30-frame post-load gate, per-drone +0x4d0 / +0x4d4.
2. Talk functions with the exact RNG call order (incl. PatrolTalk's per-frame Rand(2) for non-type-3 drones and
   AttackTalk every frame in SniperFire / Abseil states).
3. Pain/Death force-overlap; second DeathTalk skipped by id comparison.
4. Streams: FileRef → LUT[−1−ref]; decode mono ADPCM from dataOffset for `endPos & ~0xff` bytes at 22050 Hz (×pitch);
   treat as a 3-D voice with the bank's TrackingType/radii; max 2 concurrent SFX streams (third fails silently); a
   short start latency is faithful but optional; ends → node finishes → talk handle clears.
5. Facial path only matters on levels with facial scripts (e.g. 07000007 civilians).

## 7. Open points
- Exact start latency of a non-precached stream (CD timing) [S].
- Header word 0 (=1) and the 52-byte record's meaning for SFX streams (music side) [S].
- `*(drone+0x108) & 1` meaning (alternate voice on 0x14..0x16) [S].
