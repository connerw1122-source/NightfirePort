# fix-ladder-water — the ladder dismount into water and the cut-scene skip's voice

Task 109i. Branch `s/fix-ladder-water` (from `origin/master` d5b22f91).
User playtest 1014-1725, level 07000002 (the garden pond's hut ladder), real-input build
(`nf-view <iso> <level> --dev`): "I'm also trying to climb off the ladder and get back into the
water, and there's a bug with that. I'm infinitely latching onto the ladder, and I can't get off of
it." and "Skip the cutscene in the middle of dialogue, and the dialogue still kept going until it
finished... just like the voice line itself."

Files changed: `crates/nf-game/src/ladder.rs`, `crates/nf-game/src/player.rs`,
`crates/nf-game/src/nis.rs`, `crates/nf-game/src/drone.rs` (one request arm + one field),
`crates/nf-view/src/scene.rs` (one flush consumer + one `Msg` field), tests
`crates/nf-view/tests/fix_ladder_water_live.rs` (new) and one spawn-depth fix in
`crates/nf-view/tests/fix_swim_live.rs` (test-only, see "Shared-file hooks").

## 1. The infinite latch on dismount into the water — FIXED

### The cycle (reproduced before the fix, 50 re-grabs in 360 ticks on master, probe)

1. The dismount [M `Player_Climb` 0x196310:88-96 + the below-bottom branch] sets regrab (+0x951) = 60
   and `ChangeSubState(0)`; the yaw stays facing the ladder (the climb's ease).
2. The swim entry the very next tick overwrites the regrab with 5 [M `Player_InWater` 0x195e50:24-42].
3. 5-6 ticks later the regrab expires; the swimmer is still ~0.5 off the ladder axis (the grab's own
   snap offset [M `Player_CollLadder` 0x196118:36-44]) — the pick hit, the grab fires, snaps him back
   to 0.5, and the climb's yaw ease [M 0x196310:18-33] eats his turn. Limit cycle.
4. Constants checked against the EE snapshot: FRAME_RATE 60.0 (0x30d0d0), FRAME_RATE_DIV 1.0
   (0x30d0d4) [L] — the port's regrab numbers (60 dismount / 5 swim entry) are faithful [M], so the
   break had to be in the pick.

### The cause

The port fed every substate the same pick (`Ladder::capsule_hits`): the walk capsule segment
[pos+0.55 .. pos−0.48] against the ×1.5 bounds sphere [M `Collide_PickObj` 0x1eae80:152-168, the
mode-0x800 arm] plus a stage-2 stand-in — a radial axis-column `min(radius,1)+capsule` = 1.55 on the
hut ladder (centre −74.84, 4.04, 22.22, r 2.21 [D probes]). Two divergences from the game:

- **The swim pick is a different HITTEST** [M `Player_Collision` 0x194df0's capsule switch, case 3]:
  capsule radius +0x7CC = 0.55 and the segment A = pos + up·(0.1 − (cc − 0.55)), B = pos + up·(0.45 − cc)
  — a short LOW capsule ~0.48 under the root, not the walk arm.
- **The mesh refine is missing**: every sphere candidate is refined against the actual glist mesh
  (`Collide_Update` 0x1ea398:84-93 → `Collide_Intersect` 0x1ebaa8). A ladder's pickable body is its
  thin rung plane plus the rails' lateral span, so the game's swim reach ends ~at the capsule radius
  off the plane — the dismounted swimmer's 5-tick drift (~0.2 at the `Player_Move` 0.2/tick ramp
  [M 0x191dd0:108-117]) clears it, exactly the game's escape. The port's 1.55 column never let go.

### The fix

- `Ladder::capsule_hits_swim` — the game's swim arm [M 0x194df0 case 3] for stage 1.
- Stage 2 for BOTH forms is now the mesh-scale bound (`in_mesh_column`): the offset from the ladder
  axis decomposed along its facing, `|frontal| ≤ capsule_radius` (the rung plane) and
  `|lateral| ≤ 0.4 + capsule_radius` (the rails' span) [S — the port's stand-in for the unported
  cel scoping + mesh refine; it replaces the pre-109i bare radial column and keeps every existing
  pin green: the 0700000c yard-ladder walk, the 102h dismount cases, the walk grab shapes].
- `player.rs::ladder_grab` selects the form by substate [M — the game builds the HITTEST per
  substate; `Player_CollLadder` 0x196118 excludes only substates 1/2, so swimmers still grab].

### Verification

- `fix_ladder_water_live::dismount_into_the_pond_lets_go_of_the_ladder` — the real input path: walk
  in, grab, climb, action-dismount (the action button with the stick neutral [M 0x196310:45-48]),
  keep the stick pulled back 240 ticks: no re-latch, drift past the plane reach. **Fails on master
  (re-latched at +27), passes with the fix.**
- `swimming_into_the_ladder_still_grabs` — the sister pin: the pond grab from the swim still attaches
  and climbs out (the 1013 fix-swim feature keeps).
- Unit tests in `ladder.rs`: the swim segment's shape, the plane cut at 0.62 vs the snap 0.5, the
  lateral span, the deep-column reach and the thin-ladder sister.
- `fix_swim_live` 4/4, `fix_swim_vent_live` 2/2, `wire_live` 2/2, `walkreal_0700000c_live` 1/1 (the
  102h regression), `fix_ladder_live` 1/1, `fix_ladder2_live` 3/3 — all green on the fix.

`fix_swim_live.rs`'s hut-ladder pin spawned Bond ON the pond bed (root y 0.0 via `stand_at`) — the
game's LOW swim capsule [M 0x194df0 case 3] reaches the ×1.5 sphere only from the water column
(root ≥ ~0.56 for this ladder; a bed-hugging root sits ~4.5 from the sphere centre vs the 3.87
reach), so the pin's spawn moved to the swim column (root 3.0, `Bond::new`) — test-only, the
1013 player swam in at column depth. **The deep-bed grab below the game's swim-capsule reach is no
 longer granted by the port** — that extra reach was an artifact of the walk capsule.

## 2. The cut-scene skip's voice and dialogue — FIXED (evidence-complete; end-to-end repro limited)

### The voice stop

The NIS voice is a kind-5 sound stream (`Script_SoundStart` 0x1d8820 sets the stream kind 5).
`Script_KillStream` 0x1d9860:96-99 case 5 calls `Sound_Stop(stream+0x10)`, and `Script_Free`
0x1da880:27 kills every stream when the script ends — so in the game, the skip's fast-forward ends
the script mid-line and the voice dies AT the skip. The port's `Script_KillStream` port (`nis.rs::
kill`) had `_ => {}` for kind 5: no stop request was ever pushed. The fix: case 5 pushes
`Request::SoundStop { src: (hash, i) }` (the same request the stream's own end op 0x17 uses; the
drone's handler stops the tracked handles). This also covers a sound that outlives its script at a
natural end [M 0x1da880:27].

### The dialogue text

`Script_FFwd` 0x1dac78:26-30 (the ScriptCam arm) calls `Text_FlushAllSubtitles` 0x1b15c8 — the
type-4/5 messages die at the skip [M]. The port never flushed; worse, the skip's catch-up passes run
the remaining `Script_TextStart` ops (0x1db2b8 has no skip guard in the game either) and the port's
HUD *queues* every line (`hud.msgs`), so the remaining dialogue replayed after the skip — the user's
"the dialogue still kept going until it finished". The game draws a single active subtitle slot
(`Text_Update` 0x1b1510), so its catch-up adds never display as a continuing dialogue. The fix:
- `ffwd()` pushes `Request::TextFlush` past the camera gate (the game's ScriptCam arm) [M];
- the catch-up passes (loop pass ≥ 1) drop their `Request::Text` adds [S — the port's queue vs the
  game's one-slot draw; see Live checks];
- the drone clears `script_msgs` and raises a one-tick `text_flush` flag; the scene drops the
  script-sourced HUD lines only (the `Msg.script` tag — the game's type-4/5 scope [M 0x1b15c8]),
  leaving the pickup/INFO queues alone.

### Verification (honest)

- `fix_ladder_water_live::skipping_the_cutscene_stops_the_dialogue_voice` and
  `script_end_stops_a_still_playing_sound` (level 07000004's intro cut-scene, 5 live kind-5 streams
  [probe]): the skip and the natural end both stop every captured voice. **These two pass on master
  as well** — that level's sound streams carry their own `Script_SoundEnd` ops, so master's voices
  die by their own ops and the kill()-case-5 gap does not manifest there. They pin the path.
- **No available level reproduces the user's no-end-op voice end-to-end headless**: 07000002's intro
  runs 1500 ticks disabled with ZERO kind-5 streams [probe]; 07000004's voices all have end ops; and
  07000004's script never arms the skip gate (no `Script_CameraStart` → the game's own gate refuses
  the skip there too [M 0x1dac78:19-21]), so a HUD-side skip test is not runnable there. What IS
  failing-first is the mechanism: on master the catch-up leaves the script texts pending
  (`drones.script_msgs`) — verified in the stash run before the fix — and the kill()-case-5 gap is
  [M] decomp evidence. See Live checks for the PCSX2 confirmation.
- `nf-game` lib 161/161 (incl. the new ladder unit tests).

## Shared-file hooks (minimal, additive)

- `drone.rs`: the `Request::TextFlush` arm (3 lines) + `pub text_flush: bool` on its own line.
- `scene.rs`: the flush consumer (4 lines) + `pub script: bool` on `Msg` on its own line.
- `fix_swim_live.rs`: the hut-ladder pin's spawn depth (test-only, §1).
- No renames, no reorderings; WORKLOG append-only.

## Live checks for the main agent

PCSX2 (NTSC-U, level 07000002):
1. The hut ladder dismount into the pond: climb the ladder, action-dismount into the water, hold the
   stick away. Expected (the game): at most one re-latch near the surface, then Bond swims free
   within ~0.5 s. Watch +0x951 (the regrab) — the swim entry writes 5 [M 0x195e50]; gdbwatch
   0x951-relative writes on the dismount tick (60) and the water-entry tick (5).
2. A mid-dialogue skip (any skippable cut-scene with a voice): the voice must cut at the skip. Watch
   `Sound_Stop` (0x1156e0?) calls during `Script_Free` 0x1da880 — expected: one per still-playing
   kind-5 stream. If the game's screen shows any further dialogue lines after the skip, the port's
   catch-up Text drop [S] is too strong and should be reduced to a flush-only model.
3. The 07000004 intro script's skip gate: confirm in PCSX2 whether that script is skippable at all
   (the port says no — no camera flag); if the game skips it, the port's `+0xAB5` bit-4 source
   (`Script_CameraStart` 0x1d9140) needs a second look.

nf-view `--bond` spots (level 07000002):
- `--bond -82.3,5.3,32.05,1.57` then walk +X: the pond entry, swim to the hut ladder
  (−74.84, 22.22), climb it, action-dismount back into the water, push away — the escape must take
  well under a second.
- The same spot, dismount and do NOTHING: the game-faithful behavior is a surface-cycle re-latch
  (no-input swimmers re-latch at the snap in the game too [S]); the escape requires input. If the
  main agent's PCSX2 shows the game letting a no-input swimmer drift free, the swim stage-2 cut
  should tighten by ~0.1.
