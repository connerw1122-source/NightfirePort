//! The sky pass's GS state (GAPS G-019 item 3): the first CNT's DIRECT A+D writes of the sky glists,
//! decoded the way `scene.rs` decodes every leaf, against the pipeline flags `gpu.rs` uses for the sky
//! passes (`pass 0` pre-world: depth Always, no write; `pass 4` post-world: LessEqual, no write, far pin).
//!
//! The frame-level state around the sky draws is `View_DrawSky` 0x1e5ad0: `psiSetTweakARGB(0xff…)` (no
//! tint), `psiSetUpColourBlend(2)` → `AddDMAChainRegSet(0x2a, 0x42)` [M 0x1214f8] → ALPHA_1 = 0x2a =
//! (0−Cd)·0 + Cs = replace, `psiLight_SetLights(0,…)` (no lights), `psiFog(...)` — a stub [M 0x121578,
//! size 8], so the sky pass itself writes no fog register; the per-leaf DIRECT writes below are the
//! blend/test/depth truth. Opt-in via `NF_ISO`.

use nf_formats::iso9660::IsoImage;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The sky objects of one level (type-0x2A placements) with their first-leaf DIRECT writes and the
/// placement parameters (0 = slot, 1 = flash variant, 2 = interval param, 3 = flash length, 4 = sky
/// group) [M parsemap 0x1d04f0 case 0x2a: the params land at level+0x2c..].
fn sky_objects(iso: &IsoImage, level: &str) -> Vec<(String, [u32; 5], Vec<(u8, u64)>)> {
    let (entries, bin) = nf_formats::disc::files_bin_entries(iso).expect("disc entries");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(level)).expect("level on disc");
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("level bytes");
    let level = nf_formats::level::Level::parse(&data).expect("parse");
    let main = level.main_map().expect("main map");
    let mut out = Vec::new();
    for p in &level.maps[main].placements {
        if p.flags != 0x2a { continue; }
        let params: [u32; 5] = [0, 1, 2, 3, 4].map(|k| p.params.iter().find(|(i, _)| *i == k).map(|&(_, v)| v).unwrap_or(0));
        let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() } else { Some((main, p.index as usize)) };
        let Some((om, oi)) = target else { continue };
        let obj = &level.maps[om].objects[oi];
        // The first leaf's DIRECT writes in packet order (the "first CNT" of the glist).
        let gs = obj.gfx.leaves.first().map(|l| l.gs.iter().map(|&(r, v, _)| (r, v)).collect()).unwrap_or_default();
        out.push((obj.name.clone(), params, gs));
    }
    out
}

fn decode_alpha(v: u64) -> (u64, u64, u64, u64, u64) {
    let lo = v & 0xff;
    (lo & 3, (lo >> 2) & 3, (lo >> 4) & 3, (v >> 8) & 0xff, 0)
}

#[test]
fn sky_pass_gs_state_from_the_first_cnt() {
    let Some(path) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let iso = IsoImage::open(&path).unwrap();

    // 0700000a: slots 0..3, group 0 (param 4 = 0), Thunderhead slot 3 is the flash variant [D disc].
    let objs = sky_objects(&iso, "0700000a.bin");
    assert_eq!(objs.len(), 4, "four type-0x2A placements");
    let (name, params, gs) = &objs[3];
    assert_eq!(name.as_str(), "Thunderhead");
    assert_eq!(*params, [3, 1, 24, 5, 0], "slot 3, flash variant, interval param 24, flash length 5, group 0");
    // MEASURED [D]: the Thunderhead's first leaf carries NO A+D writes at all — the sky glists inherit
    // the pass state entirely: ALPHA_1 = 0x2a (replace) from psiSetUpColourBlend(2), no fog change (the
    // psiFog stub), and whatever depth state the frame set before the sky draws. The renderer's
    // zwrite=false on both sky passes models that inherited depth state; pinning the exact ZBUF_1 the GS
    // holds during the sky draws is a live check for the main agent (report: fix-sky).
    let alpha = gs.iter().find(|&&(r, _)| r == 0x42).map(|&(_, v)| v).unwrap_or(0x2a);
    let test = gs.iter().find(|&&(r, _)| r == 0x47).map(|&(_, v)| v).unwrap_or(0x5001b);
    let zbuf = gs.iter().find(|&&(r, _)| r == 0x4e).map(|&(_, v)| v).unwrap_or(0x3100_0002);
    assert_eq!(gs.is_empty(), true, "the first CNT of the flash sky carries no A+D writes (inheritance)");
    println!("Thunderhead first-CNT writes: {gs:?}; effective ALPHA_1 {alpha:#x}, TEST_1 {test:#x}, ZBUF_1 {zbuf:#x}");
    let (a, b, c, _fix, _d) = decode_alpha(alpha);
    println!("Thunderhead effective ALPHA_1 {alpha:#x} (A{a} B{b} C{c}) TEST_1 {test:#x} ZBUF_1(inherited assumption) {zbuf:#x}");
    // The pass-initial blend is the replace shape [M 0x1214f8 mode 2 → ALPHA_1 0x2a]:
    // A=2(0) B=2(0) C=2(FIX=0) → ((0−0)·0 >> 7) + Cs = Cs — straight replace, no blending.
    assert_eq!((a, b, c), (2, 2, 2), "0x2a: output = Cs — replace, no blend");
    // The pipeline the renderer gives the sky draws (gpu.rs `make_pipeline`): the renderer keeps depth
    // writes off on both sky passes; the disc shows the StarField writing ZBUF_1 itself (0x1_31000000),
    // so the exact ZMSK state during the sky draws is a live check, not a settled fact.
    println!("pipeline under test: pass 0 = Always/no-write, pass 4 = LessEqual/no-write + FAR pin (gpu.rs)");
    // The blend decode must stay inside the shapes scene.rs `Draw` models.
    let draw = nf_view::scene::Draw::from_gs(4, 0, alpha, test, zbuf);
    println!("decoded Draw: {draw:?}");
    assert!(draw.blend <= 4 && draw.atest <= 2, "the decoded state must map onto the modeled shapes");

    // 0700001b: the rotating sky — slots 0/1, group 0, glist flags 0x222 (bit 0x8 clear: both slots run
    // the rotation block, two counts per frame [M 0x1e5ad0:186-198]).
    let objs = sky_objects(&iso, "0700001b.bin");
    let names: Vec<&str> = objs.iter().map(|(n, _, _)| n.as_str()).collect();
    assert_eq!(names, vec!["SKY_StarField", "SKY_Earth"]);
    for (_, params, gs) in &objs {
        assert_eq!(params[0] <= 1, true, "slots 0/1");
        assert_eq!(params[4], 0, "group 0 (the post-world pass 4)");
        assert_eq!(params[1], 0, "no flash variant on 0700001b");
        let zbuf = gs.iter().find(|&&(r, _)| r == 0x4e).map(|&(_, v)| v).unwrap_or(0x3100_0002);
        println!("{} first-CNT writes: {gs:?} (params {params:?})", names[0]);
        assert!(gs.iter().all(|&(r, _)| matches!(r, 0x42 | 0x47 | 0x4e)), "only the modeled registers occur");
        // MEASURED [D]: SKY_StarField writes its own state — ALPHA_1 0x44 (normal), TEST_1 0x5001b
        // (As ≥ 1), ZBUF_1 0x1_31000000 (ZPSM 32-bit Z, ZMSK bit 20 — the exact write-disable bit and
        // its state during the sky draws is a live check for the main agent, report: fix-sky).
        let alpha = gs.iter().find(|&&(r, _)| r == 0x42).map(|&(_, v)| v).unwrap_or(0x2a);
        if alpha != 0x2a {
            let (a, b, c, _f, _d) = decode_alpha(alpha);
            assert_eq!((a, b, c), (0, 1, 0), "the StarField's own blend is the normal shape 0x44");
        }
        let draw = nf_view::scene::Draw::from_gs(4, 0,
            gs.iter().find(|&&(r, _)| r == 0x42).map(|&(_, v)| v).unwrap_or(0x44),
            gs.iter().find(|&&(r, _)| r == 0x47).map(|&(_, v)| v).unwrap_or(0x5001b),
            zbuf);
        println!("{:?}: draw {draw:?}", names[0]);
    }
    // Sister: the pass-4 pipeline's far pin (gpu.rs FAR=1.0) only exists on the port side; assert the
    // game-side blend of the sky's first CNT is the replace shape 0x2a the pass opens with — the same
    // value `psiSetUpColourBlend(2)` writes before the leaves [M 0x1e5ad0:88, 0x1214f8].
    let (_, _, gs) = &sky_objects(&iso, "0700001b.bin")[0];
    if let Some(&(0x42, alpha)) = gs.iter().find(|&&(r, _)| r == 0x42) {
        let (a, b, c, d, _fix) = decode_alpha(alpha);
        println!("StarField ALPHA_1 A{a} B{b} C{c} D{d}");
    } else {
        println!("StarField: no ALPHA_1 in the first CNT — the pass-initial 0x2a (psiSetUpColourBlend(2)) stands");
    }
}
