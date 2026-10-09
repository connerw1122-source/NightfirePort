"""Independent cross-check of FILES.BIN extraction with NightfireResearch's nightfire-re (no licence: we only RUN it).

Their path is independent of ours: pycdlib (a third ISO 9660 implementation) dumps the disc, and their
PlaystationEurocomHandler reads FileList at the hard-coded ELF offset they derived (memaddr 0x2446A0), not via the
symbol table. Their own entry point refuses images whose hash it doesn't know (ours differs, UNKNOWNS U-001), so this
driver calls the two handlers directly and stubs the post-processing steps (bin expansion, map parsing).

Usage (from the repo root, with the nightfire-re venv):
  work/third_party/.venv-nfre/Scripts/python -I tools/crosscheck/nfre_files_bin.py <iso> <nightfire-re dir> <work dir> <out.tsv>
Writes `name<TAB>size<TAB>sha1` sorted by name (hashcode names restored from their renamed outputs).
"""
import hashlib
import os
import re
import sys


def main(iso, nfre_dir, work, out_tsv):
    sys.path.insert(0, os.path.abspath(nfre_dir))
    from platform_ps2 import ps2_eurocom_handler, ps2_iso_handler  # noqa: E402  (their code)
    from platform_hash import PlatformHash  # noqa: E402

    dump = os.path.join(work, "dump")
    if not os.path.exists(os.path.join(dump, "FILES.BIN")):
        ps2_iso_handler.PlaystationIsoHandler().dump_iso(iso, dump)
    # Stop after the 118 top-level files.
    ps2_eurocom_handler.extract_bin.extract_all = lambda *_a, **_k: None
    ps2_eurocom_handler.parse_map.parse_maps = lambda *_a, **_k: None
    ps2_eurocom_handler.extract_audio.extract_all = lambda *_a, **_k: None
    ps2_eurocom_handler.PlaystationEurocomHandler().dump_eurocom_files(dump, PlatformHash("SLUS-20579"))

    target = os.path.abspath(os.path.join(dump, "..", "platform_ps2", "ps2_archives_extracted"))
    rows = []
    for fname in os.listdir(target):
        path = os.path.join(target, fname)
        if not os.path.isfile(path):
            continue
        m = re.match(r"^([0-9a-fA-F]{8})_.*\.bin$", fname)   # they rename known hashcodes: 07000001_Name.bin
        name = m.group(1) + ".bin" if m else fname
        data = open(path, "rb").read()
        rows.append((name, len(data), hashlib.sha1(data).hexdigest()))
    rows.sort()
    with open(out_tsv, "w", newline="\n") as f:
        f.write("name\tsize\tsha1\n")
        for r in rows:
            f.write(f"{r[0]}\t{r[1]}\t{r[2]}\n")
    print(f"{len(rows)} files -> {out_tsv}")


if __name__ == "__main__":
    main(*sys.argv[1:5])
