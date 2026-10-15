//! Level coronas (docs/systems/world-objects.md "Coronas"): `Corona_Create` 0x18da00, placement type 0xFC.
//!
//! Measured [M] and live [L, `work/ee_start_05.bin`]: the create makes a **class-1** object
//! (`obj+0xFF = 1`) via `control_create_object(0, …)` — and `control_funcs[1]` is **empty**, so no update
//! function ever runs. The 13 live coronas of 07000005 keep their create-time state forever: alpha byte
//! `+0x106 = 0xFF` (from `control_init_object`), scale `+0xE8 = 1.0`, `+0xFA = 0`, `+0xF4 = 0`, drawn
//! `+0xFC = 2` while on screen (the renderer's 2→1→0 cycle). There is no instance struct and no RNG draw.
//!
//! What the game does beyond the static instance: `obj+0xD8 = glist` (the placement's object) and
//! `obj+0xF0 = glist+0x44 | (old & 0x30008000)` (`Control_SetGList` 0x134960) — the glist record's flags
//! word becomes the object's display flags. For this level's coronas that word is the object's own flags
//! (0x22D `Corona_Glow2` / `Corona_test1`, 0x229 `Window_corona`), so bit 0x1 holds: `View_AddObjects`
//! (0x1e62d8) puts bit-0x1 objects in **draw list 1** with a mirrored (−1) distance sort key — the alpha
//! list, drawn with each leaf's own GS blend state. `+0xDC = 0`, so the distance-fade branch is skipped.
//!
//! `Corona_Update` 0x18da78 exists but sits in `control_funcs[0x45]` (the 85×12 update table
//! [M 0x26f890; dispatcher stride 0xC at 0x1336f8:62-64]): a *runtime* corona class with a LOS
//! alpha fade (±0x10 per `GameState+52` parity tick, bit 0x2000 while the line of sight is
//! clear). **No class-0x45 creator exists on the disc** — the full caller scan (every
//! `sb …,0xFF` class store in ACTION.ELF's text, the create-call first args, the
//! `Script_CreateEntity` 0x1d8968 kind switch, the searchlight/light/vehicle/explosion
//! creators, direct `jal 0x18da78`) found none — so U-042 is closed as dead code and nothing
//! of it is ported. Full mechanism + proof: docs/systems/corona.md.

use nf_formats::placement::Placement;

/// The main map's type-0xFC placements (`parsemap_create_dynamic_objects` case 0xfc → `Corona_Create`;
/// the 0xA000 cels/templates are not created).
pub fn placements(placements: &[Placement]) -> Vec<usize> {
    placements.iter().enumerate()
        .filter(|(_, p)| p.flags & 0xff == 0xfc && p.flags & 0xa000 == 0)
        .map(|(i, _)| i).collect()
}
