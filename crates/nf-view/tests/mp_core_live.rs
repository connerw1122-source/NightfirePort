//! mp-core: the multiplayer match core against the real arenas (docs/atlas/multiplayer.md §6
//! phase 1). The spawn tables pin all 8 arenas' type-0x25 placements through the real registration
//! (`MP_RegisterSpawnPoint` 0x184f20 — the floor snap, the +1.6 eye, the 64-entry table), the spawn
//! choice pins `MP_GetSpawnPoint` 0x185098's arithmetic per `MPSettings+0x448` policy, the clock
//! pins `MPGame+0x400` through the real `Sim::step_bond` path, and the MP-object registration pins
//! `MP_RegisterMPObject` 0x185990's scenario gates. Opt-in via NF_ISO.

use nf_formats::iso9660::IsoImage;
use nf_formats::level::Level;
use nf_game::drone::Rng;
use nf_game::mp::{scenario, MpMatch, MpSettings, MpSlot, SpawnPoint};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// (level code, MP spawn placements) — the pinned §1 table's spawn column [D mp_arenas_live.rs].
const ARENA_SPAWNS: [(u32, usize); 8] = [
    (0x07000023, 32), (0x07000024, 32), (0x07000025, 32), (0x07000026, 30),
    (0x07000027, 31), (0x07000028, 31), (0x07000029, 32), (0x0700004b, 32),
];

/// Reads an arena's level file straight from the disc [D] — the same path the viewer's load uses.
fn read_level(path: &str, code: u32) -> Level {
    let i = IsoImage::open(path).unwrap();
    let (entries, bin) = nf_formats::disc::files_bin_entries(&i).unwrap();
    let name = format!("{code:08x}.bin");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(&name))
        .unwrap_or_else(|| panic!("arena {code:08x}.bin missing"));
    let data = i.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    Level::parse(&data).unwrap()
}

/// The 0x25/0x26 placements of the main map, in placement order.
fn mp_placements(level: &Level) -> (Vec<nf_formats::placement::Placement>, Vec<nf_formats::placement::Placement>) {
    let m = level.main_map().unwrap();
    let map = &level.maps[m];
    let spawns = map.placements.iter().filter(|p| p.flags & 0xff == 0x25).cloned().collect();
    let objs = map.placements.iter().filter(|p| p.flags & 0xff == 0x26).cloned().collect();
    (spawns, objs)
}

fn param(p: &nf_formats::placement::Placement, k: u32) -> u32 {
    p.params.iter().find(|(i, _)| *i == k).map_or(0, |&(_, v)| v)
}

/// All 8 arenas' 0x25 placements register into the 64-entry table in placement order, at the game's
/// floor-snapped +1.6 eye positions, with the team counts summing to `SpawnPntCount` [M 0x184f20;
/// D the pinned §1 spawn column].
#[test]
fn spawn_tables_match_the_placements_for_all_8_arenas() {
    let Some(path) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    for (code, expect) in ARENA_SPAWNS {
        let level = read_level(&path, code);
        let (places, _) = mp_placements(&level);
        assert_eq!(places.len(), expect, "arena {code:08x}: the 0x25 placement count");
        let world = nf_game::world::CollisionWorld::from_triangles(Vec::new());
        let mut m = MpMatch::init(MpSettings::deathmatch(code));
        // The game rejects team 2 outright [M 0x184f20:51-53] — the registered count is the
        // placement count minus those rejects (measured: some arenas' placements carry team 2).
        let accepted: Vec<&nf_formats::placement::Placement> =
            places.iter().filter(|p| param(p, 0) as u16 != 2).collect();
        for p in &places {
            let team = param(p, 0) as u16;
            m.register_spawn_point(p.translation, p.rotation, team, &world);
        }
        assert_eq!(m.spawns.count as usize, accepted.len(),
            "arena {code:08x}: SpawnPntCount = the placements minus the team-2 rejects");
        assert!(m.spawns.points.iter().take(m.spawns.count as usize).all(|p| p.used == 1),
            "arena {code:08x}: every registered point used");
        assert_eq!(m.spawns.team_count[0] as usize + m.spawns.team_count[1] as usize, accepted.len(),
            "arena {code:08x}: SpawnPntTeamCount sums to SpawnPntCount");
        // The k-th registered point is the k-th accepted placement (FFA range [0,0x40): the
        // first-free-slot scan fills the table in placement order), x/z exact, y = the snap + 1.6.
        for (i, p) in accepted.iter().enumerate() {
            let sp = &m.spawns.points[i];
            assert_eq!(sp.pos[0], p.translation[0], "arena {code:08x} point {i}: x");
            assert_eq!(sp.pos[2], p.translation[2], "arena {code:08x} point {i}: z");
            let expect_y = match world.segment([p.translation[0], p.translation[1], p.translation[2]],
                                              [p.translation[0], p.translation[1] - 3.0, p.translation[2]]) {
                Some(h) => p.translation[1] - 3.0 * h + 1.6, // the build_PointOnFloor snap [M 0x1c95a8]
                None => p.translation[1] + 1.6,              // no floor within 3.0: the pos stays [M 0x1c95a8:49-58]
            };
            assert_eq!(sp.pos[1], expect_y, "arena {code:08x} point {i}: the snapped eye y");
            assert_eq!((sp.facing[0], sp.facing[1]), (p.rotation[0], p.rotation[1]), "arena {code:08x} point {i}: facing");
        }
        // Sister: the spawn counts differ across arenas (30/31/32), so a constant pin fails.
        let (_, e0) = ARENA_SPAWNS[0];
        assert!(ARENA_SPAWNS.iter().any(|(_, e)| *e != e0), "sister: the pin is not constant");
    }
}

/// The spawn choice reproduces `MP_GetSpawnPoint` 0x185098's arithmetic exactly: the candidate gate
/// (nearest-enemy sq-dist > 2.0), the running argmin/argmax trackers, the three policies
/// (`MPSettings+0x448`: 0 → the argmin, 1 → the argmax, 2 → `cands[Rand_Rand(n)]`), the −1
/// fallbacks, and the team-game table split. Pure unit — no ISO needed.
#[test]
fn spawn_choice_matches_the_decompile_arithmetic() {
    let pos = |x: f32| [x, 1.6, 0.0, 0.0];
    let mut m = MpMatch::init(MpSettings::deathmatch(0x07000023));
    // Six used points on a line (x = 0..5), two unused slots interleaved. (The floor snap is
    // re-derived against the same empty world here — the real-world snap is pinned by the
    // end-to-end test below.)
    for (i, x) in [0.0f32, 1.0, 2.0, 3.0, 4.0, 5.0].iter().enumerate() {
        m.spawns.points[i * 2] = SpawnPoint { pos: pos(*x), facing: [0.0; 4], used: 1 };
    }
    // One enemy at x = 0 (slot 1): the candidates are the points whose nearest-enemy sq-dist > 2.0
    // [M 0x185098:106,137] — dist²(x) = x², so x = 0 and x = 1 (0.0, 1.0) fail the gate; the
    // candidates are x = 2..5 (4.0..25.0), four of them.
    m.game.slots[1] = MpSlot { pos: Some([0.0, 1.6, 0.0]), ..Default::default() };
    // Policy 1 → the argmax of the nearest-enemy distance = the farthest point (x = 5, index 10).
    m.settings.spawn_policy = 1;
    assert_eq!(m.get_spawn_point(0, 0, &mut Rng::new(1)), 10, "policy 1: the farthest candidate");
    // Policy 0 → the argmin among the candidates = x = 2 (index 4, dist² 4.0) — the decompile's
    // running-min tracker, NOT the atlas's "2nd-farthest" gloss.
    m.settings.spawn_policy = 0;
    assert_eq!(m.get_spawn_point(0, 0, &mut Rng::new(1)), 4, "policy 0: the least-far candidate");
    // Policy 2 → cands[Rand_Rand(n_cands)] with n_cands = 5; Rand_Rand's LCG [M 0x1e3738] steps y
    // (×30903) then x (×18000) and returns ((y & 0xffff) × n) >> 16 — recomputed here independently.
    m.settings.spawn_policy = 2;
    let (mut y, mut x) = (0x208a_5de8u32 ^ 1u32, 0x0659_5adbu32 ^ 1u32.wrapping_mul(0x9e37_79b9)); // Rng::new(1)
    y = 30903u32.wrapping_mul(y & 0xffff).wrapping_add(y >> 16);
    x = 18000u32.wrapping_mul(x & 0xffff).wrapping_add(x >> 16);
    let r = ((y & 0xffff) * 4u32) >> 16;
    let expect = [4u16, 6, 8, 10][r as usize]; // the candidates in scan order (dist² > 2.0: x = 2..5)
    assert_eq!(m.get_spawn_point(0, 0, &mut Rng::new(1)) as usize, expect as usize,
        "policy 2: cands[Rand_Rand(5)] with the game's LCG (r = {r})");
    // The me-exclusion: the enemy's own slot picking sees no other slot → every used point is a
    // 9999-candidate, the argmin stays unset (9999 ≤ 9999 keeps the −1) → policy 0 falls back to
    // the range start 0 [M 0x185098:165-176].
    m.game.slots[1] = MpSlot::default();
    m.settings.spawn_policy = 0;
    assert_eq!(m.get_spawn_point(0, 1, &mut Rng::new(1)), 0, "no other slots: policy 0 falls back to lo");
    // Policy 1 with the same empty field: the argmax takes the FIRST candidate (9999 > −9999), the
    // first used point (index 0).
    m.settings.spawn_policy = 1;
    assert_eq!(m.get_spawn_point(0, 1, &mut Rng::new(1)), 0, "no other slots: policy 1 takes the first used");
    // Sister: an unknown policy value returns the range start [M 0x185098:181-183].
    m.settings.spawn_policy = 9;
    assert_eq!(m.get_spawn_point(0, 0, &mut Rng::new(1)), 0, "unknown policy: the range start");
}

/// The team-game split: `MP_Init` derives `+0x396 = (scenario >> 29) & 1` [M 0x183438:41]; the
/// table splits [0,0x20)/[0x20,0x40) and team 2 is rejected at registration [M 0x184f20:51-53,55-65].
#[test]
fn team_games_split_the_spawn_table() {
    let mut s = MpSettings::deathmatch(0x07000023);
    s.scenario = scenario::TEAM_DEATHMATCH; // bit29 set
    let mut m = MpMatch::init(s);
    assert_eq!(m.settings.team_game, 1, "the team-game derivation");
    let mut ffa = MpMatch::init(MpSettings::deathmatch(0x07000023));
    // Team 2 is rejected outright; the counts stay 0.
    ffa.register_spawn_point([0.0, 1.6, 0.0], [0.0; 3], 2, &nf_game::world::CollisionWorld::from_triangles(Vec::new()));
    assert_eq!(ffa.spawns.count, 0, "team 2 rejected [M 0x184f20:51-53]");
    // In a team game team 1 registers into [0x20,0x40) — slot 0x20, not 0.
    m.register_spawn_point([0.0, 1.6, 0.0], [0.0; 3], 1, &nf_game::world::CollisionWorld::from_triangles(Vec::new()));
    assert!(m.spawns.points[0].used == 0 && m.spawns.points[0x20].used == 1, "team 1 lands at 0x20");
    assert_eq!(m.spawns.team_count[1], 1);
    // Sister: the same registration under FFA lands at slot 0.
    ffa.register_spawn_point([0.0, 1.6, 0.0], [0.0; 3], 1, &nf_game::world::CollisionWorld::from_triangles(Vec::new()));
    assert!(ffa.spawns.points[0].used == 1, "FFA: team 1 lands at 0");
}

/// `MP_RegisterMPObject` 0x185990 gates every type on the scenario word: a Deathmatch stores
/// nothing from 07000023's 27 type-0x26 placements, the CTF scenario stores exactly the param-0
/// ∈ {0,1} subset, and the counts re-derive from the placements themselves (the sister).
#[test]
fn mp_object_registration_gates_on_the_scenario() {
    let Some(path) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let level = read_level(&path, 0x07000023);
    let (_, objs) = mp_placements(&level);
    assert_eq!(objs.len(), 27, "the pinned 0x26 count [D §1]");
    let ctf_n = objs.iter().filter(|p| matches!(param(p, 0), 0 | 1)).count();
    // Deathmatch: every type's scenario gate fails → nothing stored.
    let mut dm = MpMatch::init(MpSettings::deathmatch(0x07000023));
    for p in &objs {
        let ty = param(p, 0);
        let cfg: [u32; 7] = std::array::from_fn(|k| param(p, k as u32));
        dm.register_mp_object(p.translation, p.rotation, ty, cfg, p.hashcode);
    }
    assert_eq!(dm.objects.created.len() + dm.objects.demolition.len() + dm.objects.blueprint_items.len()
        + dm.objects.goldeneye[0].len() + dm.objects.goldeneye[1].len() + dm.objects.protection.len(), 0,
        "deathmatch stores no MP objects (the type gates [M 0x185990:47-149])");
    // CTF: exactly the type-0/1 subset.
    let mut s = MpSettings::deathmatch(0x07000023);
    s.scenario = scenario::CAPTURE_THE_FLAG;
    let mut ctf = MpMatch::init(s);
    for p in &objs {
        let ty = param(p, 0);
        let cfg: [u32; 7] = std::array::from_fn(|k| param(p, k as u32));
        ctf.register_mp_object(p.translation, p.rotation, ty, cfg, p.hashcode);
    }
    assert_eq!(ctf.objects.created.len(), ctf_n, "CTF stores exactly the flag/base records");
    // Sister: the stored records keep the placement's translation in the matrix's translation row.
    if let Some(rec) = ctf.objects.created.first() {
        assert_eq!(rec.matrix[3][0], objs.iter().find(|p| matches!(param(p, 0), 0 | 1)).unwrap().translation[0]);
    } else {
        assert_eq!(ctf_n, 0, "no CTF records and none expected");
    }
}

/// The deliverable end-to-end: `nf_view::load_mp("07000023.bin")` boots the match — Bond stands at
/// the policy's spawn point, and the match clock (`MPGame+0x400`) advances through the real
/// `Sim::step_bond` path, latching the time-up when the limit passes.
#[test]
fn the_mp_load_spawns_bond_and_runs_the_clock() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let mut scene = nf_view::load_mp(&iso, "07000023.bin", false).expect("the arena loads");
    let sim = scene.sim.as_mut().unwrap();
    let m = sim.mp.as_mut().expect("--mp builds the match");
    assert!(m.settings.started, "MP_Start ran [M 0x183a50:207]");
    assert_eq!(m.spawns.count, 32, "arena 0x07000023's 32 spawn points [D §1]");
    let sp = m.spawns.points.iter().position(|p| p.used == 1 && (p.pos[0] - sim.bond.pos[0]).abs() < 0.01
        && (p.pos[2] - sim.bond.pos[2]).abs() < 0.01).expect("Bond stands at a registered spawn point");
    assert!(sim.bond.pos[1] > m.spawns.points[sp].pos[1] - 1.7 && sim.bond.pos[1] < m.spawns.points[sp].pos[1] + 1.0,
        "Bond's body sits by the spawn's eye height (the point y = floor + 1.6)");
    // The real-world snap: the registered point's y = the floor below the placement + 1.6
    // [M 0x1c95a8 + 0x184f20:83] — recomputed from the sim's collision world.
    {
        let level = read_level(&iso, 0x07000023);
        let (places, _) = mp_placements(&level);
        let p = &places[sp];
        let expect_y = match sim.world.segment([p.translation[0], p.translation[1], p.translation[2]],
                                               [p.translation[0], p.translation[1] - 3.0, p.translation[2]]) {
            Some(h) => p.translation[1] - 3.0 * h + 1.6,
            None => p.translation[1] + 1.6,
        };
        assert_eq!(m.spawns.points[sp].pos[1], expect_y, "the real floor snap + 1.6 eye");
    }
    // The clock: 60 ticks ≈ 1 s of the game's accumulation (the fixed tick [M 0x184268:55,140 with
    // the port's dt — the module doc's deviation]).
    for _ in 0..60 { sim.step_bond(Default::default(), false, false, false); }
    let c60 = sim.mp.as_ref().unwrap().game.clock;
    assert!((c60 - 1.0).abs() < 0.01, "60 ticks = 1 s on the round clock, got {c60}");
    let m = sim.mp.as_ref().unwrap();
    assert!((m.game.elapsed - m.game.clock).abs() < 1e-6, "+0x412 accumulates the same delta [M 0x184268:44-46]");
    // Sister: the time-up latch fires once the limit (10.0) passes — shorten the limit and step past it.
    let m = sim.mp.as_mut().unwrap();
    m.game.limit = 1.0;
    for _ in 0..70 { sim.step_bond(Default::default(), false, false, false); }
    let m = sim.mp.as_ref().unwrap();
    assert!(m.game.time_up, "the limit latched switch_channels[0xfe] [M 0x184268:142-145]");
    // Sister 2: the clock is bit-deterministic — a fresh match accumulates the same f32 value.
    let mut m2 = MpMatch::init(MpSettings::deathmatch(0x07000023));
    m2.settings.started = true;
    for _ in 0..60 { m2.update(1.0 / 60.0, false, &mut nf_game::drone::Rng::new(1)); }
    assert_eq!(m2.game.clock, c60, "the clock is bit-deterministic: the same 60-tick fold");
}
