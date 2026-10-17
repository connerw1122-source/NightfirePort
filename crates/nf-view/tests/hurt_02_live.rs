//! hurt-02: 07000002's `electrocute` volume — the level's one Hurt placement (0xE4) and the
//! electric-arc puzzle's damage half, driven end to end through the game's own chain.
//!
//! The data [D probe, agent-ctx census02.md]:
//! - Hurt #13 `electrocute` at (−60.99, 7.33, −30.16), the box under the arc over the cable gap
//!   above the secret bunker; params 0=1, 1=1, **2=0x8D(141) = the gate channel**, 3=0xffff (no
//!   sound), 4=1 (damage 1.0).
//! - Emitter #9 `ParticleEmmiter` (0xF2) at (−60.99, 7.62, −30.15): params **2=0x82 (on channel)**,
//!   3=0 (off), **4=0x8D (out — driven while particles live)**.
//! - Destroyable cable light #392 at the middle light: out channel **0x82**.
//! - Wire #187 + ThirdIcon #384 cross the gap through the arc.
//!
//! The chain [M]: smashing cable light #392 writes ch 0x82 [Destroy_CollisionHandler 0x12bd48:42];
//! Emitter #9 is enabled while its on-channel is ON [Emitter_Update 0x17b080:94, the spawn gates
//! :180/:259] — the arc starts on the smash; while particles live the emitter drives its out ch
//! 0x8D each tick [0x17b080:127 reset, :163 set]; the Hurt's gate channel 0x8D then lets
//! `Hurt_Update` 0x17e5d0 run and the contact damage flows through `Player_DealWithObjHit`
//! 0x194458:94-101 → `Player_HandlePain` (type 1..7 bypass armour [0x1902c8:84-104]). On a fresh
//! boot ch 0x82 is off, so the emitter is disabled and the Hurt parks with the disabled bit set
//! [Hurt_Create 0x17e870 tail].
//!
//! The test raises ch 0x82 exactly the way the game's destroyable does and lets the port's own
//! WorldFx emitter + hurt module carry the rest. The placement only REACHES the bunker-top gap
//! (the volume hangs over the open gap, so Bond is parked back into the centre band every tick);
//! the damage is the real tick path's (`Sim::step_bond`).

#[path = "common/walker.rs"]
mod walker;

use walker::Walker;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

#[test]
fn the_electrocute_volume_hurts_once_the_cable_light_smash_feeds_the_chain() {
    let scene = nf_view::load(&iso(), "07000002.bin", false).expect("load 07000002");
    let mut w = Walker::new(scene);
    // This test MEASURES damage — the walker's god top-up must stay off.
    w.top_up_health = false;
    {
        let sim = w.scene.sim.as_ref().unwrap();
        // [D] exactly one Hurt on the level, the bunker-arc `electrocute`.
        assert_eq!(sim.hurts.len(), 1, "the census: one 0xE4 placement (#13)");
        let h = &sim.hurts[0];
        assert_eq!(h.gate, 141, "the gate = param 2 = 0x8D [D #13; Emitter #9's out 0x8D]");
        assert!((h.lo[0] + h.hi[0]) * 0.5 - -60.99 < 3.0, "the box sits on the arc spot [D], lo/hi {:?}", (h.lo, h.hi));
        // [M Hurt_Create 0x17e870 tail] the gate channel is off on a fresh boot → the volume parks
        // with the disabled bit set; [M Emitter_Update 0x17b080:94] the emitter is gated off too.
        assert!(h.disabled, "fresh boot: ch 0x8D off → Hurt_Update's disabled bit set");
        assert!(!sim.drones.channels.on(141), "fresh boot: the emitter's out is off");
    }

    let (lo, hi) = {
        let h = &w.scene.sim.as_ref().unwrap().hurts[0];
        (h.lo, h.hi)
    };
    let cx = (lo[0] + hi[0]) * 0.5;
    let cz = (lo[2] + hi[2]) * 0.5;
    let mid_y = (lo[1] + hi[1]) * 0.5;
    // The volume hangs over the cable gap AND in the bunker chat-guard's dialogue range: the level's
    // own script holds the sim (`disabled`) while the chat plays. Every tick therefore parks Bond
    // back in the centre band, waits out any dialogue hold, then takes exactly one clean tick — the
    // placement only REACHES the spot; the damage is the real tick path's.
    let park = |w: &mut Walker, y: f32| {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = [cx, y, cz];
        sim.bond.prev_pos = sim.bond.pos;
        // The gap is open below: zero the accumulated fall so Bond stays in the box's band.
        sim.bond.vel = [0.0; 3];
        sim.bond.fall_speed = 0.0;
        sim.bond.grounded = false;
    };
    let tick_in_box = |w: &mut Walker, y: f32| {
        park(w, y);
        w.wait_enabled(600, "the bunker chat hold");
        w.wait(1);
    };

    // Sister — before the smash: the arc is off (the emitter's out never rises), so standing inside
    // the volume for 20 real ticks hurts nothing.
    for _ in 0..20 { tick_in_box(&mut w, mid_y); }
    let hp_off = w.scene.sim.as_ref().unwrap().drones.health.health;
    assert!((hp_off - 100.0).abs() < 0.01, "arc off: no contact damage [M 0x17e5d0 disabled bit], hp {hp_off}");

    // Smash cable light #392 — the death write ch 0x82 [M 0x12bd48:42] — and the chain runs:
    // Emitter #9 enables [M 0x17b080:94], its particles live, its out ch 0x8D rises [M 0x17b080:163].
    {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.drones.channels.set(0x82, 1, sim.drones.frame);
    }
    for _ in 0..10 { tick_in_box(&mut w, mid_y); }
    assert!(w.scene.sim.as_ref().unwrap().drones.channels.on(141),
        "the smash → emitter chain drives ch 0x8D [M 0x17b080:163]");

    // The arc burning: standing in the volume drains health through the real tick path (1.0 per
    // contact tick × the difficulty factor 0.5 [M Player_HandlePain 0x1902c8 + the 0x01..=0x04
    // damage group 0x1d3f60]).
    for _ in 0..20 { tick_in_box(&mut w, mid_y); }
    let hp_in = w.scene.sim.as_ref().unwrap().drones.health.health;
    assert!(hp_in < hp_off - 2.0, "the arc hurts: hp {hp_off} → {hp_in} [M 0x194458:94-101 + 0x17ea18]");

    // Out of the volume (the arc still burning): the drain stops — contact damage only.
    for _ in 0..20 {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = [cx + (hi[0] - lo[0]) + 2.5, mid_y, cz];
        sim.bond.prev_pos = sim.bond.pos;
        sim.bond.vel = [0.0; 3];
        sim.bond.fall_speed = 0.0;
        w.wait_enabled(600, "the bunker chat hold");
        w.wait(1);
    }
    let hp_out = w.scene.sim.as_ref().unwrap().drones.health.health;
    assert!((hp_out - hp_in).abs() < 0.5, "outside the box the volume stops [M the collide contact], hp {hp_in} → {hp_out}");
}
