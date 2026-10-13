# walkreal-01-cover — the 07000001 contact walk un-parked

Branch `s/walkreal-01-cover` (from origin/master c9132e0). The task: make the walker break a guard's
cover the way a player does (re-aim at exposed parts, step to a clear line, fire), un-ignore
`walkreal_07000001_live::walk_from_the_spawn_to_the_exit_with_the_contact`, and show the walk green on
master — no RNG reseeding.

## What the reproduction showed (on this master, before the fix)

The stall had evolved past the driver's snapshot (more RNG draws have landed since fix-ai-traces2), in
two layers:

1. **Ammo exhaustion with a broken resupply.** The fight (with the old torso-only aim spraying the
   trigger while the ray was blocked) drained clip+spare to 1 round; the resupply walk_nav_to'd to an
   ammo pickup that sat 1.33 u away BEHIND a cycling doorway — the nav's final-arrive radius (1.8)
   declared "arrived" without crossing, so `resupply` returned false three attempts running.
2. **A survivor the walker could not hit.** A wounded guard (health 10) sat ~11 u out with EVERY hit
   box's eye→centre segment blocked (`exposed false` for 10800 ticks): the old `shoot` aimed only at
   root+0.3, fired on aim-lock even when the ray hit the wall at t 0.82 (the driver's guard-16
   symptom, now guard 15/17 depending on the stream), and its 900-tick fallback sidesteps oscillated
   ±60 ticks around the same pinned corner.
3. With the fight unresolved the escort stayed in LeadHide 0x1f / LeadWait 0x20 ([M AllyLeadHide
   0x15ec08, the lead wait 0x15e4f0/0x15ea28]), parked 32 u short of the LoadPointActivator plate.

## The fix (all in `tests/common/walker.rs` — every walk benefits)

1. **`Walker::exposed_aim(di)`** — enumerate the drone's own hit boxes (`Data::…`, `hit_boxes` [L
   hit-box dump]; the game damages per part group in `NDrone2_HitDamage` 0x145558 — head 5 ×100, the
   arms 53–56 ×1.0, legs ×0.75) and return the clearest box centre: the eye→centre world segment
   (`World::segment`) must miss everything. Preference: the clear box closest to the torso anchor
   (root+0.3), so a clear torso wins and a head/arm pick happens only when the torso is buried.
2. **`shoot` rework** — aim (turn+look through the real input) at the exposed part, re-picking every
   30 ticks; **the trigger stays down while nothing is exposed** (firing into the wall only burns the
   clip — a player holds fire); while blocked, flank: alternate sidesteps every 90 ticks plus a
   forward push beyond 8 u; after 9 s pinned, **walk the nav graph to a flank point** (perpendicular
   to the guard bearing, alternating the side, growing the offset 3→5→7) and re-check — the player's
   walk-around-the-cover. The 900-tick no-damage fallback (4 repositions) stays for
   clear-aim-but-missing cases. No RNG draws added anywhere (the walker's inputs are the same kinds
   the old code sent; the streams the game owns are untouched).
3. **`resupply` close-up** — after the nav, if no ammo arrived and the pickup is still >0.9 away,
   walk_to it directly (arrive 0.7, lenient): walk_to's stuck handler opens the cycling doorway on
   the way. The 01 west-hall ammo box now lands.
4. **The stuck handler's repeat-door guard** — the second stuck-cycle use of the SAME door without
   Bond crossing it falls through to the plain wedge ladder (back up, sidestep, jump) instead of
   looping open→auto-close→reopen forever. This un-wedged the stairwell-head threshold on the final
   leg (the door sat ON the lane at 1.1 below Bond's approach line, opened, re-closed, and re-hijacked
   the mitigation every 60-tick sample — the sidestep ladder never ran until now).

## Verification

- **The un-ignored walk is GREEN on this master**:
  `walk_from_the_spawn_to_the_exit_with_the_contact … ok` — 78110 sim ticks, 23.5 s wall: intro ch 0xB
  → the guards cleared through the real fire path (the exposed-part aim kills the buried wounded
  guard) → the escort's LeadHide/PlayerInWay recoveries → the contact walks his route onto the
  LoadPointActivator plate → Bond descends the stairwell and steps on → the AND-touch ch 0x10 fires →
  `level_exit = Some(07000002)`.
- **Nothing regressed** — every other walker user re-run green: all 19 `walkreal_*` binaries (the
  same ignored probes as master), all 22 `walk_*` binaries, and the full four-crate suite
  (`nf-formats`, `nf-audio`, `nf-game` green; only the 3 `disc_golden` bigf tests fail — the slim-ISO
  allowance; nf-view green across all binaries in batches covering every target).
- The walker changes touch the shared `walk_to` stuck ladder: the repeat-door guard only changes
  behaviour when the SAME door is picked twice in one leg — the previously-passing walks (which
  crossed their doors) are unaffected, as the green walk/walkreal sweep shows.

## Was the survivor's cover the game's behaviour or a port bug?

Not settled either way — the walk now clears such guards either way, so the question stopped blocking.
What I can say with evidence: the surviving guards were the game's OWN cover choice executed in the
port (state 0x5c UnderCover crouch at a cover node; the port's `DroneFunc_ConsiderExplosive`/
`HandleExplosives` lanes, docs/port-map/guard-ai.md), and the old failure was the WALKER's (a torso-
only aim + fire-on-lock while the ray hit the wall, with sidesteps pinned in a corner). Whether the
port's UnderCover guard exposes/peeks exactly when the game's does (the game's cover guard leans out
to fire) is still open — a `gdbwatch` pass on Drone+0x5c-state guards in 07000001 during the first
fight would settle it (below).

## Live checks for the main agent

1. **The buried-guard exposure**: 07000001, fight at the first room: with a guard in 0x5c (crouch in
   cover), `gdbwatch` the guard's anim state + the cover lean for 10 s — does the game's guard expose
   head/arms periodically (peek-fire cycle) where the port's holds a static crouch?
2. **The stairwell threshold**: after this walker fix, the port's own walk proves the descent is
   walkable; no live check needed.
3. Render spot (visual only, no change expected): `--bond -13,6.5,11.5,2.5,0` on 07000001 should show
   the west hall with the stairwell door ahead — the wedge spot the walker now escapes.

## What's left

- The whiteout-falloff/occluder items from the earlier sessions are other tasks' lanes.
- The UnderCover peek-cycle question (above) is the only follow-up worth a live pass; the walker now
  handles both answers.

## Commit map

- `crates/nf-view/tests/common/walker.rs` — exposed_aim, the shoot rework, the resupply close-up, the
  repeat-door guard (shared file for the walkreal tests only; not a src file).
- `crates/nf-view/tests/walkreal_07000001_live.rs` — the `#[ignore]` removed.
