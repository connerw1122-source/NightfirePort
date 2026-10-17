//! cutscene-guard-models: the skins of script-animated cut-scene actors, through the real
//! `nf_view::load` + ScriptPlayer path.
//!
//! The game's actor create path for script anims: `Script_AnimStart` (ACTION.ELF 0x1d8f40) reads the
//! op-0x0A arguments (anim script, skin, start frame) and calls `AnimObjectNew` (0x1be910), which
//! 1. substitutes skin 0x05000090 per level [M disasm 0x1be958-0x1be9ac: `lw $v1, 0x3774($v1)` reads
//!    GameState+0xC (0x2a3774, the level id — `lw $v0, 0xc($s7)` at 0x12c070 is the same field) ==
//!    0x0700000c → 0x05000087, == 0x07000014 → 0x05000093, then `movz $s7, $v1, $v0`],
//! 2. looks the hash up in the loaded-file hashtable (`hashtable_getitem` 0x1ca5b0) — the table holds
//!    every archive entry the loader processed (`LoaderProcess` 0x1d4880 case 3/4/5 → `AnimLoadFile`
//!    0x1bdf40 registers skins/sequences/anims; nothing frees between levels), and writes the REQUESTED
//!    hash into the skin record's first word (`*puVar13 = (int)param_2` [M 0x1becd8]) — so a fresh-boot
//!    EE snapshot records exactly which skins the game asked for.
//!
//! Verified against the game's own boot snapshot [L work/ee_start_07000001.bin]: the skin records for
//! 0x05000009/17/2b/36/38/39/3a/4f carry their own hash at +0x00 (requested); every other skin record
//! still holds the malloc fill 0x98989898 (never requested).
//!
//! Opt-in via NF_ISO.

use nf_game::player::Input;

fn load(archive: &str) -> Option<nf_view::scene::Scene> {
    let iso = std::env::var("NF_ISO").ok()?;
    Some(nf_view::load(&iso, archive, false).expect("level loads"))
}

/// The anim objects the simulation created (per script stream) and their skin hashes.
fn anim_skins(scene: &nf_view::scene::Scene) -> Vec<(u32, u8, u32)> {
    scene
        .sim
        .as_ref()
        .unwrap()
        .drones
        .scripts
        .objects
        .list
        .iter()
        .flatten()
        .filter_map(|o| match o {
            nf_game::nis::Obj::Anim { skin, src, .. } => Some((src.0, src.1, *skin)),
            _ => None,
        })
        .collect()
}

/// The scene characters built for script objects: (skin hashcode, script hash, stream).
fn script_characters(scene: &nf_view::scene::Scene) -> Vec<(u32, u32, u8)> {
    scene
        .characters
        .iter()
        .filter_map(|c| c.script.map(|(h, si)| (c.skin.hashcode, h, si)))
        .collect()
}

/// 07000001's intro cut-scene plays at level start: ScriptPlayer placement #277's script A is
/// 0x06000087 with mode 1 (`SP_CreateScriptPlayer` 0x1db9a8:79-85's state 7 auto-play; the game's
/// boot snapshot shows the SP in state 2 with mode A 1 [L obj 0x018fbe60: +0x24 = 2, +0x26 = 1]).
/// Its op-0x0A entries [D 07800001.bin entry 06000087, 2120 ticks, 15 streams]:
///   stream 1: ANIM(script 0x0600055e, skin 0x0500002b)
///   stream 2: ANIM(script 0x0600055f, skin 0x05000017)
///   stream 3: ANIM(script 0x0600055d, skin 0x0500004f)
///   stream 4: ANIM(script 0x0600055c, skin 0x05000009)
/// All four are in 07000001's own archive (28 kind-3 entries; the game's boot hashtable lists them
/// [L ee_start_07000001.bin t_hashtable @0x31cbe0, m_nhti 875 @0x30d7d0]). Skin 0x05000009 is also
/// the PLAYER's skin on 0x07000001..04 [M Player_Init 0x1910f8:93-105: `case 0x7000001..0x7000004:
/// uVar3 = 0x5000009`].
#[test]
fn intro_cutscene_actors_use_the_script_skins() {
    let Some(scene) = load("07000001.bin") else { eprintln!("NF_ISO not set — skipping"); return };
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(
            sim.drones.scripts.lib.contains_key(&0x0600_0087),
            "0x06000087 (the intro cut-scene, pack 07800001.bin) must be in the script lib"
        );
        let sp = sim
            .drones
            .scripts
            .players
            .iter()
            .find(|p| p.scripts.contains(&0x0600_0087))
            .expect("a ScriptPlayer plays 0x06000087");
        assert_eq!((sp.state, sp.modes[0]), (7, 1), "the intro SP must be the mode-1 auto-play (state 7 pre-first-tick)");
    }
    let mut scene = scene;
    for _ in 0..10 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
    }
    let skins = anim_skins(&scene);
    eprintln!("anim objects after 10 ticks: {skins:08x?}");
    // The stream↔skin pairing exactly as the script names it (the port's `anim_start` order).
    for (stream, skin) in [(1u8, 0x0500_002bu32), (2, 0x0500_0017), (3, 0x0500_004f), (4, 0x0500_0009)] {
        assert!(
            skins.iter().any(|&(h, si, sk)| h == 0x0600_0087 && si == stream && sk == skin),
            "stream {stream}'s intro actor must carry the script's skin {skin:08x}, got {skins:08x?}"
        );
    }
    // The scene builds a drawable character per actor skin.
    let chars = script_characters(&scene);
    eprintln!("script characters: {chars:08x?}");
    for want in [0x0500_002bu32, 0x0500_0017, 0x0500_004f, 0x0500_0009] {
        assert!(
            chars.iter().any(|&(skin, _, _)| skin == want),
            "no scene character built for skin {want:08x} — the intro actor would not draw"
        );
    }
}

/// 07000001's gameplay guards: the placement skins are 0x05000036 ×7, 0x05000039 ×7, 0x0500003a ×5,
/// 0x0500004f ×1 [D main-map 0x1A records, param 0 of the twenty 0x0F placements] — and ONE of them
/// is a CAPTAIN: `NDrone2_DefaultInit` 0x14b300 substitutes the skin by the drone's kind when the
/// captain flag (+0x19, the difficulty-gated mode-settings property) is set — kind 1 → 0x05000038,
/// kind 2 → 0x0500002f, kind 3 → 0x05000006, kind 6 → 0x05000093, kind 7 → 0x050000b2 [M 0x14b300
/// :528-548 switch on *(u16*)(DIVars+0xD8)] — before `AnimObjectNew` (:552). The game's boot
/// snapshot shows 15 live drones wearing 36×6, 39×4, 3a×3, 4f×1 AND ONE 0x05000038 (the captain;
/// the placement skins alone cannot produce it) [L scan of obj class-2 records, obj+0xE0 → DIVars+0xE0].
/// 07000001's guards' cut-scene 0x06000086 dresses its stream-3 actor in that same 0x05000038
/// [D entry 06000086 stream 3: ANIM(06000550, 05000038)] — the captain is the guard the scene mirrors.
#[test]
fn the_captain_guard_wears_0x05000038_on_07000001() {
    let Some(scene) = load("07000001.bin") else { eprintln!("NF_ISO not set — skipping"); return };
    let sim = scene.sim.as_ref().unwrap();
    let present: Vec<(u32, bool, u16)> = sim
        .drones
        .list
        .iter()
        .filter(|d| d.present)
        .map(|d| (d.skin, d.captain, d.kind))
        .collect();
    eprintln!("present drones: {present:08x?}");
    assert_eq!(present.len(), 15, "15 drones present at start (golden drones.tsv at_start)");
    assert!(
        present.iter().any(|&(skin, captain, _)| captain && skin == 0x0500_0038),
        "exactly the game's captain skin: a present captain wearing 0x05000038, got {present:08x?}"
    );
    for skin in [0x0500_0036u32, 0x0500_0039, 0x0500_003a, 0x0500_004f] {
        assert!(
            present.iter().any(|&(s, _, _)| s == skin),
            "the placed skin {skin:08x} must still be worn by someone at start"
        );
    }
}

/// 07000001's second in-engine cut-scene (0x06000086, SP #276, mask 1 out 0x33) names the guards'
/// skins [D entry 06000086 streams 1/3/12]: (0x0600054e, 0x0500003a) + weapon 6, (0x06000550,
/// 0x05000038) + weapon 0x1c, (0x0600054f, 0x05000036) + weapon 0x1c. The game never plays it: NO
/// placement on any map of 07000001 and no script event writes switch_channels[0x33] [D scan: every
/// 0x1A record of all 54 maps + every op-0x12 event 0xD of the archive's and the pack's scripts],
/// and the boot snapshot shows its SP still in state 0 [L obj 0x018fbfc0: +0x24 = 0]. This test
/// pins that proven state: both SPs sit waiting exactly as the game's do. (Firing ch 0x33 by hand
/// does NOT start it in the port — some port system clears the channel within one tick, where the
/// game's `switch_channels` persist until written; noted as an open item in the report, kept out of
/// pass/fail because the game's own behaviour here is "never fires".)
#[test]
fn guards_cutscene_sp_waits_exactly_like_the_game() {
    let Some(scene) = load("07000001.bin") else { eprintln!("NF_ISO not set — skipping"); return };
    let sim = scene.sim.as_ref().unwrap();
    let sp = sim
        .drones
        .scripts
        .players
        .iter()
        .find(|p| p.scripts.contains(&0x0600_0086))
        .expect("SP #276 (0x06000086) exists");
    assert_eq!(sp.state, 0, "the guards' cut-scene SP waits (game [L] obj 0x018fbfc0 state 0)");
    assert_eq!((sp.mask, sp.out), (1, 0x33), "mask 1 on channel 0x33 [D placement params (5,1),(6,0x33)]");
}
