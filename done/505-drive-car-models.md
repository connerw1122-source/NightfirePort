## Your task: `drive-car-models`, the driving missions' car models

From the driving VIV archives (500 refpack, 501 the DRIVING.ELF map in docs/atlas/driving-elf.md), find and decode
the player's and the AI cars' models (format, LODs, textures, wheel attachment points). A `nf_formats` module + an
`nf-extract` command dumping OBJ locally (never committed). Golden: per-model vertex/triangle counts + buffer
SHA-1s. Document the format in docs/formats/. No renderer work.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/drive-car-models`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox3 at 2026-10-15T19:59Z

---
## Report (done, 2026-10-15T21:00Z)


---
## Report from the worker (done, 2026-10-15T21:00Z)

# `drive-car-models` — the driving missions' car models

Branch `s/drive-car-models`, 2026-10-15, worker session (one sandbox, ~2 h). Task: from the
driving VIV archives, find and decode the player's and the AI cars' models (format, LODs,
textures, wheel attachment points); a `nf_formats` module + an `nf-extract` command dumping
OBJ locally; goldens (per-model vertex/triangle counts + buffer SHA-1s); the format documented
in docs/formats/. No renderer work.

## Done

1. **The data path works in this sandbox**: `DRIVING/` was restored from
   `Nightfire-driving.7z` (428,903,899 B, the same restore `drive-crp-format` used — the slim
   on-foot ISO zeroes the `/DRIVING` directory records). `MISC.VIV` matches its
   `disc_files.tsv` golden size (189,097,360 B).
2. **Format decoded** (docs/formats/car-model.md, everything tagged [M]/[D]):
   - the car `.crp` article window: `Base`, `Name`, `Skel`, `an/aq/as/cs/ef/el`, `in#0`,
     `si#60`, `sr#12..46` (named resources like `EAGL::taxi::{as 0004}::Model[0].00`),
     directory `ELFd+ELFr+Sect+ns#0` (ns#0 = the `.ssh` texture sheet filename);
   - `Skel` = the named part table — **the wheel/hub attachment points are parts here**:
     the vehicle setup resolves `leftfrontwheel`/`rightfrontwheel`/`leftrearwheel`/
     `rightrearwheel` (+ hubs, fins/rudders for boats) by name through `FUN_001c9980`
     0x1c9980, with the name-pointer tables at EE 0x334070/0x334080/0x334090 [M
     FUN_001d2648 0x1d2648];
   - the embedded MIPS ET_REL module (ELFd || ELFr glued; `.data` = the model, `.symtab`
     names everything, `.rel.data` all `R_MIPS_32`): `__Model:::taxi::{as NNNN}::Model[L].00`
     descriptors = **variant × LOD** (taxi: 8 variants, LOD0/LOD1), `__EAGL::TAR::` texture
     refs (resolved against the `.ssh` sheet by content hash), `MatCar*` materials,
     `__EAGL::GeoPrimState:::STATE::` render states;
   - the descriptor's batch table (`[0xa0000000][0][count]` + `count/2` ×
     `[0xa000ffff][ptr]`) and the draw buffers: **VIF streams** — `V4_32 NUM=1` header, three
     `V4_8 NUM=1` quads, then `V4_32 NUM=n` with n (x,y,z,1.0) f32 quads = the vertices
     (taxi LOD0: 1087; jungle_truck LOD0 total: 7070).
3. **Port**: `crates/nf-formats/src/carmodel.rs` (`CarModel::parse` over a decompressed
   `.crp`: article, name, `Skel`, variants, resources, instances, sheet name, module glue;
   `CarModule::parse`: ELF sections/symbols/relocs, model descriptors, batch groups, buffer
   byte ranges + SHA-1 + a VIF census). Unit tests on a hand-built container (2).
4. **CLI**: `nf-extract carmodels <file.crp|file.viv> [out-dir]` — reads a car `.crp` or a
   `DRIVING/*.VIV` (RefPack-aware), prints a TSV census, writes per-buffer `.bin`s + an OBJ
   (vertices only) into the out-dir (local, never committed).
5. **Goldens** (measured on the disc data): `golden/carmodel_golden.tsv` — per car: parts,
   model count, per-(variant, LOD) `vertices(v4_32 w=1.0) : v4_32_all`, buffer count, a SHA-1
   over the ordered buffer SHA-1 list, article + sheet name. Test
   `tests/carmodel_golden.rs` (gated on `NF_DRIVING`/`<repo>/DRIVING`, skips without it):
   `misc_car_models_match_the_golden` (diff + the one-byte-tamper sister) and
   `misc_car_models_have_named_wheel_parts` (the four wheel names in every car's `Skel` +
   a non-trivial-parts sister). 2/2 green with `NF_DRIVING=$(pwd)/DRIVING`.
6. **Wheel attachment points** (the task's ask): the parts named
   `leftfrontwheel`/`rightfrontwheel`/`leftrearwheel`/`rightrearwheel` (+ `leftfronthub`
   etc.) in the `Skel` table, resolved by the game into vehicle-object indices (wheels at
   obj+0x248, hubs at obj+0x258) [M 0x1d2648]. The golden tests pin the four wheel names for
   all five MISC cars.

## What's left (honest list)

- **Triangle connectivity**: the per-triangle index list inside the draw buffers is not
  decoded — the OBJ is vertices-only. The reader of these packets is the EAGL GeoPrim
  library in DRIVING.ELF (0x262000–0x2a7000 [D strings]); pinning its per-packet consumer is
  the next pass (live check C-4 in docs/formats/car-model.md §8). Until then the golden pins
  vertex counts + buffer SHA-1s, not triangle counts.
- **The DMA head words** of each buffer (`[0x3f0][0][0x600000db]…`) and the exact semantics
  of the `0xa000ffff`/`0xa0000000` batch words are [S].
- **Non-1.0 V4_32 quads** (normals/UVs) are counted but not interpreted; bus_tokyo keeps its
  vertices in non-V4_32 streams (the golden pins both totals).
- **Mission-car spot check**: the goldens cover the five plain `MISC.VIV` cars; the 25+
  RefPack-compressed mission cars (paris_henchbigbox, alpha_sub, …) parse through the same
  code path (`nf-extract carmodels MIS01.VIV`) but are not golden-pinned.
- **The main agent's render/parity pass**: the OBJs are ready in an out-dir; nothing visual
  was verified here (no GPU).

## Verification: what was run

- `NF_DRIVING=$(pwd)/DRIVING cargo test --offline --release -p nf-formats --test
  carmodel_golden` → **2/2 passed** (the golden + the wheel-names test, sisters included;
  both skip cleanly without the DRIVING folder).
- `cargo test --offline --release -p nf-formats --lib carmodel` → 2/2 (hand-built-container
  unit tests).
- `cargo build --offline --release -p nf-extract` clean; `nf-extract carmodels` run against
  all five `MISC.VIV` cars from the restored `DRIVING/` folder (census TSVs in /tmp, not
  committed).
- The full four-crate suite: the fast runner could not stay alive in this sandbox
  (background processes are killed between tool calls; two `nohup setsid` launches died),
  so the run below is the foreground crates-my-tests-touch command instead — see the suite
  section of the WORKLOG for the exact totals.

## ntfy

(sent after this report; see the finish steps)
