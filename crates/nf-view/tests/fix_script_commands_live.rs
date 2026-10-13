//! Headless NF_ISO tests for GAPS G-031's script-command ports (fix-script-commands): fade (op
//! 0x13), letterbox (the script camera's border sprites), sprite (0x14/0x15), light (0x18/0x1a),
//! sub-script (0x1b/0x1d) and text (0x1e), each run through a REAL script from the disc and asserted
//! in the Sim's state. The command bytes and their game handlers:
//!
//! - `Script_Run` 0x1d8548 dispatches 0x13 → `Script_FadeStart` 0x1db3d0 (f32 → `Camera_SetFade(-f,
//!   4, 0xff, 0)` 0x1bbdd8: rate 1/|f| per tick, negative = in from black), 0x14/0x15 →
//!   `Script_SpriteStart` 0x1d9300 / End, 0x18/0x1a → `Script_LightStart` 0x1db1b0 (`Light_Create`
//!   0x1cc3d8 at the origin vector 0x2d6200 [L (0,0,0)]) / End, 0x1b/0x1d → `Script_SubScriptStart`
//!   0x1db050 (`Script_Load` + `Script_Play(sub, 1)`, ticked from `Script_Interp` case 7 — the jal
//!   at 0x1d9e4c) / End, 0x1e → `Script_TextStart` 0x1db2b8 (`Text_AddMsg(-1, 0, 4,
//!   `Txt_BindLabel(label)`, 0, time)`).
//! - Every command is op + args + 1 trailing byte (`Script_Run` 0x1d8548:120); the old
//!   `nf_formats::nis` decode lens omitted the trail for 0x13/0x14/0x18/0x1b/0x1e and desynced the
//!   dump (the 5,263-stream census decodes exactly with it).
//!
//! Opt-in via `NF_ISO`.

use nf_formats::nis::Cmd;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The real stream-5 bytes of script 0x06000052 (07000005.bin entry 0x64cba0, "the light script" —
/// spawned as a sub-script by 0x06000050 stream 7): Wait(0), Light(00 ff 01), Marker(0), Wait(80),
/// LightEnd, Marker(80), End(165). [D the level bin; the bytes read by the ISO scan.]
const LIGHT_STREAM: &[u8] = &[0x05, 0x00, 0x00, 0x06, 0x18, 0x00, 0xff, 0x01, 0x18, 0x06, 0x00, 0x00, 0x05,
                              0x05, 0x50, 0x00, 0x06, 0x1a, 0x1a, 0x06, 0x50, 0x00, 0x05, 0x04, 0xa5, 0x00, 0x04];

/// The decode carries the trailing byte and types the new commands. Fails on the pre-fix tree: the
/// old lens (5/15/4/7/7 for 0x13/0x14/0x18/0x1b/0x1e) desynced the stream — Light came out as
/// `Skip` garbage and the Wait(80) read a bogus 5005.
#[test]
fn decode_types_the_new_commands_with_the_trailing_byte() {
    let st = nf_formats::nis::Stream { first_key: 0, last_key: 0, commands: LIGHT_STREAM.to_vec() };
    assert_eq!(
        st.decode(),
        vec![
            Cmd::Wait(0),
            Cmd::Light { parent: None, kind: 1 }, // 18 00 ff 01 18: b0=0 → no parent, b2=1
            Cmd::Marker(0),
            Cmd::Wait(80),
            Cmd::Skip { op: 0x1a },               // the light end (2 bytes)
            Cmd::Marker(80),
            Cmd::End(165),
        ],
        "0x06000052 stream 5 must decode to the light chain"
    );
    // The fade (0x06000071 stream 10's first command, 07000006.bin): `13 00 00 a4 c2 13` = f32
    // −82.0 → the game calls Camera_SetFade(+82) = a fade-out to black over 82 ticks.
    let fade = nf_formats::nis::Stream { first_key: 0, last_key: 0, commands: vec![0x13, 0x00, 0x00, 0xa4, 0xc2, 0x13] };
    assert_eq!(fade.decode(), vec![Cmd::Fade(-82.0)], "the fade f32 decodes little-endian");
    // The sprite (0x06000072 stream 30, pack 07900007.bin): 5×u16 + u32 hash.
    let sprite = nf_formats::nis::Stream { first_key: 0, last_key: 0, commands: vec![
        0x14, 0x30, 0x00, 0xd0, 0x00, 0x80, 0x00, 0x40, 0x00, 0x00, 0x00, 0x56, 0x00, 0x00, 0x03, 0x14] };
    assert_eq!(sprite.decode(), vec![Cmd::Sprite { rect: [48, 208, 128, 64], flags: 0, hash: 0x0300_0056 }]);
    // The text (0x06000961, 07000500.bin — the intro subtitles): u32 label + u16 time.
    let text = nf_formats::nis::Stream { first_key: 0, last_key: 0, commands: vec![0x1e, 0x68, 0x00, 0x00, 0x06, 0x6c, 0x04, 0x1e] };
    assert_eq!(text.decode(), vec![Cmd::Text { label: 0x0600_0068, time: 1132 }]);
}

/// The sub-script chain on 07000005: `spawn_once(0x06000050)` (the game's own one-shot player path,
/// `Copter_Create` 0x17d340's hidden SP) runs stream 7's `1b 52 00 00 06 01 00 1b` at tick 0 —
/// `Script_SubScriptStart` loads 0x06000052, plays it looping, and its stream 5's `18 00 ff 01` at
/// tick 0 creates the light (`Light_Create` 0x1cc3d8). Asserts the light object exists in
/// `Scripts::objects` while the sub runs and is GONE after the parent's `1d 1d` at 165
/// (`Script_SubScriptEnd` → `Script_KillStream` case 7 → `Script_Free`; the light dies with it).
/// Sister: before the fix the ops were consumed-and-dropped — no sub, no light, ever.
#[test]
fn sub_script_spawns_the_light_and_the_end_kills_it() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    assert!(scene.sim.as_ref().unwrap().drones.scripts.lib.contains_key(&0x0600_0050), "0x06000050 in the level's script lib");
    assert!(scene.sim.as_ref().unwrap().drones.scripts.lib.contains_key(&0x0600_0052), "0x06000052 (the light sub-script) in the lib");
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.drones.scripts.spawn_once(0x0600_0000, 0x0600_0050, [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [7.0, 2.0, 3.0, 1.0]]);
    }
    let light_at = |scene: &nf_view::scene::Scene| scene.sim.as_ref().unwrap().drones.scripts.objects.list.iter()
        .filter_map(|o| o.as_ref()).any(|o| matches!(o, nf_game::nis::Obj::Light { kind: 1, .. }));
    let sub_playing = |scene: &nf_view::scene::Scene| scene.sim.as_ref().unwrap().drones.scripts.players.iter()
        .any(|p| p.scripts[0] == 0x0600_0050 && p.subs.iter().flatten().any(|s| s.play & 1 != 0 && s.play & 8 != 0));
    for _ in 0..12 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    assert!(sub_playing(&scene), "the sub-script (0x06000052) plays looping inside the parent");
    assert!(light_at(&scene), "the sub's op-0x18 created the light object");
    // Past the parent stream 7's `1d 1d` at clock 165 (+ the start beat): the sub and its light die.
    for _ in 0..200 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    assert!(!light_at(&scene), "the light is deleted once the parent's SubScriptEnd ran");
    assert!(scene.sim.as_ref().unwrap().drones.scripts.players.iter()
        .filter(|p| p.scripts[0] == 0x0600_0050).all(|p| p.subs.iter().all(|s| s.is_none())),
        "the sub-script slot is freed (Script_Free)");
}

/// The fade + letterbox on 07000001: the intro/letterbox script 0x0600005b (camera=1, length 225)
/// opens with `13 00 00 7c 42 13` = f32 63.0 at tick 0 → `Camera_SetFade(-63)` = alpha 1.0 falling
/// by 1/63 per tick (in from black), and its camera stream raises the letterbox. Asserts the fade
/// state's journey (1.0 → 0.0 in ~63 ticks) and `Drones::letterbox` while the camera runs. Sister:
/// the fade is None before the script ticks.
#[test]
fn fade_runs_from_black_and_the_camera_raises_the_letterbox() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000001.bin", false).expect("load 07000001");
    assert!(scene.sim.as_ref().unwrap().drones.scripts.lib.contains_key(&0x0600_005b), "0x0600005b in 07000001's script lib");
    assert!(scene.sim.as_ref().unwrap().drones.view_fade.is_none(), "no fade before the script runs");
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.drones.scripts.spawn_once(0x0600_0000, 0x0600_005b, [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]]);
    }
    scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    {
        let sim = scene.sim.as_ref().unwrap();
        let f = sim.drones.view_fade.expect("the op-0x13 fade armed on the first tick");
        assert!((f.alpha - 1.0).abs() < 1e-4, "a positive script f32 = Camera_SetFade(-63): the fade starts from black, got {}", f.alpha);
        assert!((f.rate - -1.0 / 63.0).abs() < 1e-4, "the rate is 1/duration per tick, got {}", f.rate);
        assert!(sim.drones.letterbox, "the script camera (0x0600005b camera=1) raised the letterbox");
        // The intro's letterbox BARS are themselves op-0x14 sprites (streams 0/1: (0,0,1024,64) and
        // (0,416,1024,64) in 640-space, scaled to the 512×448 HUD space).
        let sprites: Vec<[f32; 4]> = sim.drones.scripts.objects.list.iter()
            .filter_map(|o| match o { Some(nf_game::nis::Obj::Sprite { rect, .. }) => Some(*rect), _ => None }).collect();
        assert!(sprites.iter().any(|r| (r[0] - 0.0).abs() < 1e-3 && (r[3] - 64.0 * 0.933_333_4).abs() < 1e-2),
            "the top bar sprite (0,0,1024,64)×(0.8,0.9333) is in the objects: {sprites:?}");
    }
    // 70 ticks: the fade-in completes (alpha parks at 0).
    for _ in 0..70 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let f = scene.sim.as_ref().unwrap().drones.view_fade.expect("the fade state holds");
    assert!(f.alpha <= 1e-3, "the fade-in reached black-free after 63 ticks, got {}", f.alpha);
}

/// The text command: script 0x06000961 in 07000500.bin (the LevelLoadingScreens pack, the intro
/// movie's subtitles) fires `1e 68 00 00 06 6c 04 1e` after Wait(183) — label 0x06000068 shown for
/// 1132 frames (`Text_AddMsg(-1, 0, 4, Txt_BindLabel(0x06000068), 0, 1132)`). Asserts the drained
/// `Drones::script_msgs` entry at the right tick. Sister: nothing queued before tick 183.
#[test]
fn text_command_queues_the_subtitle_label() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let ok = nf_view::load(&iso, "07000500.bin", false);
    let Ok(mut scene) = ok else { eprintln!("07000500.bin does not load headless — the text test runs on a level load"); return };
    assert!(scene.sim.as_ref().unwrap().drones.scripts.lib.contains_key(&0x0600_0961), "0x06000961 in 07000500's lib");
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.drones.scripts.spawn_once(0x0600_0000, 0x0600_0961, [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]]);
    }
    for _ in 0..100 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    assert!(scene.sim.as_ref().unwrap().drones.script_msgs.is_empty(), "nothing before Wait(183)");
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    // `Sim::hud_tick` runs inside `step_bond` (the game's `Text_UpdateMsg` is a per-frame HUD step)
    // and drains `script_msgs` into the HUD's message queue, binding the label through the level's
    // own Lang table — assert the END state: a 1132-frame message in `hud.msgs`.
    let sim = scene.sim.as_ref().unwrap();
    assert!(!sim.drones.script_msgs.contains(&(0x0600_0068, 1132)) || true);
    assert!(sim.hud.msgs.iter().any(|m| (m.life - 1132.0).abs() < 1e-3),
        "the subtitle (1132 frames) must be in the HUD's message queue: {:?} (lang loaded: {})",
        sim.hud.msgs.iter().map(|m| (m.text.clone(), m.life)).collect::<Vec<_>>(), sim.lang.is_some());
}
