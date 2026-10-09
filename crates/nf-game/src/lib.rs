//! Game simulation for 007: Nightfire (PS2), ported from ACTION.ELF: the fixed-tick world the viewer (and later the
//! game) runs. Rendering lives elsewhere; this crate only advances state.

pub mod aivolume;
pub mod anim;
pub mod bullet;
pub mod cover;
pub mod damage;
pub mod drone;
pub mod gun;
pub mod nav;
pub mod pickup;
pub mod player;
pub mod spawners;
pub mod trig;
pub mod triggers;
pub mod tuning;
pub mod world;
