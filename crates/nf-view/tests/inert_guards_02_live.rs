//! inert-guards-02 (694): the 07000002 drone census and the AI wake locks — the playtests'
//! "npcs that shouldnt be able to be shot and some of the enemies are invincible with no AI"
//! and "one specific guard … standing next to a civilian that won't move, no AI, doesn't do
//! anything". Opt-in via `NF_ISO`.
//!
//! The mechanism, from the decomp: every 07000002 guard carries a switch channel (placement
//! parameter 3, DCVars+0x134 [M NDrone2_DefaultInit 0x14b300 / the port's Drone::new]); a drone
//! whose channel is off sits in WAIT_SWITCH (0x01) with the AI off [M the port's state::WAIT_SWITCH
//! enter arm + NDrone2_WaitSwitch]. The level's intro cut-scene freezes everyone
//! (`Drone_EnableAll(0)` [M 0x1383e8] — the port's `DroneEnable{on:false}` request) and the
//! hand-back re-enables everyone EXCEPT the WaitSwitch drones [M 0x1383e8:14 — they stay disabled
//! until their own channel wakes them]. The wake writers are the level's Touch triggers (out
//! channel = the zone's guards' switch channel, [D] the 07000002 placement dump in the census):
//! the bunker exit pads set ch 21 (the garden guards d00-d04), the terrace pads ch 22/23/24, the
//! back-garden pad ch 25. Damageable (+0x14) is skin-keyed in the game [M DefaultInit 0x14b300:
//! only skins 0x0500003b/0x3c (the Kimiko guests) and 0x05000053 start with +0x14 = 0] — NONE of
//! 07000002's skins is on that list, so every 02 guard and civilian is shootable, exactly as the
//! game makes it.

use nf_view::scene::Sim;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load() -> Sim {
    let iso = iso().expect("NF_ISO");
    let mut scene = nf_view::load(&iso, "07000002.bin", false).expect("load 07000002");
    scene.sim.take().expect("sim")
}

/// One step through the real path; the level's scripts grab Bond into cut-scenes within seconds,
/// so each tick carries the game's own skip action like the walkreal walker [M nis.rs].
fn step(sim: &mut Sim, input: nf_game::player::Input) {
    if sim.disabled { sim.drones.scripts.skip_request = true; }
    sim.step_bond(input, false, false, false);
}

fn walk_ticks(sim: &mut Sim, n: u32) {
    let walk = nf_game::player::Input { forward: 1.0, ..Default::default() };
    for _ in 0..n { step(sim, walk); }
}

/// Teleport into a trigger's volume centre (the touch test reads the position against the volume,
/// `Trigger_Hit`'s contact-classification port) and settle.
fn teleport(sim: &mut Sim, pos: [f32; 3]) {
    sim.bond.pos = pos;
    sim.bond.prev_pos = pos;
}

const WAIT_SWITCH: u8 = 0x01;

/// The full census + the wake chain for the three named zones + the damage path, in one load (the
/// level load + the 600-tick walks are heavy; the assertions are ordered blocks).
#[test]
fn the_guards_wake_from_their_zone_pads_and_take_damage() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return; };
    let mut sim = load();

    // ── The start census ──────────────────────────────────────────────────────────────────────
    assert_eq!(sim.drones.list.len(), 25, "07000002's drone count changed — re-derive the census");
    for (i, dr) in sim.drones.list.iter().enumerate() {
        assert!(dr.present, "d{i} is not present at start");
        assert!(dr.damageable, "d{i} (skin {:#x}) starts non-damageable — the game only exempts 0x3b/0x3c/0x53 [M 0x14b300]", dr.skin);
        assert!(dr.health > 0.0, "d{i} starts dead");
    }
    // The guards' switch channels (placement param 3 [M +0x134]): the zones the census dumped.
    let swch = |p: usize| sim.drones.list.iter().find(|d| d.placement == p).unwrap().switch_channel;
    assert_eq!((swch(442), swch(443)), (21, 21), "guards 442/443 wait on ch 21 (the bunker pads t02/t10 [D placement 341/383])");
    assert_eq!((swch(447), swch(448)), (23, 23), "guards 447/448 wait on ch 23 (the terrace pads [D placement 342/343/424])");
    assert_eq!((swch(461), swch(462)), (8, 8), "the far-garden pair 461/462 waits on ch 8");

    // ── 600 ticks of the real walk: nobody turns invincible, nobody vanishes ─────────────────
    walk_ticks(&mut sim, 600);
    for (i, dr) in sim.drones.list.iter().enumerate() {
        assert!(dr.present, "d{i} vanished during the walk (present=false)");
        assert!(dr.damageable, "d{i} lost damageable during the walk");
        assert!(dr.health > 0.0, "d{i} lost health with no combat (health {})", dr.health);
    }
    // The dormancy IS the port's state machine: the zone-gated guards sit in WAIT_SWITCH
    // (enabled false after the intro's EnableAll(0)/(1) cycle, which skips WaitSwitch drones
    // [M 0x1383e8:14]) until their channel — not a missing update handler.
    for p in [442usize, 443, 444, 445, 446, 447, 448, 449] {
        let dr = sim.drones.list.iter().find(|d| d.placement == p).unwrap();
        assert_eq!(dr.state, WAIT_SWITCH, "guard {p} is not in WAIT_SWITCH after the intro (state {:#x}) — its gate broke", dr.state);
        assert!(!dr.enabled, "guard {p} runs AI while its switch channel is still off");
    }

    // ── The bunker exit pad (placement 341, out 21) wakes the garden guards ───────────────────
    // Volume x −25.7..−24.3, y 4.7..6.1, z 40.8..44.4 [D the census dump].
    teleport(&mut sim, [-25.0, 5.4, 42.5]);
    walk_ticks(&mut sim, 240);
    assert!(sim.drones.channels.on(21), "the bunker exit pad did not set channel 21");
    for p in [442usize, 443, 444, 445, 446] {
        let dr = sim.drones.list.iter().find(|d| d.placement == p).unwrap();
        assert!(dr.enabled, "guard {p} did not wake on its channel 21");
        assert_ne!(dr.state, WAIT_SWITCH, "guard {p} woke but stayed in WAIT_SWITCH (state {:#x})", dr.state);
    }
    // d00 (placement 442, the guard beside the bunker) actually runs AI: it noticed Bond.
    let d00 = sim.drones.list.iter().find(|d| d.placement == 442).unwrap();
    assert!(d00.alert >= 1.0 || d00.state >= 0x40,
        "the bunker guard 442 never reacted to Bond beside him (alert {}, state {:#x})", d00.alert, d00.state);
    // The other zones' guards correctly stay gated (their pads were not crossed).
    for p in [447usize, 448, 449, 450, 451] {
        let dr = sim.drones.list.iter().find(|d| d.placement == p).unwrap();
        assert!(!dr.enabled, "guard {p} woke without its zone pad (channel {})", dr.switch_channel);
    }

    // ── The terrace pad (placement 342, out 23) wakes the guard + civilian pair ───────────────
    // The playtest's "a guard standing next to a civilian that won't move": d05 (placement 447,
    // the terrace guard) and d23 (placement 465, the kind-8 civilian) stand 1.3 units apart, both
    // gated on ch 23 [D the census]. The pad's volume x −35.9..−34.5, y 5.8..7.2, z 3.3..6.9.
    teleport(&mut sim, [-35.2, 6.4, 5.0]);
    for _ in 0..240 { step(&mut sim, Default::default()); } // neutral: stay on the pad
    assert!(sim.drones.channels.on(23), "the terrace pad did not set channel 23");
    for p in [447usize, 448, 449, 465] {
        let dr = sim.drones.list.iter().find(|d| d.placement == p).unwrap();
        assert!(dr.enabled, "the terrace pair member {p} did not wake on channel 23");
        assert_ne!(dr.state, WAIT_SWITCH, "member {p} woke but stayed in WAIT_SWITCH");
    }

    // ── A woken guard takes bullet damage — nobody is invincible ─────────────────────────────
    // d05 (placement 447) is awake and 6+ units from Bond; a PlayerBullet through the real damage
    // path (player_bullets tick → hit → Damage) must land [M damage.rs's damageable gate].
    let si = sim.drones.list.iter().position(|d| d.placement == 447).unwrap();
    // Re-place Bond 5 units off the guard's current position at his own height (the woken guard
    // patrols — aim at the live box), clear of any cover between.
    let gp = sim.drones.list[si].pos;
    teleport(&mut sim, [gp[0] + 5.0, gp[1] + 0.2, gp[2]]);
    let from = [sim.bond.pos[0], sim.bond.pos[1] + 1.5, sim.bond.pos[2]];
    let hp0 = sim.drones.list[si].health;
    {
        let data = nf_game::drone::Data {
            tables: &sim.tables, props: &sim.props, scripts: &sim.scripts, weapons: &sim.weapons,
            world: &sim.world, level: sim.level, sequences: &sim.seqs, skins: &sim.skins,
            skeletons: &sim.skeletons, nav: &sim.nav, covers: &sim.covers, tuning: &sim.tuning,
            hitboxes: &sim.hitboxes, volumes: &sim.volumes, sfx: sim.sfx.as_ref(), models: &sim.models,
            surfaces: sim.surfaces.as_ref(), voices: sim.voices.as_deref(), difficulty: sim.drones.difficulty,
        };
        let boxes = sim.drones.list[si].hit_boxes(&data);
        let torso = boxes.iter().find(|b| b.part == 4).expect("the torso box");
        let chest = [torso.m[3][0], torso.m[3][1], torso.m[3][2]];
        eprintln!("DBG from {:?} chest {:?} seg {:?}", from, chest, sim.world.segment(from, chest));
        assert!(sim.world.segment(from, chest).is_none(), "the eye→torso ray is blocked — move the shot point");
        let dv = [chest[0] - from[0], chest[1] - from[1], chest[2] - from[2]];
        let l = (dv[0] * dv[0] + dv[1] * dv[1] + dv[2] * dv[2]).sqrt();
        let dir = [dv[0] / l, dv[1] / l, dv[2] / l];
        sim.drones.player_bullets.push(nf_game::drone::PlayerBullet {
            pos: from, dir, speed: 4.0, travelled: 0.0, range: 80.0, damage: 3.5, weapon: 2,
            first: true, dead: false, bounces: 0, homing: false, target: None, gravity: false, guided_cam: false, snd: 0,
        });
    }
    for _ in 0..40 { step(&mut sim, Default::default()); }
    assert!(sim.drones.list[si].health < hp0,
        "the awake terrace guard took no damage (health {} after the shot) — the invincible-enemy symptom",
        sim.drones.list[si].health);
}

/// The far-garden pair (placements 461/462) waits on channel 8, which NO 07000002 trigger, door,
/// inverter or Destroy object writes ([D] the census dump of all 36 triggers / the door ch_open
/// list / the inverters / the 0xE1 Destroy placements — their channels are 72, 0 and 130). The
/// game's remaining writer is the script event command 0xd [M Script_EventHandler 0x1d94c0:166
/// `switch_channels[ch] = value`], so the pair is LEGITIMATELY dormant until that script event —
/// the game holds them exactly the same way.
#[test]
fn the_far_garden_pair_is_legitimately_dormant_on_channel_8() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return; };
    let mut sim = load();
    walk_ticks(&mut sim, 600);
    for p in [461usize, 462] {
        let dr = sim.drones.list.iter().find(|d| d.placement == p).unwrap();
        assert_eq!(dr.switch_channel, 8, "the far-garden guard {p} changed its wake channel");
        assert_eq!(dr.state, WAIT_SWITCH, "guard {p} left WAIT_SWITCH with no channel-8 writer (state {:#x})", dr.state);
        assert!(!dr.enabled, "guard {p} runs AI while ch 8 is off");
        assert!(dr.present && dr.damageable && dr.health > 0.0,
            "guard {p} is not a normal, shootable, present guard");
    }
    assert!(!sim.drones.channels.on(8), "channel 8 turned on with no writer found in the port");
}
