# Searchlight part (the 0x40-byte instance at obj+0x110; class 0x22)

Proof levels: see [README.md](README.md). Instance size from `Searchlight_Create` 0x1cb4f8:
`control_create_object(0x40, …)` [M 001cb4f8:25]. One creator, placement type 0xDE, makes every
part; **the kind field inst+0x00 selects which part** (param 0 of the placement). Level:
ee_start_05 (07000005 at start) unless noted.

Every instance reader/writer in the decompile is in exactly two functions: `Searchlight_Create`
0x1cb4f8 and `Searchlight_Update` 0x1cb900 (grep `Searchlight` over work/decomp/ACTION.ELF → only
the creator, the update, the dispatcher `parsemap_create_dynamic_objects` 0x1d04f0 case 0xDE, and
index.tsv; `param_offsets.py obj_tag` adds no other taker). Renderer/draw code consumes only the
obj header. So every instance row below is [M*] (all hits listed, they agree) unless marked.

## How one creator makes five part kinds

`Searchlight_Create(pos, rot, level_tag* lv, celglist* g)` [M 001cb4f8]:

1. `control_create_object(0x40, pos, rot, 0)` → obj; instance = obj+0xE0 → obj+0x110 (:25,:30).
2. Writes kind (lv+0x2C), draws `Rand_Rand(0x1e0)` for the phase, copies the placement rot VECTOR
   into inst+0x10..0x1F, the three channel bytes (:31-49), computes half/mid from params 4/5
   (:50-65), class byte 0x22 (:66), obj+0xF4 = 3 if the on-channel is nonzero else 0 (:67-72),
   obj+0xD8 = g, obj+0xF0 = g+0x44, `build_LinkToRoom` (:73-76).
3. **The linking walk** (:77-124): walks the object list of the cel the new part just linked into
   (`obj+0x20` → `cel+0x8` head, next = obj+0x8); skips self (:79) and class ≠ 0x22 (:82, `'"'`);
   links only when `Vec_Dist3D < 3.0` strict (:83-84). Pairing by kind slot only (:87,:100):
   `my.links[other.kind] = other` when I am a head, else `other.links[my.kind] = me` when the
   other is a head. A dark head's non-mount parts get `View_SetDrawInNoViews` (:95,:106).
4. **Kind-2 (spot) epilogue** (:125-168) runs after the walk: takes a 0x50-byte psiLight node from
   the pool at 0x2c74c0 (mallocs 0xa00 = 32×0x50 when empty, :129-136), inits it (table below),
   stores the node at **inst+0x28**, and moves the spot obj −10000 on all three axes (:165-167).

Ordering consequence [M + L]: a spot created **after** its head links while still in place
(head.links[2] = spot); a spot created **before** its head is already at −10000 when the head's
walk runs → permanent orphan. All 7 mounts and both spots of 07000005 have links[1] = 0 [L] —
backlinks toward the head are never written (only heads get slots filled by their own walk).

## The five kinds ([L] ee_start_05: 29 class-0x22 objects, kind histogram {0:7, 1:7, 2:2, 3:6, 4:7})

| kind | part | update behaviour |
|---|---|---|
| 0 | the mount (searchlight.md's "disc" naming for kind 0 is a slip — see Erratum) | only the shared block's azimuth copy |
| 1 | the head — the only kind the update advances (`if (*psVar2 != 1) return;` [M 001cb900:89]) | state machine 0 sweep / 1 track / 2 broken / 3 off |
| 2 | the spot — the projected ground disc + psiLight; moved onto each ray hit | positioned by the head's ray |
| 3 | the beam volume | matrix = Mat_Align2Dir(aim, up = Bond − head) |
| 4 | the bulb / corona billboard | pos = head + 0.4·aim; alpha/scale by angle to the eye |

**Erratum (atlas/searchlight.md kinds table):** its kind-0 row ("the projected light disc — moved
onto each ray hit") describes kind 2; the update moves links[2] (the kind-2 spot) onto the hit and
only copies the azimuth to links[0] [M 001cb900:187-195 vs 216-246]. The same doc's own snapshot
line ("7 mounts") and the port's kind names are right.

## Layout

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x00 | u16 | kind: 0 mount / 1 head / 2 spot / 3 beam / 4 corona. Written once at create | [M* 001cb4f8:31, 001cb900:89] [L-read: obj 016e95a0 → 016e96b0 = 1; histogram above] |
| +0x02 | u16 | pad (memset 0, never touched) | [M* create+update are the only accessors] |
| +0x04 | f32 | head: sweep phase, init `(float)Rand_Rand(0x1e0)` ∈ [0,480), advanced `+= FRAME_RATE_MUL` with the `fmodf(…,480)` result discarded [M 001cb900:363-365]. spot: reused as the "ticks since Bond last lit" timer (reset 0 / `+= 1.0` / `> FRAME_RATE·15` → back to sweep) [M 001cb900:254,258,261,263] | [M*] [L-read: head 016e95a0 inst 016e96b4 = 95.0; both spot nodes show 470.0/449.0 — their never-advanced initial draws] |
| +0x08..0x0F | — | pad (never touched) | [M*] |
| +0x10 | f32 | placement rot.x (unused by the update) | [M* 001cb4f8:43] |
| +0x14 | f32 | placement rot.y = **the azimuth centre** of the sweep/track (clamped ±inst+0x38) | [M* 001cb4f8:44, 001cb900:154-165,362] [L-read: 016e96c4 = 1.5708 (π/2)] |
| +0x18 | f32 | placement rot.z (unused) | [M* 001cb4f8:45] |
| +0x1C | f32 | placement rot.w (unused) | [M* 001cb4f8:46] |
| +0x20 | ptr | links[0] — head: the kind-0 mount. Gets `+0xFA \|= 0x20` and the head's azimuth (obj+0x54) every shared-block tick; never hidden/deleted by kill or break | [M* 001cb4f8:87, 001cb900:181-193] [L-read: head 016e95a0 links[0] = 016e9440] |
| +0x24 | ptr | links[1] — non-head part: its head obj (only when this part was created *after* the head). Measured: always 0 on 07000005 (all parts precede their heads) | [M* 001cb4f8:100] [L-read: 016e96b8 = 0] |
| +0x28 | ptr | **two meanings.** head: links[2] = the kind-2 spot obj (the ray target; 0 = unlinked head, can never arm). spot: the 0x50-byte psiLight node (teardown on kill/break: `LList_Remove(0x2c74cc)`, `psiLight_Delete(node+0x34)`, back to pool 0x2c74c0) | [M* 001cb4f8:164, 001cb900:95-116,419-440] [L-read: head 016e95a0 → 016e9020 (spot obj); spot 016e8ec0 → 01fca2e0 (node)] |
| +0x2C | ptr | links[3] — head: the kind-3 beam obj | [M* 001cb4f8:87, 001cb900:188,196] [L-read: 016e96d4 = 016e92e0] |
| +0x30 | ptr | links[4] — head: the kind-4 corona obj | [M* 001cb4f8:87, 001cb900:282] [L-read: 016e96d8 = 016e9180] |
| +0x34 | u8 | on/off switch channel (param 1). Create: nonzero ⇒ obj+0xF4 = 3 (starts off). State 3 re-lights when `switch_channels[+0x34]` raised. Cleared (with +0x35/+0x36) on the kill path | [M* 001cb4f8:47,67-72, 001cb900:128,134,399] [L-read: 016e96e4 = 0xa4 = 164 on the dark head 016e95a0] |
| +0x35 | u8 | kill channel (param 2): while nonzero and `switch_channels[+0x35]` raised → state 3, links[2..4] deleted, +0x34/35/36 zeroed — one-shot, never re-lights | [M* 001cb4f8:48, 001cb900:92-137] |
| +0x36 | u8 | alarm channel (param 3): state 1 writes `switch_channels[+0x36] = 1` every tick (gated `!= 0`). Cleared on kill | [M* 001cb4f8:49, 001cb900:137,142-144] |
| +0x37 | u8 | pad | [M*] |
| +0x38 | f32 | half-range = (max(p4,p5) − min) · DEG2RAD · 0.5 rad. The ±half of both clamps | [M* 001cb4f8:65, 001cb900:155,163,171,179] [L-read: 016e96e8 = 0.0873 (5°)] |
| +0x3C | f32 | mid = (max + min) · DEG2RAD · 0.5 — **the polar (depression) centre** | [M* 001cb4f8:64, 001cb900:170-180,359] [L-read: 016e96ec = 6.1087 (350.06°)] |

### The psiLight node (0x50 bytes, pool 0x2c74c0/0x2c74cc; reachable from a spot's inst+0x28)

Writer-side facts from Create [M 001cb4f8:139-163]; Update only tears it down
[001cb900:105-108,430-433]. Layout beyond these offsets not mapped.

| off | value at create |
|---|---|
| +0x10..0x1C | 4×f32 = the spot's position **before** the −10000 move [L: orphan spot 016e8ec0's node 01fca2e0 holds (61.42, 33.14, 106.61) — 1.5 from head 016dfe70] |
| +0x20/+0x22/+0x24/+0x26 | u16 = 0 |
| +0x28 | u16 = `Rand_Rand(0)` (flicker seed) [L: 0 on both live nodes] |
| +0x2A | u16 = 0 |
| +0x2C | u16 = 0xFFFF |
| +0x30 | ptr = the spot obj |
| +0x34 | u32 = `psiLight_Create(node)` handle [L: 0 on both live nodes] |
| +0x3C | f32 = 500.0 (light radius) |
| +0x40 | f32 = 1.0 |
| +0x44/+0x45/+0x46 | u8 = 0xFF (colour) |
| +0x47 | u8 = 2 |
| +0x48 | u8 = 1 |

## Param map (from the creator; params at level_tag+0x2C+4k)

| level_tag param | read as | Instance offset |
|---|---|---|
| 0 (+0x2C) | u16 | inst+0x00 (kind) |
| 1 (+0x30) | u8 | inst+0x34 (on channel); also obj+0xF4 = 3 when nonzero |
| 2 (+0x34) | u8 | inst+0x35 (kill channel) |
| 3 (+0x38) | u8 | inst+0x36 (alarm channel) |
| 4 (+0x3C) | u32 (signed degrees) | inst+0x38/0x3C via min/max: half = (max−min)·π/180/2, mid = (max+min)·π/180/2 |
| 5 (+0x40) | u32 (signed degrees) | same two stores |

Create also consumes no RNG beyond the phase (`Rand_Rand(0x1e0)`) and, for spots, the node's
flicker seed (`Rand_Rand(0)`) — one or two draws per part, in creation order (port-critical).

## The obj header fields this family leans on

| header off | use | proof |
|---|---|---|
| +0x30 | spot pos = ray hit + 0.01·normal; head pos feeds the ray origin/beam trans; kill/break sounds at it | [M 001cb900:220-223,237-245,203-205] |
| +0x50 / +0x54 / +0x58 | the head's polar / azimuth / roll angles (spot gets π/2−atan2(n.y,√(nx²+nz²)) / atan2(nx,nz) / 0). **The drawn matrix follows these** [L: head 016dfe70's matrix row 2 = (0.5824,−0.0471,0.8116) = the aim of yaw 0.6224/polar 0.0471] | [M 001cb900:153,169,226,232,235,236] |
| +0xF4 | state u16: 0 sweep / 1 track / 2 broken / 3 off | [M 001cb900:94,140,253,264,418] |
| +0xFA | flags2: mount/beam/spot get `\|= 0x20`; beam `\|= 0x24`; corona parts `\|= 0x20`; the tail sets the head's 0x20 | [M 001cb900:192,225,271,333,353,391] |
| +0xFC | the head's draw counter gates the corona block; state-2 smoke gate | [M 001cb900:287,374] |
| +0x106 | the corona part's alpha byte: `(dot−0.2)·4·255` low byte while visible | [M 001cb900:334,336] [L: 016e0290 = 0x2e, 016e1310 = 0x7d, undrawn parts keep 0xFF] |
| +0xE8 | the corona part's scale: `(dot−0.2)·4·4`, ×1.5 when the LOS passes | [M 001cb900:335,346] [L: 1.088 / 2.947 / 0.744 on the three lit coronas, 1.0 on the dark ones] |
| +0xF0 | corona part's 0x2000 = LOS-clear bit (± set/clear per tick); +0x229 base from the glist | [M 001cb900:342,345] [L: 0x0022229 on lit parts, 0x000229 dark] |
| +0xEC + GameState+52 | state-2 smoke phase: `(GameState+52 + obj+0xEC) % 7 == 0` → `Gas_Init` | [M 001cb900:375-384] |
| +0xD0 | HITDATA contact list — the break walk (entries: next +4, other obj +0x58, damage f32 +0x8) | [M 001cb900:406-461] |
| +0xD8/+0x20 | glist / cel: ray and LOS cel args; `Control_NextLOD` swaps +0xD8 ← glist+0x40 (the damaged model) on kill/break | [M 001cb900:207,338,138,454] |

## State machine [M 001cb900] (head only)

- **kill gate (any state)** :92-138 — inst+0x35 raised: state 3; light node freed; links[2..4]
  objects marked `+0xFE \|= 1` (deleted by the movement pass) and their link slots zeroed;
  inst+0x34/35/36 = 0; `Control_NextLOD(head)`. links[0] (mount) untouched.
- **0 sweep** :357-367 — polar = sinf(phase·0x3C567751)·half + mid; azimuth = sinf(phase·SWEEP +
  π/2)·half + inst+0x14; phase += FRAME_RATE_MUL (fmodf discarded). **The (−π,π] normalisation of
  both stores is not visible in the decompile** (Ghidra dropped block 0x1cbc74, WARNING line 3)
  but is [L]-proven: head 016e03f0 stores −1.838 where the raw sum is 4.445 (mid 4.9742 − 2π);
  head 016e0970 likewise −1.4748. The port implements it.
- **1 track** :141-354 — raise alarm channel; azimuth += 0.15·AngleDiff(obj+0x54, atan2f(dx,dz)),
  clamp inst+0x14±half; polar += 0.15·AngleDiff(obj+0x50, −atan2f(dy−1,√(dx²+dz²))), clamp
  mid±half; then the shared block.
- **shared block** (states 0, 1, and any state ≥ 4 — unreachable) :186-353 — mount copy; then the
  spot only: ray `Collide_RayIntersect(head+0x30, head+100·aim, obj cel, self, flags 0x18)` (the
  pick mask does not exclude the player — the spot lands on Bond); hit → spot pos/angles +
  DrawInAllViews, miss → NoViews; arm/lost test `Vec_Dist3D(spot, player) ≤ 2.0` (0→1 with spot
  inst+4 = 0; state 1 counts inst+4, ≥ FRAME_RATE·15 → back to 0); then beam matrix; then corona.
- **2 broken** :373-393 — every 7th drawn frame `Gas_Init` smoke (dir (0,0,Rand_FRand(π)), 0.05,
  0.25, id 0x2000039); polar += 0.05 while < 0.8 (store confirmed obj+0x50 :392 — closes the
  atlas's "not re-pinned" note).
- **3 off** :395-404 — on-channel raised → state 0 + `Sound_Play3D(0xE5, …)` and **jump straight
  to the contact walk** (no sweep, no part updates this tick).
- **contact walk (tail)** :406-462 — every HITDATA entry whose other obj is class 5 with damage
  (entry+0x8) > 0.0 → break: state 2, spot/beam/corona objects deleted, light node freed,
  `Control_NextLOD`, `Sound_Play3D(0xE6, …)`.

## Port mismatches (crates/nf-game/src/searchlight.rs — report only)

1. **Kill path deletes the wrong parts** — game: links[2..4] objects marked for deletion
   (`+0xFE |= 1`, gone from the list next tick), mount untouched. Port hides every link
   **including the mount** and keeps the objects:
   `for p in links.into_iter().flatten() { self.parts[p].drawn = false; … }` (searchlight.rs:218-221).
   Visual: the port's mount vanishes on the channel kill; the game's stays.
2. **Break path same deletion difference** — game deletes links[2..4] and calls `Control_NextLOD`
   (damaged model) [M 001cb900:438-454]; port hides all links, no model swap (searchlight.rs:423-429).
3. **Break gate** — game breaks only when the contact entry's damage float > 0.0
   (`if (fVar16 < *(float *)(aiStack_d0[0] + 8))` 001cb900:417); the port's
   `for &(from, to) in bullets { … seg_box … }` (searchlight.rs:418-419) has no damage value — any
   segment hit breaks.
4. **Relight tick** — game: state 3 + on-channel → state 0, sound, `goto` the contact walk (no
   sweep, no part updates, no visibility restore that tick) [M 001cb900:399-403]; port sets
   `parts_now = true` and force-redraws all links (searchlight.rs:264-271) — one tick early.
5. **Corona LOS origin** — game: `Collide_LineOfSight(glb_viewer+0xc0, corona+0x30, …, 8)` — the
   **camera** position [M 001cb900:338-340]; port: `d.world.line_of_sight(player.eye, corona_pos)`
   (searchlight.rs:390). Visual difference when the camera and Bond's eye diverge.
6. **Alarm write not gated** — game `if (inst+0x36 != 0) switch_channels[inst+0x36] = 1;`
   [M 001cb900:142-144]; port `channels.set(self.parts[head].alarm as u32, 1, frame)`
   (searchlight.rs:238) unconditional. No behavioural difference today: `Channels::set` ignores
   channel 0 (triggers.rs:82) — but the game never writes switch_channels[0] here.
7. **Kill-tick tail** — game still runs the contact walk after the kill block; port
   `return sounds;` before `bullet_check` (searchlight.rs:222). One-tick, inert.
8. **(−π,π] normalisation is decompile-invisible** — the port's `sweep()` normalises both angles
   (searchlight.rs:96-99); the C shows no such code (removed block 0x1cbc74) but live heads prove
   the game does it [L: −1.838/−1.4748]. Correct, but unverifiable against the C — keep the [L]
   pins in the tests.
9. **Angle→matrix path** — the game writes angles (+0x50/+0x54/+0x58) and the drawn matrix follows
   via the engine's rot→matrix path; the port writes matrices directly (align2dir for head/mount/
   beam, normal-frame for the spot). [L]-equivalent (live matrices match), not [M].
10. **LOST_LIMIT hardcodes 60·15** (searchlight.rs:24); the game reads the `FRAME_RATE` global
    (0x31d0d0). Equal on NTSC-U.

No disagreement on: kind/links/channel bytes/half/mid layout, param map, DEG2RAD+0.5·(hi±lo)
arithmetic, the 3.0-strict same-cel create-time-only linking, the 100-unit ray with the player's
sphere pickable, the 2.0 arm radius, spot +0.01 normal offset, corona 0.4 offset / 0.2 gate /
(·4, ·255, ×1.5) shading, phase init 0..479.

## [L-read] spot checks (work/ee_start_05.bin, file offset = EE address)

Walk: head 0x2705a0 → first obj u32@0x2705b4 = 0x01319e30 → next = obj+0x14 (523 objects, count
u32@0x30ca98 = 523); filter class byte obj+0xFF == 0x22 → 29; instance = u32 at obj+0xE0.

1. **kind (inst+0x00)** — head 0x016e95a0, inst 0x016e96b0: `00000001` = kind 1. Histogram over
   all 29: {0:7, 1:7, 2:2, 3:6, 4:7} — matches the atlas's mount/head/spot/beam/corona counts.
2. **half/mid (inst+0x38/0x3C)** — head 0x016e95a0: `3db22b0d` 0.0873 (= (355−345)·DEG2RAD/2) and
   `40c37a15` 6.1087 (= 350·DEG2RAD) ⇒ placement params 4/5 = (345, 355). Head 0x016e03f0:
   0.6109/4.9742 ⇒ (250, 320).
3. **links (inst+0x20..0x33)** — head 0x016e95a0: links[0] = 016e9440 (mount), links[2] =
   016e9020 (spot — the only linked head), links[3] = 016e92e0 (beam), links[4] = 016e9180
   (corona). Head 0x016dfe70: links[2] = 0 — the measured orphan head.
4. **on-channel (inst+0x34)** — 0xa4 (164) on head 016e95a0 and 0x61 (97) on head 016dbb60 (the
   two state-3 heads); 0 elsewhere. Their live obj+0xF4 = 3.
5. **state + angles (obj+0xF4/+0x50/+0x54)** — head 016dfe70: state 0, polar +0.0471, azimuth
   +0.6224 — reproduces `sweep(317, 0.3054, 0.3054, 0.7854)` bit-for-bit. Head 016e03f0 (mid
   4.9742): polar −1.8380 = raw 4.445 − 2π (the normalisation the C hides).
6. **spot + light node** — spot 016e9020 pos = (−10057.84, −9963.38, −10059.73) (the −10000 move);
   its node 01fca330: +0x3C = 500.0, +0x40 = 1.0, +0x44..47 = ff ff ff 02, +0x48 = 1, +0x2C =
   0xFFFF, +0x30 = 016e9020, +0x34 = 0. The orphan 016e8ec0's node keeps its original position
   (61.42, 33.14, 106.61).
7. **corona part alpha/scale/flags** — lit parts 016e0290 (α 0x2e, scale 1.088, +0xF0 0x2229,
   +0xFC 2), 016e1310 (0x7d, 2.947), 016e1890 (0x1f, 0.744); dark parts keep α 0xFF, scale 1.0,
   +0xF0 0x229, +0xFC 0. Beams of active heads carry +0xFA = 0x0004 (the beam block's 0x24 with
   0x20 consumed); the dark head's beam 016e92e0 sits at 0x0000/0xFC 0.

## Live checks (for the main agent)

Pointer chain: `DynamicObjList 0x2705a0 → u32@head+0x14 → obj → +0x14 …`; a class-0x22 obj's
instance = `u32@obj+0xE0`. On 07000005 at start the dark head with the linked spot is obj
0x016e95a0 (state 3, on-channel 164, spot 0x016e9020 linked).

1. **Relight via the channel** — `python -I tools/trace/fieldprobe.py poke 0x26fc90+164 1 1 --shot out.png`
   (byte at switch_channels+0xa4 = 0x26fd34). Predicted: the relay clunk (0xE5) and head 016e95a0
   enters state 0 (obj+0xF4 = 0); next ticks the ray repositions spot 016e9020 from −10000 onto
   the wall it aims at (azimuth centre π/2 → +x, mid 6.1087 ≈ 10° above horizontal — the gate wall
   above the road). Watch: `watch 0x016e95a4` (obj+0xF4).
2. **Sweep half-range** — `poke <inst of 016dfe70>+0x38 4 <f32 0.02>`: the sweeping head's arc
   collapses to ±0.02 rad around its centre. The phase (+4) is rewritten every frame — `watch` it
   instead of poking.
3. **Break via state** — `poke 0x016e95a0+0xf4 2 2` (state 2): smoke puffs every 7th frame at the
   head and the polar angle droops toward 0.8 (+0x50 climbs 0.05/tick, visible with `watch
   0x016e95f0`). (The real path is a damaging bullet registering in obj+0xD0.)
4. **Orphan adoption** — `poke 0x016e96c8 4 0x016e8ec0` (write the orphan spot into head
   016dfe70's links[2] at inst+0x28): within a tick the orphan's position (+0x30) should jump from
   −10000s onto the first ray hit — direct proof of the links[2]-is-the-ray-target reading.
