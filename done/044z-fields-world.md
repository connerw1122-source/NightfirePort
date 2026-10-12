## Your task: `fields-world`, the field map for the world object structs: every field, every reader, with proof

Scope: `obj_tag` (the generic object: `docs/atlas/object.md` has a start; the class byte +0xFF, flags +0xF0/+0xFA/+0xFE, the matrix +0x60/+0x90, the world sphere +0x80), `level_tag` (the per-placement parameter block: params at +0x2C+4k, and the header fields below +0x2C, such as `+2` key count, `+4` rate, `+8` anim id, which are NOT mapped yet), and every per-type instance struct the port ports: trigger, door, pickup, switch, SimpleScript (SS), fusebox, CamSubject, Destroy, ladder, wire, searchlight, corona, script player and its entities (`Script_CreateEntity`), the mission objective record (0x18 bytes), and the glist/leaf/celglist render structs the model tasks rely on.

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

**Branch:** `s/fields-world`.

---
claimed-by: sandbox3 at 2026-10-12T19:19Z

---
## Report (done, fixed)

# Report: fields-world (044z)

Branch `s/fields-world` off origin/master (2026-10-12). Task: the field map for the world object
structs — every field, every reader, with proof.

## What's done

**18 field maps in `docs/fields/`** (sprite.md format; coverage table in `docs/fields/README.md`):

`obj-tag.md` (generic 0x110 header, reader-list status per row), `level-tag.md` (the 0xB0
placement param block: +2 key count, +4 rate, +8 key-list pointer — all three verified to the
parser 0x1d1b68 and their readers; per-creator param map covering all 48 direct-reader creators),
`trigger.md`, `door.md`, `pickup.md`, `switch.md`, `ss.md`, `fusebox.md`, `script-player.md`,
`searchlight.md`, `corona.md`, `camsubject.md`, `destroy.md`, `ladder.md`, `wire.md`,
`objective.md` (the 0x18 record + the 0x10-byte OBJ_STATE query result), `glist.md`
(celglist_tag), `leaf.md` (the obj+0x100..0x106 colour bytes + the tweak).

Method (docs/fields/RECIPE.md is the reusable recipe, `tools/fields/param_offsets.py` the scanner):
decode the creator (instance size, class byte, param map) → grep the whole decompile for every
access at each candidate offset (both pointer-arithmetic scalings) → follow each hit's base
pointer → [L] snapshot spot checks from `work/ee_start_05.bin` (live object-list walk) → port
cross-check. Subagent sessions covered door/ladder, trigger/switch/fusebox, pickup/ss/script-
player, searchlight/corona/camsubject (their worklog entries are in /home/z/my-project/worklog.md;
their drafts were re-verified before promotion — the door agent's DOORINFO map and the pickup
agent's atlas errata are theirs).

**Every doc ends with a "Live checks" section** for the main agent (fieldprobe/gdbwatch): the
pointer chain, the byte range, the expected screen/state, and the census to run per field the
port relies on.

## Port fixes (all fail-first; every fix cites its doc entry and the game address)

1. **Fusebox live-wire hurt polarity** (`crates/nf-game/src/fusebox.rs`): the game hurts on a
   group-1 wire death **iff +0x56 (param 2) == 0** [M `FuseBox_Update` 0x1adfe8:99-101]; the port
   had `!= 0`. Also implemented the **+0x5C clear**: a group-1 death zeroes group 0's second
   channel word before it can fire [M 0x1adfe8:102]. Tests `live_wire_hurts_only_when_param2_unset`
   / `group1_death_cancels_group0_second_word` fail against the old code (shown). NF_ISO
   `fusebox_07000014_param2_decides_the_live_wire_hurt` (tests/fields_live.rs): **all five of
   07000014's real fuseboxes carry param 2 non-zero** (225/113/216/224/112) — the old port hurt
   the player on every live-wire cut; the fixed one never does, matching the game.
2. **Switch inhibit-gate polarity** (`crates/nf-game/src/useobj.rs`): usable while the param-5
   channel is **OFF** [M `Switch_Activate` 0x1b02a8:22]; the port required ON. **Busy is a
   one-shot latch**: with param 4 set, the switch dies after its first channel change (+0x18
   bumps on every change, never resets [M `Switch_Update` 0x1adea8:22,29]); the port had a
   re-armable 60-tick cooldown. Tests `switch_inhibit_gate_blocks_only_while_on` /
   `switch_busy_is_one_shot_never_rearms` fail against the old code (shown).
3. **Raw 0xDC trigger param shift removed** (`crates/nf-game/src/triggers.rs` `build()`): every
   trigger family — typed wrappers and raw 0xDC — shares one layout (out = p1, inputs = p2..p8,
   mask = p9, lock = p10, hide = p11; type byte at p0) [M `Trigger_Create` 0x1b1720:61-91]; the
   port shifted raw-0DC params by one (`*suggested*`). Not latent: the NF_ISO census finds **3 raw
   0xDC placements on 0700000c** (kind 2 TOUCH, out 27) — the old shift broke their out channel.
   Test `raw_0dc_triggers_use_the_unshifted_param_layout`.

NF_ISO tests live in `crates/nf-game/tests/fields_live.rs`; unit tests in `fusebox.rs` /
`useobj.rs`.

The fusebox fix also exposed a test that had **codified the old misreading**:
`crates/nf-view/tests/walk_07000014_live.rs:106` asserted "the live wire hurts 5.0" — it failed
against the corrected polarity and its assertion is updated to the game's actual behavior (no
hurt with +0x56 configured; all five of the level's fuseboxes carry param 2 non-zero). This is
exactly the README's motivating failure mode: a test passing on a wrong reading.

## How it was verified

- Every claimed reading cites decompile file:line in its doc row; scaling rules (Ghidra's
  pointer-typed arithmetic) written down and applied (RECIPE.md).
- The 48 `level_tag` readers enumerated by the scanner; the per-creator param map cross-checked
  against each struct's own doc.
- [L] snapshot checks: object-list walks on `ee_start_05.bin` (523 objects, class histogram) and
  `ee_start_07000001/02.bin` where noted; disc-level checks through the NF_ISO tests (07000014
  fusebox params, the 0700000c 0DC census).
- Full suite green before push (nf-game / nf-formats / nf-audio / nf-view, NF_ISO set).

## What's left / open items for the main agent

- **The live pass** ([L-verified]) for anything gameplay-visible: each doc's "Live checks" section
  has exact probes. Highest value: the switch gate poke (0x1b02a8's +0x14), the fusebox +0x56
  polarity (watch `switch_channels[param2]` while cutting), the Destroy +0x04 health walk
  (07000002), the searchlight kill-path part deletion.
- **Port fixes deferred** (documented in the docs' "Port mismatches" sections, not fixed here):
  the SS playhead/channel model (ss.md — the channel is an OUTPUT and the playhead advances
  unconditionally in states 1/8/0x40); the searchlight kill/break paths (searchlight.md — the
  port hides the mount and never swaps the LOD); pickup respawn/kind-5/MP registry (pickup.md);
  the trigger MoviePlayer kind, SEQ_AND window gate, Channels::default time seeding (trigger.md);
  the SP-idle and SP_Activate facing gates for switches (switch.md); Destroy is not modelled at
  all (destroy.md — placement 0xE1 is unhandled).
- **Atlas errata found** (for the main agent to fold in): docs/atlas/script-player.md's SSTREAM
  base is +0xA8 not +0xB0; docs/atlas/object.md's drone "placement+0x34" spawn-frame gate is
  level_tag+0x34 = param 2; atlas door.md's "Activate reads +0x66" is wrong (the draft corrects
  it); "p10 = 0 → 1.0" for the EndLevel script player is wrong (p10 = 100 through the EE's
  non-IEEE multiply).
- obj-tag.md's [M1] rows are the gdbwatch census targets (the doc lists the expected accessor
  set); the class-3 header reading is s/044x-fields-player's obj-tag-player.md — merge order
  should keep both.

Shared files touched beyond the task's named set: `docs/fields/README.md` (additive section),
`docs/WORKLOG.md` (append-only), `crates/nf-game/src/{fusebox,useobj,triggers}.rs` (the fixes),
`crates/nf-game/tests/fields_live.rs` (new), `tools/fields/{RECIPE.md,param_offsets.py}` (new
tooling). No game bytes, tokens or URLs committed.
