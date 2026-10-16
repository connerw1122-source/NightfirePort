//! blood-puff-render live tests: the blood puff's RENDER side, end to end.
//!
//! fix_flesh_hit_effects proved the POOL half (the 0x0200_08bc puff with the game's `Gas_Init`
//! data + the draw slots). The playtest symptom survived it: the main agent's render
//! (2026-10-16, master a4156da0) still showed nothing — so this file pins the DRAW chain through
//! the real fire path, the state the GPU actually consumes, and the two defects the task found:
//!
//! 1. `Layer::frustum_cullable`: the Effect pools never frustum-cull. Their build-time sphere is
//!    the IDENTITY-matrix centre (for the blood mesh, `BodyHit` obj.centre ≈ the world origin,
//!    radius 0.032 [D 07000005 map 10 obj 41]) while the verts are posed per frame through the
//!    attachment matrices — the game culls these objects per frame at the LIVE matrix
//!    (`View_AddObjects` 0x1e62d8 → `Vision_InView` 0x1e89e0 on obj+0x8c/obj+0x80), so a cull on
//!    the build sphere drops every puff the moment the origin leaves the frustum, wherever the
//!    puffs are posed (the same unsoundness as A-11's truck vanish).
//! 2. The GPU palette's colour slot for a live gas slot carries the pool's rgba/255 — the game's
//!    draw tint: `View_SetupRenderModes_Tweak` 0x1e5888:46-49 feeds the gas object's
//!    obj+0x103..0x106 (which `gases_smoke` 0x1ab2d0's tail rewrites from the gas datum's colour
//!    every tick, the 255→0 fade) into `psiSetTweakARGB` 0x124770, whose byte/255 floats multiply
//!    the drawn colour AND alpha. The GPU path carried only the ambient dim, so the render never
//!    faded the puff (the CPU `posed_vertices`' gas_rgba did).
//!
//! Assertions are numeric: the group's material/indices, the skin-attr matrix index, the palette
//! billboard at the puff, the colour slot, and the posed vertex positions at the puff. Opt-in via
//! `NF_ISO`.

use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn load() -> nf_view::scene::Scene {
    unsafe { std::env::set_var("NF_SILENT", "1"); }
    nf_view::load(&iso(), "07000005.bin", false).expect("load 07000005")
}

fn settle(scene: &mut nf_view::scene::Scene, ticks: u32) {
    for _ in 0..3000 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); if !scene.sim.as_ref().unwrap().disabled { break; } }
    for _ in 0..ticks { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
}

/// The camera axes of the render (lib.rs `cam_axes` over the shot's camera: pitch × π/2).
fn shot_opts(scene: &nf_view::scene::Scene) -> nf_view::scene::ViewOpts {
    let sim = scene.sim.as_ref().unwrap();
    let (yaw, pitch) = (sim.bond.yaw, sim.bond.pitch * std::f32::consts::FRAC_PI_2);
    let f = [pitch.cos() * yaw.sin(), pitch.sin(), pitch.cos() * yaw.cos()];
    let r = [-yaw.cos(), 0.0, yaw.sin()];
    let dot = r[0] * f[0] + r[1] * f[1] + r[2] * f[2];
    let mut u = [r[1] * f[2] - r[2] * f[1], r[2] * f[0] - r[0] * f[2], r[0] * f[1] - r[1] * f[0]];
    let l = (u[0] * u[0] + u[1] * u[1] + u[2] * u[2]).sqrt().max(1e-9);
    u = [u[0] / l, u[1] / l, u[2] / l];
    let _ = dot;
    let mut opts = nf_view::scene::ViewOpts::default();
    opts.right = r;
    opts.up = u;
    opts
}

/// Fire at the first live guard's torso through the real path; returns (scene, puff, pool index).
fn fire_at_a_guard() -> (nf_view::scene::Scene, nf_game::gas::Gas, usize) {
    let mut scene = load();
    settle(&mut scene, 60);
    let (gi, guard_pos) = {
        let sim = scene.sim.as_ref().unwrap();
        let yaw = sim.bond.yaw;
        let fwd = [yaw.sin(), 0.0, yaw.cos()];
        let spot = [sim.bond.pos[0] + fwd[0] * 5.0, sim.bond.pos[1], sim.bond.pos[2] + fwd[2] * 5.0];
        let gi = sim.drones.list.iter().position(|d| d.present && d.enabled && !d.corpse && d.health > 0.0)
            .expect("07000005 has a live guard");
        (gi, spot)
    };
    scene.sim.as_mut().unwrap().drones.list[gi].pos = guard_pos;
    for _ in 0..3 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let (chest, hp0) = {
        let sim = scene.sim.as_ref().unwrap();
        let data = nf_game::drone::Data {
            tables: &sim.tables, props: &sim.props, scripts: &sim.scripts, weapons: &sim.weapons,
            world: &sim.world, level: sim.level, sequences: &sim.seqs, skins: &sim.skins,
            skeletons: &sim.skeletons, nav: &sim.nav, covers: &sim.covers, tuning: &sim.tuning,
            hitboxes: &sim.hitboxes, volumes: &sim.volumes, sfx: sim.sfx.as_ref(), models: &sim.models,
            surfaces: sim.surfaces.as_ref(), voices: sim.voices.as_deref(), difficulty: sim.drones.difficulty,
        };
        let boxes = sim.drones.list[gi].hit_boxes(&data);
        let torso = boxes.iter().find(|b| b.part == 4).expect("the torso box");
        ([torso.m[3][0], torso.m[3][1], torso.m[3][2]], sim.drones.list[gi].health)
    };
    {
        let sim = scene.sim.as_mut().unwrap();
        let d = [chest[0] - sim.bond.pos[0], chest[1] - sim.bond.head()[1], chest[2] - sim.bond.pos[2]];
        let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-6);
        sim.bond.yaw = d[0].atan2(d[2]);
        sim.bond.pitch = (d[1] / l).asin() / std::f32::consts::FRAC_PI_2;
    }
    let mut pool_idx = None;
    for _ in 0..40 {
        scene.sim.as_mut().unwrap().step_bond(Input { fire: true, ..Input::default() }, true, false, false);
        pool_idx = scene.sim.as_ref().unwrap().drones.gas.puffs.iter().position(|p| p.mesh == 0x0200_08bc);
        if pool_idx.is_some() { break; }
        if scene.sim.as_ref().unwrap().drones.list[gi].health < hp0 { break; }
    }
    let _ = pool_idx.expect("no 0x0200_08bc puff in the gas pool after firing at a guard (the flesh arm never ran)");
    // Age the puff so its live alpha has left the 255 seed (the tint assertion must discriminate
    // the fixed colour slot from the old ambient 1.0): `gases_smoke` 0x1ab2d0's tail decays the
    // colour every tick (the flesh arm's end colour 0xffffff00 → alpha −255/60 per tick).
    for _ in 0..6 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let pi = scene.sim.as_ref().unwrap().drones.gas.puffs.iter().position(|p| p.mesh == 0x0200_08bc)
        .expect("the blood puff died before its 60-tick life");
    let puff = scene.sim.as_ref().unwrap().drones.gas.puffs[pi].clone();
    assert!(puff.rgba[3] < 255.0, "the aged puff's alpha must have left the seed 255, got {}", puff.rgba[3]);
    (scene, puff, pi)
}

/// THE draw chain: the pool puff's slot rides a real material with indices, its verts' skin attrs
/// index the palette slot that carries the billboard AT the puff, and the colour slot carries the
/// game's obj+0x103..0x106 tint. A failure here is the "the puff is in the pool but nothing
/// draws" symptom, independent of any GPU.
#[test]
fn the_blood_puffs_draw_chain_is_live() {
    let (scene, puff, pi) = fire_at_a_guard();
    let sim = scene.sim.as_ref().unwrap();

    // The slot: the gas attachment whose slot index reads THIS pool entry (mesh-filtered).
    let ai = scene.attachments.iter().position(|a| matches!(a.gas, Some((s, 0x0200_08bc)) if s as usize == pi))
        .expect("no gas draw slot reads the blood puff's pool index");

    // The BodyHit object's texture key from the archive (by_hash is first-wins, so the scene's
    // merged maps never shift it).
    let iso_img = nf_formats::iso9660::IsoImage::open(iso()).expect("iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&iso_img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000005.bin")).expect("07000005");
    let archive = iso_img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("archive");
    let level = nf_formats::level::Level::parse(&archive).expect("level");
    let &(bm, bo) = level.by_hash.get(&0x0200_08bc).expect("the blood mesh resolves in 07000005");
    let blood_obj = &level.maps[bm].objects[bo];
    let tex = blood_obj.gfx.leaves.iter().flat_map(|l| &l.batches).find_map(|b| b.texture).expect("the blood mesh's texture");
    let key = Some((bm, tex));

    // 1. The group's material: Effect, the BodyHit texture, one strip per slot, the mesh's own flags.
    let mats: Vec<usize> = scene.materials.iter().enumerate()
        .filter(|(_, m)| m.layer == nf_view::scene::Layer::Effect && m.key == key).map(|(i, _)| i).collect();
    assert!(!mats.is_empty(), "no Effect material for the blood mesh's texture {key:?} — the gas slots emitted nothing into the draw groups");
    let &mi = mats.iter().find(|&&i| scene.materials[i].index_count >= (nf_view::scene::GAS_DRAW_SLOTS * 6 * 3) as u32)
        .expect("the gas group's material carries fewer than one strip per slot");
    let m = &scene.materials[mi];
    assert_eq!(m.draw.pass, 5, "the gas group draws in the effects pass");
    assert!(m.index_count == (nf_view::scene::GAS_DRAW_SLOTS * 6 * 3) as u32, "the gas group's index count = 3 indices × 6 tris per slot, got {}", m.index_count);

    // 2. The slot's verts' skin attrs index the attachment's palette slot.
    let (layout, attrs) = scene.skin_layout();
    let slot_char_verts: Vec<usize> = scene.char_verts.iter().enumerate()
        .filter(|(_, cv)| cv.attachment == Some(ai as u32)).map(|(i, _)| i).collect();
    assert_eq!(slot_char_verts.len(), 8, "the blood slot's strip carries 8 verts");
    for &i in &slot_char_verts {
        let a = &attrs[i];
        assert_eq!(a.a, layout.attach_base + ai as u32, "the gas vert's palette matrix slot");
        assert_eq!(a.b, layout.attach_base + ai as u32, "the gas vert's second matrix slot");
    }

    // 3. The palette slot: the billboard AT the puff with the shot's camera axes.
    let opts = shot_opts(&scene);
    let (pal, cols) = scene.skin_palette(&layout, None, 240.0, opts, None);
    let pm = pal[(layout.attach_base + ai as u32) as usize];
    let tr = (pm[3][0] - puff.pos[0]).powi(2) + (pm[3][1] - puff.pos[1]).powi(2) + (pm[3][2] - puff.pos[2]).powi(2);
    assert!(tr < 0.25, "the gas slot's palette matrix sits {:.2} from the puff (the billboard never posed)", tr.sqrt());
    let row_len = |r: [f32; 4]| (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
    let (r0, r1, r2) = (row_len(pm[0]), row_len(pm[1]), row_len(pm[2]));
    assert!((r0 - puff.scale).abs() < 1e-3 && (r1 - puff.scale).abs() < 1e-3 && (r2 - puff.scale).abs() < 1e-3,
        "the billboard's basis rows carry the puff's scale {} (got {r0}, {r1}, {r2})", puff.scale);

    // 4. THE colour fix: the slot's colour = the pool's live rgba/255 (the game's TAmbient tint,
    //    alpha included — the fade).
    let c = cols[(layout.col_att + ai as u32) as usize];
    for k in 0..4 {
        let want = puff.rgba[k] / 255.0;
        assert!((c[k] - want).abs() < 1.0 / 255.0, "the gas slot's colour channel {k} = {} want {want} (obj+0x{} tint)", c[k], 0x103 + k);
    }

    // 5. The posed verts (the render-side assertion the task asked for): the slot's 8 verts land
    //    at the puff, not at the build-time identity position.
    let posed = scene.posed_vertices(None, 240.0, opts);
    for &i in &slot_char_verts {
        let p = posed[i].pos;
        let d = ((p[0] - puff.pos[0]).powi(2) + (p[1] - puff.pos[1]).powi(2) + (p[2] - puff.pos[2]).powi(2)).sqrt();
        assert!(d < 0.5, "the blood puff's posed vertex sits {d:.2} from the puff — the draw contributes no pixels there");
    }
    let _ = sim;
}

/// The cull rule: the Effect pools never frustum-cull (their build-time sphere is the
/// identity-matrix origin), the World bulk always does, the rest only under the NF_CULL override.
#[test]
fn the_effect_pools_never_frustum_cull() {
    use nf_view::scene::Layer;
    assert!(Layer::World.frustum_cullable(false), "the static World bulk culls always");
    assert!(Layer::World.frustum_cullable(true));
    assert!(!Layer::Effect.frustum_cullable(false), "the Effect pools must not cull by default");
    assert!(!Layer::Effect.frustum_cullable(true), "the Effect pools must not cull even under NF_CULL — their build sphere is the identity origin, the verts pose per frame");
    assert!(!Layer::Character.frustum_cullable(false));
    assert!(Layer::Character.frustum_cullable(true), "NF_CULL keeps exercising the Character layer's build centres (real placements)");
}
