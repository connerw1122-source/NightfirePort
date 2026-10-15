# Report: mp-debriefing-data (branch `s/mp-debriefing-data`)

Task: port what ends an MP match (the scenario rules — time/score limits), the debriefing's
rows and columns (label ids, values, ordering), and the return flow. Full write-up:
`docs/systems/mp-debriefing.md` (addresses + evidence tags throughout).

## What's done

1. **What ends a match** — `MP_CheckForEndCondition` 0x184268 and `MP_Update` 0x184bd0's state
   machine ported whole into `crates/nf-game/src/mp/mod.rs`: the Demolition/Protection arm
   jumps to state 6 WITHOUT latching the time-up channel (the restart loop), Top Agent's
   elimination test (deaths ≥ the lives limit; the port carries the out-state as one
   `MpSlot::out` flag — *Deviation* documented), the top-score scratch (`MPGame+0x18C`),
   the frag latch (0xfd → state 1), the time-up latch (0xfe → state 2), then states 1→3
   (`MP_SortOutWhoWon`), 2→3, 3→4 (`GameState+0x24 = 1`), 4→5 (`GameState+0x1C = 0x40000033`
   + the level request — disasm-pinned: `sw $v0,0x1c($a2)` @0x184e80; the older mp-menu-flow
   note's "+0x28" was the decompiler's decimal offset), and 6's 5-second restart.
   `MpMatch::player_killed` ports `MP_PlayerKilled` 0x188f70's credit rules (deaths always,
   kills gated on the team rule, suicide −1, the ±2.0 weapon-bonus arm as a flag, the score
   delta only for Arena/Team/KOH with bit30 clear, team scores only in Team Arena). The
   assassin +5/+3 arms need the target globals — recorded, not ported (phase 2).
2. **The debriefing board** — `crates/nf-game/src/mp/debrief.rs`: `MpMatch::get_mp_score`
   (kills for Arena/Team Arena, the slot score for the objective modes, 0 otherwise —
   `Menu_GetMPScore` 0x203af8) and `MpMatch::debriefing()` (the (slot, TOTAL) pair array over
   humans-then-bots, the stable descending bubble sort, the rank walk where ties share the
   "1st"-"4th" captions [L 0x309520 = 0x01000297-9a], the cap at 4 rows, names
   (codename / `Menu_GetBotShortName` = 0x010002e2+character), the portrait character, and the
   result line: draw 0x010002a0, "%s wins the match." 0x0100029f with the winner's codename
   (disasm: the arg is `MPSettings+slot·0x30+0`), or the team announce 0x1c7/0x1c8 — ported in
   the disasm's literal order (team0 < team1 → MI6), with the suspected original inversion
   flagged [S] and a live check.
3. **The page data** — the debriefing page 0x40000033's menu-def dump (43 controls): header
   0x151 "Debriefing", hint 0x2c1 "~A Continue ~Y Replay", columns Agent 0x14e / Points 0x24c /
   Victories 0x686 / Deaths 0x2be / TOTAL 0x163, the per-slot widgets 0x1000015d/57/56/58/59/
   240/15a/5c and the result line 0x1000023d. `0x1000015b` is sent by the handler but does not
   exist in any of the 56 pages — a dead send. `MP_CHARACTERS_SMALL` (29 portrait rows from
   0x2df8f8) lifted and disc-pinned.
4. **The front-end surface** — `crates/nf-view/src/menu.rs`: `FrontEnd::mp_debriefing` +
   `open_mp_debriefing(&MpMatch)` (the host's case-4 handover), the getters
   `mp_debriefing_rows()` / `mp_debriefing_result()` / `mp_debriefing_portrait()`, the page's
   label consts, and `Action::Extra` (the ~Y arm): the debriefing's Confirm returns to the
   scenario page (C_MPDBG 0x4b — the port keeps the menu-side MP state, since the game's
   `Menu_RestoreMPSettings` exists only because the match compacted MPSettings in place);
   Extra re-requests the arena with `mp_match_settings` preserved (C_MPDBG 0x5d's
   ResetMap_LevelToLoad(+0x1A8) + the fade push).
5. **Docs corrections recorded** (in mp-debriefing.md, additive): the decompiler prints struct
   offsets in DECIMAL — several existing mp offset comments carried the decimal numbers as hex
   (e.g. the MpGame state is `MPGame+0x188`, not +0x392; team scores +0x180/+0x184; the
   debriefing navigation writes `GameState+0x1C`, not +0x28). The port's FIELD meanings were
   all verified correct; only the comment hexes drift.

## How it was verified

- `crates/nf-game/tests/mp_debriefing.rs` — 9 headless tests: the kill/score/team rules, the
  +2.0 bonus and the suicide rules, the frag-limit end → the sort → the status, time-up →
  states 2→3→4→5 with the debriefing request, Demolition/Protection's state-6 restart (and
  that 0xfe stays clear), Top Agent's elimination + `MP_SortOutWhoWon`'s tie semantics (the
  survivor count re-increments after a tie — the draw verdict only when the tie is the last
  qualifier), the board's sort/ranks/ties/cap-at-4 + the result line, `get_mp_score` per
  scenario word, and the single-player board. Every step has a sister assertion.
- `crates/nf-view/tests/mp_debriefing_flow.rs` — 6 tests: the handover opens 0x40000033 with
  the right rows/portraits/ranks (disc-pinned textures), the result-line variants (win /
  draw / team-MI6), Confirm → the scenario page, Extra → the arena re-request with the config
  kept, the static label ids, plus 2 NF_ISO tests binding every debriefing label against
  USATxt.dat and pinning `mp_characters_small` byte-for-byte against the disc's ACTION.ELF.
- **What I ran** (release, --offline, -j2):
  - `cargo test -p nf-game` — full crate: **350 passed / 0 failed** across 39 binaries.
  - `cargo test -p nf-view --test mp_debriefing_flow --test mp_menu_flow --test
    mp_menu_lists_live --test mp_core_live --test menus_flow` — **55 passed / 0 failed**
    (the MP/front-end area my edits touch).
  - `cargo test -p nf-view` (the whole crate) was started but the sandbox shell killed it at
    the 10-minute limit mid-run — by then every reported binary was green and the run's
    FAILED scan showed none; the five binaries above are the ones my changes can affect
    (the Action enum gained a variant; the MpPlayer literal gained a field).
  - The fast-runner suite (`tools/test/suite.py`) could NOT run in this sandbox: background
    jobs are reaped here (the nohup'd runner died within its first minute, as the previous
    workers also recorded), and the foreground window fit only the direct runs above.
- Two research subagents produced the evidence base (the debriefing handler's disasm, the
  label/string dumps, the menu-def page dump); their dumps are reproducible from the cited
  addresses.

## What's left (for the next message)

- The sim-side wiring: the host loop calling `MpMatch::update` in real matches and consuming
  `request_debriefing`/`restart_pending` (the front-end handover exists;
  `open_mp_debriefing` is the entry point). `MP_RestartScenario`'s body (the demo/prot
  re-init) and the carried-object drops on death are phase 2 (no MP objects are driven yet).
- The assassin +5/+3 scoring (needs the `Assasin`/`DAT_0030d774` target globals).
- The 0x10 elimination's `out` flag source (the player-state byte 0x11/0x12) once the sim's
  players report their state; the port's single-flag shape is documented.
- A full `tools/test/suite.py` pass on the merged tree (see above — environmental).
- Draw-side: the page render (the main agent; the getters + the menu-def geometry in
  mp-debriefing.md §4 are the surface). The [S] team-announce inversion (§4) would settle
  with one PCSX2 look (live check 2).

## Live checks for the main agent

See `docs/systems/mp-debriefing.md` §6 — four checks: `GameState+0x1C`/`+0x24` on a natural
end (expect 0x40000033 / 1), the team-announce label (settles the [S] inversion),
`MPGame+0x188`'s 1/2/6/3/4/5 sequence, and a known 3-bot Arena board's rank captions.

## ntfy

curl → HTTP 429 "limit reached: daily message quota reached"; python3 urllib → HTTP 429; wget
→ exit 0 (accepted, or at least not rejected with a printed error). Not all three failed, so
per the rules this is not an "ntfy failed" record — the exact outputs are here for the driver
in case the message did not land (the same pattern the mp-menu-flow worker recorded).
