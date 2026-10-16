//! fix-npcs: the C2 "drawn but never attacks / can't be damaged" class, the searchlight alarm path and
//! the idle-guard regression gates, all through the real viewer path (`nf_view::load` + `Sim::step_bond`).
//!
//! Game evidence:
//! - The NIS freeze: `Drone_EnableAll(0)` walks the whole llist calling `NDrone2_Enable(0)` per drone —
//!   `obj+0xF0 |= 0x10|0x20` (not drawn, not picked), `+0x4F8 &= ~0x1801A0` [M 0x149dd0:8-15, 0x1383e8:13-23];
//!   `Drone_CollisionHandler` returns while `NPCGlobals == 0` [M 0x140968:62]. `Script_Free`'s 0x80 flag
//!   re-enables every drone EXCEPT state-1 WaitSwitch ones [M 0x1da880:41-43, 0x1383e8:14].
//! - The searchlight spot ray's pick flags 0x18 do not exclude the player's class [M Collide_PickObj
//!   0x1eae80:26-40 — the class-3 exclusion is mask 0x400], so the spot lands on Bond's world sphere and
//!   `Vec_Dist3D(spot, player) <= 2.0` [M Searchlight_Update 0x1cb900:251] arms the light in play.
//! - The alarm: state 1 sets the alarm channel every tick [M 0x1cbf58 area], `DroneFunc_CheckAlarmRaised`
//!   0x1453a8 counts ch 0x37 → `Music_Event(2,1)`, `UpdateMusicalEvents` hysteresis → per-level dispatch →
//!   `UpdateMFX_01CastleB` 0x113060: `under_attack ? PickRandomCombat(0) : PickRandomStealth()`.
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(archive: &str) -> nf_view::scene::Scene {
    let iso = iso().expect("NF_ISO");
    nf_view::load(&iso, archive, false).expect("load")
}

/// The mid-intro freeze (07000002, tick 120 — the level-start NIS holds ~1540 ticks): every present drone
/// is disabled — invisible to the renderer and immune to bullets — exactly like the game's
/// `Drone_EnableAll(0)`; and once the intro ends the ch-waiter statues stay disabled while the civilian
/// (team 2, HostageIdle) stands visible. This is the user's bathhouse C2 report: on the pre-fix port the
/// frozen drones stayed `enabled` (drawn mid-intro, damageable by a bullet already in flight).
#[test]
fn the_intro_freeze_disables_every_drone_and_shields_them_from_bullets() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load("07000002.bin");
    let (all_disabled, bullet_immune) = {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..120 { sim.step_bond(Input::default(), false, false, false); }
        assert!(sim.disabled, "the level-start NIS holds Bond at tick 120");
        let all_disabled = sim.drones.list.iter().filter(|d| d.present).all(|d| !d.enabled);
        assert!(all_disabled, "the NIS freeze disables every present drone (obj+0xF0 0x10|0x20 per NDrone2_Enable(0))");
        // A bullet in flight passes through the frozen drones: fire the P2K at the nearest one.
        let target = sim.drones.list.iter().enumerate().filter(|(_, d)| d.present).min_by(|a, b| {
            let da = (a.1.pos[0] - sim.bond.pos[0]).powi(2) + (a.1.pos[2] - sim.bond.pos[2]).powi(2);
            let db = (b.1.pos[0] - sim.bond.pos[0]).powi(2) + (b.1.pos[2] - sim.bond.pos[2]).powi(2);
            da.partial_cmp(&db).unwrap()
        }).map(|(i, _)| i).unwrap();
        let hp0 = sim.drones.list[target].health;
        {
            let d = nf_game::drone::Data { tables: &sim.tables, props: &sim.props, scripts: &sim.scripts, weapons: &sim.weapons, world: &sim.world, level: sim.level, sequences: &sim.seqs, skins: &sim.skins, skeletons: &sim.skeletons, nav: &sim.nav, covers: &sim.covers, tuning: &sim.tuning, hitboxes: &sim.hitboxes, volumes: &sim.volumes, sfx: None, models: &sim.models, surfaces: None, voices: None, difficulty: 1 };
            let from = sim.bond.head();
            let tp = sim.drones.list[target].pos;
            let dir = [tp[0] - from[0], tp[1] + 0.8 - from[1], tp[2] - from[2]];
            let l = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
            let dir = [dir[0] / l, dir[1] / l, dir[2] / l];
            let snd = nf_formats::sfx::SfxHearing { id: 0x403, inner: 0.0, outer: 0.0, weight: 0.0, looped: false, cull: false };
            sim.drones.fire_player(&d, from, dir, 2, snd);
        }

        for _ in 0..6 { sim.step_bond(Input::default(), false, false, false); }
        (all_disabled, sim.drones.list[target].health >= hp0)
    };
    assert!(bullet_immune, "a bullet fired during the freeze passes through the disabled drones (Drone_CollisionHandler's NPCGlobals gate)");
    // After the intro: the ch-waiter statues sit in WaitSwitch disabled (the game never draws them), the
    // civilian woman next to them (d21, team 2, HostageIdle) is enabled and drawn.
    let (waiter_states, civ) = {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..2900 { sim.step_bond(Input::default(), false, false, false); }
        let waiters: Vec<(u8, bool)> = sim.drones.list.iter().filter(|d| d.present && d.switch_channel != 0 && d.switch_channel != 55)
            .take(4).map(|d| (d.state, d.enabled)).collect();
        assert!(!waiters.is_empty(), "07000002 places ch-waiter guards");
        for (s, e) in &waiters {
            assert_eq!(*s, nf_game::drone::state::WAIT_SWITCH, "the bathhouse guards wait in WaitSwitch");
            assert!(!e, "a WaitSwitch drone stays disabled after the NIS re-enable (0x1383e8:14)");
        }
        let civ = sim.drones.list.iter().enumerate().find(|(_, d)| d.present && d.team == 2).map(|(i, _)| i);
        (waiters, civ)
    };
    let _ = waiter_states;
    if let Some(ci) = civ {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.list[ci].enabled, "the team-2 hostage civilian stands enabled after the intro");
        let fs = scene.frame_state(None, 0.0, nf_view::scene::ViewOpts::default());
        let hidden_by = sim.char_drone.iter().position(|c| *c == Some(ci));
        if let Some(h) = hidden_by { assert!(!fs.hidden[h], "the enabled civilian is drawn by the renderer"); }
    }
    let _ = all_disabled;
}

/// The searchlight's spot ray must pick the player: the ray's pick flags 0x18 do not exclude the player's
/// class [M Collide_PickObj 0x1eae80:26-40 — the class-3 exclusion is mask 0x400], so when the beam is on
/// Bond the spot lands on his world sphere (`obj+0x80` centre, `obj+0x8C` radius 1.0576 [L]) instead of
/// running past him to the ground behind, and the arm test `Vec_Dist3D(spot, glb_players+0x30) <= 2.0`
/// [M Searchlight_Update 0x1cb900:251] fires. The test drives the REAL `Searchlights::update` against the
/// real 07000006 world with a pinned head (`half = 0` holds the sweep at the set yaw/pitch) beaming over
/// the open spawn courtyard: the ray crosses Bond's chest and its static continuation hits the ground ~3
/// units past him — pre-fix the spot lands there (dist > 2.0, no arm); post-fix the sphere hit replaces it,
/// the spot lands on him, and the light arms (state 0 → 1) and raises its alarm channel.
#[test]
fn the_spot_ray_picks_bond_and_arms_the_light_from_his_sphere() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load("07000006.bin");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); }
        // Bond on the open high terrace (floor 50.7, clear air at chest height, verified by a ray scan);
        // the pinned head 7.5 south at his sphere-centre height +3.05: the shallow ray crosses his chest
        // and its static continuation hits the terrace floor ~2.4 past him (no arm pre-fix).
        // The discriminator needs 100 units of clear air along the ray, which the castle's terraces and
        // doors never offer at chest height (measured by probe: the gate courtyards are covered, the high
        // terraces are parapet-stepped and door-gapped). So the pinned head and Bond go into the open sky
        // above the map (y 150 — above every tower), where the level ray finds nothing at all: pre-fix the
        // static ray misses, the spot never draws and the light never arms; post-fix the player-pick sphere
        // is the ray's first hit, the spot lands on Bond, and the arm test fires. The ray decision itself —
        // the thing the fix changes — is exercised against the real `Searchlights::update` and world data.
        let bond_pos = [40.0f32, 150.0, -4.3];
        let centre = [bond_pos[0] - 0.1347, bond_pos[1] - 0.1267, bond_pos[2] + 0.2402];
        let head_pos = [40.0f32, centre[1], -2.9];
        let pitch = -0.05f32; // rising into clear sky; still crosses the sphere (0.38 off its centre)
        let yaw = std::f32::consts::PI; // −z, toward Bond and the tower plane behind him
        use nf_game::searchlight::{Part, Searchlights};
        let part = |kind: u8, pos: [f32; 3]| Part {
            placement: 0, kind, pos, base_yaw: yaw, on: 0, off: 0, alarm: 55, half: 0.0, mid: pitch,
            phase: 0.0, drawn: kind != 2, matrix: [[0.0; 4]; 4], state: 0, pitch, yaw, links: [None; 5],
            killed: false, light: None, alpha: 255, scale: 1.0, box_min: [0.0; 3], box_max: [0.0; 3],
            spawn_pos: pos, created: 0,
        };
        let mut head = part(1, head_pos);
        head.links = [None, None, Some(1), None, None];
        let spot = part(2, [0.0, -10000.0, 0.0]);
        let mut sl = Searchlights::default();
        sl.parts = vec![head, spot];
        // The real update path with a Player view of the airborne Bond (sph = the world sphere centre).
        let d = sim.data();
        let player = nf_game::drone::Player {
            pos: bond_pos, target: bond_pos, eye: bond_pos, aim: [0.0, 0.0, 1.0], sph: centre, hanging: false,
            capsule: ([bond_pos[0], bond_pos[1] + 0.55, bond_pos[2]], [bond_pos[0], bond_pos[1] - 0.76, bond_pos[2]]),
            cel: None };
        let mut rng = nf_game::drone::Rng::new(0x5eed);
        let mut channels = sim.drones.channels.clone();
        eprintln!("[ray] head {head_pos:?} pitch {pitch:.3} bond {bond_pos:?} sph {centre:?}");
        let mut armed = false; let mut on_him = false; let mut alarmed = false;
        for _ in 0..10 {
            sl.update(&d, &player, &mut channels, 0, &[], &mut rng, sim.drones.frame);
            if sl.parts[0].state == 1 { armed = true; }
            if channels.on(55) { alarmed = true; }
            let sp = sl.parts[1].pos;
            let dist = ((sp[0] - bond_pos[0]).powi(2) + (sp[1] - bond_pos[1]).powi(2) + (sp[2] - bond_pos[2]).powi(2)).sqrt();
            if dist <= 2.0 { on_him = true; }
            eprintln!("[ray] spot ({:.2},{:.2},{:.2}) dist {dist:.2} state {}", sp[0], sp[1], sp[2], sl.parts[0].state);
        }
        assert!(on_him, "the spot lands on Bond (dist <= 2.0): the ray picked his world sphere, not the ground behind him");
        assert!(armed, "the arm test fires (state 0 → 1) [M 0x1cb900:251]");
        assert!(alarmed, "tracking raises the alarm channel [M state-1 entry]");
    }
}

/// Sister (the regression gate for the enabled idle guards): a same-floor idle guard on 07000006 sees
/// Bond within seconds, alerts and attacks — the enabled-guard AI the C2 waiters must not be confused
/// with (their statues are DISABLED drones, not idle guards).
#[test]
fn an_idle_gate_guard_sees_and_attacks_bond_on_the_same_floor() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load("07000006.bin");
    let (saw, attacked) = {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); }
        // The idle gate guards (placements 487/488, the pre-gate list[15]/[16]): enabled IDLE with no
        // switch channel on the spawn floor (39). Drone_Create's min-difficulty gate [M 0x136be0:49]
        // holds back 07000006's six param2=2 placements (442/447/635/648/680/704) at the direct load's
        // difficulty 1, so the guard sits at list[13] (the d1 roster order, verified by probe).
        assert!(sim.drones.list[13].present && sim.drones.list[13].enabled && sim.drones.list[13].state == nf_game::drone::state::IDLE,
            "the idle gate guard (placement 487) is an enabled idle guard");
        let d15_idx = 13;
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, [33.5, 39.0, 22.5], 0.0);
        let mut saw = false; let mut attacked = false;
        for _ in 0..600 {
            sim.step_bond(Input::default(), false, false, false);
            let d15 = &sim.drones.list[d15_idx];
            if d15.los { saw = true; }
            if d15.state != nf_game::drone::state::IDLE && d15.alert >= 1.0 { attacked = true; }
        }
        (saw, attacked)
    };
    assert!(saw, "the idle guard's sight pass runs (vision_on via the IDLE enter, 0x15bf18:30)");
    assert!(attacked, "the idle guard alerts and leaves IDLE for combat (EnemyLookForOpponent → 0x56, 0x177ed0)");
}

/// The user's 07000007 guest retest result: fix_drone4's walk (`a_party_guest_walks_her_mission_route`)
/// still passes with the freeze mirror in — the routed guest walks her route, the PartyGirls stand
/// (game-accurate [M 0x160140 case 3]) and the waiter guests stay disabled on their channels. Locked by
/// fix_drone4_live; this test only re-checks the waiter half on the post-freeze timeline, where the
/// guests that finished their one-shot mission walk stand still (Civilian after arrival).
#[test]
fn the_party_waiter_guests_stay_disabled_and_the_walkers_settle() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load("07000007.bin");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); }
        // The ch-waiter guests (state 0x1) stay disabled — the statues the game hides.
        let waiters = sim.drones.list.iter().filter(|d| d.present && !d.enabled).count();
        assert!(waiters > 0, "waiter guests stay disabled on their channels ({waiters})");
        // Every enabled guest is in a civilian family state (0x10/0x11/0x2e/0x2f/0x30/0x31/0x73/0x74/0xb4),
        // never a combat state — nobody attacks at the party.
        // The unarmed guests (the party girls, weapon 0) must stay civilian; the ARMED guests (the
        // level's disguised guards, weapon != 0) turn combat once an alarm reaches them — with the
        // game's muzzle-light draws in the shared RNG (fix-lighting) that alarm lands inside this
        // window, so the armed guests' states are not part of this gate (their ambush is the
        // mission layer's). Logged for the report either way.
        let bad: Vec<_> = sim.drones.list.iter().enumerate()
            .filter(|(_, d)| d.present && d.enabled && d.weapon == 0
                && !matches!(d.state, 0x10 | 0x11 | 0x2e | 0x2f | 0x30 | 0x31 | 0x73 | 0x74 | 0xb4))
            .map(|(i, d)| (i, d.state, d.alert, d.skin)).collect();
        eprintln!("armed guests in combat: {:?}",
            sim.drones.list.iter().enumerate().filter(|(_, d)| d.present && d.enabled && d.weapon != 0
                && !matches!(d.state, 0x10 | 0x11 | 0x2e | 0x2f | 0x30 | 0x31 | 0x73 | 0x74 | 0xb4))
                .map(|(i, d)| (i, d.state)).collect::<Vec<_>>());
        assert!(bad.is_empty(), "no unarmed enabled guest sits in a combat state ({bad:?})");
    }
}
