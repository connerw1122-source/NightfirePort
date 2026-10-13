//! fix-render-artifacts (045i): the user's 2026-10-12 playtest render-artifact items, pinned headlessly.
//!
//! What this file pins (NF_ISO, real disc data through `nf_view::load`):
//! 1. The subtractive baked-shadow blend lead: ALPHA_1 0x7F00000042 = (2,0,0,1) = Cd − Cs·As maps to the
//!    port's blend 2 (wgpu ReverseSubtract, factors SrcAlpha/One — the exact GS equation) and the packed
//!    shape occurs on the disc (models/world.md's 312 writes). Verified on HEAD: the mapping was never
//!    additive/normal, so the "shadows brighten because the subtractive shape is mis-mapped" hypothesis
//!    does not hold on this branch — these tests keep it from regressing.
//! 2. The three shadow-decal objects the user's screenshots show (0700000c `pylonshadow`, 07000012
//!    `SHADOW_Map`/`Office-Z_shadowmaps`) parse with the game's own state [D disc data]: ALPHA_1 0x44
//!    normal, dark vertex rgb, shape in texture alpha — the GS/ported math darkens the destination for
//!    both. The port's material state matches the disc state; if a future fix changes what the game does
//!    at runtime (the VU1 shadow path), these pins make the disc-state baseline explicit.
//! 3. The cull-hole exclusion: from the user's screenshot spots, no in-frustum World static is dropped by
//!    the renderer's sphere or cel-box culls — the "sky shows through cull holes" theory is excluded on
//!    HEAD (the pale-blue fog colour would otherwise fill every hole).

use nf_formats::{disc, iso9660::IsoImage, level::Level};
use nf_view::scene::{Draw, Layer, Scene};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn parse_level(iso_path: &str, archive: &str) -> Level {
    let img = IsoImage::open(iso_path).expect("open ISO");
    let (entries, bin) = disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(archive)).expect("archive entry");
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read archive");
    Level::parse(&data).expect("parse level")
}

/// The lead's packed subtractive shape and the other measured disc shapes decode to the right blends
/// (docs/models/world.md §3 table), and blend 2 is the GS equation (0−Cs)·As+Cd exactly.
#[test]
fn the_subtractive_shadow_blend_maps_to_the_exact_gs_equation() {
    // (ALPHA_1, expected blend) — the measured table, models/world.md:83-89.
    for (alpha, want, name) in [
        (0x44u64, 0u8, "normal (Cs−Cd)·As+Cd"),
        (0x7F00000048, 1, "additive (Cs−0)·As+Cd"),
        (0x7F00000042, 2, "subtractive (0−Cs)·As+Cd — the shadow shape"),
        (0x58, 4, "env (Cs−0)·Ad+Cd"),
    ] {
        let d = Draw::from_gs(1, 1, alpha, 0x5001b, 0x3100_0000);
        assert_eq!(d.blend, want, "ALPHA_1 {alpha:#x} must decode to blend {want} ({name})");
    }
    // Sister: the port's blend-2 semantics are the GS equation — ReverseSubtract with (SrcAlpha, One) is
    // dst − src·srcAlpha = Cd − Cs·As (gpu.rs `make_pipeline` blend 2; pinned by its own shader text).
    let src = include_str!("../src/gpu.rs");
    let blend2 = src.split("2 => wgpu::BlendState").nth(1).unwrap().split("3 =>").next().unwrap();
    assert!(blend2.contains("ReverseSubtract") && blend2.contains("SrcAlpha") && blend2.contains("BlendFactor::One"),
        "world blend 2 must stay ReverseSubtract with factors (SrcAlpha, One) = Cd − Cs·As");
}

/// The two shadow-decal textures the user's artifacts draw (0700000c `pylonshadow` tex 002d:003b,
/// 07000012 `SHADOW_Map` tex 0026:005b) and their object state on the disc.
#[test]
fn the_artifact_decal_objects_parse_with_their_disc_state() {
    let Some(iso_path) = iso() else { eprintln!("NF_ISO not set — skipping"); return };

    // 0700000c: `pylonshadow` (placement type 0x19, creates a cel) — the "pale crane" spot's decal.
    let level = parse_level(&iso_path, "0700000c.bin");
    let m = level.main_map().unwrap();
    let pylon = level.maps[m].placements.iter().enumerate()
        .filter_map(|(pi, p)| {
            let t = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() } else { Some((m, p.index as usize)) };
            t.filter(|&(mo, oi)| level.maps[mo].objects[oi].name.trim() == "pylonshadow").map(|t| (pi, t))
        })
        .collect::<Vec<_>>();
    assert_eq!(pylon.len(), 1, "0700000c places exactly one pylonshadow");
    let (pi, (map, obj)) = pylon[0];
    let o = &level.maps[map].objects[obj];
    let p = &level.maps[m].placements[pi];
    assert!(p.creates_cel(), "pylonshadow is a runtime cel (its primary glist draws in list 0)");
    assert_eq!(p.flags & 0xff, 0x19);
    // The leaf: ALPHA_1 0x44 normal, ZMSK set (no depth write is 0x131000000's bit 32), the shadow shape in
    // texture alpha, the vertex rgb black — the game's darkening equation (Cs−Cd)·As+Cd with Cs = tex·vc = 0.
    let leaf = o.gfx.leaves.iter().find(|l| !l.batches.is_empty()).expect("pylonshadow leaf");
    assert_eq!(leaf.gs.iter().find(|(r, _, _)| *r == 0x42).map(|(_, v, _)| *v), Some(0x44), "pylonshadow ALPHA_1 is 0x44 normal on the disc");
    let batch = &leaf.batches[0];
    let vc = batch.verts[0].rgba;
    assert!(vc[0] == 0 && vc[1] == 0 && vc[2] == 0 && vc[3] > 0, "pylonshadow vertex colour is black rgb with shaped alpha, got {vc:?}");
    let tex = level.maps[map].textures.iter().find(|t| t.index as u32 == batch.texture.unwrap());
    let tex = tex.expect("pylonshadow texture");
    let pal = tex.palette_rgba8();
    let rgba: Vec<u8> = tex.frame(0).iter().flat_map(|&i| pal[i as usize]).collect();
    let n = (rgba.len() / 4).max(1);
    let mean_r = rgba.chunks(4).map(|px| px[0] as u32).sum::<u32>() / n as u32;
    assert!(mean_r > 128, "the pylon decal's texture is WHITE-rgb with the lattice shape in alpha (mean rgb {mean_r}) — the black vertex colour is what darkens it, not the texture");

    // 07000012: `SHADOW_Map` — the corridor "cross" spot's floating decal quads.
    let level = parse_level(&iso_path, "07000012.bin");
    let m = level.main_map().unwrap();
    let shmap = level.maps[m].placements.iter().enumerate()
        .filter_map(|(pi, p)| {
            let t = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() } else { Some((m, p.index as usize)) };
            t.filter(|&(mo, oi)| level.maps[mo].objects[oi].name.trim() == "SHADOW_Map").map(|t| (pi, t))
        })
        .collect::<Vec<_>>();
    assert!(!shmap.is_empty() && shmap.len() <= 2, "07000012 places SHADOW_Map (got {})", shmap.len());
    // The corridor instance sits ~1.6 units up — the "floating quads" of the screenshot.
    let (pi, (map, obj)) = *shmap.iter().find(|&&(pi, _)| {
        let t = &level.maps[m].placements[pi].translation;
        t[1] > 1.0 && t[0] > -6.0 && t[0] < -1.0 && t[2] > 15.0 && t[2] < 30.0
    }).expect("SHADOW_Map floats at (~-3.2, 1.6, ~21.4) in the corridor");
    let t = &level.maps[m].placements[pi].translation;
    assert!((t[1] - 1.6).abs() < 0.1, "SHADOW_Map floats at ({:.2},{:.2},{:.2}) in the corridor", t[0], t[1], t[2]);
    let o = &level.maps[map].objects[obj];
    let leaf = o.gfx.leaves.iter().find(|l| !l.batches.is_empty()).expect("SHADOW_Map leaf");
    assert_eq!(leaf.gs.iter().find(|(r, _, _)| *r == 0x42).map(|(_, v, _)| *v), Some(0x44), "SHADOW_Map ALPHA_1 is 0x44 normal on the disc");
    for b in &leaf.batches {
        for v in &b.verts {
            assert!(v.rgba[0] <= 6 && v.rgba[1] <= 6 && v.rgba[2] <= 6 || v.rgba[3] == 0,
                "SHADOW_Map vertex rgb stays near-black (got {:?}) — the game's decal darkens through the vertex colour", v.rgba);
        }
    }
}

/// The A-11e sphere cull must be SOUND: every World material's bounding radius must cover its own drawn
/// vertices (max |v − centre|). The glist radius undershoots the drawn geometry on the disc (0700000c's
/// floor decal at (28.54, 0.33, −182.49): glist r 0.6, drawn verts reach 0.644) — an under-covering sphere
/// lets `sphere_outside` cull draws that still have visible geometry at the frustum edge (the user's
/// pale fog-coloured patches appear where the port skips what the game draws). The game's own cull
/// consults the bbox next to the sphere (`View_AddObjects` 0x1e62d8 passes obj+0x8c AND obj+0x80 to
/// `Vision_InView` 0x1e89e0). Fail-first: pre-fix this reads 0.6 < 0.644 on the disc.
#[test]
fn the_world_cull_sphere_covers_its_drawn_vertices() {
    let Some(iso_path) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for archive in ["0700000c.bin", "07000012.bin"] {
        let scene = nf_view::load(&iso_path, archive, false).expect("load");
        let mut worst: Option<(usize, f32, f32)> = None;
        for (mi, m) in scene.materials.iter().enumerate() {
            if m.layer != Layer::World { continue }
            let mut d2max = (m.radius * m.radius).max(0.0);
            for i in scene.indices[m.first_index as usize..(m.first_index + m.index_count) as usize].iter() {
                let p = &scene.vertices[*i as usize].pos;
                let d2 = (p[0] - m.centre[0]).powi(2) + (p[1] - m.centre[1]).powi(2) + (p[2] - m.centre[2]).powi(2);
                if d2.is_finite() && d2 > d2max { d2max = d2; }
            }
            let need = d2max.sqrt();
            if need > m.radius + 1e-4 {
                let worse = (need - m.radius) > worst.map_or(0.0, |(_, _, gap)| gap);
                if worse || worst.is_none() { worst = Some((mi, need, need - m.radius)); }
            }
        }
        assert!(worst.is_none(), "{archive}: World material {:?} has a cull sphere {:.4} under its drawn verts' {:.4}",
            worst.map(|(mi, _, _)| mi), worst.map_or(0.0, |(_, _, g)| 0.0), worst.map_or(0.0, |(_, n, _)| n));
    }
}

/// The sky/sun object's red-glow batches parse with blend 0 on the disc (the port's `Draw::from_gs`): the
/// "T + red vertical line" sun artifact is the port drawing these batches with the leaf's own ALPHA_1 — the
/// game's sky pass state is G-019's open item. This pin names the batches the next fix must re-route.
#[test]
fn the_sun_glow_batches_carry_normal_blend_on_the_disc() {
    let Some(iso_path) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene: Scene = nf_view::load(&iso_path, "07000012.bin", false).expect("load");
    // The sun glow texture 0026:0147 (32×32, red disc on black, OPAQUE — the port's decode, probe-verified).
    let glow: Vec<_> = scene.materials.iter().filter(|m| m.key == Some((0x26, 0x147))).collect();
    assert!(!glow.is_empty(), "the red sun-glow texture 0026:0147 binds at least one material on 07000012");
    let sky: Vec<_> = glow.iter().filter(|m| matches!(m.layer, Layer::SkyCamera | Layer::SkyFixed)).collect();
    assert!(!sky.is_empty(), "the sun-glow batches draw in the sky pass");
    for m in &sky {
        assert_eq!(m.draw.blend, 0, "on the disc the sun-glow leaf carries ALPHA_1 0x44 (blend 0) — the port uses it verbatim; the game's sky-pass override is the open G-019 delta");
        assert!(m.draw.pass == 0 || m.draw.pass == 4, "sky-glow batches ride a sky pass, got pass {}", m.draw.pass);
    }
}
