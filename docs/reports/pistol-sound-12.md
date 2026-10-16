# pistol-sound-12 — the starting pistol on 07000012 sounds right

Playtest 1012-1723 (07000012): "the pistol you start with does not sound right". **Result: the
sound-id → bank resolution is already correct** — every id the pistol requests resolves from the
bank the game would use, with that bank's own effect record and sample bytes (tests below). What
the sweep *did* find: one wrong measurement in the fix-weapon-sounds report (corrected), a misread
port comment on `weapon_def+0x60`, and one remaining game-vs-port deviation on the shot — the
missing op-1 pitch jitter — listed under "what's left" with live checks for the main agent.

## The chain, verified link by link

- **The kit** [M `Player_InitWeapon` 0x1a7ef8, case 0x7000012]: a cold 07000012 start equips
  weapon **0x10** with 7 rounds (no RamLoad on a fresh start — the `+0x27a` gate). Weapon 0x10 is
  the Magnum family's tower sibling: it shares the Raptor Magnum's anim scripts (fire 0x06000490,
  reload 0x06000495) but has its own record — first-person model 0x0500003f, ammo 0x0b, fire sfx
  (+0x60) = **0x0603** vs 0x0e's 0x1e1 [L ee_snapshot.bin @ 0x2bf150 + 0x10*0x10c, all fields
  dumped]. The port's cold start was asserted live: weapon 0x10, clip 7
  (`crates/nf-view/tests/pistol_sound_12_live.rs`, `level12s_own_bank_and_the_pistol_scripts_ground_truth`).
- **The bank** [M `Sound_Ready` 0x1dceb8:59-61]: `case 0x7000012: SFXLoadSoundBank(0x1f)` — level
  12's own bank is hash 0x1f = SBINFO **slot 22** [D docs/formats/sound-bank.md slot table]. The
  port's `nf_game::sound::level_bank(0x0700_0012)` → 0x1f matches, and the driver's primary on
  level 12 *is* slot 22 (asserted through `Driver::item_slot` in the repro test).
- **The ids** [D the test's own sweep over every parsed disc bank]: fire script 0x06000490 =
  `op1@1=0x1e1`; reload script 0x06000495 = `op1@36=0x1e2, op1@85=0x1e3` (the dumps are asserted).
  The disc holds 0x1e1/0x1e2/0x1e3 in slots 2,10–13,16,**21–24** — the own bank 22 carries all
  three, so the pistol needs no spare bank on its own level.
- **The remap** [M `AnimProcessScriptCmds` 0x1c1b38:270-273]: an op-1 id of 0x1e1 is replaced with
  **0x602** while the current weapon (+0xdc→+0x62) is 0x10. The tower pistol's audible shot IS the
  own bank's 0x602; the raw 0x1e1 must never sound while 0x10 is current (asserted). 0x602 is also
  in slot 22 — see the correction below.
- **The bullet-spawn layer** [M 0x1c1b38 case 7:417-436]: an op 7 plays `weapon_def+0x60` through
  `Bullet_init` [M 0x12c5a0:269-274; `param_7 == -1` = silent]. Weapon 0x10's +0x60 = 0x0603
  [L ee_snapshot]. But the player's fire script carries **no op 7** [D the dump], and the sim-side
  player spawn passes 0xffff = short −1 = silent [M 0x1a4bf0:288; 0x12c5a0:5 `short param_7`] — so
  0x603 rides only the bot/world-anim paths (bots play it with `Rand(500)−250` pitch [M
  0x178f68:277]). `+0x60`'s only two consumers play it as a sound; the port's `weapon.rs:297-300`
  comment ("the view-anim datum id … NOT a sound id") is a misread — `Player_MuzzleFlash` consumes
  +0x4C and `AnimObjectDraw` +0x80, never +0x60 [M 0x1a4298:141, 0x1c4490:258]. No lib code was
  changed for this (the accessor is unused on the sound side today); the comment should be
  corrected by a task that touches weapon.rs.

## The correction to fix-weapon-sounds.md

Its table row "remap targets / hardcoded … | 2 everywhere" claims the 0x10 remap target 0x602 is
held by slot 2 only. **The disc says otherwise** [D this test's sweep, `slots_with` over every
parsed bank]: 0x602 and 0x603 are held by slots 2,10–13,16,21–24 — the tower banks included. On
07000012 the own-bank-first rule therefore resolves the pistol's shot from the own bank, not from
the MP spare. (The row's other entries were not re-measured here.)

## The tests (all green, `crates/nf-view/tests/pistol_sound_12_live.rs`)

Through the real path (`nf_view::load` → `Sim::step_bond` fire/reload input → the view-script
tick's op-1 → the `Sounds` pool → `nf_view::sound_tick` → the driver):

1. `level12s_own_bank_and_the_pistol_scripts_ground_truth` — own == 22 [M Sound_Ready]; the
   reload/raw-fire ids and the remap target + record id are all in the own bank [D]; the cold kit
   is weapon 0x10/clip 7 [M 0x1a7ef8]; the fire/reload script op-1 dumps are pinned.
2. `the_start_pistols_sounds_come_from_the_game_owned_banks` — THE repro: firing the start pistol
   starts **0x602** from slot 22 with the bank's own decode (`Driver::played_sample_hash` ∈ the
   bank's FNV set), reloading starts **0x1e2/0x1e3** from slot 22 likewise, and the raw **0x1e1
   never starts** (the remap holds).
3. `a_different_weapons_id_does_not_match_the_pistols_sample` — the sister: driver-level starts of
   0x602, 0x1e1 and the P2K's 0x403 each resolve from their rule-resolved bank with that bank's
   decode, and the three sample hashes are pairwise distinct — the assertion genuinely
   discriminates a wrong-id/wrong-bank take. (Side fact: 0x403 is NOT in the tower banks [D] — a
   P2K given on 12 sounds through the spare MP bank by design.)

## Suite

`cargo test --offline --release` over nf-game/nf-formats/nf-audio/nf-view, run per-binary with
`--test-threads=1` (foreground chunks — background processes do not survive between tool calls in
this sandbox): **413/415 binaries, 1756 passed, 7 failed, 72 ignored**; 2 tail binaries unreached
in the window. The 7 failures are exactly the known allowed set, none regression: `probe_pylon_
render` + `fix_0c_pylon_live`×2 (wgpu: no GPU adapter in the sandbox), `driving_ai_follow`
(pre-existing), `disc_golden`×3 (slim ISO). The new test binary passed in-suite as well. Ran
before the final push on this tree (= origin/master + this task's commits).

## What's left (for the driver / a follow-up)

The resolution being clean, the best-supported cause of "does not sound right" is the **missing
op-1 pitch jitter**: the game randomises every op-1 sound's pitch by `Rand(500)−250`
[docs/systems/anim-engine.md op 1; the bot fire call shows the same shape, M 0x178f68:277], while
the port's op-1 site (`nf-view/src/scene.rs:3348-3353`) passes no pitch, so every shot is
identical. Fixing it means drawing from the shared RNG per shot — a determinism-stream change that
needs its own task and a bit-exactness review (the game draws it too, so the port's silence on
that draw is itself a divergence).

## Live checks for the main agent

1. **The shot's pitch jitter** — on 07000012, fire the start pistol and watch `Sound_Play3D`
   (0x1dd548) calls with id 0x602: read the pitch argument each shot. Expected: it varies in
   −250..+249 across shots (the `Rand(500)−250` draw). If constant 0, the jitter lives elsewhere.
2. **RNG stream** — watch the shared RNG draw count across one shot: expected +1 draw per op-1
   sound (the pitch). This settles whether a port fix must draw from the shared stream.
3. **0x603 on a player shot** — expected: never plays for the player (no op 7 in the fire script,
   the sim spawn is silent-short −1); it should appear only for NPC magnum fire.
4. **The 0x602 take** — capture the console's 0x602 audio on 12 and compare against the port's
   bank-22 decode (FNV of channel 0 in `played_sample_hash`): expected identical; a mismatch would
   point at a VAG-decode edge in slot 22's sample, not at resolution.

## ntfy

(not attempted yet at write time — sent right after this commit; any failure text lands here.)
