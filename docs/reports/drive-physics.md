# drive-physics — the driving missions' car physics (first pass)

Branch `s/drive-physics`. Task 506: the player car's physics step from DRIVING.ELF — the tuning
parameters, the integration, and the collision-response shape, as a headless `nf_game::driving`
sim with tests that drive a scripted input and assert the decompile's numbers. No rendering.

## What's done

**docs/systems/drive-physics.md** (evidence-tagged; the map for everything below):

- The per-tick contract [M]: the frame tick `FUN_00159d10` 0x159d10 runs the physics step
  `FUN_001ff3d8` 0x1ff3d8 (passes: car↔car `FUN_001fbbd8`/`FUN_001f9d98`; car↔world
  `FUN_001fc2f0` with 8 hull points extrapolated by `vel·dt`; integration `FUN_001f89e0` → VU0
  microcode, flush `FUN_001f8cf0`; gravity/reset `FUN_001f7c60`; static bodies; then the per-car
  controller `FUN_00188200` → engine/gears/steer `FUN_0018cef8` + tyres `FUN_00190f88` whose
  forces land for the NEXT tick). The timestep is 1/60 NTSC (`FUN_002d6b90`'s {60,60,50,60}
  table; sim+0xdc0 [M `FUN_001feda0`:9-12]).
- The tuning [D+M]: **the .tun files are ASCII `KEY=VALUE` text** (no magic — the
  docs/formats/driving-viv.md "fogS/Farp magic" note misread the `fogSTART=`/`Farplanedist=`
  text keys [D]); loaded via `sprintf("%s%s\%s.tun", "data\tuning\", ns, name)` (strings
  0x36da48/0x36da58 read from the ELF) with the `\default.tun` fallback [M `FUN_00144120`], and
  per-var substring lookup + `%f` sscanf that **keeps the compiled-in value on no match**
  [M `FUN_001426a8`]. The Physics namespaces' registered globals: Rigid 0x3390d0-0x339164
  (GRAVITY −34, BODGE_PLAYER_GRAVITY −13, SLEEP_VEL 5, GROUND_FRICTION_COEFF 0.05,
  NATURAL_ANGULAR_DAMPING 0.995, M_LIMIT 100, AM_LIMIT 10, DEPTH_FORCE_SCALE 4.5, SPEED_SCALE
  0.25, SPEED/TOTAL_FORCE_LIMIT 10/40), Physical 0x331970-0x3319d4 (ROLLING_RESISTANCE 0.1,
  BASE_FRICTION_MASS 8, MAX_WHEEL_SPIN_RATE 0.05, …), Friction 0x3319d8/0x331a18 (the 13
  `WSurface_k*` rows, compiled-in all 1.0). The shipped `Physics\{Rigid,Physical,Friction}
  \default.tun` trio is byte-identical in all 8 mission VIVs [D; extracted via the repo's
  BIGF+RefPack readers], and the Rigid file's values equal the ELF's compiled-in initials.
- **Per-car physics is `data\sim\attrib\pvehicle\<car>.atr`, not a .tun** [D] — INI-style with
  typed suffixes; Bond's car is `vanquish` (MASS 1660, MAXACC 3, MAXREVACC 2.4, MAXBRAKE 3.15,
  MAXSTEERING 0.09, MODSTEERSPEED 10, MINSTEER 0.425, GEAR_LIMIT 18/29/50/54/69, GEAR_RATIO
  0.65/0.59/0.52/0.45/0.44, FRICTIONLIMITFRONT/REAR 19, TYREGRIPFACTOR 8, SPRING_STIFFNESS
  80/98, SPRING_DAMPING 400/350, WHEEL_RADIUS 0.365, BRAKING_FRICTION 0.3, HANDBRAKE_FORCE 0.1,
  wheelbase 1.9 m). The pvehicle schema + defaults registered by `FUN_001968f8` [M] (offsets
  documented, e.g. MAXACC+0x14, MAXSTEERING+0x20, GEAR_LIMIT base +0xd0 / RATIO base +0xe8
  indexed by gear 0=reverse..5).
- The engine/gearbox [M]: the auto-shift is a stateless walk (`FUN_0018aa88` 0x18aa88:36-48) —
  gear g holds |v| ∈ [LIMIT(g−1), LIMIT(g)); the down-shift hold (`:53-56`) rides the inhibit
  byte set to 0xf on every change (`FUN_0018cef8`:412) and decremented per tick (:911-912).
  **rpm = |v|·6000/LIMIT[gear]** (`:62`, the walk's last-read limit — gear 5 denominators on
  LIMIT4) + the wheelspin term; the neutral rev is **throttle·7000** (`:32`); SUB_PHYSICS cars
  scale |v|·100. The drive force [M `FUN_0018cef8`:414-421]:
  `accel = boost + throttle·(MAXACC·GEAR_RATIO[gear]) − brake·MAXBRAKE`; reverse decays
  MAXREVACC by `(speed − SPEED_CUTOFF_REVERSE)·SPEED_CUTOFF_RATE_REVERSE` (:401-407).
- The steering [M `FUN_0018cef8`:458-474]:
  `steer = input·MAXSTEERING·max(1 − min(1,(speed − MODSTEERSPEED)/25), MINSTEER)` — vanquish:
  full 0.09 rad at ≤10 m/s, floored at 0.03825 rad from 35 m/s.
- The tyres [M `FUN_00190f88`]: speed = |vel| (:394); the grip boost `(1 − min(1,speed/15))·2.5+1.0`
  (:395-397, 3.5 at rest → 1.0 at 15 m/s); the coast/handbrake drift path (:401-416, factor 2.0
  and the `min(speed,20)·0.05` scale); the slip angle clamp ±0.222 (:464-474); the axle lateral
  forces **clamped ±12.5** (:506-508) — with the force API (`FUN_001f87d8`: `acc += F·mass`
  [M], mass at instance+0x4c = the MASS attribute [M `FUN_001f6e98`:114]) these are
  **accelerations in m/s²**.
- The gravity bodge [M `FUN_001f7c60`:136-144 + the ELF strings 0x396578/0x396588 [D]]:
  **cars integrate under −13 (BODGE_PLAYER_GRAVITY); only the model name `jungle_truck` gets
  the full −34** (paradis_car takes the same type-1 path).
- **A finding that changes the model**: the whole-velocity `×0.8`-per-tick ground damping
  (`vel −= vel·4·GROUND_FRICTION_COEFF`, :174-184) is gated by `instance+0x6c > 2 && +0x6d > 3`
  (:171-173), and the car kinds are 1-3 (`FUN_001f6e98` stores param_4 ∈ {1,2,3} at +0x6d) —
  so the branch reads as the **prop** path; applied to a car it would cap speed at
  `accel·dt/0.2` ≈ 0.16 m/s, which the game plainly does not do. The port therefore applies
  ROLLING_RESISTANCE (0.1) to cars and asserts the 0.8 factor as data only.

**The port** (`crates/nf-game/src/driving.rs`, registered in lib.rs):

- `Tuning` — the .tun parser with the game's `%f`-scan and prefix-lookup semantics;
  `Rigid::game_defaults()/from_tun()` (the compiled-in initials + the tun override);
  `Friction::game_defaults()` (13 surfaces, all 1.0).
- `CarParams` — the pvehicle schema defaults [M] + `vanquish()` [D] + `parse_atr` (the INI
  form: `[section]`, `KEY.suffix=VALUE`, `//` comments, whitespace).
- `Car`/`CarInput`/`FlatWorld` — the headless step at `DT = 1/60`: the gearbox walk + RPM →
  the steering falloff → the drive/brake/reverse accel → the bicycle-model yaw rate
  (`speed·tan(steer)/wheelbase`, wheelbase from the vanquish wheel offsets) limited by the
  game's lateral clamp `12.5·boost·friction[surface]` → semi-implicit Euler integration, the
  flat surface holding the ride height. The VU0 macro's exact integration form is [S] here
  (`.vutext` @0x324800 not lifted) — flagged in the doc and in the live checks.

**Tests** (`crates/nf-game/tests/drive_physics.rs` — **13/13 green**): the .tun parser (with
the `%f`-stop and absent-key cases); the Rigid defaults + override pass; the vanquish constants
vs the .atr parser; the shift points 18/29/50/54/69 with the RPM denominators per gear (incl.
gear 5's LIMIT4); the neutral rev throttle·7000; the steering 0.09 → 0.045 (half falloff) →
0.03825 (the floor); drive 3·0.65 = 1.95 (gear 1) and 3·0.59 (gear 2), full brake −3.15; the
reverse decay 2.4 − cut·0.075 floored at 0; the gravity bodge −13 over one discrete second
(the semi-implicit Euler sum asserted exactly) vs −34 for the jungle_truck case, landing on the
flat surface; the grip boost 3.5/2.25/1.0 curve and the lateral-clamp sweep (speeds 2..60 ×
steers, |yaw_rate·speed| ≤ 12.5·boost); the 90-second scripted flat run visiting every gear in
order and coasting down gently (no ×0.8 cliff); and two real-file tests gated on the mission
VIVs (`/home/z/driving-refpack/DRIVING`, override `NF_DRIVING_VIV`) — the shipped
`vanquish.atr` and `Physics\Rigid\default.tun` parse to exactly the ported constants. **No game
bytes are committed** — the repo carries only the parsers and the constants cited from the
decompile; the disc files stay on the working set.

## Verification

- `cargo test --offline --release -p nf-game --test drive_physics` → **13 passed, 0 failed**
  (both real-file tests ran against MIS01.VIV in this sandbox).
- The fast runner on the final tree merged with origin/master (WORKLOG conflict resolved, both
  kept): `python3 tools/test/suite.py . target --only drive_physics,nf_game,nf_formats,nf_view,nf_audio,…`
  → **315 passed, 0 failed**, 0 TIMEOUT/CRASH; the build step compiled all **353** test binaries
  of the four crates clean. The remaining per-level NF_ISO binaries did not fit the window
  (this sandbox kills detached background processes — the background start died empty, the same
  experience the two previous workers recorded), so the suite ran in foreground slices.
- Commands run: the two cargo test invocations above + the suite slice. Crates touched: nf-game
  (the module + tests), nf-formats (consumed by the real-file tests, unchanged).

## What's left (the next message continues from the branch)

- **The VU0 integration macro** (`.vutext` @0x324800, entered via `_vcallms 0x568` in
  `FUN_001f89e0`): the port's Euler form is [S]; lifting the macro makes pos/orientation/vel
  bit-exact and settles the mass division and the M_LIMIT/AM_LIMIT clamp form.
- **Car↔world collision against the track data** (`FUN_001fc2f0` + the road quads): the port's
  `FlatWorld` is the stand-in; the response functions' forms are documented (`FUN_001fb318`/
  `FUN_001fade8`/`FUN_001fc198`, the car↔car impulse `FUN_001f9d98` with the
  SPEED_SCALE/DEPTH_FORCE_SCALE/TOTAL_FORCE_LIMIT tunings) but not wired to level data.
- **Suspension** (`SPRING_*` attrs + the 18 world colliders @0x331a60): the spring-damper form
  `F = fps·Δv·compression` [M `FUN_00190f88`:870-975] is documented; the per-collider table and
  its consumer are not lifted.
- **Handbrake** (`HANDBRAKE_FORCE` 0.1 / `TOTAL_HANDBRAKE_FRICTION_SCALE` 0.15 + the +0x29b
  path `FUN_0018cef8`:288-309): parsed, not yet modelled in the step.
- The Friction tun's name mismatch (the file's `frictionCoefficients[WSurface_k*]` keys vs the
  registered `friction[k*]` — live check below) and the exact decay of the shift-inhibit hold.

## Live checks for the main agent

1. **Top speed + shift points on a straight** (any driving mission, PCSX2): watch
   `DAT_00339f50`'s car — the actor+0x288 |velocity| float for the player car (find it via the
   instance at 0x339190+id·0x80 +0x20, |v|). Expected from the port: the gear byte (actor+0x2d8)
   steps 1→2 at 18 m/s, →3 at 29, →4 at 50, →5 at 54; the run tops out where the decompile's
   drive force balances drag — the measured top speed pins the missing drag term.
2. **The ground-damping predicate** — with `tools/trace/gdbwatch.py` on instance+0x20 (the
   velocity vec3) of the player car: record the per-tick delta while coasting at speed. If the
   velocity decays ~0.8× per tick the damping DOES hit cars (and the game's tyre forces
   overwhelm it — the port must then model that too); if it decays ~1-2 % per tick, the prop
   reading stands and ROLLING_RESISTANCE is the coast-down.
3. **Steering/turn rate** — hold full steer at a steady 20 m/s and 40 m/s, record the yaw rate
   (the class matrix's rotation per tick, class+0x00-0x3f at the sim's class array
   `sim+0x24+id·0x500`). Expected shape: the yaw rate·speed (lateral accel) saturates at
   12.5·boost m/s²; the measured pair pins the bicycle-model constant the port chose [S].
4. **The bodge gravity on camera** — drive off any jump (Paris mis01's ramps): the car should
   fall noticeably slower than a real 1 g (−13 vs −34 m/s² — the port asserts the discrete
   fall). A screenshot pair (apex → landing) pins it.
5. **The Friction tun's dead keys** — gdbwatch the `friction[kGRAVEL]` global 0x3319d8+2·4
   while driving on gravel: if it reads 1.0 (the compiled-in default) the shipped file's 0.9 is
   dead in this build [S]; if 0.9, a normalizer exists and the port's Friction::from_tun must
   match it.

## ntfy failed

(all three fallbacks, 2026-10-15 ~21:20 UTC — the same daily quota the previous task hit)
1. `curl …` → `{"code":42908,"http":429,"error":"limit reached: daily message quota reached; …"}`
2. python3 urllib → `urllib.error.HTTPError: HTTP Error 429: Too Many Requests`
3. `wget -S` probe → `HTTP/1.1 429 Too Many Requests`
