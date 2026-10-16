//! The Secret Unlocks codes — Codenames ▸ Edit ▸ Secret Unlocks (page 0x40000020,
//! `P_CNNAME_Handler` 0x213790 in cheat mode [M 0x213790:31-38 `cheat_mode = param_6 == 0x4000001d`]).
//!
//! The typed string is validated by `Menu_SpecialCodenameCheck` 0x200238 [M 0x213a98
//! C_KEYBOARD_Handler:67 — called with the text buffer 0x34e028 on the ✓ key (control id 0x3ea)].
//! The validator builds 52 stack entries of 0x38 bytes each [M 0x200238's memset run: 104 memsets,
//! 2 per entry; the strcmp loop walks stride 0x38 from sp+0x28 to sp+0xb60 = 52·0x38 and compares
//! the typed code against entry+0x00]. Entry layout [M 0x200238 + the store scan + the match arm
//! 0x201860-0x2018c8]:
//!
//! ```text
//! struct CodeEntry {          // 0x38 bytes, on the validator's stack
//!     char code[16];          // +0x00 — the strcmp target (NUL-padded, ≤ 8 chars [M 0x213a98:29-31])
//!     u8   pad[0x20];
//!     u32  level_flags;       // +0x20 — written to the 12 `sp_level[i]+0x10` flags
//!     u32  pad2;
//!     u64  cheat_mask;        // +0x28 — the `Menu_SetBonus` 0x1ff898 argument
//! }
//! ```
//!
//! On a match [M 0x201860-0x2018c8]:
//! 1. the 12 `sp_level[i]+0x10` flags are set from `level_flags` bit i (bool each; the loop at
//!    0x201890-0x2018ac writes `sp_level + 0x10 + i·0x18`, i = 0..11) — the port keeps them packed
//!    in `Progress::nightfire_status` (bit i = flag i, same packing `Menu_GetNightfireStatus`
//!    0x202360 uses);
//! 2. `Menu_SetBonus(cheat_mask, 0, 0)` runs — the disassembly pins both trailing args to 0
//!    [M 0x2018b4 `addu a1,zero,zero`, 0x2018b8 `addu a2,zero,zero`], and `Menu_SetBonus`
//!    0x1ff898's zero-overwrite arm ORs: `bonus[0] |= cheat_mask` [M 0x1ff898:79-84]. So a code
//!    only ever SETS its bits (the `^` shapes in the table are idempotent against the OR).
//!
//! The `cheat_mask` bits are reward ids in the `bonus` bitmask's id space: `bonus` 0x2e1b88 is
//! u64[21] (the port keeps `Progress::bonus`), the save carries slot 0 [M LS_MakeBonus 0x1ce3b0],
//! and `PlrStarts_ProcessRewardCounter` 0x199210 sets `1 << (desc.bit + rank)` for a reward's
//! descriptor — the same bits `Menu_UnlockMPSkins` 0x1ff2f8 switches on (ids 0x26..0x2f+ →
//! `mp_characters`) [M 0x1ff2f8:100-160].
//!
//! The 12 `sp_level` flag effects ([M] the +0x20 words from the store scan at 0x200770/0x2007c8/
//! …/0x200a9c/0x200af8, all stride 0x38): PASSPORT writes 0xffffffff (every flag 1), POWDER…
//! VACUUM write `current ^ bit` (bit 2..11 — toggle one flag), everything else writes the current
//! mask back (no change).
//!
//! The grant codes (AU PP7 … LAUNCH, Q LAB) run `Menu_UpgradeCheat(bonus, id, count)` 0x1fffe8 —
//! grant up to `count` still-unset reward bits among the descriptors whose RAW bit equals `id`
//! [M 0x1fffe8:84-152: two passes over the 12 entries × counters 1..4, `REWARDINFO+4` (the raw
//! desc bit) compared to `param_2`]. The widened bit (kind-4 groups) comes from
//! `PlrStarts_ProcessRewardCounter`'s group walk [M 0x199210:58-77: descriptors sharing the packed
//! {bit,kind} u32 across slots 1..4 of every entry; the set bit is `desc.bit + rank`]. The 12
//! gold-group ids are therefore 0,1,2 (group 0) / 3 / 6 / 9,10 (group 9) / 12 / 15 / 18 / 21 / 24,
//! and the grant args (0,1)(3,1)(6,1)(9,1)(0,2)(0xc,1)(0,3)(0xf,1)(0x12,1)(0x15,1)(9,2)(0x18,1)
//! cover all of them.
//!
//! Key ids on the entry grid [M C_KEYBOARD_Handler 0x213a98]: a character cell appends its char,
//! id 1000 (0x3e8) deletes the last char (:39-43), id 1001 (0x3e9) appends the space char
//! (:28-36 + :75-85 — the only other cell that reaches the append), id 0x3ea submits (:45-74).
//! Entry caps at 8 chars (:29-31/:76-78). Feedback [M :67-73]: a match opens an option box with
//! label 0x100029d and sets `cn_modified_flag`; no match returns without any feedback.

use crate::progress::{Progress, REWARDS_TABLE};

/// One code: the string as the game stores it (game data — the address is cited per entry),
/// its level-flag effect and its unlock bits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SecretCode {
    /// The stored string (what strcmp matches; uppercase, ≤ 8 chars).
    pub code: &'static str,
    /// The .rodata address the string lives at in ACTION.ELF (the citation for the game data).
    pub addr: u32,
    /// The word written to the 12 `sp_level[i]+0x10` flags (entry +0x20).
    pub level_flags: LevelFlags,
    /// The `bonus[0] |= mask` word (entry +0x28).
    pub bonus_or: u64,
    /// `Menu_UpgradeCheat(bonus, id, count)` grants, in call order.
    pub grants: &'static [(u32, u32)],
}

/// The `sp_level[i]+0x10` flag effect of one code.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LevelFlags {
    /// Entries 12+ store the current mask back — nothing changes.
    #[default]
    Keep,
    /// PASSPORT stores 0xffffffff — every flag set.
    SetAll,
    /// POWDER…VACUUM store `current ^ bit` — toggle one flag (bit 2..11).
    Toggle(u32),
}

/// The 52 codes in table order, with their stored-string addresses.
/// Every string/address pair below is [L] ACTION.ELF .rodata (read from `work/elf/ACTION.ELF` at
/// the cited address); the effects are [M] per the module doc's store scan of 0x200238.
pub static SECRET_CODES: [SecretCode; 52] = [
    code("PASSPORT", 0x308578, LevelFlags::SetAll, 0, &[]),
    code("POWDER",   0x308588, LevelFlags::Toggle(1 << 2), 0, &[]),
    code("TRACTION", 0x308590, LevelFlags::Toggle(1 << 3), 0, &[]),
    code("BONSAI",   0x3085a0, LevelFlags::Toggle(1 << 4), 0, &[]),
    code("HIGHRISE", 0x3085a8, LevelFlags::Toggle(1 << 5), 0, &[]),
    code("MELTDOWN", 0x3085b8, LevelFlags::Toggle(1 << 6), 0, &[]),
    code("FLAME",    0x3085c8, LevelFlags::Toggle(1 << 7), 0, &[]),
    code("AQUA",     0x3085d0, LevelFlags::Toggle(1 << 8), 0, &[]),
    code("PARADISE", 0x3085d8, LevelFlags::Toggle(1 << 9), 0, &[]),
    code("BLASTOFF", 0x3085e8, LevelFlags::Toggle(1 << 10), 0, &[]),
    code("VACUUM",   0x3085f8, LevelFlags::Toggle(1 << 11), 0, &[]),
    // PARTY/BOWLER share the OR word 0xffefc000000000 [M 0x200aec `or s0,fp,s0`; bits 38-55
    // except the unused 44]: every MP unlock bit but one.
    code("PARTY",    0x308600, LevelFlags::Keep, 0x00ff_efc0_0000_0000, &[]),
    code("BOWLER",   0x308608, LevelFlags::Keep, 1 << 38, &[]),
    code("BLACKTIE", 0x308610, LevelFlags::Keep, 1 << 39, &[]),
    code("MARTIAL",  0x308620, LevelFlags::Keep, 1 << 40, &[]),
    // BITESIZE and JOELWADE share bit 41 [M one `ori 0x8000; dsll 26; xor` triple at 0x200c38
    // feeds both +0x28 stores (0x200c4c rt=s0, 0x200c94 rt=s0)].
    code("BITESIZE", 0x308628, LevelFlags::Keep, 1 << 41, &[]),
    code("JOELWADE", 0x308638, LevelFlags::Keep, 1 << 41, &[]),
    code("ASSASSIN", 0x308648, LevelFlags::Keep, 1 << 42, &[]),
    code("DENTAL",   0x308658, LevelFlags::Keep, 1 << 43, &[]),
    // NUMBER 1 really jumps to bit 54 [M 0x200d90 `dsll32 v0,v0,7` → 2^(15+32+7)].
    code("NUMBER 1", 0x308660, LevelFlags::Keep, 1 << 54, &[]),
    code("VOODOO",   0x308670, LevelFlags::Keep, 1 << 45, &[]),
    code("JANUS",    0x308678, LevelFlags::Keep, 1 << 46, &[]),
    code("MIDAS",    0x308680, LevelFlags::Keep, 1 << 47, &[]),
    code("BADGIRL",  0x308688, LevelFlags::Keep, 1 << 48, &[]),
    code("SLICK",    0x308690, LevelFlags::Keep, 1 << 49, &[]),
    code("NUCLEAR",  0x308698, LevelFlags::Keep, 1 << 50, &[]),
    code("CIRCUS",   0x3086a0, LevelFlags::Keep, 1 << 51, &[]),
    code("HEADCASE", 0x3086a8, LevelFlags::Keep, 1 << 52, &[]),
    // BLIMP and HUGE EGO share bit 53 the same way [M one triple at 0x2010b0; 0x2010c4 and
    // 0x20110c both store rt=s0].
    code("BLIMP",    0x3086b8, LevelFlags::Keep, 1 << 53, &[]),
    code("HUGE EGO", 0x3086c0, LevelFlags::Keep, 1 << 53, &[]),
    code("ZERO G",   0x3086d0, LevelFlags::Keep, 1 << 55, &[]),
    // GAMEROOM: `or s2,fp,s2` with 0xfc00 dsll32 14 = bits 56-61 [M 0x2005f0/0x2005f4/0x2011b4].
    code("GAMEROOM", 0x3086d8, LevelFlags::Keep, 0x3f00_0000_0000_0000, &[]),
    code("TARGET",   0x3086e8, LevelFlags::Keep, 1 << 56, &[]),
    code("TRANSMIT", 0x3086f0, LevelFlags::Keep, 1 << 57, &[]),
    code("TEAMWORK", 0x308700, LevelFlags::Keep, 1 << 58, &[]),
    code("TNT",      0x308710, LevelFlags::Keep, 1 << 59, &[]),
    code("GUARDIAN", 0x308718, LevelFlags::Keep, 1 << 60, &[]),
    code("ORBIT",    0x308728, LevelFlags::Keep, 1 << 61, &[]),
    code("BOOM",     0x308730, LevelFlags::Keep, 1 << 62, &[]),
    // Q LAB = uVar5 = the OR of UpgradeCheat calls 2,3,6,7,8,9,10,11,12 [M 0x200238:595] — every
    // gold-group grant except the ones subsumed by (0,3) and (9,2).
    code("Q LAB",    0x308738, LevelFlags::Keep, 0,
        &[(3, 1), (6, 1), (12, 1), (0, 3), (15, 1), (18, 1), (21, 1), (9, 2), (24, 1)]),
    code("AU PP7",   0x308740, LevelFlags::Keep, 0, &[(0, 1)]),
    code("LIFTOFF",  0x308748, LevelFlags::Keep, 0, &[(3, 1)]),
    code("SHUTTER",  0x308750, LevelFlags::Keep, 0, &[(6, 1)]),
    code("SCOPE",    0x308758, LevelFlags::Keep, 0, &[(9, 1)]),
    code("P2000",    0x308760, LevelFlags::Keep, 0, &[(0, 2)]),
    code("SLEEPY",   0x308768, LevelFlags::Keep, 0, &[(12, 1)]),
    code("AU P2K",   0x308770, LevelFlags::Keep, 0, &[(0, 3)]),
    code("SESAME",   0x308778, LevelFlags::Keep, 0, &[(15, 1)]),
    code("ZAP",      0x308780, LevelFlags::Keep, 0, &[(18, 1)]),
    code("PHOTON",   0x308788, LevelFlags::Keep, 0, &[(21, 1)]),
    code("MAGAZINE", 0x308790, LevelFlags::Keep, 0, &[(9, 2)]),
    code("LAUNCH",   0x3087a0, LevelFlags::Keep, 0, &[(24, 1)]),
];

const fn code(code: &'static str, addr: u32, level_flags: LevelFlags, bonus_or: u64,
    grants: &'static [(u32, u32)]) -> SecretCode {
    SecretCode { code, addr, level_flags, bonus_or, grants }
}

/// `Menu_SpecialCodenameCheck` 0x200238's port: find the exact match (the game strcmps the typed
/// buffer [M 0x201860-0x201868], so the compare is case-sensitive and exact) and apply its entry.
/// Returns `true` on a match (the caller shows the 0x100029d box [M 0x213a98:71-72]); a miss
/// returns `false` and changes nothing [M 0x213a98:68-70].
pub fn submit(progress: &mut Progress, typed: &str) -> bool {
    let Some(entry) = SECRET_CODES.iter().find(|c| c.code == typed) else { return false };
    // 2. the 12 sp_level flags [M 0x201884-0x2018ac] — the port keeps them packed as
    // nightfire_status (bit i = flag i).
    match entry.level_flags {
        LevelFlags::Keep => {}
        LevelFlags::SetAll => progress.nightfire_status |= 0xfff,
        LevelFlags::Toggle(bit) => progress.nightfire_status ^= bit,
    }
    // 1. Menu_SetBonus(mask, 0, 0): the zero-overwrite arm ORs [M 0x1ff898:79-84].
    progress.bonus[0] |= entry.bonus_or;
    // 3. the Menu_UpgradeCheat grants, in call order [M 0x2005e8-0x2006d0's call list; the game
    // computes the u64s first and SetBonus ORs them last, but the OR makes the order immaterial].
    for &(id, count) in entry.grants {
        upgrade_cheat(progress, id, count);
    }
    true
}

/// `Menu_UpgradeCheat(bonus, id, count)` 0x1fffe8: grant up to `count` still-unset reward bits
/// among the descriptors whose RAW bit equals `id`. Two passes [M 0x1fffe8:84-152]: count the set
/// ones, then set unset ones until the count reaches `count`. The set bit is the widened kind-4
/// bit (`desc.bit + rank` in the packed-{bit,kind} group [M 0x199210:58-77]).
pub fn upgrade_cheat(progress: &mut Progress, id: u32, count: u32) {
    let mut have = 0u32;
    for (ei, e) in REWARDS_TABLE.iter().enumerate() {
        for c in 1..=4usize {
            if e.desc[c].bit as u32 == id && widened_is_set(progress, ei, c) { have += 1; }
        }
    }
    if have >= count { return; }
    for (ei, e) in REWARDS_TABLE.iter().enumerate() {
        for c in 1..=4usize {
            if have >= count { return; }
            if e.desc[c].bit as u32 == id && !widened_is_set(progress, ei, c) {
                progress.bonus[0] |= 1u64 << widened_bit(ei, c);
                have += 1;
            }
        }
    }
}

/// The kind-4 group walk [M 0x199210:58-77]: descriptors (slots 1..4 of every entry, table
/// order) sharing the packed {bit,kind} u32 word form the group; a descriptor's widened bit is
/// `bit + rank` (the matches strictly before it). Kinds 0..3 never share a word in the shipped
/// table, so only the kind-4 groups widen.
fn widened_bit(entry_idx: usize, slot: usize) -> u32 {
    let d = REWARDS_TABLE[entry_idx].desc[slot];
    let word = d.bit as u32 | ((d.kind as u32) << 16);
    let mut rank = 0u32;
    for (i, e) in REWARDS_TABLE.iter().enumerate() {
        if i > entry_idx { break; }
        let last = if i == entry_idx { slot } else { 5 };
        for c in 1..last {
            let w = e.desc[c].bit as u32 | ((e.desc[c].kind as u32) << 16);
            if w == word { rank += 1; }
        }
    }
    d.bit as u32 + rank
}

fn widened_is_set(progress: &Progress, entry_idx: usize, slot: usize) -> bool {
    progress.bonus[0] & (1u64 << widened_bit(entry_idx, slot)) != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_is_52_and_codes_are_unique_and_short() {
        assert_eq!(SECRET_CODES.len(), 52);
        for (i, c) in SECRET_CODES.iter().enumerate() {
            assert!(c.code.len() <= 8, "code {} too long", i);
            assert!(c.code.chars().all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == ' '),
                "code {} has a non-grid char", i);
        }
        for i in 0..SECRET_CODES.len() {
            for j in i + 1..SECRET_CODES.len() {
                assert_ne!(SECRET_CODES[i].code, SECRET_CODES[j].code, "dup {} {}", i, j);
            }
        }
    }

    #[test]
    fn valid_code_sets_its_unlock_and_invalid_sets_nothing() {
        // A fresh profile: bonus[0] = 0, nightfire_status = the boot word 0x3
        // [D ACTION.ELF .data 0x1d91e0: rows 0-1's +0x10 = 1].
        let mut p = Progress::default();
        // The miss path: nothing changes (the game returns without feedback [M 0x213a98:68-70]).
        assert!(!submit(&mut p, "NOTACODE"));
        assert_eq!(p.bonus[0], 0);
        assert_eq!(p.nightfire_status, 0x3);
        // BOWLER sets exactly bit 38.
        assert!(submit(&mut p, "BOWLER"));
        assert_eq!(p.bonus[0], 1 << 38);
        assert_eq!(p.nightfire_status, 0x3, "the boot flags ride untouched");
        // A second code ORs on top (Menu_SetBonus's OR arm [M 0x1ff898:79-84]).
        assert!(submit(&mut p, "GAMEROOM"));
        assert_eq!(p.bonus[0], (1 << 38) | 0x3f00_0000_0000_0000);
        // The compare is exact (the game strcmps [M 0x201860]): case and padding miss.
        let mut q = Progress::default();
        assert!(!submit(&mut q, "bowler"));
        assert!(!submit(&mut q, "BOWLER "));
        assert_eq!(q.bonus[0], 0);
    }

    #[test]
    fn passport_sets_all_twelve_level_flags() {
        let mut p = Progress::default();
        assert!(submit(&mut p, "PASSPORT"));
        assert_eq!(p.nightfire_status, 0xfff);
        // Idempotent (the game re-writes the same bools).
        assert!(submit(&mut p, "PASSPORT"));
        assert_eq!(p.nightfire_status, 0xfff);
    }

    #[test]
    fn mission_codes_toggle_one_level_flag_each() {
        let mut p = Progress::default();
        // POWDER toggles flag 2 on top of the boot word [M the ^4 store at 0x2007c8; the boot
        // rows 0-1 stand, D .data 0x1d91e0].
        assert!(submit(&mut p, "POWDER"));
        assert_eq!(p.nightfire_status, 0x3 | (1 << 2));
        assert!(submit(&mut p, "POWDER"));
        assert_eq!(p.nightfire_status, 0x3);
        // VACUUM toggles flag 11 [M the ^0x800 store at 0x200a9c] and leaves the others.
        assert!(submit(&mut p, "POWDER"));
        assert!(submit(&mut p, "VACUUM"));
        assert_eq!(p.nightfire_status, 0x3 | (1 << 2) | (1 << 11));
    }

    #[test]
    fn grant_codes_fill_the_gold_groups_in_table_order() {
        let mut p = Progress::default();
        // AU PP7 = UpgradeCheat(id 0, 1): the first unset bit of the bit-0 kind-4 group —
        // entry 07000005's (rank 0) [M 0x1fffe8 + 0x199210's group walk].
        assert!(submit(&mut p, "AU PP7"));
        assert_eq!(p.bonus[0], 1 << 0);
        // P2000 = (0, 2): one more of the same group → bit 1.
        assert!(submit(&mut p, "P2000"));
        assert_eq!(p.bonus[0], (1 << 0) | (1 << 1));
        // AU P2K = (0, 3): the last group member → bit 2; then the group is full and a repeat
        // grants nothing.
        assert!(submit(&mut p, "AU P2K"));
        assert_eq!(p.bonus[0], (1 << 0) | (1 << 1) | (1 << 2));
        let before = p.bonus[0];
        assert!(submit(&mut p, "AU P2K"));
        assert_eq!(p.bonus[0], before);
        // SCOPE = (9, 1): the bit-9 group's first member.
        assert!(submit(&mut p, "SCOPE"));
        assert_eq!(p.bonus[0] & 0xffff, (1 << 0) | (1 << 1) | (1 << 2) | (1 << 9));
        // MAGAZINE = (9, 2): the second member (bit 10) — one new bit.
        assert!(submit(&mut p, "MAGAZINE"));
        assert_eq!(p.bonus[0] & 0xffff, (1 << 0) | (1 << 1) | (1 << 2) | (1 << 9) | (1 << 10));
        // SESAME = (15, 1): the single-member bit-15 group.
        assert!(submit(&mut p, "SESAME"));
        assert_eq!(p.bonus[0] & 0xffff, (1 << 0) | (1 << 1) | (1 << 2) | (1 << 9) | (1 << 10) | (1 << 15));
    }

    #[test]
    fn q_lab_grants_every_gold_group_bit() {
        let mut p = Progress::default();
        assert!(submit(&mut p, "Q LAB"));
        // Groups: 0 → 0,1,2; 9 → 9,10; singles 3,6,12,15,18,21,24.
        let want: u64 = (1 << 0) | (1 << 1) | (1 << 2) | (1 << 3) | (1 << 6) | (1 << 9) | (1 << 10)
            | (1 << 12) | (1 << 15) | (1 << 18) | (1 << 21) | (1 << 24);
        assert_eq!(p.bonus[0], want);
        // And it is idempotent (already-set bits are not re-granted — `have >= count`).
        assert!(submit(&mut p, "Q LAB"));
        assert_eq!(p.bonus[0], want);
    }

    #[test]
    fn party_covers_every_mp_bit_except_44() {
        let mut p = Progress::default();
        assert!(submit(&mut p, "PARTY"));
        let mut want = 0u64;
        for b in 38..=55 {
            if b != 44 { want |= 1 << b; }
        }
        assert_eq!(p.bonus[0], want);
    }

    #[test]
    fn medal_bits_survive_a_code_entry() {
        // A medal earned first (SetLevelBonus's path), then a code: the OR keeps it [M 0x1ff898].
        let mut p = Progress::default();
        p.set_level_bonus(0x0700_0005, 1, 0); // the bronze counter's bit (26)
        let medal_bits = p.bonus[0];
        assert_ne!(medal_bits, 0);
        assert!(submit(&mut p, "BOWLER"));
        assert_eq!(p.bonus[0], medal_bits | (1 << 38));
    }
}
