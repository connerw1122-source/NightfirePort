//! The external dev panel's commands (`nf_view::panel`), headless through the real load + tick path. Opt-in: NF_ISO.

use nf_view::panel;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }
fn q(s: &str) -> Vec<(String, String)> { panel::query(s) }

#[test]
fn spawned_npc_is_a_live_drawn_drone() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    assert!(!scene.npc_pool.is_empty(), "07000005 must have a pool of NPCs");
    let skin = scene.npc_pool[0].skin;
    let chars = scene.npc_pool[0].chars.clone();
    // Sister: before the spawn the pool characters are hidden (no drone).
    let opts = nf_view::scene::ViewOpts::default();
    assert!(scene.frame_state(None, 0.0, opts).hidden[chars[0]], "a pool NPC starts hidden");
    let (status, _) = panel::apply(&mut scene, &q(&format!("do=npc&skin={skin:08x}")));
    eprintln!("{status}");
    assert!(status.starts_with("spawned NPC"), "{status}");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..120 { sim.step_bond(Default::default(), false, false, false); }
    let di = sim.char_drone[chars[0]].expect("pool character mapped to the new drone");
    let dr = &sim.drones.list[di];
    assert!(dr.present && dr.enabled && dr.started, "the spawned drone runs its AI");
    assert!(!scene.frame_state(None, 120.0, opts).hidden[chars[0]], "the spawned NPC is drawn");
    // The catalog reports one fewer free NPC of that skin.
    let c = panel::catalog(&scene);
    assert!(c.contains(&format!("\"skin\":\"{skin:08x}\",\"free\":{}", nf_view::scene::NPC_POOL - 1)), "{c}");
    // Clearing hides it again.
    assert_eq!(panel::clear_npcs(&mut scene), 1);
    assert!(scene.frame_state(None, 121.0, opts).hidden[chars[0]]);
}

#[test]
fn give_weapon_and_arena() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let (status, switch) = panel::apply(&mut scene, &q("do=weapon&id=22"));
    eprintln!("{status}");
    assert_eq!(switch, Some(0x16));
    assert!(scene.sim.as_ref().unwrap().drones.inventory.owned[0x16], "SG5 owned after the give");
    let (status, _) = panel::apply(&mut scene, &q("do=arena"));
    eprintln!("{status}");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..60 { sim.step_bond(Default::default(), false, false, false); }
    assert!(sim.drones.list.iter().all(|d| !d.present), "arena: no NPCs left");
    assert!(sim.drones.pickups.iter().all(|p| p.gone), "arena: no pickups left");
    for _ in 0..600 { sim.step_bond(Default::default(), false, false, false); }
    assert!(sim.drones.level_exit.is_none(), "arena must not fire the level exit (a gone LOAD_LEVEL trigger means fired)");
    // Sister: an unknown command is reported, not applied.
    let (status, _) = panel::apply(&mut scene, &q("do=nonsense"));
    assert!(status.contains("unknown"));
}
