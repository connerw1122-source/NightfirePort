//! fix-06-driver: the 07000006 friendly truck driver (skin 0x05000035, dtype 13 → dtypenew 0x17,
//! team 3), driven through the real paths (`nf_view::load` + `Sim::step_bond`). Playtest
//! 1011-1535 (06): "he does get scared, but because of where he's standing, he like crouches on
//! top of the boxes that he's next to... Also, you can still shoot him, and he like flies back
//! like a guard would with the same guard death sound."
//!
//! The chain of facts (each cited at the asserts):
//! - The game's driver states have NO msg-0x14 (SOUND) arm: `TruckDriverIdle` 0x160ed0 routes
//!   msgs 0x11/0x12/0x15/0x16/0x1e to `DroneVision_EnemyAlerts` and has no 0x14 case (default →
//!   unhandled) [M 0x160ed0:95-103]; `TruckDriverInit` 0x160bd0's arm is 0x11/0x12/0x13/0x16/0x1e
//!   → 0x33 [M 0x160bd0:97-105]; `TruckDriverInitAlert` 0x160d98 handles no sound msg at all. The
//!   port's extra SOUND halves ran the guard heard-noise chain (0xa2-0xa5) on him, whose ATTACK
//!   path drew a weapon (DRAW_WEAPON 0x66 EXIT: weapon 6, anim set 4 — the guard's set): the
//!   playtest's "flies back like a guard with the same guard death sound". Fixed: the driver never
//!   hears gunfire; his scare paths are NearArmedBond (Idle case 3, [M 0x160ed0:60-70]), the
//!   bullets, the screams/shouts and the alarm — exactly the game's.
//! - Shooting him IS game-accurate: `NDrone2_DealWithObjHit` case 5 — the shooter is the player
//!   (obj type 3/0x36 → hittable, no team gate; the team gate (drone+0x44 2..3) only stops AI
//!   shooters) [M 0x13fff0:181-235], and his death runs the shared death path with his own line:
//!   `NDrone2_DeathTalk` 0x13eff8 defaults an unlisted voice type to line 1 (the driver's voice
//!   type is 0xf — skin 0x35 [M 0x14b300]) while the guards' (type 1) get line 4 [M 0x13eff8:16-31].
//! - The scare: `TruckDriverIdle` case 3 NearArmedBond → CivilianScared 0x12 [M 0x160ed0] whose
//!   ENTER (alt mode ≠ 7 — the driver's is 0 [L probe]) goes straight to CivilianHiding 0x13, whose
//!   ENTER plays the cower 0x19 [M 0x15d3a0:31-43, 0x15d520:17-22]; the cower anim (aset 12 → danim
//!   159 → script 060000cd → seq 04000048) has a constant x/z root [D nf-extract] — he cowers IN
//!   PLACE, and the floor snap (`Drone_FeetOnPoint` 0x140660, the port's pos.y = floor + hip 1.0)
//!   keeps his feet on the courtyard floor.
//!
//! Opt-in via NF_ISO.
//! Run: `NF_ISO=… cargo test -p nf-view --test fix_06_driver_live -- --nocapture --test-threads=1`

use nf_game::drone::state;
use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn tick_idle(sim: &mut nf_view::scene::Sim, n: u32) {
    for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
}

fn stand_armed_at(sim: &mut nf_view::scene::Sim, at: [f32; 3], to: [f32; 3]) {
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, at, 0.0);
    sim.gun.weapon = 2;
    sim.gun.switch_request = 2;
    sim.gun.switch_state = 0;
    let e = sim.bond.head();
    let d = [to[0] - e[0], to[1] - e[1], to[2] - e[2]];
    sim.bond.yaw = d[0].atan2(d[2]);
}

const NOISE_STATES: [u8; 4] = [0xa2, 0xa3, 0xa4, 0xa5];

/// The shot: gunfire must never put the driver into the guard heard-noise chain or hand him a
/// weapon — he keeps his civilian anim set, dies through the shared death path with his own death
/// line, and his protected-actor death raises the civilians channel. Sister: a guard in the same
/// level still reacts to the same gunfire, and the driver's near-scare path still works.
#[test]
fn gunfire_never_arms_the_driver_and_his_death_is_his_own() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000006.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    let di = sim.drones.list.iter().position(|dr| dr.skin == 0x0500_0035).expect("the 07000006 driver (skin 0x05000035)");
    tick_idle(sim, 920); // the 880-frame sit-in-truck script [D nf-extract 0600011d] ends into TruckDriverIdle
    assert_eq!(sim.drones.list[di].state, state::TRUCK_DRIVER_IDLE, "the driver idles before the shots");
    assert_eq!(sim.drones.list[di].anim_set, 12, "the driver plays the civilian anim set");
    assert_eq!(sim.drones.list[di].team, 3, "dtype 13 → team 3 [M GetDroneTypeAttackTypeFriend 0x14cbd8]");
    let dp = sim.drones.list[di].pos;

    // Stand armed Bond 8 units south (the open side — the crate stack shields him from the east)
    // and shoot him (re-aim each tick at his live pos, the player's own behaviour).
    stand_armed_at(sim, [dp[0], dp[1], dp[2] + 8.0], dp);
    let mut drew_weapon = false;
    let mut heard_noise = false;
    let mut left_civilian_set = false;
    let mut death_seen = false;
    let mut guard_reacted = false;
    for _ in 0..900u32 {
        let tp = sim.drones.list[di].pos;
        let e = sim.bond.head();
        // Aim at the live centroid of his hit boxes (the cower shifts the body with the pose).
        {
            let d = sim.data();
            let boxes = sim.drones.list[di].hit_boxes(&d);
            let (mut cx, mut cy, mut cz) = (0.0f32, 0.0f32, 0.0f32);
            for b in &boxes { cx += b.m[3][0]; cy += b.m[3][1]; cz += b.m[3][2]; }
            let n = boxes.len().max(1) as f32;
            let d = [cx / n - e[0], cy / n - e[1], cz / n - e[2]];
            sim.bond.yaw = d[0].atan2(d[2]);
            sim.bond.pitch = (d[1] / (d[0] * d[0] + d[2] * d[2]).sqrt().max(1e-4)).atan() / (std::f32::consts::FRAC_PI_2);
        }
        // The P2K is semi-auto: a fresh trigger pull per shot (hold-fire fires once); the clip is
        // refilled like the player's own reload between pulls.
        let firing = sim.drones.frame % 12 == 0;
        if firing { sim.gun.clip = 7; sim.gun.spare = 40; }
        sim.step_bond(Input::default(), firing, false, false);
        let dr = &sim.drones.list[di];
        if matches!(dr.state, 0x66) { drew_weapon = true; }
        if NOISE_STATES.contains(&dr.state) { heard_noise = true; }
        if dr.anim_set != 12 && !dr.corpse { left_civilian_set = true; }
        if dr.corpse { death_seen = true; break; }
        // Sister: some team-1 guard leaves his idle under the same gunfire (the sound dispatch
        // still reaches the guards' own 0x14 arms [M the guard IDLE/Patrol handlers]).
        if !guard_reacted {
            guard_reacted = sim.drones.list.iter().any(|g| g.team == 1 && g.present && g.enabled && !g.corpse
                && (matches!(g.state, state::ALERT | state::ATTACK) || NOISE_STATES.contains(&g.state)));
        }
    }
    let dr = &sim.drones.list[di];
    assert!(death_seen, "the driver is damageable — the shots kill him [M 0x13fff0 case 5: the player's shot has no team gate]");
    assert!(!drew_weapon, "gunfire never sends the driver to DrawWeapon 0x66 (no msg-0x14 arm in the game's driver states [M 0x160ed0/0x160bd0/0x160d98])");
    assert!(!heard_noise, "gunfire never puts the driver into the guard heard-noise chain 0xa2-0xa5");
    assert!(!left_civilian_set, "the driver keeps his civilian anim set 12 — no guard-set weapon draw");
    assert!(guard_reacted, "sister: the guards still react to the same gunfire");
    // His death line is his own: DeathTalk defaults voice type 0xf to line 1; the guards' (type 1)
    // death line is 4 [M 0x13eff8:16-31]. The talk is queued forced — unless the same line's sfx is
    // already the current line, which the game skips [M 0x13eff8:76-85]: the scare's pain line and
    // line 1 share sfx 925 here, so an empty queue with that line still playing is the same sound.
    let d = sim.data();
    let line_sfx = |i: usize| d.voices.and_then(|v| v.get(i)).map(|l| l.sfx);
    let s1 = line_sfx(1);
    let playing = dr.voice.map(|v| v.1);
    assert!(dr.talks.contains(&(1usize, true)) || playing == s1,
        "the driver's death sound is line 1 (voice type 0xf default [M 0x13eff8 default arm]): queued {:?} or line 1's sfx {s1:?} playing, got {playing:?}", dr.talks);
    // This level's voice bank shares line 1's sfx with the guards' line 4 (both 139 [L d.voices]) —
    // the game itself gives the driver the "guard death sound"; the playtest's deviation was the
    // guard death ANIM (the weapon draw's anim set 12 → 4), which the SOUND-arm fix removes.
    // The protected actor: team 3 with props 0x53/0x54 → his death raises ch 0x60 ("Killed
    // Civilians") [M OnInitDeath 0x145bd0:21-24, reason 3].
    let mut failed = false;
    for _ in 0..180u32 {
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.channels.on(0x60) { failed = true; break; }
    }
    assert!(failed, "killing the protected driver raises ch 0x60 [M 0x145bd0 reason 3]");
}

/// The scare: armed Bond in his facing cone sends him down the scared chain, and the cower keeps
/// him IN PLACE on the floor at the game's position (the cower anim's x/z root is constant [D
/// 04000048], the game's CivilianScared runs straight to CivilianHiding at alt mode 0 [M 0x15d3a0],
/// and the floor snap keeps his feet on the courtyard floor [M 0x140660]). Sister: he stays alive
/// and unarmed — a scare is not a shot.
#[test]
fn the_scared_cower_keeps_him_in_place_on_the_floor() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000006.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    let di = sim.drones.list.iter().position(|dr| dr.skin == 0x0500_0035).expect("the driver");
    tick_idle(sim, 920);
    let before = sim.drones.list[di].pos;
    let floor = sim.world.segment([before[0], before[1], before[2]], [before[0], before[1] - 2.5, before[2]])
        .map(|t| before[1] - 2.5 * t);
    let floor = floor.expect("the driver stands on floor after the start script");
    assert!((before[1] - 1.0 - floor).abs() < 0.05, "idle: the game's ground convention pos.y = floor + hip 1.0, got {} vs {floor}", before[1]);

    // Armed Bond 4 units ahead in his facing cone: TruckDriverIdle case 3 NearArmedBond → 0x12.
    let dyaw = sim.drones.list[di].yaw;
    stand_armed_at(sim, [before[0] + dyaw.sin() * 4.0, before[1], before[2] + dyaw.cos() * 4.0], before);
    tick_idle(sim, 180);
    let dr = &sim.drones.list[di];
    assert!(matches!(dr.state, state::CIVILIAN_SCARED | state::CIVILIAN_HIDING),
        "the driver reacts to armed Bond within 6 through TruckDriverIdle's scare [M 0x160ed0 case 3] — got {:#x}", dr.state);
    assert_eq!(dr.anim.state, 0x19, "CivilianHiding ENTER plays the cower 0x19 [M 0x15d520 case 1]");
    assert!(dr.alt_mode != 7, "alt mode {} ≠ 7: CivilianScared runs straight to CivilianHiding — no run-to-point [M 0x15d3a0:31-43]", dr.alt_mode);
    // The game's position: IN PLACE (the cower anim's x/z root is constant [D seq 04000048])...
    let moved = ((dr.pos[0] - before[0]).powi(2) + (dr.pos[2] - before[2]).powi(2)).sqrt();
    assert!(moved < 0.05, "the cower happens where he stands (the game moves him nothing) — moved {moved:.3}");
    // ...ON THE FLOOR: the feet stay at the courtyard floor the floor snap found [M 0x140660] —
    // the playtest's old build drifted him onto the neighbouring crates (the no-gravity stand-in);
    // with the game's gravity he crouches at ground level.
    assert!((dr.pos[1] - 1.0 - floor).abs() < 0.05, "the scared pose keeps his feet on the floor ({} vs floor {floor})", dr.pos[1]);
    // Sisters: a scare is not a shot — alive, and never the guard set.
    assert!(!dr.corpse && dr.health > 0.0, "the scare never damages him");
    assert_eq!(dr.anim_set, 12, "the cower plays in the civilian set");
}
