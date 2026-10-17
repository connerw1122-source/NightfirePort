//! guard-blast-smoke (task 727, uncited-audit gap 2) on 07000005 — grenades and smoke did nothing
//! to guards: the port applied the blast as direct damage × `+0x14C` and sent msg 8 (the bullet
//! route), and the smoke cloud was visual-only.
//!
//! The game's path [M]:
//! - `Explode_Propagate` case 2 → `Drone_ExplosiveHit` 0x13aab8: the +0x2B4 attacker stamp, then
//!   `SendMsgSelf(9, DroneImpact*, 0, 0)`. `DroneFunc_HandleImpact` case 9 (0x1469b8:173-201):
//!   allow = 0 → `NDrone2_ExplosiveImpact(impact, resume, 0)` — damage + PainTalk + the resume
//!   entry (the idle/patrol guards' (0x56, 0) pair: hostile at once, no stagger); allow = 1 → the
//!   +0x118 resume stamp and state 0x54 (the 0x47 stagger anim). A kill routes per kind: 0x0D →
//!   0x44, 0x1B/0x1C → 0x92, 0x1D → 0xBF, else **0x45 DeathByExplosion** [M 0x1465e4-0x14697c] —
//!   the launch anim 0x44 with the `NDrone2_ExplosionDir` variant, follow-on 0x47 DEAD.
//! - The damage is the raw falloff amount: `HitDamage(f12 = *(float*)impact, part −1)` [M disasm
//!   0x14654c] — no `+0x14C` multiply on this path (that is the bullet callers' job).
//! - The smoke cloud (class 0x2A, `Occlude_Create(pos, 1200, 1)` 0x17ea80) touches the drone in
//!   the per-tick collision scan (`Drone_CollisionHandler` 0x140968:85) and
//!   `NDrone2_DealWithObjHit` case 0x2A sends msg 0x17 [M disasm 0x1405a8]. `HandleImpact` case
//!   0x17 (props 0x36/0x34 immune) always enters 0x7B: cough 0x20 → 0x7C (cough again, timer C at
//!   frame + 5; further 0x17 re-arms) → timer C → 0x7D (anim 10, follow-on the stamped resume) [M
//!   0x1469b8:197-249, 0x173c50, 0x16b878, 0x173d30].
//!
//! 07000005's guards are all 10 health [L the roster run] and the frag's minimum in-radius damage
//! is 12.5 (radius 8, damage 100 [L record 0x34]) — inside the radius every guard dies, so the
//! frag's real-path pins here are the kill (0x45, not the old 0x44) and the untouched control. The
//! surviving-guard stagger (0x54) is pinned at the state-machine level with a direct msg-9 send on
//! a guard standing in AimStandFire (the firing states are the (current, 1) family [M DSTATE scan]
//! — no 07000005 blast survives at 10 hp to reach it live).
//!
//! Opt-in: set `NF_ISO` to the disc image.

use nf_formats::anim::Script;
use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, drone, level::Level, levelbin::LevelBin, weapon::WeaponData};
use nf_game::drone::{msg, state, Data, Drones, Player, Rng};
use std::collections::HashMap;

pub struct L {
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
    volumes: Vec<nf_game::aivolume::AiVolume>,
    models: HashMap<u32, nf_game::pickup::Model>,
    tuning: nf_formats::tuning::Tuning,
    hitboxes: nf_formats::hitbox::CollisionBones,
    sfx: Option<nf_formats::sfx::SfxTable>,
}

impl L {
    pub fn load() -> Option<L> {
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
        let skeletons = lb.entries.iter().filter(|e| e.kind == 6).filter_map(|e| nf_formats::character::parse_skeleton(lb.data(&data, e)).ok().map(|s| (s.id as u8, s))).collect();
        let m = level.main_map().unwrap();
        let nav = nf_game::nav::Network::build(level.maps[m].ai_network.as_ref().unwrap(), &world, 0x0700_0005);
        let covers = nf_game::cover::build(&level, &nav, &world);
        let volumes = nf_game::aivolume::build(&level, 0x0700_0005);
        Some(L {
            tables: drone::DroneAnimData::from_action_elf(&elf).unwrap(),
            props: drone::PropertyTables::from_action_elf(&elf).unwrap(),
            weapons: WeaponData::from_action_elf(&elf).unwrap(),
            tuning: { let mut t = nf_formats::tuning::Tuning::from_action_elf(&elf).unwrap(); nf_game::tuning::read_tuning_vars(&mut t, 0x0700_0005); t },
            hitboxes: nf_formats::hitbox::CollisionBones::from_action_elf(&elf).unwrap(),
            sequences: disc::level_sequences(&iso, "07000005.bin").unwrap(),
            models: nf_game::pickup::models(&level),
            sfx: nf_formats::sfx::SfxTable::from_action_elf(&elf).ok(),
            level, scripts, world, skeletons, nav, covers, volumes,
        })
    }

    pub fn data(&self) -> Data<'_> {
        Data { tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world, level: 0x0700_0005,
            sequences: &self.sequences, skins: &self.level.skins, skeletons: &self.skeletons, nav: &self.nav, covers: &self.covers, tuning: &self.tuning,
            hitboxes: &self.hitboxes, volumes: &self.volumes, sfx: self.sfx.as_ref(), models: &self.models, surfaces: None, voices: None, difficulty: 1,
            ai_points: &[] }
    }

    /// The full roster with one Rng through `level_drones` (the game's order), then a fresh `Drones`
    /// rng for the tick stream.
    pub fn sim(&self) -> (Drones, Rng) {
        let d = self.data();
        let mut rng = Rng::new(0x1234_5678);
        let mut lights = nf_game::light::LightList::default();
        let drones = nf_game::drone::level_drones(&d, &self.level, &mut rng, &mut lights);
        let s = Drones::new(drones, Rng::new(0x777));
        (s, Rng::new(0x777))
    }
}

pub fn bond_at(p: [f32; 3]) -> Player { Player { pos: p, target: [p[0], p[1] + 0.5, p[2]], eye: [p[0], p[1] + 1.0, p[2]], aim: [-1.0, 0.0, 0.0], sph: p, capsule: ([p[0], p[1] + 1.086, p[2]], p), hanging: false, cel: None } }

/// A live grenade at rest at `pos` (the flashbang harness's shape): the fuse is the +0x70 & 0x200
/// record bit's 210 ticks ([L record 0x34/0x35/0x36 all carry it]), the owner usize::MAX = the
/// player's throw, state 0 so the spawn tick runs and the fuse ticks from tick 1.
pub fn grenade_at(d: &Data, weapon_type: u16, pos: [f32; 3]) -> nf_game::grenade::Grenade {
    let rec = &d.weapons.records[weapon_type as usize];
    let f32_at = |o: usize| f32::from_le_bytes(rec[o..o + 4].try_into().unwrap());
    nf_game::grenade::Grenade {
        pos,
        velocity: [0.0, 0.0, 0.0],
        speed: 0.0,
        travelled: 0.0,
        range: 210.0,
        damage: f32_at(0x0c),
        radius: f32_at(0x08),
        weapon_type,
        mesh: 0,
        owner: usize::MAX,
        state: 0,
        bounces: 0,
        smoke_popped: false,
        resting: true,
        stuck: false,
        seek_target: None,
        phase: None,
    }
}

fn idx(sim: &Drones, placement: usize) -> usize {
    sim.list.iter().position(|dr| dr.placement == placement).unwrap_or_else(|| panic!("placement {placement} on the roster"))
}

/// The frag kill is the game's DeathByExplosion: a 0x34 popped a few units from the yard guards
/// kills every guard the falloff reaches (radius 8, damage 100 [L]; 10-health guards [L]) and each
/// of them routes 0x45 — NOT the bullet death 0x44 the port used to set — then DEAD with the death
/// channel raised; a guard outside the radius is untouched (no damage, no alert, no state change).
#[test]
fn frag_kill_routes_death_by_explosion_and_the_control_is_untouched() {
    let Some(l) = L::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    // [L record 0x34]: radius 8, damage 100 — the real frag's blast.
    let rec = &d.weapons.records[0x34];
    let f32_at = |o: usize| f32::from_le_bytes(rec[o..o + 4].try_into().unwrap());
    let (radius, damage) = (f32_at(0x08), f32_at(0x0c));
    assert_eq!((radius, damage), (8.0, 100.0), "the frag record [L 0x2bf150 + 0x34·0x10C]");

    let (mut sim, _rng) = l.sim();
    // The ENABLED guards (the WaitSwitch waiters are disabled — `NDrone2_Enable(0)` sets
    // obj+0xF0 & 0x20 and the collide engine skips them, so no blast reaches them [M 0x1eae80]):
    // pl 5 (idle, 61.3/27.1/99.5) and pl 6 (patrol, 50.8/27.9/93.6) are 11.7 apart — a blast at
    // their midpoint is ~5.9 from each (100·(1−trunc(5.5)/8) = 37.5 → both die); pl 8 (150.2,
    // 16.8, 100.8) is the far control.
    let a = idx(&sim, 5);
    let b = idx(&sim, 6);
    let far = idx(&sim, 8);
    // Settle one tick (the started ENTERs: every guard sits in his start state) before baselines.
    let p0 = bond_at([-500.0, 0.0, -500.0]);
    sim.tick(&d, &p0);
    // The blast at the pair's midpoint.
    let pa = sim.list[a].pos;
    let pb = sim.list[b].pos;
    let blast = [(pa[0] + pb[0]) / 2.0, pa[1] + 0.4, (pa[2] + pb[2]) / 2.0];
    let da = ((pa[0] - blast[0]).powi(2) + (pa[1] - blast[1]).powi(2) + (pa[2] - blast[2]).powi(2)).sqrt();
    let db = ((pb[0] - blast[0]).powi(2) + (pb[1] - blast[1]).powi(2) + (pb[2] - blast[2]).powi(2)).sqrt();
    assert!(da < 7.0 && db < 7.0, "both guards inside the kill band, da {da} db {db}");
    let pf = sim.list[far].pos;
    let df = ((pf[0] - blast[0]).powi(2) + (pf[1] - blast[1]).powi(2) + (pf[2] - blast[2]).powi(2)).sqrt();
    assert!(df > radius + 1.0, "the control guard outside the radius, dist {df}");
    let far_health = sim.list[far].health;
    let far_state = sim.list[far].state;

    sim.grenades.push(grenade_at(&d, 0x34, blast));
    // Bond far from the blast so his own hurt path is out of the picture.
    let p = bond_at([blast[0] + 60.0, blast[1], blast[2]]);
    for f in 1..=500u32 {
        sim.tick(&d, &p);
        if f > 220 && matches!(sim.list[a].state, state::DEAD | state::FADE | state::FADE_FAST)
            && matches!(sim.list[b].state, state::DEAD | state::FADE | state::FADE_FAST) { break; }
    }
    // Both in-radius guards died THROUGH the game's route: DeathByExplosion (0x45) first, then
    // DEAD. (The old port set Death_Anim 0x44 from spawn_blast directly — the bullet death.)
    for (k, &i) in [a, b].iter().enumerate() {
        let log = &sim.list[i].log;
        assert!(log.iter().any(|&(_, s)| s == state::DEATH_BY_EXPLOSION),
            "guard {k}: the kill routed DeathByExplosion, log {:?} tail {:?}", &log[..log.len().min(6)], log.iter().rev().take(4).collect::<Vec<_>>());
        assert!(matches!(sim.list[i].state, state::DEAD | state::FADE | state::FADE_FAST),
            "guard {k}: dead at the end, state {:#x}", sim.list[i].state);
        assert!(sim.list[i].health <= 0.0, "guard {k}: health {}", sim.list[i].health);
    }
    // The control: untouched — no damage (the falloff cut him), no state change, no alert.
    assert_eq!(sim.list[far].health, far_health, "the control guard's health is untouched");
    assert_eq!(sim.list[far].state, far_state, "the control guard's state is untouched");
    assert!(sim.list[far].alert < 1.0, "the control guard never alerted (the game's msg-9 path alerts only the hit drones)");
}

/// The smoke states: a 0x36 popped at a guard raises the cloud (`Occlude_Create(pos, 1200, 1)`);
/// the guard inside runs SmokedOut 0x7B → 0x7C and STAYS in the family while the cloud lives (the
/// per-tick 0x17 re-arms), and when the cloud clears he recovers 0x7D → the resume state. The
/// prop-0x34 guard (gas-proof — pl 5/6/8/9 carry it on this level [L the props run]) is immune
/// [M 0x1469b8:198-205]; a guard outside the cloud never enters it.
#[test]
fn smoke_cloud_puts_guards_in_the_smoke_states_until_it_clears() {
    let Some(l) = L::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let (mut sim, _rng) = l.sim();
    // pl 7 (the tower sniper, 62.2/34.5/104.2, no immunity props [L]) smokes; pl 5 carries prop
    // 0x34 (gas-proof) and must not; pl 267 (the other tower, 15.7 away) is the far control.
    let a = idx(&sim, 7);
    let immune = idx(&sim, 5);
    let far = idx(&sim, 267);
    // Settle one tick (the started ENTERs) before the baselines.
    let p0 = bond_at([-500.0, 0.0, -500.0]);
    sim.tick(&d, &p0);
    let pa = sim.list[a].pos;
    let pi = sim.list[immune].pos;
    let pf = sim.list[far].pos;
    let far_dist = ((pf[0] - pa[0]).powi(2) + (pf[2] - pa[2]).powi(2)).sqrt();
    assert!(far_dist > 15.0, "the control guard far outside the cloud, {far_dist}");
    let (far_health, far_state) = (sim.list[far].health, sim.list[far].state);

    // Two grenades: one at the smoker, one at the immune guard (both pop at the 210 fuse).
    sim.grenades.push(grenade_at(&d, 0x36, [pa[0], pa[1] + 0.4, pa[2]]));
    sim.grenades.push(grenade_at(&d, 0x36, [pi[0], pi[1] + 0.4, pi[2]]));
    let p = bond_at([pa[0] + 60.0, pa[1], pa[2]]);
    // Tick to the pop (the 210 fuse).
    let mut popped = 0;
    for f in 1..=240u32 {
        sim.tick(&d, &p);
        if sim.smoke_clouds.len() == 2 { popped = f; break; }
    }
    assert!(popped > 0, "both smoke grenades popped");
    assert_eq!(sim.smoke_clouds[0].ticks, 1200, "the cloud's lifetime is the re-armed fuse [M 0x12e114]");
    let family = |s: u8| matches!(s, state::SMOKED_OUT | state::SMOKED_OUT_LOOP | state::SMOKED_OUT_RECOVER);
    // The in-cloud guard enters the family and stays in it while the cloud lives: sample across
    // 90 ticks (the cough anim runs, the Loop re-arms every tick, the 5-tick timer cannot win).
    let mut seen_enter = false;
    for _ in 1..=90u32 {
        sim.tick(&d, &p);
        if family(sim.list[a].state) { seen_enter = true; }
    }
    assert!(seen_enter, "the guard entered the smoke family, state {:#x}", sim.list[a].state);
    assert!(family(sim.list[a].state), "still smoked at pop+90 (the cloud re-arms), state {:#x}", sim.list[a].state);
    assert!(sim.list[a].health == 10.0, "smoke never damages, health {}", sim.list[a].health);
    // The prop-0x34 guard: immune, never in the family (his cloud is at his own feet).
    assert!(!family(sim.list[immune].state), "the gas-proof guard never smoked, state {:#x}", sim.list[immune].state);
    // The control: untouched.
    assert!(!family(sim.list[far].state), "the control guard never smoked");
    assert_eq!(sim.list[far].state, far_state, "the control guard's state is untouched");
    assert_eq!(sim.list[far].health, far_health, "the control guard's health is untouched");

    // The cloud clears → the Loop's last timer runs out → Recover (0x7D) → the resume state (the
    // stamped +0x118: the sniper-idle pair's 0x2A SniperAim). Shrink the cloud's life (the port's
    // public field — the game reaches the same point by its 1200-tick countdown) and tick past
    // the 5-tick timer and the recover anim.
    for c in &mut sim.smoke_clouds { c.ticks = 2; }
    let mut recovered = false;
    for _ in 0..240 {
        sim.tick(&d, &p);
        // The stamped resume for a sniper-idle guard is 0x2A (his pair's resume); the state may
        // progress into the fire state.
        if matches!(sim.list[a].state, state::SNIPER_AIM | state::SNIPER_FIRE) { recovered = true; }
        if recovered { break; }
    }
    assert!(recovered, "the guard recovered to the resume state after the cloud cleared, state {:#x}", sim.list[a].state);
    assert!(!family(sim.list[a].state), "left the smoke family");
}

/// The surviving-guard stagger (HandleImpact case 9's allow arm): a guard in an allow = 1 state
/// (the firing family) hit by a survivable blast enters 0x54, stamps the resume (+0x118) and
/// returns to it when the 0x47 stagger anim completes; the damage is the raw falloff amount (no
/// +0x14C). Driven through `send(msg::EXPLOSION)` — no 07000005 blast is survivable at 10 hp to
/// reach it live (the frag's minimum in-radius damage is 12.5).
#[test]
fn blast_on_a_firing_guard_staggers_through_0x54_back_to_the_resume() {
    let Some(l) = L::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let (mut sim, mut rng) = l.sim();
    // An enabled guard (the per-drone tick must run for the anim to progress).
    let i = idx(&sim, 5);
    // Settle one tick, then force the state (the direct write is the tests' established stand-in
    // for a state the level's scripting cannot reach on demand — ai_live.rs:1276). The settle
    // tick consumes the first-ENTER arm; a forced state afterwards is the state machine's own.
    let p0 = bond_at([-500.0, 0.0, -500.0]);
    sim.tick(&d, &p0);
    // AimStandFire (0x68): an allow = 1 state (the (current, 1) family [M DSTATE scan]).
    sim.list[i].state = state::AIM_STAND_FIRE;
    let pos = sim.list[i].pos;
    let health = sim.list[i].health;
    // The msg-9 payload: last_damage = the falloff amount, blast = the impact's +0x10 VECTOR —
    // a survivable 5.0, from the front (variant 1).
    sim.list[i].last_damage = 5.0;
    sim.list[i].blast = Some([pos[0], pos[1], pos[2] + 3.0]);
    sim.list[i].send(&d, &mut rng, sim.frame, msg::EXPLOSION);
    assert_eq!(sim.list[i].state, state::EXPLOSIVE_IMPACT, "the stagger state 0x54");
    assert_eq!(sim.list[i].impact_resume, state::AIM_STAND_FIRE, "the resume stamped into +0x118");
    assert!(sim.list[i].health < health, "the damage applied (5.0 raw, no +0x14C)");
    // Tick: the ENTER's re-run ran inside send (the damage applied once — 5.0, not 10.0); the
    // 0x47 stagger anim plays and its follow-on returns to the resume.
    let p = bond_at([pos[0] + 60.0, pos[1], pos[2]]);
    let mut returned = false;
    for _ in 0..240 {
        sim.tick(&d, &p);
        if sim.list[i].state == state::AIM_STAND_FIRE { returned = true; break; }
        assert!(matches!(sim.list[i].state, state::EXPLOSIVE_IMPACT),
            "the guard stays staggered until the anim completes, state {:#x}", sim.list[i].state);
    }
    assert!(returned, "the stagger exits back to the resume state");
}

/// A tased (down) guard hit by a blast dies the bullet way (the taser family's case 8/9 arm →
/// 0x44 [M 0x162eb0]), NOT DeathByExplosion — the family arm sits above the per-state table.
#[test]
fn blast_on_a_stunned_guard_dies_the_bullet_way() {
    let Some(l) = L::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let (mut sim, mut rng) = l.sim();
    let i = idx(&sim, 5);
    // Settle one tick first (the started ENTERs), then force the state.
    let p0 = bond_at([-500.0, 0.0, -500.0]);
    sim.tick(&d, &p0);
    sim.list[i].state = state::STUNNED;
    sim.list[i].last_damage = 5.0;
    sim.list[i].blast = Some(sim.list[i].pos);
    sim.list[i].send(&d, &mut rng, sim.frame, msg::EXPLOSION);
    assert_eq!(sim.list[i].state, state::DEATH_ANIM, "the taser family's case 9 → 0x44");
}
