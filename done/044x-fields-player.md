## Your task: `fields-player`, the field map for the player, HUD and weapon structs: every field, every reader, with proof

Scope: `BLData` (the player's data block, `obj+0xE0` of the player object; the port reads dozens of its offsets: health +0x894, armour +0x8B0, ammo +0x170, clips +0x1B8, the switch timer +0x95E, the HUD block +0x7E0, pitch +0x8A8 and more), the remaining HUD pane structs and every `*SprInfo` table, `weapon_data` records (0x10C each: `docs/formats/weapon-data.md`), `ammo_data`, `PlayerStats` / `PlrStat_*`, and the player object's class-3 use of `obj_tag`.

**Why:** worker sessions have repeatedly read a field confidently and wrongly (a draw layer read as alpha, an objective
channel taken from the wrong record, a team gate the decompile contradicts). Each cost the user a playtest round. Read
`docs/fields/README.md` first: the proof levels, and the method the main agent used on the HUD sprite
(`docs/fields/sprite.md`, the worked example).

**The method, per struct:**
1. **Find every reader and writer of every offset.** Grep the whole decompile (`work/decomp/ACTION.ELF/*.c`) for the
   struct's access pattern (the base pointer's name varies, so search the offset with its typical forms: `+ 0x38)`,
   `[0xe]` and so on), then confirm each hit really is this struct (follow where the base pointer comes from). Where the C
   looks incomplete, check the disassembly. A field is **[M*]** only when every hit is listed in its row and they agree.
2. **Write disagreements down.** The same offset meaning different things for different object classes, a union, a
   reused byte: record each case and the condition that selects it. Never resolve a conflict silently.
3. **Cross-check with the EE snapshots** (`work/ee_start_*.bin`, file offset = EE address): find live instances and
   check the values make sense (**[L-read]**, with the address and the value).
4. **Check the port.** For every field the port already uses (grep `crates/` for the offset or the name), check the
   port's reading against your map. A mismatch is a bug: fix it with a test (fail first, with a sister assertion), or
   if it's big, describe it in the report.
5. **Write the live checks for the main agent.** For each field the port relies on for something visible or gameplay-
   relevant, give an exact probe the main agent can run in PCSX2:
   `python -I tools/trace/fieldprobe.py poke <addr expr> <width> <value> --shot out.png` (or `watch` / `diff`), how to
   find the instance live (the pointer chain from a global), and **what the screen or state should show** if your
   reading is right. If the game rewrites the field every frame, give the input that drives it instead (the
   sprite.md lesson). Put these in a "Live checks" section at the end of each doc.

6. **Ask for the live census.** The main agent can now record, in the running game, every instruction that reads or writes a field (`tools/trace/gdbwatch.py <start> <end> read|write|readwrite --seconds N`; worked example in `docs/fields/sprite.md`: `+0x38`'s three accessors). For each field the port relies on, give the census to run: how to find a live instance (the pointer chain from a global), the byte range, the access type, the game situation (the level and what the player does), and the accessor functions your static pass expects. A census that finds an accessor you didn't list means the static pass missed a reader; that's the point of it.

**Output:** `docs/fields/<struct>.md` per struct, in the format of `docs/fields/sprite.md` (offset, type, meaning,
proof with every reader/writer cited), and the coverage table in `docs/fields/README.md` updated with your structs.
No game bytes in the docs: offsets, values you read, names and addresses only.

**Reproduce before you fix.** The user's play is the reference, and previous fixes passed tests but still failed in play. For each bug:
1. Write a headless NF_ISO test that drives the real input path (the `nf-view` input mapping into `Sim`, or `nf_view::load` plus `dev::scenario`), and show it reproducing the user's symptom.
2. Then find the cause in the decompile and fix it.
If you can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Update the doc entry** to say it's fixed, with your commit.

**Branch:** `s/fields-player`.

---
claimed-by: sandbox2 at 2026-10-12T16:37Z

---
## Report (done, 2026-10-12T19:34Z)

# 044x-fields-player — the field map for the player, HUD and weapon structs

Branch `s/044x-fields-player` (from origin/master). The task: map BLData, the HUD pane structs and every
`*SprInfo` table, the `weapon_data` record, `ammo_data`, the `PlrStat_*` family, and the player object's
class-3 use of `obj_tag` — every field, every reader, with proof — then check the port and fix what
mismatches.

## What's done

**Docs** (format of `docs/fields/sprite.md`; every row carries [M] decompile citations with file:line or
`mipsdis` addresses, [L] snapshot values, and the port status; each doc ends with "Live checks for the
main agent" and "The census to run"):

- `docs/fields/bldata.md` — BLData end to end (0x000..0x970; the size is [M] `Player_Init` 0x1910f8:33's
  `control_create_object(0x970, …)`): the position snapshot, the velocity block, the floor/aim/crosshair
  block (incl. +0x114 auto-aim target, +0x130/+0x131/+0x132/+0x133 states), the grapple block 0x140-0x15F,
  the ammo store (33 × u16), the 114 × 0xC weapon records (lane map +0/+4/+6/+7/+8; **+0x4F4 = clip[0x45]**,
  [D] `lh v0,1268(s1)`), the key ring 0x70C..0x73E, the HITTEST scratch 0x740-0x7DF, the HUD pointer +0x7E0
  (`HUD_Init` 0x199e88 mallocs 0x2C8 — a pointer, not embedded), the object-pointer block 0x7E8-0x890
  (+0x884 is an obj_tag POINTER, not an index), and the full 0x894-0x96F state block. The RamSave copy map
  anchors the layout; the HandlePain arithmetic is spelled out.
- `docs/fields/hud-panes.md` — HUDPANE_tag (0x20) and HUDPANECREATE_tag (0x1C) complete; both pane lists
  decoded; **all 26 SprInfo tables** inventoried (address/size/entries/consumers — every one a clean 0x2C
  stride; HealthSprInfo is 24 entries, 1056 = 24·44); the BSS status tables' boot filler
  (`__static_initialization_and_destruction_0` 0x1a0058) with the 0x30-stride source region
  0x2FC2E0..0x2FC4F0; TextMsgFormats 0x2BF108; the AmmoPane CREATE-tag worked example that reproduces the
  port's `AMMO_AT (496,394)`.
- `docs/fields/weapon_data.md` — the 0x10C record mapped over a 58-file reader inventory: damage +0x0C,
  pellets +0x2C, refire +0x40, muzzle colour/scale/intensity +0x51..0x58, **+0x60 = a u16 view-anim datum
  id**, zoom +0x88 f32 (Upgrade_Weapon writes 8.0f/16.0f), ammo type +0x90, clip capacity +0x92, rumble
  +0x94, the 14 view-script hashes +0xA0..+0xD8, casing vector +0xF8. Zero/readerless columns listed.
- `docs/fields/ammo-data.md` — the 34 × 12 records (+0 unused, +2 max carried, +4 casing gfx id, +8 name
  label), the BL+0x170 indexing/clamp logic, and the complete weapon→ammo map from the measured table.
- `docs/fields/player-stats.md` — PlrMissionStats (8 × 56, every counter's writer and reader), BondMoments,
  ScoringTable (12 × 60) + the 9-row arrays, ScoringTableDifMul, RewardsTable, the bonus bitmask, the
  GetScore formula row by row, the medal/gold logic and the save path. **The whole system is unported** —
  stated with the port's stand-in citations.
- `docs/fields/obj-tag-player.md` — the `control_funcs` class table (0x26f890; the player is class 3 with
  update 0x193600 / collide 0x1949a8 / no delete; re-classed to **0x12** on death, back on respawn;
  `glb_players` is 4 slots), then every header field's class-3 meaning incl. +0xF4 (the player state
  0/1/2/3/4), +0xF6 (the substates), +0xF8 (the per-view draw mask — [L] 0x001E), +0xFA/0xFC/0xFD/0xFF,
  +0x106 (the alpha floor 0x28).
- `docs/fields/sprite.md` — a corrections/extensions section (the label's three cases, the runtime
  rt+0x1C/0x20/0x22/0x28 fields, the 0x1000 clip-to-screen flag, the BSS fillers, TextMsgFormats).
- `docs/fields/README.md` — the coverage table updated with all seven rows.

**Port fixes** (each: decompile proof → fail-first test shown red on the pre-fix code → green after):

1. **The armour gate by pain type** [M `Player_HandlePain` 0x1902c8:84-104]: only pain type 0 (bullets)
   lets armour absorb; types 1..7 (fall 6, drowning 7, hurt volumes) set armourFraction 0.0. The port's
   `PlayerHealth::pain_generic` always absorbed. Fixed via the shared `apply(…, armour_frac)`.
   Tests: `nf_game::bullet::tests::armour_bypass_by_pain_type` (red pre-fix: "a fall must not drain
   armour") + the NF_ISO sister `bullet_damage_still_absorbs_into_armour`.
2. **Fall/drown damage bypassed the whole pain pipeline** — `scene.rs` subtracted raw damage from health
   (no difficulty factor, no pain flash BL+0x968, no direction bits BL+0x967). Now `pain_generic(…, 6)` /
   `(…, 7)` [M 0x1949a8:117-126, 0x191a70:136-143]. Fail-first: `fall_damage_runs_the_full_pain_pipeline`
   (the flash assert is red pre-fix; armour-untouched is the sister).
3. **The clip regen was unmodeled** [M `Player_Update` disasm 0x193eb8-0x19405c]: while the view-anim
   state ≠ 9, clip[0x4A]/[0x4E]/[0x33] refill on `frame & 3 == 0`, clip[0x4C]/[0x4F] on `& 1 == 0`,
   clip[0x45] (the grenade belt) on `% 6 == 0`, each +FRAME_RATE_MUL toward `weapon_data[w].clip_size`.
   Ported into `step_bond`; fail-first `clip_slots_regen_on_the_game_schedule` (with clip[0x44] as the
   must-not-regen control). NOTE: this block was almost ported wrong from the C — Ghidra prints shifted
   pointers (`puVar2 + 0x13d`); the disassembly gives the true addresses.
4. **weapon.rs**: +0x60 retyped from a u32 "fire_sound" to the u16 `view_anim_datum` [M 0x1c7964 `lhu`];
   +0xCC/+0xD8 comments corrected to script hashes (the flag tests work because script-present IS the
   flag; the table values are 0x0600xxxx on every armed weapon [L]).
5. **The +0xCC root-height correction** (docs): the every-tick copy is `obj+0xC0..0xCC = obj+0x30..0x3C`
   (matrix row 3, flags2-bit-4-gated) — NOT a root-height write; the root height lives at AnimObject+0xCC
   and the feet-point math reads it there [M 0x194df0:86,222-236]. `docs/systems/player.md` §Collision and
   the `player.rs` CROUCH_ROOT_HEIGHT comment corrected; the port's VALUES were already right, so no
   behaviour change and no test (stated honestly).

**Shared files touched** (minimal, listed per the rules): `docs/systems/player.md` (the +0xCC paragraph),
`crates/nf-game/src/player.rs` (comment only), `crates/nf-view/src/scene.rs` (two damage sites + the regen
block), `crates/nf-game/src/bullet.rs`, `crates/nf-formats/src/weapon.rs`, `docs/fields/sprite.md`,
`docs/WORKLOG.md` (appended only).

## How it was verified

- 7 research subagents (parallel; 2 re-scoped after timeouts; the BLData 0x000-0x170 region grepped by
  hand), every one citing file:line; the ELF-vs-snapshot byte identity checked for weapon_data/ammo_data/
  the HUD tables; the live player/obj read from `ee_start_05.bin` (health 100.0, armour 0.0, the switch
  timer 28, the draw mask 0x001E, the class byte 3, AnimObject+0xCC 1.0347…).
- The fail-first demonstrations are in the transcript: all four new tests red on the pre-fix code
  (`assertion failed: a fall must not drain armour`, the flash assert, the regen assert, the sister's
  first version also caught a wrong expectation of mine — armour fully absorbing an 18-effective hit
  leaves health at 100, which the corrected sister now asserts).
- Full suite (`--no-fail-fast`): **101 test binaries, 473 passed, the 3 allowed `disc_golden` failures**
  (the slim ISO). New tests: 4 (1 nf-game unit + 3 NF_ISO in `crates/nf-view/tests/fields_player_live.rs`).

## What's left (reported, deliberately not fixed)

- **The locks never consult the key ring**: [M] `Lock_Activate` 0x1aed00:19,26 / `Lock_Update` 0x1ae3d8:143
  gate on `BL+0x70C+id != 0`, `Lock_ReLock` 0x1b0750 revokes. The port's `lock.rs` is burn-only and the
  gadgets are unported (docs/port-map/gadgets.md), so a key check would be dead code today — flagged for
  the gadget task. Also: no HUD reader of the key ring exists (the key display path is elsewhere/unknown).
- The zoom block (BL+0x8D0/0x8D4/0x8D8 + the record lane 0), the freeze gate (+0x94A/0x94B), the pain
  overlay f32 (+0x8BC), water_fade's missing renderer consumer, the hurt-volume pain types (the game
  passes `Hurt_GetType`; the port passes 0 — the port's hurt records don't carry a type), the cheat's
  health-subtraction skip, the missing weapon_data accessors (+0x40 refire, +0x2C pellets, +0x1C range,
  +0x94 rumble, +0x7C view-model hash, +0x58 flash intensity), the HUD flag 0x1000, obj+0xF4's state
  machine (needed before death/respawn is ported), and the unported scoring system (see player-stats.md).
- Live checks wanted from the main agent: the three per-doc sections ( armour-bypass-on-a-fall watch, the
  BL+0x4F4 regen watch, the +0x95E writer census, the class-byte 3→0x12 flip, the obj+0xF8 draw mask, the
  obj+0x106 alpha floor, the ammo clamp) and the four `gdbwatch` censuses (BLData 0x970, the HUD block
  0x2C8, weapon_data during a fire pass, PlrMissionStats during a fight).

## Live checks for the main agent

Consolidated: `docs/fields/bldata.md` §Live checks (falls with armour; the grenade-belt regen; the
+0x95E writer), `hud-panes.md` (the pane count gate; the 0x1000 exclusion), `weapon_data.md` (the +0x60
datum id; the +0x40 refire cadence; the regen caps), `ammo-data.md` (the reserve clamp; the casing id),
`player-stats.md` (the accuracy row; the detection penalty), `obj-tag-player.md` (the class flip; the
draw mask; the alpha floor). Each names the address, the access type and the expected screen/state.
