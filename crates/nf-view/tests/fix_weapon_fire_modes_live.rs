//! fix-weapon-fire-modes + fix-fire-modes2: the MGL's impact/delay pair and the AT-420 Sentinel's
//! guided missile camera, through the real input path (`nf_view::load_with_carry` +
//! `Sim::step_bond`).
//!
//! Ground truth: docs/live/2026-10-13-weapon-modes-pcsx2.md — the Militek MGL is a TWIN pair,
//! 0x2A 'Grenade' = IMPACT detonation, 0x2B 'Delay' = a ~210-tick fuse; the AT-420 Sentinel 0x2C
//! 'Guided' puts the view ON the rocket and the look input steers it, until impact (red static,
//! then Bond's view). The user: "The MGL grenade launcher is always delayed when I believe it
//! should be impact for one of the modes."
//!
//! fix-fire-modes2: every impact blast flows through master's single path — the bullet world-hit
//! queues `pending_blasts` (`Bullet_CollisionHandler` 0x12d068-0x12d0bc [M]) and the grenade
//! contact blasts from the `GrenadeEvent::Bounce` arm, both drained by ONE `spawn_blast`
//! (`Explode_Create` 0x17cbd0 [M]) per contact. The old branch's inline `blast_at` and the
//! `Grenade::explode_on_impact` twin are gone: these tests pin exactly ONE explosion entry and
//! ONE damage application per impact.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str) -> nf_view::scene::Scene {
    nf_view::load_with_carry(iso, "07000005.bin", false, None).expect("load 07000005")
}

fn tick(scene: &mut nf_view::scene::Scene, n: u32) {
    if let Some(sim) = scene.sim.as_mut() {
        for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
    }
}

/// Pitches the view down over `ramp` ticks (the real look input) so the round hits the floor a
/// couple of units ahead: the blast lands inside its own radius of Bond (the damage application is
/// observable), and a homing rocket's 0.66-dot cone pointed at the floor excludes the standing
/// guards [M Bullet_homing 0x12f358's cone gate].
fn aim_down(scene: &mut nf_view::scene::Scene, ramp: u32, look: f32) {
    for _ in 0..ramp {
        let sim = scene.sim.as_mut().unwrap();
        let mut input = Input::default();
        input.look = look;
        sim.step_bond(input, false, false, false);
    }
}

/// The MGL's two modes through the real fire path: 0x2A 'Grenade' detonates ON the wall
/// (`+0x70 & 0x4` → `Explode_Create(wd+8, wd+8, wd+0xC, …)` [M Bullet_CollisionHandler
/// 0x12ce80:194-206; L ee_start_05.bin 0x2A's +0x70 = 0x00000004, radius 6 / damage 75]),
/// while the 'Delay' twin 0x2B (+0x70 = 0x00000280 — 0x100 bounce + 0x200 fuse, no 0x4 [L])
/// bounces and dies on its 210-tick fuse (`Bullet_DoTrails` 0x12d890:277 BU+0xFC −= 1/tick,
/// exploded at < 0 [M]; the PCSX2 pass measured ~3–4 s ≈ 210 ticks). The mode switch is the real
/// B press: 0x2A's twin `+0x05` = 1 → the switch request 0x2B (`Player_Weapon` 0x1a24c8:246-260),
/// gated on `Player_WeaponHasAmmo` (the twin needs rounds — the PCSX2 pass saw the press do
/// nothing while 0x2B was empty).
///
/// fix-fire-modes2: the contact blast runs through master's ONE path (the `GrenadeEvent::Bounce`
/// arm reads the record's +0x70 and queues `pending_blasts`; ONE `spawn_blast` drains it) — so the
/// impact shot must produce exactly ONE explosion entry and ONE damage application.
#[test]
fn mgl_grenade_0x2a_detonates_on_impact_and_delay_0x2b_runs_its_fuse() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    tick(&mut scene, 30);
    aim_down(&mut scene, 40, -1.0);

    let explosion_tick = |scene: &mut nf_view::scene::Scene, fire_ticks: u32|
        -> Option<(u32, usize, usize, (f32, f32, usize, u16))> {
        // Fire once (the real trigger through step_bond's `trigger`), then watch the blast.
        let sim = scene.sim.as_mut().unwrap();
        sim.gun.reload_state = 0;
        sim.gun.fire_state = false;
        sim.gun.clip = 6;
        sim.gun.pending = 0;
        let owned = sim.drones.inventory.owned.len();
        for w in [0x2ausize, 0x2b] {
            if w < owned { sim.drones.inventory.owned[w] = true; }
            if let Some(c) = sim.drones.inventory.clip.get_mut(w) { *c = 6; }
        }
        let mut input = Input::default();
        input.fire = true;
        let mut saw_grenade = false;
        let mut boom = None;
        for t in 0..fire_ticks {
            let sim = scene.sim.as_mut().unwrap();
            let (ex0, ph0) = (sim.drones.explosions.len(), sim.drones.player_hits.len());
            sim.step_bond(if t == 0 { input } else { Input::default() }, t == 0, false, false);
            let sim = scene.sim.as_mut().unwrap();
            if sim.drones.grenades.iter().any(|g| g.weapon_type == 0x2a || g.weapon_type == 0x2b) { saw_grenade = true; }
            if sim.drones.explosions.len() > ex0 {
                let delta = sim.drones.explosions.len() - ex0;
                let ph_delta = sim.drones.player_hits.len() - ph0;
                let e = *sim.drones.explosions.last().unwrap();
                boom = Some((t, delta, ph_delta, (e.1, e.2, e.3, e.4)));
                break;
            }
        }
        assert!(saw_grenade, "the MGL fired a grenade projectile");
        boom
    };

    // Mode 1: 0x2A 'Grenade' — the wall detonates it, fast (the PCSX2 pass: "on contact, within
    // about 0.3 s"; point-blank the round reaches the wall in a few ticks at 0.44 u/tick).
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.gun.weapon = 0x2a;
        sim.gun.switch_request = 0x2a;
        sim.gun.switch_state = 0;
    }
    tick(&mut scene, 10);
    let boom = explosion_tick(&mut scene, 240).expect("0x2A 'Grenade' explodes on impact");
    assert!(boom.0 < 120, "the impact mode detonated on the wall within 2 s, got tick {}", boom.0);
    // fix-fire-modes2: exactly ONE explosion entry and ONE damage application per contact.
    assert_eq!(boom.1, 1, "exactly ONE explosion entry for the 0x2A contact (the old inline blast_at + queued spawn_blast would double it)");
    assert_eq!(boom.2, 1, "exactly ONE damage application (one player_hits entry at the blast tick)");
    assert_eq!(boom.3, (6.0, 75.0, usize::MAX, 0x2a), "the blast entry is the record's radius/damage with the player as owner [L ee_start_05.bin 0x2A: r6/d75]");

    // Mode 2: the REAL mode press (the port's B key on the game's mode action) swaps to the twin
    // 0x2B 'Delay' (twin +0x05 = 1 [L]) — the switch request goes through the normal machinery.
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.gun.switch_state = 0;
        sim.gun.reload_state = 0;
        sim.gun.fire_state = false;
        let mut input = Input::default();
        input.mode_switch = true;
        sim.step_bond(input, false, false, false);
        assert_eq!(sim.gun.switch_request, 0x2b, "the mode press on 0x2A requests the 'Delay' twin 0x2B [M 0x1a24c8:246-260]");
    }
    tick(&mut scene, 200);
    {
        let sim = scene.sim.as_mut().unwrap();
        assert_eq!(sim.gun.weapon, 0x2b, "the twin swap completed onto the 'Delay' record");
    }
    // The 'Delay' round must NOT detonate on the wall: it bounces and runs its 210-tick fuse.
    // Watch only the contact window first (an impact would land well under 120 ticks, like the
    // 0x2A shot), then let the fuse run out.
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.gun.reload_state = 0;
        sim.gun.fire_state = false;
        sim.gun.clip = 6;
        sim.gun.pending = 0;
        let before = sim.drones.explosions.len();
        let mut input = Input::default();
        input.fire = true;
        let mut contact_boom = None;
        for t in 0..150 {
            let sim = scene.sim.as_mut().unwrap();
            sim.step_bond(if t == 0 { input } else { Input::default() }, t == 0, false, false);
            if sim.drones.explosions.len() > before { contact_boom = Some(t); break; }
        }
        assert!(contact_boom.is_none(), "0x2B 'Delay' must not explode on contact — the wall shot at tick {:?} detonated early (the user's symptom is the reverse pairing)", contact_boom);
    }
    let mut fuse_boom = None;
    for _ in 0..260 {
        let sim = scene.sim.as_mut().unwrap();
        let ex0 = sim.drones.explosions.len();
        sim.step_bond(Default::default(), false, false, false);
        let sim = scene.sim.as_mut().unwrap();
        if sim.drones.explosions.len() > ex0 { fuse_boom = Some(sim.drones.explosions.len() - ex0); break; }
    }
    let fuse_delta = fuse_boom.expect("the 'Delay' round dies on its fuse (~210 ticks)");
    assert_eq!(fuse_delta, 1, "exactly ONE explosion entry for the fuse death too");
}

/// Sister (unit, no ISO): the grenade's event contract the Sim's arms consume. The record's
/// `+0x70` flags live in the weapon table (the Sim reads them — the old `Grenade::explode_on_impact`
/// twin is gone): a `+0x4` record's round reports ONE `Bounce` at the contact (the Sim's Bounce arm
/// turns it into the blast [M Bullet_CollisionHandler 0x12d068-0x12d0bc]) and never a contact
/// Explode; its range-out is the ONE Explode event the Sim's `+0x200` gate removes SILENTLY
/// [M Bullet_update 0x12e7b0:158-162 + the Bullet_DoTrails fuse gate 0x12d890:262-281]; a fuse
/// record bounces and explodes only at the fuse.
#[test]
fn grenade_events_match_the_records() {
    use nf_game::grenade::{Grenade, GrenadeEvent};
    use nf_game::world::{CollisionWorld, V3};
    let wall = CollisionWorld::from_triangles(vec![
        [[10.0, -10.0, -10.0], [10.0, 10.0, 10.0], [10.0, 10.0, -10.0]],
        [[10.0, -10.0, -10.0], [-10.0, -10.0, 10.0], [-10.0, 10.0, 10.0]],
    ]);
    let mk = |impact: bool| Grenade {
        pos: [0.0, 1.0, 0.0], velocity: [0.44, 0.0, 0.0], speed: 0.44, travelled: 0.0,
        range: if impact { 909.0 } else { 210.0 }, damage: 75.0, radius: 6.0,
        weapon_type: if impact { 0x2a } else { 0x2b }, mesh: 0x0200_0860, owner: usize::MAX,
        state: 0, bounces: 0, smoke_popped: false,
        resting: false, // fix-grenade-throw's slow-bounce latch (merged alongside)
        stuck: false, seek_target: None, // fix-oddjob-hat
    };
    let mut rng = nf_game::drone::Rng::new(0);
    // 0x2A: the first contact is ONE Bounce event (the Sim's Bounce arm blasts on it)…
    let mut g = mk(true);
    let mut bounce_at = None;
    let mut others = Vec::new();
    for t in 0..100 {
        match g.update(&wall, &mut rng) {
            Some(GrenadeEvent::Bounce(_)) => { bounce_at = Some(t); break; }
            Some(other) => others.push(other),
            None => {}
        }
    }
    assert_eq!(bounce_at, Some(23), "the impact round reaches the wall at the first tick's half-speed draw [M 0x12ce80:194-206]");
    assert!(others.is_empty(), "no contact Explode before the bounce — the blast is the Sim's Bounce arm, not the grenade's");
    // …and its range-out emits exactly ONE Explode (the Sim's +0x200 gate removes it silently —
    // the game's state-4 arm has no Explode_Create [M 0x12e7b0:158-162]).
    let mut g = mk(true);
    let mut events = Vec::new();
    for _ in 0..950 { if let Some(ev) = g.update(&CollisionWorld::empty(), &mut rng) { events.push(ev); } }
    assert_eq!(events.len(), 1, "the range-out is the single death event");
    assert!(matches!(events[0], GrenadeEvent::Explode { .. }), "and it is the Explode the Sim silences");
    assert_eq!(g.state, 2, "the round is dead after it");
    // 0x2B: a wall touch is a BOUNCE, the explosion only at the 210-tick fuse.
    let mut g = mk(false);
    let mut bounced = false;
    let mut events = Vec::new();
    for _ in 0..209 { if let Some(ev) = g.update(&wall, &mut rng) { if matches!(ev, GrenadeEvent::Bounce(_)) { bounced = true; } events.push(ev); } }
    assert!(bounced, "the delay grenade bounces off the wall");
    assert!(!events.iter().any(|e| matches!(e, GrenadeEvent::Explode { .. })), "no wall detonation before the fuse");
    let mut boomed = false;
    for _ in 0..4 { if let Some(GrenadeEvent::Explode { .. }) = g.update(&CollisionWorld::empty(), &mut rng) { boomed = true; break; } }
    assert!(boomed, "the delay grenade explodes at its 210-tick fuse [M Bullet_DoTrails 0x12d890:277]");
}

/// The AT-420 Sentinel's guided missile: firing 'Guided' 0x2C (`+0x6C = 0x12204` — the 0x4 bit [L])
/// hands the view to the rocket (`Bullet_init` 0x12c5a0:234-254: `Player_ChangeSubState
/// (owner, 10)`, `BLData+0x878 = the bullet`, `Player_SetCamMode(BLData, 0xc)`), the look steers it
/// (`Bullet_update` 0x12e7b0:177-244), and the impact freezes the view for the 15 ticks of
/// `BLData+0x944` before the normal view returns (`Bullet_Delete` 0x12e660:26-30 +
/// `Player_Update` 0x193600:242-256). Sister: the 'Unguided' twin 0x2D (`+0x6C = 0x1a200`, no 0x4
/// [L]) never rides the camera — the PCSX2 pass saw the camera only on 0x2C.
///
/// fix-fire-modes2: the rocket's world hit queues exactly ONE blast through `pending_blasts`
/// (`+0x70 & 4` + `b.bounces == 0`, `Bullet_CollisionHandler` 0x12d068-0x12d0bc [M]; 0x2C carries
/// 0x4, radius 8 / damage 100 [L ee_start_05.bin]) — one explosion entry for the whole flight.
#[test]
fn the_sentinel_guided_mode_rides_the_missile_and_the_unguided_twin_does_not() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    tick(&mut scene, 30);
    {
        let sim = scene.sim.as_mut().unwrap();
        let owned = sim.drones.inventory.owned.len();
        for w in [0x2cusize, 0x2d] {
            if w < owned { sim.drones.inventory.owned[w] = true; }
            if let Some(c) = sim.drones.inventory.clip.get_mut(w) { *c = 4; }
        }
        sim.gun.weapon = 0x2c;
        sim.gun.switch_request = 0x2c;
        sim.gun.switch_state = 0;
        sim.gun.reload_state = 0;
        sim.gun.fire_state = false;
        sim.gun.clip = 4;
    }
    tick(&mut scene, 10);
    // Fire once: the camera must hand over to the missile.
    let mut input = Input::default();
    input.fire = true;
    let sim = scene.sim.as_mut().unwrap();
    sim.step_bond(input, true, false, false);
    tick(&mut scene, 2);
    let sim = scene.sim.as_mut().unwrap();
    assert!(sim.missile_cam.is_some(), "the 'Guided' shot puts the camera on the missile [M 0x12c5a0:234-254]");
    let cam0 = sim.missile_cam.unwrap();
    assert!(cam0.riding, "the missile is alive and the view rides it");
    // The view override tracks the missile as it flies (speed +0x20 = 0.25 u/tick [L]).
    for _ in 0..20 { step_default(scene.sim.as_mut().unwrap()); }
    let sim = scene.sim.as_mut().unwrap();
    let cam1 = sim.missile_cam.expect("still riding after 20 ticks (range 200, speed 0.25)");
    let moved = ((cam1.pos[0] - cam0.pos[0]).powi(2) + (cam1.pos[1] - cam0.pos[1]).powi(2) + (cam1.pos[2] - cam0.pos[2]).powi(2)).sqrt();
    assert!(moved > 1.0, "the camera rides the flying missile (moved {:.2} units in 20 ticks)", moved);
    // The look input steers the missile's direction [M 0x12e7b0:177-244].
    let d0 = cam1.dir;
    for _ in 0..12 {
        let sim = scene.sim.as_mut().unwrap();
        let mut steer = Input::default();
        steer.turn = 1.0;
        if sim.missile_cam.is_some() && sim.missile_cam.unwrap().riding { sim.step_bond(steer, false, false, false); } else { break; }
    }
    let sim = scene.sim.as_mut().unwrap();
    if let Some(cam2) = sim.missile_cam {
        let dot = d0[0] * cam2.dir[0] + d0[1] * cam2.dir[1] + d0[2] * cam2.dir[2];
        assert!(dot < 0.999, "holding the turn input curves the missile (dot {:.5})", dot);
    }
    // The impact: exactly ONE explosion entry (the queued blast), then the frozen view counts
    // down and the normal view returns.
    let mut blasts = Vec::new();
    let mut restored_at = None;
    for _ in 0..300 {
        let sim = scene.sim.as_mut().unwrap();
        let ex0 = sim.drones.explosions.len();
        sim.step_bond(Default::default(), false, false, false);
        let sim = scene.sim.as_mut().unwrap();
        if sim.drones.explosions.len() > ex0 {
            for e in &sim.drones.explosions[ex0..] { blasts.push((e.1, e.2, e.3, e.4)); }
        }
        if sim.missile_cam.is_none() { restored_at = Some(blasts.len()); break; }
        let cam = sim.missile_cam.unwrap();
        if !cam.riding { assert!(cam.restore <= 15, "the static countdown starts at 15 [M 0x12e660:28]"); }
    }
    assert!(restored_at.is_some(), "the view returned to Bond after the missile died");
    assert_eq!(blasts.len(), 1, "exactly ONE explosion entry for the rocket hit (fix-fire-modes2: the inline blast_at double is gone)");
    assert_eq!(blasts[0], (8.0, 100.0, usize::MAX, 0x2c), "the blast is the record's radius/damage with the player as owner [L ee_start_05.bin 0x2C: r8/d100]");
}

fn step_default(sim: &mut nf_view::scene::Sim) { sim.step_bond(Default::default(), false, false, false); }

/// fix-fire-modes2's headline test: one unguided rocket (0x2D, `+0x70 = 0x4`, radius 8 / damage
/// 100 [L ee_start_05.bin]) fired point-blank at a wall — exactly ONE explosion entry and exactly
/// ONE damage application. The pre-merge tree ran the branch's inline `blast_at` AND master's
/// queued `spawn_blast` on the same contact: two `Explode_Propagate` passes (the inline entry was
/// wiped by the per-tick `explosions.clear()` before the view saw it, but the damage doubled).
#[test]
fn rocket_hit_applies_the_blast_exactly_once() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    tick(&mut scene, 30);
    aim_down(&mut scene, 40, -1.0);
    {
        let sim = scene.sim.as_mut().unwrap();
        let owned = sim.drones.inventory.owned.len();
        if 0x2d < owned { sim.drones.inventory.owned[0x2d] = true; }
        if let Some(c) = sim.drones.inventory.clip.get_mut(0x2d) { *c = 4; }
        sim.gun.weapon = 0x2d;
        sim.gun.switch_request = 0x2d;
        sim.gun.switch_state = 0;
        sim.gun.reload_state = 0;
        sim.gun.fire_state = false;
        sim.gun.clip = 4;
    }
    tick(&mut scene, 10);
    let mut input = Input::default();
    input.fire = true;
    let mut boom = None;
    for t in 0..240 {
        let sim = scene.sim.as_mut().unwrap();
        let (ex0, ph0) = (sim.drones.explosions.len(), sim.drones.player_hits.len());
        sim.step_bond(if t == 0 { input } else { Input::default() }, t == 0, false, false);
        let sim = scene.sim.as_mut().unwrap();
        if sim.drones.explosions.len() > ex0 {
            let e = *sim.drones.explosions.last().unwrap();
            boom = Some((sim.drones.explosions.len() - ex0, sim.drones.player_hits.len() - ph0, (e.1, e.2, e.3, e.4)));
            break;
        }
    }
    let (ex_delta, ph_delta, entry) = boom.expect("the point-blank rocket hits the wall and blasts");
    assert_eq!(ex_delta, 1, "exactly ONE explosion entry for the rocket hit");
    assert_eq!(ph_delta, 1, "exactly ONE damage application (the blast radius 8 covers point-blank Bond)");
    assert_eq!(entry, (8.0, 100.0, usize::MAX, 0x2d), "the blast is the record's radius/damage with the player as owner [L ee_start_05.bin 0x2D: r8/d100]");
    // Sister: nothing re-blasts afterwards — the dead bullet's contact is consumed once (the
    // per-tick `explosions.clear()` wipes the list each tick, so count the per-tick deltas).
    let mut extra = 0;
    for _ in 0..60 {
        step_default(scene.sim.as_mut().unwrap());
        // The grenade section clears `explosions` at the head of every tick and the drain pushes
        // after it — the end-of-tick length IS that tick's blast count.
        extra += scene.sim.as_ref().unwrap().drones.explosions.len();
    }
    assert_eq!(extra, 0, "no second blast from the same contact");
}

/// Sister: the 'Unguided' twin 0x2D (+0x6C = 0x1a200 — no 0x4 bit [L ee_start_05.bin]) fires a
/// plain rocket: no camera handover ever.
#[test]
fn the_unguided_twin_never_rides_the_camera() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    tick(&mut scene, 30);
    {
        let sim = scene.sim.as_mut().unwrap();
        let owned = sim.drones.inventory.owned.len();
        for w in [0x2cusize, 0x2d] {
            if w < owned { sim.drones.inventory.owned[w] = true; }
            if let Some(c) = sim.drones.inventory.clip.get_mut(w) { *c = 4; }
        }
        sim.gun.weapon = 0x2d;
        sim.gun.switch_request = 0x2d;
        sim.gun.switch_state = 0;
        sim.gun.reload_state = 0;
        sim.gun.fire_state = false;
        sim.gun.clip = 4;
    }
    tick(&mut scene, 10);
    let mut input = Input::default();
    input.fire = true;
    let sim = scene.sim.as_mut().unwrap();
    sim.step_bond(input, true, false, false);
    for _ in 0..60 { step_default(scene.sim.as_mut().unwrap()); }
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.missile_cam.is_none(), "the 'Unguided' record has no `+0x6C & 4` — the camera stays with Bond [L record; M 0x12c5a0:234]");
}
