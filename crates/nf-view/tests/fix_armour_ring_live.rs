//! fix-armour-ring: the health pane's armour state — the bottom-left ring's 8 armour segments
//! (`HealthSprInfo[0..8]`, template tint 0x7d6d59 over the sheet's orange-red arc art) appear when
//! `BLData+0x8B0 > 0` and hide at armour 0 [M `HUD_UpdateHealthPane` 0x19b9c8:139-158: `armour*0.16
//! == 0.0` → every segment's +0x38 = 0xFF (hidden); else the template layer with alpha 1.0 for
//! segment 0, 0.75 while `k < armour*0.16`, 0.35 above]. The health ring (segments 8..14, neutral
//! art tinted the health colour green 0x52a88b / orange 0xc6984e / red 0x9d1912 at runtime, plus
//! the 87×86 centre-disc sprite 15 [M 0x19b9c8:159-207]) keeps its health colour in both states.
//!
//! That answers 100n-fix-context-hints2's open item: the "orange 007 ring" in the PCSX2 captures is
//! THIS pane with the capture god-cheat's armour 50 (tools/trace/cheats.py writes health 100 +
//! armour 50 to BLData+0x894/0x8B0 every frame) — the ring is the pane's own art (tex 0x0300007A),
//! not a separate 007-logo draw. There is no extra logo sprite inside the ring: sprite 15's alpha
//! plane is a hollow circle outline [L tex 0x0300007A uv (102,113,85,86)], and the ring's centre
//! stays empty; the armour segments' art is the orange-red band the captures show.
//!
//! The overlay is a pure function of the Sim's state, so the tests set the state (through the real
//! pickup take path, and through the same write cheats.py's god makes) and read the quads.

use nf_game::pickup::Taken;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The health pane's ring quads from the HUD sheet 0x0300007A on the LEFT half (x centre < 256 of
/// 512×448) whose tint is `rgb` (the game's gs() half-range: channel/128), as (x0, y0, x1, y1,
/// alpha). Two collisions are filtered: the ammo digits share the tint (AmmoSprInfo[0] = 0x7d6d59)
/// bottom-right, and a pickup message draws the bar from the same sheet (uv u ≈ 185–196 texels
/// [M HUD_UpdateStatusPane 0x19c310:340-346]) on the left — the ring's arc art lives in the sheet's
/// u < 128 columns, so the run's texture + u0 pin the ring's own quads.
fn pane_quads(scene: &nf_view::scene::Scene, f: &nf_view::gpu::HudFrame, rgb: [f32; 3]) -> Vec<[f32; 5]> {
    let Some(ti) = scene.hud_textures.iter().position(|(h, _)| *h == 0x0300_007a) else { return Vec::new() };
    let mut out = Vec::new();
    for r in f.runs.iter().filter(|r| r.0 == ti + 1) {
        for q in f.v[r.2 as usize..(r.2 + r.3) as usize].chunks_exact(6) {
            let (a, b) = (q[0], q[2]);
            let c = a.c;
            if a.uv[0] > 0.5 { continue; } // the message bar's columns (u ≈ 185–196 texels) are not ring art
            if (c[0] - rgb[0]).abs() > 1e-4 || (c[1] - rgb[1]).abs() > 1e-4 || (c[2] - rgb[2]).abs() > 1e-4 || c[3] <= 0.0 { continue; }
            let (x0, y0, x1, y1) = ((a.p[0] + 1.0) * 256.0, (1.0 - a.p[1]) * 224.0, (b.p[0] + 1.0) * 256.0, (1.0 - b.p[1]) * 224.0);
            if (x0 + x1) / 2.0 >= 256.0 { continue; } // the ammo digits are bottom-right
            out.push([x0, y0, x1, y1, c[3]]);
        }
    }
    out.sort_by(|p, q| p[0].total_cmp(&q[0]).then(p[1].total_cmp(&q[1])));
    out
}

/// gs()'s values for the pane's colours/alphas (hud.rs `gs`).
const BRONZE: [f32; 3] = [125.0 / 128.0, 109.0 / 128.0, 89.0 / 128.0]; // HealthSprInfo[0..8] colour 0x7d6d59
const GREEN: [f32; 3] = [82.0 / 128.0, 168.0 / 128.0, 139.0 / 128.0]; // the health colour 0x52a88b (health ≥ 57)
const A_FULL: f32 = 127.0 / 128.0; // gs(_, 255) — segment 0 at armour > 0
const A_LIT: f32 = 95.0 / 128.0; // gs(_, 191) — 0.75·255
const A_DIM: f32 = 44.0 / 128.0; // gs(_, 89) — 0.35·255

/// The level starts without armour: the 8 armour segments are hidden (layer 0xFF) and the ring is
/// the green health state — 7 health segments + the centre disc, all the health colour.
#[test]
fn at_armour_zero_the_armour_segments_hide_and_the_ring_stays_green() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    {
        let s = scene.sim.as_ref().unwrap();
        assert_eq!(s.drones.health.armour, 0.0, "the level starts without armour");
        assert_eq!(s.drones.health.health, 100.0, "the level starts at full health (the green state)");
    }
    let f = nf_view::hud::overlay(&scene, 4.0 / 3.0);
    let bronze = pane_quads(&scene, &f, BRONZE);
    assert!(bronze.is_empty(), "armour 0 hides all eight armour segments [M 0x19b9c8:140-141]; drew {bronze:?}");
    let green = pane_quads(&scene, &f, GREEN);
    assert_eq!(green.len(), 8, "the green ring = 7 health segments (8..14) + the centre disc (15)");
    // The centre disc: HealthSprInfo[15] = 87×86 at pane (4,4) → screen (20,341)..(107,427) — the
    // ring's hollow outline (the alpha plane is a circle band [L the sheet's uv (102,113,85,86)]).
    let disc = green.iter().find(|q| q[0] < 25.0 && q[1] < 345.0 && q[2] > 100.0 && q[3] > 420.0);
    assert!(disc.is_some(), "the centre disc draws in the health colour: {green:?}");
}

/// The real take path: stand on the level's kind-3 pickup (Pickup_Handler 0x18e650 case 3 →
/// `Inventory::take` hard-sets `BLData+0x8B0` = 50.0 [M pickup.rs]) and the pane gains the 8 armour
/// segments — segment 0 at alpha 1.0, the rest 0.75 (armour 50 → all under `armour*0.16 = 8.0`).
/// Sister: the health ring keeps its green — armour adds segments, it does not recolour the ring.
#[test]
fn taking_an_armour_pickup_lights_the_eight_armour_segments() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let spot = {
        let sim = scene.sim.as_ref().unwrap();
        let (k, _) = sim.drones.pickups.iter().enumerate()
            .find(|(_, p)| p.kind == nf_game::pickup::kind::ARMOUR && !p.gone)
            .expect("07000005 places a kind-3 armour pickup");
        let m = sim.placed_pickup_pose(k).expect("the armour has a placed instance");
        [m[3][0], m[3][1], m[3][2]]
    };
    // The touch check reaches the pickup from a small ring around it (fix-hud's take pattern).
    let mut taken: Option<Taken> = None;
    for off in [[0.0f32, 1.2, -0.6], [0.0, 1.2, 0.6], [0.6, 1.2, 0.0], [-0.6, 1.2, 0.0], [0.0, 0.9, 0.0]] {
        {
            let sim = scene.sim.as_mut().unwrap();
            sim.bond = nf_game::player::Bond::stand_at(&sim.world, [spot[0] + off[0], spot[1] + off[1], spot[2] + off[2]], 0.0);
            sim.hud.taken_seen = sim.drones.taken.len();
        }
        // The scene's HUD update consumes `drones.taken` within the same tick (taken_seen syncs,
        // scene.rs), so the take's observable is the game effect itself: the armour write.
        for _ in 0..60 {
            let sim = scene.sim.as_mut().unwrap();
            sim.step_bond(Default::default(), false, false, false);
            if sim.drones.health.armour >= 50.0 { taken = sim.drones.taken.last().map(|(_, t)| t.clone()); break; }
        }
        if taken.is_some() { break; }
    }
    assert!(matches!(taken, Some(Taken::Armour)), "took the armour pickup: {taken:?}");
    {
        let s = scene.sim.as_ref().unwrap();
        assert_eq!(s.drones.health.armour, 50.0, "the armour take hard-sets 50.0 [M 0x18e650 case 3]");
    }
    let f = nf_view::hud::overlay(&scene, 4.0 / 3.0);
    let bronze = pane_quads(&scene, &f, BRONZE);
    assert_eq!(bronze.len(), 8, "armour 50 lights all eight armour segments [M 0x19b9c8:143-156]");
    let mut a: Vec<f32> = bronze.iter().map(|q| q[4]).collect();
    a.sort_by(f32::total_cmp);
    assert_eq!(a, vec![A_LIT, A_LIT, A_LIT, A_LIT, A_LIT, A_LIT, A_LIT, A_FULL],
        "segment 0 at 1.0, the rest 0.75 (all k < 50·0.16 = 8.0) [M 0x19b9c8:144-150]");
    // Segment 0's pin: HealthSprInfo[0] at pane (32,30) 32×32 → screen (48,367)..(80,399).
    let seg0 = bronze.iter().find(|q| (q[0] - 48.0).abs() < 0.6 && (q[1] - 367.0).abs() < 0.6);
    assert!(seg0.is_some(), "segment 0 draws at (48,367): {bronze:?}");
    // Sister: the health ring is unchanged by the armour state.
    assert_eq!(pane_quads(&scene, &f, GREEN).len(), 8, "the green ring keeps its 7 segments + disc");
}

/// Sister on the value tracking: armour 25 (the same write cheats.py's god makes, `BLData+0x8B0`)
/// lights k 1..3 at 0.75 and leaves k 4..7 at 0.35 (`25·0.16 = 4.0`), and armour 0 through the same
/// write hides the segments again (the game's `armour*0.16 == 0.0` test [M 0x19b9c8:140]).
#[test]
fn the_armour_segments_track_the_value() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    scene.sim.as_mut().unwrap().drones.health.armour = 25.0;
    let f = nf_view::hud::overlay(&scene, 4.0 / 3.0);
    let bronze = pane_quads(&scene, &f, BRONZE);
    assert_eq!(bronze.len(), 8, "armour 25 draws all eight (one disc each, two alpha tiers)");
    let mut a: Vec<f32> = bronze.iter().map(|q| q[4]).collect();
    a.sort_by(f32::total_cmp);
    assert_eq!(a, vec![A_DIM, A_DIM, A_DIM, A_DIM, A_LIT, A_LIT, A_LIT, A_FULL],
        "k 1..3 lit at 0.75, k 4..7 dim at 0.35 (25·0.16 = 4.0) [M 0x19b9c8:144-150]");
    scene.sim.as_mut().unwrap().drones.health.armour = 0.0;
    let f = nf_view::hud::overlay(&scene, 4.0 / 3.0);
    assert!(pane_quads(&scene, &f, BRONZE).is_empty(), "the same write back to 0 hides the segments again");
}
