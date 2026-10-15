# Subtitles: the cut-scene and FMV-briefing lines the Subtitles option switches

The AV options' Subtitles row (`CNAVOPT` control 0x10000136, `P_CNAVOPTIONS_Handler` 0x214818) writes the
profile byte `GameState+0x60` = **DAT_002a37c8** [M, docs/systems/screen-adjust.md's control table;
`Profile::gset.subtitles` in the port]. Three readers switch on it:

1. `Text_AddMsg` (0x1b0c30) — the message queue every subtitle rides;
2. `Sound_DoSubtitle` (0x1dde58) — the drone barks' spoken lines;
3. `ResetMap_Load` (0x1d6360) — whether the FMV briefing's subtitle ScriptPlayer is created at all.

Tags: [M] decomp / disassembly read here, [D] disc, [L] snapshot, [S] suggested. Port:
`crates/nf-game/src/subtitle.rs` (`SubtitleTrack`), `crates/nf-view/src/movie.rs::briefing_subtitles`,
the wiring in `drone.rs`'s `Request::Text` / `TextFlush` arms.

## The queue (`Text_AddMsg` 0x1b0c30, size 1448)

`Text_AddMsg(char player, u8 id, TXTMSG_TYPE type, char* text, u32 arg5, s16 time)` appends a `TXT_MSG`
node to the list at **0x31997c** (nodes allocated from the DList at 0x319970, 32 at a time; strings from
the 0x80-slot CirBuf at 0x2bf140 [M `Text_Init` 0x1b1428]):

| node | field |
|---|---|
| +0x08 | the (word-wrapped) text pointer |
| +0x10 | arg5 (passed through) |
| +0x14 | s16 text length |
| +0x18 | f32 time remaining (ticks) |
| +0x1c | type (0 = free node) |
| +0x1d | id (a type-4 add with id 0xff stores 0) |
| +0x1e | player |

**The flag gate** [M 0x1b0c30, disasm-checked]: an add passes only when `player < 4` AND
`((type == 4 && id == 0xff) || DAT_002a37c8 != 0 || (unsigned)(type − 4) > 1)` — types 0–3 and 6 always
pass, **types 4/5 (the subtitles) only with the option on**, except the id-0xff bypass that
`Sound_DoSubtitle` uses after its own gate (below).

**Type 4 → 5 retype** [M 0x1b0c30]: a type-4 add while the current movie has finished
(`psiMovieFinished` 0x1082e8 != 0, and `DAT_002d88c0+0x29 == 0`) is stored as type 5 — the remaining
briefing lines continue on the 2-line presenter after the movie is gone.

**Duration** [M disasm 0x1b104c-0x1b10ac]: `time < 1` → stored as-is (a 0-time head is deleted by the
next `Text_UpdateMsg`, so it never displays); otherwise `time × len/origLen + 0.5` (the word-wrap part
ratio — with one part exactly `time`), and the `max(60, …)` clamp at 0x1b1060 applies **only to types
outside 4/5** (`bnel v0,zero,001b10b4` at 0x1b1050 skips it for the subtitle types). A line wider than
the format's width is split into consecutive parts, each with its ratio's share of the time
[the wrap loop at 0x1b0e80].

**Timing** [M `Text_UpdateMsg` 0x1b11d8]: per tick the displayed head's +0x18 decrements by
`FRAME_RATE_MUL` (1.0 [L]); at ≤ 0 the head is deleted (`Text_DelMsg` 0x1b1640) and the next queued
node of the same (player, type) becomes the head. Queued nodes' timers do not run — a line displays
from `max(push_tick, previous end)`. `Text_FlushAllSubtitles` (0x1b15c8) — `Script_FFwd`'s skip arm
(0x1dac78:28-29) — deletes every type-4/5 node and hides the sprites.

## Who draws them

`Game_Run` (0x1c9790:20-24) picks the presenter per tick [M]:

```c
if (ScriptCam == 0 && psiMovieFinished()) Text_Update2Line();   // type 5, two lines
else                                       Text_Update();      // type 4, one line
```

**Type 4 — `Text_Update` (0x1b1510)** [M]: `Text_UpdateMsg(-1, 4, &SubTitleMsg@0x30cd98)`; when a head
exists, `Sprite_SetText(SubTitleSpr, head->text, …)` (0x1de698 — the third argument is unused by the
callee, disasm 0x1de698-0x1de714: only a0/a1 are read), draw layer `+0x38 = 0` (0xff = hidden when no
head; the field is the draw layer, docs/fields' HUD sprite note), `+0x39` = the highest active viewer
index (5 down to 0, the `glb_viewer` walk). `SubTitleSpr` is `Sprite_Create2(&SubtitlesInfo@0x319940)`
[M `Text_Init` 0x1b1428] — the template sits in .sbss and is zero at rest [L symtab, section 15], so the
sprite's own fmt/geometry come from whatever the font load wrote there before the first line; the port
models the track only (the main agent draws).

**Type 5 — `Text_Update2Line` (0x1b09f0)** [M]: the head's text wraps at `0.6 × TextMsgFormats[5].width`
(= 276 px) into `SubTitleStr` and the remainder at 460 px into `SubTitleStr2`; the two sprites are
`Sprite_SetParams(spr, 0, 256, 402, 0, 0x7d6d59ff)` / `(spr, 0, 256, 422, 0, 0x7d6d59ff)` — centred at
x 256, y 402/422, colour **0x7d6d59ff** (the game's gold 0x7D6D59, alpha ff), fmt 0x30cdb0 (the centre-
justify fmt, docs/systems/mission.md's banner note: `Font_DrawText` flag 2 → x −= w/2).

**The format table** [L ACTION.ELF .data 0x2bf108, `TextMsgFormats`, 8 bytes per type: `FMT* fmt; u16
width`]: entry 1/2 {0x30c6b0, 480}, entry 3 {0x30c6b8, 480}, **entry 4 (type 4) {0x30c6b0, 480}**,
**entry 5 (type 5) {0x30c6b0, 460}**, entry 6 {0x30c6b0, 256}. The two fmt pointers are font records
whose name field reads "cdorm0" [L bytes at 0x30c6b0+0x18].

## Source 1: the NIS / script-player cut-scene lines

`Script_TextStart` (0x1db2b8) — script op **0x1e**, command `op | u32 label | u16 time | trail` (8
bytes, little-endian fields) [M; the lens in nf_formats::nis]:

```c
Text_AddMsg(-1 /*player*/, 0 /*id*/, 4 /*type*/, Txt_BindLabel(label, 0), 0, time);
```

The label binds through the level's Lang table (`Txt_BindLabel` 0x1cad60). The game stores the bound
string; the port carries the raw label and binds at draw/hud-drain time (the `drones.taken` pattern).
A script can fire several 0x1e ops on one tick — the FIFO chains them.

Port: the runtime raises `nis::Request::Text`, `Drones` pushes it into **`subtitle_track`**
(`crate::subtitle::SubtitleTrack`) at the script tick and, when the flag is on, into `script_msgs`
(the HUD drain — the same add the game stores once [M]).

## Source 2: the FMV briefing intro lines

`ResetMap_Load` (0x1d6360) phase 0 [M; content measured in docs/systems/levels.md §5.4]: loads the
global pack `07000500.bin` (FILES.BIN), then — only when `DAT_002a37c8 != 0` — looks the movie code up
in the compiled chain (0x1d6360:40-99, no jump table [L]) and `SP_Create(0x2d6200, 0x2d6200, script, 0,
1, 1, 0, 0)` — a ScriptPlayer playing script A at once (mode 1) while `psiStartBackgroundMovie` plays
the briefing, so the script's tick axis is the movie's. The 14 scripts are single streams of op-0x1e
commands [D]:

| movie code | script | movie code | script |
|---|---|---|---|
| 0x7100001 | 0x0600095A | 0x7300004 | 0x0600095D |
| 0x7100005 | 0x06000961 | 0x7300008 | 0x06000962 |
| 0x7100009 | 0x06000963 | 0x730000b | 0x06000964 |
| 0x710000c | 0x06000965 | 0x730000d | 0x06000966 |
| 0x7100011 | 0x06000967 | 0x7300013 | 0x06000968 |
| 0x7100014 | 0x06000969 | 0x7300017 | 0x0600097A |
| 0x7100018 | 0x0600096B | 0x730001b | 0x0600096C |

Movie codes map 1:1 from the level codes 0x070000xx per `ResetMap_LevelToLoad` (0x1d3800), except
0x0700001b → 0x7100018 and 0x0700004a → 0x710004a, which has **no row** (no intro subtitles) [M].
Port: `nf_game::subtitle::intro_script_for` (the table) and `nf_view::movie::briefing_subtitles(iso,
code, enabled)` — reads the pack, parses the script, walks its Wait + Text commands on `Script_Run`'s
clock (a command after `Wait(t)` fires on tick `t + 1`: `Script_Update` 0x1da998 runs the stream with
the pre-increment clock, then adds `FRAME_RATE_MUL`), and returns the `SubtitleTrack`. Measured on the
disc: 0x06000961's four lines start at ticks 184 / 1340 / 2113 / 3570 with labels 0x06000068-6b and
times 1132 / 756 / 988 / 555 [D] (pinned by `subtitles_live::briefing_track_is_pinned_to_the_disc_script`).

## Source 3: the spoken lines (`Sound_DoSubtitle` 0x1dde58) — not ported

`Sound_UpdateSounds` (0x1dda70:98) calls `Sound_DoSubtitle(sound_id)` when a voiced sound plays [M]. It
scans the `Snd2Lbl` table (0x1e4 entries of `{u32 label | 0x80000000, u32 sound_id}`), gates on the
flag **unless the entry carries the 0x80000000 "always show" bit**, binds `label & 0x7fffffff`, and adds
`Text_AddMsg(-1, 0xff, 4, text, 0, g_fSubtitleDurationModification × SFXOutputData[id].f10 × 60)` — the
id-0xff bypass above; the duration is the sound's length seconds × 60 ticks scaled by
`g_fSubtitleDurationModification` (0x30d084) [M]. The port's drones do not render bark text; the flag
read and the table are documented here for whoever ports it.

## Port status

* `crates/nf-game/src/subtitle.rs`: `SubtitleTrack { enabled, lines }` — the queue as an absolute
  timeline (`SubtitleLine { label, start, end }`), `push` (the flag gate, the verbatim type-4/5
  duration, the FIFO chain), `active_subtitle(t) -> Option<label>` (the per-tick query the draw code
  calls), `flush` (the skip's `Text_FlushAllSubtitles`), `from_script` (the briefing walk).
  6 unit tests on the queue semantics.
* `Drones::subtitle_track` + the `Request::Text` / `TextFlush` arms: the track fills at the script
  tick; `script_msgs` and the HUD drain only when the flag is on (the game's AddMsg gate). The host
  wires `subtitle_track.enabled` from `Profile::gset.subtitles` / the AV row's control 0x10000136 —
  the fresh profile's default is Off [M profile.rs], so nothing shows until it is wired.
* `nf_view::movie::briefing_subtitles` for the FMV kind.
* Tests: `nf-view/tests/subtitles_live.rs` (4, ISO-gated) — the disc-pinned 0x06000961 timeline, the
  no-row codes, the Off gate through the real Sim, the ON queue at the right tick. 
  `fix_script_commands_live::text_command_queues_the_subtitle_label` now pins the ON behaviour
  explicitly (it sets the flag).
* Not ported: the spoken-line barks (source 3), the word-wrap part split (a text-layout concern the
  draw code owns — the track stores labels), and the type-4→5 retype's presenter difference (both
  types ride the same track; the main agent's draw code chooses the presenter the way `Game_Run` does:
  `ScriptCam != 0 || movie playing` → 1-line, else 2-line).
