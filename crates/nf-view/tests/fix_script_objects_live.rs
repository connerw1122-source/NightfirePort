//! Headless NF_ISO tests for fix-script-objects (task 289) — GAPS G-031's script-created objects:
//!
//! 1. **The `SP_Hit` mask-2 flag** (`SP_Hit` 0x1dc470, called from `SP_Update` 0x1dbda8:66 while
//!    the countdown byte is 0): an explosion reaching one of the current script's stream objects
//!    sets trigger-flag bit 0x2, so a mask-2 SP fires. The disc's case is 07000007's urn SP #312
//!    (script 0x0600007A, mode 64, mask 2, out 0) [D levels.md §5.3 + the placement dump]: the
//!    mode-64 load beat primes it in state 1 with the urn-shard entities created at the SP base
//!    (the same prime pattern the 07000005 EndLevel SP shows live [L ee_start_05.bin]); the blast
//!    then replays the script (state 2) — the shards fly again and the tick-1 Sound(1523) queues.
//! 2. **The script-created AI volume** (`Drone_AIVolume_Create` 0x1376c0 via `Script_CreateEntity`
//!    0x1d8968's 0xF5 arm): 07000005's truck script 0x06000A22 stream 2 (glist 0x0200040C,
//!    params [0]) — the disc's only script-created volume [D the 289 census]. The volume joins
//!    the level's list the tick the script creates it, its half-extents resolve from the glist's
//!    map-object bounds, and its matrix tracks the live stream entity.
//!
//! Run with: `NF_ISO=… cargo test -p nf-view --test fix_script_objects_live -- --test-threads=1`

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The urn SP #312: placement kind 0xD9, params (0, 0x0600007A) (2, 64) (5, mask 2) — the port's
/// `Player::from_placement` maps them to scripts[0]/modes[0]/mask [D the 07000007 placement dump].
const URN_SCRIPT: u32 = 0x0600_007a;

fn urn_player(sim: &nf_view::scene::Sim) -> &nf_game::nis::Player {
    sim.drones.scripts.players.iter().find(|p| p.scripts[0] == URN_SCRIPT)
        .expect("the urn SP #312 is a placed script player")
}

/// The mask-2 flag fires the urn SP: the mode-64 prime leaves it in state 1 with the shard
/// entities created; a blast whose sphere reaches a shard (the SP base + the shards' local
/// centres) sets `SP_Hit`'s bit 0x2, the mask matches, and the shatter script replays (state 2,
/// playing) with the tick-1 Sound(1523) queued. Fails on the pre-fix tree: the flag byte had no
/// bit 0x2 source (only the channel 1 and activation 4 bits), so no blast ever fired the SP.
#[test]
fn the_urn_sp_fires_when_a_blast_reaches_the_shards() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load 07000007");
    let sim = scene.sim.as_mut().unwrap();
    // The first tick runs the mode-64 prime (`SP_CreateScriptPlayer@0x1db9a8:81-86`'s load beat).
    sim.step_bond(nf_game::player::Input::default(), false, false, false);
    {
        let p = urn_player(sim);
        assert_eq!(p.state, 1, "the mode-64 prime leaves the urn SP in state 1 (loaded, stopped)");
        assert_eq!(p.mask, 2, "SP #312's mask is 2 (explosion hits only)");
    }
    // The prime ran one Script_Update (clock 1.0): only the Wait(0) streams (9 and 11) created
    // their shard entities; the Wait(1) streams fire on the replay. [D the script decode.]
    let shards = |sim: &nf_view::scene::Sim| sim.drones.scripts.objects.list.iter().flatten()
        .filter(|o| matches!(o, nf_game::nis::Obj::Entity { src: (s, _), .. } if *s == URN_SCRIPT)).count();
    assert!(shards(sim) >= 2, "the prime created the Wait(0) streams' shard entities");
    // The blast: SP #312's placement sits at (49.262, 43.009, −53.464) [D the placement dump] and
    // the shards' keys start at the script base (key 0 pos (0,0,0)); a radius-3 blast at the base
    // reaches them the way the game's `Collide_SphereIntersect` sweep did.
    sim.drones.explosions.push(([49.26, 43.01, -53.46], 3.0, 100.0, usize::MAX, 0x34));
    sim.step_bond(nf_game::player::Input::default(), false, false, false);
    {
        let p = urn_player(sim);
        assert_eq!(p.state, 2, "the mask-2 SP fired on the blast (SP_Hit bit 0x2 through mask 2)");
        assert!(p.info[p.cur].as_ref().is_some_and(nf_game::nis::Script::playing),
            "the shatter script is replaying");
    }
    // The replay's ticks recreate every stream's shard (the Wait(1) streams now fire) and queue
    // the tick-1 Sound(1523) — the urn break's audio (script stream 12). Stream 11's shard ends
    // at its own EntityEnd on replay tick 1, so the live count settles at 10 (streams 0-9).
    sim.step_bond(nf_game::player::Input::default(), false, false, false);
    assert!(sim.drones.sounds.iter().any(|s| s.id == 1523), "the urn break's sound 1523 queued");
    assert!(shards(sim) >= 10, "the replay recreated every stream's shard, got {}", shards(sim));
}

/// Sister: a blast FAR from the shards (30 units east) never fires the SP — the sphere test is
/// the mechanism, not any blast anywhere.
#[test]
fn a_far_blast_never_fires_the_urn_sp() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load 07000007");
    let sim = scene.sim.as_mut().unwrap();
    sim.step_bond(nf_game::player::Input::default(), false, false, false);
    sim.drones.explosions.push(([49.26 + 30.0, 43.01, -53.46], 3.0, 100.0, usize::MAX, 0x34));
    sim.step_bond(nf_game::player::Input::default(), false, false, false);
    assert_eq!(urn_player(sim).state, 1, "the far blast leaves the urn SP waiting");
    assert!(!sim.drones.sounds.iter().any(|s| s.id == 1523), "no urn sound on a far blast");
}

/// The truck script's AI volume: SP #37 (the placed ScriptPlayer whose script A is 0x06000A22)
/// is mode-64 primed at boot, and stream 2's Entity (obj_type 0xF5, glist 0x0200040C, params
/// [0]) creates the volume on the first tick. It joins `Sim::volumes` with the glist's
/// half-extents, factor 0 (param 0), kind 0 (param 1), tracking the live stream entity. Fails on
/// the pre-fix tree: the 0xF5 arm made a plain `Obj::Entity` and no volume was ever registered.
#[test]
fn the_truck_script_creates_its_ai_volume() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let sim = scene.sim.as_mut().unwrap();
    let before = sim.volumes.len();
    for _ in 0..4 { sim.step_bond(nf_game::player::Input::default(), false, false, false); }
    let vols: Vec<&nf_game::aivolume::AiVolume> = sim.volumes.iter()
        .filter(|v| v.src == Some((0x0600_0a22, 2))).collect();
    assert_eq!(vols.len(), 1, "the truck script's stream-2 AI volume was registered once");
    let v = vols[0];
    assert_eq!(v.placement, usize::MAX, "a script-created volume has no placement");
    assert_eq!(v.kind, 0, "param 1 = 0 → the visibility-box kind");
    assert_eq!(v.factor, 0.0, "param 0 = 0 → the 0.0 visibility factor");
    assert!(v.half.iter().any(|&h| h > 0.0), "the half-extents resolved from glist 0x0200040C's bounds: {:?}", v.half);
    // The volume's matrix tracks the live stream entity (the game keys obj+0x90 per tick).
    let live = sim.drones.scripts.entity_matrix((0x0600_0a22, 2)).expect("the volume's entity is live");
    let d: f32 = (0..3).map(|k| (v.m[3][k] - live[3][k]).powi(2)).sum::<f32>().sqrt();
    assert!(d < 0.01, "the volume tracks the live entity matrix (off by {d})");
    assert!(sim.volumes.len() > before, "a volume was ADDED, not a reused one");
}
