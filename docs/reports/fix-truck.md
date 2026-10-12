# fix-truck — the 07000005 truck level end against tests/traces/05-truck.trace

Branch: `s/fix-truck` (from master ff6797b). Ground truth: `tests/traces/05-truck.trace` (the user's
PCSX2 run, god mode).

## What's done

1. **The end-of-level chain measured from the trace** (frames relative to the last 07000005 frame
   37173): ch 0x72 on at −317 (the gate guard at his post), ch 0x70 **and** 0x65 on the same frame −300
   (the parked plate; Bond in the bed at (29.885, 37.887, 51.656)), ch 0x73 at −273, ch 0x74 at −272,
   ch 0x1D at 0, and 07000006 loads on the next frame. Bond's parked position at the end:
   (29.8852, 37.8869, 51.6557).
2. **The 0.49 m park-height error root-caused** — two layers:
   - *Fixed:* the port's script hand-off (`PlayerEnable` in `crates/nf-view/src/scene.rs`) re-snapped
     Bond to the floor with a ray from the hand-off matrix y + 1.0. The game's `Player_Enable`
     (0x195498:28-46) writes `obj+0x30` from the matrix **directly** (a 16-byte copy), zeroes the
     velocities, and its only collision pass is the wire/creep-wall sphere test (flags 0x8c0) — no floor
     snap. The port now does the same. Event 6's "previous stream object" storage (0x1d94c0:80-84) and
     `Script_Free`'s flag-0x100 application (0x1da880:41-43) were re-verified and match the port.
   - *Diagnosed, not fixed:* the game walks Bond from the gatehouse plate to the truck inside a
     cut-scene — his obj+0x30 tracks a script walk with a neutral pad [D trace 33950–34780] and stands
     at the pickup (61.5, 27.85, 96.28) [D 34780–35080] before the ride. The walk is SP #298's script
     0x06000994 (length 1400 [D level archive]): stream 14 animates Bond's skin (0x05000007) with
     0x06000995, stream 10 disables him and re-creates the hand-off entity 0x0200063A, stream 11 fires
     event 6. The port's SP #298 never activates, so the port's Bond is freed on the road mid-route
     (matrix (92.25, 19.42, 103.58)) and the parked truck's tailgate shoves his capsule up onto the bed
     rails: the port rides at truck-root +1.06..1.23 (feet), the game at +0.62..0.81. That is the whole
     +0.44/+0.49.
3. **Evidence that the truck itself is right:** the truck's nine part matrices match the game — the root
   bone translation in `work/ee_start_05.bin` (154.0753, 15.7997, 93.9530) vs the port's frame-1
   discrete0 (154.0516, 15.7996, 93.9524) [L vs port]. The rest-surface identification: the port's Bond
   rests on a flat-up triangle of body part 020003be (the rails, world y 37.28 at the park); the game's
   stand implies 36.852, which is chassis part 020003c3's top (36.65–36.96 in Bond's column).
4. **The harness gap fixed:** `truck_ride_to_the_castle_ends_the_level` (and the new test) now set Bond
   up exactly like the nf-view headless path — `nf_view::load` then `Bond::new([100, 26, 108])` — not
   the old `bond.pos =` assignment, which kept the level-start Bond state and diverged from
   `nf-view --bond 100,26,108 --frame N`.
5. **Tests** (`crates/nf-view/tests/level_reload_live.rs`):
   - `truck_ride_park_and_chain_match_the_trace` — asserts the parked y = 37.887 ± 0.05 (xz within 0.5
     of the trace) and the chain order with the script-driven gaps (0x70+0x65 same frame; 0x73 +27,
     0x74 +28, 0x1D +300; 0x72 strictly before 0x70). It failed on the pre-fix code at
     `ch 0x70 must fire` (shown in the run), and still fails after the enable fix for the reason in (2)
     — so it is `#[ignore]`d with the full diagnosis, like its older sister.
   - Diagnostic probes (NF_ISO-gated): the park surfaces, the exact rest triangle, the hand-in
     trajectory, and dumps of the gatehouse cut-scene streams, the 0x06000994 streams and the level's
     script players.
6. **Timeline cross-check:** the game's truck script restarted at trace frame 31488 (the user's level
   restart — Bond (0,0,0) at 31489); 0x73 fires at its script tick 5412 [M] at trace 36900. The port
   runs the same script from tick 1 — the same anim frames, ~2166 ticks earlier in wall-clock. The
   park-side script gaps are unaffected.

## What's verified and how

- Every number above is read from the trace ([D]), the level archive via the port's own parsers ([D]),
  the EE snapshot ([L]) or the decompile ([M], with addresses). The pre-fix failure was shown by running
  the new test (it panicked at `ch 0x70 must fire`).
- The enable fix's [M] basis: `Player_Enable` 0x195498, `Script_EventHandler` case 6 at 0x1d94c0,
  `Script_Free` 0x1da880 — read in `work/decomp/ACTION.ELF`.
- Full suite: `NF_ISO=<iso> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio
  -p nf-view -- --test-threads=1` — run on the slim ISO before the push (see the commit; only the 3
  allowed disc_golden failures).

## What's left

1. Port SP #298's activation + the player-follows-script-anim mechanic (Bond's skin-0x05000007 anim
   carrying obj+0x30 during a cut-scene). After that the parked y and the chain should land on the
   trace's numbers; `truck_ride_park_and_chain_match_the_trace` is the acceptance test (un-ignore it).
2. Verify the 0x0200063A hand-off marker's key interpolation (the port puts it at (92.25, 19.42, 103.58)
   at creation; the game's Bond is never taken there [D 33607–35092]).
3. Not started (time): the C2/07000006 stationary-guard class, the grenade impact/explosion and
   flashbang-stun follow-ups, the 07000007 guest walk, the 07000005 torso twist, the 07000006 searchlight
   alarm.

## Files touched

- `crates/nf-view/src/scene.rs` — the PlayerEnable hand-off (shared file; minimal, cited above).
- `crates/nf-view/tests/level_reload_live.rs` — the tests and probes.
- `docs/WORKLOG.md` — the fix-truck entry.

No game bytes, no private URLs, tokens or credentials anywhere.

## Independent audit

A read-only auditor re-checked the five load-bearing decompile claims against
`work/decomp/ACTION.ELF`: (1) `Player_Enable` copies MATRIX+0x30→obj+0x30 directly, spherical angles via
`vecutil_cartesian_to_spherical_acc`, the only collision query is the `Collide_SphereIntersect`
wire/creep-wall dispatch; (2) `Player_Disable`'s two arms; (3) `Script_Free`'s 0x80/0x100 flag arms; (4)
`Script_EventHandler` case 6's previous-stream (idx−1) matrix storage, cross-checked against three
independent functions' stream layout; (5) `Script_AnimStart`'s new-object + `AnimGetFirstTrans` root
placement with no player special case. All five VERIFIED with line evidence. Caveat noted: the
dispatched `Player_CollCreepWall` handler does its own ray Y snap (a '+' cel hit) — not applicable to the
truck bed; and `AnimGetFirstTrans` writes only when `sAnimScript+0x78` has bit 0x800000.
