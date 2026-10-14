# fix-0c-hatch — 0700000c: the display vent hatch (task 101g)

Playtest results 1013-1758, 0700000c: after burning the four display-vent latches (locks out 14-17 at
(7.09/4.54, 15.70/16.87, −112.35)) "the hatch actually opened very slowly, but climbing through and
dropping down, he still can't get through because he can't crouch through". Branch `s/fix-0c-hatch`
(the task footer said `s/0c-hatch`; the header's `s/fix-0c-hatch` is used — no remote branch existed
under either name when I started).

## Done: the hatch opens at the game's speed (symptom 1) — FIXED

**Cause [M].** The port's SimpleScript playhead advanced `1/rate` key-units per tick
(`SsList::tick`, `crates/nf-game/src/ss.rs`), which made a `keys−1`-key sweep take `rate × (keys−1)`
ticks. The game computes the per-tick step **once at create** as `(key count − 1) / rate` —
`SS_Create` 0x1a9c58:51 (`puVar2[1] = (float)last / (float)rate`, stored at ext+0x04) — and
`SS_Update` 0x1a9948:103-108 advances `t += step * FRAME_RATE_MUL` (2-key scripts add `step` alone).
`GS_SetRefreshRate` 0x17d2b8:15 sets `FRAME_RATE_MUL = 60.0 / FRAME_RATE` = **1.0 at the game's
60 Hz** (the port's fixed tick), so the rate word is the **whole sweep's duration in ticks**.

**Evidence [D].** The 0700000c records: `ventmesh-front` #138 — `extra[7] = 0x00320006` (6 keys,
rate 50), ch 18, param 0 = 64 (the 0x40 armed state), sound 470, spline; `VentBlockingPoly` #179 —
2 keys, rate 10, ch 18; warehouse `ventmesh` #180 — 6 keys, rate 50, ch 84. The hatch therefore
opens in **50 ticks (0.83 s)**, not 250 (4.2 s) — five times slow, exactly the playtest report.
The 2-key SSs (#179 and 07000012's screens, rate 1) are untouched by the fix: with `last == 1` the
old and new step coincide, so those tests were never at risk.

**Fix.** `SsList::tick` (`crates/nf-game/src/ss.rs`): `let step = last / s.rate as f32;` plus the
module-doc correction (the rate paragraph was marked `[S]` — now verified `[M]`).

**Tests** (`crates/nf-view/tests/fix_0c_hatch_live.rs`, NF_ISO-gated, shown failing first):
- `the_hatch_sweep_takes_the_game_rate_ticks` — burns the four hinges through the sim, counts the
  ticks to each last key: **250 → 51** (the mux tick + 50). Sister assertions: #179 sweeps in ~10
  and both playheads STAY at their last keys while ch 18 holds.
- `the_warehouse_ventmesh_sweep_takes_the_game_rate_ticks` — the VentLatch burn → ch 84 → #180:
  **251 → 51**.
- `fix_level_0c_live.rs` (fix-level-0c's suite) still passes — the endpoints are unchanged.

## The traversal (symptom 2) — mapped, two legs verified, two legs open

I reproduced the traversal symptom headlessly and mapped the whole area with collision probes
(`floor_below` / `segment_hit` grids over the sim's world), then checked the game side. What the
level actually is [D placements + probes]:

- The vent opening in the wall plane z ≈ −112.25: x 4.54..7.02, y 15.05..17.50 (the closed hatch
  #138 fills it; the box y 15.05..17.50 matches the latch y's 15.70/16.87).
- After the open: the hatch's top edge lands at y ≈ 15.2-15.9 (probe: rays block below ~15.2, pass
  15.4..17.0) — a ~2.2-tall clear band; the blocker #179 rises to 20.0.
- The duct: a raised ramp (x 4.5..7.0, z −110.5..−112) rising 17.22 → 17.50 to the wall, with a
  wall face at z −110.02 spanning y 13.5..~17.1 across the walkway. The walkway ramp below runs
  12.25 → 14.01.
- Behind the wall: an inner ledge at 14.72-14.81 (z −112.5..−115.5+), a mid floor at 11.27 below
  it, and the display case floor ~6.3-7.3 at the bottom.
- **There is no creepwall in 0700000c at all** [D `nf-extract kind-census`: no 0x3F row], and no
  ladder near the vent (the level's only ladders are #164/#495 at z ≈ −177..−187) — so the task's
  "creep/vent volume" option is out, and the game's climb substate is entered **only** by
  ladder-object contact [M `Player_CollLadder` 0x196118 ← `Player_DealWithObjHit` class 0x2D;
  `Player_Update` 0x193600 case 1 dispatch].

**Verified working in the port's physics** (headless, real `step_bond` input, hatch open):
Bond placed on the duct ramp (the game's `--bond` spot below), crouched, walked forward off the
edge — the capsule drops **through the open hole** and lands on the inner ledge (root ~15.3 over
the 14.72 floor) and walks on. So the crouched capsule (top = feet + 1.20 [M `Player_Collision`
0x194df0]) fits the open band fine — the "can't crouch through" is not a capsule-vs-gap fit.

**Open leg 1 — the ascent.** From the walkway (14.01) the duct top (17.2-17.5) is 3.2 up across a
vertical face; no ladder/creepwall serves it, so the real game must reach it another way (a jump
route from the walkway's side ledges at 15.2? a start from a higher floor? the Q-hint "the ways in
and out of this building are on higher floors" points up). I did not find the game's ascent and did
not guess a fix.
**Open leg 2 — the descent into the display case.** From the inner ledge (14.72) the mid floor
(11.27) and the case floor (~7.3 at (5.4, −113)) lie below; I mapped the floors but not the hole
the game drops Bond through before the 03:47 cut.

The two traversal tests are in the test file **`#[ignore]`d with these reasons** (a red suite would
block everyone; the work resumes from them). No `player.rs` change was made — the only code change
is the one-line SS rate fix.

## Files

- `crates/nf-game/src/ss.rs` — the playhead step fix + the doc correction (the only code change).
- `crates/nf-view/tests/fix_0c_hatch_live.rs` — new: 2 passing sweep tests + 2 ignored traversal
  tests with the reasons.
- `docs/levels/0700000c.md` §6 — the rate fix + the traversal state noted in the vent row.
- `docs/WORKLOG.md` — my entry appended (and re-appended at the end through the merge conflict).
- Shared files: none touched (`lock.rs`, `props.rs`, `player.rs`, `drone.rs`, `scene.rs`, `lib.rs`
  are unmodified).

## Suite

Run on the final tree (merged with origin/master 810c109): **905 passed, 3 failed, 29 ignored
(18 min)** — the 3 failures are exactly the allowed `disc_golden` trio (bigf_header_fields,
bigf_tables, directory_tree); no TIMEOUT lines; the 29 ignored include this task's 2
traversal tests. The baseline run before the fix (master + tests) was 885 passed / 3 failed / 27
ignored in 17 min.

## Live checks for the main agent

PCSX2 (0700000c, the display area — Bond teleported to the walkway at (5.8, ~14.7, −108.5), all
four hinges burnt):
1. **The hatch speed** — burn the four hinges with the laser watch and watch `ventmesh-front`
   (#138): expected — the panel slides down smoothly in about **0.8 s** (50 ticks), not ~4 s.
2. **The ascent** — from the walkway in front of the display (spot (5.8, 14.0, −109.5), facing
   −z): what does the game do at the wall face z −110.02? Watch for a climb/jump-grab onto the
   duct ramp (top 17.2-17.5) or a route around (the side ledges at 15.2, x < 4.5 / x > 7.0).
   `gdbwatch.py` on Bond's substate word (BLData+0xF6/obj+0xF6) while pushing forward would tell
   whether substate 1 (climb) ever engages here.
3. **The drop through the open hole** — Bond on the duct ramp at (5.8, ~17.5, −111.5) crouched,
   hold forward: expected — he drops through the hole onto the inner ledge (14.72), then the route
   down into the display case (the mid floor 11.27 → the case ~7.3 at (5.4, −113)): where does the
   game's hole sit on the ledge?

nf-view (`--bond x,y,z,yaw,pitch`, after burning — or with the SSs at their open keys):
- `--bond 5.8,15.8,-108.5,3.14,0` — the walkway before the display: the closed hatch covers the
  opening; after burning, the hatch open at the new speed and the blocker risen.
- `--bond 5.8,18.4,-110.8,3.14,0` — on the duct ramp: the open hole below-front, the drop through
  lands on the inner ledge (verified headless; the render side should show the hatch panel slid
  down INTO the wall slot — the framelist POSED RENDER is still render-side open work, so the
  hatch will not be visible moving, only the collision behaves).

## What's left (for the next message)

1. Identify the game's ascent to the duct ramp (the live checks above; then port it — likely
   `player.rs`).
2. Map/port the descent from the inner ledge into the display case to (5.4, −113, ~7.3), un-ignore
   the two traversal tests, and drive them green through the real input path.
3. The warehouse vent #180/#184's own traversal (the same rate fix applies; its gap is the
   `ventmesh` hole at (53.8, 5.5, −163.6)) — untouched so far.
