# The field map

What every field of the game's runtime structs means, and **how sure we are**. Started 2026-10-12 after several
confident misreadings by worker sessions (a draw layer read as alpha, an objective channel read from the wrong record,
a team gate that the decompile contradicts), each of which cost a playtest round.

## Proof levels (every field row carries one)

| Tag | Meaning | Enough for the port to rely on? |
|---|---|---|
| **[S]** | suggested: a name or guess, from one context or another project | no |
| **[M1]** | one reader or writer in the decompile, read by hand | no, for anything visible or gameplay-relevant |
| **[M*]** | **every** reader and writer found (Ghidra cross-references on the offset, listed in the row) and they agree | yes, for internal state |
| **[L-read]** | the value read live in PCSX2 (or an EE snapshot) is what the reading predicts, in a known game state | yes, together with [M*] |
| **[L-verified]** | **poked or driven live and the predicted effect seen** (screenshot or state change), with the probe command recorded | yes: required for anything the player sees |

Rule: a field the port relies on for something visible or for gameplay must reach **[M*] + [L-verified]**. A
disagreement between readers is written in the row; it's never resolved silently.

## The live probe

`tools/trace/fieldprobe.py` (PINE; PCSX2 running with the level loaded, nobody playing, no other PINE client):
`dump`, `find`, `ptrs`, `watch`, `poke ... --shot` (pins the value during the screenshot, then restores it), `diff`, `shot`.
Lessons so far:
- **Fields the game recomputes every frame can't be poked directly.** Poke their input instead. The HUD sprite layer
  (`+0x38`) is rewritten by `HUD_UpdateAmmoPane` each frame; poking the switch timer `BL+0x95E` that drives it works.
- **Timers count down under you.** Pin the value while the screenshot is taken (`poke` does this).
- **Get the address right.** Dump the target first. The probe prints the original value, and a surprise there means
  the wrong address (the first poke hit a neighbouring sprite).

## Coverage

| Struct | Doc | Fields mapped | [L-verified] | Notes |
|---|---|---|---|---|
| sprite (runtime HUD sprite) + HUD pane list | [sprite.md](sprite.md) | 14 | 3 | started 2026-10-12 |

Static passes for the remaining structs are queued as worker tasks (`docs/fields/<struct>.md`, [M*] level); the main
agent then runs the live pass for the fields the port relies on.
