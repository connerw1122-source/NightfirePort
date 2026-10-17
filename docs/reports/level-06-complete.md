# level-06-complete — 07000006 (Mayhew B: the castle interior to the party)

Branch `s/level-06-complete` (from origin/master 0603094e). Sandbox window: this message's ~2 h
(14:43–16:42 UTC); the report written 15:57 UTC.

## What's done

1. **`crates/nf-view/tests/walkreal_07000006_live.rs`** — the level's walkreal (the file did not
   exist; only the teleporting `walk_07000006_live.rs` was on master). It loads 07000006 through
   `nf_view::load_with_carry` with the truck route's carried gates `[0x1d, 0x45, 0x70]`, asserts
   the ch-0x1D start #496 (57.18, 39.73, 22.61) [M Player_Start 0x190fd0], and walks the level
   with `Sim::step_bond` inputs only — no teleports after the spawn. Two tests, both `#[ignore]`
   with the exact blockers in their headers (see "Open" below).

2. **The walked route (every leg proven by an actual run; anchors [D census] + the walked
   probes):**
   - **THE DISMOUNT** — the truck-ride spawn stands on the WineTruck's bed/dock (Bond ~1.1 above
     the ground; [D #336 WineTruck @(64.48, 38.00, 28.06)]; probe: east/south walk free, west/
     north pin on the bed rails at z≈20.5 and the cab at x≈55.5). The walk goes off the bed's
     open south side — the player's own dismount.
   - The driver-greeting plates (#756/#757/#782 → ch 0xB9) and the arrival plates' OR #498 →
     **ch 0x46** (the objective reveal) fire from the walk itself.
   - The dock's west end → the west lane → around the well's west rim (the direct line to the
     well #65 pins at z≈20.5, x≈31 — the well's north fence [D floor grid]) → the garden hedge
     gap (the hedge band at z≈8.8 and the hedge at x≈22.6, z 4..8 wedge nav-assisted legs; the
     walked lane crosses the band at x≈24.4 and turns at the hedge's south end) → the guardhouse
     courtyard.
   - **E-door #47 Door_Top_I1** (36.50, 40.00, −16.25) opened with the player's use key through
     `use_door`; into the guardroom (the nav graph covers this room [D leg probe: 4 nodes]).
   - **The FuseBox #747 Laser cut** (in the cut variant): the Laser (0x4E) drawn with the real
     weapon-cycle input, `shoot_point` burns the group-0 bundle → **ch 57 + ch 103** [M
     FuseBox_Update 0x1adfe8:110-115] — the same chain `fusebox_06_live` pins (4/4 green).
     MEASURED IN THE WALK: the cut also raises **ch 55** (the alarm) — the beam clips the alarm
     bundle (group 1, p2 = 55) on the way in; the game's stealth cut is not silent.
   - **E-door #59 Door_Top_I2** (27.88, 40.00, −23.00, auto-close) opened; the searchlight
     courtyard mouth (the entry plates #525/#526 → 0x52 stage SP #808's boot NIS, skipped with
     the game's own skip action [M SP_Update 0x1dbda8:129 → Script_FFwd 0x1dac78]).
   - **E-door #71 Door_Tunnel_G9** (3.75, 42.00, −51.13) opened; the tunnel climb y 42 → 49;
     **E-door #72 Door_Tunnel_G9_2** (4.00, 49.00, −64.88) opened; the upper gallery (y≈50.1)
     reached through the doorway spot (trigger #521 (4.62, 49.80, −66.65) [D census]).

3. **The objective list [L ee_start_05.bin record 1; ported in `c07000006.rs`]:** the level has
   EXACTLY ONE objective — 0x04000001 "Find a way inside to the party" (hint 0x0400005c),
   complete ch 0x02, revealed by ch 0x46, no fail objective (fail 0xffffffff; the only standing
   fail hooks are death → ch 0x62 and killed civilians → ch 99 [M 0x1827d8:98-107]). It completes
   ONLY through the dressing cut-scene 0x06000071's closing event 0xD [2,1] → ch 0x02 [M
   Script_EventHandler 0x1d94c0 case 0xD] → Mission_MonitorObjectives state 3→4 [M 0x1827d8:78-85].
   The optional-objective list is empty; the level's optional content (the fusebox cut, the
   armour #754/#755, the 007 tokens #750/#751) is covered by the interactables task (706a, on
   master).

4. **The exit [D census §3; M 0x1d94c0 case 0x12]:** the level has NO 0xE8 LOAD_LEVEL. The only
   exit is the walk-in LoadPoint #746 (0xEB, out ch 0x83) → SP #810 → the dressing scene →
   event 0x12 [0x07000007] = `Player_RamSave` + `ResetMap_LevelToLoad(0x07000007)`. That chain is
   pinned GREEN on master through the real trigger by `walk_07000006_live.rs` (2/2 green here:
   ch 0x83 → `mission.states[1][0] == 4` → `level_exit == Some(0x0700_0007)`, plus the
   away-from-the-volume sister). My walkreal reaches the gallery one room short of it (below).

## What was verified in this window

- `walkreal_07000006_live` (2 tests, ignored with diagnosis) — compiles, runs green as ignored.
- `walk_07000006_live` 2/2, `interact_07000006_live` 9/9, `fusebox_06_live` 4/4 — the touched
  crate's (nf-view) 07000006 binaries all green on this branch's tree.
- **The full four-crate suite did NOT complete in this window**: the background runner
  (`nohup`/`setsid python3 tools/test/suite.py`) died twice between tool calls with empty output
  (background jobs do not survive here), and the foreground time went to the route work. The
  next session must run the full suite on this branch before merge.

## Open (for the next message)

1. **The guardroom wedge (the walk's hard stop).** The scripted idle guard #438 (skin 0x500002e,
   drone 2, script 0x0600021F [D census §4]) engages at (36.0, 41.0, −18.9) inside the guardroom
   and never dies: 8 real-fire `shoot()` engagements (the carried P2K, 34 and 240 rounds both
   tried) leave it alive with its position barely moving — the idle-scripted guards may be
   invulnerable, or the walker's aim never reaches a live hit box. Its body wedges the walked
   lane to the courtyard. Next: read the guard's shootability in the decompile (Drone_Damage
   0x18c330 vs the idle-script arm; does the game mark mission NPCs unshootable?), and check
   whether the walked route even needs the guardroom (door #47's courtyard connects around).
2. **The gallery → dressing-room leg (the walk's one route gap).** The port's collision gives no
   walked connection from the upper gallery (y≈50.1, z −65..−70) to the dressing room (y 42.5–47,
   x 31.5–35.5, z −48.5..−63.5): the gallery's north lip at z≈−65.5 (x 20–30) blocks walking AND
   jumping [D probe: grounded at 50.4 across 400 ticks with jump pulses]; the east corridor
   (x 38–39.5, y 50.1) is walled at BOTH ends (z −31.5 north [D probe], the room side west); no
   door placement sits between [D the 12-door dump: #46 Locked_Door_Top_I (36.25, 51.06, −68.38)
   is ON the gallery and locks ch 200 [M U-044]]. The game's F3 route must use something the
   port's floors don't model yet — a stair volume the floor rays miss, a scripted carry, or the
   AI patrol graph (AI point 5 at (2.33, 54.67, −52.50) patrols a 9-node path ABOVE the tunnel
   [D ai dump]; AI point 0's 30-node path covers the tunnel/courtyard).
3. The full suite on the final tree (the runner must be run in foreground chunks here).

## Live checks for the main agent

1. At the guardroom (F3, fresh boot): shoot guard #438 (the idle scripted guard inside the
   guardroom near (27.5, 39, −10.3)→(36, 41, −18.9)). Does he take damage? Watch his health and
   whether the game flags him unshootable (the muzzle-blood/no-hit difference). Expected [S]:
   either he dies normally (then the walker's aim is the bug) or he is scripted-invincible (then
   the walk must path around, not fight).
2. The gallery → dressing room (F3): after E-opening Door_Tunnel_G9_2 and stepping into the
   gallery, how does the game's own route reach the dressing room's LoadPoint (35.5, 44.5, −54)?
   Walk the gallery east/north with the debugger position overlay and record where the floor
   descends (a stair volume the port's floor rays miss is the prime suspect — AI point 5 patrols
   at y 54.7 above the tunnel). The exact stair/floor address + a `--bond` sweep would settle it.
3. Render check (for the skin-swap gap already on record): `--bond 35.5,44.5,-54.1,2.2,0` inside
   the dressing room during the dressing scene — the F3 tux skin 0x050000D swap (Bond's model
   vanishes mid-scene in the port; [M Player_Init 0x1910f8:96-107], flagged unported).

## Usage

not visible to me (no token/cost/context figures are exposed in this environment; the IM gateway
metadata shows only session/chat/channel ids).

## How to run

```
NF_ISO=<abs path to Nightfire-onfoot.iso> cargo test --offline --release -p nf-view \
  --test walkreal_07000006_live -- --test-threads=1 --nocapture   # the ignored walk (with RUST_LOG-style eprintln routes)
NF_ISO=$NF_ISO cargo test --offline --release -p nf-view --test walk_07000006_live \
  --test interact_07000006_live --test fusebox_06_live -- --test-threads=1   # the green pins
```
