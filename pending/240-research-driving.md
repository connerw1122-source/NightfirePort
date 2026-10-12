## Your task: `research-driving`, an atlas of the driving levels (DRIVING.ELF) for a later port phase

The port covers the on-foot levels (ACTION.ELF). Nightfire's driving missions run on a separate executable, DRIVING.ELF (its build identity is in `docs/build-identity.md`; the decompile is in `work/decomp/DRIVING.ELF/`). Nothing has been mapped yet. Write the map so a later phase can plan the port.

1. **Which levels:** the driving missions, their archive files on the disc, and how the front end hands over to DRIVING.ELF and back (the ELF switch, the arguments, the save state carried).
2. **The main loop and the subsystems:** the frame loop, the vehicle physics (the car model, the wheel and suspension, the collision), the weapons and gadgets on the car, the AI traffic and the pursuers, the camera, the HUD, sound and music. For each: the entry points, the key structs with offsets, and how far it shares code with ACTION.ELF (the same engine functions at other addresses, or new code).
3. **The data formats:** which of the on-foot formats (`docs/formats/`) the driving archives reuse unchanged, which differ, and what's new (the road and track data, the vehicle definitions). Run the existing parsers in `crates/nf-formats` over the driving archives and record what parses and what doesn't, as numbers.
4. **The input path:** `PS2PadDevice`, `InputDevice::DeadZoneChop/Scale` (GAPS G-004; the DRIVING.SYM names are hints only, verify them).
5. **A port estimate:** what can be reused, what's new, and a phased plan with sizes.

Deliverables: `docs/atlas/driving.md` (the map), `docs/formats/` notes for any new formats, an NF_ISO test that parses every driving archive with the current parsers and pins the results as golden numbers, and the plan in your report.

**This is a research task.** No game code changes. Rules:
- Every fact gets a tag ([M] with the function and address, [D], [L], [S]). Check the disassembly wherever the decompiled C looks incomplete.
- Use subagents in parallel, and have an independent auditor re-derive your key claims from the decompile. Fix or retag anything it can't reproduce.
- Never commit game bytes: numbers, offsets, names and hashes only.

**Branch:** `s/research-driving`.
