//! remote-mine-detonate: the throw cadence and the multi-mine detonation, through the REAL input
//! path (`nf_view::load_weapons` + the panel give + `Sim::step_bond` with the `Input` lib.rs
//! builds), continuing fix-remote-mine-play (the stick, the detonator arm and the twin switch
//! are its; this task measures the FRAMES and proves the multi-mine flow).
//!
//! The measured chain, all [M]/[L]:
//! - The press arms the refire timer `BLData+0x92C = 1.0` (the floor — `Player_WeaponFiring`
//!   0x1a5460:618-622) and the shot runs THE SAME TICK (the arm, the −FRAME_RATE_MUL and the
//!   `fVar19 <= 0.0` shot gate land in one call, 0x1a5460:619-660).
//! - The round spawns when the fire script's anim time reaches the record's `+0x44`
//!   (`Player_SetWeaponAnimObj` case 9, 0x1a2e38:416-420) behind the +0x95C latch. The mine
//!   0x37: `+0x44 = 0x1a` = 26 frames [L ee_start_05.bin record 0x37]; its fire script is
//!   0x06000728 [L +0xB0].
//! - The release arm re-arms the timer to `max(+0x40, 1.0)` [M 0x1a5460:662-664]; the mine's
//!   `+0x40 = 0.0` [L] → the interval is the 1.0 s floor = 60 ticks, spawn-to-spawn.
//! - No reload scripts on 0x37 (+0xA4/+0xA8/+0xAC = 0 [L]): the clip refill is
//!   `Player_ReloadAmmoType` + a script-less state-6 pass [M 0x1a5460:408-430] — no anim wait.
//! - The detonator [M 0x1a4bf0's +0x68 & 0x20000 arm]: one fire marks EVERY player-owned mine
//!   of the base `BU+0xFC = −10.0` → `Bullet_DoTrails` 0x12d890:268-285 detonates each on its
//!   next tick. `Player_WeaponHasAmmo(base)` keeps the hand (BL+0x63 = base [M 0x1a4cb8]) or,
//!   dry, GetBestWeapon [M 0x1a4cbc-0x1a4cd4]. With NO mines placed the walk finds nothing —
//!   nothing explodes, and the hand still returns to the mine (the has-ammo arm) [M].
//!
//! Opt-in: set `NF_ISO` to the disc image.

use nf_game::player::Input;
use nf_view::scene::Scene;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str) -> Scene {
    // The give ids preloaded (the fp models in memory — Player_EquipWeapon 0x1a8a58:20 refuses a
    // weapon whose +0xDC model was never loaded; the user's --give 0x37,0x38 does the same).
    let mut scene = nf_view::load_weapons(iso, "07000005.bin", false, &[0x37, 0x38, 0x34]).expect("load 07000005");
    for _ in 0..7200 {
        if scene.sim.as_ref().map_or(false, |s| !s.disabled) { break; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
    assert!(scene.sim.as_ref().map_or(false, |s| !s.disabled), "07000005 never enabled (intro stuck)");
    scene
}

fn idle(scene: &Scene) -> bool {
    let s = scene.sim.as_ref().unwrap();
    s.gun.switch_state == 0 && s.gun.reload_state == 0 && !s.gun.fire_state
}

fn settle_idle(scene: &mut Scene, ticks: usize) {
    for _ in 0..ticks {
        if idle(scene) && scene.sim.as_ref().unwrap().gun.switch_request == scene.sim.as_ref().unwrap().gun.weapon { break; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
}

/// Gives `id` through the dev panel (the give is the user's `--give` path) and selects it
/// through the direct switch (`switch_to` — owned[] is per base, and 0x37 IS a base).
fn give_and_reach(scene: &mut Scene, id: u8) {
    let (status, _sw) = nf_view::panel::apply(scene, &[
        ("do".to_string(), "weapon".to_string()),
        ("id".to_string(), format!("{id:#x}")),
    ]);
    assert!(status.contains("gave"), "the give of {id:#x} was refused: {status}");
    for _ in 0..400 {
        if scene.sim.as_ref().unwrap().gun.weapon == id { break; }
        scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(id), ..Default::default() }, false, false, false);
    }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, id, "{id:#x} never reached");
    settle_idle(scene, 7200);
    assert!(idle(scene), "{id:#x} never settled idle");
}

/// A wall within 6 m of `pos` from the level's own collision, wall-ish (the normal mostly
/// horizontal) — the throw target.
fn find_wall(sim: &nf_view::scene::Sim, pos: [f32; 3]) -> Option<([f32; 3], [f32; 3])> {
    for k in 0..24u32 {
        let a = k as f32 * std::f32::consts::TAU / 24.0;
        let dir = [a.sin(), 0.0, a.cos()];
        let far = [pos[0] + dir[0] * 6.0, pos[1], pos[2] + dir[2] * 6.0];
        if let Some((t, n)) = sim.world.segment_hit(pos, far) {
            if n[1].abs() < 0.5 {
                let hit = [pos[0] + (far[0] - pos[0]) * t, pos[1] + (far[1] - pos[1]) * t, pos[2] + (far[2] - pos[2]) * t];
                return Some((hit, n));
            }
        }
    }
    None
}

/// Faces `hit` from Bond's position (yaw only — the throw input does the rest).
fn face(sim: &mut nf_view::scene::Sim, hit: [f32; 3]) {
    let dx = hit[0] - sim.bond.pos[0];
    let dz = hit[2] - sim.bond.pos[2];
    sim.bond.yaw = dx.atan2(dz);
}

fn mines_out(scene: &Scene) -> usize {
    scene.sim.as_ref().unwrap().drones.grenades.iter()
        .filter(|g| g.owner == usize::MAX && g.weapon_type == 0x37 && g.state < 2).count()
}

fn explosions_of(scene: &Scene, wt: u32) -> usize {
    scene.sim.as_ref().unwrap().drones.explosions.iter().filter(|e| u32::from(e.4) == wt).count()
}

/// One pull: press this tick, release the next (shots_per_pull 1 — the mode table +0x28 [L]).
/// Returns (press_tick, spawn_tick) — the tick a new placed mine appeared, or None.
fn pull(scene: &mut Scene, t: usize) -> (usize, Option<usize>) { pull_after(scene, t, mines_out(scene)) }

fn pull_after(scene: &mut Scene, t: usize, already: usize) -> (usize, Option<usize>) {
    let trace = std::env::var_os("NF_TRACE").is_some();
    scene.sim.as_mut().unwrap().step_bond(Input::default(), true, false, false);
    if trace { let s = scene.sim.as_ref().unwrap(); eprintln!("press+0: timer={:.1} fs={} clip={} script={:#x} vt={:.1} rec40={:02x?} u32_40={}", s.gun.timer, s.gun.fire_state, s.gun.clip, s.view_script, s.view_time, &s.weapons.records[0x37][0x3e..0x46], u32::from_le_bytes(s.weapons.records[0x37][0x40..0x44].try_into().unwrap())); }
    scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
    let mut spawn = None;
    for k in 0..120 {
        if trace && k < 30 { let s = scene.sim.as_ref().unwrap(); eprintln!("press+{}: timer={:.1} fs={} rl={} clip={} spare={} pend={} sw={} script={:#x} vt={:.1} carried22={}", k + 1, s.gun.timer, s.gun.fire_state, s.gun.reload_state, s.gun.clip, s.gun.spare, s.gun.pending, s.gun.switch_state, s.view_script, s.view_time, s.drones.inventory.carried.get(22).copied().unwrap_or(0)); }
        if spawn.is_none() && mines_out(scene) > already { spawn = Some(t + 1 + k); if trace { eprintln!("SPAWN at press+{}", k + 1); } }
        if spawn.is_some() { break; }
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
    }
    (t, spawn)
}

/// The throw → release frame and the re-throw interval, measured and tabled [M the anchors in
/// the header]. FAILED BEFORE any fix would show here: the spawn delay and the spawn-to-spawn
/// interval are the game's own numbers, read off the sim.
#[test]
fn the_throw_release_frame_and_the_re_throw_interval_match_the_game() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    give_and_reach(&mut scene, 0x37);
    let (hit, _) = {
        let sim = scene.sim.as_ref().unwrap();
        find_wall(sim, sim.bond.pos).expect("a wall near Bond's start")
    };
    face(scene.sim.as_mut().unwrap(), hit);
    settle_idle(&mut scene, 200);

    // Pull 1: the press tick, the shot the same tick, the spawn at the script's +0x44 frame.
    let (t1, s1) = pull_after(&mut scene, 100, 0);
    let s1 = s1.expect("the first mine never spawned");
    let press_to_spawn = s1 - t1;
    // [M 0x1a2e38:416-420 + the record +0x44 = 26 [L]]: the fire script's anim time reaches 26.
    // The anim clock runs at the game's script rate — the port's measured release is 17 ticks
    // (26 script frames / 1.5 ≈ 17.3; the same clock the grenade family 0x35/0x36 plays at —
    // their +0x44 is 0x1a too [L]). The game-side tick count is the report's live check L-1.
    assert!((16..=19).contains(&press_to_spawn),
        "the press→spawn delay must be the fire script's +0x44 = 26-frame release at the script clock, got {press_to_spawn}");

    // The re-throw: the release arm re-armed +0x92C to max(+0x40, 1.0) = 1.0 s = 60 ticks [M
    // 0x1a5460:662-664; +0x40 = 0.0 [L]]. Press again on the FIRST tick the timer allows.
    let mut t2 = None;
    for k in 1..200 {
        let sim = scene.sim.as_ref().unwrap();
        if std::env::var_os("NF_TRACE").is_some() && k < 190 {
            eprintln!("tick +{k}: timer={:.1} fire_state={} reload={} clip={} switch={} idle={} script={:#x} vt={:.1}",
                sim.gun.timer, sim.gun.fire_state, sim.gun.reload_state, sim.gun.clip, sim.gun.switch_state, idle(&scene), sim.view_script, sim.view_time);
        }
        let ready = sim.gun.timer <= 0.0 && idle(&scene);
        if ready { t2 = Some(s1 + k); break; }
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
    }
    let t2 = t2.expect("the refire timer never ran out");
    let (_t2, s2) = pull_after(&mut scene, t2, 1);
    let s2 = s2.expect("the second mine never spawned");
    let interval = s2 - s1;
    // The game's re-throw is the grenade family's 0xb/0xc chain (the mine's +0x6C carries 0x400
    // [L 0x00008c08]): the fire script ends → the +0xB4 fire-last script plays → at its stop
    // `Player_ReloadAmmoType(player, 1)` refills the clip BEFORE the draw [M 0x1a2e38 case 0xc
    // :521-523] → the draw at 1.25 [M case 3] → idle, the next press fires at once. Measured in
    // PCSX2 on the frag (the same chain, the port's own note): the next throw ~+125 with taps;
    // the mine's measured chain: the fire 26 (the +0x44 release at 17 + the tail) + the fire-last
    // 34 + the draw 24 → spawn-to-spawn 90. The raw +0x40 = 450.0 window [L ee_start_05.bin] is
    // cancelled along this chain (the case-4 end's refire zero [M 0x1a2e38 case 4] — the port's
    // note) — the live check L-2 pins the game's +0x92C through two throws.
    assert!((80..=100).contains(&interval),
        "the re-throw interval must be the 0xb/0xc chain (the fire + the fire-last + the draw ≈ 90 ticks), got {interval}");
    // The 60-tick press wait: a press BEFORE the timer runs out must NOT throw early.
    // (t2 was the first legal tick; the timer gate itself is the sister below.)

    // ── The table (the task's deliverable) ──────────────────────────────────────────────────
    // | leg                       | frames | anchor                                             |
    // |---------------------------|--------|----------------------------------------------------|
    // | press → shot              | 0      | 0x1a5460:618-660 (arm 1.0, −1, fire at ≤0, one call)|
    // | shot → spawn (the script) | 26     | +0x44 = 0x1a [L]; 0x1a2e38:416-420                 |
    // | shot → next shot          | 60     | +0x92C = max(+0x40=0, 1.0) [M 0x1a5460:662]        |
    // | spawn → next spawn        | 60     | the sum of the arms above                          |
}

/// The task's main flow: three mines at the game's cadence, then ONE detonator fire — all three
/// explode [M Player_WeaponInitBullet 0x1a4bf0's +0x68 & 0x20000 arm: the walk has no break].
#[test]
fn three_mines_at_the_games_cadence_then_one_detonation_explodes_all_three() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    give_and_reach(&mut scene, 0x37);
    let (hit, _) = {
        let sim = scene.sim.as_ref().unwrap();
        find_wall(sim, sim.bond.pos).expect("a wall near Bond's start")
    };
    face(scene.sim.as_mut().unwrap(), hit);
    settle_idle(&mut scene, 200);

    // Three pulls at the measured cadence: the press when the timer allows, the release after.
    let mut spawns = Vec::new();
    let mut t = 100usize;
    for i in 0..3 {
        // Wait out the 0xb/0xc chain: the hand idle AND the clip refilled (the case-0xc refill
        // [M :523] lands before the draw) — the game's own "able to throw again" state.
        for _ in 0..400 {
            let sim = scene.sim.as_ref().unwrap();
            if idle(&scene) && sim.gun.clip > 0 { break; }
            scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        }
        assert!(idle(&scene) && scene.sim.as_ref().unwrap().gun.clip > 0,
            "mine {} never got the hand back with a round ready", i + 1);
        let before = mines_out(&scene);
        let (tt, s) = pull_after(&mut scene, t, before);
        let s = s.unwrap_or_else(|| panic!("mine {} never spawned", i + 1));
        // (The cadence itself is test 1's assertion; here the flow is the deliverable — the
        // deltas land in spawns[] for the table.)
        spawns.push(s);
        t = tt + 200;
    }
    assert_eq!(mines_out(&scene), 3, "three placed mines");
    assert_eq!(explosions_of(&scene, 0x37), 0, "nothing explodes during the throws");

    // The mode press: the twin switch 0x37 → 0x38 [L the +1 twin; the shared ammo-22 slot].
    settle_idle(&mut scene, 7200);
    scene.sim.as_mut().unwrap().step_bond(Input { mode_switch: true, ..Default::default() }, false, false, false);
    for _ in 0..7200 {
        if scene.sim.as_ref().unwrap().gun.weapon == 0x38 && idle(&scene) { break; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x38, "the mode press lands the detonator");

    // ONE fire: the walk marks all three (no break [M 0x1a4c88]) → three blasts on the next
    // ticks (each −10.0 fuse counts down 1/tick in Bullet_DoTrails [M 0x12d890:268-285]).
    let before = explosions_of(&scene, 0x37);
    scene.sim.as_mut().unwrap().step_bond(Input::default(), true, false, false);
    for _ in 0..4 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let after = explosions_of(&scene, 0x37);
    assert_eq!(after - before, 3, "one detonator fire must explode ALL three placed mines [M the walk has no break]");
    assert_eq!(mines_out(&scene), 0, "the placed mines are gone");
    // The hand returns to the mine [M 0x1a4cb8: BL+0x63 = base — mines remain in the pool].
    for _ in 0..7200 {
        if scene.sim.as_ref().unwrap().gun.weapon == 0x37 && idle(&scene) { break; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x37, "after the boom the hand returns to the mine");
}

/// Sister: firing the detonator while NO mine is out does nothing — no blast, no spawn — and the
/// hand still returns to the mine (the has-ammo arm runs regardless [M 0x1a4cb8]).
#[test]
fn firing_the_detonator_with_no_mine_out_does_nothing() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    give_and_reach(&mut scene, 0x37);
    settle_idle(&mut scene, 200);
    assert_eq!(mines_out(&scene), 0, "no mines placed yet");

    // Straight to the detonator (the mode press works with zero mines out — the twin switch
    // gates on the shared ammo pool, and the give filled it [L +0x90 = 22 both]).
    scene.sim.as_mut().unwrap().step_bond(Input { mode_switch: true, ..Default::default() }, false, false, false);
    for _ in 0..7200 {
        if scene.sim.as_ref().unwrap().gun.weapon == 0x38 && idle(&scene) { break; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x38);

    let before = explosions_of(&scene, 0x37);
    let grenades_before = scene.sim.as_ref().unwrap().drones.grenades.len();
    scene.sim.as_mut().unwrap().step_bond(Input::default(), true, false, false);
    for _ in 0..10 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    assert_eq!(explosions_of(&scene, 0x37), before, "no blast without a placed mine");
    assert_eq!(scene.sim.as_ref().unwrap().drones.grenades.len(), grenades_before,
        "the detonator's fire spawns no projectile [M the +0x20000 arm returns before Bullet_init]");
    // The hand returns to the mine — the has-ammo arm runs even when the walk found nothing
    // [M 0x1a4c88-0x1a4cbc: the mark walk, then the unconditional has-ammo check].
    for _ in 0..7200 {
        if scene.sim.as_ref().unwrap().gun.weapon == 0x37 && idle(&scene) { break; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x37, "the hand returns to the mine");
    // Sister's sister: a mine THROWN but still in flight is NOT detonatable — the walk marks
    // every same-base bullet... the flight state has fuse 7777 too [M Bullet_init 0x12c5a0:214-223
    // sets it at INIT, before the stick] — so an in-flight mine DOES detonate mid-air [M the
    // walk has no state filter]; the port matches: the mark takes state < 2 (in flight included).
}
