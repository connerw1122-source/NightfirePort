# exact-sine-sites-merge — landing 110k + 110m (task 111b)

Branch `s/exact-sine-sites-merge` (from origin/master cf4f33cd). Session 15:02–16:4x UTC 2026-10-15.
The 2-hour window ended mid-task: the merge is landed and both held-branch symptoms are resolved or
root-caused, but the escort-walk fix itself needs the next session. Status: **blocked** (reason at the end).

## What's done

1. **The merge is landed.** `origin/s/exact-sine-sites-1` (110k) and `origin/s/exact-sine-sites-3` (110m)
   merged onto master; `docs/GAPS.md` G-046 is **one combined entry** (all three swap groups + the
   PORT_ONLY stand-ins, all four report paths); `docs/WORKLOG.md` unions kept for every conflicting entry.
   No game-behaviour code was touched beyond the two branches' own changes.
2. **The wire-icon failure is fixed from the game's math** (`fix_08_walls_live`
   `the_wire_icons_straddle_set_is_its_own_west_room_cel`, 4/4 green on the merged tree). The old
   assertion `icon.centre == icon.pos` (labelled "[D probe]") was in fact the **old host stand-in's
   artifact**: the host `yaw.sin_cos()` form with no C0 factors produced the placement origin bit-exactly.
   The game never does that — `Control_BuildWorldSph` 0x133070 transforms the object's local glist bounds
   centre through `View_RotTransMatrix` 0x1e5168 (C0 = PS2Sinf(1.5707964) = 1.0000001 [L probe] factoring
   every non-±0 element) with chop VU0 MAC adds, which accumulates ~1–2 ulps of the ~133-magnitude
   translation. The chain is [L]-verified against live `obj+0x80` on 07000005 and transcribed bit-for-bit
   in `exact_sine_sites_live` (docs/reports/exact-sine-sites-1.md), so the disc-derived game-chain value
   `[133.42567, 48.326435, -101.2065]` is the expectation now, with a sister that the move stays under
   1e-3 (the same sphere for gameplay; the straddle-set assertions below it are unchanged and green).
3. **The escort-walk bisect is done, with the mechanism** (details below): master PASSES, master+110k
   PASSES (and reaches level_exit), so **110m's four player-motion trig sites are the sole cause**; the
   merged tree with `NF_HOST_TRIG=1` reproduces master+110k **bit-identically** (same `level_exit` tick
   469242), proving nothing else in the merged tree moves this scenario.
4. **Replay tooling landed** (shared-file hooks, listed): `trig::host_trig_init_from_env()` +
   `NF_HOST_TRIG=1` read in `nf_view::load` — a whole-process fallback of the four 110m sites to the host
   stand-ins, so any test binary can be run through both models without a test-local switch.

## The escort-walk bisect (walkreal_07000001_live::walk_from_the_spawn_to_the_exit_with_the_contact)

| tree | result |
|---|---|
| master (cf4f33cd) | **PASS**, 81.4 s — the contact settles at (−28.3, 6.3, 14.3), dxz to the plate ≈ 4.9 < 6.0; the AND-touch does not fire (the known stairwell-head stall, docs/reports/walkreal-a.md) |
| master + 110k | **PASS**, 255.0 s — `level_exit at tick 469242` (the AND-touch fires; 110k's world-sphere/spread/spin exactness even improves the walk) |
| master + both (the merged tree, default exact) | **FAIL** — the driver's suite run: the contact settles at (−18.8, 6.3, 14.3), line-218 assert, 882 s alone / 600 s suite timeout; my partial run shows the 12-round hide/clear grind |
| merged tree + `NF_HOST_TRIG=1` (the four 110m sites back on host) | **PASS**, 253.6 s — `level_exit at tick 469242`, **bit-identical to master+110k** |

(The 110m report's claim that the walk already fails "on master at d38e945d with this task's changes
stashed" did not reproduce: master passes today, cleanly. Their stash run most plausibly left the
branch's `trig.rs` hooks in place — a stash of tracked modifications keeps untracked new test files and
any partially-staged state; either way, the direct master run above is decisive.)

**The first divergent tick and the decision it flips** (from the per-round logs, master vs the merged
tree, identical through the intro, the channel write, the guard-0/13 fights and the approach legs):

- `approach guard 19 nav 1`: master arrives at **+123 ticks** (fight ends clip 7 spare 0); the exact
  tree arrives at **+112 ticks** (fight ends clip 8 spare 0) — Bond's exact-motion path crosses the
  walker's arrive radius 11 ticks (0.18 s) earlier. The ulp-wide per-sine deltas amplify to ~0.5 units of
  path through the same floor-snap/wall-slide feedback the two prior exact-math merges documented
  (fix-exact-math-swap's t28/t835 mechanism; exact-sine-sites-3's 600-tick walk saw the identical
  amplification at the courtyard gate).
- The shifted fight timeline flips the contact's **AllyLead hide windows** (LeadHide 0x15ec08's
  armed-within-14 rule): master round 0 — no armed guard near him, he walks his route to (3.5, 6.3, 7.3);
  exact round 0 — he hides at (6.6, 6.3, 13.6), which re-arms guards [5, 6, 7, 11] for clearing.
- From there the walker's choreography — written and tuned against the host-math timeline — wedges: the
  ammo-resupply leg sticks on a door for its full 6000-tick cap, the 12 follow/clear rounds grind
  (~882 s), and the contact parks at (−18.8, 6.3, 14.3), ~14 units short of the plate → line 218 fails.

**Why this is the game's math, not a port bug:** every swapped site is disassembly-pinned
([M] in the three exact-sine reports); no unswapped host-trig stand-in remains on this path (the drone
walk/route code has no per-tick trig site; the remaining head-track acos/sin-cos sites are the survey's
PORT_ONLY stand-ins, G-046). The port at exact motion *is* the game's simulation as far as the port goes;
the fight timeline legitimately moves. What breaks is the **test harness's host-timeline choreography**
(guard-clear ordering, resupply walks, stuck mitigation), not a game-math stand-in.

## What's left (for the next session / the main agent)

The escort walk passing 2/2 under exact motion needs a decision that needs live evidence:

1. **Live checks for the main agent (PCSX2, retail disc, 07000001):**
   - Escort the contact as a player (clear the same three opening guards; do NOT one-tap guard 19 from
     the master-timeline spot) and watch the contact's state byte + route index through
     `NDrone2_DSTATE_AllyLead` 0x15e698 / `AllyLeadHide` 0x15ec08. Expected question answered: does the
     game's own contact reach the LoadPointActivator plate volume (centre (−33.1, 2.3, 15.4)) when the
     fight timeline matches the port's exact-math run (guard 19 dies 1 shot later)?
   - `gdbwatch` on the contact's `Drone+pos` (and guard 19's health) during the guard-19 window: the
     game's death tick for guard 19 tells us which timeline (clip 7 = host-like, clip 8 = exact) the real
     game plays — i.e. whether the exact outcome is the game's.
2. Then either **re-choreograph the walker for the exact timeline** (guard-clear order / resupply path /
   stuck handling — test-harness work, no game code) and show 2/2, or re-scope the line-218 assertion if
   the game itself stalls the contact there.
3. The full four-crate suite on the final tree (the window did not fit it; chunks below).

## Verification run on the merged tree (foreground chunks; the sandbox kills background jobs)

- `fix_08_walls_live` 4/4 (after the expectation fix) · `exact_sine_sites_live` 1/1 ·
  `exact_sine_sites3_live` 6/6 · `ai_live` 35/35 (110k's re-pinned seed holds on the merged tree) ·
  `trig_live` 4/4 · `diff_calls` 4/4 · `walkreal_07000002_live` 1/1 · `wire_live` 2/0/2i ·
  `fix_ladder_live` 3/0/1i · `fix_wire2_live` 3/0 · `fix_camera_exact_math` 1/1 · nf-game lib 175/175 ·
  `walkreal_07000005_live` 0/2i (parked, per walkreal-a) · `walkreal_07000001_live` 1/2 (the known escort
  failure; passes with `NF_HOST_TRIG=1`).
- Full suite: **not run** — the window; the chunks above cover both branches' own fail-first tests, the
  AI/bullet consumers and the movement walks. The next session must run the full suite on the final tree
  before merging (allowed failures on the slim ISO: the 3 `disc_golden`).

## Shared files touched (small, additive, per the rules)

- `docs/GAPS.md` — G-046 resolved to one combined entry (the merge requirement).
- `docs/WORKLOG.md` — conflict unions (all entries kept) + this task's entries.
- `crates/nf-game/src/trig.rs` — +11 lines: `pub fn host_trig_init_from_env()` (no behaviour change
  without `NF_HOST_TRIG=1`).
- `crates/nf-view/src/lib.rs` — +1 line in `load()` calling it.
- `crates/nf-view/tests/fix_08_walls_live.rs` — the wire-icon expectation (this task's fix).

## ntfy

(see the WORKLOG final entry; sent after this report is pushed)
