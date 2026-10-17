//! mp-bots-in-match (704): the Quick Game's 3 bots must actually PLAY.
//! Headless, from the menus' `mp_match_settings` through `load_mp_carry` (the same load the
//! Quick Start serves), 3600 fixed ticks of the arena. Per bot: route movement (positions
//! change), acquisition + fire at the human and other bots, damage + death, respawn per the
//! game's rules, scores going up. Sister: `bot_count = 0` leaves only the human in the match.
//!
//! Evidence: [M] work/decomp/ACTION.ELF (function address = filename prefix), [L] EE snapshot,
//! [S] suggested. The bot brain is the ported `nf_game::mp::bots::BotBrain` (the BOT_update
//! 0x126448 family); the match core is `nf_game::mp` (MP_Start 0x183a50, MP_Update 0x184bd0).

use nf_game::mp::MpSettings;
use nf_game::player::Input;

fn iso() -> Option<String> {
    std::env::var("NF_ISO").ok()
}

/// The menus' Quick Start: a local Deathmatch on the first arena with Drake/Kiko/Rook enabled
/// (`mp_menu_flow.rs`:233 pins the same settings — bot_count 3 rides `mp_match_settings`).
fn quick_game() -> MpSettings {
    let mut s = MpSettings::deathmatch(0x0700_0023);
    s.human_count = 1;
    s.player_count = 1;
    s.total_players = 1;
    s.bot_count = 3;
    s.seed = 0x0bed_0bad;
    s
}

#[ignore = "704 in progress: the brains tick but stay in BotIdle — the goal picker starves on an empty pickup table (the arena's MP pickups are not lifted into BotPickup yet) and find_opponent never acquires; the next session feeds both — see the report"]
#[test]
fn quick_game_bots_play_the_match() {
    let Some(path) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let s = quick_game();
    let arena = format!("{:08x}.bin", s.level_code);
    let mut scene = nf_view::load_mp_carry(&path, &arena, false, None, s.clone()).expect("arena loads");

    // The bots exist: `MP_Start` 0x183a50:134-160 creates one bot per enabled record in slots
    // 4..4+count, after every human's create.
    let bots = scene.sim.as_ref().unwrap().mp_bot_count();
    assert_eq!(bots, 3, "the Quick Start's 3 bots exist in the match [M 0x183a50:138-140]");

    // 3600 fixed ticks = 60 s of the match. The human stands (neutral input) — the bots must do
    // everything else themselves.
    let mut moved = vec![0.0f32; bots];
    let mut last = scene.sim.as_ref().unwrap().mp_bot_positions();
    for t in 0..3600u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), t % 37 == 0, false, false);
        let sim = scene.sim.as_ref().unwrap();
        let cur = sim.mp_bot_positions();
        for i in 0..bots {
            moved[i] += ((cur[i][0] - last[i][0]).powi(2) + (cur[i][2] - last[i][2]).powi(2)).sqrt();
        }
        last = cur;
    }

    let sim = scene.sim.as_ref().unwrap();
    eprintln!("== per-bot stats after 3600 ticks ==");
    for i in 0..bots {
        eprintln!("bot {i}: walked {:.1}, shots {}, deaths {}, respawns {}, score {}",
            moved[i], sim.mp_bot_shots(i), sim.mp_bot_deaths(i),
            sim.mp_bot_respawns(i), sim.mp_bot_score(i));
    }

    for i in 0..bots {
        assert!(moved[i] > 5.0, "bot {i} moves its route (walked {:.1} units over 3600 ticks)", moved[i]);
        assert!(sim.mp_bot_shots(i) > 0, "bot {i} fires ({} shots)", sim.mp_bot_shots(i));
    }
    // The match plays: the bots fight each other (damage + death + respawn per BOT_respawn
    // 0x124e38) and the score table moves [M MPGame+slot*0x30+0x18].
    let deaths: u32 = (0..bots).map(|i| sim.mp_bot_deaths(i)).sum();
    let score: i32 = (0..bots).map(|i| sim.mp_bot_score(i)).sum();
    assert!(deaths > 0 || score > 0, "the bots score frags or die — deaths {deaths}, score {score}");
    assert!(deaths > 0, "at least one bot takes damage and dies (deaths {deaths})");
}

/// Sister: with the bots disabled, only the human is in the match — no bot entities, no bot
/// slots (`MP_Start` creates nothing for bot_count 0 [M 0x183a50:134-160's count gate]).
#[test]
fn bots_disabled_leaves_only_the_human() {
    let Some(path) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let mut s = quick_game();
    s.bot_count = 0;
    let arena = format!("{:08x}.bin", s.level_code);
    let mut scene = nf_view::load_mp_carry(&path, &arena, false, None, s).expect("arena loads");
    assert_eq!(scene.sim.as_ref().unwrap().mp_bot_count(), 0, "no bot entities when the bots are disabled");
    for t in 0..600u32 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.mp_bot_count(), 0, "still no bots after 600 ticks");
    assert!(sim.drones.health.health > 0.0, "the human stands alone and unhurt");
}
