//! mp-bots-states: the ported bot state machine against a real MP arena (docs/atlas/multiplayer.md
//! §4, phase 4 after 401). One bot on Arena 1 (0x07000023) through the real load path — the disc's
//! 0x25 spawn placements drive `MpMatch::register_spawn_point` 0x184f20 and the policy pick
//! (`MP_GetSpawnPoint` 0x185098), the disc's 0xF0 placements are the pickup table (the
//! `MP_PICKUP` rows), and the brain runs the ported goal machine (0xF9/0xEB, the visit floats
//! MP_PICKUP+0x80) plus the fire decision (0xCF/0xD2). Sister: the "stand" preset holds position.
//! Opt-in via NF_ISO.

use nf_formats::iso9660::IsoImage;
use nf_formats::level::Level;
use nf_game::drone::Rng;
use nf_game::mp::bots::{BotBrain, BotCtx, BotPickup, BotRecord, OtherPlayer};
use nf_game::mp::{MpMatch, MpSettings};
use nf_game::world::CollisionWorld;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Reads Arena 1's level file straight from the disc [D] — the viewer's load path.
fn read_arena(path: &str, code: u32) -> Level {
    let i = IsoImage::open(path).unwrap();
    let (entries, bin) = nf_formats::disc::files_bin_entries(&i).unwrap();
    let name = format!("{code:08x}.bin");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(&name))
        .unwrap_or_else(|| panic!("arena {code:08x}.bin missing"));
    let data = i.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    Level::parse(&data).unwrap()
}

/// The 0x25 / 0xF0 placements of the main map, in placement order [D].
fn arena_placements(level: &Level) -> (Vec<nf_formats::placement::Placement>, Vec<nf_formats::placement::Placement>) {
    let m = level.main_map().unwrap();
    let map = &level.maps[m];
    let spawns = map.placements.iter().filter(|p| p.flags & 0xff == 0x25).cloned().collect();
    let pickups = map.placements.iter().filter(|p| p.flags & 0xff == 0xf0).cloned().collect();
    (spawns, pickups)
}

/// The match + bot rig on the real arena: the spawns register through the game's path, the bot
/// spawns at the policy's point (policy 0, the port's pinned arithmetic), the pickups come from
/// the disc.
struct ArenaRig {
    brain: BotBrain,
    pickups: Vec<BotPickup>,
    others: Vec<OtherPlayer>,
    world: CollisionWorld,
    rng: Rng,
    now: f32,
    stationary: bool,
    vision: bool,
}

impl ArenaRig {
    fn new(path: &str) -> ArenaRig {
        let level = read_arena(path, 0x07000023);
        let (spawn_places, pickup_places) = arena_placements(&level);
        let world = CollisionWorld::from_triangles(Vec::new());
        let mut m = MpMatch::init(MpSettings::deathmatch(0x07000023));
        for p in &spawn_places {
            let team = p.params.iter().find(|(i, _)| *i == 0).map_or(0u16, |&(_, v)| v as u16);
            m.register_spawn_point(p.translation, p.rotation, team, &world);
        }
        assert!(m.spawns.count >= 30, "Arena 1's spawn table, got {}", m.spawns.count);
        m.start(&mut Rng::new(1));
        // the bot's spawn through the real policy path (policy 0, me = slot 4) [M 0x185098]
        let mut policy_rng = Rng::new(0xB0C);
        let idx = m.get_spawn_point(0, 4, &mut policy_rng) as usize;
        let spawn = m.spawns.points[idx].pos;
        // the boot record, enabled (the menu's +0x0E byte) — preset 0 stats [L ee_snapshot]
        let mut rec = BotRecord([0x01, 0x00, 0x04, 0x00, 0x96, 0x00, 0x02, 0x96, 0x32, 0x00, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x06, 0x00]);
        rec.0[0x0E] = 1;
        let mut brain = BotBrain::bot_init(&rec, 4, [spawn[0], spawn[1] - 1.6, spawn[2]], 0.0);
        brain.weapon = nf_game::mp::bots::BotWeapon { interval: 6, burst: 3, aggression_mul: 1.0 };
        brain.vars.weapon_id = 0x02;
        brain.vars.clip_rounds = 50;
        // the pickup table: the arena's real 0xF0 placements [D]; the MP weapon ids are runtime
        // random (`Pickup_MakeRandomWeaponSet`) — carried as class-0 rows with id 0 [S]
        let pickups: Vec<BotPickup> = pickup_places.iter().map(|p| BotPickup {
            pos: p.translation,
            weapon_id: 0,
            mp_class: 0,
            is_obj_flag2: false,
            visit: [0.0; 4],
        }).collect();
        // the stationary "player" at another registered spawn (slot 0)
        let p0 = m.spawns.points[(idx + 7) % m.spawns.count as usize].pos;
        ArenaRig {
            brain,
            pickups,
            others: vec![
                OtherPlayer { index: 0, pos: [p0[0], p0[1] - 1.6, p0[2]], alive: true, team: 0, flags: 0, dist2: 0.0, health: 100.0, score: 0.0 },
                OtherPlayer { index: 4, pos: [spawn[0], spawn[1] - 1.6, spawn[2]], alive: true, team: 0, flags: 0, dist2: 0.0, health: 100.0, score: 0.0 },
            ],
            world,
            rng: Rng::new(0x77E),
            now: 0.0,
            stationary: false,
            vision: false,
        }
    }

    fn step(&mut self) {
        let mut others = self.others.clone();
        for o in others.iter_mut() {
            if o.index == 4 || o.index == 0 {
                o.dist2 = (o.pos[0] - self.brain.pos[0]).powi(2)
                    + (o.pos[1] - self.brain.pos[1]).powi(2)
                    + (o.pos[2] - self.brain.pos[2]).powi(2);
                o.flags = 0x2 | if self.vision { 0x4 } else { 0 };
            }
        }
        {
            let ctx = BotCtx {
                now: self.now,
                scenario: 0x1,
                my_index: 4,
                my_team: 0,
                pickups: &self.pickups,
                others: &others,
                world: &self.world,
                health: 100.0,
                max_health: 100.0,
                stationary: self.stationary,
            };
            self.brain.step(&ctx, &mut self.rng, true);
        }
        self.now += 1.0 / 60.0;
        let (idx, t) = self.brain.last_visit_stamp;
        if idx != usize::MAX {
            self.pickups[idx].visit[0] = t;
            self.brain.last_visit_stamp = (usize::MAX, 0.0);
        }
    }
}

/// One bot on Arena 1 leaves its spawn through the real spawn path, picks a pickup goal from the
/// disc's placements, walks to it, and the visit float fires — the decompile's chain: BotInit →
/// 0xF9 → pickGoal (the deathmatch fall-through to pickups [M 0x126b88:317-319]) → initGoal +
/// gotoGoal (the 300.0 deadline [M 0x1278a8:50,58]) → 0xEB → the visit stamp now+45.0 [M
/// 0x129958:10-15] → 0xF9.
#[test]
fn bot_leaves_spawn_and_reaches_a_pickup_on_arena_1() {
    let Some(path) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let mut r = ArenaRig::new(&path);
    let start = r.brain.pos;
    r.step();
    assert_eq!(r.brain.state, nf_game::mp::bots::state::BOT_IDLE, "the BotInit chain lands on 0xF9");
    let mut left = false;
    let mut visited = false;
    for _ in 0..60 * 120 {
        r.step();
        if (r.brain.pos[0] - start[0]).powi(2) + (r.brain.pos[2] - start[2]).powi(2) > 1.0 {
            left = true;
        }
        if !r.brain.visited_pickups.is_empty() {
            visited = true;
            break;
        }
    }
    assert!(left, "the bot leaves its spawn; trace {:?}", r.brain.trace);
    assert!(visited, "the bot reaches a pickup within 120 s; pos {:?} trace {:?}", r.brain.pos, r.brain.trace);
    let i = r.brain.visited_pickups[0];
    let visit = r.pickups[i].visit[0];
    assert!(visit >= 45.0, "the MP_PICKUP+0x80 float = arrival + 45.0 [M 0x129958], got {visit}");
    let goal_pos = r.pickups[i].pos;
    assert!(
        (r.brain.pos[0] - goal_pos[0]).powi(2) + (r.brain.pos[2] - goal_pos[2]).powi(2) < 4.0,
        "the bot ends at the visited pickup; pos {:?} vs {:?}", r.brain.pos, goal_pos
    );
    // the trace walked the goal state
    assert!(r.brain.trace.iter().any(|e| e.2 == nf_game::mp::bots::state::BOT_GOTO_GOAL_POSITION));
    // sister within the same test: a second bot with the stand preset at the same spawn never moves
    let mut s = ArenaRig::new(&path);
    s.stationary = true;
    let sstart = s.brain.pos;
    for _ in 0..600 {
        s.step();
    }
    assert_eq!(s.brain.pos, sstart, "the stand preset holds position");
}

/// The fire decision on the same arena data: with the stationary player visible inside the
/// engage range, the bot's chain is the heartbeat sighted latch [M 0x16e060:504-509] → 0xEC
/// [M 0x175268:9-13] → 0xCF → 0xD2 → the burst deadline [M 0x16f5e8:25, 0x179d40] → a shot
/// aimed at the player, within the 5 s latch window.
#[test]
fn bot_takes_a_shot_at_a_stationary_player_on_arena_1() {
    let Some(path) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let mut r = ArenaRig::new(&path);
    r.pickups.clear(); // nothing to fetch: the idle bot turns to the candidate
    r.vision = true; // the CanSeeObject stand-in [S — the range/params are not lifted]
    // put the player 8 u from the bot (inside the 20 u engage stand-in [S combat-range default])
    r.others[0].pos = [r.brain.pos[0] + 8.0, r.brain.pos[1], r.brain.pos[2]];
    let mut shot_at = None;
    for i in 0..60 * 10 {
        r.step();
        if !r.brain.shots.is_empty() {
            shot_at = Some(r.brain.shots[0].tick);
            break;
        }
        assert!(i < 60 * 10 - 1, "no shot within 10 s; trace {:?}", r.brain.trace);
    }
    let t = shot_at.unwrap();
    let dir = r.brain.shots[0].dir;
    assert!(dir[0] > 0.99, "the shot aims at the stationary player, dir {dir:?}");
    assert!(t >= 7, "the first shot waits for the burst deadline [M 0x179d40], tick {t}");
    assert!(r.brain.trace.iter().any(|e| e.2 == nf_game::mp::bots::state::BOT_ATTACK_FIRE),
        "the 0xD2 fire state ran; trace {:?}", r.brain.trace);
}
