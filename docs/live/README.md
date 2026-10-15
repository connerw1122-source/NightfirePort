# docs/live — ground truth from the real game

Records of what the retail game (007: Nightfire, PS2 NTSC-U) actually does, measured by the main agent in
PCSX2 with the trace tools (`tools/trace/pcsx2_shot.py`, `gdbwatch.py`, `gdbcall.py`, `fieldprobe.py`,
`pad_log.py`, `goto_level.py`, PINE). Files are dated measurement sessions; the port's fixes cite them as
[L] evidence.

- `2026-10-13-portal-ground-truth.md` — portal culling at three placed spots (06 courtyard, 07 window,
  07 clock hall), the game's visible-cel list, the chain of cel 0x0179fb30, and the forced-objects refutation.
- `2026-10-13-weapon-modes-pcsx2.md` — the fire modes of every weapon in a multiplayer match (the Sentinel's
  guided camera, MGL impact vs delay, the 32-weapon mode table, the mode button).
- `2026-10-14-weapon-measurements-pcsx2.md` — live measurements: rocket speed, grenade release timing, the
  stunner, the laser watch, the remote mine, the AIMS-20, SG5 casings, guard sighting→first shot, per-level
  loadouts and gadget cycles.

## The backlog rule

Every open question that needs the real game lives in **`backlog.md`** — one ledger row per question, with its
procedure (level, spot, tool, what to read), the expected answers and what each would change in the port, the
evidence so far, and a status (open / answered / obsolete). It is harvested from every report's "Live checks
for the main agent" section and from the `UNKNOWNS.md`/`GAPS.md` rows that need live play.

**New reports add their live checks to `docs/live/backlog.md` with a row id (L-NNN), not only in the report.**
When a check is answered, mark its row `answered` with `SETTLED:` naming the `docs/live/` file (or report)
that records the answer, and add the answer to the session that measured it. A check settled by a live session
but left only in the report is a check lost.
