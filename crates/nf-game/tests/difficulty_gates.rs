//! difficulty-effects: the create-time difficulty gates, numbers only, synthetic placements
//! (no disc needed). Every gate cites its game function/address; each assertion has a sister
//! that would fail on the pre-task code (no gate at all) or on an inverted threshold.

use nf_formats::placement::Placement;

fn placement(flags: u8, params: &[(u32, u32)]) -> Placement {
    Placement {
        index: 0,
        hashcode: u32::MAX,
        flags: flags as u32,
        translation: [10.0, 0.0, 10.0],
        rotation: [0.0, 0.0, 0.0],
        params: params.to_vec(),
        extra: [0; 9],
    }
}

/// `Lock_Create` 0x1ae1f8:67-69 → `Lock_Unlock` 0x1b0650:28-31: a lock tagged with
/// level_tag+0x44 (param 6) starts unlocked (state +0xf4 = 1) ONLY on difficulty 1
/// (Operative). The pre-task port never read param 6 or the difficulty.
#[test]
fn lock_param6_tag_auto_unlocks_on_operative_only() {
    let tagged = [placement(0x2e, &[(5, 134), (6, 1)])];
    let untagged = [placement(0x2e, &[(5, 134)])];

    for (difficulty, want) in [(1u32, true), (2, false), (3, false), (4, false)] {
        let locks = nf_game::lock::Lock::create(&tagged, difficulty, |_| None);
        assert_eq!(locks.len(), 1);
        assert_eq!(locks[0].unlock_on_operative, true, "param 6 tag must be recorded");
        assert_eq!(locks[0].unlocked, want, "difficulty {difficulty}: the param-6 lock's create state");
    }
    for difficulty in [1u32, 2, 3, 4] {
        let locks = nf_game::lock::Lock::create(&untagged, difficulty, |_| None);
        assert_eq!(locks[0].unlock_on_operative, false);
        assert_eq!(locks[0].unlocked, false, "untagged locks never auto-unlock (d{difficulty})");
    }
    // Sister: the same placement flips state exactly at the difficulty boundary 1 → 2.
    let d1 = nf_game::lock::Lock::create(&tagged, 1, |_| None)[0].unlocked;
    let d2 = nf_game::lock::Lock::create(&tagged, 2, |_| None)[0].unlocked;
    assert!(d1 && !d2, "the gate must flip between Operative and Agent");
    // An unlocked lock takes no laser burn (the game's state +0xf4 = 1 is not the burnt state
    // but the laser path is moot on it): burn() reports nothing completed.
    let mut ch = nf_game::triggers::Channels::default();
    let mut locks = nf_game::lock::Lock::create(&tagged, 1, |_| None);
    assert!(!locks[0].burn(&mut ch, 0), "an unlocked lock must not burn");
}

/// `Copter_Create` 0x17d340:76-94: `obj+0x68 = DiffMod * 299.0` with
/// d1 1.0 / d2 1.5 / d3 2.0 / d4 4.0. The pre-task port had no HP at all.
#[test]
fn copter_hp_scales_299_by_difficulty() {
    let placements = [placement(0x2f, &[])];
    for (difficulty, hp) in [(1u32, 299.0), (2, 448.5), (3, 598.0), (4, 1196.0)] {
        let copters = nf_game::copter::Copter::create(&placements, difficulty);
        assert_eq!(copters.len(), 1);
        assert!(
            (copters[0].hp - hp).abs() < 1e-4,
            "difficulty {difficulty}: hp {} must be {hp}", copters[0].hp
        );
    }
    // Sister: an off-table difficulty (0) falls back to the decomp's 1.0 default.
    let copters = nf_game::copter::Copter::create(&placements, 0);
    assert!((copters[0].hp - 299.0).abs() < 1e-4);
}

/// `Hint_Create` 0x1b3248:12-25's stage gate, every row of the decomp condition.
#[test]
fn hint_stage_gate_table() {
    use nf_game::player::hint_stage_gates as g;
    // stage 0 = always (the only stage value found on the disc's hint markers so far).
    for d in [1u32, 2, 3, 4] {
        assert!(g(0, d), "stage 0 must gate in at difficulty {d}");
    }
    // stages 1..=4 = a difficulty CEILING: the hint is refused when `stage < difficulty`, so
    // it shows while difficulty <= stage (more help on easier settings — stage 1 is
    // Operative-only, stage 3 shows everywhere).
    assert!(g(1, 1) && !g(1, 2) && !g(1, 3), "stage 1 = Operative only");
    assert!(g(2, 1) && g(2, 2) && !g(2, 3), "stage 2 = through Agent");
    assert!(g(3, 1) && g(3, 2) && g(3, 3), "stage 3 = everywhere");
    // stage 4: the first check (4 < d) is false for every reachable difficulty and 4 is not
    // one of the 5/6/7 exact rows — the decomp lets it through at every difficulty.
    assert!(g(4, 1) && g(4, 2) && g(4, 3));
    // stages 5/6/7 = exactly Operative/Agent/00 Agent.
    for (stage, only) in [(5u32, 1u32), (6, 2), (7, 3)] {
        for d in [1u32, 2, 3] {
            assert_eq!(g(stage, d), d == only, "stage {stage} at difficulty {d}");
        }
    }
    // Sister: the old no-gate behaviour would return true for (6, 3) too.
    assert!(!g(6, 3));
}
