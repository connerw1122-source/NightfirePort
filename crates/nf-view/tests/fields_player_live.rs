//! fields-player repro tests: the BLData/HUD field maps of `docs/fields/` (044x-fields-player),
//! driven through the REAL path the viewer uses (`nf_view::load` + `Sim::step_bond`).
//!
//! The evidence:
//! - Fall damage runs the FULL `Player_HandlePain` pipeline: `Player_CollisionHandler` 0x1949a8:117-126
//!   calls `Player_HandlePain(timer - 60, type 6)` — the difficulty factor, the pain flash `BL+0x968`
//!   (`+= min(d·21.333, 128)`, cap 255), the vertical direction bits `BL+0x967 = 2` [M 0x1902c8:121-125]
//!   and NO armour (types 1..7 set armourFraction 0.0 [M 0x1902c8:84-104]). The port used to subtract
//!   the raw damage from health directly: no difficulty, no flash, no bits.
//! - The per-tick clip regen [M Player_Update 0x193600 disasm 0x193eb8-0x19405c]: while the view-anim
//!   state is not 9, clip[0x4A]/clip[0x4E]/clip[0x33] refill on `frame & 3 == 0`, clip[0x4C]/clip[0x4F]
//!   on `frame & 1 == 0` and clip[0x45] (the grenade belt, `BL+0x4F4`) on `frame % 6 == 0` — each
//!   +FRAME_RATE_MUL toward `weapon_data[w].clip_size` (+0x92). The port never regenerated: a used
//!   grenade belt / gadget charge stayed empty forever.
//!
//! Opt-in via `NF_ISO`.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str) -> nf_view::scene::Scene {
    nf_view::load(iso, "07000005.bin", false).expect("load 07000005")
}

fn tick_idle(sim: &mut nf_view::scene::Sim, n: usize) {
    for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
}

/// The fall: teleport Bond 8 units above the spawn pad, armour 50 (a kind-3 pickup's dose), let him
/// land. The damage must arrive through `PlayerHealth::pain_generic(.., 6)`: flash up, vertical bits,
/// armour UNTOUCHED, health down by the difficulty-scaled amount.
#[test]
fn fall_damage_runs_the_full_pain_pipeline() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    assert!(!scene.sim.as_ref().unwrap().disabled, "07000005 starts enabled");
    let landed = {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos[1] += 8.0;
        sim.drones.health.armour = 50.0; // the kind-3 pickup's write [M Pickup_Handler 0x18e650:220-225]
        let before = sim.drones.health.health;
        let mut hit = None;
        for t in 0..300 {
            tick_idle(sim, 1);
            if sim.drones.health.health < before { hit = Some(t); break; }
        }
        assert!(hit.is_some(), "Bond never took the fall damage");
        let h = &sim.drones.health;
        // [M 0x1902c8:126-130] flash += min(d·21.333, 128) — a 17-unit fall (8 units, timer ≈ 77)
        // costs ≈ 10.2 at difficulty 1 → flash ≈ 128+. The pre-fix port left flash 0.
        assert!(h.flash > 100.0, "the fall lights the pain flash (BL+0x968), got {}", h.flash);
        // [M 0x1902c8:121-125] pain type 6 → BL+0x967 = 2 (the vertical pair).
        assert_eq!(h.dir_bits, 2, "a fall sets the vertical damage bits (BL+0x967 = 2)");
        // [M 0x1902c8:84-104] types 1..7 bypass the armour — the 50 stays.
        assert_eq!(h.armour, 50.0, "the fall must not drain the armour");
        sim.drones.health.health
    };
    assert!(landed < 100.0);
}

/// Sister of the fall test through the unit path: bullets (type 0) DO drain the armour first —
/// the same pipeline, the other arm of the game's pain-type switch (see also the in-crate test
/// `nf_game::bullet::tests::armour_bypass_by_pain_type`).
#[test]
fn bullet_damage_still_absorbs_into_armour() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    let sim = scene.sim.as_mut().unwrap();
    let mut h = sim.drones.health;
    h.health = 100.0; // normalize: the test targets the armour arithmetic, not the loadout
    h.armour = 10.0; // below the effective 18 so the remainder reaches health
    let mut rng = sim.drones.rng.clone();
    h.pain(&mut rng, 30.0, 1, 0x0700_0005); // type 0: armour absorbs
    assert!((h.armour - 0.0).abs() < 1e-4, "30 -> 18 effective, the 10 armour empties");
    assert!((h.health - 92.0).abs() < 1e-4, "the remainder 8 reaches health");
}

/// The clip regen [M Player_Update 0x193600 disasm 0x193eb8-0x19405c]: after 60 idle ticks
/// (a second) the grenade belt (clip[0x45], `frame % 6`) and the watch (clip[0x4A], `frame & 3`)
/// have refilled toward their clip_size, while a slot OUTSIDE the game's six (clip[0x44]) stays put.
#[test]
fn clip_slots_regen_on_the_game_schedule() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    let sim = scene.sim.as_mut().unwrap();
    assert!(!sim.disabled);
    sim.drones.inventory.clip[0x45] = 90;
    sim.drones.inventory.clip[0x4A] = 90;
    sim.drones.inventory.clip[0x44] = 90; // control: not one of the game's six slots
    tick_idle(sim, 60);
    assert!(sim.drones.inventory.clip[0x45] > 90, "the grenade belt regenerates (BL+0x4F4, frame % 6)");
    assert!(sim.drones.inventory.clip[0x4A] > 90, "the watch charge regenerates (BL+0x530, frame & 3)");
    assert_eq!(sim.drones.inventory.clip[0x44], 90, "a slot outside the game's six must not regen");
}
