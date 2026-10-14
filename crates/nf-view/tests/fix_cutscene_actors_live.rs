//! fix-cutscene-actors live tests: the cut-scene actors on 07000005/06/07 — the skins the scripts
//! name (`Script_AnimStart` 0x1d8f40), whether the port resolves and draws them, and where the
//! actors end up when a scene hands back (the freeze pair event 5 / event 7, `Script_Free` 0x1da880,
//! the second-switch delete `NDrone2_PreDroneControl` 0x148db0).
//!
//! Every pinned value is read off the disc (the pack/level archives, [D]) or the decompile ([M]).
//! Opt-in via `NF_ISO`.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(archive: &str) -> nf_view::scene::Scene {
    let iso = iso().expect("NF_ISO");
    nf_view::load(&iso, archive, false).expect("level loads")
}

fn playing(scene: &nf_view::scene::Scene, hash: u32) -> bool {
    scene.sim.as_ref().unwrap().drones.scripts.players.iter().any(|p| {
        p.scripts.contains(&hash) && p.state == 2 && p.info[p.cur].as_ref().is_some_and(|s| s.play & 1 != 0)
    })
}

/// The (script, stream) → skin map of every live `Obj::Anim` of `hash`, sorted.
fn live_anims(scene: &nf_view::scene::Scene, hash: u32) -> Vec<(u8, u32, u32)> {
    let mut v: Vec<(u8, u32, u32)> = scene.sim.as_ref().unwrap().drones.scripts.objects.list.iter().flatten()
        .filter_map(|o| match o {
            nf_game::nis::Obj::Anim { skin, script, src: (h, st), .. } if *h == hash => Some((*st, *skin, *script)),
            _ => None,
        }).collect();
    v.sort_unstable();
    v
}

/// The Kimiko scene (07000007 pack 07800007.bin, 0x06000073): the script's actors are exactly the
/// three `Script_AnimStart` streams [D the pack's stream decode]:
/// stream 1 → skin 0x0500003b (script 0x060004f1), stream 2 → 0x05000015 (0x060004f2), stream 3 →
/// 0x0500000d (0x06000225, the tuxedo Bond double) — all three in 07000007's own archive [D bin-list].
/// The hand-back: event 6 (stream 7) snapshots stream 6's entity (the single key 498 at
/// [91.52, 43.8513, -31.09] [D], SP #420's base is identity [D placement dump]) and `Script_Free`
/// re-enables the party; the ch-0x48 drones (second channel = the trigger channel, param 6 = 72,
/// second tag 100 → the alert-block mode 0x15 whose type row carries 0xb4 [M 0x14c938 case 5 +
/// DroneTypeSettings[0x15].alt]) delete through `NDrone2_PreDroneControl` 0x148db0 when the freeze
/// lifts — the fireplace Kimiko drone among them [D placement 414].
#[test]
fn the_kimiko_scene_actors_and_the_handback() {
    if iso().is_none() { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load("07000007.bin");
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [94.3, 44.6, -45.9]; // TouchOnce #412's volume (out 0x48)
        for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    }
    assert!(playing(&scene, 0x0600_0073), "the Kimiko scene did not start from trigger 412");
    let anims = live_anims(&scene, 0x0600_0073);
    assert_eq!(anims, vec![(1u8, 0x0500_003b, 0x0600_04f1), (2, 0x0500_0015, 0x0600_04f2), (3, 0x0500_000d, 0x0600_0225)],
        "the Kimiko scene's live actors must be exactly the script's three Script_AnimStart streams");
    // Every actor skin resolves to a built character with the right skin (the draw path exists).
    for (_, skin, _) in &anims {
        assert!(scene.characters.iter().any(|c| c.skin.hashcode == *skin), "skin {skin:08x} has no built character");
    }
    // The frozen party: event 5 at the scene's tick 0.
    let sim = scene.sim.as_ref().unwrap();
    assert!(!sim.drones.npc_enabled, "event 5 must freeze the drones while the scene plays");
    let kimiko_drone = sim.drones.list.iter().enumerate()
        .find(|(_, d)| d.skin == 0x0500_0015 && d.present).expect("the fireplace Kimiko drone is present pre-scene");
    let kimiko_pos = kimiko_drone.1.pos;
    assert!((kimiko_pos[0] - 92.06).abs() < 0.5 && (kimiko_pos[2] + 31.65).abs() < 0.5,
        "the Kimiko drone stands at her placement, got {kimiko_pos:?}");

    // The game's skip action ends the scene; `Script_Free` kills the doubles, hands Bond back at
    // stream 6's entity (key 498) and lifts the freeze; the ch-0x48 drones delete, the rest stand.
    let hall_guards: Vec<([f32; 3], f32, f32)> = sim.drones.list.iter()
        .filter(|d| d.present && d.skin != 0x0500_0015 && [93.7, 94.8].iter().any(|x| (d.pos[0] - x).abs() < 0.6) && (d.pos[2] + 48.0).abs() < 0.6)
        .map(|d| (d.pos, d.pos[0], d.pos[2])).collect();
    assert!(hall_guards.len() >= 2, "the two hall guards near the -48 wall must be present pre-skip");
    drop(sim);
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.drones.scripts.skip_request = true;
        for _ in 0..240 { sim.step_bond(Input::default(), false, false, false); }
    }
    assert!(!playing(&scene, 0x0600_0073), "the skip must end the Kimiko scene");
    let sim = scene.sim.as_ref().unwrap();
    // The doubles are gone (Script_Free's stream pass, no keep-objects flag in 0x06000073 [D]).
    assert!(live_anims(&scene, 0x0600_0073).is_empty(), "Script_Free must delete the scene's actor doubles");
    // Bond stands at the hand-back marker (stream 6's entity, key 498).
    assert!((sim.bond.pos[0] - 91.52).abs() < 0.5 && (sim.bond.pos[1] - 43.85).abs() < 0.5 && (sim.bond.pos[2] + 31.09).abs() < 0.5,
        "Bond hands back at the stream-6 entity's key-498 spot, got {:?}", sim.bond.pos);
    // The party is re-enabled (event 7's 0x80 flag → Drone_EnableAll(1)).
    assert!(sim.drones.npc_enabled, "Script_Free's 0x80 flag must re-enable the drones");
    // The ch-0x48 drones deleted (second switch → alt state 0xb4), the untouched hall guards stand
    // exactly where they froze.
    assert!(!sim.drones.list.iter().any(|d| d.skin == 0x0500_0015 && d.present),
        "the fireplace Kimiko drone deletes through its 0x48 second switch when the freeze lifts");
    for (_, x, z) in &hall_guards {
        assert!(sim.drones.list.iter().any(|d| d.present && (d.pos[0] - x).abs() < 0.05 && (d.pos[2] - z).abs() < 0.05),
            "a hall guard left its frozen post ({x}, {z}) after the scene");
    }
}

/// The dressing scene (07000006 0x06000071 — the F3 regression's scene): the script's two actors
/// are the suit (stream with skin 0x05000007) and the tuxedo (skin 0x0500000d) [D bin-list of
/// 07000006 + models-characters §3]; both skins resolve in 07000006's archive.
#[test]
fn the_dressing_scene_actors_resolve() {
    if iso().is_none() { eprintln!("NF_ISO not set — skipping"); return };
    let scene = load("07000006.bin");
    let nis = &scene.sim.as_ref().unwrap().drones.scripts.lib[&0x0600_0071];
    let mut skins: Vec<(u8, u32)> = Vec::new();
    for (si, st) in nis.streams.iter().enumerate() {
        for cmd in st.decode() {
            if let nf_formats::nis::Cmd::Anim { skin, .. } = cmd { skins.push((si as u8, skin)); }
        }
    }
    skins.sort_unstable();
    assert_eq!(skins, vec![(1u8, 0x0500_0007), (2, 0x0500_000d)], "the dressing scene's Script_AnimStart skins [D]");
    for (_, skin) in &skins {
        let built = scene.characters.iter().any(|c| c.script.is_some_and(|(h, si)| h == 0x0600_0071 && skins.iter().any(|(s, sk)| *s == si && sk == skin)) && c.skin.hashcode == *skin);
        assert!(built, "the dressing scene's skin {skin:08x} did not build a character");
    }
}

/// The ledge scene (07000005 0x06000994): three actors [D] — Bond (0x0500007), and the two
/// guards 0x05000030 / 0x0500002e — all in 07000005's own archive.
#[test]
fn the_ledge_scene_actors_resolve() {
    if iso().is_none() { eprintln!("NF_ISO not set — skipping"); return };
    let scene = load("07000005.bin");
    let nis = &scene.sim.as_ref().unwrap().drones.scripts.lib[&0x0600_0994];
    let mut skins: Vec<(u8, u32)> = Vec::new();
    for (si, st) in nis.streams.iter().enumerate() {
        for cmd in st.decode() {
            if let nf_formats::nis::Cmd::Anim { skin, .. } = cmd { skins.push((si as u8, skin)); }
        }
    }
    skins.sort_unstable();
    assert_eq!(skins, vec![(1u8, 0x0500_0030), (3, 0x0500_002e), (14, 0x0500_0007)], "the ledge scene's Script_AnimStart skins [D] (stream 14 re-uses its stream object for Bond — the Script_AnimStart kill-and-recreate)\n");
    for (_, skin) in &skins {
        assert!(scene.characters.iter().any(|c| c.skin.hashcode == *skin), "the ledge scene's skin {skin:08x} did not build");
    }
}

/// The Drake fly-through (07000007 pack 07900007.bin, 0x06000072): the 13 guest anim skins [D] all
/// resolve and build (the user's "models in that cutscene aren't accurate" inventory check).
#[test]
fn the_drake_flythrough_guest_skins_resolve() {
    if iso().is_none() { eprintln!("NF_ISO not set — skipping"); return };
    let scene = load("07000007.bin");
    let nis = &scene.sim.as_ref().unwrap().drones.scripts.lib[&0x0600_0072];
    let mut skins: Vec<(u8, u32)> = Vec::new();
    for (si, st) in nis.streams.iter().enumerate() {
        for cmd in st.decode() {
            if let nf_formats::nis::Cmd::Anim { skin, .. } = cmd { skins.push((si as u8, skin)); }
        }
    }
    skins.sort_unstable();
    assert_eq!(skins.len(), 13, "the fly-through carries 13 Script_AnimStart streams [D]");
    let want: Vec<u32> = [0x0500_0016u32, 0x0500_0015, 0x0500_000d, 0x0500_0022, 0x0500_0019, 0x0500_0020, 0x0500_0021, 0x0500_0024, 0x0500_0023].to_vec();
    for (_, skin) in &skins {
        assert!(want.contains(skin), "unexpected fly-through skin {skin:08x}");
        assert!(scene.characters.iter().any(|c| c.skin.hashcode == *skin), "fly-through skin {skin:08x} did not build");
    }
}

/// The impact-effect script (0x060007c4): `Bullet_CollisionHandler` 0x12ce80:212-231 Script_Loads it
/// on the weapon `+0x70` 0x100000/0x200000 arms, but NO placement on any of 07000005/06/07
/// references it — the scene build must sweep it explicitly so its entity streams
/// (Laser_Blast_frame_1/2, Vertical_smoke, After_glow_test, cloud_1, smoke [D]) have instances to
/// draw; before the fix none of them built (the "models are missing" class).
#[test]
fn the_impact_effect_script_models_build() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for level in ["07000005.bin", "07000006.bin", "07000007.bin"] {
        let scene = load(level);
        let nis = &scene.sim.as_ref().unwrap().drones.scripts.lib[&0x0600_07c4];
        let mut want: Vec<(u8, u32)> = Vec::new();
        for (si, st) in nis.streams.iter().enumerate() {
            for cmd in st.decode() {
                if let nf_formats::nis::Cmd::Entity { hash, obj_type, .. } = cmd {
                    if !matches!(obj_type, 0xea | 0xeb | 0xf5 | 0xf2) { want.push((si as u8, hash)); }
                }
            }
        }
        want.sort_unstable();
        want.dedup();
        assert!(!want.is_empty(), "{level}: the impact script has drawn entity streams");
        for (si, hash) in &want {
            assert!(scene.attachments.iter().any(|a| a.script == Some((0x0600_07c4, *si, *hash))),
                "{level}: the impact-effect entity {hash:08x} (stream {si}) has no built instance — the model is missing");
        }
    }
}
