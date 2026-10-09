//! `ReadTuningVars` (ACTION.ELF 0x1d3a80): the per-level values of the tuning globals, applied over the ELF's static
//! values (`nf_formats::tuning::Tuning::from_action_elf`) at level load. Checked against the game's own function in the
//! R5900 interpreter for every level (tests/tuning_live.rs). Spec: docs/systems/drone-damage.md.

pub use nf_formats::tuning::Tuning;

/// The game's `ReadTuningVars` for `level` (`GameState+12`): common values, then the level group's, then for some
/// groups a shared tail. Levels outside every group keep the values they had.
pub fn read_tuning_vars(t: &mut Tuning, level: u32) {
    // Generated from the decomp of ReadTuningVars (0x1d3a80) by gen_tuning.py; values are the exact bit patterns.
    t.Plr_AimSpeed_X = f32::from_bits(0x3e028f5c);
    t.Plr_AimSpeed_Y = f32::from_bits(0x3e0b4396);
    t.Plr_AimTurnSpeed_X = f32::from_bits(0x3d4ccccd);
    t.Plr_AimTurnSpeed_Y = f32::from_bits(0x3d75c28f);
    t.Plr_ScopeSpeed_X = f32::from_bits(0x3c75c28f);
    t.Plr_ScopeSpeed_Y = f32::from_bits(0x3c343958);
    t.Plr_ScopeSpeed_Y_Mul = f32::from_bits(0x40400000);
    t.Plr_NoAimTurnSpeed_X = f32::from_bits(0x3d23d70a);
    t.Plr_NoAimTurnSpeed_Y = f32::from_bits(0x3c23d70a);
    t.Plr_NoAimTurnSpeed_Y_Mul = f32::from_bits(0x40000000);
    t.Plr_NoAimTurnSpeed_Y_Steps = f32::from_bits(0x42f00000);
    t.ContinueHealthBoostHard = f32::from_bits(0x42480000);
    t.Autoaim_Angle_H = f32::from_bits(0x3df5c28f);
    t.Autoaim_Angle_V = f32::from_bits(0x3e6147ae);
    t.Autoaim_Range = f32::from_bits(0x41c80000);
    t.Autoaim_LockOnMul = f32::from_bits(0x3fb33333);
    t.Autoaim_EasyMul = f32::from_bits(0x3ff33333);
    t.Autoaim_NormalMul = f32::from_bits(0x3f800000);
    t.Plr_ScopeSpeed_X_Mul = f32::from_bits(0x40400000);
    t.Plr_ScopeSpeed_X_Steps = f32::from_bits(0x42f00000);
    t.Plr_ScopeSpeed_Y_Steps = f32::from_bits(0x42f00000);
    t.Plr_NoAimTurnSpeed_X_Mul = f32::from_bits(0x40000000);
    t.Plr_NoAimTurnSpeed_X_Steps = f32::from_bits(0x42f00000);
    t.ContinueHealthBoostEasy = f32::from_bits(0x42480000);
    t.ContinueHealthBoostMedium = f32::from_bits(0x42480000);
    t.Autoaim_HardMul = f32::from_bits(0);
    let tail = match level {
        0x07000001 | 0x07000002 | 0x07000003 | 0x07000004 => {
            t.Plr_DMod_Hard = f32::from_bits(0x3f99999a);
            t.DroneDamage_Hard = f32::from_bits(0x3f4ccccd);
            t.DroneFiring_Accuracy_Easy = f32::from_bits(0x3f000000);
            t.DroneFiring_Accuracy_Normal = f32::from_bits(0x3f333333);
            t.Plr_DMod_Easy = f32::from_bits(0x3f000000);
            t.Plr_DMod_Normal = f32::from_bits(0x3f333333);
            t.DroneDamage_Normal = f32::from_bits(0x3f800000);
            t.DroneFiring_Accuracy_Hard = f32::from_bits(0x3f800000);
            true
        }
        0x07000005 | 0x07000006 | 0x07000007 | 0x07000008 => {
            t.Plr_DMod_Easy = f32::from_bits(0x3f19999a);
            t.Plr_DMod_Normal = f32::from_bits(0x3f333333);
            t.DroneFiring_Accuracy_Easy = f32::from_bits(0x3f4ccccd);
            t.DroneFiring_Accuracy_Normal = f32::from_bits(0x3f666666);
            t.DroneFiring_Accuracy_Hard = f32::from_bits(0x3fa66666);
            t.Plr_DMod_Hard = f32::from_bits(0x3f800000);
            t.DroneDamage_Normal = f32::from_bits(0x3f800000);
            t.DroneDamage_Hard = f32::from_bits(0x3f4ccccd);
            true
        }
        0x07000009 | 0x0700000a | 0x0700000b => {
            t.DroneDamage_Easy = f32::from_bits(0x3fc00000);
            t.DroneDamage_Normal = f32::from_bits(0x3f800000);
            t.DroneDamage_Hard = f32::from_bits(0x3f4ccccd);
            t.DroneDamage_Head = f32::from_bits(0x41200000);
            t.DroneDamage_Legs = f32::from_bits(0x3f400000);
            t.DroneDamage_Arms = f32::from_bits(0x3f800000);
            t.DroneDamage_Torso = f32::from_bits(0x3f800000);
            t.DroneArmour_Helmet = f32::from_bits(0x3f800000);
            t.DroneArmour_Combat = f32::from_bits(0x3e800000);
            t.DroneArmour_Jacket = f32::from_bits(0x3f000000);
            t.DroneArmour_Vest = f32::from_bits(0x3f400000);
            t.DroneFiring_BurstDelay_Min = f32::from_bits(0x41700000);
            t.DroneFiring_BurstDelay_Normal = f32::from_bits(0x42340000);
            t.DroneFiring_BurstDelay_Max = f32::from_bits(0x42700000);
            t.DroneFiring_BurstDelay_MinDist = f32::from_bits(0x40800000);
            t.DroneFiring_BurstDelay_MaxDist = f32::from_bits(0x41f00000);
            t.DroneFiring_Accuracy_Easy = f32::from_bits(0x3f4ccccd);
            t.DroneFiring_Accuracy_Normal = f32::from_bits(0x3f800000);
            t.DroneFiring_Accuracy_Hard = f32::from_bits(0x3f800000);
            t.DroneFiring_NewSighting_TimeToHit = f32::from_bits(0x40000000);
            t.DroneFiring_TargetFirstMoved_TimeToHit = f32::from_bits(0x40000000);
            t.DroneFiring_TargetFirstMoved_Accuracy = f32::from_bits(0x3e800000);
            t.DroneFiring_TargetMoving_Accuracy = f32::from_bits(0x3f400000);
            t.DroneFiring_TargetFirstStopped_TimeToHit = f32::from_bits(0x40000000);
            t.DroneFiring_TargetFirstStopped_Accuracy = f32::from_bits(0x3f800000);
            t.DroneFiring_TooClose_Distance = f32::from_bits(0x40400000);
            t.DroneFiring_TooClose_Accuracy = f32::from_bits(0x40000000);
            t.DroneFiring_TooClose_Damage = f32::from_bits(0x40000000);
            t.DroneFiring_PlayerBackShot_Damage = f32::from_bits(0x40000000);
            t.DroneCaptain_Mod_BulletDamage = f32::from_bits(0x40000000);
            t.DroneCaptain_Mod_BulletAccuracy = f32::from_bits(0x40000000);
            t.DroneCaptain_Mod_Health = f32::from_bits(0x40000000);
            t.ContinueHealthBoostEasy = f32::from_bits(0x42480000);
            t.ContinueHealthBoostMedium = f32::from_bits(0x42480000);
            t.ContinueHealthBoostHard = f32::from_bits(0x42480000);
            t.Plr_DMod_Easy = f32::from_bits(0x3f19999a);
            t.Plr_DMod_Normal = f32::from_bits(0x3f4ccccd);
            t.Plr_DMod_Hard = f32::from_bits(0x3f800000);
            t.Plr_AimSpeed_X = f32::from_bits(0x3e028f5c);
            t.Plr_AimSpeed_Y = f32::from_bits(0x3e0b4396);
            t.Plr_AimTurnSpeed_X = f32::from_bits(0x3d4ccccd);
            t.Plr_AimTurnSpeed_Y = f32::from_bits(0x3d75c28f);
            t.Plr_ScopeSpeed_X = f32::from_bits(0x3c75c28f);
            t.Plr_ScopeSpeed_X_Mul = f32::from_bits(0x40400000);
            t.Plr_ScopeSpeed_X_Steps = f32::from_bits(0x42f00000);
            t.Plr_ScopeSpeed_Y = f32::from_bits(0x3c343958);
            t.Plr_ScopeSpeed_Y_Mul = f32::from_bits(0x40400000);
            t.Plr_ScopeSpeed_Y_Steps = f32::from_bits(0x42f00000);
            t.Plr_NoAimTurnSpeed_X = f32::from_bits(0x3d23d70a);
            t.Plr_NoAimTurnSpeed_X_Mul = f32::from_bits(0x40000000);
            t.Plr_NoAimTurnSpeed_X_Steps = f32::from_bits(0x42f00000);
            t.Plr_NoAimTurnSpeed_Y = f32::from_bits(0x3c23d70a);
            t.Plr_NoAimTurnSpeed_Y_Mul = f32::from_bits(0x40000000);
            t.Plr_NoAimTurnSpeed_Y_Steps = f32::from_bits(0x42f00000);
            t.Autoaim_Angle_H = f32::from_bits(0x3df5c28f);
            t.Autoaim_Angle_V = f32::from_bits(0x3e6147ae);
            t.Autoaim_Range = f32::from_bits(0x41c80000);
            t.Autoaim_LockOnMul = f32::from_bits(0x3fb33333);
            t.Autoaim_EasyMul = f32::from_bits(0x3ff33333);
            t.Autoaim_NormalMul = f32::from_bits(0x3f800000);
            t.Autoaim_HardMul = f32::from_bits(0);
            false
        }
        0x0700000c | 0x0700000d => {
            t.Plr_DMod_Easy = f32::from_bits(0x3ecccccd);
            t.Plr_DMod_Normal = f32::from_bits(0x3f333333);
            t.DroneDamage_Head = f32::from_bits(0x41200000);
            t.DroneArmour_Jacket = f32::from_bits(0x3f000000);
            t.DroneFiring_BurstDelay_Min = f32::from_bits(0x41700000);
            t.DroneFiring_BurstDelay_Normal = f32::from_bits(0x42340000);
            t.DroneFiring_BurstDelay_Max = f32::from_bits(0x42700000);
            t.DroneFiring_BurstDelay_MinDist = f32::from_bits(0x40800000);
            t.DroneFiring_BurstDelay_MaxDist = f32::from_bits(0x41f00000);
            t.DroneFiring_Accuracy_Easy = f32::from_bits(0x3f19999a);
            t.DroneFiring_Accuracy_Normal = f32::from_bits(0x3f4ccccd);
            t.DroneFiring_Accuracy_Hard = f32::from_bits(0x3f99999a);
            t.DroneFiring_TargetFirstMoved_Accuracy = f32::from_bits(0x3e800000);
            t.DroneFiring_TargetMoving_Accuracy = f32::from_bits(0x3f400000);
            t.DroneFiring_TargetFirstStopped_Accuracy = f32::from_bits(0x3dcccccd);
            t.DroneFiring_TargetFirstStopped_TimeToHit = f32::from_bits(0x3f800000);
            t.DroneFiring_TooClose_Distance = f32::from_bits(0x40400000);
            t.Plr_DMod_Hard = f32::from_bits(0x3f99999a);
            t.DroneDamage_Easy = f32::from_bits(0x3fc00000);
            t.DroneDamage_Normal = f32::from_bits(0x3f800000);
            t.DroneDamage_Hard = f32::from_bits(0x3f4ccccd);
            t.DroneDamage_Legs = f32::from_bits(0x3f400000);
            t.DroneDamage_Arms = f32::from_bits(0x3f800000);
            t.DroneDamage_Torso = f32::from_bits(0x3f800000);
            t.DroneArmour_Helmet = f32::from_bits(0x3f800000);
            t.DroneArmour_Combat = f32::from_bits(0x3e800000);
            t.DroneArmour_Vest = f32::from_bits(0x3f400000);
            t.DroneFiring_NewSighting_TimeToHit = f32::from_bits(0x40000000);
            t.DroneFiring_TargetFirstMoved_TimeToHit = f32::from_bits(0x40000000);
            t.DroneFiring_TooClose_Accuracy = f32::from_bits(0x40000000);
            t.DroneFiring_TooClose_Damage = f32::from_bits(0x40000000);
            t.DroneFiring_PlayerBackShot_Damage = f32::from_bits(0x40000000);
            t.DroneCaptain_Mod_BulletDamage = f32::from_bits(0x3fc00000);
            t.DroneCaptain_Mod_BulletAccuracy = f32::from_bits(0x3fc00000);
            t.DroneCaptain_Mod_Health = f32::from_bits(0x40000000);
            false
        }
        0x07000011 | 0x07000012 | 0x07000013 | 0x0700004a => {
            t.Plr_DMod_Normal = f32::from_bits(0x3f19999a);
            t.DroneDamage_Normal = f32::from_bits(0x3f99999a);
            t.DroneFiring_Accuracy_Easy = f32::from_bits(0x3f333333);
            t.DroneFiring_Accuracy_Normal = f32::from_bits(0x3f4ccccd);
            t.Plr_DMod_Easy = f32::from_bits(0x3f000000);
            t.Plr_DMod_Hard = f32::from_bits(0x3f800000);
            t.DroneDamage_Hard = f32::from_bits(0x3f800000);
            t.DroneFiring_Accuracy_Hard = f32::from_bits(0x3f800000);
            true
        }
        0x07000014 | 0x07000015 | 0x07000016 => {
            t.Plr_DMod_Easy = f32::from_bits(0x3ecccccd);
            t.Plr_DMod_Normal = f32::from_bits(0x3f19999a);
            t.DroneDamage_Hard = f32::from_bits(0x3f4ccccd);
            t.DroneFiring_Accuracy_Easy = f32::from_bits(0x3f000000);
            t.DroneFiring_Accuracy_Normal = f32::from_bits(0x3f333333);
            t.Plr_DMod_Hard = f32::from_bits(0x3f800000);
            t.DroneDamage_Normal = f32::from_bits(0x3f800000);
            t.DroneFiring_Accuracy_Hard = f32::from_bits(0x3f800000);
            true
        }
        0x0700001b => {
            t.Plr_DMod_Normal = f32::from_bits(0x3f333333);
            t.DroneFiring_Accuracy_Hard = f32::from_bits(0x3f99999a);
            t.DroneFiring_Accuracy_Normal = f32::from_bits(0x3f4ccccd);
            t.DroneFiring_Accuracy_Easy = f32::from_bits(0x3f19999a);
            t.DroneDamage_Hard = f32::from_bits(0x3f800000);
            t.DroneDamage_Easy = f32::from_bits(0x3f800000);
            t.Plr_DMod_Hard = f32::from_bits(0x3f800000);
            t.Plr_DMod_Easy = f32::from_bits(0x3f19999a);
            t.DroneFiring_TooClose_Distance = f32::from_bits(0x40400000);
            t.DroneFiring_TargetFirstStopped_Accuracy = f32::from_bits(0x3f800000);
            t.DroneFiring_TargetMoving_Accuracy = f32::from_bits(0x3f400000);
            t.DroneFiring_TargetFirstMoved_Accuracy = f32::from_bits(0x3e800000);
            t.DroneFiring_BurstDelay_MaxDist = f32::from_bits(0x41f00000);
            t.DroneFiring_BurstDelay_MinDist = f32::from_bits(0x40800000);
            t.DroneFiring_BurstDelay_Max = f32::from_bits(0x42700000);
            t.DroneFiring_BurstDelay_Normal = f32::from_bits(0x42340000);
            t.DroneFiring_BurstDelay_Min = f32::from_bits(0x41700000);
            t.DroneArmour_Vest = f32::from_bits(0x3f400000);
            t.DroneArmour_Jacket = f32::from_bits(0x3f000000);
            t.DroneArmour_Combat = f32::from_bits(0x3e800000);
            t.DroneArmour_Helmet = f32::from_bits(0x3f800000);
            t.DroneDamage_Torso = f32::from_bits(0x3f800000);
            t.DroneDamage_Arms = f32::from_bits(0x3f800000);
            t.DroneDamage_Legs = f32::from_bits(0x3f400000);
            t.DroneDamage_Head = f32::from_bits(0x41200000);
            t.DroneDamage_Normal = f32::from_bits(0x3f800000);
            t.DroneFiring_NewSighting_TimeToHit = f32::from_bits(0x40000000);
            t.DroneFiring_TargetFirstMoved_TimeToHit = f32::from_bits(0x40000000);
            t.DroneFiring_TargetFirstStopped_TimeToHit = f32::from_bits(0x40000000);
            t.DroneFiring_TooClose_Accuracy = f32::from_bits(0x40000000);
            t.DroneFiring_TooClose_Damage = f32::from_bits(0x40000000);
            t.DroneFiring_PlayerBackShot_Damage = f32::from_bits(0x40000000);
            t.DroneCaptain_Mod_BulletDamage = f32::from_bits(0x40000000);
            t.DroneCaptain_Mod_BulletAccuracy = f32::from_bits(0x40000000);
            t.DroneCaptain_Mod_Health = f32::from_bits(0x40000000);
            false
        }
        0x07000021 | 0x07000022 | 0x07000023 | 0x07000024 | 0x07000025 | 0x07000026 | 0x07000027 | 0x07000028 | 0x07000029 | 0x0700004b | 0x0700004c => {
            t.DroneDamage_Easy = f32::from_bits(0x40000000);
            t.DroneDamage_Normal = f32::from_bits(0x3fc00000);
            t.DroneDamage_Hard = f32::from_bits(0x3f000000);
            t.DroneDamage_Head = f32::from_bits(0x41200000);
            t.DroneDamage_Legs = f32::from_bits(0x3f400000);
            t.DroneDamage_Arms = f32::from_bits(0x3f800000);
            t.DroneDamage_Torso = f32::from_bits(0x3f800000);
            t.DroneArmour_Helmet = f32::from_bits(0x3f000000);
            t.DroneArmour_Combat = f32::from_bits(0x3e800000);
            t.DroneArmour_Jacket = f32::from_bits(0x3f000000);
            t.DroneArmour_Vest = f32::from_bits(0x3f400000);
            t.DroneFiring_BurstDelay_Min = f32::from_bits(0x41700000);
            t.DroneFiring_BurstDelay_Normal = f32::from_bits(0x42340000);
            t.DroneFiring_BurstDelay_Max = f32::from_bits(0x42700000);
            t.DroneFiring_BurstDelay_MinDist = f32::from_bits(0x40800000);
            t.DroneFiring_BurstDelay_MaxDist = f32::from_bits(0x41f00000);
            t.DroneFiring_Accuracy_Easy = f32::from_bits(0x3f4ccccd);
            t.DroneFiring_Accuracy_Normal = f32::from_bits(0x3f800000);
            t.DroneFiring_Accuracy_Hard = f32::from_bits(0x3fc00000);
            t.DroneFiring_NewSighting_TimeToHit = f32::from_bits(0x40000000);
            t.DroneFiring_TargetFirstMoved_TimeToHit = f32::from_bits(0x40000000);
            t.DroneFiring_TargetFirstMoved_Accuracy = f32::from_bits(0x3e800000);
            t.DroneFiring_TargetMoving_Accuracy = f32::from_bits(0x3f400000);
            t.DroneFiring_TargetFirstStopped_TimeToHit = f32::from_bits(0x40000000);
            t.DroneFiring_TargetFirstStopped_Accuracy = f32::from_bits(0x3f800000);
            t.DroneFiring_TooClose_Distance = f32::from_bits(0x40800000);
            t.DroneFiring_TooClose_Accuracy = f32::from_bits(0x40000000);
            t.DroneFiring_TooClose_Damage = f32::from_bits(0x40000000);
            t.DroneFiring_PlayerBackShot_Damage = f32::from_bits(0x40000000);
            t.DroneCaptain_Mod_BulletDamage = f32::from_bits(0x40000000);
            t.DroneCaptain_Mod_BulletAccuracy = f32::from_bits(0x40000000);
            t.DroneCaptain_Mod_Health = f32::from_bits(0x40000000);
            t.ContinueHealthBoostEasy = f32::from_bits(0x42480000);
            t.ContinueHealthBoostMedium = f32::from_bits(0x42480000);
            t.ContinueHealthBoostHard = f32::from_bits(0x42480000);
            t.Plr_DMod_Multi = f32::from_bits(0x40800000);
            t.Plr_DMod_Head = f32::from_bits(0x40800000);
            t.Plr_DMod_LowerLimb = f32::from_bits(0x3f4ccccd);
            t.Plr_DMod_UpperLimb = f32::from_bits(0x3f4ccccd);
            t.Plr_AimSpeed_X = f32::from_bits(0x3e028f5c);
            t.Plr_AimSpeed_Y = f32::from_bits(0x3e0b4396);
            t.Plr_AimTurnSpeed_X = f32::from_bits(0x3d4ccccd);
            t.Plr_AimTurnSpeed_Y = f32::from_bits(0x3d75c28f);
            t.Plr_ScopeSpeed_X = f32::from_bits(0x3c75c28f);
            t.Plr_ScopeSpeed_X_Mul = f32::from_bits(0x40400000);
            t.Plr_ScopeSpeed_X_Steps = f32::from_bits(0x42f00000);
            t.Plr_ScopeSpeed_Y = f32::from_bits(0x3c343958);
            t.Plr_ScopeSpeed_Y_Mul = f32::from_bits(0x40400000);
            t.Plr_ScopeSpeed_Y_Steps = f32::from_bits(0x42f00000);
            t.Plr_NoAimTurnSpeed_X = f32::from_bits(0x3d23d70a);
            t.Plr_NoAimTurnSpeed_X_Mul = f32::from_bits(0x40000000);
            t.Plr_NoAimTurnSpeed_X_Steps = f32::from_bits(0x42f00000);
            t.Plr_NoAimTurnSpeed_Y = f32::from_bits(0x3c23d70a);
            t.Plr_NoAimTurnSpeed_Y_Mul = f32::from_bits(0x40000000);
            t.Plr_NoAimTurnSpeed_Y_Steps = f32::from_bits(0x42f00000);
            t.Autoaim_Angle_H = f32::from_bits(0x3df5c28f);
            t.Autoaim_Angle_V = f32::from_bits(0x3e6147ae);
            t.Autoaim_Range = f32::from_bits(0x41c80000);
            t.Autoaim_LockOnMul = f32::from_bits(0x3fb33333);
            t.Autoaim_EasyMul = f32::from_bits(0x3ff33333);
            t.Autoaim_NormalMul = f32::from_bits(0x3f800000);
            t.Autoaim_HardMul = f32::from_bits(0);
            false
        }
        _ => {
            false
        }
    };
    if tail {
        t.DroneFiring_TooClose_Distance = f32::from_bits(0x40400000);
        t.DroneFiring_TargetFirstStopped_Accuracy = f32::from_bits(0x3f800000);
        t.DroneFiring_TargetMoving_Accuracy = f32::from_bits(0x3f400000);
        t.DroneFiring_TargetFirstMoved_Accuracy = f32::from_bits(0x3e800000);
        t.DroneFiring_BurstDelay_MaxDist = f32::from_bits(0x41f00000);
        t.DroneFiring_BurstDelay_MinDist = f32::from_bits(0x40800000);
        t.DroneFiring_BurstDelay_Max = f32::from_bits(0x42700000);
        t.DroneFiring_BurstDelay_Normal = f32::from_bits(0x42340000);
        t.DroneFiring_BurstDelay_Min = f32::from_bits(0x41700000);
        t.DroneArmour_Vest = f32::from_bits(0x3f400000);
        t.DroneArmour_Jacket = f32::from_bits(0x3f000000);
        t.DroneArmour_Combat = f32::from_bits(0x3e800000);
        t.DroneArmour_Helmet = f32::from_bits(0x3f800000);
        t.DroneDamage_Torso = f32::from_bits(0x3f800000);
        t.DroneDamage_Arms = f32::from_bits(0x3f800000);
        t.DroneDamage_Legs = f32::from_bits(0x3f400000);
        t.DroneDamage_Head = f32::from_bits(0x41200000);
        t.DroneDamage_Easy = f32::from_bits(0x3fc00000);
        t.DroneFiring_NewSighting_TimeToHit = f32::from_bits(0x40000000);
        t.DroneFiring_TargetFirstMoved_TimeToHit = f32::from_bits(0x40000000);
        t.DroneFiring_TargetFirstStopped_TimeToHit = f32::from_bits(0x40000000);
        t.DroneFiring_TooClose_Accuracy = f32::from_bits(0x40000000);
        t.DroneFiring_TooClose_Damage = f32::from_bits(0x40000000);
        t.DroneFiring_PlayerBackShot_Damage = f32::from_bits(0x40000000);
        t.DroneCaptain_Mod_BulletDamage = f32::from_bits(0x40000000);
        t.DroneCaptain_Mod_BulletAccuracy = f32::from_bits(0x40000000);
        t.DroneCaptain_Mod_Health = f32::from_bits(0x40000000);
    }
}
