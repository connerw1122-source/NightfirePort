//! tase-scream-broadcast (task 736, the uncited-gameplay audit runner-up): entering the tase
//! state broadcasts the scream — `NDrone2_DSTATE_Taser` 0x162b98's case-1 tail [M, disasm
//! 0x162d0c-0x162d40]: prop 0x32 set, the once-per-life bit clear (+0x4F8 & 8 — the bit the
//! hurt scream shares, `DroneFunc_SendHurtMessage` 0x145e28), the active type id not 2 (+0xC5),
//! then the msg-0x12 broadcast `NDrone2_DroneAlertToPosition` 0x14a498 from the victim's
//! position. The port screamed only on bullets and punches. Opt-in via NF_ISO (the slim ISO is
//! enough; the real input path — the Stunner 0x4a equipped, aimed, held on the guard).

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn dxz(a: [f32; 3], b: [f32; 3]) -> f32 { ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt() }

/// The stunner setup from fix_stunner_press_live, narrowed to a guard that can scream: prop 0x32
/// set and the active type id not 2 [M 0x162d0c-0x162d40 — the game's own gates].
fn setup_stunner_on_guard(scene: &mut nf_view::scene::Scene) -> usize {
    let (gi, screams) = {
        let sim = scene.sim.as_ref().unwrap();
        let d = sim.data();
        match sim.drones.llist.iter().find(|&&i| {
            let dr = &sim.drones.list[i];
            dr.present && dr.enabled && !dr.corpse && dr.health > 0.0 && dr.dtype != 2
                && dr.prop(&d, 0x32)
                && (dr.pos[0] - sim.bond.pos[0]).powi(2) + (dr.pos[2] - sim.bond.pos[2]).powi(2) < 400.0
        }) {
            Some(&i) => (i, true),
            None => (0, false),
        }
    };
    assert!(screams, "no prop-0x32 guard within 20 u of Bond's start on 07000005 — the scream arm cannot pin here");
    let gpos = scene.sim.as_ref().unwrap().drones.list[gi].pos;
    let mut dev = nf_view::dev::Dev { scenario: None, ..Default::default() };
    dev.teleport(scene, [gpos[0] - 2.5, gpos[1], gpos[2]], 0.0);
    let draw = nf_game::player::Input { switch_to: Some(0x4a), ..Default::default() };
    let mut ok = false;
    for _ in 0..60 {
        scene.sim.as_mut().unwrap().step_bond(draw, false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == 0x4a { ok = true; break; }
    }
    assert!(ok, "the Stunner (0x4a) did not equip");
    for _ in 0..70 { scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input::default(), false, false, false); }
    for _ in 0..60 {
        let (turn, look) = {
            let sim = scene.sim.as_ref().unwrap();
            let bp = sim.bond.pos;
            let g = sim.drones.list[gi].pos;
            let dx = g[0] - bp[0];
            let dz = g[2] - bp[2];
            let dy = g[1] - (bp[1] + sim.bond.eye - sim.bond.crouch_drop);
            let desired = dx.atan2(dz);
            let err = desired - sim.bond.yaw;
            let want_pitch = (dy / dxz(bp, g).max(0.5)).atan() / std::f32::consts::FRAC_PI_2;
            (err.clamp(-0.08, 0.08), (want_pitch - sim.bond.pitch).clamp(-0.08, 0.08))
        };
        scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { turn, look, ..Default::default() }, false, false, false);
    }
    gi
}

fn steer_and_fire(scene: &mut nf_view::scene::Scene, gi: usize, fire: bool) {
    let (turn, look) = {
        let sim = scene.sim.as_ref().unwrap();
        let bp = sim.bond.pos;
        let g = sim.drones.list[gi].pos;
        let dx = g[0] - bp[0];
        let dz = g[2] - bp[2];
        let dy = g[1] - (bp[1] + sim.bond.eye - sim.bond.crouch_drop);
        let desired = dx.atan2(dz);
        let err = desired - sim.bond.yaw;
        let want_pitch = (dy / dxz(bp, g).max(0.5)).atan() / std::f32::consts::FRAC_PI_2;
        (err.clamp(-0.08, 0.08), (want_pitch - sim.bond.pitch).clamp(-0.08, 0.08))
    };
    scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { turn, look, ..Default::default() }, fire, false, false);
}

/// The tase ENTER screams: the fall's first tick sets the once-per-life bit (`+0x4F8 |= 8` [M
/// 0x162b98 case-1 tail]) — the port's `screamed` — and a re-tase (the held beam re-falls him
/// from STUNNED [M 0x162eb0 case 7 → 0x4a], re-running the TASER ENTER and its scream arm) does
/// not reset it: the hurt scream and the tase scream share the once-per-life bit [M
/// DroneFunc_SendHurtMessage 0x145e28:11-13/32 + 0x162b98 case 1].
#[test]
fn the_tase_enter_screams_and_only_once() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let gi = setup_stunner_on_guard(&mut scene);

    // Hold the tase until the guard falls into DSTATE_Taser 0x4a [M 0x1469b8 case 7].
    let mut fell = None;
    for t in 0..400u32 {
        steer_and_fire(&mut scene, gi, true);
        if scene.sim.as_ref().unwrap().drones.list[gi].state == nf_game::drone::state::TASER { fell = Some(t); break; }
    }
    fell.expect("the guard falls into DSTATE_Taser 0x4a under a held tase");
    // The ENTER ran: the scream arm set the once bit [M 0x162b98:58-72]. (Fail-first: without the
    // tase arm `screamed` stays false here — only the bullet/punch hurt path sets it, and this
    // guard was never shot or punched.)
    assert!(scene.sim.as_ref().unwrap().drones.list[gi].screamed,
        "the tase ENTER screams (prop 0x32, once bit set) [M 0x162b98:58-72]");

    // Release: the fall wait expires into STUNNED 0x4b [M 0x162b98 case 0xc — the held beam
    // re-arms +30 every tick (case 7), so the wait only runs while released]. Then hold again:
    // the STUNNED msg 7 re-falls him into TASER [M 0x162eb0 case 7] — the TASER ENTER (and its
    // scream arm) runs a SECOND time.
    let mut cycled = false;
    for _ in 0..90u32 {
        steer_and_fire(&mut scene, gi, false);
        if scene.sim.as_ref().unwrap().drones.list[gi].state == nf_game::drone::state::STUNNED { cycled = true; break; }
    }
    assert!(cycled, "the released fall wait reached STUNNED [M 0x162b98 case 0xc]");
    let mut re_fell = false;
    for _ in 0..90u32 {
        steer_and_fire(&mut scene, gi, true);
        if scene.sim.as_ref().unwrap().drones.list[gi].state == nf_game::drone::state::TASER { re_fell = true; break; }
    }
    assert!(re_fell, "the held beam re-fell him from STUNNED [M 0x162eb0 case 7] — the ENTER re-ran");
    // The once-per-life bit holds across the second ENTER (no reset, no second queue — the
    // hurt-scream share [M 0x145e28 + 0x162b98 both gate on +0x4F8 & 8]).
    assert!(scene.sim.as_ref().unwrap().drones.list[gi].screamed,
        "the once bit survives the second tase ENTER [M 0x162d24-0x162d2c]");
}
