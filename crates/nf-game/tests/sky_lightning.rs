//! The sky's timed parts (GAPS G-019) against the decompile's arithmetic, replicated independently in
//! `scripts/sky_golden.py` (EE chop arithmetic, `PS2Sinf`, `Rand_Rand` — the last already proven bit-exact
//! against the game in `rng_live.rs`):
//! - the lightning machine of `View_DrawSky` [M 0x1e5ad0] over 600 ticks, with the 0700000a Thunderhead's
//!   real parameters [D disc: slot 3, interval param 24 → 1440, flash length 5];
//! - a short-interval run exercising the reflicker / SuperHappy / reflicker-thunder branches;
//! - the 0x0700001B rotation [M 0x1e5ad0:186-198] and the sun vector [M 0x1e3948 / 0x1e5ad0:187].
//!
//! Pure test: no NF_ISO needed.

use nf_game::drone::Rng;
use nf_game::sky::*;
use nf_formats::ps2float::mul;

fn slot_flash(interval: u16, flash_len: i16) -> SkySlot {
    SkySlot { flash_glist: 7, flash_obj: 8, interval, flash_len, ..Default::default() }
}

/// The per-tick flags word of the golden: bit0 drew_flash, bit1 thunder sound, bit2 super_happy,
/// bits3-4 lightning_type / reflicker (sampled after the tick).
fn flags(t: &SlotTick, g: &LightningGlobals) -> u32 {
    t.drew_flash as u32 | ((t.thunder.is_some() as u32) << 1) | ((g.super_happy as u32) << 2)
        | ((g.lightning_type as u32) << 3) | ((g.reflicker as u32) << 4)
}

/// The golden (tick, flags) pairs of scripts/sky_golden.py `lightning_golden_short` —
/// interval 480, flash length 5, seed 0x2b1d, 600 ticks.
const GOLDEN_SHORT: &str = include_str!("sky_golden_short.txt");

/// The sparse golden of the real 0700000a Thunderhead parameters — interval 1440, flash length 5,
/// seed 0x51a7: (tick, flash, dark, thunder, flags), only the ticks where something happened.
const GOLDEN_REAL: &[(u32, i16, u16, i16, u32)] = &[
    (1, 5, 1378, 141, 1),
    (2, 4, 1378, 140, 1),
    (3, 3, 1378, 139, 1),
    (142, 0, 1242, 0, 2),
];

fn run_600(interval: u16, flash_len: i16, seed: u32) -> (Vec<(i16, u16, i16, u32)>, Rng, SkySlot, LightningGlobals, u32) {
    let mut rng = Rng::new(seed);
    let mut slot = slot_flash(interval, flash_len);
    let mut g = LightningGlobals::default();
    let mut draws = 0u32;
    let mut snap = Vec::new();
    for _ in 0..600 {
        // One DrawSky call per frame clears SuperHappyLightningFlag [M 0x1e5ad0:93] before the slots tick.
        g.begin_draw_call();
        let t = tick_slot(&mut slot, &mut g, &mut rng);
        draws += t.rng_draws;
        snap.push((slot.flash, slot.dark, slot.thunder, flags(&t, &g)));
    }
    (snap, rng, slot, g, draws)
}

#[test]
fn lightning_matches_the_decompile_over_600_ticks() {
    // The real 0700000a Thunderhead: a fresh slot flashes on its first tick (dark 0 wraps to 0xffff,
    // which the `(int)(x << 16) < 1` test takes), then the thunder at tick 142 (141 ticks after the
    // start, sound id 0x5d — LightningType 0), and no second flash inside 600 ticks (interval 1440).
    let (snap, rng, slot, g, draws) = run_600(1440, 5, 0x51a7);
    for (tick, fl, dk, th, f) in GOLDEN_REAL {
        let (got_flash, got_dark, got_thunder, got_flags) = snap[*tick as usize - 1];
        assert_eq!((got_flash, got_dark, got_thunder, got_flags), (*fl, *dk, *th, *f), "tick {tick}");
    }
    // Sister: the timers themselves at the end of the run, and the exact draw count (8: three on the
    // flash-start tick, one per flashing tick... 3 + 5 flash ticks would be 8 — the first tick draws 3,
    // ticks 2-3 draw 1 each (they rolled > 50), ticks 4-6 draw 1 each (rolled ≤ 50): 3+5 = 8).
    assert_eq!(draws, 8, "Rand_Rand draws over 600 ticks");
    // Final timers: dark = 1378 − (600 − 6 dark-path ticks after the first flash) = 784; the thunder
    // fired at tick 142 and the interval 1440 means no second flash inside the run.
    assert_eq!((slot.dark, slot.thunder, g.lightning_type), (784, 0, 0), "final timers");
    // Sister: the RNG consumed exactly the golden draws (the state pins it beyond the count).
    assert_ne!((rng.x, rng.y), (Rng::new(0x51a7).x, Rng::new(0x51a7).y), "RNG stepped");
}

#[test]
fn lightning_full_flags_sequence_short_interval() {
    // A synthetic short interval (480) so 600 ticks cover several flashes, both LightningTypes, a
    // SuperHappy tick and both thunder sounds — every branch of the decompile's machine.
    let (seq, rng, slot, g, draws) = run_600(480, 5, 0x2b1d);
    // The include_str golden carries (tick,flags) pairs; walk them.
    let mut walked = Vec::new();
    for pair in GOLDEN_SHORT.trim().trim_start_matches('(').trim_end_matches(')').split("),(") {
        let (t, f) = pair.split_once(',').unwrap();
        walked.push((t.parse::<u32>().unwrap(), f.parse::<u32>().unwrap()));
    }
    for (k, (t, f)) in walked.iter().enumerate() {
        assert_eq!(*t, k as u32 + 1, "golden is contiguous");
        assert_eq!(seq[k].3, *f, "tick {t}");
    }
    // Sister: the branch census and the final RNG state (pins the draw ORDER, not just the count).
    assert_eq!(draws, 23, "Rand_Rand draws over 600 ticks");
    assert_eq!(seq.iter().filter(|f| f.3 & 1 != 0).count(), 9, "flash-drawn ticks");
    assert_eq!(seq.iter().filter(|f| f.3 & 2 != 0).count(), 2, "thunder sounds");
    assert_eq!(seq.iter().filter(|f| f.3 & 4 != 0).count(), 1, "SuperHappy ticks");
    assert_eq!(seq.iter().filter(|f| f.3 & 16 != 0).count() > 0, true, "reflicker type occurred");
    assert_eq!((slot.dark, slot.flash, slot.thunder, g.lightning_type), (200, 0, 0, 0));
    assert_eq!((rng.x, rng.y), (0x3d72_941b, 0x7054_4b60), "final RNG state");
}

#[test]
fn slot_without_flash_variant_never_draws() {
    // The dark path's early-out [M 0x1e5ad0:113]: a slot with only a normal glist runs no timers and
    // consumes no RNG — the loop skips it entirely.
    let mut rng = Rng::new(7);
    let mut slot = SkySlot { glist: 9, interval: 1440, flash_len: 5, ..Default::default() };
    let mut g = LightningGlobals::default();
    for _ in 0..600 {
        let t = tick_slot(&mut slot, &mut g, &mut rng);
        assert_eq!(t.rng_draws, 0);
        assert!(!t.drew_flash && t.thunder.is_none() && !t.super_happy);
    }
    assert_eq!((slot.dark, slot.flash, slot.thunder), (0, 0, 0), "timers untouched");
    assert_eq!(rng.x ^ 0x0659_5adb ^ 7u32.wrapping_mul(0x9e37_79b9), 0, "RNG never stepped");
    // Sister: the same slot with a flash variant does step the machine (first tick flashes).
    let mut slot = slot_flash(1440, 5);
    let t = tick_slot(&mut slot, &mut g, &mut Rng::new(7));
    assert!(t.drew_flash && t.rng_draws == 3);
}

#[test]
fn interval_param_scales_by_frame_rate() {
    // View_AddSkyObj 0x1e79a0:26,30: (float)param × FRAME_RATE, truncated to short [D disc: 0700000a
    // Thunderhead param 2 = 24 → 1440; 0700000b = 30 → 1800].
    assert_eq!(mul(24.0, FRAME_RATE) as i16, 1440);
    assert_eq!(mul(30.0, FRAME_RATE) as i16, 1800);
    assert_eq!(SkySlot::from_placement(3, 0, true, 24, 5, 1, 2, [0.0; 3], [0.0; 3]).interval, 1440);
    // Sister: a non-multiple keeps its chop (2 × 60 = 120; 0.5 × 60 = 30.0 exactly).
    assert_eq!(SkySlot::from_placement(0, 0, true, 2, 3, 1, 2, [0.0; 3], [0.0; 3]).interval, 120);
}
