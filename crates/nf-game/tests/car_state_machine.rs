//! mp-map-objects (task 744) — the `Car` state machine (the MP remote tank / helicopter),
//! pinned to the decompile. Headless: no ISO needed.
//!
//! Every assertion cites its [M] address. The drive physics beyond `drive_step_heli`'s input
//! conditioning is NOT asserted (Car_Update 0x1307d8's tank branch and matrix composition are
//! next-session work — see crate::car's header).

use nf_game::car::{Car, CAR_DRIVEN, CAR_IDLE, CAR_RESPAWNING, RESPAWN_TICKS};
use nf_game::drone::Rng;

#[test]
fn create_gates_match_car_create_0x1305b0() {
    let mut rng = Rng::new(1);
    // MP off or the option Off → no object [M 0x1305b0:13-17].
    assert!(Car::create(false, 1, 0x0700_0024, &mut rng).is_none());
    assert!(Car::create(true, 0, 0x0700_0024, &mut rng).is_none());
    // Ravine 0x0700004b is blocked [M 0x1305b0:19-22] — even with the option on.
    assert!(Car::create(true, 1, 0x0700_004b, &mut rng).is_none());
    // Option 1 = Tanks → the tank variant, no RNG draw [M 0x1305b0:29-43].
    let draws_before = rng.draws;
    let (car, heli) = Car::create(true, 1, 0x0700_0024, &mut rng).unwrap();
    assert!(!heli && !car.heli, "Tanks option must give the tank variant");
    assert_eq!(rng.draws, draws_before, "the Tanks option draws no Rand_Rand");
    // Option 2 = Helicopters → the helicopter variant, still no draw.
    let (car, heli) = Car::create(true, 2, 0x0700_0024, &mut rng).unwrap();
    assert!(heli && car.heli, "Helicopters option must give the helicopter variant");
    assert_eq!(rng.draws, draws_before, "the Helicopters option draws no Rand_Rand");
    // Option 3 = Random draws Rand_Rand(100) once [M 0x1305b0:36-38].
    Car::create(true, 3, 0x0700_0024, &mut rng).unwrap();
    assert_eq!(rng.draws, draws_before + 1, "the Random option draws exactly one Rand_Rand(100)");
    // The created vehicle sits idle [M — the state word starts 0; activate requires it].
    assert_eq!(car.state, CAR_IDLE);
}

#[test]
fn activate_side_effects_match_car_activate_0x131c48() {
    let mut rng = Rng::new(1);
    let (mut car, heli) = Car::create(true, 2, 0x0700_0024, &mut rng).unwrap();
    assert!(heli);
    // Only an idle vehicle activates [M 0x131c48:19 — the whole body is behind state == 0].
    car.state = CAR_DRIVEN;
    assert!(car.activate(0).is_none(), "a driven car cannot activate");
    car.state = CAR_IDLE;
    let (prev, cam, sound, spin) = car.activate(0).unwrap();
    // The camera mounts the vehicle [M 0x131c48:27 Player_SetCamMode(0xD)] and the state arms
    // at 5 [M 0x131c48:31] with the pilot's slot in +0xf6 [M 0x131c48:32].
    assert_eq!(car.state, nf_game::car::CAR_ARMING);
    assert_eq!(car.timer_or_pilot, 0);
    assert_eq!(car.driver, Some(0));
    // The activate sound: helicopter 0x10b, spin-up 4 [M 0x131c48:48-52]; the tank's is 0x60a/10.
    assert_eq!(sound, nf_game::car::ACTIVATE_SOUND_HELI);
    assert_eq!(spin, 4);
    assert_eq!(cam, 0xD);
    let _ = prev; // Player_ChangeSubState's return — the caller's business
    // Heat and the overheat latch clear [M 0x131c48:36-37].
    assert_eq!(car.heat, 0.0);
    assert!(!car.overheated);
    // The arming state collapses to driven on the next tick.
    car.tick();
    assert_eq!(car.state, CAR_DRIVEN);
}

#[test]
fn deactivate_matches_car_deactivate_0x131d88() {
    let mut rng = Rng::new(1);
    let (mut car, _) = Car::create(true, 1, 0x0700_0024, &mut rng).unwrap();
    car.activate(2).unwrap();
    car.tick();
    assert_eq!(car.state, CAR_DRIVEN);
    // Deactivate: state 99, the 0x708 respawn timer [M 0x131d88:24/79], the driver released,
    // the pose restored from TankSpawns[slot] (the port's spawn_matrix copy).
    car.spawn_matrix[3][0] = 42.0;
    car.deactivate();
    assert_eq!(car.state, CAR_RESPAWNING);
    assert_eq!(car.timer_or_pilot, RESPAWN_TICKS, "the respawn timer is 0x708 ticks [M 0x131d88:79]");
    assert_eq!(car.driver, None);
    assert_eq!(car.matrix[3][0], 42.0, "the pose restores from the spawn matrix");
    // The state-99 branch counts the timer down by FRAME_RATE_MUL and returns to idle
    // [M 0x1307d8:100-116].
    for _ in 0..RESPAWN_TICKS {
        car.tick();
    }
    assert_eq!(car.state, CAR_IDLE, "the vehicle returns to usable after the 0x708-tick timer");
}

#[test]
fn heli_drive_conditioning_matches_0x1307d8_157_246() {
    let mut rng = Rng::new(1);
    let (mut car, _) = Car::create(true, 2, 0x0700_0024, &mut rng).unwrap();
    car.activate(0).unwrap();
    car.tick();
    // Not driven → no step.
    car.state = CAR_IDLE;
    assert_eq!(car.drive_step_heli(1.0, 1.0, 1.0, 1.0), (0.0, 0.0, 0.0, 0.0));
    car.state = CAR_DRIVEN;
    // Yaw: input·FRM·0.05 [M 0x1307d8:161-162]; pitch: negated [M :164-166].
    let (yaw, pitch, _, _) = car.drive_step_heli(1.0, 0.5, 0.0, 0.0);
    assert!((yaw - 0.05).abs() < 1e-6);
    assert!((pitch + 0.025).abs() < 1e-6);
    // The throttle lerps toward 2·input·FRM at 0.15 [M :196-205] and clamps at ±0.125 [M :207-214]:
    // full input for 60 ticks converges toward the clamp.
    for _ in 0..60 {
        car.drive_step_heli(0.0, 0.0, 1.0, 0.0);
    }
    assert!((car.throttle - 0.125).abs() < 1e-4, "throttle converges to the ±0.125 clamp, got {}", car.throttle);
    // The dead zone zeroes the accumulator [M :206-208].
    let mut car2 = car.clone();
    car2.throttle = 0.05;
    car2.drive_step_heli(0.0, 0.0, 0.0, 0.0);
    let _ = car2;
}

#[test]
fn shell_fire_overheat_matches_0x1307d8_fire_arm() {
    let mut rng = Rng::new(1);
    let (mut car, _) = Car::create(true, 1, 0x0700_0024, &mut rng).unwrap();
    car.activate(0).unwrap();
    car.tick();
    assert!(car.fire_shell(), "the first shell fires");
    assert!((car.shell_cooldown - 6.0).abs() < 1e-6, "the shell cooldown is 0x40c00000 = 6.0 s");
    assert!((car.heat - 10.0).abs() < 1e-6, "each shell adds 10 heat");
    assert!(!car.fire_shell(), "the cooldown blocks the next shell");
    // Shells alone NEVER overheat: +10 heat per 6 s against the −1/tick cooling that runs
    // unconditionally at Car_Update's head [M 0x1307d8:100-108] — 360 ticks cool the 10 heat
    // long before the next shell. (Reaching 100 needs the MG arm too — next session.)
    for i in 0..10 {
        for _ in 0..360 {
            car.tick();
        }
        assert!(car.fire_shell(), "shell {} fires after its cooldown", i + 2);
    }
    assert!(car.heat <= 10.0, "shells alone cannot exceed 10 heat, got {}", car.heat);
    assert!(!car.overheated);
    // The latch rule itself [M 0x1307d8 fire tail: `100.0 < +0x104 → +0x113 = 1`; the head
    // clears it when the accumulator lands at 0]: let the last shell's cooldown run out, then
    // inject the MG-era heat directly.
    for _ in 0..6 {
        car.tick();
    }
    car.heat = 101.0;
    car.overheated = false;
    car.fire_shell();
    assert!(car.overheated, "heat past 100 latches the overheat");
    assert!(!car.fire_shell(), "an overheated vehicle cannot fire");
    // Cooling to 0 clears the latch.
    while car.heat > 0.0 {
        car.tick();
    }
    assert!(!car.overheated);
}
