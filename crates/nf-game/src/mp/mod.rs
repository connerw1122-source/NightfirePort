//! Multiplayer (bots groundwork). Phase-4 data layer: the per-bot config
//! records, the `default_bot_stats` presets, the bot state table, and the
//! `BotGlobal` dispatch skeleton — see `bots.rs` and `docs/fields/bot-vars.md`.
//! Nothing in the sim calls this yet; the dispatch is additionally behind the
//! `mp-bots` feature (`#[cfg(any(test, feature = "mp-bots"))]`).

pub mod bots;
