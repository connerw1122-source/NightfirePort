//! Playtest 1016 (and 5+ earlier reports, with screenshots): "the room culling fucking up the player model" —
//! the first-person hands/gun vanish. `Scene::skin_palette` sent every non-guard character to the FAR
//! matrix when the room at its position was not in the frame's visibility mask; the view model is posed
//! just in front of the camera, so in a doorway or at a room edge its point fell in an unmarked cel and
//! the whole model collapsed. The game draws the view model in its own list-2 pass after the world, never
//! through a cel's object list [M Game_Draw: the view-model draw after psiLight_SetLights(0);
//! View_SetDrawInNoViews is the only hide]. Opt-in via NF_ISO.

use nf_view::scene::{Scene, ViewOpts};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

const FAR_Y: f32 = -10000.0;

/// A hand-built palette layout covering every character (the GPU renderer builds the real one; the
/// layout is plain data — scene.rs `SkinLayout`). `skin_palette` breaks at the first character past
/// `char_base.len()` — the dev spawn's added character — which is exactly the "missing from the
/// frame's palette" candidate this helper detects.
fn full_layout(scene: &Scene) -> nf_view::scene::SkinLayout {
    let mut char_base = Vec::with_capacity(scene.characters.len());
    let mut b = 0u32;
    for c in &scene.characters { char_base.push(b); b += c.skeleton.bones as u32 + 1; }
    let fs = scene.frame_state(None, 0.0, ViewOpts::default());
    let attach_base = b;
    let door_base = attach_base + fs.attach.len() as u32;
    let (doors, picks) = scene.sim.as_ref().map(|s| (s.drones.doors.list.len() + s.drones.sss.list.len(), s.drones.pickups.len())).unwrap_or((0, 0));
    let pick_base = door_base + doors as u32;
    // The colour table: cols is sized col_one + 1; the writers reach col_att + attachments,
    // col_pick + pickups, col_sl + searchlights and the per-character mult indices.
    let parts = scene.sim.as_ref().map(|s| s.drones.searchlights.parts.len()).unwrap_or(0);
    let attaches = fs.attach.len();
    let col_one = (scene.characters.len()).max(attaches + picks + parts) as u32 + 1;
    nf_view::scene::SkinLayout {
        char_base,
        attach_base,
        sl_base: door_base,
        door_base,
        pick_base,
        n_mats: pick_base + picks as u32,
        col_att: 0,
        col_pick: attaches as u32,
        col_sl: (attaches + picks) as u32,
        col_one,
    }
}

#[test]
fn the_view_model_is_never_room_culled() {
    let Some(iso) = iso() else { return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    for _ in 0..90 { scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input::default(), false, false, false); }
    let l = full_layout(&scene);
    let weapon = scene.sim.as_ref().unwrap().gun.weapon;
    let vm = *scene.view_models.get(&weapon).expect("the held weapon has a view model");
    // The worst case of a doorway: every room hidden in the frame's mask.
    let none_visible = vec![false; scene.cels.len().max(1)];
    let (pal, _) = scene.skin_palette(&l, None, 0.0, ViewOpts::default(), Some(&none_visible));
    let base = l.char_base[vm] as usize;
    assert!(pal[base][3][1] != FAR_Y, "the view model (character {vm}) was sent FAR by the room mask");
    // Sister: the room test still works — some non-guard, non-view-model character in a hidden room is FAR.
    let culled_other = (0..scene.characters.len()).filter(|&ci| ci != vm && !scene.view_models.values().any(|&v| v == ci))
        .any(|ci| l.char_base.get(ci).is_some_and(|&b| pal[b as usize][3][1] == FAR_Y));
    assert!(culled_other, "no other character was room-culled with every room hidden (the test lost its teeth)");
}
