//! The driving missions' car models against the measured golden
//! (`golden/carmodel_golden.tsv`): the five plain `MISC.VIV` car `.crp`s — article name, the
//! `Skel` part table (the wheel/hub attachment points), the `ns#0` texture-sheet name, the
//! per-(variant, LOD) vertex counts and the draw-buffer SHA-1 list digest.
//!
//! Read path: the same `NF_DRIVING`/`<repo>/DRIVING` folder the other driving tests use
//! (docs/formats/driving-viv.md). The five `MISC.VIV` cars are the only **plain** `.crp`
//! entries [D], so this test needs no RefPack data. Without the folder every test here skips
//! (the slim ISO carries no `DRIVING/` bytes).
//!
//! Format: docs/formats/car-model.md; parser `nf_formats::carmodel`.

use nf_formats::{bigf, carmodel, refpack, sha1};

fn driving_dir() -> Option<std::path::PathBuf> {
    if let Some(p) = std::env::var_os("NF_DRIVING") {
        let p = std::path::PathBuf::from(p);
        return p.is_dir().then_some(p);
    }
    let p = std::path::PathBuf::from("DRIVING");
    p.is_dir().then_some(p)
}

fn sha1_hex(b: &[u8]) -> String { sha1::hex(&sha1::sha1(b)) }

/// The digest over the ordered per-buffer SHA-1 strings (the golden's `buffer_list_sha1`).
fn list_digest(bufs: &[String]) -> String {
    let mut h = sha1::Sha1::new();
    for b in bufs {
        h.update(b.as_bytes());
    }
    sha1::hex(&h.finish())
}

fn load_misc_cars(dir: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    let viv = std::fs::read(dir.join("MISC.VIV")).expect("read DRIVING/MISC.VIV");
    let g = bigf::Bigf::parse(&viv, viv.len() as u64).expect("MISC.VIV: BIGF");
    let mut out = Vec::new();
    for e in &g.entries {
        if !e.name.contains("car\\model") || !e.name.ends_with(".crp") { continue; }
        let bytes = viv[e.offset as usize..(e.offset + e.size) as usize].to_vec();
        let bytes = if refpack::is_refpack(&bytes) {
            refpack::decompress(&bytes).unwrap_or_else(|err| panic!("{}: refpack: {err}", e.name))
        } else {
            bytes
        };
        out.push((e.name.rsplit('\\').next().unwrap_or(&e.name).to_string(), bytes));
    }
    out
}

/// One canonical row per car: `car\tparts\tmodels\tvariant.lod:vertices:v4_32_all\tbuffers\t
/// buffer_list_sha1\tvertices\tv4_32_all\tsheet`.
fn canonical(name: &str, bytes: &[u8]) -> String {
    let m = carmodel::CarModel::parse(bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
    let article = m.name().expect("name").unwrap_or("-");
    let parts = m.skel().expect("skel").map(|s| s.parts.join(",")).unwrap_or_default();
    let sheet = m.sheet_name().expect("sheet").unwrap_or("-");
    let cm = carmodel::CarModule::parse(m.module_bytes().expect("module bytes"))
        .unwrap_or_else(|e| panic!("{name}: module: {e}"));
    let mut per_model = cm.vertices_per_model();
    per_model.sort_by_key(|(v, l, _)| (*v, *l));
    let cells: Vec<String> = per_model
        .iter()
        .map(|(v, l, _)| {
            let (tot, pos): (u32, u32) = cm
                .models
                .iter()
                .find(|m| m.variant == *v && m.lod == *l)
                .map(|m| {
                    m.groups
                        .iter()
                        .flat_map(|g| g.buffers.iter())
                        .filter_map(|b| cm.buffers.iter().find(|x| x.offset == *b))
                        .fold((0u32, 0u32), |(a, p), x| {
                            (a + x.census.v4_32_quads, p + x.census.v4_32_pos_quads)
                        })
                })
                .unwrap_or((0, 0));
            format!("{v}.{l}:{pos}:{tot}")
        })
        .collect();
    format!(
        "{name}\t{parts}\t{}\t{}\t{}\t{}\t{}\t{}\t{article}\t{sheet}\n",
        cm.models.len(),
        cells.join(","),
        cm.buffers.len(),
        list_digest(&cm.buffers.iter().map(|b| b.sha1.clone()).collect::<Vec<_>>()),
        cm.buffers.iter().map(|b| b.census.vertices()).sum::<u32>(),
        cm.buffers.iter().map(|b| b.census.v4_32_quads).sum::<u32>(),
    )
}

fn diff(golden: &str, actual: &str) -> Vec<String> {
    let g: Vec<&str> = golden.lines().collect();
    let a: Vec<&str> = actual.lines().collect();
    let mut out = Vec::new();
    for i in 0..g.len().max(a.len()) {
        match (g.get(i), a.get(i)) {
            (Some(x), Some(y)) if x == y => {}
            (Some(x), Some(y)) => out.push(format!("row {i}:\n  golden {x}\n  actual {y}")),
            (Some(x), None) => out.push(format!("row {i} missing in actual: {x}")),
            (None, Some(y)) => out.push(format!("row {i} extra in actual: {y}")),
            (None, None) => {}
        }
    }
    out
}

#[test]
fn misc_car_models_match_the_golden() {
    let Some(dir) = driving_dir() else { eprintln!("skip: no DRIVING/ folder (set NF_DRIVING)"); return };
    let golden = include_str!("../golden/carmodel_golden.tsv");
    let cars = load_misc_cars(&dir);
    assert_eq!(cars.len(), 5, "the five plain MISC.VIV car models");
    let mut rows = Vec::new();
    for (name, bytes) in &cars {
        rows.push(canonical(name, bytes));
    }
    let actual = format!(
        "car\tparts\tmodels\tvariant.lod:vertices:v4_32_all\tbuffers\tbuffer_list_sha1\tvertices\tv4_32_all\tarticle\tsheet\n{}",
        rows.join("")
    );
    if let Some(p) = std::env::var_os("CARMODEL_REGEN") {
        let p = std::path::PathBuf::from(p);
        std::fs::write(&p, &actual).unwrap();
        eprintln!("regenerated {}", p.display());
        return;
    }
    let d = diff(golden, &actual);
    assert!(d.is_empty(), "{} differences:\n{}", d.len(), d.join("\n"));
    // Sister: a one-byte change to one decompressed model must be caught.
    let (name, bytes) = &cars[0];
    let mut bad = bytes.clone();
    let mid = bad.len() / 2;
    bad[mid] ^= 0x40;
    let tampered = canonical(name, &bad);
    assert_ne!(tampered, rows[0], "sister: a one-byte change must be caught");
}

#[test]
fn misc_car_models_have_named_wheel_parts() {
    // The vehicle code resolves the wheel attachment points by these part names
    // [M FUN_001d2648 0x1d2648, name table EE 0x334070..0x334094].
    let Some(dir) = driving_dir() else { eprintln!("skip: no DRIVING/ folder (set NF_DRIVING)"); return };
    for (name, bytes) in load_misc_cars(&dir) {
        let m = carmodel::CarModel::parse(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        let sk = m.skel().expect("skel").unwrap_or_else(|| panic!("{name}: no Skel"));
        for w in ["leftfrontwheel", "rightfrontwheel", "leftrearwheel", "rightrearwheel"] {
            assert!(sk.parts.iter().any(|p| p == w), "{name}: part {w} missing in {:?}", sk.parts);
        }
        // Sister: the hub names are a DIFFERENT table (EE 0x334080) and cars need not have
        // every hub — but a Skel part name that no consumer looks up must not silently pass:
        // check that the part list is non-trivial and each name parses as ASCII.
        assert!(sk.parts.iter().all(|p| p.is_ascii() && !p.is_empty()), "{name}: bad part name");
        assert!(sk.count as usize == sk.parts.len(), "{name}: count {} vs parts {}", sk.count, sk.parts.len());
    }
}
