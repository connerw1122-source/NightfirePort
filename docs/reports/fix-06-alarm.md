# fix-06-alarm — the 07000006 searchlight alarm now sounds

Branch `s/fix-06-alarm` (from master 7785582). Session 16:02– UTC 2026-10-14.

## Done

1. **The courtyard's alarm siren sounds.** The user (playtest 1013-1825) walks into a 06 searchlight
   beam, the guards react — and nothing goes off. The guards' half was 045l's work (alarm channel 55
   [M Searchlight_Update 0x1cb900:142-144] → the guards' second switch); the *sound* half was
   missing entirely: the game's **SoundTrigger objects** (`SoundTrigger_Create` 0x1b2628,
   `SoundTrigger_Update` 0x1b2790, placement kind 0xF9 `oSoundTrigger`) had never been ported. The
   disc gives 07000006 four of them [D 07000006 placements 665/702/703/712, `nf-extract placements
   … f9`], and **placement 702 is the alarm**: ON channel **55** (the searchlights' alarm write),
   sound id **329 = 0x149 — LOOPED, inner 400 / outer 500** [D SfxTable, `nf-extract sfx 149`],
   2-D (`Sound_Play`, the global klaxon), OFF channel 177, one-shot. The other three are one-shot
   stingers (ch 47 → 219 and ch 57 → 241 3-D, ch 103 → 331 2-D) — same object type, ported with it.
2. **The param layout, decoded from the disc.** `SoundTrigger_Create`'s decompile reads the
   placement record at fixed offsets (+0x2c, +0x30, +0x34, +0x38, +0x3c, +0x40, +0x48) — those are
   the params read **sequentially, param k at +0x2c+4k**: p0 = ON channel, p1 = one-shot flag,
   p2 = sound id (0/0xffff → no object; the ON channel must also be non-zero), p3 = 2-D flag,
   p4 = OFF channel, p5 = the `GlobalVoiceHandle` claim, and the keep-when-ON-clears byte is the
   record's +0x48 low byte — the placement's **param count** (all four 06 triggers carry 8 params →
   disabled). The layout is pinned by the searchlights' own placements [D placements 2/4/6:
   on/off/alarm = params 1/2/3 = 64/57/55, the channels 045l proved by behaviour]. The
   `extra[8]` word (+0x44) is the difficulty gate: skip when below the difficulty, or 5/6/7 without
   difficulty 1/2/3 [M 0x1b2628:24-40; the gate value is 0 on all four 06 triggers].
3. **The update loop is operation-for-operation** [M 0x1b2790]: idle until the ON channel rises;
   start (2-D `Sound_Play` or 3-D `Sound_Play3D` at the placement); stop on the OFF channel or when
   the ON channel clears and the keep byte is 0; a finished one-shot dies; the single
   `GlobalVoiceHandle` slot is claimed on start, freed on stop/finish, and a global trigger that
   finds it busy **kills itself**; kills are `obj+0xfe` bit 0, which the object handler turns into
   `control_delete_object` [M 0x1336f8:72-89] — the port drops them from the list.
4. **The alarm music half was already ported and is now verified live**: ch 55 feeds
   `DroneFunc_CheckAlarmRaised` 0x1453a8's `Music_Event(2,1)` stream (the port's CheckAlarmRaised
   block, drone.rs), whose `UpdateMFX_01CastleB` 0x113060 hysteresis demands a combat section
   ~12 ticks in — the new test asserts the combat Jump on the real command board. The wallhug
   freeze's `Get_Music_Event(2)` reader (`SP_Update` 0x1dbda8) is the port's `prev_event2`
   (drone.rs:7820) — untouched.

## How it was verified

- `crates/nf-view/tests/fix_06_alarm_live.rs` (NF_ISO, the real `nf_view::load` + `Sim::step_bond`
  path), **fail-first** — on pre-fix master `stepping_into_a_beam_sounds_the_alarm_siren` failed
  exactly at the siren probe (ch 55 rose, sound 329 never played):
  - `stepping_into_a_beam_sounds_the_alarm_siren`: at the 06-searchlight beam spot (4.8, 42.09,
    −16.0) ch 55 rises and the siren (id 329) is active within ≤ 2 ticks (the same-frame object
    walk + the deferred start [M 0x1b2790; `Sound_Play` is node-only, the start rides the next
    `Sound_UpdateSounds`]); it **holds** across 300 more ticks (looped, OFF ch 177 never rises
    here); the music board then carries a combat-section `Jump` (the CheckAlarmRaised → CastleB
    chain). Sisters inside the test: no siren and ch 55 low before the beam.
  - `outside_the_beam_nothing_sounds`: 500 ticks at the 06-firstsight corridor spot (83.58, 35.13,
    33.21) — ch 55 low, no siren node, no combat jump.
- Neighbours, foreground chunks, all green on the fixed tree: `nf-game --lib` 150/150 (the
  `sound.rs` unit tests), `fix_drone3_live` 5/5, `fix_searchlight_alarm_live` 2/2 (045l's tests
  unchanged), `spawner_kill_live` 5 passed + 1 pre-existing ignore (its own note: test setup, the
  fix's assertions pass), `fix_npcs_live` 4/4.
- Full suite (`tools/test/suite.py`, slim ISO, the final tree — master was already current) on
  2026-10-14 16:41–17:16 UTC: **1053 passed, 4 failed, 33 ignored (35 min)**. The 4: the three
  allowed `disc_golden` tests, plus `TIMEOUT weapon_matrix_live after 600 s (last started:
  matrix_07000015)` — the weapon-matrix2 runner's all-27-level sweep exceeds the suite's 600 s
  per-binary cap on this sandbox. It is pre-existing, not this task's: the same binary timed out
  the same way on today's pre-fix baseline run (16:37, `matrix_07000014` — the same progress
  point), and this task adds only a 4-entry list ticked once per frame.
- NF_SILENT stays respected: the tests read the EE-side node pool (`Sounds::active_ids`) and the
  music command board — never the driver's output; no new draws were added anywhere.

## Shared files touched (small, additive)

- `crates/nf-game/src/sound.rs` — the `SoundTrigger`/`SoundTriggers` type (create + tick), additive.
- `crates/nf-game/src/drone.rs` — three additive hooks, listed: the `sound_triggers` field (own
  line, merge-lesson 4), `SoundTriggers::create(&main_map.placements, self.difficulty)` next to the
  `map_sounds` init, and `sound_triggers.tick(&mut self.audio, &self.channels)` next to
  `sss.tick`. No existing lines changed.

## What's left / live checks for the main agent

1. **Hear it in PCSX2** (no GPU/audio here). Walk into a courtyard beam on 06 and confirm: the
   klaxon loops while the beam holds you, the music leaves stealth ~a second in, and the klaxon
   stops when the alarm ends. The siren's OFF channel 177 has exactly one disc writer: **touch
   trigger placement 765**, box (32.47, 42.22, −59.18)–(38.30, 46.79, −48.99) — the ground past the
   courtyard [D 07000006 trigger 765, `nf-extract triggers`]. So the game's klaxon should stop as
   Bond climbs out of the courtyard into that volume; if it instead stops with the beam, the
   177-beat divergence is the thing to re-record.
2. **The other three triggers** should be audible at their own beats: the searchlight light-up
   clunk (ch 57 → sound 241, 3-D at 28.66, 40.41, −18.80 — once per level load), ch 47 → 219, and
   ch 103 → 331 (2-D). If any never plays in-game, its ON channel's writer is the next port item —
   the trigger side is done.
3. **Global-voice interaction**: a global sound trigger dies if another is already playing
   [M 0x1b2790:19-28]. 06's four triggers carry no global flag, so nothing should bite here — but
   later levels (e.g. Bond's radio lines as global triggers) will exercise the slot; worth a spot
   check whenever a level with a `global` sound trigger loads.
4. Render spots (045l's list stands): `--bond 4.8,42.1,-16.0,-2.2,0` (the beam + the guards'
   reaction) — with this fix the same spot should now also sound the siren in a PCSX2 run with
   audio.

## Doc updates

- `docs/WORKLOG.md` — this task's entry (appended).
