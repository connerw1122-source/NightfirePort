# Atlas port-check: `crates/nf-game/src/drone.rs` offsets vs the game

Plan v2 §2: "Any `+0xNNN` in a comment must exist in the atlas." This file is the audit of every
`+0xNNN` the port's drone.rs mentions (164 mentions, 141 unique offsets), each checked against the
decomp (ACTION.ELF `e7ffdcb3…`). Generated for atlas-1; regenerate whenever drone.rs gains an offset.

Verdicts: **OK** (evidence supports offset+meaning), **OK-with-note** (right field, wording can
mislead), **SUPPORTED** (grep-level evidence in the named function), **MISMATCH** (port reads the
wrong word or wrong meaning), **UNVERIFIED** (not re-measured this pass — the docs' original
evidence stands until re-checked). Bases: `obj` = the 0x110-byte object header (`docs/atlas/object.md`);
`Drone` = the 0xD20 instance at obj+0xE0 (`docs/atlas/drone.md`); other bases named inline.

## Mismatches — fix these in the port (or the atlas, if the port is right)

### 1. Grenades are param 11, the port reads param 24 (drone.rs:564-566)

The port comment says: "`NDrone2_DefaultInit` (0x14b300 line 349): `+0xBCC` and `+0xBCE` from
param 24's raw value. `uVar4 = piVar19[0x18]`". The hex index was read as decimal: `0x18` = 24,
but `piVar19` is the **DIVars** pointer = `Drone+0xC00`, and **params start at `piVar19[13]`**
(`Drone_Create` 0x136be0 copies `placement+0x2C` (33 words) to `Drone+0xC34` = `piVar19[13]`
[disasm]). So `piVar19[0x18]` = **params[11]** = `placement+0x2C+44` = `Drone+0xC60`.

The port's `param(24)` therefore reads `placement+0x2C+96` — the alert behaviour block's middle
word (for 07000005 guards: `0xECF377E5`/`0xAC700787`, [L] `Drone+0xC94`) — instead of the grenade
word. The decoded switch itself is right in the port (1→0x36 stun, 2→0x35 frag, 4→0x34,
5→`(raw&0xF)+0x3A` count 1, 6→0x6C count 1, default 0/0 — all match `NDrone2_DefaultInit`
0x14b300), and in 07000005 both words decode to "no grenades" (params[11] = 0 everywhere [L]),
so the bug is silent on the test level. Any level whose guards carry grenades will misread.

Fix: `let p11 = param(11);` in `drone.rs` (`p24` → `p11`), same decode.

### 2. `Drone+0xCD8` is the disabled-drone resume state, not "death channel pending" (drone.rs:375, 472)

`NDrone2_DSTATE_Disabled` 0x172440: on its update message it calls
`Drone_SM_SetState(..., *(u32*)(Drone+0xCD8), 0)` — `+0xCD8` holds the **state id to resume into
when the channel releases the drone**. The port's "CombatNoRoute alternation" (drone.rs:375) and
"death channel to raise (+0xCD8 pending)" (drone.rs:472) do not match that read. The alternation
and death-channel stories may involve other fields; re-check both before porting the Disabled
state's neighbours.

### 3. `+0xD8` is the voice set; the damage docs' "type +0xD8" is a different field

`NDrone2_DefaultInit` writes the **voice set** at `Drone+0xD8` (u16, from the model-id switch;
[L] 07000005 guards = 2) and the voice variant at `+0xDA`. The damage-side "type" is `+0xC5`
(dtypenew) / `+0xC7` (DTYPE). drone.rs:503 correctly says "the voice set (+0xD8)"; drone.rs:442's
damage comment "type `+0xD8`" should read the dtypenew/DTYPE pair. Also keep `obj+0xD8` (glist
pointer, object header) distinct from both.

### 4. `obj+0xEC` (wobble) is rewritten after creation (drone.rs:354)

`Drone_Create` 0x136be0 sets `obj+0xEC = Rand_Rand(10000)` [disasm], but the [L] live value is
`0x0001A677` (> 10000) on every 07000005 drone — some later writer reuses the word (unidentified;
`control_init_object` inits it to `GameState[52]`, a frame count, which 0x1A677 resembles). The
port keeps its own copy of the Rand draw, so behaviour matches at creation time, but the port
comment should not claim the live field holds the wobble phase. Open item: find the writer that
overwrites `obj+0xEC`.

## Systematic findings

- **Hex/decimal param indices**: the `piVar19[k]` indexing starts params at k = 13. Any port
  comment quoting `piVar19[0xNN]` as "param 0xNN/NN" is off by 13. Verified params (port numbering
  = placement words, 0-based): 0 model id, 1 script, 3 channel, 4 group, 5 mode, 6 second switch,
  7 →+0x148, 8 second tag, 9 weapon, 10 →+0x45, 11 grenades, 12 death channel, 13 special-death
  script, 14 sight class, 15.. behaviour blocks.
- **obj vs Drone base**: the port's comments are mostly careful ("obj+0xFC", "Drone+0x4F8"); the
  two traps found are `Drone+0x100` (damage multiplier, f32) vs `obj+0x100` (ambient byte) — both
  appear in drone.rs and are different fields — and `+0x50/+0xEC` which exist in both structs with
  different meanings (`obj+0x50` rotation angles; `Drone+0x50` f32 0.075 default; `obj+0xEC`
  wobble/frame; `Drone+0xEC` f32 4.0).
- The port's sight-constant table (param 14 classes 1-5 + default), the captain difficulty
  properties (9/5/8), and the 07000005 param-9==0x16 property patch all match `NDrone2_DefaultInit`
  exactly.

## Offset-by-offset audit

| Offset | Base | Port claim | Verdict | Evidence / note |
|---|---|---|---|---|
| 0x2 | obj | (ForcedList link area) | OK | header links — see atlas object.md |
| 0x8 | obj | cel-list next | OK | atlas object.md |
| 0xc | Drone | obj back-reference | OK | NDrone2_CreateObj 0x14d7c8 (disasm sw $s0, 0xc($v0)); [L] equals obj on all 13 |
| 0xe | ? | (port claim not mapped this pass) | UNVERIFIED | grep evidence available in port_check_grep.tsv |
| 0x10 | Drone | BodyGlow object ptr | OK | NDrone2_DefaultInit 0x14b300 tail: BodyGlow_Create(obj) -> +0x10 |
| 0x14 | Drone | damageable byte | OK | DefaultInit: 1 default, 0 for models 0x3B/3C/53 |
| 0x15 | Drone | female voice | OK | DefaultInit sets 1 for female voice sets |
| 0x17 | Drone | another drone within 3 units | UNVERIFIED | small-offset bunched flag; not located this pass |
| 0x18 | Drone | (init zero group) | OK | DefaultInit zeroes +0x18; set 1 for voice-set-2 model override |
| 0x19 | Drone | captain | OK | DefaultInit: props 9/5/8 by difficulty 1/2/3 -> +0x19 = 1 |
| 0x1a | Drone | special-death free root motion | UNVERIFIED | flag exists around special-death fns (grep 0x1a in SpecialDeath 0x163c68); semantics not re-measured |
| 0x1c | Drone | big hit | UNVERIFIED | hit-path flag (grep 0x1c in NDrone2_HitDamage 0x145558 area); not re-measured |
| 0x20 | Drone | DroneAnim_EventFunc flags +0x20 (event 0xF/0x12) | UNVERIFIED | grep hits incl. DroneAnim_EventFunc 0x13b750; not re-measured |
| 0x21 | Drone | DroneAnim_EventFunc flags +0x21 | UNVERIFIED | same |
| 0x24 | ? | (port claim not mapped this pass) | UNVERIFIED | grep evidence available in port_check_grep.tsv |
| 0x28 | ? | (port claim not mapped this pass) | UNVERIFIED | grep evidence available in port_check_grep.tsv |
| 0x2a | Drone | Bond crosshair on drone (until frame) | SUPPORTED | grep: DroneWeap_OpponentIsAimingAtMe 0x17a500 uses +0x2a |
| 0x2e | Drone | (small flag) | UNVERIFIED | grep hits in UnderCover states |
| 0x30 | ? | (port claim not mapped this pass) | UNVERIFIED | grep evidence available in port_check_grep.tsv |
| 0x33 | Drone | investigating | UNVERIFIED | grep hits incl. DSTATE_HeardNoiseAlert 0x16b540 |
| 0x34 | Drone | turned | UNVERIFIED | grep hits in hearing fns |
| 0x3a | Drone | head shot | UNVERIFIED | grep: DSTATE_Dead 0x163fe8, HitDamage 0x145558 |
| 0x3b | Drone | firing flag | SUPPORTED | grep: many firing-state readers (DroneWeap_Fire 0x17a8c0, StepAimRight 0x1676a0) |
| 0x3c | Drone | burst done | UNVERIFIED | grep hits in firing states |
| 0x41 | Drone | sight lost since last shot | UNVERIFIED | grep hits few (0x176a28 sight, 0x178f68, 0x179740) |
| 0x43 | Drone | noise level (never reset) | UNVERIFIED | grep: DSTATE_HeardNoiseAlert 0x16b540 area |
| 0x44 | Drone | team | OK | DefaultInit tail: +0x44 == 1 -> PlrStat_LogEnemySpawned |
| 0x45 | Drone | params[10] byte | OK | DefaultInit: *(char*)(+0x45) = (char)piVar19[0x17] |
| 0x50 | Drone | (f32 at +0x50: 0.075 default rot-related) | OK | DefaultInit: instance+0x50 = 0x3d99999a when obj rot.x == 0 (distinct from obj+0x50) |
| 0x52 | ? | (port claim not mapped this pass) | UNVERIFIED | grep evidence available in port_check_grep.tsv |
| 0x54 | obj | (with Drone+0x694: rotation word) | OK | obj+0x54 = rot.y; Drone+0x694 = its copy (DefaultInit) |
| 0x60 | anim | player anim flags (L1 aim / feet-on-surface) | OK | different struct (player anim datum) — port comment already scopes it |
| 0x62 | AnimObject | (byte at AnimObject+0x62) | OK | DefaultInit reads *(AnimObject+0x62) for the +0x3E decision |
| 0x78 | Drone | (info flag 0x20000000 cleared) | UNVERIFIED | not chased this pass |
| 0x90 | ? | (port claim not mapped this pass) | UNVERIFIED | grep evidence available in port_check_grep.tsv |
| 0xa0 | Drone | hip height above feet | OK | DefaultInit: 1.0; [L] 1.14806/1.15059 per-model overrides |
| 0xac | Drone | health | OK | DefaultInit 10.0; [L] all 13 = 10.0 |
| 0xb0 | Drone | max health | OK | DefaultInit: +0xB0 = +0xAC at init end |
| 0xb4 | Drone | accuracy stat | OK | DefaultInit (captain × 1/Mod_BulletAccuracy); [L] 5 |
| 0xb5 | Drone | aggression stat | OK | [L] 0; stat table per docs |
| 0xb8 | Drone | reaction stat | OK | [L] 0; docs/systems/drone-ai.md reaction math |
| 0xc5 | Drone | dtypenew | OK | DefaultInit/DoModeSettings; [L] 2 = sniper |
| 0xc6 | Drone | alternate mode | OK | DefaultInit 0x18 case |
| 0xc7 | Drone | DTYPE (param 15) | OK | DefaultInit reads +0xC7 == 0xC for the property patch |
| 0xc8 | Drone | normal block mode (param 16) | OK | [L] matches DoModeSettingsNEW outputs |
| 0xc9 | Drone | alert block mode (param 21) | OK | [L] 2/3 values |
| 0xd8 | Drone | voice set (u16) | OK-with-note | DefaultInit writes voice set; damage docs citing "+0xD8 type" should read +0xC5 (dtypenew) — flagged below |
| 0xe4 | Drone | half fov e4 | OK | DefaultInit pi/2; 0 for sight class 5 |
| 0xe8 | Drone | sight range e8 | OK | DefaultInit sight-class switch matches port table exactly |
| 0xec | Drone | (f32 4.0 at init) | OK | DefaultInit: instance+0xEC = 0x40800000 (distinct from obj+0xEC) |
| 0xf0 | Drone | engage range f0 | OK | DefaultInit matches port constants incl. the 07000005 param-9==0x16 -> 10.0 special |
| 0xf4 | obj | state u16 (drone SM mirror) | OK | [L] mirrors +0x10C |
| 0xfc | Drone | combat sight range fc | OK | DefaultInit matches port table |
| 0x100 | Drone | damage multiplier f32 | OK-with-note | DefaultInit: 1.0/2.0/captain Mod; NOTE obj+0x100 is the ambient byte — keep bases straight |
| 0x106 | ? | (port claim not mapped this pass) | UNVERIFIED | grep evidence available in port_check_grep.tsv |
| 0x108 | Drone | state-machine id (creation counter) | OK | [L] 1..13 newest=1 |
| 0x114 | ? | (port claim not mapped this pass) | UNVERIFIED | grep evidence available in port_check_grep.tsv |
| 0x118 | Drone | state after flinch | UNVERIFIED | docs-cited; not re-measured |
| 0x120 | ? | (port claim not mapped this pass) | UNVERIFIED | grep evidence available in port_check_grep.tsv |
| 0x124 | ? | (port claim not mapped this pass) | UNVERIFIED | grep evidence available in port_check_grep.tsv |
| 0x134 | Drone | switch channel (params[3]) | OK | DefaultInit; [L] equals params[3] on all 13 |
| 0x135 | Drone | second switch (params[6]) | OK | DefaultInit |
| 0x13a | Drone | second tag (params[8]) | OK | DefaultInit; [L] 0x64 on second-switch drones |
| 0x13c | Drone | death channel (params[12]) | OK | DefaultInit; [L] 10/0xA4/0x64 values |
| 0x14c | Drone | damage-taken multiplier | OK | DefaultInit 1.0 |
| 0x150 | Drone | last damage | UNVERIFIED | docs-cited; not re-measured |
| 0x1a0 | Drone | distance to opponent | SUPPORTED | grep: NDrone2_GetHeadPosOld 0x141150 etc. in the opponent-info cluster |
| 0x1c4 | Drone | yaw to opponent (ATAN2_APPROX) | UNVERIFIED | docs-cited; not re-measured |
| 0x1d0 | ? | (port claim not mapped this pass) | UNVERIFIED | grep evidence available in port_check_grep.tsv |
| 0x1d8 | Drone | hits taken | UNVERIFIED | docs-cited |
| 0x1dc | ? | (port claim not mapped this pass) | UNVERIFIED | grep evidence available in port_check_grep.tsv |
| 0x1dd | ? | (port claim not mapped this pass) | UNVERIFIED | grep evidence available in port_check_grep.tsv |
| 0x228 | Drone | vision bits (LOS/seen/ever-seen) | UNVERIFIED | docs-cited (DroneVision_* 0x176a28/0x176f18 grep) |
| 0x22c | Drone | AI-volume light factor | UNVERIFIED | docs-cited (Drone_ProcessOpponents 0x1390c0) |
| 0x230 | Drone | visibility factor | UNVERIFIED | docs-cited |
| 0x270 | Drone | seen ticks | UNVERIFIED | docs-cited |
| 0x274 | Drone | unseen ticks | UNVERIFIED | docs-cited |
| 0x31c | ? | (port claim not mapped this pass) | UNVERIFIED | grep evidence available in port_check_grep.tsv |
| 0x330 | Drone | explosive position | UNVERIFIED | docs-cited (msg 0x21) |
| 0x350 | Drone | explosive run distance | UNVERIFIED | docs-cited |
| 0x3c6 | Drone | move quad | UNVERIFIED | docs-cited |
| 0x470 | Drone | last hit | UNVERIFIED | docs-cited |
| 0x480 | Drone | velocity accumulator | UNVERIFIED | DefaultInit zeroes +0x480; docs drone-collision.md |
| 0x4c4 | Drone | anim yaw offset | UNVERIFIED | docs-cited |
| 0x4d0 | Drone | current speech line id | OK-zero-at-init | DefaultInit zeroes; runtime use per docs/systems/speech.md |
| 0x4d8 | Drone | normal behaviour block (params[16-18]) | OK | DefaultInit -> DoModeSettings*(..., params+15, ...); property get/set all read +0x4d8 |
| 0x4dc | ? | (port claim not mapped this pass) | UNVERIFIED | grep evidence available in port_check_grep.tsv |
| 0x4e8 | Drone | alert behaviour block (params[21-23]) | OK | DoModeSettingsNEW |
| 0x4f8 | Drone | flags word | OK | bit map in atlas/drone.md; DefaultInit mask + Enable/Disable verified by disasm-read decomp |
| 0x4fc | Drone | alert bits (0x10 attacking) | OK | DefaultInit sets 0x10 for Attack-start; [L] 0x10 on the two Attack drones |
| 0x504 | ? | (port claim not mapped this pass) | UNVERIFIED | grep evidence available in port_check_grep.tsv |
| 0x510 | Drone | loudness this frame | UNVERIFIED | docs-cited |
| 0x514 | Drone | alertness share | UNVERIFIED | docs-cited |
| 0x524 | Drone | attack began frame | UNVERIFIED | docs-cited |
| 0x538 | ? | (port claim not mapped this pass) | UNVERIFIED | grep evidence available in port_check_grep.tsv |
| 0x540 | Drone | last flinch frame | UNVERIFIED | docs-cited |
| 0x54c | Drone | finished-turn remainder | UNVERIFIED | docs-cited (DroneAnim_OnEndAnim 0x13e2c0) |
| 0x55c | ? | (port claim not mapped this pass) | UNVERIFIED | grep evidence available in port_check_grep.tsv |
| 0x568 | Drone | danim class | UNVERIFIED | docs-cited |
| 0x56a | Drone | danim state | UNVERIFIED | docs-cited |
| 0x574 | ? | (port claim not mapped this pass) | UNVERIFIED | grep evidence available in port_check_grep.tsv |
| 0x578 | Drone | blend-wait frames | UNVERIFIED | docs-cited |
| 0x579 | Drone | event-7 one-shot end flag | UNVERIFIED | docs-cited |
| 0x57a | Drone | one-shot running | UNVERIFIED | docs-cited |
| 0x57b | Drone | state danim started | UNVERIFIED | docs-cited |
| 0x584 | ? | (port claim not mapped this pass) | UNVERIFIED | grep evidence available in port_check_grep.tsv |
| 0x594 | ? | (port claim not mapped this pass) | UNVERIFIED | grep evidence available in port_check_grep.tsv |
| 0x596 | ? | (port claim not mapped this pass) | UNVERIFIED | grep evidence available in port_check_grep.tsv |
| 0x598 | ? | (port claim not mapped this pass) | UNVERIFIED | grep evidence available in port_check_grep.tsv |
| 0x59c | Drone | stand-idle hold | UNVERIFIED | docs-cited |
| 0x5a0 | Drone | hearing return state | UNVERIFIED | docs-cited |
| 0x5a2 | Drone | start/return state | OK | DefaultInit (+DoModeSettings); [L] 0x29/0x56/6/0x85 |
| 0x5a4 | Drone | alternate state | OK | DefaultInit path; [L] 0x85 on channel-paired drones, 0 on sniper |
| 0x5b0 | Drone | hearing home | UNVERIFIED | docs-cited |
| 0x694 | ? | (port claim not mapped this pass) | UNVERIFIED | grep evidence available in port_check_grep.tsv |
| 0x710 | Drone | goal point | UNVERIFIED | docs-cited (cover) |
| 0x830 | Drone | in-cover point | UNVERIFIED | docs-cited |
| 0x860 | Drone | combat route (AIRoute) | OK | DefaultInit: AINetwork_ResetRoute(+0x860, obj, 0); +0x80 flag when +0x23 |
| 0x954 | Drone | nav path id | OK-zero-at-init | DefaultInit zeroes |
| 0x970 | Drone | patrol route (AIRoute) | OK | DefaultInit: AINetwork_ResetRoute(+0x970, obj, 0) |
| 0xaf0 | Drone | last sound position | UNVERIFIED | docs-cited |
| 0xb38 | Drone | last heard frame | UNVERIFIED | docs-cited |
| 0xb3c | Drone | heard peak | UNVERIFIED | docs-cited |
| 0xbae | Drone | last wait node | UNVERIFIED | docs-cited |
| 0xbb4 | Drone | cover node | UNVERIFIED | docs-cited |
| 0xbb8 | Drone | cover side / leave flags | UNVERIFIED | docs-cited |
| 0xbbb | Drone | cover type | UNVERIFIED | docs-cited |
| 0xbbc | Drone | (per-voice-set 0xC/0x40/5/9/8/10000) | OK | DefaultInit |
| 0xbbe | Drone | (per-voice-set) | OK | DefaultInit |
| 0xbc0 | ? | (port claim not mapped this pass) | UNVERIFIED | grep evidence available in port_check_grep.tsv |
| 0xbc4 | ? | (port claim not mapped this pass) | UNVERIFIED | grep evidence available in port_check_grep.tsv |
| 0xbc8 | ? | (port claim not mapped this pass) | UNVERIFIED | grep evidence available in port_check_grep.tsv |
| 0xbcc | Drone | grenade weapon id | MISMATCH-INDEX | semantics OK (switch 1->0x36 stun, 2->0x35 frag, 4->0x34, 5->raw&0xF+0x3A count 1, 6->0x6C count 1; port logic identical) but the raw word is params[11], NOT param 24 — see below |
| 0xbce | Drone | grenade count | MISMATCH-INDEX | same source word as +0xBCC |
| 0xbd4 | Drone | special-death script (params[13]) | OK | DefaultInit; !=0x600021F sets 0x4F8&0x10 |
| 0xbe0 | Drone | special-death position (obj pos copy) | OK | DefaultInit |
| 0xbf4 | Drone | special-death yaw (rot copy word) | OK | DefaultInit (+0xBF0..0xBFC = obj+0x50) |
| 0xc48 | ? | (port claim not mapped this pass) | UNVERIFIED | grep evidence available in port_check_grep.tsv |
| 0xcd0 | Drone | timer D (sniper 6-s return) | UNVERIFIED | docs-cited (msg 0xC/0xD timers); not re-measured |
| 0xcd8 | Drone | CombatNoRoute alternation / death channel pending | MISMATCH-DETAIL | NDrone2_DSTATE_Disabled 0x172440 reads +0xCD8 as a STATE id passed to Drone_SM_SetState on re-enable — it is the disabled-drone resume state, which subsumes neither port wording exactly |
| 0xcfc | ? | (port claim not mapped this pass) | UNVERIFIED | grep evidence available in port_check_grep.tsv |

## Method and honesty

- First-hand [M] evidence this pass: `NDrone2_DefaultInit` 0x14b300 (read end to end),
  `Drone_Create` 0x136be0, `NDrone2_CreateObj` 0x14d7c8 (disasm), `NDrone2_Enable` 0x149dd0,
  `DroneSpawner_SpawnDrone` 0x137cb0, `NDrone2_DSTATE_Disabled` 0x172440, `LList_Add` 0x1cca28,
  `control_create_object` 0x1344e0, `control_movement_object_handler` 0x1336f8,
  `Control_BuildWorldSph` 0x133070, `View_AddObjects` 0x1e62d8, `control_first_object` 0x134300
  (disasm), the `control_funcs` table bytes, and the 07000005 snapshot values quoted in
  `docs/atlas/drone.md`.
- UNVERIFIED rows carry grep evidence only (which drone functions touch the offset — the counts
  and example functions were recorded during this pass); they restate docs/systems/drone-*.md
  claims that were themselves written from this decomp, but were not re-measured here. Treat them
  as "documented, not independently re-checked".
- No Rust code was changed in this task (atlas-1 scope).
