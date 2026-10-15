//! fix-aims20-lights-render: the AIMS-20's four side lights — the idle all-off gate is
//! RECORD-DEPENDENT, through the real input path (`Sim::step_bond` with the `Input` the window
//! builds).
//!
//! The game, measured in PCSX2 (docs/live/2026-10-14-weapon-measurements-pcsx2.md, "The side
//! lights"): the four round lights are **lit red from the moment the gun is drawn and STAY LIT
//! WHILE IDLE** — the port darkened the whole row after ~0.75 s with no stick input in BOTH
//! modes, which is what the main agent's render showed.
//!
//! The code [M 0x1a5460 disasm 0x1a5bf4-68, read this time in the DISASSEMBLY not the C]: the
//! all-off gate's `sVar5` chain at the C's lines 305-314 is a decompiler artifact. The BNEL at
//! 0x1a5bf8 (held record id != 0x1a → jump 0x1a5c40, the `FRAME_RATE * 45.0 <` compare) EXECUTES
//! its delay slot `lh $v0, 0x2f0($s2)`, so the compare reads **clip[0x1A]** (the burst slot,
//! BL+0x2f0) when the held record is 0x1b — and the idle counter +0x93a (the reload at 0x1a5c3c)
//! only when it is 0x1a. The disc's burst mag is 30 [L weapon_data 0x1a +0x92], so in grenade
//! mode the gate can never fire: the row stays lit while idle. Burst mode keeps the 0.75 s
//! dead-still all-off (the port already had that half; 109d's live check 3 asked for this
//! re-read).
//!
//! Sister facts held fixed: bone 8's blink gate reads +0x93a in BOTH records (the load at
//! 0x1a5ad8 precedes any record branch), and the whole aims block runs on the record's BASE
//! weapon (+2) == 0x1a [M 0x1a5a94-9c] — 0x1b's base is 0x1a, so the lights attach in grenade
//! mode too.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str) -> nf_view::scene::Scene {
    nf_view::load_weapons(iso, "07000005.bin", false, &[0x1a, 0x1b]).expect("load 05 with the AIMS pair")
}

/// Equip one side of the AIMS pair and let the draw machine settle (the way the dev give does:
/// `Player_EquipWeapon` 0x1a8a58 owned + a full clip, then the switch request).
fn draw_side(scene: &mut nf_view::scene::Scene, id: usize) {
    {
        let sim = scene.sim.as_mut().unwrap();
        let wd = sim.weapons.clone();
        let rounds = wd.clip_size(id).max(1);
        if sim.drones.inventory.equip_weapon(&wd, id, rounds, true)
            || sim.drones.inventory.owned.get(id).copied().unwrap_or(false) {
            sim.gun.switch_request = id as u8;
        }
    }
    for _ in 0..200 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon as usize, id, "the AIMS side is drawn");
}

/// The aims lights of the held view model with their frame_state matrices.
fn aims_rows(scene: &nf_view::scene::Scene, wid: u8) -> Vec<(u8, [[f32; 4]; 4])> {
    let vm = *scene.view_models.get(&wid).expect("the AIMS view model");
    let fs = scene.frame_state(None, 0.0, Default::default());
    scene.attachments.iter().zip(fs.attach.iter())
        .filter(|(a, _)| a.aims_light.is_some() && a.character as usize == vm)
        .map(|(a, m)| (a.aims_light.unwrap(), *m)).collect()
}

/// A collapsed matrix lost its rotation (the port collapses a hidden mesh to a point by zeroing
/// the 3×3, `Scene::frame_state`'s `collapse`).
fn collapsed(m: [[f32; 4]; 4]) -> bool { m[0][0] == 0.0 && m[1][1] == 0.0 }

/// Grenade mode (0x1b held): dead-still idle for 60+ ticks must NOT darken the row — the 45
/// all-off gate reads the burst slot's rounds there (≤30 on disc), not the idle counter
/// [M 0x1a5460 disasm 0x1a5bf4-68]. Before the fix the port read +0x93a in both modes (the C's
/// line-312 artifact) and collapsed all four lights 0.75 s into any idle — the measured
/// "staying lit while idle" contradicted it.
#[test]
fn grenade_mode_keeps_the_row_lit_while_dead_still() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    // The grenade side drawn, its own slot loaded (4 rounds — the armoury pickup's count) and the
    // BURST slot filled to the disc mag (30) — the most binding case for the gate: the idle counter
    // climbs past 45 while clip[0x1A] = 30 < 45, so only the burst-clip read keeps the row lit.
    {
        let sim = scene.sim.as_mut().unwrap();
        let wd = sim.weapons.clone();
        sim.drones.inventory.equip_ammo(&wd, 0x1b, 4);
        sim.drones.inventory.equip_ammo(&wd, 0x1a, 30);
    }
    draw_side(&mut scene, 0x1b_usize);
    // Dead-still: no input at all. The idle counter climbs (the analog reset needs a deflected
    // stick [M 0x17ee88:80-84]) — in burst that would cross the 45 gate; in grenade the gate
    // reads clip[0x1A] instead.
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
        let s = sim;
        assert!(s.drones.aims_idle > 45, "the idle counter climbed past the burst gate's 45");
        assert_eq!(s.gun.weapon, 0x1b, "still holding the grenade side");
        assert_eq!(s.drones.inventory.clip.get(0x1a).copied().unwrap_or(0), 30,
            "the burst slot holds its full 30-round mag [L weapon_data 0x1a +0x92] — 45 < 30 is false");
    }
    let rows = aims_rows(&scene, 0x1b);
    assert_eq!(rows.len(), 4, "bones 8..0xB each carry one light [M 0x1a5460:259-266]");
    // Bone 8 blinks while idle > 15 in BOTH records (the +0x93a load at 0x1a5ad8 precedes any
    // record branch) — lit or blink-dark per the 20/40 duty phase; 9/10/11 must all stay lit.
    for (bone, m) in &rows {
        if *bone == 8 { continue; }
        assert!(!collapsed(*m), "grenade mode dead-still: light bone {bone} stays lit (the 45 gate reads clip[0x1A]=30, not the idle counter) [M 0x1a5460 disasm 0x1a5bf4-68]");
    }
    // Sister: the same 60 dead-still ticks in BURST mode (0x1a held) DO darken the row — the
    // gate reads +0x93a there [M 0x1a5c3c], the half the port already had.
    draw_side(&mut scene, 0x1a_usize);
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
        assert!(sim.drones.aims_idle > 45, "burst dead-still climbs the counter");
    }
    let rows = aims_rows(&scene, 0x1a);
    for (bone, m) in &rows {
        assert!(collapsed(*m), "burst mode dead-still: light bone {bone} goes dark after 45 idle ticks [M 0x1a5c3c-68]");
    }
}
