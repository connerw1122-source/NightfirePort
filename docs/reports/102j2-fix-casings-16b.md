# 102j2-fix-casings-16b — the straddle walk-in narrowed; door_e restored; the escort walk is the open item

Branch `s/102j2-fix-casings-16b` = `s/fix-casings-16` (97632680) merged with origin/master (b9ed8b8a at 18:31 UTC; master confirmed unchanged at the final pull — the tree is the merged final tree). Commits: d6848c68 (the narrowing), e999b1d3 (the site gate).

## What the merge HELD, and the root cause

`s/fix-casings-16`'s per-tick `straddle_walkin` (door.rs) + its drone.rs hook broke two master tests on 07000001. Reproduced first (`level_end_live::door_e_opens_then_e_closes_on_07000001` FAILED on the merged tree: after the second E the state read **0x4**, not 0x10):

- **The state gate let a CLOSING leaf through.** The walk-in skipped only `state & 0xc != 0` (OPEN 4 / OPENING 8). A CLOSING (0x10) straddle door passed the gate, the walk-in re-`activate`d it every tick, and the port's slide activate turns 0x10 into OPENING [M Door_Activate 0x135be0: `state & 0x12` → SetState(8)] — a leaf barely off its open end snapped straight back to OPEN. The game's per-tick door scan NEVER does this: both scan arms sit behind an OPEN-door test — `(state & 2) == 0` inside `(state & 6) != 0` [M 0x1363c8 @0x136678, the ch_open-went-off auto-close] and `uVar7 & 4` [M 0x1363c8 @0x136610, the open_timer arm] — and its only action is one `Door_Activate` to close. The scan never touches a moving leaf.
- **The "straddle" param is not a gameplay trigger.** obj+0xfd bit 3 (par 4, `Door_Create` 0x136100) is consumed by exactly one reader: `Door_Link2Portal` 0x136788 → `build_LinkDoors2Portals` 0x1c8ac8 — a portal/culling link, not an open-on-walk-in flag. The decompile has NO per-tick open for a closed idle door at all: the opens are E [M Player_Activate 0x1a5170 → Door_Activate(obj, player, 1), action-button-gated — verified in Player_WeaponFiring 0x1a5460:393-402], the ch_open channel arms, the +0x68 one-shot edge, and the drone AI [M NDrone2_OpenDoor 0x14aa38]. The contact open fix-casings-16 stood in for is the unrebuilt U-041 collide path.

## The fix (commits d6848c68, e999b1d3)

`crates/nf-game/src/door.rs` — `straddle_walkin` narrowed to the game's scan domain:
1. `state != CLOSED → skip` — the stand-in is the scan's open-side complement: a CLOSED IDLE door only, never a moving leaf (fixes the door_e reopen).
2. `ch_open != 0 → skip` — a ch_open door is `Door_Update`'s own channel arm's job [M 0x1363c8]; the stand-in covers only the channel-less family (16's arrival door, ch_open 0 [M 0x136100 + glist 0x803f00a0]).
3. Silent lock pre-check — `locked_flag != 0 || (ch_lock != 0 && channel off) → skip` before `activate`: the per-tick scan plays no lock sound (the ch_lock thud is `Door_Activate`'s E-path arm [M 0x135be0]); the old path re-thudded every tick.
The proximity metric is unchanged (the game's own `Vec_SqDist3D(door+0x80, actor+0x80) < (door+0x8C + actor+0x8C)² · 2` [M 0x136678-0x136684]).

`crates/nf-game/src/drone.rs` — the hook is now scoped to the site the game evidences: `if d.level == 0x0700_0016`. Everything else in the call (actor list, sounds dispatch) is unchanged. Shared-file note: drone.rs keeps exactly the one additive block fix-casings-16 added, now level-gated; no other hooks touched.

Why the site gate: on 01, ANY proximity-open deviates from master. With the narrowed gate still active on 01 the escort walk failed (`walkreal_07000001_live`, door-state log captured over the full run: door #1 open 132k/520k ticks, door #2 36k, the plate doors #3/#4 never open; the contact pinned at (4.4, 18.4) by an armed guard 3.3 away whose exposure never cleared — Bond dry at clip 1 spare 0). With the stand-in at master behavior (no walk-in on 01) it **still** failed (560 s run) — the morning bisect's "master passes" no longer holds on today's master (fix-06-driver and others merged into it all day), or the walk is timing-sensitive near the 600 s binary limit. What IS established on this tree: the broad walk-in passes the escort but breaks door_e; no walk-in passes door_e but the escort fails on today's master+16-merge tree. The two HELD tests are in tension through 01's guard/escort door dance, and the real resolver is the U-041 collide path (per-leaf contact), not a finer sphere. Task A-11b already removed a per-tick `react_to_hit` call once for breaking closes — the anti-crush needs the proxy-list contact, not a sphere.

## Tests

- `level_end_live` **6/6**: `door_e_opens_then_e_closes_on_07000001` (the HELD test — PASSES: 0x8 → 0x4 → 0x10 after the second E) + the new `the_walkin_leaves_a_closing_straddle_door_alone_on_07000016` (shown FAILING pre-fix: `reopened_early=Some((0, 8))` — instant reopen; after the fix: saw_closing ✓, saw_closed ✓, no early reopen, the walk-in re-opens only AFTER the close completed — sister). The test lives on 16's arrival door (glist 0x803f00a0) because the walk-in is now 16-scoped.
- `casings16_live` **2/2** (16's arrival door still opens on a no-E walk-at; the locked sister stays shut).
- `nf-game --lib` **150/0** (the door unit tests included).
- `fix_npc_doors_live` 1 ignored (pre-existing harness note, unchanged).
- `walkreal_07000001_live` **FAILED** on the final tree (see above — the open item; 236 s and 560 s runs, both at the contact-plate assert).
- The full `tools/test/suite.py` could NOT run to completion: this sandbox kills background jobs between tool calls (nohup/setsid both died within seconds, suite.out empty), and the synchronous window left ~40 min after the two 4-minute escort runs and three rebuild cycles. What ran instead, synchronously, on the final tree: the binaries above. The next session should run the full suite (the walkreal_* family especially) on this branch.

## Left / open

1. **`walkreal_07000001_live` — the contact must reach the LoadPointActivator plate.** Passes with the broad walk-in, fails without it on today's tree. Next session: verify it on plain origin/master FIRST (the bisect's premise may be stale — master moved: fix-06-driver, fix-guard-weapons2, fix-oddjob-hat, s/104a2-fix-escort-01b all landed today; 104a2 is another worker on exactly this test). Then rebuild U-041 per-leaf contact (the real fix): a door's collide proxy (obj+0xd0, the leaf's live pose) — `Door_ReactToHit` 0x135980's own list — replaces every sphere stand-in.
2. The full suite on the final tree (totals line for the record).

## Live checks for the main agent

PCSX2, 07000016 (the walk-in premise — fix-casings-16's check 2, still unverified):
1. The first door past the arrival elevator: walk at it with NO action press — expect it to slide open on contact. Spot (port coords): `--bond 5.17,0.9,-65.3,3.09,0` walking forward.
2. Same door opened, then step back and let it close, then stand in the doorway: does the game re-open it on contact (the anti-crush / U-041)?
PCSX2, 07000001 (the escort — the open item):
3. From the spawn, follow the contact west without firing: does he walk his route to the stairwell/plate area untouched? Note any door he opens himself (DSTATE_OpenDoor) vs doors that open on his approach — that distinguishes the AI path from U-041.
4. The interior shoji doors (port coords ≈ (7.9, 5.4, 2.9) and (6.99, 5.4, 11.1), placements 13/14): E-only, or do they open when walked into with no E?
nf-view: `--bond 7.887,5.354,2.372,0,0` (01, door #0 threshold) — after this branch, E opens/closes it and the close completes (no walk-in on 01); `--bond 5.17,0.9,-65.3,3.09,0` (16) — the arrival door opens with no E.
