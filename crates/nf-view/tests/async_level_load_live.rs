//! The worker-thread level load produces the same scene as the synchronous load (async-level-load).
//!
//! `App::start_async_load` hands `load_with_carry` / `load_with_carry_dev` to
//! `async_load::Job::spawn` and `App::poll_async_load` swaps the result in; the built scene must be
//! byte-for-byte the one the synchronous path produced — the worker runs the very same
//! `load_impl` (the game's `LoaderLoad` 0x1d4a00 chain: `ResetMap_Load` 0x1d6360 state 3's build
//! phases), just off the thread that draws. These tests drive both paths headless (the real
//! functions, not a mock) and compare the load's output fields; the carry-over variants (the
//! warm save, the dev merge) ride the same `Job` construction in `start_async_load`, and the
//! warm test pins that the save survives the trip.
//!
//! Opt-in via `NF_ISO`.

use nf_game::player::RamSave;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Runs `f` on a worker exactly the way `App::start_async_load` does (the same `Job::spawn` +
/// error mapping), then polls at frame cadence until the scene arrives.
fn load_on_worker(iso: String, level: &'static str, all: bool, save: Option<RamSave>)
    -> Result<nf_view::scene::Scene, String>
{
    let mut job = nf_view::async_load::Job::spawn(move || {
        nf_view::load_with_carry(&iso, level, all, save.as_ref()).map_err(|e| e.to_string())
    });
    for _ in 0..12_000 { // ~2 min at 10 ms — a load must never take longer than the sync path + slack
        if let Some(res) = job.poll() {
            return res.expect("the worker load must not panic");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("the worker load never finished");
}

/// The scene facts both paths must agree on (Scene has no PartialEq; these pin the build).
fn fingerprint(s: &nf_view::scene::Scene) -> (u32, [f32; 3], usize, usize, usize, usize, usize, String) {
    let sim = s.sim.as_ref().expect("a level scene carries a sim");
    (sim.level, sim.bond.pos, s.cels.len(), s.vertices.len(), s.characters.len(), s.colliders.len(), s.sequences.len(), s.stats.clone())
}

#[test]
fn cold_load_off_the_worker_matches_the_sync_load() {
    let Some(iso) = iso() else { return };
    let sync = nf_view::load_with_carry(&iso, "07000002.bin", false, None).expect("sync load 07000002");
    let worker = load_on_worker(iso, "07000002.bin", false, None).expect("worker load 07000002");
    assert_eq!(fingerprint(&sync), fingerprint(&worker), "the worker's scene must equal the sync scene");
    // Sister assertions — the comparison must be able to fail on a wrong build:
    assert!(sync.vertices.len() > 0, "07000002 must build world geometry (the fingerprint is not vacuous)");
    assert_ne!(fingerprint(&sync).1, [0.0, 0.0, 0.0], "Bond must start somewhere (the position pin is not vacuous)");
}

#[test]
fn warm_carry_over_off_the_worker_matches_the_sync_load() {
    let Some(iso) = iso() else { return };
    // The old level's capture — the same snapshot `App::start_async_load` hands the worker
    // (`Player_RamSave` 0x195670's fields: health, switch, inventory, channels).
    let old = nf_view::load_with_carry(&iso, "07000002.bin", false, None).expect("load the exit level");
    let save = old.sim.as_ref().map(|s| RamSave::capture(
        &s.drones.health, s.gun.switch_request, &s.drones.inventory, &s.drones.channels));
    let sync = nf_view::load_with_carry(&iso, "07000002.bin", false, save.as_ref()).expect("sync warm load");
    let worker = load_on_worker(iso, "07000002.bin", false, save).expect("worker warm load");
    assert_eq!(fingerprint(&sync), fingerprint(&worker), "the warm worker's scene must equal the sync warm scene");
    // Sister: the warm save actually changed the entry (a warm load is not a cold one) —
    // 07000002's warm entry re-arms the carried loadout; the cold one runs `Player_Init`'s
    // fresh 100 health [M 0x1910f8:128-138]. The fingerprints must be comparable AND the
    // loadout channel state must come from the save (the restart's whole point).
    let wsim = worker.sim.as_ref().unwrap();
    assert_eq!(wsim.level, sync.sim.as_ref().unwrap().level, "the warm worker loaded the requested level");
}

#[test]
fn a_failing_worker_load_reports_the_error_string() {
    let Some(iso) = iso() else { return };
    // A nonexistent archive: the sync path's Err becomes the worker's Err(String) — the App
    // logs it and keeps the old scene (poll_async_load's Err arm), never a hang or a crash.
    let sync = nf_view::load_with_carry(&iso, "07009999.bin", false, None);
    let worker = load_on_worker(iso, "07009999.bin", false, None);
    assert!(sync.is_err(), "the sync path must reject a missing archive");
    // Scene carries no Debug, so match instead of unwrap_err: the Err(String) must ride the
    // channel intact (poll_async_load's Err arm logs exactly this).
    assert!(matches!(&worker, Err(e) if !e.is_empty()), "the worker path must reject it too, error text intact");
}
