# bonus-007 — the 007 bonus ("Bond moves") award

## What was done

The user (2026-10-16): "007 bonuses for doing certain actions are missing". The award chain in
the game is: a per-level object raises switch channel **0x65..0x6E (101-110)** → `Trigger_Calc`
0x1b1900:12-20 [M] calls `PlrStat_DoneBondMoment(0, ch)` 0x199448 [M] → behind the mission gate
and the `BondMoments[id−0x65]` new-moment gate it sets the flag, bumps `PlrMissionStats+0x20`
(the Mission Stats' Bond Moves row) and writes `glb_blokes[0]+0x948 = 200` — the HUD's 007-icon
flash timer, counted down by `FRAME_RATE_MUL` (1.0 at the port's fixed 60 Hz — GS_SetRefreshRate 0x17d2b8:15;
[L 0x30d0d8] = 1.0 in the EE snapshots) per `HUD_UpdateHealthPane` 0x19b9c8:232-260 pane update
(the icon sprite's +0x38 draw layer is 0 while the timer is up; 0xFF = not drawn only at timer < 1). The chime is **level data**: a
SoundTrigger placement (0xF9, `SoundTrigger_Create` 0x1b2628 / `SoundTrigger_Update` 0x1b2790)
whose ON channel is the bond channel plays the level's sting.

Two of the three user-visible pieces were already ported by earlier tasks: the count
(`Trigger_Calc`'s fold in `drone.rs` → `progress.rs::done_bond_moment`, fix-progress) and the
chime (`SoundTriggers`, fix-06-alarm — on 07000005 placement 82 gate 0x65 → sound **331**
`SFX_BOND_MOMENT_BM_CAS_BEAM`, placement 81 gate 0x69 → **332** `..._CAS_TRUCK` [D placements +
the ELF name→id table]). The two missing pieces, fixed here:

1. **The HUD flash state (BL+0x948) did not exist.** `progress.rs`'s comment said "owned by the
   view" but nothing modelled it, so the draw code has no flag to read. Added
   `PlayerHealth.bond_moment_flash: u16` (`crates/nf-game/src/bullet.rs`, own line at the end —
   merge lesson 4), written **200 exactly on the award tick** (inside the same new-moment gate
   the game uses, so a re-risen channel never re-flashes [M 0x199448:20-25]), decayed by 1 per
   tick next to the pane's other timers (`drone.rs`, the same place `health.flash` decays).
   Nothing draws — the state + countdown are the deliverable.
2. **The pause SCORE tab hardcoded "0"** for Bond Moves / Dispatched / Subdued / Surrendered /
   007 Bonus (`menu.rs` row 3). Added the counters to `menu::Stats` (own lines), a public
   `Progress::stats()` getter, and `pause_stats(sim)` synced by the host exactly where the
   OBJECTIVES rows sync (`lib.rs`, both sites) [M C_GCPAUSE_Handler 0x204408:447-498 reads the
   record's rows]. The tab now shows the live tally; shots/accuracy/time keep their existing
   (still-zero) fill — pre-existing, unchanged.

Plus `docs/systems/bond-moves.md`: the award path, the sound-id family table (from the ELF
name→id table [D]), and the census of every story level's bond moves (bond channel, the
trigger/script placement that raises it, the chime SoundTrigger), from the disc via
`cargo run -p nf-view --example census_bond_moves` (scratch example, not committed).

## How it was verified

`crates/nf-view/tests/bond_moves_live.rs` (ISO-gated, the real input path — `nf_view::load` +
`Walker::step_bond`/inputs, no teleports; the truck-ride route is `gate_guard_truck_live`'s):

- `truck_ride_awards_bond_move_101_with_flash_and_chime`: the cold start → the gatehouse porch
  plate (ch 0x5C at tick 1884, matching the trace) → the hand-off → the crouched ride → the park
  → **the award at tick 5386, the same tick ch 0x65 rises**: count 0→1, `BondMoments[0] = 1`,
  flash = **200**, and **chime 331 playing the same tick** (the gate-0x65 SoundTrigger). Decay
  pinned exactly: 199 the next tick, −1/tick. Sisters inside the run: ch 0x69 never rises, moment 105 stays 0, chime 332
  never plays.
- `idling_at_the_start_awards_nothing`: 600 ticks at the cold start — count 0, flash 0, all ten
  bond channels off, neither chime plays.

Result: **2/2 pass** (9.1 s wall). The flash is pinned exactly: 200 at the award tick, 199 the next (−1/tick). "Reproduce first": on the pre-fix tree this test cannot
compile (no `bond_moment_flash` field) — the missing state IS the symptom; the count and chime
assertions pin the already-ported halves so the whole chain is now guarded end to end (the
per-tick assertion fails if anyone breaks the award fold, the SoundTriggers, or the gate).

Also ran: `cargo build -p nf-game -p nf-view --tests` clean; `nf-view` test binaries compile
after the `Stats` field additions (all struct literals spread `..Stats::default()`).

## What's left

- The HUD draw of the 007 icon (reads `sim.drones.health.bond_moment_flash`; the game's
  animation is `HUD_UpdateHealthPane` 0x19b9c8:239-257 — scale `(200−t)·0.01`, the sin bounce on
  the y-scale, the sprite's +0x38 layer 0 while the timer is up, 0xFF (not drawn) at timer < 1). Visual → the main agent renders it.
- The pause SCORE tab's shots/accuracy/time still fill 0 (pre-existing gap).
- The census's per-level "what the player does" descriptions are one hop deeper than this task
  went (the chains behind channels 61/62/112/113/127-129/133…); the channel-level census is
  complete in `docs/systems/bond-moves.md`.
- MP maps (0780/0790/07F0) not censused; `DoneBondMoment`'s MP arm exists [M] but the MP UI is
  out of scope.

## Live checks for the main agent

1. `gdbwatch` on `BondMoments` 0x2bcde0 (expect the awarded id's halfword → 1) and
   `PlrMissionStats+0x20` 0x2bcc40 (expect +1) during any award, and on Bond's BLData+0x948
   (expect 200 at the award tick, then −1 per frame).
2. A render check list for the (future) 007-icon draw: on 07000005, walk the truck ride; at the
   park the icon should flash by the health pane for ~1.65 s (200→101 at −1/frame at 60 Hz).

## Usage

not visible to me

## Suite

`tools/test/suite.py` in the background was killed between tool calls (this sandbox reaps
background jobs — the pause-music-1b entry saw the same), so the crates my tests touch ran in
the foreground instead: `cargo test --offline --release -p nf-game -p nf-view -- --test-threads=1`
— every binary passes except `fix_0c_pylon_live`'s 2 tests, which panic on `wgpu`'s adapter
expect at the sandbox's missing GPU (pre-existing environment failure, unrelated to this task;
the same run passes them nowhere else here). nf-formats/nf-audio untouched by this task.

## Independent audit

A post-implementation audit (subagent, 2026-10-17) verified the award-gate equivalence
(drone.rs fold vs 0x199448:20-25), the SCORE counter mapping (+0x0C/+0x10/+0x14/+0x20/+0x24),
the merge discipline (WORKLOG pure append, own-line fields, no weakened tests) and the census
rows; it caught three errors, all fixed in this revision: the decay was modelled −2/tick but
FRAME_RATE_MUL is 1.0 at the port's 60 Hz ([L 0x30d0d8] in the EE snapshots — now −1/tick, the
test pins 200→199), the "hidden below 101" description misread sprite+0x38 (it is the draw
layer, 0xFF = not drawn — docs/fields/sprite.md:30), and BM_STAB3 is id 0x14a (330), not 0x14f.
