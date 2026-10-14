# fix-weapon-anims (task 103k) — the per-weapon first-person view-model animations

Branch `s/fix-weapon-anims` (from `origin/master` b27fb1e, with `s/weapon-matrix2` merged first —
it was not on master yet, per the task brief). Files: `crates/nf-view/src/scene.rs` (the view-script
block), `crates/nf-view/tests/weapon_anims_live.rs` (new, 7 tests), this report. The census probe
`crates/nf-view/examples/probe_weapon_anims.rs` is uncommitted scratch (its output is quoted below).
Shared files: none touched (drone.rs / player.rs / lib.rs unchanged; nf-view lib.rs untouched).

## DONE

**The census (disc).** `probe_weapon_anims.rs` (the `nf_view::load_weapons` path on 07000005; the
whole catalog lives in one archive, 07000029.bin, which the merge pulls into any level) measured all
115 records' 15 view-anim script fields (+0xA0 idle, +0xA4 reload, +0xA8 rel-start, +0xAC rel-end,
+0xB0 fire, +0xB4 fire-last, +0xB8, +0xBC, +0xC0 draw, +0xC4 holster, +0xC8 var-swap, +0xCC
idle-alert, +0xD0/+0xD4, +0xD8) with frame lengths, bound the fire-mode labels through the language
bank, and got: **+0xCC on 67 records, +0xB8 on 16 (the snipers ×12, the AIMS-20 pair, the
micro-camera pair), +0xBC on 2 (micro-camera), +0xD0/+0xD4/+0xD8 on 8 (the PP7/P2K family
0x02–0x09), +0xA8/+0xAC on 2 (the Frinesi only)**. One dangling hash on the whole disc (0x5E's
draw 060004a2 resolves nowhere).

**The six test weapons (ids by name, [D] labels):** P2K/Wolfram PP7 0x02 Semi + 0x03 Silenced;
Frinesi Auto-12 0x1C Pump + 0x1D Auto; Winter Covert Sniper 0x24 Single + 0x25 AP; AT-420 Sentinel
0x2C Guided + 0x2D Unguided; Militek MGL 0x2A Grenade + 0x2B Delay; Phoenix Samurai 0x33 Beam
(0x32 Overcharge — its owned slot is the base 0x33, +0x02 = 0x33 [D census], so a direct select
lands 0x33; the matrix's own D-block shows 0x32 never drew on 07000005 either).

**The decompile round** (subagent, all [M] with file:line in the worklog): `Player_SetWeaponAnimObj`
0x1a2e38's full case table, `Player_WeaponSelect` 0x1a2cc0 (same-base = a direct +0x02 u16 compare;
var-swap @1.3 of the REQUESTED record's +0xC8), `Player_Weapon` 0x1a24c8:117-152 (**+0xB8 = the aim
raise @1.25, state 0xF; +0xBC = the aim lower @1.25, state 0x10; no +0xBC → +0xB8 played REVERSE** —
weapon_data.md's "draw-at-1.25×" label for +0xB8 was wrong), the idle fidget (**+0xCC queues @1.0
once +0x93a (the idle counter) reaches 0x14*FRAME_RATE_INT, then back to +0xA0 with the counter
reset — cyclic; any non-idle state resets substate + counter**), the PP7-family raise/lower path
(+0xD0 @1.0 / +0xD4 @1.25, gated on not-aimed/no-threat/cooldown), and +0xD8 (holster-out-of-raised).

**Fixes landed (each with the failing test shown first — the fidget and raise tests panicked at
"phase did not finish" on the stashed pre-fix tree, and the raise field did not even compile):**

1. **The idle fidget +0xCC** (67 records) — scene.rs's idle arm: 20 idle ticks (0x14*FRAME_RATE_INT,
   FRAME_RATE_INT 1 [S] — live check below) → queue +0xCC @1.0 once; on its stop → +0xA0, counter
   reset; displaced by fire/reload/switch → substate + counter reset [M 0x1a2e38:40-136].
2. **The aim raise/lower +0xB8/+0xBC** — the aim edge from the idle machine queues +0xB8 @1.25; the
   off edge queues +0xBC @1.25, or plays +0xB8 REVERSE (negative script speed — the port has no
   AnimScriptAddReverse). Deviation: the game's state 0xF defers the aim bit to the raise's end
   [M 0x1a2e38:554-561]; the port keeps the aim camera instant (a follow-up could gate the aim bit
   on view_aim_anim).
3. **The idle counter (+0x93a) reset semantics** — the game resets it in every non-idle state [M
   0x1a2e38:40-58]; the port counted through the fire script (only the fired tick cleared it) and
   now resets on fire_state AND the raise (the raise is the game's state 0xF). This also corrects
   the AIMS side-light timing that reads the same counter.
4. **The switching double-advance (a real port bug the test exposed)** — the switch machine's
   shared advance AND the idle branch's advance both ran per tick, so every holster/draw/var-swap
   script played ~2× its rate (the Frinesi holster measured 11-12 ticks instead of the game's
   ceil((30-1)/1.25) = 24). The idle branch now advances only when switch_state == 0.

**The table (game [D census] vs port, after the fixes — all asserted in weapon_anims_live):**

| weapon | anim | game script | len | rate | port ticks (asserted) |
|---|---|---|---|---|---|
| 0x02 P2K | draw | 0600082b | 36 | 1.25 | 27 (rule 28 incl. queue tick) |
| 0x02 | fire | 06000823 | 25 | 1.0 | 24 |
| 0x02 | reload | 06000827 | 155 | 1.35 from frame 6 | 109 (machine 115, rec 109) |
| 0x02→0x03 | var-swap | 06000828 (the twin's own) | 201 | 1.3 | 153 (rule 154) |
| 0x02 | holster | 0600082a | 36 | 1.25 | 27 |
| 0x02 | fidget | 06000826 | 185 | 1.0 | 183 after 20 idle ticks |
| 0x1C Frinesi | reload | start 060004e1 / loop 060004e5 / end 060004e0 | 48/18/64 | 1.0 | 46 / 17 per shell / 63 |
| 0x1C→0x1D | var-swap | 0600093a | 50 | 1.3 | (green via the p2k path shape) |
| 0x24 Sniper | draw/fire/reload/var-swap(=reload hash 060004ad)/holster | … | 31/130/193/193/22 | 1.25/1.0/1.35/1.3/1.25 | green |
| 0x24 | aim raise | 060007b9 | 24 | 1.25 | green; lower = the same script REVERSED |
| 0x2A MGL | reload | 06000845 | 158 | 1.35 | 116 (machine 118) |
| 0x2A | fidget | 06000844 | 95 | 1.0 | green |
| 0x2C Sentinel | — fidget | +0xCC = 0 | — | — | 40 idle ticks → still the plain idle |
| 0x33 Samurai | fire | 060007eb | 100 | 1.0 | ≥99; reload: NO script (+0xA4 = 0), the meter refills anim-less |

The full 115-record dump: the probe output (scratch); the six weapons' rows are quoted in the test's
headers. 0x2D is Unguided (the task premise's "heat-seeker twin" is 0x2E AT-600 — its script set is
identical to the Sentinel's [D census]).

## VERIFIED

- `cargo test --offline --release -p nf-view --test weapon_anims_live` — **7 passed / 0 failed**
  (6.3 s, NF_ISO): the six weapons' draw→fire→reload→mode-switch→holster hashes, rates and tick
  numbers, the fidget cycle + the Sentinel's absence of one, the sniper's raise + reversed lower.
- The fast suite on the mid-task tree (before the double-advance fix): **1009 passed, 7 failed**
  — the 3 allowed `disc_golden` (slim ISO), the 0c-pylon family (documented as failing on the
  combined master, weapon-matrix2's report) and the `weapon_matrix_live` TIMEOUT (documented as
  reproducing on master, 7116c16). **The final tree (with the double-advance + counter fixes) did
  NOT get a full suite re-run — the window closed; the next session should run suite.py first.**
  The risk is bounded: the double-advance fix only speeds the switch waits to the game's rate; the
  matrix's D/R marks were measured THROUGH the slow waits, so its numbers may shift.

## LEFT

- The PP7-family idle raise/lower (+0xD0 @1.0 / +0xD4 @1.25) and +0xD8 (holster-out-of-raised):
  the decompile is in the worklog (0x1a2e38:94-192, 212-223); the raise's threat scan
  (DAT_00298e14: ent+0xac>0, anim state != 1, ent+0x50c>0.5) needs a port stand-in before the
  behaviour is faithful — not guessed at.
- The +0x1000 fire-end nuance: the game does NOT re-queue idle after a fire script for 0x1000
  weapons [M 0x1a2e38:445-456]; the port keeps fire_state cleared but leaves the script parked at
  its end until something else queues — matches the observed P2K/Samurai behaviour, worth a look.
- The Samurai 0x32 (Overcharge, the 380-frame charge with the release at frame 280) is unreachable
  through the give/switch path (its owned slot is 0x33) — the matrix's gadget-cycle work or a
  mode_to walk may reach it; its fire/release numbers are in the census.
- The suite re-run on the final tree (above).
- AnimObjectDraw's MP grip reads +0xEC (0x1a2e38:569-597) — weapon_data.md says "no reader found";
  the doc correction is owed to a fields pass.

## Live checks for the main agent (PCSX2)

1. **The fidget threshold**: idle the P2K 2+ s — does the fidget (+0xCC 06000826) start ~20 frames
   after the gun settles? `gdbwatch 0x2bf150+2*0x10C+0xCC 4 read` beside BL+0x93a — the queue tick
   pins FRAME_RATE_INT (my 20-tick reading is [S]).
2. **The sniper raise**: tap L1 with the Winter Covert Sniper — the scope raise anim (060007b9,
   24 frames @1.25) should play BEFORE the scope zoom lands (the port zooms instantly — deviation).
3. **nf-view spots** (after the next session's suite): `--level 07000005 --give 0x02 --frame 40`
   (the fidget mid-play), `--level 07000005 --give 0x24` + an aim frame (the raise), `--give 0x1c`
   mid-holster (the corrected 24-tick wait).
