//! fix-spawner-p7 unit tests: the spawner's p7 KILL channel against `DroneSpawner_Control` 0x137e68
//! (the decompile + the disasm 0x137fbc-0x1380b4) and the live measurement (docs/fields/
//! hitdata-spawner-ally.md "Live result": 07000005's gate spawner removed both spawned guards when
//! ch 0x43 latched). Render-independent.

use nf_game::spawners::{Action, Spawner};
use nf_game::triggers::Channels;

fn spawner_with_kill(kill: u32) -> Spawner {
    spawner_params(|k| match k { 7 => kill, _ => 0 })
}

/// mode 2, group 1, max 2, 1 wave, no trigger; p5/p6/p7 come from `extra`.
fn spawner_params(extra: impl Fn(u32) -> u32) -> Spawner {
    Spawner::new(0, |k| match k {
        0 => 2,  // mode: spawn while alive < max
        1 => 1,  // group
        2 => 2,  // max alive
        3 => 1,  // waves
        4 => 0,  // no trigger (active at load)
        _ => extra(k),
    }, vec![0, 1], &Channels::default()).unwrap()
}

/// Two spawned drones; the closure set says both are alive; `drawn` controls the obj+0xFC gate.
fn setup(sp: &mut Spawner, ch: &mut Channels, drawn: [bool; 2]) {
    let mut rng_out = 0u32;
    let mut rand = |_: u32| { rng_out += 1; rng_out - 1 };
    let alive = |_: usize| true;
    let drawnf = |i: usize| drawn[i];
    let spot = |_: usize, _: f32| true;
    // The no-trigger spawner is active at load: two updates spawn both templates (one per tick).
    let a = sp.update(ch, 1, 0x0700_0005, alive, drawnf, spot, &mut rand);
    assert!(matches!(a, Action::Spawn(0) | Action::Spawn(1)), "first spawn, got {a:?}");
    let a = sp.update(ch, 2, 0x0700_0005, alive, drawnf, spot, &mut rand);
    assert!(matches!(a, Action::Spawn(_) ), "second spawn, got {a:?}");
    assert_eq!(sp.spawned.len(), 2);
}

/// The live measurement: on 07000005 (a kill-all level) the p7 latch removes BOTH spawned drones,
/// zeroes the slots and disables the spawner in the same pass [L: 15 → 13 drones, +0x1E = 0,
/// obj+0xFE |= 1].
#[test]
fn p7_on_a_kill_all_level_removes_every_spawned_drone_and_the_spawner() {
    let mut sp = spawner_with_kill(0x43);
    let mut ch = Channels::default();
    setup(&mut sp, &mut ch, [true, true]); // even drawn ones go on a kill-all level
    ch.set(0x43, 1, 3);
    let mut rand = |_: u32| 0u32;
    let a = sp.update(&mut ch, 4, 0x0700_0005, |_: usize| true, |_: usize| true, |_: usize, _: f32| true, &mut rand);
    match a {
        Action::Remove(v) => assert_eq!(v.len(), 2, "both spawned drones removed [L 15 → 13]"),
        other => panic!("expected Remove, got {other:?}"),
    }
    assert!(sp.gone, "the spawner deletes itself [M 0x1380b4-ish: obj+0xFE |= 1]");
    assert!(!sp.active, "+0x1E = 0 [M 0x137e68:287]");
    assert!(sp.spawned.is_empty(), "the slots zeroed [M `sw $zero, ($v1)` 0x138094]");
    // And it never spawns again.
    let a = sp.update(&mut ch, 5, 0x0700_0005, |_: usize| true, |_: usize| true, |_: usize, _: f32| true, &mut rand);
    assert_eq!(a, Action::None, "the latched spawner is gone");
}

/// The keep levels: 0x07000011 removes only the drones whose `obj+0xFC` byte is 0 (not drawn
/// recently) and keeps the spawner while any drawn drone remains [M 0x138054-0x138098; the disasm:
/// `lbu $v0, 0xfc($s0)`, `bnez` → the kept count, the spawner returns when the count != 0].
#[test]
fn p7_on_a_keep_level_removes_only_the_undrawn_and_keeps_the_spawner() {
    let mut sp = spawner_with_kill(0x1d);
    let mut ch = Channels::default();
    setup(&mut sp, &mut ch, [true, false]);
    ch.set(0x1d, 1, 3);
    let mut rand = |_: u32| 0u32;
    let drawn = [true, false];
    let a = sp.update(&mut ch, 4, 0x0700_0011, |_: usize| true, |i| drawn[i], |_: usize, _: f32| true, &mut rand);
    match a {
        Action::Remove(v) => assert_eq!(v, vec![1], "only the undrawn drone (obj+0xFC == 0) is removed"),
        other => panic!("expected Remove, got {other:?}"),
    }
    assert!(!sp.gone, "the spawner stays while a drawn drone remains [M 0x138081: `if (sVar8 != 0) return`]");
    // The survivor stays: the next update removes nothing more while it is drawn.
    let a = sp.update(&mut ch, 5, 0x0700_0011, |_: usize| true, |_: usize| true, |_: usize, _: f32| true, &mut rand);
    assert_eq!(a, Action::Remove(Vec::new()), "nothing more to remove");
    assert!(!sp.gone);
    // Drawn → undrawn: the next pass removes it too and the spawner deletes itself.
    let a = sp.update(&mut ch, 6, 0x0700_0011, |_: usize| false, |_: usize| false, |_: usize, _: f32| true, &mut rand);
    assert_eq!(a, Action::Remove(Vec::new()));
    assert!(sp.gone, "once none remain the spawner deletes itself [M 0x137e68:281-286]");
}

/// The level split (the disasm's `s7` through the always-executed delay slot at 0x137fe8):
/// kill-all for < 0x07000011, 0x07000014/15 and > 0x07000016 except 0x0700004A; the obj+0xFC-gated
/// keep for 0x07000011..13, 0x07000016 and 0x0700004A.
#[test]
fn the_kill_all_level_split_matches_the_disasm() {
    let kill_all = |level: u32| {
        let mut sp = spawner_with_kill(0x43);
        let mut ch = Channels::default();
        setup(&mut sp, &mut ch, [true, true]);
        ch.set(0x43, 1, 3);
        let mut rand = |_: u32| 0u32;
        let a = sp.update(&mut ch, 4, level, |_: usize| true, |_: usize| true, |_: usize, _: f32| true, &mut rand);
        let removed = matches!(a, Action::Remove(ref v) if v.len() == 2);
        removed && sp.gone // a drawn pair survives only on the keep levels
    };
    for level in [0x0700_0005u32, 0x0700_0001, 0x0700_0010] {
        assert!(kill_all(level), "level {level:#x} < 0x07000011: kill-all");
    }
    for level in [0x0700_0014u32, 0x0700_0015] {
        assert!(kill_all(level), "level {level:#x}: kill-all");
    }
    for level in [0x0700_0017u32, 0x0700_0016 + 0x100, 0x0700_004b] {
        assert!(kill_all(level), "level {level:#x} > 0x07000016 (not 0x4a): kill-all");
    }
    for level in [0x0700_0011u32, 0x0700_0012, 0x0700_0013, 0x0700_0016, 0x0700_004a] {
        assert!(!kill_all(level), "level {level:#x}: the obj+0xFC-gated keep (a drawn pair survives)");
    }
}

/// The latch fires only when `Control` runs: the port's `npc_enabled` gate sits at the call site
/// (drone.rs), which the NF_ISO tests drive; here the channel-on-while-gated shape: the latch does
/// not skip the stop channel — p5 with the latch pauses (returns) instead of deleting [M 0x137e68:79-81].
#[test]
fn the_stop_channel_pauses_a_latched_spawner_and_deletes_an_unlatched_one() {
    let mut sp = spawner_params(|k| match k { 7 => 0x43, 5 => 0x16, _ => 0 });
    let mut ch = Channels::default();
    setup(&mut sp, &mut ch, [true, true]);
    ch.set(0x43, 1, 3);
    ch.set(0x16, 1, 3); // the stop channel
    let mut rand = |_: u32| 0u32;
    let a = sp.update(&mut ch, 4, 0x0700_0005, |_: usize| true, |_: usize| true, |_: usize, _: f32| true, &mut rand);
    assert_eq!(a, Action::None, "latched + stop: un-trigger and pause [M 0x137e68:79-81]");
    assert!(!sp.gone, "the latched spawner survives the stop channel this tick");
    assert!(!sp.active, "+0x1E = 0");
    // The next tick (stop still on, latch still set): still a pause.
    let a = sp.update(&mut ch, 5, 0x0700_0005, |_: usize| true, |_: usize| true, |_: usize, _: f32| true, &mut rand);
    assert_eq!(a, Action::None);
    assert!(!sp.gone);
    // An un-latched spawner with the stop channel deletes itself [M 0x137e68:82].
    let mut sp2 = spawner_params(|k| match k { 5 => 0x16, _ => 0 });
    let mut ch2 = Channels::default();
    setup(&mut sp2, &mut ch2, [true, true]);
    ch2.set(0x16, 1, 3);
    let a = sp2.update(&mut ch2, 4, 0x0700_0005, |_: usize| true, |_: usize| true, |_: usize, _: f32| true, &mut rand);
    assert_eq!(a, Action::None);
    assert!(sp2.gone, "the un-latched spawner deletes itself on the stop channel");
}

/// The done path: the done-channel write waits for the spawned drones to die when p6 OR p7 is set;
/// with both clear the spawner disables immediately [M 0x137e68:126-143].
#[test]
fn the_done_path_waits_for_the_spawned_when_p6_or_p7_is_set() {
    // p7 set (no done channel): the spawner waits while a spawned drone lives.
    let mut sp = spawner_with_kill(0x43);
    let mut ch = Channels::default();
    setup(&mut sp, &mut ch, [true, true]);
    let mut rand = |_: u32| 0u32;
    // waves = 1 and both templates used → the done check.
    let a = sp.update(&mut ch, 4, 0x0700_0005, |_: usize| true, |_: usize| true, |_: usize, _: f32| true, &mut rand);
    assert_eq!(a, Action::None, "p7 set: wait for the spawned drones");
    assert!(!sp.gone, "still waiting");
    // p6 and p7 clear: the immediate disable.
    let mut sp = Spawner::new(0, |k| match k { 0 => 2, 1 => 1, 2 => 2, 3 => 1, _ => 0 }, vec![0, 1], &Channels::default()).unwrap();
    let mut ch = Channels::default();
    setup(&mut sp, &mut ch, [true, true]);
    let a = sp.update(&mut ch, 4, 0x0700_0005, |_: usize| true, |_: usize| true, |_: usize, _: f32| true, &mut rand);
    assert_eq!(a, Action::None);
    assert!(sp.gone, "p6 == 0 && p7 == 0: the immediate disable [M 0x137e68:127-129]");
}
