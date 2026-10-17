//! The user's playtest request (2026-10-16, "unlock all the levels"): the window sets `menu::UNLOCK_ALL`
//! before creating any front end, and every `FrontEnd` then opens every mission row [S port choice].
//! Sister: without the switch a fresh profile keeps the game's locks.

use nf_view::menu::{FrontEnd, CAMPAIGN, UNLOCK_ALL};
use std::sync::atomic::Ordering;

#[test]
fn the_window_switch_opens_every_mission_and_the_default_keeps_the_locks() {
    UNLOCK_ALL.store(false, Ordering::Relaxed);
    let locked = FrontEnd::new();
    assert!((0..CAMPAIGN.len()).any(|i| !locked.unlocked_row(i)), "a fresh profile has locked missions");
    UNLOCK_ALL.store(true, Ordering::Relaxed);
    let open = FrontEnd::new();
    assert!((0..CAMPAIGN.len()).all(|i| open.unlocked_row(i)), "every mission row is open under the switch");
    UNLOCK_ALL.store(false, Ordering::Relaxed);
}
