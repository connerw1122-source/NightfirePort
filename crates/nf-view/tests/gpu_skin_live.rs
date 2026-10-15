//! G-043 GPU skinning: the palette + per-vertex attributes (`Scene::skin_layout` / `skin_palette`) must reproduce the
//! CPU pose (`Scene::animate`) and colour (`Scene::posed_vertices`) exactly, vertex for vertex. The shader does the
//! same arithmetic as `apply` below. Opt-in: NF_ISO.

use nf_formats::placement::transform;
use nf_view::scene::{Scene, SkinAttr, ViewOpts};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The shader's vertex step, on the CPU — including fix-gpu-facial-morphs' morph term (`SkinIt`
/// 0x10bf30's `pos += Σ_m weight[m] × target[morph + m]` before the blend, read from the same
/// buffers the shader gets: the weight blocks and the concatenated targets).
fn apply(p: &[[[f32; 4]; 4]], a: &SkinAttr, pos0: [f32; 3], wts: &[f32], tgt: &[f32]) -> [f32; 3] {
    let mut pos = pos0;
    if a.mtb >= 0 {
        let (mwb, cnt, tend) = (a.mwb as usize, a.cnt as usize, a.tend as usize);
        for m in 0..cnt {
            let w = wts[(mwb + m / 4) * 4 + m % 4];
            if w != 0.0 {
                let e = a.mtb as usize + m * 3;
                if e + 2 < tend {
                    for k in 0..3 { pos[k] += w * tgt[e + k]; }
                }
            }
        }
    }
    let pa = transform(&p[a.a as usize], pos);
    let pb = transform(&p[a.b as usize], pos);
    [0, 1, 2].map(|k| a.w * pa[k] + (1.0 - a.w) * pb[k])
}

fn check(scene: &mut Scene, ticks: u32) {
    for _ in 0..ticks { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let opts = ViewOpts::default();
    let t = ticks as f32;
    let cpu = scene.animate(None, t, opts);
    let cpu_v = scene.posed_vertices(None, t, opts);
    let (layout, attrs) = scene.skin_layout();
    let (pal, cols) = scene.skin_palette(&layout, None, t, opts, None);
    let wts = scene.skin_morph_weights(&layout);
    let tgt_f: Vec<f32> = scene.morph_target_blob().chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect();
    assert_eq!(attrs.len(), scene.char_verts.len());
    assert_eq!(cpu.len(), scene.char_verts.len());
    let (mut worst, mut at) = (0f32, 0usize);
    let mut col_bad = 0usize;
    for (i, (a, cv)) in attrs.iter().zip(&scene.char_verts).enumerate() {
        let g = apply(&pal, a, cv.pos, &wts, &tgt_f);
        let e = (0..3).map(|k| (g[k] - cpu[i][k]).abs()).fold(0f32, f32::max);
        if e > worst { worst = e; at = i; }
        // Colour: the CPU path rounds rgba·m to a byte; the GPU multiplies in float. Allow one step.
        let base = scene.vertices[scene.char_vertex_start + i].rgba;
        let c = cols[a.col as usize];
        for k in 0..3 {
            let gpu = base[k] as f32 * c[k];
            if (gpu - cpu_v[i].rgba[k] as f32).abs() > 1.0 { col_bad += 1; break; }
        }
    }
    eprintln!("{} vertices, {} palette matrices, worst position error {worst} at {at}, colour mismatches {col_bad}", attrs.len(), pal.len());
    assert!(worst < 1e-3, "GPU palette position differs from the CPU pose by {worst} at vertex {at}");
    assert_eq!(col_bad, 0, "GPU colour table differs from the CPU multiplier");
}

#[test]
fn gpu_palette_matches_the_cpu_pose_on_07000005_06_07() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for lvl in ["07000005.bin", "07000006.bin", "07000007.bin"] {
        let mut scene = nf_view::load(&iso, lvl, false).expect("load");
        check(&mut scene, 1);
        check(&mut scene, 240);
    }
}

#[test]
fn sister_a_wrong_palette_is_caught() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    let opts = ViewOpts::default();
    let cpu = scene.animate(None, 1.0, opts);
    let (layout, attrs) = scene.skin_layout();
    let (mut pal, _) = scene.skin_palette(&layout, None, 1.0, opts, None);
    for m in pal.iter_mut() { m[3][1] += 1.0; }
    let i = attrs.iter().position(|a| a.a != a.b && a.mtb < 0).expect("a blended non-morph vertex");
    let g = apply(&pal, &attrs[i], scene.char_verts[i].pos, &[], &[]);
    assert!((g[1] - cpu[i][1]).abs() > 0.5, "a shifted palette must move the vertex");
}

/// fix-lighting groundwork: skinned vertices decode a unit normal (high 16 bits of skinning words 0..2, x 64).
#[test]
fn skinned_normals_are_unit_length() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let img = nf_formats::iso9660::IsoImage::open(&iso).expect("iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000005.bin")).expect("archive");
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read");
    let level = nf_formats::level::Level::parse(&data).expect("parse");
    let mut n = 0usize;
    let mut bad = 0usize;
    for map in &level.maps {
        for o in &map.objects {
            if !o.gfx.is_skinned() { continue; }
            for leaf in &o.gfx.leaves { for b in &leaf.batches { if !b.skinned { continue; } for v in &b.verts {
                let l = v.normal.iter().map(|&c| (c as f32 / 128.0).powi(2)).sum::<f32>().sqrt();
                n += 1;
                if !(0.9..1.1).contains(&l) { bad += 1; }
            } } }
        }
    }
    eprintln!("{n} skinned vertices, {bad} with a non-unit normal");
    assert!(n > 1000);
    assert!(bad * 100 < n, "more than 1% of skinned normals are not unit length");
}
