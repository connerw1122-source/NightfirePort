## Your task: `live-backlog`, one ledger of open questions and the live checks that settle them

Idea taken from REA (morluto/rea, MIT): its `record_unknown` / `update_unknown` keep every unresolved question with the
authority or environment still needed to settle it, plus the supporting AND contradicting evidence, under a revision, and
"only qualifying observed evidence can verify a resolution". This project scatters the same thing across ~60 reports'
"Live checks for the main agent" sections, `docs/UNKNOWNS.md` and `docs/GAPS.md`, so checks get lost.

1. **`docs/live/backlog.md`**: harvest every "Live checks" item from `docs/reports/*.md` (and the UNKNOWNS rows that need
   the real game). One row each: id (L-001…), the question, the exact procedure (level, `--pos`/`--pitch` spot, the
   tool: `tools/trace/pcsx2_shot.py`, `gdbwatch.py`, `gdbcall.py --spec …`, `fieldprobe.py`, what to read), the
   expected answer(s) and what each answer would change in the port, the evidence for and against so far (cite the
   report/doc), and a status: open / answered (with the answer and the docs/live file that records it) / obsolete (with why).
2. Mark the ones already answered: the ground-truth doc `docs/live/2026-10-13-portal-ground-truth.md` settled several
   portal checks, the 05 gate-door icon context (0xFF) settled fix-swim-vent's, the PCSX2 menu captures settled
   fix-menus'. Search `docs/live/` and the merge log.
3. Order the open ones by value: which unblock a held or parked task, which affect the most levels. Group them by level
   and spot, so one PCSX2 session can clear several.
4. Add a short rule to `docs/live/README.md`: new reports add their live checks to the backlog with an id, not only in
   the report.

Docs only. **Branch:** `s/live-backlog`.
