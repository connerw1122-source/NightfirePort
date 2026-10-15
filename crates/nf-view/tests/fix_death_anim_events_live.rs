//! fix-death-anim-events: the death script 0x06000151's op-4 events on Bond (task 110i).
//!
//! Evidence base:
//! - `Player_ChangeSubState` 0x190630 case 0xd: `AnimScriptAdd(obj, 0x6000151)` (the gravity
//!   global is −9.4 in retail, so the 0x60008e0 arm never plays [M 0x1c7e78:73, 0x1910f8:153]).
//!   The script (nf-extract 07000005.bin 06000151 — the tool prints args as hex): one sequence
//!   window (frames 1..297, sequence 0x040000a3) + four op-4 commands: frame 5 kind 5, frame 8
//!   kind 4, frame 0x98=152 kind 4, frame 0x116=278 kind 6.
//! - The dead-tick clock: the script's +0x90 reads 12, 32, 52 at dead frames 8, 18, 28 [L PCSX2
//!   trace] — 2.0 frames per dead tick (speed 1.0 [M AnimScriptInit 0x1c1600] × FRAME_RATE_MUL
//!   2.0, `GS_SetRefreshRate` 0x17d2b8:15). Events fire in the (from, to] windows the clock
//!   passes; the death tick ran the empty just-started window (1, 1], tick 2 covers (1, 3], so
//!   frame 5 lands on dead tick 3, frame 8 on tick 5, 152 on tick 77 ((151, 153]), 278 on tick 140
//!   ((277, 279]) — the clock reads 2·tick−1.
//! - The effects for the DEAD player: the death marker set obj+0xff = 0x12 [M 0x190810:72], so
//!   `AnimObjectUpdate` 0x1befd8 is not the class-3 arm — the script's `ScriptFootstepType` sets
//!   the anim object's bit 4 and calls `Effect_Create(1.0, animobj, Bond, 0x42 | foot)` [M
//!   :184-199] → `Effect_Player` 0x1cec48's 0x40 arm: the surface's footstep sound
//!   (EffectInfo+0x16) at the feet contact, `Sound_SetAlertness(0)` [M 0x1de210] — not heard. Kind
//!   6 → `Effect_Create(…, 0xc0)` [M 0x1c1b38 case 6] → `Effect_Body` 0x1cf400: the surface's body
//!   sound (EffectInfo+0x18), `Sound_ModAlertness(0.5)` [M 0x1de220]. The contact's material is
//!   the feet ray's hit `Player_FeetOnPoint` 0x194188:183-191 copies into the anim object.
//! - The contact material under Bond's 07000005 spawn and the two columns' ids are computed by the
//!   test through the same world/surface tables the sim uses.
//!
//! Opt-in via `NF_ISO`.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str) -> nf_view::scene::Scene {
    nf_view::load(iso, "07000005.bin", false).expect("load")
}

fn tick(sim: &mut nf_view::scene::Sim, n: u32) {
    for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
}

/// Flushes the audio pool (the render side's `audio.update`) and returns the ids started since
/// the last flush — a node only becomes a `Cmd::Start` there [M Sounds::update], so a sound the
/// sim created this tick surfaces in the flush right after the tick (the fix_bond_damage_live
/// precedent).
fn flush_started(sim: &mut nf_view::scene::Sim) -> Vec<(u32, [f32; 3])> {
    let listener = (sim.bond.pos, [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]);
    sim.drones.audio.update(None, listener, &|_| true);
    let ids = sim.drones.audio.out.iter().filter_map(|c| match c {
        nf_game::sound::Cmd::Start { id, pos: Some(p), .. } => Some((*id, *p)),
        _ => None,
    }).collect();
    sim.drones.audio.out.clear();
    ids
}

/// `id` started within `tol` of Bond's feet — the death events play at the feet contact, so a
/// nearby guard's own footstep on the same material must not count.
fn started_at(ids: &[(u32, [f32; 3])], id: u32, at: [f32; 3], tol: f32) -> bool {
    ids.iter().any(|&(i, p)| i == id && (p[0] - at[0]).powi(2) + (p[1] - at[1]).powi(2) + (p[2] - at[2]).powi(2) < tol * tol)
}

/// The floor material + contact point under Bond (the port's read of the game's feet-contact
/// shadow, `Player_FeetOnPoint` 0x194188:183-191).
fn floor_under(sim: &nf_view::scene::Sim) -> Option<(u8, [f32; 3])> {
    let p = sim.bond.pos;
    let top = [p[0], p[1] - 0.1, p[2]];
    let bottom = [p[0], p[1] - 1.6, p[2]];
    sim.world.segment_surface(top, bottom, true).map(|(t, _, m)| {
        (m, [top[0] + (bottom[0] - top[0]) * t, top[1] + (bottom[1] - top[1]) * t, top[2] + (bottom[2] - top[2]) * t])
    })
}

/// Each op-4 command of the death script fires exactly once, on the dead tick the 2.0-frame clock
/// passes its frame (frame 5 → dead tick 3 [L PCSX2 +0x90 rate]), with the kind/foot/sound the
/// game's chain gives. Sister: nothing fires while alive.
#[test]
fn death_events_fire_once_at_the_clock_ticks() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    let sim = scene.sim.as_mut().unwrap();
    tick(sim, 10);
    // The death script and the sound ids its events map to at Bond's floor material.
    assert!(sim.scripts.contains_key(&0x0600_0151), "the death script 06000151 is in 07000005 [D]");
    let (mat, _) = floor_under(sim).expect("Bond stands on a surface");
    assert!(mat != 0, "the contact material is non-zero (Effect_Create's guard [M 0x1ce688])");
    let surfaces = sim.surfaces.as_ref().expect("the surface table");
    let footstep_id = nf_formats::sfx::SurfaceSounds::get(&surfaces.footstep, mat);
    let body_id = nf_formats::sfx::SurfaceSounds::get(&surfaces.body, mat);
    // SISTER: alive — a non-fatal hit starts nothing.
    let (h, r) = (&mut sim.drones.health, &mut sim.drones.rng); h.pain_generic(r, 1.0, 1, 0x0700_0005, 0);
    tick(sim, 5);
    assert!(!sim.drones.death.active && sim.drones.death.events.is_empty(), "alive: no death, no events");
    // The kill (the task's path: health 0, then step_bond) and the first 145 dead ticks — frame
    // 278 (0x116) lands on dead tick 140.
    sim.drones.health.health = 0.0;
    tick(sim, 145);
    let ev = &sim.drones.death.events;
    assert_eq!(ev.len(), 4, "each of the script's four op-4 commands fired exactly once");
    assert_eq!(ev[0], nf_game::death::DeathEvent { frame: 5, kind: 5, dead_tick: 3, sound: footstep_id, foot: 8 },
        "frame 5 (kind 5) on dead tick 3, the footstep sound at the contact material");
    assert_eq!(ev[1], nf_game::death::DeathEvent { frame: 8, kind: 4, dead_tick: 5, sound: footstep_id, foot: 4 },
        "frame 8 (kind 4) in the window (7, 9] — dead tick 5, the other foot");
    assert_eq!(ev[2], nf_game::death::DeathEvent { frame: 152, kind: 4, dead_tick: 77, sound: footstep_id, foot: 4 },
        "frame 0x98=152 (kind 4) in the window (151, 153] — dead tick 77");
    assert_eq!(ev[3], nf_game::death::DeathEvent { frame: 278, kind: 6, dead_tick: 140, sound: body_id, foot: 0 },
        "frame 0x116=278 (kind 6, the body effect) in the window (277, 279] — dead tick 140");
}

/// The sounds the events queue, tick by tick through the real input path: the footstep sound on
/// dead ticks 3 / 5 / 77 and the body sound on 140, quiet in between and after, nothing while
/// alive. (The footsteps play at SetAlertness 0 — audible, not heard; the body sound halves its
/// alertness, `Sound_ModAlertness(0.5)` [M 0x1de220].)
#[test]
fn death_event_sounds_queue_on_their_ticks() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    let sim = scene.sim.as_mut().unwrap();
    tick(sim, 10);
    let (mat, _) = floor_under(sim).expect("Bond stands on a surface");
    let surfaces = sim.surfaces.as_ref().expect("the surface table");
    let footstep_id = nf_formats::sfx::SurfaceSounds::get(&surfaces.footstep, mat).unwrap_or(0xffff);
    let body_id = nf_formats::sfx::SurfaceSounds::get(&surfaces.body, mat).unwrap_or(0xffff);
    assert!(footstep_id != 0xffff && body_id != 0xffff, "Bond's floor material has both columns");
    let at = floor_under(sim).expect("contact").1;
    // Alive: neither sound appears in a plain tick.
    tick(sim, 3);
    let ids = flush_started(sim);
    assert!(!started_at(&ids, footstep_id, at, 1.5) && !started_at(&ids, body_id, at, 1.5), "alive: no death-event sounds");
    // The kill. Dead tick 1 (the death tick): the just-started window (1, 1] — quiet.
    sim.drones.health.health = 0.0;
    tick(sim, 1);
    let ids = flush_started(sim);
    assert!(!started_at(&ids, footstep_id, at, 1.5), "dead tick 1: no event");
    tick(sim, 1); // dead tick 2: (1, 3] — quiet.
    let ids = flush_started(sim);
    assert!(!started_at(&ids, footstep_id, at, 1.5), "dead tick 2: no event");
    tick(sim, 1); // dead tick 3: (3, 5] → frame 5.
    let ids = flush_started(sim);
    assert!(started_at(&ids, footstep_id, at, 1.5), "dead tick 3: frame 5's footstep sound {footstep_id} queued at the contact");
    tick(sim, 1); // dead tick 4: (5, 7] — quiet.
    let ids = flush_started(sim);
    assert!(!started_at(&ids, footstep_id, at, 1.5), "dead tick 4: no event");
    tick(sim, 1); // dead tick 5: (7, 9] → frame 8.
    let ids = flush_started(sim);
    assert!(started_at(&ids, footstep_id, at, 1.5), "dead tick 5: frame 8's footstep sound queued");
    tick(sim, 71); // dead ticks 6..=76: quiet until frame 152's window.
    let ids = flush_started(sim);
    assert!(!started_at(&ids, footstep_id, at, 1.5) && !started_at(&ids, body_id, at, 1.5), "ticks 6..=76: no events");
    tick(sim, 1); // dead tick 77: (151, 153] → frame 152.
    let ids = flush_started(sim);
    assert!(started_at(&ids, footstep_id, at, 1.5), "dead tick 77: frame 152's footstep sound queued");
    tick(sim, 62); // dead ticks 78..=139: quiet until frame 278's window.
    let ids = flush_started(sim);
    assert!(!started_at(&ids, footstep_id, at, 1.5) && !started_at(&ids, body_id, at, 1.5), "ticks 78..=139: no events");
    tick(sim, 1); // dead tick 140: (277, 279] → frame 278, kind 6 — the body sound.
    let ids = flush_started(sim);
    assert!(started_at(&ids, body_id, at, 1.5), "dead tick 140: frame 278's body sound {body_id} queued at the contact");
    assert!(!started_at(&ids, footstep_id, at, 1.5), "dead tick 140: no footstep");
    tick(sim, 5); // past the script's last command: nothing re-fires.
    let ids = flush_started(sim);
    assert!(!started_at(&ids, body_id, at, 1.5) && !started_at(&ids, footstep_id, at, 1.5), "past frame 278: quiet");
}
