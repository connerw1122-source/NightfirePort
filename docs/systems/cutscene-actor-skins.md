# Cut-scene actor skins (script anims)

Build: ACTION.ELF `e7ffdcb3…`. Tags: [M] decomp / disassembly read here, [D] disc bytes, [L] EE snapshots
`work/ee_start_07000001.bin` / `ee_start_05.bin`, [S] suggested. Port: `crates/nf-game/src/nis.rs`
(`Script::anim_start`), `crates/nf-view/src/scene.rs` (character build), `crates/nf-game/src/drone.rs`
(`captain_setup`).

## The actor create path [M]

Every op-0x0A stream command (`u32 animation script, u32 skin, u32 start frame`) creates one cut-scene
actor: `Script_AnimStart` 0x1d8f40 → `AnimObjectNew` 0x1be910. The skin the actor wears is whatever hash
survives `AnimObjectNew`'s two gates:

1. **The per-level substitution [M 0x1be958-0x1be9ac]** — `lw $v1, 0x3774($v1)` reads `GameState+0xC`
   (0x2a3774; the current level id — the same field `Break_Update` reads with `lw $v0, 0xc($s7)` at
   0x12c070; the "+0x12" in older notes was the decompiler's decimal offset). For level 0x0700000C,
   skin 0x05000090 → 0x05000087; for 0x07000014, 0x05000090 → 0x05000093 (`movz $s7, $v1, $v0`).
   Only 0x05000090 is covered — the disassembly has exactly these two compare/movz pairs.
2. **The loaded-file hashtable** — `hashtable_getitem` 0x1ca5b0 maps the hash to its loaded record. The
   table holds every archive entry the loader processed (`LoaderProcess` 0x1d4880 case 3/4/5 →
   `AnimLoadFile` 0x1bdf40 registers skins/sequences/anims per map; nothing frees between levels), so a
   hash missing from the level's own archive resolves only if an earlier session loaded it. A miss makes
   `AnimObjectNew` return 0 (no anim record).

`AnimObjectNew` then writes the REQUESTED hash into the skin record's first word (`*puVar13 = (int)param_2`
[M 0x1becd8]) — unrequested records keep the malloc fill 0x98989898. **A fresh-boot EE snapshot therefore
records exactly which skins the game asked for** [L ee_start_07000001.bin]: records 0x05000009/17/2b/36/
38/39/3a/4f carry their own hash; the other 20 skin records still read 0x98989898.

The record feeds the draw: +0x04 celglist (+0x2C flag to apply it), +0x08 part-hash array (count +0x2D —
each part becomes a discrete child object of type 0x3D), +0x30 skeleton index.

## The drone-backed gameplay guards (the captain) [M]

Placed guards (`Drone_Create` via placement type 0x0F) take their skin from placement param 0 — EXCEPT the
captains: `NDrone2_DefaultInit` 0x14b300 (:528-548) switches on the drone's kind (`*(u16*)(DIVars+0xD8)`)
when the captain flag (+0x19, set by the mode settings' difficulty-gated properties: 9 on difficulty 1, 5
on 2, 8 on 3) is set:

| kind | captain skin |
|---|---|
| 1 | 0x05000038 |
| 2 | 0x0500002f (+ the +0x18 flag) |
| 3 | 0x05000006 |
| 6 | 0x05000093 |
| 7 | 0x050000b2 |

then `AnimObjectNew(obj, substituted)` (:552) and the `DroneCaptain_Mod_*` stat buffs. [L
ee_start_07000001.bin] confirms it live: 15 present drones wear 36×6, 39×4, 3a×3, 4f×1 — and ONE
0x05000038, which no placement names and no reachable script animates. The port's `captain_setup`
(drone.rs) mirrors this and produces the same set [test `cutscene_guard_models_live`].

## The cut-scene drone gate [M]

`Script_EventHandler` 0x1d94c0 case 5 fires `Drone_EnableAll(0, script_hash)` 0x1383e8 → per drone
`NDrone2_Enable(0)` 0x149dd0: `obj+0xF0 |= 0x10|0x20` (bit 0x10 = **not drawn** [M docs/fields/obj-tag.md
+0xF0], the +0x4F8 update bits cleared) and `NPCGlobals = 0` (Drone_Control/ spawners/ collisions stop).
`Script_Free` 0x1da880 re-enables (`Drone_EnableAll(1)`) when the script set the 0x80 flag (event 7); the
re-enable skips state-1 WaitSwitch drones and refuses corpses. Ported as `Request::DroneEnable` →
`Drones::npc_enabled` + per-drone `enabled` (the view's character build skips `!enabled` drones).

## Per-level tables (port vs game)

**07000001** — intro cut-scene 0x06000087 (pack 07800001.bin, SP #277 mode 1 auto-play; the game's boot
snapshot has it PLAYING, state 2 [L obj 0x018fbe60]):

| stream | anim script | skin (script) | port | game [L] |
|---|---|---|---|---|
| 1 | 0x0600055e | 0x0500002b | 0x0500002b | 0x0500002b (record +0x00 = own hash) |
| 2 | 0x0600055f | 0x05000017 | 0x05000017 | 0x05000017 |
| 3 | 0x0600055d | 0x0500004f | 0x0500004f | 0x0500004f |
| 4 | 0x0600055c | 0x05000009 | 0x05000009 | 0x05000009 (the player's skin on 07-1..4 [M 0x1910f8:93-105]) |

Guards' cut-scene 0x06000086 (SP #276, mask 1 out 0x33): streams 1/3/12 name (0x0600054e, 0x0500003a
w6), (0x06000550, 0x05000038 w0x1c), (0x0600054f, 0x05000036 w0x1c). **The game never plays it**: no
placement on any of 07000001's 54 maps and no script event 0xD writes switch_channels[0x33] [D scan],
and its SP sits in state 0 at boot [L obj 0x018fbfc0]. Dead content in both.

**07000005** — the ledge cut-scene 0x06000994 (SP #298, plate 301 → ch 0x43): streams 1/3/14 name
(0x06000996, 0x05000030 w0x16), (0x06000997, 0x0500002e w0x16), (0x06000995, 0x05000007) — 0x05000007 is
the default player skin [M 0x1910f8 default arm] and 0x0500002e/30 are the level's most common placed
guard skins [D: 16 placements — 2e×6, 2f×5, 30×5]. The truck 0x06000a22 animates skin 0x05000005 [D,
docs/formats/nis-script.md].

**07000006** — SPs #218 0x06000066, #219 0x06000a28, #220 0x06000071 (all mode 0 on channels 0x34/0x28/
0x83) and the #0x2000408 looping pair 0x06000055/56. Their scripts carry NO op-0x0A anim entries [D dump]
— the level's cut-scene characters are the drone-backed kind (07000006's "driver", skin 0x05000035, is a
placed drone; docs/systems/drone-animation.md), not script actors.

**0700000c / 07000014** — no ScriptPlayer script carries op-0x0A entries [D dump of all seven]; the
0x05000090 substitution applies to the 7 + 15 PLACED drones only [D placement scan; G-021 already ports
it at load].

## Port status

`nf_view::load` merges every ScriptPlayer script's anim skins (and sub-scripts') from foreign archives
when the level's own lacks them (models-characters G-021), and keys the 0x05000090 variant data under the
placement hash on 0700000c/14. `Script::anim_start` keeps the script's raw skin hash, exactly like
`Script_AnimStart`; nf-view builds one drawable character per (script, stream) from `level.skins`. The
intro actors' skins and stream pairings are pinned by `crates/nf-view/tests/cutscene_guard_models_live.rs`
against the [L]-verified game behaviour. Known divergence (open): a hand-written switch-channel value is
cleared by some port system within one tick, where the game's `switch_channels` persist until written —
invisible to shipped content on 07000001 (ch 0x33 has no writer), worth a census on the other levels.
