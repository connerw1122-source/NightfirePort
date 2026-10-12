# NightfirePort task `research-driving`

**Remember your task.** You are working on the task `research-driving` and only on it. Re-read this whole file at the start of **every** turn, after every message you get, before you do anything else, and begin each reply with one line: `Task: research-driving, branch s/research-driving`. Your claimed copy is `claimed/240-research-driving.md` on the `queue` branch. If you lose track of anything (the task, the credentials, the file locations), it's all here.

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

**Also for this task:** the driving archives. Run this after the setup:
```bash
get Nightfire-driving.7z 428903899 && unz Nightfire-driving.7z && rm -f Nightfire-driving.7z
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

## Your task: `research-driving`, an atlas of the driving levels (DRIVING.ELF) for a later port phase

The port covers the on-foot levels (ACTION.ELF). Nightfire's driving missions run on a separate executable, DRIVING.ELF (its build identity is in `docs/build-identity.md`; the decompile is in `work/decomp/DRIVING.ELF/`). Nothing has been mapped yet. Write the map so a later phase can plan the port.

1. **Which levels:** the driving missions, their archive files on the disc, and how the front end hands over to DRIVING.ELF and back (the ELF switch, the arguments, the save state carried).
2. **The main loop and the subsystems:** the frame loop, the vehicle physics (the car model, the wheel and suspension, the collision), the weapons and gadgets on the car, the AI traffic and the pursuers, the camera, the HUD, sound and music. For each: the entry points, the key structs with offsets, and how far it shares code with ACTION.ELF (the same engine functions at other addresses, or new code).
3. **The data formats:** which of the on-foot formats (`docs/formats/`) the driving archives reuse unchanged, which differ, and what's new (the road and track data, the vehicle definitions). Run the existing parsers in `crates/nf-formats` over the driving archives and record what parses and what doesn't, as numbers.
4. **The input path:** `PS2PadDevice`, `InputDevice::DeadZoneChop/Scale` (GAPS G-004; the DRIVING.SYM names are hints only, verify them).
5. **A port estimate:** what can be reused, what's new, and a phased plan with sizes.

Deliverables: `docs/atlas/driving.md` (the map), `docs/formats/` notes for any new formats, an NF_ISO test that parses every driving archive with the current parsers and pins the results as golden numbers, and the plan in your report.

**This is a research task.** No game code changes. Rules:
- Every fact gets a tag ([M] with the function and address, [D], [L], [S]). Check the disassembly wherever the decompiled C looks incomplete.
- Use subagents in parallel, and have an independent auditor re-derive your key claims from the decompile. Fix or retag anything it can't reproduce.
- Never commit game bytes: numbers, offsets, names and hashes only.

**Branch:** `s/research-driving`.

## When you finish

1. Push your branch `s/research-driving` (never `master`), with `docs/reports/research-driving.md` as its last commit.
2. Then finish the queue item: `bash <queue checkout>/finish.sh 240-research-driving done docs/reports/research-driving.md` (or `blocked`, with the reason in the report).
3. Re-read this file once more and check that you did everything it asks.
