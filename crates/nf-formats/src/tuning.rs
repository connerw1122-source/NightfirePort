//! The tuning globals `ReadTuningVars` (ACTION.ELF 0x1d3a80) sets per level: drone damage / armour / firing, captain
//! modifiers, Bond's damage modifiers, aim speeds and auto-aim. This reads their static values (the build's defaults,
//! kept by levels the function's switch leaves alone); `nf_game::tuning::read_tuning_vars` applies the per-level
//! switch. Field names are the ELF symbol names. Spec: docs/systems/drone-damage.md.

use crate::buf::{FormatError, Result};
use crate::elf::Elf;

macro_rules! tuning {
    ($($f:ident),* $(,)?) => {
        #[allow(non_snake_case)]
        #[derive(Clone, Debug, PartialEq, Default)]
        pub struct Tuning { $(pub $f: f32),* }

        impl Tuning {
            /// The symbol names, in field order.
            pub const NAMES: &'static [&'static str] = &[$(stringify!($f)),*];

            pub fn get(&self, name: &str) -> Option<f32> { match name { $(stringify!($f) => Some(self.$f),)* _ => None } }

            fn set(&mut self, name: &str, v: f32) -> bool { match name { $(stringify!($f) => { self.$f = v; true })* _ => false } }
        }
    };
}

tuning!(
    Plr_AimSpeed_X, Plr_AimSpeed_Y, Plr_AimTurnSpeed_X, Plr_AimTurnSpeed_Y, Plr_ScopeSpeed_X, Plr_ScopeSpeed_Y,
    Plr_ScopeSpeed_Y_Mul, Plr_NoAimTurnSpeed_X, Plr_NoAimTurnSpeed_Y, Plr_NoAimTurnSpeed_Y_Mul, Plr_NoAimTurnSpeed_Y_Steps,
    ContinueHealthBoostHard, Autoaim_Angle_H, Autoaim_Angle_V, Autoaim_Range, Autoaim_LockOnMul, Autoaim_EasyMul,
    Autoaim_NormalMul, Plr_ScopeSpeed_X_Mul, Plr_ScopeSpeed_X_Steps, Plr_ScopeSpeed_Y_Steps, Plr_NoAimTurnSpeed_X_Mul,
    Plr_NoAimTurnSpeed_X_Steps, ContinueHealthBoostEasy, ContinueHealthBoostMedium, Autoaim_HardMul, Plr_DMod_Hard,
    DroneDamage_Hard, DroneFiring_Accuracy_Easy, DroneFiring_Accuracy_Normal, Plr_DMod_Easy, Plr_DMod_Normal,
    DroneDamage_Normal, DroneFiring_Accuracy_Hard, DroneDamage_Easy, DroneDamage_Head, DroneDamage_Legs, DroneDamage_Arms,
    DroneDamage_Torso, DroneArmour_Helmet, DroneArmour_Combat, DroneArmour_Jacket, DroneArmour_Vest,
    DroneFiring_BurstDelay_Min, DroneFiring_BurstDelay_Normal, DroneFiring_BurstDelay_Max, DroneFiring_BurstDelay_MinDist,
    DroneFiring_BurstDelay_MaxDist, DroneFiring_NewSighting_TimeToHit, DroneFiring_TargetFirstMoved_TimeToHit,
    DroneFiring_TargetFirstMoved_Accuracy, DroneFiring_TargetMoving_Accuracy, DroneFiring_TargetFirstStopped_TimeToHit,
    DroneFiring_TargetFirstStopped_Accuracy, DroneFiring_TooClose_Distance, DroneFiring_TooClose_Accuracy,
    DroneFiring_TooClose_Damage, DroneFiring_PlayerBackShot_Damage, DroneCaptain_Mod_BulletDamage,
    DroneCaptain_Mod_BulletAccuracy, DroneCaptain_Mod_Health, Plr_DMod_Multi, Plr_DMod_Head, Plr_DMod_LowerLimb,
    Plr_DMod_UpperLimb,
);

impl Tuning {
    /// The static values of every tuning global in ACTION.ELF (4-byte floats, found by symbol).
    pub fn from_action_elf(data: &[u8]) -> Result<Tuning> {
        let elf = Elf::parse(data)?;
        let syms = elf.symbols(data)?;
        let segs = elf.load_segments(data)?;
        let mut t = Tuning::default();
        for &name in Tuning::NAMES {
            let addr = syms.iter().find(|s| s.name == name).map(|s| s.value)
                .ok_or_else(|| FormatError::Invalid(format!("ACTION.ELF has no symbol {name}")))?;
            let v = segs.iter().find_map(|(va, b, _)| (addr >= *va && (addr - va) as usize + 4 <= b.len())
                .then(|| f32::from_le_bytes(b[(addr - va) as usize..][..4].try_into().unwrap())))
                .unwrap_or(0.0); // .bss: zero at load
            t.set(name, v);
        }
        Ok(t)
    }

    /// The symbol address of each field (for checks against the game's own code).
    pub fn addresses(data: &[u8]) -> Result<Vec<(&'static str, u32)>> {
        let elf = Elf::parse(data)?;
        let syms = elf.symbols(data)?;
        Tuning::NAMES.iter().map(|&n| syms.iter().find(|s| s.name == n).map(|s| (n, s.value))
            .ok_or_else(|| FormatError::Invalid(format!("ACTION.ELF has no symbol {n}")))).collect()
    }
}
