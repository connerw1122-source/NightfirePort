## Your task: `research-multiplayer`, an atlas of multiplayer and the bots, for a later port phase

Nightfire PS2 has split-screen multiplayer with AI bots (the arenas, such as 0x0700004b, which `map-finale` found
is a multiplayer map). Nothing of it is mapped. Write the map so a later phase can plan the port:
1. **The arenas:** every multiplayer level on the disc, its spawn points (`MP_RegisterSpawnPoint`), objects
   (`MP_RegisterMPObject`) and pickups, and the game modes (`MPSettings`: deathmatch, team, capture-the-flag and the
   rest; their rules, scoring, timers).
2. **The bots:** the AI behind them (their states, navigation, weapon choice, difficulty settings, how they differ from the
   campaign drones), with entry points and key structs with offsets.
3. **Split screen:** the viewport setup, per-player HUD, input routing for 2–4 pads.
4. **What the port can reuse** (the drone AI, the nav graph, the pickups) and what's new, with a phased plan and sizes.

Deliverables: `docs/atlas/multiplayer.md`, an NF_ISO test that loads every arena with the current parser and pins
what parses, and the plan in your report.

**This is a research task.** No game code changes. Every fact gets a tag ([M] with the function and address, [D], [L],
[S]); use subagents in parallel, and have an independent auditor re-derive your key claims. Never commit game bytes.

**Branch:** `s/research-multiplayer`.

---
claimed-by: sandbox1 at 2026-10-13T10:00Z
