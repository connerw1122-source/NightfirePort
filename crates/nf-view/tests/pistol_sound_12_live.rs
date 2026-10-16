//! pistol-sound-12: the tower start pistol (07000012) resolves its sounds from the banks the game
//! would use, and its bullet-spawn shot is the weapon record's own id.
//!
//! The chain, every link cited:
//! - The kit [M Player_InitWeapon 0x1a7ef8 case 0x7000012]: a cold 07000012 start equips weapon
//!   **0x10** with 7 rounds (the Magnum family's tower sibling — same anim scripts as the Raptor
//!   Magnum 0x0e, own first-person model 0x0500003f, own ammo 0x0b) [L ee_snapshot.bin @ 0x2bf150 +
//!   0x10*0x10c].
//! - The bank [M Sound_Ready 0x1dceb8:59-61]: `case 0x7000012: SFXLoadSoundBank(0x1f)` — level 12's
//!   own bank is hash 0x1f = SBINFO slot 22 [D docs/formats/sound-bank.md slot table].
//! - The ids [D this test's own disc sweep over the parsed banks]: the shared fire/reload scripts
//!   (fire 0x06000490 `op1@1=0x1e1`, reload 0x06000495 `op1@36=0x1e2, op1@85=0x1e3` [L weapon_data
//!   0x10 +0xB0/+0xA4; the dump asserted below]) request 0x1e1/0x1e2/0x1e3, and the disc holds all
//!   three — and the remap target 0x602 and the record id 0x603 — in slots 2,10–13,16,**21–24**:
//!   the own bank 22 carries every id the pistol can request. (fix-weapon-sounds.md's "remap
//!   targets → 2 everywhere" row was measured wrong on this point; the sweep here is authoritative
//!   [D].)
//! - The remap [M AnimProcessScriptCmds 0x1c1b38:270-273]: an op-1 id of 0x1e1 is replaced with
//!   **0x602** while the player's current weapon (+0xdc→+0x62) is 0x10 — so the tower pistol's
//!   audible shot IS the own bank's 0x602 (the raw 0x1e1 must never sound: asserted below).
//! - The bullet-spawn layer [M 0x1c1b38 case 7:417-436]: an op 7 plays `weapon_def+0x60` through
//!   `Bullet_init` [M 0x12c5a0:269-274, `param_7 == -1` = silent]. For weapon 0x10, +0x60 =
//!   **0x0603** [L ee_snapshot @ 0x2bf150 + 0x10*0x10c + 0x60]. But the player's fire script
//!   carries NO op 7 [D the dump below], and the sim-side player spawn passes 0xffff = short −1 =
//!   silent [M 0x1a4bf0:288; 0x12c5a0:5 `short param_7`] — so 0x603 rides only the bot/world-anim
//!   paths (bots play it with `Rand(500)−250` pitch [M 0x178f68:277]). (+0x60's only consumers
//!   play it as a sound; the port's old "view-anim datum" comment misread it — MuzzleFlash
//!   consumes +0x4C, AnimObjectDraw +0x80 [M 0x1a4298:141, 0x1c4490:258].)
//! - The port resolves every pistol id from slot 22 with the own bank's decode (the repro below);
//!   the resolution is NOT the bug. The remaining game-vs-port deviation on this shot is the op-1
//!   pitch jitter — the game draws `Rand(500)−250` per op-1 sound [M docs/systems/anim-engine.md
//!   op 1; 0x178f68's bot call shows the same shape], the port's op-1 site carries no pitch —
//!   reported to the driver with live checks in docs/reports/pistol-sound-12.md.
//!
//! Assertions are numeric through the real path: `nf_view::load` → `Sim::step_bond` (fire/reload
//! input) → the view-script tick's op-1 plays → `Sounds` pool → `nf_view::sound_tick` → the driver;
//! the played voice's sample FNV (`Driver::played_sample_hash`) must equal the owning bank's decode
//! and `Driver::item_slot` must name the bank the rule prescribes.
//!
//! Opt-in via `NF_ISO`; run with `--test-threads=1`.

use nf_formats::iso9660::IsoImage;
use nf_formats::soundbank::{bank_path, bank_slots, Bank};
use nf_game::player::Input;
use std::collections::{HashMap, HashSet};

/// The disc's banks by SBINFO slot, and the level's own slot (`Sound_Ready`'s table [M]).
fn disc_banks(level: u32) -> (usize, HashMap<usize, (Bank, Vec<u8>)>) {
    let iso = IsoImage::open(std::env::var("NF_ISO").unwrap()).unwrap();
    let sbi = iso.read_file(iso.find("PS2/SBINFO.SBI").unwrap()).unwrap();
    let slots = bank_slots(&sbi).unwrap();
    let hash = nf_game::sound::level_bank(level).unwrap();
    let own = slots.iter().position(|&h| h == hash).unwrap();
    let mut map = HashMap::new();
    for (slot, &h) in slots.iter().enumerate() {
        if h == u32::MAX { continue; }
        let f = |ext: &str| iso.find(&bank_path("ENGLISH", slot, ext)).and_then(|e| iso.read_file(e).ok());
        let (Some(shf), Some(sfx), Some(sbf)) = (f("SHF"), f("SFX"), f("SBF")) else { continue };
        if let Ok(b) = Bank::parse(&shf, &sfx) { map.insert(slot, (b, sbf)); }
    }
    (own, map)
}

/// Every disc slot whose bank holds `id` (ascending — the driver's spare order).
fn slots_with(banks: &HashMap<usize, (Bank, Vec<u8>)>, id: u32) -> Vec<usize> {
    let mut v: Vec<usize> = banks.iter().filter(|(_, (b, _))| b.effect(id).is_some()).map(|(&s, _)| s).collect();
    v.sort();
    v
}

/// The bank that owns `id` on this level (the level's own bank first — the driver's lookup rule — else
/// the lowest spare slot that has it) and the FNV set of its non-stream samples for the id.
fn owning(banks: &HashMap<usize, (Bank, Vec<u8>)>, own: usize, id: u32) -> (usize, HashSet<u64>) {
    let has = |slot: usize| banks.get(&slot).is_some_and(|(b, _)| b.effect(id).is_some());
    let slot = if has(own) { own } else { (0..25).find(|&s| has(s)).unwrap() };
    let (b, sbf) = &banks[&slot];
    let fx = nf_formats::soundbank::Effect::parse(b.effect(id).unwrap()).unwrap();
    let hashes = fx.subs.iter().filter(|s| s.file >= 0).filter_map(|s| b.decode(sbf, s.file as usize).ok())
        .map(|(pcm, _)| nf_audio::sfx::pcm_hash(&nf_audio::Pcm { data: pcm, loop_start: None })).collect();
    (slot, hashes)
}

fn load(archive: &str) -> Option<nf_view::scene::Scene> {
    // Never through the speakers (NF_SILENT), as in fix_weapon_sounds_live.rs.
    unsafe { std::env::set_var("NF_SILENT", "1"); }
    let iso = std::env::var("NF_ISO").ok()?;
    Some(nf_view::load(&iso, archive, false).expect("07000012 loads (the native path — the kit weapon is the level's own)"))
}

/// The level boots into its intro (`sim.disabled`): step until playable, ≤400 ticks.
fn free(sim: &mut nf_view::scene::Sim) -> bool {
    for _ in 0..400 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { return true; } }
    !sim.disabled
}

/// Record every live item's (id → owning slot, sample hash).
fn harvest(sound: &nf_audio::sfx::Sound, seen: &mut HashMap<u32, (usize, u64)>) {
    for id in sound.driver.ids() {
        if let (Some(slot), Some(h)) = (sound.driver.item_slot(id), sound.driver.played_sample_hash(id)) {
            seen.insert(id, (slot, h));
        }
    }
}

/// One game tick through the real sound path (see fix_weapon_sounds_live.rs `tick`).
fn tick(sound: &mut nf_audio::sfx::Sound, scene: &mut nf_view::scene::Scene, seen: &mut HashMap<u32, (usize, u64)>) {
    let sim = scene.sim.as_mut().unwrap();
    nf_view::sound_tick(sound, sim);
    harvest(sound, seen);
    let all: Vec<u32> = (0..nf_audio::sfx::HW_VOICES as u32).collect();
    let mut out = Vec::new();
    sound.driver.update(&mut out, &all);
}

/// The probe: level 12's own bank, the pistol ids' disc spread, and the fire/reload scripts' ops.
#[test]
fn level12s_own_bank_and_the_pistol_scripts_ground_truth() {
    let Some(mut scene) = load("07000012.bin") else { eprintln!("NF_ISO not set — skipping"); return };
    let (own, banks) = disc_banks(0x0700_0012);
    assert_eq!(own, 22, "07000012 → bank hash 0x1f → slot 22 [M Sound_Ready 0x1dceb8:59-61]");
    for id in [0x1e1u32, 0x1e2, 0x1e3, 0x602, 0x603, 0x282, 0x403] {
        eprintln!("sfx {id:#05x}: held by slots {:?} [D]", slots_with(&banks, id));
    }
    // The reload/draw ids the scripts request raw are in the own bank [D fix-weapon-sounds.md:21].
    assert!(slots_with(&banks, 0x1e2).contains(&own) && slots_with(&banks, 0x1e3).contains(&own),
        "the reload ids 0x1e2/0x1e3 must be in the own bank 22");
    assert!(slots_with(&banks, 0x1e1).contains(&own), "the raw fire id 0x1e1 must be in the own bank 22");
    // The remap target and the record id are in the own bank too [D the sweep above] — the tower
    // pistol's shot needs no spare bank at all. (fix-weapon-sounds.md's "2 everywhere" row for the
    // remap targets measured wrong; this sweep is the authoritative disc read.)
    assert!(slots_with(&banks, 0x602).contains(&own), "the remap target 0x602 is in the own bank 22");
    assert!(slots_with(&banks, 0x603).contains(&own), "the record id 0x603 is in the own bank 22");
    // The kit: a cold start equips 0x10 with 7 rounds [M 0x1a7ef8 case 0x7000012].
    let sim = scene.sim.as_mut().unwrap();
    assert!(free(sim), "the intro frees");
    let gun = &scene.sim.as_ref().unwrap().gun;
    eprintln!("cold-start weapon = {:#x}, clip = {}, reload_state = {}", gun.weapon, gun.clip, gun.reload_state);
    assert_eq!(gun.weapon as usize, 0x10, "07000012's cold kit weapon is 0x10 [M Player_InitWeapon]");
    // The shared scripts' ops: the fire script must carry the raw op-1 0x1e1 (the remap's input);
    // dump every op so the op-7 (bullet spawn + its sound) question is answered from the data.
    let sim = scene.sim.as_ref().unwrap();
    for (name, id) in [("fire", 0x0600_0490u32), ("fire_last", 0x0600_0491u32), ("reload", 0x0600_0495u32)] {
        match sim.scripts.get(&id) {
            Some(sc) => {
                let ops: Vec<String> = sc.commands.iter()
                    .map(|c| if c.op == 1 { format!("op1@{}={:#x}", c.args[0], c.args[1]) }
                         else if c.op == 7 { format!("op7@{}", c.args.first().map(|a| a.to_string()).unwrap_or_default()) }
                         else { format!("op{}", c.op) })
                    .collect();
                eprintln!("script {name} {id:08x}: {:?}", ops);
                if name == "fire" {
                    assert!(sc.commands.iter().any(|c| c.op == 1 && c.args.len() >= 2 && c.args[1] as u32 == 0x1e1),
                        "the shared fire script's op-1 must be the raw 0x1e1 (the remap's input [M 0x1c1b38:270])");
                }
                if name == "reload" {
                    assert!(sc.commands.iter().any(|c| c.op == 1 && c.args.len() >= 2 && c.args[1] as u32 == 0x1e2)
                         && sc.commands.iter().any(|c| c.op == 1 && c.args.len() >= 2 && c.args[1] as u32 == 0x1e3),
                        "the shared reload script's op-1s must be 0x1e2/0x1e3");
                }
            }
            None => eprintln!("script {name} {id:08x}: NOT in this level's script set"),
        }
    }
}

/// THE repro: fire the start pistol on 07000012 through the real path; every id it requests must
/// start with the owning bank's own sample bytes (own bank 22 first, else the lowest spare — the
/// fix-weapon-sounds rule), and the raw Magnum id 0x1e1 must NEVER sound while 0x10 is current
/// (the game replaces it with 0x602 [M 0x1c1b38:270-273]).
#[test]
fn the_start_pistols_sounds_come_from_the_game_owned_banks() {
    let Some(mut scene) = load("07000012.bin") else { eprintln!("NF_ISO not set — skipping"); return };
    let (own, banks) = disc_banks(0x0700_0012);
    assert_eq!(own, 22, "07000012 → slot 22 [M Sound_Ready]");
    {
        let sim = scene.sim.as_mut().unwrap();
        assert!(free(sim), "the intro frees");
        assert_eq!(sim.gun.weapon as usize, 0x10, "the cold kit weapon [M 0x1a7ef8]");
    }
    let mut sound = nf_view::build_sound(&mut scene).expect("the level has a bank");
    let mut seen = HashMap::new();
    // Fire (clip 7): hold the trigger until the remapped fire sound starts.
    let mut fired_at = None;
    for t in 0..40 {
        scene.sim.as_mut().unwrap().step_bond(Input { fire: true, ..Input::default() }, true, false, false);
        tick(&mut sound, &mut scene, &mut seen);
        if seen.contains_key(&0x602) { fired_at = Some(t); break; }
    }
    let Some(_at) = fired_at else {
        panic!("the start pistol's fire sound 0x602 never started a voice (fire ids seen: {:?}); all seen {seen:?}",
            seen.keys().filter(|k| **k >= 0x1e1 && **k <= 0x604).collect::<Vec<_>>());
    };
    // Reload: the shared reload script's 0x1e2/0x1e3 (no remap rule covers them [M 0x1c1b38:249-284]).
    for _ in 0..300 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, true, false);
        tick(&mut sound, &mut scene, &mut seen);
        if seen.contains_key(&0x1e2) && seen.contains_key(&0x1e3) { break; }
    }
    // Every pistol sound resolves per the own-bank-first rule with that bank's own decode.
    let mut checked = Vec::new();
    for id in [0x602u32, 0x1e2, 0x1e3] {
        let Some(&(slot, h)) = seen.get(&id) else { continue };
        let (want_slot, want) = owning(&banks, own, id);
        assert_eq!(slot, want_slot, "sound {id:#x} resolved from slot {slot}, the rule's owning bank is {want_slot}");
        assert!(want.contains(&h), "sound {id:#x}'s sample hash {h} is not the owning bank's decode {want:?}");
        checked.push(id);
    }
    assert!(checked.contains(&0x602), "the fire sound 0x602 must be among the checked ids; seen {seen:?}");
    assert!(!checked.is_empty(), "no pistol sound resolved; seen {seen:?}");
    // The remap: the raw Magnum id must never have started while 0x10 was current [M 0x1c1b38:270-273].
    assert!(!seen.contains_key(&0x1e1), "the raw 0x1e1 started a voice — the 0x10 remap leaked");
}

/// The sister (the discriminator): a different weapon's id does NOT match the pistol's sample — the
/// driver-level starts of the Magnum's raw fire id (0x1e1: what the shared script plays for 0x0e,
/// and exactly what would sound if the 0x10 remap were missing) and the P2K's fire id (0x403) are
/// different takes from the pistol's remapped 0x602, each from its own rule-resolved bank.
#[test]
fn a_different_weapons_id_does_not_match_the_pistols_sample() {
    let Some(mut scene) = load("07000012.bin") else { eprintln!("NF_ISO not set — skipping"); return };
    let (own, banks) = disc_banks(0x0700_0012);
    let mut sound = nf_view::build_sound(&mut scene).expect("the level has a bank");
    let mut hashes = HashMap::new();
    for id in [0x602u32, 0x1e1, 0x403] {
        let mut out = Vec::new();
        let rc = sound.driver.start(&mut out, id, Some([0.0; 3]), 0x9999, None);
        let (want_slot, want) = owning(&banks, own, id);
        assert_eq!(rc, 0, "driver start of {id:#x} failed (no owning row?)");
        let slot = sound.driver.item_slot(id);
        let h = sound.driver.played_sample_hash(id);
        assert_eq!(slot, Some(want_slot), "sound {id:#x} from slot {slot:?}, the rule's owning bank is {want_slot}");
        assert!(h.is_some_and(|h| want.contains(&h)), "sound {id:#x} hash {h:?} not the owning bank's decode {want:?}");
        hashes.insert(id, h);
        sound.driver.remove(0x9999, None);
        let mut out2 = Vec::new();
        sound.driver.update(&mut out2, &[]);
    }
    // The three takes are pairwise distinct samples — the pistol's remapped shot is not the Magnum's
    // raw take nor the P2K's, so the repro's hash assertion genuinely discriminates.
    let (h602, h1e1, h403) = (hashes[&0x602], hashes[&0x1e1], hashes[&0x403]);
    assert_ne!(h602, h1e1, "0x602 (the 0x10 remap) and 0x1e1 (the Magnum's raw fire) decoded to the same sample");
    assert_ne!(h602, h403, "0x602 and the P2K's 0x403 decoded to the same sample");
    assert_ne!(h1e1, h403, "0x1e1 and 0x403 decoded to the same sample");
}
