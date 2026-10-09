"""Independent cross-check of the BIGF reader with unvivtool (GPL-3: we only RUN it, never copy code).

For every BIGF archive in a directory tree (e.g. the pycdlib dump made by nfre_files_bin.py), unvivtool's get_info()
gives the entry table and unviv() extracts the files; we hash them. Output rows match golden/bigf_entries.tsv minus
the per-archive index: `path<TAB>name<TAB>offset<TAB>size<TAB>sha1`, sorted.

Usage: work/third_party/.venv-uviv/Scripts/python -I tools/crosscheck/unviv_bigf.py <dump root> <scratch dir> <out.tsv>
"""
import hashlib
import os
import shutil
import sys

import unvivtool


def main(root, scratch, out_tsv):
    rows = []
    for dirpath, _dirs, files in os.walk(root):
        for fn in files:
            path = os.path.join(dirpath, fn)
            with open(path, "rb") as f:
                if f.read(4) != b"BIGF":
                    continue
            info = unvivtool.get_info(path)
            rel = "/" + os.path.relpath(path, root).replace(os.sep, "/").upper()
            dest = os.path.join(scratch, rel.strip("/").replace("/", "_"))
            shutil.rmtree(dest, ignore_errors=True)
            os.makedirs(dest)
            # Extract each entry by its 1-based index into its own folder: names repeat inside an archive
            # (MIS3EN.SPE has two different `garage.asf`), and unvivtool creates no subdirectories for `data\...`.
            for i, (name, ofs, size) in enumerate(zip(info["files"], info["files_offsets"], info["files_sizes"])):
                one = os.path.join(dest, str(i))
                sub = os.path.dirname(name.replace("\\", os.sep))
                os.makedirs(os.path.join(one, sub), exist_ok=True)
                if not unvivtool.unviv(path, one, fileidx=i + 1, overwrite=1):
                    raise SystemExit(f"unviv failed on {rel} entry {i}")
                got = [os.path.join(dp, x) for dp, _d, fs in os.walk(one) for x in fs]
                if len(got) != 1:
                    raise SystemExit(f"{rel} entry {i} ({name!r}): expected 1 extracted file, got {got}")
                p = got[0]
                data = open(p, "rb").read()
                if len(data) != size:
                    raise SystemExit(f"{rel}: {name} extracted {len(data)} bytes, table says {size}")
                rows.append((rel, name, ofs, size, hashlib.sha1(data).hexdigest()))
    rows.sort()
    with open(out_tsv, "w", newline="\n") as f:
        f.write("path\tname\toffset\tsize\tsha1\n")
        for r in rows:
            f.write("\t".join(map(str, r)) + "\n")
    print(f"{len(rows)} entries -> {out_tsv}")


if __name__ == "__main__":
    main(*sys.argv[1:4])
