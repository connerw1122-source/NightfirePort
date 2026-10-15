//! fix-hat-details — Oddjob's hat (0x45): the wall-contact sound and the in-flight cap, through the
//! REAL input path (`nf_view::load_weapons` + `Sim::step_bond`), playtest 1014-1725.
//!
//! The user: "It makes a weird sound when it hits a wall. I'm not sure if that's accurate to the
//! game. And also, if you throw enough of them, you stop seeing them flying in the air."
//!
//! What the disc says:
//! - The wall contact [M `Bullet_CollisionHandler` 0x12ce80 — the +0x100 sticky arm, disasm
//!   0x12d43c-0x12d470]: the hat's arm writes 1500.0 into BU+0xFC and calls
//!   `Sound_Play3D(0x4a7, obj+0x30, f12=96.0, f13=-1.0)` — the SAME clang the satchel 0x3b plays.
//!   The per-material impact sound does NOT join it: `Effect_Create`'s flags for the hat are 0
//!   (the 0x61 needs `weapon_data+0x70 & 2` — the hat's +0x70 is 0x100 [L ee_start_05.bin];
//!   `+0x6c & 0x100` and `+0x70 & 0x1000` are clear), so `Effect_Bullet` 0x1ce8a8:86-92's
//!   `flags & 0x40` arm (the `EffectInfo[material&0x3f]+0x14` sound) never runs. The bounce
//!   clang 0x51c needs the bounce block (LAB_0012d4bc) which a +0x100 world hit never reaches.
//!   So the game's hat-on-wall = exactly ONE 0x4a7 clang — this test pins it.
//! - The in-flight cap [M `View_AddObjects` 0x1e62d8 — the per-view draw list]:
//!   `viewer+8+ring*4` buffer, `viewer+0x14+ring*2` cursor, `viewer+0x1a+ring*2` size, and the
//!   add is `entry = buf + cursor*0xc; cursor = (cursor + 1) & (size - 1)` — a RING: when a
//!   frame's adds pass the size the OLDEST-ADDED entries are reused. [L ee_snapshot.bin]
//!   viewer[0] @0x1fc6c10: sizes 512/512/128, viewer[5] @0x1fc69b0 the same — the per-view
//!   draw list holds 512 entries (ring 2 = 128 for the +0xf0 & 0x2000 billboards; a flying
//!   bullet has both bits clear [M `Bullet_update` state 0 clears 0x10/0x2000 at 0x12e7b0] so
//!   it rides ring 0). The cel lists are newest-first [M `control_link_object_to_cel` 0x1346b8
//!   prepends], so a full ring overwrites the OLDEST objects. Every flying bullet object with a
//!   +0x5C glist draws through it [M `Bullet_update` state 0 → `hashtable_set_object_to_entity_gfx`
//!   0x12e8f0-0x12e900] — there is no projectile count limit below the ring.
//!   The port's draw pools the projectiles into GRENADE_SLOTS static slots (pre-fix 8) and draws
//!   only `Sim::drones.grenades[0..8]` — hats 9+ are INVISIBLE while they fly (the user's
//!   symptom: the stuck hats occupy the early slots for their 1500-tick life [M 0x12e7b0:363-371],
//!   so every later throw is unseen). The fix sizes the pool to the game's draw list (512,
//!   oldest-ADDED-first overwrite — the port's Vec order is spawn order, so the newest 512 draw,
//!   matching the game's newest-first cel add).

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn q(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

/// Load pre-merged for the weapon, give it through the panel (the user's path), wait for the draw.
fn load_weapon(iso: &str, id: u8, rounds: i16) -> nf_view::scene::Scene {
    let mut scene = nf_view::load_weapons(iso, "07000005.bin", false, &[id]).expect("load 05");
    let (msg, switch_to) = nf_view::panel::apply(&mut scene, &q(&[("do", "weapon"), ("id", &id.to_string()), ("rounds", &rounds.to_string())]));
    eprintln!("[give] {id:#04x} rounds={rounds}: {msg} -> {switch_to:?}");
    assert_eq!(switch_to, Some(id), "the panel must switch to {id:#04x}");
    for _ in 0..180 {
        scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(id), ..Default::default() }, false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == id && scene.sim.as_ref().unwrap().gun.switch_state == 0 { break; }
    }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, id, "the weapon {id:#04x} is in hand");
    for _ in 0..4 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    scene
}

/// Sound ids the audio driver was asked to start this tick (`Cmd::Start` — the same batch the
/// windowed loop builds, matrix.rs's `sample`), with the start position.
fn started_sounds(scene: &mut nf_view::scene::Scene) -> Vec<(u32, [f32; 3])> {
    let sim = scene.sim.as_mut().unwrap();
    let (pos, f) = (sim.bond.head(), sim.bond.look());
    let norm = { let l = (f[0] * f[0] + f[2] * f[2]).sqrt().max(1e-6); [f[2] / l, 0.0, -f[0] / l] };
    sim.drones.map_sounds.update(&mut sim.drones.audio, pos);
    sim.drones.audio.update(sim.sfx.as_ref(), (pos, norm, [0.0, 1.0, 0.0], f), &|_| false);
    sim.drones.audio.take_commands().into_iter()
        .filter_map(|c| match c { nf_game::sound::Cmd::Start { id, pos: Some(p), .. } if id != 0 => Some((id, p)), _ => None })
        .collect()
}

fn park_guards_behind(scene: &mut nf_view::scene::Scene) {
    let sim = scene.sim.as_mut().unwrap();
    let bp = sim.bond.pos;
    for (i, dr) in sim.drones.list.iter_mut().enumerate() {
        if !dr.present { continue; }
        dr.pos = [bp[0] + 2.0 + i as f32 * 0.5, dr.pos[1], bp[2] - 30.0];
    }
}

/// The hat's projectile mesh (weapon_data[0x45]+0x5C) as the draw resolves it.
fn hat_mesh(scene: &nf_view::scene::Scene) -> u32 {
    scene.sim.as_ref().map(|s| s.weapons.projectile_mesh(0x45)).unwrap_or(0)
}

/// ── The wall contact: exactly ONE 0x4a7 clang, nothing else — no bounce 0x51c, no per-material
/// impact sound ────
/// Game [M]: the +0x100 stick arm plays 0x4a7 (disasm 0x12d43c-0x12d470: `oriu $a0, $zero, 0x4a7`
/// → `jal Sound_Play3D` 0x1dd548 with f12 = 96.0/0x42c80000, f13 = −1.0/0xbf800000); the material
/// sound arm (`Effect_Bullet` 0x1ce8a8:86-92) is gated on effect flags 0x40 and the hat's
/// `Effect_Create` flags are 0 (no +0x70 & 2, no +0x6c & 0x100, no +0x70 & 0x1000 [M 0x12ce80]);
/// the bounce block never runs for a +0x100 world hit (LAB_0012d4bc's gate needs +0x70 & 0x80 or
/// a char hit). So the ONLY sound the game plays at the wall is the 0x4a7 clang.
#[test]
fn the_hats_wall_contact_is_the_single_game_clang() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_weapon(&iso, 0x45, 200);
    park_guards_behind(&mut scene);
    // A shallow throw: the hat lands 3+ units ahead, so the contact point is far enough from
    // Bond's camera that the throw-anim's own sounds (the gun script's 0x41 whoosh at the camera,
    // soundlog `gun script 0600097e frame 15`) stay out of the contact window.
    scene.sim.as_mut().unwrap().bond.pitch = -0.45;
    let mut thrown = false;
    for _ in 0..120 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), true, false, false);
        if !scene.sim.as_ref().unwrap().drones.grenades.is_empty() { thrown = true; break; }
    }
    assert!(thrown, "the hat never threw");
    let mut all: Vec<(u32, u32, [f32; 3])> = Vec::new(); // (tick, id, pos)
    let mut stick_tick = None;
    let mut stick_pos = [0.0f32; 3];
    for t in 0..300u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        for (id, p) in started_sounds(&mut scene) { all.push((t, id, p)); }
        let sim = scene.sim.as_ref().unwrap();
        if let Some(g) = sim.drones.grenades.first() {
            if g.stuck && stick_tick.is_none() { stick_tick = Some(t); stick_pos = g.pos; }
        }
        if stick_tick.is_some() && t > stick_tick.unwrap() + 30 { break; }
    }
    // The sounds AT the hat (the contact point) — the throw whoosh plays at Bond's camera, a
    // guard's voice 30 units behind: neither is the contact.
    let at_hat = |p: [f32; 3]| (p[0] - stick_pos[0]).powi(2) + (p[1] - stick_pos[1]).powi(2) + (p[2] - stick_pos[2]).powi(2) < 1.5f32.powi(2);
    let impact: Vec<u32> = all.iter()
        .filter(|(t, _, p)| stick_tick.map_or(false, |st| *t >= st) && at_hat(*p))
        .map(|(_, id, _)| *id).collect();
    eprintln!("[sounds] stick_tick={stick_tick:?} at {stick_pos:?} impact sounds = {impact:?}");
    eprintln!("[sounds] full sequence = {:?}", all.iter().map(|(t, id, _)| (*t, *id)).collect::<Vec<_>>());
    assert!(stick_tick.is_some(), "the hat never stuck");
    let clangs = impact.iter().filter(|&&id| id == 0x4a7).count();
    assert_eq!(clangs, 1, "the wall contact played {clangs} 0x4a7 clangs (the game plays exactly one)");
    let extras: Vec<u32> = impact.iter().copied().filter(|&id| id != 0x4a7).collect();
    assert!(extras.is_empty(), "extra sounds at the wall contact: {extras:?} (the game plays only the 0x4a7 clang — no 0x51c bounce, no EffectInfo material sound)");
}

/// ── Enough hats: every hat in flight still draws (the game's 512-entry per-view draw list) ────
/// The game draws EVERY flying bullet with a +0x5C glist [M `Bullet_update` state 0
/// 0x12e8f0-0x12e900 → `View_SetDrawInAllViews`]; the per-view draw list is a 512-entry ring with
/// oldest-ADDED-first reuse [M `View_AddObjects` 0x1e62d8 `cursor = (cursor+1) & (size-1)`;
/// [L ee_snapshot.bin viewer[0] sizes 512/512/128] — a count no hat spree reaches. The port's
/// pre-fix draw pool showed only the FIRST 8 grenades (`GRENADE_SLOTS = 8`): with the stuck hats
/// holding slots for their 1500-tick life, hat 9+ flew INVISIBLE — the user's "if you throw
/// enough of them, you stop seeing them flying in the air".
#[test]
fn every_thrown_hat_still_draws_while_it_flies() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_weapon(&iso, 0x45, 200);
    park_guards_behind(&mut scene);
    let mesh = hat_mesh(&scene);
    assert_ne!(mesh, 0, "the hat's +0x5C mesh did not resolve");
    // The playtest ran `nf-view <iso> <level> --dev`, whose dev panel pins the CURRENT weapon's
    // clip at its `weapon_data+0x92` size every tick (`dev.rs`'s infinite_ammo arm — on by
    // default: `dev.then(|| Dev { infinite_ammo: true, .. })` in lib.rs's App build). Without
    // the pin each hat spends the record's +0x91 = 100 [M 0x1a5460:654 — the fire's
    // `RoundToFire(+0x91, +0x91)`] and the reload gate starves the spree; the user's session
    // threw at the record's 26-tick interval (+0x40 = 0x1a [L ee_start_05.bin @0x2c398c]). The
    // pin lines below are the dev arm's own (clip = max(clip, clip_size) on the inventory slot
    // and the live gun).
    scene.sim.as_mut().unwrap().bond.pitch = 0.9;
    let mut thrown = 0usize;
    let mut pulse = 0u32;
    for _ in 0..1200u32 {
        {
            let sim = scene.sim.as_mut().unwrap();
            let wi = 0x45usize;
            let cs = sim.weapons.clip_size(wi);
            if cs > 0 {
                if let Some(c) = sim.drones.inventory.clip.get_mut(wi) { *c = (*c).max(cs); }
                sim.gun.clip = sim.gun.clip.max(cs as u32);
            }
        }
        let fire = pulse < 3;
        pulse = pulse.wrapping_add(1);
        if pulse >= 55 { pulse = 0; }
        let before = scene.sim.as_ref().unwrap().drones.grenades.len();
        scene.sim.as_mut().unwrap().step_bond(Input::default(), fire, false, false);
        let after = scene.sim.as_ref().unwrap().drones.grenades.len();
        if after > before {
            thrown += after - before;
            if thrown >= 12 { break; }
        }
    }
    let sim = scene.sim.as_ref().unwrap();
    let live = sim.drones.grenades.iter().filter(|g| g.state < 2).count();
    eprintln!("[throw] thrown={thrown} live_in_sim={live}");
    assert!(thrown >= 12, "only {thrown} hats threw");
    assert!(live > 8, "only {live} hats live — the spree must pass the pre-fix 8-slot pool to be a test");
    // The draw: every live hat's position must appear on a hat-mesh projectile attachment.
    // (`FrameState.hidden` is per CHARACTER — the projectile slots self-hide by collapsing their
    // matrix far below the world, so the y > −1000 check is the visibility test.)
    let fs = scene.frame_state(None, 0.0, Default::default());
    let mut drawn: Vec<[f32; 3]> = Vec::new();
    for (k, a) in scene.attachments.iter().enumerate() {
        if let Some((_, m)) = a.projectile {
            if m == mesh {
                let t = fs.attach[k];
                if t[3][1] > -1000.0 { drawn.push([t[3][0], t[3][1], t[3][2]]); }
            }
        }
    }
    eprintln!("[draw] hat-mesh instances drawing: {}", drawn.len());
    let mut missing = 0usize;
    for g in sim.drones.grenades.iter().filter(|g| g.state < 2) {
        let hit = drawn.iter().any(|p| {
            (p[0] - g.pos[0]).powi(2) + (p[1] - g.pos[1]).powi(2) + (p[2] - g.pos[2]).powi(2) < 0.5f32.powi(2)
        });
        if !hit { missing += 1; eprintln!("[draw] UNDRAWN hat at {:?} (stuck={})", g.pos, g.stuck); }
    }
    assert_eq!(missing, 0, "{missing} live hat(s) did not draw (the pre-fix 8-slot pool drops hats 9+)");
}
