//! Game simulation for 007: Nightfire (PS2), ported from ACTION.ELF: the fixed-tick world the viewer (and later the
//! game) runs. Rendering lives elsewhere; this crate only advances state.

pub mod bullet;
pub mod cover;
pub mod drone;
pub mod nav;
pub mod spawners;
pub mod trig;
pub mod triggers;
pub mod world;
