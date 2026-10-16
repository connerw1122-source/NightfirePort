//! scorpion-modes-12: the AT-600 Scorpion's fire modes through the real input path — playtest
//! 1012-1723 (07000012): "the scorpion launcher heat seeker mode doesn't work".
//!
//! The mode pair [L ee_start_05.bin weapon_data, pinned below against the ISO parse]: record
//! 0x2E 'Heatseeker' carries the `+0x6C` 0x20000 homing bit, 'Unguided' 0x2F does not; the pair
//! twin-links through `+0x05` and the ○ press switches through `Player_Weapon` 0x1a24c8's twin
//! arm (`fire_mode_switch` + `Player_WeaponHasAmmo` 0x1a16c0). The homing is `Bullet_homing`
//! 0x12f358, run from `Bullet_update` 0x12e7b0:483 after the move for `+0x6C & 0x20000` rounds:
//! nearest heat-seekable object [M `Bullet_heatseekable` 0x12f248] within 1000 units inside the
//! 0.66-dot cone of the round's facing with a clear line of sight, then
//! `dir += clamp((to_target − dir)·0.0333, ±0.01)` per component, normalised.
//!
//! The keyboard binding is M (the pad's ○, action 12) — `Input::mode_switch`. One level load
//! serves every phase (the 07000012 archive is ~4 M triangles; the old five-load test process was
//! OOM-killed).

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn norm(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if l > 0.0 { [v[0] / l, v[1] / l, v[2] / l] } else { v }
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }

/// Give the pair ammo and hold `w`, auto-aim off (the test measures the HOMING, not the game's
/// auto-aim assist [M Player_AutoAim 0x1a7c98 — on by default, and with it on the shot leaves
/// nearly on-axis]).
fn hold(scene: &mut nf_view::scene::Scene, w: u8) {
    let sim = scene.sim.as_mut().unwrap();
    sim.auto_aim = false;
    for x in [0x2eusize, 0x2f] {
        if let Some(c) = sim.drones.inventory.clip.get_mut(x) { *c = 4; }
        if let Some(o) = sim.drones.inventory.owned.get_mut(x) { *o = true; }
    }
    sim.gun.weapon = w;
    sim.gun.switch_request = w;
    sim.gun.switch_state = 0;
    sim.gun.reload_state = 0;
    sim.gun.fire_state = false;
    sim.gun.clip = 4;
}

/// Place Bond `dist` units from `guard` with a clear line, facing `off_axis` radians away from
/// the guard. Deterministic: the first ring spot that holds — no script hold within 60 ticks, the
/// guard's floor height, a clear world line and a facing 0.75..0.98 off the bearing.
fn aim_setup(scene: &mut nf_view::scene::Scene, guard: usize, dist: f32, off_axis: f32) {
    let gp = scene.sim.as_ref().unwrap().drones.list[guard].pos;
    for k in 0..24u32 {
        let ang = k as f32 * (std::f32::consts::FRAC_PI_2 / 6.0);
        let spot = [gp[0] + dist * ang.cos(), gp[1], gp[2] + dist * ang.sin()];
        let bearing = (gp[0] - spot[0]).atan2(gp[2] - spot[2]);
        {
            let sim = scene.sim.as_mut().unwrap();
            sim.bond = nf_game::player::Bond::stand_at(&sim.world, spot, bearing + off_axis);
        }
        // A script trigger at the spot re-disables Bond — the reject window.
        let mut held = false;
        for _ in 0..60u32 {
            scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
            if scene.sim.as_ref().unwrap().disabled { held = true; break; }
        }
        if held { continue; }
        let sim = scene.sim.as_ref().unwrap();
        // stand_at drops through hollow geometry — the spot must hold the guard's floor.
        if (sim.bond.pos[1] - gp[1]).abs() > 3.0 { continue; }
        let head = sim.bond.head();
        let blocked = sim.world.segment(head, gp).is_some();
        let to = norm([gp[0] - spot[0], gp[1] - spot[1], gp[2] - spot[2]]);
        let off = dot(sim.bond.look(), to);
        if !blocked && off > 0.75 && off < 0.98 {
            for _ in 0..8 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
            assert!(!scene.sim.as_ref().unwrap().disabled, "the chosen spot left Bond disabled");
            return;
        }
    }
    panic!("no ring spot around guard {guard} with a clear line and an off-axis facing");
}

/// The fire-mode round trip: the disc record pins, the ○ swap through the real input, the
/// heat-seeker curving into a guard, the unguided twin flying straight, and the never-locks-Bond
/// sister — one level load, phases in order.
#[test]
fn the_scorpion_fire_modes_through_the_real_input_path() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000012.bin", false).expect("load 07000012");

    // ── phase A: the level's intro, then the disc record pins ──
    for _ in 0..60 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    for _ in 0..4000u32 {
        let dis = scene.sim.as_ref().map_or(false, |s| s.disabled);
        if !dis { break; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(!sim.disabled, "07000012's intro never released Bond");
        let wd = &sim.weapons;
        let f6c = |w: usize| u32::from_le_bytes(wd.records[w][0x6c..0x70].try_into().unwrap());
        let twin = |w: usize| wd.records[w][5] as i8;
        assert_eq!(f6c(0x2e) & 0x2_0000, 0x2_0000, "0x2E 'Heatseeker' carries the 0x20000 homing bit [L ee_start_05.bin]");
        assert_eq!(f6c(0x2f) & 0x2_0000, 0, "'Unguided' 0x2F carries no homing bit [L]");
        assert_eq!((0x2ei32 + twin(0x2e) as i32) & 0xff, 0x2f, "0x2E's twin steps to 0x2F [L]");
        assert_eq!((0x2fi32 + twin(0x2f) as i32) & 0xff, 0x2e, "0x2F's twin steps back to 0x2E [L]");
        assert_eq!(wd.records[0x2e][0x2e], 1, "the mode count is 1 — ○ is the twin switch [L]");
        // The HUD's mode label [L]: BOTH records carry name 0x05000048 — the launcher's name is
        // the same string in both modes (the disc's own data). The HUD reads the held record's
        // label (panel.rs `wd.name_label(id)`), so the value is stable across the swap.
        assert_eq!(wd.name_label(0x2e), wd.name_label(0x2f), "both records carry name 0x05000048 [L]");
    }

    // ── phase B: the ○ swap 0x2F → 0x2E through the real input ──
    hold(&mut scene, 0x2f);
    {
        let sim = scene.sim.as_mut().unwrap();
        let mut input = Input::default();
        input.mode_switch = true;
        sim.step_bond(input, false, false, false);
        assert_eq!(sim.gun.switch_request, 0x2e, "○ on the Unguided Scorpion requests the Heatseeker twin [M 0x1a24c8:246-260]");
    }
    for _ in 0..200 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert_eq!(sim.gun.weapon, 0x2e, "the switch completes onto the Heatseeker record");
        assert_eq!(sim.gun.switch_state, 0, "and settles idle");
    }
    {
        let sim = scene.sim.as_mut().unwrap();
        let mut input = Input::default();
        input.mode_switch = true;
        sim.step_bond(input, false, false, false);
        assert_eq!(sim.gun.switch_request, 0x2f, "○ again toggles back [M 0x1a16c0 twin −1]");
    }
    for _ in 0..200 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x2f, "back on the Unguided record");

    // The shooting guard: the first enabled, alive guard after the intro.
    let guard = {
        let sim = scene.sim.as_ref().unwrap();
        (0..sim.drones.list.len()).find(|&i| {
            let d = &sim.drones.list[i];
            d.present && !d.corpse && d.enabled && d.health > 0.0
        }).expect("07000012 has an enabled guard after the intro")
    };
    let off_axis = 0.42f32; // ~24° — inside the 0.66-dot (≈49°) acquisition cone, clearly off-axis
    let hp0 = scene.sim.as_ref().unwrap().drones.list[guard].health;

    // ── phase C: the heat-seeker, fired off-axis, curves into the guard ──
    aim_setup(&mut scene, guard, 18.0, off_axis);
    hold(&mut scene, 0x2e);
    {
        let sim = scene.sim.as_mut().unwrap();
        let mut input = Input::default();
        input.fire = true;
        sim.step_bond(input, true, false, false);
    }
    for _ in 0..2 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let mut dir0 = None;
    let (mut locked, mut curved) = (false, false);
    let mut rocket_died = false;
    for _ in 0..3000u32 {
        {
            let sim = scene.sim.as_ref().unwrap();
            if let Some(b) = sim.drones.player_bullets.last() {
                if !b.dead {
                    let gp = sim.drones.list[guard].pos;
                    let to = norm([gp[0] - b.pos[0], gp[1] - b.pos[1], gp[2] - b.pos[2]]);
                    if let Some(d0) = dir0 { if dot(b.dir, to) - dot(d0, to) > 0.03 { curved = true; } }
                    if b.target == Some(guard) { locked = true; }
                    if dir0.is_none() { dir0 = Some(b.dir); }
                }
            }
        }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if sim.drones.list[guard].health < hp0 { break; }
        if !sim.drones.player_bullets.iter().any(|b| !b.dead) { rocket_died = true; break; }
    }
    assert!(locked, "the heat-seeker locked the guard (BU+0x40) [M 0x12f3b4]");
    assert!(curved, "the round's direction curved toward the guard [M 0x12f614]");
    assert!(scene.sim.as_ref().unwrap().drones.list[guard].health < hp0,
        "the heat-seeker hit the guard (the homing rocket reaches him at the 0.0625 u/tick quarter speed [L +0x6C & 0x200])");
    let _ = rocket_died;

    // ── phase D: the unguided twin, the same off-axis shot, flies straight and misses ──
    hold(&mut scene, 0x2f);
    aim_setup(&mut scene, guard, 18.0, off_axis);
    {
        let sim = scene.sim.as_mut().unwrap();
        let mut input = Input::default();
        input.fire = true;
        sim.step_bond(input, true, false, false);
    }
    for _ in 0..2 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let hp1 = scene.sim.as_ref().unwrap().drones.list[guard].health;
    let mut dir0 = None;
    let mut straight = true;
    for _ in 0..3000u32 {
        {
            let sim = scene.sim.as_ref().unwrap();
            if let Some(b) = sim.drones.player_bullets.last() {
                if !b.dead {
                    if let Some(d0) = dir0 { if dot(b.dir, d0) < 0.9995 { straight = false; } }
                    if dir0.is_none() { dir0 = Some(b.dir); }
                    assert_ne!(b.target, Some(guard), "the Unguided round never locks [L: no homing bit]");
                }
            }
        }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        if !scene.sim.as_ref().unwrap().drones.player_bullets.iter().any(|b| !b.dead) { break; }
    }
    assert!(straight, "the Unguided rocket's direction never changes [L: no homing bit]");

    // ── phase E: the never-locks-Bond sister — no seekable object, no lock, ever ──
    {
        let sim = scene.sim.as_mut().unwrap();
        // The candidates' health floors (the game's `+0xac > 0` gate [M 0x12f248]); Bond (the
        // owner, BU+0x30) is not a candidate object at all.
        for dr in sim.drones.list.iter_mut() { dr.health = 0.0; }
    }
    hold(&mut scene, 0x2e);
    {
        let sim = scene.sim.as_mut().unwrap();
        let mut input = Input::default();
        input.fire = true;
        sim.step_bond(input, true, false, false);
    }
    for _ in 0..2 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    for _ in 0..1200u32 {
        {
            let sim = scene.sim.as_ref().unwrap();
            for b in &sim.drones.player_bullets {
                if !b.dead {
                    assert_eq!(b.target, None, "no seekable object → no lock, ever [M 0x12f3b4]");
                }
            }
        }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        if !scene.sim.as_ref().unwrap().drones.player_bullets.iter().any(|b| !b.dead) { break; }
    }
}
