//! mp-map-objects (task 744) — probe: what does the port's sim already do with the MP maps'
//! movers? Playtest 1016-1355: "on the multiplayer maps, like Gondola (Skyrail), it's missing
//! the actual gondolas. It has these things called remote tanks and like helicopters that you
//! should be able to pilot, and you can't."
//!
//! Evidence so far [D census, work/ scratch mp_census]:
//! - Skyrail 0x07000024 main-map placement [4]: kind 0xD9 SP_CreateScriptPlayer, params
//!   {0: 0x06000058 script, 2: 8 (loop mode)}, object OBJECT_CableCarbody 0x0200037a,
//!   t = (0,0,0) — the gondola carriage is a script player entity.
//! - Ravine 0x0700004b: four kind-0xE2 SS placements with 25-key paths (block 0x19), rate 8500.
//! - Sub Pen 0x07000025: three kind-0xD9 lift script players + touch-plate triggers.
//! - Every arena: 4 kind-0xDA Car_Create (RCCar) placements — the remote tank.
//!
//! Run: NF_ISO=<iso> cargo test --offline --release -p nf-game --test mp_map_objects_live

use nf_formats::{disc, iso9660::IsoImage, level::Level};

fn load_arena(code: u32) -> (IsoImage, Level) {
    let path = std::env::var_os("NF_ISO").expect("NF_ISO not set");
    let iso = IsoImage::open(path.to_str().unwrap()).unwrap();
    let (entries, bin) = disc::files_bin_entries(&iso).unwrap();
    let name = format!("{code:08x}.bin");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(&name))
        .unwrap_or_else(|| panic!("arena {code:08x}.bin missing"));
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    let level = Level::parse(&data).unwrap();
    (iso, level)
}

#[test]
fn probe_skyrail_script_players() {
    let (_iso, level) = load_arena(0x07000024);
    let m = level.main_map().unwrap();
    let ps = &level.maps[m].placements;
    let script_players: Vec<_> = ps.iter().enumerate().filter(|(_, p)| p.flags & 0xff == 0xd9)
        .map(|(i, p)| (i, p.params.clone())).collect();
    println!("skyrail: {} placements, {} 0xD9 script players", ps.len(), script_players.len());
    for (i, params) in &script_players {
        println!("  placement {i}: {:?}", params.iter().map(|(k, v)| format!("{k}: {v:#x}")).collect::<Vec<_>>());
    }
    let has_gondola_script = level.nis.contains_key(&0x0600_0058);
    println!("nis lib: {} scripts, has 0x06000058: {has_gondola_script}", level.nis.len());
    if let Some(n) = level.nis.get(&0x0600_0058) {
        println!("  0x06000058: streams {}, length {}", n.streams.len(), n.length);
    }
    assert!(has_gondola_script, "skyrail: gondola script 0x06000058 not in the level's nis lib");
    assert_eq!(script_players.len(), 1, "skyrail: expected exactly one 0xD9 placement (the gondola) [D]");
    let ss: Vec<_> = ps.iter().filter(|p| p.flags & 0xff == 0xe2).collect();
    println!("skyrail: {} 0xE2 SS placements", ss.len());
}

#[test]
fn probe_ravine_ss_movers() {
    let (_iso, level) = load_arena(0x0700004b);
    let m = level.main_map().unwrap();
    let ps = &level.maps[m].placements;
    let movers: Vec<_> = ps.iter().enumerate().filter(|(_, p)| p.flags & 0xff == 0xe2)
        .map(|(i, p)| {
            let keys = p.extra[7] & 0xffff;
            let rate = p.extra[7] >> 16;
            let path = p.extra[8] & 0xffff;
            (i, keys, rate, path)
        }).collect();
    println!("ravine: {} 0xE2 SS placements (idx, keys, rate, path):", movers.len());
    for mv in &movers { println!("  {mv:?}"); }
    let blocks = level.path_blocks();
    println!("path blocks: {}", blocks.len());
    assert!(movers.iter().any(|&(_, keys, rate, _)| keys == 25 && rate == 8500),
        "ravine: expected the cable-car SS placements (25 keys, rate 8500) [D]");
}

#[test]
fn probe_cars_on_all_arenas() {
    for code in [0x07000023u32, 0x07000024, 0x07000026, 0x07000027, 0x07000028, 0x07000029, 0x0700004b] {
        let (_iso, level) = load_arena(code);
        let m = level.main_map().unwrap();
        let cars = level.maps[m].placements.iter().filter(|p| p.flags & 0xff == 0xda).count();
        println!("{code:08x}: {cars} 0xDA Car placements");
        assert_eq!(cars, 4, "arena {code:08x}: expected 4 Car_Create placements [D census]");
    }
    // Sub Pen 07000025 has none (the census: its movers are the script-player lifts) [D].
    let (_iso, level) = load_arena(0x07000025);
    let m = level.main_map().unwrap();
    let cars = level.maps[m].placements.iter().filter(|p| p.flags & 0xff == 0xda).count();
    assert_eq!(cars, 0, "Sub Pen: expected no Car_Create placements [D census]");
}

/// A minimal `nis::World` for the script-player probe.
struct ProbeWorld;
impl nf_game::nis::World for ProbeWorld {
    fn anim_length(&self, _script: u32) -> Option<u16> { Some(100) }
    fn discrete0(&self, _skin: u32, _script: u32, _frame: f32) -> Option<nf_formats::anim::Mat4> { None }
    fn level(&self) -> u32 { 0x0700_0024 }
}

/// Does the gondola script player exist and move? Skyrail placement [4] (kind 0xD9, script
/// 0x06000058, mode 8 loop, mask 0) is the gondola carriage [D census].
#[test]
fn probe_gondola_script_ticks() {
    let (_iso, level) = load_arena(0x07000024);
    let m = level.main_map().unwrap();
    let ps = &level.maps[m].placements;
    let lib: std::collections::HashMap<u32, std::sync::Arc<nf_formats::nis::Nis>> =
        level.nis.iter().map(|(&h, n)| (h, std::sync::Arc::new(n.clone()))).collect();
    let mut scripts = nf_game::nis::Scripts::new(lib, ps);
    println!("players: {}", scripts.players.len());
    for (i, p) in scripts.players.iter().enumerate() {
        println!("  player {i}: placement {} scripts {:?} modes {:?} mask {} state {}", p.placement, p.scripts, p.modes, p.mask, p.state);
    }
    let mut ch = nf_game::triggers::Channels { v: vec![0u8; 256], time: vec![0u32; 256], writes: None, script_channels: Default::default() };
    let mut w = ProbeWorld;
    let idx = scripts.players.iter().position(|p| p.scripts[0] == 0x0600_0058)
        .expect("skyrail: no script player for 0x06000058 (the gondola)");
    let mut first: Option<[f32; 3]> = None;
    let mut last: Option<[f32; 3]> = None;
    for frame in 0..600u32 {
        let skip = scripts.begin_tick(&mut ch, &w, frame);
        scripts.update_player(idx, &mut ch, &w, frame, skip, nf_game::nis::SpStart { health_ok: true, mission: 0, music2: 0, blasts: Vec::new() });
        let p = &scripts.players[idx];
        let pos = [p.live_matrix[3][0], p.live_matrix[3][1], p.live_matrix[3][2]];
        if frame == 0 || frame == 1 { first = Some(pos); }
        if frame % 100 == 0 { println!("  t{frame}: live_matrix pos {:.2} {:.2} {:.2} state {}", pos[0], pos[1], pos[2], p.state); }
        last = Some(pos);
    }
    let (f, l) = (first.unwrap(), last.unwrap());
    println!("first {f:?} last {l:?}");
    let p = &scripts.players[idx];
    println!("info[0] loaded: {}", p.info[0].is_some());
    if let Some(s) = &p.info[0] {
        println!("  clock {:.2} play {:#x}", s.clock, s.play);
        for (i, st) in s.stream_bindings().iter().enumerate() {
            println!("  stream {i}: {:?}", st);
        }
    }
    let ents: Vec<String> = scripts.objects.list.iter().flatten().filter_map(|o| match o {
        nf_game::nis::Obj::Entity { hash, matrix, src, .. } =>
            Some(format!("src {src:?} hash {hash:#x} pos {:.2} {:.2} {:.2}", matrix[3][0], matrix[3][1], matrix[3][2])),
        _ => None,
    }).collect();
    println!("entities: {}", ents.len());
    for e in &ents { println!("  {e}"); }
    assert!(f != l || last.unwrap()[2] != 0.0, "gondola did not move (identity matrix?)");
}
