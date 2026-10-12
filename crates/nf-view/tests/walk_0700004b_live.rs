//! map-finale: the 0700004b survey (docs/levels/0700004b.md). The multiplayer arena: it loads but
//! has no campaign start, no triggers, no doors and no drones — the sister assertion for the
//! story-order claims in the level doc. Opt-in via NF_ISO.

#[test]
fn the_multiplayer_arena_loads_without_a_campaign_path() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "0700004b.bin", false).expect("load 0700004b");
    let sim = scene.sim.as_ref().unwrap();
    assert!(scene.start.is_none(), "no type-0x24 campaign start [D survey — Bond stands at the origin]");
    assert!(sim.drones.triggers.is_empty(), "no triggers [D]");
    assert!(sim.drones.doors.list.is_empty(), "no doors [D]");
    assert!(sim.drones.list.is_empty(), "no drones [D]");
    assert!(!sim.drones.pickups.is_empty(), "the arena's pickups parsed [D]");
    // The mission layer is inert: no record for 0x0700004b ([L] the 24-record table), so no monitor
    // outcome and no level exit can ever fire here.
    assert_eq!(sim.drones.level_exit, None, "no mission end on an MP wrapper");
}
