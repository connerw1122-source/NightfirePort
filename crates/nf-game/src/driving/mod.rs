//! The driving missions' AI: the road network (`road.rs`, the `RNgp` article) and the AI car step
//! (`ai.rs`). Task `drive-ai-traffic`; docs/systems/driving-ai.md is the write-up. 506's vehicle
//! physics (s/drive-physics) lands beside this module — `ai::SimCar` is the seam.

pub mod ai;
pub mod road;
