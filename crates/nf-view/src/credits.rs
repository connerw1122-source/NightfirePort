//! The credits roll's content and timing — `P_CREDITS_Handler` 0x2152b0 (page 0x40000030) and
//! `Menu_SetupCredits` 0x1f9a88. State + getters only: the main agent draws it.
//!
//! Opened from AV Options (page 0x40000031, its Credits action row) or from the end-of-game win
//! page `P_WINGAME_Handler` 0x215100 (page 0x40000053: it plays the background movie 0x073f0048
//! and, when `psiMovieFinished` reports done, sends msg 0x44 with 0x40000030 — the page change to
//! credits [M 0x215100:48]).
//!
//! Handler message cases [M 0x2152b0]: 0x4c enter (the setup below), 0x51 re-show
//! (`Menu_InitCredits` 0x202420 drops the cached table so the next enter rebuilds it), 0x50 the
//! per-frame tick, 0x4d leave (`Menu_KillCredits` 0x202428 is empty; `SFXStopMusic` +
//! `SFXMusicSetVolume(music_vol_524)` restore, the menu-loop sounds 0x1d7/0x470 restart and
//! `menu_movie = 1`). No input cases: the roll cannot be skipped.
//!
//! Enter (0x4c) [M 0x2152b0:72-94]: frame 0; the line table from `Menu_SetupCredits(0x30d54c)`;
//! the two list controls (0x10000213/0x10000214) cleared (msg 0x62) and set to list mode (0x2b/1);
//! `fading = 0`; `page_from = param_6`; the background control 0x1000023c's alpha word = 0xff when
//! `GameState[0x52] == 0` else 0x404040ff (the level-bonus flag `Menu_SetLevelBonus` 0x1ffa10:68
//! sets, persisted through the PTP save [M 0x1c8198:57/0x1c8560:62]); `music_vol_524 =
//! SFXMusicGetVolume()`; the menu-loop sounds 0x1d7/0x470 removed; `menu_movie = 0`; and
//! `SFXStartMusic(0x27, 0)` — the credits music track.
//!
//! Tick (0x50) [M 0x2152b0:103-195], in the game's 512×448 menu space:
//! 1. `frame += 1`.
//! 2. Every 14th frame (`frame % 0xe == 0`) while lines remain: the next line enters list slot
//!    `index_519` of BOTH columns at y 0x200 (512); the left item's text = the line's left string
//!    (msg 0x18), the fonts by kind (msg 0x23, [`credits_data::KIND_FORMAT`]); when the line is
//!    two-column (`w == 0`) the right item gets x 0xf0 w 0x100 and the line's right string, the
//!    left item x 0x10 w 0x100; when wide, the left item gets x 0x10 w 0x1e0 and the right item
//!    hides (0x2b/1). Then `index_519 = (index_519 + 1) % 26` and `name_520 += 1` — 26 slots
//!    cycling.
//! 3. Music fade once `frame/14 > 578 − 50`: `volume = (578×14 − frame) × music_vol/700`, clamped
//!    to ≥ 0 [M 0x2152b0:165-168] — a linear fade reaching 0 the frame the last line enters.
//! 4. End once `frame/14 > 578 + 18` [M :169-184]: `fading = 1`; a delayed message 0x13 (19
//!    frames) to the manager — msg 0x44 with 0x40000002 (the main menu) when `page_from` was the
//!    win page 0x40000053, else msg 0x5f (pop back to the opener); also a delayed 0x68 and an exit
//!    process on control 0x10000215 (params 0xe1/0xff/0 over 0xf frames — a page fade [S]).
//! 5. Every frame, ALL 26 slots of both columns scroll y −= 2 [M :186-194] — so a line that just
//!    entered shows y 510 on its enter frame, and travels 256 frames crossing the screen.

#[path = "credits_data.rs"]
pub mod credits_data;

use credits_data::CREDITS;

/// The credits page id [M Handler_HandleMessage 0x2029e0:411-413 → P_CREDITS_Handler].
pub const PAGE_CREDITS: u32 = 0x4000_0030;
/// The end-of-game win page [M 0x2029e0:569-572 → P_WINGAME_Handler].
pub const PAGE_WINGAME: u32 = 0x4000_0053;
/// The main menu, the credits' return page after the win path [M 0x2152b0:171-174].
pub const PAGE_MAIN: u32 = 0x4000_0002;
/// The AV Options page, the other opener [L menu-def page 0x40000031; P_CNAVOPTIONS 0x214818].
pub const PAGE_CN_AV: u32 = 0x4000_0031;

/// List slots per column [M the `% 0x1a` cycle, 0x2152b0:162, and the 0..0x1a scroll loop :185-194].
pub const LIST_SLOTS: usize = 26;
/// A line enters every 14th frame [M `frame % 0xe`, 0x2152b0:107].
pub const LINE_PERIOD: u32 = 14;
/// A line's entry y [M `*(u16*)(item + 0x72) = 0x200`, 0x2152b0:117-119].
pub const LINE_Y_START: i32 = 0x200;
/// Scroll speed, both columns, every frame [M `+= -2`, 0x2152b0:190-191].
pub const SCROLL_PER_FRAME: i32 = 2;
/// The credits music track [M `SFXStartMusic(0x27, 0)`, 0x2152b0:93].
pub const MUSIC_TRACK: u32 = 0x27;
/// The menu-loop sounds the enter/leave swap [M SFXRemove/SFXStart(0, ·) 0x2152b0:90-92, 0x215100:16-33].
pub const MENU_LOOP_SFX: [u32; 2] = [0x1d7, 0x470];
/// The end-of-game background movie the win page plays before the credits [M 0x215100:20].
pub const WINGAME_MOVIE: u32 = 0x073f_0048;
/// The delayed-message delay on the exit [M `__Menu_SendDelayedMessage__(0x13, …)`, 0x2152b0:172-178].
pub const EXIT_DELAY: u32 = 0x13;
/// Total lines [M the count 0x242 `Menu_SetupCredits` stores at 0x30d54c: 0x1b18 / 0xc].
pub const LINE_COUNT: usize = CREDITS.len();
/// The first frame where `frame % 14 == 0` — line 0 enters [M the tick increments `frame` first].
pub const FIRST_LINE_FRAME: u32 = LINE_PERIOD;
/// The frame the last line enters [M the `name_520 < count` guard — the 578th line at 578×14].
pub const LAST_LINE_FRAME: u32 = LINE_COUNT as u32 * LINE_PERIOD;
/// The fade trigger `count + 0x12 < frame/14` first holds at `frame = (578 + 19) × 14` [M 0x2152b0:169].
pub const FADE_FRAME: u32 = (LINE_COUNT as u32 + 18 + 1) * LINE_PERIOD;
/// The frame the delayed page change fires [M the 0x13 delay from the fade trigger].
pub const EXIT_FRAME: u32 = FADE_FRAME + EXIT_DELAY;
/// The music fade's first frame: `count − 0x32 < frame/14` first holds at `(578 − 49) × 14` [M :165].
pub const MUSIC_FADE_FRAME: u32 = (LINE_COUNT as u32 - 49) * LINE_PERIOD;
/// The fade's divisor [M `(float)music_vol_524 / 700.0`, 0x2152b0:166 — 50 lines × 14 frames].
pub const MUSIC_FADE_SPAN: f32 = 700.0;

/// Where the credits go at the end, decided by the opener [M 0x2152b0:171-177].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreditsExit {
    /// `page_from == 0x40000053` (the win page): msg 0x44 → the main menu 0x40000002.
    ToMain,
    /// Any other opener (AV Options): msg 0x5f — pop back to it.
    PopBack(u32),
}

/// The roll's state — the handler's own statics (`frame_518`, `name_520`, `index_519`,
/// `fading_517`, `page_from_523`, `music_vol_524`) plus the GameState[0x52] alpha flag.
#[derive(Clone, Debug)]
pub struct CreditsRoll {
    /// Frames since the page entered (the 0x4c reset).
    pub frame: u32,
    /// `name_520` — how many lines have entered; the next one is [`CREDITS`][next_line].
    pub next_line: usize,
    /// `index_519` — the NEXT list slot to fill (cycles mod 26).
    pub slot: usize,
    /// `fading_517`.
    pub fading: bool,
    /// `page_from_523` — the opener's page id.
    pub page_from: u32,
    /// `music_vol_524` — `SFXMusicGetVolume()` at enter.
    pub music_vol: i32,
    /// `GameState[0x52]` — the level-bonus flag; the drawer uses it for the background's alpha
    /// word (0xff when clear, 0x404040ff when set) [M 0x2152b0:83-88].
    pub bonus_flag: bool,
}

/// One visible line: its index in [`CREDITS`], its slot, and its y in the 512×448 space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VisibleLine {
    pub line: usize,
    pub slot: usize,
    /// y — the item's +0x72 word [M 0x2152b0:117-119/190-191].
    pub y: i32,
    /// True when the line is wide (the left string across 0x10..0x1f0) [M :158-161].
    pub wide: bool,
}

impl CreditsRoll {
    /// The 0x4c enter path.
    pub fn new(page_from: u32, music_vol: i32, bonus_flag: bool) -> Self {
        Self { frame: 0, next_line: 0, slot: 0, fading: false, page_from, music_vol, bonus_flag }
    }

    /// The 0x50 tick. Returns [`CreditsExit`] the frame the delayed page change fires. The music
    /// fade and the scroll stay lazy: [`Self::music_volume`] / [`Self::slot_y`] compute them, the
    /// caller applies them through its own audio/draw stacks.
    pub fn tick(&mut self) -> Option<CreditsExit> {
        self.frame += 1;
        // Step 2: the 14-frame line entry [M 0x2152b0:107-163].
        if self.frame % LINE_PERIOD == 0 && self.next_line < LINE_COUNT {
            self.slot = (self.slot + 1) % LIST_SLOTS;
            self.next_line += 1;
        }
        // Step 4: the fade trigger [M :169-184]; the exit fires EXIT_DELAY frames later.
        if !self.fading && self.frame / LINE_PERIOD > LINE_COUNT as u32 + 18 {
            self.fading = true;
        }
        if self.fading && self.frame >= EXIT_FRAME {
            return Some(match self.page_from {
                PAGE_WINGAME => CreditsExit::ToMain,
                from => CreditsExit::PopBack(from),
            });
        }
        None
    }

    /// The music volume the handler would set this frame [M 0x2152b0:165-168; None = no change].
    pub fn music_volume(&self) -> Option<i32> {
        if self.frame / LINE_PERIOD > LINE_COUNT as u32 - 50 {
            let v = (LINE_COUNT as f32 * LINE_PERIOD as f32 - self.frame as f32)
                * (self.music_vol as f32 / MUSIC_FADE_SPAN);
            Some((v as i32) * i32::from(v > 0.0))
        } else {
            None
        }
    }

    /// How far the roll has got: the last line index that has entered (None before the first).
    pub fn lines_entered(&self) -> Option<usize> {
        self.next_line.checked_sub(1)
    }

    /// The line occupying `slot` and its y (the game's item +0x72), or None while empty.
    ///
    /// The most recent line sits at `slot − 1` (mod 26) [M the post-increment at :162]; the line of
    /// age k (0 = most recent) entered at frame `(next_line − k) × 14`, was stamped at y 0x200 and
    /// the same tick's scroll already took it to 510 [M the handler's order: the entry code, then
    /// the 26-slot −2 loop, both in one 0x50 tick].
    pub fn slot_y(&self, slot: usize) -> Option<(usize, i32)> {
        if self.next_line == 0 {
            return None;
        }
        let age = (self.slot + LIST_SLOTS - 1 - slot) % LIST_SLOTS;
        if age >= self.next_line {
            return None;
        }
        let line = self.next_line - 1 - age;
        let enter_frame = line as u32 * LINE_PERIOD + LINE_PERIOD;
        Some((line, LINE_Y_START - SCROLL_PER_FRAME * (self.frame - enter_frame + 1) as i32))
    }

    /// Every visible line this frame, oldest first (the getters the main agent draws).
    pub fn visible(&self) -> Vec<VisibleLine> {
        let mut out = Vec::new();
        if self.next_line == 0 {
            return out;
        }
        let shown = self.next_line.min(LIST_SLOTS);
        for age in (0..shown).rev() {
            let line = self.next_line - 1 - age;
            let slot = (self.slot + LIST_SLOTS - 1 - age) % LIST_SLOTS;
            if let Some((_, y)) = self.slot_y(slot) {
                out.push(VisibleLine { line, slot, y, wide: CREDITS[line].w != 0 });
            }
        }
        out
    }
}
