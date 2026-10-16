# mp-defaults-and-setup — report (2026-10-15, branch s/mp-defaults-and-setup)

Task: make the port's MP defaults the game's (boot defaults + the Explosive-Scenery reward gate),
and add the Health Handicap third prompt to the per-agent setup page with draw getters.

## What's done

**1. The three differing defaults now match `bootup_bootup` 0x1c7e78** (crates/nf-view/src/menu.rs,
`FrontEnd::new`):
- Player Mods row 4 Location Damage `mp_player_mods[1]` = **1 (On)** [M store 0x1c7ff8 `sw $a2,0x1c8($s0)`];
- Enviro-Mods row 1 `mp_envmods[0]` = **2 (Random)** [M store 0x1c7fac `sw $t0,0x1c0($s0)`];
- Everything else re-checked against the same function (+0x198=0, +0x19C=10, +0x1A0=10, +0x1A4=1,
  +0x1AC=1, +0x1B4=0, +0x1B8=0, +0x1BC=0, +0x1C4=1, +0x1CC=0, +0x1D0=0, +0x1D4=0; roster slots
  8×0x30 with +0x20=team i&1, +0x28=1, +0x2c=0) — the port already matched, and the four EE
  snapshots read the same values [L ee_start_05/07000001/0700000a/ee_snapshot.bin at MPSettings
  0x2a47a0, file offset = EE address].
- Where the game initialises MPSettings: `bootup_bootup` 0x1c7e78 is the only fresh-boot source
  (.data starts zeroed [D ELF 0x19e6a0]). The codename profile persists THREE rows:
  `Menu_UpdateDefaultCodename` 0x1f8218 saves +0x1BC/+0x1C0/+0x1C4 into the record's +0x2e/0x2f/0x30
  and `Menu_MapDefaultCodename` 0x1f7fa8 restores them (0x1f802c/0x1f806c); match-to-match reuse
  goes through `Menu_StoreMPSettings` 0x2027d0 / `Menu_RestoreMPSettings` 0x202848. **Not ported** —
  the port's `nf_game::profile::Profile` doesn't model MP rows and adding them touches the save
  format (next worker).

**2. The Explosive-Scenery reward gate, ported.** [M] `Menu_UnlockMPSettings` 0x1ff698 (called on
every P_MPSCENARIO page enter, 0x217348:23) ORs the profile's four bonus words (bonus[0..3] @
0x2e1b88) into a mask, then walks 12 levels × counters 1..4 through `PlrStarts_ProcessRewardCounter`
0x199210 over `RewardsTable` @0x2bc920 (12 × 0x40: level id, then per counter u16 id + u16 type).
The shipped table's ONLY type-3 entry is level 0x07000014 counter 3, id 0x3e [D dumped from the
ELF]; it sets `mp_stuff[0x370]` @0x2df2d0. So the gate = bonus-mask bit 0x3e. The port: new
`mp_bonus_mask: u64` field (0 on a fresh profile [L snapshots]) + `mp_explosive_unlocked()` +
`mp_enviro_value_label(row)` reproducing P_MPENVIROMODS 0x210030's msg-0x4c node lists:
- locked: the row's list is the single node (value 0x10, label 0x1000076 "Locked") [M the locked
  branch binds 0x1000076@0x10] → Left/Right is a no-op, and closing the page stores **0x10** into
  +0x1D4 [M the msg-0x4b read-back store 0x210464] — exactly what PCSX2 shows;
- unlocked: On(0x1b7)@1 then Off(0x1b6)@0; a stale 0x10 displays the first-added node (On) [M
  List_SendMessage 0x2196e8's msg-0x1e no-match keeps the first node current] and the close stores
  the current node's value;
- match start masks the row &1 [M P_MPCONFIRM 0x20ec00's 0x20f708/0x20f710] — ported in
  `mp_confirm_start`; the unlock-everything cheat does NOT open the row [M menu_unlock_everything
  never touches the bonus words].

**3. Health Handicap, the setup page's third prompt.** [M] P_MPSETUP 0x20d7a8 + C_RBMPSETUP
0x20de58: states 2 (Choose Team, team games) → 3 (Choose Character) → **4 (Health Handicap)** → 5
(done). The handicap list is 8 nodes with values −75..+100 whose labels are the handler's own .data
strings 0x30d390..0x30d3c8 [D "-75","-50","-25","0","+25","+50","+75","+100"] — the same literals
P_CNMPOPTIONS 0x2144d8 uses for its per-profile handicap row (label 0x296). The pick lands in
MPSettings+slot·0x30+0x2c (fresh boot 0 → the "0" node), sets mpjoin+1 and state 5; when every
joined agent is done, Menu_MPAreWeReady 0x1f78d8 fires the 0x44 page change to 0x40000012
(P_MPOPTIONS). Port: `mp_setup_state` now 2/3/4/5, `mp_handicap: i32` (the roster +0x2c), new
`MP_HANDICAPS`, the MpSetup handler (Confirm steps 2→3→4→5, Back walks 4→3→2-team-only per the 0x6b
arm), and the draw getters the main agent asked for:
- `mp_setup_prompt_label()` — 0x37c / 0x37d / 0x296;
- `mp_setup_image()` — Phoenix 0x30001a3 / MI6 0x30001a2 by team selection [M the 0x49/0x54 arm],
  the character row's preview (MP_CHARACTERS[x].0 = Menu_GetItemFromHash(0x2df8f8, v, 0x1d) →
  item+0), the green cross 0x30001a1 on the handicap step;
- `mp_setup_value_text(lang)` — the team/character label ids bound through Lang, the handicap's
  literal string;
- plus `mp_enviro_value_label(row)` (above) for the Enviro-Mods values.

**4. Quick-Game correction** (found while auditing the writers of +0x1D4): C_SBMPSCEN 0x20c600
stores **+0x1CC = 2 (Helicopters)** at 0x20c734 — docs/systems/mp-menus.md §5 said 0; the doc and
`mp_quick_start` are fixed (Car_Create 0x1305b0:29-40 spawns choppers on 2). The quick start never
writes +0x1C0, so the boot value 2 (Random) persists; it clears +0x1D4 [M 0x20c748].

## How it was verified

- New test binary `mp_defaults_setup` (10 tests, ids/numbers only): the boot defaults table, the
  locked/unlocked gate incl. the 0x10 close-write and the &1 match-start mask, the enviro value
  labels, the quick-start writes, the three-prompt flow with per-step label/image/value pins, the
  handicap re-select of the stored value, the back walk, FFA skipping the team, the node table.
- `mp_menu_flow` updated for the 3-step flow (every character→options path gained the handicap
  confirm) — 9/9. `menus_flow` 36/36, `dev_menu_flow` 5/5.
- **Suite slice on the final merged tree** (origin/master merged at 8ba16019; suite.py --only,
  foreground): **410 passed, 0 failed, 0 ignored in 8 min** over the 18 binaries mp_defaults_setup,
  mp_menu_flow, menus_flow, menu_movies, dev_menu_flow, campaign_flow_live, screen_adjust,
  fix_weapon_modes_live, fix_viewmodel_zoom_live, fix_interp_ghosting_live, fix_hud_zoom_tilt_live,
  mp_bots_live, mp_core_live, mp_menu_lists_live + the nf_view/nf_game/nf_formats/nf_audio unit
  binaries (359 test binaries build clean). The full 359-binary pass cannot fit this window:
  background runs are killed between tool calls in this sandbox (the driver's background
  `suite.py` start died twice — once on a missing cargo PATH, once reaped), so foreground `--only`
  slices are the working mode, as in screen-adjust-model.
- Known pre-existing failures NOT in the slice: fix_0c_pylon_live's 2 render tests (wgpu
  request_adapter panic — no GPU here; fails identically on a clean tree, and mp-match-rules
  recorded the same).
- `cargo test --release -p nf-view` ran the whole crate's binaries green except the 2 no-GPU pylon
  tests.

## What's left

- The codename profile's three saved MP rows (+0x1BC/+0x1C0/+0x1C4 ↔ record+0x2e/0x2f/0x30) —
  evidence above; needs a Profile save-format decision.
- `mp_bonus_mask` has no writer yet (no port code earns reward bits); the gate is a function of it,
  so a future rewards task only sets bits.
- mp_stuff[0x378]/[0x379] skin-exclusivity (Menu_GetMPSkins's second/third args) and agents 2-4's
  handicap prompts (the port models the keyboard's agent 1) remain unported, as before.

## Live checks for the main agent

1. P_MPENVIROMODS page 0x40000028, fresh boot: row 3 (+0x1D4) shows **"Locked"** (label 0x1000076)
   and Left/Right do nothing; after visiting the page once, MPSettings+0x1D4 @0x2a4974 should read
   **0x10** [L currently 0 before any visit; access type: word read after page exit].
2. Player Mods page 0x40000017 row 4 shows **On**; Enviro-Mods row 1 shows **Random** (fresh boot).
3. P_MPSETUP page 0x40000051 on Team Arena: after the character pick, the prompt label 0x10000296
   ("Health Handicap"), the green-cross image (msg-0x24 id 0x30001a1 on control 0x100001a0), the
   value text walking "-75".."0".." +100" per ◀ ▶, default selection "0" (MPSettings roster
   +slot·0x30+0x2c = 0 → word read at 0x2a47a0+slot*0x30+0x2c).
4. The team step's image should be Phoenix's art on selection 0 and swap to MI6's on ▶ (ids
   0x30001a3 / 0x30001a2 on 0x100001a0).
5. Render spots (keyboard agent 1): `--bond` not needed — the pages are menu-only; draw the port's
   Page::MpSetup through the three states and compare with PCSX2's three prompts on Team Arena.

## ntfy failed

All three fallbacks rejected — the ntfy.sh daily message quota is exhausted (22:44 UTC):
1. `curl -sS -H "Title: NightfirePort" -d "…" https://ntfy.sh/nightfire-yoda-8k3x` →
   `{"code":42908,"http":429,"error":"limit reached: daily message quota reached; increase your limits with a paid plan, see https://ntfy.sh","link":"https://ntfy.sh/docs/publish/#limitations"}`
2. `python3 urllib` → `urllib.error.HTTPError: HTTP Error 429: Too Many Requests`
3. `wget --post-data …` → `HTTP request sent, awaiting response... 429 Too Many Requests` /
   `ERROR 429: Too Many Requests` (a first quiet attempt exited 0 but the verbose retry shows the
   same 429; nothing was published)
