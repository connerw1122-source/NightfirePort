//! Throwaway probe (fix-swim-vent2). READ-ONLY: verifies the stored-plane decode against the raw
//! disc blocks of one level. For every collision triangle whose WORLD position lands inside a query
//! box (default: the 07000008 cage wall at x 135.3), print the winding cross, the plane halfword,
//! the raw s16 plane record and the decoded stored normal — plus the level-wide flip count.
//!
//! Run: cargo run --offline --release -p nf-view --example probe_planes -- <iso> <level> [x0 x1 y0 y1 z0 z1]

use nf_formats::disc::files_bin_entries;
use nf_formats::iso9660::IsoImage;
use nf_formats::level::Level;

fn xform(m: &[[f32; 4]; 4], p: [f32; 3]) -> [f32; 3] {
    // row-major 4x4 with translation in the last row (world.rs transform convention)
    let mut o = [0f32; 3];
    for k in 0..3 {
        o[k] = m[0][k] * p[0] + m[1][k] * p[1] + m[2][k] * p[2] + m[3][k];
    }
    o
}

fn main() {
    let mut args = std::env::args().skip(1);
    let iso = args.next().unwrap_or_else(|| std::env::var("NF_ISO").unwrap_or_default());
    let level_name = args.next().unwrap_or_else(|| "07000008.bin".into());
    let box6: Vec<f32> = args.by_ref().filter_map(|s| s.parse().ok()).collect();
    let (x0, x1, y0, y1, z0, z1) = match box6.len() {
        6 => (box6[0], box6[1], box6[2], box6[3], box6[4], box6[5]),
        _ => (135.0, 135.7, 45.0, 54.0, -117.0, -99.0),
    };
    let iso = IsoImage::open(&iso).expect("iso");
    let (entries, bin) = files_bin_entries(&iso).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(&level_name)).expect("level entry");
    let bytes = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("level bytes");
    let level = Level::parse(&bytes).expect("level");
    let (mut shown, mut flips, mut total) = (0, 0u32, 0u32);
    let (insts, _) = level.instances(false);
    for inst in &insts {
        let map = &level.maps[inst.map];
        let Some(coll) = map.objects.get(inst.object).and_then(|o| o.collision.as_ref()) else { continue };
        for bx in coll.boxes.iter().filter(|b| b.children.is_none()) {
            let base = bx.vblock as usize * 0x20;
            for t in &coll.tris[bx.tris.start as usize..bx.tris.end as usize] {
                total += 1;
                let v = t.v.map(|i| {
                    let w = |k: usize| coll.words.get(base + i as usize + k).copied().unwrap_or(0) as f32;
                    [w(0) * bx.scale + bx.origin[0], w(1) * bx.scale + bx.origin[1], w(2) * bx.scale + bx.origin[2]]
                });
                let wv: Vec<[f32; 3]> = v.iter().map(|p| xform(&inst.matrix, *p)).collect();
                let in_box = wv.iter().all(|p| p[0] >= x0 && p[0] <= x1 && p[1] >= y0 && p[1] <= y1 && p[2] >= z0 && p[2] <= z1);
                let p = t.plane as usize;
                let pw = |k: usize| coll.words.get(base + p + k).copied().unwrap_or(0);
                let n = [pw(0) as f32 / 16384.0, pw(1) as f32 / 16384.0, pw(2) as f32 / 16384.0];
                let e1 = [v[1][0] - v[0][0], v[1][1] - v[0][1], v[1][2] - v[0][2]];
                let e2 = [v[2][0] - v[0][0], v[2][1] - v[0][1], v[2][2] - v[0][2]];
                let cr = [e1[1] * e2[2] - e1[2] * e2[1], e1[2] * e2[0] - e1[0] * e2[2], e1[0] * e2[1] - e1[1] * e2[0]];
                let d = n[0] * cr[0] + n[1] * cr[1] + n[2] * cr[2];
                if d < 0.0 { flips += 1; }
                if in_box && shown < 24 {
                    shown += 1;
                    println!(
                        "tri obj {} plane {p}: winding n [{:.3} {:.3} {:.3}] stored n [{:.4} {:.4} {:.4}] raw s16 [{}, {}, {}] dist_raw_lo {} dot {d:.3}{}",
                        inst.object, cr[0], cr[1], cr[2], n[0], n[1], n[2], pw(0), pw(1), pw(2), pw(3),
                        if d < 0.0 { "  FLIP" } else { "" },
                    );
                    for q in &wv { println!("   v [{:.3} {:.3} {:.3}]", q[0], q[1], q[2]); }
                }
            }
        }
    }
    println!("level {level_name}: {total} collision tris, {flips} would flip (stored opposes winding)");
}
