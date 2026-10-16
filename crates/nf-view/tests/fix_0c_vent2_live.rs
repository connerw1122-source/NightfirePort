//! fix-0c-vent2: the display vent's remaining traversal legs (0700000c, the playtest's
//! "he can't crouch through"). 101g-fix-0c-hatch verified the drop through the open wall hole
//! onto the inner ledge; this session mapped the rest of the route and found the next blockers:
//!
//! 1. **Channel 0 was a dead channel in the port.** The level's pulley winches #351/#364/#377 —
//!    the level doc's "the vent route ... is the way down" — are SimpleScripts listening on
//!    **ch 0** [D #351/#364/#377: rate 75, ch 0, collision hashes 802d0092/9f/ac]. The game's
//!    `SS_Update` 0x1a9948:20-23 reads `switch_channels[ch]` and self-writes it while the channel
//!    VALUE is 0, for any channel word including 0; the trigger/AND writers copy Out into the
//!    array unconditionally [M 0x1b1a78]. The port's `Channels::set` early-returned on c == 0 and
//!    `Channels::on` hardcoded false — a ch-0 SS could never run. Fixed: ch 0 is a real channel.
//! 2. The verified legs, as a regression: from the duct top the crouched capsule drops through
//!    the open hole onto the balcony (14.61-14.81, z −112.5..−121.5) and walks it.
//!
//! Route maps (this session's probes, kept in `probe_vent.rs`): the balcony is continuous
//! x −2..14, z −112.5..−121.5 at 14.61-14.81 with no holes and no ceilings up to y 26; the lower
//! walkway (11.27) runs under its north half (z −112.4..−115.7, x −2..14); the ground (6.27)
//! with a rising ramp (6.5 → 7.7 southward) at the south-east (x 10..12, z −121.5..−123); the
//! space under the lower walkway at y 7.0 is open at x 7..9 from the south. OPEN: what drives
//! the ch-0 pulleys in the game (the SS_Update self-latch semantics), the crouched wedge at the
//! balcony's south wall (Bond bobs ungrounded over floor 14.61), and the ascent to the duct.
//!
//! Opt-in via `NF_ISO`.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load_at(pos: [f32; 3], yaw: f32) -> nf_view::scene::Scene {
    let iso = iso().expect("NF_ISO");
    let mut scene = nf_view::load(&iso, "0700000c.bin", false).expect("load 0700000c");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
    for _ in 0..100 { sim.step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_mut().unwrap();
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, pos, yaw);
    for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
    scene
}

fn burn_display_hinges(scene: &mut nf_view::scene::Scene) {
    let sim = scene.sim.as_mut().unwrap();
    for k in 0..sim.drones.locks.len() {
        if (14..=17).contains(&sim.drones.locks[k].out) {
            sim.drones.locks[k].burn(&mut sim.drones.channels, 1);
        }
    }
}

/// The fix: channel 0 is a real channel. `SS_Update` 0x1a9948:20-23 indexes
/// `switch_channels[ch]` for any ch, and the winches #351/#364/#377 listen on ch 0 [D].
#[test]
fn channel_0_is_a_real_channel() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut ch = nf_game::triggers::Channels::default();
    ch.set(0, 1, 7);
    assert!(ch.on(0), "a write to channel 0 must stick and read back [M 0x1a9948:20-23]");
    assert_eq!(ch.time[0], 7, "the nonzero-write time stamp covers ch 0 like every channel [M 0x1b1a78]");
    ch.set(0, 0, 9);
    assert!(!ch.on(0), "a zero write clears ch 0");
}

/// The winches: their SSs carry the channel word 0 [D #351/#364/#377] — which `SS_Operate`
/// 0x1a9df8:47-48 short-circuits as "no channel" (`== 0` -> return 0), and their mask 0x2 is the
/// bullet-presence bit (the obj+0xD0 contact chain, unported headlessly). grates-02 corrected the
/// old channel-seek model [M SS_Update 0x1a9948's state machine]: a ch-0 poke must NOT move them;
/// they hold key 0 until a bullet operates them in the real game.
#[test]
fn the_pulley_winches_hold_key0_a_channel_0_poke_moves_nothing() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_at([5.8, 15.8, -107.5], std::f32::consts::PI);
    let sim = scene.sim.as_mut().unwrap();
    let ss351 = sim.drones.sss.of_placement(351).expect("the pulley SS exists [D #351]");
    let ss364 = sim.drones.sss.of_placement(364).expect("the pulley SS exists [D #364]");
    let ss377 = sim.drones.sss.of_placement(377).expect("the pulley SS exists [D #377]");
    let (r351, r364, r377) = (
        sim.drones.sss.list[ss351].rate,
        sim.drones.sss.list[ss364].rate,
        sim.drones.sss.list[ss377].rate,
    );
    assert_eq!((r351, r364, r377), (75, 75, 75), "the winch SSs' rate word [D #351/#364/#377]");
    // Raise ch 0 and tick: the playheads must NOT move (the channel word 0 is no channel [M
    // 0x1a9df8:47-48]; state 0x40 + operate = 0 rests without re-posing [M 0x1a9948:85]).
    let frame = sim.drones.channels.time[18];
    sim.drones.channels.set(0, 1, frame + 1);
    assert!(sim.drones.channels.on(0), "ch 0 holds");
    let (m351, m364, m377) = (
        sim.drones.sss.list[ss351].matrix,
        sim.drones.sss.list[ss364].matrix,
        sim.drones.sss.list[ss377].matrix,
    );
    for _ in 0..40 { sim.step_bond(Input::default(), false, false, false); }
    let (t351, t364, t377) = (
        sim.drones.sss.list[ss351].t,
        sim.drones.sss.list[ss364].t,
        sim.drones.sss.list[ss377].t,
    );
    assert_eq!((t351, t364, t377), (0.0, 0.0, 0.0), "the ch-0 poke moves no winch playhead [M 0x1a9df8:47-48]");
    assert_eq!(sim.drones.sss.list[ss351].matrix, m351, "winch #351's pose holds");
    assert_eq!(sim.drones.sss.list[ss364].matrix, m364, "winch #364's pose holds");
    assert_eq!(sim.drones.sss.list[ss377].matrix, m377, "winch #377's pose holds");
}

/// The verified legs as a regression (101g's open-hole drop): from the duct top the crouched
/// capsule drops through the open hole onto the balcony and walks it — grounded, south along it.
#[test]
fn from_the_duct_top_the_hole_drops_bond_onto_the_balcony() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_at([5.8, 18.6, -110.8], std::f32::consts::PI);
    burn_display_hinges(&mut scene);
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_mut().unwrap();
    let ss138 = sim.drones.sss.of_placement(138).expect("ventmesh-front SS [D #138]");
    let last138 = (sim.drones.sss.list[ss138].keys.len() - 1) as u16;
    let open138 = nf_game::door::interp(&sim.drones.sss.list[ss138].keys, 0, last138, last138 as f32, sim.drones.sss.list[ss138].spline).unwrap();
    assert_eq!(sim.drones.sss.list[ss138].matrix, open138, "the hatch is fully open (the held pose is the last key's)");
    sim.step_bond(Input { crouch: true, ..Default::default() }, false, false, false);
    assert!(sim.bond.crouched, "the crouch key crouched Bond");
    // Crouch-walk forward off the duct edge, through the hole, onto the balcony.
    let mut landed = None;
    for tick in 1..=400 {
        let inp = if sim.bond.crouched { Input { forward: 1.0, ..Default::default() } } else { Input { forward: 1.0, crouch: true, ..Default::default() } };
        sim.step_bond(inp, false, false, false);
        let p = sim.bond.pos;
        if sim.bond.grounded && p[2] < -112.5 && p[1] < 16.0 { landed = Some((tick, p)); break; }
    }
    let (tick, p) = landed.expect("Bond lands grounded behind the wall (the balcony 14.61-14.81)");
    assert!(p[1] > 14.0 && p[1] < 16.0, "he lands on the balcony level, at y {:.2}", p[1]);
    assert!(p[2] > -122.0, "he lands on the balcony (z {:.2})", p[2]);
    // Sister: he keeps walking the balcony southward, still over the 14.61 floor.
    for _ in 0..120 { sim.step_bond(Input { forward: 1.0, ..Default::default() }, false, false, false); }
    let p2 = sim.bond.pos;
    assert!(p2[2] < p[2], "he walks south along the balcony (from z {:.2} to {:.2})", p[2], p2[2]);
    assert!(p2[1] > 13.5 && p2[1] < 16.5, "he stays on the balcony level, at y {:.2}", p2[1]);
    eprintln!("landed at tick {tick} on ({:.2}, {:.2}, {:.2}); walked to ({:.2}, {:.2}, {:.2})",
        p[0], p[1], p[2], p2[0], p2[1], p2[2]);
}
