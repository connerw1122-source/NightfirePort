# drone-abseil-elevator — the drone abseil family (0x8e–0x92) and the ElevatorJumper (0x8d)

Branch `s/drone-abseil-elevator` (from origin/master 170bcf01). Task 720.

## What's done

**Disc reach [D].** `nf-extract drones` over the ISO: exactly five placements carry param 1 =
0x06000892 — 07000011 ×2 (skin 0x05000008, at (18.895, 46.516, 1.434) and (19.745, 46.580, 0.363),
switch channels 0x13/0x14, the second gated to difficulty ≥ 2 by param 2 [M Drone_Create
0x136be0:49]) and 0700004a ×3 (skins 0x05000006/08, in the elevator shaft at Y −7.5/−50/−95).
No placement uses 0x0600089C (mode 0x1b) — that branch has no disc instance.

**The wiring [M].** `NDrone2_DefaultInit` 0x14b300:615-627: the placement msg 0x06000892 sets the
mode byte +0xC5 = 0x1c (0x0600089C → 0x1b), saves the skin's anim set +0xda to +0xdc and switches
to set 0xf, consumes the script word. The mode ∈ {0x1b, 0x1c} check at :757-766 arms the rope flag
+0x23; `DoTypeSettingsOLD(+0xC5)` :676 (or the explicit :764 write for the NEW chain) starts the
drone at 0x8e — `DroneTypeSettings` rows 27/28 start AbseilInit [L rows = `8e00 …`]. Ported in
`Drone::new` (crates/nf-game/src/drone.rs).

**The states [M].** All six ported into the dispatch, each citing its decomp line:
AbseilInit 0x8e (0x169610: SetDAnimNow(0xb), timer C = now + Rand_Rand(60), rope = the position),
AbseilSlide 0x8f (0x169730: CallAnim(3), +0x50 = 6.5, mode 0x1b drops the rope to −1000; the UPDATE
lands on the feet probe — anim+0x60 & 8, i.e. the collision handler's own floor probe, NOT an anim
end — restores the anim set, mode ← +0xC4 (= 0, the generic guard), the land anim 0x21 with the
DrawWeapon follow-on 0x67; the mode-0x1c arms: no opponent → hang, the rope point tracks the
opponent's Y down, within 3.0 → hang), AbseilHang 0x90 (0x169940: CallAnim(2), +0x50 = 0,
DroneWeap_Fire(1), the velocity decay ×0.98, back to the slide when the opponent is > 3.0 below),
AbseilStepOff 0x91 (0x1738e0), AbseilDeath 0x92 (0x169b28: the 0700004a free-fall arm, danim 0x40,
DropWeapon, the corpse settles then danim 10 + DEAD), ElevatorJumper 0x8d (0x169488: find the
class-0x1c door within 2.0 [0x14a940], open it [0x14aa38], when open danim 0x1e + timer C at +30,
the timer's msg 0xc → Attack 0x56).

**The movement [M, interpreter-verified].** `NDrone2_Move`'s +0x23 branch (0x14eff8) and the
collision handler's air branch (0x140968:75-111: the probe, pos += vel × REC, NO gravity, NO
velocity zeroing) are ported. The descent math was pinned by running the REAL function in this
repo's EE interpreter (`nf-formats::ee::Machine`, harness cases over straight/diagonal/old-velocity/
zero-speed/accumulate geometries): `velocity += normalise(rope − pos) · (REC_FRAME_RATE · +0x50)`
with the old velocity KEPT (it accumulates — the rope slide accelerates at 6.5 u/s²), the EE FPU
round-toward-zero (mul.s/add.s chop, div.s nearest [L the interpreter's pinned model]) and the VU
chop on the vector ops. The unit tests assert the interpreter's exact f32 bit patterns.

**Findings worth merging back.**
1. `ElevatorJumper` 0x8d, `AbseilStepOff` 0x91 and `AbseilDeath` 0x92 have **no decomp writer**:
   every constant `Drone_SM_SetState` and every `+0x5a2` write in ACTION.ELF is accounted for
   elsewhere and no `DroneTypeSettings` row starts at them. The docs' gloss "0x8D ElevatorJumper
   (07000014's Rook)" (actors.md §2/§4, dronetype-tables.md row 141) has no code path behind it —
   Rook is a kind-9 CivilianInit drone driven by the level's scripts. The state is ported and the
   test drives it directly, the way a coder slot would.
2. The AbseilSlide landing poll (`obj+0xdc → +0x60 & 8`) is the FEET-ON-FLOOR flag the collision
   handler sets, not "the anim finished" — the drone lands when its feet find ground.
3. The hang's `DCVars+0x1c = (int)Rand_FRand(1.0)` truncates to 0; the +0x4f8 bit-0x40 velocity
   decay in the collision handler's air branch has no decomp writer (skipped, documented).

## How it was verified

- `nf-game` lib tests: **243 passed, 0 failed** — including the two new unit tests (the
  interpreter-pinned descent math, the air collision branch).
- `nf-view` live tests (`abseil_elevator_live.rs`, real level data): **3 passed, 0 failed** —
  1. the 07000011 abseiler: WAIT_SWITCH on ch 0x13 [D] → ch fires → AbseilInit → (timer ≤
     Rand_Rand(60)) → Slide → Hang in the same tick, the rope fields and set 0xf asserted;
  2. the rope follows the opponent: Bond 20 below → the re-slide arm, the rope point tracks his Y,
     the descent accelerates; Bond level → hang with the ×0.98 decay; then the mode-0x1b branch
     drops the rope to −1000 and the floor probe lands the drone (mode ← 0, the set restored);
  3. the ElevatorJumper on 07000014: the forced state binds the door within 2.0, Door_Activate
     opens it, the jump anim + timer C arm at +30, and the timer lands in Attack → the combat
     chain (the log shows the 0x56 step).
- Fail-first evidence: before the port, the two 07000011/0700004a drones sat in mode 0 with no
  rope fields (the probe run printed `mode 0x0 air false`); the first build with the wiring showed
  the mode/air/set asserts going green only after the dtypenew fix.

## What's left

- **The full fast suite did not complete in the sandbox window.** It was started in the background
  at 20:15 UTC on the pre-task tree and killed when the final tree was ready (the window closed
  before a fresh run could finish: the binary build alone takes ~40 min). The touched crates ran
  green as listed above. The next session should run
  `NF_ISO=… nohup python3 tools/test/suite.py > suite.out 2>&1 &` on this branch merged with
  origin/master and report the total line.
- `nf-formats` / `nf-audio` were not touched by this task (no source change), but were not re-run.
- Live checks for the main agent (PCSX2), if wanted:
  1. 07000011, difficulty 1: at level start the rope guard (18.9, 46.5, 1.4) hangs at its
     placement, playing the set-0xF grab anim, before its trigger (ch 0x13) fires.
  2. After the trigger: AbseilInit → Slide → Hang without an opponent; the drone stays at the rope
     top (only a small creep).
  3. 0700004a: riding the lift down past the shaft guards — each should follow the lift down,
     hanging within ~3 units above it and firing.
  4. Shooting a hanging guard on 0700004a: the corpse falls free (collision off) and the death
     channel raises on the state's exit.

## Usage

`not visible to me` — this session runs through an IM-gateway worker with no token, cost or
context-window figures exposed. The only environment numbers visible are the sandbox clock
(`date -u`: started 20:14 UTC, this report written 22:24 UTC) and the repo's git log.
