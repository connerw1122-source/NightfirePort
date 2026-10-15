# Secret Unlocks — the codes and what they unlock

Codenames → Edit \<name\> → Secret Unlocks opens the letter grid (0–9, A–Z, ← → ✓). The page is the
game's codename-entry page **0x40000020** running in cheat mode: `P_CNNAME_Handler` 0x213790 sets
`cheat_mode` when the enter param is the codename wheel 0x4000001d [M 0x213790:31-38
`cheat_mode = param_6 == 0x4000001d`] and binds the cheat prompt label 0x10002d4 instead of the
codename text [M :35-42]. Everything below is ACTION.ELF.

## Input flow

- The grid cells are the standard keyboard control (msg 0x10000074 → `C_KEYBOARD_Handler`
  0x213a98). A character cell appends its char; **id 1000 (0x3e8) deletes the last char**
  [M :39-43]; **id 1001 (0x3e9) appends the space** [M :28-36 + :75-85 — the only other cell that
  reaches the append, which is how the stored "HUGE EGO"/"NUMBER 1"/"ZERO G"/"Q LAB" strings are
  reachable]; **id 0x3ea submits** [M :45-74]. The typed string lives in the buffer 0x34e028 and is
  capped at 8 chars [M :29-31/:76-78].
- On ✓ in cheat mode: `Menu_SpecialCodenameCheck(0x34e028)` [M :67]. A match opens an option box
  with **label 0x100029d** (the valid-code feedback) and sets `cn_modified_flag` [M :71-72]; a miss
  **returns with no feedback at all** [M :68-70].
- `C_KEYPAD_Handler` 0x216810 (msg 0x100000ba) is the *in-game* keypad widget: it appends to the
  shared key buffer `DAT_002a37dd` the door/lock codes read (`Lock_Update` 0x1ae3d8). The Secret
  Unlocks page does not use it — its validation path is the keyboard handler above.

## The validator

`Menu_SpecialCodenameCheck(char* typed)` 0x200238 builds 52 stack entries of 0x38 bytes each (104
memsets, 2 per entry) and strcmps the typed string against each entry's code [M the loop at
0x201860-0x201868, stride 0x38 from sp+0x28 to sp+0xb60 = 52·0x38]. Entry layout [M the store scan
+ the match arm 0x201860-0x2018c8]:

```text
struct CodeEntry {          // 0x38 bytes on the validator's stack
    char code[16];          // +0x00 — the strcmp target (NUL-padded)
    u8   pad[0x20];
    u32  level_flags;       // +0x20 — written to the 12 sp_level[i]+0x10 flags
    u32  pad2;
    u64  cheat_mask;        // +0x28 — the Menu_SetBonus argument
}
```

On a match [M 0x201860-0x2018c8]:

1. the 12 `sp_level[i]+0x10` flags are written from `level_flags` bit i (one bool each, loop at
   0x201890-0x2018ac into `sp_level + 0x10 + i·0x18`) — the same flags `Menu_GetNightfireStatus`
   0x202360 packs, i.e. the port's `Progress::nightfire_status` bits;
2. `Menu_SetBonus(cheat_mask, 0, 0)` runs — the disassembly pins both trailing args to 0
   [M 0x2018b4/0x2018b8 `addu aN,zero,zero`], and `Menu_SetBonus` 0x1ff898's zero-overwrite arm
   ORs: **`bonus[0] |= cheat_mask`** [M 0x1ff898:79-84]. Codes only ever SET bits; the `^` shapes
   in the table are idempotent under the OR.
3. no reward counters are touched directly — the grant codes carry `Menu_UpgradeCheat` calls (below).

## Where the result is saved

`bonus` 0x2e1b88 is u64[21] (the port's `Progress::bonus`); the save carries slot 0 as 32+32 bits
[M `LS_MakeBonus` 0x1ce3b0:26-30 `BIN_PushBits(GetBonus(0) >> 32, 0x20)` + low half], restored by
the load path. The 12 `sp_level[i]+0x10` flags ride the MSSN block's nightfire status
(`Menu_SetNightfireStatus` 0x2023a0). The codes' effect therefore persists in the profile.

## What the bits mean (the unlock side)

The `bonus` bitmask's bit index is a **reward id**: `PlrStarts_ProcessRewardCounter` 0x199210 sets
`1 << (desc.bit + rank)` for a reward descriptor, and `Menu_UnlockMPSkins` 0x1ff2f8 switches on
those ids (0x26..0x2f+ → `mp_characters` entries) [M 0x1ff2f8:100-160]; `Menu_UnlockMPSettings`
0x1ff698 reads the same mask. `Menu_UpgradeCheat` 0x1fffe8 grants `count` still-unset bits among
the descriptors whose RAW bit equals `id` [M 0x1fffe8:84-152 — two passes over the 12 entries ×
counters 1..4, REWARDINFO+4 compared to param_2]. The kind-4 groups widen their bit by rank within
the group of descriptors sharing the packed {bit,kind} u32 across slots 1..4 [M 0x199210:58-77],
which makes the 12 gold-group reward ids: 0,1,2 (group 0) / 3 / 6 / 9,10 (group 9) / 12 / 15 / 18 /
21 / 24. The grant args used by the codes — (0,1) (3,1) (6,1) (9,1) (0,2) (0xc,1) (0,3) (0xf,1)
(0x12,1) (0x15,1) (9,2) (0x18,1) — cover exactly those 12. Per-reward player-facing names (which
cheat each label names, e.g. the group-0 params 0x1002d/0x10032/0x10036) need the .txt label bind —
not read yet (see the live checks).

## The code table

All strings are game data read from ACTION.ELF .rodata (the `addr` column; verified against the
disassembly of 0x200238, whose per-entry constants Ghidra partially mis-folded — the disassembly is
authoritative for every bit below). Bits not listed under a code: 25 and 44 are referenced by no
code in the shipped table.

| # | stored string | addr | level_flags (+0x20) | bonus bits (ORed, +0x28) | grants (UpgradeCheat id,count) |
|---|---------------|------|---------------------|--------------------------|-------------------------------|
| 0 | PASSPORT | 0x308578 | 0xffffffff — all 12 flags set | — | — |
| 1 | POWDER | 0x308588 | current ^ bit2 | — | — |
| 2 | TRACTION | 0x308590 | current ^ bit3 | — | — |
| 3 | BONSAI | 0x3085a0 | current ^ bit4 | — | — |
| 4 | HIGHRISE | 0x3085a8 | current ^ bit5 | — | — |
| 5 | MELTDOWN | 0x3085b8 | current ^ bit6 | — | — |
| 6 | FLAME | 0x3085c8 | current ^ bit7 | — | — |
| 7 | AQUA | 0x3085d0 | current ^ bit8 | — | — |
| 8 | PARADISE | 0x3085d8 | current ^ bit9 | — | — |
| 9 | BLASTOFF | 0x3085e8 | current ^ bit10 | — | — |
| 10 | VACUUM | 0x3085f8 | current ^ bit11 | — | — |
| 11 | PARTY | 0x308600 | keep | bits 38-55 except 44 (`0x00ff_efc0_0000_0000`) — every MP unlock but one [M 0x200aec] | — |
| 12 | BOWLER | 0x308608 | keep | 38 | — |
| 13 | BLACKTIE | 0x308610 | keep | 39 | — |
| 14 | MARTIAL | 0x308620 | keep | 40 | — |
| 15 | BITESIZE | 0x308628 | keep | 41 | — |
| 16 | JOELWADE | 0x308638 | keep | 41 (shares BITESIZE's — one `ori/dsll/xor` triple at 0x200c38 feeds both +0x28 stores) | — |
| 17 | ASSASSIN | 0x308648 | keep | 42 | — |
| 18 | DENTAL | 0x308658 | keep | 43 | — |
| 19 | NUMBER 1 | 0x308660 | keep | 54 (`dsll32 v0,v0,7` at 0x200d90) | — |
| 20 | VOODOO | 0x308670 | keep | 45 | — |
| 21 | JANUS | 0x308678 | keep | 46 | — |
| 22 | MIDAS | 0x308680 | keep | 47 | — |
| 23 | BADGIRL | 0x308688 | keep | 48 | — |
| 24 | SLICK | 0x308690 | keep | 49 | — |
| 25 | NUCLEAR | 0x308698 | keep | 50 | — |
| 26 | CIRCUS | 0x3086a0 | keep | 51 | — |
| 27 | HEADCASE | 0x3086a8 | keep | 52 | — |
| 28 | BLIMP | 0x3086b8 | keep | 53 | — |
| 29 | HUGE EGO | 0x3086c0 | keep | 53 (shares BLIMP's triple at 0x2010b0) | — |
| 30 | ZERO G | 0x3086d0 | keep | 55 | — |
| 31 | GAMEROOM | 0x3086d8 | keep | 56-61 (`0x3f00_0000_0000_0000`) | — |
| 32 | TARGET | 0x3086e8 | keep | 56 | — |
| 33 | TRANSMIT | 0x3086f0 | keep | 57 | — |
| 34 | TEAMWORK | 0x308700 | keep | 58 | — |
| 35 | TNT | 0x308710 | keep | 59 | — |
| 36 | GUARDIAN | 0x308718 | keep | 60 | — |
| 37 | ORBIT | 0x308728 | keep | 61 | — |
| 38 | BOOM | 0x308730 | keep | 62 | — |
| 39 | Q LAB | 0x308738 | keep | — | (3,1)(6,1)(12,1)(0,3)(15,1)(18,1)(21,1)(9,2)(24,1) — the OR of UpgradeCheat calls 2,3,6,7,8,9,10,11,12 [M 0x200238:595] = every gold-group reward |
| 40 | AU PP7 | 0x308740 | keep | — | (0,1) |
| 41 | LIFTOFF | 0x308748 | keep | — | (3,1) |
| 42 | SHUTTER | 0x308750 | keep | — | (6,1) |
| 43 | SCOPE | 0x308758 | keep | — | (9,1) |
| 44 | P2000 | 0x308760 | keep | — | (0,2) |
| 45 | SLEEPY | 0x308768 | keep | — | (12,1) |
| 46 | AU P2K | 0x308770 | keep | — | (0,3) |
| 47 | SESAME | 0x308778 | keep | — | (15,1) |
| 48 | ZAP | 0x308780 | keep | — | (18,1) |
| 49 | PHOTON | 0x308788 | keep | — | (21,1) |
| 50 | MAGAZINE | 0x308790 | keep | — | (9,2) |
| 51 | LAUNCH | 0x3087a0 | keep | — | (24,1) |

Reading of the columns: PASSPORT marks every level's `sp_level` flag (unlock the whole campaign
list); POWDER…VACUUM toggle one level flag each (flag i = the i-th `SP_LEVEL_HASHES` row); PARTY +
the single-bit codes set MP-unlock reward bits; the grant codes set gold-group reward bits (the
cheats the bonus page names). Which cheat/character each bit names in the UI is the label side —
see the open item below.

## The port

- `nf_game::secret_codes` (`crates/nf-game/src/secret_codes.rs`): the 52-entry `SECRET_CODES` table
  (each entry cites its .rodata address), `submit(&mut Progress, &str) -> bool` (the validator's
  port — exact compare, level flags, the OR, the grants in call order), and `upgrade_cheat(id,
  count)` (`Menu_UpgradeCheat`'s two-pass grant with the kind-4 group walk). The widened bit is
  computed from the descriptors' own packed {bit,kind} word per 0x199210's group scan — note this
  differs from `progress::gold_group`, which builds the word from descriptor 0 (see the report's
  open items).
- `nf_view::menu` (`crates/nf-view/src/menu.rs`): `Page::CnName` (id 0x40000020), the
  `SecretEntry` model (`cursor`/`typed`/`accepted`/`pending_submit`), `SECRET_GRID` (39 cells:
  0-9, A-Z, ← Delete, → Space, ✓ Submit — the drawn layout is the drawing task's), `secret_press`
  (append with the 8-char cap / delete / space / flag submit) and `secret_submit(&mut Progress)`
  (the host drains it — the same split as `results_enter`). The codename wheel's row 0 routes to
  the page [M C_SBCNOPTIONS 0x213528 case 0]; the overlay draws a stub grid until the drawing task
  lands.

## Tests

`crates/nf-game/src/secret_codes.rs` (`#[cfg(test)]`, 8): the table's shape/uniqueness, a valid
code sets its unlock + an invalid one sets nothing (sister assertions in one test), PASSPORT's 12
flags, the toggle codes, the grant codes' fill order, Q LAB's full coverage, PARTY's bit range,
medal bits surviving a code. `crates/nf-view/src/menu.rs` (`secret_grid_tests`, 3): typing BOWLER
through the grid cells and submitting, the 8-char cap + delete, and the wheel row-0 routing.

## Open items

- The per-bit player-facing names (labels 0x10002d4, 0x100029d and the reward params
  0x1002d/0x10032/0x10036/…) live in the disc's .txt data — not read; the port draws placeholder
  text for the accepted box.
- The kind-4 group walk in `secret_codes::widened_bit` (descriptors' own {bit,kind} word, slots
  1..4) vs `progress::gold_group` (descriptor 0's word): both cannot be right; the codes' grant
  path needs the former (the UpgradeCheat ids are the kind-4 raw bits 0/3/6/9/…). Flagged for the
  live checks; do not change either without re-deriving 0x199210.
- `Menu_SetBonus` also recomputes the weapon upgrades (`ResetUpgrade` 0x1ff898's tail +
  `Set_Upgrade`): the port does not model the upgrade arrays — the codes set the bits only.
