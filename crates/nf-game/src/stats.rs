//! The Mission Stats page's data model (`P_NFSTATS_Handler` 0x209938): the rows the page shows,
//! as display strings + ids. Docs: `docs/systems/mission-stats.md`; the row table and every
//! format below cite the decompile. No drawing — the view binds `id` through the language bank
//! and prints `current`/`target`/`total` as-is.
//!
//! The page enter runs `PlrStat_GetScore(0)` — the commit [M 0x209938:36-38, the commit block
//! 0x1984b0:301-315] — then reads the entry. A missing entry draws nothing [M :38-40], which is
//! this module's `None`.

use crate::progress::Progress;

/// `SeparateNumber__FUiPc` 0x1dec78: comma grouping — `<1e3` plain, `<1e6` one group, else two
/// (`%d,%.3d,%.3d`, the millions group unpadded) [M the format strings 0x30d090/0x30d098].
pub fn separate_number(v: u32) -> String {
    let ones = v % 1000;
    let thousands = (v / 1000) % 1000;
    let millions = (v / 1000) / 1000;
    if millions == 0 {
        if thousands == 0 {
            format!("{}", ones) // `%d` of v % 1000 [M 0x1dec78:12-14]
        } else {
            format!("{},{:03}", thousands, ones) // `%d,%.3d` — %.3d zero-pads [M :15-17]
        }
    } else {
        format!("{},{:03},{:03}", millions, thousands, ones) // `%d,%.3d,%.3d` [M :18+]
    }
}

/// `Timer_Seconds2String__FiUi` 0x1deaa8, mode 0: seconds = `fptoui(cs × 0.01 + 0.5)` (round
/// half up [M :17]); hours = (s/3600)%60; hours ≠ 0 → `hh:mm:ss` [S — the branch's sprintf args
/// are dropped in the decompile; the case-2 `%.2d:%.2d` mm:ss is exact [M :21-24]]; negative →
/// zeros [M :19-23].
pub fn timer_string(cs: i64) -> String {
    if cs < 0 {
        return "00:00".into();
    }
    let s = (cs as f32 * 0.01 + 0.5) as i64; // fptoui(param_1 * 0.01f + 0.5f) [M 0x1deaa8:17]
    let h = (s / 3600) % 60;
    let m = (s / 60) % 60;
    let sec = s % 60;
    if h != 0 {
        format!("{:02}:{:02}:{:02}", h, m, sec) // [S the hour branch]
    } else {
        format!("{:02}:{:02}", m, sec) // case 2 [M]
    }
}

/// The pct cell: `fptoui(ratio × 100 + 0.5)` — round half up [M 0x209938:92/100/106].
fn pct(ratio: f32) -> String {
    format!("{}%", (ratio * 100.0 + 0.5) as u32)
}

/// One Mission Stats row: the Txt label id the view binds, the three display cells exactly as
/// the game formats them, and the indent flag (`Menu_AddRow`'s param_6: two leading spaces on
/// the three Opponents sub-rows [M 0x201c10:107-112]).
#[derive(Clone, Debug, PartialEq)]
pub struct StatsRow {
    /// The row's label id [M 0x209938's `Menu_AddRow`/`Txt_BindLabel` pairs]. 0x15a is the
    /// label-only "Opponents" group row (msg 0x10, no cells [M 0x209938:66-68]); 0 marks the
    /// blank spacer row (the static string 0x30d2d8 [M :117 — the port's pause SCORE tab reads
    /// it as a blank row too, `menu.rs` pause arm 3]).
    pub id: u32,
    /// The bound label text (`lang.bind(id)`, or `<0x…>` when no bank — the port's bind
    /// convention, `menu.rs results_enter`).
    pub category: String,
    /// The Current cell.
    pub current: String,
    /// The Target cell (`----` on the stealth row [M 0x201df0's −1 arm]).
    pub target: String,
    /// The TOTAL cell: `SeparateNumber(fptoui(raw))` [M 0x201c10:99-101].
    pub total: String,
    /// `Menu_AddRow` param_6: the two-space indent (rows 0x15b/0x15c/0x16e [M 0x209938]).
    pub indented: bool,
}

/// `P_NFSTATS_Handler` 0x209938's rows for `base_map`'s entry. Runs the game's page-enter
/// `PlrStat_GetScore(0)` (the commit) first, then formats. `ms` is the mission status the game
/// reads (`Mission_Status()` — 6 on the results path; anything else zeroes the runtime fields,
/// 0x1984b0:316-333). Returns `None` when the hash is not a `ScoringTable` entry (the game's
/// `score_table == 0` arm [M :38-40]). Row order, gating and formats per
/// `docs/systems/mission-stats.md`'s table.
pub fn stats_rows(
    progress: &mut Progress,
    base_map: u32,
    ms: u32,
    difficulty: u32,
    lang: Option<&nf_formats::hud::Lang>,
) -> Option<Vec<StatsRow>> {
    progress.get_score(base_map, ms, difficulty, true)?; // [M 0x209938:36 — GetScore(0)]
    let idx = progress.table.iter().position(|e| e.hash == base_map)?;
    let e = progress.table[idx];
    let bind = |id: u32| -> String {
        lang.and_then(|l| l.bind(id)).map(str::to_string).unwrap_or_else(|| format!("<{id:#x}>"))
    };
    let row = |id: u32, cur: String, tgt: String, raw: f32, indented: bool| StatsRow {
        id,
        category: bind(id),
        current: cur,
        target: tgt,
        total: separate_number(raw as u32), // fptoui(raw) [M 0x201c10:99-101]
        indented,
    };
    let mut out = Vec::with_capacity(14);
    // The stealth trio (subdued 0x15c, surrendered 0x16e, stealth 0x15d) shows only when the
    // entry's +0x38 flag != 0 [M :68 — the flag is `new_scores`] — iVar7's 3 extra rows shift
    // the tail rows' indices only.
    let stealth = e.new_scores != 0;
    out.push(row(0x165, format!("{}", e.rows[0].value), format!("{}", e.rows[0].count), e.rows[0].raw, false));
    // The label-only "Opponents" group row (msg 0x10, no value cells) [M :66-68].
    out.push(StatsRow { id: 0x15a, category: bind(0x15a), current: String::new(), target: String::new(), total: String::new(), indented: false });
    out.push(row(0x15b, format!("{}", e.rows[1].value), format!("{}", e.rows[1].count), e.rows[1].raw, true));
    if stealth {
        out.push(row(0x15c, format!("{}", e.rows[2].value), format!("{}", e.rows[2].count), e.rows[2].raw, true));
        out.push(row(0x16e, format!("{}", e.rows[3].value), format!("{}", e.rows[3].count), e.rows[3].raw, true));
        // Stealth: the current is the pct of the ratio; the target is −1 → `----` [M :87-92].
        out.push(row(0x15d, pct(e.rows[4].ratio), "----".into(), e.rows[4].raw, false));
    }
    // Accuracy: the target is the par itself formatted with %d%% (Menu_AddRowPercentage does not
    // scale its target [M 0x201df0:78-88]; the par 100 prints "100%").
    out.push(row(0x15e, pct(e.rows[5].ratio), format!("{}%", e.rows[5].count), e.rows[5].raw, false));
    // Health's target is the literal 100 [M :105-108].
    out.push(row(0x15f, pct(e.rows[6].ratio), "100%".into(), e.rows[6].raw, false));
    // Time: current = the elapsed 1/100 s, target = par × 100, both through the timer format
    // [M :103-108 — entry +0x1c = rows[7].value, entry +0x20 × 100 = rows[7].count × 100].
    out.push(row(
        0x160,
        timer_string(e.rows[7].value as i64),
        timer_string(e.rows[7].count as i64 * 100),
        e.rows[7].raw,
        false,
    ));
    out.push(row(0x161, format!("{}", e.rows[8].value), format!("{}", e.rows[8].count), e.rows[8].raw, false));
    // The tail: the blank spacer row (the static 0x30d2d8 [M :117]), then Subtotal (+0x2c),
    // Difficulty Bonus (+0x28), TOTAL (+0x34), each a label row with SeparateNumber in the
    // TOTAL column [M :117-123].
    out.push(StatsRow { id: 0, category: String::new(), current: String::new(), target: String::new(), total: String::new(), indented: false });
    let tail = |id: u32, v: u32| StatsRow {
        id,
        category: bind(id),
        current: String::new(),
        target: String::new(),
        total: separate_number(v),
        indented: false,
    };
    out.push(tail(0x162, e.base));
    out.push(tail(0x164, e.bonus));
    out.push(tail(0x163, e.last));
    Some(out)
}

/// The page's header cells [M 0x209938:44-56 — the 0x1000204 bind, then cells 1/2/3 =
/// 0x10001e1/0x10001e2/0x163]: (Category, Current, Target, TOTAL). The same quartet the pause
/// SCORE tab binds [M 0x204408:447-498].
pub const STATS_HEADER: (u32, u32, u32, u32) = (0x1000_0204, 0x1000_01e1, 0x1000_01e2, 0x163);
