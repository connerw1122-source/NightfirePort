//! cutscene-audio-sync: the level start/end cut-scene audio measurement (docs/reports/
//! cutscene-audio-sync.md). For each voice line / music cue the script starts — stream sounds
//! (op 0x16) and anim op-1 voice lines — this logs the script tick it fired on and the sound's
//! real length from the disc (decoded PCM / pitch, or the STREAMS.BIN entry), so the drift
//! (if any) is a number, not a guess.
//!
//! Game rules the numbers test [M]:
//! - `Script_Update` 0x1da998:33 adds FRAME_RATE_MUL (global 0x30D0D8 = 60/FRAME_RATE [M
//!   GS_SetRefreshRate 0x17d2b8]) per update — 1.0 at 60 Hz; the 30 fps arm doubles it
//!   (`main` 0x109080:53-61 forces skip 2 while a background FMV plays).
//! - `Script_Run` 0x1d8548:27-30 fires every command whose wait-until the stream clock has
//!   reached, synchronously, catch-up within the same pass.
//! - `Script_SoundStart` 0x1d8820 starts the sound on that tick (the EE batches to the IOP;
//!   audible ≤ 1 vblank later [M SFXUpdate 0x1156c0]).
//! - The facial/body anim op-1 line fires in `AnimProcessScriptCmds` 0x1c1b38 case 1 when the
//!   sAnimScript frame counter (rate·FRAME_RATE_MUL per frame, AnimScriptTick 0x1c2630:61)
//!   crosses the authored frame.
//!
//! Headless, numbers only, NF_SILENT. Opt-in via NF_ISO.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// A bare output-less sound stack (the cutscene_skip_audio precedent) — banks and streams load,
/// nothing reaches a device.
fn bare_sound(scene: &mut nf_view::scene::Scene) -> nf_audio::sfx::Sound {
    let mut snd = nf_audio::sfx::Sound {
        driver: Default::default(),
        output: None,
        music_files: None,
        music: None,
        music_pending: Vec::new(),
    };
    if let Some((slot, bank, sbf)) = scene.sound_bank.take() { snd.driver.load_bank(slot, bank, sbf); }
    if let Some((lut, bin)) = scene.streams.take() { snd.driver.load_streams(lut, std::sync::Arc::new(bin)); }
    snd
}

fn carry(on: &[u32]) -> nf_game::player::RamSave {
    let mut ch = nf_game::triggers::Channels::default();
    for &c in on { ch.set(c, 1, 0); }
    nf_game::player::RamSave::capture(&Default::default(), 0, &nf_game::pickup::Inventory::level_start(0x73, 34), &ch)
}

/// The sound's length in 60 Hz ticks from the disc [D]: a bank sample (PCM bytes → 2 samples/byte,
/// SPU pitch 0x1000 = 48 kHz) or a STREAMS.BIN entry (FileRef < 0 plays entry −1 − FileRef).
fn len_ticks(bank: &Option<(usize, nf_formats::soundbank::Bank, Vec<u8>)>, streams: &Option<(Vec<[u32; 4]>, Vec<u8>)>, id: u32) -> Option<f32> {
    let (_, b, sbf) = bank.as_ref()?;
    let fx = b.effect(id)?;
    let e = nf_formats::soundbank::Effect::parse(fx).ok()?;
    let sub = e.subs.first()?;
    if sub.file < 0 {
        let (lut, bin) = streams.as_ref()?;
        let entry = *lut.get((-1 - sub.file) as usize)?;
        let data = nf_formats::soundbank::stream_data(bin, entry).ok()?;
        Some(data.len() as f32 * 2.0 / 48000.0 * 60.0)
    } else {
        let h = b.samples.get(sub.file as usize)?;
        let rate = 48000.0 * (h.pitch as f32 / 4096.0);
        Some(h.size as f32 * 2.0 / rate * 60.0)
    }
}

/// Run one scenario, logging every sound start (tick, id, disc length) and the playing SPs'
/// clocks at that tick. `walk`: forward-input ticks before idling (the intro plates).
fn run(tag: &str, level: &str, save: Option<&nf_game::player::RamSave>, walk: u32, ticks: u32, stand_at: Option<[f32; 3]>) {
    let Some(iso) = iso() else { return };
    let mut scene = match save {
        Some(s) => nf_view::load_with_carry(&iso, level, false, Some(s)).expect("load"),
        None => nf_view::load(&iso, level, false).expect("load"),
    };
    let bank = scene.sound_bank.take();
    let streams = scene.streams.take();
    let mut snd = bare_sound(&mut scene);
    if let Some((slot, ref b, ref s)) = bank {
        let b2 = nf_formats::soundbank::Bank { samples: b.samples.clone(), effects: b.effects.clone(), sfx: b.sfx.clone() };
        snd.driver.load_bank(slot, b2, s.clone());
    }
    if let Some((ref lut, ref bin)) = streams {
        snd.driver.load_streams(lut.clone(), std::sync::Arc::new(bin.clone()));
    }
    if let Some(at) = stand_at {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [at[0], at[1] + 1.0, at[2]];
    }
    eprintln!("[{tag}] bond spawn {:?}", scene.sim.as_ref().unwrap().bond.pos);
    // Per-tick multiset diff of the driver's live items: an increase = a sound started this
    // tick, a decrease = a sound ended this tick (the driver removes ended items in the same
    // update). sound_tick consumes the EE command queue itself (lib.rs take_commands), so the
    // item list is the observable.
    use std::collections::BTreeMap;
    let mut prev: BTreeMap<u32, i32> = BTreeMap::new();
    for t in 0..ticks {
        let input = if t < walk { let mut i = Input::default(); i.forward = 1.0; i } else { Input::default() };
        scene.sim.as_mut().unwrap().step_bond(input, false, false, false);
        nf_view::sound_tick(&mut snd, scene.sim.as_mut().unwrap());
        let mut now: BTreeMap<u32, i32> = BTreeMap::new();
        for (id, del, _, _, _) in snd.driver.items_probe() { if !del { *now.entry(id).or_insert(0) += 1; } }
        let mut ids: Vec<u32> = now.keys().chain(prev.keys()).copied().collect();
        ids.sort_unstable(); ids.dedup();
        for id in ids {
            let (a, b) = (now.get(&id).copied().unwrap_or(0), prev.get(&id).copied().unwrap_or(0));
            if b > a {
                let clocks: Vec<String> = scene.sim.as_ref().unwrap().drones.scripts.players.iter().enumerate()
                    .filter_map(|(i, sp)| sp.info[sp.cur].as_ref().filter(|s| s.play & 1 != 0).map(|s| format!("sp{i}:{:.0}", s.clock)))
                    .collect();
                eprintln!("[{tag}] END   t{t} id {id:#x} clocks {}", clocks.join(","));
            }
            if a > b {
                let clocks: Vec<String> = scene.sim.as_ref().unwrap().drones.scripts.players.iter().enumerate()
                    .filter_map(|(i, sp)| sp.info[sp.cur].as_ref().filter(|s| s.play & 1 != 0).map(|s| format!("sp{i}:{:.0}", s.clock)))
                    .collect();
                eprintln!("[{tag}] START t{t} id {id:#x} len {:?} clocks {}",
                    len_ticks(&bank, &streams, id).map(|l| format!("{l:.0}")), clocks.join(","));
            }
        }
        prev = now;
        if scene.sim.as_ref().unwrap().drones.level_exit.is_some() { eprintln!("[{tag}] level exit at t{t}"); break; }
    }
    eprintln!("[{tag}] done");
}

#[test]
fn probe_cutscene_audio_starts() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    unsafe { std::env::set_var("NF_SILENT", "1"); }
    // A. The control: 07000002's boot intro (0x0600_0084) auto-runs on a plain load.
    run("02-intro", "07000002.bin", None, 0, 1700, None);
    // B. 07000006's truck-arrival intro (the carried truck-ride entry, start #496).
    run("06-intro", "07000006.bin", Some(&carry(&[0x1d, 0x45, 0x70])), 0, 3300, None);
    // C. 07000008's station intro: SP #294 (0x06000077, 3880 ticks) on plate #237, ~3.5 m from the spawn.
    run("08-intro", "07000008.bin", None, 400, 4600, None);
    // D. 07000006's outro: the dressing scene 0x06000071 (SP #810, 1129 ticks) via LoadPoint #746.
    let lp = {
        let mut scene = nf_view::load_with_carry(&iso().unwrap(), "07000006.bin", false, Some(&carry(&[0x1d, 0x45, 0x70]))).expect("load");
        scene.sim.as_ref().unwrap().drones.triggers.iter().find(|t| t.out == 0x83).expect("LoadPoint").centre()
    };
    run("06-outro", "07000006.bin", Some(&carry(&[0x1d, 0x45, 0x70])), 0, 2400, Some(lp));
}
