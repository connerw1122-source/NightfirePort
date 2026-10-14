# fix-swim — the 07000002 pond swim and the water ladder under the hut

Branch `s/fix-swim` (from `origin/master` ccaea18, merged with master b65d6f7 mid-task; the WORKLOG
conflict was resolved by keeping both entries). Task `102h-fix-swim`.

## What was wrong (the user's playtest, results 1013-1825, `--bond -82.3,5.3,32.05,1.57`)

"This is hardly swimming … spamming the crouch button, and I very, very slowly got into the water",
"I could hear my footsteps too", "I can get in the water, but I can't crouch in water. I'm not sure
how to swim", "There's a ladder in the water under that hut. I can't latch onto it and climb it".

Reproduced headless first (`crates/nf-view/tests/fix_swim_live.rs`, run against the pre-fix tree):
walking in DID enter the swim substate (tick 126 from the bank, feet 0.3 under the pond surface —
the `Player_InWater` entry was already correct), but the stroke was planar-only (look input did
nothing, y frozen at the entry depth), crouch/jump fired the invented ±2.4/tick vertical, the pond
footstep slosh (sfx 129) played every stride mid-pond, and swimming into the hut ladder never
attached. One harness note: 07000002's scripts grab Bond into cut-scenes within seconds — every test
step carries the game's own skip (`drones.scripts.skip_request`), exactly like the walkreal walker.

## The fixes (each cites the decompile)

1. **The swim stroke rides the LOOK direction** — `crates/nf-game/src/player.rs`
   (`physics_tick`'s swim branch). `Player_Move` 0x191dd0:149-165 [M]: in substate 3 the forward
   speed rides `RotMatrixX(obj+0x50 − BL+0x8a8·π/2)` + `Mat_GetDir` — dive by looking down, surface
   by looking up; strafe stays on the horizontal local X (the game overwrites the X lane after the
   dir add). The port's `Bond::look` is the same convention (pitch +1 = up = the game's
   `dir.y = sin(0x8a8·π/2)`). The case-3 tail clamp [M 0x193600:201-220] already ported (rise at
   most to `surface − 0.3`). **The crouch/jump ±2.4/tick vertical was [S] fiction and is deleted**:
   case 3 of `Player_Update` 0x193600:181-222 reads NO crouch/jump input, and `Player_HandleJump`
   0x1916b0 is only ever called from `Player_SSWalk` 0x192310:69. The old text's "crouch dives,
   jump surfaces" is not what the game does — the decompile is authoritative; diving is look-down +
   forward. The multiplier is always 1.0 in swim (case 3 calls `Player_Move(0x3f800000, …)`;
   the 0.75 is `Player_SSCrouch` 0x1924c0:58's own argument) — a crouch-flag leak into `k` is fixed.
   `enter_water` additionally clears the strafe smoother (`Vec_Zero(BL+0x20)` [M 0x195e50 disasm]).

2. **The ladder grab gate excludes ONLY climb (1) and grapple (2)** — `player.rs::ladder_grab`.
   `Player_CollLadder` 0x196118:16-17 [M]: `substate ∉ {1, 2}` + regrab (+0x951) + health. The old
   `substate != SS_WALK` gate made swim_tick's grab call dead code (fix-ladder's WORKLOG claim
   "swim_tick now runs the grab scan" was defeated by it) and also blocked crouch (4). One line:
   `matches!(self.substate, SS_CLIMB | SS_GRAPPLE)`.

3. **The grab pick is the game's shape** — `crates/nf-game/src/ladder.rs::capsule_hits`.
   `Collide_PickObj` 0x1eae80:152-168 [M]: the player's HITTEST capsule segment (A = pos+up·0.55,
   B = pos+up·(0.55 − AnimObj+0xCC), root 1.0347 [M `Player_Collision` 0x194df0:313-337]) against the
   ladder's world sphere inflated ×1.5 plus the capsule radius (pick mode +0x7D4 = 0x800), refined
   against the glist mesh by `Collide_Intersect` 0x1ebaa8 (NOT ported — needs the mesh triangles;
   noted below). The old [S] axis-column band (±0.5 y slack) rejected the deep swimmer at the hut
   ladder (the pond bed there sits at y≈0; the band started at 1.33) that the game's pick catches —
   `Player_CollLadder` 0x196118:23-31 then lifts pos.y to `bottom + 0.02`, the climb-out assist.

4. **No footsteps while swimming, the MonitorAir sounds instead** — `crates/nf-view/src/scene.rs`
   (shared file, two small hooks, listed below) + `crates/nf-game/src/sound.rs` (a bookkeeping
   field). The game's step needs a CACHED ground hit the swim capsule never gets (`Effect_Create`
   0x1ce688:63 early-outs unless `(hit+0x50 & 0x3f) != 0` [M]); the port's floor ray still finds the
   pond bed mid-pond, so the scene gate skips the step accumulation while substate == SS_SWIM.
   `Player_MonitorAir` 0x191a70:101-143 [M]: the head crossing `cel+0x90` starts the dive loop
   (`Sound_Play3D(100, −1, −1, 0x27)`, stopped on surfacing) and plays the gasp (`Sound_Play(100,
   0x60c)`); the drown tick plays 0x60b on the same 30-tick cadence as its `HandlePain` [M
   0x191a70:135-143]. All three are hooked. The old "swim stroke sound" framing has no decompile
   basis — no stroke sfx exists in the player path; these three ARE the game's swim sounds.

## Verification (all shown failing first where the fix changes behaviour)

- `player::tests::the_ladder_grab_gate_excludes_only_climb_and_grapple` (nf-game, no NF_ISO): swim
  (3) and crouch (4) grab a hand-built ladder in one tick; sisters: the snap holds the axis and the
  y-snap never drops the swimmer below `bottom + 0.02`. FAILED pre-fix (both stayed in their
  substate), PASS after — verified by replaying the tests against a HEAD player.rs with only the
  test bodies injected (both FAILED there).
- `player::tests::swim_moves_along_the_look_direction_and_the_surface_clamps` (nf-game): pitch −0.5
  + forward dives (y −0.5+ and moves along the look dir), pitch +1 + forward surfaces and clamps at
  exactly `surface − 0.3`, crouch/jump spam with neutral look moves NOTHING. FAILED pre-fix (planar
  movement + the ±2.4 block), PASS after (same replay proof).
- `fix_swim_live` (nf-view, NF_ISO, 4/4): walking off the bank enters the swim and holds the
  game's walk-parity stroke cap (≥ 0.085/tick over a 60-tick window; the ±0.1/tick smoothed cap is
  `Player_Move` 0x191dd0:92-138 [M]) — regression pin; the look-directed dive/surface/clamp through
  the real level (pond surface 3.7675/3.765/4.339 [D probe]); NO pond footstep slosh (129) and no
  0x27/0x60c during a head-above surface swim (pre-fix: 129 ×10 in 240 ticks); the hut ladder
  (placement centre −74.84, 4.04, 22.22, bottom 2.83 — 0.94 into the pond) GRABS FROM THE SWIM at
  +52 and the top dismount lands y 7.118 above top 7.102.
- Sisters green on the fixed tree: `fix_ladder2_live` 4/4, `fix_ladder_live` 3/3 + 1 ignored,
  `fix_swim_vent_live` 2/2 (the vent crawl untouched), `walkreal_probe3_live` 2/2 (the 02 walk
  route + its ladder), nf-game lib 145/145.

## Shared-file hooks (small, additive)

- `scene.rs`: the footstep gate gained one clause (`&& self.bond.substate != SS_SWIM`); the
  MonitorAir sound block (dive loop / gasp / drown 0x60b) sits inside the existing `!disabled`
  region after the drown-damage arm. Nothing moved or removed.
- `sound.rs`: `Sounds.swim_prev_under: bool` (own line) — the under-water edge state so the scene
  hook needs no new Sim field.
- `player.rs` / `ladder.rs`: the task's files. `LadderCtx.sink` is now unread (the [S] sink went
  away with the fiction); the field stays for API stability.

## What's left / live checks for the main agent

- **The entry depth** (the "very slowly got into the water" feel): the port enters the swim when
  Bond's cel becomes a water cel AND `pos.y − 0.3 < cel+0x90` — on the 02 bank slope that happened
  at feet 0.62 under (pos 3.747 vs surface 3.7675), because the port's `find_cel` (smallest
  containing bbox) only returns water cel 17 from that depth. The game's `build_FindCel`
  0x1c8cb0:163-232 tie-break cannot be ported onto the port's cel data (same limitation the
  fix-swim-vent5 report documented). **Live check 1:** in PCSX2 at the 02 bank (`--bond -82.3,5.3,
  32.05,1.57`, walk in slowly), with gdbwatch on the player's `obj+0x20` (cel) and `cel+0x90`
  (surface): report the tick where substate `obj+0xF6` flips to 3 relative to `pos.y` — if the real
  game enters at feet 0.3 under, the port's cel assignment is late and needs a water-cel fallback
  (e.g. `Player_FeetOnPoint`-style straddle). If the game also enters ~0.6 deep, the port matches.
- **The swimmer sinks through the pond bed**: the port's grounding path drops mat-0xE planes
  (world.rs:793), so a DIVING Bond can pass through the visual bed (the game's swim capsule
  collides with the cel geometry — `Collide_SphereIntersect` 0x1ec940 takes the cel). Not fixed
  here (world.rs is not this task's file). **Live check 2:** dive to the 02 pond bed in PCSX2 —
  does the game's Bond rest on the bed or sink through? If he rests, the port's swim capsule needs
  the bed planes back when substate == 3.
- **The glist mesh refine** in the ladder pick (`Collide_Intersect` 0x1ebaa8) is not ported; the
  inflated-sphere pick is slightly wider than the game for thin ladders. The grab gate's yaw window
  (±π/2 [M 0x196118:18-22]) remains the real gate.
- `Sound_StopAllWithId(0x12/0x3d/0xf0)` in `ChangeSubState` 0x190630:75-88 [M] (the substate-change
  scrub) is not ported — the port has no per-group sound taxonomy; the footstep gate covers the
  audible case. The swim exit's `BL+0x4C = 2` anim-blend write is likewise not ported (cosmetic).
- **Other levels with grapple/swim water**: the swim system is level-agnostic (cel+0x90 driven);
  the ladder-in-water case exists wherever a 0x22 placement's bottom dips under a water cel. The
  02 hut ladder is the verified one; a census of ladder-bottom vs water-cel overlaps per level is
  cheap follow-up work (`Ladder::all(level)` + the cels' water values).

## Suite

The full four-crate suite did not fit this session's window (setup + research consumed most of it);
what ran on the final tree (merged with origin/master b65d6f7): nf-game lib 145/145, the four new
`fix_swim_live` tests 4/4, and the affected families — `fix_ladder2_live` 4/4, `fix_ladder_live`
3/3 (+1 ignored), `fix_swim_vent_live` 2/2, `walkreal_probe3_live` 2/2 — all with
`--test-threads=1`. No RNG draws added; no wall-clock in Sim.

## ntfy

(filled below if the notification fails)
