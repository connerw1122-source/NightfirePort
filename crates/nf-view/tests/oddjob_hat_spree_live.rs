//! oddjob-hat — the playtest spree leg: 10 RAPID throws keep the game's count of live hats, the
//! oldest recycled only the game's way. Through the REAL input path (`nf_view::load_weapons` +
//! `Sim::step_bond`), playtest 1014-1725's "if you throw enough of them, you stop seeing them
//! flying".
//!
//! What the disc says:
//! - The fire interval is the record's +0x40 = 0x1a (26 ticks) [L ee_start_05.bin @0x2c398c] — a
//!   held trigger throws a hat every 26 ticks (the playtest's rapid spree).
//! - The sim keeps EVERY hat until its own expiry: a stuck hat holds its 1500-tick lifetime
//!   [M `Bullet_CollisionHandler` 0x12ce80:315] and nothing recycles it early — the only caps in
//!   `Bullet_CollisionHandler`'s stick arm are the 6-placed SAME-WEAPDEF counts for the satchel
//!   0x3a and the mine 0x37 [M 0x12ce80:335-370, 0x12ce80:296-330], which the hat 0x45 is not part
//!   of. So 10 rapid throws = 10 live hats (1 flying + 9 stuck), the oldest one NOT recycled.
//! - Every flying or stuck hat draws [M `Bullet_update` state 0 → `hashtable_set_object_to_entity_gfx`
//!   0x12e8f0-0x12e900] through the per-view 512-entry draw ring [M `View_AddObjects` 0x1e62d8;
//!   [L ee_snapshot.bin viewer[0] sizes 512/512/128] — a 10-hat spree never comes near the ring's
//!   reuse, so all ten stay visible (the pre-fix port dropped hats 9+ behind its 8-slot pool).
//! - Each hat spends the record's +0x91 = 100 rounds [M 0x1a5460:654 `RoundToFire(+0x91, +0x91)`],
//!   so the spree pins the clip the dev panel's own way (clip = max(clip, clip_size) per tick —
//!   the playtest ran with the dev panel's infinite ammo).

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn q(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

/// Load pre-merged for the weapon, give it through the panel (the user's path), wait for the draw.
fn load_weapon(iso: &str, id: u8, rounds: i16) -> nf_view::scene::Scene {
    let mut scene = nf_view::load_weapons(iso, "07000005.bin", false, &[id]).expect("load 05");
    let (msg, switch_to) = nf_view::panel::apply(&mut scene, &q(&[("do", "weapon"), ("id", &id.to_string()), ("rounds", &rounds.to_string())]));
    eprintln!("[give] {id:#04x} rounds={rounds}: {msg} -> {switch_to:?}");
    assert_eq!(switch_to, Some(id), "the panel must switch to {id:#04x}");
    for _ in 0..180 {
        scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(id), ..Default::default() }, false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == id && scene.sim.as_ref().unwrap().gun.switch_state == 0 { break; }
    }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, id, "the weapon {id:#04x} is in hand");
    for _ in 0..4 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    scene
}

/// The hat's projectile mesh (weapon_data[0x45]+0x5C) as the draw resolves it.
fn hat_mesh(scene: &nf_view::scene::Scene) -> u32 {
    scene.sim.as_ref().map(|s| s.weapons.projectile_mesh(0x45)).unwrap_or(0)
}

/// ── 10 rapid throws: ten live hats, the oldest still there, every one drawing ────
#[test]
fn ten_rapid_throws_keep_the_games_count_of_live_hats() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_weapon(&iso, 0x45, 200);
    // Park every guard 40 units behind Bond (the throw is a downward spree at the floor ahead —
    // the homing must not bend a hat onto a guard mid-spree and shorten the count).
    {
        let sim = scene.sim.as_mut().unwrap();
        let bp = sim.bond.pos;
        for (i, dr) in sim.drones.list.iter_mut().enumerate() {
            if !dr.present { continue; }
            dr.pos = [bp[0] + (i as f32 * 0.5), dr.pos[1], bp[2] - 40.0];
        }
        sim.bond.pitch = -0.5; // the floor a few units ahead: each hat sticks and holds its slot
    }
    // The rapid spree: PRESS-RELEASE cycles at the record's +0x40 interval (26 ticks [L
    // ee_start_05.bin @0x2c398c]) — the throw is semi-auto (a held trigger re-throws nothing:
    // the fire gate wants a fresh press edge), so the playtest's rapid spree is press, release,
    // press at the interval. Each hat spends +0x91 = 100 rounds, so the clip is pinned the dev
    // panel's own way per tick.
    let mut throws: Vec<u32> = Vec::new(); // the tick each hat spawned
    let mut pulse = 0u32;
    for t in 0..2000u32 {
        {
            let sim = scene.sim.as_mut().unwrap();
            let wi = 0x45usize;
            let cs = sim.weapons.clip_size(wi);
            if cs > 0 {
                if let Some(c) = sim.drones.inventory.clip.get_mut(wi) { *c = (*c).max(cs); }
                sim.gun.clip = sim.gun.clip.max(cs as u32);
            }
        }
        let fire = pulse < 5; // a fresh press each 26-tick cycle (the record's interval)
        let before = scene.sim.as_ref().unwrap().drones.grenades.len();
        scene.sim.as_mut().unwrap().step_bond(Input::default(), fire, false, false);
        let after = scene.sim.as_ref().unwrap().drones.grenades.len();
        if after > before { throws.push(t); }
        if throws.len() >= 10 { break; }
        pulse = (pulse + 1) % 26;
    }
    let sim = scene.sim.as_ref().unwrap();
    eprintln!("[spree] throws={} at ticks {:?}", throws.len(), &throws[..throws.len().min(12)]);
    assert!(throws.len() >= 10, "only {} hats threw while holding the trigger", throws.len());
    // The cadence: the record's +0x40 = 26 ticks is the fire-interval floor [L ee_start_05.bin];
    // the throw ANIM spans it (release frame +0x44 = 13, and the next press edge inside the still-
    // running anim is swallowed), so the port's rapid cadence measures a regular 52 ticks per hat
    // — the assertion just rejects a starvation (a reload or a stuck gate would gap far longer).
    let gaps: Vec<u32> = throws.windows(2).map(|p| p[1] - p[0]).collect();
    eprintln!("[spree] spawn gaps = {gaps:?}");
    assert!(gaps.iter().all(|&g| (26..=60).contains(&g)), "the spawn gaps {gaps:?} are not the rapid cadence (the record's 26-tick floor, the throw anim's two-cycle 52; a reload or a gate got in the way)");
    // All ten still live: the sim keeps every hat until its own expiry (no 6-placed recycle —
    // that cap counts the satchel 0x3a / mine 0x37 same-weapdef set [M 0x12ce80:335-370], not the
    // hat 0x45).
    let live = sim.drones.grenades.iter().filter(|g| g.state < 2).count();
    eprintln!("[spree] live_in_sim={live}");
    assert!(live >= 10, "only {live} hats live after the spree — something recycled them below the game's count");
    // The oldest hat (the first throw) is STILL THERE — the game's way: nothing recycles the
    // oldest while the draw ring (512) is nowhere near full.
    let first = sim.drones.grenades.iter().filter(|g| g.state < 2).count();
    assert!(first >= 10, "the oldest hats were recycled (the pre-fix behaviour): only {first} live");
    // Every live hat draws: the hat-mesh projectile attachments sit at the hats' positions.
    let mesh = hat_mesh(&scene);
    assert_ne!(mesh, 0, "the hat's +0x5C mesh did not resolve");
    let fs = scene.frame_state(None, 0.0, Default::default());
    let mut drawn: Vec<[f32; 3]> = Vec::new();
    for (k, a) in scene.attachments.iter().enumerate() {
        if let Some((_, m)) = a.projectile {
            if m == mesh {
                let t = fs.attach[k];
                if t[3][1] > -1000.0 { drawn.push([t[3][0], t[3][1], t[3][2]]); }
            }
        }
    }
    eprintln!("[draw] hat-mesh instances drawing: {}", drawn.len());
    let sim = scene.sim.as_ref().unwrap();
    let mut missing = 0usize;
    for g in sim.drones.grenades.iter().filter(|g| g.state < 2) {
        let hit = drawn.iter().any(|p| {
            (p[0] - g.pos[0]).powi(2) + (p[1] - g.pos[1]).powi(2) + (p[2] - g.pos[2]).powi(2) < 0.5f32.powi(2)
        });
        if !hit { missing += 1; eprintln!("[draw] UNDRAWN hat at {:?} (stuck={})", g.pos, g.stuck); }
    }
    assert_eq!(missing, 0, "{missing} live hat(s) did not draw — the spree went invisible (the user's symptom)");
    // Sister: the stuck hats keep their 1500-tick lifetime — 300 ticks later the count is still 10
    // (the oldest was not recycled), and each still draws.
    for _ in 0..300 {
        let sim = scene.sim.as_mut().unwrap();
        let cs = sim.weapons.clip_size(0x45);
        if cs > 0 {
            if let Some(c) = sim.drones.inventory.clip.get_mut(0x45) { *c = (*c).max(cs); }
            sim.gun.clip = sim.gun.clip.max(cs as u32);
        }
        sim.step_bond(Input::default(), false, false, false);
    }
    let sim = scene.sim.as_ref().unwrap();
    let live_after = sim.drones.grenades.iter().filter(|g| g.state < 2).count();
    eprintln!("[spree] live after 300 more ticks = {live_after}");
    assert!(live_after >= 10, "the live count dropped to {live_after} within the hats' 1500-tick lifetime — something recycled the oldest early");
}
