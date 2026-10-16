#![cfg(feature = "mp-bots")]
//! net-determinism — the MP match sim is a pure function of `(settings, seed, per-tick inputs)`.
//!
//! What these tests prove, and their sisters (the docs-first testing rule — every assertion here
//! pairs with one that would fail if the property were absent):
//!
//! 1. `two_sims_same_inputs_hash_identically_every_tick` — two INDEPENDENT sims (own `Rng`,
//!    own `CollisionWorld`, own brains) fed the same scripted inputs for 10 000 ticks, bots on,
//!    every scenario: the combined match+brains hash is equal **every tick**. Sister: the
//!    one-extra-RNG-draw sim diverges (proves the hash sees the shared stream), and the seed
//!    change diverges.
//! 2. `hash_catches_a_one_bit_state_difference` — flipping ONE bit of one field (an f32's bit
//!    pattern, a kill count, a settings word, a bot var) changes the hash. Sister: equal states
//!    hash equal (in `mp::hash`'s unit tests) — so the inequality comes from the bit, not noise.
//! 3. `replay_matches_the_recording_tick_for_tick` — a run driven live and the same run through
//!    `mp::replay::run_record` (settings + seed + per-tick inputs ONLY) produce identical
//!    per-tick hashes: the record is a complete lockstep transcript. Sister: dropping one
//!    `BulletHit` input from the record diverges from the dropped tick onward.
//!
//! Numbers and ids only — no game strings. Everything here is headless (no ISO): the arenas'
//! level data is not needed because the sim's inputs (spawn registrations, slot positions) are
//! scripted; the arena-content paths are `mp_arenas_live`/`mp_core_live`'s job.
//!
//! Run: `cargo test --offline --release -p nf-game --features mp-bots --test mp_lockstep`

use nf_game::drone::Rng;
use nf_game::mp::bots::{BotBrain, BotCtx, BotPickup, BotRecord, BotWeapon, OtherPlayer};
use nf_game::mp::hash::{state_hash, Hasher};
use nf_game::mp::replay::{apply_event, run_record, MpEvent, MpRecord, MpTickInput, ObjReg, SpawnReg, DT};
use nf_game::mp::{scenario, MpMatch, MpSettings};
use nf_game::world::CollisionWorld;

/// The soak length: 10 000 ticks ≈ 166.7 s of match time.
const TICKS: u32 = 10_000;

/// Every scenario word (the 12 `mp::scenario` constants — the menu's 13 rows map onto these).
const SCENARIOS: &[u32] = &[
    scenario::DEATHMATCH,
    scenario::TEAM_DEATHMATCH,
    scenario::CAPTURE_THE_FLAG,
    scenario::DEATH_LIMIT_FFA,
    scenario::DEMOLITION,
    scenario::PROTECTION,
    scenario::BLUEPRINT,
    scenario::GOLDENEYE,
    scenario::ASSASSIN,
    scenario::KOTH_FFA,
    scenario::KOTH_TEAM,
    scenario::UPLINK,
];

/// The joined slots: humans 0–1, bots 4–7 (the game's layout [M atlas §2]).
const JOINED: [usize; 6] = [0, 1, 4, 5, 6, 7];

/// Humans only — the replay-harness configuration. The bot brains draw from the shared `Rng` as
/// part of the simulation (their `Rand_*` draws are the game's `BOT_*` rolls), so a core-event
/// record is a COMPLETE transcript only for a bots-off match; bots-on determinism is proven by
/// the two-sim soak (every peer drives the same brains from the same stream — they are not
/// inputs and are not recorded).
const HUMANS: [usize; 2] = [0, 1];

fn settings_for(scen: u32) -> MpSettings {
    let mut s = MpSettings::deathmatch(0x07000011);
    s.scenario = scen;
    s.frag_limit = -1; // never ends inside the soak
    s.time_limit = -1;
    s.human_count = 2;
    s.bot_count = 4;
    s.total_players = 6;
    s.player_count = 2;
    s.spawn_policy = 2; // the drawing policy — spawn picks consume the shared stream [M 0x185098]
    for (i, p) in s.players.iter_mut().enumerate() {
        p.team = (i % 2) as u32; // harmless in FFA; consistent in team games
    }
    s
}

/// The replay configuration: humans only, no bots (see the `HUMANS` note).
fn settings_humans(scen: u32) -> MpSettings {
    let mut s = settings_for(scen);
    s.bot_count = 0;
    s.total_players = 2;
    s
}

/// The one bot boot record (preset 0 stats), enabled — the rig's record from `mp_bots_live` [L
/// ee_snapshot: the menu's +0x0E enable byte, preset stats].
fn bot_record() -> BotRecord {
    let mut rec = BotRecord([0x01, 0x00, 0x04, 0x00, 0x96, 0x00, 0x02, 0x96, 0x32, 0x00, 0x00, 0x00, 0x00, 0x01,
        0x00, 0x00, 0x06, 0x00]);
    rec.0[0x0E] = 1;
    rec
}

/// One independent sim + its four bot brains. NOTHING is shared with any other sim: own `Rng`
/// (seeded `seed`), own empty `CollisionWorld`, own brains — what lockstep peers are.
struct Sim {
    m: MpMatch,
    brains: Vec<BotBrain>,
    rng: Rng,
    world: CollisionWorld,
    pickups: Vec<BotPickup>,
    now: f32,
    spawns: Vec<usize>,
    spawn_regs: Vec<SpawnReg>,
    obj_regs: Vec<ObjReg>,
}

fn build_sim(scen: u32, seed: u32, humans_only: bool) -> Sim {
    let joined: &[usize] = if humans_only { &HUMANS } else { &JOINED };
    let mut rng = Rng::new(seed);
    let mut m = MpMatch::init(if humans_only { settings_humans(scen) } else { settings_for(scen) });
    let world = CollisionWorld::from_triangles(vec![]);
    // 8 registered spawn points, alternating teams (the sim's own registration path).
    let mut spawn_regs = Vec::new();
    for i in 0..8usize {
        let pos = [10.0 + 13.0 * i as f32, 0.0, 5.0 + 7.0 * i as f32];
        m.register_spawn_point(pos, [0.0, 0.0, 1.0], (i % 2) as u16, &world);
        spawn_regs.push(SpawnReg { pos, facing: [0.0, 0.0, 1.0], team: (i % 2) as u16 });
    }
    // the scenario's MP objects (the 0x26 placements create_mode_objects builds from):
    // config word 0 = the type, word 1 = the team [M 0x185990].
    let cfg = |ty: u32, team: u32| [ty, team, 0, 0, 0, 0, 0];
    let mut obj_regs: Vec<ObjReg> = Vec::new();
    macro_rules! reg {
        ($pos:expr, $ty:expr, $team:expr) => {{
            m.register_mp_object($pos, [0.0, 0.0, 1.0], $ty, cfg($ty, $team), 0);
            obj_regs.push(ObjReg { pos: $pos, facing: [0.0, 0.0, 1.0], mp_type: $ty, config: cfg($ty, $team), cel: 0 });
        }};
    }
    match scen {
        scenario::CAPTURE_THE_FLAG => {
            reg!([10.0, 0.0, 80.0], 0, 0);
            reg!([10.0, 0.0, 90.0], 0, 1);
            reg!([20.0, 0.0, 80.0], 1, 0);
            reg!([20.0, 0.0, 90.0], 1, 1);
        }
        scenario::UPLINK => {
            reg!([15.0, 0.0, 85.0], 2, 0);
        }
        scenario::BLUEPRINT => {
            reg!([15.0, 0.0, 85.0], 4, 0);
            reg!([30.0, 0.0, 85.0], 5, 0);
            reg!([40.0, 0.0, 85.0], 5, 1);
        }
        scenario::GOLDENEYE => {
            reg!([30.0, 0.0, 85.0], 6, 0);
            reg!([40.0, 0.0, 85.0], 6, 1);
            reg!([50.0, 0.0, 85.0], 7, 0);
            reg!([60.0, 0.0, 85.0], 7, 1);
        }
        scenario::DEMOLITION => {
            reg!([30.0, 0.0, 85.0], 3, 0);
            reg!([40.0, 0.0, 85.0], 3, 1);
        }
        scenario::PROTECTION => {
            reg!([30.0, 0.0, 85.0], 8, 0);
            reg!([40.0, 0.0, 85.0], 8, 1);
        }
        scenario::KOTH_FFA | scenario::KOTH_TEAM => {
            reg!([25.0, 0.0, 85.0], 9, 0);
        }
        _ => {}
    }
    m.start(&mut rng);
    m.create_mode_objects(&mut rng); // the scenario objects (draws for demo/prot/GE/blueprint)
    // humans 0–1, bots 4–7, all through the policy draw (consumes the shared stream).
    let mut spawns: Vec<usize> = Vec::new();
    for &slot in joined {
        let team = m.settings.players[slot].team as u16;
        let idx = m.get_spawn_point(team, slot, &mut rng) as usize;
        let p = m.spawns.points[idx].pos;
        let pos = [p[0], p[1], p[2]];
        m.set_slot(slot, pos);
        spawns.push(slot);
    }
    let pickups: Vec<BotPickup> = (0..4)
        .map(|i| BotPickup {
            pos: [40.0 + 9.0 * i as f32, 0.0, 30.0 + 11.0 * i as f32],
            weapon_id: 0,
            mp_class: 0,
            is_obj_flag2: false,
            visit: [0.0; 4],
        })
        .collect();
    let rec = bot_record();
    let brains = joined[2..]
        .iter()
        .map(|&slot| {
            let pos = m.game.slots[slot].pos.unwrap();
            let mut b = BotBrain::bot_init(&rec, slot as u8, pos, 0.0);
            b.weapon = BotWeapon { interval: 6, burst: 3, aggression_mul: 1.0 };
            b.vars.weapon_id = 0x02;
            b.vars.clip_rounds = 50;
            b
        })
        .collect();
    Sim { m, brains, rng, world, pickups, now: 0.0, spawns, spawn_regs, obj_regs }
}

/// Drives one tick: the recorded-input order — positions → events → `update_objects` → `update`
/// — then steps every bot brain on the shared sim (`ctx.now` accumulated 1/60 per tick, the
/// established rig pattern).
fn drive(sim: &mut Sim, _t: u32, input: &MpTickInput) -> (u64, u64) {
    for &(slot, pos) in &input.set_slots {
        sim.m.set_slot(slot, pos);
    }
    for ev in &input.events {
        apply_event(&mut sim.m, ev, &mut sim.rng);
    }
    sim.m.update_objects(DT, &input.on_hill, &mut sim.rng);
    sim.m.update(DT, input.paused, &mut sim.rng);

    let others: Vec<OtherPlayer> = (0..8usize)
        .map(|i| {
            let pos = sim.m.game.slots[i].pos.unwrap_or([0.0; 3]);
            OtherPlayer {
                index: i,
                pos,
                alive: sim.m.game.slots[i].pos.is_some() && !sim.m.game.slots[i].dead,
                team: sim.m.settings.players[i].team,
                flags: 0x2 | 0x4, // present + visible (empty world ⇒ LOS clear)
                dist2: 0.0,
                health: 100.0,
                score: sim.m.game.slots[i].score,
            }
        })
        .collect();
    for bi in 0..sim.brains.len() {
        let slot = sim.brains[bi].vars.slot as usize;
        let mut others = others.clone();
        for o in others.iter_mut() {
            o.dist2 = (o.pos[0] - sim.brains[bi].pos[0]).powi(2)
                + (o.pos[1] - sim.brains[bi].pos[1]).powi(2)
                + (o.pos[2] - sim.brains[bi].pos[2]).powi(2);
        }
        let ctx = BotCtx {
            now: sim.now,
            scenario: sim.m.settings.scenario,
            my_index: slot,
            my_team: sim.m.settings.players[slot].team,
            pickups: &sim.pickups,
            others: &others,
            world: &sim.world,
            health: 100.0,
            max_health: 100.0,
            stationary: false,
        };
        sim.brains[bi].step(&ctx, &mut sim.rng, true);
    }
    sim.now += DT;

    // match-only hash (what `run_record` returns) and the combined match+brains hash.
    let match_hash = state_hash(&sim.m);
    let mut h = Hasher::new();
    h.u64(match_hash);
    for b in &sim.brains {
        h.u64(b.state_hash());
    }
    (match_hash, h.finish())
}

/// The scripted per-tick inputs — a pure function of (scenario, tick), built once and fed to
/// BOTH sims. This is the lockstep input stream.
fn build_script(scen: u32, ticks: u32, joined: &[usize]) -> Vec<MpTickInput> {
    let n = joined.len();
    let mut script = Vec::with_capacity(ticks as usize);
    for t in 0..ticks {
        let mut input = MpTickInput::default();
        // the humans patrol (their per-tick input: a position update each tick)
        input.set_slots.push((0, [20.0 + ((t % 600) as f32) * 0.05, 0.0, 10.0]));
        input.set_slots.push((1, [30.0 - ((t % 450) as f32) * 0.05, 0.0, 20.0]));
        // a kill every 90 ticks across the joined slots (assassin resets, streaks, last_killer)
        if t % 90 == 17 {
            let vi = ((t / 90) % n as u32) as usize;
            let ki = (((t / 90) + 1) % n as u32) as usize;
            input.events.push(MpEvent::Killed {
                victim: joined[vi],
                killer: Some(joined[ki]),
                explosive: t % 180 == 17,
                golden_gun: t % 270 == 17,
            });
        }
        // a bullet hit every 37 ticks (friendly-fire / last-shooter paths)
        if t % 37 == 5 {
            let vi = ((t / 37) % n as u32) as usize;
            let ai = (((t / 37) + 3) % n as u32) as usize;
            input.events.push(MpEvent::BulletHit { victim: joined[vi], attacker: joined[ai] });
        }
        // a respawn every 300 ticks (a policy draw + the spawn reset)
        if t % 300 == 100 {
            let si = ((t / 300) % n as u32) as usize;
            input.events.push(MpEvent::Respawn { slot: joined[si] });
        }
        // the scenario-object touches
        if t % 240 == 77 {
            let slot = joined[(t / 240) as usize % 4.min(n)];
            match scen {
                scenario::CAPTURE_THE_FLAG => input.events.push(MpEvent::TouchFlag { slot }),
                scenario::UPLINK => input.events.push(MpEvent::TouchUplink { term: 0, slot }),
                scenario::GOLDENEYE => input.events.push(MpEvent::TouchGeItem { item: 0, slot }),
                scenario::BLUEPRINT => input.events.push(MpEvent::TouchBlueprint { slot }),
                scenario::DEMOLITION | scenario::PROTECTION => {
                    input.events.push(MpEvent::DamageSite { dmg: 0.5, hitter: slot });
                }
                _ => {}
            }
        }
        if scen == scenario::GOLDENEYE && t == 9000 {
            input.events.push(MpEvent::GeStrikeComplete);
        }
        // the hill mask rotates over the joined slots in the KOTH scenarios
        if matches!(scen, scenario::KOTH_FFA | scenario::KOTH_TEAM) {
            input.on_hill[joined[(t / 60) as usize % 4.min(n)]] = true;
        }
        script.push(input);
    }
    script
}

struct RunOut {
    match_hashes: Vec<u64>,
    full_hashes: Vec<u64>,
    sim: Sim,
}

fn run_sim(
    scen: u32,
    seed: u32,
    script: &[MpTickInput],
    extra_draw_before_start: bool,
    humans_only: bool,
) -> RunOut {
    let mut sim = build_sim(scen, seed, humans_only);
    if extra_draw_before_start {
        // an out-of-band draw on the shared stream — the sister sim's unaccounted RNG use
        let _ = sim.m.get_spawn_point(0, 0, &mut sim.rng);
    }
    let mut match_hashes = Vec::with_capacity(script.len());
    let mut full_hashes = Vec::with_capacity(script.len());
    for (t, input) in script.iter().enumerate() {
        let (mh, fh) = drive(&mut sim, t as u32, input);
        match_hashes.push(mh);
        full_hashes.push(fh);
    }
    RunOut { match_hashes, full_hashes, sim }
}

/// The soak: two independent sims, same inputs, 10 000 ticks, bots on, every scenario —
/// identical combined hashes every tick.
#[test]
fn two_sims_same_inputs_hash_identically_every_tick() {
    for &scen in SCENARIOS {
        let script = build_script(scen, TICKS, &JOINED);
        let a = run_sim(scen, 0x1234_5678, &script, false, false);
        let b = run_sim(scen, 0x1234_5678, &script, false, false);
        assert_eq!(a.full_hashes.len(), TICKS as usize);
        for (t, (x, y)) in a.full_hashes.iter().zip(b.full_hashes.iter()).enumerate() {
            assert_eq!(x, y, "scenario {scen:#x} diverged at tick {t}");
        }
        // the run was not vacuous (DM checked in depth; every scenario's clock folded 10 000
        // ticks: 10 000 × fl(1/60) bit-exact in both sims — equality itself pins the fold)
        if scen == scenario::DEATHMATCH {
            assert!(a.sim.m.game.clock > 100.0, "the round clock folded the soak");
            let killer_kills: i32 = a.sim.m.game.slots.iter().map(|s| s.kills.max(0)).sum();
            assert!(killer_kills > 0, "the scripted kills scored");
            assert!(a.sim.brains.iter().any(|b| !b.shots.is_empty()), "a bot fired");
        }
    }
}

/// Sister A: one unaccounted draw on the shared stream desyncs the run — the hash sees it.
#[test]
fn one_extra_rng_draw_diverges() {
    let script = build_script(scenario::DEATHMATCH, 2000, &JOINED);
    let a = run_sim(scenario::DEATHMATCH, 0x1234_5678, &script, false, false);
    let b = run_sim(scenario::DEATHMATCH, 0x1234_5678, &script, true, false);
    let first = a.match_hashes.iter().zip(b.match_hashes.iter()).position(|(x, y)| x != y);
    assert!(first.is_some(), "the extra draw must desync the shared stream");
}

/// Sister B: a different seed (the per-match seeding) produces a different run.
#[test]
fn different_seed_diverges() {
    let script = build_script(scenario::CAPTURE_THE_FLAG, 2000, &JOINED);
    let a = run_sim(scenario::CAPTURE_THE_FLAG, 7, &script, false, false);
    let b = run_sim(scenario::CAPTURE_THE_FLAG, 8, &script, false, false);
    let first = a.match_hashes.iter().zip(b.match_hashes.iter()).position(|(x, y)| x != y);
    assert!(first.is_some(), "seeds must matter — one stream per match");
}

/// The hash's sensitivity: flipping ONE bit of one state field flips the hash. An f32 is hashed
/// by bit pattern (not value), an integer by its bytes, a bot var by its own hash.
#[test]
fn hash_catches_a_one_bit_state_difference() {
    let script = build_script(scenario::DEATHMATCH, 300, &JOINED);
    let mut a = run_sim(scenario::DEATHMATCH, 0x1234_5678, &script, false, false);
    let base = state_hash(&a.sim.m);

    // 1. one BIT of an f32 (the score): a=0.0/b=−0.0-style neighbor
    let bits = a.sim.m.game.slots[0].score.to_bits();
    a.sim.m.game.slots[0].score = f32::from_bits(bits ^ 1);
    assert_ne!(state_hash(&a.sim.m), base, "an f32 bit flip must show");
    a.sim.m.game.slots[0].score = f32::from_bits(bits);

    // 2. one bit of an integer (a kill count)
    a.sim.m.game.slots[1].kills ^= 1;
    let after_int = state_hash(&a.sim.m);
    assert_ne!(after_int, base, "an int bit flip must show");
    a.sim.m.game.slots[1].kills ^= 1;

    // 3. one bit of a settings word
    a.sim.m.settings.radar ^= 1;
    assert_ne!(state_hash(&a.sim.m), base, "a settings bit flip must show");
    a.sim.m.settings.radar ^= 1;

    // 4. one bit of a bot brain var (the brains are sim state too)
    let bh = a.sim.brains[0].state_hash();
    a.sim.brains[0].vars.flags ^= 1;
    assert_ne!(a.sim.brains[0].state_hash(), bh, "a bot var bit flip must show");
    a.sim.brains[0].vars.flags ^= 1;
    assert_eq!(a.sim.brains[0].state_hash(), bh);

    // 5. and the restored state hashes back to the base (no hidden drift)
    assert_eq!(state_hash(&a.sim.m), base);
}

/// The replay contract: a live run and `run_record` over (settings, seed, per-tick inputs) hash
/// identically every tick — the record is a complete transcript.
#[test]
fn replay_matches_the_recording_tick_for_tick() {
    let scen = scenario::TEAM_DEATHMATCH;
    let script = build_script(scen, 4000, &HUMANS);
    let run = run_sim(scen, 0x00C0FFEE, &script, false, true);
    let rec = MpRecord {
        seed: 0x00C0FFEE,
        settings: settings_humans(scen),
        with_mode_objects: true,
        spawn_registrations: run.sim.spawn_regs.clone(),
        object_registrations: run.sim.obj_regs.clone(),
        slot_spawns: run.sim.spawns.clone(),
        ticks: script.clone(),
    };
    let (_m2, replay_hashes) = run_record(&rec);
    assert_eq!(replay_hashes.len(), run.match_hashes.len());
    for (t, (x, y)) in run.match_hashes.iter().zip(replay_hashes.iter()).enumerate() {
        assert_eq!(x, y, "the replay diverged from the recording at tick {t}");
    }
}

/// Sister: one dropped input (a bullet hit at tick 500) diverges from the dropped tick onward —
/// the transcript is exact, not approximate.
#[test]
fn replay_catches_a_dropped_input() {
    let scen = scenario::DEATHMATCH;
    let script = build_script(scen, 2000, &HUMANS);
    let run = run_sim(scen, 42, &script, false, true);
    let mut script2 = script.clone();
    // drop the kill at tick 467 (467 % 90 == 17 — a scripted kill tick)
    assert_eq!(467 % 90, 17);
    let dropped = script2[467]
        .events
        .iter()
        .position(|e| matches!(e, MpEvent::Killed { .. }))
        .expect("the script kills at tick 467");
    script2[467].events.remove(dropped);
    let rec = MpRecord {
        seed: 42,
        settings: settings_humans(scen),
        with_mode_objects: true,
        spawn_registrations: run.sim.spawn_regs.clone(),
        object_registrations: run.sim.obj_regs.clone(),
        slot_spawns: run.sim.spawns.clone(),
        ticks: script2,
    };
    let (_m2, hashes) = run_record(&rec);
    assert_eq!(hashes[466], run.match_hashes[466], "identical up to the drop");
    assert_ne!(hashes[467], run.match_hashes[467], "the dropped input must show");
    let last = hashes.len() - 1;
    assert_ne!(hashes[last], run.match_hashes[last], "the divergence persists");
}
