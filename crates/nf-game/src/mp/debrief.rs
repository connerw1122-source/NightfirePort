//! The debriefing data builder — `P_MPDEBRIEFING_Handler` 0x212198 msg 0x4c's rows, ranks and
//! result line, as pure data the draw code can bind (mp-debriefing-data2, ported from 621's
//! mp-debriefing-data; `docs/systems/mp-debriefing.md` §4). [M] line numbers are that decomp
//! file's. The builder was verified line-by-line against
//! `work/decomp/ACTION.ELF/00212198_P_MPDEBRIEFING_Handler__FUcP9M_CONTROLUiUiii.c` in the 621
//! reconciliation (the draw compare is pair0's score vs pair1's score — decomp `aiStack_118[2]`
//! vs `(int)auStack_108[0]` are two halves of adjacent 8-byte pairs).

use super::{scenario, MpMatch};

/// The rank captions [L 0x309520 = 0x01000297/98/99/9a "1st"-"4th"; disasm 0x2121d0-0x2121f0
/// copies exactly these 16 bytes]. Rows beyond 4 never render (the row loop caps at 4 —
/// decomp line 219 `while (iVar2 < 4)`, disasm `slti $v0,$s1,4` @0x212904).
pub const RANK_LABELS: [u32; 4] = [0x0100_0297, 0x0100_0298, 0x0100_0299, 0x0100_029a];

/// The result-line labels [L USATxt.dat]: the winner template and the draw text.
pub const RESULT_WINS_LABEL: u32 = 0x0100_029f; // "%s wins the match."
pub const RESULT_DRAW_LABEL: u32 = 0x0100_02a0; // "Match was a draw"
/// The team names the team announce binds [M 0x212198:237-247 — 0x1c8 when team0 < team1,
/// 0x1c7 when team0 >= team1; the [S] inversion concern is in the doc].
pub const TEAM_PHOENIX_LABEL: u32 = 0x1c7;
pub const TEAM_MI6_LABEL: u32 = 0x1c8;

/// One row's name cell: a human's codename (`MPSettings+s·0x30+0x00`) or a bot's short name
/// (`Menu_GetBotShortName(character)` 0x201918 — label 0x10002e2+character, the literal "Bot"
/// @0x30d290 for character >= 0x1d) [M 0x212198:167-177].
#[derive(Clone, Debug, PartialEq)]
pub enum MpDebriefName {
    /// The codename string.
    Human(String),
    /// The bot's character id — `bot_short_name_label` resolves the caption label.
    Bot(u32),
}

/// One rendered row of the debriefing (the game renders at most 4).
#[derive(Clone, Debug, PartialEq)]
pub struct MpDebriefRow {
    /// The sorted slot's game slot index (the values all read that slot's records).
    pub slot: u8,
    /// The rank index 0..3 (ties share the rank of the previous row [M 0x212198:131-143]) —
    /// the caption is `RANK_LABELS[rank]`.
    pub rank: u8,
    /// The name cell.
    pub name: MpDebriefName,
    /// `MPSettings+s·0x30+0x24` — the skin/character id the portrait looks up [M :159-166].
    pub character: u32,
    /// `"%d"` of `(int)(float)MPGame+s·0x30+0x18` [M :178-185].
    pub points: i32,
    /// `"%d"` of `MPGame+s·0x30+0x04` (the kills) [M :186-192].
    pub victories: i32,
    /// `"%d"` of `MPGame+s·0x30+0x08` (the deaths) [M :193-198].
    pub deaths: i32,
    /// `"%d"` of `Menu_GetMPScore(s)` [M :199-204].
    pub total: i32,
}

/// The result line under the plate [M 0x212198:220-250].
#[derive(Clone, Debug, PartialEq)]
pub enum MpDebriefResult {
    /// The bound 0x010002a0 "Match was a draw".
    Draw,
    /// `sprintf(mp_result.451, "%s wins the match." 0x0100029f, codename)` — non-team games
    /// [M :222-229; disasm 0x212a64-0x212a7c: the arg is the record base +0x00].
    Wins(String),
    /// The team announce: the label id (0x1c7 Phoenix / 0x1c8 MI6) fed to the same template
    /// [M :233-249].
    Team(u32),
}

/// The whole debriefing board.
#[derive(Clone, Debug, PartialEq)]
pub struct MpDebriefing {
    /// The rows in display order (up to 4; rows the game hides beyond the player count are
    /// not carried).
    pub rows: Vec<MpDebriefRow>,
    /// The result line.
    pub result: MpDebriefResult,
}

impl MpMatch {
    /// `Menu_GetMPScore` 0x203af8 — the TOTAL column and the ranking key: kills for Arena and
    /// Team Arena, the slot score for the objective modes, 0 for anything else [M 0x203af8].
    pub fn get_mp_score(&self, slot: usize) -> i32 {
        match self.settings.scenario {
            scenario::DEATHMATCH | scenario::TEAM_DEATHMATCH => self.game.slots[slot].kills,
            scenario::DEATH_LIMIT_FFA
            | scenario::ASSASSIN
            | scenario::CAPTURE_THE_FLAG
            | scenario::DEMOLITION
            | scenario::PROTECTION
            | scenario::BLUEPRINT
            | scenario::GOLDENEYE
            | scenario::KOTH_FFA
            | scenario::UPLINK
            | scenario::KOTH_TEAM => self.game.slots[slot].score as i32,
            _ => 0,
        }
    }

    /// The debriefing board [M P_MPDEBRIEFING_Handler 0x212198, msg 0x4c, decomp lines 83-250].
    pub fn debriefing(&self) -> MpDebriefing {
        // 1. The (slot, score) pairs: humans 0..+0x1AC then bots 4..4+0x1B0 [M decomp :93-111].
        let humans = (self.settings.human_count as usize).min(4);
        let bots = (self.settings.bot_count as usize).min(4);
        let mut slots: Vec<usize> = (0..humans).collect();
        slots.extend(4..4 + bots);
        let mut scores: Vec<i32> = slots.iter().map(|&s| self.get_mp_score(s)).collect();
        // 2. The stable descending bubble sort over `MPSettings+0x194` entries [M decomp :112-130].
        //    (The game sorts its whole pair array; the port's array is exactly humans+bots,
        //    which is what +0x194 counts [M MP_Start 0x183a50:164].)
        let n = scores.len();
        loop {
            let mut swapped = false;
            for i in 1..n {
                if scores[i - 1] < scores[i] {
                    scores.swap(i - 1, i);
                    slots.swap(i - 1, i);
                    swapped = true;
                }
            }
            if !swapped {
                break;
            }
        }
        // 3. The rank walk — the rank advances only when the score differs from the previous
        //    row's [M decomp :131-143].
        let mut rows = Vec::new();
        let mut rank = 0usize;
        for (i, &s) in slots.iter().enumerate() {
            if i > 0 && scores[i] != scores[i - 1] {
                rank = i;
            }
            let name = if s < 4 {
                MpDebriefName::Human(self.settings.players[s].codename.clone())
            } else {
                MpDebriefName::Bot(self.settings.players[s].skin)
            };
            rows.push(MpDebriefRow {
                slot: s as u8,
                rank: rank as u8,
                name,
                character: self.settings.players[s].skin,
                points: self.game.slots[s].score as i32,
                victories: self.game.slots[s].kills,
                deaths: self.game.slots[s].deaths,
                total: scores[i],
            });
            if rows.len() == 4 {
                break; // the row loop caps at 4 [decomp line 219; disasm 0x212904]
            }
        }
        // 4. The result line [M decomp :220-250].
        let result = if self.settings.scenario & 0x2000_0000 == 0 {
            // Non-team: the top two TOTALs tie → the draw text; else the winner's codename
            // [M :222-229]. With a single entry the game compares uninitialized stack — the
            // port treats it as a win (*Deviation*).
            if n >= 2 && scores[0] == scores[1] {
                MpDebriefResult::Draw
            } else if let Some(&s) = slots.first() {
                MpDebriefResult::Wins(self.settings.players[s].codename.clone())
            } else {
                MpDebriefResult::Draw
            }
        } else {
            // Team: (float)MPGame+0x180 < +0x184 → 0x1c8 "MI6", >= → 0x1c7 "Phoenix", equal →
            // the draw [M :233-249; the compares are FLOATS in the decomp, lines 233/237]. The
            // [S] inversion note is in the doc.
            let (a, b) = (self.game.team_scores[0], self.game.team_scores[1]);
            if a == b {
                MpDebriefResult::Draw
            } else if a < b {
                MpDebriefResult::Team(TEAM_MI6_LABEL)
            } else {
                MpDebriefResult::Team(TEAM_PHOENIX_LABEL)
            }
        };
        MpDebriefing { rows, result }
    }

    /// `Menu_GetBotShortName` 0x201918: character 0..0x1c → label 0x010002e2+character
    /// [M :10-94]; >= 0x1d → the literal "Bot" @0x30d290 (None here — the caller binds its
    /// own text) [M :97].
    pub fn bot_short_name_label(character: u32) -> Option<u32> {
        (character < 0x1d).then(|| 0x0100_02e2 + character)
    }
}
