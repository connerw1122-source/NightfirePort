//! The locks (`Lock_Create` 0x1ae1f8, `Lock_Update` 0x1ae3d8, `Lock_Activate` 0x1aed00,
//! `Lock_Unlock` 0x1b0650; placement type 0x2E, `parsemap_create_dynamic_objects` case 0x2E —
//! fix-objects2): the burnable locks the Q-Laser / laser watch burns through — 07000008's
//! `Object_SafeBolt_1` ×2 (the safe's two bolts, out channels 134/135 [D #207/#208]; mux #278
//! ORs them into ch 136, which opens the `Object_SafeDoor` SimpleScript and plays
//! `ScriptPlayer_SafePickUps`).
//!
//! The game's model [M]:
//! - `Lock_Create` (0x1ae1f8): `control_create_object(0x20, …)`, class 0x20; the record's u16 words
//!   at +4/+6 from placement params 3/4 (0xffff here — the take/key channels), the burn timer base
//!   `u16[+8] = (tag+2 == 0 ? 0 : tag+2 − 1)` scaled by `FRAME_RATE / u16[tag+4]`, flags2 |= 0x44,
//!   and the framelist pose (Spline_Interp) when the key count is non-zero. The difficulty gate
//!   (`0x1ae1f8:67-69`, GameState+0x40 DECIMAL = +0x28 hex = 0x2a3790): `level_tag+0x44` (param 6)
//!   non-zero AND the selected difficulty == 1 (Operative) -> `Lock_Unlock(obj, 1)` at create,
//!   which sets the lock's state word +0xf4 = 1 (unlocked) [M Lock_Unlock 0x1b0650:28-31] — the
//!   tagged locks start open on Operative only (docs/systems/progress.md "doors auto-unlock").
//! - `Lock_Update` (0x1ae3d8) case 0 reads the ACTIVATING weapon off the hit: 0x4E/0x4F (the
//!   Q-Decryptor, :57) or the laser family — the `− 0x56 < 2` band at :162 (0x56/0x57; the watch's
//!   scoped laser 0x54 arrives through the same hit path) — and advances the burn; the open writes
//!   `switch_channels[record u16[+8]] = 1` (:194-195) — placement param 5 [D #207: 0x86 = 134].
//! - `Lock_Activate` (0x1aed00): the E-interaction plays the locked/burnt sounds only; the burn is
//!   the laser's doing, not the use key.
//!
//! Port status (fix-party-mission): FIXED in play — the Lazer Watch 0x4E/0x4F's bullets burn the
//! lock through the bullet pass's segment-vs-sphere contact (`Lock_Update` case 0's cell walk, the
//! weapon id already on the port's BU), and the burn is instant (the create-time base is 0 on this
//! level's data [M 0x1ae1f8 — 07000008's #207/#208 param 2 = 0]); the chain test drives both bolts
//! through the real path (watch select, aim, fire) to ch 136. The watch's fp view-model beam and
//! the bolt meshes are still render-side work (0x2E is not in the port's dynamic draw list). The
//! gadget itself was previously not ported (docs/port-map/gadgets.md — all of 0x4A..0x5E are out of
//! selection), so the burn was a method the (future) `Player_LaserPointer` 0x1a65d8 calls per tick
//! while its ray hits the lock's world sphere, and tests could drive it directly. The framelist
//! open-animation is not posed — render-side, noted in the report.
//! ported (docs/port-map/gadgets.md — all of 0x4A..0x5E are out of selection), so the burn is a
//! method the (future) `Player_LaserPointer` 0x1a65d8 calls per tick while its ray hits the lock's
//! world sphere, and tests can drive it directly. The bolt meshes are not drawn yet (the 0x2E
//! placements are not in the port's dynamic draw list) and the framelist open-animation is not
//! posed — both render-side, noted in the report.

use crate::triggers::Channels;
use crate::world::V3;
use nf_formats::placement::Placement;

#[derive(Clone, Debug)]
pub struct Lock {
    pub placement: usize,
    /// The object's world sphere (the create's `build_LinkToRoom` position + the glist bounds).
    pub pos: V3,
    pub radius: f32,
    /// Record u16[+8] = placement param 5: the channel written on burn-through [M 0x1ae3d8:194].
    pub out: u16,
    /// Placement params 3/4 (the record's +4/+6 key/take words, 0xffff on the safe bolts [D]).
    pub key_channels: [u16; 2],
    /// Ticks of laser burn applied (`Lock_Update`'s timer). The create-time base is 0 on this
    /// level's data [M 0x1ae1f8: the tag+2 word is 0], so the first tick of burn completes it.
    pub burn: u32,
    pub burned: bool,
    /// level_tag+0x44 (param 6): the "starts unlocked on Operative" tag [M 0x1ae1f8:67-69].
    pub unlock_on_operative: bool,
    /// The create-time unlock state (the game's lock state word +0xf4 = 1 via `Lock_Unlock`
    /// 0x1b0650:28-31): true iff `unlock_on_operative` and the selected difficulty is 1 [M].
    pub unlocked: bool,
}

impl Lock {
    /// `Lock_Create` over the main map's 0x2E placements. `difficulty` is the selected difficulty
    /// (`GameState+0x28`, 0x2a3790) — the param-6 tag auto-unlocks on Operative [M 0x1ae1f8:67-69].
    pub fn create(placements: &[Placement], difficulty: u32, object: impl Fn(&Placement) -> Option<(V3, f32)>) -> Vec<Lock> {
        let mut out = Vec::new();
        for (i, p) in placements.iter().enumerate() {
            if p.flags & 0xff != 0x2e { continue; }
            let param = |k: u32| p.params.iter().find(|(j, _)| *j == k).map_or(0, |&(_, v)| v);
            let (centre, radius) = object(p).unwrap_or((p.translation, 1.0));
            out.push(Lock {
                placement: i,
                pos: [p.translation[0] + centre[0], p.translation[1] + centre[1], p.translation[2] + centre[2]],
                radius,
                out: param(5) as u16,
                key_channels: [param(3) as u16, param(4) as u16],
                burn: 0,
                burned: false,
                unlock_on_operative: param(6) != 0,
                unlocked: param(6) != 0 && difficulty == 1,
            });
        }
        out
    }

    /// One tick of the laser's burn (`Player_LaserPointer` 0x1a65d8's hook — the gadget is not
    /// ported; see the module doc). `Lock_Update` writes the out channel when the burn completes
    /// [M 0x1ae3d8:194-195]. Returns whether it completed this tick.
    pub fn burn(&mut self, ch: &mut Channels, frame: u32) -> bool {
        if self.burned || self.unlocked { return false; }
        self.burned = true; // the create-time burn base is 0 [M 0x1ae1f8]
        if self.out != 0 { ch.set(self.out as u32, 1, frame); }
        true
    }

    /// The lock of a placement index (the render's per-instance lookup, when the meshes land).
    pub fn of_placement(list: &[Lock], placement: usize) -> Option<usize> {
        list.iter().position(|l| l.placement == placement)
    }
}
