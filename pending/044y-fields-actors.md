## Your task: `fields-actors`, the field map for the actor structs: every field, every reader, with proof

Scope: `Drone_tag` and `DCVars` (the drone instance and its control vars; the port reads many offsets, e.g. +0x4F8 flags, +0x2B4 killer, +0x504 alertness, +0x5A4 alt state, +0xCF8 scared radius), `DroneTypeSettings` (0x29aee8), the state-function table (0x29b300), `AnimObject` (`obj+0xDC`: +0x62/+0x63 weapon bytes, the LOD fade bytes), `HITDATA_tag`, and the spawner / ally / mission-fail fields `fix-actors` used.

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

**Branch:** `s/fields-actors`.
