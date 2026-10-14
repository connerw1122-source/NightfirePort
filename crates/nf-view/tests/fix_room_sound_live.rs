//! fix-room-sound live tests: the room environment (`Sound_UpdateListeners` 0x1dd278 →
//! `SFXSetEnvironment(cel s16 +0x98, underwater = (cel+0x3c & 0x10000000) || env >= 20)` from the
//! active viewer's cel) and the map parse that feeds it (`parseentity_fixup_entity` 0x1d00c0: class
//! 0xc023 → +0x98 = 0x9898, the −26472 underwater sentinel; class 0xc047 → +0x98 = placement param 9,
//! `level_tag+0x50` as a raw u16; the 0x10000000 flags bit = param 3 ≠ 0, both classes).
//!
//! The pinned cross-check is 07000005 [L work/ee_start_05.bin]: 19 room cels, all flag word 0xc047 —
//! 15 cels with flags dword 0x0804c047 carrying env 0, 4 cels with flags dword 0x1804c047 carrying
//! env 40 (0x28). Before the fix `sound_tick` hardcoded `drv.set_environment(0, 0)` (the G-016
//! stand-in), so an underwater room stayed dry and un-reverbed.
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// A bare output-less sound stack: the driver is what the assertions watch.
fn bare_sound() -> nf_audio::sfx::Sound {
    nf_audio::sfx::Sound {
        driver: Default::default(),
        output: None,
        music_files: None,
        music: None,
        music_pending: Vec::new(),
    }
}

/// The parse keeps the cel env value and the underwater flag: 07000005's 19 room cels split
/// 15 × (env 0, dry) / 4 × (env 40, underwater bit) [L ee_start_05.bin].
#[test]
fn the_cel_parse_keeps_the_room_environment_07000005() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let dry = scene.cels.iter().filter(|c| c.env == 0 && !c.underwater).count();
    let underwater = scene.cels.iter().filter(|c| c.env == 40 && c.underwater).count();
    assert_eq!(scene.cels.len(), 19, "the level's room-cel count [L ee_start_05.bin]");
    assert_eq!(dry, 15, "the 0x0804c047 cels: env 0, param 3 clear [L]");
    assert_eq!(underwater, 4, "the 0x1804c047 cels: env 40, param 3 set [L]");
    // Sister: nothing else — every cel is one of the two pinned kinds.
    assert_eq!(dry + underwater, scene.cels.len(), "no cel outside the two pinned kinds");
}

/// `sound_tick` feeds the active viewer's cel environment to `SFXSetEnvironment`: the spawn cel's
/// value at load, and the env-40 cel's (40, underwater) once Bond stands in one — the game sends it
/// every frame with no change detection [M 0x1dd4b0-0x1dd4fc]. A cel-less viewer sends nothing (the
/// driver keeps its previous targets); `env >= 20` alone forces the underwater argument.
#[test]
fn sound_tick_feeds_the_cel_environment_to_the_driver() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let mut snd = bare_sound();
    let idx40 = scene.cels.iter().position(|c| c.env == 40).expect("an env-40 cel exists");

    {
        let sim = scene.sim.as_mut().unwrap();
        // At the spawn: whatever cel holds Bond's eye drives the send. The IOP maps the call as
        // `SFXSetEnvironment @0x0db4`: reverb target = clamp(env, 0, 100) (the 0x9898 sentinel → 0 +
        // its SFXUnderWater flag), and the EE's underwater argument → SFXNewIndoorsVolume 50/100.
        nf_view::sound_tick(&mut snd, sim);
        let (reverb, _underwater, indoors) = snd.driver.environment();
        match sim.cel_at(sim.bond.head()) {
            Some(i) => {
                let c = &sim.cels[i];
                assert_eq!(reverb, c.env as i32, "the spawn cel's env reaches the driver");
                assert_eq!(indoors, if c.underwater || c.env >= 20 { 50 } else { 100 },
                    "the underwater argument = flag || env >= 20 (the IOP's indoors volume)");
            }
            None => assert_eq!((reverb, indoors), (0, 100), "a cel-less viewer sends nothing: the driver's fresh state"),
        }
        // Stand inside the env-40 cel: a point whose eye-position resolves to it (the bbox centre can
        // lose find_cel's smallest-volume tie-break to an overlapping dry cel — scan until it wins).
        let (cmin, cmax) = { let c = &scene.cels[idx40]; (c.min, c.max) };
        let off = { let h = sim.bond.head(); let p = sim.bond.pos; [h[0] - p[0], h[1] - p[1], h[2] - p[2]] };
        let mut placed = false;
        'scan: for fx in [0.5, 0.25, 0.75, 0.4, 0.6] {
            for fy in [0.5, 0.3, 0.7] {
                let p = [cmin[0] + (cmax[0] - cmin[0]) * fx,
                         cmin[1] + (cmax[1] - cmin[1]) * fy,
                         cmin[2] + (cmax[2] - cmin[2]) * fx];
                if sim.cel_at(p) == Some(idx40) {
                    sim.bond.pos = [p[0] - off[0], p[1] - off[1], p[2] - off[2]];
                    placed = true;
                    break 'scan;
                }
            }
        }
        assert!(placed, "a point inside the env-40 cel resolves to it");
        assert_eq!(sim.cel_at(sim.bond.head()), Some(idx40), "the teleported eye is in the env-40 cel");
        nf_view::sound_tick(&mut snd, sim);
        let (reverb, _underwater, indoors) = snd.driver.environment();
        assert_eq!(reverb, 40, "the env-40 cel's value reaches the driver (pre-fix: the hardcoded 0)");
        assert_eq!(indoors, 50, "the underwater argument (env 40 >= 20 and the flag set) → the muffled mix");
    }
}
