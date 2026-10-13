//! probe: walkable-cell map of the 07000002 bunker/courtyard region (y ~5.4-6.5). Diagnostic.
use nf_formats::iso9660::IsoImage;
fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }
#[test]
fn probe_bunker_cells() {
    let iso = iso();
    let img = IsoImage::open(&iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000002.bin")).expect("archive");
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read");
    let level = nf_formats::level::Level::parse(&data).expect("parse");
    let world = nf_game::world::CollisionWorld::build(&level, &level.instances(true).0);
    // Rows z −36..−19, cols x −62..−41 at capsule mid y 6.3 (feet 5.4, floor 5.34): '.' free, '#' blocked.
    let mut z = -36.0f32;
    while z <= -19.0 {
        let mut row = String::new();
        let mut x = -62.0f32;
        while x <= -41.0 {
            let (a, b) = ([x, 6.75, z], [x, 6.05, z]);
            let (push, _) = world.capsule_push(a, b, 0.55);
            let blocked = push.iter().any(|v| v.abs() > 0.05);
            row.push(if blocked { '#' } else { '.' });
            x += 1.0;
        }
        eprintln!("z={z:6.1}: {row}   (x -62..-41)");
        z += 1.0;
    }
    eprintln!("legend: x from -62 (left) to -41 (right); sensor band x -47.65..-46.26 z -25.14..-21.55");
}
