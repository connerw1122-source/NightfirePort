//! Headless NF_ISO tests for `character-room-light`: which room-cel ambient a character uses in the
//! game vs the port's lookup, whether dynamic lights add to characters, and the per-character
//! ambient multiplier (docs/systems/render.md "Characters", docs/atlas/render-port-check.md §1.1).
//!
//! Game evidence [M]:
//! - A character's ambient source is its **linked cel** `obj+0x20`
//!   (`control_link_object_to_cel` 0x1346b8 sets it and snaps the 0xff,ff,ff init sentinel of
//!   `control_init_object` 0x134630:22-23; `Control_CreateObjEx` 0x1343c8:39 links at create through
//!   `build_LinkToRoom` 0x1c9680 → `build_FindCel` 0x1c8cb0 at the world sphere). Per tick the
//!   movement pass (`control_movement_object_handler` 0x1336f8:209-211) runs
//!   `Control_BuildWorldSph` 0x133070 → `control_handle_cel_change` 0x133288 →
//!   `Lights_CalcAmbientLight` 0x1cb178 for every MOVED object (flags2 0x20):
//!   `Cel_ObjectLeftCel` 0x1e0d58 walks the current cel's portals and returns the neighbour across
//!   the crossed one; a miss keeps the cel while `build_point_in_bounding_box` 0x1c9500 still
//!   contains the position; otherwise the object re-links (`build_LinkToRoom`) and, when that also
//!   fails, the default case clamps the position into the OLD cel and re-links it — the ambient
//!   keeps coming from the linked cel either way.
//! - `Lights_CalcAmbientLight` 0x1cb178 ramps `obj+0x100..0x102` toward
//!   `cel + switch_channels[cel+0xa2]`, bytes +0x9a/0x9c/0x9e (the runtime bytes
//!   `parseentity_fixup_entity` 0x1d00c0:117-136 fills: params 0/1/2, the 0xc047 second triple at
//!   params 6/7/8), through `ClrStep(…, 0.1)` — only when `obj+0xF0 & 0x800` and the cel is linked.
//! - The draw multiplies `tint × ambient >> 8` (`View_SetupRenderModes_Tweak` 0x1e5888:52-55); the
//!   characters take the SAME path — `View_DrawObjList` 0x1e69a0:96 → `AnimObjectDraw` (flags2 & 8)
//!   → `View_SetupRenderModes` 0x1e78b0:20 → the same Tweak — plus
//!   `Lights_CalcClosestLights(obj+0x80, obj+0xF0)` (0x1e69a0:73-81 / 0x1e78b0:21-23): dynamic
//!   lights DO add to characters (skins 0x8a0/0x28a0 carry flag 0x80, fix-lighting2 [D]). The
//!   `bCharacterLight`/`bShadowCharacter`/`AnimColRad` globals (0x30cdd8..0x30cddf) are written
//!   (0x1e69a0:70-74, cleared in `AnimObjectDraw` 0x1c4a8c) but have NO reader in the EE code —
//!   scanned every `lb/lh/lw/lwc1` base-`$gp` access; no per-character special lighting exists.
//! - The tint `obj+0x103..0x105` is written only by `Script_SetColour` 0x1db770 /
//!   `SP_SetColour` 0x1dca90 (scripts), default 0xff.
//!
//! The port keeps a per-tick position lookup (`Sim::ambient_at` → `find_cel`'s smallest-volume
//! stand-in for build_FindCel's raycast tie-break, fix-render2). These tests pin the observable
//! contract on the disc data: every drone-backed character renders at its OWN cel's ambient, the
//! sentinel stays only on the characters the game itself leaves unlinked, and the dynamic lights
//! the game would add are available to the character draws.
//!
//! Opt-in via `NF_ISO`.

use nf_view::scene;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Step the sim `n` ticks with no input (the viewer's per-frame `step_bond`).
fn step(scene: &scene::Scene, n: usize) {
    let sim = scene.sim.as_ref().unwrap() as *const scene::Sim as *mut scene::Sim;
    for _ in 0..n {
        unsafe { (*sim).step_bond(Default::default(), false, false, false) };
    }
}

fn close(a: [f32; 3], b: [f32; 3]) -> bool {
    a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() < 1.0 / 255.0)
}

/// The multiplier floats `(tint × ambient) >> 8` of `View_SetupRenderModes_Tweak` (0x1e5888) for the
/// white tint, on the game's integer path.
fn expect_bytes(a: [u8; 3]) -> [f32; 3] {
    [0, 1, 2].map(|k| ((0xffu32 * a[k] as u32) >> 8) as f32 / 255.0)
}

/// Every room-cel bbox containing `p` (the overlap the smallest-volume tie-break arbitrates).
fn containing(cels: &[scene::Cel], p: [f32; 3]) -> Vec<usize> {
    let mut v = Vec::new();
    for (i, c) in cels.iter().enumerate() {
        let inside = c.min[0] <= p[0] && p[0] <= c.max[0] && c.min[1] <= p[1] && p[1] <= c.max[1] && c.min[2] <= p[2] && p[2] <= c.max[2];
        if inside { v.push(i); }
    }
    v
}

/// 07000007 (the party): every drone-backed character must carry its OWN cel's ambient — the port's
/// stand-in for the game's linked-cel ramp — and the 0xff sentinel may remain only on characters the
/// game itself leaves unlinked (no cel contains their position: the origin script actors; the port's
/// dev-pool copies are parked outside every cel too). The visible difference the user reported
/// ("characters lit by the room": no) would show here as a full-bright (1.0) guest or a guest
/// carrying another room's ambient.
#[test]
fn ballroom_guests_lit_by_their_own_room_on_07000007() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000007.bin", false).expect("load 07000007");
    let sim = scene.sim.as_ref().expect("sim");
    step(&scene, 5);
    let cels = &sim.cels;
    assert!(!cels.is_empty(), "07000007 parses no room cels");
    let mut sentinel_visible = 0usize;
    let mut distinct: Vec<[u8; 3]> = Vec::new();
    for (ci, ch) in scene.characters.iter().enumerate() {
        let pos = [ch.matrix[3][0], ch.matrix[3][1], ch.matrix[3][2]];
        let inside = containing(cels, pos);
        let amb = sim.char_ambient[ci];
        if sim.char_drone[ci].is_some() {
            // A placed guest: the game links it at create and ramps toward its cel's triple.
            assert_eq!(inside.len(), 1, "char {ci}: unique containment expected at the placement");
            let expect = cels[inside[0]].ambient[0];
            let mult = sim.char_multiplier(ci);
            assert!(close(mult, expect_bytes(expect)),
                "char {ci} multiplier {mult:?} != its own cel {}/255 ({expect:?}) — lit by the wrong room", inside[0]);
            assert!(!close(mult, [1.0; 3]), "char {ci} renders at the pre-link 1.0 multiplier (full bright)");
            if !distinct.contains(&expect) { distinct.push(expect); }
        } else if amb == [0xff; 3] {
            // The sentinel: only legitimate while the game would also leave the object unlinked —
            // its position must be inside NO cel (the origin script actors, the parked dev pool).
            assert!(inside.is_empty(),
                "char {ci} (skin {:08x}) sits in cel(s) {inside:?} but never snapped off the 0xff sentinel — the game would have linked and snapped it",
                ch.skin.hashcode);
            // And it must not draw: a script-less, drone-less character draws at its matrix only
            // when not hidden — the dev pool hides by role, the origin actors carry no start draw
            // in the game (their objects do not exist until a script creates them).
            assert!(matches!(ch.role, nf_formats::drone::SpawnRole::Reinforcement { .. }) || ch.start.is_none(),
                "char {ci} is sentinel-lit but would draw (role {:?}, start {})", ch.role, ch.start.is_some());
            sentinel_visible += 1;
        }
    }
    // The per-room property: the guests' own cels carry more than one ambient triple on this level.
    assert!(distinct.len() >= 3, "expected guests in rooms with distinct ambients, got {distinct:?}");
    eprintln!("07000007: {} guests hit {} distinct room ambients; {sentinel_visible} legitimate unlinked sentinels", scene.characters.len(), distinct.len());
    // The multipliers reach the vertex buffer: some Character-layer vertex is darker than the raw rgba.
    let posed = scene.posed_vertices(None, 0.0, Default::default());
    let start = scene.char_vertex_start;
    assert!(posed.iter().zip(scene.vertices[start..].iter())
        .any(|(p, v)| p.rgba[0] < v.rgba[0] || p.rgba[1] < v.rgba[1] || p.rgba[2] < v.rgba[2]),
        "no Character-layer vertex was dimmed by the ambient multiply");
}

/// The party's dynamic lights: 07000007 places ~104 active kind-0 lights (the candles/lamps,
/// `Light_Create` 0x1cc3d8 via the type-0x4E placements and the map radiators) and the character
/// draws must see them (`View_DrawObjList` 0x1e69a0:73-81 passes the skin flags 0x8a0 — bit 0x80
/// admits kinds 0/1/2 — to `Lights_CalcClosestLights` 0x1cb2e0; the port's per-draw `pick2`).
#[test]
fn party_lights_are_available_to_the_character_draws() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000007.bin", false).expect("load 07000007");
    let sim = scene.sim.as_ref().expect("sim");
    step(&scene, 2);
    let kind0 = sim.drones.lights.lights.iter().filter(|l| l.kind == 0 && l.active).count();
    assert!(kind0 >= 32, "07000007's kind-0 lights missing: only {kind0} active");
    // The frame-resolved set (what gpu.rs picks from per draw) must return a light for guests in
    // three different rooms — the dynamic add the game applies on top of the cel ambient.
    let resolved = sim.drones.lights.resolve(&|i: u32| sim.drones.searchlights.parts.get(i as usize).map(|p| p.pos).unwrap_or([0.0; 3]));
    let mut rooms_with_light = 0usize;
    for ci in [9usize, 30, 38] {
        let ch = &scene.characters[ci];
        let p = [ch.matrix[3][0], ch.matrix[3][1] + 1.0, ch.matrix[3][2], 2.0];
        let (lp, lc) = resolved.pick2(p, 0x8a0);
        let got = lp.iter().zip(lc.iter()).any(|(pos, col)| col[3] > 0.0 && (col[0] + col[1] + col[2]) > 0.0 && pos[3] != 0.0);
        eprintln!("guest {ci} at {:?}: kind-0 light {} ({:?})", [p[0], p[1], p[2]], got, lc[0]);
        if got { rooms_with_light += 1; }
    }
    assert!(rooms_with_light >= 2, "the party's kind-0 lights reach {rooms_with_light}/3 probed guests — the muzzle/radiator add would be missing");
}

/// G-042 (07000002 bathhouse civilian, skin 0x05000038): her SKIN texels must not be white. The
/// level's other skins may legitimately carry white patches (the towel itself, teeth) and the map's
/// one 100 %-white texture (map 32 texture 4, 32×32, 1024/1024 white rgb) is a sheen map — only
/// 98/1024 texels opaque — drawn as a faint overlay batch. The civilian's base skin textures (map
/// 32 textures 0/5/7/9) are not white, so there is no texture lookup to fix.
#[test]
fn bathhouse_civilian_skin_texels_are_not_white() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let img = nf_formats::iso9660::IsoImage::open(&iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000002.bin")).expect("archive");
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read");
    let level = nf_formats::level::Level::parse(&data).expect("parse level");
    let skin = level.skins.get(&0x05000038).expect("the bathhouse civilian's skin 0x05000038 is missing");
    let mut checked = 0usize;
    let mut sheen = 0usize;
    for mh in skin.main.iter().flatten() {
        let Some(&(m, o)) = level.by_hash.get(mh) else { continue };
        for leaf in &level.maps[m].objects[o].gfx.leaves {
            for b in &leaf.batches {
                let Some(t) = b.texture else { continue };
                if t as usize >= level.maps[m].textures.len() { continue; }
                let tex = &level.maps[m].textures[t as usize];
                let rgba = tex.frame(0).iter().flat_map(|&i| tex.palette_rgba8()[i as usize]).collect::<Vec<u8>>();
                let n = rgba.len() / 4;
                if n == 0 { continue; }
                let mut white_opaque = 0usize;
                let mut opaque = 0usize;
                for px in rgba.chunks(4) {
                    if px[3] >= 128 {
                        opaque += 1;
                        if px[0] >= 240 && px[1] >= 240 && px[2] >= 240 { white_opaque += 1; }
                    }
                }
                if white_opaque * 2 > n {
                    // A genuinely white BASE texture — over half the texels are both opaque and
                    // white (the sheen overlay is 98/1024 = 9.6 %: white rgb, alpha-masked).
                    panic!("civilian mesh {mh:08x} texture {t} is {white_opaque}/{n} opaque-white texels — investigate the lookup");
                }
                if white_opaque == n && opaque * 20 < n { sheen += 1; } // white rgb, mostly transparent: a sheen map
                checked += 1;
            }
        }
    }
    eprintln!("G-042: {checked} civilian skin textures checked, {sheen} sheen-style (white rgb, low alpha) — the base skin is not white");
    assert!(checked > 0, "the civilian's skin textures did not resolve");
}
