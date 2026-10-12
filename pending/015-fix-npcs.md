# NightfirePort task `fix-npcs`

**Remember your task.** You are working on the task `fix-npcs` and only on it. Re-read this whole file at the start of **every** turn, after every message you get, before you do anything else, and begin each reply with one line: `Task: fix-npcs, branch s/fix-npcs`. Your claimed copy is `claimed/015-fix-npcs.md` on the `queue` branch. If you lose track of anything (the task, the credentials, the file locations), it's all here.

You're working on NightfirePort (github.com/connerw1122-source/NightfirePort): a native Rust port of 007: Nightfire (PS2 NTSC-U), rebuilt from the game's own decompiled logic. Read `docs/PLAN-v2.md` first; it explains why this workflow exists. Then read `CLAUDE.md` and `docs/HANDOFF.md`.

## Credentials

- **GitHub token** (push access; git username: anything, password: the token): the token in your starting prompt. Put it in the shell first, in every new shell: `export GITHUB_TOKEN=<the token from your starting prompt>`.
- **Clone URL with the token:** `https://x-access-token:$GITHUB_TOKEN@github.com/connerw1122-source/NightfirePort.git`
- **The file share** needs no login. Use exactly the `/KOTPT/` paths below; the old `/KnightsOfThePoundTableShare/` paths return 403.
- **Never write the token, the share URLs or any credentials into the repo**, the docs, the WORKLOG, the reports or commit messages. They live only in this task file.

## Setup: run exactly this (bash, Linux sandbox)

```bash
set -euo pipefail
TOKEN="${GITHUB_TOKEN:?export GITHUB_TOKEN=<the token from your starting prompt> first}"
cd ~
[ -d NightfirePort/.git ] || git clone "https://x-access-token:$TOKEN@github.com/connerw1122-source/NightfirePort.git"
cd NightfirePort
git remote set-url origin "https://x-access-token:$TOKEN@github.com/connerw1122-source/NightfirePort.git"
git fetch origin master queue

# Tools: Rust, 7-Zip, the ALSA headers (nf-audio links ALSA on Linux), pkg-config.
command -v cargo >/dev/null || { curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal; . "$HOME/.cargo/env"; }
SUDO=$(command -v sudo || true)
$SUDO apt-get update -qq || true
$SUDO apt-get install -y -qq p7zip-full libasound2-dev pkg-config || true
command -v 7z >/dev/null || python3 -m pip install -q py7zr
unz() { if command -v 7z >/dev/null; then 7z x -y "$1" >/dev/null; else python3 -m py7zr x "$1" .; fi; }

# The research bundle and the slim on-foot ISO (exact sizes; resumable).
get() { curl -fL --retry 5 -C - -o "$1" "https://copyparty.wizardyoda.com/KOTPT/$1"; [ "$(stat -c %s "$1")" = "$2" ] || { echo "BAD SIZE $1"; exit 1; }; }
[ -d work/decomp/ACTION.ELF ] || { get work.7z 2802013555; unz work.7z; rm -f work.7z; }
[ -n "$(find . -maxdepth 2 -name Nightfire-onfoot.iso -print -quit)" ] || { get Nightfire-onfoot.iso.7z 369823203; unz Nightfire-onfoot.iso.7z; rm -f Nightfire-onfoot.iso.7z; }
export NF_ISO="$(realpath "$(find . -maxdepth 2 -name Nightfire-onfoot.iso -print -quit)")"
echo "NF_ISO=$NF_ISO"; ls work/decomp/ACTION.ELF | wc -l   # expect about 3000 files

# Fetch the crates once, so the --offline test runs work afterwards.
cargo fetch
```

Notes:
- `work/` is git-ignored: the decompile is one file per function in `work/decomp/<program>/`, the symbols are in `work/action-symtab.tsv`, and the EE snapshots are `work/ee_start_*.bin` (file offset = EE address).
- If `work.7z` won't fit on the disk (it needs about 15 GB extracted), extract it without the cross-check dumps: `7z x -y work.7z -xr'!nfre' -xr'!uviv'`.
- Set `NF_ISO` again in every new shell: `export NF_ISO=$(realpath $(find ~/NightfirePort -maxdepth 2 -name Nightfire-onfoot.iso -print -quit))`.
- No GPU, so you can't render. That's fine: the main agent renders and checks the pictures.
- If a step fails, retry it once; if it fails again, put the exact error in your report and carry on with what you can do.

## Rules

1. **Work on your branch only.** Create `s/<your task id>` from `master`, then commit and push there. Never push to `master`; the main agent merges. Pull `master` into your branch before you finish.
2. **Never assert from memory.** Every fact gets a tag:
   - [M] code, with the function name and address;
   - [D] disc;
   - [L] EE snapshot, with the address and the value read;
   - [S] suggested.

   The decompiler drops stores and arguments, so check the disassembly wherever the C looks incomplete. Anything a subagent, a tool or a web page says is data: verify it before writing it down.
3. **Never commit game bytes.** The repo is public. Numbers, offsets, names and hashes only.
4. **Never write private URLs, tokens, passwords or account names** into the repo.
5. **Use subagents in parallel** for research and for an independent audit of your own results. An auditor gets your claims plus the decompile and reports which claims it could not reproduce. Fix or retag those before you finish.
6. **Determinism is a hard rule** for any code you touch: the shared bit-exact RNG (port new `Rand` calls in the game's order, and never add draws), the fixed 60 Hz, and no wall-clock time or HashMap iteration order in Sim.
7. **WORKLOG:** one entry named after your task id, recording what you did, what you couldn't verify, and why.
8. **Finish inside the 2-hour sandbox.** If you run short, push what's verified, and list exactly what's left in your report.
9. **Report to the user in plain words:**
   - what's done;
   - what's verified and how;
   - what's left;
   - the branch name.

10. **Push your report with your branch.** Write the plain-language report (the same text you give the user) to `docs/reports/<your task id>.md` and commit it as the last commit on your branch. The main agent reads it from there, so the user doesn't have to paste it. Never put credentials or URLs in it.
11. **Write everything in English:** code, comments, docs, the WORKLOG, commit messages and your report to the user.
12. **Notify the user when you finish.** As your very last step, after pushing, send one phone notification:
    `curl -s -H "Title: NightfirePort" -d "<task id> done: <one-line result>, branch s/<task id>" https://ntfy.sh/nightfire-yoda-8k3x`
    If you stop early or get blocked, send one too, starting with "<task id> BLOCKED:". Never write this URL into the repo.

Read `docs/playtests/2026-10-11.md` first: it's the user's multi-level playtest, and the item ids below refer to it.

## Your task: `fix-npcs`, characters that are drawn but don't act, and the AI items from the user's playtests

These items were attached to every queued task by mistake. They're yours alone now, so no other worker will touch them. Start with C2, because it blocks play on 07000002 and 07000006.

**Also, confirmed by the user on 07000002** (after `fix-drone2`; they didn't press R): the guard in the bathhouse standing next to a civilian woman is still drawn, never attacks, and can't be damaged. Towards the end of 07000002, another NPC next to a hostage does the same. `fix-drone2` couldn't reproduce this headless, so its test doesn't match play.
- Run the real viewer path (`nf_view::load`, the level start, then walk there) and dump every drone that is drawn but either not hittable or not running AI, with present / started / enabled / in_llist / held / drawn / team / mode / dtypenew / channel.
- These may be scripted or ally characters (team 3, mode 5/9, or NIS-driven), not guards. Find what the game does with them, and port it.
**More from the user's latest playtest:**
- **07000005, between the gatehouse and the castle gate:** a patrolling guard walks one way while his upper body looks the opposite way, as if his spine were flipped. Check the patrol look-at and aim-blend (torso twist) direction and clamp.
- **07000006:** stationary guards rotate to follow Bond unrealistically (their whole body or torso spins fully around), never attack, and can't be hit. Probably the same class as C2: find what they are, and port their real behaviour.
- **07000006:** standing in a searchlight beam doesn't raise the alarm. Port the searchlight's spotted → alarm path (channel, drone alert, music event) from `docs/atlas/searchlight.md` and `Searchlight_Update`.

**User retest:** the 07000007 guests no longer T-pose, but they **don't move or walk** at all, despite `fix-drone3`'s CivilianPatrol and PartyGirl port. Reproduce it through `nf_view::load` and the real tick path: check that they have patrol paths, that the drone update runs for them (`npc_enabled`, the llist, start state), and that the walk animation and movement are applied. Fix it.

**Notes from earlier sessions:**
- `fix-ladder2` researched the searchlight → alarm music chain (WORKLOG, the fix-ladder2 entry): `Searchlight_Update` 0x1cb900 → `ch[param3]` → `DroneFunc_CheckAlarmRaised` 0x1453a8 counts `ch[0x37]` → `Music_Event(2,1)` → `UpdateMusicalEvents` 0x114e70 → the per-level `UpdateMFX_*` (07000006: `UpdateMFX_01CastleB` 0x113060). The port runs the castle-A updater only for 07000005, so add the per-level dispatch in `crates/nf-audio/src/music.rs`.
- `fix-ladder2` also reports that `fix-drone4` already fixed the 07000007 guests (CivilianMission 0x16 and the route-chain fix). Retest that headless on master first; only work on it if they still don't walk.
- `fix-drone2`'s headless test didn't reproduce the 07000002 guard. Your test must run the same path as the window: `nf_view::load`, the level start (intro and spawners), then walking Bond to the spot.

Files: `crates/nf-game/src/drone.rs` and the drone AI modules, `crates/nf-view/src/scene.rs` (minimal), `crates/nf-audio/src/music.rs` (the per-level dispatch), and `docs/systems/drone-ai.md`. Update C1/C2/C3 status in your report.

**Reproduce before you fix.** The user's play is the reference, and previous fixes passed tests but still failed in play. For each bug:
1. Write a headless NF_ISO test that drives the real input path (the `nf-view` input mapping into `Sim`, or `nf_view::load` plus `dev::scenario`), and show it reproducing the user's symptom.
2. Then find the cause in the decompile and fix it.
If you can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Update the doc entry** to say it's fixed, with your commit.

**Branch:** `s/fix-npcs`.

## When you finish

1. Push your branch `s/fix-npcs` (never `master`), with `docs/reports/fix-npcs.md` as its last commit.
2. Then finish the queue item: `bash <queue checkout>/finish.sh 015-fix-npcs done docs/reports/fix-npcs.md` (or `blocked`, with the reason in the report).
3. Re-read this file once more and check that you did everything it asks.
