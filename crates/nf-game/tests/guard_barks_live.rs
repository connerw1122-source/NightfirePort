//! Guard barks and the first-alert state (guard-barks-first-alert, audit gap ⑨):
//! - the spotting bark (`NDrone2_SeenPlayerTalk` 0x13f1d0, entries 0x49..0x4E — streamed ids 191/
//!   192/194/195/186/178) plays once through the real sim + audio path, inside the SpeechSFX
//!   cooldown rules (the 30-frame post-load gate, the per-entry repeat delay, the per-drone
//!   "line still referenced" busy gate);
//! - a guard's first alert routes through `DroneFunc_FirstAlertState` (0x147f58) → state 0x39
//!   (`NDrone2_DSTATE_InterogateAssist` 0x1616c0) with the game's timing (timer C at
//!   now + Rand(120)), then the assist wait (0x3a) and its end into Attack;
//! - the MP chatter family (`InjuredTalk` 0x13fa20 / `RunForCoverTalk` 0x13fd08 /
//!   `UnderCoverTalk` 0x13fe80 / `UnderCoverAimTalk` 0x13fb98) is dead in single player: the
//!   gate `MPSettings+0x184 == 0` [L ee_start_05.bin @0x2a4924; M disasm of all four] returns
//!   before any draw, so no chatter line can ever play and the RNG stream is untouched.
//! Opt-in: set `NF_ISO` to the disc image. Tests stay silent (NF_SILENT is set by the runner).

use nf_formats::anim::Script;
use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, drone, level::Level, levelbin::LevelBin, weapon::WeaponData};
use nf_game::drone::{Data, Drone, Drones, Player, Rng};
use std::collections::HashMap;

/// Everything `Data` borrows, owned (the ai_live harness, verbatim).
struct Level05 {
    tables: drone::DroneAnimData,
    props: drone::PropertyTables,
    weapons: WeaponData,
    level: Level,
    scripts: HashMap<u32, Script>,
    world: nf_game::world::CollisionWorld,
    sequences: HashMap<u32, nf_formats::anim::Sequence>,
    skeletons: HashMap<u8, nf_formats::character::Skeleton>,
    nav: nf_game::nav::Network,
    covers: Vec<nf_game::cover::CoverNode>,
    sfx: nf_formats::sfx::SfxTable,
    tuning: nf_formats::tuning::Tuning,
    hitboxes: nf_formats::hitbox::CollisionBones,
    volumes: Vec<nf_game::aivolume::AiVolume>,
    models: std::collections::HashMap<u32, nf_game::pickup::Model>,
    hud: nf_formats::hud::HudTables,
    surfaces: nf_formats::sfx::SurfaceSounds,
    voices: Vec<nf_formats::sfx::VoiceLine>,
}

impl Level05 {
    fn load() -> Option<Level05> {
        let path = std::env::var_os("NF_ISO")?;
        let iso = IsoImage::open(&path).unwrap();
        let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
        let (entries, bin) = disc::files_bin_entries(&iso).unwrap();
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000005.bin")).unwrap();
        let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
        let level = Level::parse(&data).unwrap();
        let lb = LevelBin::parse(&data).unwrap();
        let scripts = lb.entries.iter().filter(|e| e.kind == 5).filter_map(|e| Script::parse(lb.data(&data, e)).ok()).map(|s| (s.hashcode, s)).collect();
        let world = nf_game::world::CollisionWorld::build(&level, &level.instances(true).0);
        let skeletons = lb.entries.iter().filter(|e| e.kind == 6).filter_map(|e| nf_formats::character::parse_skeleton(lb.data(&data, e)).ok()).map(|k| (k.id as u8, k)).collect();
        let m = level.main_map().unwrap();
        let nav = nf_game::nav::Network::build(level.maps[m].ai_network.as_ref().unwrap(), &world, 0x0700_0005);
        let covers = nf_game::cover::build(&level.maps[m].placements, &nav, &world);
        let volumes = nf_game::aivolume::build(&level, 0x0700_0005);
        Some(Level05 {
            tables: drone::DroneAnimData::from_action_elf(&elf).unwrap(),
            props: drone::PropertyTables::from_action_elf(&elf).unwrap(),
            weapons: WeaponData::from_action_elf(&elf).unwrap(),
            sfx: nf_formats::sfx::SfxTable::from_action_elf(&elf).unwrap(),
            tuning: { let mut t = nf_formats::tuning::Tuning::from_action_elf(&elf).unwrap(); nf_game::tuning::read_tuning_vars(&mut t, 0x0700_0005); t },
            hitboxes: nf_formats::hitbox::CollisionBones::from_action_elf(&elf).unwrap(),
            sequences: disc::level_sequences(&iso, "07000005.bin").unwrap(),
            models: nf_game::pickup::models(&level),
            hud: nf_formats::hud::HudTables::from_action_elf(&elf).unwrap(),
            surfaces: nf_formats::sfx::SurfaceSounds::from_action_elf(&elf).unwrap(),
            voices: nf_formats::sfx::voice_lines(&elf).unwrap(),
            level, scripts, world, skeletons, nav, covers, volumes,
        })
    }

    fn sim(&self, drones: Vec<Drone>, rng: Rng) -> Drones {
        let mut s = Drones::new(drones, rng); s.attach_level(&self.level);
        s.difficulty = 1;
        s
    }

    fn data(&self) -> Data<'_> {
        Data { tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world, level: 0x0700_0005,
            sequences: &self.sequences, skins: &self.level.skins, skeletons: &self.skeletons, nav: &self.nav, covers: &self.covers, tuning: &self.tuning, hitboxes: &self.hitboxes, volumes: &self.volumes,
            sfx: Some(&self.sfx), models: &self.models, surfaces: Some(&self.surfaces), voices: Some(&self.voices), difficulty: 1 }
    }

    fn drones_seeded(&self, alert: Option<f32>, x: f32, seed: u32) -> (Vec<Drone>, usize, Rng) {
        let seed = std::env::var("NF_TEST_SEED").ok().and_then(|v| v.parse().ok()).unwrap_or(seed);
        let d = self.data();
        let mut rng = Rng::new(seed);
        let mut lights = nf_game::light::LightList::default();
        let mut drones = nf_game::drone::level_drones(&d, &self.level, &mut rng, &mut lights);
        if let Some(a) = alert { for dr in &mut drones { dr.alert = a; dr.alert_floor = a; } }
        let found = drones.iter().position(|dr| (dr.pos[0] - x).abs() < 0.01).unwrap();
        (drones, found, rng)
    }
}

fn bond_at(p: [f32; 3]) -> Player { Player { pos: p, target: [p[0], p[1] + 0.5, p[2]], eye: [p[0], p[1] + 1.0, p[2]], aim: [-1.0, 0.0, 0.0], sph: p, capsule: ([p[0], p[1] + 1.086, p[2]], p), hanging: false, cel: None } }

/// The seen-bark sfx ids: `Drone_SfxInfo` entries 0x49..=0x4e [L ee_start_05.bin @0x2812d0+idx·32].
fn seen_line_sfx(l: &Level05) -> Vec<u32> { (0x49..=0x4e).map(|i| l.voices[i as usize].sfx).collect() }

/// Spotting Bond plays the game's bark once, within the cooldown rules: with the guard at his
/// courtyard post and Bond stepping into view, exactly one SeenPlayerTalk line (0x49..0x4e) starts
/// through the real audio path — after the 30-frame post-load gate (`NDrone2_PlaySFX` 0x141848:
/// voices open at DronePostLoadFrame + 30 [L 108180 at the snapshot]) — and never repeats inside
/// the window (the per-entry repeat delays of 0x49..0x4e are 0 [L], so the once-ness comes from
/// the state machine: the bark fires on the first-sight and CombatNewSighting enters only, and
/// this guard does not lose Bond in the window).
#[test]
fn spot_bark_plays_once_within_cooldown() {
    use nf_game::sound::Cmd;
    let Some(l) = Level05::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let (drones, gi, rng) = l.drones_seeded(None, 61.342, 2);
    let mut sim = l.sim(drones, rng);
    let away = bond_at([0.0, -500.0, 0.0]);
    for _ in 0..10 { sim.tick(&d, &away); }
    let player = bond_at([67.31, 26.5, 99.47]);
    let seen_ids = seen_line_sfx(&l);
    let mut seen_heard: Vec<(u32, u32)> = Vec::new();
    for t in 0..1500 {
        sim.tick(&d, &player);
        sim.audio.update(Some(&l.sfx), (player.eye, [0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]), &|_| false);
        for c in sim.audio.take_commands() { if let Cmd::Start { id, .. } = c { if seen_ids.contains(&id) { seen_heard.push((t, id)); } } }
    }
    eprintln!("seen barks: {seen_heard:?} (ids {seen_ids:?})");
    // The bark fires once on the first spotting — this guard does not lose Bond again inside the
    // first window (the trace: noticed at tick 61, engaged from then on); the later lines in a
    // 1500-tick fight are the game's CombatNewSighting re-enters (0x5b, each re-gated by the 1-in-3
    // roll — 1032/1269 below), which need a full lose-and-regain cycle in between.
    let first_window: Vec<(u32, u32)> = seen_heard.iter().copied().filter(|(t, _)| *t < 1000).collect();
    assert_eq!(first_window.len(), 1, "the spotting bark plays exactly once on the first sighting: {seen_heard:?}");
    assert!(first_window[0].0 >= 30, "inside the 30-frame post-load gate");
    assert!(seen_ids.contains(&first_window[0].1), "a SeenPlayerTalk line");
    // Every re-bark sits at least a lose/regain cycle apart (the game's re-sight rule).
    for w in seen_heard.windows(2) { assert!(w[1].0 > w[0].0 + 16, "no re-bark inside the 16-tick loss gate: {seen_heard:?}"); }
}

/// A guard's first alert goes through state 0x39 with the game's timing. The precondition is the
/// game's ConsiderAlerted copy: `DroneVision_ConsiderAlerted` (0x176250:226) shifts the
/// interrogating source's `+0x4FC` 0x20 into the receiver's 0x200000 and msg 0x1f
/// (`DroneFunc_DoForcedAttack` 0x148c60) leaves the forced-attack pending bit (`+0x4F8` 0x20000000);
/// `DroneFunc_FirstAttack` (0x148010:24-38) — not the drone's own sight — consults
/// `DroneFunc_FirstAlertState` (0x147f58), which returns 0x39 on {05, 07, 41}. ENTER arms timer C
/// at now + Rand(120) [M 0x161704-24] (exactly one RNG draw) and sets the interrogate marker
/// (`+0x4FC |= 0x20` [M 0x178308]); TIMER_C walks him to 10 units from Bond (GoToGoal, return
/// state 0x3a) or waits; the wait ends into Attack when the interrogator stops
/// (`TestEndInterogationAssist` 0x178348), with `EndInterogateAssist`'s `+0x4FC = (& 0xffdfffdf) |
/// 0x10` on the exit [M 0x178318].
#[test]
fn first_alert_routes_through_state_0x39() {
    let Some(l) = Level05::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let (drones, gi, rng) = l.drones_seeded(None, 61.342, 2);
    let mut sim = l.sim(drones, rng);
    let away = bond_at([0.0, -500.0, 0.0]);
    for _ in 0..10 { sim.tick(&d, &away); }
    let player = bond_at([67.31, 26.5, 99.47]);
    for _ in 0..4 { sim.tick(&d, &player); }
    let frame = sim.frame;
    // The receiver: gate guard, never attacked (the first attack is pending), not sighting Bond
    // himself — he is around the corner from (67.31, 26.5, 99.47).
    {
        let r = &mut sim.list[gi];
        r.alert_bits = 0x20_0000;           // ConsiderAlerted: the interrogating source's 0x20 << 16
        r.alerted_by_drone = true;          // the +0x4F8 0x20000 latch
        r.own_sight = false;
        r.first_attack_done = false;
    // The 0x39 ENTER refuses a guard without an opponent (`+0x170 == 0` → NoOpponent 0x60 [M
    // 0x1616c4-cf]); the assisting guard in the game acquired Bond as his opponent earlier
    // (`NDrone2_FindOpponent` 0x1424e0 runs from ControlSTANDARD every frame) — the port's proxy
    // for a held opponent is `ever_seen` [S]. ConsiderAlerted also stamped `+0x528` = the source
    // (the game's forced attack always arrives through it) — modelled with a neighbouring guard
    // holding the interrogate marker 0x20.
    r.ever_seen = true;
    r.alert_source = Some(if gi == 0 { 1 } else { 0 }); // ConsiderAlerted 0x176250:226's +0x528 stamp
    }
    let src_i = if gi == 0 { 1 } else { 0 };
    sim.list[src_i].alert_bits = 0x20;      // the interrogator's +0x4FC marker
    let draws_before = sim.rng.draws;
    sim.list[gi].send(&d, &mut sim.rng, frame, nf_game::drone::msg::FORCED_ATTACK);
    assert_eq!(sim.list[gi].state, nf_game::drone::state::INTEROGATE_ASSIST, "FirstAlertState routed the first alert to 0x39");
    assert_eq!(sim.rng.draws - draws_before, 1, "the route draws exactly once: the ENTER's Rand(120)");
    let timer = sim.list[gi].timer_c.expect("timer C armed");
    assert!(timer > frame && timer <= frame + 120, "timer C inside Rand(120) ticks: {timer} vs frame {frame}");
    assert!(sim.list[gi].alert_bits & 0x20 != 0, "SetInterogateAssist: +0x4FC |= 0x20");
    // Tick through the timer with Bond far away: the assist walks toward 10 units from his
    // object (GoToGoal, return state 0x3a) — the game's GoToGoal UPDATE re-attacks when the
    // walker SEES Bond, so the far spot also keeps him unseen — and settles in the wait state
    // (an unreachable goal takes the direct 0x3a branch, [M 0x1616c0 TIMER_C else-arm]).
    let mut saw_walk = false;
    let mut saw_wait = false;
    for _ in 0..600 {
        sim.tick(&d, &away);
        let s = sim.list[gi].state;
        if s == nf_game::drone::state::GO_TO_GOAL {
            saw_walk = true;
            assert_eq!(sim.list[gi].return_state, nf_game::drone::state::INTEROGATE_ASSIST_WAIT, "+0x5A2 = 0x3a on the reachable goal");
        }
        if s == nf_game::drone::state::INTEROGATE_ASSIST_WAIT { saw_wait = true; break; }
        assert!(matches!(s, nf_game::drone::state::INTEROGATE_ASSIST | nf_game::drone::state::GO_TO_GOAL), "still assisting, in {s:#x}");
    }
    assert!(saw_wait, "the assist settles in the wait state 0x3a");
    assert!(sim.list[gi].alert_bits & 0x20 != 0, "the wait state re-sets the interrogate marker");
    let _ = saw_walk;
    // The interrogator stops (his +0x4FC leaves the {0x20, 0x200000} state) → the next UPDATE ends
    // the assist into Attack, and the 0x3a exit runs EndInterogateAssist.
    sim.list[src_i].alert_bits = 0x10;
    sim.tick(&d, &away);
    // The 0x3a UPDATE ends the assist into Attack (0x56), whose ENTER re-runs FirstAttack — the
    // receiver's own 0x10 (EndInterogateAssist's set) suppresses the FirstAlertState route and the
    // normal first attack takes over (combat). Both land out of the assist family.
    let s = sim.list[gi].state;
    assert!(!matches!(s, nf_game::drone::state::INTEROGATE_ASSIST | nf_game::drone::state::INTEROGATE_ASSIST_WAIT),
        "TestEndInterogationAssist ends the assist, got {s:#x}");
    let b = sim.list[gi].alert_bits;
    assert!(b & 0x20 == 0 && b & 0x20_0000 == 0 && b & 0x10 != 0, "EndInterogateAssist: (& 0xffdfffdf) | 0x10, got {b:#x}");
    // The within-10 branch: a fresh receiver; Bond steps to 8 units a tick before the armed
    // timer fires, so TIMER_C sees dist ≤ 10 and lands straight in the wait state (the 10-unit
    // arrival radius, [M 0x15add0 + 0x1517d8:80-83]). Two ticks at 8 units cannot clear the
    // ~61-tick sighting floor, so the assist is never broken by sight.
    let (drones2, gi2, rng2) = l.drones_seeded(None, 61.342, 2);
    let mut sim2 = l.sim(drones2, rng2);
    for _ in 0..10 { sim2.tick(&d, &away); }
    {
        let r2 = &mut sim2.list[gi2];
        r2.alert_bits = 0x20_0000;
        r2.alerted_by_drone = true;
        r2.own_sight = false;
        r2.first_attack_done = false;
        r2.ever_seen = true;
        let src2 = if gi2 == 0 { 1 } else { 0 };
        r2.alert_source = Some(src2);
    }
    let src2 = if gi2 == 0 { 1 } else { 0 };
    sim2.list[src2].alert_bits = 0x20;
    sim2.list[gi2].send(&d, &mut sim2.rng, sim2.frame, nf_game::drone::msg::FORCED_ATTACK);
    assert_eq!(sim2.list[gi2].state, nf_game::drone::state::INTEROGATE_ASSIST, "the route again");
    let timer2 = sim2.list[gi2].timer_c.expect("timer armed");
    while sim2.frame + 2 < timer2 { sim2.tick(&d, &away); }
    let near = bond_at([sim2.list[gi2].pos[0] + 8.0, sim2.list[gi2].pos[1], sim2.list[gi2].pos[2]]);
    for _ in 0..3 { sim2.tick(&d, &near); }
    assert_eq!(sim2.list[gi2].state, nf_game::drone::state::INTEROGATE_ASSIST_WAIT, "within 10 units the TIMER_C lands in the wait state");
}

/// The first-alert route's suppress list: a source that ATTACKED (`+0x4FC` 0x10 → the receiver's
/// 0x100000) disqualifies the assist (`DroneFunc_FirstAlertState` 0x147f58 returns 0) — the
/// forced attack falls through to the normal first attack, which talks (`NDrone2_AttackTalk`) and
/// goes to combat. And on a level outside {05, 07, 41} the 0x200000 flag routes nowhere.
#[test]
fn first_alert_suppress_bits_fall_through_to_attack() {
    let Some(l) = Level05::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let (drones, gi, rng) = l.drones_seeded(None, 61.342, 2);
    let mut sim = l.sim(drones, rng);
    let away = bond_at([0.0, -500.0, 0.0]);
    for _ in 0..10 { sim.tick(&d, &away); }
    let player = bond_at([67.31, 26.5, 99.47]);
    for _ in 0..4 { sim.tick(&d, &player); }
    let frame = sim.frame;
    let r = &mut sim.list[gi];
    r.alert_bits = 0x10_0000;           // the source attacked (0x10 << 16): the suppress list hits
    r.alerted_by_drone = true;
    r.own_sight = false;
    r.first_attack_done = false;
    sim.list[gi].send(&d, &mut sim.rng, frame, nf_game::drone::msg::FORCED_ATTACK);
    let s = sim.list[gi].state;
    assert!(s != nf_game::drone::state::INTEROGATE_ASSIST && sim.list[gi].first_attack_done,
        "a disqualified alert falls through to the normal first attack (combat route), got {s:#x}");
}

/// The MP chatter family is dead in single player: the four call sites (RunForCover enter with a
/// claimed cover, UnderCoverInit / UnderCoverAim / UnderCoverReturn enters, the BulletImpact
/// survival path) all sit behind `MPSettings+0x184 == 0 → return` [M disasm], so running a guard
/// through them consumes no RNG (the game draws nothing either) and queues no line; the chatter
/// entries (0xb9..0xd3, 0x11b..0x128, 0x133..0x13c) can never start.
#[test]
fn mp_chatter_is_silent_in_single_player() {
    let Some(l) = Level05::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let (drones, gi, rng) = l.drones_seeded(None, 61.342, 2);
    let mut sim = l.sim(drones, rng);
    let away = bond_at([0.0, -500.0, 0.0]);
    for _ in 0..10 { sim.tick(&d, &away); }
    let chatter: Vec<usize> = (0xb9..=0xd3).chain(0x11b..=0x128).chain(0x133..=0x13c).collect();
    // RunForCover enter with a claimed cover node (the +0x6F4 gate), then the three under-cover
    // enters, then a survived bullet (the InjuredTalk path) — every site the family hangs on.
    sim.list[gi].goal = Some(sim.list[gi].pos);
    sim.list[gi].send(&d, &mut sim.rng, sim.frame, nf_game::drone::msg::ENTER);
    let draws_after_cover = sim.rng.draws;
    for s in [nf_game::drone::state::UNDER_COVER_INIT, nf_game::drone::state::UNDER_COVER_AIM, nf_game::drone::state::UNDER_COVER_RETURN] {
        sim.list[gi].state = s;
        sim.list[gi].send(&d, &mut sim.rng, sim.frame, nf_game::drone::msg::ENTER);
        assert!(sim.list[gi].talks.is_empty(), "no chatter line queued in state {s:#x}");
    }
    assert_eq!(sim.rng.draws - draws_after_cover, 0, "the four chatter sites draw nothing (the game's gate returns first)");
    // A survived bullet through the real path (the pistol, as in guards_talk_in_a_fight): the
    // InjuredTalk site runs and refuses at the MP gate — no chatter entry can appear (the pain
    // line 0x13 and the flinch rolls are the game's own SP behaviour).
    let g = sim.list[gi].pos;
    let player = bond_at([g[0] + 3.0, g[1], g[2]]);
    let e = [g[0] + 3.0, g[1] + 1.2, g[2]];
    let to = [g[0], g[1] + 0.3, g[2]];
    let n = ((to[0] - e[0]).powi(2) + (to[1] - e[1]).powi(2) + (to[2] - e[2]).powi(2)).sqrt();
    sim.fire_player(&d, e, [(to[0] - e[0]) / n, (to[1] - e[1]) / n, (to[2] - e[2]) / n], 2, l.sfx.get(0x403));
    sim.tick(&d, &player);
    assert!(sim.list[gi].health > 0.0, "the guard survived the shot");
    assert!(sim.list[gi].talks.iter().all(|(idx, _)| !chatter.contains(idx)), "no chatter entry queued: {:?}", sim.list[gi].talks);
}
