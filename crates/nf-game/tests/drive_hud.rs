//! drive-hud-data numbers-only tests: the driving HUD's per-frame snapshot follows the sim.
//! The vehicle group is wired to `driving::Car` (506's physics) — every assertion cites the
//! DRIVING.ELF address; the mission groups carry the E-command semantics at their proven ids
//! (docs/atlas/driving-elf.md, the dispatch table 0x36fda0).

use nf_game::driving::{Car, CarInput, CarParams, FlatWorld, Rigid, DT};
use nf_game::driving_hud::{CarReadout, DriveHud, MissionTimer, Objective, ObjectiveState, WeaponReadout};

fn vanquish() -> Car {
    Car::new(CarParams::vanquish(), [0.0; 3], 0.0)
}

/// The readout is a per-frame copy of the car the HUD would read [M 0x18aa88]: 600 full-
/// throttle ticks on the flat test surface, snapshot refreshed every tick — speed, gear,
/// rpm, position and heading must match the car at every tick, and the gearbox must climb
/// on the vanquish limits (GEAR_LIMIT1=18, GEAR_LIMIT2=29 [D vanquish.atr]).
#[test]
fn car_readout_follows_the_sim_tick_by_tick() {
    let mut car = vanquish();
    let mut hud = DriveHud::new();
    hud.observe(&car);
    assert_eq!(hud.car, CarReadout { speed: 0.0, gear: 1, rpm: 0.0, reversing: false, pos: [0.0; 3], yaw: 0.0 });

    let world = FlatWorld::paved();
    let rigid = Rigid::game_defaults();
    let input = CarInput { throttle: 1.0, brake: 0.0, steer: 0.0, handbrake: false };
    for tick in 0..600 {
        car.step(&input, &world, &rigid, DT);
        hud.observe(&car);
        assert_eq!(hud.car.speed, car.speed, "tick {tick}: the readout speed IS the car's |vel| [M 0x18aa88:26-29]");
        assert_eq!(hud.car.gear, car.gear, "tick {tick}: the readout gear IS actor+0x2d8 [M]");
        assert_eq!(hud.car.rpm, car.rpm, "tick {tick}: the readout rpm IS FUN_0018aa88's [M :62]");
        assert_eq!(hud.car.pos, car.pos, "tick {tick}: instance+0x10 [M]");
        assert_eq!(hud.car.yaw, car.yaw, "tick {tick}: the class yaw [M]");
    }
    assert!(hud.car.speed > 15.0, "600 ticks of full throttle reach gear-2 speeds — got {}", hud.car.speed);
    assert!(hud.car.gear >= 2, "the gearbox climbed past gear 1 — got {}", hud.car.gear);
    assert!(hud.car.rpm > 0.0);
}

/// The gearbox boundary the readout reports [M 0x18aa88:36-48]: 18.0 (== LIMIT1) shifts to 2,
/// 17.9 stays in 1, and the rpm is speed·6000/LIMIT[gear] [M :62].
#[test]
fn car_readout_reports_the_gear_boundaries_exactly() {
    let mut car = vanquish();
    car.vel = [18.0, 0.0, 0.0];
    car.update_rpm_gear(0.0, true);
    let hud = CarReadout::of(&car);
    assert_eq!(hud.gear, 2, "speed == GEAR_LIMIT1 shifts up [M 0x18aa88:36-48]");
    assert!((hud.rpm - 18.0 * 6000.0 / 29.0).abs() < 1e-3, "rpm = speed·6000/LIMIT2 [M :62] — got {}", hud.rpm);

    let mut car = vanquish();
    car.vel = [17.9, 0.0, 0.0];
    car.update_rpm_gear(0.0, true);
    let hud = CarReadout::of(&car);
    assert_eq!(hud.gear, 1, "below the limit the gear holds [M]");
    assert!((hud.rpm - 17.9 * 6000.0 / 18.0).abs() < 1e-3, "rpm = speed·6000/LIMIT1 [M :62] — got {}", hud.rpm);

    // The neutral rev [M 0x18aa88:32]: not-yet-driving → rpm = throttle·7000, gear stays.
    let mut car = vanquish();
    car.update_rpm_gear(0.25, false);
    let hud = CarReadout::of(&car);
    assert!((hud.rpm - 0.25 * 7000.0).abs() < 1e-4, "the neutral rev is throttle·7000 [M :32]");
    assert_eq!(hud.gear, 1);
}

/// The mission timer ticks at the fixed 60 Hz and obeys its commands [M ids: ESetTimer 507/509,
/// EStartTimer 381/383, EStopTimer 357/359, ETimerDrawOn 309/311, ETimerDrawOff 315/317].
#[test]
fn mission_timer_follows_the_commands_and_the_fixed_tick() {
    let mut t = MissionTimer::new(90.0);
    assert_eq!((t.remaining, t.initial), (90.0, 90.0));
    assert!(!t.running && !t.draw);
    t.set_draw(true);
    assert!(t.draw, "ETimerDrawOn [M id 309/311]");
    t.set_draw(false);
    assert!(!t.draw, "ETimerDrawOff [M id 315/317]");
    t.set_draw(true);

    t.start();
    for _ in 0..60 {
        t.update(DT);
    }
    assert!((t.remaining - 89.0).abs() < 1e-2,
        "one second of the fixed 1/60 s tick [M 0x2d6b90; 60·(1/60) accumulates in f32] — got {}", t.remaining);

    t.stop();
    for _ in 0..60 {
        t.update(DT);
    }
    assert!((t.remaining - 89.0).abs() < 1e-2, "EStopTimer holds the value [M id 357/359]");

    t.set(30.0);
    assert_eq!((t.remaining, t.initial), (30.0, 30.0), "ESetTimer re-arms [M id 507/509]");
    t.start();
    for _ in 0..30 * 60 + 60 {
        t.update(DT);
    }
    // unproven-tests-gameplay: the old "floors at zero" was [S]; the game's running timer is
    // `value + rate*(tick_now - t0)` with NO clamp — only the display is gated [M DRIVING.ELF the
    // timer slot at world+0x150+index*0x18; FUN_0023e580 0x23e580:58]. 30 s + 1 s of ticks past it.
    assert!((t.remaining + 1.0).abs() < 1e-2, "the countdown runs past zero unclamped [M]: {}", t.remaining);
}

/// The objective records keep the game's 4-param shape [M 00151510_FUN_00151510.c: the record
/// is {vtable 0x370e70, p1..p4}] and the commands' states [M ids 225/226/227/228].
#[test]
fn objective_records_keep_the_game_shape_and_states() {
    let mut hud = DriveHud::new();
    hud.objective_add([1, 2, 3, 4]);
    hud.objective_add([5, 6, 7, 8]);
    assert_eq!(hud.objectives[0], Objective { params: [1, 2, 3, 4], state: ObjectiveState::Added });
    hud.objective_pass(1);
    assert_eq!(hud.objectives[1].state, ObjectiveState::Passed, "EObjectivePass [M id 228]");
    hud.objective_fail(1);
    assert_eq!(hud.objectives[1].state, ObjectiveState::Failed, "EObjectiveFail [M id 226]");
    hud.objective_insert(0, [9, 8, 7, 6]);
    assert_eq!(hud.objectives[0], Objective { params: [9, 8, 7, 6], state: ObjectiveState::Inserted },
        "EObjectiveInsert lands at the front [M id 227, position semantics S]");
    assert_eq!(hud.objectives.len(), 3);
}

/// ETargetBeaconOnOff (321/323) toggles in place — the position stands [M the toggle;
/// the target struct is open, see the report's live checks].
#[test]
fn beacon_toggles_in_place() {
    let mut hud = DriveHud::new();
    hud.beacons.push(nf_game::driving_hud::Beacon { pos: [1.0, 0.0, 2.0], on: false });
    hud.beacon_toggle(0);
    assert!(hud.beacons[0].on, "ETargetBeaconOnOff [M id 321/323]");
    hud.beacon_toggle(0);
    assert!(!hud.beacons[0].on);
    assert_eq!(hud.beacons[0].pos, [1.0, 0.0, 2.0]);
}

/// ESetVisualDamage (188) stores the record's five params verbatim [M 00159638's record
/// shape {vtable 0x370558, p1..p5}]; the vehicle refresh never touches the mission groups.
#[test]
fn visual_damage_stores_verbatim_and_observe_leaves_the_mission_state() {
    let mut hud = DriveHud::new();
    hud.visual_damage_set([0.25, 0.5, 0.75, 1.0, 0.0]);
    assert_eq!(hud.visual_damage, [0.25, 0.5, 0.75, 1.0, 0.0], "the 5-param record, verbatim [M]");
    hud.timer.set(42.0);
    hud.timer.start();
    hud.timer.set_draw(true);
    hud.weapon = WeaponReadout { weapon: 7, ammo: 120 };

    // A frame with the car (the mission state is not the car's to touch [M — the HUD reads
    // the car and the mission state from different systems]).
    let mut car = vanquish();
    car.vel = [5.0, 0.0, 0.0];
    car.update_rpm_gear(1.0, true);
    hud.observe(&car);
    assert_eq!(hud.car.speed, 5.0);
    assert_eq!(hud.visual_damage, [0.25, 0.5, 0.75, 1.0, 0.0]);
    assert_eq!(hud.timer.remaining, 42.0);
    assert!(hud.timer.running && hud.timer.draw);
    assert_eq!(hud.weapon, WeaponReadout { weapon: 7, ammo: 120 });
    assert_eq!(hud.health, None, "the health source is unproven [M-negative FUN_001862c0]");
}
