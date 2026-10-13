//! fix-world-fx live tests: the presentation FX through the REAL paths — the block-0x30 emitter
//! defs off the disc (`parsemap_block_particles` 0x1d2060 → `Emitter_LoadDefs` 0x17ab40), the FX
//! placements through `nf_view::load` + `Sim::step_bond` (the real input-path sim), and the
//! behaviours against their decompiles (docs/atlas/emitter.md, ripples.md, flicker.md, cloud.md,
//! apocalypse.md). Opt-in via NF_ISO.

use nf_formats::level::Level;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn parse_level(iso: &str, name: &str) -> Level {
    let img = nf_formats::iso9660::IsoImage::open(iso).unwrap();
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).unwrap();
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name)).unwrap_or_else(|| panic!("no {name}"));
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    Level::parse(&data).unwrap()
}

fn step(scene: &mut nf_view::scene::Scene, n: u32) {
    if let Some(sim) = scene.sim.as_mut() {
        for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
    }
}

/// The block-0x30 defs parse off the disc and every 0xf2 placement's def hash resolves
/// (`Emitter_Create` 0x17ae00 looks the placement's param 0 up in the loaded table). Before
/// fix-world-fx the block was skipped entirely (levels.md §7.1: 0xf2 "unhandled") — this is the
/// fail-first: no defs, no emitters.
#[test]
fn emitter_defs_parse_and_the_placements_resolve() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for name in ["07000014.bin", "07000015.bin", "07000002.bin"] {
        let level = parse_level(&iso, name);
        let m = level.main_map().unwrap();
        let defs: Vec<_> = level.maps.iter().flat_map(|mp| mp.emitter_defs.iter()).collect();
        assert!(!defs.is_empty(), "{name}: the block-0x30 emitter defs parse");
        for d in &defs {
            assert!(d.max > 0, "{name}: def {:08x} has a slot count", d.hash);
            assert!(!d.keys.is_empty(), "{name}: def {:08x} has colour keys", d.hash);
        }
        let emitters: Vec<_> = level.maps[m].placements.iter()
            .filter(|p| p.flags & 0xff == 0xf2).collect();
        assert!(!emitters.is_empty(), "{name}: has 0xf2 placements");
        for p in &emitters {
            let hash = p.params.iter().find(|(i, _)| *i == 0).map(|&(_, v)| v).unwrap_or(0);
            assert!(defs.iter().any(|d| d.hash == hash), "{name}: emitter placement def {:08x} resolves", hash);
        }
    }
}

/// The ripples through the real sim (07000001's pools): the FX attach sees the 0x21 placements and
/// 600 idle ticks leave ripple rings in the gas pool (before the fix: no puffs — nothing spawned).
#[test]
fn ripples_run_through_the_real_sim() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000001.bin", false).unwrap();
    let sim = scene.sim.as_ref().unwrap();
    assert!(!sim.drones.fx.ripples.is_empty(), "07000001 has 0x21 placements (levels.md §7.1)");
    let mut occupied = 0;
    for i in 0..6 {
        step(&mut scene, 100);
        let Some(sim) = scene.sim.as_ref() else { continue };
        if !sim.drones.gas.puffs.is_empty() { occupied += 1; }
        let _ = i;
    }
    assert!(occupied > 0, "600 idle ticks leave a ripple ring alive in the pool");
}

/// The emitters through the real sim on 07000014: the level's emitters are channel-gated (params 2/3),
/// so the test fires each emitter's ON channel — the same `switch_channels` write the game's triggers
/// and scripts make (`Emitter_Update` 0x17b080 reads it every tick) — and counts the sprites.
#[test]
fn emitters_run_through_the_real_sim() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000014.bin", false).unwrap();
    let sim = scene.sim.as_ref().unwrap();
    assert!(!sim.drones.fx.emitters.is_empty(), "07000014 has 0xf2 placements (levels.md §7.1)");
    let channels: Vec<u16> = sim.drones.fx.emitters.iter().map(|e| e.ch_on).filter(|&c| c != 0).collect();
    for c in channels { scene.sim.as_mut().unwrap().drones.channels.set(c as u32, 1, 0); }
    let mut sprites = 0;
    for _ in 0..6 {
        step(&mut scene, 100);
        sprites += scene.sim.as_ref().map(|s| s.drones.fx.particle_sprites().len()).unwrap_or(0);
    }
    assert!(sprites > 0, "600 idle ticks with the ON channels set produce emitter particles (got {sprites})");
}

/// The apocalypse placements carry their countdown (`Apocalypse_Create` 0x18dcc0) and latch on the
/// channel through the real sim's Channels (`Apocalypse_Update` 0x18d4f0).
#[test]
fn apocalypse_placements_carry_their_countdown() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000015.bin", false).unwrap();
    let sim = scene.sim.as_ref().unwrap();
    assert!(!sim.drones.fx.apocalypses.is_empty(), "07000015 has 0x4a placements (levels.md §7.1)");
    let a = &sim.drones.fx.apocalypses[0];
    assert!((a.remaining - a.total).abs() < 1e-4 && a.total > 0.0, "the countdown starts at its total (param1·60)");
    assert!(!a.armed, "unlatched before the channel fires");
    let ch = a.channel as u32;
    let sim = scene.sim.as_mut().unwrap();
    sim.drones.channels.set(ch, 1, sim.drones.frame);
    step(&mut scene, 30);
    let sim = scene.sim.as_ref().unwrap();
    let a = &sim.drones.fx.apocalypses[0];
    assert!(a.armed, "the channel latch sets through the real Channels");
    assert!(a.remaining < a.total, "the countdown runs");
    assert!(a.fade > 0.0, "the whiteout value rises (remaining·5/total)");
}

/// The cloud drift phases accumulate and wrap (`cloud_update` 0x18d768) through the real sim.
#[test]
fn clouds_drift_through_the_real_sim() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000006.bin", false).unwrap();
    let sim = scene.sim.as_ref().unwrap();
    assert!(!sim.drones.fx.clouds.is_empty(), "07000006 has 0xd4 placements (levels.md §7.1)");
    let before: Vec<[f32; 2]> = sim.drones.fx.clouds.iter().map(|c| c.phase).collect();
    step(&mut scene, 300);
    let sim = scene.sim.as_ref().unwrap();
    let after: Vec<[f32; 2]> = sim.drones.fx.clouds.iter().map(|c| c.phase).collect();
    assert!(before != after, "300 ticks of drift move the phases");
    for c in &sim.drones.fx.clouds {
        assert!(c.phase[0] >= 0.0 && c.phase[0] < c.wrap.max(1e-6), "the phase stays inside the fmod wrap");
    }
}
