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
//!   and the framelist pose (Spline_Interp) when the key count is non-zero. `GameState+0x40 == 1`
//!   (a loaded save) unlocks at once.
//! - `Lock_Update` (0x1ae3d8) case 0 reads the ACTIVATING weapon off the hit: 0x4E/0x4F (the
//!   Q-Decryptor, :57) or the laser family — the `− 0x56 < 2` band at :162 (0x56/0x57; the watch's
//!   scoped laser 0x54 arrives through the same hit path) — and advances the burn; the open writes
//!   `switch_channels[record u16[+8]] = 1` (:194-195) — placement param 5 [D #207: 0x86 = 134].
//! - `Lock_Activate` (0x1aed00): the E-interaction plays the locked/burnt sounds only; the burn is
//!   the laser's doing, not the use key.
//!
//! Port status: the record, the out channel and the burn hook exist; the gadget itself is not
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
}

impl Lock {
    /// `Lock_Create` over the main map's 0x2E placements.
    pub fn create(placements: &[Placement], object: impl Fn(&Placement) -> Option<(V3, f32)>) -> Vec<Lock> {
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
            });
        }
        out
    }

    /// One tick of the laser's burn (`Player_LaserPointer` 0x1a65d8's hook — the gadget is not
    /// ported; see the module doc). `Lock_Update` writes the out channel when the burn completes
    /// [M 0x1ae3d8:194-195]. Returns whether it completed this tick.
    pub fn burn(&mut self, ch: &mut Channels, frame: u32) -> bool {
        if self.burned { return false; }
        self.burned = true; // the create-time burn base is 0 [M 0x1ae1f8]
        if self.out != 0 { ch.set(self.out as u32, 1, frame); }
        true
    }

    /// The lock of a placement index (the render's per-instance lookup, when the meshes land).
    pub fn of_placement(list: &[Lock], placement: usize) -> Option<usize> {
        list.iter().position(|l| l.placement == placement)
    }
}
