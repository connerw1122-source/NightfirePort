//! The fuseboxes (`FuseBox_Create` 0x1b0448, `FuseBox_Update` 0x1adfe8, `FuseBox_Register` 0x1b0970;
//! placement type 0x31 with a script hash in param 0 — 07000014's `OBJECT_FuseBox1`, the security
//! sabotage of "Sabotage base security and escape from room", and 07000008's GuardRoom box #219).
//!
//! The game's model [M]:
//! - `FuseBox_Create` stores param 1 → +0x54, param 2 → +0x56, param 3 → +0x5C, param 4 → +0x5E
//!   (two channel words per wire group), sets class 0x3C, and `SP_Create`s the box's script
//!   (param 0, mode 0x40 = preloaded) with `FuseBox_Register` as the create callback. The preload
//!   runs the script at load (`SP_CreateScriptPlayer` 0x1db9a8's load beat), so the box's mesh AND
//!   wires exist from frame 1 — nothing opens a lid.
//! - `FuseBox_Register` (0x1b0970) files every entity the script hands it into the box's two wire
//!   groups (10 slots each at +0x04/+0x2C) and counts them at +0x58/+0x5A. The hand-off is the
//!   script's event 0xF (`Script_EventHandler` 0x1d94c0:172-175 case 0xF passes the CURRENT
//!   stream's entity and the event's param 0 as the group; the port's decode of the real scripts:
//!   07000008's 0x060008AD stream 2 → group 0, 07000014's 0x06000A1B streams 2/6 → groups 0/1).
//! - `FuseBox_Update` runs per frame: for each group, each registered wire's hit list
//!   (`obj+0xD0`) is filtered with `Collide_FilterBullets(list, 0xfffd)` [M 0x1adfe8:~88] —
//!   class-5 (bullet) hitters whose `weapon_definition+0x10` category word ANDs with 0xfffd
//!   non-zero are DROPPED (`Collide_FilterBullets` 0x1ece30:24-31; the same drop-polarity the
//!   breakables use, `Break_Update` 0x12bfd8:15 mask `ext+4|0x106`, and MP demolition, mask 1).
//!   A wire whose hit list still holds an entry after the filter DIES (`Control_NextLOD` +
//!   `Effect_RemoveSObjs` — the wire's sub-objects are unlinked, the visual goes away).
//!   So wires die to hits that SURVIVE the filter: any non-class-5 hitter, and class-5 bullets
//!   whose category word is 0x0000 or 0x0002. On the disc [L weapon_data dump + USATxt.dat]:
//!   every gun is 0x0008 (filtered — gunfire never cuts a wire), while the **Laser** (0x4E/0x4F,
//!   the disc's own name) is 0x0002 and cuts; grenades/Ronin/Shaver are 0x0000. The laser watch
//!   (0x54) and the Decryptor (0x56) beams carry 0x0100 — filtered, they do NOT cut.
//!   Removing a **group 1** wire hurts Bond 5.0 (`Player_Hurt(0x40a00000, …)` at the dying wire)
//!   **iff +0x56 == 0** [M 0x1adfe8:99-101] — a configured param-2 channel disables the hurt —
//!   and zeroes group 0's second word +0x5C [M 0x1adfe8:102]. When a group's ALIVE count reaches
//!   0 with a non-zero registered count, both of its channel words are written
//!   (`switch_channels[ch] = 1` + timestamp) and the ext words are zeroed: group 0 → param 1 then
//!   param 3, group 1 → param 2 then param 4 [M 0x1adfe8:110-123] — one-shot by the zeroing.
//!
//! Port notes: the wire's world sphere is the registered script entity's matrix position [S — the
//! game collides the bullet's HITTEST against the wire's geometry; the radius 0.5 is unmeasured].
//! `Player_Activate` 0x1a5170 has NO class-0x3C arm [M 0x1a5170:95-160: SP(class 0 + obj+0xE4),
//! door 0x1C, trigger 0x1D, SS 0x25, car 0x28, GT 0x36, monitor 0x38, lock 0x3A, GunImp 0x47,
//! PCQWorm 0x49] — pressing E never reaches a fusebox; the old two-E-presses stand-in is gone.

use crate::triggers::Channels;
use nf_formats::placement::Placement;
use crate::world::V3;

/// One 5.0-damage live-wire hurt (`Player_Hurt(0x40a00000 = 5.0f32, …)`) [M 0x1adfe8:100].
pub const LIVE_WIRE_HURT: f32 = 5.0;

/// The wire groups' slot count (`FuseBox_Create`'s `control_create_object(0x60, …)`: two
/// `u32[10]` groups at +0x04/+0x2C [M 0x1b0448:12, 0x1b0970:22-25]).
pub const WIRE_SLOTS: usize = 10;

/// The hit filter's mask [M 0x1adfe8:88 `Collide_FilterBullets(wire+0xD0, 0xfffd)`].
pub const WIRE_HIT_MASK: u16 = 0xfffd;

/// A cut event the tick's update produced (drained by `Drones::tick`).
#[derive(Debug, PartialEq)]
pub enum Cut {
    /// The wire entity at `obj` died: its sub-objects are removed [M 0x1adfe8:93-96].
    RemoveWire { obj: usize },
    /// A group-1 wire (the entity at `obj`) died while +0x56 == 0: hurt Bond `LIVE_WIRE_HURT`
    /// at the wire's sphere [M 0x1adfe8:99-101] — the caller resolves the position (the wire is
    /// a script entity or anim object).
    LiveWireHurt { obj: usize },
}

#[derive(Default)]
pub struct Group {
    /// The registered wire entities (the `scripts.objects` indices `FuseBox_Register` filed,
    /// `None` once cut). Index = the game's slot (up to 10).
    pub slots: [Option<usize>; WIRE_SLOTS],
    /// The per-tick hit flag per slot: the wire's hit list held an entry that SURVIVED
    /// `Collide_FilterBullets(list, 0xfffd)` this tick [M 0x1adfe8:88-91]. Cleared every tick
    /// (the game's `Collide_Update` 0x1ea398 clears every obj+0xD0 at the head of the frame).
    hit: [bool; WIRE_SLOTS],
    /// +0x58/+0x5A: the registered count [M 0x1b0970:24].
    count: u16,
}

pub struct FuseBox {
    pub placement: usize,
    /// The box's world position (the placement translation + the object's centre).
    pub pos: V3,
    /// +0x54/+0x56/+0x5C/+0x5E: group 0's two channels, then group 1's (placement params 1–4).
    /// Zeroed when written, exactly like the game's ext [M 0x1adfe8:113-122] — that zeroing is
    /// what makes the tail one-shot.
    pub ch: [u16; 4],
    /// The two wire groups (the ext's +0x04/+0x2C slot arrays and +0x58/+0x5A counts).
    pub groups: [Group; 2],
}

impl FuseBox {
    /// `FuseBox_Create` over the main map's 0x31 placements that carry a script hash in param 0
    /// (07000014's `OBJECT_FuseBox1`, script 0x06000A3B). The E-interaction reach is the box's
    /// world sphere (`Control_BuildWorldSph`'s object centre + radius, the same helper the doors use).
    pub fn create(placements: &[Placement], object: impl Fn(&Placement) -> Option<(V3, f32)>) -> (Vec<FuseBox>, Vec<(V3, f32)>) {
        let mut out = Vec::new();
        let mut spheres = Vec::new();
        for (i, p) in placements.iter().enumerate() {
            if p.flags & 0xff != 0x31 { continue; }
            let param = |k: u32| p.params.iter().find(|(j, _)| *j == k).map_or(0, |&(_, v)| v);
            let script = param(0);
            // The game's fuseboxes preload a wire/lid script; a 0x31 without one is another class of
            // object (the pickup boxes — `Pickup_Create`'s own case) and keeps its old handling.
            if script < 0x0600_0000 { continue; }
            let (centre, radius) = object(p).unwrap_or((p.translation, 1.0));
            let pos = [p.translation[0] + centre[0], p.translation[1] + centre[1], p.translation[2] + centre[2]];
            out.push(FuseBox {
                placement: i,
                pos,
                ch: [param(1) as u16, param(2) as u16, param(3) as u16, param(4) as u16],
                groups: Default::default(),
            });
            spheres.push((pos, radius.max(1.0)));
        }
        (out, spheres)
    }

    /// `FuseBox_Register` (0x1b0970, the SP create-callback): `(obj, group)` — guarded by
    /// `group < 2`; a slot already holding the object is left alone (no duplicate); the first
    /// zero slot takes it and `count++` [M 0x1b0970:20-25].
    pub fn register_wire(&mut self, obj: usize, group: u16) {
        if group as usize >= 2 { return; }
        let g = &mut self.groups[group as usize];
        if g.slots.contains(&Some(obj)) { return; }
        let Some(k) = (0..WIRE_SLOTS).find(|&k| g.slots[k].is_none()) else { return };
        g.slots[k] = Some(obj);
        g.count += 1;
    }

    /// Every alive registered wire as `(group, slot, obj)` [M 0x1adfe8's slot walk].
    pub fn alive_wires(&self) -> impl Iterator<Item = (usize, usize, usize)> + '_ {
        (0..2).flat_map(move |g| (0..WIRE_SLOTS).filter_map(move |k| {
            self.groups[g].slots[k].map(|obj| (g, k, obj))
        }))
    }
    /// `Collide_FilterBullets(list, 0xfffd)`'s verdict for one bullet contact: a class-5 hitter
    /// whose category word (`weapon_definition+0x10` [M Bullet_init 0x12c5a0 ext+0x44 = the weapon
    /// def]) survives the filter (kept ⇔ `kind & 0xfffd == 0`) — else the node is dropped
    /// [M 0x1ece30:24-31]. `kind` = the bullet weapon's `weapon_data+0x10` [L dump: guns 0x0008,
    /// the Laser 0x0002, the watch/Decryptor beams 0x0100].
    pub fn hit_survives_filter(kind: u16) -> bool { kind & WIRE_HIT_MASK == 0 }

    /// A contact from this tick's bullet walk on the wire at `(group, slot)` whose kind word
    /// SURVIVED the filter (`Collide_Update` 0x1ea398 pushes every pick onto the hit object's
    /// `obj+0xD0`; the fusebox's filter then decides).
    pub fn mark_hit(&mut self, group: usize, slot: usize) {
        if group < 2 && slot < WIRE_SLOTS { self.groups[group].hit[slot] = true; }
    }

    /// Drop this tick's hit flags (`Collide_Update` 0x1ea398's first loop clears every hit list
    /// at the head of the frame).
    pub fn clear_hits(&mut self) {
        for g in &mut self.groups { g.hit = [false; WIRE_SLOTS]; }
    }

    /// `FuseBox_Update` (0x1adfe8) for one frame: a marked wire dies (its entity is removed,
    /// `Effect_RemoveSObjs` [M :93-96]); a group-1 death hurts iff +0x56 == 0 and cancels +0x5C
    /// [M :99-102]; a group whose alive count hit 0 with a non-zero registered count writes both
    /// of its channel words and zeroes them (one-shot) [M :110-123]. Returns the cut events; the
    /// caller resolves the hurt position (the game hurts at the dying wire's `obj+0x80` sphere
    /// [M :100]).
    pub fn update(&mut self, ch: &mut Channels, frame: u32) -> Vec<Cut> {
        let mut out = Vec::new();
        for g in 0..2 {
            let mut alive = 0u16;
            for k in 0..WIRE_SLOTS {
                let Some(obj) = self.groups[g].slots[k] else { continue };
                if !self.groups[g].hit[k] { alive += 1; continue; }
                // The wire died: remove its sub-objects (the port drops the entity), clear the slot.
                out.push(Cut::RemoveWire { obj });
                self.groups[g].slots[k] = None;
                if g == 1 {
                    if self.ch[1] == 0 {
                        // The hurt rides the DYING wire's world sphere [M 0x1adfe8:100].
                        out.push(Cut::LiveWireHurt { obj });
                    }
                    // A group-1 death zeroes group 0's second word (+0x5C) before it can fire
                    // [M 0x1adfe8:102].
                    self.ch[2] = 0;
                }
            }
            // The group emptied with wires registered: write both channel words with the
            // timestamp, then zero the ext words [M 0x1adfe8:110-123 — +0x54+2g then +0x5C+2g,
            // i.e. group g's words are ch[g] (main) and ch[g+2] (second)].
            if alive == 0 && self.groups[g].count != 0 {
                for c in [self.ch[g], self.ch[g + 2]] {
                    if c != 0 { ch.set(c as u32, 1, frame); }
                }
                self.ch[g] = 0;
                self.ch[g + 2] = 0;
            }
        }
        out
    }
}

/// The wire entity's world sphere radius [S — the game collides the bullet's HITTEST against the
/// wire's real geometry; unmeasured, 0.5 covers the fusebox's interior wires].
pub const WIRE_RADIUS: f32 = 0.5;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::triggers::Channels;

    fn box_with(ch: [u16; 4]) -> FuseBox {
        FuseBox { placement: 0, pos: [0.0; 3], ch, groups: Default::default() }
    }

    /// FuseBox_Register 0x1b0970: the first zero slot takes the object, count++.
    #[test]
    fn register_files_wires_into_their_groups() {
        let mut b = box_with([137, 0, 0, 0]);
        b.register_wire(7, 0);
        b.register_wire(9, 1);
        b.register_wire(7, 0); // no duplicate
        assert_eq!(b.alive_wires().collect::<Vec<_>>(), vec![(0, 0, 7), (1, 0, 9)], "one slot per group, no dup");
    }

    /// Collide_FilterBullets 0x1ece30 + the disc category words [L dump]: the Laser (0x0002)
    /// survives the 0xfffd filter and kills the wire; ordinary gunfire (0x0008) is dropped;
    /// the watch/Decryptor beams (0x0100) are dropped too.
    #[test]
    fn the_laser_cuts_and_gunfire_does_not() {
        assert!(FuseBox::hit_survives_filter(0x0002), "the Laser's category word survives 0xfffd [L weapon_data 0x4E]");
        assert!(!FuseBox::hit_survives_filter(0x0008), "gunfire's 0x0008 is dropped [L weapon_data 0x02/0x06]");
        assert!(!FuseBox::hit_survives_filter(0x0100), "the watch/Decryptor beams 0x0100 are dropped [L 0x54/0x56]");

        // One registered wire; the group-0 channel 137 fires when it dies.
        let mut b = box_with([137, 0, 0, 0]);
        b.register_wire(0, 0);
        // A kept-kind hit this tick: the wire dies, ch 137 fires, the removal is requested.
        b.mark_hit(0, 0);
        let mut ch = Channels::default();
        let cuts = b.update(&mut ch, 10);
        assert!(matches!(cuts.as_slice(), [Cut::RemoveWire { obj: 0 }]), "the wire's entity is removed: {cuts:?}");
        assert!(ch.on(137), "group 0 emptied with count 1: ch 137 (param 1) fires [M 0x1adfe8:110-115]");
        // One-shot: no hits, no wires left — nothing refires.
        let mut ch2 = Channels::default();
        let cuts2 = b.update(&mut ch2, 11);
        assert!(cuts2.is_empty() && !ch2.on(137), "the ext words were zeroed: the tail cannot refire [M 0x1adfe8:113-122]");
    }

    /// Sister of the laser cut: a dropped-kind hit (gunfire 0x0008) never reaches `mark_hit`,
    /// so the wire survives — the user's "you have to burn it with the laser".
    #[test]
    fn gunfire_never_kills_a_wire() {
        let mut b = box_with([137, 0, 0, 0]);
        b.register_wire(0, 0);
        assert!(!FuseBox::hit_survives_filter(0x0008), "the P2K's category word is dropped by the filter");
        // The bullet walk only calls mark_hit for surviving kinds — no mark, no death.
        let mut ch = Channels::default();
        let cuts = b.update(&mut ch, 10);
        assert!(cuts.is_empty(), "no kept hit → the wire survives [M 0x1adfe8:89-91 alive++]");
        assert!(!ch.on(137));
    }

    /// docs/fields/fusebox.md +0x56: the live-wire hurt fires on a group-1 death while +0x56
    /// (param 2) is UNSET [M 0x1adfe8:99-101], and the death clears +0x5C [M :102] so group 0's
    /// second word never fires.
    #[test]
    fn group1_death_hurts_only_when_param2_unset_and_cancels_group0_second_word() {
        // No param-2 channel: cutting the group-1 wire hurts 5.0 at the dying wire's position.
        let mut b = box_with([10, 0, 11, 12]);
        b.register_wire(0, 0);
        b.register_wire(1, 1);
        b.mark_hit(1, 0);
        let mut ch = Channels::default();
        ch.set(10, 1, 0); // group 0's first word already fired earlier (its wire is still alive here)
        let cuts = b.update(&mut ch, 1);
        assert!(cuts.iter().any(|c| matches!(c, Cut::LiveWireHurt { obj: 1 })),
            "group-1 death with +0x56 == 0 hurts at the dying wire (obj 1): {cuts:?}");
        assert!(!ch.on(11), "+0x5C is cleared by the group-1 death, never fires [M 0x1adfe8:102]");
        assert!(ch.on(12), "group 1's own second word (+0x5E) fires with the group empty");

        // Sister: with param 2 configured the same death never hurts, and the channels fire.
        let mut b = box_with([10, 5, 11, 12]);
        b.register_wire(1, 1);
        b.mark_hit(1, 0);
        let mut ch = Channels::default();
        let cuts = b.update(&mut ch, 1);
        assert!(!cuts.iter().any(|c| matches!(c, Cut::LiveWireHurt { .. })),
            "group-1 death with +0x56 configured never hurts [M 0x1adfe8:99-101]");
        assert!(ch.on(5) && ch.on(12), "group 1's words fire");
    }
}
