//! view-model-unlit: the first-person view model takes no point lights. `Game_Draw` 0x1e6f50 draws it in
//! the list-2 pass after `psiLight_SetLights(0, …)` zeroed the slots, and an animated object's draw
//! (`AnimObjectDraw` 0x1c4490) never re-uploads them [M View_DrawObjList 0x1e69a0:75-98]. The port
//! tags the view model's draws with `VIEW_MODEL_NODE`, and the renderer zeroes their light slots.
//! Opt-in: set `NF_ISO` to the disc image.
use nf_view::scene::{Layer, VIEW_MODEL_NODE};

#[test]
fn the_view_model_draws_are_tagged_and_nothing_else_is() {
    unsafe { std::env::set_var("NF_SILENT", "1"); }
    let Ok(iso) = std::env::var("NF_ISO") else { return };
    let scene = nf_view::load_weapons(&iso, "07000008.bin", false, &[0x2d]).expect("load 08");
    let tagged: Vec<_> = scene.materials.iter().filter(|m| m.node == VIEW_MODEL_NODE).collect();
    assert!(!tagged.is_empty(), "the view model's draws carry VIEW_MODEL_NODE");
    assert!(tagged.iter().all(|m| m.layer != Layer::World), "no world draw is tagged (node drives World cel culling)");
    // Sister: the level's other characters (guards, Bond's third-person body) stay untagged and keep their lights.
    let other_chars = scene.materials.iter().filter(|m| m.layer == Layer::Character && m.node != VIEW_MODEL_NODE).count();
    assert!(other_chars > 0, "the non-view-model characters keep untagged draws");
}
