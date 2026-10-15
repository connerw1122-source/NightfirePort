//! G-046 exact-sine sites: the world-sphere builds (`Control_BuildWorldSph` 0x133070 →
//! `View_RotTransMatrix` 0x1e5168) for the ladders, wires, third-icons and hint markers of the real
//! levels, bit-compared against the game's arithmetic transcribed from the disassembly. Fail-first: on the
//! pre-swap code these centres come from host `sin_cos` without the game's PS2Sinf pair and its
//! `PS2Sinf(π/2) = 1.0000001` factors, so the comparisons below failed. Opt-in: set `NF_ISO` to the disc.

use nf_formats::iso9660::IsoImage;
use nf_formats::level::Level;
use nf_formats::ps2float as pf;

/// `View_RotTransMatrix` 0x1e5168 for the yaw-only euler (0, yaw, 0) + `Control_BuildWorldSph` 0x133070's
/// VU0 centre transform, transcribed from the disassembly (the same chain the in-crate tests document):
/// C0 = PS2Sinf(0+π/2) = 1.0000001 (one ulp above 1.0 — the L probe in trig.rs), s0 = PS2Sinf(0) = 0.0;
/// rows [c·C0, 0, −(C0·s)] / [0−0, C0·C0, c·0 + C0·s·0] / [C0·s + c·0·0, −(C0·0), C0·c − 0·s·0]; then
/// ACC = row0·cx; + row1·cy; + row2·cz; + trans (products chopped, sums guard-bit [M anim::mul]).
fn game_world_sph(local: [f32; 3], yaw: f32, t: [f32; 3]) -> [f32; 3] {
    let half = f32::from_bits(0x3fc9_0fdb);
    let m32 = pf::mul;
    let fadd = |x, y| pf::add_noguard(x, y, true);
    let fsub = |x: f32, y: f32| pf::add_noguard(x, -y, true);
    let c0 = nf_formats::anim::sinf(fadd(0.0, half));
    let s0 = nf_formats::anim::sinf(0.0);
    let c = nf_formats::anim::sinf(fadd(yaw, half));
    let s = nf_formats::anim::sinf(yaw);
    let (m00, m01, m02) = (m32(c, c0), s0, -m32(c0, s));
    let (m10, m11, m12) = (fsub(m32(s0, s), m32(m32(c0, c), s0)), m32(c0, c0), fadd(m32(c, s0), m32(m32(c0, s), s0)));
    let (m20, m21, m22) = (fadd(m32(c0, s), m32(m32(c, s0), s0)), -m32(c0, s0), fsub(m32(c0, c), m32(m32(s0, s), s0)));
    let chain = |r0: f32, r1: f32, r2: f32, tr: f32| {
        let acc = m32(r0, local[0]);
        let acc = pf::add(acc, m32(r1, local[1]));
        let acc = pf::add(acc, m32(r2, local[2]));
        pf::add(acc, tr)
    };
    [chain(m00, m10, m20, t[0]), chain(m01, m11, m21, t[1]), chain(m02, m12, m22, t[2])]
}

/// The pre-fix port formula (host `sin_cos`, no C0 factors): the sister.
fn host_world_sph(local: [f32; 3], yaw: f32, t: [f32; 3]) -> [f32; 3] {
    let (s, c) = yaw.sin_cos();
    [t[0] + local[0] * c + local[2] * s, t[1] + local[1], t[2] - local[0] * s + local[2] * c]
}

fn level(name: &str) -> Option<Level> {
    let path = std::env::var_os("NF_ISO")?;
    let iso = IsoImage::open(&path).unwrap();
    let (entries, bin) = nf_formats::disc::files_bin_entries(&iso).unwrap();
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name))?;
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    Some(Level::parse(&data).unwrap())
}

/// The placement→object resolution walk the creators share (`Ladder::all` / `Wire::all` / `wire::icons` /
/// `player::hints`): every placement of `flags & 0xff == kind` with a resolved map object, in placement
/// order, as (translation, rotation[1], centre). Wires mirror `Wire::create`'s refusal of type 1
/// (`Player_CollWire` 0x197130:99-106) so the zip stays aligned.
fn walk(level: &Level, kind: u8) -> Vec<([f32; 3], f32, [f32; 3])> {
    let mut out = Vec::new();
    for (maps_i, m) in level.maps.iter().enumerate() {
        for p in &m.placements {
            if p.flags & 0xff != kind as u32 { continue; }
            let obj: Option<(usize, usize)> = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                else if (p.index as usize) < m.objects.len() { Some((maps_i, p.index as usize)) } else { None };
            let Some((om, oi)) = obj else { continue };
            let Some(o) = level.maps.get(om).and_then(|map| map.objects.get(oi)) else { continue };
            if kind == 0x3e && p.params.iter().find(|(i, _)| *i == 1).map_or(0, |&(_, v)| v) == 1 { continue; }
            out.push((p.translation, p.rotation[1], o.centre));
        }
    }
    out
}

/// Bit-compares the port's centres against the game formula; returns (placements, placements where the
/// host formula differs from the game's — the fail-first signal, summed across families because some
/// symmetric bounds centres (lx = lz = 0, tidy ly) agree by rounding).
fn check(name: &str, got: &[[f32; 3]], data: &[([f32; 3], f32, [f32; 3])], what: &str) -> (usize, usize) {
    assert_eq!(got.len(), data.len(), "{name} {what}: the port walked {} but the disc walk found {}", got.len(), data.len());
    assert!(!got.is_empty(), "{name}: no {what} on the disc — pick another level");
    let mut host_diffs = 0;
    for (i, ((t, yaw, local), g)) in data.iter().zip(got.iter()).enumerate() {
        let want = game_world_sph(*local, *yaw, *t);
        let old = host_world_sph(*local, *yaw, *t);
        for k in 0..3 {
            assert_eq!(g[k].to_bits(), want[k].to_bits(), "{name} {what} #{i} comp {k}: port {:e} vs game {:e} (yaw {yaw}, local {local:?}, t {t:?})", g[k], want[k]);
        }
        if (0..3).any(|k| old[k].to_bits() != want[k].to_bits()) { host_diffs += 1; }
    }
    println!("{name} {what}: {} placements, {host_diffs} differ from the host formula", got.len());
    (got.len(), host_diffs)
}

#[test]
fn ladder_wire_icon_hint_world_spheres_match_the_game() {
    let (mut n, mut diffs) = (0usize, 0usize);
    // 07000002: the bunker level — ladders (type 0x22) and the two wires (type 0x3E) [D placements].
    if let Some(l) = level("07000002.bin") {
        let ladders = nf_game::ladder::all(&l);
        let (a, b) = check("07000002", &ladders.iter().map(|x| x.centre).collect::<Vec<_>>(), &walk(&l, 0x22), "ladders");
        n += a; diffs += b;
        let wires = nf_game::wire::all(&l);
        let (a, b) = check("07000002", &wires.iter().map(|x| x.centre).collect::<Vec<_>>(), &walk(&l, 0x3e), "wires");
        n += a; diffs += b;
        let icons = nf_game::wire::icons(&l);
        let (a, b) = check("07000002", &icons.iter().map(|x| x.centre).collect::<Vec<_>>(), &walk(&l, 0xfa), "icons");
        n += a; diffs += b;
        let hints = nf_game::player::hints(&l);
        let (a, b) = check("07000002", &hints.iter().map(|x| x.centre).collect::<Vec<_>>(), &walk(&l, 0x33), "hints");
        n += a; diffs += b;
    }
    // 07000005: the estate — wires + icons + ladders (no type-0x33 hint markers on this level).
    if let Some(l) = level("07000005.bin") {
        let wires = nf_game::wire::all(&l);
        let (a, b) = check("07000005", &wires.iter().map(|x| x.centre).collect::<Vec<_>>(), &walk(&l, 0x3e), "wires");
        n += a; diffs += b;
        let icons = nf_game::wire::icons(&l);
        let (a, b) = check("07000005", &icons.iter().map(|x| x.centre).collect::<Vec<_>>(), &walk(&l, 0xfa), "icons");
        n += a; diffs += b;
        let ladders = nf_game::ladder::all(&l);
        let (a, b) = check("07000005", &ladders.iter().map(|x| x.centre).collect::<Vec<_>>(), &walk(&l, 0x22), "ladders");
        n += a; diffs += b;
    }
    assert!(n > 0, "no world-sphere placements found — check the level ids");
    // Sister (global): the host formula must differ from the game's on SOME disc placement, else this
    // test could not have failed pre-fix.
    assert!(diffs > 0, "sister failed: the host formula agrees with the game on all {n} placements");
    println!("total: {n} placements, {diffs} where the host formula differs from the game's");
}
