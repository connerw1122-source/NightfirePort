//! The credits roll's line table [M Menu_SetupCredits 0x1f9a88 — the function builds 578 (0x242)
//! 12-byte entries on the stack, heap-copies 0x1b18 bytes, stores the count 0x242 through its
//! caller's pointer 0x30d54c and returns the heap copy]. P_CREDITS_Handler 0x2152b0 walks them.
//!
//! Entry layout at `credits_516 + i*0xc` [M 0x2152b0:121-161]:
//! `+0` left string, `+4` right string (`Txt_BindLabel` result, an ELF literal, `" "` at 0x30d240,
//! or NULL = the game sends msg 0x18 with 0 → the blank string 0x30d5a8), `+8` left font kind,
//! `+9` right font kind, `+10` wide flag. The kinds pick the label format the handler sends with
//! msg 0x23 [M 0x2152b0:122-147]: kind 1 → 0x30d560 (`ff 01 fe 03`), kind 2 → 0x30d568
//! (`ff 03 fe 03`), kind 0 → 0x30d570 (`ff 02 fe 03` + the `%s%s` tail 0x30d5b8 `fc 64 fb 64` —
//! the sprintf result equals the default label format DAT_003096c0 [D ELF bytes]). `ff 0N` = the
//! face, `fe 03` = centred (the port's reading of the same tags in menu.rs).
//!
//! Extracted 2026-10-15 by parsing the function's own memset/store sequence (the decompile of
//! ACTION.ELF 0x1f9a88, entry k at frame offset 0x1bd0 − 12k); every one of the 1156 string slots
//! is accounted for: 209 label binds + 492 ELF literals + 12 .data strings + 241 `" "` gaps +
//! 202 NULLs = 578 × 2. The label ids resolve through `Txt_BindLabel` (bank<<24|index,
//! 0x1cad60) — the port binds them with `Lang::bind` at draw time.
//!
//! Wide rows (`w != 0`) draw the LEFT string across 0x10..0x1f0 (w 0x1e0); two-column rows put the
//! left string in x 0x10 w 0x100 and the right string in x 0xf0 w 0x100 [M 0x2152b0:148-161].

use super::credits_data::CreditLine as L;

/// One string slot of a credit line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Str {
    /// `Txt_BindLabel(id, 0)` — the id into USATxt.dat (bank<<24 | index).
    Label(u32),
    /// The ELF's own literal (a name).
    Lit(&'static str),
    /// The gap string `" "` at 0x30d240 [D ELF byte 0x20].
    Space,
    /// NULL — msg 0x18 with 0 binds the blank string 0x30d5a8 [M Label_SendMessage 0x218890:44-47].
    Null,
}

/// One credits line: left/right strings, the per-side font kinds and the wide flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CreditLine {
    pub l: Str,
    pub r: Str,
    pub kl: u8,
    pub kr: u8,
    pub w: u8,
}

/// The order is the function's own initialisation order = the array's memory order (the copy loop
/// walks frame-0x1bd0 → frame-0xd0+24 ascending [M 0x1f9a88:3423-3443]).
pub const CREDITS: [CreditLine; 578] = [
    L { l: Str::Label(0x10002a5), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 0
    L { l: Str::Label(0x10002a6), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 1
    L { l: Str::Label(0x10002a7), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 2
    L { l: Str::Label(0x10002a8), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 3
    L { l: Str::Label(0x10002a9), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 4
    L { l: Str::Label(0x10002aa), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 5
    L { l: Str::Label(0x10002ab), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 6
    L { l: Str::Label(0x10002ac), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 7
    L { l: Str::Label(0x10002ad), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 8
    L { l: Str::Label(0x10002ae), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 9
    L { l: Str::Label(0x10002af), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 10
    L { l: Str::Label(0x10002b0), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 11
    L { l: Str::Label(0x10002b1), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 12
    L { l: Str::Label(0x10002b2), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 13
    L { l: Str::Label(0x10002b3), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 14
    L { l: Str::Label(0x10002b4), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 15
    L { l: Str::Label(0x10002b5), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 16
    L { l: Str::Label(0x10002b6), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 17
    L { l: Str::Label(0x10002b7), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 18
    L { l: Str::Label(0x10002b8), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 19
    L { l: Str::Label(0x1000319), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 20
    L { l: Str::Label(0x100031a), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 21
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 22
    L { l: Str::Label(0x10002b9), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 23
    L { l: Str::Label(0x10002ba), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 24
    L { l: Str::Label(0x10002bb), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 25
    L { l: Str::Label(0x10002bc), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 26
    L { l: Str::Label(0x10002bd), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 27
    L { l: Str::Label(0x10002be), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 28
    L { l: Str::Label(0x10002bf), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 29
    L { l: Str::Label(0x10002c0), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 30
    L { l: Str::Label(0x10002c1), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 31
    L { l: Str::Label(0x10002c2), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 32
    L { l: Str::Label(0x10002c3), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 33
    L { l: Str::Label(0x10002c4), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 34
    L { l: Str::Label(0x10002c5), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 35
    L { l: Str::Label(0x10002c6), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 36
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 37
    L { l: Str::Label(0x10002c7), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 38
    L { l: Str::Label(0x10002c8), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 39
    L { l: Str::Label(0x10002c9), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 40
    L { l: Str::Label(0x10002ca), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 41
    L { l: Str::Label(0x10002cb), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 42
    L { l: Str::Label(0x10002cc), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 43
    L { l: Str::Label(0x10002cd), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 44
    L { l: Str::Label(0x10002ce), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 45
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 46
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 47
    L { l: Str::Label(0x10002a3), r: Str::Null, kl: 1, kr: 0, w: 1 }, // 48
    L { l: Str::Label(0x10002a4), r: Str::Null, kl: 1, kr: 0, w: 1 }, // 49
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 50
    L { l: Str::Label(0x10002e0), r: Str::Label(0x10002e1), kl: 1, kr: 1, w: 0 }, // 51
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 52
    L { l: Str::Lit("EUROCOM"), r: Str::Label(0x100021f), kl: 1, kr: 1, w: 0 }, // 53
    L { l: Str::Label(0x2a4), r: Str::Label(0x1000236), kl: 2, kr: 2, w: 0 }, // 54
    L { l: Str::Lit("Tim Rogers"), r: Str::Lit("Brad Gour"), kl: 0, kr: 0, w: 0 }, // 55
    L { l: Str::Space, r: Str::Lit("Andrew Brownsword"), kl: 0, kr: 0, w: 0 }, // 56
    L { l: Str::Label(0x2a8), r: Str::Space, kl: 2, kr: 0, w: 0 }, // 57
    L { l: Str::Lit("Simon Mills"), r: Str::Label(0x1000237), kl: 0, kr: 2, w: 0 }, // 58
    L { l: Str::Space, r: Str::Lit("Stuart Riffle"), kl: 0, kr: 0, w: 0 }, // 59
    L { l: Str::Label(0x2a9), r: Str::Lit("Nathan Whitaker"), kl: 2, kr: 0, w: 0 }, // 60
    L { l: Str::Lit("Steve Camber"), r: Str::Lit("David Lucas"), kl: 0, kr: 0, w: 0 }, // 61
    L { l: Str::Lit("Mark Duffill"), r: Str::Lit("Mike Kiernan"), kl: 0, kr: 0, w: 0 }, // 62
    L { l: Str::Lit("Ashley Finney"), r: Str::Lit("Alan Bucior"), kl: 0, kr: 0, w: 0 }, // 63
    L { l: Str::Lit("Joel Garabedian"), r: Str::Lit("Nenad Jankovic"), kl: 0, kr: 0, w: 0 }, // 64
    L { l: Str::Lit("Jason Gosling"), r: Str::Lit("Ryan Ho"), kl: 0, kr: 0, w: 0 }, // 65
    L { l: Str::Lit("Mike Halsall"), r: Str::Lit("Josh Brown"), kl: 0, kr: 0, w: 0 }, // 66
    L { l: Str::Lit("Green Jam Canal"), r: Str::Lit("Gary Bearchell"), kl: 0, kr: 0, w: 0 }, // 67
    L { l: Str::Lit("Tim Rogers"), r: Str::Lit("Gary Shaw"), kl: 0, kr: 0, w: 0 }, // 68
    L { l: Str::Lit("Kev Stainwright"), r: Str::Lit("Manu Singh"), kl: 0, kr: 0, w: 0 }, // 69
    L { l: Str::Space, r: Str::Lit("Tim Morris"), kl: 0, kr: 0, w: 0 }, // 70
    L { l: Str::Label(0x100021c), r: Str::Lit("Ted Middleton"), kl: 2, kr: 0, w: 0 }, // 71
    L { l: Str::Lit("Andrew Hutchings"), r: Str::Lit("Peter Lolley"), kl: 0, kr: 0, w: 0 }, // 72
    L { l: Str::Space, r: Str::Lit("Dennis Tierney"), kl: 0, kr: 0, w: 0 }, // 73
    L { l: Str::Label(0x2aa), r: Str::Lit("Peter Ju"), kl: 2, kr: 0, w: 0 }, // 74
    L { l: Str::Lit("Chris Jordan"), r: Str::Lit("Todd Allendorf"), kl: 0, kr: 0, w: 0 }, // 75
    L { l: Str::Lit("Kevin Marks"), r: Str::Lit("Kenneth Strickland"), kl: 0, kr: 0, w: 0 }, // 76
    L { l: Str::Lit("Andy Mitchell"), r: Str::Space, kl: 0, kr: 0, w: 0 }, // 77
    L { l: Str::Lit("Richard Parkin"), r: Str::Label(0x1000230), kl: 0, kr: 2, w: 0 }, // 78
    L { l: Str::Lit("Bob Smith"), r: Str::Lit("Lance Wall"), kl: 0, kr: 0, w: 0 }, // 79
    L { l: Str::Space, r: Str::Lit("Amar Atwal"), kl: 0, kr: 0, w: 0 }, // 80
    L { l: Str::Label(0x2ab), r: Str::Lit("Ju-Lian Kwan"), kl: 2, kr: 0, w: 0 }, // 81
    L { l: Str::Lit("Rob Benton"), r: Str::Space, kl: 0, kr: 0, w: 0 }, // 82
    L { l: Str::Space, r: Str::Label(0x1000220), kl: 0, kr: 2, w: 0 }, // 83
    L { l: Str::Label(0x2ac), r: Str::Lit("Scott Jackson"), kl: 2, kr: 0, w: 0 }, // 84
    L { l: Str::Lit("Leavon Archer"), r: Str::Space, kl: 0, kr: 0, w: 0 }, // 85
    L { l: Str::Lit("Gary Baker"), r: Str::Label(0x1000238), kl: 0, kr: 2, w: 0 }, // 86
    L { l: Str::Lit("Andy Bee"), r: Str::Lit("Colin Doncaster"), kl: 0, kr: 0, w: 0 }, // 87
    L { l: Str::Lit("Nigel Bentley"), r: Str::Space, kl: 0, kr: 0, w: 0 }, // 88
    L { l: Str::Lit("Michael Boylan"), r: Str::Label(0x1000239), kl: 0, kr: 2, w: 0 }, // 89
    L { l: Str::Lit("Neil Davies"), r: Str::Lit("Eduardo Agostini"), kl: 0, kr: 0, w: 0 }, // 90
    L { l: Str::Lit("Richard Godwin"), r: Str::Lit("David Carman"), kl: 0, kr: 0, w: 0 }, // 91
    L { l: Str::Lit("Calvin Golkowski"), r: Str::Lit("Gregg Haggman"), kl: 0, kr: 0, w: 0 }, // 92
    L { l: Str::Lit("Simon Kirk"), r: Str::Lit("Curtis Hoines"), kl: 0, kr: 0, w: 0 }, // 93
    L { l: Str::Lit("Adrian Mannion"), r: Str::Lit("Ross Harris"), kl: 0, kr: 0, w: 0 }, // 94
    L { l: Str::Lit("Stuart McReath"), r: Str::Space, kl: 0, kr: 0, w: 0 }, // 95
    L { l: Str::Lit("Alan Pashley"), r: Str::Label(0x100023a), kl: 0, kr: 2, w: 0 }, // 96
    L { l: Str::Lit("Paul Robinson"), r: Str::Lit("David Adams"), kl: 0, kr: 0, w: 0 }, // 97
    L { l: Str::Space, r: Str::Lit("Gordon Chen"), kl: 0, kr: 0, w: 0 }, // 98
    L { l: Str::Label(0x2ad), r: Str::Space, kl: 2, kr: 0, w: 0 }, // 99
    L { l: Str::Lit("Martin Holliday"), r: Str::Label(0x1000224), kl: 0, kr: 2, w: 0 }, // 100
    L { l: Str::Space, r: Str::Lit("Phil Chow"), kl: 0, kr: 0, w: 0 }, // 101
    L { l: Str::Label(0x2ae), r: Str::Lit("Kim Gill"), kl: 2, kr: 0, w: 0 }, // 102
    L { l: Str::Lit("Darren Hyland"), r: Str::Lit("Josh Lee"), kl: 0, kr: 0, w: 0 }, // 103
    L { l: Str::Lit("Ludovic le Camus"), r: Str::Lit("Yvonne Vuu Carinha"), kl: 0, kr: 0, w: 0 }, // 104
    L { l: Str::Lit("Jon Maine"), r: Str::Lit("Garth Buzzard"), kl: 0, kr: 0, w: 0 }, // 105
    L { l: Str::Lit("Tom Noone"), r: Str::Lit("Anthony Leonati"), kl: 0, kr: 0, w: 0 }, // 106
    L { l: Str::Lit("Brian Riordan"), r: Str::Lit("Jude Godin"), kl: 0, kr: 0, w: 0 }, // 107
    L { l: Str::Space, r: Str::Lit("Mark Fraser"), kl: 0, kr: 0, w: 0 }, // 108
    L { l: Str::Label(0x100021d), r: Str::Lit("David White"), kl: 2, kr: 0, w: 0 }, // 109
    L { l: Str::Lit("Mark Jackson"), r: Str::Lit("Matt Head"), kl: 0, kr: 0, w: 0 }, // 110
    L { l: Str::Lit("Rob Matthews"), r: Str::Lit("Jim Tibbitts"), kl: 0, kr: 0, w: 0 }, // 111
    L { l: Str::Lit("Stephen Oades"), r: Str::Lit("William Wong"), kl: 0, kr: 0, w: 0 }, // 112
    L { l: Str::Lit("Lee Pullen"), r: Str::Lit("Carl Jarrett"), kl: 0, kr: 0, w: 0 }, // 113
    L { l: Str::Lit("John Stopforth"), r: Str::Lit("Toby Carr"), kl: 0, kr: 0, w: 0 }, // 114
    L { l: Str::Space, r: Str::Lit("Trevor Jones"), kl: 0, kr: 0, w: 0 }, // 115
    L { l: Str::Label(0x2af), r: Str::Lit("Barry Dempsey"), kl: 2, kr: 0, w: 0 }, // 116
    L { l: Str::Lit("Neil Baldwin"), r: Str::Lit("George Ashcroft"), kl: 0, kr: 0, w: 0 }, // 117
    L { l: Str::Space, r: Str::Label(0x100023b), kl: 0, kr: 0, w: 0 }, // 118
    L { l: Str::Label(0x2b0), r: Str::Lit("Vince Fung"), kl: 2, kr: 0, w: 0 }, // 119
    L { l: Str::Lit("Steve Duckworth"), r: Str::Lit("Becky Chow"), kl: 0, kr: 0, w: 0 }, // 120
    L { l: Str::Space, r: Str::Lit("Chris Trimble"), kl: 0, kr: 0, w: 0 }, // 121
    L { l: Str::Label(0x2a5), r: Str::Lit("Larry Weiss"), kl: 2, kr: 0, w: 0 }, // 122
    L { l: Str::Lit("Bill Beacham"), r: Str::Lit("Ivetta Harte"), kl: 0, kr: 0, w: 0 }, // 123
    L { l: Str::Space, r: Str::Lit("Shayne Herrera"), kl: 0, kr: 0, w: 0 }, // 124
    L { l: Str::Label(0x2a7), r: Str::Lit("Ian Sorenson"), kl: 2, kr: 0, w: 0 }, // 125
    L { l: Str::Lit("Bill Beacham"), r: Str::Lit("Amy Bond"), kl: 0, kr: 0, w: 0 }, // 126
    L { l: Str::Lit("Andy Collins"), r: Str::Lit("David Mosher"), kl: 0, kr: 0, w: 0 }, // 127
    L { l: Str::Lit("Gareth Glover"), r: Str::Lit("Stephen Tang"), kl: 0, kr: 0, w: 0 }, // 128
    L { l: Str::Lit("Kev Harrison"), r: Str::Lit("Chris Jung"), kl: 0, kr: 0, w: 0 }, // 129
    L { l: Str::Lit("Aaron Jenkins"), r: Str::Lit("Marc Wilhelm"), kl: 0, kr: 0, w: 0 }, // 130
    L { l: Str::Space, r: Str::Lit("Mike Kaczmarek"), kl: 0, kr: 0, w: 0 }, // 131
    L { l: Str::Label(0x2a6), r: Str::Lit("Art Matsuura"), kl: 2, kr: 0, w: 0 }, // 132
    L { l: Str::Lit("John Whiston"), r: Str::Lit("Will Friedewald"), kl: 0, kr: 0, w: 0 }, // 133
    L { l: Str::Space, r: Str::Lit("Mark West"), kl: 0, kr: 0, w: 0 }, // 134
    L { l: Str::Label(0x100021e), r: Str::Lit("Russel Cahalan"), kl: 2, kr: 0, w: 0 }, // 135
    L { l: Str::Lit("Lee Campbell"), r: Str::Lit("Mark Sullivan"), kl: 0, kr: 0, w: 0 }, // 136
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 0 }, // 137
    L { l: Str::Label(0x2b1), r: Str::Label(0x100023c), kl: 2, kr: 2, w: 0 }, // 138
    L { l: Str::Lit("Michael Robinson"), r: Str::Lit("Daniel Dociu"), kl: 0, kr: 0, w: 0 }, // 139
    L { l: Str::Space, r: Str::Lit("Kirk Henderson"), kl: 0, kr: 0, w: 0 }, // 140
    L { l: Str::Label(0x2b2), r: Str::Lit("Norman Felchle"), kl: 2, kr: 0, w: 0 }, // 141
    L { l: Str::Lit("Richard Charles"), r: Str::Lit("Gia Luc"), kl: 0, kr: 0, w: 0 }, // 142
    L { l: Str::Space, r: Str::Lit("Jason Courtney"), kl: 0, kr: 0, w: 0 }, // 143
    L { l: Str::Label(0x2b3), r: Str::Lit("Ken Nicholls"), kl: 2, kr: 0, w: 0 }, // 144
    L { l: Str::Lit("Stuart Bee"), r: Str::Space, kl: 0, kr: 0, w: 0 }, // 145
    L { l: Str::Lit("Duncan Bradshaw"), r: Str::Label(0x2af), kl: 0, kr: 2, w: 0 }, // 146
    L { l: Str::Lit("Nick Connolly"), r: Str::Lit("Jennifer Lewis"), kl: 0, kr: 0, w: 0 }, // 147
    L { l: Str::Lit("Andrew Hardy"), r: Str::Space, kl: 0, kr: 0, w: 0 }, // 148
    L { l: Str::Lit("Lee Hill"), r: Str::Label(0x100023d), kl: 0, kr: 2, w: 0 }, // 149
    L { l: Str::Lit("Oliver Madden"), r: Str::Lit("Olo"), kl: 0, kr: 0, w: 0 }, // 150
    L { l: Str::Lit("Chris Jackson"), r: Str::Lit("Adam Boyd"), kl: 0, kr: 0, w: 0 }, // 151
    L { l: Str::Lit("Jamie Salmon"), r: Str::Space, kl: 0, kr: 0, w: 0 }, // 152
    L { l: Str::Lit("Jamie Small"), r: Str::Label(0x100023e), kl: 0, kr: 2, w: 0 }, // 153
    L { l: Str::Space, r: Str::Lit("Jason Fredrickson"), kl: 0, kr: 0, w: 0 }, // 154
    L { l: Str::Space, r: Str::Lit("Bryan Rennie"), kl: 0, kr: 0, w: 0 }, // 155
    L { l: Str::Label(0x100021f), r: Str::Lit("Todd Araki"), kl: 1, kr: 0, w: 0 }, // 156
    L { l: Str::Space, r: Str::Lit("Gregg Sabitz"), kl: 0, kr: 0, w: 0 }, // 157
    L { l: Str::Label(0x1000220), r: Str::Space, kl: 2, kr: 0, w: 0 }, // 158
    L { l: Str::Lit("Dave Carson"), r: Str::Label(0x100023f), kl: 0, kr: 2, w: 0 }, // 159
    L { l: Str::Space, r: Str::Lit("Jeff Tymoschuk"), kl: 0, kr: 0, w: 0 }, // 160
    L { l: Str::Label(0x1000221), r: Str::Space, kl: 2, kr: 0, w: 0 }, // 161
    L { l: Str::Lit("Cate Latchford"), r: Str::Label(0x1000240), kl: 0, kr: 2, w: 0 }, // 162
    L { l: Str::Space, r: Str::Lit("Francois \"KoiHZ\" Lafleur"), kl: 0, kr: 0, w: 0 }, // 163
    L { l: Str::Label(0x1000222), r: Str::Space, kl: 2, kr: 0, w: 0 }, // 164
    L { l: Str::Lit("Erik Tiemens"), r: Str::Label(0x1000226), kl: 0, kr: 2, w: 0 }, // 165
    L { l: Str::Lit("Richard Kriegler"), r: Str::Lit("Hanno Lemke"), kl: 0, kr: 0, w: 0 }, // 166
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 0 }, // 167
    L { l: Str::Label(0x1000223), r: Str::Label(0x1000241), kl: 2, kr: 2, w: 0 }, // 168
    L { l: Str::Lit("Derek Thompsen"), r: Str::Lit("Scott Blackwood"), kl: 0, kr: 0, w: 0 }, // 169
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 0 }, // 170
    L { l: Str::Label(0x1000239), r: Str::Label(0x1000228), kl: 2, kr: 2, w: 0 }, // 171
    L { l: Str::Lit("Yukiko Arikawa"), r: Str::Lit("Ofer Estline"), kl: 0, kr: 0, w: 0 }, // 172
    L { l: Str::Lit("Darren Pattenden"), r: Str::Space, kl: 0, kr: 0, w: 0 }, // 173
    L { l: Str::Space, r: Str::Label(0x1000242), kl: 0, kr: 2, w: 0 }, // 174
    L { l: Str::Label(0x1000224), r: Str::Lit("Jason DeLong"), kl: 2, kr: 0, w: 0 }, // 175
    L { l: Str::Lit("Stephen Tang"), r: Str::Lit("Al Escudero"), kl: 0, kr: 0, w: 0 }, // 176
    L { l: Str::Lit("Chris Jung"), r: Str::Lit("Rob Davidson"), kl: 0, kr: 0, w: 0 }, // 177
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 0 }, // 178
    L { l: Str::Label(0x1000225), r: Str::Label(0x100022a), kl: 2, kr: 2, w: 0 }, // 179
    L { l: Str::Lit("Scott Whitney"), r: Str::Lit("Jim Boland"), kl: 0, kr: 0, w: 0 }, // 180
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 0 }, // 181
    L { l: Str::Label(0x1000226), r: Str::Label(0x1000243), kl: 2, kr: 2, w: 0 }, // 182
    L { l: Str::Lit("Jon Horsley"), r: Str::Lit("Nick Corea"), kl: 0, kr: 0, w: 0 }, // 183
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 0 }, // 184
    L { l: Str::Label(0x2a5), r: Str::Label(0x1000244), kl: 2, kr: 2, w: 0 }, // 185
    L { l: Str::Lit("Hunter Smith"), r: Str::Lit("Arlaine Walker"), kl: 0, kr: 0, w: 0 }, // 186
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 0 }, // 187
    L { l: Str::Label(0x1000227), r: Str::Label(0x1000245), kl: 2, kr: 2, w: 0 }, // 188
    L { l: Str::Lit("David Luoto"), r: Str::Lit("Serena Blackwood"), kl: 0, kr: 0, w: 0 }, // 189
    L { l: Str::Space, r: Str::Lit("Lia Siojo"), kl: 0, kr: 0, w: 0 }, // 190
    L { l: Str::Label(0x1000228), r: Str::Space, kl: 2, kr: 0, w: 0 }, // 191
    L { l: Str::Lit("Joel Wade"), r: Str::Label(0x100022d), kl: 0, kr: 2, w: 0 }, // 192
    L { l: Str::Space, r: Str::Lit("Marcel Laforce"), kl: 0, kr: 0, w: 0 }, // 193
    L { l: Str::Label(0x1000229), r: Str::Space, kl: 2, kr: 0, w: 0 }, // 194
    L { l: Str::Lit("Ken Rogers"), r: Str::Label(0x1000247), kl: 0, kr: 2, w: 0 }, // 195
    L { l: Str::Space, r: Str::Lit("Harry Ross"), kl: 0, kr: 0, w: 0 }, // 196
    L { l: Str::Label(0x100022a), r: Str::Lit("Mark Little"), kl: 2, kr: 0, w: 0 }, // 197
    L { l: Str::Lit("Erin Turner"), r: Str::Lit("Scott Taylor"), kl: 0, kr: 0, w: 0 }, // 198
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 0 }, // 199
    L { l: Str::Label(0x100022b), r: Str::Label(0x1000248), kl: 2, kr: 2, w: 0 }, // 200
    L { l: Str::Lit("Michael Condrey, Morgan Roarty"), r: Str::Lit("Elaine McMurtrie"), kl: 0, kr: 0, w: 0 }, // 201
    L { l: Str::Lit("John Vifian"), r: Str::Lit("Gabbi Villarrubia"), kl: 0, kr: 0, w: 0 }, // 202
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 0 }, // 203
    L { l: Str::Label(0x100022c), r: Str::Label(0x1000249), kl: 2, kr: 2, w: 0 }, // 204
    L { l: Str::Lit("Brent Iverson"), r: Str::Lit("Darcy Pajak"), kl: 0, kr: 0, w: 0 }, // 205
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 0 }, // 206
    L { l: Str::Label(0x100022d), r: Str::Label(0x100024a), kl: 2, kr: 2, w: 0 }, // 207
    L { l: Str::Lit("Jim Rushing"), r: Str::Lit("Martin McQueen"), kl: 0, kr: 0, w: 0 }, // 208
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 0 }, // 209
    L { l: Str::Label(0x100022e), r: Str::Label(0x100024b), kl: 2, kr: 2, w: 0 }, // 210
    L { l: Str::Lit("Stephen Barry"), r: Str::Lit("Jamey Brewster"), kl: 0, kr: 0, w: 0 }, // 211
    L { l: Str::Space, r: Str::Lit("Carvy Spindler"), kl: 0, kr: 0, w: 0 }, // 212
    L { l: Str::Label(0x100022f), r: Str::Space, kl: 2, kr: 0, w: 0 }, // 213
    L { l: Str::Lit("Chuck Beaver"), r: Str::Label(0x2b3), kl: 0, kr: 2, w: 0 }, // 214
    L { l: Str::Space, r: Str::Lit("Jimmy Li"), kl: 0, kr: 0, w: 0 }, // 215
    L { l: Str::Label(0x1000230), r: Str::Lit("Fred Zhao"), kl: 2, kr: 0, w: 0 }, // 216
    L { l: Str::Lit("Nathan 'Acorn' Pooley"), r: Str::Lit("Carlo Audia"), kl: 0, kr: 0, w: 0 }, // 217
    L { l: Str::Lit("James Petrick"), r: Str::Lit("Liam Millar"), kl: 0, kr: 0, w: 0 }, // 218
    L { l: Str::Space, r: Str::Lit("Leo Chiu"), kl: 0, kr: 0, w: 0 }, // 219
    L { l: Str::Label(0x1000231), r: Str::Lit("Matthew McGillivray"), kl: 2, kr: 0, w: 0 }, // 220
    L { l: Str::Lit("Dave Steele"), r: Str::Lit("Sean Smillie"), kl: 0, kr: 0, w: 0 }, // 221
    L { l: Str::Space, r: Str::Lit("Brad Thompson"), kl: 0, kr: 0, w: 0 }, // 222
    L { l: Str::Label(0x1000232), r: Str::Lit("Jason Brown"), kl: 2, kr: 0, w: 0 }, // 223
    L { l: Str::Lit("Tim Attuquayefio"), r: Str::Lit("Steven Yago"), kl: 0, kr: 0, w: 0 }, // 224
    L { l: Str::Space, r: Str::Lit("Matthew Marteinsson"), kl: 0, kr: 0, w: 0 }, // 225
    L { l: Str::Label(0x1000233), r: Str::Lit("Edward Lambke"), kl: 2, kr: 0, w: 0 }, // 226
    L { l: Str::Lit("Shinichiro Ohyama"), r: Str::Lit("Sean Desharnais"), kl: 0, kr: 0, w: 0 }, // 227
    L { l: Str::Lit("Colin Clarke"), r: Str::Lit("Ryan Greenhow"), kl: 0, kr: 0, w: 0 }, // 228
    L { l: Str::Lit("James Impey"), r: Str::Lit("Phil Nakis"), kl: 0, kr: 0, w: 0 }, // 229
    L { l: Str::Lit("William Douglas"), r: Str::Lit("Edward Vanance"), kl: 0, kr: 0, w: 0 }, // 230
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 0 }, // 231
    L { l: Str::Label(0x1000234), r: Str::Label(0x100024c), kl: 2, kr: 2, w: 0 }, // 232
    L { l: Str::Lit("Seth Mespelli"), r: Str::Label(0x100031b), kl: 0, kr: 2, w: 0 }, // 233
    L { l: Str::Lit("Rob Walton"), r: Str::Label(0x100024d), kl: 0, kr: 2, w: 0 }, // 234
    L { l: Str::Lit("Patrick O'Shaughnessy"), r: Str::Lit("Joanne Bondoc"), kl: 0, kr: 0, w: 0 }, // 235
    L { l: Str::Lit("Tom Benson"), r: Str::Space, kl: 0, kr: 0, w: 0 }, // 236
    L { l: Str::Lit("Johnson Lee"), r: Str::Label(0x100024e), kl: 0, kr: 2, w: 0 }, // 237
    L { l: Str::Lit("Thomas Burkett"), r: Str::Lit("David Wiens"), kl: 0, kr: 0, w: 0 }, // 238
    L { l: Str::Lit("James Stevens T. DeGuzman"), r: Str::Space, kl: 0, kr: 0, w: 0 }, // 239
    L { l: Str::Lit("Russ Fan"), r: Str::Label(0x1000251), kl: 0, kr: 2, w: 0 }, // 240
    L { l: Str::Lit("Petr Filipchyk"), r: Str::Label(0x1000228), kl: 0, kr: 2, w: 0 }, // 241
    L { l: Str::Space, r: Str::Lit("Chacko Sonny"), kl: 0, kr: 0, w: 0 }, // 242
    L { l: Str::Label(0x1000235), r: Str::Space, kl: 2, kr: 0, w: 0 }, // 243
    L { l: Str::Lit("Alan Grey"), r: Str::Label(0x1000220), kl: 0, kr: 2, w: 0 }, // 244
    L { l: Str::Lit("Albert Cin"), r: Str::Lit("Rob Stahl"), kl: 0, kr: 0, w: 0 }, // 245
    L { l: Str::Lit("Casey Arnold"), r: Str::Space, kl: 0, kr: 0, w: 0 }, // 246
    L { l: Str::Lit("Reid Lind"), r: Str::Label(0x1000224), kl: 0, kr: 2, w: 0 }, // 247
    L { l: Str::Lit("Jay Jaccobs"), r: Str::Lit("Andrew Rai"), kl: 0, kr: 0, w: 0 }, // 248
    L { l: Str::Lit("Rodney Fleig"), r: Str::Lit("Jorge Riverol"), kl: 0, kr: 0, w: 0 }, // 249
    L { l: Str::Lit("Chris Smith"), r: Str::Lit("Wan Chiu"), kl: 0, kr: 0, w: 0 }, // 250
    L { l: Str::Lit("Edward Ng"), r: Str::Lit("Jack Burton"), kl: 0, kr: 0, w: 0 }, // 251
    L { l: Str::Lit("Liam Bickford"), r: Str::Space, kl: 0, kr: 0, w: 0 }, // 252
    L { l: Str::Lit("Justin Couron"), r: Str::Label(0x1000252), kl: 0, kr: 2, w: 0 }, // 253
    L { l: Str::Lit("Lincoln Anderson"), r: Str::Lit("Felix Kupis"), kl: 0, kr: 0, w: 0 }, // 254
    L { l: Str::Lit("Ryan Jones"), r: Str::Space, kl: 0, kr: 0, w: 0 }, // 255
    L { l: Str::Lit("Chris Lowe"), r: Str::Label(0x1000253), kl: 0, kr: 2, w: 0 }, // 256
    L { l: Str::Lit("Devon Dossett"), r: Str::Lit("Chris Larsen"), kl: 0, kr: 0, w: 0 }, // 257
    L { l: Str::Lit("Eizer Queja"), r: Str::Lit("Paul Waggonner"), kl: 0, kr: 0, w: 0 }, // 258
    L { l: Str::Lit("Roy Huerta"), r: Str::Lit("Sean O' Connor"), kl: 0, kr: 0, w: 0 }, // 259
    L { l: Str::Lit("Joe Greene"), r: Str::Lit("Larry Mast"), kl: 0, kr: 0, w: 0 }, // 260
    L { l: Str::Lit("Tara Boyd"), r: Str::Lit("Kevin Cunningham"), kl: 0, kr: 0, w: 0 }, // 261
    L { l: Str::Lit("Jason Bianchi"), r: Str::Lit("Eric Scharf"), kl: 0, kr: 0, w: 0 }, // 262
    L { l: Str::Lit("Matt Ybarra"), r: Str::Space, kl: 0, kr: 0, w: 0 }, // 263
    L { l: Str::Lit("Alex Gannaway"), r: Str::Label(0x1000237), kl: 0, kr: 2, w: 0 }, // 264
    L { l: Str::Lit("Josh Kaufmann"), r: Str::Lit("Sebastien Juanico"), kl: 0, kr: 0, w: 0 }, // 265
    L { l: Str::Lit("Sam Worf"), r: Str::Lit("Tim Morten"), kl: 0, kr: 0, w: 0 }, // 266
    L { l: Str::Lit("Joe Humfield"), r: Str::Space, kl: 0, kr: 0, w: 0 }, // 267
    L { l: Str::Lit("Jeremy Ruckenbrod"), r: Str::Label(0x1000312), kl: 0, kr: 1, w: 0 }, // 268
    L { l: Str::Lit("Kit Stormont"), r: Str::Label(0x1000259), kl: 0, kr: 2, w: 0 }, // 269
    L { l: Str::Lit("Alex Freeman"), r: Str::Label(0x100031c), kl: 0, kr: 2, w: 0 }, // 270
    L { l: Str::Lit("Kevin T Lee"), r: Str::Lit("Atsuko Matsumoto"), kl: 0, kr: 0, w: 0 }, // 271
    L { l: Str::Lit("Frederick Ng"), r: Str::Space, kl: 0, kr: 0, w: 0 }, // 272
    L { l: Str::Lit("Shereif Fattouh"), r: Str::Label(0x100025a), kl: 0, kr: 2, w: 0 }, // 273
    L { l: Str::Lit("Michaelangelo Mesina"), r: Str::Label(0x100031d), kl: 0, kr: 2, w: 0 }, // 274
    L { l: Str::Lit("Gayle Lautenschlager"), r: Str::Lit("John Pemberton"), kl: 0, kr: 0, w: 0 }, // 275
    L { l: Str::Lit("Benjamin Buckland"), r: Str::Space, kl: 0, kr: 0, w: 0 }, // 276
    L { l: Str::Lit("William Gatley"), r: Str::Label(0x100025b), kl: 0, kr: 2, w: 0 }, // 277
    L { l: Str::Lit("William Nguyen"), r: Str::Label(0x100031e), kl: 0, kr: 2, w: 0 }, // 278
    L { l: Str::Lit("Allen Rusakovsky"), r: Str::Lit("Jonathan Silverman"), kl: 0, kr: 0, w: 0 }, // 279
    L { l: Str::Lit("Adam Ellington"), r: Str::Space, kl: 0, kr: 0, w: 0 }, // 280
    L { l: Str::Lit("Chris Seckler"), r: Str::Label(0x100025c), kl: 0, kr: 2, w: 0 }, // 281
    L { l: Str::Lit("Eric Hatton"), r: Str::Label(0x100031f), kl: 0, kr: 2, w: 0 }, // 282
    L { l: Str::Lit("Sergio Arizpe"), r: Str::Lit("Joel Knutson"), kl: 0, kr: 0, w: 0 }, // 283
    L { l: Str::Lit("Jim Larimore"), r: Str::Space, kl: 0, kr: 0, w: 0 }, // 284
    L { l: Str::Lit("Steve Jenkins"), r: Str::Label(0x100024f), kl: 0, kr: 2, w: 0 }, // 285
    L { l: Str::Lit("Hunter Barnett"), r: Str::Label(0x1000320), kl: 0, kr: 2, w: 0 }, // 286
    L { l: Str::Lit("Randy Pace"), r: Str::Lit("Gabriel Darone"), kl: 0, kr: 0, w: 0 }, // 287
    L { l: Str::Lit("Frank Morales"), r: Str::Space, kl: 0, kr: 0, w: 0 }, // 288
    L { l: Str::Lit("Kevin Lee"), r: Str::Label(0x1000250), kl: 0, kr: 2, w: 0 }, // 289
    L { l: Str::Lit("Chad Nolden"), r: Str::Label(0x1000321), kl: 0, kr: 2, w: 0 }, // 290
    L { l: Str::Lit("Justin Ruiz"), r: Str::Lit("Erick Arnold"), kl: 0, kr: 0, w: 0 }, // 291
    L { l: Str::Lit("Mike Bratcher"), r: Str::Lit("Ben Flax"), kl: 0, kr: 0, w: 0 }, // 292
    L { l: Str::Lit("Sam Mansour"), r: Str::Lit("Christopher Heil"), kl: 0, kr: 0, w: 0 }, // 293
    L { l: Str::Lit("Thad Oba"), r: Str::Lit("Rory Loeb"), kl: 0, kr: 0, w: 0 }, // 294
    L { l: Str::Lit("Rommel TabTab"), r: Str::Lit("Justin Pace"), kl: 0, kr: 0, w: 0 }, // 295
    L { l: Str::Lit("Trent Best"), r: Str::Lit("Jose Paredes"), kl: 0, kr: 0, w: 0 }, // 296
    L { l: Str::Lit("Ken Riley"), r: Str::Lit("Paul Reilly"), kl: 0, kr: 0, w: 0 }, // 297
    L { l: Str::Lit("Nick Olson"), r: Str::Lit("Jason Robak"), kl: 0, kr: 0, w: 0 }, // 298
    L { l: Str::Lit("Jerimiah Holland"), r: Str::Lit("Walter Roth"), kl: 0, kr: 0, w: 0 }, // 299
    L { l: Str::Lit("Rashad Redic"), r: Str::Lit("David Song"), kl: 0, kr: 0, w: 0 }, // 300
    L { l: Str::Lit("Chris Ward"), r: Str::Lit("Cameron Travelli"), kl: 0, kr: 0, w: 0 }, // 301
    L { l: Str::Lit("Donnie Taylor"), r: Str::Lit("Marcelo Vásquez"), kl: 0, kr: 0, w: 0 }, // 302
    L { l: Str::Lit("Dave Cohen"), r: Str::Lit("Brian Yip"), kl: 0, kr: 0, w: 0 }, // 303
    L { l: Str::Lit("Wes Webb"), r: Str::Lit("Andres Yulo"), kl: 0, kr: 0, w: 0 }, // 304
    L { l: Str::Lit("Lee Hahn"), r: Str::Space, kl: 0, kr: 0, w: 0 }, // 305
    L { l: Str::Lit("Adam Katsarelis"), r: Str::Label(0x100025d), kl: 0, kr: 2, w: 0 }, // 306
    L { l: Str::Lit("Andy Grossberg"), r: Str::Lit("Nicklas Mether "), kl: 0, kr: 0, w: 0 }, // 307
    L { l: Str::Lit("Brett DeCelle"), r: Str::Lit("Steven Saunders"), kl: 0, kr: 0, w: 0 }, // 308
    L { l: Str::Lit("Daniel Furr"), r: Str::Lit("Markus Lubik"), kl: 0, kr: 0, w: 0 }, // 309
    L { l: Str::Lit("Jonathan Beyer"), r: Str::Lit("Javier Martinez"), kl: 0, kr: 0, w: 0 }, // 310
    L { l: Str::Lit("Josh Hereshoff"), r: Str::Lit("Matteo Milandri"), kl: 0, kr: 0, w: 0 }, // 311
    L { l: Str::Lit("Andre Jong"), r: Str::Lit("Samy Ben romdhane"), kl: 0, kr: 0, w: 0 }, // 312
    L { l: Str::Lit("Dave Constantino"), r: Str::Lit("Andreas Dietz"), kl: 0, kr: 0, w: 0 }, // 313
    L { l: Str::Lit("Jerome Fernandez"), r: Str::Lit("Jelle de Vaal"), kl: 0, kr: 0, w: 0 }, // 314
    L { l: Str::Lit("Chris Smith"), r: Str::Lit("Eduardo Lage"), kl: 0, kr: 0, w: 0 }, // 315
    L { l: Str::Lit("Matt Coughlan"), r: Str::Lit("Jorge Ysart"), kl: 0, kr: 0, w: 0 }, // 316
    L { l: Str::Lit("Ryan Edwards"), r: Str::Space, kl: 0, kr: 0, w: 0 }, // 317
    L { l: Str::Lit("Chris Beaver"), r: Str::Label(0x1000313), kl: 0, kr: 2, w: 0 }, // 318
    L { l: Str::Lit("Richard Hartzell"), r: Str::Lit("Christine Jean"), kl: 0, kr: 0, w: 0 }, // 319
    L { l: Str::Lit("Dan Dwonch"), r: Str::Lit("Nathalie Duret"), kl: 0, kr: 0, w: 0 }, // 320
    L { l: Str::Lit("Nelson Rodriguez"), r: Str::Lit("Maëllenn Lumineau"), kl: 0, kr: 0, w: 0 }, // 321
    L { l: Str::Space, r: Str::Lit("Franck Badin"), kl: 0, kr: 0, w: 0 }, // 322
    L { l: Str::Label(0x1000254), r: Str::Lit("Frédéric Aubrun"), kl: 1, kr: 0, w: 0 }, // 323
    L { l: Str::Space, r: Str::Lit("David Gély"), kl: 0, kr: 0, w: 0 }, // 324
    L { l: Str::Label(0x1000255), r: Str::Lit("Christine Delholme"), kl: 2, kr: 0, w: 0 }, // 325
    L { l: Str::Lit("Alex Plachowski"), r: Str::Lit("Sam Yazmadjian "), kl: 0, kr: 0, w: 0 }, // 326
    L { l: Str::Space, r: Str::Lit("David Lapp"), kl: 0, kr: 0, w: 0 }, // 327
    L { l: Str::Label(0x1000256), r: Str::Lit("Sylvain Caburrosso"), kl: 2, kr: 0, w: 0 }, // 328
    L { l: Str::Lit("Matt Vann"), r: Str::Space, kl: 0, kr: 0, w: 0 }, // 329
    L { l: Str::Space, r: Str::Label(0x1000314), kl: 0, kr: 2, w: 0 }, // 330
    L { l: Str::Label(0x1000257), r: Str::Lit("Michaela Bartelt"), kl: 2, kr: 0, w: 0 }, // 331
    L { l: Str::Lit("Bruce Dixon"), r: Str::Lit("Nadine Monschau"), kl: 0, kr: 0, w: 0 }, // 332
    L { l: Str::Lit("Nick Dolce"), r: Str::Lit("Bettina Bachon"), kl: 0, kr: 0, w: 0 }, // 333
    L { l: Str::Lit("Kevin Bailey"), r: Str::Lit("Effective Media"), kl: 0, kr: 0, w: 0 }, // 334
    L { l: Str::Lit("Sean Bailey"), r: Str::Lit("Nadine Monschau"), kl: 0, kr: 0, w: 0 }, // 335
    L { l: Str::Lit("Ariel Gonzalez"), r: Str::Lit("Dirk Vojtilo"), kl: 0, kr: 0, w: 0 }, // 336
    L { l: Str::Lit("Shelton Allemand"), r: Str::Space, kl: 0, kr: 0, w: 0 }, // 337
    L { l: Str::Lit("Mark Siefert"), r: Str::Label(0x1000315), kl: 0, kr: 2, w: 0 }, // 338
    L { l: Str::Space, r: Str::Lit("Luis Pinés"), kl: 0, kr: 0, w: 0 }, // 339
    L { l: Str::Label(0x100025e), r: Str::Lit("Antonio Yago"), kl: 2, kr: 0, w: 0 }, // 340
    L { l: Str::Label(0x1000322), r: Str::Lit("Elixabete Aldama"), kl: 2, kr: 0, w: 0 }, // 341
    L { l: Str::Lit("Jillian Goldberg"), r: Str::Lit("Enrique Sánchez-Real"), kl: 0, kr: 0, w: 0 }, // 342
    L { l: Str::Space, r: Str::Lit("Sergio Vargas"), kl: 0, kr: 0, w: 0 }, // 343
    L { l: Str::Label(0x100025f), r: Str::Space, kl: 2, kr: 0, w: 0 }, // 344
    L { l: Str::Lit("Don Mesa"), r: Str::Label(0x1000316), kl: 0, kr: 2, w: 0 }, // 345
    L { l: Str::Lit("Lincoln Hershberger"), r: Str::Lit("Micaela Belluzzo"), kl: 0, kr: 0, w: 0 }, // 346
    L { l: Str::Space, r: Str::Lit("Alessandro Tajana"), kl: 0, kr: 0, w: 0 }, // 347
    L { l: Str::Label(0x1000260), r: Str::Lit("Matteo Milandri"), kl: 2, kr: 0, w: 0 }, // 348
    L { l: Str::Lit("Jeane Wong"), r: Str::Lit("Andrea Santambrogio"), kl: 0, kr: 0, w: 0 }, // 349
    L { l: Str::Space, r: Str::Lit("Claudio Scolastici"), kl: 0, kr: 0, w: 0 }, // 350
    L { l: Str::Label(0x1000261), r: Str::Space, kl: 2, kr: 0, w: 0 }, // 351
    L { l: Str::Lit("Jerris Mungai"), r: Str::Label(0x1000317), kl: 0, kr: 2, w: 0 }, // 352
    L { l: Str::Space, r: Str::Lit("Rebecca Persson"), kl: 0, kr: 0, w: 0 }, // 353
    L { l: Str::Label(0x1000262), r: Str::Lit("Mia Opancar"), kl: 2, kr: 0, w: 0 }, // 354
    L { l: Str::Lit("Rich Ridlen"), r: Str::Lit("Mattias Göthe"), kl: 0, kr: 0, w: 0 }, // 355
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 0 }, // 356
    L { l: Str::Label(0x1000263), r: Str::Label(0x1000318), kl: 2, kr: 2, w: 0 }, // 357
    L { l: Str::Lit("Sharon Maher"), r: Str::Lit("U-TRAX Multi Media"), kl: 0, kr: 0, w: 0 }, // 358
    L { l: Str::Space, r: Str::Lit(" Localisations BV"), kl: 0, kr: 0, w: 0 }, // 359
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 360
    L { l: Str::Label(0x1000264), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 361
    L { l: Str::Lit("Danny Bilson"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 362
    L { l: Str::Lit("Paul DeMeo"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 363
    L { l: Str::Space, r: Str::Null, kl: 0, kr: 0, w: 1 }, // 364
    L { l: Str::Label(0x1000265), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 365
    L { l: Str::Lit("Danny Bilson"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 366
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 367
    L { l: Str::Label(0x1000266), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 368
    L { l: Str::Lit("Lisa Beard"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 369
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 370
    L { l: Str::Label(0x1000267), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 371
    L { l: Str::Lit("April Webster"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 372
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 373
    L { l: Str::Label(0x1000268), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 374
    L { l: Str::Lit("Carlos Agosti, C.A.S."), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 375
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 376
    L { l: Str::Label(0x1000269), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 377
    L { l: Str::Lit("Maxwell Caufield"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 378
    L { l: Str::Lit("Samantha Eggar"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 379
    L { l: Str::Lit("Kimberly Davies"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 380
    L { l: Str::Lit("Michael Ensign"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 381
    L { l: Str::Lit("Ian Abercrombie"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 382
    L { l: Str::Lit("Tamlyn Tomita"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 383
    L { l: Str::Lit("Lena Reno"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 384
    L { l: Str::Lit("Jeanne Mori"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 385
    L { l: Str::Lit("Gustavo Rex"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 386
    L { l: Str::Lit("Richard Whiten"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 387
    L { l: Str::Lit("Kai Wulff"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 388
    L { l: Str::Lit("Yoshio Be"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 389
    L { l: Str::Lit("Jean Gilpin"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 390
    L { l: Str::Lit("Vanessa Marshall"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 391
    L { l: Str::Lit("Gregg Berger"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 392
    L { l: Str::Lit("Grant Albrecht"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 393
    L { l: Str::Lit("Hiroshi Taguro"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 394
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 395
    L { l: Str::Label(0x100026a), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 396
    L { l: Str::Lit("David Pierce"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 397
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 398
    L { l: Str::Label(0x100026b), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 399
    L { l: Str::Lit("Renee Poirier"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 400
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 401
    L { l: Str::Label(0x100026c), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 402
    L { l: Str::Lit("Stefan Van Niekerk"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 403
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 404
    L { l: Str::Label(0x100026d), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 405
    L { l: Str::Lit("Dan Michelson"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 406
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 407
    L { l: Str::Label(0x100026e), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 408
    L { l: Str::Lit("Craig Young"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 409
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 410
    L { l: Str::Label(0x100026f), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 411
    L { l: Str::Lit("Sandro Licastro"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 412
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 413
    L { l: Str::Label(0x1000270), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 414
    L { l: Str::Lit("Robert Parent"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 415
    L { l: Str::Lit("Joel Krich"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 416
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 417
    L { l: Str::Label(0x1000271), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 418
    L { l: Str::Lit("Jason Camp"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 419
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 420
    L { l: Str::Label(0x1000272), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 421
    L { l: Str::Lit("Ben Guthrie"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 422
    L { l: Str::Lit("Dave Hobbins"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 423
    L { l: Str::Lit("Joel Pennington"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 424
    L { l: Str::Lit("Eric Perrier"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 425
    L { l: Str::Lit("Dan Vargas"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 426
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 427
    L { l: Str::Label(0x1000273), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 428
    L { l: Str::Lit("Ken Tsia"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 429
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 430
    L { l: Str::Label(0x1000274), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 431
    L { l: Str::Lit("Jeff Martens"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 432
    L { l: Str::Lit("John Mayhew"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 433
    L { l: Str::Lit("Vince Ng"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 434
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 435
    L { l: Str::Label(0x1000275), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 436
    L { l: Str::Lit("Donovan Stinson"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 437
    L { l: Str::Lit("Jennifer Lines"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 438
    L { l: Str::Lit("Akiva Maas"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 439
    L { l: Str::Lit("Jeffrey Fisher"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 440
    L { l: Str::Lit("Sarah Edmondson"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 441
    L { l: Str::Lit("Erin Karpluk"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 442
    L { l: Str::Lit("Trever Havixbeck"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 443
    L { l: Str::Lit("Darryl Scheelar"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 444
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 445
    L { l: Str::Label(0x1000276), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 446
    L { l: Str::Lit("Burke Trieschman, Open Door Productions"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 447
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 448
    L { l: Str::Label(0x1000277), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 449
    L { l: Str::Lit("SPG Studios"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 450
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 451
    L { l: Str::Label(0x1000278), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 452
    L { l: Str::Lit("Scott Gershin & Soundelux DMG"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 453
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 454
    L { l: Str::Label(0x1000279), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 455
    L { l: Str::Lit("1K Studios"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 456
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 457
    L { l: Str::Label(0x100027a), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 458
    L { l: Str::Lit("Dan Krech Productions"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 459
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 460
    L { l: Str::Label(0x100027b), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 461
    L { l: Str::Lit("Mondo Media"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 462
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 463
    L { l: Str::Lit("MGM Interactive Inc."), r: Str::Null, kl: 1, kr: 0, w: 1 }, // 464
    L { l: Str::Label(0x1000281), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 465
    L { l: Str::Lit("David Bishop"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 466
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 467
    L { l: Str::Label(0x100027c), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 468
    L { l: Str::Lit("Travis Rutherford"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 469
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 470
    L { l: Str::Label(0x100027d), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 471
    L { l: Str::Lit("Neil Haldar"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 472
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 473
    L { l: Str::Label(0x100027e), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 474
    L { l: Str::Lit("Jack Cheng"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 475
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 476
    L { l: Str::Label(0x100027f), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 477
    L { l: Str::Lit("Hugo Martinez"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 478
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 479
    L { l: Str::Label(0x1000287), r: Str::Null, kl: 1, kr: 0, w: 1 }, // 480
    L { l: Str::Label(0x1000288), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 481
    L { l: Str::Lit("Ron Sufrin"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 482
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 483
    L { l: Str::Label(0x1000289), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 484
    L { l: Str::Lit("MJ Miller"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 485
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 486
    L { l: Str::Label(0x100028a), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 487
    L { l: Str::Lit("Michael Moore"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 488
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 489
    L { l: Str::Label(0x100028b), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 490
    L { l: Str::Lit("Don Hulson"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 491
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 492
    L { l: Str::Label(0x1000283), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 493
    L { l: Str::Lit("Adam Scoll"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 494
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 495
    L { l: Str::Label(0x100028c), r: Str::Null, kl: 1, kr: 0, w: 1 }, // 496
    L { l: Str::Label(0x1000226), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 497
    L { l: Str::Lit("Simon Mathew"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 498
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 499
    L { l: Str::Label(0x2b4), r: Str::Null, kl: 2, kr: 0, w: 1 }, // 500
    L { l: Str::Lit("David Pope and Keith Snelgrove"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 501
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 502
    L { l: Str::Label(0x100028d), r: Str::Null, kl: 1, kr: 0, w: 1 }, // 503
    L { l: Str::Lit("Nikki Deen"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 504
    L { l: Str::Lit("John Muirhead"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 505
    L { l: Str::Lit("Graham Lenden"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 506
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 507
    L { l: Str::Label(0x100028e), r: Str::Null, kl: 1, kr: 0, w: 1 }, // 508
    L { l: Str::Lit("Liz Ramaswamy"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 509
    L { l: Str::Lit("Michael Jerchower"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 510
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 511
    L { l: Str::Label(0x100028f), r: Str::Null, kl: 1, kr: 0, w: 1 }, // 512
    L { l: Str::Lit("M. Neil Cummings"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 513
    L { l: Str::Lit("Jeremy Kenner"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 514
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 515
    L { l: Str::Label(0x2b4), r: Str::Null, kl: 1, kr: 0, w: 1 }, // 516
    L { l: Str::Lit("Bruce McMillan, John Batter, Nick Earl, "), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 517
    L { l: Str::Lit("John Riccitiello, Chip Lange, Frank Gibeau,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 518
    L { l: Str::Lit("Larry Castro, Don Mattrick, Jeff Karp,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 519
    L { l: Str::Lit("Steve Schnur, Mark Lange, Sam Hofer,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 520
    L { l: Str::Lit("Tom Raycove, Peter Miller, Eric Leblanc,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 521
    L { l: Str::Lit("Jessica Cecena, Jerry Newton, Eric Kornblum,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 522
    L { l: Str::Lit("Aimee McCrary, Michael Yasko, Mike Deir,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 523
    L { l: Str::Lit("Chris Espiritu, Kima Hayuk, Roger Metcalf,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 524
    L { l: Str::Lit("Rich Rogers, Erik Kraber, Michael Marsh,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 525
    L { l: Str::Lit("Cedric Bermond, Leonid Larionov,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 526
    L { l: Str::Lit("Alex Fennell, Craig Brown, Linda Matteson,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 527
    L { l: Str::Lit("Kathleen Tarnutzer, Phil Simon, Drew Huffman,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 528
    L { l: Str::Lit("Sean Murphy, Suzanne McNamee, Dan Hay,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 529
    L { l: Str::Lit("Tim Keon, Milly Ng, Robert Atkins, Levelord,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 530
    L { l: Str::Lit("Tom Mustaine, Steve Nix, Ritual Entertainment,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 531
    L { l: Str::Lit("Donny Miele, Joe Kucan, Curt Weintraub,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 532
    L { l: Str::Lit("Marilee Lear, Tiffany Baker, Rodney Munnings,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 533
    L { l: Str::Lit("Tracy Bohl, Lisa Hatch, Nicholas Savalas,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 534
    L { l: Str::Lit("Miché Bond, Tammi Cave, Eva LeTourneau,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 535
    L { l: Str::Lit("Elena Nekrassova, Carla Marrero, Erika Reuter,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 536
    L { l: Str::Lit("Lara Reynolds, Jennifer Young, Iain McCaig,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 537
    L { l: Str::Lit("Craig Mullens, Alan Sonneman, Philip Campbell,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 538
    L { l: Str::Lit("Bob Kurtez, Kurt Rauf, Gary Sauer,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 539
    L { l: Str::Lit("Karen Stephens, Nathalie Fernandez,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 540
    L { l: Str::Lit("Sandra Picaper, David Lapp, Roger Freddi,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 541
    L { l: Str::Lit("Andy Agostini, Dejan Stanisavlijevic,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 542
    L { l: Str::Lit("Alan Harrison, Gio Sasso, Stephen Rowe,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 543
    L { l: Str::Lit("MCL Motor Cars, Dr. Brad Remington and family,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 544
    L { l: Str::Lit("Finish Line Motorsports, Inc., Phillip Lim,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 545
    L { l: Str::Lit("Amanda Armer-Petrie, Harvey Resnick,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 546
    L { l: Str::Lit("Manu's Bronco, Rick Stringfellow, Phil Tiqui,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 547
    L { l: Str::Lit("Sean Letts, Tim Wilson, Matt Wynne,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 548
    L { l: Str::Lit("Justin Prazen, Roz, Larry Shapiro."), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 549
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 550
    L { l: Str::Label(0x1000290), r: Str::Null, kl: 1, kr: 0, w: 1 }, // 551
    L { l: Str::Lit("Hugh Binns, Mat Sneap, Dick Alton,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 552
    L { l: Str::Lit("Kev Holt, Andrew Govan, Tony Peach,"), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 553
    L { l: Str::Lit("Lloyd Baker."), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 554
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 555
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 556
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 557
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 558
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 559
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 560
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 561
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 562
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 563
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 564
    L { l: Str::Label(0x10002a1), r: Str::Null, kl: 0, kr: 0, w: 1 }, // 565
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 566
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 567
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 568
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 569
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 570
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 571
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 572
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 573
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 574
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 575
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 576
    L { l: Str::Space, r: Str::Space, kl: 0, kr: 0, w: 1 }, // 577
];

/// The label kinds' format prefixes the handler sends with msg 0x23 [M 0x2152b0:122-147; D ELF bytes
/// at 0x30d560/0x30d568/0x30d570, the sprintf tail 0x30d5b8]. Index = the entry's kind byte.
pub const KIND_FORMAT: [&[u8]; 3] = [b"\xff\x02\xfe\03\xfc\x64\xfb\x64", b"\xff\x01\xfe\x03", b"\xff\x03\xfe\x03"];
