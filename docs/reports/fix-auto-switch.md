# fix-auto-switch — report

Branch `s/fix-auto-switch` (from origin/master 6be39ed; merged origin/master 2302fc7). Task:
turn the pickup auto-switch ON (re-open the fix-pickups-remainder §3 gate) without breaking the
01 escort walk. **Done: the drain is live, the walk is green, and the decompile says why the
wedge never existed in the game.**

## What was done

### 1. Reproduce before fix — the wedge does not reproduce on master

Opened the gate behind an env flag and ran
`walkreal_07000001_live::walk_from_the_spawn_to_the_exit_with_the_contact` twice: **green both
times, `level_exit at tick 319323` — identical to the gated master runs.** With `NF_PICKUPLOG`
on, the run shows why the reported scenario is unreachable on current master:

- `[take] tick 776 weapon 0x12 kind 0` and **`[take] tick 817 weapon 0x0e kind 0`** — the port
  takes a **placed Raptor pickup at the spawn during the intro cut-scene**, so Bond already
  holds 0x0E when the escort starts. The mid-escort "first Raptor drop" switch (the wedge
  trigger, fix-pickups-remainder §3) can never fire.
- A switch that DOES fire mid-walk (`0x0E→0x1C at tick 85489`, a placed weapon during the
  escort phase) walks on fine — the draw machine swaps hands and the follow continues.

The fix-pickups-remainder session branched from c72446f with the census in flight; on that
tree the spawn pickups were not yet takeable, so the first Raptor in Bond's hands came from a
guard drop mid-escort and the switch fired during the follow. On master the census fixes landed
and the placed pickups are takeable — the scenario dissolved. (Details of what I tried are in
the worklog entry; both runs are deterministic — same binary, same 319323 ticks.)

### 2. Why the wedge never existed in the game: the view-state census

Full census of every reader of the view state (`+0xF4` on the first-person anim object at
`BLData+0x7e8`) over ACTION.ELF — 12 functions:

- `Player_WeaponFiring` 0x1a5460 — gates fire/reload (start only from state 0, continue in 9/0xb).
- `Player_Weapon` 0x1a24c8 — gates the aim toggle and the ammo/mag swap.
- `Player_Move` 0x191dd0 — zeroes movement inputs **only** while a grapple gadget (0x50/0x51)
  is in view state 9. The ONLY movement tie to the view state in the game.
- `Player_Update` 0x193600 — while state 9, skips regen timers and the weapon dispatch.
- `HUD_MonitorNightSight` 0x19e9d8 — night-vision fade behind state 0.
- `Player_WeaponSelect` 0x1a2cc0 / `Player_SetWeaponAnimObj` 0x1a2e38 — the machine itself.

**Nothing gates walking, walk speed, the capsule or the walk anim on the switch states** (1
holster / 3 draw / 8 variant): `Player_SSWalk` 0x192310, `Player_SSCrouch` 0x1924c0,
`Player_ResetWalkAnim` 0x195a88, `Player_HandleJump` 0x1916b0 have zero reads of the view
state or the request byte. `Player_SetWeaponAnimObj`'s cases write only the view-model object
(`BLData+0x7e8`), BLData weapon-flow bytes and sounds — never Bond's position/matrix, never a
body-anim call from the switch path. The draw machine touches only the first-person view
model, so it cannot wedge a walker; the port's machine already mirrors this, and its only
switch-side movement interaction (fire/reload gating) matches.

### 3. The fix: gate open, request-only [M Player_EquipWeapon 0x1a8a58's tail]

`crates/nf-view/src/scene.rs` — the `if false` block replaced with the unconditional drain:

```c
// the game's tail:
if (((&DAT_002a38d2)[idx * 0x158] != '\0') &&
    (Player_IsBetterWeapon__FP7obj_tags(iVar2, iVar10) != 0)) {
  *(char *)(*(int *)(iVar2 + 0xdc) + 99) = (char)iVar10;   // anim+0x63 = id — THE ONLY effect
}
```

The take path in `drone.rs` (merged with fix-pickups-remainder) computes the request; the drain
now applies it unconditionally. **The gated code's `prev_weapon`/`prev_gadget` writes are
dropped**: `+0x930/+0x932` are `Player_WeaponChange` 0x1a6f80's cycle bookkeeping and the
auto-switch tail does not write them; the drain writes only the request byte, as the game does.
The draw machine consumes it on the next idle tick (the port's machine = the game's
`Player_SetWeaponAnimObj` default tail: `anim+0x64 = anim+0x62` → `Player_WeaponSelect`).

No `Rand_*` draws added — the seed-pinned tests moved nowhere (full suite confirms).

### 4. Tests — RED first, then green (NF_ISO, real path)

`crates/nf-view/tests/fix_auto_switch_live.rs` (new), on 07000005 through `nf_view::load` + the
dev `sg5-pickup` scenario + `Sim::step_bond` steered with real `Input` (turn/forward only; no
field pokes on the take/switch path):

- **Main — `taking_a_better_weapon_switches_through_the_draw_machine`**: Bond (spawn P2K 0x02,
  rank 16) walks to the dropped SG5 (0x16, rank 6 [L BestWeapon 0x2bede0]); the take fires the
  auto-switch, the machine holsters and draws, and settles idle: `[auto-switch] swapped to the
  SG5 (0x16) at tick 86, switch_state 0`. **Shown failing on the gated master** (both tests
  red: the request was dropped, Bond held the P2K forever), green after the fix.
- **Sister — `taking_a_worse_weapon_does_not_switch`**: after holding the SG5, a dropped
  worse-ranked Weapon (0x0A, rank 17; picked as the first non-owned worse-ranked id — a 0x06
  drop here is an Ammo take since the P2K is already owned, and `Player_EquipAmmo` 0x1a8d18 has
  no switch tail) is taken into the inventory while the held weapon and the machine state stay
  put — `Player_IsBetterWeapon` 0x1a6eb0's verdict `rank(pickup) < rank(current)` is false.
- `walkreal_07000001_live` re-run green on the final tree (the regression the task guards).

### 5. Full suite

`NF_ISO=$NF_ISO cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p
nf-audio -p nf-view -- --test-threads=1`: **198 test binaries, 197 green, 1 binary with
failures = `disc_golden` (3 tests — the allowed slim-ISO count). No other failure.**

## Shared files touched

- `crates/nf-view/src/scene.rs` — the drain block + its comment only (the task-named file; the
  change is the gate re-open itself). No `player.rs` / `pickup.rs` / `drone.rs` /
  `walker.rs` changes. New test file `fix_auto_switch_live.rs`.

## FOUND, not fixed (needs its own task — shared pickup.rs, census-coupled)

**The port takes pickups while Bond is script-disabled.** The game's touch loop requires the
player OBJ state to be 1 (playing) [M `Pickup_Update` 0x18e148:209:
`(*(short *)(iVar7 + 0xf4) == 1) && (*(char *)(iVar7 + 0xff) != 0x12)`]; the port's predicate
has no such gate, so on 07000001 the spawn's placed pickups are taken DURING the intro
cut-scene (P2K ammo @tick 368, 0x12 @776, Raptor 0x0E @817, 0x12 ammo @1000) and — with the
drain now live — Bond auto-switches hands mid-cut-scene. Whether the REAL game takes them
depends on what Bond's obj state is during the 07000001 intro — that is live-check #1 below.
This is why the old wedge scenario vanished (Bond reaches the escort already holding the
Raptor), but the predicate gap exists with the gate OFF too; it predates this task.

## Live checks for the main agent

1. **The disabled-take predicate** (decides live-check-worthy fix): PCSX2, 07000001, watch the
   placed kind-0/1 pickups at the spawn room during the intro (their taken/hidden flag at
   `obj+0xF0`, and Bond's `BLData+0x1ba` owned bytes for 0x12/0x0E). If the game does NOT take
   them before the intro tail, the port needs the obj-state==1 gate in its touch predicate
   (then re-run the pickup census + this task's tests: `fix_auto_switch_live` phase 1 uses an
   ENABLED walk-take and survives it).
2. **The re-opened drain vs the game** (carry-over from fix-pickups-remainder check #1, now
   testable on the port side too): PCSX2 07000001 — take the first Raptor guard's drop with the
   P2K held; the game should holster and draw the Raptor (flag 0x2a38d2 = 1 [L], rank 9 < 15).
   Port side: nf-view on 07000001 near the guard's death spot — the same swap must play.
3. **nf-view render/`--bond` spots**:
   - `--dev --scenario sg5-pickup` on 07000005: walk forward over the drop — the holster/draw
     cycle should play (holster ≈0.5 s, draw ≈0.5 s at 1.25x) and settle idle with the SG5
     (the main test asserts the logic; this checks the picture).
   - `--bond -2.78,6.84,-12.4,<spawn yaw>,0` on 07000001: during the intro, whether Bond's
     hands change (the disabled-take divergence visible live).

## What's left

Nothing for the gate itself. Follow-ups: the disabled-take predicate (live check #1 first);
the 07000001 placed-Raptor-at-spawn question interacts with every walkreal escort timing —
worth a dedicated pass with the predicate settled.
