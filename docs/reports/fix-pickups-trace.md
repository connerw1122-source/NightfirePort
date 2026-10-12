# fix-pickups-trace — the Raptor drops, against the real game's trace

Branch `s/fix-pickups-trace` (from master 9aac84e). Everything below was driven through the real
path (`nf_view::load` + `Sim::step_bond`, NF_ISO) and checked against the PCSX2 trace
`tests/traces/01-raptor.trace` (07000001, god + the recorder's infinite ammo).

## What the trace says, and what the port does now

| trace | port after this task |
|---|---|
| 6.0 s: weapon **0x0E** becomes owned with clip 9 (`weap 52710 e clip 0->9 owned 0->1`) | identical: the first Raptor-guard drop is a kind-0 pickup of **0x0E** (the Raptor Magnum); taking it sets owned[0xE], clip 9, and adds **nothing** to the spare |
| no spare type-3 event at 6.0 s | the test asserts `carried[3] == 0` right after the first take |
| +6 (19.8 s), +7 (71.0 s), +7 (74.3 s), +5 (76.3 s) of ammo type 3 | the walkthrough takes 5 Raptor drops: `Weapon{0xE}` then `Ammo{0xE, 6}, Ammo{0xE, 7}, Ammo{0xE, 4}, Ammo{0xE, 7}` — every round count inside the game's window 4..8 |
| every drop the user walked over was taken | every drop taken; nothing left on the floor (asserted) |
| HUD armour 50.0, spare values are real takes only | god writes armour 50.0; spares only ever move by takes |

## The 0x0E chain (task item 1) — measured, and the port already matched it

- The guards carry the Raptor via the placement's **param 9 = 0xE** [D 07000001.bin: 5 guards, skins
  0x05000036/39/3a] → `NDrone2_DefaultInit` stores it in `anim+0x62`.
- The drop's pickup id is `weapon_data[w]+0x02` = 0x0E (self-base) [M `DroneWeap_DropWeapon`
  0x17a2a0:96-98]; the port's `drone.rs::drop_weapon` does exactly this (`weapons.base(w)`).
- `Upgrade_Weapon(0x0E)` = 0x0E [M 0x1a8f30 — 0x0E is in no remap table], so the take equips 0x0E.
- `weapon_data[0x0E]`: ammo type **3** (+0x90, ".357 Magnum rounds", cap 36), clip **9** (+0x92),
  selectable, flags68 0x1000, third-person model 0x020004b2 [D ACTION.ELF `weapon_data` — read back
  by running `_GLOBAL_$I$weapon_data`, the table cross-checked against PCSX2 (WORKLOG 0017)].
- The HUD name binds the RAW id's label (+0x38 = 0x0500002D) = **"Raptor Magnum"** [D USATxt.dat;
  the take-message rule is [M 0x18e650:246]] — asserted in the test.
- The old "Raptor = weapon 6" naming in the WORKLOG was wrong: weapon 6 is the **Wolfram P2K**
  (type 2, clip 16, label 0x05000036). Nothing in the port needed to change for the id/clip/ammo
  chain — test 1 (`the_first_raptor_drop_is_weapon_0x0e_clip_9_and_the_hud_names_it`) pins it so it
  cannot regress.

## The user's bug (task item 2) — reproduced, then fixed

**Symptom**: with the play cheats (god + infinite ammo — the trace session's own setup), the first
Raptor drop equips but every later drop refuses forever ("some later drops can't be picked up").

**Reproduction** (before the fix; shown by stashing the one-file fix):
`NF_PICKUPLOG=1` on the pre-fix build prints the first take
(`weapon 0xe kind 0 rounds 4 → Some(Weapon { weapon: 14 })`) and then the second drop refusing every
tick (`weapon 0xe kind 0 rounds 6 → None`, 100+ lines), and the new test
`with_dev_god_and_infinite_ammo_every_raptor_drop_is_taken_and_the_spare_matches_the_trace` panics
at "drop 7 (weapon 0xe) is taken — the user's symptom is a drop that cannot be picked up".

**Cause** [M `nf-view/src/dev.rs` `Dev::after_tick`]: the dev infinite-ammo refill wrote
`carried[i] = max(carried[i], 200)` — above every ammo cap (type 3 caps at 36 [D `ammo_data`]) — so
`Player_EquipAmmo`'s cap check refused every owned-weapon drop. It also topped **unowned** clips to
their clip size, so the first take's `Player_EquipWeapon` (0x1a8a58) saw `old = clip[0xE] = 9` and
pushed the whole drop's rounds into the spare instead of none (the game adds only the rounds beyond
a from-zero clip).

**What the game side actually is**: the session's "infinite ammo" was the **recorder's cheat**
(`tools/trace/trace_record.py --ammo` / `cheats.py`): every 30 frames, each **owned** weapon's clip
(BL+0x1B8+w·0xC) and each spare type (BL+0x170+a·2) topped back to the **most seen this run**, the
best only moving up, both tables reset when BLData changes (a level load). Spares therefore never
exceed a take → no cap ever refuses (trace: spare type 3 goes 0→6→13→20→25 against cap 36), and an
unowned gun's clip stays 0 → the first take adds no spare.

**Fix** (`crates/nf-view/src/dev.rs`): the infinite-ammo branch now ports the recorder's cheat
exactly — every 30th tick, owned weapons' clips and all spare types topped to `best_*` (max-seen),
tables reset on `Sim::level` change; the `max(200)` line is gone. God's armour write corrected to
50.0 (cheats.py `F50`; the trace's armour column reads 50.0). After the fix the same test runs the
full 5-guard walkthrough green.

## The drop rounds formula (task item 3) — checked

`rounds = (short)(int)(clip_size(w) · f)`, `f = Rand_FRand(0.5) + 0.5` when `clip_size(w) < 10`,
else `+ 0.25` [M 0x17a2a0:88-98]. For the Raptor guards w = 0x0E, clip 9 < 10 → factor +0.5 →
rounds ∈ 4..8. The port computes `mul(clip as f32, add(rng.frand(0.5), factor)) as i32 as i16` in
`drop_weapon` — same factor branch, same single `Rand_FRand(0.5)` draw per drop drawn before
`Pickup_CreateSimple` (the 0x1A grenade-launcher twin's second draw comes after, :103), same
truncation. The four trace deltas (+6/+7/+7/+5) all sit in 4..8, and the test asserts every observed
count is inside that window — a port using weapon 6's clip 16 (8..15) or the ≥ 10 branch could not
produce 4..6.

## Tests and verification

- `crates/nf-view/tests/fix_pickups_trace_live.rs` (new, NF_ISO, real input path):
  1. `the_first_raptor_drop_is_weapon_0x0e_clip_9_and_the_hud_names_it` — kills a Raptor guard
     through the fire path, walks onto the drop: `Taken::Weapon{0xE}`, owned, clip 9, spare type 3
     still 0, HUD message "Raptor Magnum".
  2. `with_dev_god_and_infinite_ammo_every_raptor_drop_is_taken_and_the_spare_matches_the_trace` —
     under god + the recorder's infinite ammo: 5 Raptor guards killed (a switch-gated one enables
     via its own gate channel — the stand-in for the walking that turns the level's channels on),
     every drop taken, takes = Weapon{0xE} + Ammo{0xE} × 4 with rounds ∈ 4..8, spare = the sum of
     the takes only (≤ cap), nothing left on the floor. **Fails on the pre-fix port** at the first
     refused drop (proven by stashing the fix).
- Updated for the new cheat shape (minimal, noted in WORKLOG): `dev_tools_live.rs` — the
  "every clip slot topped" assertion now filters to OWNED weapons (the recorder cheat leaves
  unowned slots 0; the switch cycle only offers owned weapons); `menu_live.rs` — the toggle test
  seeds the best-seen tables at the full clip before draining.
- Full suite: `NF_ISO=… cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio
  -p nf-view -- --test-threads=1` (see the WORKLOG entry for the run's result on the slim ISO).

## Left open (not ported, with the game-side reason)

- The trace's non-owned clip creep (`weap` lines: 0x33/0x45/0x4C/0x4F clips creeping toward 100
  during the run): neither cheat tool writes those (both gate on the owned byte), so the writer is
  some other game code; no play symptom hangs on it, so it stays unexplained.
- The recorder tops on the game's global frame counter (`GameState+0x34`); the port uses its
  per-level tick counter — the cadence (30 frames) matches, the phase differs, which is
  unobservable in play.
