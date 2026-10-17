//! Game simulation for 007: Nightfire (PS2), ported from ACTION.ELF: the fixed-tick world the viewer (and later the
//! game) runs. Rendering lives elsewhere; this crate only advances state.

// Two match arms for the same (state, message) leave the second dead: a merge once renamed the guard's
// GoToGoalPosition (0x63) arms to the ally's 0x28, freezing guards on their way to a goal, and only this lint saw it.
#![deny(unreachable_patterns)]

pub mod aipoint;
pub mod aivolume;
pub mod prof;
pub mod ally;
pub mod anim;
pub mod bullet;
pub mod car;
pub mod corona;
pub mod cover;
pub mod creepwall;
pub mod damage;
pub mod door;
pub mod drone;
pub mod env;
pub mod explode;
pub mod gun;
pub mod grenade;
pub mod ladder;
pub mod kiko;
pub mod input;
pub mod light;
pub mod mp;
pub mod fusebox;
pub mod gadget;
pub mod gas;
pub mod copter;
pub mod casing;
pub mod effect;
pub mod lock;
pub mod hurt;
pub mod ss;
pub mod sensor;
pub mod mission;
pub mod ptp;
pub mod driving;
pub mod driving_hud;
pub mod music;
pub mod death;
pub mod nav;
pub mod nis;
pub mod props;
pub mod pickup;
pub mod player;
pub mod progress;
pub mod secret_codes;
pub mod profile;
pub mod stats;
pub mod ps2math;
pub mod searchlight;
pub mod sky;
pub mod sound;
pub mod spawners;
pub mod space_missile;
pub mod trig;
pub mod triggers;
pub mod tuning;
pub mod useobj;
pub mod wire;
pub mod world;
pub mod worldobj;
pub mod world_fx;
pub mod subtitle;
