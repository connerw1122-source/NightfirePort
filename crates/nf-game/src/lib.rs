//! Game simulation for 007: Nightfire (PS2), ported from ACTION.ELF: the fixed-tick world the viewer (and later the
//! game) runs. Rendering lives elsewhere; this crate only advances state.

pub mod aivolume;
pub mod anim;
pub mod bullet;
pub mod corona;
pub mod cover;
pub mod damage;
pub mod door;
pub mod drone;
pub mod env;
pub mod gun;
pub mod grenade;
pub mod ladder;
pub mod music;
pub mod nav;
pub mod nis;
pub mod pickup;
pub mod player;
pub mod searchlight;
pub mod sound;
pub mod spawners;
pub mod trig;
pub mod triggers;
pub mod tuning;
pub mod wire;
pub mod world;
