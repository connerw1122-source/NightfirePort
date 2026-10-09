#!/usr/bin/env bash
# Independent cross-check of the ISO 9660 reader: list and hash every file with 7-Zip's own ISO handler
# (streamed with `7z x -so`, nothing written to disk), then diff against `nf-extract manifest`.
# Usage: tools/crosscheck/iso-7zip.sh <iso> <out.tsv>        (writes "path<TAB>size<TAB>sha1", sorted by path)
set -euo pipefail
iso="$1"; out="$2"
SZ="${SEVENZIP:-/c/Program Files/7-Zip/7z.exe}"
"$SZ" l -slt -ba "$iso" | awk -F' = ' '/^Path = /{p=$2} /^Folder = /{f=$2} /^Size = /{s=$2} /^$/{if(p!="" && f=="-") print p"\t"s; p="";f="";s=""} END{if(p!="" && f=="-") print p"\t"s}' |
while IFS=$'\t' read -r path size; do
  h=$("$SZ" x -so "$iso" "$path" | sha1sum | cut -d' ' -f1)
  printf '/%s\t%s\t%s\n' "$(echo "$path" | tr '\134' '/' | tr '[:lower:]' '[:upper:]')" "$size" "$h"
done | sort > "$out"
