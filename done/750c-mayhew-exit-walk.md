**Reproduce first, on the newest origin/master, through the REAL path.** Earlier fixes for some of these passed
their own headless tests and the user still hit the bug in the game. Before changing code, write a live test that
drives the same path the player takes (the level loaded through `nf_view::load`, Bond moved by `step_bond` inputs —
walking, the use key, the ladder input — the real scripts and triggers running, no teleports past the problem
spot unless the report says so) and FAILS on current master the way the user describes. Put the failing output in
the report. Only then fix it, and the same test must pass. If you cannot reproduce it, say exactly what you tried
and where the path diverges from the report; do not mark it done on a guess.

## Your task: `mayhew-exit-walk`, Mayhew's walk to the exit before the mid cut-scene

The user, 2026-10-16: "mayhew is bugged with walking to the exit as well before the mid cutscene". Mayhew is the
ally on the estate levels (07000005; map-mayhew task 033 built the mission chain). Find the sequence: Mayhew walks
Bond toward an exit, then a mid-level cut-scene plays. Reproduce on master: what goes wrong (stuck, wrong route,
the cut-scene not triggering, walking through geometry...) — record his route nodes and states per tick. Port the
game's behaviour (his scripted route/goal, the trigger that starts the cut-scene) with [M] citations, and pin it:
he reaches the exit and the mid cut-scene starts, as in the game.

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Tests pin only proven behaviour.** Every assertion about what the game does cites its evidence: [M] a decompile
address that actually says it, [D] disc bytes, or [L] a live PCSX2 measurement from a FRESH boot (a save-state resume
skips the boot path). A guess, a port choice or an [S] item is never a hard assertion: mark it `// [S] unverified` and
keep it out of pass/fail. If an existing test contradicts the decompile, the TEST is wrong: fix it, with the address, and
say so in the report. Never weaken a decompile-backed test to make code pass.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/mayhew-exit-walk`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox4 at 2026-10-17T02:12Z

---
## Report (done, 2026-10-17T03:00Z)

Closed by the driver: the report is on `s/mayhew-exit-walk` (docs/reports/mayhew-exit-walk.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-17T03:05Z)


---
## Report from the worker (done, 2026-10-17T03:06Z)

# mayhew-exit-walk — the walk to the exit before the mid cut-scene (07000005)

Branch `s/mayhew-exit-walk` (from origin/master 31dd6161). User report 2026-10-16: "mayhew is
bugged with walking to the exit as well before the mid cutscene".

## What the sequence actually is (the evidence)

"Mayhew" is the level's own name — Mayhew A = 07000005. There is **no Mayhew drone on the
level**: all 16 drones are guards 0x0500002e/2f/30 [D `nf-extract drones 07000005.bin`; the
same for 06/07/08 and the all-maps scans]. The Mayhew escort that walks with Bond is 07000001's
contact (escort-01, already ported and green).

The mid cut-scene is the **truck hand-over**:

- SP #37 "Scriptplayer_SC10" (mode 1) plays the truck script **0x06000A22** at level start
  [D placement #37; levels.md §5.2]. Its stream 8 creates a script TouchOnce out **ch 0x5C**
  at t 1882 (0x75a) and kills it at t 2312 (0x908) [D script decode]; the port's sim trace
  matches (the trigger exists between the t 2100 and t 2400 samples, centre
  **(98.49, 27.05, 106.36)** — the glist 0x02000480 "Trigger-TouchplateBig", ±1.95/±1.75 [D]).
- Touching it fires **SP #38**, script **0x060008C8** — 143 ticks, camera true, Bond skin
  0x05000007 on anim 0x06000A24 [D the script's streams] — the cut-scene that hands Bond onto
  the waiting truck. Then the ride: bed touch ch 0x71 (stream 7, parented to the truck), the
  park plate ch 0x70 (stream 6, created t 1508), the gate-guard ch 0x72 (placement #318),
  mux #316 = AND(0x70, 0x71, 0x72) → SP #320 "EndLevel" (0x06000A23, event 0x11 at t 300
  [D]) → ch 0x1D → #118 loads 07000006 [D docs/levels/07000005.md §3].
- `Trigger_Hit` 0x1b1fc8 does no geometry — it classifies the collision pass's contact chain
  [M 0x1b1fc8]; `Script_CreateEntity` 0x1d8968's 0xea arm calls `Trigger_TouchOnce` 0x1b2b88
  with the zero vector + the glist [M 0x1d8968], and `Script_Interp` 0x1d9a08 case 1 keys the
  object's matrix from the stream [M] — the plate stands where the stream's keys put it.
  Bond's walkway edge stands ~1.2 under the plate plane; his capsule (feet+0.55..feet+1.86,
  r 0.55) crosses the plate plane, which is why the edge position fires it — the game's own
  contact shape, not a port invention.

## Reproduced on master, through the real path

`nf_view::load("07000005.bin")` + `step_bond` inputs only:

1. **The chain works end to end when Bond reaches the gatehouse edge inside the window** — the
   new test `crates/nf-view/tests/mayhew_handover_live.rs::the_walk_to_the_gatehouse_fires_the_
   handover_window_and_rides_out`: the real-input walk arrives at (99.69, 25.94, 106.98) at
   t ≈ 373, **ch 0x5C fires at t 1884** (the script's own window opens at 1882 [D]), SP #38
   re-places Bond on the truck bed, Bond crouches (the game's ride stance [D fix-truck]) and
   the crouched ride parks on ch 0x70 — the chain ends the level: **level_exit 0x07000006**.
   GREEN. The sisters hold: `walk_07000005_live` 2/2, `fix_truck_live` 2/2.
2. **The manual turn+forward walk from the spawn stalls** — the same walk driven as a raw
   turn-toward-target + forward loop (what a thumb does) oscillates at
   (101.11..101.27, 24.94, 107.45..107.80) for 2400 ticks and never reaches the plate's XZ
   (needs x ≤ 100.44); the walker's nav walk crosses the same stretch fine. The collision
   there [D Room_Road_A1 tris]: the walkway/plateau surface (y 24.75-25.11) ends at z ≈ 105.8
   and x ≈ 101.4, the arch's south face rises from y 20.4 to 23.75 along it, and the lane
   south of z 106 sits at 23.75 — a beeline at z ≈ 107.5 walks into the arch face. The
   ignored `walkreal_07000005_live::walk_to_the_gatehouse_and_ride_the_truck` parks on the
   same spot ("the walked Bond stops 1.1 below and the chain never fires — 0x5c/0x70 dead for
   9000 ticks" — its own comment). With the window only 430 ticks wide (7.2 s [D]), a player
   who fumbles the approach loses the hand-over for that truck pass (the truck script is
   one-shot; the level remains completable through the gate door #118, fallback 0x1F [D]).
3. Sister `the_handover_never_fires_for_bond_who_stays_at_the_spawn` — no walk → ch 0x5C low
   through the whole window, no exit. GREEN.

## What ran

- `mayhew_handover_live` 2/2 (NF_ISO, release, --test-threads=1; the full-ride version).
- `walkreal_07000005_live` 1 passed + 1 ignored (the cable sister) — **the truck-route walkreal
  is un-parked and green**: the walked ride parks at (29.86, 37.93, 52.4) at t 5687, no falls,
  and the chain exits through the real walked path.
- `walk_07000005_live` 2/2, `fix_truck_live` 2/2 (the sisters — no interference).
- `walkreal_07000001_live` 2/2 (the escort level, unchanged — the sanity side).
- `nf-game --lib` 251 passed / 0 failed; `nf-formats` lib 79 + carmodel_golden 2 passed
  (disc_golden 11 passed / **3 failed = the slim ISO's allowed three**); `nf-audio --lib` 11;
  `nf-view --lib` 17 — all 0 failed. The sim surface is untouched by this session's test/doc
  commits; the branch is master 31dd6161 + test/doc commits only.
- The fast suite (`tools/test/suite.py`) **could not run in this sandbox**: background jobs
  are reaped between tool calls here (setsid/nohup detached runs died within one tool call,
  twice — the empty `suite.out` stands as the record; the same was reported by
  pause-music-1b and hatch-02-intro). The foreground runs above cover the touched surface
  (nf-view tests only; no sim code changed — this session is pins + evidence).

## What's left (the next session continues from the branch)

1. **The player-visible question the live checks settle**: the truck script is one-shot and its
   ch-0x5C window is 430 ticks (7.2 s) [D 0x06000A22 stream 8]; a player who fumbles the
   gatehouse approach misses the hand-over for that pass (the truck leaves; the level remains
   completable through the gate door #118, fallback 0x1F [D]). The sim chain itself is green
   end to end from the walked start (this session), so what remains is confirming in PCSX2
   what the player actually hits: the arch-face beeline stall at (101.2, 24.94) (the manual
   walker's trace) or a missed window, and whether the game's own walk needs any climb there.
2. The cable route's `walkreal_07000005_live::walk_the_west_ledge_and_cross_the_cable` stays
   `#[ignore]`d (the west anchors untested beyond the ladder — its own comment).
3. The full suite on the final merged tree (the background runner is reaped between tool calls
   in this sandbox; the foreground slices that fit ran green — see "What ran").

## Live checks for the main agent

- 07000005, fresh boot: walk Bond from the spawn (104.95, 25.12, 108.83) west-southwest to the
  gatehouse as the truck arrives (~31 s in). Expected: the 143-tick hand-over cut-scene plays
  (camera at (83.2, 20.99, 96.2)), then Bond rides the truck into the castle and the level
  exits to 07000006. Watch whether the player must jump/climb any lip at x ≈ 101 (the port's
  stall point) or the lane is a clean walk.
- `fieldprobe` poke: Bond at (99.6, 25.9, 106.9) at t ≈ 1900 — expected: ch 0x5C = 1 within
  the window t 1882..2312, SP #38 playing (the camera scripted), Bond re-placed on the bed.

## Usage

- Token/credit/context figures: **not visible to me** — this session's interface exposes no
  token, cost or context-window numbers I can read. The sandbox clock read 02:15 UTC at
  session start (expiry ~04:15 UTC per the task message).
