//! The driving missions' model: the vehicle physics (`physics.rs`, task `drive-physics`), the
//! road network (`road.rs`, the `RNgp` article) and the AI car step (`ai.rs`, task
//! `drive-ai-traffic`). docs/systems/drive-physics.md and docs/systems/driving-ai.md are the
//! write-ups; `ai::SimCar` is the physics seam (506's `Car` plugs in — see `ai::AiDriver`).

pub mod ai;
// 647 drive-mission-scripts: the mission state machine (objectives, fails, timers, the PTP end).
pub mod mission;
pub mod physics;
pub mod road;

// 506's physics landed as the single file `driving.rs`; this task moved it to `physics.rs`
// (507's `driving/` directory made the two layouts collide, rustc E0761). The re-export keeps
// every public path 506's tests and callers use: `nf_game::driving::{Car, CarInput, CarParams,
// FlatWorld, Rigid, Tuning, DT, Friction, SURFACES}`.
pub use physics::{Car, CarInput, CarParams, DT, FlatWorld, Friction, Rigid, SURFACES, Tuning};
