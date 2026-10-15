# fix-pickups-ammo — weapon and ammo pickups: caps, dropped guns, sounds

Branch `s/fix-pickups-ammo` from `origin/master` 4ea550b7. Every fix cites the doc entry and the
game address; every fix has an NF_ISO test through the real Sim path (`nf_view::load` +
`Sim::step_bond`, the same walk `Pickup_Update` 0x18e148 runs per tick) plus a sister assertion.
The disc-wide placement census (every on-foot level's 0xF0 params) drove the audit; it lives as an
uncommitted scratch example.

## Done

**1. The take sound for weapon/ammo pickups is forced 0xF5** — `Pickup_Handler`'s message switch
cases 0 and 1 (weapon and ammo, new and owned) both land on `LAB_0018ee80`:
`*(ushort*)(PICKUPINFO+0x2a) = 0xf5` before `Sound_Play3D` [M 0x18e650:290-300, :334-337]. The
placement's own `+0x2a` (what `Pickup_Create` stored, 0xFFFF when the word is 0 [M 0x18dda0]) is
NOT what plays for kinds 0/1. The disc places weapon/ammo pickups with other words: the 07000002
sniper 0x13F, the 07000008 rocket and the 0700000b/0d 0x2e piles 0x13F, the 07000001/05 id-6
boxes (0xF5 / 0x13F), the 0700000c/d 0x1f boxes 0xFFFF, the 07000008 0x0a boxes 0xFFFF [D the
census]. The port played `p.sound` — 0x13F, or nothing at all. Fixed in `drone.rs`
`update_pickups`: `Taken::Weapon | Taken::Ammo => Some(0xF5)` (kinds 3/6 keep their forced
0x141/0x14C, kinds 2/4/5 keep the pickup's own word, as the decompile does).

**2. The ammo path's id is the composition** `upgrade_ammo_id(upgrade_weapon(raw))` —
`Pickup_Handler` passes `uVar5 = Upgrade_Weapon(+0x24)` to `Player_EquipAmmo` for BOTH the owned
kind-0 arm and kind 1 [M 0x18e650:142-144, case 0 else-branch + case 1], and `Player_EquipAmmo`
then applies its own 0x1f/0x25 remap [M 0x1a8d18:11-19]. The port skipped the outer
`Upgrade_Weapon`. Visible on the disc: the id-6 ammo boxes (07000001 ×2, 07000005 ×2 [D]) —
`Upgrade_Weapon(6) = 2` [L UpgradedHandguns 0x2bee08 = [2,4,6,8]], and record 2's ammo type is 1
(cap 70) while record 6's is 2 (cap 90) [L weapon_data]. The old port credited the type-2 pool
(an invisible pool Bond's HUD never shows for the P2K) and refused at the wrong cap. The message's
ammo NAME stays the RAW id's type — the game reads `weapon_data[raw+0x24]+0x90` for the name while
the pool goes to the remapped id's type [M 0x18e650 case 1] — a game quirk, reproduced (scene.rs
already keyed the name off the raw id). Fixed in `pickup.rs take()`.

**3. `Pickup_Create`'s level-conditional rounds override** — `if (GameState+0x12 == 0x7000008 &&
id == 0x2c) rounds = 4` [M 0x18dda0, the non-set create path]. The vault's rocket placement says 5
[D the census]; the game creates 4. With clip_size(0x2c) = 4 and the type-14 cap 4 [L weapon_data
+ ammo_data], the take grants 4 loaded + 0 spare — the HUD's 4/4. The old port granted 4 loaded +
1 spare (4/5). New `Pickup::create_rounds_override(level, id, rounds)` in `pickup.rs`, applied at
the placement loaders and the script-reservation builder in `scene.rs` (the two `Pickup_Create`
callers the port has; `Pickup_CreateSimple` drops never carry 0x2c on 07000008 — no drone holds
the launcher — so the drop path needs no hook, noted below).

**4. A hidden pickup is still takeable** — `Pickup_Update`'s take path never reads `obj+0xF0` [M
0x18e148 — no +0xF0 access in the function; the respawn flow hides THROUGH state 2, which the
touch loop skips]. `Script_HideObj` 0x1db808 sets/clears exactly this bit for hide/show. The disc
case: `DroneWeap_DropWeapon` 0x17a2a0:110-112 creates the AR's 0x1B grenade pile with
`obj+0xF0 |= 0x10` — invisible for its whole 7200-tick life, and the game collects it walking
over the drop (the AR comes with its GL ammo). The port's `!p.hidden` gate refused it forever
(the pile despawned unseen). Fixed in `drone.rs`: the state-2 wait still gates the respawn-hidden
pickups; the hidden bit itself no longer blocks the touch.

**5. The kind-4 label rides the take** — case 4: `if (PICKUPINFO+0x34 != 0)
Txt_BindLabel(+0x34)` and the shared tail `Text_AddMsg`s it for 180 ticks [M 0x18e650 case 4 +
the message tail]. The label is the placement's param 6 (`Pickup_Create`'s param_13). The port
dropped the word entirely and never showed a message. Disc evidence: 0700000b's id-0x47 item
(switch 0x3f) carries label **0x2000036** [D the params]; 07000008's briefcase (switch 0x4A)
carries 0 → no message (unchanged). `Pickup` gained `label: u32` (own line, at the end),
`Taken::Label` carries it, and scene.rs binds it when non-zero.

## Not changed (checked, already right)

`Player_EquipWeapon`/`Player_EquipAmmo`'s caps (`ammo_data[t]+0x02`, parsed correctly), the
`+0x68 & 0x10` empty-clip fill and its slot rule, the clip→spare split on a new gun, the refusal
returns that leave the pickup on the floor, `Pickup_Update`'s touch predicate (radius `obj+0x8C`
+ 1.1 from the sphere centre to the player's position, LOS from the sphere centres, the lifetime
decrement only when not taken), the drop rounds formula `clip · (FRand(0.5)+0.5 | +0.25)` and the
7200-tick drop lifetime, the auto-switch rule (already live; a kind-0 0x1E take does not switch —
0x1E is not in BestWeapon). `Upgrade_Weapon`'s 0x52 special (clears `BLData+0x59e`, rewrites the
held 0x53 → 0x52 [M 0x1a8a58:80-85]) is NOT modelled: `+0x59e` is write-only in the decompile
export and no disc path equips 0x52 as a new weapon except the two 0700000c/0x12 kind-0 pickups —
flagged as a residual, see below.

## Verification

- New: `crates/nf-game/src/pickup.rs` units 12/12 (the composition, the override, the label ride).
- New: `crates/nf-view/tests/fix_pickups_ammo_live.rs` 4/4 (NF_ISO): the id-6 boxes credit type 1
  (41→61→70 clamped, the second box returns 9) with the raw-type message and the take sound; a
  dropped 0xE gun taken twice (weapon clip 9 → ammo 4..8 → clamped at the cap 36 → refused at
  36); the vault rocket created with 4 and taken to 4/0 with the forced 0xF5 (the placement's
  0x13F never queues); the hidden 0x1B pile taken at Bond's feet (clip-fill to clip[0x1b], no
  spare); the 0700000b item take shows the bound 0x2000036 text.
- Neighbours green: `fix_pickups_remainder_live` (the census over the levels, with the override +
  the composed-id refusal model), `fix_pickups_trace_live` 2/2, `fix_auto_switch_live` 6/6,
  `fix_hud_live` 6/6, `fix_objects_live` 2/2, `nf-game --lib pickup` 12/12, `pickup_live` 1/1.
- Full suite (the fast runner, on the FINAL tree fea0e100 = the merge of origin/master
  d1e8f6a7): **1158 passed, 6 failed, 63 ignored, ZERO timeouts** — the 6 failures are the 3
  `disc_golden` allowed on the slim ISO and the 3 pre-existing pylon GPU-adapter failures the
  previous merges document (weapon-matrix4b, fix-aim-raise2). Run in foreground chunks
  (`suite.py --only …`) because this sandbox kills background jobs between tool calls.

## Shared files touched (minimal, additive)

- `crates/nf-game/src/drone.rs` — 3 hooks: the take-gate comment + the removed `!p.hidden`; the
  `take_sound` match arms (Weapon/Ammo → 0xF5); one added field line (`label: 0`) in the
  script-spawn fallback Pickup literal.
- `crates/nf-view/src/scene.rs` — the two placement loaders: `create_rounds_override` +
  `p2.label = param(6)`; `script_pickup_reservations` gained a `level_id` param and the same two
  lines; the `Taken::Label { label }` consumer binds it.
- `crates/nf-view/src/lib.rs` — one pattern widened (`Taken::Label { .. }`) in the dev fallback text.
- `crates/nf-view/tests/fix_hud_live.rs` — the message-mapping mirror updated for the label arm.
- `crates/nf-view/tests/fix_pickups_remainder_live.rs` — the census's expected rounds apply the
  override and its legit-refusal model uses the composed id.
- `nis.rs`, `player.rs`, `scene.rs` draw paths: untouched.

## Residuals / risks

- The 0x52 EquipWeapon special (above) is unmodelled; the two kind-0 0x52 pickups (0700000c,
  07000012) equip and own fine — only the `+0x59e` clear and the held-0x53 rewrite are missing.
- The script-spawn fallback pickup (a pack-script spawn with no reserved slot, `drone.rs` ~7546)
  does not apply the rounds override nor carry a label — `PendingPickup` only carries entity
  params 0–4. The disc's 07000008 safe script IS in the level archive (the reserved-slot path
  covers it), so no known level hits the fallback with 0x2c.
- The AR-pile take (fix 4) is the one behavior I could not verify against PCSX2 from here — the
  decompile evidence is unambiguous (no hidden check in the take path), but please confirm in
  play (live check 4 below).

## Live checks for the main agent

PCSX2 (spot → action → expected):
1. 07000008 vault, rocket placement at (172.8, 42.8, −116.8): take it → the HUD shows 4/4 (no
   spare rocket), the message names the launcher, and the take sound is the standard pickup blip
   (0xF5), not the 0x13F cue the placement carries.
2. 07000001, the two 9mm boxes at (−3.3, 5.8, −9.1) and (−6.5, 5.8, −9.1): take both → the P2K's
   spare (type 1, cap 70) goes 41 → 61 → 70, the second message says 9; the message names the
   raw id-6's ammo type's name (the game's own quirk); at 70 a further box would stay on the
   floor.
3. 07000002, the sniper pickup at (−73.6, 6.2, 23.5): the take sound is the pickup blip (0xF5),
   and the spare sniper rounds = 10 − 5 = 5.
4. Any AR guard drop (e.g. 07000006): walk over the dropped AR → expect the "Picked up Assault
   Rifle" message AND a second "N × Grenades"-style message right after, with the GL rounds
   (clip_size(0x1B)=6 · (0.25+FRand(0.5)) → 1..4) added. THE key confirmation for fix 4.
5. 0700000b, the id-0x47 item at (2.5, 58.4, 42.3): taking it shows the label 0x2000036's text.

nf-view `--bond` spots (render):
- 07000008 `--bond 172.8,42.8,-116.8,0,0` — the rocket launcher model on the shelf, and after the
  take (walk-in) 4/4 in the ammo pane.
- 0700000b `--bond 2.5,58.4,42.3,0,0` — the item pickup drawn; on take the label text bar.
- 07000001 `--bond -3.3,5.8,-9.1,0,0` — the ammo boxes drawn at their placements.

## Left

Nothing from the task list. The two residuals above are out of this task's scope (no disc symptom
beyond the missing +0x59e write). Failing-first note: the four NF_ISO tests assert the exact
negations of the pre-fix behaviors (play p.sound / credit type 2 / create 5 rounds / refuse
hidden), each deterministic from the pre-fix diff — the sandbox's killed background jobs ate the
window for a separate pre-fix-tree rebuild run.
