# weapon-model-coverage — every weapon has its first-person model on every level

The user: "Every single weapon needs to have a first person model" (the dev panel's weapon gives on
any level), and load logs still printed `session mesh merge: … (27 weapon-model hashes still
missing)` on 07000005. Branch `s/weapon-model-coverage`.

## What the census found [D — measured through the port's own parsers]

Method: `weapon_data` (115 × 0x10c) read from the EE snapshot at 0x2bf150 (filled by the boot
initialiser; docs/formats/weapon-data.md), `ammo_data` (34 × 12) at 0x2c69b8, `FileList` (118 × 32) at 0x2446a0 — all in `work/ee_snapshot.bin` (file offset = EE address); every
`.bin` archive on the disc scanned with the repo's `LevelBin`/`MapFile` rules (kind-3 skins, kind-5
scripts, ENTITY_PARAMS (0x04) object hashcodes in kind-0/1/2/11 maps). The same numbers then
re-proved in-tree by `crates/nf-view/tests/weapon_model_census_live.rs` through the REAL merge
functions (`merge_weapon_models` + `merge_weapon_assets`), all 20 story levels.

1. **Every fp-armed weapon's first-person skin (+0xDC) exists on the disc** — 41 distinct skins
   over 95 weapon ids; the other 19 ids (0x39, 0x47-0x49, 0x5f-0x69, 0x6b, 0x70-0x72: the
   gadget/consumable tail) have `+0xDC = 0` and are usable without a model, exactly as the shipped
   game (`Player_EquipWeapon` only refuses +0xDC == 0 [M]).
2. **A different archive per weapon class, exactly as suspected**: most fp skins live on the MP
   packs — 07000029, 0700004B, 07000046, 07000043, 07000049 — not on any story archive. The disc
   index (task 100a) already resolves them; the census confirms every one lands.
3. **Hash aliases are benign**: the silenced twins share the bare weapon's skin (0x02/0x03 →
   05000088, …), the variant families share theirs (0x24-0x29 → 05000043, the darts 0x3b-0x40 →
   05000076, …). First-wins keeps every shared resolution consistent.
4. **The "27 still missing" line is a mid-load stage count, not the final state.** The session mesh
   merge (`merge_weapon_models`, lib.rs) sweeps EARLIER story archives only — the game's own
   session-accumulation shape — and logs its remaining count; the dev/give merge
   (`merge_weapon_assets`) then covers most of the rest from the fp skins' owning archives.
5. **The one real miss the merges could never reach**: weapon 0x63's third-person mesh
   **0x02000312** lives only on 07000004 and 07000016 [D census]. On 07000001-03 there is no
   earlier story archive to sweep, and no wanted fp skin's first holder is 07000004/16, so
   `merge_weapon_assets` never opened them — the given 0x63 had no held-gun mesh there. **Fixed**:
   the disc index now tabulates map objects (`DiscIndex.objects`, cache V2) and
   `merge_weapon_assets` tops up the still-missing weapon-model meshes from the archive the index
   names for them (the same session-accumulation mechanism, one `take_maps_holding` per archive;
   surgical for `--give` — only the given weapons' own meshes can open an archive — whole-table
   for `--dev`).
6. **The honest residue is the game's own**: the casing meshes **0x02000446** (`ammo_data[6]`+4)
   and **0x02000443** (`ammo_data[10]`+4) are on NO archive of the disc [D census; EE snapshot
   0x2c69b8] — the shipped game cannot resolve them either (the same class as the invisible stun
   grenade 0x0200071f, docs/models/weapons-items.md §3). The load log now says so explicitly:
   `weapon models: 2 hash(es) unresolved after the merge, on no archive of the disc:
   02000446 02000443`.

## Census table — weapon → model hash → source archive → resolved

Grouped by fp skin (variants/silenced twins share one). "Resolved" = fp skin + its main/extra
meshes + skeleton + view scripts + the record's flash (+0x48)/third-person (+0x80)/projectile
(+0x5C) meshes, on all 20 story levels, after the fix (asserted per level by the census test).

| weapon ids | fp skin (+0xDC) | source archive(s) | resolved |
|---|---|---|---|
| 0x02 0x03 | 05000088 | 07000029.BIN (+27 more) | yes |
| 0x04 0x05 | 050000a9 | 07000029.BIN (+27 more) | yes |
| 0x06 0x07 0x46 | 05000047 | 07000029.BIN (+27 more) | yes |
| 0x08 0x09 | 050000a8 | 07000029.BIN (+27 more) | yes |
| 0x0a 0x0b 0x0c 0x0d | 05000051 | 07000029.BIN (+17 more) | yes |
| 0x0e 0x0f | 05000040 | 07000029.BIN (+13 more) | yes |
| 0x10 | 0500003f | 07000029.BIN (+13 more) | yes |
| 0x11 | 05000083 | 07000029.BIN (+14 more) | yes |
| 0x12 0x13 | 05000048 | 07000029.BIN (+15 more) | yes |
| 0x14 0x15 | 05000050 | 07000029.BIN (+16 more) | yes |
| 0x16 0x17 | 050000c0 | 07000029.BIN (+11 more) | yes |
| 0x18 0x19 | 05000049 | 07000029.BIN (+16 more) | yes |
| 0x1a 0x1b | 0500007d | 07000029.BIN (+12 more) | yes |
| 0x1c 0x1d | 0500004a | 07000029.BIN (+17 more) | yes |
| 0x1e 0x1f 0x20 0x21 0x22 0x23 | 05000071 | 07000029.BIN (+15 more) | yes |
| 0x24 0x25 0x26 0x27 0x28 0x29 | 05000043 | 07000029.BIN (+11 more) | yes |
| 0x2a 0x2b | 0500008d | 07000029.BIN (+13 more) | yes |
| 0x2c 0x2d 0x2e 0x2f 0x6d | 05000046 | 07000029.BIN (+14 more) | yes |
| 0x30 0x31 | 0500007a | 07000029.BIN (+10 more) | yes |
| 0x32 0x33 0x6a 0x6e 0x6f | 0500007f | 07000029.BIN (+13 more) | yes |
| 0x34 | 05000041 | 07000029.BIN (+23 more) | yes |
| 0x35 | 05000070 | 07000029.BIN (+19 more) | yes |
| 0x36 | 0500006f | 07000029.BIN (+16 more) | yes |
| 0x37 | 05000072 | 07000029.BIN (+12 more) | yes |
| 0x38 0x4e 0x4f 0x5a 0x5c | 0500004b | 07000043.BIN (+23 more) | yes |
| 0x3a | 05000073 | 07000029.BIN (+13 more) | yes |
| 0x3b 0x3c 0x3d 0x3e 0x3f 0x40 0x6c | 05000076 | 07000029.BIN (+15 more) | yes |
| 0x41 | 0500005d | 07000029.BIN (+9 more) | yes |
| 0x42 | 05000091 | 07000029.BIN (+9 more) | yes |
| 0x43 0x44 | 050000a3 | 07000046.BIN (+4 more) | yes |
| 0x45 | 050000b6 | 07000029.BIN (+9 more) | yes |
| 0x4a 0x4b 0x4c 0x4d | 0500003e | 07000046.BIN (+14 more) | yes |
| 0x50 0x51 | 05000044 | 07000029.BIN (+22 more) | yes |
| 0x52 0x53 | 05000080 | 07000029.BIN (+14 more) | yes |
| 0x54 0x55 | 05000045 | 07000046.BIN (+10 more) | yes |
| 0x56 0x57 | 0500005c | 07000046.BIN (+6 more) | yes |
| 0x58 | 0500005b | 07000046.BIN (+4 more) | yes |
| 0x59 | 05000079 | 07000046.BIN (+8 more) | yes |
| 0x5b | 050000bc | 07000046.BIN (+8 more) | yes |
| 0x5d 0x5e | 05000042 | 07000046.BIN (+21 more) | yes |
| 0x01 | 050000ab | 07000043.BIN (+17 more) | yes |
| 0x39 0x47 0x48 0x49 0x5f 0x60 0x61 0x62 0x63 0x64 0x65 0x66 0x67 0x68 0x69 0x6b 0x70 0x71 0x72 | — (no fp model) | — | yes (usable without a model, as the game) |

(The "+N more" counts each skin's remaining holding archives — up to 27 archives carry a popular
skin; the disc index keeps the first in FileList order, the game's accumulating-table order.)

Per level (the miss counts the whole-table wanted-mesh list — records' +0x80/+0x4C/+0x48/+0x5C/
+0x7C, the sleeves, the casing meshes — not covered at each stage):

| level | session-merge stage (the old log line) | after the fix (residue) |
|---|---|---|
| 07000001 | 37 | 2 — 02000446 02000443 (on no archive) |
| 07000002 | 37 | 2 — 02000446 02000443 (on no archive) |
| 07000003 | 37 | 2 — 02000446 02000443 (on no archive) |
| 07000004 | 33 | 2 — 02000446 02000443 (on no archive) |
| 07000005 | 27 | 2 — 02000446 02000443 (on no archive) |
| 07000006 | 27 | 2 — 02000446 02000443 (on no archive) |
| 07000007 | 26 | 2 — 02000446 02000443 (on no archive) |
| 07000008 | 25 | 2 — 02000446 02000443 (on no archive) |
| 07000009 | 25 | 2 — 02000446 02000443 (on no archive) |
| 0700000a | 25 | 2 — 02000446 02000443 (on no archive) |
| 0700000b | 25 | 2 — 02000446 02000443 (on no archive) |
| 0700000c | 21 | 2 — 02000446 02000443 (on no archive) |
| 0700000d | 21 | 2 — 02000446 02000443 (on no archive) |
| 07000011 | 17 | 2 — 02000446 02000443 (on no archive) |
| 07000012 | 17 | 2 — 02000446 02000443 (on no archive) |
| 07000013 | 16 | 2 — 02000446 02000443 (on no archive) |
| 07000014 | 11 | 2 — 02000446 02000443 (on no archive) |
| 07000015 | 11 | 2 — 02000446 02000443 (on no archive) |
| 07000016 | 11 | 2 — 02000446 02000443 (on no archive) |
| 0700001b | 9 | 2 — 02000446 02000443 (on no archive) |

The session-stage counts differ from the raw pre-merge want (e.g. 07000005 wants 37 before any
merge; its session sweep from 07000001-04 leaves 27 — the user's log line) because the earlier
archives already carry many meshes; the fixed end state is the same two no-archive hashes
everywhere, and 0x63's 0x02000312 now resolves on 07000001-03 too (the census test's pre-fix
run had it in the residue there).

## Verification

- `crates/nf-view/tests/weapon_model_census_live.rs` (new):
  - `dev_panel_weapons_resolve_on_three_levels` — `load_weapons` with all 95 fp-armed ids on
    07000005, 0700000c, 07000014: every one builds its first-person view model.
  - `census_every_story_level_resolves_the_table` — the real `merge_weapon_models` +
    `merge_weapon_assets` (dev = `Wanted::All`) on all 20 story levels; asserts the residue is
    exactly {02000446, 02000443} (nothing that IS on the disc stays missing) and that every
    fp-armed weapon's skin/skeleton/main/flash/tp/proj meshes resolve per level.
  - `a_bogus_hash_reports_missing` — a doctored fp hash 0xdeadbeef: the index names no archive,
    the merge reports "skin deadbeef is on no archive of the disc", merges nothing for it, and it
    never enters the skin table (the fists skins still merge, as Bond always owns them).
- `crates/nf-view/tests/weapon_assets_live.rs`: 5/5 pass with the change (its id loops widened to
  the full table 1..=0x72).
- Crates run before pushing: `cargo test --offline --release -p nf-view --test
  weapon_model_census_live --test weapon_assets_live -- --test-threads=1` (8/8 green) and
  `cargo check --offline --release -p nf-view --tests`. nf-formats/nf-game/nf-audio untouched.
- Full fast suite on the final tree: see the total line below.

## Suite total (fast runner, final tree merged with origin/master)

(to be filled in from suite_final.out before the finish push)

## Render spots for the main agent (data only — the look needs your render)

- `--bond 14.8,16.1,2.3,0.0` on 07000011, dev-give weapon 0x63 and drop it / arm a guard with it:
  its held-gun mesh 0x02000312 should draw (before the fix it had no mesh on 07000001-03).
- Any level, `--dev`: open the dev panel's weapon list — every gun shows a first-person model
  (07000005's own weapons always did; the MP-pack class — e.g. the Golden Gun 0x2a/0x2b, the
  Frinesi 0x50/0x51 — is the new-coverage set).

## What's left

- Nothing known in this lane: every on-disc weapon-model hash resolves on every single-player
  level, and the two no-archive hashes are the shipped game's own gap (live check optional: a
  PCSX2 read of the game's hashtable for 0x02000446/0x02000443 during a shotgun reload on any
  level would confirm the game misses them the same way — expected result: lookup miss).
